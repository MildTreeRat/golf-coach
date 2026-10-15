//! A golfer's shots graded per club and per player: the strike profile, the six topics' grades, the
//! two blends, and the strengths and weaknesses list beside them. [M37 P1]
//!
//! **No Python twin, and none is coming.** This is the first analysis written Rust first with no
//! Python behind it at all: ADR-034 §5 designed it after the pivot, frozen Python never gains it
//! (ADR-035 clause 4), and its vectors are worked by hand (`spec/vectors/profile/hand/`, M37 P5)
//! rather than recorded from an oracle. So, like [`crate::capability`] and [`crate::shot_result`],
//! there is nothing here to mirror one for one. The design is ADR-034 §5 and the program plan's
//! §M37; the M37 plan (`docs/plans/m37-strike-profile-and-grades.md`) holds the interview decisions
//! and calls each item below cites.
//!
//! # A grade is a share of good shots
//!
//! A topic's grade is the share of a club's tracked shots meeting the topic's criterion, as a
//! [`RateEstimate`] with a Wilson interval, and it is **withheld below [`RATE_MINIMUM_N`]** judged
//! shots (ADR-034 §5.1). No tour band stands behind any of it: a share is honest without one, and an
//! outlier lowers it, which is what the user asked a grade to do.
//!
//! # Absent, withheld, graded: three states with three spellings
//!
//! - **Absent**: the topic cannot exist for this club, because its input is not printed or its shape
//!   was never declared. It is **not in `topics` at all**: no grade, no name, no blend entry.
//! - **Withheld**: printed, and too thin to grade. It is in `topics` with a [`Share::Withheld`]
//!   naming how many shots it has and needs, and its blend lists it as excluded.
//! - **Graded**: in `topics` with a [`Share::Graded`], and averaged by its blend.
//!
//! A blend with no graded topic is `None`, never 0 (ADR-034 §5.5). `analysis::scoring`'s mean
//! answers 0.0 for an empty list, so a swing with every checkpoint unscored prints an
//! `overall_score` of 0.0; the blends do not copy that. **On the wire an absent value is an absent
//! key, never `null`**, for the same reason: a reader that meets `"blend": null` has been handed a
//! field to forget to check. Every `Option` here is skipped when `None`, and `tests/strike.rs` holds
//! the whole profile to writing no `null` anywhere. A `null` that arrives still reads as absent.
//!
//! # The checks hold the parts to each other
//!
//! Most of a profile is derived twice over: a blend from its topics' grades, a heel/toe count from
//! the grid's columns, a shape topic's printed and projected split from its grade's `n`. Each pair is
//! two spellings of one fact that could disagree, so [`Validate`] holds them equal rather than
//! trusting the builder, and a graded share below the floor is refused rather than rendered. That is
//! the [`crate::baseline`] rule one level up: a withheld number is structurally incapable of reading
//! as a number.
//!
//! # What is not here
//!
//! The arithmetic. The Wilson interval is `analysis::stats` (M37 P2), the per-shot judgments and the
//! builders `analysis::strike` (P2, P6, P7), and the list `feedback::profile_rules` (P8), joined by
//! `core::profile::run_profile` (P9), the one crate holding both halves. **Mechanics enters none of
//! it** (Decision 12): no pose measurement is in [`STAT_SCOPES`], so none can reach a profile's
//! numbers or its list, and a test in each builder says so. A pose metric's history is
//! `crate::club_profile`'s, read by its own verbs.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::baseline::{minimum_n, BaselineClaim, Interval, MetricBaseline};
use crate::club::{category_of, ClubId};
use crate::dispersion::{target_for, MetricDispersion};
use crate::intent::{ClubCategory, TargetShape};
use crate::unscored::UnscoredReason;
use crate::{each, nested, ContractError, Validate};

// --- The constants ------------------------------------------------------------------------------

/// The fewest judged shots a share is shown at, for every topic grade and every share shown beside
/// one. **The user's, in the M37 interview** (decision 2).
///
/// The interval at the floor is what makes 10 defensible: Wilson 95% at a 50% share is about ±26
/// points at n = 10, ±33 at 5 and ±20 at 20, so a share at the floor already reads as the coarse
/// thing it is, and a floor set too low shows as a wide interval rather than a confident wrong grade
/// (the argument [`crate::baseline::DEFAULT_MINIMUM_N`]'s doc makes for its floors).
///
/// **Not that table's `Spread` row, though it is also 10.** That floor bounds a standard deviation's
/// relative error and this one a proportion's interval, so moving one must not move the other.
pub const RATE_MINIMUM_N: i64 = 10;

/// A strike is centered when it lands within this many millimetres of face center:
/// `hypot(h, v) ≤ 10` where both offsets hold values, `|h| ≤ 10` where only the horizontal does.
/// **The user's, and provisional** (decision 4).
///
/// Judgment with no measurement behind it, because no device supplies an offset yet. M42's impact
/// reader, which reads the offset off Full Swing's impact video in the iPhone's Live Photo, will
/// have an error of its own, and that measured error is what revises this. The impact grid bins at
/// the same number on each axis (the M37 plan's call 10).
pub const CENTERED_RADIUS_MM: f64 = 10.0;

/// A shot is consistent when its carry is within this fraction of its club's median carry, *and* its
/// start-line offline is within the same fraction **of that median carry** of the club's median
/// offline. **The user's** (decision 1).
///
/// One fraction, scaled per club: about ±15 yd for a 150-yard 7 iron and ±25 yd for a 250-yard
/// driver, where a fixed yardage would be harsh on the driver and lenient on a wedge. The lateral
/// number is `start_line_offline_yds`, a straight-line projection, and that is right here:
/// consistency asks whether a shot was like the golfer's other shots, not where it finished.
pub const CONSISTENCY_FRACTION: f64 = 0.10;

