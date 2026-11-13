//! Orientation: which end of the label is the heel and which edge faces the top line, read off the
//! hosel and shaft in the photo. [M46 P10]
//!
//! # Why the club, and the hand only as a cross-check
//!
//! A reading's signs are the club's (the M46 plan's C4): `+x` toward the toe and `+y` toward the
//! top line, whichever hand the club is and however the photo was held. The label cannot give them.
//! It is printed one way and sold for either hand (P4 finding 5), and the finder finds it in its
//! printed frame at any turn (P9). So a label stuck logo-down looks, to the finder, like one stuck
//! logo-up. The club can give them. The hosel takes the shaft at the heel, and the shaft runs up from
//! there and away from the toe. Its side of the label is the heel and its direction is up, so it
//! settles both of [`Orientation`]'s flips. That makes a label stuck logo-down *read*, with both
//! flags flipped, rather than refused (P8 finding 3 left that to this phase).
//!
//! The golfer's hand is not a source, because a hand alone would assume the label was stuck logo-up.
//! It is a check. Seen from the front, the two flips fix which hand the club is
//! ([`Orientation::handedness`]), so a shaft read on the wrong side shows up as the other hand. A
//! disagreement is refused, and so is no shaft at all: a guessed side is a guessed sign (the plan's
//! rule 6). A caller with no golfer to ask (the desk tool) passes no hand, and the shaft alone decides.
//!
//! # How the shaft is found
//!
//! 1. **A window in the printed frame.** The label's box, grown by [`SEARCH_REACH_MM`] on every
//!    side, is cut out of the photo through the found label's map at [`ORIENT_PX_PER_MM`], as the
//!    crop is (`rectify`). In it a millimetre is a millimetre wherever the camera stood. Much of the
//!    window usually falls outside the photo, and that part is unknown.
//! 2. **Dark.** A pixel is dark when its luminance is at most [`SHAFT_DARK_RATIO`] of the label's
//!    white, the window's 90th percentile inside the label's box. The label is the one thing of known
//!    colour in every photo, so it is the reference, whatever the exposure.
//! 3. **Thin.** A disc [`SHAFT_MAX_WIDTH_MM`] across is rolled through the dark (a morphological
//!    opening, by an exact Euclidean distance transform). What the disc cannot reach is thin: a shaft
//!    or a hosel. A driver's face is dark too, and is one region with its hosel and shaft, so the
//!    opening is what parts the shaft from the face. For it, the label's box and the unknown count as
//!    dark. A dark face round the label would otherwise be a thin rim, and a dark strip along the
//!    photo's edge would be a thin bar.
//! 4. **Bars.** Each 4-connected run of thin, dark pixels outside the label's box (the crate's
//!    `component`) is a bar, with its principal axis, length, width and two ends. Its end nearer the
//!    cross is the end the hosel holds.
//! 5. **A shaft.** A bar is a shaft when every one of these holds:
//!    - it is at least [`SHAFT_MIN_LENGTH_MM`] long and [`SHAFT_MIN_ASPECT`] times as long as wide;
//!    - its near end is within [`SHAFT_MAX_GAP_MM`] of the label's box;
//!    - it runs at [`SHAFT_MIN_ANGLE_DEG`] to [`SHAFT_MAX_ANGLE_DEG`] from the label's long axis, as
//!      a lie does;
//!    - its near end is off the cross toward one end of the label, by at least [`HEEL_MIN_SHARE`] of
//!      the box's half on that side;
//!    - it leans the same way, its far end further toward that end than its near one.
//!
//!    The last two are two cues to the heel, where it sits and which way it leans, and they must agree.
//! 6. **One answer.** Every shaft must give the same orientation. Shafts that disagree, such as a
//!    dark bar leaving each end of the head, are [`OrientMiss::Ambiguous`] and never chosen between.
//!    It is M42's rule for the replay, where shafts at both ends are no shaft.
//! 7. **The hand.** Given one, it must be the orientation's ([`OrientMiss::HandDisagrees`]).
//!
//! # Why not M42's shaft stage
//!
//! `crate::shaft` finds a dark bar joining one end of a head bar, with every length in ball
//! diameters, in a replay seen from above. Here there is no head bar and no ball, and a driver's face
//! is as dark as its shaft. So this stage keeps its idea (a dark bar leaving one end makes that end
//! the heel, and bars at both ends decide nothing) and reuses `mask` and `component`. It measures in
//! the label's millimetres, and it adds the opening.
//!
//! # What it approximates
//!
//! - **The shaft is taken to lie in the face's plane.** The window is cut through the label's map, and
//!   a real shaft rises behind a lofted face, so its drawn angle is the lie foreshortened by the loft
//!   and the camera's tilt. Seen square to the face, `tan(drawn) = tan(lie) · cos(loft)`. A 62° lie
//!   behind a 34° face draws at about 57°, a 64° lie behind a 60° lob wedge at about 46°, and a
//!   driver's 55 to 60° at 55 to 60°. The angle bracket allows for all of them. Only the side and the
//!   sense of up are used, never the angle's value.
//! - **The shaft must be dark.** A graphite shaft is. A chrome steel shaft may be bright (P9 finding
//!   9), and then only a dark ferrule or hosel could stand in for it. A dark background hides a dark
//!   shaft. Both are refused rather than guessed, and P29's real photos are the first to say how
//!   often.
//! - **The photo must hold the hosel and some shaft**, which `TapeNotRead::NoOrientation`'s sentence
//!   asks the golfer for. The tape camera's frame (P22) is where that is asked first.
//!
//! Every tunable is a starting value set on synthetic photos only (P3 finding 10), and each doc names
//! the bracket P29 may move it within.

