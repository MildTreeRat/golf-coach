//! One golfer's own history, turned into what it is allowed to say. `analysis/baseline.py`.
//! [Career mode step 4; M36 P11]
//!
//! `storage::corpus` answers *how many distinct swings does this golfer have*. This answers *and what
//! does that support claiming*, which, at the `n` on disk today, is nothing at all, and printing
//! nothing is the whole feature.
//!
//! # The pooling has to agree with the counting
//!
//! [`CareerCorpus::metric_counts`] is computed by the reader; the values behind those counts are
//! pooled here. If the two used different rules the printed `n` would not be the number of values
//! averaged under it, and the disagreement would appear only in the cases the rule exists for (a
//! re-uploaded clip, one shot photo across two swings): only where it is invisible and load-bearing
//! at once. So neither side owns the rule. Both call [`CorpusSwing::artifact_key`], and
//! [`build_baseline`] keeps one value per `(metric, artifact key)`. The career family holds the two
//! equal on every recorded corpus, since each vector's `input.corpus` carries the reader's counts and
//! its `expected.baseline` the `n`s built from them.
//!
//! # Where the judgment lives, and where it does not
//!
//! The thresholds are data ([`contracts::baseline::METRIC_MINIMUM_N`]), the statistics are
//! arithmetic ([`crate::stats`]), and what is left here is the assembly. Revising a floor once a bay
//! session lands 20–30 swings should be an edit to a table with a comment, not a change to any logic.
//!
//! Nothing here reaches [`crate::benchmarks::distributions`]. Comparing a personal baseline to the
//! tour band is real, and it lives one layer out in `comparison` (M36 P12), so that this module
//! cannot quietly become a change to how a swing is scored (ADR-010 §2). The Python pins it by
//! parsing this file's imports; here [`crate::comparison`]'s tests read this file's code lines for a
//! `benchmarks` path, and the `use` list below names none.
//!
//! Pure functions over `contracts`, no I/O: the corpus arrives assembled, and this crate depends on
//! `contracts` alone, never on `storage` (ADR-008 as a cargo edge).
//!
//! # Where the port reads differently from the Python, each invisible to every caller
//!
//! - [`pooled_samples`] answers a `BTreeMap`, sorted by metric, where Python's dict is in the order
//!   each metric was first seen. Nothing reads that order: [`build_baseline`] sorts the names, and
//!   `dispersion` (P12) looks a metric up by name.
//! - `min` and `max` are spelled out as Python's, the **first** extreme in sample order, rather than
//!   `f64::min`, which documents either of `0.0` and `-0.0` as a correct answer for the pair. The two
//!   compare equal and a report prints them differently, so the shorter spelling would be faithful
//!   only by a target's accident.
//! - `n` is a `len()` here and an `i64` in [`WithheldClaim`] and [`MetricBaseline`] (P6's finding:
//!   every Python `int` field is `i64`), so the cast happens once, where a count becomes a field.

use std::collections::{BTreeMap, HashMap, HashSet};

use contracts::baseline::{
    minimum_n, minimum_sessions, BaselineClaim, Interval, MetricBaseline, MetricSample,
    PersonalBaseline, SessionSample, WithheldClaim,
};
use contracts::career::{CareerCorpus, CorpusSwing};

use crate::stats::{mean_and_sd, mean_ci, percentile, sd_ci};

/// metric → the deduplicated values behind its `n`, oldest first.
///
/// Public because `dispersion` (P12) needs the values themselves rather than the guarded summary of
/// them, and should not have to re-derive the dedupe rule to get at them. The guard governs what a
/// [`PersonalBaseline`] *asserts*; it is not an attempt to keep the numbers away from the rest of the
/// codebase, which would be security theatre against ourselves.
pub fn pooled_samples(corpus: &CareerCorpus) -> BTreeMap<String, Vec<MetricSample>> {
    let mut samples: BTreeMap<String, Vec<MetricSample>> = BTreeMap::new();
    let mut seen: HashSet<(String, String)> = HashSet::new();

    // `corpus.swings` is already oldest-`captured_at` first, so first-wins keeps the earliest
    // reading of a repeated artifact: the same survivor the reader chose when it collapsed the
    // re-uploads, rather than a second, differently chosen one.
    for swing in corpus.swings.iter().filter(|s| s.counts_toward_metrics()) {
        for measurement in &swing.measurements {
            let Some(key) = swing.artifact_key(measurement) else {
                continue;
            };
            if !seen.insert((measurement.name.clone(), key.clone())) {
                continue;
            }
            samples
                .entry(measurement.name.clone())
                .or_default()
                .push(sample(swing, measurement, key));
        }
    }
    samples
}

