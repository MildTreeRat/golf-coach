"""Swing bundle store — role-based swing assignment, dedupe, and the documented
newest-wins limitation when two swings are simultaneously missing the same role.
"""

from __future__ import annotations

import hashlib
from datetime import UTC, datetime

import pytest

from golf_coach.storage.bundle_store import SwingBundleStore
from golf_coach.storage.manifest import Role

_SESSION = "2026-08-06"


def _upload(
    store: SwingBundleStore,
    session_id: str,
    role: Role,
    data: bytes,
    *,
    filename: str = "clip.mov",
    content_type: str = "video/quicktime",
    swing_id: str | None = None,
):
    incoming = store.root / ".incoming"
    incoming.mkdir(parents=True, exist_ok=True)
    digest = hashlib.sha256(data).hexdigest()
    scratch = incoming / f"{digest}-{role.value}.part"
    scratch.write_bytes(data)
    return store.assign_from_path(
        session_id=session_id,
        role=role,
        tmp_path=scratch,
        digest=digest,
        original_filename=filename,
        content_type=content_type,
        size_bytes=len(data),
        swing_id=swing_id,
    )


@pytest.fixture
def store(tmp_path):
    return SwingBundleStore(tmp_path)


def test_current_session_id_uses_injected_now(store) -> None:
    now = datetime(2026, 3, 5, 9, 30, tzinfo=UTC)

    assert store.current_session_id(now=now) == "2026-03-05"


def test_two_different_roles_arrive_into_the_same_swing(store) -> None:
    first = _upload(store, _SESSION, Role.FACE_ON, b"face-on-bytes")
    second = _upload(store, _SESSION, Role.DOWN_THE_LINE, b"dtl-bytes")

    assert first.swing_id == second.swing_id == "1"
    assert second.status == "collecting"
    assert second.missing_roles == [Role.SHOT_SCREEN]


def test_role_arrival_order_does_not_matter(store) -> None:
    _upload(store, _SESSION, Role.SHOT_SCREEN, b"screen")
    _upload(store, _SESSION, Role.DOWN_THE_LINE, b"dtl")
    result = _upload(store, _SESSION, Role.FACE_ON, b"face-on")

    assert result.swing_id == "1"
    assert result.status == "complete"


def test_second_swing_only_opens_once_first_is_complete(store) -> None:
    _upload(store, _SESSION, Role.FACE_ON, b"swing1-face")
    _upload(store, _SESSION, Role.DOWN_THE_LINE, b"swing1-dtl")
    _upload(store, _SESSION, Role.SHOT_SCREEN, b"swing1-screen")  # completes swing 1

    result = _upload(store, _SESSION, Role.FACE_ON, b"swing2-face")

    assert result.swing_id == "2"


def test_duplicate_bytes_dedupe_instead_of_opening_a_new_swing(store) -> None:
    data = b"same-bytes"
    first = _upload(store, _SESSION, Role.FACE_ON, data)
    second = _upload(store, _SESSION, Role.FACE_ON, data)

    assert second.deduped is True
    assert second.swing_id == first.swing_id
    assert len(store.get_session(_SESSION)) == 1


def test_different_bytes_reupload_of_a_filled_role_opens_a_new_swing(store) -> None:
    # Documented limitation: without an explicit swing_id, a second, different upload
    # of a role a swing already has is treated as the next swing, not a re-record.
    _upload(store, _SESSION, Role.FACE_ON, b"first-recording")

    result = _upload(store, _SESSION, Role.FACE_ON, b"different-recording")

    assert result.swing_id == "2"


def test_swing_id_repair_path_overwrites_the_original_slot(store) -> None:
    _upload(store, _SESSION, Role.FACE_ON, b"first-recording")

    result = _upload(store, _SESSION, Role.FACE_ON, b"corrected-recording", swing_id="1")

    assert result.swing_id == "1"
    manifest = store.get_swing(_SESSION, "1")
    assert manifest is not None
    assert manifest.roles[Role.FACE_ON].content_sha256 == hashlib.sha256(
        b"corrected-recording"
    ).hexdigest()
    stored_files = [p for p in (store.root / _SESSION / "1").iterdir() if p.name != "manifest.json"]
    assert len(stored_files) == 1


def test_newest_wins_when_two_swings_are_missing_the_same_role(store) -> None:
    _upload(store, _SESSION, Role.FACE_ON, b"swing1-face")
    _upload(store, _SESSION, Role.FACE_ON, b"swing2-face")  # opens swing 2, both miss dtl

    result = _upload(store, _SESSION, Role.DOWN_THE_LINE, b"dtl")

    assert result.swing_id == "2"


def test_corrupt_swing_dir_is_skipped_not_fatal(store) -> None:
    _upload(store, _SESSION, Role.FACE_ON, b"face-on")

    corrupt_dir = store.root / _SESSION / "corrupt"
    corrupt_dir.mkdir(parents=True)
    (corrupt_dir / "manifest.json").write_text("{not json", encoding="utf-8")

    assert [m.swing_id for m in store.get_session(_SESSION)] == ["1"]


def test_missing_session_directory_is_empty_not_an_error(store) -> None:
    assert store.get_session("no-such-session") == []
