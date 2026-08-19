//! SQLite `jobs.sqlite` + `jobs/<id>/` artefacts. Atomic write = tmp + rename.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use rusqlite::{params, Connection, OptionalExtension};

use crate::error::{error_type, Error};
use crate::types::{rfc3339_to_unix, JobStatus, MeshJob};

const QUEUE_STALE_S: u64 = 60;
const CONFIRM_TTL_S: u64 = 86_400;

pub struct Store {
    root: PathBuf,
    db: Mutex<Connection>,
    _ephemeral: Option<tempfile::TempDir>,
}

impl Store {
    pub fn open(root: impl Into<PathBuf>) -> Result<Self, Error> {
        let root = root.into();
        fs::create_dir_all(&root)?;
        fs::create_dir_all(root.join("jobs"))?;
        let db_path = root.join("jobs.sqlite");
        let db = Connection::open(&db_path)?;
        db.busy_timeout(std::time::Duration::from_millis(5_000))?;
        db.execute_batch(
            r#"
            PRAGMA journal_mode=WAL;
            CREATE TABLE IF NOT EXISTS jobs (
                id TEXT PRIMARY KEY,
                json TEXT NOT NULL,
                status TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                idempotency_key TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_jobs_updated ON jobs(updated_at);
            "#,
        )?;
        let store = Self {
            root,
            db: Mutex::new(db),
            _ephemeral: None,
        };
        store.import_job_dirs()?;
        store.recover_on_open()?;
        Ok(store)
    }

    pub fn ephemeral() -> Result<Self, Error> {
        let tmp = tempfile::TempDir::new()?;
        let mut store = Self::open(tmp.path())?;
        store._ephemeral = Some(tmp);
        Ok(store)
    }

    pub fn from_env() -> Result<Self, Error> {
        match std::env::var("TEXT2MESH_STORE") {
            Ok(s) if s.is_empty() => Self::ephemeral(),
            Ok(s) => Self::open(PathBuf::from(s)),
            Err(_) => {
                let root = dirs::data_dir()
                    .unwrap_or_else(|| {
                        std::env::var("HOME")
                            .map(|h| PathBuf::from(h).join(".local/share"))
                            .unwrap_or_else(|_| PathBuf::from(".").join(".local/share"))
                    })
                    .join("text2mesh");
                Self::open(root)
            }
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>, Error> {
        self.db
            .lock()
            .map_err(|_| Error::new(error_type::INTERNAL, "db lock poisoned"))
    }

    fn import_job_dirs(&self) -> Result<(), Error> {
        let jobs_dir = self.root.join("jobs");
        let entries = match fs::read_dir(&jobs_dir) {
            Ok(e) => e,
            Err(_) => return Ok(()),
        };
        for ent in entries.flatten() {
            let path = ent.path();
            if !path.is_dir() {
                continue;
            }
            let job_json = path.join("job.json");
            if !job_json.is_file() {
                continue;
            }
            let raw = match fs::read_to_string(&job_json) {
                Ok(s) => s,
                Err(_) => continue,
            };
            let job: MeshJob = match serde_json::from_str(&raw) {
                Ok(j) => j,
                Err(_) => continue,
            };
            let db = self.lock()?;
            db.execute(
                "INSERT INTO jobs (id, json, status, created_at, updated_at, idempotency_key)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(id) DO NOTHING",
                params![
                    job.id,
                    raw,
                    status_str(job.status),
                    job.created_at,
                    job.updated_at,
                    job.idempotency_key,
                ],
            )?;
        }
        Ok(())
    }

    fn recover_on_open(&self) -> Result<(), Error> {
        let jobs = self.list(None, 10_000)?;
        for mut job in jobs {
            if job.status == JobStatus::Running && job.plane.map(|p| p.is_local()).unwrap_or(true) {
                job.status = JobStatus::Failed;
                job.error = Some(
                    Error::new(
                        error_type::ENGINE_INTERRUPTED,
                        "local job was running at process start",
                    )
                    .with_hint("sidecar/child did not survive restart"),
                );
                job.touch();
                self.update(&job)?;
            }
        }
        Ok(())
    }

    pub fn create(&self, job: &MeshJob) -> Result<(), Error> {
        let json = serde_json::to_string_pretty(job)?;
        self.job_dir(&job.id)?;
        self.atomic_write(&self.job_json_path(&job.id), json.as_bytes())?;
        let db = self.lock()?;
        db.execute(
            "INSERT INTO jobs (id, json, status, created_at, updated_at, idempotency_key)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                job.id,
                json,
                status_str(job.status),
                job.created_at,
                job.updated_at,
                job.idempotency_key,
            ],
        )?;
        Ok(())
    }

    pub fn get(&self, id: &str) -> Result<Option<MeshJob>, Error> {
        let db = self.lock()?;
        let json: Option<String> = db
            .query_row("SELECT json FROM jobs WHERE id = ?1", [id], |row| {
                row.get(0)
            })
            .optional()?;
        match json {
            Some(s) => Ok(Some(serde_json::from_str(&s)?)),
            None => Ok(None),
        }
    }

    pub fn get_by_idempotency(&self, key: &str) -> Result<Option<MeshJob>, Error> {
        let db = self.lock()?;
        let json: Option<String> = db
            .query_row(
                "SELECT json FROM jobs WHERE idempotency_key = ?1 ORDER BY created_at DESC LIMIT 1",
                [key],
                |row| row.get(0),
            )
            .optional()?;
        match json {
            Some(s) => Ok(Some(serde_json::from_str(&s)?)),
            None => Ok(None),
        }
    }

    pub fn list(&self, status: Option<JobStatus>, limit: u32) -> Result<Vec<MeshJob>, Error> {
        let limit = limit.clamp(1, 100);
        let db = self.lock()?;
        let mut jobs = Vec::new();
        if let Some(st) = status {
            let mut stmt =
                db.prepare("SELECT json FROM jobs WHERE status = ?1 ORDER BY id DESC LIMIT ?2")?;
            let rows = stmt.query_map(params![status_str(st), limit], |row| {
                row.get::<_, String>(0)
            })?;
            for r in rows {
                jobs.push(serde_json::from_str(&r?)?);
            }
        } else {
            let mut stmt = db.prepare("SELECT json FROM jobs ORDER BY id DESC LIMIT ?1")?;
            let rows = stmt.query_map([limit], |row| row.get::<_, String>(0))?;
            for r in rows {
                jobs.push(serde_json::from_str(&r?)?);
            }
        }
        Ok(jobs)
    }

    pub fn update(&self, job: &MeshJob) -> Result<(), Error> {
        let json = serde_json::to_string_pretty(job)?;
        self.atomic_write(&self.job_json_path(&job.id), json.as_bytes())?;
        let db = self.lock()?;
        let n = db.execute(
            "UPDATE jobs SET json = ?1, status = ?2, updated_at = ?3 WHERE id = ?4",
            params![json, status_str(job.status), job.updated_at, job.id],
        )?;
        if n == 0 {
            return Err(Error::not_found(&job.id));
        }
        Ok(())
    }

    pub fn job_dir(&self, id: &str) -> Result<PathBuf, Error> {
        let dir = self.root.join("jobs").join(id);
        fs::create_dir_all(&dir)?;
        Ok(dir)
    }

    pub fn job_json_path(&self, id: &str) -> PathBuf {
        self.root.join("jobs").join(id).join("job.json")
    }

    pub fn artifact_path(&self, id: &str, name: &str) -> PathBuf {
        self.root.join("jobs").join(id).join(name)
    }

    pub fn write_artifact(&self, id: &str, name: &str, bytes: &[u8]) -> Result<PathBuf, Error> {
        let dir = self.job_dir(id)?;
        let path = dir.join(name);
        self.atomic_write(&path, bytes)?;
        Ok(path)
    }

    pub fn atomic_write(&self, path: &Path, bytes: &[u8]) -> Result<(), Error> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut tmp = path.as_os_str().to_os_string();
        tmp.push(".tmp");
        let tmp = PathBuf::from(tmp);
        fs::write(&tmp, bytes)?;
        fs::rename(&tmp, path)?;
        Ok(())
    }

    /// Watchdog tick (design §8.1). `now_unix` is injected for tests.
    pub fn watchdog_tick(&self, now_unix: u64) -> Result<Vec<String>, Error> {
        let jobs = self.list(None, 10_000)?;
        let mut flipped = Vec::new();
        for mut job in jobs {
            let created = rfc3339_to_unix(&job.created_at).unwrap_or(now_unix);
            let age = now_unix.saturating_sub(created);
            let mut change = false;
            match job.status {
                JobStatus::Queued if age > QUEUE_STALE_S => {
                    job.status = JobStatus::Failed;
                    job.error = Some(Error::new(
                        error_type::WATCHDOG_QUEUE,
                        "queued with no worker past queue_stale_secs",
                    ));
                    change = true;
                }
                JobStatus::NeedsConfirm if age > CONFIRM_TTL_S => {
                    job.status = JobStatus::Failed;
                    job.error = Some(Error::new(
                        error_type::SPEND_GATED,
                        "needs_confirm older than confirm_ttl; no POST happened",
                    ));
                    change = true;
                }
                JobStatus::Running
                    if job.plane.map(|p| p.is_local()).unwrap_or(true)
                        && age > job.budget.max_wall_s.max(crate::types::WAIT_MIN_S) =>
                {
                    job.status = JobStatus::Failed;
                    job.error = Some(Error::new(
                        error_type::WAIT_TIMEOUT,
                        "local job exceeded max_wall_s",
                    ));
                    change = true;
                }
                _ => {}
            }
            if change {
                job.touch();
                let id = job.id.clone();
                self.update(&job)?;
                flipped.push(id);
            }
        }
        Ok(flipped)
    }
}

fn status_str(s: JobStatus) -> &'static str {
    match s {
        JobStatus::Queued => "queued",
        JobStatus::NeedsConfirm => "needs_confirm",
        JobStatus::Submitted => "submitted",
        JobStatus::Running => "running",
        JobStatus::WaitingUpstream => "waiting_upstream",
        JobStatus::Succeeded => "succeeded",
        JobStatus::Degraded => "degraded",
        JobStatus::Failed => "failed",
        JobStatus::Cancelled => "cancelled",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ComputeMode, JobSubmit};

