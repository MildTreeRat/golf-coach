//! One child process, one thread, one job. [M23 P4]
//!
//! [`Worker`] owns a `python -m golf_coach.pose.worker` and the threads that read it: it spawns one
//! (clause 9's interpreter resolution), reads the handshake (clause 3), hands it one [`Job`] at a
//! time and waits for clause 4's two replies under clause 6's two timeouts, and kills it when
//! anything goes wrong. `Pool` — N of these, a bounded queue, clause 5's retry and an orderly
//! shutdown — is P5's, and everything here is written to be driven by one of its threads.
//!
//! # Why there is a thread at all
//!
//! Three reasons, and only the first is the obvious one.
//!
//! - **A blocking read cannot be given a deadline.** Clause 6 needs two, so the read happens on its
//!   own thread and this one waits on [`std::sync::mpsc::Receiver::recv_timeout`]. That is the whole
//!   of the concurrency argument ADR-033 makes against tokio: `recv_timeout` is a timeout with no
//!   runtime.
//! - **A drained pipe is what stops a deadlock.** The job line goes out while the worker may already
//!   be writing a 16.4 MB reply; a design that wrote first and read afterwards could block in
//!   `write` against a pipe buffer only a read can empty. The reader thread starts before the first
//!   job and never stops draining.
//! - **Parsing a reply is not free.** [`crate::Reply::job_id`] measured 0.44 s for the corpus's
//!   largest, and that is 0.44 s which does not belong on the thread holding the pool's deadline.
//!
//! # What EOF means here
//!
//! The pipe is the liveness signal clause 1 gets for free, and it arrives as the reader thread
//! dropping its sender. **A worker that closes stdout with a job outstanding has crashed**, whatever
//! its exit code says, and [`PoseError::Crashed`] carries the stderr tail this module kept for
//! exactly that moment — a crash with no stderr is a shrug, and the tail is what makes it a report.

use std::collections::VecDeque;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, ExitStatus, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use contracts::keypoints::KeypointsFile;

use crate::protocol::{write_line, FailureReason, Handshake, Job, Lines, Reply, PROTOCOL_VERSION};
use crate::PoseError;

/// Frames per second one worker poses at, and clause 6's scale factor for the work deadline.
///
/// **Measured, not budgeted**: 344 frames in 37.3 s of 2160×3840 portrait 4K at `mediapipe:heavy` on
/// this desktop, which P3 re-confirmed through the sidecar itself at 8.8 and 10.1 fps on two runs of
/// the same clip. It is deliberately *not* ADR-030 §3's figure — that one is ~2.5× optimistic for
/// this corpus's footage, and P1 corrected it.
///
/// **It is per variant, and it is configuration.** ADR-002 puts `lite` at roughly four times
/// `heavy`'s speed, so a pool of `lite` workers left on this number carries a 4× margin it did not
/// ask for. That is the harmless direction — a deadline too generous stalls one job, where one too
/// tight throws away minutes of pose — but it is still the wrong number, and
/// [`Config::fps_estimate`] is where a `lite` pool says so.
pub const DEFAULT_FPS_ESTIMATE: f64 = 9.2;

/// Clause 6's margin on the derived work deadline.
///
/// 3 and not 1.5, because the two errors do not cost the same: a deadline that fires early kills a
/// healthy worker and throws away up to nine minutes of pose, then pays for it again on the retry,
/// while one that fires late leaves a single job stalled while the rest of the pool keeps draining.
/// [`DEFAULT_FPS_ESTIMATE`] was also measured on an idle desktop, and the machine this ships to is a
/// laptop running two or three workers and two captures at once.
pub const DEFAULT_MARGIN: f64 = 3.0;

/// Clause 6's `FLOOR`: the shortest work deadline, whatever the frame count derives.
///
/// The ADR names the floor and leaves its value to this phase. What it has to cover is the per-job
/// cost that does *not* scale with clip length: constructing a `PoseLandmarker` over the 30 MB
/// `heavy` bundle, the sha256 pass over a 38–49 MB clip (~0.1 s), and serializing the reply. A
/// zero-frame acceptance is the case that makes a floor necessary at all — `Reply::Accepted` allows
/// `frames` of 0 exactly as `ClipMetadata.frame_count` does, and `0 / 9.2 * 3` is a deadline that
/// has already expired.
pub const DEFAULT_WORK_FLOOR: Duration = Duration::from_secs(30);

/// Clause 6's flat accept timeout: how long a worker may take to say it opened the container.
///
/// Opening a container and reading its metadata does not scale with clip length — P3 measured the
/// acceptance line arriving in **0.09 s** on a 4K clip — so this is two orders of magnitude of
/// headroom, and a worker past it is wedged rather than busy. It is not two seconds because the
/// *first* job on a fresh worker also pays for the lazy `import cv2` that `_open_clip` defers, and
/// paying a respawn to save eight seconds on a cold import is the wrong trade.
pub const DEFAULT_ACCEPT_TIMEOUT: Duration = Duration::from_secs(10);

