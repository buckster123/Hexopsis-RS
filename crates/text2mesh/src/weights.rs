//! Weight catalog, license stamps, and CLI `weights pull`.
//!
//! Never auto-pull on generate (FR-CMP-18). Hunyuan ids refuse closed (D19).
//! DINOv3: file on disk + flag off → present:true, accepted:false.

use std::fs;
use std::path::{Path, PathBuf};

use crate::config::env_truthy;
use crate::error::{error_type, Error};
use crate::probe::disk_free_mb;
use crate::types::WeightRow;

/// Catalog ids we name (design §13). Not vendor checkpoint filenames.
pub const PREVIEW_FEEDFORWARD: &str = "preview.feedforward";
pub const QUALITY_STACK: &str = "quality.stack";
pub const ENCODER_DINOV3: &str = "encoder.dinov3_vitl16";
pub const ENCODER_OPENCLIP: &str = "encoder.openclip_vit_b32";
pub const NATIVE_TEXT_DIT: &str = "native.text_dit";

/// PRD alias → catalog id.
const DINOV3_ALIASES: &[&str] = &["encoder.dinov3", "encoder.dinov3_vitl16", "dinov3"];

#[derive(Debug, Clone, Copy)]
pub struct WeightMeta {
    pub id: &'static str,
    pub license_tag: &'static str,
    pub license_label: &'static str,
    pub want_bytes: u64,
}

pub const CATALOG: &[WeightMeta] = &[
    WeightMeta {
        id: PREVIEW_FEEDFORWARD,
        license_tag: "mit",
        license_label: "MIT",
        want_bytes: 2200 * 1024 * 1024,
    },
    WeightMeta {
        id: QUALITY_STACK,
        license_tag: "mit",
        license_label: "MIT",
        want_bytes: 16 * 1024 * 1024 * 1024,
    },
    WeightMeta {
        id: ENCODER_DINOV3,
        license_tag: "dinov3",
        license_label: "DINOv3",
        want_bytes: 607_000_000,
    },
    WeightMeta {
        id: ENCODER_OPENCLIP,
        license_tag: "mit",
        license_label: "MIT/OpenCLIP",
        want_bytes: 350_000_000,
    },
    WeightMeta {
        id: NATIVE_TEXT_DIT,
        license_tag: "mit",
        license_label: "MIT",
        want_bytes: 4 * 1024 * 1024 * 1024,
    },
];

pub fn weights_dir() -> PathBuf {
    if let Ok(p) = std::env::var("TEXT2MESH_WEIGHTS_DIR") {
        if !p.is_empty() {
            return PathBuf::from(p);
        }
    }
    match std::env::var("TEXT2MESH_STORE") {
        Ok(s) if !s.is_empty() => PathBuf::from(s).join("weights"),
        _ => dirs::data_dir()
            .unwrap_or_else(|| {
                std::env::var("HOME")
                    .map(|h| PathBuf::from(h).join(".local/share"))
                    .unwrap_or_else(|_| PathBuf::from(".").join(".local/share"))
            })
            .join("text2mesh")
            .join("weights"),
    }
}

pub fn resolve_id(id: &str) -> Option<&'static WeightMeta> {
    let id = id.trim();
    if DINOV3_ALIASES.contains(&id) {
        return CATALOG.iter().find(|m| m.id == ENCODER_DINOV3);
    }
    CATALOG.iter().find(|m| m.id == id)
}

pub fn is_hunyuan_id(id: &str) -> bool {
    let n = id.to_ascii_lowercase();
    n.contains("hunyuan") || n.contains("hy3d")
}

pub fn scan_dir(dir: &Path) -> Vec<WeightRow> {
    let dinov3_env = env_truthy("TEXT2MESH_ACCEPT_DINOV3");
    CATALOG
        .iter()
        .map(|m| row_for(dir, m, dinov3_env))
        .collect()
}

pub fn scan_env() -> Vec<WeightRow> {
    scan_dir(&weights_dir())
}

pub fn quality_present(rows: &[WeightRow]) -> bool {
    rows.iter().any(|r| r.id == QUALITY_STACK && r.present)
}

pub fn preview_present(rows: &[WeightRow]) -> bool {
    rows.iter()
        .any(|r| r.id == PREVIEW_FEEDFORWARD && r.present)
}

pub fn dinov3_accepted(rows: &[WeightRow]) -> bool {
    rows.iter()
        .find(|r| r.id == ENCODER_DINOV3)
        .map(|r| r.accepted)
        .unwrap_or_else(|| env_truthy("TEXT2MESH_ACCEPT_DINOV3"))
}

pub fn dinov3_present_unaccepted(rows: &[WeightRow]) -> bool {
    rows.iter()
        .any(|r| r.id == ENCODER_DINOV3 && r.present && !r.accepted)
}

