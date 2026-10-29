"""The upload server's front door: the closed write routes, session status, token auth, the picker
and the bind guard.

**Since M29 this server writes no swing** (the M29 plan's D11). The eight routes that wrote under
`sessions/` or `shots/` answer 410 naming the Rust verb that does their work, and the tests that
pinned what each route wrote became the refusal tests below. What those tests held is held where
the work went: the store's rules (dedupe, "newest swing lacking that role", the `swing_id` override,
write-once attribution, no club backfill) in `tests/storage/test_bundle_store.py` and the Rust
storage family; the golfer, club and delete repairs in `crates/core/tests/verbs.rs`, as
`golf-core set-golfer`, `set-club` and `delete-swing`. Disk state is filed through the frozen store
(`conftest.py`'s `file_swing`), as the upload route filed it.
"""

from __future__ import annotations

import hashlib
from datetime import UTC, datetime
from pathlib import Path

import pytest
from fastapi.testclient import TestClient

from golf_coach.api.app import create_app
from golf_coach.contracts.bag import BagEntry
from golf_coach.contracts.club import ClubId, parse_club
from golf_coach.contracts.golfer import Handedness
from golf_coach.storage.bag_store import BagStore
from golf_coach.storage.bundle_store import SwingBundleStore
from golf_coach.storage.golfer_store import GolferStore
from golf_coach.storage.manifest import Role
from golf_coach.storage.session_meta import set_current_player

_REPO_ROOT = Path(__file__).resolve().parents[2]


_TOKEN = "s3cret-token"


@pytest.fixture
def store(tmp_path):
    return SwingBundleStore(tmp_path)


@pytest.fixture
def client(store):
    # token=None pins the unauthenticated case explicitly, so these tests keep passing
    # whether or not the developer running them has GOLF_UPLOAD_TOKEN in their .env.
    return TestClient(create_app(store=store, token=None, worker=None))


@pytest.fixture
def auth_client(store):
    return TestClient(create_app(store=store, token=_TOKEN, worker=None))


def _every_file(root: Path) -> dict[str, bytes]:
    """Every file under `root`, by relative path: what a refusal must leave as it found it."""
    if not root.exists():
        return {}
    return {
        path.relative_to(root).as_posix(): path.read_bytes()
        for path in sorted(root.rglob("*"))
        if path.is_file()
    }


# ------------------------------------------------------------- the closed write routes [M29 P32]
#
# Each route, a body it would once have accepted, and the Rust verb its refusal must name. The
# swing routes echo the ids back into the command, so a golfer can run what the page shows.

_SESSION = "2026-08-20"

_CLOSED = [
    pytest.param(
        "post",
        "/api/uploads",
        {"params": {"role": "face_on", "filename": "x.mov"}, "content": b"face-on-bytes"},
        ["golf-lab import-export", "golf-lab import-photos", "--into-session"],
        id="upload",
    ),
    pytest.param(
        "post",
        "/api/sessions/current/golfer",
        {"json": {"name": "Aaron", "handedness": "right"}},
        ["golf-lab import-photos --into-session S --player P", "golf-core set-golfer"],
        id="golfer-cursor",
    ),
    pytest.param(
        "post",
        "/api/sessions/current/club",
        {"json": {"club": "7i"}},
        ["golf-lab import-photos --into-session S --player P --club C", "golf-core set-club"],
        id="club-cursor",
    ),
    pytest.param(
        "post",
        f"/api/sessions/{_SESSION}/swings/1/golfer",
        {"json": {"name": "Aaron", "handedness": "right"}},
        [f"golf-core set-golfer --session {_SESSION} --swing 1 --name N"],
        id="swing-golfer",
    ),
    pytest.param(
        "post",
        f"/api/sessions/{_SESSION}/swings/1/club",
        {"json": {"club": "sand wedge"}},
        [f"golf-core set-club --session {_SESSION} --swing 1 --club C"],
        id="swing-club",
    ),
    pytest.param(
        "post",
        f"/api/sessions/{_SESSION}/swings/1/mishit",
        {"json": {"verdict": "confirmed"}},
        [f"golf-core flag-mishit {_SESSION}/1 --confirm, --clear or --auto"],
        id="mishit",
    ),
    pytest.param(
        "post",
        f"/api/sessions/{_SESSION}/swings/1/analyze",
        {},
        [f"golf-lab analyze {_SESSION}/1"],
        id="analyze",
    ),
    pytest.param(
        "delete",
        f"/api/sessions/{_SESSION}/swings/1",
        {},
        [f"golf-core delete-swing --session {_SESSION} --swing 1"],
        id="delete",
    ),
]


