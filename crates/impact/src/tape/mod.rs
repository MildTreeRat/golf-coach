//! Where on the face, read off a STRIKEPRO impact label stuck on it: the drill's method, the second
//! beside the overhead replay. [M46 P4]
//!
//! **P4 lands the labels' geometry and nothing that reads a photo.** The reader the M46 plan's C5
//! names, `read_tape`, arrives in P7 to P13 and takes one of these [`LabelType`]s. What is here is
//! the shape every later phase draws, crops and judges against: each label type's outline and
//! printed centre, and which label each club category takes.
//!
//! **P7 adds the geometry between a photo and a label** (`geometry.rs`): [`Homography`], the
//! perspective map from four corners in the photo to the same four on the label, and [`rectify`],
//! which cuts the label out through it into a [`Crop`] in the printed frame at [`CROP_PX_PER_MM`].
//! P9's label finder gives it the corners.
//!
//! **P8 adds what the reader answers** ([`TapeOutcome`], C5's three) and [`read_tape`] as a stub
//! that finds no label in any photo, so that `golf-lab read-tape` is built over the shapes P9 to
//! P12 fill.
//!
//! **P9 replaces the stub's first step with the label finder** (`label.rs`): [`find_label`] fits the
//! label type's measured outline to the bright, neutral region in the photo that it fits, and returns
//! the box's four corners in the printed frame with the crop cut through them, or a [`LabelMiss`].
//!
//! **P10 orients a found label from the club** (`orient.rs`): [`orient`] finds the shaft leaving
//! the label's heel end and running up, which settles both of [`Orientation`]'s flips, and checks
//! them against the golfer's hand when one is given. A label stuck logo-down is read, not refused.
//!
//! **P11 finds the marks on an oriented label** (`mark.rs`): [`find_marks`] takes each dark spot on
//! the crop wide enough to be a strike's, against the label's own white, and [`read_tape`] reads a
//! fresh label's one mark.
//!
//! **P12 tells the new marks from the old** (`diff.rs`): [`diff_marks`] aligns the club's last crop
//! on this one by the label's printing and asks of each spot whether it has darkened since, so a
//! used label's newest mark is read, two are read when a photo was missed, and a label whose old
//! marks are all gone is read as new.
//!
//! # A Rust table, not `tape_labels.json`
//!
//! The plan's C3 asked for JSON read by `include_str!`, as `contracts::catalogue` reads its rows,
//! **or** a Rust table if a hand parser would pass 80 lines (the M46 plan's P4). It would: this
//! crate takes no `serde` (its manifest says why), and a parser of nested arrays, numbers, strings
//! and `null`, plus the code to lift its values into these fields, is well past that. Nothing outside
//! Rust reads the geometry either, since the app is handed it as normalised coordinates (C20). So
//! the table is `tape_labels.rs`, beside this file, and the compiler checks every field a parser
//! would have had to.
//!
//! # The printed frame
//!
//! Every number is in the label's **printed frame**: millimetres, the origin at the printed red
//! cross, `+y` toward the edge the logo runs along, and `+x` toward the printed right, the end the
//! logo's `.com` reads toward. That is how the label lies flat, and it is the one frame that does
//! not depend on the golfer.
//!
//! **It is not C4's reading frame**, which puts `+x` toward the toe whichever hand the club is. A
//! label stuck logo-up on a right-handed club, seen face-on with the shaft up, has its printed left
//! at the toe (C4: the toe is on the image's left), so there a reading's `x` is the printed `x`
//! negated, and on a left-handed club it is the printed `x`. `y` is the same in both. Which way a
//! photo lies is the reader's to settle from the hosel (P10), never from this table. Distances are
//! the same in both frames, so [`LabelType::inscribed_radius_mm`] needs no conversion.
//!
//! The labels are sold for either hand (the M46 plan's P0 finding 2), and each measured outline is
//! mirror-symmetric about its own vertical middle to within the measurement. The printed cross is
//! not quite on that middle, and sits up to 0.6 mm to one side on the hybrid label (P3 finding 4),
//! which is printing registration or measurement error. Nothing is known to fix it per label.

