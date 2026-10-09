//! The analyzed result for one swing. `contracts/swing.py`. [M22 P2]
//!
//! [`SwingResult`] is the merged, analyzed view of one swing: the aligned data streams plus the
//! phase segmentation, per-checkpoint scores and overall score. [`SwingBundleResult`] wraps it with
//! the second view and the windows, and is what `conformance.py` serializes.
//!
//! `CheckpointOutcome` is not ported here. It is a Python `NamedTuple` and not a payload shape —
//! nothing serializes it — so P2's round-trip gate cannot see it, and it lands with the evaluators
//! that return it in P5. The note it carries is worth bringing across when it does: in Rust it wants
//! to be an enum rather than a struct of two optionals, because *"a score and a refusal are
//! different things and a shape that can be half of each invites code that reads a band edge off a
//! checkpoint that has none"* is a thing the type system can enforce outright here.

use serde::{Deserialize, Deserializer, Serialize};

use crate::alignment::SwingAlignment;
use crate::detections::FrameDetections;
use crate::feedback::FeedbackPayload;
use crate::intent::PracticeGoal;
use crate::keypoints::FrameKeypoints;
use crate::shot::ShotData;
use crate::unscored::{UnscoredCheckpoint, UnscoredReason};
use crate::{each, ge, le, nested, ContractError, Validate};

/// The generation of the analysis engine. Stamped onto every [`SwingBundleResult`] the pipeline
/// writes, so a stored artifact can say which code produced it.
///
/// **Bump it whenever the engine's output changes *meaning*** — a new measurement, a corrected phase
/// instant, a re-cut benchmark band. Not for refactors, and not for anything that leaves every
/// number where it was.
///
/// **The history is in two halves, and each lives where it was made.** Versions 1–16 are in
/// `contracts/swing.py`'s ledger, where each records what moved and whether a stored artifact from
/// the version before is *missing* a quantity or *disagrees* about one. They are not copied here:
/// three hundred lines of measurement that a second copy could only get wrong. From 17 the bump is
/// Rust's (ADR-035 clause 3) and frozen Python's constant stays at 16 on purpose, so those entries
/// have no Python twin and are written below, in the same form. The test
/// `every_version_from_seventeen_has_a_ledger_entry` holds the two together, as
/// `tests/test_docs_truth.py` does for the Python half. The number itself gates every vector in
/// `spec/`, and `tests/round_trip.rs` pins it against the committed ones.
///
/// **Each entry from 17 names its class in brackets after the date** (M36 P14), because
/// [`COMPARABLE_FROM`] is read off them: `[disagrees]` when an artifact from the version before
/// reports a number this engine would not, `[missing]` when it only lacks a new measurement, and
/// `[shape]` when no number moved at all. Python's half says the same thing in prose — *disagrees*
/// and *missing* are its words, bump by bump — and the classes are those words made checkable.
///
/// 16 -> 17 (2026-10-01, M32 P8) [shape]: **no number moved.** `ShotData` gained seven keys
///   (`attack_angle`, `dynamic_loft`, `low_point`, `impact_offset_h`, `impact_offset_v`,
///   `impact_position_v`, `carry_offline`) and `ShotProvenance` three (`parser_version`,
///   `fields_present`, `corrections`), every one defaulting. A version-16 artifact is *missing*
///   them and disagrees about nothing: its shot reads with each at its default, which is what this
///   engine writes for every shot stored so far, since no source fills any of them yet. Every
///   committed vector was re-recorded by `golf-core rerecord` against `spec/declarations/v17.json`,
///   which moved the version and added the ten keys and nothing else, so every other value in
///   `spec/` is still the one Python recorded. **`contracts/swing.py` stays at 16 on purpose**: the
///   frozen lab never writes these keys, and `api/state.py::is_outdated` compares with `<`, so it
///   reads a v17 artifact as current rather than asking for a re-run it cannot do. Nobody should
///   "fix" the gap by bumping Python.
pub const ANALYSIS_VERSION: i64 = 17;

