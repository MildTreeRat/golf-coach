//! The newest mark on a used label, on synthetic sequences: each photo of a label on the club taken
//! after a strike and read against the last photo's crop, as the phone reads it against
//! `last/<club>.rgb`. [M46 P12]
//!
//! Each sequence is drawn by `tape_scene`, one label struck mark by mark, and each photo is taken
//! from its own pose, so the finder fits each a little differently, as it will a hand-held phone's.
//! Every reading is held to the scene's own marks, turned into the reading frame by the scene's
//! [`truth`] and never by the reader's orientation. The marks are 5 mm in radius, above the 4.5 mm
//! P11 found a whole mark needs (P11 finding 6), and at least 6.5 mm inside the outline (finding 7).

mod tape_scene;

use impact::tape::{
    diff_marks, read_tape, Change, DiffMiss, Holds, Homography, LabelType, TapeOutcome,
    TapeReading, TapeRefusal, DIFF_SEARCH_MM, STRIKEPRO_DRIVER, STRIKEPRO_HYBRID, STRIKEPRO_IRON,
};
use impact::Frame;
use tape_scene::{bottom_at, framed, keep, truth, Hand, Mark, Scene};

const RADIUS_MM: f64 = 5.0;

/// The most a read mark may lie from the scene's own, millimetres: P11's bound, which this phase
/// keeps, since a new mark is read where its spot is.
const WITHIN_MM: f64 = 0.2;

