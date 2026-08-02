"""Compare candidate *address* (takeaway-onset) rules against GolfDB ground truth. [M4-REF Phase B6]

Usage:
    python scripts/golfdb/tune_address.py
    python scripts/golfdb/tune_address.py --estimator mediapipe:lite --limit 200
    python scripts/golfdb/tune_address.py --detail clip_relative

The sibling of `tune_phases.py`, which sweeps top/impact only and never touched address. Phase B5
swept the two address constants ad hoc and kept only the results table, so there was no runnable
address report at all — this is that report, and the record of what was tried.

Reads the **cached** keypoints (`extract_pose.py` must have run), so a rule change is a one-second
experiment rather than a re-run of the estimator.

### Why address is reported differently from top and impact

Address is the only instant whose error is dominated by the *corpus mix* rather than the rule. The
face-on corpus is ~47% broadcast slow-motion, where a swing occupies four times as many frames, and
a pooled median in frames therefore substantially measures how much slow-motion is in the sample.
Every table here is split by `slow_motion` and carries `med_norm` (error as a fraction of the
clip's own address-to-impact span) alongside the raw frame counts.

Two reference rows are permanent and should never be removed:

- **`current (fixed stall)`** — the pre-B6 shipped rule, reimplemented locally rather than imported,
  so it stays a stable baseline after `phases.py` changes.
- **`prior_tempo`** — `top - 3.5 x (impact - top)`, which uses **no wrist signal at all**. It scores
  a median of 11 frames. Any candidate that does not clearly beat it is not extracting information
  from the pose; that is the bar, not the `current` row.

### Rejected families, kept as rules

`setup_ball`, `noise_floor`, `ramp_extrapolation`, `torso_energy`, `upper_energy`, `shoulder_turn`
and `persistence` are all *worse* than the wrist-speed family. They stay here so the next person to
have one of these ideas can re-run it in a second instead of rebuilding the harness. See
docs/M4_POSE_BAKEOFF.md Phase B6 for the numbers and why each fails.
"""

from __future__ import annotations

import argparse
import math
import statistics
import sys
from collections.abc import Callable
from typing import NamedTuple

import common
import extract_pose

from golf_coach.analysis.phases import (
    _lead_wrist_xy,
    _top_and_impact,
    _wrist_confident,
    _wrist_speed,
)
from golf_coach.analysis.smoothing import smooth_keypoints
from golf_coach.contracts.keypoints import FrameKeypoints, PoseLandmark
from golf_coach.contracts.reference import ReferenceSwing

_MIN_VISIBILITY = 0.5

# Landmark sets for the multi-joint "motion energy" candidates. The takeaway is a body rotation, so
# in principle the torso should announce it before the hands do; at 160x160 it does not.
_TORSO = (
    PoseLandmark.LEFT_SHOULDER,
    PoseLandmark.RIGHT_SHOULDER,
    PoseLandmark.LEFT_HIP,
    PoseLandmark.RIGHT_HIP,
)
_UPPER = _TORSO + (
    PoseLandmark.LEFT_ELBOW,
    PoseLandmark.RIGHT_ELBOW,
    PoseLandmark.LEFT_WRIST,
    PoseLandmark.RIGHT_WRIST,
)

# The tempo ratio behind `prior_tempo` and behind the shipped bounded fallback: the GolfDB median
# over 1,399 clips (ADR-012), not a book value.
_PRIOR_TEMPO_RATIO = 3.5


class Series(NamedTuple):
    """Everything a candidate rule may read, precomputed once per clip.

    `top` and `impact` are the *detected* instants, not the labelled ones — a rule that consumed
    ground truth for them would flatter itself. They are accurate enough to be an input (median 2
    and 1 frames) and they supply the clip's own time base, which is what the winning family needs.
    """

    xy: list[tuple[float, float]]
    speeds: list[float]
    top: int
    impact: int
    shoulder_width: float | None
    torso_energy: list[float]
    upper_energy: list[float]
    shoulder_turn: list[float]


Rule = Callable[[Series], int]


# --- helpers shared by several rules ---------------------------------------------------------


def _quiet_walk(signal: list[float], top: int, fraction: float, stall: int) -> int | None:
    """Walk back from the top; return the frame after the last run of `stall` quiet frames.

    `None` when the signal never settles — the caller decides what to do about it, which is the
    whole point of Phase B6: the shipped rule used to answer `0` here.
    """
    if top <= 0:
        return 0
    peak = max(signal[1 : top + 1], default=0.0)
    if peak <= 0.0:
        return 0
    threshold = peak * fraction

    quiet = 0
    for index in range(top, -1, -1):
        if signal[index] < threshold:
            quiet += 1
            if quiet >= stall:
                return min(index + quiet, top)
        else:
            quiet = 0
    return None


