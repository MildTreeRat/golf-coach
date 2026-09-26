//! The tour trajectory model — is this the *shape* of a swing that works? [M22 P5b]
//!
//! The port of `benchmarks/trajectory.py`. [`super::joint`] places six scalars as a combination;
//! this places the **whole motion**: every tracked landmark, over normalised event time, against a
//! basis fitted on 415 face-on tour swings from 116 golfers.
//!
//! Two statistics, and they answer different questions:
//!
//! - **T² — distance inside the model's own subspace.** An unusual *amount* of the things tour
//!   swings do vary in. Calibrated: leave-one-player-out exceedance of p90 is 9.9% against a 10%
//!   target.
//! - **Q — the reconstruction residual, distance *off* the subspace.** A shape the tour basis
//!   cannot represent at all. This is the one nothing else here can express: not "too much of a
//!   normal thing" but "a thing this population does not do".
//!
//! **Q is not calibrated, and the artifact says so.** Its leave-one-player-out exceedance is 12%
//! against the same 10% target, because a golfer the basis never saw has idiosyncrasies that land in
//! the residual *by construction*. Q therefore partly measures "a different person" rather than "a
//! worse swing". [`contracts::placements::PlacementSpec::calibrated`] is where that ships to a
//! reader.
//!
//! # One artifact per camera, and both are here now [M22 P7]
//!
//! Python keys `_MODEL_FILES` by view and embeds two files — **not** one model with a view column,
//! because the two are fitted on different landmark lists over different clips, so they are
//! different objects that happen to answer the same question. The down-the-line fit drops the lead
//! arm, hidden by the torso from behind, and adds both ankles; it carries 510 swings against the
//! face-on 415. Only the face-on artifact crossed in P5b, on P2's rule for the `contracts/`
//! registries applied to a data file — a model lands with its caller, so its first run is also its
//! first gate — and P7 brings its caller, `engine::dtl_placements`.
//!
//! **Four of the fifteen corpus vectors reach the second basis and eleven refuse it**, which is the
//! mirror image of P5b's finding: face-on, the trail arm crosses the torso and *all fifteen* are
//! refused. From behind it is the lead arm that hides, and the down-the-line fit drops it — so the
//! same footage that cannot be placed face-on can sometimes be placed from the rear. Those four
//! vectors are the only committed evidence for this artifact's own arithmetic; the eleven gate the
//! refusal. `tests/alignment.rs` pins the split.

use std::sync::OnceLock;

use contracts::keypoints::FrameKeypoints;
use contracts::placements::{DOWN_THE_LINE, FACE_ON};
use contracts::swing::PhaseSegment;
use serde::Deserialize;

use crate::pyfmt::{round_to, OrderedMap};
use crate::trajectory::{anchors_from_phases, build_trajectory};

/// ADR-032 §5: embedded from the Python package path, one copy on disk.
const FACE_ON_MODEL_JSON: &str =
    include_str!("../../../../src/golf_coach/analysis/benchmarks/trajectory_model_v1.json");

/// The second camera's basis, same rule. [M22 P7]
const DTL_MODEL_JSON: &str =
    include_str!("../../../../src/golf_coach/analysis/benchmarks/trajectory_model_dtl_v1.json");

/// Where one swing's motion sits against the tour basis.
#[derive(Debug, Clone, PartialEq)]
pub struct TrajectoryPlacement {
    pub t2: f64,
    pub t2_percentile: f64,
    pub t2_percentile_clamped: bool,
    pub q: f64,
    pub q_percentile: f64,
    pub q_percentile_clamped: bool,
    pub population_n: i64,
    pub population_players: i64,
    /// Share of the squared reconstruction error falling in each anchor interval, largest first.
    /// This is the "when" — an unusual shape that is all in `top->impact` is a different fault from
    /// the same magnitude spread evenly, and no scalar checkpoint can make that distinction.
    ///
    /// A [`OrderedMap`] for [`super::joint::JointPlacement::contributions`]' reason: `engine.py`
    /// reads the first key and interpolates it into a sentence, and the intervals are inserted in
    /// the artifact's own event order before the stable sort. Note the key here is `-share`, not
    /// `-abs(share)` — a squared residual cannot be negative, and the two sorts are not
    /// interchangeable in general.
    pub residual_by_interval: OrderedMap<f64>,
}

