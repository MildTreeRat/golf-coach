"""Parsed-shot cache — parse each image once, keep the result. [ADR-014]

OCR on a rectified 4K photo is slow enough that re-running an import over a session's
worth of screenshots would be painful, so parses are written out and keyed by the
image's SHA-256. Content addressing rather than filename means re-importing the same
photo under a different name is still a cache hit, and re-taking a photo of a *different*
shot never collides.

One JSON file per shot: a bad parse is deleted by deleting one file, and there is no
read-modify-write on a shared index to lose data to. This is deliberately a flat-file
store and not SQLite — the `storage/` module owns the database, and pulling that forward
would drag the rest of M3 into this change.
"""

from __future__ import annotations

import hashlib
import re
from pathlib import Path

from golf_coach.contracts.shot import ShotData

_SUFFIX = ".shot.json"
_UNSAFE = re.compile(r"[^A-Za-z0-9._-]+")


def hash_image(data: bytes) -> str:
    """Content hash of an image — the cache key for its parse."""
    return hashlib.sha256(data).hexdigest()


class ShotStore:
    """Directory of parsed shots, one JSON file each, keyed by source-image hash."""

    def __init__(self, root: Path) -> None:
        self._root = Path(root)

    @property
    def root(self) -> Path:
        return self._root

    def path_for(self, key: str) -> Path:
        return self._root / f"{_UNSAFE.sub('-', key)}{_SUFFIX}"

    def has(self, key: str) -> bool:
        return self.path_for(key).exists()

    def get(self, key: str) -> ShotData | None:
        path = self.path_for(key)
        if not path.exists():
            return None
        return ShotData.model_validate_json(path.read_bytes())

    def put(self, shot: ShotData) -> Path:
        """Write a shot, keyed by its source image hash (falling back to its id)."""
        self._root.mkdir(parents=True, exist_ok=True)
        path = self.path_for(self._key_for(shot))
        path.write_text(shot.model_dump_json(indent=2), encoding="utf-8")
        return path

    def all(self) -> list[ShotData]:
        """Every stored shot, newest first. Unreadable files are skipped, not fatal."""
        if not self._root.exists():
            return []
        shots: list[ShotData] = []
        for path in sorted(self._root.glob(f"*{_SUFFIX}")):
            try:
                shots.append(ShotData.model_validate_json(path.read_bytes()))
            except (ValueError, OSError):
                continue
        shots.sort(key=lambda s: s.timestamp, reverse=True)
        return shots

    @staticmethod
    def _key_for(shot: ShotData) -> str:
        if shot.provenance is not None and shot.provenance.image_sha256:
            return shot.provenance.image_sha256
        return shot.shot_id
