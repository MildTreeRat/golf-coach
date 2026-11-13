//! The tape reader's geometry: four corners in a photo to the label's millimetres and back, and the
//! label cut out of the photo and squared to its own frame. [M46 P7]
//!
//! # Why this crate writes its own homography
//!
//! `crates/ocr` already holds one, `cv::warp`'s `get_perspective_transform` and `warp_perspective`,
//! and the M46 plan's C1 forbids reaching it. Those are OpenCV-faithful ports on `image::RgbImage`,
//! inside the one crate that may reach `ort`, and this crate is on the phone's path
//! (`crates/ocr/tests/ort_reach.rs`). They also answer a different question: theirs must match
//! OpenCV's pixels, and this has no oracle to match at all, only the geometry. So this one
//! normalises its points before the solve, which OpenCV's does not, and refuses a quad no photo could
//! give rather than returning OpenCV's zero matrix. `docs/REFACTOR_LEDGER.md` records the
//! near-duplicate and why it stays.
//!
//! # The three frames
//!
//! - **The photo**: continuous pixel coordinates, `x` right and `y` down, with pixel `(i, j)`
//!   covering `[i, i + 1) × [j, j + 1)` (the crate doc's convention).
//! - **The label**: millimetres in the printed frame (the module doc): the origin at the printed
//!   cross, `+x` toward the printed right and `+y` toward the logo's edge. It is not C4's reading
//!   frame. The reading converts to that once the hosel has said which side is the toe (P10).
//! - **The crop**: [`CropGrid`], continuous pixel coordinates over the label's box at a fixed
//!   number of pixels a millimetre, with row 0 along the box's top edge. So a crop shows the label
//!   as it is printed, logo up, whichever way the photo held it.
//!
//! # What refuses, and what does not
//!
//! [`Homography::from_corners`] refuses four corners that no photo of a flat, opaque label could
//! give ([`Degenerate`]). Whether they are *this* label's corners is not judged here. The aspect
//! check is the label finder's (P9).

use super::LabelType;
use crate::frame::Frame;

/// The rectified crop's pixels per millimetre (the M46 plan's C4, provisional), the same for every
/// label type so that a crop's pixels are one size everywhere. [M46 P7]
///
/// **10, set from the photos' own sampling and the crop's bytes.** No mark's size is known before
/// P29 (P3 finding 10), so the mark is bounded rather than measured:
/// - **The photo's sampling.** On P3's flat photo, which arrived at half an iPhone's size (P3
///   finding 2), the labels' boxes span about 7.4 (driver), 8.3 (hybrid) and 8.8 to 9.2 (iron)
///   pixels a millimetre, read off the photo by eye. A full-size photo framed the same way samples
///   twice that, about 15 to 18. 10 keeps most of a full-size photo's detail and magnifies a
///   half-size one by at most 1.35. At 5, a crop would discard detail the photo holds before P11
///   and P12 have tried to use it.
/// - **The mark.** At 10, a pixel is 0.1 mm. The ladder's tightest radius (5 mm, `contracts::drill`)
///   and C24's `WRONG_MM` (5 mm) are each 50 pixels. A mark even 2 mm across covers about 300
///   pixels, so its centroid rests on hundreds of pixels at any size a ball could leave.
/// - **The bytes.** The crops are 905 × 496 (driver), 735 × 344 (hybrid) and 730 × 399 (iron)
///   pixels, which is 0.74 to 1.3 MB as the raw RGB C16 keeps for the diff, one per club. As JPEG,
///   the three boxes cut from P3's photo by eye and resized (Pillow, bilinear) came to 36 to 58 KB
///   at quality 75 and 47 to 76 KB at quality 85, D6's "tens of KB". The grid paper in the boxes'
///   corners adds bytes a club head may not. At 8 they were 27 to 56 KB, at 5 12 to 26 KB, and at 12
///   47 to 99 KB.
///
/// **What revises it**: P29's first real marks, their size and whether the diff (P12) needs finer
/// registration, and the JPEG quality P18 chooses. Changing it changes every crop's size, so after
/// P13 it moves with `TAPE_READER_VERSION`.
pub const CROP_PX_PER_MM: f64 = 10.0;

