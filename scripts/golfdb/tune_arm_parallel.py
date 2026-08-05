"""Can we detect the arm-parallel instants well enough to score them? [M5 gate]

Usage:
    python scripts/golfdb/tune_arm_parallel.py
    python scripts/golfdb/tune_arm_parallel.py --estimator mediapipe:lite --limit 200
    python scripts/golfdb/tune_arm_parallel.py --detail cross

`golfdb_v1.json` carries tour distributions for `mid_backswing_frac` and `mid_downswing_frac` that
nothing computes for a user's swing, because `phases.py` locates only 3 of GolfDB's 8 annotated
events. GolfDB defines **mid-backswing** and **mid-downswing** as *the lead arm parallel to the
ground*, which — unlike `toe_up` (a club-shaft event, gated on M2) — is a body pose a face-on camera
can see: the lead wrist passes through the height of the lead shoulder.

This script decides whether that is worth shipping, **before** either checkpoint is written. It is
the same shape as `tune_address.py`: read the Tier 1 keypoint cache, run candidate rules, score them
against the hand-annotated frames, split by capture speed.

### The bar is not "small error in frames"

Two numbers make this gate stricter than it looks.

1. **The tour spread is tiny.** `mid_backswing_frac` runs 0.615-0.773 from p10 to p90 — about 0.16
   of a backswing, or **~4 frames** on a real-time clip. An error that would be excellent for
   address detection would consume this entire distribution and leave a checkpoint that is pure
   noise dressed as a measurement.
2. **A constant beats a bad detector.** Because the spread is that tight, "assume the tour median
   fraction" is already a strong predictor. `prior_frac` does exactly that and reads **no pose
   signal at all** — the counterpart to `tune_address.py`'s permanent `prior_tempo` row. A pose rule
   that does not clearly beat it is not extracting information from the swing; it is handing back
   the population median with extra steps, and the resulting checkpoint would score every golfer as
   average by construction.

So the headline column is `frac_err` — the error in the *metric the checkpoint would actually
report*, in the same units as the band it would be compared against. Frames are diagnostic; the
fraction is the verdict.

Both spans are reported twice, once from **detected** boundaries (the production path, where the
median-7-frame address error propagates straight into the denominator) and once from **labelled**
ones (which isolates the arm-parallel signal itself). The gap between them is how much of the damage
is address error rather than this rule.
"""

from __future__ import annotations

import argparse
import statistics
import sys
from collections.abc import Callable
from typing import NamedTuple

import common
import extract_pose
from pydantic import TypeAdapter

from golf_coach.analysis.phases import (
    _lead_wrist_xy,
    _motion_start,
    _top_and_impact,
    _wrist_confident,
)
from golf_coach.analysis.smoothing import smooth_keypoints
from golf_coach.contracts.keypoints import FrameKeypoints, PoseLandmark

_MIN_VISIBILITY = 0.5

# Tour medians for the no-pose baseline, from `golfdb_v1.json` (all/all/all, n=1399).
_PRIOR_MID_BACKSWING_FRAC = 0.693
_PRIOR_MID_DOWNSWING_FRAC = 0.567

# p10-p90 width of each metric in the reference population — what an error has to be small against
# for the checkpoint to carry information.
_TOUR_SPREAD = {"mid_backswing": 0.773 - 0.615, "mid_downswing": 0.660 - 0.429}

# PCE tolerance, matching bakeoff.py: a detection counts as correct within 1/30th of the clip's own
# event span, so the criterion means the same thing on real-time and slow-motion footage.
_PCE_TOLERANCE_DIVISOR = 30


class Series(NamedTuple):
    """Everything a rule may read for one clip, precomputed once.

    `height` is the signal both pose rules run on: lead wrist `y` minus lead shoulder `y`, so it is
    **positive with the hands below the shoulder**, crosses zero exactly when the lead arm is
    parallel to the ground, and goes negative once the hands are higher. Comparing `y` to `y` needs
    no pixel-aspect correction — that only matters when mixing the two axes.
    """

    height: list[float]
    confident: list[bool]
    detected: tuple[int, int, int]  # (address, top, impact) from our own detectors
    labelled: tuple[int, int, int]  # (address, top, impact) from GolfDB