/// The oldest engine generation whose stored numbers today's engine still agrees with. The corpus
/// reader pools a swing stored by this generation or a later one, and excludes one stored by an
/// older one as `OUTDATED`, named and counted but contributing no sample (`storage::corpus`).
/// [M36 P14, the M36 plan's decisions 1 and 2]
///
/// **Two questions, so two numbers.** [`ANALYSIS_VERSION`] answers *is there a newer engine to
/// run*, which a re-analysis picks its targets by (`storage::state::is_outdated`). This answers
/// *does a stored number still mean what today's would*, which is what pooling needs. Frozen
/// Python's corpus asks the first question, and asked of Rust's 17 it excluded all 13 swings on
/// disk though 16 -> 17 moved no number (M36 P10's scratch run). A bump that only *adds* a
/// measurement is no reason to drop a swing either: the older artifact lacks the new number, which
/// shows as a smaller `n` for that metric alone, and that is the honest count.
///
/// **Why 14.** Python's ledger (`contracts/swing.py`) says of every bump whether an older artifact
/// *disagrees* about a number or is only *missing* one. 13 -> 14, the heavy pose model, is the
/// newest that disagrees: every landmark moved under every score. 14 -> 15 (the `flight_*` family)
/// and 15 -> 16 (the `pivot_*` family) are missing bumps, each measured byte-identical on every
/// stored `overall_score` and `checkpoint_scores` entry, and 16 -> 17 is shape only. No version-14
/// or version-15 artifact exists in `data/`, so today 14 pools exactly what 16 would.
///
/// **It moves on a `[disagrees]` entry and on nothing else.** The test
/// `comparable_from_is_the_newest_disagreeing_generation` holds it to the newest `[disagrees]`
/// entry in the ledger above, or to 14, Python's half's answer, while there is none. It needs no
/// re-record when it moves: `read_corpus` takes the generations as a parameter, and every vector
/// records the pair it was read under (the M36 plan's call 7). Frozen Python has no such constant
/// and keeps its own rule until M40 deletes it (ADR-035 clause 4); over `data/` the two pool the
/// same swings, because everything stored there is at Python's installed 16.
pub const COMPARABLE_FROM: i64 = 14;

// The oldest generation this engine agrees with cannot be one newer than this engine: a build
// where it is fails to compile rather than excluding every swing on disk.
const _: () = assert!(COMPARABLE_FROM <= ANALYSIS_VERSION);

/// The six segments of a golf swing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SwingPhase {
    Address,
    Backswing,
    Transition,
    Downswing,
    Impact,
    FollowThrough,
}

impl SwingPhase {
    /// Python's `SwingPhase.value` — the same string the wire form carries.
    ///
    /// It exists because the *prose* needs it: `measure.py` interpolates `to_phase.value` into two
    /// refusal sentences a golfer is shown, and `docs/CONFORMANCE.md` §3 compares those exactly. Going
    /// through `serde_json::to_string` to reach a word for a sentence would allocate, quote it, and
    /// make a sentence depend on a serializer; spelling the table twice would let the sentence and the
    /// wire name drift. So it is spelled once here, and the wire-name test below reads *this* rather
    /// than its own copy — which is what makes them one table rather than two that agree today.
    pub fn as_str(self) -> &'static str {
        match self {
            SwingPhase::Address => "address",
            SwingPhase::Backswing => "backswing",
            SwingPhase::Transition => "transition",
            SwingPhase::Downswing => "downswing",
            SwingPhase::Impact => "impact",
            SwingPhase::FollowThrough => "follow_through",
        }
    }
}

/// A contiguous span of frames belonging to one swing phase.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Self")]
pub struct PhaseSegment {
    pub phase: SwingPhase,
    pub start_frame: i64,
    pub end_frame: i64,
    pub start_ms: f64,
    pub end_ms: f64,
    /// False when this boundary is an estimate rather than something found in the signal. A detector
    /// that fails should say so rather than return a plausible-looking number (ADR-013): the
    /// estimate is good enough to place a measurement window, but a consumer that *divides* by the
    /// boundary — tempo does — must drop its score instead of reporting one. Defaults true so a
    /// segment nobody flagged reads as detected.
    #[serde(default = "crate::yes")]
    pub detected: bool,
}

crate::validated!(PhaseSegment);

impl Validate for PhaseSegment {
    fn validate(&self) -> Result<(), ContractError> {
        ge("PhaseSegment.start_frame", self.start_frame, 0)?;
        ge("PhaseSegment.end_frame", self.end_frame, 0)?;
        ge("PhaseSegment.start_ms", self.start_ms, 0.0)?;
        ge("PhaseSegment.end_ms", self.end_ms, 0.0)
    }
}

