//! Which mechanics checkpoints exist, and what to call them. [M22 P5]
//!
//! The port of `contracts/checkpoints.py`, and one of the three registries
//! [`crate`]'s doc says P2 left out on purpose: a round-trip gate proves a *shape* carries every
//! field on disk and can say nothing about a table. It lands here now because the code that walks
//! it — `analysis::checkpoints::mechanics` — lands in the same phase, so the `checkpoints` stage
//! on all 21 vectors is what proves the order and the flags below rather than a reviewer's eye.
//!
//! **Identity only — no evaluator lives here**, exactly as in Python. The direction is ADR-008's:
//! `caveats.py` builds the coaching prose out of this table and may not import `analysis`, so the
//! table is upstream of the functions that score it.
//!
//! # Order is load-bearing
//!
//! `SwingResult.unscored` is built by walking [`CHECKPOINT_REGISTRY`], and
//! `docs/CONFORMANCE.md` §3 compares list order exactly. Append rather than insert — the same
//! sentence the Python module carries, and it means the same thing here.

/// One checkpoint's identity: what it is called, and what shape its band has.
///
/// A `const` struct of `&'static str` rather than anything owned, which is the Rust spelling of
/// Python's choice to make this a `NamedTuple` instead of a `BaseModel`: it is a compile-time
/// constant, never parsed from JSON and never validated at a boundary, so it carries no
/// [`crate::Validate`] impl and is not in the [`crate::validated!`] family.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CheckpointSpec {
    /// The name on `CheckpointScore.name` and in `SwingResult.unscored` — an identifier, not prose.
    pub name: &'static str,
    /// The same checkpoint in a sentence, for caveat text a model reads. Spaced words, because
    /// "hip shift at top" reads as English where `hip_shift_at_top` reads as a key.
    pub label: &'static str,
    /// The quantity this checkpoint judges: a key in `measure::POSE_MEASUREMENTS`, a `checkpoint`
    /// row in `benchmarks/ranges.json`, and a `Measurement.metric` on a stored swing.
    pub metric: &'static str,
    /// True when only overshoot is judged and the band is `[0, high]`, mirroring
    /// `CheckpointScore.one_sided`. False means *less is not better* — a low number on a two-sided
    /// checkpoint is a failure that looks like a strength.
    pub one_sided: bool,
}

/// Every mechanics checkpoint that ships, in the order they are evaluated and reported.
///
/// `one_sided` here must agree with what each evaluator puts on its `CheckpointScore`; the
/// `checkpoints` stage gate pins that across all 21 vectors, which is the Rust counterpart of the
/// Python test that pins the same pair.
pub static CHECKPOINT_REGISTRY: &[CheckpointSpec] = &[
    CheckpointSpec {
        name: "tempo",
        label: "tempo",
        metric: "tempo_ratio",
        one_sided: false,
    },
    CheckpointSpec {
        name: "head_sway",
        label: "head sway",
        metric: "head_sway_norm",
        one_sided: true,
    },
    CheckpointSpec {
        name: "finish_balance",
        label: "finish balance",
        metric: "finish_balance_norm",
        one_sided: true,
    },
    CheckpointSpec {
        name: "hip_sway",
        label: "hip sway",
        metric: "hip_sway_norm",
        one_sided: false,
    },
    CheckpointSpec {
        name: "hip_shift_at_top",
        label: "hip shift at top",
        metric: "hip_shift_at_top_norm",
        one_sided: true,
    },
    // Signed, and its sign is camera-relative — so it is the one checkpoint that cannot be scored
    // without knowing who swung, and the one whose metric name does not echo its own.
    CheckpointSpec {
        name: "head_stays_back",
        label: "head stays back",
        metric: "head_hip_gain_norm",
        one_sided: false,
    },
];

/// The checkpoints a **late top** is known to invalidate, so a bundle synchronized on the ball
/// strike can retire them rather than ship a number measured off the wrong frame.
///
/// **A claim about evidence, not about which frames a checkpoint reads**, which is why it is a
/// hand-held set rather than a derivation — `hip_shift_at_top` reads the top too and is
/// deliberately not a member. See the Python module for the measurement that would earn a second
/// one.
///
/// A slice rather than a set: it holds one entry, membership is tested a handful of times per
/// swing, and the ordered-scan form is the same shape [`spec_for`] already uses. P7 is the phase
/// that reads it.
pub static CONTRADICTED_BY_A_LATE_TOP: &[&str] = &["tempo"];

/// Registered checkpoint names, in evaluation order.
pub fn checkpoint_names() -> Vec<&'static str> {
    CHECKPOINT_REGISTRY.iter().map(|spec| spec.name).collect()
}

/// The spec registered under `name`.
///
/// Python raises `KeyError` here rather than returning `None`, on the grounds that every caller
/// holds a name that came out of the registry in the first place — so a miss is a wiring bug and
/// not a data condition. [`Option`] would invite a caller to handle it; a panic says the same
/// thing the `KeyError` does, at the same moment.
pub fn spec_for(name: &str) -> &'static CheckpointSpec {
    CHECKPOINT_REGISTRY
        .iter()
        .find(|spec| spec.name == name)
        .unwrap_or_else(|| panic!("no checkpoint registered under {name:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_order_is_the_reported_order() {
        assert_eq!(
            checkpoint_names(),
            vec![
                "tempo",
                "head_sway",
                "finish_balance",
                "hip_sway",
                "hip_shift_at_top",
                "head_stays_back",
            ]
        );
    }

    #[test]
    fn every_metric_is_distinct_and_named_for_its_band_row() {
        let mut metrics: Vec<&str> = CHECKPOINT_REGISTRY.iter().map(|s| s.metric).collect();
        metrics.sort_unstable();
        let before = metrics.len();
        metrics.dedup();
        assert_eq!(metrics.len(), before, "two checkpoints share a metric");
    }

    #[test]
    fn spec_for_finds_every_registered_name() {
        for spec in CHECKPOINT_REGISTRY {
            assert_eq!(spec_for(spec.name), spec);
        }
    }

    #[test]
    #[should_panic(expected = "no checkpoint registered under \"spine_angle\"")]
    fn spec_for_refuses_an_unregistered_name() {
        spec_for("spine_angle");
    }

    #[test]
    fn the_late_top_set_names_only_registered_checkpoints() {
        for name in CONTRADICTED_BY_A_LATE_TOP {
            spec_for(name);
        }
    }
}
