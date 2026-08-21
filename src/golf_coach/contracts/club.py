"""The club vocabulary — which club hit a shot, as a closed set. [M9 P1]

This repo can say how a swing compares to a tour population and how it compares to the golfer's
own history. It cannot say **how far you hit your 7 iron**, because no shot on disk records which
club hit it. That one missing field is the whole of M9 — see
[ADR-024](../../docs/decisions/024-per-club-shot-history.md). This module is the vocabulary it is
recorded in, and nothing consumes it yet.

## Why a closed enum and not free text

`contracts/golfer.py:slugify` exists because "Aaron" and "aaron" splitting one golfer's baseline in
two is a silent, undetectable failure — each half then reports a confident trend over half the
data. "7i" / "7 iron" / "seven" splitting one club's carry average three ways is the same failure
with the same invisibility, and a carry average is exactly the kind of number nobody audits. So the
same posture applies: a closed set inside, one tolerant parser at the boundary, and `None` rather
than a guess when the text does not resolve.

## Why a specific club, when `ClubCategory` already exists

`ClubCategory` in `intent.py` keys benchmark rows and is the right grain for *bands* — it is the
wrong grain for *history*. "Mid iron" pools a 6 and an 8, so "my 7 iron" stops being expressible,
and that is the entire question M9 was raised to answer. The category is therefore **derived** from
the club rather than stored beside it (`category_of`), so band lookup keeps working unchanged and
there is one table here rather than two vocabularies free to disagree.

## Why the wedges are `pw`/`gw`/`sw`/`lw` and not `52`/`56`/`60`

Naming a wedge by its loft puts a measurement inside an identifier. Two 7 irons have different
lofts and one golfer's changes when it is bent or replaced, so `7i` is a **slot** and the loft
belongs to the *physical club* currently occupying it — which is a bag entry, arriving in P2. Name
the slot by its loft and a single re-grind renames the club and orphans every shot it ever hit
(ADR-024 §2).

## What this must not be wired into

`PracticeGoal.club` stays `ClubCategory.ALL`. `ranges.json` holds club-agnostic rows only, so
handing `resolve_range` a real category makes every checkpoint resolve no band and the fundamentals
panel goes dark. ADR-010 gated per-club bands and cut none; passing a derived category into scoring
is the obvious-looking move that breaks the thing that already works.

Stdlib only — no pydantic here, because a `StrEnum` and two functions is the whole shape (ADR-008).
"""

from __future__ import annotations

from enum import StrEnum

from golf_coach.contracts.intent import ClubCategory


class ClubId(StrEnum):
    """One slot in a bag. **Declaration order is canonical bag order** and is read, not decorative.

    P2's `Bag.club_ids` presents clubs in this order and the bag page lists them in it, so a golfer
    reads their bag the way it sits in the bag rather than alphabetically — where `3w` would land
    between `2h` and `5h`. Sorting anywhere downstream is the bug this ordering exists to prevent.

    Member *names* are spelled out because `3w` is not a valid Python identifier. That is not
    incidental bookkeeping: `parse_club` derives "3 wood" from the name and "3wood" from the value,
    so the two spellings of every club come from the declaration itself rather than a second list.
    """

    DRIVER = "driver"

    THREE_WOOD = "3w"
    FIVE_WOOD = "5w"
    SEVEN_WOOD = "7w"

    TWO_HYBRID = "2h"
    THREE_HYBRID = "3h"
    FOUR_HYBRID = "4h"
    FIVE_HYBRID = "5h"

    ONE_IRON = "1i"
    TWO_IRON = "2i"
    THREE_IRON = "3i"
    FOUR_IRON = "4i"
    FIVE_IRON = "5i"
    SIX_IRON = "6i"
    SEVEN_IRON = "7i"
    EIGHT_IRON = "8i"
    NINE_IRON = "9i"

    PITCHING_WEDGE = "pw"
    GAP_WEDGE = "gw"
    SAND_WEDGE = "sw"
    LOB_WEDGE = "lw"

    PUTTER = "putter"


