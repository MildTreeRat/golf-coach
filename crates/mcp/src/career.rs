//! Career mode, in the shapes an MCP tool returns. `src/golf_coach/mcp/career.py`. [M29 P12]
//!
//! The companion to [`crate::query`], and a separate module for Python's reason: that one is pure
//! reads over the two stores, and these tools read a corpus and then run it through three analysis
//! layers — the guarded baseline, the bias and scatter discriminator, and the tour join
//! (`analysis::{baseline, dispersion, comparison}`, the same builders `golf-core`'s career verbs
//! print). Nothing here touches the protocol; `crate::tools` puts a view on the wire.
//!
//! **These views carry claims about a golfer, and the guard decides whether a claim may be made.**
//! A withheld claim ships as `None`, and a refusal keeps its evidence (`n`, `n_sessions`, the
//! per-session counts), because "you have 9 shots over 2 sessions, a trend needs 12 over 3" is what
//! makes a refusal actionable. [`THE_UNBLOCK`] says in the payload not to reconstruct the refused
//! figure from that evidence, which is Python's module doc at length.
//!
//! **The window and the two sessions are the same operation**: `get_shot_trends` narrows the corpus
//! by date and `compare_sessions` by session, and both hand the result to the one
//! `build_baseline`, so a per-session mean faces exactly the guard a pooled one does.
//!
//! # The corpus is Rust's, and it differs from frozen Python's on purpose
//!
//! Every tool reads [`storage::corpus::read_corpus`] under this build's [`versions`], as the career
//! verbs do, so the corpus these views count is ADR-024's M35 and M36 addenda's, not frozen
//! Python's:
//!
//! - **A swing is outdated only below `COMPARABLE_FROM`**, not below the installed engine, so a
//!   swing whose stored numbers today's engine still agrees with pools.
//! - **A photo-only entry is admitted**, read from its `shot_analysis.json`, and deduped by photo
//!   with the filmed copy winning.
//! - **A drill is `untracked`**: it is in no count, no aggregate and no `n`.
//!
//! No Python-recorded case reaches any of the three: every recorded tree was written by frozen
//! Python, at its own engine version, with no photo entry and no intent, so every recorded count
//! reproduces. `spec/vectors/mcp/hand/career-corpus.json` is the case that reaches all three
//! through these tools (the M29 plan's P12).
//!
//! **Two of Python's sentences would deny the drill**, because each lists every reason Python's
//! corpus had for a missing sample, and so each gains it: the "no sample" note of
//! [`shot_trends`] ([`DRILL_IN_THE_NOTE`], declared by path in `crate::family`, since it moves a
//! recorded answer), and the sampleless sentence of [`compare_sessions`], which would call a
//! session of drills re-uploads (the hand case alone reaches it).
//!
//! # The clock is a parameter
//!
//! Only `get_shot_trends` with `days` reads "now" (the plan's P8 finding 8), and it reads it after
//! the golfer resolves, as Python's does, so a miss never touches the clock. A live server reads the
//! system's; a recorded call reads its own `now`, and one recorded with none must not read one at
//! all ([`Clock::Unset`]).

use std::collections::BTreeSet;
use std::path::Path;

use analysis::baseline::build_baseline;
use analysis::comparison::build_standing;
use analysis::dispersion::build_dispersion;
use contracts::baseline::{BaselineClaim, Interval, MetricBaseline, WithheldClaim};
use contracts::career::{CareerCorpus, ExclusionReason, Narrowing};
use contracts::comparison::MetricComparison;
use contracts::dispersion::MetricDispersion;
use contracts::golfer::Golfer;
use contracts::mishit::MISHIT_EXCLUDED_METRICS;
use contracts::swing::{ANALYSIS_VERSION, COMPARABLE_FROM};
use contracts::time::Timestamp;
use serde::Serialize;
use storage::corpus::{narrow_to, read_corpus, EngineVersions};
use storage::golfer_store::{slugify, GolferStore};

use crate::query::{self, NotFound};
use crate::server::Clock;