/// Result of evaluating one swing checkpoint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Self")]
pub struct CheckpointScore {
    pub name: String,
    /// 0 = fail .. 1 = ideal.
    pub score: f64,
    pub passed: bool,
    /// Measured value.
    #[serde(default)]
    pub observed: Option<f64>,
    #[serde(default)]
    pub expected_low: Option<f64>,
    #[serde(default)]
    pub expected_high: Option<f64>,
    #[serde(default)]
    pub message: String,

    /// Where `observed` sits in the reference population, as opposed to `score`, which only says how
    /// far outside the *band* it fell. **Informational and never affects `score` or `passed`**
    /// (ADR-010 §2). Clamped to `[10, 90]` because the tails were never stored — read 90 as "at or
    /// beyond the 90th", not as a precise rank.
    ///
    /// It exists because `score` is not comparable across checkpoints: the decay is in band-widths
    /// and the bands are different widths, so a 0.6 means three different real-world things.
    #[serde(default)]
    pub percentile: Option<f64>,
    /// Sample size behind `percentile`, so a reader can weigh how much it is worth.
    #[serde(default)]
    pub population_n: Option<i64>,
    /// True when lower is strictly better and the band is `[0, high]`; false when both tails are
    /// faults. Consumers need this to turn a percentile into a distance from *ideal* — at the 10th
    /// percentile a one-sided metric is excellent and a two-sided one is as wrong as at the 90th.
    #[serde(default)]
    pub one_sided: bool,
}

crate::validated!(CheckpointScore);

impl Validate for CheckpointScore {
    fn validate(&self) -> Result<(), ContractError> {
        ge("CheckpointScore.score", self.score, 0.0)?;
        le("CheckpointScore.score", self.score, 1.0)?;
        ge("CheckpointScore.percentile", self.percentile, 0.0)?;
        le("CheckpointScore.percentile", self.percentile, 100.0)?;
        ge("CheckpointScore.population_n", self.population_n, 0)
    }
}

/// A quantity measured off one swing, carrying no claim about whether it is good.
///
/// Deliberately **not** a [`CheckpointScore`]: no band, no `passed`, no `score`. The distinction is
/// the point. A metric earns a verdict only once a population distribution exists to judge it
/// against (ADR-010 §2 — no score beats a wrong one), and until then it is data. Making that a
/// *type* rather than a convention means a measurement is structurally incapable of being rendered
/// as a fault, however it travels.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Self")]
pub struct Measurement {
    pub name: String,
    pub value: f64,
    /// `shoulder_widths` | `degrees` | `ratio` | `mph` | `yards` | `rpm`.
    pub unit: String,
    /// Where it came from: `pose:face_on`, `launch_monitor:hd_golf`, `model:flight_v1`.
    pub source: String,
    /// How and where it was taken, and any sign convention. A bare number cannot be re-derived or
    /// re-normalized later; this is what makes it auditable when a band is eventually cut from it.
    #[serde(default)]
    pub detail: String,
}

crate::validated!(Measurement);

impl Validate for Measurement {
    fn validate(&self) -> Result<(), ContractError> {
        Ok(())
    }
}

/// Read the names-only `unscored` form every artifact written before 2026-08-19 uses.
///
/// ADR-032 §4's third named validator, and the one that is a **coercion rather than a check**: an
/// old `analysis.json` says `["tempo"]` and means "tempo went missing and this file does not know
/// why", which is exactly [`UnscoredReason::Unrecorded`]. A port that rejects the bare form cannot
/// read the older half of `data/processed/`.
///
/// Dropping the entries instead would have been the silent failure — a stored swing judged on five
/// fundamentals would start reading as one judged on six, and `overall_score` would gain a
/// checkpoint it never had. The engine never *writes* `Unrecorded`; seeing one means you are looking
/// at an artifact that predates the reason.
fn unscored_accepting_bare_names<'de, D>(
    deserializer: D,
) -> Result<Vec<UnscoredCheckpoint>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Entry {
        // String first: `untagged` tries the variants in order, and the full form would otherwise
        // never be reached by a bare name anyway, but the reverse order makes the error message on a
        // malformed object name the wrong variant.
        Name(String),
        Full(UnscoredCheckpoint),
    }

    Ok(Vec::<Entry>::deserialize(deserializer)?
        .into_iter()
        .map(|entry| match entry {
            Entry::Name(name) => UnscoredCheckpoint {
                name,
                reason: UnscoredReason::Unrecorded,
                detail: String::new(),
            },
            Entry::Full(entry) => entry,
        })
        .collect())
}

/// The complete analyzed result for one swing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Self")]
pub struct SwingResult {
    pub swing_id: String,
    pub session_id: String,