/// How close to a line three of the four corners may lie, as twice their triangle's area once
/// [`normalise`] has set the four corners' mean distance from their centroid to `√2`. A square
/// there gives 4.
///
/// It is numerical, not a judgment of the label's shape. A square label foreshortened by a tilt `θ`
/// gives `8 cos θ / (1 + cos² θ)`, so the tolerance refuses only past about 89.99999°, where the
/// solve would be ill-conditioned anyway. Refusing a plausible quad that is merely steep is the label finder's
/// aspect check (P9).
const COLLINEAR_TOLERANCE: f64 = 1e-6;

/// The 8 × 8 solve's smallest usable pivot, on the normalised system, whose entries are of order
/// 1. It backstops [`COLLINEAR_TOLERANCE`] and is not expected to fire past it.
const PIVOT_TOLERANCE: f64 = 1e-12;

/// The four triples of four corners, each leaving one out.
const TRIPLES: [[usize; 3]; 4] = [[1, 2, 3], [0, 2, 3], [0, 1, 3], [0, 1, 2]];

/// A plane-to-plane perspective map from photo pixels to label millimetres, and its inverse.
///
/// It is held as both 3 × 3 matrices, each scaled so that its `w` (the third homogeneous
/// coordinate) is positive on the label's side of the horizon. So [`Homography::inverse`] cannot
/// fail, and [`Homography::apply`] can tell a point on the label's plane in front of the camera from
/// one beyond its horizon.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Homography {
    forward: [[f64; 3]; 3],
    backward: [[f64; 3]; 3],
}

/// Why four corners make no homography a photo of a label could have. [M46 P7]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Degenerate {
    /// A coordinate is NaN or infinite.
    NotFinite,
    /// Three corners on one side lie on a line, or two coincide, so no plane-to-plane map pairs
    /// the four.
    Collinear,
    /// The map would carry part of the quad through infinity. The corners cross (a bow-tie), or one
    /// lies inside the triangle of the other three. A camera sees a flat label's convex outline as
    /// convex, all of it on one side of the plane's horizon, so no photo gives this.
    Folded,
    /// The corners pair up as a mirror image, so the photo would show the label from behind. A
    /// STRIKEPRO label is opaque (P3 finding 4), so the corners are in the wrong order, or the photo
    /// itself was mirrored. Reading through such a map would put every mark on the wrong side of the
    /// cross, a guessed sign (the M46 plan's rule 6).
    Mirrored,
}

impl Homography {
    /// The map taking each `photo` corner, in photo pixels, to the `label` corner at the same
    /// index, in label millimetres.
    ///
    /// The direct solve: with `h₃₃` fixed at 1, each pair gives two linear equations in the other
    /// eight entries, and the 8 × 8 system is solved by Gaussian elimination with partial pivoting.
    /// Each side is first moved and scaled so that its centroid is the origin and its mean distance
    /// from it is `√2` (Hartley's normalisation). That makes the system's entries of order 1 whether
    /// the photo is 1000 or 4000 pixels wide, and puts the origin inside the quad, where `h₃₃` cannot
    /// be 0 for a quad that is not [`Degenerate::Folded`].
    ///
    /// The corners may be in any order, so long as both sides list them in the same one. Seen from
    /// in front, the photo's `y` runs down and the label's up, so the two lists have opposite signed
    /// areas: top-left, top-right, bottom-right, bottom-left runs clockwise on the screen and on the
    /// printed label alike, and its shoelace sum is positive in the photo's numbers and negative in
    /// the label's. [`Degenerate::Mirrored`] is a pair of lists whose signs agree.
    pub fn from_corners(
        photo: [[f64; 2]; 4],
        label: [[f64; 2]; 4],
    ) -> Result<Homography, Degenerate> {
        if photo.iter().chain(&label).flatten().any(|c| !c.is_finite()) {
            return Err(Degenerate::NotFinite);
        }
        let (from, from_points) = normalise(photo)?;
        let (to, to_points) = normalise(label)?;
        let solved = solve_normalised(from_points, to_points).ok_or(Degenerate::Collinear)?;
        let forward = multiply(&multiply(&to.undo(), &solved), &from.matrix());
        // `w` is affine in the photo point and is 1 at the corners' centroid (the normalised
        // origin, where `h₃₃` is), so its four values average to 1. One at or below 0 means the
        // horizon runs through the quad.
        if photo
            .iter()
            .any(|&corner| !is_positive(w_at(&forward, corner)))
        {
            return Err(Degenerate::Folded);
        }
        let det = determinant(&forward);
        // With `w` positive, the sign of `det` is the sign of the map's local area scale at every
        // point of the quad. The photo's `y` runs down and the label's up, so a front view is
        // negative.
        if det > 0.0 {
            return Err(Degenerate::Mirrored);
        }
        let backward = invert(&forward, det).ok_or(Degenerate::Collinear)?;
        Ok(Homography { forward, backward })
    }

