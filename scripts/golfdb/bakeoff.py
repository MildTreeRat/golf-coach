"""Pose-estimator bake-off against GolfDB ground-truth events. [M4-REF Phase B0]

Usage:
    python scripts/golfdb/extract_pose.py --videos <dir> --estimator <name>   # once per estimator
    python scripts/golfdb/bakeoff.py
    python scripts/golfdb/bakeoff.py --clips 40 --only mediapipe:lite,rtmpose:m

Locks the estimator *before* the full corpus extraction, because whatever produces the reference
corpus must also process the user's swings — otherwise the common-mode bias that justifies
comparing the two doesn't cancel, and re-running hundreds of clips is expensive.

**GolfDB supplies the test set for free.** There are no pose labels to score against, but there are
hand-annotated *event* frames — so we score each estimator by how well the real
`smooth_keypoints -> segment_phases` path recovers address / top / impact. That measures the thing
we care about (does this pose feed our metrics?) rather than COCO AP on generic photographs, and it
holds every stage but the estimator fixed.

### Reported measures

- **`med_err`** — median absolute error in frames. Directly interpretable, but not comparable
  across a corpus that is ~46% slow-motion, where a frame is worth much less time.
- **`med_norm`** — the same error as a fraction of that swing's own address-to-impact duration.
  **The headline number**: fps-invariant, so slow-motion and real-time clips are judged alike.
- **`PCE`** — percentage of correct events, GolfDB's own metric, at tolerance
  `delta = max(round(n / 30), 1)` frames where `n` is the ground-truth address-to-impact span.
  Included so the numbers can be read next to published ones; the constant is our reading of their
  convention, not a value they state, so prefer `med_norm` when the two disagree.
- **`med_err_frames_by_speed`** — `med_err` split into slow-motion and real-time clips. The pooled
  frame median mixes two populations a frame does not mean the same thing in: every instant scales
  with capture speed (address 4 vs 17 frames, top 1 vs 4, impact 0 vs 3), so the pooled number
  partly reports how much slow-motion is in the sample. Prefer `med_norm`, which is already
  fps-invariant; this split exists to make the effect visible rather than to be optimized against.
  It is what exposed the fixed frame count in the address rule (M4-REF Phase B6).
"""

from __future__ import annotations

import argparse
import json
import statistics
import sys
import time
from typing import Any

import common
import estimators
import extract_pose

from golf_coach.analysis.phases import segment_phases
from golf_coach.analysis.smoothing import smooth_keypoints
from golf_coach.contracts.reference import ReferenceSwing
from golf_coach.contracts.swing import PhaseSegment, SwingPhase

DEFAULT_CLIPS = 100
OUTPUT_JSON = common.REPO_ROOT / "docs" / "pose_bakeoff_v1.json"

# GolfDB's PCE tolerance scales with swing duration so slow-motion and real-time clips are held to
# the same standard. See the module docstring on the constant.
_PCE_TOLERANCE_DIVISOR = 30

EVENTS = ("address", "top", "impact")


def _detected(phases: list[PhaseSegment]) -> dict[str, int] | None:
    """Address / top / impact as `analyze_swing.py` reports them, from the phase boundaries."""
    by_phase = {segment.phase: segment for segment in phases}
    address = by_phase.get(SwingPhase.ADDRESS)
    transition = by_phase.get(SwingPhase.TRANSITION)
    impact = by_phase.get(SwingPhase.IMPACT)
    if address is None or transition is None or impact is None:
        return None
    return {
        "address": address.end_frame,
        "top": (transition.start_frame + transition.end_frame) // 2,
        "impact": impact.start_frame,
    }