/// The one sentence every refusal in this module resolves to. In the payload rather than left to
/// the tool's description, because a description is read once on connect and a payload is read at
/// the moment the model is deciding what to say.
pub const THE_UNBLOCK: &str = "The data does not exist yet — this needs a bay session of 20-30 \
                               swings with shots attached. Report the refusal. Do not estimate \
                               the withheld figure from the sample counts below it.";

/// Attached to every comparison, whatever the `n`: a score is a mean over the checkpoints that
/// survived (ADR-013), so two sessions can differ because a checkpoint went unmeasured in one, and
/// nothing here tests a difference for significance.
pub const COMPARE_NOTE: &str = "A difference between two sessions is not by itself evidence that \
                                anything changed. Scores are means over the checkpoints that could \
                                be measured, so a session where one went unmeasured scores on \
                                fewer fundamentals than one where it did not, and no significance \
                                test is applied to any difference here. Per-metric means are \
                                absent unless each session on its own carried enough swings to \
                                support one.";

/// The reasons frozen Python's "no sample" note ends on: every reason its corpus had for a metric
/// to carry no sample. `crate::family` declares the rewrite of the recorded note from this to
/// `stale, ` and [`DRILL_IN_THE_NOTE`].
pub const FROZEN_NOTE_REASONS: &str = "stale, or was analyzed by an older engine.";

/// The same list ended with the reason Rust's corpus adds, the drill, which the note would
/// otherwise deny (`shot_trends`).
pub const DRILL_IN_THE_NOTE: &str =
    "was analyzed by an older engine, or was hit as a drill, which counts toward no stat.";

/// The engine generations every corpus here is read under: this build's, as `golf-core`'s career
/// verbs read theirs (`career_version` 1).
pub fn versions() -> EngineVersions {
    EngineVersions {
        installed: ANALYSIS_VERSION,
        comparable_from: COMPARABLE_FROM,
    }
}

// -------------------------------------------------------------------------------------------------
// The views, each in its pydantic model's field order
// -------------------------------------------------------------------------------------------------

/// A claim the guard declined, flattened from [`WithheldClaim`] for the wire.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Refusal {
    /// `center | spread | trend`.
    pub claim: String,
    pub have_n: i64,
    pub need_n: i64,
    pub have_sessions: i64,
    pub need_sessions: i64,
    /// What is missing, in a sentence.
    pub reason: String,
}

/// One session's contribution to one metric. Evidence, and only conditionally a claim.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SessionPoint {
    pub session_id: String,
    /// CPython's `isoformat()` of the session's earliest arrival, `+00:00` and all.
    pub captured_at: String,
    /// Distinct contributing artifacts from this session.
    pub n: i64,
    /// Present only when the trend guard allowed it.
    pub mean: Option<f64>,
}

/// One metric, everything career mode knows about it, and every refusal standing in the gaps.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MetricProfile {
    pub name: String,
    pub unit: String,
    pub source: String,
    pub n: i64,
    pub n_sessions: i64,

    // What their own history says (`contracts::baseline`).
    pub center: Option<f64>,
    pub center_ci_low: Option<f64>,
    pub center_ci_high: Option<f64>,
    pub median: Option<f64>,
    pub sd: Option<f64>,
    pub sd_ci_low: Option<f64>,
    pub sd_ci_high: Option<f64>,
    pub minimum: Option<f64>,
    pub maximum: Option<f64>,

    // What the shape of the miss points at (`contracts::dispersion`).
    pub bias: String,
    pub scatter: String,
    pub pattern: Option<String>,
    pub points_at: Option<String>,
    pub target: Option<f64>,
    pub tolerance: Option<f64>,
    pub within_session_sd: Option<f64>,

    // Where that sits in the tour population (`contracts::comparison`).
    pub tour_standing: String,
    pub tour_reading: Option<String>,
    pub tour_percentile: Option<f64>,
    pub tour_percentile_clamped: bool,
    pub tour_band_low: Option<f64>,
    pub tour_band_high: Option<f64>,
    pub tour_population_n: Option<i64>,
    pub tour_population_players: Option<i64>,
    pub outside_by: Option<f64>,

    pub sessions: Vec<SessionPoint>,
    pub caveats: Vec<String>,
    /// Refused for want of data. More swings fix these.
    pub withheld: Vec<Refusal>,
    /// Refused for want of a reference. More swings do not fix these.
    pub unavailable: Vec<String>,
}