    /// The label point a photo point maps to, or `None` when the photo point is on or beyond the
    /// label plane's horizon, which no point of the label is.
    pub fn apply(&self, point: [f64; 2]) -> Option<[f64; 2]> {
        project(&self.forward, point)
    }

    /// The map the other way, from label millimetres to photo pixels.
    pub fn inverse(&self) -> Homography {
        Homography {
            forward: self.backward,
            backward: self.forward,
        }
    }

    /// The forward matrix, row-major, acting on `(x, y, 1)`. For a review image or a log line.
    /// Its scale is arbitrary, and positive on the label's side of the horizon.
    pub fn matrix(&self) -> [[f64; 3]; 3] {
        self.forward
    }
}

impl LabelType {
    /// The corners of the label's box in the printed frame: top-left, top-right, bottom-right,
    /// bottom-left, as a photo of the label stuck logo-up and seen face-on shows them. [M46 P7]
    ///
    /// The box is the record's, [`LabelType::width_mm`] by [`LabelType::height_mm`] placed about the
    /// cross by [`LabelType::centre_from_left_mm`] and [`LabelType::centre_from_top_mm`], which the
    /// outline touches on each side to within 0.2 mm (P4 finding 3). None of the labels is a
    /// rectangle, so these are not points on the label. They are where the lines along its four
    /// extremes meet, which is what the label finder fits (P9) and what [`CropGrid`] spans.
    pub fn box_corners_mm(&self) -> [[f64; 2]; 4] {
        let left = -self.centre_from_left_mm;
        let right = self.width_mm - self.centre_from_left_mm;
        let top = self.centre_from_top_mm;
        let bottom = self.centre_from_top_mm - self.height_mm;
        [[left, top], [right, top], [right, bottom], [left, bottom]]
    }
}

/// The rectified crop's pixel grid: the label's box at `px_per_mm`, with row 0 along its top edge.
/// [M46 P7]
///
/// It is a value of its own, apart from the pixels, because a stored crop is read back by its grid
/// alone. A tap on the stored JPEG becomes millimetres through it (C6), and so do the
/// normalised label coordinates the app paints (C20).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CropGrid {
    /// Pixels across, the box's width at `px_per_mm`, rounded.
    pub width: usize,
    /// Pixels down, the box's height at `px_per_mm`, rounded.
    pub height: usize,
    pub px_per_mm: f64,
    /// The printed-frame `x` of the crop's left edge, the box's left edge.
    pub left_mm: f64,
    /// The printed-frame `y` of the crop's top edge, the box's top edge.
    pub top_mm: f64,
}

impl CropGrid {
    /// The grid for one label type.
    ///
    /// The width and height are rounded, so the last column or row may end up to half a pixel short
    /// of the box or past it. The map between pixels and millimetres is exact either way, because it
    /// runs from the top-left corner at `px_per_mm`.
    ///
    /// # Panics
    ///
    /// When `px_per_mm` is not a positive finite number. It is [`CROP_PX_PER_MM`] or a test's own,
    /// so another value is a wiring bug (`docs/CODE_STANDARDS.md` R8).
    pub fn new(label: &LabelType, px_per_mm: f64) -> CropGrid {
        assert!(
            px_per_mm.is_finite() && px_per_mm > 0.0,
            "a crop's pixels per millimetre must be positive and finite, not {px_per_mm}"
        );
        CropGrid {
            width: (label.width_mm * px_per_mm).round() as usize,
            height: (label.height_mm * px_per_mm).round() as usize,
            px_per_mm,
            left_mm: -label.centre_from_left_mm,
            top_mm: label.centre_from_top_mm,
        }
    }

    /// The printed-frame millimetres of a point in the crop's continuous pixel coordinates. Pixel
    /// `(i, j)`'s centre is `(i + 0.5, j + 0.5)`.
    pub fn to_label_mm(&self, [x, y]: [f64; 2]) -> [f64; 2] {
        [
            self.left_mm + x / self.px_per_mm,
            self.top_mm - y / self.px_per_mm,
        ]
    }

    /// The crop's continuous pixel coordinates of a point in printed-frame millimetres.
    pub fn to_crop_px(&self, [x, y]: [f64; 2]) -> [f64; 2] {
        [
            (x - self.left_mm) * self.px_per_mm,
            (self.top_mm - y) * self.px_per_mm,
        ]
    }
}

