//! What is new on a used label since the last photo: this crop diffed against the club's last.
//! [M46 P12]
//!
//! D4's rule is one photo a strike, so on a used label the strike is what has changed since the
//! last photo. The previous crop is the club's last (C16's `last/<club>.rgb`, ADR-037 clause 3), cut
//! at the same [`CROP_PX_PER_MM`] from the same label type, so both crops are the label's box in its
//! printed frame on one [`CropGrid`] (P7 finding 6), and the diff is pixel to pixel.
//!
//! # How the newest mark is found
//!
//! 1. **Each crop's darkening and marks**, by `mark.rs` alone: its interior, its own white, each
//!    pixel's darkening against that white, and its spots. Each crop is measured against its own
//!    white so that two photos lit or exposed differently compare.
//! 2. **The alignment.** Each crop was cut through its own fit of the label, so one printed point
//!    lies a little apart in the two. Every whole-pixel translation up to [`DIFF_SEARCH_MM`] either
//!    way is tried, coarse to fine over a pyramid, for the least mean difference in darkening over
//!    the pixels where neither crop is dark and wide. So the crops are aligned on the label's
//!    printing, which every label of a type shares, and never on the marks, which differ by the very
//!    strike being looked for. Two answers are refused as [`DiffMiss::Unaligned`] rather than taken:
//!    a best shift on the search's edge, which may be the edge of a slope that runs on past it, and
//!    one the printing did not pick out ([`ALIGN_MAX_SHARE`]).
//! 3. **The change**: each pixel's darkening less the previous crop's at the aligned pixel. A pixel
//!    is newly dark at [`MARK_MIN_CONTRAST`] or more of change, the same step a mark is dark by.
//! 4. **What each spot holds.** Each of this crop's spots ([`find_marks`]'s) holds something new when
//!    at least [`PART_MIN_AREA_MM2`] of it is newly dark, and an old mark when as much of it lies on
//!    one of the previous crop's spots. Holding both is an overlap. Holding neither is a spot dark
//!    now that was neither a mark before nor darkened since, and cannot be told.
//! 5. **Which label.** A previous mark is still here when half of it lies on what is dark and wide
//!    here. When none is, the label is new (the plan's P12): the golfer has stuck on a fresh one,
//!    and the fresh label's rule (`mark.rs`) reads it. When some are and some are not, it cannot be
//!    told, since a mark does not leave a label.
//!
//! # Why the reading is the spot's, not the change's
//!
//! The plan's P12 names the change's components as the new marks. Here the change only decides
//! which of this crop's spots are new, and a new mark is read where its spot is, as on a fresh label.
//! Two reasons. The alignment is to a whole pixel and was searched on the printing, so a reading
//! taken from the change would carry its error, while a spot's centroid is measured in this crop
//! alone. And a mark is lighter than the printing it covers (P3 finding 4, P11), so the change has a
//! hole along every stroke under a new mark, which would pull a centroid taken from it. The spot's
//! is P11's, held within 0.2 mm over the printing.
//!
//! # The outcomes ([`Change::newest`])
//!
//! - **A new label** (every previous mark gone, or none before): the fresh label's rule, with
//!   `new_label`. One whole spot is read; two or more need a pick (P2 finding 3: the skipped photo
//!   is never a label's first).
//! - **Some previous marks gone and some not**, or **an overlap**, or **a spot that cannot be
//!   told**: the golfer's pick.
//! - **The same label**: no new spot is `no_new_mark`. One or two new spots are read, each in its own
//!   right (C10: two are a photo missed). A new spot cut by the label's edge makes the photo
//!   `mark_clipped`, even beside a whole one, since a reading of the whole one alone would leave a
//!   strike uncounted and unrecorded. A new spot too long to be one mark, or more than two, needs
//!   a pick.
//!
//! # What it cannot tell
//!
//! - **A strike landing nearly on an old mark.** Only the crescent it adds is newly dark, and under
//!   [`PART_MIN_AREA_MM2`] the spot holds an old mark alone. For two marks 5 mm in radius that is a
//!   strike within about 1.2 mm of the old one's centre, which comes out `no_new_mark`. A pending
//!   `no_new_mark` can still be picked (C15).
//! - **A strike over an old mark that darkens it.** The renderer lays a mark over a mark as the same
//!   colour, so the synthetic tests never see this. A real strike may darken an old mark, and the
//!   change would then show it as new there, holding both: an overlap, and a pick.
//! - **A false shift.** Printing that repeats inside the reach can line up at a wrong shift when
//!   the true one lies beyond it, and [`ALIGN_MAX_SHARE`] does not see a partial match. An old mark
//!   is still more than half on itself when the shift is wrong by up to about 2.6 mm, so it holds
//!   an old mark or both and is never new. Wrong by more, every old mark is gone, and the label
//!   would be called new: so a new label is called only when no spot stands where an old mark stood
//!   (`PLACE_TAKEN_MM`), and otherwise the golfer picks.
//! - **A turn or a scale between the two fits.** Only a translation is searched, as the plan says.
//!   The finder lands within 0.25 mm on synthetic photos (P9), where a turn would not show, and P29's
//!   photos are the first that can say whether the real fits need more.
//!
//! # The previous crop's outside
//!
//! The previous crop arrives as a bare [`Frame`] (C5, and C16's raw RGB), without [`Crop::outside`].
//! [`rectify`](super::rectify) writes an outside pixel as pure black, so a pure black pixel is taken
//! for outside and left out of the diff (P7 finding 6, P8 finding 5). A photo's own pure black on a
//! white label would be left out with it, which only loses that pixel.

