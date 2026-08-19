//! Job director: validate → persist queued → plan → mock / sidecar / confirm / honest fail.

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::compiler::{compile_view_contract, CompileOpts};
use crate::error::{error_type, Error};
use crate::export::{inspect_glb, ExportClass};
use crate::gates::score_g3_g4;
use crate::hash::sha256_bytes;
use crate::idle::IdleUnload;
use crate::mock_glb::emit_mock_glb_seeded;
use crate::orbit::decode_png;
use crate::planner::plan;
use crate::remote::RemoteOutcome;
use crate::remote_meshy::Meshy;
use crate::remote_tripo::Tripo;
use crate::router::{route_job, RouteDecision};
use crate::sidecar::{run_sidecar, sidecar_bin_from_env, SidecarCfg};
use crate::store::Store;
use crate::system_check::{estimate as estimate_job, probe_from_env};
use crate::t2i::{estimate_orbit, synthesize_orbit, view_count, MockT2i, T2iProvider};
use crate::t2i_imaginarium::Imaginarium;
use crate::types::{
    ArtifactKind, Estimate, JobStatus, JobSubmit, Manifest, ManifestHashes, MeshJob, PlaneId,
    ProbeSnapshot, SpendPolicy, Synthesis, T2iProviderId, Timings, WaitResult, MANIFEST_SCHEMA,
    WAIT_MAX_S, WAIT_MIN_S,
};

const MAX_UPLOAD_BYTES: u64 = 32 * 1024 * 1024;

struct FinishGlb<'a> {
    engine: &'a str,
    plane: PlaneId,
    disclaimer: Option<&'a str>,
    sidecar_protocol: Option<&'a str>,
    synthesis: Option<Synthesis>,
}

pub struct App {
    pub store: Store,
    pub allow_mock: bool,
    pub allow_spend: bool,
    pub allow_ungated: bool,
    /// Injected probe (tests). None → env probe.
    probe: Option<ProbeSnapshot>,
    t2i: Option<Box<dyn T2iProvider>>,
    sidecar_bin: Option<PathBuf>,
    sidecar_cancel_grace: Duration,
    cancel: Arc<AtomicBool>,
    meshy: Option<Meshy>,
    tripo: Option<Tripo>,
    idle: Arc<IdleUnload>,
}

impl App {
    pub fn new(store: Store, allow_mock: bool) -> Self {
        Self {
            store,
            allow_mock,
            allow_spend: false,
            allow_ungated: false,
            probe: None,
            t2i: None,
            sidecar_bin: sidecar_bin_from_env(),
            sidecar_cancel_grace: Duration::from_secs(30),
            cancel: Arc::new(AtomicBool::new(false)),
            meshy: None,
            tripo: None,
            idle: Arc::new(IdleUnload::new(120)),
        }
    }

    pub fn from_env() -> Result<Self, Error> {
        let cfg = crate::config::Config::from_env();
        let idle = IdleUnload::from_env();
        idle.spawn_watch();
        Ok(Self {
            store: Store::from_env()?,
            allow_mock: cfg.allow_mock,
            allow_spend: cfg.allow_spend,
            allow_ungated: cfg.allow_ungated,
            probe: None,
            t2i: None,
            sidecar_bin: cfg.sidecar.or_else(sidecar_bin_from_env),
            sidecar_cancel_grace: Duration::from_secs(30),
            cancel: Arc::new(AtomicBool::new(false)),
            meshy: Meshy::from_env()?,
            tripo: Tripo::from_env()?,
            idle,
        })
    }

    pub fn for_test(allow_mock: bool) -> Self {
        Self {
            store: Store::ephemeral().expect("ephemeral store"),
            allow_mock,
            allow_spend: false,
            allow_ungated: true,
            probe: Some(ProbeSnapshot::cpu_only(allow_mock)),
            t2i: None,
            sidecar_bin: None,
            sidecar_cancel_grace: Duration::from_millis(100),
            cancel: Arc::new(AtomicBool::new(false)),
            meshy: None,
            tripo: None,
            idle: Arc::new(IdleUnload::new(0)),
        }
    }

