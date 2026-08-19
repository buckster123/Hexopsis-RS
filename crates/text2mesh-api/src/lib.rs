//! REST face. Bind default 127.0.0.1:8796. Non-loopback requires TEXT2MESH_TOKEN.

use std::sync::Arc;

use axum::extract::{Multipart, Path, Query, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use text2mesh::error::error_type;
use text2mesh::{
    load_xdg_env, App, ArtifactKind, ComputeMode, Config, Error, JobSubmit, PlaneId, Quality,
    VERSION,
};

mod ui;

#[derive(Clone)]
pub struct ApiState {
    pub app: Arc<App>,
    pub token: Option<String>,
}

pub fn router(state: ApiState) -> Router {
    Router::new()
        .route("/", get(ui_home))
        .route("/jobs/{id}", get(ui_job_page))
        .route("/ui/system-check", get(ui_system_check))
        .route("/ui/estimate", post(ui_estimate))
        .route("/ui/jobs", post(ui_create_job))
        .route("/ui/jobs/{id}", get(ui_job_fragment))
        .route("/ui/jobs/{id}/confirm", post(ui_confirm))
        .route("/ui/jobs/{id}/cancel", post(ui_cancel))
        .route("/static/htmx.min.js", get(htmx_js))
        .route("/v1/health", get(health))
        .route("/v1/system-check", get(system_check))
        .route("/v1/estimate", post(estimate))
        .route("/v1/jobs", post(create_job).get(list_jobs))
        .route("/v1/jobs/{id}", get(get_job))
        .route("/v1/jobs/{id}/cancel", post(cancel_job))
        .route("/v1/jobs/{id}/confirm", post(confirm_job))
        .route("/v1/jobs/{id}/artifact", get(get_artifact))
        .route("/v1/openapi.json", get(openapi))
        .with_state(state)
}

pub async fn run() -> anyhow::Result<()> {
    run_inner(true).await
}

/// CLI `serve` already installed a subscriber.
pub async fn run_from_cli() -> anyhow::Result<()> {
    run_inner(false).await
}

async fn run_inner(init_tracing: bool) -> anyhow::Result<()> {
    load_xdg_env();
    if init_tracing {
        tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("text2mesh=info")),
            )
            .with_writer(std::io::stderr)
            .init();
    }

    let cfg = Config::from_env();
    let addr = cfg.bind_addr().map_err(|e| anyhow::anyhow!(e))?;
    if !addr.ip().is_loopback() && cfg.token.is_none() {
        anyhow::bail!("non-loopback bind {} requires TEXT2MESH_TOKEN", cfg.bind);
    }
    let token = if addr.ip().is_loopback() {
        None
    } else {
        cfg.token.clone()
    };
    let app = App::from_env()?;
    let state = ApiState {
        app: Arc::new(app),
        token,
    };
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!(%addr, "text2mesh-api listening");
    axum::serve(listener, router(state)).await?;
    Ok(())
}

async fn health() -> Json<Value> {
    Json(json!({ "ok": true, "version": VERSION }))
}

async fn system_check(State(state): State<ApiState>) -> Json<Value> {
    let sc = text2mesh::system_check::build_system_check(
        &state.app.probe(),
        &text2mesh::SpendPolicy {
            allow_spend: state.app.allow_spend,
            max_usd: 2.0,
        },
    );
    Json(serde_json::to_value(sc).unwrap_or(json!({})))
}

async fn estimate(State(state): State<ApiState>, Json(spec): Json<JobSubmit>) -> Json<Value> {
    let est = state.app.estimate(&spec);
    Json(serde_json::to_value(est).unwrap_or(json!({})))
}

