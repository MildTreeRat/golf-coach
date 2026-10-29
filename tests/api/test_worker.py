"""The background worker: what triggers it, what doesn't, and what a failure leaves behind.

Every test here runs on the **base install**. The worker takes its `runner` by injection
precisely so this file can drive the whole lifecycle — queue, threading, state transitions,
idempotence — without cv2, mediapipe or paddleocr being importable at all.

**Since M29 no route submits to it.** Its two callers were the upload route, on the third role
landing, and "Analyze anyway"; the M29 plan's D11 closed both. The app still starts it, and its
startup recovery is what still reaches it there, so these tests drive it the way it can still be
reached: `start()` on an event loop of their own, `submit()` called from that loop as its docstring
requires, and the recovery pass `start()` runs. Swings are filed through the frozen store
(`conftest.py`'s `file_swing`), as the upload route filed them. The last two tests are the app's
half: its lifespan starts the worker it was given, and starts none when given none.
"""

from __future__ import annotations

import asyncio
import threading
import time
from collections.abc import Awaitable, Callable

import pytest
from fastapi.testclient import TestClient

from golf_coach.api.app import create_app
from golf_coach.api.pipeline import PipelineOutcome, record_state
from golf_coach.api.state import AnalysisState, input_hashes, load_state, now, save_state
from golf_coach.api.worker import AnalysisWorker, should_analyze
from golf_coach.contracts.feedback import FeedbackPayload
from golf_coach.contracts.swing import SwingBundleResult, SwingResult
from golf_coach.storage.bundle_store import SwingBundleStore
from golf_coach.storage.manifest import load_manifest, manifest_path

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
    """Stands in for the pipeline. Records every call; optionally fails or blocks.

    **It writes the terminal state, because the pipeline does.** `record_state` is part of what a
    run leaves behind — the sidecar is a denormalised copy of `analysis.json`, so it belongs to
    whatever wrote that file — and a stand-in that skipped it would let these tests pass over a
    worker that never reached `done` in production.
    """

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
        outcome = self.outcome or PipelineOutcome(result=_result(), video_path=None)
        manifest = load_manifest(manifest_path(swing_dir))
        assert manifest is not None
        record_state(swing_dir, manifest, outcome, started_at=now())
        return outcome


@pytest.fixture
def store(tmp_path):
    return SwingBundleStore(tmp_path)


def _run(worker: AnalysisWorker, body: Callable[[], Awaitable[None]]) -> None:
    """Start `worker` on a loop of its own, run `body` on that loop, then stop it.

    One loop for all three, so every `submit` in `body` is made from the event-loop thread, as
    `AnalysisWorker.submit` requires; the runner itself runs in a thread off it, as in production.
    """

    async def main() -> None:
        await worker.start()
        try:
            await body()
        finally:
            await worker.stop()

    asyncio.run(main())


async def _wait_for(swing_dir, statuses, timeout: float = 5.0):
    """Poll the state sidecar until it reaches one of `statuses`, yielding to the loop between.

    The runner works in another thread, so a sleep-poll is the honest way to observe it, and an
    `asyncio.sleep` rather than a blocking one is what lets the consumer pick the job up at all.
    """
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        state = load_state(swing_dir)
        if state is not None and state.status in statuses:
            return state
        await asyncio.sleep(0.02)
    state = load_state(swing_dir)
    raise AssertionError(f"never reached {statuses}; last state was {state!r}")


def _file_bundle(file_swing, store, roles=_ROLES) -> str:
    for i, role in enumerate(roles):
        filed = file_swing(store, role, f"clip-{i}-{role}".encode())
    return filed.session_id


