//! Mean and spread together, read as evidence about a cause. `analysis/dispersion.py`.
//! [Career mode step 5; M36 P12]
//!
//! [`crate::baseline`] answers *where does this golfer sit and how tightly*. This answers *and what
//! is that shape evidence for*: which family of cause is worth investigating, given that the causes
//! themselves are invisible to this instrument.
//!
//! # Nothing here reads a raw value to decide a finding
//!
//! Every statistic comes from the [`MetricBaseline`] step 4 already guarded, so when the CENTER
//! claim was refused there is no mean to test a bias against: not a mean this module declines to
//! use, a mean that does not exist in its input. `Measurement` made a measurement structurally
//! incapable of reading as a verdict, step 4 made an under-powered statistic structurally absent,
//! and this keeps that property by *consuming* the guarded shape rather than re-deriving from
//! [`pooled_samples`] and remembering to check.
//!
//! The one place raw samples are read is [`within_session_sd`], which needs the per-session grouping
//! the baseline flattens away, and it is itself gated on SPREAD being ready, so it cannot become a
//! route around the seal.
//!
//! # Two findings rather than a score
//!
//! A bias and a scatter are separate facts about a golfer and the pair is the informative thing: the
//! same 6-degree average miss means opposite things at sd 1 and at sd 9. Collapsing them into one
//! number would throw away exactly the contrast the milestone exists to read.
//!
//! Pure functions over `contracts`, and no [`crate::benchmarks`]: comparing a golfer to the tour
//! population lives one layer out in [`crate::comparison`], so that this cannot quietly become a
//! change to how a swing is scored (ADR-010 §2). `comparison`'s tests pin that by reading this
//! file's code lines, as `tests/analysis/test_comparison.py` parses this module's imports.
//!
//! # The arithmetic is CPython's, and one sum is not `sum()`
//!
//! [`within_session_sd`] squares each session's sd and takes the root of the pooled variance with
//! `**`, which is C `pow` ([`crate::stats::pow`], M36 P11's measurement), so neither is a product
//! nor `sqrt`. It accumulates with a plain `+=` where `mean_and_sd` calls `sum()`, so here the fold
//! is a plain fold and **not** [`pyfmt::sum`]. And it walks the sessions in the order each was first
//! met, as the Python dict does, because that order is the order the floats are added in.

use std::collections::HashMap;

use contracts::baseline::{BaselineClaim, MetricBaseline, MetricSample};
use contracts::career::CareerCorpus;
use contracts::dispersion::{
    target_for, DispersionPattern, Finding, GolferDispersion, MetricDispersion,
    SCATTER_ONLY_READING, SESSION_DRIFT_FACTOR,
};

use crate::baseline::{build_baseline, pooled_samples};
use crate::stats::{mean_and_sd, pow};

/// Every metric's miss-shape for one golfer, or the refusal standing in for it.
///
/// Builds the baseline itself rather than taking one, so the guarded statistics and the raw
/// per-session samples provably describe the same corpus. Handing in a baseline built from a
/// different read would be a way to pair one golfer's spread with another's sessions.
pub fn build_dispersion(corpus: &CareerCorpus) -> GolferDispersion {
    let baseline = build_baseline(corpus);
    let samples = pooled_samples(corpus);

    GolferDispersion {
        player_id: corpus.player_id.clone(),
        // `baseline.metrics` is a `BTreeMap`, so this walk is Python's `sorted(...items())`.
        metrics: baseline
            .metrics
            .iter()
            .map(|(name, metric)| {
                let metric_samples = samples.get(name).map_or(&[][..], Vec::as_slice);
                (name.clone(), dispersion_for(metric, metric_samples))
            })
            .collect(),
        built_from_swings: baseline.built_from_swings,
        built_from_sessions: baseline.built_from_sessions,
    }
}

/// One metric's two findings and, when both were answerable, the pattern they make.
///
/// `samples` is used for one thing only, separating within-session scatter from drift between
/// sessions; everything that decides a finding comes from `baseline`. Python's default of `()` is
/// `&[]` here.
pub fn dispersion_for(baseline: &MetricBaseline, samples: &[MetricSample]) -> MetricDispersion {
    let declared = target_for(&baseline.name);

    let mut dispersion = MetricDispersion {
        name: baseline.name.clone(),
        unit: baseline.unit.clone(),
        source: baseline.source.clone(),
        n: baseline.n,
        n_sessions: baseline.n_sessions,
        target: declared.and_then(|row| row.target),
        tolerance: declared.map(|row| row.tolerance),
        bias: Finding::default(),
        center: None,
        center_ci: None,
        offset: None,
        scatter: Finding::default(),
        sd: None,
        sd_ci: None,
        within_session_sd: None,
        pattern: None,
        points_at: None,
        caveats: Vec::new(),
        withheld: Vec::new(),
        unavailable: Vec::new(),
    };

    let Some(declared) = declared else {
        // No tolerance means neither question can be asked: a finding would be measured against a
        // borrowed error term, which is how a metric acquires a verdict nobody derived for it.
        dispersion.unavailable.push(format!(
            "{} is not registered in METRIC_TARGETS, so there is no measurement error to judge \
             either a bias or a spread against",
            baseline.name
        ));
        return dispersion;
    };

    decide_bias(
        &mut dispersion,
        baseline,
        declared.target,
        declared.tolerance,
    );
    decide_scatter(&mut dispersion, baseline, declared.tolerance);

    if declared.target.is_none() {
        dispersion.unavailable.push(format!(
            "no target for {}: {}",
            baseline.name, declared.no_target_reason
        ));
    }

    carry_refusals(&mut dispersion, baseline);
    add_session_evidence(&mut dispersion, baseline, samples);
    read_the_pair(&mut dispersion);
    dispersion
}