/// The label cut out of a photo and squared to its own frame: what [`rectify`] returns. [M46 P7]
#[derive(Debug, Clone, PartialEq)]
pub struct Crop {
    pub grid: CropGrid,
    /// `grid.width × grid.height` pixels. A pixel in [`Crop::outside`] is black and holds nothing.
    pub frame: Frame,
    /// One flag a pixel, row-major: its sample fell outside the photo, so the crop holds nothing
    /// there. A separate mask because a [`Frame`] has no alpha, and any colour chosen to mean
    /// "nothing" is a colour a photo can hold.
    pub outside: Vec<bool>,
}

/// The label in `frame` squared to its printed frame at `px_per_mm`, through `photo_to_label`, the
/// map [`Homography::from_corners`] builds from the photo to the label. [M46 P7]
///
/// Each crop pixel's centre is taken to label millimetres by its [`CropGrid`], to the photo by the
/// inverse map, and sampled there bilinearly between the four nearest pixel centres. A sample in the
/// photo's outermost half pixel takes the edge pixel's value. A sample outside the photo is marked
/// in [`Crop::outside`] and left black, and so is every pixel when `frame` is not well formed
/// ([`Frame::is_well_formed`]), so a crop is always its grid's size.
///
/// There is no smoothing before the sampling. A photo that samples the label more finely than the
/// crop does is decimated, and its finest printing may alias. [`CROP_PX_PER_MM`] says why that
/// costs nothing a mark needs.
pub fn rectify(
    frame: &Frame,
    photo_to_label: &Homography,
    label: &LabelType,
    px_per_mm: f64,
) -> Crop {
    let grid = CropGrid::new(label, px_per_mm);
    let pixels = grid.width * grid.height;
    let mut rgb = vec![0; pixels * 3];
    let mut outside = vec![true; pixels];
    let readable = frame.is_well_formed() && frame.width > 0 && frame.height > 0;
    for row in 0..grid.height {
        for column in 0..grid.width {
            let mm = grid.to_label_mm([column as f64 + 0.5, row as f64 + 0.5]);
            let sampled = project(&photo_to_label.backward, mm)
                .filter(|_| readable)
                .and_then(|at| sample(frame, at));
            if let Some(colour) = sampled {
                let at = row * grid.width + column;
                rgb[at * 3..at * 3 + 3].copy_from_slice(&colour);
                outside[at] = false;
            }
        }
    }
    Crop {
        grid,
        frame: Frame {
            width: grid.width,
            height: grid.height,
            rgb,
        },
        outside,
    }
}

/// The frame's colour at a continuous point, bilinear between the four nearest pixel centres, or
/// `None` outside `[0, width) × [0, height)`. The caller has checked the frame is well formed and
/// not empty.
fn sample(frame: &Frame, [x, y]: [f64; 2]) -> Option<[u8; 3]> {
    let inside = (0.0..frame.width as f64).contains(&x) && (0.0..frame.height as f64).contains(&y);
    if !inside {
        return None;
    }
    // Pixel centres sit at `i + 0.5`, so the outermost half pixel is clamped to the edge's centre.
    let axis = |at: f64, size: usize| {
        let along = (at - 0.5).clamp(0.0, (size - 1) as f64);
        let low = along.floor() as usize;
        (low, (low + 1).min(size - 1), along - low as f64)
    };
    let (x0, x1, tx) = axis(x, frame.width);
    let (y0, y1, ty) = axis(y, frame.height);
    let [a, b, c, d] = [
        frame.pixel(x0, y0),
        frame.pixel(x1, y0),
        frame.pixel(x0, y1),
        frame.pixel(x1, y1),
    ];
    Some(std::array::from_fn(|k| {
        let top = f64::from(a[k]) * (1.0 - tx) + f64::from(b[k]) * tx;
        let bottom = f64::from(c[k]) * (1.0 - tx) + f64::from(d[k]) * tx;
        // A weighted mean of bytes stays within 0 to 255, so the cast only rounds.
        (top * (1.0 - ty) + bottom * ty).round() as u8
    }))
}

/// A translation and a uniform scale taking points to Hartley's normal position.
struct Similarity {
    scale: f64,
    centre: [f64; 2],
}

impl Similarity {
    fn matrix(&self) -> [[f64; 3]; 3] {
        let (scale, [x, y]) = (self.scale, self.centre);
        [
            [scale, 0.0, -scale * x],
            [0.0, scale, -scale * y],
            [0.0, 0.0, 1.0],
        ]
    }

