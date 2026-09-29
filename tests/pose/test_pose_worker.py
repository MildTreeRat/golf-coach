"""The pose sidecar worker, driven over real pipes with no MediaPipe in the test run. [M23 P3]

`worker.main` takes its three streams as arguments, so these tests run it on a thread between two
`os.pipe()`s. That is deliberately not a `StringIO`: the framing, the flush after every line and
the order of the two replies per job (ADR-033 clause 4) are exactly the properties a buffered
in-memory stream would hide, and a test that reached for a real subprocess instead would have no
way to install a fake `estimate_pose` inside it.

What is faked is the two heavy calls and nothing else — `_open_clip` and `_estimate`. The protocol
handling, the validation, the failure classification and the `KeypointsFile` it builds are the real
code. Every reply asserted here is also parsed by `crates/pose`'s `Lines::read`, which **validates
on arrival**, so a field this file is happy with and `contracts` is not would fail in P4 instead;
the bound that matters most is `Reply::Accepted`'s `width`/`height` `> 0`, which is why
`test_the_container_that_will_not_say_how_big_it_is_is_unreadable` exists.

Named `test_pose_worker.py` rather than the mirrored `test_worker.py`, which is the one place this
file departs from `CLAUDE.md`'s "tests mirror src package by package": `tests/api/test_worker.py`
already exists, and with no `__init__.py` under `tests/` pytest refuses two test modules with the
same basename. Setting `--import-mode=importlib` for the whole suite would have kept the mirror and
was declined as a harness change that P3 has no business making for one file.
"""

from __future__ import annotations

import hashlib
import json
import os
import sys
import threading
from collections.abc import Iterator
from dataclasses import dataclass
from pathlib import Path
from typing import Any

import pytest

from golf_coach.contracts.keypoints import NUM_POSE_LANDMARKS, FrameKeypoints, Landmark
from golf_coach.pose import worker


@dataclass
class _FakeFrame:
    """Stands in for `capture.source.Frame` without the numpy image."""

    index: int
    timestamp_ms: float
    camera_id: str | None


class _FakeSource:
    """A `FileVideoSource` that decodes nothing. `closed` is what pins the context-manager use."""

    def __init__(
        self,
        *,
        fps: float = 59.9651365485183,
        width: int = 2160,
        height: int = 3840,
        frame_count: int = 3,
        camera_id: str | None = None,
    ) -> None:
        self.fps = fps
        self.width = width
        self.height = height
        self.frame_count = frame_count
        self.camera_id = camera_id
        self.closed = False

    def __enter__(self) -> _FakeSource:
        return self

    def __exit__(self, *exc: object) -> None:
        self.closed = True

    def frames(self) -> Iterator[_FakeFrame]:
        for index in range(self.frame_count):
            yield _FakeFrame(
                index=index,
                timestamp_ms=index / self.fps * 1000.0,
                camera_id=self.camera_id,
            )


def _fake_keypoints(frames: Any, model_path: Path, variant: str) -> list[FrameKeypoints]:
    """One `FrameKeypoints` per frame the source yields, so the generator is really consumed."""
    return [
        FrameKeypoints(
            frame_index=frame.index,
            timestamp_ms=frame.timestamp_ms,
            landmarks=[
                Landmark(x=0.5, y=0.5, z=0.0, visibility=1.0) for _ in range(NUM_POSE_LANDMARKS)
            ],
            camera_id=frame.camera_id,
        )
        for frame in frames
    ]


class _Capture:
    """A stderr the worker can `print` to and a test can read back.

    Deliberately not a `TextIOWrapper`: `main` reconfigures the two protocol streams and must leave
    a logging stream that has no `reconfigure` alone, which this is the only thing that checks.
    """

    def __init__(self) -> None:
        self._parts: list[str] = []

    def write(self, text: str) -> int:
        self._parts.append(text)
        return len(text)

    def flush(self) -> None:
        pass

    def text(self) -> str:
        return "".join(self._parts)


