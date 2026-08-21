"""The bag — which physical club is in each slot, what it is, and what used to be. [M9 P2/P3]

`contracts/club.py` gave the vocabulary: `7i` names a **slot**, not an object. This module is the
object. Two 7 irons have different lofts and one golfer's changes when it is bent or replaced, so
loft, make, model, shaft and length belong to a per-golfer record of the club currently occupying
the slot — a bag entry — and never to the identifier (ADR-024 §2).

## Why the bag is declared when everything else here is derived

Almost nothing in this repo is typed in: phases, measurements, scores and bands all recompute from
the clips. A bag cannot. **"Clubs used" stays derived** from shot history — always true, no upkeep
— and this is a different question that also has an answer: a club in the bag you have not hit has
no statistics and is not an error, and a club you have hit that has left the bag still has real
history. Collapsing the two into one list loses which is which; P14's `BagProfile` keeps them
apart for that reason.

The reason to record it *now*, before the bay session that makes any of it speak, is that loft is
the anchor club fitting needs and it is unrecoverable after the fact. The fitting models are not —
so the inputs land now and the models wait (ADR-024, Deferred).

## Retention is not versioning, and the difference is load-bearing

A club that leaves the bag is **kept**, on `Bag.retired`, rather than deleted. That is retention: a
golfer who goes back to last year's 7 iron re-declares it instead of retyping a loft nobody
measured twice, and the record of the club that was there is not destroyed by the act of replacing
it.

What it deliberately is **not** is ADR-024's deferred *bag entry versioning* — attributing each
stored shot to the stint that hit it. Nothing reads `Bag.retired`. P16 still builds its caveat from
the **current** entry's `recorded_at`, naming a date rather than modelling a history, exactly as
the ADR says. The shelf makes that modelling possible later without making it a promise now, and
the day something starts joining shots to stints this paragraph is wrong — that needs a decision in
ADR-024 rather than an addendum to it.

Nothing consumes this yet: P3 puts it on disk, P14 composes `BagEntry` into a club profile, P16
reads `recorded_at`.

Stdlib + pydantic only (ADR-008).
"""

from __future__ import annotations

from datetime import datetime
from typing import Self

from pydantic import BaseModel, Field, field_validator, model_validator

from golf_coach.contracts.club import ClubId
from golf_coach.contracts.golfer import PLAYER_ID


class BagEntry(BaseModel):
    """The physical club occupying one slot, as the golfer declared it.

    Every descriptive field is optional and empty by default, `loft_deg` most importantly: a golfer
    who has never put their irons on a loft machine still has a bag, and refusing to record one
    until they do would mean recording nothing. The work that needs loft refuses **per club** when
    it is absent, which is `SwingResult.unscored`'s posture applied to a different missing input —
    the clubs that do have a loft are unaffected, and the one that does not is named rather than
    defaulted to a book value nobody measured (ADR-010 §2, ADR-024 §2).
    """

    club: ClubId = Field(
        description=(
            "The slot this club fills. Carried on the entry even though `Bag.entries` is keyed by "
            "it, because an entry travels alone: `BagStore.set_entry` upserts one, and P14's "
            "`ClubProfile.bag_entry` holds one with no dict around it. `Bag` pins the two against "
            "each other rather than trusting them to agree."
        )
    )

    loft_deg: float | None = Field(
        default=None,
        description="Measured loft. None means unmeasured, and never a catalogue default.",
    )
    make: str = ""
    model: str = ""
    shaft: str = ""
    length_in: float | None = None

    recorded_at: datetime = Field(
        description=(
            "When this entry was declared. Load-bearing, not bookkeeping: a bag entry that changes "
            "makes the shots before and after it two different physical clubs pooled under one "
            "name. P16 turns that into a caveat naming this date rather than withholding the "
            "statistic, because an entry recorded late for a club that never changed is the more "
            "likely case and the cost of being wrong is one sentence."
        )
    )

    retired_at: datetime | None = Field(
        default=None,
        description=(
            "When this club left the bag, or None while it is still in it. `Bag` pins the two "
            "states against the list the entry is in, so this field is never the only thing "
            "claiming a club is current. It exists because a club can leave with no successor — "
            "taking the 3 wood out and putting nothing in the slot — and then nothing else on disk "
            "says when. A stint's end is derivable from the next stint's `recorded_at` only while "
            "there is a next stint, which is exactly the case this field is not needed for."
        ),
    )

    def same_club_as(self, other: BagEntry) -> bool:
        """Is this the same physical club as `other`, ignoring when it was declared?

        The store's whole upsert decision turns on this: re-saving an unchanged row is an edit and
        must not move `recorded_at`, while a genuinely different club retires the old one. Club
        identity lives here rather than in `BagStore` because it is a fact about what a bag entry
        *is*, and a store that owned the definition would be a second place to change it.

        Derived by excluding the two timestamps rather than by listing the descriptive fields, so a
        field added to `BagEntry` later is compared from the day it is added. A hand-listed tuple's
        failure mode is a new field that silently never counts as a change — a re-shafted club that
        reads as the same one and pools two populations of shots.

        This instrument has no serial numbers, so two identically-specified 7 irons are one club as
        far as anything here can see. That is the honest answer rather than a limitation to work
        around: nothing downstream could act on the distinction.
        """
        timestamps = {"recorded_at", "retired_at"}
        return self.model_dump(exclude=timestamps) == other.model_dump(exclude=timestamps)


