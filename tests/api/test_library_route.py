"""`GET /api/sessions` — the enumeration the library page is built on.

Until this route landed the API could answer *today* (`/api/sessions/current`) and it could answer
a session whose id you already knew, and nothing walked the sessions directory. So a swing from a
previous session was reachable only by typing its `results.html?session=…&swing=…` URL by hand,
which is exactly how fifteen analyzed swings ended up effectively invisible.

The load-bearing test here is the last one: the list and the detail route must describe a swing
identically, because they render the same row in two places. `_swing_row` is the shared
projection, and a second literal in either route is what this pins against.

Base install: manifests and sidecars are seeded onto disk directly, so there is no worker, no
threads and nothing that imports cv2.
"""

from __future__ import annotations

from datetime import UTC, datetime

import pytest
from fastapi.testclient import TestClient

from golf_coach.api.app import _STATIC_DIR, create_app
from golf_coach.api.state import AnalysisState, save_state
from golf_coach.contracts.club import parse_club
from golf_coach.storage.bundle_store import SwingBundleStore
from golf_coach.storage.manifest import (
    Role,
    RoleFile,
    SwingManifest,
    manifest_path,
    save_manifest,
)

_WHEN = datetime(2026, 8, 23, 1, 0, tzinfo=UTC)
_ROLES = ("face_on", "down_the_line", "shot_screen")


@pytest.fixture
def store(tmp_path):
    return SwingBundleStore(tmp_path)


@pytest.fixture
def client(store):
    # worker=None: this is a read path, and a running consumer would race the seeding.
    return TestClient(create_app(store=store, token=None, worker=None))


def _swing(
    store,
    session_id: str,
    swing_id: str,
    *,
    roles=_ROLES,
    club: str | None = "7i",
    player_id: str | None = "aaron",
    state: AnalysisState | None = None,
    video_on_disk: bool = False,
):
    """Put one swing bundle on disk the way an upload plus a worker run would have left it."""
    swing_dir = store.root / session_id / swing_id
    swing_dir.mkdir(parents=True, exist_ok=True)
    save_manifest(
        SwingManifest(
            swing_id=swing_id,
            session_id=session_id,
            created_at=_WHEN,
            updated_at=_WHEN,
            player_id=player_id,
            club=parse_club(club) if club else None,
            roles={
                Role(role): RoleFile(
                    role=Role(role),
                    filename=f"{role}.abc123.mov",
                    content_sha256=f"sha-{session_id}-{swing_id}-{role}",
                    original_filename=f"{role}.mov",
                    content_type="video/quicktime",
                    size_bytes=10,
                    received_at=_WHEN,
                )
                for role in roles
            },
        ),
        manifest_path(swing_dir),
    )
    if video_on_disk:
        (swing_dir / "aligned.mp4").write_bytes(bytes(64))
    if state is not None:
        save_state(state, swing_dir)
    return swing_dir


def _done(score: float, headline: str, *, video: str | None = None) -> AnalysisState:
    return AnalysisState(
        status="done",
        score=score,
        headline=headline,
        video=video,
        video_codec="avc1" if video else None,
    )


def test_sessions_come_back_newest_first(client, store) -> None:
    """Session ids are dates, so the store sorts them oldest-first for a chronological walk. A
    library is read the other way round — the swing you want is almost always the last one."""
    _swing(store, "2026-08-07-aaron1", "1")
    _swing(store, "2026-08-09", "2")
    _swing(store, "2026-08-23", "1")

    payload = client.get("/api/sessions").json()

    assert [s["session_id"] for s in payload["sessions"]] == [
        "2026-08-23",
        "2026-08-09",
        "2026-08-07-aaron1",
    ]


def test_swings_stay_in_the_order_the_store_filled_them(client, store) -> None:
    """Oldest-first within a session, matching `session_detail`. Which end a *page* reads from is
    the page's call; two routes disagreeing about it is not."""
    for swing_id in ("1", "2", "10"):
        _swing(store, "2026-08-23", swing_id)

    payload = client.get("/api/sessions").json()

    assert [s["swing_id"] for s in payload["sessions"][0]["swings"]] == ["1", "2", "10"]


