//! The swing as a path through time — the feature vector the trajectory model reads. [M22 P5b]
//!
//! The port of `analysis/trajectory.py`. [`crate::measure`] produces scalars at instants; this
//! produces the **shape of the whole motion**: every tracked landmark, resampled onto a normalised
//! event axis, hip-relative and shoulder-width scaled. It is still measuring, not judging —
//! nothing here reads a band, a basis or a quantile, and [`crate::benchmarks::trajectory`] is what
//! turns the vector into a placement.
//!
//! # Two resamplers, one rule about bridging
//!
//! [`crate::pivot`] is the second reader of this module and it takes [`sample_positions`] and
//! [`interpolate_gaps`] rather than keeping copies. The Python marks the second one private by name
//! (`_interpolate_gaps`) and shares it by intent, on the grounds that it is *the rule for how much
//! of a landmark's timeline may be bridged before the bridge is an invention* — a second
//! [`MAX_MISSING`] would be a second thing to keep in step. `pub(crate)` is the Rust spelling of
//! exactly that: visible to its one sibling, invisible outside the crate.
//!
//! # Its constants are its own, and that is deliberate
//!
//! [`MIN_VISIBILITY`] and [`MIN_SHOULDER_WIDTH`] here hold the same values as
//! [`crate::measure`]'s and are **not** imports of them — the Python declares its own pair too.
//! `crate::pivot` does the opposite and imports `measure`'s. Reproduced rather than tidied: these
//! are dials over different instruments (a resampled event-time path against a per-frame window),
//! and merging them would make a retune of one silently move the other. `docs/CODE_STANDARDS.md`
//! treats a shared *value* as weaker evidence than a shared *reason*.

use contracts::keypoints::{FrameKeypoints, PoseLandmark};
use contracts::swing::{PhaseSegment, SwingPhase};

/// Landmark name -> index, for the fourteen the two fitted models track between them.
///
/// Names rather than raw indices because the shipped artifact stores names: an artifact that said
/// `[7, 8, 11, …]` would be unreadable, and worse, would silently survive a landmark being swapped
/// out from under it.
///
/// **A slice rather than a map**, for [`crate::measure::POSE_MEASUREMENTS`]'s reason in reverse: the
/// order here is *not* an answer (nothing iterates it — [`build_trajectory`] looks up the model's
/// own landmark list), so the only requirement is that a name resolves to the same landmark it does
/// in Python. Fourteen linear comparisons per model load is not a cost worth a `HashMap` and its
/// non-deterministic iteration.
pub static LANDMARK_INDEX: &[(&str, PoseLandmark)] = &[
    ("left_ear", PoseLandmark::LeftEar),
    ("right_ear", PoseLandmark::RightEar),
    ("left_shoulder", PoseLandmark::LeftShoulder),
    ("right_shoulder", PoseLandmark::RightShoulder),
    ("left_elbow", PoseLandmark::LeftElbow),
    ("right_elbow", PoseLandmark::RightElbow),
    ("left_wrist", PoseLandmark::LeftWrist),
    ("right_wrist", PoseLandmark::RightWrist),
    ("left_hip", PoseLandmark::LeftHip),
    ("right_hip", PoseLandmark::RightHip),
    ("left_knee", PoseLandmark::LeftKnee),
    ("right_knee", PoseLandmark::RightKnee),
    // Ankles are here for the down-the-line set rather than the face-on one. From behind, the lead
    // arm is hidden by the torso (elbow and wrist track in under half of frames,
    // M4_POSE_BAKEOFF §Phase G), so that model drops the lead arm and takes the feet instead —
    // visible in ~1.00 of frames from either camera. A name absent from this map makes
    // `build_trajectory` return None for every swing, which is silent rather than loud, so the map
    // must cover the union of every view's list and not just the one that came first.
    ("left_ankle", PoseLandmark::LeftAnkle),
    ("right_ankle", PoseLandmark::RightAnkle),
];

