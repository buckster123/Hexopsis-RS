//! Parent half of `meshplane/1` (design §17). Stdio NDJSON. Paths confined to the job dir.

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use crate::error::{error_type, Error};
use crate::types::JobSubmit;

#[derive(Debug)]
pub struct SidecarResult {
    pub glb: Vec<u8>,
    pub engine: String,
}

pub struct SidecarCfg {
    pub wall: Duration,
    pub handshake: Duration,
    pub cancel_grace: Duration,
    pub cancel: Option<Arc<AtomicBool>>,
    pub args: Vec<String>,
}

impl Default for SidecarCfg {
    fn default() -> Self {
        Self {
            wall: Duration::from_secs(1_800),
            handshake: Duration::from_secs(30),
            cancel_grace: Duration::from_secs(30),
            cancel: None,
            args: Vec::new(),
        }
    }
}

pub struct SidecarProbe {
    pub ok: bool,
    pub engine: Option<String>,
    pub protocol: Option<String>,
    pub reason: Option<String>,
}

fn confine(path: &Path, job_dir: &Path) -> Result<PathBuf, Error> {
    let can = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let root = job_dir
        .canonicalize()
        .unwrap_or_else(|_| job_dir.to_path_buf());
    if !can.starts_with(&root) {
        return Err(Error::new(
            error_type::ENGINE_CRASH,
            format!("sidecar path {} is outside the job dir", can.display()),
        ));
    }
    Ok(can)
}

fn resolve_under_job(path: &Path, job_dir: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        job_dir.join(path)
    }
}

fn sigterm(child: &Child) {
    let _ = Command::new("kill")
        .args(["-TERM", &child.id().to_string()])
        .status();
}

fn reap_or_kill(child: &mut Child, grace: Duration) -> Result<(), Error> {
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return Ok(()),
            Ok(None) if start.elapsed() >= grace => {
                let _ = child.kill();
                let _ = child.wait();
                return Ok(());
            }
            Ok(None) => thread::sleep(Duration::from_millis(20)),
            Err(e) => return Err(Error::new(error_type::ENGINE_CRASH, e.to_string())),
        }
    }
}

fn cancelled(flag: &Option<Arc<AtomicBool>>) -> bool {
    flag.as_ref().is_some_and(|f| f.load(Ordering::SeqCst))
}

/// Fast probe: spawn, read handshake, kill. Uses the probe handshake budget.
pub fn probe_sidecar(bin: &Path, handshake: Duration) -> SidecarProbe {
    if !bin.is_file() {
        return SidecarProbe {
            ok: false,
            engine: None,
            protocol: None,
            reason: Some("binary is not a file".into()),
        };
    }
    let mut child = match Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            return SidecarProbe {
                ok: false,
                engine: None,
                protocol: None,
                reason: Some(e.to_string()),
            };
        }
    };
    let stdout = match child.stdout.take() {
        Some(s) => s,
        None => {
            let _ = child.kill();
            return SidecarProbe {
                ok: false,
                engine: None,
                protocol: None,
                reason: Some("stdout missing".into()),
            };
        }
    };
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        if let Some(line) = BufReader::new(stdout).lines().next() {
            let _ = tx.send(line);
        }
    });
    let line = match rx.recv_timeout(handshake) {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => {
            let _ = child.kill();
            let _ = child.wait();
            return SidecarProbe {
                ok: false,
                engine: None,
                protocol: None,
                reason: Some(e.to_string()),
            };
        }
        Err(_) => {
            let _ = child.kill();
            let _ = child.wait();
            return SidecarProbe {
                ok: false,
                engine: None,
                protocol: None,
                reason: Some("no handshake".into()),
            };
        }
    };
    let _ = child.kill();
    let _ = child.wait();
    let v: Value = match serde_json::from_str(&line) {
        Ok(v) => v,
        Err(_) => {
            return SidecarProbe {
                ok: false,
                engine: None,
                protocol: None,
                reason: Some("handshake is not json".into()),
            };
        }
    };
    let proto = v
        .get("protocol")
        .and_then(|p| p.as_str())
        .map(str::to_string);
    let engine = v.get("engine").and_then(|e| e.as_str()).map(str::to_string);
    if proto.as_deref() != Some("meshplane/1") {
        return SidecarProbe {
            ok: false,
            engine,
            protocol: proto,
            reason: Some("protocol mismatch".into()),
        };
    }
    SidecarProbe {
        ok: true,
        engine,
        protocol: proto,
        reason: None,
    }
}

