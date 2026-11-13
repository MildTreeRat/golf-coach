//! The label finder, on synthetic photos of a label stuck on a club. [M46 P9]
//!
//! Each photo is drawn by `tape_scene` through a pinhole camera, so where the label's box corners
//! land is the camera's answer, and the finder never sees it. A found label is judged by how far its
//! map puts every outline vertex, and the cross, from where the camera put them, in label
//! millimetres. No photo of a label on a club exists before P29 (P3 finding 10), so these are the
//! finder's only tests until then.

mod tape_scene;

use impact::tape::{
    find_label, read_tape, FoundLabel, Homography, LabelMiss, LabelType, OutlineFit, TapeOutcome,
    TapeRefusal, STRIKEPRO_DRIVER, STRIKEPRO_HYBRID, STRIKEPRO_IRON,
};
use tape_scene::{keep, Camera, Card, Hand, Head, Mark, Scene};

/// How far from the camera's truth a found label may put any point of its outline, millimetres.
const FOUND_WITHIN_MM: f64 = 0.25;

/// The worst distance, millimetres, between where the found map puts each outline vertex and the
/// cross, and where they are.
fn worst_miss_mm(scene: &Scene, corners: [[f64; 2]; 4]) -> f64 {
    let map = Homography::from_corners(corners, scene.label.box_corners_mm())
        .expect("a found label's corners make a map");
    scene
        .label
        .outline_mm
        .iter()
        .chain(&[[0.0, 0.0]])
        .map(|&printed| {
            let found = map
                .apply(scene.photo_of(printed))
                .expect("on the label's side of the horizon");
            (found[0] - printed[0]).hypot(found[1] - printed[1])
        })
        .fold(0.0, f64::max)
}

/// Draws the scene, finds its label, and holds the result to the camera's truth.
fn found(scene: &Scene, name: &str) -> (FoundLabel, OutlineFit) {
    let photo = scene.draw();
    keep(&photo, name);
    let (found, fit) = find_label(&photo, scene.label)
        .unwrap_or_else(|miss| panic!("{name}: no label found ({miss:?})"));
    let miss = worst_miss_mm(scene, found.corners);
    // Printed with `--nocapture`, for the plan's findings.
    eprintln!("{name}: {miss:.3} mm off at worst, {fit:?}");
    assert!(
        miss <= FOUND_WITHIN_MM,
        "{name}: the found label is {miss:.3} mm off at worst ({fit:?})"
    );
    assert_eq!(
        (found.crop.grid.width, found.crop.grid.height),
        (
            (scene.label.width_mm * 10.0).round() as usize,
            (scene.label.height_mm * 10.0).round() as usize
        ),
        "{name}: the crop is the label's box"
    );
    (found, fit)
}

/// The mean luminance of a band of the crop, `from_mm` to `to_mm` down from its top edge, over its
/// middle third across.
fn band_luminance(crop: &impact::tape::Crop, from_mm: f64, to_mm: f64) -> f64 {
    let grid = crop.grid;
    let (rows, columns) = (
        (from_mm * grid.px_per_mm) as usize..(to_mm * grid.px_per_mm) as usize,
        grid.width / 3..grid.width * 2 / 3,
    );
    let mut total = (0.0, 0usize);
    for row in rows {
        for column in columns.clone() {
            let at = (row * grid.width + column) * 3;
            let [r, g, b] = [0, 1, 2].map(|k| f64::from(crop.frame.rgb[at + k]));
            total.0 += 0.299 * r + 0.587 * g + 0.114 * b;
            total.1 += 1;
        }
    }
    total.0 / total.1 as f64
}

fn labels() -> [&'static LabelType; 3] {
    [&STRIKEPRO_DRIVER, &STRIKEPRO_HYBRID, &STRIKEPRO_IRON]
}

#[test]
fn each_label_type_is_found_face_on_where_the_camera_put_it() {
    for label in labels() {
        let scene = Scene::new(label, Camera::face_on(160.0));
        found(&scene, &format!("face-on {}", label.id));
    }
}

