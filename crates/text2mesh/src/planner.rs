//! Pure planner (design §7.2). First-reason order is load-bearing.

use crate::error::error_type;
use crate::types::{
    ComputeMode, Degrade, DeviceKind, JobSubmit, PlaneChoice, PlaneId, ProbeSnapshot, Quality,
    Route, SpendPolicy,
};

const PREVIEW_VRAM_MB: u32 = 6144;
const STANDARD_VRAM_MB: u32 = 24_576;
const PREVIEW_DISK_MB: u64 = 2200;
const STANDARD_DISK_MB: u64 = 16_384;

pub fn plan(
    spec: &JobSubmit,
    probe: &ProbeSnapshot,
    spend: &SpendPolicy,
) -> Result<PlaneChoice, Degrade> {
    if spec.export.print_wrap {
        return Err(Degrade::new(
            error_type::LICENSE_PRINT_WRAP_UNAVAILABLE,
            "print_wrap requested and no non-GPL wrap is linked",
        ));
    }

    if spec.route == Route::Analytic {
        if probe.cadre_live {
            return Ok(choice(PlaneId::LocalAnalytic, None, vec![]));
        }
        return Err(Degrade::new(
            error_type::ANALYTIC_UNAVAILABLE,
            "Cadre is not live (TEXT2MESH_CADRE_URL / TEXT2MESH_CADRE_BIN missing)",
        ));
    }

    let mut degrades = Vec::new();
    let mut quality = spec.quality;
    let rewrite = if spec.compute == ComputeMode::Auto && quality == Quality::Ultra {
        quality = Quality::High;
        degrades.push("quality.step_down".into());
        Some(Quality::High)
    } else {
        None
    };

    if let Some(p) = spec.provider {
        return plan_pinned(p, spec, probe, spend, quality, rewrite, degrades);
    }

    match spec.compute {
        ComputeMode::Local => pick_local(spec, probe, quality, rewrite, degrades),
        ComputeMode::Remote => pick_remote(probe, spend, rewrite, degrades, false),
        ComputeMode::Auto => pick_auto(spec, probe, spend, quality, rewrite, degrades),
    }
}

fn choice(plane: PlaneId, quality_rewrite: Option<Quality>, degrades: Vec<String>) -> PlaneChoice {
    PlaneChoice {
        plane,
        quality_rewrite,
        degrades,
    }
}

fn plan_pinned(
    p: PlaneId,
    spec: &JobSubmit,
    probe: &ProbeSnapshot,
    spend: &SpendPolicy,
    quality: Quality,
    rewrite: Option<Quality>,
    degrades: Vec<String>,
) -> Result<PlaneChoice, Degrade> {
    if spec.compute == ComputeMode::Local && p.is_remote() {
        return Err(Degrade::new(
            error_type::UNSUPPORTED,
            "mode=local never calls remote",
        ));
    }
    if spec.compute == ComputeMode::Remote && p.is_mock() {
        return Err(Degrade::new(
            error_type::UNSUPPORTED,
            "mode=remote never uses mock",
        ));
    }
    match p {
        PlaneId::LocalMock => Ok(choice(PlaneId::LocalMock, rewrite, degrades)),
        PlaneId::LocalSidecar => {
            if !probe.sidecar_alive {
                return Err(Degrade::new(
                    error_type::NOT_CONFIGURED,
                    "TEXT2MESH_SIDECAR is missing or not a file",
                ));
            }
            Ok(choice(p, rewrite, degrades))
        }
        PlaneId::LocalPreview => {
            if let Some(d) = local_quality_blocker(spec, probe, quality, true) {
                return Err(d);
            }
            Ok(choice(p, rewrite, degrades))
        }
        PlaneId::LocalAnalytic => {
            if probe.cadre_live {
                Ok(choice(p, rewrite, degrades))
            } else {
                Err(Degrade::new(
                    error_type::ANALYTIC_UNAVAILABLE,
                    "Cadre is not live",
                ))
            }
        }
        PlaneId::RemoteColony | PlaneId::RemoteTripo | PlaneId::RemoteMeshy => {
            if !remote_configured(p, probe) {
                return Err(Degrade::new(
                    error_type::NOT_CONFIGURED,
                    format!("{} is not configured", p.as_str()),
                ));
            }
            let usd = catalog_usd(p);
            spend_gate(usd, spend)?;
            Ok(choice(p, rewrite, degrades))
        }
        PlaneId::RemoteHunyuanHosted => {
            if !probe.hunyuan_allowed {
                return Err(Degrade::new(
                    error_type::LICENSE_BLOCKED,
                    "remote.hunyuan_hosted is inert without D19 gates",
                ));
            }
            if !probe.hunyuan_key {
                return Err(Degrade::new(
                    error_type::NOT_CONFIGURED,
                    "hunyuan hosted key missing",
                ));
            }
            let usd = catalog_usd(p);
            spend_gate(usd, spend)?;
            Ok(choice(p, rewrite, degrades))
        }
    }
}

