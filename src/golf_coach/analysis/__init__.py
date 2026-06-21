"""Analysis module — the pure functional core. [M4]

Merges keypoints + detections + shot data, segments swing phases, evaluates
checkpoints, and scores the swing. No I/O, no hardware, no network — just contracts
in, `SwingResult` out. This is what makes it trivially testable and runnable on
simulated data before any hardware exists.
"""

from golf_coach.analysis.engine import analyze_swing

__all__ = ["analyze_swing"]
