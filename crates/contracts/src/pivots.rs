//! The swing as three moving points, and the seam a calibrated source will produce instead.
//! [M22 P5b]
//!
//! The port of `contracts/pivots.py`, and the last of the three registries [`crate`]'s doc says P2
//! left out. Like [`placements`](crate::placements) it lands with its walker — the `pivot_face_on`
//! group of the `measurements` stage — so the order, the units and the ten `detail` sentences below
//! are proved by committed vectors rather than by review.
//!
//! A **pivot observation** is one instant of the swing reduced to the three points the turn happens
//! about — the shoulder-line midpoint, the hip-line midpoint and the hands — plus the orientation
//! of the two lines through the first two. A **pivot measurement** is one number computed off the
//! whole path of those points.
//!
//! # Why the observation is a contract and not an implementation detail
//!
//! True rotation is 3-D and ADR-011 says this capture tier cannot recover it. So M17 ships an
//! explicitly interim instrument — 2-D, per view, never blended — behind the shape a *calibrated*
//! source would fill in. When the fiducial markers land, the calibrated producer becomes a second
//! producer of [`PivotObservation`] in [`FrameOfReference::Calibrated3d`], and the rule checks in
//! `analysis::pivot`, this registry and the overlay do not change. Only the `interim_reason` prose
//! flips (ADR-029 Decision 3).
//!
//! The consequence to preserve: **the rule checks read [`PivotObservation`] and never
//! `FrameKeypoints`.** A check typed over frames is a check only the pose producer can ever feed,
//! and the seam buys nothing.
//!
//! Nothing here is judged. These are `Measurement`s and not `CheckpointScore`s — no band, no
//! `passed`, no contribution to `overall_score` — because a rotation checkpoint needs a calibrated
//! instrument and a population, and M17 has neither (ADR-010 §2).

use std::ops::Range;

use serde::{Deserialize, Serialize};

use crate::placements::{DOWN_THE_LINE, FACE_ON};

/// How many samples one swing is resampled to before any check reads it.
///
/// **Odd, and that is a contract rather than a taste.** `analysis::trajectory::sample_positions`
/// walks event time as `t = span·i/(steps−1)`, which reaches the middle anchor at an integer `i`
/// only when `steps` is odd — so an even count puts the top of the backswing *between* two samples
/// and the two spans below could not meet on it. 41 is the odd neighbour of the fitted trajectory
/// models' 40 steps, so the two resamplings read the swing at the same temporal resolution.
pub const PIVOT_SAMPLES: usize = 41;

/// Address through the top, as indices into a [`PivotObservation`] list.
///
/// Event time is the index space and the anchors are integers in it: sample 0 is address, the
/// middle sample is the top, the last is impact. That is why the checks need no phase lookup — and
/// why they must not attempt one. `measure::phase_bounds` returns *frame* indices off a
/// `[PhaseSegment]`; these are sample indices off a resampled path, the down-the-line view has no
/// phases at all, and the two index spaces look alike enough to be swapped by accident.
///
/// A function rather than a `const`, because [`Range`] is not [`Copy`] and a `const` one would be
/// silently cloned at each use — which reads as though the span were being consumed. Python's
/// `range` object is re-iterable and this is the closest spelling of that.
///
/// The top belongs to both spans: a reversal is a move *between* consecutive samples, so the pair
/// spanning the top is the backswing's last and the downswing's first.
pub fn backswing_span() -> Range<usize> {
    0..PIVOT_SAMPLES / 2 + 1
}

/// The top through impact. See [`backswing_span`].
pub fn downswing_span() -> Range<usize> {
    PIVOT_SAMPLES / 2..PIVOT_SAMPLES
}

/// Which space a [`PivotObservation`]'s coordinates live in.
///
/// The seam, in one field. Two image-plane values ship today and carry no metric scale: an
/// uncalibrated phone's pixels, foreshortened by wherever it happened to stand.
/// [`Calibrated3d`](FrameOfReference::Calibrated3d) is what a fiducial-calibrated source will
/// declare, and the only value under which a rotation number could ever earn a band.
///
/// Two image-plane members rather than one because a face-on and a down-the-line observation are
/// **two scales that must never be blended** — the same rule the `_dtl` name suffix carries into
/// the registry below.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FrameOfReference {
    ImagePlaneFaceOn,
    ImagePlaneDtl,
    // Spelled out, because serde's `snake_case` rule writes `calibrated3d` — it does not insert a
    // separator before a digit, and Python's member value has one. The one member no vector can
    // catch, since nothing produces it yet.
    #[serde(rename = "calibrated_3d")]
    Calibrated3d,
}

