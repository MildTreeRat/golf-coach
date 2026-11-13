//! Where on the club face the ball was struck, heel to toe, read off Full Swing's impact replay.
//! [M42 P11, P12]
//!
//! Full Swing's grid screen plays a slow-motion overhead replay of each strike in a panel beside
//! the numbers, and a Live Photo of the screen taken while it plays catches the replay in its
//! motion part: 1 to 2 s of HEVC, 31 to 61 frames (the M42 plan's finding 9). From above, the ball
//! is a white disc and an iron head a light bar two or three ball-diameters long, with a dark shaft
//! leaving one end. Heel to toe is visible from there and high to low is not, so this crate reads
//! `impact_offset_h` and never `impact_offset_v`. Why a clip at all, in the user's words: "The
//! graphic doesn't accurately show where the ball was hit. The video is the best indicator we can
//! get."
//!
//! **This reader is `contracts::face_impact`'s `ImpactMethod::OverheadReplay`** (M46 P5, ADR-037
//! clause 1), whose declared axes, heel to toe and not high to low, are this paragraph's as a rule.
//! It has no version key, so a `FaceImpact` it made would have nothing for `method_version` to hold,
//! and nothing in M46 makes one.
//!
//! **It has no Python twin.** No Python ever read the replay and frozen Python never gains it
//! (ADR-035 clause 4), so there is nothing to port and no vector recorded from one; its tests draw
//! their own frames. It depends on nothing, std only, and decodes no video: frames arrive as RGB
//! bytes, from `golf-lab`'s ffmpeg on the laptop (M42 P13, P14b) and from the phone's own decoder
//! once M38's `crates/app` calls it. That later caller is why `crates/ocr/tests/ort_reach.rs`
//! guards this crate although it has no dependency to guard.
//!
//! # The stages, in the order they run
//!
//! - `frame` — [`Frame`], a pixel's luminance, and the clipped rectangles every stage reads;
//! - `mask` — the three colour classes (white, light, dark) and a class's pixels as a mask;
//! - `component` — 4-connected components, each with its area, centroid, bounding box and
//!   principal axis from second moments;
//! - `ball` — [`find_ball`]: per frame the white components shaped like a ball, chained across
//!   consecutive frames, the longest chain the ball;
//! - `head` — per frame, from the first to one past the ball's last, the light bar shaped like a
//!   club head whose axis passes closest to the ball, the ball's disc taken out of the light mask
//!   while the ball is on screen;
//! - `contact` — the frame whose head passes closest to the ball, which must be within reach of it;
//! - `shaft` — the dark bar joining one end of the contact head, which makes that end the heel;
//! - the offset, in [`read_impact`]: the ball's centre projected on the heel-to-toe axis, from the
//!   axis's midpoint, scaled by the ball.
//!
//! One more public item reads frames and is no stage of the replay's: [`sharpest_frame`] (P14),
//! which picks the frame of a clip whose grid of numbers the still's screen reader reads, so a clip
//! imports a shot with no still beside it (the M42 plan's decision 7). It sits in `frame` because
//! the phone needs it beside [`read_impact`].
//!
//! # What it approximates, stated rather than measured
//!
//! - **The scale is the ball.** Pixels become millimetres through [`BALL_DIAMETER_MM`], the Rules
//!   of Golf minimum; nothing else in the replay says how large anything is.
//! - **The face centre is the head bar's midpoint** (P12). The hosel lengthens the bar at the heel,
//!   so the midpoint sits a little heelward of the true centre, by an amount nobody measured.
//! - **Its accuracy is not judged in M42** (the plan's decision 3): no face-tape or spray test
//!   stands behind any number here, and the user reviews P13's images afterwards. The constants
//!   below are starting values, each with what it was set from, and P13 may tune each within the
//!   bracket its doc names and no further.
//! - **On the five grid clips it reads nothing** (P13, run through `golf-lab read-impact`): the
//!   ball stage never finds the replay's ball, at any tunable's value, and every clip is refused
//!   by name. The tunables' block below says why; a reader that gets past it needs a ball stage
//!   call 12 did not design, which P13 was not allowed to.
//!
//! # Coordinates
//!
//! Continuous pixel coordinates in the frames, `x` right and `y` down: pixel `(i, j)` covers
//! `[i, i + 1) × [j, j + 1)` and its centre is `(i + 0.5, j + 0.5)`. A rectangle given in them is
//! clipped to whole pixels as `TextBox.rgb`'s box is (M42 P2), so a box, a ball and a club head can
//! be drawn on one review image without a half-pixel shift between them.
//!
//! # The second method: a STRIKEPRO label on the face (M46)
//!
//! [`tape`] answers the same question from a photo of a STRIKEPRO impact label stuck on the face,
//! taken after every strike of a drill (the M46 plan and ADR-037). Unlike the replay it sees both
//! axes, heel to toe and high to low, in millimetres from the label's printed centre, and its scale
//! is the label rather than the ball. The stages and approximations above are the replay's alone.
//! The tape's reader is to reuse `frame`, `mask` and `component` (the plan's C1) and to stay std
//! only, as this crate is. Since P4 the module holds the labels' geometry, and the reader lands in
//! P7 to P13.