Rule = Callable[[Series, bool], int]


def _lead_arm_height(frames: list[FrameKeypoints]) -> tuple[list[float], list[bool]]:
    """Lead wrist height relative to the lead shoulder, plus a per-frame confidence mask.

    Holds the last confident value through dim frames so the series stays continuous, exactly as
    `_lead_wrist_xy` does, and reports separately which frames were actually tracked — a zero
    crossing inside a held stretch is an artifact of the hold, not a swing event.
    """
    heights: list[float] = []
    confident: list[bool] = []
    last = 0.0
    for frame in frames:
        wrist = frame.landmark(PoseLandmark.LEFT_WRIST)
        shoulder = frame.landmark(PoseLandmark.LEFT_SHOULDER)
        good = min(wrist.visibility, shoulder.visibility) >= _MIN_VISIBILITY
        if good:
            last = wrist.y - shoulder.y
        heights.append(last)
        confident.append(good)
    return heights, confident


def _span(series: Series, labelled_span: bool, backswing: bool) -> tuple[int, int]:
    address, top, impact = series.labelled if labelled_span else series.detected
    return (address, top) if backswing else (top, impact)


def rule_cross(series: Series, labelled_span: bool) -> int:
    """First frame where the lead arm passes through horizontal — a zero crossing of `height`.

    A crossing beats an extremum here: through the backswing the hands rise monotonically past
    shoulder height, so the *sign change* is sharply localized, whereas `|height|` is flat near its
    minimum and its argmin wanders with landmark noise (the same asymmetry argument `phases.py`
    makes when it rejects parabola refinement of the top).
    """
    return _first_crossing(series, labelled_span, backswing=True)


def rule_cross_downswing(series: Series, labelled_span: bool) -> int:
    return _first_crossing(series, labelled_span, backswing=False)


def _first_crossing(series: Series, labelled_span: bool, backswing: bool) -> int:
    lo, hi = _span(series, labelled_span, backswing)
    lo, hi = max(0, lo), min(len(series.height) - 1, hi)
    if hi <= lo:
        return lo

    # Backswing: hands start below the shoulder (positive) and rise above it (negative).
    # Downswing: the reverse. Look for the first sign change in the expected direction.
    for index in range(lo + 1, hi + 1):
        if not (series.confident[index] and series.confident[index - 1]):
            continue
        before, after = series.height[index - 1], series.height[index]
        if backswing and before > 0.0 >= after:
            return index
        if not backswing and before <= 0.0 < after:
            return index
    # Never crossed (a short backswing that stays below the shoulder, or lost tracking): fall back
    # to the closest approach, which is at least the right kind of answer.
    window = range(lo, hi + 1)
    return min(window, key=lambda i: abs(series.height[i]))


def rule_argmin(series: Series, labelled_span: bool) -> int:
    lo, hi = _span(series, labelled_span, backswing=True)
    lo, hi = max(0, lo), min(len(series.height) - 1, hi)
    return min(range(lo, hi + 1), key=lambda i: abs(series.height[i])) if hi > lo else lo


def rule_argmin_downswing(series: Series, labelled_span: bool) -> int:
    lo, hi = _span(series, labelled_span, backswing=False)
    lo, hi = max(0, lo), min(len(series.height) - 1, hi)
    return min(range(lo, hi + 1), key=lambda i: abs(series.height[i])) if hi > lo else lo


def rule_prior(series: Series, labelled_span: bool) -> int:
    """No pose signal at all: place the instant at the tour-median fraction of the span.

    The bar every pose rule has to clear. See the module docstring.
    """
    lo, hi = _span(series, labelled_span, backswing=True)
    return lo + round(_PRIOR_MID_BACKSWING_FRAC * max(hi - lo, 0))


def rule_prior_downswing(series: Series, labelled_span: bool) -> int:
    lo, hi = _span(series, labelled_span, backswing=False)
    return lo + round(_PRIOR_MID_DOWNSWING_FRAC * max(hi - lo, 0))