fn sample(
    swing: &CorpusSwing,
    measurement: &contracts::swing::Measurement,
    key: String,
) -> MetricSample {
    MetricSample {
        metric: measurement.name.clone(),
        value: measurement.value,
        unit: measurement.unit.clone(),
        source: measurement.source.clone(),
        session_id: swing.session_id.clone(),
        captured_at: swing.captured_at,
        artifact_key: key,
        swing_ref: swing.swing_ref(),
    }
}

/// Everything this golfer's history supports saying about them, and every refusal.
///
/// An empty corpus is an empty baseline rather than an error: "this golfer has no swings yet" is the
/// first true answer about every golfer, and `read_corpus` takes the same posture.
pub fn build_baseline(corpus: &CareerCorpus) -> PersonalBaseline {
    let samples = pooled_samples(corpus);

    let every = || samples.values().flatten();
    let contributing_swings: HashSet<&str> = every().map(|s| s.swing_ref.as_str()).collect();
    let contributing_sessions: HashSet<&str> = every().map(|s| s.session_id.as_str()).collect();

    PersonalBaseline {
        player_id: corpus.player_id.clone(),
        metrics: samples
            .iter()
            .map(|(name, metric_samples)| (name.clone(), baseline_for(name, metric_samples)))
            .collect(),
        built_from_swings: count(contributing_swings.len()),
        built_from_sessions: count(contributing_sessions.len()),
    }
}

/// The guard. `None` means the claim may be made.
///
/// Public because `dispersion` gates on the same claims and must produce the same refusals. A second
/// copy would be step 4's own lesson repeating: the dedupe rule was private until pooling nearly
/// disagreed with counting, and a guard that disagrees with itself fails in the same invisible way,
/// two floors drifting apart with the looser one deciding what gets said.
///
/// `n` and `n_sessions` are `i64` because that is what a [`WithheldClaim`] carries and what
/// [`minimum_n`] answers in; a caller holding a `len()` casts once, as [`build_baseline`] does.
pub fn refuse(
    metric: &str,
    claim: BaselineClaim,
    n: i64,
    n_sessions: i64,
) -> Option<WithheldClaim> {
    let need_n = minimum_n(metric, claim);
    let need_sessions = minimum_sessions(claim);
    if n >= need_n && n_sessions >= need_sessions {
        return None;
    }

    let mut shortfalls: Vec<String> = Vec::new();
    if n < need_n {
        shortfalls.push(format!("{need_n} samples (there are {n})"));
    }
    if n_sessions < need_sessions {
        shortfalls.push(format!(
            "{need_sessions} sessions (there are {n_sessions}) — repeated swings in one bay hour \
             are one occasion, not several"
        ));
    }

    Some(WithheldClaim {
        claim,
        have_n: n,
        need_n,
        have_sessions: n_sessions,
        need_sessions,
        reason: format!(
            "{} needs {}",
            claim_phrasing(claim),
            shortfalls.join(" and ")
        ),
    })
}

/// What each claim would be asserting, in the voice a refusal has to explain itself in. Python's
/// `_CLAIM_PHRASING` dict; a `match` here, so a claim added without a phrasing fails the build where
/// the dict would raise `KeyError` on the first refusal.
fn claim_phrasing(claim: BaselineClaim) -> &'static str {
    match claim {
        BaselineClaim::Center => "a typical value for this golfer",
        BaselineClaim::Spread => "a claim about how repeatable this is",
        BaselineClaim::Trend => "a claim that this has moved",
    }
}

// ------------------------------------------------------------------------------------- internals

