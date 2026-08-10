"""Compare candidate top/impact detection rules against GolfDB ground truth. [M4-REF]

Usage:
    python scripts/golfdb/tune_phases.py
    python scripts/golfdb/tune_phases.py --estimator mediapipe:lite --limit 200

Reads **cached** keypoints (`extract_pose.py` must have run) so an algorithm change is a
one-second experiment rather than a re-run of the estimator. Scores every candidate rule on the
same clips and prints median / mean absolute error plus a failure rate, for `top` and `impact`.

This exists because `phases.py`'s original rule — "top = global minimum of lead-wrist `y`" — was
tuned against a single clip and is wrong on tour swings, where the hands finish *higher* than they
were at the top (see docs/M4_POSE_BAKEOFF.md). Throwaway tuning scaffolding: the winning rule moves
into `phases.py`, this stays as the record of what was tried.
"""

from __future__ import annotations

import argparse
import statistics
import sys
from collections.abc import Callable

import common
import extract_pose

from golf_coach.analysis.phases import _lead_wrist_xy, _wrist_speed
from golf_coach.analysis.smoothing import smooth_keypoints
from golf_coach.contracts.keypoints import FrameKeypoints, PoseLandmark
from golf_coach.contracts.reference import ReferenceSwing

_MIN_VISIBILITY = 0.5

# A rising run survives a dip of this fraction of the rise it has already accumulated. Real
# downswings are not perfectly monotone in a noisy 2D track; a hard monotonicity test shatters
# them into fragments, and zero tolerance would score the noise rather than the swing.
_DRAWDOWN_TOLERANCE = 0.25

Rule = Callable[[list[float], list[float], list[bool]], tuple[int, int]]


# --- candidate rules: (ys, speeds, confident) -> (top, impact) ------------------------------


def rule_argmin_all(ys: list[float], speeds: list[float], ok: list[bool]) -> tuple[int, int]:
    """What `phases.py` does today. Included as the baseline to beat."""
    top = min(range(len(ys)), key=ys.__getitem__)
    impact = max(range(top, len(ys)), key=ys.__getitem__)
    return top, impact


def rule_before_peak_speed(ys: list[float], speeds: list[float], ok: list[bool]) -> tuple[int, int]:
    """Restrict the minimum search to before the fastest wrist motion (the downswing)."""
    peak = max(range(len(speeds)), key=speeds.__getitem__)
    top = min(range(max(peak, 1)), key=ys.__getitem__)
    impact = max(range(top, len(ys)), key=ys.__getitem__)
    return top, impact


def rule_max_rise(ys: list[float], speeds: list[float], ok: list[bool]) -> tuple[int, int]:
    """The (i < j) pair maximizing `y[j] - y[i]`: the deepest descent-to-impact in the clip.

    Models the swing's actual geometry — the largest sustained *fall* of the hands is the
    downswing — and yields top and impact together, with no address estimate and no thresholds.
    """
    best = (-1.0, 0, 0)
    low = 0
    for j in range(1, len(ys)):
        if ys[j] - ys[low] > best[0]:
            best = (ys[j] - ys[low], low, j)
        if ys[j] < ys[low]:
            low = j
    return best[1], best[2]


def rule_max_rise_confident(
    ys: list[float], speeds: list[float], ok: list[bool]
) -> tuple[int, int]:
    """`rule_max_rise` over confidently-tracked frames only."""
    best = (-1.0, 0, 0)
    low = -1
    for j in range(len(ys)):
        if not ok[j]:
            continue
        if low >= 0 and ys[j] - ys[low] > best[0]:
            best = (ys[j] - ys[low], low, j)
        if low < 0 or ys[j] < ys[low]:
            low = j
    return (best[1], best[2]) if best[0] > 0 else rule_max_rise(ys, speeds, ok)


