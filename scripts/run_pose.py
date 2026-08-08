"""Dev CLI: run MediaPipe pose over a video file and write a skeleton-overlay clip. [M1]

Usage:
    python scripts/run_pose.py data/raw/my_swing.mp4
    python scripts/run_pose.py data/raw/face_on.mov --camera-id face_on

Reads the clip, runs pose estimation, then writes the keypoints as JSON and a
skeleton-overlay video side by side under data/processed/. This is the first visible,
hardware-free milestone artifact (ROADMAP M1).

**Frames are never accumulated** (M7 Phase 1). This used to do `list(source.frames())`, which
holds every decoded BGR frame at once — ~2.9 GB for a 1080p60 8-second clip and ~15 GB for 10 s
of 4K60, so the first real iPhone clip would have taken the machine down. It survived only
because the committed sample is 480x854. Both the pose pass and the overlay pass now stream, so
peak memory is one frame plus the keypoints (a few MB), whatever the clip.

The overlay needs the pixels a second time and `VideoSource` is a one-shot iterator, so the file
is simply decoded twice. Weighed against the alternatives: a single pass would mean turning
`estimate_pose` into a generator, but its `frames -> list[FrameKeypoints]` signature is fixed and
shared with the GolfDB bake-off, and a streaming variant can be added beside it later without
this commit foreclosing it; buffering just the frames the overlay wants still costs ~800 MB at
720p and does nothing for the 4K case. A second decode is measurably cheap — decode runs ~675 fps
against MediaPipe's ~106 (docs/M7_TWO_PHONE_SPIKE.md), so it is a few percent of the run.
"""

from __future__ import annotations

import argparse
import sys
from collections.abc import Iterable
from pathlib import Path

from golf_coach.capture.file import FileVideoSource
from golf_coach.capture.source import Frame
from golf_coach.config import settings
from golf_coach.contracts.keypoints import ClipMetadata, FrameKeypoints, KeypointsFile
from golf_coach.pose.estimator import estimate_pose
from golf_coach.pose.overlay import draw_skeleton
from golf_coach.storage.keypoints_io import save_keypoints
from golf_coach.storage.manifest import hash_file


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(
        description="Run MediaPipe pose over a video file into keypoints JSON + an overlay clip."
    )
    parser.add_argument("video", type=Path, help="video file to process")
    parser.add_argument(
        "--camera-id",
        default=None,
        help=(
            "which camera shot this clip, e.g. face_on or down_the_line (ADR-011). Recorded on "
            "every frame so a keypoints file stays attributable to a view once the pixels are gone."
        ),
    )
    args = parser.parse_args(argv)
    video_path: Path = args.video

    if not video_path.exists():
        print(f"error: {video_path} not found", file=sys.stderr)
        return 1

    # Pass 1: pose. The generator goes straight into estimate_pose, so no frame outlives its
    # iteration and the clip's resolution stops mattering to memory.
    with FileVideoSource(video_path, camera_id=args.camera_id) as source:
        fps, width, height = source.fps, source.width, source.height
        reported_frames = source.frame_count
        keypoints = estimate_pose(source.frames())

    if not keypoints:
        print(f"No frames read from {video_path}", file=sys.stderr)
        return 1

    # A file that opens cleanly and then decodes short produces a plausible-looking analysis of an
    # incomplete swing — the silent failure this guards (docs/M7_TWO_PHONE_SPIKE.md, Q2).
    if reported_frames and reported_frames != len(keypoints):
        print(
            f"warning: {video_path.name} reports {reported_frames} frames but decoded "
            f"{len(keypoints)}; the clip metadata records what was actually decoded.",
            file=sys.stderr,
        )

    settings.processed_dir.mkdir(parents=True, exist_ok=True)
    stem = video_path.stem

    json_path = settings.processed_dir / f"{stem}.keypoints.json"
    save_keypoints(
        KeypointsFile(
            clip=ClipMetadata(
                fps=fps,
                width=width or None,
                height=height or None,
                frame_count=len(keypoints),
                source_sha256=hash_file(video_path),
            ),
            frames=keypoints,
        ),
        json_path,
    )

    # Pass 2: overlay, decoding the file a second time rather than having held it.
    overlay_path = settings.processed_dir / f"{stem}.overlay.mp4"
    with FileVideoSource(video_path, camera_id=args.camera_id) as source:
        _write_overlay(source.frames(), keypoints, overlay_path, fps)

    print(f"Wrote {len(keypoints)} frames of keypoints -> {json_path}")
    print(f"Wrote skeleton overlay -> {overlay_path}")
    return 0


def _write_overlay(
    frames: Iterable[Frame], keypoints: list[FrameKeypoints], out_path: Path, fps: float
) -> None:
    """Render the skeleton over a *stream* of frames, sized from the first one that arrives."""
    import cv2

    writer: cv2.VideoWriter | None = None
    try:
        for frame, kp in zip(frames, keypoints, strict=False):
            if writer is None:
                height, width = frame.image.shape[:2]
                fourcc = cv2.VideoWriter_fourcc(*"mp4v")
                writer = cv2.VideoWriter(str(out_path), fourcc, fps, (width, height))
            writer.write(draw_skeleton(frame.image, kp))
    finally:
        if writer is not None:
            writer.release()


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
