//! Frozen enums and MeshJob / JobSubmit / honesty surfaces (design §1–3, §13–14).

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::Error;

pub const JOB_SCHEMA: &str = "text2mesh.job.v1";
pub const MANIFEST_SCHEMA: &str = "text2mesh.manifest.v1";
pub const SYSTEM_CHECK_SCHEMA: &str = "text2mesh.system_check.v1";
pub const ESTIMATE_SCHEMA: &str = "text2mesh.estimate.v1";

pub const WAIT_MIN_S: u64 = 30;
pub const WAIT_DEFAULT_S: u64 = 1800;
pub const WAIT_MAX_S: u64 = 86_400;

pub const PRODUCT: &str = "text2mesh";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Route {
    #[default]
    Auto,
    Analytic,
    ViewContract,
    Native,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PromptClass {
    Analytic,
    Creature,
    Character,
    Product,
    Vehicle,
    Architecture,
    Prop,
    Unknown,
}

impl PromptClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Analytic => "analytic",
            Self::Creature => "creature",
            Self::Character => "character",
            Self::Product => "product",
            Self::Vehicle => "vehicle",
            Self::Architecture => "architecture",
            Self::Prop => "prop",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Quality {
    Preview,
    #[default]
    Standard,
    High,
    Ultra,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComputeMode {
    #[default]
    Auto,
    Local,
    Remote,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeviceKind {
    Cpu,
    NvidiaCuda,
    AmdRocm,
    GpuVulkan,
    AppleMetal,
}

impl DeviceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Cpu => "cpu",
            Self::NvidiaCuda => "nvidia.cuda",
            Self::AmdRocm => "amd.rocm",
            Self::GpuVulkan => "gpu.vulkan",
            Self::AppleMetal => "apple.metal",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "cpu" => Some(Self::Cpu),
            "nvidia.cuda" => Some(Self::NvidiaCuda),
            "amd.rocm" => Some(Self::AmdRocm),
            "gpu.vulkan" => Some(Self::GpuVulkan),
            "apple.metal" => Some(Self::AppleMetal),
            _ => None,
        }
    }
}

impl Serialize for DeviceKind {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for DeviceKind {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Self::parse(&s).ok_or_else(|| serde::de::Error::custom(format!("unknown device {s}")))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlaneId {
    LocalMock,
    LocalSidecar,
    LocalPreview,
    LocalAnalytic,
    RemoteMeshy,
    RemoteTripo,
    RemoteColony,
    RemoteHunyuanHosted,
}

impl PlaneId {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::LocalMock => "local.mock",
            Self::LocalSidecar => "local.sidecar",
            Self::LocalPreview => "local.preview",
            Self::LocalAnalytic => "local.analytic",
            Self::RemoteMeshy => "remote.meshy",
            Self::RemoteTripo => "remote.tripo",
            Self::RemoteColony => "remote.colony",
            Self::RemoteHunyuanHosted => "remote.hunyuan_hosted",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "local.mock" => Some(Self::LocalMock),
            "local.sidecar" => Some(Self::LocalSidecar),
            "local.preview" => Some(Self::LocalPreview),
            "local.analytic" => Some(Self::LocalAnalytic),
            "remote.meshy" => Some(Self::RemoteMeshy),
            "remote.tripo" => Some(Self::RemoteTripo),
            "remote.colony" => Some(Self::RemoteColony),
            "remote.hunyuan_hosted" => Some(Self::RemoteHunyuanHosted),
            _ => None,
        }
    }

    pub fn is_local(self) -> bool {
        matches!(
            self,
            Self::LocalMock | Self::LocalSidecar | Self::LocalPreview | Self::LocalAnalytic
        )
    }

    pub fn is_remote(self) -> bool {
        !self.is_local()
    }

    pub fn is_mock(self) -> bool {
        matches!(self, Self::LocalMock)
    }
}

impl Serialize for PlaneId {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for PlaneId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Self::parse(&s).ok_or_else(|| serde::de::Error::custom(format!("unknown plane {s}")))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Queued,
    NeedsConfirm,
    Submitted,
    Running,
    WaitingUpstream,
    Succeeded,
    Degraded,
    Failed,
    Cancelled,
}

