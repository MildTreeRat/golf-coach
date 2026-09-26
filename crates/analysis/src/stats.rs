//! `analysis/stats.py`, and only the part the engine reaches. [M22 P4]
//!
//! One definition of "percentile", shared by the checkpoint evaluators and by the GolfDB
//! reference-aggregation script that cuts the bands they score against. Two drifting definitions
//! — one cutting a band, the other measuring the swing scored against it — produce a systematic
//! bias that is almost impossible to spot after the fact, which is why the Python keeps them in
//! one place and why this port keeps them in one place too.
//!
//! # Why only `percentile`
//!
//! The Python module also holds `mean_and_sd`, `mean_ci` and `sd_ci` with their three critical-value
//! tables. They are career mode's — `analysis/baseline.py` and `analysis/dispersion.py` reach them
//! and nothing `scripts/conformance.py::run_vector` touches does. Porting them here would ship ~130
//! lines of tabulated constants that no committed vector can gate, which is the thing M22 P2
//! declined to do with the three `contracts/` registries and declined for the same reason: code
//! that arrives before its gate is code nobody can say is right. They go with their callers, in
//! [§M29](../../../../ROADMAP.md) when the lab is retired.

/// The `q`-quantile of `values` by linear interpolation between adjacent ranks.
///
/// `q` is a fraction in `[0, 1]` (so `0.9` is the 90th percentile). The convention is
/// `numpy.percentile`'s default and R's type-7, chosen in the Python so the result is continuous in
/// `q` and matches what anyone cross-checking these numbers in numpy or pandas computes.
///
/// `None` on an empty input, where the Python raises `ValueError`. The difference is deliberate and
/// it is not a widening: an empty sample has no percentile, and both halves refuse to name one. A
/// `Result` with an error type would be the literal translation and would buy nothing — the single
/// caller in the reachable engine ([`crate::measure::measure_finish_balance`]) has already
/// established its input is non-empty, so there is no error path for a message to be read on.
///
/// Out-of-range `q` is a **panic**, not a `None`, and that asymmetry is the Python's too: an empty
/// input is a fact about a clip and a `q` outside `[0, 1]` is a fact about the caller's source code.
/// Folding the second into the first would let a typo in a quantile constant read as an unmeasurable
/// swing, which is exactly the confusion ADR-010 §2 exists to prevent.
pub fn percentile(values: &[f64], q: f64) -> Option<f64> {
    assert!((0.0..=1.0).contains(&q), "q must be in [0, 1], got {q}");
    if values.is_empty() {
        return None;
    }
    let mut ordered = values.to_vec();
    // `total_cmp` rather than `partial_cmp().unwrap()`: it is total, so it cannot panic on a NaN
    // that reached here from a landmark. It differs from Python's sort in one way — it orders
    // `-0.0` before `0.0` where Python's stable sort leaves them in input order — and that
    // difference cannot reach the answer, because the two are numerically equal and this function
    // reads values rather than positions.
    ordered.sort_by(f64::total_cmp);
    if ordered.len() == 1 {
        return Some(ordered[0]);
    }

    let position = q * (ordered.len() - 1) as f64;
    // `int(position)` in Python truncates toward zero, and `position` is non-negative because `q`
    // is, so this is a floor. `as usize` truncates the same way.
    let lower = position as usize;
    let upper = (lower + 1).min(ordered.len() - 1);
    let fraction = position - lower as f64;
    Some(ordered[lower] + (ordered[upper] - ordered[lower]) * fraction)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_sample_has_no_percentile() {
        assert_eq!(percentile(&[], 0.9), None);
    }

    #[test]
    fn a_single_value_is_its_own_every_quantile() {
        assert_eq!(percentile(&[4.5], 0.0), Some(4.5));
        assert_eq!(percentile(&[4.5], 0.9), Some(4.5));
        assert_eq!(percentile(&[4.5], 1.0), Some(4.5));
    }

    #[test]
    fn the_ends_are_the_extremes_and_the_middle_interpolates() {
        let values = [4.0, 1.0, 3.0, 2.0];
        assert_eq!(percentile(&values, 0.0), Some(1.0));
        assert_eq!(percentile(&values, 1.0), Some(4.0));
        // position = 0.5 * 3 = 1.5, so halfway between the 2nd and 3rd of [1,2,3,4].
        assert_eq!(percentile(&values, 0.5), Some(2.5));
    }

    /// The one call site's quantile, on a series where p90 and `max` disagree — which is the whole
    /// reason `FINISH_DRIFT_QUANTILE` is not `max()`.
    #[test]
    fn p90_is_not_the_maximum() {
        let drifts = [0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 9.0];
        let p90 = percentile(&drifts, 0.90).unwrap();
        assert!(p90 < 9.0, "p90 {p90} should not be dragged to the outlier");
    }

    #[test]
    #[should_panic(expected = "q must be in [0, 1]")]
    fn a_quantile_outside_the_unit_interval_is_a_bug_not_a_refusal() {
        percentile(&[1.0, 2.0], 90.0);
    }
}
