//! The label finder: where a STRIKEPRO label lies in a photo of it on the club, as the four corners
//! of its box, or no label at all. [M46 P9]
//!
//! # How it finds one
//!
//! 1. **Bright and neutral.** A label is white and opaque (P3 finding 4), so it is among the
//!    brightest things in a photo of a club face, and has no colour of its own. A pixel is a
//!    candidate when its luminance is above a level and its saturation at most
//!    [`LABEL_SATURATION_MAX`]. The level is Otsu's over the whole photo, and then Otsu's again over
//!    what passed the first. A steel face is the photo's second-brightest class or merges with the
//!    label in the brightest, depending on what else is in frame, and one of the two levels parts
//!    it from the label either way. Each level is tried.
//! 2. **A region the size of a label.** Each 4-connected region (the crate's `component`) large
//!    enough to be the label at [`MIN_PX_PER_MM`], and wholly inside the photo, has its holes
//!    filled. Those are the printing (the navy logo, the red cross and the black dashes) and any
//!    strike mark, all of which sit inside the label's white.
//! 3. **The label's proportions.** The filled region's second moments give it an aspect, which must
//!    lie within [`ASPECT_TOLERANCE`] of the outline's own.
//! 4. **The outline, fitted.** The region's edge is located to a fraction of a pixel, where the
//!    luminance crosses halfway between the label's white and whatever is beside it. Then the label
//!    type's own outline ([`LabelType::outline_mm`]) is fitted to it through a homography. The fit
//!    is Levenberg–Marquardt on each edge point's distance, in the photo, from the outline as the
//!    map projects it there. It is weighted robustly in three stages that narrow, so that a notch
//!    does not pull it. A notch comes from the logo reaching the edge (P3 finding 7) or a mark there.
//!    The fit starts twice from the region's moments, once each way along its long axis, because a
//!    moment cannot tell the logo's edge from the bottom one.
//! 5. **Accepted only when it fits, and the right way up.** At least [`MIN_INLIER_SHARE`] of the
//!    edge must lie within [`OUTLINE_INLIER_MM`] of the outline, and no more than
//!    [`MAX_OUTSIDE_SHARE`] beyond it. At least [`MIN_OUTLINE_SUPPORT`] of the outline must have
//!    edge beside it. At the cross the map must sample at least [`MIN_PX_PER_MM`], and foreshorten
//!    no more than [`ASPECT_TOLERANCE`]. And the band across the logo's letters must be darker than
//!    the band as deep in from the bottom edge by [`LOGO_CONTRAST`], which is what says the fit's
//!    top is the label's.
//! 6. **Exactly one.** Fits from either level and either start that land on the same label are one
//!    label, and the best is kept. Two labels that differ are refused as [`LabelMiss::Ambiguous`],
//!    never chosen between (the M46 plan's rule 6). That covers two labels of the type in one
//!    photo, and one label fitted two ways that each passed.
//!
//! The corners are the box's ([`LabelType::box_corners_mm`]), mapped into the photo through the
//! fitted map. So they pair with the printed frame by index, whichever way the label lies in the
//! photo, and the crop is cut logo-up (P7 finding 3).
//!
//! # Why not four edge lines
//!
//! The plan's P9 asked for a rounded rectangle's corners, from its four edge lines intersected. None
//! of the labels is a rectangle (P4 finding 2), and the driver label has no straight edge at all. So
//! the box's corners are points off the label, and no line in the photo runs through two of them
//! (P7 finding 3). Fitting the whole measured outline is the general form of the same idea. The
//! corners still come from edges and never from the rounded pixels, and every edge point the region
//! has is used.
//!
//! **The aspect check runs on the region, before the fit.** The plan put it on the squared crop, but
//! a crop cut through a fitted outline has the label's aspect by construction. So the check that
//! refuses a non-label by its proportions is made on the region's moments. The fitted map's own
//! foreshortening is held to the same tolerance afterwards.
//!
//! # What it approximates
//!
//! The label is taken to be flat, which a homography needs. A driver's face has bulge and roll, so a
//! label on one is not quite flat. The fit's residual is where that shows, and P29's real photos are
//! the first to measure it. Every tunable below is a starting value set on synthetic photos (P3
//! finding 10), and the logo's band on P3's flat photo of unused labels on graph paper, the one real
//! photo there is. Each doc says what it was set from, and P29 may move it within its bracket.
//!
//! **On that flat photo**, through `golf-lab read-tape`, all three labels are found against the
//! paper with no head behind them. The fitted map samples 7.95 (driver), 8.35 (hybrid) and 9.0
//! (iron) pixels a millimetre, as P7 read the photo by eye. The printed red cross lies 0.9, 0.3 and
//! 0.2 mm from where the fit puts the label's centre. The paper is nearly as white as the label, so
//! the edge there is faint, and P3 measured the outlines and the cross to ±0.5 mm (P9's finding).

use super::geometry::gauss;
use super::{rectify, FoundLabel, Homography, LabelType, CROP_PX_PER_MM};
use crate::component::components_with_pixels;
use crate::frame::{luminance, Frame, Rect};
use crate::mask::Mask;

// --- The tunables. -------------------------------------------------------------------------------
//
// Set at M46 P9 on synthetic photos only, since no photo of a label on a club exists before P29 (P3
// finding 10). Each names what it was set from and the bracket P29 may move it within, as the
// replay's block in `lib.rs` does.

/// The most saturation a label pixel may have, `(max − min) / max` over its three channels.
///
/// The label is white (P3 finding 4). Its printing is navy and red, which are saturated (0.65 and
/// 0.8), and so are a hitting mat's green and most backgrounds. A white under warm room light, such
/// as (240, 225, 200), is 0.17. Grey steel is about 0.05, and is kept out by luminance instead. P29
/// brackets it at `[0.15, 0.4]`.
pub const LABEL_SATURATION_MAX: f64 = 0.25;

/// The fewest photo pixels a millimetre of label may span at its cross, as the geometric mean over
/// the two axes. Fewer is refused as no label.
///
/// At 4, the crop's 10 a millimetre ([`CROP_PX_PER_MM`]) magnifies the photo 2.5 times, and the iron
/// label spans about 290 pixels across. That is about a tenth of a full-size photo's width, which is
/// the label photographed from much further than the tape camera's frame asks (P22). P3's flat photo
/// sampled 7.4 to 9.2 at half size (P7 finding 4). P29 brackets it at `[2.5, 6]`.
pub const MIN_PX_PER_MM: f64 = 4.0;

/// How far a region's aspect may lie from its label type's, as a ratio either way. It also bounds
/// how far the fitted map may foreshorten at the cross, as its larger stretch over its smaller.
///
/// A label tilted `θ` from face-on is foreshortened by `cos θ` along the tilt, so its aspect moves by
/// up to `1 / cos θ`. That is 1.15 at the 30° the plan's synthetic photos reach (P13), and 1.3
/// allows about 40°. P9's synthetic photos at 30° foreshortened by up to 1.18 at the cross, the
/// perspective adding to the tilt. It does **not** tell the driver label from the iron label, whose
/// aspects are 1.825 and 1.830 (P8 finding 4). The outline fit does that. P29 brackets it at
/// `[1.15, 1.6]`.
pub const ASPECT_TOLERANCE: f64 = 1.3;

/// How near the fitted outline an edge point must lie, millimetres, to count as on it.
///
/// P3 measured the outlines to ±0.5 mm (P3 finding 4), and P4 drew them within 0.33 mm of the edge
/// points they came from (P4 finding 3). On a sharp synthetic photo the fit's edge points lie within
/// a few hundredths of a millimetre. A blurred photo's edge ran 0.7 to 1 mm inside the label's top
/// along the logo (P3 finding 7), which this keeps near the limit. P29 brackets it at `[0.5, 2.0]`.
pub const OUTLINE_INLIER_MM: f64 = 1.0;