/// One golfer's whole career picture, or the refusal standing in for it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GolferProfile {
    pub player_id: String,
    pub display_name: String,
    pub handedness: String,
    /// Distinct entries after re-uploads collapsed: Rust's corpus, so a photo-only entry counts and
    /// a drill does not (the module doc).
    pub distinct_swings: usize,
    pub swing_dirs_seen: i64,
    pub sessions: i64,
    pub built_from_swings: i64,
    pub mishits_excluded: usize,
    pub metrics: Vec<MetricProfile>,
    /// True when the guard refused every claim on every metric.
    pub nothing_sayable: bool,
    pub note: String,
}

/// Whether one metric has moved for one golfer, over a window.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MetricTrend {
    pub player_id: String,
    pub metric: String,
    pub unit: Option<String>,
    /// `None` means the whole history was used.
    pub window_days: Option<i64>,
    pub n: i64,
    pub n_sessions: i64,
    /// True when the trend guard filled in the per-session means.
    pub ready: bool,
    pub sessions: Vec<SessionPoint>,
    pub withheld: Option<Refusal>,
    pub note: String,
}

/// One metric across two sessions. Every number here is separately gated.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MetricDelta {
    pub metric: String,
    pub unit: String,
    pub n_a: i64,
    pub n_b: i64,
    pub mean_a: Option<f64>,
    pub mean_b: Option<f64>,
    /// `mean_b - mean_a`, only when both session means survived the guard.
    pub delta: Option<f64>,
    pub withheld: Vec<Refusal>,
}

/// Two sessions of one golfer's, held against each other.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SessionsCompared {
    pub player_id: String,
    pub session_a: String,
    pub session_b: String,
    /// From `get_session_summary`: a mean over that session's swings, every golfer's.
    pub mean_score_a: Option<f64>,
    pub mean_score_b: Option<f64>,
    pub swings_a: usize,
    pub swings_b: usize,
    pub metrics: Vec<MetricDelta>,
    pub note: String,
}

// -------------------------------------------------------------------------------------------------
// The misses
// -------------------------------------------------------------------------------------------------
//
// Every career tool starts by resolving a name, so every one can miss the same way, and the hint
// differs each time because what to do next differs.

/// `missing_golfer_profile`.
pub fn missing_golfer_profile(player: &str) -> NotFound {
    query::missing(
        format!(
            "No golfer matching {} is registered.",
            pyfmt::str_repr(player)
        ),
        "Names are matched loosely (case and punctuation are folded), so this means nobody by \
         that name has swings on file yet.",
    )
}

/// `missing_golfer_trend`.
pub fn missing_golfer_trend(player: &str) -> NotFound {
    query::missing(
        format!(
            "No golfer matching {} is registered.",
            pyfmt::str_repr(player)
        ),
        "Call get_golfer_profile with the name to see whether they have any swings on file.",
    )
}

/// `missing_golfer_comparison`.
pub fn missing_golfer_comparison(player: &str) -> NotFound {
    query::missing(
        format!(
            "No golfer matching {} is registered.",
            pyfmt::str_repr(player)
        ),
        "compare_sessions needs a golfer, because a session's swings can belong to more than one \
         person. Call get_golfer_profile to see who is on file.",
    )
}

// -------------------------------------------------------------------------------------------------
// The three readers
// -------------------------------------------------------------------------------------------------

/// `resolve_golfer`: a typed name or a stored id to a registered golfer. `slugify` folds both onto
/// one id, which is what makes "Aaron" and "aaron" one history rather than two with half the data
/// each. A name that slugs to nothing is no golfer.
pub fn resolve_golfer(golfers_dir: &Path, player: &str) -> Option<Golfer> {
    let player_id = slugify(player);
    if player_id.is_empty() {
        return None;
    }
    GolferStore::new(golfers_dir).get(&player_id)
}

