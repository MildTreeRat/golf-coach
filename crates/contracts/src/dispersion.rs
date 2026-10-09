//! Which *family* of cause a golfer's own numbers point at. `contracts/dispersion.py`. [M36 P7]
//!
//! [`crate::baseline::PersonalBaseline`] says where a golfer sits and how tightly they cluster.
//! This says what that combination is evidence *for* — and it is the reason career mode is worth
//! building at all.
//!
//! # The argument
//!
//! This instrument cannot see why a club face is open: grip, lead-wrist angle and release timing are
//! all invisible to it (wrists jitter ~6x more than hips, 14.5% of frames fail the visibility gate,
//! and the club needs M2). So the causal question looks unanswerable. It is not, because **the shape
//! of a miss is itself evidence**. A miss that repeats at nearly the same size every swing is
//! produced by something that is the same every swing — grip, alignment, ball position, face at
//! address — all checkable *before* you swing. A miss that moves shot to shot cannot be produced by
//! anything static, so it points at timing and release. Separating the two needs no view of the
//! body at all, only the mean and the spread of one number over enough swings.
//!
//! # Two findings, never one verdict
//!
//! `bias` (the center sits further from the target than measurement error explains) and `scatter`
//! (the spread is larger than measurement error explains) are answered independently, because a
//! golfer can have both and the pair says more than either. Both are decided by an *interval*, never
//! a point estimate — bias needs the mean's 95% CI to clear the tolerance band entirely, scatter
//! needs the sd's 95% CI lower bound to clear the tolerance — so a tolerance set too low surfaces as
//! [`Finding::NotEstablished`], an honest "cannot tell", never as a confident wrong pattern. That is
//! not "you are fine", and every sentence rendered from it has to say so.
//!
//! # Observation and interpretation stay separate
//!
//! [`DispersionPattern`] names what was *observed*; [`PATTERN_READING`] carries the coaching
//! reading, phrased as what to check rather than as a diagnosis, because this system cannot see any
//! of the things it points at. A pattern must be structurally incapable of reading as "your grip is
//! wrong".
//!
//! Reuses [`Interval`] and [`WithheldClaim`] from [`crate::baseline`] rather than restating them,
//! since a dispersion refused for want of `n` *is* a baseline refused for want of `n`. The builder
//! is `analysis::dispersion` (M36 P12).
//!
//! # Every table is frozen Python's, word for word
//!
//! The readings, the twenty registered targets and their provenance are printed by the reports, so
//! a paraphrase here is a report that prints another sentence. `tests/aggregates.rs` holds every
//! one to a table extracted from frozen Python (`tests/data/python_tables.json`) and to every
//! sentence the career family recorded.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::baseline::{Interval, WithheldClaim};
use crate::{each, gt, nested, ContractError, Validate};

/// The three answers to "is this established?", and the third is not a failure.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Finding {
    /// The interval clears the tolerance. This is evidence, at the 95% level.
    Established,
    /// There is enough data to ask, and the answer is "cannot tell from this". Never read as "no,
    /// there is no bias" — an interval straddling the tolerance is consistent with both.
    NotEstablished,
    /// The question could not be asked at all: too few samples (see `withheld`), or the metric has
    /// no declared target to measure a bias against (see `unavailable`). Python's field default.
    #[default]
    Withheld,
}

impl Finding {
    /// Every finding, in declaration order.
    pub const ALL: [Finding; 3] = [
        Finding::Established,
        Finding::NotEstablished,
        Finding::Withheld,
    ];

    /// The wire name.
    pub const fn as_str(self) -> &'static str {
        match self {
            Finding::Established => "established",
            Finding::NotEstablished => "not_established",
            Finding::Withheld => "withheld",
        }
    }
}

impl Validate for Finding {
    fn validate(&self) -> Result<(), ContractError> {
        Ok(())
    }
}

/// What the mean and the spread together look like. An observation, not a diagnosis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DispersionPattern {
    /// A repeatable miss with no established scatter. The half that is checkable before you swing.
    Biased,
    /// Scatter with no established bias — the miss moves, so nothing static is producing it.
    Scattered,
    /// Both. They are separable problems and the repeatable one is the cheaper one to remove.
    BiasedAndScattered,
    /// Neither cleared its tolerance. "Nothing distinguishable", not "nothing wrong".
    NothingEstablished,
}

impl DispersionPattern {
    /// Every pattern, in declaration order.
    pub const ALL: [DispersionPattern; 4] = [
        DispersionPattern::Biased,
        DispersionPattern::Scattered,
        DispersionPattern::BiasedAndScattered,
        DispersionPattern::NothingEstablished,
    ];