def _evaluate(name: str, swings: list[ReferenceSwing]) -> dict[str, Any]:
    """Score one estimator's cached keypoints on how well `segment_phases` recovers the events.

    Reads the Tier 1 cache written by `extract_pose.py` rather than running the estimator here.
    Extraction is the expensive, one-time part; scoring is a fast pass over JSON, so a change to
    the segmentation logic can be re-scored across every estimator in seconds.
    """
    from golf_coach.storage.keypoints_io import load_keypoints

    print(f"\n=== {name} ===")
    cache = common.KEYPOINTS_DIR / extract_pose.estimator_slug(name)
    if not cache.is_dir():
        raise FileNotFoundError(
            f"no cache at {cache} — run: python scripts/golfdb/extract_pose.py --estimator {name}"
        )

    errors: dict[str, list[int]] = {e: [] for e in EVENTS}
    normalized: dict[str, list[float]] = {e: [] for e in EVENTS}
    correct: dict[str, int] = dict.fromkeys(EVENTS, 0)
    # Split the raw frame errors by capture speed. A pooled frame median over a corpus that is
    # ~47% broadcast slow-motion substantially measures the corpus mix rather than the rule —
    # the effect is small for top and impact and dominant for address (M4-REF Phase B6).
    by_speed: dict[str, dict[str, list[int]]] = {
        e: {"slow_motion": [], "real_time": []} for e in EVENTS
    }
    scored = 0
    unsegmentable = 0
    missing = 0
    started = time.perf_counter()
    frames_seen = 0

    for position, swing in enumerate(swings, start=1):
        path = cache / f"{swing.source_id}.keypoints.json"
        if not path.exists():
            missing += 1
            continue

        # Cached clips predate the clip-metadata envelope and load as bare arrays; newly
        # extracted ones are enveloped. `load_keypoints` takes either.
        keypoints = load_keypoints(path).frames
        if len(keypoints) < 6:
            missing += 1
            continue
        frames_seen += len(keypoints)

        detected = _detected(segment_phases(smooth_keypoints(keypoints)))
        if detected is None:
            unsegmentable += 1
            continue

        # GolfDB event frames are absolute in the source video; videos_160 clips are already
        # trimmed to `start`, so rebase onto the clip the same way upstream's dataloader does.
        origin = swing.events["start"]
        truth = {e: swing.events[e] - origin for e in EVENTS}
        span = truth["impact"] - truth["address"]
        # A few videos_160 clips are shorter than their own annotation span, so the labelled
        # impact frame does not exist in the file. Scoring those measures the mismatch, not us.
        if span <= 0 or truth["impact"] >= len(keypoints):
            unsegmentable += 1
            continue
        tolerance = max(round(span / _PCE_TOLERANCE_DIVISOR), 1)

        stratum = "slow_motion" if swing.slow_motion else "real_time"
        for event in EVENTS:
            delta = abs(detected[event] - truth[event])
            errors[event].append(delta)
            normalized[event].append(delta / span)
            by_speed[event][stratum].append(delta)
            if delta <= tolerance:
                correct[event] += 1
        scored += 1

        if position % 20 == 0:
            print(f"  {position}/{len(swings)} clips...")

    elapsed = time.perf_counter() - started
    result: dict[str, Any] = {
        "estimator": name,
        "clips_scored": scored,
        "clips_unsegmentable": unsegmentable,
        "clips_not_cached": missing,
        "frames": frames_seen,
        "scoring_seconds": round(elapsed, 1),
        "events": {},
    }
    for event in EVENTS:
        if not errors[event]:
            continue
        result["events"][event] = {
            "med_err_frames": round(statistics.median(errors[event]), 2),
            "med_norm_err": round(statistics.median(normalized[event]), 4),
            "pce": round(100.0 * correct[event] / scored, 1) if scored else 0.0,
            "med_err_frames_by_speed": {
                stratum: round(statistics.median(deltas), 2)
                for stratum, deltas in by_speed[event].items()
                if deltas
            },
        }

    if scored:
        result["mean_norm_err"] = round(
            statistics.fmean(statistics.median(normalized[e]) for e in EVENTS), 4
        )
        result["mean_pce"] = round(
            statistics.fmean(result["events"][e]["pce"] for e in EVENTS if e in result["events"]),
            1,
        )
    _print_result(result)
    return result


