//! The mark on a fresh label, on synthetic photos of a label stuck on a club with its hosel and
//! shaft in frame. [M46 P11]
//!
//! Each photo is drawn by `tape_scene`, with marks at known printed-frame millimetres, and each
//! reading is held to the scene's own answer: the mark's centre turned into the reading frame by the
//! scene's [`truth`], never by the reader's orientation. No label is hit before P29 (P3 finding 10),
//! so these are the mark's only tests until then, and the marks are the renderer's guess at a
//! strike: discs of `tape_scene::MARK`, darker than the white and lighter than the printing.

mod tape_scene;

use impact::tape::{
    find_marks, read_tape, LabelType, Spot, TapeOutcome, TapePick, TapeReading, TapeRefusal,
    MARK_MAX_ELONGATION, STRIKEPRO_DRIVER, STRIKEPRO_HYBRID, STRIKEPRO_IRON,
};
use tape_scene::{framed, keep, tilted_poses, truth, Hand, Mark, Scene, Shaft};

/// A mark 12 mm across, inside the iron label's inscribed circle with room, and well below the
/// contact patch a full swing leaves, which no photo here has measured.
const RADIUS_MM: f64 = 6.0;

/// The most a read mark may lie from the scene's own, millimetres: the section's bound.
const WITHIN_MM: f64 = 0.2;

fn labels() -> [&'static LabelType; 3] {
    [&STRIKEPRO_DRIVER, &STRIKEPRO_HYBRID, &STRIKEPRO_IRON]
}

/// Printed-frame places a whole 6 mm mark fits on each label, over its printing: the cross, the
/// dashed rings and grid, and the driver's text.
fn places(label: &LabelType) -> [[f64; 2]; 4] {
    match label.id {
        "strikepro_driver" => [[0.0, 0.0], [-28.0, 1.0], [20.0, -6.0], [6.0, 11.0]],
        "strikepro_hybrid" => [[0.0, 0.0], [-20.0, 3.0], [15.0, -3.0], [4.0, 6.0]],
        _ => [[0.0, 0.0], [-18.5, 2.0], [14.0, -3.0], [3.0, 11.0]],
    }
}

/// An outcome without its crop's pixels, for a failure's message.
fn brief(outcome: &TapeOutcome) -> String {
    match outcome {
        TapeOutcome::Read(reading) => format!("read {:?}", reading.marks),
        TapeOutcome::NeedsPick(pick) => format!("needs a pick ({:?})", pick.orientation),
        TapeOutcome::NotRead { reason, found } => {
            format!(
                "not read: {} (label found: {})",
                reason.as_str(),
                found.is_some()
            )
        }
    }
}

fn mark(centre_mm: [f64; 2], radius_mm: f64) -> Mark {
    Mark {
        centre_mm,
        radius_mm,
    }
}

/// Draws the scene, keeps it under `TAPE_SCENE_DIR` when that is set, and reads it as fresh with
/// the scene's own hand. That the hand only checks the shaft is `tape_orient.rs`'s to pin.
fn read(scene: &Scene, name: &str) -> TapeOutcome {
    let photo = scene.draw();
    keep(&photo, name);
    read_tape(&photo, None, scene.label, Some(scene.hand))
}

/// A scene's mark in the reading frame, from the scene alone.
fn truth_of(scene: &Scene, mark: &Mark) -> [f64; 2] {
    truth(scene).to_reading_mm(mark.centre_mm)
}

/// Holds a fresh label's one mark to the scene's own, and returns how far off it was.
fn reads_its_mark(scene: &Scene, name: &str) -> f64 {
    let outcome = read(scene, name);
    let TapeOutcome::Read(TapeReading {
        orientation,
        new_label,
        marks,
        ..
    }) = &outcome
    else {
        panic!("{name}: not read: {}", brief(&outcome));
    };
    assert!(*new_label, "{name}");
    assert_eq!(*orientation, truth(scene), "{name}");
    assert_eq!(marks.len(), 1, "{name}");
    let [h, v] = truth_of(scene, &scene.marks[0]);
    let error = (marks[0].offset_h_mm - h).hypot(marks[0].offset_v_mm - v);
    let disc = std::f64::consts::PI * scene.marks[0].radius_mm.powi(2);
    // Printed with `--nocapture`, for the plan's findings.
    eprintln!(
        "{name}: off {error:.3} mm, area {:.1} of {disc:.1} mm²",
        marks[0].area_mm2
    );
    assert!(
        error <= WITHIN_MM,
        "{name}: {:?} is {error:.3} mm off",
        marks[0]
    );
    assert!(
        (marks[0].area_mm2 - disc).abs() <= 0.05 * disc,
        "{name}: area {}",
        marks[0].area_mm2
    );
    error
}