def _seed_a_swing_in(tmp_path: Path) -> tuple[SwingBundleStore, GolferStore]:
    """One filed swing in `_SESSION` with a golfer and a cursor beside it: something for each
    closed route to have changed, had it still run."""
    store = SwingBundleStore(tmp_path / "sessions")
    golfers = GolferStore(tmp_path / "golfers")
    golfers.get_or_create("Dave", Handedness.LEFT)
    tmp = tmp_path / "upload.part"
    tmp.write_bytes(b"face-on")
    store.assign_from_path(
        session_id=_SESSION,
        role=Role.FACE_ON,
        tmp_path=tmp,
        digest=hashlib.sha256(b"face-on").hexdigest(),
        original_filename="face_on.mov",
        content_type="application/octet-stream",
        size_bytes=7,
        player_id="dave",
        club=ClubId.SEVEN_IRON,
    )
    set_current_player(store.root / _SESSION, "dave")
    return store, golfers


@pytest.mark.parametrize(("method", "path", "request_kwargs", "names"), _CLOSED)
def test_each_closed_route_refuses_naming_its_verb_and_writes_nothing(
    tmp_path, method, path, request_kwargs, names
) -> None:
    store, golfers = _seed_a_swing_in(tmp_path)
    client = TestClient(create_app(store=store, golfers=golfers, token=None, worker=None))
    before = _every_file(tmp_path)

    res = getattr(client, method)(path, **request_kwargs)

    assert res.status_code == 410
    detail = res.json()["detail"]
    assert "moved to Rust at M29" in detail
    for name in names:
        assert name in detail, detail
    assert _every_file(tmp_path) == before


@pytest.mark.parametrize(("method", "path", "request_kwargs", "names"), _CLOSED)
def test_a_closed_route_refuses_whatever_it_is_sent(client, method, path, request_kwargs, names):
    """No body is read or parsed: what once was a 400 (a bogus role, an unparseable club), a 422
    (an unknown verdict) or a 404 (no such swing) is the same 410, since none of it is this
    server's to judge any more."""
    del request_kwargs, names
    junk = {} if method == "delete" else {"content": b"not json at all"}

    res = getattr(client, method)(path, **junk)

    assert res.status_code == 410


@pytest.mark.parametrize(("method", "path", "request_kwargs", "names"), _CLOSED)
def test_a_closed_route_is_still_behind_the_upload_token(
    auth_client, method, path, request_kwargs, names
) -> None:
    """The guard resolves before the handler, so a closed route tells an anonymous caller nothing.
    With the token it reaches the refusal, header or `?t=` alike."""
    del names
    assert getattr(auth_client, method)(path, **request_kwargs).status_code == 401
    wrong = {"headers": {"X-Upload-Token": "not-the-token"}}
    assert getattr(auth_client, method)(path, **request_kwargs, **wrong).status_code == 401

    right = {"headers": {"X-Upload-Token": _TOKEN}}
    assert getattr(auth_client, method)(path, **request_kwargs, **right).status_code == 410
    params = {**request_kwargs.get("params", {}), "t": _TOKEN}
    by_query = {key: value for key, value in request_kwargs.items() if key != "params"}
    assert getattr(auth_client, method)(path, params=params, **by_query).status_code == 410


def test_a_refused_upload_writes_nothing_to_disk(client, tmp_path) -> None:
    """The load-bearing half of the closure for a 2 GiB clip: the body is never streamed, so
    nothing lands in `.incoming` to be orphaned."""
    client.post(
        "/api/uploads",
        params={"role": "face_on", "filename": "x.mov"},
        content=b"x" * (2 * 1024 * 1024),
    )

    incoming = tmp_path / ".incoming"
    assert not incoming.exists() or not list(incoming.iterdir())


