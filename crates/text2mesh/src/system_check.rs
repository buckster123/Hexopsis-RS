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
    probe.devices = crate::probe::probe_devices();
    let rows = crate::weights::scan_env();
    probe.weights_present = crate::weights::quality_present(&rows);
    probe.preview_weights = crate::weights::preview_present(&rows);
    probe.dinov3_accepted = crate::weights::dinov3_accepted(&rows);
    probe.licenses_accepted = crate::weights::licenses_ok(&rows);
    probe.tripo_key = key_present("TRIPO_API_KEY");
    probe.meshy_key = key_present("MESHY_API_KEY");
    probe.hunyuan_key = key_present("HUNYUAN_API_KEY");
    probe.keys_present = probe.tripo_key || probe.meshy_key || probe.hunyuan_key;
    probe.allow_mock = allow_mock || env_truthy("TEXT2MESH_ALLOW_MOCK");
    probe.hunyuan_allowed = env_truthy("TEXT2MESH_ALLOW_HUNYUAN")
        && std::env::var("TEXT2MESH_HUNYUAN_ATTESTATION")
            .map(|p| std::path::Path::new(&p).is_file())
            .unwrap_or(false);
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
    if let Some(mb) = crate::probe::disk_free_mb(&crate::weights::weights_dir()) {
        probe.disk_free_mb = mb;
    }
    probe
}