/// How long a spawned interpreter may take to produce its handshake line.
///
/// Generous where [`DEFAULT_ACCEPT_TIMEOUT`] is tight, because this one is paid once per worker and
/// covers a whole CPython startup: the interpreter, `golf_coach.config` and the pydantic-settings
/// import behind it, and `resolve_variant`'s look at the model directory. Seconds on a warm
/// filesystem, and the thing it exists to bound is a worker blocked on a network path.
pub const DEFAULT_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(60);

/// How much of a worker's stderr is kept for the failure that will want it.
///
/// A Python traceback out of `estimate_pose` runs ten to thirty lines, so this holds a whole one
/// rather than its last fragment — the difference between a `pose_failed` a human can act on and one
/// that only says something went wrong. It is a tail and not a log: stderr is the worker's own
/// logging (clause 1), and a long-lived worker would otherwise grow a buffer for the session.
const STDERR_TAIL_LINES: usize = 40;

/// How long a child gets to exit on its own after its stdin is closed, before it is killed.
///
/// Closing stdin is how a caller says shut down — the worker's `readline` returns empty and `main`
/// returns 0 — and a worker that takes it exits in milliseconds. The grace is here so the exit
/// *code* survives: a crashed worker's code is the one fact its report has that the stderr tail
/// does not, and reaping it with a kill would overwrite it. Bounded rather than a bare `wait()` so
/// that a genuinely hung child still terminates.
///
/// **A pool pays it once, not once per worker** — `crate::pool::Pool::shutdown` closes every stdin
/// before it waits on any of them, which is why this is shared with that module rather than private
/// to this one.
pub(crate) const EXIT_GRACE: Duration = Duration::from_secs(1);

/// The module a worker runs: `python -m golf_coach.pose.worker`.
const WORKER_MODULE: [&str; 2] = ["-m", "golf_coach.pose.worker"];

/// How a [`Worker`] is started, and what deadlines it holds its child to.
///
/// Values rather than anything read from the environment, because `crates/pose` has no configuration
/// of its own and should not grow one: `config.py` is where this repo's settings live, and M24 is
/// the caller that will map them onto this.
#[derive(Debug, Clone)]
pub struct Config {
    /// Clause 9's first step: an explicit interpreter, tried before `PATH` and before `.venv`.
    pub python: Option<PathBuf>,
    /// The source checkout to resolve `.venv` and the child's working directory against. `None`
    /// searches upward from the current directory — see [`repo_root`].
    pub repo_root: Option<PathBuf>,
    /// What the interpreter is asked to run. [`WORKER_MODULE`] in production; the one seam the tests
    /// use, because a stub that speaks the protocol is the only way to exercise a crash, a hang and
    /// a garbage line with no MediaPipe anywhere in `cargo test`.
    pub worker_args: Vec<String>,
    /// Which pose bundle this worker serves, passed as `GOLF_POSE_MODEL_VARIANT`. `None` leaves the
    /// worker on its configured default.
    ///
    /// Set on the *process* and not sent per job, because a worker serves exactly the variant it
    /// announced (ADR-033's first addendum): the model for another one may not be on disk, and
    /// [`Self::fps_estimate`] is per variant, so a worker that switched bundle mid-flight would make
    /// every deadline derived from it wrong. **A pool running two variants is two sets of workers.**
    pub variant: Option<String>,
    /// See [`DEFAULT_HANDSHAKE_TIMEOUT`].
    pub handshake_timeout: Duration,
    /// See [`DEFAULT_ACCEPT_TIMEOUT`].
    pub accept_timeout: Duration,
    /// Clause 6's `FLOOR`. See [`DEFAULT_WORK_FLOOR`].
    pub work_floor: Duration,
    /// Clause 6's `fps_estimate`, per variant. See [`DEFAULT_FPS_ESTIMATE`].
    pub fps_estimate: f64,
    /// Clause 6's `MARGIN`. See [`DEFAULT_MARGIN`].
    pub margin: f64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            python: None,
            repo_root: None,
            worker_args: WORKER_MODULE.iter().map(|arg| (*arg).to_string()).collect(),
            variant: None,
            handshake_timeout: DEFAULT_HANDSHAKE_TIMEOUT,
            accept_timeout: DEFAULT_ACCEPT_TIMEOUT,
            work_floor: DEFAULT_WORK_FLOOR,
            fps_estimate: DEFAULT_FPS_ESTIMATE,
            margin: DEFAULT_MARGIN,
        }
    }
}

impl Config {
    /// Clause 6's derived work deadline: `max(FLOOR, frames / fps_estimate * MARGIN)`.
    ///
    /// Derived rather than flat because this corpus spans **14.1×** — 344 frames to 4,837 — and one
    /// constant cannot serve both ends of that without being absurd at one of them. `frames` is the
    /// acknowledged count, which is the container's claim and the only scale factor that exists
    /// before a frame has been posed.
    ///
    /// A non-positive or non-finite `fps_estimate` derives nothing rather than an infinity, and the
    /// floor answers: a misconfigured pool gets a short deadline it will notice, not an eternal one
    /// it will not. `Duration::from_secs_f64` panics on both, which is the other half of the reason
    /// this is checked rather than trusted.
    pub fn work_deadline(&self, frames: i64) -> Duration {
        let derived = frames.max(0) as f64 / self.fps_estimate * self.margin;
        let derived = if derived.is_finite() && derived > 0.0 {
            Duration::from_secs_f64(derived)
        } else {
            Duration::ZERO
        };
        derived.max(self.work_floor)
    }
}

