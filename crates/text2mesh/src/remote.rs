//! Shared remote HTTP: 402/429 mapping, poll, download. No vendor keys in logs.

use std::time::Duration;

use serde_json::Value;

use crate::error::{error_type, Error};

pub const MESHY_DEFAULT: &str = "https://api.meshy.ai";
pub const TRIPO_DEFAULT: &str = "https://openapi.tripo3d.ai/v3";

#[derive(Debug, Clone)]
pub struct RemoteArtifact {
    pub glb: Vec<u8>,
    pub engine: String,
    pub upstream_id: String,
    pub usd: Option<f64>,
}

#[derive(Debug)]
pub enum RemoteOutcome {
    Done(RemoteArtifact),
    Waiting { upstream_id: String },
}

pub fn require_key(name: &str) -> Result<String, Error> {
    match std::env::var(name) {
        Ok(v) if !v.is_empty() => Ok(v),
        _ => Err(
            Error::new(error_type::NOT_CONFIGURED, format!("{name} is missing")).with_hint(
                "set the key in ~/.config/text2mesh/env (0600); we never POST without it",
            ),
        ),
    }
}

pub fn data_uri_png(bytes: &[u8]) -> String {
    format!(
        "data:image/png;base64,{}",
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, bytes)
    )
}

pub fn map_vendor_http(
    status: reqwest::StatusCode,
    body: &str,
    retry_after: Option<&str>,
) -> Error {
    match status.as_u16() {
        402 => Error::new(error_type::SPEND_PROVIDER_402, body),
        429 => {
            let mut e = Error::new(error_type::RATE_LIMIT, body);
            if let Some(ra) = retry_after {
                e = e.with_hint(format!("Retry-After: {ra}"));
            }
            e
        }
        401 | 403 => Error::new(error_type::NOT_CONFIGURED, "vendor token rejected"),
        _ => Error::new(error_type::UNSUPPORTED, format!("vendor {status}: {body}")),
    }
}

pub struct VendorHttp {
    pub base: String,
    token: String,
    client: reqwest::blocking::Client,
    pub poll: Duration,
    pub wall: Duration,
}

impl VendorHttp {
    pub fn new(base: String, token: String) -> Result<Self, Error> {
        if token.is_empty() {
            return Err(Error::new(
                error_type::NOT_CONFIGURED,
                "vendor token is empty",
            ));
        }
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .map_err(|e| Error::new(error_type::INTERNAL, e.to_string()))?;
        Ok(Self {
            base: base.trim_end_matches('/').into(),
            token,
            client,
            poll: Duration::from_secs(2),
            wall: Duration::from_secs(1_800),
        })
    }

    pub fn fast_poll(mut self) -> Self {
        self.poll = Duration::from_millis(10);
        self
    }

    fn auth(&self, req: reqwest::blocking::RequestBuilder) -> reqwest::blocking::RequestBuilder {
        req.header("Authorization", format!("Bearer {}", self.token))
    }

    pub fn post_json(&self, path: &str, body: &Value) -> Result<Value, Error> {
        let resp = self
            .auth(self.client.post(format!("{}{path}", self.base)))
            .json(body)
            .send()
            .map_err(|e| Error::new(error_type::UNSUPPORTED, e.to_string()))?;
        let retry = resp
            .headers()
            .get("retry-after")
            .and_then(|v| v.to_str().ok())
            .map(str::to_string);
        let status = resp.status();
        let text = resp.text().unwrap_or_default();
        if !status.is_success() {
            return Err(map_vendor_http(status, &text, retry.as_deref()));
        }
        serde_json::from_str(&text).map_err(|e| Error::new(error_type::UNSUPPORTED, e.to_string()))
    }

    pub fn get_json(&self, path_or_url: &str) -> Result<Value, Error> {
        let url = if path_or_url.starts_with("http") {
            path_or_url.to_string()
        } else {
            format!("{}{path_or_url}", self.base)
        };
        let resp = self
            .auth(self.client.get(url))
            .send()
            .map_err(|e| Error::new(error_type::UNSUPPORTED, e.to_string()))?;
        let retry = resp
            .headers()
            .get("retry-after")
            .and_then(|v| v.to_str().ok())
            .map(str::to_string);
        let status = resp.status();
        let text = resp.text().unwrap_or_default();
        if !status.is_success() {
            return Err(map_vendor_http(status, &text, retry.as_deref()));
        }
        serde_json::from_str(&text).map_err(|e| Error::new(error_type::UNSUPPORTED, e.to_string()))
    }

    pub fn get_bytes(&self, url: &str) -> Result<Vec<u8>, Error> {
        let resp = self
            .client
            .get(url)
            .send()
            .map_err(|e| Error::new(error_type::UNSUPPORTED, e.to_string()))?;
        if !resp.status().is_success() {
            return Err(map_vendor_http(
                resp.status(),
                &resp.text().unwrap_or_default(),
                None,
            ));
        }
        resp.bytes()
            .map(|b| b.to_vec())
            .map_err(|e| Error::new(error_type::UNSUPPORTED, e.to_string()))
    }
}

