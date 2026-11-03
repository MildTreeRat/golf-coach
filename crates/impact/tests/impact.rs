//! The whole reader on frames drawn here: call 12 of the M42 plan, from the ball to the offset,
//! held from outside. [M42 P12]
//!
//! Every clip is the same strike drawn noise-free on a dark green field: a ball of `D` = 24 px at
//! rest in frames 0 to 5 and gone after; a light head bar 60 × 10 px with its long axis at 30°,
//! coming at the ball along the face's normal one ball diameter a frame; and a dark shaft bar
//! 120 × 6 px leaving the head's heel end along the head's axis. In frame 6, one past the ball's
//! last, the head lies over the spot the ball left, which is the contact frame call 12 looks one
//! frame past the ball for. Each test changes one thing about that strike.
//!
//! The contact frame's head is drawn centred on the half-pixel grid, so the bar is point-symmetric
//! about its centre and the midpoint the reader measures is that centre exactly: the offset is
//! then held to 0.5 mm (0.28 px at this ball) with no rasterization error inside it.

use impact::{
    read_impact, Frame, ImpactReading, ImpactRefusal, BALL_DIAMETER_MM, BALL_MAX_FRAC,
    BALL_MIN_FRAC, CONTACT_MAX_DISTANCE, HEAD_MAX_LENGTH, HEAD_MIN_ASPECT, HEAD_MIN_LENGTH,
    SHAFT_MAX_JOIN,
};

/// 960 × 720, because the ball (24 px) stays inside the size bracket at every tuning P13 may make
/// only on a shorter side in `[24 / 0.04, 24 / 0.02]` = `[600, 1200]` (P11's finding).
const WIDTH: usize = 960;
const HEIGHT: usize = 720;
const FIELD: [u8; 3] = [60, 80, 50];
const WHITE: [u8; 3] = [255, 255, 255];
/// Light at every tuning P13 may make (`LIGHT_MIN` up to 190, `LIGHT_SPREAD` down to 40).
const HEAD_RGB: [u8; 3] = [230, 230, 230];
/// Dark at every tuning: luminance 20 against the field's ≈ 70.6, under 0.4 × it at the bracket's
/// strictest `DARK_RATIO`, while the field stays above 0.8 × itself at the loosest.
const SHAFT_RGB: [u8; 3] = [20, 20, 20];

const D: f64 = 24.0;
/// On a pixel corner, so the disc is point-symmetric and its centroid is this exactly.
const BALL: [f64; 2] = [480.0, 360.0];
const HEAD_LENGTH: f64 = 60.0;
const HEAD_WIDTH: f64 = 10.0;
const SHAFT_LENGTH: f64 = 120.0;
const SHAFT_WIDTH: f64 = 6.0;
/// The ball's last frame; the contact frame is the one after it.
const BALL_LAST: usize = 5;
const CONTACT: usize = BALL_LAST + 1;
const FRAMES: usize = 9;
/// The first frame the head is drawn in.
const HEAD_FIRST: usize = 2;
/// How far the head travels a frame, pixels, along the face's normal: one ball diameter, so in the
/// ball's last frame the face is ≈ 7 px from the ball's edge, clear of it.
const STEP: f64 = D;

/// The head's long axis, at 30°, pointing from the heel toward the toe.
fn toe_ward() -> [f64; 2] {
    let angle = 30f64.to_radians();
    [angle.cos(), angle.sin()]
}

/// The face's normal: the head travels along it toward the ball.
fn forward() -> [f64; 2] {
    let [x, y] = toe_ward();
    [-y, x]
}

fn dot(a: [f64; 2], b: [f64; 2]) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}

fn moved(point: [f64; 2], along: [f64; 2], by: f64) -> [f64; 2] {
    [point[0] + by * along[0], point[1] + by * along[1]]
}

/// Millimetres at the ball as pixels.
fn px(mm: f64) -> f64 {
    mm * D / BALL_DIAMETER_MM
}

/// The contact head's centre for a ball `along_mm` toward the toe of the bar's midpoint: the point
/// of the half-pixel grid within a pixel of the head's axis that comes nearest it. The search is
/// a few grid steps wide, and the projection it lands on is within a fraction of a pixel of the
/// one asked for (each test checks how near).
fn contact_centre(along_mm: f64) -> [f64; 2] {
    let ideal = moved(BALL, toe_ward(), -px(along_mm));
    let snap = |v: f64| (v * 2.0).round() / 2.0;
    let mut best: Option<(f64, [f64; 2])> = None;
    for i in -4..=4 {
        for j in -4..=4 {
            let centre = [
                snap(ideal[0]) + f64::from(i) * 0.5,
                snap(ideal[1]) + f64::from(j) * 0.5,
            ];
            let from = [BALL[0] - centre[0], BALL[1] - centre[1]];
            if dot(from, forward()).abs() > 1.0 {
                continue;
            }
            let error = (dot(from, toe_ward()) - px(along_mm)).abs();
            if best.is_none_or(|(least, _)| error < least) {
                best = Some((error, centre));
            }
        }
    }
    best.expect("a grid point near the axis").1
}

