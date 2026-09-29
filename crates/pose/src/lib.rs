//! The pose sidecar boundary: how the Rust core asks Python for landmarks. [M23 P2]
//!
//! Seventh crate in this workspace, and the one [ADR-033](../../../docs/decisions/033-the-pose-sidecar-protocol.md)
//! decides into existence. Pose is the one thing that stays Python — ADR-030 §2 keeps MediaPipe
//! there and §3 puts it in *"a long-lived, bundled worker pool"* — so this crate is the other side
//! of that pipe: a job out, a `KeypointsFile` back, and no MediaPipe anywhere in Rust, now or later.
//!
//! # What is here, and what is not yet
//!
//! [`protocol`] is the vocabulary — the message shapes and the line framing, knowing nothing about
//! who is on the other end (M23 P2). [`worker`] is one child process and the threads that talk to
//! it: interpreter resolution, the handshake, one job at a time under clause 6's two timeouts, and
//! a typed error for every way a child can fail (M23 P4). [`pool`] is N of those behind a bounded
//! queue, with clause 5's retry on a *fresh* worker and a shutdown that terminates (M23 P5).
//! [`writer`] is clause 8's other half — the caller writing `{camera_id}.keypoints.json`, since the
//! worker decides no path — and the `golf-pose` binary drives one clip through all four (M23 P6).
//!
//! **Nothing calls any of it**: M22 built a whole analysis engine wired to nothing and this is the
//! other half of what M24 needs, so M24 is the milestone that gives both a caller at once.
//!
//! [`PoseError`] is here already, ahead of the code that raises most of it, because clause 5's retry
//! table spans both halves: the reasons a *worker* reports are
//! [`protocol::FailureReason`], and the pool's own — crash, EOF, non-zero exit, `work_timeout` — are
//! this enum's. Splitting the table across two phases would have left half of it unwritten and
//! neither half obviously incomplete.
//!
//! # The invariant this does not break
//!
//! `CLAUDE.md` binds the analysis core to stdlib plus `contracts`, and ADR-030 §8 extends that to the
//! Rust core's scoring path. This crate is a process boundary, not a scoring path: it depends on
//! `contracts` for the `KeypointsFile` it carries and on serde to put it on a wire. **No numeric
//! library between a measurement and a verdict** is the rule that was always meant, and it holds —
//! nothing here computes anything.

pub mod pool;
pub mod protocol;
pub mod worker;
pub mod writer;

pub use pool::{Completed, Pool, PoolConfig};
pub use protocol::{FailureReason, Handshake, Job, Lines, Reply, PROTOCOL_VERSION};
pub use worker::{Config, Handle, Outcome, Worker};
// `writer::write` and `writer::to_json` are deliberately left qualified: `pose::write` reads as a
// verb about posing rather than about a file, and the module name is what says which it is.
pub use writer::{keypoints_path, KEYPOINTS_SUFFIX};

use std::fmt;
use std::io;
use std::path::PathBuf;
use std::time::Duration;

