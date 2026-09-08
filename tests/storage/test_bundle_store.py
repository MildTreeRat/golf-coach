"""Swing bundle store — role-based swing assignment, dedupe, and the documented
newest-wins limitation when two swings are simultaneously missing the same role.
"""

from __future__ import annotations

import hashlib
from datetime import UTC, datetime

import pytest

from golf_coach.contracts.club import ClubId
from golf_coach.contracts.mishit import MishitVerdict
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
    player_id: str | None = None,
    club: ClubId | None = None,
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
        player_id=player_id,
        club=club,
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


# --------------------------------------------------------------------------- golfer attribution


def test_a_new_swing_is_stamped_with_the_current_golfer(store) -> None:
    result = _upload(store, _SESSION, Role.FACE_ON, b"face-on", player_id="aaron")

    assert result.player_id == "aaron"
    assert store.get_swing(_SESSION, "1").player_id == "aaron"


def test_uploading_with_no_golfer_selected_leaves_the_swing_unlabeled(store) -> None:
    """Uploads are never blocked on identity, so an anonymous swing is a supported state."""
    result = _upload(store, _SESSION, Role.FACE_ON, b"face-on")

    assert result.player_id is None
    assert store.get_swing(_SESSION, "1").player_id is None


def test_a_later_role_labels_a_swing_that_was_still_anonymous(store) -> None:
    """The two-phone case: phone A uploaded before a golfer was picked, phone B after."""
    _upload(store, _SESSION, Role.FACE_ON, b"face-on")

    second = _upload(store, _SESSION, Role.DOWN_THE_LINE, b"dtl", player_id="aaron")

    assert second.swing_id == "1"
    assert store.get_swing(_SESSION, "1").player_id == "aaron"


def test_an_attributed_swing_is_never_restamped_by_a_later_upload(store) -> None:
    """Handing the club over must not rewrite the previous golfer's swings."""
    _upload(store, _SESSION, Role.FACE_ON, b"face-on", player_id="aaron")

    _upload(store, _SESSION, Role.DOWN_THE_LINE, b"dtl", player_id="dave")

    assert store.get_swing(_SESSION, "1").player_id == "aaron"


def test_switching_golfer_stamps_only_subsequent_swings(store) -> None:
    _upload(store, _SESSION, Role.FACE_ON, b"aarons-swing", player_id="aaron")

    _upload(store, _SESSION, Role.FACE_ON, b"daves-swing", player_id="dave")

    assert store.get_swing(_SESSION, "1").player_id == "aaron"
    assert store.get_swing(_SESSION, "2").player_id == "dave"


def test_attribute_unlabeled_adopts_only_the_anonymous_swings(store) -> None:
    _upload(store, _SESSION, Role.FACE_ON, b"one")
    _upload(store, _SESSION, Role.FACE_ON, b"two", player_id="dave")
    _upload(store, _SESSION, Role.FACE_ON, b"three")

    changed = store.attribute_unlabeled(_SESSION, "aaron")

    assert changed == ["1", "3"]
    assert [m.player_id for m in store.get_session(_SESSION)] == ["aaron", "dave", "aaron"]


def test_attribute_unlabeled_is_idempotent(store) -> None:
    _upload(store, _SESSION, Role.FACE_ON, b"one")
    store.attribute_unlabeled(_SESSION, "aaron")

    assert store.attribute_unlabeled(_SESSION, "aaron") == []


def test_set_player_overwrites_one_swing_and_leaves_its_neighbours(store) -> None:
    """The explicit human-driven repair path — the one place an attribution is overwritten."""
    _upload(store, _SESSION, Role.FACE_ON, b"one", player_id="dave")
    _upload(store, _SESSION, Role.FACE_ON, b"two", player_id="dave")

    repaired = store.set_player(_SESSION, "1", "aaron")

    assert repaired.player_id == "aaron"
    assert store.get_swing(_SESSION, "2").player_id == "dave"


def test_set_player_on_a_missing_swing_returns_none(store) -> None:
    assert store.set_player(_SESSION, "99", "aaron") is None


# ------------------------------------------------------------------------------ club attribution


def test_a_new_swing_is_stamped_with_the_current_club(store) -> None:
    result = _upload(store, _SESSION, Role.FACE_ON, b"face-on", club=ClubId.SEVEN_IRON)

    assert result.club is ClubId.SEVEN_IRON
    assert store.get_swing(_SESSION, "1").club is ClubId.SEVEN_IRON