def test_an_unauthenticated_upload_writes_nothing_to_disk(auth_client, tmp_path) -> None:
    # The guard is a route dependency precisely so it runs before the handler.
    auth_client.post(
        "/api/uploads",
        params={"role": "face_on", "filename": "x.mov"},
        content=b"x" * (2 * 1024 * 1024),
    )

    incoming = tmp_path / ".incoming"
    assert not incoming.exists() or not list(incoming.iterdir())


@pytest.mark.parametrize("segment", ["..", ".hidden", "with space"])
def test_the_closed_delete_still_refuses_a_traversing_id(client, segment) -> None:
    """`_safe` still runs first at the swing routes, so the ids the refusal echoes into its command
    are ones the store could have named. A bare `..` is normalised away by the client before it
    reaches routing, which is why the accepted answers include the redirect/405 the router gives
    it."""
    res = client.delete(f"/api/sessions/2026-08-06/swings/{segment}")
    assert res.status_code in (400, 404, 405, 307)


# --------------------------------------------------------------------------- the read routes


def test_session_current_and_detail_reflect_filed_swings(client, store, file_swing) -> None:
    file_swing(store, "face_on", b"a")
    file_swing(store, "down_the_line", b"b")

    current = client.get("/api/sessions/current").json()
    detail = client.get(f"/api/sessions/{current['session_id']}").json()

    assert len(detail["swings"]) == 1
    swing = detail["swings"][0]
    assert swing["roles"]["face_on"] is not None
    assert swing["roles"]["down_the_line"] is not None
    assert swing["roles"]["shot_screen"] is None


def test_the_read_routes_report_a_stored_club(client, store, file_swing) -> None:
    """Wherever they already report `player_id`, the session and swing views report the club."""
    filed = file_swing(store, "face_on", b"data", club=ClubId.PITCHING_WEDGE)

    session_id = filed.session_id
    assert client.get(f"/api/sessions/{session_id}").json()["swings"][0]["club"] == "pw"
    assert client.get(f"/api/sessions/{session_id}/swings/1").json()["club"] == "pw"


def test_server_binds_loopback_only_by_default() -> None:
    # Regression guard against a future edit silently widening the bind address.
    source = (_REPO_ROOT / "scripts" / "run_server.py").read_text(encoding="utf-8")

    assert '_DEFAULT_HOST = "127.0.0.1"' in source


# --- Token auth (ADR-016) -----------------------------------------------------------
# Tailscale Funnel makes these routes publicly reachable, so the token is the only thing
# between the open internet and what the read routes serve.


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


# --------------------------------------------------------------------------- the two cursors
#
# Nothing writes a cursor here any more, and an older session's `session.json` still holds one:
# the read routes serve it, and the picker's bag follows its golfer.


@pytest.fixture
def golfer_client(tmp_path):
    """A client whose golfer registry is also isolated to tmp_path."""
    store = SwingBundleStore(tmp_path / "sessions")
    golfers = GolferStore(tmp_path / "golfers")
    return TestClient(create_app(store=store, golfers=golfers, token=None, worker=None))


def _point_the_cursor_at(tmp_path: Path, name: str, handedness: Handedness) -> str:
    """Register a golfer and point today's session at them, as the closed golfer route did."""
    golfer = GolferStore(tmp_path / "golfers").get_or_create(name, handedness)
    store = SwingBundleStore(tmp_path / "sessions")
    set_current_player(store.root / store.current_session_id(), golfer.player_id)
    return golfer.player_id


def test_no_golfer_is_selected_until_someone_picks_one(golfer_client) -> None:
    body = golfer_client.get("/api/sessions/current/golfer").json()

    assert body["player_id"] is None
    assert body["golfer"] is None


def test_a_stored_golfer_cursor_is_served(golfer_client, tmp_path) -> None:
    _point_the_cursor_at(tmp_path, "Aaron", Handedness.RIGHT)

    body = golfer_client.get("/api/sessions/current/golfer").json()

    assert body["golfer"] == {"player_id": "aaron", "display_name": "Aaron", "handedness": "right"}


