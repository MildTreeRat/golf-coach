//! `golf-lab reanalyze`: re-run every stored result that is out of date. `scripts/reanalyze.py`.
//! [M29 P27]
//!
//! **"Out of date" has the script's three forms, asked in its order, and only the first is
//! reported**: no `analysis.json` at all; a clip re-uploaded since it was analysed
//! (`AnalysisState::matches`); an older engine (`storage::state::is_outdated`, which reads this
//! build's [`ANALYSIS_VERSION`]). Each in the script's words, so a dry run over `data/` reads as the
//! script's would under Rust's engine. The walk is the script's too: the bundle store directly,
//! sessions oldest first and swings by number, rather than the corpus, which collapses re-uploads
//! into one swing while every one of those directories still holds an artifact a reader serves.
//! Named targets are re-run whether or not they are current, and `--all` re-runs everything.
//!
//! # Two things the script could not ask
//!
//! - **A fourth form, last: a shot an older screen parser read.** Frozen Python has one parser, so
//!   its artifacts could not go stale that way. Rust has the ledger ([`SCREEN_PARSER_VERSION`]),
//!   and a run re-reads a stale shot (`golf_lab::shot`), so a result whose shot an older parser read
//!   is one today's code would not produce: the case the script was written for, one ledger over.
//!   Without it, a parser bump would leave every stored shot stale and this verb reporting nothing
//!   to do. It is read off the stored artifact's own shot (`swing.shot.provenance.parser_version`,
//!   tolerantly, as `stored_analysis_version` reads its key), so a shot with no provenance is
//!   current, as `parse_is_current` has it.
//! - **Photo entries** (the M29 plan's C7). An entry with a shot photo and no face-on clip has no
//!   swing to analyse, so the script's run would fail it for want of a face-on view. Here it is
//!   re-run as `golf-core run-shot --session` runs it, through its own code
//!   (`golf_core::session::analyse_entry`), after the shot lookup has re-read a stale stored shot
//!   (`shot::shot_for`; that verb analyses only from the store, the plan's P24 finding 8). Its
//!   staleness is `shot_analysis.json`'s, asked in the same order and words: none, a photo that is
//!   not the one it was computed from (`shot_analysis_matches`), an older engine, an older parser.
//!   **A photo entry the corpus counts as a duplicate is never out of date**: `run-shot` skips it,
//!   because a result written for it is one no reader reads, so listing it would leave a second run
//!   reporting it again, and the script is idempotent. Named or under `--all`, it is listed as up to
//!   date and its run prints the corpus's sentence.
//!
//! # Where the text differs from the script's (the plan's C8)
//!
//! - **No render** (C4). `--video` is refused by the binary, and the stale-video line, which the
//!   script ends by telling the user to re-run with `--video`, says that `golf-lab` renders none
//!   instead.
//! - **A run that raises in Python is a `FAILED` line here**, and the walk goes on: Python's
//!   traceback ends the whole run at that swing.
//! - **A photo entry's lines** are this verb's own: its one-line diff has the rows a
//!   `shot_analysis.json` holds (`version`, `measurements`), a note the shot lookup left is printed
//!   as `! <note>` because no artifact keeps it, and only the shot's `needs_review` asks for a
//!   human, since a photo entry lacks a face-on clip by definition.

use std::collections::BTreeMap;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use contracts::shot::{parse_is_current, SCREEN_PARSER_VERSION};
use contracts::shot_result::ShotResult;
use contracts::swing::{SwingBundleResult, ANALYSIS_VERSION, COMPARABLE_FROM};
use contracts::Timestamp;
use golf_core::pyrepr::py_repr;
use golf_core::serialize;
use golf_core::session::{analyse_entry, Duplicates, Outcome as EntryOutcome, ShotDirs};
use serde_json::{Map, Value};
use storage::bundle_store::SwingBundleStore;
use storage::corpus::EngineVersions;
use storage::manifest::{load_manifest, manifest_path, Role, SwingManifest};
use storage::state::{
    is_outdated, load_analysis, load_shot_analysis, load_state, shot_analysis_matches,
    stored_analysis_version,
};

