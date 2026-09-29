//! `Worker` against a stub that speaks ADR-033 and never poses anything. [M23 P4]
//!
//! The gate is `tests/stub_worker.py`: a few dozen lines of stdlib Python, one mode per way a child
//! process can behave. **No MediaPipe, no 4K clip and no 40 seconds anywhere in `cargo test`** —
//! which is the same move `crates/trigger`'s `testing::bay` made for audio, for the same reason. A
//! crash mid-job, a hang, a garbage line and a reply about somebody else's job cannot be provoked in
//! the real worker without breaking it, and they are exactly the cases `Worker` exists to survive.
//!
//! The one test that runs the *real* worker is behind `GOLF_POSE_REAL_WORKER` and skipped by
//! default. It is ~40 s, it needs the `vision` extra and the `heavy` bundle, and **P6 is where the
//! real path is gated properly** — against a committed keypoints file rather than a frame count.

use std::path::{Path, PathBuf};
use std::time::Duration;

use pose::protocol::{FailureReason, Job};
use pose::worker::{interpreter_candidates, Config, Outcome, Worker};
use pose::PoseError;

/// The checkout this test is compiled inside. `CARGO_MANIFEST_DIR` is the build machine's path and
/// `worker::repo_root` deliberately does not use it — but a test binary only ever runs where it was
/// built, and a test that searched would be testing the search.
fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crates/pose has a grandparent")
        .to_path_buf()
}

/// A config that runs the stub in one mode, with clause 6's timeouts wound down to test length.
///
/// `fps_estimate` is absurd on purpose: it drives the derived half of the work deadline to nothing
/// so that `work_floor` is what every hang test actually waits out, which keeps the suite's slowest
/// path a bounded second rather than the 30 s a real floor would cost.
fn stub(mode: &str) -> Config {
    let script = repo()
        .join("crates")
        .join("pose")
        .join("tests")
        .join("stub_worker.py");
    Config {
        worker_args: vec![script.display().to_string(), mode.to_string()],
        accept_timeout: Duration::from_secs(1),
        work_floor: Duration::from_secs(1),
        fps_estimate: 1e9,
        ..Config::default()
    }
}

fn job(id: &str) -> Job {
    Job {
        job_id: id.to_string(),
        clip_path: PathBuf::from("/clips/face_on.MOV"),
        frame_range: None,
        camera_id: Some("face_on".to_string()),
        pose_model_variant: None,
    }
}

fn spawn(mode: &str) -> Worker {
    Worker::spawn(&stub(mode)).unwrap_or_else(|e| panic!("spawning the {mode} stub: {e}"))
}

#[test]
fn a_worker_handshakes_and_then_answers_one_job() {
    let mut worker = spawn("ready");
    assert_eq!(worker.estimator(), "mediapipe:heavy");
    assert_eq!(worker.variant(), "heavy");
    assert!(worker.model().to_string_lossy().ends_with(".task"));

    match worker.run(&job("j-1")).expect("the job ran") {
        Outcome::Done(keypoints) => {
            assert_eq!(keypoints.frames.len(), 2);
            assert_eq!(keypoints.pose_estimator.as_deref(), Some("mediapipe:heavy"));
            // Clause 8: the worker returned landmarks and decided no path. Nothing was written.
            assert_eq!(keypoints.clip.and_then(|clip| clip.frame_count), Some(2));
        }
        other => panic!("expected landmarks, got {other:?}"),
    }
}

#[test]
fn the_process_is_warm_across_jobs_and_each_reply_is_attributed_to_its_own() {
    // ADR-030 §3's claim, checked rather than assumed: the second job pays no startup. What a
    // long-lived process buys is the interpreter and the imports — the landmarker is per job.
    let mut worker = spawn("ready");
    for id in ["j-1", "j-2", "j-3"] {
        match worker.run(&job(id)).expect("the job ran") {
            Outcome::Done(keypoints) => assert_eq!(keypoints.frames.len(), 2),
            other => panic!("{id}: expected landmarks, got {other:?}"),
        }
    }
}

#[test]
fn a_worker_that_dies_mid_job_is_a_crash_carrying_the_stderr_that_explains_it() {
    let mut worker = spawn("crash_after_accept");
    match worker.run(&job("j-1")) {
        Err(err @ PoseError::Crashed { .. }) => {
            let message = err.to_string();
            // The whole point of keeping a tail: the traceback is the diagnosis, and an exit code
            // on its own is a shrug.
            assert!(message.contains("the graph closed"), "{message}");
            assert!(message.contains("exit code 3"), "{message}");
            // Clause 5's crash row — a transient fault in a long-lived C++ graph.
            assert!(err.retried());
        }
        other => panic!("expected a crash, got {other:?}"),
    }
}