def test_golfer_routes_are_behind_the_upload_token(auth_client) -> None:
    assert auth_client.get("/api/golfers").status_code == 401
    assert auth_client.get("/api/sessions/current/golfer").status_code == 401


def test_no_club_is_selected_until_someone_picks_one(client) -> None:
    body = client.get("/api/sessions/current/club").json()

    assert body["club"] is None


def test_club_routes_are_behind_the_upload_token(auth_client) -> None:
    assert auth_client.get("/api/sessions/current/club").status_code == 401


def test_the_cursor_reads_never_write_a_session(client, tmp_path) -> None:
    client.get("/api/sessions/current/golfer")
    client.get("/api/sessions/current/club")

    assert _every_file(tmp_path) == {}


# ------------------------------------------------------------------ the picker's inputs (M9 P7)
#
# `GET /api/clubs` exists so `static/index.html` never holds a second copy of the club taxonomy.
# The tests below therefore assert against `ClubId` itself rather than against a written-out list:
# a literal here would be the very duplicate the route was added to prevent, and it would pass
# happily on the day a club is added to the enum and forgotten everywhere else.


def test_the_picker_is_offered_every_club_in_canonical_order(client) -> None:
    body = client.get("/api/clubs").json()

    assert body["clubs"] == [club.value for club in ClubId]


def test_every_club_the_picker_offers_is_one_the_parser_accepts(client) -> None:
    """The pin that makes the picker's list and `parse_club` one vocabulary.

    It ran through the club cursor until M29 closed it; the bag routes parse the same way
    (`_resolve_club`), and `golf-core set-club` through Rust's twin of the same parser. Without it
    the two can drift in either direction — a club served but unparseable, or a rename landing in
    `ClubId` and reaching the page as a tap that is refused.
    """
    for club in client.get("/api/clubs").json()["clubs"]:
        assert parse_club(club) == ClubId(club), club


def test_a_golfer_with_no_bag_gets_the_taxonomy_and_nothing_else(golfer_client, tmp_path) -> None:
    """`bag` is empty rather than absent: the page renders one section instead of two."""
    _point_the_cursor_at(tmp_path, "Aaron", Handedness.RIGHT)

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
    _point_the_cursor_at(tmp_path, "Aaron", Handedness.RIGHT)
    bags = BagStore(tmp_path / "golfers")
    for club in (ClubId.PITCHING_WEDGE, ClubId.DRIVER, ClubId.SEVEN_IRON):
        bags.set_entry("aaron", BagEntry(club=club, recorded_at=datetime.now(tz=UTC)))

    assert golfer_client.get("/api/clubs").json()["bag"] == ["driver", "7i", "pw"]


def test_one_golfers_bag_is_not_another_golfers(golfer_client, tmp_path) -> None:
    """The bag follows the session's golfer cursor, so a cursor pointed elsewhere changes it."""
    bags = BagStore(tmp_path / "golfers")
    bags.set_entry("aaron", BagEntry(club=ClubId.SEVEN_IRON, recorded_at=datetime.now(tz=UTC)))

    _point_the_cursor_at(tmp_path, "Aaron", Handedness.RIGHT)
    assert golfer_client.get("/api/clubs").json()["bag"] == ["7i"]

    _point_the_cursor_at(tmp_path, "Rory", Handedness.LEFT)
    assert golfer_client.get("/api/clubs").json()["bag"] == []


def test_an_unreadable_bag_costs_the_shortcut_and_not_the_picker(golfer_client, tmp_path) -> None:
    """`BagStore.get`'s tolerance, at the route. A corrupt bag must not 500 the page someone is
    holding between swings — it costs the "in the bag" section and nothing else."""
    _point_the_cursor_at(tmp_path, "Aaron", Handedness.RIGHT)
    (tmp_path / "golfers" / "aaron.bag.json").write_text("{ not json", encoding="utf-8")

    res = golfer_client.get("/api/clubs")

    assert res.status_code == 200
    assert res.json()["bag"] == []
    assert res.json()["clubs"] == [club.value for club in ClubId]


def test_the_picker_route_is_behind_the_upload_token(auth_client) -> None:
    assert auth_client.get("/api/clubs").status_code == 401
