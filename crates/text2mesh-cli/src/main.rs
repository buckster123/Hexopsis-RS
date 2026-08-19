//! text2mesh CLI — human/ops face. `--json` on every command.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand};
use serde_json::{json, Value};
use text2mesh::error::error_type;
use text2mesh::{
    compile_view_contract, load_xdg_env, App, ArtifactKind, CompileOpts, ComputeMode, DeviceKind,
    Error, JobStatus, JobSubmit, PlaneId, Quality, Route, SystemCheck, T2iProviderId,
    WAIT_DEFAULT_S, WAIT_MAX_S, WAIT_MIN_S,
};

#[derive(Parser)]
#[command(
    name = "text2mesh",
    version,
    about = "Tessera-RS — image or text → honest GLB"
)]
struct Cli {
    #[arg(long, global = true)]
    json: bool,
    #[arg(long, global = true)]
    store: Option<PathBuf>,
    #[arg(long, global = true)]
    allow_spend: bool,
    #[command(subcommand)]
    cmd: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Honesty probe (report_complete + ready; no ok-for-readiness).
    #[command(name = "system-check")]
    SystemCheck {
        #[arg(long)]
        refresh: bool,
    },
    /// Free cost/time estimate. Never paid.
    Estimate {
        #[command(flatten)]
        job: JobArgs,
    },
    /// Submit + wait. Exit 0 only if succeeded; 1 if degraded.
    Generate {
        #[command(flatten)]
        job: JobArgs,
        #[arg(long, default_value_t = WAIT_DEFAULT_S)]
        timeout_s: u64,
    },
    /// Open the spend gate on a needs_confirm job.
    Confirm {
        job: String,
    },
    Status {
        job: String,
    },
    Wait {
        job: String,
        #[arg(long, default_value_t = WAIT_DEFAULT_S)]
        timeout_s: u64,
    },
    Cancel {
        job: String,
    },
    Artifact {
        job: String,
        #[arg(long, default_value = "glb")]
        kind: String,
    },
    /// View Contract compile (pure, no T2I).
    Compile {
        #[arg(long)]
        prompt: String,
        #[arg(long)]
        out: Option<PathBuf>,
        #[arg(long, default_value = "standard")]
        quality: String,
        #[arg(long)]
        seed: Option<u64>,
    },
    Jobs {
        #[arg(long)]
        status: Option<String>,
        #[arg(long, default_value_t = 20)]
        limit: u32,
    },
    /// Bind HTTP API (127.0.0.1:8796).
    Serve,
    /// Point .mcp.json at text2mesh-mcp; this subcommand only explains that.
    Mcp,
    /// Catalog weights. CLI only — never exposed on MCP.
    Weights {
        #[command(subcommand)]
        cmd: WeightsCmd,
    },
}

#[derive(Subcommand)]
enum WeightsCmd {
    /// Fetch or stamp a catalog id after accepting its license. Never auto-runs on generate.
    Pull {
        id: String,
        #[arg(long = "accept-license", required = true)]
        accept_license: String,
    },
}

#[derive(Args, Clone)]
struct JobArgs {
    #[arg(long)]
    prompt: Option<String>,
    #[arg(long, visible_alias = "image")]
    image: Option<PathBuf>,
    #[arg(long, default_value = "auto")]
    route: String,
    #[arg(long, default_value = "standard")]
    quality: String,
    #[arg(long, default_value = "auto")]
    compute: String,
    #[arg(long)]
    provider: Option<String>,
    #[arg(long)]
    prefer_device: Option<String>,
    #[arg(long)]
    seed: Option<u64>,
    #[arg(long)]
    preset: Option<String>,
    #[arg(long)]
    allow_neural_cad: bool,
    #[arg(long)]
    allow_native_text: bool,
    #[arg(long)]
    license_override: Option<String>,
    #[arg(long, default_value_t = 2.0)]
    max_usd: f64,
    #[arg(long)]
    max_credits: Option<u64>,
    #[arg(long, default_value_t = WAIT_DEFAULT_S)]
    max_wall_s: u64,
    #[arg(long)]
    idempotency_key: Option<String>,
    #[arg(long)]
    keep_largest: bool,
    #[arg(long)]
    force_opaque: bool,
    #[arg(long)]
    unit_cube: bool,
    #[arg(long)]
    uv_atlas: bool,
    #[arg(long)]
    print_wrap: bool,
}

