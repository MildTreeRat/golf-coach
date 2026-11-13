//! The orientation, on synthetic photos of a label stuck on a club with its hosel and shaft in
//! frame. [M46 P10]
//!
//! Each photo is drawn by `tape_scene`, and which way the club lies is the scene's own answer, worked
//! out here from its hand and how the label was stuck, never from the reader. The reader finds the
//! label (P9) and then the shaft, and must give that answer or refuse. No photo of a label on a club
//! exists before P29 (P3 finding 10), so these are the orientation's only tests until then.

mod tape_scene;

use impact::tape::{
    find_label, orient, read_tape, FoundLabel, LabelType, OrientMiss, Orientation, Shaft as Found,
    TapeOutcome, TapePick, TapeRefusal, STRIKEPRO_DRIVER, STRIKEPRO_HYBRID, STRIKEPRO_IRON,
};
use tape_scene::{keep, Camera, Hand, Head, Scene, Shaft};

fn labels() -> [&'static LabelType; 3] {
    [&STRIKEPRO_DRIVER, &STRIKEPRO_HYBRID, &STRIKEPRO_IRON]
}

/// Which way the scene's club lies across its label, from the scene alone. In the face's plane the
/// toe is toward `−x` on a right-handed club and `+x` on a left-handed one (C4), and the top line is
/// `+y`. A label stuck logo-down has its printed frame turned half a turn from the plane's.
fn truth(scene: &Scene) -> Orientation {
    let toe = match scene.hand {
        Hand::Right => -1.0,
        Hand::Left => 1.0,
    };
    let turn = if scene.logo_up { 1.0 } else { -1.0 };
    Orientation {
        toe_printed_right: toe * turn > 0.0,
        top_line_printed_top: turn > 0.0,
    }
}

/// A scene of `label` on the head its type suits, the camera aimed beside the label toward the
/// heel and up, so the hosel and some shaft are in frame, as the tape camera's frame will ask.
fn framed(
    label: &'static LabelType,
    hand: Hand,
    logo_up: bool,
    [pitch, yaw, roll]: [f64; 3],
    distance_mm: f64,
) -> Scene {
    let heel = match hand {
        Hand::Right => 1.0,
        Hand::Left => -1.0,
    };
    let camera = Camera::aimed(pitch, yaw, roll, distance_mm, [heel * 12.0, 15.0]);
    let mut scene = Scene::new(label, camera);
    scene.hand = hand;
    scene.logo_up = logo_up;
    scene
}

/// Draws the scene, finds its label, and orients it with `hand` as the golfer's.
fn oriented(
    scene: &Scene,
    name: &str,
    hand: Option<Hand>,
) -> (
    impact::Frame,
    FoundLabel,
    Result<(Orientation, Found), OrientMiss>,
) {
    let photo = scene.draw();
    keep(&photo, name);
    let (found, _) = find_label(&photo, scene.label)
        .unwrap_or_else(|miss| panic!("{name}: no label found ({miss:?})"));
    let answer = orient(&photo, &found, scene.label, hand);
    // Printed with `--nocapture`, for the plan's findings.
    eprintln!("{name}: {answer:?}");
    (photo, found, answer)
}

/// Holds an oriented scene to its truth, with no hand and with the scene's own.
fn holds_its_truth(scene: &Scene, name: &str) -> Found {
    let (photo, found, answer) = oriented(scene, name, None);
    let (orientation, shaft) =
        answer.unwrap_or_else(|miss| panic!("{name}: not oriented ({miss:?})"));
    assert_eq!(orientation, truth(scene), "{name}");
    assert_eq!(orientation.handedness(), scene.hand, "{name}");
    assert_eq!(
        orient(&photo, &found, scene.label, Some(scene.hand)).map(|(o, _)| o),
        Ok(orientation),
        "{name}: the golfer's own hand agrees"
    );
    shaft
}

#[test]
fn each_hand_either_way_up_is_oriented_by_its_shaft_on_every_label() {
    for label in labels() {
        for hand in [Hand::Right, Hand::Left] {
            for logo_up in [true, false] {
                let scene = framed(label, hand, logo_up, [0.0, 0.0, 0.0], 200.0);
                let name = format!("{hand:?} logo-up {logo_up} {}", label.id);
                holds_its_truth(&scene, &name);
            }
        }
    }
}

