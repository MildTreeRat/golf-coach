//! Where a golfer's own center sits in the tour population. `contracts/comparison.py`. [M36 P7]
//!
//! [`crate::baseline::PersonalBaseline`] says where this golfer sits; a stored `Distribution` says
//! where 122 tour players sit. This is the join, and the last piece of reasoning career mode needs:
//! it turns "a consistent 0.31 of head sway" into "and that sits inside the range tour swings
//! occupy". `MetricBaseline` mirrors `Distribution`'s shape on purpose, so the join is a lookup.
//!
//! It was kept out of the baseline and the dispersion on purpose: neither of their builders reads
//! `benchmarks` at all, which is what stops a personal statistic from quietly becoming a change to
//! how a swing is scored (ADR-010 §2). Landing the join in its own module keeps that property — the
//! boundary moved by one layer, it did not dissolve.
//!
//! # A tour band is a description, not a target
//!
//! Career mode exists because *"a 15-handicap held to a tour p10–p90 fails everything forever"*. So
//! [`Standing::Outside`] must not read as a fault, and nothing here carries a `score` or a `passed`.
//! What this shape asserts is placement in a population and nothing else.
//!
//! # Decided by the interval, like everything else in this milestone
//!
//! [`Standing`] is read off the mean's 95% CI, never off the mean. A center a hair outside p90 with
//! an interval straddling it has not been shown to be outside anything, and
//! [`Standing::Straddles`] says so — the discipline `dispersion` applies to bias and scatter, with
//! the same payoff: too small an `n` surfaces as an honest "cannot tell".
//!
//! # Refusals, for different reasons
//!
//! Refusals are in `unavailable` rather than absent, because a comparison nobody made and a
//! comparison this repo declines to make are different facts. Reuses [`Interval`] and
//! [`WithheldClaim`]: a standing refused for want of `n` *is* a baseline refused for want of `n`,
//! and it should print the same sentence. The builder is `analysis::comparison` (M36 P12); every
//! sentence here is frozen Python's, held by `tests/aggregates.rs`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::baseline::{Interval, WithheldClaim};
use crate::career::{LAUNCH_MONITOR_SOURCE_PREFIX, MODEL_SOURCE_PREFIX};
use crate::{each, nested, ContractError, Validate};

/// Where the golfer's center sits relative to the tour band. Placement, never a verdict.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Standing {
    /// The whole 95% interval sits within p10–p90: a typical value the tour population routinely
    /// produces.
    Inside,
    /// The whole interval sits beyond one edge. **Not a fault** — see [`STANDING_READING`].
    Outside,
    /// The interval crosses an edge, so the data does not settle which side the center is on. An
    /// honest "cannot tell", exactly like `Finding::NotEstablished`.
    Straddles,
    /// The question could not be asked: no center survived the baseline guard (see `withheld`), or
    /// this metric has no population it may be compared against (see `unavailable`). Python's
    /// field default.
    #[default]
    Withheld,
}

impl Standing {
    /// Every standing, in declaration order.
    pub const ALL: [Standing; 4] = [
        Standing::Inside,
        Standing::Outside,
        Standing::Straddles,
        Standing::Withheld,
    ];

    /// The wire name.
    pub const fn as_str(self) -> &'static str {
        match self {
            Standing::Inside => "inside",
            Standing::Outside => "outside",
            Standing::Straddles => "straddles",
            Standing::Withheld => "withheld",
        }
    }

    /// `STANDING_READING[standing]`, or `None` for [`Standing::Withheld`], which has no reading:
    /// Python's builder looks one up only once a center was placed, so its `KeyError` is never met
    /// and this answers the absence instead.
    pub fn reading(self) -> Option<&'static str> {
        STANDING_READING
            .get(self as usize)
            .map(|(_, sentence)| *sentence)
    }
}

impl Validate for Standing {
    fn validate(&self) -> Result<(), ContractError> {
        Ok(())
    }
}