/// What one job produced: landmarks, or a named reason there are none.
///
/// The third possibility — that the *sidecar* failed rather than the clip — is the `Err` of
/// [`Worker::run`], and that split is what clause 5's retry table is written on: a [`FailureReason`]
/// is a fact about a clip which the pool passes along, and a [`PoseError`] is a fact about the
/// machine. Both answer `retried()`, which is what P5 branches on.
///
/// **There is no third variant for a partial result**, and there never will be: a clip that could
/// not be posed produces a named failure, never an empty `frames` list and never a zero-filled one.
/// ADR-010 §2 at the process boundary.
#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    /// The landmarks, exactly as the worker built them. The *caller* writes the file (clause 8).
    Done(KeypointsFile),
    /// Clause 5's table. `detail` is the worker's own sentence about this clip.
    Failed {
        reason: FailureReason,
        detail: String,
    },
}

/// One message off a worker's stdout, in the order clause 1 puts them: a handshake, then replies.
///
/// The two are different types and the reader thread reads one of each shape in turn, which is why
/// the channel carries a sum rather than a [`Reply`]. A second handshake, or a reply before one, is
/// a protocol violation and is reported as one instead of being quietly coerced.
#[derive(Debug)]
enum Incoming {
    Handshake(Handshake),
    Reply(Reply),
}

/// Why a candidate interpreter did not become a worker.
///
/// The distinction is the whole of clause 9's fallback. [`Self::NotThisOne`] means *this program is
/// not our worker* — it could not be started, it died without speaking, or it printed something that
/// is not a protocol message — and the search moves on. [`Self::Answered`] means a worker spoke and
/// said no, which is an answer for the pool rather than a reason to try another Python.
enum Rejected {
    NotThisOne(String),
    Answered(PoseError),
}

/// The child's two writable handles, behind one lock. [M23 P5]
///
/// `stdin` is here rather than beside the reader threads because closing it is how a caller says
/// *shut down* — the worker's `readline` returns empty and `main` returns 0 — and `child` is here
/// because the same lock has to serve the one caller that cannot wait: a [`Handle`] held by a thread
/// that does not own this worker. See [`Handle`] for why that exists at all.
struct Takedown {
    child: Child,
    stdin: Option<ChildStdin>,
    status: Option<ExitStatus>,
    reaped: bool,
}

impl Takedown {
    /// `try_wait`, remembering the answer. Called from both halves of every teardown, which is why
    /// the status is stored rather than returned: whichever one reaps first, the other still has the
    /// child's real exit code to put in a report.
    fn poll(&mut self) -> Option<ExitStatus> {
        if self.status.is_none() {
            self.status = self.child.try_wait().ok().flatten();
        }
        self.status
    }

    fn kill(&mut self) {
        if self.poll().is_some() {
            return;
        }
        let _ = self.child.kill();
        // `wait` and not `try_wait`: a killed child is reaped in microseconds and skipping it leaves
        // a zombie for the lifetime of the pool, which on a long session is one per replaced worker.
        self.status = self.child.wait().ok();
    }
}

/// The one way to stop a worker from a thread that does not own it. [M23 P5]
///
/// A [`Worker`] is held by one thread and [`Worker::run`] blocks in it, so the pool's `shutdown` has
/// no way to reach the child through the `Worker` itself — and *has* to reach it, because a wedged
/// child would otherwise hold shutdown for the whole derived work deadline, up to 26 minutes on this
/// corpus. A cloneable handle to the child's two writable ends is the narrowest thing that solves
/// that: it can close stdin (the polite half) and it can kill (the bounded half), and it can do
/// neither *to a job* — nothing here can cancel work or read a reply.
///
/// Cheap to clone and safe to hold after the worker is gone: every method is idempotent, and one on
/// an already-reaped child is a lock, a `try_wait` and a return.
#[derive(Clone)]
pub struct Handle {
    inner: Arc<Mutex<Takedown>>,
}

impl Handle {
    /// A poisoned lock is recovered rather than propagated, for the reason `Proc::stderr_tail`
    /// already does it: the data behind it is a child handle and an exit status, and a panic on
    /// another thread does not make them meaningless. Refusing to kill a child because a thread
    /// panicked is the wrong direction to fail in.
    fn locked(&self) -> std::sync::MutexGuard<'_, Takedown> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Close the child's stdin, which is how this protocol says *no more jobs*.
    ///
    /// The polite half, and the whole of what clause 7's orderly shutdown is: the real worker
    /// finishes the job it holds, writes its reply, reads EOF and exits 0. A worker that ignores it
    /// is what [`Self::kill`] is for.
    pub fn close_stdin(&self) {
        drop(self.locked().stdin.take());
    }