/// This module's own visibility gate. See the crate-doc note above on why it is not
/// [`crate::measure::MIN_VISIBILITY`].
pub const MIN_VISIBILITY: f64 = 0.5;
/// This module's own degenerate-width floor. Same note.
pub const MIN_SHOULDER_WIDTH: f64 = 0.02;
/// A landmark missing more of its timeline than this is being invented rather than bridged.
pub const MAX_MISSING: f64 = 0.40;

/// The landmark `name` refers to, or `None` if this pipeline does not track it.
fn landmark_index(name: &str) -> Option<PoseLandmark> {
    LANDMARK_INDEX
        .iter()
        .find(|(key, _)| *key == name)
        .map(|(_, index)| *index)
}

/// The three instants the model is anchored on — address, top, impact — or `None`.
///
/// Recovered from the boundary chain `segment_phases` builds rather than re-detected:
///
/// - **address** is the end of the ADDRESS segment, which *is* `motion_start` — the frame the
///   sustained takeaway begins, and the closest thing this pipeline has to GolfDB's "address".
/// - **top** is the midpoint of TRANSITION, which brackets the detected top symmetrically. Reading
///   the midpoint rather than a boundary means the clamping at either end (against motion start or
///   against impact) degrades the estimate instead of destroying it.
/// - **impact** is the start of the IMPACT segment, which *is* the detected impact frame.
///
/// The other four events GolfDB annotates — toe-up, mid-backswing, mid-downswing,
/// mid-follow-through — are deliberately not here. This pipeline does not detect them, and a model
/// anchored on instants it cannot produce could never score a real swing.
///
/// **The name is shared with `alignment::anchors_from_phases` and they answer different
/// questions.** That one builds a `SwingAnchors` for two clips to be warped onto each other; this
/// one returns the three fractional frame positions the resamplers read. `engine.py` imports this
/// one under an alias for that reason, and the crate-relative path does the same job here.
pub fn anchors_from_phases(phases: &[PhaseSegment]) -> Option<(f64, f64, f64)> {
    // `find` rather than a name->segment map, because Python builds one only to read three keys and
    // the list is six entries long.
    //
    // **It takes the first segment of each phase where Python's dict comprehension takes the last**,
    // and the note here first claimed they agreed — corrected in M22 P6, which ported the same lookup
    // into `alignment::anchors_from_phases` and had to state the rule to write it. The two differ only
    // on a phase list naming one phase twice, which `segment_phases` cannot produce: it emits the six
    // phases once each in canonical order, and every caller passes its output unchanged or shifted. So
    // they agree on the whole reachable domain, which is the standard P5 and P5b set for a deliberate
    // divergence that survives — but it is an equivalence with a domain, not an identity.
    let of = |phase: SwingPhase| phases.iter().find(|segment| segment.phase == phase);
    let address = of(SwingPhase::Address)?;
    let transition = of(SwingPhase::Transition)?;
    let impact = of(SwingPhase::Impact)?;

    let anchors = (
        address.end_frame as f64,
        (transition.start_frame + transition.end_frame) as f64 / 2.0,
        impact.start_frame as f64,
    );
    // Strictly increasing or the resampling below divides by zero and smears one phase into
    // another. A collapsed window means detection failed, which is a refusal rather than a fixup.
    if !(anchors.0 < anchors.1 && anchors.1 < anchors.2) {
        return None;
    }
    Some(anchors)
}