fn corpus_of(sessions_dir: &Path, golfer: &Golfer) -> CareerCorpus {
    read_corpus(sessions_dir, &golfer.player_id, versions())
}

/// `golfer_profile`: everything one golfer's history supports saying, and every refusal. `None` if
/// the golfer is unknown. All three layers are built over one corpus, so the baseline, the findings
/// and the tour placement describe the same swings.
pub fn golfer_profile(
    sessions_dir: &Path,
    golfers_dir: &Path,
    player: &str,
) -> Option<GolferProfile> {
    let golfer = resolve_golfer(golfers_dir, player)?;
    let corpus = corpus_of(sessions_dir, &golfer);
    let baseline = build_baseline(&corpus);
    let dispersion = build_dispersion(&corpus);
    let standing = build_standing(&corpus);

    let mut metrics: Vec<MetricProfile> = baseline
        .metrics
        .iter()
        .map(|(name, metric)| {
            profile(
                metric,
                dispersion.metrics.get(name),
                standing.metrics.get(name),
            )
        })
        .collect();
    let mishits = corpus.mishit_shots();
    note_mishit_exclusion(&mut metrics, mishits);

    let nothing_sayable = baseline.nothing_sayable();
    Some(GolferProfile {
        player_id: golfer.player_id,
        display_name: golfer.display_name,
        handedness: golfer.handedness.as_str().to_string(),
        distinct_swings: corpus.distinct_swings(),
        swing_dirs_seen: corpus.swing_dirs_seen,
        sessions: baseline.built_from_sessions,
        built_from_swings: baseline.built_from_swings,
        mishits_excluded: mishits,
        metrics,
        nothing_sayable,
        note: if nothing_sayable {
            THE_UNBLOCK.to_string()
        } else {
            String::new()
        },
    })
}

/// Why a trend over a window could not be asked, which no recorded call meets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WindowRefused {
    /// The call was made with no clock (a recorded call whose `now` is null), and a window reads
    /// one. The recorder refused such a call, so a case holding one is malformed.
    NoClock,
    /// The recorded `now` is not a timestamp.
    UnreadableClock(String),
    /// `now - days` falls outside the years 1 to 9999, where CPython's `datetime` raises
    /// `OverflowError` and its SDK answers an error result. So does this, in its own words.
    OutOfRange { days: i64 },
}

/// `datetime.min` and `datetime.max` in UTC, as microseconds since the epoch: the instants a window
/// may start at, because Python's subtraction raises outside them.
const DATETIME_MIN_MICROS: i64 = -62_135_596_800_000_000;
const DATETIME_MAX_MICROS: i64 = 253_402_300_800_000_000 - 1;
const MICROS_PER_DAY: i64 = 86_400_000_000;

/// `datetime.now(UTC) - timedelta(days=days)`, with "now" off `clock`.
fn window_start(clock: &Clock, days: i64) -> Result<Timestamp, WindowRefused> {
    let now = match clock {
        Clock::System => Timestamp::now_utc(),
        Clock::At(text) => {
            Timestamp::parse(text).map_err(|_| WindowRefused::UnreadableClock(text.clone()))?
        }
        Clock::Unset => return Err(WindowRefused::NoClock),
    };
    days.checked_mul(MICROS_PER_DAY)
        .and_then(|back| now.utc_micros().checked_sub(back))
        .filter(|start| (DATETIME_MIN_MICROS..=DATETIME_MAX_MICROS).contains(start))
        .map(Timestamp::from_utc_micros)
        .ok_or(WindowRefused::OutOfRange { days })
}