pub fn run_sidecar(
    bin: &Path,
    job_id: &str,
    spec: &JobSubmit,
    job_dir: &Path,
    conditioned: &Path,
    cfg: SidecarCfg,
) -> Result<SidecarResult, Error> {
    if !bin.is_file() {
        return Err(Error::new(
            error_type::NOT_CONFIGURED,
            format!("sidecar binary missing: {}", bin.display()),
        ));
    }
    let scratch = job_dir.join("scratch");
    std::fs::create_dir_all(&scratch)?;
    let out_glb = job_dir.join("artifact.glb");
    let stderr_path = job_dir.join("log.stderr.txt");
    let stderr_file = std::fs::File::create(&stderr_path)?;
    let mut cmd = Command::new(bin);
    for a in &cfg.args {
        cmd.arg(a);
    }
    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::from(stderr_file))
        .current_dir(job_dir)
        .spawn()
        .map_err(|e| Error::new(error_type::NOT_CONFIGURED, e.to_string()))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| Error::new(error_type::ENGINE_CRASH, "sidecar stdout missing"))?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| Error::new(error_type::ENGINE_CRASH, "sidecar stdin missing"))?;

    let (tx, rx) = mpsc::channel::<Result<String, std::io::Error>>();
    thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            if tx.send(line).is_err() {
                break;
            }
        }
    });

    let hs = match rx.recv_timeout(cfg.handshake) {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(Error::new(
                error_type::NOT_CONFIGURED,
                format!("sidecar handshake: {e}"),
            ));
        }
        Err(_) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(Error::new(
                error_type::NOT_CONFIGURED,
                "no meshplane handshake in 30 s",
            ));
        }
    };
    let hs_v: Value = serde_json::from_str(&hs).map_err(|e| {
        let _ = child.kill();
        Error::new(error_type::UNSUPPORTED, e.to_string())
    })?;
    if hs_v.get("protocol").and_then(|p| p.as_str()) != Some("meshplane/1") {
        let _ = child.kill();
        let _ = child.wait();
        return Err(Error::new(
            error_type::UNSUPPORTED,
            format!("sidecar protocol {:?}", hs_v.get("protocol")),
        ));
    }
    let engine = hs_v
        .get("engine")
        .and_then(|e| e.as_str())
        .unwrap_or("sidecar")
        .to_string();

    if cancelled(&cfg.cancel) {
        sigterm(&child);
        reap_or_kill(&mut child, cfg.cancel_grace)?;
        return Err(Error::new(error_type::CANCELLED, "cancelled"));
    }

    let submit = json!({
        "op": "submit",
        "job": {
            "id": job_id,
            "quality": spec.quality,
            "seed": spec.seed,
        },
        "paths": {
            "conditioned": conditioned.display().to_string(),
            "scratch": scratch.display().to_string(),
            "out_glb": out_glb.display().to_string(),
        }
    });
    use std::io::Write;
    writeln!(stdin, "{submit}").map_err(|e| Error::new(error_type::ENGINE_CRASH, e.to_string()))?;
    stdin
        .flush()
        .map_err(|e| Error::new(error_type::ENGINE_CRASH, e.to_string()))?;
    drop(stdin);

    let start = Instant::now();
    let mut glb_path: Option<PathBuf> = None;
    loop {
        if cancelled(&cfg.cancel) {
            sigterm(&child);
            reap_or_kill(&mut child, cfg.cancel_grace)?;
            return Err(Error::new(error_type::CANCELLED, "cancelled"));
        }
        let remaining = cfg.wall.saturating_sub(start.elapsed());
        if remaining.is_zero() {
            let _ = child.kill();
            let _ = child.wait();
            return Err(Error::new(
                error_type::WAIT_TIMEOUT,
                "sidecar exceeded wall",
            ));
        }
        let line = match rx.recv_timeout(remaining.min(Duration::from_millis(200))) {
            Ok(Ok(s)) => s,
            Ok(Err(e)) => {
                let _ = child.kill();
                return Err(Error::new(error_type::ENGINE_CRASH, e.to_string()));
            }
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        };
        if line.trim().is_empty() {
            continue;
        }
        let v: Value = serde_json::from_str(&line)
            .map_err(|e| Error::new(error_type::ENGINE_CRASH, e.to_string()))?;
        match v.get("op").and_then(|o| o.as_str()) {
            Some("progress") | Some("pong") => {}
            Some("fail") => {
                let _ = child.kill();
                let _ = child.wait();
                let et = v
                    .get("error_type")
                    .and_then(|x| x.as_str())
                    .unwrap_or(error_type::ENGINE_CRASH)
                    .to_string();
                let msg = v
                    .get("message")
                    .and_then(|x| x.as_str())
                    .unwrap_or("sidecar fail")
                    .to_string();
                return Err(Error {
                    error_type: et,
                    message: msg,
                    hint: None,
                    also: Vec::new(),
                });
            }
            Some("artifact") => {
                let p = v
                    .get("path")
                    .and_then(|x| x.as_str())
                    .ok_or_else(|| Error::new(error_type::ENGINE_CRASH, "artifact missing path"))?;
                let resolved = resolve_under_job(Path::new(p), job_dir);
                glb_path = Some(confine(&resolved, job_dir)?);
                break;
            }
            _ => {}
        }
    }

    let status = match child.try_wait() {
        Ok(Some(s)) => s,
        Ok(None) => {
            let leftover = cfg.wall.saturating_sub(start.elapsed());
            let deadline = Instant::now() + leftover;
            loop {
                if let Ok(Some(s)) = child.try_wait() {
                    break s;
                }
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(Error::new(
                        error_type::WAIT_TIMEOUT,
                        "sidecar exceeded wall after artifact",
                    ));
                }
                thread::sleep(Duration::from_millis(20));
            }
        }
        Err(e) => return Err(Error::new(error_type::ENGINE_CRASH, e.to_string())),
    };
    if !status.success() {
        return Err(Error::new(
            error_type::ENGINE_CRASH,
            format!("sidecar exit {status}"),
        ));
    }
    let path = glb_path
        .ok_or_else(|| Error::new(error_type::ENGINE_CRASH, "sidecar exited without artifact"))?;
    let glb = std::fs::read(&path)?;
    Ok(SidecarResult { glb, engine })
}

pub fn sidecar_bin_from_env() -> Option<PathBuf> {
    std::env::var("TEXT2MESH_SIDECAR")
        .ok()
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn confine_rejects_escape() {
        let dir = tempfile::tempdir().unwrap();
        let job = dir.path().join("jobs/jid");
        std::fs::create_dir_all(&job).unwrap();
        let evil = dir.path().join("outside.glb");
        std::fs::write(&evil, b"x").unwrap();
        let err = confine(&evil, &job).unwrap_err();
        assert_eq!(err.error_type, error_type::ENGINE_CRASH);
    }
}