mod ball;
mod component;
mod contact;
mod frame;
mod head;
mod mask;
mod shaft;
pub mod tape;

pub use ball::{find_ball, Ball};
pub use frame::{sharpest_frame, Frame};

/// The ball's diameter, millimetres: the Rules of Golf minimum, 1.680 in. It is the reading's only
/// scale, so a ball imaged at `D` pixels makes a pixel `42.67 / D` mm at the ball's depth.
pub const BALL_DIAMETER_MM: f64 = 42.67;

// --- The tunables. -------------------------------------------------------------------------------
//
// Call 12 of the M42 plan sets each starting value and its bracket, and P13 may move each within
// its bracket only, recording before and after in the plan's findings. None was set on a clip: the
// planning read measured the stills (finding 4) and looked at the clips by eye (finding 9).
//
// **P13 ran them on the five grid clips and moved none.** `golf-lab read-impact` read none of the
// five at these values, and none at either end of any one bracket, nor at the two combinations of
// bracket ends tried (the plan's P13 findings hold every run). The ball stage is why: at these
// values the replay's ball is never a candidate, its bounding box measuring 1.35–1.47 wide over tall on all five
// against the fixed `ASPECT` ceiling of 1.33 (`ball.rs`), and the chain it would have to win is
// always won by a mark of the screen's own (a letter of the player's name, a digit of the grid),
// which lasts the whole clip where the ball leaves at impact. So every value below is still its
// starting value, and what each doc says it was set from is still all that stands behind it.

/// The *white* class's floor: every channel at least this. White is the ball.
///
/// Set from the screen's white value digits in the rectified stills, (233–251, 233–250, 221–249)
/// across the photos (finding 4): a floor below their dimmest channel, with room for the replay's
/// compression and a dimmer exposure. P13 brackets it at `[170, 230]`.
pub const WHITE_MIN: u8 = 200;

/// The *white* class's spread: `max − min` over the three channels at most this, so a lit yellow
/// sign letter (blue at 0.32–0.47 of `min(red, green)`, finding 4) is never white however bright.
/// The white digits measured blue at 0.95–1.0 of `min(red, green)`, well inside it. P13 brackets it
/// at `[25, 70]`.
pub const WHITE_SPREAD: u8 = 45;

/// The *light* class's floor: every channel at least this. Light is the iron head.
///
/// **A starting value and not a measurement**: the planning read saw the head as a light grey bar
/// (finding 9) and measured none of its pixels. It sits below [`WHITE_MIN`] so a head greyer than
/// the ball is still light, which makes every white pixel light too; that is why the head stage
/// removes the ball's disc from the light mask. The stills' unlit sign letters, (187, 183, 176) and
/// (200–215, 206–214, 197–206), are light by this rule. P13 brackets it at `[110, 190]`.
pub const LIGHT_MIN: u8 = 140;

/// The *light* class's spread, `max − min` at most this: grey, not a colour. A starting value with
/// no measurement behind it, like [`LIGHT_MIN`]. P13 brackets it at `[40, 90]`.
pub const LIGHT_SPREAD: u8 = 60;

/// The *dark* class: luminance `0.299r + 0.587g + 0.114b` at most this fraction of the median
/// luminance of the search region round the ball. Dark is the shaft.
///
/// Relative rather than absolute because the replay's exposure is the photographer's and not the
/// device's: a shaft is dark against whatever field its region holds. A starting value with no
/// measurement behind it. P13 brackets it at `[0.4, 0.8]`.
pub const DARK_RATIO: f64 = 0.6;

/// The smallest ball, as a fraction of the frame's shorter side, by equivalent diameter
/// `2·sqrt(area / π)`.
///
/// Finding 9 saw the ball ≈ 40 px across in a 1920 × 1440 frame, ≈ 0.028 of the short side, which
/// is the same fraction at the 960-wide decode P14b reads (call 14). The bracket this and
/// [`BALL_MAX_FRAC`] make is ≈ 0.36× to 2.1× of it, for a replay zoomed in or out or a photo taken
/// from further back. A fraction of the shorter side so a portrait clip, rotation applied, has the
/// bounds a landscape one has. P13 brackets it at `[0.005, 0.02]`.
///
/// **Measured at P13**, at the 960-wide decode: the replay's ball 18.2–24.4 px by equivalent
/// diameter on the five grid clips, 0.024–0.027 of the shorter side, as finding 9 said. The marks
/// that won the chain instead measured 17–26 px, so no floor in the bracket keeps them out and the
/// ball in; it stays 0.01.
pub const BALL_MIN_FRAC: f64 = 0.01;

