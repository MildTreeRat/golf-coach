"""Does MediaPipe's `z` carry signal, or only its own noise? The gate for M8.1 step 1.

Usage:
    python scripts/golfdb/tune_z_channel.py [--landmarks N]

Stdlib only. Reads the Tier 1 keypoint caches for `mediapipe:lite` and `mediapipe:full`.

### Why this is a question at all

The trajectory model needs a decision it cannot make for itself: are its features `x/y`, or
`x/y/z`? Two arguments pull opposite ways and neither is settled by assertion.

**Against.** MediaPipe's `z` is not depth from two cameras. It is a per-image *guess* at depth
relative to the hips, learned from a single frame, and owning a second camera does not improve it —
[ADR-011](../../docs/decisions/011-camera-synchronization.md)'s addendum already ruled that
hand-held phones can be aligned but never fused, so no triangulated depth exists anywhere in this
project.

**For.** A noisy channel is not a worthless one. Reference and user swings are measured by the same
estimator, so much of `z`'s bias is common-mode and cancels — which is exactly the argument
[ADR-012](../../docs/decisions/012-golfdb-reference-data.md) makes for using GolfDB at all. And
`z` has simply never been screened.

So screen it the way every metric in the panel was screened, on
`tune_spatial_metric.py`'s bar: **spread ÷ noise ≥ 2.0**, per landmark, with `x` and `y` measured
the same way beside it as the control.

### How noise is measured, and why this screen is generous

Noise is the **lite-vs-full disagreement on identical pixels** — two estimators, same frames, so
any difference is the instrument rather than the swing. Both caches hold the same 461 face-on
clips, which is what makes the comparison possible at all.

`tune_spatial_metric.py` divides by `noise + boundary`, where boundary is the error in *locating*
the instant. There is no boundary term here: this reads at GolfDB's hand-labelled event frames, so
instant error is zero by construction. **That makes this screen generous to every axis** — if a
coordinate fails here, it would fail harder in production, where the instant has to be detected.

**And it is generous to `z` a second time, which matters more.** Lite and full are two sizes of the
same architecture trained the same way, so they make *correlated* errors about monocular depth —
they agree with each other while both guessing. Estimator agreement is a good proxy for error on
`x` and `y`, where the evidence is in the pixels, and a poor one on `z`, where it is inferred. So
`z`'s ratio here is an **upper bound** on its real standing, not an estimate of it. The number that
would settle it is a fit with and against `z` compared on leave-one-player-out exceedance, which is
a step-3 experiment rather than something this screen can answer.

Everything is normalised by shoulder width, the same scale-invariance ruler `measure.py` uses, so
the three coordinates are compared on one footing. MediaPipe documents `z` as sharing `x`'s scale,
which is what makes that legitimate.
"""

from __future__ import annotations

import json
import statistics
import sys
from typing import Any

import common

from golf_coach.contracts.keypoints import PoseLandmark

_MIN_RATIO = 2.0

# The 12 the trajectory model will use: torso, arms, legs and one head reference. MediaPipe's face
# detail (eyes, mouth) and hand detail (pinky, index, thumb) are dropped — they are noisy and
# nothing in the panel reads them.
_LANDMARKS: tuple[tuple[str, PoseLandmark], ...] = (
    ("left_ear", PoseLandmark.LEFT_EAR),
    ("right_ear", PoseLandmark.RIGHT_EAR),
    ("left_shoulder", PoseLandmark.LEFT_SHOULDER),
    ("right_shoulder", PoseLandmark.RIGHT_SHOULDER),
    ("left_elbow", PoseLandmark.LEFT_ELBOW),
    ("right_elbow", PoseLandmark.RIGHT_ELBOW),
    ("left_wrist", PoseLandmark.LEFT_WRIST),
    ("right_wrist", PoseLandmark.RIGHT_WRIST),
    ("left_hip", PoseLandmark.LEFT_HIP),
    ("right_hip", PoseLandmark.RIGHT_HIP),
    ("left_knee", PoseLandmark.LEFT_KNEE),
    ("right_knee", PoseLandmark.RIGHT_KNEE),
)

# The eight annotated events, excluding GolfDB's clip bounds. Same instants the trajectory model
# will resample onto.
_EVENTS = common.EVENT_NAMES[1:-1]

_MIN_VISIBILITY = 0.5
_MIN_SHOULDER_WIDTH = 0.02

_LITE = "mediapipe-lite"
_FULL = "mediapipe-full"