use super::geometry::rectify_onto;
use super::{CropGrid, FoundLabel, Handedness, Homography, LabelType, Orientation};
use crate::component::components;
use crate::frame::{luminance, Frame, Rect};
use crate::mask::Mask;

// --- The tunables. -------------------------------------------------------------------------------
//
// Set at M46 P10 on synthetic photos only, since no photo of a label on a club exists before P29 (P3
// finding 10). Each names what it was set from and the bracket P29 may move it within, as the label
// finder's block does.

/// The most luminance a shaft pixel may have, as a fraction of the label's white.
///
/// A graphite shaft is black or near it, and a driver's painted face about as dark. The renderer's
/// shaft is 0.22 of the white and its carbon face 0.18, its hitting mat 0.46, and its steel face 0.68.
/// A graphite shaft's gloss lifts it where it catches the light, by an amount no photo here has
/// measured. P29 brackets it at `[0.25, 0.45]`.
pub const SHAFT_DARK_RATIO: f64 = 0.35;

/// The widest dark run that is thin enough to be a shaft or hosel: the opening's disc, across.
///
/// A graphite shaft is about 9 mm across at its tip, and a hosel 12 to 16 mm. The narrowest face a
/// label is stuck on, an iron's, is taller than the iron label's 39.9 mm. So 20 mm is under every face
/// and over every hosel. P29 brackets it at `[15, 30]`.
pub const SHAFT_MAX_WIDTH_MM: f64 = 20.0;

/// The least length of a shaft, as much of it as the photo shows.
///
/// The photo must hold the hosel and some shaft (`TapeNotRead::NoOrientation`'s sentence). 25 mm is
/// under three shaft widths, so a photo framed a little tight still has enough. A dark mark shorter
/// than that is as likely a groove's shadow or a scuff. P29 brackets it at `[15, 50]`.
pub const SHAFT_MIN_LENGTH_MM: f64 = 25.0;

/// The least length over width of a shaft, the width taken across the whole bar, hosel included.
///
/// A visible 25 mm of a 9 mm shaft is 2.8. M42's replay holds its shaft to 3 (`crate::shaft`), but
/// there the shaft runs off a head several diameters long. Here a dark hosel 14 mm across can join
/// the bar and widen it. P29 brackets it at `[2, 4]`.
pub const SHAFT_MIN_ASPECT: f64 = 2.5;

/// The farthest a shaft's near end may lie from the label's box, millimetres.
///
/// On a steel iron a dark shaft starts where the hosel's metal ends, at its ferrule, 30 mm along the
/// renderer's hosel, which puts its near end 22 mm from the iron label's box. A real iron's hosel is
/// longer, by an amount no photo here has measured. On a dark driver the hosel joins the face, and the
/// bar starts at the face's edge, 7.5 mm beyond the driver label's box in the renderer. P29 brackets
/// it at `[50, 120]`.
pub const SHAFT_MAX_GAP_MM: f64 = 80.0;

/// The flattest a shaft may run from the label's long axis, degrees.
///
/// A lie of 55 to 64° draws at about 46 to 64° once a lofted face foreshortens it (the module doc).
/// The lowest is a lob wedge's, and the camera's tilt moves it either way. A dark bar along the face, such
/// as a groove or the top line's shadow, lies near 0°. P29 brackets it at `[25, 45]`.
pub const SHAFT_MIN_ANGLE_DEG: f64 = 35.0;

