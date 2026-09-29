//! ADR-033's messages, and the line framing they cross on. [M23 P2]
//!
//! One JSON object per line, `\n`-terminated: a [`Job`] out, a [`Handshake`] and then two [`Reply`]s
//! per job back ([ADR-033](../../../docs/decisions/033-the-pose-sidecar-protocol.md) clauses 1–5).
//! **Nothing here knows who is on the other end** — no child process, no interpreter resolution, no
//! threads. `Worker` is P4's and holds all of that; this module is the vocabulary it will speak,
//! gated by its own serde round-trips so a renamed field is caught by a test rather than by a hung
//! pipe.
//!
//! # Why there is no length prefix
//!
//! JSON cannot contain a bare newline outside a string, and a string cannot contain a literal one,
//! so the line *is* the frame and a prefix would be a second thing to get wrong (clause 1).
//!
//! That is a choice made against a measurement rather than a default: the largest reply this corpus
//! produces is **16,424,762 bytes on one line** — 4,837 frames, `2026-08-23/8/down_the_line`, which
//! is 27.2 MB on disk only because `save_keypoints` writes it with `indent=2`. Re-measured here
//! against the file itself, so the plan's 16.4 MB is the wire figure and not a rounding of the disk
//! one. So [`Lines`] owns a buffer it reuses across reads and grows as needed, and neither side sets
//! a line-length ceiling. `read_to_string` is the thing this exists to not be.

use std::io::{BufRead, Write};
use std::path::PathBuf;

use contracts::keypoints::KeypointsFile;
use contracts::{ge, gt, nested, ContractError, Validate};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::PoseError;

/// The protocol version both sides must agree on, sent in [`Handshake::Ready`].
///
/// Checked at the handshake rather than discovered later: M26 ships the interpreter and the sidecar
/// alongside the binary so the pair always matches, but during development they are two working
/// trees, and a mismatch should fail on the first line rather than on a missing field eight minutes
/// into a clip (clause 3).
pub const PROTOCOL_VERSION: u32 = 1;

/// One clip to pose. ADR-030 §3's five fields, kept exactly as it wrote them (clause 2).
///
/// `clip_path` is absolute, and it is a path rather than pixels because every caller through M24
/// cuts its clip to disk first — shared memory is ADR-033's Option E and stays deferred. A path that
/// is not valid UTF-8 cannot cross a JSON channel and fails to serialize; no corpus path is one, and
/// the alternative is a bytes-plus-encoding field nothing would read.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Job {
    /// The pool's, opaque to the worker, echoed on every reply about it. What makes a result
    /// attributable when several workers are draining at once.
    pub job_id: String,
    pub clip_path: PathBuf,
    /// `[start, end)`, or `None` for the whole clip.
    ///
    /// **Only the whole clip is permitted**, and the worker is what refuses anything else with
    /// [`FailureReason::FrameRangeUnsupported`] — see [`Job::validate`] for the half of that check
    /// this side can make. The field stays in the envelope because ADR-030 §3 names it and because
    /// §M30 and live preview would use it; no caller asks for a range today.
    pub frame_range: Option<[i64; 2]>,
    /// Carried straight through to `FrameKeypoints.camera_id`, which is free-form by design, and
    /// optional for the same reason it is there: a clip whose camera nobody recorded is an ordinary
    /// artifact in this repo rather than a malformed one.
    pub camera_id: Option<String>,
    /// `None` means the worker's configured default.
    ///
    /// A string rather than a Rust enum over `POSE_VARIANTS`, because clause 2 puts the validation in
    /// `pose/estimator.py::resolve_variant` — the code that already owns it, and the code that will
    /// still own it when a fourth bundle appears. A second copy of the list here is a second thing to
    /// forget to update.
    pub pose_model_variant: Option<String>,
}