/// Frame positions for `steps` samples evenly spaced in *event* time, not clock time.
///
/// Event time runs `0..anchors.len()-1` with the integers landing on the anchors; each sample maps
/// back to a fractional frame by interpolating between the two anchors bracketing it. A
/// slow-motion clip and a real-time clip of the same swing therefore yield the same sample
/// positions — which is the only reason a corpus that is ~47% slow-motion can be pooled at all,
/// and the reason a phone at 60fps can be compared to broadcast footage.
///
/// `t` is `span * i / (steps - 1)` in exactly that association order, and **today that is a
/// transcription rather than a requirement** — which is worth writing down, because the opposite
/// looks true. `span` is 2 at every call site in this crate (three anchors, both resamplers), and
/// scaling by a power of two is exact in IEEE 754, so `(2*i)/d` and `2*(i/d)` are bit-identical.
/// Mutating one into the other passes all 21 vectors and every test here, and that is correct
/// rather than a hole. It stops being true the day a model is fitted on four anchors: `span` is
/// then 3, the two forms diverge in the last bits, and `low` can pick a different bracket at a
/// boundary. Kept in the Python's order so that day costs nothing.
pub fn sample_positions(anchors: &[f64], steps: usize) -> Vec<f64> {
    let span = anchors.len() - 1;
    (0..steps)
        .map(|i| {
            let t = (span * i) as f64 / (steps - 1) as f64;
            let low = (t as usize).min(span - 1);
            anchors[low] + (t - low as f64) * (anchors[low + 1] - anchors[low])
        })
        .collect()
}

/// Bridge low-visibility gaps per column, or `None` if any column is too holey to trust.
///
/// `pub(crate)` rather than private: [`crate::pivot`] is the second resampler in this package and
/// shares the rule rather than restating it. See the module doc.
pub(crate) fn interpolate_gaps(columns: &[Vec<Option<f64>>]) -> Option<Vec<Vec<f64>>> {
    let mut filled: Vec<Vec<f64>> = Vec::with_capacity(columns.len());
    for column in columns {
        let known: Vec<(usize, f64)> = column
            .iter()
            .enumerate()
            .filter_map(|(i, v)| v.map(|v| (i, v)))
            .collect();
        if known.is_empty()
            || (column.len() - known.len()) as f64 / column.len() as f64 > MAX_MISSING
        {
            return None;
        }
        let mut out: Vec<f64> = Vec::with_capacity(column.len());
        for (i, value) in column.iter().enumerate() {
            if let Some(value) = value {
                out.push(*value);
                continue;
            }
            // The Python rebuilds `before`/`after` per hole; this walks the sorted `known` list
            // instead. Same two neighbours, and the arithmetic below is identical — the difference
            // is quadratic work, not a different answer.
            let after = known.partition_point(|(k, _)| *k < i);
            match (after.checked_sub(1).map(|j| known[j]), known.get(after)) {
                // Leading gap: hold the first known value.
                (None, Some((_, first))) => out.push(*first),
                // Trailing gap: hold the last known value.
                (Some((_, last)), None) => out.push(last),
                (Some((lo_i, lo_v)), Some((hi_i, hi_v))) => {
                    out.push(lo_v + (i - lo_i) as f64 / (hi_i - lo_i) as f64 * (hi_v - lo_v));
                }
                // `known` is non-empty, so one of the two sides always exists.
                (None, None) => unreachable!("a column with no known values was refused above"),
            }
        }
        filled.push(out);
    }
    Some(filled)
}