/// The steepest a shaft may run from the label's long axis, degrees.
///
/// A shaft standing straight up the label would not lean to either end, and its lean is one of the
/// two cues to the heel. P29 brackets it at `[80, 89]`.
pub const SHAFT_MAX_ANGLE_DEG: f64 = 85.0;

/// How far off the cross toward one end a shaft's near end must be, as a share of the box's half on
/// that side.
///
/// A hosel is at the face's heel end, which is at or beyond the label's end. A bar starting near the
/// middle of the label says nothing about which end is the heel. The renderer's shafts start 1.2
/// (driver) to 1.5 (iron) of the half out. P29 brackets it at `[0.2, 0.8]`.
pub const HEEL_MIN_SHARE: f64 = 0.33;

/// The window's samples per millimetre. A 9 mm shaft is 9 samples across, and a driver label's
/// window is about 390 × 350 samples. On P3's flat photo a release read takes 0.30 s with the
/// orientation where it took 0.28 s without (P9's finding), so the window is cheap beside the finder.
pub const ORIENT_PX_PER_MM: f64 = 1.0;

/// How far beyond the label's box the window reaches on every side, millimetres: the farthest
/// [`SHAFT_MAX_GAP_MM`] can reach, plus [`SHAFT_MIN_LENGTH_MM`] of shaft beyond it, with room.
pub const SEARCH_REACH_MM: f64 = 150.0;

/// The margin round the label's box that is the label's and not the head's, millimetres, for the
/// edge's blend. The finder places the box within 0.25 mm on synthetic photos (P9).
const BOX_MARGIN_MM: f64 = 1.5;

/// The share of the box's samples darker than the label's white. The box holds the label and, in its
/// corners, whatever is beside it, so the white is high in the order and not its top, which glare
/// may hold.
const WHITE_PERCENTILE: f64 = 0.9;

/// A squared distance standing for none: further than any two samples of a window can be.
const FAR: f64 = 1e12;

/// The shaft that oriented a label, in the label's printed frame: what the reader judged, kept for a
/// review and for P29's tuning. [M46 P10]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shaft {
    /// The end of the bar nearer the cross, millimetres: where the hosel holds it.
    pub near_mm: [f64; 2],
    /// The other end, where the bar leaves the window or the photo.
    pub far_mm: [f64; 2],
    pub length_mm: f64,
    /// Across the whole bar, hosel included when it is as dark as the shaft.
    pub width_mm: f64,
    /// From the label's long axis, `0` to `90`.
    pub angle_deg: f64,
}

/// Why a found label was not oriented. The tape reader answers each as `no_orientation`. The
/// distinction is for a test and a review. [M46 P10]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrientMiss {
    /// Nothing in the photo was a shaft (the module doc's step 5).
    NoShaft,
    /// Shafts that point to different orientations, such as dark bars leaving both ends.
    Ambiguous,
    /// The shaft says one hand of club and the golfer's hand is the other.
    HandDisagrees,
}

/// One dark, thin run of the window, in the printed frame's millimetres.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Bar {
    ends: [[f64; 2]; 2],
    length: f64,
    width: f64,
}