def _load_cache(slug: str, source_id: str) -> list[dict[str, Any]] | None:
    """Frames from one estimator's Tier 1 cache, or None if this clip was never extracted.

    Tolerant of both shapes on purpose. `data/processed/*.keypoints.json` is a `KeypointsFile`
    object (`{clip, frames}`); the reference cache writes the bare frame list, because clip
    metadata for a 160x160 crop of broadcast footage says nothing worth storing 461 times. Reading
    either means this script does not care which corpus it is pointed at.
    """
    path = common.KEYPOINTS_DIR / slug / f"{source_id}.keypoints.json"
    if not path.exists():
        return None
    payload = json.loads(path.read_text(encoding="utf-8"))
    return payload["frames"] if isinstance(payload, dict) else payload


def _shoulder_width(frame: dict[str, Any]) -> float | None:
    """This frame's shoulder width, or None when either shoulder is not confidently seen."""
    left = frame["landmarks"][PoseLandmark.LEFT_SHOULDER]
    right = frame["landmarks"][PoseLandmark.RIGHT_SHOULDER]
    if min(left["visibility"], right["visibility"]) < _MIN_VISIBILITY:
        return None
    width = abs(left["x"] - right["x"])
    return width if width >= _MIN_SHOULDER_WIDTH else None


def _hip_midpoint(frame: dict[str, Any]) -> dict[str, float] | None:
    """The body-frame origin, or None when a hip is not confidently seen.

    Everything is measured relative to this, and that is not cosmetic. A landmark's **absolute**
    position in the frame is mostly where the golfer happens to stand in a 160x160 broadcast crop,
    which is framing variance rather than swing variance — measuring its spread would credit `x`
    and `y` with signal they do not have. It would also compare unlike things: MediaPipe defines
    `z` as depth *relative to the hips* already, so leaving `x`/`y` in image coordinates would put
    the three axes in two different frames of reference and rig the comparison before it ran.
    """
    left = frame["landmarks"][PoseLandmark.LEFT_HIP]
    right = frame["landmarks"][PoseLandmark.RIGHT_HIP]
    if min(left["visibility"], right["visibility"]) < _MIN_VISIBILITY:
        return None
    return {axis: (left[axis] + right[axis]) / 2 for axis in ("x", "y", "z")}


def collect(
    swings: list[Any],
) -> tuple[dict[tuple[str, str], list[float]], dict[tuple[str, str], list[float]], int]:
    """Per (landmark, axis): hip-relative observed values, and lite-vs-full disagreements.

    Values are pooled across the eight labelled instants — the screen asks whether a coordinate
    varies more across tour swings than the instrument wobbles, which is a question about the
    channel rather than about any one instant.
    """
    values: dict[tuple[str, str], list[float]] = {}
    noise: dict[tuple[str, str], list[float]] = {}
    used = 0

    for swing in swings:
        lite = _load_cache(_LITE, swing.source_id)
        full = _load_cache(_FULL, swing.source_id)
        if lite is None or full is None:
            continue
        used += 1

        # GolfDB's event indices are absolute frames in the *source video*; the cached clips are
        # trimmed, so they have to be rebased on `start` exactly as upstream's own dataloader does
        # (`events -= events[0]`). Skipping this is the classic mis-read of this dataset, and it
        # fails quietly: the indices simply run off the end of a short clip and most of the corpus
        # silently drops out instead of erroring.
        start = swing.events.get("start")
        if start is None:
            continue

        for event in _EVENTS:
            absolute = swing.events.get(event)
            if absolute is None:
                continue
            index = absolute - start
            if index < 0 or index >= len(lite) or index >= len(full):
                continue
            lite_frame, full_frame = lite[index], full[index]

            width = _shoulder_width(lite_frame)
            origin = _hip_midpoint(lite_frame)
            if width is None or origin is None:
                continue

            for name, landmark in _LANDMARKS:
                a = lite_frame["landmarks"][landmark]
                b = full_frame["landmarks"][landmark]
                if min(a["visibility"], b["visibility"]) < _MIN_VISIBILITY:
                    continue
                for axis in ("x", "y", "z"):
                    values.setdefault((name, axis), []).append((a[axis] - origin[axis]) / width)
                    # Noise is a difference of two readings of the same pixels, so the origin
                    # cancels; only the scale normalisation is needed.
                    noise.setdefault((name, axis), []).append(abs(a[axis] - b[axis]) / width)

    return values, noise, used


