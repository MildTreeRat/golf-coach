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
//! Nothing yet finds the corners. That is P9's label finder.
//!
//! **P8 adds what the reader answers** ([`TapeOutcome`], C5's three) and [`read_tape`] as a stub
//! that finds no label in any photo, so that `golf-lab read-tape` is built over the shapes P9 to
//! P12 fill. The stub is P9's to replace.
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

mod geometry;
mod tape_labels;

use crate::frame::Frame;

pub use geometry::{rectify, Crop, CropGrid, Degenerate, Homography, CROP_PX_PER_MM};
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
}

/// Which way the club lies across the label's printed frame: what turns a printed millimetre into
/// C4's reading frame, `+x` toward the toe and `+y` toward the top line, whichever hand the club
/// is. It comes from the hosel and shaft (P10), never from the golfer's hand alone. [M46 P8]
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
    /// the table assumes (P4 finding 5). Whether logo-down is turned or refused is P9's and P10's.
    pub top_line_printed_top: bool,
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
/// ADR-037 clause 3), and `None` reads the label as fresh. [M46 P8]
///
/// **A stub until P9.** It answers `no_label` for every well-formed photo and `decode_failed` for
/// one that is not, so that `golf-lab read-tape` and its review images are built and tested over
/// the shapes before anything finds a label. P9 replaces it with the label finder, P10 adds the
/// orientation, and P11 and P12 the marks.
pub fn read_tape(photo: &Frame, previous: Option<&Frame>, label: &LabelType) -> TapeOutcome {
    let _ = (previous, label);
    let reason = if photo.is_well_formed() {
        TapeRefusal::NoLabel
    } else {
        TapeRefusal::DecodeFailed
    };
    TapeOutcome::NotRead {
        reason,
        found: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_stub_finds_no_label_in_a_photo_and_refuses_pixels_that_are_not_one() {
        let photo = Frame {
            width: 4,
            height: 2,
            rgb: vec![90; 4 * 2 * 3],
        };
        let outcome = read_tape(&photo, Some(&photo), &STRIKEPRO_IRON);
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
            read_tape(&short, None, &STRIKEPRO_DRIVER),
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