/// The `dispersion::METRIC_TARGETS` row whose tolerance separates straight from a shape: within it
/// a shot is straight, beyond it a fade (+) or a draw (−), for either hand (ADR-034 §5.3).
///
/// **A row name, not a new constant**, because ADR-034 §5.3 reuses the tolerance rather than
/// minting one, and a second 2.0 here would be free to drift from it. Read it with
/// [`target_for`] where it is used. `tests/strike.rs` holds the row to the 0 ± 2.0 decided.
pub const STRAIGHT_TOLERANCE_ROW: &str = "face_to_path_deg";

/// The `METRIC_TARGETS` row whose tolerance says a landing finished on line: ±9 yd, which is
/// ADR-034 §5.3's default, confirmed for a landing by the user knowing it is generous for a wedge
/// and tight for a driver (decision 8). A row name for [`STRAIGHT_TOLERANCE_ROW`]'s reason.
pub const ON_LINE_TOLERANCE_ROW: &str = "start_line_offline_yds";

/// The measurement a club's ball-speed spread is read from. [`StrikeProfile::ball_speed`] carries
/// its baseline whole, from the club's `ClubProfile`, never recomputed.
pub const BALL_SPEED_METRIC: &str = "ball_speed_mph";

/// The measurements whose dispersion a [`StrikeProfile`] carries: carry, start line and
/// face-to-path, "each through the existing dispersion" (§M37). Copied from the club's
/// `ClubProfile.dispersion`, which already ran them, rather than run again.
pub const STRIKE_DISPERSION_METRICS: [&str; 3] =
    ["carry_distance_yds", "start_line_deg", "face_to_path_deg"];

/// The `ShotData` key strike location reads, and the key [`STAT_SCOPES`] gives the heel/toe lean.
const IMPACT_OFFSET_H: &str = "impact_offset_h";

// --- The topics ---------------------------------------------------------------------------------

/// The six things a club's shots are graded on (ADR-034 §5.2), in the order a report lists them.
///
/// The three shapes count as three topics, each graded only over shots declared as that shape: a
/// golfer who never practises a draw has no draw topic, rather than a draw graded on fades.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Topic {
    /// Where on the face, judged by [`CENTERED_RADIUS_MM`]. **Only from a measured face-impact
    /// offset** (decision 3): HD Golf's printed `CENTER`/`HEEL`/`TOE` word is shown and never
    /// graded, so on HD Golf, and on every stored shot, this topic is absent until M42.
    StrikeLocation,
    /// How often a shot lands near the club's typical shot ([`CONSISTENCY_FRACTION`]). A mishit is
    /// a miss whatever its numbers (decision 6).
    Consistency,
    /// Low point ahead of the ball, for an iron, wedge or hybrid ([`low_point_graded_for`]). No
    /// device prints it today.
    LowPoint,
    /// Shots declared a fade, hit as one and finished on line.
    Fade,
    /// Shots declared a draw, hit as one and finished on line.
    Draw,
    /// Shots declared straight, hit straight and finished on line.
    Straight,
}

impl Topic {
    /// Every topic, in declaration order, which is the order `topics` lists them in.
    pub const ALL: [Topic; 6] = [
        Topic::StrikeLocation,
        Topic::Consistency,
        Topic::LowPoint,
        Topic::Fade,
        Topic::Draw,
        Topic::Straight,
    ];

    /// The wire name.
    pub const fn as_str(self) -> &'static str {
        match self {
            Topic::StrikeLocation => "strike_location",
            Topic::Consistency => "consistency",
            Topic::LowPoint => "low_point",
            Topic::Fade => "fade",
            Topic::Draw => "draw",
            Topic::Straight => "straight",
        }
    }

    /// The declared shape this topic grades, or `None` for the three that are not shapes.
    pub const fn shape(self) -> Option<TargetShape> {
        match self {
            Topic::Fade => Some(TargetShape::Fade),
            Topic::Draw => Some(TargetShape::Draw),
            Topic::Straight => Some(TargetShape::Straight),
            Topic::StrikeLocation | Topic::Consistency | Topic::LowPoint => None,
        }
    }

    /// The topic a declared shape is graded under.
    pub const fn for_shape(shape: TargetShape) -> Topic {
        match shape {
            TargetShape::Fade => Topic::Fade,
            TargetShape::Draw => Topic::Draw,
            TargetShape::Straight => Topic::Straight,
        }
    }

    /// Whether this topic's criterion needs to know the club (the M37 plan's call 9).
    ///
    /// Consistency is judged against the club's median and low point by the club's category, so a
    /// swing tagged with no club cannot be judged on either. Strike location and the shapes need no
    /// club, so an untagged swing counts toward the player's grade for them. An arm per topic rather
    /// than a list of the two, so a seventh topic is a compile error here until someone decides.
    pub const fn needs_a_club(self) -> bool {
        match self {
            Topic::Consistency | Topic::LowPoint => true,
            Topic::StrikeLocation | Topic::Fade | Topic::Draw | Topic::Straight => false,
        }
    }

    /// The `ShotData` keys this topic reads, every one of which must be printed for the topic to
    /// exist for a club (the M37 plan's call 7; ADR-034 §2's "not printed produces nothing").
    ///
    /// Strike location needs the horizontal offset alone: the vertical one adds rows to the grid
    /// when it is printed and is never required. Consistency reads carry and the start direction its
    /// offline is projected from. A shape needs the two angles face-to-path is made of, **and** a
    /// shot declared that shape, which no `ShotData` key can say.
    pub const fn inputs(self) -> &'static [&'static str] {
        match self {
            Topic::StrikeLocation => &[IMPACT_OFFSET_H],
            Topic::Consistency => &["carry_distance", "launch_direction"],
            Topic::LowPoint => &["low_point"],
            Topic::Fade | Topic::Draw | Topic::Straight => &["club_face_angle", "club_path"],
        }
    }
}

impl Validate for Topic {
    fn validate(&self) -> Result<(), ContractError> {
        Ok(())
    }
}

// Row `i` of `ALL` is the topic whose discriminant is `i`, so a topic added out of place fails the
// build rather than reordering every report.
const _: () = {
    let mut i = 0;
    while i < Topic::ALL.len() {
        assert!(Topic::ALL[i] as usize == i);
        i += 1;
    }
};