def test_a_complete_bundle_is_the_automatic_trigger(store, tmp_path, file_swing) -> None:
    """`should_analyze` is the rule, and recovery at start is what still applies it: a complete
    bundle with no result is queued and analysed without anyone asking."""
    runner = RecordingRunner()
    session_id = _file_bundle(file_swing, store)
    assert should_analyze(store.get_swing(session_id, "1"))

    async def body() -> None:
        state = await _wait_for(tmp_path / session_id / "1", {"done"})
        assert state.score == 84.0
        assert state.headline == "Work on tempo first."
        assert state.partial is False

    _run(AnalysisWorker(store, runner=runner), body)

    assert len(runner.calls) == 1


def test_an_incomplete_bundle_does_not_trigger_analysis(store, file_swing) -> None:
    # The whole trigger policy in one test: two of three roles starts nothing at all.
    runner = RecordingRunner()
    session_id = _file_bundle(file_swing, store, roles=_ROLES[:2])
    assert not should_analyze(store.get_swing(session_id, "1"))

    async def body() -> None:
        await asyncio.sleep(0.2)

    _run(AnalysisWorker(store, runner=runner), body)

    assert runner.calls == []


def test_force_runs_a_partial_bundle(store, tmp_path, file_swing) -> None:
    """What "Analyze anyway" reached: `submit(force=True)` over a bundle that will never be
    complete, recorded as partial with the roles it lacked."""
    runner = RecordingRunner()
    worker = AnalysisWorker(store, runner=runner)
    session_id = _file_bundle(file_swing, store, roles=("face_on",))

    async def body() -> None:
        assert runner.calls == []
        assert worker.submit(session_id, "1", force=True) is True
        state = await _wait_for(tmp_path / session_id / "1", {"done"})
        assert state.partial is True
        assert sorted(state.missing_roles) == ["down_the_line", "shot_screen"]

    _run(worker, body)

    assert len(runner.calls) == 1


def test_an_unknown_swing_is_not_queued(store, file_swing) -> None:
    worker = AnalysisWorker(store, runner=RecordingRunner())
    _file_bundle(file_swing, store, roles=("face_on",))

    async def body() -> None:
        assert worker.submit("2026-01-01", "99", force=True) is False

    _run(worker, body)


def test_failure_is_recorded_and_the_consumer_survives(store, tmp_path, file_swing) -> None:
    runner = RecordingRunner(error=RuntimeError("mediapipe exploded"))
    worker = AnalysisWorker(store, runner=runner)
    session_id = _file_bundle(file_swing, store, roles=("face_on",))

    async def body() -> None:
        worker.submit(session_id, "1", force=True)
        state = await _wait_for(tmp_path / session_id / "1", {"failed"})
        assert state.error == "RuntimeError: mediapipe exploded"

        # The queue must still be alive: a second job has to be picked up.
        runner.error = None
        file_swing(store, "face_on", b"different", swing_id="2")
        worker.submit(session_id, "2", force=True)
        second = await _wait_for(tmp_path / session_id / "2", {"done"})
        assert second.status == "done"

    _run(worker, body)

    assert len(runner.calls) == 2


def test_pipeline_returning_no_result_is_a_failure_not_a_crash(
    store, tmp_path, file_swing
) -> None:
    runner = RecordingRunner(outcome=PipelineOutcome(error="no usable face-on view"))
    worker = AnalysisWorker(store, runner=runner)
    session_id = _file_bundle(file_swing, store, roles=("face_on",))

    async def body() -> None:
        worker.submit(session_id, "1", force=True)
        state = await _wait_for(tmp_path / session_id / "1", {"failed"})
        assert state.error == "no usable face-on view"

    _run(worker, body)


def test_unchanged_bundle_is_not_reanalyzed(store, tmp_path, file_swing) -> None:
    runner = RecordingRunner()
    worker = AnalysisWorker(store, runner=runner)
    session_id = _file_bundle(file_swing, store)

    async def body() -> None:
        await _wait_for(tmp_path / session_id / "1", {"done"})
        # Same bytes, same hashes: the stored result still describes them.
        assert worker.submit(session_id, "1") is False
        await asyncio.sleep(0.1)

    _run(worker, body)

    assert len(runner.calls) == 1


