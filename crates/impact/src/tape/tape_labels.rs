//! The STRIKEPRO label types' geometry, and which label each club category takes. [M46 P4]
//!
//! Data, written as Rust rather than JSON for the reason the module doc gives. Every number came
//! from one photo, by the method [`PROVENANCE`] names. The M46 plan's P3 finding 4 holds the full
//! table and how it was measured, and P4's finding holds how the outlines were drawn from it.
//!
//! # How the outlines were drawn
//!
//! P3 found each label's edge where the paper's grid lines stop, sampled every 1 mm along the grid,
//! and turned the points into the printed frame by the cross's arms. P4 drew the outlines from those
//! same points. They took the median radius about the cross in each 2° sector, simplified those by
//! Douglas–Peucker until no sector's median lay more than 0.2 mm off, and rounded to 0.1 mm. Every
//! edge point P3 kept is within 0.33 mm of its outline.
//!
//! **Where the edge was hidden**, by the logo near the top edge or by glare, there were no points:
//! about the cross, the driver's 54° to 92°, the hybrid's 58° to 86° and 216° to 264°, and the
//! iron's 72° to 118°. There the outline takes the label's measured opposite side, mirrored about
//! the outline's vertical middle. Where that side was hidden too, under the middle of the logo, a
//! straight chord bridges the rest. Finding 4 measured the hybrid's and the iron's tops flat there.
//! On the driver, a quadratic through the edge on either side of the whole gap departs by at most
//! 0.34 mm from a chord across all of it, so the shorter chord left is closer still. Against P3's
//! brightness outline, which is good to 0.2–0.4 mm away from the logo, the hybrid's mirrored
//! bottom-left lies within 0.3 mm. The iron's mirrored top-left corner meets the measured left
//! shoulder within 0.1 mm, where a chord would have cut it by about 0.8 mm.

use super::{CategoryLabel, LabelType};

/// Where every number in this file came from. P3 finding 11's words, written once.
///
/// **If the paper's squares are not 1 mm, every size here scales with them**, so a ruler laid on any
/// one label settles all three (P3 finding 11). That is the user's to offer. Strike Tape publishes no
/// sizes, and a published size that differs by more than 1 mm is the user's to decide.
pub const PROVENANCE: &str = "measured at M46 P3 off one photo of unused labels on graph paper \
     (`data/raw/strike_tape/2026-11-12/da3ef6f9-image.jpg`, sha256 \
     `5741307ba794deb3f90dfdec01c6faa2c746f6bc973edde4061c23039c4c8f15`), rectified by its grid; \
     assumes the paper's squares are exactly 1 mm; ±0.5 mm, the dashed printing ±1 mm; no size is \
     published (P3 finding 11)";

/// The driver label: an asymmetric oval, widest about 9 mm above the cross, flatter on top and
/// rounder below. Two dashed rings surround the red circle, marked `-7%` and `-15%` (P3 finding 4).
pub static STRIKEPRO_DRIVER: LabelType = LabelType {
    id: "strikepro_driver",
    width_mm: 90.5,
    height_mm: 49.6,
    centre_from_left_mm: 45.2,
    centre_from_top_mm: 25.1,
    inscribed_radius_mm: 24.4,
    outline_mm: &DRIVER_OUTLINE,
};

/// The hybrid label: a flat top edge, straight to within 0.3 mm over 55 mm, with rounded ends and a
/// bottom that curves up into them. Two dashed elliptical rings, unmarked (P3 finding 4).
pub static STRIKEPRO_HYBRID: LabelType = LabelType {
    id: "strikepro_hybrid",
    width_mm: 73.5,
    height_mm: 34.4,
    centre_from_left_mm: 36.2,
    centre_from_top_mm: 17.6,
    inscribed_radius_mm: 16.8,
    outline_mm: &HYBRID_OUTLINE,
};

/// The iron label: a straight bottom and sides, and a shouldered top, which rises at about 22° from
/// each side to a flat run of about 20 mm under the logo. A dashed grid, and the cross 4.8 mm below
/// the outline's box centre, the maker's choice (P3 findings 4 and 6).
pub static STRIKEPRO_IRON: LabelType = LabelType {
    id: "strikepro_iron",
    width_mm: 73.0,
    height_mm: 39.9,
    centre_from_left_mm: 36.6,
    centre_from_top_mm: 24.8,
    inscribed_radius_mm: 14.6,
    outline_mm: &IRON_OUTLINE,
};

/// Every label type, in the order a pack lists them.
pub static LABEL_TYPES: [&LabelType; 3] = [&STRIKEPRO_DRIVER, &STRIKEPRO_IRON, &STRIKEPRO_HYBRID];