class _Sidecar:
    """`worker.main` on a thread, with a pipe in each direction."""

    def __init__(self) -> None:
        to_worker, stdin_w = os.pipe()
        stdout_r, from_worker = os.pipe()
        self._worker_stdin = os.fdopen(to_worker, "r", encoding="utf-8", newline="\n")
        self._worker_stdout = os.fdopen(from_worker, "w", encoding="utf-8", newline="\n")
        self._writer = os.fdopen(stdin_w, "w", encoding="utf-8", newline="\n")
        self._reader = os.fdopen(stdout_r, "r", encoding="utf-8", newline="\n")
        self.stderr = _Capture()
        self.exit_code: int | None = None
        self._thread = threading.Thread(target=self._run, daemon=True)
        self._thread.start()

    def _run(self) -> None:
        try:
            self.exit_code = worker.main(
                stdin=self._worker_stdin, stdout=self._worker_stdout, stderr=self.stderr
            )
        finally:
            # Closing the worker's end is what gives the reader EOF, so a test that expects no
            # further line gets one rather than hanging.
            self._worker_stdout.close()

    def send(self, job: dict[str, Any]) -> None:
        self.send_raw(json.dumps(job))

    def send_raw(self, line: str) -> None:
        self._writer.write(line + "\n")
        self._writer.flush()

    def read(self) -> dict[str, Any]:
        line = self._reader.readline()
        assert line, f"the worker wrote nothing; stderr was {self.stderr.text()!r}"
        assert line.endswith("\n"), "a reply must be one newline-terminated line (clause 1)"
        parsed = json.loads(line)
        assert isinstance(parsed, dict)
        return parsed

    def close_stdin(self) -> None:
        self._writer.close()

    def close(self, *, timeout: float = 10.0) -> int | None:
        """Close the worker's stdin — the pool's way of saying shut down — and reap the thread."""
        self.close_stdin()
        self._thread.join(timeout)
        assert not self._thread.is_alive(), "the worker did not exit when its stdin closed"
        return self.exit_code


@pytest.fixture
def sidecar(monkeypatch: pytest.MonkeyPatch, tmp_path: Path) -> Iterator[Any]:
    """A worker factory whose model, extras and two heavy calls are all under the test's control.

    `models_dir` points at a tmp directory holding a zero-byte `.task`: clause 3's check is
    `exists()`, so a real 30 MB bundle would make these tests slower without making them stricter.
    """
    models = tmp_path / "models"
    models.mkdir()
    (models / "pose_landmarker_heavy.task").write_bytes(b"")
    monkeypatch.setattr(worker.settings, "models_dir", models)
    monkeypatch.setattr(worker.settings, "pose_model_variant", "heavy")
    monkeypatch.setattr(worker, "_missing_extras", lambda: [])
    monkeypatch.setattr(worker, "_estimate", _fake_keypoints)

    started: list[_Sidecar] = []

    def start() -> _Sidecar:
        side = _Sidecar()
        started.append(side)
        return side

    yield start
    for side in started:
        side.close_stdin()
        side._thread.join(10)


def _clip(tmp_path: Path, name: str = "face_on.MOV", body: bytes = b"not really a movie") -> Path:
    path = tmp_path / name
    path.write_bytes(body)
    return path


def _job(clip: Path, **over: Any) -> dict[str, Any]:
    """ADR-033 clause 2's envelope, with all five fields present as the Rust side writes them.

    The parameter is `clip` and not `clip_path` so that `**over` can override that very field.
    """
    job: dict[str, Any] = {
        "job_id": "j1",
        "clip_path": str(clip),
        "frame_range": None,
        "camera_id": "face_on",
        "pose_model_variant": "heavy",
    }
    job.update(over)
    return job


# --- the handshake -------------------------------------------------------------------------------


