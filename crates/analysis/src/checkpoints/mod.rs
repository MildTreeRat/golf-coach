//! Swing checkpoints — mechanics (pose) now, outcome (shot-vs-intent) later. [M22 P5]
//!
//! The port of `analysis/checkpoints/`. Ships the pose-only **mechanics** checkpoints
//! ([`mechanics`]) and binds each to its registered name in [`CHECKPOINT_EVALUATORS`].
//! `outcome.py` is absent in Python too — this is the named seam where it will land.
//!
//! Which checkpoints exist is [`contracts::checkpoints::CHECKPOINT_REGISTRY`], not a list written
//! out here. That module is deliberately upstream: `contracts/caveats.py` builds the caveat prose
//! a model reads out of the same registry, and ADR-008 forbids it importing `analysis`.
//!
//! `head_stays_back` is the only one that needs to know *who* swung: its sign is camera-relative,
//! so it takes a [`Handedness`] and refuses with `NO_HANDEDNESS` without one.

pub mod mechanics;

use contracts::golfer::Handedness;
use contracts::intent::{ClubCategory, PlayerProfile};
use contracts::keypoints::FrameKeypoints;
use contracts::swing::{CheckpointScore, PhaseSegment};
use contracts::unscored::UnscoredReason;
use contracts::Validate;

/// What one evaluator returns: a score, or the reason there is no score.
///
/// **An enum rather than the Python `NamedTuple`'s struct of two optionals**, which is the note
/// `crates/contracts/src/swing.rs` left for this phase. The Python docstring argues that *"a score
/// and a refusal are different things and a shape that can be half of each invites code that reads
/// a band edge off a checkpoint that has none"* — in Python that argument can only be made in
/// prose plus an `assert`, and here the type system enforces it outright. The `assert
/// judged.reason is not None` that `engine.py`'s checkpoint loop carries has no counterpart to
/// write: the compiler will not let the case exist.
#[derive(Debug, Clone, PartialEq)]
pub enum CheckpointOutcome {
    /// The verdict.
    Scored(CheckpointScore),
    /// Which condition stopped it, and which window, landmark group or lookup failed.
    Unscored {
        reason: UnscoredReason,
        detail: String,
    },
}

impl CheckpointOutcome {
    /// Wrap a verdict, checking its bounds on the way through.
    ///
    /// **The validate call is here because Python's is at `CheckpointScore(...)`.** Pydantic runs
    /// `Field(ge=…, le=…)` on construction and Rust has no constructor hook, so ADR-032 §4's rule
    /// — the constraints are behaviour, not documentation — needs an explicit call at the moment a
    /// score is assembled. This is the one gate every evaluator passes through, so putting it here
    /// rather than at six call sites is what makes "every score is validated" true by
    /// construction rather than by six people remembering.
    pub fn scored(score: CheckpointScore) -> Self {
        score
            .validate()
            .unwrap_or_else(|e| panic!("an evaluator built an invalid CheckpointScore: {e}"));
        Self::Scored(score)
    }

    pub fn unscored_for(reason: UnscoredReason, detail: impl Into<String>) -> Self {
        Self::Unscored {
            reason,
            detail: detail.into(),
        }
    }

    /// The verdict, or `None` — the shape `engine.py`'s `judged.score is not None` reads.
    pub fn score(&self) -> Option<&CheckpointScore> {
        match self {
            Self::Scored(score) => Some(score),
            Self::Unscored { .. } => None,
        }
    }
}

/// One shape every checkpoint is called through, whatever it actually reads.
pub type Evaluate = fn(
    &[FrameKeypoints],
    &[PhaseSegment],
    Option<Handedness>,
    ClubCategory,
    Option<&PlayerProfile>,
) -> CheckpointOutcome;

/// Registered name -> the call that scores it, so the engine dispatches instead of listing.
///
/// The closures exist to absorb the two evaluators that legitimately don't take the full argument
/// set — `evaluate_tempo` reads phase instants and never touches keypoints, and
/// `evaluate_head_stays_back` is the only one that needs `handedness`. Widening their real
/// signatures to match would put parameters in them that they must then be trusted to ignore;
/// adapting at the registry keeps each function honest about what it reads.
/// `measure::POSE_MEASUREMENTS` absorbs `tempo_ratio` the same way, for the same reason.
///
/// A slice of pairs rather than a map, which is the shape `POSE_MEASUREMENTS` already uses here:
/// six entries, looked up by a name that came out of the registry, and a `HashMap` would buy
/// nothing but a nondeterministic iteration order in a crate where order is repeatedly an answer.
pub static CHECKPOINT_EVALUATORS: &[(&str, Evaluate)] = &[
    (
        mechanics::TEMPO_CHECKPOINT,
        |_keypoints, phases, _handedness, club, profile| {
            mechanics::evaluate_tempo(phases, club, profile)
        },
    ),
    (
        mechanics::HEAD_SWAY_CHECKPOINT,
        |keypoints, phases, _handedness, club, profile| {
            mechanics::evaluate_head_sway(keypoints, phases, club, profile)
        },
    ),
    (
        mechanics::FINISH_BALANCE_CHECKPOINT,
        |keypoints, phases, _handedness, club, profile| {
            mechanics::evaluate_finish_balance(keypoints, phases, club, profile)
        },
    ),
    (
        mechanics::HIP_SWAY_CHECKPOINT,
        |keypoints, phases, _handedness, club, profile| {
            mechanics::evaluate_hip_sway(keypoints, phases, club, profile)
        },
    ),
    (
        mechanics::HIP_SHIFT_AT_TOP_CHECKPOINT,
        |keypoints, phases, _handedness, club, profile| {
            mechanics::evaluate_hip_shift_at_top(keypoints, phases, club, profile)
        },
    ),
    (
        mechanics::HEAD_STAYS_BACK_CHECKPOINT,
        |keypoints, phases, handedness, club, profile| {
            mechanics::evaluate_head_stays_back(keypoints, phases, handedness, club, profile)
        },
    ),
];

/// The evaluator registered under `name`.
///
/// Panics on a miss, the way `contracts::checkpoints::spec_for` does and for the same reason:
/// Python raises `KeyError` here deliberately, so that a spec added without an evaluator fails
/// naming the checkpoint rather than as a silently mismatched zip.
pub fn evaluator_for(name: &str) -> Evaluate {
    CHECKPOINT_EVALUATORS
        .iter()
        .find(|(registered, _)| *registered == name)
        .map(|(_, evaluate)| *evaluate)
        .unwrap_or_else(|| panic!("no evaluator registered under {name:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use contracts::checkpoints::CHECKPOINT_REGISTRY;

    /// The pin Python keeps as a test rather than as a zip: a spec with no evaluator, or an
    /// evaluator with no spec, is a wiring bug and this is where it surfaces.
    #[test]
    fn every_registered_checkpoint_has_an_evaluator_and_vice_versa() {
        let specs: Vec<&str> = CHECKPOINT_REGISTRY.iter().map(|s| s.name).collect();
        let evaluators: Vec<&str> = CHECKPOINT_EVALUATORS.iter().map(|(n, _)| *n).collect();
        assert_eq!(specs, evaluators, "the registry and the dispatch disagree");
    }

    #[test]
    #[should_panic(expected = "no evaluator registered under \"spine_angle\"")]
    fn an_unregistered_name_is_a_wiring_bug() {
        evaluator_for("spine_angle");
    }

    #[test]
    fn a_refusal_carries_no_score() {
        let outcome = CheckpointOutcome::unscored_for(UnscoredReason::NoBand, "nothing to judge");
        assert!(outcome.score().is_none());
    }
}
