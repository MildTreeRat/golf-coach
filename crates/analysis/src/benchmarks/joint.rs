//! The tour joint-distribution model — how unusual is this *combination*? [M22 P5b]
//!
//! The port of `benchmarks/joint.py`. [`super::store`]'s `ranges.json` says whether each of six
//! numbers is in band and [`super::distributions`]' `golfdb_v1.json` says where each one sits in the
//! tour population. Neither can say anything about the six **together**, and that is the gap this
//! closes: a swing can pass all six bands individually while sitting at a combination no tour
//! player produces, because six independent range checks have no way to express "this is fine only
//! if that is also true".
//!
//! There is something to express. Over the 458 face-on clips the bands were cut from,
//! `head_sway_norm` and `hip_shift_at_top_norm` correlate at +0.44 and `head_sway_norm` and
//! `head_hip_gain_norm` at -0.39.
//!
//! **Unusual is not bad.** Every clip behind the model is a tour professional, so it knows the shape
//! of swings that work and nothing at all about swings that don't — the same limit ADR-012 records
//! for the bands. The output is therefore a percentile within the tour population, never a score,
//! and [`placement_for`] is not on the scoring path any more than
//! [`super::distributions::Distribution::percentile_of`] is (ADR-010 §2).
//!
//! # Where the insertion-order edge lands
//!
//! [`JointPlacement::contributions`] is a [`OrderedMap`], not a map, and ADR-032 §3's third edge is
//! why: `engine.py` takes `next(iter(contributions))` and interpolates the **name** into a
//! `Measurement.detail` as "Largest contributor". The shares are built in the model's own metric
//! order and then stably sorted by descending magnitude, so on a tie the winner is whichever metric
//! came first in the artifact. A `HashMap` would answer a correct number under a wrong name.

use std::sync::OnceLock;

use serde::Deserialize;

use crate::pyfmt::{round_to, OrderedMap};

/// ADR-032 §5, as in [`super::store`]: embedded from the Python package path, one copy on disk.
const JOINT_MODEL_JSON: &str =
    include_str!("../../../../src/golf_coach/analysis/benchmarks/joint_model_v1.json");

/// Where one swing's six metrics sit, as a combination, in the tour population.
#[derive(Debug, Clone, PartialEq)]
pub struct JointPlacement {
    pub distance: f64,
    pub percentile: f64,
    pub percentile_clamped: bool,
    pub population_n: i64,
    pub population_players: i64,
    /// Metric -> share of the squared distance it accounts for, largest first. Shares sum to 1.0
    /// and **may be negative**: a negative share means that metric is pulling the swing *towards*
    /// the population given the others, which is a real thing a correlated model can say and a band
    /// cannot. That is also why the sort key is `-abs(share)` and not `-share` — the two are not
    /// interchangeable, and on `[-0.4, 0.4]` the first ties where the second does not.
    pub contributions: OrderedMap<f64>,
}

/// A robust center, a robust scale, and the inverse correlation of the tour population.
#[derive(Debug, Clone, Deserialize)]
pub struct JointModel {
    pub kind: String,
    pub metrics: Vec<String>,
    pub n: i64,
    pub n_players: i64,
    pub clip_z: f64,
    pub center: Vec<f64>,
    pub scale: Vec<f64>,
    pub precision: Vec<Vec<f64>>,
    pub distance_quantiles: Quantiles,
}

/// The five stored points, as named fields rather than a map.
///
/// Python indexes `self.distance_quantiles["p10"]`, which raises on a malformed artifact; a struct
/// makes the same artifact fail at the parse instead, which is the earlier and louder of the two.
/// The field names are the file's keys.
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct Quantiles {
    pub p10: f64,
    pub p25: f64,
    pub p50: f64,
    pub p75: f64,
    pub p90: f64,
}

impl JointModel {
    /// Robust z per metric, clipped as the fit was, or `None` if any metric is absent.
    ///
    /// **All six or nothing.** A Mahalanobis distance over a subset is a different quantity — the
    /// marginal distribution of those metrics — and computing it would need the covariance
    /// re-inverted for that subset. Returning `None` and letting the caller name what was missing is
    /// the same choice `unscored` makes everywhere else: no number beats a wrong one (ADR-010 §2).
    /// In practice this abstains most often when tempo is absent, which is the ~14% of clips where
    /// address detection declines to guess.
    fn standardize(&self, observed: &OrderedMap<f64>) -> Option<Vec<f64>> {
        self.metrics
            .iter()
            .enumerate()
            .map(|(index, metric)| {
                let value = observed.get(metric)?;
                let z = (value - self.center[index]) / self.scale[index];
                // `max(-clip, min(clip, z))` exactly as written: `clamp` panics on a NaN bound and
                // orders its arguments differently, and the two disagree on a NaN `z` — which a
                // divide by a zero scale could produce from an artifact nothing validates.
                Some(self.clip_z.min(z).max(-self.clip_z))
            })
            .collect()
    }

