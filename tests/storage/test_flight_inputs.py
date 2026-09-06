"""The join that finds a shot's club — and the four different `None`s it tells apart. [M15 P10]

`ShotData` has no club field and never will, so ADR-027 §Decision 3's loft prior has to reach one
through the swing the shot arrived with. Everything here is about that reach and about what it
finds missing, because the missing states are the corpus's whole answer today: one repair is a bay
session, another is the bag page, and a reader that reported both as "no loft" would send the
golfer to the wrong one.

Builders arrive as fixtures (`swing`, `corpus_dir`) from `conftest.py` — the same directory shape
`test_corpus.py` uses, for the same reason: the manifests are written by hand, so these run on a
base install with no pipeline and no OCR.
"""

from __future__ import annotations

from datetime import UTC, datetime
from pathlib import Path

import pytest

from golf_coach.contracts.bag import BagEntry
from golf_coach.contracts.club import ClubId
from golf_coach.contracts.golfer import Handedness
from golf_coach.contracts.shot import ShotData, ShotProvenance, ShotSource
from golf_coach.storage.bag_store import BagStore
from golf_coach.storage.flight_inputs import LoftGap, loft_for_club, read_flight_inputs
from golf_coach.storage.golfer_store import GolferStore

_WHEN = datetime(2026, 8, 23, 3, 37, tzinfo=UTC)


def _at(day: int) -> datetime:
    return datetime(2026, 8, day, 12, tzinfo=UTC)


def _shot(shot_id: str, photo: str | None = "shot-hash") -> ShotData:
    """A parsed shot, keyed to a screen photo the way `ShotStore` files one."""
    return ShotData(
        shot_id=shot_id,
        session_id="2026-08-23",
        timestamp=_WHEN,
        source=ShotSource.SCREEN,
        ball_speed=89.8,
        launch_angle=22.4,
        carry_distance=126.1,
        provenance=(
            None
            if photo is None
            else ShotProvenance(device="hd_golf", parse_confidence=0.9, image_sha256=photo)
        ),
    )


@pytest.fixture
def golfers_dir(tmp_path: Path) -> Path:
    """A registry holding one right-handed golfer and an empty bag."""
    root = tmp_path / "golfers"
    GolferStore(root).get_or_create("Aaron", Handedness.RIGHT)
    return root


def _declare(golfers_dir: Path, club: ClubId, loft_deg: float | None) -> None:
    BagStore(golfers_dir).set_entry(
        "aaron", BagEntry(club=club, make="Titleist", loft_deg=loft_deg, recorded_at=_WHEN)
    )


# --------------------------------------------------------------------------- the join itself


def test_a_shot_reaches_its_club_and_loft_through_the_swing_it_arrived_with(
    corpus_dir, swing, golfers_dir
) -> None:
    """The finding M15 P10 exists to have produced, in miniature.

    ADR-027's 2026-09-05g addendum reported that no shot on disk resolves a loft, having looked at
    `ShotData` — where a club never appears. One join later it does, and the difference between
    the two readings is a milestone's worth of refusals.
    """
    swing(corpus_dir, "2026-08-23", "4", club=ClubId.SEVEN_IRON, shot_screen="photo-a")
    _declare(golfers_dir, ClubId.SEVEN_IRON, 30.5)

    row = read_flight_inputs(
        [_shot("2026-08-23-4", "photo-a")], sessions_dir=corpus_dir, golfers_dir=golfers_dir
    )[0]

    assert row.swing_ref == "2026-08-23/4"
    assert row.club is ClubId.SEVEN_IRON
    assert row.loft_deg == 30.5
    assert row.loft_gap is None
    assert row.handedness is Handedness.RIGHT