/// The offset a ball would read against a head centred on `centre`, millimetres, unrounded.
fn drawn_offset_mm(centre: [f64; 2]) -> f64 {
    let from = [BALL[0] - centre[0], BALL[1] - centre[1]];
    dot(from, toe_ward()) * BALL_DIAMETER_MM / D
}

fn field() -> Frame {
    Frame {
        width: WIDTH,
        height: HEIGHT,
        rgb: FIELD.repeat(WIDTH * HEIGHT),
    }
}

fn set(frame: &mut Frame, x: usize, y: usize, rgb: [u8; 3]) {
    let at = (y * frame.width + x) * 3;
    frame.rgb[at..at + 3].copy_from_slice(&rgb);
}

/// Paints the pixels whose centres lie within `reach` of `center` on both axes of the box test
/// `inside`, which gets each centre's offset from `center`.
fn paint(
    frame: &mut Frame,
    center: [f64; 2],
    reach: f64,
    rgb: [u8; 3],
    inside: impl Fn(f64, f64) -> bool,
) {
    let span = |c: f64, limit: usize| {
        let low = (c - reach).floor().max(0.0) as usize;
        let high = ((c + reach).ceil().max(0.0) as usize).min(limit);
        low..high
    };
    for y in span(center[1], frame.height) {
        for x in span(center[0], frame.width) {
            if inside(x as f64 + 0.5 - center[0], y as f64 + 0.5 - center[1]) {
                set(frame, x, y, rgb);
            }
        }
    }
}

fn ball(frame: &mut Frame) {
    let radius = D / 2.0;
    paint(frame, BALL, radius + 1.0, WHITE, |dx, dy| {
        dx * dx + dy * dy <= radius * radius
    });
}

/// A bar `length` × `width` centred on `center`, its long side along the head's axis.
fn bar(frame: &mut Frame, center: [f64; 2], length: f64, width: f64, rgb: [u8; 3]) {
    let (along, across) = (toe_ward(), forward());
    paint(
        frame,
        center,
        (length + width) / 2.0 + 1.0,
        rgb,
        |dx, dy| {
            dot([dx, dy], along).abs() <= length / 2.0 && dot([dx, dy], across).abs() <= width / 2.0
        },
    );
}

/// Where the shaft leaves the head.
#[derive(Clone, Copy, PartialEq)]
enum Shaft {
    AtHeel,
    AtToe,
    AtBoth,
    None,
}

/// One frame's head at `centre`, with its shaft.
fn club(frame: &mut Frame, centre: [f64; 2], shaft: Shaft) {
    let reach = (HEAD_LENGTH + SHAFT_LENGTH) / 2.0;
    if matches!(shaft, Shaft::AtHeel | Shaft::AtBoth) {
        let at = moved(centre, toe_ward(), -reach);
        bar(frame, at, SHAFT_LENGTH, SHAFT_WIDTH, SHAFT_RGB);
    }
    if matches!(shaft, Shaft::AtToe | Shaft::AtBoth) {
        let at = moved(centre, toe_ward(), reach);
        bar(frame, at, SHAFT_LENGTH, SHAFT_WIDTH, SHAFT_RGB);
    }
    bar(frame, centre, HEAD_LENGTH, HEAD_WIDTH, HEAD_RGB);
}

/// The strike: the ball at rest to [`BALL_LAST`], and the club drawn from [`HEAD_FIRST`] at
/// `head(frame)` (`None` for no club in that frame) with the shaft `shaft(frame)` gives.
fn strike(head: impl Fn(usize) -> Option<[f64; 2]>, shaft: impl Fn(usize) -> Shaft) -> Vec<Frame> {
    (0..FRAMES)
        .map(|index| {
            let mut frame = field();
            if index <= BALL_LAST {
                ball(&mut frame);
            }
            if let Some(centre) = head(index).filter(|_| index >= HEAD_FIRST) {
                club(&mut frame, centre, shaft(index));
            }
            frame
        })
        .collect()
}

