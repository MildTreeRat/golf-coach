"""The shared percentile definition. [M4-REF]

`percentile` cuts benchmark bands *and* summarizes the swing measured against them, so a change
here shifts both sides at once. These tests pin the convention (linear interpolation between
adjacent ranks — numpy's default) rather than just a few values.
"""

from __future__ import annotations

import pytest

from golf_coach.analysis.stats import percentile


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