/// A failure of the pool's own, as against a [`FailureReason`] a worker reported about a clip.
///
/// The split matters because the two halves reach a caller differently: a `FailureReason` is a fact
/// about a *clip* that the pool passes along, and one of these is a fact about the *sidecar* — which
/// for [`Self::InterpreterNotFound`] and [`Self::Unavailable`] means the machine cannot pose at all
/// and no clip will change that.
///
/// Every variant carries what it looked for or how long it waited, the way `capture::Error::Gone`
/// carries the device id: these are the strings a session prints when it stops, and a message about
/// a fact is worth less than the fact.
#[derive(Debug)]
pub enum PoseError {
    /// No Python answered. Carries the list that was tried, because **the fix is in the list** —
    /// the same reason `audio/trigger.py`'s `TriggerUnavailable` carries "run `cargo build
    /// --release`" rather than "the binary is missing". Clause 9's resolution order is config, then
    /// `PATH`, then the known `.venv` location.
    InterpreterNotFound { looked_for: Vec<PathBuf> },
    /// An interpreter was found and would not start.
    Spawn { program: PathBuf, detail: String },
    /// The handshake said this worker cannot pose (clause 3). A pool-level failure at startup, never
    /// a failed job: a machine with no `.task` bundle says so when the session starts rather than 30
    /// seconds into the golfer's first swing.
    Unavailable { reason: String, detail: String },
    /// The worker speaks a protocol this build does not. Refused on the handshake line rather than
    /// discovered as a missing field eight minutes into a clip.
    ProtocolMismatch { theirs: u32, ours: u32 },
    /// No `accepted` line inside the flat accept timeout (clause 6). Opening a container does not
    /// scale with clip length, so a worker that has not acked is **wedged, not busy**.
    AcceptTimeout { after: Duration },
    /// The derived work deadline passed: `max(FLOOR, frames / fps_estimate * MARGIN)`. Carries the
    /// frame count it was derived from, because "timed out after 112s" is unreadable without it.
    WorkTimeout { after: Duration, frames: i64 },
    /// EOF on stdout, or a non-zero exit. Carries the tail of stderr the pool kept, which is what
    /// makes this actionable instead of a shrug (clause 1).
    Crashed { detail: String, stderr: String },
    /// A line arrived that is not a message, or is a message about nothing that was asked for.
    /// [`Worker`] kills the child on one: stdout is protocol, so a corrupted channel is not
    /// recoverable by reading more of it. The error reported is this and not [`Self::Crashed`],
    /// because *what the line said* is the diagnosis and "the worker died" would throw it away.
    Protocol(String),
    /// The pool's `submit` refused because the queue is at its bound (clause 7). **A value, not a
    /// block**: backpressure the session engine can act on — drop the analysis, tell the golfer the
    /// backlog is full, stop cutting clips — where an unbounded queue is that decision made
    /// silently and badly, one swing at a time.
    QueueFull { bound: usize },
    /// A job submitted to a pool that has shut down, or one whose worker died as the pool was
    /// closing. [M23 P5]
    ///
    /// **A variant clause 7 does not name**, and it exists because the alternative was to answer a
    /// late `submit` with [`Self::QueueFull`] — which is a bound the caller could wait out, and this
    /// is not. A pool is over once it is shut down; the jobs it never started are handed back by
    /// `Pool::shutdown` rather than reported here.
    ShutDown,
    /// The pipe itself. Distinct from [`Self::Crashed`] because an I/O error on a live worker is not
    /// yet evidence the worker died.
    Io(io::Error),
}

impl PoseError {
    /// Whether clause 5 retries the job this failed on, once, on a fresh worker.
    ///
    /// The other half of clause 5's table; [`FailureReason::retried`] is the worker-reported half,
    /// and between them they are the whole of it.
    ///
    /// **[`Self::AcceptTimeout`] is not a row in that table**, and this answers `true` on the crash
    /// row's reasoning — a process that never acked is killed and replaced, which is a transient
    /// fault in a long-lived C++ graph by another name. Opened as a P2 finding, left standing by P4,
    /// and **P5 kept it**: the pool that implements the retry is built, it branches on this method,
    /// and a wedged startup that is *not* retried would fail a clip for a fault in the worker rather
    /// than in the clip. P4 added one neighbour — [`Self::Spawn`] also covers a startup that never
    /// handshook, which is a fact about the machine and is not retried.
    pub fn retried(&self) -> bool {
        match self {
            // A transient fault in a long-lived C++ graph, and the box may simply have been busy.
            Self::Crashed { .. } | Self::WorkTimeout { .. } | Self::AcceptTimeout { .. } => true,
            // A corrupted channel is not fixed by reading more of it, but the *process* might be —
            // P4 reports it as a crash, so the crash row governs what happens to the job.
            Self::Protocol(_) => true,
            // Facts about the machine. No number of fresh workers installs an interpreter, grows a
            // model bundle or reconciles two protocol versions.
            Self::InterpreterNotFound { .. }
            | Self::Spawn { .. }
            | Self::Unavailable { .. }
            | Self::ProtocolMismatch { .. } => false,
            // Never reached a worker at all, and retrying a refusal is how a bound stops being one.
            // A pool that has shut down is not going to answer a second time either.
            Self::QueueFull { .. } | Self::ShutDown => false,
            // Unclassifiable without knowing what failed, and guessing "retry" on an unknown I/O
            // error is how a job runs twice for nothing.
            Self::Io(_) => false,
        }
    }
}

