//! A golfer's shots judged and counted: the per-shot judgments a topic's grade is the share of
//! (M37 P2), the builder that counts them into one club's strike profile and topic grades (P6), and
//! the one that pools them over the bag into the player's (P7).
//!
//! **No Python twin, and none is coming**, for `contracts::strike`'s reason: ADR-034 §5 designed the
//! grades after the pivot, frozen Python never gains them (ADR-035 clause 4), and their vectors are
//! worked by hand (`spec/vectors/profile/hand/`, M37 P5). So nothing here carries a name over from
//! the Python package, and the tests hold each rule to the M37 plan's decisions rather than to an
//! oracle.
//!
//! **The judgments and the counting are kept apart.** Each judgment answers for one shot and knows
//! nothing of the others; [`build_club_grades`] decides which shots a topic sees, takes the club's
//! typical shot, and turns the answers into shares. That is the crate's measure/judge split one
//! level up: each rule is tested at its own edge once, so a builder counting wrongly cannot hide a
//! rule judging wrongly, or the reverse.
//!
//! # Through the analysed view, by type
//!
//! Every judgment reads its shot as an [`AnalysedShot`], whose one constructor is
//! `capability::analysed_view`. So any reading outside the shot's printed ∩ analysed set is `None`
//! before a rule sees it: a field the device does not print (HD Golf's impact offsets, its low point)
//! cannot be graded, and neither can one it prints only to show (HD Golf's club speed). That is
//! ADR-034 §2 held by the compiler rather than by each judgment remembering, and it is why the
//! strike-location topic is absent on every stored shot until M42 with no line here saying so.
//!
//! # `None` is "cannot judge", never "missed"
//!
//! A judgment whose input holds no value answers `None`, and P6 counts that shot as unscored for the
//! topic, under its reason and outside the share's denominator (ADR-010 §2; the M37 plan's call 8).
//! A `false` is a judged miss. They are different values here so that no builder can add one to the
//! other.
//!
//! # Every edge is inclusive, and every tolerance is read rather than restated
//!
//! At its boundary a shot is good: centered at exactly 10 mm, straight at exactly ±2.0°, on line at
//! exactly ±9.0 yd, consistent at exactly ±10%. One rule for all four, so a hand-worked vector at an
//! edge needs nothing looked up. Low point has no tolerance to be inclusive of: "ahead of the ball"
//! is `> 0`, because a low point at the ball is not ahead of it (decision 7). The straight and on-line
//! tolerances are `dispersion::METRIC_TARGETS` rows read through `target_for`, so there is no second
//! 2.0 or 9.0 here to drift from them.
//!
//! **Nothing here reads handedness.** Face-to-path is golfer-relative, so a face open to the path is
//! a fade for either hand (the M37 plan's finding 5, correcting §M37's "the sign flips"), and which
//! side of the line a left-hander's fade finishes on is already inside the projected landing.
//!
//! # The player: every shot judged once, by its own club's criterion
//!
//! [`build_player_grades`] does not add the clubs' grades up. A withheld club share carries no
//! successes to add, and a photo filed under two clubs (a tagging slip) would be counted twice, so
//! the player's `judged` could pass its `n_shots`. It judges the whole corpus once instead, each
//! photo on its first swing as a club's grade does, and judges each shot by the **criterion its
//! owner's own shots set**: which topics exist for that club, its category, and its typical shot
//! (ADR-034 §5.4). So a 7 iron that is consistent against the 7 iron's median pools as consistent,
//! whatever the driver's median would say, and on a corpus with no tagging slip the player's
//! club-bound topics are exactly the clubs' sums. The untagged shots are an owner of their own,
//! with no club, so they reach strike location and the shapes and nothing else (the M37 plan's
//! call 9).

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use contracts::baseline::{minimum_n, BaselineClaim, Interval};
use contracts::capability::{analysed_on, analysed_view, fields_holding_a_value};
use contracts::career::{CareerCorpus, CorpusSwing, Narrowing};
use contracts::club::{category_of, ClubId};
use contracts::club_profile::{BagProfile, ClubProfile};
use contracts::dispersion::{target_for, MetricDispersion};
use contracts::intent::{ClubCategory, PracticeMode, TargetShape};
use contracts::shot::ShotData;
use contracts::strike::{
    is_club_independent, low_point_graded_for, Blend, ClubGrades, FaceColumn, FaceRow, HeelToe,
    ImpactCell, ImpactMap, OfflineBasis, PlayerGrades, ShapeShares, Share, StrikeProfile, Topic,
    TopicGrade, UnscoredCount, WithheldShare, BALL_SPEED_METRIC, CENTERED_RADIUS_MM,
    CONSISTENCY_FRACTION, ON_LINE_TOLERANCE_ROW, RATE_MINIMUM_N, STRAIGHT_TOLERANCE_ROW,
    STRIKE_DISPERSION_METRICS,
};
use contracts::swing::Measurement;
use contracts::unscored::{UnscoredCheckpoint, UnscoredReason};
use contracts::ContractError;

use crate::dispersion::build_dispersion;
use crate::shot_measure::{
    measure_carry_distance, measure_face_to_path, measure_start_line_offline,
};
use crate::stats::{mean_and_sd, mean_ci, percentile, wilson};

/// The flight's curved landing offline, ADR-027's projection: the measurement row a landing is
/// projected from when nothing printed one (the M37 plan's call 6). Recorded only when the curve
/// was drawn, which needs a printed spin axis (`crate::flight_measure`'s module doc).
pub const PROJECTED_LANDING_ROW: &str = "flight_landing_offline_yds";

/// The name a refused flight's one `unscored` entry carries, for all six flight numbers at once
/// (`crate::flight_measure::flight_unscored`). Its reason is why a shot whose flight never flew has
/// no projected landing.
pub const FLIGHT_REFUSAL_ROW: &str = "flight_carry_yds";

