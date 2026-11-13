//! The STRIKEPRO label geometry, held to what P3 measured and to the shape later phases rely on.
//! [M46 P4]
//!
//! The record is P3 finding 4's table, copied here on purpose: this test is what says the table in
//! `src/tape/tape_labels.rs` is still the measurement and not an edit. Each outline is then held to
//! that record within the measurement's ±0.5 mm, and to being a polygon every later phase can draw,
//! fill and crop without a special case.

use impact::tape::{
    category_label, label_type, CategoryLabel, LabelType, CATEGORY_LABELS, LABEL_TYPES, PROVENANCE,
};

/// P3 finding 4's stated accuracy for an edge, millimetres.
const MEASURED_TO_MM: f64 = 0.5;

/// P3 finding 4's table: id, width × height, the cross from the left and top edges, and the
/// inscribed radius about the cross.
const RECORD: [(&str, f64, f64, f64, f64, f64); 3] = [
    ("strikepro_driver", 90.5, 49.6, 45.2, 25.1, 24.4),
    ("strikepro_hybrid", 73.5, 34.4, 36.2, 17.6, 16.8),
    ("strikepro_iron", 73.0, 39.9, 36.6, 24.8, 14.6),
];

/// P3 finding 4's edge distance from the cross every 30°, 0° the printed right and 90° up, with
/// the angles the logo or glare hid left out.
#[rustfmt::skip]
const PROFILE: [(&str, &[(f64, f64)]); 3] = [
    (
        "strikepro_driver",
        &[
            (0.0, 43.1), (30.0, 39.6), (90.0, 25.1), (120.0, 28.2), (150.0, 40.5), (180.0, 43.3),
            (210.0, 34.1), (240.0, 26.5), (270.0, 24.4), (300.0, 26.2), (330.0, 32.8),
        ],
    ),
    (
        "strikepro_hybrid",
        &[
            (0.0, 35.9), (30.0, 34.2), (60.0, 21.0), (90.0, 17.6), (120.0, 20.3), (150.0, 33.8),
            (180.0, 34.9), (210.0, 27.7), (270.0, 16.8), (300.0, 19.1), (330.0, 27.7),
        ],
    ),
    (
        "strikepro_iron",
        &[
            (0.0, 36.3), (30.0, 33.9), (60.0, 27.3), (120.0, 27.6), (150.0, 34.4), (180.0, 36.4),
            (210.0, 29.3), (240.0, 16.9), (270.0, 14.6), (300.0, 17.0), (330.0, 30.5),
        ],
    ),
];

fn label(id: &str) -> &'static LabelType {
    label_type(id).unwrap_or_else(|| panic!("no label type {id}"))
}

/// The outline's edges, each from a vertex to the next, the last closing to the first.
fn edges(label: &LabelType) -> impl Iterator<Item = ([f64; 2], [f64; 2])> + '_ {
    let n = label.outline_mm.len();
    (0..n).map(move |i| (label.outline_mm[i], label.outline_mm[(i + 1) % n]))
}

