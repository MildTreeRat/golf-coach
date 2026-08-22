"""Upload API — role-based ingestion, session status, token auth, and the bind guard."""

from __future__ import annotations

import hashlib
from datetime import UTC, datetime
from pathlib import Path

import pytest
from fastapi.testclient import TestClient

from golf_coach.api.app import create_app
from golf_coach.contracts.bag import BagEntry
from golf_coach.contracts.club import ClubId
from golf_coach.storage.bag_store import BagStore
from golf_coach.storage.bundle_store import SwingBundleStore
from golf_coach.storage.golfer_store import GolferStore

_REPO_ROOT = Path(__file__).resolve().parents[2]


_TOKEN = "s3cret-token"


@pytest.fixture
def client(tmp_path):
    # token=None pins the unauthenticated case explicitly, so these tests keep passing
    # whether or not the developer running them has GOLF_UPLOAD_TOKEN in their .env.
    store = SwingBundleStore(tmp_path)
    return TestClient(create_app(store=store, token=None))


@pytest.fixture
def auth_client(tmp_path):
    store = SwingBundleStore(tmp_path)
    return TestClient(create_app(store=store, token=_TOKEN))


def _pick_club(client, club: str = "7i", token: str | None = None) -> None:
    """Select the session's club, which every successful upload now needs (M9 P6).

    Called at the call site rather than folded into the fixtures on purpose: the tests below that
    do *not* call it are the ones asserting the refusal, and that has to be visible in the test
    rather than hidden in a fixture.
    """
    headers = {"X-Upload-Token": token} if token else {}
    res = client.post("/api/sessions/current/club", json={"club": club}, headers=headers)
    assert res.status_code == 200


def test_upload_returns_swing_and_status(client) -> None:
    _pick_club(client)

    res = client.post(
        "/api/uploads", params={"role": "face_on", "filename": "clip.mov"}, content=b"face-on-bytes"
    )

    assert res.status_code == 200
    body = res.json()
    assert body["swing_id"] == "1"
    assert body["role"] == "face_on"
    assert body["status"] == "collecting"
    assert body["deduped"] is False
    assert "waiting on" in body["message"]


def test_unknown_role_is_rejected(client) -> None:
    res = client.post(
        "/api/uploads", params={"role": "bogus", "filename": "x.mov"}, content=b"data"
    )

    assert res.status_code == 400


def test_session_current_and_detail_reflect_uploads(client) -> None:
    _pick_club(client)
    client.post("/api/uploads", params={"role": "face_on", "filename": "x.mov"}, content=b"a")
    client.post("/api/uploads", params={"role": "down_the_line", "filename": "y.mov"}, content=b"b")

    current = client.get("/api/sessions/current").json()
    detail = client.get(f"/api/sessions/{current['session_id']}").json()

    assert len(detail["swings"]) == 1
    swing = detail["swings"][0]
    assert swing["roles"]["face_on"] is not None
    assert swing["roles"]["down_the_line"] is not None
    assert swing["roles"]["shot_screen"] is None


def test_duplicate_upload_dedupes(client) -> None:
    _pick_club(client)
    data = b"same-bytes"
    client.post("/api/uploads", params={"role": "face_on", "filename": "x.mov"}, content=data)

    res = client.post("/api/uploads", params={"role": "face_on", "filename": "x.mov"}, content=data)

    assert res.json()["deduped"] is True


def test_large_payload_round_trips_with_correct_hash(client, tmp_path) -> None:
    _pick_club(client)
    data = b"x" * (5 * 1024 * 1024)  # 5 MiB, exercises the chunked streaming path

    res = client.post(
        "/api/uploads", params={"role": "shot_screen", "filename": "big.jpg"}, content=data
    )

    assert res.status_code == 200
    body = res.json()
    swing_dir = tmp_path / body["session_id"] / body["swing_id"]
    stored_files = [p for p in swing_dir.iterdir() if p.name != "manifest.json"]
    assert len(stored_files) == 1
    stored_digest = hashlib.sha256(stored_files[0].read_bytes()).hexdigest()
    assert stored_digest == hashlib.sha256(data).hexdigest()


