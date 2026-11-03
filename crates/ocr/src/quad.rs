//! Where the launch-monitor screen is in a photo: four corners, or none. [M29 P2]
//!
//! The twin of `launch_monitor/screen/preprocess.py`'s `find_screen_quad`, with `_as_quad`,
//! `_as_rotated_rect`, `_plausible_area`, `_downscale_for_detection` and `_order_quad`, over the
//! OpenCV ports in [`crate::cv`]. The search is frozen Python's, setting for setting: shrink to
//! 1000 px wide, grey, a bilateral blur, then for each of two Canny presets and three closing
//! sizes the five largest outer contours, each tried at three polygon tolerances for a convex
//! four-gon of plausible size that fills its contour; the first that passes wins. If none does,
//! the rotated bounding box of the first plausibly sized contour stands in, and if there is none
//! of those either the photo is read uncropped (P3's `prepare_screen`).
//!
//! # How close it lands (M29 P2, the 13 bay photos)
//!
//! **Given frozen Python's decoded pixels**, identical: every intermediate (the 1000 px copy, the
//! grey, the blur, both edge maps, all six closings, every contour, every polygon) and every
//! corner, bit for bit, on all 13, by the same path (four corners on two, the rotated box on five,
//! nothing on six). Everything before the corners is integer arithmetic or OpenCV's own `f32`
//! order, so nothing is merely near.
//!
//! **Given this crate's own decode**, the same, because the decode is exact on all but the HEIC
//! (`crate::decode`'s doc), whose quad still lands where Python's does. It was not always: with
//! `zune-jpeg` decoding, one level from libjpeg-turbo on about 0.5% of values, this found a
//! rotated box on 2026-08-23-4, where Python finds none, and moved one corner of 2026-08-23-11 by
//! 114 px. The search is that sensitive to its pixels, which is why the decoder is a hand port of
//! libjpeg-turbo's rather than any JPEG decoder (the M29 plan's P2 finding).
//!
//! # Where it departs from its twin: a quad that orders with a corner repeated (M42 P3)
//!
//! [`order_quad`] gives each role to the corner with the extreme `x + y` or `y - x`, and nothing
//! stops one corner winning two roles. A four-gon with one corner far enough out to a side wins
//! both of that side's roles there, and the "ordered" quad then names one point twice and loses
//! another. On two of Full Swing's photos the finder returned `Corners` (Canny 30, closing 5,
//! tolerance 0.04) with top-right and bottom-right the same point, (5706.3, 2638.9) on
//! `IMG_2893.HEIC` and (5706.3, 2707.5) on `IMG_2894.HEIC`, so the warp cut a 1600 px sliver
//! from each and the recognizer found no text on it at all (the M42 plan's finding 5). Such a
//! four-gon is convex and plausibly sized, so `as_quad` passed it. Frozen Python's
//! `_order_quad` and `find_screen_quad` have the same defect and keep it (ADR-035 clause 4).
//!
//! Here a four-gon whose ordering repeats a corner is refused as `as_quad` refuses one: the search
//! goes on to the next contour and setting, and the contour's rotated box may still stand in. A
//! rotated box whose ordering repeats a corner is not kept as the stand-in either. Both photos now
//! stand in with that contour's rotated box and read every tile label (the plan's P3 finding).
//! Both checks read only the ordered corners, and a refusal only ever narrows what was accepted,
//! so a photo whose quad ordered cleanly before (or that had none) is answered exactly as before,
//! by the same path. The 13 bay photos are (`tests/quad.rs`, unchanged), which is why the change
//! moves no stored shot and `SCREEN_PARSER_VERSION` stays where it is (the M42 plan's call 3).

use image::RgbImage;

use crate::cv::canny::canny;
use crate::cv::contours::{
    approx_poly_dp, arc_length, contour_area, find_external_contours, is_contour_convex, Point,
};
use crate::cv::filter::{bilateral_filter, rgb_to_gray};
use crate::cv::morph::close_rect;
use crate::cv::rect::{box_points, min_area_rect};
use crate::cv::resize::{resize_area, shrink_to_width};

/// Four corners, top-left, top-right, bottom-right, bottom-left, in the photo's pixels.
pub type Quad = [[f32; 2]; 4];