/// One swing as a flat `steps * landmarks.len() * axes.len()` vector, or `None` if unmeasurable.
///
/// Row-major by timestep, so the vector reads as "the whole body at t=0, then t=1, …" and a slice
/// of it is a moment rather than a landmark's history. The shipped basis is fitted in this order;
/// changing it silently re-pairs every coefficient with the wrong coordinate.
///
/// `mirror` folds a left-handed swing onto the right-handed convention a face-on camera implies.
/// That is a sign flip on `x` **and** a swap of every left/right pair — flipping without swapping
/// would put the lead arm on the trail side and score the golfer against a shape nobody swings.
///
/// # The one Python behaviour not reproduced, and why it is unreachable
///
/// Python guards `positions[0] < 0` and then indexes with `int(position)`, so a *later* negative
/// position would index from the end of the list rather than raise. It cannot happen: both callers
/// pass strictly increasing anchors (checked in [`anchors_from_phases`] and in
/// `pivot::pivot_observations`), which makes `positions` monotone, so `positions[0]` is the
/// minimum. Rust clamps instead of wrapping, which is the same answer on every reachable input and
/// a saner one on the input neither language should get.
pub fn build_trajectory(
    keypoints: &[FrameKeypoints],
    anchors: &[f64],
    steps: usize,
    landmarks: &[String],
    axes: &[String],
    mirror: bool,
) -> Option<Vec<f64>> {
    if keypoints.len() < 2 || steps < 2 {
        return None;
    }

    let indices: Vec<PoseLandmark> = landmarks
        .iter()
        .map(|name| landmark_index(name))
        .collect::<Option<_>>()?;

    let positions = sample_positions(anchors, steps);
    if positions[0] < 0.0 || *positions.last()? > (keypoints.len() - 1) as f64 {
        return None;
    }

    let mut columns: Vec<Vec<Option<f64>>> =
        vec![Vec::with_capacity(steps); indices.len() * axes.len()];
    let mut widths: Vec<f64> = Vec::new();

    for position in &positions {
        let low = (*position as usize).min(keypoints.len() - 1);
        let high = (low + 1).min(keypoints.len() - 1);
        let frac = position - low as f64;

        // Both bracketing frames must clear the gate: interpolating from one good and one guessed
        // landmark produces a plausible number, which is worse than no sample at all.
        let read = |index: PoseLandmark, axis: &str| -> Option<f64> {
            let a = keypoints[low].landmark(index);
            let b = keypoints[high].landmark(index);
            if a.visibility.min(b.visibility) < MIN_VISIBILITY {
                return None;
            }
            let (a, b) = match axis {
                "x" => (a.x, b.x),
                "y" => (a.y, b.y),
                "z" => (a.z, b.z),
                // `getattr(a, axis)` raises on anything else, and the axis list comes off a fitted
                // artifact. A panic says the same thing at the same moment.
                other => panic!("a trajectory model asked for an axis named {other:?}"),
            };
            Some(a * (1.0 - frac) + b * frac)
        };

        let origin: Vec<Option<f64>> = axes
            .iter()
            .map(|axis| {
                let left = read(PoseLandmark::LeftHip, axis)?;
                let right = read(PoseLandmark::RightHip, axis)?;
                Some((left + right) / 2.0)
            })
            .collect();

        if let (Some(left), Some(right)) = (
            read(PoseLandmark::LeftShoulder, "x"),
            read(PoseLandmark::RightShoulder, "x"),
        ) {
            let width = (left - right).abs();
            if width >= MIN_SHOULDER_WIDTH {
                widths.push(width);
            }
        }

        for (i, index) in indices.iter().enumerate() {
            for (j, axis) in axes.iter().enumerate() {
                let value = read(*index, axis);
                let base = origin[j];
                columns[i * axes.len() + j].push(match (value, base) {
                    (Some(value), Some(base)) => Some(value - base),
                    _ => None,
                });
            }
        }
    }

    if widths.is_empty() {
        return None;
    }
    // One ruler for the whole swing rather than one per frame. A per-frame width would renormalise
    // away the very torso motion this is trying to see, and is noisier besides.
    //
    // `sort_by` with a total order on f64: no width here can be NaN (it is `abs` of a difference of
    // two finite landmark coordinates), and `total_cmp` says so without a partial-order unwrap.
    widths.sort_by(f64::total_cmp);
    let scale = widths[widths.len() / 2];

    let scaled: Vec<Vec<Option<f64>>> = columns
        .iter()
        .map(|column| column.iter().map(|v| v.map(|v| v / scale)).collect())
        .collect();
    let mut filled = interpolate_gaps(&scaled)?;

    if mirror {
        filled = mirror_columns(&filled, landmarks, axes);
    }

    Some(
        (0..steps)
            .flat_map(|t| {
                (0..landmarks.len())
                    .flat_map(move |i| (0..axes.len()).map(move |j| (i * axes.len() + j, t)))
            })
            .map(|(column, t)| filled[column][t])
            .collect(),
    )
}