/// Whether low point is graded for a club of this category (decision 7): ahead of the ball is good
/// for an iron, a wedge or a hybrid. **Absent for a driver or a wood**, where hitting up is correct,
/// and for a putter. `All` is no club, which low point cannot be judged without.
pub const fn low_point_graded_for(category: ClubCategory) -> bool {
    match category {
        ClubCategory::Hybrid
        | ClubCategory::LongIron
        | ClubCategory::MidIron
        | ClubCategory::ShortIron
        | ClubCategory::Wedge => true,
        ClubCategory::Driver | ClubCategory::Wood | ClubCategory::Putter | ClubCategory::All => {
            false
        }
    }
}

/// Where a shot's landing offline came from (the M37 plan's call 6).
///
/// **There is no third variant for "none".** A shot with no basis has no landing, and it is counted
/// under the `unscored` reason that says why (`spin_axis_unresolved`, `no_handedness`, the flight's
/// own refusal) rather than under a basis that is not one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OfflineBasis {
    /// The device printed it (`carry_offline`, where `capability::analysed_on` names it). Used
    /// wherever it exists, so a device that starts printing one replaces the projection with no
    /// code change (ADR-034 §5.3).
    Printed,
    /// `flight_landing_offline_yds`, ADR-027's flight model with the curve, recorded only when the
    /// spin axis resolved. ADR-034 §5.3's one sanctioned inference, and **named as projected
    /// wherever it appears**.
    Projected,
}

impl OfflineBasis {
    /// The wire name, which is also the word a report prints beside a projected landing.
    pub const fn as_str(self) -> &'static str {
        match self {
            OfflineBasis::Printed => "printed",
            OfflineBasis::Projected => "projected",
        }
    }
}

// --- Stat scope ---------------------------------------------------------------------------------

/// Whether a raw statistic means anything pooled across a bag (ADR-034 §4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatScope {
    /// Scales with the club, so a raw number exists per club and never per player: a
    /// driver-plus-wedge carry average means nothing.
    ClubOnly,
    /// The same question whatever the club, so the player gets a raw number too.
    ClubIndependent,
}

/// Each statistic's scope, keyed by measurement name (the M37 plan's call 17).
///
/// The shot rows in `analysis::shot_measure`'s order, then the flight rows in
/// `analysis::flight_measure`'s, then the two face-impact offsets. **The offsets are keyed by their
/// `ShotData` names** because no measurement row carries them: strike location reads the shot
/// directly, and what is pooled is the lean they add up to.
///
/// Club-independent are the two angles and the offsets: a face 3° open to the path, a ball started
/// 2° right and a strike 8 mm toward the toe mean the same with any club. Everything in yards, mph,
/// rpm or seconds scales with the club, and so does launch. **A metric not here has no scope and
/// gets no player-level number**, which is where every pose measurement sits: mechanics is a
/// separate panel (Decision 12), and a metric added tomorrow is held back by omission rather than
/// pooled by default. `analysis`'s `shot_measure` tests hold every registered shot and flight row
/// to having a row here.
pub const STAT_SCOPES: &[(&str, StatScope)] = &[
    ("face_to_path_deg", StatScope::ClubIndependent),
    ("start_line_deg", StatScope::ClubIndependent),
    ("carry_distance_yds", StatScope::ClubOnly),
    ("total_distance_yds", StatScope::ClubOnly),
    ("start_line_offline_yds", StatScope::ClubOnly),
    ("ball_speed_mph", StatScope::ClubOnly),
    ("launch_angle_deg", StatScope::ClubOnly),
    ("flight_carry_yds", StatScope::ClubOnly),
    ("flight_apex_yds", StatScope::ClubOnly),
    ("flight_descent_angle_deg", StatScope::ClubOnly),
    ("flight_time_s", StatScope::ClubOnly),
    ("flight_landing_offline_yds", StatScope::ClubOnly),
    ("flight_spin_rpm", StatScope::ClubOnly),
    (IMPACT_OFFSET_H, StatScope::ClubIndependent),
    ("impact_offset_v", StatScope::ClubIndependent),
];

/// This statistic's scope, or `None` when it has none ([`STAT_SCOPES`]).
pub fn scope_of(metric: &str) -> Option<StatScope> {
    STAT_SCOPES
        .iter()
        .find(|(name, _)| *name == metric)
        .map(|(_, scope)| *scope)
}

/// May the player be shown a raw number for this statistic? Only a registered club-independent one.
pub fn is_club_independent(metric: &str) -> bool {
    scope_of(metric) == Some(StatScope::ClubIndependent)
}

// --- A share, graded or withheld ----------------------------------------------------------------

/// A share of good shots, with its Wilson interval. The arithmetic is `analysis::stats::wilson`
/// (M37 P2); this is the shape it answers in.
///
/// `share` is `successes / n` carried rather than left to the reader, because the phone and the
/// report both show it and neither should divide. So it is checked to be exactly that quotient,
/// which a builder computing it the one way meets bit for bit, and a hand-worked vector meets by
/// writing the quotient's shortest form (`repr(7 / 12)` is `0.5833333333333334`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Self")]
pub struct RateEstimate {
    pub successes: i64,
    pub n: i64,
    pub share: f64,
    /// The Wilson score interval at 95%. It always holds the share and stays inside [0, 1], which
    /// is the point of choosing Wilson over the normal approximation at these `n`, so a builder whose
    /// rounding strays past either end at 0 or `n` successes is refused here.
    pub interval: Interval,
}

crate::validated!(RateEstimate);

impl Validate for RateEstimate {
    fn validate(&self) -> Result<(), ContractError> {
        if self.n < 1 {
            return Err(refuse(
                "RateEstimate",
                format!("n is {}, and a share of no shots is not a share", self.n),
            ));
        }
        if self.successes < 0 || self.successes > self.n {
            return Err(refuse(
                "RateEstimate",
                format!(
                    "{} successes is not between 0 and n = {}",
                    self.successes, self.n
                ),
            ));
        }
        let quotient = self.successes as f64 / self.n as f64;
        if self.share != quotient {
            return Err(refuse(
                "RateEstimate.share",
                format!(
                    "{} is not {} / {} = {quotient}",
                    self.share, self.successes, self.n
                ),
            ));
        }
        nested("RateEstimate.interval", Some(&self.interval))?;
        let Interval { low, high, .. } = self.interval;
        let holds = 0.0 <= low && low <= self.share && self.share <= high && high <= 1.0;
        if !holds {
            return Err(refuse(
                "RateEstimate.interval",
                format!(
                    "[{low}, {high}] does not hold the share {} inside [0, 1]",
                    self.share
                ),
            ));
        }
        Ok(())
    }
}