def test_a_tagged_club_with_no_bag_entry_names_the_bag_page_and_not_the_bay(
    corpus_dir, swing, golfers_dir
) -> None:
    """Six shots on disk are exactly here, all `3w`, and the distinction decides what to say.

    `NO_CLUB_TAG` is repaired at the bay or on the results page; this is repaired by looking a club
    up once. Collapsing the two would tell a golfer with a fully tagged session to go and tag it.
    """
    swing(corpus_dir, "2026-08-23", "6", club=ClubId.THREE_WOOD, shot_screen="photo-b")

    row = read_flight_inputs(
        [_shot("2026-08-23-6", "photo-b")], sessions_dir=corpus_dir, golfers_dir=golfers_dir
    )[0]

    assert row.club is ClubId.THREE_WOOD
    assert row.loft_deg is None
    assert row.loft_gap is LoftGap.NO_BAG_ENTRY


def test_an_entry_with_no_declared_loft_is_a_third_state_again(
    corpus_dir, swing, golfers_dir
) -> None:
    """Five of this bag's six entries are here: M12's set lookup filled in makes, not numbers.

    A blank `loft_deg` is ADR-026 §1's "nobody, the manufacturer included, has said" rather than
    an absent club, and the repair is one field rather than a lookup.
    """
    swing(corpus_dir, "2026-08-23", "1", club=ClubId.FIVE_IRON, shot_screen="photo-c")
    _declare(golfers_dir, ClubId.FIVE_IRON, None)

    row = read_flight_inputs(
        [_shot("2026-08-23-1", "photo-c")], sessions_dir=corpus_dir, golfers_dir=golfers_dir
    )[0]

    assert row.club is ClubId.FIVE_IRON
    assert row.loft_gap is LoftGap.NO_DECLARED_LOFT


def test_an_untagged_swing_is_the_gap_the_club_cursor_closes(
    corpus_dir, swing, golfers_dir
) -> None:
    """Both 2026-08-10 shots are here — they predate the club tag entirely."""
    swing(corpus_dir, "2026-08-10", "1", club=None, shot_screen="photo-d")

    row = read_flight_inputs(
        [_shot("2026-08-10-1", "photo-d")], sessions_dir=corpus_dir, golfers_dir=golfers_dir
    )[0]

    assert row.swing_ref == "2026-08-10/1"
    assert row.club is None
    assert row.loft_gap is LoftGap.NO_CLUB_TAG


def test_a_shot_no_swing_carries_is_a_state_and_not_an_error(corpus_dir, golfers_dir) -> None:
    """A screenshot imported on its own. `import_shot_screens.py` produces exactly this."""
    rows = read_flight_inputs(
        [_shot("loose-1", "photo-none"), _shot("loose-2", photo=None)],
        sessions_dir=corpus_dir,
        golfers_dir=golfers_dir,
    )

    assert [row.loft_gap for row in rows] == [LoftGap.NO_SWING, LoftGap.NO_SWING]
    assert [row.swing_ref for row in rows] == [None, None]
    assert [row.handedness for row in rows] == [None, None]


# --------------------------------------------------------------------------- the awkward cases


def test_the_earliest_arrival_is_read_and_the_re_uploads_are_named(
    corpus_dir, swing, golfers_dir
) -> None:
    """Three swing directories on disk share one 2026-08-10 photo, and they can disagree.

    `storage/corpus.py` settles the same question for the face-on clip and this follows it: a
    re-upload's timestamp dates the upload rather than the swing, so the latest club cursor is a
    stale reading of one swing and not a competing one. What must never happen is silence — a club
    chosen by dictionary order would be a spin inferred from whichever directory sorted first.
    """
    swing(corpus_dir, "2026-08-10", "1", club=ClubId.NINE_IRON, shot_screen="photo-e",
          created_at=_at(10))
    swing(corpus_dir, "2026-08-07", "1", club=ClubId.SEVEN_IRON, shot_screen="photo-e",
          created_at=_at(7))
    swing(corpus_dir, "2026-08-09", "2", club=None, shot_screen="photo-e", created_at=_at(9))
    _declare(golfers_dir, ClubId.SEVEN_IRON, 30.5)
    _declare(golfers_dir, ClubId.NINE_IRON, 41.0)

    row = read_flight_inputs(
        [_shot("2026-08-10-1", "photo-e")], sessions_dir=corpus_dir, golfers_dir=golfers_dir
    )[0]

    assert row.swing_ref == "2026-08-07/1"
    assert row.loft_deg == 30.5
    assert row.also_attached_to == ("2026-08-09/2", "2026-08-10/1")


