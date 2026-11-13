//! The marks on a label: each dark spot on the squared crop large enough to be a strike's, where
//! its middle lies, and whether the label's edge cuts it. [M46 P11]
//!
//! # How a mark is found
//!
//! 1. **The label's interior.** The crop is the label's box, so its corners hold the club face or
//!    whatever is beside the label. A pixel is the label's when its centre lies inside the type's
//!    outline ([`LabelType::outline_mm`]) and the photo held it. The interior is the label less a
//!    rim [`MARK_EDGE_MARGIN_MM`] wide, where the outline's measurement, the finder's fit and the
//!    edge's blend leave the face showing. A dark face would be a mark there.
//! 2. **The label's own white**, from a band [`WHITE_BAND_MM`] wide just inside that rim, as the
//!    plan's P11 asks. It is a plane over the crop, `a + b·column + c·row`, fitted robustly. The
//!    band's median starts it, and each round keeps the samples within a few spreads of the last
//!    plane. That drops the printing the band crosses (the logo's swoosh and letters, the dashed
//!    rings) and any glare. A plane rather than one level because light falls across a label
//!    unevenly, and a gradient would otherwise darken one end into a mark.
//! 3. **Dark.** Each interior pixel's darkening is `(white − luminance) / white`, 0 on clean
//!    label and 1 on black, and a pixel is dark at [`MARK_MIN_CONTRAST`] or more.
//! 4. **Wide.** A disc [`MARK_OPEN_RADIUS_MM`] in radius is rolled through the dark (a
//!    morphological opening, `orient.rs`'s), and what it cannot reach is dropped. That strips the
//!    printing, every part of which is a thin stroke, and keeps a mark whole however much printing
//!    it covers or touches.
//! 5. **Spots.** Each 4-connected run of what is left, of at least [`MARK_MIN_AREA_MM2`], is a
//!    [`Spot`]. Its middle is its pixels' centroid weighted by their darkening, the plan's call.
//! 6. **Clipped.** A spot that reaches the interior's rim is cut by the label's edge, so its
//!    centroid is not its mark's middle.
//!
//! On a fresh label ([`on_a_fresh_label`]) one spot is the strike, one clipped spot is
//! `mark_clipped`, none is `no_new_mark`, and two or more need the golfer's pick. So does one spot
//! longer than [`MARK_MAX_ELONGATION`], which is taken for two marks run together.
//!
//! # Why the printing is told apart by its shape
//!
//! The printing is dark too: the navy logo, the red cross and ring, and the black dashes and text
//! (P3 finding 4). A strike is "a darker mark" (P3 finding 8), and nothing more is known of it
//! before P29. Three ways to tell them apart were open:
//! - **By colour.** The navy and the red are saturated, but the mark's colour is unknown, and a
//!   mark over the printing may keep the printing's. A rule on colour would rest on a guess.
//! - **By where the printing is.** It is the same on every label of a type, but its layout is not
//!   in [`LabelType`] (P4 finding 9) and was measured to ±1 mm (P3 finding 4). A mask of it would
//!   also cut a hole wherever a mark covers the printing, and move that mark's centroid.
//! - **By shape**, which is what is done. Every stroke of the printing is thin, from the 0.45 mm
//!   dashes to the logo's letters, about 1 mm across and 3 mm tall (P9 finding 4). A ball leaves a
//!   mark many millimetres across. The opening tells them apart by width alone and needs neither
//!   the mark's colour nor the printing's place.
//!
//! # Why the reading is not anchored on the printed cross
//!
//! P9 finding 10 left it to this phase. On P3's flat photo the fitted centre lay up to 0.9 mm from
//! the printed cross, and every crop holds the cross, so measuring each mark from the cross found
//! in the crop would take the fit's translation out of every reading. It is not done, for two
//! reasons. The cross is what a centred strike covers, so the anchor would fail on the strikes the
//! drill is for, and a reading measured from the cross on some photos and from the fit on others
//! would mix two frames in one pattern. And on P3's photo the label lay on white paper, where its
//! edge is faint. On a club face the edge is strong, and the synthetic finder lands within 0.25 mm.
//! P29's real photos are the first that can say whether the 0.9 mm was the paper's.
//!
//! # What it approximates
//!
//! A shadow across the label, darker than [`MARK_MIN_CONTRAST`] and wider than the opening's disc,
//! is a mark here, since a plane cannot follow a shadow's edge. A mark speckled by the ball's
//! dimples may break into pieces narrower than the disc. Both are for P29's photos to show. Every
//! tunable is a starting value set on synthetic photos only (P3 finding 10), and each doc names the
//! bracket P29 may move it within.

