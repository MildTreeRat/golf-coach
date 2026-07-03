"""Mechanics checkpoints — pose-based, intent-independent. [M4-PoC]

The PoC implements one: **tempo**, the classic backswing:downswing time ratio (~3:1, "Tour
Tempo"). It reads the phase timings produced by `phases.segment_phases`, compares the
observed ratio against the benchmark band resolved from the store (ADR-010), and returns a
`CheckpointScore`. If the store has no band for tempo, it returns `None` — no score beats a
wrong one (ADR-010 §2). Pure function, stdlib only.
"""

from __future__ import annotations

from golf_coach.analysis.benchmarks import resolve_range
from golf_coach.contracts.intent import ClubCategory, PlayerProfile
from golf_coach.contracts.swing import CheckpointScore, PhaseSegment, SwingPhase

_TEMPO_CHECKPOINT = "tempo"
_TEMPO_RANGE_KEY = "tempo_ratio"


def _score_within_range(observed: float, low: float, high: float) -> float:
    """1.0 inside `[low, high]`, decaying linearly (by band-widths) to 0.0 outside.

    Shared by mechanics checkpoints; kept local while tempo is the only caller (extract to a
    checkpoints-wide helper once a second checkpoint needs it).
    """
    if low <= observed <= high:
        return 1.0
    width = high - low
    if width <= 0:
        return 0.0
    distance = (low - observed) if observed < low else (observed - high)
    return max(0.0, 1.0 - distance / width)


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