def test_a_club_with_no_golfer_to_own_a_bag_reports_the_repair_it_actually_needs(
    corpus_dir, swing, golfers_dir
) -> None:
    """An unattributed swing can still carry a club, and there is then no bag to look it up in."""
    swing(corpus_dir, "2026-08-23", "9", player_id=None, club=ClubId.THREE_WOOD,
          shot_screen="photo-f")

    row = read_flight_inputs(
        [_shot("2026-08-23-9", "photo-f")], sessions_dir=corpus_dir, golfers_dir=golfers_dir
    )[0]

    assert row.player_id is None
    assert row.handedness is None
    assert row.loft_gap is LoftGap.NO_BAG_ENTRY


def test_missing_directories_are_an_empty_join_rather_than_a_traceback(tmp_path: Path) -> None:
    """The first run on a fresh checkout, and the state `scripts/simulate_flight.py` prints for."""
    rows = read_flight_inputs(
        [_shot("2026-08-23-4")],
        sessions_dir=tmp_path / "nothing-here",
        golfers_dir=tmp_path / "nor-here",
    )

    assert [row.loft_gap for row in rows] == [LoftGap.NO_SWING]


def test_the_callers_order_survives_because_it_is_the_shots_own(
    corpus_dir, swing, golfers_dir
) -> None:
    """`ShotStore.all()` sorts newest first, and a reader that re-sorted would hide that."""
    swing(corpus_dir, "2026-08-23", "4", club=ClubId.SEVEN_IRON, shot_screen="photo-a")
    ids = ["c", "a", "b"]

    rows = read_flight_inputs(
        [_shot(shot_id, "photo-a") for shot_id in ids],
        sessions_dir=corpus_dir,
        golfers_dir=golfers_dir,
    )

    assert [row.shot.shot_id for row in rows] == ids


# --------------------------------------------------------------------------- the join's other half


def test_the_bag_lookup_is_reachable_without_the_photo_join(golfers_dir) -> None:
    """M15 P11's route in: `api/pipeline.py` is already holding the manifest.

    Joining a shot photo's sha256 against every swing on disk would be re-deriving what that shell
    knows, and `_swings_by_shot_photo` parses every manifest in every session to do it. Both routes
    run the same `_resolve_loft`, which is what stops a second definition of "declared loft" from
    drifting away from this one.
    """
    _declare(golfers_dir, ClubId.SEVEN_IRON, 30.5)

    assert loft_for_club("aaron", ClubId.SEVEN_IRON, golfers_dir=golfers_dir) == (30.5, None)


def test_the_bag_lookup_reports_the_same_four_gaps(golfers_dir) -> None:
    """The distinction is the whole reason `LoftGap` exists, so it must survive the shorter route.

    `NO_SWING` is the one value this route cannot produce — a caller here has a swing in hand — and
    that asymmetry is stated on `loft_for_club` rather than left to be discovered.
    """
    _declare(golfers_dir, ClubId.THREE_WOOD, None)

    assert loft_for_club("aaron", None, golfers_dir=golfers_dir) == (None, LoftGap.NO_CLUB_TAG)
    assert loft_for_club(None, ClubId.SEVEN_IRON, golfers_dir=golfers_dir) == (
        None,
        LoftGap.NO_BAG_ENTRY,
    )
    assert loft_for_club("aaron", ClubId.SEVEN_IRON, golfers_dir=golfers_dir) == (
        None,
        LoftGap.NO_BAG_ENTRY,
    )
    assert loft_for_club("aaron", ClubId.THREE_WOOD, golfers_dir=golfers_dir) == (
        None,
        LoftGap.NO_DECLARED_LOFT,
    )


def test_a_golfer_with_no_bag_on_disk_is_a_gap_and_not_an_error(golfers_dir) -> None:
    """A registered golfer who has never opened the bag page is an ordinary state, not a failure."""
    assert loft_for_club("aaron", ClubId.SEVEN_IRON, golfers_dir=golfers_dir) == (
        None,
        LoftGap.NO_BAG_ENTRY,
    )