def test_a_tagged_swing_is_never_retagged_by_a_later_upload(store) -> None:
    """Reaching for the next club must not rewrite the shot hit with the last one."""
    _upload(store, _SESSION, Role.FACE_ON, b"face-on", club=ClubId.SEVEN_IRON)

    _upload(store, _SESSION, Role.DOWN_THE_LINE, b"dtl", club=ClubId.PITCHING_WEDGE)

    assert store.get_swing(_SESSION, "1").club is ClubId.SEVEN_IRON


def test_switching_club_tags_only_subsequent_swings(store) -> None:
    _upload(store, _SESSION, Role.FACE_ON, b"the-seven", club=ClubId.SEVEN_IRON)

    _upload(store, _SESSION, Role.FACE_ON, b"the-wedge", club=ClubId.PITCHING_WEDGE)

    assert store.get_swing(_SESSION, "1").club is ClubId.SEVEN_IRON
    assert store.get_swing(_SESSION, "2").club is ClubId.PITCHING_WEDGE


def test_a_later_role_tags_a_swing_that_was_still_untagged(store) -> None:
    """The `_place` stamp-if-empty branch: a swing written before the club field existed."""
    _upload(store, _SESSION, Role.FACE_ON, b"face-on")

    second = _upload(store, _SESSION, Role.DOWN_THE_LINE, b"dtl", club=ClubId.SEVEN_IRON)

    assert second.swing_id == "1"
    assert store.get_swing(_SESSION, "1").club is ClubId.SEVEN_IRON


def test_a_deduped_upload_reports_the_stored_club_not_the_requested_one(store) -> None:
    """The retry reads back what the swing says, so a stale cursor cannot appear to have won."""
    _upload(store, _SESSION, Role.FACE_ON, b"face-on", club=ClubId.SEVEN_IRON)

    retry = _upload(store, _SESSION, Role.FACE_ON, b"face-on", club=ClubId.PITCHING_WEDGE)

    assert retry.deduped is True
    assert retry.club is ClubId.SEVEN_IRON


def test_set_club_overwrites_one_swing_and_leaves_its_neighbours(store) -> None:
    """The club's only repair path — per swing, because nothing may reach backwards for it."""
    _upload(store, _SESSION, Role.FACE_ON, b"one", club=ClubId.PITCHING_WEDGE)
    _upload(store, _SESSION, Role.FACE_ON, b"two", club=ClubId.PITCHING_WEDGE)

    repaired = store.set_club(_SESSION, "1", ClubId.SEVEN_IRON)

    assert repaired.club is ClubId.SEVEN_IRON
    assert store.get_swing(_SESSION, "2").club is ClubId.PITCHING_WEDGE


def test_set_club_on_a_missing_swing_returns_none(store) -> None:
    assert store.set_club(_SESSION, "99", ClubId.SEVEN_IRON) is None


def test_set_club_leaves_the_golfer_alone(store) -> None:
    _upload(store, _SESSION, Role.FACE_ON, b"one", player_id="aaron", club=ClubId.PITCHING_WEDGE)

    repaired = store.set_club(_SESSION, "1", ClubId.SEVEN_IRON)

    assert repaired.player_id == "aaron"


def test_set_player_leaves_the_club_alone(store) -> None:
    """The other direction, written out separately: one repair must not undo the other."""
    _upload(store, _SESSION, Role.FACE_ON, b"one", player_id="dave", club=ClubId.SEVEN_IRON)

    repaired = store.set_player(_SESSION, "1", "aaron")

    assert repaired.club is ClubId.SEVEN_IRON


def test_attribute_unlabeled_stamps_the_golfer_and_touches_no_club(store) -> None:
    """There is deliberately no bulk backfill for the club, and the golfer's must not become one."""
    _upload(store, _SESSION, Role.FACE_ON, b"one")
    _upload(store, _SESSION, Role.FACE_ON, b"two", club=ClubId.SEVEN_IRON)

    store.attribute_unlabeled(_SESSION, "aaron")

    assert [m.player_id for m in store.get_session(_SESSION)] == ["aaron", "aaron"]
    assert [m.club for m in store.get_session(_SESSION)] == [None, ClubId.SEVEN_IRON]