#[test]
fn a_label_up_to_thirty_degrees_off_square_and_turned_any_way_is_found() {
    let poses = [
        (
            &STRIKEPRO_DRIVER,
            Camera::new(30.0, 0.0, 15.0, 180.0, [-4.0, 6.0]),
        ),
        (
            &STRIKEPRO_DRIVER,
            Camera::new(-12.0, 26.0, 180.0, 185.0, [3.0, 0.0]),
        ),
        (
            &STRIKEPRO_HYBRID,
            Camera::new(0.0, -30.0, -40.0, 150.0, [4.0, -3.0]),
        ),
        (
            &STRIKEPRO_HYBRID,
            Camera::new(10.0, 28.0, -90.0, 150.0, [-3.0, 5.0]),
        ),
        (
            &STRIKEPRO_IRON,
            Camera::new(20.0, 21.0, 120.0, 160.0, [0.0, 0.0]),
        ),
        (
            &STRIKEPRO_IRON,
            Camera::new(-25.0, 15.0, 65.0, 170.0, [6.0, 4.0]),
        ),
    ];
    for (index, (label, camera)) in poses.into_iter().enumerate() {
        let tilt = camera.tilt_deg();
        assert!(tilt <= 30.0 + 1e-9, "pose {index} is {tilt:.1}° off square");
        let (_, fit) = found(
            &Scene::new(label, camera),
            &format!("pose {index} {}", label.id),
        );
        assert!(fit.foreshortening > 1.0);
    }
}

#[test]
fn a_left_handed_club_and_a_label_stuck_logo_down_are_found_in_the_printed_frame() {
    for label in labels() {
        for (hand, logo_up) in [
            (Hand::Left, true),
            (Hand::Right, false),
            (Hand::Left, false),
        ] {
            let mut scene = Scene::new(label, Camera::new(8.0, -14.0, 10.0, 165.0, [0.0, 0.0]));
            scene.hand = hand;
            scene.logo_up = logo_up;
            let name = format!("{hand:?} logo-up {logo_up} {}", label.id);
            // The crop is cut logo-up whichever way the label lay: the logo's band along its top is
            // darker than a band as deep along its bottom.
            let (found, _) = found(&scene, &name);
            let height = label.height_mm;
            let (top, bottom) = (
                band_luminance(&found.crop, 1.0, 4.5),
                band_luminance(&found.crop, height - 4.5, height - 1.0),
            );
            assert!(
                top + 10.0 < bottom,
                "{name}: top {top:.0}, bottom {bottom:.0}"
            );
        }
    }
}

#[test]
fn marks_on_the_label_and_its_edge_and_a_logo_reaching_the_edge_do_not_lose_it() {
    for label in labels() {
        let mut scene = Scene::new(label, Camera::new(-15.0, 20.0, 30.0, 165.0, [0.0, 0.0]));
        let r = label.inscribed_radius_mm;
        scene.marks = vec![
            Mark {
                centre_mm: [0.0, 0.0],
                radius_mm: 3.0,
            },
            Mark {
                centre_mm: [6.0, -3.0],
                radius_mm: 3.5,
            },
            Mark {
                centre_mm: [-9.0, 2.0],
                radius_mm: 3.0,
            },
            Mark {
                centre_mm: [12.0, 5.0],
                radius_mm: 2.5,
            },
            Mark {
                centre_mm: [-4.0, -8.0],
                radius_mm: 3.0,
            },
            Mark {
                centre_mm: [2.0, 9.0],
                radius_mm: 2.5,
            },
            // Two strikes off the bottom edge, clipped by it: notches in the label's white.
            Mark {
                centre_mm: [8.0, -r - 1.0],
                radius_mm: 3.5,
            },
            Mark {
                centre_mm: [-14.0, -r - 0.5],
                radius_mm: 3.0,
            },
        ];
        // The logo's swoosh crossing the top edge, so the white has a notch there too, as P3's
        // blurred outline did (P3 finding 7).
        scene.logo_gap_mm = -0.3;
        let (_, fit) = found(&scene, &format!("marked {}", label.id));
        assert!(fit.inlier_share < 1.0, "the notches are not on the outline");
    }
}

#[test]
fn a_dark_head_and_a_steel_one_each_hold_any_label() {
    for label in labels() {
        for head in [Head::Iron, Head::Driver, Head::None] {
            let mut scene = Scene::new(label, Camera::new(5.0, 10.0, -5.0, 170.0, [0.0, 0.0]));
            scene.head = head;
            found(&scene, &format!("{head:?} head {}", label.id));
        }
    }
}

#[test]
fn a_photo_with_no_label_in_it_finds_none() {
    // On a dark driver there is nothing bright at all. A steel iron's face is bright and neutral,
    // and about as long as a label is tall, so it is a candidate that the outline then refuses.
    for (head, furthest) in [
        (Head::Driver, LabelMiss::NoCandidate),
        (Head::Iron, LabelMiss::OutlineMisfit),
    ] {
        let mut scene = Scene::new(&STRIKEPRO_IRON, Camera::face_on(160.0));
        scene.head = head;
        scene.no_label = true;
        let photo = scene.draw();
        keep(&photo, &format!("no label {head:?}"));
        for label in labels() {
            let answer = find_label(&photo, label).map(|(_, fit)| fit);
            assert!(
                answer == Err(furthest) || answer == Err(LabelMiss::NoCandidate),
                "{head:?} {}: {answer:?}",
                label.id
            );
        }
    }
}