    /// The wire name. The dispersion report prints it with `_` as a space.
    pub const fn as_str(self) -> &'static str {
        match self {
            DispersionPattern::Biased => "biased",
            DispersionPattern::Scattered => "scattered",
            DispersionPattern::BiasedAndScattered => "biased_and_scattered",
            DispersionPattern::NothingEstablished => "nothing_established",
        }
    }

    /// `PATTERN_READING[pattern]`: what this pattern is evidence for.
    pub const fn reading(self) -> &'static str {
        PATTERN_READING[self as usize].1
    }
}

impl Validate for DispersionPattern {
    fn validate(&self) -> Result<(), ContractError> {
        Ok(())
    }
}

/// What each pattern is evidence *for*, in the voice a golfer can act on. Deliberately hedged:
/// every one names something this system cannot see, so none of them may assert it.
///
/// **Deliberately metric-agnostic, and that was found by running it.** The first cut named the
/// checks outright — "grip, alignment, ball position, face at address" — which reads correctly
/// under `face_to_path_deg` and is nonsense under `head_sway_norm`, where it printed unchanged. One
/// reading serves every metric, so it may only name the *class* of cause; naming the specific check
/// is a per-metric job, and the module with the vocabulary for it is `feedback`.
///
/// In [`DispersionPattern::ALL`] order, which the `const` block below holds, so
/// [`DispersionPattern::reading`] indexes it by discriminant.
pub const PATTERN_READING: [(DispersionPattern, &str); 4] = [
    (
        DispersionPattern::Biased,
        "A miss that repeats at about the same size every swing. Whatever produces it is the same \
         every time, which points at something already set before the swing starts rather than at \
         something that happens during it. None of that is visible here, so the test is empirical: \
         change one thing at setup, hit another set, and see whether the center moves.",
    ),
    (
        DispersionPattern::Scattered,
        "The miss moves shot to shot, which is not what a setup error looks like — anything fixed \
         before the swing would produce the same error each time. Points at timing and release \
         rather than at something checkable at address.",
    ),
    (
        DispersionPattern::BiasedAndScattered,
        "Both a repeatable miss and real shot-to-shot movement. Work the repeatable half first: it \
         is checkable before you swing, and removing it makes what is left easier to read.",
    ),
    (
        DispersionPattern::NothingEstablished,
        "Neither a repeatable miss nor scatter beyond what this instrument's own error explains. \
         That is not a clean bill — it is that nothing here can be distinguished from noise at \
         this sample size.",
    ),
];

// Row `i` is the pattern whose discriminant is `i`, so a row added out of place fails the build
// rather than reading another pattern's sentence.
const _: () = {
    let mut i = 0;
    while i < DispersionPattern::ALL.len() {
        assert!(DispersionPattern::ALL[i] as usize == i);
        assert!(PATTERN_READING[i].0 as usize == i);
        i += 1;
    }
};

/// The reading when a metric has scatter but no declared target, so only half the pair exists.
pub const SCATTER_ONLY_READING: &str =
    "This varies more shot to shot than measurement error explains, which points at timing rather \
     than at anything fixed before the swing. Whether it is centered in the right place is a \
     separate question, and this metric has no target to answer it against yet.";

/// What a metric is aiming at, and how close counts as arrived.
///
/// `tolerance` is the smallest difference that can be told apart from this pipeline's own error.
/// It does two jobs and both are the same quantity used correctly: it bounds how far the center
/// must sit from the target before a bias is real, and — because measurement error inflates
/// observed spread (`sd_obs^2 ~ sd_true^2 + sd_err^2`) — it is also the level a spread must exceed
/// before the scatter is the golfer's rather than the instrument's.
///
/// A row of a `const` table rather than a serde shape: nothing on disk holds one, and the table
/// carries its metric in the row, so Python's key and `metric` field — two spellings that could
/// disagree — are one here.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MetricTarget {
    pub metric: &'static str,
    /// The value this metric is aiming at, or `None` when no target is established. `None` is a
    /// real answer for most of the panel and is never a stand-in for zero — see `no_target_reason`.
    pub target: Option<f64>,
    /// `Field(gt=0.0)`: measurement error in this metric's own units.
    pub tolerance: f64,
    /// Where `tolerance` came from, in enough detail to redo it. The verbose dispersion report
    /// prints it.
    pub provenance: &'static str,
    /// Why no target is declared, required whenever `target` is `None`; `""` when one is. A bias
    /// claim that is simply absent is indistinguishable from one nobody thought to make.
    pub no_target_reason: &'static str,
}

