"""What a club *is* — one slot of one manufacturer's model, as published. [M12 P1]

`contracts/club.py` gave the vocabulary (`7i` names a **slot**) and `contracts/bag.py` gave the
object occupying it. This module is the object's *specification*, and it exists because five
free-text fields turned out not to be what a club is: a Titleist T150 7 iron and a TaylorMade
Stealth 7 iron differ in loft, lie, length, offset, bounce, shaft weight and shaft profile, and
every one of those moves launch, spin and start line. `BagEntry.shaft` was one string standing in
for six independent facts. See [ADR-026](../../docs/decisions/026-club-specification-lookup.md).

## Why the unit is a slot of a model and not a model

A set has a different loft, lie, length and swing weight in every slot, so "Titleist T150" is not a
specification and "Titleist T150 7 iron" is. The key is `(make, model, model_year, club)`. This
falls out of ADR-024 §2 rather than adding to it: `7i` is a slot in *this golfer's* bag, a T150 7
iron is a slot in *the manufacturer's* set, and a `BagEntry` is the join between them.

## Why every field is optional, and why that is not the same rule as ADR-010 §2

Nothing here is defaulted, interpolated between slots or guessed — a spec the source will not
commit to arrives blank, and blank renders blank rather than zero. That is ADR-010 §2 unchanged.

What ADR-026 §1 *did* change is which value counts as known. `bag.py:74` used to say loft was
`None` unless measured, "never a catalogue default", and that reasoning compared the wrong pair: it
assumed the choice was book loft versus measured loft, where the choice a golfer with no loft
machine faces is **book loft versus nothing** — and nothing won for a whole milestone, leaving
`data/processed/golfers/` with no `.bag.json` at all. So the manufacturer's number is the loft. The
club bent 2° strong reads its book loft and nothing downstream knows; that cost is on the record in
ADR-026 §1 and is accepted rather than solved, because the field is editable by the person who had
it bent.

## Why two closed vocabularies here and none for `head_type`

`shaft_material` and `shaft_flex` are `StrEnum`s with tolerant parsers, for ADR-024 §1's reason:
"S", "stiff" and "Stiff Flex" are one flex, and left as free text they are three, splitting one
population silently the way "7i"/"7 iron"/"seven" would have split a club's carry average.

`head_type`, `set_composition`, `grind`, `shaft_kick_point` and `swing_weight` stay strings on
purpose. They are marketing vocabulary with no agreed boundary — one maker's "players distance" is
another's "game improvement" — so a closed set here would be this module inventing a taxonomy
rather than recording one, and a parser refusing text that is *correct* is worse than a string
nobody can group by. `swing_weight` is a string because "D2" is a letter-and-digit scale, not a
number to do arithmetic on.

## Five surfaces now stand on this field list, and four of them derive it

`BagEntry` inherits it (P2), `clubs/catalogue.py` keys and stores it (P3), `clubs/lookup.py` walks
`model_fields` to build both the schema the model is given and the coercion that reads its answer
(P4), and `api/app.py` derives `BagEntryRequest` from it and `_spec_of` back out of a `BagEntry`
(P5). None of those four writes the names down twice — adding a field here is meant to reach all of
them without a second edit.

The fifth is `api/static/career.html`'s `SPEC_FIELDS`, and it **is** hand-written, because a form
control is a layout decision and not a type. `tests/api/test_career_page.py` parses it out of the
HTML and compares it to `model_fields`, so the one surface that cannot derive the list is pinned
against the one that defines it. A field added here without a control fails that test rather than
rendering nowhere.

Stdlib + pydantic only (ADR-008).
"""

from __future__ import annotations

from datetime import datetime
from enum import StrEnum
from typing import Self, TypeVar

from pydantic import BaseModel, Field, model_validator

from golf_coach.contracts.club import ClubId

_Member = TypeVar("_Member", bound=StrEnum)


