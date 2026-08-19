//! MCP dispatch (protocol 2024-11-05). stdout is JSON-RPC only.

use serde_json::{json, Value};
use text2mesh::error::error_type;
use text2mesh::{
    compile_view_contract, mcp_schema, App, ArtifactKind, CompileOpts, Error, JobStatus, JobSubmit,
    T2iProviderId, VERSION, WAIT_DEFAULT_S, WAIT_MAX_S, WAIT_MIN_S,
};

pub const PROTOCOL_VERSION: &str = "2024-11-05";

pub fn initialize_result() -> Value {
    json!({
        "protocolVersion": PROTOCOL_VERSION,
        "capabilities": { "tools": {} },
        "serverInfo": {
            "name": "text2mesh-mcp",
            "version": VERSION,
            "implementation": "hand-rolled"
        }
    })
}

pub fn tools_list_result() -> Value {
    json!({ "tools": mcp_schema::tool_schemas() })
}

pub fn dispatch_sync(app: &App, req: &Value) -> Option<Value> {
    let method = req.get("method").and_then(|m| m.as_str()).unwrap_or("");
    let id = req.get("id").cloned();
    if id.is_none() || method.starts_with("notifications/") {
        return None;
    }
    let result = match method {
        "initialize" => Ok(initialize_result()),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(tools_list_result()),
        "tools/call" => return Some(rpc_ok(id, tools_call_sync(app, req))),
        other => Err(format!("method not found: {other}")),
    };
    Some(match result {
        Ok(r) => rpc_ok(id, r),
        Err(msg) if msg.starts_with("method not found") => json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": { "code": -32601, "message": msg }
        }),
        Err(msg) => rpc_ok(id, tool_error(&Error::new(error_type::INTERNAL, msg))),
    })
}

pub async fn dispatch(app: &App, req: &Value) -> Option<Value> {
    let method = req.get("method").and_then(|m| m.as_str()).unwrap_or("");
    let id = req.get("id").cloned();
    if id.is_none() || method.starts_with("notifications/") {
        return None;
    }
    if method == "tools/call" {
        return Some(rpc_ok(id, tools_call(app, req).await));
    }
    dispatch_sync(app, req)
}