/// Mean trajectory, principal-component basis, and the population's own distance quantiles.
#[derive(Debug, Clone, Deserialize)]
pub struct TrajectoryModel {
    pub kind: String,
    pub landmarks: Vec<String>,
    pub axes: Vec<String>,
    pub events: Vec<String>,
    pub steps: usize,
    pub n: i64,
    pub n_players: i64,
    pub dimensions: usize,
    pub components: usize,
    pub explained_variance: f64,
    pub mean: Vec<f64>,
    pub scale: Vec<f64>,
    pub basis: Vec<Vec<f64>>,
    pub t2_quantiles: super::joint::Quantiles,
    pub q_quantiles: super::joint::Quantiles,
    /// Both figures ship inside the artifact; read them before quoting either number at a golfer.
    pub leave_one_player_out_exceedance: std::collections::BTreeMap<String, f64>,
}

impl TrajectoryModel {
    /// Project one swing onto the basis, or `None` when it cannot be built.
    ///
    /// Returns `None` rather than a guess whenever the anchors collapse or a landmark is missing too
    /// much of its timeline — ADR-010 §2's rule, applied to a vector instead of a scalar.
    pub fn placement_for(
        &self,
        keypoints: &[FrameKeypoints],
        phases: &[PhaseSegment],
        left_handed: bool,
    ) -> Option<TrajectoryPlacement> {
        let anchors = anchors_from_phases(phases)?;
        let vector = build_trajectory(
            keypoints,
            &[anchors.0, anchors.1, anchors.2],
            self.steps,
            &self.landmarks,
            &self.axes,
            left_handed,
        )?;
        self.placement_from_vector(&vector)
    }

    /// Project an already-built feature vector. Shared by both entry points.
    pub fn placement_from_vector(&self, vector: &[f64]) -> Option<TrajectoryPlacement> {
        if vector.len() != self.dimensions {
            return None;
        }

        let centered: Vec<f64> = vector.iter().zip(&self.mean).map(|(v, m)| v - m).collect();
        let scores: Vec<f64> = self
            .basis
            .iter()
            .map(|row| row.iter().zip(&centered).map(|(b, c)| b * c).sum())
            .collect();

        let t2 = scores
            .iter()
            .zip(&self.scale)
            .map(|(s, sd)| (s / sd).powi(2))
            .sum::<f64>()
            .sqrt();

        let reconstructed: Vec<f64> = (0..self.dimensions)
            .map(|d| {
                (0..scores.len())
                    .map(|k| scores[k] * self.basis[k][d])
                    .sum()
            })
            .collect();
        let residual: Vec<f64> = centered
            .iter()
            .zip(&reconstructed)
            .map(|(c, r)| c - r)
            .collect();
        let q = residual.iter().map(|r| r * r).sum::<f64>().sqrt();

        let (t2_pct, t2_clamped) = percentile(t2, self.t2_quantiles);
        let (q_pct, q_clamped) = percentile(q, self.q_quantiles);
        Some(TrajectoryPlacement {
            t2: round_to(t2, 4),
            t2_percentile: round_to(t2_pct, 1),
            t2_percentile_clamped: t2_clamped,
            q: round_to(q, 4),
            q_percentile: round_to(q_pct, 1),
            q_percentile_clamped: q_clamped,
            population_n: self.n,
            population_players: self.n_players,
            residual_by_interval: self.by_interval(&residual),
        })
    }