use super::mark::{
    darkening, on_a_fresh_label, spot_pixels, wide_dark, Fresh, Interior, White,
    MARK_MAX_ELONGATION, MARK_MIN_CONTRAST,
};
#[cfg(doc)]
use super::{find_marks, CROP_PX_PER_MM};
use super::{Crop, CropGrid, LabelType, Spot};
use crate::frame::Frame;

// --- The tunables. -------------------------------------------------------------------------------
//
// Set at M46 P12 on synthetic photos only, since no label is hit before P29 (P3 finding 10). Each
// names the bracket P29 may move it within.

/// How far the alignment searches either way on each axis, millimetres: the plan's ±2 mm, for the
/// label's re-find error.
///
/// The finder lands within 0.25 mm on synthetic photos (P9), and two drawn photos of one label from
/// different poses were fitted within 0.05 mm of each other at the cross (P12's finding), so every
/// shift found there was 0. A real fit's error is unknown, and a shift beyond the search is refused
/// rather than taken at its edge. P29 brackets it at `[1, 4]`.
pub const DIFF_SEARCH_MM: f64 = 2.0;

/// The least area, square millimetres, of a spot's newly dark part for the spot to hold something
/// new, and of its part on a previous mark for it to hold an old one.
///
/// About the opening's own disc (12.6 mm², [`MARK_OPEN_RADIUS_MM`](super::MARK_OPEN_RADIUS_MM)).
/// On the synthetic photos an old mark seen twice showed at most 0.06 mm² newly dark round its edge,
/// a new mark 5 mm in radius was 31 mm² new and up, and an old mark on its own place 0 mm² (P12's
/// finding). A real mark seen in two photos will differ by more, and how much is P29's to measure.
/// The cost of the floor is a strike landing within about 1.2 mm of an old mark's centre, whose new
/// crescent is under it (the module doc). P29 brackets it at `[5, 30]`.
pub const PART_MIN_AREA_MM2: f64 = 12.0;