impl FrameOfReference {
    /// The wire spelling, for the one refusal sentence that names it.
    ///
    /// Python's `StrEnum` makes `str(value)` the member's value; Rust's `Display` would have to be
    /// written anyway, so it is written once here rather than at the `format!` site.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ImagePlaneFaceOn => "image_plane_face_on",
            Self::ImagePlaneDtl => "image_plane_dtl",
            Self::Calibrated3d => "calibrated_3d",
        }
    }
}

impl std::fmt::Display for FrameOfReference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One resampled instant of the swing: three points, two line orientations, one space.
///
/// A plain struct for the same reason Python makes it a `NamedTuple` rather than a `BaseModel` — it
/// is arithmetic, not a boundary shape; nothing parses one from JSON, so it carries no
/// [`crate::Validate`] impl. A swing is [`PIVOT_SAMPLES`] of these in event time.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PivotObservation {
    /// Shoulder-line midpoint, `(x, y)` in [`frame_of_reference`](Self::frame_of_reference).
    pub shoulder: (f64, f64),
    /// Hip-line midpoint. The best-tracked of the three.
    pub hip: (f64, f64),
    /// Midpoint of both wrists, or `None` where the pair is tracked too sparsely to be bridged at
    /// all. Drawn by the overlay and deliberately unmeasured: M14 P3 put face-on hand tracking at
    /// 0.63-0.68 over a whole clip, so a through-swing hand-path number would report the tracker's
    /// noise as the golfer's.
    ///
    /// **Optional for exactly that reason, and it took a corpus run to see it** (M17 P5). The two
    /// line orientations below go `None` where the projection collapses; this one goes `None` where
    /// the tracker was never confident. When the three points shared one interpolation gate, the
    /// hands failing it refused the *whole swing* — the landmark nothing measures holding a veto
    /// over the five metrics that ignore it, and every face-on swing in the corpus recorded no
    /// rotation number at all. The shoulder and hip midpoints are never optional, because a swing
    /// that cannot produce them is not an observation.
    pub hands: Option<(f64, f64)>,
    /// Shoulder-line orientation as a **unit** vector, or `None` where the two landmarks collapsed
    /// within `measure::MIN_DIRECTION_LENGTH`.
    ///
    /// Not an angle in degrees, and a later reader will want to "simplify" it to one. A 2-D line
    /// angle here passes through a **projection singularity**: face-on the shoulder line is
    /// full-width at address and collapses toward zero width at the top, down-the-line it is the
    /// reverse, and `atan2` is worst-conditioned exactly where the segment is shortest — which is
    /// also where MediaPipe is estimating an occluded shoulder. A reversal check reading raw
    /// degrees fires hardest on the *cleanest* turns. Degrees are derived once, at the measurement
    /// boundary.
    pub shoulder_line: Option<(f64, f64)>,
    /// Hip-line orientation, same rule.
    pub hip_line: Option<(f64, f64)>,
    /// Which space the coordinates above are in. Every consumer that compares two observations must
    /// check this first — two frames of reference are two instruments.
    pub frame_of_reference: FrameOfReference,
    /// Left for a future club detector to write. Nothing in M17 populates it, no measurement reads
    /// it, and the overlay draws no club: the one detection spike (ADR-017) was a no-go on exposure
    /// time. The slot is here so that detector needs no contract change.
    pub club_head: Option<(f64, f64)>,
}

