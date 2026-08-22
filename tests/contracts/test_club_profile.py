"""The club-profile shape: ordered like a bag, and unable to conflate two different questions.

Every pin here guards a failure that produces a plausible-looking answer rather than an error:

- a nested `MetricBaseline` or `MetricDispersion` that does not survive a round-trip loses the
  refusals inside it, and a refusal that goes missing renders as a blank cell — which reads as
  zero, the one thing `contracts/baseline.py` exists to prevent;
- `category` disagreeing with `category_of` puts a club in two families at once, and the one that
  keys benchmark rows is not necessarily the one the bag page prints;
- `clubs_used` and `clubs_declared` collapsing into each other loses which of "go hit it" and
  "this is history, not your current bag" a row is asking for;
- `n_swings` and `n_shots` collapsing overstates what a carry statistic can be built from — a club
  filmed six times with two screen photos has a distance ceiling of two, and nobody audits a carry;
- `clubs` drifting to alphabetical order puts `3w` between `2h` and `5h`, which a golfer cannot
  tell apart from a bug in their bag;
- a club appearing twice means whichever consumer looks it up first silently drops the other, and
  `profile_for` is one such consumer.

Assertions derive expected order and coverage from `ClubId` rather than re-typing them, so a club
added later is covered on the day it is added (R6).
"""

from __future__ import annotations

from datetime import UTC, datetime

import pytest
from pydantic import ValidationError

from golf_coach.contracts.bag import BagEntry
from golf_coach.contracts.baseline import (
    BaselineClaim,
    Interval,
    MetricBaseline,
    WithheldClaim,
)
from golf_coach.contracts.club import ClubId, category_of
from golf_coach.contracts.club_profile import BagProfile, ClubProfile
from golf_coach.contracts.dispersion import Finding, MetricDispersion

_WHEN = datetime(2026, 8, 22, 12, 0, tzinfo=UTC)


def _profile(club: ClubId, **overrides: object) -> ClubProfile:
    return ClubProfile(club=club, **overrides)


def test_a_populated_profile_round_trips_through_json() -> None:
    """The composition is the phase, so the nesting is what has to survive storage.

    A baseline carrying a claim *and* a refusal, and a dispersion carrying a finding, because those
    are the two halves P17/P18/P19 render and the halves a hand-rolled serializer would flatten.
    """
    baseline = MetricBaseline(
        name="carry_distance_yds",
        unit="yards",
        source="launch_monitor:hd_golf",
        n=6,
        n_sessions=2,
        mean=164.2,
        mean_ci=Interval(low=158.0, high=170.4),
        withheld=[
            WithheldClaim(
                claim=BaselineClaim.SPREAD,
                have_n=6,
                need_n=10,
                have_sessions=2,
                reason="6 shots on this club; a spread claim needs 10",
            )
        ],
        ready=[BaselineClaim.CENTER],
    )
    dispersion = MetricDispersion(
        name="carry_distance_yds",
        unit="yards",
        source="launch_monitor:hd_golf",
        n=6,
        n_sessions=2,
        tolerance=5.0,
        scatter=Finding.NOT_ESTABLISHED,
        unavailable=["no declared target, so no bias finding"],
    )
    profile = ClubProfile(
        club=ClubId.SEVEN_IRON,
        bag_entry=BagEntry(club=ClubId.SEVEN_IRON, loft_deg=34.0, recorded_at=_WHEN),
        in_bag=True,
        n_swings=6,
        n_shots=6,
        n_sessions=2,
        metrics={"carry_distance_yds": baseline},
        dispersion={"carry_distance_yds": dispersion},
        caveats=["the bag entry for this club was recorded after some of these shots"],
    )
    bag = BagProfile(player_id="aaron", clubs=(profile,), untagged_swings=2)

    restored = BagProfile.model_validate_json(bag.model_dump_json())

    assert restored == bag
    club = restored.clubs[0]
    assert club.metrics["carry_distance_yds"].mean == 164.2
    assert club.metrics["carry_distance_yds"].withheld[0].need_n == 10, "the refusal survived"
    assert club.dispersion["carry_distance_yds"].unavailable == [
        "no declared target, so no bias finding"
    ]
    assert club.bag_entry is not None and club.bag_entry.loft_deg == 34.0
    assert restored.untagged_swings == 2


def test_category_is_derived_for_every_club() -> None:
    """One mapping, walked rather than re-listed — a stored copy is a second thing to disagree."""
    for club in ClubId:
        assert _profile(club).category is category_of(club)


