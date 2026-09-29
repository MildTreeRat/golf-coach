//! `Pool` against the stub worker: N jobs, one retry, a bound, and a shutdown that ends. [M23 P5]
//!
//! Same gate as `worker.rs` and for the same reason — `tests/stub_worker.py`, stdlib only, one mode
//! per way a child process can behave, and **no MediaPipe, no 4K clip and no 40 seconds anywhere in
//! `cargo test`**. What this file adds over that one is everything that only breaks under
//! concurrency: a reply delivered to the wrong job, a retry that reuses the worker that just died, a
//! queue that grows instead of refusing, and a shutdown that waits on a child which is never coming
//! back.
//!
//! Two modes here are P5's: `crash_once`, which dies on its first job and answers in the fresh
//! process the retry spawns, and `slow`, which is the only way a test can watch three workers drain
//! at once when nothing is actually posing.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use pose::pool::{Completed, Pool, PoolConfig};
use pose::protocol::{FailureReason, Job};
use pose::worker::{Config, Outcome};
use pose::PoseError;

/// The checkout this test is compiled inside — see `tests/worker.rs` for why `CARGO_MANIFEST_DIR` is
/// right here and wrong in `worker::repo_root`.
fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crates/pose has a grandparent")
        .to_path_buf()
}

/// A pool of `size` stubs in one mode, with clause 6's timeouts wound down to test length.
///
/// `fps_estimate` is absurd on purpose, exactly as in `worker.rs`: it drives the derived half of the
/// work deadline to nothing so `work_floor` is what a hang actually waits out, which keeps the
/// slowest path here a bounded second rather than 30.
fn pool_of(size: usize, mode: &str, marker: Option<&Path>) -> PoolConfig {
    let script = repo()
        .join("crates")
        .join("pose")
        .join("tests")
        .join("stub_worker.py");
    let mut worker_args = vec![script.display().to_string(), mode.to_string()];
    if let Some(marker) = marker {
        worker_args.push(marker.display().to_string());
    }
    PoolConfig {
        worker: Config {
            worker_args,
            accept_timeout: Duration::from_secs(1),
            work_floor: Duration::from_secs(1),
            fps_estimate: 1e9,
            ..Config::default()
        },
        size,
        queue_bound: 4,
    }
}

fn start(size: usize, mode: &str) -> Pool {
    Pool::new(&pool_of(size, mode, None))
        .unwrap_or_else(|e| panic!("starting a pool of {size} {mode} stubs: {e}"))
}

/// A job whose `camera_id` is its own id, so the stub echoes the id back *inside* the payload.
fn job(id: &str) -> Job {
    Job {
        job_id: id.to_string(),
        clip_path: PathBuf::from(format!("/clips/{id}.MOV")),
        frame_range: None,
        camera_id: Some(id.to_string()),
        pose_model_variant: None,
    }
}

/// Wait for `n` results, failing the test rather than hanging it.
///
/// 30 s and not `recv`: every deadline in these configs is a second, so a test that reaches this
/// bound has found a job the pool will never answer — which is the one failure mode this crate
/// cannot report on its own, because there is nothing to report it *to*.
fn drain(pool: &Pool, n: usize) -> Vec<Completed> {
    let mut out = Vec::with_capacity(n);
    while out.len() < n {
        match pool.results().recv_timeout(Duration::from_secs(30)) {
            Ok(completed) => out.push(completed),
            Err(err) => panic!("waiting for result {} of {n}: {err}", out.len() + 1),
        }
    }
    out
}

