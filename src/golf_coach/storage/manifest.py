"""Swing bundle manifest — the record of which files have arrived for one swing.

A swing is complete once all three roles (face-on video, down-the-line video, shot-
screen photo) have arrived; until then it is still useful in a `collecting` state.
`status()` is deliberately computed, not persisted: this phase has nothing that
consumes a `ready`/`analyzed` signal, so persisting one would claim more than is
true. When an analysis worker is built later it can add an `analyzed_at` field as a
pure addition, with no migration of manifests written by this code.

`player_id` was added exactly that way (career mode, step 1): optional, defaulted,
and read through the same tolerant loader, so the four manifests written before it
existed still load and simply report `None`. `club` (M9 P4) is the second field
added under that pattern, and the pattern is the reason neither addition needed a
migration: a new optional field is a pure addition, so every manifest already on
disk keeps loading and answers `None`.
"""

from __future__ import annotations

import hashlib
import os
from datetime import datetime
from enum import StrEnum
from pathlib import Path
from typing import Literal

from pydantic import BaseModel, Field

from golf_coach.contracts.club import ClubId


class Role(StrEnum):
    """Which camera angle or document an uploaded file represents."""

    FACE_ON = "face_on"
    DOWN_THE_LINE = "down_the_line"
    SHOT_SCREEN = "shot_screen"


EXPECTED_ROLES: tuple[Role, ...] = (Role.FACE_ON, Role.DOWN_THE_LINE, Role.SHOT_SCREEN)

_DEFAULT_SUFFIX: dict[Role, str] = {
    Role.FACE_ON: ".mov",
    Role.DOWN_THE_LINE: ".mov",
    Role.SHOT_SCREEN: ".jpg",
}

_MANIFEST_NAME = "manifest.json"


class RoleFile(BaseModel):
    """One uploaded file, filling one role slot in a swing."""

    role: Role
    filename: str  # relative to the swing directory
    content_sha256: str
    original_filename: str
    content_type: str
    size_bytes: int
    received_at: datetime
    warnings: list[str] = Field(default_factory=list)


class SwingManifest(BaseModel):
    """Everything known about one swing: identity plus whichever roles have arrived."""

    swing_id: str
    session_id: str
    created_at: datetime
    updated_at: datetime
    roles: dict[Role, RoleFile] = Field(default_factory=dict)

    player_id: str | None = Field(
        default=None,
        description=(
            "Who swung it, stamped from the session cursor when the swing was created "
            "(`storage.session_meta`). None means nobody had selected a golfer yet — an expected "
            "state, since uploads are never blocked on it, and one the upload page surfaces so it "
            "can be repaired while the session is still happening. **Write-once in practice**: "
            "the store sets it only when it is None, so switching the cursor to a second golfer "
            "mid-session cannot rewrite swings already attributed to the first."
        ),
    )

    club: ClubId | None = Field(
        default=None,
        description=(
            "Which club hit it, stamped from the session cursor when the swing was created "
            "(`storage.session_meta`), exactly as `player_id` is. **Write-once in practice** for "
            "the same reason: switching the cursor to the next club mid-session must not rewrite "
            "the swings already hit with the last one. None means the swing predates the field — "
            "and unlike `player_id`, that is the *only* thing it means, because the upload route "
            "refuses a swing with no club selected (ADR-024 §5). The asymmetry is repairability: "
            "an untagged golfer can be reached backwards over a session that usually had one "
            "golfer, whereas a session has many clubs and nothing but memory can say which hit "
            "what. Optional in the shape, required at the boundary (R12) — which is what lets "
            "every manifest written before M9 keep loading."
        ),
    )

    def missing_roles(self) -> list[Role]:
        return [role for role in EXPECTED_ROLES if role not in self.roles]

    def status(self) -> Literal["collecting", "complete"]:
        return "complete" if not self.missing_roles() else "collecting"


def hash_bytes(data: bytes) -> str:
    """Content hash of a file's bytes — the identity used for dedupe."""
    return hashlib.sha256(data).hexdigest()


def hash_file(path: Path, *, chunk_bytes: int = 1 << 20) -> str:
    """The same hash as `hash_bytes`, read in chunks so a 200 MB clip never lands in memory.

    Mirrors the streaming digest the upload route already builds as bytes arrive
    (`api/app.py`) — same algorithm, same hex digest, so the two are interchangeable.
    """
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        while chunk := handle.read(chunk_bytes):
            digest.update(chunk)
    return digest.hexdigest()


def content_filename(role: Role, sha256: str, original_filename: str) -> str:
    """A content-addressed filename for a role slot, e.g. `face_on.deadbeef1234.mov`."""
    suffix = Path(original_filename).suffix or _DEFAULT_SUFFIX[role]
    return f"{role.value}.{sha256[:12]}{suffix}"


def manifest_path(swing_dir: Path) -> Path:
    return swing_dir / _MANIFEST_NAME


def load_manifest(path: Path) -> SwingManifest | None:
    """Tolerant read: a missing or corrupt manifest is `None`, never an exception."""
    if not path.exists():
        return None
    try:
        return SwingManifest.model_validate_json(path.read_bytes())
    except (ValueError, OSError):
        return None


def save_manifest(manifest: SwingManifest, path: Path) -> None:
    """Atomic write (tmp + replace) — safe against a concurrent status-page read."""
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp_path = path.with_suffix(".tmp")
    tmp_path.write_text(manifest.model_dump_json(indent=2), encoding="utf-8")
    os.replace(tmp_path, path)
