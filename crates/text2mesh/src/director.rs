//! Job director: validate → persist queued → plan → mock / confirm / honest fail.

use std::path::Path;
use std::time::{Duration, Instant};

use crate::compiler::{compile_view_contract, CompileOpts};
use crate::error::{error_type, Error};
use crate::gates::score_g3_g4;
use crate::hash::sha256_bytes;
use crate::mock_glb::{emit_mock_glb_seeded, has_vertex_color};
use crate::orbit::{decode_png, mock_view_png};
use crate::planner::plan;
use crate::router::{route_job, RouteDecision};
use crate::store::Store;
use crate::system_check::{estimate as estimate_job, probe_from_env};
use crate::types::{
    ArtifactKind, Estimate, JobStatus, JobSubmit, Manifest, ManifestHashes, MaterialMode, MeshJob,
    PlaneId, ProbeSnapshot, SpendPolicy, T2iProviderId, Timings, WaitResult, MANIFEST_SCHEMA,
    WAIT_MAX_S, WAIT_MIN_S,
};

const MAX_UPLOAD_BYTES: u64 = 32 * 1024 * 1024;

pub struct App {
    pub store: Store,
    pub allow_mock: bool,
    pub allow_spend: bool,
    pub allow_ungated: bool,
    /// Injected probe (tests). None → env probe.
    probe: Option<ProbeSnapshot>,
}

impl App {
    pub fn new(store: Store, allow_mock: bool) -> Self {
        Self {
            store,
            allow_mock,
            allow_spend: false,
            allow_ungated: false,
            probe: None,
        }
    }

    pub fn from_env() -> Result<Self, Error> {
        let cfg = crate::config::Config::from_env();
        Ok(Self {
            store: Store::from_env()?,
            allow_mock: cfg.allow_mock,
            allow_spend: cfg.allow_spend,
            allow_ungated: cfg.allow_ungated,
            probe: None,
        })
    }

    pub fn for_test(allow_mock: bool) -> Self {
        Self {
            store: Store::ephemeral().expect("ephemeral store"),
            allow_mock,
            allow_spend: false,
            allow_ungated: true,
            probe: Some(ProbeSnapshot::cpu_only(allow_mock)),
        }
    }

    pub fn with_probe(mut self, probe: ProbeSnapshot) -> Self {
        self.probe = Some(probe);
        self
    }

    pub fn probe(&self) -> ProbeSnapshot {
        self.probe
            .clone()
            .unwrap_or_else(|| probe_from_env(self.allow_mock))
    }

    pub fn spend_policy(&self, spec: &JobSubmit) -> SpendPolicy {
        SpendPolicy {
            allow_spend: spec.allow_spend || self.allow_spend,
            max_usd: spec.max_usd,
        }
    }

    pub fn validate_submit(spec: &JobSubmit) -> Result<(), Error> {
        if !(WAIT_MIN_S..=WAIT_MAX_S).contains(&spec.max_wall_s) {
            return Err(Error::new(
                error_type::SPEC_REJECTED,
                format!(
                    "max_wall_s {} outside {WAIT_MIN_S}..={WAIT_MAX_S} (will not clamp)",
                    spec.max_wall_s
                ),
            ));
        }
        let has_p = spec
            .prompt
            .as_deref()
            .map(str::trim)
            .is_some_and(|s| !s.is_empty());
        let has_i = spec
            .image_path
            .as_deref()
            .map(str::trim)
            .is_some_and(|s| !s.is_empty());
        if has_p == has_i {
            return Err(Error::new(
                error_type::SPEC_REJECTED,
                "exactly one of prompt or image_path is required",
            ));
        }
        if let Some(p) = &spec.prompt {
            let n = p.trim().chars().count();
            if !(1..=4000).contains(&n) {
                return Err(Error::new(
                    error_type::SPEC_REJECTED,
                    "prompt must be 1..=4000 unicode chars after trim",
                ));
            }
        }
        if let Some(k) = &spec.idempotency_key {
            if k.len() > 128 {
                return Err(Error::new(
                    error_type::SPEC_REJECTED,
                    "idempotency_key exceeds 128 bytes",
                ));
            }
        }
        Ok(())
    }

