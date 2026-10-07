//! Scoring policies — the intent selects how the two axes combine. `analysis/scoring.py`. [M22 P6]
//!
//! Per [ADR-009]: a swing has two independent axes — **mechanics** (pose) and **outcome** (ball
//! flight) — and the golfer's practice *mode* picks the policy that weights them into the overall
//! score. The engine hands the two lists in separately, so a policy never has to know which axis a
//! [`CheckpointScore`] came from.
//!
//! Only `Fundamentals` is implemented, here as in Python: mechanics 100%, outcome ignored and left
//! `None`. The other modes are named seams and [`policy_for`] refuses them.
//!
//! [ADR-009]: ../../../docs/decisions/009-swing-scoring-model.md
//!
//! # Why this module is P6's and not P5's
//!
//! It is the judging half by subject and it ports with the assembly, because no stage vector
//! records `mechanics_score`. `spec/vectors/stages/` holds the engine's *function* boundaries and
//! `combine` is called from inside `analyze_swing`, after the checkpoint loop — so the only
//! committed answer for it is the engine vector's `swing.mechanics_score`, which is what the
//! end-to-end gate compares. P5's own note says so; this is that note discharged.
//!
//! # The Strategy is a function pointer
//!
//! Python spells this as a `Protocol` plus a class per mode. Here it is a `fn` and a lookup, which
//! is the shape [`crate::checkpoints::CHECKPOINT_EVALUATORS`] already uses in this crate for the
//! same job: one call signature, a table that maps a registered key to it, and a panic on a key
//! nothing registers. A trait with a single implementor and no dynamic-dispatch site would be a
//! seam that costs a reader an indirection and buys nothing until the second mode exists.

use contracts::intent::PracticeMode;
use contracts::swing::CheckpointScore;

/// The two sub-scores and their blended overall, all on a 0-100 scale.
///
/// `mechanics` and `outcome` are optional because "no checkpoints on this axis" and "this axis
/// scored zero" are different facts and only one of them is a verdict (ADR-010 §2). `overall` is
/// not, because a policy always answers.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AxisScores {
    pub mechanics: Option<f64>,
    pub outcome: Option<f64>,
    pub overall: f64,
}

/// One shape every policy is called through: the two axes' scores in, the blend out.
pub type Combine = fn(&[CheckpointScore], &[CheckpointScore]) -> AxisScores;

/// Mean of the 0..1 checkpoint scores, scaled to 0..100. `0.0` for an empty list.
///
/// **`0.0` and not `None`, which reads wrong and is the Python's answer.** It is not a claim that a
/// swing with nothing scored scored zero — `SwingResult.checkpoint_scores` being empty is what says
/// that, and `feedback::rules::headline` returns `None` rather than a clean bill when it is. Kept as
/// written because `SwingBundleResult.overall_score` is a required float on disk and every stored
/// artifact carries this convention.
///
/// The sum is left to right, and that is **not** what Python's `sum()` does on the interpreter that
/// recorded the vectors: since CPython 3.12 it is compensated (Neumaier; `pyfmt::sum` is the port).
/// This comment said otherwise until M34 P2 measured it. On four of the 21 engine vectors —
/// `2026-08-23-1`, `-3`, `-5` and `tempo-too-quick` — the two means part by one or two ulp, which
/// `docs/CONFORMANCE.md` §3's `RTOL` absorbs, so every vector still conforms. The code is left as
/// it is on purpose: whether the engine's `sum()` sites move to `pyfmt::sum` is a question M34 P2
/// routed rather than answered (`docs/plans/m34-screen-reader.md`, its findings).
fn mean_percent(scores: &[CheckpointScore]) -> f64 {
    if scores.is_empty() {
        return 0.0;
    }
    let total: f64 = scores.iter().map(|score| score.score).sum();
    total / scores.len() as f64 * 100.0
}

/// Grade mechanics only; outcome is informational (ADR-009 §PoC boundary).
///
/// `outcome` is taken and ignored rather than absent from the signature, because the argument is
/// what makes this a policy: the outcome axis exists and this mode chooses not to weigh it, which is
/// a different statement from the axis not being available.
fn fundamentals(mechanics: &[CheckpointScore], _outcome: &[CheckpointScore]) -> AxisScores {
    let mechanics_score = mean_percent(mechanics);
    AxisScores {
        mechanics: Some(mechanics_score),
        outcome: None,
        overall: mechanics_score,
    }
}