/// `shot_trends`: whether one metric has moved across sessions. `Ok(None)` if the golfer is
/// unknown, which is decided before the clock is read.
///
/// A metric this golfer has never recorded is an empty trend rather than a miss: "no samples of
/// that" is a real answer, the posture `read_corpus` takes toward a golfer with no swings.
pub fn shot_trends(
    sessions_dir: &Path,
    golfers_dir: &Path,
    metric: &str,
    player: &str,
    days: Option<i64>,
    clock: &Clock,
) -> Result<Option<MetricTrend>, WindowRefused> {
    let Some(golfer) = resolve_golfer(golfers_dir, player) else {
        return Ok(None);
    };
    let mut corpus = corpus_of(sessions_dir, &golfer);
    if let Some(days) = days {
        let since = window_start(clock, days)?;
        corpus = narrow_to(
            &corpus,
            &Narrowing {
                since: Some(since),
                ..Narrowing::default()
            },
        );
    }

    let baseline = build_baseline(&corpus);
    let Some(found) = baseline.metrics.get(metric) else {
        return Ok(Some(MetricTrend {
            player_id: golfer.player_id,
            metric: metric.to_string(),
            unit: None,
            window_days: days,
            n: 0,
            n_sessions: 0,
            ready: false,
            sessions: Vec::new(),
            withheld: None,
            // Python's sentence names every reason its corpus had for a sample to be missing, and
            // Rust's corpus has one more, the drill, which it would otherwise deny. So the list
            // gains it (`DRILL_IN_THE_NOTE`, declared in `crate::family`).
            note: format!(
                "No sample of {} in this window. Either this golfer has never recorded it, or \
                 every swing carrying it is unanalyzed, stale, {DRILL_IN_THE_NOTE} Call \
                 get_golfer_profile to see which metrics do have samples.",
                pyfmt::str_repr(metric)
            ),
        }));
    };

    let ready = found.supports(BaselineClaim::Trend);
    Ok(Some(MetricTrend {
        player_id: golfer.player_id,
        metric: metric.to_string(),
        unit: Some(found.unit.clone()),
        window_days: days,
        n: found.n,
        n_sessions: found.n_sessions,
        ready,
        sessions: points(found),
        withheld: found
            .withheld
            .iter()
            .find(|r| r.claim == BaselineClaim::Trend)
            .map(refusal),
        note: if ready {
            String::new()
        } else {
            format!(
                "No trend may be described. The per-session counts below are evidence for the \
                 refusal, not a series to read a direction off. {THE_UNBLOCK}"
            )
        },
    }))
}

/// `compare_sessions`: two sessions of one golfer's, per metric and per score. `None` if the golfer
/// is unknown.
///
/// Each session is narrowed out of the same corpus and run through `build_baseline` alone, so a
/// per-session mean faces the identical CENTER guard a pooled mean faces. The scores are
/// `get_session_summary`'s, which reads the session whoever swung in it, as Python's does.
pub fn compare_sessions(
    sessions_dir: &Path,
    golfers_dir: &Path,
    session_a: &str,
    session_b: &str,
    player: &str,
) -> Option<SessionsCompared> {
    let golfer = resolve_golfer(golfers_dir, player)?;
    let corpus = corpus_of(sessions_dir, &golfer);
    let session = |id: &str| Narrowing {
        sessions: Some(vec![id.to_string()]),
        ..Narrowing::default()
    };
    let left = build_baseline(&narrow_to(&corpus, &session(session_a)));
    let right = build_baseline(&narrow_to(&corpus, &session(session_b)));

    let summary_a = query::get_session_summary(sessions_dir, session_a);
    let summary_b = query::get_session_summary(sessions_dir, session_b);

    let names: BTreeSet<&String> = left.metrics.keys().chain(right.metrics.keys()).collect();
    let metrics: Vec<MetricDelta> = names
        .into_iter()
        .map(|name| delta(name, left.metrics.get(name), right.metrics.get(name)))
        .collect();

    let note = format!(
        "{COMPARE_NOTE}{}{}",
        sampleless(&corpus, session_a, summary_a.as_ref(), &metrics, Side::A),
        sampleless(&corpus, session_b, summary_b.as_ref(), &metrics, Side::B),
    );
    Some(SessionsCompared {
        player_id: golfer.player_id,
        session_a: session_a.to_string(),
        session_b: session_b.to_string(),
        mean_score_a: summary_a.as_ref().and_then(|s| s.mean_score),
        mean_score_b: summary_b.as_ref().and_then(|s| s.mean_score),
        swings_a: summary_a.as_ref().map_or(0, |s| s.analyzed_count),
        swings_b: summary_b.as_ref().map_or(0, |s| s.analyzed_count),
        metrics,
        note,
    })
}

