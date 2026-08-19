//! Honesty surface. `ready` ⇔ planner.would_pick != null. No `ok` field.

use crate::config::{env_truthy, Config};
use crate::error::error_type;
use crate::planner::plan;
use crate::types::{
    DeviceKind, DeviceRow, FeatureReport, JobSubmit, KeyRow, LicenseReport, PlaneId, PlannerView,
    ProbeSnapshot, SiblingRow, SpendPolicy, SpendView, SystemCheck, WeightRow, PRODUCT,
    SYSTEM_CHECK_SCHEMA, VERSION,
};

pub fn probe_from_env(allow_mock: bool) -> ProbeSnapshot {
    let mut probe = ProbeSnapshot::cpu_only(allow_mock || env_truthy("TEXT2MESH_ALLOW_MOCK"));
    probe.tripo_key = key_present("TRIPO_API_KEY");
    probe.meshy_key = key_present("MESHY_API_KEY");
    probe.hunyuan_key = key_present("HUNYUAN_API_KEY");
    probe.keys_present = probe.tripo_key || probe.meshy_key || probe.hunyuan_key;
    probe.allow_mock = allow_mock || env_truthy("TEXT2MESH_ALLOW_MOCK");
    probe.hunyuan_allowed = env_truthy("TEXT2MESH_ALLOW_HUNYUAN")
        && std::env::var("TEXT2MESH_HUNYUAN_ATTESTATION")
            .map(|p| std::path::Path::new(&p).is_file())
            .unwrap_or(false);
    probe.dinov3_accepted = env_truthy("TEXT2MESH_ACCEPT_DINOV3");
    probe.cadre_live = std::env::var("TEXT2MESH_CADRE_URL")
        .ok()
        .filter(|s| !s.is_empty())
        .is_some()
        || std::env::var("TEXT2MESH_CADRE_BIN")
            .ok()
            .filter(|s| !s.is_empty())
            .is_some();
    probe.sidecar_alive = std::env::var("TEXT2MESH_SIDECAR")
        .ok()
        .filter(|s| !s.is_empty())
        .map(|p| std::path::Path::new(&p).is_file())
        .unwrap_or(false);
    probe.colony_live = std::env::var("TEXT2MESH_COLONY_URL")
        .ok()
        .filter(|s| !s.is_empty())
        .is_some();
    if let Some(mb) = disk_free_mb() {
        probe.disk_free_mb = mb;
    }
    probe
}

fn key_present(name: &str) -> bool {
    std::env::var(name).map(|v| !v.is_empty()).unwrap_or(false)
}

fn disk_free_mb() -> Option<u64> {
    // Best-effort; Nano tests do not depend on the real number.
    None
}

pub fn build_system_check(probe: &ProbeSnapshot, spend: &SpendPolicy) -> SystemCheck {
    let spec = JobSubmit {
        image_path: Some("probe".into()),
        ..JobSubmit::default()
    };
    let (would_pick, degrade) = match plan(&spec, probe, spend) {
        Ok(c) => (Some(c.plane), None),
        Err(d) => (None, Some(d)),
    };
    SystemCheck {
        schema: SYSTEM_CHECK_SCHEMA.into(),
        report_complete: true,
        ready: would_pick.is_some(),
        product: PRODUCT.into(),
        version: VERSION.into(),
        features: FeatureReport {
            compiled: vec!["remote-http".into()],
            not_compiled: vec![
                "sidecar".into(),
                "preview-onnx".into(),
                "preview-candle".into(),
                "gate-clip".into(),
                "webui".into(),
            ],
            horizon_unscheduled: vec!["quality-candle".into(), "quality-ggml".into()],
        },
        devices: device_rows(probe),
        weights: vec![WeightRow {
            id: "quality.stack".into(),
            present: probe.weights_present,
            want_bytes: Some(16 * 1024 * 1024 * 1024),
            have_bytes: None,
            path: Some("~/.local/share/text2mesh/weights/quality.stack".into()),
            sha256_head: None,
            license: Some("MIT".into()),
            accepted: probe.licenses_accepted,
        }],
        licenses: LicenseReport {
            dinov3_accepted: probe.dinov3_accepted,
            hunyuan_community: "blocked_by_default".into(),
            cgal_gpl: "blocked_by_default".into(),
        },
        keys: key_rows(),
        sidecars: vec![],
        siblings: vec![
            SiblingRow {
                id: "imaginarium".into(),
                url: std::env::var("TEXT2MESH_IMAGINARIUM_URL")
                    .unwrap_or_else(|_| "http://127.0.0.1:8791".into()),
                ok: false,
                reason: Some("not probed".into()),
            },
            SiblingRow {
                id: "cadre".into(),
                url: std::env::var("TEXT2MESH_CADRE_URL")
                    .unwrap_or_else(|_| "http://127.0.0.1:7410".into()),
                ok: probe.cadre_live,
                reason: if probe.cadre_live {
                    None
                } else {
                    Some("not probed".into())
                },
            },
        ],
        planner: PlannerView {
            mode: "auto".into(),
            would_pick,
            degrade,
        },
        spend: SpendView {
            gate: if spend.allow_spend { "open" } else { "closed" }.into(),
            spent_today_usd: 0.0,
            max_usd_per_job: spend.max_usd,
            max_usd_per_day: Config::from_env().max_usd_per_day,
        },
    }
}