/// What each placed standing means, in the voice a golfer can act on — deliberately hedged,
/// because the most likely misreading of this whole module is that a tour band is a target.
///
/// **Metric-agnostic on purpose.** One reading serves every metric, so it may name only the class
/// of statement being made — the rule `dispersion::PATTERN_READING` learned when a `BIASED` text
/// naming "grip, alignment, ball position" printed unchanged under `head_sway_norm`.
///
/// In [`Standing::ALL`] order for the three placed standings, which the `const` block below holds,
/// so [`Standing::reading`] indexes it by discriminant and finds nothing for `Withheld`.
pub const STANDING_READING: [(Standing, &str); 3] = [
    (
        Standing::Inside,
        "This golfer's typical value sits inside the range tour swings occupy. That is a statement \
         about where the number falls in a population, not a pass — the band describes what tour \
         players do, and being inside it is not evidence that nothing here is worth working on.",
    ),
    (
        Standing::Outside,
        "This golfer's typical value sits outside the middle 80% of tour swings. Read it as a \
         distance from a population, not as a fault: the band describes what tour professionals \
         do, and holding an amateur to it means failing everything forever. What makes it useful \
         is direction — `outside_by` says which way this metric would move to look more like that \
         population, and by how much. On a one-sided magnitude, below the band is the good side.",
    ),
    (
        Standing::Straddles,
        "The interval around this golfer's typical value crosses the edge of the tour range, so \
         which side of it they sit on is not settled by this much data. Not 'borderline' — \
         unresolved.",
    ),
];

const _: () = {
    let mut i = 0;
    while i < Standing::ALL.len() {
        assert!(Standing::ALL[i] as usize == i);
        i += 1;
    }
    let mut i = 0;
    while i < STANDING_READING.len() {
        assert!(STANDING_READING[i].0 as usize == i);
        i += 1;
    }
    assert!(Standing::Withheld as usize == STANDING_READING.len());
};

/// Metrics that **have** a stored distribution and still may not be compared against it.
///
/// One entry, the mirror image of a baseline finding rather than a new rule.
/// `head_hip_offset_impact_norm` is the one metric a *personal* baseline can interpret and a tour
/// band cannot: its sign is camera-relative, a personal corpus is single-handed so the sign is
/// consistent within it, and the GolfDB population is not. M6.5 blocked it from becoming a
/// checkpoint for exactly that reason. A stored distribution existing is not the same as a stored
/// distribution meaning something.
pub const TOUR_COMPARISON_BLOCKED: [(&str, &str); 1] = [(
    "head_hip_offset_impact_norm",
    "its sign is camera-relative, and the tour distribution is cut from GolfDB's mixed-handedness \
     population — the reason M6.5 blocked this metric from becoming a checkpoint. A personal \
     baseline reads the sign because one golfer swings from one side; averaging left- and \
     right-handed swings puts the two signs in one number, so placing a personal center in that \
     population would compare a real quantity against a meaningless one",
)];

/// `TOUR_COMPARISON_BLOCKED.get(metric)`.
pub fn tour_comparison_blocked(metric: &str) -> Option<&'static str> {
    TOUR_COMPARISON_BLOCKED
        .iter()
        .find(|(name, _)| *name == metric)
        .map(|(_, reason)| *reason)
}

/// Why a launch-monitor metric has no population here, and it is not an oversight: every
/// distribution in this repo is derived from GolfDB, pose estimated off broadcast video with no
/// ball flight in it. A launch-monitor reference corpus is a different acquisition problem.
pub const NO_LAUNCH_MONITOR_POPULATION: &str =
    "no tour population exists for it. Every distribution in this repo comes from GolfDB, which is \
     pose estimated from broadcast video and contains no ball flight — a launch-monitor reference \
     would have to be acquired, not derived";

/// A simulated quantity, and the one refusal here that is not about missing data. ADR-027's flight
/// is a model evaluated on this shot's launch conditions, so placing it among *measured* swings
/// would report the model's agreement with itself as a fact about the golfer — the pooling hazard
/// the `flight_` prefix exists to prevent (ADR-027 §Decision 6), one layer out. Acquiring a
/// launch-monitor population would not unlock this one.
pub const NO_MODEL_POPULATION: &str =
    "it is a model output rather than a reading, and no population of model outputs is a \
     population of swings — compare the measured quantity beside it instead";

/// The generic case: a pose metric nobody cut a distribution for.
pub const NO_POPULATION: &str = "no reference distribution is stored for it";