/// One metric's statistics, with every claim the guard refused stripped back out.
///
/// Computed first and *then* removed, rather than guarded before computing. It reads backwards but it
/// is the safer order: one place decides what survives, so a statistic can never reach the output by
/// a path that forgot to ask.
fn baseline_for(name: &str, samples: &[MetricSample]) -> MetricBaseline {
    let values: Vec<f64> = samples.iter().map(|sample| sample.value).collect();
    let sessions = sessions_of(samples);
    let (n, n_sessions) = (count(values.len()), count(sessions.len()));

    let mut ready = Vec::new();
    let mut withheld = Vec::new();
    for claim in BaselineClaim::ALL {
        match refuse(name, claim, n, n_sessions) {
            None => ready.push(claim),
            Some(refusal) => withheld.push(refusal),
        }
    }

    // A metric is in the pool only once it has a sample, so there is always a mean to take.
    let (mean, sd) = mean_and_sd(&values).expect("a pooled metric has at least one sample");
    let first = &samples[0];
    let mut baseline = MetricBaseline {
        name: name.to_string(),
        unit: first.unit.clone(),
        source: first.source.clone(),
        n,
        n_sessions,
        mean: None,
        mean_ci: None,
        median: None,
        sd: None,
        sd_ci: None,
        minimum: None,
        maximum: None,
        sessions,
        ready,
        withheld,
    };

    if baseline.supports(BaselineClaim::Center) {
        baseline.mean = Some(mean);
        baseline.mean_ci = interval(mean_ci(&values));
        baseline.median = percentile(&values, 0.5);
    }

    if baseline.supports(BaselineClaim::Spread) {
        baseline.sd = sd;
        baseline.sd_ci = interval(sd_ci(&values));
        baseline.minimum = first_extreme(&values, |value, best| value < best);
        baseline.maximum = first_extreme(&values, |value, best| value > best);
    }

    if baseline.supports(BaselineClaim::Trend) {
        let means = session_means(samples);
        for session in &mut baseline.sessions {
            session.mean = Some(means[session.session_id.as_str()]);
        }
    }

    baseline
}

/// Per-session counts, oldest first. `mean` stays `None` until `Trend` opens it.
///
/// Sorted on `(captured_at, session_id)`: the instant first ([`contracts::Timestamp`]'s order, as an
/// aware `datetime`'s is), then the id, so two sessions whose first samples share an instant are
/// ordered by name and not by which was met first. The career family's `session-tie` makes the two
/// orders disagree.
fn sessions_of(samples: &[MetricSample]) -> Vec<SessionSample> {
    let mut counts: HashMap<&str, i64> = HashMap::new();
    let mut first_seen: Vec<&MetricSample> = Vec::new();
    for sample in samples {
        let n = counts.entry(sample.session_id.as_str()).or_insert(0);
        if *n == 0 {
            first_seen.push(sample);
        }
        *n += 1;
    }

    // `sort_by` is stable, as `sorted` is; no two entries tie on both keys anyway, since each
    // session appears once.
    first_seen.sort_by(|a, b| {
        a.captured_at
            .cmp(&b.captured_at)
            .then_with(|| a.session_id.cmp(&b.session_id))
    });
    first_seen
        .into_iter()
        .map(|sample| SessionSample {
            session_id: sample.session_id.clone(),
            captured_at: sample.captured_at,
            n: counts[sample.session_id.as_str()],
            mean: None,
        })
        .collect()
}

/// Each session's mean over its own samples, with CPython's compensated `sum()`.
fn session_means(samples: &[MetricSample]) -> HashMap<&str, f64> {
    let mut grouped: HashMap<&str, Vec<f64>> = HashMap::new();
    for sample in samples {
        grouped
            .entry(sample.session_id.as_str())
            .or_default()
            .push(sample.value);
    }
    grouped
        .into_iter()
        .map(|(session_id, values)| (session_id, pyfmt::sum(&values) / values.len() as f64))
        .collect()
}

fn interval(bounds: Option<(f64, f64)>) -> Option<Interval> {
    bounds.map(|(low, high)| Interval {
        low,
        high,
        confidence: Interval::DEFAULT_CONFIDENCE,
    })
}

/// Python's `min`/`max` over floats: the first value that no later one beats under `better`, since
/// CPython replaces its running answer only on a strict comparison. `None` on an empty slice, which
/// no caller passes (a spread is never ready on fewer than ten samples).
fn first_extreme(values: &[f64], better: impl Fn(f64, f64) -> bool) -> Option<f64> {
    values
        .iter()
        .copied()
        .reduce(|best, value| if better(value, best) { value } else { best })
}