def test_declared_and_hit_are_separate_lists_in_all_four_combinations() -> None:
    """The distinction ADR-024 §2 turns on, asserted from both lists so a club cannot hide.

    A club in the bag you have not hit has no statistics and is not an error; a club you have hit
    that has left the bag still has real history. Reading only one list would pass with the two
    collapsed.
    """
    declared_and_hit = _profile(ClubId.DRIVER, in_bag=True, n_swings=8)
    declared_unhit = _profile(ClubId.THREE_WOOD, in_bag=True, n_swings=0)
    retired_but_hit = _profile(ClubId.SEVEN_IRON, in_bag=False, n_swings=5)
    neither = _profile(ClubId.LOB_WEDGE, in_bag=False, n_swings=0)

    bag = BagProfile(
        player_id="aaron",
        clubs=(declared_and_hit, declared_unhit, retired_but_hit, neither),
    )

    assert [p.club for p in bag.clubs_used] == [ClubId.DRIVER, ClubId.SEVEN_IRON]
    assert [p.club for p in bag.clubs_declared] == [ClubId.DRIVER, ClubId.THREE_WOOD]
    assert bag.clubs_used != bag.clubs_declared, "the two lists collapsed into one question"


def test_swings_and_shots_are_independent_counts() -> None:
    """The carry ceiling. Six clips with two screen photos is two distance samples, not six."""
    profile = _profile(ClubId.SEVEN_IRON, n_swings=6, n_shots=2, n_sessions=2)

    restored = ClubProfile.model_validate_json(profile.model_dump_json())

    assert (restored.n_swings, restored.n_shots) == (6, 2), "one counter is standing in for two"
    assert restored.n_swings > restored.n_shots


def test_clubs_must_be_in_bag_order() -> None:
    """The ordering pin, with a set where insertion, alphabetical and bag order all differ.

    Inserted pw, 3w, driver, 7i; alphabetically that is 3w, 7i, driver, pw. Only bag order puts the
    driver first and the wedge last, which is the order a golfer looks down at.
    """
    inserted = [ClubId.PITCHING_WEDGE, ClubId.THREE_WOOD, ClubId.DRIVER, ClubId.SEVEN_IRON]
    with pytest.raises(ValidationError, match="not in bag order"):
        BagProfile(player_id="aaron", clubs=tuple(_profile(c) for c in inserted))

    alphabetical = sorted(inserted, key=lambda club: club.value)
    with pytest.raises(ValidationError, match="not in bag order"):
        BagProfile(player_id="aaron", clubs=tuple(_profile(c) for c in alphabetical))

    ordered = tuple(club for club in ClubId if club in set(inserted))
    bag = BagProfile(player_id="aaron", clubs=tuple(_profile(c) for c in ordered))
    assert [p.club for p in bag.clubs] == [
        ClubId.DRIVER,
        ClubId.THREE_WOOD,
        ClubId.SEVEN_IRON,
        ClubId.PITCHING_WEDGE,
    ]


def test_a_full_bag_in_declaration_order_is_accepted() -> None:
    """The order is the declaration's, so the whole taxonomy must pass unmodified (R6)."""
    bag = BagProfile(player_id="aaron", clubs=tuple(_profile(club) for club in ClubId))
    assert [p.club for p in bag.clubs] == list(ClubId)


def test_a_club_may_not_appear_twice() -> None:
    """Two profiles for one club means one of them is silently dropped by every lookup."""
    with pytest.raises(ValidationError, match="appears twice"):
        BagProfile(
            player_id="aaron",
            clubs=(_profile(ClubId.SEVEN_IRON, n_swings=5), _profile(ClubId.SEVEN_IRON)),
        )


def test_profile_for_finds_a_club_or_returns_none() -> None:
    """None is a real answer here — neither hit nor declared — and not an error to recover from."""
    bag = BagProfile(
        player_id="aaron",
        clubs=(_profile(ClubId.DRIVER, n_swings=3), _profile(ClubId.SEVEN_IRON, n_swings=5)),
    )

    seven = bag.profile_for(ClubId.SEVEN_IRON)
    assert seven is not None and seven.n_swings == 5
    assert bag.profile_for(ClubId.SAND_WEDGE) is None


def test_an_empty_bag_profile_is_empty_rather_than_an_error() -> None:
    """The state on disk today: every stored swing predates the tag, so nothing profiles."""
    bag = BagProfile(player_id="aaron", untagged_swings=2)

    assert bag.clubs == ()
    assert bag.clubs_used == ()
    assert bag.clubs_declared == ()
    assert bag.profile_for(ClubId.SEVEN_IRON) is None
    assert bag.untagged_swings == 2, "the swings that exist are still counted somewhere"