    /// Distance, percentile and per-metric decomposition, or `None` if a metric is missing.
    pub fn placement_for(&self, observed: &OrderedMap<f64>) -> Option<JointPlacement> {
        let z = self.standardize(observed)?;

        // precision @ z, then z . (precision @ z) — the quadratic form, term by term so each
        // metric's share of it falls out of the same arithmetic rather than a second pass.
        let weighted: Vec<f64> = (0..z.len())
            .map(|i| (0..z.len()).map(|j| self.precision[i][j] * z[j]).sum())
            .collect();
        let terms: Vec<f64> = (0..z.len()).map(|i| z[i] * weighted[i]).collect();
        let squared: f64 = terms.iter().sum();
        let distance = squared.max(0.0).sqrt();

        let shares: OrderedMap<f64> = self
            .metrics
            .iter()
            .zip(&terms)
            .map(|(metric, term)| {
                (
                    metric.clone(),
                    if squared > 0.0 {
                        round_to(term / squared, 4)
                    } else {
                        0.0
                    },
                )
            })
            .collect();

        let (percentile, clamped) = self.percentile_of(distance);
        Some(JointPlacement {
            distance: round_to(distance, 4),
            percentile: round_to(percentile, 1),
            percentile_clamped: clamped,
            population_n: self.n,
            population_players: self.n_players,
            contributions: shares.sorted_by(|_, share| -share.abs()),
        })
    }

    /// Where this distance falls among tour swings' own distances, in `[10, 90]`.
    ///
    /// Same five stored quantiles and the same clamp as
    /// [`super::distributions::Distribution::percentile_of`], for the same reason: the tails were
    /// never stored, so a swing past them is reported as "at least this unusual" rather than
    /// extrapolated. The clamp matters more here than there — a swing far outside the population is
    /// exactly the interesting case, and reporting it as 90 is a floor on its strangeness, not a
    /// measurement of it.
    fn percentile_of(&self, distance: f64) -> (f64, bool) {
        let q = self.distance_quantiles;
        let points = [
            (10.0, q.p10),
            (25.0, q.p25),
            (50.0, q.p50),
            (75.0, q.p75),
            (90.0, q.p90),
        ];
        if distance <= points[0].1 {
            return (10.0, true);
        }
        if distance >= points[4].1 {
            return (90.0, true);
        }
        for window in points.windows(2) {
            let ((low_pct, low_val), (high_pct, high_val)) = (window[0], window[1]);
            if low_val <= distance && distance <= high_val {
                if high_val == low_val {
                    return (low_pct, false);
                }
                let span = (distance - low_val) / (high_val - low_val);
                return (low_pct + span * (high_pct - low_pct), false);
            }
        }
        // Unreachable for a monotone quantile set, and kept rather than made a panic because the
        // Python has it — the same dead arm `distributions::percentile_of` carries, and for the same
        // reason: an out-of-order artifact should degrade a coaching call, not crash it.
        (50.0, false)
    }
}

#[derive(Deserialize)]
struct ModelFile {
    model: JointModel,
}

/// The fitted tour joint-distribution model.
///
/// Infallible after the one parse, and there is no `joint_dataset_info` beside it: the `dataset`
/// block is provenance that `conformance.py::run_vector` never reads, the same cut
/// [`super::distributions`] made on `DatasetInfo`. It stays in the artifact, unparsed.
pub fn load_joint_model() -> &'static JointModel {
    static MODEL: OnceLock<JointModel> = OnceLock::new();
    MODEL.get_or_init(|| {
        let parsed: ModelFile = serde_json::from_str(JOINT_MODEL_JSON)
            .expect("joint_model_v1.json ships in this crate and parses");
        parsed.model
    })
}