/// The `ShotData` key a device prints a landing offline under, and so the name its tile refusal
/// (`printed_blank`, `misread`) carries in the artifact's `unscored`.
pub const PRINTED_LANDING_FIELD: &str = "carry_offline";

// --- The view -----------------------------------------------------------------------------------

/// A shot as analysis may judge it: every reading outside `capability::analysed_on` already `None`.
///
/// A newtype rather than a `ShotData` each judgment promises to view first, because a promise each
/// judgment makes is one the next judgment added can forget. Its field is private and
/// [`AnalysedShot::of`] is the only way to build one, so no judgment here can be handed a raw shot.
#[derive(Debug, Clone, PartialEq)]
pub struct AnalysedShot(ShotData);

impl AnalysedShot {
    /// The analysed view of `shot` (`capability::analysed_view`).
    ///
    /// Refuses a shot whose device `devices.json` does not declare, as the view does: a shot from a
    /// device nobody declared could print anything, so nothing about it may be graded.
    pub fn of(shot: &ShotData) -> Result<Self, ContractError> {
        analysed_view(shot).map(Self)
    }

    /// The viewed shot, every reading analysis may not use already blank.
    pub fn shot(&self) -> &ShotData {
        &self.0
    }
}

// --- Where on the face --------------------------------------------------------------------------

/// Did the strike land within [`CENTERED_RADIUS_MM`] of face center? (Decision 4.)
///
/// `hypot(h, v) ≤ 10` where both offsets hold values, `|h| ≤ 10` where only the horizontal does: a
/// device that prints heel-to-toe alone is judged on the axis it prints rather than assumed centered
/// on the other. `None` without a horizontal offset, which is strike location's one required input
/// (`Topic::StrikeLocation.inputs()`), so a vertical offset alone judges nothing.
///
/// `f64::hypot` rather than `pyfmt::hypot`: that one is CPython's three-argument norm, ported to the
/// bit for a recorded answer, and there is no recorded answer here.
pub fn is_centered(shot: &AnalysedShot) -> Option<bool> {
    let horizontal = shot.0.impact_offset_h?;
    let from_center = match shot.0.impact_offset_v {
        Some(vertical) => horizontal.hypot(vertical),
        None => horizontal.abs(),
    };
    Some(from_center <= CENTERED_RADIUS_MM)
}

/// Which cell of the impact grid the strike landed in (the M37 plan's call 10): the column by the
/// horizontal offset, and the row by the vertical one, or no row when the vertical holds no value.
///
/// Each axis is binned at ±[`CENTERED_RADIUS_MM`], **inclusive in the center**, so "heel" and "toe"
/// mean beyond the radius as `HeelToe`'s counts do. The center cell is therefore a square where
/// [`is_centered`] is a circle, and a strike at (9, 9) mm sits in the center cell without being
/// centered (`hypot` is 12.7). On purpose: the grid is the picture of where, and the rate is the
/// judgment (`contracts::strike::ImpactMap`'s doc).
///
/// `None` without a horizontal offset, for [`is_centered`]'s reason.
pub fn impact_cell(shot: &AnalysedShot) -> Option<(FaceColumn, Option<FaceRow>)> {
    let column = match bin(shot.0.impact_offset_h?) {
        Ordering::Less => FaceColumn::Heel,
        Ordering::Equal => FaceColumn::Center,
        Ordering::Greater => FaceColumn::Toe,
    };
    let row = shot.0.impact_offset_v.map(|vertical| match bin(vertical) {
        Ordering::Less => FaceRow::Low,
        Ordering::Equal => FaceRow::Center,
        Ordering::Greater => FaceRow::High,
    });
    Some((column, row))
}

/// An offset against the grid's one binning rule: beyond the radius below it, within it (inclusive),
/// or beyond it above. One function for both axes, so the two cannot bin differently.
fn bin(offset_mm: f64) -> Ordering {
    if offset_mm < -CENTERED_RADIUS_MM {
        Ordering::Less
    } else if offset_mm > CENTERED_RADIUS_MM {
        Ordering::Greater
    } else {
        Ordering::Equal
    }
}

// --- Low point ----------------------------------------------------------------------------------

/// Was the swing's low point ahead of the ball? `low_point > 0` inches, `+` = ahead (decision 7).
///
/// **Strictly ahead**: a low point at 0 is at the ball, which is the thin-or-fat margin the topic
/// exists to grade, so it is a miss rather than a pass.
///
/// `None` without a low point, and `None` for a club category low point is not graded for
/// ([`low_point_graded_for`]: a driver or a wood, where hitting up is correct, a putter, or no club).
/// P6 never asks about those, since the topic is absent for them; answering `None` rather than
/// `false` means that a builder that did ask would count an unscored shot, never a miss.
pub fn low_point_ahead(shot: &AnalysedShot, category: ClubCategory) -> Option<bool> {
    if !low_point_graded_for(category) {
        return None;
    }
    Some(shot.0.low_point? > 0.0)
}

// --- Shape --------------------------------------------------------------------------------------

/// The shape this shot was hit, by face-to-path against [`STRAIGHT_TOLERANCE_ROW`]'s tolerance (the
/// M37 plan's call 5): straight within it, **inclusive**, a fade beyond it on the `+` side (face
/// open to the path), a draw beyond it on the `−` side.
///
/// Face-to-path is `shot_measure::measure_face_to_path` over the view, the function the
/// `face_to_path_deg` row is recorded by, so the shape and the row cannot disagree about which
/// angle is subtracted from which. `None` when either angle holds no value.
///
/// For either hand, as the module doc says: the sign is golfer-relative.
pub fn shape_of(shot: &AnalysedShot) -> Option<TargetShape> {
    let face_to_path = measure_face_to_path(&shot.0)?;
    let (target, tolerance) = target_and_tolerance(STRAIGHT_TOLERANCE_ROW);
    let open_by = face_to_path - target;
    Some(if open_by > tolerance {
        TargetShape::Fade
    } else if open_by < -tolerance {
        TargetShape::Draw
    } else {
        TargetShape::Straight
    })
}