mod diff;
mod geometry;
mod label;
mod mark;
mod orient;
mod tape_labels;

use crate::frame::Frame;
use diff::{diff_measured, fresh, Measured, Newest};

pub use diff::{
    diff_marks, Change, DiffMiss, Holds, ALIGN_MAX_SHARE, DIFF_SEARCH_MM, PART_MIN_AREA_MM2,
};
pub use geometry::{rectify, Crop, CropGrid, Degenerate, Homography, CROP_PX_PER_MM};
pub use label::{
    find_label, LabelMiss, OutlineFit, ASPECT_TOLERANCE, LABEL_SATURATION_MAX, LOGO_CONTRAST,
    MAX_OUTSIDE_SHARE, MIN_INLIER_SHARE, MIN_OUTLINE_SUPPORT, MIN_PX_PER_MM, OUTLINE_INLIER_MM,
};
pub use mark::{
    find_marks, MarkMiss, Spot, MARK_EDGE_MARGIN_MM, MARK_MAX_ELONGATION, MARK_MIN_AREA_MM2,
    MARK_MIN_CONTRAST, MARK_OPEN_RADIUS_MM, WHITE_BAND_MM,
};
pub use orient::{
    orient, OrientMiss, Shaft, HEEL_MIN_SHARE, ORIENT_PX_PER_MM, SEARCH_REACH_MM, SHAFT_DARK_RATIO,
    SHAFT_MAX_ANGLE_DEG, SHAFT_MAX_GAP_MM, SHAFT_MAX_WIDTH_MM, SHAFT_MIN_ANGLE_DEG,
    SHAFT_MIN_ASPECT, SHAFT_MIN_LENGTH_MM,
};
pub use tape_labels::{
    CATEGORY_LABELS, LABEL_TYPES, PROVENANCE, STRIKEPRO_DRIVER, STRIKEPRO_HYBRID, STRIKEPRO_IRON,
};

/// One STRIKEPRO label type: its outline and its printed centre, in millimetres, in the printed
/// frame (the module doc). Every number is from one unused label, measured off one photo
/// ([`PROVENANCE`]).
///
/// The record numbers (size, centre and inscribed radius) are kept beside the outline rather than
/// derived from it, because they are what the user took as the record (P3 finding 11). The outline
/// agrees with each of them to within the measurement's ±0.5 mm, and `tests/tape_labels.rs` holds it
/// there.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LabelType {
    /// `strikepro_driver`, `strikepro_iron` or `strikepro_hybrid`. A stored strike names its label
    /// type by this (the M46 plan's C8), so a spelling here is a stored value.
    pub id: &'static str,
    /// The outline's extent across, left edge to right edge, at the label's widest.
    pub width_mm: f64,
    /// The outline's extent top to bottom, at the label's tallest.
    pub height_mm: f64,
    /// **The label's centre**, the printed red cross, from the outline's left edge. The user chose
    /// the cross over the outline's box centre (P3 finding 6): the maker printed the cross off the
    /// box centre on the iron label on purpose, and the distance rings are centred on it.
    pub centre_from_left_mm: f64,
    /// The printed red cross, down from the outline's top edge.
    pub centre_from_top_mm: f64,
    /// The largest circle about the cross that stays on the label: the cross's distance to the
    /// nearest edge, which is the bottom edge on all three types.
    ///
    /// A ladder radius above it is trimmed (the M46 plan's C9), so that a strike declared off the
    /// tape is outside every circle (C6).
    pub inscribed_radius_mm: f64,
    /// The outline, counter-clockwise from the vertex nearest `+x`, about the cross. Every ray from
    /// the cross meets it once, which the test holds. It closes back to its first vertex.
    pub outline_mm: &'static [[f64; 2]],
}

