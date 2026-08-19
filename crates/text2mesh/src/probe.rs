//! Device / disk probe. Count **device VRAM**, never host RAM or GTT.
//!
//! Probe budgets (design §24): 5 s per command, 20 s total. Must not be reused
//! as sidecar generate or vendor poll timers.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::types::{DeviceKind, DeviceProbe};

const CMD_BUDGET: Duration = Duration::from_secs(5);
const TOTAL_BUDGET: Duration = Duration::from_secs(20);

/// Always includes CPU. Never treats GTT, rocminfo pools, or vulkan host heaps as VRAM.
pub fn probe_devices() -> Vec<DeviceProbe> {
    let start = Instant::now();
    let mut out = vec![DeviceProbe {
        kind: DeviceKind::Cpu,
        vram_mb: None,
        shared: false,
        slow: true,
        name: None,
    }];

    let amd = read_amd_sysfs();
    let vulkan = if start.elapsed() < TOTAL_BUDGET {
        parse_vulkan_summary(
            &capture_timeout("vulkaninfo", &["--summary"], CMD_BUDGET).unwrap_or_default(),
        )
    } else {
        None
    };
    let nvidia = if start.elapsed() < TOTAL_BUDGET {
        parse_nvidia_smi(
            &capture_timeout(
                "nvidia-smi",
                &[
                    "--query-gpu=name,memory.total",
                    "--format=csv,noheader,nounits",
                ],
                CMD_BUDGET,
            )
            .unwrap_or_default(),
        )
    } else {
        None
    };
    let rocm_name = if start.elapsed() < TOTAL_BUDGET {
        parse_rocminfo_gpu_name(&capture_timeout("rocminfo", &[], CMD_BUDGET).unwrap_or_default())
    } else {
        None
    };

    if let Some(n) = nvidia {
        out.push(n);
    }

    if let Some(amd) = amd {
        let name = vulkan
            .as_ref()
            .and_then(|v| v.name.clone())
            .or(rocm_name.clone())
            .or_else(|| Some("AMD GPU".into()));
        let shared = amd.shared || vulkan.as_ref().is_some_and(|v| v.shared);
        let vram_mb = Some(amd.vram_mb);
        if rocm_name.is_some() {
            out.push(DeviceProbe {
                kind: DeviceKind::AmdRocm,
                vram_mb,
                shared,
                slow: shared || amd.vram_mb < 6144,
                name: name.clone(),
            });
        }
        out.push(DeviceProbe {
            kind: DeviceKind::GpuVulkan,
            vram_mb,
            shared,
            slow: shared || amd.vram_mb < 6144,
            name,
        });
    } else if let Some(v) = vulkan {
        // INTEGRATED_GPU without sysfs: honest unknown carve-out, never vulkan host heaps.
        let vram = if v.shared { None } else { v.vram_mb };
        out.push(DeviceProbe {
            kind: DeviceKind::GpuVulkan,
            vram_mb: vram,
            shared: v.shared,
            slow: v.shared || v.vram_mb.unwrap_or(0) < 6144,
            name: v.name,
        });
        if let Some(name) = rocm_name {
            out.push(DeviceProbe {
                kind: DeviceKind::AmdRocm,
                vram_mb: vram,
                shared: v.shared,
                slow: true,
                name: Some(name),
            });
        }
    } else if let Some(name) = rocm_name {
        // rocminfo GPU exists but we refuse its pool sizes (host RAM on iGPU).
        out.push(DeviceProbe {
            kind: DeviceKind::AmdRocm,
            vram_mb: None,
            shared: true,
            slow: true,
            name: Some(name),
        });
    }

    if cfg!(target_os = "macos") {
        out.push(DeviceProbe {
            kind: DeviceKind::AppleMetal,
            vram_mb: None,
            shared: true,
            slow: false,
            name: Some("Apple GPU".into()),
        });
    }

    out
}

/// Free MiB on the filesystem that holds `path` (or its first existing ancestor).
pub fn disk_free_mb(path: &Path) -> Option<u64> {
    let target = existing_ancestor(path)?;
    let text = capture_timeout("df", &["-Pk", &target.to_string_lossy()], CMD_BUDGET)?;
    parse_df_pk(&text)
}

pub fn host_ram_mb() -> Option<u64> {
    let text = std::fs::read_to_string("/proc/meminfo").ok()?;
    parse_memtotal_mb(&text)
}

struct AmdSysfs {
    vram_mb: u32,
    shared: bool,
}

#[derive(Debug, Clone)]
struct VulkanHint {
    name: Option<String>,
    shared: bool,
    /// Only filled for discrete GPUs; iGPU heaps are host RAM on RADV.
    vram_mb: Option<u32>,
}

pub fn parse_df_pk(text: &str) -> Option<u64> {
    let line = text.lines().nth(1)?;
    let avail_k: u64 = line.split_whitespace().nth(3)?.parse().ok()?;
    Some(avail_k / 1024)
}

pub fn parse_memtotal_mb(meminfo: &str) -> Option<u64> {
    for line in meminfo.lines() {
        if let Some(rest) = line.strip_prefix("MemTotal:") {
            let kb: u64 = rest.split_whitespace().next()?.parse().ok()?;
            return Some(kb / 1024);
        }
    }
    None
}