use crate::analyze::{analyze_swing_dir, resolve_swing_dir};
use crate::coach::Coach;
use crate::import::spelled;
use crate::pipeline::{PipelineOptions, Tools};
use crate::shot::{parser_name, shot_for};

/// The render a run leaves standing (`ALIGNED_NAME`), which this verb only ever checks for.
pub const ALIGNED_NAME: &str = "aligned.mp4";

pub const NEVER_ANALYZED: &str = "never analyzed";
pub const REUPLOADED: &str = "an input was re-uploaded after it was analyzed";
pub const BY_REQUEST: &str = "up to date, re-run by request";

/// What one invocation asks: the script's arguments, with the pipeline's options resolved.
#[derive(Debug, Clone)]
pub struct Request {
    /// `SESSION/SWING` refs or swing directories; none is every swing that needs it.
    pub refs: Vec<String>,
    pub sessions_dir: PathBuf,
    /// `--player`: one golfer's swings, by stored id.
    pub player: Option<String>,
    /// `--all`: current results too.
    pub everything: bool,
    pub dry_run: bool,
    /// `--verbose`: the pipeline's own narration, per swing.
    pub verbose: bool,
    /// The shot store, the golfer registry and coaching (`--coaching`, off by default: C5).
    pub options: PipelineOptions,
}

/// What a target held before the run, read when it was collected: a swing's `analysis.json`, raw,
/// or a photo entry's `shot_analysis.json`.
#[derive(Debug, Clone)]
pub enum Before {
    Swing(Option<Map<String, Value>>),
    Photo(Option<Box<ShotResult>>),
}

/// One swing directory to re-run, and the sentence saying why.
#[derive(Debug, Clone)]
pub struct Target {
    pub swing_dir: PathBuf,
    pub manifest: SwingManifest,
    pub reason: String,
    pub before: Before,
}

impl Target {
    /// `SESSION/SWING`, from the manifest.
    pub fn reference(&self) -> String {
        format!("{}/{}", self.manifest.session_id, self.manifest.swing_id)
    }
}

/// A photo entry is the corpus's: no face-on clip, and a shot photo.
pub fn is_photo_entry(manifest: &SwingManifest) -> bool {
    !manifest.roles.contains_key(&Role::FaceOn) && manifest.roles.contains_key(&Role::ShotScreen)
}

/// The corpus's duplicates, one reader per sessions directory, so a target named by a path outside
/// `--sessions-dir` is judged against its own.
#[derive(Default)]
pub struct DuplicatesByRoot(BTreeMap<PathBuf, Duplicates>);

impl DuplicatesByRoot {
    fn at(&mut self, root: &Path) -> &mut Duplicates {
        self.0
            .entry(root.to_path_buf())
            .or_insert_with(|| Duplicates::new(root, versions()))
    }
}

/// This build's engine pair, which is what `run-shot` reads the corpus under.
fn versions() -> EngineVersions {
    EngineVersions {
        installed: ANALYSIS_VERSION,
        comparable_from: COMPARABLE_FROM,
    }
}

/// Where a photo entry's store sits, as `analyse_entry` reads it: the directory above its session.
fn sessions_root(swing_dir: &Path) -> PathBuf {
    let absolute = std::path::absolute(swing_dir).unwrap_or_else(|_| swing_dir.to_path_buf());
    absolute
        .parent()
        .and_then(Path::parent)
        .map_or_else(|| absolute.clone(), Path::to_path_buf)
}

fn engine_reason(stored: i64) -> String {
    format!("analyzed by engine version {stored}, {ANALYSIS_VERSION} is installed")
}

fn parser_reason(version: i64) -> String {
    format!(
        "its shot was read by {}, screen parser {SCREEN_PARSER_VERSION} is installed",
        parser_name(version)
    )
}

/// `_reason_to_rerun` for a swing, with the parser's form after the script's three.
pub fn swing_reason(
    swing_dir: &Path,
    manifest: &SwingManifest,
    analysis: Option<&Map<String, Value>>,
) -> Option<String> {
    let Some(analysis) = analysis else {
        return Some(NEVER_ANALYZED.to_string());
    };
    if load_state(swing_dir).is_some_and(|state| !state.matches(manifest)) {
        return Some(REUPLOADED.to_string());
    }
    if is_outdated(Some(analysis)) {
        return Some(engine_reason(stored_analysis_version(Some(analysis))));
    }
    stored_shot_parser(analysis)
        .filter(|&version| version < SCREEN_PARSER_VERSION)
        .map(parser_reason)
}

