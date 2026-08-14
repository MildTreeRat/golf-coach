"""Swing checkpoints — mechanics (pose) now, outcome (shot-vs-intent) in full M4. [M4-PoC+]

Ships the pose-only **mechanics** checkpoints (`mechanics.py`) and binds each to its registered
name in `CHECKPOINT_EVALUATORS`. `outcome.py` is intentionally absent until M2/M3 bring club
detection and launch-monitor data online — this package is the named seam where it will land
(ADR-009 §Contract shape).

Which checkpoints exist is `contracts.checkpoints.CHECKPOINT_REGISTRY`, not a list written out
here. That module is deliberately upstream of this one: `contracts/caveats.py` builds the caveat
prose a model reads out of the same registry, and ADR-008 forbids it importing `analysis`.

`head_stays_back` is the only one that needs to know *who* swung: its sign is camera-relative, so
it takes a `Handedness` and returns `None` without one. See `mechanics.evaluate_head_stays_back`.
"""

from collections.abc import Callable

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
from golf_coach.contracts.golfer import Handedness
from golf_coach.contracts.intent import ClubCategory, PlayerProfile
from golf_coach.contracts.keypoints import FrameKeypoints
from golf_coach.contracts.swing import CheckpointScore, PhaseSegment

#: One shape every checkpoint is called through, whatever it actually reads.
Evaluate = Callable[
    [
        list[FrameKeypoints],
        list[PhaseSegment],
        Handedness | None,
        ClubCategory,
        PlayerProfile | None,
    ],
    CheckpointScore | None,
]

#: Registered name -> the call that scores it, so the engine dispatches instead of listing.
#:
#: The lambdas exist to absorb two evaluators that legitimately don't take the full argument set —
#: `evaluate_tempo` reads phase instants and never touches keypoints, and `evaluate_head_stays_back`
#: is the only one that needs `handedness`. Widening their real signatures to match would put
#: parameters in them that they must then be trusted to ignore; adapting at the registry keeps each
#: function honest about what it reads. `measure.POSE_MEASUREMENTS` absorbs `tempo_ratio` the same
#: way, for the same reason.
#:
#: Keyed by name rather than ordered alongside `CHECKPOINT_REGISTRY` so that a spec added without an
#: evaluator (or vice versa) fails as a `KeyError` naming the checkpoint, rather than as a silently
#: mismatched zip. A test pins the two sides against each other.
CHECKPOINT_EVALUATORS: dict[str, Evaluate] = {
    TEMPO_CHECKPOINT: lambda keypoints, phases, handedness, club, profile: evaluate_tempo(
        phases, club=club, profile=profile
    ),
    HEAD_SWAY_CHECKPOINT: lambda keypoints, phases, handedness, club, profile: evaluate_head_sway(
        keypoints, phases, club=club, profile=profile
    ),
    FINISH_BALANCE_CHECKPOINT: (
        lambda keypoints, phases, handedness, club, profile: evaluate_finish_balance(
            keypoints, phases, club=club, profile=profile
        )
    ),
    HIP_SWAY_CHECKPOINT: lambda keypoints, phases, handedness, club, profile: evaluate_hip_sway(
        keypoints, phases, club=club, profile=profile
    ),
    HIP_SHIFT_AT_TOP_CHECKPOINT: (
        lambda keypoints, phases, handedness, club, profile: evaluate_hip_shift_at_top(
            keypoints, phases, club=club, profile=profile
        )
    ),
    HEAD_STAYS_BACK_CHECKPOINT: (
        lambda keypoints, phases, handedness, club, profile: evaluate_head_stays_back(
            keypoints, phases, handedness, club=club, profile=profile
        )
    ),
}

# The name constants are public because an evaluator returning `None` still has to be *named* —
# `SwingResult.unscored` reports which checkpoints could not be measured, and the engine should not
# be re-typing those strings as literals next to the calls that produce them.
__all__ = [
    "CHECKPOINT_EVALUATORS",
    "FINISH_BALANCE_CHECKPOINT",
    "HEAD_STAYS_BACK_CHECKPOINT",
    "HEAD_SWAY_CHECKPOINT",
    "HIP_SHIFT_AT_TOP_CHECKPOINT",
    "HIP_SWAY_CHECKPOINT",
    "TEMPO_CHECKPOINT",
    "Evaluate",
    "evaluate_finish_balance",
    "evaluate_head_stays_back",
    "evaluate_head_sway",
    "evaluate_hip_shift_at_top",
    "evaluate_hip_sway",
    "evaluate_tempo",
]