def test_server_binds_loopback_only_by_default() -> None:
    # Regression guard against a future edit silently widening the bind address.
    source = (_REPO_ROOT / "scripts" / "run_server.py").read_text(encoding="utf-8")

    assert '_DEFAULT_HOST = "127.0.0.1"' in source


# --- Token auth (ADR-016) -----------------------------------------------------------
# Tailscale Funnel makes these routes publicly reachable, so the token is the only thing
# between the open internet and a 2 GiB write to disk.


def test_upload_rejected_without_token(auth_client) -> None:
    res = auth_client.post(
        "/api/uploads", params={"role": "face_on", "filename": "x.mov"}, content=b"data"
    )

    assert res.status_code == 401


def test_upload_rejected_with_wrong_token(auth_client) -> None:
    res = auth_client.post(
        "/api/uploads",
        params={"role": "face_on", "filename": "x.mov"},
        content=b"data",
        headers={"X-Upload-Token": "not-the-token"},
    )

    assert res.status_code == 401


def test_upload_accepted_with_token_header(auth_client) -> None:
    _pick_club(auth_client, token=_TOKEN)

    res = auth_client.post(
        "/api/uploads",
        params={"role": "face_on", "filename": "x.mov"},
        content=b"data",
        headers={"X-Upload-Token": _TOKEN},
    )

    assert res.status_code == 200
    assert res.json()["role"] == "face_on"


def test_upload_accepted_with_token_query_param(auth_client) -> None:
    # The `?t=` form is what makes the one-time setup link work on a phone.
    _pick_club(auth_client, token=_TOKEN)

    res = auth_client.post(
        "/api/uploads",
        params={"role": "face_on", "filename": "x.mov", "t": _TOKEN},
        content=b"data",
    )

    assert res.status_code == 200


def test_session_routes_require_token(auth_client) -> None:
    assert auth_client.get("/api/sessions/current").status_code == 401
    assert auth_client.get("/api/sessions/anything").status_code == 401
    assert (
        auth_client.get("/api/sessions/current", headers={"X-Upload-Token": _TOKEN}).status_code
        == 200
    )


def test_static_page_stays_reachable_without_token(auth_client) -> None:
    # The phone has to be able to load the page before it can learn the token from `?t=`.
    res = auth_client.get("/")

    assert res.status_code == 200
    assert "Swing Upload" in res.text


def test_rejected_upload_writes_nothing_to_disk(auth_client, tmp_path) -> None:
    # The guard is a route dependency precisely so it runs before `request.stream()`.
    auth_client.post(
        "/api/uploads",
        params={"role": "face_on", "filename": "x.mov"},
        content=b"x" * (2 * 1024 * 1024),
    )

    incoming = tmp_path / ".incoming"
    assert not incoming.exists() or not list(incoming.iterdir())


# --------------------------------------------------------------------------- golfer identity


@pytest.fixture
def golfer_client(tmp_path):
    """A client whose golfer registry is also isolated to tmp_path."""
    store = SwingBundleStore(tmp_path / "sessions")
    golfers = GolferStore(tmp_path / "golfers")
    return TestClient(create_app(store=store, golfers=golfers, token=None, worker=None))


def _session_swings(client):
    session_id = client.get("/api/sessions/current").json()["session_id"]
    return client.get(f"/api/sessions/{session_id}").json()["swings"]


def test_no_golfer_is_selected_until_someone_picks_one(golfer_client) -> None:
    body = golfer_client.get("/api/sessions/current/golfer").json()

    assert body["player_id"] is None
    assert body["golfer"] is None


def test_uploading_is_not_blocked_by_a_missing_golfer(golfer_client) -> None:
    """The deliberate choice: a bay session must never stall on a form.

    Still true after M9 P6, and the club is not a counter-example: an untagged *golfer* is
    repairable later and an untagged *club* is not, so only one of the two blocks. See the
    club block below.
    """
    _pick_club(golfer_client)

    res = golfer_client.post(
        "/api/uploads", params={"role": "face_on", "filename": "a.mov"}, content=b"aaa"
    )

    assert res.status_code == 200
    assert res.json()["player_id"] is None