/// MIT weights are accepted by being present; DINOv3 needs the flag or stamp.
pub fn licenses_ok(rows: &[WeightRow]) -> bool {
    !dinov3_present_unaccepted(rows)
}

pub fn catalog_empty_rows() -> Vec<WeightRow> {
    CATALOG.iter().map(empty_row).collect()
}

fn empty_row(m: &WeightMeta) -> WeightRow {
    WeightRow {
        id: m.id.into(),
        present: false,
        want_bytes: Some(m.want_bytes),
        have_bytes: None,
        path: Some(display_path(&weights_dir().join(m.id))),
        sha256_head: None,
        license: Some(m.license_label.into()),
        accepted: false,
    }
}

fn row_for(dir: &Path, m: &WeightMeta, dinov3_env: bool) -> WeightRow {
    let dest = dir.join(m.id);
    let (present, have, sha) = measure(&dest);
    let stamped = license_stamped(dir, m);
    let accepted = if m.license_tag == "dinov3" {
        dinov3_env || stamped
    } else {
        present || stamped
    };
    WeightRow {
        id: m.id.into(),
        present,
        want_bytes: Some(m.want_bytes),
        have_bytes: have,
        path: Some(display_path(&dest)),
        sha256_head: sha,
        license: Some(m.license_label.into()),
        accepted,
    }
}

fn license_stamped(dir: &Path, m: &WeightMeta) -> bool {
    let path = stamp_path(dir, m.id);
    let Ok(text) = fs::read_to_string(&path) else {
        return false;
    };
    text.to_ascii_lowercase()
        .contains(&m.license_tag.to_ascii_lowercase())
}

fn stamp_path(dir: &Path, id: &str) -> PathBuf {
    dir.join(format!("{id}.license"))
}

fn measure(path: &Path) -> (bool, Option<u64>, Option<String>) {
    if path.is_file() {
        return file_measure(path);
    }
    if !path.is_dir() {
        return (false, None, None);
    }
    let mut total = 0u64;
    let mut first_file: Option<PathBuf> = None;
    if let Ok(rd) = fs::read_dir(path) {
        for ent in rd.flatten() {
            let p = ent.path();
            let name = ent.file_name().to_string_lossy().to_ascii_lowercase();
            if name.ends_with(".license") || name == "readme" || name.starts_with('.') {
                continue;
            }
            if let Ok(meta) = ent.metadata() {
                if meta.is_file() {
                    total = total.saturating_add(meta.len());
                    if first_file.is_none() {
                        first_file = Some(p);
                    }
                }
            }
        }
    }
    if total == 0 {
        return (false, None, None);
    }
    let sha = first_file.as_deref().and_then(sha256_head_file);
    (true, Some(total), sha)
}

fn file_measure(path: &Path) -> (bool, Option<u64>, Option<String>) {
    let Ok(meta) = fs::metadata(path) else {
        return (false, None, None);
    };
    if meta.len() == 0 {
        return (false, Some(0), None);
    }
    (true, Some(meta.len()), sha256_head_file(path))
}

fn sha256_head_file(path: &Path) -> Option<String> {
    let bytes = fs::read(path).ok()?;
    if bytes.is_empty() {
        return None;
    }
    let label = crate::hash::sha256_bytes(&bytes);
    let hex = label.trim_start_matches("sha256:");
    Some(hex.chars().take(8).collect())
}

