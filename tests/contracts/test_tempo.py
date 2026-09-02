"""The tempo trainer's shapes, and the two invariants a beat sequence must satisfy.

These are about `contracts/tempo.py` alone — that a `BeatPattern` cannot be built saying one thing
in its beats and another in its durations. Whether the *numbers* are the tour's is
`tests/analysis/test_tempo_trainer.py`'s question.
"""

from __future__ import annotations

import pytest
from pydantic import ValidationError

from golf_coach.contracts.tempo import (
    SWING_INSTANTS,
    Beat,
    BeatPattern,
    BeatRole,
    TempoPattern,
    TempoPlan,
)


def _pattern(**overrides) -> BeatPattern:
    fields = {
        "mode": TempoPattern.CUES,
        "beats": (
            Beat(at_ms=0.0, role=BeatRole.TAKEAWAY),
            Beat(at_ms=900.0, role=BeatRole.TOP),
            Beat(at_ms=1167.0, role=BeatRole.IMPACT),
        ),
        "backswing_ms": 900.0,
        "downswing_ms": 267.0,
        "ratio": 900.0 / 267.0,
        "source": "seeded",
    }
    return BeatPattern(**{**fields, **overrides})


def _plan(**overrides) -> TempoPlan:
    """A plan with the fields every one must carry. Neither anchor has a default on purpose — a
    plan that cannot say what it was fitted to is one a reader cannot tell apart from the
    tour-median case. The two are the tour medians here, so this fixture is that case."""
    fields = {
        "patterns": (_pattern(),),
        "anchor_downswing_ms": 267.0,
        "anchor_backswing_ms": 900.0,
    }
    return TempoPlan(**{**fields, **overrides})


def test_a_pattern_carries_its_own_durations_not_the_plans() -> None:
    """The reason durations live on the pattern: two patterns, two different answers.

    `GRID` snaps the ratio to an integer and `CUES` does not, so a single `ratio` on `TempoPlan`
    would be false for one of them — quietly, since both are plausible numbers.
    """
    grid = _pattern(mode=TempoPattern.GRID, backswing_ms=801.0, downswing_ms=267.0, ratio=3.0)
    cues = _pattern()

    plan = _plan(patterns=(grid, cues))

    assert plan.patterns[0].ratio != plan.patterns[1].ratio
    assert plan.default_pattern is grid


def test_every_pattern_puts_its_beats_in_order_and_ends_at_impact() -> None:
    for pattern in (
        _pattern(),
        _pattern(
            mode=TempoPattern.GRID,
            beats=(
                Beat(at_ms=0.0, role=BeatRole.TAKEAWAY),
                Beat(at_ms=267.0, role=BeatRole.SUBDIVISION),
                Beat(at_ms=534.0, role=BeatRole.SUBDIVISION),
                Beat(at_ms=801.0, role=BeatRole.TOP),
                Beat(at_ms=1068.0, role=BeatRole.IMPACT),
            ),
            backswing_ms=801.0,
            downswing_ms=267.0,
            ratio=3.0,
        ),
    ):
        times = [b.at_ms for b in pattern.beats]
        assert times == sorted(times)
        assert len(set(times)) == len(times), "two beats at the same instant is one click"
        assert pattern.beats[0].role is BeatRole.TAKEAWAY
        assert pattern.beats[0].at_ms == 0.0
        assert pattern.beats[-1].role is BeatRole.IMPACT


def test_the_top_and_impact_beats_agree_with_the_durations() -> None:
    """The invariant that makes the beats and the numbers one statement rather than two.

    A pattern whose `TOP` beat did not sit at `backswing_ms` would print one tempo and play
    another, and the golfer would practice to the one they can hear.
    """
    for pattern in (_pattern(), _pattern(mode=TempoPattern.GRID)):
        top = next(b for b in pattern.beats if b.role is BeatRole.TOP)
        impact = next(b for b in pattern.beats if b.role is BeatRole.IMPACT)

        assert top.at_ms == pytest.approx(pattern.backswing_ms)
        assert impact.at_ms == pytest.approx(pattern.backswing_ms + pattern.downswing_ms)


