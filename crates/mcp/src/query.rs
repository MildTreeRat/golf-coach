//! Reads what the pipeline wrote, in the shapes an MCP tool returns. `src/golf_coach/mcp/query.py`.
//! [M29 P10]
//!
//! Pure reads over the session directory, through `crates/storage`'s bundle store and tolerant
//! readers. Nothing here touches the protocol: a view is a plain struct, and `crate::answer` puts
//! it on the wire.
//!
//! **The views are not passthroughs, and the difference is the point** (Python's module doc says
//! it at length). A model presents whatever it is handed as fact, so what this repository knows to
//! be provisional survives the trip out: a flagged shot parse arrives labelled, an alignment below
//! `full` carries the sentence saying what that does not support, every unscored checkpoint says
//! *why* it was not scored, and the placements, pivots and simulated rows leave `measurements`
//! with the detail that a bare float would lose.
//!
//! **Shots are joined by photo hash, not by session id**: `get_session_summary` aggregates the
//! shots attached to that session's swings, which is not the shot store's view of the session.
//!
//! # Where the shot view answers differently from Python, on purpose [M29 P11]
//!
//! Two departures, both the program plan's §M29 and both in [`shot_view`], so they reach every
//! tool that shows a shot: `get_recent_shots`, `get_shot_by_id`, `get_swing`'s `shot`, and through
//! it `get_session_summary`'s averages.
//!
//! - **The warnings go through M34's golfer-facing filter** (`screen::golfer_warnings`): the
//!   `no tile found for '<label>'` line is about the screen's layout, not the photo, and is on every
//!   stored shot (every bay photo lacks `Bounce & Roll`). It tells a golfer about a stat their
//!   screen never showed them, which ADR-034 §2 says a golfer is never told. It stays in the stored
//!   `provenance.warnings`. Every other line, the tie warning included, is kept. This moves recorded
//!   answers, and `golf_mcp::family::DECLARED` declares them by path: `tests/tools.rs` holds Rust's
//!   answer to the recorded one with the filter applied at those paths, and to nothing looser.
//! - **M32's seven fields join the metric list** ([`NUMERIC_METRICS`], [`TEXT_METRICS`]), because
//!   frozen Python's `ShotData` never gained them. No stored shot carries one yet, so no recorded
//!   answer moves; `spec/vectors/mcp/hand/shot-view.json` is the case that reaches them.
//!
//! # Two keys `get_swing` answers that Python's view never had [M29 P17]
//!
//! `feedback/coach.py`'s JSON entry writes the coaching brief from this view rather than from
//! frozen Python's `SwingBundleResult` (the plan's D15, ADR-035 clause 1), and two lines of that
//! brief needed something no view carried. Each is an added key, declared by path in
//! `golf_mcp::family::DECLARED`:
//!
//! - **[`CheckpointView::one_sided`]**, the stored score's own. The brief prints "lower is better"
//!   or "both extremes are faults" beside every percentile, because at the 10th percentile a
//!   one-sided metric is excellent and a two-sided one is as wrong as at the 90th. The briefing
//!   tells every client to "read `one_sided` before describing a low number as a strength", and
//!   until this key no tool answered one.
//! - **[`UnscoredView::family`]**, `checkpoint` or `flight`. A checkpoint that could not be scored
//!   was going to be in `overall_score`; a `flight_*` number the simulation could not produce never
//!   was, and a brief that called both "excluded from overall_score" invites a model to explain a
//!   score that never moved. Every stored swing carries a flight refusal, so the split is not a
//!   corner case. It is `contracts::unscored::is_inference_reason`, read off the reason.
//!
//! **What is not applied: the capability's analysed view.** `contracts::capability::analysed_view`
//! blanks a reading the device prints only to be shown, which on every stored HD Golf shot is
//! `club_head_speed` and `smash_factor`. That is what *grading* may read. This view is what the
//! screen *said*, which the phone shows too (ADR-034 §2), and a model asked "how fast was my club"
//! is right to quote the screen's number. So every reading the shot holds is reported.
//!
//! # Where this reads differently from Python, and why none of it is recorded
//!
//! The views are Python's field for field, and every recorded answer reproduces outside the
//! declared paths above (`tests/tools.rs`). What differs is in the readers underneath, each named
//! where it is decided:
//!
//! - **The stored files are read by `serde_json`, not `json.loads`** (`storage::state`'s crate doc):
//!   a `NaN` or an out-of-range number makes an artifact unreadable here, which no engine writes.
//! - **A stored shot is held to Rust's `ShotData`**, where Python re-validates it through pydantic.
//!   Both ignore keys they do not know, so a shot either side wrote reads on both; pydantic's lax
//!   mode also reads a number written as a string, which no writer here produces.
//! - **An unscored reason is any of Rust's**, where Python's closed enum reads the two photo-side
//!   reasons (`printed_blank`, `misread`) as `unrecorded`. Neither is ever written into an
//!   `analysis.json` (the plan's C6), so the two agree on everything stored, and on a file that
//!   broke that rule this names the cause rather than "this result does not know".
//! - **`str()` of a container** (a list or an object where a name was expected) is Python's repr,
//!   except that an object's keys come out sorted, because `serde_json`'s maps are. No engine
//!   writes one; see [`py_str`].

use std::collections::BTreeMap;
use std::path::Path;

use contracts::alignment::AlignmentQuality;
use contracts::career::MODEL_SOURCE_PREFIX;
use contracts::mishit::{MishitVerdict, MISHIT_EXCLUDED_METRICS};
use contracts::shot::ShotData;
use contracts::unscored::{is_inference_reason, spec_for, UnscoredReason};
use golf_core::pyrepr::py_repr;
use serde::Serialize;
use serde_json::{Map, Number, Value};
use storage::bundle_store::SwingBundleStore;
use storage::manifest::{load_manifest, manifest_path, SwingManifest};
use storage::state::{
    load_analysis, load_state, resolve_pivots, resolve_placements, AnalysisState,
};