/// Outline detection runs on a copy this wide (`_DETECT_WIDTH`), because the closing kernels are
/// fixed pixel sizes that only bridge the bezel's gaps at about this scale.
pub const DETECT_WIDTH: usize = 1000;

/// A candidate must cover at least this much of the frame, and no more than the upper bound
/// (`_MIN_QUAD_AREA_RATIO`, `_MAX_QUAD_AREA_RATIO`): a picture frame behind the monitor never
/// wins, and neither does a contour that merely traced the photo's own border.
const MIN_QUAD_AREA_RATIO: f64 = 0.15;
const MAX_QUAD_AREA_RATIO: f64 = 0.92;

/// A four-gon more than this multiple of its contour's area came from an open edge fragment, not
/// a closed screen outline (`_MAX_QUAD_FILL_RATIO`).
const MAX_QUAD_FILL_RATIO: f64 = 1.6;

/// The search order, verbatim (`_CANNY_PRESETS`, `_CLOSE_SIZES`, `_APPROX_EPSILONS`).
const CANNY_PRESETS: [(f64, f64); 2] = [(30.0, 120.0), (10.0, 60.0)];
const CLOSE_SIZES: [usize; 3] = [5, 15, 25];
const APPROX_EPSILONS: [f64; 3] = [0.02, 0.04, 0.01];

/// The bilateral filter's settings: a 9 px disc, sigma 75 for both colour and space.
const BILATERAL_D: i32 = 9;
const BILATERAL_SIGMA: f64 = 75.0;

/// How many of the largest contours each setting looks at.
const CANDIDATES: usize = 5;

/// Which of `find_screen_quad`'s two answers this is. Python returns only the corners; the path is
/// kept here because the gate checks it, and a reader of a bad crop wants to know which it was.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum QuadSource {
    /// A contour reduced to a convex four-gon: the Canny `low` threshold, the closing size and the
    /// polygon tolerance that found it.
    Corners {
        low: f64,
        close: usize,
        epsilon: f64,
    },
    /// No contour reduced to four corners, and this is the rotated bounding box of the first
    /// plausibly sized one, found at this `low` and closing size.
    RotatedBox { low: f64, close: usize },
}

/// The screen's outline in a photo.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScreenQuad {
    pub corners: Quad,
    pub source: QuadSource,
}

/// `find_screen_quad`: the monitor's outline as four ordered corners in the photo's own pixels, or
/// `None` when nothing convincing was found (frozen Python then parses the photo uncropped).
pub fn find_screen_quad(image: &RgbImage) -> Option<ScreenQuad> {
    let (width, height) = (image.width() as usize, image.height() as usize);
    let (scale, small, sw, sh) = match shrink_to_width(width, height, DETECT_WIDTH) {
        Some((scale, w, h)) => (
            scale,
            resize_area(image.as_raw(), width, height, 3, w, h),
            w,
            h,
        ),
        None => (1.0, image.as_raw().clone(), width, height),
    };
    let gray = rgb_to_gray(&small);
    let smoothed = bilateral_filter(&gray, sw, sh, BILATERAL_D, BILATERAL_SIGMA, BILATERAL_SIGMA);
    let frame_area = (sh * sw) as f64;

    let mut fallback: Option<ScreenQuad> = None;
    for (low, high) in CANNY_PRESETS {
        let edges = canny(&smoothed, sw, sh, low, high);
        for close in CLOSE_SIZES {
            let closed = close_rect(&edges, sw, sh, close);
            for (contour, area) in largest_contours(&closed, sw, sh) {
                if area < frame_area * MIN_QUAD_AREA_RATIO {
                    break; // sorted, so every remaining contour is smaller still
                }
                // A four-gon `order_quad` gives one corner two roles is refused as `as_quad`
                // refuses one, and the search goes on (the module doc's M42 section).
                if let Some((quad, epsilon)) = as_quad(&contour, area, frame_area) {
                    let corners = order_quad(divide(quad, scale));
                    if !repeats_a_corner(&corners) {
                        return Some(ScreenQuad {
                            corners,
                            source: QuadSource::Corners {
                                low,
                                close,
                                epsilon,
                            },
                        });
                    }
                }
                if fallback.is_none() {
                    fallback = as_rotated_rect(&contour, frame_area)
                        .map(|corners| order_quad(divide(corners, scale)))
                        .filter(|corners| !repeats_a_corner(corners))
                        .map(|corners| ScreenQuad {
                            corners,
                            source: QuadSource::RotatedBox { low, close },
                        });
                }
            }
        }
    }
    fallback
}