    /// Whether the child has already gone, so a caller can spend one grace on N of them rather than
    /// N graces in a row (P4's finding 6).
    pub fn has_exited(&self) -> bool {
        self.locked().poll().is_some()
    }

    /// Kill it now, and reap it so no zombie is left behind.
    pub fn kill(&self) {
        self.locked().kill();
    }

    /// Close stdin, give the child [`EXIT_GRACE`] to go on its own, then kill it. Idempotent.
    ///
    /// The grace is what keeps the child's *own* exit code — the one fact a crash report has that
    /// the stderr tail does not — from being overwritten by a kill. The lock is released between
    /// polls rather than held across the wait, so a shutdown on another thread is never blocked
    /// behind a second's sleep.
    fn reap(&self) -> Option<ExitStatus> {
        {
            let mut takedown = self.locked();
            if takedown.reaped {
                return takedown.status;
            }
            takedown.reaped = true;
            drop(takedown.stdin.take());
        }
        let until = Instant::now() + EXIT_GRACE;
        while !self.has_exited() && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(5));
        }
        let mut takedown = self.locked();
        takedown.kill();
        takedown.status
    }

    /// Write one job line, or `None` if stdin has already been closed.
    ///
    /// `None` rather than an error, because *who* is waiting decides what it means: to
    /// [`Worker::write`] a closed stdin is a worker that is already gone, and it has the stderr tail
    /// to say so where this does not.
    fn send(&self, job: &Job) -> Option<Result<(), PoseError>> {
        let mut takedown = self.locked();
        let stdin = takedown.stdin.as_mut()?;
        Some(write_line(stdin, job))
    }
}

/// A child process and the two threads reading it, with one way to take all three down.
///
/// Separate from [`Worker`] so that a handshake which fails still reaps its child: the failure paths
/// in [`Worker::spawn`] outnumber the success one, and a `Drop` that existed only once the handshake
/// had succeeded would leak a process on every one of them.
struct Proc {
    /// The child itself, shared with every [`Handle`] handed out for it.
    handle: Handle,
    incoming: Receiver<Result<Incoming, PoseError>>,
    tail: Arc<Mutex<VecDeque<String>>>,
    threads: Vec<JoinHandle<()>>,
}

