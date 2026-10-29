//! `keypoints_for` against `crates/pose`'s stub worker: every branch of the pose cache. [M29 P22]
//!
//! The stub is `crates/pose/tests/stub_worker.py`, stdlib Python that speaks ADR-033 and poses
//! nothing, one mode per way a worker can behave. Three of its modes are this file's
//! (`short_decode`, `empty_clip`, `hashing`). So, as in `crates/pose`'s own tests, there is no
//! MediaPipe, no clip and no 40 seconds here, and these run in `cargo test` on any machine with a
//! Python. The real worker is `tests/keypoints.rs`' forced run, which needs `data/` and the model.
//!
//! Each case names the line of `api/pipeline.py::keypoints_for` it ports, and checks the log and the
//! notes as text, because both are Python's wording and a lab run must read the same.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use contracts::keypoints::{ClipMetadata, FrameKeypoints, KeypointsFile, Landmark};
use golf_lab::pose::{keypoints_for, KeypointsError, Sidecar};
use pose::{Config, PoolConfig};
use storage::keypoints_io::{keypoints_path, load_keypoints};
use storage::manifest::{Role, SwingManifest};

const SHA: &str = "21831919bc67aa3d1f8c4e6b0d2e5f7a9b1c3d5e7f9a1b3c5d7e9f1a3b5c7d9e";
const CLIP: &str = "face_on.21831919bc67.MOV";

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// One stub worker in `mode`, with clause 6's timeouts wound down to test length as `crates/pose`'s
/// tests wind them.
fn stub(mode: &str) -> PoolConfig {
    let script = repo().join("crates/pose/tests/stub_worker.py");
    PoolConfig {
        worker: Config {
            worker_args: vec![script.display().to_string(), mode.to_string()],
            accept_timeout: Duration::from_secs(5),
            work_floor: Duration::from_secs(5),
            fps_estimate: 1e9,
            ..Config::default()
        },
        size: 1,
        queue_bound: 4,
    }
}

fn on_stub(mode: &str) -> Sidecar {
    Sidecar::new("heavy", stub(mode))
}

fn manifest() -> SwingManifest {
    serde_json::from_value(serde_json::json!({
        "swing_id": "11", "session_id": "2026-08-23",
        "created_at": "2026-08-23T05:50:20.421608Z",
        "updated_at": "2026-08-23T05:50:29.716126Z",
        "roles": {"face_on": {
            "role": "face_on", "filename": CLIP, "content_sha256": SHA,
            "original_filename": "shot-11-front.MOV", "content_type": "video/quicktime",
            "size_bytes": 15, "received_at": "2026-08-23T05:50:25.978185Z", "warnings": []}}
    }))
    .unwrap()
}

/// A swing directory holding a stand-in clip, which only the `hashing` mode ever opens.
fn swing() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join(CLIP), b"not a real clip").unwrap();
    dir
}

/// A cached file for the face-on view, measured from `sha` by `estimator`.
fn cache(dir: &Path, sha: Option<&str>, estimator: Option<&str>) -> KeypointsFile {
    let landmark = Landmark {
        x: 0.125,
        y: 0.875,
        z: 0.0,
        visibility: 0.5,
    };
    let keypoints = KeypointsFile {
        clip: Some(ClipMetadata {
            fps: Some(59.96),
            width: Some(2160),
            height: Some(3840),
            frame_count: Some(3),
            source_sha256: sha.map(str::to_string),
        }),
        frames: (0..3)
            .map(|i| FrameKeypoints {
                frame_index: i,
                timestamp_ms: i as f64 * 16.7,
                landmarks: vec![landmark.clone()],
                camera_id: Some("face_on".to_string()),
            })
            .collect(),
        pose_estimator: estimator.map(str::to_string),
    };
    pose::writer::write(&keypoints, &keypoints_path(dir, Role::FaceOn)).unwrap();
    keypoints
}

struct Run {
    result: Result<Option<KeypointsFile>, KeypointsError>,
    log: Vec<String>,
    notes: Vec<String>,
}