fn key_present(name: &str) -> bool {
    std::env::var(name).map(|v| !v.is_empty()).unwrap_or(false)
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
        tier: detect_tier(probe),
        features: FeatureReport {
            compiled: vec!["remote-http".into(), "webui".into()],
            not_compiled: vec![
                "sidecar".into(),
                "preview-onnx".into(),
                "preview-candle".into(),
                "gate-clip".into(),
            ],
            horizon_unscheduled: vec!["quality-candle".into(), "quality-ggml".into()],
        },
        devices: device_rows(probe),
        weights: weight_rows_from_probe(probe),
        licenses: LicenseReport {
            dinov3_accepted: probe.dinov3_accepted,
            hunyuan_community: "blocked_by_default".into(),
            cgal_gpl: "blocked_by_default".into(),
            hunyuan_reasons: vec![
                "territory_eu_uk_kr".into(),
                "mau_cap".into(),
                "no_train_on_outputs".into(),
                "hk_law".into(),
            ],
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
    sc.weights = crate::weights::scan_env();
    sc.siblings = probe_siblings();
    sc.sidecars = probe_sidecar_rows();
    sc
}

fn detect_tier(probe: &ProbeSnapshot) -> Option<String> {
    // Nano: no quality weights and (vram null or <6 GiB or shared). Sidecar cargo feature is off.
    if probe.weights_present {
        return None;
    }
    let (vram, shared) = gpu_vram(probe);
    if vram == 0 || vram < 6144 || shared {
        Some("nano".into())
    } else {
        None
    }
}

fn gpu_vram(probe: &ProbeSnapshot) -> (u32, bool) {
    let mut best = 0u32;
    let mut shared = false;
    for d in &probe.devices {
        if d.kind == DeviceKind::Cpu {
            continue;
        }
        let v = d.vram_mb.unwrap_or(0);
        if v >= best {
            best = v;
            shared = d.shared;
        }
    }
    (best, shared)
}

fn weight_rows_from_probe(probe: &ProbeSnapshot) -> Vec<WeightRow> {
    let mut rows = crate::weights::catalog_empty_rows();
    for r in &mut rows {
        match r.id.as_str() {
            "quality.stack" => {
                r.present = probe.weights_present;
                r.accepted = probe.licenses_accepted && probe.weights_present;
            }
            "preview.feedforward" => {
                r.present = probe.preview_weights;
                r.accepted = probe.preview_weights;
            }
            "encoder.dinov3_vitl16" => {
                r.accepted = probe.dinov3_accepted;
            }
            _ => {}
        }
    }
    rows
}

fn probe_siblings() -> Vec<SiblingRow> {
    let url = std::env::var("TEXT2MESH_IMAGINARIUM_URL")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "http://127.0.0.1:8791".into());
    let token = std::env::var("TEXT2MESH_IMAGINARIUM_TOKEN")
        .ok()
        .filter(|s| !s.is_empty());
    // blocking reqwest must not run (or drop) on a tokio worker.
    let url_h = url.clone();
    let (ok, reason) =
        std::thread::spawn(
            move || match crate::t2i_imaginarium::Imaginarium::new(url_h, token) {
                Ok(im) => {
                    if im.health() {
                        (true, None)
                    } else {
                        (false, Some("health failed".into()))
                    }
                }
                Err(e) => (false, Some(e.message)),
            },
        )
        .join()
        .unwrap_or((false, Some("health probe panicked".into())));
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
                name: d.name.clone(),
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

    #[test]
    fn cpu_only_is_nano_and_lists_catalog() {
        let probe = ProbeSnapshot::cpu_only(false);
        let sc = build_system_check(&probe, &SpendPolicy::default());
        assert_eq!(sc.tier.as_deref(), Some("nano"));
        assert_eq!(sc.weights.len(), 5);
        assert!(sc
            .licenses
            .hunyuan_reasons
            .iter()
            .any(|r| r == "territory_eu_uk_kr"));
    }

    #[test]
    fn krackan_shared_512_degrades_or_remote() {
        use crate::types::DeviceProbe;
        let probe = ProbeSnapshot {
            devices: vec![
                DeviceProbe {
                    kind: DeviceKind::Cpu,
                    vram_mb: None,
                    shared: false,
                    slow: true,
                    name: None,
                },
                DeviceProbe {
                    kind: DeviceKind::GpuVulkan,
                    vram_mb: Some(512),
                    shared: true,
                    slow: true,
                    name: Some("AMD Radeon 840M".into()),
                },
            ],
            weights_present: true,
            licenses_accepted: true,
            sidecar_alive: true,
            disk_free_mb: 100_000,
            ..ProbeSnapshot::default()
        };
        let closed = SpendPolicy::default();
        let sc = build_system_check(&probe, &closed);
        let vk = sc
            .devices
            .iter()
            .find(|d| d.kind == DeviceKind::GpuVulkan)
            .unwrap();
        assert_eq!(vk.vram_mb, Some(512));
        assert!(vk.shared);
        assert_eq!(vk.name.as_deref(), Some("AMD Radeon 840M"));
        assert!(sc.planner.would_pick.is_none());
        assert_eq!(
            sc.planner.degrade.as_ref().map(|d| d.error_type.as_str()),
            Some(error_type::VRAM_SHORT)
        );

        let mut open_probe = probe.clone();
        open_probe.tripo_key = true;
        open_probe.keys_present = true;
        let open = SpendPolicy {
            allow_spend: true,
            max_usd: 2.0,
        };
        let sc = build_system_check(&open_probe, &open);
        assert_eq!(sc.planner.would_pick, Some(PlaneId::RemoteTripo));
        assert_ne!(sc.planner.would_pick, Some(PlaneId::LocalSidecar));
    }

    #[test]
    fn live_krackan_sysfs_system_check_is_honest() {
        // Field truth on Krackan: 512 MiB shared iGPU. CI without amdgpu is a no-op.
        let probe = probe_from_env(false);
        let Some(vk) = probe
            .devices
            .iter()
            .find(|d| d.kind == DeviceKind::GpuVulkan)
        else {
            return;
        };
        if !vk.shared {
            return;
        }
        assert!(
            vk.vram_mb.is_some_and(|v| (256..=2048).contains(&v)),
            "shared vulkan vram_mb={:?} must be the carve-out, not host RAM",
            vk.vram_mb
        );
        let spend = SpendPolicy::default();
        let sc = build_system_check(&probe, &spend);
        let row = sc
            .devices
            .iter()
            .find(|d| d.kind == DeviceKind::GpuVulkan)
            .unwrap();
        assert!(row.shared);
        assert!(row.vram_mb.unwrap_or(0) < 4096);
        match sc.planner.would_pick {
            Some(PlaneId::RemoteTripo | PlaneId::RemoteMeshy | PlaneId::RemoteColony) => {}
            Some(PlaneId::LocalSidecar | PlaneId::LocalPreview) => {
                panic!("shared 512 MiB must not pick local quality")
            }
            Some(other) => {
                if other != PlaneId::LocalMock {
                    panic!("unexpected plane {other:?}");
                }
            }
            None => {
                let t = sc.planner.degrade.as_ref().map(|d| d.error_type.as_str());
                assert!(
                    matches!(
                        t,
                        Some("vram_short")
                            | Some("weights_missing")
                            | Some("spend.gated")
                            | Some("not_configured")
                    ),
                    "got {t:?}"
                );
            }
        }
    }
}
