//! Mechanics checkpoints — pose-based, intent-independent. [M22 P5]
//!
//! The port of `analysis/checkpoints/mechanics.py`. **This module judges; it does not measure.**
//! The quantities come from [`crate::measure`] (P4) and everything here turns one into a verdict:
//! resolve a band, score against it, place it in the reference population, phrase it for a golfer.
//! The split is what lets a new metric be measured across the corpus before a band for it exists.
//!
//! Every checkpoint is measured from **face-on 2D pose**. Which of them ship, and in what order,
//! is [`contracts::checkpoints::CHECKPOINT_REGISTRY`]; the Python module docstring is what each
//! one *means*, and it is not restated here — it carries the metric-definitions history and the
//! HARDWARE-REVALIDATE note, neither of which a port can keep true on its own.
//!
//! # The six evaluators are one skeleton written six times, and that is the port
//!
//! Measure, refuse or continue; resolve a band, refuse or continue; score; phrase; place. A
//! generic `judge(spec, measured, phrasing)` would collapse them and is deliberately not written:
//! the phrasing is not a parameter but a branch structure that differs per checkpoint — tempo has
//! three arms and a prescription clause, `head_stays_back` re-signs its observation before it
//! scores and phrases a magnitude its band does not carry — and every phase of M22 gets diffed
//! against its Python source by eye at least once. A shape that merges the six cannot be. What
//! *is* shared is shared in Python too: [`score_within_range`], [`population_placement`] and
//! [`placement_clause`] are the same three helpers with the same three jobs.
//!
//! # Where the strings come from
//!
//! Every interpolation below goes through [`crate::pyfmt`], and `docs/CONFORMANCE.md` §3 compares
//! these sentences **exactly**. Three of them reach the fifth edge — `f"aim under {band.high}"`
//! has no format spec at all, so it is [`pyfmt::repr`] and not Rust's `{}`, which would write `4`
//! where CPython writes `4.0`.

use contracts::checkpoints::spec_for;
use contracts::golfer::Handedness;
use contracts::intent::{ClubCategory, PlayerProfile};
use contracts::keypoints::FrameKeypoints;
use contracts::swing::{CheckpointScore, PhaseSegment};
use contracts::unscored::UnscoredReason;

use crate::benchmarks::distributions::load_distribution_any;
use crate::benchmarks::store::resolve_range;
use crate::checkpoints::CheckpointOutcome;
use crate::measure::{
    measure_finish_balance, measure_head_hip_gain, measure_head_sway, measure_hip_shift_at_top,
    measure_hip_sway, measure_tempo_ratio, tempo_timings, MeasureOutcome,
};
use crate::pyfmt;

// Name, band key and band shape all come off the spec rather than being retyped here.
// `contracts::checkpoints` owns the pairing because `caveats.py` has to build prose out of it and
// cannot import this package (ADR-008); keeping a second copy is what let the shipped caveat text
// claim five checkpoints while six ran.
//
// `&'static str` constants rather than Python's module attributes holding a whole spec: the names
// are public vocabulary that `SwingResult.unscored`, `feedback/rules.py` and the tests all match
// on, while the band key and `one_sided` are read once inside the evaluator that needs them. A
// `spec_for` call at the top of each one is what keeps them off this list.
pub const TEMPO_CHECKPOINT: &str = "tempo";
pub const HEAD_SWAY_CHECKPOINT: &str = "head_sway";
pub const FINISH_BALANCE_CHECKPOINT: &str = "finish_balance";
pub const HIP_SWAY_CHECKPOINT: &str = "hip_sway";
pub const HIP_SHIFT_AT_TOP_CHECKPOINT: &str = "hip_shift_at_top";
pub const HEAD_STAYS_BACK_CHECKPOINT: &str = "head_stays_back";

/// Carry a measurement's refusal outward unchanged.
///
/// [`crate::measure`] already decided which of its conditions failed, and it is the only layer
/// that still has the window and the landmark group in hand. Re-deriving a reason here would be a
/// second opinion on evidence this module can no longer see — and the first opinion is the one
/// that was there.
fn no_measurement(measured: &MeasureOutcome) -> CheckpointOutcome {
    let reason = measured
        .reason
        .expect("a measurement with no value must carry a reason");
    CheckpointOutcome::unscored_for(reason, measured.detail.clone())
}

/// The judging-side refusal: the number is fine, there is nothing to judge it against.
///
/// Names the club because that is what makes it actionable — [`resolve_range`] matches
/// `club_category` exactly, so a miss here is usually a per-club row that does not exist rather
/// than a metric with no band at all (ADR-010 addendum, 2026-08-18).
fn no_band(range_key: &str, club: ClubCategory) -> CheckpointOutcome {
    CheckpointOutcome::unscored_for(
        UnscoredReason::NoBand,
        format!(
            "no band in ranges.json for {range_key} at club={}",
            club.as_str()
        ),
    )
}