fn pick_local(
    spec: &JobSubmit,
    probe: &ProbeSnapshot,
    quality: Quality,
    rewrite: Option<Quality>,
    degrades: Vec<String>,
) -> Result<PlaneChoice, Degrade> {
    // User-pinned GPU missing is the interesting reason (row 7), checked first in local mode.
    if let Some(dev) = spec.prefer_device {
        if !has_device(probe, dev) {
            return Err(Degrade::new(
                error_type::DEVICE_MISSING,
                format!("prefer_device={} is not present", dev.as_str()),
            ));
        }
    }
    if probe.sidecar_alive {
        return Ok(choice(PlaneId::LocalSidecar, rewrite, degrades));
    }
    if let Some(d) = local_quality_blocker(spec, probe, quality, false) {
        if probe.allow_mock {
            return Ok(choice(PlaneId::LocalMock, rewrite, degrades));
        }
        return Err(d);
    }
    Ok(choice(PlaneId::LocalSidecar, rewrite, degrades))
}

fn pick_auto(
    spec: &JobSubmit,
    probe: &ProbeSnapshot,
    spend: &SpendPolicy,
    quality: Quality,
    rewrite: Option<Quality>,
    degrades: Vec<String>,
) -> Result<PlaneChoice, Degrade> {
    let pinned_gpu_missing = spec
        .prefer_device
        .is_some_and(|d| d != DeviceKind::Cpu && !has_device(probe, d));

    let local_reason = if pinned_gpu_missing {
        Some(Degrade::new(
            error_type::DEVICE_MISSING,
            "prefer_device GPU is not present; auto will not silent-CPU a quality run",
        ))
    } else {
        local_quality_blocker(spec, probe, quality, false)
    };

    // Auto still honours VRAM/weights. A sidecar *binary* on disk is not a 24 GB GPU.
    if local_reason.is_none() {
        return Ok(choice(PlaneId::LocalSidecar, rewrite, degrades));
    }

    if let Some((plane, usd)) = first_remote(probe) {
        spend_gate(usd, spend)?;
        return Ok(choice(plane, rewrite, degrades));
    }

    if probe.allow_mock {
        return Ok(choice(PlaneId::LocalMock, rewrite, degrades));
    }

    Err(local_reason.unwrap_or_else(|| Degrade::new(error_type::UNSUPPORTED, "no feasible plane")))
}

fn pick_remote(
    probe: &ProbeSnapshot,
    spend: &SpendPolicy,
    rewrite: Option<Quality>,
    degrades: Vec<String>,
    allow_mock: bool,
) -> Result<PlaneChoice, Degrade> {
    if allow_mock {
        return Err(Degrade::new(
            error_type::UNSUPPORTED,
            "mode=remote never uses mock",
        ));
    }
    if let Some((plane, usd)) = first_remote(probe) {
        spend_gate(usd, spend)?;
        return Ok(choice(plane, rewrite, degrades));
    }
    Err(Degrade::new(
        error_type::NOT_CONFIGURED,
        "no remote key/colony is configured",
    ))
}

