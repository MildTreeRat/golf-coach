//! `golf-lab build-vectors`: new conformance vectors cut from `data/`, recorded by Rust.
//! `scripts/conformance_vectors.py`'s `_corpus` (with `build_stages` over what it builds) and
//! `_storage_corpus_real`. [M29 P30]
//!
//! ADR-035 clause 3 makes Rust the recorder, and the M29 plan's D19 says what this one may do: build
//! the vectors a stored swing gives, write each only where no file exists, and refuse an existing
//! one by name, because `golf-core rerecord` stays the only thing that rewrites a committed vector.
//! Three kinds, each the twin's:
//!
//! - **`corpus/{session}-{swing}.json.gz`**, one per stored `analysis.json` at this engine
//!   (`_corpus_vector`). The engine input is assembled from the files on disk, as P25's finding 9
//!   lays out: the windows from `analysis.json`, the shot and the intent from its `swing`, the
//!   strikes from each view's `audio.json`, and the handedness and loft resolved as the pipeline
//!   resolves them ([`handedness_for`], [`loft_for`]: `_identity`). [`corpus_input`] cuts it to the
//!   windows, and the cut input, read back as it was serialized, is what `golf_core::run` answers.
//! - **`stages/corpus/{session}-{swing}.json.gz`**, one per corpus vector (`build_stages`): the
//!   engine's intermediates over the same input, `golf_core::stages::run_stages`.
//! - **`storage/corpus/real.json.gz`**, the sessions on disk as one corpus case
//!   (`_storage_corpus_real`): what the corpus reader opens, each `analysis.json` slimmed, read for
//!   the one golfer `data/` holds and narrowed once per club the corpus holds.
//!
//! # Each is verified before anything is written, as its twin verified it
//!
//! Self-consistent files are evidence of nothing, which is the reason each twin checks what it
//! built against something it did not build. So does this, and **a build that fails a check writes
//! nothing at all**:
//!
//! - a corpus vector must reproduce its stored `analysis.json` (`_verify_against_stored`): the scores,
//!   measurements, refusals, shot and intent and the ranked feedback under `docs/CONFORMANCE.md` §3's
//!   rules, and the phases once the window's offset is added back. The scored path sees the same
//!   frames either way, so a difference means the input was reconstructed wrongly;
//! - a stage vector must compose onto its corpus vector's answer (`golf_core::stages::verify_compose`,
//!   `_verify_stages_compose`);
//! - the real corpus's slimmed tree must read, through `golf_core::storage_family`'s runner, exactly
//!   as the sessions directory itself reads ([`read_and_narrow`]).
//!
//! # What differs from the twins
//!
//! - **The oracle is Rust's.** Every vector carries `provenance.oracle: "rust"` and
//!   `provenance.recorded_by: "golf-lab build-vectors"` ([`RECORDED_BY`]), and no `python_version`
//!   or `pydantic_version`. The storage case names `_run_corpus` in `worked_against`, which is how
//!   `golf_core::storage_family::StorageCase` reads a case no Python recorded.
//! - **The corpus is read under Rust's engine pair**, `ANALYSIS_VERSION` installed and
//!   `COMPARABLE_FROM` as the line, which every Rust verb reads it under. Frozen Python could read
//!   only under its own engine twice.
//! - **The slimmed tree carries what Rust's reader opens, which is more than frozen Python's.** Each
//!   `analysis.json` keeps `swing.unscored` beside `swing.measurements` and `swing.shot`, because
//!   since M37 the reader lifts its shot-side refusals onto `CorpusSwing::shot_unscored`. A photo
//!   entry's `shot_analysis.json` travels whole, because since M35 the reader opens it. The verify
//!   is what holds the claim: a tree slimmed as `_slim_analysis` slims it does not read as `data/`
//!   reads, wherever a stored swing has a flight refusal (the M29 plan's P30 finding).
//! - **The skip line names `golf-lab reanalyze`**, which re-runs a stale artifact now.
//! - **A file is created only where none exists**, by `create_new`, so one that appeared between
//!   the build and the write is refused rather than replaced.
//!
//! Python's `_corpus` skips an artifact at another engine version with a line and goes on, and so
//! does this. A sessions directory that is not there is a refusal, as `_storage_corpus_real`'s is.