/// Which way the club lies across a found label, and the shaft that says so, or why it is not known.
/// [M46 P10]
///
/// The module doc gives the steps. `hand` is the golfer's, the club's hand, and only checks the
/// answer. With `None` the shaft alone decides.
pub fn orient(
    photo: &Frame,
    found: &FoundLabel,
    label: &LabelType,
    hand: Option<Handedness>,
) -> Result<(Orientation, Shaft), OrientMiss> {
    // The finder built this map from these corners before it accepted them, so it holds.
    let map = Homography::from_corners(found.corners, label.box_corners_mm())
        .map_err(|_| OrientMiss::NoShaft)?;
    let [[left, top], _, [right, bottom], _] = label.box_corners_mm();
    let grid = CropGrid {
        width: ((right - left + 2.0 * SEARCH_REACH_MM) * ORIENT_PX_PER_MM).round() as usize,
        height: ((top - bottom + 2.0 * SEARCH_REACH_MM) * ORIENT_PX_PER_MM).round() as usize,
        px_per_mm: ORIENT_PX_PER_MM,
        left_mm: left - SEARCH_REACH_MM,
        top_mm: top + SEARCH_REACH_MM,
    };
    let window = rectify_onto(photo, &map, grid);
    let (width, height) = (grid.width, grid.height);
    let on_box: Vec<bool> = (0..width * height)
        .map(|at| {
            let centre = [(at % width) as f64 + 0.5, (at / width) as f64 + 0.5];
            let [x, y] = grid.to_label_mm(centre);
            (left - BOX_MARGIN_MM..=right + BOX_MARGIN_MM).contains(&x)
                && (bottom - BOX_MARGIN_MM..=top + BOX_MARGIN_MM).contains(&y)
        })
        .collect();
    let values: Vec<f64> = window
        .frame
        .rgb
        .chunks_exact(3)
        .map(|p| luminance([p[0], p[1], p[2]]))
        .collect();
    let mut label_values: Vec<f64> = values
        .iter()
        .zip(&on_box)
        .zip(&window.outside)
        .filter(|((_, &on), &outside)| on && !outside)
        .map(|((&value, _), _)| value)
        .collect();
    if label_values.is_empty() {
        return Err(OrientMiss::NoShaft);
    }
    label_values.sort_by(f64::total_cmp);
    let white = label_values[((label_values.len() - 1) as f64 * WHITE_PERCENTILE).round() as usize];
    let dark: Vec<bool> = values
        .iter()
        .zip(&window.outside)
        .map(|(&value, &outside)| !outside && value <= SHAFT_DARK_RATIO * white)
        .collect();
    let solid: Vec<bool> = (0..width * height)
        .map(|at| dark[at] || on_box[at] || window.outside[at])
        .collect();
    let radius = SHAFT_MAX_WIDTH_MM / 2.0 * ORIENT_PX_PER_MM;
    let thin = thin_parts(&solid, width, height, radius);
    let bars = Mask {
        rect: Rect {
            x0: 0,
            y0: 0,
            x1: width,
            y1: height,
        },
        bits: (0..width * height)
            .map(|at| thin[at] && dark[at] && !on_box[at])
            .collect(),
    };
    let mut chosen: Option<(Orientation, Shaft)> = None;
    let mut ambiguous = false;
    for component in components(&bars) {
        let bar = Bar {
            ends: component.ends.map(|end| grid.to_label_mm(end)),
            length: component.length / ORIENT_PX_PER_MM,
            width: component.width / ORIENT_PX_PER_MM,
        };
        let Some((orientation, shaft)) = judge(&bar, label) else {
            continue;
        };
        match chosen {
            Some((kept, _)) if kept != orientation => ambiguous = true,
            Some((_, longest)) if longest.length_mm >= shaft.length_mm => {}
            _ => chosen = Some((orientation, shaft)),
        }
    }
    if ambiguous {
        return Err(OrientMiss::Ambiguous);
    }
    let (orientation, shaft) = chosen.ok_or(OrientMiss::NoShaft)?;
    if hand.is_some_and(|hand| hand != orientation.handedness()) {
        return Err(OrientMiss::HandDisagrees);
    }
    Ok((orientation, shaft))
}

/// The orientation a bar gives when it is a shaft (the module doc's step 5), or `None`.
fn judge(bar: &Bar, label: &LabelType) -> Option<(Orientation, Shaft)> {
    if bar.length < SHAFT_MIN_LENGTH_MM || bar.length < SHAFT_MIN_ASPECT * bar.width {
        return None;
    }
    let [a, b] = bar.ends;
    let (near, far) = if a[0].hypot(a[1]) <= b[0].hypot(b[1]) {
        (a, b)
    } else {
        (b, a)
    };
    let [[left, top], _, [right, bottom], _] = label.box_corners_mm();
    let gap = (left - near[0])
        .max(near[0] - right)
        .max(0.0)
        .hypot((bottom - near[1]).max(near[1] - top).max(0.0));
    if gap > SHAFT_MAX_GAP_MM {
        return None;
    }
    let run = [far[0] - near[0], far[1] - near[1]];
    let angle_deg = run[1].abs().atan2(run[0].abs()).to_degrees();
    if !(SHAFT_MIN_ANGLE_DEG..=SHAFT_MAX_ANGLE_DEG).contains(&angle_deg) {
        return None;
    }
    // `left` is negative and `right` positive, so the share is positive on the end it is toward.
    let heel_printed_right = near[0] > 0.0;
    let share = near[0] / if heel_printed_right { right } else { left };
    let leans_to_heel = if heel_printed_right {
        run[0] > 0.0
    } else {
        run[0] < 0.0
    };
    if share < HEEL_MIN_SHARE || !leans_to_heel {
        return None;
    }
    Some((
        Orientation {
            toe_printed_right: !heel_printed_right,
            top_line_printed_top: run[1] > 0.0,
        },
        Shaft {
            near_mm: near,
            far_mm: far,
            length_mm: bar.length,
            width_mm: bar.width,
            angle_deg,
        },
    ))
}

