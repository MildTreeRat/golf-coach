"""The pivot registry, the index space it pins, and the ways either could quietly become a lie.

`tests/contracts/test_placements.py`'s counterpart. Same two halves: the registry's own internal
consistency, and the facts it shares with modules outside `contracts` — the view spellings, and the
one that is easy to miss, that `PIVOT_SAMPLES` being odd is *only* meaningful against
`analysis/trajectory.sample_positions`' arithmetic. An even count would still pass every assertion
about the two spans below while putting the top of the backswing between two samples.

The other half — that the engine actually emits these ten rows — needs the synthetic swing and
lives in `tests/analysis/test_engine.py` (the face-on five) and `tests/analysis/test_engine_bundle
.py` (the `_dtl` five, and that the two never become one name). Without those, a registry naming a
metric no check computes is internally consistent and wrong about the data, exactly as
`test_placements.py` warns.
"""

from __future__ import annotations

import pytest

from golf_coach.contracts.pivots import (
    BACKSWING_SPAN,
    DOWNSWING_SPAN,
    PIVOT_MEASUREMENT_REGISTRY,
    PIVOT_SAMPLES,
    PIVOTS_BY_NAME,
    FrameOfReference,
    PivotObservation,
    pivot_measurement_names,
    spec_for,
)
from golf_coach.contracts.placements import DOWN_THE_LINE, FACE_ON

_DTL_SUFFIX = "_dtl"


def _unsuffixed(name: str) -> str:
    """The face-on spelling of a name, whichever view it came from."""
    return name[: -len(_DTL_SUFFIX)] if name.endswith(_DTL_SUFFIX) else name


def test_the_registry_has_no_duplicate_names() -> None:
    """`PIVOTS_BY_NAME` is built by comprehension, so a duplicate would silently drop one."""
    names = pivot_measurement_names()
    assert len(names) == len(set(names)) == len(PIVOTS_BY_NAME)


def test_every_pivot_belongs_to_one_of_the_two_views() -> None:
    """A third view string is a measurement no camera in this system produced."""
    for spec in PIVOT_MEASUREMENT_REGISTRY:
        assert spec.view in (FACE_ON, DOWN_THE_LINE), f"{spec.name} claims view {spec.view!r}"


def test_every_name_carries_the_pivot_prefix() -> None:
    """The prefix is what makes a row readable in a measurement list without a lookup.

    The registry is append-only, so a name that breaks the convention cannot be renamed later —
    it is on disk in every `analysis.json` written after it shipped.
    """
    for spec in PIVOT_MEASUREMENT_REGISTRY:
        assert spec.name.startswith("pivot_"), spec.name


def test_the_views_come_in_matched_pairs_sharing_one_check() -> None:
    """The seam, asserted: the view decides the name, the check does not know which camera it read.

    A `_dtl` spec naming its own check would be a second implementation of the same rule — and the
    two would drift, which is the whole reason `check` is a field rather than the name with the
    suffix cut off.
    """
    face_on = {spec.name: spec for spec in PIVOT_MEASUREMENT_REGISTRY if spec.view == FACE_ON}
    dtl = {spec.name: spec for spec in PIVOT_MEASUREMENT_REGISTRY if spec.view == DOWN_THE_LINE}

    assert set(dtl) == {f"{name}{_DTL_SUFFIX}" for name in face_on}
    for name, spec in dtl.items():
        partner = face_on[_unsuffixed(name)]
        assert spec.check == partner.check, f"{name} and {partner.name} name different checks"
        assert spec.unit == partner.unit


def test_the_name_suffix_and_the_unit_agree() -> None:
    """`_norm` iff shoulder widths, `_deg` iff degrees — read off the name, so it must be true.

    A `_norm` row emitted in degrees is a number a coaching model will compare against the wrong
    thing, and nothing downstream re-checks the unit against the name.
    """
    units = {"_norm": "shoulder_widths", "_deg": "degrees"}
    for spec in PIVOT_MEASUREMENT_REGISTRY:
        stem = _unsuffixed(spec.name)
        matching = [suffix for suffix in units if stem.endswith(suffix)]
        assert len(matching) == 1, f"{spec.name} carries no unit suffix"
        assert spec.unit == units[matching[0]], f"{spec.name} is a {spec.unit}"


def test_every_spec_says_why_it_is_interim() -> None:
    """P6 derives the caveat from this field; an empty one is a number that ships unqualified."""
    for spec in PIVOT_MEASUREMENT_REGISTRY:
        assert spec.interim_reason.strip(), spec.name
        assert spec.detail.strip(), spec.name


def test_spec_for_raises_on_an_unregistered_name() -> None:
    with pytest.raises(KeyError):
        spec_for("pivot_nothing_at_all")


def test_the_sample_count_is_odd() -> None:
    """The middle sample has to exist for the two spans to meet on the top of the backswing."""
    assert PIVOT_SAMPLES % 2 == 1


def test_the_two_spans_cover_the_swing_and_meet_only_at_the_top() -> None:
    """Every sample belongs to exactly one half, except the top, which belongs to both.

    A gap loses samples no check ever reads; a wider overlap counts a reversal in both halves,
    which is precisely what splitting the metric into two names was for.
    """
    backswing, downswing = set(BACKSWING_SPAN), set(DOWNSWING_SPAN)
    assert backswing | downswing == set(range(PIVOT_SAMPLES))
    assert backswing & downswing == {PIVOT_SAMPLES // 2}


def test_the_middle_sample_lands_on_the_top_anchor() -> None:
    """Why odd, from the other side — and the only assertion here that can actually fail on it.

    `sample_positions` walks event time as `t = span·i/(steps−1)`, so the middle anchor falls on an
    integer sample only when `steps` is odd. Every other assertion in this file passes with an even
    count while the top of the backswing sits between two samples and `BACKSWING_SPAN`'s last index
    is not the top at all.
    """
    from golf_coach.analysis.trajectory import sample_positions

    top_frame = 20.0
    positions = sample_positions((0.0, top_frame, 40.0), PIVOT_SAMPLES)

    assert len(positions) == PIVOT_SAMPLES
    assert positions[BACKSWING_SPAN[-1]] == pytest.approx(top_frame)
    assert positions[DOWNSWING_SPAN[0]] == pytest.approx(top_frame)


def test_an_observation_carries_no_club_head_unless_one_is_given() -> None:
    """The slot exists for a detector that does not: M17 populates nothing (ADR-017 was a no-go)."""
    observation = PivotObservation(
        shoulder=(0.5, 0.3),
        hip=(0.5, 0.6),
        hands=(0.5, 0.7),
        shoulder_line=(1.0, 0.0),
        hip_line=None,
        frame_of_reference=FrameOfReference.IMAGE_PLANE_FACE_ON,
    )
    assert observation.club_head is None
    assert observation.hip_line is None, "a collapsed line is None, never an angle"
