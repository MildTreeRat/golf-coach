"""The render's readback probe. Skipped cleanly when the vision extra (OpenCV) isn't installed.

Covers the one output this repo writes that nothing downstream reads back: `aligned.mp4` is
consumed by a browser, days later, by a human, so a silent encoder failure surfaces nowhere near
the run that caused it. See `side_by_side.probe_render`.
"""

from __future__ import annotations

from pathlib import Path

import pytest

cv2 = pytest.importorskip("cv2")
np = pytest.importorskip("numpy")

from golf_coach.contracts.alignment import (  # noqa: E402  (after importorskip)
    AlignmentQuality,
    FramePairing,
)
from golf_coach.contracts.keypoints import (  # noqa: E402
    NUM_POSE_LANDMARKS,
    FrameKeypoints,
    Landmark,
)
from golf_coach.pose.side_by_side import (  # noqa: E402
    Panel,
    RenderResult,
    probe_render,
    render_side_by_side,
)

_FRAMES = 8


def _keypoints(count: int) -> list[FrameKeypoints]:
    """A standing skeleton, repeated. Nothing here reads the pose — only that it draws."""
    landmarks = [Landmark(x=0.5, y=0.5, visibility=1.0) for _ in range(NUM_POSE_LANDMARKS)]
    return [
        FrameKeypoints(frame_index=i, timestamp_ms=i * 16.0, landmarks=landmarks)
        for i in range(count)
    ]


def _render(out_path: Path, count: int = _FRAMES) -> RenderResult:
    schedule = [FramePairing(tau=i / count, frame_a=i, frame_b=i) for i in range(count)]
    keypoints = _keypoints(count)
    return render_side_by_side(
        out_path,
        schedule,
        Panel(keypoints, "face-on"),
        Panel(keypoints, "down-the-line"),
        fps=60.0,
        quality=AlignmentQuality.FULL,
    )


def test_a_render_reads_back_the_frames_it_wrote(tmp_path: Path) -> None:
    """The claim the log cannot make. [M8.3]

    `frames` is `len(schedule)` — what the renderer *intended*. Only `frames_read` has been
    through an encoder and a container, and the two agreeing is the whole assertion.
    """
    result = _render(tmp_path / "aligned.mp4")

    assert result.codec is not None
    assert result.frames == _FRAMES
    assert result.frames_read == result.frames


def test_an_unopenable_file_raises_rather_than_reading_as_empty(tmp_path: Path) -> None:
    """Zero frames and no video are different failures, and only one of them is recoverable.

    Returning 0 here would make an unwritten render indistinguishable from an empty schedule,
    which is a legitimate outcome (ADR-013 — the clips shared no swing time).
    """
    broken = tmp_path / "not-a-video.mp4"
    broken.write_bytes(b"this is not an mp4")

    with pytest.raises(RuntimeError, match="will not open"):
        probe_render(broken)


def test_an_empty_schedule_writes_nothing_and_probes_nothing(tmp_path: Path) -> None:
    """No file was written, so there is nothing to read back — and that is not a failure."""
    out_path = tmp_path / "aligned.mp4"

    result = render_side_by_side(out_path, [], Panel([], "a"), Panel([], "b"))

    assert result == RenderResult(0, None, None)
    assert not out_path.exists()