/// A share that is too thin to show, and what it is waiting for. **Not a zero and not a grade**: it
/// is excluded from its blend and named, as `overall_score` routes an unscorable checkpoint to
/// `unscored` rather than averaging it (ADR-034 §5.5).
///
/// `need_n` is carried, though it is always [`RATE_MINIMUM_N`], so a refusal says what would lift it
/// on its own, as [`crate::baseline::WithheldClaim`] does.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Self")]
pub struct WithheldShare {
    pub have_n: i64,
    pub need_n: i64,
}

crate::validated!(WithheldShare);

impl Validate for WithheldShare {
    fn validate(&self) -> Result<(), ContractError> {
        if self.need_n != RATE_MINIMUM_N {
            return Err(refuse(
                "WithheldShare.need_n",
                format!("{} is not the floor, {RATE_MINIMUM_N}", self.need_n),
            ));
        }
        if self.have_n < 0 || self.have_n >= self.need_n {
            return Err(refuse(
                "WithheldShare.have_n",
                format!(
                    "{} is not a count below the floor of {}",
                    self.have_n, self.need_n
                ),
            ));
        }
        Ok(())
    }
}

/// A share, or the reason it is not shown. An enum rather than a share beside a flag, so a withheld
/// share has no number in it to render by mistake.
///
/// Written `{"graded": {…}}` or `{"withheld": {…}}`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Self", rename_all = "snake_case")]
pub enum Share {
    /// At or above [`RATE_MINIMUM_N`] judged shots.
    Graded(RateEstimate),
    /// Below it.
    Withheld(WithheldShare),
}

crate::validated!(Share);

impl Validate for Share {
    fn validate(&self) -> Result<(), ContractError> {
        match self {
            Share::Graded(rate) => {
                nested("Share.graded", Some(rate))?;
                if rate.n < RATE_MINIMUM_N {
                    return Err(refuse(
                        "Share.graded",
                        format!(
                            "n = {} is below the floor of {RATE_MINIMUM_N}, so it is withheld",
                            rate.n
                        ),
                    ));
                }
                Ok(())
            }
            Share::Withheld(withheld) => nested("Share.withheld", Some(withheld)),
        }
    }
}

impl Share {
    /// The shots this share is over, graded or not.
    pub fn n(&self) -> i64 {
        match self {
            Share::Graded(rate) => rate.n,
            Share::Withheld(withheld) => withheld.have_n,
        }
    }

    /// The estimate, when there is one to show.
    pub fn graded(&self) -> Option<&RateEstimate> {
        match self {
            Share::Graded(rate) => Some(rate),
            Share::Withheld(_) => None,
        }
    }
}

/// Shots a topic was printed for and could not judge, under one reason (the M37 plan's call 8).
///
/// A blank or misread input is **unscored for that topic and never a miss** (ADR-010 §2), so it is
/// counted here rather than in the share's denominator.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Self")]
pub struct UnscoredCount {
    pub reason: UnscoredReason,
    /// At least 1: a reason nothing fell under is not listed.
    pub n: i64,
}

crate::validated!(UnscoredCount);

impl Validate for UnscoredCount {
    fn validate(&self) -> Result<(), ContractError> {
        if self.n < 1 {
            return Err(refuse(
                "UnscoredCount.n",
                format!("{} shots is not a count worth listing", self.n),
            ));
        }
        Ok(())
    }
}

/// Each count valid, each reason once, in `UnscoredReason` declaration order, so two builds of one
/// corpus list them identically.
fn check_counts(field: &str, counts: &[UnscoredCount]) -> Result<(), ContractError> {
    each(field, counts)?;
    for pair in counts.windows(2) {
        if pair[0].reason as usize >= pair[1].reason as usize {
            return Err(refuse(
                field,
                format!(
                    "{:?} is listed after {:?}: each reason once, in declaration order",
                    pair[1].reason, pair[0].reason
                ),
            ));
        }
    }
    Ok(())
}

// --- One topic's grade --------------------------------------------------------------------------

/// A shape topic's other share and its landings (ADR-034 §5.3).
///
/// The topic's **grade** is "hit the shape and finished on line", which is
/// [`TopicGrade::grade`] and enters the blends. This holds what shows beside it: "hit the shape",
/// so a golfer can see whether the shaping or the aiming failed, and where the landings came from.
/// The on-line share is not repeated here, so the two cannot disagree.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Self")]
pub struct ShapeShares {
    /// Of the judged shots, the share classified as the declared shape by face-to-path against
    /// [`STRAIGHT_TOLERANCE_ROW`]'s tolerance. Never in a blend.
    pub hit_shape: Share,
    /// Of the grade's shots, how many had a printed landing offline ([`OfflineBasis::Printed`]).
    #[serde(default)]
    pub printed: i64,
    /// And how many a projected one ([`OfflineBasis::Projected`]), which a report names as
    /// projected.
    #[serde(default)]
    pub projected: i64,
    /// Judged shots with no landing at all, by reason. Out of the grade's denominator and in
    /// `hit_shape`'s (call 6).
    #[serde(default)]
    pub landing_unscored: Vec<UnscoredCount>,
}

crate::validated!(ShapeShares);

impl Validate for ShapeShares {
    fn validate(&self) -> Result<(), ContractError> {
        nested("ShapeShares.hit_shape", Some(&self.hit_shape))?;
        if self.printed < 0 || self.projected < 0 {
            return Err(refuse(
                "ShapeShares",
                format!(
                    "printed {} and projected {} must be counts",
                    self.printed, self.projected
                ),
            ));
        }
        check_counts("ShapeShares.landing_unscored", &self.landing_unscored)
    }
}