def test_the_handshake_is_the_first_line_and_names_the_verified_model(sidecar: Any) -> None:
    side = sidecar()

    hello = side.read()

    assert hello == {
        "pose": "ready",
        "protocol": worker.PROTOCOL_VERSION,
        "estimator": "mediapipe:heavy",
        "variant": "heavy",
        "model": str(worker.settings.models_dir / "pose_landmarker_heavy.task"),
    }
    assert side.close() == 0


def test_the_handshake_is_written_before_any_job_is_read(sidecar: Any) -> None:
    """Clause 3: *before it reads anything*. Reading a reply with nothing sent proves the order."""
    side = sidecar()

    assert side.read()["pose"] == "ready"

    side.close()


def test_a_missing_model_is_unavailable_at_startup_and_a_non_zero_exit(sidecar: Any) -> None:
    """The point of clause 3: a machine that cannot pose says so when the session starts."""
    (worker.settings.models_dir / "pose_landmarker_heavy.task").unlink()
    side = sidecar()

    hello = side.read()

    assert hello["pose"] == "unavailable"
    assert hello["reason"] == "model_absent"
    assert "pose_landmarker_heavy.task" in hello["detail"]
    assert side.close() == 1


def test_a_missing_vision_extra_is_unavailable_rather_than_a_traceback(
    sidecar: Any, monkeypatch: pytest.MonkeyPatch
) -> None:
    """Named so the pool can print the fix, the way `TriggerUnavailable` carries `cargo build`."""
    monkeypatch.setattr(worker, "_missing_extras", lambda: ["mediapipe"])
    side = sidecar()

    hello = side.read()

    assert hello["pose"] == "unavailable"
    assert hello["reason"] == "vision_extra_absent"
    assert "mediapipe" in hello["detail"] and "[vision]" in hello["detail"]
    assert side.close() == 1


def test_an_unknown_configured_variant_is_unavailable_and_not_a_missing_model(
    sidecar: Any, monkeypatch: pytest.MonkeyPatch
) -> None:
    """`GOLF_POSE_MODEL_VARIANT` set to a typo is an operator error, reported as its own thing."""
    monkeypatch.setattr(worker.settings, "pose_model_variant", "heavvy")
    side = sidecar()

    hello = side.read()

    assert hello["pose"] == "unavailable"
    assert hello["reason"] == "bad_variant"
    assert "heavvy" in hello["detail"]
    assert side.close() == 1


# --- a good job ----------------------------------------------------------------------------------


def test_a_job_is_accepted_then_done_in_that_order(sidecar: Any, tmp_path: Path) -> None:
    """Clause 4's two replies, and the acceptance carries the claim, not the decoded count."""
    clip = _clip(tmp_path)
    source = _FakeSource(frame_count=3, camera_id="face_on")
    with pytest.MonkeyPatch.context() as patch:
        patch.setattr(worker, "_open_clip", lambda path, camera_id: source)
        side = sidecar()
        side.read()
        side.send(_job(clip))

        accepted = side.read()
        done = side.read()
        side.close()

    assert accepted == {
        "job_id": "j1",
        "pose": "accepted",
        "frames": 3,
        "fps": source.fps,
        "width": 2160,
        "height": 3840,
    }
    assert done["pose"] == "done" and done["job_id"] == "j1"
    assert source.closed, "the clip's capture handle must be released"