/// The most the best shift's cost may be of the median cost over the whole reach, both at the
/// pyramid's top, for the alignment to stand.
///
/// A shift that lines the printing up leaves the label's noise, and every other shift leaves the
/// printing's strokes twice over as well, so the share is well under 1 when the printing picked a
/// shift and near 1 when none lines it up: the true shift beyond the reach, or a crop that is not
/// this label's. A best shift inside the reach is then any shift.
///
/// It is set loose, to refuse only a crop the printing does not match at all, because a refusal
/// sends every marked photo to a pick, and what a false shift risks is guarded by the old marks
/// themselves and by `PLACE_TAKEN_MM` (the module doc's limits). On the synthetic photos an aligned
/// pair came to 0.07 to 0.15, and 0.22 for a shift that is not a whole number of the top's blocks.
/// A reference moved 2.5 mm, half a millimetre beyond the reach, came to 0.49 at the reach's edge,
/// which the edge refused, and a partial match of repeated strokes to 0.48 (P12's finding). Real
/// photos' noise raises every share toward 1. P29 brackets it at `[0.3, 0.9]`.
pub const ALIGN_MAX_SHARE: f64 = 0.7;

/// A previous mark is still on the label when this share of its pixels lies on what is dark and wide
/// here. Half, because a mark that stayed is all of it there, give or take its edge, and one on a
/// label that was peeled off is none of it, unless a new mark happens to land on its place.
const OLD_PRESENT_SHARE: f64 = 0.5;

/// A spot whose centre lies this near an old mark's, millimetres in each crop's own printed frame,
/// stands where that mark stood, and vetoes calling the label new. Two fits of one label disagree by
/// at most the search's reach on each axis, √2 × 2 mm, and twice the reach leaves the rest for a
/// mark's centroid moving between two photos of it. Without the veto, an alignment wrong by more
/// than about 2.6 mm would leave even the smallest old mark less than half on itself, call the label
/// new, and read a lone old mark as a fresh label's first strike.
const PLACE_TAKEN_MM: f64 = 2.0 * DIFF_SEARCH_MM;

/// A shift is scored only where this share of this crop's aligning pixels have a partner in the
/// previous crop. Every shift in reach leaves most of a label to compare.
const ALIGN_MIN_OVERLAP: f64 = 0.5;

/// Halvings from the crop to the pyramid's top, where the whole reach is searched: at
/// [`CROP_PX_PER_MM`]'s 10, the top is 2.5 pixels a millimetre and the reach 5 pixels either way.
/// The printing's 0.45 mm dashes still darken the blocks they cross there.
const PYRAMID_STEPS: usize = 2;

/// How far either way each finer level searches about the coarser level's best, doubled. One
/// coarse pixel is two fine ones, so 2 covers it with a pixel to spare.
const REFINE_RADIUS: i64 = 2;

/// Why two crops could not be compared. [M46 P12]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffMiss {
    /// The previous crop is not this label type's grid: another size, or not well formed.
    OtherGrid,
    /// One crop's white could not be told ([`super::MarkMiss::NoWhite`]'s reasons).
    NoWhite,
    /// No shift in reach leaves enough of the label to compare, or the best lies on the search's
    /// edge, so the two crops may lie further apart than it reaches.
    Unaligned,
}

/// What one of this crop's spots holds, by the change. [M46 P12]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Holds {
    /// It is newly dark and lies on no previous mark: a new mark.
    New,
    /// It lies on a previous mark and has not darkened: an old mark.
    Old,
    /// Both: a new mark touching or over an old one.
    Both,
    /// Neither: dark now, yet neither a previous mark nor darkened since.
    Unclear,
}

/// The diff of this crop against the previous one. [M46 P12]
#[derive(Debug, Clone, PartialEq)]
pub struct Change {
    /// The alignment, crop pixels: the printed point at pixel `p` here is at `p + shift_px` in the
    /// previous crop.
    pub shift_px: [i64; 2],
    /// The mean difference in darkening at that shift, over the pixels it was searched on: the
    /// label's noise and what the two photos sample differently. For P29 to read, gating nothing.
    pub residual: f64,
    /// The least cost at the pyramid's top as a share of the median over the whole reach: near 1
    /// when no shift lines the printing up, and at most [`ALIGN_MAX_SHARE`] here.
    pub match_share: f64,
    /// The previous crop's spots, in its own pixels, each with whether it is still on this label.
    pub old: Vec<(Spot, bool)>,
    /// This crop's spots, [`find_marks`]'s, each with what it holds.
    pub spots: Vec<(Spot, Holds)>,
}