// --- Where it landed ----------------------------------------------------------------------------

/// A shot's landing offline and where it came from (the M37 plan's call 6).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Landing {
    /// Yards right (`+`) or left of the target line at the landing.
    pub offline_yds: f64,
    /// Printed or projected. A report names a projected one as projected wherever it shows it.
    pub basis: OfflineBasis,
}

/// Where this shot landed, off the line, or the reason nothing says (the M37 plan's call 6).
///
/// In order, the first that holds:
///
/// 1. **Printed**: the view's `carry_offline`, which holds a value only where `capability` names it
///    printed and analysed on this shot. Used wherever it exists, so a device that starts printing
///    one replaces the projection with no change here (ADR-034 §5.3).
/// 2. **Projected**: the artifact's [`PROJECTED_LANDING_ROW`] measurement, the flight model's curved
///    landing, ADR-034 §5.3's one sanctioned inference. Matched by name alone, because a name has
///    one provenance (`crate::flight_measure`'s rule), so a second source cannot hide under it.
/// 3. **None**, as the reason the artifact's shot-side `unscored` gives: the printed tile's refusal
///    first ([`PRINTED_LANDING_FIELD`]: `printed_blank`, `misread`), since a device that prints a
///    landing makes that tile the landing and the projection only a stand-in for it; then the
///    projection's own ([`PROJECTED_LANDING_ROW`]: `spin_axis_unresolved`, `no_handedness`); then a
///    refused flight's ([`FLIGHT_REFUSAL_ROW`]). **`Unrecorded`** when none of them is there, which
///    is ADR-010 §2's "this artifact does not know" rather than a cause invented for it.
///
/// A shot answered `Err` leaves a shape topic's on-line share and is counted under its reason; it
/// stays in "hit the shape" (call 6).
pub fn offline_basis(
    shot: &AnalysedShot,
    measurements: &[Measurement],
    shot_unscored: &[UnscoredCheckpoint],
) -> Result<Landing, UnscoredReason> {
    if let Some(offline_yds) = shot.0.carry_offline {
        return Ok(Landing {
            offline_yds,
            basis: OfflineBasis::Printed,
        });
    }
    if let Some(row) = measurements
        .iter()
        .find(|row| row.name == PROJECTED_LANDING_ROW)
    {
        return Ok(Landing {
            offline_yds: row.value,
            basis: OfflineBasis::Projected,
        });
    }
    let named = |name: &str| {
        shot_unscored
            .iter()
            .find(|entry| entry.name == name)
            .map(|entry| entry.reason)
    };
    Err(named(PRINTED_LANDING_FIELD)
        .or_else(|| named(PROJECTED_LANDING_ROW))
        .or_else(|| named(FLIGHT_REFUSAL_ROW))
        .unwrap_or(UnscoredReason::Unrecorded))
}

/// Did the landing finish on line? Within [`ON_LINE_TOLERANCE_ROW`]'s ±9 yd of the target line,
/// **inclusive** (decision 8, ADR-034 §5.3's default confirmed for a landing knowing it is generous
/// for a wedge and tight for a driver).
pub fn on_line(offline_yds: f64) -> bool {
    let (target, tolerance) = target_and_tolerance(ON_LINE_TOLERANCE_ROW);
    (offline_yds - target).abs() <= tolerance
}

// --- Consistency --------------------------------------------------------------------------------

/// Where a shot went, as consistency reads it: how far it carried, and how far right (`+`) or left of
/// the line it **started**, projected to that carry (`start_line_offline_yds`, not the landing).
///
/// Also the shape of a club's typical shot, which is the median of each over the club's non-mishit
/// judged shots (the M37 plan's call 4, built in P6): two medians taken apart, not the shot nearest
/// both.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CarryAndOffline {
    pub carry_yds: f64,
    pub offline_yds: f64,
}

/// This shot's carry and start-line offline, through the view, or `None` when either is missing.
///
/// Read by `shot_measure`'s own `measure_carry_distance` and `measure_start_line_offline`, which are
/// what record the `carry_distance_yds` and `start_line_offline_yds` rows, so a shot's consistency is
/// judged on exactly the numbers its dispersion pools. Consistency's inputs
/// (`Topic::Consistency.inputs()`) are carry and the launch direction the offline is projected from.
pub fn carry_and_offline(shot: &AnalysedShot) -> Option<CarryAndOffline> {
    Some(CarryAndOffline {
        carry_yds: measure_carry_distance(&shot.0)?,
        offline_yds: measure_start_line_offline(&shot.0)?,
    })
}

/// Was this shot like the club's typical one? (Decisions 1 and 6, call 4.)
///
/// Its carry within [`CONSISTENCY_FRACTION`] of the typical carry, **and** its start-line offline
/// within the same fraction *of the typical carry* of the typical offline, both **inclusive**. One
/// window in yards, scaled by the club: ±15 yd for a 150-yard 7 iron on both axes, ±25 yd for a
/// 250-yard driver.
///
/// **A mishit is a miss whatever its numbers** (decision 6). It stays in the club's judged count and
/// fails here, so an outlier lowers the grade, which is what the user asked of it. Its numbers are
/// still left out of the typical shot (call 4), so the outlier the grade punishes does not also move
/// the yardstick.
pub fn is_consistent(shot: CarryAndOffline, typical: CarryAndOffline, mishit: bool) -> bool {
    if mishit {
        return false;
    }
    let window_yds = CONSISTENCY_FRACTION * typical.carry_yds;
    (shot.carry_yds - typical.carry_yds).abs() <= window_yds
        && (shot.offline_yds - typical.offline_yds).abs() <= window_yds
}

// --- One club's grades: the builder (P6) --------------------------------------------------------

/// The `ShotData` key strike location reads, and so the key `baseline::minimum_n` floors the heel/toe
/// lean at. `Topic::StrikeLocation.inputs()` names the same one, and a test holds the two equal.
pub const IMPACT_OFFSET_H: &str = "impact_offset_h";

