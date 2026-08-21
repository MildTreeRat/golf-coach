"""The bag on disk: what a write is allowed to destroy, which is nothing.

`GolferStore`'s tests pin identity stability. These pin **retention**, because the two ways this
loses data are both silent:

- a club replaced or removed vanishes along with the loft nobody measured twice, and the golfer
  discovers it the next time they want the old club back;
- a write onto a bag that cannot be parsed starts a fresh one, taking the whole shelf with it —
  the failure that got worse the moment the shelf existed, since the shelf is the part that was
  meant to survive being replaced.

The clock belongs to the store, so the timestamp assertions here are orderings and identities
rather than literals — a test that pins a wall-clock value pins the machine it ran on.
"""

from __future__ import annotations

from datetime import UTC, datetime

import pytest

from golf_coach.contracts.bag import Bag, BagEntry
from golf_coach.contracts.club import ClubId
from golf_coach.contracts.golfer import Handedness
from golf_coach.storage.bag_store import BagStore
from golf_coach.storage.golfer_store import GolferStore

_STALE = datetime(2020, 1, 1, tzinfo=UTC)


@pytest.fixture
def bags(tmp_path):
    return BagStore(tmp_path / "golfers")


def _entry(club: ClubId, **overrides: object) -> BagEntry:
    """A declaration as a caller writes one: `recorded_at` supplied and about to be ignored."""
    return BagEntry(club=club, recorded_at=_STALE, **overrides)


# --------------------------------------------------------------------------- reads


def test_a_golfer_with_no_bag_reads_as_none(bags) -> None:
    """Not an error: the bag is declared, and most golfers will not have declared one yet."""
    assert bags.get("aaron") is None


def test_a_corrupt_bag_reads_as_none(bags) -> None:
    """A reader gets leniency — a bad file costs re-entering a loft, not a 500 between swings."""
    bags.set_entry("aaron", _entry(ClubId.SEVEN_IRON))
    bags.path_for("aaron").write_text("{not json", encoding="utf-8")

    assert bags.get("aaron") is None


def test_a_bag_round_trips_through_the_store(bags) -> None:
    bags.set_entry("aaron", _entry(ClubId.SEVEN_IRON, loft_deg=34.0, make="Titleist"))
    bags.set_entry("aaron", _entry(ClubId.PITCHING_WEDGE, loft_deg=46.0))

    stored = bags.get("aaron")

    assert stored.club_ids == (ClubId.SEVEN_IRON, ClubId.PITCHING_WEDGE)
    assert stored.entries[ClubId.SEVEN_IRON].make == "Titleist"
    assert stored.entries[ClubId.PITCHING_WEDGE].loft_deg == 46.0


def test_save_writes_the_bag_it_was_given(bags) -> None:
    """`save` stamps nothing — `updated_at` belongs to whoever assembled the bag."""
    bag = Bag(player_id="aaron", updated_at=_STALE)

    bags.save(bag)

    assert bags.get("aaron") == bag


# --------------------------------------------------------------------------- the clock


def test_the_store_stamps_recorded_at_and_ignores_the_callers(bags) -> None:
    """One stamping site, in the layer that knows when the write happened."""
    stored = bags.set_entry("aaron", _entry(ClubId.SEVEN_IRON))

    assert stored.entries[ClubId.SEVEN_IRON].recorded_at > _STALE
    assert stored.entries[ClubId.SEVEN_IRON].retired_at is None


# --------------------------------------------------------------------------- upsert


def test_re_saving_an_unchanged_club_does_not_move_recorded_at(bags) -> None:
    """P19's save button on an unedited row. Re-stamping would caveat a club that never changed."""
    first = bags.set_entry("aaron", _entry(ClubId.SEVEN_IRON, loft_deg=34.0, make="Titleist"))
    declared_at = first.entries[ClubId.SEVEN_IRON].recorded_at

    again = bags.set_entry("aaron", _entry(ClubId.SEVEN_IRON, loft_deg=34.0, make="Titleist"))

    assert again.entries[ClubId.SEVEN_IRON].recorded_at == declared_at
    assert again.retired == (), "an unchanged row retired a club"
    assert bags.get("aaron").entries[ClubId.SEVEN_IRON].recorded_at == declared_at