use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::{self, ErrorKind, Write};
use std::path::{Component, Path, PathBuf};

use contracts::career::CAREER_VERSION;
use contracts::club::ClubId;
use contracts::swing::{ANALYSIS_VERSION, COMPARABLE_FROM};
use golf_core::compare::compare;
use golf_core::pyrepr::py_repr;
use golf_core::rerecord::{new_vector_bytes, RUST_ORACLE};
use golf_core::stages::{run_stages, verify_compose};
use golf_core::storage_family::{read_and_narrow, run_storage, RUN_CORPUS};
use golf_core::{run, VectorInput};
use serde_json::{json, Map, Value};
use storage::audio_io::{audio_path, load_audio};
use storage::corpus::{read_corpus, EngineVersions};
use storage::keypoints_io::{keypoints_path, load_keypoints};
use storage::manifest::{load_manifest, manifest_path, Role, MANIFEST_NAME};
use storage::session_meta::SESSION_META_NAME;
use storage::state::{ANALYSIS_NAME, SHOT_ANALYSIS_NAME, STATE_NAME};

use crate::import::spelled;
use crate::pipeline::{corpus_input, handedness_for, loft_for, VIEWS};

/// `provenance.recorded_by` on every vector this builds: the verb, as a ledger entry's `by` names
/// `golf-core rerecord`.
pub const RECORDED_BY: &str = "golf-lab build-vectors";

/// The golfer the real corpus is read for, and the only one `data/` holds. `_REAL_PLAYER`.
pub const REAL_PLAYER: &str = "aaron";

/// The real corpus case's id, which is also its path under the vectors root less the suffix.
pub const REAL_STORAGE_ID: &str = "storage/corpus/real";

/// What each slimmed `analysis.json` keeps of its `swing`: `_slim_analysis`' two, and `unscored`,
/// which Rust's reader lifts and frozen Python's does not read (the module doc).
const SLIM_SWING_KEYS: [&str; 3] = ["measurements", "shot", "unscored"];

/// What the real case carries of each swing directory as written, beside its slimmed
/// `analysis.json`: `_SWING_FILES`, and a photo entry's artifact (the module doc).
const SWING_FILES: [&str; 3] = [MANIFEST_NAME, STATE_NAME, SHOT_ANALYSIS_NAME];

/// The fields of a stored swing a corpus vector must reproduce. `_verify_against_stored`'s.
const VERIFIED_SWING_FIELDS: [&str; 8] = [
    "checkpoint_scores",
    "measurements",
    "unscored",
    "mechanics_score",
    "overall_score",
    "outcome_score",
    "shot",
    "intent",
];

/// What the verb was asked: where the bundles and the golfer registry are, and the vectors root it
/// writes under (`spec/vectors/` by default).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub sessions_dir: PathBuf,
    pub golfers_dir: PathBuf,
    pub out: PathBuf,
    pub dry_run: bool,
}

/// One built vector: its id, its path under the vectors root, and its bytes as they would be
/// written. Encoded when built, so a build holds gzipped files rather than every clip as a tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Built {
    pub id: String,
    pub path: PathBuf,
    pub bytes: Vec<u8>,
}

impl Built {
    fn new(id: String, path: PathBuf, document: &Value) -> Self {
        let bytes = new_vector_bytes(document, &path);
        Self { id, path, bytes }
    }
}

/// Why a build stopped before writing anything.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildError {
    /// The sessions directory is not there, and the real corpus is read from it.
    NoSessions(PathBuf),
    /// A stored file could not be read into a vector.
    Unreadable { path: PathBuf, why: String },
    /// A built vector does not agree with what it was built from: one sentence naming the vector
    /// and the first place it parts.
    Unverified(String),
}