impl Proc {
    /// Spawn the child and start draining both of its output pipes.
    fn start(program: &Path, config: &Config) -> std::io::Result<Self> {
        let mut command = Command::new(program);
        command
            .args(&config.worker_args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(variant) = &config.variant {
            command.env("GOLF_POSE_MODEL_VARIANT", variant);
        }
        if let Some(root) = config.repo_root.clone().or_else(repo_root) {
            command.current_dir(root);
        }
        let mut child = command.spawn()?;

        let stdin = child.stdin.take();
        let stdout = child.stdout.take().expect("stdout was piped");
        let stderr = child.stderr.take().expect("stderr was piped");

        let (tx, incoming) = mpsc::channel();
        let tail = Arc::new(Mutex::new(VecDeque::with_capacity(STDERR_TAIL_LINES)));
        let logging = Arc::clone(&tail);
        let threads = vec![
            std::thread::spawn(move || read_messages(BufReader::new(stdout), &tx)),
            std::thread::spawn(move || keep_tail(BufReader::new(stderr), &logging)),
        ];

        Ok(Self {
            handle: Handle {
                inner: Arc::new(Mutex::new(Takedown {
                    child,
                    stdin,
                    status: None,
                    reaped: false,
                })),
            },
            incoming,
            tail,
            threads,
        })
    }

    /// Close stdin, let the child go, kill it if it will not, and join both readers. Idempotent.
    ///
    /// The threads are joined rather than detached because the stderr tail is complete only once its
    /// reader has seen EOF, and that tail is the substance of the crash report this is usually being
    /// called to write. Both pipes hit EOF when the child is reaped, so the join is bounded by the
    /// same grace the wait is — unless a child left a grandchild holding stderr open, which
    /// MediaPipe does not and which would be worth a finding if something ever did.
    fn reap(&mut self) -> Option<ExitStatus> {
        let status = self.handle.reap();
        for thread in self.threads.drain(..) {
            let _ = thread.join();
        }
        status
    }

    fn stderr_tail(&self) -> String {
        let tail = self.tail.lock().unwrap_or_else(PoisonError::into_inner);
        tail.iter().cloned().collect::<Vec<_>>().join("\n")
    }
}

impl Drop for Proc {
    fn drop(&mut self) {
        self.reap();
    }
}

/// Read the handshake, then replies until the stream ends, forwarding each to the [`Worker`].
///
/// Ends on the first error or at EOF, and **dropping the sender is the signal**: a `Disconnected` on
/// the other side is how the worker learns its child's stdout closed, which is clause 1's liveness
/// check. Nothing here decides whether that was a crash or a shutdown — only the thread that knows
/// whether it was waiting can decide that, and it is [`Worker::run`].
fn read_messages<R: BufRead>(stream: R, tx: &Sender<Result<Incoming, PoseError>>) {
    let mut lines = Lines::new(stream);
    match lines.read::<Handshake>() {
        Ok(Some(handshake)) => {
            if tx.send(Ok(Incoming::Handshake(handshake))).is_err() {
                return;
            }
        }
        Ok(None) => return,
        Err(err) => {
            let _ = tx.send(Err(err));
            return;
        }
    }
    loop {
        match lines.read::<Reply>() {
            Ok(Some(reply)) => {
                if tx.send(Ok(Incoming::Reply(reply))).is_err() {
                    return;
                }
            }
            Ok(None) => return,
            Err(err) => {
                let _ = tx.send(Err(err));
                return;
            }
        }
    }
}

/// Keep the last [`STDERR_TAIL_LINES`] lines a worker logged, for whatever fails next.
fn keep_tail<R: BufRead>(stream: R, tail: &Mutex<VecDeque<String>>) {
    for line in stream.lines().map_while(Result::ok) {
        let mut tail = tail.lock().unwrap_or_else(PoisonError::into_inner);
        if tail.len() == STDERR_TAIL_LINES {
            tail.pop_front();
        }
        tail.push_back(line);
    }
}

/// The source checkout this build is running inside, or `None` if it is not running inside one.
///
/// Searches upward from the current directory for the pair of manifests only the repo root has —
/// `pyproject.toml` beside `Cargo.toml`, which is the layout the workspace manifest chose
/// deliberately. That is the runtime equivalent of `config.py`'s `REPO_ROOT`, and it inherits the
/// same assumption ADR-030 records as dying at packaging time: **§M26 ships an interpreter and there
/// is no checkout to find**, at which point [`Config::python`] is set and this is never called.
///
/// Searching rather than baking `CARGO_MANIFEST_DIR` in at compile time, because that constant is
/// the path the *build machine* had, and a binary copied elsewhere would go on insisting on it.
pub fn repo_root() -> Option<PathBuf> {
    let mut dir = std::env::current_dir().ok()?;
    loop {
        if dir.join("pyproject.toml").is_file() && dir.join("Cargo.toml").is_file() {
            return Some(dir);
        }
        if !dir.pop() {
            return None;
        }
    }
}

/// Where a source checkout leaves its interpreter — clause 9's third step.
fn venv_python(root: &Path) -> PathBuf {
    if cfg!(windows) {
        root.join(".venv").join("Scripts").join("python.exe")
    } else {
        root.join(".venv").join("bin").join("python")
    }
}

/// The first file named `name` (or `name.exe`) on `PATH`.
///
/// Hand-rolled rather than a `which` crate, for the reason every dependency here is weighed: this is
/// a dozen lines against another crate in the tree of a process boundary meant to have almost none.
fn on_path(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    let names: Vec<String> = if cfg!(windows) {
        vec![format!("{name}.exe"), name.to_string()]
    } else {
        vec![name.to_string()]
    };
    std::env::split_paths(&path).find_map(|dir| {
        names
            .iter()
            .map(|name| dir.join(name))
            .find(|candidate| candidate.is_file())
    })
}

/// Clause 9's resolution order, as the list of interpreters that exist: the configured one, then
/// `PATH`, then the source checkout's `.venv`.
///
/// **It is a list and not a first hit, and that is this phase's one departure from how clause 9
/// reads.** `audio/trigger.py::binary()` takes the first `golf-trigger` on `PATH` because any
/// `golf-trigger` is the right one; a Python is not interchangeable that way — it has to be a Python
/// that can *import `golf_coach`*, and on a source-checkout dev box the `python` on `PATH` is a
/// system interpreter that cannot. Measured on this box: `PATH` answers with a Python 3.13 that
/// raises `ModuleNotFoundError: No module named 'golf_coach'`, and the `.venv` found after it is the
/// one that works. So the *order* is clause 9's, unchanged, and what "answer" means is a handshake
/// rather than a file existing — which is the reading clause 9's own *"if none of them answer"*
/// already allows. Recorded in ADR-033's second addendum.
///
/// Duplicates are dropped, so a `PATH` entry that *is* the venv is not tried twice.
pub fn interpreter_candidates(config: &Config) -> Vec<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(explicit) = &config.python {
        candidates.push(explicit.clone());
    }
    for name in ["python", "python3"] {
        if let Some(found) = on_path(name) {
            candidates.push(found);
        }
    }
    if let Some(root) = config.repo_root.clone().or_else(repo_root) {
        let venv = venv_python(&root);
        if venv.is_file() {
            candidates.push(venv);
        }
    }
    let mut seen: Vec<PathBuf> = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        if !seen.contains(&candidate) {
            seen.push(candidate);
        }
    }
    seen
}

/// What [`interpreter_candidates`] looked at, for the error raised when it found nothing.
///
/// The bare names rather than resolved paths, because a `PATH` that answers nothing has no path to
/// report, and "looked for python, python3" is the sentence that tells someone what to install.
fn searched(config: &Config) -> Vec<PathBuf> {
    let mut looked = Vec::new();
    if let Some(explicit) = &config.python {
        looked.push(explicit.clone());
    }
    looked.push(PathBuf::from("python"));
    looked.push(PathBuf::from("python3"));
    if let Some(root) = config.repo_root.clone().or_else(repo_root) {
        looked.push(venv_python(&root));
    }
    looked
}

