"""Swing checkpoints — mechanics (pose) now, outcome (shot-vs-intent) in full M4. [M4-PoC+]

Ships six pose-only **mechanics** checkpoints (`mechanics.py`): tempo, head sway, finish balance,
hip sway, hip shift at the top, and head stays back. `outcome.py` is intentionally absent until
M2/M3 bring club detection and launch-monitor data online — this package is the named seam where it
will land (ADR-009 §Contract shape).

`head_stays_back` is the only one that needs to know *who* swung: its sign is camera-relative, so
it takes a `Handedness` and returns `None` without one. See `mechanics.evaluate_head_stays_back`.
"""

from golf_coach.analysis.checkpoints.mechanics import (
    FINISH_BALANCE_CHECKPOINT,
    HEAD_STAYS_BACK_CHECKPOINT,
    HEAD_SWAY_CHECKPOINT,
    HIP_SHIFT_AT_TOP_CHECKPOINT,
    HIP_SWAY_CHECKPOINT,
    TEMPO_CHECKPOINT,
    evaluate_finish_balance,
    evaluate_head_stays_back,
    evaluate_head_sway,
    evaluate_hip_shift_at_top,
    evaluate_hip_sway,
    evaluate_tempo,
)

# The name constants are public because an evaluator returning `None` still has to be *named* —
# `SwingResult.unscored` reports which checkpoints could not be measured, and the engine should not
# be re-typing those strings as literals next to the calls that produce them.
__all__ = [
    "FINISH_BALANCE_CHECKPOINT",
    "HEAD_STAYS_BACK_CHECKPOINT",
    "HEAD_SWAY_CHECKPOINT",
    "HIP_SHIFT_AT_TOP_CHECKPOINT",
    "HIP_SWAY_CHECKPOINT",
    "TEMPO_CHECKPOINT",
    "evaluate_finish_balance",
    "evaluate_head_stays_back",
    "evaluate_head_sway",
    "evaluate_hip_shift_at_top",
    "evaluate_hip_sway",
    "evaluate_tempo",
]