class ShaftMaterial(StrEnum):
    """What the shaft is made of. Three answers, because that is how shafts are actually sold."""

    STEEL = "steel"
    GRAPHITE = "graphite"
    MULTI_MATERIAL = "multi_material"


class ShaftFlex(StrEnum):
    """How much the shaft bends. **Declaration order is softest to stiffest** and is read.

    Ordering the members is not decorative: a flex scale has a direction, and a page listing them
    alphabetically would put ladies between extra-stiff and regular. `ClubId` makes the same
    promise about bag order for the same reason — the enum is the one place the order is written
    down, so nothing downstream has to sort and get it backwards.

    The scale is not linear and is not comparable across makers: one brand's regular is another's
    stiff, and no number here would make that true. So this is a label, never an arithmetic
    quantity, and nothing should subtract two of them.
    """

    LADIES = "ladies"
    SENIOR = "senior"
    REGULAR = "regular"
    STIFF = "stiff"
    X_STIFF = "x_stiff"
    XX_STIFF = "xx_stiff"


def _normalize(text: str) -> str:
    """Fold to the form the alias tables are keyed in: lowercase, alphanumerics only.

    Deliberately the same fold `club.py:_normalize` applies, spelled out again rather than imported
    across, because these are two vocabularies and tying them together would mean a change made for
    club spellings silently changing how a flex parses. Two four-line functions that agree today is
    the cheaper failure than one that has to serve both.

    A trailing "flex" is dropped, which is what turns "Stiff Flex", "R Flex" and "Regular Flex"
    into the three aliases already in the table rather than three more rows to remember to add.
    Nothing else is stripped — a prefix or an infix is text this does not recognise, and
    recognising less is the point.
    """
    folded = "".join(ch for ch in text.lower() if ch.isalnum())
    if folded.endswith("flex") and folded != "flex":
        folded = folded[: -len("flex")]
    return folded


def _build_aliases(members: type[_Member], extra: dict[str, str]) -> dict[str, _Member]:
    """Every accepted spelling -> its member: the enum's own forms, plus the ones it cannot derive.

    The derived half is `club.py:_build_aliases`'s argument unchanged — the canonical value and the
    member name both come off the declaration, so a member added later is parseable without editing
    a table. The hand-written half is what that argument does not cover: "S" and "A" are not
    derivable from `STIFF` and `SENIOR` by any rule, they are the letter codes actually printed on
    shaft bands, and no amount of deriving invents them.

    Raises on a collision, exactly as `club.py` does. Two members claiming one spelling would make
    the parser return whichever was declared last, silently — and a graphite shaft recorded as
    steel is a wrong number no reader downstream can detect.
    """
    aliases: dict[str, _Member] = {}
    for member in members:
        for form in (member.value, member.name.replace("_", " ")):
            key = _normalize(form)
            if aliases.setdefault(key, member) is not member:
                raise ValueError(f"alias {key!r} is claimed by both {aliases[key]} and {member}")
    for spelling, value in extra.items():
        key = _normalize(spelling)
        member = members(value)
        if aliases.setdefault(key, member) is not member:
            raise ValueError(f"alias {key!r} is claimed by both {aliases[key]} and {member}")
    return aliases


#: Spellings no rule derives from the declaration. The letter codes are what is printed on the
#: shaft band, which is the only place most golfers have ever seen their flex written down.
#:
#: **"composite" and "hybrid" are deliberately absent.** "Composite" is used by makers for both a
#: graphite shaft and a genuinely multi-material one, so it names two members and may not pick
#: either; "hybrid" already names a club here (`ClubId.THREE_HYBRID`), and a material table that
#: quietly answers a club-type word is the kind of near-miss `parse_club` refuses on principle.
_MATERIAL_EXTRA = {
    "carbon": "graphite",
    "carbon fiber": "graphite",
    "carbon fibre": "graphite",
    "multi material": "multi_material",
}