/// The five largest outer contours with their areas, largest first, equal areas in OpenCV's
/// order (Python's `sorted(..., reverse=True)` is stable, and so is `sort_by`).
fn largest_contours(closed: &[u8], width: usize, height: usize) -> Vec<(Vec<Point>, f64)> {
    let mut contours: Vec<(Vec<Point>, f64)> = find_external_contours(closed, width, height)
        .into_iter()
        .map(|contour| {
            let area = contour_area(&contour);
            (contour, area)
        })
        .collect();
    contours.sort_by(|a, b| b.1.total_cmp(&a.1));
    contours.truncate(CANDIDATES);
    contours
}

/// `_as_quad`: the contour reduced to four corners, if it plausibly is a screen outline, with the
/// tolerance that reduced it.
fn as_quad(contour: &[Point], area: f64, frame_area: f64) -> Option<([[f32; 2]; 4], f64)> {
    let perimeter = arc_length(contour);
    for epsilon in APPROX_EPSILONS {
        let approx = approx_poly_dp(contour, epsilon * perimeter);
        if approx.len() != 4 || !is_contour_convex(&approx) {
            continue;
        }
        let quad_area = contour_area(&approx);
        if !plausible_area(quad_area, frame_area) {
            continue;
        }
        if area > 0.0 && quad_area / area > MAX_QUAD_FILL_RATIO {
            continue;
        }
        let corner = |p: Point| [p.x as f32, p.y as f32];
        return Some((
            [
                corner(approx[0]),
                corner(approx[1]),
                corner(approx[2]),
                corner(approx[3]),
            ],
            epsilon,
        ));
    }
    None
}

/// `_as_rotated_rect`: the contour's minimal rotated box, if its area is plausible. The area is
/// `width * height` as Python multiplies them, in `f64` from the `f32` sides.
fn as_rotated_rect(contour: &[Point], frame_area: f64) -> Option<[[f32; 2]; 4]> {
    let rect = min_area_rect(contour);
    if !plausible_area(
        f64::from(rect.size[0]) * f64::from(rect.size[1]),
        frame_area,
    ) {
        return None;
    }
    Some(box_points(&rect))
}

/// `_plausible_area`.
fn plausible_area(area: f64, frame_area: f64) -> bool {
    let ratio = area / frame_area;
    (MIN_QUAD_AREA_RATIO..=MAX_QUAD_AREA_RATIO).contains(&ratio)
}

/// `quad / scale` on a `float32` array: NumPy 2 casts the Python scalar to the array's `float32`
/// and divides there (NEP 50), so this does too.
fn divide(quad: [[f32; 2]; 4], scale: f64) -> [[f32; 2]; 4] {
    let scale = scale as f32;
    quad.map(|[x, y]| [x / scale, y / scale])
}

/// `_order_quad`: top-left has the smallest `x + y`, bottom-right the largest, top-right the
/// smallest `y - x`, bottom-left the largest, each in `f32` and each the first on a tie
/// (`np.argmin` and `np.argmax`).
pub fn order_quad(points: [[f32; 2]; 4]) -> Quad {
    let sums = points.map(|[x, y]| x + y);
    let diffs = points.map(|[x, y]| y - x);
    let first = |values: &[f32; 4], better: fn(f32, f32) -> bool| -> usize {
        let mut best = 0;
        for i in 1..4 {
            if better(values[i], values[best]) {
                best = i;
            }
        }
        best
    };
    let less = |a: f32, b: f32| a < b;
    let greater = |a: f32, b: f32| a > b;
    [
        points[first(&sums, less)],
        points[first(&diffs, less)],
        points[first(&sums, greater)],
        points[first(&diffs, greater)],
    ]
}

