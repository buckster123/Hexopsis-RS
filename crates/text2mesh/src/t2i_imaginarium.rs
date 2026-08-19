//! Imaginarium HTTP client. Never reads `XAI_API_KEY`.

use std::time::Duration;

use serde_json::{json, Value};

use crate::error::{error_type, Error};
use crate::t2i::{T2iCost, T2iImage, T2iProvider};
use crate::types::T2iProviderId;

const PROBE_TIMEOUT: Duration = Duration::from_secs(5);
const JOB_TIMEOUT: Duration = Duration::from_secs(120);

pub struct Imaginarium {
    pub base: String,
    pub token: Option<String>,
    client: reqwest::blocking::Client,
}

impl Imaginarium {
    pub fn from_env() -> Option<Self> {
        let base = std::env::var("TEXT2MESH_IMAGINARIUM_URL")
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "http://127.0.0.1:8791".into());
        let token = std::env::var("TEXT2MESH_IMAGINARIUM_TOKEN")
            .ok()
            .filter(|s| !s.is_empty());
        Self::new(base, token).ok()
    }

    pub fn new(base: String, token: Option<String>) -> Result<Self, Error> {
        let client = reqwest::blocking::Client::builder()
            .timeout(JOB_TIMEOUT)
            .build()
            .map_err(|e| Error::new(error_type::INTERNAL, e.to_string()))?;
        Ok(Self {
            base: base.trim_end_matches('/').into(),
            token,
            client,
        })
    }

    /// Probe budget 5 s. Sibling public route is `GET /health` (design §19.1).
    pub fn health(&self) -> bool {
        for path in ["/health", "/v1/health"] {
            let ok = self
                .client
                .get(format!("{}{path}", self.base))
                .timeout(PROBE_TIMEOUT)
                .send()
                .map(|r| r.status().is_success())
                .unwrap_or(false);
            if ok {
                return true;
            }
        }
        false
    }

    fn auth(&self, req: reqwest::blocking::RequestBuilder) -> reqwest::blocking::RequestBuilder {
        if let Some(t) = &self.token {
            req.header("Authorization", format!("Bearer {t}"))
                .header("X-Imaginarium-Token", t)
        } else {
            req
        }
    }

    fn map_status(status: reqwest::StatusCode, body: &str) -> Error {
        match status.as_u16() {
            402 => Error::new(error_type::SPEND_PROVIDER_402, body),
            429 => Error::new(error_type::RATE_LIMIT, body),
            401 | 403 => Error::new(error_type::NOT_CONFIGURED, "imaginarium token rejected"),
            _ => Error::new(
                error_type::T2I_UNAVAILABLE,
                format!("imaginarium {status}: {body}"),
            ),
        }
    }

    fn post_json(&self, path: &str, body: &Value) -> Result<Value, Error> {
        let resp = self
            .auth(self.client.post(format!("{}{path}", self.base)))
            .json(body)
            .send()
            .map_err(|e| Error::new(error_type::T2I_UNAVAILABLE, e.to_string()))?;
        let status = resp.status();
        let text = resp.text().unwrap_or_default();
        if !status.is_success() {
            return Err(Self::map_status(status, &text));
        }
        serde_json::from_str(&text)
            .map_err(|e| Error::new(error_type::T2I_UNAVAILABLE, e.to_string()))
    }

    fn download_asset(&self, v: &Value) -> Result<T2iImage, Error> {
        if !v.get("ok").and_then(|x| x.as_bool()).unwrap_or(true) {
            let et = v
                .get("error_type")
                .and_then(|x| x.as_str())
                .unwrap_or(error_type::T2I_UNAVAILABLE);
            let msg = v
                .get("error")
                .and_then(|x| x.as_str())
                .unwrap_or("imaginarium job not ok");
            return Err(Error::new(
                if et == "spend_limit" {
                    error_type::SPEND_ESTIMATE_EXCEEDED
                } else {
                    error_type::T2I_UNAVAILABLE
                },
                msg,
            ));
        }
        let job_id = v.get("job_id").and_then(|x| x.as_str()).map(str::to_string);
        let usd = v.pointer("/usage/estimated_usd").and_then(|x| x.as_f64());
        let asset = v
            .get("assets")
            .and_then(|a| a.as_array())
            .and_then(|a| a.first())
            .ok_or_else(|| {
                Error::new(
                    error_type::T2I_UNAVAILABLE,
                    "no assets in imaginarium result",
                )
            })?;
        if let Some(url) = asset.get("content_url").and_then(|x| x.as_str()) {
            let abs = if url.starts_with("http") {
                url.to_string()
            } else {
                format!("{}{url}", self.base)
            };
            let resp = self
                .auth(self.client.get(abs))
                .send()
                .map_err(|e| Error::new(error_type::T2I_UNAVAILABLE, e.to_string()))?;
            if !resp.status().is_success() {
                return Err(Self::map_status(
                    resp.status(),
                    &resp.text().unwrap_or_default(),
                ));
            }
            let bytes = resp
                .bytes()
                .map_err(|e| Error::new(error_type::T2I_UNAVAILABLE, e.to_string()))?
                .to_vec();
            return Ok(T2iImage { bytes, job_id, usd });
        }
        if let Some(url) = asset.get("upstream_url").and_then(|x| x.as_str()) {
            let resp = self
                .client
                .get(url)
                .send()
                .map_err(|e| Error::new(error_type::T2I_UNAVAILABLE, e.to_string()))?;
            let bytes = resp
                .bytes()
                .map_err(|e| Error::new(error_type::T2I_UNAVAILABLE, e.to_string()))?
                .to_vec();
            return Ok(T2iImage { bytes, job_id, usd });
        }
        Err(Error::new(
            error_type::T2I_UNAVAILABLE,
            "imaginarium asset has no content_url or upstream_url",
        ))
    }
}