/// `Field(gt=0.0)`, then `_reason_required_without_target` with Python's message. Every row of
/// [`METRIC_TARGETS`] passes, which `tests` holds.
impl Validate for MetricTarget {
    fn validate(&self) -> Result<(), ContractError> {
        gt("MetricTarget.tolerance", self.tolerance, 0.0)?;
        if self.target.is_none() && self.no_target_reason.is_empty() {
            return Err(ContractError {
                field: "MetricTarget".to_string(),
                problem: format!("{}: target is None and no reason was given", self.metric),
            });
        }
        Ok(())
    }
}

/// Provenance shared by the six pose tolerances, which are **measured rather than judged**.
///
/// `scripts/golfdb/tune_spatial_metric.py` computes `noise` (median |lite − full| at identical
/// labelled instants — two estimators on the same frames, so their disagreement floors how well the
/// quantity can be known) plus `bound` (median |labelled − detected| under one estimator — the cost
/// of our own segmentation). Each tolerance is the two summed, in the metric's own units.
///
/// **An estimate of our error, not a measurement of it**: GolfDB is broadcast tour footage, ~47%
/// slow-motion, on cameras nobody here controls, and the bay is two hand-held iPhones. The first
/// thing a real bay session should revise, as [`crate::baseline::METRIC_MINIMUM_N`]'s floors are.
const TUNED: &str = "spread/error harness over 461 face-on GolfDB clips, 2026-08-12: \
     `python scripts/golfdb/tune_spatial_metric.py`, tolerance = noise + bound";

/// The same harness, corpus and command, re-run for the four hand metrics [M14 P4, P5].
///
/// A second constant because a provenance string carries a date. The re-run **reproduced all seven
/// earlier tolerances to the digit**, then all nine with P5's two registered — the only check that
/// numbers derived weeks apart came off the same instrument. `hand_separation_norm`'s noise column
/// (0.057) is the largest of any pose metric, as "wrists jitter ~6x more" predicts; it still clears
/// the ratio-of-2 screen at 4.1 (the four: 4.1, 11.5, 6.1, 4.2). Three of the four mix `x` and `y`,
/// so the harness's numbers include GolfDB's `pixel_aspect` correction and a bay clip's would not;
/// `hand_offset_from_hips_norm` is `x`-over-`x` and is instead the one the harness flagged bimodal.
const TUNED_HANDS: &str = "spread/error harness over 461 face-on GolfDB clips, 2026-09-03: \
     `python scripts/golfdb/tune_spatial_metric.py`, tolerance = noise + bound. Same run \
     reproduced the 2026-08-12 tolerances unchanged";

/// The judgment behind the two launch-monitor angle tolerances, and it is judgment: there is **no
/// instrument-error evidence for the OCR path at all** — the estimator-disagreement trick has no
/// analogue for a photographed screen. 2 degrees of face-to-path is too small for a coaching action
/// to follow from and sits clear of any plausible misread. Erring wide is the safe direction: it
/// costs claims, where erring narrow buys confident claims about the simulator's own noise.
const JUDGED_DEGREES: &str =
    "judgment, 2026-08-12: no instrument-error evidence exists for the OCR \
     path. 2 degrees is below the level a coaching action follows from. Revise from a real bay \
     session's repeats";

/// The two distance tolerances, judgment for the same reason as [`JUDGED_DEGREES`]: 5 yards is
/// below the level a coaching action follows from and clear of a misread three-digit number.
///
/// **A single constant is the wrong shape, and this is the deferral, recorded rather than hidden.**
/// Distance error plausibly scales with the club, so the right tolerance is per club and needs a
/// per-club sample this repo does not have; until then one wide constant, so the wedge end is not
/// judged against the driver's error.
const JUDGED_YARDS: &str = "judgment, 2026-08-21 (M9 P8): no instrument-error evidence exists for \
     the OCR path. 5 yards is below the level a coaching action follows from. The correct tolerance \
     is per club and scales with carry; deferred until a per-club sample exists, and erring wide \
     until then";

/// The offline tolerance, which is **not a fresh judgment — it is [`JUDGED_DEGREES`] converted**.
/// `start_line_offline_yds` is `carry * sin(start_line_deg)`, so the widest club decides it: 250 yd ×
/// sin(2°) = 8.7, rounded up to 9. At a 125-yard wedge two degrees is 4.4 yards, so the wedge is
/// judged at about twice its own geometry's tolerance — deliberate, the safe direction, and a
/// deferral to revise with [`JUDGED_YARDS`] in one pass.
const JUDGED_OFFLINE: &str = "judgment, 2026-08-21 (M9 P9): _JUDGED_DEGREES's 2 degrees carried \
     through carry * sin(angle) at the widest club in the bag - 250 yd x sin(2 deg) = 8.7, rounded \
     up. Offline error scales with carry, so one constant is the wrong shape and the wedge end is \
     judged wide on purpose; per-club deferred until a per-club sample exists";

