//! text2mesh-mcp — newline-delimited JSON-RPC over stdio. stdout is sacred.

use std::sync::Arc;

use anyhow::Result;
use serde_json::{json, Value};
use text2mesh::{load_xdg_env, App};
use text2mesh_mcp::{dispatch, PROTOCOL_VERSION};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::sync::Mutex;

const MAX_FRAME_BYTES: u64 = 32 * 1024 * 1024;

enum Frame {
    Value(Value),
    Eof,
    ParseError(String),
    Oversized,
}

#[tokio::main]
async fn main() -> Result<()> {
    load_xdg_env();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("text2mesh=info")),
        )
        .with_writer(std::io::stderr)
        .init();

    tracing::info!(protocol = PROTOCOL_VERSION, "text2mesh-mcp starting");

    let app = Arc::new(App::from_env()?);
    let stdout = Arc::new(Mutex::new(tokio::io::stdout()));
    let mut reader = BufReader::new(tokio::io::stdin());

    loop {
        match read_frame(&mut reader).await? {
            Frame::Eof => break,
            Frame::Oversized => {
                write_frame(
                    &stdout,
                    &json!({
                        "jsonrpc": "2.0",
                        "id": null,
                        "error": { "code": -32700, "message": "parse error: frame exceeds 32 MiB" }
                    }),
                )
                .await?;
            }
            Frame::ParseError(e) => {
                write_frame(
                    &stdout,
                    &json!({
                        "jsonrpc": "2.0",
                        "id": null,
                        "error": { "code": -32700, "message": format!("parse error: {e}") }
                    }),
                )
                .await?;
            }
            Frame::Value(req) => {
                let app = Arc::clone(&app);
                let stdout = Arc::clone(&stdout);
                tokio::spawn(async move {
                    if let Some(resp) = dispatch(&app, &req).await {
                        if let Err(e) = write_frame(&stdout, &resp).await {
                            tracing::error!(error = %e, "write frame");
                        }
                    }
                });
            }
        }
    }
    Ok(())
}

async fn read_frame(reader: &mut BufReader<tokio::io::Stdin>) -> Result<Frame> {
    let mut line = String::new();
    let n = {
        let mut limited = reader.take(MAX_FRAME_BYTES);
        limited.read_line(&mut line).await? as u64
    };
    if n == 0 {
        return Ok(Frame::Eof);
    }
    if n >= MAX_FRAME_BYTES && !line.ends_with('\n') {
        loop {
            let buf = reader.fill_buf().await?;
            if buf.is_empty() {
                break;
            }
            if let Some(pos) = buf.iter().position(|&b| b == b'\n') {
                reader.consume(pos + 1);
                break;
            }
            let len = buf.len();
            reader.consume(len);
        }
        return Ok(Frame::Oversized);
    }
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Ok(Frame::Value(
            json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        ));
    }
    match serde_json::from_str(trimmed) {
        Ok(v) => Ok(Frame::Value(v)),
        Err(e) => Ok(Frame::ParseError(e.to_string())),
    }
}

async fn write_frame(stdout: &Arc<Mutex<tokio::io::Stdout>>, value: &Value) -> Result<()> {
    let mut buf = serde_json::to_string(value)?;
    buf.push('\n');
    let mut out = stdout.lock().await;
    out.write_all(buf.as_bytes()).await?;
    out.flush().await?;
    Ok(())
}