    /// The inverse of [`Similarity::matrix`], written out rather than inverted.
    fn undo(&self) -> [[f64; 3]; 3] {
        let (scale, [x, y]) = (self.scale, self.centre);
        [
            [1.0 / scale, 0.0, x],
            [0.0, 1.0 / scale, y],
            [0.0, 0.0, 1.0],
        ]
    }
}

/// The four points moved so their centroid is the origin and scaled so their mean distance from it
/// is `√2`, with the similarity that did it. Refuses [`Degenerate::Collinear`] when any three of
/// them lie within [`COLLINEAR_TOLERANCE`] of a line, which covers two that coincide and all four
/// at one point.
fn normalise(points: [[f64; 2]; 4]) -> Result<(Similarity, [[f64; 2]; 4]), Degenerate> {
    let centre = [0, 1].map(|axis| points.iter().map(|p| p[axis]).sum::<f64>() / 4.0);
    let spread = points
        .iter()
        .map(|p| (p[0] - centre[0]).hypot(p[1] - centre[1]))
        .sum::<f64>()
        / 4.0;
    let scale = std::f64::consts::SQRT_2 / spread;
    // All four at one point leaves no spread to divide by, and coordinates near f64's limit leave
    // no finite scale.
    if !(scale.is_finite() && scale > 0.0) {
        return Err(Degenerate::Collinear);
    }
    let normal = points.map(|p| [(p[0] - centre[0]) * scale, (p[1] - centre[1]) * scale]);
    for [i, j, k] in TRIPLES {
        let (a, b, c) = (normal[i], normal[j], normal[k]);
        let twice_area = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
        if twice_area.abs() <= COLLINEAR_TOLERANCE {
            return Err(Degenerate::Collinear);
        }
    }
    Ok((Similarity { scale, centre }, normal))
}

/// The matrix with `h₃₃ = 1` taking each `from` point to its `to` point, or `None` when the system
/// is singular. Both sides are normalised ([`normalise`]).
///
/// For a pair `(x, y) → (u, v)`, `u (h₃₁x + h₃₂y + 1) = h₁₁x + h₁₂y + h₁₃`, and likewise `v` with
/// the second row: two equations linear in the eight unknowns.
fn solve_normalised(from: [[f64; 2]; 4], to: [[f64; 2]; 4]) -> Option<[[f64; 3]; 3]> {
    let mut system = [[0.0; 9]; 8];
    for (i, ([x, y], [u, v])) in from.into_iter().zip(to).enumerate() {
        system[i] = [x, y, 1.0, 0.0, 0.0, 0.0, -x * u, -y * u, u];
        system[i + 4] = [0.0, 0.0, 0.0, x, y, 1.0, -x * v, -y * v, v];
    }
    let h = gauss(system)?;
    Some([[h[0], h[1], h[2]], [h[3], h[4], h[5]], [h[6], h[7], 1.0]])
}

/// Gaussian elimination with partial pivoting on an 8 × 8 system with its right-hand side as the
/// ninth column, then back substitution. `None` when a pivot is at or below [`PIVOT_TOLERANCE`].
///
/// The label finder's outline fit (`label.rs`, M46 P9) solves its normal equations with it too,
/// since its homography has the same eight unknowns.
pub(super) fn gauss(mut system: [[f64; 9]; 8]) -> Option<[f64; 8]> {
    for column in 0..8 {
        let pivot = (column..8)
            .max_by(|&a, &b| system[a][column].abs().total_cmp(&system[b][column].abs()))?;
        if !is_positive(system[pivot][column].abs() - PIVOT_TOLERANCE) {
            return None;
        }
        system.swap(column, pivot);
        let lead = system[column];
        for row in system.iter_mut().skip(column + 1) {
            let factor = row[column] / lead[column];
            for (value, above) in row.iter_mut().zip(lead).skip(column) {
                *value -= factor * above;
            }
        }
    }
    let mut solution = [0.0; 8];
    for (index, row) in system.iter().enumerate().rev() {
        let known: f64 = (index + 1..8).map(|k| row[k] * solution[k]).sum();
        solution[index] = (row[8] - known) / row[index];
    }
    Some(solution)
}

/// `a · b`, 3 × 3.
fn multiply(a: &[[f64; 3]; 3], b: &[[f64; 3]; 3]) -> [[f64; 3]; 3] {
    std::array::from_fn(|i| std::array::from_fn(|j| (0..3).map(|k| a[i][k] * b[k][j]).sum()))
}