    #[serde(default)]
    pub phases: Vec<PhaseSegment>,
    #[serde(default)]
    pub checkpoint_scores: Vec<CheckpointScore>,

    /// Quantities measured off this swing that are **not** judged — no band, no score, no pass/fail.
    /// A consumer must never render these as verdicts or infer one from them. Independent of
    /// `checkpoint_scores`: a metric can be measured here and scored there, or measured here and
    /// scored nowhere.
    #[serde(default)]
    pub measurements: Vec<Measurement>,

    /// Checkpoints that were attempted but could not be scored, each with the reason. Dropping the
    /// score is correct (ADR-010 §2), but dropping it *silently* is not: `overall_score` is a mean
    /// over whatever survived, so without this list a two-checkpoint swing and a three-checkpoint
    /// swing are indistinguishable.
    #[serde(default, deserialize_with = "unscored_accepting_bare_names")]
    pub unscored: Vec<UnscoredCheckpoint>,

    // Dual-axis scoring (ADR-009). The practice intent this swing was judged against, plus the two
    // independent sub-scores. `overall_score` is the policy-weighted blend (for the Fundamentals
    // PoC: overall == mechanics_score, outcome_score is None).
    #[serde(default)]
    pub intent: Option<PracticeGoal>,
    #[serde(default)]
    pub mechanics_score: Option<f64>,
    #[serde(default)]
    pub outcome_score: Option<f64>,
    /// 0-100 policy-weighted blend.
    pub overall_score: f64,

    // The merged source data this result was computed from. Optional so a lightweight result (from
    // storage, scores only) can omit the heavy streams — which is what `conformance.py`'s
    // `EXCLUDED_FROM_RESULT` drops before a vector is written.
    #[serde(default)]
    pub keypoints: Vec<FrameKeypoints>,
    #[serde(default)]
    pub detections: Vec<FrameDetections>,
    #[serde(default)]
    pub shot: Option<ShotData>,
}

crate::validated!(SwingResult);

impl Validate for SwingResult {
    fn validate(&self) -> Result<(), ContractError> {
        ge("SwingResult.mechanics_score", self.mechanics_score, 0.0)?;
        le("SwingResult.mechanics_score", self.mechanics_score, 100.0)?;
        ge("SwingResult.outcome_score", self.outcome_score, 0.0)?;
        le("SwingResult.outcome_score", self.outcome_score, 100.0)?;
        ge("SwingResult.overall_score", self.overall_score, 0.0)?;
        le("SwingResult.overall_score", self.overall_score, 100.0)?;
        each("SwingResult.phases", &self.phases)?;
        each("SwingResult.checkpoint_scores", &self.checkpoint_scores)?;
        each("SwingResult.measurements", &self.measurements)?;
        each("SwingResult.unscored", &self.unscored)?;
        each("SwingResult.keypoints", &self.keypoints)?;
        each("SwingResult.detections", &self.detections)?;
        nested("SwingResult.intent", self.intent.as_ref())?;
        nested("SwingResult.shot", self.shot.as_ref())
    }
}

/// One swing analyzed from the two camera views plus its shot photo. [M7 Phase 4]
///
/// The two views are not equals and this shape says so. **Only the face-on view is scored** — it is
/// the canonical pose angle the checkpoints were validated against, and the GolfDB corpus the bands
/// came from is face-on, so a down-the-line metric would have no reference data to be judged
/// against. The down-the-line clip contributes alignment anchors and nothing else (ADR-015).
///
/// Every frame index in here — `swing.phases`, `alignment`'s anchors — addresses the **whole** clip,
/// even when a window was used to find the swing inside a longer recording. The window is a search
/// restriction, not a coordinate system.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Self")]
pub struct SwingBundleResult {
    pub swing_id: String,
    pub session_id: String,

    /// The face-on view's scored result, with the shot attached if one was found.
    pub swing: SwingResult,

    /// Which generation of the engine produced this ([`ANALYSIS_VERSION`]). **Defaults to 0, not to
    /// the current version, and that is load-bearing**: this field is read back by parsing artifacts
    /// written before it existed, and a default of "current" would make every legacy file claim to
    /// be up to date — the one wrong answer that cannot be recovered from, since it is
    /// indistinguishable from a correct one.
    #[serde(default)]
    pub analysis_version: i64,

    /// How the two views correspond, or `None` when there was no usable down-the-line clip. Read
    /// `alignment.quality` before presenting the side-by-side video as synchronized.
    #[serde(default)]
    pub alignment: Option<SwingAlignment>,