/// The pixels of `solid` that a disc of `radius` pixels, held inside `solid`, cannot cover: the
/// set less its opening. Beyond the grid counts as solid, so nothing is thin for touching the edge.
fn thin_parts(solid: &[bool], width: usize, height: usize, radius: f64) -> Vec<bool> {
    let opened = opening(solid, width, height, radius);
    solid
        .iter()
        .zip(&opened)
        .map(|(&s, &covered)| s && !covered)
        .collect()
}

/// The pixels of `solid` that a disc of `radius` pixels, held wholly inside `solid`, can cover: the
/// morphological opening, by two exact distance transforms. Beyond the grid counts as solid. The
/// shaft keeps what the disc cannot reach ([`thin_parts`]), and the mark finder keeps what it can,
/// which strips the label's thin printing from a mark (`mark.rs`). [M46 P10, shared at P11]
pub(super) fn opening(solid: &[bool], width: usize, height: usize, radius: f64) -> Vec<bool> {
    let r2 = radius * radius;
    let open: Vec<bool> = solid.iter().map(|&s| !s).collect();
    let to_open = squared_distance(&open, width, height);
    // The centres a disc fits round: further than `radius` from every pixel not in the set.
    let core: Vec<bool> = to_open.iter().map(|&d| d > r2).collect();
    let to_core = squared_distance(&core, width, height);
    solid
        .iter()
        .zip(&to_core)
        .map(|(&s, &d)| s && d <= r2)
        .collect()
}

/// The squared distance from each pixel's centre to the nearest pixel centre in `targets`, over a
/// `width × height` grid, row-major, or about [`FAR`] when there is no target.
///
/// Felzenszwalb and Huttenlocher's exact transform (2012): the squared distance separates into a
/// pass along every row and then every column, each the lower envelope of parabolas.
pub(super) fn squared_distance(targets: &[bool], width: usize, height: usize) -> Vec<f64> {
    let mut grid: Vec<f64> = targets.iter().map(|&t| if t { 0.0 } else { FAR }).collect();
    let mut line = Vec::with_capacity(width.max(height));
    for row in grid.chunks_exact_mut(width.max(1)) {
        line.clear();
        line.extend_from_slice(row);
        row.copy_from_slice(&distance_along(&line));
    }
    for column in 0..width {
        line.clear();
        line.extend((0..height).map(|row| grid[row * width + column]));
        for (row, value) in distance_along(&line).into_iter().enumerate() {
            grid[row * width + column] = value;
        }
    }
    grid
}