/// Place one swing's measured metrics in the tour joint distribution.
pub fn placement_for(observed: &OrderedMap<f64>) -> Option<JointPlacement> {
    load_joint_model().placement_for(observed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at_the_center() -> OrderedMap<f64> {
        let model = load_joint_model();
        model
            .metrics
            .iter()
            .zip(&model.center)
            .map(|(metric, value)| (metric.clone(), *value))
            .collect()
    }

    #[test]
    fn the_shipped_artifact_parses_and_is_square() {
        let model = load_joint_model();
        assert_eq!(model.kind, "mahalanobis");
        let width = model.metrics.len();
        assert_eq!(model.center.len(), width);
        assert_eq!(model.scale.len(), width);
        assert_eq!(model.precision.len(), width);
        for row in &model.precision {
            assert_eq!(row.len(), width);
        }
    }

    /// **All six or nothing.** A subset is a different quantity, not a partial answer, so the
    /// refusal is the honest output — and it is the branch the corpus reaches whenever address
    /// detection declines to guess a tempo.
    #[test]
    fn one_missing_metric_refuses_the_whole_placement() {
        let mut observed = at_the_center();
        for metric in load_joint_model().metrics.clone() {
            let full = placement_for(&observed);
            assert!(full.is_some());
            observed = observed
                .iter()
                .filter(|(name, _)| *name != metric)
                .map(|(name, value)| (name.to_string(), *value))
                .collect();
            assert!(
                placement_for(&observed).is_none(),
                "dropping {metric} still produced a placement"
            );
            observed = at_the_center();
        }
    }

    /// A swing sitting exactly on the robust centre has distance 0 and every share 0 — the
    /// `squared > 0` arm's `else`, which is the one place the contributions are not a ratio.
    #[test]
    fn a_swing_at_the_center_has_no_distance_and_no_contributions() {
        let placement = placement_for(&at_the_center()).expect("six metrics present");
        assert_eq!(placement.distance, 0.0);
        assert!(placement.contributions.iter().all(|(_, v)| *v == 0.0));
        assert_eq!(placement.percentile, 10.0);
        assert!(placement.percentile_clamped);
    }

    /// The zero-share fall-through keeps the model's own metric order, because a stable sort over
    /// equal keys moves nothing. That order is what `engine.py` reads a *name* off.
    #[test]
    fn the_degenerate_shares_keep_the_artifact_order() {
        let placement = placement_for(&at_the_center()).unwrap();
        let names: Vec<&str> = placement.contributions.keys().collect();
        assert_eq!(names, load_joint_model().metrics.iter().collect::<Vec<_>>());
    }

    /// Shares are ordered by **magnitude**, so a metric pulling the swing *towards* the population
    /// can lead. `-share` instead of `-abs(share)` would bury it, which is a correct number under a
    /// wrong name in the sentence a golfer reads.
    ///
    /// The input is the correlation this model exists for: `head_sway_norm` and `head_hip_gain_norm`
    /// correlate at -0.39 on the fitted corpus, so sway two robust-sd *below* centre with the gain
    /// one *above* produces a genuinely negative share on the gain. Six independent band checks
    /// cannot say that at all.
    #[test]
    fn contributions_are_ranked_by_magnitude_and_not_by_sign() {
        let model = load_joint_model();
        let mut observed = at_the_center();
        observed.insert("head_sway_norm", model.center[1] - 2.0 * model.scale[1]);
        observed.insert("head_hip_gain_norm", model.center[5] + model.scale[5]);
        let placement = placement_for(&observed).expect("six metrics present");

        let magnitudes: Vec<f64> = placement
            .contributions
            .iter()
            .map(|(_, share)| share.abs())
            .collect();
        assert!(
            magnitudes.windows(2).all(|w| w[0] >= w[1]),
            "not descending by magnitude: {magnitudes:?}"
        );
        assert_eq!(
            placement.contributions.get("head_hip_gain_norm"),
            Some(&-0.0067),
            "the negative share the two metrics' correlation produces: {:?}",
            placement.contributions
        );
        // ...and it is ranked second, ahead of four larger-indexed zeros, because the sort reads
        // its magnitude and not its sign.
        let names: Vec<&str> = placement.contributions.keys().collect();
        assert_eq!(names[..2], ["head_sway_norm", "head_hip_gain_norm"]);
    }

    /// The clip is the fit's, applied in both directions, and it is what keeps one wild metric from
    /// dominating a distance the other five agree on.
    #[test]
    fn a_metric_far_outside_the_population_is_clipped_to_the_fitted_bound() {
        let model = load_joint_model();
        let mut near = at_the_center();
        let mut far = at_the_center();
        let metric = model.metrics[0].clone();
        near.insert(
            metric.clone(),
            model.center[0] + model.clip_z * model.scale[0],
        );
        far.insert(metric, model.center[0] + 50.0 * model.scale[0]);
        assert_eq!(
            placement_for(&near).unwrap().distance,
            placement_for(&far).unwrap().distance
        );
    }

    /// The tails were never stored, so the report past them is "at least this unusual" and the flag
    /// says so. A reader who takes a clamped 90 for a rank draws a wrong conclusion from a right
    /// number.
    #[test]
    fn the_percentile_clamps_at_both_stored_edges() {
        let model = load_joint_model();
        let q = model.distance_quantiles;
        assert_eq!(model.percentile_of(q.p10 - 1.0), (10.0, true));
        assert_eq!(model.percentile_of(q.p90 + 1.0), (90.0, true));
        let (mid, clamped) = model.percentile_of((q.p25 + q.p50) / 2.0);
        assert!(!clamped);
        assert!(mid > 25.0 && mid < 50.0, "{mid}");
    }

    /// Between two stored points the answer is linear in the distance, which is the whole of what
    /// "interpolated between the quantiles" means — and the arm a port could get right at the edges
    /// and wrong in the middle.
    #[test]
    fn the_percentile_interpolates_linearly_between_two_stored_points() {
        let model = load_joint_model();
        let q = model.distance_quantiles;
        let (half, _) = model.percentile_of((q.p50 + q.p75) / 2.0);
        assert!((half - 62.5).abs() < 1e-9, "{half}");
    }
}