#[test]
fn a_worker_that_dies_before_acknowledging_is_still_a_crash_and_not_a_timeout() {
    // It never acked, so the flat accept timeout is what was running — but EOF arrives first and
    // says more, and reporting a timeout for a process that is already gone would be a lie.
    let mut worker = spawn("crash_before_accept");
    match worker.run(&job("j-1")) {
        Err(PoseError::Crashed { detail, .. }) => {
            assert!(detail.contains("exit code 3"), "{detail}");
        }
        other => panic!("expected a crash, got {other:?}"),
    }
}

#[test]
fn a_line_that_is_not_a_message_is_a_protocol_violation_rather_than_a_hang() {
    // stdout is protocol and stderr is logging (clause 1), so a dependency that printed has
    // corrupted the channel. The worker is killed with it, and *what the line said* is the report.
    let mut worker = spawn("garbage");
    match worker.run(&job("j-1")) {
        Err(err @ PoseError::Protocol(_)) => {
            assert!(err.to_string().contains("not a protocol message"), "{err}");
            assert!(err.retried(), "the process might be fixed by a fresh one");
        }
        other => panic!("expected a protocol violation, got {other:?}"),
    }
}

#[test]
fn a_worker_that_never_acknowledges_hits_the_flat_accept_timeout() {
    let mut worker = spawn("hang_accept");
    match worker.run(&job("j-1")) {
        Err(PoseError::AcceptTimeout { after }) => assert_eq!(after, Duration::from_secs(1)),
        other => panic!("expected an accept timeout, got {other:?}"),
    }
}

#[test]
fn a_worker_that_acknowledges_and_then_hangs_hits_the_derived_work_deadline() {
    let mut worker = spawn("hang_work");
    match worker.run(&job("j-1")) {
        // The two timeouts are distinguishable, which is the reason clause 4 has an acceptance at
        // all: this one says "too slow for the work it took on", where the other says "wedged".
        Err(err @ PoseError::WorkTimeout { .. }) => {
            let PoseError::WorkTimeout { after, frames } = &err else {
                unreachable!()
            };
            assert_eq!(*after, Duration::from_secs(1));
            // Derived from the count the acceptance carried, and the message says so.
            assert_eq!(*frames, 2);
            assert!(err.to_string().contains("posing 2 frames"), "{err}");
        }
        other => panic!("expected a work timeout, got {other:?}"),
    }
}

#[test]
fn a_model_absent_handshake_is_a_pool_level_failure_and_never_a_failed_job() {
    // Clause 3: the machine says it cannot pose when the session starts, not thirty seconds into
    // the golfer's first swing. And it stops the interpreter search — a worker spoke and said no.
    match Worker::spawn(&stub("unavailable")) {
        Err(err @ PoseError::Unavailable { .. }) => {
            assert!(err.to_string().contains("model_absent"), "{err}");
            assert!(!err.retried(), "no number of fresh workers grows a model");
        }
        other => panic!("expected an unavailable handshake, got {:?}", other.err()),
    }
}

#[test]
fn a_protocol_this_build_does_not_speak_is_refused_on_the_handshake_line() {
    // Not discovered as a missing field eight minutes into a clip, which is the whole reason
    // clause 3 puts a version on the first line.
    match Worker::spawn(&stub("bad_protocol")) {
        Err(PoseError::ProtocolMismatch { theirs, ours }) => {
            assert_eq!(theirs, 99);
            assert_eq!(ours, pose::PROTOCOL_VERSION);
        }
        other => panic!("expected a protocol mismatch, got {:?}", other.err()),
    }
}

#[test]
fn a_failure_with_no_acceptance_before_it_is_legal_and_not_a_violation() {
    // P3's finding: `clip_unreadable`, `bad_job` and `frame_range_unsupported` are all decided
    // before or instead of opening a container, so `failed` is a legal first reply.
    let mut worker = spawn("fail_before_accept");
    match worker
        .run(&job("j-1"))
        .expect("a named failure is an outcome, not an error")
    {
        Outcome::Failed { reason, detail } => {
            assert_eq!(reason, FailureReason::ClipUnreadable);
            assert_eq!(detail, "no such file");
            assert!(!reason.retried(), "the second read finds the same file");
        }
        other => panic!("expected a named failure, got {other:?}"),
    }
}

