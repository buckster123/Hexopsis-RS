//! G0–G4 consistency gates (design §6). G3/G4 are pure image stats.

use image::{Rgba, RgbaImage};

use crate::contract::ViewContract;
use crate::error::error_type;

pub const GATE_VERSION: &str = "g0_v0";
pub const FACE: &str = "a face, two eyes, front of a head";
pub const BACK: &str = "the back of a head, no face";
pub const BG_DEFAULT: [u8; 3] = [0xB4, 0xB4, 0xB4];

#[derive(Debug, Clone)]
pub struct GateScores {
    pub gate_version: String,
    pub encoder: String,
    pub g0: Option<f32>,
    pub g1_mean: Option<f32>,
    pub g2: Option<f32>,
    pub g3: Option<f32>,
    pub g4: Option<f32>,
    pub failed: Vec<String>,
}

fn chebyshev_bg(p: Rgba<u8>, bg: [u8; 3]) -> bool {
    (p[0] as i16 - bg[0] as i16).unsigned_abs() <= 18
        && (p[1] as i16 - bg[1] as i16).unsigned_abs() <= 18
        && (p[2] as i16 - bg[2] as i16).unsigned_abs() <= 18
}

pub fn subject_mask(img: &RgbaImage, bg: [u8; 3]) -> Vec<bool> {
    let has_alpha = img.pixels().any(|p| p[3] < 255);
    img.pixels()
        .map(|p| {
            if has_alpha {
                p[3] >= 16
            } else {
                !chebyshev_bg(*p, bg)
            }
        })
        .collect()
}

pub fn g3_framing(img: &RgbaImage, bg: [u8; 3]) -> Result<f32, &'static str> {
    let w = img.width() as usize;
    let h = img.height() as usize;
    let mask = subject_mask(img, bg);
    let n = mask.len();
    let sub = mask.iter().filter(|b| **b).count();
    if n == 0 {
        return Err("empty");
    }
    let frac = sub as f32 / n as f32;
    if !(0.28..=0.82).contains(&frac) {
        return Err("fraction");
    }
    let mut min_x = w;
    let mut max_x = 0;
    let mut min_y = h;
    let mut max_y = 0;
    for y in 0..h {
        for x in 0..w {
            if mask[y * w + x] {
                min_x = min_x.min(x);
                max_x = max_x.max(x);
                min_y = min_y.min(y);
                max_y = max_y.max(y);
            }
        }
    }
    if sub == 0 {
        return Err("empty");
    }
    let glue_lr = min_x <= 4 && max_x + 4 >= w.saturating_sub(1);
    let glue_tb = min_y <= 4 && max_y + 4 >= h.saturating_sub(1);
    if glue_lr || glue_tb {
        return Err("bbox");
    }
    Ok(frac)
}

fn luma(p: Rgba<u8>) -> f32 {
    (0.2126 * p[0] as f32 + 0.7152 * p[1] as f32 + 0.0722 * p[2] as f32) / 255.0
}

pub fn g4_lighting(canon: &RgbaImage, other: &RgbaImage, bg: [u8; 3]) -> Result<f32, &'static str> {
    let (yc, rc) = bbox_stats(canon, bg).ok_or("canon")?;
    let (yo, ro) = bbox_stats(other, bg).ok_or("other")?;
    if yc == 0.0 {
        return Err("dark");
    }
    let rel = ((yo - yc) / yc).abs();
    if rel > 0.18 {
        return Err("luma");
    }
    for i in 0..3 {
        if (ro[i] - rc[i]).abs() > 0.15 {
            return Err("grayworld");
        }
    }
    Ok(rel)
}