    /// The `[start, end)` frame range the swing was found in, when the clip held more than one.
    /// Recorded because it is not cosmetic: it decides which frames were scored.
    #[serde(default)]
    pub face_on_window: Option<(i64, i64)>,
    #[serde(default)]
    pub down_the_line_window: Option<(i64, i64)>,

    /// Everything that degraded, in order of discovery. A consumer that renders the score and
    /// ignores this is exactly the silent failure ADR-013 and ADR-014 were written against.
    #[serde(default)]
    pub notes: Vec<String>,

    /// Ranked coaching tips. Left `None` by `analysis`, which must not import `feedback` (ADR-008)
    /// — the caller fills it in so the serialized artifact is the complete result rather than
    /// something a reader has to recompute.
    #[serde(default)]
    pub feedback: Option<FeedbackPayload>,
}

crate::validated!(SwingBundleResult);

impl Validate for SwingBundleResult {
    fn validate(&self) -> Result<(), ContractError> {
        nested("SwingBundleResult.swing", Some(&self.swing))?;
        nested("SwingBundleResult.alignment", self.alignment.as_ref())?;
        nested("SwingBundleResult.feedback", self.feedback.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_phase_wire_names_are_the_python_values() {
        let every = [
            SwingPhase::Address,
            SwingPhase::Backswing,
            SwingPhase::Transition,
            SwingPhase::Downswing,
            SwingPhase::Impact,
            SwingPhase::FollowThrough,
        ];
        assert_eq!(
            every.map(SwingPhase::as_str),
            [
                "address",
                "backswing",
                "transition",
                "downswing",
                "impact",
                "follow_through"
            ]
        );
        // And `as_str` really is the wire name, which is what lets a sentence built from it and a
        // serialized payload be checked against one table instead of two.
        for variant in every {
            assert_eq!(
                serde_json::to_string(&variant).unwrap(),
                format!("\"{}\"", variant.as_str())
            );
        }
    }

    #[test]
    fn a_bare_unscored_name_expands_to_an_unrecorded_entry() {
        let json = r#"{"swing_id":"1","session_id":"s","overall_score":0.0,
                       "unscored":["tempo",{"name":"head_sway","reason":"no_band","detail":"x"}]}"#;
        let result: SwingResult = serde_json::from_str(json).unwrap();
        assert_eq!(result.unscored[0].name, "tempo");
        assert_eq!(result.unscored[0].reason, UnscoredReason::Unrecorded);
        assert_eq!(result.unscored[0].detail, "");
        assert_eq!(result.unscored[1].reason, UnscoredReason::NoBand);
        assert_eq!(result.unscored[1].detail, "x");
    }

    /// The coercion writes the expanded form back, which is what makes it a *migration* rather than
    /// a reader quirk: the bare list is accepted once and never re-emitted.
    #[test]
    fn a_coerced_entry_serializes_in_the_full_form() {
        let json = r#"{"swing_id":"1","session_id":"s","overall_score":0.0,"unscored":["tempo"]}"#;
        let result: SwingResult = serde_json::from_str(json).unwrap();
        let back = serde_json::to_value(&result).unwrap();
        assert_eq!(
            back["unscored"],
            serde_json::json!([{"name": "tempo", "reason": "unrecorded", "detail": ""}])
        );
    }

    #[test]
    fn the_bundle_version_defaults_to_zero_and_not_to_current() {
        let json = r#"{"swing_id":"1","session_id":"s",
                       "swing":{"swing_id":"1","session_id":"s","overall_score":0.0}}"#;
        let bundle: SwingBundleResult = serde_json::from_str(json).unwrap();
        assert_eq!(bundle.analysis_version, 0);
        assert_ne!(bundle.analysis_version, ANALYSIS_VERSION);
    }

    #[test]
    fn a_score_off_the_hundred_point_scale_is_refused_wherever_it_sits() {
        let bad_overall = r#"{"swing_id":"1","session_id":"s","overall_score":100.5}"#;
        let bad_mechanics =
            r#"{"swing_id":"1","session_id":"s","overall_score":0.0,"mechanics_score":-1.0}"#;
        assert!(serde_json::from_str::<SwingResult>(bad_overall).is_err());
        assert!(serde_json::from_str::<SwingResult>(bad_mechanics).is_err());
    }

