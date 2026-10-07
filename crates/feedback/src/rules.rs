//! Rule-based feedback. `feedback/rules.py`. [M22 P6]
//!
//! Maps each `CheckpointScore` in a `SwingResult` to a plain-English [`Tip`], **ordered so the thing
//! worth working on comes first**, plus a one-line headline naming it. Pure function, no I/O. The tip
//! text itself is authored in `analysis/checkpoints/mechanics.py` and passed through.
//!
//! # Why ordering needs two different signals
//!
//! A swing produces two kinds of checkpoint, and no single number ranks both:
//!
//! - **Failures** are ranked by `score`. `_score_within_range` decays in band-widths, so a lower
//!   score is a bigger overshoot. The percentile *cannot* do this job: `ranges.json`'s bands were cut
//!   at the reference p10/p90 (ADR-012) and `Distribution::percentile_of` clamps to `[10, 90]`
//!   because the tails were never stored — so **every** failing checkpoint reports percentile 90
//!   whether it missed by a hair or by triple. The percentile saturates exactly where the band ends.
//! - **Passes** are ranked by percentile, because `score` is *exactly 1.0* for every one of them and
//!   carries no information at all. This is the case that motivated the whole thing: a swing can
//!   score 100/100 with every checkpoint in band, and only the percentile can say that its head sway
//!   is nonetheless higher than 83% of tour swings — the one thing on that swing worth watching.
//!
//! So: failures first, worst overshoot leading; then passes, closest-to-the-edge leading. Severity
//! stays on `score` for the same saturation reason.
//!
//! # Two portability edges land in this file
//!
//! **P4's `max` tie-break**, and this is the third module to meet it. [`headline`]'s closest-call
//! branch is `max(checkpoints, key=…)`, which in Python returns the **first** maximum and in Rust's
//! `max_by` returns the last — so a swing with two equally-placed passing checkpoints would name a
//! different one. `analysis::phases::first_argmax` is the same fix one crate over and cannot be
//! reached from here (ADR-008), so [`first_max_by_tail`] is it spelled again, with the rule in its
//! name rather than in a comment.
//!
//! **`:.0%`**, in [`tip_for`]'s fallback, which is why `pyfmt` grew a `percent`. It is unreachable
//! on every committed vector — each scored checkpoint carries a `message` — so it is gated by
//! `pyfmt`'s unit tests. Until M34 P1 that function lived in `analysis`, which this crate may not
//! import, so it was re-spelled here and pinned against the same CPython answers. `pyfmt` is its
//! own crate now, below both, and the copy and its agreement test are gone: one rule, one spelling.
//!
//! Nothing else here formats a float. The sentences a golfer reads come off
//! `checkpoints/mechanics.rs` and `contracts::unscored`, already rendered.

use contracts::feedback::{FeedbackPayload, Severity, Tip};
use contracts::swing::{CheckpointScore, SwingResult};
use contracts::unscored::{is_inference_reason, UnscoredCheckpoint};
use contracts::Validate;

/// A checkpoint that failed but still scored at or above this is a minor miss; below it, major.
const MINOR_SCORE_FLOOR: f64 = 0.5;

/// Distance from the population median to the stored tail, in percentile points. `percentile_of`
/// clamps at p10/p90, so 40 points is the full usable half-width of the distribution and a tail
/// distance of 1.0 means "at or past the rail".
const TAIL_HALF_SPAN: f64 = 40.0;

/// A *passing* checkpoint this far into the tail is close enough to the edge to name in the headline
/// when nothing has actually failed. 0.5 is the midpoint between the median and the rail — roughly
/// the 70th percentile for a one-sided metric.
const WATCH_TAIL: f64 = 0.5;

/// How far into the population tail this observation sits: 0.0 at the median, 1.0 at the rail.
///
/// Returns `None` when no percentile was resolved, so callers fall back rather than treating an
/// unknown placement as a typical one.
///
/// The two metric shapes need different arithmetic, which is what `one_sided` is for. For a one-sided
/// metric (lower is strictly better) only the *upper* half is a fault, so sitting at the 10th
/// percentile is excellent and scores 0.0. For a two-sided metric both rails are equally wrong, so
/// the distance is symmetric about the median.
fn tail_distance(checkpoint: &CheckpointScore) -> Option<f64> {
    let offset = checkpoint.percentile? - 50.0;
    if checkpoint.one_sided {
        // Python's `max(0.0, x)`. A `CheckpointScore` carries `ge(percentile, 0)` and
        // `le(percentile, 100)`, so there is no NaN here for the two languages' `max` to disagree
        // about — Python returns the first argument on a NaN and `f64::max` returns the other one.
        return Some((offset / TAIL_HALF_SPAN).max(0.0));
    }
    Some(offset.abs() / TAIL_HALF_SPAN)
}