/// A `len()` as the `i64` a contract field holds. A corpus past `i64::MAX` swings is not a corpus.
fn count(len: usize) -> i64 {
    i64::try_from(len).expect("a count fits in an i64")
}

#[cfg(test)]
mod tests {
    //! `tests/analysis/test_baseline.py`, ported case for case. What is tested is the **refusal**:
    //! a guard that is slightly too permissive fails in the one direction nobody notices, a
    //! confident-looking sentence about a tendency built out of two swings.
    //!
    //! Corpora are built from the contracts directly. The reader has its own tests, and mixing the
    //! two would make a guard failure look like a reader failure.

    use super::*;
    use contracts::placements::placement_names;
    use contracts::swing::Measurement;
    use contracts::Timestamp;

    const POSE: &str = "pose:face_on";
    const LM: &str = "launch_monitor:hd_golf";

    fn at(day: u32) -> Timestamp {
        Timestamp::parse(&format!("2026-08-{day:02}T12:00:00Z")).expect("a valid timestamp")
    }

    fn measurement(name: &str, value: f64, source: &str) -> Measurement {
        let unit = if source == LM {
            "degrees"
        } else {
            "shoulder_widths"
        };
        Measurement {
            name: name.into(),
            value,
            unit: unit.into(),
            source: source.into(),
            detail: "test".into(),
        }
    }

    /// One distinct swing. The defaults give every swing its own clip, photo and session; a test
    /// that needs otherwise sets the field. `swing_id` is unique per swing even when several share a
    /// session, since two colliding on `session/swing` would silently read as one.
    fn swing(index: u32, values: &[(&str, f64)], source: &str) -> CorpusSwing {
        CorpusSwing {
            player_id: "aaron".into(),
            session_id: format!("2026-08-{index:02}"),
            swing_id: index.to_string(),
            captured_at: at(index),
            face_on_sha256: format!("clip-{index}"),
            shot_sha256: Some(format!("photo-{index}")),
            club: None,
            measurements: values
                .iter()
                .map(|&(name, value)| measurement(name, value, source))
                .collect(),
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
        }
    }

    fn corpus(swings: Vec<CorpusSwing>) -> CareerCorpus {
        serde_json::from_value(serde_json::json!({"player_id": "aaron"}))
            .map(|empty: CareerCorpus| CareerCorpus { swings, ..empty })
            .expect("an empty corpus parses")
    }

    /// `n` distinct swings, each in its own session, each carrying one measurement.
    fn n_swings(n: u32, metric: &str) -> CareerCorpus {
        corpus(
            (1..=n)
                .map(|i| swing(i, &[(metric, 0.2 + f64::from(i) * 0.01)], POSE))
                .collect(),
        )
    }

    fn close(actual: f64, expected: f64) {
        assert!((actual - expected).abs() < 1e-12, "{actual} != {expected}");
    }

    // --- the refusal ---------------------------------------------------------------------------

    /// The state on disk today: two swings, and nothing sayable about any of it.
    #[test]
    fn the_guard_refuses_below_the_floor_and_says_what_it_needs() {
        let baseline = build_baseline(&n_swings(2, "head_sway_norm"));
        let metric = &baseline.metrics["head_sway_norm"];
        assert!(baseline.nothing_sayable());
        assert_eq!(metric.n, 2);
        assert!(metric.ready.is_empty());

        let claims: Vec<BaselineClaim> = metric.withheld.iter().map(|w| w.claim).collect();
        assert_eq!(claims, BaselineClaim::ALL);
        let center = &metric.withheld[0];
        assert_eq!((center.have_n, center.need_n), (2, 5));
        assert!(center.reason.contains("5 samples"), "{}", center.reason);
        assert!(center.reason.contains("there are 2"), "{}", center.reason);
    }

