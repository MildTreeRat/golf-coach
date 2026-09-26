//! Which population placements exist, and how each may be spoken. [M22 P5b]
//!
//! The port of `contracts/placements.py`, and the second of the three registries
//! [`crate`]'s doc says P2 left out on purpose. It lands with the code that walks it — the
//! `placements` group of the `measurements` stage — so the order and the names below are proved by
//! committed vectors rather than by a reviewer's eye.
//!
//! [`checkpoints`](crate::checkpoints)'s counterpart for the quantities that are deliberately
//! **not** checkpoints. A placement says where one swing sits against a population of tour
//! professionals. None of them has a band, none touches `overall_score`, and that is by
//! construction: the corpus behind them is entirely tour swings, so the models know the shape of
//! swings that work and nothing whatever about swings that do not. *Unusual is not bad* is the
//! whole reading rule, and it is the opposite of how a distance-from-the-tour-population number
//! looks.
//!
//! # Order is load-bearing
//!
//! `analysis::engine` emits placements in this order and `feedback/coach.py` renders them in it,
//! so `docs/CONFORMANCE.md` §3's exact list comparison reads it as part of the answer. Append,
//! never insert.
//!
//! # The view strings live here, not in the model loader
//!
//! Both this registry and `analysis::benchmarks::trajectory` need them, and `contracts` cannot
//! import `analysis` (ADR-008), so the dependency only points one way from here. They are *not*
//! `storage.manifest.Role`'s vocabulary (`face_on`, underscored): that enum names a file uploaded
//! from a phone, this names a fitted basis, and a shared spelling would quietly merge two
//! different things.

/// The two camera positions with a fitted population behind them, spelled as
/// `analysis/benchmarks/trajectory.py` keys its artifacts.
pub const FACE_ON: &str = "face-on";
/// The second camera. See [`FACE_ON`].
pub const DOWN_THE_LINE: &str = "down-the-line";

/// One population placement's identity, and the two facts that decide how it may be read.
///
/// A `const` struct of `&'static str`, the same Rust spelling of "compile-time constant, never
/// parsed from JSON" that [`crate::checkpoints::CheckpointSpec`] uses — so no [`crate::Validate`]
/// impl and no place in the [`crate::validated!`] family.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlacementSpec {
    /// The `Measurement.name` this is recorded under; consumers match on it, so it is an
    /// identifier. The `_dtl` suffix is part of the name rather than a `view` field on the
    /// measurement because `analysis/baseline.py::pooled_samples` groups a golfer's history by
    /// name — one name for two cameras would pool two bases into one personal baseline.
    pub name: &'static str,
    /// The same quantity in a sentence, for prose a model reads.
    pub label: &'static str,
    /// Which fitted basis produced it — [`FACE_ON`] or [`DOWN_THE_LINE`]. Never compare a value
    /// against the other view's: two cameras, two populations, two scales.
    pub view: &'static str,
    /// `Measurement.unit`, so the registry and the emitted measurement cannot disagree.
    pub unit: &'static str,
    /// Whether its own exceedance rate was validated against the target on held-out players. False
    /// means the number is real but its *rate* is not: the residual models over-flag any golfer the
    /// tour basis never saw, which is every golfer using this system. An uncalibrated placement is
    /// read beside its calibrated partner and never alone.
    pub calibrated: bool,
}