def _rising_runs(
    ys: list[float], ok: list[bool], floor_fraction: float = 0.0
) -> list[tuple[float, int, int]]:
    """Maximal near-monotone rising runs as `(rise, start, end)`, tolerating small dips.

    `floor_fraction` is the B7 candidate: an absolute floor on the drawdown, as a fraction of the
    clip's own wrist range, so noise cannot end a run in its first frames where the *relative*
    tolerance is still near zero. `0.0` reproduces the pre-B7 behaviour exactly, which is what keeps
    every rule above a fixed baseline.
    """
    runs: list[tuple[float, int, int]] = []
    start = peak_at = -1
    peak = 0.0
    tracked = [y for y, good in zip(ys, ok, strict=False) if good]
    floor = floor_fraction * (max(tracked) - min(tracked)) if tracked else 0.0
    for i, y in enumerate(ys):
        if not ok[i]:
            continue
        if start < 0:
            start, peak, peak_at = i, y, i
            continue
        if y >= peak:
            peak, peak_at = y, i
            continue
        rise = peak - ys[start]
        if rise > 0 and (peak - y) > max(_DRAWDOWN_TOLERANCE * rise, floor):
            runs.append((rise, start, peak_at))
            start, peak, peak_at = i, y, i
        elif y < ys[start]:
            start, peak, peak_at = i, y, i
    if start >= 0 and peak > ys[start]:
        runs.append((peak - ys[start], start, peak_at))
    return runs


def rule_best_rising_run(ys: list[float], speeds: list[float], ok: list[bool]) -> tuple[int, int]:
    """The largest near-monotone rising run. Rejects wobbly late-clip tracking blowouts."""
    runs = _rising_runs(ys, ok)
    if not runs:
        return rule_max_rise(ys, speeds, ok)
    _, start, end = max(runs, key=lambda r: r[0])
    return start, end


def rule_fastest_rising_run(
    ys: list[float], speeds: list[float], ok: list[bool]
) -> tuple[int, int]:
    """The rising run with the highest *rate* — the quickest descent of the hands."""
    runs = _rising_runs(ys, ok)
    if not runs:
        return rule_max_rise(ys, speeds, ok)
    _, start, end = max(runs, key=lambda r: r[0] / max(r[2] - r[1], 1))
    return start, end


def _earliest_major_rise(
    ys: list[float], ok: list[bool], fraction: float, floor: float = 0.0
) -> tuple[int, int]:
    """The *earliest* rising run within `fraction` of the largest.

    Encodes the one piece of structure every golf swing has and no threshold can fake: the
    downswing happens **before** the follow-through. When a late tracking blowout produces a rise
    that rivals the real downswing, "biggest" picks the artefact and "earliest among the big ones"
    does not.
    """
    runs = _rising_runs(ys, ok, floor)
    if not runs:
        return rule_max_rise(ys, [], ok)
    largest = max(r[0] for r in runs)
    major = [r for r in runs if r[0] >= fraction * largest]
    _, start, end = min(major, key=lambda r: r[1])
    return start, end


def _major_rise_rule(fraction: float, floor: float = 0.0) -> Rule:
    def rule(ys: list[float], speeds: list[float], ok: list[bool]) -> tuple[int, int]:
        return _earliest_major_rise(ys, ok, fraction, floor)

    return rule


# --- Phase B7: an absolute floor under the drawdown test --------------------------------------
#
# `_DRAWDOWN_TOLERANCE` is a fraction of the run's own accumulated rise, which is near zero in the
# first frames of a run — so noise ends the run there. On real bay footage a golfer hovering at the
# top produced a 0.002 wobble that split the descent and put the top 10 frames late. These rows
# sweep the floor; `earliest_major_rise_80` (floor 0) is the baseline they must beat or match.
#
# The pooled columns below are too coarse to choose between these — median and impact do not move at
# all, and the mean shifts by hundredths. The decision was made on a *paired* per-clip comparison
# against floor 0 (see the table in `phases._DRAWDOWN_FLOOR`), which is what separates 12-better /
# 12-worse at 0.012 from 14-better / 18-worse at 0.020. Re-derive it that way, not from this table.
_FLOOR_SWEEP = (0.008, 0.010, 0.012, 0.014, 0.020)

RULES: dict[str, Rule] = {
    "argmin_all (current)": rule_argmin_all,
    "before_peak_speed": rule_before_peak_speed,
    "max_rise": rule_max_rise,
    "max_rise_confident": rule_max_rise_confident,
    "best_rising_run": rule_best_rising_run,
    "fastest_rising_run": rule_fastest_rising_run,
    **{
        f"earliest_major_rise_{int(f * 100):02d}": _major_rise_rule(f)
        for f in (0.40, 0.50, 0.60, 0.70, 0.75, 0.80, 0.85, 0.90, 0.95)
    },
    **{
        f"major_rise_80 +floor {k:.3f}": _major_rise_rule(0.80, k) for k in _FLOOR_SWEEP
    },
}


