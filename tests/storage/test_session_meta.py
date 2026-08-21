"""The session's cursors — tolerance, round trips, and the independence of the two.

Moved out of `test_golfer_store.py` at M9 P4, when `session.json` stopped being about golfers
alone. `tests/` mirrors `src/golf_coach/` package by package, and club-cursor tests in a file
named for the golfer store is the kind of misfile that makes the next person grep twice.

The property most worth pinning here is the *independence* of the two cursors. Setting one used to
replace the whole record, so setting a club would have cleared the golfer — silently, on disk,
mid-session, and only visible later as a swing attributed to nobody.
"""

from __future__ import annotations

import pytest

from golf_coach.contracts.club import ClubId
from golf_coach.storage.session_meta import (
    SessionMeta,
    load_session_meta,
    save_session_meta,
    session_meta_path,
    set_current_club,
    set_current_player,
)

# --------------------------------------------------------------------------- tolerant reads


def test_missing_session_meta_reads_as_nothing_selected(tmp_path) -> None:
    """"Nobody selected yet" is every session's starting state, not an error."""
    meta = load_session_meta(tmp_path)

    assert meta.player_id is None
    assert meta.club is None


def test_corrupt_session_meta_reads_as_nothing_selected(tmp_path) -> None:
    session_meta_path(tmp_path).write_text("{not json", encoding="utf-8")

    meta = load_session_meta(tmp_path)

    assert meta.player_id is None
    assert meta.club is None


def test_a_session_written_before_club_existed_still_loads(tmp_path) -> None:
    """Written as a literal, for the reason the manifest's twin of this test states."""
    session_meta_path(tmp_path).write_text(
        '{"player_id": "aaron", "updated_at": "2026-08-12T12:00:00Z"}', encoding="utf-8"
    )

    meta = load_session_meta(tmp_path)

    assert meta.player_id == "aaron"
    assert meta.club is None


# --------------------------------------------------------------------------- golfer cursor


def test_session_cursor_round_trips(tmp_path) -> None:
    session_dir = tmp_path / "2026-08-12"

    stored = set_current_player(session_dir, "aaron")

    assert stored.player_id == "aaron"
    assert load_session_meta(session_dir).player_id == "aaron"


def test_session_cursor_can_be_repointed(tmp_path) -> None:
    session_dir = tmp_path / "2026-08-12"
    set_current_player(session_dir, "aaron")

    set_current_player(session_dir, "dave")

    assert load_session_meta(session_dir).player_id == "dave"


def test_saving_meta_creates_the_session_directory(tmp_path) -> None:
    session_dir = tmp_path / "2026-08-12"

    save_session_meta(SessionMeta(player_id="aaron"), session_dir)

    assert session_meta_path(session_dir).exists()


# --------------------------------------------------------------------------- club cursor


def test_club_cursor_round_trips(tmp_path) -> None:
    session_dir = tmp_path / "2026-08-12"

    stored = set_current_club(session_dir, ClubId.SEVEN_IRON)

    assert stored.club is ClubId.SEVEN_IRON
    assert load_session_meta(session_dir).club is ClubId.SEVEN_IRON


def test_club_cursor_can_be_repointed(tmp_path) -> None:
    """The ordinary bay motion: hit some 7 irons, then reach for the wedge."""
    session_dir = tmp_path / "2026-08-12"
    set_current_club(session_dir, ClubId.SEVEN_IRON)

    set_current_club(session_dir, ClubId.SAND_WEDGE)

    assert load_session_meta(session_dir).club is ClubId.SAND_WEDGE


def test_club_cursor_survives_a_round_trip_through_disk_as_an_enum(tmp_path) -> None:
    """`ClubId` is a `StrEnum`, so a string would compare equal and hide a broken parse."""
    session_dir = tmp_path / "2026-08-12"
    set_current_club(session_dir, ClubId.THREE_WOOD)

    loaded = load_session_meta(session_dir).club

    assert isinstance(loaded, ClubId)


# --------------------------------------------------------------------------- independence


def test_setting_the_club_preserves_the_golfer(tmp_path) -> None:
    """The regression P4 exists to prevent. Setting one cursor used to replace the whole record."""
    session_dir = tmp_path / "2026-08-12"
    set_current_player(session_dir, "aaron")

    set_current_club(session_dir, ClubId.SEVEN_IRON)

    meta = load_session_meta(session_dir)
    assert meta.player_id == "aaron"
    assert meta.club is ClubId.SEVEN_IRON


def test_setting_the_golfer_preserves_the_club(tmp_path) -> None:
    """The same property from the other side.

    Written out rather than folded into the test above, even though one helper serves both setters
    today: the next cursor added here may well be hand-rolled, and a hand-rolled setter breaks
    exactly one direction of this.
    """
    session_dir = tmp_path / "2026-08-12"
    set_current_club(session_dir, ClubId.SEVEN_IRON)

    set_current_player(session_dir, "aaron")

    meta = load_session_meta(session_dir)
    assert meta.club is ClubId.SEVEN_IRON
    assert meta.player_id == "aaron"


@pytest.mark.parametrize(
    ("clear", "survivor"),
    [
        (lambda d: set_current_club(d, None), "player_id"),
        (lambda d: set_current_player(d, None), "club"),
    ],
)
def test_clearing_one_cursor_leaves_the_other_alone(tmp_path, clear, survivor) -> None:
    """Clearing is a repoint at `None`, not a reset — passing `None` must not widen its blast."""
    session_dir = tmp_path / "2026-08-12"
    set_current_player(session_dir, "aaron")
    set_current_club(session_dir, ClubId.SEVEN_IRON)

    clear(session_dir)

    assert getattr(load_session_meta(session_dir), survivor) is not None


def test_both_cursors_reach_disk_not_just_the_model(tmp_path) -> None:
    """Reads go through the same loader that writes, so the file itself is worth one assertion."""
    session_dir = tmp_path / "2026-08-12"
    set_current_player(session_dir, "aaron")
    set_current_club(session_dir, ClubId.SEVEN_IRON)

    written = session_meta_path(session_dir).read_text(encoding="utf-8")

    assert '"aaron"' in written
    assert '"7i"' in written


def test_setting_a_cursor_stamps_updated_at(tmp_path) -> None:
    session_dir = tmp_path / "2026-08-12"

    stored = set_current_club(session_dir, ClubId.SEVEN_IRON)

    assert stored.updated_at is not None
