"""Mechanics checkpoints — pose-based, intent-independent. [M4-PoC+]

Three checkpoints, all measured from **face-on 2D pose** (the canonical pose-camera placement,
ADR-003 addendum) — deliberately the ones this single view reads well:

- **tempo** — backswing:downswing time ratio (~3:1, "Tour Tempo"), from phase timings.
- **head_sway** — lateral (`x`) head travel from address to impact, in shoulder-widths.
- **finish_balance** — how still the body settles through the follow-through, in shoulder-widths.

Each compares an observed value against a benchmark band resolved from the store (ADR-010) and
returns a `CheckpointScore`, or `None` when the data is unusable or the store has no band — no
score beats a wrong one (ADR-010 §2). Pure functions, stdlib only. Checkpoints needing depth
(spine tilt, hip rotation, swing plane) are *not* here — they need a down-the-line/synced view
(ADR-011) and stay deferred; see docs/M4_FUNDAMENTALS_PANEL.md.

HARDWARE-REVALIDATE: `head_sway` / `finish_balance` thresholds are provisional, uncalibrated
placeholders (see ranges.json provenance). Recalibrate against captured ground-truth data and
revisit the deferred depth checkpoints when the down-the-line camera / 3D fusion land (ADR-011).
"""

from __future__ import annotations

from golf_coach.analysis.benchmarks import resolve_range
from golf_coach.contracts.intent import ClubCategory, PlayerProfile
from golf_coach.contracts.keypoints import FrameKeypoints, PoseLandmark
from golf_coach.contracts.swing import CheckpointScore, PhaseSegment, SwingPhase

_TEMPO_CHECKPOINT = "tempo"
_TEMPO_RANGE_KEY = "tempo_ratio"

_HEAD_SWAY_CHECKPOINT = "head_sway"
_HEAD_SWAY_RANGE_KEY = "head_sway_norm"

_FINISH_BALANCE_CHECKPOINT = "finish_balance"
_FINISH_BALANCE_RANGE_KEY = "finish_balance_norm"

# Landmarks dimmer than this are treated as unreliable (MediaPipe convention, matches
# phases.py / overlay.py).
_MIN_VISIBILITY = 0.5

# A face-on shoulder width (normalized) below this is degenerate (golfer turned side-on or
# shoulders mis-detected) — we can't form a reliable scale, so the checkpoint bails.
_MIN_SHOULDER_WIDTH = 0.02

# Fewest follow-through frames needed to judge how still the finish settles.
_MIN_FINISH_FRAMES = 3


def _score_within_range(observed: float, low: float, high: float) -> float:
    """1.0 inside `[low, high]`, decaying linearly (by band-widths) to 0.0 outside.

    Shared scorer for every mechanics checkpoint in this module. For a one-sided "lower is
    better" metric (sway, finish drift) pass `low=0.0`, so any non-negative value inside the
    band scores 1.0 and only overshoot past `high` is penalised.
    """
    if low <= observed <= high:
        return 1.0
    width = high - low
    if width <= 0:
        return 0.0
    distance = (low - observed) if observed < low else (observed - high)
    return max(0.0, 1.0 - distance / width)


def _phase_bounds(phases: list[PhaseSegment], phase: SwingPhase) -> tuple[int, int] | None:
    """Inclusive `(start_frame, end_frame)` for a phase, or None if it isn't present."""
    for segment in phases:
        if segment.phase is phase:
            return segment.start_frame, segment.end_frame
    return None


def _mean_point(
    keypoints: list[FrameKeypoints], lo: int, hi: int, which: PoseLandmark
) -> tuple[float, float] | None:
    """Mean `(x, y)` of a landmark over the inclusive frame span, confident samples only."""
    points = [
        (kp.landmark(which).x, kp.landmark(which).y)
        for kp in keypoints[lo : hi + 1]
        if kp.landmark(which).visibility >= _MIN_VISIBILITY
    ]
    if not points:
        return None
    return (
        sum(x for x, _ in points) / len(points),
        sum(y for _, y in points) / len(points),
    )


