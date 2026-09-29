"""The pose sidecar worker: one clip path in on stdin, landmarks out on stdout. [M23 P3]

ADR-033's Python half. The Rust core (`crates/pose`) spawns `python -m golf_coach.pose.worker`,
reads one handshake line, then writes one job line per clip and reads two reply lines back. This
module is that loop and nothing else: **it writes no file, decides no output path, downloads no
model, and has never heard of a swing directory** (clause 8 — Rust owns the artifact layout, the
same division `crates/capture` already has).

Everything that does the actual work is reused unchanged, which is ADR-030 §2's rule — *"what
changes is the process that calls them, and nothing else"*. `estimate_pose` poses,
`FileVideoSource` decodes, `_to_frame_keypoints` maps, `KeypointsFile` is the shape. A standalone
`sidecar/` package was declined for M23 because it forks the landmark mapping and the contract
into a second copy now, for a benefit §M29 is the right milestone to decide.

**The process is warm, not the landmarker** (ADR-030 §3). A fresh `PoseLandmarker` per job is
already what `estimate_pose` does — it builds one inside its `with` block — because
`RunningMode.VIDEO` carries cross-frame tracking state and the estimator forces
strictly-increasing timestamps. So the call below is deliberately *not* hoisted out of the loop,
and what a long-lived process buys is the interpreter and the imports, not a reused graph.

**stdout is protocol and stderr is logging** (clause 1). The reply stream is captured once at
startup and `sys.stdout` is pointed at stderr for every job, so a native library or a dependency
that prints cannot corrupt the channel. That is wider than the ADR's "for the duration of the pose
call": opening a container is also a native library that can print, and the boundary of a job is
easier to reason about than the boundary of one call.

**No caller in Python, deliberately.** `scripts/run_pose.py` and `api/pipeline.py` keep calling
`estimate_pose` directly (ADR-033 §Consequences); nothing in Python should route through Rust to
reach a Python function.
"""

from __future__ import annotations

import hashlib
import importlib.util
import json
import sys
import traceback
from collections.abc import Iterable, Iterator
from contextlib import ExitStack, redirect_stdout
from pathlib import Path
from typing import IO, TYPE_CHECKING, Any, Protocol

from golf_coach.config import settings
from golf_coach.contracts.keypoints import ClipMetadata, FrameKeypoints, KeypointsFile
from golf_coach.pose.estimator import model_filename, pose_estimator_name, resolve_variant

if TYPE_CHECKING:
    from golf_coach.capture.source import Frame

#: Must equal `crates/pose`'s `PROTOCOL_VERSION`. Sent in the handshake and refused by the pool
#: when it does not recognize it (clause 3) — M26 ships the interpreter and the sidecar together so
#: the pair always matches, but during development they are two working copies, and a mismatch is
#: worth failing on the first line rather than on a missing field eight minutes into a clip.
PROTOCOL_VERSION = 1

#: The job envelope's fields, and the whole set of them. Mirrors `crates/pose`'s
#: `#[serde(deny_unknown_fields)]` on `Job`: a key this build has not heard of means the two halves
#: disagree about the protocol, and a strict writer deserves an equally strict reader facing it.
_JOB_FIELDS = frozenset({"job_id", "clip_path", "frame_range", "camera_id", "pose_model_variant"})

#: Read size for `_sha256`. One MiB, matching `api/app.py`'s upload hasher — the corpus's clips are
#: 38-49 MB each, so this is about the latency of the acceptance line, not about memory.
_HASH_CHUNK_BYTES = 1 << 20

#: What the `vision` extra means to this module: OpenCV decodes the container, MediaPipe poses it.
#: Named here rather than left implicit in two lazy imports, because `_startup` has to be able to
#: say which one is missing.
_VISION_MODULES = ("cv2", "mediapipe")


class _ClipSource(Protocol):
    """The slice of `FileVideoSource` a job needs. Structural, so that import can stay lazy."""

    @property
    def fps(self) -> float: ...
    @property
    def width(self) -> int: ...
    @property
    def height(self) -> int: ...
    @property
    def frame_count(self) -> int: ...
    def frames(self) -> Iterator[Frame]: ...
    def __enter__(self) -> Any: ...
    def __exit__(self, *exc: object) -> None: ...


