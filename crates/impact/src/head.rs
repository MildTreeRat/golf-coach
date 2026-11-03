//! The club head: in each frame, the light bar near the ball. [M42 P12]
//!
//! No Python twin (see the crate doc). Call 12 of the M42 plan is the rule, with nothing added:
//! the light components of the search region, the ball's disc taken out of the light mask while
//! the ball is still on screen, those shaped like a head ([`HEAD_MIN_LENGTH`] to
//! [`HEAD_MAX_LENGTH`] ball diameters long and at least [`HEAD_MIN_ASPECT`] times as long as wide),
//! and of those the one whose axis segment passes closest to the ball's centre.

use crate::ball::Ball;
use crate::component::{components, segment_distance, Component};
use crate::frame::Frame;
use crate::mask::{is_light, Mask};
use crate::{HEAD_MAX_LENGTH, HEAD_MIN_ASPECT, HEAD_MIN_LENGTH};

/// The radius of the ball's disc taken out of the light mask, in ball diameters: call 12's 0.55,
/// a twentieth of a diameter past the ball's own edge so the rim that compression and blur smear
/// round it goes too. Fixed rather than tunable, since call 12 does not list it among P13's.
const DISC_RADIUS: f64 = 0.55;

/// One frame's club head: the bar, and how far its axis segment passes from the ball's centre.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Head {
    pub bar: Component,
    /// Pixels, from the ball's centre to the nearest point of the bar's axis segment.
    pub distance: f64,
}

/// The head in each frame from the first to one past the ball's last (or the clip's last, when
/// the ball lasts to its end): `None` where no light component is shaped like one.
///
/// One past the ball's last because the frame that shows contact may already have lost the ball,
/// under the head or into flight, and the head there still lies where the strike was.
pub(crate) fn heads(frames: &[Frame], ball: &Ball) -> Vec<Option<Head>> {
    let last = (ball.last_frame + 1).min(frames.len() - 1);
    frames[..=last]
        .iter()
        .enumerate()
        .map(|(index, frame)| {
            let present = (ball.first_frame..=ball.last_frame).contains(&index);
            head_in(frame, ball, present)
        })
        .collect()
}

