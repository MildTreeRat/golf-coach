"""What a stored shot has to borrow from the swing it was hit with. [M15 P10]

`analysis/flight_infer.py` resolves the two launch conditions the HD Golf screen does not print,
and it needs two things the shot itself does not carry: **the club's loft**, which picks the branch
of the spin solve, and **the golfer's handedness**, which flips the spin axis out of the contract's
`+ = fade` into the integrator's geometric sign. Neither is on `ShotData` and neither ever will be
— a launch monitor reports a ball, not a bag — so this module is the join that finds them.

**The join key is the shot photo's sha256**, which both sides already record: `ShotStore` files a
parse under `ShotProvenance.image_sha256`, and `SwingManifest.roles[SHOT_SCREEN].content_sha256` is
the same digest, taken as the upload streamed in. So no new identifier is introduced and nothing is
re-hashed here — the same content addressing `api/pipeline.py::_shot_for` uses to attach a cached
parse without opening the image, read the other way round.

## The finding this module exists to have produced

ADR-027's 2026-09-05g addendum says every shot on disk refuses the loft prior *because no shot
carries a club*. That is true of `ShotData` and false of the corpus: **eleven of the thirteen shots
are attached to a swing that carries a club tag** — five `7i` and six `3w`, tagged at capture on
2026-08-23 — and the tag reaches the shot the moment anything performs this join. What is actually
missing is narrower and cheaper to fix: only the 7 iron has a *declared loft* (30.5 deg, M15 P1's
bag correction), so the six 3 wood shots refuse for a bag entry nobody has filled in rather than
for a bay session nobody has taken. `LoftGap` is that distinction made countable.

## Why it takes shots rather than reading them

`ShotStore` lives in `launch_monitor/`, and `storage/` may not import it (ADR-008: modules import
`contracts` and never each other). The shells and `scripts/` may reach both, so the caller reads
the shots — `ShotStore(settings.shots_dir).all()`, already tolerant of an unreadable file — and
hands them here.

**M15 P11 turned out to want the other half of this, not this.** `api/pipeline.py` is already
holding the manifest when it needs a loft, so joining a shot photo's sha256 against every swing on
disk would be re-deriving what it knows — and `_swings_by_shot_photo` parses every manifest in
every session to do it. `loft_for_club` is that half exposed: the bag lookup and its four-way gap,
with the join skipped. Both routes run the same `_resolve_loft`, so there is one definition of what
a declared loft is and one of what its absence means.

Base install only: manifests, bags and golfers are JSON, and nothing here opens a video or an image.
"""

from __future__ import annotations

from collections.abc import Iterable
from enum import StrEnum
from pathlib import Path
from typing import NamedTuple

from golf_coach.contracts.club import ClubId
from golf_coach.contracts.golfer import Handedness
from golf_coach.contracts.shot import ShotData
from golf_coach.storage.bag_store import BagStore
from golf_coach.storage.bundle_store import SwingBundleStore
from golf_coach.storage.golfer_store import GolferStore
from golf_coach.storage.manifest import Role, SwingManifest


class LoftGap(StrEnum):
    """Why a shot has no loft — four different repairs wearing one `None`.

    The reason to enumerate them rather than report "no loft": three of the four are a minute's
    typing on a page that already exists, and the fourth is a swing nobody uploaded. A count of
    bare refusals cannot tell those apart, and M15 P9 reported the whole corpus as one refusal
    when it was really two.
    """

    #: No swing on disk carries this shot's photo. Nothing to join to — the shot was imported from
    #: a screenshot on its own rather than arriving inside a swing bundle.
    NO_SWING = "no_swing"
    #: The swing is there and nobody said what it was hit with. The repair M9 built the club cursor
    #: for, and `POST /api/sessions/{s}/swings/{w}/club` after the fact.
    NO_CLUB_TAG = "no_club_tag"
    #: The swing names a club the golfer's bag has never declared. **Six shots on disk are here**,
    #: all `3w`. Look the club up on the bag page (M12) and it resolves.
    NO_BAG_ENTRY = "no_bag_entry"
    #: The bag entry exists and carries no loft. Five of this bag's six entries are here: M12's set
    #: lookup filled in makes and models, and only the 7 iron has ever been given a number.
    NO_DECLARED_LOFT = "no_declared_loft"


class FlightInputs(NamedTuple):
    """One stored shot with whatever the swing, the bag and the golfer register add to it."""

    shot: ShotData
    #: `"session/swing"`, or `None` when no swing on disk holds this photo.
    swing_ref: str | None
    player_id: str | None
    club: ClubId | None
    #: Ready to pass to `analysis.flight_infer.flight_for_shot`. Book loft off the bag entry, never
    #: a catalogue default and never inferred from the slot: a `3w` is a club name, not an angle.
    loft_deg: float | None
    handedness: Handedness | None
    #: `None` exactly when `loft_deg` is not.
    loft_gap: LoftGap | None
    #: Other swings holding the same photo, in the same arrival order the survivor was chosen by.
    #: Three swing directories on disk share one 2026-08-10 screenshot — the upload path was tested
    #: by re-sending a bundle — and a join that silently picked one of them would be choosing a
    #: club tag by luck.
    also_attached_to: tuple[str, ...] = ()


