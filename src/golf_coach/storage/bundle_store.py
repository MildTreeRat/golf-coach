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
    ) -> AssignmentResult:
        """Slot an already-streamed-to-disk file into a swing. Must hold `_lock`."""
        with self._lock:
            session_dir = self._root / session_id
            manifests = self.get_session(session_id)

            if swing_id is not None:
                target = self.get_swing(session_id, swing_id) or _new_manifest(session_id, swing_id)
                return self._place(
                    session_dir, target, role, tmp_path, digest,
                    original_filename, content_type, size_bytes, deduped=False,
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
                    )

            candidate = next(
                (m for m in reversed(manifests) if role not in m.roles),
                None,
            )
            if candidate is None:
                candidate = _new_manifest(session_id, _next_swing_id(manifests))

            return self._place(
                session_dir, candidate, role, tmp_path, digest,
                original_filename, content_type, size_bytes, deduped=False,
            )

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
    ) -> AssignmentResult:
        swing_dir = session_dir / manifest.swing_id
        swing_dir.mkdir(parents=True, exist_ok=True)
        filename = content_filename(role, digest, original_filename)

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
        )


def _new_manifest(session_id: str, swing_id: str) -> SwingManifest:
    now = datetime.now(tz=UTC)
    return SwingManifest(swing_id=swing_id, session_id=session_id, created_at=now, updated_at=now)


def _next_swing_id(manifests: list[SwingManifest]) -> str:
    existing = [int(m.swing_id) for m in manifests if m.swing_id.isdigit()]
    return str(max(existing, default=0) + 1)


def _swing_sort_key(name: str) -> tuple[int, str]:
    """Numeric swing dirs sort numerically; anything else (e.g. `.incoming`) sorts after."""
    return (int(name), "") if name.isdigit() else (10**9, name)
