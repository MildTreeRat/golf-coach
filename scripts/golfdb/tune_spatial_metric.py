"""Is a candidate spatial metric signal, or is it landmark jitter? [M6.5]

Usage:
    python scripts/golfdb/tune_spatial_metric.py
    python scripts/golfdb/tune_spatial_metric.py --metric hip_sway_norm

The repo has harnesses for *temporal* rules — `tune_address.py`, `tune_arm_parallel.py`,
`bakeoff.py` all score how well something locates an instant in time. It has had none for the
other kind of metric, which is why "should we add this checkpoint?" has been answerable only by
argument. This is that harness.

## What it measures, and why these three columns

`tune_arm_parallel.py` refused two candidate checkpoints by showing that a rule reading no pose
signal at all beat every pose rule. Its logic was *error versus tour spread*: a metric is worthless
when the error in measuring it is comparable to the variation you are trying to detect. The same
logic applies here, but the error term is different — nothing here detects an instant, so the
question becomes how much of a metric's clip-to-clip variation is real.

- **noise** — median |lite - full| per clip, both measured at GolfDB's labelled instants. Two
  estimators looking at the identical frames should agree; whatever they disagree by is a floor on
  how well the quantity can be known. Both caches already exist for all 461 face-on clips, so this
  costs no new pose extraction.
- **boundary** — median |labelled - detected| per clip, both under `mediapipe:lite`. This is the
  cost of *our* segmentation rather than the estimator's, and it is the term that catches a metric
  that is sound in principle but reads a window hung off the weak address boundary (median 7
  frames of error, 40% of clips over 10).
- **spread** — p10..p90 of the metric across the corpus, at labelled instants. The variation a band
  would be cut from.

**The verdict is `spread / (noise + boundary)`.** Below ~2 the metric mostly measures the pipeline
rather than the golfer, and a band cut from it would be a band on our own error. Well above it, the
population genuinely varies more than we mis-measure. This is a screening tool, not a hypothesis
test — it says which candidates deserve a band, not that any of them is correct.

Prints a per-metric table. Nothing is written; deciding what to promote is a human call, and
`ranges.json` provenance is hand-written by ADR-010.
"""

from __future__ import annotations

import argparse
import statistics
import sys

import common
import derive_pose_metrics as derive
import extract_pose

from golf_coach.analysis.measure import POSE_MEASUREMENTS
from golf_coach.analysis.phases import segment_phases
from golf_coach.analysis.smoothing import smooth_keypoints
from golf_coach.storage.keypoints_io import load_keypoints

#: The two estimators whose disagreement forms the noise floor. Both are cached for every face-on
#: clip; `mediapipe:lite` is what production runs (ADR-002 addendum).
_PRIMARY = "mediapipe:lite"
_FOIL = "mediapipe:full"

#: Below this, a metric's population spread is not clearly larger than our own measurement error.
_MIN_RATIO = 2.0


def _quantile(values: list[float], q: float) -> float:
    ordered = sorted(values)
    if not ordered:
        return float("nan")
    return ordered[min(len(ordered) - 1, max(0, int(q * (len(ordered) - 1))))]