use crate::answer::Ordered;
use crate::caveats::ALIGNMENT_CAVEAT;
use crate::source::ShotSource;

// -------------------------------------------------------------------------------------------------
// The misses
// -------------------------------------------------------------------------------------------------

/// Nothing matched, said out loud rather than by returning nothing.
///
/// A tool returning `None` serializes to a result with no content block, which reads the same as a
/// call that silently did nothing; naming the miss, and saying what to do about it, is the
/// difference between a useful retry and an invented answer.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct NotFound {
    /// Always false. This is the miss case.
    pub found: bool,
    pub message: String,
}

/// `_missing(what, hint)`: the sentence and the retry it points at, one space apart. `crate::career`
/// builds its three golfer misses through it too, as `career.py` reaches for `query._missing`.
pub(crate) fn missing(what: String, hint: &str) -> NotFound {
    NotFound {
        found: false,
        message: format!("{what} {hint}"),
    }
}

/// `missing_swing`. The ids are quoted as Python's `!r` quotes them (`pyfmt::str_repr`), because a
/// model retries with what it reads.
pub fn missing_swing(session_id: &str, swing_id: &str) -> NotFound {
    missing(
        format!(
            "No swing {} in session {}.",
            pyfmt::str_repr(swing_id),
            pyfmt::str_repr(session_id)
        ),
        "Call list_sessions to see which sessions and swing ids exist.",
    )
}

/// `missing_session`.
pub fn missing_session(session_id: &str) -> NotFound {
    missing(
        format!("No session {}.", pyfmt::str_repr(session_id)),
        "Call list_sessions to see which sessions exist.",
    )
}

/// `missing_shot`. [M29 P11]
pub fn missing_shot(shot_id: &str) -> NotFound {
    missing(
        format!("No shot with id {}.", pyfmt::str_repr(shot_id)),
        "Call get_recent_shots to see which shot ids exist.",
    )
}

// -------------------------------------------------------------------------------------------------
// The views, each in its pydantic model's field order
// -------------------------------------------------------------------------------------------------

/// One swing as it appears in a listing: cheap enough to render a whole session from.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SwingSummary {
    pub swing_id: String,
    pub session_id: String,
    /// `queued | running | done | failed | not analyzed`, or the stale sentence.
    pub status: String,
    /// Overall 0–100, `None` until analyzed.
    pub score: Option<f64>,
    /// The one thing to work on first.
    pub headline: Option<String>,
    /// Which of the three files landed, by wire name, sorted.
    pub roles: Vec<String>,
    /// Analyzed without all three roles.
    pub partial: bool,
}

/// A day at the bay, as a listing entry.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SessionSummary {
    pub session_id: String,
    pub swing_count: usize,
    pub analyzed_count: usize,
    pub swings: Vec<SwingSummary>,
}

/// One scored fundamental, with the band it was judged against and where it sits.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CheckpointView {
    pub name: String,
    pub observed: Option<f64>,
    pub expected_low: Option<f64>,
    pub expected_high: Option<f64>,
    pub passed: Option<bool>,
    pub score: Option<f64>,
    pub percentile: Option<f64>,
    /// A `Number`, because Python's `int` is: whatever integer was stored is answered.
    pub population_n: Option<Number>,
    pub message: Option<String>,
    /// Whether only one side of the band is a fault, as the stored score says; `None` when it does
    /// not say. Python's view has no such key (the module doc). [M29 P17]
    pub one_sided: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TipView {
    pub checkpoint: String,
    pub text: String,
    pub severity: Option<String>,
}

/// A metric value: a launch monitor's number, or its own words for a categorical read-out.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Metric {
    Number(f64),
    Text(String),
}

/// A launch-monitor shot with its trustworthiness hoisted to the top level, where a consumer that
/// quotes a carry cannot miss that the parse was flagged.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ShotView {
    pub shot_id: String,
    pub session_id: String,
    pub source: String,
    pub needs_review: bool,
    pub warnings: Vec<String>,
    pub parse_confidence: Option<f64>,
    pub metrics: Ordered<Metric>,
}

/// One population placement, with the sentence that makes it readable attached to it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PlacementView {
    pub name: String,
    pub value: f64,
    pub unit: String,
    pub view: String,
    pub calibrated: bool,
    pub detail: String,
}

/// One rotation measurement, with the sentence that says it is interim attached to it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PivotView {
    pub name: String,
    pub value: f64,
    pub unit: String,
    pub view: String,
    pub interim_reason: String,
    pub detail: String,
}

/// One number a model produced rather than an instrument measured, with the provenance a bare
/// float would lose.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SimulatedView {
    pub name: String,
    pub value: f64,
    pub unit: String,
    pub source: String,
    pub detail: String,
}

/// One checkpoint that produced no score, and why: carried, never inferred.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct UnscoredView {
    pub name: String,
    /// The stable token a consumer branches on.
    pub reason: String,
    /// The reason as a clause that can be read to a golfer.
    pub why: String,
    pub refilming_helps: bool,
    /// `flight` for a ball-flight number the simulation could not produce, which was never part of
    /// `overall_score`, and `checkpoint` for everything else. Python's view has no such key (the
    /// module doc). [M29 P17]
    pub family: &'static str,
}

/// [`UnscoredView::family`]'s two values.
pub const CHECKPOINT_FAMILY: &str = "checkpoint";
pub const FLIGHT_FAMILY: &str = "flight";

/// The family an unscored entry with this reason belongs to. A photo-side reason reads as
/// `checkpoint`: neither is ever written into `analysis.json` (the plan's C6), and the one thing the
/// split decides is whether the entry is a flight number.
pub fn family_of(reason: UnscoredReason) -> &'static str {
    if is_inference_reason(reason) {
        FLIGHT_FAMILY
    } else {
        CHECKPOINT_FAMILY
    }
}