def test_a_rerecorded_clip_invalidates_the_result(store, tmp_path, file_swing) -> None:
    runner = RecordingRunner()
    worker = AnalysisWorker(store, runner=runner)
    session_id = _file_bundle(file_swing, store)

    async def body() -> None:
        first = await _wait_for(tmp_path / session_id / "1", {"done"})
        # Re-file face_on with different bytes into the same swing. The hash moves, so the
        # recorded inputs no longer match and the result is stale by construction.
        file_swing(store, "face_on", b"a better take", swing_id="1")
        assert not first.matches(store.get_swing(session_id, "1"))
        assert worker.submit(session_id, "1") is True
        await _wait_for(tmp_path / session_id / "1", {"queued", "running"})
        state = await _wait_for(tmp_path / session_id / "1", {"done"})
        assert state.matches(store.get_swing(session_id, "1"))

    _run(worker, body)

    assert len(runner.calls) == 2


def test_the_loop_is_not_blocked_while_a_run_is_parked(store, tmp_path, file_swing) -> None:
    """The reason the pipeline runs in a thread: a 4-minute pose must not stall the server's loop,
    which the read routes answer on."""
    runner = RecordingRunner()
    runner.gate = threading.Event()
    worker = AnalysisWorker(store, runner=runner)
    session_id = _file_bundle(file_swing, store)

    async def body() -> None:
        await _wait_for(tmp_path / session_id / "1", {"running"})
        # Analysis is now parked inside the runner. The loop must still answer immediately.
        started = time.monotonic()
        await asyncio.sleep(0.01)
        file_swing(store, "face_on", b"next swing", swing_id="2")
        assert worker.submit(session_id, "2", force=True) is True
        assert time.monotonic() - started < 2.0
        runner.gate.set()
        await _wait_for(tmp_path / session_id / "1", {"done"})
        await _wait_for(tmp_path / session_id / "2", {"done"})

    _run(worker, body)


@pytest.mark.parametrize("status", ["queued", "running"])
def test_recovery_reruns_what_a_previous_process_left_unfinished(
    store, tmp_path, file_swing, status
) -> None:
    """No consumer survives a restart, so a `queued` or `running` state at start is orphaned, and
    recovery runs it again, forced, even for a bundle that is not complete."""
    runner = RecordingRunner()
    session_id = _file_bundle(file_swing, store, roles=("face_on",))
    manifest = store.get_swing(session_id, "1")
    save_state(
        AnalysisState(status=status, inputs=input_hashes(manifest), queued_at=now()),
        tmp_path / session_id / "1",
    )

    async def body() -> None:
        state = await _wait_for(tmp_path / session_id / "1", {"done"})
        assert state.partial is True

    _run(AnalysisWorker(store, runner=runner), body)

    assert len(runner.calls) == 1


def test_the_app_starts_the_worker_it_is_given(store, tmp_path, file_swing) -> None:
    """The lifespan is the one place the closed server still reaches the worker: a complete bundle
    with no result, on disk when the server starts, is analysed by recovery."""
    runner = RecordingRunner()
    session_id = _file_bundle(file_swing, store)

    worker = AnalysisWorker(store, runner=runner)

    with TestClient(create_app(store=store, token=None, worker=worker)):
        deadline = time.monotonic() + 5.0
        while time.monotonic() < deadline and not runner.calls:
            time.sleep(0.02)
        while time.monotonic() < deadline:
            state = load_state(tmp_path / session_id / "1")
            if state is not None and state.status == "done":
                break
            time.sleep(0.02)

    assert len(runner.calls) == 1
    assert load_state(tmp_path / session_id / "1").status == "done"


def test_an_app_with_no_worker_analyses_nothing(store, tmp_path, file_swing) -> None:
    session_id = _file_bundle(file_swing, store)

    with TestClient(create_app(store=store, token=None, worker=None)) as client:
        assert client.get(f"/api/sessions/{session_id}").status_code == 200
        time.sleep(0.2)

    assert load_state(tmp_path / session_id / "1") is None