    /// The sentences in full, both shortfalls joined and each alone, as `refuse` builds them.
    #[test]
    fn a_refusal_names_every_shortfall_in_the_claims_own_voice() {
        assert_eq!(
            refuse("head_sway_norm", BaselineClaim::Trend, 2, 2)
                .unwrap()
                .reason,
            "a claim that this has moved needs 12 samples (there are 2) and 3 sessions (there are \
             2) — repeated swings in one bay hour are one occasion, not several"
        );
        assert_eq!(
            refuse("tempo_ratio", BaselineClaim::Center, 7, 1)
                .unwrap()
                .reason,
            "a typical value for this golfer needs 8 samples (there are 7)"
        );
        let spread = refuse("hip_shift_at_top_norm", BaselineClaim::Spread, 11, 4).unwrap();
        assert_eq!(
            spread.reason,
            "a claim about how repeatable this is needs 12 samples (there are 11)"
        );
        assert_eq!((spread.have_sessions, spread.need_sessions), (4, 1));
        // At both floors exactly, the claim opens.
        assert_eq!(refuse("head_sway_norm", BaselineClaim::Trend, 12, 3), None);
        assert_eq!(
            refuse("head_sway_norm", BaselineClaim::Trend, 12, 2)
                .unwrap()
                .reason,
            "a claim that this has moved needs 3 sessions (there are 2) — repeated swings in one \
             bay hour are one occasion, not several"
        );
    }

    /// Withheld means absent, not flagged. This is the test that would catch someone "helpfully"
    /// populating the mean anyway and gating only the display.
    #[test]
    fn a_withheld_claim_leaves_no_number_to_render() {
        let metric =
            build_baseline(&n_swings(2, "head_sway_norm")).metrics["head_sway_norm"].clone();
        assert_eq!(
            (
                metric.mean,
                metric.median,
                metric.sd,
                metric.minimum,
                metric.maximum
            ),
            (None, None, None, None, None)
        );
        assert_eq!((metric.mean_ci, metric.sd_ci), (None, None));
        assert!(metric.sessions.iter().all(|s| s.mean.is_none()));
    }

    /// A refusal has to be actionable: how much data there is, and from how many occasions.
    #[test]
    fn the_evidence_behind_a_refusal_is_always_present() {
        let metric = &build_baseline(&n_swings(3, "head_sway_norm")).metrics["head_sway_norm"];
        assert_eq!((metric.n, metric.n_sessions), (3, 3));
        let sessions: Vec<(&str, i64)> = metric
            .sessions
            .iter()
            .map(|s| (s.session_id.as_str(), s.n))
            .collect();
        assert_eq!(
            sessions,
            [("2026-08-01", 1), ("2026-08-02", 1), ("2026-08-03", 1)]
        );
    }

    // --- per-claim grain -----------------------------------------------------------------------

    /// At n = 6 the mean is resolved to about half a personal spread, but the sample sd still
    /// carries ~32% relative error: enough for a typical value, not for "repeatable".
    #[test]
    fn center_can_be_ready_while_spread_is_still_withheld() {
        let metric = &build_baseline(&n_swings(6, "head_sway_norm")).metrics["head_sway_norm"];
        assert!(metric.supports(BaselineClaim::Center));
        assert!(!metric.supports(BaselineClaim::Spread));
        assert!(metric.mean.is_some() && metric.mean_ci.is_some() && metric.median.is_some());
        assert!(metric.sd.is_none() && metric.sd_ci.is_none());
        assert!(metric.minimum.is_none() && metric.maximum.is_none());
    }

    #[test]
    fn spread_opens_at_its_own_floor_and_brings_the_range_with_it() {
        let metric = &build_baseline(&n_swings(10, "head_sway_norm")).metrics["head_sway_norm"];
        assert!(metric.supports(BaselineClaim::Spread));
        close(metric.minimum.unwrap(), 0.21);
        close(metric.maximum.unwrap(), 0.30);
        let (sd, sd_ci) = (metric.sd.unwrap(), metric.sd_ci.as_ref().unwrap());
        assert!(sd_ci.low < sd && sd < sd_ci.high);
    }

    /// Twelve swings hit in one bay hour are one occasion, however many of them there are.
    #[test]
    fn a_trend_needs_occasions_not_swings() {
        let swings = (1..=12)
            .map(|i| {
                let mut s = swing(i, &[("head_sway_norm", 0.2 + f64::from(i) * 0.01)], POSE);
                s.session_id = "2026-08-01".into();
                s
            })
            .collect();
        let metric = build_baseline(&corpus(swings)).metrics["head_sway_norm"].clone();
        assert_eq!((metric.n, metric.n_sessions), (12, 1));
        assert!(metric.supports(BaselineClaim::Center) && metric.supports(BaselineClaim::Spread));
        assert!(!metric.supports(BaselineClaim::Trend));

        let refusal = metric
            .withheld
            .iter()
            .find(|w| w.claim == BaselineClaim::Trend)
            .unwrap();
        assert_eq!((refusal.have_sessions, refusal.need_sessions), (1, 3));
        assert!(refusal.reason.contains("3 sessions"));
        assert!(
            !refusal.reason.contains("12 samples"),
            "the sample floor was met; only sessions are short"
        );
    }

