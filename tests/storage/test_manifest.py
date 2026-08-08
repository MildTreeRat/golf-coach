"""Manifest model — status derivation, atomic round trip, tolerant loading, content hashing."""

from __future__ import annotations

import hashlib
from datetime import UTC, datetime

from golf_coach.storage.manifest import (
    Role,
    RoleFile,
    SwingManifest,
    hash_bytes,
    hash_file,
    load_manifest,
    manifest_path,
    save_manifest,
)

_NOW = datetime(2026, 8, 6, 12, 0, tzinfo=UTC)


def _role_file(role: Role, digest: str = "abc") -> RoleFile:
    return RoleFile(
        role=role,
        filename=f"{role.value}.{digest}.mov",
        content_sha256=digest,
        original_filename="clip.mov",
        content_type="video/quicktime",
        size_bytes=1024,
        received_at=_NOW,
    )


def _manifest(**roles: RoleFile) -> SwingManifest:
    return SwingManifest(
        swing_id="1", session_id="2026-08-06", created_at=_NOW, updated_at=_NOW, roles=roles
    )


def test_status_is_collecting_until_all_roles_present() -> None:
    manifest = _manifest(face_on=_role_file(Role.FACE_ON))

    assert manifest.status() == "collecting"
    assert manifest.missing_roles() == [Role.DOWN_THE_LINE, Role.SHOT_SCREEN]


def test_status_is_complete_once_all_three_roles_present() -> None:
    manifest = _manifest(
        face_on=_role_file(Role.FACE_ON),
        down_the_line=_role_file(Role.DOWN_THE_LINE),
        shot_screen=_role_file(Role.SHOT_SCREEN),
    )

    assert manifest.status() == "complete"
    assert manifest.missing_roles() == []


def test_manifest_round_trips_through_disk(tmp_path) -> None:
    manifest = _manifest(face_on=_role_file(Role.FACE_ON))
    path = manifest_path(tmp_path)

    save_manifest(manifest, path)
    restored = load_manifest(path)

    assert restored is not None
    assert restored.swing_id == "1"
    assert restored.roles[Role.FACE_ON].content_sha256 == "abc"


def test_missing_manifest_returns_none_not_an_error(tmp_path) -> None:
    assert load_manifest(manifest_path(tmp_path)) is None


def test_corrupt_manifest_returns_none_not_an_error(tmp_path) -> None:
    path = manifest_path(tmp_path)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("{not json", encoding="utf-8")

    assert load_manifest(path) is None


def test_hash_file_matches_hashing_the_bytes_in_one_go(tmp_path) -> None:
    """Chunked or not, it must be the same digest — a clip hashed on upload and again on
    disk has to dedupe against itself."""
    path = tmp_path / "clip.mov"
    data = bytes(range(256)) * 40  # 10 KB, spanning several chunks below
    path.write_bytes(data)

    assert hash_file(path, chunk_bytes=1024) == hash_bytes(data)
    assert hash_file(path) == hashlib.sha256(data).hexdigest()


def test_hash_file_is_chunk_size_invariant(tmp_path) -> None:
    path = tmp_path / "clip.mov"
    path.write_bytes(b"x" * 5000 + b"y" * 3)

    digests = {hash_file(path, chunk_bytes=size) for size in (1, 7, 4096, 1 << 20)}

    assert len(digests) == 1


def test_hash_file_handles_an_empty_file(tmp_path) -> None:
    path = tmp_path / "empty.mov"
    path.write_bytes(b"")

    assert hash_file(path) == hash_bytes(b"")