#[test]
fn one_mark_on_a_fresh_label_is_read_within_a_fifth_of_a_millimetre_either_hand_either_way_up() {
    let mut worst: f64 = 0.0;
    for label in labels() {
        for (index, (hand, logo_up)) in [
            (Hand::Right, true),
            (Hand::Right, false),
            (Hand::Left, true),
            (Hand::Left, false),
        ]
        .into_iter()
        .enumerate()
        {
            let mut scene = framed(label, hand, logo_up, [0.0, 0.0, 0.0], 200.0);
            let at = places(label)[index];
            scene.marks = vec![mark(at, RADIUS_MM)];
            let name = format!("mark {at:?} {hand:?} logo-up {logo_up} {}", label.id);
            worst = worst.max(reads_its_mark(&scene, &name));
        }
    }
    eprintln!("face-on: worst {worst:.3} mm");
}

#[test]
fn a_mark_on_a_label_up_to_thirty_degrees_off_square_is_read_within_a_fifth_of_a_millimetre() {
    let mut worst: f64 = 0.0;
    for (index, pose) in tilted_poses().into_iter().enumerate() {
        let mut scene = pose.scene();
        let at = places(pose.label)[index % 4];
        scene.marks = vec![mark(at, RADIUS_MM)];
        let name = format!("tilted pose {index} mark {at:?} {}", pose.label.id);
        worst = worst.max(reads_its_mark(&scene, &name));
    }
    eprintln!("tilted: worst {worst:.3} mm");
}

#[test]
fn an_unmarked_label_has_no_new_mark_and_none_of_its_printing_is_taken_for_one() {
    let mut scenes = Vec::new();
    for label in labels() {
        scenes.push((
            format!("unmarked {}", label.id),
            framed(label, Hand::Right, true, [0.0, 0.0, 0.0], 200.0),
        ));
        // At 240 mm the label samples about 4.2 photo pixels a millimetre, near the finder's
        // least (`MIN_PX_PER_MM`), so its printing is as soft as a found label's can be.
        scenes.push((
            format!("unmarked far {}", label.id),
            framed(label, Hand::Left, false, [0.0, 0.0, 0.0], 240.0),
        ));
    }
    for (index, pose) in tilted_poses().into_iter().enumerate() {
        scenes.push((
            format!("unmarked pose {index} {}", pose.label.id),
            pose.scene(),
        ));
    }
    for (name, scene) in scenes {
        let outcome = read(&scene, &name);
        let TapeOutcome::NotRead {
            reason: TapeRefusal::NoNewMark,
            found: Some(found),
        } = &outcome
        else {
            panic!("{name}: {}", brief(&outcome));
        };
        assert_eq!(
            find_marks(&found.crop, scene.label),
            Ok(Vec::new()),
            "{name}"
        );
    }
}

/// The outline's lowest point at printed `x`: where a vertical line there meets its bottom edge.
fn bottom_at(label: &LabelType, x: f64) -> f64 {
    let outline = label.outline_mm;
    let mut bottom = f64::INFINITY;
    for (i, &[x0, y0]) in outline.iter().enumerate() {
        let [x1, y1] = outline[(i + 1) % outline.len()];
        if (x0 <= x) != (x1 <= x) {
            bottom = bottom.min(y0 + (x - x0) * (y1 - y0) / (x1 - x0));
        }
    }
    bottom
}

#[test]
fn a_mark_the_labels_edge_cuts_is_clipped_and_keeps_the_label() {
    // Most of a mark on the label, its centre 1 mm in from the bottom edge, and clear of the band
    // the finder compares the logo with (±12 mm across), which a dark notch there would dim (P9
    // finding 10). Then the iron label's end, the shortest label's. A mark mostly off the label
    // leaves less than `MARK_MIN_AREA_MM2` inside the rim, and is no mark at all.
    let mut scenes: Vec<(String, Scene)> = labels()
        .into_iter()
        .map(|label| {
            let mut scene = framed(label, Hand::Right, true, [0.0, 0.0, 0.0], 200.0);
            scene.marks = vec![mark([17.0, bottom_at(label, 17.0) + 1.0], RADIUS_MM)];
            (format!("clipped bottom {}", label.id), scene)
        })
        .collect();
    let mut end = framed(&STRIKEPRO_IRON, Hand::Left, true, [0.0, 0.0, 0.0], 200.0);
    end.marks = vec![mark([34.0, 0.0], RADIUS_MM)];
    scenes.push(("clipped end strikepro_iron".into(), end));
    for (name, scene) in scenes {
        let outcome = read(&scene, &name);
        let TapeOutcome::NotRead {
            reason: TapeRefusal::MarkClipped,
            found: Some(found),
        } = &outcome
        else {
            panic!("{name}: {}", brief(&outcome));
        };
        let spots = find_marks(&found.crop, scene.label).unwrap();
        assert!(
            matches!(spots.as_slice(), [Spot { clipped: true, .. }]),
            "{name}: {spots:?}"
        );
    }
}

