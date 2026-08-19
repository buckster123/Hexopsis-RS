//! Deterministic glTF 2.0 BINARY tetrahedron with POSITION + COLOR_0 (never PBR).

use serde_json::{json, Value};

const GLB_MAGIC: u32 = 0x4654_6C67;
const GLB_VERSION: u32 = 2;
const CHUNK_JSON: u32 = 0x4E4F_534A;
const CHUNK_BIN: u32 = 0x004E_4942;

/// Default mock artefact (empty input, seed 0). Hash is stable across runs.
pub fn emit_mock_glb() -> Vec<u8> {
    emit_mock_glb_seeded(&[], 0)
}

/// Contents are a deterministic function of `sha256(input || seed_le)`. Geometry is fixed.
pub fn emit_mock_glb_seeded(input: &[u8], seed: u64) -> Vec<u8> {
    let mut key = input.to_vec();
    key.extend_from_slice(&seed.to_le_bytes());
    let digest = crate::hash::sha256_bytes(&key);
    let hue = digest.as_bytes()[7]; // first hex after prefix; variation without greying

    let positions: [[f32; 3]; 4] = [
        [1.0, 1.0, 1.0],
        [1.0, -1.0, -1.0],
        [-1.0, 1.0, -1.0],
        [-1.0, -1.0, 1.0],
    ];
    // Distinct RGB so COLOR_0 has variation (grey-only would be export.materials_missing).
    let mut colors: [[f32; 3]; 4] = [
        [1.0, 0.12, 0.12],
        [0.12, 1.0, 0.12],
        [0.12, 0.12, 1.0],
        [1.0, 1.0, 0.12],
    ];
    let jitter = (hue as f32) / 255.0 * 0.08;
    for c in &mut colors {
        c[0] = (c[0] - jitter).clamp(0.05, 1.0);
    }

    let indices: [u16; 12] = [0, 1, 2, 0, 2, 3, 0, 3, 1, 1, 3, 2];

    let mut bin = Vec::with_capacity(120);
    for p in &positions {
        for x in p {
            bin.extend_from_slice(&x.to_le_bytes());
        }
    }
    for c in &colors {
        for x in c {
            bin.extend_from_slice(&x.to_le_bytes());
        }
    }
    for i in &indices {
        bin.extend_from_slice(&i.to_le_bytes());
    }
    while !bin.len().is_multiple_of(4) {
        bin.push(0);
    }

    let json = json!({
        "asset": { "version": "2.0", "generator": "text2mesh-mock" },
        "scene": 0,
        "scenes": [{ "nodes": [0] }],
        "nodes": [{ "mesh": 0 }],
        "meshes": [{
            "primitives": [{
                "attributes": { "POSITION": 0, "COLOR_0": 1 },
                "indices": 2,
                "mode": 4
            }]
        }],
        "accessors": [
            {
                "bufferView": 0,
                "componentType": 5126,
                "count": 4,
                "type": "VEC3",
                "min": [-1.0, -1.0, -1.0],
                "max": [1.0, 1.0, 1.0]
            },
            {
                "bufferView": 1,
                "componentType": 5126,
                "count": 4,
                "type": "VEC3"
            },
            {
                "bufferView": 2,
                "componentType": 5123,
                "count": 12,
                "type": "SCALAR"
            }
        ],
        "bufferViews": [
            { "buffer": 0, "byteOffset": 0, "byteLength": 48, "target": 34962 },
            { "buffer": 0, "byteOffset": 48, "byteLength": 48, "target": 34962 },
            { "buffer": 0, "byteOffset": 96, "byteLength": 24, "target": 34963 }
        ],
        "buffers": [{ "byteLength": bin.len() }]
    });
    let mut json_bytes = serde_json::to_vec(&json).unwrap_or_else(|_| b"{}".to_vec());
    while !json_bytes.len().is_multiple_of(4) {
        json_bytes.push(b' ');
    }

    let total = 12 + 8 + json_bytes.len() + 8 + bin.len();
    let mut out = Vec::with_capacity(total);
    write_u32(&mut out, GLB_MAGIC);
    write_u32(&mut out, GLB_VERSION);
    write_u32(&mut out, total as u32);
    write_u32(&mut out, json_bytes.len() as u32);
    write_u32(&mut out, CHUNK_JSON);
    out.extend_from_slice(&json_bytes);
    write_u32(&mut out, bin.len() as u32);
    write_u32(&mut out, CHUNK_BIN);
    out.extend_from_slice(&bin);
    out
}

