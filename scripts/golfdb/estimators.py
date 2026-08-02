"""Uniform pose-estimator interface for the bake-off. [M4-REF Phase B0]

Every candidate is wrapped as `frames -> list[FrameKeypoints]` so the bake-off can push all of them
through the *same* `smooth_keypoints` → `segment_phases` → checkpoint path and compare like with
like. Nothing here is imported by the package; this is bake-off scaffolding.

**On the RTMPose adapter.** RTMPose emits 17 COCO keypoints, not MediaPipe's 33, and adopting it
for real would be a contract change (`PoseLandmark`, `NUM_POSE_LANDMARKS`, the overlay skeleton,
every landmark reference in `mechanics.py`). For *measurement* we don't need that: COCO-17 happens
to contain every landmark our three checkpoints read — wrists, ears, shoulders, hips, nose — so we
map those into the 33-slot layout and leave the rest at `visibility=0.0`. The result is a valid
`FrameKeypoints` for our metrics and deliberately **not** a general-purpose MediaPipe substitute.
`estimate_pose` never sees it; only the bake-off does.
"""

from __future__ import annotations

from collections.abc import Callable, Sequence
from pathlib import Path

from golf_coach.capture.source import Frame
from golf_coach.config import settings
from golf_coach.contracts.keypoints import NUM_POSE_LANDMARKS, FrameKeypoints, Landmark
from golf_coach.contracts.keypoints import PoseLandmark as PL

Estimator = Callable[[Sequence[Frame]], list[FrameKeypoints]]

# --- MediaPipe -----------------------------------------------------------------------------

_MEDIAPIPE_VARIANTS = ("lite", "full", "heavy")
_MEDIAPIPE_URL = (
    "https://storage.googleapis.com/mediapipe-models/pose_landmarker/"
    "pose_landmarker_{v}/float16/latest/pose_landmarker_{v}.task"
)


def ensure_mediapipe_model(variant: str) -> Path:
    """Local path to a pose-landmarker bundle, downloading it once if missing."""
    if variant not in _MEDIAPIPE_VARIANTS:
        raise ValueError(f"unknown MediaPipe variant {variant!r}")
    import urllib.request

    settings.models_dir.mkdir(parents=True, exist_ok=True)
    path = settings.models_dir / f"pose_landmarker_{variant}.task"
    if not path.exists():
        urllib.request.urlretrieve(_MEDIAPIPE_URL.format(v=variant), path)  # trusted https
    return path


def mediapipe_estimator(variant: str) -> Estimator:
    """Our production path, parameterized by model size.

    Literally `estimate_pose` with an explicit bundle, so the bake-off exercises the real code path
    rather than a reimplementation of it.
    """
    from golf_coach.pose.estimator import estimate_pose

    model_path = ensure_mediapipe_model(variant)

    def run(frames: Sequence[Frame]) -> list[FrameKeypoints]:
        return estimate_pose(frames, model_path=model_path)

    return run


# --- RTMPose (via rtmlib) ------------------------------------------------------------------

# COCO-17 index -> our PoseLandmark slot. Only the landmarks our checkpoints actually read.
_COCO17_TO_POSE: dict[int, PL] = {
    0: PL.NOSE,
    3: PL.LEFT_EAR,
    4: PL.RIGHT_EAR,
    5: PL.LEFT_SHOULDER,
    6: PL.RIGHT_SHOULDER,
    9: PL.LEFT_WRIST,
    10: PL.RIGHT_WRIST,
    11: PL.LEFT_HIP,
    12: PL.RIGHT_HIP,
}

# rtmlib returns per-keypoint confidence on the same scale we treat as visibility. Below this we
# leave the slot empty rather than pass a guess through as a measurement.
_RTM_MIN_SCORE = 0.3

# RTMPose-m ONNX (body7, 256x192), the pose half of rtmlib's `balanced` tier. Pinned here rather
# than reached through `Body` so the person detector can be skipped — see `rtmpose_estimator`.
_RTMPOSE_M_URL = (
    "https://download.openmmlab.com/mmpose/v1/projects/rtmposev1/onnx_sdk/"
    "rtmpose-m_simcc-body7_pt-body7_420e-256x192-e48f03d0_20230504.zip"
)
_RTMPOSE_M_INPUT_SIZE = (192, 256)


def rtmpose_estimator() -> Estimator:
    """RTMPose-m via `rtmlib` — Apache-2.0, onnxruntime only (no mmcv, no mmpose, no torch).

    **Runs the pose model directly, with no person detector.** rtmlib's `Body` wrapper pairs
    RTMPose-m with a YOLOX-m detector re-run on every frame, which measured **3.1 fps** here against
    MediaPipe lite's ~106 — a 35x gap that is almost entirely the detector, not the pose model.
    Calling `RTMPose` with no `bboxes` makes it treat the whole frame as the box, which measured
    **66.3 fps (21.6x faster)** with the keypoints agreeing to a median 0.84 px out of 160.

    This is sound *for this corpus specifically*: `videos_160` clips are GolfDB's own bounding-box
    crops around the golfer, resized to square, so the full frame already **is** the person box and
    the detector was re-deriving a constant. It also removes a confound — detector jitter would have
    been scored as pose jitter.

    It does **not** generalize to full-frame video. If RTMPose were ever adopted for production, the
    user's own phone clips are not pre-cropped and would need a detector (or a bbox tracked once and
    reused). Recorded in docs/M4_POSE_BAKEOFF.md as a condition on the measurement.
    """
    import numpy as np
    from rtmlib import RTMPose

    pose = RTMPose(
        onnx_model=_RTMPOSE_M_URL,
        model_input_size=_RTMPOSE_M_INPUT_SIZE,
        backend="onnxruntime",
        device="cpu",
    )

    def run(frames: Sequence[Frame]) -> list[FrameKeypoints]:
        results: list[FrameKeypoints] = []
        for frame in frames:
            height, width = frame.image.shape[:2]
            keypoints, scores = pose(frame.image)
            landmarks = [
                Landmark(x=0.0, y=0.0, z=0.0, visibility=0.0) for _ in range(NUM_POSE_LANDMARKS)
            ]
            if len(keypoints):
                # Highest mean-confidence detection = the golfer, not a spectator.
                best = int(np.argmax(scores.mean(axis=1)))
                person, person_scores = keypoints[best], scores[best]
                for coco_index, slot in _COCO17_TO_POSE.items():
                    score = float(person_scores[coco_index])
                    if score < _RTM_MIN_SCORE:
                        continue
                    x, y = person[coco_index]
                    landmarks[slot] = Landmark(
                        # rtmlib returns pixels; our contract is normalized to the frame.
                        x=float(x) / width,
                        y=float(y) / height,
                        z=0.0,
                        # RTMPose emits a raw keypoint score, not a probability — it runs slightly
                        # past 1.0 — whereas `Landmark.visibility` is bounded [0, 1]. Clamp, the
                        # same way `pose/estimator.py` defensively clamps MediaPipe's.
                        visibility=min(1.0, max(0.0, score)),
                    )
            results.append(
                FrameKeypoints(
                    frame_index=frame.index,
                    timestamp_ms=frame.timestamp_ms,
                    landmarks=landmarks,
                )
            )
        return results

    return run


# --- registry ------------------------------------------------------------------------------

CANDIDATES: dict[str, Callable[[], Estimator]] = {
    "mediapipe:lite": lambda: mediapipe_estimator("lite"),
    "mediapipe:full": lambda: mediapipe_estimator("full"),
    "mediapipe:heavy": lambda: mediapipe_estimator("heavy"),
    "rtmpose:m": rtmpose_estimator,
}