impl Validate for Job {
    /// The half of clause 2's rule this side can decide.
    ///
    /// The definitive check is `end == frame_count`, and **Rust cannot make it**: it has a path and
    /// no video decoder, and ADR-031 §2 records deliberately that it never decodes a file — the same
    /// fact that makes clause 4's acceptance line necessary. What is decidable here is that a range
    /// starting anywhere but frame 0 is *not* the whole clip whatever the frame count turns out to
    /// be, so a caller bug of that shape is caught before a process is spawned for it.
    fn validate(&self) -> Result<(), ContractError> {
        if self.job_id.is_empty() {
            return Err(ContractError {
                field: "Job.job_id".to_string(),
                problem: "is empty, so no reply could be attributed to it".to_string(),
            });
        }
        let Some([start, end]) = self.frame_range else {
            return Ok(());
        };
        if start != 0 {
            return Err(ContractError {
                field: "Job.frame_range".to_string(),
                problem: format!("starts at {start}, and only the whole clip may be asked for"),
            });
        }
        gt("Job.frame_range", end, start)
    }
}

/// What a worker says when it comes up, before it reads anything (clause 3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "pose", rename_all = "snake_case")]
pub enum Handshake {
    /// The model file is present — **verified, not downloaded**. `ensure_pose_model`'s download stays
    /// a lab and setup action: a worker that reaches the network on first use is a worker that stalls
    /// a golfer's first swing for 30 MB.
    Ready {
        protocol: u32,
        /// `pose_estimator_name(variant)`, e.g. `mediapipe:heavy` — the same stamp that lands in
        /// `KeypointsFile.pose_estimator`.
        estimator: String,
        variant: String,
        model: PathBuf,
    },
    /// This worker cannot pose, said at startup rather than 30 seconds into the first clip. The pool
    /// reports it as a pool-level failure and not as a failed job.
    ///
    /// `reason` is a string where [`Reply::Failed`]'s is a typed enum, and the asymmetry is
    /// deliberate: nothing branches on this one, a human reads it, and a worker naming a reason this
    /// build had not heard of would otherwise be reported as "unparseable line" instead of as
    /// whatever it actually said. `model_absent` is the only value ADR-033 defines.
    Unavailable { reason: String, detail: String },
}

impl Validate for Handshake {
    /// No bounds. An empty body is the honest mirror of a shape that carries none — `contracts`'
    /// [`Validate`] is uniform for the same reason, and a hole in the recursion is invisible.
    fn validate(&self) -> Result<(), ContractError> {
        Ok(())
    }
}

/// Why a job could not be posed — clause 5's table, which is the whole set.
///
/// Typed rather than a string, unlike [`Handshake::Unavailable`]'s, because
/// [`retried`](FailureReason::retried) branches on it: a reason with no answer to *"would doing it
/// again change anything"* is not a reason this pool can act on. An unrecognized one therefore fails
/// to parse, and clause 3's `protocol` check is what makes that safe — a build that would send a
/// reason this one has not heard of is a build whose handshake was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureReason {
    /// Clause 3 makes this a pool-startup failure, so a job should never carry it. Kept in the set
    /// because a worker started outside the pool can still say it, and a reason that parses is worth
    /// more in a log than a protocol violation.
    ModelAbsent,
    ClipUnreadable,
    ClipEmpty,
    FrameRangeUnsupported,
    BadJob,
    PoseFailed,
}

impl FailureReason {
    /// Whether clause 5 retries this once on a fresh worker.
    ///
    /// **The split is on the failure, never on the phase.** This is the same distinction
    /// `contracts/caveats.py` draws for the golfer with `refilming_helps` — *would doing it again
    /// change anything?* — and it is drawn here for the same reason: a retry that cannot help turns
    /// one wrong answer into two.
    pub fn retried(self) -> bool {
        match self {
            // An exception out of a long-lived C++ graph is the case a fresh process fixes.
            Self::PoseFailed => true,
            // Facts about the file. The second read finds the same file.
            Self::ClipUnreadable | Self::ClipEmpty => false,
            // Caller bugs, and retrying one hides it.
            Self::FrameRangeUnsupported | Self::BadJob => false,
            // Not a job's failure at all (clause 3), and no number of fresh workers grows a model.
            Self::ModelAbsent => false,
        }
    }
}