/// Why the spread is never compared to the tour spread, though both are called `sd`. **A category
/// error, recorded rather than computed.** `Distribution.sd` is *between-player* variation (458
/// clips over 122 players, under four swings each); a personal `sd` is *within-player*, how much
/// one golfer varies shot to shot. Comparing them would come out flattering for everyone, since
/// one person always varies less than 122 people do.
pub const SPREAD_NOT_COMPARABLE: &str =
    "the tour sd measures how much 122 players differ from each other, while a personal sd \
     measures how much one golfer varies shot to shot. They are different quantities and \
     comparing them would flatter every golfer alive";

/// Why `metric` has no usable tour population, in the voice a refusal has to explain itself in.
///
/// Keyed on `Measurement.source` rather than on a list of metric names, so a launch-monitor metric
/// added tomorrow inherits the right sentence — the reason `CorpusSwing::artifact_key` dispatches
/// on the prefix too. M15 P12 registered `model:` there, and the generic sentence here would have
/// told a reader of a simulated carry that the repair is to cut a distribution for it.
pub fn no_population_reason(metric: &str, source: &str) -> String {
    let reason = if source.starts_with(LAUNCH_MONITOR_SOURCE_PREFIX) {
        NO_LAUNCH_MONITOR_POPULATION
    } else if source.starts_with(MODEL_SOURCE_PREFIX) {
        NO_MODEL_POPULATION
    } else {
        NO_POPULATION
    };
    format!("{metric}: {reason}")
}

/// One golfer, one metric: where their center sits in the tour population, or why it cannot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Self")]
pub struct MetricComparison {
    pub name: String,
    pub unit: String,
    pub source: String,

    /// Distinct contributing artifacts. Identical to `MetricBaseline.n`.
    pub n: i64,
    pub n_sessions: i64,

    // --- the golfer's side, carried and never recomputed ------------------------------------
    /// The golfer's mean, carried from `MetricBaseline.mean`, so present only when the CENTER guard
    /// allowed it.
    #[serde(default)]
    pub center: Option<f64>,
    #[serde(default)]
    pub center_ci: Option<Interval>,

    // --- the population's side --------------------------------------------------------------
    #[serde(default)]
    pub standing: Standing,
    /// [`STANDING_READING`] for the standing, when there is one.
    #[serde(default)]
    pub reading: Option<String>,

    /// Where `center` falls in the tour population, from `Distribution.percentile_of`.
    /// Informational only and deliberately off the scoring path (ADR-010 §2).
    #[serde(default)]
    pub percentile: Option<f64>,
    /// True when the percentile landed on a rail. The stored quantiles stop at p10/p90, so a center
    /// past either is "at least this extreme" rather than a rank; said out loud because a bare 90
    /// reads as one.
    #[serde(default)]
    pub percentile_clamped: bool,

    /// The population's p10.
    #[serde(default)]
    pub band_low: Option<f64>,
    /// The population's p90.
    #[serde(default)]
    pub band_high: Option<f64>,
    /// Signed distance from the nearer band edge, only when `standing` is `Outside` — negative
    /// below p10, positive above p90. What makes the direction in `STANDING_READING[Outside]` a
    /// promise the shape keeps, since `percentile` clamps at the rails.
    #[serde(default)]
    pub outside_by: Option<f64>,
    /// Clips behind the distribution.
    #[serde(default)]
    pub population_n: Option<i64>,
    /// Distinct golfers behind it — a different sample size from `population_n`, and the smaller
    /// one binds, as `MINIMUM_SESSIONS` reasons for a personal corpus.
    #[serde(default)]
    pub population_players: Option<i64>,

    /// Refusals for want of `n`, inherited whole from the CENTER guard.
    #[serde(default)]
    pub withheld: Vec<WithheldClaim>,
    /// Refusals no amount of swinging fixes — no population, or one this metric may not be placed
    /// in. Apart from `withheld` because one says book another bay hour and the other says this
    /// needs a reference corpus first.
    #[serde(default)]
    pub unavailable: Vec<String>,
}

crate::validated!(MetricComparison);

impl Validate for MetricComparison {
    fn validate(&self) -> Result<(), ContractError> {
        nested("MetricComparison.center_ci", self.center_ci.as_ref())?;
        each("MetricComparison.withheld", &self.withheld)
    }
}

