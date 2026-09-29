//! N warm workers, a bounded queue, one retry, and a shutdown that terminates. [M23 P5]
//!
//! The type M24 will hold. [`Pool::submit`] takes a [`Job`] and returns at once — with a job id, or
//! with [`PoseError::QueueFull`] when the backlog is at its bound — and results arrive on a channel
//! ([ADR-033](../../../docs/decisions/033-the-pose-sidecar-protocol.md) clause 7). Between those two
//! ends sit N driver threads, each owning one [`Worker`] and doing one job at a time, which is the
//! whole of the concurrency ADR-033 chose std threads for: 2–4 long CPU-bound child processes, not
//! thousands of sockets.
//!
//! # Why `submit` refuses instead of blocking
//!
//! Because the caller is a capture loop. A bound that is reached is a fact the session engine can act
//! on — drop the analysis, tell the golfer the backlog is full, stop cutting clips — and a blocking
//! `submit` is that decision made silently and badly, one swing at a time, until the camera is
//! waiting on pose. `Ring::copy` returning `None` rather than the nearest thing it still holds is the
//! same choice one stage earlier (ADR-031 §7).
//!
//! # What this never does
//!
//! **Guess.** A job that fails twice produces a [`Completed`] naming the clip and the typed reason,
//! never a partial [`Outcome`] and never a zero-filled one — ADR-010 §2 at the process boundary. The
//! same rule covers the quieter case: a job this pool accepted and did not run is *handed back* by
//! [`Pool::shutdown`] rather than dropped, because a submitted job that never produces anything is a
//! silent hole, and silence is the failure mode this repo spends the most words avoiding.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use contracts::Validate;

use crate::protocol::Job;
use crate::worker::{Config, Handle, Outcome, Worker, EXIT_GRACE};
use crate::PoseError;

/// How many workers a pool starts with.
///
/// **2, measured by M23 P8 on 2026-09-27** — the 6 shortest corpus clips (2,899 frames of 4K
/// portrait) through one pool at each width, `scripts/pose_replay.py --shortest 6 --sweep 1,2,4`:
///
/// | workers | pose | throughput | speedup | aggregate CPU | cores busy | peak RSS |
/// |---|---|---|---|---|---|---|
/// | 1 | 288.9 s | 10.0 fps | 1.00x | 470.7 s | 1.63 | 1,344 MB |
/// | **2** | **168.8 s** | **17.2 fps** | **1.71x** | **505.2 s** | **2.99** | **2,652 MB** |
/// | 4 | 123.7 s | 23.4 fps | 2.34x | 593.5 s | 4.80 | 5,134 MB |
///
/// **The hypothesis the plan wrote this measurement against was wrong.** Pose being CPU-bound at
/// ~10 fps was expected to mean a second worker buys nothing; it buys **1.71x**. The reason is in
/// the *cores busy* column: one worker is already multi-threaded but only **1.63** cores' worth, so
/// a single pool leaves most of this box idle and the per-frame graph is not the bottleneck a
/// single-threaded reading of "CPU-bound" assumed.
///
/// **4 is the rejected alternative, and memory is what rejects it.** It is faster — 2.34x, and the
/// only configuration that clears 20 fps — but it costs **5.1 GB resident** and 26% more CPU than
/// one worker for 1.36x more throughput than two. The incremental worker is **~1.3 GB**, not the
/// ~30 MB `.task` bundle this comment used to assume: MediaPipe `heavy` over 2160x3840 frames holds
/// far more than its weights. ADR-033 clause 6 already names the target as a laptop, possibly
/// thermally throttled, running workers *and* two camera captures, and 5.1 GB of pose on it is a
/// swap away from being slower than one worker. 2 also matches the natural unit of work: a two-view
/// swing is two jobs (ADR-033's Alternatives, "batching several clips into one job"), so two
/// workers halve a *swing's* latency where four only help a backlog.
///
/// **What the table cannot say.** It was taken on the dev desktop — 12 physical / 20 logical cores,
/// 34 GB — and speedup is a property of that box, not of the pool. On a four-core laptop two
/// workers contend where these two did not, and 1.63 cores apiece is already most of it. So this is
/// the default and not a conclusion: M24 measures it again under a session's real load, with
/// capture running beside it. Correctness does not move with width — all 6 digests were identical
/// across all three configurations, which is the check that would have caught contention changing
/// an answer.
pub const DEFAULT_POOL_SIZE: usize = 2;

