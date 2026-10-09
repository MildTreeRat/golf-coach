//! What one golfer's own history says about them — and what it is not yet allowed to say.
//! `contracts/baseline.py`. [M36 P6]
//!
//! `golfdb_v1.json` says what good looks like across a tour population. This says what *normal*
//! looks like for the person actually swinging: their typical value, their spread, their movement
//! across sessions — the half of the scoring model that makes a fix testable.
//!
//! **The guard is the product, not the statistics.** Career mode was deferred for one reason: it
//! would have produced a confident-looking trend line over three points. The corpus stopped `n`
//! being inflated by re-uploads; this stops a correct `n` of 2 being rendered as a baseline anyway.
//!
//! # Withheld means absent, not flagged
//!
//! Every statistic is gated behind a claim, and when a claim is withheld **the field is `None`** —
//! not populated beside a `ready: false` a caller might forget to read, so there is no number to
//! render. `Measurement`'s principle one level up: a baseline built from two swings is structurally
//! incapable of reading as a baseline. What is *always* populated is the evidence — `n`,
//! `n_sessions` and the per-session breakdown — because that is what makes a refusal actionable.
//!
//! # Three claims, not one verdict
//!
//! They have different appetites for `n`, so one threshold would either block a defensible mean or
//! ship a spread the data cannot support: [`BaselineClaim::Center`] needs the fewest,
//! [`BaselineClaim::Spread`] more (the sample sd's relative error is `1 / sqrt(2(n-1))`, 71% at n=2
//! and still 24% at n=10), and [`BaselineClaim::Trend`] the most, **and repeated occasions rather
//! than repeated swings**, which is why sessions are gated separately.
//!
//! The builder is `analysis::baseline` (M36 P11); the guard's tables are here because
//! `analysis::dispersion` gates on the same claims and must refuse at the same floors.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{each, gt, lt, nested, ContractError, Timestamp, Validate};

/// The three things a personal history can be asked to assert, each gated separately.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BaselineClaim {
    /// Where this golfer's values sit — the mean, its interval, and the median.
    Center,
    /// How tightly they cluster. The claim career mode exists for: a tight spread points at a
    /// *static* cause (grip, setup), a wide one at timing, and those have different fixes. Also the
    /// claim most easily faked by too small an `n`, since repeating one measurement drives the
    /// variance toward zero.
    Spread,
    /// Whether the center has moved across sessions.
    Trend,
}

impl BaselineClaim {
    /// Every claim, in declaration order — the order `for claim in BaselineClaim` walks, and so the
    /// order a baseline's `ready` and `withheld` lists are each built in.
    pub const ALL: [BaselineClaim; 3] = [
        BaselineClaim::Center,
        BaselineClaim::Spread,
        BaselineClaim::Trend,
    ];

    /// The wire name.
    pub const fn as_str(self) -> &'static str {
        match self {
            BaselineClaim::Center => "center",
            BaselineClaim::Spread => "spread",
            BaselineClaim::Trend => "trend",
        }
    }
}

impl Validate for BaselineClaim {
    fn validate(&self) -> Result<(), ContractError> {
        Ok(())
    }
}

/// Default minimum samples per claim. **Judgment, documented as judgment.**
///
/// The obvious place to derive them from is M6.5's spread/error ratios, and that would be wrong:
/// that ratio is *population* spread over *instrument* error, while what binds a personal baseline
/// is the golfer's own shot-to-shot variability — unmeasured, and much larger. Deriving from tempo's
/// r = 2.4 yields a resolved personal mean at n ≈ 3, which is absurd on its face. What makes
/// judgment safe is the interval: a floor set too low shows as a visibly wide interval, not as a
/// confident wrong number.
///
/// The stated criterion is the sample sd's own relative error, `1 / sqrt(2(n-1))`: 71% at n=2, 35%
/// at n=5, 24% at n=10, 19% at n=15, 14% at n=25.
///
/// - `Center` 5 — SEM is then ~0.45 sd, and t(4) = 2.78 keeps the interval honestly wide.
/// - `Spread` 10 — sd known to ~24%: enough to tell a tight spread from a wide one, not to rank two
///   similar ones.
/// - `Trend` 12 — with the session floor doing the real work.
///
/// In [`BaselineClaim::ALL`] order, which the `const` block below holds, so a lookup indexes it by
/// discriminant. [`crate::mishit::MISHIT_MIN_CLEAN_SHOTS`] is this table's `Center` row reused, and
/// `tests` holds the two equal.
pub const DEFAULT_MINIMUM_N: [(BaselineClaim, i64); 3] = [
    (BaselineClaim::Center, 5),
    (BaselineClaim::Spread, 10),
    (BaselineClaim::Trend, 12),
];

