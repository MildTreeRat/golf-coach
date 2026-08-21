"""The club taxonomy: complete, unambiguous, and ordered the way a bag is.

Every pin here guards a failure that is silent at the point it happens and only surfaces as a
wrong number weeks later:

- a club with no `CLUB_CATEGORY` row raises inside `category_of`, on a golfer who did nothing
  wrong except own a club someone forgot to map;
- a club mapping to `ClubCategory.ALL` would re-point per-club work at the club-agnostic band row
  and report it as a per-club answer;
- an alias claimed by two clubs silently pools one club's shots into another's carry average,
  which nothing downstream can detect;
- declaration order drifting to alphabetical reorders every bag view, and `3w` between `2h` and
  `5h` is not a bag anyone recognises.

Assertions walk the registries rather than re-listing them, so a club added later is covered by
these tests on the day it is added.
"""

from __future__ import annotations

import pytest

from golf_coach.contracts.club import CLUB_CATEGORY, ClubId, category_of, parse_club
from golf_coach.contracts.intent import ClubCategory


def test_every_club_has_a_category() -> None:
    """The exhaustiveness pin. `category_of` indexes `CLUB_CATEGORY` with no `.get` default."""
    assert set(CLUB_CATEGORY) == set(ClubId)


def test_no_club_resolves_to_the_club_agnostic_category() -> None:
    """`ALL` is the fallback the shipped bands are keyed on, not a family any club belongs to.

    A club landing here would make per-club work read the all-club row and still call the result
    per-club — the one way this module could produce a confidently wrong answer rather than none.
    """
    for club in ClubId:
        assert category_of(club) is not ClubCategory.ALL, f"{club} resolves to the fallback row"


def test_every_category_used_is_a_real_club_category() -> None:
    """The mapping targets `intent.ClubCategory` rather than a second vocabulary of its own."""
    assert set(CLUB_CATEGORY.values()) <= set(ClubCategory)


def test_parse_club_round_trips_every_canonical_id() -> None:
    """Whatever a `ClubId` serialises as must parse back to it — derived from the enum, not typed.

    This is the pin that matters for storage: a club written to a manifest as `7i` and read back
    through `parse_club` has to be the same club, or history splits at the boundary.
    """
    for club in ClubId:
        assert parse_club(club.value) is club


def test_parse_club_accepts_the_ways_a_golfer_writes_it() -> None:
    """The spellings a phone keyboard at a bay actually produces."""
    assert parse_club("7 iron") is ClubId.SEVEN_IRON
    assert parse_club("7-iron") is ClubId.SEVEN_IRON
    assert parse_club("7I") is ClubId.SEVEN_IRON
    assert parse_club("Seven Iron") is ClubId.SEVEN_IRON
    assert parse_club(" Driver ") is ClubId.DRIVER
    assert parse_club("pitching wedge") is ClubId.PITCHING_WEDGE
    assert parse_club("3 wood") is ClubId.THREE_WOOD
    assert parse_club("4 hybrid") is ClubId.FOUR_HYBRID


def test_parse_club_refuses_rather_than_guessing() -> None:
    """`None` on anything that does not resolve, including text that looks nearly right.

    "wedge" and "iron" name a category and not a club, and "7" is a 7 iron or a 7 wood. Nudging
    any of them to a nearest match buys a tag nobody typed and a carry average nobody can audit.
    """
    for junk in ("", "   ", "banana", "10i", "wedge", "iron", "wood", "7", "seven", "0i", "6w"):
        assert parse_club(junk) is None, f"{junk!r} should not resolve to a club"


def test_every_derived_spelling_resolves_to_its_own_club() -> None:
    """All three derived forms round-trip, and none is claimed by a second club.

    `_build_aliases` raises on a collision at import, so reaching this test means the table built.
    What this adds is the other direction: that each of the three spellings it derives — the id,
    the spelled name, and the digit-plus-word — actually reaches the club it came from. A form
    that silently overwrote another would have raised; a form that resolves to nobody would not.
    """
    seen: dict[str, ClubId] = {}
    for club in ClubId:
        forms = {club.value, club.name.replace("_", "")}
        if club.value[0].isdigit():
            forms.add(f"{club.value[0]} {club.name.split('_')[-1]}")  # "7 iron", "3 wood"
        for form in forms:
            resolved = parse_club(form)
            assert resolved is club, f"{form!r} resolved to {resolved} rather than {club}"
            assert seen.setdefault(form.lower(), club) is club, f"{form!r} is claimed twice"


def test_declaration_order_is_bag_order() -> None:
    """P2's `Bag.club_ids` reads this order. Sorting it anywhere downstream is the bug."""
    clubs = list(ClubId)
    assert clubs[0] is ClubId.DRIVER
    assert clubs[-1] is ClubId.PUTTER

    irons = [c for c in clubs if c.value.endswith("i")]
    assert [c.value for c in irons] == sorted(c.value for c in irons), "irons are out of order"

    def first(category: ClubCategory) -> int:
        return next(i for i, c in enumerate(clubs) if category_of(c) is category)

    assert first(ClubCategory.WOOD) < first(ClubCategory.HYBRID) < first(ClubCategory.LONG_IRON)
    assert first(ClubCategory.LONG_IRON) < first(ClubCategory.WEDGE)


def test_category_of_rejects_an_unmapped_club() -> None:
    """The bare index is deliberate — an unmapped club fails loudly where it was added.

    Pinned with a stand-in rather than a real `ClubId`, since every real one is mapped and the
    test above is what keeps it that way.
    """
    with pytest.raises(KeyError):
        category_of("5x")  # type: ignore[arg-type]
