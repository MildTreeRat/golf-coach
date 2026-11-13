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