/// The two launch conditions — **recorded for a model rather than for a verdict**, so with no
/// target a tolerance's only job is the scatter finding. 2 degrees is what the other two angle
/// fields already claim (not [`JUDGED_DEGREES`] itself, which argues from a coaching action nobody
/// takes off a launch angle). 4 mph is [`JUDGED_YARDS`]'s 5 yards through the ~1.4 yd/mph the two
/// stored shots show — 5 / 1.4 = 3.6, rounded **up**; that ratio is two shots of one club and is
/// club-specific, stated rather than hidden.
const JUDGED_LAUNCH: &str = "judgment, 2026-08-21 (M9 P10): no instrument-error evidence exists \
     for the OCR path. Launch angle takes _JUDGED_DEGREES's 2 degrees, the claim the other two \
     angle fields already make; ball speed takes _JUDGED_YARDS's 5 yards through the ~1.4 yd/mph \
     the stored shots show - 5 / 1.4 = 3.6, rounded up. That ratio is club-specific; per-club \
     deferred, erring wide";

/// The two absolute durations, and why they are not [`TUNED`]: the harness **cannot** score a
/// duration (its labelled phases carry frame indices in `start_ms`, so the comparison would be
/// frames against milliseconds). They are composed from M4-REF's instant errors instead, since a
/// duration is a subtraction of two instants. At 30 fps (33.4 ms a frame): backswing = top − motion
/// start, (2 + 7) frames = 300 ms; downswing = impact − top, (1 + 2) frames = 100 ms. 300 ms on a
/// ~267 ms downswing is about 1.1 of ratio, the same size as `tempo_ratio`'s tuned 0.943 arrived at
/// independently — the only cross-check available.
const FROM_INSTANTS: &str = "composed from M4-REF instant errors, 2026-08-20: a duration is a \
     subtraction of two instants, so its error is theirs summed - address 7 frames, top 2, impact \
     1, at the corpus's 30 fps. tune_spatial_metric.py cannot measure these; see ADR-023";

const fn row(
    metric: &'static str,
    target: Option<f64>,
    tolerance: f64,
    provenance: &'static str,
    no_target_reason: &'static str,
) -> MetricTarget {
    MetricTarget {
        metric,
        target,
        tolerance,
        provenance,
        no_target_reason,
    }
}

