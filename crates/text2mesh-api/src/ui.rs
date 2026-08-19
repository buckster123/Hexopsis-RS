//! Loopback HTMX studio — camera-ring groutbench (S11). No SPA.

use maud::{html, Markup, PreEscaped, DOCTYPE};
use text2mesh::system_check::build_system_check;
use text2mesh::{App, Estimate, JobStatus, MeshJob, SpendPolicy, SystemCheck};

pub fn page(app: &App, job: Option<&MeshJob>) -> Markup {
    let probe = probe_inner(app);
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { "Tessera" }
                style { (PreEscaped(CSS)) }
                script src="/static/htmx.min.js" {}
                script type="module" src="https://ajax.googleapis.com/ajax/libs/model-viewer/3.5.0/model-viewer.min.js" {}
            }
            body {
                header class="chrome" {
                    h1 { "Tessera" }
                    span class="muted" { "127.0.0.1:8796" }
                    (probe)
                }
                form #gen {
                    label class="tile drop" for="image" {
                        "Drop a still"
                        input #image type="file" name="image" accept="image/png,image/jpeg";
                    }
                    label for="prompt" { "Prompt" }
                    textarea #prompt name="prompt" rows="3" placeholder="a red fox wearing a yellow raincoat" {}
                    div class="row" {
                        label for="quality" { "Quality" }
                        select #quality name="quality" {
                            option value="preview" { "preview" }
                            option value="standard" selected { "standard" }
                            option value="high" { "high" }
                            option value="ultra" { "ultra" }
                        }
                        label for="compute" { "Compute" }
                        select #compute name="compute" {
                            option value="auto" selected { "auto" }
                            option value="local" { "local" }
                            option value="remote" { "remote" }
                        }
                        label for="provider" { "Plane" }
                        select #provider name="provider" {
                            option value="" selected { "planner" }
                            option value="local.mock" { "local.mock" }
                            option value="local.sidecar" { "local.sidecar" }
                            option value="remote.meshy" { "remote.meshy" }
                            option value="remote.tripo" { "remote.tripo" }
                        }
                    }
                    label class="check" {
                        input type="checkbox" name="allow_spend" value="1";
                        " allow_spend"
                    }
                    label class="check" {
                        input type="checkbox" name="allow_native_text" value="1";
                        " allow_native_text"
                    }
                    div class="row" {
                        button type="button"
                            hx-post="/ui/estimate"
                            hx-include="#gen"
                            hx-target="#estimate"
                            hx-swap="innerHTML" { "Estimate" }
                        button type="submit"
                            hx-post="/ui/jobs"
                            hx-encoding="multipart/form-data"
                            hx-target="#job"
                            hx-swap="outerHTML"
                            hx-disabled-elt="this" { "Generate" }
                    }
                }
                div #estimate class="tile muted" { "Estimate is free. Submit never guesses." }
                (job_card(job))
            }
        }
    }
}

pub fn probe_inner(app: &App) -> Markup {
    let sc = build_system_check(
        &app.probe(),
        &SpendPolicy {
            allow_spend: app.allow_spend,
            max_usd: 2.0,
        },
    );
    probe_markup(&sc)
}

pub fn probe_markup(sc: &SystemCheck) -> Markup {
    let pick = sc.planner.would_pick.map(|p| p.as_str()).unwrap_or("null");
    let degrade = sc
        .planner
        .degrade
        .as_ref()
        .map(|d| d.error_type.as_str())
        .unwrap_or("");
    let head = if sc.ready {
        format!("Ready · would_pick={pick}")
    } else if degrade.is_empty() {
        format!("Not ready · would_pick={pick}")
    } else {
        format!("Not ready · would_pick={pick} · {degrade}")
    };
    html! {
        div #probe class="probe"
            hx-get="/ui/system-check"
            hx-trigger="load, every 30s"
            hx-swap="outerHTML" {
            (head)
        }
    }
}

pub fn estimate_markup(est: &Estimate) -> Markup {
    let gate = &est.gate;
    let usd = if est.usd_uncertain {
        format!("~${:.2} (uncertain)", est.usd)
    } else {
        format!("${:.2}", est.usd)
    };
    let plane = est.plane.map(|p| p.as_str()).unwrap_or("none");
    html! {
        p {
            (usd) " · gate " (gate) " · " (plane)
            @if est.usd > 0.0 && gate == "closed" {
                " · will not POST"
            }
        }
    }
}

pub fn job_card(job: Option<&MeshJob>) -> Markup {
    match job {
        None => html! {
            section #job class="job tile" {
                p class="muted" { "No jobs. Estimate first — submit never guesses." }
            }
        },
        Some(job) => job_filled(job),
    }
}

