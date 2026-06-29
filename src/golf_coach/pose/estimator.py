"""MediaPipe pose estimation. [M1]

Runs MediaPipe Pose over captured frames and maps its raw output into our `FrameKeypoints`
contract, so nothing downstream ever imports MediaPipe (ADR-008). Requires the `vision`
extra (`pip install -e '.[vision]'`).

Uses MediaPipe's **Tasks API** (`PoseLandmarker`): recent mediapipe releases (0.10.3x)
removed the legacy `mp.solutions` API, leaving only Tasks. The Tasks API needs a model
asset (`.task`), which we fetch once into `data/models/` (see `_ensure_model`).

The heavy MediaPipe call lives in `estimate_pose`; the raw-landmark -> contract mapping is
isolated in the pure `_to_frame_keypoints` helper so it can be unit-tested without the ML
stack.
"""

from __future__ import annotations

import urllib.request
from collections.abc import Iterable, Sequence
from pathlib import Path
from typing import Protocol

from golf_coach.capture.source import Frame
from golf_coach.config import settings
from golf_coach.contracts.keypoints import NUM_POSE_LANDMARKS, FrameKeypoints, Landmark

# Lite pose-landmarker bundle (~5 MB). Fast and accurate enough for the M1 skeleton PoC;
# swap to the full/heavy variant later if accuracy through impact needs it.
_MODEL_FILENAME = "pose_landmarker_lite.task"
_MODEL_URL = (
    "https://storage.googleapis.com/mediapipe-models/pose_landmarker/"
    "pose_landmarker_lite/float16/latest/pose_landmarker_lite.task"
)


def _ensure_model(models_dir: Path) -> Path:
    """Return the local path to the pose model, downloading it once if missing."""
    models_dir.mkdir(parents=True, exist_ok=True)
    model_path = models_dir / _MODEL_FILENAME
    if not model_path.exists():
        urllib.request.urlretrieve(_MODEL_URL, model_path)  # trusted https model bundle
    return model_path


class _RawLandmark(Protocol):
    """Structural type for a single MediaPipe landmark (x, y, z, visibility)."""

    x: float
    y: float
    z: float
    visibility: float


def _clamp_visibility(value: float) -> float:
    """MediaPipe visibility is nominally [0, 1]; clamp defensively for the contract."""
    return min(1.0, max(0.0, value))


def _to_frame_keypoints(
    raw: Sequence[_RawLandmark] | None,
    frame_index: int,
    timestamp_ms: float,
) -> FrameKeypoints:
    """Map MediaPipe's per-frame landmarks into our contract.

    When MediaPipe finds no body (``raw is None``), emit 33 placeholder landmarks at
    visibility 0 so there is exactly one record per frame and the timeline stays aligned
    for downstream phase segmentation (M4-PoC).
    """
    if raw is None:
        landmarks = [
            Landmark(x=0.0, y=0.0, z=0.0, visibility=0.0) for _ in range(NUM_POSE_LANDMARKS)
        ]
    else:
        landmarks = [
            Landmark(x=lm.x, y=lm.y, z=lm.z, visibility=_clamp_visibility(lm.visibility))
            for lm in raw
        ]
    return FrameKeypoints(
        frame_index=frame_index,
        timestamp_ms=timestamp_ms,
        landmarks=landmarks,
    )


def estimate_pose(
    frames: Iterable[Frame], model_path: str | Path | None = None
) -> list[FrameKeypoints]:
    """Run MediaPipe Pose over frames and return one FrameKeypoints per frame.

    `model_path` defaults to the bundle under `settings.models_dir` (downloaded on first
    use); pass an explicit path to override.
    """
    import cv2
    import mediapipe as mp
    from mediapipe.tasks import python as mp_python
    from mediapipe.tasks.python import vision

    resolved = Path(model_path) if model_path is not None else _ensure_model(settings.models_dir)

    options = vision.PoseLandmarkerOptions(
        base_options=mp_python.BaseOptions(model_asset_path=str(resolved)),
        running_mode=vision.RunningMode.VIDEO,
    )

    results: list[FrameKeypoints] = []
    last_ts = -1
    with vision.PoseLandmarker.create_from_options(options) as landmarker:
        for frame in frames:
            rgb = cv2.cvtColor(frame.image, cv2.COLOR_BGR2RGB)
            mp_image = mp.Image(image_format=mp.ImageFormat.SRGB, data=rgb)
            # detect_for_video needs strictly-increasing int ms timestamps.
            ts = max(last_ts + 1, int(frame.timestamp_ms))
            last_ts = ts
            detected = landmarker.detect_for_video(mp_image, ts)
            raw = detected.pose_landmarks[0] if detected.pose_landmarks else None
            results.append(_to_frame_keypoints(raw, frame.index, frame.timestamp_ms))
    return results