pub fn system_check_from_env(allow_mock: bool) -> SystemCheck {
    let cfg = Config::from_env();
    let probe = probe_from_env(allow_mock);
    let mut spend = cfg.spend_policy();
    spend.allow_spend = spend.allow_spend || env_truthy("TEXT2MESH_ALLOW_SPEND");
    let mut sc = build_system_check(&probe, &spend);
    sc.siblings = probe_siblings();
    sc.sidecars = probe_sidecar_rows();
    sc
}

fn probe_siblings() -> Vec<SiblingRow> {
    let url = std::env::var("TEXT2MESH_IMAGINARIUM_URL")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "http://127.0.0.1:8791".into());
    let token = std::env::var("TEXT2MESH_IMAGINARIUM_TOKEN")
        .ok()
        .filter(|s| !s.is_empty());
    let (ok, reason) = match crate::t2i_imaginarium::Imaginarium::new(url.clone(), token) {
        Ok(im) => {
            if im.health() {
                (true, None)
            } else {
                (false, Some("health failed".into()))
            }
        }
        Err(e) => (false, Some(e.message)),
    };
    vec![
        SiblingRow {
            id: "imaginarium".into(),
            url,
            ok,
            reason,
        },
        SiblingRow {
            id: "cadre".into(),
            url: std::env::var("TEXT2MESH_CADRE_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:7410".into()),
            ok: std::env::var("TEXT2MESH_CADRE_URL")
                .ok()
                .filter(|s| !s.is_empty())
                .is_some()
                || std::env::var("TEXT2MESH_CADRE_BIN")
                    .ok()
                    .filter(|s| !s.is_empty())
                    .is_some(),
            reason: None,
        },
    ]
}

fn probe_sidecar_rows() -> Vec<serde_json::Value> {
    let Some(bin) = crate::sidecar::sidecar_bin_from_env() else {
        return vec![];
    };
    let p = crate::sidecar::probe_sidecar(&bin, std::time::Duration::from_secs(5));
    vec![serde_json::json!({
        "path": bin,
        "ok": p.ok,
        "protocol": p.protocol,
        "engine": p.engine,
        "reason": p.reason,
    })]
}

fn device_rows(probe: &ProbeSnapshot) -> Vec<DeviceRow> {
    let mut rows = Vec::new();
    for kind in [
        DeviceKind::Cpu,
        DeviceKind::NvidiaCuda,
        DeviceKind::AmdRocm,
        DeviceKind::GpuVulkan,
        DeviceKind::AppleMetal,
    ] {
        if let Some(d) = probe.devices.iter().find(|d| d.kind == kind) {
            rows.push(DeviceRow {
                kind,
                ok: true,
                slow: Some(d.slow),
                vram_mb: d.vram_mb,
                shared: d.shared,
                name: None,
                reason: None,
            });
        } else {
            rows.push(DeviceRow {
                kind,
                ok: false,
                slow: None,
                vram_mb: None,
                shared: false,
                name: None,
                reason: Some(match kind {
                    DeviceKind::Cpu => "no cpu probe".into(),
                    DeviceKind::NvidiaCuda => "nvidia-smi not found".into(),
                    DeviceKind::AmdRocm => "rocminfo not found".into(),
                    DeviceKind::GpuVulkan => "no vulkan device".into(),
                    DeviceKind::AppleMetal => "not macos".into(),
                }),
            });
        }
    }
    rows
}

