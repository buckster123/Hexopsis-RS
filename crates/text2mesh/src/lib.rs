//! text2mesh (Tessera-RS) core — MeshJob, planner, mock plane, store.
//!
//! Faces (MCP / CLI / HTTP) stay thin. Binding decisions: `docs/CHARTER.md`.

pub mod canonical;
pub mod classify;
pub mod compiler;
pub mod config;
pub mod contract;
pub mod director;
pub mod error;
pub mod export;
pub mod gates;
pub mod hash;
pub mod idle;
pub mod mcp_schema;
pub mod mock_glb;
pub mod orbit;
pub mod planner;
pub mod probe;
pub mod remote;
pub mod remote_meshy;
pub mod remote_tripo;
pub mod router;
pub mod sidecar;
pub mod store;
pub mod system_check;
pub mod t2i;
pub mod t2i_imaginarium;
pub mod types;
pub mod weights;

pub use compiler::{compile_view_contract, CompileOpts};
pub use config::Config;
pub use contract::ViewContract;
pub use director::App;
pub use error::{error_type, Error};
pub use planner::plan;
pub use store::Store;
pub use types::*;

/// Load `~/.config/text2mesh/env` and `/etc/text2mesh/env` without logging secrets.
/// Env already set wins. `XAI_API_KEY` is never imported.
pub fn load_xdg_env() {
    let mut paths = Vec::new();
    if let Some(dir) = dirs::config_dir() {
        paths.push(dir.join("text2mesh").join("env"));
    }
    paths.push(std::path::PathBuf::from("/etc/text2mesh/env"));
    for p in paths {
        if p.is_file() {
            load_env_file(&p);
        }
    }
}

fn load_env_file(path: &std::path::Path) {
    let Ok(text) = std::fs::read_to_string(path) else {
        return;
    };
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let k = k.trim();
        if k.is_empty() || k == "XAI_API_KEY" {
            continue;
        }
        let v = v.trim().trim_matches(|c| c == '"' || c == '\'');
        if std::env::var_os(k).is_none() {
            // SAFETY: process start, before worker threads in CLI/MCP/API mains.
            unsafe { std::env::set_var(k, v) };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_never_serializes_pending() {
        let s = serde_json::to_string(&JobStatus::Queued).unwrap();
        assert!(!s.contains("pending"));
    }
}