/// The least share of a region's edge points that must lie on the fitted outline.
///
/// The rest is a region's edge that is not the label's: a notch where the logo or a mark reaches the
/// edge, or a highlight on the face that joined the label's white. A notch adds its inner side to
/// the region's edge, so this falls with every notch. On P9's synthetic photos a clean label scored
/// 1.0, and one whose swoosh crossed its top edge and two strikes notched its bottom 0.80 to 0.82.
/// That is why it is 0.7 and not 0.8. The fits it refused there were refused by two or more other
/// measures too: it is the least decisive of them, kept so that a region whose edge is mostly not
/// the label's is refused on that count alone. P29 brackets it at `[0.6, 0.95]`.
pub const MIN_INLIER_SHARE: f64 = 0.7;

/// The most of a region's edge points that may lie more than [`OUTLINE_INLIER_MM`] **outside** the
/// fitted outline.
///
/// A notch only ever falls inside the outline: a dark mark, the logo or a shadow on the label's
/// white. So an edge point beyond the outline is white the label does not have, from a highlight on
/// the face that merged with it, or an outline that is not this label's. Every true fit on P9's
/// synthetic photos had none, and so did all three on P3's flat photo. The fits refused there had
/// up to 40%, and those with under 5% failed [`MIN_OUTLINE_SUPPORT`] or the logo. P29 brackets it at
/// `[0.02, 0.15]`.
pub const MAX_OUTSIDE_SHARE: f64 = 0.05;

/// The least share of the label's outline, in 1 mm steps along it, that must have an edge point on
/// it.
///
/// [`MIN_INLIER_SHARE`] asks that the region's edge be the label's. This asks the converse, that the
/// label's edge was seen. Without it, a region holding only part of a label, or a large white region
/// with a label-shaped bay in it, could fit well on the part it has. On P9's synthetic photos a true
/// fit scored 0.955 to 1.0, the low end with its edge notched, and every one on P3's flat photo
/// 1.0. P29 brackets it at `[0.7, 0.97]`.
pub const MIN_OUTLINE_SUPPORT: f64 = 0.9;

// --- The fit's own settings. These are numerical, not judgments of a photo. -----------------------

/// Huber's kernel width for the first stage, millimetres. Every residual pulls, but the large ones
/// only linearly, so a start from the region's moments, which may be several millimetres off,
/// converges without the far points steering it.
const HUBER_MM: f64 = 1.0;

/// Tukey's biweight width for the second stage, millimetres. A point further than this from the
/// outline has no weight at all, so a notch's deeper points stop pulling once the fit is close.
/// Twice [`OUTLINE_INLIER_MM`], so that the stage still sees every point the acceptance counts.
const TUKEY_MM: f64 = 2.0;

/// Tukey's width for the last stage, millimetres. A notch's points within [`TUKEY_MM`] of the edge
/// still pull: on P9's synthetic photos, the logo's target and two strikes notching a driver label
/// left its fit 0.43 mm off. At this width only the edge itself is weighed. It is wider than P3's
/// ±0.5 mm on the outline (P3 finding 4), so a real label's departures from its measured outline
/// stay in.
const TUKEY_FINE_MM: f64 = 0.6;

/// The most edge points a fit iterates on. A longer edge is subsampled evenly along its scan order,
/// and every point is still used to judge the result. A label at the least sampling has about 900,
/// and a full-size photo of one several thousand.
const FIT_POINTS: usize = 500;

/// The most iterations a stage takes. Each stage stops sooner once a step moves the map by less than
/// [`CONVERGED_MM`].
const MAX_ITERATIONS: usize = 60;

/// A step that moves the label's box corners by less than this in the photo has converged, pixels.
const CONVERGED_PX: f64 = 1e-4;

/// How many of a level's largest regions are fitted.
const REGIONS_PER_LEVEL: usize = 4;

/// The step along the outline at which [`MIN_OUTLINE_SUPPORT`] is counted, millimetres.
const SUPPORT_STEP_MM: f64 = 1.0;

/// Two fits whose corners all lie within this many millimetres of each other found the same label.
const SAME_LABEL_MM: f64 = 2.0;

// --- The logo, which says which edge is the top. ---------------------------------------------------
//
// The outline alone cannot always say. Fitted the wrong way up, the hybrid label's flat top and
// gently curved bottom take each other's place to 0.35 mm RMS once the map may foreshorten as well
// (P9's finding), and a blurred real edge is as far off as that. The navy `STRIKE⊕TAPE.com` logo is
// centred along the top edge (P3 finding 4), and that says which edge is the top at once. So a fit
// is accepted only when its band across the logo's letters is darker than the band as deep in from
// the bottom edge, by `LOGO_CONTRAST`. Then the logo is where the fit puts the top. It also refuses
// a white shape with no logo, and an outline of another type laid over a label at a slant.
//
// Each band's darkness is measured against its own white, `1 − mean / p90` over its samples, so a
// light falling across the label from above or below does not count as the logo. On P3's flat photo
// the iron label's bottom band was 13 levels dimmer at its white than its top band.

/// The band across the logo's letters, millimetres in from the box's top edge, nearer and further.
/// On P3's flat photo the letters lie 3.2 to 6.2 mm below the top edge on all three labels, under a
/// thin swoosh that comes within about 1 mm of it (P3 finding 7). The band is the letters' rows with
/// a little over half a millimetre either side. The bottom band is the same depth in from the
/// bottom edge.
const LOGO_BAND_MM: (f64, f64) = (2.5, 6.5);

/// The bands' half-width either side of the box's middle, millimetres. On P3's flat photo the
/// letters span about ±16 mm on the hybrid and iron labels and ±19 mm on the driver's. At ±12 the band
/// stays on the iron label, whose top drops away from its 20 mm flat run at 22° (P3 finding 4).
const LOGO_HALF_WIDTH_MM: f64 = 12.0;

/// How finely the bands are sampled, millimetres.
const LOGO_SAMPLE_MM: f64 = 0.25;

/// How much darker the band across the logo's letters must be than the band as deep in from the
/// bottom edge, each as `1 − mean / p90` of its own samples.
///
/// On P3's flat photo, fitted the right way up, the three labels measured 0.33 (driver), 0.20
/// (hybrid) and 0.18 (iron). The iron label is lowest because its bottom band holds a dashed line
/// of its grid. Turned the wrong way up, each measures about the negative of that. P9's synthetic
/// labels, drawn to match, measured 0.17 to 0.35. A strike's mark across the bottom band darkens
/// that band and narrows the margin: two such strikes took the synthetic iron label from 0.22 to
/// 0.17. A label marked that hard along its bottom edge can fall under this, and then it is
/// refused, never read upside down. P29 brackets it at `[0.04, 0.15]`.
pub const LOGO_CONTRAST: f64 = 0.08;

/// How a found label fitted: what the acceptance judged, kept for a review and for P29's tuning.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OutlineFit {
    /// The root mean square distance of the edge points on the outline from it, millimetres.
    pub rms_mm: f64,
    /// The share of the region's edge points within [`OUTLINE_INLIER_MM`] of the outline.
    pub inlier_share: f64,
    /// The share of them further than that outside it ([`MAX_OUTSIDE_SHARE`]).
    pub outside_share: f64,
    /// The share of the outline with an edge point beside it ([`MIN_OUTLINE_SUPPORT`]).
    pub support: f64,
    /// Photo pixels a millimetre at the cross, the geometric mean over the two axes.
    pub px_per_mm: f64,
    /// The map's larger stretch over its smaller at the cross: 1 face-on, `1 / cos θ` tilted `θ`.
    pub foreshortening: f64,
    /// How much darker the band across the logo's letters is than the band as deep in from the
    /// bottom edge, each as `1 − mean / p90` of its own samples. A fit must reach
    /// [`LOGO_CONTRAST`].
    pub logo_contrast: f64,
    /// How many edge points the region had.
    pub edge_points: usize,
}