/// The head's path: at `contact` in the contact frame, one [`STEP`] a frame along the face's
/// normal before and after it.
fn path(contact: [f64; 2]) -> impl Fn(usize) -> Option<[f64; 2]> {
    move |index| {
        let frames_from_contact = index as f64 - CONTACT as f64;
        Some(moved(contact, forward(), frames_from_contact * STEP))
    }
}

fn read(frames: &[Frame]) -> ImpactReading {
    match read_impact(frames) {
        Ok(reading) => reading,
        Err(refusal) => panic!("refused: {}", refusal.sentence()),
    }
}

// The preconditions every test below leans on, against the constants as they stand, checked when
// this file compiles: a P13 tuning that moves a constant past the drawing fails here, by the
// clause it broke, rather than somewhere in a test's arithmetic.
const _: () = {
    assert!(HEAD_MIN_LENGTH * D < HEAD_LENGTH && HEAD_LENGTH < HEAD_MAX_LENGTH * D);
    assert!(HEAD_LENGTH / HEAD_WIDTH > HEAD_MIN_ASPECT);
    // A pixel off the axis at contact, and two diameters further when the club stops short.
    assert!(1.0 < CONTACT_MAX_DISTANCE * D && CONTACT_MAX_DISTANCE * D < 2.0 * D - 1.0);
    // The shaft starts where the head ends; the join's reach is several pixels at its tightest.
    assert!(SHAFT_MAX_JOIN * D > 2.0);
    // In the ball's last frame the face is clear of the ball by more than a pixel, so the two are
    // never one component of the white mask.
    assert!(STEP - 1.0 - HEAD_WIDTH / 2.0 - D / 2.0 > 1.5);
};

#[test]
fn the_test_ball_is_inside_the_size_bracket() {
    let short = WIDTH.min(HEIGHT) as f64;
    assert!(BALL_MIN_FRAC * short < D && D < BALL_MAX_FRAC * short);
}

#[test]
fn a_strike_toward_the_toe_reads_positive_at_the_drawn_contact_frame() {
    let centre = contact_centre(6.0);
    assert!((drawn_offset_mm(centre) - 6.0).abs() < 0.1, "{centre:?}");
    let reading = read(&strike(path(centre), |_| Shaft::AtHeel));
    assert!((reading.offset_h_mm - 6.0).abs() <= 0.5, "{reading:?}");
    assert_eq!(reading.contact_frame, CONTACT);
    assert_eq!(reading.frames, FRAMES);
    assert!(
        (reading.ball_center[0] - BALL[0]).abs() <= 0.5,
        "{reading:?}"
    );
    assert!(
        (reading.ball_center[1] - BALL[1]).abs() <= 0.5,
        "{reading:?}"
    );
    assert!((reading.ball_diameter_px - D).abs() <= 1.0, "{reading:?}");
    // The heel is the end the shaft leaves, the toe the other.
    let (heel, toe) = (
        moved(centre, toe_ward(), -HEAD_LENGTH / 2.0),
        moved(centre, toe_ward(), HEAD_LENGTH / 2.0),
    );
    for (got, drawn) in [(reading.heel, heel), (reading.toe, toe)] {
        let off = (got[0] - drawn[0]).hypot(got[1] - drawn[1]);
        assert!(off <= 1.5, "{got:?} is {off} px from {drawn:?}");
    }
}

#[test]
fn a_strike_toward_the_heel_reads_negative() {
    let centre = contact_centre(-6.0);
    assert!((drawn_offset_mm(centre) + 6.0).abs() < 0.1, "{centre:?}");
    let reading = read(&strike(path(centre), |_| Shaft::AtHeel));
    assert!((reading.offset_h_mm + 6.0).abs() <= 0.5, "{reading:?}");
    assert_eq!(reading.contact_frame, CONTACT);
}

#[test]
fn the_shaft_at_the_other_end_flips_the_sign() {
    // The toe-ward strike's head, its shaft moved to the far end: that end is now the heel, so the
    // ball lies 6 mm toward the heel of the same midpoint.
    let centre = contact_centre(6.0);
    let reading = read(&strike(path(centre), |_| Shaft::AtToe));
    assert!((reading.offset_h_mm + 6.0).abs() <= 0.5, "{reading:?}");
    assert_eq!(reading.contact_frame, CONTACT);
}

#[test]
fn no_bar_is_no_club_head() {
    // The shaft comes on its own: dark, it is never taken for the light head.
    let centre = contact_centre(6.0);
    let frames: Vec<Frame> = (0..FRAMES)
        .map(|index| {
            let mut frame = field();
            if index <= BALL_LAST {
                ball(&mut frame);
            }
            let at = moved(
                path(centre)(index).unwrap(),
                toe_ward(),
                -(HEAD_LENGTH + SHAFT_LENGTH) / 2.0,
            );
            bar(&mut frame, at, SHAFT_LENGTH, SHAFT_WIDTH, SHAFT_RGB);
            frame
        })
        .collect();
    assert_eq!(read_impact(&frames), Err(ImpactRefusal::NoClubHead));
}

