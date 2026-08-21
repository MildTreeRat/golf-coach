"""Declared bags — one JSON file per golfer, beside that golfer's own record. [M9 P3]

`data/processed/golfers/<player_id>.bag.json`, sharing a directory with
`<player_id>.golfer.json`. The suffix is what keeps them apart: `GolferStore.list_all` globs
`*.golfer.json` and this store globs nothing, so neither sees the other's files. A bag belongs to
exactly one golfer and is keyed by exactly the same id, so a second top-level directory would name
the same thing twice — and `golfer_store.py`'s reason for sitting beside `sessions/` rather than
inside it ("a golfer outlives any one session") applies to the bag unchanged.

Flat files, `GolferStore`'s shape throughout: tolerant reads, atomic writes, no index, no SQLite.

**The store owns the clock.** `BagEntry.recorded_at` is required and has no default because the
contract stays dumb and the writer stamps it — `Golfer.created_at` and `GolferStore` are the
precedent. So `set_entry` discards whatever `recorded_at` the caller passed, the same way
`get_or_create` discards the `handedness` argument for a golfer it already knows. One stamping
site, in the layer that knows when the write happened.

**Nothing here deletes a club.** Replacing or removing one moves the outgoing entry to
`Bag.retired`, which is append-only — see `contracts/bag.py` for why retention is not the bag entry
versioning ADR-024 defers.
"""

from __future__ import annotations

import os
from datetime import UTC, datetime
from pathlib import Path

from golf_coach.contracts.bag import Bag, BagEntry
from golf_coach.contracts.club import ClubId

_SUFFIX = ".bag.json"