// ------------------------------------------------------------------------------------- internals

/// Is the center displaced from the target by more than measurement error explains?
///
/// Established only when the mean's whole 95% interval sits outside `[target ± tolerance]`. An
/// interval that straddles the band is consistent with a real bias *and* with none, so it settles
/// nothing, and saying so is the point: the alternative is a point estimate one sample away from the
/// other side of the line.
fn decide_bias(
    dispersion: &mut MetricDispersion,
    baseline: &MetricBaseline,
    target: Option<f64>,
    tolerance: f64,
) {
    let (Some(target), Some(mean), Some(mean_ci)) = (target, baseline.mean, &baseline.mean_ci)
    else {
        dispersion.bias = Finding::Withheld;
        return;
    };

    dispersion.center = Some(mean);
    dispersion.center_ci = Some(mean_ci.clone());
    dispersion.offset = Some(mean - target);

    let clears = mean_ci.low > target + tolerance || mean_ci.high < target - tolerance;
    dispersion.bias = if clears {
        Finding::Established
    } else {
        Finding::NotEstablished
    };
}

/// Is the spread larger than measurement error explains?
///
/// The **lower** bound of the sd interval is what has to clear the tolerance, not the estimate.
/// Measurement error inflates observed spread (`sd_obs² ≈ sd_true² + sd_err²`), so a sample sd at the
/// tolerance is exactly what a perfectly repeatable golfer measured by this pipeline would produce.
fn decide_scatter(dispersion: &mut MetricDispersion, baseline: &MetricBaseline, tolerance: f64) {
    let (Some(sd), Some(sd_ci)) = (baseline.sd, &baseline.sd_ci) else {
        dispersion.scatter = Finding::Withheld;
        return;
    };

    dispersion.sd = Some(sd);
    dispersion.sd_ci = Some(sd_ci.clone());
    dispersion.scatter = if sd_ci.low > tolerance {
        Finding::Established
    } else {
        Finding::NotEstablished
    };
}

/// Bring across the step-4 refusals behind whichever finding is withheld for want of `n`.
///
/// Only the two claims this step reads: a `Trend` refusal is real but is not why a bias or a scatter
/// is missing here, and listing it would send someone to book a third session to answer a question
/// that needs a longer one. Python's `needed.get(claim) is Finding.WITHHELD` answers `None`, and so
/// false, for `Trend`; the `match` says the same with no lookup to miss.
fn carry_refusals(dispersion: &mut MetricDispersion, baseline: &MetricBaseline) {
    let (bias, scatter) = (dispersion.bias, dispersion.scatter);
    dispersion.withheld = baseline
        .withheld
        .iter()
        .filter(|refusal| {
            let needed = match refusal.claim {
                BaselineClaim::Center => Some(bias),
                BaselineClaim::Spread => Some(scatter),
                BaselineClaim::Trend => None,
            };
            needed == Some(Finding::Withheld)
        })
        .cloned()
        .collect();
}

/// Separate shot-to-shot scatter from drift between occasions, when the data allows it.
///
/// `sd` pools every sample the golfer has, which mixes two quantities: how much one swing varies from
/// the next, and how much the golfer has moved between sessions. The cause reading is about the first
/// ("your release is inconsistent" is a claim about one bay hour), so a pooled spread inflated by
/// drift would point at timing when the truth is that something changed in between.
///
/// Classification stays on the pooled figure. Splitting them properly is a variance decomposition
/// with its own `n` requirements and no corpus to test one against; a caveat naming the possibility
/// is the honest amount to say today. Its two spreads are CPython's `:.3g` ([`pyfmt::general`] at
/// precision 3, the career family's `dispersion-drift` case).
fn add_session_evidence(
    dispersion: &mut MetricDispersion,
    baseline: &MetricBaseline,
    samples: &[MetricSample],
) {
    // SPREAD was refused, so a within-session spread would be a spread statistic reaching the output
    // around the guard rather than through it.
    let Some(sd) = baseline.sd else {
        return;
    };
    let Some(within) = within_session_sd(samples) else {
        return;
    };

    dispersion.within_session_sd = Some(within);
    if sd > within * SESSION_DRIFT_FACTOR {
        dispersion.caveats.push(format!(
            "Pooled spread ({}) runs wider than the within-session spread ({}), so part of it is \
             movement between sessions rather than shot-to-shot scatter. Read the per-session \
             breakdown before calling this a timing problem.",
            pyfmt::general(sd, 3),
            pyfmt::general(within, 3)
        ));
    }
}

