//! Keyword classifier. Token lists are frozen; creature tokens match `evals/text2/species.txt`.

use crate::types::PromptClass;

const HUMANOID: &[&str] = &[
    "person",
    "human",
    "humanoid",
    "man",
    "woman",
    "child",
    "boy",
    "girl",
    "character",
    "android",
    "robot person",
    "portrait",
];

const SPECIES: &str = include_str!("../../../evals/text2/species.txt");

const PRODUCT: &[&str] = &[
    "product shot",
    "product photo",
    "consumer",
    "gadget",
    "bottle",
    "mug",
    "chair",
    "lamp",
    "shoe",
];

const VEHICLE: &[&str] = &[
    "car",
    "truck",
    "motorcycle",
    "bicycle",
    "van",
    "bus",
    "airplane",
    "plane",
    "boat",
    "ship",
    "locomotive",
    "scooter",
    "vehicle",
];

const ARCHITECTURE: &[&str] = &[
    "building",
    "house",
    "tower",
    "cathedral",
    "skyscraper",
    "temple",
    "castle",
    "architecture",
];

const ANALYTIC_WORDS: &[&str] = &[
    "fillet",
    "chamfer",
    "extrude",
    "bore",
    "through-hole",
    "through hole",
    "standoff",
    "flange",
    "iso 2768",
];

pub fn creature_tokens() -> Vec<String> {
    SPECIES
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|s| s.to_string())
        .collect()
}

fn has_token(hay_lower: &str, token: &str) -> bool {
    let t = token.to_lowercase();
    if t.contains(' ') || t.contains('-') {
        return hay_lower.contains(&t);
    }
    hay_lower
        .split(|c: char| !c.is_ascii_alphanumeric())
        .any(|w| w == t)
}

fn has_dimension(s: &str) -> bool {
    let lower = s.to_lowercase();
    let bytes = lower.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_digit() {
            let mut j = i;
            while j < bytes.len() && (bytes[j].is_ascii_digit() || bytes[j] == b'.') {
                j += 1;
            }
            let rest = lower[j..].trim_start();
            if rest.starts_with("mm")
                || rest.starts_with("cm")
                || rest.starts_with("inches")
                || rest.starts_with("inch")
                || rest.starts_with("in")
                || (rest.starts_with('m')
                    && rest
                        .chars()
                        .nth(1)
                        .map(|c| !c.is_ascii_alphanumeric())
                        .unwrap_or(true))
            {
                return true;
            }
            i = j.max(i + 1);
        } else {
            i += 1;
        }
    }
    false
}

fn has_metric_fastener(s: &str) -> bool {
    let lower = s.to_lowercase();
    lower.split(|c: char| !c.is_ascii_alphanumeric()).any(|w| {
        let b = w.as_bytes();
        b.len() >= 2
            && b[0] == b'm'
            && b[1].is_ascii_digit()
            && b[1] != b'0'
            && b[1] != b'1'
            && b[1..].iter().all(|c| c.is_ascii_digit())
    })
}

pub fn analytic_signals(prompt: &str) -> bool {
    let lower = prompt.to_lowercase();
    if has_dimension(&lower) {
        if ANALYTIC_WORDS.iter().any(|w| lower.contains(w)) {
            return true;
        }
        if lower.contains("bracket") {
            return true;
        }
        if lower.contains("box ") || lower.contains("cylinder") || lower.contains("tube ") {
            return true;
        }
        return true;
    }
    has_metric_fastener(&lower)
}

/// Subject class for the View Contract. Humanoid wins over creature. Analytic is last.
pub fn classify(prompt: &str) -> PromptClass {
    let lower = prompt.to_lowercase();
    if HUMANOID.iter().any(|t| has_token(&lower, t)) {
        return PromptClass::Character;
    }
    let species = creature_tokens();
    if species.iter().any(|t| has_token(&lower, t)) {
        return PromptClass::Creature;
    }
    if PRODUCT.iter().any(|t| has_token(&lower, t)) {
        return PromptClass::Product;
    }
    if VEHICLE.iter().any(|t| has_token(&lower, t)) {
        return PromptClass::Vehicle;
    }
    if ARCHITECTURE.iter().any(|t| has_token(&lower, t)) {
        return PromptClass::Architecture;
    }
    if analytic_signals(prompt) {
        return PromptClass::Analytic;
    }
    PromptClass::Unknown
}

pub fn both_analytic_and_visual(prompt: &str) -> bool {
    analytic_signals(prompt)
        && matches!(
            classify(prompt),
            PromptClass::Creature | PromptClass::Character
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn species_file_matches_inline_design_list() {
        let got = creature_tokens();
        let expect = [
            "creature", "monster", "animal", "beast", "dragon", "fox", "cat", "dog", "bird",
            "wolf", "bear", "horse", "fish", "snake", "wearing",
        ];
        assert_eq!(got, expect);
    }

    #[test]
    fn fox_raincoat_is_creature() {
        assert_eq!(
            classify("a red fox wearing a yellow raincoat"),
            PromptClass::Creature
        );
    }

    #[test]
    fn portrait_is_character() {
        assert_eq!(
            classify("portrait of a woman in a blue coat"),
            PromptClass::Character
        );
    }

    #[test]
    fn mug_product_shot() {
        assert_eq!(
            classify("a ceramic coffee mug, product shot"),
            PromptClass::Product
        );
    }

    #[test]
    fn box_mm_is_analytic() {
        assert_eq!(classify("box 20x10x5 mm"), PromptClass::Analytic);
    }

    #[test]
    fn fox_with_mm_stays_visual_class() {
        // humanoid/creature wins; router (S7) still picks ViewContract
        assert_eq!(classify("a fox 20 mm tall"), PromptClass::Creature);
        assert!(both_analytic_and_visual("a fox 20 mm tall"));
    }
}