fn parse_nvidia_smi(text: &str) -> Option<DeviceProbe> {
    let line = text.lines().map(str::trim).find(|l| !l.is_empty())?;
    let mut parts = line.split(',').map(str::trim);
    let name = parts.next()?.to_string();
    if name.is_empty() || name.to_ascii_lowercase().contains("failed") {
        return None;
    }
    let mb: u32 = parts.next()?.split_whitespace().next()?.parse().ok()?;
    Some(DeviceProbe {
        kind: DeviceKind::NvidiaCuda,
        vram_mb: Some(mb),
        shared: false,
        slow: mb < 6144,
        name: Some(name),
    })
}

/// First non-CPU physical GPU from `vulkaninfo --summary`. Ignores llvmpipe.
fn parse_vulkan_summary(text: &str) -> Option<VulkanHint> {
    let mut device_type = String::new();
    let mut name = String::new();
    for line in text.lines() {
        let t = line.trim();
        if let Some(v) = t.strip_prefix("deviceType") {
            device_type = v.trim_start_matches([' ', '=']).trim().to_string();
            name.clear();
        } else if let Some(v) = t.strip_prefix("deviceName") {
            name = v.trim_start_matches([' ', '=']).trim().to_string();
            if is_software_raster(name.as_str()) || device_type.contains("CPU") {
                name.clear();
                continue;
            }
            if !device_type.is_empty() {
                let shared = device_type.contains("INTEGRATED");
                return Some(VulkanHint {
                    name: Some(pretty_gpu_name(&name)),
                    shared,
                    vram_mb: None,
                });
            }
        }
    }
    None
}

/// Marketing name of the first GPU agent. **Never** read pool Size (host RAM).
fn parse_rocminfo_gpu_name(text: &str) -> Option<String> {
    let mut name: Option<String> = None;
    let mut marketing: Option<String> = None;
    for line in text.lines() {
        let t = line.trim();
        if let Some(v) = t.strip_prefix("Marketing Name:") {
            marketing = Some(v.trim().to_string());
        } else if let Some(v) = t.strip_prefix("Name:") {
            name = Some(v.trim().to_string());
        } else if t.starts_with("Device Type:") && t.ends_with("GPU") {
            let raw = marketing
                .clone()
                .filter(|s| !s.is_empty())
                .or(name.clone())?;
            if raw.contains("CPU") || raw.starts_with("gfx") {
                continue;
            }
            return Some(pretty_gpu_name(&raw));
        }
    }
    None
}

fn parse_amd_sysfs_pair(vram_total: u64, gtt_total: u64) -> Option<(u32, bool)> {
    if vram_total == 0 {
        return None;
    }
    let vram_mb = bytes_to_mb(vram_total);
    // GTT is host RAM the iGPU can snoop. Never add it to vram_mb.
    let shared = gtt_total > vram_total.saturating_mul(4) || vram_mb < 6144;
    Some((vram_mb, shared))
}

fn read_amd_sysfs() -> Option<AmdSysfs> {
    let drm = Path::new("/sys/class/drm");
    let entries = std::fs::read_dir(drm).ok()?;
    for ent in entries.flatten() {
        let name = ent.file_name();
        let name = name.to_string_lossy();
        if !is_primary_card(&name) {
            continue;
        }
        let dev = ent.path().join("device");
        let vendor = std::fs::read_to_string(dev.join("vendor")).ok()?;
        if !vendor.trim().eq_ignore_ascii_case("0x1002") {
            continue;
        }
        let vram = read_u64_file(&dev.join("mem_info_vram_total"))?;
        let gtt = read_u64_file(&dev.join("mem_info_gtt_total")).unwrap_or(0);
        let (vram_mb, shared) = parse_amd_sysfs_pair(vram, gtt)?;
        return Some(AmdSysfs { vram_mb, shared });
    }
    None
}

fn is_primary_card(name: &str) -> bool {
    let Some(rest) = name.strip_prefix("card") else {
        return false;
    };
    !rest.is_empty() && rest.bytes().all(|b| b.is_ascii_digit())
}

fn read_u64_file(path: &Path) -> Option<u64> {
    std::fs::read_to_string(path).ok()?.trim().parse().ok()
}

fn bytes_to_mb(bytes: u64) -> u32 {
    (bytes / (1024 * 1024)) as u32
}

fn pretty_gpu_name(raw: &str) -> String {
    let cut = raw
        .split(" Graphics")
        .next()
        .unwrap_or(raw)
        .split(" (")
        .next()
        .unwrap_or(raw)
        .trim();
    cut.to_string()
}

fn is_software_raster(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    n.contains("llvmpipe")
        || n.contains("lavapipe")
        || n.contains("swiftshader")
        || n.contains("softpipe")
}

fn existing_ancestor(path: &Path) -> Option<PathBuf> {
    let mut p = if path.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        path.to_path_buf()
    };
    loop {
        if p.exists() {
            return Some(p);
        }
        if !p.pop() {
            return Some(PathBuf::from("."));
        }
    }
}

