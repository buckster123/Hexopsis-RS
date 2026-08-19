//! S9: fixture child speaking `meshplane/1`. Uses `CARGO_BIN_EXE_meshplane-fixture`.

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;

use text2mesh::mock_glb::has_vertex_color;
use text2mesh::sidecar::{run_sidecar, SidecarCfg};
use text2mesh::types::{ComputeMode, JobSubmit, PlaneId, ProbeSnapshot, Quality};
use text2mesh::{error_type, App};

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_meshplane-fixture"))
}

fn cfg(args: &[&str]) -> SidecarCfg {
    SidecarCfg {
        wall: Duration::from_secs(30),
        handshake: Duration::from_secs(5),
        cancel_grace: Duration::from_millis(150),
        cancel: None,
        args: args.iter().map(|s| (*s).to_string()).collect(),
    }
}

fn job_tree() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let job_dir = dir.path().join("jobs").join("jid");
    std::fs::create_dir_all(job_dir.join("input")).unwrap();
    let cond = job_dir.join("input/conditioned.png");
    std::fs::write(&cond, text2mesh::types::minimal_png_1x1()).unwrap();
    (dir, job_dir, cond)
}

#[test]
fn fixture_writes_confined_glb() {
    let (_keep, job_dir, cond) = job_tree();
    let spec = JobSubmit {
        quality: Quality::Preview,
        seed: Some(1),
        ..JobSubmit::default()
    };
    let r = run_sidecar(&fixture(), "jid", &spec, &job_dir, &cond, cfg(&[])).unwrap();
    assert!(has_vertex_color(&r.glb));
    assert_eq!(r.engine, "fixture");
}

#[test]
fn crash_maps_engine_crash() {
    let (_keep, job_dir, cond) = job_tree();
    let err = run_sidecar(
        &fixture(),
        "jid",
        &JobSubmit::default(),
        &job_dir,
        &cond,
        cfg(&["crash"]),
    )
    .unwrap_err();
    assert_eq!(err.error_type, error_type::ENGINE_CRASH);
}

#[test]
fn bad_protocol_unsupported() {
    let (_keep, job_dir, cond) = job_tree();
    let err = run_sidecar(
        &fixture(),
        "jid",
        &JobSubmit::default(),
        &job_dir,
        &cond,
        cfg(&["bad-protocol"]),
    )
    .unwrap_err();
    assert_eq!(err.error_type, error_type::UNSUPPORTED);
}

#[test]
fn escape_path_engine_crash() {
    let (_keep, job_dir, cond) = job_tree();
    let err = run_sidecar(
        &fixture(),
        "jid",
        &JobSubmit::default(),
        &job_dir,
        &cond,
        cfg(&["escape"]),
    )
    .unwrap_err();
    assert_eq!(err.error_type, error_type::ENGINE_CRASH);
    let _ = std::fs::remove_file("/tmp/text2mesh-sidecar-escape.glb");
}

#[test]
fn mute_handshake_not_configured() {
    let (_keep, job_dir, cond) = job_tree();
    let mut c = cfg(&["mute"]);
    c.handshake = Duration::from_millis(250);
    let err =
        run_sidecar(&fixture(), "jid", &JobSubmit::default(), &job_dir, &cond, c).unwrap_err();
    assert_eq!(err.error_type, error_type::NOT_CONFIGURED);
}

#[test]
fn hang_cancel() {
    let (_keep, job_dir, cond) = job_tree();
    let mut c = cfg(&["hang"]);
    c.cancel = Some(Arc::new(AtomicBool::new(true)));
    let err =
        run_sidecar(&fixture(), "jid", &JobSubmit::default(), &job_dir, &cond, c).unwrap_err();
    assert_eq!(err.error_type, error_type::CANCELLED);
}

#[test]
fn director_image_through_fixture() {
    let bin = fixture();
    let app = App::for_test(false)
        .with_sidecar(bin)
        .with_probe(ProbeSnapshot {
            sidecar_alive: true,
            allow_mock: false,
            ..ProbeSnapshot::cpu_only(false)
        });
    let dir = tempfile::tempdir().unwrap();
    let png = dir.path().join("dot.png");
    std::fs::write(&png, text2mesh::types::minimal_png_1x1()).unwrap();
    let job = app
        .submit(JobSubmit {
            image_path: Some(png.to_string_lossy().into_owned()),
            compute: ComputeMode::Local,
            provider: Some(PlaneId::LocalSidecar),
            quality: Quality::Preview,
            ..JobSubmit::default()
        })
        .unwrap();
    assert_eq!(job.status, text2mesh::JobStatus::Degraded);
    assert_eq!(job.plane, Some(PlaneId::LocalSidecar));
    assert!(job.artifacts.glb.is_some());
    let man: text2mesh::types::Manifest = serde_json::from_str(
        &std::fs::read_to_string(job.artifacts.manifest.as_ref().unwrap()).unwrap(),
    )
    .unwrap();
    assert_eq!(man.engine.as_deref(), Some("fixture"));
    assert_eq!(man.sidecar_protocol.as_deref(), Some("meshplane/1"));
    assert!(!man.ok);
}