_FLEX_EXTRA = {
    "l": "ladies",
    "w": "ladies",
    "a": "senior",
    "m": "senior",
    "r": "regular",
    "reg": "regular",
    "s": "stiff",
    "x": "x_stiff",
    "extra stiff": "x_stiff",
    "xx": "xx_stiff",
    "double extra stiff": "xx_stiff",
}

_MATERIAL_ALIASES = _build_aliases(ShaftMaterial, _MATERIAL_EXTRA)
_FLEX_ALIASES = _build_aliases(ShaftFlex, _FLEX_EXTRA)


def parse_shaft_material(text: str) -> ShaftMaterial | None:
    """Free text to a shaft material, or `None`. **The only place text becomes a `ShaftMaterial`.**

    Same posture as `parse_club` (`club.py:185`) and for the same asymmetry: a refused value costs
    one correction in a form the golfer is already looking at, and a nudged one puts a graphite
    shaft's weight against a steel label where nothing downstream will ever flag it. So text this
    does not recognise is `None` and never a nearest match.

    Callers reject `None` rather than substituting a default — for the lookup path in P4 that means
    leaving the field blank on the form, which is visible, rather than filling it with a guess,
    which is not.
    """
    return _MATERIAL_ALIASES.get(_normalize(text))


def parse_shaft_flex(text: str) -> ShaftFlex | None:
    """Free text to a shaft flex, or `None`. **The only place text becomes a `ShaftFlex`.**

    Accepts the letter code, the word and the "… Flex" spelling — "S", "stiff", "Stiff Flex",
    "  STIFF  " are one flex — and refuses everything else, including text that looks close:
    "regular-ish" is `None` and not `REGULAR`, because a hedge is not a flex and recording it as
    one loses the hedge.
    """
    return _FLEX_ALIASES.get(_normalize(text))


class SpecProvenance(BaseModel):
    """Where a specification came from, so a looked-up number and a typed one stay distinguishable.

    `ranges.json` carries provenance per row for this reason and it is the same reason here: the
    fields on `ClubSpec` do not say whether a loft was read off a manufacturer's page, recalled by
    a model, or typed in by the golfer holding the club, and those are not equally trustworthy. A
    reader that cannot tell them apart has to treat all three as the weakest.

    It is a separate block rather than three fields on `ClubSpec` because it is **not part of club
    identity** — see `BagEntry.same_club_as`, which compares by exclusion and must exclude this
    whole block (ADR-026 §4). Nesting it makes that exclusion one name instead of three, and three
    names is three chances to add a fourth field that is never excluded.
    """

    source: str = Field(
        description=(
            "What produced these values — `\"llm:claude-opus-5\"`, `\"catalogue\"` or `\"typed\"`. "
            "A string and not an enum: the model id is part of the answer, and pinning the set "
            "would mean editing a contract every time the coaching model moves."
        )
    )
    retrieved_at: datetime = Field(
        description=(
            "When it was retrieved. A specification is stable for the life of a model, so this "
            "dates the *retrieval* and not the club — it is what lets a catalogue row looked up "
            "under one model be re-checked later without guessing how old it is."
        )
    )
    notes: str = Field(
        default="",
        description=(
            "Anything the source said about its own confidence, kept verbatim. Empty is the "
            "normal case; a value here is usually the reason a field beside it is blank."
        ),
    )


