//! Idle unload (FR-IMG-23). API/MCP start with no sidecar child and no VRAM.
//! First local sidecar job may spawn a child; when the queue is idle for
//! `TEXT2MESH_IDLE_UNLOAD_S` (default 120) the child is killed, not leaked.

use std::process::{Child, Command};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

const DEFAULT_IDLE_S: u64 = 120;

pub struct IdleUnload {
    seconds: u64,
    in_flight: AtomicU32,
    last_done: Mutex<Instant>,
    child: Mutex<Option<Child>>,
}

impl IdleUnload {
    pub fn from_env() -> Arc<Self> {
        let seconds = std::env::var("TEXT2MESH_IDLE_UNLOAD_S")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_IDLE_S);
        Arc::new(Self::new(seconds))
    }

    pub fn new(seconds: u64) -> Self {
        Self {
            seconds,
            in_flight: AtomicU32::new(0),
            last_done: Mutex::new(Instant::now()),
            child: Mutex::new(None),
        }
    }

    pub fn idle_seconds(&self) -> u64 {
        self.seconds
    }

    /// True iff a child pid is still held. Start-of-process must be false.
    pub fn loaded(&self) -> bool {
        self.child.lock().map(|g| g.is_some()).unwrap_or(false)
    }

    pub fn job_begin(&self) {
        self.in_flight.fetch_add(1, Ordering::SeqCst);
    }

    pub fn job_end(&self) {
        self.in_flight.fetch_sub(1, Ordering::SeqCst);
        if let Ok(mut g) = self.last_done.lock() {
            *g = Instant::now();
        }
        if self.seconds == 0 {
            self.unload();
        }
    }

    /// Adopt a leftover / warm child so `tick` can kill it after idle.
    pub fn attach(&self, child: Child) {
        if let Ok(mut g) = self.child.lock() {
            if let Some(mut old) = g.take() {
                let _ = old.kill();
                let _ = old.wait();
            }
            *g = Some(child);
        }
    }

    pub fn unload(&self) {
        if let Ok(mut g) = self.child.lock() {
            if let Some(mut child) = g.take() {
                let pid = child.id();
                let _ = Command::new("kill")
                    .args(["-TERM", &pid.to_string()])
                    .status();
                let start = Instant::now();
                loop {
                    match child.try_wait() {
                        Ok(Some(_)) => break,
                        Ok(None) if start.elapsed() >= Duration::from_secs(2) => {
                            let _ = child.kill();
                            let _ = child.wait();
                            break;
                        }
                        Ok(None) => thread::sleep(Duration::from_millis(20)),
                        Err(_) => {
                            let _ = child.kill();
                            break;
                        }
                    }
                }
            }
        }
    }

    pub fn tick(&self) {
        if self.in_flight.load(Ordering::SeqCst) > 0 {
            return;
        }
        if !self.loaded() {
            return;
        }
        let idle = self
            .last_done
            .lock()
            .map(|g| g.elapsed())
            .unwrap_or(Duration::ZERO);
        if idle >= Duration::from_secs(self.seconds) {
            self.unload();
        }
    }

    /// Background reaper for long-lived API/MCP. No-op when already unloaded.
    pub fn spawn_watch(self: &Arc<Self>) {
        let this = Arc::clone(self);
        let _ = thread::Builder::new()
            .name("text2mesh-idle-unload".into())
            .spawn(move || loop {
                thread::sleep(Duration::from_secs(5));
                this.tick();
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Stdio;

    fn sleep_child() -> Child {
        Command::new("sleep")
            .arg("30")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("sleep")
    }

    fn alive(pid: u32) -> bool {
        Command::new("kill")
            .args(["-0", &pid.to_string()])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    #[test]
    fn starts_unloaded() {
        let u = IdleUnload::new(120);
        assert!(!u.loaded());
        assert_eq!(u.idle_seconds(), 120);
    }

    #[test]
    fn zero_idle_unloads_on_job_end() {
        let u = IdleUnload::new(0);
        let child = sleep_child();
        let pid = child.id();
        u.attach(child);
        assert!(u.loaded());
        assert!(alive(pid));
        u.job_begin();
        u.job_end();
        assert!(!u.loaded());
        assert!(!alive(pid));
    }

    #[test]
    fn tick_kills_after_idle() {
        let u = IdleUnload::new(0);
        let child = sleep_child();
        let pid = child.id();
        u.attach(child);
        if let Ok(mut g) = u.last_done.lock() {
            *g = Instant::now()
                .checked_sub(Duration::from_secs(1))
                .unwrap_or_else(Instant::now);
        }
        u.tick();
        assert!(!u.loaded());
        assert!(!alive(pid));
    }

    #[test]
    fn in_flight_blocks_unload() {
        let u = IdleUnload::new(0);
        let child = sleep_child();
        let pid = child.id();
        u.attach(child);
        u.job_begin();
        u.tick();
        assert!(u.loaded(), "must not kill a running sidecar");
        assert!(alive(pid));
        u.job_end();
        assert!(!alive(pid));
    }
}
