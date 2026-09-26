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
/// The history of the sixteen versions lives in `contracts/swing.py`, where each one records what
/// moved and whether a stored artifact from the version before is *missing* a quantity or
/// *disagrees* about one. It is deliberately not copied here: it is three hundred lines of
/// measurement that a second copy could only get wrong, and `CLAUDE.md`'s rule about counts in prose
/// applies to a port as much as to a doc. What is copied is the number, because the version gates
/// every vector in `spec/` and a port that claims a different one is certifying against answers this
/// engine has retracted — `tests/round_trip.rs` pins it against the committed vectors.
pub const ANALYSIS_VERSION: i64 = 16;

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
}