// -------------------------------------------------------------------------------------------------
// Internals
// -------------------------------------------------------------------------------------------------

/// `_profile`: three layers about one metric, flattened into one view.
///
/// `withheld` is the baseline's refusals alone, and `unavailable` is the dispersion layer's then the
/// tour join's, deduplicated keeping the first, as `dict.fromkeys` does. `caveats` is the
/// dispersion layer's, replaced whole, which is why the mishit sentence is appended afterwards.
fn profile(
    baseline: &MetricBaseline,
    dispersion: Option<&MetricDispersion>,
    standing: Option<&MetricComparison>,
) -> MetricProfile {
    let mut unavailable: Vec<String> = Vec::new();
    for reason in dispersion
        .map(|d| d.unavailable.as_slice())
        .unwrap_or_default()
        .iter()
        .chain(
            standing
                .map(|s| s.unavailable.as_slice())
                .unwrap_or_default(),
        )
    {
        if !unavailable.contains(reason) {
            unavailable.push(reason.clone());
        }
    }

    MetricProfile {
        name: baseline.name.clone(),
        unit: baseline.unit.clone(),
        source: baseline.source.clone(),
        n: baseline.n,
        n_sessions: baseline.n_sessions,
        center: baseline.mean,
        center_ci_low: low(baseline.mean_ci.as_ref()),
        center_ci_high: high(baseline.mean_ci.as_ref()),
        median: baseline.median,
        sd: baseline.sd,
        sd_ci_low: low(baseline.sd_ci.as_ref()),
        sd_ci_high: high(baseline.sd_ci.as_ref()),
        minimum: baseline.minimum,
        maximum: baseline.maximum,

        bias: dispersion
            .map_or("withheld", |d| d.bias.as_str())
            .to_string(),
        scatter: dispersion
            .map_or("withheld", |d| d.scatter.as_str())
            .to_string(),
        pattern: dispersion
            .and_then(|d| d.pattern)
            .map(|p| p.as_str().to_string()),
        points_at: dispersion.and_then(|d| d.points_at.clone()),
        target: dispersion.and_then(|d| d.target),
        tolerance: dispersion.and_then(|d| d.tolerance),
        within_session_sd: dispersion.and_then(|d| d.within_session_sd),

        tour_standing: standing
            .map_or("withheld", |s| s.standing.as_str())
            .to_string(),
        tour_reading: standing.and_then(|s| s.reading.clone()),
        tour_percentile: standing.and_then(|s| s.percentile),
        tour_percentile_clamped: standing.is_some_and(|s| s.percentile_clamped),
        tour_band_low: standing.and_then(|s| s.band_low),
        tour_band_high: standing.and_then(|s| s.band_high),
        tour_population_n: standing.and_then(|s| s.population_n),
        tour_population_players: standing.and_then(|s| s.population_players),
        outside_by: standing.and_then(|s| s.outside_by),

        sessions: points(baseline),
        caveats: dispersion.map(|d| d.caveats.clone()).unwrap_or_default(),
        withheld: baseline.withheld.iter().map(refusal).collect(),
        unavailable,
    }
}

/// `_note_mishit_exclusion`: the mishit sentence on carry and total distance, once the corpus holds
/// a mishit. The names are `MISHIT_EXCLUDED_METRICS`, the list `CorpusSwing::artifact_key` withholds
/// on, so the prose cannot name a metric the exclusion does not touch.
fn note_mishit_exclusion(metrics: &mut [MetricProfile], mishit_shots: usize) {
    if mishit_shots == 0 {
        return;
    }
    let (shots, were) = if mishit_shots == 1 {
        ("shot", "was")
    } else {
        ("shots", "were")
    };
    let sentence = format!(
        "{mishit_shots} {shots} {were} set aside as a mishit — a top or duff carrying far below \
         that club's own median — and left out of this average; every other metric still counts \
         them, and get_bag_profile names which club and which shots."
    );
    for metric in metrics
        .iter_mut()
        .filter(|m| MISHIT_EXCLUDED_METRICS.contains(&m.name.as_str()))
    {
        metric.caveats.push(sentence.clone());
    }
}

