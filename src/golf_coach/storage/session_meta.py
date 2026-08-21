"""`session.json` — the session's cursors: who is swinging, and with what.
[Career mode, step 1; club added M9 P4]

**These are cursors, not records.** They say who the *next* swing should be attributed to and
which club it should be tagged with. The record of what actually happened lives on each
`SwingManifest`, stamped when the swing is created and never rewritten from here. Keeping the two
apart is the whole reason a buddy can take a few swings mid-session without rewriting the history
of the golfer who was hitting before them — and, since M9, the reason reaching for the next club
does not retag the shots hit with the last one.

Why the cursor is server-side at all, rather than each phone remembering a name the way it
remembers its role: two phones upload into the *same* swing. `bundle_store` already establishes
the principle — "two people holding two phones can't be trusted to type matching swing numbers" —
and a name typed twice on two phone keyboards is the same problem with worse consequences, since a
mismatch splits one golfer's baseline in two instead of misfiling one clip. Role is genuinely
per-phone and correctly lives in the phone's `localStorage`; the golfer is per-session and shared,
so exactly one copy exists and both phones read it. ADR-024 §5 quotes that argument to justify
reading the club from here too: two phones holding two `localStorage` club values would disagree,
and the disagreement would land in the manifest.

A sidecar for the same reason `analysis.state.json` is one: it records something a human chose,
next to but not inside the manifest that records what arrived from a camera.
"""

from __future__ import annotations

import os
from datetime import UTC, datetime
from pathlib import Path

from pydantic import BaseModel

from golf_coach.contracts.club import ClubId

SESSION_META_NAME = "session.json"


class SessionMeta(BaseModel):
    """Session-scoped choices: the golfer cursor and the club cursor.

    The two are independent — a golfer swings many clubs and a club is swung by many golfers — and
    keeping them independent is what `_update_cursor` exists to guarantee.

    `updated_at` is **one timestamp for the whole record**, not one per cursor: it says when the
    session's choices last changed, and nothing reads it today. It therefore cannot answer "when
    was the club chosen", so a surface wanting that (P19) needs a field, not this one.
    """

    player_id: str | None = None
    club: ClubId | None = None
    updated_at: datetime | None = None


def session_meta_path(session_dir: Path) -> Path:
    return session_dir / SESSION_META_NAME


def load_session_meta(session_dir: Path) -> SessionMeta:
    """The session's cursors, or an empty set of them.

    Returns `SessionMeta()` rather than `None` for a missing or corrupt file: "nothing selected
    yet" is a real, expected state — it is every session's state until someone picks — so callers
    should not have to distinguish it from "no file". Tolerant like `load_manifest`.

    Tolerant here does **not** soften the club's requiredness: this returns `club=None` and the
    upload route turns that into a 409 (P6). Refusing is the caller's job, not the reader's.
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


def _update_cursor(session_dir: Path, **changes: object) -> SessionMeta:
    """Change the named cursors, preserve every field not named, and persist.

    Load-modify-save, in one place, because the alternative is the bug this function was written
    to fix. Until M9 there was one cursor, so `set_current_player` could construct a whole fresh
    `SessionMeta` and lose nothing. With a second cursor that same line silently clears the club
    every time someone picks a golfer. Writing load-modify-save out in each setter would fix it
    today and re-introduce it the day a third cursor arrives and one setter forgets to carry it —
    so the preservation lives here, where a new cursor gets it without asking.

    `model_validate` on a merged dump rather than `model_copy(update=...)`: `model_copy` skips
    validators, which `storage/bag_store.py:_write` already rejected on the same ground. There are
    no validators on `SessionMeta` today; the point is that one added later still runs.
    """
    stored = load_session_meta(session_dir)
    meta = SessionMeta.model_validate(
        {**stored.model_dump(), **changes, "updated_at": datetime.now(tz=UTC)}
    )
    save_session_meta(meta, session_dir)
    return meta


def set_current_player(session_dir: Path, player_id: str | None) -> SessionMeta:
    """Point the cursor at a golfer (or clear it) and persist. Returns the stored meta."""
    return _update_cursor(session_dir, player_id=player_id)


def set_current_club(session_dir: Path, club: ClubId | None) -> SessionMeta:
    """Point the cursor at a club (or clear it) and persist. Returns the stored meta.

    No backfill counterpart, deliberately. `bundle_store.attribute_unlabeled` reaches backwards
    over a session's unlabeled swings because a session usually has one golfer; a session has many
    clubs, so the same reach would confidently mislabel every earlier swing (ADR-024 §5). The club
    gets a per-swing repair route instead.
    """
    return _update_cursor(session_dir, club=club)