fn run(dir: &Path, role: Role, force: bool, sidecar: &mut Sidecar) -> Run {
    let mut log = Vec::new();
    let mut notes = Vec::new();
    let result = keypoints_for(
        dir,
        &manifest(),
        role,
        force,
        sidecar,
        &mut |line: &str| log.push(line.to_string()),
        &mut notes,
    );
    Run { result, log, notes }
}

fn posed(run: Run) -> (KeypointsFile, Vec<String>, Vec<String>) {
    match run.result {
        Ok(Some(keypoints)) => (keypoints, run.log, run.notes),
        other => panic!("expected landmarks, got {other:?} {:?}", run.log),
    }
}

/// What the stub's two frames become once the lab stamps them: the manifest's sha, the configured
/// estimator, the decoded count.
fn assert_stamped(keypoints: &KeypointsFile, dir: &Path) {
    let clip = keypoints.clip.as_ref().unwrap();
    assert_eq!(clip.source_sha256.as_deref(), Some(SHA));
    assert_eq!(clip.frame_count, Some(2));
    assert_eq!(
        (clip.fps, clip.width, clip.height),
        (Some(60.0), Some(1080), Some(1920))
    );
    assert_eq!(keypoints.pose_estimator.as_deref(), Some("mediapipe:heavy"));
    assert!(keypoints
        .frames
        .iter()
        .all(|frame| frame.camera_id.as_deref() == Some("face_on")));
    let written = load_keypoints(&keypoints_path(dir, Role::FaceOn)).unwrap();
    assert_eq!(&written, keypoints, "the file written is the one returned");
}

#[test]
fn a_role_the_manifest_lacks_is_none_and_says_nothing() {
    let dir = swing();
    let mut sidecar = on_stub("ready");
    let run = run(dir.path(), Role::DownTheLine, false, &mut sidecar);
    assert!(matches!(run.result, Ok(None)));
    assert!(run.log.is_empty() && run.notes.is_empty());
    assert!(!sidecar.started());
}

#[test]
fn a_current_cache_is_read_and_no_worker_is_started() {
    let dir = swing();
    let cached = cache(dir.path(), Some(SHA), Some("mediapipe:heavy"));
    // A mode that would refuse to start, so a spawn could not pass unnoticed.
    let mut sidecar = on_stub("unavailable");
    let (read, log, notes) = posed(run(dir.path(), Role::FaceOn, false, &mut sidecar));
    assert_eq!(read, cached);
    assert_eq!(log, ["  face_on: 3 frames from cache"]);
    assert!(notes.is_empty());
    assert!(!sidecar.started());
}

#[test]
fn another_clips_cache_is_posed_again_and_replaced() {
    let dir = swing();
    cache(dir.path(), Some("e19f864d8635"), Some("mediapipe:heavy"));
    let mut sidecar = on_stub("ready");
    let (keypoints, log, notes) = posed(run(dir.path(), Role::FaceOn, false, &mut sidecar));
    assert_eq!(
        log,
        [
            "  face_on: cached keypoints are for a different clip, re-running pose",
            "  face_on: running pose over shot-11-front.MOV ...",
            "  face_on: 2 frames -> face_on.keypoints.json",
        ]
    );
    assert!(notes.is_empty(), "{notes:?}");
    assert_stamped(&keypoints, dir.path());
    // A legacy bare array records no clip, so it is another clip's too.
    fs::write(
        keypoints_path(dir.path(), Role::FaceOn),
        r#"[{"frame_index": 0, "timestamp_ms": 0.0, "landmarks": []}]"#,
    )
    .unwrap();
    let again = run(dir.path(), Role::FaceOn, false, &mut sidecar);
    assert_eq!(
        again.log[0],
        "  face_on: cached keypoints are for a different clip, re-running pose"
    );
}