/// One Python worker process, and the one job it is doing.
///
/// Held by one thread and never shared. `submit`, a bounded queue and clause 5's retry across a
/// *fresh* worker are the pool's, which is P5; what this type guarantees is narrower and is what
/// that pool is built on — **one job at a time, clause 4's two replies in order, and a typed error
/// for every way the child can fail instead of a hang or a panic**.
pub struct Worker {
    proc: Proc,
    config: Config,
    program: PathBuf,
    estimator: String,
    variant: String,
    model: PathBuf,
}

impl Worker {
    /// Resolve an interpreter, start it, and read its handshake. Clauses 3 and 9.
    ///
    /// Candidates are tried in clause 9's order until one *answers*: a [`Handshake`] whose protocol
    /// version this build understands, or a refusal a worker made deliberately. A candidate that
    /// cannot be started, dies without speaking, or prints something that is not a protocol message
    /// is not our worker, and the search moves on carrying what it said — see
    /// [`interpreter_candidates`] for why that fallback exists rather than a first hit.
    ///
    /// [`Handshake::Unavailable`] is an answer and stops the search. It is also a **pool-level**
    /// failure and not a failed job (clause 3): a machine with no `.task` bundle says so when the
    /// session starts rather than thirty seconds into the golfer's first swing, and no number of
    /// fresh workers grows a model.
    pub fn spawn(config: &Config) -> Result<Self, PoseError> {
        let candidates = interpreter_candidates(config);
        if candidates.is_empty() {
            return Err(PoseError::InterpreterNotFound {
                looked_for: searched(config),
            });
        }
        let mut rejected: Vec<String> = Vec::new();
        for program in &candidates {
            match Self::start(program, config) {
                Ok(worker) => return Ok(worker),
                Err(Rejected::Answered(err)) => return Err(err),
                Err(Rejected::NotThisOne(why)) => {
                    rejected.push(format!("{} — {why}", program.display()));
                }
            }
        }
        Err(PoseError::Spawn {
            // The last candidate is the `.venv` when there is one, which is the interpreter whoever
            // reads this meant to use. The detail carries every candidate and why each did not
            // answer, because on this failure the list *is* the diagnosis.
            program: candidates.last().cloned().unwrap_or_default(),
            detail: format!(
                "no interpreter answered the handshake: {}",
                rejected.join("; ")
            ),
        })
    }

    fn start(program: &Path, config: &Config) -> Result<Self, Rejected> {
        let mut proc = Proc::start(program, config)
            .map_err(|err| Rejected::NotThisOne(format!("could not start it: {err}")))?;

        match proc.incoming.recv_timeout(config.handshake_timeout) {
            Ok(Ok(Incoming::Handshake(Handshake::Ready {
                protocol,
                estimator,
                variant,
                model,
            }))) => {
                if protocol != PROTOCOL_VERSION {
                    return Err(Rejected::Answered(PoseError::ProtocolMismatch {
                        theirs: protocol,
                        ours: PROTOCOL_VERSION,
                    }));
                }
                Ok(Self {
                    proc,
                    config: config.clone(),
                    program: program.to_path_buf(),
                    estimator,
                    variant,
                    model,
                })
            }
            Ok(Ok(Incoming::Handshake(Handshake::Unavailable { reason, detail }))) => {
                Err(Rejected::Answered(PoseError::Unavailable {
                    reason,
                    detail,
                }))
            }
            // **Not reachable today, and deliberately not a panic.** `read_messages` parses the
            // first line as a `Handshake`, so a worker that got clause 3's order wrong sends a
            // line that is not one and arrives on the arm below instead — indistinguishable, from
            // here, from a program that simply printed. An `unreachable!()` would put a panic on a
            // process boundary whose whole job is to survive what the other side does.
            Ok(Ok(Incoming::Reply(reply))) => {
                proc.reap();
                Err(Rejected::NotThisOne(format!(
                    "a reply about job {:?} arrived before the handshake",
                    reply.job_id()
                )))
            }
            // A line that is not a message. Any program can print, and this is the main way a
            // candidate that is not our worker announces itself, so the search moves on. It is also
            // where a worker that replied before handshaking lands, for the reason just above.
            Ok(Err(err)) => {
                proc.reap();
                Err(Rejected::NotThisOne(err.to_string()))
            }
            // Died without speaking. `ModuleNotFoundError` is what this looks like on a dev box, and
            // the stderr tail is the sentence that says so.
            Err(RecvTimeoutError::Disconnected) => {
                let status = proc.reap();
                Err(Rejected::NotThisOne(format!(
                    "{}: {}",
                    exit_detail(status),
                    one_line(&proc.stderr_tail())
                )))
            }
            // A wedged startup is an answer: trying two more interpreters would spend two more
            // handshake timeouts to learn the same thing.
            Err(RecvTimeoutError::Timeout) => {
                proc.reap();
                Err(Rejected::Answered(PoseError::Spawn {
                    program: program.to_path_buf(),
                    detail: format!(
                        "no handshake within {:.0}s",
                        config.handshake_timeout.as_secs_f64()
                    ),
                }))
            }
        }
    }