/// The label a club category takes, or that it takes none and why.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CategoryLabel {
    Label(&'static LabelType),
    /// The category is in the drill's table on purpose and has no label. `why` is for whoever reads
    /// the table, not a sentence for a golfer.
    NoLabel {
        why: &'static str,
    },
}

/// The label type with this [`LabelType::id`], or `None` for an id no label type has.
pub fn label_type(id: &str) -> Option<&'static LabelType> {
    LABEL_TYPES.iter().copied().find(|label| label.id == id)
}

/// The label a category takes, by `contracts::intent::ClubCategory`'s wire name.
///
/// By wire name because this crate takes no dependency, `contracts` included. `None` means the
/// table does not name the category at all, which for a real category is a missing row, never a
/// "no label": `CategoryLabel::NoLabel` is how a category is named as having none.
pub fn category_label(category: &str) -> Option<CategoryLabel> {
    CATEGORY_LABELS
        .iter()
        .find(|(name, _)| *name == category)
        .map(|(_, label)| *label)
}

/// What [`read_tape`] answers for one photo: the M46 plan's C5, ADR-037 clause 3. [M46 P8]
#[derive(Debug, Clone, PartialEq)]
pub enum TapeOutcome {
    /// The label was found, squared and oriented, and one or two new marks were placed on it. Two
    /// means a photo was missed, and both count (C10).
    Read(TapeReading),
    /// The label was found, squared and oriented, but the new mark cannot be told apart (an
    /// overlap, or more than two candidates). The golfer taps it on the crop (C6).
    NeedsPick(TapePick),
    /// Nothing was placed, and why.
    ///
    /// `found` is the label when the reader got that far, so for every reason past the finder
    /// (`no_orientation`, `no_new_mark`, `mark_clipped`) and never for `no_label` or
    /// `decode_failed`. C5 names a bare reason, and the label is here because C15 holds a
    /// `no_new_mark` strike pending "with its crop" until it is declared off the tape, and because a
    /// refusal's review image is how a refusal is tuned (P8's finding).
    NotRead {
        reason: TapeRefusal,
        found: Option<FoundLabel>,
    },
}

impl TapeOutcome {
    /// The label this photo was found to hold, whatever the outcome, when it was found at all.
    pub fn found(&self) -> Option<&FoundLabel> {
        match self {
            TapeOutcome::Read(reading) => Some(&reading.found),
            TapeOutcome::NeedsPick(pick) => Some(&pick.found),
            TapeOutcome::NotRead { found, .. } => found.as_ref(),
        }
    }
}

/// A label found in a photo and squared to its printed frame. [M46 P8]
#[derive(Debug, Clone, PartialEq)]
pub struct FoundLabel {
    /// The label's corners in the photo, in its pixels, paired with
    /// [`LabelType::box_corners_mm`] by index: the printed frame's top-left, top-right,
    /// bottom-right and bottom-left. [`Homography::from_corners`] of the two is the map the crop was
    /// cut through, so it is not held twice.
    pub corners: [[f64; 2]; 4],
    /// The label cut out through that map at [`CROP_PX_PER_MM`].
    pub crop: Crop,
}

/// A photo read: the label, which way the club lay, and the new marks on it. [M46 P8]
#[derive(Debug, Clone, PartialEq)]
pub struct TapeReading {
    pub found: FoundLabel,
    pub orientation: Orientation,
    /// The label has no mark but the new ones: a fresh label, so the club's label counter restarts
    /// (D4, P11).
    pub new_label: bool,
    /// One or two. Their order is the reader's and means nothing.
    pub marks: Vec<TapeMark>,
}

/// A photo whose new mark the golfer must tap: the label, squared and oriented, so that the tap
/// becomes millimetres in the reading frame (C6). [M46 P8]
#[derive(Debug, Clone, PartialEq)]
pub struct TapePick {
    pub found: FoundLabel,
    pub orientation: Orientation,
}

