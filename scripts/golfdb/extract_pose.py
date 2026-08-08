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

Serialization matches `scripts/run_pose.py` — the same `{"clip": ..., "frames": [...]}` envelope, so
these files load through `storage.keypoints_io.load_keypoints` and can be pushed through
`analyze_swing` unchanged — just written compact and rounded, because a thousand indented
full-precision clips is gigabytes. Clips extracted before the envelope existed are bare arrays and
still load; nothing forces a re-extraction.

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


def _serialize(keypoints: list, path: Path, clip: dict | None = None) -> None:
    """Write keypoints in the run_pose.py envelope, compact and rounded.

    Hand-built rather than routed through `save_keypoints` because the rounding above is what keeps
    a thousand clips from being gigabytes, and pydantic will not round on the way out. The shape is
    the same one `load_keypoints` reads, which is what matters — and clips already in the cache
    stay bare arrays, which that loader also accepts, so no re-extraction is forced.
    """
    import json

    frames = [
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
    payload = {"frames": frames}
    if clip is not None:
        payload = {"clip": clip, "frames": frames}
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
    from golf_coach.storage.manifest import hash_file

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

        # `list()` is safe here and nowhere else in the repo: videos_160 clips are 160px square
        # crops of ~200 frames, ~15 MB in total. `estimators.Estimator` takes a Sequence (the
        # RTMPose adapter indexes it), so streaming would be a signature change for no gain.
        with FileVideoSource(clip) as source:
            clip_meta = {
                "fps": source.fps,
                "width": source.width or None,
                "height": source.height or None,
                "source_sha256": hash_file(clip),
            }
            frames = list(source.frames())
        if not frames:
            missing += 1
            continue

        clip_meta["frame_count"] = len(frames)
        _serialize(estimator(frames), target, {k: v for k, v in clip_meta.items() if v is not None})
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