async fn create_job(
    headers: HeaderMap,
    State(state): State<ApiState>,
    Json(spec): Json<JobSubmit>,
) -> Response {
    if let Some(resp) = deny_auth(&headers, &state) {
        return resp;
    }
    match state.app.submit(spec) {
        Ok(job) => {
            let body = json!({
                "ok": true,
                "job_id": job.id,
                "status": job.status,
                "poll_url": format!("/v1/jobs/{}", job.id)
            });
            (StatusCode::ACCEPTED, Json(body)).into_response()
        }
        Err(e) => error_response(&e),
    }
}

async fn list_jobs(headers: HeaderMap, State(state): State<ApiState>) -> Response {
    if let Some(resp) = deny_auth(&headers, &state) {
        return resp;
    }
    match state.app.list(None, 20) {
        Ok(jobs) => (StatusCode::OK, Json(json!({ "ok": true, "jobs": jobs }))).into_response(),
        Err(e) => error_response(&e),
    }
}

async fn get_job(
    headers: HeaderMap,
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> Response {
    if let Some(resp) = deny_auth(&headers, &state) {
        return resp;
    }
    match state.app.status(&id) {
        Ok(job) => (StatusCode::OK, Json(json!({ "ok": true, "job": job }))).into_response(),
        Err(e) => error_response(&e),
    }
}

async fn cancel_job(
    headers: HeaderMap,
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> Response {
    if let Some(resp) = deny_auth(&headers, &state) {
        return resp;
    }
    match state.app.cancel(&id) {
        Ok(job) => (StatusCode::OK, Json(json!({ "ok": true, "job": job }))).into_response(),
        Err(e) => error_response(&e),
    }
}

async fn confirm_job(
    headers: HeaderMap,
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> Response {
    if let Some(resp) = deny_auth(&headers, &state) {
        return resp;
    }
    match state.app.confirm(&id) {
        Ok(job) => (StatusCode::OK, Json(json!({ "ok": true, "job": job }))).into_response(),
        Err(e) => error_response(&e),
    }
}

#[derive(Deserialize)]
struct ArtifactQuery {
    kind: Option<String>,
    view_id: Option<String>,
}

async fn get_artifact(
    headers: HeaderMap,
    State(state): State<ApiState>,
    Path(id): Path<String>,
    Query(q): Query<ArtifactQuery>,
) -> Response {
    if let Some(resp) = deny_auth(&headers, &state) {
        return resp;
    }
    let kind = q.kind.as_deref().unwrap_or("glb");
    let kind = match ArtifactKind::parse(kind) {
        Some(k) => k,
        None => {
            return error_response(&Error::new(
                error_type::SPEC_REJECTED,
                format!("unknown kind {kind}"),
            ));
        }
    };
    match state.app.artifact_view(&id, kind, q.view_id.as_deref()) {
        Ok((path, _sha, _n, media)) => match std::fs::read(&path) {
            Ok(bytes) => (StatusCode::OK, [(header::CONTENT_TYPE, media)], bytes).into_response(),
            Err(e) => error_response(&Error::from(e)),
        },
        Err(e) => error_response(&e),
    }
}

fn ui_forbidden(state: &ApiState) -> Option<Response> {
    if state.token.is_some() {
        Some((StatusCode::NOT_FOUND, "WebUI is loopback-only").into_response())
    } else {
        None
    }
}

async fn ui_home(State(state): State<ApiState>) -> Response {
    if let Some(r) = ui_forbidden(&state) {
        return r;
    }
    ui::page(&state.app, None).into_response()
}

async fn ui_job_page(State(state): State<ApiState>, Path(id): Path<String>) -> Response {
    if let Some(r) = ui_forbidden(&state) {
        return r;
    }
    let job = state.app.status(&id).ok();
    ui::page(&state.app, job.as_ref()).into_response()
}

async fn ui_system_check(State(state): State<ApiState>) -> Response {
    if let Some(r) = ui_forbidden(&state) {
        return r;
    }
    ui::probe_inner(&state.app).into_response()
}

async fn ui_estimate(State(state): State<ApiState>, mut multipart: Multipart) -> Response {
    if let Some(r) = ui_forbidden(&state) {
        return r;
    }
    match form_to_submit(&state.app, &mut multipart).await {
        Ok(spec) => {
            let est = state.app.estimate(&spec);
            ui::estimate_markup(&est).into_response()
        }
        Err(e) => ui_err(&e),
    }
}

async fn ui_create_job(State(state): State<ApiState>, mut multipart: Multipart) -> Response {
    if let Some(r) = ui_forbidden(&state) {
        return r;
    }
    match form_to_submit(&state.app, &mut multipart).await {
        Ok(spec) => match state.app.submit(spec) {
            Ok(job) => {
                let mut resp = ui::job_card(Some(&job)).into_response();
                if let Ok(v) = HeaderValue::from_str(&format!("/jobs/{}", job.id)) {
                    resp.headers_mut().insert("HX-Push-Url", v);
                }
                resp
            }
            Err(e) => ui_err(&e),
        },
        Err(e) => ui_err(&e),
    }
}

async fn ui_job_fragment(State(state): State<ApiState>, Path(id): Path<String>) -> Response {
    if let Some(r) = ui_forbidden(&state) {
        return r;
    }
    match state.app.status(&id) {
        Ok(job) => ui::job_card(Some(&job)).into_response(),
        Err(e) => ui_err(&e),
    }
}

async fn ui_confirm(State(state): State<ApiState>, Path(id): Path<String>) -> Response {
    if let Some(r) = ui_forbidden(&state) {
        return r;
    }
    match state.app.confirm(&id) {
        Ok(job) => ui::job_card(Some(&job)).into_response(),
        Err(e) => ui_err(&e),
    }
}

async fn ui_cancel(State(state): State<ApiState>, Path(id): Path<String>) -> Response {
    if let Some(r) = ui_forbidden(&state) {
        return r;
    }
    match state.app.cancel(&id) {
        Ok(job) => ui::job_card(Some(&job)).into_response(),
        Err(e) => ui_err(&e),
    }
}

async fn htmx_js() -> Response {
    (
        [(header::CONTENT_TYPE, "application/javascript")],
        include_bytes!("../static/htmx.min.js").as_slice(),
    )
        .into_response()
}

fn ui_err(err: &Error) -> Response {
    let body = format!("{}: {}", err.error_type, err.message);
    (StatusCode::BAD_REQUEST, body).into_response()
}

async fn form_to_submit(app: &App, multipart: &mut Multipart) -> Result<JobSubmit, Error> {
    let mut spec = JobSubmit::default();
    let mut saw_image = false;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| Error::new(error_type::SPEC_REJECTED, e.to_string()))?
    {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "prompt" => {
                let t = field
                    .text()
                    .await
                    .map_err(|e| Error::new(error_type::SPEC_REJECTED, e.to_string()))?;
                if !t.trim().is_empty() {
                    spec.prompt = Some(t);
                }
            }
            "image" => {
                let bytes = field
                    .bytes()
                    .await
                    .map_err(|e| Error::new(error_type::SPEC_REJECTED, e.to_string()))?;
                if !bytes.is_empty() {
                    let dir = app.store.root().join("uploads");
                    std::fs::create_dir_all(&dir)?;
                    let path = dir.join(format!("{}.bin", ulid::Ulid::new()));
                    std::fs::write(&path, &bytes)?;
                    spec.image_path = Some(path.to_string_lossy().into_owned());
                    saw_image = true;
                }
            }
            "quality" => {
                let t = field.text().await.unwrap_or_default();
                spec.quality = match t.as_str() {
                    "preview" => Quality::Preview,
                    "high" => Quality::High,
                    "ultra" => Quality::Ultra,
                    _ => Quality::Standard,
                };
            }
            "compute" => {
                let t = field.text().await.unwrap_or_default();
                spec.compute = match t.as_str() {
                    "local" => ComputeMode::Local,
                    "remote" => ComputeMode::Remote,
                    _ => ComputeMode::Auto,
                };
            }
            "provider" => {
                let t = field.text().await.unwrap_or_default();
                spec.provider = PlaneId::parse(&t);
            }
            "allow_spend" => spec.allow_spend = true,
            "allow_native_text" => spec.allow_native_text = true,
            _ => {}
        }
    }
    let _ = saw_image;
    Ok(spec)
}

async fn openapi() -> Json<Value> {
    Json(json!({
        "openapi": "3.1.0",
        "info": { "title": "text2mesh", "version": VERSION },
        "paths": {
            "/v1/jobs/{id}": {
                "get": { "summary": "Poll job. Wrapper ok means found, not meshed." }
            }
        }
    }))
}

fn deny_auth(headers: &HeaderMap, state: &ApiState) -> Option<Response> {
    let Some(token) = &state.token else {
        return None;
    };
    let got = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));
    if got == Some(token.as_str()) {
        None
    } else {
        Some(
            (
                StatusCode::UNAUTHORIZED,
                Json(json!({ "ok": false, "error_type": "not_configured", "message": "bearer token required" })),
            )
                .into_response(),
        )
    }
}