/// One club's strike profile, the grades of every topic that exists for it, and their blend.
/// [M37 P6]
///
/// `corpus` is the golfer's corpus, whole or already narrowed to the club: it is narrowed to
/// `profile.club` here either way, as `club_profile::build_bag_profile` narrows before it builds,
/// and for its reason (a club graded on the bag's shots is a 7 iron judged by drives). `profile` is
/// that club's `ClubProfile` from the same corpus, and the ball-speed baseline and the three
/// dispersions are **copied from it, never recomputed** (the M37 plan's finding 11).
///
/// # Which shots a topic sees
///
/// - **A shot is a candidate** when its swing counts toward metrics (analyzed, not stale, not
///   outdated), its parse was not flagged, it is tracked, and it carries its shot and its photo's
///   hash (the M37 plan's call 8). **Each photo is judged once**, on the first swing carrying it in
///   corpus order: the dedupe `baseline::pooled_samples` keys a launch-monitor sample on, so a
///   grade's `judged` and the baseline's `n` count the same photos.
/// - **A topic exists** when every one of its inputs is analysed on at least one candidate, the union
///   `capability::printed_fields` takes; low point only for a club it is graded for; a shape only
///   when a candidate was declared `shot_shaping` with that shape (call 7). A topic that does not
///   exist is not in `topics` at all.
/// - **Where it exists, each candidate is judged, unscored, or not seen.** An input not analysed on
///   this shot (another layout's screen, or a tile the parse never located) is not seen: neither a
///   judgment nor a refusal. Every input analysed and the judgment answering is a judged shot. Every
///   input analysed and the judgment answering `None` means a tile printed and holding no value,
///   which is **unscored** under the reason the artifact named for that tile, else `printed_blank`,
///   and never a miss (ADR-010 §2).
///
/// "Analysed" rather than "printed" (`capability::analysed_on`, not `printed_on`): the two agree on
/// every input for every declared device today, and where a device someday prints a judged reading
/// only to show it, the topic is absent rather than every shot unscored as blank, since the view
/// blanks that reading on every shot (ADR-034 §2: shown-only is never graded).
///
/// # The typical shot
///
/// Consistency judges each shot against the club's typical one: the median carry and, apart, the
/// median start-line offline of its **non-mishit** judged shots (call 4), so the outliers the grade
/// punishes do not also move the yardstick. A club whose every judged shot is a mishit has no
/// typical shot and needs none: a mishit is a miss before the window is read (decision 6), so its
/// grade is 0 of n rather than a club with its shots dropped.
///
/// # What is never read
///
/// No pose measurement, and no handedness. The only measurement row any topic reads is the flight's
/// projected landing ([`PROJECTED_LANDING_ROW`]); every other input is a tile of the shot itself,
/// through the analysed view. So mechanics enters no grade (Decision 12), and a test changes every
/// pose row of a corpus to say so.
///
/// A shot from a device `devices.json` does not declare is not a candidate, since nothing about it may
/// be graded ([`AnalysedShot::of`]), and it still counts in `n_shots`, which is evidence and not a
/// judgment.
pub fn build_club_grades(corpus: &CareerCorpus, profile: &ClubProfile) -> ClubGrades {
    let club = profile.club;
    let narrowed = corpus.narrowed_to(&Narrowing {
        club: Some(club),
        ..Narrowing::default()
    });
    let shots = candidates(&narrowed.swings);
    let owner = Criteria::of(&shots, Some(club));
    let owned: Vec<Owned> = shots
        .iter()
        .map(|shot| Owned {
            shot,
            owner: &owner,
        })
        .collect();
    let tally = PoolTally::of(owner.topics.iter().copied(), &owned);
    let topics: Vec<TopicGrade> = tally.topics.iter().map(Tally::grade).collect();
    ClubGrades {
        club,
        n_shots: count(narrowed.distinct_shots()),
        strike: StrikeProfile {
            impacts: tally.impact_map(),
            ball_speed: profile.metrics.get(BALL_SPEED_METRIC).cloned(),
            dispersion: carried_dispersion(profile),
        },
        blend: blend_of(&topics),
        topics,
    }
}