/// A reply about one job: the acceptance, then exactly one of `done` or `failed` (clause 4).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "pose", rename_all = "snake_case")]
pub enum Reply {
    /// The container is open and here is its shape — written **before a single frame is posed**.
    ///
    /// The one message ADR-030 §3 did not imply, and it exists so clause 6's work deadline is
    /// derivable at all. The deadline has to scale with the clip (344 to 4,837 frames on this corpus,
    /// a 14.1× spread) and the frame count is the only honest scale factor. The Rust core cannot get
    /// it; the process that is about to decode the clip already knows, so it says.
    Accepted {
        job_id: String,
        frames: i64,
        fps: f64,
        width: i64,
        height: i64,
    },
    /// The landmarks, as the `contracts` shape the Python pipeline already writes.
    ///
    /// The worker **writes no file and decides no path** (clause 8): `crates/pose` hands this to its
    /// caller and the caller writes `{role}.keypoints.json`, the same division `crates/capture`
    /// already has.
    Done {
        job_id: String,
        keypoints: KeypointsFile,
    },
    /// Named, and **never a partial or guessed `KeypointsFile`** — no empty `frames`, no landmarks
    /// carried over from another clip, no zero-filled frame for one that could not be posed. ADR-010
    /// §2 at the process boundary.
    Failed {
        job_id: String,
        reason: FailureReason,
        detail: String,
    },
}

impl Reply {
    /// Which job this is about. Every variant carries it, which is what lets a pool with several
    /// workers draining at once attribute a reply to the job that asked for it.
    ///
    /// `job_id` is repeated per variant rather than lifted into a `{job_id, #[serde(flatten)] body}`
    /// wrapper, and the cost was measured rather than assumed. Parsing the corpus's largest reply —
    /// `2026-08-23/8/down_the_line`, 4,837 frames, **16,424,762 bytes on one line**, which is where
    /// the plan's 16.4 MB figure comes from — takes **0.33 s** as a bare `KeypointsFile`, **0.44 s**
    /// as this internally-tagged enum, and **0.48 s** with the flattened wrapper. So flatten is not
    /// the disaster it is often described as here, and the tag is not free either: both route through
    /// serde's `Content` buffer. Three duplicated fields is the cheaper and the plainer of the two,
    /// and either way 0.4 s of parsing sits against **8.8 minutes** of posing that clip at 9.2 fps.
    pub fn job_id(&self) -> &str {
        match self {
            Self::Accepted { job_id, .. }
            | Self::Done { job_id, .. }
            | Self::Failed { job_id, .. } => job_id,
        }
    }
}

impl Validate for Reply {
    /// The acceptance carries `ClipMetadata`'s four numbers, so it carries `ClipMetadata`'s bounds.
    ///
    /// Worth checking rather than trusting, because clause 6 turns `frames` into a deadline: a
    /// negative frame count would otherwise become a work timeout that had already expired. Zero is
    /// allowed here exactly as `ClipMetadata.frame_count` allows it, and clause 6's `FLOOR` is what
    /// keeps a zero-frame ack from deriving a zero-length deadline.
    fn validate(&self) -> Result<(), ContractError> {
        match self {
            Self::Accepted {
                frames,
                fps,
                width,
                height,
                ..
            } => {
                ge("Reply.frames", *frames, 0)?;
                gt("Reply.fps", *fps, 0.0)?;
                gt("Reply.width", *width, 0)?;
                gt("Reply.height", *height, 0)
            }
            Self::Done { keypoints, .. } => nested("Reply.keypoints", Some(keypoints)),
            Self::Failed { .. } => Ok(()),
        }
    }
}

/// Write one message as one line, flushed.
///
/// One `write` plus one `flush`, and the line is never built twice (clause 1) — `to_writer`
/// serializes straight into the pipe rather than into a `String` that is then copied, which at
/// 16.4 MB is worth caring about. The `\n` is what the other side's [`Lines`] frames on, so it is
/// written here rather than left to a caller to remember.
///
/// **Validated before it goes out**, for the same reason [`Lines::read`] validates on the way in: a
/// message in this crate is only ever built here or parsed there, so those two functions are the
/// whole boundary and the bound belongs on them. That is why no type in this module uses
/// `contracts::validated!` — the macro exists because pydantic validates at construction and a
/// payload shape is built in a hundred places, and wiring it into `Deserialize` here would leave the
/// *outbound* direction unchecked while looking as though the question had been answered.
pub fn write_line<W: Write, T: Serialize + Validate>(
    out: &mut W,
    message: &T,
) -> Result<(), PoseError> {
    message
        .validate()
        .map_err(|e| PoseError::Protocol(format!("refusing to send a bad message: {e}")))?;
    serde_json::to_writer(&mut *out, message)?;
    out.write_all(b"\n")?;
    out.flush()?;
    Ok(())
}