/// The largest ball, as a fraction of the frame's shorter side; see [`BALL_MIN_FRAC`]. P13 brackets
/// it at `[0.04, 0.10]`.
pub const BALL_MAX_FRAC: f64 = 0.06;

/// The fewest consecutive frames a ball chain must last. The ball sits at rest while the club
/// comes, and the club reaches it over ≈ 4–6 frames (finding 9), so a white speck lasting one or
/// two frames is compression or the ball already in flight. P13 brackets it at `[2, 5]`.
///
/// A floor, so it cannot keep out a mark that outlasts the ball: on the five grid clips the chain
/// that won ran 32 to 58 frames, the whole clip or nearly, at every value in the bracket (P13).
pub const BALL_MIN_FRAMES: usize = 3;

/// The shortest club head, in ball diameters, by its length along its principal axis.
///
/// Finding 9 saw an iron head from above as a light bar ≈ 2–3 ball diameters long, by eye. The
/// floor sits below that for a head the replay shows a little foreshortened, and above the ball's
/// own one diameter, so the ball (or what is left of it when its disc is cleared) is never a head
/// however its aspect falls. A starting value. P13 brackets it at `[1.0, 2.0]`.
pub const HEAD_MIN_LENGTH: f64 = 1.5;

/// The longest club head, in ball diameters; see [`HEAD_MIN_LENGTH`]. Past finding 9's three with
/// room for a light hosel and ferrule lengthening the bar at the heel (the shaft beyond them is
/// dark, so it never joins). A starting value. P13 brackets it at `[4, 7]`.
pub const HEAD_MAX_LENGTH: f64 = 5.0;

/// The least length over width a club head has. An iron seen from above is a bar several times
/// longer than it is deep (finding 9, by eye; its depth was not measured), where the ball, a round
/// mark and two blurred club images lying side by side are all squat. A starting value. P13
/// brackets it at `[2.0, 3.5]`.
pub const HEAD_MIN_ASPECT: f64 = 2.5;

/// How close to the ball's centre the contact frame's head must pass, in ball diameters, measured
/// to the nearest point of the head's axis segment.
///
/// A face touching the ball holds the ball's centre half a diameter plus half the head's depth
/// from the head's axis, ≈ 0.7 for a head half a ball deep; 1.0 is that with room for a frame that
/// caught the club a little early, and the frame after the ball has gone usually shows the head
/// over where it sat, far nearer. A starting value. P13 brackets it at `[0.5, 1.5]`.
pub const CONTACT_MAX_DISTANCE: f64 = 1.0;

/// How near one end of a shaft's axis segment must lie to one end of the head's for the shaft to
/// join the head there, in ball diameters. The shaft leaves the hosel, which ends the bar at the
/// heel; half a diameter allows for where a blurred bar's extreme pixels fall. A starting value.
/// P13 brackets it at `[0.3, 1.0]`.
pub const SHAFT_MAX_JOIN: f64 = 0.5;

/// The largest offset a reading may hold, millimetres: the screen validator's plausible range for
/// `impact_offset_h` (`crates/screen/src/validate.rs`, ±60), which a reading must pass to be
/// stored as a shot's. Copied rather than imported because this crate depends on nothing.
const OFFSET_LIMIT_MM: f64 = 60.0;

/// The reading of one clip: the offset and what it was measured from, in the frames' pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImpactReading {
    /// Heel to toe, millimetres, positive toward the toe and rounded to 0.1 mm: `impact_offset_h`.
    pub offset_h_mm: f64,
    /// The frame the club head's segment passes closest to the ball's centre.
    pub contact_frame: usize,
    /// How many frames the clip had.
    pub frames: usize,
    /// The ball's centre: the median over its chain.
    pub ball_center: [f64; 2],
    /// The ball's diameter, pixels, which is the reading's scale ([`BALL_DIAMETER_MM`]).
    pub ball_diameter_px: f64,
    /// The head bar's end the shaft joins: the hosel.
    pub heel: [f64; 2],
    /// The head bar's other end.
    pub toe: [f64; 2],
}