class Bag(BaseModel):
    """One golfer's declared bag, plus every club that has left it. The unit `BagStore` stores."""

    player_id: str = Field(description="The `Golfer.player_id` this bag belongs to.")
    entries: dict[ClubId, BagEntry] = Field(
        default_factory=dict,
        description=(
            "Slot -> the club in it. Sparse: a bag holds what has been declared, and an absent "
            "club is undeclared rather than missing."
        ),
    )
    retired: tuple[BagEntry, ...] = Field(
        default=(),
        description=(
            "Finished stints, oldest first — every club that has left the bag, in the order it "
            "left. **Append-only**: `BagStore.restore_entry` copies off this list and leaves the "
            "record in place, so a 7 iron swapped out, back in and out again is three stints here "
            "rather than one. A flat list and not a dict keyed by club, because `BagEntry.club` "
            "already names the slot and a second key is a second thing that can disagree with the "
            "entry it holds — the bug `_keys_match_entries` exists to catch."
        ),
    )
    updated_at: datetime

    @field_validator("player_id")
    @classmethod
    def _validate_player_id(cls, value: str) -> str:
        """Same slug check `Golfer` applies, from the same pattern.

        Enforced here rather than in the store because `BagStore` writes `<root>/<player_id>.json`
        and reads the id straight into that path. Validating in the contract is what makes "already
        slug-validated, so no second sanitising step" true at the point it matters.
        """
        if not PLAYER_ID.match(value):
            raise ValueError(f"player_id {value!r} is not a slug (expected {PLAYER_ID.pattern})")
        return value

    @model_validator(mode="after")
    def _keys_match_entries(self) -> Self:
        """A slot may not hold a club that says it is a different club.

        The same guard `_build_aliases` raises with, for the same reason: filing a sand wedge's
        entry under `7i` is invisible at the point it happens and surfaces later as a loft attached
        to the wrong club's history. One of the two spellings has to lose, and a mismatch is a
        caller bug rather than a state worth reconciling — so it raises instead of picking.
        """
        for slot, entry in self.entries.items():
            if entry.club is not slot:
                raise ValueError(f"entry filed under {slot} declares itself {entry.club}")
        return self

    @model_validator(mode="after")
    def _live_and_retired_are_separated(self) -> Self:
        """`retired_at` must agree with which list the entry is in.

        Kept apart from `_keys_match_entries` because it catches a different failure: that one stops
        a loft reaching the wrong club, this one stops a club the golfer no longer owns being
        presented as the one in their hands. Both directions are checked — a live entry parked on
        the shelf is a club that has quietly left the bag while still being listed as history, which
        loses it from `club_ids` without anything reporting a change.
        """
        for slot, entry in self.entries.items():
            if entry.retired_at is not None:
                raise ValueError(f"entry in slot {slot} is in the bag but carries retired_at")
        for entry in self.retired:
            if entry.retired_at is None:
                raise ValueError(f"retired entry for {entry.club} has no retired_at")
        return self

    @property
    def club_ids(self) -> tuple[ClubId, ...]:
        """The declared clubs, in canonical bag order.

        Walks `ClubId` rather than the dict, so order comes from the declaration in `club.py` and
        not from what happened to be added first. Sorting is the bug this exists to prevent: `3w`
        lands between `2h` and `5h` alphabetically, which is not a bag anyone recognises.
        """
        return tuple(club for club in ClubId if club in self.entries)

    def retired_for(self, club: ClubId) -> tuple[BagEntry, ...]:
        """This slot's finished stints, oldest first — so `[-1]` is the club held before this one.

        One home for that lookup. `BagStore.restore_entry` is its only caller today and P19's bag
        page will be the second; two hand-rolled filters over `retired` are two places to get the
        ordering backwards, and backwards here restores the oldest club a golfer ever owned.
        """
        return tuple(entry for entry in self.retired if entry.club is club)