/// Which parser read the stored artifact's shot: `None` when it holds no shot, or a shot with no
/// provenance, which no parser read. Anything but a non-negative integer is 0, as the contract's
/// default and `stored_analysis_version`'s rule have it.
fn stored_shot_parser(analysis: &Map<String, Value>) -> Option<i64> {
    let provenance = analysis
        .get("swing")?
        .get("shot")?
        .as_object()?
        .get("provenance")?
        .as_object()?;
    Some(match provenance.get("parser_version") {
        Some(Value::Number(number)) => match (number.as_i64(), number.as_u64()) {
            (Some(version), _) if version >= 0 => version,
            (None, Some(_)) => i64::MAX,
            _ => 0,
        },
        _ => 0,
    })
}

/// The same question of a photo entry's `shot_analysis.json` (the module doc).
pub fn photo_reason(
    manifest: &SwingManifest,
    result: Option<&ShotResult>,
    duplicate: bool,
) -> Option<String> {
    if duplicate {
        return None;
    }
    let Some(result) = result else {
        return Some(NEVER_ANALYZED.to_string());
    };
    if !shot_analysis_matches(result, manifest) {
        return Some(REUPLOADED.to_string());
    }
    if result.analysis_version < ANALYSIS_VERSION {
        return Some(engine_reason(result.analysis_version));
    }
    (!parse_is_current(&result.shot)).then(|| {
        parser_reason(
            result
                .shot
                .provenance
                .as_ref()
                .map_or(0, |provenance| provenance.parser_version),
        )
    })
}

/// `_collect`: the targets, and every named ref that resolved to nothing.
pub fn collect(request: &Request, duplicates: &mut DuplicatesByRoot) -> (Vec<Target>, Vec<String>) {
    let mut missing = Vec::new();
    let mut pairs: Vec<(PathBuf, SwingManifest)> = Vec::new();
    if request.refs.is_empty() {
        let store = SwingBundleStore::new(&request.sessions_dir);
        for session_id in store.list_session_ids() {
            for manifest in store.get_session(&session_id) {
                let swing_dir = store.root().join(&session_id).join(&manifest.swing_id);
                pairs.push((swing_dir, manifest));
            }
        }
    } else {
        for reference in &request.refs {
            let swing_dir = resolve_swing_dir(reference, &request.sessions_dir);
            let manifest = swing_dir
                .as_deref()
                .and_then(|dir| load_manifest(&manifest_path(dir)));
            match (swing_dir, manifest) {
                (Some(swing_dir), Some(manifest)) => pairs.push((swing_dir, manifest)),
                _ => missing.push(reference.clone()),
            }
        }
    }

    let mut targets = Vec::new();
    for (swing_dir, manifest) in pairs {
        if request
            .player
            .as_ref()
            .is_some_and(|player| manifest.player_id.as_ref() != Some(player))
        {
            continue;
        }
        let (reason, before) = if is_photo_entry(&manifest) {
            let before = load_shot_analysis(&swing_dir);
            let duplicate = duplicates
                .at(&sessions_root(&swing_dir))
                .of(&manifest)
                .is_some();
            (
                photo_reason(&manifest, before.as_ref(), duplicate),
                Before::Photo(before.map(Box::new)),
            )
        } else {
            let before = load_analysis(&swing_dir);
            (
                swing_reason(&swing_dir, &manifest, before.as_ref()),
                Before::Swing(before),
            )
        };
        if reason.is_none() && !request.everything && request.refs.is_empty() {
            continue;
        }
        targets.push(Target {
            swing_dir,
            manifest,
            reason: reason.unwrap_or_else(|| BY_REQUEST.to_string()),
            before,
        });
    }
    (targets, missing)
}