/// What a photo's crop holds that is new: the reader's answer before the label and orientation are
/// put round it. [M46 P12]
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Newest {
    /// One or two new marks, and whether the label holds nothing else.
    Marks {
        spots: Vec<Spot>,
        new_label: bool,
    },
    /// A new mark is cut by the label's edge.
    Clipped,
    Unmarked,
    /// Which is new is the golfer's to say.
    Pick,
}

/// The fresh label's rule (`mark.rs`), as a newest: P11's reader with no previous crop, and this
/// one's when the label is new.
pub(super) fn fresh(spots: &[Spot]) -> Newest {
    match on_a_fresh_label(spots) {
        Fresh::Mark(spot) => Newest::Marks {
            spots: vec![spot],
            new_label: true,
        },
        Fresh::Clipped => Newest::Clipped,
        Fresh::Unmarked => Newest::Unmarked,
        Fresh::Pick => Newest::Pick,
    }
}

impl Change {
    /// The outcome the module doc's list gives this change.
    pub(super) fn newest(&self) -> Newest {
        let still = self.old.iter().filter(|(_, here)| *here).count();
        if still == 0 {
            // A new label is called by the alignment, so it is checked without it: nothing here
            // may stand where an old mark stood, by any fit of the label the search allows.
            let taken = self.spots.iter().any(|(spot, _)| {
                self.old.iter().any(|(old, _)| {
                    let [dx, dy] = [0, 1].map(|k| spot.centre_mm[k] - old.centre_mm[k]);
                    dx.hypot(dy) <= PLACE_TAKEN_MM
                })
            });
            if taken {
                return Newest::Pick;
            }
            let spots: Vec<Spot> = self.spots.iter().map(|(spot, _)| *spot).collect();
            return fresh(&spots);
        }
        let unsure = self
            .spots
            .iter()
            .any(|(_, holds)| matches!(holds, Holds::Both | Holds::Unclear));
        if still < self.old.len() || unsure {
            return Newest::Pick;
        }
        let new: Vec<Spot> = self
            .spots
            .iter()
            .filter(|(_, holds)| *holds == Holds::New)
            .map(|(spot, _)| *spot)
            .collect();
        if new.is_empty() {
            Newest::Unmarked
        } else if new.iter().any(|spot| spot.clipped) {
            Newest::Clipped
        } else if new.len() > 2 || new.iter().any(|s| s.elongation > MARK_MAX_ELONGATION) {
            Newest::Pick
        } else {
            Newest::Marks {
                spots: new,
                new_label: false,
            }
        }
    }
}

/// One crop measured as `mark.rs` measures it, with what the diff needs kept. [`read_tape`]
/// measures the photo's crop once and hands it to both the marks and the diff.
///
/// [`read_tape`]: super::read_tape
pub(super) struct Measured {
    interior: Interior,
    darkening: Vec<f64>,
    wide: Vec<bool>,
    spots: Vec<(Spot, Vec<usize>)>,
}

impl Measured {
    pub(super) fn of(crop: &Crop, label: &LabelType) -> Result<Measured, DiffMiss> {
        let interior = Interior::of(crop, label);
        let white = White::fit(crop, &interior).ok_or(DiffMiss::NoWhite)?;
        let darkening = darkening(crop, &interior, &white);
        let wide = wide_dark(&darkening, &interior, crop.grid);
        let spots = spot_pixels(&darkening, &wide, &interior, crop.grid);
        Ok(Measured {
            interior,
            darkening,
            wide,
            spots,
        })
    }

    /// The crop's spots, [`find_marks`]'s.
    pub(super) fn spots(&self) -> Vec<Spot> {
        self.spots.iter().map(|(spot, _)| *spot).collect()
    }

    /// The pyramid's base: the darkening where the label is and no mark is.
    fn aligning(&self) -> Vec<bool> {
        (0..self.wide.len())
            .map(|at| self.interior.holds(at) && !self.wide[at])
            .collect()
    }
}