use super::orient::{opening, squared_distance};
use super::{Crop, CropGrid, LabelType, Orientation, TapeMark};
use crate::component::components_with_pixels;
use crate::frame::{luminance, Rect};
use crate::mask::Mask;

// --- The tunables. -------------------------------------------------------------------------------
//
// Set at M46 P11 on synthetic photos only, since no label is hit before P29 (P3 finding 10). Each
// names what it was set from and the bracket P29 may move it within, as the finder's block does.

/// The least darkening a mark's pixel has, `(white − luminance) / white`.
///
/// The renderer's guessed mark is 0.5 (`tape_scene::MARK`), its navy 0.8, red 0.63 and black 0.87.
/// The label's noise and the light's gradient left over by the white's plane are about 0.01. A
/// real mark's darkening is unknown, and a shadow lower than this is not a mark. P29 brackets it
/// at `[0.1, 0.35]`.
pub const MARK_MIN_CONTRAST: f64 = 0.2;

/// The radius of the disc the dark is opened by, millimetres: a dark run narrower than twice this
/// is printing, not a mark.
///
/// The printing's widest strokes are the logo's letters, about 1 mm across and 3 mm tall (P9
/// finding 4), and the logo's red target, a ring about 4.7 mm across round a dot. A 4 mm disc fits
/// in none of them, even where a soft photo runs a letter's strokes together. A ball's mark is
/// many millimetres across. P29 brackets it at `[1.0, 3.0]`.
pub const MARK_OPEN_RADIUS_MM: f64 = 2.0;

/// The least area of a mark, square millimetres.
///
/// Above the opening's own disc (12.6 mm²) and the logo's target run solid by a soft photo (about
/// 17 mm²), and far below the contact patch a ball leaves, which no photo here has measured: a mark
/// 6.2 mm across is 30 mm². A spot smaller than this is a speck. P29 brackets it at `[15, 100]`.
pub const MARK_MIN_AREA_MM2: f64 = 30.0;

/// How far in from the label's outline a mark's pixel must lie, millimetres.
///
/// The outline is measured to ±0.5 mm (P3 finding 4), the finder lands within 0.25 mm on
/// synthetic photos (P9), and the edge blends over about a photo pixel. Within this rim the club
/// face may show, and a dark driver face would be a mark. P29 brackets it at `[0.5, 2.0]`.
pub const MARK_EDGE_MARGIN_MM: f64 = 1.0;

/// The most a lone spot may be longer than it is wide, `√(λ₁ / λ₂)` of its second moments, before
/// it is taken for two marks run together and handed to the golfer to pick.
///
/// A round mark is 1, and an ellipse is its axes' ratio. Two marks `r` in radius with their
/// centres `d` apart come to about `√(1 + d² / r²)`, so the gate passes a pair closer than about
/// `0.8 r`, read as one mark between them, at most `0.42 r` from each. A pair further apart is
/// refused rather than read halfway. A real mark's own shape is unknown: a ball rolling up a
/// lofted face may smear it. P29 brackets it at `[1.15, 1.8]`.
pub const MARK_MAX_ELONGATION: f64 = 1.3;

/// The width of the band the label's white is estimated from, millimetres in from the interior's
/// rim. Wide enough to hold thousands of clean samples on every type, and narrow enough to stay
/// near the edge, which is where the plan's P11 puts it and where a centred mark is not.
pub const WHITE_BAND_MM: f64 = 3.0;

/// How close to the interior's rim a spot must come to be cut by it, millimetres. The opening
/// keeps a cut mark up to the rim where the rim is straight, and pulls back a little where it
/// curves. 0.5 mm is five crop pixels.
const CLIP_REACH_MM: f64 = 0.5;