    pub fn submit(&self, spec: JobSubmit) -> Result<MeshJob, Error> {
        Self::validate_submit(&spec)?;

        if let Some(key) = spec.idempotency_key.as_deref() {
            if let Some(existing) = self.store.get_by_idempotency(key)? {
                return Ok(existing);
            }
        }

        let id = ulid::Ulid::new().to_string();
        let mut job = MeshJob::from_submit(id, &spec);
        job.allow_spend = spec.allow_spend || self.allow_spend;
        self.store.create(&job)?;

        match route_job(&spec) {
            RouteDecision::Analytic => {
                return self.fail(
                    job,
                    Error::new(
                        error_type::ANALYTIC_UNAVAILABLE,
                        "Cadre is not configured (set TEXT2MESH_CADRE_URL or TEXT2MESH_CADRE_BIN)",
                    )
                    .with_hint("dimensioned prompts stay on Cadre; pass --allow-neural-cad to force View Contract"),
                );
            }
            RouteDecision::Native => {
                return self.fail(
                    job,
                    Error::new(
                        error_type::NOT_CONFIGURED,
                        "native text-3D is opt-in and no native plane is wired in S7",
                    ),
                );
            }
            RouteDecision::Image | RouteDecision::ViewContract => {}
        }

        if let Some(path) = spec.image_path.as_deref() {
            let p = Path::new(path);
            if !p.is_file() {
                return self.fail(
                    job,
                    Error::new(
                        error_type::NOT_CONFIGURED,
                        format!("image file not found: {path}"),
                    )
                    .with_hint("pass an existing PNG/JPEG path"),
                );
            }
            let meta = std::fs::metadata(p)?;
            if meta.len() > MAX_UPLOAD_BYTES {
                return self.fail(
                    job,
                    Error::new(error_type::SPEC_REJECTED, "image exceeds 32 MiB compressed"),
                );
            }
            let bytes = std::fs::read(p)?;
            let hash = sha256_bytes(&bytes);
            job.input.image_hash_raw = Some(hash.clone());
            job.input.image_hash_conditioned = Some(hash);
            let dest = self
                .store
                .write_artifact(&job.id, "input/original.bin", &bytes)?;
            let _ = dest;
            self.store
                .write_artifact(&job.id, "input/conditioned.png", &bytes)?;
        }

        self.plan_and_run(job, &spec)
    }

    fn plan_and_run(&self, mut job: MeshJob, spec: &JobSubmit) -> Result<MeshJob, Error> {
        let probe = self.probe();
        let spend = self.spend_policy(spec);
        match plan(spec, &probe, &spend) {
            Err(d) if d.error_type == error_type::SPEND_GATED => {
                job.status = JobStatus::NeedsConfirm;
                job.error = Some(d.into());
                job.touch();
                self.store.update(&job)?;
                Ok(job)
            }
            Err(d) => {
                job.status = JobStatus::Failed;
                job.error = Some(d.into());
                job.touch();
                self.store.update(&job)?;
                Ok(job)
            }
            Ok(choice) => {
                job.plane = Some(choice.plane);
                job.compute.actual = Some(choice.plane);
                job.degrades.extend(choice.degrades);
                if let Some(q) = choice.quality_rewrite {
                    job.quality = q;
                }
                if choice.plane == PlaneId::LocalMock {
                    let job = if spec.prompt.as_deref().is_some_and(|s| !s.trim().is_empty()) {
                        self.synthesize_text(job, spec)?
                    } else {
                        job
                    };
                    if job.status == JobStatus::Failed {
                        return Ok(job);
                    }
                    self.run_mock(job, spec)
                } else {
                    self.fail(
                        job,
                        Error::new(
                            error_type::NOT_CONFIGURED,
                            format!(
                                "plane {} is not wired in S0–S4 (sidecar/remote adapters later)",
                                choice.plane.as_str()
                            ),
                        )
                        .with_hint(
                            "use --compute local --provider local.mock, or TEXT2MESH_ALLOW_MOCK=1",
                        ),
                    )
                }
            }
        }
    }