    /// Split the squared residual across the anchor intervals it falls in.
    ///
    /// The vector is row-major by timestep, so a timestep's slice is contiguous and each sample's
    /// position in event time says which interval it belongs to.
    fn by_interval(&self, residual: &[f64]) -> OrderedMap<f64> {
        let per_step = self.landmarks.len() * self.axes.len();
        let spans = self.events.len() - 1;
        let mut totals: OrderedMap<f64> = self
            .events
            .windows(2)
            .map(|pair| (format!("{}->{}", pair[0], pair[1]), 0.0))
            .collect();
        let names: Vec<String> = totals.keys().map(|k| k.to_string()).collect();

        for step in 0..self.steps {
            let t = (spans * step) as f64 / (self.steps - 1) as f64;
            let interval = &names[(t as usize).min(spans - 1)];
            let chunk = &residual[step * per_step..(step + 1) * per_step];
            let before = totals.get(interval).copied().unwrap_or(0.0);
            totals.insert(
                interval.clone(),
                before + chunk.iter().map(|v| v * v).sum::<f64>(),
            );
        }

        let grand: f64 = totals.iter().map(|(_, v)| *v).sum();
        if grand <= 0.0 {
            return names.into_iter().map(|name| (name, 0.0)).collect();
        }
        let shares: OrderedMap<f64> = totals
            .iter()
            .map(|(name, total)| (name.to_string(), round_to(total / grand, 4)))
            .collect();
        shares.sorted_by(|_, share| -share)
    }
}

/// Where `value` falls among the population's own, in `[10, 90]`, clamped at the edges.
///
/// Same five stored points and the same clamp as
/// [`super::distributions::Distribution::percentile_of`] and
/// [`super::joint::JointModel::percentile_of`]: the tails were never stored, so past them the honest
/// report is "at least this unusual" rather than a precise-looking extrapolation.
///
/// **A third copy of one walk, and it stays three.** The Python has three too. They are not one
/// function because each reads its quantiles off a different shape, and the two that return a
/// `clamped` flag differ from the one that does not — merging them means a flag no caller of
/// `Distribution` reads, or a second return value thrown away at two sites. `docs/CODE_STANDARDS.md`
/// counts that as duplication worth keeping; the port's job is to match, not to improve.
fn percentile(value: f64, quantiles: super::joint::Quantiles) -> (f64, bool) {
    let points = [
        (10.0, quantiles.p10),
        (25.0, quantiles.p25),
        (50.0, quantiles.p50),
        (75.0, quantiles.p75),
        (90.0, quantiles.p90),
    ];
    if value <= points[0].1 {
        return (10.0, true);
    }
    if value >= points[4].1 {
        return (90.0, true);
    }
    for window in points.windows(2) {
        let ((low_pct, low_val), (high_pct, high_val)) = (window[0], window[1]);
        if low_val <= value && value <= high_val {
            if high_val == low_val {
                return (low_pct, false);
            }
            return (
                low_pct + (value - low_val) / (high_val - low_val) * (high_pct - low_pct),
                false,
            );
        }
    }
    (50.0, false)
}

#[derive(Deserialize)]
struct ModelFile {
    model: TrajectoryModel,
}

/// The fitted tour trajectory model for one camera view.
///
/// # Panics
///
/// On a view with no artifact here, which is now anything but the two spelled ones. Python raises
/// `KeyError` and the message names what it does have; this says the same thing.
pub fn load_trajectory_model(view: &str) -> &'static TrajectoryModel {
    static FACE_ON_MODEL: OnceLock<TrajectoryModel> = OnceLock::new();
    static DTL_MODEL: OnceLock<TrajectoryModel> = OnceLock::new();
    // Python's `@lru_cache(maxsize=len(_MODEL_FILES))`, which is a cache per *view* and not a cache
    // of one: both bases are read on every two-camera bundle, so a single slot would reparse 64 KB
    // of JSON twice per swing.
    let parse = |json: &'static str, file: &'static str| {
        let parsed: ModelFile = serde_json::from_str(json)
            .unwrap_or_else(|e| panic!("{file} ships in this crate and parses: {e}"));
        parsed.model
    };
    match view {
        FACE_ON => FACE_ON_MODEL.get_or_init(|| parse(FACE_ON_MODEL_JSON, "trajectory_model_v1")),
        DOWN_THE_LINE => DTL_MODEL.get_or_init(|| parse(DTL_MODEL_JSON, "trajectory_model_dtl_v1")),
        other => {
            panic!("no trajectory model for view {other:?}; have [{DOWN_THE_LINE:?}, {FACE_ON:?}]")
        }
    }
}

/// Place one swing's motion against that camera's tour basis.
pub fn trajectory_placement_for(
    keypoints: &[FrameKeypoints],
    phases: &[PhaseSegment],
    left_handed: bool,
    view: &str,
) -> Option<TrajectoryPlacement> {
    load_trajectory_model(view).placement_for(keypoints, phases, left_handed)
}

