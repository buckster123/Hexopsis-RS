//! `text2mesh.view_contract.v1` types.

use serde::{Deserialize, Serialize};

use crate::types::{CameraPreset, PromptClass, T2iProviderId};

pub const VIEW_CONTRACT_SCHEMA: &str = "text2mesh.view_contract.v1";

pub const JANUS_NEGATIVES: &[&str] = &[
    "second face",
    "face on the back of the head",
    "two faces",
    "duplicate head",
    "extra limbs",
    "cropped limbs",
    "text",
    "watermark",
    "multiple subjects",
    "logo",
];

pub const OTHER_NEGATIVES: &[&str] = &[
    "text",
    "watermark",
    "logo",
    "multiple subjects",
    "cropped object",
    "hands holding the object",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ViewContract {
    pub schema: String,
    pub contract_id: String,
    pub created_at: String,
    pub prompt: ContractPrompt,
    pub subject_lock: SubjectLock,
    pub camera_ring: CameraRing,
    pub lighting: Lighting,
    pub background: Background,
    pub style_lock: StyleLock,
    pub negatives: Vec<String>,
    pub seed_policy: SeedPolicy,
    pub frame: Frame,
    pub t2i: T2iRef,
    pub compile_notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContractPrompt {
    pub raw: String,
    pub normalized: String,
    pub hash: String,
    pub language: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SubjectLock {
    pub identity_phrase: String,
    pub class: PromptClass,
    pub attributes: Vec<String>,
    pub canonical_view_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CameraRing {
    pub preset: CameraPreset,
    pub count: u32,
    pub convention: String,
    pub distance: f64,
    pub fov_deg: f64,
    pub cameras: Vec<Camera>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Camera {
    pub id: String,
    pub role: String,
    pub azimuth_deg: i32,
    pub elevation_deg: i32,
    pub roll_deg: i32,
    pub required: bool,
    pub prompt_suffix: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Lighting {
    pub rig: String,
    pub locked: bool,
    pub key_azimuth_deg: i32,
    pub fill_ratio: f64,
    pub white_balance: String,
    pub prompt_lock: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Background {
    pub mode: String,
    pub hex: String,
    pub alpha_preferred: bool,
    pub prompt_lock: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StyleLock {
    pub medium: String,
    pub albedo_bias: bool,
    pub prompt_lock: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SeedPolicy {
    pub family_seed: u64,
    pub hero_seed: u64,
    pub orbit_seed_mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub aspect: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct T2iRef {
    pub provider: T2iProviderId,
    pub model: Option<String>,
    pub quality_tier: String,
}

impl ViewContract {
    pub fn assembled_prompt(&self, cam: &Camera, orbit: bool) -> String {
        let mut body = format!(
            "{}, {}, {}, {}, {}, azimuth {} degrees, elevation {} degrees, full subject in frame",
            self.subject_lock.identity_phrase,
            self.style_lock.prompt_lock,
            self.background.prompt_lock,
            self.lighting.prompt_lock,
            cam.prompt_suffix,
            cam.azimuth_deg,
            cam.elevation_deg
        );
        if orbit {
            body.push_str(", same design as the reference");
        }
        let neg = self.negatives.join(", ");
        format!("{body}\nNEGATIVE: {neg}")
    }
}