def test_a_session_with_no_swing_bundle_is_not_listed(client, store) -> None:
    """`session.json` alone is a session someone *opened* at the bay — a golfer and club cursor
    were set and nothing was filmed. Four of those exist on the real disk; listing them would be
    four rows a golfer can do nothing with."""
    _swing(store, "2026-08-23", "1")
    (store.root / "2026-08-30").mkdir(parents=True)
    (store.root / "2026-08-30" / "session.json").write_text("{}", encoding="utf-8")

    payload = client.get("/api/sessions").json()

    assert [s["session_id"] for s in payload["sessions"]] == ["2026-08-23"]


def test_a_listed_swing_carries_what_a_row_shows(client, store) -> None:
    """Score, headline, club, golfer and whether there is something to watch — in one trip.

    The alternative was a list of ids plus one detail fetch per session, which is six round trips
    to draw one screen on a phone.
    """
    _swing(
        store,
        "2026-08-23",
        "1",
        state=_done(97.5, "Work on tempo first.", video="aligned.mp4"),
        video_on_disk=True,
    )

    swing = client.get("/api/sessions").json()["sessions"][0]["swings"][0]

    assert swing["player_id"] == "aaron"
    assert swing["club"] == "7i"
    assert swing["status"] == "complete"
    assert swing["analysis"]["score"] == 97.5
    assert swing["analysis"]["headline"] == "Work on tempo first."
    assert swing["analysis"]["has_video"] is True


def test_a_swing_nothing_has_analyzed_is_still_listed(client, store) -> None:
    """A bundle with no sidecar is not a hole in the history — it is the swing most likely to want
    a human, so it must appear rather than being filtered into invisibility."""
    _swing(store, "2026-08-23", "1", club=None, player_id=None)

    swing = client.get("/api/sessions").json()["sessions"][0]["swings"][0]

    assert swing["analysis"]["status"] == "none"
    assert swing["analysis"]["has_video"] is False
    assert swing["club"] is None
    assert swing["player_id"] is None


def test_the_list_and_the_detail_describe_a_swing_identically(client, store) -> None:
    """The no-drift pin. Both routes render the same row through `_swing_row`; a second literal in
    either is what makes the upload page and the library disagree about a swing."""
    _swing(
        store,
        "2026-08-23",
        "1",
        state=_done(88.0, "Work on hip sway first.", video="aligned.mp4"),
        video_on_disk=True,
    )

    listed = client.get("/api/sessions").json()["sessions"][0]["swings"][0]
    detailed = client.get("/api/sessions/2026-08-23").json()["swings"][0]

    assert listed == detailed


def test_the_list_is_gated_like_every_other_api_route(store) -> None:
    """Funnel makes these routes publicly reachable, so an enumeration of every swing on disk is
    the last one that may be open (ADR-016)."""
    client = TestClient(create_app(store=store, token="s3cret-token", worker=None))
    _swing(store, "2026-08-23", "1")

    assert client.get("/api/sessions").status_code == 401
    assert client.get(
        "/api/sessions", headers={"X-Upload-Token": "s3cret-token"}
    ).status_code == 200


def test_the_library_page_ships_beside_the_routes_it_calls() -> None:
    """Static, so nothing else would notice it going missing."""
    page = (_STATIC_DIR / "library.html").read_text(encoding="utf-8")

    assert "/api/sessions" in page
    assert "results.html?session=" in page


def test_the_library_page_says_something_when_the_route_is_not_there() -> None:
    """The page shipped stuck on "Loading..." against a server older than the route.

    `StaticFiles` reads from disk per request, so a running server serves a brand-new page
    immediately and still 404s the route that page calls — and the first version handled only
    401, so `res.json()` threw on the error body, the rejection escaped `load()`, and nothing
    was left to write a message into the DOM. A page that fails must say so.
    """
    page = (_STATIC_DIR / "library.html").read_text(encoding="utf-8")

    assert "sessionsRes.status === 404" in page
    assert "run_server.py" in page      # names the repair, not just the failure
    assert "!sessionsRes.ok" in page    # and every other non-OK status reports too
