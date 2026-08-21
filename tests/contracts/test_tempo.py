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
    """A plan with the fields every one must carry. `anchor_backswing_ms` has no default on
    purpose — a plan that cannot say which backswing it was built from is one a reader cannot
    tell apart from the tour-median case."""
    fields = {"patterns": (_pattern(),), "anchor_backswing_ms": 900.0}
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


def test_a_plan_must_say_which_backswing_it_was_built_from() -> None:
    """The field that separates "the tour median" from "yours", which the beats cannot show.

    Two golfers 167 ms apart on the backswing — the gap between the LPGA and PGA cohorts — get
    different targets from the same code, and a reader holding only the beats cannot tell whether
    the one in front of them was fitted or defaulted.
    """
    with pytest.raises(ValidationError):
        TempoPlan(patterns=(_pattern(),))

    assert _plan().anchored is False, "defaulting to the median must not claim to be anchored"
    assert _plan(anchored=True, anchor_backswing_ms=1001.0).anchor_backswing_ms == 1001.0