/// Whether two of an ordered quad's corners are the same point: [`order_quad`] gave one corner
/// two roles, so the four-gon was not a screen it could order (the module doc's M42 section).
fn repeats_a_corner(quad: &Quad) -> bool {
    (0..4).any(|i| (i + 1..4).any(|j| quad[i] == quad[j]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corners_are_ordered_clockwise_from_the_top_left() {
        let quad = order_quad([[900.0, 700.0], [10.0, 20.0], [880.0, 30.0], [5.0, 690.0]]);
        assert_eq!(
            quad,
            [[10.0, 20.0], [880.0, 30.0], [900.0, 700.0], [5.0, 690.0]]
        );
    }

    #[test]
    fn a_dark_screen_on_a_light_wall_is_found_by_its_four_corners() {
        // A 400x300 photo: a light background and a dark, slightly tilted screen covering about
        // half of it. Small enough that no shrink happens, so the corners come back exactly.
        let (w, h) = (400u32, 300u32);
        let inside = |x: f32, y: f32| -> bool {
            // The screen: (60,50) (340,40) (350,250) (50,260), a convex four-gon.
            let corners = [(60.0, 50.0), (340.0, 40.0), (350.0, 250.0), (50.0, 260.0)];
            (0..4).all(|i| {
                let (ax, ay) = corners[i];
                let (bx, by) = corners[(i + 1) % 4];
                (bx - ax) * (y - ay) - (by - ay) * (x - ax) >= 0.0
            })
        };
        let image = RgbImage::from_fn(w, h, |x, y| {
            if inside(x as f32, y as f32) {
                image::Rgb([20, 25, 30])
            } else {
                image::Rgb([200, 200, 190])
            }
        });
        let found = find_screen_quad(&image).expect("a screen");
        assert!(
            matches!(found.source, QuadSource::Corners { .. }),
            "{found:?}"
        );
        let expected = [[60.0, 50.0], [340.0, 40.0], [350.0, 250.0], [50.0, 260.0]];
        for (got, want) in found.corners.iter().zip(expected) {
            assert!(
                (got[0] - want[0]).abs() <= 2.0 && (got[1] - want[1]).abs() <= 2.0,
                "{found:?}"
            );
        }
    }

    #[test]
    fn a_diamond_orders_with_a_corner_repeated() {
        // The defect (the M42 plan's finding 5), pinned: on a square turned 45 degrees the top
        // and left corners tie on x + y, the top and right on y - x, and the first-on-a-tie rule
        // gives the top corner both the top-left and the top-right role. The left one is lost.
        let quad = order_quad([[200.0, 20.0], [330.0, 150.0], [200.0, 280.0], [70.0, 150.0]]);
        assert_eq!(
            quad,
            [[200.0, 20.0], [200.0, 20.0], [330.0, 150.0], [200.0, 280.0]]
        );
        assert!(repeats_a_corner(&quad));
    }

    #[test]
    fn a_diamond_screen_is_never_found_with_a_corner_repeated() {
        // A 400x300 photo of a dark diamond on a light wall, its diagonals 360 and 180 px, 27% of
        // the frame: plausibly sized, convex and four-cornered. Before M42 P3 it came back as
        // `Corners` [[20, 149], [380, 151], [380, 151], [20, 149]], the left and right corners
        // each given two roles. A square turned 45 degrees does not show it here: its corners
        // reduce a pixel off their ties, (199, 20) (330, 149) (201, 280) (70, 151), and order
        // cleanly (measured at M42 P3), so the diamond is twice as wide as it is tall and ties
        // nothing. Since, the contour's rotated box stands in, its four corners distinct.
        let image = RgbImage::from_fn(400, 300, |x, y| {
            if (x as i32 - 200).abs() + 2 * (y as i32 - 150).abs() <= 180 {
                image::Rgb([20, 25, 30])
            } else {
                image::Rgb([200, 200, 190])
            }
        });
        if let Some(found) = find_screen_quad(&image) {
            assert!(!repeats_a_corner(&found.corners), "{found:?}");
        }
    }

    #[test]
    fn a_blank_photo_has_no_screen() {
        let image = RgbImage::from_pixel(300, 200, image::Rgb([128, 128, 128]));
        assert_eq!(find_screen_quad(&image), None);
    }
}
