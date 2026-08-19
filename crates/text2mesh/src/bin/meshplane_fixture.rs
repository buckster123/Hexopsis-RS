//! Fixture `meshplane/1` child. Writes the in-process mock GLB. Not a quality engine.
//!
//! Modes (argv[1] or `MESHPLANE_FIXTURE_MODE`): default | crash | hang | mute | bad-protocol | escape

use std::io::{self, BufRead, Write};
use std::time::Duration;

use serde_json::{json, Value};
use text2mesh::mock_glb::emit_mock_glb_seeded;

fn handshake(protocol: &str) {
    let hs = json!({
        "protocol": protocol,
        "engine": "fixture",
        "version": "0.1.0",
        "caps": { "image_to_mesh": true, "pbr": false, "tiers": ["preview"] },
        "licenses": ["MIT"],
        "devices": ["cpu"]
    });
    let mut out = io::stdout();
    writeln!(out, "{hs}").unwrap();
    out.flush().unwrap();
}

fn mode() -> String {
    std::env::args()
        .nth(1)
        .or_else(|| std::env::var("MESHPLANE_FIXTURE_MODE").ok())
        .unwrap_or_default()
}

fn main() {
    let mode = mode();
    match mode.as_str() {
        "mute" => loop {
            std::thread::sleep(Duration::from_secs(60));
        },
        "bad-protocol" => {
            handshake("nope/0");
            std::process::exit(2);
        }
        _ => handshake("meshplane/1"),
    }

    if mode == "hang" {
        loop {
            std::thread::sleep(Duration::from_secs(60));
        }
    }
    if mode == "crash" {
        std::process::exit(1);
    }

    let stdin = io::stdin();
    let line = stdin.lock().lines().next().and_then(|r| r.ok());
    let Some(line) = line else {
        std::process::exit(2)
    };
    let v: Value = serde_json::from_str(&line).unwrap_or(Value::Null);
    if v.get("op").and_then(|o| o.as_str()) != Some("submit") {
        let fail = json!({"op":"fail","error_type":"engine.crash","message":"expected submit"});
        writeln!(io::stdout(), "{fail}").ok();
        std::process::exit(2);
    }
    let paths = &v["paths"];
    let seed = v["job"]["seed"].as_u64().unwrap_or(0);
    let glb = emit_mock_glb_seeded(b"sidecar-fixture", seed);

    let out_glb = if mode == "escape" {
        "/tmp/text2mesh-sidecar-escape.glb".to_string()
    } else {
        paths["out_glb"]
            .as_str()
            .unwrap_or("artifact.glb")
            .to_string()
    };
    if let Some(parent) = std::path::Path::new(&out_glb).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(&out_glb, &glb).unwrap();
    let art = json!({"op":"artifact","kind":"glb","path": out_glb});
    let mut out = io::stdout();
    writeln!(out, "{art}").unwrap();
    out.flush().unwrap();
}
