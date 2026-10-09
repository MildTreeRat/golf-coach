//! `analysis/stats.py`, whole. [M22 P4, M36 P11]
//!
//! One definition of "percentile", shared by the checkpoint evaluators and by the GolfDB
//! reference-aggregation script that cuts the bands they score against. Two drifting definitions
//! — one cutting a band, the other measuring the swing scored against it — produce a systematic
//! bias that is almost impossible to spot after the fact, which is why the Python keeps them in
//! one place and why this port keeps them in one place too.
//!
//! The intervals ([`mean_and_sd`], [`mean_ci`], [`sd_ci`]) live here for the same reason: a
//! personal baseline reports an interval beside every statistic, and an interval computed one way
//! in the baseline and another way anywhere else is the kind of disagreement nobody spots.
//!
//! # Why the intervals arrived a milestone after `percentile`
//!
//! M22 ported `percentile` alone because it was all the engine reached, and the career half had no
//! committed vector to gate it: ~130 lines of tabulated constants arriving before their gate is
//! code nobody can say is right. M36 recorded that gate (`spec/vectors/career/`, whose `baseline`
//! key reaches every interval, and whose `past-the-tables` case reaches both expansions past the
//! tables' last row), so they came with their first caller, [`crate::baseline`], rather than waiting
//! for §M29 as this doc used to say.
//!
//! # Only the 95% level, and no distribution object
//!
//! The critical values are three hardcoded tables up to df 30 and an expansion beyond, because the
//! Python's analysis core has no scipy (ADR-008) and this port has no numeric library in the scoring
//! path (ADR-030 §1, as `lib.rs` says). There is no `confidence` parameter: career mode reports one
//! level, and a parameter that silently ignored its argument would be worse than not having one.
//!
//! # `**` is C `pow`, and the port says so — measured, M36 P11
//!
//! A float `**` in CPython is `float_pow`, which is the platform's C `pow`, and the Python here
//! raises to the 2nd (`(value - mean) ** 2`, `sd ** 2`), the 3rd and the 5th power. On this box's
//! UCRT that `pow` is **not** the correctly rounded product. Against CPython's own answers (M36
//! P11, a scratch comparison): the product `x * x` missed 12 of 20,000 random squares, and
//! `b * b * b` missed 1,307 of the 5,940 Wilson-Hilferty values for df 31–3,000, where `powf` missed
//! none of either, and reproduced 2,400 intervals over random samples bit for bit. So the
//! crate-private `pow` calls `powf`, which is the same C `pow`, and never a product. (`powi` with a constant
//! exponent is the product; with a runtime one it happens to lower to `pow` on MSVC, which is a
//! property of a target, not a promise.)
//!
//! The exponent goes through [`std::hint::black_box`] because LLVM rewrites `pow(x, 2.0)` to
//! `x * x` whenever it can see the 2.0, and only when optimising: the same comparison with the
//! constant visible missed 0 squares in a test build and 12 in a release build. Unhidden, the gate
//! would certify arithmetic the release verbs do not run. None of this can move a value past
//! `docs/CONFORMANCE.md` §3's `RTOL`; it is a last bit, and it is here because a last bit is what a
//! report's `-0.0` or a `.Nf` at a half is decided by.

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

// ------------------------------------------------------------------------------------------------
// Confidence intervals [career mode, step 4]
// ------------------------------------------------------------------------------------------------

/// Two-sided 95% Student-t critical values, `t(0.975, df)` for df 1..30. Standard published values.
/// Above df 30 the Cornish-Fisher expansion in [`t_critical`] is accurate to ~1e-4, far below the
/// precision anything downstream quotes.
const T_CRITICAL_95: [f64; 30] = [
    12.706, 4.303, 3.182, 2.776, 2.571, 2.447, 2.365, 2.306, 2.262, 2.228, //
    2.201, 2.179, 2.160, 2.145, 2.131, 2.120, 2.110, 2.101, 2.093, 2.086, //
    2.080, 2.074, 2.069, 2.064, 2.060, 2.056, 2.052, 2.048, 2.045, 2.042,
];

