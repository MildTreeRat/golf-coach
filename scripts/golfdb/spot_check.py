"""Render a contact sheet of extracted poses at ground-truth instants. [M4-REF verification]

Usage:
    python scripts/golfdb/spot_check.py --videos <dir>
    python scripts/golfdb/spot_check.py --videos <dir> --estimator mediapipe:full --clips 8

Writes `data/reference/golfdb/spot_check_<estimator>.png` (gitignored): one row per clip, one
column per GolfDB-annotated instant, skeleton drawn by the **production** `draw_skeleton`.

**Why this exists.** Every other check in this milestone is numerical — PCE against event labels,
percentiles, paired significance tests. All of them can look healthy while the underlying pose is
wrong in a way that happens not to move an event index. The bands in `ranges.json` are cut from
these keypoints, so at some point a human has to look at the skeletons. `videos_160` are 160x160
broadcast crops, the lowest-quality input anywhere in this project.

Frames come from the *ground-truth* event labels, not `segment_phases`, so a bad tile means bad
pose rather than bad segmentation — the same separation `derive_pose_metrics.py` relies on.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

import common
import extract_pose

from golf_coach.contracts.keypoints import FrameKeypoints
from golf_coach.contracts.reference import ReferenceSwing

# The instants the bands actually depend on: address and impact bound `head_sway`, follow-through
# is where `finish_balance` reads hips, and the top is what tempo is measured from.
INSTANTS = ("address", "top", "impact", "finish")

# 160x160 is too small to judge a skeleton by eye; upscale with nearest-neighbour so the joint
# markers stay crisp rather than being blurred into the interpolation.
_TILE = 260
_LABEL_STRIP = 22


def _tile(
    frames: list, keypoints: list[FrameKeypoints], index: int, label: str
) -> object:
    import cv2

    from golf_coach.pose.overlay import draw_skeleton

    index = max(0, min(index, len(frames) - 1, len(keypoints) - 1))
    canvas = draw_skeleton(frames[index], keypoints[index])
    canvas = cv2.resize(canvas, (_TILE, _TILE), interpolation=cv2.INTER_NEAREST)
    strip = cv2.copyMakeBorder(
        canvas, _LABEL_STRIP, 0, 0, 0, cv2.BORDER_CONSTANT, value=(0, 0, 0)
    )
    cv2.putText(
        strip, f"{label} @{index}", (4, 15), cv2.FONT_HERSHEY_SIMPLEX, 0.45, (255, 255, 255), 1
    )
    return strip


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description="Contact sheet of poses at ground-truth instants.")
    parser.add_argument("--videos", required=True, type=Path, help="videos_160 directory")
    parser.add_argument("--estimator", default="mediapipe:lite")
    parser.add_argument("--clips", type=int, default=5)
    args = parser.parse_args(argv)

    import cv2
    import numpy as np
    from pydantic import TypeAdapter

    from golf_coach.capture.file import FileVideoSource

    videos_dir = args.videos.expanduser()
    cache = common.KEYPOINTS_DIR / extract_pose.estimator_slug(args.estimator)
    if not cache.is_dir():
        print(f"error: no cache at {cache} — run extract_pose.py first", file=sys.stderr)
        return 1

    adapter = TypeAdapter(list[FrameKeypoints])
    swings: list[ReferenceSwing] = common.bakeoff_sample(common.load_swings(), args.clips)

    rows = []
    for swing in swings:
        path = cache / f"{swing.source_id}.keypoints.json"
        clip = common.clip_path(videos_dir, swing.source_id)
        if not path.exists() or not clip.exists():
            continue
        keypoints = adapter.validate_json(path.read_bytes())
        with FileVideoSource(clip) as source:
            frames = [f.image for f in source.frames()]
        if not frames:
            continue

        origin = swing.events["start"]
        row = [
            _tile(frames, keypoints, swing.events[name] - origin, f"{swing.source_id} {name}")
            for name in INSTANTS
        ]
        rows.append(np.hstack(row))
        print(f"  clip {swing.source_id}: {len(frames)} frames, {swing.club or '?'}")

    if not rows:
        print("error: nothing to render", file=sys.stderr)
        return 1

    sheet = np.vstack(rows)
    out = common.REFERENCE_DIR / f"spot_check_{extract_pose.estimator_slug(args.estimator)}.png"
    out.parent.mkdir(parents=True, exist_ok=True)
    cv2.imwrite(str(out), sheet)
    print(f"\nWrote {out}  ({len(rows)} clips x {len(INSTANTS)} instants)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