/// Why no label was found, by the furthest step any region reached. The tape reader answers each as
/// `no_label`. The distinction is for a test and a review. [M46 P9]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LabelMiss {
    /// No bright, neutral region was the size a label could be and wholly inside the photo.
    NoCandidate,
    /// Regions of the size, and none had the label type's proportions ([`ASPECT_TOLERANCE`]).
    WrongProportions,
    /// A region with the proportions, and the label type's outline did not fit it.
    OutlineMisfit,
    /// More than one place in the photo fits the label, and the finder does not choose.
    Ambiguous,
}

/// The one label of type `label` in `photo`, with its corners and its crop, and how it fitted, or
/// why there is none. [M46 P9]
///
/// The module doc gives the steps. A photo that is not well formed ([`Frame::is_well_formed`]) has no
/// label in it. [`super::read_tape`] refuses it as `decode_failed` before asking.
pub fn find_label(photo: &Frame, label: &LabelType) -> Result<(FoundLabel, OutlineFit), LabelMiss> {
    if !photo.is_well_formed() || photo.width < 3 || photo.height < 3 {
        return Err(LabelMiss::NoCandidate);
    }
    let outline = Outline::of(label);
    let image = Luma::of(photo);
    // A region's own pixels leave out its holes, which the printing makes. Half the outline's area
    // at the least sampling is a floor no label passes under, and it keeps the fills to few regions.
    let min_area = (outline.area * MIN_PX_PER_MM.powi(2) / 2.0) as usize;
    let mut furthest = LabelMiss::NoCandidate;
    let mut fitted: Vec<Fitted> = Vec::new();
    for level in otsu_levels(&image.values) {
        let mask = Mask {
            rect: Rect::whole(photo),
            bits: image
                .values
                .iter()
                .zip(&image.neutral)
                .map(|(&value, &neutral)| neutral && f64::from(value) > level)
                .collect(),
        };
        let mut regions = components_with_pixels(&mask, min_area.max(1));
        regions.sort_by(|a, b| b.0.area.cmp(&a.0.area));
        for (component, members) in regions.into_iter().take(REGIONS_PER_LEVEL) {
            let bbox = component.bbox;
            // A region at the photo's edge is cut by it, so its outline is not all there. It is
            // the background as often as a label.
            if bbox.x0 == 0 || bbox.y0 == 0 || bbox.x1 == photo.width || bbox.y1 == photo.height {
                continue;
            }
            let region = Region::filled(&members, bbox, photo.width);
            let Some(aspect) = region.aspect() else {
                continue;
            };
            let ratio = aspect / outline.aspect;
            if !(1.0 / ASPECT_TOLERANCE..=ASPECT_TOLERANCE).contains(&ratio) {
                furthest = furthest.max(LabelMiss::WrongProportions);
                continue;
            }
            furthest = furthest.max(LabelMiss::OutlineMisfit);
            let edge = region.edge(&image);
            let px_per_mm = (region.area as f64 / outline.area).sqrt();
            for turned in [false, true] {
                let start = region.affine_start(&outline, turned);
                let fit = fit_outline(&outline, label, &image, &edge, &start, px_per_mm);
                if let Some(fit) = fit {
                    if fit.accepted() {
                        fitted.push(fit);
                    }
                }
            }
        }
    }
    let mut distinct: Vec<Fitted> = Vec::new();
    for fit in fitted {
        match distinct.iter_mut().find(|kept| kept.same_label(&fit)) {
            Some(kept) => {
                if fit.better_than(kept) {
                    *kept = fit;
                }
            }
            None => distinct.push(fit),
        }
    }
    match distinct.as_slice() {
        [] => Err(furthest),
        [one] => {
            // `fit_outline` built this map from these corners before accepting them, so it holds.
            let map = Homography::from_corners(one.corners, label.box_corners_mm())
                .map_err(|_| LabelMiss::OutlineMisfit)?;
            let crop = rectify(photo, &map, label, CROP_PX_PER_MM);
            Ok((
                FoundLabel {
                    corners: one.corners,
                    crop,
                },
                one.fit,
            ))
        }
        _ => Err(LabelMiss::Ambiguous),
    }
}

/// A pixel's saturation is at most [`LABEL_SATURATION_MAX`]. Black is neutral, and is kept out by
/// luminance.
fn is_neutral([r, g, b]: [u8; 3]) -> bool {
    let (low, high) = (r.min(g).min(b), r.max(g).max(b));
    high == 0 || f64::from(high - low) <= LABEL_SATURATION_MAX * f64::from(high)
}

/// The photo's luminance, and which pixels are neutral, computed once for every level.
struct Luma {
    width: usize,
    height: usize,
    /// `f32` because a full-size photo has 12 million of them, and a byte's luminance needs no more.
    values: Vec<f32>,
    neutral: Vec<bool>,
}

impl Luma {
    fn of(photo: &Frame) -> Luma {
        let count = photo.width * photo.height;
        let (mut values, mut neutral) = (Vec::with_capacity(count), Vec::with_capacity(count));
        for y in 0..photo.height {
            for x in 0..photo.width {
                let rgb = photo.pixel(x, y);
                values.push(luminance(rgb) as f32);
                neutral.push(is_neutral(rgb));
            }
        }
        Luma {
            width: photo.width,
            height: photo.height,
            values,
            neutral,
        }
    }

    /// The luminance at a pixel, or `None` off the photo.
    fn at(&self, x: isize, y: isize) -> Option<f64> {
        let inside =
            (0..self.width as isize).contains(&x) && (0..self.height as isize).contains(&y);
        inside.then(|| f64::from(self.values[y as usize * self.width + x as usize]))
    }
}

/// The levels a candidate's luminance must be above: Otsu's over the photo, then Otsu's over what is
/// above that, each as the boundary between the last luminance below it and the first above.
fn otsu_levels(values: &[f32]) -> Vec<f64> {
    let mut histogram = [0u64; 256];
    for &value in values {
        histogram[(value.round() as usize).min(255)] += 1;
    }
    let mut levels = Vec::new();
    if let Some(first) = otsu(&histogram, 0) {
        levels.push(first as f64 + 0.5);
        if let Some(second) = otsu(&histogram, first + 1) {
            levels.push(second as f64 + 0.5);
        }
    }
    levels
}

/// Otsu's threshold over the histogram's bins from `from` up: the last bin of the lower class, chosen
/// to maximise the variance between the two classes. `None` when the bins hold fewer than two values.
fn otsu(histogram: &[u64; 256], from: usize) -> Option<usize> {
    let bins = histogram.get(from..)?;
    let total: f64 = bins.iter().map(|&n| n as f64).sum();
    let sum: f64 = bins
        .iter()
        .enumerate()
        .map(|(i, &n)| i as f64 * n as f64)
        .sum();
    let (mut below, mut below_sum) = (0.0, 0.0);
    let mut best: Option<(usize, f64)> = None;
    for (i, &n) in bins.iter().enumerate() {
        below += n as f64;
        below_sum += i as f64 * n as f64;
        let above = total - below;
        if below == 0.0 {
            continue;
        }
        if above == 0.0 {
            break;
        }
        let gap = below_sum / below - (sum - below_sum) / above;
        let between = below * above * gap * gap;
        if best.is_none_or(|(_, most)| between > most) {
            best = Some((i, between));
        }
    }
    best.map(|(i, _)| from + i)
}