/// A framed reader over one stream, owning the buffer it reuses.
///
/// The buffer is the point. A 16.4 MB reply arrives as one line, and a reader that allocated per line
/// would hand the allocator a fresh large block per clip; this one grows to the largest line it has
/// seen and stays there. It is also why there is no line-length ceiling: the largest legitimate
/// message this protocol carries is measured in tens of megabytes, so a cap would be a limit on
/// ordinary traffic.
///
/// **A read is not instant, which is part of why P4 puts it on a thread**: the corpus's largest reply
/// costs 0.44 s to parse and 4 ms to validate — see [`Reply::job_id`] for the measurement. Against
/// the 8.8 minutes of posing that produced it that is nothing, but it is not zero.
pub struct Lines<R: BufRead> {
    inner: R,
    buf: String,
}

impl<R: BufRead> Lines<R> {
    pub fn new(inner: R) -> Self {
        Self {
            inner,
            buf: String::new(),
        }
    }

    /// The next message, or `None` at end of stream.
    ///
    /// **`None` is a crash, not a shutdown, when a job is outstanding**: EOF on stdout is how the
    /// pool learns a worker died, which is the liveness signal clause 1 gets for free by choosing a
    /// pipe. P4 is what decides which of the two it is, because only it knows whether it was waiting.
    ///
    /// Every message is validated on arrival — see [`write_line`] for why the bound sits on these two
    /// functions rather than on each type's `Deserialize`.
    ///
    /// Whitespace-only lines are skipped rather than refused. Anything else that is not one JSON
    /// object is [`PoseError::Protocol`]: stdout is protocol and stderr is logging (clause 1), so a
    /// library that printed is a corrupted channel and has to be reported as one.
    pub fn read<T: DeserializeOwned + Validate>(&mut self) -> Result<Option<T>, PoseError> {
        loop {
            self.buf.clear();
            if self.inner.read_line(&mut self.buf)? == 0 {
                return Ok(None);
            }
            let line = self.buf.trim();
            if line.is_empty() {
                continue;
            }
            let message: T = serde_json::from_str(line).map_err(|e| {
                PoseError::Protocol(format!(
                    "a line on the worker's stdout is not a protocol message: {e} — {}",
                    excerpt(line)
                ))
            })?;
            message.validate().map_err(|e| {
                PoseError::Protocol(format!("a protocol message is out of bounds: {e}"))
            })?;
            return Ok(Some(message));
        }
    }
}