/// One pivot measurement's identity, and the facts that decide how it may be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PivotMeasurementSpec {
    /// The `Measurement.name` this is recorded under. The `_dtl` suffix is part of the name and not
    /// a `view` field, for [`crate::placements::PlacementSpec`]'s reason: `pooled_samples` groups a
    /// golfer's history by name, and one name for two cameras would pool two scales into one
    /// baseline.
    ///
    /// The `pivot_` prefix is load-bearing in the small way that matters in an append-only
    /// registry: a reader scanning a measurement list should not need a lookup to see what a row is.
    pub name: &'static str,
    /// The same quantity in a sentence, for prose a model reads.
    pub label: &'static str,
    /// Which camera produced it — [`FACE_ON`] or [`DOWN_THE_LINE`]. Never compare a value against
    /// the other view's.
    pub view: &'static str,
    /// `Measurement.unit`, so the registry and the emitted measurement cannot disagree.
    pub unit: &'static str,
    /// `Measurement.detail` — how and where it was taken, and the sign convention. Fixed prose, and
    /// that is the difference from [`crate::placements::PlacementSpec`], whose detail is written at
    /// the engine call site because it interpolates a percentile. Nothing here varies per swing.
    pub detail: &'static str,
    /// The unsuffixed key of the rule in `analysis::pivot::check_for` that computes it. The face-on
    /// and `_dtl` members of a pair name the **same** check: the view decides the name and the check
    /// does not know which camera it is reading, which is what lets one implementation serve two
    /// views without string surgery on the `_dtl` suffix.
    pub check: &'static str,
    /// The sentence a calibrated producer would clear — why this number is provisional, said in
    /// terms of what is missing rather than as a hedge. Derived into caveat prose rather than typed
    /// into it, so growing the registry cannot leave the prose behind.
    pub interim_reason: &'static str,
}

// The two positional metrics are ratios of a horizontal excursion to a horizontal shoulder width,
// so the frame's pixel aspect cancels; what does not cancel is the foreshortening, which changes
// with where the phone stood. Said once here and quoted by four specs.
const POSITIONAL_INTERIM: &str = "measured in one uncalibrated phone's image plane: the ratio \
cancels the pixel aspect but not the foreshortening, which changes with where the camera stood, so \
it compares within a camera position and not across sessions or views. Fiducial calibration clears \
it.";

// The angle metrics carry the projection singularity as well, and it sits at opposite ends of the
// swing in the two views — which is why each view states its own half. This narrows the general
// "down-the-line sees rotation best" reading, which holds for the centre paths and not for these.
const ANGLE_INTERIM_FACE_ON: &str = "a projected 2-D line angle, not a turn: face-on the shoulder \
line collapses toward zero width at the top of the backswing, so this reads the half of the swing \
its camera sees squarely and is worst-conditioned at the other end. Its down-the-line partner is \
conditioned the opposite way; read the pair, never one alone. Fiducial calibration clears it.";

const ANGLE_INTERIM_DTL: &str =
    "a projected 2-D line angle, not a turn: down-the-line the shoulder \
line is collapsed at address and opens through the turn, so this reads the half of the swing its \
camera sees squarely and is worst-conditioned at the other end. Its face-on partner is conditioned \
the opposite way; read the pair, never one alone. Fiducial calibration clears it.";