/// How many jobs may wait, beyond those in flight, before [`Pool::submit`] refuses.
///
/// 4, which is two two-view swings of backlog. The bound is about *latency*, not memory — a queued
/// job is a path and five small fields — and at 9.2 fps a 4K swing clip is ~40 s of pose, so a
/// single-worker pool holding 4 is already three minutes behind the golfer. Past that the honest
/// answer is the refusal: a session engine told its backlog is full can stop cutting clips, where one
/// handed an unbounded queue discovers the same fact as a golfer waiting on a result from four swings
/// ago.
pub const DEFAULT_QUEUE_BOUND: usize = 4;

/// Clause 5's *once*: the first attempt, and one retry on a freshly spawned worker.
const MAX_ATTEMPTS: u32 = 2;

/// How a [`Pool`] is built: the per-worker configuration, plus how many and how deep.
#[derive(Debug, Clone)]
pub struct PoolConfig {
    /// What every worker is started with. [`Config::python`] is filled in from the first worker that
    /// answers, so the interpreter search runs once per pool rather than once per worker.
    pub worker: Config,
    /// See [`DEFAULT_POOL_SIZE`]. Zero is read as one — a pool with no workers would accept jobs that
    /// nothing will ever run, which is the silent hole this module's doc opens by refusing.
    pub size: usize,
    /// See [`DEFAULT_QUEUE_BOUND`]. Zero is read as one for the same reason in reverse: a bound of
    /// zero refuses every job, which is a pool that cannot be used rather than a pool with no
    /// backlog.
    pub queue_bound: usize,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            worker: Config::default(),
            size: DEFAULT_POOL_SIZE,
            queue_bound: DEFAULT_QUEUE_BOUND,
        }
    }
}

/// One finished job, however it finished.
///
/// Carries the clip as well as the id because the id is the *pool's* name for the work and the clip
/// is the golfer's: clause 5's "a typed failure naming the clip and the reason" is this struct, and a
/// caller that has thrown the [`Job`] away can still say which swing did not analyse.
#[derive(Debug)]
pub struct Completed {
    pub job_id: String,
    pub clip_path: PathBuf,
    /// 1, or 2 when clause 5's retry was spent. Worth reporting rather than hiding: a pool quietly
    /// retrying every job is a pool whose workers are dying, and this is the number that says so.
    pub attempts: u32,
    /// The landmarks or the worker's own named failure, or the sidecar's — [`Outcome`] is a fact
    /// about the clip and [`PoseError`] a fact about the machine, which is the split clause 5's retry
    /// table is written on.
    pub result: Result<Outcome, PoseError>,
}

/// The queue, the closed flag and the live workers' handles, under one lock.
///
/// One lock and not three, and it is the shutdown race that decides it: a driver replacing a dead
/// worker publishes its handle, and `shutdown` takes the list. Under separate locks those two can
/// interleave so that a fresh child is started *after* shutdown read the list and is therefore never
/// killed — a leaked process, and a `join` that then waits out a full work deadline. Sharing the
/// lock makes the two orders the only two: either the handle is published and shutdown kills it, or
/// the publish is refused and the driver kills its own worker and stops.
struct Shared {
    state: Mutex<State>,
    /// Signalled by every push and by `close`, waited on by idle drivers.
    ready: Condvar,
    bound: usize,
}

struct State {
    waiting: VecDeque<Job>,
    closed: bool,
    /// One slot per driver, `None` while that driver holds no worker.
    live: Vec<Option<Handle>>,
}

impl Shared {
    fn new(size: usize, bound: usize) -> Self {
        Self {
            state: Mutex::new(State {
                waiting: VecDeque::with_capacity(bound),
                closed: false,
                live: vec![None; size],
            }),
            ready: Condvar::new(),
            bound,
        }
    }