#[test]
fn a_club_up_to_thirty_degrees_off_square_and_turned_any_way_is_oriented() {
    // Each pose's aim, a plane point at the photo's centre, keeps the whole label and some shaft in
    // the 800 × 600 photo once the camera is turned. Pose 2's yaw and roll swing the shaft toward
    // the photo's top edge, so it is aimed further along the shaft than the rest.
    let poses = [
        (
            &STRIKEPRO_DRIVER,
            Hand::Right,
            true,
            [25.0, -15.0, 90.0],
            [12.0, 15.0],
        ),
        (
            &STRIKEPRO_DRIVER,
            Hand::Left,
            false,
            [-12.0, 26.0, 180.0],
            [-12.0, 15.0],
        ),
        (
            &STRIKEPRO_HYBRID,
            Hand::Left,
            true,
            [0.0, -30.0, -40.0],
            [-22.0, 26.0],
        ),
        (
            &STRIKEPRO_HYBRID,
            Hand::Right,
            false,
            [10.0, 28.0, -90.0],
            [12.0, 15.0],
        ),
        (
            &STRIKEPRO_IRON,
            Hand::Right,
            true,
            [20.0, 21.0, 120.0],
            [12.0, 15.0],
        ),
        (
            &STRIKEPRO_IRON,
            Hand::Left,
            true,
            [-25.0, 15.0, 65.0],
            [-12.0, 15.0],
        ),
    ];
    for (index, (label, hand, logo_up, [pitch, yaw, roll], aim)) in poses.into_iter().enumerate() {
        let mut scene = framed(label, hand, logo_up, [pitch, yaw, roll], 200.0);
        scene.camera = Camera::aimed(pitch, yaw, roll, 200.0, aim);
        let tilt = scene.camera.tilt_deg();
        assert!(tilt <= 30.0 + 1e-9, "pose {index} is {tilt:.1}° off square");
        holds_its_truth(&scene, &format!("pose {index} {hand:?} {}", label.id));
    }
}

#[test]
fn the_shaft_found_is_the_one_the_scene_drew() {
    // Face-on, the shaft's drawn angle is the lie, since the renderer's shaft lies in the face.
    for label in labels() {
        let scene = framed(label, Hand::Right, true, [0.0, 0.0, 0.0], 200.0);
        let shaft = holds_its_truth(&scene, &format!("drawn {}", label.id));
        assert!(
            (shaft.angle_deg - 62.0).abs() <= 2.0,
            "{}: {shaft:?}",
            label.id
        );
        // Both ends lie on the line the scene drew the shaft along, from the hosel's base at the
        // head's own place (`tape_scene`'s `IRON_FACE` and `DRIVER_FACE`).
        let base = if label.id == "strikepro_driver" {
            [52.0, 18.0]
        } else {
            [40.0, 12.0]
        };
        let (s, c) = 62f64.to_radians().sin_cos();
        for end in [shaft.near_mm, shaft.far_mm] {
            let across = ((end[0] - base[0]) * s - (end[1] - base[1]) * c).abs();
            assert!(across <= 2.0, "{}: {end:?} is {across:.1} mm off", label.id);
        }
        assert!(
            shaft.length_mm >= 25.0 && shaft.width_mm <= 16.0,
            "{shaft:?}"
        );
    }
}

#[test]
fn a_hand_that_disagrees_with_the_shaft_is_refused_and_keeps_the_label() {
    for logo_up in [true, false] {
        let scene = framed(
            &STRIKEPRO_IRON,
            Hand::Right,
            logo_up,
            [6.0, -8.0, 4.0],
            200.0,
        );
        let name = format!("left golfer, right club, logo-up {logo_up}");
        let (photo, found, answer) = oriented(&scene, &name, Some(Hand::Left));
        assert_eq!(
            answer.map(|(o, _)| o),
            Err(OrientMiss::HandDisagrees),
            "{name}"
        );
        assert_eq!(
            read_tape(&photo, None, scene.label, Some(Hand::Left)),
            TapeOutcome::NotRead {
                reason: TapeRefusal::NoOrientation,
                found: Some(found),
            },
            "{name}"
        );
    }
}