/// 1.0 inside `[low, high]`, decaying linearly (by band-widths) to 0.0 outside.
///
/// Shared scorer for every mechanics checkpoint here. For a one-sided "lower is better" metric
/// pass `low = 0.0`, so any non-negative value inside the band scores 1.0 and only overshoot past
/// `high` is penalised.
pub fn score_within_range(observed: f64, low: f64, high: f64) -> f64 {
    if low <= observed && observed <= high {
        return 1.0;
    }
    let width = high - low;
    // **Load-bearing in Python, redundant here, and kept anyway.** `1.0 - distance / 0.0` raises
    // `ZeroDivisionError` in CPython, where in Rust it is `-inf` and the `max` below flattens it to
    // the same `0.0` this returns — so a mutation deleting this guard survives every test and all
    // 21 vectors. It stays because the equivalence is an accident of two later lines: move the
    // clamp, or return the raw decay, and the guard is what stops a `-inf` reaching
    // `CheckpointScore`'s `ge(0)` bound with a message about the wrong thing.
    if width <= 0.0 {
        return 0.0;
    }
    let distance = if observed < low {
        low - observed
    } else {
        observed - high
    };
    (1.0 - distance / width).max(0.0)
}

/// Where `observed` sits in the reference population: `(percentile, n)`, or `(None, None)`.
///
/// **Informational only — never feeds `score` or `passed`.** Scoring reads `ranges.json` and
/// nothing else (ADR-010 §2, and the percentile addendum that admits this call). The band answers
/// *is this a fault*; the percentile answers *how far off, in units comparable to the other
/// checkpoints*, which is what lets `feedback` rank tips instead of listing them.
///
/// Queries the **same stratum the band was cut from** — all three axes `"all"`. See
/// [`load_distribution_any`] for the p90 that consistency is protecting.
fn population_placement(range_key: &str, observed: f64) -> (Option<f64>, Option<i64>) {
    match load_distribution_any(range_key) {
        None => (None, None),
        Some(distribution) => (
            Some(pyfmt::round_to(distribution.percentile_of(observed), 1)),
            Some(distribution.n),
        ),
    }
}

/// Plain-English population placement, phrased for a golfer rather than a statistician.
///
/// `pct` is the share of the population *below* the observed value, so a swing under the median is
/// described against the share it undercuts (`below_label`) and one over it against the share it
/// exceeds (`above_label`). Callers supply both labels because the comparison only reads naturally
/// in the metric's own vocabulary — "a quicker tempo than" and "more head movement than" are not
/// the same sentence with a sign flipped.
///
/// `Distribution::percentile_of` clamps to `[10, 90]` because the tails were never stored, so at
/// the rails this says "at least" rather than quoting a rank the stored quantiles cannot support.
fn placement_clause(pct: f64, n: i64, below_label: &str, above_label: &str) -> String {
    let (share, label, at_rail) = if pct <= 50.0 {
        (100.0 - pct, below_label, pct <= 10.0)
    } else {
        (pct, above_label, pct >= 90.0)
    };
    let qualifier = if at_rail { "at least " } else { "" };
    format!(
        "{label} {qualifier}{}% of {n} tour swings",
        pyfmt::fixed(share, 0)
    )
}