/// `tail_distance` with Python's `or 0.0`, which also swallows a resolved `0.0` — same answer.
fn tail_or_median(checkpoint: &CheckpointScore) -> f64 {
    tail_distance(checkpoint).unwrap_or(0.0)
}

/// Sort key putting the most actionable checkpoint first (ascending). `_rank_key`.
///
/// Failures group ahead of passes, then each group is ordered by the signal that actually
/// discriminates within it — see the module doc for why those are different signals.
fn rank_key(checkpoint: &CheckpointScore) -> (u8, f64) {
    if !checkpoint.passed {
        return (0, checkpoint.score);
    }
    (1, -tail_or_median(checkpoint))
}

/// `INFO` if it passed, else `MINOR`/`MAJOR` by how far outside the band it landed.
///
/// Deliberately still on `score`, not on the percentile: past the band edge the percentile is pinned
/// at the rail and cannot tell a near-miss from a gross one.
fn severity_for(checkpoint: &CheckpointScore) -> Severity {
    if checkpoint.passed {
        return Severity::Info;
    }
    if checkpoint.score >= MINOR_SCORE_FLOOR {
        Severity::Minor
    } else {
        Severity::Major
    }
}

fn tip_for(checkpoint: &CheckpointScore) -> Tip {
    // Python's `message or …` — an empty string is as falsy as a missing one, and `message` defaults
    // to `""` rather than to `None`.
    let text = if checkpoint.message.is_empty() {
        format!(
            "{}: score {}.",
            checkpoint.name,
            pyfmt::percent(checkpoint.score, 0)
        )
    } else {
        checkpoint.message.clone()
    };
    Tip {
        checkpoint: checkpoint.name.clone(),
        text,
        severity: severity_for(checkpoint),
    }
}

/// Python's `str.capitalize()`: the first character uppercased and **the rest lowercased**.
///
/// The second half is the part worth spelling out. Every registered checkpoint name is lowercase
/// today so `to_uppercase` on the first character alone would agree — and a future `hip_ROM` would
/// read as `Hip ROM` here and `Hip rom` in Python, which is a compared string.
fn capitalize(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase(),
    }
}

/// Say plainly that a checkpoint was attempted and could not be scored, and why. `_unmeasured_tip`.
///
/// `overall_score` is a mean over the checkpoints that *did* produce a score, so without this the
/// golfer cannot tell a swing graded on three things from one graded on two (ADR-013).
///
/// **The remedy is read, not inferred.** This used to hold a table keyed on checkpoint name and
/// decide between "film it again" and "pick a golfer" by checking whether the metric had survived
/// into `SwingResult.measurements` — if the number was there the footage must have been readable, so
/// the missing score had to be the handedness case. It gave the right answer for the one checkpoint
/// it covered and could never have covered a second, because the signal it read was a side effect
/// rather than the cause. `contracts::unscored` carries the cause itself, so this is a lookup.
fn unmeasured_tip(entry: &UnscoredCheckpoint) -> Tip {
    Tip {
        checkpoint: entry.name.clone(),
        text: format!(
            "{} could not be scored on this swing, so it is not included in the score. {}",
            capitalize(&entry.name.replace('_', " ")),
            entry.spec().remedy,
        ),
        severity: Severity::Info,
    }
}

/// Python's `max(checkpoints, key=tail)`, which returns the **first** maximum where Rust's `max_by`
/// returns the last. P4's fourth portability edge, in a third module.
///
/// A strict `>` and a forward walk is the whole of it — the first element wins every tie because no
/// later equal one displaces it. Reached only when nothing failed, and only decides *which name* the
/// clean-bill headline mentions, so a wrong tie-break here is a correct number attached to the wrong
/// label: the failure ADR-032 §3 calls the hardest of these to diagnose.
fn first_max_by_tail(checkpoints: &[CheckpointScore]) -> Option<&CheckpointScore> {
    let mut best: Option<&CheckpointScore> = None;
    for checkpoint in checkpoints {
        if best.is_none_or(|b| tail_or_median(checkpoint) > tail_or_median(b)) {
            best = Some(checkpoint);
        }
    }
    best
}

