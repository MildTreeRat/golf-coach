"""The tour joint-distribution model: loading, placement, and the seams it must not cross.

Like `test_distributions.py`, these run against the **real** committed `joint_model_v1.json`
rather than a fixture. The artifact ships inside the wheel, so the failure worth catching is
someone re-deriving it into a shape the loader cannot read — which a fixture would hide.

They also run on a base install with no extras, which is the point of the split these tests
defend: the model is *fitted* by `scripts/golfdb/derive_joint_model.py` under the `research`
extra, and *evaluated* here by stdlib arithmetic over committed numbers (ADR-022).
"""

from __future__ import annotations

import math

from golf_coach.analysis.benchmarks import (
    joint_dataset_info,
    load_joint_model,
    placement_for,
)
from golf_coach.contracts.checkpoints import CHECKPOINT_REGISTRY


def _center_reading() -> dict[str, float]:
    """A swing sitting exactly at the tour center on every metric."""
    model = load_joint_model()
    return dict(zip(model.metrics, model.center, strict=True))


def test_dataset_provenance_is_present() -> None:
    info = joint_dataset_info()
    assert info.name == "GolfDB"
    assert "McNally" in info.citation
    # The licensing posture must travel with the artifact, not live only in an ADR (ADR-012 §2).
    assert "aggregate" in info.license_note.lower()
    assert info.metric_definitions_version >= 3


def test_the_model_covers_exactly_the_metrics_the_panel_scores() -> None:
    """The structural pin, and the reason this test file earns its place.

    A joint model fitted over five metrics while six are scored would place every swing in a
    population missing one of its own dimensions, and nothing else here would notice. Tying the
    artifact to `CHECKPOINT_REGISTRY` means adding a checkpoint fails this test until the model is
    re-derived — the same discipline `test_checkpoints.py` applies to bands.
    """
    assert set(load_joint_model().metrics) == {spec.metric for spec in CHECKPOINT_REGISTRY}


def test_the_shapes_agree() -> None:
    model = load_joint_model()
    size = len(model.metrics)
    assert len(model.center) == size
    assert len(model.scale) == size
    assert len(model.precision) == size
    assert all(len(row) == size for row in model.precision)
    assert all(s > 0 for s in model.scale), "a zero scale would divide by zero on every swing"


def test_the_population_was_big_enough_to_fit_on() -> None:
    model = load_joint_model()
    assert model.n >= 400
    assert model.n_players >= 100


def test_distance_quantiles_are_ordered() -> None:
    q = load_joint_model().distance_quantiles
    assert q["p10"] < q["p25"] < q["p50"] < q["p75"] < q["p90"]


def test_a_swing_at_the_tour_center_is_the_least_unusual_thing_there_is() -> None:
    placement = placement_for(_center_reading())
    assert placement is not None
    assert placement.distance == 0.0
    assert placement.percentile == 10.0
    assert placement.percentile_clamped is True


def test_a_missing_metric_refuses_rather_than_guessing() -> None:
    """No number beats a wrong one (ADR-010 §2).

    The realistic case is tempo, which address detection declines to produce on some clips. A
    distance computed over five of six metrics would be a different quantity wearing the same
    name.
    """
    reading = _center_reading()
    del reading["tempo_ratio"]
    assert placement_for(reading) is None
    assert placement_for({}) is None


def test_contributions_sum_to_one_and_name_the_metric_responsible() -> None:
    model = load_joint_model()
    reading = _center_reading()
    # Push one metric four robust standard deviations out, leaving the rest at center.
    index = model.metrics.index("head_sway_norm")
    reading["head_sway_norm"] = model.center[index] + 4 * model.scale[index]

    placement = placement_for(reading)
    assert placement is not None
    assert math.isclose(sum(placement.contributions.values()), 1.0, abs_tol=1e-3)
    assert next(iter(placement.contributions)) == "head_sway_norm"


def test_the_quadratic_form_is_never_negative() -> None:
    """The precision matrix must stay positive definite, or a distance could come out imaginary.

    Cheap to check and worth checking: an ill-conditioned re-derive is exactly the kind of thing
    that produces a plausible-looking artifact which fails only on unusual swings.
    """
    model = load_joint_model()
    for scale in (-4.0, -1.5, 0.5, 3.0):
        reading = {
            metric: model.center[i] + scale * model.scale[i]
            for i, metric in enumerate(model.metrics)
        }
        placement = placement_for(reading)
        assert placement is not None
        assert placement.distance >= 0.0


def test_a_wilder_swing_is_never_placed_closer_than_a_tamer_one() -> None:
    """Monotone along a ray from the center — the property that makes the number readable."""
    model = load_joint_model()
    previous = -1.0
    for scale in (0.0, 0.5, 1.0, 2.0, 3.0):
        reading = {
            metric: model.center[i] + scale * model.scale[i]
            for i, metric in enumerate(model.metrics)
        }
        placement = placement_for(reading)
        assert placement is not None
        assert placement.distance >= previous
        previous = placement.distance


def test_the_percentile_clamps_instead_of_extrapolating() -> None:
    """Past p90 the honest report is "at least this unusual", not an invented number."""
    model = load_joint_model()
    absurd = {
        metric: model.center[i] + 50 * model.scale[i]
        for i, metric in enumerate(model.metrics)
    }
    placement = placement_for(absurd)
    assert placement is not None
    assert placement.percentile == 90.0
    assert placement.percentile_clamped is True


def test_placement_is_not_reachable_from_the_scoring_path() -> None:
    """The firewall, asserted the only way it can be before the model is surfaced.

    `mechanics.py` is what turns a measurement into a verdict. If it ever imports this module, a
    population placement has become an input to a score — which is exactly what ADR-010 §2 forbids
    and what `test_population.py` pins for percentiles. Written now, while the answer is trivially
    true, because the moment it stops being true is the moment nobody is looking.
    """
    import golf_coach.analysis.checkpoints.mechanics as mechanics
    import golf_coach.analysis.scoring as scoring

    for module in (mechanics, scoring):
        source = module.__doc__ or ""
        assert "joint" not in {
            name for name in dir(module) if not name.startswith("_")
        }, f"{module.__name__} has grown a reference to the joint model"
        assert "benchmarks.joint" not in source
