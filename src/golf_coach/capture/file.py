"""FileVideoSource: read frames from a video file on disk. [M1]

A `VideoSource` adapter over `cv2.VideoCapture`. This is the hardware-free way to feed the
pipeline today (ADR-007) — point it at a phone/sample swing clip. The live ELP camera
adapter (capture/camera.py) will implement the same `VideoSource` port later, so nothing
downstream changes when we swap to real hardware.

Requires the `vision` extra (`pip install -e '.[vision]'`) for OpenCV, which is why this
adapter is imported directly (`from golf_coach.capture.file import FileVideoSource`) rather
than re-exported from the package __init__ — importing the port stays dependency-free.
"""

from __future__ import annotations

from collections.abc import Iterator
from pathlib import Path

import cv2

from golf_coach.capture.source import Frame

# Fallback when a container reports an unusable fps (0 or NaN). 30fps matches the typical
# phone clip we bootstrap M1 with (ROADMAP M1).
_DEFAULT_FPS = 30.0


class FileVideoSource:
    """Yields frames from a video file in order. Implements the `VideoSource` port.

    Use as a context manager so the underlying capture handle is always released::

        with FileVideoSource("data/raw/swing.mp4") as src:
            for frame in src.frames():
                ...
    """

    def __init__(self, path: str | Path, *, camera_id: str | None = None) -> None:
        self._path = Path(path)
        self._camera_id = camera_id
        self._cap: cv2.VideoCapture | None = None

    def __enter__(self) -> FileVideoSource:
        if not self._path.exists():
            raise FileNotFoundError(f"Video file not found: {self._path}")
        cap = cv2.VideoCapture(str(self._path))
        if not cap.isOpened():
            raise OSError(f"OpenCV could not open video: {self._path}")
        self._cap = cap
        return self

    def __exit__(self, *exc: object) -> None:
        if self._cap is not None:
            self._cap.release()
            self._cap = None

    @property
    def camera_id(self) -> str | None:
        """Which camera this file came from, or None if the caller didn't say (ADR-011)."""
        return self._camera_id

    @property
    def fps(self) -> float:
        """Frames per second reported by the container, with a sane fallback."""
        cap = self._require_cap()
        fps = cap.get(cv2.CAP_PROP_FPS)
        # Some containers report 0 or NaN; fall back so timestamps stay monotonic.
        if not fps or fps != fps or fps <= 0:  # `fps != fps` catches NaN
            return _DEFAULT_FPS
        return float(fps)

    @property
    def width(self) -> int:
        """Frame width in pixels as the container reports it; 0 when it won't say."""
        return self._prop(cv2.CAP_PROP_FRAME_WIDTH)

    @property
    def height(self) -> int:
        """Frame height in pixels as the container reports it; 0 when it won't say."""
        return self._prop(cv2.CAP_PROP_FRAME_HEIGHT)

    @property
    def frame_count(self) -> int:
        """The container's *claim* about how many frames it holds; 0 when it won't say.

        Deliberately not the same thing as the number of frames `frames()` yields, and not part of
        the `VideoSource` port — a live camera has no length. Treat a disagreement between the two
        as a signal: a file that opens cleanly and then decodes short is the silent failure mode
        (docs/M7_TWO_PHONE_SPIKE.md, Q2 PARTIAL), so callers that care should compare and warn.
        """
        return self._prop(cv2.CAP_PROP_FRAME_COUNT)

    def frames(self) -> Iterator[Frame]:
        """Yield frames in order until the file is exhausted."""
        cap = self._require_cap()
        fps = self.fps
        camera_id = self._camera_id
        index = 0
        while True:
            ok, image = cap.read()
            if not ok:
                break
            yield Frame(
                index=index,
                timestamp_ms=index / fps * 1000.0,
                image=image,
                camera_id=camera_id,
            )
            index += 1

    def _prop(self, prop: int) -> int:
        """A container property as a non-negative int; unreported values (0, -1, NaN) become 0."""
        value = self._require_cap().get(prop)
        if not value or value != value or value < 0:  # `value != value` catches NaN
            return 0
        return int(value)

    def _require_cap(self) -> cv2.VideoCapture:
        if self._cap is None:
            raise RuntimeError(
                "FileVideoSource must be used as a context manager (use `with`)."
            )
        return self._cap