/// Everything the coach needs about one swing: mechanics, outcome, and what to distrust.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SwingView {
    pub swing_id: String,
    pub session_id: String,
    pub status: String,
    pub club: Option<String>,
    pub mishit: Option<String>,
    pub overall_score: Option<f64>,
    pub mechanics_score: Option<f64>,
    pub outcome_score: Option<f64>,
    pub headline: Option<String>,
    pub checkpoints: Vec<CheckpointView>,
    pub tips: Vec<TipView>,
    pub unscored: Vec<UnscoredView>,
    pub measurements: Ordered<f64>,
    pub simulated: Vec<SimulatedView>,
    pub population: Vec<PlacementView>,
    pub rotation: Vec<PivotView>,
    pub analysis_version: Option<Number>,
    pub alignment_quality: Option<String>,
    pub alignment_caveat: Option<String>,
    pub shot: Option<ShotView>,
    pub notes: Vec<String>,
}

impl SwingView {
    /// A swing with nothing analyzed: its identity, its status, and what the manifest stamps.
    fn bare(
        swing_id: &str,
        session_id: &str,
        status: String,
        club: Option<String>,
        mishit: Option<String>,
    ) -> Self {
        SwingView {
            swing_id: swing_id.to_string(),
            session_id: session_id.to_string(),
            status,
            club,
            mishit,
            overall_score: None,
            mechanics_score: None,
            outcome_score: None,
            headline: None,
            checkpoints: Vec::new(),
            tips: Vec::new(),
            unscored: Vec::new(),
            measurements: Ordered::default(),
            simulated: Vec::new(),
            population: Vec::new(),
            rotation: Vec::new(),
            analysis_version: None,
            alignment_quality: None,
            alignment_caveat: None,
            shot: None,
            notes: Vec::new(),
        }
    }
}

/// Per-session aggregates across both axes of ADR-009's model.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SessionDetail {
    pub session_id: String,
    pub swing_count: usize,
    pub analyzed_count: usize,
    pub mean_score: Option<f64>,
    pub best_score: Option<f64>,
    pub worst_score: Option<f64>,
    /// checkpoint → `passed/measured`, sorted by name as Python's `sorted(…)` builds it.
    pub checkpoint_pass_rates: BTreeMap<String, String>,
    pub unscored_counts: BTreeMap<String, usize>,
    pub shots_attached: usize,
    /// Attached shots whose parse was flagged (ADR-014).
    pub shots_needing_review: usize,
    /// Attached shots a CONFIRMED verdict held out of the distance averages (ADR-028 §3).
    pub mishits_excluded: usize,
    /// The mean of each metric over attached, trusted shots.
    pub shot_averages: BTreeMap<String, f64>,
    /// Each analyzed swing's lead coaching point, in order.
    pub headlines: Vec<String>,
}

// -------------------------------------------------------------------------------------------------
// Sessions and swings
// -------------------------------------------------------------------------------------------------

/// `list_sessions`: sessions newest first, each with its swings, at most `limit` of them (a
/// negative limit is none, as Python's `[: max(0, limit)]` is).
///
/// Reads `score` and `headline` off the `analysis.state.json` sidecar rather than opening every
/// `analysis.json`, which is what that denormalisation exists for.
pub fn list_sessions(sessions_dir: &Path, limit: i64) -> Vec<SessionSummary> {
    if !sessions_dir.exists() {
        return Vec::new();
    }
    let store = SwingBundleStore::new(sessions_dir);
    let mut session_ids = store.list_session_ids();
    session_ids.reverse();
    let take = usize::try_from(limit.max(0)).unwrap_or(usize::MAX);
    session_ids
        .into_iter()
        .take(take)
        .map(|session_id| {
            // Each swing's directory is named by its manifest's `swing_id`, as Python joins it,
            // and not by the directory the manifest was found in.
            let swings: Vec<SwingSummary> = store
                .get_session(&session_id)
                .iter()
                .map(|manifest| {
                    let dir = sessions_dir.join(&session_id).join(&manifest.swing_id);
                    summarize(manifest, &dir)
                })
                .collect();
            SessionSummary {
                swing_count: swings.len(),
                analyzed_count: swings.iter().filter(|s| s.score.is_some()).count(),
                session_id,
                swings,
            }
        })
        .collect()
}

/// `get_swing`: one swing's full analysis, or `None` when there is no such swing.
pub fn get_swing(sessions_dir: &Path, session_id: &str, swing_id: &str) -> Option<SwingView> {
    let swing_dir = sessions_dir.join(session_id).join(swing_id);
    let manifest = SwingBundleStore::new(sessions_dir).get_swing(session_id, swing_id)?;

    let state = load_state(&swing_dir);
    let status = status_of(state.as_ref(), &swing_dir);
    let analysis = load_analysis(&swing_dir);
    // Off the manifest, not the analysis: both are stamped on the swing rather than derived by the
    // engine, so an unanalyzed swing carries them too.
    let club = manifest.club.map(|club| club.as_str().to_string());
    let mishit = manifest.mishit.map(|verdict| verdict.as_str().to_string());

    let Some(analysis) = analysis else {
        return Some(SwingView::bare(swing_id, session_id, status, club, mishit));
    };

    let swing = object(analysis.get("swing"));
    let feedback = object(analysis.get("feedback"));
    let alignment = object(analysis.get("alignment"));
    let (alignment_quality, alignment_caveat) = alignment_view(alignment);
    Some(SwingView {
        swing_id: swing_id.to_string(),
        session_id: session_id.to_string(),
        status,
        club,
        mishit,
        overall_score: number(swing.get("overall_score")),
        mechanics_score: number(swing.get("mechanics_score")),
        outcome_score: number(swing.get("outcome_score")),
        headline: text(feedback.get("headline")),
        checkpoints: list(swing.get("checkpoint_scores"))
            .iter()
            .map(checkpoint)
            .collect(),
        tips: list(feedback.get("tips")).iter().map(tip).collect(),
        unscored: list(swing.get("unscored")).iter().map(unscored).collect(),
        measurements: measurements(swing),
        simulated: simulated(swing),
        // Resolved off the whole artifact, through the one definition every reader shares.
        population: resolve_placements(Some(&analysis))
            .into_iter()
            .map(|row| PlacementView {
                name: row.name,
                value: row.value,
                unit: row.unit,
                view: row.view.to_string(),
                calibrated: row.calibrated,
                detail: row.detail,
            })
            .collect(),
        rotation: resolve_pivots(Some(&analysis))
            .into_iter()
            .map(|row| PivotView {
                name: row.name,
                value: row.value,
                unit: row.unit,
                view: row.view.to_string(),
                interim_reason: row.interim_reason.to_string(),
                detail: row.detail,
            })
            .collect(),
        analysis_version: version(&analysis),
        alignment_quality,
        alignment_caveat,
        shot: swing
            .get("shot")
            .and_then(Value::as_object)
            .and_then(shot_view_from_dict),
        notes: list(analysis.get("notes")).iter().map(py_str).collect(),
    })
}