#[test]
fn two_marks_apart_on_a_fresh_label_need_a_pick_and_each_is_found_where_it_is() {
    for label in labels() {
        let mut scene = framed(label, Hand::Left, true, [6.0, -8.0, 4.0], 200.0);
        let pair = [[-12.0, 2.0], [12.0, -2.0]];
        scene.marks = pair.map(|at| mark(at, 5.0)).to_vec();
        let name = format!("two apart {}", label.id);
        let outcome = read(&scene, &name);
        let TapeOutcome::NeedsPick(TapePick { found, orientation }) = &outcome else {
            panic!("{name}: {}", brief(&outcome));
        };
        assert_eq!(*orientation, truth(&scene), "{name}");
        // Both spots are where the scene put them, for P12's diff to choose between.
        let spots = find_marks(&found.crop, scene.label).unwrap();
        assert_eq!(spots.len(), 2, "{name}: {spots:?}");
        for at in pair {
            let nearest = spots
                .iter()
                .map(|s| (s.centre_mm[0] - at[0]).hypot(s.centre_mm[1] - at[1]))
                .fold(f64::INFINITY, f64::min);
            assert!(
                nearest <= WITHIN_MM,
                "{name}: {at:?} is {nearest:.3} mm off"
            );
        }
    }
}

#[test]
fn two_marks_run_together_need_a_pick_and_a_pair_closer_than_the_gate_reads_between_them() {
    for label in labels() {
        // Centres 8 mm apart, 1.6 radii: one spot, too long to be one mark.
        let mut scene = framed(label, Hand::Right, false, [0.0, 0.0, 0.0], 200.0);
        scene.marks = vec![mark([-4.0, 0.0], 5.0), mark([4.0, 0.0], 5.0)];
        let name = format!("run together {}", label.id);
        let outcome = read(&scene, &name);
        let TapeOutcome::NeedsPick(TapePick { found, .. }) = &outcome else {
            panic!("{name}: {}", brief(&outcome));
        };
        let spots = find_marks(&found.crop, scene.label).unwrap();
        assert!(
            matches!(spots.as_slice(), [one] if one.elongation > MARK_MAX_ELONGATION),
            "{name}: {spots:?}"
        );
        eprintln!("{name}: elongation {:.3}", spots[0].elongation);

        // Centres 3 mm apart, 0.6 radii: the gate passes them as one mark, read at their middle,
        // 1.5 mm from each. That is the gate's known cost (`MARK_MAX_ELONGATION`'s doc).
        scene.marks = vec![mark([-1.5, 0.0], 5.0), mark([1.5, 0.0], 5.0)];
        let name = format!("close pair {}", label.id);
        let TapeOutcome::Read(reading) = read(&scene, &name) else {
            panic!("{name}: not read");
        };
        let [h, v] = truth(&scene).to_reading_mm([0.0, 0.0]);
        let error = (reading.marks[0].offset_h_mm - h).hypot(reading.marks[0].offset_v_mm - v);
        assert!(error <= WITHIN_MM, "{name}: {:?}", reading.marks);
    }
}

#[test]
fn a_speck_smaller_than_a_strike_is_no_mark() {
    let mut scene = framed(&STRIKEPRO_HYBRID, Hand::Right, true, [0.0, 0.0, 0.0], 200.0);
    // 2.5 mm in radius, 19.6 mm²: wider than the opening's disc, under `MARK_MIN_AREA_MM2`.
    scene.marks = vec![mark([-8.0, 3.0], 2.5)];
    let outcome = read(&scene, "speck");
    assert!(
        matches!(
            outcome,
            TapeOutcome::NotRead {
                reason: TapeRefusal::NoNewMark,
                found: Some(_)
            }
        ),
        "{}",
        brief(&outcome)
    );
}

#[test]
fn with_a_previous_crop_a_marked_label_needs_a_pick_until_p12() {
    let mut scene = framed(&STRIKEPRO_IRON, Hand::Right, true, [8.0, 6.0, -10.0], 200.0);
    scene.marks = vec![mark([5.0, 2.0], RADIUS_MM)];
    let photo = scene.draw();
    let TapeOutcome::Read(fresh) = read_tape(&photo, None, scene.label, None) else {
        panic!("the fresh label is read");
    };
    let previous = &fresh.found.crop.frame;
    // Its one mark may be the last strike's, so it is not read as new without the diff.
    assert_eq!(
        read_tape(&photo, Some(previous), scene.label, None),
        TapeOutcome::NeedsPick(TapePick {
            found: fresh.found.clone(),
            orientation: fresh.orientation,
        })
    );
    // A label with no mark at all has nothing new, whatever came before.
    scene.marks.clear();
    let unmarked = scene.draw();
    assert!(matches!(
        read_tape(&unmarked, Some(previous), scene.label, None),
        TapeOutcome::NotRead {
            reason: TapeRefusal::NoNewMark,
            found: Some(_)
        }
    ));
}

#[test]
fn a_marked_label_with_no_shaft_is_refused_for_its_orientation_before_any_mark_is_looked_for() {
    let mut scene = framed(
        &STRIKEPRO_DRIVER,
        Hand::Left,
        true,
        [5.0, 10.0, -5.0],
        200.0,
    );
    scene.shaft = Shaft::Missing;
    scene.marks = vec![mark([3.0, -2.0], RADIUS_MM)];
    let outcome = read(&scene, "marked, no shaft");
    assert!(
        matches!(
            outcome,
            TapeOutcome::NotRead {
                reason: TapeRefusal::NoOrientation,
                found: Some(_)
            }
        ),
        "{}",
        brief(&outcome)
    );
}