/// A region with its holes filled, over its bounding box and a one-pixel margin.
struct Region {
    /// The box and its margin, in the photo's pixels. The region does not touch the photo's edge,
    /// so the margin is inside the photo.
    rect: Rect,
    /// Row-major over `rect`.
    inside: Vec<bool>,
    /// How many pixels it holds, its holes included.
    area: usize,
    centroid: [f64; 2],
    /// The second central moments of its pixel centres: `xx`, `xy`, `yy`.
    moments: [f64; 3],
}

/// The four directions a pixel's edge-sharing neighbours lie in.
const STEPS: [[isize; 2]; 4] = [[1, 0], [-1, 0], [0, 1], [0, -1]];

impl Region {
    /// The region made of `members` (indices row-major into a photo `width` wide, inside `bbox`),
    /// with every pixel it encloses added: whatever a fill from the margin cannot reach without
    /// crossing it.
    fn filled(members: &[usize], bbox: Rect, width: usize) -> Region {
        let rect = Rect {
            x0: bbox.x0 - 1,
            y0: bbox.y0 - 1,
            x1: bbox.x1 + 1,
            y1: bbox.y1 + 1,
        };
        let (w, h) = (rect.width(), rect.height());
        let mut inside = vec![false; w * h];
        for &index in members {
            let (x, y) = (index % width, index / width);
            inside[(y - rect.y0) * w + (x - rect.x0)] = true;
        }
        let mut outside = vec![false; w * h];
        let mut stack: Vec<usize> = (0..w)
            .flat_map(|x| [x, (h - 1) * w + x])
            .chain((0..h).flat_map(|y| [y * w, y * w + w - 1]))
            .collect();
        for &index in &stack {
            outside[index] = true;
        }
        while let Some(index) = stack.pop() {
            let (x, y) = ((index % w) as isize, (index / w) as isize);
            for [dx, dy] in STEPS {
                let (nx, ny) = (x + dx, y + dy);
                if nx < 0 || ny < 0 || nx >= w as isize || ny >= h as isize {
                    continue;
                }
                let next = ny as usize * w + nx as usize;
                if !inside[next] && !outside[next] {
                    outside[next] = true;
                    stack.push(next);
                }
            }
        }
        for (cell, &out) in inside.iter_mut().zip(&outside) {
            *cell = !out;
        }
        let centre = |index: usize| {
            [
                (rect.x0 + index % w) as f64 + 0.5,
                (rect.y0 + index / w) as f64 + 0.5,
            ]
        };
        let cells: Vec<usize> = (0..w * h).filter(|&index| inside[index]).collect();
        let n = cells.len().max(1) as f64;
        let mut centroid = [0.0; 2];
        for &index in &cells {
            let [x, y] = centre(index);
            centroid[0] += x / n;
            centroid[1] += y / n;
        }
        let mut moments = [0.0; 3];
        for &index in &cells {
            let [x, y] = centre(index);
            let (dx, dy) = (x - centroid[0], y - centroid[1]);
            moments[0] += dx * dx / n;
            moments[1] += dx * dy / n;
            moments[2] += dy * dy / n;
        }
        Region {
            rect,
            inside,
            area: cells.len(),
            centroid,
            moments,
        }
    }

    /// The square root of its moments' larger eigenvalue over the smaller: 1 for a disc, a
    /// rectangle's length over its width. `None` for a region with no spread across.
    fn aspect(&self) -> Option<f64> {
        let (major, minor, _) = principal(self.moments);
        (minor > 0.0).then(|| (major / minor).sqrt())
    }

    /// Where its edge lies, a point per pixel side between it and the outside. The point is placed
    /// along that side's normal where the luminance crosses halfway between the label's white and
    /// what is beside it. Those two are the medians of the pixels one inside and two outside each
    /// side, past the edge's own blended pixels.
    fn edge(&self, image: &Luma) -> Vec<[f64; 2]> {
        let w = self.rect.width();
        let mut sides: Vec<([isize; 2], [isize; 2])> = Vec::new();
        for (index, &cell) in self.inside.iter().enumerate() {
            if !cell {
                continue;
            }
            let (x, y) = (index % w, index / w);
            for step in STEPS {
                // The region is inside its margin, so every neighbour is in `rect`.
                let (nx, ny) = (x as isize + step[0], y as isize + step[1]);
                if !self.inside[ny as usize * w + nx as usize] {
                    let pixel = [(self.rect.x0 + x) as isize, (self.rect.y0 + y) as isize];
                    sides.push((pixel, step));
                }
            }
        }
        let along = |(pixel, step): ([isize; 2], [isize; 2]), k: isize| {
            image.at(pixel[0] + k * step[0], pixel[1] + k * step[1])
        };
        let white = crate::median(sides.iter().filter_map(|&side| along(side, -1)).collect());
        let beside = crate::median(sides.iter().filter_map(|&side| along(side, 2)).collect());
        let halfway = match (white, beside) {
            (Some(white), Some(beside)) => (white + beside) / 2.0,
            _ => f64::NAN,
        };
        sides
            .into_iter()
            .map(|side| {
                let (pixel, step) = side;
                let samples = [-1, 0, 1, 2].map(|k| along(side, k));
                let offset = crossing(samples, halfway).unwrap_or(0.5);
                [
                    pixel[0] as f64 + 0.5 + offset * step[0] as f64,
                    pixel[1] as f64 + 0.5 + offset * step[1] as f64,
                ]
            })
            .collect()
    }

    /// The label-to-photo affine map that gives the outline the region's centroid and moments, with
    /// the outline's long axis along the region's. `turned` takes the other way along it, a half
    /// turn. Either way the map keeps a front view's handedness (`Degenerate::Mirrored`).
    fn affine_start(&self, outline: &Outline, turned: bool) -> Affine {
        let (photo_major, photo_minor, photo_angle) = principal(self.moments);
        let (label_major, label_minor, label_angle) = principal(outline.moments);
        let rotation = |angle: f64| [[angle.cos(), -angle.sin()], [angle.sin(), angle.cos()]];
        let scale = [
            (photo_major / label_major).sqrt(),
            (photo_minor / label_minor).sqrt(),
        ];
        // The label's `y` runs up and the photo's down, so a front view reverses orientation: one
        // axis flips. Flipping the other instead is the same map turned half a turn.
        let flip = if turned { [-1.0, 1.0] } else { [1.0, -1.0] };
        let (u, v) = (rotation(photo_angle), rotation(label_angle));
        // `u · diag(scale · flip) · vᵀ`.
        let a: [[f64; 2]; 2] = std::array::from_fn(|i| {
            std::array::from_fn(|j| (0..2).map(|k| u[i][k] * scale[k] * flip[k] * v[j][k]).sum())
        });
        let at_centroid = [
            a[0][0] * outline.centroid[0] + a[0][1] * outline.centroid[1],
            a[1][0] * outline.centroid[0] + a[1][1] * outline.centroid[1],
        ];
        Affine {
            a,
            t: [
                self.centroid[0] - at_centroid[0],
                self.centroid[1] - at_centroid[1],
            ],
        }
    }
}

/// Where along four luminance samples, one a pixel at offsets −1, 0, 1 and 2 along a side's normal,
/// the luminance crosses `level`. Taken between the side's own two pixels when it crosses there,
/// else between the pair either side, by linear interpolation. `None` when it crosses nowhere.
fn crossing(samples: [Option<f64>; 4], level: f64) -> Option<f64> {
    for start in [1usize, 0, 2] {
        let (Some(a), Some(b)) = (samples[start], samples[start + 1]) else {
            continue;
        };
        if (a - level) * (b - level) <= 0.0 && a != b {
            return Some(start as f64 - 1.0 + (a - level) / (a - b));
        }
    }
    None
}

