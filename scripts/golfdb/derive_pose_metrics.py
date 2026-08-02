"""Measure pose metrics over the reference corpus, at ground-truth instants. [M4-REF Phase B3]

Usage:
    python scripts/golfdb/derive_pose_metrics.py --estimator mediapipe:lite

Reads cached keypoints (Tier 1) plus GolfDB's annotated events, computes `head_sway_norm` and
`finish_balance_norm` for each clip, and merges them back into `swings.jsonl` (Tier 2) so
`derive_reference.py` can aggregate them into bands.

**Phases come from GolfDB's labels, never from `segment_phases`.** That isolates the *estimator*
from our *segmentation*: a wide sway distribution then means "this estimator on tour swings", not
"our top-detection drifted". It also means these bands stay valid across future segmentation
changes.

The checkpoint code is reused verbatim — we call `evaluate_head_sway` / `evaluate_finish_balance`
and keep their `observed` values, so the corpus is measured by exactly the code that will measure
the user's swing. Reimplementing the metrics here would be the fastest possible way to produce
bands that quietly do not match what they are compared against.
"""

from __future__ import annotations

import argparse
import statistics
import sys

import common
import extract_pose
from pydantic import TypeAdapter

from golf_coach.analysis.checkpoints import evaluate_finish_balance, evaluate_head_sway
from golf_coach.contracts.keypoints import FrameKeypoints, Landmark
from golf_coach.contracts.reference import ReferenceSwing
from golf_coach.contracts.swing import PhaseSegment, SwingPhase

# Half-widths matching phases.py, so ground-truth-derived windows have the same shape as the ones
# the checkpoints normally receive.
_TRANSITION_HALF_FRAMES = 3
_IMPACT_HALF_FRAMES = 2

_ADAPTER = TypeAdapter(list[FrameKeypoints])


def _phases_from_events(events: dict[str, int], origin: int, n: int) -> list[PhaseSegment] | None:
    """Build our six `PhaseSegment`s from GolfDB's annotated instants.

    GolfDB labels eight events; we need six contiguous phases. The mapping is direct for the
    boundaries we care about — address, top, impact, finish — with the same small symmetric windows
    `phases.py` puts around the top and impact. Timestamps are frame indices scaled by a nominal
    rate: only *ratios* are used downstream, and roughly half the corpus is slow-motion, so an
    absolute rate would be fiction.
    """
    address = events["address"] - origin
    top = events["top"] - origin
    impact = events["impact"] - origin
    finish = events["finish"] - origin
    if not 0 <= address < top < impact <= finish < n:
        return None

    bounds = [
        0,
        address,
        max(address, top - _TRANSITION_HALF_FRAMES),
        min(impact, top + _TRANSITION_HALF_FRAMES),
        impact,
        min(n - 1, impact + _IMPACT_HALF_FRAMES),
        n - 1,
    ]
    for index in range(1, len(bounds)):
        bounds[index] = max(bounds[index], bounds[index - 1])

    order = [
        SwingPhase.ADDRESS,
        SwingPhase.BACKSWING,
        SwingPhase.TRANSITION,
        SwingPhase.DOWNSWING,
        SwingPhase.IMPACT,
        SwingPhase.FOLLOW_THROUGH,
    ]
    return [
        PhaseSegment(
            phase=phase,
            start_frame=bounds[i],
            end_frame=bounds[i + 1],
            start_ms=float(bounds[i]),
            end_ms=float(bounds[i + 1]),
        )
        for i, phase in enumerate(order)
    ]


def _apply_pixel_aspect(keypoints: list[FrameKeypoints], aspect: float) -> list[FrameKeypoints]:
    """Rescale `y` so it shares units with `x` (see `ReferenceSwing.pixel_aspect`).

    Applied to the keypoints rather than inside a metric, so the checkpoint code stays untouched
    and the correction is visibly a property of *this corpus*, not of the metric.
    """
    return [
        frame.model_copy(
            update={
                "landmarks": [
                    Landmark(x=lm.x, y=lm.y * aspect, z=lm.z, visibility=lm.visibility)
                    for lm in frame.landmarks
                ]
            }
        )
        for frame in keypoints
    ]


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description="Derive pose metrics at GolfDB ground truth.")
    parser.add_argument("--estimator", default="mediapipe:lite")
    parser.add_argument(
        "--no-aspect-correction",
        action="store_true",
        help="skip the pixel-aspect rescale, to see how much the 16:9 assumption is worth",
    )
    args = parser.parse_args(argv)

    cache = common.KEYPOINTS_DIR / extract_pose.estimator_slug(args.estimator)
    if not cache.is_dir():
        print(f"error: no cache at {cache} — run extract_pose.py first", file=sys.stderr)
        return 1

    swings: list[ReferenceSwing] = common.load_swings()
    measured = skipped = 0
    raw: dict[str, list[float]] = {"head_sway_norm": [], "finish_balance_norm": []}

    for swing in swings:
        path = cache / f"{swing.source_id}.keypoints.json"
        if not path.exists():
            continue

        keypoints = _ADAPTER.validate_json(path.read_bytes())
        phases = _phases_from_events(swing.events, swing.events["start"], len(keypoints))
        if phases is None:
            skipped += 1
            continue

        corrected = (
            keypoints
            if args.no_aspect_correction
            else _apply_pixel_aspect(keypoints, swing.pixel_aspect)
        )

        sway = evaluate_head_sway(corrected, phases)
        balance = evaluate_finish_balance(corrected, phases)
        found = False
        if sway is not None and sway.observed is not None:
            swing.metrics["head_sway_norm"] = sway.observed
            raw["head_sway_norm"].append(sway.observed)
            found = True
        if balance is not None and balance.observed is not None:
            swing.metrics["finish_balance_norm"] = balance.observed
            raw["finish_balance_norm"].append(balance.observed)
            found = True
        if found:
            swing.pose_estimator = args.estimator
            measured += 1
        else:
            skipped += 1

    if args.no_aspect_correction:
        print("(dry run: aspect correction disabled, swings.jsonl NOT updated)")
    else:
        with common.SWINGS_JSONL.open("w", encoding="utf-8") as handle:
            for swing in swings:
                handle.write(swing.model_dump_json() + "\n")
        print(f"Updated {common.SWINGS_JSONL}")

    print(f"Measured {measured} clip(s), skipped {skipped} (unusable phases or landmarks)")
    for metric, values in raw.items():
        if not values:
            continue
        ordered = sorted(values)
        print(
            f"  {metric:<22} n={len(values):>4}  median={statistics.median(ordered):.3f}  "
            f"p90={ordered[int(0.9 * (len(ordered) - 1))]:.3f}  max={ordered[-1]:.3f}"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