/// Minimum **distinct sessions** per claim. `Trend` is 3 because a trend is a claim about repeated
/// occasions, and twelve swings in one bay hour are one occasion however many there are — the
/// reasoning `Distribution.n_players` encodes for the tour bands, pointed at the axis a personal
/// corpus varies on. Two sessions can only ever draw a line; three is the fewest that can disagree
/// with one. In [`BaselineClaim::ALL`] order, as [`DEFAULT_MINIMUM_N`] is.
pub const MINIMUM_SESSIONS: [(BaselineClaim, i64); 3] = [
    (BaselineClaim::Center, 1),
    (BaselineClaim::Spread, 1),
    (BaselineClaim::Trend, 3),
];

// Row `i` of each table is the claim whose discriminant is `i`, so a row added out of place fails
// the build rather than answering another claim's floor.
const _: () = {
    let mut i = 0;
    while i < BaselineClaim::ALL.len() {
        assert!(BaselineClaim::ALL[i] as usize == i);
        assert!(DEFAULT_MINIMUM_N[i].0 as usize == i);
        assert!(MINIMUM_SESSIONS[i].0 as usize == i);
        i += 1;
    }
};

/// Per-metric overrides, for the metrics with recorded reasons to be noisier than the default.
///
/// - `tempo_ratio` — the noisiest instrument in the panel (spread/error 2.4, against 8.2 for
///   `finish_balance`), the only metric that depends on the **address** instant (a median error of
///   7 frames, 40% of clips over 10), and missing outright on ~14% of clips (ADR-013), so its `n`
///   grows more slowly than every other metric's from the same swings.
/// - `hip_shift_at_top_norm` — spread/error 3.6, the second-noisiest.
///
/// **Every launch-monitor metric takes the defaults**, the least-informed row here: there is no
/// instrument-error evidence for the OCR path at all. They are the first floors a real bay session
/// should revise; a floor moved without evidence is a refusal nobody derived.
///
/// Each override is a list of the claims it moves rather than a full row, because Python's lookup
/// falls back to the default per claim (`.get(claim, DEFAULT_MINIMUM_N[claim])`), so an override
/// naming one claim leaves the other two at the default. Both today name all three.
pub const METRIC_MINIMUM_N: [(&str, &[(BaselineClaim, i64)]); 2] = [
    (
        "tempo_ratio",
        &[
            (BaselineClaim::Center, 8),
            (BaselineClaim::Spread, 15),
            (BaselineClaim::Trend, 18),
        ],
    ),
    (
        "hip_shift_at_top_norm",
        &[
            (BaselineClaim::Center, 6),
            (BaselineClaim::Spread, 12),
            (BaselineClaim::Trend, 14),
        ],
    ),
];

/// Samples this metric needs before it may make this claim.
pub fn minimum_n(metric: &str, claim: BaselineClaim) -> i64 {
    METRIC_MINIMUM_N
        .iter()
        .find(|(name, _)| *name == metric)
        .and_then(|(_, floors)| floors.iter().find(|(c, _)| *c == claim))
        .map_or(DEFAULT_MINIMUM_N[claim as usize].1, |(_, n)| *n)
}

/// Distinct sessions this claim needs, regardless of metric.
pub fn minimum_sessions(claim: BaselineClaim) -> i64 {
    MINIMUM_SESSIONS[claim as usize].1
}