def _prior(top: int, impact: int) -> int:
    """Address implied purely by a tour-median tempo ratio. No pose signal whatsoever."""
    return max(0, top - round(_PRIOR_TEMPO_RATIO * max(impact - top, 1)))


# --- reference rows --------------------------------------------------------------------------


def rule_current(series: Series) -> int:
    """The pre-B6 shipped rule: 5% of peak wrist speed, a **fixed** 4-frame stall, falling to 0.

    Reimplemented here rather than imported so it stays a fixed baseline once `phases.py` moves on.
    """
    found = _quiet_walk(series.speeds, series.top, 0.05, 4)
    return 0 if found is None else found


def rule_prior_tempo(series: Series) -> int:
    """The bar every pose-based rule has to clear. See the module docstring."""
    return _prior(series.top, series.impact)


# --- the adopted family: the quiet run scales with the clip's own time base -------------------


def _clip_relative(fraction: float, alpha: float, bounded: bool) -> Rule:
    """Quiet-run length as a fraction of the clip's own downswing duration.

    A fixed frame count is the one fps-dependent absolute in the address path: the downswing is 8
    frames in a real-time clip and 30 in a broadcast slow-motion one, so "4 quiet frames" means
    something four times different in the two halves of this corpus. A slow takeaway also spends
    long stretches below any fraction of its own peak, which is why the fixed-stall rule stops
    mid-takeaway and reads *late* (signed median +4 frames, 66% of clips late).
    """

    def rule(series: Series) -> int:
        downswing = max(series.impact - series.top, 1)
        stall = max(2, round(alpha * downswing))
        found = _quiet_walk(series.speeds, series.top, fraction, stall)
        if found is not None:
            return found
        return _prior(series.top, series.impact) if bounded else 0

    return rule


# --- rejected families (kept runnable; see docs/M4_POSE_BAKEOFF.md Phase B6) ------------------


