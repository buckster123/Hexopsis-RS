//! Pure View Contract compiler (design §4). No network, no LLM.

use unicode_normalization::UnicodeNormalization;

use crate::classify::classify;
use crate::contract::{
    Background, Camera, CameraRing, ContractPrompt, Frame, Lighting, SeedPolicy, StyleLock,
    SubjectLock, T2iRef, ViewContract, JANUS_NEGATIVES, OTHER_NEGATIVES, VIEW_CONTRACT_SCHEMA,
};
use crate::error::{error_type, Error};
use crate::hash::sha256_str;
use crate::types::{CameraPreset, PromptClass, Quality, T2iProviderId};

const CAMERA_PHRASES: &[&str] = &[
    "front view",
    "side view",
    "back view",
    "top view",
    "bottom view",
    "three-quarter",
    "three quarter",
    "wide-angle",
    "wide angle",
    "close-up",
    "closeup",
    "bird's eye",
    "birds eye",
    "worm's eye",
    "orthographic",
    "isometric",
    "3/4",
];

pub fn normalize_prompt(raw: &str) -> String {
    let nfc: String = raw.nfc().collect();
    let trimmed = nfc.trim();
    let mut out = String::new();
    let mut prev_space = false;
    for c in trimmed.chars() {
        if c == ' ' || c == '\t' || c == '\n' || c == '\r' {
            if !prev_space {
                out.push(' ');
                prev_space = true;
            }
        } else {
            out.push(c);
            prev_space = false;
        }
    }
    out
}

pub fn identity_phrase(normalized: &str) -> String {
    let stripped = strip_camera_phrases(normalized);
    let collapsed = normalize_prompt(&stripped);
    if collapsed.is_empty() {
        normalized.to_string()
    } else {
        collapsed
    }
}

fn strip_camera_phrases(s: &str) -> String {
    let lower: String = s.to_lowercase();
    let mut drop = vec![false; s.len()];
    let mut phrases: Vec<&str> = CAMERA_PHRASES.to_vec();
    phrases.sort_by_key(|p| std::cmp::Reverse(p.len()));
    for p in phrases {
        let pl = p.to_lowercase();
        let mut search = 0;
        while let Some(rel) = lower[search..].find(&pl) {
            let start = search + rel;
            let end = start + pl.len();
            for b in drop.iter_mut().take(end).skip(start) {
                *b = true;
            }
            search = end;
        }
    }
    let mut out = String::new();
    for (i, c) in s.char_indices() {
        let nbytes = c.len_utf8();
        if drop.iter().take(i + nbytes).skip(i).any(|&d| d) {
            continue;
        }
        out.push(c);
    }
    out
}

pub fn preset_for_quality(q: Quality, override_preset: Option<CameraPreset>) -> CameraPreset {
    if let Some(p) = override_preset {
        return p;
    }
    match q {
        Quality::Preview => CameraPreset::Cardinal4,
        Quality::Standard => CameraPreset::Cardinal4HeroTop,
        Quality::High | Quality::Ultra => CameraPreset::Cardinal4HeroTopQuarters,
    }
}

struct CamSpec {
    id: &'static str,
    az: i32,
    el: i32,
    required: bool,
    role: &'static str,
    suffix: &'static str,
}