    /// A refusal inside a nested model has to surface as a refusal of the whole tree, or the bounds
    /// are documentation after all.
    #[test]
    fn a_bad_checkpoint_fails_the_bundle_that_holds_it() {
        let json = r#"{"swing_id":"1","session_id":"s","swing":{
            "swing_id":"1","session_id":"s","overall_score":0.0,
            "checkpoint_scores":[{"name":"tempo","score":1.5,"passed":true}]}}"#;
        let err = serde_json::from_str::<SwingBundleResult>(json).unwrap_err();
        assert!(err.to_string().contains("CheckpointScore.score"), "{err}");
    }

    /// `ANALYSIS_VERSION` and the ledger in its doc move together, or neither moves — the Rust half
    /// of `tests/test_docs_truth.py::test_the_version_ledger_documents_the_installed_version`, over
    /// the versions only Rust has (17 on). A bump with no entry is prose that has quietly stopped
    /// describing the code, and every other test reads the constant, not the paragraph. [M32 P8]
    #[test]
    fn every_version_from_seventeen_has_a_ledger_entry() {
        let documented: Vec<i64> = ledger().into_iter().map(|(to, _)| to).collect();

        assert!(
            !documented.is_empty(),
            "no version ledger parsed out of swing.rs — has its format changed?"
        );
        let missing: Vec<i64> = (17..=ANALYSIS_VERSION)
            .filter(|version| !documented.contains(version))
            .collect();
        assert!(
            missing.is_empty(),
            "ANALYSIS_VERSION is {ANALYSIS_VERSION}; the ledger above it documents no entry for \
             {missing:?} — say what a stored artifact from the older engine is missing or disagrees \
             about, in the same edit as the bump"
        );
    }

    /// The three classes a ledger entry from 17 may name (the ledger's doc).
    const CLASSES: [&str; 3] = ["shape", "missing", "disagrees"];

    /// What [`COMPARABLE_FROM`] is while Rust's half of the ledger holds no `[disagrees]` entry:
    /// 13 -> 14 in `contracts/swing.py`, the newest bump Python's half calls a disagreement.
    const PYTHON_NEWEST_DISAGREES: i64 = 14;

    /// Rust's half of the version ledger, as `(to, class)` per entry, from the doc lines spelled
    /// `/// 16 -> 17 (when) [class]: …` — Python's `#: 16 -> 17 (` in Rust's comment, with the class
    /// after the date. An entry with no class reads as `None`, for the test that refuses one.
    fn ledger() -> Vec<(i64, Option<&'static str>)> {
        include_str!("swing.rs")
            .lines()
            .filter_map(|line| line.trim_start().strip_prefix("/// "))
            .filter_map(|entry| {
                let (from, rest) = entry.split_once(" -> ")?;
                let (to, rest) = rest.split_once(' ')?;
                rest.starts_with('(').then_some(())?;
                from.parse::<i64>().ok()?;
                let class = rest
                    .split_once(") [")
                    .and_then(|(_, after)| after.split_once("]:"))
                    .map(|(class, _)| class);
                Some((to.parse().ok()?, class))
            })
            .collect()
    }

    /// **`COMPARABLE_FROM` is what the ledger says it is**, so it cannot be left behind by a bump
    /// that makes stored numbers disagree, nor moved by one that only adds a key: every entry from 17
    /// names one class, and the constant is the newest `[disagrees]` entry's version, else Python's
    /// 14. [M36 P14, the M36 plan's decision 2]
    #[test]
    fn comparable_from_is_the_newest_disagreeing_generation() {
        let entries = ledger();
        let unclassed: Vec<i64> = entries
            .iter()
            .filter(|(to, class)| *to >= 17 && !class.is_some_and(|c| CLASSES.contains(&c)))
            .map(|(to, _)| *to)
            .collect();
        assert!(
            unclassed.is_empty(),
            "the ledger entries to {unclassed:?} name no class — say which of {CLASSES:?} the bump \
             is, after the date, `(when) [class]:`"
        );

        let newest = entries
            .iter()
            .filter(|(_, class)| *class == Some("disagrees"))
            .map(|(to, _)| *to)
            .max()
            .unwrap_or(PYTHON_NEWEST_DISAGREES);
        assert_eq!(
            COMPARABLE_FROM, newest,
            "the ledger's newest `[disagrees]` entry, or Python's {PYTHON_NEWEST_DISAGREES} while \
             there is none, puts COMPARABLE_FROM at {newest}; it moves with that entry, in the same \
             edit"
        );
    }
}
