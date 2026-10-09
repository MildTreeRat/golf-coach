//! A personal center, placed in the tour population. `analysis/comparison.py`.
//! [Career mode step 6; M36 P12]
//!
//! [`crate::baseline`] answers *where does this golfer sit*, [`crate::dispersion`] answers *what is
//! that shape evidence for*, and this answers the third question, the one everything else here could
//! already ask about a single swing: *and how does that compare to the tour*.
//!
//! # A module of its own because of what it reaches
//!
//! `baseline` and `dispersion` reach no [`crate::benchmarks`], and that is not stylistic: it is what
//! stops a personal statistic from becoming a change to how a swing is scored (ADR-010 §2). The join
//! has to reach [`crate::benchmarks::distributions`] by definition, so it lands here and the boundary
//! moves out by one layer instead of dissolving. The tests below hold the other two to it by reading
//! their code lines, as `tests/analysis/test_comparison.py` parses their imports.
//!
//! # It reads the guarded baseline, never the samples
//!
//! Same posture as `dispersion`: when step 4 refused CENTER there is no mean *in the input* to place.
//! Not a mean this module declines to use, a mean that does not exist in what it was handed. That is
//! how "withheld means absent" survives being consumed one more time.
//!
//! # The stratum is the one the bands were cut from
//!
//! [`load_distribution_any`], the `("all", "all", "all")` defaults, exactly as
//! `checkpoints::mechanics` places a swing and for the reason recorded there: `tempo_ratio`'s face-on
//! stratum has p90 5.00 against the all-view 4.71, so a narrower stratum than the band's would let
//! one number read "inside the band" and "past the 90th percentile" at once. Moving to per-club
//! strata is a later change that has to move the band and the placement together.
//!
//! `round(placement, 1)` is [`pyfmt::round_to`], CPython's correctly rounded `round`, because the
//! percentile is printed.

use contracts::baseline::{BaselineClaim, Interval, MetricBaseline};
use contracts::career::CareerCorpus;
use contracts::comparison::{
    no_population_reason, tour_comparison_blocked, GolferStanding, MetricComparison, Standing,
    SPREAD_NOT_COMPARABLE,
};

use crate::baseline::build_baseline;
use crate::benchmarks::distributions::load_distribution_any;

/// Every metric's placement in the tour population for one golfer, or the refusal for it.
///
/// Builds the baseline itself rather than taking one, for `build_dispersion`'s reason: handing in a
/// baseline read from a different corpus would be a way to place one golfer's center using another
/// golfer's `n`.
pub fn build_standing(corpus: &CareerCorpus) -> GolferStanding {
    let baseline = build_baseline(corpus);

    GolferStanding {
        player_id: corpus.player_id.clone(),
        metrics: baseline
            .metrics
            .iter()
            .map(|(name, metric)| (name.clone(), comparison_for(metric)))
            .collect(),
        built_from_swings: baseline.built_from_swings,
        built_from_sessions: baseline.built_from_sessions,
    }
}

