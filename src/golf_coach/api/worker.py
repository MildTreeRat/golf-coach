"""The background worker that joins ingestion to analysis. [M7 Phase 5]

Before this existed, an upload landed a file and stopped; somebody had to run
`scripts/analyze_bundle.py` by hand. This is the piece that closes the loop — the phone uploads
the last of the three files, and by the time you have walked back from the bay the results page
is populated.

**An in-process `asyncio.Queue` with one consumer.** One swing every couple of minutes does not
justify Celery, Redis, or a second process, and a task queue that shares the server's lifetime
has no broker to be down. Concurrency is 1 because the work is CPU-bound anyway: pose on a 4K
clip runs about 9.5 fps, so a 40-second view is ~4.5 minutes of MediaPipe, and running two at
once would only make both slower.

**Uploads must never wait on analysis.** The pipeline is synchronous, heavy, and imports OpenCV,
so it runs via `asyncio.to_thread`. Nothing on the event loop blocks: a phone can upload the next
swing while the previous one is still being posed.

**Analysis triggers only on a complete bundle.** Anything less is an explicit human decision —
the upload page's "Analyze anyway" button, which reaches `submit(force=True)`. A timer that
guessed when you were finished uploading would be wrong in both directions: too short and it
analyses a face-on clip alone while the second phone is still on the walk back, too long and it
sits idle after a swing you shot deliberately from one angle.

The `runner` seam exists so tests can drive all of this without cv2, mediapipe or paddleocr
anywhere near them.
"""

from __future__ import annotations

import asyncio
import logging
from collections.abc import Callable
from datetime import datetime
from pathlib import Path

from golf_coach.api.pipeline import Log, PipelineOptions, PipelineOutcome, analyze_swing_dir
from golf_coach.api.state import (
    AnalysisState,
    input_hashes,
    load_state,
    now,
    save_state,
)
from golf_coach.storage.bundle_store import SwingBundleStore
from golf_coach.storage.manifest import SwingManifest

logger = logging.getLogger(__name__)

#: What the worker actually calls to do the work. Swapped in tests.
Runner = Callable[[Path, PipelineOptions, Log], PipelineOutcome]

#: (session_id, swing_id, force). `None` is the shutdown sentinel.
_Job = tuple[str, str, bool]


def _default_runner(swing_dir: Path, options: PipelineOptions, log: Log) -> PipelineOutcome:
    return analyze_swing_dir(swing_dir, options=options, log=log)