#[test]
fn a_bar_stopping_two_diameters_short_is_no_contact() {
    let centre = contact_centre(6.0);
    let stopped = moved(centre, forward(), -2.0 * D);
    // The strike's path until it reaches `stopped`, two frames short of contact, and there after.
    let head = move |index: usize| {
        let at = path(centre)(index).unwrap();
        let ahead = dot([at[0] - stopped[0], at[1] - stopped[1]], forward());
        Some(if ahead > 0.0 { stopped } else { at })
    };
    assert_eq!(
        read_impact(&strike(head, |_| Shaft::AtHeel)),
        Err(ImpactRefusal::NoContact)
    );
}

#[test]
fn no_shaft_is_no_shaft() {
    let centre = contact_centre(6.0);
    assert_eq!(
        read_impact(&strike(path(centre), |_| Shaft::None)),
        Err(ImpactRefusal::NoShaft)
    );
}

#[test]
fn shafts_at_both_ends_are_no_shaft() {
    // Which end is the heel would be a guess, and a guessed toe is a guessed sign (M42 P12).
    let centre = contact_centre(6.0);
    assert_eq!(
        read_impact(&strike(path(centre), |_| Shaft::AtBoth)),
        Err(ImpactRefusal::NoShaft)
    );
}

#[test]
fn a_shaft_hidden_at_contact_is_read_from_the_frame_before() {
    let centre = contact_centre(6.0);
    let hidden = |index| {
        if index == CONTACT {
            Shaft::None
        } else {
            Shaft::AtHeel
        }
    };
    let reading = read(&strike(path(centre), hidden));
    assert!((reading.offset_h_mm - 6.0).abs() <= 0.5, "{reading:?}");
    assert_eq!(
        reading.contact_frame, CONTACT,
        "the geometry stays the contact frame's"
    );
    // At the other end in the frame before, the sign follows it.
    let at_toe = |index| {
        if index == CONTACT {
            Shaft::None
        } else {
            Shaft::AtToe
        }
    };
    let reading = read(&strike(path(centre), at_toe));
    assert!((reading.offset_h_mm + 6.0).abs() <= 0.5, "{reading:?}");
}

#[test]
fn only_the_nearest_earlier_frame_is_asked_for_the_shaft() {
    // Two frames before contact is one too many: call 12 asks the nearest earlier frame with a
    // head, and no further.
    let centre = contact_centre(6.0);
    let early = |index| {
        if index == CONTACT - 2 {
            Shaft::AtHeel
        } else {
            Shaft::None
        }
    };
    assert_eq!(
        read_impact(&strike(path(centre), early)),
        Err(ImpactRefusal::NoShaft)
    );
}

#[test]
fn a_projection_past_sixty_mm_is_out_of_range() {
    // 62 mm is past the bar's toe end (60 px is ≈ 107 mm long, so its half is ≈ 53 mm); the head
    // segment still passes within reach, measured to the toe end.
    let centre = contact_centre(62.0);
    assert!((drawn_offset_mm(centre) - 62.0).abs() < 0.1, "{centre:?}");
    match read_impact(&strike(path(centre), |_| Shaft::AtHeel)) {
        Err(ImpactRefusal::OutOfRange(offset)) => {
            assert!((offset - 62.0).abs() <= 0.5, "{offset}");
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn every_refusal_says_why_in_words() {
    let cases = [
        (ImpactRefusal::NoFrames, "the clip held no frames to read"),
        (
            ImpactRefusal::NoBall,
            "no ball could be found sitting still in the replay",
        ),
        (
            ImpactRefusal::NoClubHead,
            "the replay showed the ball but no club head near it",
        ),
        (
            ImpactRefusal::NoContact,
            "the club head never reached the ball in the replay",
        ),
        (
            ImpactRefusal::NoShaft,
            "no shaft could be seen leaving the club head, so its heel could not be told from \
             its toe",
        ),
        (
            ImpactRefusal::OutOfRange(61.34),
            "the strike read +61.3 mm from the middle of the face, outside the 60 mm either way \
             a reading may be",
        ),
        (
            ImpactRefusal::OutOfRange(-75.0),
            "the strike read -75.0 mm from the middle of the face, outside the 60 mm either way \
             a reading may be",
        ),
    ];
    for (refusal, sentence) in cases {
        assert_eq!(refusal.sentence(), sentence);
    }
}