def _shoulder_width(keypoints: list[FrameKeypoints], lo: int, hi: int) -> float | None:
    """Mean face-on shoulder width (normalized) over a span — the scale-invariance ruler.

    Returns None if too few confident frames or the width is degenerate (side-on / mis-detect).
    """
    widths = [
        abs(kp.landmark(PoseLandmark.LEFT_SHOULDER).x - kp.landmark(PoseLandmark.RIGHT_SHOULDER).x)
        for kp in keypoints[lo : hi + 1]
        if kp.landmark(PoseLandmark.LEFT_SHOULDER).visibility >= _MIN_VISIBILITY
        and kp.landmark(PoseLandmark.RIGHT_SHOULDER).visibility >= _MIN_VISIBILITY
    ]
    if not widths:
        return None
    width = sum(widths) / len(widths)
    return width if width >= _MIN_SHOULDER_WIDTH else None


def _hip_center_points(
    keypoints: list[FrameKeypoints], lo: int, hi: int
) -> list[tuple[float, float]]:
    """Per-frame hip-center `(x, y)` over the inclusive span, confident frames only."""
    points: list[tuple[float, float]] = []
    for kp in keypoints[lo : hi + 1]:
        left = kp.landmark(PoseLandmark.LEFT_HIP)
        right = kp.landmark(PoseLandmark.RIGHT_HIP)
        if left.visibility >= _MIN_VISIBILITY and right.visibility >= _MIN_VISIBILITY:
            points.append(((left.x + right.x) / 2, (left.y + right.y) / 2))
    return points


def _tempo_timings(phases: list[PhaseSegment]) -> tuple[float, float] | None:
    """Extract (backswing_ms, downswing_ms) from the phase boundaries, or None if unavailable.

    Uses motion start (`BACKSWING.start_ms`), the top of the backswing (center of the
    `TRANSITION` window), and impact (`IMPACT.start_ms`).
    """
    by_phase = {segment.phase: segment for segment in phases}
    backswing = by_phase.get(SwingPhase.BACKSWING)
    transition = by_phase.get(SwingPhase.TRANSITION)
    impact = by_phase.get(SwingPhase.IMPACT)
    if backswing is None or transition is None or impact is None:
        return None

    motion_start_ms = backswing.start_ms
    top_ms = (transition.start_ms + transition.end_ms) / 2
    impact_ms = impact.start_ms

    backswing_ms = top_ms - motion_start_ms
    downswing_ms = impact_ms - top_ms
    if backswing_ms <= 0 or downswing_ms <= 0:
        return None
    return backswing_ms, downswing_ms


def evaluate_tempo(
    phases: list[PhaseSegment],
    club: ClubCategory = ClubCategory.ALL,
    profile: PlayerProfile | None = None,
) -> CheckpointScore | None:
    """Score swing tempo (backswing:downswing ratio) against the benchmark band.

    Returns `None` when the phases don't yield a usable tempo or when the store has no band
    for this checkpoint — in both cases the caller simply omits a tempo score.
    """
    timings = _tempo_timings(phases)
    if timings is None:
        return None
    backswing_ms, downswing_ms = timings
    observed = backswing_ms / downswing_ms

    band = resolve_range(_TEMPO_RANGE_KEY, club, profile)
    if band is None:
        return None

    score = _score_within_range(observed, band.low, band.high)
    passed = band.low <= observed <= band.high
    if passed:
        message = f"Good tempo - {observed:.1f}:1 backswing:downswing (ideal ~3:1)."
    elif observed < band.low:
        message = (
            f"Tempo too quick - {observed:.1f}:1. The downswing is rushing the backswing; "
            "feel a smoother, fuller backswing (aim ~3:1)."
        )
    else:
        message = (
            f"Tempo too slow - {observed:.1f}:1. The backswing is dragging relative to the "
            "downswing; let the downswing flow a touch quicker (aim ~3:1)."
        )

    return CheckpointScore(
        name=_TEMPO_CHECKPOINT,
        score=score,
        passed=passed,
        observed=round(observed, 2),
        expected_low=band.low,
        expected_high=band.high,
        message=message,
    )