/// The distance from the cross to the nearest point of the outline.
fn outline_inscribed_radius(label: &LabelType) -> f64 {
    edges(label)
        .map(|(a, b)| {
            let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
            let t = (-(a[0] * dx + a[1] * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
            (a[0] + t * dx).hypot(a[1] + t * dy)
        })
        .fold(f64::INFINITY, f64::min)
}

/// Where a ray from the cross at `degrees` leaves the outline, as a distance.
fn ray_hit(label: &LabelType, degrees: f64) -> f64 {
    let (dx, dy) = (degrees.to_radians().cos(), degrees.to_radians().sin());
    edges(label)
        .filter_map(|(a, b)| {
            let (ex, ey) = (b[0] - a[0], b[1] - a[1]);
            let det = ex * dy - dx * ey;
            if det == 0.0 {
                return None;
            }
            let t = (ex * a[1] - ey * a[0]) / det;
            let s = (dx * a[1] - dy * a[0]) / det;
            (t > 0.0 && (0.0..=1.0).contains(&s)).then_some(t)
        })
        .fold(f64::INFINITY, f64::min)
}

fn angle_degrees(vertex: [f64; 2]) -> f64 {
    vertex[1].atan2(vertex[0]).to_degrees().rem_euclid(360.0)
}

#[test]
fn every_label_type_holds_the_record_p3_measured() {
    assert_eq!(
        LABEL_TYPES.len(),
        RECORD.len(),
        "one record row per label type"
    );
    for (id, width, height, from_left, from_top, inscribed) in RECORD {
        let label = label(id);
        assert_eq!(label.id, id);
        assert_eq!(label.width_mm, width, "{id} width");
        assert_eq!(label.height_mm, height, "{id} height");
        assert_eq!(
            label.centre_from_left_mm, from_left,
            "{id} cross from the left edge"
        );
        assert_eq!(
            label.centre_from_top_mm, from_top,
            "{id} cross from the top edge"
        );
        assert_eq!(
            label.inscribed_radius_mm, inscribed,
            "{id} inscribed radius"
        );
    }
}

#[test]
fn label_ids_are_distinct_and_unknown_ones_resolve_to_none() {
    for (i, a) in LABEL_TYPES.iter().enumerate() {
        for b in &LABEL_TYPES[i + 1..] {
            assert_ne!(a.id, b.id);
        }
        assert!(std::ptr::eq(label_type(a.id).unwrap(), *a));
    }
    assert!(label_type("strikepro_putter").is_none());
    assert!(label_type("").is_none());
}

#[test]
fn each_outline_spans_the_recorded_extents_within_the_measurement() {
    for label in LABEL_TYPES {
        let xs = label.outline_mm.iter().map(|v| v[0]);
        let ys = label.outline_mm.iter().map(|v| v[1]);
        let (left, right) = (
            xs.clone().fold(f64::INFINITY, f64::min),
            xs.fold(f64::NEG_INFINITY, f64::max),
        );
        let (bottom, top) = (
            ys.clone().fold(f64::INFINITY, f64::min),
            ys.fold(f64::NEG_INFINITY, f64::max),
        );
        let recorded = [
            ("left edge", -label.centre_from_left_mm, left),
            (
                "right edge",
                label.width_mm - label.centre_from_left_mm,
                right,
            ),
            ("top edge", label.centre_from_top_mm, top),
            (
                "bottom edge",
                label.centre_from_top_mm - label.height_mm,
                bottom,
            ),
        ];
        for (edge, expected, outline) in recorded {
            assert!(
                (outline - expected).abs() <= MEASURED_TO_MM,
                "{}: the outline's {edge} is at {outline}, the record's at {expected}",
                label.id
            );
        }
    }
}

#[test]
fn each_outlines_inscribed_radius_agrees_with_the_record_within_the_measurement() {
    for label in LABEL_TYPES {
        let outline = outline_inscribed_radius(label);
        assert!(
            (outline - label.inscribed_radius_mm).abs() <= MEASURED_TO_MM,
            "{}: the outline's inscribed radius is {outline}, the record's {}",
            label.id,
            label.inscribed_radius_mm
        );
    }
}

/// Finding 4's profile summarised each angle as the median edge distance within ±4° of it, which on
/// a tight curve is not the distance at the angle itself. At the driver's 150° and the hybrid's 60°
/// the edge points within 1.5° measure 39.1–40.5 and 20.0–20.5 mm, where the profile says 40.5 and
/// 21.0, and the outline passes through the points. So the profile is held at twice the edge's
/// accuracy, and the extents and inscribed radius above at the edge's own.
#[test]
fn each_outline_passes_through_finding_4s_profile() {
    for (id, profile) in PROFILE {
        let label = label(id);
        for &(degrees, distance) in profile {
            let hit = ray_hit(label, degrees);
            assert!(
                (hit - distance).abs() <= 2.0 * MEASURED_TO_MM,
                "{id} at {degrees}°: the outline is {hit:.2} mm from the cross, the profile {distance}"
            );
        }
    }
}

/// Strictly increasing angle about the cross, all the way round once, is what makes the outline a
/// simple, counter-clockwise polygon that every ray from the cross leaves exactly once. A renderer,
/// a point-in-label test and the crop can then all take it as it is.
#[test]
fn each_outline_winds_once_counter_clockwise_round_the_cross() {
    for label in LABEL_TYPES {
        let outline = label.outline_mm;
        assert!(outline.len() >= 3, "{}", label.id);
        assert!(
            outline.iter().flatten().all(|v| v.is_finite()),
            "{}: every vertex finite",
            label.id
        );
        let angles: Vec<f64> = outline.iter().copied().map(angle_degrees).collect();
        assert!(
            angles.windows(2).all(|pair| pair[0] < pair[1]),
            "{}: the vertices turn counter-clockwise, once",
            label.id
        );
        let doubled_area: f64 = edges(label).map(|(a, b)| a[0] * b[1] - b[0] * a[1]).sum();
        assert!(doubled_area > 0.0, "{}: counter-clockwise", label.id);
    }
}

#[test]
fn every_club_category_takes_a_label_or_is_named_as_taking_none() {
    // `contracts::intent::ClubCategory`'s wire names, every variant. This crate cannot see the enum,
    // so `contracts`' own drill test walks it against `category_label` (the M46 plan's P6); this
    // list is what P3's answers say each one takes.
    let expected = [
        ("driver", Some("strikepro_driver")),
        ("wood", Some("strikepro_hybrid")),
        ("hybrid", Some("strikepro_hybrid")),
        ("long_iron", Some("strikepro_iron")),
        ("mid_iron", Some("strikepro_iron")),
        ("short_iron", Some("strikepro_iron")),
        ("wedge", Some("strikepro_iron")),
        ("putter", None),
        ("all", None),
    ];
    assert_eq!(
        CATEGORY_LABELS.len(),
        expected.len(),
        "one row per category"
    );
    for (category, label) in expected {
        match (category_label(category), label) {
            (Some(CategoryLabel::Label(found)), Some(id)) => assert_eq!(found.id, id, "{category}"),
            (Some(CategoryLabel::NoLabel { why }), None) => {
                assert!(!why.is_empty(), "{category} says why it takes no label")
            }
            (found, _) => panic!("{category}: expected {label:?}, found {found:?}"),
        }
    }
    assert_eq!(
        category_label("fairway"),
        None,
        "an unnamed category is no row, not no label"
    );
}

#[test]
fn the_provenance_names_the_photo_by_its_sha256() {
    assert!(PROVENANCE.contains("data/raw/strike_tape/2026-11-12/da3ef6f9-image.jpg"));
    assert!(PROVENANCE.contains("5741307ba794deb3f90dfdec01c6faa2c746f6bc973edde4061c23039c4c8f15"));
    assert!(PROVENANCE.contains("exactly 1 mm"));
}