/// One topic, for one club or for the player: its grade, the evidence, and what it could not judge.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Self")]
pub struct TopicGrade {
    pub topic: Topic,
    /// Shots judged on this topic: tracked, counting toward metrics, and holding the topic's inputs
    /// (call 8). Always populated, because it is what makes a withheld grade actionable.
    pub judged: i64,
    /// The share of judged shots meeting the criterion, or why it is not shown. For a shape topic it
    /// is the share that hit the shape **and** finished on line, over the judged shots with a
    /// landing.
    pub grade: Share,
    /// Shots the topic was printed for and could not judge, by reason (call 8).
    #[serde(default)]
    pub unscored: Vec<UnscoredCount>,
    /// Present on a shape topic and only there.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shape: Option<ShapeShares>,
}

crate::validated!(TopicGrade);

impl Validate for TopicGrade {
    fn validate(&self) -> Result<(), ContractError> {
        let field = format!("TopicGrade[{}]", self.topic.as_str());
        if self.judged < 0 {
            return Err(refuse(&field, format!("judged is {}", self.judged)));
        }
        nested(&format!("{field}.grade"), Some(&self.grade))?;
        check_counts(&format!("{field}.unscored"), &self.unscored)?;
        match (self.topic.shape(), &self.shape) {
            (None, None) => {
                if self.grade.n() != self.judged {
                    return Err(refuse(
                        &field,
                        format!(
                            "the grade is over {} shots and {} were judged",
                            self.grade.n(),
                            self.judged
                        ),
                    ));
                }
            }
            (None, Some(_)) => {
                return Err(refuse(&field, "only a shape topic carries shape shares"));
            }
            (Some(_), None) => {
                return Err(refuse(&field, "a shape topic carries its shape shares"));
            }
            (Some(_), Some(shares)) => {
                nested(&format!("{field}.shape"), Some(shares))?;
                let no_landing: i64 = shares.landing_unscored.iter().map(|count| count.n).sum();
                let held = [
                    (
                        shares.hit_shape.n(),
                        self.judged,
                        "hit_shape is over the judged shots",
                    ),
                    (
                        self.grade.n() + no_landing,
                        self.judged,
                        "the grade's shots and the landing refusals are the judged shots",
                    ),
                    (
                        shares.printed + shares.projected,
                        self.grade.n(),
                        "the printed and projected landings are the grade's shots",
                    ),
                ];
                for (got, want, rule) in held {
                    if got != want {
                        return Err(refuse(&field, format!("{rule}: {got} != {want}")));
                    }
                }
            }
        }
        Ok(())
    }
}

/// The equal-weight mean of the graded topics' shares (ADR-034 §5.4; the M37 plan's call 11).
///
/// No interval of its own: the topics' intervals do not combine into one without a model of how
/// they correlate, and an interval nobody derived is the confident wrong number this module exists
/// to refuse. Equal weights because no weighting has been measured. **Absent when nothing is graded**
/// (its owner's `blend` is `None`), never 0.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Self")]
pub struct Blend {
    /// In [0, 1].
    pub mean: f64,
    /// The graded topics averaged, in topic order.
    pub averaged: Vec<Topic>,
    /// The withheld topics left out, in topic order. Each one's grade says why, so the reason is
    /// read there rather than copied here. An absent topic is in neither list.
    #[serde(default)]
    pub excluded: Vec<Topic>,
}

crate::validated!(Blend);

impl Validate for Blend {
    fn validate(&self) -> Result<(), ContractError> {
        if self.averaged.is_empty() {
            return Err(refuse(
                "Blend.averaged",
                "a blend of nothing is absent, not a blend",
            ));
        }
        if !(0.0..=1.0).contains(&self.mean) {
            return Err(refuse(
                "Blend.mean",
                format!("{} is not a share", self.mean),
            ));
        }
        Ok(())
    }
}

/// The topics in [`Topic::ALL`] order, each once and valid, none judging more shots than its owner
/// holds, and the blend exactly the graded ones averaged and the withheld ones excluded.
fn check_topics(
    owner: &str,
    n_shots: i64,
    topics: &[TopicGrade],
    blend: Option<&Blend>,
) -> Result<(), ContractError> {
    each(&format!("{owner}.topics"), topics)?;
    for pair in topics.windows(2) {
        if pair[0].topic >= pair[1].topic {
            return Err(refuse(
                &format!("{owner}.topics"),
                format!(
                    "{} is listed after {}: each topic once, in topic order",
                    pair[1].topic.as_str(),
                    pair[0].topic.as_str()
                ),
            ));
        }
    }
    if let Some(over) = topics.iter().find(|grade| grade.judged > n_shots) {
        return Err(refuse(
            &format!("{owner}.topics"),
            format!(
                "{} judged {} of {n_shots} shots",
                over.topic.as_str(),
                over.judged
            ),
        ));
    }
    let (graded, withheld): (Vec<&TopicGrade>, Vec<&TopicGrade>) = topics
        .iter()
        .partition(|grade| grade.grade.graded().is_some());
    let graded: Vec<Topic> = graded.iter().map(|grade| grade.topic).collect();
    let withheld: Vec<Topic> = withheld.iter().map(|grade| grade.topic).collect();
    match blend {
        None if graded.is_empty() => Ok(()),
        None => Err(refuse(
            &format!("{owner}.blend"),
            format!("absent while {graded:?} are graded"),
        )),
        Some(blend) => {
            nested(&format!("{owner}.blend"), Some(blend))?;
            if blend.averaged != graded || blend.excluded != withheld {
                return Err(refuse(
                    &format!("{owner}.blend"),
                    format!(
                        "averages {:?} and excludes {:?}, where the graded topics are {graded:?} \
                         and the withheld {withheld:?}",
                        blend.averaged, blend.excluded
                    ),
                ));
            }
            Ok(())
        }
    }
}

// --- Where on the face --------------------------------------------------------------------------

/// Heel to toe, by the horizontal offset (`+` = toe), binned at ±[`CENTERED_RADIUS_MM`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FaceColumn {
    Heel,
    Center,
    Toe,
}