#: Club -> the benchmark-row family it belongs to. The one mapping, so a club's category cannot be
#: two different things in two places. `tests/contracts/test_club.py` pins that every `ClubId` has
#: a row, which is what lets `category_of` index this bare.
#:
#: Written out rather than derived from the digit, because the iron boundaries are the judgement
#: call here — 1i-4i long, 5i-7i mid, 8i-9i short is a convention, not arithmetic, and burying it
#: in a range comparison makes it look like a fact. A table can be read and argued with.
#:
#: **No club maps to `ClubCategory.ALL`.** `ALL` is the club-agnostic fallback the shipped bands
#: are keyed on; a club resolving to it would silently re-point per-club work at the all-club row.
CLUB_CATEGORY: dict[ClubId, ClubCategory] = {
    ClubId.DRIVER: ClubCategory.DRIVER,
    ClubId.THREE_WOOD: ClubCategory.WOOD,
    ClubId.FIVE_WOOD: ClubCategory.WOOD,
    ClubId.SEVEN_WOOD: ClubCategory.WOOD,
    ClubId.TWO_HYBRID: ClubCategory.HYBRID,
    ClubId.THREE_HYBRID: ClubCategory.HYBRID,
    ClubId.FOUR_HYBRID: ClubCategory.HYBRID,
    ClubId.FIVE_HYBRID: ClubCategory.HYBRID,
    ClubId.ONE_IRON: ClubCategory.LONG_IRON,
    ClubId.TWO_IRON: ClubCategory.LONG_IRON,
    ClubId.THREE_IRON: ClubCategory.LONG_IRON,
    ClubId.FOUR_IRON: ClubCategory.LONG_IRON,
    ClubId.FIVE_IRON: ClubCategory.MID_IRON,
    ClubId.SIX_IRON: ClubCategory.MID_IRON,
    ClubId.SEVEN_IRON: ClubCategory.MID_IRON,
    ClubId.EIGHT_IRON: ClubCategory.SHORT_IRON,
    ClubId.NINE_IRON: ClubCategory.SHORT_IRON,
    ClubId.PITCHING_WEDGE: ClubCategory.WEDGE,
    ClubId.GAP_WEDGE: ClubCategory.WEDGE,
    ClubId.SAND_WEDGE: ClubCategory.WEDGE,
    ClubId.LOB_WEDGE: ClubCategory.WEDGE,
    ClubId.PUTTER: ClubCategory.PUTTER,
}


def category_of(club: ClubId) -> ClubCategory:
    """The benchmark-row family a club belongs to.

    Indexes `CLUB_CATEGORY` bare, with no `.get` default, for the same reason
    `UnscoredCheckpoint.spec` does: the exhaustiveness test makes a `KeyError` unreachable, and a
    fallback would mislabel a new club as some other family instead of failing where it was added.
    """
    return CLUB_CATEGORY[club]


#: The trailing letter of a numeric club id, spelled the way a golfer says it. A `KeyError` here is
#: the intended behaviour for a club id with an unrecognised suffix — the alias table is built at
#: import, so a `5x` added to `ClubId` fails immediately rather than parsing as nothing forever.
_SUFFIX_WORDS = {"w": "wood", "h": "hybrid", "i": "iron"}


def _normalize(text: str) -> str:
    """Fold text to its comparison key: lowercase, alphanumerics only.

    Dropping every non-alphanumeric rather than just spaces and hyphens is what makes "7-iron",
    "7 iron" and "7iron" one key. It is the same folding `slugify` performs, minus the accent pass
    — no club name in this taxonomy carries a diacritic.
    """
    return "".join(ch for ch in text.lower() if ch.isalnum())


def _build_aliases() -> dict[str, ClubId]:
    """Every accepted spelling -> its club, derived from the enum rather than listed beside it.

    Three forms per club, all read off the declaration: the canonical value (`7i`), the member name
    (`seveniron`, which is how "seven iron" folds), and for numeric clubs the digit with its suffix
    expanded (`7iron`). A club added to `ClubId` therefore becomes parseable in all three spellings
    without touching this function — which is the point, since the failure mode of a hand-listed
    alias table is a new club that silently parses as `None` at the bay.

    **No bare-number aliases.** "7" is a 7 iron or a 7 wood and this cannot tell which, so it does
    not try; unrecognised text is `None`'s job, not a guess's.

    Raises on a collision. Two clubs claiming one spelling would make `parse_club` return whichever
    was declared last, silently, and pooling a 3 wood's carries into a 3 hybrid's is precisely the
    kind of wrong number no one downstream can detect.
    """
    aliases: dict[str, ClubId] = {}
    for club in ClubId:
        forms = {club.value, club.name.replace("_", "")}
        if club.value[0].isdigit():
            forms.add(f"{club.value[0]}{_SUFFIX_WORDS[club.value[1:]]}")
        for form in forms:
            key = _normalize(form)
            if aliases.setdefault(key, club) is not club:
                raise ValueError(f"alias {key!r} is claimed by both {aliases[key]} and {club}")
    return aliases


_ALIASES = _build_aliases()


def parse_club(text: str) -> ClubId | None:
    """Free text to a club, or `None`. **The only place text becomes a `ClubId`.**

    Accepts the canonical id, the spoken name and the mixed form — "7i", "7 iron", "7-iron",
    "Seven Iron", " Driver ", "pitching wedge" — and returns `None` for anything else, including
    text that looks close. "wedge" and "iron" name a *category* and not a club; "10i" is not in the
    bag. Neither is nudged toward a nearest match, because the cost is asymmetric: a refused tag
    costs one retype at the bay, and a wrong one pools a wedge's carries into a 7 iron's average
    where nothing downstream will ever flag it (ADR-024 §5).

    Callers are expected to reject `None` rather than substitute a default, exactly as
    `GolferStore.get_or_create` refuses a name that slugs to nothing instead of inventing an id
    that would quietly collect every unnamed swing into one fictional golfer.
    """
    return _ALIASES.get(_normalize(text))