/// Diff `crop` against `previous`, the club's last crop of the same label type (the module doc).
/// [M46 P12]
pub fn diff_marks(crop: &Crop, previous: &Frame, label: &LabelType) -> Result<Change, DiffMiss> {
    same_grid(crop, previous)?;
    diff_measured(crop, &Measured::of(crop, label)?, previous, label)
}

fn same_grid(crop: &Crop, previous: &Frame) -> Result<(), DiffMiss> {
    let grid = crop.grid;
    let same = previous.width == grid.width && previous.height == grid.height;
    if previous.is_well_formed() && same {
        Ok(())
    } else {
        Err(DiffMiss::OtherGrid)
    }
}

/// [`diff_marks`] with this crop already measured.
pub(super) fn diff_measured(
    crop: &Crop,
    now: &Measured,
    previous: &Frame,
    label: &LabelType,
) -> Result<Change, DiffMiss> {
    same_grid(crop, previous)?;
    let grid = crop.grid;
    let before = Crop {
        grid,
        frame: previous.clone(),
        outside: previous.rgb.chunks_exact(3).map(|p| p == [0; 3]).collect(),
    };
    let then = Measured::of(&before, label)?;
    let reach = (DIFF_SEARCH_MM * grid.px_per_mm).round() as i64;
    let aligned = align(
        Level::base(grid, &now.darkening, &now.aligning()),
        Level::base(grid, &then.darkening, &then.aligning()),
        reach,
    )?;
    let shift = aligned.shift;
    let partner = |at: usize, [dx, dy]: [i64; 2]| -> Option<usize> {
        let x = (at % grid.width) as i64 + dx;
        let y = (at / grid.width) as i64 + dy;
        let inside = (0..grid.width as i64).contains(&x) && (0..grid.height as i64).contains(&y);
        inside.then(|| y as usize * grid.width + x as usize)
    };
    let mut on_old = vec![false; grid.width * grid.height];
    for (_, pixels) in &then.spots {
        for &at in pixels {
            on_old[at] = true;
        }
    }
    let part_min = (PART_MIN_AREA_MM2 * grid.px_per_mm * grid.px_per_mm).ceil() as usize;
    let spots = now
        .spots
        .iter()
        .map(|(spot, pixels)| {
            let (mut new, mut old) = (0, 0);
            for &at in pixels {
                let Some(there) = partner(at, shift) else {
                    continue;
                };
                let change = now.darkening[at] - then.darkening[there];
                new += usize::from(then.interior.holds(there) && change >= MARK_MIN_CONTRAST);
                old += usize::from(on_old[there]);
            }
            let holds = match (new >= part_min, old >= part_min) {
                (true, false) => Holds::New,
                (false, true) => Holds::Old,
                (true, true) => Holds::Both,
                (false, false) => Holds::Unclear,
            };
            (*spot, holds)
        })
        .collect();
    let back = [-shift[0], -shift[1]];
    let old = then
        .spots
        .iter()
        .map(|(spot, pixels)| {
            let here = pixels
                .iter()
                .filter(|&&at| partner(at, back).is_some_and(|p| now.wide[p]))
                .count();
            (
                *spot,
                here as f64 >= OLD_PRESENT_SHARE * pixels.len() as f64,
            )
        })
        .collect();
    Ok(Change {
        shift_px: shift,
        residual: aligned.residual,
        match_share: aligned.match_share,
        old,
        spots,
    })
}

/// One level of the alignment's pyramid: a darkening, `NaN` where a pixel does not align, so that
/// the cost's inner loop reads one array a side.
struct Level {
    width: usize,
    height: usize,
    value: Vec<f64>,
}

impl Level {
    fn base(grid: CropGrid, value: &[f64], aligning: &[bool]) -> Level {
        Level {
            width: grid.width,
            height: grid.height,
            value: value
                .iter()
                .zip(aligning)
                .map(|(&v, &a)| if a { v } else { f64::NAN })
                .collect(),
        }
    }

    fn aligning(&self) -> usize {
        self.value.iter().filter(|v| !v.is_nan()).count()
    }