/// `get_session_summary`: aggregates over a session's analyzed swings and the shots attached to
/// them, or `None` only when the session directory does not exist. A session with nothing analyzed
/// yet is a real answer, not a missing one.
pub fn get_session_summary(sessions_dir: &Path, session_id: &str) -> Option<SessionDetail> {
    if !sessions_dir.join(session_id).exists() {
        return None;
    }
    let manifests = SwingBundleStore::new(sessions_dir).get_session(session_id);

    let mut scores: Vec<f64> = Vec::new();
    let mut headlines = Vec::new();
    let mut measured: BTreeMap<String, usize> = BTreeMap::new();
    let mut passed: BTreeMap<String, usize> = BTreeMap::new();
    let mut unscored_counts: BTreeMap<String, usize> = BTreeMap::new();
    // Sorted, because only `sorted(metric_totals.items())` is answered.
    let mut metric_totals: BTreeMap<String, Vec<f64>> = BTreeMap::new();
    let (mut shots_attached, mut shots_flagged, mut mishits_excluded) = (0, 0, 0);
    // `_MISHIT_SKIP`: the shot fields a CONFIRMED mishit is held out on. `contracts::mishit` names
    // the measurements with a `_yds` suffix; the shot fields they are read off do not carry it.
    let mishit_skip: Vec<&str> = MISHIT_EXCLUDED_METRICS
        .iter()
        .map(|name| name.strip_suffix("_yds").unwrap_or(name))
        .collect();

    for manifest in &manifests {
        let Some(view) = get_swing(sessions_dir, session_id, &manifest.swing_id) else {
            continue;
        };
        let Some(score) = view.overall_score else {
            continue;
        };
        scores.push(score);
        if let Some(headline) = &view.headline {
            headlines.push(headline.clone());
        }
        for checkpoint in &view.checkpoints {
            *measured.entry(checkpoint.name.clone()).or_default() += 1;
            if checkpoint.passed == Some(true) {
                *passed.entry(checkpoint.name.clone()).or_default() += 1;
            }
        }
        for entry in &view.unscored {
            *unscored_counts.entry(entry.name.clone()).or_default() += 1;
        }
        let Some(shot) = &view.shot else {
            continue;
        };
        shots_attached += 1;
        if shot.needs_review {
            // A flagged parse must not move an average it would silently poison.
            shots_flagged += 1;
            continue;
        }
        // The manual verdict only: a session is too small to hold one club's distribution, so the
        // automatic rule the corpus applies is deliberately not run here (ADR-028 §3).
        let topped = manifest.mishit == Some(MishitVerdict::Confirmed);
        if topped {
            mishits_excluded += 1;
        }
        for (key, value) in shot.metrics.iter() {
            if topped && mishit_skip.contains(&key) {
                continue;
            }
            if let Metric::Number(value) = value {
                metric_totals
                    .entry(key.to_string())
                    .or_default()
                    .push(*value);
            }
        }
    }

    Some(SessionDetail {
        session_id: session_id.to_string(),
        swing_count: manifests.len(),
        analyzed_count: scores.len(),
        mean_score: mean(&scores).map(|m| pyfmt::round_to(m, 1)),
        best_score: first_extreme(&scores, |x, best| x > best).map(|m| pyfmt::round_to(m, 1)),
        worst_score: first_extreme(&scores, |x, worst| x < worst).map(|m| pyfmt::round_to(m, 1)),
        checkpoint_pass_rates: measured
            .iter()
            .map(|(name, count)| {
                let passed = passed.get(name).copied().unwrap_or(0);
                (name.clone(), format!("{passed}/{count}"))
            })
            .collect(),
        unscored_counts,
        shots_attached,
        shots_needing_review: shots_flagged,
        mishits_excluded,
        shot_averages: metric_totals
            .iter()
            .filter_map(|(key, values)| Some((key.clone(), pyfmt::round_to(mean(values)?, 1))))
            .collect(),
        headlines,
    })
}

/// `sum(values) / len(values)`, with CPython's compensated `sum` (3.12 on), or `None` for nothing.
/// The session-summaries case's half-even mean is what the compensation and `round_to` decide.
fn mean(values: &[f64]) -> Option<f64> {
    (!values.is_empty()).then(|| pyfmt::sum(values) / values.len() as f64)
}

/// Python's `max` or `min` over floats: the **first** extreme, replaced only by a strictly better
/// one (ADR-032 §3's fifth edge).
fn first_extreme(values: &[f64], better: impl Fn(f64, f64) -> bool) -> Option<f64> {
    let (&first, rest) = values.split_first()?;
    Some(
        rest.iter()
            .fold(first, |best, &x| if better(x, best) { x } else { best }),
    )
}

// -------------------------------------------------------------------------------------------------
// Shots
// -------------------------------------------------------------------------------------------------

/// `recent_shots`: the most recent `count` shots, newest first, each as its view. [M29 P11]
pub fn recent_shots(source: &ShotSource, count: i64) -> Vec<ShotView> {
    source.recent(count).iter().map(shot_view).collect()
}

