"""Small statistical helpers for the analysis core — pure, stdlib only. [M4-REF, career mode]

One definition of "percentile", shared by the checkpoint evaluators and by the GolfDB
reference-aggregation script (`scripts/golfdb/derive_reference.py`). Two drifting definitions of
`p90` — one cutting a benchmark band, the other measuring the swing scored against it — would
produce a systematic bias that is almost impossible to spot after the fact, so they share this.

Linear interpolation between the two nearest ranks (the same convention as `numpy.percentile`'s
default and R's type-7), chosen so the result is continuous in `q` and matches what anyone
cross-checking these numbers in numpy or pandas will compute.

Career mode step 4 added `mean_ci` / `sd_ci`. They live here for the same reason `percentile` does:
a personal baseline reports an interval beside every statistic, and an interval computed one way
in the baseline and another way anywhere else is the kind of disagreement nobody spots.

No numpy, and no scipy: the analysis core runs on the base install, which is pydantic and the
stdlib (ADR-008). The critical values the two intervals need are therefore small hardcoded tables
rather than a distribution object — see `_T_CRITICAL_95`.
"""

from __future__ import annotations

import math
from collections.abc import Iterable, Sequence


def percentile(values: Iterable[float], q: float) -> float:
    """The `q`-quantile of `values` by linear interpolation between adjacent ranks.

    `q` is a fraction in `[0, 1]` (so `0.9` is the 90th percentile). Raises `ValueError` on an
    empty input — an empty sample has no percentile, and silently returning `0.0` would quietly
    poison a benchmark band.
    """
    ordered = sorted(values)
    if not ordered:
        raise ValueError("percentile() requires at least one value")
    if not 0.0 <= q <= 1.0:
        raise ValueError(f"q must be in [0, 1], got {q}")
    if len(ordered) == 1:
        return ordered[0]

    position = q * (len(ordered) - 1)
    lower = int(position)
    upper = min(lower + 1, len(ordered) - 1)
    fraction = position - lower
    return ordered[lower] + (ordered[upper] - ordered[lower]) * fraction


# --------------------------------------------------------------------------------------
# Confidence intervals [career mode, step 4]
# --------------------------------------------------------------------------------------
#
# Only the 95% level is tabulated. Career mode reports one confidence level and there is no
# argument on the table for a second one; a `confidence` parameter that silently ignored its
# argument would be worse than not having it, so `mean_ci`/`sd_ci` take none.

#: Two-sided 95% Student-t critical values, `t(0.975, df)` for df 1..30. Standard published
#: values. Above df=30 the Cornish-Fisher expansion in `_t_critical` is accurate to ~1e-4, which
#: is far below the precision anything downstream quotes.
_T_CRITICAL_95: tuple[float, ...] = (
    12.706, 4.303, 3.182, 2.776, 2.571, 2.447, 2.365, 2.306, 2.262, 2.228,
    2.201, 2.179, 2.160, 2.145, 2.131, 2.120, 2.110, 2.101, 2.093, 2.086,
    2.080, 2.074, 2.069, 2.064, 2.060, 2.056, 2.052, 2.048, 2.045, 2.042,
)

#: Chi-square upper-tail critical values for df 1..30. `_CHI2_UPPER` is `P(X > x) = 0.025` (the
#: large one) and `_CHI2_LOWER` is `P(X > x) = 0.975` (the small one). The variance interval divides
#: by these, so the *upper* critical value produces the *lower* bound — which is why they read
#: crossed over in `sd_ci`.
_CHI2_UPPER_95: tuple[float, ...] = (
    5.024, 7.378, 9.348, 11.143, 12.833, 14.449, 16.013, 17.535, 19.023, 20.483,
    21.920, 23.337, 24.736, 26.119, 27.488, 28.845, 30.191, 31.526, 32.852, 34.170,
    35.479, 36.781, 38.076, 39.364, 40.646, 41.923, 43.195, 44.461, 45.722, 46.979,
)
_CHI2_LOWER_95: tuple[float, ...] = (
    0.000982, 0.0506, 0.216, 0.484, 0.831, 1.237, 1.690, 2.180, 2.700, 3.247,
    3.816, 4.404, 5.009, 5.629, 6.262, 6.908, 7.564, 8.231, 8.907, 9.591,
    10.283, 10.982, 11.689, 12.401, 13.120, 13.844, 14.573, 15.308, 16.047, 16.791,
)

#: The standard normal 97.5th percentile, which every fallback above df=30 converges to.
_Z_975 = 1.959964


def _t_critical(df: int) -> float:
    """`t(0.975, df)`, from the table or the Cornish-Fisher expansion beyond it."""
    if df <= len(_T_CRITICAL_95):
        return _T_CRITICAL_95[df - 1]
    z = _Z_975
    return z + (z**3 + z) / (4 * df) + (5 * z**5 + 16 * z**3 + 3 * z) / (96 * df**2)


def _chi2_critical(df: int, *, upper: bool) -> float:
    """A 95% chi-square critical value, from the table or Wilson-Hilferty beyond it.

    Wilson-Hilferty maps chi-square onto the normal via a cube root; its error at df=30 is under
    0.2% against the tabulated values, and it improves from there.
    """
    if df <= len(_CHI2_UPPER_95):
        return (_CHI2_UPPER_95 if upper else _CHI2_LOWER_95)[df - 1]
    z = _Z_975 if upper else -_Z_975
    return df * (1 - 2 / (9 * df) + z * math.sqrt(2 / (9 * df))) ** 3


def mean_and_sd(values: Sequence[float]) -> tuple[float, float | None]:
    """Sample mean, and the **sample** standard deviation (n-1) or None when n < 2.

    `n-1` rather than `n` because these are always a sample of a golfer's swings and never the
    population of them — there is no `n` at which they stop being a sample.
    """
    n = len(values)
    if n == 0:
        raise ValueError("mean_and_sd() requires at least one value")
    mean = sum(values) / n
    if n < 2:
        return mean, None
    variance = sum((value - mean) ** 2 for value in values) / (n - 1)
    return mean, math.sqrt(variance)


def mean_ci(values: Sequence[float]) -> tuple[float, float] | None:
    """95% confidence interval for the mean, or None when n < 2.

    Student-t rather than normal, because the whole point of reporting this is the small-`n` case:
    at n=5 the t multiplier is 2.78 against the normal's 1.96, so using `z` would report an
    interval 30% narrower than the data supports — the exact error the interval exists to prevent.
    """
    n = len(values)
    if n < 2:
        return None
    mean, sd = mean_and_sd(values)
    if sd is None:
        return None
    half_width = _t_critical(n - 1) * sd / math.sqrt(n)
    return (mean - half_width, mean + half_width)


def sd_ci(values: Sequence[float]) -> tuple[float, float] | None:
    """95% confidence interval for the standard deviation, or None when n < 2.

    From the chi-square interval on the variance, square-rooted. Deliberately asymmetric — the
    upper bound sits much further from the estimate than the lower one at small `n`, and that
    asymmetry is the honest shape of the thing: a small sample can rule out a *large* spread far
    less easily than a small one.

    This is the interval that matters most in career mode, because spread is the claim the
    milestone exists for. At n=10 it runs roughly 0.69x to 1.83x the sample sd.
    """
    n = len(values)
    if n < 2:
        return None
    _, sd = mean_and_sd(values)
    if sd is None:
        return None
    df = n - 1
    scaled = df * sd**2
    return (
        math.sqrt(scaled / _chi2_critical(df, upper=True)),
        math.sqrt(scaled / _chi2_critical(df, upper=False)),
    )