/// Metric → what it aims at and how close counts, in Python's declaration order. Every registered
/// metric gets a scatter finding; only the ones with a `target` get a bias finding, and the rest say
/// why in the row rather than being quietly absent.
///
/// The split is not squeamishness. Declaring a target is declaring what *good* is, which this repo
/// does in exactly one place — a benchmark band with a derivation behind it (ADR-010 §2). Two of the
/// targets are already the repo's stated position (one-sided bands `[0, high]`) and three are
/// physics or geometry rather than population. For the rest a target would be an invention.
///
/// **The six `flight_*` measurements are deliberately absent** [M15 P11]: five are a deterministic
/// function of measurements already here (a scatter finding on `flight_carry_yds` would re-report
/// `carry_distance_yds`'s spread with a model's error folded in, under a second name), and
/// `flight_spin_rpm` is solved rather than measured, so there is no honest error floor for it. The
/// pivot family is absent too (M17 P5): an uncalibrated image plane is not repeatable yet.
/// `analysis::dispersion` refuses out loud for an unregistered metric, naming this table.
pub const METRIC_TARGETS: [MetricTarget; 20] = [
    // Zero is straight by ball-flight physics, not by a distribution — the face and the path
    // agreeing produces no curvature whoever is swinging. The metric the milestone was argued from.
    row("face_to_path_deg", Some(0.0), 2.0, JUDGED_DEGREES, ""),
    row("start_line_deg", Some(0.0), 2.0, JUDGED_DEGREES, ""),
    // The same quantity in the units a golfer thinks in, and zero by geometry: a ball that starts
    // on the target line is offline by nothing. Its scatter is nearly `start_line_deg`'s; its bias
    // is the one worth reading, because it is comparable to a carry.
    row("start_line_offline_yds", Some(0.0), 9.0, JUDGED_OFFLINE, ""),
    // The two distances: **no target, and not a gap to fill later** — how far a golfer *should* hit
    // a club is not a number this repo has. A tolerance still buys the scatter finding. Do not add
    // `METRIC_MINIMUM_N` overrides for them; every override there was moved off a measured figure.
    row(
        "carry_distance_yds",
        None,
        5.0,
        JUDGED_YARDS,
        "how far a golfer should hit a given club is not a number this repo has. Every \
         distribution here is cut from GolfDB, which contains no ball flight, and a tour carry \
         band would judge an amateur against a population they are not in",
    ),
    row(
        "total_distance_yds",
        None,
        5.0,
        JUDGED_YARDS,
        "the same as carry, plus one more: the gap between them is roll, which is the ground \
         rather than the swing - a target would judge a golfer for the mat they hit off",
    ),
    // The launch conditions are what a fitting model *consumes*, so a "right" value is that model's
    // output, and the model does not exist — the question is the wrong way round.
    row(
        "ball_speed_mph",
        None,
        4.0,
        JUDGED_LAUNCH,
        "a 'right' ball speed is an output of club fitting rather than an input to coaching - the \
         club's loft and the golfer's delivery decide it, and this repo has no model that reads \
         either. Recorded so that model has data on the day it exists",
    ),
    row(
        "launch_angle_deg",
        None,
        2.0,
        JUDGED_LAUNCH,
        "optimal launch is per club, per ball speed and per spin rate, so a single number for it \
         would be wrong for every club in the bag. Same missing model as ball speed and the same \
         deferral",
    ),
    // The two one-sided magnitudes: the shipped bands are `[0.0, 0.43]` and `[0.0, 0.29]`, so a
    // target of 0 is not a new claim. **A bias here is weaker than it looks**: zero head sway is not
    // attainable, so what a bias asserts is *a consistent amount above measurement error*, not that
    // the amount is too much — that is the tour band's question, which `analysis::comparison` asks.
    row("head_sway_norm", Some(0.0), 0.060, TUNED, ""),
    row("finish_balance_norm", Some(0.0), 0.024, TUNED, ""),
    row(
        "hip_sway_norm",
        None,
        0.050,
        TUNED,
        "'less is better' is not established for it — some lateral hip travel is a weight shift \
         rather than a fault, and nothing here knows how much. It has a two-sided band as of \
         2026-08-12, which is a range rather than a target: `analysis.comparison` asks whether the \
         personal center sits inside it, so no point value has to be invented here",
    ),
    row(
        "hip_shift_at_top_norm",
        None,
        0.053,
        TUNED,
        "same as hip_sway_norm: it has a one-sided band as of 2026-08-12, but a band edge is not a \
         target, and the right amount of hip travel going back is not a number this repo has. Its \
         lower edge is omitted for being unmeasurable, not for being zero",
    ),
    row(
        "head_hip_offset_impact_norm",
        None,
        0.073,
        TUNED,
        "career mode step 4 established that the sign is readable here — a personal corpus is \
         single-handed by construction, so the camera-relative ambiguity that blocks a tour band \
         does not apply. A readable sign is not a target: staying behind the ball is right, and \
         how far behind is not established",
    ),
    row(
        "head_hip_gain_norm",
        None,
        0.080,
        TUNED,
        "it has a band (a two-sided tour range) rather than a value, and the band is the wrong \
         shape to read a target off: too little head-hip separation is the head drifting forward \
         with the hips, too much is hanging back through impact, and the right amount for one \
         golfer is not a number this repo has. `analysis.comparison` asks the band's own question \
         instead — whether the personal center's interval sits inside it — which needs no point \
         target invented here. Note the sign convention: values live in a right-handed camera \
         frame, so a personal center is only meaningful against a corpus folded the same way",
    ),
    // The one metric whose error is entirely our own segmentation: the harness reports noise 0.000,
    // because tempo reads phase instants only and both estimators were handed GolfDB's labelled
    // ones. The whole 0.943 is `bound` — the price of the address instant (M4-REF).
    row(
        "tempo_ratio",
        None,
        0.943,
        TUNED,
        "its target is a band (2.72-4.71 from 1,399 tour swings), not a value. Reading it would \
         import `benchmarks` into the personal-baseline path, which is the boundary \
         `analysis.baseline` holds on purpose (ADR-010 §2). `analysis.comparison` answers the \
         band's question directly instead — whether the center's interval sits inside it — so no \
         point target has to be invented here (career mode step 6)",
    ),
    // The hand metrics [M14 P4, P5], registered the day they were added: a metric absent from this
    // table is silent in career mode, and silent-by-omission is the one outcome nobody chose.
    row(
        "hand_separation_norm",
        None,
        0.074,
        TUNED_HANDS,
        "how far apart a golfer's hands sit on the grip is not a fault, it is a grip - \
         overlapping, interlocking and ten-finger are all played on tour and this repo has no \
         position on which. What the scatter finding buys is the thing the metric was added for: \
         a separation that moves between swings of one golfer is the wrists being mis-placed \
         rather than the hands moving, so it reads as a warning about the other hand numbers \
         rather than about the swing",
    ),
    row(
        "hand_height_norm",
        None,
        0.069,
        TUNED_HANDS,
        "how far the hands hang below the shoulders at address is set by club length and the \
         golfer's build before posture gets a say - a driver and a wedge are different numbers for \
         the same golfer standing correctly to both. There is no band, and a single value across \
         the bag would be wrong for every club in it. `narrow_to(club=)` (M9 P13) is what makes \
         the per-club version askable",
    ),
    row(
        "hand_offset_from_hips_norm",
        None,
        0.050,
        TUNED_HANDS,
        "the harness flagged this one **bimodal - 26% of 455 face-on clips negative** - which is \
         the camera-relative sign meeting a mixed-handedness corpus, exactly as `measure.py` \
         predicted and exactly what `head_hip_offset_impact_norm` was checked for and found clear \
         of. A target cut across both modes would sit in the empty middle and read every \
         left-handed golfer as a gross fault. Handedness has to be resolved before this has a \
         target; the tolerance below is unaffected, because our error in measuring the number \
         does not care which way the golfer stands",
    ),
    row(
        "trail_hand_roll_deg",
        None,
        7.4,
        TUNED_HANDS,
        "it is a proxy and is labelled one in `POSE_MEASUREMENTS`: two image points on the back of \
         one hand, standing in for rotation about the shaft, seen from the camera worst placed to \
         see it. A target would assert that a particular face-on angle is the right grip, which is \
         a claim about golf this repo has no evidence for and which MediaPipe Hands, not this, \
         would be the instrument for. The scatter finding still works: whether one golfer sets the \
         trail hand the same way twice is answerable without knowing where it should be",
    ),
    row(
        "backswing_ms",
        None,
        300.0,
        FROM_INSTANTS,
        "there is a tour median (about 901 ms) and it is deliberately not a target. Tempo is \
         already scored once, as a ratio, and declaring a target for each half would put a second \
         verdict on one fundamental and count it twice — ADR-023 is explicit that these ship as \
         distributions and never as a band. The tour distribution is still readable where it \
         belongs, in `analysis/tempo_trainer.py`, which builds a metronome from it rather than a \
         judgment",
    ),
    row(
        "downswing_ms",
        None,
        100.0,
        FROM_INSTANTS,
        "the same reason as `backswing_ms` — one fundamental is scored once. Worth noting what the \
         corpus says about this half in particular: tour downswing duration barely moves at all \
         (p10 200 ms, p90 300 ms, and a between-club sd of 6.9 ms against 47.0 ms between \
         golfers), so it looks more like a constant than any other quantity here. That is a reason \
         it makes a good metronome, not a reason to score it",
    ),
];