/// `get_shot`: one shot by id, or `None`. [M29 P11]
///
/// A linear scan of the oldest-first stream, as Python's is: the store is keyed by photo hash, not
/// by shot id, so there is no index to consult, and a range session's worth of files is small. The
/// scan's order decides which of two shots sharing an id is answered (`crate::source`).
pub fn get_shot(source: &ShotSource, shot_id: &str) -> Option<ShotView> {
    source
        .stream()
        .find(|shot| shot.shot_id == shot_id)
        .map(|shot| shot_view(&shot))
}

/// A numeric shot field, read off the typed shot.
pub type Reading = fn(&ShotData) -> Option<f64>;

/// A categorical shot field, in the source's own words.
pub type ReadOut = fn(&ShotData) -> Option<&str>;

/// `_METRIC_FIELDS`: the `ShotData` fields that are metrics rather than identity or provenance.
///
/// **Frozen Python's fifteen in its order, with M32's seven joined** (the program plan's §M32 sends
/// them here, since Python's list never sees them) [M29 P11]. Each of M32's goes after the last of
/// Python's of its kind: the six numbers after `apex_height`, in `ShotData`'s order, and
/// `impact_position_v` after `impact_position`, its horizontal partner. So the numbers still come
/// first and the read-outs last, and Python's fifteen keep their relative order in the text a model
/// reads. None of the seven is on any stored shot yet, so the list's growth moves no recorded
/// answer (the module doc). M42's two after M32's, for the same reason: no stored shot holds
/// either, so no recorded answer moves. `backspin` and `sidespin` are Full Swing's printed spin,
/// shown as printed, and the list grows by them so a client sees what the screen said.
///
/// Typed reads rather than Python's `getattr` by name, so a field renamed in `ShotData` fails to
/// compile here; the unit test below is the other direction, a field added there and missing here.
pub const NUMERIC_METRICS: [(&str, Reading); 21] = [
    ("club_head_speed", |s| s.club_head_speed),
    ("club_face_angle", |s| s.club_face_angle),
    ("club_path", |s| s.club_path),
    ("ball_speed", |s| s.ball_speed),
    ("launch_angle", |s| s.launch_angle),
    ("launch_direction", |s| s.launch_direction),
    ("spin_rate", |s| s.spin_rate),
    ("spin_axis", |s| s.spin_axis),
    ("smash_factor", |s| s.smash_factor),
    ("carry_distance", |s| s.carry_distance),
    ("total_distance", |s| s.total_distance),
    ("bounce_and_roll", |s| s.bounce_and_roll),
    ("apex_height", |s| s.apex_height),
    ("attack_angle", |s| s.attack_angle),
    ("dynamic_loft", |s| s.dynamic_loft),
    ("low_point", |s| s.low_point),
    ("impact_offset_h", |s| s.impact_offset_h),
    ("impact_offset_v", |s| s.impact_offset_v),
    ("carry_offline", |s| s.carry_offline),
    ("backspin", |s| s.backspin),
    ("sidespin", |s| s.sidespin),
];

/// The categorical read-outs at the end of the list, in the source's own words: Python's two, then
/// M32's vertical partner of `impact_position`.
pub const TEXT_METRICS: [(&str, ReadOut); 3] = [
    ("shot_type", |s| s.shot_type.as_deref()),
    ("impact_position", |s| s.impact_position.as_deref()),
    ("impact_position_v", |s| s.impact_position_v.as_deref()),
];

/// `_shot_view`, with the warnings a golfer is shown (the module doc's first departure).
pub fn shot_view(shot: &ShotData) -> ShotView {
    let mut metrics = Ordered::default();
    for (name, read) in NUMERIC_METRICS {
        if let Some(value) = read(shot) {
            metrics.insert(name, Metric::Number(value));
        }
    }
    for (name, read) in TEXT_METRICS {
        if let Some(value) = read(shot) {
            metrics.insert(name, Metric::Text(value.to_string()));
        }
    }
    let provenance = shot.provenance.as_ref();
    ShotView {
        shot_id: shot.shot_id.clone(),
        session_id: shot.session_id.clone(),
        source: shot.source.as_str().to_string(),
        needs_review: provenance.is_some_and(|p| p.needs_review),
        warnings: provenance
            .map(|p| screen::golfer_warnings(&p.warnings))
            .unwrap_or_default(),
        parse_confidence: provenance.map(|p| p.parse_confidence),
        metrics,
    }
}

/// `_shot_view_from_dict`: the shot embedded in `analysis.json`, read through `ShotData`, so a
/// stored shot that no longer validates is dropped rather than half-reported.
fn shot_view_from_dict(raw: &Map<String, Value>) -> Option<ShotView> {
    serde_json::from_value::<ShotData>(Value::Object(raw.clone()))
        .ok()
        .map(|shot| shot_view(&shot))
}

// -------------------------------------------------------------------------------------------------
// Internals
// -------------------------------------------------------------------------------------------------

/// `_summarize`: one listing row, off the state sidecar when there is one.
fn summarize(manifest: &SwingManifest, swing_dir: &Path) -> SwingSummary {
    let mut roles: Vec<String> = manifest
        .roles
        .keys()
        .map(|role| role.as_str().to_string())
        .collect();
    // `sorted(role.value …)`: by wire name, which is not `Role`'s declaration order.
    roles.sort();
    if let Some(state) = load_state(swing_dir) {
        return SwingSummary {
            swing_id: manifest.swing_id.clone(),
            session_id: manifest.session_id.clone(),
            status: status_of(Some(&state), swing_dir),
            score: state.score,
            // The sidecar's own, unfiltered: Python reads `state.headline` as it stands.
            headline: state.headline,
            roles,
            partial: state.partial,
        };
    }
    // No sidecar: nothing analyzed, or analyzed by `analyze_bundle.py` before the state file
    // existed (the 2026-08-07 sessions). One `analysis.json` parse on those swings only, rather
    // than a listing saying "not analyzed" of a swing `get_swing` answers a moment later.
    //
    // **An empty object is "not analyzed" here** and "done" in [`status_of`], because Python tests
    // `_dict(…)`'s truthiness here and `is None` there. Kept, not reconciled: both are recorded.
    let analysis = load_analysis(swing_dir).unwrap_or_default();
    SwingSummary {
        swing_id: manifest.swing_id.clone(),
        session_id: manifest.session_id.clone(),
        status: if analysis.is_empty() {
            "not analyzed"
        } else {
            "done"
        }
        .to_string(),
        score: number(object(analysis.get("swing")).get("overall_score")),
        headline: text(object(analysis.get("feedback")).get("headline")),
        roles,
        partial: false,
    }
}