/// Score swing tempo (backswing:downswing ratio) against the benchmark band.
///
/// Produces no score when the phases don't yield a usable tempo or when the store has no band for
/// this checkpoint. The two are reported as different reasons, because the first is a clip worth
/// shooting again and the second is not the golfer's to fix at all.
pub fn evaluate_tempo(
    phases: &[PhaseSegment],
    club: ClubCategory,
    profile: Option<&PlayerProfile>,
) -> CheckpointOutcome {
    let spec = spec_for(TEMPO_CHECKPOINT);
    let measured = measure_tempo_ratio(phases);
    let Some(observed) = measured.value else {
        return no_measurement(&measured);
    };

    let Some(band) = resolve_range(spec.metric, club, profile) else {
        return no_band(spec.metric, club);
    };

    let score = score_within_range(observed, band.low, band.high);
    let passed = band.low <= observed && observed <= band.high;
    // Quote the resolved band rather than a hardcoded "~3:1". The band is sourced data (ADR-010)
    // and has already been re-cut once, so a literal here would have started lying when it moved.
    let target = format!(
        "tour range {}-{}:1",
        pyfmt::g(band.low),
        pyfmt::g(band.high)
    );

    // The verdict prescribes a *backswing* duration, not a ratio and not a downswing (M13,
    // ADR-023's 2026-09-02 addendum). The downswing is how hard the golfer swung — the thing they
    // feel rather than choose — so it is read as given and the backswing is what the sentence asks
    // them to move. Both edges are the same band applied to that observed downswing, which is why
    // this sentence cannot contradict the ratio verdict above.
    //
    // `tempo_timings` is the same call `measure_tempo_ratio` made, so durations are present
    // whenever a ratio is — the `expect` is the narrowing, not a second opinion.
    let (backswing_ms, downswing_ms) = tempo_timings(phases)
        .durations
        .expect("a tempo ratio implies both durations");
    let prescription = format!(
        " Your downswing was {} ms; at the tour ratio that wants a {}-{} ms backswing, and yours \
         was {}.",
        pyfmt::fixed(downswing_ms, 0),
        pyfmt::fixed(band.low * downswing_ms, 0),
        pyfmt::fixed(band.high * downswing_ms, 0),
        pyfmt::fixed(backswing_ms, 0),
    );

    let mut message = if passed {
        format!(
            "Good tempo - {}:1 backswing:downswing (inside the {target}).{prescription}",
            pyfmt::fixed(observed, 1)
        )
    } else if observed < band.low {
        format!(
            "Tempo too quick - {}:1.{prescription} Take it back longer.",
            pyfmt::fixed(observed, 1)
        )
    } else {
        format!(
            "Tempo too slow - {}:1.{prescription} Take it back shorter.",
            pyfmt::fixed(observed, 1)
        )
    };

    let (pct, population_n) = population_placement(spec.metric, observed);
    if let (Some(pct), Some(n)) = (pct, population_n) {
        let clause = placement_clause(pct, n, "a quicker tempo than", "a slower tempo than");
        message.push_str(&format!(" You swing with {clause}."));
    }

    CheckpointOutcome::scored(CheckpointScore {
        name: TEMPO_CHECKPOINT.to_string(),
        score,
        passed,
        observed: Some(pyfmt::round_to(observed, 2)),
        expected_low: Some(band.low),
        expected_high: Some(band.high),
        message,
        percentile: pct,
        population_n,
        one_sided: spec.one_sided,
    })
}

/// Score lateral head stability: `x` travel of the head center from address to impact.
///
/// The head center is the **ear midpoint**, measured in shoulder-widths so it is independent of
/// the golfer's distance from the camera. Both endpoints are means over a window, which is what
/// makes this checkpoint robust: averaging N frames suppresses zero-mean landmark jitter by √N, so
/// the remaining error is dominated by *definition*, not by the pose model.
pub fn evaluate_head_sway(
    keypoints: &[FrameKeypoints],
    phases: &[PhaseSegment],
    club: ClubCategory,
    profile: Option<&PlayerProfile>,
) -> CheckpointOutcome {
    let spec = spec_for(HEAD_SWAY_CHECKPOINT);
    let measured = measure_head_sway(keypoints, phases);
    let Some(observed) = measured.value else {
        return no_measurement(&measured);
    };

    let Some(band) = resolve_range(spec.metric, club, profile) else {
        return no_band(spec.metric, club);
    };

    let score = score_within_range(observed, band.low, band.high);
    let passed = band.low <= observed && observed <= band.high;
    let mut message = if passed {
        format!(
            "Good head stability - {} shoulder-widths of lateral head movement to impact (nicely \
             centered over the ball).",
            pyfmt::fixed(observed, 2)
        )
    } else {
        format!(
            "Head sway - {} shoulder-widths of lateral movement to impact. Keep your head \
             centered over the ball (aim under {}).",
            pyfmt::fixed(observed, 2),
            pyfmt::repr(band.high)
        )
    };

    let (pct, population_n) = population_placement(spec.metric, observed);
    if let (Some(pct), Some(n)) = (pct, population_n) {
        let clause = placement_clause(pct, n, "less head movement than", "more head movement than");
        message.push_str(&format!(" That is {clause}."));
    }

    CheckpointOutcome::scored(CheckpointScore {
        name: HEAD_SWAY_CHECKPOINT.to_string(),
        score,
        passed,
        observed: Some(pyfmt::round_to(observed, 2)),
        expected_low: Some(band.low),
        expected_high: Some(band.high),
        message,
        percentile: pct,
        population_n,
        one_sided: spec.one_sided,
    })
}