BACKSWING_RULES: dict[str, Rule] = {
    "cross": rule_cross,
    "argmin": rule_argmin,
    "prior_frac": rule_prior,
}
DOWNSWING_RULES: dict[str, Rule] = {
    "cross": rule_cross_downswing,
    "argmin": rule_argmin_downswing,
    "prior_frac": rule_prior_downswing,
}


class Scored(NamedTuple):
    source_id: str
    slow_motion: bool
    event: str  # "mid_backswing" | "mid_downswing"
    truth: int
    truth_frac: float
    span: int  # the clip's own address-to-impact span, for normalization and PCE
    denominator: int  # backswing or downswing length, whichever this metric divides by
    origin: int  # start of the metric's span (address or top)
    predicted: dict[str, int]  # (rule, span_source) -> frame, keyed "<rule>/<det|lab>"


def _summarize(errors: list[float]) -> tuple[float, float, int]:
    if not errors:
        return (float("nan"), float("nan"), 0)
    return (
        statistics.median(errors),
        statistics.fmean(errors),
        sum(1 for e in errors if e > 10) * 100 // len(errors),
    )


def _frac(row: Scored, key: str) -> float:
    """The metric the checkpoint would actually report, from this rule's detected frame."""
    if row.denominator <= 0:
        return float("nan")
    return (row.predicted[key] - row.origin) / row.denominator


def _report(rows: list[Scored], event: str, rules: dict[str, Rule]) -> None:
    subset = [r for r in rows if r.event == event]
    if not subset:
        return
    slow = [r for r in subset if r.slow_motion]
    real = [r for r in subset if not r.slow_motion]
    spread = _TOUR_SPREAD[event]

    print(
        f"\n### {event}  -  {len(subset)} clips ({len(slow)} slow-motion, {len(real)} real-time)"
    )
    print(f"    tour p10-p90 spread = {spread:.3f}; an error near that is a useless checkpoint\n")
    header = (
        f"{'rule':<20}{'med':>6}{'mean':>7}{'>10':>6}{'norm':>7}{'PCE':>7}"
        f"{'frac_err':>10}{'vs spread':>11} | {'slow':>6}{'real':>6}"
    )
    print(header)
    print("-" * len(header))

    for name in rules:
        for source, label in (("det", "detected"), ("lab", "labelled")):
            key = f"{name}/{source}"
            errors = [float(abs(r.predicted[key] - r.truth)) for r in subset]
            normalized = [abs(r.predicted[key] - r.truth) / r.span for r in subset]
            correct = sum(
                1
                for r in subset
                if abs(r.predicted[key] - r.truth) <= max(round(r.span / _PCE_TOLERANCE_DIVISOR), 1)
            )
            frac_errors = [
                abs(_frac(r, key) - r.truth_frac)
                for r in subset
                if r.denominator > 0
            ]
            median, mean, over = _summarize(errors)
            frac_err = statistics.median(frac_errors) if frac_errors else float("nan")
            slow_med, _, _ = _summarize(
                [float(abs(r.predicted[key] - r.truth)) for r in slow]
            )
            real_med, _, _ = _summarize(
                [float(abs(r.predicted[key] - r.truth)) for r in real]
            )
            print(
                f"{name + ' (' + label + ')':<20}{median:>6.1f}{mean:>7.1f}{over:>5}%"
                f"{statistics.median(normalized):>7.3f}{100.0 * correct / len(subset):>6.1f}%"
                f"{frac_err:>10.3f}{frac_err / spread:>10.0%}  | "
                f"{slow_med:>6.1f}{real_med:>6.1f}"
            )


