"""The bag shape: ordered like a bag, and unable to file a club under the wrong slot.

Every pin here guards a failure that produces a plausible-looking number rather than an error:

- an entry filed under the wrong slot attaches one club's loft to another club's history, and
  loft is the anchor every future fitting answer is computed from;
- `club_ids` drifting to insertion or alphabetical order reorders every bag view, and a golfer
  reading `3w` between `2h` and `5h` has no way to tell that from a bug in their bag;
- a `player_id` that is not a slug becomes a filename in `BagStore`, which reads it into a path
  with no second sanitising step precisely because this check exists;
- a loft that does not survive a round-trip reads as unmeasured, and unmeasured refuses — so the
  failure is a silently withheld answer rather than a wrong one;
- `same_club_as` returning true for two different clubs pools two populations of shots under one
  name, and returning false for one club re-saved unchanged hands P16 a bag-changed caveat over a
  club that never changed — the store's whole upsert decision is this one comparison;
- an entry in the wrong list is a club the golfer no longer owns presented as the one in their
  hands, or a club still in the bag that has dropped out of `club_ids` with nothing recording why.

Assertions derive expected order from `ClubId` rather than re-typing it, so a club added later is
covered on the day it is added.
"""

from __future__ import annotations

from datetime import UTC, datetime

import pytest
from pydantic import ValidationError

from golf_coach.contracts.bag import Bag, BagEntry
from golf_coach.contracts.club import ClubId

_WHEN = datetime(2026, 8, 21, 12, 0, tzinfo=UTC)
_LATER = datetime(2026, 8, 21, 13, 0, tzinfo=UTC)


def _entry(club: ClubId, **overrides: object) -> BagEntry:
    return BagEntry(club=club, recorded_at=_WHEN, **overrides)


def test_bag_round_trips_through_json() -> None:
    """Enum keys, an absent loft and a present one all survive storage.

    `BagStore` writes this with `model_dump_json` and reads it back with `model_validate_json`, so
    anything that does not round-trip here is a field that quietly resets every time the bag is
    saved.
    """
    bag = Bag(
        player_id="aaron",
        entries={
            ClubId.SEVEN_IRON: _entry(ClubId.SEVEN_IRON, loft_deg=34.0, make="Titleist"),
            ClubId.SAND_WEDGE: _entry(ClubId.SAND_WEDGE, shaft="Dynamic Gold", length_in=35.25),
        },
        updated_at=_WHEN,
    )

    restored = Bag.model_validate_json(bag.model_dump_json())

    assert restored == bag
    assert restored.entries[ClubId.SEVEN_IRON].loft_deg == 34.0
    assert restored.entries[ClubId.SAND_WEDGE].loft_deg is None, "unmeasured must stay unmeasured"


def test_club_ids_is_bag_order_and_not_insertion_or_alphabetical() -> None:
    """The ordering pin, with a set of clubs where all three orders differ.

    Inserted pw, 3w, driver, 7i; alphabetically that is 3w, 7i, driver, pw. Only bag order puts
    the driver first and the wedge last, which is the order a golfer looks down at.
    """
    inserted = [ClubId.PITCHING_WEDGE, ClubId.THREE_WOOD, ClubId.DRIVER, ClubId.SEVEN_IRON]
    bag = Bag(
        player_id="aaron",
        entries={club: _entry(club) for club in inserted},
        updated_at=_WHEN,
    )

    assert bag.club_ids == (
        ClubId.DRIVER,
        ClubId.THREE_WOOD,
        ClubId.SEVEN_IRON,
        ClubId.PITCHING_WEDGE,
    )
    assert list(bag.club_ids) != inserted, "club_ids fell back to insertion order"
    assert [c.value for c in bag.club_ids] != sorted(c.value for c in inserted), "sorted, not bag"


def test_club_ids_covers_exactly_the_declared_clubs() -> None:
    """Derived from `ClubId`, so it must still be a view of the entries and not of the taxonomy."""
    every = Bag(
        player_id="aaron",
        entries={club: _entry(club) for club in ClubId},
        updated_at=_WHEN,
    )
    assert every.club_ids == tuple(ClubId), "a full bag is the declaration order, unfiltered"

    partial = Bag(
        player_id="aaron",
        entries={ClubId.FIVE_HYBRID: _entry(ClubId.FIVE_HYBRID)},
        updated_at=_WHEN,
    )
    assert partial.club_ids == (ClubId.FIVE_HYBRID,)
    assert set(partial.club_ids) == set(partial.entries)


def test_an_empty_bag_is_empty_rather_than_an_error() -> None:
    """A golfer who has declared nothing still has a bag — P15 profiles clubs they have hit."""
    assert Bag(player_id="aaron", updated_at=_WHEN).club_ids == ()


def test_an_entry_filed_under_the_wrong_slot_is_rejected() -> None:
    """The mismatch guard. A sand wedge under `7i` is a wrong loft nothing downstream can detect."""
    with pytest.raises(ValidationError, match="declares itself"):
        Bag(
            player_id="aaron",
            entries={ClubId.SEVEN_IRON: _entry(ClubId.SAND_WEDGE)},
            updated_at=_WHEN,
        )


def test_player_id_must_be_a_slug() -> None:
    """`BagStore` reads this straight into a filename, trusting the contract to have checked."""
    for bad in ("Aaron Sierra", "", "../aaron", "aaron_sierra"):
        with pytest.raises(ValidationError, match="is not a slug"):
            Bag(player_id=bad, updated_at=_WHEN)

    assert Bag(player_id="aaron-sierra", updated_at=_WHEN).player_id == "aaron-sierra"