    fn synthesize_text(&self, mut job: MeshJob, spec: &JobSubmit) -> Result<MeshJob, Error> {
        let prompt = spec.prompt.as_deref().unwrap_or("").trim();
        let contract = compile_view_contract(
            prompt,
            CompileOpts {
                quality: spec.quality,
                camera_preset: spec.camera_preset,
                family_seed: spec.seed.unwrap_or(42),
                t2i_provider: T2iProviderId::Mock,
            },
        )?;
        job.input.contract_id = Some(contract.contract_id.clone());
        let bytes = serde_json::to_vec_pretty(&contract)?;
        let cpath = self
            .store
            .write_artifact(&job.id, "contract.json", &bytes)?;
        job.artifacts.contract = Some(cpath.to_string_lossy().into_owned());

        let mut decoded = Vec::new();
        for cam in &contract.camera_ring.cameras {
            let png = mock_view_png(&contract, &cam.id)?;
            let rel = format!("views/{}.png", cam.id);
            let p = self.store.write_artifact(&job.id, &rel, &png)?;
            job.artifacts.views.push(p.to_string_lossy().into_owned());
            decoded.push((cam.id.clone(), decode_png(&png)?));
        }

        if !self.allow_ungated {
            return self.fail(
                job,
                Error::new(
                    error_type::FEATURE_OFF,
                    "G0–G2 need a CLIP encoder; set TEXT2MESH_ALLOW_UNGATED=1 to run G3/G4 only",
                )
                .with_hint("CI / mock text path uses ALLOW_UNGATED; not an M3 claim"),
            );
        }
        job.degrades.push("gate.encoder_missing".into());
        let scores = score_g3_g4(&contract, &decoded);
        if !scores.failed.is_empty() {
            let specific = scores.failed[0].clone();
            return self.fail(
                job,
                Error::new(error_type::VIEW_CONSISTENCY, "required view failed G3/G4")
                    .with_also(scores.failed)
                    .with_hint(specific),
            );
        }
        job.stage = Some("gate".into());
        job.touch();
        self.store.update(&job)?;
        Ok(job)
    }

    fn run_mock(&self, mut job: MeshJob, spec: &JobSubmit) -> Result<MeshJob, Error> {
        if job.cancel_requested {
            job.status = JobStatus::Cancelled;
            job.touch();
            self.store.update(&job)?;
            return Ok(job);
        }
        job.status = JobStatus::Running;
        job.stage = Some("export".into());
        job.pct = 50;
        job.touch();
        self.store.update(&job)?;

        let mut input = Vec::new();
        if let Some(p) = &job.input.prompt {
            input.extend_from_slice(p.as_bytes());
        }
        if let Some(h) = &job.input.image_hash_conditioned {
            input.extend_from_slice(h.as_bytes());
        }
        let glb = emit_mock_glb_seeded(&input, spec.seed.unwrap_or(0));
        if !has_vertex_color(&glb) {
            return self.fail(
                job,
                Error::new(
                    error_type::EXPORT_MATERIALS_MISSING,
                    "mock GLB missing COLOR_0",
                ),
            );
        }
        let hash = sha256_bytes(&glb);
        let glb_path = self.store.write_artifact(&job.id, "artifact.glb", &glb)?;
        self.store
            .write_artifact(&job.id, "artifact.glb.sha256", hash.as_bytes())?;

        job.status = JobStatus::Degraded;
        job.pct = 100;
        job.stage = Some("export".into());
        job.degrades.push(error_type::EXPORT_MATERIAL_MODE.into());
        job.spend.actual_usd = Some(0.0);
        job.spend.estimated_usd = Some(0.0);
        job.artifacts.glb = Some(glb_path.to_string_lossy().into_owned());
        job.error = None;

        let manifest = Manifest {
            schema: MANIFEST_SCHEMA.into(),
            job_id: job.id.clone(),
            ok: false,
            status: JobStatus::Degraded,
            plane: Some(PlaneId::LocalMock),
            engine: Some("mock".into()),
            disclaimer: Some("not-a-model".into()),
            material_mode: Some(MaterialMode::VertexColor),
            hashes: ManifestHashes {
                glb: Some(hash),
                job: None,
                contract: None,
            },
            degrades: job.degrades.clone(),
            spend: job.spend.clone(),
            timings: Timings {
                run_ms: Some(1),
                total_ms: Some(1),
                queue_ms: None,
            },
            quality: Some(crate::types::Quality::Preview),
        };
        let man_bytes = serde_json::to_vec_pretty(&manifest)?;
        let man_path = self
            .store
            .write_artifact(&job.id, "manifest.json", &man_bytes)?;
        job.artifacts.manifest = Some(man_path.to_string_lossy().into_owned());
        job.touch();
        self.store.update(&job)?;
        Ok(job)
    }