def _report_detail(rows: list[Scored], event: str, name: str) -> None:
    subset = [r for r in rows if r.event == event]
    key = f"{name}/det"
    if not subset or key not in subset[0].predicted:
        print(f"error: unknown rule {name!r}", file=sys.stderr)
        return
    signed = [r.predicted[key] - r.truth for r in subset]
    print(f"\nSigned error for {name!r} on {event} (detected span; positive = late):")
    print(
        f"  median {statistics.median(signed):+.1f}   "
        f"early {sum(1 for e in signed if e < 0) * 100 // len(signed)}%   "
        f"late {sum(1 for e in signed if e > 0) * 100 // len(signed)}%"
    )
    worst = sorted(subset, key=lambda r: -abs(r.predicted[key] - r.truth))
    print(f"\n  {'clip':>6} {'err':>6} {'found':>6} {'truth':>6} {'frac':>6} {'true':>6}  slow-mo")
    for row in worst[:12]:
        print(
            f"  {row.source_id:>6} {row.predicted[key] - row.truth:>+6} "
            f"{row.predicted[key]:>6} {row.truth:>6} {_frac(row, key):>6.3f} "
            f"{row.truth_frac:>6.3f}  {'yes' if row.slow_motion else 'no'}"
        )


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(
        description="Gate arm-parallel instant detection against GolfDB ground truth."
    )
    parser.add_argument("--estimator", default="mediapipe:lite")
    parser.add_argument("--limit", type=int, default=0)
    parser.add_argument("--detail", default="", help="rule name for a signed-error breakdown")
    args = parser.parse_args(argv)

    cache = common.KEYPOINTS_DIR / extract_pose.estimator_slug(args.estimator)
    if not cache.is_dir():
        print(f"error: no cache at {cache} — run extract_pose.py first", file=sys.stderr)
        return 1

    adapter = TypeAdapter(list[FrameKeypoints])
    swings = {s.source_id: s for s in common.load_swings()}

    cached = sorted(cache.glob("*.keypoints.json"), key=lambda p: int(p.name.split(".")[0]))
    if args.limit:
        cached = cached[: args.limit]

    rows: list[Scored] = []
    skipped = 0
    for path in cached:
        swing = swings.get(path.name.split(".")[0])
        if swing is None or swing.view != common.VIEW_FACE_ON:
            continue
        keypoints = adapter.validate_json(path.read_bytes())
        if len(keypoints) < 6:
            continue

        origin = swing.events["start"]
        events = {name: swing.events[name] - origin for name in swing.events}
        address, top, impact = events["address"], events["top"], events["impact"]
        mid_back, mid_down = events["mid_backswing"], events["mid_downswing"]
        # Same exclusion as bakeoff.py: a few videos_160 clips are shorter than their annotation.
        if impact >= len(keypoints) or not 0 <= address < top < impact:
            skipped += 1
            continue

        smoothed = smooth_keypoints(keypoints)
        xy = _lead_wrist_xy(smoothed)
        ys = [y for _, y in xy]
        det_top, det_impact = _top_and_impact(ys, _wrist_confident(smoothed), len(smoothed))
        det_address, _ = _motion_start(xy, det_top, det_impact)
        heights, confident = _lead_arm_height(smoothed)

        series = Series(
            height=heights,
            confident=confident,
            detected=(det_address, det_top, det_impact),
            labelled=(address, top, impact),
        )

        for event, truth, rules, denom, start in (
            ("mid_backswing", mid_back, BACKSWING_RULES, top - address, address),
            ("mid_downswing", mid_down, DOWNSWING_RULES, impact - top, top),
        ):
            if denom <= 0:
                continue
            rows.append(
                Scored(
                    source_id=swing.source_id,
                    slow_motion=swing.slow_motion,
                    event=event,
                    truth=truth,
                    truth_frac=(truth - start) / denom,
                    span=impact - address,
                    denominator=denom,
                    origin=start,
                    predicted={
                        f"{name}/{source}": rule(series, source == "lab")
                        for name, rule in rules.items()
                        for source in ("det", "lab")
                    },
                )
            )

    if not rows:
        print("error: no scorable clips", file=sys.stderr)
        return 1

    clips = len({r.source_id for r in rows})
    print(f"Scored {clips} cached face-on clips ({args.estimator})")
    if skipped:
        print(f"Excluded {skipped} clip(s) shorter than their annotation or out of order")

    _report(rows, "mid_backswing", BACKSWING_RULES)
    _report(rows, "mid_downswing", DOWNSWING_RULES)
    if args.detail:
        _report_detail(rows, "mid_backswing", args.detail)
        _report_detail(rows, "mid_downswing", args.detail)
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