/// Score finish balance: how far the hip-center drifts from its own mean through follow-through.
///
/// A balanced swing settles into a held finish (small drift); an off-balance one keeps staggering.
/// Drift is summarized at the **p90** of the per-frame series rather than its `max` — `max` made
/// this the one checkpoint where a single mis-detected frame passed straight through to the score
/// unattenuated, and it read hips at the most occluded moment in the swing.
pub fn evaluate_finish_balance(
    keypoints: &[FrameKeypoints],
    phases: &[PhaseSegment],
    club: ClubCategory,
    profile: Option<&PlayerProfile>,
) -> CheckpointOutcome {
    let spec = spec_for(FINISH_BALANCE_CHECKPOINT);
    let measured = measure_finish_balance(keypoints, phases);
    let Some(observed) = measured.value else {
        return no_measurement(&measured);
    };

    let Some(band) = resolve_range(spec.metric, club, profile) else {
        return no_band(spec.metric, club);
    };

    let score = score_within_range(observed, band.low, band.high);
    let passed = band.low <= observed && observed <= band.high;
    let mut message = if passed {
        format!(
            "Balanced finish - the body settles within {} shoulder-widths through the \
             follow-through (held and steady).",
            pyfmt::fixed(observed, 2)
        )
    } else {
        format!(
            "Unbalanced finish - {} shoulder-widths of drift after impact. Swing to a held, \
             balanced finish (aim under {}).",
            pyfmt::fixed(observed, 2),
            pyfmt::repr(band.high)
        )
    };

    let (pct, population_n) = population_placement(spec.metric, observed);
    if let (Some(pct), Some(n)) = (pct, population_n) {
        let clause = placement_clause(pct, n, "a steadier finish than", "a looser finish than");
        message.push_str(&format!(" That is {clause}."));
    }

    CheckpointOutcome::scored(CheckpointScore {
        name: FINISH_BALANCE_CHECKPOINT.to_string(),
        score,
        passed,
        observed: Some(pyfmt::round_to(observed, 2)),
        expected_low: Some(band.low),
        expected_high: Some(band.high),
        message,
        percentile: pct,
        population_n,
        one_sided: spec.one_sided,
    })
}

/// Score lateral hip travel from address to impact — the **two-sided** checkpoint.
///
/// The structural twin of [`evaluate_head_sway`] measuring a different thing: head sway asks
/// whether the golfer stayed centred, hip sway asks how much the lower body moved laterally into
/// the shot.
///
/// **Two-sided, and that is the substance rather than a detail.** Every other spatial band here is
/// `[0, p90]`, which asserts that less is better — not established for hip travel, where the tour
/// p10 is 0.14 and a `[0, p90]` band would score a golfer who barely moves their lower body as
/// perfect. The lower edge is safe to assert because it clears the instrument by 2.8x.
pub fn evaluate_hip_sway(
    keypoints: &[FrameKeypoints],
    phases: &[PhaseSegment],
    club: ClubCategory,
    profile: Option<&PlayerProfile>,
) -> CheckpointOutcome {
    let spec = spec_for(HIP_SWAY_CHECKPOINT);
    let measured = measure_hip_sway(keypoints, phases);
    let Some(observed) = measured.value else {
        return no_measurement(&measured);
    };

    let Some(band) = resolve_range(spec.metric, club, profile) else {
        return no_band(spec.metric, club);
    };

    let score = score_within_range(observed, band.low, band.high);
    let passed = band.low <= observed && observed <= band.high;
    let target = format!("tour range {}-{}", pyfmt::g(band.low), pyfmt::g(band.high));
    let mut message = if passed {
        format!(
            "Good lower-body movement - {} shoulder-widths of lateral hip travel to impact \
             (inside the {target}).",
            pyfmt::fixed(observed, 2)
        )
    } else if observed < band.low {
        // Deliberately phrased as a fact about the population rather than as a cause. This
        // pipeline measures lateral hip position; it does not see weight, pressure or rotation,
        // and the standing caveats forbid inferring them.
        format!(
            "Little hip movement - {} shoulder-widths of lateral hip travel to impact, under the \
             {target}. Almost every tour swing moves the hips further into the shot than this.",
            pyfmt::fixed(observed, 2)
        )
    } else {
        format!(
            "Hip slide - {} shoulder-widths of lateral hip travel to impact, past the {target}. \
             The hips are travelling sideways more than tour swings do.",
            pyfmt::fixed(observed, 2)
        )
    };

    let (pct, population_n) = population_placement(spec.metric, observed);
    if let (Some(pct), Some(n)) = (pct, population_n) {
        let clause = placement_clause(pct, n, "less hip travel than", "more hip travel than");
        message.push_str(&format!(" That is {clause}."));
    }

    CheckpointOutcome::scored(CheckpointScore {
        name: HIP_SWAY_CHECKPOINT.to_string(),
        score,
        passed,
        observed: Some(pyfmt::round_to(observed, 2)),
        expected_low: Some(band.low),
        expected_high: Some(band.high),
        message,
        percentile: pct,
        population_n,
        one_sided: spec.one_sided,
    })
}

