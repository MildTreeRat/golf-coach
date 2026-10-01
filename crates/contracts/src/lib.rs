//! The shapes the analysis engine reads and writes. [M22 P2]
//!
//! The Rust half of `src/golf_coach/contracts/`, and the third crate in this workspace.
//! [ADR-008](../../../docs/decisions/008-project-structure.md)'s rule — *"modules never import
//! each other; everything imports `contracts/`"* — becomes a cargo dependency edge here:
//! `crates/analysis` will depend on this crate and this crate depends on nothing of ours, so the
//! rule is enforced by the build rather than by reading a diff (ADR-032 §1).
//!
//! Module names carry over one for one, so `swing.py` ↔ [`swing`]. Every phase of M22 is going to
//! be diffed against its Python source by eye at least once, and a port that renames things while
//! also translating them cannot be.
//!
//! # What is here, and what is deliberately not
//!
//! ADR-032 §1 scopes this crate to *what is reachable from `SwingBundleResult` and from
//! `conformance.py::run_vector`'s inputs* — the **payload** shapes. That is nine of the
//! twenty-eight modules in the Python package: [`alignment`], [`detections`], [`feedback`],
//! [`golfer`], [`intent`], [`keypoints`], [`shot`], [`swing`], [`unscored`].
//!
//! `contracts/` also holds three *registries* the engine walks — `checkpoints.py`'s
//! `CHECKPOINT_REGISTRY`, `pivots.py`'s `PIVOT_MEASUREMENT_REGISTRY` and `placements.py` — and P2
//! shipped none of them, because P2's gate cannot see them. Round-tripping a vector proves a shape
//! carries every field on disk; it says nothing about a table of checkpoint specs. They land with
//! the phase that walks them, which keeps ADR-032's *"a gate per stage"* true rather than shipping
//! untested data one phase early. [`checkpoints`] is the first to arrive: P5 ports the evaluators
//! that walk it, so the `checkpoints` stage on all 21 vectors is what proves its order and its
//! `one_sided` flags. **P5b brought the other two** — [`placements`] and [`pivots`] — with the
//! `measurements` stage's `placements` and `pivot_face_on` groups, so all three registries are now
//! here and each arrived with its walker. A `Bag` with no Rust caller would be the same mistake one
//! step further out, and ADR-030's addendum already makes that argument about implementations.
//!
//! One thing a registry's arrival does *not* settle is that every row of it is exercised. Half of
//! [`pivots::PIVOT_MEASUREMENT_REGISTRY`] is down-the-line and P7 is the phase that walks it, so the
//! five `_dtl` rows ship gated only by the structural tests beside them. That is recorded in
//! `analysis`'s `tests/measurements.rs` rather than hidden.
//!
//! **[`capability`] is the one module with no Python twin** (M32). It is ADR-034 §2's device
//! capability model, frozen Python never gains it (ADR-035 clause 4), and the data it reads,
//! `devices.json`, sits beside this crate's `Cargo.toml` rather than under `src/golf_coach/`,
//! because nothing Python reads it.
//!
//! # Pydantic constraints are runtime checks, and so are these
//!
//! ADR-032 §4: `Field(ge=…)` is a *validator*, not an annotation. A port that renders the bounds
//! as documentation silently accepts values Python rejects, and the two implementations then
//! disagree on malformed data — which no vector exercises, so nothing would catch it.
//!
//! Forty-seven of the package's seventy bounds are reachable from the payload shapes, and three of
//! its validators: [`alignment::SwingAnchors`]'s ordering rule, [`alignment::SwingAlignment`]'s
//! serialized `quality_summary`, and [`swing::SwingResult`]'s `unscored` coercion. Those are
//! exactly the three ADR-032 §4 singles out as *"not bounds"* — so on the ported surface the
//! footnote is the whole list, and the other twenty-three bounds and seven validators sit in
//! modules §8 does not port (`audio`, `baseline`, `dispersion`, `reference`, `tempo`, `bag`,
//! `club_profile`, `club_spec`, `golfer`'s name coercion).
//!
//! **Rust has no constructor hook, so the check runs at the two moments it can.** Deserialization
//! is one — [`validated!`] wires [`Validate`] into every payload type's `Deserialize` impl, which
//! is the boundary malformed data actually arrives at. A value assembled in code is the other, and
//! there [`Validate::validate`] is an explicit call the builder makes; it recurses into nested
//! models so one call at the root checks the tree, the way pydantic's per-model construction adds
//! up to the same thing.
//!
//! # One thing the gate cannot ask for
//!
//! **Serialized JSON is not byte-comparable with Python's, and was never going to be.** Python
//! writes `-1.636758133827243e-05`; `serde_json` writes the same f64 as
//! `-0.00001636758133827243`. Both are shortest-round-trip forms of one number and both parse back
//! to identical bits — they differ on when to reach for an exponent. Seventy-seven distinct floats
//! in the committed vectors take the exponent form, all of them landmark `z` values. So every
//! comparison against Python output is **structural**, over parsed values, which is what
//! `conformance.compare_results` already does and what `tests/round_trip.rs` does here. Anything
//! that wanted to hash or byte-diff an artifact across the two languages would need a formatter,
//! not a tolerance — and nothing does.