/// `_status_of`: the swing's analysis status, with staleness folded in. A re-uploaded clip changes
/// a hash, the recorded inputs stop matching the manifest, and the stored score stops describing
/// the files on disk, so `done` would hand out a number for footage no longer present.
fn status_of(state: Option<&AnalysisState>, swing_dir: &Path) -> String {
    let Some(state) = state else {
        return if load_analysis(swing_dir).is_none() {
            "not analyzed"
        } else {
            "done"
        }
        .to_string();
    };
    if let Some(manifest) = load_manifest(&manifest_path(swing_dir)) {
        if !state.matches(&manifest) {
            return "stale — inputs changed since this was analyzed".to_string();
        }
    }
    wire_name(&state.status)
}

/// An enum's wire name, through its own `Serialize`, so no name is retyped here. `crate::flight`
/// names its reasons and shapes through it too.
pub(crate) fn wire_name<T: Serialize>(value: &T) -> String {
    match serde_json::to_value(value) {
        Ok(Value::String(name)) => name,
        other => unreachable!("a unit enum serializes to its name, not {other:?}"),
    }
}

/// `_measurements`: the non-placement, non-pivot, measured half of `swing.measurements`, flattened
/// to name → value. Unit and detail are dropped on purpose: for a pose metric they are provenance
/// for a derivation step, which a coaching answer cannot act on. A repeated name keeps its first
/// place and its last value, as a `dict` does.
fn measurements(swing: &Map<String, Value>) -> Ordered<f64> {
    let mut flat = Ordered::default();
    for entry in list(swing.get("measurements"))
        .iter()
        .filter_map(Value::as_object)
    {
        let (Some(name), Some(value)) = (
            entry.get("name").and_then(Value::as_str),
            number(entry.get("value")),
        ) else {
            continue;
        };
        if contracts::placements::placement_by_name(name).is_some()
            || contracts::pivots::pivot_by_name(name).is_some()
            || is_modelled(entry)
        {
            continue;
        }
        flat.insert(name, value);
    }
    flat
}

/// Whether one stored measurement row is a model's output rather than a reading. A row with no
/// `source` at all stays measured: that is every artifact written before provenance was recorded.
fn is_modelled(entry: &Map<String, Value>) -> bool {
    entry
        .get("source")
        .and_then(Value::as_str)
        .is_some_and(|source| source.starts_with(MODEL_SOURCE_PREFIX))
}

/// `_simulated`: the `model:`-sourced rows, with the unit and detail `measurements` drops. A row
/// missing either is still returned with what it has.
fn simulated(swing: &Map<String, Value>) -> Vec<SimulatedView> {
    list(swing.get("measurements"))
        .iter()
        .filter_map(Value::as_object)
        .filter(|entry| is_modelled(entry))
        .filter_map(|entry| {
            let name = entry.get("name").and_then(Value::as_str)?;
            let value = number(entry.get("value"))?;
            // `str(entry.get(key, ""))`: an absent key is empty, and a present `null` is `"None"`.
            let field = |key: &str| entry.get(key).map(py_str).unwrap_or_default();
            Some(SimulatedView {
                name: name.to_string(),
                value,
                unit: field("unit"),
                source: field("source"),
                detail: field("detail"),
            })
        })
        .collect()
}

/// `_version`: the engine generation that wrote this artifact, or `None` when it does not say.
/// Any integer, negative included, and never a bool or a float: where `stored_analysis_version`
/// reads an unreadable version as 0 for a staleness check, a model told 0 would report a number,
/// and there is no engine 0.
fn version(analysis: &Map<String, Value>) -> Option<Number> {
    match analysis.get("analysis_version") {
        Some(Value::Number(n)) if n.is_i64() || n.is_u64() => Some(n.clone()),
        _ => None,
    }
}

/// `_alignment_view`: the tier, and on every tier but `full` the sentence a reader needs beside it.
/// `synchronized` ranks above `full` and still carries it: a heard strike pins one instant on a
/// shared clock and says nothing about the instants between the anchors.
fn alignment_view(alignment: &Map<String, Value>) -> (Option<String>, Option<String>) {
    let Some(raw) = alignment.get("quality").and_then(Value::as_str) else {
        return (None, None);
    };
    let Ok(quality) = serde_json::from_value::<AlignmentQuality>(Value::String(raw.to_string()))
    else {
        return (Some(raw.to_string()), None);
    };
    if quality == AlignmentQuality::Full {
        return (Some(wire_name(&quality)), None);
    }
    let summary =
        text(alignment.get("quality_summary")).unwrap_or_else(|| quality.summary().to_string());
    // `str.format(summary=…)` over a template whose only field is `{summary}`.
    let caveat = ALIGNMENT_CAVEAT.replace("{summary}", &summary);
    (Some(wire_name(&quality)), Some(caveat))
}

/// `_checkpoint`.
fn checkpoint(raw: &Value) -> CheckpointView {
    let data = object(Some(raw));
    CheckpointView {
        name: py_str(data.get("name").unwrap_or(&Value::String(String::new()))),
        observed: number(data.get("observed")),
        expected_low: number(data.get("expected_low")),
        expected_high: number(data.get("expected_high")),
        passed: data.get("passed").and_then(Value::as_bool),
        score: number(data.get("score")),
        percentile: number(data.get("percentile")),
        population_n: match data.get("population_n") {
            Some(Value::Number(n)) if n.is_i64() || n.is_u64() => Some(n.clone()),
            // `isinstance(True, int)` is true, so Python keeps a bool and pydantic's lax `int`
            // reads it as 1 or 0 (measured, pydantic 2.13.4).
            Some(Value::Bool(b)) => Some(Number::from(u8::from(*b))),
            _ => None,
        },
        message: text(data.get("message")),
        one_sided: data.get("one_sided").and_then(Value::as_bool),
    }
}