/// Swap every left/right pair and negate `x`.
///
/// A name with neither prefix partners with itself, because `str.replace` leaves it unchanged and
/// `list.index` then finds it — so a model tracking `nose` would still have its `x` negated and
/// nothing swapped. Reproduced rather than special-cased: neither shipped artifact lists one, and
/// the day one does, the Python and this must still agree.
fn mirror_columns(columns: &[Vec<f64>], landmarks: &[String], axes: &[String]) -> Vec<Vec<f64>> {
    let partner: Vec<usize> = landmarks
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let other = match name.strip_prefix("left_") {
                Some(rest) => format!("right_{rest}"),
                None => match name.strip_prefix("right_") {
                    Some(rest) => format!("left_{rest}"),
                    None => name.clone(),
                },
            };
            // `list.index` is the *first* match, which matters only for a duplicated landmark name
            // — and costs nothing to get right.
            landmarks
                .iter()
                .position(|name| *name == other)
                .unwrap_or(i)
        })
        .collect();

    let mut out: Vec<Vec<f64>> = columns.to_vec();
    for i in 0..landmarks.len() {
        for (j, axis) in axes.iter().enumerate() {
            let source = &columns[partner[i] * axes.len() + j];
            out[i * axes.len() + j] = if axis == "x" {
                source.iter().map(|v| -v).collect()
            } else {
                source.clone()
            };
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{frame, phase, put};
    use contracts::swing::PhaseSegment;

    fn segment(kind: SwingPhase, start: i64, end: i64) -> PhaseSegment {
        phase(kind, start, end, true)
    }

    fn names(values: &[&str]) -> Vec<String> {
        values.iter().map(|s| s.to_string()).collect()
    }

    /// A still body with a 0.30 shoulder width and centred hips, read over `n` frames.
    ///
    /// `testing::a_body` is the shared fixture and is deliberately not reused here: it places the
    /// hips and shoulders where the *measuring* tests want them. These tests read an exact
    /// normalized coordinate back out of the vector, so the geometry they depend on is spelled out
    /// at the one place that depends on it.
    fn still(n: i64) -> Vec<FrameKeypoints> {
        (0..n)
            .map(|i| {
                let mut f = frame(i);
                put(&mut f, PoseLandmark::LeftHip, 0.4, 0.5, 1.0);
                put(&mut f, PoseLandmark::RightHip, 0.6, 0.5, 1.0);
                put(&mut f, PoseLandmark::LeftShoulder, 0.35, 0.3, 1.0);
                put(&mut f, PoseLandmark::RightShoulder, 0.65, 0.3, 1.0);
                f
            })
            .collect()
    }

    #[test]
    fn the_anchors_are_address_end_transition_midpoint_and_impact_start() {
        let phases = vec![
            segment(SwingPhase::Address, 0, 10),
            segment(SwingPhase::Backswing, 11, 29),
            segment(SwingPhase::Transition, 30, 35),
            segment(SwingPhase::Downswing, 36, 49),
            segment(SwingPhase::Impact, 50, 52),
        ];
        assert_eq!(anchors_from_phases(&phases), Some((10.0, 32.5, 50.0)));
    }

    #[test]
    fn a_missing_phase_refuses_rather_than_guessing() {
        let phases = vec![
            segment(SwingPhase::Address, 0, 10),
            segment(SwingPhase::Transition, 30, 35),
        ];
        assert_eq!(anchors_from_phases(&phases), None);
    }

    /// A collapsed detection window divides by zero downstream, so it is refused here. The
    /// *non-strict* case is the one worth pinning: equal anchors pass a `<=` test and fail a `<`.
    #[test]
    fn anchors_that_do_not_strictly_increase_are_refused() {
        let phases = vec![
            segment(SwingPhase::Address, 0, 30),
            segment(SwingPhase::Transition, 28, 32),
            segment(SwingPhase::Impact, 30, 32),
        ];
        assert_eq!(anchors_from_phases(&phases), None);
    }

    /// The integers land on the anchors, which is the whole contract `PIVOT_SAMPLES` being odd
    /// depends on.
    #[test]
    fn an_odd_step_count_puts_a_sample_on_the_middle_anchor() {
        let positions = sample_positions(&[10.0, 40.0, 50.0], 5);
        assert_eq!(positions, vec![10.0, 25.0, 40.0, 45.0, 50.0]);
    }

    #[test]
    fn an_even_step_count_straddles_the_middle_anchor() {
        let positions = sample_positions(&[0.0, 10.0, 20.0], 4);
        assert!(!positions.contains(&10.0), "{positions:?}");
    }

    #[test]
    fn the_last_sample_lands_exactly_on_the_final_anchor() {
        for steps in [2, 3, 40, 41] {
            let positions = sample_positions(&[3.0, 17.0, 29.0], steps);
            assert_eq!(*positions.last().unwrap(), 29.0, "steps={steps}");
            assert_eq!(positions[0], 3.0, "steps={steps}");
        }
    }

    #[test]
    fn a_gap_between_two_known_values_is_interpolated_linearly() {
        let column = vec![Some(0.0), None, None, Some(3.0), Some(4.0), Some(5.0)];
        let filled = interpolate_gaps(&[column]).unwrap();
        assert_eq!(filled[0], vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0]);
    }

    #[test]
    fn leading_and_trailing_gaps_hold_the_nearest_known_value() {
        let column = vec![None, Some(2.0), Some(4.0), Some(6.0), Some(8.0), None];
        let filled = interpolate_gaps(&[column]).unwrap();
        assert_eq!(filled[0], vec![2.0, 2.0, 4.0, 6.0, 8.0, 8.0]);
    }

    /// The gate is `>`, not `>=`: exactly 40% missing is bridged and one sample more is not.
    #[test]
    fn a_column_missing_more_than_max_missing_refuses_the_whole_vector() {
        let at_the_limit: Vec<Option<f64>> = (0..10)
            .map(|i| if i < 4 { None } else { Some(i as f64) })
            .collect();
        assert!(interpolate_gaps(&[at_the_limit]).is_some());

        let past_it: Vec<Option<f64>> = (0..10)
            .map(|i| if i < 5 { None } else { Some(i as f64) })
            .collect();
        assert!(interpolate_gaps(&[past_it]).is_none());
    }

    #[test]
    fn a_column_with_nothing_known_refuses() {
        assert!(interpolate_gaps(&[vec![None, None]]).is_none());
    }

    /// One holey column refuses the vector even when every other column is complete — the property
    /// that makes all fifteen corpus swings refuse a face-on trajectory placement.
    #[test]
    fn one_holey_column_refuses_the_whole_set() {
        let good = vec![Some(1.0); 10];
        let bad: Vec<Option<f64>> = (0..10)
            .map(|i| if i < 5 { None } else { Some(1.0) })
            .collect();
        assert!(interpolate_gaps(&[good.clone(), bad]).is_none());
        assert!(interpolate_gaps(&[good.clone(), good]).is_some());
    }

    /// A body standing still, read at two landmarks over two axes: the vector is row-major by
    /// timestep, so a slice of it is a moment.
    #[test]
    fn the_vector_is_row_major_by_timestep() {
        let vector = build_trajectory(
            &still(4),
            &[0.0, 1.0, 3.0],
            3,
            &names(&["left_shoulder", "right_shoulder"]),
            &names(&["x", "y"]),
            false,
        )
        .expect("a still body is measurable");
        assert_eq!(vector.len(), 3 * 2 * 2);
        // Shoulder width is 0.30, so the lead shoulder sits at (0.35-0.5)/0.30 = -0.5.
        assert!((vector[0] - -0.5).abs() < 1e-12, "{vector:?}");
        assert!((vector[2] - 0.5).abs() < 1e-12, "{vector:?}");
        // Every timestep of a still body is the same moment.
        assert_eq!(vector[0..4], vector[4..8]);
    }

    /// **The ruler is the median per-sample shoulder width, and it is the `len/2` median.**
    ///
    /// Not the mean, which one wide sample drags; and not `(len - 1) / 2`, which is the *lower*
    /// median and a different number whenever the count is even. Both are one character away and
    /// neither is caught by any committed vector — the six synthetic swings have a constant
    /// shoulder width, so all three rules agree on them, and the fifteen corpus swings never reach
    /// this line at all (see `tests/measurements.rs`). So this test is the only thing standing
    /// between a port and a silently rescaled trajectory.
    ///
    /// Eleven frames with ten usable widths of 0.10..0.19 and an eleventh below the floor:
    /// upper median 0.15, lower median 0.14, mean 0.145, all distinct.
    #[test]
    fn the_ruler_is_the_upper_median_width_and_not_the_mean() {
        let frames: Vec<FrameKeypoints> = (0..11)
            .map(|i| {
                let mut f = frame(i);
                let width = if i == 10 {
                    // Below `MIN_SHOULDER_WIDTH`, so this sample contributes no width and the
                    // count is even — which is the only case the two medians differ on.
                    0.01
                } else {
                    0.10 + i as f64 * 0.01
                };
                put(&mut f, PoseLandmark::LeftHip, 0.4, 0.5, 1.0);
                put(&mut f, PoseLandmark::RightHip, 0.6, 0.5, 1.0);
                put(
                    &mut f,
                    PoseLandmark::LeftShoulder,
                    0.5 - width / 2.0,
                    0.3,
                    1.0,
                );
                put(
                    &mut f,
                    PoseLandmark::RightShoulder,
                    0.5 + width / 2.0,
                    0.3,
                    1.0,
                );
                f
            })
            .collect();
        let vector = build_trajectory(
            &frames,
            &[0.0, 5.0, 10.0],
            11,
            &names(&["left_shoulder"]),
            &names(&["x", "y"]),
            false,
        )
        .expect("eleven readable samples");
        // Sample 0's lead shoulder sits 0.05 left of the hip centre, over a ruler of 0.15.
        assert!(
            (vector[0] - -1.0 / 3.0).abs() < 1e-12,
            "the ruler moved: {} (mean would give {}, the lower median {})",
            vector[0],
            -0.05 / 0.145,
            -0.05 / 0.14,
        );
    }

    /// **Both bracketing frames must clear the gate, not either one.**
    ///
    /// Interpolating from one good and one guessed landmark produces a plausible number, which is
    /// worse than no sample at all. `min` is what says so; `max` reads every sample here and is
    /// invisible to every committed vector, because the synthetic swings are fully confident and
    /// the corpus ones refuse the vector for an unrelated reason.
    ///
    /// Dimming alternate frames means *every* sample brackets one dim frame, so the correct rule
    /// drops all but the last and refuses on [`MAX_MISSING`] — while `max` would read all eleven.
    /// The knee rather than a shoulder, so the shoulder-width ruler is not what fails.
    #[test]
    fn a_sample_needs_both_of_its_bracketing_frames() {
        let mut frames = still(11);
        for i in (1..11).step_by(2) {
            put(
                &mut frames[i],
                PoseLandmark::LeftKnee,
                0.5,
                0.8,
                MIN_VISIBILITY - 0.01,
            );
        }
        assert!(
            build_trajectory(
                &frames,
                &[0.0, 5.0, 10.0],
                11,
                &names(&["left_knee"]),
                &names(&["x", "y"]),
                false,
            )
            .is_none(),
            "a gate that took either frame would have read every sample"
        );
    }

    /// Mirroring is a sign flip on `x` **and** a left/right swap. Flipping alone would put the lead
    /// arm on the trail side, which is a shape nobody swings — so this asserts both halves, and a
    /// port that forgot the swap fails it.
    #[test]
    fn the_mirror_flips_x_and_swaps_the_pair() {
        let columns = vec![
            vec![-0.5, -0.5], // left_shoulder x
            vec![0.2, 0.2],   // left_shoulder y
            vec![0.5, 0.5],   // right_shoulder x
            vec![0.4, 0.4],   // right_shoulder y
        ];
        let mirrored = mirror_columns(
            &columns,
            &names(&["left_shoulder", "right_shoulder"]),
            &names(&["x", "y"]),
        );
        assert_eq!(mirrored[0], vec![-0.5, -0.5], "left x takes -(right x)");
        assert_eq!(mirrored[1], vec![0.4, 0.4], "left y takes right y");
        assert_eq!(mirrored[2], vec![0.5, 0.5], "right x takes -(left x)");
        assert_eq!(mirrored[3], vec![0.2, 0.2], "right y takes left y");
    }

    /// A landmark with neither prefix partners with itself and still has its `x` negated — Python's
    /// `str.replace` leaves it unchanged and `list.index` finds it. Nothing ships like this today;
    /// the test is what keeps the two languages agreeing the day something does.
    #[test]
    fn an_unpaired_landmark_mirrors_onto_itself() {
        let columns = vec![vec![0.3], vec![0.7]];
        let mirrored = mirror_columns(&columns, &names(&["nose"]), &names(&["x", "y"]));
        assert_eq!(mirrored[0], vec![-0.3]);
        assert_eq!(mirrored[1], vec![0.7]);
    }

    #[test]
    fn a_landmark_this_pipeline_does_not_track_refuses_the_vector() {
        assert!(build_trajectory(
            &still(4),
            &[0.0, 1.0, 3.0],
            3,
            &names(&["left_thumb"]),
            &names(&["x", "y"]),
            false,
        )
        .is_none());
    }

    /// Anchors reaching past the last frame refuse rather than clamping: a sample read off the end
    /// of the clip is an invention, and the Python's guard is on the *positions*, not the anchors.
    #[test]
    fn anchors_reaching_outside_the_clip_refuse() {
        assert!(build_trajectory(
            &still(4),
            &[0.0, 2.0, 9.0],
            3,
            &names(&["left_shoulder"]),
            &names(&["x", "y"]),
            false,
        )
        .is_none());
    }

    /// No sample produced a shoulder width above the floor, so there is no ruler and no vector —
    /// distinct from the too-holey refusal above, and reached by a golfer turned side-on.
    #[test]
    fn a_collapsed_shoulder_line_leaves_no_ruler() {
        let frames: Vec<FrameKeypoints> = (0..4)
            .map(|i| {
                let mut f = frame(i);
                put(&mut f, PoseLandmark::LeftHip, 0.5, 0.5, 1.0);
                put(&mut f, PoseLandmark::RightHip, 0.5, 0.5, 1.0);
                put(&mut f, PoseLandmark::LeftShoulder, 0.5, 0.3, 1.0);
                put(&mut f, PoseLandmark::RightShoulder, 0.505, 0.3, 1.0);
                f
            })
            .collect();
        assert!(build_trajectory(
            &frames,
            &[0.0, 1.0, 3.0],
            3,
            &names(&["left_shoulder"]),
            &names(&["x", "y"]),
            false,
        )
        .is_none());
    }

    /// An unconfident *bracketing* frame drops the sample even when the frame the position
    /// truncates to is confident — the rule `measure::midpoint_series` states and this one reuses.
    /// The column is then one sample short of complete, which is under `MAX_MISSING`, so it bridges
    /// rather than refusing: two different gates, and this pins that they are.
    #[test]
    fn an_unconfident_bracketing_frame_drops_the_sample_without_refusing_the_swing() {
        // Eleven samples over eleven frames, so a position lands on each frame; dimming frame 3
        // costs the two samples that bracket it, which is 2 of 11 and under `MAX_MISSING`.
        let mut frames = still(11);
        put(
            &mut frames[3],
            PoseLandmark::LeftShoulder,
            0.35,
            0.3,
            MIN_VISIBILITY - 0.01,
        );
        let vector = build_trajectory(
            &frames,
            &[0.0, 5.0, 10.0],
            11,
            &names(&["left_shoulder"]),
            &names(&["x", "y"]),
            false,
        )
        .expect("two dropped samples of eleven bridge rather than refuse");
        // Bridged, not invented: the still body reads -0.5 everywhere, gaps included.
        assert!(vector.iter().step_by(2).all(|x| (x - -0.5).abs() < 1e-12));
    }
}