#[test]
fn a_pose_failure_after_the_acceptance_is_an_outcome_and_is_the_one_reason_worth_retrying() {
    // The other side of the previous test. `pose_failed` is an exception out of a long-lived C++
    // graph, which is the one row in clause 5's worker half a fresh process can fix — and it is
    // still an `Ok`, because the *sidecar* did not fail. Only the pool decides what to do next.
    let mut worker = spawn("fail_after_accept");
    match worker
        .run(&job("j-1"))
        .expect("a named failure is an outcome")
    {
        Outcome::Failed { reason, detail } => {
            assert_eq!(reason, FailureReason::PoseFailed);
            assert!(detail.contains("the graph closed"), "{detail}");
            assert!(reason.retried());
        }
        other => panic!("expected a pose failure, got {other:?}"),
    }
}

#[test]
fn a_candidate_that_prints_before_handshaking_is_passed_over_rather_than_believed() {
    // Distinct from a candidate that *dies*: this one is alive and talking, and still not our
    // worker. Clause 1 gives stdout to the protocol, so a first line of human text is the same
    // evidence either way — and either way the search moves on and keeps what it saw.
    match Worker::spawn(&stub("chatty")) {
        Err(err @ PoseError::Spawn { .. }) => {
            let message = err.to_string();
            assert!(message.contains("not a protocol message"), "{message}");
            assert!(message.contains("loading the graph"), "{message}");
        }
        other => panic!("expected a spawn failure, got {:?}", other.err()),
    }
}

#[test]
fn a_bad_job_with_no_id_to_echo_is_attributed_to_the_job_in_flight() {
    // A worker owns one job at a time, so an unattributed failure is still attributable. Refusing
    // it would turn a readable failure back into the silent one it was written to replace.
    let mut worker = spawn("bad_job_unattributed");
    match worker
        .run(&job("j-1"))
        .expect("attributed to the job in flight")
    {
        Outcome::Failed { reason, .. } => assert_eq!(reason, FailureReason::BadJob),
        other => panic!("expected a bad_job failure, got {other:?}"),
    }
}

#[test]
fn a_reply_about_another_job_is_refused_rather_than_delivered_to_this_one() {
    // The kind of thing that only breaks under concurrency, which is why `job_id` is echoed at all.
    let mut worker = spawn("wrong_job");
    match worker.run(&job("j-1")) {
        Err(err @ PoseError::Protocol(_)) => {
            let message = err.to_string();
            assert!(message.contains("somebody-elses-job"), "{message}");
            assert!(message.contains("j-1"), "{message}");
        }
        other => panic!("expected a protocol violation, got {other:?}"),
    }
}

#[test]
fn a_done_with_no_acceptance_before_it_is_refused() {
    // Unlike a failure, this one is not legal: with no frame count there was never a deadline, so
    // the reply arrived under a rule nobody applied.
    let mut worker = spawn("done_without_accept");
    match worker.run(&job("j-1")) {
        Err(err @ PoseError::Protocol(_)) => {
            assert!(err.to_string().contains("no acceptance"), "{err}");
        }
        other => panic!("expected a protocol violation, got {other:?}"),
    }
}

#[test]
fn a_second_acceptance_for_one_job_is_refused() {
    let mut worker = spawn("second_accept");
    match worker.run(&job("j-1")) {
        Err(err @ PoseError::Protocol(_)) => {
            assert!(err.to_string().contains("second acceptance"), "{err}");
        }
        other => panic!("expected a protocol violation, got {other:?}"),
    }
}

#[test]
fn a_reply_before_the_handshake_is_indistinguishable_from_the_wrong_program() {
    // **A P4 finding, and the reason this test is named for what happens rather than for what was
    // expected.** Clause 3's ordering is enforced by the reader: the first line off a candidate is
    // parsed as a `Handshake`, so a worker that answered a job before handshaking has not sent a
    // protocol message at all as far as this side can tell, and it lands on the same fallback path
    // as a program that printed. The report is still honest — it names every candidate and the
    // line each one sent — it just cannot say *which* of the two went wrong.
    match Worker::spawn(&stub("reply_first")) {
        Err(err @ PoseError::Spawn { .. }) => {
            let message = err.to_string();
            assert!(message.contains("not a protocol message"), "{message}");
            assert!(message.contains("failed"), "{message}");
        }
        other => panic!("expected a spawn failure, got {:?}", other.err()),
    }
}

#[test]
fn a_wedged_startup_is_bounded_rather_than_waited_out_forever() {
    let config = Config {
        handshake_timeout: Duration::from_millis(400),
        ..stub("hang_handshake")
    };
    match Worker::spawn(&config) {
        Err(PoseError::Spawn { detail, .. }) => {
            assert!(detail.contains("no handshake within"), "{detail}");
        }
        other => panic!("expected a spawn failure, got {:?}", other.err()),
    }
}

