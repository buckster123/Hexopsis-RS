//! Meshy public HTTP (docs.meshy.ai). Inert without `MESHY_API_KEY`.

use std::time::Instant;

use serde_json::json;

use crate::error::{error_type, Error};
use crate::remote::{
    data_uri_png, meshy_poll_state, meshy_task_id, RemoteArtifact, RemoteOutcome, VendorHttp,
    MESHY_DEFAULT,
};

pub struct Meshy {
    http: VendorHttp,
}

impl Meshy {
    pub fn from_env() -> Result<Option<Self>, Error> {
        let Ok(key) = crate::remote::require_key("MESHY_API_KEY") else {
            return Ok(None);
        };
        let base = std::env::var("TEXT2MESH_MESHY_URL")
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| MESHY_DEFAULT.into());
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
            "image_url": data_uri_png(png),
            "target_formats": ["glb"],
            "should_texture": true,
            "enable_pbr": true,
        });
        self.submit_and_poll("/openapi/v1/image-to-3d", body)
    }

    pub fn run_text(&self, prompt: &str) -> Result<RemoteOutcome, Error> {
        let body = json!({
            "mode": "preview",
            "prompt": prompt,
            "target_formats": ["glb"],
        });
        self.submit_and_poll("/openapi/v2/text-to-3d", body)
    }

    fn submit_and_poll(&self, path: &str, body: serde_json::Value) -> Result<RemoteOutcome, Error> {
        let created = self.http.post_json(path, &body)?;
        let id = meshy_task_id(&created)?;
        let start = Instant::now();
        loop {
            if start.elapsed() > self.http.wall {
                return Ok(RemoteOutcome::Waiting { upstream_id: id });
            }
            let task = self.http.get_json(&format!("{path}/{id}"))?;
            let (status, glb, credits) = meshy_poll_state(&task);
            match status {
                "SUCCEEDED" => {
                    let url = glb.ok_or_else(|| {
                        Error::new(error_type::UNSUPPORTED, "meshy succeeded without glb url")
                    })?;
                    let bytes = self.http.get_bytes(url)?;
                    let _ = credits;
                    return Ok(RemoteOutcome::Done(RemoteArtifact {
                        glb: bytes,
                        engine: "meshy".into(),
                        upstream_id: id,
                        usd: None,
                    }));
                }
                "FAILED" | "CANCELED" => {
                    let msg = task
                        .pointer("/task_error/message")
                        .and_then(|x| x.as_str())
                        .unwrap_or(status);
                    return Err(Error::new(error_type::UNSUPPORTED, msg));
                }
                _ => std::thread::sleep(self.http.poll),
            }
        }
    }
}