/// Place a swing whose anchors are already known, rather than re-segmenting to find them. [M22 P7]
///
/// The bundle path has already segmented the down-the-line clip to build its alignment anchors, and
/// those are the same three instants this model resamples onto. Passing them in rather than calling
/// `segment_phases` a second time is not only cheaper — it makes it *impossible* for the frames the
/// trajectory is built on and the frames the warp pins to disagree, which is the same argument
/// `engine::analyze_swing_bundle` already makes about reusing the face-on phases.
///
/// Python defaults `view` to [`DOWN_THE_LINE`] here where every other entry point defaults to
/// [`FACE_ON`], because this signature exists for the rear clip. Rust has no default arguments, so
/// the one caller spells it and the asymmetry disappears.
pub fn placement_from_anchors(
    keypoints: &[FrameKeypoints],
    anchors: (f64, f64, f64),
    left_handed: bool,
    view: &str,
) -> Option<TrajectoryPlacement> {
    let model = load_trajectory_model(view);
    let vector = build_trajectory(
        keypoints,
        &[anchors.0, anchors.1, anchors.2],
        model.steps,
        &model.landmarks,
        &model.axes,
        left_handed,
    )?;
    model.placement_from_vector(&vector)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shipped_basis_parses_and_its_shapes_agree() {
        let model = load_trajectory_model(FACE_ON);
        assert_eq!(model.kind, "pca_trajectory");
        assert_eq!(
            model.dimensions,
            model.steps * model.landmarks.len() * model.axes.len(),
            "the basis is fitted row-major by timestep"
        );
        assert_eq!(model.mean.len(), model.dimensions);
        assert_eq!(model.basis.len(), model.components);
        assert_eq!(model.scale.len(), model.components);
        for row in &model.basis {
            assert_eq!(row.len(), model.dimensions);
        }
    }

    /// The artifact carries both exceedance figures, and Q's is the one that is not 10%. A reader
    /// quoting Q without it is quoting a number whose *rate* was never validated.
    #[test]
    fn the_artifact_carries_the_uncalibrated_figure_for_q() {
        let held_out = &load_trajectory_model(FACE_ON).leave_one_player_out_exceedance;
        let (target, t2, q) = (held_out["target"], held_out["t2"], held_out["q"]);
        assert!(
            (t2 - target).abs() < 0.01,
            "T2 hits the target, which is what `calibrated: true` means"
        );
        assert!(
            q > target + 0.02,
            "Q misses it by enough that its rate is not validated: {q} against {target}"
        );
    }

    /// A vector of the wrong width is refused rather than projected onto a basis it does not match:
    /// the arithmetic below would happily produce a plausible number from a truncated one.
    #[test]
    fn a_vector_of_the_wrong_width_is_refused() {
        let model = load_trajectory_model(FACE_ON);
        assert!(model.placement_from_vector(&[0.0; 3]).is_none());
        assert!(model
            .placement_from_vector(&vec![0.0; model.dimensions - 1])
            .is_none());
    }

    /// A swing sitting exactly on the fitted mean has no distance inside the subspace and none off
    /// it — and the all-zero residual takes the `grand <= 0` arm, which is the only place the
    /// interval shares are not a ratio.
    #[test]
    fn a_swing_at_the_mean_has_no_distance_and_no_residual() {
        let model = load_trajectory_model(FACE_ON);
        let placement = model
            .placement_from_vector(&model.mean)
            .expect("the mean is the right width");
        assert_eq!(placement.t2, 0.0);
        assert_eq!(placement.q, 0.0);
        assert!(placement
            .residual_by_interval
            .iter()
            .all(|(_, v)| *v == 0.0));
        // The degenerate arm keeps the artifact's own event order, because there is no sort at all.
        let names: Vec<&str> = placement.residual_by_interval.keys().collect();
        assert_eq!(names, vec!["address->top", "top->impact"]);
    }

    /// Shape the basis *can* represent lands in T² and leaves Q at zero; shape it cannot lands in Q.
    /// That split is the whole reason two numbers ship instead of one, so it is asserted rather than
    /// described.
    #[test]
    fn t2_reads_inside_the_subspace_and_q_reads_off_it() {
        let model = load_trajectory_model(FACE_ON);

        let inside: Vec<f64> = model
            .mean
            .iter()
            .zip(&model.basis[0])
            .map(|(m, b)| m + b * model.scale[0])
            .collect();
        let placed = model.placement_from_vector(&inside).unwrap();
        assert_eq!(placed.t2, 1.0, "one component's own sd is one sd of T2");
        // Not zero: the stored basis rows are orthonormal only to the precision they were written
        // at, so a pure component leaves ~1e-4 of residual. The point is the ratio — four orders of
        // magnitude below T2 — not an exact zero the artifact cannot give.
        assert!(placed.q <= 0.001, "q={}", placed.q);
    }

    /// Every interval share is a fraction of the squared residual and they sum to 1 — the property
    /// that makes "most of it falls in `top->impact`" a readable sentence rather than a raw total.
    #[test]
    fn the_interval_shares_are_a_partition_sorted_largest_first() {
        let model = load_trajectory_model(FACE_ON);
        // Residual concentrated in the second half of the timeline: a shape the basis cannot fit,
        // placed late.
        let mut vector = model.mean.clone();
        let per_step = model.landmarks.len() * model.axes.len();
        for step in model.steps / 2..model.steps {
            for d in 0..per_step {
                vector[step * per_step + d] += if d % 2 == 0 { 0.4 } else { -0.4 };
            }
        }
        let placement = model.placement_from_vector(&vector).unwrap();
        let shares: Vec<f64> = placement
            .residual_by_interval
            .iter()
            .map(|(_, v)| *v)
            .collect();
        assert!(
            (shares.iter().sum::<f64>() - 1.0).abs() < 1e-9,
            "{shares:?}"
        );
        assert!(shares.windows(2).all(|w| w[0] >= w[1]), "{shares:?}");
        assert_eq!(
            placement.residual_by_interval.first_key(),
            Some("top->impact"),
            "the injected shape is all in the second half"
        );
    }

    #[test]
    fn the_percentile_clamps_at_both_stored_edges() {
        let q = load_trajectory_model(FACE_ON).t2_quantiles;
        assert_eq!(percentile(q.p10 - 1.0, q), (10.0, true));
        assert_eq!(percentile(q.p90 + 1.0, q), (90.0, true));
        let (mid, clamped) = percentile((q.p50 + q.p75) / 2.0, q);
        assert!(!clamped);
        assert!((mid - 62.5).abs() < 1e-9, "{mid}");
    }

    /// The second basis is a **different object** answering the same question, not the same fit with
    /// a flag on it — so the thing worth pinning is that its two distinguishing facts are true.
    ///
    /// It drops the lead arm, which is what the torso hides from behind, and it adds both ankles;
    /// and it was fitted on more swings than the face-on one. A wiring mistake that handed both
    /// slots the same JSON would pass every conformance vector on the face-on side and silently
    /// place the rear clip against the wrong population, so the check is on the artifacts rather
    /// than on the plumbing.
    #[test]
    fn the_two_bases_are_different_fits_and_not_one_with_a_view_flag() {
        let face_on = load_trajectory_model(FACE_ON);
        let dtl = load_trajectory_model(DOWN_THE_LINE);
        assert!(face_on.landmarks.iter().any(|name| name == "left_wrist"));
        assert!(
            !dtl.landmarks.iter().any(|name| name == "left_wrist"),
            "the down-the-line fit drops the lead arm: {:?}",
            dtl.landmarks
        );
        assert!(dtl.landmarks.iter().any(|name| name == "left_ankle"));
        assert!(!face_on.landmarks.iter().any(|name| name == "left_ankle"));
        assert_eq!((face_on.n, dtl.n), (415, 510));
        // Same shape, so a vector built for one is the right *length* for the other. That is exactly
        // why the landmark check above is the load-bearing one: `placement_from_vector`'s only guard
        // is `len(vector) != dimensions`, and it would accept the wrong basis's vector.
        assert_eq!(face_on.dimensions, dtl.dimensions);
    }

    #[test]
    #[should_panic(expected = "no trajectory model for view \"overhead\"")]
    fn an_unfitted_view_is_a_wiring_bug() {
        load_trajectory_model("overhead");
    }
}