/// A symmetric 2 × 2 matrix's eigenvalues, larger first, and the angle of the larger's eigenvector,
/// from its entries `xx`, `xy`, `yy`.
fn principal([xx, xy, yy]: [f64; 3]) -> (f64, f64, f64) {
    let mean = (xx + yy) / 2.0;
    let radius = ((xx - yy) / 2.0).hypot(xy);
    (
        mean + radius,
        mean - radius,
        0.5 * (2.0 * xy).atan2(xx - yy),
    )
}

/// A map from label millimetres to photo pixels, `p = a · l + t`.
struct Affine {
    a: [[f64; 2]; 2],
    t: [f64; 2],
}

/// A label type's outline, ready to measure a point's distance from.
struct Outline {
    segments: Vec<Segment>,
    perimeter: f64,
    area: f64,
    centroid: [f64; 2],
    /// The second central moments of its area: `xx`, `xy`, `yy`.
    moments: [f64; 3],
    /// [`Region::aspect`]'s measure of it.
    aspect: f64,
    /// `√2` over its vertices' mean distance from the cross: the fit's normalisation of it.
    scale: f64,
}

/// One side of an outline.
struct Segment {
    start: [f64; 2],
    /// A unit vector along it.
    direction: [f64; 2],
    length: f64,
    /// The unit normal pointing out of the label.
    normal: [f64; 2],
    /// The outward direction at its start vertex: its normal and the previous side's, averaged.
    corner_normal: [f64; 2],
    /// How far along the outline its start lies from the first vertex.
    arc: f64,
}

/// The nearest point of an outline to a point: the signed distance, positive outside, and how far
/// along the outline the nearest point lies.
struct Nearest {
    residual: f64,
    arc: f64,
}

impl Outline {
    fn of(label: &LabelType) -> Outline {
        let vertices = label.outline_mm;
        let n = vertices.len();
        // The outline runs counter-clockwise (`tests/tape_labels.rs` holds it), so outward is to
        // the right of each side.
        let mut segments: Vec<Segment> = Vec::with_capacity(n);
        let mut arc = 0.0;
        for (i, &start) in vertices.iter().enumerate() {
            let end = vertices[(i + 1) % n];
            let (dx, dy) = (end[0] - start[0], end[1] - start[1]);
            let length = dx.hypot(dy);
            let direction = [dx / length, dy / length];
            segments.push(Segment {
                start,
                direction,
                length,
                normal: [direction[1], -direction[0]],
                corner_normal: [0.0; 2],
                arc,
            });
            arc += length;
        }
        for i in 0..n {
            let (previous, this) = (segments[(i + n - 1) % n].normal, segments[i].normal);
            let (x, y) = (previous[0] + this[0], previous[1] + this[1]);
            let length = x.hypot(y);
            segments[i].corner_normal = if length > 0.0 {
                [x / length, y / length]
            } else {
                this
            };
        }
        // The polygon's area, centroid and second moments, by Green's theorem.
        let (mut area2, mut cx, mut cy, mut ixx, mut ixy, mut iyy) = (0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
        for (i, &[x0, y0]) in vertices.iter().enumerate() {
            let [x1, y1] = vertices[(i + 1) % n];
            let cross = x0 * y1 - x1 * y0;
            area2 += cross;
            cx += (x0 + x1) * cross;
            cy += (y0 + y1) * cross;
            ixx += (x0 * x0 + x0 * x1 + x1 * x1) * cross;
            iyy += (y0 * y0 + y0 * y1 + y1 * y1) * cross;
            ixy += (x0 * y1 + 2.0 * x0 * y0 + 2.0 * x1 * y1 + x1 * y0) * cross;
        }
        let area = area2 / 2.0;
        let centroid = [cx / (6.0 * area), cy / (6.0 * area)];
        let moments = [
            ixx / (12.0 * area) - centroid[0] * centroid[0],
            ixy / (24.0 * area) - centroid[0] * centroid[1],
            iyy / (12.0 * area) - centroid[1] * centroid[1],
        ];
        let (major, minor, _) = principal(moments);
        Outline {
            segments,
            perimeter: arc,
            area,
            centroid,
            moments,
            aspect: (major / minor).sqrt(),
            scale: std::f64::consts::SQRT_2 * n as f64
                / vertices.iter().map(|v| v[0].hypot(v[1])).sum::<f64>(),
        }
    }

    fn nearest(&self, point: [f64; 2]) -> Nearest {
        let mut best = (f64::INFINITY, 0, 0.0);
        for (index, side) in self.segments.iter().enumerate() {
            let relative = [point[0] - side.start[0], point[1] - side.start[1]];
            let t = (relative[0] * side.direction[0] + relative[1] * side.direction[1])
                .clamp(0.0, side.length);
            let off = [
                relative[0] - t * side.direction[0],
                relative[1] - t * side.direction[1],
            ];
            let squared = off[0] * off[0] + off[1] * off[1];
            if squared < best.0 {
                best = (squared, index, t);
            }
        }
        let (_, index, t) = best;
        let side = &self.segments[index];
        let foot = [
            side.start[0] + t * side.direction[0],
            side.start[1] + t * side.direction[1],
        ];
        let off = [point[0] - foot[0], point[1] - foot[1]];
        let normal = if t > 0.0 && t < side.length {
            side.normal
        } else {
            // Nearest a vertex: the distance runs along the line to it, outward or inward as the
            // vertex's own outward direction says.
            let corner = if t <= 0.0 {
                side.corner_normal
            } else {
                self.segments[(index + 1) % self.segments.len()].corner_normal
            };
            let distance = off[0].hypot(off[1]);
            if distance > 1e-12 {
                let sign = if corner[0] * off[0] + corner[1] * off[1] >= 0.0 {
                    1.0
                } else {
                    -1.0
                };
                [sign * off[0] / distance, sign * off[1] / distance]
            } else {
                corner
            }
        };
        Nearest {
            residual: normal[0] * off[0] + normal[1] * off[1],
            arc: side.arc + t,
        }
    }
}

/// A label-to-photo homography being fitted, between normalised frames. A label point `l`, in
/// millimetres, is first `l' = label_scale · l`, and a photo point `q`, in pixels, is `u =
/// photo_scale · (q − centre)`. Then `u = ((h₀l'₀ + h₁l'₁ + h₂) / w, (h₃l'₀ + h₄l'₁ + h₅) / w)`, with
/// `w = h₆l'₀ + h₇l'₁ + 1`. Both scales put their points at about `√2` from their middle, as
/// `Homography::from_corners` normalises its own, so the eight unknowns are of order 1.
///
/// **Label to photo, and measured in the photo.** The first version mapped the photo's edge points
/// into the label and measured them there, and on a label with notches it walked off to a map that
/// squeezed every edge point onto one part of the outline: a residual measured where the map can
/// shrink the points is one the map can make small. Measured in the photo, to the outline projected
/// there, the edge points stay where they are and only a map that lays the outline over them scores
/// well (P9's finding).
#[derive(Debug, Clone, Copy)]
struct Model {
    h: [f64; 8],
    centre: [f64; 2],
    photo_scale: f64,
    label_scale: f64,
}

impl Model {
    /// The affine `start` in this model's form, which is exact for it: an affine map is a
    /// homography with `h₆ = h₇ = 0`.
    fn from_affine(start: &Affine, centre: [f64; 2], photo_scale: f64, label_scale: f64) -> Model {
        let (s, k) = (photo_scale, photo_scale / label_scale);
        let [[a, b], [c, d]] = start.a;
        Model {
            h: [
                k * a,
                k * b,
                s * (start.t[0] - centre[0]),
                k * c,
                k * d,
                s * (start.t[1] - centre[1]),
                0.0,
                0.0,
            ],
            centre,
            photo_scale,
            label_scale,
        }
    }

