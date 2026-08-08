"""Upload API — role-based ingestion, session status, token auth, and the bind guard."""

from __future__ import annotations

import hashlib
from pathlib import Path

import pytest
from fastapi.testclient import TestClient

from golf_coach.api.app import create_app
from golf_coach.storage.bundle_store import SwingBundleStore

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


def test_upload_returns_swing_and_status(client) -> None:
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
    data = b"same-bytes"
    client.post("/api/uploads", params={"role": "face_on", "filename": "x.mov"}, content=data)

    res = client.post("/api/uploads", params={"role": "face_on", "filename": "x.mov"}, content=data)

    assert res.json()["deduped"] is True


def test_large_payload_round_trips_with_correct_hash(client, tmp_path) -> None:
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