def test_the_done_reply_carries_the_keypoints_file_the_pipeline_would_have_written(
    sidecar: Any, tmp_path: Path
) -> None:
    """`contracts/keypoints.py`'s shape, with `exclude_none` as `storage/keypoints_io.py` has it."""
    clip = _clip(tmp_path)
    with pytest.MonkeyPatch.context() as patch:
        patch.setattr(worker, "_open_clip", lambda path, camera_id: _FakeSource(frame_count=2))
        side = sidecar()
        side.read()
        side.send(_job(clip))
        side.read()
        done = side.read()
        side.close()

    kp = done["keypoints"]
    assert kp["pose_estimator"] == "mediapipe:heavy"
    assert kp["clip"] == {
        "fps": 59.9651365485183,
        "width": 2160,
        "height": 3840,
        # The DECODED count, deliberately not the container's claim of 3 in the acceptance above.
        "frame_count": 2,
        "source_sha256": worker._sha256(clip),
    }
    assert len(kp["frames"]) == 2
    assert len(kp["frames"][0]["landmarks"]) == NUM_POSE_LANDMARKS
    # `exclude_none=True`: an absent key, never an explicit null. The comparison against the stored
    # corpus is structural either way (P1 finding 4), but omitting the same keys is free.
    assert "camera_id" not in kp["frames"][0]


def test_source_sha256_is_the_digest_of_the_file_the_worker_read(
    sidecar: Any, tmp_path: Path
) -> None:
    """P3's decision, recorded in ADR-033's addendum: the worker hashes the bytes it decoded.

    The value has to equal what `bundle_store` recorded as `RoleFile.content_sha256`, because that
    is what the 30 stored keypoints files carry and they are M23 P7's only oracle. Verified against
    all 30 of them when the decision was taken; pinned here on bytes a test can own.
    """
    body = b"\x00\x01two frames of nothing at all"
    clip = _clip(tmp_path, body=body)
    with pytest.MonkeyPatch.context() as patch:
        patch.setattr(worker, "_open_clip", lambda path, camera_id: _FakeSource(frame_count=1))
        side = sidecar()
        side.read()
        side.send(_job(clip))
        side.read()
        done = side.read()
        side.close()

    assert done["keypoints"]["clip"]["source_sha256"] == hashlib.sha256(body).hexdigest()


def test_one_worker_answers_several_jobs_because_the_process_is_warm(
    sidecar: Any, tmp_path: Path
) -> None:
    """ADR-030 §3: the process is long-lived, and it is the landmarker that is per job."""
    first, second = _clip(tmp_path, "a.MOV"), _clip(tmp_path, "b.MOV", b"other bytes")
    with pytest.MonkeyPatch.context() as patch:
        patch.setattr(worker, "_open_clip", lambda path, camera_id: _FakeSource(frame_count=1))
        side = sidecar()
        side.read()
        side.send(_job(first, job_id="one"))
        side.read()
        done_one = side.read()
        side.send(_job(second, job_id="two"))
        side.read()
        done_two = side.read()
        assert side.close() == 0

    assert [done_one["job_id"], done_two["job_id"]] == ["one", "two"]
    assert (
        done_one["keypoints"]["clip"]["source_sha256"]
        != done_two["keypoints"]["clip"]["source_sha256"]
    ), "each reply hashes its own clip"


def test_a_blank_line_is_skipped_rather_than_refused(sidecar: Any, tmp_path: Path) -> None:
    """`Lines::read` skips whitespace-only lines, and this side matches it."""
    clip = _clip(tmp_path)
    with pytest.MonkeyPatch.context() as patch:
        patch.setattr(worker, "_open_clip", lambda path, camera_id: _FakeSource(frame_count=1))
        side = sidecar()
        side.read()
        side.send_raw("   ")
        side.send_raw("")
        side.send(_job(clip))

        assert side.read()["pose"] == "accepted"
        assert side.read()["pose"] == "done"
        assert side.close() == 0


# --- failures ------------------------------------------------------------------------------------


def test_an_unreadable_clip_fails_without_an_acceptance(sidecar: Any, tmp_path: Path) -> None:
    """No container, no shape to ack — so the failure is the only reply about this job."""

    def boom(path: Path, camera_id: str | None) -> Any:
        raise FileNotFoundError(f"Video file not found: {path}")

    with pytest.MonkeyPatch.context() as patch:
        patch.setattr(worker, "_open_clip", boom)
        side = sidecar()
        side.read()
        side.send(_job(tmp_path / "gone.MOV"))
        reply = side.read()
        side.close()

    assert reply["pose"] == "failed"
    assert reply["reason"] == "clip_unreadable"
    assert reply["job_id"] == "j1"
    assert "gone.MOV" in reply["detail"]