#[test]
fn a_white_card_of_the_labels_size_or_a_disc_is_not_a_label() {
    // The iron label's box as a plain white card: its square top corners are where the label is
    // shouldered.
    let mut card = Scene::new(
        &STRIKEPRO_IRON,
        Camera::new(5.0, 10.0, 0.0, 160.0, [0.0, 0.0]),
    );
    card.card = Some(Card::Rectangle {
        width_mm: STRIKEPRO_IRON.width_mm,
        height_mm: STRIKEPRO_IRON.height_mm,
    });
    let photo = card.draw();
    keep(&photo, "card");
    assert_eq!(
        find_label(&photo, &STRIKEPRO_IRON).map(|(_, fit)| fit),
        Err(LabelMiss::OutlineMisfit)
    );
    let mut disc = Scene::new(&STRIKEPRO_DRIVER, Camera::face_on(160.0));
    disc.card = Some(Card::Disc { radius_mm: 30.0 });
    let photo = disc.draw();
    keep(&photo, "disc");
    for label in labels() {
        assert_eq!(
            find_label(&photo, label).map(|(_, fit)| fit),
            Err(LabelMiss::WrongProportions),
            "{}",
            label.id
        );
    }
}

#[test]
fn a_label_read_as_another_type_is_refused() {
    // P8 finding 4: the driver label's aspect and the iron label's differ by 0.3%, so only the
    // outline tells them apart.
    for (drawn, asked) in [
        (&STRIKEPRO_DRIVER, &STRIKEPRO_IRON),
        (&STRIKEPRO_IRON, &STRIKEPRO_DRIVER),
        (&STRIKEPRO_HYBRID, &STRIKEPRO_IRON),
        (&STRIKEPRO_IRON, &STRIKEPRO_HYBRID),
    ] {
        let scene = Scene::new(drawn, Camera::new(6.0, -8.0, 4.0, 165.0, [0.0, 0.0]));
        let photo = scene.draw();
        let answer = find_label(&photo, asked).map(|(_, fit)| fit);
        assert!(
            matches!(
                answer,
                Err(LabelMiss::OutlineMisfit | LabelMiss::WrongProportions)
            ),
            "{} read as {}: {answer:?}",
            drawn.id,
            asked.id
        );
    }
}

#[test]
fn a_label_cut_by_the_photos_edge_or_too_far_to_read_is_refused() {
    // Moved until its toe end leaves the photo.
    let mut cut = Scene::new(
        &STRIKEPRO_IRON,
        Camera::new(0.0, 0.0, 0.0, 150.0, [-40.0, 0.0]),
    );
    cut.head = Head::Driver;
    let photo = cut.draw();
    keep(&photo, "cut");
    assert_eq!(
        find_label(&photo, &STRIKEPRO_IRON).map(|(_, fit)| fit),
        Err(LabelMiss::NoCandidate)
    );
    // At 400 mm a millimetre of label is 2.5 pixels, under the finder's floor.
    let far = Scene::new(&STRIKEPRO_DRIVER, Camera::face_on(400.0));
    let photo = far.draw();
    keep(&photo, "far");
    assert!(find_label(&photo, &STRIKEPRO_DRIVER).is_err());
}

#[test]
fn two_labels_of_one_type_in_one_photo_are_ambiguous() {
    let mut scene = Scene::new(&STRIKEPRO_HYBRID, Camera::face_on(200.0));
    scene.head = Head::None;
    scene.label_at_mm = [0.0, 24.0];
    scene.second_label_at_mm = Some([0.0, -24.0]);
    let photo = scene.draw();
    keep(&photo, "two");
    assert_eq!(
        find_label(&photo, &STRIKEPRO_HYBRID).map(|(_, fit)| fit),
        Err(LabelMiss::Ambiguous)
    );
}

#[test]
fn read_tape_answers_no_label_for_a_photo_the_finder_finds_none_in() {
    // What `read_tape` does with a label it finds is P10's, in `tests/tape_orient.rs`.
    let mut empty = Scene::new(&STRIKEPRO_IRON, Camera::face_on(160.0));
    empty.no_label = true;
    assert_eq!(
        read_tape(&empty.draw(), None, &STRIKEPRO_IRON, None),
        TapeOutcome::NotRead {
            reason: TapeRefusal::NoLabel,
            found: None,
        }
    );
}
