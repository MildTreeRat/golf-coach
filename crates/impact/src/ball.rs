//! The ball: a white disc that stays put across consecutive frames until the club takes it.
//! [M42 P11]
//!
//! No Python twin (see the crate doc). Call 12 of the M42 plan is the rule, and this module is it
//! with nothing added: per frame the white components shaped like a ball, chained across
//! consecutive frames, the longest chain the ball.

use crate::component::{components, Component};
use crate::frame::{Frame, Rect};
use crate::mask::{is_white, Mask};
use crate::{ImpactRefusal, BALL_MAX_FRAC, BALL_MIN_FRAC, BALL_MIN_FRAMES};

// Call 12's fixed numbers: not among P13's tunables, so not public.

/// A candidate's bounding box, width over height. A disc's is 1; the bracket admits one squashed by
/// a pixel or two of blur on a ball 20 px across, and refuses a digit, which is taller than wide.
///
/// **This is what refuses the real ball** (M42 P13): on the five grid clips the replay's ball is an
/// ellipse wider than tall, its white component's box measuring 1.35, 1.38, 1.40, 1.44 and 1.47 at
/// the 960-wide decode (`IMG_2894`, `IMG_2892`, `IMG_2893`, `IMG_2889`, `IMG_2891`, one frame each
/// with the ball at rest), and 1.27 to 1.53 across the white class's bracket ends. So at the
/// starting values it is never a candidate, and where a bracket end brings one clip's ball inside
/// (`IMG_2892` at `WHITE_MIN` 170 and `WHITE_SPREAD` 70: 1.27) a mark of the screen's own still
/// outlasts it and wins the chain. Fixed by call 12 and not among P13's tunables, so P13 left it as it is and recorded
/// it; widening it is the first change a later look at the reader would make, and not the last
/// (the plan's P13 findings).
const ASPECT: [f64; 2] = [0.75, 1.33];

/// Its share of its bounding box. A drawn disc fills `π/4 ≈ 0.785` and a square 1, so a white tile
/// corner or a block of text is never a ball, and a ring (an `0`, an `o`) fills far less.
const FILL: [f64; 2] = [0.65, 0.92];

/// How far a candidate may sit from the previous frame's and still be the same ball, in the
/// candidate's own diameters. The ball is at rest until it is struck, so anything that moves half
/// its width in a frame is not it, or is it already in flight.
const CHAIN_STEP: f64 = 0.5;

/// The search region's side, in ball diameters: the club head is looked for in the square of this
/// side centred on the ball, which holds a head 2–3 diameters long (finding 9) arriving from any
/// direction with room to spare, and keeps the grid's digits beside the replay panel out.
const SEARCH_SIDE: f64 = 12.0;

/// The ball, as [`find_ball`] chained it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ball {
    /// Its centre, pixels: the median over the chain, each axis on its own.
    pub center: [f64; 2],
    /// Its diameter `D`, pixels: the median of the chain's equivalent diameters, `2·sqrt(area/π)`.
    pub diameter: f64,
    /// The chain's first frame.
    pub first_frame: usize,
    /// The chain's last frame, `last_ball` in call 12: the club head is looked for up to one frame
    /// past it, because the frame that shows contact may already have lost the ball under the head.
    pub last_frame: usize,
}

impl Ball {
    /// The square of side `12·D` centred on the ball, clipped to the frame.
    pub(crate) fn search_region(&self, width: usize, height: usize) -> Rect {
        Rect::square(self.center, SEARCH_SIDE * self.diameter, width, height)
    }
}

/// One frame's ball-shaped white component.
#[derive(Debug, Clone, Copy)]
struct Candidate {
    center: [f64; 2],
    diameter: f64,
}

/// Where a candidate's chain stands: how long it is, the frame it started in, and the candidate it
/// continues in the frame before.
#[derive(Debug, Clone, Copy)]
struct Link {
    length: usize,
    start: usize,
    previous: Option<usize>,
}