    /// `pose_estimator_name(variant)` as this worker announced it, e.g. `mediapipe:heavy`.
    pub fn estimator(&self) -> &str {
        &self.estimator
    }

    /// The pose bundle this worker serves, and the only one it will accept a job for.
    pub fn variant(&self) -> &str {
        &self.variant
    }

    /// The `.task` bundle the handshake named, verified present by the worker at startup.
    pub fn model(&self) -> &Path {
        &self.model
    }

    /// Which interpreter answered — the one fact [`interpreter_candidates`]' fallback makes worth
    /// asking for.
    pub fn program(&self) -> &Path {
        &self.program
    }

    /// Hand this worker one job and wait for clause 4's two replies.
    ///
    /// Blocks for as long as the clip deserves: [`Config::accept_timeout`] for the acknowledgement,
    /// then [`Config::work_deadline`] derived from the frame count that acknowledgement carried.
    ///
    /// **A failure can arrive with no acknowledgement before it**, and that is not an error:
    /// `clip_unreadable`, `bad_job` and `frame_range_unsupported` are all decided before or instead
    /// of opening a container, so [`Reply::Failed`] is a legal first reply (P3's finding). A `done`
    /// with no acknowledgement is not legal, because then no frame count ever arrived and the
    /// deadline it was judged against was never the right one.
    ///
    /// Every error path kills the child. Clause 5's retry is a retry **on a fresh worker**, so a
    /// worker that has failed is spent: `Err` from this method means P5's pool replaces it rather
    /// than asking it again.
    pub fn run(&mut self, job: &Job) -> Result<Outcome, PoseError> {
        self.write(job)?;

        let accept_timeout = self.config.accept_timeout;
        let frames = match self.next_reply(accept_timeout, None, &job.job_id)? {
            Reply::Accepted { frames, .. } => frames,
            Reply::Failed { reason, detail, .. } => return Ok(Outcome::Failed { reason, detail }),
            Reply::Done { .. } => {
                return Err(self.corrupt("a `done` arrived with no acceptance before it"))
            }
        };

        let deadline = self.config.work_deadline(frames);
        match self.next_reply(deadline, Some(frames), &job.job_id)? {
            Reply::Done { keypoints, .. } => Ok(Outcome::Done(keypoints)),
            Reply::Failed { reason, detail, .. } => Ok(Outcome::Failed { reason, detail }),
            Reply::Accepted { .. } => Err(self.corrupt("a second acceptance arrived for one job")),
        }
    }

    /// Kill this worker's child and reap it. Idempotent, and `Drop` does it too.
    pub fn kill(&mut self) {
        self.proc.reap();
    }

    /// A [`Handle`] on this worker's child, for a thread that does not own the worker. [M23 P5]
    ///
    /// `Pool` holds one per slot so that `shutdown` can close every stdin at once and kill whatever
    /// is left, rather than waiting on a driver thread that is blocked inside [`Self::run`] on a
    /// deadline measured in minutes.
    pub fn handle(&self) -> Handle {
        self.proc.handle.clone()
    }

    fn write(&mut self, job: &Job) -> Result<(), PoseError> {
        // A broken pipe here is the child having gone rather than an I/O fault worth its own
        // variant: the write is the first thing to notice a worker that died while idle.
        match self.proc.handle.send(job) {
            Some(Ok(())) => Ok(()),
            Some(Err(PoseError::Io(err))) => {
                Err(self.died(&format!("writing a job failed: {err}")))
            }
            Some(Err(other)) => Err(other),
            None => Err(self.died("its stdin is already closed")),
        }
    }

    /// The next reply about the job in flight, or the typed reason there will not be one.
    ///
    /// `frames` is `Some` once the acknowledgement has arrived, and it decides which of clause 6's
    /// two timeouts this is. They mean different things: one says *wedged*, the other says *slower
    /// than three times the rate it was measured at, on work it has already accepted*.
    fn next_reply(
        &mut self,
        timeout: Duration,
        frames: Option<i64>,
        job_id: &str,
    ) -> Result<Reply, PoseError> {
        match self.proc.incoming.recv_timeout(timeout) {
            Ok(Ok(Incoming::Reply(reply))) => {
                self.attribute(&reply, job_id)?;
                Ok(reply)
            }
            Ok(Ok(Incoming::Handshake(_))) => {
                Err(self.corrupt("a second handshake arrived mid-job"))
            }
            // A corrupted channel is not recoverable by reading more of it, so the worker goes with
            // it — but the error reported is the protocol violation and not a crash, because *what
            // the line said* is the diagnosis, and "the worker died" would throw it away.
            Ok(Err(err)) => {
                self.proc.reap();
                Err(err)
            }
            Err(RecvTimeoutError::Disconnected) => {
                Err(self.died("stdout closed with a job in flight"))
            }
            Err(RecvTimeoutError::Timeout) => {
                self.proc.reap();
                Err(match frames {
                    Some(frames) => PoseError::WorkTimeout {
                        after: timeout,
                        frames,
                    },
                    None => PoseError::AcceptTimeout { after: timeout },
                })
            }
        }
    }