fn job_filled(job: &MeshJob) -> Markup {
    let terminal = job.status.is_terminal();
    let cls = format!("job tile {}", status_class(job.status));
    let body = job_body(job, terminal);
    if !terminal {
        html! {
            section #job class=(cls)
                hx-get=(format!("/ui/jobs/{}", job.id))
                hx-trigger="every 2s"
                hx-swap="outerHTML" {
                (body)
            }
        }
    } else {
        html! {
            section #job class=(cls) { (body) }
        }
    }
}

fn job_body(job: &MeshJob, terminal: bool) -> Markup {
    let st = status_word(job.status);
    let live = if job.status == JobStatus::Failed {
        "assertive"
    } else {
        "polite"
    };
    html! {
        p role="status" aria-live=(live) { (headline(job)) }
        p class="mono muted" { (job.id) " · " (st) }
        (banner(job))
        (views(job))
        (preview(job))
        div class="row" {
            @if job.status == JobStatus::NeedsConfirm {
                button type="button"
                    hx-post=(format!("/ui/jobs/{}/confirm", job.id))
                    hx-target="#job"
                    hx-swap="outerHTML" { "Confirm spend" }
            }
            @if !terminal {
                button type="button"
                    hx-post=(format!("/ui/jobs/{}/cancel", job.id))
                    hx-target="#job"
                    hx-swap="outerHTML" { "Cancel" }
            }
            @if job.status.has_artifact() {
                @if job.status == JobStatus::Degraded {
                    a class="dl amber" href=(format!("/v1/jobs/{}/artifact?kind=glb", job.id)) download { "Download degraded GLB" }
                } @else {
                    a class="dl teal" href=(format!("/v1/jobs/{}/artifact?kind=glb", job.id)) download { "Download GLB" }
                }
            }
        }
    }
}

fn headline(job: &MeshJob) -> String {
    match job.status {
        JobStatus::Queued => "Queued".into(),
        JobStatus::NeedsConfirm => "Spend gate closed".into(),
        JobStatus::Submitted => "Submitted".into(),
        JobStatus::Running => format!("Running · {}", job.stage.as_deref().unwrap_or("—")),
        JobStatus::WaitingUpstream => "Waiting on vendor".into(),
        JobStatus::Succeeded => "Succeeded · GLB+PBR · ok=true".into(),
        JobStatus::Degraded => "Degraded — not a finished mesh".into(),
        JobStatus::Failed => format!(
            "Failed · {}",
            job.error
                .as_ref()
                .map(|e| e.error_type.as_str())
                .unwrap_or("error")
        ),
        JobStatus::Cancelled => "Cancelled".into(),
    }
}

fn banner(job: &MeshJob) -> Markup {
    if job.status != JobStatus::Degraded {
        return html! {};
    }
    let mut lines = Vec::new();
    if job.degrades.iter().any(|d| d == "export.material_mode") {
        lines.push("Degraded — vertex colour, not PBR. Valid GLB. Not a shipping material. status=degraded · ok=false.");
    }
    if job.degrades.iter().any(|d| d == "gate.encoder_missing") {
        lines
            .push("Degraded — ran ungated. CLIP G0–G2 skipped (encoder missing). G3–G4 still ran.");
    }
    let mock = job
        .artifacts
        .manifest
        .as_ref()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| {
            v.get("disclaimer")
                .and_then(|d| d.as_str())
                .map(str::to_string)
        });
    html! {
        div class="banner-degraded" {
            @for l in &lines { p { (l) } }
            @if mock.as_deref() == Some("not-a-model") {
                p { "Degraded — mock, not a model. engine=mock · COLOR_0 only. CI/Nano fixture. Never succeeded." }
            }
            @if lines.is_empty() && mock.is_none() {
                p { "Degraded — ok=false. Read degrades[] before you ship this." }
            }
        }
    }
}

fn views(job: &MeshJob) -> Markup {
    if job.artifacts.views.is_empty() {
        return html! {};
    }
    html! {
        div class="ring" aria-label="View Contract stills" {
            @for p in &job.artifacts.views {
                @let id = p.rsplit('/').next().unwrap_or("view").trim_end_matches(".png");
                img class="hex" src=(format!("/v1/jobs/{}/artifact?kind=view&view_id={id}", job.id))
                    alt=(id);
            }
        }
    }
}

