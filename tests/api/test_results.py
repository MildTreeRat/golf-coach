"""The read path a results page depends on: the swing detail route, and serving the video.

These seed `analysis.json` and `analysis.state.json` onto disk directly rather than running a
pipeline. That is the point of the sidecar being a plain file — the render path is testable on
the base install with no worker, no threads, and no cv2.
"""

from __future__ import annotations

import json

import pytest
from fastapi.testclient import TestClient

from golf_coach.api.app import create_app
from golf_coach.api.state import AnalysisState, save_state
from golf_coach.storage.bundle_store import SwingBundleStore

_TOKEN = "s3cret-token"
_ROLES = ("face_on", "down_the_line", "shot_screen")

_ANALYSIS = {
    "swing_id": "1",
    "session_id": "seeded",
    "swing": {
        "swing_id": "1",
        "session_id": "seeded",
        "overall_score": 86.0,
        "mechanics_score": 86.0,
        "checkpoint_scores": [
            {
                "name": "tempo", "score": 0.58, "passed": False, "observed": 1.89,
                "expected_low": 2.72, "expected_high": 4.71, "percentile": 10.0,
                "population_n": 1399, "one_sided": False, "message": "Tempo too quick",
            }
        ],
        "unscored": [],
        "shot": {"shot_id": "seeded-1", "session_id": "seeded", "source": "screen",
                 "carry_distance": 195.9, "ball_speed": 142.1},
    },
    "notes": ["the face_on clip claims 400 frames but only 334 decoded"],
    "feedback": {
        "swing_id": "1", "overall_score": 86.0, "headline": "Work on tempo first.",
        "tips": [{"checkpoint": "tempo", "text": "Tempo too quick", "severity": "minor"}],
    },
}


@pytest.fixture
def store(tmp_path):
    return SwingBundleStore(tmp_path)


@pytest.fixture
def client(store):
    # worker=None: these are read-path tests, and a running consumer would race the seeding.
    return TestClient(create_app(store=store, token=None, worker=None))


def _seed(client, store, *, roles=_ROLES, analysis=True, video=False, state_kwargs=None):
    """Upload `roles`, then drop analysis artifacts beside them as a worker would have."""
    for role in roles:
        res = client.post(
            "/api/uploads", params={"role": role, "filename": f"{role}.mov"},
            content=role.encode(),
        )
    session_id = res.json()["session_id"]
    swing_dir = store.root / session_id / "1"

    if analysis:
        (swing_dir / "analysis.json").write_text(json.dumps(_ANALYSIS), encoding="utf-8")
    if video:
        (swing_dir / "aligned.mp4").write_bytes(b"\x00" * 4096)
    fields = {
        "status": "done",
        "inputs": {},
        "score": 86.0,
        "headline": "Work on tempo first.",
        "video": "aligned.mp4" if video else None,
        "video_codec": "avc1" if video else None,
    }
    fields.update(state_kwargs or {})
    save_state(AnalysisState(**fields), swing_dir)
    return session_id


def test_swing_detail_returns_the_stored_result(client, store) -> None:
    session_id = _seed(client, store)

    body = client.get(f"/api/sessions/{session_id}/swings/1").json()

    assert body["status"] == "complete"
    assert body["analysis"]["status"] == "done"
    assert body["analysis"]["score"] == 86.0
    assert body["result"]["swing"]["overall_score"] == 86.0
    assert body["result"]["feedback"]["tips"][0]["severity"] == "minor"
    assert body["result"]["notes"][0].startswith("the face_on clip claims")


def test_session_detail_carries_the_analysis_block(client, store) -> None:
    # This is what the upload page polls to decide whether to show a results link at all.
    session_id = _seed(client, store)

    swing = client.get(f"/api/sessions/{session_id}").json()["swings"][0]

    assert swing["analysis"]["status"] == "done"
    assert swing["analysis"]["score"] == 86.0
    assert swing["analysis"]["headline"] == "Work on tempo first."


