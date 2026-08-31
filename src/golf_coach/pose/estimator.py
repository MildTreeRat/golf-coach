"""MediaPipe pose estimation. [M1]

Runs MediaPipe Pose over captured frames and maps its raw output into our `FrameKeypoints`
contract, so nothing downstream ever imports MediaPipe (ADR-008). Requires the `vision`
extra (`pip install -e '.[vision]'`).

Uses MediaPipe's **Tasks API** (`PoseLandmarker`): recent mediapipe releases (0.10.3x)
removed the legacy `mp.solutions` API, leaving only Tasks. The Tasks API needs a model
asset (`.task`), which we fetch once into `data/models/` (see `ensure_pose_model`).

**Which bundle runs is configuration, not a constant** — `settings.pose_model_variant`, one of
`POSE_VARIANTS`. `pose_estimator_name` is how the rest of the system says which one measured a
given artifact; `api/pipeline.py` compares that name against a cached pose run before reusing it,
for the reason `contracts/audio.py` gives about its detector version.

The heavy MediaPipe call lives in `estimate_pose`; the raw-landmark -> contract mapping is
isolated in the pure `_to_frame_keypoints` helper so it can be unit-tested without the ML
stack.
"""

from __future__ import annotations

import urllib.request
from collections.abc import Iterable, Sequence
from pathlib import Path
from typing import TYPE_CHECKING, Protocol

from golf_coach.config import settings
from golf_coach.contracts.keypoints import NUM_POSE_LANDMARKS, FrameKeypoints, Landmark

if TYPE_CHECKING:
    # Type-only: `Frame` carries a numpy image, so it cannot live in `contracts/` (see
    # `capture/source.py`). Importing it at runtime would make this module's cheapness depend on
    # `capture` never hoisting its own numpy import — which R2 explicitly allows it to do.
    from golf_coach.capture.source import Frame

#: The three pose-landmarker bundles Google publishes, cheapest first: lite (~5 MB), full (~9 MB),
#: heavy (~30 MB). Which one runs is `settings.pose_model_variant`, not a constant here, because
#: it is an operational choice the bay makes and re-makes — see that setting for why it is heavy
#: and what the bake-off measured.
POSE_VARIANTS = ("lite", "full", "heavy")

_MODEL_URL = (
    "https://storage.googleapis.com/mediapipe-models/pose_landmarker/"
    "pose_landmarker_{variant}/float16/latest/pose_landmarker_{variant}.task"
)


def resolve_variant(variant: str | None = None) -> str:
    """The variant to run: the argument, else `settings.pose_model_variant`. Validated here.

    Validated rather than passed through, because an unknown name would otherwise surface as a
    404 inside `urlretrieve` on the first swing of a session — long after the typo, and phrased
    as a network failure rather than as a bad setting.
    """
    resolved = variant if variant is not None else settings.pose_model_variant
    if resolved not in POSE_VARIANTS:
        raise ValueError(
            f"unknown MediaPipe pose variant {resolved!r}; expected one of {POSE_VARIANTS}"
        )
    return resolved


def pose_estimator_name(variant: str | None = None) -> str:
    """`"mediapipe:heavy"` — the name a stored artifact records the measuring instrument by.

    Deliberately the same vocabulary as `scripts/golfdb/estimators.py`'s registry keys and
    `benchmarks/golfdb_v1.json`'s `dataset.pose_estimator`, so a cached pose run, a bake-off row
    and a band's provenance can be compared as strings without a translation table between them.
    """
    return f"mediapipe:{resolve_variant(variant)}"


def model_filename(variant: str | None = None) -> str:
    """Bundle filename for a variant. One per variant, so switching does not re-download."""
    return f"pose_landmarker_{resolve_variant(variant)}.task"


def ensure_pose_model(models_dir: Path, variant: str | None = None) -> Path:
    """Return the local path to the pose model, downloading it once if missing.

    Downloads to a temp path and renames, the same way `api/state.py` and the `storage/` writers
    do. Fetching straight to `model_path` meant a Ctrl-C or a dropped connection mid-download left
    a truncated `.task` that the `exists()` check below accepts forever after — every later
    `estimate_pose` then failing inside MediaPipe with an opaque model-parse error that nothing
    connects back to the interrupted download.

    Each variant caches under its own filename, so moving `settings.pose_model_variant` back and
    forth costs one download per variant and not one per switch.
    """
    models_dir.mkdir(parents=True, exist_ok=True)
    resolved = resolve_variant(variant)
    model_path = models_dir / model_filename(resolved)
    if not model_path.exists():
        tmp = model_path.with_suffix(".part")
        try:
            url = _MODEL_URL.format(variant=resolved)
            urllib.request.urlretrieve(url, tmp)  # trusted https model bundle
            tmp.replace(model_path)
        finally:
            tmp.unlink(missing_ok=True)
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
    camera_id: str | None = None,
) -> FrameKeypoints:
    """Map MediaPipe's per-frame landmarks into our contract.

    When MediaPipe finds no body (``raw is None``), emit 33 placeholder landmarks at
    visibility 0 so there is exactly one record per frame and the timeline stays aligned
    for downstream phase segmentation (M4-PoC).

    `camera_id` rides along from the capturing `Frame` so a keypoints list stays attributable to
    a view after the pixels are gone (ADR-011).
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
        camera_id=camera_id,
    )


def estimate_pose(
    frames: Iterable[Frame],
    model_path: str | Path | None = None,
    variant: str | None = None,
) -> list[FrameKeypoints]:
    """Run MediaPipe Pose over frames and return one FrameKeypoints per frame.

    `frames` is consumed **lazily**, one at a time — pass `source.frames()` straight in and no
    decoded frame outlives its iteration. That is what keeps a 4K clip from needing gigabytes;
    the returned keypoints are small (33 landmarks/frame) and are the only thing accumulated.

    `model_path` defaults to the bundle for `variant` under `settings.models_dir` (downloaded on
    first use); pass an explicit path to override. `variant` defaults to
    `settings.pose_model_variant` — the bake-off (`scripts/golfdb/estimators.py`) is the caller
    that names one, because comparing variants is the whole point of it.
    """
    import cv2
    import mediapipe as mp
    from mediapipe.tasks import python as mp_python
    from mediapipe.tasks.python import vision

    resolved = (
        Path(model_path)
        if model_path is not None
        else ensure_pose_model(settings.models_dir, variant)
    )

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
            results.append(
                _to_frame_keypoints(raw, frame.index, frame.timestamp_ms, frame.camera_id)
            )
    return results