fn preview(job: &MeshJob) -> Markup {
    if !job.status.has_artifact() {
        return html! {
            div class="well empty" { "Well empty until a GLB exists." }
        };
    }
    let amber = job.status == JobStatus::Degraded;
    html! {
        div class={"well " @if amber { "amber-rim" } @else { "teal-rim" }} {
            (PreEscaped(format!(
                r#"<model-viewer src="/v1/jobs/{}/artifact?kind=glb" camera-controls shadow-intensity="0" auto-rotate="false"></model-viewer>"#,
                job.id
            )))
        }
    }
}

fn status_word(s: JobStatus) -> &'static str {
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

fn status_class(s: JobStatus) -> &'static str {
    match s {
        JobStatus::Succeeded => "status-succeeded",
        JobStatus::Degraded | JobStatus::NeedsConfirm | JobStatus::WaitingUpstream => {
            "status-degraded"
        }
        JobStatus::Failed => "status-failed",
        _ => "status-quiet",
    }
}

const CSS: &str = r#"
:root {
  --bg: #141210; --surface: #1e1b18; --surface-2: #25211d;
  --ink: #efe7d8; --muted: #9a8f82;
  --amber: #e0a04a; --teal: #3bb8c8; --fail: #c45c4a; --grout: #2a2622;
  --radius-tile: 4px; --grout-gap: 2px;
  --font-ui: ui-sans-serif, system-ui, "Ubuntu", "Noto Sans", sans-serif;
  --font-mono: ui-monospace, "Ubuntu Mono", "Noto Sans Mono", monospace;
}
* { box-sizing: border-box; }
html, body { margin: 0; background: var(--bg); color: var(--ink); font: 15px/1.45 var(--font-ui); }
h1 { font-size: 0.95rem; letter-spacing: .28em; text-transform: uppercase; font-weight: 600; margin: 0; }
.chrome { display: flex; gap: 1rem; align-items: baseline; padding: 0.85rem 1rem; border-bottom: 1px solid var(--grout); }
.muted { color: var(--muted); }
.mono { font-family: var(--font-mono); font-size: 0.78rem; }
.tile { background: var(--surface); border-radius: var(--radius-tile); padding: 0.85rem 1rem; margin: 0.75rem 1rem; }
#gen { display: grid; gap: 0.5rem; margin: 1rem; }
.row { display: flex; flex-wrap: wrap; gap: 0.6rem; align-items: center; }
label { font-size: 0.75rem; letter-spacing: .08em; text-transform: uppercase; color: var(--muted); }
textarea, select, input[type=file] { background: var(--surface-2); color: var(--ink); border: 1px solid var(--grout); border-radius: var(--radius-tile); padding: 0.45rem; font: inherit; }
textarea { width: 100%; }
button, .dl { background: var(--grout); color: var(--ink); border: 0; border-radius: var(--radius-tile); padding: 0.45rem 0.8rem; cursor: pointer; text-decoration: none; font: inherit; }
button:focus-visible, a:focus-visible, select:focus-visible, textarea:focus-visible { outline: 2px solid var(--teal); }
.drop { display: block; padding: 1.2rem; border: 1px dashed var(--grout); }
.probe { font-size: 0.78rem; color: var(--muted); margin-left: auto; }
.status-degraded, .banner-degraded, .needs-confirm { color: var(--bg); background: var(--amber); }
.status-succeeded { color: var(--bg); background: var(--teal); }
.status-failed { color: var(--ink); background: var(--fail); }
.banner-degraded { padding: 0.6rem 0.8rem; border-radius: var(--radius-tile); margin: 0.5rem 0; }
.banner-degraded p { margin: 0.2rem 0; }
.ring { display: flex; flex-wrap: wrap; gap: 2px; background: var(--grout); padding: 2px; margin: 0.75rem 0; justify-content: center; }
.hex { width: 72px; height: 80px; object-fit: cover; clip-path: polygon(50% 0%, 100% 25%, 100% 75%, 50% 100%, 0% 75%, 0% 25%); background: var(--surface); }
.well { min-height: 220px; background: var(--surface-2); border: 2px solid var(--grout); border-radius: 8px; display: grid; place-items: center; }
.well.amber-rim { border-color: var(--amber); }
.well.teal-rim { border-color: var(--teal); }
.well.empty { color: var(--muted); font-size: 0.85rem; }
model-viewer { width: 100%; height: 240px; background: #0e0c0a; }
.dl.amber { background: var(--amber); color: var(--bg); }
.dl.teal { background: var(--teal); color: var(--bg); }
.check { text-transform: none; letter-spacing: 0; color: var(--ink); }
@media (prefers-reduced-motion: reduce) {
  .ring { flex-wrap: nowrap; overflow-x: auto; }
}
"#;