/// One metric's placement, or the refusal standing in for it.
///
/// Two kinds of refusal, returning by different routes. A metric with no usable population returns
/// early carrying only that, because no amount of swinging changes it, and printing "needs 5 samples"
/// beside it would send someone to the bay to answer a question that will still be unanswerable. A
/// metric with a population but no center carries the step-4 refusal instead, which *is* actionable.
pub fn comparison_for(baseline: &MetricBaseline) -> MetricComparison {
    let mut comparison = MetricComparison {
        name: baseline.name.clone(),
        unit: baseline.unit.clone(),
        source: baseline.source.clone(),
        n: baseline.n,
        n_sessions: baseline.n_sessions,
        center: None,
        center_ci: None,
        standing: Standing::default(),
        reading: None,
        percentile: None,
        percentile_clamped: false,
        band_low: None,
        band_high: None,
        outside_by: None,
        population_n: None,
        population_players: None,
        withheld: Vec::new(),
        unavailable: Vec::new(),
    };

    if let Some(blocked) = tour_comparison_blocked(&baseline.name) {
        comparison.unavailable.push(format!(
            "{} has a stored distribution and may not be placed in it: {blocked}",
            baseline.name
        ));
        return comparison;
    }

    let Some(distribution) = load_distribution_any(&baseline.name) else {
        comparison
            .unavailable
            .push(no_population_reason(&baseline.name, &baseline.source));
        return comparison;
    };

    comparison.band_low = Some(distribution.p10);
    comparison.band_high = Some(distribution.p90);
    comparison.population_n = Some(distribution.n);
    comparison.population_players = Some(distribution.n_players);

    // Said only once the golfer actually has a spread. Before that the question is not askable and
    // `withheld` already explains why; after it, the absence of an obvious-looking sd-against-sd line
    // is the thing that needs a reason attached.
    if baseline.sd.is_some() {
        comparison.unavailable.push(format!(
            "{}: the spread is not placed against the tour spread — {SPREAD_NOT_COMPARABLE}",
            baseline.name
        ));
    }

    let (Some(mean), Some(mean_ci)) = (baseline.mean, &baseline.mean_ci) else {
        comparison.withheld = baseline
            .withheld
            .iter()
            .filter(|refusal| refusal.claim == BaselineClaim::Center)
            .cloned()
            .collect();
        return comparison;
    };

    comparison.center = Some(mean);
    comparison.center_ci = Some(mean_ci.clone());

    let placement = distribution.percentile_of(mean);
    comparison.percentile = Some(pyfmt::round_to(placement, 1));
    // `percentile_of` clamps at the stored quantiles, so a rail value is a floor on how extreme the
    // center is rather than its rank. Flagged rather than left to the reader: `contracts::swing`
    // records that a bare 90 has been misread as a rank before.
    comparison.percentile_clamped = placement <= 10.0 || placement >= 90.0;

    let standing = place(mean_ci, distribution.p10, distribution.p90);
    comparison.standing = standing;
    // Python's `STANDING_READING[standing]`, which would raise for `Withheld`; `place` never answers
    // it, so the lookup cannot miss and a miss is a bug here, not a fact about a golfer.
    comparison.reading = Some(
        standing
            .reading()
            .expect("place answers a standing with a reading")
            .to_string(),
    );
    if standing == Standing::Outside {
        // Measured from the center rather than from the interval: the interval decided *whether*
        // this is outside, and the size of the gap is a statement about the golfer's typical value.
        comparison.outside_by = Some(if mean > distribution.p90 {
            mean - distribution.p90
        } else {
            mean - distribution.p10
        });
    }
    comparison
}

// ------------------------------------------------------------------------------------- internals

/// Where the *interval* sits relative to `[p10, p90]`.
///
/// Read off the interval and never the point estimate, `dispersion`'s rule for a bias. A center a
/// hair past p90 with an interval that crosses it has not been shown to be outside anything, and the
/// difference between saying so and reporting `Outside` is the difference between a placement that
/// survives the next swing and one that flips on it.
fn place(center_ci: &Interval, band_low: f64, band_high: f64) -> Standing {
    if center_ci.low >= band_low && center_ci.high <= band_high {
        return Standing::Inside;
    }
    if center_ci.high < band_low || center_ci.low > band_high {
        return Standing::Outside;
    }
    Standing::Straddles
}

#[cfg(test)]
mod tests {
    //! `tests/analysis/test_comparison.py`, ported case for case. What is tested is the
    //! **placement**: a center is put in the tour population only when the data supports putting it
    //! anywhere, and the two questions this join must not answer stay unanswered. The tour side is
    //! the real packaged `golfdb_v1.json`, because this module's whole job is agreeing with that
    //! file.

