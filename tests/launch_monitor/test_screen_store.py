"""The parsed-shot cache and the source that reads it back.

Content addressing is the point: importing the same photo twice must not re-pay the OCR,
and re-importing it under a different filename must still be recognized as the same shot.
"""

from __future__ import annotations

from datetime import UTC, datetime, timedelta

from golf_coach.contracts.shot import ShotData, ShotProvenance, ShotSource
from golf_coach.launch_monitor.screen.source import ScreenShotDataSource
from golf_coach.launch_monitor.screen.store import ShotStore, hash_image


def _shot(
    shot_id: str,
    *,
    digest: str,
    minutes: int = 0,
    session: str = "range",
    needs_review: bool = False,
) -> ShotData:
    return ShotData(
        shot_id=shot_id,
        session_id=session,
        timestamp=datetime(2026, 8, 4, 12, 0, tzinfo=UTC) + timedelta(minutes=minutes),
        source=ShotSource.SCREEN,
        carry_distance=128.1,
        provenance=ShotProvenance(
            device="hd_golf",
            parse_confidence=0.95,
            needs_review=needs_review,
            image_sha256=digest,
        ),
    )


def test_hash_is_content_addressed_not_name_addressed() -> None:
    assert hash_image(b"same pixels") == hash_image(b"same pixels")
    assert hash_image(b"same pixels") != hash_image(b"other pixels")


def test_round_trip_through_the_store(tmp_path) -> None:
    store = ShotStore(tmp_path)
    digest = hash_image(b"photo")

    store.put(_shot("s1", digest=digest))

    assert store.has(digest)
    restored = store.get(digest)
    assert restored is not None
    assert restored.carry_distance == 128.1
    assert restored.provenance is not None
    assert restored.provenance.device == "hd_golf"


def test_reimporting_the_same_image_hits_the_cache(tmp_path) -> None:
    store = ShotStore(tmp_path)
    digest = hash_image(b"photo")
    store.put(_shot("s1", digest=digest))

    assert store.has(digest) is True
    assert store.has(hash_image(b"a different photo")) is False
    assert len(store.all()) == 1


def test_missing_store_directory_is_empty_not_an_error(tmp_path) -> None:
    assert ShotStore(tmp_path / "not-created-yet").all() == []


def test_unreadable_files_are_skipped_not_fatal(tmp_path) -> None:
    store = ShotStore(tmp_path)
    store.put(_shot("s1", digest="aaa"))
    (tmp_path / "corrupt.shot.json").write_text("{not json", encoding="utf-8")

    assert [s.shot_id for s in store.all()] == ["s1"]


def test_source_serves_shots_newest_first(tmp_path) -> None:
    store = ShotStore(tmp_path)
    store.put(_shot("first", digest="a", minutes=0))
    store.put(_shot("second", digest="b", minutes=5))
    store.put(_shot("third", digest="c", minutes=10))

    source = ScreenShotDataSource(store)

    assert [s.shot_id for s in source.recent(2)] == ["third", "second"]
    # ...but streams in the order they were actually hit.
    assert [s.shot_id for s in source.stream()] == ["first", "second", "third"]


def test_source_can_filter_by_session_and_hide_flagged_parses(tmp_path) -> None:
    store = ShotStore(tmp_path)
    store.put(_shot("keep", digest="a", session="monday"))
    store.put(_shot("other-session", digest="b", session="tuesday"))
    store.put(_shot("suspect", digest="c", session="monday", needs_review=True))

    everything = ScreenShotDataSource(store, session_id="monday")
    trusted = ScreenShotDataSource(store, session_id="monday", include_needs_review=False)

    assert {s.shot_id for s in everything.recent(10)} == {"keep", "suspect"}
    assert {s.shot_id for s in trusted.recent(10)} == {"keep"}


def test_source_accepts_a_bare_path(tmp_path) -> None:
    ShotStore(tmp_path).put(_shot("s1", digest="a"))

    assert len(ScreenShotDataSource(tmp_path).recent(5)) == 1
