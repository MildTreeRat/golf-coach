//! The contact frame: the one whose club head passes closest to the ball. [M42 P12]
//!
//! No Python twin (see the crate doc). Call 12 of the M42 plan is the rule, with nothing added.

use crate::ball::Ball;
use crate::head::Head;
use crate::{ImpactRefusal, CONTACT_MAX_DISTANCE};

/// Of the frames with a head, the one whose head's axis segment passes closest to the ball's
/// centre, ties to the earliest.
///
/// [`ImpactRefusal::NoClubHead`] when no frame had a head; [`ImpactRefusal::NoContact`] when the
/// closest passes further than [`CONTACT_MAX_DISTANCE`] ball diameters from the ball's centre.
pub(crate) fn contact_frame(heads: &[Option<Head>], ball: &Ball) -> Result<usize, ImpactRefusal> {
    let mut best: Option<(usize, f64)> = None;
    for (index, head) in heads.iter().enumerate() {
        let Some(head) = head else { continue };
        if best.is_none_or(|(_, nearest)| head.distance < nearest) {
            best = Some((index, head.distance));
        }
    }
    let Some((frame, distance)) = best else {
        return Err(ImpactRefusal::NoClubHead);
    };
    if distance > CONTACT_MAX_DISTANCE * ball.diameter {
        return Err(ImpactRefusal::NoContact);
    }
    Ok(frame)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::component::Component;
    use crate::frame::Rect;

    const BALL: Ball = Ball {
        center: [100.0, 100.0],
        diameter: 20.0,
        first_frame: 0,
        last_frame: 3,
    };

    /// A head at `distance`; the stage reads nothing of it but that.
    fn at(distance: f64) -> Option<Head> {
        let empty = Rect {
            x0: 0,
            y0: 0,
            x1: 0,
            y1: 0,
        };
        let bar = Component {
            area: 0,
            centroid: [0.0; 2],
            bbox: empty,
            axis: [1.0, 0.0],
            length: 0.0,
            width: 0.0,
            ends: [[0.0; 2]; 2],
        };
        Some(Head { bar, distance })
    }

    #[test]
    fn the_nearest_head_is_contact_and_a_tie_goes_to_the_earlier() {
        let heads = [None, at(9.0), at(4.0), None, at(4.0)];
        assert_eq!(contact_frame(&heads, &BALL), Ok(2));
    }

    #[test]
    fn no_head_is_no_club_head_and_a_far_one_no_contact() {
        assert_eq!(
            contact_frame(&[None, None], &BALL),
            Err(ImpactRefusal::NoClubHead)
        );
        let reach = CONTACT_MAX_DISTANCE * BALL.diameter;
        assert_eq!(contact_frame(&[at(reach)], &BALL), Ok(0), "at the limit");
        assert_eq!(
            contact_frame(&[at(reach + 0.01)], &BALL),
            Err(ImpactRefusal::NoContact)
        );
    }
}