fn preset_cams(preset: CameraPreset) -> &'static [CamSpec] {
    const C4: &[CamSpec] = &[
        CamSpec {
            id: "front",
            az: 0,
            el: 15,
            required: true,
            role: "Tripo front",
            suffix: "front view, camera on +Z",
        },
        CamSpec {
            id: "right",
            az: 90,
            el: 15,
            required: true,
            role: "Tripo right",
            suffix: "right side view",
        },
        CamSpec {
            id: "back",
            az: 180,
            el: 15,
            required: true,
            role: "Janus",
            suffix: "back view, camera on -Z",
        },
        CamSpec {
            id: "left",
            az: 270,
            el: 15,
            required: true,
            role: "Tripo left",
            suffix: "left side view",
        },
    ];
    const C6: &[CamSpec] = &[
        CamSpec {
            id: "hero",
            az: 35,
            el: 22,
            required: true,
            role: "single-image 3D primary",
            suffix: "three-quarter view from the front-right",
        },
        CamSpec {
            id: "front",
            az: 0,
            el: 15,
            required: true,
            role: "front",
            suffix: "front view, camera on +Z",
        },
        CamSpec {
            id: "right",
            az: 90,
            el: 15,
            required: true,
            role: "right",
            suffix: "right side view",
        },
        CamSpec {
            id: "back",
            az: 180,
            el: 15,
            required: true,
            role: "Janus",
            suffix: "back view, camera on -Z",
        },
        CamSpec {
            id: "left",
            az: 270,
            el: 15,
            required: true,
            role: "left",
            suffix: "left side view",
        },
        CamSpec {
            id: "top",
            az: 0,
            el: 75,
            required: false,
            role: "droppable",
            suffix: "top-down view",
        },
    ];
    const C8: &[CamSpec] = &[
        CamSpec {
            id: "hero",
            az: 35,
            el: 22,
            required: true,
            role: "single-image 3D primary",
            suffix: "three-quarter view from the front-right",
        },
        CamSpec {
            id: "front",
            az: 0,
            el: 15,
            required: true,
            role: "front",
            suffix: "front view, camera on +Z",
        },
        CamSpec {
            id: "right",
            az: 90,
            el: 15,
            required: true,
            role: "right",
            suffix: "right side view",
        },
        CamSpec {
            id: "back",
            az: 180,
            el: 15,
            required: true,
            role: "Janus",
            suffix: "back view, camera on -Z",
        },
        CamSpec {
            id: "left",
            az: 270,
            el: 15,
            required: true,
            role: "left",
            suffix: "left side view",
        },
        CamSpec {
            id: "top",
            az: 0,
            el: 75,
            required: false,
            role: "droppable",
            suffix: "top-down view",
        },
        CamSpec {
            id: "qne",
            az: 45,
            el: 18,
            required: false,
            role: "optional quarter",
            suffix: "three-quarter view from the front-right, slightly higher",
        },
        CamSpec {
            id: "qnw",
            az: 315,
            el: 18,
            required: false,
            role: "optional quarter",
            suffix: "three-quarter view from the front-left, slightly higher",
        },
    ];
    match preset {
        CameraPreset::Cardinal4 => C4,
        CameraPreset::Cardinal4HeroTop => C6,
        CameraPreset::Cardinal4HeroTopQuarters => C8,
        CameraPreset::NativePassthrough => &[],
    }
}

struct ClassLocks {
    rig: &'static str,
    bg_mode: &'static str,
    light_lock: &'static str,
    bg_lock: &'static str,
    medium: &'static str,
    style_lock: &'static str,
    fov: f64,
    janus: bool,
}

fn locks(class: PromptClass) -> ClassLocks {
    match class {
        PromptClass::Creature | PromptClass::Character => ClassLocks {
            rig: "overcast",
            bg_mode: "neutral_gray",
            light_lock: "even overcast studio lighting, no hard shadows",
            bg_lock: "plain neutral gray background",
            medium: "photoreal",
            style_lock: "photoreal product photography, single subject",
            fov: 35.0,
            janus: true,
        },
        PromptClass::Architecture => ClassLocks {
            rig: "overcast",
            bg_mode: "neutral_gray",
            light_lock: "even overcast daylight",
            bg_lock: "plain neutral gray background",
            medium: "photoreal",
            style_lock: "architectural model, single building, no people",
            fov: 42.0,
            janus: false,
        },
        PromptClass::Vehicle => ClassLocks {
            rig: "studio_three_point",
            bg_mode: "neutral_gray",
            light_lock: "studio three-point lighting, soft key, fill, rim",
            bg_lock: "plain neutral gray background",
            medium: "photoreal",
            style_lock: "vehicle product shot, single vehicle, no riders",
            fov: 38.0,
            janus: false,
        },
        PromptClass::Product | PromptClass::Prop | PromptClass::Unknown | PromptClass::Analytic => {
            ClassLocks {
                rig: "studio_three_point",
                bg_mode: "neutral_gray",
                light_lock: "studio three-point lighting, soft key, fill, rim",
                bg_lock: "plain neutral gray background",
                medium: "photoreal",
                style_lock: "photoreal product shot, single object, catalog",
                fov: 35.0,
                janus: false,
            }
        }
    }
}