    #[test]
    fn a_ready_trend_fills_in_the_per_session_means() {
        let swings = (1..=12)
            .map(|i| {
                let mut s = swing(i, &[("head_sway_norm", 0.2 + f64::from(i) * 0.01)], POSE);
                s.session_id = format!("2026-08-{:02}", i % 3);
                s
            })
            .collect();
        let metric = build_baseline(&corpus(swings)).metrics["head_sway_norm"].clone();
        assert_eq!((metric.n, metric.n_sessions), (12, 3));
        assert!(metric.supports(BaselineClaim::Trend));
        assert!(metric.sessions.iter().all(|s| s.mean.is_some()));
        assert_eq!(metric.sessions.iter().map(|s| s.n).sum::<i64>(), 12);
    }

    /// Two sessions whose first samples share an instant are ordered by id, not by arrival.
    #[test]
    fn sessions_tied_on_the_instant_sort_by_id() {
        let mut later_id = swing(1, &[("head_sway_norm", 0.2)], POSE);
        later_id.session_id = "b".into();
        let mut earlier_id = swing(2, &[("head_sway_norm", 0.3)], POSE);
        earlier_id.session_id = "a".into();
        // The same instant, spelled in another offset.
        earlier_id.captured_at = Timestamp::parse("2026-08-01T17:30:00+05:30").unwrap();
        let metric = &build_baseline(&corpus(vec![later_id, earlier_id])).metrics["head_sway_norm"];
        let ids: Vec<&str> = metric
            .sessions
            .iter()
            .map(|s| s.session_id.as_str())
            .collect();
        assert_eq!(ids, ["a", "b"]);
    }

    // --- per-metric floors ---------------------------------------------------------------------

    /// `tempo_ratio` needs 8 where `head_sway_norm` needs 5, and the gap is recorded, not vibes.
    #[test]
    fn a_noisier_metric_stays_silent_where_a_cleaner_one_speaks() {
        let swings = (1..=6)
            .map(|i| {
                let i_f = f64::from(i);
                swing(
                    i,
                    &[
                        ("head_sway_norm", 0.2 + i_f * 0.01),
                        ("tempo_ratio", 2.4 + i_f * 0.05),
                    ],
                    POSE,
                )
            })
            .collect();
        let baseline = build_baseline(&corpus(swings));
        assert!(baseline.metrics["head_sway_norm"].supports(BaselineClaim::Center));
        assert!(!baseline.metrics["tempo_ratio"].supports(BaselineClaim::Center));
        // A metric added tomorrow is gated by default rather than ungated by omission.
        assert_eq!(minimum_n("some_future_metric", BaselineClaim::Center), 5);
        assert_eq!(minimum_n("some_future_metric", BaselineClaim::Spread), 10);
    }

    // --- what may be pooled --------------------------------------------------------------------

    /// Unanalyzed, stale and outdated each contribute nothing, and nothing is not zero.
    #[test]
    fn a_swing_that_counts_toward_no_metric_moves_no_statistic() {
        let mut swings: Vec<CorpusSwing> = (1..=5)
            .map(|i| swing(i, &[("head_sway_norm", 0.25)], POSE))
            .collect();
        let mut unanalyzed = swing(10, &[("head_sway_norm", 9.9)], POSE);
        unanalyzed.analyzed = false;
        let mut stale = swing(11, &[("head_sway_norm", 9.9)], POSE);
        stale.stale = true;
        let mut outdated = swing(12, &[("head_sway_norm", 9.9)], POSE);
        outdated.outdated = true;
        swings.extend([unanalyzed, stale, outdated]);

        let metric = &build_baseline(&corpus(swings)).metrics["head_sway_norm"];
        assert_eq!(metric.n, 5);
        assert_eq!(metric.mean, Some(0.25));
    }