/// `main` after the arguments: list the targets, stop on a dry run, else re-run each and report.
/// Returns the exit code: 0 clean, 1 something wants a human (a run failed, a role is missing, a
/// shot needs review, a video went stale), 2 a named target could not be found.
pub fn reanalyze(
    request: &Request,
    tools: &mut Tools<'_>,
    coach: &mut dyn Coach,
    clock: &mut dyn FnMut() -> Timestamp,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> io::Result<u8> {
    let mut duplicates = DuplicatesByRoot::default();
    let (targets, missing) = collect(request, &mut duplicates);
    let sessions_dir = spelled(&request.sessions_dir);
    for reference in &missing {
        writeln!(
            err,
            "error: no swing bundle at {} (looked in {})",
            pyfmt::str_repr(reference),
            sessions_dir.display()
        )?;
    }
    if !missing.is_empty() {
        return Ok(2);
    }
    if targets.is_empty() {
        writeln!(
            out,
            "Nothing to re-analyze in {} — every stored result is current.",
            sessions_dir.display()
        )?;
        return Ok(0);
    }

    writeln!(
        out,
        "{} swing(s) to re-analyze (engine version {ANALYSIS_VERSION}):",
        targets.len()
    )?;
    for target in &targets {
        writeln!(out, "  {:<24} {}", target.reference(), target.reason)?;
    }
    if request.dry_run {
        writeln!(out, "\n--dry-run: nothing was changed.")?;
        return Ok(0);
    }

    let (mut failures, mut attention) = (0usize, 0usize);
    for target in &targets {
        writeln!(out, "\n{}  ({})", target.reference(), target.reason)?;
        let (ok, wants_attention) = match &target.before {
            Before::Swing(before) => {
                run_swing(target, before.as_ref(), request, tools, coach, clock, out)?
            }
            Before::Photo(before) => run_photo(
                target,
                before.as_deref(),
                request,
                tools,
                &mut duplicates,
                out,
            )?,
        };
        failures += usize::from(!ok);
        attention += usize::from(wants_attention);
    }

    writeln!(
        out,
        "\nRe-analyzed {}/{} swing(s).",
        targets.len() - failures,
        targets.len()
    )?;
    if failures > 0 {
        writeln!(out, "{failures} failed — see above.")?;
    }
    Ok(if failures > 0 || attention > 0 { 1 } else { 0 })
}

/// A log that prints only under `--verbose`, and keeps the first write that failed for the caller.
fn narrate<'a>(
    verbose: bool,
    out: &'a mut dyn Write,
    failed: &'a mut Option<io::Error>,
) -> impl FnMut(&str) + 'a {
    move |line: &str| {
        if verbose && failed.is_none() {
            if let Err(error) = writeln!(out, "{line}") {
                *failed = Some(error);
            }
        }
    }
}

/// `_run_one` for a swing: the pipeline, then the one-line diff and the stale-video check.
fn run_swing(
    target: &Target,
    before: Option<&Map<String, Value>>,
    request: &Request,
    tools: &mut Tools<'_>,
    coach: &mut dyn Coach,
    clock: &mut dyn FnMut() -> Timestamp,
    out: &mut dyn Write,
) -> io::Result<(bool, bool)> {
    let mut failed = None;
    let outcome = {
        let mut log = narrate(request.verbose, out, &mut failed);
        analyze_swing_dir(
            &target.swing_dir,
            &request.options,
            tools,
            coach,
            clock,
            &mut log,
        )
    };
    if let Some(error) = failed {
        return Err(error);
    }
    let outcome = match outcome {
        Ok(outcome) => outcome,
        Err(error) => {
            writeln!(out, "  FAILED: {error}")?;
            return Ok((false, true));
        }
    };
    let Some(result) = &outcome.result else {
        writeln!(
            out,
            "  FAILED: {}",
            outcome.error.as_deref().unwrap_or("None")
        )?;
        return Ok((false, true));
    };

    writeln!(out, "  {}", diff(&swing_rows(before, result)).join(" | "))?;
    let after = serialize(result);
    let stale_video = target.swing_dir.join(ALIGNED_NAME).exists()
        && anchors(before) != anchors(after.as_object());
    if stale_video {
        writeln!(
            out,
            "  ! the alignment anchors moved, so {ALIGNED_NAME} no longer matches this result \
             — and golf-lab renders no side-by-side video to replace it (the M29 plan's C4)"
        )?;
    }
    Ok((true, outcome.flagged || stale_video))
}

