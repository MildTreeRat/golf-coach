"""Swing bundle store — groups uploaded files into swings, keyed by role.

Swing identity is assigned by the store, not the uploader: two people holding two
phones can't be trusted to type matching swing numbers. Each upload declares only
its role; the store slots it into the newest swing in the session lacking that
role, opening a new one if none does. Content-addressed dedupe means a retried or
double-tapped upload never creates a phantom swing.

Known, documented limitation: if two swings are simultaneously missing the same
role, "newest wins" can attribute an out-of-order upload to the wrong swing. This
is not engineered around here — the status page surfaces incomplete swings
immediately, and the `swing_id` repair path (an explicit, human-driven override)
fixes a misattribution by hand. See `tests/storage/test_bundle_store.py` for the
pinned-down behavior.

Golfer attribution follows the same shape and for the same reason. `player_id` is
stamped from the session cursor (`storage.session_meta`) as files land, never sent
by the uploading phone — the two phones would have to type matching names, which is
the assumption this store already refuses to make about swing numbers. Stamping is
**write-once**: a swing that already names a golfer is never re-attributed by
anything here, so switching the cursor mid-session touches only swings that were
still unlabeled. Getting it wrong is repaired the same explicit human-driven way,
via `set_player`.

The club is stamped from the same cursor by the same rule, and differs from the
golfer in two ways worth knowing before reading the code. It is **required at
the boundary** where the golfer is not, because an untagged golfer is repairable
later and an untagged club is not recoverable by anything except memory. And it
has **no backfill counterpart**: `attribute_unlabeled` may reach backwards over
a session because a session usually has one golfer, whereas a session has many
clubs, so the same reach would confidently mislabel every earlier swing.
`set_club` is the whole of the club's repair story. Both asymmetries are argued
in ADR-024 §5 and restated where they bite, in
`storage.session_meta.set_current_club`.

Flat files, one manifest.json per swing, mirroring the pattern in
`launch_monitor/screen/store.py` — no shared index, no read-modify-write across
swings, no SQLite.
"""

from __future__ import annotations

import os
import threading
from dataclasses import dataclass
from datetime import UTC, datetime
from pathlib import Path

from golf_coach.contracts.club import ClubId
from golf_coach.storage.manifest import (
    Role,
    RoleFile,
    SwingManifest,
    content_filename,
    load_manifest,
    manifest_path,
    save_manifest,
)


@dataclass
class AssignmentResult:
    session_id: str
    swing_id: str
    role: Role
    status: str
    missing_roles: list[Role]
    deduped: bool
    player_id: str | None = None
    club: ClubId | None = None