impl T2iProvider for Imaginarium {
    fn id(&self) -> T2iProviderId {
        T2iProviderId::Imaginarium
    }

    fn estimate(&self, n_t2i: u32, n_i2i: u32, model: Option<&str>) -> Result<T2iCost, Error> {
        // Sibling catalog is per-image (`n`). No distinct I2I unit → OQ-9 uncertain.
        let n = n_t2i.saturating_add(n_i2i).max(1);
        let body = json!({
            "kind": "image",
            "model": model.unwrap_or("2.0"),
            "n": n,
        });
        let v = self.post_json("/v1/estimate", &body)?;
        let usd = v
            .get("estimated_usd")
            .and_then(|x| x.as_f64())
            .or_else(|| v.pointer("/usage/estimated_usd").and_then(|x| x.as_f64()))
            .ok_or_else(|| {
                Error::new(
                    error_type::T2I_UNAVAILABLE,
                    "estimate missing estimated_usd",
                )
            })?;
        Ok(T2iCost {
            usd,
            usd_uncertain: n_i2i > 0,
            model: v
                .get("model")
                .and_then(|x| x.as_str())
                .map(str::to_string)
                .or_else(|| model.map(str::to_string)),
        })
    }

    fn generate(&self, prompt: &str) -> Result<T2iImage, Error> {
        let body = json!({
            "prompt": prompt,
            "model": "2.0",
            "n": 1,
            "aspect_ratio": "1:1",
            "resolution": "1k",
        });
        let v = self.post_json("/v1/images/generations", &body)?;
        self.download_asset(&v)
    }

    fn edit(
        &self,
        prompt: &str,
        hero_job_id: Option<&str>,
        extra_png: &[&[u8]],
    ) -> Result<T2iImage, Error> {
        let mut images = Vec::new();
        if let Some(id) = hero_job_id {
            images.push(format!("library:{id}"));
        }
        for png in extra_png.iter().take(2) {
            images.push(format!(
                "data:image/png;base64,{}",
                base64::Engine::encode(&base64::engine::general_purpose::STANDARD, png)
            ));
        }
        if images.is_empty() {
            return Err(Error::new(
                error_type::T2I_UNAVAILABLE,
                "edit requires library:{job_id} or source bytes (no bare paths)",
            ));
        }
        if images.len() > 3 {
            images.truncate(3);
        }
        let body = json!({
            "prompt": prompt,
            "images": images,
            "model": "2.0",
            "n": 1,
            "aspect_ratio": "1:1",
            "resolution": "1k",
        });
        let v = self.post_json("/v1/images/edits", &body)?;
        self.download_asset(&v)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::compiler::{compile_view_contract, CompileOpts};
    use crate::orbit::mock_view_png;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    fn png_bytes() -> Vec<u8> {
        mock_view_png(
            &compile_view_contract("fox", CompileOpts::default()).unwrap(),
            "hero",
        )
        .unwrap()
    }

    fn read_http(s: &mut std::net::TcpStream) -> String {
        use std::io::ErrorKind;
        s.set_read_timeout(Some(Duration::from_millis(500))).ok();
        let mut buf = Vec::new();
        let mut tmp = [0u8; 2048];
        loop {
            match s.read(&mut tmp) {
                Ok(0) => break,
                Ok(n) => buf.extend_from_slice(&tmp[..n]),
                Err(e) if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::TimedOut => {
                    break
                }
                Err(_) => break,
            }
            if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                let headers = String::from_utf8_lossy(&buf[..pos]);
                let cl = headers
                    .lines()
                    .find(|l| l.to_ascii_lowercase().starts_with("content-length:"))
                    .and_then(|l| l.split(':').nth(1))
                    .and_then(|v| v.trim().parse::<usize>().ok())
                    .unwrap_or(0);
                let need = pos + 4 + cl;
                while buf.len() < need {
                    match s.read(&mut tmp) {
                        Ok(0) => break,
                        Ok(n) => buf.extend_from_slice(&tmp[..n]),
                        Err(_) => break,
                    }
                }
                break;
            }
        }
        String::from_utf8_lossy(&buf).into_owned()
    }