/// One line of [`squared_distance`]: `min over p of (q − p)² + f(p)` at every `q`.
fn distance_along(f: &[f64]) -> Vec<f64> {
    let n = f.len();
    if n == 0 {
        return Vec::new();
    }
    // Where parabola `p` and parabola `q` meet.
    let meet = |p: usize, q: usize| {
        let (pf, qf) = (p as f64, q as f64);
        ((f[q] + qf * qf) - (f[p] + pf * pf)) / (2.0 * (qf - pf))
    };
    let mut vertices = vec![0usize; n];
    let mut bounds = vec![0.0; n + 1];
    bounds[0] = f64::NEG_INFINITY;
    bounds[1] = f64::INFINITY;
    let mut k = 0;
    for q in 1..n {
        let mut s = meet(vertices[k], q);
        // `bounds[0]` is −∞ and `s` is finite, so this stops at `k = 0` at the latest.
        while s <= bounds[k] {
            k -= 1;
            s = meet(vertices[k], q);
        }
        k += 1;
        vertices[k] = q;
        bounds[k] = s;
        bounds[k + 1] = f64::INFINITY;
    }
    let mut k = 0;
    (0..n)
        .map(|q| {
            while bounds[k + 1] < q as f64 {
                k += 1;
            }
            let d = q as f64 - vertices[k] as f64;
            d * d + f[vertices[k]]
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::super::STRIKEPRO_IRON;
    use super::*;

    #[test]
    fn the_distance_transform_is_the_brute_force_one() {
        let (width, height) = (13, 9);
        let targets: Vec<bool> = (0..width * height)
            .map(|at| [5, 17, 40, 41, 77, 100].contains(&at))
            .collect();
        let fast = squared_distance(&targets, width, height);
        for (at, &fast) in fast.iter().enumerate() {
            let (x, y) = ((at % width) as f64, (at / width) as f64);
            let slow = (0..width * height)
                .filter(|&t| targets[t])
                .map(|t| ((t % width) as f64 - x).powi(2) + ((t / width) as f64 - y).powi(2))
                .fold(f64::INFINITY, f64::min);
            assert_eq!(fast, slow, "pixel {at}");
        }
        let none = squared_distance(&[false; 6], 3, 2);
        assert!(none.iter().all(|&d| d >= FAR));
    }

    #[test]
    fn an_opening_keeps_a_disc_and_drops_the_bar_leaving_it() {
        // A disc of radius 12 at (20, 20) with a bar 5 wide leaving it to the right, on 60 × 40.
        let (width, height) = (60, 40);
        let solid: Vec<bool> = (0..width * height)
            .map(|at| {
                let (x, y) = ((at % width) as f64 + 0.5, (at / width) as f64 + 0.5);
                (x - 20.0).hypot(y - 20.0) <= 12.0
                    || ((20.0..=55.0).contains(&x) && (y - 20.0).abs() <= 2.5)
            })
            .collect();
        let thin = thin_parts(&solid, width, height, 4.0);
        let at = |x: usize, y: usize| thin[y * width + x];
        assert!(!at(20, 20), "the disc's middle is not thin");
        assert!(!at(10, 20), "nor its rim");
        assert!(at(45, 20) && at(54, 20), "the bar is");
        assert!(!at(45, 5), "and nothing outside the set is");
    }

    /// A bar from `near` running `length` at `angle` degrees from `+x`, 9 mm wide.
    fn bar(near: [f64; 2], angle: f64, length: f64) -> Bar {
        let (s, c) = angle.to_radians().sin_cos();
        Bar {
            ends: [near, [near[0] + length * c, near[1] + length * s]],
            length,
            width: 9.0,
        }
    }

    #[test]
    fn a_shaft_is_judged_by_its_length_its_angle_its_side_and_its_lean() {
        let label = &STRIKEPRO_IRON;
        let oriented = |b: Bar| judge(&b, label).map(|(orientation, _)| orientation);
        let right_logo_up = Orientation {
            toe_printed_right: false,
            top_line_printed_top: true,
        };
        // A right-handed club seen face-on with the label logo-up: the shaft leaves the heel, on the
        // printed right, and runs up and further right.
        assert_eq!(oriented(bar([54.0, 38.5], 62.0, 40.0)), Some(right_logo_up));
        // The same club with the label stuck logo-down: the printed frame is turned half a turn.
        assert_eq!(
            oriented(bar([-54.0, -38.5], 242.0, 40.0)),
            Some(Orientation {
                toe_printed_right: true,
                top_line_printed_top: false
            })
        );
        // A left-handed club, logo-up.
        assert_eq!(
            oriented(bar([-54.0, 38.5], 118.0, 40.0)),
            Some(Orientation {
                toe_printed_right: true,
                top_line_printed_top: true
            })
        );
        // The listed order of the ends does not matter.
        let mut reversed = bar([54.0, 38.5], 62.0, 40.0);
        reversed.ends.swap(0, 1);
        assert_eq!(oriented(reversed), Some(right_logo_up));
        // Too short, too fat, too flat, too steep, leaning toward the toe, starting near the middle,
        // and too far from the label are each no shaft.
        assert_eq!(oriented(bar([54.0, 38.5], 62.0, 20.0)), None);
        assert_eq!(
            oriented(Bar {
                width: 18.0,
                ..bar([54.0, 38.5], 62.0, 40.0)
            }),
            None
        );
        assert_eq!(oriented(bar([54.0, 38.5], 20.0, 40.0)), None);
        assert_eq!(oriented(bar([54.0, 38.5], 88.0, 40.0)), None);
        assert_eq!(oriented(bar([54.0, 38.5], 118.0, 40.0)), None);
        assert_eq!(oriented(bar([6.0, 30.0], 62.0, 40.0)), None);
        assert_eq!(oriented(bar([140.0, 60.0], 62.0, 40.0)), None);
    }
}