use std::fmt;

pub mod alignment;
pub mod capability;
pub mod checkpoints;
pub mod detections;
pub mod feedback;
pub mod golfer;
pub mod intent;
pub mod keypoints;
pub mod pivots;
pub mod placements;
pub mod shot;
pub mod swing;
pub mod unscored;

/// A value a contract refuses, naming the field the way pydantic's error does.
///
/// Deliberately not an enum over the bound kinds. The consumer of this is a human reading a test
/// failure or a malformed artifact's rejection, and `"CheckpointScore.score: 1.5 is not <= 1"` is
/// the whole of what they need; branching on *which* comparison failed is not a thing any caller
/// here wants to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractError {
    /// `"Model.field"`, so a failure deep in a tree says where it came from.
    pub field: String,
    /// What was wrong with it, as a clause: `"-1 is not >= 0"`.
    pub problem: String,
}

impl fmt::Display for ContractError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.field, self.problem)
    }
}

impl std::error::Error for ContractError {}

/// The runtime checks one model carries. Implemented by every payload shape, including the ones
/// that carry none.
///
/// Uniform rather than only-where-needed on purpose: [`validate`](Validate::validate) recurses into
/// nested models, and a hole in that recursion is invisible — the parent compiles, the child is
/// simply never asked. An empty body is the honest mirror of a pydantic model with no `Field`
/// constraints, and reading one tells you that directly instead of leaving you to check.
pub trait Validate {
    /// Check this value and everything it owns. `Ok(())` means it could have come out of pydantic.
    fn validate(&self) -> Result<(), ContractError>;
}

/// Wire [`Validate`] into a type's `Deserialize`, so reading one from JSON applies its bounds.
///
/// Uses serde's `remote = "Self"` trick: the derive on the type generates `Type::deserialize` and
/// `Type::serialize` as *inherent* functions rather than trait impls, and these hand-written impls
/// call through. The alternative is a mirror struct per model that exists only to be validated and
/// converted — thirty duplicated field lists, which is thirty things that drift out of step with
/// the shapes they shadow.
#[macro_export]
macro_rules! validated {
    ($ty:ty) => {
        impl serde::Serialize for $ty {
            fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                <$ty>::serialize(self, serializer)
            }
        }

        impl<'de> serde::Deserialize<'de> for $ty {
            fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let value = <$ty>::deserialize(deserializer)?;
                $crate::Validate::validate(&value).map_err(serde::de::Error::custom)?;
                Ok(value)
            }
        }
    };
}

/// `default = true`, for the two fields whose pydantic default is `True`.
///
/// `#[serde(default)]` on a `bool` is `false`, and both of the fields that want this
/// (`SwingAnchors.motion_start_detected`, `PhaseSegment.detected`) default *true* so a value nobody
/// flagged reads as detected rather than as estimated. Getting one wrong inverts an ADR-013
/// disclosure silently, which is why it is a named function rather than a literal at two sites.
pub(crate) fn yes() -> bool {
    true
}

/// An instant, carried in the lexical form it arrived in.
///
/// **A string rather than a parsed date-time, and that is a decision rather than a shortcut.**
/// Nothing on the ported surface does arithmetic with one: `grep` over `analysis/` and
/// `feedback/rules.py` finds no read of `ShotData.timestamp` or `CoachingProvenance.generated_at` at
/// all. They are carried through the engine and written back out unchanged, so the only property
/// that matters is that they come out the way they went in — which a parse-and-reformat cannot
/// promise, since pydantic writes `2026-08-10T01:38:46.828488Z` and a normalizing round trip is free
/// to answer `+00:00`. Keeping the lexeme makes that class of drift impossible and costs this crate
/// a dependency it would otherwise carry for nothing.
///
/// It sits at the crate root rather than in [`shot`] because [`feedback`] wants one too, and the
/// Python modules do not depend on each other — they each reach for `datetime`. A `Timestamp` owned
/// by `shot` would invent an edge between two modules that have none.
///
/// If a phase ever needs to *order* two shots, this becomes a parsed type and the parse gets its own
/// vectors. Until then a comparison would be lexical and wrong on any pair that disagreed about the
/// spelling of UTC, so no comparison is offered.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct Timestamp(pub String);