impl fmt::Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoSessions(path) => write!(
                f,
                "{} is missing, and the real corpus is read from it",
                spelled(path).display()
            ),
            Self::Unreadable { path, why } => write!(f, "{}: {why}", spelled(path).display()),
            Self::Unverified(why) => f.write_str(why),
        }
    }
}

/// What became of one built vector.
#[derive(Debug, PartialEq, Eq)]
pub enum Fate {
    Written,
    /// A dry run's answer for a vector with no file yet.
    WouldWrite,
    /// A file is already there, and it is left as it is.
    Refused,
    /// The write failed, and nothing is left at the path.
    Failed(String),
}

/// **The build**: every vector `data/` gives, built and verified, and nothing written. `log` hears
/// the skip lines, which `_corpus` prints to stderr.
pub fn build(
    sessions_dir: &Path,
    golfers_dir: &Path,
    log: &mut dyn FnMut(&str),
) -> Result<Vec<Built>, BuildError> {
    if !sessions_dir.is_dir() {
        return Err(BuildError::NoSessions(sessions_dir.to_path_buf()));
    }
    let mut built = Vec::new();
    for swing_dir in stored_swing_dirs(sessions_dir)? {
        let analysis_path = swing_dir.join(ANALYSIS_NAME);
        let stored: Value = read_json(&analysis_path)?;
        if stored["analysis_version"].as_i64() != Some(ANALYSIS_VERSION) {
            // A vector cut from a stale artifact would pin the previous engine's answers.
            log(&format!(
                "  skip {}: stored at v{}, engine is v{ANALYSIS_VERSION} — run golf-lab reanalyze",
                crate::file_name(&swing_dir),
                python_str(&stored["analysis_version"]),
            ));
            continue;
        }
        let [session_id, swing_id] = ["session_id", "swing_id"].map(|key| stored[key].as_str());
        let (Some(session_id), Some(swing_id)) = (session_id, swing_id) else {
            return Err(unreadable(
                &analysis_path,
                "no session_id or swing_id to name it by",
            ));
        };
        let name = format!("{session_id}-{swing_id}");
        let (engine, stages) = corpus_vectors(&swing_dir, &stored, &name, golfers_dir)?;
        built.push(engine);
        built.push(stages);
    }
    built.push(storage_real(sessions_dir)?);
    Ok(built)
}

/// Write each built vector under `out`, or in a dry run say what would be: a file only where none
/// is, and each independently, so one refusal or failure leaves the others' fates their own.
pub fn write(built: &[Built], out: &Path, dry_run: bool) -> Vec<Fate> {
    built
        .iter()
        .map(|vector| {
            let target = out.join(&vector.path);
            if dry_run {
                return if target.symlink_metadata().is_ok() {
                    Fate::Refused
                } else {
                    Fate::WouldWrite
                };
            }
            if let Some(parent) = target.parent() {
                if let Err(e) = fs::create_dir_all(parent) {
                    return Fate::Failed(format!("making {}: {e}", spelled(parent).display()));
                }
            }
            match OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&target)
            {
                Ok(mut file) => match file.write_all(&vector.bytes) {
                    Ok(()) => Fate::Written,
                    Err(e) => {
                        drop(file);
                        let _ = fs::remove_file(&target);
                        Fate::Failed(format!("writing {}: {e}", spelled(&target).display()))
                    }
                },
                Err(e) if e.kind() == ErrorKind::AlreadyExists => Fate::Refused,
                Err(e) => Fate::Failed(format!("creating {}: {e}", spelled(&target).display())),
            }
        })
        .collect()
}