/// One new mark on the label. [M46 P8]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TapeMark {
    /// The mark's centroid in the crop's pixels, in [`CropGrid`]'s continuous coordinates.
    pub centroid_px: [f64; 2],
    /// Where it landed, heel to toe, millimetres from the label's centre in C4's reading frame:
    /// `+` toward the toe, as `ShotData`'s `impact_offset_h` reads.
    pub offset_h_mm: f64,
    /// High to low, millimetres from the label's centre: `+` toward the top line.
    pub offset_v_mm: f64,
    /// The mark's area on the label, square millimetres: what [`find_marks`] kept of it. No
    /// strike's size is known before P29 (P3 finding 10), so the reader reports each, for the
    /// review's ring and for P29 to set [`MARK_MIN_AREA_MM2`] from. [M46 P11]
    pub area_mm2: f64,
}

/// Which way the club lies across the label's printed frame: what turns a printed millimetre into
/// C4's reading frame, `+x` toward the toe and `+y` toward the top line, whichever hand the club
/// is. It comes from the hosel and shaft ([`orient`]), never from the golfer's hand alone. [M46 P8,
/// P10]
///
/// Two flips and no turn, because a label is stuck along the face, so its printed axes lie along
/// the face's (the module doc). Each flip is its own inverse, so one map runs both ways.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Orientation {
    /// The toe is toward the printed right, the end the logo's `.com` reads toward. A label stuck
    /// logo-up has it so on a left-handed club, and has its printed left at the toe on a
    /// right-handed one.
    pub toe_printed_right: bool,
    /// The top line is toward the printed top, the logo's edge: the label was stuck logo-up, as
    /// the table assumes (P4 finding 5). The finder (P9) finds a label stuck logo-down in its
    /// printed frame as it finds one at any turn, by the outline's shape. [`orient`] reads it,
    /// with this flag `false` and the toe's flipped too, because the shaft says which way is up.
    pub top_line_printed_top: bool,
}

/// Which hand a club is for: the golfer's, as `contracts::golfer::Handedness` holds it. A caller
/// maps that across by hand, since this crate takes no dependency, `contracts` included. [M46 P10]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Handedness {
    Right,
    Left,
}

impl Handedness {
    /// `contracts::golfer::Handedness`'s wire name.
    pub const fn as_str(self) -> &'static str {
        match self {
            Handedness::Right => "right",
            Handedness::Left => "left",
        }
    }
}

impl Orientation {
    /// A point in the printed frame, in the reading frame.
    pub fn to_reading_mm(self, [x, y]: [f64; 2]) -> [f64; 2] {
        [
            if self.toe_printed_right { x } else { -x },
            if self.top_line_printed_top { y } else { -y },
        ]
    }

    /// A point in the reading frame, in the printed frame: the same flips, each its own inverse.
    pub fn to_printed_mm(self, reading: [f64; 2]) -> [f64; 2] {
        self.to_reading_mm(reading)
    }

    /// The hand of club that lies this way across a label seen from the front. [M46 P10]
    ///
    /// Face-on, with the top line up, a right-handed club has its toe on the photo's left (C4) and
    /// a left-handed one on its right. A label stuck logo-up lies in the photo as printed, so a
    /// right-handed club has its toe printed left and a left-handed one printed right. Stuck
    /// logo-down, the label is turned half a turn, which flips both flags and keeps the hand. Only
    /// a mirror would flip one alone, and the finder refuses a mirrored label (P7's `Mirrored`). So
    /// the two flags agree on a left-handed club and differ on a right-handed one.
    pub fn handedness(self) -> Handedness {
        if self.toe_printed_right == self.top_line_printed_top {
            Handedness::Left
        } else {
            Handedness::Right
        }
    }
}