/// Low to high, by the vertical offset (`+` = high), binned at ±[`CENTERED_RADIUS_MM`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FaceRow {
    Low,
    Center,
    High,
}

/// One cell of the impact grid and the shots that landed in it (the M37 plan's call 10).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Self")]
pub struct ImpactCell {
    pub horizontal: FaceColumn,
    /// Absent for a shot whose vertical offset holds no value, which puts it on the grid's one
    /// horizontal-only row rather than guessing a height for it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vertical: Option<FaceRow>,
    /// At least 1: the grid lists only the cells something landed in.
    pub n: i64,
}

crate::validated!(ImpactCell);

impl Validate for ImpactCell {
    fn validate(&self) -> Result<(), ContractError> {
        if self.n < 1 {
            return Err(refuse(
                "ImpactCell.n",
                format!("{}: an empty cell is not listed", self.n),
            ));
        }
        Ok(())
    }
}

/// Which way off-center strikes lean (the M37 plan's call 10).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Self")]
pub struct HeelToe {
    /// Shots with a horizontal offset.
    pub n: i64,
    /// Of them, beyond the radius toward the heel, and toward the toe. The grid's outer columns,
    /// added up.
    pub heel: i64,
    pub toe: i64,
    /// The mean horizontal offset in millimetres, `+` = toe, with its 95% interval. Present only
    /// once `n` holds the `Center` floor `baseline::minimum_n` gives the offset, as a baseline's
    /// mean is: a lean is a claim about where the center sits.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mean_offset_mm: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mean_offset_ci: Option<Interval>,
}

crate::validated!(HeelToe);

impl Validate for HeelToe {
    fn validate(&self) -> Result<(), ContractError> {
        if self.heel < 0 || self.toe < 0 || self.heel + self.toe > self.n {
            return Err(refuse(
                "HeelToe",
                format!(
                    "heel {} and toe {} are not a split of {} shots",
                    self.heel, self.toe, self.n
                ),
            ));
        }
        match (self.mean_offset_mm, &self.mean_offset_ci) {
            (None, None) => Ok(()),
            (Some(mean), Some(ci)) => {
                nested("HeelToe.mean_offset_ci", Some(ci))?;
                let floor = minimum_n(IMPACT_OFFSET_H, BaselineClaim::Center);
                if self.n < floor {
                    return Err(refuse(
                        "HeelToe.mean_offset_mm",
                        format!(
                            "shown over {} shots, below the center floor of {floor}",
                            self.n
                        ),
                    ));
                }
                let holds = ci.low <= mean && mean <= ci.high;
                if !holds {
                    return Err(refuse(
                        "HeelToe.mean_offset_ci",
                        format!("[{}, {}] does not hold the mean {mean}", ci.low, ci.high),
                    ));
                }
                Ok(())
            }
            _ => Err(refuse(
                "HeelToe",
                "the mean offset and its interval are shown together or not at all",
            )),
        }
    }
}

/// Where on the face, as a picture and as a lean.
///
/// **The grid is the picture and the rate is the judgment, and they differ on purpose.** The grid
/// bins each axis at ±[`CENTERED_RADIUS_MM`], so its center cell is a square; the centered rate,
/// which is the strike-location topic's grade, uses the radius, a circle. A strike at (9, 9) mm is in
/// the center cell and is not centered (`hypot` is 12.7). A box would have been simpler to grade
/// and would call a strike 14 mm off center a good one.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Self")]
pub struct ImpactMap {
    /// The cells something landed in: the horizontal-only cells first, then low to high, each row
    /// heel to toe.
    #[serde(default)]
    pub grid: Vec<ImpactCell>,
    pub heel_toe: HeelToe,
}

crate::validated!(ImpactMap);

impl Validate for ImpactMap {
    fn validate(&self) -> Result<(), ContractError> {
        each("ImpactMap.grid", &self.grid)?;
        nested("ImpactMap.heel_toe", Some(&self.heel_toe))?;
        for pair in self.grid.windows(2) {
            let key = |cell: &ImpactCell| (cell.vertical, cell.horizontal);
            if key(&pair[0]) >= key(&pair[1]) {
                return Err(refuse(
                    "ImpactMap.grid",
                    format!(
                        "{:?} is listed after {:?}: each cell once, row by row",
                        key(&pair[1]),
                        key(&pair[0])
                    ),
                ));
            }
        }
        let landed = |column: Option<FaceColumn>| -> i64 {
            self.grid
                .iter()
                .filter(|cell| column.is_none_or(|column| cell.horizontal == column))
                .map(|cell| cell.n)
                .sum()
        };
        let held = [
            (landed(None), self.heel_toe.n, "n"),
            (landed(Some(FaceColumn::Heel)), self.heel_toe.heel, "heel"),
            (landed(Some(FaceColumn::Toe)), self.heel_toe.toe, "toe"),
        ];
        for (grid, lean, what) in held {
            if grid != lean {
                return Err(refuse(
                    "ImpactMap",
                    format!("the grid holds {grid} for heel_toe's {what} of {lean}"),
                ));
            }
        }
        Ok(())
    }
}

// --- The profile --------------------------------------------------------------------------------

/// The raw statistics beside a set of grades: where on the face, and the spreads of the shot
/// numbers (§M37's list).
///
/// **Carried, never recomputed.** The ball-speed baseline and the dispersions are copied whole from
/// the club's `ClubProfile`, which `analysis::club_profile` already built through
/// `CareerCorpus::narrowed_to`, refusals and all (the M37 plan's finding 11). The centered rate is
/// not here: it is the strike-location topic's grade.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Self")]
pub struct StrikeProfile {
    /// Present exactly when the strike-location topic is, so never on a device that does not print
    /// an impact offset, which is every real device until M42.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub impacts: Option<ImpactMap>,
    /// The [`BALL_SPEED_METRIC`] baseline, whose `sd` is the ball-speed spread. Absent at player
    /// level, where ball speed is club-only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ball_speed: Option<MetricBaseline>,
    /// [`STRIKE_DISPERSION_METRICS`] → its dispersion, for those the corpus holds; sorted by name.
    #[serde(default)]
    pub dispersion: BTreeMap<String, MetricDispersion>,
}