/// A photo entry's run: the shot lookup, re-reading a stale stored shot, then `run-shot`'s own
/// analysis of the entry.
fn run_photo(
    target: &Target,
    before: Option<&ShotResult>,
    request: &Request,
    tools: &mut Tools<'_>,
    duplicates: &mut DuplicatesByRoot,
    out: &mut dyn Write,
) -> io::Result<(bool, bool)> {
    let mut failed = None;
    let found = {
        let mut log = narrate(request.verbose, out, &mut failed);
        shot_for(
            &target.swing_dir,
            &target.manifest,
            &request.options.shot,
            &mut *tools.reader,
            &mut log,
        )
    };
    if let Some(error) = failed {
        return Err(error);
    }
    let found = match found {
        Ok(found) => found,
        Err(error) => {
            writeln!(out, "  FAILED: {error}")?;
            return Ok((false, true));
        }
    };
    if found.shot.is_none() {
        // Nothing to analyse: `run-shot` would analyse whatever older parse the store still holds,
        // which the swing route would not attach either.
        writeln!(
            out,
            "  FAILED: {}",
            found.note.as_deref().unwrap_or("no shot")
        )?;
        return Ok((false, true));
    }
    if let Some(note) = &found.note {
        writeln!(out, "  ! {note}")?;
    }

    let root = sessions_root(&target.swing_dir);
    let dirs = ShotDirs {
        sessions: root.clone(),
        golfers: request.options.golfers_dir.clone(),
        shots: request.options.shot.shots_dir.clone(),
    };
    match analyse_entry(&target.manifest, &dirs, duplicates.at(&root)) {
        EntryOutcome::Written(result) => {
            writeln!(out, "  {}", diff(&photo_rows(before, &result)).join(" | "))?;
            let needs_review = result
                .shot
                .provenance
                .as_ref()
                .is_some_and(|provenance| provenance.needs_review);
            Ok((true, needs_review))
        }
        EntryOutcome::Duplicate(detail) => {
            writeln!(out, "  skipped: {detail}")?;
            Ok((true, false))
        }
        EntryOutcome::NoParse(detail) | EntryOutcome::Failed(detail) => {
            writeln!(out, "  FAILED: {detail}")?;
            Ok((false, true))
        }
        EntryOutcome::Swing | EntryOutcome::NoPhoto => {
            writeln!(
                out,
                "  FAILED: the manifest changed during the run and no longer names a photo entry"
            )?;
            Ok((false, true))
        }
    }
}

// ---------------------------------------------------------------------------------------------
// The one-line diff
// ---------------------------------------------------------------------------------------------

/// A value as the script's diff holds it: one `json.loads` gave, a string it formatted, or a tuple
/// it built. Printed with `str()`, which is `repr()` inside a tuple, and compared with `==`.
#[derive(Debug, Clone)]
pub enum Py {
    Json(Value),
    Text(String),
    Tuple(Vec<Py>),
}

impl Py {
    fn int(value: i64) -> Py {
        Py::Json(Value::from(value))
    }

    /// `str()`: a formatted string as itself, anything else as its `repr`.
    pub fn str(&self) -> String {
        match self {
            Py::Text(text) => text.clone(),
            other => other.repr(),
        }
    }

    fn repr(&self) -> String {
        match self {
            Py::Json(value) => py_repr(value),
            Py::Text(text) => pyfmt::str_repr(text),
            Py::Tuple(items) => {
                let items: Vec<String> = items.iter().map(Py::repr).collect();
                match items.as_slice() {
                    [one] => format!("({one},)"),
                    _ => format!("({})", items.join(", ")),
                }
            }
        }
    }
}

/// Python's `==`, not the derived one: a tuple is never a list, and numbers compare by value,
/// `True` as 1, so `(574, 790) == (574.0, 790.0)` although the two print differently.
impl PartialEq for Py {
    fn eq(&self, other: &Py) -> bool {
        match (self, other) {
            (Py::Json(a), Py::Json(b)) => json_eq(a, b),
            (Py::Text(a), Py::Text(b)) => a == b,
            (Py::Tuple(a), Py::Tuple(b)) => a == b,
            _ => false,
        }
    }
}