/// The policy for a practice mode.
///
/// Panics for every mode but `Fundamentals`, which is Python's `NotImplementedError` — the same
/// choice [`crate::checkpoints::evaluator_for`] and `contracts::checkpoints::spec_for` make, and for
/// the same reason: a mode with no policy is a wiring bug, and a caller handed an `Option` here
/// would have to invent a fallback score, which is the one thing ADR-010 §2 forbids.
///
/// The `match` is exhaustive rather than an `if` with a fall-through, so adding a `PracticeMode`
/// stops the build here — at the table that has to grow — rather than compiling and failing at
/// runtime on the first golfer who selects it.
///
/// The message no longer says "lands in full M4", as Python's still does (M32). ADR-034 §5 retired
/// that wait — a shot is graded per club over many shots, never one swing at a time — and the
/// frozen copy stays wrong until M40 deletes it, as `contracts/intent.py`'s docs do.
pub fn policy_for(mode: PracticeMode) -> Combine {
    match mode {
        PracticeMode::Fundamentals => fundamentals,
        PracticeMode::ShotShaping | PracticeMode::Performance | PracticeMode::Drill => {
            panic!("{mode:?} has no single-swing scoring policy (ADR-034 §5)")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scored(name: &str, score: f64) -> CheckpointScore {
        CheckpointScore {
            name: name.to_string(),
            score,
            passed: score >= 1.0,
            observed: None,
            expected_low: None,
            expected_high: None,
            message: String::new(),
            percentile: None,
            population_n: None,
            one_sided: false,
        }
    }

    #[test]
    fn fundamentals_is_the_mechanics_mean_as_a_percentage() {
        let scores = [scored("a", 1.0), scored("b", 0.5), scored("c", 0.75)];
        let blend = policy_for(PracticeMode::Fundamentals)(&scores, &[]);
        assert_eq!(blend.mechanics, Some(75.0));
        assert_eq!(blend.overall, 75.0);
        assert_eq!(blend.outcome, None);
    }

    /// The outcome axis is ignored rather than blended, so handing it scores moves nothing. Today
    /// the engine always passes an empty list; this is what says the *policy* is the reason, not the
    /// emptiness.
    #[test]
    fn an_outcome_score_does_not_reach_the_blend() {
        let mechanics = [scored("a", 1.0)];
        let outcome = [scored("carry", 0.0)];
        let with = policy_for(PracticeMode::Fundamentals)(&mechanics, &outcome);
        let without = policy_for(PracticeMode::Fundamentals)(&mechanics, &[]);
        assert_eq!(with, without);
        assert_eq!(with.overall, 100.0);
    }

    /// An empty axis is `0.0`, not `None` and not a panic — the convention every stored artifact
    /// carries. `mechanics` is still `Some`, because the policy did run.
    #[test]
    fn nothing_scored_blends_to_zero_rather_than_to_nothing() {
        let blend = policy_for(PracticeMode::Fundamentals)(&[], &[]);
        assert_eq!(blend.mechanics, Some(0.0));
        assert_eq!(blend.overall, 0.0);
    }

    /// The mean is summed left to right, and this pins that order rather than the value.
    ///
    /// **The name is older than the finding that this is not Python's order** (M34 P2): CPython
    /// 3.12's compensated `sum()` makes this very input's mean exactly `20.0`, where the left fold
    /// asserted here gives `20.000000000000004`. The difference sits inside `RTOL`, and the test
    /// stays as written until the routing question in `mean_percent`'s doc is answered.
    #[test]
    fn the_mean_sums_in_the_pythons_order() {
        let scores = [scored("a", 0.1), scored("b", 0.2), scored("c", 0.3)];
        let expected = ((0.0 + 0.1) + 0.2 + 0.3) / 3.0 * 100.0;
        assert_eq!(mean_percent(&scores), expected);
    }

    #[test]
    #[should_panic(expected = "has no single-swing scoring policy")]
    fn an_unimplemented_mode_refuses_rather_than_grading_mechanics_anyway() {
        policy_for(PracticeMode::Performance);
    }
}