/// One frame's head, `ball_present` saying whether the ball's chain holds this frame (and so
/// whether its disc is taken out of the light mask). Of two candidates at one distance, the first
/// a row-major scan meets.
pub(crate) fn head_in(frame: &Frame, ball: &Ball, ball_present: bool) -> Option<Head> {
    let region = ball.search_region(frame.width, frame.height);
    let mut mask = Mask::of(frame, region, is_light);
    if ball_present {
        mask.clear_disc(ball.center, DISC_RADIUS * ball.diameter);
    }
    let lengths = HEAD_MIN_LENGTH * ball.diameter..=HEAD_MAX_LENGTH * ball.diameter;
    let mut best: Option<Head> = None;
    for bar in components(&mask) {
        if !lengths.contains(&bar.length) || bar.length < HEAD_MIN_ASPECT * bar.width {
            continue;
        }
        let distance = segment_distance(bar.ends, ball.center);
        if best.is_none_or(|held| distance < held.distance) {
            best = Some(Head { bar, distance });
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    const WIDTH: usize = 400;
    const HEIGHT: usize = 300;
    const FIELD: [u8; 3] = [60, 80, 50];
    const BALL: Ball = Ball {
        center: [200.0, 150.0],
        diameter: 24.0,
        first_frame: 0,
        last_frame: 0,
    };

    /// Paints `rgb` on every pixel whose centre `inside` accepts.
    fn paint(frame: &mut Frame, rgb: [u8; 3], inside: impl Fn(f64, f64) -> bool) {
        for y in 0..frame.height {
            for x in 0..frame.width {
                if inside(x as f64 + 0.5, y as f64 + 0.5) {
                    let at = (y * frame.width + x) * 3;
                    frame.rgb[at..at + 3].copy_from_slice(&rgb);
                }
            }
        }
    }

    /// An axis-aligned bar `length` × `width`, long side horizontal, centred on `center`.
    fn bar(frame: &mut Frame, center: [f64; 2], length: f64, width: f64) {
        paint(frame, [230, 230, 230], |x, y| {
            (x - center[0]).abs() <= length / 2.0 && (y - center[1]).abs() <= width / 2.0
        });
    }

    fn field() -> Frame {
        Frame {
            width: WIDTH,
            height: HEIGHT,
            rgb: FIELD.repeat(WIDTH * HEIGHT),
        }
    }

    fn ball(frame: &mut Frame) {
        let radius = BALL.diameter / 2.0;
        paint(frame, [255, 255, 255], |x, y| {
            (x - BALL.center[0]).powi(2) + (y - BALL.center[1]).powi(2) <= radius * radius
        });
    }

    #[test]
    fn a_head_touching_the_ball_is_its_own_bar_while_the_ball_is_on_screen() {
        // The bar's lower edge (y ≤ 143) meets the disc's top (y ≥ 138) by five rows: one
        // component in the light mask unless the disc comes out first.
        let mut frame = field();
        ball(&mut frame);
        bar(&mut frame, [200.0, 138.0], 60.0, 10.0);
        let head = head_in(&frame, &BALL, true).expect("the bar, with the ball taken out");
        assert!((head.bar.length - 60.0).abs() <= 1.5, "{head:?}");
        assert!((head.bar.centroid[0] - 200.0).abs() < 1e-9, "{head:?}");
        // The disc also took a notch out of the bar's lower middle (its rim is 13.2 px out, past
        // the bar's edge at 7), which lifts the centroid and the axis with it: ≈ 12.5 px from the
        // ball's centre where the bar was drawn at 12.
        assert!(head.distance > 12.0 && head.distance < 13.0, "{head:?}");
        // Left in, the ball joins the bar: what the mask holds there is the pair, far wider than
        // the bar, if it passes for a head at all (at the starting aspect it does not).
        assert!(head_in(&frame, &BALL, false).is_none_or(|pair| pair.bar.width > 25.0));
    }

    #[test]
    fn of_two_bars_the_one_passing_nearer_the_ball_is_the_head() {
        let mut frame = field();
        bar(&mut frame, [200.0, 100.0], 60.0, 10.0); // 50 px above the ball's centre
        bar(&mut frame, [200.0, 180.0], 60.0, 10.0); // 30 px below it
        let head = head_in(&frame, &BALL, false).unwrap();
        assert!((head.bar.centroid[1] - 180.0).abs() < 1e-9, "{head:?}");
        assert!((head.distance - 30.0).abs() < 1e-9, "{head:?}");
    }

    #[test]
    fn a_bar_outside_the_heads_shape_is_no_head() {
        let d = BALL.diameter;
        for (length, width) in [
            (0.8 * HEAD_MIN_LENGTH * d, 4.0),                    // too short
            (1.1 * HEAD_MAX_LENGTH * d, 10.0),                   // too long
            (2.5 * d, (2.5 * d / HEAD_MIN_ASPECT).ceil() + 4.0), // too squat
        ] {
            let mut frame = field();
            bar(&mut frame, [200.0, 110.0], length, width);
            assert!(
                head_in(&frame, &BALL, false).is_none(),
                "{length} × {width}"
            );
        }
    }

    #[test]
    fn the_head_is_looked_for_until_one_past_the_balls_last_frame() {
        let frames = vec![field(); 6];
        let ball = Ball {
            last_frame: 2,
            ..BALL
        };
        assert_eq!(heads(&frames, &ball).len(), 4, "frames 0 to 3");
        let to_the_end = Ball {
            last_frame: 5,
            ..BALL
        };
        assert_eq!(
            heads(&frames, &to_the_end).len(),
            6,
            "no frame 6 to look in"
        );
    }
}