/// The single thing to work on, or a clean bill with the closest call named. `_headline`.
///
/// `None` when there is nothing to say (no checkpoints scored at all) — an empty headline is a
/// truthful "we have no verdict", which the caller renders differently from "everything is fine".
fn headline(checkpoints: &[CheckpointScore]) -> Option<String> {
    if checkpoints.is_empty() {
        return None;
    }

    let failures: Vec<&CheckpointScore> = checkpoints.iter().filter(|c| !c.passed).collect();
    if !failures.is_empty() {
        // `min` returns the first minimum in both languages, which is the half of P4's edge that
        // needs no help — and is exactly why the `max` above does.
        let worst = failures
            .iter()
            .copied()
            .min_by(|a, b| a.score.total_cmp(&b.score))
            .expect("a non-empty list has a minimum");
        return Some(format!(
            "Work on {} first. {}",
            worst.name.replace('_', " "),
            worst.message
        ));
    }

    let closest = first_max_by_tail(checkpoints).expect("a non-empty list has a maximum");
    let clean = format!(
        "All {} checkpoints are inside tour range.",
        checkpoints.len()
    );
    if tail_or_median(closest) >= WATCH_TAIL {
        return Some(format!(
            "{clean} Closest to the edge: {}.",
            closest.name.replace('_', " ")
        ));
    }
    Some(clean)
}

/// Produce ranked rule-based tips from a swing result. `build_feedback`.
///
/// **The ball-flight entries in `unscored` are deliberately not turned into tips** [M15 P11]. They
/// share that list because `contracts::unscored` is where this repo names an absence, but a [`Tip`]
/// carries a `checkpoint` and a simulated flight is not one — ADR-027 §Decision 6 gives it
/// measurement names and no `CHECKPOINT_REGISTRY` entry. [`unmeasured_tip`]'s sentence is the
/// specific harm: *"so it is not included in the score"* is true of every checkpoint and false of a
/// `flight_*` measurement, which was never in `overall_score` to be excluded from. The refusal still
/// reaches every reader through `SwingResult.unscored`, which is where a fact about the launch
/// monitor's tiles belongs; it does not belong in a golfer's tips.
pub fn build_feedback(result: &SwingResult) -> FeedbackPayload {
    // Stable, which is load-bearing: the key is a coarse `(group, signal)` pair and ties inside a
    // group are broken by `CHECKPOINT_REGISTRY` order, because that is the order `checkpoint_scores`
    // arrives in — and every *passing* checkpoint scores exactly 1.0, so ties are the common case
    // rather than the edge.
    //
    // **`sort_unstable_by` survives every test in this workspace** (M22 P6's mutation sweep, one of
    // two survivors), and it is the third time this repo has recorded that: Rust's unstable sort is
    // an insertion sort below 20 elements and `CHECKPOINT_REGISTRY` holds six, so it *is* stable at
    // every length the engine reaches. P3 found the same thing about `OrderedMap::sorted_by` and
    // wrote the same conclusion — the assurance comes from the documented stability of this call and
    // not from any table, and a seventh checkpoint does not change that while twenty is the
    // threshold. Worth knowing which assurances come from where.
    //
    // ADR-032 §3's third edge in its other spelling: Python's `sorted` is stable by guarantee, and a
    // port that reads "sorts the same" as "produces the same order" has a correct set of tips in an
    // order that says the wrong thing is worth working on first.
    let mut ranked: Vec<&CheckpointScore> = result.checkpoint_scores.iter().collect();
    ranked.sort_by(|a, b| {
        let (ga, sa) = rank_key(a);
        let (gb, sb) = rank_key(b);
        ga.cmp(&gb).then(sa.total_cmp(&sb))
    });

    let mut tips: Vec<Tip> = ranked.into_iter().map(tip_for).collect();
    tips.extend(
        result
            .unscored
            .iter()
            .filter(|entry| !is_inference_reason(entry.reason))
            .map(unmeasured_tip),
    );

    let payload = FeedbackPayload {
        swing_id: result.swing_id.clone(),
        overall_score: result.overall_score,
        headline: headline(&result.checkpoint_scores),
        tips,
        // The sidecar's, and written by nothing in Rust (ADR-030 §2). `None` on every vector.
        coaching_text: None,
        coaching: None,
        annotated_video_path: None,
    };
    // Pydantic validates at `FeedbackPayload(...)`; the same explicit call every assembled shape in
    // this port carries, for ADR-032 §4's reason.
    payload
        .validate()
        .unwrap_or_else(|e| panic!("build_feedback assembled an invalid FeedbackPayload: {e}"));
    payload
}