/// Chi-square upper-tail critical values for df 1..30. This one is `P(X > x) = 0.025` (the large
/// one) and [`CHI2_LOWER_95`] is `P(X > x) = 0.975` (the small one). The variance interval divides
/// by these, so the *upper* critical value produces the *lower* bound, which is why they read crossed
/// over in [`sd_ci`].
const CHI2_UPPER_95: [f64; 30] = [
    5.024, 7.378, 9.348, 11.143, 12.833, 14.449, 16.013, 17.535, 19.023, 20.483, //
    21.920, 23.337, 24.736, 26.119, 27.488, 28.845, 30.191, 31.526, 32.852, 34.170, //
    35.479, 36.781, 38.076, 39.364, 40.646, 41.923, 43.195, 44.461, 45.722, 46.979,
];
const CHI2_LOWER_95: [f64; 30] = [
    0.000982, 0.0506, 0.216, 0.484, 0.831, 1.237, 1.690, 2.180, 2.700, 3.247, //
    3.816, 4.404, 5.009, 5.629, 6.262, 6.908, 7.564, 8.231, 8.907, 9.591, //
    10.283, 10.982, 11.689, 12.401, 13.120, 13.844, 14.573, 15.308, 16.047, 16.791,
];

/// The standard normal 97.5th percentile, which every expansion above df 30 converges to.
const Z_975: f64 = 1.959964;

/// Python's `x ** y` on two floats: C `pow`, as CPython's `float_pow` calls it.
///
/// The module doc carries the measurement. In short: the platform `pow` is not the correctly
/// rounded product even at `y = 2`, so neither `powi` nor `x * x` reproduces the recorded answers,
/// and `black_box` keeps LLVM from turning a visible `2.0` into `x * x` in release builds only.
///
/// `pub(crate)` for [`crate::dispersion`]'s pooled within-session spread (M36 P12), whose `sd ** 2`
/// and `(squares / degrees) ** 0.5` are the same C `pow`: the second is **not** `math.sqrt`, and a
/// `sqrt` there would be faithful only where the UCRT's `pow(x, 0.5)` happens to round as `sqrt`.
pub(crate) fn pow(x: f64, y: f64) -> f64 {
    x.powf(std::hint::black_box(y))
}

/// `t(0.975, df)`, from the table or the Cornish-Fisher expansion beyond it.
///
/// `df` is at least 1: both callers have already returned on `n < 2`. Python's `[df - 1]` at df 0
/// would read the table's *last* row by negative indexing; this indexes out of bounds and panics
/// instead, since a df of 0 here is a bug in this module and not a fact about a sample.
fn t_critical(df: usize) -> f64 {
    if df <= T_CRITICAL_95.len() {
        return T_CRITICAL_95[df - 1];
    }
    let z = Z_975;
    // Term for term as the Python groups it. `4 * df` and `96 * df**2` are Python ints, exact until
    // the division converts them, which `as f64` does at the same place; `5 * z**5` is the int 5
    // widened to a float before the product, which is what `5.0 *` is.
    z + (pow(z, 3.0) + z) / (4 * df) as f64
        + (5.0 * pow(z, 5.0) + 16.0 * pow(z, 3.0) + 3.0 * z) / (96 * df * df) as f64
}

/// A 95% chi-square critical value, from the table or Wilson-Hilferty beyond it.
///
/// Wilson-Hilferty maps chi-square onto the normal via a cube root; its error at df 30 is under 0.2%
/// against the tabulated values, and it improves from there. `df` is at least 1, as in
/// [`t_critical`].
fn chi2_critical(df: usize, upper: bool) -> f64 {
    if df <= CHI2_UPPER_95.len() {
        return if upper { CHI2_UPPER_95 } else { CHI2_LOWER_95 }[df - 1];
    }
    let z = if upper { Z_975 } else { -Z_975 };
    // `2 / (9 * df)` is int-by-int true division, which CPython rounds once from two exactly
    // representable operands: the same single rounding as dividing their `f64`s.
    let a = 2.0 / (9 * df) as f64;
    df as f64 * pow(1.0 - a + z * a.sqrt(), 3.0)
}

