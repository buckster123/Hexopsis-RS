//! MCP tool JSON schemas generated from the type layer (D18).

use serde_json::{json, Value};

use crate::types::{WAIT_DEFAULT_S, WAIT_MAX_S, WAIT_MIN_S};

pub fn tool_schemas() -> Vec<Value> {
    vec![
        tool(
            "text2mesh_system_check",
            "Honesty probe: devices, weights, keys (present/len/head), planner.would_pick. Free.",
            json!({
                "type": "object",
                "properties": {
                    "refresh": { "type": "boolean", "default": false }
                },
                "additionalProperties": false
            }),
        ),
        tool(
            "text2mesh_estimate",
            "Free cost/time estimate. Never paid. Same input subset as submit.",
            job_submit_schema(false),
        ),
        tool(
            "text2mesh_compile_contract",
            "Compile a View Contract from a prompt (pure, no T2I). S5.",
            json!({
                "type": "object",
                "properties": {
                    "prompt": { "type": "string" },
                    "quality": { "type": "string", "enum": ["preview", "standard", "high", "ultra"], "default": "standard" },
                    "camera_preset": { "type": ["string", "null"] },
                    "seed": { "type": ["integer", "null"] }
                },
                "required": ["prompt"],
                "additionalProperties": false
            }),
        ),
        tool(
            "text2mesh_submit",
            "Mint job_id, persist, plan, maybe run mock. Confirm with job_id + allow_spend.",
            job_submit_schema(true),
        ),
        tool(
            "text2mesh_status",
            "Non-blocking job snapshot.",
            json!({
                "type": "object",
                "properties": { "job_id": { "type": "string" } },
                "required": ["job_id"],
                "additionalProperties": false
            }),
        ),
        tool(
            "text2mesh_wait",
            "Block until terminal or timeout. Wrapper ok=true if the job exists; inspect job.status.",
            json!({
                "type": "object",
                "properties": {
                    "job_id": { "type": "string" },
                    "timeout_s": {
                        "type": "integer",
                        "default": WAIT_DEFAULT_S,
                        "minimum": WAIT_MIN_S,
                        "maximum": WAIT_MAX_S
                    }
                },
                "required": ["job_id"],
                "additionalProperties": false
            }),
        ),
        tool(
            "text2mesh_cancel",
            "Cancel a job. Mock is immediate cancelled.",
            json!({
                "type": "object",
                "properties": { "job_id": { "type": "string" } },
                "required": ["job_id"],
                "additionalProperties": false
            }),
        ),
        tool(
            "text2mesh_artifact",
            "Return a filesystem path (not a blob) for glb|manifest|contract|view|log.",
            json!({
                "type": "object",
                "properties": {
                    "job_id": { "type": "string" },
                    "kind": { "type": "string", "enum": ["glb", "manifest", "contract", "view", "log"], "default": "glb" },
                    "view_id": { "type": ["string", "null"] }
                },
                "required": ["job_id"],
                "additionalProperties": false
            }),
        ),
        tool(
            "text2mesh_list_jobs",
            "Newest first. Children hidden unless include_children.",
            json!({
                "type": "object",
                "properties": {
                    "status": { "type": ["string", "null"] },
                    "limit": { "type": "integer", "default": 20, "minimum": 1, "maximum": 100 },
                    "include_children": { "type": "boolean", "default": false }
                },
                "additionalProperties": false
            }),
        ),
    ]
}

fn tool(name: &str, description: &str, input_schema: Value) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": input_schema
    })
}

fn job_submit_schema(include_confirm: bool) -> Value {
    let mut props = json!({
        "prompt": { "type": "string" },
        "image_path": { "type": "string" },
        "route": { "type": "string", "enum": ["auto", "analytic", "view_contract", "native"], "default": "auto" },
        "quality": { "type": "string", "enum": ["preview", "standard", "high", "ultra"], "default": "standard" },
        "compute": { "type": "string", "enum": ["auto", "local", "remote"], "default": "auto" },
        "provider": { "type": ["string", "null"] },
        "prefer_device": { "type": ["string", "null"] },
        "seed": { "type": ["integer", "null"] },
        "camera_preset": { "type": ["string", "null"] },
        "allow_spend": { "type": "boolean", "default": false },
        "allow_neural_cad": { "type": "boolean", "default": false },
        "allow_native_text": { "type": "boolean", "default": false },
        "license_override": { "type": ["string", "null"] },
        "max_usd": { "type": "number", "default": 2.0 },
        "max_credits": { "type": ["integer", "null"] },
        "max_wall_s": {
            "type": "integer",
            "default": WAIT_DEFAULT_S,
            "minimum": WAIT_MIN_S,
            "maximum": WAIT_MAX_S
        },
        "idempotency_key": { "type": ["string", "null"] }
    });
    if include_confirm {
        props
            .as_object_mut()
            .unwrap()
            .insert("job_id".into(), json!({ "type": "string" }));
    }
    json!({
        "type": "object",
        "properties": props,
        "additionalProperties": false
    })
}

pub fn wait_timeout_schema() -> Value {
    tool_schemas()
        .into_iter()
        .find(|t| t["name"] == "text2mesh_wait")
        .map(|t| t["inputSchema"]["properties"]["timeout_s"].clone())
        .unwrap_or(Value::Null)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wait_default_1800() {
        let tools = tool_schemas();
        let wait = tools
            .iter()
            .find(|t| t["name"] == "text2mesh_wait")
            .expect("text2mesh_wait");
        assert_eq!(
            wait["inputSchema"]["properties"]["timeout_s"]["default"],
            1800
        );
        assert_eq!(
            wait["inputSchema"]["properties"]["timeout_s"]["minimum"],
            30
        );
        assert_eq!(
            wait["inputSchema"]["properties"]["timeout_s"]["maximum"],
            86400
        );
        assert_eq!(WAIT_DEFAULT_S, 1800);
    }

    #[test]
    fn schema_drift_cli_mcp_openapi() {
        // Faces must share WAIT_* constants; CLI clap default equals this schema default.
        let wait = tool_schemas()
            .into_iter()
            .find(|t| t["name"] == "text2mesh_wait")
            .unwrap();
        assert_eq!(
            wait["inputSchema"]["properties"]["timeout_s"]["default"]
                .as_u64()
                .unwrap(),
            WAIT_DEFAULT_S
        );
        assert_eq!(
            wait["inputSchema"]["properties"]["timeout_s"]["minimum"]
                .as_u64()
                .unwrap(),
            WAIT_MIN_S
        );
        assert_eq!(
            wait["inputSchema"]["properties"]["timeout_s"]["maximum"]
                .as_u64()
                .unwrap(),
            WAIT_MAX_S
        );
    }

    #[test]
    fn no_weights_pull_on_mcp() {
        for t in tool_schemas() {
            let name = t["name"].as_str().unwrap_or("");
            assert!(
                !name.contains("weight"),
                "weights pull is CLI-only, found {name}"
            );
        }
    }
}