/// A confidence interval — one shape for both the mean's and the standard deviation's.
///
/// Carried rather than computed by the consumer because a point estimate without its uncertainty
/// reads as a fact, and the whole difference between a baseline over 6 swings and one over 60 is the
/// width of this.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Self")]
pub struct Interval {
    pub low: f64,
    pub high: f64,
    /// `Field(gt=0.0, lt=1.0)`: a confidence of 0 or 1 is not an interval.
    #[serde(default = "Interval::default_confidence")]
    pub confidence: f64,
}

crate::validated!(Interval);

impl Interval {
    /// pydantic's default. Every interval the builder makes is at it.
    pub const DEFAULT_CONFIDENCE: f64 = 0.95;

    fn default_confidence() -> f64 {
        Self::DEFAULT_CONFIDENCE
    }

    pub fn width(&self) -> f64 {
        self.high - self.low
    }
}

impl Validate for Interval {
    fn validate(&self) -> Result<(), ContractError> {
        gt("Interval.confidence", self.confidence, 0.0)?;
        lt("Interval.confidence", self.confidence, 1.0)
    }
}

/// A claim the guard refused, and everything needed to know when it will be allowed.
///
/// `ExcludedSwing`'s analogue: a claim that is simply missing is indistinguishable from one nobody
/// thought to make, and from a bug in the guard.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Self")]
pub struct WithheldClaim {
    pub claim: BaselineClaim,
    pub have_n: i64,
    pub need_n: i64,
    pub have_sessions: i64,
    /// 1 for the claims that do not gate on sessions at all.
    #[serde(default = "WithheldClaim::default_need_sessions")]
    pub need_sessions: i64,
    /// The sentence a human needs, naming what is still missing.
    pub reason: String,
}

crate::validated!(WithheldClaim);

impl WithheldClaim {
    fn default_need_sessions() -> i64 {
        1
    }
}

impl Validate for WithheldClaim {
    fn validate(&self) -> Result<(), ContractError> {
        Ok(())
    }
}

/// What one session contributed to one metric.
///
/// Always present, even while `Trend` is withheld — this *is* the evidence behind the refusal.
/// `mean` is the one field that is a claim rather than a count, so it alone is gated.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Self")]
pub struct SessionSample {
    pub session_id: String,
    pub captured_at: Timestamp,
    pub n: i64,
    /// This session's mean, populated only when `Trend` is ready: a per-session mean is a center
    /// claim at even smaller `n` than the pooled one.
    #[serde(default)]
    pub mean: Option<f64>,
}

crate::validated!(SessionSample);

impl Validate for SessionSample {
    fn validate(&self) -> Result<(), ContractError> {
        Ok(())
    }
}

/// One value of one metric, with the artifact it was derived from — the unit of pooling.
///
/// `artifact_key` is what makes the pooling honest: three re-uploads of one clip share a key and
/// contribute one sample. See [`crate::career::CorpusSwing::artifact_key`]. The builder's working
/// shape; no vector holds one.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Self")]
pub struct MetricSample {
    pub metric: String,
    pub value: f64,
    pub unit: String,
    pub source: String,
    pub session_id: String,
    pub captured_at: Timestamp,
    pub artifact_key: String,
    /// `session/swing` of the swing that carried it.
    pub swing_ref: String,
}

crate::validated!(MetricSample);

impl Validate for MetricSample {
    fn validate(&self) -> Result<(), ContractError> {
        Ok(())
    }
}

/// One golfer, one metric: what their own history supports saying, and what it does not.
///
/// Read the `None`s as refusals, not as missing data — every one is itemised in `withheld` with the
/// `n` it is waiting for.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Self")]
pub struct MetricBaseline {
    pub name: String,
    pub unit: String,
    /// Carried from `Measurement.source`, so provenance survives.
    pub source: String,

    /// Distinct contributing artifacts, never swing directories.
    pub n: i64,
    /// Distinct sessions those artifacts came from.
    pub n_sessions: i64,

