"""`POST /api/sessions/{session_id}/swings/{swing_id}/mishit` — the golfer's verdict. [M16 P5]

The exclusion arithmetic is pinned over contracts in `tests/storage/test_corpus.py` and
`tests/analysis/test_club_profile_builder.py`; `bundle_store.set_mishit` in
`tests/storage/test_bundle_store.py`. What this file checks is the envelope and the two gates: a
verdict reaches disk, an unknown swing is a 404, and a swing with no shot screen is a 409 — a
mishit on a swing with no ball flight has nothing to act on.

Disk state is built by upload rather than by a pipeline, the standing shape in `tests/api/`.
"""

from __future__ import annotations

import pytest
from fastapi.testclient import TestClient

from golf_coach.api.app import create_app
from golf_coach.contracts.golfer import Handedness
from golf_coach.contracts.mishit import MishitVerdict
from golf_coach.storage.bundle_store import SwingBundleStore
from golf_coach.storage.golfer_store import GolferStore
from golf_coach.storage.manifest import Role


@pytest.fixture
def store(tmp_path) -> SwingBundleStore:
    return SwingBundleStore(tmp_path / "sessions")


@pytest.fixture
def golfers(tmp_path) -> GolferStore:
    store = GolferStore(tmp_path / "golfers")
    store.get_or_create("Aaron", Handedness.RIGHT)
    return store


@pytest.fixture
def client(store, golfers) -> TestClient:
    return TestClient(create_app(store=store, golfers=golfers, token=None, worker=None))


def _seed_swing(client: TestClient, *, tag: str, with_shot: bool = True) -> tuple[str, str]:
    """One swing, tagged with a club, with or without its shot-screen photo."""
    client.post("/api/sessions/current/golfer", json={"name": "Aaron", "handedness": "right"})
    client.post("/api/sessions/current/club", json={"club": "7 iron"})
    roles = ("face_on", "down_the_line", "shot_screen") if with_shot else ("face_on",)
    res = None
    for role in roles:
        res = client.post(
            "/api/uploads",
            params={"role": role, "filename": f"{role}.mov"},
            content=f"{role}-{tag}".encode(),
        )
    assert res is not None
    body = res.json()
    return body["session_id"], body["swing_id"]


def test_a_verdict_reaches_the_manifest_and_null_clears_it(client, store) -> None:
    session_id, swing_id = _seed_swing(client, tag="a")

    res = client.post(
        f"/api/sessions/{session_id}/swings/{swing_id}/mishit", json={"verdict": "confirmed"}
    )
    assert res.status_code == 200
    assert res.json()["mishit"] == "confirmed"
    assert store.get_swing(session_id, swing_id).mishit is MishitVerdict.CONFIRMED

    res = client.post(
        f"/api/sessions/{session_id}/swings/{swing_id}/mishit", json={"verdict": None}
    )
    assert res.status_code == 200
    assert res.json()["mishit"] is None
    assert store.get_swing(session_id, swing_id).mishit is None


def test_cleared_is_accepted_too(client, store) -> None:
    session_id, swing_id = _seed_swing(client, tag="b")
    res = client.post(
        f"/api/sessions/{session_id}/swings/{swing_id}/mishit", json={"verdict": "cleared"}
    )
    assert res.status_code == 200
    assert store.get_swing(session_id, swing_id).mishit is MishitVerdict.CLEARED


def test_an_unknown_verdict_is_rejected(client) -> None:
    session_id, swing_id = _seed_swing(client, tag="c")
    res = client.post(
        f"/api/sessions/{session_id}/swings/{swing_id}/mishit", json={"verdict": "topped"}
    )
    assert res.status_code == 422


def test_an_unknown_swing_is_a_404(client) -> None:
    res = client.post(
        "/api/sessions/2026-08-10/swings/99/mishit", json={"verdict": "confirmed"}
    )
    assert res.status_code == 404


def test_a_swing_with_no_shot_screen_is_a_409(client, store) -> None:
    session_id, swing_id = _seed_swing(client, tag="d", with_shot=False)
    assert Role.SHOT_SCREEN not in store.get_swing(session_id, swing_id).roles

    res = client.post(
        f"/api/sessions/{session_id}/swings/{swing_id}/mishit", json={"verdict": "confirmed"}
    )
    assert res.status_code == 409
    assert store.get_swing(session_id, swing_id).mishit is None
