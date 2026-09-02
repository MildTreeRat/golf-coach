"""The club specification shape: two closed vocabularies that refuse, and a blank that stays blank.

Every pin here guards a failure that produces a plausible-looking club rather than an error:

- `parse_shaft_flex` nudging "regular-ish" to `REGULAR` records a hedge as a fact, and a flex is
  the field a golfer is least likely to re-check once it looks filled in;
- the flex spellings splitting — "S" one club, "Stiff Flex" another — is `parse_club`'s failure
  moved to a new vocabulary, and it is invisible in exactly the same way;
- an alias claimed by two members makes the parser return whichever was declared last, so a
  graphite shaft lands under `STEEL` with nothing downstream able to tell;
- `ShaftFlex` losing its softest-to-stiffest declaration order puts ladies between extra-stiff and
  regular on P6's form, which reads as a bug in the bag rather than a bug in the enum;
- a `ClubSpec` that fills anything in by itself is ADR-026 §2's rule broken at the source — the
  whole point of a blank field is that it renders blank instead of as a number nobody supplied;
- a `loft_range_deg` that runs high to low makes every "is this loft inside the range" comparison
  `False`, including for the loft the club is actually set to.

Assertions derive their expectations from the enums rather than re-typing the member list, so a
material or flex added later is covered on the day it is added.
"""

from __future__ import annotations

from datetime import UTC, datetime

import pytest
from pydantic import ValidationError

from golf_coach.contracts.club import ClubId
from golf_coach.contracts.club_spec import (
    ClubSpec,
    ShaftFlex,
    ShaftMaterial,
    SpecProvenance,
    parse_shaft_flex,
    parse_shaft_material,
)

_WHEN = datetime(2026, 8, 31, 12, 0, tzinfo=UTC)


@pytest.mark.parametrize(
    "text", ["S", "s", "stiff", "Stiff", "Stiff Flex", "  STIFF  ", "stiff-flex"]
)
def test_every_spelling_of_stiff_is_one_flex(text: str) -> None:
    """The four ways a shaft band, a retailer and a golfer each write the same flex.

    This is the whole reason `ShaftFlex` is a closed set. Left as free text these are four values,
    and a bag with two of them in it has two shafts where the golfer has one — the same silent
    split `slugify` exists to stop for golfers and `parse_club` for clubs.
    """
    assert parse_shaft_flex(text) is ShaftFlex.STIFF


@pytest.mark.parametrize(
    ("text", "expected"),
    [
        ("L", ShaftFlex.LADIES),
        ("A", ShaftFlex.SENIOR),
        ("senior", ShaftFlex.SENIOR),
        ("R", ShaftFlex.REGULAR),
        ("Reg", ShaftFlex.REGULAR),
        ("R Flex", ShaftFlex.REGULAR),
        ("X", ShaftFlex.X_STIFF),
        ("Extra Stiff", ShaftFlex.X_STIFF),
        ("x_stiff", ShaftFlex.X_STIFF),
        ("XX", ShaftFlex.XX_STIFF),
    ],
)
def test_letter_codes_and_words_reach_the_same_members(text: str, expected: ShaftFlex) -> None:
    """The letter code is the only place most golfers have seen their flex written down.

    Neither "A" nor "R" is derivable from `SENIOR` or `REGULAR` by any rule, which is why the alias
    table has a hand-written half at all — deriving alone would refuse the printed spelling.
    """
    assert parse_shaft_flex(text) is expected


@pytest.mark.parametrize(
    ("text", "expected"),
    [
        ("steel", ShaftMaterial.STEEL),
        ("Steel", ShaftMaterial.STEEL),
        ("graphite", ShaftMaterial.GRAPHITE),
        ("Carbon Fiber", ShaftMaterial.GRAPHITE),
        ("carbon fibre", ShaftMaterial.GRAPHITE),
        ("multi-material", ShaftMaterial.MULTI_MATERIAL),
    ],
)
def test_shaft_materials_parse_from_their_spoken_spellings(
    text: str, expected: ShaftMaterial
) -> None:
    assert parse_shaft_material(text) is expected


@pytest.mark.parametrize("text", ["regular-ish", "fairly stiff", "flex", "medium", "", "   ", "7i"])
def test_a_flex_that_is_not_one_refuses_rather_than_nudging(text: str) -> None:
    """`None` and never a nearest match — `parse_club`'s asymmetry applied to a second vocabulary.

    "regular-ish" is the one that matters: it *contains* a member's whole spelling, so any parser
    built on substring matching returns `REGULAR` and loses the hedge that was the point of typing
    it. A refused value costs one correction in a form already on screen; an accepted wrong one
    costs a shaft profile nobody re-checks.
    """
    assert parse_shaft_flex(text) is None


