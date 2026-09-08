"""The mishit rule: gross tops only, and never a shot in the low half of normal dispersion.

Every pin here guards a failure that would look like a working answer:

- a floor that fires below `MISHIT_MIN_CLEAN_SHOTS` samples deletes a shot from a club that has no
  established carry to call it an outlier of;
- a floor computed from the mean rather than the median moves with the very tops it is meant to
  find, so a bag with two bad shots stops flagging the third;
- `MISHIT_EXCLUDED_METRICS` growing a third entry silently widens what a mishit is withheld from,
  and the ADR says it is carry and total only.
"""

from __future__ import annotations

from golf_coach.contracts.mishit import (
    MISHIT_CARRY_FRACTION,
    MISHIT_EXCLUDED_METRICS,
    MISHIT_MIN_CLEAN_SHOTS,
    MishitVerdict,
    mishit_carry_floor,
)


def test_too_few_samples_is_no_floor_not_a_low_one() -> None:
    """Below the sample floor there is no club history, so nothing can be an outlier of it."""
    carries = [155.0, 20.0, 160.0, 158.0]  # an obvious top among them, and only four shots
    assert len(carries) < MISHIT_MIN_CLEAN_SHOTS
    assert mishit_carry_floor(carries) is None


def test_the_floor_is_half_the_median() -> None:
    carries = [155.0, 160.0, 158.0, 162.0, 159.0]  # median 159
    assert mishit_carry_floor(carries) == MISHIT_CARRY_FRACTION * 159.0


def test_a_gross_top_is_below_the_floor_and_a_low_normal_shot_is_not() -> None:
    """The line the whole milestone is about: a 20-yard 7 iron out, a 140-yard one in."""
    established = [155.0, 160.0, 158.0, 162.0, 159.0]
    floor = mishit_carry_floor(established)
    assert floor is not None
    assert 20.0 < floor  # a top: flagged
    assert 140.0 > floor  # the low edge of this golfer's own dispersion: kept


def test_the_median_does_not_chase_the_tops_it_is_looking_for() -> None:
    """One top already in the sample must not lower the floor past a second real shot.

    A mean over [150, 152, 148, 151, 20] is 124; half of that is 62, which would wave a genuine
    130-yard shot through. The median is 150, the floor 75, and both the 130 and the 20 are
    judged correctly.
    """
    with_one_top = [150.0, 152.0, 148.0, 151.0, 20.0]
    floor = mishit_carry_floor(with_one_top)
    assert floor is not None
    assert floor == MISHIT_CARRY_FRACTION * 150.0
    assert 20.0 < floor  # the top is still caught
    assert 130.0 > floor  # a real shot is not swept in with it


def test_excluded_metrics_are_carry_and_total_only() -> None:
    assert MISHIT_EXCLUDED_METRICS == frozenset({"carry_distance_yds", "total_distance_yds"})


def test_verdict_values_round_trip() -> None:
    assert MishitVerdict("confirmed") is MishitVerdict.CONFIRMED
    assert MishitVerdict("cleared") is MishitVerdict.CLEARED
    assert MishitVerdict.CONFIRMED.value == "confirmed"