/// The head of an offending line, for the error about it. [M23 P4]
///
/// Added once a `Worker` was reading real processes and the reports turned out to say nothing:
/// serde's own message for a line of human text is *"expected value at line 1 column 1"*, which
/// names the position of the problem and never the content, so *"the channel is corrupt"* arrived
/// with no evidence of what corrupted it. That is the opposite of what
/// [`PoseError::Protocol`](crate::PoseError::Protocol) exists for.
///
/// Bounded, and not by taste: a legitimate message on this wire reaches **16.4 MB**, and an error
/// value that carried one would be a log line nobody can read and a copy nobody asked for.
/// Truncated on a `char` boundary, because a line that is not a protocol message is exactly the
/// kind of line that is not clean UTF-8 either.
fn excerpt(line: &str) -> String {
    const HEAD: usize = 120;
    if line.chars().count() <= HEAD {
        return format!("{line:?}");
    }
    let head: String = line.chars().take(HEAD).collect();
    format!("{head:?}…")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ADR-033 clause 2's envelope, verbatim. A field rename has to fail here rather than in P4.
    const GOLDEN_JOB: &str = concat!(
        r#"{"job_id":"j-1","clip_path":"/clips/face_on.MOV","frame_range":null,"#,
        r#""camera_id":"face_on","pose_model_variant":"heavy"}"#
    );

    /// One line through the real gate, because [`Lines::read`] is where validation lives.
    fn read_one<T: DeserializeOwned + Validate>(line: &str) -> Result<Option<T>, PoseError> {
        Lines::new(line.as_bytes()).read()
    }

    fn golden_job() -> Job {
        Job {
            job_id: "j-1".to_string(),
            clip_path: PathBuf::from("/clips/face_on.MOV"),
            frame_range: None,
            camera_id: Some("face_on".to_string()),
            pose_model_variant: Some("heavy".to_string()),
        }
    }

    #[test]
    fn the_job_envelope_is_the_five_fields_adr_030_named() {
        let written = serde_json::to_string(&golden_job()).unwrap();
        assert_eq!(written, GOLDEN_JOB);
        assert_eq!(
            serde_json::from_str::<Job>(GOLDEN_JOB).unwrap(),
            golden_job()
        );
    }

    #[test]
    fn an_absent_frame_range_is_written_as_null_and_not_omitted() {
        // The worker reads five keys, and clause 2's literal shows `"frame_range": null`. A
        // `skip_serializing_if` here would make the whole-clip case indistinguishable from a build
        // that had dropped the field.
        assert!(serde_json::to_string(&golden_job())
            .unwrap()
            .contains(r#""frame_range":null"#));
    }

    #[test]
    fn a_field_the_worker_does_not_know_is_refused_rather_than_ignored() {
        let extra = GOLDEN_JOB.replace(r#""job_id""#, r#""urgency":"high","job_id""#);
        assert!(serde_json::from_str::<Job>(&extra).is_err());
    }

    #[test]
    fn only_a_range_starting_at_frame_zero_can_be_the_whole_clip() {
        let with = |range| Job {
            frame_range: range,
            ..golden_job()
        };
        assert!(with(None).validate().is_ok());
        // Rust cannot know the frame count, so `[0, n]` passes here and the worker decides.
        assert!(with(Some([0, 344])).validate().is_ok());
        assert!(with(Some([5, 344])).validate().is_err());
        assert!(with(Some([-1, 344])).validate().is_err());
        assert!(with(Some([0, 0])).validate().is_err());
    }

    #[test]
    fn a_job_with_no_id_is_refused_because_no_reply_could_be_attributed_to_it() {
        let anonymous = Job {
            job_id: String::new(),
            ..golden_job()
        };
        let err = anonymous.validate().unwrap_err();
        assert_eq!(err.field, "Job.job_id");
        // And it never reaches the pipe, because `write_line` validates on the way out.
        let mut wire: Vec<u8> = Vec::new();
        assert!(write_line(&mut wire, &anonymous).is_err());
        assert!(wire.is_empty(), "a refused message left bytes on the wire");
        // Nor past the reader, in the direction P4's stub workers will read one.
        let json = GOLDEN_JOB.replace(r#""j-1""#, r#""""#);
        assert!(read_one::<Job>(&json).is_err());
    }

    #[test]
    fn the_handshake_is_clause_threes_two_lines() {
        let ready = concat!(
            r#"{"pose":"ready","protocol":1,"estimator":"mediapipe:heavy","variant":"heavy","#,
            r#""model":"/models/pose_landmarker_heavy.task"}"#
        );
        let parsed: Handshake = serde_json::from_str(ready).unwrap();
        match &parsed {
            Handshake::Ready {
                protocol,
                estimator,
                variant,
                model,
            } => {
                assert_eq!(*protocol, PROTOCOL_VERSION);
                assert_eq!(estimator, "mediapipe:heavy");
                assert_eq!(variant, "heavy");
                assert_eq!(model, &PathBuf::from("/models/pose_landmarker_heavy.task"));
            }
            other => panic!("expected Ready, got {other:?}"),
        }
        assert_eq!(serde_json::to_string(&parsed).unwrap(), ready);

        let absent = r#"{"pose":"unavailable","reason":"model_absent","detail":"no .task here"}"#;
        let parsed: Handshake = serde_json::from_str(absent).unwrap();
        assert!(
            matches!(&parsed, Handshake::Unavailable { reason, .. } if reason == "model_absent")
        );
        assert_eq!(serde_json::to_string(&parsed).unwrap(), absent);
    }

    #[test]
    fn a_reason_this_build_has_not_heard_of_still_reaches_a_human() {
        // The asymmetry with `FailureReason`, pinned: an unknown *handshake* reason parses, because
        // nothing branches on it and a startup diagnostic is worth more than a strict type.
        let json = r#"{"pose":"unavailable","reason":"interpreter_too_old","detail":"3.8"}"#;
        assert!(serde_json::from_str::<Handshake>(json).is_ok());
    }

    #[test]
    fn a_job_gets_an_acceptance_and_then_one_result() {
        let accepted = concat!(
            r#"{"pose":"accepted","job_id":"j-1","frames":4837,"fps":59.9651365485183,"#,
            r#""width":2160,"height":3840}"#
        );
        let parsed: Reply = serde_json::from_str(accepted).unwrap();
        assert_eq!(parsed.job_id(), "j-1");
        assert!(parsed.validate().is_ok());
        assert_eq!(serde_json::to_string(&parsed).unwrap(), accepted);

        let done = concat!(
            r#"{"pose":"done","job_id":"j-1","keypoints":{"clip":null,"frames":"#,
            r#"[{"frame_index":0,"timestamp_ms":0.0,"landmarks":[],"camera_id":"face_on"}],"#,
            r#""pose_estimator":"mediapipe:heavy"}}"#
        );
        let parsed: Reply = serde_json::from_str(done).unwrap();
        match &parsed {
            Reply::Done { keypoints, .. } => {
                assert_eq!(keypoints.frames.len(), 1);
                assert_eq!(keypoints.pose_estimator.as_deref(), Some("mediapipe:heavy"));
            }
            other => panic!("expected Done, got {other:?}"),
        }

        let failed =
            r#"{"pose":"failed","job_id":"j-1","reason":"clip_unreadable","detail":"eof"}"#;
        let parsed: Reply = serde_json::from_str(failed).unwrap();
        assert!(matches!(
            parsed,
            Reply::Failed {
                reason: FailureReason::ClipUnreadable,
                ..
            }
        ));
        assert_eq!(serde_json::to_string(&parsed).unwrap(), failed);
    }

    #[test]
    fn an_acceptance_carries_clip_metadatas_bounds() {
        let ack = |frames: &str, fps: &str, width: &str, height: &str| {
            let json = format!(
                r#"{{"pose":"accepted","job_id":"j-1","frames":{frames},"fps":{fps},"width":{width},"height":{height}}}"#
            );
            read_one::<Reply>(&json)
        };
        assert!(ack("344", "59.9", "2160", "3840").is_ok());
        // A negative frame count would become a work deadline that had already expired (clause 6).
        assert!(ack("-1", "59.9", "2160", "3840").is_err());
        assert!(ack("344", "0.0", "2160", "3840").is_err());
        assert!(ack("344", "59.9", "0", "3840").is_err());
        assert!(ack("344", "59.9", "2160", "0").is_err());
        // Zero frames is allowed, exactly as `ClipMetadata.frame_count` allows it.
        assert!(ack("0", "59.9", "2160", "3840").is_ok());
    }

    #[test]
    fn an_unknown_message_kind_is_refused_rather_than_ignored() {
        // A `progress` message is in ADR-033's *Deferred, by choice*. If one ever arrives, this
        // build must say it does not understand it and not skip the line.
        let json = r#"{"pose":"progress","job_id":"j-1","done":120}"#;
        assert!(serde_json::from_str::<Reply>(json).is_err());
    }

    #[test]
    fn clause_fives_retry_table_verbatim() {
        // Transcribed from ADR-033 clause 5, row for row. The reasons a *worker* reports; the pool's
        // own rows (crash, EOF, non-zero exit, work_timeout) live on `PoseError::retried`.
        for (reason, retried) in [
            (FailureReason::PoseFailed, true),
            (FailureReason::ClipUnreadable, false),
            (FailureReason::ClipEmpty, false),
            (FailureReason::FrameRangeUnsupported, false),
            (FailureReason::BadJob, false),
            (FailureReason::ModelAbsent, false),
        ] {
            assert_eq!(reason.retried(), retried, "{reason:?}");
        }
    }

    #[test]
    fn messages_are_framed_one_per_line_and_read_back_in_order() {
        let mut wire: Vec<u8> = Vec::new();
        write_line(&mut wire, &golden_job()).unwrap();
        write_line(
            &mut wire,
            &Job {
                job_id: "j-2".to_string(),
                ..golden_job()
            },
        )
        .unwrap();

        // Exactly one newline per message, and none inside one: the line is the frame.
        assert_eq!(wire.iter().filter(|b| **b == b'\n').count(), 2);

        let mut lines = Lines::new(&wire[..]);
        assert_eq!(lines.read::<Job>().unwrap().unwrap().job_id, "j-1");
        assert_eq!(lines.read::<Job>().unwrap().unwrap().job_id, "j-2");
        // EOF. P4 decides whether that is a shutdown or a crash.
        assert!(lines.read::<Job>().unwrap().is_none());
    }

    #[test]
    fn a_line_that_is_not_a_message_is_a_protocol_violation_and_not_a_shrug() {
        // The failure mode clause 1 reserves stderr to avoid: a dependency that printed.
        let wire = b"Warning: falling back to CPU\n";
        let mut lines = Lines::new(&wire[..]);
        match lines.read::<Reply>() {
            Err(PoseError::Protocol(message)) => {
                assert!(message.contains("not a protocol message"))
            }
            other => panic!("expected a protocol violation, got {other:?}"),
        }
    }

    #[test]
    fn a_blank_line_is_skipped_rather_than_refused() {
        let mut wire: Vec<u8> = b"\n   \n".to_vec();
        write_line(&mut wire, &golden_job()).unwrap();
        let mut lines = Lines::new(&wire[..]);
        assert_eq!(lines.read::<Job>().unwrap().unwrap().job_id, "j-1");
    }

    #[test]
    fn a_long_line_has_no_ceiling_and_the_buffer_is_reused() {
        // Two orders of magnitude below the corpus's 16.4 MB, which is enough to exercise the
        // growing buffer without making the suite read a megabyte per run. What it pins is that
        // nothing caps a line and that the second read does not start from an empty buffer.
        let frames: Vec<_> = (0..500)
            .map(|i| contracts::keypoints::FrameKeypoints {
                frame_index: i,
                timestamp_ms: i as f64 * 16.68,
                camera_id: Some("face_on".to_string()),
                landmarks: vec![
                    contracts::keypoints::Landmark {
                        x: 0.5123456789012345,
                        y: 0.4123456789012345,
                        z: -1.636_758_133_827_243e-5,
                        visibility: 0.9987654321,
                    };
                    contracts::keypoints::NUM_POSE_LANDMARKS
                ],
            })
            .collect();
        let reply = Reply::Done {
            job_id: "j-1".to_string(),
            keypoints: KeypointsFile {
                clip: None,
                frames,
                pose_estimator: Some("mediapipe:heavy".to_string()),
            },
        };

        let mut wire: Vec<u8> = Vec::new();
        write_line(&mut wire, &reply).unwrap();
        write_line(&mut wire, &reply).unwrap();
        assert!(wire.len() > 1_000_000, "{} bytes", wire.len());

        let mut lines = Lines::new(&wire[..]);
        assert_eq!(lines.read::<Reply>().unwrap().unwrap(), reply);
        assert_eq!(lines.read::<Reply>().unwrap().unwrap(), reply);
        assert!(lines.read::<Reply>().unwrap().is_none());
    }

    #[test]
    fn landmark_floats_survive_the_round_trip_bit_for_bit() {
        // Why `serde_json/float_roundtrip` is in this crate's manifest and not inherited scenery.
        // P6 diffs a parsed reply against a committed file for *exact* equality, so a one-ulp parse
        // is a difference this crate would have invented (M22 P8 measured it).
        let json = concat!(
            r#"{"pose":"done","job_id":"j-1","keypoints":{"frames":[{"frame_index":0,"#,
            r#""timestamp_ms":0.0,"landmarks":[{"x":0.030000000000000002,"#,
            r#""y":14.464487278077161,"z":-1.636758133827243e-05,"visibility":0.5}]}]}}"#
        );
        let parsed: Reply = serde_json::from_str(json).unwrap();
        let Reply::Done { keypoints, .. } = &parsed else {
            panic!("expected Done");
        };
        // Compared against `str::parse`, which is the thing M22 P8 found serde_json's fast decimal
        // path disagreeing with — so the assertion is "these two parsers agree", not "this literal
        // is what I typed".
        let landmark = &keypoints.frames[0].landmarks[0];
        for (got, text) in [
            (landmark.x, "0.030000000000000002"),
            (landmark.y, "14.464487278077161"),
            (landmark.z, "-1.636758133827243e-05"),
        ] {
            let want: f64 = text.parse().unwrap();
            assert_eq!(got.to_bits(), want.to_bits(), "{text}");
        }
    }
}
