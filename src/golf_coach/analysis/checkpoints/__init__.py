"""Swing checkpoints — mechanics (pose) now, outcome (shot-vs-intent) in full M4. [M4-PoC+]

Ships three pose-only **mechanics** checkpoints (`mechanics.py`): tempo, head sway, and finish
balance. `outcome.py` is intentionally absent until M2/M3 bring club detection and
launch-monitor data online — this package is the named seam where it will land (ADR-009
§Contract shape).
"""

from golf_coach.analysis.checkpoints.mechanics import (
    evaluate_finish_balance,
    evaluate_head_sway,
    evaluate_tempo,
)

__all__ = ["evaluate_tempo", "evaluate_head_sway", "evaluate_finish_balance"]