    // --- Center ---------------------------------------------------------------------------
    #[serde(default)]
    pub mean: Option<f64>,
    #[serde(default)]
    pub mean_ci: Option<Interval>,
    #[serde(default)]
    pub median: Option<f64>,

    // --- Spread ---------------------------------------------------------------------------
    #[serde(default)]
    pub sd: Option<f64>,
    #[serde(default)]
    pub sd_ci: Option<Interval>,
    #[serde(default)]
    pub minimum: Option<f64>,
    #[serde(default)]
    pub maximum: Option<f64>,

    // --- Trend ----------------------------------------------------------------------------
    /// Oldest first. Present whatever the guard says; only the per-session `mean` inside is gated.
    #[serde(default)]
    pub sessions: Vec<SessionSample>,

    #[serde(default)]
    pub ready: Vec<BaselineClaim>,
    #[serde(default)]
    pub withheld: Vec<WithheldClaim>,
}

crate::validated!(MetricBaseline);

impl Validate for MetricBaseline {
    fn validate(&self) -> Result<(), ContractError> {
        nested("MetricBaseline.mean_ci", self.mean_ci.as_ref())?;
        nested("MetricBaseline.sd_ci", self.sd_ci.as_ref())?;
        each("MetricBaseline.sessions", &self.sessions)?;
        each("MetricBaseline.withheld", &self.withheld)
    }
}

impl MetricBaseline {
    /// May this metric make this claim?
    pub fn supports(&self, claim: BaselineClaim) -> bool {
        self.ready.contains(&claim)
    }
}

/// Every metric one golfer's own history has an opinion about — or is refusing to have one.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Self")]
pub struct PersonalBaseline {
    pub player_id: String,

    /// Metric name -> baseline, sorted by name. A `BTreeMap` is that order: the builder inserts in
    /// `sorted(samples)`, and a `str` sorts by code point as UTF-8 bytes do, so iterating this is
    /// iterating Python's dict.
    #[serde(default)]
    pub metrics: BTreeMap<String, MetricBaseline>,

    /// Distinct swings that contributed at least one value. Not `CareerCorpus::distinct_swings`: an
    /// unanalyzed, stale or outdated swing is a real swing that contributed nothing, and this is the
    /// count that explains the `n`s.
    #[serde(default)]
    pub built_from_swings: i64,
    /// Distinct sessions those contributing swings came from.
    #[serde(default)]
    pub built_from_sessions: i64,
}

crate::validated!(PersonalBaseline);

impl Validate for PersonalBaseline {
    fn validate(&self) -> Result<(), ContractError> {
        for (name, metric) in &self.metrics {
            metric.validate().map_err(|e| ContractError {
                field: format!("PersonalBaseline.metrics[{name:?}] -> {}", e.field),
                problem: e.problem,
            })?;
        }
        Ok(())
    }
}

impl PersonalBaseline {
    /// Claims ready across every metric.
    pub fn claims_ready(&self) -> usize {
        self.metrics.values().map(|metric| metric.ready.len()).sum()
    }

