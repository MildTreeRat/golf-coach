"""Which landmarks should a model track, from this camera? The gate for M8.2's feature list.

Usage:
    python scripts/golfdb/tune_landmarks.py [--view face-on|down-the-line] [--keep 12]

Stdlib only. Reads the Tier 1 caches for `mediapipe:lite` and `mediapipe:full`.

### Why this is a question rather than a copy

The face-on trajectory model tracks twelve landmarks, chosen by anatomy: torso, arms, legs, one
head reference. It is tempting to hand the same twelve to a down-the-line model, and it is exactly
the assumption that already cost this project once — `segment_phases()` was pointed at the lead
wrist in both views, and from behind that landmark is tracked in **39%** of frames
(M4_POSE_BAKEOFF §Phase F). A landmark list is a property of the *camera*, not of the skeleton.

### The screen

Per landmark, the same two-part bar every metric in the panel had to clear, generalised from
`tune_z_channel.py`'s per-axis version:

- **visibility** — the fraction of labelled instants where the landmark is confidently tracked. A
  landmark the camera cannot see is not a feature, however much it would vary if it could.
- **ratio = motion / noise** — how far the landmark travels *during a swing*, against the median
  lite-vs-full disagreement on identical pixels. Under `_MIN_RATIO` the landmark mostly reports its
  own estimation error.

**"Motion" is the within-swing range, and the first version of this script got that wrong.** It
scored cross-corpus spread of the landmark's position — variance *between* golfers — and ranked
ankles and heels top while dropping the hips. Both were artifacts. Ankles barely move in a golf
swing; their positional spread is leg length and stance width, which is anatomy. And the hips came
last because everything is measured relative to the hip midpoint, so they sit near zero by
construction. Ranking on between-golfer variance would have built a basis that spends its
components describing body size — the same class of mistake as the pixel-aspect bug in §Phase E,
where variance that was not about the swing got into the model. The question a trajectory model
asks of a landmark is *does this move, informatively, while the club is swung*.

A landmark is scored on its **worse** axis, not its average: a feature that is solid in `x` and
noise in `y` still contributes a noisy column to every component that reads it.

### What this screen can and cannot decide

**It is an exclusion floor, not a relevance ranking, and the ordering should not be read as one.**
Both views put ankles and heels at the top, which is not a claim that feet matter most in a golf
swing — they barely move. Their ratio is high because their *noise* is nearly zero: a stationary,
well-tracked landmark beats a fast, occluded one on any signal-to-noise measure, and the wrists are
the fastest and least certain points on the body.

That is the honest limit. This answers *is this landmark measurable from this camera* — the
question `tune_spatial_metric.py` was built for. It cannot answer *does this landmark help*, which
depends on what the model is for. Use the failures: a landmark below the floor will contribute
noise to every component that reads it. Decide the rest **empirically**, by fitting candidate sets
and comparing leave-one-player-out exceedance — the method that settled `z` in §Phase E, where a
channel passed its screen and then lost the fit.

Both caveats from `tune_z_channel.py` carry over. This reads at hand-labelled instants, so there
is no boundary term and the screen is generous; and lite-vs-full understates error wherever the
two estimators fail the same way, which is most likely exactly where the body is occluded.

Prints a recommendation and changes nothing. The list moves into a model by hand.
"""

from __future__ import annotations

import statistics
import sys

import common
import tune_z_channel as zc

from golf_coach.contracts.keypoints import NUM_POSE_LANDMARKS, PoseLandmark

_MIN_RATIO = 2.0
#: Below this a landmark is absent often enough that interpolation would be inventing the swing
#: rather than bridging a blink — the same instinct as `analysis/trajectory.py::MAX_MISSING`.
_MIN_VISIBILITY_FRACTION = 0.60

_AXES = ("x", "y")


