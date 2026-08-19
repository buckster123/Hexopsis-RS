//! glTF 2.0 export honesty (design §21). Parser-valid + materials as claimed.

use gltf::Semantic;

use crate::error::{error_type, Error};
use crate::types::MaterialMode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportClass {
    /// UV / textures present — the only path that may `succeeded`.
    UvAtlas,
    VertexColor,
    FactorsOnly,
    Missing,
}

#[derive(Debug, Clone)]
pub struct ExportReport {
    pub class: ExportClass,
    pub material_mode: Option<MaterialMode>,
    pub parser_ok: bool,
}

impl ExportClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::UvAtlas => "uv_atlas",
            Self::VertexColor => "vertex_color",
            Self::FactorsOnly => "factors_only",
            Self::Missing => "missing",
        }
    }
}

/// Parse with the `gltf` crate, then classify materials. Unparseable → `engine.crash`.
pub fn inspect_glb(bytes: &[u8]) -> Result<ExportReport, Error> {
    let g = gltf::Gltf::from_slice(bytes).map_err(|e| {
        Error::new(error_type::ENGINE_CRASH, format!("gltf parse: {e}"))
            .with_hint("engine did not write a glTF 2.0 GLB")
    })?;

    let mut has_color0 = false;
    let mut has_tex = false;
    let mut nondefault_factors = false;
    let mut any_prim = false;

    for mesh in g.meshes() {
        for prim in mesh.primitives() {
            any_prim = true;
            if prim.get(&Semantic::Colors(0)).is_some() {
                has_color0 = true;
            }
            let mat = prim.material();
            let pbr = mat.pbr_metallic_roughness();
            if pbr.base_color_texture().is_some() || pbr.metallic_roughness_texture().is_some() {
                has_tex = true;
            }
            let f = pbr.base_color_factor();
            if (f[0] - 1.0).abs() > 1e-4
                || (f[1] - 1.0).abs() > 1e-4
                || (f[2] - 1.0).abs() > 1e-4
                || (f[3] - 1.0).abs() > 1e-4
            {
                nondefault_factors = true;
            }
            if (pbr.metallic_factor() - 1.0).abs() > 1e-4 {
                nondefault_factors = true;
            }
            if (pbr.roughness_factor() - 1.0).abs() > 1e-4 {
                nondefault_factors = true;
            }
        }
    }
    if g.textures().len() > 0 || g.images().len() > 0 {
        has_tex = true;
    }

    if !any_prim {
        return Ok(ExportReport {
            class: ExportClass::Missing,
            material_mode: None,
            parser_ok: true,
        });
    }

    let class = if has_tex {
        ExportClass::UvAtlas
    } else if has_color0 {
        ExportClass::VertexColor
    } else if nondefault_factors {
        ExportClass::FactorsOnly
    } else {
        ExportClass::Missing
    };
    let material_mode = match class {
        ExportClass::UvAtlas => Some(MaterialMode::UvAtlas),
        ExportClass::VertexColor => Some(MaterialMode::VertexColor),
        ExportClass::FactorsOnly => Some(MaterialMode::FactorsOnly),
        ExportClass::Missing => None,
    };
    Ok(ExportReport {
        class,
        material_mode,
        parser_ok: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock_glb::{emit_grey_glb, emit_mock_glb};

    #[test]
    fn mock_is_vertex_color_not_pbr() {
        let r = inspect_glb(&emit_mock_glb()).unwrap();
        assert!(r.parser_ok);
        assert_eq!(r.class, ExportClass::VertexColor);
        assert_eq!(r.material_mode, Some(MaterialMode::VertexColor));
    }

    #[test]
    fn grey_default_is_missing() {
        let r = inspect_glb(&emit_grey_glb()).unwrap();
        assert_eq!(r.class, ExportClass::Missing);
    }

    #[test]
    fn garbage_is_engine_crash() {
        let err = inspect_glb(b"not a glb").unwrap_err();
        assert_eq!(err.error_type, error_type::ENGINE_CRASH);
    }
}