/// Sample mean, and the **sample** standard deviation (n − 1), or `None` for the sd when n < 2.
///
/// `n − 1` rather than `n` because these are always a sample of a golfer's swings and never the
/// population of them; there is no `n` at which they stop being a sample.
///
/// `None` on an empty input, where the Python raises `ValueError`, for [`percentile`]'s reason: an
/// empty sample has no mean, both halves refuse to name one, and the one caller that reaches this
/// with a possibly empty list ([`crate::baseline`]'s per-metric builder) never does, since a metric
/// with no sample has no entry.
///
/// Both sums are CPython's compensated `sum()` ([`pyfmt::sum`]), which is what 3.12 and later
/// compute and what the career family was recorded by; a plain fold differs in the last bits on any
/// sample whose terms do not add exactly.
pub fn mean_and_sd(values: &[f64]) -> Option<(f64, Option<f64>)> {
    let n = values.len();
    if n == 0 {
        return None;
    }
    let mean = pyfmt::sum(values) / n as f64;
    if n < 2 {
        return Some((mean, None));
    }
    let squares: Vec<f64> = values.iter().map(|value| pow(value - mean, 2.0)).collect();
    let variance = pyfmt::sum(&squares) / (n - 1) as f64;
    Some((mean, Some(variance.sqrt())))
}

/// 95% confidence interval for the mean, `(low, high)`, or `None` when n < 2.
///
/// Student-t rather than normal, because the whole point of reporting this is the small-`n` case: at
/// n = 5 the t multiplier is 2.78 against the normal's 1.96, so using `z` would report an interval
/// 30% narrower than the data supports, the exact error the interval exists to prevent.
pub fn mean_ci(values: &[f64]) -> Option<(f64, f64)> {
    let n = values.len();
    if n < 2 {
        return None;
    }
    let (mean, sd) = mean_and_sd(values)?;
    let sd = sd?;
    // `t * sd / sqrt(n)` left to right, as Python evaluates it: the product first.
    let half_width = t_critical(n - 1) * sd / (n as f64).sqrt();
    Some((mean - half_width, mean + half_width))
}