    pub fn sidecar_loaded(&self) -> bool {
        self.idle.loaded()
    }

    pub fn with_probe(mut self, probe: ProbeSnapshot) -> Self {
        self.probe = Some(probe);
        self
    }

    pub fn with_t2i(mut self, t2i: Box<dyn T2iProvider>) -> Self {
        self.t2i = Some(t2i);
        self
    }

    pub fn with_sidecar(mut self, bin: PathBuf) -> Self {
        self.sidecar_bin = Some(bin);
        self
    }

    pub fn with_meshy(mut self, meshy: Meshy) -> Self {
        self.meshy = Some(meshy);
        self
    }

    pub fn with_tripo(mut self, tripo: Tripo) -> Self {
        self.tripo = Some(tripo);
        self
    }

    pub fn cancel_flag(&self) -> Arc<AtomicBool> {
        self.cancel.clone()
    }

    fn is_test(&self) -> bool {
        self.probe.is_some()
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
                let native_remote = spec.allow_native_text && choice.plane.is_remote();
                let job = if spec.prompt.as_deref().is_some_and(|s| !s.trim().is_empty())
                    && !native_remote
                {
                    self.synthesize_text(job, spec, choice.plane)?
                } else {
                    job
                };
                if job.status.is_terminal() || job.status == JobStatus::NeedsConfirm {
                    return Ok(job);
                }
                match choice.plane {
                    PlaneId::LocalMock => self.run_mock(job, spec),
                    PlaneId::LocalSidecar => self.run_sidecar_plane(job, spec),
                    PlaneId::RemoteMeshy | PlaneId::RemoteTripo => {
                        self.run_remote(job, spec, choice.plane)
                    }
                    other => self.fail(
                        job,
                        Error::new(
                            error_type::NOT_CONFIGURED,
                            format!("plane {} is not wired (colony is post-S10)", other.as_str()),
                        )
                        .with_hint(
                            "use --compute local --provider local.mock, TEXT2MESH_SIDECAR, or a Meshy/Tripo key",
                        ),
                    ),
                }
            }
        }
    }

    fn wants_paid_t2i(&self, plane: PlaneId) -> bool {
        if let Some(p) = &self.t2i {
            return p.id() == T2iProviderId::Imaginarium;
        }
        if self.is_test() {
            return false;
        }
        plane != PlaneId::LocalMock && Imaginarium::from_env().is_some_and(|i| i.health())
    }

    fn paid_provider(&self) -> Result<Imaginarium, Error> {
        Imaginarium::from_env()
            .ok_or_else(|| Error::new(error_type::T2I_UNAVAILABLE, "Imaginarium is not configured"))
    }

    fn synthesize_text(
        &self,
        mut job: MeshJob,
        spec: &JobSubmit,
        plane: PlaneId,
    ) -> Result<MeshJob, Error> {
        let prompt = spec.prompt.as_deref().unwrap_or("").trim();
        let paid = self.wants_paid_t2i(plane);
        let t2i_id = if paid {
            T2iProviderId::Imaginarium
        } else {
            T2iProviderId::Mock
        };
        let contract = compile_view_contract(
            prompt,
            CompileOpts {
                quality: spec.quality,
                camera_preset: spec.camera_preset,
                family_seed: spec.seed.unwrap_or(42),
                t2i_provider: t2i_id,
            },
        )?;
        job.input.contract_id = Some(contract.contract_id.clone());
        let bytes = serde_json::to_vec_pretty(&contract)?;
        let cpath = self
            .store
            .write_artifact(&job.id, "contract.json", &bytes)?;
        job.artifacts.contract = Some(cpath.to_string_lossy().into_owned());

        if paid {
            let n_views = view_count(spec.quality);
            let cost = {
                let got = if let Some(p) = &self.t2i {
                    estimate_orbit(p.as_ref(), n_views)
                } else {
                    match self.paid_provider() {
                        Ok(im) => estimate_orbit(&im, n_views),
                        Err(e) => return self.fail(job, e),
                    }
                };
                match got {
                    Ok(c) => c,
                    Err(e) => return self.fail(job, e),
                }
            };
            job.spend.estimated_usd = Some(cost.usd);
            job.spend.usd_uncertain = cost.usd_uncertain;
            if cost.usd > 0.0 && !self.spend_policy(spec).allow_spend {
                job.status = JobStatus::NeedsConfirm;
                job.error = Some(
                    Error::new(
                        error_type::SPEND_GATED,
                        format!("t2i estimate usd={} > 0 and allow_spend is false", cost.usd),
                    )
                    .with_hint("call estimate, then resubmit with --allow-spend"),
                );
                job.touch();
                self.store.update(&job)?;
                return Ok(job);
            }
            if cost.usd > self.spend_policy(spec).max_usd {
                return self.fail(
                    job,
                    Error::new(
                        error_type::SPEND_ESTIMATE_EXCEEDED,
                        format!(
                            "t2i estimate usd={} exceeds max_usd={}",
                            cost.usd,
                            self.spend_policy(spec).max_usd
                        ),
                    ),
                );
            }
        } else if plane != PlaneId::LocalMock && !self.allow_mock {
            return self.fail(
                job,
                Error::new(
                    error_type::T2I_UNAVAILABLE,
                    "no live T2I provider (Imaginarium) and mock T2I is not allowed",
                )
                .with_hint("TEXT2MESH_ALLOW_SPEND=1 with Imaginarium, or TEXT2MESH_ALLOW_MOCK=1"),
            );
        }

        job.status = JobStatus::Running;
        job.stage = Some("t2i".into());
        job.touch();
        self.store.update(&job)?;

        let orbit = if let Some(p) = &self.t2i {
            synthesize_orbit(p.as_ref(), &contract)?
        } else if paid {
            synthesize_orbit(&self.paid_provider()?, &contract)?
        } else {
            synthesize_orbit(&MockT2i, &contract)?
        };
        if orbit.independent_t2i {
            job.degrades.push("t2i.i2i".into());
        }
        job.spend.actual_usd = Some(job.spend.actual_usd.unwrap_or(0.0) + orbit.usd);

        let mut decoded = Vec::new();
        for (cam_id, png) in &orbit.views {
            let rel = format!("views/{cam_id}.png");
            let p = self.store.write_artifact(&job.id, &rel, png)?;
            job.artifacts.views.push(p.to_string_lossy().into_owned());
            decoded.push((cam_id.clone(), decode_png(png)?));
        }

        let canonical = contract.subject_lock.canonical_view_id.clone();
        if let Some((_, png)) = orbit
            .views
            .iter()
            .find(|(id, _)| *id == canonical)
            .or_else(|| orbit.views.first())
        {
            let dest = self
                .store
                .write_artifact(&job.id, "input/conditioned.png", png)?;
            job.input.image_hash_conditioned = Some(sha256_bytes(png));
            let _ = dest;
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
        self.finish_glb(
            job,
            &glb,
            FinishGlb {
                engine: "mock",
                plane: PlaneId::LocalMock,
                disclaimer: Some("not-a-model"),
                sidecar_protocol: None,
                synthesis: Some(Synthesis::HeroOrbit),
            },
        )
    }

    fn run_sidecar_plane(&self, mut job: MeshJob, spec: &JobSubmit) -> Result<MeshJob, Error> {
        if job.cancel_requested || self.cancel.load(std::sync::atomic::Ordering::SeqCst) {
            job.status = JobStatus::Cancelled;
            job.touch();
            self.store.update(&job)?;
            return Ok(job);
        }
        let bin = self.sidecar_bin.clone().or_else(sidecar_bin_from_env);
        let Some(bin) = bin else {
            return self.fail(
                job,
                Error::new(error_type::NOT_CONFIGURED, "TEXT2MESH_SIDECAR is missing")
                    .with_hint("point TEXT2MESH_SIDECAR at a meshplane/1 binary"),
            );
        };
        let job_dir = self.store.job_dir(&job.id)?;
        let conditioned = self.ensure_conditioned(&job)?;
        job.status = JobStatus::Running;
        job.stage = Some("form".into());
        job.pct = 40;
        job.touch();
        self.store.update(&job)?;

        let wall = Duration::from_secs(spec.max_wall_s.max(WAIT_MIN_S));
        let cfg = SidecarCfg {
            wall,
            handshake: Duration::from_secs(30),
            cancel_grace: self.sidecar_cancel_grace,
            cancel: Some(self.cancel.clone()),
            args: Vec::new(),
        };
        self.idle.job_begin();
        let result = match run_sidecar(&bin, &job.id, spec, &job_dir, &conditioned, cfg) {
            Ok(r) => {
                self.idle.job_end();
                r
            }
            Err(e) if e.error_type == error_type::CANCELLED => {
                self.idle.job_end();
                job.status = JobStatus::Cancelled;
                job.error = Some(e);
                job.touch();
                self.store.update(&job)?;
                return Ok(job);
            }
            Err(e) => {
                self.idle.job_end();
                return self.fail(job, e);
            }
        };
        self.finish_glb(
            job,
            &result.glb,
            FinishGlb {
                engine: &result.engine,
                plane: PlaneId::LocalSidecar,
                disclaimer: None,
                sidecar_protocol: Some("meshplane/1"),
                synthesis: Some(Synthesis::HeroOrbit),
            },
        )
    }

    fn ensure_conditioned(&self, job: &MeshJob) -> Result<PathBuf, Error> {
        let p = self.store.artifact_path(&job.id, "input/conditioned.png");
        if p.is_file() {
            return Ok(p);
        }
        let hero = job
            .artifacts
            .views
            .iter()
            .find(|v| v.ends_with("hero.png"))
            .or_else(|| {
                job.artifacts
                    .views
                    .iter()
                    .find(|v| v.ends_with("front.png"))
            })
            .or_else(|| job.artifacts.views.first());
        if let Some(v) = hero {
            let bytes = std::fs::read(v)?;
            return self
                .store
                .write_artifact(&job.id, "input/conditioned.png", &bytes);
        }
        Err(Error::new(
            error_type::NOT_CONFIGURED,
            "sidecar needs input/conditioned.png (image or a synthesized hero view)",
        ))
    }

    fn run_remote(
        &self,
        mut job: MeshJob,
        spec: &JobSubmit,
        plane: PlaneId,
    ) -> Result<MeshJob, Error> {
        job.status = JobStatus::Submitted;
        job.stage = Some("submit".into());
        job.touch();
        self.store.update(&job)?;

        let native =
            spec.allow_native_text && spec.prompt.as_deref().is_some_and(|s| !s.trim().is_empty());
        let outcome = if native {
            let prompt = spec.prompt.as_deref().unwrap_or("").trim();
            match plane {
                PlaneId::RemoteMeshy => match self.meshy_or_err() {
                    Ok(m) => m.run_text(prompt),
                    Err(e) => return self.fail(job, e),
                },
                PlaneId::RemoteTripo => match self.tripo_or_err() {
                    Ok(t) => t.run_text(prompt),
                    Err(e) => return self.fail(job, e),
                },
                _ => unreachable!(),
            }
        } else {
            let png_path = match self.ensure_conditioned(&job) {
                Ok(p) => p,
                Err(e) => return self.fail(job, e),
            };
            let png = std::fs::read(&png_path)?;
            match plane {
                PlaneId::RemoteMeshy => match self.meshy_or_err() {
                    Ok(m) => m.run_image(&png),
                    Err(e) => return self.fail(job, e),
                },
                PlaneId::RemoteTripo => match self.tripo_or_err() {
                    Ok(t) => t.run_image(&png),
                    Err(e) => return self.fail(job, e),
                },
                _ => unreachable!(),
            }
        };
        match outcome {
            Ok(RemoteOutcome::Waiting { upstream_id }) => {
                job.status = JobStatus::WaitingUpstream;
                job.upstream_id = Some(upstream_id);
                job.error = Some(Error::new(
                    error_type::WAIT_TIMEOUT,
                    "remote poll window expired; upstream_id retained",
                ));
                job.touch();
                self.store.update(&job)?;
                Ok(job)
            }
            Ok(RemoteOutcome::Done(art)) => {
                job.upstream_id = Some(art.upstream_id.clone());
                if let Some(usd) = art.usd {
                    job.spend.actual_usd = Some(usd);
                }
                job.status = JobStatus::Running;
                self.finish_glb(
                    job,
                    &art.glb,
                    FinishGlb {
                        engine: &art.engine,
                        plane,
                        disclaimer: None,
                        sidecar_protocol: None,
                        synthesis: if native {
                            Some(Synthesis::NativePassthrough)
                        } else {
                            Some(Synthesis::HeroOrbit)
                        },
                    },
                )
            }
            Err(e) => self.fail(job, e),
        }
    }

    fn meshy_or_err(&self) -> Result<&Meshy, Error> {
        self.meshy.as_ref().ok_or_else(|| {
            Error::new(error_type::NOT_CONFIGURED, "MESHY_API_KEY is missing")
                .with_hint("set MESHY_API_KEY; we never POST without it")
        })
    }

    fn tripo_or_err(&self) -> Result<&Tripo, Error> {
        self.tripo.as_ref().ok_or_else(|| {
            Error::new(error_type::NOT_CONFIGURED, "TRIPO_API_KEY is missing")
                .with_hint("set TRIPO_API_KEY; we never POST without it")
        })
    }

    fn finish_glb(
        &self,
        mut job: MeshJob,
        glb: &[u8],
        meta: FinishGlb<'_>,
    ) -> Result<MeshJob, Error> {
        let report = match inspect_glb(glb) {
            Ok(r) => r,
            Err(e) => return self.fail(job, e),
        };
        if report.class == ExportClass::Missing {
            return self.fail(
                job,
                Error::new(
                    error_type::EXPORT_MATERIALS_MISSING,
                    "GLB has default-only factors, no COLOR_0, and no textures",
                ),
            );
        }
        let mockish = meta.plane == PlaneId::LocalMock || meta.engine == "mock";
        let pbr = report.class == ExportClass::UvAtlas && !mockish;
        let status = if pbr {
            JobStatus::Succeeded
        } else {
            JobStatus::Degraded
        };
        let hash = sha256_bytes(glb);
        let glb_path = self.store.write_artifact(&job.id, "artifact.glb", glb)?;
        self.store
            .write_artifact(&job.id, "artifact.glb.sha256", hash.as_bytes())?;

        job.status = status;
        job.pct = 100;
        job.stage = Some("export".into());
        if !pbr
            && !job
                .degrades
                .iter()
                .any(|d| d == error_type::EXPORT_MATERIAL_MODE)
        {
            job.degrades.push(error_type::EXPORT_MATERIAL_MODE.into());
        }
        if job.spend.actual_usd.is_none() {
            job.spend.actual_usd = Some(0.0);
        }
        if job.spend.estimated_usd.is_none() {
            job.spend.estimated_usd = Some(job.spend.actual_usd.unwrap_or(0.0));
        }
        job.artifacts.glb = Some(glb_path.to_string_lossy().into_owned());
        job.error = None;

        let manifest = Manifest {
            schema: MANIFEST_SCHEMA.into(),
            job_id: job.id.clone(),
            ok: status == JobStatus::Succeeded,
            status,
            plane: Some(meta.plane),
            engine: Some(meta.engine.into()),
            disclaimer: meta.disclaimer.map(str::to_string),
            material_mode: report.material_mode,
            hashes: ManifestHashes {
                glb: Some(hash),
                job: None,
                contract: job.input.contract_id.clone(),
            },
            degrades: job.degrades.clone(),
            spend: job.spend.clone(),
            timings: Timings {
                run_ms: Some(1),
                total_ms: Some(1),
                queue_ms: None,
            },
            quality: Some(if meta.plane == PlaneId::LocalMock {
                crate::types::Quality::Preview
            } else {
                job.quality
            }),
            sidecar_protocol: meta.sidecar_protocol.map(str::to_string),
            synthesis: meta.synthesis,
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
        self.cancel.store(true, std::sync::atomic::Ordering::SeqCst);
        job.cancel_requested = true;
        let remote = job.plane.is_some_and(|p| p.is_remote());
        if remote
            && matches!(
                job.status,
                JobStatus::Submitted | JobStatus::Running | JobStatus::WaitingUpstream
            )
        {
            // Design §8.3: keep state; cancel_requested is the honest flag.
        } else if job.plane == Some(PlaneId::LocalMock)
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
        self.artifact_view(job_id, kind, None)
    }

    pub fn artifact_view(
        &self,
        job_id: &str,
        kind: ArtifactKind,
        view_id: Option<&str>,
    ) -> Result<(std::path::PathBuf, String, u64, &'static str), Error> {
        let job = self.status(job_id)?;
        let needs_terminal = matches!(kind, ArtifactKind::Glb | ArtifactKind::Manifest);
        if needs_terminal && !job.status.has_artifact() {
            return Err(Error::new(
                error_type::EXPORT_NOT_READY,
                format!(
                    "job is {} (need succeeded or degraded)",
                    status_name(job.status)
                ),
            ));
        }
        let path = match kind {
            ArtifactKind::Glb => self.store.artifact_path(job_id, "artifact.glb"),
            ArtifactKind::Manifest => self.store.artifact_path(job_id, "manifest.json"),
            ArtifactKind::Contract => self.store.artifact_path(job_id, "contract.json"),
            ArtifactKind::Log => self.store.artifact_path(job_id, "log.stderr.txt"),
            ArtifactKind::View => {
                if let Some(id) = view_id.filter(|s| !s.is_empty()) {
                    if id.contains('/') || id.contains("..") {
                        return Err(Error::new(
                            error_type::SPEC_REJECTED,
                            "view_id must be a camera id",
                        ));
                    }
                    self.store.artifact_path(job_id, &format!("views/{id}.png"))
                } else if let Some(p) = job.artifacts.views.first() {
                    std::path::PathBuf::from(p)
                } else {
                    self.store.artifact_path(job_id, "views/hero.png")
                }
            }
        };
        if !path.is_file() {
            return Err(Error::new(
                error_type::EXPORT_NOT_READY,
                format!("{} is not on disk", path.display()),
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
        let mut est = estimate_job(spec, &self.probe(), &self.spend_policy(spec));
        let text = spec.prompt.as_deref().is_some_and(|s| !s.trim().is_empty());
        if !text {
            return est;
        }
        let n_views = view_count(spec.quality);
        if let Some(p) = &self.t2i {
            if let Ok(c) = estimate_orbit(p.as_ref(), n_views) {
                est.usd += c.usd;
                est.usd_uncertain = est.usd_uncertain || c.usd_uncertain;
                est.breakdown.push(crate::types::EstimateStep {
                    step: "t2i.orbit".into(),
                    usd: c.usd,
                    n: n_views,
                });
            }
        } else if !self.is_test() {
            if let Some(im) = Imaginarium::from_env() {
                match estimate_orbit(&im, n_views) {
                    Ok(c) => {
                        est.usd += c.usd;
                        est.usd_uncertain = est.usd_uncertain || c.usd_uncertain;
                        est.breakdown.push(crate::types::EstimateStep {
                            step: "t2i.orbit".into(),
                            usd: c.usd,
                            n: n_views,
                        });
                    }
                    Err(_) => est.usd_uncertain = true,
                }
            }
        }
        est
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
    use crate::types::{ComputeMode, JobSubmit, MaterialMode, PlaneId, ProbeSnapshot, Quality};

    #[test]
    fn process_starts_without_sidecar_vram() {
        let app = App::for_test(true);
        assert!(
            !app.sidecar_loaded(),
            "API/MCP/CLI must start with no sidecar child"
        );
    }

    #[test]
    fn generate_does_not_auto_pull_weights() {
        let dir = tempfile::tempdir().unwrap();
        let before = std::fs::read_dir(dir.path())
            .map(|rd| rd.count())
            .unwrap_or(0);
        let app = App::for_test(true);
        let (_tmp, png) = write_dot();
        let _ = app.submit(JobSubmit {
            image_path: Some(png.to_string_lossy().into_owned()),
            compute: ComputeMode::Local,
            provider: Some(PlaneId::LocalMock),
            ..JobSubmit::default()
        });
        let after = std::fs::read_dir(dir.path())
            .map(|rd| rd.count())
            .unwrap_or(0);
        assert_eq!(before, after, "generate must not write a weights pull");
    }

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

    #[test]
    fn imaginarium_estimate_then_fire_mock_mesh() {
        let (base, h) = crate::t2i_imaginarium::tests::serve_fake();
        let im = crate::t2i_imaginarium::Imaginarium::new(base, Some("tok".into())).unwrap();
        let mut app = App::for_test(true).with_t2i(Box::new(im));
        app.allow_spend = true;
        let job = app
            .submit(JobSubmit {
                prompt: Some("a red fox wearing a yellow raincoat".into()),
                compute: ComputeMode::Local,
                provider: Some(PlaneId::LocalMock),
                quality: Quality::Preview,
                allow_spend: true,
                ..JobSubmit::default()
            })
            .unwrap();
        assert_eq!(job.status, JobStatus::Degraded);
        assert!(job.artifacts.contract.is_some());
        assert_eq!(job.artifacts.views.len(), 4);
        assert!(job.spend.estimated_usd.unwrap_or(0.0) > 0.0);
        drop(h);
    }

    #[test]
    fn imaginarium_spend_gated_needs_confirm() {
        let (base, h) = crate::t2i_imaginarium::tests::serve_fake();
        let im = crate::t2i_imaginarium::Imaginarium::new(base, Some("tok".into())).unwrap();
        let app = App::for_test(false)
            .with_t2i(Box::new(im))
            .with_probe(ProbeSnapshot {
                sidecar_alive: true,
                allow_mock: false,
                ..ProbeSnapshot::cpu_only(false)
            })
            .with_sidecar(std::path::PathBuf::from("/bin/true"));
        let job = app
            .submit(JobSubmit {
                prompt: Some("a red fox wearing a yellow raincoat".into()),
                compute: ComputeMode::Local,
                provider: Some(PlaneId::LocalSidecar),
                quality: Quality::Preview,
                allow_spend: false,
                ..JobSubmit::default()
            })
            .unwrap();
        assert_eq!(job.status, JobStatus::NeedsConfirm);
        assert_eq!(
            job.error.as_ref().unwrap().error_type,
            error_type::SPEND_GATED
        );
        drop(h);
    }

    #[test]
    fn sidecar_text_without_t2i_unavailable() {
        let app = App::for_test(false).with_probe(ProbeSnapshot {
            sidecar_alive: true,
            allow_mock: false,
            ..ProbeSnapshot::cpu_only(false)
        });
        let job = app
            .submit(JobSubmit {
                prompt: Some("a red fox".into()),
                compute: ComputeMode::Local,
                provider: Some(PlaneId::LocalSidecar),
                ..JobSubmit::default()
            })
            .unwrap();
        assert_eq!(job.status, JobStatus::Failed);
        assert_eq!(job.error.unwrap().error_type, error_type::T2I_UNAVAILABLE);
    }

    #[test]
    fn tripo_image_fixture_degraded() {
        let (base, h) = crate::remote::tests::serve_fake(crate::remote::tests::FakeMode::Ok);
        let tripo = crate::remote_tripo::Tripo::for_test(base, "tok".into()).unwrap();
        let mut app = App::for_test(false)
            .with_tripo(tripo)
            .with_probe(ProbeSnapshot {
                tripo_key: true,
                keys_present: true,
                ..ProbeSnapshot::cpu_only(false)
            });
        app.allow_spend = true;
        let (_tmp, png) = write_dot();
        let job = app
            .submit(JobSubmit {
                image_path: Some(png.to_string_lossy().into_owned()),
                compute: ComputeMode::Remote,
                provider: Some(PlaneId::RemoteTripo),
                allow_spend: true,
                quality: Quality::Preview,
                ..JobSubmit::default()
            })
            .unwrap();
        assert_eq!(job.status, JobStatus::Degraded);
        assert_eq!(job.plane, Some(PlaneId::RemoteTripo));
        assert_eq!(job.upstream_id.as_deref(), Some("task_tripo_1"));
        let man: Manifest = serde_json::from_str(
            &std::fs::read_to_string(job.artifacts.manifest.as_ref().unwrap()).unwrap(),
        )
        .unwrap();
        assert_eq!(man.engine.as_deref(), Some("tripo"));
        assert!(!man.ok);
        drop(h);
    }

    #[test]
    fn meshy_402_fails_job() {
        let (base, h) = crate::remote::tests::serve_fake(crate::remote::tests::FakeMode::Credit402);
        let meshy = crate::remote_meshy::Meshy::for_test(base, "tok".into()).unwrap();
        let mut app = App::for_test(false)
            .with_meshy(meshy)
            .with_probe(ProbeSnapshot {
                meshy_key: true,
                keys_present: true,
                ..ProbeSnapshot::cpu_only(false)
            });
        app.allow_spend = true;
        let (_tmp, png) = write_dot();
        let job = app
            .submit(JobSubmit {
                image_path: Some(png.to_string_lossy().into_owned()),
                compute: ComputeMode::Remote,
                provider: Some(PlaneId::RemoteMeshy),
                allow_spend: true,
                ..JobSubmit::default()
            })
            .unwrap();
        assert_eq!(job.status, JobStatus::Failed);
        assert_eq!(
            job.error.as_ref().unwrap().error_type,
            error_type::SPEND_PROVIDER_402
        );
        drop(h);
    }

    #[test]
    fn remote_without_client_never_posts() {
        let app = App::for_test(false).with_probe(ProbeSnapshot {
            tripo_key: true,
            keys_present: true,
            ..ProbeSnapshot::cpu_only(false)
        });
        let mut app = app;
        app.allow_spend = true;
        let (_tmp, png) = write_dot();
        let job = app
            .submit(JobSubmit {
                image_path: Some(png.to_string_lossy().into_owned()),
                compute: ComputeMode::Remote,
                provider: Some(PlaneId::RemoteTripo),
                allow_spend: true,
                ..JobSubmit::default()
            })
            .unwrap();
        assert_eq!(job.status, JobStatus::Failed);
        assert_eq!(job.error.unwrap().error_type, error_type::NOT_CONFIGURED);
    }

    #[test]
    fn remote_spend_gated_needs_confirm() {
        let app = App::for_test(false).with_probe(ProbeSnapshot {
            tripo_key: true,
            keys_present: true,
            ..ProbeSnapshot::cpu_only(false)
        });
        let (_tmp, png) = write_dot();
        let job = app
            .submit(JobSubmit {
                image_path: Some(png.to_string_lossy().into_owned()),
                compute: ComputeMode::Remote,
                provider: Some(PlaneId::RemoteTripo),
                allow_spend: false,
                ..JobSubmit::default()
            })
            .unwrap();
        assert_eq!(job.status, JobStatus::NeedsConfirm);
        assert_eq!(
            job.error.as_ref().unwrap().error_type,
            error_type::SPEND_GATED
        );
    }

    #[test]
    fn remote_cancel_keeps_submitted() {
        let app = App::for_test(false);
        let mut job = MeshJob::from_submit(
            ulid::Ulid::new().to_string(),
            &JobSubmit {
                image_path: Some("dot.png".into()),
                ..JobSubmit::default()
            },
        );
        job.status = JobStatus::Submitted;
        job.plane = Some(PlaneId::RemoteMeshy);
        job.upstream_id = Some("task_meshy_1".into());
        app.store.create(&job).unwrap();
        let c = app.cancel(&job.id).unwrap();
        assert_eq!(c.status, JobStatus::Submitted);
        assert!(c.cancel_requested);
        assert_eq!(c.upstream_id.as_deref(), Some("task_meshy_1"));
    }
}