class _Unavailable(Exception):
    """This worker cannot pose, and says so at startup rather than 30 s into the first clip.

    `reason` is free-form where a job's failure reason is a typed enum, and `crates/pose` mirrors
    that asymmetry on purpose: nothing branches on this string, a human reads it, so a worker
    naming something the Rust build had not heard of is reported as what it said instead of as an
    unparseable line. `model_absent` is the only value ADR-033 itself defines; the other two below
    are this phase's, and they are the two other ways a source checkout fails to be able to pose.
    """

    def __init__(self, reason: str, detail: str) -> None:
        super().__init__(f"{reason}: {detail}")
        self.reason = reason
        self.detail = detail


def _sha256(path: Path) -> str:
    """The digest of the bytes this worker actually decoded — `ClipMetadata.source_sha256`.

    **This is P3's answer to the one field ADR-033 clause 4 left open**, recorded in that ADR's
    2026-09-26 addendum. The alternative was leaving it for Rust to fill from the manifest, and it
    is not merely the less honest of the two — it would break the milestone's gate.
    `api/pipeline.py` fills the field from `SwingManifest.roles[role].content_sha256`, the digest
    taken as the upload streamed in, and `bundle_store` then *moves* those same bytes into the
    swing directory. So hashing the file on disk reproduces the stored value exactly: checked
    against all **30/30** corpus keypoints files, 30 matches and 0 mismatches. A worker that left
    the field `None` would put every reply structurally out of step with the only oracle M23 P7
    has.
    """
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(_HASH_CHUNK_BYTES), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _missing_extras() -> list[str]:
    """Which of `_VISION_MODULES` this install cannot import, the cheapest way there is.

    `find_spec` locates a module without executing it, which answers the question these installs
    actually fail on — *is the `vision` extra installed* — in microseconds, where a real
    `import mediapipe` costs seconds on every worker's startup and drags the ML stack into every
    test that drives this module. A native library that is present but broken still surfaces,
    later and as `pose_failed` on the first job.
    """
    return [name for name in _VISION_MODULES if importlib.util.find_spec(name) is None]


def _open_clip(clip_path: Path, camera_id: str | None) -> _ClipSource:
    """`FileVideoSource`, imported at first use rather than at module scope.

    Lazy for the reason `api/pipeline.py`'s is: it needs the `vision` extra, and `_startup` wants
    to report an absent extra as a handshake failure the pool can act on rather than as an
    ImportError traceback halfway through the first job.
    """
    from golf_coach.capture.file import FileVideoSource

    return FileVideoSource(clip_path, camera_id=camera_id)


def _estimate(frames: Iterable[Frame], model_path: Path, variant: str) -> list[FrameKeypoints]:
    """`estimate_pose`, with the model named explicitly so this process cannot reach the network.

    `estimate_pose(frames)` would call `ensure_pose_model`, which downloads a missing bundle —
    harmless here because `_startup` already verified the file, but only by coincidence of
    ordering. Passing the verified path makes "the worker never downloads" (clause 3) a property of
    the call instead of a property of what happened earlier, and ties the landmarks to the exact
    file the handshake named.
    """
    from golf_coach.pose.estimator import estimate_pose

    return estimate_pose(frames, model_path=model_path, variant=variant)


def _write(out: IO[str], payload: str) -> None:
    r"""Write one line and flush. The `\n` is the frame, so it is written here, not by a caller.

    Two writes rather than `payload + "\n"`: the largest reply this corpus produces is 16.4 MB on
    one line (4,837 frames), and concatenating a newline onto it copies all of it for one byte.
    """
    out.write(payload)
    out.write("\n")
    out.flush()


def _reply(out: IO[str], message: dict[str, Any]) -> None:
    """Write any reply but `done`, whose payload is spliced instead — see `_send_done`."""
    _write(out, json.dumps(message, separators=(",", ":")))


def _send_done(out: IO[str], job_id: str, keypoints: KeypointsFile) -> None:
    """The success reply, with the `KeypointsFile` serialized once and spliced into its envelope.

    `model_dump_json` rather than `model_dump()` inside a dict handed to `json.dumps`, because the
    latter builds the whole 16.4 MB payload twice — once as Python objects, once as text. The
    envelope is three fixed keys, so splicing is an f-string and not a second JSON writer.

    `exclude_none=True` is `storage/keypoints_io.py`'s flag, kept for two reasons: every optional
    field in `crates/contracts`' `KeypointsFile` carries `#[serde(default)]`, so an omitted key and
    an explicit null parse identically on the other side; and the stored corpus files were written
    with it, so a reply and its oracle omit the same keys.
    """
    body = keypoints.model_dump_json(exclude_none=True)
    _write(out, f'{{"job_id":{json.dumps(job_id)},"pose":"done","keypoints":{body}}}')