/// Pooled within-session standard deviation, or `None` when it cannot be formed.
///
/// Needs at least two sessions carrying at least two samples each: one session cannot show drift
/// between sessions, and a session of one contributes no within-session spread to pool. The module
/// doc says why the order, the `+=` and both `**`s are spelled as they are.
fn within_session_sd(samples: &[MetricSample]) -> Option<f64> {
    // Python's `dict.setdefault`: one group per session, in the order each was first met.
    let mut grouped: Vec<Vec<f64>> = Vec::new();
    let mut index: HashMap<&str, usize> = HashMap::new();
    for sample in samples {
        let slot = *index.entry(sample.session_id.as_str()).or_insert_with(|| {
            grouped.push(Vec::new());
            grouped.len() - 1
        });
        grouped[slot].push(sample.value);
    }

    let usable: Vec<&Vec<f64>> = grouped.iter().filter(|values| values.len() >= 2).collect();
    if usable.len() < 2 {
        return None;
    }

    let mut squares = 0.0;
    let mut degrees: usize = 0;
    for values in usable {
        // Every usable group has two values, so `sd` is always there; Python's `continue` on `None`
        // is kept rather than turned into a panic, since it costs nothing.
        let Some((_, Some(sd))) = mean_and_sd(values) else {
            continue;
        };
        squares += (values.len() - 1) as f64 * pow(sd, 2.0);
        degrees += values.len() - 1;
    }
    if degrees == 0 {
        return None;
    }
    Some(pow(squares / degrees as f64, 0.5))
}

/// Turn the two findings into a pattern, and the pattern into something to check.
///
/// A pattern needs *both* findings answered. `Scattered` asserts that a bias was looked for and not
/// found, which is false when no target existed to look against, so a metric with scatter and no
/// target gets the reading without the pattern rather than a pattern that overstates it.
fn read_the_pair(dispersion: &mut MetricDispersion) {
    let (bias, scatter) = (dispersion.bias, dispersion.scatter);

    if bias == Finding::Withheld || scatter == Finding::Withheld {
        if scatter == Finding::Established {
            dispersion.points_at = Some(SCATTER_ONLY_READING.to_string());
        }
        return;
    }

    let pattern = match (
        bias == Finding::Established,
        scatter == Finding::Established,
    ) {
        (true, true) => DispersionPattern::BiasedAndScattered,
        (true, false) => DispersionPattern::Biased,
        (false, true) => DispersionPattern::Scattered,
        (false, false) => DispersionPattern::NothingEstablished,
    };
    dispersion.pattern = Some(pattern);
    dispersion.points_at = Some(pattern.reading().to_string());
}

#[cfg(test)]
mod tests {
    //! `tests/analysis/test_dispersion.py`, ported case for case. What is tested is the **reading**:
    //! that a repeatable miss and a scattered one come out as different things and, the case that
    //! matters more, that neither comes out at all when the data cannot support it. The assertions
    //! cluster on the boundaries: a bias inside the tolerance, an interval straddling it, a metric
    //! with no target at all.
    //!
    //! Corpora are built from the contracts directly, no disk, as `baseline`'s tests are, so a
    //! discriminator failure can never be mistaken for a reader failure.

    use std::collections::BTreeSet;

    use super::*;
    use contracts::career::CorpusSwing;
    use contracts::dispersion::METRIC_TARGETS;
    use contracts::pivots::pivot_measurement_names;
    use contracts::swing::Measurement;
    use contracts::Timestamp;

    use crate::flight_measure::FLIGHT_MEASUREMENTS;
    use crate::measure::POSE_MEASUREMENTS;
    use crate::shot_measure::SHOT_MEASUREMENTS;

    const POSE: &str = "pose:face_on";
    const LM: &str = "launch_monitor:hd_golf";

    fn measurement(name: &str, value: f64) -> Measurement {
        let lm = name.ends_with("_deg");
        Measurement {
            name: name.into(),
            value,
            unit: if lm { "degrees" } else { "shoulder_widths" }.into(),
            source: if lm { LM } else { POSE }.into(),
            detail: "test".into(),
        }
    }

    /// One swing per value, each with its own clip and shot photo so nothing dedupes away. Python's
    /// `datetime(2026, 8, 1 + (i % 28), 12)`.
    fn corpus_with(metric: &str, values: &[f64], sessions: Option<&[&str]>) -> CareerCorpus {
        let swings = values
            .iter()
            .enumerate()
            .map(|(i, &value)| CorpusSwing {
                player_id: "aaron".into(),
                session_id: sessions.map_or_else(
                    || format!("2026-08-{:02}", i + 1),
                    |sessions| sessions[i].to_string(),
                ),
                swing_id: i.to_string(),
                captured_at: Timestamp::parse(&format!("2026-08-{:02}T12:00:00Z", 1 + i % 28))
                    .expect("a valid timestamp"),
                face_on_sha256: format!("clip-{i}"),
                shot_sha256: Some(format!("photo-{i}")),
                club: None,
                measurements: vec![measurement(metric, value)],
                duplicates: Vec::new(),
                conflicting_shots: Vec::new(),
                analyzed: true,
                stale: false,
                analysis_version: 0,
                outdated: false,
                shot_needs_review: false,
                auto_mishit: false,
                manual_mishit: None,
                missing_roles: Vec::new(),
            })
            .collect();
        empty_corpus("aaron", swings)
    }

