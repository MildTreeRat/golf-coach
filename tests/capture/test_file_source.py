"""FileVideoSource tests. Skipped cleanly when the vision extra (OpenCV) isn't installed."""

from __future__ import annotations

from pathlib import Path

import pytest

cv2 = pytest.importorskip("cv2")
np = pytest.importorskip("numpy")

from golf_coach.capture.file import FileVideoSource  # noqa: E402  (after importorskip)


def _write_clip(
    path: Path, frame_count: int, fps: float = 30.0, size: tuple[int, int] = (64, 48)
) -> None:
    width, height = size
    fourcc = cv2.VideoWriter_fourcc(*"mp4v")
    writer = cv2.VideoWriter(str(path), fourcc, fps, (width, height))
    if not writer.isOpened():
        pytest.skip("No mp4 encoder available in this environment")
    try:
        for i in range(frame_count):
            writer.write(np.full((height, width, 3), i % 256, dtype=np.uint8))
    finally:
        writer.release()


def test_yields_ordered_frames_with_timestamps(tmp_path: Path) -> None:
    clip = tmp_path / "clip.mp4"
    _write_clip(clip, frame_count=10, fps=30.0)

    with FileVideoSource(clip) as src:
        assert src.fps == pytest.approx(30.0, abs=1.0)
        frames = list(src.frames())

    assert len(frames) >= 1  # codecs may drop a frame; at least produces output
    indices = [f.index for f in frames]
    assert indices == sorted(indices)
    assert indices[0] == 0
    for f in frames:
        assert f.timestamp_ms == pytest.approx(f.index / 30.0 * 1000.0, abs=1.0)


def test_missing_file_raises(tmp_path: Path) -> None:
    with pytest.raises(FileNotFoundError):
        with FileVideoSource(tmp_path / "nope.mp4"):
            pass


def test_camera_id_defaults_to_none(tmp_path: Path) -> None:
    """Unlabelled is the honest default — most clips in this repo predate the idea."""
    clip = tmp_path / "clip.mp4"
    _write_clip(clip, frame_count=5)

    with FileVideoSource(clip) as src:
        assert src.camera_id is None
        assert all(frame.camera_id is None for frame in src.frames())


def test_camera_id_is_stamped_on_every_frame(tmp_path: Path) -> None:
    """The ADR-011 seam: one camera per source, so it is set once and rides every frame."""
    clip = tmp_path / "clip.mp4"
    _write_clip(clip, frame_count=5)

    with FileVideoSource(clip, camera_id="down_the_line") as src:
        assert src.camera_id == "down_the_line"
        frames = list(src.frames())

    assert frames
    assert all(frame.camera_id == "down_the_line" for frame in frames)


def test_reports_container_dimensions_and_length(tmp_path: Path) -> None:
    clip = tmp_path / "clip.mp4"
    _write_clip(clip, frame_count=10, size=(64, 48))

    with FileVideoSource(clip) as src:
        assert (src.width, src.height) == (64, 48)
        # The container's claim; codecs may yield a frame more or less than they advertise.
        assert src.frame_count == pytest.approx(10, abs=1)


def test_source_can_be_iterated_twice_by_reopening(tmp_path: Path) -> None:
    """What the two-pass CLIs rely on instead of buffering every decoded frame."""
    clip = tmp_path / "clip.mp4"
    _write_clip(clip, frame_count=8)

    with FileVideoSource(clip) as src:
        first = [f.index for f in src.frames()]
    with FileVideoSource(clip) as src:
        second = [f.index for f in src.frames()]

    assert first == second