/// The fewest band samples a white is fitted from. Every type's band holds tens of thousands at
/// [`super::CROP_PX_PER_MM`].
const WHITE_MIN_SAMPLES: usize = 200;

/// Rounds of the white's robust fit. The plane settles in two or three.
const WHITE_ROUNDS: usize = 5;

/// A sample is kept for the next round within this many spreads of the plane. The spread is the
/// residuals' median absolute deviation, scaled to a normal's standard deviation.
const WHITE_KEEP_SPREADS: f64 = 3.0;

/// The least spread, luminance levels, so a flat white with no noise still keeps its samples.
const WHITE_MIN_SPREAD: f64 = 1.0;

/// The largest share of the band that may lie well above the fitted white. Glare takes a little,
/// and a white fitted to the band's darker part leaves the real white above it.
const WHITE_MAX_BRIGHTER_SHARE: f64 = 0.25;

/// The least white anywhere in the crop, luminance levels. A plane below it is no white a mark
/// could be seen against, and is refused rather than divided by.
const WHITE_MIN_LEVEL: f64 = 32.0;

/// One dark spot on a label's crop, large enough to be a strike's mark, in the printed frame. It
/// says nothing of whether the mark is new: that is the label's history (P12). [M46 P11]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spot {
    /// Its centroid in the crop's pixels, weighted by darkening, in [`CropGrid`]'s continuous
    /// coordinates.
    pub centroid_px: [f64; 2],
    /// The same point in the printed frame's millimetres.
    pub centre_mm: [f64; 2],
    pub area_mm2: f64,
    /// `√(λ₁ / λ₂)` of its pixels' second moments: 1 for a disc ([`MARK_MAX_ELONGATION`]).
    pub elongation: f64,
    /// It reaches the interior's rim, so the label's edge cuts it.
    pub clipped: bool,
}

impl Spot {
    /// The spot as a reading, in C4's frame, which `orientation` turns the printed frame into.
    pub fn reading(&self, orientation: Orientation) -> TapeMark {
        let [offset_h_mm, offset_v_mm] = orientation.to_reading_mm(self.centre_mm);
        TapeMark {
            centroid_px: self.centroid_px,
            offset_h_mm,
            offset_v_mm,
            area_mm2: self.area_mm2,
        }
    }
}

/// Why a crop's marks could not be looked for. [M46 P11]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkMiss {
    /// The band held too few samples, or too few that agree, or a white too dark to see a mark
    /// against. The tape reader hands such a label to the golfer to pick.
    NoWhite,
}

/// Every spot on the label in `crop`, in the order a row-major scan meets them, or why none could
/// be looked for. The module doc gives the steps. [M46 P11]
pub fn find_marks(crop: &Crop, label: &LabelType) -> Result<Vec<Spot>, MarkMiss> {
    let interior = Interior::of(crop, label);
    let white = White::fit(crop, &interior).ok_or(MarkMiss::NoWhite)?;
    Ok(spots(
        &darkening(crop, &interior, &white),
        &interior,
        crop.grid,
    ))
}

/// What one photo of a fresh label holds: the plan's C5, the single-mark rule P12 reuses when the
/// previous crop's marks are gone. [M46 P11]
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Fresh {
    /// One spot, whole and round enough to be one mark.
    Mark(Spot),
    /// One spot, cut by the label's edge.
    Clipped,
    Unmarked,
    /// Two or more spots, or one too long to be one mark: which is new is the golfer's to say.
    Pick,
}

pub(super) fn on_a_fresh_label(spots: &[Spot]) -> Fresh {
    match spots {
        [] => Fresh::Unmarked,
        [one] if one.clipped => Fresh::Clipped,
        [one] if one.elongation > MARK_MAX_ELONGATION => Fresh::Pick,
        [one] => Fresh::Mark(*one),
        _ => Fresh::Pick,
    }
}