fn rpc_ok(id: Option<Value>, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn tool_ok(value: &Value) -> Value {
    json!({
        "content": [{ "type": "text", "text": pretty(value) }],
        "isError": false
    })
}

fn tool_error(err: &Error) -> Value {
    let payload = json!({
        "ok": false,
        "error_type": err.error_type,
        "message": err.message,
        "hint": err.hint,
        "also": err.also
    });
    json!({
        "content": [{ "type": "text", "text": pretty(&payload) }],
        "isError": true
    })
}

fn pretty(v: &Value) -> String {
    serde_json::to_string_pretty(v).unwrap_or_else(|_| v.to_string())
}

fn tools_call_sync(app: &App, req: &Value) -> Value {
    let params = req.get("params").cloned().unwrap_or(Value::Null);
    let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let args = params.get("arguments").cloned().unwrap_or(json!({}));
    match name {
        "text2mesh_wait" => tool_error(&Error::new(
            error_type::INTERNAL,
            "text2mesh_wait requires the async MCP loop",
        )),
        other => call_tool(app, other, &args),
    }
}

async fn tools_call(app: &App, req: &Value) -> Value {
    let params = req.get("params").cloned().unwrap_or(Value::Null);
    let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let args = params.get("arguments").cloned().unwrap_or(json!({}));
    if name == "text2mesh_wait" {
        return call_wait(app, &args).await;
    }
    call_tool(app, name, &args)
}

fn call_tool(app: &App, name: &str, args: &Value) -> Value {
    match name {
        "text2mesh_system_check" => {
            let sc = text2mesh::system_check::build_system_check(
                &app.probe(),
                &text2mesh::SpendPolicy {
                    allow_spend: app.allow_spend,
                    max_usd: 2.0,
                },
            );
            tool_ok(&serde_json::to_value(sc).unwrap_or(json!({})))
        }
        "text2mesh_estimate" => match serde_json::from_value::<JobSubmit>(args.clone()) {
            Ok(spec) => tool_ok(&serde_json::to_value(app.estimate(&spec)).unwrap_or(json!({}))),
            Err(e) => tool_error(&Error::new(error_type::SPEC_REJECTED, e.to_string())),
        },
        "text2mesh_compile_contract" => {
            let prompt = args
                .get("prompt")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim();
            if prompt.is_empty() {
                tool_error(&Error::new(error_type::SPEC_REJECTED, "prompt is required"))
            } else {
                match compile_view_contract(
                    prompt,
                    CompileOpts {
                        family_seed: args.get("seed").and_then(|v| v.as_u64()).unwrap_or(42),
                        t2i_provider: T2iProviderId::Mock,
                        ..CompileOpts::default()
                    },
                ) {
                    Ok(c) => tool_ok(&serde_json::to_value(&c).unwrap_or(json!({}))),
                    Err(e) => tool_error(&e),
                }
            }
        }
        "text2mesh_submit" => call_submit(app, args),
        "text2mesh_status" => match job_id(args) {
            Ok(id) => match app.status(&id) {
                Ok(job) => tool_ok(&json!({ "ok": true, "job": job })),
                Err(e) => tool_error(&e),
            },
            Err(e) => tool_error(&e),
        },
        "text2mesh_cancel" => match job_id(args) {
            Ok(id) => match app.cancel(&id) {
                Ok(job) => tool_ok(&json!({ "ok": true, "job": job })),
                Err(e) => tool_error(&e),
            },
            Err(e) => tool_error(&e),
        },
        "text2mesh_artifact" => call_artifact(app, args),
        "text2mesh_list_jobs" => {
            let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(20) as u32;
            let status = args
                .get("status")
                .and_then(|v| v.as_str())
                .and_then(parse_status);
            match app.list(status, limit) {
                Ok(jobs) => tool_ok(&json!({ "ok": true, "jobs": jobs })),
                Err(e) => tool_error(&e),
            }
        }
        other => tool_error(&Error::new(
            error_type::UNSUPPORTED,
            format!("unknown tool {other}"),
        )),
    }
}

fn call_submit(app: &App, args: &Value) -> Value {
    if let Some(id) = args.get("job_id").and_then(|v| v.as_str()) {
        if args.get("allow_spend").and_then(|v| v.as_bool()) == Some(true) {
            return match app.confirm(id) {
                Ok(job) => tool_ok(&json!({ "ok": true, "job": job })),
                Err(e) => tool_error(&e),
            };
        }
    }
    match serde_json::from_value::<JobSubmit>(args.clone()) {
        Ok(spec) => match app.submit(spec) {
            Ok(job) => tool_ok(&json!({ "ok": true, "job": job })),
            Err(e) => tool_error(&e),
        },
        Err(e) => tool_error(&Error::new(error_type::SPEC_REJECTED, e.to_string())),
    }
}

async fn call_wait(app: &App, args: &Value) -> Value {
    let id = match job_id(args) {
        Ok(id) => id,
        Err(e) => return tool_error(&e),
    };
    let timeout_s = args
        .get("timeout_s")
        .and_then(|v| v.as_u64())
        .unwrap_or(WAIT_DEFAULT_S);
    if !(WAIT_MIN_S..=WAIT_MAX_S).contains(&timeout_s) {
        return tool_error(&Error::new(
            error_type::SPEC_REJECTED,
            format!("timeout_s {timeout_s} outside {WAIT_MIN_S}..={WAIT_MAX_S}"),
        ));
    }
    match app.wait_async(&id, timeout_s).await {
        Ok(w) => {
            // wrapper ok=true whenever the job exists (design §10.6)
            tool_ok(&json!({
                "ok": true,
                "job": w.job,
                "wait_timed_out": w.wait_timed_out,
                "error_type": w.error_type
            }))
        }
        Err(e) => tool_error(&e),
    }
}

fn call_artifact(app: &App, args: &Value) -> Value {
    let id = match job_id(args) {
        Ok(id) => id,
        Err(e) => return tool_error(&e),
    };
    let kind = args.get("kind").and_then(|v| v.as_str()).unwrap_or("glb");
    let kind = match ArtifactKind::parse(kind) {
        Some(k) => k,
        None => {
            return tool_error(&Error::new(
                error_type::SPEC_REJECTED,
                format!("unknown kind {kind}"),
            ));
        }
    };
    match app.artifact(&id, kind) {
        Ok((path, sha, bytes, media)) => tool_ok(&json!({
            "path": path,
            "sha256": sha,
            "bytes": bytes,
            "media_type": media
        })),
        Err(e) => tool_error(&e),
    }
}

fn job_id(args: &Value) -> Result<String, Error> {
    args.get("job_id")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .ok_or_else(|| Error::new(error_type::SPEC_REJECTED, "job_id required"))
}

fn parse_status(s: &str) -> Option<JobStatus> {
    serde_json::from_value(json!(s)).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use text2mesh::{ComputeMode, PlaneId};

    #[test]
    fn initialize_protocol() {
        let v = initialize_result();
        assert_eq!(v["protocolVersion"], "2024-11-05");
        assert_eq!(v["serverInfo"]["name"], "text2mesh-mcp");
    }

    #[test]
    fn wait_schema_default_1800() {
        let list = tools_list_result();
        let wait = list["tools"]
            .as_array()
            .unwrap()
            .iter()
            .find(|t| t["name"] == "text2mesh_wait")
            .unwrap();
        assert_eq!(
            wait["inputSchema"]["properties"]["timeout_s"]["default"],
            1800
        );
    }

    #[test]
    fn notification_skips_response() {
        let app = App::for_test(true);
        let req = json!({"jsonrpc":"2.0","method":"notifications/initialized"});
        assert!(dispatch_sync(&app, &req).is_none());
    }

    #[test]
    fn echo_id() {
        let app = App::for_test(true);
        let req = json!({"jsonrpc":"2.0","id":"abc","method":"ping"});
        let resp = dispatch_sync(&app, &req).unwrap();
        assert_eq!(resp["id"], "abc");
    }

    #[test]
    fn submit_mock_is_error_false_degraded() {
        let app = App::for_test(true);
        let tmp = tempfile::TempDir::new().unwrap();
        let png = tmp.path().join("dot.png");
        std::fs::write(&png, text2mesh::minimal_png_1x1()).unwrap();
        let req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "text2mesh_submit",
                "arguments": {
                    "image_path": png.to_string_lossy(),
                    "compute": "local",
                    "provider": "local.mock"
                }
            }
        });
        let resp = dispatch_sync(&app, &req).unwrap();
        assert_eq!(resp["result"]["isError"], false);
        let text = resp["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("degraded"));
        let _ = ComputeMode::Local;
        let _ = PlaneId::LocalMock;
    }
}