    /// Where a label point lands, normalised, with its `w`, or `None` on or beyond the horizon.
    fn project(&self, l: [f64; 2]) -> Option<([f64; 2], f64)> {
        let (x, y) = (self.label_scale * l[0], self.label_scale * l[1]);
        let h = &self.h;
        let w = h[6] * x + h[7] * y + 1.0;
        // NaN as well as the horizon and beyond it.
        if w.is_nan() || w <= 1e-9 {
            return None;
        }
        let u = [
            (h[0] * x + h[1] * y + h[2]) / w,
            (h[3] * x + h[4] * y + h[5]) / w,
        ];
        u.iter().all(|c| c.is_finite()).then_some((u, w))
    }

    /// The projection's derivatives by the eight unknowns, at a label point that lands at `u` with
    /// `w`: one row a coordinate.
    fn jacobian(&self, l: [f64; 2], u: [f64; 2], w: f64) -> [[f64; 8]; 2] {
        let (x, y) = (self.label_scale * l[0] / w, self.label_scale * l[1] / w);
        [
            [x, y, 1.0 / w, 0.0, 0.0, 0.0, -u[0] * x, -u[0] * y],
            [0.0, 0.0, 0.0, x, y, 1.0 / w, -u[1] * x, -u[1] * y],
        ]
    }

    fn normalised(&self, q: [f64; 2]) -> [f64; 2] {
        [
            self.photo_scale * (q[0] - self.centre[0]),
            self.photo_scale * (q[1] - self.centre[1]),
        ]
    }

    /// Where a label point lands in the photo's pixels.
    fn photo_of(&self, l: [f64; 2]) -> Option<[f64; 2]> {
        let (u, _) = self.project(l)?;
        Some([
            self.centre[0] + u[0] / self.photo_scale,
            self.centre[1] + u[1] / self.photo_scale,
        ])
    }

    /// The label point a photo point sees, or `None` when none does in front of the horizon.
    /// `u · w = (h₀l'₀ + h₁l'₁ + h₂, h₃l'₀ + h₄l'₁ + h₅)` is linear in `l'`, so it is two equations
    /// solved directly.
    fn label_of(&self, q: [f64; 2]) -> Option<[f64; 2]> {
        let [u0, u1] = self.normalised(q);
        let h = &self.h;
        let (a, b, e) = (h[0] - u0 * h[6], h[1] - u0 * h[7], u0 - h[2]);
        let (c, d, f) = (h[3] - u1 * h[6], h[4] - u1 * h[7], u1 - h[5]);
        let det = a * d - b * c;
        if !(det.is_finite() && det.abs() > 1e-300) {
            return None;
        }
        let l = [
            (e * d - b * f) / det / self.label_scale,
            (a * f - e * c) / det / self.label_scale,
        ];
        self.project(l)?;
        Some(l)
    }

    /// The map's two stretches at the cross, larger first, in photo pixels a millimetre: the
    /// singular values of its Jacobian there.
    fn stretches_at_cross(&self) -> Option<(f64, f64)> {
        let (u, w) = self.project([0.0, 0.0])?;
        let h = &self.h;
        let k = self.label_scale / self.photo_scale / w;
        let j = [
            [(h[0] - u[0] * h[6]) * k, (h[1] - u[0] * h[7]) * k],
            [(h[3] - u[1] * h[6]) * k, (h[4] - u[1] * h[7]) * k],
        ];
        let det = (j[0][0] * j[1][1] - j[0][1] * j[1][0]).abs();
        let sum = j.iter().flatten().map(|v| v * v).sum::<f64>();
        // σ₁² + σ₂² is the sum of squares, and σ₁σ₂ is |det|.
        let spread = (sum * sum / 4.0 - det * det).max(0.0).sqrt();
        let (larger, smaller) = (
            (sum / 2.0 + spread).sqrt(),
            (sum / 2.0 - spread).max(0.0).sqrt(),
        );
        (smaller > 0.0).then_some((larger, smaller))
    }

    fn stepped(&self, step: [f64; 8]) -> Model {
        Model {
            h: std::array::from_fn(|i| self.h[i] + step[i]),
            ..*self
        }
    }

    /// How far `other` moves the label's box corners from where this puts them, in photo pixels.
    fn movement(&self, other: &Model, corners: &[[f64; 2]; 4]) -> f64 {
        corners
            .iter()
            .map(|&c| match (self.photo_of(c), other.photo_of(c)) {
                (Some(a), Some(b)) => (a[0] - b[0]).hypot(a[1] - b[1]),
                _ => f64::INFINITY,
            })
            .fold(0.0, f64::max)
    }
}

/// The outline projected into the normalised photo by a model, with each vertex's derivatives.
/// A homography maps a straight side to a straight side, so the projected outline is the polygon
/// of the projected vertices.
struct Projected {
    vertices: Vec<[f64; 2]>,
    jacobians: Vec<[[f64; 8]; 2]>,
    /// `+1` when the projected outline runs counter-clockwise in the photo's own coordinates, and
    /// `−1` when it runs clockwise, as a front view's does. Outward is to the right of each side,
    /// times this.
    winding: f64,
}

impl Projected {
    /// `None` when a vertex is on or beyond the horizon.
    fn of(model: &Model, outline: &Outline) -> Option<Projected> {
        let mut vertices = Vec::with_capacity(outline.segments.len());
        let mut jacobians = Vec::with_capacity(outline.segments.len());
        for side in &outline.segments {
            let (u, w) = model.project(side.start)?;
            vertices.push(u);
            jacobians.push(model.jacobian(side.start, u, w));
        }
        let n = vertices.len();
        let twice_area: f64 = (0..n)
            .map(|i| {
                let ([x0, y0], [x1, y1]) = (vertices[i], vertices[(i + 1) % n]);
                x0 * y1 - x1 * y0
            })
            .sum();
        Some(Projected {
            vertices,
            jacobians,
            winding: if twice_area >= 0.0 { 1.0 } else { -1.0 },
        })
    }

    /// The outward unit normal of side `i`.
    fn normal(&self, i: usize) -> [f64; 2] {
        let n = self.vertices.len();
        let (a, b) = (self.vertices[i], self.vertices[(i + 1) % n]);
        let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
        let length = dx.hypot(dy).max(1e-300);
        [self.winding * dy / length, -self.winding * dx / length]
    }

    /// The signed distance of `u` from the projected outline, positive outside, and its
    /// derivatives by the eight unknowns. The nearest point's place along its side and the normal
    /// are held fixed, which makes it a point-to-tangent residual.
    fn residual(&self, u: [f64; 2]) -> (f64, [f64; 8]) {
        let n = self.vertices.len();
        let mut best = (f64::INFINITY, 0, 0.0);
        for i in 0..n {
            let (a, b) = (self.vertices[i], self.vertices[(i + 1) % n]);
            let d = [b[0] - a[0], b[1] - a[1]];
            let squared = d[0] * d[0] + d[1] * d[1];
            let t = if squared > 0.0 {
                (((u[0] - a[0]) * d[0] + (u[1] - a[1]) * d[1]) / squared).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let off = [u[0] - a[0] - t * d[0], u[1] - a[1] - t * d[1]];
            let distance = off[0] * off[0] + off[1] * off[1];
            if distance < best.0 {
                best = (distance, i, t);
            }
        }
        let (_, i, t) = best;
        let (a, b) = (self.vertices[i], self.vertices[(i + 1) % n]);
        let foot = [a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])];
        let off = [u[0] - foot[0], u[1] - foot[1]];
        let normal = if t > 0.0 && t < 1.0 {
            self.normal(i)
        } else {
            // Nearest a vertex: along the line to it, outward or inward as the vertex's own
            // outward direction, its two sides' normals summed, says.
            let vertex = if t <= 0.0 { i } else { (i + 1) % n };
            let (before, after) = (self.normal((vertex + n - 1) % n), self.normal(vertex));
            let corner = [before[0] + after[0], before[1] + after[1]];
            let distance = off[0].hypot(off[1]);
            if distance > 1e-12 {
                let sign = if corner[0] * off[0] + corner[1] * off[1] >= 0.0 {
                    1.0
                } else {
                    -1.0
                };
                [sign * off[0] / distance, sign * off[1] / distance]
            } else {
                let length = corner[0].hypot(corner[1]).max(1e-300);
                [corner[0] / length, corner[1] / length]
            }
        };
        let (ja, jb) = (&self.jacobians[i], &self.jacobians[(i + 1) % n]);
        let derivative = std::array::from_fn(|k| {
            let moved = |j: &[[f64; 8]; 2]| normal[0] * j[0][k] + normal[1] * j[1][k];
            -((1.0 - t) * moved(ja) + t * moved(jb))
        });
        (normal[0] * off[0] + normal[1] * off[1], derivative)
    }
}