/// How much larger the pooled spread must be than the within-session spread before it is worth
/// saying that part of it is drift between occasions rather than shot-to-shot scatter. Judgment,
/// and deliberately loose: it only ever *adds a caveat*, so being wrong costs a sentence.
pub const SESSION_DRIFT_FACTOR: f64 = 1.25;

/// What this metric aims at, or `None` if it is not registered at all.
///
/// An unregistered metric is refused both findings rather than defaulted into one — there is no
/// tolerance for it, so neither question can be asked. One notch more conservative than
/// [`crate::baseline::minimum_n`]'s fallback: a metric added tomorrow is silent by omission rather
/// than judged against a borrowed error term.
pub fn target_for(metric: &str) -> Option<&'static MetricTarget> {
    METRIC_TARGETS.iter().find(|row| row.metric == metric)
}

/// One golfer, one metric: the shape of their miss, and what that shape is evidence for.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Self")]
pub struct MetricDispersion {
    pub name: String,
    pub unit: String,
    pub source: String,

    /// Distinct contributing artifacts. Identical to `MetricBaseline.n`.
    pub n: i64,
    pub n_sessions: i64,

    #[serde(default)]
    pub target: Option<f64>,
    /// `None` only when the metric is unregistered — see [`target_for`].
    #[serde(default)]
    pub tolerance: Option<f64>,

    // --- bias -----------------------------------------------------------------------------
    #[serde(default)]
    pub bias: Finding,
    /// The golfer's mean, carried from `MetricBaseline.mean`, so it is present only when the CENTER
    /// guard allowed it — this shape never recomputes a sealed statistic.
    #[serde(default)]
    pub center: Option<f64>,
    #[serde(default)]
    pub center_ci: Option<Interval>,
    /// `center - target`: the size and direction of the miss.
    #[serde(default)]
    pub offset: Option<f64>,