/// The ball, from one clip's frames in time order.
///
/// Public, where the plan had it crate-private, because `tests/ball.rs` holds its centre, diameter
/// and frames to call 12, and an integration test sees only public items.
///
/// The ball is the **longest** chain of candidates in consecutive frames, each within
/// `CHAIN_STEP` (0.5) × its own diameter of the one before, and at least [`BALL_MIN_FRAMES`] long.
/// A tie in length goes to the chain that starts first; a candidate with two predecessors on
/// equally long chains continues the nearer, and the first met in scan order after that.
///
/// # Panics
///
/// When a frame's buffer is not `width × height × 3` bytes, or the frames differ in size: see
/// [`crate::read_impact`].
pub fn find_ball(frames: &[Frame]) -> Result<Ball, ImpactRefusal> {
    let Some(first) = frames.first() else {
        return Err(ImpactRefusal::NoFrames);
    };
    for (index, frame) in frames.iter().enumerate() {
        assert!(
            frame.is_well_formed(),
            "frame {index}: {} bytes of rgb for {} × {} pixels",
            frame.rgb.len(),
            frame.width,
            frame.height
        );
        assert!(
            (frame.width, frame.height) == (first.width, first.height),
            "frame {index} is {} × {} and frame 0 is {} × {}: one clip's frames share a size",
            frame.width,
            frame.height,
            first.width,
            first.height
        );
    }
    let candidates: Vec<Vec<Candidate>> = frames.iter().map(candidates).collect();

    let mut links: Vec<Vec<Link>> = Vec::with_capacity(frames.len());
    for (index, here) in candidates.iter().enumerate() {
        let before = index.checked_sub(1).map(|b| (&candidates[b], &links[b]));
        let row = here
            .iter()
            .map(|candidate| link(candidate, index, before))
            .collect();
        links.push(row);
    }

    // The longest chain, ties to the earliest start. Chains of one length ending in the same frame
    // start in the same frame, so the first met in (frame, scan) order wins what is left.
    let mut best: Option<(usize, usize)> = None;
    for (index, row) in links.iter().enumerate() {
        for (at, here) in row.iter().enumerate() {
            let better = match best {
                None => true,
                Some((b_index, b_at)) => {
                    let held = links[b_index][b_at];
                    here.length > held.length
                        || (here.length == held.length && here.start < held.start)
                }
            };
            if better {
                best = Some((index, at));
            }
        }
    }
    let Some((last_frame, mut at)) = best else {
        return Err(ImpactRefusal::NoBall);
    };
    let chain = links[last_frame][at];
    if chain.length < BALL_MIN_FRAMES {
        return Err(ImpactRefusal::NoBall);
    }

    let (mut xs, mut ys, mut diameters) = (Vec::new(), Vec::new(), Vec::new());
    for index in (chain.start..=last_frame).rev() {
        let candidate = candidates[index][at];
        xs.push(candidate.center[0]);
        ys.push(candidate.center[1]);
        diameters.push(candidate.diameter);
        if let Some(previous) = links[index][at].previous {
            at = previous;
        }
    }
    let median = |values| crate::median(values).expect("a chain holds at least one candidate");
    Ok(Ball {
        center: [median(xs), median(ys)],
        diameter: median(diameters),
        first_frame: chain.start,
        last_frame,
    })
}

/// One frame's white components that are shaped and sized like a ball.
fn candidates(frame: &Frame) -> Vec<Candidate> {
    let short_side = frame.width.min(frame.height) as f64;
    let (smallest, largest) = (BALL_MIN_FRAC * short_side, BALL_MAX_FRAC * short_side);
    let mask = Mask::of(frame, Rect::whole(frame), is_white);
    components(&mask)
        .into_iter()
        .filter(|c: &Component| {
            let diameter = c.equivalent_diameter();
            (smallest..=largest).contains(&diameter)
                && (ASPECT[0]..=ASPECT[1]).contains(&c.aspect())
                && (FILL[0]..=FILL[1]).contains(&c.fill())
        })
        .map(|c| Candidate {
            center: c.centroid,
            diameter: c.equivalent_diameter(),
        })
        .collect()
}

/// Where `candidate`'s chain stands, given the frame before's candidates and their links.
fn link(
    candidate: &Candidate,
    index: usize,
    before: Option<(&Vec<Candidate>, &Vec<Link>)>,
) -> Link {
    let mut best: Option<(usize, f64, usize)> = None; // (length, distance, at)
    if let Some((candidates, links)) = before {
        for (at, (previous, its)) in candidates.iter().zip(links).enumerate() {
            let distance = (candidate.center[0] - previous.center[0])
                .hypot(candidate.center[1] - previous.center[1]);
            if distance > CHAIN_STEP * candidate.diameter {
                continue;
            }
            let better = match best {
                None => true,
                Some((length, nearest, _)) => {
                    its.length > length || (its.length == length && distance < nearest)
                }
            };
            if better {
                best = Some((its.length, distance, at));
            }
        }
    }
    match best {
        Some((length, _, at)) => Link {
            length: length + 1,
            start: before.map_or(index, |(_, links)| links[at].start),
            previous: Some(at),
        },
        None => Link {
            length: 1,
            start: index,
            previous: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_search_region_is_twelve_diameters_clipped() {
        let ball = Ball {
            center: [100.0, 30.0],
            diameter: 10.0,
            first_frame: 0,
            last_frame: 4,
        };
        let region = ball.search_region(640, 480);
        assert_eq!((region.x0, region.x1), (40, 160));
        assert_eq!((region.y0, region.y1), (0, 90), "clipped at the top");
    }
}
