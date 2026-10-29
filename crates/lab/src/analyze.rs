//! One swing bundle analysed, coached, written and recorded: `api/pipeline.py::analyze_swing_dir`
//! from the engine call on, with `record_state`, `_recorded_video`, `PipelineOutcome` and
//! `resolve_swing_dir`. [M29 P26]
//!
//! [`analyze_swing_dir`] runs [`crate::pipeline::assemble`] (P25), then Python's steps after the
//! engine, in Python's order and words: the engine and its feedback (`golf_core::run_bundle`), the
//! pipeline's notes after the engine's, the loft repair once the engine has reported `no_club_loft`,
//! coaching ([`crate::coach`]), `analysis.json` without the keypoints and detections
//! (`golf_core::serialize`'s drop), and `analysis.state.json` ([`record_state`]), which is written
//! for every run that had a manifest, failed or not, because the state is a denormalised copy of the
//! analysis and whoever writes one must write the other (`record_state`'s docstring).
//!
//! # Departures
//!
//! - **No render** (the plan's C4). No run attempts one, so the state carries an existing
//!   `aligned.mp4` forward exactly as Python's does for `--no-video` ([`recorded_video`]), and a
//!   render's codec note never reaches `notes`.
//! - **`analysis.json` is written twice when coaching is on**: once before the coaching entry is
//!   asked, because it reads the swing through `golf-mcp`, which reads the file (P17's finding 4),
//!   and again with the paragraph or the note folded in. Python writes once, after an in-process
//!   call. Both writes are atomic (`storage::state::save_analysis`), where Python's `write_text` is
//!   not, so a reader never meets half a file.
//! - **The artifact carries only reasons frozen Python knows** (the plan's C6). Frozen Python's
//!   `UnscoredReason` is closed, so an `analysis.json` naming a photo-side reason would be refused by
//!   every Python reader of it (ADR-035 clause 4). The engine never writes one into a `SwingResult`
//!   (`contracts::unscored`'s doc says so), and [`write_analysis`] refuses one if it ever did, before
//!   anything is written: [`UNREADABLE_BY_FROZEN_PYTHON`], which `tests/analyze.rs` holds to the
//!   committed schema's enum.
//! - **The clock is the caller's** (`clock`), as every Rust store that stamps a time takes it.

use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

use contracts::swing::SwingBundleResult;
use contracts::unscored::{UnscoredCheckpoint, UnscoredReason};
use contracts::Timestamp;
use golf_core::{run_bundle, serialize};
use storage::manifest::{manifest_path, Role, SwingManifest};
use storage::state::{
    input_hashes, load_state, save_analysis, save_state, AnalysisState, Status, ANALYSIS_NAME,
};

use crate::coach::{Coach, CoachAnswer, CoachRequest};
use crate::import::spelled;
use crate::pipeline::{assemble, Assembly, PipelineError, PipelineOptions, Tools};

/// The reasons Rust names that frozen Python's `UnscoredReason` does not: the two photo-side ones,
/// which belong in `shot_analysis.json` and never in a swing's artifact (C6).
pub const UNREADABLE_BY_FROZEN_PYTHON: [UnscoredReason; 2] =
    [UnscoredReason::PrintedBlank, UnscoredReason::Misread];

/// What a run produced: `PipelineOutcome`, less the render's three fields. `result` is `None` when
/// nothing could be scored, and then `error` says why.
#[derive(Debug, Clone)]
pub struct Outcome {
    pub result: Option<SwingBundleResult>,
    pub analysis_path: Option<PathBuf>,
    /// The roles the bundle never received.
    pub missing: Vec<Role>,
    /// Worth a human's attention: a role never arrived, or the shot needs review. Exit code 1.
    pub flagged: bool,
    /// Set only when `result` is `None`. Exit code 2.
    pub error: Option<String>,
}

impl Outcome {
    fn failed(missing: Vec<Role>, error: String) -> Self {
        Outcome {
            result: None,
            analysis_path: None,
            missing,
            flagged: false,
            error: Some(error),
        }
    }
}

/// Why a run stopped where Python's would have raised: the assembly's own reasons, an artifact
/// frozen Python could not read, or a file that would not write.
#[derive(Debug)]
pub enum AnalyzeError {
    Pipeline(PipelineError),
    /// The result names a reason in [`UNREADABLE_BY_FROZEN_PYTHON`], so nothing was written.
    UnreadableReason {
        checkpoint: String,
        reason: UnscoredReason,
    },
    Write {
        path: PathBuf,
        error: io::Error,
    },
}

impl fmt::Display for AnalyzeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AnalyzeError::Pipeline(error) => write!(f, "{error}"),
            AnalyzeError::UnreadableReason { checkpoint, reason } => write!(
                f,
                "the result names {checkpoint} unscored as {}, a reason frozen Python's \
                 UnscoredReason does not know, so no analysis.json was written (the M29 plan's C6)",
                wire_name(*reason)
            ),
            AnalyzeError::Write { path, error } => {
                write!(f, "could not write {}: {error}", spelled(path).display())
            }
        }
    }
}