impl Validate for Timestamp {
    fn validate(&self) -> Result<(), ContractError> {
        Ok(())
    }
}

/// `Field(ge=bound)`. Passes when the value is absent, exactly as an unset optional does.
pub fn ge<T>(field: &str, value: impl Into<Option<T>>, bound: T) -> Result<(), ContractError>
where
    T: PartialOrd + fmt::Display,
{
    check(field, value, bound, ">=", |v, b| v >= b)
}

/// `Field(gt=bound)`.
pub fn gt<T>(field: &str, value: impl Into<Option<T>>, bound: T) -> Result<(), ContractError>
where
    T: PartialOrd + fmt::Display,
{
    check(field, value, bound, ">", |v, b| v > b)
}

/// `Field(le=bound)`.
pub fn le<T>(field: &str, value: impl Into<Option<T>>, bound: T) -> Result<(), ContractError>
where
    T: PartialOrd + fmt::Display,
{
    check(field, value, bound, "<=", |v, b| v <= b)
}

fn check<T>(
    field: &str,
    value: impl Into<Option<T>>,
    bound: T,
    symbol: &str,
    holds: fn(&T, &T) -> bool,
) -> Result<(), ContractError>
where
    T: PartialOrd + fmt::Display,
{
    match value.into() {
        // NaN fails every comparison, so it is refused by `holds` rather than by a special case —
        // which is what pydantic does too, and is the answer ADR-010 §2 wants: a quantity that is
        // not a number is not a measurement.
        Some(v) if !holds(&v, &bound) => Err(ContractError {
            field: field.to_string(),
            problem: format!("{v} is not {symbol} {bound}"),
        }),
        _ => Ok(()),
    }
}

/// Validate every element of a list of models, naming the index that failed.
///
/// The path is joined with ` -> ` rather than a dot because each model already names itself, and
/// `KeypointsFile.frames[0] -> FrameKeypoints.landmarks[0] -> Landmark.visibility` reads as the
/// route it is. A dot would read as one long attribute chain with every model name doubled.
pub fn each<T: Validate>(field: &str, values: &[T]) -> Result<(), ContractError> {
    for (i, value) in values.iter().enumerate() {
        value.validate().map_err(|e| ContractError {
            field: format!("{field}[{i}] -> {}", e.field),
            problem: e.problem,
        })?;
    }
    Ok(())
}

/// Validate an optional nested model, naming the field it hangs off.
pub fn nested<T: Validate>(field: &str, value: Option<&T>) -> Result<(), ContractError> {
    match value {
        None => Ok(()),
        Some(value) => value.validate().map_err(|e| ContractError {
            field: format!("{field} -> {}", e.field),
            problem: e.problem,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bound_names_the_field_and_the_comparison() {
        let err = ge("CheckpointScore.score", -0.5, 0.0).unwrap_err();
        assert_eq!(err.to_string(), "CheckpointScore.score: -0.5 is not >= 0");
    }

    #[test]
    fn an_absent_optional_passes_every_bound() {
        assert!(ge("x", None::<f64>, 0.0).is_ok());
        assert!(gt("x", None::<f64>, 0.0).is_ok());
        assert!(le("x", None::<f64>, 1.0).is_ok());
    }

    #[test]
    fn nan_is_refused_rather_than_compared() {
        assert!(ge("x", f64::NAN, 0.0).is_err());
        assert!(le("x", f64::NAN, 1.0).is_err());
    }

    #[test]
    fn a_timestamp_survives_the_spelling_it_arrived_in() {
        let json = "\"2026-08-10T01:38:46.828488Z\"";
        let parsed: Timestamp = serde_json::from_str(json).unwrap();
        assert_eq!(serde_json::to_string(&parsed).unwrap(), json);
    }

    #[test]
    fn gt_and_ge_differ_on_the_bound_itself() {
        assert!(ge("x", 0.0, 0.0).is_ok());
        assert!(gt("x", 0.0, 0.0).is_err());
    }
}