def test_a_container_that_refuses_on_enter_is_unreadable_and_not_a_crash(
    sidecar: Any, tmp_path: Path
) -> None:
    """The shape of the real `FileVideoSource`, which is where M23 P5 found this. [M23 P5]

    `FileVideoSource.__init__` stores a path and `__enter__` raises `FileNotFoundError` for one it
    cannot open — so a job naming a missing clip used to take the traceback out of `main` and kill
    the worker, and the pool read that as a crashed process: retried on a fresh interpreter,
    reported as a crash, and never as the `clip_unreadable` clause 5 says not to retry.

    The test above raises from the *constructor*, which is why it passed throughout. This one raises
    from `__enter__`, so it fails on the structure rather than on the exception type.
    """

    class _RefusesOnEnter:
        def __enter__(self) -> Any:
            raise FileNotFoundError(f"Video file not found: {tmp_path / 'gone.MOV'}")

        def __exit__(self, *exc: object) -> None:
            raise AssertionError("__exit__ cannot run when __enter__ refused")

    with pytest.MonkeyPatch.context() as patch:
        patch.setattr(worker, "_open_clip", lambda path, camera_id: _RefusesOnEnter())
        side = sidecar()
        side.read()
        side.send(_job(tmp_path / "gone.MOV"))
        reply = side.read()
        # The worker is still alive and still serving: a fact about a file is not a fact about the
        # process, which is the whole distinction clause 5's retry table is built on.
        side.send(_job(tmp_path / "gone.MOV", job_id="j2"))
        second = side.read()
        side.close()

    assert (reply["pose"], reply["reason"]) == ("failed", "clip_unreadable")
    assert "gone.MOV" in reply["detail"]
    assert (second["job_id"], second["reason"]) == ("j2", "clip_unreadable")


def test_the_container_that_will_not_say_how_big_it_is_is_unreadable(
    sidecar: Any, tmp_path: Path
) -> None:
    """P2 finding 5: `Reply::Accepted` bounds width and height at `> 0`, so a zero cannot be acked.

    Acking one would be read as a protocol violation by the pool — reporting the wrong thing about
    the wrong side of the pipe — so the worker refuses the clip instead.
    """
    clip = _clip(tmp_path)
    with pytest.MonkeyPatch.context() as patch:
        patch.setattr(worker, "_open_clip", lambda path, camera_id: _FakeSource(width=0, height=0))
        side = sidecar()
        side.read()
        side.send(_job(clip))
        reply = side.read()
        side.close()

    assert (reply["pose"], reply["reason"]) == ("failed", "clip_unreadable")
    assert "0x0" in reply["detail"]


def test_a_clip_that_decodes_no_frames_is_empty_and_not_a_pose_failure(
    sidecar: Any, tmp_path: Path
) -> None:
    """It opened, so it was acked; it yielded nothing, which is a fact about the file (clause 5)."""
    clip = _clip(tmp_path)
    with pytest.MonkeyPatch.context() as patch:
        patch.setattr(worker, "_open_clip", lambda path, camera_id: _FakeSource(frame_count=0))
        side = sidecar()
        side.read()
        side.send(_job(clip))

        assert side.read()["pose"] == "accepted"
        reply = side.read()
        side.close()

    assert (reply["pose"], reply["reason"]) == ("failed", "clip_empty")