/// The player's grades: each topic's share pooled over every tracked shot in the bag, the untagged
/// ones included, each shot judged by its own club's criterion (ADR-034 §5.4; decision 11). [M37 P7]
///
/// # How the pool is drawn
///
/// - **Each owner's criterion comes from its own swings alone**: every club's from the swings
///   tagged with it, which is the corpus [`build_club_grades`] narrows to, and the untagged swings'
///   from theirs. The criterion is which topics exist for that owner (the M37 plan's call 7), the
///   club's category for low point, and its typical shot for consistency (call 4). So a shot pools
///   as its club's grade judged it, and a 7 iron consistent against the 7 iron's median is
///   consistent here though the driver's median would fail it.
/// - **The untagged swings are an owner with no club** (call 9). Consistency and low point need a
///   club ([`Topic::needs_a_club`]), so they never exist for that owner and its shots are not judged
///   on them; strike location and a shape exist for it by the rule a club's do, so a shape needs
///   an untagged shot declared that shape before an untagged shot is judged on it. How many photos
///   it holds is `untagged_shots`.
/// - **A topic exists for the player when it exists for any owner**, and each shot is judged on it
///   only when it exists for that shot's own owner: a driver that prints a low point lends the
///   player nothing on low point, because the driver's own grade has no such topic (decision 7).
/// - **The whole corpus is judged once**, each photo on its first candidate swing in corpus order
///   ([`build_club_grades`]' rule), rather than the clubs' tallies added. Added, a photo filed under
///   two clubs would count twice and the player could judge more shots than `n_shots`; and a
///   withheld club share has no successes to add. Judged once, the heel/toe lean is also taken over
///   the offsets in corpus order, the order `stats::mean_and_sd`'s compensated sum is defined over,
///   rather than club by club.
///
/// # The raw numbers are club-independent only
///
/// No ball speed (it scales with the club), and the dispersion of only those
/// [`STRIKE_DISPERSION_METRICS`] that [`is_club_independent`] passes, taken from
/// `dispersion::build_dispersion` over the **whole** corpus (ADR-034 §4; the M37 plan's call 17). A
/// driver-plus-wedge carry spread describes nobody, so it is not here to be read.
///
/// The player's blend is the club's rule over the player's own topics, never an average of the
/// clubs' blends. Like a club's grades, nothing here reads a pose measurement or a handedness.
pub fn build_player_grades(corpus: &CareerCorpus) -> PlayerGrades {
    let owners: BTreeMap<Option<ClubId>, Criteria> = corpus
        .swings
        .iter()
        .map(|swing| swing.club)
        .collect::<BTreeSet<Option<ClubId>>>()
        .into_iter()
        .map(|club| {
            let own = corpus.swings.iter().filter(|swing| swing.club == club);
            (club, Criteria::of(&candidates(own), club))
        })
        .collect();
    let shots = candidates(&corpus.swings);
    let owned: Vec<Owned> = shots
        .iter()
        .map(|shot| Owned {
            shot,
            owner: &owners[&shot.swing.club],
        })
        .collect();
    let topics = Topic::ALL
        .into_iter()
        .filter(|topic| owners.values().any(|owner| owner.has(*topic)));
    let tally = PoolTally::of(topics, &owned);
    let topics: Vec<TopicGrade> = tally.topics.iter().map(Tally::grade).collect();
    let untagged_shots = corpus
        .swings
        .iter()
        .filter(|swing| swing.club.is_none())
        .filter_map(|swing| swing.shot_sha256.as_deref())
        .collect::<BTreeSet<&str>>()
        .len();
    PlayerGrades {
        n_shots: count(corpus.distinct_shots()),
        untagged_shots: count(untagged_shots),
        strike: StrikeProfile {
            impacts: tally.impact_map(),
            ball_speed: None,
            dispersion: player_dispersion(corpus),
        },
        blend: blend_of(&topics),
        topics,
    }
}

/// Every club's grades and the player's: the grades half of a `GradedProfile`, which
/// `core::profile::run_profile` (M37 P9) joins to the strengths and weaknesses lists, since only
/// `core` holds `feedback` beside this crate.
///
/// No wire shape of its own. What is written is the `GradedProfile`, and a second serialized
/// spelling of the same grades would be a second thing for the phone to read.
#[derive(Debug, Clone, PartialEq)]
pub struct ProfileGrades {
    /// One per club with a shot photo, in bag order.
    pub clubs: Vec<ClubGrades>,
    pub player: PlayerGrades,
}

/// Each club's grades and the player's, from one corpus and the `BagProfile` built from it.
/// [M37 P7]
///
/// **Only a club with a shot photo is graded** (the M37 plan's P5 finding 5): `build_bag_profile`
/// also profiles a club the bag declares and the golfer never hit, which has a loft worth showing
/// and nothing to grade, so it is not a club of the profile at all rather than a club of absent
/// topics. A club hit only on clips with no photo is left out the same way, since every grade is a
/// share of photos. The order is the `BagProfile`'s, which is bag order.
///
/// `bag` must be built from this `corpus`: each club's ball speed and dispersions are copied from
/// it ([`build_club_grades`]), so a profile of another corpus would pair one golfer's spread with
/// another's grades.
pub fn build_grades(corpus: &CareerCorpus, bag: &BagProfile) -> ProfileGrades {
    ProfileGrades {
        clubs: bag
            .clubs
            .iter()
            .filter(|profile| profile.n_shots > 0)
            .map(|profile| build_club_grades(corpus, profile))
            .collect(),
        player: build_player_grades(corpus),
    }
}

/// The club's dispersion of [`STRIKE_DISPERSION_METRICS`], copied whole from its `ClubProfile`, for
/// those its corpus holds.
fn carried_dispersion(profile: &ClubProfile) -> BTreeMap<String, MetricDispersion> {
    STRIKE_DISPERSION_METRICS
        .iter()
        .filter_map(|metric| {
            profile
                .dispersion
                .get(*metric)
                .map(|dispersion| (metric.to_string(), dispersion.clone()))
        })
        .collect()
}

/// The player's dispersion of the club-independent [`STRIKE_DISPERSION_METRICS`], for those the
/// corpus holds: `build_dispersion` over the whole corpus, kept to what means something across a
/// bag. The filter reads `contracts::strike::STAT_SCOPES` rather than naming the two angles, so a
/// metric whose scope changes moves here with no edit.
fn player_dispersion(corpus: &CareerCorpus) -> BTreeMap<String, MetricDispersion> {
    let mut whole = build_dispersion(corpus).metrics;
    STRIKE_DISPERSION_METRICS
        .iter()
        .filter(|metric| is_club_independent(metric))
        .filter_map(|metric| {
            whole
                .remove(*metric)
                .map(|dispersion| (metric.to_string(), dispersion))
        })
        .collect()
}

/// The equal-weight mean of the graded topics' shares (the M37 plan's call 11), or `None` when
/// nothing is graded: a blend of nothing is absent, never 0 (ADR-034 §5.5).
///
/// Summed left to right in topic order and divided by the count, the arithmetic the hand vectors
/// were worked in. The withheld topics are named as excluded; an absent topic is in neither list,
/// because it is not in `topics` to begin with. Built here for the club because `ClubGrades` does
/// not validate without it; the player's blend is the same function over the player's own topics.
fn blend_of(topics: &[TopicGrade]) -> Option<Blend> {
    let graded: Vec<(Topic, f64)> = topics
        .iter()
        .filter_map(|grade| grade.grade.graded().map(|rate| (grade.topic, rate.share)))
        .collect();
    if graded.is_empty() {
        return None;
    }
    let mut sum = 0.0;
    for (_, share) in &graded {
        sum += share;
    }
    Some(Blend {
        mean: sum / graded.len() as f64,
        averaged: graded.iter().map(|(topic, _)| *topic).collect(),
        excluded: topics
            .iter()
            .filter(|grade| grade.grade.graded().is_none())
            .map(|grade| grade.topic)
            .collect(),
    })
}