/// The cofactor of `m` at row `r`, column `c`. Taking the other two rows and columns in cyclic order
/// carries the cofactor's sign.
fn cofactor(m: &[[f64; 3]; 3], r: usize, c: usize) -> f64 {
    let (r0, r1, c0, c1) = ((r + 1) % 3, (r + 2) % 3, (c + 1) % 3, (c + 2) % 3);
    m[r0][c0] * m[r1][c1] - m[r0][c1] * m[r1][c0]
}

fn determinant(m: &[[f64; 3]; 3]) -> f64 {
    (0..3).map(|c| m[0][c] * cofactor(m, 0, c)).sum()
}

/// `m⁻¹`, from its adjugate and its determinant `det`, or `None` when `det` is 0 or not finite.
/// Dividing by `det` rather than keeping the bare adjugate keeps `w` positive: where `m` takes `p`
/// to `w q`, the inverse takes `q` to `p / w`.
fn invert(m: &[[f64; 3]; 3], det: f64) -> Option<[[f64; 3]; 3]> {
    if !(det.is_finite() && det != 0.0) {
        return None;
    }
    Some(std::array::from_fn(|i| {
        std::array::from_fn(|j| cofactor(m, j, i) / det)
    }))
}

/// The third homogeneous coordinate `m` gives a point.
fn w_at(m: &[[f64; 3]; 3], [x, y]: [f64; 2]) -> f64 {
    m[2][0] * x + m[2][1] * y + m[2][2]
}

/// Whether `value` is above 0, and so not NaN, which compares false with everything.
fn is_positive(value: f64) -> bool {
    value > 0.0
}

/// `m` applied to a point, or `None` when its `w` is not positive or the result is not finite.
fn project(m: &[[f64; 3]; 3], [x, y]: [f64; 2]) -> Option<[f64; 2]> {
    let [u, v, w] = m.map(|row| row[0] * x + row[1] * y + row[2]);
    if !is_positive(w) {
        return None;
    }
    Some([u / w, v / w]).filter(|point| point.iter().all(|c| c.is_finite()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cyclic_cofactors_invert_a_matrix() {
        let m = [[2.0, -1.0, 0.5], [0.3, 4.0, -2.0], [0.01, 0.02, 1.0]];
        let inverse = invert(&m, determinant(&m)).unwrap();
        let product = multiply(&m, &inverse);
        for (i, row) in product.iter().enumerate() {
            for (j, value) in row.iter().enumerate() {
                let identity = if i == j { 1.0 } else { 0.0 };
                assert!((value - identity).abs() < 1e-12, "{product:?}");
            }
        }
        // Rows one and two in proportion: no inverse.
        let singular = [[1.0, 2.0, 3.0], [2.0, 4.0, 6.0], [0.0, 0.0, 1.0]];
        assert_eq!(determinant(&singular), 0.0);
        assert_eq!(invert(&singular, 0.0), None);
    }

    #[test]
    fn a_sample_blends_the_four_nearest_centres_and_clamps_the_rim() {
        // Two by one: a black pixel and a white one.
        let frame = Frame {
            width: 2,
            height: 1,
            rgb: vec![0, 0, 0, 200, 200, 200],
        };
        // Halfway between the two centres.
        assert_eq!(sample(&frame, [1.0, 0.5]), Some([100; 3]));
        // A quarter of the way.
        assert_eq!(sample(&frame, [0.75, 0.5]), Some([50; 3]));
        // The outer half pixel takes the edge pixel, on every side.
        assert_eq!(sample(&frame, [0.1, 0.0]), Some([0; 3]));
        assert_eq!(sample(&frame, [1.99, 0.99]), Some([200; 3]));
        // Outside the frame, including its far edge, which is half-open.
        assert_eq!(sample(&frame, [2.0, 0.5]), None);
        assert_eq!(sample(&frame, [-0.01, 0.5]), None);
        assert_eq!(sample(&frame, [1.0, f64::NAN]), None);
    }

    #[test]
    fn a_point_beyond_the_horizon_has_no_image() {
        // `w = 1 − x`: the line x = 1 is the horizon.
        let m = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [-1.0, 0.0, 1.0]];
        assert_eq!(project(&m, [0.5, 2.0]), Some([1.0, 4.0]));
        assert_eq!(project(&m, [1.0, 2.0]), None);
        assert_eq!(project(&m, [3.0, 2.0]), None);
    }
}
