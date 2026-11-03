//! The shaft, and from it which end of the club head is the heel. [M42 P12]
//!
//! No Python twin (see the crate doc). Call 12 of the M42 plan is the rule: in the contact frame, a
//! dark component of the search region at least [`SHAFT_MIN_LENGTH`] ball diameters long and
//! [`SHAFT_MIN_ASPECT`] times as long as wide, one end of whose axis segment lies within
//! [`SHAFT_MAX_JOIN`] ball diameters of one end of the head's. That head end is the **heel**, since
//! the hosel takes the shaft, and the other is the toe. When none is found there, the nearest
//! earlier frame with a head is asked the same, and nothing earlier than that. No shaft is
//! [`ImpactRefusal::NoShaft`], never a default orientation: a guessed toe is a guessed sign.
//!
//! Two things call 12 left open are decided here (M42 P12, for P15 to document):
//!
//! - **Shafts at both ends are no shaft.** When dark components join both of the head's ends (a
//!   shadow lying along the head would), which end is the heel is a guess, so the frame has found
//!   no heel, and the earlier frame is asked as for none.
//! - **An earlier frame lends its heel and not its geometry.** The heel found on the earlier
//!   frame's head is carried to the contact frame's head by the agreement of their axes, and the
//!   reading's heel and toe are the contact head's ends: the contact frame is the one call 12
//!   measured at the ball, and a head does not turn end for end between two frames.

use crate::ball::Ball;
use crate::component::{components, Component};
use crate::frame::Frame;
use crate::head::Head;
use crate::mask::{is_dark, median_luminance, Mask};
use crate::{ImpactRefusal, SHAFT_MAX_JOIN};

/// The shortest shaft, in ball diameters: call 12's 1.5. From above a shaft is foreshortened by
/// its lie but still runs several diameters from the hosel toward the hands, so a dark mark shorter
/// than a head's least length is not one. Fixed rather than tunable, since call 12 does not list
/// it among P13's.
const SHAFT_MIN_LENGTH: f64 = 1.5;

/// The least length over width a shaft has: call 12's 3. A shaft is far thinner than a head, so it
/// is held to a stricter aspect than [`crate::HEAD_MIN_ASPECT`]. Fixed, like the length.
const SHAFT_MIN_ASPECT: f64 = 3.0;

/// The contact head's heel and toe, in that order.
///
/// `heads` is [`crate::head::heads`]' answer and `contact` the frame
/// [`crate::contact::contact_frame`] chose, which holds a head.
pub(crate) fn heel_and_toe(
    frames: &[Frame],
    ball: &Ball,
    heads: &[Option<Head>],
    contact: usize,
) -> Result<([f64; 2], [f64; 2]), ImpactRefusal> {
    let bar = heads[contact]
        .expect("the contact frame is one with a head")
        .bar;
    let heel = match heel_end(&frames[contact], ball, &bar) {
        Some(end) => end,
        None => {
            let Some((index, earlier)) = (0..contact)
                .rev()
                .find_map(|index| heads[index].map(|head| (index, head.bar)))
            else {
                return Err(ImpactRefusal::NoShaft);
            };
            let Some(end) = heel_end(&frames[index], ball, &earlier) else {
                return Err(ImpactRefusal::NoShaft);
            };
            // A component's ends run along its axis, the lower projection first, so the earlier
            // head's end `end` is the contact head's end `end` when the axes agree and the other
            // one when they point apart. At right angles they say nothing.
            let agreement = bar.axis[0] * earlier.axis[0] + bar.axis[1] * earlier.axis[1];
            if agreement > 0.0 {
                end
            } else if agreement < 0.0 {
                1 - end
            } else {
                return Err(ImpactRefusal::NoShaft);
            }
        }
    };
    Ok((bar.ends[heel], bar.ends[1 - heel]))
}

/// Which of `bar`'s ends a shaft joins in `frame`: `Some(0)` or `Some(1)` into `bar.ends`, or
/// `None` when no shaft joins either, or shafts join both.
fn heel_end(frame: &Frame, ball: &Ball, bar: &Component) -> Option<usize> {
    let region = ball.search_region(frame.width, frame.height);
    let median = median_luminance(frame, region)?;
    let mask = Mask::of(frame, region, |rgb| is_dark(rgb, median));
    let reach = SHAFT_MAX_JOIN * ball.diameter;
    let mut joined = [false; 2];
    for shaft in components(&mask) {
        if shaft.length < SHAFT_MIN_LENGTH * ball.diameter
            || shaft.length < SHAFT_MIN_ASPECT * shaft.width
        {
            continue;
        }
        for (end, at) in bar.ends.iter().enumerate() {
            if shaft
                .ends
                .iter()
                .any(|tip| (tip[0] - at[0]).hypot(tip[1] - at[1]) <= reach)
            {
                joined[end] = true;
            }
        }
    }
    match joined {
        [true, false] => Some(0),
        [false, true] => Some(1),
        _ => None,
    }
}