def test_a_new_golfer_needs_a_handedness(golfer_client) -> None:
    """Never guessed: it is the frame of reference for every signed metric."""
    res = golfer_client.post("/api/sessions/current/golfer", json={"name": "Aaron"})

    assert res.status_code == 400
    assert "right- or left-handed" in res.json()["detail"]


def test_a_nameless_golfer_is_rejected(golfer_client) -> None:
    res = golfer_client.post(
        "/api/sessions/current/golfer", json={"name": "!!!", "handedness": "right"}
    )

    assert res.status_code == 400


def test_setting_a_golfer_adopts_the_swings_already_uploaded(golfer_client) -> None:
    _pick_club(golfer_client)
    golfer_client.post(
        "/api/uploads", params={"role": "face_on", "filename": "a.mov"}, content=b"aaa"
    )

    res = golfer_client.post(
        "/api/sessions/current/golfer", json={"name": "Aaron", "handedness": "right"}
    )

    assert res.json()["attributed"] == ["1"]
    assert [s["player_id"] for s in _session_swings(golfer_client)] == ["aaron"]


def test_switching_golfer_leaves_earlier_swings_alone(golfer_client) -> None:
    _pick_club(golfer_client)
    golfer_client.post(
        "/api/sessions/current/golfer", json={"name": "Aaron", "handedness": "right"}
    )
    golfer_client.post(
        "/api/uploads", params={"role": "face_on", "filename": "a.mov"}, content=b"aaa"
    )

    golfer_client.post(
        "/api/sessions/current/golfer", json={"name": "Dave", "handedness": "left"}
    )
    golfer_client.post(
        "/api/uploads", params={"role": "face_on", "filename": "b.mov"}, content=b"bbb"
    )

    assert [s["player_id"] for s in _session_swings(golfer_client)] == ["aaron", "dave"]


def test_a_retyped_name_does_not_create_a_second_golfer(golfer_client) -> None:
    golfer_client.post(
        "/api/sessions/current/golfer", json={"name": "Aaron", "handedness": "right"}
    )

    res = golfer_client.post(
        "/api/sessions/current/golfer", json={"name": "  aaron ", "handedness": "left"}
    )

    # Same golfer, and the stored handedness survives being contradicted.
    assert res.json()["golfer"] == {
        "player_id": "aaron",
        "display_name": "Aaron",
        "handedness": "right",
    }
    assert len(golfer_client.get("/api/golfers").json()["golfers"]) == 1


def test_the_repair_path_re_attributes_one_swing(golfer_client) -> None:
    _pick_club(golfer_client)
    golfer_client.post(
        "/api/sessions/current/golfer", json={"name": "Dave", "handedness": "left"}
    )
    for name, data in (("a.mov", b"aaa"), ("b.mov", b"bbb")):
        golfer_client.post(
            "/api/uploads", params={"role": "face_on", "filename": name}, content=data
        )
    session_id = golfer_client.get("/api/sessions/current").json()["session_id"]

    res = golfer_client.post(
        f"/api/sessions/{session_id}/swings/1/golfer",
        json={"name": "Aaron", "handedness": "right"},
    )

    assert res.status_code == 200
    assert [s["player_id"] for s in _session_swings(golfer_client)] == ["aaron", "dave"]


def test_the_repair_path_404s_on_an_unknown_swing(golfer_client) -> None:
    golfer_client.post(
        "/api/sessions/current/golfer", json={"name": "Aaron", "handedness": "right"}
    )
    session_id = golfer_client.get("/api/sessions/current").json()["session_id"]

    res = golfer_client.post(
        f"/api/sessions/{session_id}/swings/99/golfer", json={"name": "Aaron"}
    )

    assert res.status_code == 404