/// A robust loss on a residual, and the weight its iteratively reweighted least squares gives the
/// residual. The widths are in the residual's own units.
#[derive(Clone, Copy)]
enum Kernel {
    Huber(f64),
    Tukey(f64),
}

impl Kernel {
    fn loss(self, r: f64) -> f64 {
        match self {
            Kernel::Huber(k) if r.abs() <= k => r * r / 2.0,
            Kernel::Huber(k) => k * (r.abs() - k / 2.0),
            Kernel::Tukey(c) if r.abs() < c => {
                c * c / 6.0 * (1.0 - (1.0 - (r / c).powi(2)).powi(3))
            }
            Kernel::Tukey(c) => c * c / 6.0,
        }
    }

    fn weight(self, r: f64) -> f64 {
        match self {
            Kernel::Huber(k) if r.abs() <= k => 1.0,
            Kernel::Huber(k) => k / r.abs(),
            Kernel::Tukey(c) if r.abs() < c => (1.0 - (r / c).powi(2)).powi(2),
            Kernel::Tukey(_) => 0.0,
        }
    }
}

/// The total loss of the edge points (normalised) against the outline as `model` projects it, or
/// `None` when the outline crosses the horizon.
fn total_loss(
    outline: &Outline,
    points: &[[f64; 2]],
    model: &Model,
    kernel: Kernel,
) -> Option<f64> {
    let projected = Projected::of(model, outline)?;
    Some(
        points
            .iter()
            .map(|&u| kernel.loss(projected.residual(u).0))
            .sum(),
    )
}

/// One stage of the fit: Levenberg–Marquardt on the robust loss, reweighting each iteration, with
/// Marquardt's damping of the normal equations' diagonal. A step is taken only when it lowers the
/// loss.
fn refine(
    outline: &Outline,
    points: &[[f64; 2]],
    mut model: Model,
    kernel: Kernel,
    corners: &[[f64; 2]; 4],
) -> Option<Model> {
    let mut loss = total_loss(outline, points, &model, kernel)?;
    let mut damping = 1e-3;
    for _ in 0..MAX_ITERATIONS {
        let projected = Projected::of(&model, outline)?;
        let mut matrix = [[0.0; 8]; 8];
        let mut gradient = [0.0; 8];
        for &u in points {
            let (r, j) = projected.residual(u);
            let weight = kernel.weight(r);
            if weight == 0.0 {
                continue;
            }
            for (i, &ji) in j.iter().enumerate() {
                gradient[i] += weight * ji * r;
                for (k, &jk) in j.iter().enumerate().skip(i) {
                    matrix[i][k] += weight * ji * jk;
                }
            }
        }
        for i in 0..8 {
            for k in 0..i {
                matrix[i][k] = matrix[k][i];
            }
        }
        let mut accepted = None;
        for _ in 0..12 {
            let mut system = [[0.0; 9]; 8];
            for (i, row) in system.iter_mut().enumerate() {
                row[..8].copy_from_slice(&matrix[i]);
                row[i] += damping * matrix[i][i].max(1e-12);
                row[8] = -gradient[i];
            }
            if let Some(step) = gauss(system) {
                let next = model.stepped(step);
                if let Some(next_loss) = total_loss(outline, points, &next, kernel) {
                    if next_loss < loss {
                        accepted = Some((next, next_loss));
                        break;
                    }
                }
            }
            damping *= 10.0;
        }
        let Some((next, next_loss)) = accepted else {
            break;
        };
        let moved = model.movement(&next, corners);
        model = next;
        loss = next_loss;
        damping = (damping / 10.0).max(1e-12);
        if moved < CONVERGED_PX {
            break;
        }
    }
    Some(model)
}

/// A fit that reached its measures: the photo's corners of the label's box, and the measures.
struct Fitted {
    corners: [[f64; 2]; 4],
    fit: OutlineFit,
}

impl Fitted {
    fn accepted(&self) -> bool {
        let fit = &self.fit;
        fit.inlier_share >= MIN_INLIER_SHARE
            && fit.outside_share <= MAX_OUTSIDE_SHARE
            && fit.support >= MIN_OUTLINE_SUPPORT
            && fit.px_per_mm >= MIN_PX_PER_MM
            && fit.foreshortening <= ASPECT_TOLERANCE
            && fit.logo_contrast >= LOGO_CONTRAST
    }

    fn same_label(&self, other: &Fitted) -> bool {
        let tolerance = SAME_LABEL_MM * self.fit.px_per_mm.min(other.fit.px_per_mm);
        self.corners
            .iter()
            .zip(&other.corners)
            .all(|(a, b)| (a[0] - b[0]).hypot(a[1] - b[1]) <= tolerance)
    }

    fn better_than(&self, other: &Fitted) -> bool {
        (self.fit.inlier_share, -self.fit.rms_mm) > (other.fit.inlier_share, -other.fit.rms_mm)
    }
}