def test_the_three_swing_instants_are_the_ones_both_patterns_share() -> None:
    """`SWING_INSTANTS` is what a consumer may rely on being present in either mode."""
    assert SWING_INSTANTS == {BeatRole.TAKEAWAY, BeatRole.TOP, BeatRole.IMPACT}
    assert BeatRole.SUBDIVISION not in SWING_INSTANTS

    for pattern in (_pattern(), _pattern(mode=TempoPattern.GRID)):
        present = {b.role for b in pattern.beats}
        assert SWING_INSTANTS <= present


def test_a_pace_of_zero_is_refused() -> None:
    """Every beat time is multiplied by it, so zero collapses the sequence onto one instant."""
    with pytest.raises(ValidationError):
        _plan(pace=0.0)


def test_the_observed_durations_are_optional_and_are_not_zero_by_default() -> None:
    """A swing with no timeable tempo has no observed side, and that is not the same as 0 ms."""
    plan = _plan()

    assert plan.observed_backswing_ms is None
    assert plan.observed_downswing_ms is None


def test_a_plan_must_say_which_swing_it_was_built_from() -> None:
    """The two fields that separate "the tour median" from "yours", which the beats cannot show.

    Two golfers 167 ms apart on the backswing — the gap between the LPGA and PGA cohorts — get
    different targets from the same code, and a reader holding only the beats cannot tell whether
    the one in front of them was fitted or defaulted.

    Both anchors are required rather than one: `anchor_downswing_ms` is the half the fit is taken
    from and `anchor_backswing_ms` is the half it prescribes (ADR-023's 2026-09-02 addendum), and
    a plan carrying only one of them makes every surface do the tour-ratio arithmetic itself.
    """
    with pytest.raises(ValidationError):
        TempoPlan(patterns=(_pattern(),))
    with pytest.raises(ValidationError):
        TempoPlan(patterns=(_pattern(),), anchor_backswing_ms=900.0)
    with pytest.raises(ValidationError):
        TempoPlan(patterns=(_pattern(),), anchor_downswing_ms=267.0)

    assert _plan().anchored is False, "defaulting to the median must not claim to be anchored"
    fitted = _plan(anchored=True, anchor_downswing_ms=383.9, anchor_backswing_ms=1296.0)
    assert fitted.anchor_downswing_ms == 383.9
    assert fitted.anchor_backswing_ms == 1296.0


def test_a_plan_with_nothing_to_notice_says_nothing() -> None:
    """The defaults are the quiet case, and they have to be: the notice and the snap are drawn off
    these two fields alone, so a plan that had to remember to opt out of them would eventually
    tell a golfer their downswing is outside a range nobody measured it against."""
    plan = _plan()

    assert plan.downswing_in_tour_range is True
    assert plan.in_range_pace is None


def test_a_downswing_inside_the_range_cannot_carry_a_snap_offer() -> None:
    """The contradiction the model refuses, because two controls read these fields separately.

    The notice reads the bool and the snap button reads the None-ness. A plan asserting both would
    draw a button that moves the golfer off their own measured downswing with nothing above it
    saying why — and each surface, holding only its own half, would render exactly as told.
    """
    with pytest.raises(ValidationError):
        _plan(in_range_pace=1.13)

    outside = _plan(downswing_in_tour_range=False, in_range_pace=1.13)
    assert outside.in_range_pace == 1.13

    silent = _plan(downswing_in_tour_range=False)
    assert silent.in_range_pace is None, (
        "out of range with no edge to offer is a coherent state and stays legal"
    )


def test_a_snap_pace_of_zero_is_refused() -> None:
    """`in_range_pace` is played through the same multiplier as `pace`, so it is bounded the same
    way — zero collapses every beat onto one instant, and the offer is the golfer's to take."""
    with pytest.raises(ValidationError):
        _plan(downswing_in_tour_range=False, in_range_pace=0.0)