/// Score lateral hip travel from address to the top of the backswing.
///
/// **One-sided, for a reason that is not "less is better".** The lower edge is omitted because it
/// is *unmeasurable*: the tour p10 is 0.015 against a noise+boundary error of 0.053, so a band
/// edge there would separate golfers this pipeline cannot tell apart. Judging only overshoot
/// judges only the half the instrument resolves.
///
/// **Magnitude only, so this says nothing about direction** — sign would separate a slide away
/// from the target from a reverse pivot toward it, and the sign is camera-relative while
/// handedness is not resolved on the analysis path.
pub fn evaluate_hip_shift_at_top(
    keypoints: &[FrameKeypoints],
    phases: &[PhaseSegment],
    club: ClubCategory,
    profile: Option<&PlayerProfile>,
) -> CheckpointOutcome {
    let spec = spec_for(HIP_SHIFT_AT_TOP_CHECKPOINT);
    let measured = measure_hip_shift_at_top(keypoints, phases);
    let Some(observed) = measured.value else {
        return no_measurement(&measured);
    };

    let Some(band) = resolve_range(spec.metric, club, profile) else {
        return no_band(spec.metric, club);
    };

    let score = score_within_range(observed, band.low, band.high);
    let passed = band.low <= observed && observed <= band.high;
    let mut message = if passed {
        format!(
            "Steady hips going back - {} shoulder-widths of lateral hip travel to the top \
             (centered over the ball at the top).",
            pyfmt::fixed(observed, 2)
        )
    } else {
        format!(
            "Hip slide going back - {} shoulder-widths of lateral hip travel to the top. The hips \
             are travelling sideways during the backswing rather than staying centered (aim under \
             {}).",
            pyfmt::fixed(observed, 2),
            pyfmt::repr(band.high)
        )
    };

    let (pct, population_n) = population_placement(spec.metric, observed);
    if let (Some(pct), Some(n)) = (pct, population_n) {
        let clause = placement_clause(
            pct,
            n,
            "less hip slide going back than",
            "more hip slide going back than",
        );
        message.push_str(&format!(" That is {clause}."));
    }

    CheckpointOutcome::scored(CheckpointScore {
        name: HIP_SHIFT_AT_TOP_CHECKPOINT.to_string(),
        score,
        passed,
        observed: Some(pyfmt::round_to(observed, 2)),
        expected_low: Some(band.low),
        expected_high: Some(band.high),
        message,
        percentile: pct,
        population_n,
        one_sided: spec.one_sided,
    })
}

