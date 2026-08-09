"""The background worker: what triggers it, what doesn't, and what a failure leaves behind.

Every test here runs on the **base install**. The worker takes its `runner` by injection
precisely so this file can drive the whole lifecycle — queue, threading, state transitions,
idempotence — without cv2, mediapipe or paddleocr being importable at all.

Note the `with TestClient(...)` form throughout: without it Starlette never runs lifespan, the
consumer task is never spawned, and `submit()` is inert. `tests/api/test_uploads.py` relies on
exactly that to stay a pure ingestion test.
"""

from __future__ import annotations

import threading
import time

import pytest
from fastapi.testclient import TestClient

from golf_coach.api.app import create_app
from golf_coach.api.pipeline import PipelineOutcome
from golf_coach.api.state import load_state
from golf_coach.api.worker import AnalysisWorker
from golf_coach.contracts.feedback import FeedbackPayload
from golf_coach.contracts.swing import SwingBundleResult, SwingResult
from golf_coach.storage.bundle_store import SwingBundleStore

_ROLES = ("face_on", "down_the_line", "shot_screen")


def _result(swing_id: str = "1", session_id: str = "s", score: float = 84.0):
    return SwingBundleResult(
        swing_id=swing_id,
        session_id=session_id,
        swing=SwingResult(swing_id=swing_id, session_id=session_id, overall_score=score),
        feedback=FeedbackPayload(
            swing_id=swing_id, overall_score=score, headline="Work on tempo first."
        ),
    )


class RecordingRunner:
    """Stands in for the pipeline. Records every call; optionally fails or blocks."""

    def __init__(self, *, outcome=None, error=None):
        self.calls: list = []
        self.outcome = outcome
        self.error = error
        self.gate: threading.Event | None = None

    def __call__(self, swing_dir, options, log):
        self.calls.append(swing_dir)
        if self.gate is not None:
            self.gate.wait(timeout=5)
        if self.error is not None:
            raise self.error
        return self.outcome or PipelineOutcome(result=_result(), video_path=None)


@pytest.fixture
def store(tmp_path):
    return SwingBundleStore(tmp_path)


def _client(store, runner, **kwargs):
    worker = AnalysisWorker(store, runner=runner, **kwargs)
    return TestClient(create_app(store=store, token=None, worker=worker)), worker


def _upload(client, role: str, content: bytes = b"bytes", swing_id: str | None = None):
    params = {"role": role, "filename": f"{role}.mov"}
    if swing_id is not None:
        params["swing_id"] = swing_id
    return client.post("/api/uploads", params=params, content=content)


def _wait_for(swing_dir, statuses, timeout: float = 5.0):
    """Poll the state sidecar until it reaches a terminal status.

    The worker runs on TestClient's event loop in another thread, so a plain sleep-poll from
    the test thread is the honest way to observe it.
    """
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        state = load_state(swing_dir)
        if state is not None and state.status in statuses:
            return state
        time.sleep(0.02)
    state = load_state(swing_dir)
    raise AssertionError(f"never reached {statuses}; last state was {state!r}")


def test_complete_bundle_triggers_analysis(store, tmp_path) -> None:
    runner = RecordingRunner()
    client, _ = _client(store, runner)

    with client:
        for i, role in enumerate(_ROLES):
            res = _upload(client, role, content=f"clip-{i}".encode())
        assert res.json()["queued"] is True
        state = _wait_for(tmp_path / res.json()["session_id"] / "1", {"done"})

    assert len(runner.calls) == 1
    assert state.status == "done"
    assert state.score == 84.0
    assert state.headline == "Work on tempo first."
    assert state.partial is False


def test_incomplete_bundle_does_not_trigger_analysis(store) -> None:
    # The whole trigger policy in one test: two of three roles starts nothing at all.
    runner = RecordingRunner()
    client, _ = _client(store, runner)

    with client:
        _upload(client, "face_on")
        res = _upload(client, "down_the_line")
        assert res.json()["status"] == "collecting"
        assert res.json()["queued"] is False
        time.sleep(0.2)

    assert runner.calls == []


def test_analyze_anyway_runs_a_partial_bundle(store, tmp_path) -> None:
    runner = RecordingRunner()
    client, _ = _client(store, runner)

    with client:
        session_id = _upload(client, "face_on").json()["session_id"]
        assert runner.calls == []

        res = client.post(f"/api/sessions/{session_id}/swings/1/analyze")
        assert res.status_code == 200
        assert sorted(res.json()["missing_roles"]) == ["down_the_line", "shot_screen"]
        state = _wait_for(tmp_path / session_id / "1", {"done"})

    assert len(runner.calls) == 1
    assert state.partial is True
    assert sorted(state.missing_roles) == ["down_the_line", "shot_screen"]


