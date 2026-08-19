//! T2I/I2I provider surface (design §19). No keys we do not hold.

use crate::contract::ViewContract;
use crate::error::{error_type, Error};
use crate::orbit::mock_view_png;
use crate::types::T2iProviderId;

#[derive(Debug, Clone)]
pub struct T2iImage {
    pub bytes: Vec<u8>,
    pub job_id: Option<String>,
    pub usd: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct T2iCost {
    pub usd: f64,
    pub usd_uncertain: bool,
    pub model: Option<String>,
}

pub trait T2iProvider: Send + Sync {
    fn id(&self) -> T2iProviderId;
    fn estimate(&self, n_t2i: u32, n_i2i: u32, model: Option<&str>) -> Result<T2iCost, Error>;
    fn generate(&self, prompt: &str) -> Result<T2iImage, Error>;
    fn edit(
        &self,
        prompt: &str,
        hero_job_id: Option<&str>,
        extra_png: &[&[u8]],
    ) -> Result<T2iImage, Error>;
}

/// In-process gray-studio PNGs. $0. Used by `local.mock` and ALLOW_MOCK.
pub struct MockT2i;

impl T2iProvider for MockT2i {
    fn id(&self) -> T2iProviderId {
        T2iProviderId::Mock
    }

    fn estimate(&self, _n_t2i: u32, _n_i2i: u32, _model: Option<&str>) -> Result<T2iCost, Error> {
        Ok(T2iCost {
            usd: 0.0,
            usd_uncertain: false,
            model: Some("mock".into()),
        })
    }

    fn generate(&self, _prompt: &str) -> Result<T2iImage, Error> {
        Err(Error::new(
            error_type::INTERNAL,
            "MockT2i is driven via synthesize_orbit, not generate()",
        ))
    }

    fn edit(
        &self,
        _prompt: &str,
        _hero_job_id: Option<&str>,
        _extra_png: &[&[u8]],
    ) -> Result<T2iImage, Error> {
        Err(Error::new(
            error_type::INTERNAL,
            "MockT2i is driven via synthesize_orbit, not edit()",
        ))
    }
}

#[derive(Debug, Clone)]
pub struct OrbitImages {
    pub views: Vec<(String, Vec<u8>)>,
    pub usd: f64,
    pub usd_uncertain: bool,
    pub child_ids: Vec<String>,
    pub provider: T2iProviderId,
    pub independent_t2i: bool,
}

pub fn view_count(quality: crate::types::Quality) -> u32 {
    match quality {
        crate::types::Quality::Preview => 4,
        crate::types::Quality::Standard => 6,
        crate::types::Quality::High | crate::types::Quality::Ultra => 8,
    }
}

/// Hero then I2I-orbit. Mock provider never POSTs.
pub fn synthesize_orbit(
    provider: &dyn T2iProvider,
    contract: &ViewContract,
) -> Result<OrbitImages, Error> {
    if provider.id() == T2iProviderId::Mock {
        return mock_orbit(contract);
    }

    let cams = &contract.camera_ring.cameras;
    let canonical = cams
        .iter()
        .find(|c| c.id == contract.subject_lock.canonical_view_id)
        .or_else(|| cams.first())
        .ok_or_else(|| Error::new(error_type::INTERNAL, "view contract has no cameras"))?;

    let hero_prompt = contract.assembled_prompt(canonical, false);
    let hero = provider.generate(&hero_prompt)?;
    let mut usd = hero.usd.unwrap_or(0.0);
    let mut child_ids = Vec::new();
    if let Some(id) = &hero.job_id {
        child_ids.push(id.clone());
    }

    let mut views = vec![(canonical.id.clone(), hero.bytes.clone())];
    let mut independent = false;
    let hero_id = hero.job_id.clone();

    for cam in cams.iter().filter(|c| c.id != canonical.id) {
        let prompt = contract.assembled_prompt(cam, true);
        let img = match provider.edit(&prompt, hero_id.as_deref(), &[&hero.bytes]) {
            Ok(img) => img,
            Err(e) if e.error_type == error_type::T2I_UNAVAILABLE => {
                independent = true;
                provider.generate(&contract.assembled_prompt(cam, false))?
            }
            Err(e) => return Err(e),
        };
        usd += img.usd.unwrap_or(0.0);
        if let Some(id) = &img.job_id {
            child_ids.push(id.clone());
        }
        views.push((cam.id.clone(), img.bytes));
    }

    Ok(OrbitImages {
        views,
        usd,
        usd_uncertain: false,
        child_ids,
        provider: provider.id(),
        independent_t2i: independent,
    })
}

fn mock_orbit(contract: &ViewContract) -> Result<OrbitImages, Error> {
    let mut views = Vec::new();
    for cam in &contract.camera_ring.cameras {
        views.push((cam.id.clone(), mock_view_png(contract, &cam.id)?));
    }
    Ok(OrbitImages {
        views,
        usd: 0.0,
        usd_uncertain: false,
        child_ids: Vec::new(),
        provider: T2iProviderId::Mock,
        independent_t2i: false,
    })
}

/// Reserved retries billed at half an I2I unit (design §14 / OQ-9).
pub const MAX_ORBIT_EDITS: u32 = 3;

pub fn estimate_orbit(provider: &dyn T2iProvider, n_views: u32) -> Result<T2iCost, Error> {
    let n_orbit = n_views.saturating_sub(1);
    let hero = provider.estimate(1, 0, Some("2.0"))?;
    if n_orbit == 0 {
        return Ok(hero);
    }
    let orbit = provider.estimate(0, n_orbit, Some("2.0"))?;
    let unit = orbit.usd / f64::from(n_orbit);
    let reserved = f64::from(MAX_ORBIT_EDITS) * unit * 0.5;
    Ok(T2iCost {
        usd: hero.usd + orbit.usd + reserved,
        usd_uncertain: hero.usd_uncertain || orbit.usd_uncertain,
        model: orbit.model.or(hero.model),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::{compile_view_contract, CompileOpts};

    #[test]
    fn mock_orbit_emits_one_png_per_camera() {
        let c = compile_view_contract("a red fox", CompileOpts::default()).unwrap();
        let o = synthesize_orbit(&MockT2i, &c).unwrap();
        assert_eq!(o.views.len(), c.camera_ring.cameras.len());
        assert_eq!(o.usd, 0.0);
        assert_eq!(o.provider, T2iProviderId::Mock);
    }
}