    fn sample_job(store: &Store, id: &str) -> MeshJob {
        let spec = JobSubmit {
            prompt: Some("a fox".into()),
            ..JobSubmit::default()
        };
        let mut job = MeshJob::from_submit(id.into(), &spec);
        job.compute.mode = ComputeMode::Local;
        store.create(&job).unwrap();
        job
    }

    #[test]
    fn persist_roundtrip() {
        let store = Store::ephemeral().unwrap();
        let job = sample_job(&store, "01TESTSTORE00000000000000");
        let got = store.get(&job.id).unwrap().unwrap();
        assert_eq!(got.id, job.id);
        assert_eq!(got.status, JobStatus::Queued);
        let listed = store.list(None, 20).unwrap();
        assert_eq!(listed.len(), 1);
    }

    #[test]
    fn atomic_write_ignores_tmp_only() {
        let store = Store::ephemeral().unwrap();
        let dir = store.job_dir("orphan").unwrap();
        fs::write(dir.join("job.json.tmp"), b"{}").unwrap();
        let store2 = Store::open(store.root()).unwrap();
        assert!(store2.get("orphan").unwrap().is_none());
    }

    #[test]
    fn watchdog_queued_flips_failed() {
        let store = Store::ephemeral().unwrap();
        let mut job = sample_job(&store, "01TESTWATCH0000000000000");
        job.created_at = "2000-01-01T00:00:00Z".into();
        store.update(&job).unwrap();
        let flipped = store.watchdog_tick(1_800_000_000).unwrap();
        assert_eq!(flipped, vec![job.id.clone()]);
        let got = store.get(&job.id).unwrap().unwrap();
        assert_eq!(got.status, JobStatus::Failed);
        assert_eq!(got.error.unwrap().error_type, "watchdog.queue");
    }

    #[test]
    fn watchdog_needs_confirm_ttl() {
        let store = Store::ephemeral().unwrap();
        let mut job = sample_job(&store, "01TESTCONFIRM00000000000");
        job.status = JobStatus::NeedsConfirm;
        job.created_at = "2000-01-01T00:00:00Z".into();
        store.update(&job).unwrap();
        store.watchdog_tick(1_800_000_000).unwrap();
        let got = store.get(&job.id).unwrap().unwrap();
        assert_eq!(got.status, JobStatus::Failed);
        assert_eq!(got.error.unwrap().error_type, "spend.gated");
    }

    #[test]
    fn recover_local_running_interrupted() {
        let tmp = tempfile::TempDir::new().unwrap();
        let store = Store::open(tmp.path()).unwrap();
        let mut job = sample_job(&store, "01TESTBOOT00000000000000");
        job.status = JobStatus::Running;
        job.plane = Some(crate::types::PlaneId::LocalSidecar);
        store.update(&job).unwrap();
        drop(store);
        let store = Store::open(tmp.path()).unwrap();
        let got = store.get(&job.id).unwrap().unwrap();
        assert_eq!(got.status, JobStatus::Failed);
        assert_eq!(got.error.unwrap().error_type, "engine.interrupted");
    }
}