def read_flight_inputs(
    shots: Iterable[ShotData],
    *,
    sessions_dir: Path,
    golfers_dir: Path,
) -> list[FlightInputs]:
    """Join each shot to its swing, and through it to a loft and a handedness.

    Order is the caller's — `ShotStore.all()` is newest first and this preserves it — because the
    only thing a shot sorts by here is its own timestamp, and re-sorting would hide that.

    Missing directories are an empty join rather than an error: a shot with no swing beside it is a
    real state (the 2026-08-10 pair predate the bundle store) and `LoftGap.NO_SWING` is the answer,
    not an exception.
    """
    by_photo = _swings_by_shot_photo(SwingBundleStore(sessions_dir))
    golfers = GolferStore(golfers_dir)
    bags = BagStore(golfers_dir)
    # One bag read per golfer rather than one per shot: `BagStore.get` re-parses the file on every
    # call, and a session's worth of shots belongs to one or two people.
    lofts: dict[str, dict[ClubId, float | None]] = {}

    resolved: list[FlightInputs] = []
    for shot in shots:
        digest = shot.provenance.image_sha256 if shot.provenance is not None else None
        attached = by_photo.get(digest or "", ())
        if not attached:
            resolved.append(FlightInputs(shot, None, None, None, None, None, LoftGap.NO_SWING))
            continue

        # Earliest arrival wins, which is `storage/corpus.py`'s rule for the same situation stated
        # over the other artifact: a re-upload's timestamp dates the upload and not the swing, so
        # taking the latest would file a swing under the day someone retested the upload path.
        survivor, others = attached[0], attached[1:]
        player_id = survivor.player_id
        club = survivor.club
        golfer = golfers.get(player_id) if player_id else None

        if player_id is not None and player_id not in lofts:
            lofts[player_id] = _declared_lofts(bags, player_id)
        loft, gap = _resolve_loft(club, player_id, lofts.get(player_id or ""))

        resolved.append(
            FlightInputs(
                shot,
                _ref(survivor),
                player_id,
                club,
                loft,
                golfer.handedness if golfer is not None else None,
                gap,
                tuple(_ref(other) for other in others),
            )
        )
    return resolved


def loft_for_club(
    player_id: str | None,
    club: ClubId | None,
    *,
    golfers_dir: Path | None,
) -> tuple[float | None, LoftGap | None]:
    """One golfer's declared loft for one club slot, or which of the gaps it fell into. [M15 P11]

    The same rule `read_flight_inputs` applies, for the caller that has already done the join.
    `api/pipeline.py` is that caller: it is holding the manifest, so matching a shot photo's sha256
    against every swing on disk would be re-deriving what it already knows, and this is the half of
    the join it actually needs. M15 P17's `simulate_flight` is the third, and it is why
    `golfers_dir` is optional: an MCP server can be built with no golfer registry at all
    (`mcp.server.build_server`), which is a bag nobody can read rather than a bag with nothing in
    it — the same `LoftGap.NO_BAG_ENTRY` this already gives a club tag with nobody to own a bag.

    `LoftGap.NO_SWING` is the one value this cannot return — a caller here has a swing in hand.
    """
    entries = (
        _declared_lofts(BagStore(golfers_dir), player_id)
        if player_id and golfers_dir is not None
        else None
    )
    return _resolve_loft(club, player_id, entries)


def _declared_lofts(bags: BagStore, player_id: str) -> dict[ClubId, float | None]:
    """slot -> declared loft for one golfer, `{}` when there is no bag on disk.

    Read once per golfer by `read_flight_inputs`, because `BagStore.get` re-parses the file on
    every call and a session's worth of shots belongs to one or two people.
    """
    bag = bags.get(player_id)
    return {slot: entry.loft_deg for slot, entry in bag.entries.items()} if bag is not None else {}


def _resolve_loft(
    club: ClubId | None,
    player_id: str | None,
    entries: dict[ClubId, float | None] | None,
) -> tuple[float | None, LoftGap | None]:
    """The four-way branch itself, over data already in hand. No I/O, so both callers share it.

    **Book loft off the bag entry, never a catalogue default and never inferred from the slot**: a
    `3w` is a club name, not an angle, and the whole reason `LoftGap` enumerates four repairs is
    that a bare `None` sent M15 P9 to the bay for something that was a minute's typing on the bag
    page.
    """
    if club is None:
        return None, LoftGap.NO_CLUB_TAG
    if player_id is None or entries is None:
        # A club tag with nobody to own a bag. The tag is real and there is no bag to look it
        # up in, which is the same repair as an undeclared club and is reported as one.
        return None, LoftGap.NO_BAG_ENTRY
    if club not in entries:
        return None, LoftGap.NO_BAG_ENTRY
    if entries[club] is None:
        return None, LoftGap.NO_DECLARED_LOFT
    return entries[club], None


def _swings_by_shot_photo(store: SwingBundleStore) -> dict[str, tuple[SwingManifest, ...]]:
    """Every swing on disk, filed under the sha256 of the shot photo it arrived with.

    Built once rather than per shot: `get_session` parses every manifest in a session, and doing
    that thirteen times over eleven sessions is the same work eleven times over.
    """
    by_photo: dict[str, list[SwingManifest]] = {}
    for session_id in store.list_session_ids():
        for manifest in store.get_session(session_id):
            photo = manifest.roles.get(Role.SHOT_SCREEN)
            if photo is not None:
                by_photo.setdefault(photo.content_sha256, []).append(manifest)
    return {
        digest: tuple(sorted(members, key=lambda m: (m.created_at, m.session_id, m.swing_id)))
        for digest, members in by_photo.items()
    }


def _ref(manifest: SwingManifest) -> str:
    """`"session/swing"`, the reference every CLI in this repo prints and takes back."""
    return f"{manifest.session_id}/{manifest.swing_id}"