    // --- scatter --------------------------------------------------------------------------
    #[serde(default)]
    pub scatter: Finding,
    #[serde(default)]
    pub sd: Option<f64>,
    #[serde(default)]
    pub sd_ci: Option<Interval>,
    /// Pooled within-session spread, when at least two sessions carry at least two samples each.
    /// Evidence beside `sd`, which pools across occasions and so mixes scatter with drift.
    #[serde(default)]
    pub within_session_sd: Option<f64>,

    // --- the pair -------------------------------------------------------------------------
    /// Populated only when **both** findings were answerable. `Scattered` asserts that the bias was
    /// looked for and not found, which is untrue when bias was never asked.
    #[serde(default)]
    pub pattern: Option<DispersionPattern>,
    /// What to check, phrased as a check and never as a diagnosis.
    #[serde(default)]
    pub points_at: Option<String>,

    /// Things true of this reading that qualify it.
    #[serde(default)]
    pub caveats: Vec<String>,
    /// Refusals for want of `n`, inherited whole from the baseline guard.
    #[serde(default)]
    pub withheld: Vec<WithheldClaim>,
    /// Refusals no amount of swinging fixes — no declared target, no registered tolerance. Kept
    /// apart from `withheld` because the two need opposite responses: one says book another bay
    /// hour, the other says this metric needs a band before it can speak.
    #[serde(default)]
    pub unavailable: Vec<String>,
}

crate::validated!(MetricDispersion);

impl Validate for MetricDispersion {
    fn validate(&self) -> Result<(), ContractError> {
        nested("MetricDispersion.center_ci", self.center_ci.as_ref())?;
        nested("MetricDispersion.sd_ci", self.sd_ci.as_ref())?;
        each("MetricDispersion.withheld", &self.withheld)
    }
}

/// Every metric's miss-shape for one golfer, or the refusal standing in for it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Self")]
pub struct GolferDispersion {
    pub player_id: String,
    /// Metric name → dispersion, sorted by name, which a `BTreeMap` is (as
    /// [`crate::baseline::PersonalBaseline::metrics`] says).
    #[serde(default)]
    pub metrics: BTreeMap<String, MetricDispersion>,

    #[serde(default)]
    pub built_from_swings: i64,
    #[serde(default)]
    pub built_from_sessions: i64,
}

crate::validated!(GolferDispersion);

impl Validate for GolferDispersion {
    fn validate(&self) -> Result<(), ContractError> {
        for (name, metric) in &self.metrics {
            metric.validate().map_err(|e| ContractError {
                field: format!("GolferDispersion.metrics[{name:?}] -> {}", e.field),
                problem: e.problem,
            })?;
        }
        Ok(())
    }
}

impl GolferDispersion {
    /// Metrics where both findings were answerable, whatever the answers were.
    pub fn patterns_established(&self) -> usize {
        self.metrics
            .values()
            .filter(|metric| metric.pattern.is_some())
            .count()
    }