fn error_response(err: &Error) -> Response {
    let status = match err.error_type.as_str() {
        "spec.rejected" => StatusCode::BAD_REQUEST,
        "not_found" => StatusCode::NOT_FOUND,
        "export.not_ready" | "not_configured" | "weights_missing" | "feature_off" => {
            StatusCode::CONFLICT
        }
        t if t.starts_with("spend.gated") || t.starts_with("license.") => StatusCode::FORBIDDEN,
        "spend.provider_402" | "spend.estimate_exceeded" => StatusCode::PAYMENT_REQUIRED,
        "rate_limit" => StatusCode::TOO_MANY_REQUESTS,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };
    (
        status,
        Json(json!({
            "ok": false,
            "error_type": err.error_type,
            "message": err.message,
            "hint": err.hint,
        })),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use text2mesh::{ComputeMode, JobStatus, PlaneId};
    use tower::ServiceExt;

    fn png_path() -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::TempDir::new().unwrap();
        let p = dir.path().join("dot.png");
        std::fs::write(&p, text2mesh::minimal_png_1x1()).unwrap();
        (dir, p)
    }

    async fn body_json(res: axum::http::Response<Body>) -> Value {
        let bytes = axum::body::to_bytes(res.into_body(), 1024 * 1024)
            .await
            .unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn job_json_roundtrip_http_mock() {
        let app = App::for_test(true);
        let state = ApiState {
            app: Arc::new(app),
            token: None,
        };
        let router = router(state);
        let (_tmp, png) = png_path();
        let spec = JobSubmit {
            image_path: Some(png.to_string_lossy().into_owned()),
            compute: ComputeMode::Local,
            provider: Some(PlaneId::LocalMock),
            ..JobSubmit::default()
        };
        let res = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/jobs")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&spec).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::ACCEPTED);
        let created = body_json(res).await;
        assert_eq!(created["ok"], true);
        assert!(created.get("artifact_url").is_none());
        let id = created["job_id"].as_str().unwrap().to_string();
        let poll = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/v1/jobs/{id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(poll.status(), StatusCode::OK);
        let snap = body_json(poll).await;
        assert_eq!(snap["ok"], true);
        assert_eq!(snap["job"]["status"], "degraded");
        let job: text2mesh::MeshJob = serde_json::from_value(snap["job"].clone()).unwrap();
        assert_eq!(job.status, JobStatus::Degraded);
    }

    #[tokio::test]
    async fn export_not_ready_409() {
        let app = App::for_test(false);
        let mut job = text2mesh::MeshJob::from_submit(
            "01TESTEXPORTNOTREADY0000000".into(),
            &JobSubmit {
                prompt: Some("queued".into()),
                ..JobSubmit::default()
            },
        );
        job.status = JobStatus::Queued;
        app.store.create(&job).unwrap();
        let id = job.id.clone();
        let state = ApiState {
            app: Arc::new(app),
            token: None,
        };
        let res = router(state)
            .oneshot(
                Request::builder()
                    .uri(format!("/v1/jobs/{id}/artifact?kind=glb"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CONFLICT);
        let v = body_json(res).await;
        assert_eq!(v["error_type"], "export.not_ready");
    }

    #[tokio::test]
    async fn wrapper_ok_poll_running() {
        let app = App::for_test(false);
        let mut job = text2mesh::MeshJob::from_submit(
            "01TESTWRAPPEROKRUNNING00000".into(),
            &JobSubmit {
                prompt: Some("run".into()),
                ..JobSubmit::default()
            },
        );
        job.status = JobStatus::Running;
        app.store.create(&job).unwrap();
        let id = job.id.clone();
        let state = ApiState {
            app: Arc::new(app),
            token: None,
        };
        let res = router(state)
            .oneshot(
                Request::builder()
                    .uri(format!("/v1/jobs/{id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let v = body_json(res).await;
        assert_eq!(v["ok"], true);
        assert_eq!(v["job"]["status"], "running");
    }

    #[tokio::test]
    async fn webui_home_is_hexopsis_not_saas() {
        let state = ApiState {
            app: Arc::new(App::for_test(true)),
            token: None,
        };
        let res = router(state)
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(res.into_body(), 1024 * 1024)
            .await
            .unwrap();
        let html = String::from_utf8(bytes.to_vec()).unwrap();
        assert!(html.contains("Hexopsis"));
        assert!(html.contains("#e0a04a"), "amber token missing");
        assert!(!html.contains("Success!"));
        assert!(!html.contains("confetti"));
    }

    #[tokio::test]
    async fn webui_probe_never_uses_ok_for_ready() {
        let state = ApiState {
            app: Arc::new(App::for_test(false)),
            token: None,
        };
        let res = router(state)
            .oneshot(
                Request::builder()
                    .uri("/ui/system-check")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let bytes = axum::body::to_bytes(res.into_body(), 64 * 1024)
            .await
            .unwrap();
        let html = String::from_utf8(bytes.to_vec()).unwrap();
        assert!(html.contains("Not ready") || html.contains("Ready"));
        assert!(html.contains("would_pick"));
        assert!(!html.contains("ok="));
    }

    #[tokio::test]
    async fn webui_degraded_banner_not_green() {
        let app = App::for_test(true);
        let (_tmp, png) = png_path();
        let job = app
            .submit(text2mesh::JobSubmit {
                image_path: Some(png.to_string_lossy().into_owned()),
                compute: ComputeMode::Local,
                provider: Some(PlaneId::LocalMock),
                ..text2mesh::JobSubmit::default()
            })
            .unwrap();
        assert_eq!(job.status, JobStatus::Degraded);
        let id = job.id.clone();
        let state = ApiState {
            app: Arc::new(app),
            token: None,
        };
        let res = router(state)
            .oneshot(
                Request::builder()
                    .uri(format!("/ui/jobs/{id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let bytes = axum::body::to_bytes(res.into_body(), 1024 * 1024)
            .await
            .unwrap();
        let html = String::from_utf8(bytes.to_vec()).unwrap();
        assert!(html.contains("Degraded"));
        assert!(html.contains("Download degraded GLB"));
        assert!(!html.contains("Download GLB</a>")); // exact succeeded label
        assert!(!html.contains("Success!"));
    }

    #[tokio::test]
    async fn health_ok() {
        let state = ApiState {
            app: Arc::new(App::for_test(false)),
            token: None,
        };
        let res = router(state)
            .oneshot(
                Request::builder()
                    .uri("/v1/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let v = body_json(res).await;
        assert_eq!(v["ok"], true);
    }
}