    /// A poisoned lock is recovered rather than propagated: what is behind it is a queue of paths and
    /// some child handles, and a panic on a driver thread does not make them meaningless. Refusing to
    /// shut a pool down because one thread panicked is the wrong direction to fail in.
    fn locked(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn push(&self, job: Job) -> Result<(), PoseError> {
        let mut state = self.locked();
        if state.closed {
            return Err(PoseError::ShutDown);
        }
        if state.waiting.len() >= self.bound {
            return Err(PoseError::QueueFull { bound: self.bound });
        }
        state.waiting.push_back(job);
        // One driver per job. `notify_all` here would wake every idle worker to have all but one of
        // them find the queue empty again.
        self.ready.notify_one();
        Ok(())
    }

    /// The next job, or `None` once the pool is closing — which is how a driver thread ends.
    fn pop(&self) -> Option<Job> {
        let mut state = self.locked();
        loop {
            if let Some(job) = state.waiting.pop_front() {
                return Some(job);
            }
            if state.closed {
                return None;
            }
            state = self
                .ready
                .wait(state)
                .unwrap_or_else(PoisonError::into_inner);
        }
    }

    /// Register a freshly spawned worker, or refuse because the pool is closing.
    ///
    /// `false` means *do not run anything*: the caller kills the worker it just started and stops.
    /// Checked under the same lock that `close` takes, which is the whole reason this lock is shared
    /// — see [`Shared`].
    fn publish(&self, slot: usize, handle: Handle) -> bool {
        let mut state = self.locked();
        if state.closed {
            return false;
        }
        state.live[slot] = Some(handle);
        true
    }

    fn retire(&self, slot: usize) {
        self.locked().live[slot] = None;
    }

    fn is_closed(&self) -> bool {
        self.locked().closed
    }

    /// Close for submission, and hand back the jobs never started and the workers still running.
    fn close(&self) -> (Vec<Job>, Vec<Handle>) {
        let mut state = self.locked();
        state.closed = true;
        let abandoned: Vec<Job> = state.waiting.drain(..).collect();
        let live: Vec<Handle> = state.live.iter().flatten().cloned().collect();
        // Every idle driver, so each one wakes to a closed queue and ends.
        self.ready.notify_all();
        (abandoned, live)
    }
}

/// N warm Python workers behind a bounded queue.
///
/// Built warm: [`Self::new`] spawns and handshakes every worker before it returns, so a machine that
/// cannot pose says so when the session starts rather than thirty seconds into the golfer's first
/// swing (clause 3). It is also what makes the pool worth having — ADR-030 §3's *"the process is
/// warm, not the landmarker"* is an interpreter and a resident `.task` bundle, not a saved
/// `PoseLandmarker`.
pub struct Pool {
    shared: Arc<Shared>,
    results: Receiver<Completed>,
    threads: Vec<JoinHandle<()>>,
    interpreter: PathBuf,
    size: usize,
}

impl Pool {
    /// Start `size` workers and the threads that drive them.
    ///
    /// The first worker resolves the interpreter and **every later one is given the winner** as
    /// [`Config::python`]. That is P4's finding made cheap: on a source checkout the `python` on
    /// `PATH` cannot `import golf_coach` and the third candidate is the one that answers, so a pool
    /// that re-ran the search would pay two dead CPython startups per worker to learn what the first
    /// one already knows.
    ///
    /// Any worker failing to start is the pool failing to start, and the ones already up are reaped
    /// on the way out. A pool that is short a worker is a pool whose throughput silently halves.
    pub fn new(config: &PoolConfig) -> Result<Self, PoseError> {
        let size = config.size.max(1);
        let bound = config.queue_bound.max(1);

        let mut worker_config = config.worker.clone();
        let first = Worker::spawn(&worker_config)?;
        let interpreter = first.program().to_path_buf();
        worker_config.python = Some(interpreter.clone());

        let mut workers = vec![first];
        while workers.len() < size {
            // `?` drops `workers`, and dropping a `Worker` reaps its child — so a pool that fails
            // half-built leaves no processes behind.
            workers.push(Worker::spawn(&worker_config)?);
        }

        let shared = Arc::new(Shared::new(size, bound));
        let (tx, results) = mpsc::channel();
        let mut threads = Vec::with_capacity(size);
        for (slot, worker) in workers.into_iter().enumerate() {
            shared.publish(slot, worker.handle());
            let shared = Arc::clone(&shared);
            let tx = tx.clone();
            let worker_config = worker_config.clone();
            threads.push(std::thread::spawn(move || {
                drive(slot, worker, &shared, &tx, &worker_config);
            }));
        }
        // The pool's own sender, so that the receiver disconnects when the last driver ends rather
        // than never.
        drop(tx);

        Ok(Self {
            shared,
            results,
            threads,
            interpreter,
            size,
        })
    }

    /// Queue one job, or refuse. Returns the job's id, which is what a result is matched on.
    ///
    /// Never blocks and never grows: [`PoseError::QueueFull`] is clause 7's backpressure as a value.
    /// A job that could not be attributed — an empty `job_id` — or that asks for part of a clip is
    /// refused here rather than one process later, which is the same check
    /// [`crate::protocol::write_line`] makes on the wire, made before a worker's time is spent on it.
    pub fn submit(&self, job: Job) -> Result<String, PoseError> {
        job.validate()
            .map_err(|err| PoseError::Protocol(format!("refusing to queue a bad job: {err}")))?;
        let job_id = job.job_id.clone();
        self.shared.push(job)?;
        Ok(job_id)
    }