    /// The variance-collapsing failure, at the pooling layer rather than the counting layer.
    #[test]
    fn re_uploads_of_one_clip_contribute_one_pose_sample() {
        let swings = (1..=5)
            .map(|i| {
                let mut s = swing(i, &[("head_sway_norm", 0.25)], POSE);
                s.face_on_sha256 = "same-clip".into();
                s
            })
            .collect();
        let metric = &build_baseline(&corpus(swings)).metrics["head_sway_norm"];
        assert_eq!(metric.n, 1);
        assert!(!metric.supports(BaselineClaim::Center));
    }

    /// One physical swing produced one ball flight; the clip is what makes it two swings.
    #[test]
    fn two_swings_sharing_a_shot_photo_are_two_pose_readings_and_one_shot_reading() {
        let swings = [(1, "clip-a", 0.21), (2, "clip-b", 0.29)]
            .into_iter()
            .map(|(i, clip, value)| {
                let mut s = swing(i, &[("head_sway_norm", value)], POSE);
                s.face_on_sha256 = clip.into();
                s.shot_sha256 = Some("one-photo".into());
                s.measurements
                    .push(measurement("face_to_path_deg", 10.9, LM));
                s
            })
            .collect();
        let baseline = build_baseline(&corpus(swings));
        assert_eq!(baseline.metrics["head_sway_norm"].n, 2);
        assert_eq!(baseline.metrics["face_to_path_deg"].n, 1);
    }

    /// The rule `mcp.query.get_session_summary` applies before averaging a shot metric.
    #[test]
    fn a_flagged_shot_contributes_no_launch_monitor_value() {
        let swings: Vec<CorpusSwing> = (1..=3)
            .map(|i| {
                let mut s = swing(i, &[("face_to_path_deg", 10.0 + f64::from(i))], LM);
                s.shot_needs_review = i == 1;
                s
            })
            .collect();
        let corpus = corpus(swings);
        assert_eq!(build_baseline(&corpus).metrics["face_to_path_deg"].n, 2);
        let pooled = &pooled_samples(&corpus)["face_to_path_deg"];
        assert!(pooled.iter().all(|sample| sample.value != 11.0));
    }

    /// No photo hash means no artifact to key the reading on, so it is not a reading.
    #[test]
    fn a_launch_monitor_metric_needs_a_shot_photo_to_dedupe_on() {
        let mut lone = swing(1, &[("face_to_path_deg", 10.9)], LM);
        lone.shot_sha256 = None;
        assert!(!build_baseline(&corpus(vec![lone]))
            .metrics
            .contains_key("face_to_path_deg"));
    }

    /// Conservative: it can over-count against a real artifact key, never under-count.
    #[test]
    fn an_unrecognised_source_falls_back_to_swing_identity() {
        let swings = [1, 2]
            .into_iter()
            .map(|i| swing(i, &[("future_metric", f64::from(i))], "wearable:imu"))
            .collect();
        assert_eq!(
            build_baseline(&corpus(swings)).metrics["future_metric"].n,
            2
        );
    }

    /// A characterization pin, not an endorsement (ADR-022's fourth addendum): a population
    /// placement matches neither dedupe prefix, so it pools per swing directory and acquires a
    /// center at the CENTER floor like any personal metric. If this fails, the deferred decision has
    /// been taken and the addendum needs updating.
    #[test]
    fn a_placement_pools_as_a_metric_today_and_that_is_deferred() {
        let placement = placement_names()[0];
        let floor = u32::try_from(minimum_n(placement, BaselineClaim::Center)).unwrap();
        let swings = (1..=floor)
            .map(|i| {
                swing(
                    i,
                    &[(placement, 2.0 + f64::from(i) * 0.1)],
                    "population:golfdb",
                )
            })
            .collect();
        let metric = &build_baseline(&corpus(swings)).metrics[placement];
        assert_eq!(metric.n, i64::from(floor));
        assert!(metric.mean.is_some());
        assert!(metric.supports(BaselineClaim::Center));
    }

    // --- assembly ------------------------------------------------------------------------------