fn capture_timeout(bin: &str, args: &[&str], budget: Duration) -> Option<String> {
    let mut child = Command::new(bin)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let mut stdout = child.stdout.take()?;
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stdout.read_to_end(&mut buf);
        let _ = tx.send(buf);
    });
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let buf = rx.recv_timeout(Duration::from_millis(200)).ok()?;
                if !status.success() {
                    return None;
                }
                return Some(String::from_utf8_lossy(&buf).into_owned());
            }
            Ok(None) if start.elapsed() >= budget => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(_) => {
                let _ = child.kill();
                return None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn df_pk_parses_available_mb() {
        let sample = "\
Filesystem     1024-blocks      Used Available Capacity Mounted on
/dev/nvme0n1p5   561964032 383686656 149658624      72% /\n";
        assert_eq!(parse_df_pk(sample), Some(146151));
    }

    #[test]
    fn memtotal_is_not_vram() {
        let sample = "MemTotal:       23342128 kB\nMemAvailable:   13423832 kB\n";
        assert_eq!(parse_memtotal_mb(sample), Some(22795));
    }

    #[test]
    fn amd_sysfs_512_shared_not_gtt() {
        let vram = 536870912u64;
        let gtt = 21474836480u64;
        let (mb, shared) = parse_amd_sysfs_pair(vram, gtt).unwrap();
        assert_eq!(mb, 512);
        assert!(shared);
        assert!(mb < 2048, "must not report GTT 20 GiB as VRAM");
    }

    #[test]
    fn vulkan_summary_picks_840m_ignores_llvmpipe() {
        let sample = r#"
GPU0:
deviceType         = PHYSICAL_DEVICE_TYPE_INTEGRATED_GPU
deviceName         = AMD Radeon 840M Graphics (RADV KRACKAN1)
GPU1:
deviceType         = PHYSICAL_DEVICE_TYPE_CPU
deviceName         = llvmpipe (LLVM 21.1.8, 256 bits)
"#;
        let v = parse_vulkan_summary(sample).unwrap();
        assert_eq!(v.name.as_deref(), Some("AMD Radeon 840M"));
        assert!(v.shared);
        assert!(v.vram_mb.is_none(), "must not invent heap sizes");
    }

    #[test]
    fn vulkan_summary_skips_only_llvmpipe() {
        let sample = r#"
GPU0:
deviceType         = PHYSICAL_DEVICE_TYPE_CPU
deviceName         = llvmpipe (LLVM 21.1.8, 256 bits)
"#;
        assert!(parse_vulkan_summary(sample).is_none());
    }

    #[test]
    fn rocminfo_name_not_pool_size() {
        let sample = r#"
*******
Agent 1
*******
  Marketing Name:          AMD Ryzen AI 5 340 w/ Radeon 840M
  Device Type:             CPU
    Pool 1
      Size:                    23342128(0x1642c30) KB
*******
Agent 2
*******
  Marketing Name:          AMD Radeon 840M Graphics
  Device Type:             GPU
    Pool 1
      Size:                    11671064(0xb21618) KB
"#;
        assert_eq!(
            parse_rocminfo_gpu_name(sample).as_deref(),
            Some("AMD Radeon 840M")
        );
    }

    #[test]
    fn nvidia_smi_csv() {
        let d = parse_nvidia_smi("NVIDIA GeForce RTX 4090, 24564\n").unwrap();
        assert_eq!(d.kind, DeviceKind::NvidiaCuda);
        assert_eq!(d.vram_mb, Some(24564));
        assert!(!d.shared);
    }

    #[test]
    fn live_amd_sysfs_matches_carve_out() {
        let Some(amd) = read_amd_sysfs() else {
            return;
        };
        assert!(
            amd.vram_mb < 4096,
            "AMD sysfs vram_mb={} looks like GTT/host, not the carve-out",
            amd.vram_mb
        );
        assert!(amd.shared);
        if let Some(ram) = host_ram_mb() {
            assert!(
                u64::from(amd.vram_mb) + 2048 < ram,
                "vram_mb must not be host RAM ({ram})"
            );
        }
    }

    #[test]
    fn live_probe_never_uses_host_ram_as_dedicated_vram() {
        let devices = probe_devices();
        let ram = host_ram_mb();
        for d in &devices {
            if d.kind == DeviceKind::Cpu {
                assert!(d.vram_mb.is_none());
                continue;
            }
            if d.shared {
                assert!(
                    d.vram_mb.unwrap_or(0) < 4096,
                    "{:?} shared vram_mb={:?} must be the iGPU carve-out, not GTT",
                    d.kind,
                    d.vram_mb
                );
            }
            if let (Some(v), Some(ram)) = (d.vram_mb, ram) {
                assert!(
                    u64::from(v) + 1024 < ram || d.shared,
                    "{:?} vram_mb={v} ≈ host RAM {ram}",
                    d.kind
                );
            }
        }
    }

    #[test]
    fn primary_card_name() {
        assert!(is_primary_card("card1"));
        assert!(!is_primary_card("card1-DP-1"));
        assert!(!is_primary_card("renderD128"));
    }
}