    /// Each 2 × 2 block's mean over its aligning pixels, aligning when two or more are. An odd last
    /// row or column is dropped, which only trims the reach's far edge.
    fn halved(&self) -> Level {
        let (width, height) = (self.width / 2, self.height / 2);
        let mut value = vec![f64::NAN; width * height];
        for y in 0..height {
            for x in 0..width {
                let (mut sum, mut count) = (0.0, 0);
                for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                    let v = self.value[(2 * y + dy) * self.width + 2 * x + dx];
                    if !v.is_nan() {
                        sum += v;
                        count += 1;
                    }
                }
                if count >= 2 {
                    value[y * width + x] = sum / f64::from(count);
                }
            }
        }
        Level {
            width,
            height,
            value,
        }
    }

    /// The mean absolute difference between pixel `p` here and `p + shift` in `other`, over the
    /// pairs aligning in both, or `None` when fewer than `needed` are.
    fn cost(&self, other: &Level, [dx, dy]: [i64; 2], needed: usize) -> Option<f64> {
        let (mut sum, mut count) = (0.0, 0usize);
        let from = (-dx).max(0);
        let to = (other.width as i64 - dx).min(self.width as i64).max(from);
        let (from, span) = (from as usize, (to - from) as usize);
        for y in 0..self.height {
            let there_y = y as i64 + dy;
            if !(0..other.height as i64).contains(&there_y) {
                continue;
            }
            let here = &self.value[y * self.width + from..][..span];
            let there_from = there_y as usize * other.width + (from as i64 + dx) as usize;
            for (a, b) in here.iter().zip(&other.value[there_from..][..span]) {
                // `NaN` on either side drops the pair.
                let difference = (a - b).abs();
                if !difference.is_nan() {
                    sum += difference;
                    count += 1;
                }
            }
        }
        (count >= needed.max(1)).then(|| sum / count as f64)
    }
}

/// The alignment's answer: the shift, its cost at the crop's scale, and how clearly the printing
/// picked it.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Aligned {
    shift: [i64; 2],
    residual: f64,
    match_share: f64,
}