    fn fail(&self, mut job: MeshJob, err: Error) -> Result<MeshJob, Error> {
        job.status = JobStatus::Failed;
        job.error = Some(err);
        job.touch();
        self.store.update(&job)?;
        Ok(job)
    }

    pub fn status(&self, job_id: &str) -> Result<MeshJob, Error> {
        self.store
            .get(job_id)?
            .ok_or_else(|| Error::not_found(job_id))
    }

    pub fn wait(&self, job_id: &str, timeout_s: u64) -> Result<WaitResult, Error> {
        if !(WAIT_MIN_S..=WAIT_MAX_S).contains(&timeout_s) {
            return Err(Error::new(
                error_type::SPEC_REJECTED,
                format!("timeout_s {timeout_s} outside {WAIT_MIN_S}..={WAIT_MAX_S}"),
            ));
        }
        self.wait_duration(job_id, Duration::from_secs(timeout_s))
    }

    pub fn wait_duration(&self, job_id: &str, timeout: Duration) -> Result<WaitResult, Error> {
        let start = Instant::now();
        loop {
            let job = self.status(job_id)?;
            if job.status.is_terminal() {
                return Ok(WaitResult {
                    ok: true,
                    job,
                    wait_timed_out: false,
                    error_type: None,
                });
            }
            if start.elapsed() >= timeout {
                return Ok(WaitResult {
                    ok: true,
                    job,
                    wait_timed_out: true,
                    error_type: Some(error_type::WAIT_TIMEOUT.into()),
                });
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    pub async fn wait_async(&self, job_id: &str, timeout_s: u64) -> Result<WaitResult, Error> {
        if !(WAIT_MIN_S..=WAIT_MAX_S).contains(&timeout_s) {
            return Err(Error::new(
                error_type::SPEC_REJECTED,
                format!("timeout_s {timeout_s} outside {WAIT_MIN_S}..={WAIT_MAX_S}"),
            ));
        }
        let start = tokio::time::Instant::now();
        let timeout = Duration::from_secs(timeout_s);
        loop {
            let job = self.status(job_id)?;
            if job.status.is_terminal() {
                return Ok(WaitResult {
                    ok: true,
                    job,
                    wait_timed_out: false,
                    error_type: None,
                });
            }
            if start.elapsed() >= timeout {
                return Ok(WaitResult {
                    ok: true,
                    job,
                    wait_timed_out: true,
                    error_type: Some(error_type::WAIT_TIMEOUT.into()),
                });
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    }

    pub fn cancel(&self, job_id: &str) -> Result<MeshJob, Error> {
        let mut job = self.status(job_id)?;
        if job.status.is_terminal() {
            return Ok(job);
        }
        job.cancel_requested = true;
        if job.plane == Some(PlaneId::LocalMock)
            || job.status == JobStatus::Queued
            || job.status == JobStatus::NeedsConfirm
            || job.status == JobStatus::Running
        {
            job.status = JobStatus::Cancelled;
            job.error = Some(Error::new(error_type::CANCELLED, "cancelled"));
        }
        job.touch();
        self.store.update(&job)?;
        Ok(job)
    }

    pub fn confirm(&self, job_id: &str) -> Result<MeshJob, Error> {
        let mut job = self.status(job_id)?;
        if job.status != JobStatus::NeedsConfirm {
            return Err(Error::new(
                error_type::SPEC_REJECTED,
                format!("job {job_id} is not needs_confirm"),
            ));
        }
        job.allow_spend = true;
        let spec = JobSubmit {
            prompt: job.input.prompt.clone(),
            image_path: job.input.image_path.clone(),
            route: job.route,
            quality: job.quality,
            compute: job.compute.mode,
            provider: job.compute.provider,
            prefer_device: job.compute.prefer_device,
            seed: job.seed,
            camera_preset: job.camera_preset,
            allow_spend: true,
            allow_neural_cad: job.allow_neural_cad,
            allow_native_text: job.allow_native_text,
            license_override: job.license_override.clone(),
            max_usd: job.budget.max_usd,
            max_credits: job.budget.max_credits,
            max_wall_s: job.budget.max_wall_s,
            idempotency_key: job.idempotency_key.clone(),
            export: job.export.clone(),
            job_id: Some(job.id.clone()),
        };
        job.error = None;
        job.touch();
        self.store.update(&job)?;
        self.plan_and_run(job, &spec)
    }

    pub fn artifact(
        &self,
        job_id: &str,
        kind: ArtifactKind,
    ) -> Result<(std::path::PathBuf, String, u64, &'static str), Error> {
        let job = self.status(job_id)?;
        if !job.status.has_artifact() && kind != ArtifactKind::Log {
            return Err(Error::new(
                error_type::EXPORT_NOT_READY,
                format!(
                    "job is {} (need succeeded or degraded)",
                    status_name(job.status)
                ),
            ));
        }
        let name = match kind {
            ArtifactKind::Glb => "artifact.glb",
            ArtifactKind::Manifest => "manifest.json",
            ArtifactKind::Contract => "contract.json",
            ArtifactKind::View => "views",
            ArtifactKind::Log => "log.stderr.txt",
        };
        let path = self.store.artifact_path(job_id, name);
        if !path.is_file() {
            return Err(Error::new(
                error_type::EXPORT_NOT_READY,
                format!("{name} is not on disk"),
            ));
        }
        let bytes = std::fs::read(&path)?;
        let media = match kind {
            ArtifactKind::Glb => "model/gltf-binary",
            ArtifactKind::Manifest | ArtifactKind::Contract => "application/json",
            ArtifactKind::View => "image/png",
            ArtifactKind::Log => "text/plain",
        };
        Ok((path, sha256_bytes(&bytes), bytes.len() as u64, media))
    }

    pub fn list(&self, status: Option<JobStatus>, limit: u32) -> Result<Vec<MeshJob>, Error> {
        self.store.list(status, limit)
    }

    pub fn estimate(&self, spec: &JobSubmit) -> Estimate {
        estimate_job(spec, &self.probe(), &self.spend_policy(spec))
    }

    pub fn watchdog_tick(&self) -> Result<Vec<String>, Error> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        self.store.watchdog_tick(now)
    }
}

fn status_name(s: JobStatus) -> &'static str {
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

/// Compile a View Contract (S5).
pub fn compile_prompt(prompt: &str, spec: &JobSubmit) -> Result<crate::ViewContract, Error> {
    compile_view_contract(
        prompt,
        CompileOpts {
            quality: spec.quality,
            camera_preset: spec.camera_preset,
            family_seed: spec.seed.unwrap_or(42),
            t2i_provider: T2iProviderId::Mock,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ComputeMode, JobSubmit, Quality};

    #[test]
    fn text_prompt_ungated_mock_degraded() {
        let app = App::for_test(true);
        let job = app
            .submit(JobSubmit {
                prompt: Some("a red fox wearing a yellow raincoat".into()),
                compute: ComputeMode::Local,
                provider: Some(PlaneId::LocalMock),
                quality: Quality::Preview,
                ..JobSubmit::default()
            })
            .unwrap();
        assert_eq!(job.status, JobStatus::Degraded);
        assert!(job.artifacts.contract.is_some());
        assert_eq!(job.artifacts.views.len(), 4);
        assert!(job.degrades.iter().any(|d| d == "gate.encoder_missing"));
    }

    #[test]
    fn analytic_without_cadre_refuses() {
        let app = App::for_test(true);
        let job = app
            .submit(JobSubmit {
                prompt: Some("box 20x10x5 mm".into()),
                compute: ComputeMode::Local,
                provider: Some(PlaneId::LocalMock),
                ..JobSubmit::default()
            })
            .unwrap();
        assert_eq!(job.status, JobStatus::Failed);
        assert_eq!(
            job.error.as_ref().unwrap().error_type,
            error_type::ANALYTIC_UNAVAILABLE
        );
    }

    fn write_dot() -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::TempDir::new().unwrap();
        let p = dir.path().join("dot.png");
        std::fs::write(&p, crate::types::minimal_png_1x1()).unwrap();
        (dir, p)
    }

    #[test]
    fn submit_rejects_wall_under_30() {
        let app = App::for_test(true);
        let err = app
            .submit(JobSubmit {
                prompt: Some("fox".into()),
                max_wall_s: 10,
                ..JobSubmit::default()
            })
            .unwrap_err();
        assert_eq!(err.error_type, error_type::SPEC_REJECTED);
        assert!(err.message.contains("max_wall_s"));
    }

    #[test]
    fn job_json_roundtrip_local_mock() {
        let app = App::for_test(true);
        let (_tmp, png) = write_dot();
        let job = app
            .submit(JobSubmit {
                image_path: Some(png.to_string_lossy().into_owned()),
                compute: ComputeMode::Local,
                provider: Some(PlaneId::LocalMock),
                ..JobSubmit::default()
            })
            .unwrap();
        assert_eq!(job.status, JobStatus::Degraded);
        assert!(job.degrades.iter().any(|d| d == "export.material_mode"));
        let v = serde_json::to_value(&job).unwrap();
        assert_ne!(v["status"], "pending");
        assert_eq!(v["status"], "degraded");
        let back: MeshJob = serde_json::from_value(v).unwrap();
        assert_eq!(back.status, JobStatus::Degraded);
        assert_eq!(back.plane, Some(PlaneId::LocalMock));
        let man_path = back.artifacts.manifest.as_ref().unwrap();
        let man: Manifest =
            serde_json::from_str(&std::fs::read_to_string(man_path).unwrap()).unwrap();
        assert!(!man.ok);
        assert_eq!(man.disclaimer.as_deref(), Some("not-a-model"));
        assert_eq!(man.engine.as_deref(), Some("mock"));
        assert_eq!(man.material_mode, Some(MaterialMode::VertexColor));
    }

    #[test]
    fn missing_image_fails_not_configured() {
        let app = App::for_test(true);
        let job = app
            .submit(JobSubmit {
                image_path: Some("/no/such/dot.png".into()),
                compute: ComputeMode::Local,
                provider: Some(PlaneId::LocalMock),
                ..JobSubmit::default()
            })
            .unwrap();
        assert_eq!(job.status, JobStatus::Failed);
        assert_eq!(job.error.unwrap().error_type, "not_configured");
    }

    #[test]
    fn cancel_mock_immediate() {
        let app = App::for_test(true);
        let job = app
            .submit(JobSubmit {
                prompt: Some("a red fox".into()),
                compute: ComputeMode::Local,
                provider: Some(PlaneId::LocalMock),
                ..JobSubmit::default()
            })
            .unwrap();
        // already terminal degraded; cancel of a fresh queued job:
        let mut queued = job.clone();
        queued.id = ulid::Ulid::new().to_string();
        queued.status = JobStatus::Queued;
        queued.plane = Some(PlaneId::LocalMock);
        app.store.create(&queued).unwrap();
        let c = app.cancel(&queued.id).unwrap();
        assert_eq!(c.status, JobStatus::Cancelled);
    }

    #[test]
    fn wait_timeout_leaves_job() {
        let app = App::for_test(false);
        let mut job = MeshJob::from_submit(
            ulid::Ulid::new().to_string(),
            &JobSubmit {
                prompt: Some("hold".into()),
                ..JobSubmit::default()
            },
        );
        job.status = JobStatus::Running;
        app.store.create(&job).unwrap();
        let w = app
            .wait_duration(&job.id, Duration::from_millis(40))
            .unwrap();
        assert!(w.wait_timed_out);
        assert!(w.ok);
        assert_eq!(w.error_type.as_deref(), Some("wait.timeout"));
        let still = app.status(&job.id).unwrap();
        assert_eq!(still.status, JobStatus::Running);
    }

    #[test]
    fn wait_timeout_local_goes_failed() {
        let app = App::for_test(false);
        let mut job = MeshJob::from_submit(
            ulid::Ulid::new().to_string(),
            &JobSubmit {
                prompt: Some("hold".into()),
                ..JobSubmit::default()
            },
        );
        job.status = JobStatus::Running;
        job.plane = Some(PlaneId::LocalSidecar);
        job.created_at = "2000-01-01T00:00:00Z".into();
        job.budget.max_wall_s = 30;
        app.store.create(&job).unwrap();
        app.store.watchdog_tick(1_800_000_000).unwrap();
        let got = app.status(&job.id).unwrap();
        assert_eq!(got.status, JobStatus::Failed);
        assert_eq!(got.error.unwrap().error_type, "wait.timeout");
    }
}