/// Each label type with the hand and the way up its run is drawn in, so that both hands and both
/// ways up are crossed.
fn labels() -> [(&'static LabelType, Hand, bool); 3] {
    [
        (&STRIKEPRO_DRIVER, Hand::Right, true),
        (&STRIKEPRO_HYBRID, Hand::Left, false),
        (&STRIKEPRO_IRON, Hand::Left, true),
    ]
}

/// Six places on each label, in the printed frame, 12.6 mm or more apart so that each mark is its
/// own spot, and in the order a run strikes them. None lies tangent to the red ring about the cross:
/// at `[1, 4]` the iron's mark ran along it and read 0.18 mm off, the ring's stroke surviving the
/// opening beside the mark (P12's finding).
fn run(label: &LabelType) -> [[f64; 2]; 6] {
    match label.id {
        "strikepro_driver" => [
            [-26.0, 4.0],
            [-13.0, -6.0],
            [0.0, 4.0],
            [13.0, -6.0],
            [26.0, 4.0],
            [-13.0, 12.0],
        ],
        "strikepro_hybrid" => [
            [-22.0, 2.0],
            [-10.0, -6.0],
            [2.0, 3.0],
            [14.0, -6.0],
            [26.0, 1.0],
            [-10.0, 7.0],
        ],
        _ => [
            [-24.0, 3.0],
            [-11.0, -5.0],
            [1.0, 3.0],
            [13.0, -5.0],
            [25.0, 2.0],
            [-12.0, 9.0],
        ],
    }
}

/// The poses photos are taken from, in turn: the camera's pitch, yaw and roll, and its distance,
/// each a little different from the last, as a hand-held phone's are.
const POSES: [([f64; 3], f64); 6] = [
    ([0.0, 0.0, 0.0], 200.0),
    ([6.0, -8.0, 4.0], 190.0),
    ([-10.0, 5.0, -6.0], 210.0),
    ([12.0, 10.0, 8.0], 195.0),
    ([-5.0, -12.0, 3.0], 205.0),
    ([8.0, 4.0, -10.0], 200.0),
];

/// A photo of `label`, framed as its run is, from pose `pose`, holding marks at `at`.
fn scene(label: &'static LabelType, pose: usize, at: &[[f64; 2]]) -> Scene {
    let (_, hand, logo_up) = labels()
        .into_iter()
        .find(|(l, _, _)| l.id == label.id)
        .expect("a label of the run");
    let (turn, distance) = POSES[pose % POSES.len()];
    let mut scene = framed(label, hand, logo_up, turn, distance);
    scene.marks = at
        .iter()
        .map(|&centre_mm| Mark {
            centre_mm,
            radius_mm: RADIUS_MM,
        })
        .collect();
    scene
}

/// Draws the scene, keeps it under `TAPE_SCENE_DIR` when that is set, and reads it against
/// `previous` with the scene's own hand.
fn read(scene: &Scene, previous: Option<&Frame>, name: &str) -> (Frame, TapeOutcome) {
    let photo = scene.draw();
    keep(&photo, name);
    let outcome = read_tape(&photo, previous, scene.label, Some(scene.hand));
    (photo, outcome)
}

/// The crop a found label leaves, the next photo's reference, as `last/<club>.rgb` holds it.
fn crop_of(outcome: &TapeOutcome) -> Frame {
    outcome.found().expect("a found label").crop.frame.clone()
}

/// An outcome without its crop's pixels, for a failure's message.
fn brief(outcome: &TapeOutcome) -> String {
    match outcome {
        TapeOutcome::Read(reading) => {
            format!("read {:?} new_label {}", reading.marks, reading.new_label)
        }
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

/// Holds a reading to the scene's marks at `expected`, printed-frame millimetres, in any order, and
/// returns the worst error.
fn reads(outcome: &TapeOutcome, scene: &Scene, expected: &[[f64; 2]], new_label: bool) -> f64 {
    let TapeOutcome::Read(TapeReading {
        orientation,
        new_label: read_new,
        marks,
        ..
    }) = outcome
    else {
        panic!("not read: {}", brief(outcome));
    };
    assert_eq!(*read_new, new_label, "{}", brief(outcome));
    assert_eq!(*orientation, truth(scene));
    assert_eq!(marks.len(), expected.len(), "{}", brief(outcome));
    let mut worst: f64 = 0.0;
    for &at in expected {
        let [h, v] = truth(scene).to_reading_mm(at);
        let error = marks
            .iter()
            .map(|m| (m.offset_h_mm - h).hypot(m.offset_v_mm - v))
            .fold(f64::INFINITY, f64::min);
        assert!(
            error <= WITHIN_MM,
            "{at:?} is {error:.3} mm off: {}",
            brief(outcome)
        );
        worst = worst.max(error);
    }
    worst
}

/// Where the printed cross lies in a found label's crop, by the scene's own projection of it put
/// through the finder's fit: the two fits' disagreement there is what the alignment must find.
fn cross_in_crop(scene: &Scene, outcome: &TapeOutcome) -> [f64; 2] {
    let found = outcome.found().expect("a found label");
    let map = Homography::from_corners(found.corners, scene.label.box_corners_mm()).unwrap();
    let mm = map.apply(scene.photo_of([0.0, 0.0])).unwrap();
    found.crop.grid.to_crop_px(mm)
}

fn diff(outcome: &TapeOutcome, previous: &Frame, label: &LabelType) -> Change {
    let found = outcome.found().expect("a found label");
    diff_marks(&found.crop, previous, label).expect("the two crops compare")
}

/// One label struck six times, a photo after each, every photo read against the last one's crop:
/// each newest mark read within [`WITHIN_MM`], the first as a fresh label's, and the alignment
/// within a pixel and a half of what the two fits disagree by at the cross. One test a label type,
/// so the three run side by side.
fn a_run_reads_each_newest_mark(label: &'static LabelType) {
    let (_, hand, logo_up) = labels()
        .into_iter()
        .find(|(l, _, _)| l.id == label.id)
        .expect("a label of the run");
    let places = run(label);
    let mut worst: f64 = 0.0;
    let mut previous: Option<(Scene, TapeOutcome)> = None;
    for strike in 0..places.len() {
        let now = scene(label, strike, &places[..=strike]);
        let name = format!("run {} strike {strike}", label.id);
        let reference = previous.as_ref().map(|(_, outcome)| crop_of(outcome));
        let (_, outcome) = read(&now, reference.as_ref(), &name);
        // The first photo is of a fresh label, and every one after holds one new mark.
        let error = reads(&outcome, &now, &[places[strike]], strike == 0);
        worst = worst.max(error);
        if let (Some((before, before_outcome)), Some(reference)) = (&previous, &reference) {
            let change = diff(&outcome, reference, label);
            let [x, y] = cross_in_crop(&now, &outcome);
            let [px, py] = cross_in_crop(before, before_outcome);
            let expected = [px - x, py - y];
            // Printed with `--nocapture`, for the plan's findings.
            eprintln!(
                "{name} ({hand:?}, logo-up {logo_up}): off {error:.3} mm, shift {:?} for the                  fits' {:.2?}, residual {:.4}, share {:.3}",
                change.shift_px, expected, change.residual, change.match_share
            );
            for k in 0..2 {
                assert!(
                    (change.shift_px[k] as f64 - expected[k]).abs() <= 1.5,
                    "{name}: shift {:?} for the fits' {expected:?}",
                    change.shift_px
                );
            }
            let new = change.spots.iter().filter(|(_, h)| *h == Holds::New);
            assert_eq!(new.count(), 1, "{name}");
            assert!(change.old.iter().all(|(_, here)| *here), "{name}");
        }
        previous = Some((now, outcome));
    }
    eprintln!("run {}: worst {worst:.3} mm", label.id);
}

#[test]
fn a_driver_label_struck_photo_by_photo_reads_each_newest_mark_within_a_fifth_of_a_millimetre() {
    a_run_reads_each_newest_mark(&STRIKEPRO_DRIVER);
}

#[test]
fn a_hybrid_label_struck_photo_by_photo_reads_each_newest_mark_within_a_fifth_of_a_millimetre() {
    a_run_reads_each_newest_mark(&STRIKEPRO_HYBRID);
}

#[test]
fn an_iron_label_struck_photo_by_photo_reads_each_newest_mark_within_a_fifth_of_a_millimetre() {
    a_run_reads_each_newest_mark(&STRIKEPRO_IRON);
}

#[test]
fn a_photo_missed_reads_both_new_marks_and_three_new_need_a_pick() {
    for (label, _, _) in labels() {
        let places = run(label);
        let before = scene(label, 0, &places[..2]);
        let (_, first) = read(&before, None, &format!("skip {} before", label.id));
        let reference = crop_of(&first);
        let two = scene(label, 2, &places[..4]);
        let (_, outcome) = read(&two, Some(&reference), &format!("skip {} two", label.id));
        let worst = reads(&outcome, &two, &places[2..4], false);
        eprintln!("two new {}: worst {worst:.3} mm", label.id);
        let three = scene(label, 4, &places[..5]);
        let (_, outcome) = read(
            &three,
            Some(&reference),
            &format!("skip {} three", label.id),
        );
        assert!(
            matches!(outcome, TapeOutcome::NeedsPick(_)),
            "{}: {}",
            label.id,
            brief(&outcome)
        );
    }
}

#[test]
fn a_new_mark_over_or_against_an_old_one_needs_a_pick() {
    for (label, _, _) in labels() {
        let places = run(label);
        let (_, first) = read(
            &scene(label, 1, &places[..2]),
            None,
            &format!("overlap before {}", label.id),
        );
        let reference = crop_of(&first);
        let old = places[1];
        // Half over the old mark, then just touching it.
        for (pose, at) in [(3, [old[0] + 4.5, old[1]]), (5, [old[0], old[1] + 9.5])] {
            let mut marks = places[..2].to_vec();
            marks.push(at);
            let name = format!("overlap {} at {at:?}", label.id).replace(['[', ']', ','], "");
            let (_, outcome) = read(&scene(label, pose, &marks), Some(&reference), &name);
            assert!(
                matches!(outcome, TapeOutcome::NeedsPick(_)),
                "{name}: {}",
                brief(&outcome)
            );
            let change = diff(&outcome, &reference, label);
            assert!(
                change.spots.iter().any(|(_, h)| *h == Holds::Both),
                "{name}: {:?}",
                change.spots
            );
        }
    }
}

#[test]
fn a_photo_with_nothing_new_has_no_new_mark_from_any_pose_or_against_itself() {
    for (label, _, _) in labels() {
        let places = run(label);
        let (photo, first) = read(
            &scene(label, 0, &places[..3]),
            None,
            &format!("nothing before {}", label.id),
        );
        let reference = crop_of(&first);
        // A retake, or a strike that missed the tape: the same marks from another pose.
        let name = format!("nothing new {}", label.id);
        let (_, outcome) = read(&scene(label, 3, &places[..3]), Some(&reference), &name);
        assert!(
            matches!(
                outcome,
                TapeOutcome::NotRead {
                    reason: TapeRefusal::NoNewMark,
                    found: Some(_)
                }
            ),
            "{name}: {}",
            brief(&outcome)
        );
        let change = diff(&outcome, &reference, label);
        assert!(change.spots.iter().all(|(_, h)| *h == Holds::Old), "{name}");
        // The very photo against its own crop, which P11 handed to a pick.
        let again = read_tape(&photo, Some(&reference), label, None);
        assert!(
            matches!(
                again,
                TapeOutcome::NotRead {
                    reason: TapeRefusal::NoNewMark,
                    ..
                }
            ),
            "{}: {}",
            label.id,
            brief(&again)
        );
        assert_eq!(diff(&again, &reference, label).shift_px, [0, 0]);
    }
}

#[test]
fn a_fresh_label_after_a_used_one_is_read_as_new_and_two_marks_on_it_need_a_pick() {
    for (label, _, _) in labels() {
        let places = run(label);
        let (_, first) = read(
            &scene(label, 0, &places[..3]),
            None,
            &format!("used label {}", label.id),
        );
        let reference = crop_of(&first);
        // A new label, stuck the other way up, its first mark 12.6 mm or more from every old one.
        let mut fresh = scene(label, 2, &places[3..4]);
        fresh.logo_up = !fresh.logo_up;
        let name = format!("fresh {}", label.id);
        let (_, outcome) = read(&fresh, Some(&reference), &name);
        reads(&outcome, &fresh, &places[3..4], true);
        let change = diff(&outcome, &reference, label);
        assert!(change.old.iter().all(|(_, here)| !here), "{name}");
        // Two marks on a new label: which is new since the last photo is the golfer's to say (P2
        // finding 3: a skipped photo is never a label's first).
        let mut two = scene(label, 4, &places[3..5]);
        two.logo_up = !two.logo_up;
        let (_, outcome) = read(&two, Some(&reference), &format!("fresh two {}", label.id));
        assert!(
            matches!(outcome, TapeOutcome::NeedsPick(_)),
            "{}",
            brief(&outcome)
        );
        // Its one mark where an old one stood: the same label, struck again, could look so.
        let mut taken = scene(label, 4, &[[places[0][0] + 1.0, places[0][1]]]);
        taken.logo_up = !taken.logo_up;
        let (_, outcome) = read(
            &taken,
            Some(&reference),
            &format!("fresh taken {}", label.id),
        );
        assert!(
            matches!(outcome, TapeOutcome::NeedsPick(_)),
            "{}",
            brief(&outcome)
        );
    }
}

#[test]
fn a_new_mark_the_labels_edge_cuts_is_clipped() {
    for (label, _, _) in labels() {
        let places = run(label);
        let (_, first) = read(
            &scene(label, 0, &places[..2]),
            None,
            &format!("clip before {}", label.id),
        );
        let reference = crop_of(&first);
        // Its centre 1 mm in from the bottom edge, clear of both old marks and of the band the
        // finder compares the logo with (P11's clipped test).
        let mut marks = places[..2].to_vec();
        marks.push([17.0, bottom_at(label, 17.0) + 1.0]);
        let name = format!("clipped new {}", label.id);
        let (_, outcome) = read(&scene(label, 3, &marks), Some(&reference), &name);
        assert!(
            matches!(
                outcome,
                TapeOutcome::NotRead {
                    reason: TapeRefusal::MarkClipped,
                    found: Some(_)
                }
            ),
            "{name}: {}",
            brief(&outcome)
        );
    }
}

/// `frame` moved by `by` pixels, black where nothing moved in: a pixel at `p` there is at `p + by`
/// here.
fn moved(frame: &Frame, by: [i64; 2]) -> Frame {
    let mut rgb = vec![0; frame.rgb.len()];
    for y in 0..frame.height as i64 {
        for x in 0..frame.width as i64 {
            let (fx, fy) = (x - by[0], y - by[1]);
            if (0..frame.width as i64).contains(&fx) && (0..frame.height as i64).contains(&fy) {
                let from = (fy as usize * frame.width + fx as usize) * 3;
                let to = (y as usize * frame.width + x as usize) * 3;
                rgb[to..to + 3].copy_from_slice(&frame.rgb[from..from + 3]);
            }
        }
    }
    Frame { rgb, ..*frame }
}

#[test]
fn the_alignment_follows_a_reference_moved_in_reach_and_refuses_one_moved_beyond_it() {
    let label = &STRIKEPRO_IRON;
    let places = run(label);
    let (_, first) = read(&scene(label, 0, &places[..2]), None, "moved before");
    let reference = crop_of(&first);
    let now = scene(label, 2, &places[..3]);
    let (photo, outcome) = read(&now, Some(&reference), "moved now");
    reads(&outcome, &now, &places[2..3], false);
    let found = outcome.found().unwrap();
    let base = diff_marks(&found.crop, &reference, label).unwrap().shift_px;
    let by = [13, -7];
    let change = diff_marks(&found.crop, &moved(&reference, by), label).unwrap();
    assert_eq!(change.shift_px, [base[0] + by[0], base[1] + by[1]]);
    assert_eq!(
        read_tape(&photo, Some(&moved(&reference, by)), label, None),
        outcome
    );
    // 2.5 mm, beyond the search's 2 mm: no shift in reach lines the printing up, and a marked
    // label is the golfer's to pick rather than diffed at a guess.
    let reach = (DIFF_SEARCH_MM * found.crop.grid.px_per_mm) as i64;
    let beyond = moved(&reference, [reach + 5 - base[0], 0]);
    let refused = diff_marks(&found.crop, &beyond, label);
    eprintln!("moved beyond the reach: {refused:?}");
    assert_eq!(refused, Err(DiffMiss::Unaligned));
    assert!(matches!(
        read_tape(&photo, Some(&beyond), label, None),
        TapeOutcome::NeedsPick(_)
    ));
}

#[test]
fn a_reference_with_black_outside_or_of_another_label_type() {
    let label = &STRIKEPRO_HYBRID;
    let places = run(label);
    let (_, first) = read(&scene(label, 0, &places[..2]), None, "outside before");
    let reference = crop_of(&first);
    let now = scene(label, 3, &places[..3]);
    let (photo, outcome) = read(&now, Some(&reference), "outside now");
    // The top fifth of the reference held nothing, as when the label ran off the last photo:
    // black, which is left out of the diff. Every mark is clear of it, the logo is not.
    let mut part = reference.clone();
    let rows = part.height / 5;
    part.rgb[..rows * part.width * 3].fill(0);
    assert_eq!(
        read_tape(&photo, Some(&part), label, Some(now.hand)),
        outcome
    );
    // A reference of another label type is another grid: nothing to diff, so a pick.
    let (_, iron) = read(&scene(&STRIKEPRO_IRON, 0, &[[0.0, 0.0]]), None, "iron");
    let other = crop_of(&iron);
    let found = outcome.found().unwrap();
    assert_eq!(
        diff_marks(&found.crop, &other, label),
        Err(DiffMiss::OtherGrid)
    );
    assert!(matches!(
        read_tape(&photo, Some(&other), label, Some(now.hand)),
        TapeOutcome::NeedsPick(_)
    ));
}