impl fmt::Display for PoseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InterpreterNotFound { looked_for } => {
                write!(f, "no Python interpreter for the pose sidecar; looked for ")?;
                for (i, path) in looked_for.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", path.display())?;
                }
                Ok(())
            }
            Self::Spawn { program, detail } => {
                write!(f, "could not start {}: {detail}", program.display())
            }
            Self::Unavailable { reason, detail } => {
                write!(f, "the pose sidecar cannot pose ({reason}): {detail}")
            }
            Self::ProtocolMismatch { theirs, ours } => write!(
                f,
                "the pose sidecar speaks protocol {theirs}, this build speaks {ours}"
            ),
            Self::AcceptTimeout { after } => write!(
                f,
                "the pose sidecar did not acknowledge a job within {:.1}s",
                after.as_secs_f64()
            ),
            Self::WorkTimeout { after, frames } => write!(
                f,
                "posing {frames} frames did not finish within {:.0}s",
                after.as_secs_f64()
            ),
            Self::Crashed { detail, stderr } => {
                write!(f, "the pose sidecar died: {detail}")?;
                if !stderr.trim().is_empty() {
                    write!(f, "; last stderr: {}", stderr.trim())?;
                }
                Ok(())
            }
            Self::Protocol(detail) => write!(f, "the pose sidecar's channel is corrupt: {detail}"),
            Self::QueueFull { bound } => {
                write!(f, "the pose queue is full at its bound of {bound} jobs")
            }
            Self::ShutDown => write!(f, "the pose pool has shut down and takes no more jobs"),
            Self::Io(err) => write!(f, "the pose sidecar's pipe failed: {err}"),
        }
    }
}

impl std::error::Error for PoseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<io::Error> for PoseError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<serde_json::Error> for PoseError {
    /// A serialization failure on the way *out* — the only one this protocol can produce is a
    /// `clip_path` that is not valid UTF-8, which cannot cross a JSON channel at all. Parse failures
    /// on the way in come back as [`PoseError::Protocol`] instead, because a line that is not a
    /// message says something about the worker rather than about serde.
    fn from(err: serde_json::Error) -> Self {
        Self::Protocol(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pools_half_of_clause_fives_retry_table() {
        // Transcribed from ADR-033 clause 5. `AcceptTimeout` is the one row the ADR does not have —
        // see `PoseError::retried` for why this build answers `true` and what would change it.
        assert!(PoseError::Crashed {
            detail: "eof on stdout".to_string(),
            stderr: String::new()
        }
        .retried());
        assert!(PoseError::WorkTimeout {
            after: Duration::from_secs(112),
            frames: 344
        }
        .retried());
        assert!(!PoseError::Unavailable {
            reason: "model_absent".to_string(),
            detail: String::new()
        }
        .retried());
        assert!(!PoseError::QueueFull { bound: 4 }.retried());
        // P5's addition to the pool's half, for the same reason the row above is in it: neither
        // failure ever reached a worker, so there is nothing about doing it again that could differ.
        assert!(!PoseError::ShutDown.retried());
        assert!(!PoseError::InterpreterNotFound {
            looked_for: Vec::new()
        }
        .retried());
    }

    #[test]
    fn a_missing_interpreter_says_where_it_looked() {
        // The fix is the list, which is the whole reason the variant carries one.
        let err = PoseError::InterpreterNotFound {
            looked_for: vec![
                PathBuf::from(".venv/Scripts/python.exe"),
                PathBuf::from("python"),
            ],
        };
        let message = err.to_string();
        assert!(message.contains(".venv/Scripts/python.exe"), "{message}");
        assert!(message.contains("python"), "{message}");
    }

    #[test]
    fn a_crash_carries_the_stderr_that_explains_it() {
        let err = PoseError::Crashed {
            detail: "exit code 1".to_string(),
            stderr: "  RuntimeError: graph closed\n".to_string(),
        };
        assert!(err.to_string().contains("RuntimeError: graph closed"));
        // And says nothing about stderr when there was none, rather than trailing an empty clause.
        let quiet = PoseError::Crashed {
            detail: "eof on stdout".to_string(),
            stderr: "\n  \n".to_string(),
        };
        assert!(!quiet.to_string().contains("stderr"));
    }

    #[test]
    fn a_work_timeout_says_what_it_was_derived_from() {
        // `frames / 9.2 * 3` for the longest corpus clip, which is the number clause 6 quotes.
        let err = PoseError::WorkTimeout {
            after: Duration::from_secs_f64(4837.0 / 9.2 * 3.0),
            frames: 4837,
        };
        assert_eq!(
            err.to_string(),
            "posing 4837 frames did not finish within 1577s"
        );
    }
}