def collect(view: str) -> tuple[dict[int, dict[str, list[float]]], int]:
    """Per landmark: seen/total counts, and per-axis values and lite-vs-full disagreements."""
    swings = [s for s in common.load_swings() if s.view == view]
    stats: dict[int, dict[str, list[float]]] = {
        i: {"seen": [], **{f"v{a}": [] for a in _AXES}, **{f"n{a}": [] for a in _AXES}}
        for i in range(NUM_POSE_LANDMARKS)
    }
    used = 0

    for swing in swings:
        lite = zc._load_cache(zc._LITE, swing.source_id)
        full = zc._load_cache(zc._FULL, swing.source_id)
        if lite is None:
            continue
        used += 1

        start = swing.events.get("start")
        if start is None:
            continue
        per_clip: dict[int, dict[str, list[float]]] = {
            i: {a: [] for a in _AXES} for i in range(NUM_POSE_LANDMARKS)
        }

        for event in zc._EVENTS:
            absolute = swing.events.get(event)
            if absolute is None:
                continue
            index = absolute - start
            if index < 0 or index >= len(lite):
                continue
            frame = lite[index]

            width = zc._shoulder_width(frame)
            origin = zc._hip_midpoint(frame)
            if width is None or origin is None:
                continue

            other = full[index] if full is not None and index < len(full) else None
            for i in range(NUM_POSE_LANDMARKS):
                a = frame["landmarks"][i]
                tracked = a["visibility"] >= zc._MIN_VISIBILITY
                stats[i]["seen"].append(1.0 if tracked else 0.0)
                if not tracked:
                    continue
                for axis in _AXES:
                    per_clip[i][axis].append((a[axis] - origin[axis]) / width)
                if other is not None:
                    b = other["landmarks"][i]
                    if b["visibility"] >= zc._MIN_VISIBILITY:
                        for axis in _AXES:
                            stats[i][f"n{axis}"].append(abs(a[axis] - b[axis]) / width)

        # One number per landmark per clip: how far it travelled across the eight instants. The
        # median of these over the corpus is the "motion" the ratio is built on.
        for i in range(NUM_POSE_LANDMARKS):
            for axis in _AXES:
                seen = per_clip[i][axis]
                if len(seen) >= 4:
                    stats[i][f"v{axis}"].append(max(seen) - min(seen))

    return stats, used


def score(stats: dict[int, dict[str, list[float]]]) -> list[tuple[float, float, float, str, int]]:
    """`(worst_ratio, visibility, n, name, index)` per landmark, best first.

    Scored on the **worse** axis: a landmark solid in `x` and noise in `y` still contributes a
    noisy column to every component that reads it.
    """
    rows = []
    for i in range(NUM_POSE_LANDMARKS):
        seen = stats[i]["seen"]
        if not seen:
            continue
        visibility = statistics.fmean(seen)
        ratios = []
        for axis in _AXES:
            values, noise = stats[i][f"v{axis}"], stats[i][f"n{axis}"]
            if len(values) < 30 or len(noise) < 30:
                continue
            floor = statistics.median(noise)
            motion = statistics.median(values)
            ratios.append(motion / floor if floor > 0 else float("inf"))
        worst = min(ratios) if ratios else float("nan")
        rows.append((worst, visibility, float(len(seen)), PoseLandmark(i).name.lower(), i))
    rows.sort(key=lambda r: (-r[0] if r[0] == r[0] else 0, -r[1]))
    return rows


def main(argv: list[str]) -> int:
    if argv and argv[0] in {"-h", "--help"}:
        print(
            "usage: python scripts/golfdb/tune_landmarks.py "
            "[--view face-on|down-the-line] [--keep 12]",
            file=sys.stderr,
        )
        return 2

    view = argv[argv.index("--view") + 1] if "--view" in argv else common.VIEW_FACE_ON
    keep = int(argv[argv.index("--keep") + 1]) if "--keep" in argv else 12

    stats, used = collect(view)
    if not used:
        print(f"no cached clips for view={view} — run extract_pose.py", file=sys.stderr)
        return 1

    rows = score(stats)
    have_noise = any(r[0] == r[0] for r in rows)
    print(f"view={view}   clips={used}   instants/clip={len(zc._EVENTS)}")
    if not have_noise:
        print("  (no mediapipe-full cache for this view — ratios unavailable, ranking on "
              "visibility alone)")

    print(f"\n{'landmark':<20s}{'vis':>8s}{'ratio':>9s}  verdict")
    print("-" * 60)
    for worst, visibility, _n, name, _i in rows:
        ratio = f"{worst:9.2f}" if worst == worst else f"{'—':>9s}"
        ok = visibility >= _MIN_VISIBILITY_FRACTION and (worst != worst or worst >= _MIN_RATIO)
        print(f"{name:<20s}{visibility:8.2f}{ratio}  {'keep' if ok else 'DROP'}")

    eligible = [
        (w, v, n)
        for w, v, _c, n, _i in rows
        if v >= _MIN_VISIBILITY_FRACTION and (w != w or w >= _MIN_RATIO)
    ]
    print(f"\n{'=' * 60}\nRECOMMENDED LIST ({keep} of {len(eligible)} eligible)\n{'=' * 60}")
    for w, v, n in eligible[:keep]:
        print(f"  {n:<20s} vis {v:.2f}   ratio {w:.2f}" if w == w else f"  {n:<20s} vis {v:.2f}")

    dropped = [n for w, v, _c, n, _i in rows if not (
        v >= _MIN_VISIBILITY_FRACTION and (w != w or w >= _MIN_RATIO)
    )]
    print(f"\n  failed the screen ({len(dropped)}): {', '.join(dropped)}")
    print("\n  Nothing is written. A landmark list moves into a model by hand.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
