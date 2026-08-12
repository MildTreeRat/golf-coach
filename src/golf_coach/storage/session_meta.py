"""`session.json` — which golfer is currently swinging. [Career mode, step 1]

**This is a cursor, not a record.** It says who the *next* swing should be attributed to. The
record of who actually swung lives on each `SwingManifest`, stamped when the swing is created and
never rewritten from here. Keeping the two apart is the whole reason a buddy can take a few swings
mid-session without rewriting the history of the golfer who was hitting before them.

Why the cursor is server-side at all, rather than each phone remembering a name the way it
remembers its role: two phones upload into the *same* swing. `bundle_store` already establishes
the principle — "two people holding two phones can't be trusted to type matching swing numbers" —
and a name typed twice on two phone keyboards is the same problem with worse consequences, since a
mismatch splits one golfer's baseline in two instead of misfiling one clip. Role is genuinely
per-phone and correctly lives in the phone's `localStorage`; the golfer is per-session and shared,
so exactly one copy exists and both phones read it.

A sidecar for the same reason `analysis.state.json` is one: it records something a human chose,
next to but not inside the manifest that records what arrived from a camera.
"""

from __future__ import annotations

import os
from datetime import UTC, datetime
from pathlib import Path

from pydantic import BaseModel

SESSION_META_NAME = "session.json"


class SessionMeta(BaseModel):
    """Session-scoped choices. Just the golfer cursor today."""

    player_id: str | None = None
    updated_at: datetime | None = None


def session_meta_path(session_dir: Path) -> Path:
    return session_dir / SESSION_META_NAME


def load_session_meta(session_dir: Path) -> SessionMeta:
    """The session's cursor, or an empty one.

    Returns `SessionMeta()` rather than `None` for a missing or corrupt file: "no golfer selected"
    is a real, expected state — it is every session's state until someone picks one — so callers
    should not have to distinguish it from "no file". Tolerant like `load_manifest`.
    """
    try:
        return SessionMeta.model_validate_json(session_meta_path(session_dir).read_bytes())
    except (OSError, ValueError):
        return SessionMeta()


def save_session_meta(meta: SessionMeta, session_dir: Path) -> None:
    """Write atomically — the status poll reads this while the golfer form rewrites it."""
    session_dir.mkdir(parents=True, exist_ok=True)
    path = session_meta_path(session_dir)
    tmp = path.with_suffix(".tmp")
    tmp.write_text(meta.model_dump_json(indent=2), encoding="utf-8")
    os.replace(tmp, path)


def set_current_player(session_dir: Path, player_id: str | None) -> SessionMeta:
    """Point the cursor at a golfer (or clear it) and persist. Returns the stored meta."""
    meta = SessionMeta(player_id=player_id, updated_at=datetime.now(tz=UTC))
    save_session_meta(meta, session_dir)
    return meta