impl JobStatus {
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Succeeded | Self::Degraded | Self::Failed | Self::Cancelled
        )
    }

    /// Terminal with a GLB that may be fetched (succeeded or degraded).
    pub fn has_artifact(self) -> bool {
        matches!(self, Self::Succeeded | Self::Degraded)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MaterialMode {
    UvAtlas,
    VertexColor,
    FactorsOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlphaMode {
    OPAQUE,
    MASK,
    BLEND,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CameraPreset {
    Cardinal4,
    Cardinal4HeroTop,
    Cardinal4HeroTopQuarters,
    NativePassthrough,
}

impl CameraPreset {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Cardinal4 => "cardinal4",
            Self::Cardinal4HeroTop => "cardinal4_hero_top",
            Self::Cardinal4HeroTopQuarters => "cardinal4_hero_top_quarters",
            Self::NativePassthrough => "native_passthrough",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum T2iProviderId {
    Imaginarium,
    Http,
    Local,
    Mock,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Synthesis {
    HeroOrbit,
    IndependentT2i,
    NativePassthrough,
    Analytic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactKind {
    Glb,
    Manifest,
    Contract,
    View,
    Log,
}

impl ArtifactKind {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "glb" => Some(Self::Glb),
            "manifest" => Some(Self::Manifest),
            "contract" => Some(Self::Contract),
            "view" => Some(Self::View),
            "log" => Some(Self::Log),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Glb => "glb",
            Self::Manifest => "manifest",
            Self::Contract => "contract",
            Self::View => "view",
            Self::Log => "log",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputKind {
    Image,
    Text,
    Views,
    Analytic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobInput {
    pub kind: InputKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_hash_raw: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_hash_conditioned: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contract_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputeBlock {
    pub mode: ComputeMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prefer_device: Option<DeviceKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<PlaneId>,
    pub requested: ComputeMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actual: Option<PlaneId>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExportFlags {
    #[serde(default)]
    pub keep_largest_component: bool,
    #[serde(default)]
    pub force_opaque: bool,
    #[serde(default)]
    pub unit_cube: bool,
    #[serde(default)]
    pub uv_atlas: bool,
    #[serde(default)]
    pub print_wrap: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Budget {
    pub max_usd: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_credits: Option<u64>,
    pub max_wall_s: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_vram_mb: Option<u32>,
}

impl Default for Budget {
    fn default() -> Self {
        Self {
            max_usd: 2.0,
            max_credits: None,
            max_wall_s: WAIT_DEFAULT_S,
            max_vram_mb: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Spend {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimated_usd: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actual_usd: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reserved_retries_usd: Option<f64>,
    #[serde(default)]
    pub currency: String,
    #[serde(default)]
    pub usd_uncertain: bool,
}

impl Default for Spend {
    fn default() -> Self {
        Self {
            estimated_usd: None,
            actual_usd: None,
            reserved_retries_usd: None,
            currency: "USD".into(),
            usd_uncertain: false,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Artifacts {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub glb: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manifest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contract: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub views: Vec<String>,
}

pub type JobError = Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeshJob {
    pub schema: String,
    pub id: String,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_job: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<String>,
    pub input: JobInput,
    pub route: Route,
    pub quality: Quality,
    pub compute: ComputeBlock,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seed: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub camera_preset: Option<CameraPreset>,
    #[serde(default)]
    pub allow_spend: bool,
    #[serde(default)]
    pub allow_neural_cad: bool,
    #[serde(default)]
    pub allow_native_text: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license_override: Option<String>,
    #[serde(default)]
    pub export: ExportFlags,
    pub budget: Budget,
    pub status: JobStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stage: Option<String>,
    #[serde(default)]
    pub pct: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plane: Option<PlaneId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upstream_id: Option<String>,
    #[serde(default)]
    pub cancel_requested: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<JobError>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub degrades: Vec<String>,
    pub spend: Spend,
    pub artifacts: Artifacts,
}

impl MeshJob {
    pub fn from_submit(id: String, spec: &JobSubmit) -> Self {
        let now = now_rfc3339();
        let kind = if spec
            .image_path
            .as_deref()
            .is_some_and(|s| !s.trim().is_empty())
        {
            InputKind::Image
        } else {
            InputKind::Text
        };
        let prompt = spec.prompt.as_ref().map(|p| p.trim().to_string());
        let prompt_hash = prompt.as_ref().map(|p| crate::hash::sha256_str(p));
        Self {
            schema: JOB_SCHEMA.into(),
            id,
            created_at: now.clone(),
            updated_at: now,
            parent_job: None,
            idempotency_key: spec.idempotency_key.clone(),
            input: JobInput {
                kind,
                prompt,
                prompt_hash,
                image_path: spec.image_path.clone(),
                image_hash_raw: None,
                image_hash_conditioned: None,
                contract_id: None,
            },
            route: spec.route,
            quality: spec.quality,
            compute: ComputeBlock {
                mode: spec.compute,
                prefer_device: spec.prefer_device,
                provider: spec.provider,
                requested: spec.compute,
                actual: None,
            },
            seed: spec.seed,
            camera_preset: spec.camera_preset,
            allow_spend: spec.allow_spend,
            allow_neural_cad: spec.allow_neural_cad,
            allow_native_text: spec.allow_native_text,
            license_override: spec.license_override.clone(),
            export: spec.export.clone(),
            budget: Budget {
                max_usd: spec.max_usd,
                max_credits: spec.max_credits,
                max_wall_s: spec.max_wall_s,
                max_vram_mb: None,
            },
            status: JobStatus::Queued,
            stage: None,
            pct: 0,
            plane: None,
            upstream_id: None,
            cancel_requested: false,
            error: None,
            degrades: Vec::new(),
            spend: Spend::default(),
            artifacts: Artifacts::default(),
        }
    }

    pub fn touch(&mut self) {
        self.updated_at = now_rfc3339();
    }
}

fn default_max_usd() -> f64 {
    2.0
}

fn default_max_wall_s() -> u64 {
    WAIT_DEFAULT_S
}

/// Caller-settable fields. Defaults match design §3.4.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobSubmit {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_path: Option<String>,
    #[serde(default)]
    pub route: Route,
    #[serde(default)]
    pub quality: Quality,
    #[serde(default)]
    pub compute: ComputeMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<PlaneId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prefer_device: Option<DeviceKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seed: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub camera_preset: Option<CameraPreset>,
    #[serde(default)]
    pub allow_spend: bool,
    #[serde(default)]
    pub allow_neural_cad: bool,
    #[serde(default)]
    pub allow_native_text: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license_override: Option<String>,
    #[serde(default = "default_max_usd")]
    pub max_usd: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_credits: Option<u64>,
    #[serde(default = "default_max_wall_s")]
    pub max_wall_s: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<String>,
    #[serde(default)]
    pub export: ExportFlags,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub job_id: Option<String>,
}

impl Default for JobSubmit {
    fn default() -> Self {
        Self {
            prompt: None,
            image_path: None,
            route: Route::Auto,
            quality: Quality::Standard,
            compute: ComputeMode::Auto,
            provider: None,
            prefer_device: None,
            seed: None,
            camera_preset: None,
            allow_spend: false,
            allow_neural_cad: false,
            allow_native_text: false,
            license_override: None,
            max_usd: 2.0,
            max_credits: None,
            max_wall_s: WAIT_DEFAULT_S,
            idempotency_key: None,
            export: ExportFlags::default(),
            job_id: None,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ManifestHashes {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub glb: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub job: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contract: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Timings {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub queue_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub schema: String,
    pub job_id: String,
    pub ok: bool,
    pub status: JobStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plane: Option<PlaneId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub engine: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disclaimer: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub material_mode: Option<MaterialMode>,
    #[serde(default)]
    pub hashes: ManifestHashes,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub degrades: Vec<String>,
    #[serde(default)]
    pub spend: Spend,
    #[serde(default)]
    pub timings: Timings,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quality: Option<Quality>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceRow {
    pub kind: DeviceKind,
    #[serde(default)]
    pub ok: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slow: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vram_mb: Option<u32>,
    #[serde(default)]
    pub shared: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyRow {
    pub id: String,
    pub present: bool,
    pub len: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub head: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub leaked_into_process: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiblingRow {
    pub id: String,
    pub url: String,
    pub ok: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeightRow {
    pub id: String,
    pub present: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub want_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub have_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256_head: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    #[serde(default)]
    pub accepted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureReport {
    pub compiled: Vec<String>,
    pub not_compiled: Vec<String>,
    pub horizon_unscheduled: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseReport {
    pub dinov3_accepted: bool,
    pub hunyuan_community: String,
    pub cgal_gpl: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannerView {
    pub mode: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub would_pick: Option<PlaneId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub degrade: Option<Degrade>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpendView {
    pub gate: String,
    pub spent_today_usd: f64,
    pub max_usd_per_job: f64,
    pub max_usd_per_day: f64,
}

/// system-check.v1 — no `ok` field for readiness (D29).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemCheck {
    pub schema: String,
    pub report_complete: bool,
    pub ready: bool,
    pub product: String,
    pub version: String,
    pub features: FeatureReport,
    pub devices: Vec<DeviceRow>,
    pub weights: Vec<WeightRow>,
    pub licenses: LicenseReport,
    pub keys: Vec<KeyRow>,
    #[serde(default)]
    pub sidecars: Vec<serde_json::Value>,
    pub siblings: Vec<SiblingRow>,
    pub planner: PlannerView,
    pub spend: SpendView,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EstimateStep {
    pub step: String,
    pub usd: f64,
    pub n: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EstimateCaps {
    pub max_usd_per_job: f64,
    pub max_usd_per_day: f64,
    pub spent_today: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Estimate {
    pub schema: String,
    pub ok: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plane: Option<PlaneId>,
    pub usd: f64,
    pub usd_uncertain: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credits: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credit_unit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seconds_p50: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tier: Option<Quality>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub views: Option<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub breakdown: Vec<EstimateStep>,
    pub caps: EstimateCaps,
    pub gate: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<Degrade>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceProbe {
    pub kind: DeviceKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vram_mb: Option<u32>,
    #[serde(default)]
    pub shared: bool,
    #[serde(default)]
    pub slow: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeSnapshot {
    pub devices: Vec<DeviceProbe>,
    #[serde(default)]
    pub features: Vec<String>,
    #[serde(default)]
    pub weights_present: bool,
    #[serde(default)]
    pub preview_weights: bool,
    #[serde(default)]
    pub keys_present: bool,
    #[serde(default)]
    pub tripo_key: bool,
    #[serde(default)]
    pub meshy_key: bool,
    #[serde(default)]
    pub hunyuan_key: bool,
    #[serde(default)]
    pub colony_live: bool,
    #[serde(default)]
    pub allow_mock: bool,
    #[serde(default)]
    pub cadre_live: bool,
    #[serde(default)]
    pub sidecar_alive: bool,
    #[serde(default)]
    pub licenses_accepted: bool,
    /// D19: key + ALLOW_HUNYUAN + attestation + job override. Default false.
    #[serde(default)]
    pub hunyuan_allowed: bool,
    #[serde(default)]
    pub dinov3_accepted: bool,
    #[serde(default)]
    pub disk_free_mb: u64,
}

impl Default for ProbeSnapshot {
    fn default() -> Self {
        Self {
            devices: vec![DeviceProbe {
                kind: DeviceKind::Cpu,
                vram_mb: None,
                shared: false,
                slow: true,
            }],
            features: Vec::new(),
            weights_present: false,
            preview_weights: false,
            keys_present: false,
            tripo_key: false,
            meshy_key: false,
            hunyuan_key: false,
            colony_live: false,
            allow_mock: false,
            cadre_live: false,
            sidecar_alive: false,
            licenses_accepted: false,
            hunyuan_allowed: false,
            dinov3_accepted: false,
            disk_free_mb: 100_000,
        }
    }
}

impl ProbeSnapshot {
    pub fn cpu_only(allow_mock: bool) -> Self {
        Self {
            allow_mock,
            ..Self::default()
        }
    }

    pub fn paid_key(&self) -> bool {
        self.keys_present || self.tripo_key || self.meshy_key || self.hunyuan_key
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpendPolicy {
    pub allow_spend: bool,
    pub max_usd: f64,
}

impl Default for SpendPolicy {
    fn default() -> Self {
        Self {
            allow_spend: false,
            max_usd: 2.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaneChoice {
    pub plane: PlaneId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quality_rewrite: Option<Quality>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub degrades: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Degrade {
    pub error_type: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub also: Vec<String>,
}

impl Degrade {
    pub fn new(error_type: &'static str, message: impl Into<String>) -> Self {
        Self {
            error_type: error_type.to_string(),
            message: message.into(),
            also: Vec::new(),
        }
    }
}

impl From<Degrade> for Error {
    fn from(d: Degrade) -> Self {
        Error {
            error_type: d.error_type,
            message: d.message,
            hint: None,
            also: d.also,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaitResult {
    pub ok: bool,
    pub job: MeshJob,
    pub wait_timed_out: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_type: Option<String>,
}

pub fn now_rfc3339() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    unix_to_rfc3339(secs)
}

pub fn unix_to_rfc3339(secs: u64) -> String {
    let (y, m, d, hh, mm, ss) = civil_from_unix(secs as i64);
    format!("{y:04}-{m:02}-{d:02}T{hh:02}:{mm:02}:{ss:02}Z")
}

pub fn rfc3339_to_unix(s: &str) -> Option<u64> {
    let s = s.trim();
    let body = s.strip_suffix('Z').unwrap_or(s);
    let (date, time) = body.split_once('T')?;
    let mut dparts = date.split('-');
    let y: i32 = dparts.next()?.parse().ok()?;
    let mo: u32 = dparts.next()?.parse().ok()?;
    let da: u32 = dparts.next()?.parse().ok()?;
    let time = time.split(['+', '-']).next().unwrap_or(time);
    let mut tparts = time.split(':');
    let hh: u32 = tparts.next()?.parse().ok()?;
    let mm: u32 = tparts.next()?.parse().ok()?;
    let ss: u32 = tparts.next()?.split('.').next()?.parse().ok()?;
    unix_from_civil(y, mo, da, hh, mm, ss)
}

fn civil_from_unix(secs: i64) -> (i32, u32, u32, u32, u32, u32) {
    let days = secs.div_euclid(86400);
    let rem = secs.rem_euclid(86400) as u32;
    let hh = rem / 3600;
    let mm = (rem % 3600) / 60;
    let ss = rem % 60;
    let (y, m, d) = civil_from_days(days);
    (y, m, d, hh, mm, ss)
}

/// Days from 1970-01-01 → civil date (Howard Hinnant).
fn civil_from_days(z: i64) -> (i32, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 }.div_euclid(146_097);
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m as u32, d as u32)
}

fn unix_from_civil(y: i32, m: u32, d: u32, hh: u32, mm: u32, ss: u32) -> Option<u64> {
    if !(1..=12).contains(&m) || hh > 23 || mm > 59 || ss > 60 {
        return None;
    }
    let y = y as i64;
    let (y, m) = if m <= 2 { (y - 1, m + 9) } else { (y, m - 3) };
    let era = y.div_euclid(400);
    let yoe = (y - era * 400) as u64;
    let doy = (153 * m as u64 + 2) / 5 + d as u64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe as i64 - 719_468;
    let secs = days.checked_mul(86400)? + i64::from(hh) * 3600 + i64::from(mm) * 60 + i64::from(ss);
    u64::try_from(secs).ok()
}

/// Minimal 1×1 RGB PNG (red). Used by tests and HTTP mock ingest.
pub fn minimal_png_1x1() -> Vec<u8> {
    hex::decode(
        "89504e470d0a1a0a0000000d4948445200000001000000010802000000907753de\
         0000000c4944415478da63f8cfc0000003010100f70341430000000049454e44ae426082",
    )
    .unwrap_or_else(|_| vec![0x89, b'P', b'N', b'G'])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn job_status_never_pending() {
        let s = serde_json::to_string(&JobStatus::Queued).unwrap();
        assert_eq!(s, "\"queued\"");
        assert!(serde_json::from_str::<JobStatus>("\"pending\"").is_err());
    }

    #[test]
    fn plane_id_dotted() {
        let p = PlaneId::LocalMock;
        assert_eq!(serde_json::to_string(&p).unwrap(), "\"local.mock\"");
        let back: PlaneId = serde_json::from_str("\"remote.hunyuan_hosted\"").unwrap();
        assert_eq!(back, PlaneId::RemoteHunyuanHosted);
    }

    #[test]
    fn job_submit_defaults() {
        let s: JobSubmit = serde_json::from_str("{}").unwrap();
        assert_eq!(s.route, Route::Auto);
        assert_eq!(s.quality, Quality::Standard);
        assert_eq!(s.compute, ComputeMode::Auto);
        assert!(!s.allow_spend);
        assert_eq!(s.max_usd, 2.0);
        assert_eq!(s.max_wall_s, 1800);
        assert!(!s.export.uv_atlas);
    }

    #[test]
    fn rfc3339_roundtrip() {
        let s = unix_to_rfc3339(1_724_068_800);
        assert!(s.ends_with('Z'));
        let back = rfc3339_to_unix(&s).unwrap();
        assert_eq!(back, 1_724_068_800);
    }

    #[test]
    fn wait_bounds_frozen() {
        assert_eq!(WAIT_MIN_S, 30);
        assert_eq!(WAIT_DEFAULT_S, 1800);
        assert_eq!(WAIT_MAX_S, 86_400);
    }
}