def _send_accepted(out: IO[str], job_id: str, source: _ClipSource) -> None:
    """Clause 4's acceptance line: the container is open, and here is its shape.

    Written **before a single frame is posed**, because the pool's work deadline has to scale with
    the clip — 344 to 4,837 frames on this corpus, a 14.1x spread — and the Rust core cannot get
    the frame count: it has a path and no video decoder (ADR-031 §2). The process that is about to
    decode the clip already knows, so it says.

    `frames` is the container's *claim* (`CAP_PROP_FRAME_COUNT`), the only count available before
    decoding. It is deliberately not the same number as the `frame_count` in the reply's
    `ClipMetadata`, which is how many frames actually decoded — a container that claims more than
    it yields is the silent failure that field exists to record.
    """
    _reply(
        out,
        {
            "job_id": job_id,
            "pose": "accepted",
            "frames": source.frame_count,
            "fps": source.fps,
            "width": source.width,
            "height": source.height,
        },
    )


def _send_failed(out: IO[str], job_id: str, reason: str, detail: str) -> None:
    """A named failure, and **never a partial or guessed `KeypointsFile`** (clause 5, ADR-010 §2).

    `reason` must be one of `crates/pose`'s `FailureReason` variants: that side parses it into a
    typed enum because `FailureReason::retried` branches on it, so an invented reason is refused as
    a protocol violation rather than logged. The set is `model_absent`, `clip_unreadable`,
    `clip_empty`, `frame_range_unsupported`, `bad_job`, `pose_failed`.
    """
    _reply(out, {"job_id": job_id, "pose": "failed", "reason": reason, "detail": detail})


def _startup(stderr: IO[str]) -> tuple[str, Path]:
    """Resolve the variant and verify this worker can pose. Raises `_Unavailable` if it cannot.

    The model is **verified, not downloaded** (clause 3): `ensure_pose_model`'s fetch stays a lab
    and setup action, because a worker that reaches the network on first use is a worker that
    stalls a golfer's first swing for 30 MB of bundle. §M26 replaces the check's subject with a
    shipped file and changes nothing else here.
    """
    try:
        variant = resolve_variant()
    except ValueError as exc:
        # Not a reason ADR-033 names, which is what `Handshake::Unavailable`'s free-form `reason`
        # is for: `GOLF_POSE_MODEL_VARIANT` set to a typo is an operator error, and saying so beats
        # reporting it as a missing model.
        raise _Unavailable("bad_variant", str(exc)) from exc

    missing = _missing_extras()
    if missing:
        raise _Unavailable(
            "vision_extra_absent",
            f"{', '.join(missing)} not importable — pip install -e '.[vision]'",
        )

    model_path = settings.models_dir / model_filename(variant)
    if not model_path.exists():
        raise _Unavailable(
            "model_absent",
            f"{model_path} is not on disk; run scripts/run_pose.py once to fetch it",
        )

    print(f"pose worker ready: {pose_estimator_name(variant)}", file=stderr, flush=True)
    return variant, model_path