/// A share over `n` judged shots, or withheld below [`RATE_MINIMUM_N`] (decision 2).
fn share(successes: usize, n: usize) -> Share {
    if count(n) < RATE_MINIMUM_N {
        return Share::Withheld(WithheldShare {
            have_n: count(n),
            need_n: RATE_MINIMUM_N,
        });
    }
    Share::Graded(wilson(successes, n).expect("a share at the floor is over at least one shot"))
}

/// A shot a topic may see (the M37 plan's call 8), with what its device analyses on it.
struct Candidate<'a> {
    swing: &'a CorpusSwing,
    view: AnalysedShot,
    /// `capability::analysed_on` of the raw shot: printed on its screen and declared for analysis.
    analysed: BTreeSet<&'static str>,
}

impl Candidate<'_> {
    /// Is every input of `topic` analysed on this shot? Where not, the topic does not see it.
    fn sees(&self, topic: Topic) -> bool {
        topic
            .inputs()
            .iter()
            .all(|input| self.analysed.contains(input))
    }

    /// Why a topic whose inputs are all analysed here could not judge this shot: the reason the
    /// artifact named for the first input holding no value, else a plain `printed_blank`.
    ///
    /// The screen's tile refusals are named by the `ShotData` key (`screen::tile_refusals`), so the
    /// input's key is the lookup. An unstamped screen shot never reaches here, since it prints only
    /// what it holds, so a blank is a stamped read's, or a direct feed's missing value (a shot with
    /// no provenance prints everything its device declares).
    fn blank_reason(&self, topic: Topic) -> UnscoredReason {
        let held = fields_holding_a_value(self.view.shot());
        topic
            .inputs()
            .iter()
            .filter(|input| !held.contains(**input))
            .find_map(|input| {
                self.swing
                    .shot_unscored
                    .iter()
                    .find(|entry| entry.name == *input)
                    .map(|entry| entry.reason)
            })
            .unwrap_or(UnscoredReason::PrintedBlank)
    }

    /// Was this shot hit as `shape` was declared, `shot_shaping` with that `target_shape`?
    fn declared(&self, shape: TargetShape) -> bool {
        self.swing.intent.as_ref().is_some_and(|goal| {
            goal.mode == PracticeMode::ShotShaping && goal.target_shape == Some(shape)
        })
    }
}

/// The candidates among `swings`, in corpus order, each photo once.
fn candidates<'a>(swings: impl IntoIterator<Item = &'a CorpusSwing>) -> Vec<Candidate<'a>> {
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    swings
        .into_iter()
        .filter(|swing| {
            swing.counts_toward_metrics()
                && !swing.shot_needs_review
                && swing
                    .intent
                    .as_ref()
                    .is_none_or(|goal| goal.mode.is_tracked())
        })
        .filter_map(|swing| {
            let shot = swing.shot.as_ref()?;
            let photo = swing.shot_sha256.as_deref()?;
            if !seen.insert(photo) {
                return None;
            }
            Some(Candidate {
                swing,
                view: AnalysedShot::of(shot).ok()?,
                analysed: analysed_on(shot).ok()?,
            })
        })
        .collect()
}

/// One judged strike: where on the grid, whether centered, and the horizontal offset the lean is the
/// mean of.
struct Strike {
    centered: bool,
    cell: (FaceColumn, Option<FaceRow>),
    offset_h_mm: f64,
}

/// One topic's counts before any share is taken.
struct Tally {
    topic: Topic,
    judged: usize,
    /// Judged shots meeting the criterion; for a shape, the shape hit **and** on line, among the
    /// judged shots with a landing.
    successes: usize,
    unscored: Vec<UnscoredCount>,
    /// Present on a shape topic and only there.
    shape: Option<ShapeTally>,
}

/// A shape topic's other counts (ADR-034 §5.3; the M37 plan's call 6).
#[derive(Default)]
struct ShapeTally {
    /// Judged shots classified as the declared shape.
    hit: usize,
    printed: usize,
    projected: usize,
    /// Judged shots with no landing, by reason: out of the grade's denominator, in `hit`'s.
    landing_unscored: Vec<UnscoredCount>,
}

impl Tally {
    fn new(topic: Topic) -> Self {
        Tally {
            topic,
            judged: 0,
            successes: 0,
            unscored: Vec::new(),
            shape: topic.shape().map(|_| ShapeTally::default()),
        }
    }

    /// The grade these counts are: each share withheld below the floor, and a shape's on-line share
    /// over the judged shots that have a landing.
    fn grade(&self) -> TopicGrade {
        let (grade, shape) = match &self.shape {
            None => (share(self.successes, self.judged), None),
            Some(shape) => (
                share(self.successes, shape.printed + shape.projected),
                Some(ShapeShares {
                    hit_shape: share(shape.hit, self.judged),
                    printed: count(shape.printed),
                    projected: count(shape.projected),
                    landing_unscored: shape.landing_unscored.clone(),
                }),
            ),
        };
        TopicGrade {
            topic: self.topic,
            judged: count(self.judged),
            grade,
            unscored: self.unscored.clone(),
            shape,
        }
    }
}

/// Add one shot to `counts` under `reason`, keeping each reason once and in `UnscoredReason`
/// declaration order, the order `contracts::strike` checks.
fn count_under(counts: &mut Vec<UnscoredCount>, reason: UnscoredReason) {
    match counts.iter_mut().find(|counted| counted.reason == reason) {
        Some(counted) => counted.n += 1,
        None => {
            counts.push(UnscoredCount { reason, n: 1 });
            counts.sort_by_key(|counted| counted.reason as usize);
        }
    }
}