    /// Clause 4's `job_id` echo, checked — which is what makes a reply attributable.
    ///
    /// One exception, and P3 measured it: **a `bad_job` from a line that would not parse carries an
    /// empty `job_id`**, because there was nothing to echo. A worker owns one job at a time, so it
    /// is attributed to the job in flight rather than refused as unattributable; refusing it would
    /// turn a readable failure back into the silent one it was written to replace.
    fn attribute(&mut self, reply: &Reply, job_id: &str) -> Result<(), PoseError> {
        let id = reply.job_id();
        if id == job_id || (id.is_empty() && matches!(reply, Reply::Failed { .. })) {
            return Ok(());
        }
        let complaint = format!("a reply about job {id:?} arrived while {job_id:?} was in flight");
        Err(self.corrupt(&complaint))
    }

    /// The channel said something that is not this protocol. Kills the worker and names what it saw.
    fn corrupt(&mut self, what: &str) -> PoseError {
        self.proc.reap();
        PoseError::Protocol(what.to_string())
    }

    /// The child is gone. Reaps it, and attaches the exit status and the stderr tail it left.
    fn died(&mut self, what: &str) -> PoseError {
        let status = self.proc.reap();
        PoseError::Crashed {
            detail: format!("{what} ({})", exit_detail(status)),
            stderr: self.proc.stderr_tail(),
        }
    }
}

fn exit_detail(status: Option<ExitStatus>) -> String {
    match status {
        Some(status) => match status.code() {
            Some(code) => format!("exit code {code}"),
            None => format!("killed by a signal ({status})"),
        },
        None => "exit status unavailable".to_string(),
    }
}

/// A stderr tail folded onto one line, for an error message that is itself one line.
fn one_line(text: &str) -> String {
    let joined = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" / ");
    if joined.is_empty() {
        "no stderr".to_string()
    } else {
        joined
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clause_sixs_work_deadline_is_the_two_numbers_the_adr_quotes() {
        let config = Config::default();
        // 344 / 9.2 * 3 for the shortest corpus clip; the ADR says ~112 s.
        assert_eq!(config.work_deadline(344).as_secs(), 112);
        // 4,837 / 9.2 * 3 for the longest; the ADR says ~26 minutes.
        assert_eq!(config.work_deadline(4837).as_secs() / 60, 26);
    }

    #[test]
    fn a_short_clip_gets_the_floor_and_a_zero_frame_ack_cannot_derive_zero() {
        let config = Config::default();
        assert_eq!(config.work_deadline(0), DEFAULT_WORK_FLOOR);
        assert_eq!(config.work_deadline(10), DEFAULT_WORK_FLOOR);
        // And the floor is not a ceiling: the moment the derivation clears it, it wins.
        assert!(config.work_deadline(344) > DEFAULT_WORK_FLOOR);
    }

    #[test]
    fn a_misconfigured_estimate_derives_the_floor_rather_than_an_eternity() {
        // `frames / 0.0` is an infinity, and `Duration::from_secs_f64` panics on one.
        for fps_estimate in [0.0, -9.2, f64::NAN] {
            let config = Config {
                fps_estimate,
                ..Config::default()
            };
            assert_eq!(config.work_deadline(4837), DEFAULT_WORK_FLOOR);
        }
    }

    #[test]
    fn a_lite_pool_raising_the_estimate_shortens_every_deadline() {
        // ADR-002 puts `lite` at roughly 4x `heavy`, and clause 6 makes that configuration rather
        // than a constant precisely so this is a field and not an edit.
        let lite = Config {
            fps_estimate: DEFAULT_FPS_ESTIMATE * 4.0,
            ..Config::default()
        };
        assert_eq!(lite.work_deadline(4837).as_secs(), 394);
    }

    #[test]
    fn the_candidate_list_is_clause_nines_order_and_holds_no_duplicates() {
        let explicit = PathBuf::from("/nowhere/python");
        let config = Config {
            python: Some(explicit.clone()),
            ..Config::default()
        };
        let candidates = interpreter_candidates(&config);
        assert_eq!(candidates.first(), Some(&explicit));
        let mut sorted = candidates.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), candidates.len(), "{candidates:?}");
    }

    #[test]
    fn a_missing_interpreter_reports_the_names_it_searched_for() {
        let looked = searched(&Config::default());
        assert!(looked.contains(&PathBuf::from("python")));
        assert!(looked.contains(&PathBuf::from("python3")));
    }

    #[test]
    fn an_exit_status_reads_as_a_sentence_even_when_there_is_none() {
        assert_eq!(exit_detail(None), "exit status unavailable");
    }

    #[test]
    fn a_stderr_tail_folds_onto_one_line_and_says_so_when_it_is_empty() {
        assert_eq!(one_line("  \n\n  "), "no stderr");
        assert_eq!(
            one_line("Traceback (most recent call last):\n  ModuleNotFoundError: golf_coach\n"),
            "Traceback (most recent call last): / ModuleNotFoundError: golf_coach"
        );
    }
}