def _resolve_job(message: object, variant: str) -> tuple[str, Path, str | None, list[int] | None]:
    """Validate a parsed job line into `(job_id, clip_path, camera_id, frame_range)`.

    Raises `ValueError` for anything that is a caller bug — clause 5's `bad_job`, which is not
    retried, because retrying a caller bug hides it.

    The variant is checked rather than honoured: **a worker serves exactly the variant it announced
    in its handshake.** A job naming another one is refused. Honouring it was declined on two
    counts, neither of which is the worker's call to make: the model for that variant may not be on
    disk, and clause 6's `fps_estimate` is per variant (ADR-002 puts `lite` at ~4x `heavy`), so a
    pool whose workers switched bundle mid-flight would derive every deadline from the wrong
    number. One process, one instrument, for the life of the process.
    """
    if not isinstance(message, dict):
        raise ValueError(f"a job must be a JSON object, got {type(message).__name__}")
    unknown = sorted(set(message) - _JOB_FIELDS)
    if unknown:
        raise ValueError(
            f"unknown job field(s) {unknown}; this worker speaks protocol {PROTOCOL_VERSION}"
        )

    job_id = message.get("job_id")
    if not isinstance(job_id, str) or not job_id:
        raise ValueError("job_id is missing or empty, so no reply could be attributed to it")
    clip_path = message.get("clip_path")
    if not isinstance(clip_path, str) or not clip_path:
        raise ValueError("clip_path is missing or empty")
    camera_id = message.get("camera_id")
    if camera_id is not None and not isinstance(camera_id, str):
        raise ValueError("camera_id must be a string or absent")

    asked = message.get("pose_model_variant")
    if asked is not None and asked != variant:
        raise ValueError(
            f"this worker is running {variant!r} and the job asks for {asked!r}; "
            "spawn a worker per variant"
        )

    frame_range = message.get("frame_range")
    if frame_range is not None:
        if (
            not isinstance(frame_range, list)
            or len(frame_range) != 2
            or not all(isinstance(n, int) and not isinstance(n, bool) for n in frame_range)
        ):
            raise ValueError("frame_range must be [start, end) or null")
    return job_id, Path(clip_path), camera_id, frame_range


def _run_job(
    out: IO[str],
    err: IO[str],
    job_id: str,
    clip_path: Path,
    camera_id: str | None,
    frame_range: list[int] | None,
    variant: str,
    model_path: Path,
) -> None:
    """Open the clip, ack its shape, pose it, and write exactly one `done` or one `failed`."""
    # The container is **entered** inside this `try`, not merely constructed in it. [M23 P5]
    #
    # `FileVideoSource.__init__` only stores a path; `__enter__` is what refuses one it cannot open.
    # So while the `with` sat outside this block, `FileNotFoundError` escaped the one place that has
    # a reply for it: the traceback left `main`, the process exited non-zero, and the pool read a
    # missing clip as a **crashed worker** — retried once on a fresh interpreter, reported as a
    # crash, and the `clip_unreadable` clause 5 says never to retry was never sent. M23 P5's pool
    # found it, because `attempts` came back 2 for a file that does not exist; the tests here could
    # not, because they fake `_open_clip` and a fake raises from its constructor.
    #
    # `ExitStack` rather than re-indenting the body under a wider `try`: the close is still
    # guaranteed, and a `try` that spanned the whole job would catch an `OSError` from a *reply*
    # write and answer it with a second reply about the clip.
    stack = ExitStack()
    try:
        source = stack.enter_context(_open_clip(clip_path, camera_id))
    except Exception as exc:  # FileNotFoundError, OSError — a fact about the file, so not retried
        _send_failed(out, job_id, "clip_unreadable", str(exc))
        return

    with stack:
        # `FileVideoSource.__enter__` already refused a file it could not open, so anything here is
        # a container that opened and will not say how big its pictures are. It has to be refused
        # rather than acked: `Reply::Accepted` bounds `width` and `height` at `> 0` — they are
        # `ClipMetadata`'s bounds, and the pool validates every line on arrival — so a zero would
        # be read as a protocol violation, which reports the wrong thing about the wrong side.
        # `fps` cannot be zero here: `FileVideoSource` substitutes `_DEFAULT_FPS` for a container
        # that will not say, and this worker inherits that fallback rather than diverging from
        # `api/pipeline.py`, the code that wrote every file M23 P7 diffs against.
        if source.width <= 0 or source.height <= 0:
            _send_failed(
                out,
                job_id,
                "clip_unreadable",
                f"the container opened but reports {source.width}x{source.height}",
            )
            return

        # Clause 2's rule, and the half of it only this side can decide: the whole clip or nothing.
        # A worker that accepted a sub-range and posed the whole clip would return landmarks whose
        # `frame_index` meant something other than what the caller asked for, and `_auto_windows`
        # would window the wrong frames — a confident wrong answer. The count compared against is
        # the container's claim, which is what a caller computing a range from clip metadata would
        # have had in front of it too.
        if frame_range is not None and list(frame_range) != [0, source.frame_count]:
            _send_failed(
                out,
                job_id,
                "frame_range_unsupported",
                f"only the whole clip may be asked for: "
                f"[0, {source.frame_count}], not {frame_range}",
            )
            return

        _send_accepted(out, job_id, source)

        # Read before posing, because `frames()` leaves the capture handle at the end of the file
        # and the reply's `ClipMetadata` still has to carry them.
        fps, width, height = source.fps, source.width, source.height
        try:
            frames = _estimate(source.frames(), model_path, variant)
        except Exception as exc:
            # An exception out of a long-lived C++ graph is the case clause 5 retries on a fresh
            # process, so the traceback goes to stderr — which the pool keeps per worker and
            # attaches to the failure it reports. That is what makes a `pose_failed` actionable.
            # `file=err` rather than the default `sys.stderr`: the two are the same stream in the
            # real process, but the worker's logging belongs on the stream it was handed, and only
            # that makes the guarantee testable.
            traceback.print_exc(file=err)
            _send_failed(out, job_id, "pose_failed", f"{type(exc).__name__}: {exc}")
            return

    if not frames:
        # A container that opened and then decoded nothing. A fact about the file, so not retried.
        _send_failed(out, job_id, "clip_empty", "the clip opened but decoded no frames")
        return

    _send_done(
        out,
        job_id,
        KeypointsFile(
            clip=ClipMetadata(
                fps=fps,
                width=width or None,
                height=height or None,
                frame_count=len(frames),
                source_sha256=_sha256(clip_path),
            ),
            frames=frames,
            pose_estimator=pose_estimator_name(variant),
        ),
    )