fn json_eq(a: &Value, b: &Value) -> bool {
    let number = |value: &Value| match value {
        Value::Bool(flag) => Some(f64::from(u8::from(*flag))),
        Value::Number(number) => number.as_f64(),
        _ => None,
    };
    match (a, b) {
        (Value::Array(a), Value::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(a, b)| json_eq(a, b))
        }
        (Value::Object(a), Value::Object(b)) => {
            a.len() == b.len()
                && a.iter()
                    .all(|(key, a)| b.get(key).is_some_and(|b| json_eq(a, b)))
        }
        _ => match (number(a), number(b)) {
            (Some(a), Some(b)) => a == b,
            (None, None) => a == b,
            _ => false,
        },
    }
}

/// One row of the diff: its label, and the two sides.
pub type Row = (&'static str, Py, Py);

/// `_before_after`'s join: the rows that moved, as `label old -> new`, or `no change`.
pub fn diff(rows: &[Row]) -> Vec<String> {
    let moved: Vec<String> = rows
        .iter()
        .filter(|(_, old, new)| old != new)
        .map(|(label, old, new)| format!("{label} {} -> {}", old.str(), new.str()))
        .collect();
    if moved.is_empty() {
        vec!["no change".to_string()]
    } else {
        moved
    }
}

/// `_before_after`'s rows for a swing, the stored side read as tolerantly as the script reads it.
pub fn swing_rows(before: Option<&Map<String, Value>>, result: &SwingBundleResult) -> Vec<Row> {
    let after = serialize(result);
    let empty = Map::new();
    let stored = before.unwrap_or(&empty);
    let swing_before = stored
        .get("swing")
        .and_then(Value::as_object)
        .unwrap_or(&empty);
    vec![
        (
            "version",
            Py::int(stored_analysis_version(before)),
            Py::int(result.analysis_version),
        ),
        (
            "score",
            Py::Text(score(swing_before.get("overall_score"))),
            Py::Text(pyfmt::fixed(result.swing.overall_score, 4)),
        ),
        (
            "measurements",
            Py::int(count(swing_before.get("measurements"))),
            Py::int(result.swing.measurements.len() as i64),
        ),
        (
            "window",
            window(stored.get("face_on_window")),
            window(after.get("face_on_window")),
        ),
        ("anchors", anchors(before), anchors(after.as_object())),
    ]
}

/// The rows a photo entry's result holds: its engine and what it measured.
pub fn photo_rows(before: Option<&ShotResult>, after: &ShotResult) -> Vec<Row> {
    vec![
        (
            "version",
            Py::int(before.map_or(0, |before| before.analysis_version)),
            Py::int(after.analysis_version),
        ),
        (
            "measurements",
            Py::int(before.map_or(0, |before| before.measurements.len() as i64)),
            Py::int(after.measurements.len() as i64),
        ),
    ]
}

/// `_num`: a number to four places, `True` and `False` among them as Python's `int`s, else `--`.
fn score(value: Option<&Value>) -> String {
    match value {
        Some(Value::Number(number)) => pyfmt::fixed(number.as_f64().unwrap_or(f64::NAN), 4),
        Some(Value::Bool(flag)) => pyfmt::fixed(f64::from(u8::from(*flag)), 4),
        _ => "--".to_string(),
    }
}

/// `len(value or [])` over what `json.loads` can give: a list's or dict's entries, a string's code
/// points, and 0 for anything falsy. A truthy number or `True` raises `TypeError` in Python, which
/// no writer of this artifact ever produced; it counts as 0 here.
fn count(value: Option<&Value>) -> i64 {
    match value {
        Some(Value::Array(items)) => items.len() as i64,
        Some(Value::Object(map)) => map.len() as i64,
        Some(Value::String(text)) => text.chars().count() as i64,
        _ => 0,
    }
}

/// `_window`: a stored list as the tuple the engine returns, else `None`.
fn window(value: Option<&Value>) -> Py {
    match value {
        Some(Value::Array(items)) => Py::Tuple(items.iter().cloned().map(Py::Json).collect()),
        _ => Py::Json(Value::Null),
    }
}

/// `_anchors`: `(motion_start, top, impact)` per view, the instants the side-by-side render warps
/// to, or `(None,)` for a view with none.
pub fn anchors(analysis: Option<&Map<String, Value>>) -> Py {
    let none = || Py::Tuple(vec![Py::Json(Value::Null)]);
    let Some(alignment) = analysis
        .and_then(|analysis| analysis.get("alignment"))
        .and_then(Value::as_object)
    else {
        return Py::Tuple(vec![none(), none()]);
    };
    let view = |name: &str| match alignment
        .get(name)
        .and_then(Value::as_object)
        .and_then(|side| side.get("anchors"))
        .and_then(Value::as_object)
    {
        Some(anchors) => Py::Tuple(
            ["motion_start", "top", "impact"]
                .iter()
                .map(|key| Py::Json(anchors.get(*key).cloned().unwrap_or(Value::Null)))
                .collect(),
        ),
        None => none(),
    };
    Py::Tuple(vec![view("a"), view("b")])
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn the_diff_prints_and_compares_as_python_does() {
        let two = Py::Tuple(vec![Py::Json(json!(574)), Py::Json(json!(790))]);
        assert_eq!(two.str(), "(574, 790)");
        assert_eq!(Py::Tuple(vec![Py::Json(Value::Null)]).str(), "(None,)");
        assert_eq!(
            anchors(None).str(),
            "((None,), (None,))",
            "no alignment, the script's default"
        );
        // `(574, 790) == (574.0, 790.0)` in Python, though the two print differently.
        let floats = Py::Tuple(vec![Py::Json(json!(574.0)), Py::Json(json!(790.0))]);
        assert_eq!(two, floats);
        // A tuple is never the list it was read from, and `True == 1`.
        assert_ne!(two, Py::Json(json!([574, 790])));
        assert_eq!(Py::Json(json!(true)), Py::Json(json!(1)));
        assert_eq!(
            diff(&[
                ("version", Py::int(16), Py::int(17)),
                ("measurements", Py::int(3), Py::int(3))
            ]),
            vec!["version 16 -> 17"]
        );
        assert_eq!(
            diff(&[("version", Py::int(17), Py::int(17))]),
            vec!["no change"]
        );
    }

    #[test]
    fn the_stored_side_reads_as_tolerantly_as_the_script_reads_it() {
        assert_eq!(score(Some(&json!(87.25))), "87.2500");
        assert_eq!(score(Some(&json!(true))), "1.0000", "`True` is an `int`");
        assert_eq!(score(Some(&json!("87"))), "--");
        assert_eq!(score(None), "--");
        assert_eq!(count(Some(&json!([1, 2]))), 2);
        assert_eq!(count(Some(&json!({"a": 1}))), 1);
        assert_eq!(count(Some(&json!(null))), 0);
        assert_eq!(count(None), 0);
        assert_eq!(window(Some(&json!("574:790"))).str(), "None");

        let stored = json!({"alignment": {"a": {"anchors": {"top": 50, "impact": 80}}, "b": null}});
        assert_eq!(
            anchors(stored.as_object()).str(),
            "((None, 50, 80), (None,))"
        );
    }

    #[test]
    fn a_shot_with_no_provenance_names_no_parser() {
        let with = |shot: Value| json!({"swing": {"shot": shot}});
        let parser = |analysis: Value| stored_shot_parser(analysis.as_object().unwrap());
        assert_eq!(parser(with(json!(null))), None, "no shot");
        assert_eq!(parser(with(json!({"provenance": null}))), None, "no parse");
        assert_eq!(
            parser(with(json!({"provenance": {}}))),
            Some(0),
            "unstamped"
        );
        assert_eq!(
            parser(with(json!({"provenance": {"parser_version": 1}}))),
            Some(1)
        );
        assert_eq!(
            parser(with(json!({"provenance": {"parser_version": 1.0}}))),
            Some(0),
            "a float is no version, as `stored_analysis_version` reads one"
        );
        assert_eq!(parser(json!({})), None);
    }
}
