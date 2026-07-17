"""Swing analysis entry point. [M4-PoC]

Pure function: merged data streams + intent in, `SwingResult` out. Orchestrates the analysis
spine — segment phases → evaluate mechanics checkpoints → combine via the intent's scoring
policy — with no I/O, no hardware, no network. The PoC covers the pose-only Fundamentals path
(tempo checkpoint); the `detections` / `shot` inputs and the `outcome` axis are the named
seams where full M4 adds club detection and launch-monitor scoring (ADR-009).
"""

from __future__ import annotations

from golf_coach.analysis.checkpoints import (
    evaluate_finish_balance,
    evaluate_head_sway,
    evaluate_tempo,
)
from golf_coach.analysis.phases import segment_phases
from golf_coach.analysis.scoring import policy_for
from golf_coach.analysis.smoothing import smooth_keypoints
from golf_coach.contracts.detections import FrameDetections
from golf_coach.contracts.intent import PracticeGoal
from golf_coach.contracts.keypoints import FrameKeypoints
from golf_coach.contracts.shot import ShotData
from golf_coach.contracts.swing import CheckpointScore, SwingResult


def analyze_swing(
    swing_id: str,
    session_id: str,
    keypoints: list[FrameKeypoints],
    detections: list[FrameDetections] | None = None,
    shot: ShotData | None = None,
    intent: PracticeGoal | None = None,
) -> SwingResult:
    """Analyze one swing from its data streams, judged against a practice intent.

    `intent` defaults to Fundamentals (grade mechanics only). Checkpoints that can't be
    scored (e.g. no benchmark band) are dropped, so `overall_score` reflects only what was
    judged.
    """
    intent = intent or PracticeGoal()

    # Denoise once, up front, so phase instants and every checkpoint read a stable signal
    # (raw MediaPipe landmarks jitter frame-to-frame). SwingResult still keeps the raw
    # keypoints below as the source data this result was computed from.
    smoothed = smooth_keypoints(keypoints)
    phases = segment_phases(smoothed)

    mechanics: list[CheckpointScore] = []
    for checkpoint in (
        evaluate_tempo(phases, club=intent.club),
        evaluate_head_sway(smoothed, phases, club=intent.club),
        evaluate_finish_balance(smoothed, phases, club=intent.club),
    ):
        if checkpoint is not None:
            mechanics.append(checkpoint)

    # Pose-only PoC: no outcome checkpoints yet (needs M2 detection / M3 shot data).
    outcome: list[CheckpointScore] = []

    scores = policy_for(intent.mode).combine(mechanics, outcome)

    return SwingResult(
        swing_id=swing_id,
        session_id=session_id,
        phases=phases,
        checkpoint_scores=mechanics + outcome,
        intent=intent,
        mechanics_score=scores.mechanics,
        outcome_score=scores.outcome,
        overall_score=scores.overall,
        keypoints=keypoints,
        detections=detections or [],
        shot=shot,
    )