class SwingBundleStore:
    """Sessions of swings, each swing a directory of role-tagged files + a manifest."""

    def __init__(self, root: Path) -> None:
        self._root = Path(root)
        self._lock = threading.Lock()

    @property
    def root(self) -> Path:
        return self._root

    def current_session_id(self, *, now: datetime | None = None) -> str:
        """Server-side date, never client-supplied — immune to phone clock skew."""
        return f"{(now or datetime.now(tz=UTC)):%Y-%m-%d}"

    def list_session_ids(self) -> list[str]:
        """Every session on disk, oldest first. Dotted dirs (`.incoming`) are not sessions.

        Session ids are dates, so a lexical sort is a chronological one — which is what lets a
        cross-session reader walk history in order without parsing the id (career mode, step 2).
        `2026-08-07-aaron1` sorts before `2026-08-09` on the same rule, the trailing name being
        the hand-rolled attribution that `player_id` replaced.
        """
        if not self._root.exists():
            return []
        return sorted(
            p.name for p in self._root.iterdir() if p.is_dir() and not p.name.startswith(".")
        )

    def get_session(self, session_id: str) -> list[SwingManifest]:
        """Every swing in the session, oldest first. Corrupt/unreadable dirs are skipped."""
        session_dir = self._root / session_id
        if not session_dir.exists():
            return []
        manifests: list[SwingManifest] = []
        for swing_dir in sorted(session_dir.iterdir(), key=lambda p: _swing_sort_key(p.name)):
            if not swing_dir.is_dir() or swing_dir.name.startswith("."):
                continue
            manifest = load_manifest(manifest_path(swing_dir))
            if manifest is not None:
                manifests.append(manifest)
        return manifests

    def get_swing(self, session_id: str, swing_id: str) -> SwingManifest | None:
        return load_manifest(manifest_path(self._root / session_id / swing_id))

    def assign_from_path(
        self,
        *,
        session_id: str,
        role: Role,
        tmp_path: Path,
        digest: str,
        original_filename: str,
        content_type: str,
        size_bytes: int,
        swing_id: str | None = None,
        player_id: str | None = None,
        club: ClubId | None = None,
    ) -> AssignmentResult:
        """Slot an already-streamed-to-disk file into a swing. Must hold `_lock`."""
        with self._lock:
            session_dir = self._root / session_id
            manifests = self.get_session(session_id)

            if swing_id is not None:
                target = self.get_swing(session_id, swing_id) or _new_manifest(
                    session_id, swing_id, player_id, club
                )
                return self._place(
                    session_dir, target, role, tmp_path, digest,
                    original_filename, content_type, size_bytes,
                    deduped=False, player_id=player_id, club=club,
                )

            for manifest in manifests:
                existing = manifest.roles.get(role)
                if existing is not None and existing.content_sha256 == digest:
                    tmp_path.unlink(missing_ok=True)
                    return AssignmentResult(
                        session_id=session_id,
                        swing_id=manifest.swing_id,
                        role=role,
                        status=manifest.status(),
                        missing_roles=manifest.missing_roles(),
                        deduped=True,
                        player_id=manifest.player_id,
                        club=manifest.club,
                    )

            candidate = next(
                (m for m in reversed(manifests) if role not in m.roles),
                None,
            )
            if candidate is None:
                candidate = _new_manifest(session_id, _next_swing_id(manifests), player_id, club)

            return self._place(
                session_dir, candidate, role, tmp_path, digest,
                original_filename, content_type, size_bytes,
                deduped=False, player_id=player_id, club=club,
            )

    def attribute_unlabeled(self, session_id: str, player_id: str) -> list[str]:
        """Stamp every still-unlabeled swing in the session. Returns the swing ids changed.

        What makes "never block uploads" safe. Files land whether or not anyone has picked a
        golfer yet, so selecting one has to reach backwards over the swings that already arrived
        — otherwise the cost of forgetting is a permanently anonymous swing rather than a moment's
        inattention.

        Reaches backwards over *unlabeled* swings only. A swing that already names someone is
        another golfer's, and re-attributing it would be this method quietly undoing the record it
        exists to protect.
        """
        with self._lock:
            changed: list[str] = []
            for manifest in self.get_session(session_id):
                if manifest.player_id is not None:
                    continue
                manifest.player_id = player_id
                manifest.updated_at = datetime.now(tz=UTC)
                save_manifest(manifest, manifest_path(self._root / session_id / manifest.swing_id))
                changed.append(manifest.swing_id)
            return changed

    def set_player(self, session_id: str, swing_id: str, player_id: str) -> SwingManifest | None:
        """Re-attribute one swing, overwriting whatever it said. The repair path.

        The only thing here that overwrites an existing `player_id`, and it is deliberately the
        explicit human-driven override — same role the `swing_id` override plays for a
        misattributed upload. Returns None if there is no such swing.
        """
        with self._lock:
            swing_dir = self._root / session_id / swing_id
            manifest = load_manifest(manifest_path(swing_dir))
            if manifest is None:
                return None
            manifest.player_id = player_id
            manifest.updated_at = datetime.now(tz=UTC)
            save_manifest(manifest, manifest_path(swing_dir))
            return manifest

    def set_club(self, session_id: str, swing_id: str, club: ClubId) -> SwingManifest | None:
        """Retag one swing, overwriting whatever it said. The club's **only** repair path.

        Per-swing and nothing else, deliberately. `attribute_unlabeled`'s docstring explains why
        reaching backwards over a session is safe for the golfer; a session has many clubs, so the
        same reach would confidently mislabel every earlier swing (ADR-024 §5). Nothing but memory
        can say which club hit swing 3, so nothing but a human naming it should.

        Otherwise the same shape as `set_player`: the explicit human-driven override, and the one
        place a club already on a manifest is overwritten. Returns None if there is no such swing.
        """
        with self._lock:
            swing_dir = self._root / session_id / swing_id
            manifest = load_manifest(manifest_path(swing_dir))
            if manifest is None:
                return None
            manifest.club = club
            manifest.updated_at = datetime.now(tz=UTC)
            save_manifest(manifest, manifest_path(swing_dir))
            return manifest

    def _place(
        self,
        session_dir: Path,
        manifest: SwingManifest,
        role: Role,
        tmp_path: Path,
        digest: str,
        original_filename: str,
        content_type: str,
        size_bytes: int,
        *,
        deduped: bool,
        player_id: str | None = None,
        club: ClubId | None = None,
    ) -> AssignmentResult:
        swing_dir = session_dir / manifest.swing_id
        swing_dir.mkdir(parents=True, exist_ok=True)
        filename = content_filename(role, digest, original_filename)

        # Stamp-if-empty. Covers the ordinary case (the swing was created by this very upload) and
        # the one that only shows up with two phones: the face-on phone uploaded before anyone had
        # selected a golfer, the down-the-line phone uploads after, and the swing gets attributed
        # on the second file rather than staying anonymous because of which phone was faster.
        if manifest.player_id is None and player_id is not None:
            manifest.player_id = player_id

        # The club stamps by the same rule, but the rule carries less weight for it: the upload
        # route refuses a swing with no club selected, so a swing created since M9 cannot reach
        # here untagged and this branch cannot fire for it. What it still covers is a swing written
        # before the field existed receiving a later role — and tagging that from the cursor
        # current *now* is right, because now is when the swing is being completed. Write-once
        # either way: reaching for the next club must not retag the shots hit with the last one.
        if manifest.club is None and club is not None:
            manifest.club = club

        old = manifest.roles.get(role)
        if old is not None and old.filename != filename:
            old_path = swing_dir / old.filename
            if old_path.exists():
                old_path.unlink()

        dest = swing_dir / filename
        os.replace(tmp_path, dest)

        now = datetime.now(tz=UTC)
        manifest.roles[role] = RoleFile(
            role=role,
            filename=filename,
            content_sha256=digest,
            original_filename=original_filename,
            content_type=content_type,
            size_bytes=size_bytes,
            received_at=now,
        )
        manifest.updated_at = now
        save_manifest(manifest, manifest_path(swing_dir))

        return AssignmentResult(
            session_id=manifest.session_id,
            swing_id=manifest.swing_id,
            role=role,
            status=manifest.status(),
            missing_roles=manifest.missing_roles(),
            deduped=deduped,
            player_id=manifest.player_id,
            club=manifest.club,
        )


def _new_manifest(
    session_id: str,
    swing_id: str,
    player_id: str | None = None,
    club: ClubId | None = None,
) -> SwingManifest:
    now = datetime.now(tz=UTC)
    return SwingManifest(
        swing_id=swing_id,
        session_id=session_id,
        created_at=now,
        updated_at=now,
        player_id=player_id,
        club=club,
    )


def _next_swing_id(manifests: list[SwingManifest]) -> str:
    existing = [int(m.swing_id) for m in manifests if m.swing_id.isdigit()]
    return str(max(existing, default=0) + 1)


def _swing_sort_key(name: str) -> tuple[int, str]:
    """Numeric swing dirs sort numerically; anything else (e.g. `.incoming`) sorts after."""
    return (int(name), "") if name.isdigit() else (10**9, name)