/// Which label each `ClubCategory` takes, by wire name, one row per category.
///
/// The user's answers at P3 (finding 8): the driver label on the driver; the hybrid label on hybrids
/// **and fairway woods**; the iron label on every iron and wedge; and the putter none. A category
/// added to `ClubCategory` has no row until someone decides, and `contracts`' drill test walks the
/// enum against this table so that it fails rather than defaulting (the M46 plan's P6).
pub static CATEGORY_LABELS: [(&str, CategoryLabel); 9] = [
    ("driver", CategoryLabel::Label(&STRIKEPRO_DRIVER)),
    ("wood", CategoryLabel::Label(&STRIKEPRO_HYBRID)),
    ("hybrid", CategoryLabel::Label(&STRIKEPRO_HYBRID)),
    ("long_iron", CategoryLabel::Label(&STRIKEPRO_IRON)),
    ("mid_iron", CategoryLabel::Label(&STRIKEPRO_IRON)),
    ("short_iron", CategoryLabel::Label(&STRIKEPRO_IRON)),
    ("wedge", CategoryLabel::Label(&STRIKEPRO_IRON)),
    (
        "putter",
        CategoryLabel::NoLabel {
            why: "the putter takes no label and is out of the drill (the user, M46 P3 finding 8)",
        },
    ),
    (
        "all",
        CategoryLabel::NoLabel {
            why: "no club has this category: it is the benchmark rows' club-agnostic fallback \
                  (`contracts::club::CLUB_CATEGORY`)",
        },
    ),
];

// The outlines, in the printed frame about the cross, counter-clockwise from the vertex nearest +x,
// five vertices a line and left so by rustfmt, as a table is read across.

#[rustfmt::skip]
const DRIVER_OUTLINE: [[f64; 2]; 42] = [
    [44.1, 2.2], [44.4, 4.2], [45.0, 5.7], [45.2, 8.7], [44.3, 11.7],
    [42.4, 14.2], [39.9, 16.5], [36.6, 18.4], [30.1, 21.2], [27.1, 22.0],
    [18.7, 23.5], [16.5, 24.1], [8.5, 24.9], [-1.3, 25.1], [-8.3, 24.9],
    [-16.3, 24.1], [-28.9, 22.0], [-33.9, 20.1], [-36.9, 18.7], [-38.7, 17.6],
    [-42.4, 14.4], [-44.1, 11.9], [-45.0, 9.0], [-45.0, 5.5], [-43.7, 0.9],
    [-41.8, -3.6], [-38.7, -8.6], [-35.9, -11.8], [-32.2, -15.1], [-28.9, -17.5],
    [-24.4, -19.8], [-18.0, -22.0], [-8.0, -24.0], [1.0, -24.4], [4.0, -24.3],
    [11.0, -23.2], [19.0, -20.8], [26.1, -17.9], [30.1, -15.6], [36.1, -10.7],
    [39.3, -6.7], [41.3, -3.6],
];

#[rustfmt::skip]
const HYBRID_OUTLINE: [[f64; 2]; 32] = [
    [36.1, 0.5], [37.1, 4.5], [37.3, 10.0], [36.7, 11.6], [35.4, 13.6],
    [32.7, 15.6], [30.6, 16.7], [27.1, 17.5], [-26.9, 17.6], [-28.7, 17.2],
    [-31.2, 16.0], [-33.7, 14.3], [-35.5, 12.3], [-36.0, 11.1], [-36.2, 8.1],
    [-35.9, 4.1], [-35.2, 1.0], [-33.4, -3.9], [-29.7, -9.1], [-27.6, -11.2],
    [-24.6, -13.4], [-22.6, -14.4], [-20.2, -15.1], [-12.2, -16.2], [-3.2, -16.8],
    [7.4, -16.6], [13.4, -16.2], [21.4, -15.1], [22.4, -14.9], [27.7, -11.6],
    [31.4, -8.4], [34.0, -4.4],
];

#[rustfmt::skip]
const IRON_OUTLINE: [[f64; 2]; 25] = [
    [36.4, 9.6], [35.7, 12.1], [34.1, 14.2], [32.9, 15.1], [28.8, 17.1],
    [12.9, 24.0], [8.9, 24.8], [-9.1, 24.8], [-15.1, 23.4], [-29.1, 17.5],
    [-32.9, 15.7], [-35.0, 13.7], [-35.8, 12.4], [-36.5, 9.9], [-36.6, -7.1],
    [-36.4, -11.1], [-35.0, -13.3], [-33.0, -14.3], [-30.9, -14.5], [-10.9, -14.5],
    [29.6, -15.1], [32.1, -15.0], [34.2, -14.0], [35.2, -13.0], [36.1, -10.9],
];
