"""Measure pose metrics over the reference corpus, at ground-truth instants. [M4-REF Phase B3]

Usage:
    python scripts/golfdb/derive_pose_metrics.py --estimator mediapipe:lite

Reads cached keypoints (Tier 1) plus GolfDB's annotated events, computes **every metric in
`analysis.measure.POSE_MEASUREMENTS`** for each clip, and merges them back into `swings.jsonl`
(Tier 2) so `derive_reference.py` can aggregate them into bands.

**Phases come from GolfDB's labels, never from `segment_phases`.** That isolates the *estimator*
from our *segmentation*: a wide sway distribution then means "this estimator on tour swings", not
"our top-detection drifted". It also means these bands stay valid across future segmentation
changes.

The production measuring code is reused verbatim — this iterates `POSE_MEASUREMENTS`, the same
registry `analysis.engine` builds a swing's measurements from, so the corpus is measured by exactly
the code that will measure the user's swing. Reimplementing the metrics here would be the fastest
possible way to produce bands that quietly do not match what they are compared against.

Iterating the registry rather than naming metrics here is what makes a new candidate metric one
line in `measure.py` instead of an edit in three files. It also means a metric with **no band yet**
is measured across the corpus — which is the whole point, since the band is derived from these
numbers. Before the measure/judge split this script called the *evaluators*, which returned `None`
without a band, so a new metric could never produce the population it needed to acquire one.
"""

from __future__ import annotations

import argparse
import statistics
import sys

import common
import extract_pose

from golf_coach.analysis.measure import FPS_DEPENDENT_MEASUREMENTS, POSE_MEASUREMENTS
from golf_coach.contracts.keypoints import FrameKeypoints, Landmark
from golf_coach.contracts.reference import ReferenceSwing
from golf_coach.contracts.swing import PhaseSegment, SwingPhase
from golf_coach.storage.keypoints_io import load_keypoints

# Half-widths matching phases.py, so ground-truth-derived windows have the same shape as the ones
# the checkpoints normally receive.
_TRANSITION_HALF_FRAMES = 3
_IMPACT_HALF_FRAMES = 2


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

    # The absolute-duration metrics are excluded here and it is not a limitation of this script:
    # `_phases_from_events` puts *frame indices* in `start_ms`, so measuring one off these phases
    # yields a frame count named `_ms`. Written to `swings.jsonl` it would sit beside the real
    # milliseconds `derive_reference._attach_durations` computes from a clip's own frame rate, and
    # every duration band would be cut from the mixture. Ratios are immune, which is why every
    # other metric in the registry is one.
    wanted = {n: m for n, m in POSE_MEASUREMENTS.items() if n not in FPS_DEPENDENT_MEASUREMENTS}
    if skipped_metrics := sorted(FPS_DEPENDENT_MEASUREMENTS):
        print(
            f"Not measuring {', '.join(skipped_metrics)} — absolute durations cannot be read from "
            "ground-truth phases, which carry frame indices. derive_reference.py attaches them."
        )
    raw: dict[str, list[float]] = {name: [] for name in wanted}

    for swing in swings:
        path = cache / f"{swing.source_id}.keypoints.json"
        if not path.exists():
            continue

        keypoints = load_keypoints(path).frames
        phases = _phases_from_events(swing.events, swing.events["start"], len(keypoints))
        if phases is None:
            skipped += 1
            continue

        corrected = (
            keypoints
            if args.no_aspect_correction
            else _apply_pixel_aspect(keypoints, swing.pixel_aspect)
        )

        found = False
        for name, pose in wanted.items():
            # `.value`, not the outcome — a `MeasureOutcome` is never falsy, so dropping the
            # unwrap here would store the tuple as the metric and take every band derived from
            # this file with it.
            value = pose.measure(corrected, phases).value
            if value is None:
                continue
            swing.metrics[name] = value
            raw[name].append(value)
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