class BagStore:
    """Directory of declared bags, one JSON file each."""

    def __init__(self, root: Path) -> None:
        self._root = Path(root)

    @property
    def root(self) -> Path:
        return self._root

    def path_for(self, player_id: str) -> Path:
        return self._root / f"{player_id}{_SUFFIX}"

    def get(self, player_id: str) -> Bag | None:
        """One golfer's bag, or None if they have not declared one.

        Tolerant like `GolferStore.get`: an unreadable or older-schema record reads as absent rather
        than raising. A corrupt bag should cost re-entering a loft, not a 500 on the page someone is
        holding between swings.

        Read-only callers get that leniency; the writers below deliberately do not — see
        `_load_for_write`.
        """
        try:
            return Bag.model_validate_json(self.path_for(player_id).read_bytes())
        except (OSError, ValueError):
            return None

    def save(self, bag: Bag) -> None:
        """Atomic write (tmp + `os.replace`), copied from `manifest.save_manifest`.

        Writes the bag as given and stamps nothing: `updated_at` belongs to whoever assembled it,
        which for every caller here is the method that also decided what changed.
        """
        path = self.path_for(bag.player_id)
        path.parent.mkdir(parents=True, exist_ok=True)
        tmp = path.with_suffix(".tmp")
        tmp.write_text(bag.model_dump_json(indent=2), encoding="utf-8")
        os.replace(tmp, path)

    def set_entry(self, player_id: str, entry: BagEntry) -> Bag:
        """Declare the club in one slot, retiring whatever was there.

        Three cases, and the middle one is the reason this is not a plain dict assignment:

        1. **Empty slot** — install, stamped now.
        2. **The same physical club** (`BagEntry.same_club_as`) — return untouched, *without
           writing*. Re-saving an unchanged row is an edit and not a bag change, and P19's
           `POST /api/golfers/{id}/bag/{club}` will do exactly that every time someone opens the
           page and saves without editing. Moving `recorded_at` there would hand P16 a bag-changed
           caveat over shots all hit with the same club.
        3. **A different club** — stamp the outgoing entry's `retired_at`, append it to the shelf,
           and install the new one.

        The caller's `recorded_at` is discarded in 1 and 3; see the module docstring.
        """
        now = datetime.now(tz=UTC)
        bag = self._load_for_write(player_id) or Bag(player_id=player_id, updated_at=now)

        current = bag.entries.get(entry.club)
        if current is not None and current.same_club_as(entry):
            return bag

        shelf = bag.retired
        if current is not None:
            shelf = (*shelf, current.model_copy(update={"retired_at": now}))

        live = entry.model_copy(update={"recorded_at": now, "retired_at": None})
        return self._write(bag, entries={**bag.entries, entry.club: live}, retired=shelf, at=now)

    def remove_entry(self, player_id: str, club: ClubId) -> Bag | None:
        """Take a club out of the bag, keeping its record. None if the slot was already empty.

        The removed entry lands on `Bag.retired` rather than being dropped, which is what
        `restore_entry` reads and what makes an accidental removal a one-call repair instead of a
        retyped loft. It is also the only record of *when* a club left when nothing replaces it.
        """
        bag = self._load_for_write(player_id)
        if bag is None or club not in bag.entries:
            return None

        now = datetime.now(tz=UTC)
        return self._write(
            bag,
            entries={slot: e for slot, e in bag.entries.items() if slot is not club},
            retired=(*bag.retired, bag.entries[club].model_copy(update={"retired_at": now})),
            at=now,
        )

    def restore_entry(self, player_id: str, club: ClubId) -> Bag | None:
        """Put back the club this slot held before the current one. None if it never held another.

        A **copy** comes off the shelf and the shelved stint stays where it is, so the history of a
        club that goes out and comes back is two stints rather than one overwritten record. It is
        also why this can route through `set_entry` rather than reimplementing the upsert: the
        restored club is just another declaration, and it gets a fresh `recorded_at` because it
        re-entered the bag today — P16's caveat is about when the physical club started hitting the
        shots being pooled, not about when the golfer first bought it.

        `retired_for` is ordered oldest-first, so `[-1]` is the immediately previous club. Reaching
        further back is `set_entry(player_id, bag.retired_for(club)[i])`, which needs nothing here.
        """
        bag = self._load_for_write(player_id)
        if bag is None:
            return None

        stints = bag.retired_for(club)
        if not stints:
            return None
        return self.set_entry(player_id, stints[-1])

    def _load_for_write(self, player_id: str) -> Bag | None:
        """Read before a write, distinguishing "no bag yet" from "bag I cannot read".

        `get` collapses those two into None, which is right for a reader and dangerous here: a
        writer that treats an unreadable file as an empty bag replaces the whole bag *and its entire
        shelf* with whatever single club it was asked to set. The shelf is the part that was meant
        to survive replacement, so the failure got worse the moment it existed.

        Raising is `GolferStore`'s posture — "refusing to write over an existing record is what
        makes them impossible rather than unlikely" — applied to the same class of silent loss.

        A `player_id` that is not a slug needs no check here: with no file at that path this returns
        None and the caller constructs a `Bag`, whose validator refuses it before `save` can turn it
        into a filename.
        """
        path = self.path_for(player_id)
        if not path.exists():
            return None
        try:
            return Bag.model_validate_json(path.read_bytes())
        except (OSError, ValueError) as exc:
            raise ValueError(
                f"bag at {path} exists but cannot be read; refusing to overwrite it"
            ) from exc

    def _write(
        self,
        bag: Bag,
        *,
        entries: dict[ClubId, BagEntry],
        retired: tuple[BagEntry, ...],
        at: datetime,
    ) -> Bag:
        """Rebuild, validate and save. Constructed rather than `model_copy`-ed on purpose.

        `model_copy(update=...)` skips validators, and both of `Bag`'s guard the invariants these
        three methods are the only thing maintaining — that a slot's key matches its entry, and that
        a retired entry never sits in `entries`. Building the model is what makes a mistake in this
        file fail here instead of on the next read.
        """
        rebuilt = Bag(player_id=bag.player_id, entries=entries, retired=retired, updated_at=at)
        self.save(rebuilt)
        return rebuilt