# --------------------------------------------------------------------------- the shelf


def _retired(club: ClubId, when: datetime, **overrides: object) -> BagEntry:
    return BagEntry(club=club, recorded_at=_WHEN, retired_at=when, **overrides)


def test_a_bag_with_a_shelf_round_trips_through_json() -> None:
    """Retention is only real if it survives the write. The shelf is what a delete would lose."""
    bag = Bag(
        player_id="aaron",
        entries={ClubId.SEVEN_IRON: _entry(ClubId.SEVEN_IRON, make="Ping", loft_deg=32.0)},
        retired=(
            _retired(ClubId.SEVEN_IRON, _LATER, make="Titleist", loft_deg=34.0),
            _retired(ClubId.THREE_WOOD, _LATER, make="TaylorMade"),
        ),
        updated_at=_LATER,
    )

    restored = Bag.model_validate_json(bag.model_dump_json())

    assert restored == bag
    assert [e.make for e in restored.retired] == ["Titleist", "TaylorMade"], "shelf order is time"
    assert restored.retired[0].loft_deg == 34.0, "a retired club keeps the loft it was measured at"


def test_same_club_ignores_the_timestamps_and_nothing_else() -> None:
    """The comparison the store's upsert turns on, in both directions.

    False negatives cost a spurious retirement and a P16 caveat over one club's history; false
    positives pool two physical clubs under one name, which is the failure nothing can detect
    later. A loft arriving where there was none is a real change — it is the day the club became
    measurable, and the shots before it were judged without it.
    """
    titleist = _entry(ClubId.SEVEN_IRON, make="Titleist", loft_deg=34.0, shaft="Dynamic Gold")

    same_declaration_much_later = BagEntry(
        club=ClubId.SEVEN_IRON,
        make="Titleist",
        loft_deg=34.0,
        shaft="Dynamic Gold",
        recorded_at=_LATER,
        retired_at=_LATER,
    )
    assert titleist.same_club_as(same_declaration_much_later), "timestamps are not identity"

    for field, value in (
        ("make", "Ping"),
        ("model", "i230"),
        ("shaft", "Project X"),
        ("loft_deg", 32.0),
        ("length_in", 37.0),
        ("club", ClubId.EIGHT_IRON),
    ):
        other = titleist.model_copy(update={field: value})
        assert not titleist.same_club_as(other), f"{field} changed and went unnoticed"


def test_same_club_compares_every_descriptive_field_that_exists() -> None:
    """The derivation itself, since `same_club_as` excludes rather than lists.

    A field added to `BagEntry` and forgotten here would silently never count as a change. Walking
    the declaration means the pin covers it on the day it is added (R6).
    """
    compared = set(BagEntry.model_fields) - {"recorded_at", "retired_at"}
    base = _entry(ClubId.SEVEN_IRON)

    assert compared, "every field is excluded — same_club_as compares nothing"
    for field in compared:
        assert not base.same_club_as(base.model_copy(update={field: _ODD_VALUES[field]}))


#: One value per comparable field that differs from `BagEntry`'s default. Keyed by field name so
#: the test above fails loudly on a new field rather than skipping it.
_ODD_VALUES: dict[str, object] = {
    "club": ClubId.EIGHT_IRON,
    "loft_deg": 41.5,
    "make": "Ping",
    "model": "i230",
    "shaft": "Project X",
    "length_in": 37.0,
}


def test_a_retired_club_may_not_sit_in_the_bag() -> None:
    """A club the golfer no longer owns, presented as the one in their hands."""
    with pytest.raises(ValidationError, match="carries retired_at"):
        Bag(
            player_id="aaron",
            entries={ClubId.SEVEN_IRON: _retired(ClubId.SEVEN_IRON, _LATER)},
            updated_at=_LATER,
        )


def test_a_live_club_may_not_sit_on_the_shelf() -> None:
    """The other direction, and the quieter one: it drops out of `club_ids` with no record why."""
    with pytest.raises(ValidationError, match="has no retired_at"):
        Bag(
            player_id="aaron",
            retired=(_entry(ClubId.SEVEN_IRON),),
            updated_at=_LATER,
        )


def test_retired_for_is_one_slots_stints_oldest_first() -> None:
    """`restore_entry` reads `[-1]`, so backwards here restores the oldest club a golfer owned."""
    bag = Bag(
        player_id="aaron",
        retired=(
            _retired(ClubId.SEVEN_IRON, _WHEN, make="first"),
            _retired(ClubId.THREE_WOOD, _WHEN, make="a wood"),
            _retired(ClubId.SEVEN_IRON, _LATER, make="second"),
        ),
        updated_at=_LATER,
    )

    assert [e.make for e in bag.retired_for(ClubId.SEVEN_IRON)] == ["first", "second"]
    assert bag.retired_for(ClubId.SEVEN_IRON)[-1].make == "second", "the club held most recently"
    assert bag.retired_for(ClubId.DRIVER) == (), "a slot that never held another club"


def test_the_shelf_is_not_a_view_of_the_bag() -> None:
    """Retention's whole point: a club on the shelf is gone from the bag and still on disk."""
    bag = Bag(
        player_id="aaron",
        entries={ClubId.SEVEN_IRON: _entry(ClubId.SEVEN_IRON, make="Ping")},
        retired=(_retired(ClubId.THREE_WOOD, _LATER, make="TaylorMade"),),
        updated_at=_LATER,
    )

    assert bag.club_ids == (ClubId.SEVEN_IRON,), "a retired club is not in the bag"
    assert bag.retired_for(ClubId.THREE_WOOD)[0].make == "TaylorMade", "and is not lost either"