    /// True when the guard refused every claim on every metric — the state on disk today.
    pub fn nothing_sayable(&self) -> bool {
        self.claims_ready() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mishit::MISHIT_MIN_CLEAN_SHOTS;
    use serde_json::json;

    /// The tables as frozen Python holds them (`contracts/baseline.py:93–138`), so a floor moved on
    /// one side only fails here. The career family is the other half of this pin: `tests/career.rs`
    /// holds every refusal and every ready claim it recorded to these lookups.
    #[test]
    fn the_floors_are_pythons() {
        use BaselineClaim::{Center, Spread, Trend};
        let rows = [
            ("head_sway_norm", [5, 10, 12]),
            ("carry_distance_yds", [5, 10, 12]),
            ("tempo_ratio", [8, 15, 18]),
            ("hip_shift_at_top_norm", [6, 12, 14]),
        ];
        for (metric, floors) in rows {
            for (claim, floor) in [Center, Spread, Trend].into_iter().zip(floors) {
                assert_eq!(minimum_n(metric, claim), floor, "{metric} {claim:?}");
            }
        }
        assert_eq!(
            BaselineClaim::ALL.map(minimum_sessions),
            [1, 1, 3],
            "sessions"
        );
    }

    /// `contracts/mishit.py` says the mishit floor is this table's CENTER row reused; P5 left the
    /// literal for this phase to hold equal.
    #[test]
    fn the_mishit_floor_is_the_default_center_floor() {
        assert_eq!(
            i64::try_from(MISHIT_MIN_CLEAN_SHOTS).unwrap(),
            DEFAULT_MINIMUM_N[BaselineClaim::Center as usize].1
        );
    }

    #[test]
    fn claims_are_the_wire_names_in_declaration_order() {
        let names = BaselineClaim::ALL.map(|claim| serde_json::to_value(claim).unwrap());
        assert_eq!(names, [json!("center"), json!("spread"), json!("trend")]);
        for claim in BaselineClaim::ALL {
            assert_eq!(serde_json::to_value(claim).unwrap(), json!(claim.as_str()));
        }
    }

    /// `Field(gt=0.0, lt=1.0)`, both ends open, and the default when the key is absent.
    #[test]
    fn a_confidence_must_lie_strictly_inside_zero_and_one() {
        let read = |value: serde_json::Value| serde_json::from_value::<Interval>(value);
        let interval = read(json!({"low": 1.0, "high": 2.0})).unwrap();
        assert_eq!(interval.confidence, 0.95);
        assert_eq!(interval.width(), 1.0);
        assert!(read(json!({"low": 1.0, "high": 2.0, "confidence": 0.5})).is_ok());
        for refused in [0.0, 1.0, -0.1, 1.5] {
            let message = read(json!({"low": 1.0, "high": 2.0, "confidence": refused}))
                .unwrap_err()
                .to_string();
            assert!(message.contains("Interval.confidence"), "{message}");
        }
        assert!(interval.validate().is_ok());
        let nan = Interval {
            confidence: f64::NAN,
            ..interval
        };
        assert!(nan.validate().is_err());
    }

    /// A refused interval deep in a baseline names its route.
    #[test]
    fn a_nested_refusal_names_the_metric_it_sits_under() {
        let baseline = json!({
            "player_id": "aaron",
            "metrics": {"tempo_ratio": {
                "name": "tempo_ratio", "unit": "ratio", "source": "pose:face_on",
                "n": 9, "n_sessions": 1,
                "mean_ci": {"low": 2.0, "high": 3.0, "confidence": 1.0},
            }},
        });
        let message = serde_json::from_value::<PersonalBaseline>(baseline)
            .unwrap_err()
            .to_string();
        assert!(message.contains("1 is not < 1"), "{message}");
    }

    #[test]
    fn nothing_is_sayable_until_one_claim_is_ready() {
        let metric = |ready: Vec<BaselineClaim>| MetricBaseline {
            name: "head_sway_norm".into(),
            unit: "shoulder_widths".into(),
            source: "pose:face_on".into(),
            n: 5,
            n_sessions: 1,
            mean: None,
            mean_ci: None,
            median: None,
            sd: None,
            sd_ci: None,
            minimum: None,
            maximum: None,
            sessions: Vec::new(),
            ready,
            withheld: Vec::new(),
        };
        let mut baseline = PersonalBaseline {
            player_id: "aaron".into(),
            metrics: BTreeMap::from([("head_sway_norm".into(), metric(Vec::new()))]),
            built_from_swings: 5,
            built_from_sessions: 1,
        };
        assert!(baseline.nothing_sayable());
        baseline.metrics.insert(
            "tempo_ratio".into(),
            metric(vec![BaselineClaim::Center, BaselineClaim::Spread]),
        );
        assert_eq!(baseline.claims_ready(), 2);
        assert!(!baseline.nothing_sayable());
        assert!(baseline.metrics["tempo_ratio"].supports(BaselineClaim::Spread));
        assert!(!baseline.metrics["tempo_ratio"].supports(BaselineClaim::Trend));
    }
}