#[test]
fn another_estimators_cache_is_posed_again_and_an_unrecorded_one_is_named() {
    for (estimator, said) in [
        (Some("mediapipe:lite"), "mediapipe:lite"),
        (None, "an unrecorded estimator"),
    ] {
        let dir = swing();
        cache(dir.path(), Some(SHA), estimator);
        let mut sidecar = on_stub("ready");
        let (keypoints, log, notes) = posed(run(dir.path(), Role::FaceOn, false, &mut sidecar));
        assert_eq!(
            log[0],
            format!("  face_on: cached pose is from {said}, re-running with mediapipe:heavy")
        );
        assert!(notes.is_empty(), "{notes:?}");
        assert_stamped(&keypoints, dir.path());
    }
}

#[test]
fn force_poses_over_a_current_cache() {
    let dir = swing();
    cache(dir.path(), Some(SHA), Some("mediapipe:heavy"));
    let mut sidecar = on_stub("ready");
    let (keypoints, log, _) = posed(run(dir.path(), Role::FaceOn, true, &mut sidecar));
    assert_eq!(log[0], "  face_on: running pose over shot-11-front.MOV ...");
    assert_stamped(&keypoints, dir.path());
    assert!(sidecar.started());
}

#[test]
fn a_superseded_run_is_kept_when_the_clip_is_gone_or_the_machine_cannot_pose() {
    // The clip is gone: no worker is asked at all.
    let dir = tempfile::tempdir().unwrap();
    let older = cache(dir.path(), Some(SHA), Some("mediapipe:lite"));
    let mut sidecar = on_stub("ready");
    let (kept, log, notes) = posed(run(dir.path(), Role::FaceOn, false, &mut sidecar));
    assert_eq!(kept, older);
    assert_eq!(
        log[1],
        format!("  face_on: {CLIP} is gone, keeping the older pose run")
    );
    assert_eq!(
        notes,
        [
            "the face_on landmarks were measured by mediapipe:lite rather than by the configured \
          mediapipe:heavy, and pose could not be re-run because the clip is not on disk"
        ]
    );
    assert!(!sidecar.started());

    // The clip is here and no worker will start: the handshake's refusal is the reason.
    let dir = swing();
    let older = cache(dir.path(), Some(SHA), Some("mediapipe:lite"));
    let mut sidecar = on_stub("unavailable");
    let (kept, log, notes) = posed(run(dir.path(), Role::FaceOn, false, &mut sidecar));
    assert_eq!(kept, older);
    let why = "the pose sidecar cannot pose (model_absent): no .task on disk";
    assert_eq!(
        log[1],
        format!("  face_on: cannot re-run pose ({why}), keeping the older run")
    );
    assert_eq!(
        notes,
        [format!(
            "the face_on landmarks were measured by mediapipe:lite rather than by the configured \
             mediapipe:heavy, and pose could not be re-run because {why}"
        )]
    );
    assert_eq!(
        load_keypoints(&keypoints_path(dir.path(), Role::FaceOn)).unwrap(),
        older,
        "nothing written"
    );
}

#[test]
fn with_nothing_to_keep_a_missing_clip_or_a_machine_that_cannot_pose_is_none_and_a_note() {
    let dir = tempfile::tempdir().unwrap();
    let mut sidecar = on_stub("ready");
    let run_missing = run(dir.path(), Role::FaceOn, false, &mut sidecar);
    assert!(matches!(run_missing.result, Ok(None)));
    assert_eq!(
        run_missing.log,
        [format!(
            "  face_on: {CLIP} is missing from the swing directory"
        )]
    );
    assert_eq!(
        run_missing.notes,
        ["the face_on clip is recorded in the manifest but missing from disk"]
    );
    assert!(!sidecar.started());

    let dir = swing();
    let mut sidecar = on_stub("unavailable");
    let why = "the pose sidecar cannot pose (model_absent): no .task on disk";
    for _ in 0..2 {
        // The second ask is answered from the remembered refusal, not a second handshake.
        let cannot = run(dir.path(), Role::FaceOn, false, &mut sidecar);
        assert!(matches!(cannot.result, Ok(None)));
        assert_eq!(cannot.log, [format!("  face_on: no pose — {why}")]);
        assert_eq!(cannot.notes, [format!("no pose for face_on: {why}")]);
    }
    assert!(!keypoints_path(dir.path(), Role::FaceOn).exists());
}