/// Score how much rearward head-hip separation the swing created, address to impact.
///
/// "Staying behind the ball", and the first checkpoint here whose *sign* carries meaning — which
/// is why it is also the first that cannot be scored without knowing who swung.
///
/// **Refuses with `NO_HANDEDNESS` when `handedness` is `None`, and that is the feature.** A
/// face-on camera sees a left-handed swing mirrored, so the same body position lands on the
/// opposite sign. Defaulting to right-handed would read a left-handed golfer's perfectly ordinary
/// impact position as a gross fault — silently, and in the direction nothing downstream can
/// detect. An unscored checkpoint is reported, with its reason, in `SwingResult.unscored`; a
/// wrongly scored one is not reported at all.
///
/// **Two-sided**, and neither edge is "less is better": too little separation is the head drifting
/// forward with the hips, too much is hanging back behind the ball through impact.
pub fn evaluate_head_stays_back(
    keypoints: &[FrameKeypoints],
    phases: &[PhaseSegment],
    handedness: Option<Handedness>,
    club: ClubCategory,
    profile: Option<&PlayerProfile>,
) -> CheckpointOutcome {
    let spec = spec_for(HEAD_STAYS_BACK_CHECKPOINT);
    // Two quite different failures, and merging them into one refusal is what forced `feedback` to
    // infer which had happened by checking whether the metric survived into `measurements`. An
    // unreadable clip wants re-filming; an unattributed swing wants a golfer picked, and telling
    // the second golfer to go back to the bay is the wrong answer confidently given.
    let measured = measure_head_hip_gain(keypoints, phases);
    let Some(raw) = measured.value else {
        return no_measurement(&measured);
    };
    let Some(handedness) = handedness else {
        return CheckpointOutcome::unscored_for(
            UnscoredReason::NoHandedness,
            "the head-hip gain was measured, but its sign is camera-relative and no golfer is \
             attributed to this swing",
        );
    };

    // Into the right-handed camera frame the band is cut in. The corpus was folded the same way
    // (`derive_reference.py`), so `observed`, the band and the stored distribution all share one
    // convention — the percentile below would otherwise read a mirrored value against an
    // unmirrored population and quietly return a plausible, wrong rank.
    let observed = if handedness == Handedness::Right {
        raw
    } else {
        -raw
    };

    let Some(band) = resolve_range(spec.metric, club, profile) else {
        return no_band(spec.metric, club);
    };

    let score = score_within_range(observed, band.low, band.high);
    let passed = band.low <= observed && observed <= band.high;
    // Phrased as a magnitude in the golfer's own terms. The stored number is negative because the
    // band lives in a camera frame, and "-0.16 shoulder-widths of separation" is not a sentence.
    let travel = observed.abs();
    let mut message = if passed {
        format!(
            "Head stayed behind the ball - {} shoulder-widths of separation opened up between \
             head and hips through impact (inside the tour range).",
            pyfmt::fixed(travel, 2)
        )
    // `>` and `>=` are the same test here and a mutation between them survives: this arm is only
    // reached when `passed` is false, and `observed == band.high` makes `passed` true. Worth a line
    // rather than a second look, because the comparison reads like a boundary decision and is not.
    } else if observed > band.high {
        format!(
            "Head drifting forward - only {} shoulder-widths of separation between head and hips \
             by impact. The head is travelling toward the target with the hips rather than \
             staying back (tour swings open up {}-{}).",
            pyfmt::fixed(travel, 2),
            pyfmt::g(band.high.abs()),
            pyfmt::g(band.low.abs()),
        )
    } else {
        format!(
            "Hanging back - {} shoulder-widths of separation between head and hips by impact, \
             more than tour swings show (they open up {}-{}). The upper body is staying behind \
             the ball through impact rather than moving through the shot.",
            pyfmt::fixed(travel, 2),
            pyfmt::g(band.high.abs()),
            pyfmt::g(band.low.abs()),
        )
    };

    let (pct, population_n) = population_placement(spec.metric, observed);
    if let (Some(pct), Some(n)) = (pct, population_n) {
        // The population is stored in the same negative frame, so a *low* percentile is the
        // hanging-back end and a *high* one is the drifting-forward end. Labels follow the frame,
        // not the prose above.
        let clause = placement_clause(
            pct,
            n,
            "more head-behind separation than",
            "less head-behind separation than",
        );
        message.push_str(&format!(" That is {clause}."));
    }

    CheckpointOutcome::scored(CheckpointScore {
        name: HEAD_STAYS_BACK_CHECKPOINT.to_string(),
        score,
        passed,
        observed: Some(pyfmt::round_to(observed, 2)),
        expected_low: Some(band.low),
        expected_high: Some(band.high),
        message,
        percentile: pct,
        population_n,
        one_sided: spec.one_sided,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::benchmarks::store::ResolvedRange;
    use crate::testing::{a_body, a_segmentation, phase, put};
    use contracts::checkpoints::CHECKPOINT_REGISTRY;
    use contracts::keypoints::PoseLandmark;
    use contracts::swing::SwingPhase;

    /// A band without going through `ranges.json`, so a scorer test states its own edges rather
    /// than moving when a band is re-cut.
    fn band(low: f64, high: f64) -> ResolvedRange {
        ResolvedRange {
            low,
            high,
            source: "a test".to_string(),
        }
    }

    #[test]
    fn the_constants_are_the_registered_names() {
        let named = [
            TEMPO_CHECKPOINT,
            HEAD_SWAY_CHECKPOINT,
            FINISH_BALANCE_CHECKPOINT,
            HIP_SWAY_CHECKPOINT,
            HIP_SHIFT_AT_TOP_CHECKPOINT,
            HEAD_STAYS_BACK_CHECKPOINT,
        ];
        let registered: Vec<&str> = CHECKPOINT_REGISTRY.iter().map(|s| s.name).collect();
        assert_eq!(named.to_vec(), registered);
    }

    // ------------------------------------------------------------------ score_within_range

    #[test]
    fn inside_the_band_scores_one() {
        let b = band(2.72, 4.71);
        assert_eq!(score_within_range(3.0, b.low, b.high), 1.0);
        assert_eq!(score_within_range(b.low, b.low, b.high), 1.0);
        assert_eq!(score_within_range(b.high, b.low, b.high), 1.0);
    }

    #[test]
    fn one_band_width_outside_scores_zero_and_no_further() {
        let (low, high) = (2.0, 3.0);
        assert!((score_within_range(4.0, low, high) - 0.0).abs() < 1e-12);
        assert_eq!(score_within_range(10.0, low, high), 0.0);
        assert_eq!(score_within_range(-10.0, low, high), 0.0);
    }

    #[test]
    fn the_decay_is_linear_in_band_widths_on_both_sides() {
        let (low, high) = (2.0, 4.0);
        assert!((score_within_range(5.0, low, high) - 0.5).abs() < 1e-12);
        assert!((score_within_range(1.0, low, high) - 0.5).abs() < 1e-12);
    }

    /// A zero-width band scores 0 outside it rather than dividing. No shipped row is one, and the
    /// Python guards it anyway — a port that divided would answer `-inf` and then fail
    /// `CheckpointScore`'s `ge(0)` bound with a message about the wrong thing.
    #[test]
    fn a_zero_width_band_scores_zero_outside_rather_than_dividing() {
        assert_eq!(score_within_range(3.0, 2.0, 2.0), 0.0);
        assert_eq!(score_within_range(2.0, 2.0, 2.0), 1.0);
    }

    // ------------------------------------------------------------------ placement_clause

    #[test]
    fn below_the_median_is_described_against_the_share_it_undercuts() {
        assert_eq!(
            placement_clause(
                30.0,
                458,
                "less head movement than",
                "more head movement than"
            ),
            "less head movement than 70% of 458 tour swings"
        );
    }

    #[test]
    fn above_the_median_is_described_against_the_share_it_exceeds() {
        assert_eq!(
            placement_clause(
                75.0,
                458,
                "less head movement than",
                "more head movement than"
            ),
            "more head movement than 75% of 458 tour swings"
        );
    }

    /// The rails say "at least", because `percentile_of` clamps there and the stored quantiles
    /// cannot support a precise rank past them.
    #[test]
    fn both_rails_qualify_the_share() {
        assert_eq!(
            placement_clause(10.0, 458, "below", "above"),
            "below at least 90% of 458 tour swings"
        );
        assert_eq!(
            placement_clause(90.0, 458, "below", "above"),
            "above at least 90% of 458 tour swings"
        );
    }

    /// Exactly 50 takes the *below* arm, which is `<=` in Python and would be `<` in the obvious
    /// port. The two labels are different sentences, so this is a visible error on any swing that
    /// lands on the median.
    #[test]
    fn the_median_itself_takes_the_below_arm() {
        assert_eq!(
            placement_clause(50.0, 100, "below", "above"),
            "below 50% of 100 tour swings"
        );
    }

    /// `{share:.0f}` is banker's rounding, so 70.5 prints 70 and 71.5 prints 72. Rust's own
    /// `{:.0}` agrees — this pins that [`pyfmt::fixed`] is what is being called, because a port
    /// that reached for a scaled round would print 71 here.
    #[test]
    fn the_share_rounds_half_to_even_like_cpython() {
        assert_eq!(
            placement_clause(29.5, 1, "below", "above"),
            "below 70% of 1 tour swings"
        );
        assert_eq!(
            placement_clause(71.5, 1, "below", "above"),
            "above 72% of 1 tour swings"
        );
    }

    // ------------------------------------------------------------------ the evaluators

    /// **The mirror is not gated by a single committed vector, and this is what holds it.**
    ///
    /// One of the 21 vectors is left-handed and its `head_hip_gain_norm` is exactly `0.0`, so
    /// `-raw == raw` and a port that deleted the mirror outright passes the whole stage — measured
    /// by mutation, not supposed. Which matters more here than anywhere else in the panel: the
    /// mirror is the *entire reason* this checkpoint takes a `Handedness`, and getting it wrong
    /// reads a left-handed golfer's ordinary impact position as a gross fault, silently.
    ///
    /// The body below opens 0.2 shoulder-widths of separation, which lands outside the band one way
    /// round and inside it the other — so the two handednesses disagree about the observation, the
    /// verdict and the sentence, all three.
    #[test]
    fn the_handedness_mirror_flips_the_observation_and_the_verdict() {
        let mut frames = a_body(50);
        // Ears slide 0.06 across the impact window against a 0.30 shoulder width.
        for frame in frames.iter_mut().skip(40) {
            put(frame, PoseLandmark::LeftEar, 0.58, 0.20, 1.0);
            put(frame, PoseLandmark::RightEar, 0.54, 0.20, 1.0);
        }
        let phases = a_segmentation();
        let judged = |hand| match evaluate_head_stays_back(
            &frames,
            &phases,
            Some(hand),
            ClubCategory::All,
            None,
        ) {
            CheckpointOutcome::Scored(score) => score,
            other => panic!("expected a score, got {other:?}"),
        };
        let right = judged(Handedness::Right);
        let left = judged(Handedness::Left);

        assert_eq!(right.observed, Some(0.2));
        assert_eq!(left.observed, Some(-0.2));
        // The band is `[-0.67, -0.14]`, so the right-handed reading is past the high edge and the
        // left-handed one is inside it.
        assert!(!right.passed && left.passed);
        assert!(
            right.message.starts_with("Head drifting forward"),
            "{}",
            right.message
        );
        assert!(
            left.message.starts_with("Head stayed behind the ball"),
            "{}",
            left.message
        );
    }

    /// A refused measurement and a missing golfer are different answers, and the order matters:
    /// an unreadable clip is reported as unreadable even when no handedness was given, because
    /// telling that golfer to re-film is the right answer and telling them to pick a golfer is not.
    #[test]
    fn an_unreadable_clip_outranks_a_missing_golfer() {
        let frames = a_body(50);
        let phases = vec![phase(SwingPhase::Address, 0, 10, true)];
        match evaluate_head_stays_back(&frames, &phases, None, ClubCategory::All, None) {
            CheckpointOutcome::Unscored { reason, .. } => {
                assert_eq!(reason, UnscoredReason::PhaseNotSegmented);
            }
            other => panic!("expected a refusal, got {other:?}"),
        }
    }

    #[test]
    fn a_measured_swing_with_no_golfer_refuses_for_the_handedness() {
        let frames = a_body(50);
        match evaluate_head_stays_back(&frames, &a_segmentation(), None, ClubCategory::All, None) {
            CheckpointOutcome::Unscored { reason, detail } => {
                assert_eq!(reason, UnscoredReason::NoHandedness);
                assert!(detail.contains("camera-relative"), "{detail}");
            }
            other => panic!("expected a refusal, got {other:?}"),
        }
    }

    /// A timeline whose tempo ratio is exactly the f64 nearest `2.675` — one of
    /// `conformance_vectors._decimal_traps`' canonical values, where CPython's `round(x, 2)` gives
    /// `2.67` and the scaled `(x * 100).round() / 100` gives `2.68`.
    fn a_tempo_of(backswing_ms: f64, downswing_ms: f64) -> Vec<PhaseSegment> {
        let at = |phase, start_ms: f64, end_ms: f64| PhaseSegment {
            phase,
            start_frame: 0,
            end_frame: 1,
            start_ms,
            end_ms,
            detected: true,
        };
        let top = backswing_ms;
        vec![
            at(SwingPhase::Backswing, 0.0, top),
            at(SwingPhase::Transition, top, top),
            at(
                SwingPhase::Impact,
                top + downswing_ms,
                top + downswing_ms + 10.0,
            ),
        ]
    }

    /// **`pyfmt` is gated by 3,059 cases and nothing gated that the evaluators *call* it.** A
    /// mutation replacing `round_to(observed, 2)` with the scaled form passed all 21 vectors,
    /// because no committed swing lands on a value where the two rules differ. This is the call
    /// site asserted directly.
    #[test]
    fn the_observation_is_rounded_by_cpythons_rule_and_not_by_scaling() {
        let phases = a_tempo_of(2675.0, 1000.0);
        let CheckpointOutcome::Scored(score) = evaluate_tempo(&phases, ClubCategory::All, None)
        else {
            panic!("a complete timeline scores");
        };
        assert_eq!(
            score.observed,
            Some(2.67),
            "the scaled form would give 2.68"
        );
    }

    /// And the sentence, whole. Every formatting edge this checkpoint reaches is in one string:
    /// `:.1f` on the ratio, `:.0f` three times in the prescription, `:g` on both band edges, and
    /// `:.0f` on the population share — plus the `at least` the `[10, 90]` clamp puts in front of
    /// it, because a ratio of 2.675 sits under this population's p10.
    ///
    /// Written as a concatenation rather than with `\` continuations so that every space in the
    /// answer is a space someone typed. A sentence a golfer reads is not a place to let the
    /// formatter decide where the words go.
    #[test]
    fn the_tempo_sentence_is_assembled_exactly() {
        let phases = a_tempo_of(2675.0, 1000.0);
        let CheckpointOutcome::Scored(score) = evaluate_tempo(&phases, ClubCategory::All, None)
        else {
            panic!("a complete timeline scores");
        };
        assert_eq!(
            score.message,
            concat!(
                "Tempo too quick - 2.7:1.",
                " Your downswing was 1000 ms; at the tour ratio that wants a 2720-4710 ms",
                " backswing, and yours was 2675.",
                " Take it back longer.",
                " You swing with a quicker tempo than at least 90% of 1399 tour swings."
            )
        );
        assert!(!score.passed);
    }

    // ------------------------------------------------------------------ the refusals

    #[test]
    fn a_missing_band_names_the_club_that_missed() {
        let outcome = no_band("tempo_ratio", ClubCategory::MidIron);
        match outcome {
            CheckpointOutcome::Unscored { reason, detail } => {
                assert_eq!(reason, UnscoredReason::NoBand);
                assert_eq!(
                    detail,
                    "no band in ranges.json for tempo_ratio at club=mid_iron"
                );
            }
            other => panic!("expected a refusal, got {other:?}"),
        }
    }

    #[test]
    fn a_measurement_refusal_is_carried_outward_unchanged() {
        let measured =
            MeasureOutcome::unmeasurable(UnscoredReason::TooFewFrames, "only 2 usable frames");
        match no_measurement(&measured) {
            CheckpointOutcome::Unscored { reason, detail } => {
                assert_eq!(reason, UnscoredReason::TooFewFrames);
                assert_eq!(detail, "only 2 usable frames");
            }
            other => panic!("expected a refusal, got {other:?}"),
        }
    }
}