fn spend_gate(usd: f64, spend: &SpendPolicy) -> Result<(), Degrade> {
    if usd > 0.0 && !spend.allow_spend {
        return Err(Degrade::new(
            error_type::SPEND_GATED,
            format!("estimate usd={usd} > 0 and allow_spend is false"),
        ));
    }
    if usd > spend.max_usd {
        return Err(Degrade::new(
            error_type::SPEND_ESTIMATE_EXCEEDED,
            format!("estimate usd={usd} exceeds max_usd={}", spend.max_usd),
        ));
    }
    Ok(())
}

fn first_remote(probe: &ProbeSnapshot) -> Option<(PlaneId, f64)> {
    if probe.colony_live {
        return Some((PlaneId::RemoteColony, 0.0));
    }
    if probe.tripo_key || (probe.keys_present && !probe.meshy_key && !probe.hunyuan_key) {
        return Some((PlaneId::RemoteTripo, catalog_usd(PlaneId::RemoteTripo)));
    }
    if probe.meshy_key {
        return Some((PlaneId::RemoteMeshy, catalog_usd(PlaneId::RemoteMeshy)));
    }
    // Hunyuan never auto if colony/tripo/meshy/local is feasible (caller already failed local).
    if probe.hunyuan_allowed && probe.hunyuan_key {
        return Some((
            PlaneId::RemoteHunyuanHosted,
            catalog_usd(PlaneId::RemoteHunyuanHosted),
        ));
    }
    None
}

fn remote_configured(p: PlaneId, probe: &ProbeSnapshot) -> bool {
    match p {
        PlaneId::RemoteColony => probe.colony_live,
        PlaneId::RemoteTripo => probe.tripo_key || probe.keys_present,
        PlaneId::RemoteMeshy => probe.meshy_key || probe.keys_present,
        PlaneId::RemoteHunyuanHosted => probe.hunyuan_key && probe.hunyuan_allowed,
        _ => false,
    }
}

pub fn catalog_usd(plane: PlaneId) -> f64 {
    match plane {
        PlaneId::RemoteTripo => 0.54,
        PlaneId::RemoteMeshy => 0.40,
        PlaneId::RemoteHunyuanHosted => 0.50,
        _ => 0.0,
    }
}

