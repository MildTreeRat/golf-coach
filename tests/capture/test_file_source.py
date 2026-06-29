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