#[derive(Clone, Copy)]
enum Side {
    A,
    B,
}

/// `_sampleless`: explain a session that has scores and contributes no samples.
///
/// It looks like a contradiction and is not: entries are deduplicated on the face-on clip's hash,
/// so a session made entirely of re-uploads of an earlier session's clip scores normally while
/// contributing nothing to any `n`.
///
/// **Rust's corpus gives a second reason, and Python's sentence would deny it.** A drill is scored
/// by the pipeline, so the session summary counts it, and it is in no `n` (the module doc). Python's
/// sentence says the clips are re-uploads, which for a session of drills is false. So when the
/// corpus names any swing in this session `untracked`, the sentence says how many were hit as
/// drills instead, a count that is true whatever else the session holds; otherwise it is Python's,
/// which `tools/career-reuploads` records. `spec/vectors/mcp/hand/career-corpus.json` reaches the
/// drill. Python's sentence was already wrong for a session whose analyzed swings are another
/// golfer's, or are all outdated; neither is Rust's corpus departing, so neither is changed here.
fn sampleless(
    corpus: &CareerCorpus,
    session_id: &str,
    summary: Option<&query::SessionDetail>,
    metrics: &[MetricDelta],
    side: Side,
) -> String {
    let Some(summary) = summary else {
        return String::new();
    };
    if summary.analyzed_count == 0 || metrics.is_empty() {
        return String::new();
    }
    let n = |delta: &MetricDelta| match side {
        Side::A => delta.n_a,
        Side::B => delta.n_b,
    };
    if metrics.iter().any(|delta| n(delta) > 0) {
        return String::new();
    }
    let drills = corpus
        .excluded
        .iter()
        .filter(|e| e.session_id == session_id && e.reason == ExclusionReason::Untracked)
        .count();
    if drills > 0 {
        let (swings, were) = if drills == 1 {
            ("swing", "was hit there as a drill")
        } else {
            ("swings", "were hit there as drills")
        };
        return format!(
            " Session {session_id} has analyzed swings but contributes no samples to any metric: \
             {drills} {swings} of this golfer's {were}, and a drill counts toward no stat or \
             baseline — clear its intent with golf-core set-intent if it should count."
        );
    }
    format!(
        " Session {session_id} has analyzed swings but contributes no samples to any metric: its \
         clips are byte-identical re-uploads of a clip first seen in an earlier session, so they \
         are the same physical swing and were already counted there."
    )
}

/// `_delta`: one metric across two sessions, the delta present only when both centers survived.
/// Each side contributes its CENTER refusals, left then right.
fn delta(
    metric: &str,
    left: Option<&MetricBaseline>,
    right: Option<&MetricBaseline>,
) -> MetricDelta {
    let unit = left.or(right).map(|m| m.unit.clone()).unwrap_or_default();
    let mean_a = left.and_then(|m| m.mean);
    let mean_b = right.and_then(|m| m.mean);
    let withheld = [left, right]
        .into_iter()
        .flatten()
        .flat_map(|side| side.withheld.iter())
        .filter(|r| r.claim == BaselineClaim::Center)
        .map(refusal)
        .collect();
    MetricDelta {
        metric: metric.to_string(),
        unit,
        n_a: left.map_or(0, |m| m.n),
        n_b: right.map_or(0, |m| m.n),
        mean_a,
        mean_b,
        delta: mean_a.zip(mean_b).map(|(a, b)| b - a),
        withheld,
    }
}

pub(crate) fn points(baseline: &MetricBaseline) -> Vec<SessionPoint> {
    baseline
        .sessions
        .iter()
        .map(|session| SessionPoint {
            session_id: session.session_id.clone(),
            captured_at: session.captured_at.isoformat(),
            n: session.n,
            mean: session.mean,
        })
        .collect()
}