def _measure_all(frames, phases) -> dict[str, float]:
    """Every registered metric on one clip, skipping the ones that could not be measured."""
    out: dict[str, float] = {}
    for name, measure in POSE_MEASUREMENTS.items():
        value = measure(frames, phases)
        if value is not None:
            out[name] = value
    return out


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(
        description="Score candidate spatial metrics: population spread against measurement noise."
    )
    parser.add_argument("--metric", default=None, help="only this metric (default: all registered)")
    parser.add_argument("--limit", type=int, default=0, help="stop after N clips (a quick look)")
    args = parser.parse_args(argv)

    primary_dir = common.KEYPOINTS_DIR / extract_pose.estimator_slug(_PRIMARY)
    foil_dir = common.KEYPOINTS_DIR / extract_pose.estimator_slug(_FOIL)
    if not primary_dir.is_dir():
        print(f"error: no cache at {primary_dir} — run extract_pose.py first", file=sys.stderr)
        return 1

    wanted = [args.metric] if args.metric else list(POSE_MEASUREMENTS)
    unknown = [m for m in wanted if m not in POSE_MEASUREMENTS]
    if unknown:
        print(f"error: unknown metric(s): {', '.join(unknown)}", file=sys.stderr)
        return 2

    # Per metric: the labelled-instant population, and the paired per-clip deltas.
    population: dict[str, list[float]] = {m: [] for m in wanted}
    noise: dict[str, list[float]] = {m: [] for m in wanted}
    boundary: dict[str, list[float]] = {m: [] for m in wanted}

    swings = [s for s in common.load_swings() if s.view == common.VIEW_FACE_ON]
    considered = len(swings)

    clips = 0
    for swing in swings:
        if args.limit and clips >= args.limit:
            break

        primary_path = primary_dir / f"{swing.source_id}.keypoints.json"
        if not primary_path.exists():
            continue

        primary_frames = load_keypoints(primary_path).frames
        labelled = derive._phases_from_events(
            swing.events, swing.events["start"], len(primary_frames)
        )
        if labelled is None:
            continue

        primary_frames = derive._apply_pixel_aspect(primary_frames, swing.pixel_aspect)
        at_labelled = _measure_all(primary_frames, labelled)
        for name, value in at_labelled.items():
            if name in population:
                population[name].append(value)

        # Column 1: the same clip, same instants, a different estimator.
        foil_path = foil_dir / f"{swing.source_id}.keypoints.json"
        if foil_path.exists():
            foil_frames = derive._apply_pixel_aspect(
                load_keypoints(foil_path).frames, swing.pixel_aspect
            )
            foil_labelled = derive._phases_from_events(
                swing.events, swing.events["start"], len(foil_frames)
            )
            if foil_labelled is not None:
                at_foil = _measure_all(foil_frames, foil_labelled)
                for name in wanted:
                    if name in at_labelled and name in at_foil:
                        noise[name].append(abs(at_labelled[name] - at_foil[name]))

        # Column 2: the same clip, same estimator, our own segmentation instead of the labels.
        detected = segment_phases(smooth_keypoints(primary_frames))
        at_detected = _measure_all(primary_frames, detected)
        for name in wanted:
            if name in at_labelled and name in at_detected:
                boundary[name].append(abs(at_labelled[name] - at_detected[name]))

        clips += 1

    if not clips:
        print("error: no usable face-on clips found", file=sys.stderr)
        return 1

    print(f"\n{clips} face-on clips ({considered} in corpus), {_PRIMARY} vs {_FOIL}\n")
    header = (
        f"{'metric':<30}{'n':>5}{'p10':>9}{'p90':>9}{'spread':>9}"
        f"{'noise':>9}{'bound':>9}{'ratio':>8}  verdict"
    )
    print(header)
    print("-" * len(header))

    for name in wanted:
        values = population[name]
        if len(values) < 30:
            print(f"{name:<30}{len(values):>5}   (under 30 clips — nothing to say)")
            continue
        p10, p90 = _quantile(values, 0.10), _quantile(values, 0.90)
        spread = p90 - p10
        n_med = statistics.median(noise[name]) if noise[name] else float("nan")
        b_med = statistics.median(boundary[name]) if boundary[name] else float("nan")
        error = (0.0 if n_med != n_med else n_med) + (0.0 if b_med != b_med else b_med)
        ratio = spread / error if error > 0 else float("inf")
        verdict = "signal" if ratio >= _MIN_RATIO else "MOSTLY NOISE"

        # A signed metric measured camera-relative over a mixed-handedness corpus splits into two
        # opposite populations, and a band cut across both would sit in the empty middle. Flag it
        # here rather than letting it be discovered after a band shipped.
        negative = sum(1 for v in values if v < 0) / len(values)
        if 0.2 < negative < 0.8:
            verdict += f"  ** BIMODAL: {negative:.0%} negative — handedness unresolved"

        print(
            f"{name:<30}{len(values):>5}{p10:>9.3f}{p90:>9.3f}{spread:>9.3f}"
            f"{n_med:>9.3f}{b_med:>9.3f}{ratio:>8.1f}  {verdict}"
        )

    print(
        f"\nratio = spread / (noise + boundary); under {_MIN_RATIO:g} the metric mostly measures "
        "this pipeline rather than the golfer.\nA 'signal' verdict says a band is worth deriving, "
        "not that the metric is the right one to coach on."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
