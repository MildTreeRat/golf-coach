"""Golfer registry — one JSON file per golfer, keyed by `player_id`. [Career mode, step 1]

Flat files mirroring `launch_monitor/screen/store.py`: no shared index, no read-modify-write
across golfers, no SQLite. At the scale this runs at — a home lab with a handful of names, ever —
a directory listing *is* the index, and a bad record is fixed by deleting one file.

The load-bearing behaviour is `get_or_create`, and it is deliberately **read-biased**: a name that
already resolves to a known golfer returns the stored record untouched, including its handedness.
A career baseline is only as good as the join that assembles it, and the two ways this could go
wrong are both silent — a retyped name creating a second golfer (splitting one history into two
half-length ones), or a stray form submission flipping a stored handedness (inverting the sign of
every `head_hip_offset_impact_norm` comparison ever made against that baseline). Neither raises,
neither is visible on a status page, and both are discovered months later in a trend line.
Refusing to write over an existing record is what makes them impossible rather than unlikely.
"""

from __future__ import annotations

from datetime import UTC, datetime
from pathlib import Path

from golf_coach.contracts.golfer import Golfer, Handedness, slugify

_SUFFIX = ".golfer.json"


class GolferStore:
    """Directory of golfers, one JSON file each."""

    def __init__(self, root: Path) -> None:
        self._root = Path(root)

    @property
    def root(self) -> Path:
        return self._root

    def path_for(self, player_id: str) -> Path:
        return self._root / f"{player_id}{_SUFFIX}"

    def get(self, player_id: str) -> Golfer | None:
        """One golfer by id, or None if unknown.

        Tolerant like `manifest.load_manifest`: an unreadable or older-schema record reads as
        absent rather than raising. A corrupt golfer file should cost re-entering a name at the
        bay, not a 500 on the page someone is holding between swings.
        """
        path = self.path_for(player_id)
        try:
            return Golfer.model_validate_json(path.read_bytes())
        except (OSError, ValueError):
            return None

    def list_all(self) -> list[Golfer]:
        """Every known golfer, by display name. Unreadable records are skipped, not fatal."""
        if not self._root.exists():
            return []
        golfers = [
            golfer
            for path in sorted(self._root.glob(f"*{_SUFFIX}"))
            if (golfer := self.get(path.name[: -len(_SUFFIX)])) is not None
        ]
        return sorted(golfers, key=lambda g: g.display_name.lower())

    def get_or_create(self, name: str, handedness: Handedness) -> Golfer:
        """Resolve a typed name to a golfer, creating one only if the slug is genuinely new.

        `handedness` is used **only** when creating. For a known golfer the stored value wins and
        the argument is ignored — see the module docstring for why that direction is the safe one.

        Raises `ValueError` on a name that slugs to nothing, rather than inventing a placeholder
        id that would quietly collect every unnamed swing into one fictional golfer.
        """
        player_id = slugify(name)
        if not player_id:
            raise ValueError(f"name {name!r} has no usable characters for an id")

        existing = self.get(player_id)
        if existing is not None:
            return existing

        golfer = Golfer(
            player_id=player_id,
            display_name=name.strip(),
            handedness=handedness,
            created_at=datetime.now(tz=UTC),
        )
        self._root.mkdir(parents=True, exist_ok=True)
        path = self.path_for(player_id)
        tmp = path.with_suffix(".tmp")
        tmp.write_text(golfer.model_dump_json(indent=2), encoding="utf-8")
        tmp.replace(path)
        return golfer