pub(crate) fn refusal(withheld: &WithheldClaim) -> Refusal {
    Refusal {
        claim: withheld.claim.as_str().to_string(),
        have_n: withheld.have_n,
        need_n: withheld.need_n,
        have_sessions: withheld.have_sessions,
        need_sessions: withheld.need_sessions,
        reason: withheld.reason.clone(),
    }
}

pub(crate) fn low(interval: Option<&Interval>) -> Option<f64> {
    interval.map(|i| i.low)
}

pub(crate) fn high(interval: Option<&Interval>) -> Option<f64> {
    interval.map(|i| i.high)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The window starts `days` before the clock, refuses a call with no clock, and refuses a start
    /// CPython's `datetime` cannot hold rather than wrapping or panicking.
    #[test]
    fn the_window_is_read_off_the_clock_and_refused_outside_datetimes_range() {
        let at = Clock::At("2026-10-01T12:00:00Z".to_string());
        assert_eq!(
            window_start(&at, 30).unwrap(),
            Timestamp::parse("2026-09-01T12:00:00Z").unwrap()
        );
        assert_eq!(
            window_start(&at, -1).unwrap(),
            Timestamp::parse("2026-10-02T12:00:00Z").unwrap()
        );
        assert_eq!(window_start(&Clock::Unset, 30), Err(WindowRefused::NoClock));
        assert!(matches!(
            window_start(&Clock::At("yesterday".to_string()), 30),
            Err(WindowRefused::UnreadableClock(_))
        ));
        // 739,889 days back from 2026-10-01 is 0001-01-01, the oldest instant `datetime` holds,
        // and one day more is past it. `i64::MAX` days overflows the multiplication.
        let oldest = window_start(&Clock::At("2026-10-01T00:00:00Z".to_string()), 739_889);
        assert_eq!(oldest.unwrap().utc_micros(), DATETIME_MIN_MICROS);
        for days in [739_890, -3_000_000, i64::MAX, i64::MIN] {
            assert_eq!(
                window_start(&Clock::At("2026-10-01T00:00:00Z".to_string()), days),
                Err(WindowRefused::OutOfRange { days })
            );
        }
    }

    /// The mishit sentence goes on the two distance metrics alone, and in the singular for one.
    #[test]
    fn the_mishit_sentence_reaches_only_the_excluded_metrics() {
        let metric = |name: &str| MetricProfile {
            name: name.to_string(),
            unit: String::new(),
            source: String::new(),
            n: 0,
            n_sessions: 0,
            center: None,
            center_ci_low: None,
            center_ci_high: None,
            median: None,
            sd: None,
            sd_ci_low: None,
            sd_ci_high: None,
            minimum: None,
            maximum: None,
            bias: "withheld".to_string(),
            scatter: "withheld".to_string(),
            pattern: None,
            points_at: None,
            target: None,
            tolerance: None,
            within_session_sd: None,
            tour_standing: "withheld".to_string(),
            tour_reading: None,
            tour_percentile: None,
            tour_percentile_clamped: false,
            tour_band_low: None,
            tour_band_high: None,
            tour_population_n: None,
            tour_population_players: None,
            outside_by: None,
            sessions: Vec::new(),
            caveats: vec!["kept".to_string()],
            withheld: Vec::new(),
            unavailable: Vec::new(),
        };
        let mut metrics = vec![
            metric("ball_speed_mph"),
            metric("carry_distance_yds"),
            metric("total_distance_yds"),
        ];
        note_mishit_exclusion(&mut metrics, 0);
        assert!(metrics.iter().all(|m| m.caveats == ["kept"]));
        note_mishit_exclusion(&mut metrics, 1);
        assert_eq!(metrics[0].caveats, ["kept"]);
        assert!(metrics[1].caveats[1].starts_with("1 shot was set aside as a mishit — "));
        assert_eq!(metrics[1].caveats, metrics[2].caveats);
    }
}
