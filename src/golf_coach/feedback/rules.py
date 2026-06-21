"""Rule-based feedback. [M5]

Maps checkpoint scores in a SwingResult to plain-English tips. Pure function, no I/O.
"""

from __future__ import annotations

from golf_coach.contracts.feedback import FeedbackPayload
from golf_coach.contracts.swing import SwingResult


def build_feedback(result: SwingResult) -> FeedbackPayload:
    """Produce rule-based tips from a swing result (LLM coaching added in M6)."""
    raise NotImplementedError("M5: implement rule-based feedback.")