    use super::*;
    use crate::benchmarks::distributions::Distribution;
    use contracts::career::CorpusSwing;
    use contracts::swing::Measurement;
    use contracts::Timestamp;

    const POSE: &str = "pose:face_on";
    const LM: &str = "launch_monitor:hd_golf";

    /// Enough swings over enough sessions to clear the CENTER floor for every metric in the panel:
    /// `tempo_ratio` has the highest, 8.
    const ENOUGH: usize = 12;

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

    /// One swing per value, spread over three sessions, each with its own clip and photo. Python's
    /// `datetime(2026, 9, (i % 3) + 1, 12, i)`.
    fn corpus(metric: &str, values: &[f64]) -> CareerCorpus {
        let swings: Vec<CorpusSwing> = values
            .iter()
            .enumerate()
            .map(|(i, &value)| CorpusSwing {
                player_id: "aaron".into(),
                session_id: format!("2026-09-{:02}", i % 3 + 1),
                swing_id: i.to_string(),
                captured_at: Timestamp::parse(&format!("2026-09-{:02}T12:{i:02}:00Z", i % 3 + 1))
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
        serde_json::from_value(serde_json::json!({"player_id": "aaron"}))
            .map(|empty: CareerCorpus| CareerCorpus { swings, ..empty })
            .expect("an empty corpus parses")
    }

    /// `n` values with mean exactly `center` and an interval narrow enough to decide a side.
    fn tight(center: f64, n: usize, half: f64) -> Vec<f64> {
        (0..n)
            .map(|i| {
                if i % 2 == 0 {
                    center + half
                } else {
                    center - half
                }
            })
            .collect()
    }

    fn tight_default(center: f64) -> Vec<f64> {
        tight(center, ENOUGH, 0.002)
    }

    fn one(metric: &str, values: &[f64]) -> MetricComparison {
        build_standing(&corpus(metric, values)).metrics[metric].clone()
    }

    fn sway() -> &'static Distribution {
        load_distribution_any("head_sway_norm").expect("head_sway_norm ships")
    }

    // --- placing a center ----------------------------------------------------------------------

    #[test]
    fn a_center_inside_the_band_is_placed_inside() {
        let band = sway();
        let metric = one("head_sway_norm", &tight_default(band.p50));
        assert_eq!(metric.standing, Standing::Inside);
        assert_eq!(
            (metric.band_low, metric.band_high),
            (Some(band.p10), Some(band.p90))
        );
        assert_eq!(metric.population_players, Some(band.n_players));
        assert_eq!(metric.population_n, Some(band.n));
        assert_eq!(
            metric.outside_by, None,
            "nothing is past an edge it did not cross"
        );
        assert_eq!(metric.reading.as_deref(), Standing::Inside.reading());
    }

    #[test]
    fn a_center_past_p90_is_placed_above_with_the_distance() {
        let band = sway();
        let metric = one("head_sway_norm", &tight_default(band.p90 + 0.2));
        assert_eq!(metric.standing, Standing::Outside);
        let outside_by = metric.outside_by.expect("a distance");
        assert!(outside_by > 0.0);
        assert!((outside_by - (band.p90 + 0.2 - band.p90)).abs() < 1e-6);
    }

    /// The sign of `outside_by` is the whole difference between "better than tour" and "worse":
    /// `head_sway_norm` is a one-sided magnitude, so a center under p10 is *less* head movement.
    #[test]
    fn a_center_below_p10_is_outside_on_the_good_side() {
        let band = sway();
        let metric = one("head_sway_norm", &tight(band.p10 / 2.0, ENOUGH, 0.001));
        assert_eq!(metric.standing, Standing::Outside);
        assert!(metric.outside_by.expect("a distance") < 0.0);
    }

    /// A center exactly on p90 is one swing away from either side; `Straddles` is unresolved, not
    /// "borderline".
    #[test]
    fn an_interval_crossing_the_edge_settles_nothing() {
        let band = sway();
        let metric = one("head_sway_norm", &tight(band.p90, ENOUGH, 0.05));
        assert_eq!(metric.standing, Standing::Straddles);
        assert_eq!(
            metric.outside_by, None,
            "a distance past an edge asserts a side was established"
        );
    }

    /// A rail value is a floor and not a rank, so it is reported rather than left to be inferred.
    #[test]
    fn the_percentile_reports_its_own_clamp() {
        let band = sway();
        let extreme = one("head_sway_norm", &tight_default(band.p90 + 1.0));
        let middling = one("head_sway_norm", &tight_default(band.p50));
        assert_eq!(extreme.percentile, Some(90.0));
        assert!(extreme.percentile_clamped);
        assert!(middling.percentile.is_some() && !middling.percentile_clamped);
    }

    // --- the two refusals ----------------------------------------------------------------------

    /// The step-4 guard reaching one layer further out, unchanged: the refusal that arrives is the
    /// one step 4 built, not a second copy phrased differently.
    #[test]
    fn below_the_center_floor_nothing_is_placed_and_the_refusal_carries() {
        let metric = one("head_sway_norm", &[0.20, 0.21]);
        assert_eq!(metric.standing, Standing::Withheld);
        assert_eq!((metric.center, metric.percentile), (None, None));
        let claims: Vec<BaselineClaim> = metric.withheld.iter().map(|r| r.claim).collect();
        assert_eq!(claims, [BaselineClaim::Center]);
        assert!(
            metric.band_low.is_some(),
            "the band exists; it is the golfer's n that does not"
        );
    }

    /// Refused on `Measurement.source` rather than on a list of names, so a launch-monitor metric
    /// added tomorrow inherits the reason.
    #[test]
    fn a_launch_monitor_metric_has_no_population_and_says_why() {
        let metric = one("face_to_path_deg", &tight(8.6, ENOUGH, 0.5));
        assert_eq!(metric.standing, Standing::Withheld);
        assert_eq!(metric.band_low, None);
        assert_eq!(metric.unavailable.len(), 1);
        assert!(metric.unavailable[0].contains("no tour population exists"));
        assert!(
            metric.withheld.is_empty(),
            "an n refusal here would send someone to the bay for nothing"
        );
    }

    /// A stored distribution existing is not the same as one meaning something: this metric's sign
    /// is camera-relative and GolfDB mixes both handednesses.
    #[test]
    fn head_hip_offset_is_refused_although_a_distribution_exists() {
        assert!(
            load_distribution_any("head_hip_offset_impact_norm").is_some(),
            "the premise of this test is that the row exists and must not be used"
        );
        let metric = one("head_hip_offset_impact_norm", &tight(0.14, ENOUGH, 0.01));
        assert_eq!(metric.standing, Standing::Withheld);
        assert_eq!(metric.percentile, None);
        assert!(metric
            .unavailable
            .iter()
            .any(|r| r.contains("mixed-handedness")));
        assert_eq!(
            metric.unavailable,
            [format!(
                "head_hip_offset_impact_norm has a stored distribution and may not be placed in \
                 it: {}",
                tour_comparison_blocked("head_hip_offset_impact_norm").unwrap()
            )]
        );
    }

    /// A category error, recorded rather than computed, and said only once the golfer *has* a
    /// spread, since that is when its absence becomes a question worth answering.
    #[test]
    fn the_spread_is_never_placed_against_the_tour_spread() {
        let with_spread = one("head_sway_norm", &tight(0.20, ENOUGH, 0.002));
        let without = one("head_sway_norm", &tight(0.20, 6, 0.002));
        assert!(with_spread
            .unavailable
            .iter()
            .any(|r| r.contains("shot to shot")));
        assert!(!without
            .unavailable
            .iter()
            .any(|r| r.contains("shot to shot")));
        assert_eq!(
            with_spread.unavailable,
            [format!(
                "head_sway_norm: the spread is not placed against the tour spread — \
                 {SPREAD_NOT_COMPARABLE}"
            )]
        );
    }

    #[test]
    fn an_unregistered_metric_gets_the_generic_reason() {
        let metric = one("invented_metric_norm", &tight_default(1.0));
        assert_eq!(metric.standing, Standing::Withheld);
        assert_eq!(
            metric.unavailable,
            ["invented_metric_norm: no reference distribution is stored for it"]
        );
    }

    // --- the whole golfer ----------------------------------------------------------------------

    /// Four layers, one `n`: a placement counted over a different number of swings than the mean it
    /// places would be wrong only where the dedupe rule matters.
    #[test]
    fn the_standing_agrees_with_the_baseline_on_n() {
        let corpus = corpus("head_sway_norm", &tight_default(0.20));
        let baseline = build_baseline(&corpus);
        let standing = build_standing(&corpus);
        for (name, metric) in &baseline.metrics {
            assert_eq!(standing.metrics[name].n, metric.n);
            assert_eq!(standing.metrics[name].n_sessions, metric.n_sessions);
        }
    }

    #[test]
    fn nothing_placed_is_the_state_on_disk() {
        let standing = build_standing(&corpus("head_sway_norm", &[0.20, 0.21]));
        assert!(standing.nothing_placed());
        assert_eq!(standing.placements(), 0);
    }

    /// The signature is the design: a hand-built baseline with a mean but no interval must still
    /// refuse, so there is no path from raw values to a placement for a caller who forgot to check.
    #[test]
    fn comparison_for_reads_the_guarded_baseline_not_the_samples() {
        let mut baseline = build_baseline(&corpus("head_sway_norm", &tight_default(0.20))).metrics
            ["head_sway_norm"]
            .clone();
        baseline.mean_ci = None;
        assert_eq!(comparison_for(&baseline).standing, Standing::Withheld);
    }

    /// The interval's edges are inclusive for `Inside` and strict for `Outside`, as Python's are.
    #[test]
    fn the_band_edges_place_as_pythons_comparisons_do() {
        let ci = |low: f64, high: f64| Interval {
            low,
            high,
            confidence: Interval::DEFAULT_CONFIDENCE,
        };
        assert_eq!(place(&ci(1.0, 2.0), 1.0, 2.0), Standing::Inside);
        assert_eq!(place(&ci(2.0, 2.5), 1.0, 2.0), Standing::Straddles);
        assert_eq!(place(&ci(2.0 + 1e-12, 2.5), 1.0, 2.0), Standing::Outside);
        assert_eq!(place(&ci(0.0, 1.0), 1.0, 2.0), Standing::Straddles);
        assert_eq!(place(&ci(0.0, 1.0 - 1e-12), 1.0, 2.0), Standing::Outside);
    }

    // --- the import boundary -------------------------------------------------------------------

    /// The boundary step 6 moved out by one layer rather than dissolving: `baseline` and
    /// `dispersion` reach no `benchmarks`, so a personal statistic cannot quietly become a change to
    /// how a swing is scored (ADR-010 §2). Read off their source, as the Python's test parses its
    /// imports, and off their **code** lines only: both module docs name `benchmarks` to say they do
    /// not reach it.
    #[test]
    fn the_guarded_modules_still_reach_no_benchmarks() {
        for (module, source) in [
            ("baseline", include_str!("baseline.rs")),
            ("dispersion", include_str!("dispersion.rs")),
        ] {
            let reaching: Vec<&str> = source
                .lines()
                .filter(|line| !line.trim_start().starts_with("//"))
                .filter(|line| line.contains("benchmarks"))
                .collect();
            assert!(
                reaching.is_empty(),
                "analysis::{module} reaches benchmarks: the personal baseline path must not read \
                 tour data (ADR-010 §2), and the join belongs in analysis::comparison: {reaching:?}"
            );
        }
    }
}