#[tokio::main]
async fn main() -> ExitCode {
    load_xdg_env();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("text2mesh=info")),
        )
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();
    match run(cli).await {
        Ok(code) => code,
        Err(e) => {
            eprintln!("{}: {}", e.error_type, e.message);
            ExitCode::from(exit_for_error(&e))
        }
    }
}

async fn run(cli: Cli) -> Result<ExitCode, Error> {
    if let Some(root) = &cli.store {
        // SAFETY: before App/threads; CLI flag overrides store root.
        unsafe { std::env::set_var("TEXT2MESH_STORE", root) };
    }
    if cli.allow_spend {
        unsafe { std::env::set_var("TEXT2MESH_ALLOW_SPEND", "1") };
    }

    match cli.cmd {
        Command::Mcp => {
            eprintln!("run the agent face: text2mesh-mcp (stdio). Point .mcp.json at that binary.");
            if cli.json {
                println!(
                    "{}",
                    json!({
                        "ok": false,
                        "error_type": "not_configured",
                        "message": "use text2mesh-mcp for stdio MCP",
                        "hint": "argv[0] of the MCP server is text2mesh-mcp, not `text2mesh mcp`"
                    })
                );
            }
            return Ok(ExitCode::from(3));
        }
        Command::Serve => {
            return text2mesh_api::run_from_cli()
                .await
                .map(|_| ExitCode::SUCCESS)
                .map_err(|e| Error::new(error_type::INTERNAL, e.to_string()));
        }
        Command::Weights { cmd } => {
            return run_weights(cli.json, cmd);
        }
        Command::SystemCheck { refresh: _ } => {
            let sc = text2mesh::system_check::system_check_from_env(text2mesh::config::env_truthy(
                "TEXT2MESH_ALLOW_MOCK",
            ));
            emit(cli.json, serde_json::to_value(&sc)?);
            if !cli.json {
                print_system_check(&sc);
            }
            return Ok(ExitCode::SUCCESS);
        }
        Command::Compile {
            prompt,
            out,
            quality,
            seed,
        } => {
            let q = parse_quality(&quality)?;
            let contract = compile_view_contract(
                &prompt,
                CompileOpts {
                    quality: q,
                    camera_preset: None,
                    family_seed: seed.unwrap_or(42),
                    t2i_provider: T2iProviderId::Mock,
                },
            )?;
            let v = serde_json::to_value(&contract)
                .map_err(|e| Error::new(error_type::INTERNAL, e.to_string()))?;
            if let Some(path) = out {
                std::fs::write(&path, serde_json::to_vec_pretty(&contract)?)?;
            }
            emit(cli.json, v);
            if !cli.json {
                println!("{}", serde_json::to_string_pretty(&contract)?);
            }
            return Ok(ExitCode::SUCCESS);
        }
        _ => {}
    }

    let mut app = App::from_env()?;
    if cli.allow_spend {
        app.allow_spend = true;
    }

    match cli.cmd {
        Command::Estimate { job } => {
            let spec = job_submit(&job, cli.allow_spend)?;
            let est = app.estimate(&spec);
            emit(cli.json, serde_json::to_value(&est)?);
            if est.ok {
                Ok(ExitCode::SUCCESS)
            } else {
                Ok(ExitCode::from(3))
            }
        }
        Command::Generate { job, timeout_s } => {
            check_timeout(timeout_s)?;
            let spec = job_submit(&job, cli.allow_spend)?;
            let submitted = app.submit(spec)?;
            let wait = app.wait(&submitted.id, timeout_s)?;
            emit_job(cli.json, &wait.job);
            if wait.job.status == JobStatus::Degraded {
                eprintln!("DEGRADED");
            }
            Ok(ExitCode::from(exit_for_job(&wait.job, wait.wait_timed_out)))
        }
        Command::Confirm { job } => {
            let j = app.confirm(&job)?;
            emit_job(cli.json, &j);
            Ok(ExitCode::from(exit_for_job(&j, false)))
        }
        Command::Status { job } => {
            let j = app.status(&job)?;
            emit_job(cli.json, &j);
            Ok(ExitCode::from(exit_for_job(&j, false)))
        }
        Command::Wait { job, timeout_s } => {
            check_timeout(timeout_s)?;
            let wait = app.wait(&job, timeout_s)?;
            let mut v = serde_json::to_value(&wait.job)?;
            if let Some(obj) = v.as_object_mut() {
                obj.insert("wait_timed_out".into(), json!(wait.wait_timed_out));
                obj.insert("ok".into(), json!(wait.ok));
            }
            emit(cli.json, v);
            if wait.job.status == JobStatus::Degraded {
                eprintln!("DEGRADED");
            }
            Ok(ExitCode::from(exit_for_job(&wait.job, wait.wait_timed_out)))
        }
        Command::Cancel { job } => {
            let j = app.cancel(&job)?;
            emit_job(cli.json, &j);
            Ok(ExitCode::from(exit_for_job(&j, false)))
        }
        Command::Artifact { job, kind } => {
            let kind = ArtifactKind::parse(&kind).ok_or_else(|| {
                Error::new(error_type::SPEC_REJECTED, format!("unknown kind {kind}"))
            })?;
            let (path, sha, bytes, media) = app.artifact(&job, kind)?;
            emit(
                cli.json,
                json!({
                    "ok": true,
                    "path": path,
                    "sha256": sha,
                    "bytes": bytes,
                    "media_type": media
                }),
            );
            if !cli.json {
                println!("{}", path.display());
            }
            Ok(ExitCode::SUCCESS)
        }
        Command::Jobs { status, limit } => {
            let st = match status.as_deref() {
                Some(s) => Some(parse_status(s)?),
                None => None,
            };
            let jobs = app.list(st, limit)?;
            emit(cli.json, json!({ "ok": true, "jobs": jobs }));
            if !cli.json {
                for j in &jobs {
                    println!("{} {}", j.id, status_label(j.status));
                }
            }
            Ok(ExitCode::SUCCESS)
        }
        Command::Serve
        | Command::Mcp
        | Command::Compile { .. }
        | Command::Weights { .. }
        | Command::SystemCheck { .. } => unreachable!(),
    }
}

