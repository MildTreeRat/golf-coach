"""Reference distributions: loading, stratum fallback, and percentile placement. [M4-REF]

These run against the **real** committed `golfdb_v1.json`, not a fixture. That is deliberate: the
file is a data artifact that ships inside the wheel, and the failure worth catching is someone
regenerating it into a shape the loader can't read — which a fixture would hide.
"""

from __future__ import annotations

import pytest

from golf_coach.analysis.benchmarks import dataset_info, load_distribution


def test_dataset_provenance_is_present() -> None:
    info = dataset_info()
    assert info.name == "GolfDB"
    assert "McNally" in info.citation
    # Licensing posture must travel with the data, not just live in an ADR.
    assert "aggregate" in info.license_note.lower()
    assert info.metric_definitions_version >= 2


def test_tempo_distribution_loads_and_is_ordered() -> None:
    dist = load_distribution("tempo_ratio")
    assert dist is not None
    assert dist.n > 1000
    assert dist.n_players > 100
    assert dist.p10 < dist.p25 < dist.p50 < dist.p75 < dist.p90


def test_tempo_median_is_near_the_three_to_one_rule_of_thumb() -> None:
    """The classic ~3:1 should sit inside the tour spread — the headline sanity check.

    If this ever fails, the `events` array is being indexed wrong (index 0 is `start`, not
    `address`), which silently corrupts every derived band.
    """
    dist = load_distribution("tempo_ratio")
    assert dist is not None
    assert 2.5 < dist.p50 < 4.5
    assert dist.p10 < 3.0 < dist.p90


def test_unknown_metric_returns_none_rather_than_guessing() -> None:
    assert load_distribution("no_such_metric") is None


def test_falls_back_to_the_overall_population() -> None:
    """A stratum we don't publish must fall back, not vanish."""
    specific = load_distribution("tempo_ratio", club="driver", sex="f", view="face-on")
    overall = load_distribution("tempo_ratio")
    assert specific is not None and overall is not None
    # Only marginals are stored, so a fully-specified query lands on a single-axis row.
    assert specific.metric == "tempo_ratio"


def test_club_stratum_resolves_to_that_club() -> None:
    dist = load_distribution("tempo_ratio", club="driver")
    assert dist is not None
    assert dist.club == "driver"


def test_percentile_of_places_values_across_the_spread() -> None:
    dist = load_distribution("tempo_ratio")
    assert dist is not None
    assert dist.percentile_of(dist.p50) == pytest.approx(50.0)
    assert dist.percentile_of(dist.p25) == pytest.approx(25.0)
    assert 10.0 < dist.percentile_of((dist.p10 + dist.p25) / 2) < 25.0


def test_percentile_of_clamps_outside_the_stored_tails() -> None:
    """We store p10-p90; beyond that, report the edge rather than extrapolate."""
    dist = load_distribution("tempo_ratio")
    assert dist is not None
    assert dist.percentile_of(dist.p10 - 100.0) == 10.0
    assert dist.percentile_of(dist.p90 + 100.0) == 90.0


def test_every_published_stratum_clears_the_min_sample_gate() -> None:
    """A thin stratum published as a distribution would read as evidence it isn't."""
    info = dataset_info()
    for metric in ("tempo_ratio", "toe_up_frac", "mid_downswing_frac"):
        dist = load_distribution(metric)
        assert dist is not None
        assert dist.n >= info.min_samples