/// The outline fitted to `edge` from `start`, and measured, or `None` when the fit left the plane
/// or its corners make no homography a front view could give. `px_per_mm` is the region's own
/// estimate, from its area, which sets the kernels' widths in the photo.
fn fit_outline(
    outline: &Outline,
    label: &LabelType,
    image: &Luma,
    edge: &[[f64; 2]],
    start: &Affine,
    px_per_mm: f64,
) -> Option<Fitted> {
    if edge.len() < 8 {
        return None;
    }
    let n = edge.len() as f64;
    let centre = [0, 1].map(|axis| edge.iter().map(|q| q[axis]).sum::<f64>() / n);
    let spread = edge
        .iter()
        .map(|q| (q[0] - centre[0]).hypot(q[1] - centre[1]))
        .sum::<f64>()
        / n;
    let photo_scale = std::f64::consts::SQRT_2 / spread;
    if !(photo_scale.is_finite() && photo_scale > 0.0) {
        return None;
    }
    let box_mm = label.box_corners_mm();
    let mut model = Model::from_affine(start, centre, photo_scale, outline.scale);
    let points: Vec<[f64; 2]> = edge.iter().map(|&q| model.normalised(q)).collect();
    let stride = points.len().div_ceil(FIT_POINTS);
    let sample: Vec<[f64; 2]> = points.iter().step_by(stride).copied().collect();
    let to_units = px_per_mm * photo_scale;
    for kernel in [
        Kernel::Huber(HUBER_MM * to_units),
        Kernel::Tukey(TUKEY_MM * to_units),
        Kernel::Tukey(TUKEY_FINE_MM * to_units),
    ] {
        model = refine(outline, &sample, model, kernel, &box_mm)?;
    }
    // Judged in the label's millimetres, on every edge point and not the sample.
    let steps = (outline.perimeter / SUPPORT_STEP_MM).ceil().max(1.0) as usize;
    let mut seen = vec![false; steps];
    let (mut inliers, mut squares) = (0usize, 0.0);
    let mut outside = 0usize;
    for &q in edge {
        let Some(l) = model.label_of(q) else {
            continue;
        };
        let nearest = outline.nearest(l);
        if nearest.residual > OUTLINE_INLIER_MM {
            outside += 1;
        }
        if nearest.residual.abs() <= OUTLINE_INLIER_MM {
            inliers += 1;
            squares += nearest.residual * nearest.residual;
            seen[((nearest.arc / SUPPORT_STEP_MM) as usize).min(steps - 1)] = true;
        }
    }
    let corners = [
        model.photo_of(box_mm[0])?,
        model.photo_of(box_mm[1])?,
        model.photo_of(box_mm[2])?,
        model.photo_of(box_mm[3])?,
    ];
    Homography::from_corners(corners, box_mm).ok()?;
    let (larger, smaller) = model.stretches_at_cross()?;
    Some(Fitted {
        corners,
        fit: OutlineFit {
            rms_mm: if inliers > 0 {
                (squares / inliers as f64).sqrt()
            } else {
                f64::INFINITY
            },
            inlier_share: inliers as f64 / n,
            outside_share: outside as f64 / n,
            support: seen.iter().filter(|&&s| s).count() as f64 / steps as f64,
            px_per_mm: (larger * smaller).sqrt(),
            foreshortening: larger / smaller,
            logo_contrast: logo_contrast(&model, label, image)?,
            edge_points: edge.len(),
        },
    })
}

/// How much darker the band across the logo's letters is than the band as deep in from the bottom
/// edge, as the fitted map places them: [`LOGO_BAND_MM`] in from each edge and [`LOGO_HALF_WIDTH_MM`]
/// either side of the box's middle. Each band's darkness is `1 − mean / p90` of its samples, the
/// nearest photo pixel's luminance at each. `None` when a band falls off the photo.
fn logo_contrast(model: &Model, label: &LabelType, image: &Luma) -> Option<f64> {
    let [[left, top], _, [right, bottom], _] = label.box_corners_mm();
    let middle = (left + right) / 2.0;
    let (near, far) = LOGO_BAND_MM;
    let steps = |span: f64| (span / LOGO_SAMPLE_MM).round().max(1.0) as usize;
    let (across, down) = (steps(2.0 * LOGO_HALF_WIDTH_MM), steps(far - near));
    let darkness = |edge: f64, inward: f64| -> Option<f64> {
        let mut values = Vec::with_capacity(across * down);
        for i in 0..across {
            for j in 0..down {
                let x =
                    middle + LOGO_HALF_WIDTH_MM * (2.0 * (i as f64 + 0.5) / across as f64 - 1.0);
                let depth = near + (far - near) * (j as f64 + 0.5) / down as f64;
                let [px, py] = model.photo_of([x, edge + inward * depth])?;
                values.push(image.at(px.floor() as isize, py.floor() as isize)?);
            }
        }
        let mean = values.iter().sum::<f64>() / values.len() as f64;
        values.sort_by(f64::total_cmp);
        let white = values[values.len() * 9 / 10];
        (white > 0.0).then(|| 1.0 - mean / white)
    };
    Some(darkness(top, -1.0)? - darkness(bottom, 1.0)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tape::{STRIKEPRO_DRIVER, STRIKEPRO_HYBRID, STRIKEPRO_IRON};

    #[test]
    fn otsu_parts_two_clusters_between_them_and_a_third_above() {
        let mut values = vec![40.0f32; 500];
        values.extend(vec![150.0; 300]);
        values.extend(vec![235.0; 200]);
        let levels = otsu_levels(&values);
        assert_eq!(levels.len(), 2);
        // The first parts the darkest from the rest, or the brightest; the second parts what is
        // above the first, so between them the brightest stands alone at one level.
        assert!(levels.iter().any(|&l| l > 150.0 && l < 235.0), "{levels:?}");
        assert_eq!(otsu_levels(&[90.0; 10]), Vec::<f64>::new());
    }

    #[test]
    fn a_crossing_is_interpolated_between_the_sides_own_pixels_first() {
        let level = 100.0;
        // Between offsets 0 and 1, a quarter of the way.
        let at = crossing([Some(200.0), Some(150.0), Some(-50.0), Some(0.0)], level).unwrap();
        assert!((at - 0.25).abs() < 1e-12, "{at}");
        // Not there, so the pair inside it.
        let at = crossing([Some(200.0), Some(0.0), Some(-10.0), Some(-20.0)], level).unwrap();
        assert!((at - -0.5).abs() < 1e-12, "{at}");
        assert_eq!(
            crossing([Some(10.0), Some(20.0), Some(30.0), None], level),
            None
        );
    }

    #[test]
    fn neutral_means_unsaturated() {
        assert!(is_neutral([236, 236, 230]));
        assert!(is_neutral([240, 225, 200]));
        assert!(is_neutral([0, 0, 0]));
        assert!(!is_neutral([200, 40, 45])); // the red cross
        assert!(!is_neutral([35, 45, 95])); // the navy logo
        assert!(!is_neutral([95, 120, 80])); // a mat
    }

    #[test]
    fn each_outline_measures_its_own_area_and_lies_long_across() {
        for label in [&STRIKEPRO_DRIVER, &STRIKEPRO_HYBRID, &STRIKEPRO_IRON] {
            let outline = Outline::of(label);
            let box_area = label.width_mm * label.height_mm;
            assert!(
                outline.area > 0.7 * box_area && outline.area < box_area,
                "{}: {}",
                label.id,
                outline.area
            );
            let (_, _, angle) = principal(outline.moments);
            assert!(angle.abs() < 0.05, "{}: {angle}", label.id);
            assert!(outline.aspect > 1.5 && outline.aspect < 2.5, "{}", label.id);
            // A point on a side is at no distance, and one beyond it is outside by the margin.
            let side = &outline.segments[3];
            let middle = [
                side.start[0] + side.direction[0] * side.length / 2.0,
                side.start[1] + side.direction[1] * side.length / 2.0,
            ];
            assert!(outline.nearest(middle).residual.abs() < 1e-9);
            let beyond = [middle[0] + side.normal[0], middle[1] + side.normal[1]];
            assert!((outline.nearest(beyond).residual - 1.0).abs() < 1e-9);
            assert!(outline.nearest([0.0, 0.0]).residual < -label.inscribed_radius_mm + 0.5);
        }
    }

    #[test]
    fn a_region_is_filled_through_its_holes_and_not_its_bays() {
        // A 7 × 5 ring of pixels with a one-pixel hole in it, and a bay open to the outside.
        #[rustfmt::skip]
        let rows = [
            "#######",
            "#.#####",
            "#######",
            "###....",
            "#######",
        ];
        let width = 9;
        let members: Vec<usize> = rows
            .iter()
            .enumerate()
            .flat_map(|(y, row)| {
                row.chars()
                    .enumerate()
                    .filter(|&(_, c)| c == '#')
                    .map(move |(x, _)| (y + 1) * width + x + 1)
            })
            .collect();
        let bbox = Rect {
            x0: 1,
            y0: 1,
            x1: 8,
            y1: 6,
        };
        let region = Region::filled(&members, bbox, width);
        let filled = region.inside.iter().filter(|&&c| c).count();
        assert_eq!(
            filled,
            members.len() + 1,
            "the hole, and not the bay's four"
        );
    }
}
