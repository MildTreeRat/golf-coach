//! The ball's stage, on frames drawn here: call 12 of the M42 plan, held from outside. [M42 P11]
//!
//! Every frame is a dark green field with white discs on it, noise-free, so each test isolates one
//! clause of the rule. Sizes that the tunables bound are derived from the tunables, so a P13 tuning
//! within its bracket moves the test with the rule rather than breaking it.

use impact::{
    find_ball, read_impact, Ball, Frame, ImpactRefusal, BALL_MAX_FRAC, BALL_MIN_FRAC,
    BALL_MIN_FRAMES,
};

const WIDTH: usize = 640;
const HEIGHT: usize = 480;
const FIELD: [u8; 3] = [60, 80, 50];
const WHITE: [u8; 3] = [255, 255, 255];
/// The drawn ball's diameter, pixels: inside the size bracket at every tuning P13 may make, which
/// at the brackets' tightest is `[0.02, 0.04]` × 480 = `[9.6, 19.2]`. The first test holds it to
/// the constants as they stand.
const D: f64 = 16.0;

fn field() -> Frame {
    Frame {
        width: WIDTH,
        height: HEIGHT,
        rgb: FIELD.repeat(WIDTH * HEIGHT),
    }
}

/// Paints white every pixel whose centre lies within `diameter / 2` of `center`.
fn disc(frame: &mut Frame, center: [f64; 2], diameter: f64) {
    let radius = diameter / 2.0;
    for y in 0..frame.height {
        for x in 0..frame.width {
            let (dx, dy) = (x as f64 + 0.5 - center[0], y as f64 + 0.5 - center[1]);
            if dx * dx + dy * dy <= radius * radius {
                paint(frame, x, y);
            }
        }
    }
}

fn square(frame: &mut Frame, x0: usize, y0: usize, side: usize) {
    for y in y0..y0 + side {
        for x in x0..x0 + side {
            paint(frame, x, y);
        }
    }
}

fn paint(frame: &mut Frame, x: usize, y: usize) {
    let at = (y * frame.width + x) * 3;
    frame.rgb[at..at + 3].copy_from_slice(&WHITE);
}

/// `count` frames, frame `i` drawn by `draw(i, frame)`.
fn clip(count: usize, draw: impl Fn(usize, &mut Frame)) -> Vec<Frame> {
    (0..count)
        .map(|index| {
            let mut frame = field();
            draw(index, &mut frame);
            frame
        })
        .collect()
}

#[test]
fn the_test_ball_is_inside_the_size_bracket() {
    // The precondition every test below leans on, against the constants as they stand.
    let short = WIDTH.min(HEIGHT) as f64;
    assert!(BALL_MIN_FRAC * short < D && D < BALL_MAX_FRAC * short);
}

#[test]
fn a_disc_at_rest_then_gone_is_the_ball_until_it_goes() {
    let center = [250.3, 180.6];
    let frames = clip(10, |index, frame| {
        if index <= 5 {
            disc(frame, center, D);
        }
    });
    let ball = find_ball(&frames).unwrap();
    assert!((ball.center[0] - center[0]).abs() <= 0.5, "{ball:?}");
    assert!((ball.center[1] - center[1]).abs() <= 0.5, "{ball:?}");
    assert!((ball.diameter - D).abs() <= 1.0, "{ball:?}");
    assert_eq!((ball.first_frame, ball.last_frame), (0, 5));
    // Past the ball, the club head stage finds no bar, for none is drawn here (M42 P12).
    assert_eq!(read_impact(&frames), Err(ImpactRefusal::NoClubHead));
}

#[test]
fn a_disc_moving_more_than_half_its_diameter_a_frame_makes_no_chain() {
    let fast = clip(8, |index, frame| {
        disc(frame, [100.0 + 0.6 * D * index as f64, 200.0], D);
    });
    assert_eq!(find_ball(&fast), Err(ImpactRefusal::NoBall));
    // The control: under half a diameter a frame it chains, which is what makes the line above
    // a test of the step and not of the drawing.
    let slow = clip(8, |index, frame| {
        disc(frame, [100.0 + 0.4 * D * index as f64, 200.0], D);
    });
    let ball = find_ball(&slow).unwrap();
    assert_eq!((ball.first_frame, ball.last_frame), (0, 7));
}

#[test]
fn a_white_square_is_not_a_ball() {
    let frames = clip(6, |_, frame| square(frame, 300, 200, D as usize));
    assert_eq!(find_ball(&frames), Err(ImpactRefusal::NoBall));
}

#[test]
fn of_two_discs_the_longer_chain_wins() {
    let (short, long) = ([150.5, 150.5], [450.5, 300.5]);
    let frames = clip(8, |index, frame| {
        if index <= 3 {
            disc(frame, short, D);
        }
        if index <= 6 {
            disc(frame, long, D);
        }
    });
    let ball = find_ball(&frames).unwrap();
    assert!((ball.center[0] - long[0]).abs() <= 0.5, "{ball:?}");
    assert!((ball.center[1] - long[1]).abs() <= 0.5, "{ball:?}");
    assert_eq!((ball.first_frame, ball.last_frame), (0, 6));
}

#[test]
fn of_two_equally_long_chains_the_earlier_wins() {
    let (early, late) = ([150.5, 150.5], [450.5, 300.5]);
    let frames = clip(8, |index, frame| {
        if index <= 3 {
            disc(frame, early, D);
        }
        if (2..=5).contains(&index) {
            disc(frame, late, D);
        }
    });
    let ball = find_ball(&frames).unwrap();
    assert!((ball.center[0] - early[0]).abs() <= 0.5, "{ball:?}");
    assert_eq!((ball.first_frame, ball.last_frame), (0, 3));
}

#[test]
fn a_chain_must_last_the_minimum_frames() {
    let center = [320.0, 240.0];
    let lasting = |count: usize| {
        clip(count + 2, move |index, frame| {
            if index < count {
                disc(frame, center, D);
            }
        })
    };
    assert_eq!(
        find_ball(&lasting(BALL_MIN_FRAMES - 1)),
        Err(ImpactRefusal::NoBall)
    );
    let ball: Ball = find_ball(&lasting(BALL_MIN_FRAMES)).unwrap();
    assert_eq!(ball.last_frame, BALL_MIN_FRAMES - 1);
}

#[test]
fn a_disc_outside_the_size_bracket_is_not_a_ball() {
    let short = WIDTH.min(HEIGHT) as f64;
    for diameter in [0.6 * BALL_MIN_FRAC * short, 1.4 * BALL_MAX_FRAC * short] {
        let frames = clip(6, |_, frame| disc(frame, [320.0, 240.0], diameter));
        assert_eq!(
            find_ball(&frames),
            Err(ImpactRefusal::NoBall),
            "diameter {diameter}"
        );
    }
}

#[test]
fn no_frames_is_refused_as_such() {
    assert_eq!(find_ball(&[]), Err(ImpactRefusal::NoFrames));
    assert_eq!(read_impact(&[]), Err(ImpactRefusal::NoFrames));
}

#[test]
fn no_disc_is_no_ball() {
    let frames = clip(6, |_, _| {});
    assert_eq!(find_ball(&frames), Err(ImpactRefusal::NoBall));
    assert_eq!(read_impact(&frames), Err(ImpactRefusal::NoBall));
}

#[test]
#[should_panic(expected = "one clip's frames share a size")]
fn frames_of_two_sizes_are_the_callers_bug() {
    let small = Frame {
        width: 4,
        height: 4,
        rgb: FIELD.repeat(16),
    };
    let _ = find_ball(&[field(), small]);
}