#[test]
fn an_interpreter_that_cannot_be_started_is_skipped_for_the_next_candidate() {
    // Clause 9's first step naming something that is not there must not end the search: a stale
    // configured path is the commonest way this goes wrong and the cheapest to recover from.
    let config = Config {
        python: Some(PathBuf::from("/nowhere/at/all/python")),
        ..stub("ready")
    };
    let mut worker = Worker::spawn(&config).expect("the search moved on to a real interpreter");
    assert!(matches!(worker.run(&job("j-1")), Ok(Outcome::Done(_))));
}

#[test]
fn a_python_that_dies_without_speaking_is_skipped_for_one_that_answers() {
    // **The dev-box case, and this phase's finding.** On a source checkout the `python` on `PATH`
    // is a system interpreter that cannot `import golf_coach`; the `.venv` two candidates later
    // can. The stub reproduces that by refusing to answer unless it is the venv running it, which
    // is the only way one script can behave differently per interpreter.
    let venv = repo().join(".venv");
    if !venv.is_dir() {
        // Nothing to fall back *to*, so there is nothing to test. §M26 ships an interpreter and
        // `Config::python` is set, at which point this path is gone.
        return;
    }
    let candidates = interpreter_candidates(&stub("venv_only"));
    assert!(
        candidates.len() > 1,
        "this box has one interpreter, so there is no fallback to exercise: {candidates:?}"
    );
    let worker = Worker::spawn(&stub("venv_only")).expect("the venv answered");
    assert!(
        worker.program().to_string_lossy().contains(".venv"),
        "resolved {:?}, which is not the checkout's interpreter",
        worker.program()
    );
}

#[test]
fn no_interpreter_answering_reports_every_one_it_tried_and_what_each_said() {
    // The failure where the list *is* the diagnosis. Each candidate carries the exit code and the
    // stderr line that explains it, folded onto one line.
    match Worker::spawn(&stub("no_handshake")) {
        Err(err @ PoseError::Spawn { .. }) => {
            let message = err.to_string();
            assert!(message.contains("no interpreter answered"), "{message}");
            assert!(message.contains("ModuleNotFoundError"), "{message}");
            assert!(message.contains("exit code 1"), "{message}");
            assert!(!err.retried(), "a fact about the machine");
        }
        other => panic!("expected a spawn failure, got {:?}", other.err()),
    }
}

#[test]
fn a_killed_worker_refuses_the_next_job_rather_than_panicking() {
    let mut worker = spawn("ready");
    worker.kill();
    worker.kill(); // Idempotent, because `Drop` calls it again after this test does.
    assert!(worker.run(&job("j-1")).is_err());
}

/// The real worker, on the real 344-frame clip. **Skipped unless `GOLF_POSE_REAL_WORKER` is set.**
///
/// ~40 s, and it needs the `vision` extra plus the `heavy` bundle on disk. It is here so the stub
/// cannot quietly become the only thing this crate has ever talked to; it is deliberately *not* the
/// gate, because a frame count is not a conformance check. **P6 diffs a real reply against the
/// committed `face_on.keypoints.json`**, which is the comparison that means something.
#[test]
fn the_real_worker_poses_a_real_clip() {
    if std::env::var_os("GOLF_POSE_REAL_WORKER").is_none() {
        return;
    }
    let dir = repo()
        .join("data")
        .join("processed")
        .join("sessions")
        .join("2026-08-23")
        .join("11");
    let clip = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("reading {dir:?}: {e}"))
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("face_on.") && name.ends_with(".MOV"))
        })
        .expect("the corpus clip this milestone was measured on");

    let config = Config {
        // Clause 6's real numbers, because the point of this test is the real path.
        ..Config::default()
    };
    let mut worker = Worker::spawn(&config).expect("the real worker handshook");
    assert_eq!(worker.estimator(), "mediapipe:heavy");

    let mut real = job("real-1");
    real.clip_path = clip;
    match worker.run(&real).expect("the clip posed") {
        Outcome::Done(keypoints) => {
            assert_eq!(keypoints.frames.len(), 344);
            assert_eq!(keypoints.pose_estimator.as_deref(), Some("mediapipe:heavy"));
            let clip_metadata = keypoints.clip.expect("the reply carries clip metadata");
            assert!(
                clip_metadata.source_sha256.is_some(),
                "clause 4's open field"
            );
        }
        other => panic!("expected landmarks, got {other:?}"),
    }
}
