"""The shared percentile definition, and the confidence intervals. [M4-REF, career mode step 4]

`percentile` cuts benchmark bands *and* summarizes the swing measured against them, so a change
here shifts both sides at once. These tests pin the convention (linear interpolation between
adjacent ranks — numpy's default) rather than just a few values.
"""

from __future__ import annotations

import math
import statistics

import pytest

from golf_coach.analysis.stats import (
    _chi2_critical,
    _t_critical,
    mean_and_sd,
    mean_ci,
    percentile,
    sd_ci,
)


def test_endpoints_are_min_and_max() -> None:
    values = [4.0, 1.0, 3.0, 2.0]
    assert percentile(values, 0.0) == 1.0
    assert percentile(values, 1.0) == 4.0


def test_median_interpolates_between_ranks() -> None:
    assert percentile([1.0, 2.0, 3.0, 4.0], 0.5) == 2.5
    assert percentile([1.0, 2.0, 3.0], 0.5) == 2.0


def test_input_order_does_not_matter() -> None:
    assert percentile([3.0, 1.0, 2.0], 0.5) == percentile([1.0, 2.0, 3.0], 0.5)


def test_p90_matches_numpy_linear_convention() -> None:
    # numpy.percentile(range(1, 11), 90) == 9.1 — the value anyone cross-checking will get.
    assert percentile([float(v) for v in range(1, 11)], 0.9) == pytest.approx(9.1)


def test_single_value_is_its_own_percentile() -> None:
    assert percentile([7.0], 0.9) == 7.0
    assert percentile([7.0], 0.0) == 7.0


def test_p90_ignores_a_lone_outlier_where_max_would_not() -> None:
    """The property `finish_balance` depends on: one wild sample must not set the result."""
    values = [1.0] * 20 + [100.0]
    assert max(values) == 100.0
    assert percentile(values, 0.9) < 2.0


def test_empty_input_raises_rather_than_returning_zero() -> None:
    # Silently returning 0.0 would look like a perfect score / a band pinned at zero.
    with pytest.raises(ValueError):
        percentile([], 0.5)


@pytest.mark.parametrize("q", [-0.1, 1.1])
def test_out_of_range_quantile_raises(q: float) -> None:
    with pytest.raises(ValueError):
        percentile([1.0, 2.0], q)


# --------------------------------------------------------------------------- confidence intervals
#
# [Career mode, step 4] `mean_ci` / `sd_ci` are what stop a personal baseline from printing a mean
# over 6 swings the same way it prints one over 60. The critical values behind them are hardcoded
# tables (no scipy on the base install, ADR-008), so the failure worth catching is a mistyped or
# mis-indexed row — which would shift every interval by a few percent and look entirely plausible.


def test_sample_sd_uses_the_n_minus_1_denominator() -> None:
    """A golfer's swings are always a sample, never the population of their swings."""
    mean, sd = mean_and_sd([2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0])
    assert mean == 5.0
    assert sd == pytest.approx(statistics.stdev([2, 4, 4, 4, 5, 5, 7, 9]))
    assert sd == pytest.approx(2.13809, abs=1e-5)


def test_mean_ci_matches_the_textbook_t_interval() -> None:
    """n=20, df=19, t(0.975)=2.093 — computed by hand against the published critical value."""
    values = [float(x) for x in (10, 12, 8, 14, 9, 11, 13, 7, 10, 12,
                                 11, 9, 13, 8, 12, 10, 11, 9, 14, 10)]
    mean, sd = mean_and_sd(values)
    assert sd is not None
    half_width = 2.093 * sd / math.sqrt(len(values))

    low, high = mean_ci(values)
    assert low == pytest.approx(mean - half_width, abs=1e-9)
    assert high == pytest.approx(mean + half_width, abs=1e-9)


def test_sd_ci_matches_the_textbook_chi_square_interval() -> None:
    """Same sample: chi2(0.025, 19) = 32.852 and chi2(0.975, 19) = 8.907."""
    values = [float(x) for x in (10, 12, 8, 14, 9, 11, 13, 7, 10, 12,
                                 11, 9, 13, 8, 12, 10, 11, 9, 14, 10)]
    _, sd = mean_and_sd(values)
    assert sd is not None
    scaled = 19 * sd**2

    low, high = sd_ci(values)
    assert low == pytest.approx(math.sqrt(scaled / 32.852), abs=1e-9)
    assert high == pytest.approx(math.sqrt(scaled / 8.907), abs=1e-9)


def test_the_mean_interval_uses_t_not_z_at_small_n() -> None:
    """The whole point of reporting an interval is the small-`n` case.

    At n=5 the t multiplier is 2.776 against the normal's 1.96, so a normal approximation would
    report an interval ~30% narrower than the data supports — the exact error this exists to stop.
    """
    values = [1.0, 2.0, 3.0, 4.0, 5.0]
    mean, sd = mean_and_sd(values)
    assert sd is not None

    low, high = mean_ci(values)
    assert (high - low) / 2 == pytest.approx(2.776 * sd / math.sqrt(5), abs=1e-9)
    assert (high - low) / 2 > 1.96 * sd / math.sqrt(5)


def test_the_sd_interval_is_asymmetric_around_the_estimate() -> None:
    """A small sample rules out a *large* spread far less easily than a small one.

    That asymmetry is the honest shape of the thing, and it is why the spread claim carries the
    higher minimum-N floor in career mode.
    """
    values = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0]
    _, sd = mean_and_sd(values)
    assert sd is not None

    low, high = sd_ci(values)
    assert low < sd < high
    assert high - sd > sd - low


def test_intervals_narrow_as_n_grows() -> None:
    """The property every consumer relies on, over the table and past its end."""
    widths = []
    for n in (5, 10, 30, 60):
        # Same spread at every n: a repeating pattern with a fixed sd, so only `n` moves.
        values = [float(x % 5) for x in range(n)]
        low, high = mean_ci(values)
        widths.append(high - low)
    assert widths == sorted(widths, reverse=True)


def test_a_single_value_has_no_interval() -> None:
    """n=1 has no spread to estimate, and inventing one is how a baseline lies."""
    assert mean_and_sd([3.0]) == (3.0, None)
    assert mean_ci([3.0]) is None
    assert sd_ci([3.0]) is None


def test_critical_values_are_continuous_where_the_table_ends() -> None:
    """df=30 is tabulated and df=31 is approximated; a step between them means a bad fallback."""
    assert _t_critical(30) == pytest.approx(_t_critical(31), abs=5e-3)
    assert _t_critical(31) < _t_critical(30)

    for upper in (True, False):
        tabulated = _chi2_critical(30, upper=upper)
        approximated = _chi2_critical(31, upper=upper)
        assert approximated == pytest.approx(tabulated, rel=0.08)
        assert approximated > tabulated


def test_the_chi_square_fallback_reproduces_published_values() -> None:
    """Wilson-Hilferty against the tabulated df=31 row it is not allowed to see."""
    assert _chi2_critical(31, upper=True) == pytest.approx(48.232, abs=0.05)
    assert _chi2_critical(31, upper=False) == pytest.approx(17.539, abs=0.05)


def test_mean_and_sd_rejects_an_empty_sample() -> None:
    with pytest.raises(ValueError):
        mean_and_sd([])