def report(
    values: dict[tuple[str, str], list[float]], noise: dict[tuple[str, str], list[float]]
) -> dict[str, list[float]]:
    header = (
        f"\n{'landmark':<16s}{'axis':<6s}{'n':>7s}"
        f"{'spread':>10s}{'noise':>10s}{'ratio':>9s}  verdict"
    )
    print(header)
    print("-" * 78)

    by_axis: dict[str, list[float]] = {"x": [], "y": [], "z": []}
    for name, _ in _LANDMARKS:
        for axis in ("x", "y", "z"):
            observed = values.get((name, axis), [])
            wobble = noise.get((name, axis), [])
            if len(observed) < 30:
                continue
            # Spread as the standard deviation of the coordinate across the corpus, against the
            # median absolute disagreement between two estimators on the same pixels.
            spread = statistics.stdev(observed)
            floor = statistics.median(wobble)
            ratio = spread / floor if floor > 0 else float("inf")
            by_axis[axis].append(ratio)
            verdict = "signal" if ratio >= _MIN_RATIO else "MOSTLY NOISE"
            print(
                f"{name:<16s}{axis:<6s}{len(observed):>7d}{spread:>10.4f}"
                f"{floor:>10.4f}{ratio:>9.2f}  {verdict}"
            )
        print()
    return by_axis


def main(argv: list[str]) -> int:
    if argv and argv[0] in {"-h", "--help"}:
        print("usage: python scripts/golfdb/tune_z_channel.py", file=sys.stderr)
        return 2

    swings = [s for s in common.load_swings() if s.view == common.VIEW_FACE_ON]
    values, noise, used = collect(swings)
    if not used:
        print(
            "no clip has both mediapipe-lite and mediapipe-full keypoints cached — "
            "run extract_pose.py for both",
            file=sys.stderr,
        )
        return 1

    print(f"face-on clips with both estimators cached: {used}")
    print(f"instants per clip: {len(_EVENTS)}   landmarks: {len(_LANDMARKS)}")
    by_axis = report(values, noise)

    print("=" * 78)
    print("SUMMARY — median ratio across the 12 landmarks")
    print("=" * 78)
    for axis in ("x", "y", "z"):
        ratios = by_axis[axis]
        if not ratios:
            continue
        median = statistics.median(ratios)
        passing = sum(1 for r in ratios if r >= _MIN_RATIO)
        print(
            f"  {axis}: median ratio {median:6.2f}   "
            f"{passing}/{len(ratios)} landmarks clear {_MIN_RATIO:g}"
        )

    z_ratios = by_axis["z"]
    planar = by_axis["x"] + by_axis["y"]
    print(f"\n{'=' * 78}\nGATE\n{'=' * 78}")
    if not z_ratios:
        print("  z was never measurable — treat as a fail.")
        return 0

    z_median = statistics.median(z_ratios)
    planar_median = statistics.median(planar)
    z_passing = sum(1 for r in z_ratios if r >= _MIN_RATIO)

    if z_median >= _MIN_RATIO:
        print(f"  PASS — z's median ratio {z_median:.2f} clears {_MIN_RATIO:g}")
        print(f"  ({z_passing}/{len(z_ratios)} landmarks individually clear it.)")
        print(f"  Relative to x/y at {planar_median:.2f}, z carries "
              f"{z_median / planar_median:.0%} of the planar signal-to-noise.")
        print("\n  Carry z forward, but NOT on this number alone. Two estimators from the same")
        print("  family make *correlated* mistakes about monocular depth — they agree with each")
        print("  other while both guessing — so lite-vs-full flatters z more than it flatters x")
        print("  or y, and this ratio is an upper bound on z's real standing rather than an")
        print("  estimate of it. Settle it where it can be settled: fit the trajectory model")
        print("  with and without z and compare leave-one-player-out exceedance.")
    else:
        print(f"  FAIL — z's median ratio {z_median:.2f} is under {_MIN_RATIO:g}")
        print(f"  ({z_passing}/{len(z_ratios)} landmarks individually clear it.)")
        print(f"  x/y sit at {planar_median:.2f}, so z carries "
              f"{z_median / planar_median:.0%} of the planar signal-to-noise.")
        print("  Build the trajectory model on x/y only, and record this in M4_POSE_BAKEOFF.")
    print("\n  Note this screen omits the boundary term (it reads at labelled instants), so it")
    print("  is generous to every axis. A fail here would fail harder in production.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