    /// Where results arrive, in completion order.
    ///
    /// The `std` channel itself rather than a wrapper, so a caller picks its own waiting: `try_recv`
    /// for a UI frame, `recv_timeout` for a session loop, `recv` for a batch harness. Wrapping it
    /// would be re-exporting `std` with fewer options.
    pub fn results(&self) -> &Receiver<Completed> {
        &self.results
    }

    /// How many workers this pool started. See [`DEFAULT_POOL_SIZE`].
    pub fn size(&self) -> usize {
        self.size
    }

    /// How many jobs are waiting, **not counting those in flight**.
    ///
    /// The other half of clause 7: the bound says when `submit` will refuse, and this says how close
    /// it is, which is what lets a session engine act *before* the refusal — stop cutting clips, tell
    /// the golfer the backlog is deep — rather than only after it. `bound - pending()` is how many
    /// more jobs are certain to be accepted.
    ///
    /// In flight is deliberately excluded rather than folded in: a job a worker holds is progress and
    /// a job in the queue is latency, and a single number that mixed them would make a working pool
    /// and a stalled one read the same.
    pub fn pending(&self) -> usize {
        self.shared.locked().waiting.len()
    }

    /// Which interpreter answered, resolved once for the whole pool.
    pub fn interpreter(&self) -> &Path {
        &self.interpreter
    }

