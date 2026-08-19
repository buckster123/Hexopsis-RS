//! Public `error_type` strings (PRD §10.2 / design §9). Add only with a design amendment.

use serde::{Deserialize, Serialize};

/// Stable public error_type tokens.
pub mod error_type {
    pub const NOT_CONFIGURED: &str = "not_configured";
    pub const WEIGHTS_MISSING: &str = "weights_missing";
    pub const FEATURE_OFF: &str = "feature_off";
    pub const DEVICE_MISSING: &str = "device_missing";
    pub const VRAM_SHORT: &str = "vram_short";
    pub const DISK_SHORT: &str = "disk_short";
    pub const SPEND_GATED: &str = "spend.gated";
    pub const SPEND_ESTIMATE_EXCEEDED: &str = "spend.estimate_exceeded";
    pub const SPEND_PROVIDER_402: &str = "spend.provider_402";
    pub const LICENSE_BLOCKED: &str = "license.blocked";
    pub const LICENSE_DINOV3_UNACCEPTED: &str = "license.dinov3_unaccepted";
    pub const LICENSE_PRINT_WRAP_UNAVAILABLE: &str = "license.print_wrap_unavailable";
    pub const ANALYTIC_UNAVAILABLE: &str = "analytic.unavailable";
    pub const ANALYTIC_TOO_COMPLEX: &str = "analytic.too_complex";
    pub const T2I_UNAVAILABLE: &str = "t2i.unavailable";
    pub const VIEW_CONSISTENCY: &str = "view.consistency";
    pub const VIEW_HERO_TEXT_MISMATCH: &str = "view.hero_text_mismatch";
    pub const VIEW_IDENTITY_DRIFT: &str = "view.identity_drift";
    pub const VIEW_JANUS_FACE: &str = "view.janus_face";
    pub const VIEW_FRAMING: &str = "view.framing";
    pub const VIEW_LIGHTING_DRIFT: &str = "view.lighting_drift";
    pub const EXPORT_NOT_READY: &str = "export.not_ready";
    pub const EXPORT_MATERIAL_MODE: &str = "export.material_mode";
    pub const EXPORT_MATERIALS_MISSING: &str = "export.materials_missing";
    pub const ENGINE_CRASH: &str = "engine.crash";
    pub const ENGINE_INTERRUPTED: &str = "engine.interrupted";
    pub const ENGINE_OOM: &str = "engine.oom";
    pub const WAIT_TIMEOUT: &str = "wait.timeout";
    pub const WATCHDOG_QUEUE: &str = "watchdog.queue";
    pub const UNSUPPORTED: &str = "unsupported";
    pub const RATE_LIMIT: &str = "rate_limit";
    pub const SPEC_REJECTED: &str = "spec.rejected";
    pub const CANCELLED: &str = "cancelled";
    pub const IO: &str = "io";
    pub const INTERNAL: &str = "internal";
    pub const NOT_FOUND: &str = "not_found";
}

/// Structured error object shared by MeshJob, faces, and planner degrades.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[error("{message}")]
pub struct Error {
    pub error_type: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub also: Vec<String>,
}

impl Error {
    pub fn new(error_type: &'static str, message: impl Into<String>) -> Self {
        Self {
            error_type: error_type.to_string(),
            message: message.into(),
            hint: None,
            also: Vec::new(),
        }
    }

    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    pub fn with_also(mut self, also: Vec<String>) -> Self {
        self.also = also;
        self
    }

    pub fn not_found(job_id: &str) -> Self {
        Self::new(error_type::NOT_FOUND, format!("job {job_id} not found"))
    }

    pub fn is_type(&self, t: &str) -> bool {
        self.error_type == t
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::new(error_type::IO, e.to_string())
    }
}

impl From<rusqlite::Error> for Error {
    fn from(e: rusqlite::Error) -> Self {
        Self::new(error_type::IO, e.to_string()).with_hint("sqlite job store")
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Self::new(error_type::INTERNAL, e.to_string()).with_hint("json")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_types_are_stable_strings() {
        assert_eq!(error_type::SPEND_GATED, "spend.gated");
        assert_eq!(
            error_type::LICENSE_DINOV3_UNACCEPTED,
            "license.dinov3_unaccepted"
        );
        assert_eq!(error_type::VIEW_JANUS_FACE, "view.janus_face");
        assert_eq!(error_type::EXPORT_MATERIAL_MODE, "export.material_mode");
    }

    #[test]
    fn error_json_has_also() {
        let e = Error::new(error_type::VIEW_CONSISTENCY, "ladder exhausted")
            .with_hint("inspect views/")
            .with_also(vec![error_type::VIEW_IDENTITY_DRIFT.into()]);
        let v = serde_json::to_value(&e).unwrap();
        assert_eq!(v["error_type"], "view.consistency");
        assert_eq!(v["also"][0], "view.identity_drift");
        assert_eq!(v["hint"], "inspect views/");
    }
}