fn local_quality_blocker(
    spec: &JobSubmit,
    probe: &ProbeSnapshot,
    quality: Quality,
    pinned: bool,
) -> Option<Degrade> {
    let need_vram = vram_need(quality);
    let need_disk = disk_need(quality);

    if quality != Quality::Preview && !probe.weights_present {
        return Some(Degrade::new(
            error_type::WEIGHTS_MISSING,
            "quality weights are not on disk",
        ));
    }
    if quality == Quality::Preview && !probe.preview_weights && !probe.weights_present {
        return Some(Degrade::new(
            error_type::WEIGHTS_MISSING,
            "preview weights are not on disk",
        ));
    }
    if !probe.licenses_accepted && probe.weights_present {
        return Some(Degrade::new(
            error_type::LICENSE_BLOCKED,
            "required weight licenses are not accepted",
        ));
    }
    if pinned {
        if let Some(dev) = spec.prefer_device {
            if !has_device(probe, dev) {
                return Some(Degrade::new(
                    error_type::DEVICE_MISSING,
                    format!("prefer_device={} is not present", dev.as_str()),
                ));
            }
        }
    }

    let (vram, shared) = gpu_vram(probe);
    if quality != Quality::Preview {
        // shared iGPU / <6 GB → never local standard/high
        if shared || vram < PREVIEW_VRAM_MB {
            return Some(Degrade::new(
                error_type::VRAM_SHORT,
                format!("shared={shared} vram_mb={vram} need_mb={need_vram}"),
            ));
        }
        if vram < need_vram {
            return Some(Degrade::new(
                error_type::VRAM_SHORT,
                format!("vram_mb={vram} need_mb={need_vram}"),
            ));
        }
    } else {
        let cpu_ok = probe
            .devices
            .iter()
            .any(|d| d.kind == DeviceKind::Cpu && d.slow);
        if vram < PREVIEW_VRAM_MB && !cpu_ok {
            return Some(Degrade::new(
                error_type::VRAM_SHORT,
                format!("preview needs {PREVIEW_VRAM_MB} GPU VRAM or CPU slow"),
            ));
        }
    }

    if probe.disk_free_mb < need_disk {
        return Some(Degrade::new(
            error_type::DISK_SHORT,
            format!("disk_free_mb={} need_mb={need_disk}", probe.disk_free_mb),
        ));
    }

    if quality != Quality::Preview && !probe.sidecar_alive && pinned {
        return Some(Degrade::new(
            error_type::NOT_CONFIGURED,
            "sidecar binary is not alive",
        ));
    }
    if quality != Quality::Preview && !probe.sidecar_alive && !probe.weights_present {
        return Some(Degrade::new(
            error_type::NOT_CONFIGURED,
            "sidecar is not configured",
        ));
    }
    // weights present, vram ok: treat sidecar as feasible even if handshake wasn't probed
    None
}

fn vram_need(q: Quality) -> u32 {
    match q {
        Quality::Preview => PREVIEW_VRAM_MB,
        Quality::Standard | Quality::High | Quality::Ultra => STANDARD_VRAM_MB,
    }
}