def test_set_mishit_records_a_verdict_and_a_null_clears_it(store) -> None:
    """The explicit per-swing override, and reset back to automatic (ADR-028)."""
    _upload(store, _SESSION, Role.FACE_ON, b"one", club=ClubId.SEVEN_IRON)

    confirmed = store.set_mishit(_SESSION, "1", MishitVerdict.CONFIRMED)
    assert confirmed.mishit is MishitVerdict.CONFIRMED
    assert store.get_swing(_SESSION, "1").mishit is MishitVerdict.CONFIRMED

    reset = store.set_mishit(_SESSION, "1", None)
    assert reset.mishit is None
    assert store.get_swing(_SESSION, "1").mishit is None
    assert reset.club is ClubId.SEVEN_IRON  # the verdict is the only thing it touches


def test_set_mishit_on_a_missing_swing_returns_none(store) -> None:
    assert store.set_mishit(_SESSION, "99", MishitVerdict.CONFIRMED) is None


# ---------------------------------------------------------------- deleting a swing [phantoms]


def test_delete_swing_removes_the_directory(store) -> None:
    _upload(store, _SESSION, Role.FACE_ON, b"one", club=ClubId.SEVEN_IRON)

    assert store.delete_swing(_SESSION, "1") is True
    assert store.get_session(_SESSION) == []
    assert not (store.root / _SESSION / "1").exists()


def test_delete_swing_on_a_missing_swing_returns_false(store) -> None:
    """Same shape as `set_club` on a missing swing: a report, not an exception. The route turns
    it into a 404, and a double-tapped delete must not 500 a page held in a bay."""
    assert store.delete_swing(_SESSION, "99") is False


def test_delete_swing_takes_everything_in_the_directory(store) -> None:
    """Keypoints, analysis and the state sidecar are all derived from this swing alone, so they
    go with it — a state file left behind is a swing the status page renders with no files."""
    _upload(store, _SESSION, Role.FACE_ON, b"one", club=ClubId.SEVEN_IRON)
    swing_dir = store.root / _SESSION / "1"
    (swing_dir / "analysis.json").write_text("{}", encoding="utf-8")
    (swing_dir / "analysis.state.json").write_text("{}", encoding="utf-8")

    store.delete_swing(_SESSION, "1")

    assert not swing_dir.exists()


def test_deleting_a_phantom_stops_it_swallowing_the_next_swing(store) -> None:
    """Why this method exists, written as the failure it prevents.

    A corrective shot-screen re-upload opens a *new* swing, because "newest swing lacking that
    role" cannot tell a repair from the next shot. The phantom that results is missing both
    clips, so it is the newest swing lacking them — and the next real swing's face-on lands in
    it, paired with a shot screen from a different swing. Silently.
    """
    _upload(store, _SESSION, Role.FACE_ON, b"clip-1", club=ClubId.SEVEN_IRON)
    _upload(store, _SESSION, Role.SHOT_SCREEN, b"bad-photo", club=ClubId.SEVEN_IRON)
    # The correction. It cannot replace the bad photo, so it opens swing 2 holding only itself.
    phantom = _upload(store, _SESSION, Role.SHOT_SCREEN, b"good-photo", club=ClubId.SEVEN_IRON)
    assert phantom.swing_id == "2"

    # Left alone, the phantom takes the next swing's clip.
    swallowed = _upload(store, _SESSION, Role.FACE_ON, b"clip-2", club=ClubId.SEVEN_IRON)
    assert swallowed.swing_id == "2"

    store.delete_swing(_SESSION, "2")

    assert [m.swing_id for m in store.get_session(_SESSION)] == ["1"]


def test_swing_id_override_replaces_a_role_in_place(store) -> None:
    """The repair the upload page now sends. Without `swing_id` this same call opens a new swing;
    with it the bad file is overwritten and no phantom is created."""
    _upload(store, _SESSION, Role.SHOT_SCREEN, b"bad-photo", club=ClubId.SEVEN_IRON)

    result = _upload(
        store, _SESSION, Role.SHOT_SCREEN, b"good-photo",
        filename="shot.jpg", content_type="image/jpeg", swing_id="1",
    )

    assert result.swing_id == "1"
    assert [m.swing_id for m in store.get_session(_SESSION)] == ["1"]
    manifest = store.get_swing(_SESSION, "1")
    assert manifest.roles[Role.SHOT_SCREEN].original_filename == "shot.jpg"
    # The superseded file is gone, not merely unreferenced — `_place` unlinks it.
    names = {p.name for p in (store.root / _SESSION / "1").iterdir()}
    assert not any(name.endswith(".part") for name in names)
    assert len([n for n in names if n.startswith("shot_screen.")]) == 1