def main(
    *,
    stdin: IO[str] | None = None,
    stdout: IO[str] | None = None,
    stderr: IO[str] | None = None,
) -> int:
    """Handshake, then one job per line until stdin closes. Returns the process exit code.

    The streams are arguments so the loop can be driven over a pair of `os.pipe()`s by
    `tests/pose/test_worker.py`: the framing, the flushing and the two-replies-per-job order are
    the things most worth testing over a real pipe, and a test that reached for a subprocess
    instead would have no way to install a fake `estimate_pose` inside it.
    """
    stdin = stdin if stdin is not None else sys.stdin
    out = stdout if stdout is not None else sys.stdout
    err = stderr if stderr is not None else sys.stderr
    for stream in (stdin, out):
        # `json.dumps` escapes to ASCII, but `model_dump_json` emits UTF-8, so a non-ASCII
        # `camera_id` would raise on Windows' cp1252 stdout. `newline="\n"` is the other half:
        # without it Windows translates every `\n` to `\r\n`, which `Lines::read` trims but which
        # puts a stray byte on a 16.4 MB line for no reason.
        reconfigure = getattr(stream, "reconfigure", None)
        if reconfigure is not None:
            reconfigure(encoding="utf-8", newline="\n")

    try:
        variant, model_path = _startup(err)
    except _Unavailable as exc:
        _reply(out, {"pose": "unavailable", "reason": exc.reason, "detail": exc.detail})
        return 1

    _reply(
        out,
        {
            "pose": "ready",
            "protocol": PROTOCOL_VERSION,
            "estimator": pose_estimator_name(variant),
            "variant": variant,
            "model": str(model_path),
        },
    )

    # `sys.stdout` points at stderr from here on, and replies go to the stream captured above. A
    # dependency that prints — OpenCV and MediaPipe both can — would otherwise land a line of human
    # text in the middle of the protocol and be reported as a corrupted channel (clause 1).
    with redirect_stdout(err):
        while True:
            line = stdin.readline()
            if not line:
                return 0  # EOF: the pool closed our stdin, which is how it says shut down.
            if not line.strip():
                continue  # `Lines::read` skips whitespace-only lines too, in the same spirit.
            try:
                message = json.loads(line)
            except json.JSONDecodeError as exc:
                # No job_id to echo, so the reply carries an empty one. The pool hands a worker one
                # job at a time — clause 4's two replies are per job and a `Worker` owns one child
                # — so an unattributed failure is still attributable to whatever is in flight.
                # Exiting instead was declined: EOF tells the pool a worker died and nothing about
                # why, and the why is the whole value of this line.
                _send_failed(out, "", "bad_job", f"line is not JSON: {exc}")
                continue
            try:
                job = _resolve_job(message, variant)
            except ValueError as exc:
                echo = message.get("job_id") if isinstance(message, dict) else None
                _send_failed(out, echo if isinstance(echo, str) else "", "bad_job", str(exc))
                continue
            _run_job(out, err, *job, variant, model_path)


if __name__ == "__main__":
    sys.exit(main())