/// What an owner's own shots decide about judging any one of them: which topics exist for it
/// (call 7), its club's category for low point (decision 7), and its typical shot for consistency
/// (call 4). An owner is a club, or at player level the untagged shots, which is why the club is
/// optional and a topic needing one is absent without it ([`Topic::needs_a_club`]).
///
/// Taken from the owner's candidates alone and then fixed, so a shot the player pools is judged by
/// exactly what its club's grade judged it by (ADR-034 §5.4).
struct Criteria {
    category: Option<ClubCategory>,
    /// The topics that exist for this owner, in [`Topic::ALL`] order.
    topics: Vec<Topic>,
    /// The median carry and median start-line offline of the owner's non-mishit consistency shots;
    /// `None` when consistency does not exist for it, or every such shot is a mishit.
    typical: Option<CarryAndOffline>,
}

impl Criteria {
    fn of(shots: &[Candidate], club: Option<ClubId>) -> Self {
        let category = club.map(category_of);
        let topics: Vec<Topic> = Topic::ALL
            .into_iter()
            .filter(|topic| exists(*topic, shots, category))
            .collect();
        // The shots consistency judges: those that see it and hold both numbers. A mishit among them
        // is judged and missed, and kept out of the median (call 4).
        let typical = if topics.contains(&Topic::Consistency) {
            typical_shot(
                shots
                    .iter()
                    .filter(|shot| shot.sees(Topic::Consistency) && !shot.swing.is_mishit())
                    .filter_map(|shot| carry_and_offline(&shot.view)),
            )
        } else {
            None
        };
        Criteria {
            category,
            topics,
            typical,
        }
    }

    fn has(&self, topic: Topic) -> bool {
        self.topics.contains(&topic)
    }
}

/// A candidate beside the criteria of the owner it is judged by.
#[derive(Clone, Copy)]
struct Owned<'c, 'a> {
    shot: &'c Candidate<'a>,
    owner: &'c Criteria,
}

/// What a pool of shots adds up to, topic by topic: one club's, where every shot shares the club's
/// criteria, or the player's, where each keeps its own owner's.
struct PoolTally {
    /// The topics that exist for the pool, in [`Topic::ALL`] order.
    topics: Vec<Tally>,
    /// Every judged strike, in corpus order, when strike location exists; `None` when it does not,
    /// which is what keeps the impact map and the topic present together.
    strikes: Option<Vec<Strike>>,
}

impl PoolTally {
    /// `topics` are those that exist for the pool, in topic order; `shots` the pool in corpus order.
    /// Each topic sees only the shots whose owner it exists for.
    fn of(topics: impl IntoIterator<Item = Topic>, shots: &[Owned]) -> Self {
        let mut tally = PoolTally {
            topics: Vec::new(),
            strikes: None,
        };
        for topic in topics {
            let shots: Vec<Owned> = shots
                .iter()
                .copied()
                .filter(|shot| shot.owner.has(topic))
                .collect();
            let counted = match topic {
                Topic::StrikeLocation => {
                    let (counted, strikes) = tally_strikes(&shots);
                    tally.strikes = Some(strikes);
                    counted
                }
                Topic::Consistency => tally_consistency(&shots),
                Topic::LowPoint => tally_low_point(&shots),
                Topic::Fade | Topic::Draw | Topic::Straight => tally_shape(topic, &shots),
            };
            tally.topics.push(counted);
        }
        tally
    }

    /// The grid and the lean of the judged strikes, present exactly when strike location exists.
    fn impact_map(&self) -> Option<ImpactMap> {
        self.strikes.as_deref().map(impact_map)
    }
}

/// Does `topic` exist for an owner whose candidates are `shots` (the M37 plan's call 7)?
fn exists(topic: Topic, shots: &[Candidate], category: Option<ClubCategory>) -> bool {
    let analysed: BTreeSet<&str> = shots
        .iter()
        .flat_map(|shot| shot.analysed.iter().copied())
        .collect();
    let printed = topic.inputs().iter().all(|input| analysed.contains(input));
    let owner_can = match (topic, category) {
        (_, None) if topic.needs_a_club() => false,
        (Topic::LowPoint, Some(category)) => low_point_graded_for(category),
        _ => true,
    };
    let declared = topic
        .shape()
        .is_none_or(|shape| shots.iter().any(|shot| shot.declared(shape)));
    printed && owner_can && declared
}

/// Each shot `topic` sees among `shots`, judged by `judge`: the answers in corpus order, with the
/// shots whose input was printed blank counted under their reasons in `tally.unscored`.
fn judge_each<'c, 'a, T>(
    tally: &mut Tally,
    shots: impl IntoIterator<Item = Owned<'c, 'a>>,
    judge: impl Fn(Owned<'c, 'a>) -> Option<T>,
) -> Vec<(Owned<'c, 'a>, T)> {
    let mut judged = Vec::new();
    for owned in shots {
        if !owned.shot.sees(tally.topic) {
            continue;
        }
        match judge(owned) {
            Some(answer) => judged.push((owned, answer)),
            None => count_under(&mut tally.unscored, owned.shot.blank_reason(tally.topic)),
        }
    }
    tally.judged = judged.len();
    judged
}

/// Low point ahead of the ball, each shot by its own club's category (decision 7).
fn tally_low_point(shots: &[Owned]) -> Tally {
    let mut tally = Tally::new(Topic::LowPoint);
    let judged = judge_each(&mut tally, shots.iter().copied(), |owned| {
        let category = owned
            .owner
            .category
            .expect("low point exists only for an owner with a club");
        low_point_ahead(&owned.shot.view, category)
    });
    tally.successes = judged.iter().filter(|(_, ahead)| *ahead).count();
    tally
}

