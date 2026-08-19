//! Tripo public HTTP (developers.tripo3d.ai). Inert without `TRIPO_API_KEY`.

use std::time::Instant;

use serde_json::json;

use crate::error::{error_type, Error};
use crate::remote::{
    data_uri_png, tripo_poll_state, tripo_task_id, RemoteArtifact, RemoteOutcome, VendorHttp,
    TRIPO_DEFAULT,
};

pub struct Tripo {
    http: VendorHttp,
}

impl Tripo {
    pub fn from_env() -> Result<Option<Self>, Error> {
        let Ok(key) = crate::remote::require_key("TRIPO_API_KEY") else {
            return Ok(None);
        };
        let base = std::env::var("TEXT2MESH_TRIPO_URL")
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| TRIPO_DEFAULT.into());
        Ok(Some(Self::new(base, key)?))
    }

    pub fn new(base: String, key: String) -> Result<Self, Error> {
        Ok(Self {
            http: VendorHttp::new(base, key)?,
        })
    }

    pub fn for_test(base: String, key: String) -> Result<Self, Error> {
        Ok(Self {
            http: VendorHttp::new(base, key)?.fast_poll(),
        })
    }

    pub fn run_image(&self, png: &[u8]) -> Result<RemoteOutcome, Error> {
        let body = json!({
            "model": "v3.1-20260211",
            "file": {
                "type": "data_url",
                "url": data_uri_png(png),
            }
        });
        self.submit_and_poll("/generation/image-to-model", body)
    }

    pub fn run_text(&self, prompt: &str) -> Result<RemoteOutcome, Error> {
        let body = json!({
            "prompt": prompt,
            "model": "v3.1-20260211",
        });
        self.submit_and_poll("/generation/text-to-model", body)
    }

    pub fn run_multiview(&self, views: &[(&str, &[u8])]) -> Result<RemoteOutcome, Error> {
        // Named cardinals only; hero/top ignored (design §7 catalog).
        let mut files = serde_json::Map::new();
        for (name, bytes) in views {
            if matches!(*name, "front" | "left" | "back" | "right") {
                files.insert(
                    (*name).into(),
                    json!({ "type": "data_url", "url": data_uri_png(bytes) }),
                );
            }
        }
        if !files.contains_key("front") {
            return Err(Error::new(
                error_type::SPEC_REJECTED,
                "tripo multiview requires a front view",
            ));
        }
        let body = json!({
            "model": "v3.1-20260211",
            "files": files,
        });
        self.submit_and_poll("/generation/multiview-to-model", body)
    }

    fn submit_and_poll(&self, path: &str, body: serde_json::Value) -> Result<RemoteOutcome, Error> {
        let created = self.http.post_json(path, &body)?;
        let id = tripo_task_id(&created)?;
        let start = Instant::now();
        loop {
            if start.elapsed() > self.http.wall {
                return Ok(RemoteOutcome::Waiting { upstream_id: id });
            }
            let task = self.http.get_json(&format!("/tasks/{id}"))?;
            let (status, glb) = tripo_poll_state(&task);
            match status {
                "success" => {
                    let url = glb.ok_or_else(|| {
                        Error::new(error_type::UNSUPPORTED, "tripo success without model_url")
                    })?;
                    let bytes = self.http.get_bytes(url)?;
                    return Ok(RemoteOutcome::Done(RemoteArtifact {
                        glb: bytes,
                        engine: "tripo".into(),
                        upstream_id: id,
                        usd: None,
                    }));
                }
                "failed" | "cancelled" | "banned" => {
                    return Err(Error::new(error_type::UNSUPPORTED, status));
                }
                _ => std::thread::sleep(self.http.poll),
            }
        }
    }
}