pub fn meshy_task_id(v: &Value) -> Result<String, Error> {
    v.get("result")
        .and_then(|x| x.as_str())
        .or_else(|| v.get("id").and_then(|x| x.as_str()))
        .map(str::to_string)
        .ok_or_else(|| Error::new(error_type::UNSUPPORTED, "meshy create missing result id"))
}

pub fn meshy_poll_state(v: &Value) -> (&str, Option<&str>, Option<f64>) {
    let status = v.get("status").and_then(|s| s.as_str()).unwrap_or("");
    let glb = v.pointer("/model_urls/glb").and_then(|x| x.as_str());
    let credits = v.get("consumed_credits").and_then(|x| x.as_f64());
    (status, glb, credits)
}

pub fn tripo_task_id(v: &Value) -> Result<String, Error> {
    v.pointer("/data/task_id")
        .and_then(|x| x.as_str())
        .or_else(|| v.get("task_id").and_then(|x| x.as_str()))
        .map(str::to_string)
        .ok_or_else(|| Error::new(error_type::UNSUPPORTED, "tripo create missing task_id"))
}

pub fn tripo_poll_state(v: &Value) -> (&str, Option<&str>) {
    let data = v.get("data").unwrap_or(v);
    let status = data.get("status").and_then(|s| s.as_str()).unwrap_or("");
    let glb = data
        .pointer("/output/model_url")
        .and_then(|x| x.as_str())
        .or_else(|| data.pointer("/output/model").and_then(|x| x.as_str()));
    (status, glb)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;
    use std::time::Instant;

    #[test]
    fn meshy_create_and_poll_fixtures() {
        let create: Value =
            serde_json::from_str(include_str!("../tests/fixtures/remote/meshy_create.json"))
                .unwrap();
        assert_eq!(meshy_task_id(&create).unwrap(), "task_meshy_1");
        let poll: Value = serde_json::from_str(include_str!(
            "../tests/fixtures/remote/meshy_task_succeeded.json"
        ))
        .unwrap();
        let (st, glb, creds) = meshy_poll_state(&poll);
        assert_eq!(st, "SUCCEEDED");
        assert!(glb.unwrap().contains("model.glb"));
        assert_eq!(creds, Some(30.0));
    }

    #[test]
    fn tripo_create_and_poll_fixtures() {
        let create: Value =
            serde_json::from_str(include_str!("../tests/fixtures/remote/tripo_create.json"))
                .unwrap();
        assert_eq!(tripo_task_id(&create).unwrap(), "task_tripo_1");
        let poll: Value = serde_json::from_str(include_str!(
            "../tests/fixtures/remote/tripo_task_success.json"
        ))
        .unwrap();
        let (st, glb) = tripo_poll_state(&poll);
        assert_eq!(st, "success");
        assert!(glb.unwrap().starts_with("https://"));
    }

    #[test]
    fn map_402_and_429() {
        let e = map_vendor_http(reqwest::StatusCode::PAYMENT_REQUIRED, "no credits", None);
        assert_eq!(e.error_type, error_type::SPEND_PROVIDER_402);
        let e = map_vendor_http(
            reqwest::StatusCode::TOO_MANY_REQUESTS,
            "slow down",
            Some("2"),
        );
        assert_eq!(e.error_type, error_type::RATE_LIMIT);
        assert!(e.hint.as_deref().unwrap().contains("Retry-After"));
    }

    #[test]
    fn missing_key_never_builds_client() {
        let err = require_key("TEXT2MESH_NO_SUCH_VENDOR_KEY_EVER").unwrap_err();
        assert_eq!(err.error_type, error_type::NOT_CONFIGURED);
    }

    #[derive(Clone, Copy)]
    pub(crate) enum FakeMode {
        Ok,
        Credit402,
        Rate429,
    }

    fn read_http(s: &mut std::net::TcpStream) -> String {
        s.set_read_timeout(Some(Duration::from_millis(400))).ok();
        let mut buf = Vec::new();
        let mut tmp = [0u8; 2048];
        loop {
            match s.read(&mut tmp) {
                Ok(0) => break,
                Ok(n) => buf.extend_from_slice(&tmp[..n]),
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

    pub(crate) fn serve_fake(mode: FakeMode) -> (String, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).ok();
        let addr = listener.local_addr().unwrap();
        let glb = crate::mock_glb::emit_mock_glb_seeded(b"remote-fixture", 1);
        let h = thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(4);
            while Instant::now() < deadline {
                match listener.accept() {
                    Ok((mut s, _)) => {
                        s.set_nonblocking(false).ok();
                        let req = read_http(&mut s);
                        let (status, ctype, body, bin) = if req.contains("POST /openapi/") {
                            match mode {
                                FakeMode::Credit402 => (
                                    "402 Payment Required",
                                    "application/json",
                                    "no credits".into(),
                                    None,
                                ),
                                FakeMode::Rate429 => (
                                    "429 Too Many Requests",
                                    "application/json",
                                    "slow".into(),
                                    None,
                                ),
                                FakeMode::Ok => (
                                    "200 OK",
                                    "application/json",
                                    "{\"result\":\"task_meshy_1\"}".into(),
                                    None,
                                ),
                            }
                        } else if req.contains("POST /generation/") {
                            match mode {
                                FakeMode::Credit402 => (
                                    "402 Payment Required",
                                    "application/json",
                                    "no credits".into(),
                                    None,
                                ),
                                FakeMode::Rate429 => (
                                    "429 Too Many Requests",
                                    "application/json",
                                    "slow".into(),
                                    None,
                                ),
                                FakeMode::Ok => (
                                    "200 OK",
                                    "application/json",
                                    "{\"code\":0,\"data\":{\"task_id\":\"task_tripo_1\"}}".into(),
                                    None,
                                ),
                            }
                        } else if req.contains("GET /openapi/v1/image-to-3d/")
                            || req.contains("GET /openapi/v2/text-to-3d/")
                        {
                            let body = format!(
                                "{{\"id\":\"task_meshy_1\",\"status\":\"SUCCEEDED\",\"consumed_credits\":30,\"model_urls\":{{\"glb\":\"http://{addr}/glb\"}}}}"
                            );
                            ("200 OK", "application/json", body, None)
                        } else if req.contains("GET /tasks/") {
                            let body = format!(
                                "{{\"code\":0,\"data\":{{\"task_id\":\"task_tripo_1\",\"status\":\"success\",\"progress\":100,\"output\":{{\"model_url\":\"http://{addr}/glb\"}}}}}}"
                            );
                            ("200 OK", "application/json", body, None)
                        } else if req.contains("GET /glb") {
                            (
                                "200 OK",
                                "model/gltf-binary",
                                String::new(),
                                Some(glb.clone()),
                            )
                        } else {
                            ("404 Not Found", "application/json", "{}".into(), None)
                        };
                        let extra = if status.starts_with("429") {
                            "Retry-After: 2\r\n"
                        } else {
                            ""
                        };
                        if let Some(p) = bin {
                            let hdr = format!(
                                "HTTP/1.1 {status}\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                                p.len()
                            );
                            let _ = s.write_all(hdr.as_bytes());
                            let _ = s.write_all(&p);
                        } else {
                            let hdr = format!(
                                "HTTP/1.1 {status}\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\n{extra}Connection: close\r\n\r\n{body}",
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
    fn meshy_ok_downloads_glb() {
        let (base, h) = serve_fake(FakeMode::Ok);
        let m = crate::remote_meshy::Meshy::for_test(base, "tok".into()).unwrap();
        let png = crate::types::minimal_png_1x1();
        let out = m.run_image(&png).unwrap();
        let RemoteOutcome::Done(art) = out else {
            panic!("expected done");
        };
        assert_eq!(art.engine, "meshy");
        assert_eq!(art.upstream_id, "task_meshy_1");
        assert!(crate::mock_glb::has_vertex_color(&art.glb));
        drop(h);
    }

    #[test]
    fn tripo_ok_downloads_glb() {
        let (base, h) = serve_fake(FakeMode::Ok);
        let t = crate::remote_tripo::Tripo::for_test(base, "tok".into()).unwrap();
        let png = crate::types::minimal_png_1x1();
        let out = t.run_image(&png).unwrap();
        let RemoteOutcome::Done(art) = out else {
            panic!("expected done");
        };
        assert_eq!(art.engine, "tripo");
        assert_eq!(art.upstream_id, "task_tripo_1");
        drop(h);
    }

    #[test]
    fn meshy_402_maps() {
        let (base, h) = serve_fake(FakeMode::Credit402);
        let m = crate::remote_meshy::Meshy::for_test(base, "tok".into()).unwrap();
        let err = m.run_image(&crate::types::minimal_png_1x1()).unwrap_err();
        assert_eq!(err.error_type, error_type::SPEND_PROVIDER_402);
        drop(h);
    }

    #[test]
    fn tripo_429_maps() {
        let (base, h) = serve_fake(FakeMode::Rate429);
        let t = crate::remote_tripo::Tripo::for_test(base, "tok".into()).unwrap();
        let err = t.run_image(&crate::types::minimal_png_1x1()).unwrap_err();
        assert_eq!(err.error_type, error_type::RATE_LIMIT);
        assert!(err.hint.as_deref().unwrap().contains("Retry-After"));
        drop(h);
    }

    #[test]
    fn empty_token_never_posts() {
        let err = match VendorHttp::new("http://127.0.0.1:9".into(), "".into()) {
            Err(e) => e,
            Ok(_) => panic!("empty token must not build a client"),
        };
        assert_eq!(err.error_type, error_type::NOT_CONFIGURED);
    }

    #[test]
    fn live_skip_loud() {
        if std::env::var("TEXT2MESH_LIVE").ok().as_deref() != Some("1") {
            eprintln!("skip: TEXT2MESH_LIVE!=1 (live Meshy/Tripo)");
            return;
        }
        let _ = crate::remote_meshy::Meshy::from_env();
        let _ = crate::remote_tripo::Tripo::from_env();
    }
}