def test_an_exception_out_of_pose_is_pose_failed_with_the_traceback_on_stderr(
    sidecar: Any, tmp_path: Path
) -> None:
    """Clause 5 retries this one, and clause 1's stderr is what makes its failure readable."""
    clip = _clip(tmp_path)

    def explode(frames: Any, model_path: Path, variant: str) -> Any:
        raise RuntimeError("the graph fell over")

    with pytest.MonkeyPatch.context() as patch:
        patch.setattr(worker, "_open_clip", lambda path, camera_id: _FakeSource())
        patch.setattr(worker, "_estimate", explode)
        side = sidecar()
        side.read()
        side.send(_job(clip))
        side.read()
        reply = side.read()
        side.close()

    assert (reply["pose"], reply["reason"]) == ("failed", "pose_failed")
    assert reply["detail"] == "RuntimeError: the graph fell over"
    assert "Traceback" in side.stderr.text(), "the pool attaches stderr to what it reports"


def test_a_line_that_is_not_json_is_a_bad_job_rather_than_a_dead_worker(
    sidecar: Any, tmp_path: Path
) -> None:
    """Exiting would tell the pool a worker died and nothing about why. The why is the value.

    The reply carries an empty `job_id` because there is none to echo; the pool hands a worker one
    job at a time, so it is still attributable to whatever was in flight.
    """
    clip = _clip(tmp_path)
    with pytest.MonkeyPatch.context() as patch:
        patch.setattr(worker, "_open_clip", lambda path, camera_id: _FakeSource(frame_count=1))
        side = sidecar()
        side.read()
        side.send_raw("{not json at all")
        reply = side.read()

        # And the worker is still alive and still answering, which is the half that matters.
        side.send(_job(clip))
        assert side.read()["pose"] == "accepted"
        assert side.read()["pose"] == "done"
        assert side.close() == 0

    assert (reply["pose"], reply["reason"], reply["job_id"]) == ("failed", "bad_job", "")


@pytest.mark.parametrize(
    ("over", "expected_in_detail"),
    [
        ({"job_id": ""}, "job_id"),
        ({"clip_path": ""}, "clip_path"),
        ({"camera_id": 7}, "camera_id"),
        ({"frame_range": [1, 2, 3]}, "frame_range"),
        ({"frame_range": "all"}, "frame_range"),
    ],
)
def test_a_malformed_envelope_is_a_bad_job(
    sidecar: Any, tmp_path: Path, over: dict[str, Any], expected_in_detail: str
) -> None:
    """Clause 5 does not retry a caller bug, because retrying one hides it."""
    clip = _clip(tmp_path)
    side = sidecar()
    side.read()
    side.send(_job(clip, **over))
    reply = side.read()
    side.close()

    assert (reply["pose"], reply["reason"]) == ("failed", "bad_job")
    assert expected_in_detail in reply["detail"]


def test_an_unknown_job_field_is_a_bad_job(sidecar: Any, tmp_path: Path) -> None:
    """The mirror of `crates/pose`'s `deny_unknown_fields`.

    A key this build has not heard of means the two halves disagree about the protocol, and that is
    worth one refused job to learn.
    """
    clip = _clip(tmp_path)
    side = sidecar()
    side.read()
    side.send(_job(clip, priority="urgent"))
    reply = side.read()
    side.close()

    assert (reply["pose"], reply["reason"]) == ("failed", "bad_job")
    assert "priority" in reply["detail"]


def test_a_job_naming_another_variant_is_refused_rather_than_honoured(
    sidecar: Any, tmp_path: Path
) -> None:
    """One process, one instrument, for the life of the process.

    Honouring it would need a model that may not be on disk, and clause 6's `fps_estimate` is per
    variant — so a pool whose workers switched bundle mid-flight would derive every work deadline
    from the wrong number.
    """
    clip = _clip(tmp_path)
    side = sidecar()
    side.read()
    side.send(_job(clip, pose_model_variant="lite"))
    reply = side.read()
    side.close()

    assert (reply["pose"], reply["reason"]) == ("failed", "bad_job")
    assert "'lite'" in reply["detail"] and "'heavy'" in reply["detail"]