/// Every population placement that ships, in the order `analysis::engine` records them.
///
/// The two Q entries are uncalibrated on purpose and ship that way rather than being withheld: a
/// residual off a fitted basis is a genuinely different question from a distance inside it, and the
/// honest handling of a quantity that cannot be calibrated yet is to record it and label it — the
/// same call ADR-010 §2 makes for a checkpoint that cannot be measured.
pub static POPULATION_PLACEMENT_REGISTRY: &[PlacementSpec] = &[
    PlacementSpec {
        name: "tour_joint_distance",
        label: "joint distance",
        view: FACE_ON,
        unit: "sd_units",
        calibrated: true,
    },
    PlacementSpec {
        name: "tour_trajectory_t2",
        label: "trajectory T2",
        view: FACE_ON,
        unit: "sd_units",
        calibrated: true,
    },
    PlacementSpec {
        name: "tour_trajectory_q",
        label: "trajectory Q",
        view: FACE_ON,
        unit: "shoulder_widths",
        calibrated: false,
    },
    PlacementSpec {
        name: "tour_trajectory_t2_dtl",
        label: "down-the-line trajectory T2",
        view: DOWN_THE_LINE,
        unit: "sd_units",
        calibrated: true,
    },
    PlacementSpec {
        name: "tour_trajectory_q_dtl",
        label: "down-the-line trajectory Q",
        view: DOWN_THE_LINE,
        unit: "shoulder_widths",
        calibrated: false,
    },
];

/// Registered placement names, in the order they are recorded.
pub fn placement_names() -> Vec<&'static str> {
    POPULATION_PLACEMENT_REGISTRY
        .iter()
        .map(|spec| spec.name)
        .collect()
}

/// Is this measurement name a placement? The question `mcp/query.py` and `feedback/coach.py` ask
/// while partitioning a whole measurement list, and the one that genuinely *is* a data condition.
///
/// Python builds a `PLACEMENTS_BY_NAME` dict once "rather than scanned per lookup". A linear scan
/// over five `&'static str`s is not worth a map here, and a `OnceLock<HashMap>` would give up the
/// `const` nature of the table besides — but the *question* is the same one, so it keeps its own
/// name instead of being spelled out at each call site.
pub fn placement_by_name(name: &str) -> Option<&'static PlacementSpec> {
    POPULATION_PLACEMENT_REGISTRY
        .iter()
        .find(|spec| spec.name == name)
}

/// The spec registered under `name`.
///
/// Panics rather than returning [`Option`], matching [`crate::checkpoints::spec_for`] and the
/// Python's `KeyError`: callers here hold a name that came out of the registry, so a miss is a
/// wiring bug and not a data condition. Use [`placement_by_name`] for the data question.
pub fn spec_for(name: &str) -> &'static PlacementSpec {
    placement_by_name(name)
        .unwrap_or_else(|| panic!("no population placement registered under {name:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_order_is_the_recorded_order() {
        assert_eq!(
            placement_names(),
            vec![
                "tour_joint_distance",
                "tour_trajectory_t2",
                "tour_trajectory_q",
                "tour_trajectory_t2_dtl",
                "tour_trajectory_q_dtl",
            ]
        );
    }

    #[test]
    fn every_view_is_one_of_the_two_fitted_bases() {
        for spec in POPULATION_PLACEMENT_REGISTRY {
            assert!(
                spec.view == FACE_ON || spec.view == DOWN_THE_LINE,
                "{} names an unfitted view {:?}",
                spec.name,
                spec.view
            );
        }
    }

    /// The uncalibrated pair is the two residuals and nothing else — a fact `caveats.py` turns into
    /// the sentence a coaching call reads, so a spec that flipped this would change the prose.
    #[test]
    fn only_the_two_residuals_are_uncalibrated() {
        let uncalibrated: Vec<&str> = POPULATION_PLACEMENT_REGISTRY
            .iter()
            .filter(|spec| !spec.calibrated)
            .map(|spec| spec.name)
            .collect();
        assert_eq!(
            uncalibrated,
            vec!["tour_trajectory_q", "tour_trajectory_q_dtl"]
        );
    }

    #[test]
    fn spec_for_finds_every_registered_name() {
        for spec in POPULATION_PLACEMENT_REGISTRY {
            assert_eq!(spec_for(spec.name), spec);
        }
    }

    #[test]
    #[should_panic(expected = "no population placement registered under \"tour_spine_angle\"")]
    fn spec_for_refuses_an_unregistered_name() {
        spec_for("tour_spine_angle");
    }

    #[test]
    fn the_data_question_answers_none_instead_of_panicking() {
        assert!(placement_by_name("tempo_ratio").is_none());
    }
}