/// `_tip`.
fn tip(raw: &Value) -> TipView {
    let data = object(Some(raw));
    let field = |key: &str| py_str(data.get(key).unwrap_or(&Value::String(String::new())));
    TipView {
        checkpoint: field("checkpoint"),
        text: field("text"),
        severity: text(data.get("severity")),
    }
}

/// `_unscored`: one entry in the current shape, or the names-only one every artifact before
/// 2026-08-19 holds, which means "this file does not know why" and so reads as `unrecorded`. A
/// reason this build does not know reads the same way rather than failing: dropping the entry would
/// make a swing judged on five fundamentals read as one judged on six. (The module doc says which
/// reasons are known here and not to Python.)
fn unscored(raw: &Value) -> UnscoredView {
    let (name, reason) = match raw {
        Value::String(name) => (name.clone(), UnscoredReason::Unrecorded),
        other => {
            let data = object(Some(other));
            let empty = Value::String(String::new());
            let raw_reason = py_str(data.get("reason").unwrap_or(&empty));
            let reason = serde_json::from_value(Value::String(raw_reason))
                .unwrap_or(UnscoredReason::Unrecorded);
            (py_str(data.get("name").unwrap_or(&empty)), reason)
        }
    };
    let spec = spec_for(reason);
    UnscoredView {
        name,
        reason: wire_name(&reason),
        why: spec.summary.to_string(),
        refilming_helps: spec.refilming_helps,
        family: family_of(reason),
    }
}

/// `_dict`: the value when it is an object, else an empty one.
fn object(value: Option<&Value>) -> &Map<String, Value> {
    static EMPTY: std::sync::OnceLock<Map<String, Value>> = std::sync::OnceLock::new();
    value
        .and_then(Value::as_object)
        .unwrap_or_else(|| EMPTY.get_or_init(Map::new))
}