fn disk_need(q: Quality) -> u64 {
    match q {
        Quality::Preview => PREVIEW_DISK_MB,
        Quality::Standard | Quality::High | Quality::Ultra => STANDARD_DISK_MB,
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

fn has_device(probe: &ProbeSnapshot, kind: DeviceKind) -> bool {
    probe.devices.iter().any(|d| d.kind == kind)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{DeviceProbe, JobSubmit, Quality};

    fn image_spec(quality: Quality, compute: ComputeMode) -> JobSubmit {
        JobSubmit {
            image_path: Some("dot.png".into()),
            quality,
            compute,
            ..JobSubmit::default()
        }
    }

    fn cpu() -> ProbeSnapshot {
        ProbeSnapshot::cpu_only(false)
    }

    fn gpu(vram_mb: u32, shared: bool) -> DeviceProbe {
        DeviceProbe {
            kind: DeviceKind::NvidiaCuda,
            vram_mb: Some(vram_mb),
            shared,
            slow: false,
            name: None,
        }
    }

    fn closed() -> SpendPolicy {
        SpendPolicy {
            allow_spend: false,
            max_usd: 2.0,
        }
    }

    fn open() -> SpendPolicy {
        SpendPolicy {
            allow_spend: true,
            max_usd: 2.0,
        }
    }

    #[test]
    fn planner_row_01() {
        // image, standard, auto | CPU, no weights, no keys | closed → weights_missing
        let spec = image_spec(Quality::Standard, ComputeMode::Auto);
        let err = plan(&spec, &cpu(), &closed()).unwrap_err();
        assert_eq!(err.error_type, error_type::WEIGHTS_MISSING);
    }

    #[test]
    fn planner_row_02() {
        // quality weights, vram=24576, licenses ok → local sidecar
        let spec = image_spec(Quality::Standard, ComputeMode::Auto);
        let probe = ProbeSnapshot {
            devices: vec![gpu(24_576, false)],
            weights_present: true,
            licenses_accepted: true,
            sidecar_alive: true,
            disk_free_mb: 100_000,
            ..ProbeSnapshot::default()
        };
        let c = plan(&spec, &probe, &closed()).unwrap();
        assert_eq!(c.plane, PlaneId::LocalSidecar);
    }

    #[test]
    fn planner_row_03() {
        // no weights, TRIPO_API_KEY, spend open → remote.tripo
        let spec = image_spec(Quality::Standard, ComputeMode::Auto);
        let probe = ProbeSnapshot {
            tripo_key: true,
            keys_present: true,
            ..cpu()
        };
        let c = plan(&spec, &probe, &open()).unwrap();
        assert_eq!(c.plane, PlaneId::RemoteTripo);
    }

    #[test]
    fn planner_row_04() {
        // mode=local, no weights, tripo key → weights_missing (never remote)
        let spec = image_spec(Quality::Standard, ComputeMode::Local);
        let probe = ProbeSnapshot {
            tripo_key: true,
            keys_present: true,
            ..cpu()
        };
        let err = plan(&spec, &probe, &open()).unwrap_err();
        assert_eq!(err.error_type, error_type::WEIGHTS_MISSING);
    }

    #[test]
    fn planner_row_05() {
        // weights, vram=512 shared, tripo, open → remote tripo (not local standard)
        let spec = image_spec(Quality::Standard, ComputeMode::Auto);
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
            tripo_key: true,
            keys_present: true,
            disk_free_mb: 100_000,
            ..ProbeSnapshot::default()
        };
        let c = plan(&spec, &probe, &open()).unwrap();
        assert_eq!(c.plane, PlaneId::RemoteTripo);
    }

    #[test]
    fn planner_row_06() {
        // weights, vram=512 shared, no keys, closed → vram_short or spend.gated
        let spec = image_spec(Quality::Standard, ComputeMode::Auto);
        let probe = ProbeSnapshot {
            devices: vec![DeviceProbe {
                kind: DeviceKind::GpuVulkan,
                vram_mb: Some(512),
                shared: true,
                slow: true,
                name: Some("AMD Radeon 840M".into()),
            }],
            weights_present: true,
            licenses_accepted: true,
            sidecar_alive: true,
            disk_free_mb: 100_000,
            ..ProbeSnapshot::default()
        };
        let err = plan(&spec, &probe, &closed()).unwrap_err();
        assert!(
            err.error_type == error_type::VRAM_SHORT || err.error_type == error_type::SPEND_GATED,
            "got {}",
            err.error_type
        );
    }

    #[test]
    fn planner_row_07() {
        // prefer_device=cuda, local, CPU only → device_missing
        let mut spec = image_spec(Quality::Standard, ComputeMode::Local);
        spec.prefer_device = Some(DeviceKind::NvidiaCuda);
        let err = plan(&spec, &cpu(), &closed()).unwrap_err();
        assert_eq!(err.error_type, error_type::DEVICE_MISSING);
    }

    #[test]
    fn planner_row_08() {
        // usd>0, auto, remotes feasible, no local, spend closed → spend.gated
        let spec = image_spec(Quality::Standard, ComputeMode::Auto);
        let probe = ProbeSnapshot {
            tripo_key: true,
            keys_present: true,
            ..cpu()
        };
        let err = plan(&spec, &probe, &closed()).unwrap_err();
        assert_eq!(err.error_type, error_type::SPEND_GATED);
    }

    #[test]
    fn planner_row_09() {
        // allow-mock=1, no weights, no keys, CPU → local mock
        let spec = image_spec(Quality::Standard, ComputeMode::Auto);
        let probe = ProbeSnapshot {
            allow_mock: true,
            ..cpu()
        };
        let c = plan(&spec, &probe, &closed()).unwrap();
        assert_eq!(c.plane, PlaneId::LocalMock);
    }

    #[test]
    fn planner_row_10() {
        // quality=ultra, auto, weights+24GB → rewrite to high; never auto-select ultra
        let spec = image_spec(Quality::Ultra, ComputeMode::Auto);
        let probe = ProbeSnapshot {
            devices: vec![gpu(24_576, false)],
            weights_present: true,
            licenses_accepted: true,
            sidecar_alive: true,
            disk_free_mb: 100_000,
            ..ProbeSnapshot::default()
        };
        let c = plan(&spec, &probe, &closed()).unwrap();
        assert_eq!(c.plane, PlaneId::LocalSidecar);
        assert_eq!(c.quality_rewrite, Some(Quality::High));
        assert!(c.degrades.iter().any(|d| d == "quality.step_down"));
        assert_ne!(c.quality_rewrite, Some(Quality::Ultra));
    }

    #[test]
    fn planner_row_11() {
        // hunyuan key only, no D19, open → never remote.hunyuan_hosted
        let spec = image_spec(Quality::Standard, ComputeMode::Auto);
        let probe = ProbeSnapshot {
            hunyuan_key: true,
            hunyuan_allowed: false,
            keys_present: true,
            ..cpu()
        };
        match plan(&spec, &probe, &open()) {
            Ok(c) => assert_ne!(c.plane, PlaneId::RemoteHunyuanHosted),
            Err(_) => {}
        }
    }

    #[test]
    fn planner_row_12() {
        // all D19 + hunyuan + tripo, no local, open → remote.tripo (not hunyuan)
        let spec = image_spec(Quality::Standard, ComputeMode::Auto);
        let probe = ProbeSnapshot {
            hunyuan_key: true,
            hunyuan_allowed: true,
            tripo_key: true,
            keys_present: true,
            ..cpu()
        };
        let c = plan(&spec, &probe, &open()).unwrap();
        assert_eq!(c.plane, PlaneId::RemoteTripo);
    }

    #[test]
    fn local_sidecar_alive_skips_in_process_weights() {
        // User asked local and the child exists — sidecar owns weights (D28).
        let spec = image_spec(Quality::Standard, ComputeMode::Local);
        let probe = ProbeSnapshot {
            sidecar_alive: true,
            ..cpu()
        };
        let c = plan(&spec, &probe, &closed()).unwrap();
        assert_eq!(c.plane, PlaneId::LocalSidecar);
    }

    #[test]
    fn auto_sidecar_file_does_not_bypass_vram() {
        // Krackan-class: sidecar on PATH + 512 MiB shared must not win auto.
        let spec = image_spec(Quality::Standard, ComputeMode::Auto);
        let probe = ProbeSnapshot {
            devices: vec![DeviceProbe {
                kind: DeviceKind::GpuVulkan,
                vram_mb: Some(512),
                shared: true,
                slow: true,
                name: Some("AMD Radeon 840M".into()),
            }],
            sidecar_alive: true,
            tripo_key: true,
            keys_present: true,
            disk_free_mb: 100_000,
            ..ProbeSnapshot::default()
        };
        let c = plan(&spec, &probe, &open()).unwrap();
        assert_eq!(c.plane, PlaneId::RemoteTripo);
    }

    #[test]
    fn analytic_absent_refuses() {
        let spec = JobSubmit {
            prompt: Some("box 10x10x10 mm".into()),
            route: Route::Analytic,
            ..JobSubmit::default()
        };
        let err = plan(&spec, &cpu(), &closed()).unwrap_err();
        assert_eq!(err.error_type, error_type::ANALYTIC_UNAVAILABLE);
    }

    #[test]
    fn auto_never_mock_without_flag() {
        let spec = image_spec(Quality::Standard, ComputeMode::Auto);
        let err = plan(&spec, &cpu(), &closed()).unwrap_err();
        assert_ne!(
            plan(
                &spec,
                &ProbeSnapshot {
                    allow_mock: false,
                    ..cpu()
                },
                &closed()
            )
            .ok()
            .map(|c| c.plane),
            Some(PlaneId::LocalMock)
        );
        assert_eq!(err.error_type, error_type::WEIGHTS_MISSING);
    }
}