def test_golfer_routes_are_behind_the_upload_token(auth_client) -> None:
    assert auth_client.get("/api/golfers").status_code == 401
    assert auth_client.get("/api/sessions/current/golfer").status_code == 401
    assert auth_client.post(
        "/api/sessions/current/golfer", json={"name": "Aaron", "handedness": "right"}
    ).status_code == 401


# --------------------------------------------------------------- club, required at the boundary
#
# Mirrors the golfer block above, and the asymmetries are the point of reading them side by side.
# The club is required where the golfer is not, and it has no bulk backfill where the golfer does:
# an untagged golfer is repairable later, and nothing but memory can say which club hit swing 3
# (ADR-024 §5).


def test_no_club_is_selected_until_someone_picks_one(client) -> None:
    body = client.get("/api/sessions/current/club").json()

    assert body["club"] is None


def test_picking_a_club_sets_the_cursor(client) -> None:
    res = client.post("/api/sessions/current/club", json={"club": "7i"})

    assert res.status_code == 200
    assert res.json()["club"] == "7i"
    assert client.get("/api/sessions/current/club").json()["club"] == "7i"


def test_the_cursor_accepts_the_spellings_a_golfer_types(client) -> None:
    """`parse_club` reaches the route — the reason `ClubRequest.club` is a `str` and not a `ClubId`.

    Typed as the enum, pydantic would 422 this before the tolerant parser ever ran.
    """
    res = client.post("/api/sessions/current/club", json={"club": "7 Iron"})

    assert res.status_code == 200
    assert res.json()["club"] == "7i"


def test_an_unparseable_club_is_rejected_rather_than_guessed(client) -> None:
    res = client.post("/api/sessions/current/club", json={"club": "banana"})

    assert res.status_code == 400
    assert "banana" in res.json()["detail"]


def test_an_upload_with_no_club_is_refused(client) -> None:
    res = client.post(
        "/api/uploads", params={"role": "face_on", "filename": "x.mov"}, content=b"data"
    )

    assert res.status_code == 409
    # The refusal has to name the fix; the phone shows this string verbatim.
    assert "/api/sessions/current/club" in res.json()["detail"]


def test_a_clubless_upload_writes_nothing_to_disk(client, tmp_path) -> None:
    """The load-bearing half of the 409: it fires *before* `request.stream()`.

    Same property `test_rejected_upload_writes_nothing_to_disk` pins for the token guard. Refusing
    after the body had already landed would leave a 50 MB orphan in `.incoming` per refused shot.
    """
    client.post(
        "/api/uploads",
        params={"role": "face_on", "filename": "x.mov"},
        content=b"x" * (2 * 1024 * 1024),
    )

    incoming = tmp_path / ".incoming"
    assert not incoming.exists() or not list(incoming.iterdir())


def test_a_bogus_role_is_still_a_400_and_not_a_409(client) -> None:
    """Role parses first: a bad role is the client's bug, a missing club is the human's."""
    res = client.post(
        "/api/uploads", params={"role": "bogus", "filename": "x.mov"}, content=b"data"
    )

    assert res.status_code == 400


def test_an_upload_carries_the_selected_club_to_the_manifest(client, tmp_path) -> None:
    _pick_club(client, "pw")

    res = client.post(
        "/api/uploads", params={"role": "face_on", "filename": "x.mov"}, content=b"data"
    )

    assert res.json()["club"] == "pw"
    manifest = SwingBundleStore(tmp_path).get_swing(res.json()["session_id"], "1")
    assert manifest is not None
    assert manifest.club == "pw"
    # And the read routes report it wherever they already report `player_id`.
    session_id = res.json()["session_id"]
    assert client.get(f"/api/sessions/{session_id}").json()["swings"][0]["club"] == "pw"
    assert client.get(f"/api/sessions/{session_id}/swings/1").json()["club"] == "pw"