/// Every pivot measurement that ships, in the order `analysis::engine` records them: the face-on
/// view, then the down-the-line one, matching how [`crate::placements`] groups its own.
///
/// Five checks, ten metrics. Backswing and downswing reversal are two names rather than one because
/// `pooled_samples` groups by name — one name for two windows pools two distributions, and a firing
/// value could not say which half of the swing produced it.
///
/// Jitter is on the hip centre only. It is the best-tracked of the three pivot points; the hands are
/// drawn and unmeasured, on purpose (see [`PivotObservation::hands`]).
pub static PIVOT_MEASUREMENT_REGISTRY: &[PivotMeasurementSpec] = &[
    PivotMeasurementSpec {
        name: "pivot_hip_axis_drift_norm",
        label: "hip centre drift off its axis",
        view: FACE_ON,
        unit: "shoulder_widths",
        detail: "peak lateral excursion of the hip centre from its address x, over the resampled \
                 swing (address -> impact); unsigned, so handedness does not enter it",
        check: "hip_axis_drift",
        interim_reason: POSITIONAL_INTERIM,
    },
    PivotMeasurementSpec {
        name: "pivot_shoulder_axis_drift_norm",
        label: "shoulder centre drift off its axis",
        view: FACE_ON,
        unit: "shoulder_widths",
        detail: "peak lateral excursion of the shoulder centre from its address x, over the \
                 resampled swing (address -> impact); unsigned",
        check: "shoulder_axis_drift",
        interim_reason: POSITIONAL_INTERIM,
    },
    PivotMeasurementSpec {
        name: "pivot_hip_path_jitter_norm",
        label: "hip centre path roughness",
        view: FACE_ON,
        unit: "shoulder_widths",
        detail: "sample-to-sample roughness of the hip centre's path over the resampled swing; \
                 unsigned, 0 is a perfectly smooth path",
        check: "hip_path_jitter",
        interim_reason: POSITIONAL_INTERIM,
    },
    PivotMeasurementSpec {
        name: "pivot_shoulder_reversal_backswing_deg",
        label: "shoulder line reversal in the backswing",
        view: FACE_ON,
        unit: "degrees",
        detail: "largest against-the-turn move of the shoulder line between consecutive samples \
                 within the backswing (address -> top); magnitude only, 0 is a monotone turn",
        check: "shoulder_reversal_backswing",
        interim_reason: ANGLE_INTERIM_FACE_ON,
    },
    PivotMeasurementSpec {
        name: "pivot_shoulder_reversal_downswing_deg",
        label: "shoulder line reversal in the downswing",
        view: FACE_ON,
        unit: "degrees",
        detail: "largest against-the-turn move of the shoulder line between consecutive samples \
                 within the downswing (top -> impact); magnitude only, 0 is a monotone turn",
        check: "shoulder_reversal_downswing",
        interim_reason: ANGLE_INTERIM_FACE_ON,
    },
    PivotMeasurementSpec {
        name: "pivot_hip_axis_drift_norm_dtl",
        label: "down-the-line hip centre drift off its axis",
        view: DOWN_THE_LINE,
        unit: "shoulder_widths",
        detail: "peak lateral excursion of the hip centre from its address x, over the resampled \
                 swing (address -> impact); unsigned",
        check: "hip_axis_drift",
        interim_reason: POSITIONAL_INTERIM,
    },
    PivotMeasurementSpec {
        name: "pivot_shoulder_axis_drift_norm_dtl",
        label: "down-the-line shoulder centre drift off its axis",
        view: DOWN_THE_LINE,
        unit: "shoulder_widths",
        detail: "peak lateral excursion of the shoulder centre from its address x, over the \
                 resampled swing (address -> impact); unsigned",
        check: "shoulder_axis_drift",
        interim_reason: POSITIONAL_INTERIM,
    },
    PivotMeasurementSpec {
        name: "pivot_hip_path_jitter_norm_dtl",
        label: "down-the-line hip centre path roughness",
        view: DOWN_THE_LINE,
        unit: "shoulder_widths",
        detail: "sample-to-sample roughness of the hip centre's path over the resampled swing; \
                 unsigned, 0 is a perfectly smooth path",
        check: "hip_path_jitter",
        interim_reason: POSITIONAL_INTERIM,
    },
    PivotMeasurementSpec {
        name: "pivot_shoulder_reversal_backswing_deg_dtl",
        label: "down-the-line shoulder line reversal in the backswing",
        view: DOWN_THE_LINE,
        unit: "degrees",
        detail: "largest against-the-turn move of the shoulder line between consecutive samples \
                 within the backswing (address -> top); magnitude only, 0 is a monotone turn",
        check: "shoulder_reversal_backswing",
        interim_reason: ANGLE_INTERIM_DTL,
    },
    PivotMeasurementSpec {
        name: "pivot_shoulder_reversal_downswing_deg_dtl",
        label: "down-the-line shoulder line reversal in the downswing",
        view: DOWN_THE_LINE,
        unit: "degrees",
        detail: "largest against-the-turn move of the shoulder line between consecutive samples \
                 within the downswing (top -> impact); magnitude only, 0 is a monotone turn",
        check: "shoulder_reversal_downswing",
        interim_reason: ANGLE_INTERIM_DTL,
    },
];

/// Registered pivot measurement names, in the order they are recorded.
pub fn pivot_measurement_names() -> Vec<&'static str> {
    PIVOT_MEASUREMENT_REGISTRY
        .iter()
        .map(|spec| spec.name)
        .collect()
}