    fn corpus(metric: &str, values: &[f64]) -> CareerCorpus {
        corpus_with(metric, values, None)
    }

    fn empty_corpus(player_id: &str, swings: Vec<CorpusSwing>) -> CareerCorpus {
        serde_json::from_value(serde_json::json!({ "player_id": player_id }))
            .map(|empty: CareerCorpus| CareerCorpus { swings, ..empty })
            .expect("an empty corpus parses")
    }

    /// `n` values with mean exactly `center` and a spread set by `half_width`. Alternating rather
    /// than random: a test that can flake on a seed is a test that gets rerun until it passes.
    fn spread(center: f64, half_width: f64, n: usize) -> Vec<f64> {
        (0..n)
            .map(|i| {
                if i % 2 == 0 {
                    center + half_width
                } else {
                    center - half_width
                }
            })
            .collect()
    }

    fn one(metric: &str, values: &[f64]) -> MetricDispersion {
        build_dispersion(&corpus(metric, values)).metrics[metric].clone()
    }

    fn one_in(metric: &str, values: &[f64], sessions: &[&str]) -> MetricDispersion {
        build_dispersion(&corpus_with(metric, values, Some(sessions))).metrics[metric].clone()
    }

    fn close(actual: Option<f64>, expected: f64) {
        let actual = actual.expect("a value");
        assert!((actual - expected).abs() < 1e-9, "{actual} != {expected}");
    }

