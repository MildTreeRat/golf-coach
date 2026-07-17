"""Landmark temporal smoothing — pure, stdlib only. [M4-PoC+]

MediaPipe landmark trajectories jitter frame-to-frame (see M1 findings), and the phase
instants tempo depends on — the top of the backswing (global wrist-`y` minimum) and impact —
are read straight off those trajectories, so a few pixels of noise can move the detected
"top" by several frames and skew the tempo ratio. This module denoises the timeline *once*,
up front, so both `phases.segment_phases` and the mechanics checkpoints read a stable signal.

The filter is a small **centered moving average, visibility-weighted**: each smoothed
coordinate is the mean of its neighbours within a half-window, weighting every sample by its
MediaPipe `visibility` so a low-confidence landmark contributes little. Only `x`/`y` (the
image-plane coordinates every consumer uses) are denoised; `z` and `visibility` pass through
unchanged. It is deliberately the simplest thing that works — no numpy, no Savitzky-Golay, no
Kalman tracker (YAGNI for a pose-only PoC) — so the analysis core stays stdlib-only (ADR-008).

HARDWARE-REVALIDATE: window size / weighting are tuned by eye on 60fps phone clips. Revisit
against global-shutter, higher-fps capture and (ADR-011) synced multi-view when hardware
lands — faster capture may want a wider window; sharper landmarks may want less smoothing.
"""

from __future__ import annotations

from golf_coach.contracts.keypoints import (
    NUM_POSE_LANDMARKS,
    FrameKeypoints,
    Landmark,
)

# Total window (frames) of the centered moving average. Odd so it is symmetric about the
# frame being smoothed; 5 → two neighbours each side. Small: enough to kill single-frame
# jitter without blurring the genuine motion the swing traces in ~10-40 frames.
_DEFAULT_WINDOW = 5

# A sample dimmer than this still counts, but a window whose *total* weight falls below this
# is treated as "no confident samples" and we keep the raw coordinate rather than invent one.
_MIN_TOTAL_WEIGHT = 1e-6


def smooth_keypoints(
    keypoints: list[FrameKeypoints], window: int = _DEFAULT_WINDOW
) -> list[FrameKeypoints]:
    """Return a new, temporally smoothed copy of a keypoint timeline.

    Applies a visibility-weighted centered moving average to each landmark's `x`/`y` across a
    `window`-frame span (clamped at the clip ends). `frame_index`, `timestamp_ms`, and each
    landmark's `z`/`visibility` are preserved exactly. Input is not mutated. A `window <= 1`
    (or a timeline too short to smooth) returns an equivalent copy unchanged.
    """
    n = len(keypoints)
    if n == 0 or window <= 1:
        return [frame.model_copy(deep=True) for frame in keypoints]

    half = window // 2
    smoothed: list[FrameKeypoints] = []
    for i, frame in enumerate(keypoints):
        lo = max(0, i - half)
        hi = min(n, i + half + 1)
        new_landmarks: list[Landmark] = []
        for lm_index in range(NUM_POSE_LANDMARKS):
            original = frame.landmarks[lm_index]
            sx = sy = weight = 0.0
            for j in range(lo, hi):
                sample = keypoints[j].landmarks[lm_index]
                w = sample.visibility
                sx += sample.x * w
                sy += sample.y * w
                weight += w
            if weight < _MIN_TOTAL_WEIGHT:
                # No confident neighbours — trust the raw value over a fabricated average.
                new_landmarks.append(original.model_copy())
            else:
                new_landmarks.append(
                    Landmark(
                        x=sx / weight,
                        y=sy / weight,
                        z=original.z,
                        visibility=original.visibility,
                    )
                )
        smoothed.append(
            FrameKeypoints(
                frame_index=frame.frame_index,
                timestamp_ms=frame.timestamp_ms,
                landmarks=new_landmarks,
            )
        )
    return smoothed