/// **The verb**: build, then write, and report. Exit codes: 0 every vector written (or, in a dry
/// run, writable); 1 one or more refused because a file is there, the rest written; 2 nothing
/// written because the build stopped, or a write failed.
pub fn build_vectors(
    request: &Request,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> io::Result<u8> {
    let mut skipped = Vec::new();
    let built = match build(&request.sessions_dir, &request.golfers_dir, &mut |line| {
        skipped.push(line.to_string())
    }) {
        Ok(built) => built,
        Err(error) => {
            for line in &skipped {
                writeln!(err, "{line}")?;
            }
            writeln!(err, "golf-lab build-vectors: {error}; nothing was written")?;
            return Ok(2);
        }
    };
    for line in &skipped {
        writeln!(err, "{line}")?;
    }

    let fates = write(&built, &request.out, request.dry_run);
    let root = spelled(&request.out);
    let (mut written, mut refused, mut failed) = (0, 0, 0);
    for (vector, fate) in built.iter().zip(&fates) {
        let at = slashed(&vector.path);
        match fate {
            Fate::Written => {
                written += 1;
                writeln!(out, "  wrote {at}")?;
            }
            Fate::WouldWrite => {
                written += 1;
                writeln!(out, "  would write {at}")?;
            }
            Fate::Refused => {
                refused += 1;
                writeln!(
                    err,
                    "refused: {} is already at {}, and golf-core rerecord is the only thing that \
                     rewrites a committed vector",
                    vector.id,
                    root.join(&vector.path).display()
                )?;
            }
            Fate::Failed(why) => {
                failed += 1;
                writeln!(err, "golf-lab build-vectors: {why}")?;
            }
        }
    }
    let corpus = built
        .iter()
        .filter(|vector| vector.id.starts_with("corpus/"))
        .count();
    let verb = if request.dry_run {
        "Would write"
    } else {
        "Wrote"
    };
    writeln!(
        out,
        "{verb} {written} of {} vectors under {} ({corpus} corpus, {corpus} stages, 1 storage); \
         {refused} refused, {failed} failed",
        built.len(),
        root.display()
    )?;
    Ok(if failed > 0 {
        2
    } else if refused > 0 {
        1
    } else {
        0
    })
}

// ------------------------------------------------------------------------------ the corpus family

/// `sorted(SESSIONS.glob("*/*/analysis.json"))`'s directories: every swing directory holding an
/// `analysis.json`, in path order.
fn stored_swing_dirs(sessions_dir: &Path) -> Result<Vec<PathBuf>, BuildError> {
    let mut found = Vec::new();
    for session in subdirectories(sessions_dir)? {
        for swing in subdirectories(&session)? {
            if swing.join(ANALYSIS_NAME).is_file() {
                found.push(swing);
            }
        }
    }
    found.sort();
    Ok(found)
}

/// One stored swing's corpus vector and its stage vector, each verified (the module doc).
fn corpus_vectors(
    swing_dir: &Path,
    stored: &Value,
    name: &str,
    golfers_dir: &Path,
) -> Result<(Built, Built), BuildError> {
    let input = engine_input(swing_dir, stored, golfers_dir)?;
    let cut = corpus_input(&input);
    // What was serialized, read back and run, as `_vector` runs `given_json` rather than the live
    // objects: a field that does not survive the round trip fails here and not in a port.
    let given: VectorInput = serde_json::from_value(cut.input.clone()).map_err(|e| {
        BuildError::Unverified(format!(
            "corpus/{name}: the cut input does not read back into the ported shapes: {e}"
        ))
    })?;
    let id = format!("corpus/{name}");
    let engine = json!({
        "id": id,
        "analysis_version": ANALYSIS_VERSION,
        "provenance": {
            "kind": "corpus",
            "oracle": RUST_ORACLE,
            "recorded_by": RECORDED_BY,
            "note": format!("real capture, {}", shown(swing_dir)),
            "frame_offset": cut.frame_offset,
            "source_clip_frames": cut.source_clip_frames,
        },
        "input": cut.input,
        "expected": run(&given),
    });
    let offset = cut.frame_offset.get("face_on").copied().unwrap_or(0);
    verify_against_stored(&engine, stored, offset, &id)?;

    let stages = run_stages(&given);
    verify_compose(&stages, &engine).map_err(BuildError::Unverified)?;
    let stages = json!({
        "id": format!("stages/{id}"),
        "analysis_version": ANALYSIS_VERSION,
        "provenance": {
            "kind": "stages",
            "derived_from": id,
            "note": format!(
                "intermediates of {id}, recorded at each function boundary \
                 `analyze_swing_bundle` already has"
            ),
            "oracle": RUST_ORACLE,
            "recorded_by": RECORDED_BY,
        },
        "stages": stages,
    });

    let file = format!("{name}.json.gz");
    Ok((
        Built::new(id.clone(), Path::new("corpus").join(&file), &engine),
        Built::new(
            format!("stages/{id}"),
            Path::new("stages").join("corpus").join(&file),
            &stages,
        ),
    ))
}

/// The engine input the stored files give, whole clips and windows (P25's finding 9).
fn engine_input(
    swing_dir: &Path,
    stored: &Value,
    golfers_dir: &Path,
) -> Result<VectorInput, BuildError> {
    let manifest_file = manifest_path(swing_dir);
    let manifest = load_manifest(&manifest_file)
        .ok_or_else(|| unreadable(&manifest_file, "no manifest that reads"))?;
    let (handedness, _) = handedness_for(&manifest, golfers_dir);
    let (loft_deg, _) = loft_for(&manifest, golfers_dir);

    let analysis = swing_dir.join(ANALYSIS_NAME);
    // One key of the stored artifact, read as the engine input's field, the contract's checks and
    // all. A macro because each field is its own type, and `serde` is no edge of this crate's.
    macro_rules! field {
        ($value:expr, $what:literal) => {
            serde_json::from_value($value.clone())
                .map_err(|e| unreadable(&analysis, &format!("{}: {e}", $what)))?
        };
    }
    let swing = &stored["swing"];
    if !swing.is_object() {
        return Err(unreadable(&analysis, "no swing object"));
    }
    let [Some(session_id), Some(swing_id)] =
        ["session_id", "swing_id"].map(|key| stored[key].as_str().map(str::to_string))
    else {
        return Err(unreadable(&analysis, "no session_id or swing_id"));
    };

    let mut views = Vec::new();
    for (role, _) in VIEWS {
        views.push((keypoints(swing_dir, role)?, strikes(swing_dir, role)?));
    }
    let [(face_on, face_on_strikes), (down_the_line, down_the_line_strikes)]: [_; 2] =
        views.try_into().expect("VIEWS holds two views");
    let face_on = face_on.ok_or_else(|| {
        unreadable(
            &keypoints_path(swing_dir, Role::FaceOn),
            "no face-on keypoints, and the engine scores from them",
        )
    })?;
    Ok(VectorInput {
        swing_id,
        session_id,
        face_on,
        down_the_line,
        shot: field!(swing["shot"], "swing.shot"),
        intent: field!(swing["intent"], "swing.intent"),
        face_on_window: field!(stored["face_on_window"], "face_on_window"),
        down_the_line_window: field!(stored["down_the_line_window"], "down_the_line_window"),
        face_on_strikes,
        down_the_line_strikes,
        handedness,
        loft_deg,
    })
}

/// A view's keypoints, or `None` where the file is not there, as `_corpus_vector` skips the view.
fn keypoints(
    swing_dir: &Path,
    role: Role,
) -> Result<Option<contracts::keypoints::KeypointsFile>, BuildError> {
    let path = keypoints_path(swing_dir, role);
    if !path.exists() {
        return Ok(None);
    }
    load_keypoints(&path)
        .map(Some)
        .map_err(|e| unreadable(&path, &e.to_string()))
}

/// `_strikes`: the strikes heard in one view, as frame indices in that clip's numbering, in file
/// order and with a strike no frame was derived for left out; `None` with no `audio.json`, which the
/// cut treats as none heard.
fn strikes(swing_dir: &Path, role: Role) -> Result<Option<Vec<i64>>, BuildError> {
    let path = audio_path(swing_dir, role);
    if !path.exists() {
        return Ok(None);
    }
    let audio = load_audio(&path).map_err(|e| unreadable(&path, &e.to_string()))?;
    Ok(Some(
        audio
            .strikes
            .iter()
            .filter_map(|strike| strike.frame)
            .collect(),
    ))
}

/// `_verify_against_stored`: the windowed vector reproduces the stored analysis, shifted by its own
/// offset. Compared under `docs/CONFORMANCE.md` §3's rules, as Python's `compare_results`.
fn verify_against_stored(
    vector: &Value,
    stored: &Value,
    face_on_offset: i64,
    id: &str,
) -> Result<(), BuildError> {
    let first = |differences: Vec<golf_core::compare::Difference>, at: &str| {
        differences.first().map(|difference| {
            BuildError::Unverified(format!(
                "{id}: the windowed vector does not reproduce the stored analysis at {at} — \
                 {difference}"
            ))
        })
    };
    let (got, want) = (&vector["expected"]["swing"], &stored["swing"]);
    for field in VERIFIED_SWING_FIELDS {
        if let Some(error) = first(
            compare(&want[field], &got[field]),
            &format!("swing.{field}"),
        ) {
            return Err(error);
        }
    }
    // `feedback` hangs off the bundle rather than the swing, and it is the half a golfer reads.
    if let Some(error) = first(
        compare(&stored["feedback"], &vector["expected"]["feedback"]),
        "feedback",
    ) {
        return Err(error);
    }
    let shifted: Vec<Value> = got["phases"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|phase| {
            let mut phase = phase.clone();
            for key in ["start_frame", "end_frame"] {
                let frame = phase[key].as_i64().unwrap_or_default();
                phase[key] = json!(frame + face_on_offset);
            }
            phase
        })
        .collect();
    match first(
        compare(&want["phases"], &Value::Array(shifted)),
        "swing.phases",
    ) {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

// ----------------------------------------------------------------------------- the real corpus

/// `_storage_corpus_real`: the sessions on disk as one corpus case, verified to read exactly as the
/// directory itself reads.
fn storage_real(sessions_dir: &Path) -> Result<Built, BuildError> {
    let versions = EngineVersions {
        installed: ANALYSIS_VERSION,
        comparable_from: COMPARABLE_FROM,
    };
    let held: Vec<ClubId> = read_corpus(sessions_dir, REAL_PLAYER, versions)
        .swings
        .iter()
        .filter_map(|swing| swing.club)
        .collect();
    let narrowings: Map<String, Value> = ClubId::ALL
        .iter()
        .filter(|club| held.contains(club))
        .map(|club| {
            (
                format!("club-{}", club.as_str()),
                json!({"since": null, "sessions": null, "club": club.as_str()}),
            )
        })
        .collect();
    let source = shown(sessions_dir);
    let mut vector = json!({
        "id": REAL_STORAGE_ID,
        "career_version": CAREER_VERSION,
        "provenance": {
            "kind": "storage",
            "oracle": RUST_ORACLE,
            "recorded_by": RECORDED_BY,
            "worked_against": RUN_CORPUS,
            "analysis_version": ANALYSIS_VERSION,
            "source": source,
            "note": format!(
                "every session on disk: manifests, state files, session files and photo entries' \
                 shot_analysis.json as written, each analysis.json slimmed to analysis_version, \
                 swing.measurements, swing.shot and swing.unscored, verified to read exactly as \
                 {source}/ does"
            ),
        },
        "input": {
            "player_id": REAL_PLAYER,
            "versions": {"installed": versions.installed, "comparable_from": versions.comparable_from},
            "files": real_sessions_tree(sessions_dir)?,
            "narrowings": narrowings,
        },
    });
    let expected = run_storage(&vector).map_err(BuildError::Unverified)?;
    let direct = read_and_narrow(sessions_dir, &vector["input"]);
    if direct != expected {
        let differences: Vec<String> = compare(&direct, &expected)
            .iter()
            .take(5)
            .map(ToString::to_string)
            .collect();
        let differences = if differences.is_empty() {
            "(float bits differ)".to_string()
        } else {
            differences.join("; ")
        };
        return Err(BuildError::Unverified(format!(
            "{REAL_STORAGE_ID}: the slimmed tree does not read as {source} does — {differences}"
        )));
    }
    vector["expected"] = expected;
    Ok(Built::new(
        REAL_STORAGE_ID.to_string(),
        Path::new("storage").join("corpus").join("real.json.gz"),
        &vector,
    ))
}

/// `_real_sessions_tree`: what the corpus reader opens, nothing it skips. Dotted directories are
/// left out, because the store never enters them, and a directory left with nothing to carry keeps
/// its `/` key so it is still listed.
fn real_sessions_tree(root: &Path) -> Result<Map<String, Value>, BuildError> {
    let mut files = Map::new();
    for session in subdirectories(root)? {
        let session_name = crate::file_name(&session);
        if session_name.starts_with('.') {
            continue;
        }
        let carried = files.len();
        let meta = session.join(SESSION_META_NAME);
        if meta.is_file() {
            files.insert(
                format!("{session_name}/{SESSION_META_NAME}"),
                json!(raw_text(&meta)?),
            );
        }
        for swing in subdirectories(&session)? {
            let swing_name = crate::file_name(&swing);
            if swing_name.starts_with('.') {
                continue;
            }
            let prefix = format!("{session_name}/{swing_name}/");
            let before = files.len();
            for name in SWING_FILES {
                let path = swing.join(name);
                if path.is_file() {
                    files.insert(format!("{prefix}{name}"), json!(raw_text(&path)?));
                }
            }
            let analysis = swing.join(ANALYSIS_NAME);
            if analysis.is_file() {
                files.insert(
                    format!("{prefix}{ANALYSIS_NAME}"),
                    json!(slim_analysis(&raw_text(&analysis)?)),
                );
            }
            if files.len() == before {
                files.insert(prefix, json!(""));
            }
        }
        if files.len() == carried {
            files.insert(format!("{session_name}/"), json!(""));
        }
    }
    Ok(files)
}

/// `_slim_analysis`, keeping [`SLIM_SWING_KEYS`]: each key only where the file has it, since an
/// absent `analysis_version` reads as 0 and an absent `swing` as no measurements, and adding either
/// would change the answer. A file that is not a JSON object travels as it is, because it is read as
/// no analysis either way. Spelled by `serde_json` (keys sorted, no spaces) where Python's
/// `json.dumps` kept the file's order with spaces: the reader parses it, so only the value is
/// compared.
fn slim_analysis(text: &str) -> String {
    let Ok(Value::Object(loaded)) = serde_json::from_str::<Value>(text) else {
        return text.to_string();
    };
    let mut slim = Map::new();
    if let Some(version) = loaded.get("analysis_version") {
        slim.insert("analysis_version".into(), version.clone());
    }
    if let Some(swing) = loaded.get("swing") {
        let kept = match swing {
            Value::Object(swing) => Value::Object(
                SLIM_SWING_KEYS
                    .iter()
                    .filter_map(|key| Some((key.to_string(), swing.get(*key)?.clone())))
                    .collect(),
            ),
            other => other.clone(),
        };
        slim.insert("swing".into(), kept);
    }
    serde_json::to_string(&Value::Object(slim)).expect("a Value serializes")
}

// ---------------------------------------------------------------------------------- the plumbing

/// The directories directly under `dir`, sorted by path.
fn subdirectories(dir: &Path) -> Result<Vec<PathBuf>, BuildError> {
    let entries = fs::read_dir(dir).map_err(|e| unreadable(dir, &e.to_string()))?;
    let mut found = Vec::new();
    for entry in entries {
        let path = entry.map_err(|e| unreadable(dir, &e.to_string()))?.path();
        if path.is_dir() {
            found.push(path);
        }
    }
    found.sort();
    Ok(found)
}

fn read_json(path: &Path) -> Result<Value, BuildError> {
    serde_json::from_str(&raw_text(path)?).map_err(|e| unreadable(path, &e.to_string()))
}

/// `_raw_text`: the bytes as UTF-8, line endings and all.
fn raw_text(path: &Path) -> Result<String, BuildError> {
    let bytes = fs::read(path).map_err(|e| unreadable(path, &e.to_string()))?;
    String::from_utf8(bytes).map_err(|e| unreadable(path, &e.to_string()))
}

fn unreadable(path: &Path, why: &str) -> BuildError {
    BuildError::Unreadable {
        path: path.to_path_buf(),
        why: why.to_string(),
    }
}

/// `str()` of a parsed JSON value, as the skip line's f-string prints it: `None` for a missing key,
/// a string bare, anything else as Python writes it.
fn python_str(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        other => py_repr(other),
    }
}

/// A path under the repository relative to it, `/`-separated, as `relative_to(REPO).as_posix()`
/// spells the notes; a path elsewhere (a scratch copy) whole, `/`-separated.
fn shown(path: &Path) -> String {
    let path = spelled(path);
    let repo = spelled(&crate::repo_root());
    slashed(path.strip_prefix(&repo).unwrap_or(&path))
}

/// A path with `/` between its parts, whatever the platform writes.
fn slashed(path: &Path) -> String {
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => parts.push(part.to_string_lossy().into_owned()),
            Component::RootDir => parts.push(String::new()),
            Component::Prefix(prefix) => parts.push(prefix.as_os_str().to_string_lossy().into()),
            Component::CurDir | Component::ParentDir => {
                parts.push(component.as_os_str().to_string_lossy().into_owned())
            }
        }
    }
    parts.join("/")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `_slim_analysis`, and the one key it adds: what is there is kept, what is not stays absent,
    /// and a file that is no JSON object is carried as it is.
    #[test]
    fn the_slim_keeps_what_the_reader_reads_and_nothing_it_does_not() {
        let text = r#"{"analysis_version": 17, "notes": ["x"], "swing": {"measurements": [],
            "shot": null, "unscored": [], "phases": [], "checkpoint_scores": []}}"#;
        let slim: Value = serde_json::from_str(&slim_analysis(text)).unwrap();
        assert_eq!(
            slim,
            json!({"analysis_version": 17, "swing": {"measurements": [], "shot": null, "unscored": []}})
        );
        let partial: Value =
            serde_json::from_str(&slim_analysis(r#"{"swing": {"phases": []}}"#)).unwrap();
        assert_eq!(partial, json!({"swing": {}}));
        let swing_not_a_dict: Value =
            serde_json::from_str(&slim_analysis(r#"{"swing": [1]}"#)).unwrap();
        assert_eq!(swing_not_a_dict, json!({"swing": [1]}));
        for carried in ["[1, 2]", "not json", "", "null"] {
            assert_eq!(slim_analysis(carried), carried);
        }
    }

    /// The skip line prints the stored version as Python's f-string does.
    #[test]
    fn the_skip_line_prints_the_version_as_python_does() {
        assert_eq!(python_str(&Value::Null), "None");
        assert_eq!(python_str(&json!(16)), "16");
        assert_eq!(python_str(&json!("16")), "16");
        assert_eq!(python_str(&json!(16.5)), "16.5");
    }

    #[test]
    fn a_path_under_the_repository_is_shown_relative_and_slashed() {
        let under = crate::repo_root()
            .join("data")
            .join("processed")
            .join("sessions");
        assert_eq!(shown(&under), "data/processed/sessions");
        assert_eq!(
            slashed(Path::new("corpus").join("x.json.gz").as_path()),
            "corpus/x.json.gz"
        );
    }
}