#[cfg(test)]
mod tests {
    use super::*;
    use contracts::unscored::UnscoredReason;

    fn score(name: &str, score: f64, passed: bool, percentile: Option<f64>) -> CheckpointScore {
        CheckpointScore {
            name: name.to_string(),
            score,
            passed,
            observed: Some(0.1),
            expected_low: Some(0.0),
            expected_high: Some(0.5),
            message: format!("{name} says something."),
            percentile,
            population_n: Some(458),
            one_sided: true,
        }
    }

    fn a_result(scores: Vec<CheckpointScore>, unscored: Vec<UnscoredCheckpoint>) -> SwingResult {
        SwingResult {
            swing_id: "1".to_string(),
            session_id: "s".to_string(),
            phases: Vec::new(),
            checkpoint_scores: scores,
            measurements: Vec::new(),
            unscored,
            intent: None,
            mechanics_score: Some(50.0),
            outcome_score: None,
            overall_score: 50.0,
            keypoints: Vec::new(),
            detections: Vec::new(),
            shot: None,
        }
    }

    fn unscored_for(name: &str, reason: UnscoredReason) -> UnscoredCheckpoint {
        UnscoredCheckpoint {
            name: name.to_string(),
            reason,
            detail: String::new(),
        }
    }

    /// Failures lead, worst overshoot first; then passes, furthest into the tail first.
    #[test]
    fn failures_group_ahead_of_passes_and_each_group_uses_its_own_signal() {
        let result = a_result(
            vec![
                score("near_pass", 1.0, true, Some(55.0)),
                score("minor_miss", 0.8, false, Some(90.0)),
                score("edge_pass", 1.0, true, Some(88.0)),
                score("gross_miss", 0.1, false, Some(90.0)),
            ],
            Vec::new(),
        );
        let payload = build_feedback(&result);
        let names: Vec<&str> = payload
            .tips
            .iter()
            .map(|tip| tip.checkpoint.as_str())
            .collect();
        assert_eq!(
            names,
            ["gross_miss", "minor_miss", "edge_pass", "near_pass"]
        );
    }

    /// The sort is stable, so two checkpoints with the same key keep `CHECKPOINT_REGISTRY` order —
    /// the order `checkpoint_scores` arrives in. Every passing checkpoint scores exactly 1.0, so a
    /// pair with no percentile is the real case this covers.
    #[test]
    fn a_tie_keeps_the_order_the_scores_arrived_in() {
        let result = a_result(
            vec![
                score("second_registered", 1.0, true, None),
                score("first_registered", 1.0, true, None),
            ],
            Vec::new(),
        );
        let payload = build_feedback(&result);
        let names: Vec<&str> = payload
            .tips
            .iter()
            .map(|tip| tip.checkpoint.as_str())
            .collect();
        assert_eq!(names, ["second_registered", "first_registered"]);
    }

    #[test]
    fn severity_is_read_off_the_score_and_not_the_percentile() {
        // Both miss at the rail; only `score` separates a near miss from a gross one.
        assert_eq!(
            severity_for(&score("a", 0.5, false, Some(90.0))),
            Severity::Minor
        );
        assert_eq!(
            severity_for(&score("a", 0.49, false, Some(90.0))),
            Severity::Major
        );
        assert_eq!(
            severity_for(&score("a", 1.0, true, Some(90.0))),
            Severity::Info
        );
    }

    /// A one-sided metric at the low rail is *excellent* and sits at the median distance; a two-sided
    /// one is as wrong there as at the high rail. This is the whole reason the flag exists.
    #[test]
    fn the_two_metric_shapes_read_the_same_percentile_differently() {
        let mut one_sided = score("low", 1.0, true, Some(10.0));
        assert_eq!(tail_distance(&one_sided), Some(0.0));
        one_sided.one_sided = false;
        assert_eq!(tail_distance(&one_sided), Some(1.0));
        // And an unresolved percentile is not a typical placement.
        assert_eq!(tail_distance(&score("none", 1.0, true, None)), None);
    }

    #[test]
    fn the_headline_names_the_worst_failure() {
        let result = a_result(
            vec![
                score("minor_miss", 0.8, false, Some(90.0)),
                score("gross_miss", 0.1, false, Some(90.0)),
            ],
            Vec::new(),
        );
        assert_eq!(
            build_feedback(&result).headline.as_deref(),
            Some("Work on gross miss first. gross_miss says something.")
        );
    }