    /// Not `distinct_swings`: a swing that contributed nothing does not explain any `n`.
    #[test]
    fn the_baseline_reports_what_it_was_built_from() {
        let mut swings: Vec<CorpusSwing> = [(1, "2026-08-01", 0.21), (2, "2026-08-01", 0.25)]
            .into_iter()
            .chain([(3, "2026-08-02", 0.29)])
            .map(|(i, session, value)| {
                let mut s = swing(i, &[("head_sway_norm", value)], POSE);
                s.session_id = session.into();
                s
            })
            .collect();
        let mut outdated = swing(4, &[("head_sway_norm", 9.9)], POSE);
        outdated.outdated = true;
        swings.push(outdated);

        let baseline = build_baseline(&corpus(swings));
        assert_eq!(baseline.built_from_swings, 3);
        assert_eq!(baseline.built_from_sessions, 2);
    }

    #[test]
    fn provenance_survives_onto_the_baseline() {
        let metric = &build_baseline(&n_swings(2, "head_sway_norm")).metrics["head_sway_norm"];
        assert_eq!(metric.unit, "shoulder_widths");
        assert_eq!(metric.source, POSE);
    }

    #[test]
    fn metrics_are_sorted_so_two_runs_agree() {
        let values = [
            ("tempo_ratio", 2.4),
            ("head_sway_norm", 0.2),
            ("hip_sway_norm", 0.3),
        ];
        let swings = (1..=2).map(|i| swing(i, &values, POSE)).collect();
        let names: Vec<String> = build_baseline(&corpus(swings))
            .metrics
            .into_keys()
            .collect();
        assert_eq!(names, ["head_sway_norm", "hip_sway_norm", "tempo_ratio"]);
    }

    /// The first true answer about every golfer.
    #[test]
    fn an_empty_corpus_is_an_empty_baseline_not_an_error() {
        let mut empty = corpus(Vec::new());
        empty.player_id = "nobody".into();
        let baseline = build_baseline(&empty);
        assert!(baseline.metrics.is_empty());
        assert!(baseline.nothing_sayable());
        assert_eq!(baseline.built_from_swings, 0);
        assert_eq!(baseline.player_id, "nobody");
    }

    #[test]
    fn a_swing_carrying_no_measurements_produces_no_metric() {
        assert!(build_baseline(&corpus(vec![swing(1, &[], POSE)]))
            .metrics
            .is_empty());
    }

    /// Python's `min`/`max` keep the first extreme, so a `0.0` met before a `-0.0` is the minimum
    /// and prints without a sign, and the reverse is the maximum and prints with one. No vector holds
    /// a signed-zero tie, so this is the pin, through the builder rather than the helper alone.
    ///
    /// Measured (M36 P11): `reduce(f64::min)` and `reduce(f64::max)` pass this on x86_64 Windows,
    /// because that target's lowering happens to keep the accumulator on a tie. `f64::min` documents
    /// either answer for signed zeros, so the fold stays and this holds the behaviour, not the
    /// spelling.
    #[test]
    fn the_range_is_the_first_extreme_in_sample_order() {
        let zeros = |first: f64, second: f64, rest: f64| {
            let values: Vec<f64> = [first, second]
                .into_iter()
                .chain(std::iter::repeat_n(rest, 8))
                .collect();
            let swings = (1..=10)
                .map(|i| swing(i, &[("head_sway_norm", values[i as usize - 1])], POSE))
                .collect();
            build_baseline(&corpus(swings)).metrics["head_sway_norm"].clone()
        };
        let low = zeros(0.0, -0.0, 1.0);
        assert_eq!(low.minimum.map(f64::to_bits), Some(0.0f64.to_bits()));
        let high = zeros(-0.0, 0.0, -1.0);
        assert_eq!(high.maximum.map(f64::to_bits), Some((-0.0f64).to_bits()));
        let low = zeros(-0.0, 0.0, 1.0);
        assert_eq!(low.minimum.map(f64::to_bits), Some((-0.0f64).to_bits()));
        let high = zeros(0.0, -0.0, -1.0);
        assert_eq!(high.maximum.map(f64::to_bits), Some(0.0f64.to_bits()));

        // And the helper alone, on the empty slice no caller passes.
        assert_eq!(
            first_extreme(&[0.0, -0.0, 1.0], |v, b| v < b).map(f64::to_bits),
            Some(0.0f64.to_bits())
        );
        assert_eq!(
            first_extreme(&[-0.0, 0.0, -1.0], |v, b| v > b).map(f64::to_bits),
            Some((-0.0f64).to_bits())
        );
        assert_eq!(first_extreme(&[], |v, b| v < b), None);
    }
}