fn bbox_stats(img: &RgbaImage, bg: [u8; 3]) -> Option<(f32, [f32; 3])> {
    let w = img.width() as usize;
    let h = img.height() as usize;
    let mask = subject_mask(img, bg);
    let mut min_x = w;
    let mut max_x = 0;
    let mut min_y = h;
    let mut max_y = 0;
    let mut any = false;
    for y in 0..h {
        for x in 0..w {
            if mask[y * w + x] {
                any = true;
                min_x = min_x.min(x);
                max_x = max_x.max(x);
                min_y = min_y.min(y);
                max_y = max_y.max(y);
            }
        }
    }
    if !any {
        return None;
    }
    let mut ysum = 0.0f32;
    let mut rgb = [0.0f32; 3];
    let mut n = 0u32;
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let p = *img.get_pixel(x as u32, y as u32);
            ysum += luma(p);
            rgb[0] += p[0] as f32;
            rgb[1] += p[1] as f32;
            rgb[2] += p[2] as f32;
            n += 1;
        }
    }
    if n == 0 {
        return None;
    }
    let inv = 1.0 / n as f32;
    let sum = rgb[0] + rgb[1] + rgb[2];
    let ratios = if sum == 0.0 {
        [1.0 / 3.0; 3]
    } else {
        [rgb[0] / sum, rgb[1] / sum, rgb[2] / sum]
    };
    Some((ysum * inv, ratios))
}

/// G3/G4 only (no CLIP). Returns failed error_types.
pub fn score_g3_g4(contract: &ViewContract, views: &[(String, RgbaImage)]) -> GateScores {
    let bg = parse_hex(&contract.background.hex).unwrap_or(BG_DEFAULT);
    let canon_id = contract.subject_lock.canonical_view_id.as_str();
    let canon = views.iter().find(|(id, _)| id == canon_id).map(|(_, i)| i);
    let mut failed = Vec::new();
    let mut g3 = None;
    let mut g4 = None;
    let required: Vec<&str> = contract
        .camera_ring
        .cameras
        .iter()
        .filter(|c| c.required)
        .map(|c| c.id.as_str())
        .collect();
    for id in &required {
        let Some((_, img)) = views.iter().find(|(i, _)| i == id) else {
            failed.push(error_type::VIEW_FRAMING.into());
            continue;
        };
        match g3_framing(img, bg) {
            Ok(f) => g3 = Some(f),
            Err(_) => failed.push(error_type::VIEW_FRAMING.into()),
        }
        if let Some(c) = canon {
            match g4_lighting(c, img, bg) {
                Ok(f) => g4 = Some(f),
                Err(_) => failed.push(error_type::VIEW_LIGHTING_DRIFT.into()),
            }
        }
    }
    failed.sort();
    failed.dedup();
    GateScores {
        gate_version: GATE_VERSION.into(),
        encoder: "none".into(),
        g0: None,
        g1_mean: None,
        g2: None,
        g3,
        g4,
        failed,
    }
}

fn parse_hex(s: &str) -> Option<[u8; 3]> {
    let t = s.trim().trim_start_matches('#');
    if t.len() != 6 {
        return None;
    }
    Some([
        u8::from_str_radix(&t[0..2], 16).ok()?,
        u8::from_str_radix(&t[2..4], 16).ok()?,
        u8::from_str_radix(&t[4..6], 16).ok()?,
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gray_center() -> RgbaImage {
        let mut img = RgbaImage::from_pixel(64, 64, Rgba([0xB4, 0xB4, 0xB4, 255]));
        for y in 14..50 {
            for x in 14..50 {
                img.put_pixel(x, y, Rgba([180, 80, 40, 255]));
            }
        }
        img
    }

    fn glued() -> RgbaImage {
        let mut img = RgbaImage::from_pixel(64, 64, Rgba([0xB4, 0xB4, 0xB4, 255]));
        for y in 0..64 {
            for x in 0..64 {
                img.put_pixel(x, y, Rgba([10, 10, 10, 255]));
            }
        }
        img
    }

    #[test]
    fn g3_center_passes() {
        assert!(g3_framing(&gray_center(), BG_DEFAULT).is_ok());
    }

    #[test]
    fn g3_glued_fails() {
        assert!(g3_framing(&glued(), BG_DEFAULT).is_err());
    }

    #[test]
    fn g4_same_passes() {
        let a = gray_center();
        assert!(g4_lighting(&a, &a, BG_DEFAULT).is_ok());
    }

    #[test]
    fn face_back_strings_frozen() {
        assert_eq!(FACE, "a face, two eyes, front of a head");
        assert_eq!(BACK, "the back of a head, no face");
    }
}