/// How far each crop pixel lies inside the label, millimetres, and so which pixels are its
/// interior (the module doc's step 1).
pub(super) struct Interior {
    /// From each pixel's centre to the nearest pixel centre that is not the label's: outside the
    /// outline, outside the photo, or on the crop's border. 0 on those.
    pub(super) inner_mm: Vec<f64>,
}

impl Interior {
    pub(super) fn of(crop: &Crop, label: &LabelType) -> Interior {
        let grid = crop.grid;
        let (width, height) = (grid.width, grid.height);
        let mut off_label = vec![true; width * height];
        let outline = label.outline_mm;
        let mut crossings = Vec::new();
        // Each row's pixel centres between pairs of the outline's crossings are inside it, the
        // crossing count's rule run along a scanline.
        for row in 0..height {
            let y = grid.to_label_mm([0.0, row as f64 + 0.5])[1];
            crossings.clear();
            for (i, &[x0, y0]) in outline.iter().enumerate() {
                let [x1, y1] = outline[(i + 1) % outline.len()];
                if (y0 > y) != (y1 > y) {
                    crossings.push(x0 + (y - y0) * (x1 - x0) / (y1 - y0));
                }
            }
            crossings.sort_by(f64::total_cmp);
            for pair in crossings.chunks_exact(2) {
                // Column `c`'s centre is at `c + 0.5`, so it lies strictly between the two when
                // `from < c < to`.
                let [from, to] = [pair[0], pair[1]].map(|x| grid.to_crop_px([x, y])[0] - 0.5);
                let first = (from.floor() + 1.0).clamp(0.0, width as f64) as usize;
                let end = (to.ceil().clamp(0.0, width as f64) as usize).max(first);
                off_label[row * width + first..row * width + end].fill(false);
            }
        }
        for (at, off) in off_label.iter_mut().enumerate() {
            let (column, row) = (at % width, at / width);
            let border = column == 0 || row == 0 || column + 1 == width || row + 1 == height;
            *off |= crop.outside[at] || border;
        }
        let to_off = squared_distance(&off_label, width, height);
        Interior {
            inner_mm: to_off.iter().map(|&d| d.sqrt() / grid.px_per_mm).collect(),
        }
    }

    pub(super) fn holds(&self, at: usize) -> bool {
        self.inner_mm[at] >= MARK_EDGE_MARGIN_MM
    }
}

/// The label's clean white over the crop, a plane in its pixels (the module doc's step 2).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct White {
    /// `[level, per column, per row]`, about `centre`.
    plane: [f64; 3],
    centre: [f64; 2],
}

impl White {
    /// The white at a point in the crop's continuous pixel coordinates.
    pub(super) fn at(&self, [x, y]: [f64; 2]) -> f64 {
        let [level, per_column, per_row] = self.plane;
        level + per_column * (x - self.centre[0]) + per_row * (y - self.centre[1])
    }