/// `_list`: the value when it is a list, else an empty one.
fn list(value: Option<&Value>) -> &[Value] {
    value
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

/// `_number`: an int or a float as a float, and nothing else (`bool` is its own JSON type here).
fn number(value: Option<&Value>) -> Option<f64> {
    value.and_then(Value::as_f64)
}

/// `_text`: a non-empty string, else `None`.
fn text(value: Option<&Value>) -> Option<String> {
    value
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

/// Python's `str()` of a value `json.loads` produced, where a field reads a name it expects to be
/// a string: the string itself, an integer in decimal, a float as its `repr`, `True`, `False` and
/// `None`, and a container as its repr. An object's keys come out sorted, because `serde_json`'s
/// maps are, where Python's keep the file's order (the module doc).
pub fn py_str(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        other => py_repr(other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn str_of_a_parsed_value_is_pythons() {
        for (value, want) in [
            (json!("tempo"), "tempo"),
            (json!(5), "5"),
            (json!(1.0), "1.0"),
            (json!(1e-5), "1e-05"),
            (json!(true), "True"),
            (json!(null), "None"),
            (json!(["a", 1, null, "it's"]), "['a', 1, None, \"it's\"]"),
            (json!({"b": 1, "a": [false]}), "{'a': [False], 'b': 1}"),
        ] {
            assert_eq!(py_str(&value), want, "{value}");
        }
    }

    #[test]
    fn a_miss_quotes_its_ids_as_python_does() {
        assert_eq!(
            missing_swing("it's", "9").message,
            "No swing '9' in session \"it's\". Call list_sessions to see which sessions and \
             swing ids exist."
        );
        assert!(!missing_session("x").found);
    }

    /// The tolerant reads no recorded case reaches: a bool `population_n` is pydantic's 1 or 0, a
    /// non-object entry reads as an empty one, a non-string name is `str()`'d, and a reason
    /// nothing names is `unrecorded`.
    #[test]
    fn odd_entries_read_as_pythons_do() {
        let view = checkpoint(&json!({"name": 7, "population_n": true, "passed": 1}));
        assert_eq!(view.name, "7");
        assert_eq!(view.population_n, Some(Number::from(1)));
        assert_eq!(view.passed, None, "only a bool is a verdict");
        assert_eq!(checkpoint(&json!(3)).name, "");
        assert_eq!(tip(&json!({"text": null})).text, "None");

        let entry = unscored(&json!({"name": "tempo", "reason": "invented_later"}));
        assert_eq!(entry.reason, "unrecorded");
        assert!(!entry.refilming_helps);
        assert_eq!(unscored(&json!(4)).name, "");
        // Rust's own reasons are known (the module doc), where Python's enum reads them unrecorded.
        let photo = unscored(&json!({"name": "carry", "reason": "misread"}));
        assert_eq!(
            (photo.reason.as_str(), photo.refilming_helps),
            ("misread", true)
        );
    }

    #[test]
    fn the_first_extreme_wins_and_a_mean_is_compensated() {
        assert_eq!(
            first_extreme(&[1.0, 3.0, 3.0, 2.0], |x, b| x > b),
            Some(3.0)
        );
        assert_eq!(first_extreme(&[], |x, b| x < b), None);
        assert_eq!(mean(&[0.1; 10]), Some(0.1));
    }

    #[test]
    fn a_measurement_repeated_keeps_its_place_and_its_last_value() {
        let Value::Object(swing) = json!({"measurements": [
            {"name": "a", "value": 1},
            {"name": "b", "value": 2.5},
            {"name": "a", "value": 3, "source": "pose:face_on"},
            {"name": "c", "value": false},
            {"name": "flight_carry_yds", "value": 120, "source": "model:flight_v1"},
            {"name": "tour_joint_distance", "value": 2.0},
        ]}) else {
            unreachable!()
        };
        let flat = measurements(&swing);
        let flat: Vec<(&str, f64)> = flat.iter().map(|(k, v)| (k, *v)).collect();
        assert_eq!(flat, [("a", 3.0), ("b", 2.5)]);
        let rows = simulated(&swing);
        assert_eq!(rows.len(), 1);
        assert_eq!((rows[0].unit.as_str(), rows[0].detail.as_str()), ("", ""));
    }

    /// `_METRIC_FIELDS` split against `_NON_METRIC_FIELDS` exhaustively, as `test_query.py` splits
    /// Python's: every metric is a key a stored shot carries, and the keys left over are exactly the
    /// five that reach a client through the view's own fields. A field added to `ShotData` and not
    /// here fails this, which is the direction that breaks silently. [M29 P11: M32's seven joined]
    ///
    /// The shot's keys are its default serialization's ∪ `WRITTEN_WHEN_SET`: M42's two spin
    /// readings are omitted while unset (the M42 plan's call 1), and are fields all the same.
    /// [M42 P1]
    #[test]
    fn the_metric_list_and_the_view_s_own_fields_split_every_shot_field() {
        let shot: ShotData = serde_json::from_value(json!({
            "shot_id": "s", "session_id": "d", "timestamp": "2026-10-01T00:00:00Z"
        }))
        .unwrap();
        let Value::Object(keys) = serde_json::to_value(&shot).unwrap() else {
            unreachable!()
        };
        let mut rest: Vec<&str> = keys
            .keys()
            .map(String::as_str)
            .chain(contracts::shot::WRITTEN_WHEN_SET)
            .collect();
        for name in NUMERIC_METRICS
            .iter()
            .map(|(n, _)| *n)
            .chain(TEXT_METRICS.iter().map(|(n, _)| *n))
        {
            let at = rest.iter().position(|k| *k == name);
            assert!(at.is_some(), "{name} is not a ShotData field");
            rest.remove(at.unwrap());
        }
        rest.sort_unstable();
        assert_eq!(
            rest,
            ["provenance", "session_id", "shot_id", "source", "timestamp"]
        );
        // Frozen Python's `_METRIC_FIELDS`, which leads each list in its order (the list's doc).
        let numeric: Vec<&str> = NUMERIC_METRICS.iter().map(|(n, _)| *n).collect();
        let text: Vec<&str> = TEXT_METRICS.iter().map(|(n, _)| *n).collect();
        let pythons: Vec<&str> = numeric[..13].iter().chain(&text[..2]).copied().collect();
        assert_eq!(
            pythons,
            [
                "club_head_speed",
                "club_face_angle",
                "club_path",
                "ball_speed",
                "launch_angle",
                "launch_direction",
                "spin_rate",
                "spin_axis",
                "smash_factor",
                "carry_distance",
                "total_distance",
                "bounce_and_roll",
                "apex_height",
                "shot_type",
                "impact_position",
            ]
        );
    }

    /// The two departures, on one shot: M32's fields reach `metrics` in the list's order, and the
    /// warnings lose the layout line and keep the rest in order, the tie warning included.
    #[test]
    fn a_shot_view_carries_m32_s_fields_and_a_golfer_s_warnings() {
        let shot: ShotData = serde_json::from_value(json!({
            "shot_id": "s", "session_id": "d", "timestamp": "2026-10-01T00:00:00Z",
            "source": "screen", "carry_distance": 150.0, "shot_type": "DRAW",
            "attack_angle": -4.0, "carry_offline": -8.0, "impact_position_v": "LOW",
            "provenance": {"device": "synthetic", "parse_confidence": 0.9, "warnings": [
                "no tile found for 'Bounce & Roll'",
                "Club Path: label tie between 'Club Path' and 'Club Speed' - not read",
                "Spin: no value text under the label",
            ]},
        }))
        .unwrap();
        let view = shot_view(&shot);
        let keys: Vec<&str> = view.metrics.iter().map(|(k, _)| k).collect();
        assert_eq!(
            keys,
            [
                "carry_distance",
                "attack_angle",
                "carry_offline",
                "shot_type",
                "impact_position_v"
            ]
        );
        assert_eq!(
            view.warnings,
            [
                "Club Path: label tie between 'Club Path' and 'Club Speed' - not read",
                "Spin: no value text under the label",
            ]
        );
        assert_eq!(
            missing_shot("it's").message,
            "No shot with id \"it's\". Call get_recent_shots to see which shot ids exist."
        );
    }

    /// The two keys the coaching brief reads (the module doc, M29 P17): the stored sidedness, and
    /// the family read off the reason, where a reason this build does not know, or the names-only
    /// shape, is `unrecorded` and so a checkpoint.
    #[test]
    fn the_brief_s_two_keys_read_the_score_and_the_reason() {
        assert_eq!(
            checkpoint(&json!({"name": "tempo", "one_sided": false})).one_sided,
            Some(false)
        );
        assert_eq!(
            checkpoint(&json!({"name": "head_sway", "one_sided": true})).one_sided,
            Some(true)
        );
        assert_eq!(
            checkpoint(&json!({"name": "tempo", "one_sided": 1})).one_sided,
            None
        );
        assert_eq!(checkpoint(&json!({"name": "tempo"})).one_sided, None);

        let family = |raw: Value| unscored(&raw).family;
        assert_eq!(
            family(json!({"name": "flight_carry_yds", "reason": "carry_unreachable"})),
            "flight"
        );
        assert_eq!(
            family(json!({"name": "tempo", "reason": "no_band"})),
            "checkpoint"
        );
        assert_eq!(
            family(json!({"name": "tempo", "reason": "not_a_reason"})),
            "checkpoint"
        );
        assert_eq!(family(json!("tempo")), "checkpoint");
        for (reason, _) in contracts::unscored::UNSCORED_REASONS {
            let want = if is_inference_reason(*reason) {
                FLIGHT_FAMILY
            } else {
                CHECKPOINT_FAMILY
            };
            assert_eq!(family_of(*reason), want, "{reason:?}");
        }
    }
}