    /// A clean bill names the closest call only when it is past the watch threshold, and the count in
    /// the sentence is the number of checkpoints that scored.
    #[test]
    fn a_clean_bill_names_the_closest_call_only_when_it_is_close() {
        let comfortable = a_result(
            vec![
                score("a", 1.0, true, Some(50.0)),
                score("b", 1.0, true, Some(60.0)),
            ],
            Vec::new(),
        );
        assert_eq!(
            build_feedback(&comfortable).headline.as_deref(),
            Some("All 2 checkpoints are inside tour range.")
        );

        // p70 on a one-sided metric is a tail distance of exactly 0.5 — the threshold, inclusive.
        let edgy = a_result(
            vec![
                score("a", 1.0, true, Some(50.0)),
                score("hip_sway", 1.0, true, Some(70.0)),
            ],
            Vec::new(),
        );
        assert_eq!(
            build_feedback(&edgy).headline.as_deref(),
            Some("All 2 checkpoints are inside tour range. Closest to the edge: hip sway.")
        );
    }

    /// P4's fourth edge: on a tie the **first** checkpoint is named, not the last. `max_by` here
    /// would produce a correct sentence about the wrong checkpoint.
    #[test]
    fn the_closest_call_tie_goes_to_the_first_checkpoint() {
        let result = a_result(
            vec![
                score("first_at_the_edge", 1.0, true, Some(90.0)),
                score("second_at_the_edge", 1.0, true, Some(90.0)),
            ],
            Vec::new(),
        );
        assert!(build_feedback(&result)
            .headline
            .unwrap()
            .ends_with("Closest to the edge: first at the edge."));
    }

    /// Nothing scored is "no verdict", which a UI renders differently from "everything is fine".
    #[test]
    fn nothing_scored_has_no_headline_rather_than_a_clean_bill() {
        let result = a_result(
            Vec::new(),
            vec![unscored_for("tempo", UnscoredReason::NoHandedness)],
        );
        let payload = build_feedback(&result);
        assert_eq!(payload.headline, None);
        // The refusal still becomes a tip.
        assert_eq!(payload.tips.len(), 1);
    }

    /// The remedy is looked up by cause, and the name is `str.capitalize()`d — first character up,
    /// the rest down.
    #[test]
    fn an_unscored_checkpoint_says_why_in_the_words_the_reason_carries() {
        let result = a_result(
            Vec::new(),
            vec![unscored_for(
                "head_stays_back",
                UnscoredReason::NoHandedness,
            )],
        );
        let tip = &build_feedback(&result).tips[0];
        assert_eq!(tip.severity, Severity::Info);
        assert!(tip.text.starts_with(
            "Head stays back could not be scored on this swing, so it is not included in the score."
        ));
        assert!(tip
            .text
            .ends_with(contracts::unscored::spec_for(UnscoredReason::NoHandedness).remedy));
    }

    /// `capitalize` lowercases the tail, which no shipped checkpoint name can tell apart and a
    /// future one could.
    #[test]
    fn capitalize_lowercases_everything_after_the_first_character() {
        assert_eq!(capitalize("head stays back"), "Head stays back");
        assert_eq!(capitalize("hip ROM"), "Hip rom");
        assert_eq!(capitalize(""), "");
    }

    /// A flight refusal rides `unscored` and must not become a tip: the sentence would claim it was
    /// excluded from a score it was never in.
    #[test]
    fn a_flight_refusal_is_reported_but_never_becomes_a_tip() {
        let result = a_result(
            vec![score("tempo", 1.0, true, Some(50.0))],
            vec![
                unscored_for("flight_carry_yds", UnscoredReason::NoClubLoft),
                unscored_for("head_stays_back", UnscoredReason::NoHandedness),
            ],
        );
        let payload = build_feedback(&result);
        let names: Vec<&str> = payload
            .tips
            .iter()
            .map(|tip| tip.checkpoint.as_str())
            .collect();
        assert_eq!(names, ["tempo", "head_stays_back"]);
    }

    /// The fallback sentence, which every committed vector misses because every scored checkpoint
    /// carries a `message`.
    #[test]
    fn a_checkpoint_with_no_message_gets_a_score_out_of_a_hundred() {
        let mut bare = score("mystery", 0.611_111_111_111_111, false, None);
        bare.message = String::new();
        let result = a_result(vec![bare], Vec::new());
        assert_eq!(build_feedback(&result).tips[0].text, "mystery: score 61%.");
    }
}