    pub(super) fn fit(crop: &Crop, interior: &Interior) -> Option<White> {
        let grid = crop.grid;
        let centre = [grid.width as f64 / 2.0, grid.height as f64 / 2.0];
        let band = MARK_EDGE_MARGIN_MM..=MARK_EDGE_MARGIN_MM + WHITE_BAND_MM;
        let samples: Vec<[f64; 3]> = (0..grid.width * grid.height)
            .filter(|&at| band.contains(&interior.inner_mm[at]))
            .map(|at| {
                let [x, y] = pixel_centre(at, grid.width);
                [x - centre[0], y - centre[1], value(crop, at)]
            })
            .collect();
        if samples.len() < WHITE_MIN_SAMPLES {
            return None;
        }
        let mut plane = [
            crate::median(samples.iter().map(|s| s[2]).collect())?,
            0.0,
            0.0,
        ];
        for _ in 0..WHITE_ROUNDS {
            let residual = |s: &[f64; 3]| s[2] - (plane[0] + plane[1] * s[0] + plane[2] * s[1]);
            // The spread over every sample: the median ignores the printing's share of them.
            let spread = (1.4826
                * crate::median(samples.iter().map(|s| residual(s).abs()).collect())?)
            .max(WHITE_MIN_SPREAD);
            let kept: Vec<[f64; 3]> = samples
                .iter()
                .filter(|s| residual(s).abs() <= WHITE_KEEP_SPREADS * spread)
                .copied()
                .collect();
            // The white must be most of its own band, or the band is not the label's white.
            if kept.len() * 2 < samples.len() {
                return None;
            }
            plane = least_squares_plane(&kept)?;
        }
        // The white is the band's brightest class. A plane with many samples well above it was
        // fitted to something darker, a band mostly in shadow or printing, and is no white.
        let residual = |s: &[f64; 3]| s[2] - (plane[0] + plane[1] * s[0] + plane[2] * s[1]);
        let spread = (1.4826 * crate::median(samples.iter().map(|s| residual(s).abs()).collect())?)
            .max(WHITE_MIN_SPREAD);
        let brighter = samples
            .iter()
            .filter(|s| residual(s) > WHITE_KEEP_SPREADS * spread)
            .count();
        if brighter as f64 > WHITE_MAX_BRIGHTER_SHARE * samples.len() as f64 {
            return None;
        }
        let white = White { plane, centre };
        let (w, h) = (grid.width as f64, grid.height as f64);
        let darkest = [[0.0, 0.0], [w, 0.0], [w, h], [0.0, h]]
            .map(|corner| white.at(corner))
            .into_iter()
            .fold(f64::INFINITY, f64::min);
        (darkest >= WHITE_MIN_LEVEL).then_some(white)
    }
}

/// Each pixel's darkening against the white, `(white − luminance) / white`, on the interior, and 0
/// off it (the module doc's step 3).
pub(super) fn darkening(crop: &Crop, interior: &Interior, white: &White) -> Vec<f64> {
    let width = crop.grid.width;
    (0..crop.grid.width * crop.grid.height)
        .map(|at| {
            if interior.holds(at) {
                let level = white.at(pixel_centre(at, width));
                (level - value(crop, at)) / level
            } else {
                0.0
            }
        })
        .collect()
}

/// The spots of a darkening over a crop's grid (the module doc's steps 3 to 6). Every pixel in a
/// spot is at least [`MARK_MIN_CONTRAST`] dark, so each spot's weights are positive.
pub(super) fn spots(field: &[f64], interior: &Interior, grid: CropGrid) -> Vec<Spot> {
    let (width, height) = (grid.width, grid.height);
    let px = grid.px_per_mm;
    let dark: Vec<bool> = (0..width * height)
        .map(|at| interior.holds(at) && field[at] >= MARK_MIN_CONTRAST)
        .collect();
    let mask = Mask {
        rect: Rect {
            x0: 0,
            y0: 0,
            x1: width,
            y1: height,
        },
        bits: opening(&dark, width, height, MARK_OPEN_RADIUS_MM * px),
    };
    let min_area = (MARK_MIN_AREA_MM2 * px * px).ceil() as usize;
    components_with_pixels(&mask, min_area)
        .into_iter()
        .map(|(_, pixels)| {
            let (mut total, mut sum) = (0.0, [0.0; 2]);
            let mut nearest_rim = f64::INFINITY;
            for &at in &pixels {
                let weight = field[at];
                let [x, y] = pixel_centre(at, width);
                total += weight;
                sum[0] += weight * x;
                sum[1] += weight * y;
                nearest_rim = nearest_rim.min(interior.inner_mm[at]);
            }
            let centroid_px = [sum[0] / total, sum[1] / total];
            Spot {
                centroid_px,
                centre_mm: grid.to_label_mm(centroid_px),
                area_mm2: pixels.len() as f64 / (px * px),
                elongation: elongation(&pixels, width),
                clipped: nearest_rim < MARK_EDGE_MARGIN_MM + CLIP_REACH_MM,
            }
        })
        .collect()
}