#[test]
fn a_clip_that_decodes_nothing_is_none_and_a_note() {
    let dir = swing();
    let mut sidecar = on_stub("empty_clip");
    let run = run(dir.path(), Role::FaceOn, false, &mut sidecar);
    assert!(matches!(run.result, Ok(None)));
    assert_eq!(
        run.log,
        [
            "  face_on: running pose over shot-11-front.MOV ...",
            "  face_on: no frames decoded",
        ]
    );
    assert_eq!(run.notes, ["the face_on clip decoded no frames at all"]);
    assert!(!keypoints_path(dir.path(), Role::FaceOn).exists());
}

#[test]
fn a_short_decode_is_kept_and_noted() {
    let dir = swing();
    let mut sidecar = on_stub("short_decode");
    let (keypoints, log, notes) = posed(run(dir.path(), Role::FaceOn, false, &mut sidecar));
    let message = "the face_on clip claims 3 frames but only 2 decoded — this analysis may be of \
                   an incomplete swing";
    assert_eq!(log[1], format!("  face_on: warning - {message}"));
    assert_eq!(notes, [message]);
    assert_stamped(&keypoints, dir.path());
}

#[test]
fn a_worker_hash_the_manifest_does_not_record_is_noted_and_the_manifests_is_stamped() {
    let dir = swing();
    let mut sidecar = on_stub("hashing");
    let (keypoints, log, notes) = posed(run(dir.path(), Role::FaceOn, false, &mut sidecar));
    // `hashlib.sha256(b"not a real clip")`, which the stub took off the stand-in on disk.
    let theirs = "0dd0baa0a17c52b579edc0a388157f0fd647ac1ee232e22e428f6f140a95b01b";
    let message = format!(
        "the face_on clip on disk hashes to {theirs}, not to the {SHA} its manifest records, so \
         these landmarks may not be of the clip this swing was uploaded with"
    );
    assert_eq!(notes, [message.clone()]);
    assert_eq!(log[1], format!("  face_on: warning - {message}"));
    assert_stamped(&keypoints, dir.path());
}

#[test]
fn a_failed_job_stops_the_bundle_and_writes_nothing() {
    for mode in ["fail_before_accept", "fail_after_accept"] {
        let dir = swing();
        let mut sidecar = on_stub(mode);
        let run = run(dir.path(), Role::FaceOn, false, &mut sidecar);
        match run.result {
            Err(KeypointsError::Pose { clip, detail }) => {
                assert!(clip.ends_with(CLIP), "{clip:?}");
                assert!(
                    detail.starts_with("ClipUnreadable — ") || detail.starts_with("PoseFailed — "),
                    "{mode}: {detail}"
                );
            }
            other => panic!("{mode}: expected a pose failure, got {other:?}"),
        }
        assert!(!keypoints_path(dir.path(), Role::FaceOn).exists());
    }
}

#[test]
fn landmarks_from_another_estimator_are_refused() {
    // The stub serves `heavy` whatever it is asked, so a `lite` sidecar's answer is not its own.
    let dir = swing();
    let mut sidecar = Sidecar::new("lite", stub("ready"));
    match run(dir.path(), Role::FaceOn, false, &mut sidecar).result {
        Err(KeypointsError::Pose { detail, .. }) => assert!(
            detail.contains("the pose sidecar") && detail.contains("mediapipe:lite"),
            "{detail}"
        ),
        other => panic!("expected a refusal, got {other:?}"),
    }
    assert!(!keypoints_path(dir.path(), Role::FaceOn).exists());
}

#[test]
fn a_broken_cache_stops_the_bundle() {
    let dir = swing();
    fs::write(keypoints_path(dir.path(), Role::FaceOn), "{\"frames\": [").unwrap();
    let mut sidecar = on_stub("ready");
    assert!(matches!(
        run(dir.path(), Role::FaceOn, false, &mut sidecar).result,
        Err(KeypointsError::Store(_))
    ));
    assert!(!sidecar.started());
}