/// The shift that aligns `previous` on `current`, whole crop pixels within `reach` either way: the
/// whole reach searched at the pyramid's top, then each finer level about the coarser's best (the
/// module doc's step 2).
fn align(current: Level, previous: Level, reach: i64) -> Result<Aligned, DiffMiss> {
    let mut pyramid = vec![(current, previous)];
    for _ in 0..PYRAMID_STEPS {
        let (current, previous) = pyramid.last().expect("the base is there");
        let next = (current.halved(), previous.halved());
        pyramid.push(next);
    }
    let mut centre = [0, 0];
    let mut best = None;
    let mut match_share = 1.0;
    for (step, (current, previous)) in pyramid.iter().enumerate().rev() {
        let scale = 1_i64 << step;
        let limit = reach.div_euclid(scale) + i64::from(reach.rem_euclid(scale) != 0);
        let top = step == PYRAMID_STEPS;
        let radius = if top { limit } else { REFINE_RADIUS };
        let needed = (ALIGN_MIN_OVERLAP * current.aligning() as f64).ceil() as usize;
        let mut costs = Vec::new();
        best = None;
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                let shift = [centre[0] + dx, centre[1] + dy];
                if shift[0].abs() > limit || shift[1].abs() > limit {
                    continue;
                }
                if let Some(cost) = current.cost(previous, shift, needed) {
                    costs.push(cost);
                    if best.is_none_or(|(_, least)| cost < least) {
                        best = Some((shift, cost));
                    }
                }
            }
        }
        let (shift, least) = best.ok_or(DiffMiss::Unaligned)?;
        if top {
            let typical = crate::median(costs).ok_or(DiffMiss::Unaligned)?;
            match_share = if typical > 0.0 { least / typical } else { 1.0 };
        }
        centre = [2 * shift[0], 2 * shift[1]];
    }
    let (shift, residual) = best.ok_or(DiffMiss::Unaligned)?;
    let on_edge = shift[0].abs() >= reach || shift[1].abs() >= reach;
    if on_edge || match_share > ALIGN_MAX_SHARE {
        return Err(DiffMiss::Unaligned);
    }
    Ok(Aligned {
        shift,
        residual,
        match_share,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spot(centre_mm: [f64; 2]) -> Spot {
        Spot {
            centroid_px: [0.0, 0.0],
            centre_mm,
            area_mm2: 80.0,
            elongation: 1.02,
            clipped: false,
        }
    }

    fn change(old: &[bool], spots: &[(Spot, Holds)]) -> Change {
        Change {
            shift_px: [0, 0],
            residual: 0.0,
            match_share: 0.5,
            old: old.iter().map(|&here| (spot([9.0, 9.0]), here)).collect(),
            spots: spots.to_vec(),
        }
    }

    #[test]
    fn a_new_label_is_not_called_where_a_spot_stands_on_an_old_marks_place() {
        // Every old mark gone by the alignment, and the one spot 3.9 mm from where one stood: a
        // fit of the same label could put it there, so the golfer picks.
        let near = [(spot([9.0 + 3.9, 9.0]), Holds::New)];
        assert_eq!(change(&[false], &near).newest(), Newest::Pick);
        let far = [(spot([9.0 + 4.1, 9.0]), Holds::New)];
        assert!(matches!(
            change(&[false], &far).newest(),
            Newest::Marks {
                new_label: true,
                ..
            }
        ));
    }

    #[test]
    fn a_label_whose_old_marks_are_all_gone_is_read_by_the_fresh_labels_rule() {
        let one = [(spot([1.0, 2.0]), Holds::New)];
        let fresh_one = Newest::Marks {
            spots: vec![spot([1.0, 2.0])],
            new_label: true,
        };
        assert_eq!(change(&[false, false], &one).newest(), fresh_one);
        // No mark before reads the same: the label holds nothing but the new ones.
        assert_eq!(change(&[], &one).newest(), fresh_one);
        // Two marks on a new label need a pick, as on any fresh label.
        let two = [
            (spot([1.0, 2.0]), Holds::New),
            (spot([-9.0, 2.0]), Holds::New),
        ];
        assert_eq!(change(&[false], &two).newest(), Newest::Pick);
    }

    #[test]
    fn on_the_same_label_one_or_two_new_spots_are_read_and_the_old_ones_are_not() {
        let old = (spot([9.0, 9.0]), Holds::Old);
        let new = (spot([1.0, 2.0]), Holds::New);
        let second = (spot([-12.0, 0.0]), Holds::New);
        assert_eq!(
            change(&[true], &[old, new]).newest(),
            Newest::Marks {
                spots: vec![new.0],
                new_label: false
            }
        );
        assert_eq!(
            change(&[true], &[new, old, second]).newest(),
            Newest::Marks {
                spots: vec![new.0, second.0],
                new_label: false
            }
        );
        assert_eq!(change(&[true], &[old]).newest(), Newest::Unmarked);
        let third = (spot([12.0, 0.0]), Holds::New);
        assert_eq!(
            change(&[true], &[new, second, third]).newest(),
            Newest::Pick
        );
    }

    #[test]
    fn an_overlap_a_spot_untold_or_a_mark_gone_from_a_kept_label_needs_a_pick() {
        let old = (spot([9.0, 9.0]), Holds::Old);
        let new = (spot([1.0, 2.0]), Holds::New);
        let both = (spot([5.0, 5.0]), Holds::Both);
        let unclear = (spot([5.0, 5.0]), Holds::Unclear);
        assert_eq!(change(&[true], &[both]).newest(), Newest::Pick);
        assert_eq!(change(&[true], &[old, new, unclear]).newest(), Newest::Pick);
        assert_eq!(change(&[true, false], &[old, new]).newest(), Newest::Pick);
    }

    #[test]
    fn a_new_spot_cut_by_the_edge_refuses_the_photo_and_a_long_one_needs_a_pick() {
        let old = (spot([9.0, 9.0]), Holds::Old);
        let new = (spot([1.0, 2.0]), Holds::New);
        let clipped = (
            Spot {
                clipped: true,
                ..spot([0.0, -13.0])
            },
            Holds::New,
        );
        let long = (
            Spot {
                elongation: 1.6,
                ..spot([-10.0, 0.0])
            },
            Holds::New,
        );
        assert_eq!(change(&[true], &[old, clipped]).newest(), Newest::Clipped);
        assert_eq!(
            change(&[true], &[old, new, clipped]).newest(),
            Newest::Clipped
        );
        assert_eq!(change(&[true], &[old, long]).newest(), Newest::Pick);
    }

    /// A level of `width × height` whose value is `f` at each pixel, every pixel aligning.
    fn level(width: usize, height: usize, f: impl Fn(i64, i64) -> f64) -> Level {
        let value = (0..width * height)
            .map(|at| f((at % width) as i64, (at / width) as i64))
            .collect();
        Level {
            width,
            height,
            value,
        }
    }

    #[test]
    fn the_alignment_finds_a_whole_pixel_shift_and_refuses_one_it_cannot_reach() {
        // Thin strokes, 3 pixels wide, as the printing is at the crop's scale, over a broad bump
        // that slopes the cost toward the true shift from anywhere in reach. The rows are 61 apart,
        // so no shift in reach lines one up with the other: rows 28 apart, with the truth 24 beyond,
        // lined up at a shift of 4 instead, with a share of 0.48 (the module doc's limits).
        let printing = |x: i64, y: i64| {
            let column = [13, 41, 77, 102, 151].iter().any(|c| (x - c).abs() <= 1);
            let row = [9, 70].iter().any(|r| (y - r).abs() <= 1);
            let (bx, by) = ((x - 90) as f64, (y - 45) as f64);
            let bump = 0.3 * (-(bx * bx + by * by) / (2.0 * 30.0 * 30.0)).exp();
            if column || row {
                0.8
            } else {
                bump
            }
        };
        let (width, height) = (180, 90);
        let current = || level(width, height, printing);
        for truth in [[0, 0], [7, -3], [-19, 12], [4, 19]] {
            let previous = level(width, height, |x, y| printing(x - truth[0], y - truth[1]));
            let aligned = align(current(), previous, 20).expect("aligned");
            assert_eq!(aligned.shift, truth);
            assert!(aligned.residual < 1e-12, "{aligned:?}");
            // The top's blocks straddle a shift that is not a whole number of them.
            assert!(aligned.match_share < 0.5, "{aligned:?}");
        }
        // On the edge, and beyond it, the slope may run on: refused.
        for beyond in [[20, 0], [0, -24]] {
            let previous = level(width, height, |x, y| printing(x - beyond[0], y - beyond[1]));
            assert_eq!(
                align(current(), previous, 20),
                Err(DiffMiss::Unaligned),
                "{beyond:?}"
            );
        }
        // A flat previous crop, no printing at all: no shift is picked out.
        let flat = level(width, height, |_, _| 0.1);
        assert_eq!(align(current(), flat, 20), Err(DiffMiss::Unaligned));
    }

    #[test]
    fn a_previous_crop_of_another_size_is_another_grid() {
        let grid = CropGrid::new(&super::super::STRIKEPRO_IRON, super::super::CROP_PX_PER_MM);
        let crop = Crop {
            grid,
            frame: Frame {
                width: grid.width,
                height: grid.height,
                rgb: vec![200; grid.width * grid.height * 3],
            },
            outside: vec![false; grid.width * grid.height],
        };
        let other = Frame {
            width: grid.width + 1,
            height: grid.height,
            rgb: vec![200; (grid.width + 1) * grid.height * 3],
        };
        assert_eq!(
            diff_marks(&crop, &other, &super::super::STRIKEPRO_IRON),
            Err(DiffMiss::OtherGrid)
        );
        let short = Frame {
            rgb: vec![200; 5],
            ..crop.frame.clone()
        };
        assert_eq!(
            diff_marks(&crop, &short, &super::super::STRIKEPRO_IRON),
            Err(DiffMiss::OtherGrid)
        );
    }
}