/// `√(λ₁ / λ₂)` of the pixels' second central moments, unweighted: the shape of the spot, not of
/// its darkness. Moments rather than the extents `component` measures, so a stray pixel on the
/// rim does not lengthen it.
fn elongation(pixels: &[usize], width: usize) -> f64 {
    let n = pixels.len() as f64;
    let centres: Vec<[f64; 2]> = pixels.iter().map(|&at| pixel_centre(at, width)).collect();
    let mean = centres
        .iter()
        .fold([0.0; 2], |s, c| [s[0] + c[0] / n, s[1] + c[1] / n]);
    let [mut xx, mut xy, mut yy] = [0.0; 3];
    for [x, y] in &centres {
        let (dx, dy) = (x - mean[0], y - mean[1]);
        xx += dx * dx;
        xy += dx * dy;
        yy += dy * dy;
    }
    let half_trace = (xx + yy) / 2.0;
    let root = (((xx - yy) / 2.0).powi(2) + xy * xy).sqrt();
    let (major, minor) = (half_trace + root, half_trace - root);
    if minor > 0.0 {
        (major / minor).sqrt()
    } else {
        f64::INFINITY
    }
}

/// The least-squares plane `level + per_column·x + per_row·y` through `[x, y, value]` samples, by
/// its 3 × 3 normal equations and Cramer's rule, or `None` when they are singular: samples on one
/// line, which a band round a label never is.
fn least_squares_plane(samples: &[[f64; 3]]) -> Option<[f64; 3]> {
    let mut a = [[0.0; 3]; 3];
    let mut b = [0.0; 3];
    for &[x, y, v] in samples {
        let row = [1.0, x, y];
        for i in 0..3 {
            for j in 0..3 {
                a[i][j] += row[i] * row[j];
            }
            b[i] += row[i] * v;
        }
    }
    let det = |m: &[[f64; 3]; 3]| {
        m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
            - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
            + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
    };
    let whole = det(&a);
    if !whole.is_finite() || whole.abs() <= f64::EPSILON * a[0][0].powi(3) {
        return None;
    }
    Some(std::array::from_fn(|k| {
        let mut replaced = a;
        for (row, value) in replaced.iter_mut().zip(b) {
            row[k] = value;
        }
        det(&replaced) / whole
    }))
}

/// Pixel `at`'s centre, row-major over a grid `width` across.
fn pixel_centre(at: usize, width: usize) -> [f64; 2] {
    [(at % width) as f64 + 0.5, (at / width) as f64 + 0.5]
}

/// Pixel `at`'s luminance in the crop.
fn value(crop: &Crop, at: usize) -> f64 {
    let rgb = &crop.frame.rgb[at * 3..at * 3 + 3];
    luminance([rgb[0], rgb[1], rgb[2]])
}

#[cfg(test)]
mod tests {
    use super::super::{rectify, Homography, CROP_PX_PER_MM, STRIKEPRO_HYBRID, STRIKEPRO_IRON};
    use super::*;
    use crate::frame::Frame;

    /// A label's crop, each pixel `colour` at its centre's printed-frame millimetres, cut
    /// one-to-one from a photo that is the crop itself, so the crop holds those colours exactly.
    fn crop_of(label: &LabelType, colour: impl Fn([f64; 2]) -> [u8; 3]) -> Crop {
        let grid = CropGrid::new(label, CROP_PX_PER_MM);
        let mut rgb = Vec::with_capacity(grid.width * grid.height * 3);
        for at in 0..grid.width * grid.height {
            rgb.extend(colour(grid.to_label_mm(pixel_centre(at, grid.width))));
        }
        let photo = Frame {
            width: grid.width,
            height: grid.height,
            rgb,
        };
        let [w, h] = [grid.width as f64, grid.height as f64];
        let map = Homography::from_corners(
            [[0.0, 0.0], [w, 0.0], [w, h], [0.0, h]],
            label.box_corners_mm(),
        )
        .unwrap();
        rectify(&photo, &map, label, CROP_PX_PER_MM)
    }