impl std::error::Error for AnalyzeError {}

/// A reason's wire name, through its own `Serialize`, so no name is retyped here.
pub fn wire_name(reason: UnscoredReason) -> String {
    match serde_json::to_value(reason) {
        Ok(serde_json::Value::String(name)) => name,
        other => unreachable!("a reason serializes to its name, not {other:?}"),
    }
}

impl From<PipelineError> for AnalyzeError {
    fn from(error: PipelineError) -> Self {
        AnalyzeError::Pipeline(error)
    }
}

/// `resolve_swing_dir`: `SESSION/SWING` inside the store, or a path to the swing directory. The
/// path wins when it holds a manifest, and either spelling of the separator splits the pair.
pub fn resolve_swing_dir(target: &str, sessions_dir: &Path) -> Option<PathBuf> {
    let direct = PathBuf::from(target);
    if direct.is_dir() && manifest_path(&direct).exists() {
        return Some(direct);
    }
    let parts: Vec<String> = target
        .replace('\\', "/")
        .split('/')
        .map(str::to_string)
        .collect();
    if let [session, swing] = parts.as_slice() {
        let candidate = sessions_dir.join(session).join(swing);
        if manifest_path(&candidate).exists() {
            return Some(candidate);
        }
    }
    None
}

/// `analyze_swing_dir`: the whole pipeline over one bundle (the module doc).
///
/// `Ok` with `error` set for the two expected failures (no readable manifest, which records no
/// state, and no usable face-on view, which records a failed one); `Err` where Python raises.
pub fn analyze_swing_dir(
    swing_dir: &Path,
    options: &PipelineOptions,
    tools: &mut Tools<'_>,
    coach: &mut dyn Coach,
    clock: &mut dyn FnMut() -> Timestamp,
    log: &mut dyn FnMut(&str),
) -> Result<Outcome, AnalyzeError> {
    let started_at = clock();
    let assembled = match assemble(swing_dir, options, tools, log)? {
        Assembly::Refused(refusal) => {
            let outcome = Outcome::failed(refusal.missing, refusal.error);
            // No manifest, no state: there are no `inputs` to key one on.
            if let Some(manifest) = &refusal.manifest {
                record_state(swing_dir, manifest, &outcome, started_at, clock())?;
            }
            return Ok(outcome);
        }
        Assembly::Ready(assembled) => *assembled,
    };

    let mut result = run_bundle(&assembled.input);
    // The pipeline's notes after the engine's, which is the order every stored `analysis.json`
    // holds (P22's finding 5, held on all 15 by `tests/inputs.rs`).
    result.notes.extend(assembled.notes.iter().cloned());
    if let Some(remedy) = assembled.loft_note(&result.swing.unscored) {
        log(&format!("\nBall flight:\n  {remedy}"));
        result.notes.push(remedy.to_string());
    }

    // Last, on the finished result, and the one step allowed to fail without costing anything: a
    // refusal is a note and the swing keeps its score.
    if options.coaching {
        log("\nCoaching:");
        // `get_swing` reads the artifact from disk, and the brief reads `notes`, so the file the
        // entry reads is this one: every note in, the paragraph not yet.
        write_analysis(&result, swing_dir)?;
        match coach_request(swing_dir, &options.shot.shots_dir) {
            Ok((sessions_dir, shots_dir, session_id, swing_id)) => {
                let answer = coach.coach(&CoachRequest {
                    sessions_dir: &sessions_dir,
                    shots_dir: &shots_dir,
                    session_id: &session_id,
                    swing_id: &swing_id,
                });
                fold_coaching(&mut result, answer, log);
            }
            Err(note) => fold_coaching(&mut result, CoachAnswer::Note(note), log),
        }
    }

    let analysis_path = write_analysis(&result, swing_dir)?;
    log(&format!("\nWrote {}", spelled(&analysis_path).display()));

    let needs_review = assembled
        .input
        .shot
        .as_ref()
        .and_then(|shot| shot.provenance.as_ref())
        .is_some_and(|provenance| provenance.needs_review);
    let outcome = Outcome {
        flagged: needs_review || !assembled.missing.is_empty(),
        result: Some(result),
        analysis_path: Some(analysis_path),
        missing: assembled.missing,
        error: None,
    };
    record_state(
        swing_dir,
        &assembled.manifest,
        &outcome,
        started_at,
        clock(),
    )?;
    Ok(outcome)
}

/// Where `golf-mcp` finds this swing: the directory above its session, its session's name and its
/// own, made absolute, because the entry runs in the repository root and a relative path would be
/// read from there. The directory's names rather than the manifest's ids, because they are what
/// `get_swing` joins.
fn coach_request(
    swing_dir: &Path,
    shots_dir: &Path,
) -> Result<(PathBuf, PathBuf, String, String), String> {
    let absolute = |path: &Path| std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    let swing_dir = absolute(swing_dir);
    let name = |path: Option<&Path>| {
        path.and_then(Path::file_name)
            .map(|name| name.to_string_lossy().into_owned())
    };
    let session_dir = swing_dir.parent();
    match (
        session_dir.and_then(Path::parent),
        name(session_dir),
        name(Some(&swing_dir)),
    ) {
        (Some(sessions_dir), Some(session_id), Some(swing_id)) => Ok((
            sessions_dir.to_path_buf(),
            absolute(shots_dir),
            session_id,
            swing_id,
        )),
        _ => Err(format!(
            "no written coaching: {} does not sit in a sessions directory, which is where the \
             coaching entry reads a swing from. The scores and ranked tips above are unaffected.",
            spelled(&swing_dir).display()
        )),
    }
}