crate::validated!(StrikeProfile);

impl Validate for StrikeProfile {
    fn validate(&self) -> Result<(), ContractError> {
        nested("StrikeProfile.impacts", self.impacts.as_ref())?;
        if let Some(baseline) = &self.ball_speed {
            if baseline.name != BALL_SPEED_METRIC {
                return Err(refuse(
                    "StrikeProfile.ball_speed",
                    format!("is {:?}, not {BALL_SPEED_METRIC:?}", baseline.name),
                ));
            }
            nested("StrikeProfile.ball_speed", Some(baseline))?;
        }
        for (name, dispersion) in &self.dispersion {
            if !STRIKE_DISPERSION_METRICS.contains(&name.as_str()) || dispersion.name != *name {
                return Err(refuse(
                    "StrikeProfile.dispersion",
                    format!(
                        "{name:?} holds {:?}; the keys are {STRIKE_DISPERSION_METRICS:?}, each \
                         holding its own",
                        dispersion.name
                    ),
                ));
            }
            nested(
                &format!("StrikeProfile.dispersion[{name:?}]"),
                Some(dispersion),
            )?;
        }
        Ok(())
    }
}

/// Strike location exists exactly where the impacts do.
fn check_impacts(
    owner: &str,
    strike: &StrikeProfile,
    topics: &[TopicGrade],
) -> Result<(), ContractError> {
    let graded_here = topics
        .iter()
        .any(|grade| grade.topic == Topic::StrikeLocation);
    if strike.impacts.is_some() != graded_here {
        return Err(refuse(
            owner,
            "the impact map and the strike-location topic are present together or not at all",
        ));
    }
    Ok(())
}

/// One club's grades (ADR-034 §5): its strike profile, the topics that exist for it, and their blend.
/// Built by `analysis::strike` (M37 P6).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Self")]
pub struct ClubGrades {
    pub club: ClubId,
    /// Distinct tracked shot photos of this club, whatever each topic then judged: the evidence
    /// every topic's `judged` is drawn from. Never gated.
    pub n_shots: i64,
    pub strike: StrikeProfile,
    /// The topics that exist for this club, in topic order. A topic not here is absent.
    #[serde(default)]
    pub topics: Vec<TopicGrade>,
    /// The club blend, or absent when no topic is graded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blend: Option<Blend>,
}

crate::validated!(ClubGrades);

impl Validate for ClubGrades {
    fn validate(&self) -> Result<(), ContractError> {
        let owner = format!("ClubGrades[{}]", self.club);
        if self.n_shots < 0 {
            return Err(refuse(&owner, format!("n_shots is {}", self.n_shots)));
        }
        nested(&format!("{owner}.strike"), Some(&self.strike))?;
        check_topics(&owner, self.n_shots, &self.topics, self.blend.as_ref())?;
        check_impacts(&owner, &self.strike, &self.topics)?;
        let category = category_of(self.club);
        let low_point = self
            .topics
            .iter()
            .any(|grade| grade.topic == Topic::LowPoint);
        if low_point && !low_point_graded_for(category) {
            return Err(refuse(
                &owner,
                format!(
                    "low point is not graded for a {} (decision 7)",
                    category.as_str()
                ),
            ));
        }
        Ok(())
    }
}

/// The player's grades: each topic's share pooled over every tracked shot, each shot judged by its
/// own club's criterion (ADR-034 §5.4), and raw numbers only where they mean something across a bag.
/// Built by `analysis::strike` (M37 P7).
///
/// **The raw numbers are club-independent only** ([`is_club_independent`]): the strike profile here
/// carries no ball speed and no carry, which [`Validate`] refuses, because a driver-plus-wedge
/// average is not a number about anybody (ADR-034 §4).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Self")]
pub struct PlayerGrades {
    /// Distinct tracked shot photos across the bag, the untagged ones included.
    pub n_shots: i64,
    /// Of them, the ones whose swing names no club (call 9). They count toward strike location and
    /// the shapes, whose criteria need no club, and toward nothing else, so they are named.
    #[serde(default)]
    pub untagged_shots: i64,
    pub strike: StrikeProfile,
    /// The topics that exist for any club, or for the untagged shots, in topic order.
    #[serde(default)]
    pub topics: Vec<TopicGrade>,
    /// The player blend, or absent when no topic is graded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blend: Option<Blend>,
}

crate::validated!(PlayerGrades);

impl Validate for PlayerGrades {
    fn validate(&self) -> Result<(), ContractError> {
        let owner = "PlayerGrades";
        if self.untagged_shots < 0 || self.untagged_shots > self.n_shots {
            return Err(refuse(
                owner,
                format!("{} untagged of {} shots", self.untagged_shots, self.n_shots),
            ));
        }
        nested("PlayerGrades.strike", Some(&self.strike))?;
        let club_only = self
            .strike
            .ball_speed
            .iter()
            .map(|baseline| baseline.name.as_str())
            .chain(self.strike.dispersion.keys().map(String::as_str))
            .find(|metric| !is_club_independent(metric));
        if let Some(metric) = club_only {
            return Err(refuse(
                "PlayerGrades.strike",
                format!("{metric} is club-only, so the player has no raw number for it"),
            ));
        }
        check_topics(owner, self.n_shots, &self.topics, self.blend.as_ref())?;
        check_impacts(owner, &self.strike, &self.topics)
    }
}

// --- The list -----------------------------------------------------------------------------------

/// What one entry in the strengths and weaknesses list asserts (ADR-034 §5.7).
///
/// Named apart from `dispersion::Finding`, which is the answer to "is this established?". This is
/// what an established answer becomes on the list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingKind {
    /// A weakness: an established bias, the mean's interval clear of target ± tolerance.
    Bias,
    /// A weakness: established scatter, the SD's interval clear above the tolerance.
    Scatter,
    /// An established equivalence: the mean's interval inside target ± tolerance *and* the SD's
    /// upper bound below it. **"Nothing established" is never one.**
    Strength,
}