    #[test]
    fn the_interior_is_the_outline_less_its_rim() {
        let crop = crop_of(&STRIKEPRO_IRON, |_| [200; 3]);
        let interior = Interior::of(&crop, &STRIKEPRO_IRON);
        let grid = crop.grid;
        let at = |mm: [f64; 2]| {
            let [x, y] = grid.to_crop_px(mm);
            y as usize * grid.width + x as usize
        };
        // The cross is 14.6 mm above the bottom edge, the nearest (P3 finding 4).
        assert!((interior.inner_mm[at([0.0, 0.0])] - 14.55).abs() < 0.2);
        assert!(interior.holds(at([0.0, -13.0])));
        assert!(!interior.holds(at([0.0, -14.0])));
        // The top-left corner of the box is off the shouldered outline.
        assert_eq!(interior.inner_mm[at([-35.0, 23.0])], 0.0);
    }

    #[test]
    fn the_white_is_a_plane_fitted_through_the_printing_it_crosses() {
        // A gradient, rising 0.4 levels a millimetre across and falling 0.3 a millimetre up,
        // crossed by dark stripes 1 mm wide every 6 mm, a sixth of the band, as printing crosses it.
        let label = &STRIKEPRO_HYBRID;
        let clean = |[x, y]: [f64; 2]| 200.0 + 0.4 * x - 0.3 * y;
        let crop = crop_of(label, |[x, y]| {
            let level = if x.rem_euclid(6.0) < 1.0 {
                60.0
            } else {
                clean([x, y])
            };
            [level.round() as u8; 3]
        });
        let interior = Interior::of(&crop, label);
        let white = White::fit(&crop, &interior).expect("a white");
        for mm in [[0.0, 0.0], [-30.0, 10.0], [30.0, -10.0]] {
            let px = crop.grid.to_crop_px(mm);
            assert!((white.at(px) - clean(mm)).abs() < 0.6, "{mm:?}");
        }
    }

    #[test]
    fn a_band_that_is_mostly_dark_has_no_white() {
        let crop = crop_of(&STRIKEPRO_IRON, |[x, _]| {
            if x.rem_euclid(2.0) < 1.2 {
                [40; 3]
            } else {
                [220; 3]
            }
        });
        let interior = Interior::of(&crop, &STRIKEPRO_IRON);
        assert_eq!(White::fit(&crop, &interior), None);
    }

    #[test]
    fn a_disc_is_round_and_two_run_together_are_long() {
        let width = 200;
        let disc = |centres: &[[f64; 2]]| -> Vec<usize> {
            (0..width * width)
                .filter(|&at| {
                    let [x, y] = pixel_centre(at, width);
                    centres.iter().any(|c| (x - c[0]).hypot(y - c[1]) <= 30.0)
                })
                .collect()
        };
        assert!((elongation(&disc(&[[100.0, 100.0]]), width) - 1.0).abs() < 0.01);
        // Centres 40 apart, two thirds of a diameter: about √(1 + (40/30)²) ≈ 1.67 apart from the
        // overlap.
        let pair = elongation(&disc(&[[80.0, 100.0], [120.0, 100.0]]), width);
        assert!(pair > 1.4 && pair < 1.9, "{pair}");
    }

    #[test]
    fn the_fresh_label_rule_reads_one_whole_round_spot_and_nothing_else() {
        let spot = Spot {
            centroid_px: [10.0, 10.0],
            centre_mm: [1.0, 2.0],
            area_mm2: 100.0,
            elongation: 1.05,
            clipped: false,
        };
        let clipped = Spot {
            clipped: true,
            ..spot
        };
        let long = Spot {
            elongation: 1.5,
            ..spot
        };
        assert_eq!(on_a_fresh_label(&[]), Fresh::Unmarked);
        assert_eq!(on_a_fresh_label(&[spot]), Fresh::Mark(spot));
        assert_eq!(on_a_fresh_label(&[clipped]), Fresh::Clipped);
        assert_eq!(on_a_fresh_label(&[long]), Fresh::Pick);
        assert_eq!(on_a_fresh_label(&[spot, spot]), Fresh::Pick);
        assert_eq!(on_a_fresh_label(&[spot, clipped]), Fresh::Pick);
        let right_handed = Orientation {
            toe_printed_right: false,
            top_line_printed_top: true,
        };
        let mark = spot.reading(right_handed);
        assert_eq!([mark.offset_h_mm, mark.offset_v_mm], [-1.0, 2.0]);
        assert_eq!(mark.area_mm2, 100.0);
    }
}