    pub(crate) fn serve_fake() -> (String, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).ok();
        let addr = listener.local_addr().unwrap();
        let png = png_bytes();
        let h = thread::spawn(move || {
            let deadline = std::time::Instant::now() + Duration::from_secs(4);
            while std::time::Instant::now() < deadline {
                match listener.accept() {
                    Ok((mut s, _)) => {
                        s.set_nonblocking(false).ok();
                        let req = read_http(&mut s);
                        let (status, body, bin) = if req.starts_with("GET /health")
                            || req.contains("GET /v1/health")
                        {
                            ("200 OK", "{\"ok\":true}".into(), None)
                        } else if req.contains("POST /v1/estimate") {
                            (
                                "200 OK",
                                "{\"estimated_usd\":0.08,\"model\":\"2.0\"}".into(),
                                None,
                            )
                        } else if req.contains("POST /v1/images/") {
                            (
                                "200 OK",
                                "{\"ok\":true,\"job_id\":\"job_hero\",\"status\":\"done\",\"assets\":[{\"content_url\":\"/v1/library/job_hero/content\"}],\"usage\":{\"estimated_usd\":0.04}}".into(),
                                None,
                            )
                        } else if req.contains("GET /v1/library/") {
                            ("200 OK", String::new(), Some(png.clone()))
                        } else {
                            ("404 Not Found", "{}".into(), None)
                        };
                        if let Some(p) = bin {
                            let hdr = format!(
                                "HTTP/1.1 {status}\r\nContent-Type: image/png\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                                p.len()
                            );
                            let _ = s.write_all(hdr.as_bytes());
                            let _ = s.write_all(&p);
                        } else {
                            let hdr = format!(
                                "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                                body.len()
                            );
                            let _ = s.write_all(hdr.as_bytes());
                        }
                    }
                    Err(e)
                        if e.kind() == std::io::ErrorKind::WouldBlock
                            || e.kind() == std::io::ErrorKind::TimedOut =>
                    {
                        thread::sleep(Duration::from_millis(15));
                    }
                    Err(_) => break,
                }
            }
        });
        (format!("http://{addr}"), h)
    }

    #[test]
    fn estimate_and_generate_against_fake() {
        let (base, h) = serve_fake();
        let im = Imaginarium::new(base, Some("tok".into())).unwrap();
        assert!(im.health());
        let cost = im.estimate(1, 1, Some("2.0")).unwrap();
        assert!((cost.usd - 0.08).abs() < 1e-9);
        assert!(cost.usd_uncertain);
        let img = im.generate("a red fox").unwrap();
        assert!(!img.bytes.is_empty());
        assert_eq!(img.job_id.as_deref(), Some("job_hero"));
        let edited = im
            .edit("orbit left", Some("job_hero"), &[&img.bytes])
            .unwrap();
        assert!(!edited.bytes.is_empty());
        drop(h);
    }

    #[test]
    fn never_reads_xai_key() {
        // SAFETY: test isolation
        unsafe { std::env::set_var("XAI_API_KEY", "should-not-be-read") };
        let im = Imaginarium::new("http://127.0.0.1:9".into(), None).unwrap();
        assert!(im.token.is_none());
        unsafe { std::env::remove_var("XAI_API_KEY") };
    }

    #[test]
    fn live_estimate_skip_loud() {
        if std::env::var("TEXT2MESH_LIVE").ok().as_deref() != Some("1") {
            eprintln!("skip: TEXT2MESH_LIVE!=1 (live Imaginarium estimate)");
            return;
        }
        let im = Imaginarium::from_env().expect("Imaginarium::from_env");
        assert!(
            im.health(),
            "Imaginarium /health must succeed under TEXT2MESH_LIVE"
        );
        let cost = im.estimate(1, 0, Some("2.0")).expect("live estimate");
        assert!(cost.usd > 0.0);
    }
}