def test_an_absent_variant_means_this_workers_own(sidecar: Any, tmp_path: Path) -> None:
    """`pose_model_variant: null` is ADR-033's "the worker's configured default", not a bad job."""
    clip = _clip(tmp_path)
    with pytest.MonkeyPatch.context() as patch:
        patch.setattr(worker, "_open_clip", lambda path, camera_id: _FakeSource(frame_count=1))
        side = sidecar()
        side.read()
        side.send(_job(clip, pose_model_variant=None))
        side.read()
        done = side.read()
        side.close()

    assert done["keypoints"]["pose_estimator"] == "mediapipe:heavy"


# --- frame_range ---------------------------------------------------------------------------------


def test_a_sub_range_is_refused_and_not_quietly_posed_in_full(
    sidecar: Any, tmp_path: Path
) -> None:
    """Clause 2.

    A worker that posed the whole clip anyway would return landmarks whose `frame_index` meant
    something other than what the caller asked for — a confident wrong answer, which is ADR-010 §2's
    rule reaching the sidecar.
    """
    clip = _clip(tmp_path)
    with pytest.MonkeyPatch.context() as patch:
        patch.setattr(worker, "_open_clip", lambda path, camera_id: _FakeSource(frame_count=344))
        side = sidecar()
        side.read()
        side.send(_job(clip, frame_range=[100, 200]))
        reply = side.read()
        side.close()

    assert (reply["pose"], reply["reason"]) == ("failed", "frame_range_unsupported")
    assert "[0, 344]" in reply["detail"]


def test_the_whole_clip_spelled_out_as_a_range_is_accepted(sidecar: Any, tmp_path: Path) -> None:
    """`[0, frame_count]` is the one range clause 2 permits, against the container's own claim."""
    clip = _clip(tmp_path)
    with pytest.MonkeyPatch.context() as patch:
        patch.setattr(worker, "_open_clip", lambda path, camera_id: _FakeSource(frame_count=3))
        side = sidecar()
        side.read()
        side.send(_job(clip, frame_range=[0, 3]))

        assert side.read()["pose"] == "accepted"
        assert side.read()["pose"] == "done"
        assert side.close() == 0


# --- shutdown ------------------------------------------------------------------------------------


def test_closing_stdin_is_a_clean_exit(sidecar: Any) -> None:
    """EOF on stdin is how the pool says shut down — clause 5's EOF is the other direction."""
    side = sidecar()
    side.read()

    assert side.close() == 0


def test_the_worker_writes_nothing_more_after_its_stdin_closes(sidecar: Any) -> None:
    side = sidecar()
    side.read()

    assert side.close() == 0
    assert side._reader.readline() == "", "shutdown must not leave a trailing line behind"


# --- stdout is protocol --------------------------------------------------------------------------


def test_a_dependency_that_prints_cannot_corrupt_the_channel(sidecar: Any, tmp_path: Path) -> None:
    """Clause 1's reason for saving the reply stream at startup: `sys.stdout` goes to stderr.

    A `print` from inside the pose call stands in for the OpenCV or MediaPipe chatter this guards
    against — if it reached stdout the next line the pool read would not be JSON, and a whole
    worker would be reported as a corrupted channel.
    """
    clip = _clip(tmp_path)

    def chatty(frames: Any, model_path: Path, variant: str) -> list[FrameKeypoints]:
        print("INFO: initialising some native thing")
        sys.stdout.write("and a bare write too\n")
        return _fake_keypoints(frames, model_path, variant)

    with pytest.MonkeyPatch.context() as patch:
        patch.setattr(worker, "_open_clip", lambda path, camera_id: _FakeSource(frame_count=1))
        patch.setattr(worker, "_estimate", chatty)
        side = sidecar()
        side.read()
        side.send(_job(clip))

        assert side.read()["pose"] == "accepted"
        assert side.read()["pose"] == "done"
        side.close()

    assert "initialising some native thing" in side.stderr.text()
    assert "and a bare write too" in side.stderr.text()
