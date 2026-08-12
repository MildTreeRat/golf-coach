"""Golfer identity — who swung, and which way they face. [Career mode, step 1]

Everything else in this pipeline is *derived*: phases, measurements, scores and bands can all be
recomputed from the clips whenever the code improves. These two fields cannot. They are recorded
at capture time or they are lost, which is why they land before the bay session that unblocks
career mode rather than after it (ROADMAP "Career mode — one golfer, tracked over time").

**Why identity at all.** Every band in this repo describes a tour population. Career mode judges a
swing against the golfer's own history instead, and "the golfer's own history" is not expressible
while sessions are keyed by date and swings by arrival order. The workaround already happened by
hand — a session on disk is named `2026-08-07-aaron1` — and a name inside an id is a name nobody
can read back out without parsing.

**Why handedness sits here rather than in a later milestone.** `analysis.measure` records
`head_hip_offset_impact_norm`, the one measurement that is *signed*, and its sign is
camera-relative: positive means the head sits image-right of the hip center. Which side that is in
swing terms depends entirely on handedness, so a left-handed golfer's perfectly ordinary impact
position reads as a gross fault. `measure_head_hip_offset_impact` says this at length and names
resolving handedness as the precondition for the metric ever becoming a checkpoint. It is one
question asked once per golfer, and asking it later means asking it about footage nobody can
re-examine.

Distinct from `PlayerProfile` in `intent.py`, which carries `skill_level` to key benchmark rows.
That is *intent* — what the golfer was trying to do and what population should judge it. This is
*identity* — who they are, stable across sessions. Linking them is a later, additive change.

Stdlib + pydantic only (ADR-008).
"""

from __future__ import annotations

import re
import unicodedata
from datetime import datetime
from enum import StrEnum

from pydantic import BaseModel, Field, field_validator

#: A `player_id` is lowercase alphanumerics and single hyphens. Narrow on purpose: the id becomes
#: a filename in the golfer registry and travels through `api.app`'s `_SAFE_SEGMENT` path guard,
#: so anything this pattern admits is safe in both places without a second sanitising step.
PLAYER_ID = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)*$")

_NON_SLUG = re.compile(r"[^a-z0-9]+")


class Handedness(StrEnum):
    """Which side the golfer swings from — the frame of reference for every signed metric."""

    RIGHT = "right"
    LEFT = "left"


def slugify(name: str) -> str:
    """A display name to its stable `player_id`: ``"Aaron "`` -> ``"aaron"``.

    **A collision is the feature, not a bug to engineer around.** Typing "Aaron" today and "aaron"
    next week has to land on the same golfer, or a career baseline silently splits in two and each
    half reports a confident trend over half the data. Folding case and punctuation is what makes
    identity survive being retyped on a phone keyboard between swings.

    Accents are stripped rather than replaced for the same reason, and it is the case that
    motivated doing this at all: without the NFD pass "María" folds to `mar-a` while "Maria" folds
    to `maria`, so one golfer who is inconsistent about reaching for the accent key on a phone
    keyboard becomes two golfers with half a history each — silently, and in the direction nothing
    downstream can detect. `index.html` mirrors this exactly; `tests/storage/test_golfer_store.py`
    pins the cases and the parity.

    Returns the empty string when nothing survives folding — callers reject that rather than
    inventing a default, because a golfer whose id is `unknown` is indistinguishable from a golfer
    nobody named.
    """
    folded = unicodedata.normalize("NFD", name.strip().lower())
    stripped = "".join(ch for ch in folded if not unicodedata.combining(ch))
    return _NON_SLUG.sub("-", stripped).strip("-")


class Golfer(BaseModel):
    """One person, tracked across sessions. The unit a career baseline is cut per."""

    player_id: str = Field(
        description="Stable id, the slug of the name first used. Never changes once created."
    )
    display_name: str = Field(description="The name as originally typed, for showing back.")
    handedness: Handedness
    created_at: datetime

    @field_validator("player_id")
    @classmethod
    def _validate_player_id(cls, value: str) -> str:
        if not PLAYER_ID.match(value):
            raise ValueError(f"player_id {value!r} is not a slug (expected {PLAYER_ID.pattern})")
        return value