/// 95% confidence interval for the standard deviation, `(low, high)`, or `None` when n < 2.
///
/// From the chi-square interval on the variance, square-rooted. Deliberately asymmetric: the upper
/// bound sits much further from the estimate than the lower one at small `n`, and that asymmetry is
/// the honest shape of the thing, since a small sample can rule out a *large* spread far less easily
/// than a small one.
///
/// This is the interval that matters most in career mode, because spread is the claim the milestone
/// exists for. At n = 10 it runs roughly 0.69× to 1.83× the sample sd.
pub fn sd_ci(values: &[f64]) -> Option<(f64, f64)> {
    let n = values.len();
    if n < 2 {
        return None;
    }
    let (_, sd) = mean_and_sd(values)?;
    let sd = sd?;
    let df = n - 1;
    let scaled = df as f64 * pow(sd, 2.0);
    Some((
        (scaled / chi2_critical(df, true)).sqrt(),
        (scaled / chi2_critical(df, false)).sqrt(),
    ))
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

    // --- the intervals: `tests/analysis/test_stats.py`'s career-mode cases, ported -------------
    //
    // The critical values are hardcoded tables, so the failure worth catching is a mistyped or
    // mis-indexed row, which would shift every interval by a few percent and look plausible.

    fn close(actual: f64, expected: f64, abs: f64) {
        assert!(
            (actual - expected).abs() <= abs,
            "{actual} is not within {abs} of {expected}"
        );
    }

    /// The n = 20 sample the textbook cases share: df 19, t = 2.093, chi2 32.852 and 8.907.
    const TWENTY: [f64; 20] = [
        10.0, 12.0, 8.0, 14.0, 9.0, 11.0, 13.0, 7.0, 10.0, 12.0, 11.0, 9.0, 13.0, 8.0, 12.0, 10.0,
        11.0, 9.0, 14.0, 10.0,
    ];

    /// A golfer's swings are always a sample, never the population of their swings.
    #[test]
    fn the_sample_sd_uses_the_n_minus_1_denominator() {
        let (mean, sd) = mean_and_sd(&[2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0]).unwrap();
        assert_eq!(mean, 5.0);
        // `statistics.stdev` of the same list is sqrt(32 / 7).
        close(sd.unwrap(), (32.0f64 / 7.0).sqrt(), 1e-12);
        close(sd.unwrap(), 2.13809, 1e-5);
    }

    #[test]
    fn the_mean_interval_is_the_textbook_t_interval() {
        let (mean, sd) = mean_and_sd(&TWENTY).unwrap();
        let half_width = 2.093 * sd.unwrap() / 20f64.sqrt();
        let (low, high) = mean_ci(&TWENTY).unwrap();
        close(low, mean - half_width, 1e-9);
        close(high, mean + half_width, 1e-9);
    }

    #[test]
    fn the_sd_interval_is_the_textbook_chi_square_interval() {
        let sd = mean_and_sd(&TWENTY).unwrap().1.unwrap();
        let scaled = 19.0 * sd * sd;
        let (low, high) = sd_ci(&TWENTY).unwrap();
        close(low, (scaled / 32.852).sqrt(), 1e-9);
        close(high, (scaled / 8.907).sqrt(), 1e-9);
    }

    /// The whole point of reporting an interval is the small-`n` case: at n = 5 the t multiplier is
    /// 2.776 against the normal's 1.96, so a normal approximation would report an interval ~30%
    /// narrower than the data supports.
    #[test]
    fn the_mean_interval_uses_t_not_z_at_small_n() {
        let values = [1.0, 2.0, 3.0, 4.0, 5.0];
        let sd = mean_and_sd(&values).unwrap().1.unwrap();
        let (low, high) = mean_ci(&values).unwrap();
        close((high - low) / 2.0, 2.776 * sd / 5f64.sqrt(), 1e-9);
        assert!((high - low) / 2.0 > 1.96 * sd / 5f64.sqrt());
    }

    /// A small sample rules out a *large* spread far less easily than a small one.
    #[test]
    fn the_sd_interval_is_asymmetric_around_the_estimate() {
        let values: Vec<f64> = (1..=10).map(f64::from).collect();
        let sd = mean_and_sd(&values).unwrap().1.unwrap();
        let (low, high) = sd_ci(&values).unwrap();
        assert!(low < sd && sd < high);
        assert!(high - sd > sd - low);
    }

    /// The property every consumer relies on, over the table and past its end.
    #[test]
    fn intervals_narrow_as_n_grows() {
        let widths: Vec<f64> = [5, 10, 30, 60]
            .into_iter()
            .map(|n| {
                // The same spread at every n, so only `n` moves.
                let values: Vec<f64> = (0..n).map(|x| f64::from(x % 5)).collect();
                let (low, high) = mean_ci(&values).unwrap();
                high - low
            })
            .collect();
        assert!(
            widths.windows(2).all(|pair| pair[0] > pair[1]),
            "{widths:?}"
        );
    }

    /// n = 1 has no spread to estimate, and inventing one is how a baseline lies. n = 0 has no mean
    /// either: `None` where the Python raises (the doc on [`mean_and_sd`] says why).
    #[test]
    fn a_single_value_has_no_interval_and_no_value_has_no_mean() {
        assert_eq!(mean_and_sd(&[3.0]), Some((3.0, None)));
        assert_eq!(mean_ci(&[3.0]), None);
        assert_eq!(sd_ci(&[3.0]), None);
        assert_eq!(mean_and_sd(&[]), None);
        assert_eq!(mean_ci(&[]), None);
        assert_eq!(sd_ci(&[]), None);
    }

    /// df 30 is tabulated and df 31 is approximated; a step between them means a bad fallback.
    #[test]
    fn critical_values_are_continuous_where_the_tables_end() {
        close(t_critical(30), t_critical(31), 5e-3);
        assert!(t_critical(31) < t_critical(30));
        for upper in [true, false] {
            let tabulated = chi2_critical(30, upper);
            let approximated = chi2_critical(31, upper);
            assert!((approximated - tabulated).abs() <= 0.08 * tabulated);
            assert!(approximated > tabulated);
        }
    }

    /// Wilson-Hilferty against the published df 31 row it is not allowed to see.
    #[test]
    fn the_chi_square_expansion_reproduces_published_values() {
        close(chi2_critical(31, true), 48.232, 0.05);
        close(chi2_critical(31, false), 17.539, 0.05);
    }

    /// Each table's ends, against `analysis/stats.py`'s, so a row typed into the wrong slot fails.
    #[test]
    fn the_tables_are_pythons() {
        assert_eq!(
            [t_critical(1), t_critical(19), t_critical(30)],
            [12.706, 2.093, 2.042]
        );
        assert_eq!(
            [
                chi2_critical(1, true),
                chi2_critical(19, true),
                chi2_critical(30, true)
            ],
            [5.024, 32.852, 46.979]
        );
        assert_eq!(
            [
                chi2_critical(1, false),
                chi2_critical(19, false),
                chi2_critical(30, false)
            ],
            [0.000982, 8.907, 16.791]
        );
    }

    /// **CPython's own answers, compared with no tolerance**, on the one target they were recorded
    /// on: `analysis/stats.py` under CPython 3.13 on x86_64 Windows, whose `**` is the UCRT's `pow`.
    /// Another libm may answer a last bit differently, and so would CPython over it, which is why
    /// the pin is held to the target rather than claimed everywhere.
    ///
    /// This is the negative control for [`pow`]. Each row marked `*` is one where the product
    /// (`b * b * b`, `x * x`) gives a different f64 from CPython's: df 74's lower value is
    /// `past-the-tables`' n = 75 in the career family, which the family's `RTOL` cannot see, and the
    /// three squares are what LLVM's `pow(x, 2.0)` → `x * x` rewrite would change.
    #[cfg(all(target_os = "windows", target_env = "msvc"))]
    #[test]
    fn past_the_tables_the_answers_are_cpythons_to_the_bit() {
        let rows: [(usize, f64, f64, f64); 5] = [
            // (df, t, chi2 upper, chi2 lower)
            (31, 2.0394259233560734, 48.23468103890035, 17.52674945642329),
            (32, 2.036853823851457, 49.48326285186634, 18.279017751934713),
            (74, 1.992537150717587, 99.6813017410051, 52.095928879972945), // * lower
            (
                100,
                1.983968962657584,
                129.56398987066714,
                74.21620063884741,
            ), // * lower
            (
                1000,
                1.9623390937774106,
                1089.5321291419787,
                914.2556407640587,
            ), // * lower
        ];
        for (df, t, upper, lower) in rows {
            assert_eq!(t_critical(df).to_bits(), t.to_bits(), "t at df {df}");
            assert_eq!(
                chi2_critical(df, true).to_bits(),
                upper.to_bits(),
                "chi2 upper at df {df}"
            );
            assert_eq!(
                chi2_critical(df, false).to_bits(),
                lower.to_bits(),
                "chi2 lower at df {df}"
            );
        }
        let squares: [(f64, f64); 3] = [
            (-43.968390162412646, 1933.219333474145),
            (0.04363699456079361, 0.0019041872942987311),
            (-0.6665224046236177, 0.4442521158652495),
        ];
        for (x, cpython) in squares {
            assert_ne!(
                (x * x).to_bits(),
                cpython.to_bits(),
                "{x}: no longer a control"
            );
            assert_eq!(pow(x, 2.0).to_bits(), cpython.to_bits(), "{x} ** 2");
        }
    }
}