/// Python's fold: the paragraph and its provenance onto the feedback, or the note onto the notes.
fn fold_coaching(result: &mut SwingBundleResult, answer: CoachAnswer, log: &mut dyn FnMut(&str)) {
    match answer {
        CoachAnswer::Written { text, provenance } => {
            // `len(coaching.text)`: Python counts code points.
            log(&format!(
                "  {} wrote {} characters",
                provenance.model,
                text.chars().count()
            ));
            let feedback = result
                .feedback
                .as_mut()
                .expect("run_bundle builds the feedback");
            feedback.coaching_text = Some(text);
            feedback.coaching = Some(provenance);
        }
        CoachAnswer::Note(note) => {
            log(&format!("  {note}"));
            result.notes.push(note);
        }
    }
}

/// The first entry naming a reason frozen Python could not read (C6), if any.
pub fn unreadable_by_frozen_python(unscored: &[UnscoredCheckpoint]) -> Option<&UnscoredCheckpoint> {
    unscored
        .iter()
        .find(|entry| UNREADABLE_BY_FROZEN_PYTHON.contains(&entry.reason))
}

/// Write `analysis.json`, after C6's check, as `golf_core::serialize` drops it. Returns its path.
pub fn write_analysis(
    result: &SwingBundleResult,
    swing_dir: &Path,
) -> Result<PathBuf, AnalyzeError> {
    if let Some(entry) = unreadable_by_frozen_python(&result.swing.unscored) {
        return Err(AnalyzeError::UnreadableReason {
            checkpoint: entry.name.clone(),
            reason: entry.reason,
        });
    }
    let path = swing_dir.join(ANALYSIS_NAME);
    save_analysis(&serialize(result), swing_dir).map_err(|error| AnalyzeError::Write {
        path: path.clone(),
        error,
    })?;
    Ok(path)
}

/// `record_state`: write the `analysis.state.json` summary of a run that just finished.
///
/// Any existing state is read first, so a `queued_at` the frozen worker wrote survives a run it did
/// not start, and so does the render this run did not replace ([`recorded_video`]). Everything else
/// describes this run. `duration_seconds` is `timedelta.total_seconds()` over the two stamps'
/// microseconds, so it is exact to the microsecond as Python's is.
pub fn record_state(
    swing_dir: &Path,
    manifest: &SwingManifest,
    outcome: &Outcome,
    started_at: Timestamp,
    completed_at: Timestamp,
) -> Result<AnalysisState, AnalyzeError> {
    let previous = load_state(swing_dir);
    let missing: Vec<String> = manifest
        .missing_roles()
        .iter()
        .map(|role| role.as_str().to_string())
        .collect();
    let (video, video_codec) = recorded_video(swing_dir, previous.as_ref());
    let result = outcome.result.as_ref();
    let state = AnalysisState {
        status: if result.is_some() {
            Status::Done
        } else {
            Status::Failed
        },
        inputs: input_hashes(manifest),
        queued_at: previous.as_ref().and_then(|previous| previous.queued_at),
        started_at: Some(started_at),
        completed_at: Some(completed_at),
        duration_seconds: Some(
            (completed_at.utc_micros() - started_at.utc_micros()) as f64 / 1_000_000.0,
        ),
        // `outcome.error or "…"`: an empty error reads as none.
        error: result.is_none().then(|| {
            outcome
                .error
                .clone()
                .filter(|error| !error.is_empty())
                .unwrap_or_else(|| "the pipeline produced no result".to_string())
        }),
        partial: !missing.is_empty(),
        missing_roles: missing,
        video,
        video_codec,
        score: result.map(|result| result.swing.overall_score),
        headline: result
            .and_then(|result| result.feedback.as_ref())
            .and_then(|feedback| feedback.headline.clone()),
    };
    save_state(&state, swing_dir).map_err(|error| AnalyzeError::Write {
        path: storage::state::state_path(swing_dir),
        error,
    })?;
    Ok(state)
}

/// `_recorded_video` for a run that did not render, which is every run here (C4): the previous
/// state's `aligned.mp4` and its codec, **if the file still stands**, because the sidecar outlives
/// anything deleted by hand; else none.
pub fn recorded_video(
    swing_dir: &Path,
    previous: Option<&AnalysisState>,
) -> (Option<String>, Option<String>) {
    match previous {
        Some(previous) => match &previous.video {
            Some(video) if swing_dir.join(video).is_file() => {
                (Some(video.clone()), previous.video_codec.clone())
            }
            _ => (None, None),
        },
        None => (None, None),
    }
}