def test_replacing_a_club_retires_the_old_one_rather_than_deleting_it(bags) -> None:
    bags.set_entry("aaron", _entry(ClubId.SEVEN_IRON, loft_deg=34.0, make="Titleist"))

    updated = bags.set_entry("aaron", _entry(ClubId.SEVEN_IRON, loft_deg=32.0, make="Ping"))

    current = updated.entries[ClubId.SEVEN_IRON]
    assert (current.make, current.loft_deg) == ("Ping", 32.0)

    shelved = updated.retired_for(ClubId.SEVEN_IRON)
    assert [(e.make, e.loft_deg) for e in shelved] == [("Titleist", 34.0)]
    assert shelved[0].retired_at is not None
    assert shelved[0].retired_at <= current.recorded_at, "the old club left before the new arrived"


def test_measuring_a_loft_for_the_first_time_is_a_change(bags) -> None:
    """The shots before it were judged without a loft; the entry is not the one it was."""
    bags.set_entry("aaron", _entry(ClubId.SEVEN_IRON, make="Ping"))

    updated = bags.set_entry("aaron", _entry(ClubId.SEVEN_IRON, make="Ping", loft_deg=32.0))

    assert updated.entries[ClubId.SEVEN_IRON].loft_deg == 32.0
    assert len(updated.retired_for(ClubId.SEVEN_IRON)) == 1, "the unmeasured entry was kept"


def test_setting_one_club_leaves_the_others_alone(bags) -> None:
    bags.set_entry("aaron", _entry(ClubId.DRIVER, make="Callaway"))

    updated = bags.set_entry("aaron", _entry(ClubId.SEVEN_IRON, make="Ping"))

    assert updated.entries[ClubId.DRIVER].make == "Callaway"
    assert updated.club_ids == (ClubId.DRIVER, ClubId.SEVEN_IRON)


# --------------------------------------------------------------------------- removal and restore


def test_removing_a_club_shelves_it(bags) -> None:
    """Including the case nothing else records: a club that leaves with no successor."""
    bags.set_entry("aaron", _entry(ClubId.THREE_WOOD, make="TaylorMade"))

    updated = bags.remove_entry("aaron", ClubId.THREE_WOOD)

    assert updated.club_ids == ()
    assert [e.make for e in updated.retired_for(ClubId.THREE_WOOD)] == ["TaylorMade"]
    assert updated.retired_for(ClubId.THREE_WOOD)[0].retired_at is not None


def test_removing_a_club_that_is_not_in_the_bag_is_none(bags) -> None:
    bags.set_entry("aaron", _entry(ClubId.DRIVER))

    assert bags.remove_entry("aaron", ClubId.THREE_WOOD) is None
    assert bags.remove_entry("nobody", ClubId.DRIVER) is None
    assert bags.get("aaron").club_ids == (ClubId.DRIVER,), "the bag was rewritten anyway"


def test_restoring_brings_back_the_previous_club(bags) -> None:
    bags.set_entry("aaron", _entry(ClubId.SEVEN_IRON, make="Titleist", loft_deg=34.0))
    bags.set_entry("aaron", _entry(ClubId.SEVEN_IRON, make="Ping", loft_deg=32.0))

    restored = bags.restore_entry("aaron", ClubId.SEVEN_IRON)

    current = restored.entries[ClubId.SEVEN_IRON]
    assert (current.make, current.loft_deg) == ("Titleist", 34.0), "the loft came back with it"
    assert current.retired_at is None


def test_restoring_copies_off_the_shelf_and_leaves_the_stint_there(bags) -> None:
    """The append-only pin, and the one that fails if a refactor makes restore a pop.

    Out and back is two stints, not one overwritten record. A pop would make the second removal of
    the same club look like its first, which is the history this exists to keep.
    """
    bags.set_entry("aaron", _entry(ClubId.SEVEN_IRON, make="Titleist"))
    bags.set_entry("aaron", _entry(ClubId.SEVEN_IRON, make="Ping"))

    restored = bags.restore_entry("aaron", ClubId.SEVEN_IRON)

    assert [e.make for e in restored.retired_for(ClubId.SEVEN_IRON)] == ["Titleist", "Ping"]
    assert restored.entries[ClubId.SEVEN_IRON].make == "Titleist"


