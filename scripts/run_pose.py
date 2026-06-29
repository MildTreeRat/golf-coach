"""Dev CLI: run MediaPipe pose over a video file and write a skeleton-overlay clip. [M1]

Usage:
    python scripts/run_pose.py data/raw/my_swing.mp4

Reads the clip, runs pose estimation, then writes the keypoints as JSON and a
skeleton-overlay video side by side under data/processed/. This is the first visible,
hardware-free milestone artifact (ROADMAP M1).
"""

from __future__ import annotations

import sys
from pathlib import Path

from pydantic import TypeAdapter

from golf_coach.capture.file import FileVideoSource
from golf_coach.capture.source import Frame
from golf_coach.config import settings
from golf_coach.contracts.keypoints import FrameKeypoints
from golf_coach.pose.estimator import estimate_pose
from golf_coach.pose.overlay import draw_skeleton

_KEYPOINTS_ADAPTER = TypeAdapter(list[FrameKeypoints])


def main(argv: list[str]) -> int:
    if len(argv) != 1:
        print("usage: python scripts/run_pose.py <video-file>", file=sys.stderr)
        return 2

    video_path = Path(argv[0])

    # Buffer frames once so both pose estimation and the overlay see the same pixels
    # (a VideoSource is a one-shot iterator). M1 clips are a few seconds, so this fits
    # comfortably in memory.
    with FileVideoSource(video_path) as source:
        fps = source.fps
        frames = list(source.frames())

    if not frames:
        print(f"No frames read from {video_path}", file=sys.stderr)
        return 1

    keypoints = estimate_pose(frames)

    settings.processed_dir.mkdir(parents=True, exist_ok=True)
    stem = video_path.stem

    json_path = settings.processed_dir / f"{stem}.keypoints.json"
    json_path.write_bytes(_KEYPOINTS_ADAPTER.dump_json(keypoints, indent=2))

    overlay_path = settings.processed_dir / f"{stem}.overlay.mp4"
    _write_overlay(frames, keypoints, overlay_path, fps)

    print(f"Wrote {len(keypoints)} frames of keypoints -> {json_path}")
    print(f"Wrote skeleton overlay -> {overlay_path}")
    return 0


def _write_overlay(
    frames: list[Frame], keypoints: list[FrameKeypoints], out_path: Path, fps: float
) -> None:
    import cv2

    height, width = frames[0].image.shape[:2]
    fourcc = cv2.VideoWriter_fourcc(*"mp4v")
    writer = cv2.VideoWriter(str(out_path), fourcc, fps, (width, height))
    try:
        for frame, kp in zip(frames, keypoints, strict=True):
            writer.write(draw_skeleton(frame.image, kp))
    finally:
        writer.release()


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