def _series(keypoints: list[FrameKeypoints]) -> tuple[list[float], list[float], list[bool]]:
    smoothed = smooth_keypoints(keypoints)
    xy = _lead_wrist_xy(smoothed)
    ys = [y for _, y in xy]
    speeds = _wrist_speed(xy)
    ok = [
        frame.landmark(PoseLandmark.LEFT_WRIST).visibility >= _MIN_VISIBILITY
        for frame in smoothed
    ]
    return ys, speeds, ok


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description="Tune top/impact detection against ground truth.")
    parser.add_argument("--estimator", default="mediapipe:lite")
    parser.add_argument("--limit", type=int, default=0)
    parser.add_argument(
        "--detail",
        default="",
        help="rule name: list its worst clips with mean wrist visibility, to separate a bad "
        "algorithm from a bad pose track",
    )
    args = parser.parse_args(argv)

    cache = common.KEYPOINTS_DIR / extract_pose.estimator_slug(args.estimator)
    if not cache.is_dir():
        print(f"error: no cache at {cache} — run extract_pose.py first", file=sys.stderr)
        return 1

    from pydantic import TypeAdapter

    adapter = TypeAdapter(list[FrameKeypoints])
    with common.SWINGS_JSONL.open(encoding="utf-8") as handle:
        swings = {
            s.source_id: s
            for line in handle
            if line.strip()
            for s in [ReferenceSwing.model_validate_json(line)]
        }

    cached = sorted(cache.glob("*.keypoints.json"), key=lambda p: int(p.name.split(".")[0]))
    if args.limit:
        cached = cached[: args.limit]

    errors: dict[str, dict[str, list[int]]] = {
        name: {"top": [], "impact": []} for name in RULES
    }
    detail: list[tuple[int, str, float, int, int]] = []
    truncated: list[str] = []
    scored = 0
    for path in cached:
        swing = swings.get(path.name.split(".")[0])
        if swing is None:
            continue
        origin = swing.events["start"]
        truth = {"top": swing.events["top"] - origin, "impact": swing.events["impact"] - origin}

        keypoints = adapter.validate_json(path.read_bytes())
        if len(keypoints) < 6:
            continue
        # A handful of videos_160 clips are shorter than their own annotation span — the labelled
        # impact frame does not exist in the file. Scoring those measures the mismatch, not the
        # rule, so they are excluded here and skipped by the corpus pipeline too.
        if truth["impact"] >= len(keypoints):
            truncated.append(path.name.split(".")[0])
            continue
        ys, speeds, ok = _series(keypoints)

        for name, rule in RULES.items():
            top, impact = rule(ys, speeds, ok)
            errors[name]["top"].append(abs(top - truth["top"]))
            errors[name]["impact"].append(abs(impact - truth["impact"]))
            if name == args.detail:
                confidence = sum(ok) / len(ok)
                detail.append((abs(top - truth["top"]), swing.source_id, confidence, top,
                               truth["top"]))
        scored += 1

    print(f"Scored {scored} cached clips ({args.estimator})")
    if truncated:
        print(f"Excluded {len(truncated)} clip(s) shorter than their annotation: {truncated[:8]}")
    print()
    header = f"{'rule':<24} {'top med':>8} {'top mean':>9} {'top>10':>7} " \
             f"{'imp med':>8} {'imp mean':>9} {'imp>10':>7}"
    print(header)
    print("-" * len(header))
    for name in RULES:
        top, impact = errors[name]["top"], errors[name]["impact"]
        if not top:
            continue
        print(
            f"{name:<24} {statistics.median(top):>8.1f} {statistics.fmean(top):>9.1f} "
            f"{sum(1 for e in top if e > 10) * 100 // len(top):>6}% "
            f"{statistics.median(impact):>8.1f} {statistics.fmean(impact):>9.1f} "
            f"{sum(1 for e in impact if e > 10) * 100 // len(impact):>6}%"
        )

    if detail:
        detail.sort(reverse=True)
        print(f"\nWorst clips for {args.detail!r} (conf = fraction of tracked frames):")
        print(f"  {'clip':>6} {'err':>5} {'conf':>6} {'found':>6} {'truth':>6}")
        for err, clip_id, confidence, found, truth_top in detail[:12]:
            print(f"  {clip_id:>6} {err:>5} {confidence:>6.2f} {found:>6} {truth_top:>6}")
        bad = [d for d in detail if d[0] > 10]
        if bad:
            print(
                f"  {len(bad)} clip(s) over 10 frames; mean confidence "
                f"{statistics.fmean(d[2] for d in bad):.2f} vs "
                f"{statistics.fmean(d[2] for d in detail if d[0] <= 10):.2f} for the rest"
            )
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