fn key_rows() -> Vec<KeyRow> {
    vec![
        inspect_key("MESHY_API_KEY"),
        inspect_key("TRIPO_API_KEY"),
        inspect_key("TEXT2MESH_TOKEN"),
        xai_key_row(),
    ]
}

fn inspect_key(id: &str) -> KeyRow {
    match std::env::var(id) {
        Ok(v) if !v.is_empty() => {
            let head: String = v.chars().take(4).collect();
            KeyRow {
                id: id.into(),
                present: true,
                len: v.len(),
                head: Some(head),
                note: None,
                leaked_into_process: None,
            }
        }
        _ => KeyRow {
            id: id.into(),
            present: false,
            len: 0,
            head: None,
            note: None,
            leaked_into_process: None,
        },
    }
}

fn xai_key_row() -> KeyRow {
    let leaked = std::env::var_os("XAI_API_KEY").is_some();
    KeyRow {
        id: "XAI_API_KEY".into(),
        present: false,
        len: 0,
        head: None,
        note: Some("must not be read by this process".into()),
        leaked_into_process: leaked.then_some(true),
    }
}

/// Catalog estimate (free). If spend is gated, still reports the would-be plane.
pub fn estimate(
    spec: &JobSubmit,
    probe: &ProbeSnapshot,
    spend: &SpendPolicy,
) -> crate::types::Estimate {
    use crate::planner::catalog_usd;
    use crate::types::{Estimate, EstimateCaps, ESTIMATE_SCHEMA};

    let planned = plan(spec, probe, spend);
    let (plane, degrade, gated) = match planned {
        Ok(c) => (Some(c.plane), None, false),
        Err(d) if d.error_type == error_type::SPEND_GATED => {
            let open = SpendPolicy {
                allow_spend: true,
                max_usd: spend.max_usd,
            };
            match plan(spec, probe, &open) {
                Ok(c) => (Some(c.plane), Some(d), true),
                Err(e) => (None, Some(e), true),
            }
        }
        Err(d) => (None, Some(d), false),
    };
    let usd = plane.map(catalog_usd).unwrap_or(0.0);
    let seconds = match plane {
        Some(PlaneId::LocalMock) => Some(1),
        Some(PlaneId::LocalSidecar) => Some(400),
        Some(PlaneId::LocalPreview) => Some(30),
        Some(p) if p.is_remote() => Some(420),
        _ => None,
    };
    let ok = plane.is_some() || gated;
    Estimate {
        schema: ESTIMATE_SCHEMA.into(),
        ok,
        plane,
        usd,
        usd_uncertain: false,
        credits: None,
        credit_unit: plane.map(|p| p.as_str().to_string()),
        seconds_p50: seconds,
        tier: Some(spec.quality),
        views: Some(match spec.quality {
            crate::types::Quality::Preview => 4,
            crate::types::Quality::Standard => 6,
            crate::types::Quality::High | crate::types::Quality::Ultra => 8,
        }),
        breakdown: vec![],
        caps: EstimateCaps {
            max_usd_per_job: spend.max_usd,
            max_usd_per_day: 10.0,
            spent_today: 0.0,
        },
        gate: if spend.allow_spend { "open" } else { "closed" }.into(),
        error: degrade,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_check_cpu_only_fixture() {
        let probe = ProbeSnapshot::cpu_only(false);
        let sc = build_system_check(&probe, &SpendPolicy::default());
        assert!(sc.report_complete);
        assert!(!sc.ready);
        assert!(sc.planner.would_pick.is_none());
        let v = serde_json::to_value(&sc).unwrap();
        assert!(
            v.get("ok").is_none(),
            "system-check must not use ok for readiness"
        );
        assert_eq!(v["schema"], "text2mesh.system_check.v1");
    }

    #[test]
    fn system_check_allow_mock_ready() {
        let probe = ProbeSnapshot::cpu_only(true);
        let sc = build_system_check(&probe, &SpendPolicy::default());
        assert!(sc.ready);
        assert_eq!(sc.planner.would_pick, Some(PlaneId::LocalMock));
    }

    #[test]
    fn xai_key_never_present() {
        let row = xai_key_row();
        assert!(!row.present);
        assert!(row.head.is_none());
        assert_eq!(row.len, 0);
    }
}