def _print_result(result: dict[str, Any]) -> None:
    print(
        f"  scored {result['clips_scored']}  excluded {result['clips_unsegmentable']}  "
        f"not cached {result['clips_not_cached']}"
    )
    for event, stats in result["events"].items():
        by_speed = stats.get("med_err_frames_by_speed", {})
        split = "  ".join(f"{k}={v:g}fr" for k, v in by_speed.items())
        print(
            f"    {event:<8} med_err={stats['med_err_frames']:>6} fr   "
            f"med_norm={stats['med_norm_err']:>7.3f}   PCE={stats['pce']:>5.1f}%"
            + (f"   [{split}]" if split else "")
        )
    if "mean_pce" in result:
        print(f"    {'MEAN':<8} med_norm={result['mean_norm_err']:>7.3f}   "
              f"PCE={result['mean_pce']:>5.1f}%")


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description="Pose-estimator bake-off on GolfDB face-on clips.")
    parser.add_argument("--clips", type=int, default=DEFAULT_CLIPS)
    parser.add_argument(
        "--only",
        default="",
        help="comma-separated estimator names (default: all in the registry)",
    )
    parser.add_argument(
        "--merge",
        action="store_true",
        help="keep runs already in pose_bakeoff_v1.json instead of replacing them, so the "
        "four-way comparison and the full-corpus one can live in the same record",
    )
    args = parser.parse_args(argv)

    swings = common.load_swings()
    sample = common.bakeoff_sample(swings, args.clips)

    names = [n.strip() for n in args.only.split(",") if n.strip()] or list(estimators.CANDIDATES)
    unknown = [n for n in names if n not in estimators.CANDIDATES]
    if unknown:
        print(f"error: unknown estimator(s) {unknown}", file=sys.stderr)
        return 1

    # Score only clips cached for *every* estimator under comparison. A variant judged on a
    # different (or smaller) clip set would be compared on the clips, not on its pose quality.
    available = [
        {p.name.split(".")[0] for p in (common.KEYPOINTS_DIR / extract_pose.estimator_slug(n))
         .glob("*.keypoints.json")}
        for n in names
    ]
    shared = set.intersection(*available) if available else set()
    dropped = len(sample) - len([s for s in sample if s.source_id in shared])
    sample = [s for s in sample if s.source_id in shared]

    videos = len({s.group_id for s in sample})
    players = len({s.subject for s in sample})
    print(
        f"Sample: {len(sample)} face-on clips from {videos} source videos, {players} golfers"
        + (f"  ({dropped} dropped — not cached for every estimator)" if dropped else "")
    )
    if not sample:
        print("error: no clips cached for all requested estimators", file=sys.stderr)
        return 1

    results = []
    for name in names:
        try:
            results.append(_evaluate(name, sample))
        except Exception as exc:  # noqa: BLE001 - one bad backend must not lose the others
            print(f"  {name}: FAILED — {type(exc).__name__}: {exc}", file=sys.stderr)
            results.append({"estimator": name, "error": f"{type(exc).__name__}: {exc}"})

    run = {
        "sample": {
            "clips": len(sample),
            "source_videos": videos,
            "golfers": players,
            "view": common.VIEW_FACE_ON,
            "selection": "round-robin across source videos, deterministic",
        },
        "estimators": names,
        "results": results,
    }

    # Keep prior runs. The four-way comparison is only possible at the sample size every estimator
    # is cached for, while the decisive lite-vs-full comparison needs the whole corpus — so the
    # record has to hold both, keyed by the clip count they were measured at.
    runs: list[dict[str, Any]] = []
    if args.merge and OUTPUT_JSON.exists():
        previous = json.loads(OUTPUT_JSON.read_text(encoding="utf-8"))
        runs = [
            existing
            for existing in previous.get("runs", [])
            if (existing["sample"]["clips"], existing.get("estimators")) != (len(sample), names)
        ]
    runs.append(run)
    runs.sort(key=lambda r: r["sample"]["clips"])

    payload = {
        "schema_version": 2,
        "pce_tolerance": f"max(round(n/{_PCE_TOLERANCE_DIVISOR}), 1) frames, n = address..impact",
        "runs": runs,
    }
    OUTPUT_JSON.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
    print(f"\nWrote {OUTPUT_JSON} ({len(runs)} run(s))")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
