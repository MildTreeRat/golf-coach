"""The population-placement registry, and the two things that would make it a lie.

`contracts/placements.py` exists so the caveat prose can be derived rather than typed — that half
is pinned in `tests/test_docs_truth.py`. What is pinned here is the registry's own consistency, and
the one fact it shares with a module outside `contracts`: the view strings that key the fitted
artifacts.

The other half — that the registry describes the placements the engine **actually emits** — needs
the synthetic-swing fixture and so lives in `tests/analysis/test_engine.py`. That is the failure
worth naming: a registry naming four of five produces caveats that are internally consistent and
wrong about the data, and every derived assertion here still passes.
"""

from __future__ import annotations

import pytest

from golf_coach.contracts.placements import (
    DOWN_THE_LINE,
    FACE_ON,
    PLACEMENTS_BY_NAME,
    POPULATION_PLACEMENT_REGISTRY,
    placement_names,
    spec_for,
)


def test_the_registry_has_no_duplicate_names() -> None:
    """`PLACEMENTS_BY_NAME` is built by comprehension, so a duplicate would silently drop one."""
    names = placement_names()
    assert len(names) == len(set(names)) == len(PLACEMENTS_BY_NAME)


def test_every_placement_belongs_to_a_fitted_view() -> None:
    """A view string with no artifact behind it is a caveat about a model that does not exist."""
    for spec in POPULATION_PLACEMENT_REGISTRY:
        assert spec.view in (FACE_ON, DOWN_THE_LINE), f"{spec.name} claims view {spec.view!r}"


def test_the_view_strings_match_the_artifact_keys() -> None:
    """The reason the constants moved into `contracts`: one spelling, two importers.

    `benchmarks/trajectory.py` keys its two committed artifacts by these exact strings. A second
    definition that drifted would send a down-the-line swing to the face-on basis — scored against
    the wrong population, with no error anywhere.
    """
    from golf_coach.analysis.benchmarks import trajectory

    assert trajectory.FACE_ON is FACE_ON
    assert trajectory.DOWN_THE_LINE is DOWN_THE_LINE
    assert set(trajectory._MODEL_FILES) == {FACE_ON, DOWN_THE_LINE}


def test_spec_for_raises_on_an_unregistered_name() -> None:
    with pytest.raises(KeyError):
        spec_for("tour_nothing_at_all")


def test_at_least_one_placement_is_uncalibrated_and_one_is_not() -> None:
    """Both flags in use, or the caveat bullet that splits them is untested prose.

    Not an arbitrary shape assertion: `test_docs_truth` derives the uncalibrated bullet by
    filtering on this field, and a registry where every entry agreed would let that bullet name
    nothing — or everything — while still passing.
    """
    calibration = {spec.calibrated for spec in POPULATION_PLACEMENT_REGISTRY}
    assert calibration == {True, False}