def test_reaching_for_the_next_club_does_not_retag_the_last_ones(client) -> None:
    """The no-backfill asymmetry, at the route. The golfer route deliberately does the opposite."""
    _pick_club(client, "7i")
    client.post("/api/uploads", params={"role": "face_on", "filename": "a.mov"}, content=b"aaa")

    _pick_club(client, "pw")
    client.post("/api/uploads", params={"role": "face_on", "filename": "b.mov"}, content=b"bbb")

    session_id = client.get("/api/sessions/current").json()["session_id"]
    swings = client.get(f"/api/sessions/{session_id}").json()["swings"]
    assert [s["club"] for s in swings] == ["7i", "pw"]


def test_a_retry_after_the_cursor_moved_is_told_the_stored_club(client) -> None:
    """The deduped path echoes what the swing *says*, not what the retry asked for (M9 P5)."""
    _pick_club(client, "7i")
    data = b"same-bytes"
    client.post("/api/uploads", params={"role": "face_on", "filename": "a.mov"}, content=data)

    _pick_club(client, "pw")
    res = client.post(
        "/api/uploads", params={"role": "face_on", "filename": "a.mov"}, content=data
    )

    assert res.json()["deduped"] is True
    assert res.json()["club"] == "7i"


def test_picking_a_club_leaves_the_golfer_alone(golfer_client) -> None:
    """One direction of P4's `_update_cursor` pin, at the route. See the sibling test below."""
    golfer_client.post(
        "/api/sessions/current/golfer", json={"name": "Aaron", "handedness": "right"}
    )

    _pick_club(golfer_client, "7i")

    assert golfer_client.get("/api/sessions/current/golfer").json()["player_id"] == "aaron"


def test_picking_a_golfer_leaves_the_club_alone(golfer_client) -> None:
    """The other direction. Two tests rather than one: each catches a different broken setter."""
    _pick_club(golfer_client, "7i")

    golfer_client.post(
        "/api/sessions/current/golfer", json={"name": "Aaron", "handedness": "right"}
    )

    assert golfer_client.get("/api/sessions/current/club").json()["club"] == "7i"


def test_the_club_repair_path_retags_one_swing(client) -> None:
    _pick_club(client, "7i")
    for name, data in (("a.mov", b"aaa"), ("b.mov", b"bbb")):
        client.post("/api/uploads", params={"role": "face_on", "filename": name}, content=data)
    session_id = client.get("/api/sessions/current").json()["session_id"]

    res = client.post(f"/api/sessions/{session_id}/swings/1/club", json={"club": "sand wedge"})

    assert res.status_code == 200
    swings = client.get(f"/api/sessions/{session_id}").json()["swings"]
    assert [s["club"] for s in swings] == ["sw", "7i"]


def test_the_club_repair_path_404s_on_an_unknown_swing(client) -> None:
    _pick_club(client)
    session_id = client.get("/api/sessions/current").json()["session_id"]

    res = client.post(f"/api/sessions/{session_id}/swings/99/club", json={"club": "7i"})

    assert res.status_code == 404


def test_the_club_repair_path_rejects_junk_before_it_touches_a_swing(client) -> None:
    _pick_club(client)
    client.post("/api/uploads", params={"role": "face_on", "filename": "a.mov"}, content=b"aaa")
    session_id = client.get("/api/sessions/current").json()["session_id"]

    res = client.post(f"/api/sessions/{session_id}/swings/1/club", json={"club": "banana"})

    assert res.status_code == 400
    assert client.get(f"/api/sessions/{session_id}/swings/1").json()["club"] == "7i"


def test_club_routes_are_behind_the_upload_token(auth_client) -> None:
    assert auth_client.get("/api/sessions/current/club").status_code == 401
    assert auth_client.post("/api/sessions/current/club", json={"club": "7i"}).status_code == 401
    assert auth_client.post(
        "/api/sessions/2026-08-21/swings/1/club", json={"club": "7i"}
    ).status_code == 401


# ------------------------------------------------------------------ the picker's inputs (M9 P7)
#
# `GET /api/clubs` exists so `static/index.html` never holds a second copy of the club taxonomy.
# The tests below therefore assert against `ClubId` itself rather than against a written-out list:
# a literal here would be the very duplicate the route was added to prevent, and it would pass
# happily on the day a club is added to the enum and forgotten everywhere else.


