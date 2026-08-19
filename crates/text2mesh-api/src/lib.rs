//! REST face. Bind default 127.0.0.1:8796. Non-loopback requires TEXT2MESH_TOKEN.

use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use text2mesh::error::error_type;
use text2mesh::{load_xdg_env, App, ArtifactKind, Config, Error, JobSubmit, VERSION};

#[derive(Clone)]
pub struct ApiState {
    pub app: Arc<App>,
    pub token: Option<String>,
}

pub fn router(state: ApiState) -> Router {
    Router::new()
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
    let _ = q.view_id;
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
    match state.app.artifact(&id, kind) {
        Ok((path, _sha, _n, media)) => match std::fs::read(&path) {
            Ok(bytes) => (StatusCode::OK, [(header::CONTENT_TYPE, media)], bytes).into_response(),
            Err(e) => error_response(&Error::from(e)),
        },
        Err(e) => error_response(&e),
    }
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