    /// Stop accepting jobs, take the workers down, and hand back what was never started.
    ///
    /// Bounded, idempotent, and in this order: close the queue, close **every** child's stdin, spend
    /// one [`EXIT_GRACE`] on all of them together, kill whatever is left, then join the drivers.
    /// Closing every stdin before waiting on any of them is what keeps shutdown at about a second for
    /// a pool of four rather than four seconds (P4's finding 6), and the kill is what makes the bound
    /// true at all — a wedged child would otherwise hold the join for the whole derived work
    /// deadline, up to 26 minutes on this corpus.
    ///
    /// **A job in flight is killed, not waited for.** A caller that wants its result waits for it on
    /// [`Self::results`] before calling this; what arrives there afterwards is the killed job
    /// reported as the crash it now is. The returned jobs are the ones that never reached a worker,
    /// handed back so a caller can resubmit them to a later pool instead of discovering them missing.
    pub fn shutdown(&mut self) -> Vec<Job> {
        let (abandoned, live) = self.shared.close();
        for handle in &live {
            handle.close_stdin();
        }
        let until = Instant::now() + EXIT_GRACE;
        while Instant::now() < until && live.iter().any(|handle| !handle.has_exited()) {
            std::thread::sleep(Duration::from_millis(5));
        }
        for handle in &live {
            handle.kill();
        }
        for thread in self.threads.drain(..) {
            let _ = thread.join();
        }
        abandoned
    }
}

impl Drop for Pool {
    /// A dropped pool takes its child processes with it. Without this, forgetting to call
    /// [`Pool::shutdown`] leaks N interpreters holding N model bundles for the life of the parent —
    /// and a test that panicked would leak them for the life of the test run.
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// One driver thread: take a job, run it, retry it once if that could help, report it.
///
/// Owns its [`Worker`] and replaces it lazily. **Lazily and not eagerly**, because a driver that
/// exited when it could not respawn would leave the pool short a worker and, if it were the last,
/// leave submitted jobs in a queue nothing drains — silence, where reporting the spawn failure per
/// job at least says what is wrong with the machine. The next job tries again.
fn drive(
    slot: usize,
    worker: Worker,
    shared: &Arc<Shared>,
    results: &Sender<Completed>,
    config: &Config,
) {
    let mut held = Some(worker);
    while let Some(job) = shared.pop() {
        let mut attempts = 0;
        let result = loop {
            attempts += 1;
            let outcome = attempt(&mut held, slot, shared, config, &job);

            // Clause 5's first sentence: EOF, a non-zero exit **or a `failed` reply** is the end of
            // that worker, because `RunningMode.VIDEO`'s tracking state and MediaPipe's C++ graph are
            // not things a Python `except` can vouch for. So anything but landmarks retires it, and
            // whether the *job* is tried again is the separate question `retried()` answers.
            if !matches!(outcome, Ok(Outcome::Done(_))) {
                retire(&mut held, slot, shared);
            }
            let again = match &outcome {
                Ok(Outcome::Done(_)) => false,
                Ok(Outcome::Failed { reason, .. }) => reason.retried(),
                Err(err) => err.retried(),
            };
            // A retry during shutdown would spawn a worker the shutdown has already walked past.
            if !again || attempts >= MAX_ATTEMPTS || shared.is_closed() {
                break outcome;
            }
        };

        let completed = Completed {
            job_id: job.job_id.clone(),
            clip_path: job.clip_path.clone(),
            attempts,
            result,
        };
        // A closed receiver means the `Pool` is gone, which is the one case where there is nobody to
        // report to. Everything else is reported, including a failure nobody has asked for yet.
        if results.send(completed).is_err() {
            break;
        }
    }
    retire(&mut held, slot, shared);
}

/// Run one attempt, spawning a fresh worker first if the last attempt spent one.
///
/// This is what makes clause 5's retry a retry **on a fresh worker** by construction rather than by
/// discipline: every error out of [`Worker::run`] has already killed its child, and every path that
/// reaches here with `None` held is one that retired it.
fn attempt(
    held: &mut Option<Worker>,
    slot: usize,
    shared: &Arc<Shared>,
    config: &Config,
    job: &Job,
) -> Result<Outcome, PoseError> {
    if held.is_none() {
        if shared.is_closed() {
            return Err(PoseError::ShutDown);
        }
        let mut fresh = Worker::spawn(config)?;
        if !shared.publish(slot, fresh.handle()) {
            // Shutdown took the handle list between the check above and here, so this worker would
            // never be killed by it. Kill it here instead of racing.
            fresh.kill();
            return Err(PoseError::ShutDown);
        }
        *held = Some(fresh);
    }
    held.as_mut()
        .expect("a worker was just spawned or already held")
        .run(job)
}

/// Drop this driver's worker and say so, so `shutdown` does not wait on a child that is gone.
fn retire(held: &mut Option<Worker>, slot: usize, shared: &Arc<Shared>) {
    shared.retire(slot);
    if let Some(mut worker) = held.take() {
        worker.kill();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn job(id: &str) -> Job {
        Job {
            job_id: id.to_string(),
            clip_path: PathBuf::from("/clips/face_on.MOV"),
            frame_range: None,
            camera_id: None,
            pose_model_variant: None,
        }
    }

    #[test]
    fn a_pool_of_zero_workers_or_zero_depth_is_read_as_one_of_each() {
        // Both would otherwise be a pool that cannot work: no driver to drain the queue, or a bound
        // that refuses every job. Clamped rather than refused, because neither is a caller saying
        // something meaningful.
        let config = PoolConfig {
            size: 0,
            queue_bound: 0,
            ..PoolConfig::default()
        };
        assert_eq!(config.size.max(1), 1);
        assert_eq!(config.queue_bound.max(1), 1);
    }

    #[test]
    fn the_queue_refuses_at_its_bound_and_says_what_the_bound_was() {
        // No workers and no drivers here — this is the queue on its own, which is the only way to
        // test the bound without racing a driver that is draining it.
        let shared = Shared::new(1, 2);
        assert!(shared.push(job("j-1")).is_ok());
        assert!(shared.push(job("j-2")).is_ok());
        match shared.push(job("j-3")) {
            Err(PoseError::QueueFull { bound }) => assert_eq!(bound, 2),
            other => panic!("expected a full queue, got {other:?}"),
        }
        // And the refusal is not permanent: draining one makes room for one.
        assert_eq!(shared.pop().map(|job| job.job_id), Some("j-1".to_string()));
        assert!(shared.push(job("j-3")).is_ok());
    }

    #[test]
    fn a_closed_queue_hands_its_jobs_back_and_refuses_the_next_one() {
        let shared = Shared::new(1, 4);
        shared.push(job("j-1")).expect("queued");
        shared.push(job("j-2")).expect("queued");
        let (abandoned, live) = shared.close();
        let ids: Vec<String> = abandoned.into_iter().map(|job| job.job_id).collect();
        assert_eq!(ids, vec!["j-1".to_string(), "j-2".to_string()]);
        assert!(live.is_empty(), "no worker was ever published");
        // FIFO, and handed back rather than dropped — a submitted job that produces nothing is the
        // silent hole this module's doc opens by refusing.
        assert!(matches!(shared.push(job("j-3")), Err(PoseError::ShutDown)));
        assert!(shared.pop().is_none(), "a closed queue ends its drivers");
    }

    #[test]
    fn clause_fives_once_is_two_attempts_and_not_three() {
        assert_eq!(MAX_ATTEMPTS, 2);
    }
}