/// Why a photo placed nothing: C5's five, by the wire names `contracts::drill::TapeNotRead`
/// spells. [M46 P8]
///
/// **No sentences here.** This crate cannot see `contracts` (it takes no dependency), so the
/// golfer's sentence and the `retake_helps` flag are `TapeNotRead`'s, and a caller maps a reason
/// across by [`TapeRefusal::as_str`] (the M46 plan's P6 finding 5). Every reason is in
/// [`TapeRefusal::ALL`] so that the mapping can be walked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TapeRefusal {
    /// No label was found in the photo.
    NoLabel,
    /// The label was found, but the hosel and shaft were not, or they disagreed with the golfer's
    /// hand (C4).
    NoOrientation,
    /// The label was found and squared, and nothing on it is new since the last strike.
    NoNewMark,
    /// The new mark touches the label's edge, so its centre is not where the mark's middle is.
    MarkClipped,
    /// The photo is not pixels this reader can read: from [`read_tape`], a frame that is not well
    /// formed ([`Frame::is_well_formed`]); from a caller, a photo that would not decode.
    DecodeFailed,
}

impl TapeRefusal {
    /// Every reason, in C5's order.
    pub const ALL: [TapeRefusal; 5] = [
        TapeRefusal::NoLabel,
        TapeRefusal::NoOrientation,
        TapeRefusal::NoNewMark,
        TapeRefusal::MarkClipped,
        TapeRefusal::DecodeFailed,
    ];

    /// The reason's wire name, `contracts::drill::TapeNotRead`'s spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            TapeRefusal::NoLabel => "no_label",
            TapeRefusal::NoOrientation => "no_orientation",
            TapeRefusal::NoNewMark => "no_new_mark",
            TapeRefusal::MarkClipped => "mark_clipped",
            TapeRefusal::DecodeFailed => "decode_failed",
        }
    }
}