/// Every metric's placement in the tour population for one golfer, or the refusal for it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Self")]
pub struct GolferStanding {
    pub player_id: String,
    /// Metric name → comparison, sorted by name.
    #[serde(default)]
    pub metrics: BTreeMap<String, MetricComparison>,

    #[serde(default)]
    pub built_from_swings: i64,
    #[serde(default)]
    pub built_from_sessions: i64,
}

crate::validated!(GolferStanding);

impl Validate for GolferStanding {
    fn validate(&self) -> Result<(), ContractError> {
        for (name, metric) in &self.metrics {
            metric.validate().map_err(|e| ContractError {
                field: format!("GolferStanding.metrics[{name:?}] -> {}", e.field),
                problem: e.problem,
            })?;
        }
        Ok(())
    }
}

impl GolferStanding {
    /// Metrics whose center could actually be placed, wherever it landed.
    pub fn placements(&self) -> usize {
        self.metrics
            .values()
            .filter(|metric| metric.standing != Standing::Withheld)
            .count()
    }

    /// True when not one metric could be placed — the state on disk today.
    pub fn nothing_placed(&self) -> bool {
        self.placements() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_placed_standing_has_its_reading_and_withheld_has_none() {
        for (standing, sentence) in STANDING_READING {
            assert_eq!(standing.reading(), Some(sentence));
        }
        assert_eq!(Standing::Withheld.reading(), None);
        for standing in Standing::ALL {
            assert_eq!(
                serde_json::to_value(standing).unwrap(),
                json!(standing.as_str())
            );
        }
    }

    /// The prefix decides the sentence, and a prefix is a prefix: `launch_monitor` with no colon
    /// is a pose metric's generic reason, as Python's `startswith` answers.
    #[test]
    fn the_source_prefix_picks_the_refusal() {
        assert_eq!(
            no_population_reason("carry_distance_yds", "launch_monitor:hd_golf"),
            format!("carry_distance_yds: {NO_LAUNCH_MONITOR_POPULATION}")
        );
        assert_eq!(
            no_population_reason("flight_carry_yds", "model:flight_v1"),
            format!("flight_carry_yds: {NO_MODEL_POPULATION}")
        );
        for source in ["pose:face_on", "population:golfdb", "launch_monitor", ""] {
            assert_eq!(
                no_population_reason("m", source),
                format!("m: {NO_POPULATION}"),
                "{source:?}"
            );
        }
    }

    #[test]
    fn only_head_hip_offset_is_blocked() {
        assert!(tour_comparison_blocked("head_hip_offset_impact_norm").is_some());
        assert!(tour_comparison_blocked("head_hip_gain_norm").is_none());
    }

    #[test]
    fn nothing_is_placed_until_one_metric_is() {
        let metric = |standing| MetricComparison {
            name: "head_sway_norm".into(),
            unit: "shoulder_widths".into(),
            source: "pose:face_on".into(),
            n: 5,
            n_sessions: 1,
            center: None,
            center_ci: None,
            standing,
            reading: None,
            percentile: None,
            percentile_clamped: false,
            band_low: None,
            band_high: None,
            outside_by: None,
            population_n: None,
            population_players: None,
            withheld: Vec::new(),
            unavailable: Vec::new(),
        };
        let mut golfer = GolferStanding {
            player_id: "aaron".into(),
            metrics: BTreeMap::from([("head_sway_norm".into(), metric(Standing::Withheld))]),
            built_from_swings: 5,
            built_from_sessions: 1,
        };
        assert!(golfer.nothing_placed());
        golfer
            .metrics
            .insert("tempo_ratio".into(), metric(Standing::Straddles));
        assert_eq!(golfer.placements(), 1, "straddling is a placement");
        assert!(!golfer.nothing_placed());
    }

    #[test]
    fn a_bare_comparison_is_withheld_and_unclamped() {
        let metric: MetricComparison = serde_json::from_value(json!({
            "name": "m", "unit": "u", "source": "pose:face_on", "n": 0, "n_sessions": 0,
        }))
        .unwrap();
        assert_eq!(metric.standing, Standing::Withheld);
        assert!(!metric.percentile_clamped);
        assert_eq!((metric.reading, metric.population_n), (None, None));
    }
}