/// Strike location (decision 4) and the strikes the impact map is drawn from (call 10).
fn tally_strikes(shots: &[Owned]) -> (Tally, Vec<Strike>) {
    let mut tally = Tally::new(Topic::StrikeLocation);
    let strikes: Vec<Strike> = judge_each(&mut tally, shots.iter().copied(), |owned| {
        let view = &owned.shot.view;
        Some(Strike {
            centered: is_centered(view)?,
            cell: impact_cell(view)?,
            offset_h_mm: view.shot().impact_offset_h?,
        })
    })
    .into_iter()
    .map(|(_, strike)| strike)
    .collect();
    tally.successes = strikes.iter().filter(|strike| strike.centered).count();
    (tally, strikes)
}

/// Consistency, each shot against its own club's typical shot (decisions 1 and 6, call 4).
fn tally_consistency(shots: &[Owned]) -> Tally {
    let mut tally = Tally::new(Topic::Consistency);
    let judged = judge_each(&mut tally, shots.iter().copied(), |owned| {
        carry_and_offline(&owned.shot.view)
    });
    tally.successes = judged
        .iter()
        .filter(|(owned, went)| {
            let mishit = owned.shot.swing.is_mishit();
            // No typical shot means every shot its owner judges on consistency is a mishit, this one
            // included (it is among its owner's candidates, each photo on its first swing), and a
            // mishit is a miss before the window is read, so nothing needs a yardstick.
            owned
                .owner
                .typical
                .is_some_and(|typical| is_consistent(*went, typical, mishit))
        })
        .count();
    tally
}

/// The median carry and, taken apart, the median start-line offline (call 4), by
/// `stats::percentile`'s interpolation; `None` over no shots.
fn typical_shot(shots: impl Iterator<Item = CarryAndOffline>) -> Option<CarryAndOffline> {
    let (carries, offlines): (Vec<f64>, Vec<f64>) =
        shots.map(|went| (went.carry_yds, went.offline_yds)).unzip();
    Some(CarryAndOffline {
        carry_yds: percentile(&carries, 0.5)?,
        offline_yds: percentile(&offlines, 0.5)?,
    })
}

/// A shape topic over the shots declared that shape (ADR-034 §5.3; calls 5 and 6): "hit the shape"
/// over every judged shot, and "hit the shape and finished on line" over the judged shots that have
/// a landing, the others counted under why they have none.
fn tally_shape(topic: Topic, shots: &[Owned]) -> Tally {
    let declared = topic.shape().expect("a shape topic names its shape");
    let mut tally = Tally::new(topic);
    let judged = judge_each(
        &mut tally,
        shots
            .iter()
            .copied()
            .filter(|owned| owned.shot.declared(declared)),
        |owned| shape_of(&owned.shot.view),
    );
    let mut counts = ShapeTally::default();
    let mut successes = 0;
    for (owned, hit) in judged {
        let hit = hit == declared;
        counts.hit += usize::from(hit);
        let swing = owned.shot.swing;
        match offline_basis(&owned.shot.view, &swing.measurements, &swing.shot_unscored) {
            Ok(landing) => {
                match landing.basis {
                    OfflineBasis::Printed => counts.printed += 1,
                    OfflineBasis::Projected => counts.projected += 1,
                }
                successes += usize::from(hit && on_line(landing.offline_yds));
            }
            Err(reason) => count_under(&mut counts.landing_unscored, reason),
        }
    }
    tally.successes = successes;
    tally.shape = Some(counts);
    tally
}

/// The impact grid and the heel/toe lean over these strikes (the M37 plan's call 10).
///
/// The grid lists only the cells something landed in, ordered by row and then column with the
/// horizontal-only row first, which is `(Option<FaceRow>, FaceColumn)`'s own order. The lean's mean
/// and its Student-t interval are `stats::mean_and_sd` and `stats::mean_ci` over the offsets in
/// corpus order, shown only from the `Center` floor `baseline::minimum_n` gives the offset, as a
/// baseline's mean is.
fn impact_map(strikes: &[Strike]) -> ImpactMap {
    let mut cells: BTreeMap<(Option<FaceRow>, FaceColumn), i64> = BTreeMap::new();
    for strike in strikes {
        let (column, row) = strike.cell;
        *cells.entry((row, column)).or_default() += 1;
    }
    let column = |wanted: FaceColumn| -> i64 {
        cells
            .iter()
            .filter(|((_, column), _)| *column == wanted)
            .map(|(_, n)| n)
            .sum()
    };
    let offsets: Vec<f64> = strikes.iter().map(|strike| strike.offset_h_mm).collect();
    let floor = minimum_n(IMPACT_OFFSET_H, BaselineClaim::Center);
    let lean = if count(offsets.len()) >= floor {
        mean_and_sd(&offsets)
            .map(|(mean, _)| mean)
            .zip(mean_ci(&offsets))
    } else {
        None
    };
    ImpactMap {
        heel_toe: HeelToe {
            n: count(strikes.len()),
            heel: column(FaceColumn::Heel),
            toe: column(FaceColumn::Toe),
            mean_offset_mm: lean.map(|(mean, _)| mean),
            mean_offset_ci: lean.map(|(_, (low, high))| Interval {
                low,
                high,
                confidence: Interval::DEFAULT_CONFIDENCE,
            }),
        },
        grid: cells
            .into_iter()
            .map(|((vertical, horizontal), n)| ImpactCell {
                horizontal,
                vertical,
                n,
            })
            .collect(),
    }
}

fn count(len: usize) -> i64 {
    i64::try_from(len).expect("a count fits in an i64")
}

// --- The rows the tolerances come from ----------------------------------------------------------

/// A `METRIC_TARGETS` row's target and tolerance.
///
/// Panics when the row or its target is missing, because both names are constants and
/// `contracts`' `tests/strike.rs` pins the two rows to 0 ± 2.0 and 0 ± 9.0: a missing one is this
/// source wrong, not a shot.
fn target_and_tolerance(row: &str) -> (f64, f64) {
    let declared = target_for(row).unwrap_or_else(|| panic!("{row} is a METRIC_TARGETS row"));
    let target = declared
        .target
        .unwrap_or_else(|| panic!("{row} declares a target"));
    (target, declared.tolerance)
}