/// Is this measurement name a pivot row? `api/state.py::resolve_pivots` and `feedback/coach.py`
/// partition a whole measurement list through it, which is a data question.
pub fn pivot_by_name(name: &str) -> Option<&'static PivotMeasurementSpec> {
    PIVOT_MEASUREMENT_REGISTRY
        .iter()
        .find(|spec| spec.name == name)
}

/// The spec registered under `name`.
///
/// Panics rather than returning [`Option`], matching [`crate::placements::spec_for`] and
/// [`crate::checkpoints::spec_for`]: a caller here holds a name that came out of the registry, so a
/// miss is a wiring bug and not a data condition. Use [`pivot_by_name`] for the data question.
pub fn spec_for(name: &str) -> &'static PivotMeasurementSpec {
    pivot_by_name(name).unwrap_or_else(|| panic!("no pivot measurement registered under {name:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sample_count_is_odd_so_the_top_lands_on_a_sample() {
        assert_eq!(PIVOT_SAMPLES % 2, 1);
        assert_eq!(backswing_span().end - 1, downswing_span().start);
    }

    /// The top belongs to both spans, and each is the same length — the property the reversal
    /// checks' `unoriented / len(span)` coverage floor is measured against.
    #[test]
    fn the_two_spans_share_the_top_and_cover_every_sample() {
        let back = backswing_span();
        let down = downswing_span();
        assert_eq!(back.len(), down.len());
        assert_eq!(back.start, 0);
        assert_eq!(down.end, PIVOT_SAMPLES);
        assert_eq!(back.end, down.start + 1);
    }

    #[test]
    fn registry_order_is_face_on_then_down_the_line() {
        let views: Vec<&str> = PIVOT_MEASUREMENT_REGISTRY
            .iter()
            .map(|spec| spec.view)
            .collect();
        assert_eq!(
            views,
            vec![FACE_ON; 5]
                .into_iter()
                .chain(vec![DOWN_THE_LINE; 5])
                .collect::<Vec<_>>()
        );
    }

    /// Five checks, ten metrics: each face-on spec has a `_dtl` partner naming the *same* check.
    /// A registry that broke this would need string surgery on the suffix somewhere downstream,
    /// which is the thing `check` exists to avoid.
    #[test]
    fn every_face_on_spec_has_a_dtl_partner_on_the_same_check() {
        for spec in PIVOT_MEASUREMENT_REGISTRY
            .iter()
            .filter(|s| s.view == FACE_ON)
        {
            let partner = spec_for(&format!("{}_dtl", spec.name));
            assert_eq!(partner.check, spec.check);
            assert_eq!(partner.unit, spec.unit);
            assert_eq!(partner.view, DOWN_THE_LINE);
        }
        let checks: std::collections::BTreeSet<&str> = PIVOT_MEASUREMENT_REGISTRY
            .iter()
            .map(|spec| spec.check)
            .collect();
        assert_eq!(checks.len(), 5, "five checks, ten metrics");
    }

    #[test]
    fn spec_for_finds_every_registered_name() {
        for spec in PIVOT_MEASUREMENT_REGISTRY {
            assert_eq!(spec_for(spec.name), spec);
        }
    }

    #[test]
    #[should_panic(expected = "no pivot measurement registered under \"pivot_x_ray\"")]
    fn spec_for_refuses_an_unregistered_name() {
        spec_for("pivot_x_ray");
    }

    /// The wire spellings are what `conformance.py` recorded and what a calibrated producer will
    /// declare, so they are pinned rather than left to serde's renaming rule to be re-derived.
    #[test]
    fn the_frame_of_reference_spellings_are_the_python_enum_values() {
        for (value, spelled) in [
            (FrameOfReference::ImagePlaneFaceOn, "image_plane_face_on"),
            (FrameOfReference::ImagePlaneDtl, "image_plane_dtl"),
            (FrameOfReference::Calibrated3d, "calibrated_3d"),
        ] {
            assert_eq!(value.as_str(), spelled);
            assert_eq!(value.to_string(), spelled);
            assert_eq!(
                serde_json::to_string(&value).unwrap(),
                format!("\"{spelled}\"")
            );
        }
    }
}