def test_the_picker_is_offered_every_club_in_canonical_order(client) -> None:
    body = client.get("/api/clubs").json()

    assert body["clubs"] == [club.value for club in ClubId]


def test_every_club_the_picker_offers_is_one_the_cursor_accepts(client) -> None:
    """The pin that makes the picker's list and the cursor's parser one vocabulary.

    Without it the two can drift in either direction — a club served but unparseable, or a rename
    landing in `ClubId` and reaching the page as a tap that 400s at the bay.
    """
    for club in client.get("/api/clubs").json()["clubs"]:
        res = client.post("/api/sessions/current/club", json={"club": club})

        assert res.status_code == 200, club
        assert res.json()["club"] == club


def test_a_golfer_with_no_bag_gets_the_taxonomy_and_nothing_else(golfer_client) -> None:
    """`bag` is empty rather than absent: the page renders one section instead of two."""
    golfer_client.post(
        "/api/sessions/current/golfer", json={"name": "Aaron", "handedness": "right"}
    )

    assert golfer_client.get("/api/clubs").json()["bag"] == []


def test_no_golfer_at_all_is_not_an_error(client) -> None:
    """The state the page opens in. The picker has to render before anyone has typed a name."""
    body = client.get("/api/clubs").json()

    assert body["bag"] == []
    assert body["clubs"]


def test_a_declared_bag_comes_back_in_bag_order_not_the_order_it_was_declared(
    golfer_client, tmp_path
) -> None:
    """Declared wedge-first on purpose. Serving `bag.entries.keys()` passes every other test here
    and fails this one, which is the whole reason the route goes through `Bag.club_ids`."""
    golfer_client.post(
        "/api/sessions/current/golfer", json={"name": "Aaron", "handedness": "right"}
    )
    bags = BagStore(tmp_path / "golfers")
    for club in (ClubId.PITCHING_WEDGE, ClubId.DRIVER, ClubId.SEVEN_IRON):
        bags.set_entry("aaron", BagEntry(club=club, recorded_at=datetime.now(tz=UTC)))

    assert golfer_client.get("/api/clubs").json()["bag"] == ["driver", "7i", "pw"]


def test_one_golfers_bag_is_not_another_golfers(golfer_client, tmp_path) -> None:
    """The bag follows the session's golfer cursor, so handing the phone over changes it."""
    bags = BagStore(tmp_path / "golfers")
    bags.set_entry("aaron", BagEntry(club=ClubId.SEVEN_IRON, recorded_at=datetime.now(tz=UTC)))

    golfer_client.post(
        "/api/sessions/current/golfer", json={"name": "Aaron", "handedness": "right"}
    )
    assert golfer_client.get("/api/clubs").json()["bag"] == ["7i"]

    golfer_client.post(
        "/api/sessions/current/golfer", json={"name": "Rory", "handedness": "left"}
    )
    assert golfer_client.get("/api/clubs").json()["bag"] == []


def test_an_unreadable_bag_costs_the_shortcut_and_not_the_picker(golfer_client, tmp_path) -> None:
    """`BagStore.get`'s tolerance, at the route. A corrupt bag must not 500 the page someone is
    holding between swings — it costs the "in the bag" section and nothing else."""
    golfer_client.post(
        "/api/sessions/current/golfer", json={"name": "Aaron", "handedness": "right"}
    )
    (tmp_path / "golfers").mkdir(parents=True, exist_ok=True)
    (tmp_path / "golfers" / "aaron.bag.json").write_text("{ not json", encoding="utf-8")

    res = golfer_client.get("/api/clubs")

    assert res.status_code == 200
    assert res.json()["bag"] == []
    assert res.json()["clubs"] == [club.value for club in ClubId]


def test_the_picker_route_is_behind_the_upload_token(auth_client) -> None:
    assert auth_client.get("/api/clubs").status_code == 401