    fn sessions(groups: &[(&'static str, usize)]) -> Vec<&'static str> {
        groups
            .iter()
            .flat_map(|&(id, n)| std::iter::repeat_n(id, n))
            .collect()
    }

    // --- the refusal ---------------------------------------------------------------------------

    /// Two swings: both findings withheld, no pattern, and no number left to render.
    #[test]
    fn at_the_n_on_disk_nothing_can_even_be_asked() {
        let metric = one("face_to_path_deg", &[10.9, 13.2]);
        assert_eq!(metric.n, 2);
        assert_eq!(
            (metric.bias, metric.scatter),
            (Finding::Withheld, Finding::Withheld)
        );
        assert_eq!((metric.pattern, metric.points_at), (None, None));
        assert_eq!(
            (metric.center, metric.center_ci, metric.offset),
            (None, None, None)
        );
        assert_eq!((metric.sd, metric.sd_ci), (None, None));
    }

    #[test]
    fn a_refusal_names_the_shortfall_it_is_waiting_on() {
        let metric = one("face_to_path_deg", &[10.9, 13.2]);
        let claims: Vec<BaselineClaim> = metric.withheld.iter().map(|r| r.claim).collect();
        assert_eq!(claims, [BaselineClaim::Center, BaselineClaim::Spread]);
        let center = &metric.withheld[0];
        assert_eq!((center.have_n, center.need_n), (2, 5));
    }

    /// It is real, and it is not why either finding is missing: sending someone to book a third
    /// session would be answering a question this step never asked.
    #[test]
    fn a_trend_refusal_is_not_carried_here() {
        let metric = one("face_to_path_deg", &[10.9, 13.2]);
        assert!(metric
            .withheld
            .iter()
            .all(|r| r.claim != BaselineClaim::Trend));
    }

    // --- the four patterns ---------------------------------------------------------------------

    /// The static-cause half: same size every swing, so something fixed is producing it.
    #[test]
    fn a_repeatable_miss_reads_biased() {
        let metric = one("face_to_path_deg", &spread(9.0, 0.5, 12));
        assert_eq!(metric.bias, Finding::Established);
        assert_eq!(metric.scatter, Finding::NotEstablished);
        assert_eq!(metric.pattern, Some(DispersionPattern::Biased));
        close(metric.offset, 9.0);
        assert!(metric
            .points_at
            .unwrap()
            .contains("before the swing starts"));
    }

    /// The timing half: centered on the target, but nowhere near it twice running.
    #[test]
    fn a_miss_that_moves_reads_scattered() {
        let metric = one("face_to_path_deg", &spread(0.0, 10.0, 12));
        assert_eq!(metric.bias, Finding::NotEstablished);
        assert_eq!(metric.scatter, Finding::Established);
        assert_eq!(metric.pattern, Some(DispersionPattern::Scattered));
        assert!(metric.points_at.unwrap().contains("timing"));
    }

    /// They are separable problems, which is the whole reason the findings stay independent.
    #[test]
    fn both_at_once_is_a_pattern_of_its_own() {
        let metric = one("face_to_path_deg", &spread(15.0, 5.0, 12));
        assert_eq!(
            (metric.bias, metric.scatter),
            (Finding::Established, Finding::Established)
        );
        assert_eq!(metric.pattern, Some(DispersionPattern::BiasedAndScattered));
    }

    /// One reading serves every metric, so it may not name checks that fit only one of them. The
    /// first cut said "grip, alignment, ball position, face at address", which printed unchanged
    /// under `head_sway_norm` and sent a golfer to check their grip about a head that moves.
    #[test]
    fn the_reading_names_a_class_of_cause_and_never_a_specific_check() {
        let face = one("face_to_path_deg", &spread(9.0, 0.5, 12));
        let sway = one("head_sway_norm", &spread(0.30, 0.01, 12));
        assert_eq!(face.pattern, Some(DispersionPattern::Biased));
        assert_eq!(sway.pattern, Some(DispersionPattern::Biased));
        assert_eq!(face.points_at, sway.points_at);
        let reading = pyfmt::lower(&face.points_at.unwrap());
        for banned in ["grip", "ball position", "face at address", "head"] {
            assert!(!reading.contains(banned), "{banned}");
        }
    }

    #[test]
    fn nothing_established_is_not_a_clean_bill() {
        let metric = one("face_to_path_deg", &spread(0.3, 0.4, 12));
        assert_eq!(metric.pattern, Some(DispersionPattern::NothingEstablished));
        assert!(metric.points_at.unwrap().contains("not a clean bill"));
    }

    // --- the boundaries ------------------------------------------------------------------------

    /// A mean of 1.5 degrees with a tight interval is real and reproducible, and it sits inside the
    /// 2-degree tolerance: inside what this instrument can distinguish from zero.
    #[test]
    fn a_miss_inside_the_tolerance_is_not_a_miss() {
        let metric = one("face_to_path_deg", &spread(1.5, 0.2, 12));
        close(metric.center, 1.5);
        assert_eq!(metric.bias, Finding::NotEstablished);
        assert_eq!(metric.pattern, Some(DispersionPattern::NothingEstablished));
    }

    /// The underpowered case: the mean is past the tolerance and the interval is not, so the data is
    /// consistent with a real bias *and* with none.
    #[test]
    fn an_interval_straddling_the_tolerance_settles_nothing() {
        let metric = one("face_to_path_deg", &spread(2.5, 2.0, 12));
        close(metric.center, 2.5);
        let ci = metric.center_ci.expect("an interval");
        assert!(ci.low < 2.0 && 2.0 < ci.high, "{ci:?}");
        assert_eq!(metric.bias, Finding::NotEstablished);
    }

    /// The tolerance is a band around the target, not a floor: a closed face is a miss too.
    #[test]
    fn a_negative_bias_is_a_bias() {
        let metric = one("face_to_path_deg", &spread(-9.0, 0.5, 12));
        assert_eq!(metric.bias, Finding::Established);
        close(metric.offset, -9.0);
    }

    /// A sample sd at the tolerance is what a perfectly repeatable golfer measured by this pipeline
    /// would produce, so the lower bound decides.
    #[test]
    fn scatter_is_decided_on_the_lower_bound_not_the_estimate() {
        let metric = one("face_to_path_deg", &spread(0.0, 2.0, 12));
        assert!(metric.sd.unwrap() > 2.0);
        assert!(metric.sd_ci.unwrap().low < 2.0);
        assert_eq!(metric.scatter, Finding::NotEstablished);
    }

    // --- targets -------------------------------------------------------------------------------

    /// Not silently skipped: the scatter finding is real and `unavailable` carries the reason the
    /// other half is missing.
    #[test]
    fn a_metric_with_no_target_gets_scatter_and_never_a_bias() {
        let metric = one("tempo_ratio", &spread(2.4, 2.0, 15));
        assert_eq!(metric.target, None);
        assert_eq!(metric.bias, Finding::Withheld);
        assert_eq!(metric.scatter, Finding::Established);
        assert_eq!(metric.pattern, None, "half a pair is not a pattern");
        assert_eq!(metric.points_at.as_deref(), Some(SCATTER_ONLY_READING));
        assert!(metric.points_at.unwrap().contains("no target"));
        assert!(metric.unavailable.iter().any(|r| r.contains("step 6")));
        let reason = target_for("tempo_ratio").unwrap().no_target_reason;
        assert_eq!(
            metric.unavailable,
            [format!("no target for tempo_ratio: {reason}")]
        );
    }

    /// One notch more conservative than the minimum-N fallback: with no tolerance neither question
    /// can be asked, and the refusal returns before any step-4 refusal is carried.
    #[test]
    fn an_unregistered_metric_is_silent_by_omission() {
        let metric = one("future_metric", &spread(1.0, 0.1, 12));
        assert_eq!(metric.tolerance, None);
        assert_eq!(
            (metric.bias, metric.scatter),
            (Finding::Withheld, Finding::Withheld)
        );
        assert_eq!(
            metric.unavailable,
            [
                "future_metric is not registered in METRIC_TARGETS, so there is no measurement \
              error to judge either a bias or a spread against"
            ]
        );
        assert!(metric.withheld.is_empty() && metric.points_at.is_none());
    }

    fn names<'a>(registry: impl IntoIterator<Item = &'a str>) -> BTreeSet<&'a str> {
        registry.into_iter().collect()
    }

    fn targets() -> BTreeSet<&'static str> {
        names(METRIC_TARGETS.iter().map(|row| row.metric))
    }