def evaluate_head_sway(
    keypoints: list[FrameKeypoints],
    phases: list[PhaseSegment],
    club: ClubCategory = ClubCategory.ALL,
    profile: PlayerProfile | None = None,
) -> CheckpointScore | None:
    """Score lateral head stability: `x` travel of the nose from address to impact.

    Measured in shoulder-widths so it is independent of the golfer's distance from the camera.
    Face-on is the ideal view for side-to-side sway. Returns `None` if the phases/landmarks
    are unusable or the store has no band (the caller then omits a sway score).
    """
    address = _phase_bounds(phases, SwingPhase.ADDRESS)
    impact = _phase_bounds(phases, SwingPhase.IMPACT)
    if address is None or impact is None:
        return None

    nose_address = _mean_point(keypoints, address[0], address[1], PoseLandmark.NOSE)
    nose_impact = _mean_point(keypoints, impact[0], impact[1], PoseLandmark.NOSE)
    width = _shoulder_width(keypoints, address[0], address[1])
    if nose_address is None or nose_impact is None or width is None:
        return None

    observed = abs(nose_impact[0] - nose_address[0]) / width

    band = resolve_range(_HEAD_SWAY_RANGE_KEY, club, profile)
    if band is None:
        return None

    score = _score_within_range(observed, band.low, band.high)
    passed = band.low <= observed <= band.high
    if passed:
        message = (
            f"Good head stability - {observed:.2f} shoulder-widths of lateral head movement "
            "to impact (nicely centered over the ball)."
        )
    else:
        message = (
            f"Head sway - {observed:.2f} shoulder-widths of lateral movement to impact. "
            f"Keep your head centered over the ball (aim under {band.high})."
        )

    return CheckpointScore(
        name=_HEAD_SWAY_CHECKPOINT,
        score=score,
        passed=passed,
        observed=round(observed, 2),
        expected_low=band.low,
        expected_high=band.high,
        message=message,
    )


def evaluate_finish_balance(
    keypoints: list[FrameKeypoints],
    phases: list[PhaseSegment],
    club: ClubCategory = ClubCategory.ALL,
    profile: PlayerProfile | None = None,
) -> CheckpointScore | None:
    """Score finish balance: how far the hip-center drifts from its own mean through follow-through.

    A balanced swing settles into a held finish (small drift); an off-balance one keeps
    staggering. Measured in shoulder-widths for scale-invariance. Returns `None` if there are
    too few confident follow-through frames or the store has no band.
    """
    follow_through = _phase_bounds(phases, SwingPhase.FOLLOW_THROUGH)
    address = _phase_bounds(phases, SwingPhase.ADDRESS)
    if follow_through is None or address is None:
        return None

    points = _hip_center_points(keypoints, follow_through[0], follow_through[1])
    width = _shoulder_width(keypoints, address[0], address[1])
    if len(points) < _MIN_FINISH_FRAMES or width is None:
        return None

    mean_x = sum(x for x, _ in points) / len(points)
    mean_y = sum(y for _, y in points) / len(points)
    max_drift = max(((x - mean_x) ** 2 + (y - mean_y) ** 2) ** 0.5 for x, y in points)
    observed = max_drift / width

    band = resolve_range(_FINISH_BALANCE_RANGE_KEY, club, profile)
    if band is None:
        return None

    score = _score_within_range(observed, band.low, band.high)
    passed = band.low <= observed <= band.high
    if passed:
        message = (
            f"Balanced finish - the body settles within {observed:.2f} shoulder-widths through "
            "the follow-through (held and steady)."
        )
    else:
        message = (
            f"Unbalanced finish - {observed:.2f} shoulder-widths of drift after impact. "
            f"Swing to a held, balanced finish (aim under {band.high})."
        )

    return CheckpointScore(
        name=_FINISH_BALANCE_CHECKPOINT,
        score=score,
        passed=passed,
        observed=round(observed, 2),
        expected_low=band.low,
        expected_high=band.high,
        message=message,
    )