pub struct CompileOpts {
    pub quality: Quality,
    pub camera_preset: Option<CameraPreset>,
    pub family_seed: u64,
    pub t2i_provider: T2iProviderId,
}

impl Default for CompileOpts {
    fn default() -> Self {
        Self {
            quality: Quality::Standard,
            camera_preset: None,
            family_seed: 42,
            t2i_provider: T2iProviderId::Mock,
        }
    }
}

pub fn compile_view_contract(prompt: &str, opts: CompileOpts) -> Result<ViewContract, Error> {
    let raw = prompt.to_string();
    let n = prompt.trim().chars().count();
    if !(1..=4000).contains(&n) {
        return Err(Error::new(
            error_type::SPEC_REJECTED,
            "prompt must be 1..=4000 unicode chars after trim",
        ));
    }
    let normalized = normalize_prompt(prompt);
    let ident = identity_phrase(&normalized);
    let class = classify(&normalized);
    let preset = preset_for_quality(opts.quality, opts.camera_preset);
    let cams_spec = preset_cams(preset);
    let cameras: Vec<Camera> = cams_spec
        .iter()
        .map(|c| Camera {
            id: c.id.into(),
            role: c.role.into(),
            azimuth_deg: c.az,
            elevation_deg: c.el,
            roll_deg: 0,
            required: c.required,
            prompt_suffix: c.suffix.into(),
        })
        .collect();
    let canonical = if cameras.iter().any(|c| c.id == "hero") {
        "hero"
    } else {
        "front"
    };
    if preset == CameraPreset::Cardinal4 {
        debug_assert_ne!(canonical, "hero");
    }
    let lk = locks(class);
    let negatives: Vec<String> = if lk.janus {
        JANUS_NEGATIVES.iter().map(|s| (*s).into()).collect()
    } else {
        OTHER_NEGATIVES.iter().map(|s| (*s).into()).collect()
    };
    let id = ulid::Ulid::new().to_string();
    let now = crate::types::now_rfc3339();
    let notes = format!(
        "class={}; ring={}; lighting={}; canonical={}",
        class.as_str(),
        preset.as_str(),
        lk.rig,
        canonical
    );
    let tier = if opts.quality == Quality::Preview {
        "preview"
    } else {
        "quality"
    };
    Ok(ViewContract {
        schema: VIEW_CONTRACT_SCHEMA.into(),
        contract_id: id,
        created_at: now,
        prompt: ContractPrompt {
            hash: sha256_str(&normalized),
            raw,
            normalized,
            language: "en".into(),
        },
        subject_lock: SubjectLock {
            identity_phrase: ident,
            class,
            attributes: vec![],
            canonical_view_id: canonical.into(),
        },
        camera_ring: CameraRing {
            preset,
            count: cameras.len() as u32,
            convention: "y_up_azimuth_from_front".into(),
            distance: 1.6,
            fov_deg: lk.fov,
            cameras,
        },
        lighting: Lighting {
            rig: lk.rig.into(),
            locked: true,
            key_azimuth_deg: -30,
            fill_ratio: 0.4,
            white_balance: "D65".into(),
            prompt_lock: lk.light_lock.into(),
        },
        background: Background {
            mode: lk.bg_mode.into(),
            hex: "#B4B4B4".into(),
            alpha_preferred: true,
            prompt_lock: lk.bg_lock.into(),
        },
        style_lock: StyleLock {
            medium: lk.medium.into(),
            albedo_bias: false,
            prompt_lock: lk.style_lock.into(),
        },
        negatives,
        seed_policy: SeedPolicy {
            family_seed: opts.family_seed,
            hero_seed: opts.family_seed,
            orbit_seed_mode: "family_plus_view_index".into(),
        },
        frame: Frame {
            width: 1024,
            height: 1024,
            aspect: "1:1".into(),
        },
        t2i: T2iRef {
            provider: opts.t2i_provider,
            model: None,
            quality_tier: tier.into(),
        },
        compile_notes: notes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canonical::canonical_json;
    use crate::hash::sha256_str;

    #[test]
    fn fox_standard_six_hero_overcast_janus() {
        let c = compile_view_contract(
            "a red fox wearing a yellow raincoat",
            CompileOpts::default(),
        )
        .unwrap();
        assert_eq!(c.subject_lock.class, PromptClass::Creature);
        assert_eq!(c.subject_lock.canonical_view_id, "hero");
        assert_eq!(c.camera_ring.count, 6);
        assert_eq!(c.camera_ring.preset, CameraPreset::Cardinal4HeroTop);
        assert_eq!(c.lighting.rig, "overcast");
        assert_eq!(
            c.lighting.prompt_lock,
            "even overcast studio lighting, no hard shadows"
        );
        assert_eq!(c.negatives, JANUS_NEGATIVES);
        assert!(c.subject_lock.attributes.is_empty());
        assert_eq!(c.prompt.language, "en");
        let hero = c
            .camera_ring
            .cameras
            .iter()
            .find(|x| x.id == "hero")
            .unwrap();
        assert_eq!(
            hero.prompt_suffix,
            "three-quarter view from the front-right"
        );
    }

    #[test]
    fn preview_cardinal4_has_no_hero() {
        let c = compile_view_contract(
            "a ceramic coffee mug, product shot",
            CompileOpts {
                quality: Quality::Preview,
                ..CompileOpts::default()
            },
        )
        .unwrap();
        assert_eq!(c.subject_lock.canonical_view_id, "front");
        assert_eq!(c.camera_ring.count, 4);
        assert!(!c.camera_ring.cameras.iter().any(|x| x.id == "hero"));
        assert_eq!(c.t2i.quality_tier, "preview");
        assert_eq!(c.negatives, OTHER_NEGATIVES);
    }

    #[test]
    fn camera_words_stripped() {
        assert_eq!(
            identity_phrase("a dragon statue, front view"),
            "a dragon statue,"
        );
        assert_eq!(identity_phrase("a cat, back view"), "a cat,");
        assert_eq!(
            identity_phrase("a person, three-quarter view"),
            "a person, view"
        );
        assert_eq!(identity_phrase("front view"), "front view"); // empty remainder → original
    }

    #[test]
    fn high_is_eight() {
        let c = compile_view_contract(
            "a red sports car",
            CompileOpts {
                quality: Quality::High,
                ..CompileOpts::default()
            },
        )
        .unwrap();
        assert_eq!(c.camera_ring.count, 8);
        assert_eq!(c.lighting.rig, "studio_three_point");
        assert_eq!(c.camera_ring.fov_deg, 38.0);
    }

    #[test]
    fn goldens_identity_table() {
        let prompts: Vec<String> =
            serde_json::from_str(include_str!("../../../evals/text2/prompts.json")).unwrap();
        assert_eq!(prompts.len(), 24);
        let table: Vec<serde_json::Value> =
            serde_json::from_str(include_str!("../../../evals/text2/identity.json")).unwrap();
        assert_eq!(table.len(), prompts.len());
        for (p, row) in prompts.iter().zip(table.iter()) {
            assert_eq!(row["prompt"], *p);
            let ident = identity_phrase(&normalize_prompt(p));
            let class = classify(&normalize_prompt(p));
            assert_eq!(row["identity_phrase"], ident, "prompt={p}");
            assert_eq!(row["class"], class.as_str(), "prompt={p}");
        }
    }

    #[test]
    fn contract_hash_uses_canonical_not_pretty() {
        let c = compile_view_contract(
            "a red fox wearing a yellow raincoat",
            CompileOpts::default(),
        )
        .unwrap();
        let v = serde_json::to_value(&c).unwrap();
        let a = sha256_str(&canonical_json(&v));
        let pretty = serde_json::to_string_pretty(&c).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&pretty).unwrap();
        let b = sha256_str(&canonical_json(&parsed));
        assert_eq!(a, b);
    }
}
