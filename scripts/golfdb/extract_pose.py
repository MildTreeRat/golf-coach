"""Extract pose keypoints over the GolfDB clip corpus. [M4-REF Phase B2]

Usage:
    python scripts/golfdb/extract_pose.py --videos ~/Downloads/videos_160/videos_160
    python scripts/golfdb/extract_pose.py --videos <dir> --estimator rtmpose:m --limit 100

Writes Tier 1 of the reference cache: one keypoints file per clip under
`data/reference/golfdb/keypoints/<estimator>/` (gitignored).

**This is the expensive step and the reason the cache exists.** Running an estimator over the
corpus costs minutes-to-hours; everything downstream — tuning segmentation, comparing estimators,
deriving a new pose metric that nobody has thought of yet — is then a cheap pass over JSON. Keeping
the keypoints means a future checkpoint can be measured across a thousand tour swings without
touching a video again.

Serialization matches `scripts/run_pose.py` exactly (`TypeAdapter(list[FrameKeypoints])`) so these
files load through the ordinary loader and can be pushed through `analyze_swing` unchanged — just
written compact and rounded, because a thousand indented full-precision clips is gigabytes.

Resumable: clips already extracted are skipped, so an interrupted run costs nothing.
"""

from __future__ import annotations

import argparse
import sys
import time
from pathlib import Path

import common
import estimators

from golf_coach.contracts.reference import ReferenceSwing

# Landmark coordinates are normalized to the frame; five decimals is ~1/100th of a pixel on a
# 1080p frame, far below what any estimator resolves. Full float repr would roughly double the
# corpus for no recoverable information.
_COORD_DECIMALS = 5


def estimator_slug(name: str) -> str:
    """Filesystem-safe directory name for an estimator (`mediapipe:lite` -> `mediapipe-lite`)."""
    return name.replace(":", "-")


def _serialize(keypoints: list, path: Path) -> None:
    """Write keypoints as a compact JSON array, rounded, matching the run_pose.py schema."""
    import json

    payload = [
        {
            "frame_index": frame.frame_index,
            "timestamp_ms": round(frame.timestamp_ms, 3),
            "landmarks": [
                {
                    "x": round(lm.x, _COORD_DECIMALS),
                    "y": round(lm.y, _COORD_DECIMALS),
                    "z": round(lm.z, _COORD_DECIMALS),
                    "visibility": round(lm.visibility, 4),
                }
                for lm in frame.landmarks
            ],
        }
        for frame in keypoints
    ]
    path.write_text(json.dumps(payload, separators=(",", ":")), encoding="utf-8")


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description="Extract pose over GolfDB clips into the cache.")
    parser.add_argument("--videos", required=True, type=Path, help="videos_160 directory")
    parser.add_argument(
        "--estimator", default="mediapipe:lite", choices=list(estimators.CANDIDATES)
    )
    parser.add_argument(
        "--view",
        default=common.VIEW_FACE_ON,
        help="camera view to extract, or 'all' (default: face-on, our canonical placement)",
    )
    parser.add_argument("--limit", type=int, default=0, help="stop after N clips (0 = no limit)")
    parser.add_argument(
        "--sample",
        type=int,
        default=0,
        help="extract the bake-off sample of N clips instead of everything — the same "
        "round-robin selection bakeoff.py scores, so estimators are compared on identical clips",
    )
    parser.add_argument("--force", action="store_true", help="re-extract clips already cached")
    args = parser.parse_args(argv)

    videos_dir = args.videos.expanduser()
    if not videos_dir.is_dir():
        print(f"error: {videos_dir} is not a directory", file=sys.stderr)
        return 1
    if not common.SWINGS_JSONL.exists():
        print("error: run scripts/golfdb/ingest_labels.py first", file=sys.stderr)
        return 1

    from golf_coach.capture.file import FileVideoSource

    swings: list[ReferenceSwing] = common.load_swings()
    if args.sample:
        swings = common.bakeoff_sample(swings, args.sample)
    else:
        if args.view != "all":
            swings = [s for s in swings if s.view == args.view]
        swings.sort(key=lambda s: int(s.source_id))
        if args.limit:
            swings = swings[: args.limit]

    out_dir = common.KEYPOINTS_DIR / estimator_slug(args.estimator)
    out_dir.mkdir(parents=True, exist_ok=True)
    print(f"{args.estimator}: {len(swings)} clip(s), view={args.view} -> {out_dir}")

    estimator = estimators.CANDIDATES[args.estimator]()
    extracted = skipped = missing = 0
    frames_seen = 0
    started = time.perf_counter()

    for position, swing in enumerate(swings, start=1):
        target = out_dir / f"{swing.source_id}.keypoints.json"
        if target.exists() and not args.force:
            skipped += 1
            continue

        clip = common.clip_path(videos_dir, swing.source_id)
        if not clip.exists():
            missing += 1
            continue

        with FileVideoSource(clip) as source:
            frames = list(source.frames())
        if not frames:
            missing += 1
            continue

        _serialize(estimator(frames), target)
        frames_seen += len(frames)
        extracted += 1

        if position % 25 == 0:
            rate = frames_seen / max(time.perf_counter() - started, 1e-9)
            print(
                f"  {position}/{len(swings)}  ({extracted} new, {skipped} cached)  {rate:.0f} fps"
            )

    elapsed = time.perf_counter() - started
    print(
        f"\nExtracted {extracted}, skipped {skipped} cached, {missing} missing "
        f"in {elapsed:.0f}s ({frames_seen / elapsed:.0f} fps)"
        if elapsed > 0
        else f"\nExtracted {extracted}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