/// Why a clip gave no reading. Each is a named refusal rather than a default (ADR-010 §2): a
/// guessed toe is a guessed sign, and a wrong offset is worse than none.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ImpactRefusal {
    /// The clip had no frames.
    NoFrames,
    /// No white disc stayed put for [`BALL_MIN_FRAMES`] consecutive frames.
    NoBall,
    /// A ball, and no frame up to one past its last held a light bar shaped like a head near it.
    NoClubHead,
    /// A head, and it never came within reach of the ball.
    NoContact,
    /// A head at the ball, and no dark shaft joined either end, so heel and toe are unknown.
    NoShaft,
    /// An offset, millimetres, past the range `impact_offset_h` may hold.
    OutOfRange(f64),
}

impl ImpactRefusal {
    /// The refusal as a golfer reads it: lower case and unpunctuated, because callers set it after
    /// a prefix (`"impact video: {sentence}"`, the M42 plan's call 13) or a clip's name.
    pub fn sentence(&self) -> String {
        match self {
            ImpactRefusal::NoFrames => "the clip held no frames to read".to_owned(),
            ImpactRefusal::NoBall => {
                "no ball could be found sitting still in the replay".to_owned()
            }
            ImpactRefusal::NoClubHead => {
                "the replay showed the ball but no club head near it".to_owned()
            }
            ImpactRefusal::NoContact => {
                "the club head never reached the ball in the replay".to_owned()
            }
            ImpactRefusal::NoShaft => "no shaft could be seen leaving the club head, so its heel \
                                       could not be told from its toe"
                .to_owned(),
            ImpactRefusal::OutOfRange(offset) => format!(
                "the strike read {offset:+.1} mm from the middle of the face, outside the \
                 {OFFSET_LIMIT_MM:.0} mm either way a reading may be"
            ),
        }
    }
}

/// Reads the impact offset from one clip's frames, in time order: the ball, the club head in each
/// frame up to one past the ball's last, the contact frame, the heel and toe from the shaft, and
/// the offset (the crate doc lists the stages).
///
/// Every frame must be one size with `rgb` holding `width × height × 3` bytes: they are one clip's,
/// decoded by one decoder, and a frame that breaks that is the caller's bug rather than the clip's.
///
/// # Panics
///
/// When a frame's buffer is not `width × height × 3` bytes, or the frames differ in size.
pub fn read_impact(frames: &[Frame]) -> Result<ImpactReading, ImpactRefusal> {
    let ball = find_ball(frames)?;
    let heads = head::heads(frames, &ball);
    let contact = contact::contact_frame(&heads, &ball)?;
    let (heel, toe) = shaft::heel_and_toe(frames, &ball, &heads, contact)?;
    let offset_h_mm = offset_mm(&ball, heel, toe);
    if offset_h_mm.abs() > OFFSET_LIMIT_MM {
        return Err(ImpactRefusal::OutOfRange(offset_h_mm));
    }
    Ok(ImpactReading {
        offset_h_mm,
        contact_frame: contact,
        frames: frames.len(),
        ball_center: ball.center,
        ball_diameter_px: ball.diameter,
        heel,
        toe,
    })
}

/// The ball's centre projected on the heel-to-toe axis, measured from the axis segment's midpoint
/// (which stands for the face centre: the crate doc's second approximation) and positive toward the
/// toe, in millimetres through the ball's diameter, rounded to 0.1 mm as call 12 writes it:
/// `round1(offset_px × BALL_DIAMETER_MM / D)` with `round1(x) = (x · 10).round() / 10`.
fn offset_mm(ball: &Ball, heel: [f64; 2], toe: [f64; 2]) -> f64 {
    let along = [toe[0] - heel[0], toe[1] - heel[1]];
    // Never zero: the ends of a head are its length apart, at least `HEAD_MIN_LENGTH` diameters.
    let length = along[0].hypot(along[1]);
    let middle = [(heel[0] + toe[0]) / 2.0, (heel[1] + toe[1]) / 2.0];
    let offset_px = ((ball.center[0] - middle[0]) * along[0]
        + (ball.center[1] - middle[1]) * along[1])
        / length;
    let rounded = (offset_px * BALL_DIAMETER_MM / ball.diameter * 10.0).round() / 10.0;
    // A strike a hair heelward of the middle rounds to −0.0, which serializes as `-0.0`; it is the
    // middle, and is written as 0.0.
    if rounded == 0.0 {
        0.0
    } else {
        rounded
    }
}

/// The median of `values`, the mean of the middle two when the count is even; `None` when empty.
/// One rule for the ball's centre and diameter and the dark class's region, so a tie between two
/// middle values is broken the same way wherever one is taken.
pub(crate) fn median(mut values: Vec<f64>) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(f64::total_cmp);
    let middle = values.len() / 2;
    Some(if values.len() % 2 == 0 {
        (values[middle - 1] + values[middle]) / 2.0
    } else {
        values[middle]
    })
}
