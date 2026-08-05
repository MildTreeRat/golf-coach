"""Swing checkpoints — mechanics (pose) now, outcome (shot-vs-intent) in full M4. [M4-PoC+]

Ships three pose-only **mechanics** checkpoints (`mechanics.py`): tempo, head sway, and finish
balance. `outcome.py` is intentionally absent until M2/M3 bring club detection and
launch-monitor data online — this package is the named seam where it will land (ADR-009
§Contract shape).
"""

from golf_coach.analysis.checkpoints.mechanics import (
    FINISH_BALANCE_CHECKPOINT,
    HEAD_SWAY_CHECKPOINT,
    TEMPO_CHECKPOINT,
    evaluate_finish_balance,
    evaluate_head_sway,
    evaluate_tempo,
)

# The name constants are public because an evaluator returning `None` still has to be *named* —
# `SwingResult.unscored` reports which checkpoints could not be measured, and the engine should not
# be re-typing those strings as literals next to the calls that produce them.
__all__ = [
    "FINISH_BALANCE_CHECKPOINT",
    "HEAD_SWAY_CHECKPOINT",
    "TEMPO_CHECKPOINT",
    "evaluate_finish_balance",
    "evaluate_head_sway",
    "evaluate_tempo",
]