impl FindingKind {
    /// The wire name.
    pub const fn as_str(self) -> &'static str {
        match self {
            FindingKind::Bias => "bias",
            FindingKind::Scatter => "scatter",
            FindingKind::Strength => "strength",
        }
    }

    /// Bias and scatter are weaknesses; a strength is not.
    pub const fn is_weakness(self) -> bool {
        match self {
            FindingKind::Bias | FindingKind::Scatter => true,
            FindingKind::Strength => false,
        }
    }
}

/// One entry on the list: a launch-monitor metric, what is established about it, and by how much.
///
/// `ratio` is the point estimate against the tolerance (the M37 plan's call 12): `|mean − target| /
/// tolerance` for a bias and `sd / tolerance` for scatter, both above 1 when established, which is
/// what a weakness is ranked by. For a strength it is the nearer of its two bounds to failing,
/// `max(|ci end − target|, sd_ci.high) / tolerance`, at most 1 (exactly 1 only when an interval end
/// sits on the band's inclusive edge), so the smaller the stronger. The rule is
/// `feedback::profile_rules`'s (M37 P8).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Self")]
pub struct ProfileFinding {
    pub metric: String,
    pub kind: FindingKind,
    pub ratio: f64,
}

crate::validated!(ProfileFinding);

impl Validate for ProfileFinding {
    fn validate(&self) -> Result<(), ContractError> {
        let field = format!("ProfileFinding[{}]", self.metric);
        if !(self.ratio.is_finite() && self.ratio >= 0.0) {
            return Err(refuse(
                &field,
                format!("ratio {} is not a size", self.ratio),
            ));
        }
        // A tolerance to be judged against, and a scope, which no pose metric has: so the list can
        // only ever name a shot number, and mechanics stays its own panel (Decision 12).
        if target_for(&self.metric).is_none() || scope_of(&self.metric).is_none() {
            return Err(refuse(
                &field,
                "is not a launch-monitor metric with a tolerance and a scope",
            ));
        }
        Ok(())
    }
}

/// Weaknesses first, then strengths; no metric named twice for one kind, and none both a strength
/// and a weakness, which the two definitions cannot both establish. The order within each group is
/// `feedback::profile_rules`'s (M37 P8). At player level every metric is club-independent.
fn check_list(owner: &str, list: &[ProfileFinding], player: bool) -> Result<(), ContractError> {
    let field = format!("{owner}.list");
    each(&field, list)?;
    if let Some(pair) = list
        .windows(2)
        .find(|pair| !pair[0].kind.is_weakness() && pair[1].kind.is_weakness())
    {
        return Err(refuse(
            &field,
            format!(
                "the weakness on {} is listed after a strength",
                pair[1].metric
            ),
        ));
    }
    for (i, finding) in list.iter().enumerate() {
        let repeated = list[..i].iter().any(|earlier| {
            earlier.metric == finding.metric
                && (earlier.kind == finding.kind
                    || earlier.kind.is_weakness() != finding.kind.is_weakness())
        });
        if repeated {
            return Err(refuse(
                &field,
                format!(
                    "{} is listed twice, or as both a strength and a weakness",
                    finding.metric
                ),
            ));
        }
        if player && !is_club_independent(&finding.metric) {
            return Err(refuse(
                &field,
                format!(
                    "{} is club-only, so it has no player-level entry",
                    finding.metric
                ),
            ));
        }
    }
    Ok(())
}

/// One club's grades with its list beside them. The grades do not feed the list and the list does
/// not feed the grades (ADR-034 §5.7), so they are built by two crates and only joined here, by
/// `core`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Self")]
pub struct GradedClub {
    pub grades: ClubGrades,
    /// Weaknesses, then strengths. Empty means nothing is established either way.
    #[serde(default)]
    pub list: Vec<ProfileFinding>,
}

crate::validated!(GradedClub);

impl Validate for GradedClub {
    fn validate(&self) -> Result<(), ContractError> {
        nested("GradedClub.grades", Some(&self.grades))?;
        check_list(
            &format!("GradedClub[{}]", self.grades.club),
            &self.list,
            false,
        )
    }
}

/// The player's grades with the player-level list beside them, which reads a whole-corpus baseline
/// kept to club-independent metrics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Self")]
pub struct GradedPlayer {
    pub grades: PlayerGrades,
    #[serde(default)]
    pub list: Vec<ProfileFinding>,
}

crate::validated!(GradedPlayer);

impl Validate for GradedPlayer {
    fn validate(&self) -> Result<(), ContractError> {
        nested("GradedPlayer.grades", Some(&self.grades))?;
        check_list("GradedPlayer", &self.list, true)
    }
}

/// Everything `golf-core profile` answers for one golfer, and what the phone and M29's `rmcp` tool
/// read: each club's grades and list, then the player's.
///
/// No JSON schema, as no career shape has one (the M37 plan's call 14). The phone reads this struct
/// through the bridge, and `tests/strike.rs` pins its round trip.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Self")]
pub struct GradedProfile {
    pub player_id: String,
    /// One per club with a tracked shot, in canonical bag order ([`ClubId`]'s declaration order),
    /// which [`Validate`] holds for `BagProfile`'s reason: an alphabetised bag reads as a bug.
    #[serde(default)]
    pub clubs: Vec<GradedClub>,
    pub player: GradedPlayer,
}

crate::validated!(GradedProfile);

impl Validate for GradedProfile {
    fn validate(&self) -> Result<(), ContractError> {
        each("GradedProfile.clubs", &self.clubs)?;
        for pair in self.clubs.windows(2) {
            let (before, after) = (pair[0].grades.club, pair[1].grades.club);
            if before >= after {
                return Err(refuse(
                    "GradedProfile.clubs",
                    format!("{after} is listed after {before}: each club once, in bag order"),
                ));
            }
        }
        nested("GradedProfile.player", Some(&self.player))
    }
}

fn refuse(field: &str, problem: impl Into<String>) -> ContractError {
    ContractError {
        field: field.to_string(),
        problem: problem.into(),
    }
}