    fn produced() -> BTreeSet<&'static str> {
        names(
            POSE_MEASUREMENTS
                .iter()
                .map(|(name, _)| *name)
                .chain(SHOT_MEASUREMENTS.iter().map(|(name, _)| *name)),
        )
    }

    /// The pin that catches a metric added to `measure` and nowhere else. It needs `analysis`'s
    /// registries, so it landed here rather than with the table in `contracts` (M36 P7's finding).
    #[test]
    fn every_production_metric_has_a_tolerance() {
        assert_eq!(produced(), targets());
    }

    /// M15 P11's answer to the pin above, made explicit: a scatter finding on `flight_carry_yds`
    /// would re-report `carry_distance_yds`'s spread with a model's error folded in, under a second
    /// name, and `flight_spin_rpm`'s error floor is the solver's, not an instrument's. A stated
    /// refusal, so adding a row is a decision someone makes on purpose.
    #[test]
    fn the_simulated_flight_is_registered_nowhere_and_that_is_the_choice() {
        let flight = names(FLIGHT_MEASUREMENTS.iter().map(|(name, _)| *name));
        assert!(flight.is_disjoint(&targets()));
        assert!(flight.is_disjoint(&produced()));

        let metric = one("flight_carry_yds", &spread(150.0, 3.0, 12));
        assert_eq!(metric.tolerance, None);
        assert_eq!(
            (metric.bias, metric.scatter),
            (Finding::Withheld, Finding::Withheld)
        );
        assert!(metric
            .unavailable
            .iter()
            .any(|r| r.contains("METRIC_TARGETS")));
    }

    /// M17 P5's answer, for a different reason: the image plane is uncalibrated, so a scatter over
    /// two sessions would be reading where the phone stood. The membership half has the teeth, since
    /// a pivot name drifting into `POSE_MEASUREMENTS` would inherit a tolerance nobody chose.
    #[test]
    fn the_pivot_family_is_registered_nowhere_either_and_that_is_the_choice() {
        let pivots = names(pivot_measurement_names());
        assert!(pivots.is_disjoint(&targets()));
        assert!(pivots.is_disjoint(&produced()));
        assert!(pivots.is_disjoint(&names(FLIGHT_MEASUREMENTS.iter().map(|(name, _)| *name))));

        let metric = one("pivot_hip_axis_drift_norm", &spread(0.30, 0.02, 12));
        assert_eq!(metric.tolerance, None);
        assert_eq!(
            (metric.bias, metric.scatter),
            (Finding::Withheld, Finding::Withheld)
        );
        assert!(metric
            .unavailable
            .iter()
            .any(|r| r.contains("METRIC_TARGETS")));
    }

    // `test_a_target_less_metric_must_say_why` is `contracts::dispersion`'s, with the table.

    // --- the inherited guard -------------------------------------------------------------------

    /// No new thresholds: the two findings gate on the two step-4 claims, unchanged.
    #[test]
    fn bias_opens_at_centers_floor_and_scatter_at_spreads() {
        let metric = one("head_sway_norm", &spread(0.30, 0.02, 6));
        assert_eq!(metric.n, 6);
        assert_eq!(metric.bias, Finding::Established);
        assert_eq!(metric.scatter, Finding::Withheld);
        assert_eq!((metric.pattern, metric.sd), (None, None));
        let claims: Vec<BaselineClaim> = metric.withheld.iter().map(|r| r.claim).collect();
        assert_eq!(claims, [BaselineClaim::Spread]);
    }

    /// `tempo_ratio`'s per-metric floors still apply, because it is genuinely the same guard.
    #[test]
    fn a_noisier_metric_stays_silent_where_a_cleaner_one_speaks() {
        let tempo = one("tempo_ratio", &spread(2.4, 0.2, 12));
        let sway = one("head_sway_norm", &spread(0.30, 0.02, 12));
        assert_eq!(
            tempo.scatter,
            Finding::Withheld,
            "tempo's SPREAD floor is 15, not 10"
        );
        assert_ne!(sway.scatter, Finding::Withheld);
    }

    /// The structural property: this module consumes guarded statistics rather than raw values, so a
    /// sealed mean is absent from its input, not merely ignored by it.
    #[test]
    fn a_withheld_center_leaves_no_number_for_a_bias_to_be_computed_from() {
        let baseline = build_baseline(&corpus("head_sway_norm", &spread(0.30, 0.02, 3)));
        let guarded = &baseline.metrics["head_sway_norm"];
        let metric = dispersion_for(guarded, &[]);
        assert_eq!(guarded.mean, None);
        assert_eq!(metric.bias, Finding::Withheld);
        assert_eq!(metric.center, None);
    }

    // --- sessions ------------------------------------------------------------------------------

    /// Two tight sessions in different places pool into one wide spread. Read naively that is a
    /// timing problem; it is a golfer who changed something in between.
    #[test]
    fn spread_inflated_by_drift_between_sessions_says_so() {
        let mut values = spread(0.20, 0.01, 6);
        values.extend(spread(0.50, 0.01, 6));
        let metric = one_in(
            "head_sway_norm",
            &values,
            &sessions(&[("2026-08-01", 6), ("2026-08-08", 6)]),
        );
        assert_eq!(metric.n_sessions, 2);
        let within = metric.within_session_sd.expect("a within-session spread");
        assert!(metric.sd.unwrap() > within);
        assert!(metric
            .caveats
            .iter()
            .any(|c| c.contains("between sessions")));
        // The sentence in full, with both spreads at `:.3g`.
        assert_eq!(
            metric.caveats,
            [format!(
                "Pooled spread ({}) runs wider than the within-session spread ({}), so part of it \
                 is movement between sessions rather than shot-to-shot scatter. Read the \
                 per-session breakdown before calling this a timing problem.",
                pyfmt::general(metric.sd.unwrap(), 3),
                pyfmt::general(within, 3)
            )]
        );
        assert!(metric.caveats[0].starts_with(
            "Pooled spread (0.157) runs wider than the \
                                               within-session spread (0.011)"
        ));
    }

    #[test]
    fn sessions_that_agree_raise_no_caveat() {
        let metric = one_in(
            "head_sway_norm",
            &spread(0.30, 0.10, 12),
            &sessions(&[("2026-08-01", 6), ("2026-08-08", 6)]),
        );
        assert!(metric.within_session_sd.is_some());
        assert!(metric.caveats.is_empty());
    }

    #[test]
    fn one_session_cannot_show_drift_between_sessions() {
        let metric = one_in(
            "head_sway_norm",
            &spread(0.30, 0.10, 12),
            &sessions(&[("2026-08-01", 12)]),
        );
        assert_eq!(metric.n_sessions, 1);
        assert_eq!(metric.within_session_sd, None);
        assert!(metric.caveats.is_empty());
    }

    /// A spread statistic must not reach the output while SPREAD is refused: that would be a number
    /// arriving around the guard rather than through it.
    #[test]
    fn the_within_session_spread_is_gated_with_the_pooled_one() {
        let mut values = spread(0.20, 0.01, 3);
        values.extend(spread(0.50, 0.01, 3));
        let metric = one_in(
            "head_sway_norm",
            &values,
            &sessions(&[("2026-08-01", 3), ("2026-08-08", 3)]),
        );
        assert_eq!(metric.scatter, Finding::Withheld);
        assert_eq!(metric.within_session_sd, None);
    }

    /// The groups are added in first-seen order, and a session of one is not pooled. Worked by hand:
    /// sessions `b` (0.1, 0.3), `a` (0.2, 0.6, 1.0) and `c` (5.0) pool as `(1·0.02 + 2·0.16) / 3`.
    #[test]
    fn the_pooled_spread_skips_a_session_of_one_and_weights_by_degrees() {
        let sample = |session: &str, value: f64| MetricSample {
            metric: "m".into(),
            value,
            unit: "u".into(),
            source: POSE.into(),
            session_id: session.into(),
            captured_at: Timestamp::parse("2026-08-01T12:00:00Z").unwrap(),
            artifact_key: format!("{session}-{value}"),
            swing_ref: format!("{session}/{value}"),
        };
        let samples = [
            sample("b", 0.1),
            sample("a", 0.2),
            sample("c", 5.0),
            sample("b", 0.3),
            sample("a", 0.6),
            sample("a", 1.0),
        ];
        let pooled = within_session_sd(&samples).expect("two usable sessions");
        assert!((pooled - (0.34_f64 / 3.0).sqrt()).abs() < 1e-12, "{pooled}");
        assert_eq!(within_session_sd(&samples[..4]), None, "one usable session");
        assert_eq!(within_session_sd(&[]), None);
    }

    /// **CPython's own answers, compared with no tolerance**, on the target they were recorded on:
    /// `dispersion.py::_within_session_sd` under CPython 3.13.3 on x86_64 Windows (M36 P12, a scratch
    /// script, deleted). The career family is blind to both `**`s in [`within_session_sd`]: at P12,
    /// `sqrt` for `** 0.5` and the product for `sd ** 2` each passed the gate and the bit pin, so
    /// this is their negative control, as `stats`' is for its own `pow`. The first three rows are
    /// ones the product `sd * sd` answers differently and the last three ones `sqrt` does (the
    /// UCRT's `pow(x, 0.5)` differed from `sqrt` on 524 of 1,000,000 random `x`), and the test
    /// recomputes each alternative to prove the row still separates them. Held to the target for
    /// `stats`' reason: another libm answers `pow`'s last bit its own way, and CPython over it would.
    #[cfg(all(target_os = "windows", target_env = "msvc"))]
    #[test]
    fn the_pooled_spread_is_cpythons_to_the_bit() {
        type Alternative = (fn(f64) -> f64, fn(f64) -> f64);
        let rows: [(&[f64], &[f64], f64, &str); 6] = [
            (
                &[0.46, 0.42, 0.23, 0.28],
                &[0.38, 0.26],
                0.10425329730996521,
                "product",
            ),
            (
                &[0.12, 0.31, 0.41, 0.23],
                &[0.34, 0.26],
                0.11008519428151997,
                "product",
            ),
            (
                &[0.25, 0.38, 0.13],
                &[0.15, 0.31, 0.59],
                0.1806008490197836,
                "product",
            ),
            (
                &[0.37, 0.3],
                &[0.6, 0.18, 0.13, 0.34],
                0.18488171894484318,
                "sqrt",
            ),
            (
                &[0.48, 0.49],
                &[0.39, 0.14, 0.21, 0.44],
                0.12384466076500836,
                "sqrt",
            ),
            (
                &[0.17, 0.46, 0.33, 0.15],
                &[0.13, 0.22, 0.19],
                0.11668333214302716,
                "sqrt",
            ),
        ];
        // `(square, root)` as the obvious port would spell them.
        let alternatives: [(&str, Alternative); 2] = [
            ("product", (|sd| sd * sd, |v| pow(v, 0.5))),
            ("sqrt", (|sd| pow(sd, 2.0), f64::sqrt)),
        ];
        for (a, b, cpython, separates) in rows {
            let mut samples = Vec::new();
            for (session, values) in [("a", a), ("b", b)] {
                for &value in values {
                    samples.push(MetricSample {
                        metric: "m".into(),
                        value,
                        unit: "u".into(),
                        source: POSE.into(),
                        session_id: session.into(),
                        captured_at: Timestamp::parse("2026-08-01T12:00:00Z").unwrap(),
                        artifact_key: format!("{session}-{value}"),
                        swing_ref: format!("{session}/{value}"),
                    });
                }
            }
            let built = within_session_sd(&samples).expect("two usable sessions");
            assert_eq!(built.to_bits(), cpython.to_bits(), "{a:?} {b:?}");

            let (_, (square, root)) = alternatives
                .iter()
                .find(|(name, _)| *name == separates)
                .unwrap();
            let (mut squares, mut degrees) = (0.0, 0);
            for values in [a, b] {
                let sd = mean_and_sd(values).unwrap().1.unwrap();
                squares += (values.len() - 1) as f64 * square(sd);
                degrees += values.len() - 1;
            }
            assert_ne!(
                root(squares / degrees as f64).to_bits(),
                cpython.to_bits(),
                "{a:?} {b:?}: no longer a control for the {separates}"
            );
        }
    }

    // --- assembly ------------------------------------------------------------------------------

    /// Three readers of one corpus, and the `n` beside a pattern has to be the `n` behind it.
    #[test]
    fn n_agrees_with_the_baseline_for_every_metric() {
        let corpus = corpus("head_sway_norm", &spread(0.30, 0.02, 7));
        let baseline = build_baseline(&corpus);
        let dispersion = build_dispersion(&corpus);
        assert_eq!(
            dispersion.metrics.keys().collect::<Vec<_>>(),
            baseline.metrics.keys().collect::<Vec<_>>()
        );
        for (name, metric) in &dispersion.metrics {
            assert_eq!(metric.n, baseline.metrics[name].n);
            assert_eq!(metric.n_sessions, baseline.metrics[name].n_sessions);
            assert_eq!(
                corpus.metric_counts.get(name).copied().unwrap_or(metric.n),
                metric.n
            );
        }
    }

    /// The variance-collapsing failure, reaching the thing it would have corrupted: twelve copies of
    /// one swing would read as a golfer of impossible consistency.
    #[test]
    fn re_uploads_of_one_clip_cannot_manufacture_a_pattern() {
        let mut corpus = corpus("head_sway_norm", &spread(0.30, 0.02, 12));
        for swing in &mut corpus.swings {
            swing.face_on_sha256 = "one-clip".into();
        }
        let metric = &build_dispersion(&corpus).metrics["head_sway_norm"];
        assert_eq!(metric.n, 1);
        assert_eq!(metric.pattern, None);
    }

    #[test]
    fn an_empty_corpus_is_an_empty_reading_not_an_error() {
        let dispersion = build_dispersion(&empty_corpus("nobody", Vec::new()));
        assert!(dispersion.metrics.is_empty());
        assert!(dispersion.nothing_established());
        assert_eq!(dispersion.patterns_established(), 0);
        assert_eq!(dispersion.player_id, "nobody");
    }

    #[test]
    fn metrics_are_sorted_so_two_runs_agree() {
        let mut corpus = corpus("head_sway_norm", &spread(0.30, 0.02, 4));
        for swing in &mut corpus.swings {
            swing
                .measurements
                .push(measurement("face_to_path_deg", 9.0));
        }
        let names: Vec<String> = build_dispersion(&corpus).metrics.into_keys().collect();
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(names, sorted);
        assert_eq!(names, ["face_to_path_deg", "head_sway_norm"]);
    }

    #[test]
    fn provenance_survives_onto_the_reading() {
        let metric = one("face_to_path_deg", &spread(9.0, 0.5, 12));
        assert_eq!(metric.unit, "degrees");
        assert_eq!(metric.source, LM);
    }
}