def _density_anchor(xy: list[tuple[float, float]], top: int, radius: float) -> tuple[float, float]:
    """Densest cluster of wrist positions before the top — where the golfer stood the longest."""
    bins: dict[tuple[int, int], list[int]] = {}
    for index in range(top):
        key = (int(xy[index][0] // radius), int(xy[index][1] // radius))
        bins.setdefault(key, []).append(index)
    best = max(
        bins,
        key=lambda k: sum(
            len(bins.get((k[0] + dx, k[1] + dy), ())) for dx in (-1, 0, 1) for dy in (-1, 0, 1)
        ),
    )
    members = [
        i
        for dx in (-1, 0, 1)
        for dy in (-1, 0, 1)
        for i in bins.get((best[0] + dx, best[1] + dy), ())
    ]
    return (
        statistics.fmean(xy[i][0] for i in members),
        statistics.fmean(xy[i][1] for i in members),
    )


def _setup_ball(epsilon: float) -> Rule:
    """REJECTED. Onset = the frame the wrist leaves a ball around its setup position.

    Displacement is fps-invariant where speed is not, so this should have been the slow-motion fix.
    It is not: a gradual takeaway leaves a fixed-radius ball *late*, and shrinking the radius to
    compensate runs into the pose noise floor. Median 14 frames, and worse the looser the ball.
    """

    def rule(series: Series) -> int:
        if series.top <= 0 or not series.shoulder_width:
            return 0
        radius = epsilon * series.shoulder_width
        anchor = _density_anchor(series.xy, series.top, radius)
        for t in range(series.top - 1, -1, -1):
            dx = series.xy[t][0] - anchor[0]
            dy = series.xy[t][1] - anchor[1]
            if math.hypot(dx, dy) < radius:
                return min(t + 1, series.top)
        return 0

    return rule


def _noise_floor(k: float) -> Rule:
    """REJECTED. Threshold from the setup noise floor (median + k*MAD) instead of the swing peak.

    The motivating idea is sound — setup jitter and swing peak speed are unrelated quantities — but
    without a persistence requirement any single slow frame mid-takeaway trips it, and the pre-top
    window used to estimate the floor is itself mostly takeaway on slow-motion clips. Median 41.
    """

    def rule(series: Series) -> int:
        if series.top <= 0:
            return 0
        body = sorted(series.speeds[1 : series.top + 1])
        if len(body) < 4:
            return 0
        median = statistics.median(body)
        mad = statistics.median([abs(v - median) for v in body]) or 1e-9
        threshold = median + k * mad
        for t in range(series.top - 1, -1, -1):
            if series.speeds[t] < threshold:
                return min(t + 1, series.top)
        return 0

    return rule


def _ramp_extrapolation(low: float, high: float) -> Rule:
    """REJECTED. Fit the takeaway speed ramp and take its t-intercept.

    The textbook onset estimator for a gradual departure from rest, and it does remove the
    threshold-crossing bias (signed error goes to ~0). But the lead wrist's early takeaway is not
    linear in speed, so the fit is dominated by wherever the band happens to land. Median 14-19.
    """

    def rule(series: Series) -> int:
        top = series.top
        if top <= 2:
            return 0
        peak = max(series.speeds[1 : top + 1], default=0.0)
        if peak <= 0.0:
            return 0
        t_high = next((t for t in range(top, 0, -1) if series.speeds[t] >= high * peak), None)
        if t_high is None:
            return 0
        t_low = next((t for t in range(t_high, -1, -1) if series.speeds[t] <= low * peak), 0)
        points = [(t, series.speeds[t]) for t in range(t_low, t_high + 1)]
        if len(points) < 3:
            return max(t_low, 0)
        mean_t = statistics.fmean(p[0] for p in points)
        mean_v = statistics.fmean(p[1] for p in points)
        denominator = sum((p[0] - mean_t) ** 2 for p in points)
        if denominator <= 0:
            return max(t_low, 0)
        slope = sum((p[0] - mean_t) * (p[1] - mean_v) for p in points) / denominator
        if slope <= 0:
            return max(t_low, 0)
        return max(0, min(top, round(mean_t - mean_v / slope)))

    return rule


def _energy_rule(which: str, fraction: float) -> Rule:
    """REJECTED. The same quiet-run walk over a multi-joint motion-energy signal.

    Averaging N joints should suppress independent landmark noise by sqrt(N) and let the threshold
    drop far enough to catch a gradual onset. On 160x160 crops it does not pay: the torso barely
    moves during a takeaway (median 19), and upper-body energy merely *ties* the lead wrist alone
    (8.0) while costing eight landmarks of extraction. The wrist is where the takeaway shows first.
    """

    def rule(series: Series) -> int:
        signal = series.torso_energy if which == "torso" else series.upper_energy
        downswing = max(series.impact - series.top, 1)
        found = _quiet_walk(signal, series.top, fraction, max(2, round(0.25 * downswing)))
        return 0 if found is None else found

    return rule


def _shoulder_turn(fraction: float) -> Rule:
    """REJECTED. Onset of shoulder-line rotation — the takeaway *is* a shoulder turn.

    Conceptually the most attractive candidate: rotation is a large, low-frequency signal and a
    waggle moves the hands without turning the shoulders. At this resolution the shoulder line is
    too short a baseline for a stable angle, so the derivative is mostly noise. Median 24-38.
    """

    def rule(series: Series) -> int:
        downswing = max(series.impact - series.top, 1)
        found = _quiet_walk(
            series.shoulder_turn, series.top, fraction, max(2, round(0.25 * downswing))
        )
        return 0 if found is None else found

    return rule


def _persistence(minimum: float, alpha: float) -> Rule:
    """REJECTED. Net displacement over path length — an *amplitude-free* onset test.

    The one candidate that should have been immune to slow motion: jitter wanders (ratio near 0),
    a takeaway travels straight (ratio near 1), however slowly. It fails because `smooth_keypoints`
    is a centered moving average, which correlates neighbouring frames and makes setup jitter look
    directionally persistent too — and because golfers drift at address. Median 23-45.
    """

    def rule(series: Series) -> int:
        top = series.top
        if top <= 2:
            return 0
        window = max(3, round(alpha * max(series.impact - top, 1)))
        for t in range(top - window, -1, -1):
            path = sum(series.speeds[t + 1 : t + window + 1])
            if path <= 0:
                continue
            net = math.hypot(
                series.xy[t + window][0] - series.xy[t][0],
                series.xy[t + window][1] - series.xy[t][1],
            )
            if net / path < minimum:
                return min(t + 1, top)
        return 0

    return rule


RULES: dict[str, Rule] = {
    "current (fixed stall)": rule_current,
    "prior_tempo (no pose)": rule_prior_tempo,
    **{
        f"clip_relative {f:.2f}/{a:.2f}": _clip_relative(f, a, bounded=False)
        for f in (0.04, 0.05)
        for a in (0.20, 0.25, 0.30)
    },
    "clip_relative bounded": _clip_relative(0.05, 0.25, bounded=True),
    "setup_ball 0.10": _setup_ball(0.10),
    "setup_ball 0.20": _setup_ball(0.20),
    "noise_floor k=3": _noise_floor(3.0),
    "noise_floor k=10": _noise_floor(10.0),
    "ramp_extrap 0.05-0.30": _ramp_extrapolation(0.05, 0.30),
    "ramp_extrap 0.10-0.50": _ramp_extrapolation(0.10, 0.50),
    "torso_energy 0.10": _energy_rule("torso", 0.10),
    "upper_energy 0.10": _energy_rule("upper", 0.10),
    "shoulder_turn 0.05": _shoulder_turn(0.05),
    "persistence 0.70": _persistence(0.70, 0.15),
}


# --- per-clip signal construction ------------------------------------------------------------


def _motion_energy(
    frames: list[FrameKeypoints], landmarks: tuple[PoseLandmark, ...], aspect: float
) -> list[float]:
    """Mean per-frame joint displacement over a landmark set; `0.0` at the first frame."""
    energy = [0.0]
    for before, after in zip(frames, frames[1:], strict=False):
        samples = []
        for landmark in landmarks:
            a = before.landmark(landmark)
            b = after.landmark(landmark)
            if min(a.visibility, b.visibility) < _MIN_VISIBILITY:
                continue
            samples.append(math.hypot((b.x - a.x) * aspect, b.y - a.y))
        energy.append(statistics.fmean(samples) if samples else 0.0)
    return energy


def _shoulder_turn_series(frames: list[FrameKeypoints], aspect: float) -> list[float]:
    """Per-frame absolute change in shoulder-line angle."""
    angles: list[float] = []
    for frame in frames:
        left = frame.landmark(PoseLandmark.LEFT_SHOULDER)
        right = frame.landmark(PoseLandmark.RIGHT_SHOULDER)
        if min(left.visibility, right.visibility) >= _MIN_VISIBILITY:
            angles.append(math.atan2(left.y - right.y, (left.x - right.x) * aspect))
        else:
            angles.append(angles[-1] if angles else 0.0)
    return [0.0] + [abs(b - a) for a, b in zip(angles, angles[1:], strict=False)]


def _median_shoulder_width(frames: list[FrameKeypoints], aspect: float) -> float | None:
    widths = []
    for frame in frames:
        left = frame.landmark(PoseLandmark.LEFT_SHOULDER)
        right = frame.landmark(PoseLandmark.RIGHT_SHOULDER)
        if min(left.visibility, right.visibility) >= _MIN_VISIBILITY:
            widths.append(math.hypot((left.x - right.x) * aspect, left.y - right.y))
    return statistics.median(widths) if widths else None


def _series(keypoints: list[FrameKeypoints], aspect: float) -> Series:
    smoothed = smooth_keypoints(keypoints)
    xy = _lead_wrist_xy(smoothed)
    ys = [y for _, y in xy]
    top, impact = _top_and_impact(ys, _wrist_confident(smoothed), len(smoothed))
    return Series(
        xy=[(x * aspect, y) for x, y in xy],
        speeds=_wrist_speed(xy),
        top=top,
        impact=impact,
        shoulder_width=_median_shoulder_width(smoothed[: max(top, 1)], aspect),
        torso_energy=_motion_energy(smoothed, _TORSO, aspect),
        upper_energy=_motion_energy(smoothed, _UPPER, aspect),
        shoulder_turn=_shoulder_turn_series(smoothed, aspect),
    )


# --- scoring ---------------------------------------------------------------------------------


class Scored(NamedTuple):
    source_id: str
    truth: int
    span: int
    slow_motion: bool
    clean_top: bool
    predicted: dict[str, int]


def _summarize(errors: list[int]) -> tuple[float, float, int]:
    return (
        statistics.median(errors),
        statistics.fmean(errors),
        sum(1 for e in errors if e > 10) * 100 // len(errors),
    )


def _report(rows: list[Scored]) -> None:
    clean = [r for r in rows if r.clean_top]
    slow = [r for r in rows if r.slow_motion]
    real = [r for r in rows if not r.slow_motion]
    print(
        f"\n{len(rows)} clips - {len(slow)} slow-motion, {len(real)} real-time; "
        f"{len(clean)} with a clean top (|top error| <= 5)\n"
    )
    header = (
        f"{'rule':<22}{'med':>6}{'mean':>7}{'>10':>6}{'norm':>7}{'PCE':>7} | "
        f"{'slow':>6}{'>10':>6} | {'real':>6}{'>10':>6} | {'clean':>7}{'>10':>6}"
    )
    print(header)
    print("-" * len(header))
    for name in RULES:
        errors = [abs(r.predicted[name] - r.truth) for r in rows]
        if not errors:
            continue
        normalized = [abs(r.predicted[name] - r.truth) / r.span for r in rows]
        correct = sum(
            1
            for r in rows
            if abs(r.predicted[name] - r.truth) <= max(round(r.span / 30), 1)
        )
        median, mean, over = _summarize(errors)
        slow_med, _, slow_over = _summarize([abs(r.predicted[name] - r.truth) for r in slow])
        real_med, _, real_over = _summarize([abs(r.predicted[name] - r.truth) for r in real])
        clean_med, _, clean_over = _summarize([abs(r.predicted[name] - r.truth) for r in clean])
        print(
            f"{name:<22}{median:>6.1f}{mean:>7.1f}{over:>5}%"
            f"{statistics.median(normalized):>7.3f}{100.0 * correct / len(rows):>6.1f}% | "
            f"{slow_med:>6.1f}{slow_over:>5}% | {real_med:>6.1f}{real_over:>5}% | "
            f"{clean_med:>7.1f}{clean_over:>5}%"
        )


def _report_detail(rows: list[Scored], name: str) -> None:
    if name not in RULES:
        print(f"error: unknown rule {name!r}", file=sys.stderr)
        return
    signed = sorted(rows, key=lambda r: -abs(r.predicted[name] - r.truth))
    errors = [r.predicted[name] - r.truth for r in rows]
    print(f"\nSigned error for {name!r} (positive = detected late):")
    print(
        f"  median {statistics.median(errors):+.1f}   "
        f"early {sum(1 for e in errors if e < 0) * 100 // len(errors)}%   "
        f"late {sum(1 for e in errors if e > 0) * 100 // len(errors)}%"
    )
    print(f"\n  {'clip':>6} {'err':>6} {'found':>6} {'truth':>6} {'span':>6}  slow-mo")
    for row in signed[:12]:
        print(
            f"  {row.source_id:>6} {row.predicted[name] - row.truth:>+6} "
            f"{row.predicted[name]:>6} {row.truth:>6} {row.span:>6}  "
            f"{'yes' if row.slow_motion else 'no'}"
        )


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description="Tune address detection against ground truth.")
    parser.add_argument("--estimator", default="mediapipe:lite")
    parser.add_argument("--limit", type=int, default=0)
    parser.add_argument(
        "--detail",
        default="",
        help="rule name: print its signed-error distribution and worst clips, which is how the "
        "late bias of a threshold-crossing rule becomes visible",
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

    rows: list[Scored] = []
    truncated: list[str] = []
    for path in cached:
        swing = swings.get(path.name.split(".")[0])
        if swing is None:
            continue
        keypoints = adapter.validate_json(path.read_bytes())
        if len(keypoints) < 6:
            continue

        origin = swing.events["start"]
        address = swing.events["address"] - origin
        labelled_top = swing.events["top"] - origin
        impact = swing.events["impact"] - origin
        # Same exclusion as bakeoff.py / tune_phases.py: a few videos_160 clips are shorter than
        # their own annotation, so the labelled impact frame does not exist in the file.
        if impact >= len(keypoints) or impact - address <= 0:
            truncated.append(swing.source_id)
            continue

        series = _series(keypoints, swing.pixel_aspect or 1.0)
        rows.append(
            Scored(
                source_id=swing.source_id,
                truth=address,
                span=impact - address,
                slow_motion=swing.slow_motion,
                clean_top=abs(series.top - labelled_top) <= 5,
                predicted={name: rule(series) for name, rule in RULES.items()},
            )
        )

    if not rows:
        print("error: no scorable clips", file=sys.stderr)
        return 1

    print(f"Scored {len(rows)} cached clips ({args.estimator})")
    if truncated:
        print(f"Excluded {len(truncated)} clip(s) shorter than their annotation: {truncated[:8]}")
    _report(rows)
    if args.detail:
        _report_detail(rows, args.detail)
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