/// Parser-valid GLB with default metallic-roughness only — no COLOR_0, no textures.
/// Must never be `succeeded` (D9 `export.materials_missing`).
pub fn emit_grey_glb() -> Vec<u8> {
    let positions: [[f32; 3]; 4] = [
        [1.0, 1.0, 1.0],
        [1.0, -1.0, -1.0],
        [-1.0, 1.0, -1.0],
        [-1.0, -1.0, 1.0],
    ];
    let indices: [u16; 12] = [0, 1, 2, 0, 2, 3, 0, 3, 1, 1, 3, 2];
    let mut bin = Vec::new();
    for p in &positions {
        for x in p {
            bin.extend_from_slice(&x.to_le_bytes());
        }
    }
    for i in &indices {
        bin.extend_from_slice(&i.to_le_bytes());
    }
    while !bin.len().is_multiple_of(4) {
        bin.push(0);
    }
    let json = json!({
        "asset": { "version": "2.0", "generator": "text2mesh-grey" },
        "scene": 0,
        "scenes": [{ "nodes": [0] }],
        "nodes": [{ "mesh": 0 }],
        "meshes": [{
            "primitives": [{
                "attributes": { "POSITION": 0 },
                "indices": 1,
                "mode": 4
            }]
        }],
        "accessors": [
            {
                "bufferView": 0,
                "componentType": 5126,
                "count": 4,
                "type": "VEC3",
                "min": [-1.0, -1.0, -1.0],
                "max": [1.0, 1.0, 1.0]
            },
            {
                "bufferView": 1,
                "componentType": 5123,
                "count": 12,
                "type": "SCALAR"
            }
        ],
        "bufferViews": [
            { "buffer": 0, "byteOffset": 0, "byteLength": 48, "target": 34962 },
            { "buffer": 0, "byteOffset": 48, "byteLength": 24, "target": 34963 }
        ],
        "buffers": [{ "byteLength": bin.len() }]
    });
    let mut json_bytes = serde_json::to_vec(&json).unwrap_or_else(|_| b"{}".to_vec());
    while !json_bytes.len().is_multiple_of(4) {
        json_bytes.push(b' ');
    }
    let total = 12 + 8 + json_bytes.len() + 8 + bin.len();
    let mut out = Vec::with_capacity(total);
    write_u32(&mut out, GLB_MAGIC);
    write_u32(&mut out, GLB_VERSION);
    write_u32(&mut out, total as u32);
    write_u32(&mut out, json_bytes.len() as u32);
    write_u32(&mut out, CHUNK_JSON);
    out.extend_from_slice(&json_bytes);
    write_u32(&mut out, bin.len() as u32);
    write_u32(&mut out, CHUNK_BIN);
    out.extend_from_slice(&bin);
    out
}

fn write_u32(buf: &mut Vec<u8>, v: u32) {
    buf.extend_from_slice(&v.to_le_bytes());
}

pub fn glb_json_chunk(bytes: &[u8]) -> Result<Value, String> {
    if bytes.len() < 20 {
        return Err("glb too short".into());
    }
    let magic = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
    if magic != GLB_MAGIC {
        return Err("bad magic".into());
    }
    let version = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
    if version != 2 {
        return Err("not glTF 2".into());
    }
    let json_len = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
    let json_ty = u32::from_le_bytes(bytes[16..20].try_into().unwrap());
    if json_ty != CHUNK_JSON {
        return Err("first chunk is not JSON".into());
    }
    let start: usize = 20;
    let end = start.checked_add(json_len).ok_or("overflow")?;
    if end > bytes.len() {
        return Err("json chunk truncated".into());
    }
    serde_json::from_slice(&bytes[start..end]).map_err(|e| e.to_string())
}

pub fn has_vertex_color(bytes: &[u8]) -> bool {
    match glb_json_chunk(bytes) {
        Ok(v) => v
            .pointer("/meshes/0/primitives/0/attributes/COLOR_0")
            .is_some(),
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hash::sha256_bytes;

    #[test]
    fn mock_glb_has_vertex_color() {
        let glb = emit_mock_glb();
        assert!(has_vertex_color(&glb), "COLOR_0 missing");
        let json = glb_json_chunk(&glb).unwrap();
        assert!(
            json.pointer("/images").is_none()
                || json["images"].as_array().is_none_or(|a| a.is_empty())
        );
        assert_eq!(json["asset"]["version"], "2.0");
    }

    #[test]
    fn mock_glb_hash_stable() {
        let a = emit_mock_glb();
        let b = emit_mock_glb();
        assert_eq!(a, b);
        assert_eq!(sha256_bytes(&a), sha256_bytes(&b));
        assert!(sha256_bytes(&a).starts_with("sha256:"));
    }

    #[test]
    fn grey_default_material_not_succeeded() {
        assert!(has_vertex_color(&emit_mock_glb()));
        assert!(!has_vertex_color(&emit_grey_glb()));
        let json = glb_json_chunk(&emit_grey_glb()).unwrap();
        assert_eq!(json["asset"]["version"], "2.0");
        assert!(json
            .pointer("/meshes/0/primitives/0/attributes/COLOR_0")
            .is_none());
    }
}