fn display_path(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

/// CLI `weights pull ID --accept-license TAG`. Does not run from generate.
pub fn pull(id: &str, accept_license: &str) -> Result<WeightRow, Error> {
    pull_in(
        &weights_dir(),
        id,
        accept_license,
        disk_free_mb(&weights_dir()),
    )
}

pub fn pull_in(
    dir: &Path,
    id: &str,
    accept_license: &str,
    free_mb: Option<u64>,
) -> Result<WeightRow, Error> {
    if is_hunyuan_id(id) {
        return Err(Error::new(
            error_type::LICENSE_BLOCKED,
            "Hunyuan community weights are blocked_by_default (territory EU/UK/KR, MAU, no-train)",
        )
        .with_hint("D19: no default download; remote.hunyuan_hosted needs key + ALLOW_HUNYUAN + attestation + license_override"));
    }
    let Some(meta) = resolve_id(id) else {
        return Err(Error::new(
            error_type::SPEC_REJECTED,
            format!("unknown weight id {id}"),
        )
        .with_hint("catalog: preview.feedforward, quality.stack, encoder.dinov3_vitl16, encoder.openclip_vit_b32, native.text_dit"));
    };
    let tag = accept_license.trim().to_ascii_lowercase();
    let expected = meta.license_tag.to_ascii_lowercase();
    if tag != expected && !(expected == "mit" && (tag == "apache-2.0" || tag == "openclip")) {
        return Err(Error::new(
            error_type::LICENSE_BLOCKED,
            format!(
                "license tag {accept_license} does not match {} (need --accept-license {})",
                meta.id, meta.license_tag
            ),
        ));
    }

    let want_mb = (meta.want_bytes / (1024 * 1024)).max(1);
    let need = ((want_mb as f64) * 1.1).ceil() as u64;
    if let Some(free) = free_mb {
        if free < need {
            return Err(Error::new(
                error_type::DISK_SHORT,
                format!("free_mb={free} < want_mb={want_mb} * 1.1 ({need})"),
            )
            .with_hint("free disk or pull a smaller catalog id"));
        }
    }

    fs::create_dir_all(dir)?;
    fs::write(stamp_path(dir, meta.id), format!("{}\n", meta.license_tag))?;

    if let Ok(src) = std::env::var("TEXT2MESH_WEIGHTS_SRC") {
        if !src.is_empty() {
            let src_path = PathBuf::from(&src);
            if src_path.is_file() {
                let dest = dir.join(meta.id);
                fs::copy(&src_path, &dest)?;
            }
        }
    }

    let mut row = row_for(dir, meta, env_truthy("TEXT2MESH_ACCEPT_DINOV3"));
    if meta.license_tag == "dinov3" {
        row.accepted = true;
    }
    Ok(row)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn without_dinov3_env<T>(f: impl FnOnce() -> T) -> T {
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let prev = std::env::var("TEXT2MESH_ACCEPT_DINOV3").ok();
        // SAFETY: tests serialize env mutation via ENV_LOCK.
        unsafe { std::env::remove_var("TEXT2MESH_ACCEPT_DINOV3") };
        let out = f();
        match prev {
            Some(v) => unsafe { std::env::set_var("TEXT2MESH_ACCEPT_DINOV3", v) },
            None => unsafe { std::env::remove_var("TEXT2MESH_ACCEPT_DINOV3") },
        }
        out
    }

    #[test]
    fn hunyuan_ids_refuse() {
        for id in [
            "hunyuan",
            "hunyuan.community",
            "quality.hunyuan",
            "tencent-hunyuan-2.1",
            "hy3d",
        ] {
            let err = pull_in(Path::new("/tmp"), id, "mit", Some(100_000)).unwrap_err();
            assert_eq!(err.error_type, error_type::LICENSE_BLOCKED, "{id}");
        }
    }

    #[test]
    fn disk_gate_refuses_when_tight() {
        let dir = tempfile::tempdir().unwrap();
        let err = pull_in(dir.path(), QUALITY_STACK, "mit", Some(100)).unwrap_err();
        assert_eq!(err.error_type, error_type::DISK_SHORT);
        assert!(
            !dir.path()
                .join(format!("{QUALITY_STACK}.license"))
                .is_file(),
            "must not stamp license after disk refuse"
        );
    }

    #[test]
    fn dinov3_present_not_accepted_without_flag() {
        without_dinov3_env(|| {
            let dir = tempfile::tempdir().unwrap();
            let dest = dir.path().join(ENCODER_DINOV3);
            fs::write(&dest, vec![0u8; 64]).unwrap();
            let rows = scan_dir(dir.path());
            let dino = rows.iter().find(|r| r.id == ENCODER_DINOV3).unwrap();
            assert!(dino.present);
            assert!(!dino.accepted);
            assert!(dinov3_present_unaccepted(&rows));
            assert!(!licenses_ok(&rows));
        });
    }

    #[test]
    fn dinov3_pull_stamps_accepted() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join(ENCODER_DINOV3);
        fs::write(&dest, vec![1u8; 32]).unwrap();
        let row = pull_in(dir.path(), "encoder.dinov3", "dinov3", Some(10_000)).unwrap();
        assert_eq!(row.id, ENCODER_DINOV3);
        assert!(row.present);
        assert!(row.accepted);
        let rows = scan_dir(dir.path());
        assert!(licenses_ok(&rows));
    }

    #[test]
    fn mit_pull_writes_stamp_without_auto_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let row = pull_in(dir.path(), PREVIEW_FEEDFORWARD, "MIT", Some(10_000)).unwrap();
        assert!(!row.present, "no fetch URL → stamp only, not fake weights");
        assert!(dir
            .path()
            .join(format!("{PREVIEW_FEEDFORWARD}.license"))
            .is_file());
    }

    #[test]
    fn unknown_id() {
        let dir = tempfile::tempdir().unwrap();
        let err = pull_in(dir.path(), "vendor.secret.bin", "mit", Some(10_000)).unwrap_err();
        assert_eq!(err.error_type, error_type::SPEC_REJECTED);
    }

    #[test]
    fn catalog_has_five() {
        assert_eq!(CATALOG.len(), 5);
    }
}