class AnalysisWorker:
    """Runs the bundle pipeline off the event loop, one swing at a time."""

    def __init__(
        self,
        store: SwingBundleStore,
        *,
        runner: Runner | None = None,
        options: PipelineOptions | None = None,
    ) -> None:
        self._store = store
        self._runner = runner or _default_runner
        self._options = options or PipelineOptions()
        self._queue: asyncio.Queue[_Job | None] = asyncio.Queue()
        self._task: asyncio.Task[None] | None = None

    # -- lifecycle ---------------------------------------------------------------------

    async def start(self) -> None:
        """Spawn the consumer and re-queue whatever a previous run left unfinished."""
        if self._task is not None:
            return
        self._task = asyncio.create_task(self._consume(), name="analysis-worker")
        self._recover()

    async def stop(self) -> None:
        """Let the in-flight swing finish, then stop. Queued swings are dropped.

        Dropping them is safe rather than merely tolerable: `submit` wrote a `queued` state for
        each, so startup recovery picks them up again next time the server runs.
        """
        if self._task is None:
            return
        await self._queue.put(None)
        await self._task
        self._task = None

    # -- submission --------------------------------------------------------------------

    def submit(self, session_id: str, swing_id: str, *, force: bool = False) -> bool:
        """Queue a swing for analysis. Returns False if there was nothing to do.

        Safe to call from a route handler: it never blocks and never touches the pipeline.

        **Call this from the event-loop thread.** `asyncio.Queue` is not thread-safe, and a
        `put_nowait` from another thread can enqueue the job without waking a consumer already
        parked on `get()` — it would then sit untouched until unrelated traffic happened to wake
        the loop. Every production caller is a route handler or `start()`, so all of them
        already satisfy this; a caller from a worker thread would need
        `loop.call_soon_threadsafe`.
        """
        if self._task is None:
            return False
        manifest = self._store.get_swing(session_id, swing_id)
        if manifest is None:
            return False

        swing_dir = self._swing_dir(session_id, swing_id)
        state = load_state(swing_dir)
        if not force and state is not None and state.status == "done" and state.matches(manifest):
            return False
        if not force and state is not None and state.status in ("queued", "running"):
            return False

        missing = [role.value for role in manifest.missing_roles()]
        save_state(
            AnalysisState(
                status="queued",
                inputs=input_hashes(manifest),
                queued_at=now(),
                partial=bool(missing),
                missing_roles=missing,
            ),
            swing_dir,
        )
        self._queue.put_nowait((session_id, swing_id, force))
        logger.info("queued %s/%s for analysis (force=%s)", session_id, swing_id, force)
        return True

    def state(self, session_id: str, swing_id: str) -> AnalysisState | None:
        return load_state(self._swing_dir(session_id, swing_id))

    # -- the consumer ------------------------------------------------------------------

    async def _consume(self) -> None:
        while True:
            job = await self._queue.get()
            try:
                if job is None:
                    return
                await self._run(job)
            except Exception:
                # A crash here would silently stop every future analysis, which is far worse
                # than one swing failing. `_run` already records expected failures itself; this
                # is the net under the unexpected ones.
                logger.exception("analysis worker job failed: %s", job)
            finally:
                self._queue.task_done()

    async def _run(self, job: _Job) -> None:
        session_id, swing_id, force = job
        swing_dir = self._swing_dir(session_id, swing_id)
        manifest = self._store.get_swing(session_id, swing_id)
        if manifest is None:
            logger.warning("swing %s/%s vanished before analysis", session_id, swing_id)
            return

        state = load_state(swing_dir)
        if not force and state is not None and state.status == "done" and state.matches(manifest):
            return

        # `updated_at` is captured before the run so a role arriving mid-analysis can be
        # detected afterwards — the result would otherwise silently omit it.
        seen_at = manifest.updated_at
        missing = [role.value for role in manifest.missing_roles()]
        started = now()
        base = AnalysisState(
            status="running",
            inputs=input_hashes(manifest),
            queued_at=state.queued_at if state else None,
            started_at=started,
            partial=bool(missing),
            missing_roles=missing,
        )
        save_state(base, swing_dir)

        try:
            outcome = await asyncio.to_thread(
                self._runner, swing_dir, self._options, _pipeline_log
            )
        except Exception as exc:  # noqa: BLE001 - recorded, not swallowed
            logger.exception("analysis failed for %s/%s", session_id, swing_id)
            save_state(
                base.model_copy(
                    update={
                        "status": "failed",
                        "completed_at": now(),
                        "duration_seconds": _elapsed(started),
                        "error": f"{type(exc).__name__}: {exc}",
                    }
                ),
                swing_dir,
            )
            return

        if outcome.result is None:
            save_state(
                base.model_copy(
                    update={
                        "status": "failed",
                        "completed_at": now(),
                        "duration_seconds": _elapsed(started),
                        "error": outcome.error or "the pipeline produced no result",
                    }
                ),
                swing_dir,
            )
            return

        feedback = outcome.result.feedback
        save_state(
            base.model_copy(
                update={
                    "status": "done",
                    "completed_at": now(),
                    "duration_seconds": _elapsed(started),
                    "video": outcome.video_path.name if outcome.video_path else None,
                    "video_codec": outcome.video_codec,
                    "score": outcome.result.swing.overall_score,
                    "headline": feedback.headline if feedback else None,
                }
            ),
            swing_dir,
        )
        logger.info("analyzed %s/%s in %.1fs", session_id, swing_id, _elapsed(started))

        self._resubmit_if_changed(session_id, swing_id, seen_at)

    def _resubmit_if_changed(
        self, session_id: str, swing_id: str, seen_at: datetime
    ) -> None:
        """A role that landed while pose was running is not in the result we just wrote."""
        latest = self._store.get_swing(session_id, swing_id)
        if latest is None or latest.updated_at == seen_at:
            return
        if latest.status() != "complete":
            # Still incomplete, so the automatic trigger does not apply — leave it for the
            # "Analyze anyway" button rather than re-running on every straggling upload.
            return
        logger.info("%s/%s changed during analysis, re-queuing", session_id, swing_id)
        self.submit(session_id, swing_id, force=True)

    # -- startup recovery --------------------------------------------------------------

    def _recover(self) -> None:
        """Re-queue what a previous process left half-done, for today's session only.

        `queued` and `running` states are by definition orphaned at startup — no consumer
        survived the restart that could still be working on them.
        """
        session_id = self._store.current_session_id()
        for manifest in self._store.get_session(session_id):
            state = load_state(self._swing_dir(session_id, manifest.swing_id))
            if state is not None and state.status in ("queued", "running"):
                self.submit(session_id, manifest.swing_id, force=True)
                continue
            if manifest.status() != "complete":
                continue
            if state is None or (state.status == "done" and not state.matches(manifest)):
                self.submit(session_id, manifest.swing_id)

    def _swing_dir(self, session_id: str, swing_id: str) -> Path:
        return self._store.root / session_id / swing_id


def _pipeline_log(message: str) -> None:
    """The pipeline narrates for a terminal; a log file wants neither blank lines nor indent."""
    text = message.strip()
    if text:
        logger.info("%s", text)


def _elapsed(since: datetime) -> float:
    return (now() - since).total_seconds()


def should_analyze(manifest: SwingManifest) -> bool:
    """The automatic trigger: a complete bundle, and nothing less."""
    return manifest.status() == "complete"