def test_restoring_stamps_a_fresh_recorded_at(bags) -> None:
    """It re-entered the bag today; P16 caveats from when the club started hitting these shots."""
    bags.set_entry("aaron", _entry(ClubId.SEVEN_IRON, make="Titleist"))
    first = bags.get("aaron").entries[ClubId.SEVEN_IRON].recorded_at
    bags.set_entry("aaron", _entry(ClubId.SEVEN_IRON, make="Ping"))

    restored = bags.restore_entry("aaron", ClubId.SEVEN_IRON)

    assert restored.entries[ClubId.SEVEN_IRON].recorded_at >= first


def test_restoring_a_club_that_never_had_a_predecessor_is_none(bags) -> None:
    bags.set_entry("aaron", _entry(ClubId.SEVEN_IRON))

    assert bags.restore_entry("aaron", ClubId.SEVEN_IRON) is None
    assert bags.restore_entry("aaron", ClubId.DRIVER) is None
    assert bags.restore_entry("nobody", ClubId.DRIVER) is None


def test_a_removed_club_can_be_restored(bags) -> None:
    """The accidental-removal repair: one call, not a retyped loft."""
    bags.set_entry("aaron", _entry(ClubId.THREE_WOOD, make="TaylorMade", loft_deg=15.0))
    bags.remove_entry("aaron", ClubId.THREE_WOOD)

    restored = bags.restore_entry("aaron", ClubId.THREE_WOOD)

    assert restored.entries[ClubId.THREE_WOOD].loft_deg == 15.0
    assert restored.club_ids == (ClubId.THREE_WOOD,)


# --------------------------------------------------------------------------- the write guard


@pytest.mark.parametrize(
    ("mutate", "args"),
    [
        ("set_entry", (BagEntry(club=ClubId.DRIVER, recorded_at=_STALE),)),
        ("remove_entry", (ClubId.SEVEN_IRON,)),
        ("restore_entry", (ClubId.SEVEN_IRON,)),
    ],
)
def test_a_corrupt_bag_is_never_written_over(bags, mutate, args) -> None:
    """`get` collapses "no bag" and "bad bag"; a writer must not, or one upsert eats the shelf.

    All three mutators are checked because they share one reader, and a fourth added later that
    reaches for `get` instead would reintroduce exactly this.
    """
    bags.set_entry("aaron", _entry(ClubId.SEVEN_IRON, make="Titleist"))
    bags.set_entry("aaron", _entry(ClubId.SEVEN_IRON, make="Ping"))
    corrupt = "{not json"
    bags.path_for("aaron").write_text(corrupt, encoding="utf-8")

    with pytest.raises(ValueError, match="refusing to overwrite"):
        getattr(bags, mutate)("aaron", *args)

    assert bags.path_for("aaron").read_text(encoding="utf-8") == corrupt, "the file was rewritten"


def test_a_player_id_that_is_not_a_slug_never_becomes_a_filename(bags) -> None:
    """The store does no sanitising; the contract's validator runs before `save` sees a path."""
    with pytest.raises(ValueError, match="is not a slug"):
        bags.set_entry("../aaron", _entry(ClubId.DRIVER))

    assert not bags.root.exists() or list(bags.root.glob("*")) == []


# --------------------------------------------------------------------------- sharing a directory


def test_bags_and_golfers_share_a_directory_without_seeing_each_other(tmp_path) -> None:
    """The layout decision, and the only one here that can only fail on disk.

    `GolferStore.list_all` globs `*.golfer.json`, so a `.bag.json` beside it must be invisible to
    that listing — a bag showing up as an unreadable golfer would empty the golfer picker.
    """
    root = tmp_path / "golfers"
    golfers, bags = GolferStore(root), BagStore(root)

    golfers.get_or_create("Aaron", Handedness.RIGHT)
    bags.set_entry("aaron", _entry(ClubId.SEVEN_IRON, loft_deg=34.0))

    assert [g.player_id for g in golfers.list_all()] == ["aaron"]
    assert golfers.get("aaron").handedness is Handedness.RIGHT
    assert bags.get("aaron").entries[ClubId.SEVEN_IRON].loft_deg == 34.0
    assert golfers.path_for("aaron") != bags.path_for("aaron")