def test_analysis_block_reads_none_before_any_run(client, store) -> None:
    client.post("/api/uploads", params={"role": "face_on", "filename": "x.mov"}, content=b"a")
    session_id = client.get("/api/sessions/current").json()["session_id"]

    swing = client.get(f"/api/sessions/{session_id}").json()["swings"][0]

    assert swing["analysis"]["status"] == "none"
    assert swing["analysis"]["has_video"] is False


def test_swing_detail_serves_a_waiting_state_before_the_result_exists(client, store) -> None:
    session_id = _seed(client, store, roles=("face_on",), analysis=False,
                       state_kwargs={"status": "running"})

    body = client.get(f"/api/sessions/{session_id}/swings/1").json()

    assert body["result"] is None
    assert body["analysis"]["status"] == "running"
    assert sorted(body["missing_roles"]) == ["down_the_line", "shot_screen"]


def test_swing_detail_404s_for_an_unknown_swing(client) -> None:
    assert client.get("/api/sessions/2026-01-01/swings/9").status_code == 404


@pytest.mark.parametrize("segment", ["..", "../etc", ".hidden", "a/b", "with space"])
def test_path_traversal_and_junk_segments_are_rejected(client, segment) -> None:
    # `..` is a single path segment, so it matches `{session_id}` and would otherwise be
    # joined straight onto sessions_dir.
    assert client.get(f"/api/sessions/{segment}").status_code in (400, 404)
    assert client.get(f"/api/sessions/{segment}/swings/1").status_code in (400, 404)


def test_video_is_served_and_supports_range(client, store) -> None:
    session_id = _seed(client, store, video=True)

    whole = client.get(f"/api/sessions/{session_id}/swings/1/video/aligned")
    assert whole.status_code == 200
    assert whole.headers["content-type"] == "video/mp4"
    # iOS Safari will not play a <video> whose source cannot serve ranges.
    assert whole.headers.get("accept-ranges") == "bytes"

    part = client.get(
        f"/api/sessions/{session_id}/swings/1/video/aligned",
        headers={"Range": "bytes=0-99"},
    )
    assert part.status_code == 206
    assert len(part.content) == 100


def test_video_404s_when_there_is_no_aligned_render(client, store) -> None:
    session_id = _seed(client, store, video=False)

    res = client.get(f"/api/sessions/{session_id}/swings/1/video/aligned")

    assert res.status_code == 404


def test_raw_view_is_served_as_the_fallback(client, store) -> None:
    # A face-on-only bundle produces no aligned render, so the page falls back to the clip.
    session_id = _seed(client, store, roles=("face_on",), analysis=False)

    res = client.get(f"/api/sessions/{session_id}/swings/1/video/face_on")

    assert res.status_code == 200
    assert res.headers["content-type"] == "video/quicktime"
    assert res.content == b"face_on"


def test_unknown_artifact_names_are_not_served(client, store) -> None:
    session_id = _seed(client, store)

    for name in ("manifest.json", "analysis.json", "face_on.keypoints.json", "..%2Fmanifest"):
        res = client.get(f"/api/sessions/{session_id}/swings/1/video/{name}")
        assert res.status_code == 404, name


def test_video_route_accepts_the_token_as_a_query_param(store) -> None:
    # A <video> element sends no custom headers, so `?t=` is the only way it can authenticate.
    client = TestClient(create_app(store=store, token=_TOKEN, worker=None))
    for role in _ROLES:
        res = client.post(
            "/api/uploads", params={"role": role, "filename": f"{role}.mov", "t": _TOKEN},
            content=role.encode(),
        )
    session_id = res.json()["session_id"]
    (store.root / session_id / "1" / "aligned.mp4").write_bytes(b"\x00" * 128)

    url = f"/api/sessions/{session_id}/swings/1/video/aligned"
    assert client.get(url).status_code == 401
    assert client.get(url, params={"t": _TOKEN}).status_code == 200


def test_results_page_is_reachable(client) -> None:
    res = client.get("/results.html")

    assert res.status_code == 200
    assert "Aligned views" in res.text