class ClubSpec(BaseModel):
    """One slot of one model, as published. The field list `BagEntry` inherits in P2.

    Grouped below as identity, head, shaft, assembly and head performance — the grouping is real
    and P6's form renders it, so a field added here lands in the group it is declared in.

    Every field except `club` is `None` or `""` when unknown, and that is the whole of the "no
    score beats a wrong one" rule applied to inputs: a blank lie angle is a lie angle nobody knows,
    which is a true statement, where `0.0` is a club that would be unplayable.
    """

    # --- Identity: which slot of whose model this is. `catalogue_key` (P3) is built from four of
    # these, and `club` is the one field that is never blank — a spec with no slot is not a spec.
    club: ClubId = Field(
        description=(
            "The slot of the manufacturer's set this describes. Carried on the spec rather than "
            "implied by where it is filed, for `BagEntry`'s reason: an entry travels alone, and "
            "the container pins its key against this rather than trusting them to agree."
        )
    )
    make: str = ""
    model: str = ""
    model_year: int | None = Field(
        default=None,
        description=(
            "The model year, which is part of the key and not decoration: makers reuse a model "
            "name across generations with different lofts, so two `P790` 7 irons four years apart "
            "are two specifications. `None` means the year was not established — the lookup is "
            "expected to leave it blank rather than pick the newest."
        ),
    )
    head_type: str = ""
    set_composition: str = ""

    # --- Head: the numbers that decide launch and start line before the shaft is considered.
    loft_deg: float | None = Field(
        default=None,
        description=(
            "The published loft. **This is ADR-026 §1's reversal of ADR-024 §2** — it used to read "
            "'measured loft, never a catalogue default', and the alternative that reasoning "
            "assumed (a measured number) is not one a home golfer has. A club that has been bent "
            "reads its book loft here and nothing downstream knows; that is accepted, and a "
            "separate measured field is deferred to the day someone models the difference."
        ),
    )
    lie_deg: float | None = None
    bounce_deg: float | None = None
    grind: str = ""
    offset_mm: float | None = None
    face_angle_deg: float | None = None
    head_weight_g: float | None = None
    adjustable_hosel: bool | None = Field(
        default=None,
        description=(
            "Whether the hosel adjusts. `None` is not `False`: an unknown hosel and a fixed one "
            "are different states, and collapsing them would make `loft_range_deg` look absent "
            "when it is merely unretrieved."
        ),
    )
    loft_range_deg: tuple[float, float] | None = Field(
        default=None,
        description=(
            "The adjustable range as (min, max), for a club whose hosel moves. `loft_deg` stays "
            "the setting the club is in, so the two answer different questions and a driver at "
            "9.0° with a 7.25-10.75 range is fully described by both."
        ),
    )

    # --- Shaft: the six independent facts `BagEntry.shaft` was one string for (ADR-026 §3).
    shaft_model: str = ""
    shaft_material: ShaftMaterial | None = None
    shaft_flex: ShaftFlex | None = None
    shaft_weight_g: float | None = None
    shaft_torque_deg: float | None = None
    shaft_kick_point: str = ""

    # --- Assembly: what the components became once built to this golfer's club.
    length_in: float | None = None
    swing_weight: str = Field(
        default="",
        description=(
            "The swing-weight code, e.g. `\"D2\"`. A string because the scale is a letter and a "
            "digit — D2 to D3 is one point but D9 to E0 is also one point, so nothing may do "
            "arithmetic on it, and storing it as a number would invite exactly that."
        ),
    )
    total_weight_g: float | None = None
    grip: str = ""

    # --- Head performance: published, rarely known, and never inferred from the fields above.
    cor: float | None = None
    moi_g_cm2: float | None = None
    usga_conforming: bool | None = None

    @model_validator(mode="after")
    def _loft_range_is_ordered(self) -> Self:
        """An adjustable range must run low to high.

        Raises rather than sorting, on `Bag._keys_match_entries`' reasoning: a reversed pair is a
        caller bug and not a state worth reconciling, and silently swapping it would hide the
        upstream mistake that produced it. It matters because every use of a range is a comparison
        — is this loft inside it — and a reversed pair makes that answer `False` for every loft,
        including the one the club is actually set to.
        """
        if self.loft_range_deg is not None:
            low, high = self.loft_range_deg
            if low > high:
                raise ValueError(f"loft_range_deg {self.loft_range_deg} runs high to low")
        return self