#[test]
fn a_photo_with_no_shaft_in_it_is_not_oriented() {
    let mut scenes = Vec::new();
    for label in [&STRIKEPRO_DRIVER, &STRIKEPRO_IRON] {
        let mut scene = framed(label, Hand::Right, true, [5.0, 10.0, -5.0], 200.0);
        scene.shaft = Shaft::Missing;
        scenes.push((format!("no shaft {}", label.id), scene));
    }
    // The label on the mat, with no club at all.
    let mut mat = framed(&STRIKEPRO_HYBRID, Hand::Left, true, [0.0, 0.0, 0.0], 200.0);
    mat.head = Head::None;
    scenes.push(("on the mat".into(), mat));
    // Framed on the label alone, close in, so the shaft is out of the photo: what the golfer is
    // asked to take again with the hosel and some of the shaft in frame.
    scenes.push((
        "framed too tight".into(),
        Scene::new(&STRIKEPRO_IRON, Camera::face_on(160.0)),
    ));
    for (name, scene) in scenes {
        let (photo, found, answer) = oriented(&scene, &name, None);
        assert_eq!(answer.map(|(o, _)| o), Err(OrientMiss::NoShaft), "{name}");
        assert_eq!(
            read_tape(&photo, None, scene.label, Some(scene.hand)),
            TapeOutcome::NotRead {
                reason: TapeRefusal::NoOrientation,
                found: Some(found),
            },
            "{name}"
        );
    }
}

#[test]
fn a_shaft_against_a_dark_background_is_refused_or_read_right_and_never_flipped() {
    // A black floor or wall behind the club hides a dark shaft, and leaves dark slivers where the
    // background meets the head. Whatever the reader makes of that, it must not be the other side.
    for (label, head) in [
        (&STRIKEPRO_IRON, Head::Iron),
        (&STRIKEPRO_DRIVER, Head::Driver),
    ] {
        for hand in [Hand::Right, Hand::Left] {
            let mut scene = framed(label, hand, true, [5.0, -10.0, 15.0], 200.0);
            scene.head = head;
            scene.background = [38, 38, 42];
            let name = format!("dark background {hand:?} {}", label.id);
            let (_, _, answer) = oriented(&scene, &name, None);
            match answer {
                Ok((orientation, _)) => assert_eq!(orientation, truth(&scene), "{name}"),
                Err(miss) => assert_eq!(miss, OrientMiss::NoShaft, "{name}"),
            }
        }
    }
}

#[test]
fn shafts_leaving_both_ends_are_ambiguous() {
    for label in [&STRIKEPRO_DRIVER, &STRIKEPRO_IRON] {
        let mut scene = framed(label, Hand::Right, true, [0.0, 0.0, 0.0], 230.0);
        // Aimed at the head's middle, so the bar at each end is in frame.
        scene.camera = Camera::aimed(0.0, 0.0, 0.0, 230.0, [0.0, 15.0]);
        scene.shaft = Shaft::AtBothEnds;
        let name = format!("both ends {}", label.id);
        let (_, _, answer) = oriented(&scene, &name, None);
        assert_eq!(answer.map(|(o, _)| o), Err(OrientMiss::Ambiguous), "{name}");
    }
}

#[test]
fn read_tape_hands_an_oriented_label_to_the_golfer_to_pick_until_p11() {
    let scene = framed(
        &STRIKEPRO_HYBRID,
        Hand::Left,
        false,
        [10.0, 12.0, 20.0],
        200.0,
    );
    let photo = scene.draw();
    let (found, _) = find_label(&photo, scene.label).expect("the label is found");
    let picked = TapeOutcome::NeedsPick(TapePick {
        found: found.clone(),
        orientation: truth(&scene),
    });
    for hand in [None, Some(Hand::Left)] {
        assert_eq!(read_tape(&photo, None, scene.label, hand), picked);
    }
    // A previous crop changes nothing before P12.
    assert_eq!(
        read_tape(&photo, Some(&found.crop.frame), scene.label, None),
        picked
    );
}
