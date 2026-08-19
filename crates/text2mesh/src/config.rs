//! Env-driven config. Secrets stay in 0600 env files; we never log values.

use std::net::SocketAddr;
use std::path::PathBuf;

use crate::types::SpendPolicy;

pub const DEFAULT_BIND: &str = "127.0.0.1:8796";

#[derive(Debug, Clone)]
pub struct Config {
    pub bind: String,
    pub store: Option<PathBuf>,
    pub allow_spend: bool,
    pub allow_mock: bool,
    pub allow_ungated: bool,
    pub max_usd_per_job: f64,
    pub max_usd_per_day: f64,
    pub token: Option<String>,
    pub sidecar: Option<PathBuf>,
    pub imaginarium_url: String,
    pub imaginarium_token: Option<String>,
    pub meshy_url: String,
    pub tripo_url: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bind: DEFAULT_BIND.into(),
            store: None,
            allow_spend: false,
            allow_mock: false,
            allow_ungated: false,
            max_usd_per_job: 2.0,
            max_usd_per_day: 10.0,
            token: None,
            sidecar: None,
            imaginarium_url: "http://127.0.0.1:8791".into(),
            imaginarium_token: None,
            meshy_url: crate::remote::MESHY_DEFAULT.into(),
            tripo_url: crate::remote::TRIPO_DEFAULT.into(),
        }
    }
}

impl Config {
    pub fn from_env() -> Self {
        let mut c = Self::default();
        if let Ok(b) = std::env::var("TEXT2MESH_BIND") {
            if !b.is_empty() {
                c.bind = b;
            }
        }
        match std::env::var("TEXT2MESH_STORE") {
            Ok(s) if s.is_empty() => c.store = Some(PathBuf::new()),
            Ok(s) => c.store = Some(PathBuf::from(s)),
            Err(_) => {}
        }
        c.allow_spend = env_truthy("TEXT2MESH_ALLOW_SPEND");
        c.allow_mock = env_truthy("TEXT2MESH_ALLOW_MOCK");
        c.allow_ungated = env_truthy("TEXT2MESH_ALLOW_UNGATED");
        if let Ok(v) = std::env::var("TEXT2MESH_MAX_USD_PER_JOB") {
            if let Ok(n) = v.parse() {
                c.max_usd_per_job = n;
            }
        }
        if let Ok(v) = std::env::var("TEXT2MESH_MAX_USD_PER_DAY") {
            if let Ok(n) = v.parse() {
                c.max_usd_per_day = n;
            }
        }
        if let Ok(t) = std::env::var("TEXT2MESH_TOKEN") {
            if !t.is_empty() {
                c.token = Some(t);
            }
        }
        if let Ok(p) = std::env::var("TEXT2MESH_SIDECAR") {
            if !p.is_empty() {
                c.sidecar = Some(PathBuf::from(p));
            }
        }
        if let Ok(u) = std::env::var("TEXT2MESH_IMAGINARIUM_URL") {
            if !u.is_empty() {
                c.imaginarium_url = u;
            }
        }
        if let Ok(t) = std::env::var("TEXT2MESH_IMAGINARIUM_TOKEN") {
            if !t.is_empty() {
                c.imaginarium_token = Some(t);
            }
        }
        if let Ok(u) = std::env::var("TEXT2MESH_MESHY_URL") {
            if !u.is_empty() {
                c.meshy_url = u;
            }
        }
        if let Ok(u) = std::env::var("TEXT2MESH_TRIPO_URL") {
            if !u.is_empty() {
                c.tripo_url = u;
            }
        }
        c
    }

    pub fn spend_policy(&self) -> SpendPolicy {
        SpendPolicy {
            allow_spend: self.allow_spend,
            max_usd: self.max_usd_per_job,
        }
    }

    pub fn bind_addr(&self) -> Result<SocketAddr, String> {
        self.bind
            .parse()
            .map_err(|e| format!("TEXT2MESH_BIND {}: {e}", self.bind))
    }

    pub fn bind_is_loopback(&self) -> bool {
        self.bind_addr()
            .map(|a| a.ip().is_loopback())
            .unwrap_or(true)
    }
}

pub fn env_truthy(name: &str) -> bool {
    matches!(
        std::env::var(name).as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE") | Ok("yes") | Ok("YES") | Ok("on")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_bind_is_8796_loopback() {
        let c = Config::default();
        assert_eq!(c.bind, "127.0.0.1:8796");
        assert!(c.bind_is_loopback());
        assert!(!c.allow_spend);
        assert!(!c.allow_mock);
    }
}