@pytest.mark.parametrize("text", ["composite", "hybrid", "titanium", "aluminium", ""])
def test_a_material_that_names_two_answers_or_none_refuses(text: str) -> None:
    """The two deliberate absences, pinned so removing them is a decision rather than a slip.

    "composite" is used by makers for both a graphite shaft and a genuinely multi-material one, so
    it names two members and may pick neither. "hybrid" already names a club (`ClubId.THREE_HYBRID`)
    and a material table quietly answering a club-type word is the near-miss `parse_club` refuses
    on principle.
    """
    assert parse_shaft_material(text) is None


@pytest.mark.parametrize("member", list(ShaftFlex) + list(ShaftMaterial))
def test_every_member_parses_from_its_own_value(member: ShaftFlex | ShaftMaterial) -> None:
    """A member added later is parseable the day it is added, without editing the alias table.

    This is the derived half of `_build_aliases` doing its job. Its failure mode is a new flex that
    silently returns `None` at the boundary, which reads as "the golfer typed something odd" rather
    than "the vocabulary grew and the parser did not".
    """
    parse = parse_shaft_flex if isinstance(member, ShaftFlex) else parse_shaft_material
    assert parse(member.value) is member


def test_flex_declaration_order_runs_softest_to_stiffest() -> None:
    """Order is read, not decorative: P6's form lists the members in declaration order.

    Pinned here rather than left to the reader because the failure is not an exception — it is a
    picker that offers ladies between extra-stiff and regular, which a golfer has no way to
    distinguish from a bug in their own bag.
    """
    assert list(ShaftFlex) == [
        ShaftFlex.LADIES,
        ShaftFlex.SENIOR,
        ShaftFlex.REGULAR,
        ShaftFlex.STIFF,
        ShaftFlex.X_STIFF,
        ShaftFlex.XX_STIFF,
    ]


def test_a_spec_with_only_a_club_is_valid_and_fills_nothing_in() -> None:
    """The blank bag entry ADR-026 §2 promises: a slot, and no number anyone did not supply.

    The pin reads "falsy" rather than "`None`" because the shape has two kinds of blank — `None`
    for the measured fields and `""` for the free-text ones, matching `BagEntry` today so P2's
    inheritance changes no existing default. What matters is that neither is a *value*: a `0.0`
    lie angle would describe an unplayable club, and a `"steel"` shaft nobody mentioned would be a
    fact this program invented.
    """
    spec = ClubSpec(club=ClubId.SEVEN_IRON)

    assert spec.club is ClubId.SEVEN_IRON
    filled = {
        name: value
        for name, value in spec.model_dump().items()
        if name != "club" and value
    }
    assert filled == {}


def test_a_spec_round_trips_through_json() -> None:
    """Enums, a tuple range and the provenance block all survive storage.

    P3's catalogue and P5's route both move this shape as JSON, so a field that does not round-trip
    is one that quietly resets every time a club is saved — and the field most likely to do it is
    `loft_range_deg`, since a tuple is a list on the way back.
    """
    spec = ClubSpec(
        club=ClubId.DRIVER,
        make="Titleist",
        model="TSR3",
        model_year=2022,
        loft_deg=9.0,
        adjustable_hosel=True,
        loft_range_deg=(7.25, 10.75),
        shaft_material=ShaftMaterial.GRAPHITE,
        shaft_flex=ShaftFlex.STIFF,
        swing_weight="D3",
    )

    restored = ClubSpec.model_validate_json(spec.model_dump_json())

    assert restored == spec
    assert restored.loft_range_deg == (7.25, 10.75)
    assert restored.shaft_flex is ShaftFlex.STIFF


def test_an_adjustable_range_that_runs_backwards_raises() -> None:
    """Raises rather than sorting, on `Bag._keys_match_entries`' reasoning.

    Every use of a range is a containment comparison, and a reversed pair answers `False` for every
    loft including the one the club is set to — so the club looks out of its own range. Swapping it
    silently would hide whatever upstream produced it, which for M12 is an LLM that returned two
    numbers in the order it happened to say them.
    """
    with pytest.raises(ValidationError, match="runs high to low"):
        ClubSpec(club=ClubId.DRIVER, loft_range_deg=(10.75, 7.25))


def test_an_equal_ended_range_is_accepted() -> None:
    """A hosel with one setting is a degenerate range, not an error.

    The boundary is pinned because `low > high` and `low >= high` differ only here, and rejecting
    it would refuse a club that genuinely exists in favour of a tidier invariant.
    """
    spec = ClubSpec(club=ClubId.DRIVER, loft_range_deg=(9.0, 9.0))

    assert spec.loft_range_deg == (9.0, 9.0)


def test_provenance_carries_its_source_and_defaults_its_notes_empty() -> None:
    """The block `same_club_as` must exclude whole (ADR-026 §4), and why it is nested.

    `notes` defaulting to empty is what makes the common case — a clean lookup — carry no prose,
    so a value in it is always a real signal about the fields beside it rather than boilerplate.
    """
    provenance = SpecProvenance(source="llm:claude-opus-5", retrieved_at=_WHEN)

    assert provenance.source == "llm:claude-opus-5"
    assert provenance.retrieved_at == _WHEN
    assert provenance.notes == ""