fn run_weights(as_json: bool, cmd: WeightsCmd) -> Result<ExitCode, Error> {
    match cmd {
        WeightsCmd::Pull { id, accept_license } => {
            let row = text2mesh::weights::pull(&id, &accept_license)?;
            emit(as_json, serde_json::to_value(&row)?);
            if !as_json {
                eprintln!(
                    "weights {} present={} accepted={} path={}",
                    row.id,
                    row.present,
                    row.accepted,
                    row.path.as_deref().unwrap_or("-")
                );
                if !row.present {
                    eprintln!(
                        "license recorded; place files at the path (no auto-fetch; set TEXT2MESH_WEIGHTS_SRC to copy a local file)"
                    );
                }
            }
            Ok(ExitCode::SUCCESS)
        }
    }
}

fn job_submit(args: &JobArgs, allow_spend: bool) -> Result<JobSubmit, Error> {
    Ok(JobSubmit {
        prompt: args.prompt.clone(),
        image_path: args
            .image
            .as_ref()
            .map(|p| p.to_string_lossy().into_owned()),
        route: parse_route(&args.route)?,
        quality: parse_quality(&args.quality)?,
        compute: parse_compute(&args.compute)?,
        provider: match args.provider.as_deref() {
            Some(s) => Some(parse_plane(s)?),
            None => None,
        },
        prefer_device: match args.prefer_device.as_deref() {
            Some(s) => Some(parse_device(s)?),
            None => None,
        },
        seed: args.seed,
        camera_preset: None,
        allow_spend,
        allow_neural_cad: args.allow_neural_cad,
        allow_native_text: args.allow_native_text,
        license_override: args.license_override.clone(),
        max_usd: args.max_usd,
        max_credits: args.max_credits,
        max_wall_s: args.max_wall_s,
        idempotency_key: args.idempotency_key.clone(),
        export: text2mesh::ExportFlags {
            keep_largest_component: args.keep_largest,
            force_opaque: args.force_opaque,
            unit_cube: args.unit_cube,
            uv_atlas: args.uv_atlas,
            print_wrap: args.print_wrap,
        },
        job_id: None,
    })
}