/// Wait until every submitted job has been popped by a driver, i.e. is in flight.
///
/// The distinction a test cannot otherwise see, and two of these tests turn on it: a job that is
/// still *queued* is handed back by `shutdown`, where one in flight is killed and reported. Polling
/// `pending` rather than sleeping a guessed interval, because the wait is microseconds on an idle box
/// and could be anything on a loaded one.
fn in_flight(pool: &Pool) {
    let until = Instant::now() + Duration::from_secs(10);
    while pool.pending() > 0 {
        assert!(Instant::now() < until, "no driver took the job");
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// The `camera_id` the worker echoed, which is the job id these tests submit.
fn echoed(completed: &Completed) -> String {
    match &completed.result {
        Ok(Outcome::Done(keypoints)) => keypoints
            .frames
            .first()
            .and_then(|frame| frame.camera_id.clone())
            .expect("the stub echoes the job's camera_id"),
        other => panic!("expected landmarks for {}, got {other:?}", completed.job_id),
    }
}

#[test]
fn n_jobs_across_n_workers_come_back_attributed_and_in_parallel() {
    // The test that only concurrency can fail: three jobs, three workers, and each reply has to
    // reach the job that asked for it. `job_id` is checked *and* the payload is checked, because a
    // pool that matched ids while handing over the wrong landmarks would pass the first alone.
    let pool = start(3, "slow");
    assert_eq!(pool.size(), 3);
    for id in ["j-1", "j-2", "j-3"] {
        assert_eq!(pool.submit(job(id)).expect("queued"), id);
    }

    let started = Instant::now();
    let results = drain(&pool, 3);
    let elapsed = started.elapsed();

    let mut ids: Vec<String> = results.iter().map(|done| done.job_id.clone()).collect();
    ids.sort();
    assert_eq!(ids, vec!["j-1", "j-2", "j-3"]);
    for completed in &results {
        assert_eq!(echoed(completed), completed.job_id);
        assert!(completed
            .clip_path
            .to_string_lossy()
            .contains(&completed.job_id));
        assert_eq!(completed.attempts, 1, "{} was retried", completed.job_id);
    }
    // Three 0.4 s jobs on three warm workers. Serial would be 1.2 s at the very least, and the
    // workers were already handshaken before the clock started, so this is the pool draining in
    // parallel rather than a measurement of process startup.
    assert!(
        elapsed < Duration::from_secs(1),
        "three 0.4 s jobs took {elapsed:?} on three workers, which is serial"
    );
}

#[test]
fn a_crash_on_the_first_job_is_retried_on_a_fresh_worker_and_succeeds() {
    // Clause 5's crash row, end to end. The marker file is what makes the *fresh* process behave
    // differently from the one that died — a retry on the same worker could not pass this.
    let marker = std::env::temp_dir().join(format!("golf-pose-p5-{}.marker", std::process::id()));
    let _ = std::fs::remove_file(&marker);
    let pool = Pool::new(&pool_of(1, "crash_once", Some(&marker))).expect("the pool started");
    pool.submit(job("j-1")).expect("queued");

    let results = drain(&pool, 1);
    let completed = &results[0];
    assert_eq!(completed.attempts, 2, "clause 5 retries once");
    assert_eq!(echoed(completed), "j-1");
    let _ = std::fs::remove_file(&marker);
}

#[test]
fn a_job_that_fails_twice_is_reported_with_the_clip_and_never_guessed() {
    // The other side of the retry: a second failure is a fact about this clip, reported as one. No
    // partial `KeypointsFile`, no empty `frames`, no zero-filled anything — ADR-010 §2 at the
    // process boundary, which is a promise about what `Completed::result` can hold.
    let pool = start(1, "crash_after_accept");
    pool.submit(job("j-1")).expect("queued");

    let results = drain(&pool, 1);
    let completed = &results[0];
    assert_eq!(completed.attempts, 2);
    assert_eq!(completed.clip_path, PathBuf::from("/clips/j-1.MOV"));
    match &completed.result {
        Err(err @ PoseError::Crashed { .. }) => {
            assert!(err.to_string().contains("the graph closed"), "{err}");
        }
        other => panic!("expected a crash, got {other:?}"),
    }
}

#[test]
fn a_hang_is_killed_at_the_deadline_and_the_job_is_reported_not_left() {
    // A worker that acks and then never answers. The deadline fires, the child is killed, the retry
    // spends one more deadline on a fresh worker, and the job is *reported* — the failure this pool
    // must never turn into silence.
    let pool = start(1, "hang_work");
    pool.submit(job("j-1")).expect("queued");

    let results = drain(&pool, 1);
    let completed = &results[0];
    assert_eq!(completed.attempts, 2);
    match &completed.result {
        Err(PoseError::WorkTimeout { after, frames }) => {
            assert_eq!(*after, Duration::from_secs(1));
            assert_eq!(*frames, 2);
        }
        other => panic!("expected a work timeout, got {other:?}"),
    }
}

#[test]
fn a_failure_the_retry_table_does_not_retry_is_answered_once() {
    // `clip_unreadable`: a fact about the file, so the second read would find the same file. One
    // attempt, and the reason travels to the caller instead of being flattened into an error.
    let pool = start(1, "fail_before_accept");
    // Two jobs, because clause 5's *first* sentence retires the worker on any `failed` reply — so the
    // second job is answered by a worker this pool had to spawn lazily, and a pool that could only
    // fail once would hang here instead of failing twice.
    pool.submit(job("j-1")).expect("queued");
    pool.submit(job("j-2")).expect("queued");

    for completed in drain(&pool, 2) {
        assert_eq!(
            completed.attempts, 1,
            "a retry here cannot change the answer"
        );
        match &completed.result {
            Ok(Outcome::Failed { reason, detail }) => {
                assert_eq!(*reason, FailureReason::ClipUnreadable);
                assert_eq!(detail, "no such file");
            }
            other => panic!("expected a named failure, got {other:?}"),
        }
    }
}

#[test]
fn the_queue_refuses_at_its_bound_rather_than_growing() {
    // Clause 7 through the real pool. One worker that never answers, so nothing ever drains and the
    // refusal is certain — but **not certain at a fixed submission**, which is this test's finding:
    // whether the driver has popped the first job yet decides whether the bound is reached on the
    // third submission or the fourth. So the assertion is on the window, `bound + 1` through
    // `bound + size + 1`, and a refusal outside it is a bound that is not the bound.
    let mut config = pool_of(1, "hang_work", None);
    config.queue_bound = 2;
    let pool = Pool::new(&config).expect("the pool started");

    let mut accepted = 0;
    let refused = loop {
        match pool.submit(job(&format!("j-{}", accepted + 1))) {
            Ok(_) => {
                accepted += 1;
                assert!(accepted <= 3, "a bound of 2 accepted {accepted} jobs");
                assert!(pool.pending() <= 2, "the queue grew past its bound");
            }
            Err(err) => break err,
        }
    };
    assert!(accepted >= 2, "a bound of 2 refused after {accepted}");
    match refused {
        err @ PoseError::QueueFull { bound: 2 } => {
            // The refusal is a value a session engine can print, so it says what the bound was.
            assert!(err.to_string().contains("bound of 2"), "{err}");
        }
        other => panic!("expected a full queue, got {other:?}"),
    }
}

#[test]
fn shutdown_hands_back_the_jobs_it_never_started() {
    // A submitted job that produces nothing is a silent hole, so the ones that never reached a
    // worker come back rather than vanishing — a caller can resubmit them to the next pool.
    let mut pool = start(1, "hang_work");
    for id in ["j-1", "j-2", "j-3"] {
        pool.submit(job(id)).expect("queued");
    }
    // j-1 goes to the one worker and never comes back; j-2 and j-3 are what was never started.
    while pool.pending() > 2 {
        std::thread::sleep(Duration::from_millis(5));
    }
    let abandoned: Vec<String> = pool
        .shutdown()
        .into_iter()
        .map(|job| job.job_id)
        .collect::<Vec<_>>();
    // Handed back in the order they were submitted, which is what makes resubmitting them faithful.
    assert_eq!(abandoned, vec!["j-2", "j-3"]);
    assert!(matches!(pool.submit(job("j-4")), Err(PoseError::ShutDown)));
}

#[test]
fn shutdown_with_a_hung_child_still_terminates() {
    // The bound that makes `shutdown` honest. This child ignores a closed stdin — nothing short of a
    // kill ends it — and a shutdown that waited on the driver thread instead would wait out the
    // whole work deadline, which on a real clip is up to 26 minutes.
    let mut pool = start(2, "hang_work");
    pool.submit(job("j-1")).expect("queued");
    pool.submit(job("j-2")).expect("queued");
    in_flight(&pool);

    let started = Instant::now();
    assert!(pool.shutdown().is_empty(), "both jobs were in flight");
    let elapsed = started.elapsed();
    assert!(
        elapsed < Duration::from_secs(5),
        "shutting down two hung children took {elapsed:?}"
    );

    // And the jobs it killed are reported as the crashes they now are, rather than disappearing.
    for completed in drain(&pool, 2) {
        assert!(
            matches!(
                completed.result,
                Err(PoseError::Crashed { .. } | PoseError::WorkTimeout { .. })
            ),
            "{}: {:?}",
            completed.job_id,
            completed.result
        );
    }
}

#[test]
fn shutdown_lets_a_worker_that_takes_the_hint_finish_the_job_in_flight() {
    // The polite half. Closing stdin is what this protocol says for *no more jobs*, and a worker that
    // honours it finishes what it holds, replies, reads EOF and exits 0 — so the result of a job in
    // flight survives a shutdown. 0.4 s of work against a 1 s grace is the margin that makes this
    // deterministic rather than a race.
    let mut pool = start(1, "slow");
    pool.submit(job("j-1")).expect("queued");
    in_flight(&pool);
    assert!(pool.shutdown().is_empty(), "the job was already in flight");

    let results = drain(&pool, 1);
    assert_eq!(echoed(&results[0]), "j-1");
    assert_eq!(results[0].attempts, 1);
}

#[test]
fn a_dropped_pool_takes_its_children_with_it() {
    // Without `Drop`, forgetting `shutdown` leaks N interpreters holding N model bundles. The
    // observable is the same bound as the explicit shutdown's: dropping a pool of hung children
    // returns rather than blocking on them.
    let started = Instant::now();
    {
        let pool = start(2, "hang_work");
        pool.submit(job("j-1")).expect("queued");
    }
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "dropping a pool with a hung child took {:?}",
        started.elapsed()
    );
}

#[test]
fn a_pool_whose_workers_cannot_pose_does_not_start_at_all() {
    // Clause 3 at pool level: the machine says it cannot pose when the session starts, not thirty
    // seconds into the golfer's first swing. A pool that came up and failed every job would be the
    // same fact delivered too late to act on.
    match Pool::new(&pool_of(2, "unavailable", None)) {
        Err(err @ PoseError::Unavailable { .. }) => {
            assert!(err.to_string().contains("model_absent"), "{err}");
        }
        other => panic!("expected an unavailable pool, got {:?}", other.err()),
    }
}

#[test]
fn a_job_that_could_not_be_attributed_is_refused_at_submit() {
    // The same refusal `write_line` makes on the wire, made before a worker's time is spent on it:
    // an empty `job_id` is a reply nobody could attribute, and a `frame_range` is a caller bug.
    let pool = start(1, "ready");
    let mut nameless = job("j-1");
    nameless.job_id = String::new();
    assert!(matches!(pool.submit(nameless), Err(PoseError::Protocol(_))));

    let mut partial = job("j-2");
    partial.frame_range = Some([10, 20]);
    assert!(matches!(pool.submit(partial), Err(PoseError::Protocol(_))));
}

#[test]
fn the_interpreter_is_resolved_once_for_the_whole_pool() {
    // P4's finding, made cheap. On a source checkout two candidates answer nothing before the
    // `.venv` does, and a pool that re-ran clause 9's search per worker would pay both of those
    // startups N times. The observable is that the pool knows which interpreter won.
    if !repo().join(".venv").is_dir() {
        // Nothing to fall back *to*, and §M26 ships an interpreter anyway — see `worker.rs`.
        return;
    }
    let pool = Pool::new(&pool_of(2, "venv_only", None)).expect("the venv answered");
    assert!(
        pool.interpreter().to_string_lossy().contains(".venv"),
        "resolved {:?}",
        pool.interpreter()
    );
    // And the second worker is real: it answers a job, which it could only do if it was started with
    // the interpreter the first one found.
    pool.submit(job("j-1")).expect("queued");
    pool.submit(job("j-2")).expect("queued");
    let results = drain(&pool, 2);
    assert_eq!(results.len(), 2);
    for completed in &results {
        assert_eq!(echoed(completed), completed.job_id);
    }
}

/// The real `golf_coach.pose.worker` through the real pool. **Skipped unless `GOLF_POSE_REAL_WORKER`
/// is set**, because it needs the `vision` extra and the `heavy` bundle on disk.
///
/// Deliberately poses *nothing*: the clip path does not exist, so the worker answers
/// `clip_unreadable` in about a second and what this exercises is everything around the pose call —
/// a real handshake through `Pool::new`, a real job line, a real failure reply, and a shutdown that
/// closes a real interpreter's stdin. **P6 is the gate** for the reply that carries landmarks.
#[test]
fn the_real_worker_answers_through_the_pool_and_shuts_down() {
    if std::env::var_os("GOLF_POSE_REAL_WORKER").is_none() {
        return;
    }
    let mut pool = Pool::new(&PoolConfig::default()).expect("the real worker handshook");
    let mut missing = job("real-1");
    missing.clip_path = repo().join("data").join("no-such-clip.MOV");
    pool.submit(missing).expect("queued");

    let results = drain(&pool, 1);
    assert_eq!(results[0].job_id, "real-1");
    assert_eq!(results[0].attempts, 1, "a missing file is not retried");
    match &results[0].result {
        Ok(Outcome::Failed { reason, detail }) => {
            assert_eq!(*reason, FailureReason::ClipUnreadable);
            assert!(
                !detail.is_empty(),
                "the worker's own sentence about the clip"
            );
        }
        other => panic!("expected a named failure, got {other:?}"),
    }
    assert!(pool.shutdown().is_empty());
}