    /// True when not one metric could even be asked — the state on disk today.
    pub fn nothing_established(&self) -> bool {
        self.patterns_established() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn every_registered_target_validates_and_is_registered_once() {
        for row in &METRIC_TARGETS {
            assert_eq!(row.validate(), Ok(()), "{}", row.metric);
            assert_eq!(
                target_for(row.metric).map(|found| found.metric),
                Some(row.metric)
            );
            assert_eq!(
                METRIC_TARGETS
                    .iter()
                    .filter(|other| other.metric == row.metric)
                    .count(),
                1,
                "{} is registered twice, and target_for answers the first",
                row.metric
            );
            // A reason beside a declared target would print as a refusal the row does not make.
            assert_eq!(
                row.target.is_some(),
                row.no_target_reason.is_empty(),
                "{}",
                row.metric
            );
        }
        assert!(target_for("flight_carry_yds").is_none());
        assert!(target_for("pivot_hip_axis_drift_norm").is_none());
    }

    /// `test_a_target_less_metric_must_say_why`, and the bound beside it.
    #[test]
    fn a_target_less_metric_must_say_why() {
        let bare = row("x", None, 1.0, "test", "");
        assert_eq!(
            bare.validate().unwrap_err().to_string(),
            "MetricTarget: x: target is None and no reason was given"
        );
        assert!(row("x", Some(0.0), 1.0, "test", "").validate().is_ok());
        assert!(row("x", None, 1.0, "test", "because").validate().is_ok());
        for tolerance in [0.0, -1.0, f64::NAN] {
            let err = row("x", Some(0.0), tolerance, "test", "")
                .validate()
                .unwrap_err();
            assert_eq!(err.field, "MetricTarget.tolerance");
        }
    }

    #[test]
    fn each_pattern_reads_its_own_sentence() {
        for (pattern, sentence) in PATTERN_READING {
            assert_eq!(pattern.reading(), sentence);
            assert_eq!(
                serde_json::to_value(pattern).unwrap(),
                json!(pattern.as_str())
            );
        }
        for finding in Finding::ALL {
            assert_eq!(
                serde_json::to_value(finding).unwrap(),
                json!(finding.as_str())
            );
        }
    }

    /// The readings name a class of cause and never a specific check (`test_dispersion.py`'s
    /// `test_the_reading_names_a_class_of_cause_and_never_a_specific_check`, on the table alone).
    #[test]
    fn no_reading_names_a_specific_check() {
        let readings = PATTERN_READING
            .iter()
            .map(|(_, sentence)| *sentence)
            .chain([SCATTER_ONLY_READING]);
        for sentence in readings {
            let lowered = sentence.to_lowercase();
            for banned in ["grip", "ball position", "face at address", "head"] {
                assert!(!lowered.contains(banned), "{banned:?} in {sentence:?}");
            }
        }
    }

    /// Every field but the five a builder must name defaults as pydantic's does.
    #[test]
    fn a_bare_dispersion_is_withheld_on_both_findings() {
        let metric: MetricDispersion = serde_json::from_value(json!({
            "name": "future_metric", "unit": "u", "source": "pose:face_on", "n": 12, "n_sessions": 1,
        }))
        .unwrap();
        assert_eq!(
            (metric.bias, metric.scatter),
            (Finding::Withheld, Finding::Withheld)
        );
        assert_eq!(
            (metric.target, metric.tolerance, metric.pattern),
            (None, None, None)
        );
        assert!(metric.caveats.is_empty() && metric.withheld.is_empty());
        assert!(metric.unavailable.is_empty());
    }

    #[test]
    fn nothing_is_established_until_one_pattern_is() {
        let metric = |pattern| MetricDispersion {
            name: "head_sway_norm".into(),
            unit: "shoulder_widths".into(),
            source: "pose:face_on".into(),
            n: 12,
            n_sessions: 1,
            target: Some(0.0),
            tolerance: Some(0.06),
            bias: Finding::Withheld,
            center: None,
            center_ci: None,
            offset: None,
            scatter: Finding::Withheld,
            sd: None,
            sd_ci: None,
            within_session_sd: None,
            pattern,
            points_at: None,
            caveats: Vec::new(),
            withheld: Vec::new(),
            unavailable: Vec::new(),
        };
        let mut golfer = GolferDispersion {
            player_id: "aaron".into(),
            metrics: BTreeMap::from([("head_sway_norm".into(), metric(None))]),
            built_from_swings: 12,
            built_from_sessions: 1,
        };
        assert!(golfer.nothing_established());
        golfer.metrics.insert(
            "tempo_ratio".into(),
            metric(Some(DispersionPattern::NothingEstablished)),
        );
        assert_eq!(golfer.patterns_established(), 1);
        assert!(
            !golfer.nothing_established(),
            "a pattern of nothing is still a pattern"
        );
    }

    /// Read from JSON the interval refuses itself, as it arrives; assembled in code, one
    /// `validate()` at the root walks the tree and names the route.
    #[test]
    fn a_nested_refusal_names_the_metric_it_sits_under() {
        let given = json!({
            "player_id": "aaron",
            "metrics": {"tempo_ratio": {
                "name": "tempo_ratio", "unit": "ratio", "source": "pose:face_on",
                "n": 9, "n_sessions": 1,
                "sd_ci": {"low": 0.1, "high": 0.2, "confidence": 0.0},
            }},
        });
        let message = serde_json::from_value::<GolferDispersion>(given)
            .unwrap_err()
            .to_string();
        assert!(message.contains("0 is not > 0"), "{message}");

        let mut golfer: GolferDispersion = serde_json::from_value(json!({
            "player_id": "aaron",
            "metrics": {"tempo_ratio": {
                "name": "tempo_ratio", "unit": "ratio", "source": "pose:face_on",
                "n": 9, "n_sessions": 1, "sd_ci": {"low": 0.1, "high": 0.2},
            }},
        }))
        .unwrap();
        let metric = golfer.metrics.get_mut("tempo_ratio").unwrap();
        metric.sd_ci.as_mut().unwrap().confidence = 0.0;
        let err = golfer.validate().unwrap_err();
        assert_eq!(
            err.field,
            "GolferDispersion.metrics[\"tempo_ratio\"] -> MetricDispersion.sd_ci -> \
             Interval.confidence"
        );
    }
}