fn parse_route(s: &str) -> Result<Route, Error> {
    match s {
        "auto" => Ok(Route::Auto),
        "analytic" => Ok(Route::Analytic),
        "view_contract" => Ok(Route::ViewContract),
        "native" => Ok(Route::Native),
        other => Err(Error::new(
            error_type::SPEC_REJECTED,
            format!("route {other}"),
        )),
    }
}

fn parse_quality(s: &str) -> Result<Quality, Error> {
    match s {
        "preview" => Ok(Quality::Preview),
        "standard" => Ok(Quality::Standard),
        "high" => Ok(Quality::High),
        "ultra" => Ok(Quality::Ultra),
        other => Err(Error::new(
            error_type::SPEC_REJECTED,
            format!("quality {other}"),
        )),
    }
}

fn parse_compute(s: &str) -> Result<ComputeMode, Error> {
    match s {
        "auto" => Ok(ComputeMode::Auto),
        "local" => Ok(ComputeMode::Local),
        "remote" => Ok(ComputeMode::Remote),
        other => Err(Error::new(
            error_type::SPEC_REJECTED,
            format!("compute {other}"),
        )),
    }
}

fn parse_plane(s: &str) -> Result<PlaneId, Error> {
    PlaneId::parse(s).ok_or_else(|| Error::new(error_type::SPEC_REJECTED, format!("plane {s}")))
}

fn parse_device(s: &str) -> Result<DeviceKind, Error> {
    DeviceKind::parse(s).ok_or_else(|| Error::new(error_type::SPEC_REJECTED, format!("device {s}")))
}

fn parse_status(s: &str) -> Result<JobStatus, Error> {
    match s {
        "queued" => Ok(JobStatus::Queued),
        "needs_confirm" => Ok(JobStatus::NeedsConfirm),
        "submitted" => Ok(JobStatus::Submitted),
        "running" => Ok(JobStatus::Running),
        "waiting_upstream" => Ok(JobStatus::WaitingUpstream),
        "succeeded" => Ok(JobStatus::Succeeded),
        "degraded" => Ok(JobStatus::Degraded),
        "failed" => Ok(JobStatus::Failed),
        "cancelled" => Ok(JobStatus::Cancelled),
        other => Err(Error::new(
            error_type::SPEC_REJECTED,
            format!("status {other}"),
        )),
    }
}

fn check_timeout(timeout_s: u64) -> Result<(), Error> {
    if (WAIT_MIN_S..=WAIT_MAX_S).contains(&timeout_s) {
        Ok(())
    } else {
        Err(Error::new(
            error_type::SPEC_REJECTED,
            format!("timeout_s {timeout_s} outside {WAIT_MIN_S}..={WAIT_MAX_S}"),
        ))
    }
}

fn emit(as_json: bool, v: Value) {
    if as_json {
        println!(
            "{}",
            serde_json::to_string_pretty(&v).unwrap_or_else(|_| v.to_string())
        );
    }
}

fn emit_job(as_json: bool, job: &text2mesh::MeshJob) {
    if as_json {
        emit(true, serde_json::to_value(job).unwrap_or(json!({})));
    } else {
        println!("{} {}", job.id, status_label(job.status));
        if !job.degrades.is_empty() {
            eprintln!("degrades: {}", job.degrades.join(", "));
        }
        if let Some(err) = &job.error {
            eprintln!("{}: {}", err.error_type, err.message);
        }
    }
}