/// Read one photo of a label on the club: C5's reader. `previous` is the club's last crop (C16,
/// ADR-037 clause 3), and `None` reads the label as fresh. `hand` is the golfer's, which checks the
/// orientation the shaft gives (C4), and `None` leaves the shaft alone to decide. [M46 P8, P9, P10,
/// P11, P12]
///
/// A photo that is not well formed ([`Frame::is_well_formed`]) is `decode_failed`, and one in which
/// [`find_label`] finds no label is `no_label`, whatever its [`LabelMiss`]. A found label that
/// [`orient`] cannot orient is `no_orientation`, whatever its [`OrientMiss`], and carries the label.
/// Orientation comes before the marks, so no mark is looked for on a label with no sides (P8's
/// finding).
///
/// **The marks** are [`find_marks`]'s spots. A label with none is `no_new_mark`, whatever came
/// before it. With no previous crop the label is fresh (D4): one whole spot is read, with
/// `new_label`; one cut by the edge is `mark_clipped`; and two or more, or one too long to be one
/// mark ([`MARK_MAX_ELONGATION`]), need the golfer's pick. So does a label whose white could not be
/// told ([`MarkMiss`]), since that is a label found and squared whose mark cannot be placed (C5).
///
/// **With a previous crop**, a marked label is diffed against it ([`diff_marks`], whose module doc
/// lists the outcomes): one or two new marks are read, an overlap needs a pick, nothing new is
/// `no_new_mark`, and a label whose old marks are all gone is fresh. A previous crop that cannot be
/// compared ([`DiffMiss`]) leaves a marked label to the golfer's pick, since read as fresh an old
/// mark would come out as the new one, a wrong reading where a pick is owed (the M46 plan's rule 6).
pub fn read_tape(
    photo: &Frame,
    previous: Option<&Frame>,
    label: &LabelType,
    hand: Option<Handedness>,
) -> TapeOutcome {
    if !photo.is_well_formed() {
        return TapeOutcome::NotRead {
            reason: TapeRefusal::DecodeFailed,
            found: None,
        };
    }
    let found = match find_label(photo, label) {
        Ok((found, _)) => found,
        Err(_) => {
            return TapeOutcome::NotRead {
                reason: TapeRefusal::NoLabel,
                found: None,
            }
        }
    };
    let orientation = match orient(photo, &found, label, hand) {
        Ok((orientation, _)) => orientation,
        Err(_) => {
            return TapeOutcome::NotRead {
                reason: TapeRefusal::NoOrientation,
                found: Some(found),
            }
        }
    };
    // `find_marks`, measured once for the diff to reuse.
    let Ok(measured) = Measured::of(&found.crop, label) else {
        return TapeOutcome::NeedsPick(TapePick { found, orientation });
    };
    let spots = measured.spots();
    let refused = |reason, found| TapeOutcome::NotRead {
        reason,
        found: Some(found),
    };
    let newest = match previous {
        None => fresh(&spots),
        // A label with no spot has nothing new, whatever came before.
        Some(_) if spots.is_empty() => Newest::Unmarked,
        Some(previous) => match diff_measured(&found.crop, &measured, previous, label) {
            Ok(change) => change.newest(),
            Err(_) => Newest::Pick,
        },
    };
    match newest {
        Newest::Marks { spots, new_label } => TapeOutcome::Read(TapeReading {
            found,
            orientation,
            new_label,
            marks: spots.iter().map(|spot| spot.reading(orientation)).collect(),
        }),
        Newest::Clipped => refused(TapeRefusal::MarkClipped, found),
        Newest::Unmarked => refused(TapeRefusal::NoNewMark, found),
        Newest::Pick => TapeOutcome::NeedsPick(TapePick { found, orientation }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_flat_photo_has_no_label_and_pixels_that_are_not_one_are_refused() {
        let photo = Frame {
            width: 4,
            height: 2,
            rgb: vec![90; 4 * 2 * 3],
        };
        let outcome = read_tape(&photo, Some(&photo), &STRIKEPRO_IRON, None);
        assert_eq!(
            outcome,
            TapeOutcome::NotRead {
                reason: TapeRefusal::NoLabel,
                found: None
            }
        );
        assert_eq!(outcome.found(), None);
        let short = Frame {
            rgb: vec![90; 5],
            ..photo
        };
        assert_eq!(
            read_tape(&short, None, &STRIKEPRO_DRIVER, Some(Handedness::Left)),
            TapeOutcome::NotRead {
                reason: TapeRefusal::DecodeFailed,
                found: None
            }
        );
    }

    #[test]
    fn an_orientation_flips_each_axis_it_names_and_undoes_itself() {
        let point = [3.0, -2.0];
        for (toe_printed_right, top_line_printed_top, reading) in [
            (true, true, [3.0, -2.0]),
            (false, true, [-3.0, -2.0]),
            (true, false, [3.0, 2.0]),
            (false, false, [-3.0, 2.0]),
        ] {
            let orientation = Orientation {
                toe_printed_right,
                top_line_printed_top,
            };
            assert_eq!(orientation.to_reading_mm(point), reading);
            assert_eq!(orientation.to_printed_mm(reading), point);
        }
    }

    #[test]
    fn the_two_flags_agree_on_a_left_handed_club_and_a_half_turn_keeps_the_hand() {
        let hand = |toe_printed_right, top_line_printed_top| {
            Orientation {
                toe_printed_right,
                top_line_printed_top,
            }
            .handedness()
        };
        // Logo-up, then logo-down, for each hand.
        assert_eq!(hand(false, true), Handedness::Right);
        assert_eq!(hand(true, false), Handedness::Right);
        assert_eq!(hand(true, true), Handedness::Left);
        assert_eq!(hand(false, false), Handedness::Left);
        assert_eq!(Handedness::Right.as_str(), "right");
        assert_eq!(Handedness::Left.as_str(), "left");
    }

    #[test]
    fn every_reason_has_its_own_wire_name() {
        let names: Vec<&str> = TapeRefusal::ALL.iter().map(|r| r.as_str()).collect();
        assert_eq!(
            names,
            [
                "no_label",
                "no_orientation",
                "no_new_mark",
                "mark_clipped",
                "decode_failed"
            ]
        );
    }
}
