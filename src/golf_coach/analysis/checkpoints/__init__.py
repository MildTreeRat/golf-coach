"""Swing checkpoints — mechanics (pose) now, outcome (shot-vs-intent) in full M4. [M4-PoC]

The PoC ships the single **tempo** mechanics checkpoint (`mechanics.py`). `outcome.py` is
intentionally absent until M2/M3 bring club detection and launch-monitor data online — this
package is the named seam where it will land (ADR-009 §Contract shape).
"""

from golf_coach.analysis.checkpoints.mechanics import evaluate_tempo

__all__ = ["evaluate_tempo"]