fn print_system_check(sc: &SystemCheck) {
    println!(
        "text2mesh {} report_complete={} ready={} tier={}",
        sc.version,
        sc.report_complete,
        sc.ready,
        sc.tier.as_deref().unwrap_or("-")
    );
    for d in &sc.devices {
        if d.ok {
            println!(
                "  {} ok vram_mb={:?} shared={} name={}",
                d.kind.as_str(),
                d.vram_mb,
                d.shared,
                d.name.as_deref().unwrap_or("-")
            );
        }
    }
    if let Some(p) = sc.planner.would_pick {
        println!("would_pick={}", p.as_str());
    } else if let Some(d) = &sc.planner.degrade {
        println!("would_pick=null ({})", d.error_type);
    }
}

fn status_label(s: JobStatus) -> &'static str {
    match s {
        JobStatus::Queued => "queued",
        JobStatus::NeedsConfirm => "needs_confirm",
        JobStatus::Submitted => "submitted",
        JobStatus::Running => "running",
        JobStatus::WaitingUpstream => "waiting_upstream",
        JobStatus::Succeeded => "succeeded",
        JobStatus::Degraded => "degraded",
        JobStatus::Failed => "failed",
        JobStatus::Cancelled => "cancelled",
    }
}

fn exit_for_job(job: &text2mesh::MeshJob, wait_timed_out: bool) -> u8 {
    if wait_timed_out {
        return 8;
    }
    match job.status {
        JobStatus::Succeeded => 0,
        JobStatus::Degraded => 1,
        JobStatus::Cancelled => 7,
        JobStatus::NeedsConfirm => 4,
        JobStatus::Failed => job
            .error
            .as_ref()
            .map(|e| exit_for_type(&e.error_type))
            .unwrap_or(5),
        _ => 9,
    }
}

fn exit_for_error(err: &Error) -> u8 {
    if err.error_type == error_type::SPEC_REJECTED {
        2
    } else {
        exit_for_type(&err.error_type)
    }
}

fn exit_for_type(t: &str) -> u8 {
    if matches!(
        t,
        "not_configured"
            | "weights_missing"
            | "feature_off"
            | "disk_short"
            | "vram_short"
            | "device_missing"
    ) {
        3
    } else if t.starts_with("spend.") || t.starts_with("license.") {
        4
    } else if t.starts_with("engine.") || t == "wait.timeout" {
        5
    } else if t.starts_with("view.") || t.starts_with("analytic.") {
        6
    } else if t == "cancelled" {
        7
    } else {
        9
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn wait_default_1800() {
        let cli = Cli::parse_from(["text2mesh", "wait", "01ABC"]);
        match cli.cmd {
            Command::Wait { timeout_s, .. } => assert_eq!(timeout_s, 1800),
            _ => panic!("expected wait"),
        }
    }

    #[test]
    fn weights_pull_requires_license() {
        let cli = Cli::try_parse_from(["text2mesh", "weights", "pull", "quality.stack"]);
        assert!(cli.is_err());
        let cli = Cli::parse_from([
            "text2mesh",
            "weights",
            "pull",
            "encoder.dinov3_vitl16",
            "--accept-license",
            "dinov3",
        ]);
        match cli.cmd {
            Command::Weights {
                cmd: WeightsCmd::Pull { id, accept_license },
            } => {
                assert_eq!(id, "encoder.dinov3_vitl16");
                assert_eq!(accept_license, "dinov3");
            }
            _ => panic!("expected weights pull"),
        }
    }

    #[test]
    fn generate_timeout_default_1800() {
        let cli = Cli::parse_from(["text2mesh", "generate", "--prompt", "fox"]);
        match cli.cmd {
            Command::Generate { timeout_s, job } => {
                assert_eq!(timeout_s, 1800);
                assert_eq!(job.max_wall_s, 1800);
            }
            _ => panic!("expected generate"),
        }
    }
}