def test_analyze_anyway_rejects_a_bundle_with_no_face_on(store) -> None:
    # Every checkpoint is measured from the face-on view, so this cannot produce a score.
    runner = RecordingRunner()
    client, _ = _client(store, runner)

    with client:
        session_id = _upload(client, "down_the_line").json()["session_id"]
        res = client.post(f"/api/sessions/{session_id}/swings/1/analyze")

    assert res.status_code == 400
    assert "face-on" in res.json()["detail"]
    assert runner.calls == []


def test_analyze_anyway_404s_for_an_unknown_swing(store) -> None:
    client, _ = _client(store, RecordingRunner())

    with client:
        _upload(client, "face_on")
        res = client.post("/api/sessions/2026-01-01/swings/99/analyze")

    assert res.status_code == 404


def test_failure_is_recorded_and_the_consumer_survives(store, tmp_path) -> None:
    runner = RecordingRunner(error=RuntimeError("mediapipe exploded"))
    client, _ = _client(store, runner)

    with client:
        session_id = _upload(client, "face_on").json()["session_id"]
        client.post(f"/api/sessions/{session_id}/swings/1/analyze")
        state = _wait_for(tmp_path / session_id / "1", {"failed"})
        assert state.error == "RuntimeError: mediapipe exploded"

        # The queue must still be alive: a second job has to be picked up. Submitted over HTTP
        # rather than by calling `worker.submit` directly, because that is the only way it is
        # ever reached in production — see the note on `AnalysisWorker.submit`.
        runner.error = None
        _upload(client, "face_on", content=b"different", swing_id="2")
        client.post(f"/api/sessions/{session_id}/swings/2/analyze")
        second = _wait_for(tmp_path / session_id / "2", {"done"})

    assert second.status == "done"
    assert len(runner.calls) == 2


def test_pipeline_returning_no_result_is_a_failure_not_a_crash(store, tmp_path) -> None:
    runner = RecordingRunner(outcome=PipelineOutcome(error="no usable face-on view"))
    client, _ = _client(store, runner)

    with client:
        session_id = _upload(client, "face_on").json()["session_id"]
        client.post(f"/api/sessions/{session_id}/swings/1/analyze")
        state = _wait_for(tmp_path / session_id / "1", {"failed"})

    assert state.error == "no usable face-on view"


def test_unchanged_bundle_is_not_reanalyzed(store, tmp_path) -> None:
    runner = RecordingRunner()
    client, worker = _client(store, runner)

    with client:
        for role in _ROLES:
            res = _upload(client, role, content=role.encode())
        session_id = res.json()["session_id"]
        _wait_for(tmp_path / session_id / "1", {"done"})

        # Same bytes, same hashes: the stored result still describes them.
        assert worker.submit(session_id, "1") is False
        time.sleep(0.1)

    assert len(runner.calls) == 1


def test_a_rerecorded_clip_invalidates_the_result(store, tmp_path) -> None:
    runner = RecordingRunner()
    client, _ = _client(store, runner)

    with client:
        for role in _ROLES:
            res = _upload(client, role, content=role.encode())
        session_id = res.json()["session_id"]
        _wait_for(tmp_path / session_id / "1", {"done"})

        # Re-upload face_on with different bytes into the same swing. The hash moves, so the
        # recorded inputs no longer match and the result is stale by construction.
        _upload(client, "face_on", content=b"a better take", swing_id="1")
        state = _wait_for(tmp_path / session_id / "1", {"done"}, timeout=5.0)
        assert state.status == "done"

    assert len(runner.calls) == 2


def test_uploads_do_not_block_on_analysis(store, tmp_path) -> None:
    """The reason the pipeline runs in a thread: a 4-minute pose must not stall ingestion."""
    runner = RecordingRunner()
    runner.gate = threading.Event()
    client, _ = _client(store, runner)

    with client:
        for role in _ROLES:
            res = _upload(client, role, content=role.encode())
        session_id = res.json()["session_id"]
        _wait_for(tmp_path / session_id / "1", {"running"})

        # Analysis is now parked inside the runner. Ingestion must still answer immediately.
        started = time.monotonic()
        second = _upload(client, "face_on", content=b"next swing", swing_id="2")
        elapsed = time.monotonic() - started

        assert second.status_code == 200
        assert elapsed < 2.0
        assert client.get("/api/sessions/current").status_code == 200
        runner.gate.set()
        _wait_for(tmp_path / session_id / "1", {"done"})


def test_worker_disabled_leaves_ingestion_working(store) -> None:
    client = TestClient(create_app(store=store, token=None, worker=None))

    with client:
        for role in _ROLES:
            res = _upload(client, role, content=role.encode())

    assert res.status_code == 200
    assert res.json()["queued"] is False


def test_analyze_route_503s_when_the_worker_is_disabled(store) -> None:
    client = TestClient(create_app(store=store, token=None, worker=None))

    with client:
        session_id = _upload(client, "face_on").json()["session_id"]
        res = client.post(f"/api/sessions/{session_id}/swings/1/analyze")

    assert res.status_code == 503
