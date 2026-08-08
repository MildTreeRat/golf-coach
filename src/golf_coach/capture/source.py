"""VideoSource port: the interface every video source implements.

Adapters:
  - FileVideoSource  (capture/file.py)   — read a sample/phone clip  [M1, today]
  - LiveCameraSource (capture/camera.py) — read the ELP USB camera   [needs hardware]

Frames are yielded as raw numpy arrays (BGR, OpenCV convention). We keep numpy out of
the `contracts` package on purpose — pixels are an implementation detail of the I/O
edge, not a cross-module contract.
"""

from __future__ import annotations

from collections.abc import Iterator
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Protocol, runtime_checkable

if TYPE_CHECKING:
    import numpy as np


@dataclass(frozen=True)
class Frame:
    """One captured frame plus its timing and provenance metadata."""

    index: int
    timestamp_ms: float
    image: np.ndarray[Any, Any]  # BGR HxWx3 (annotation only; numpy not imported at runtime)
    camera_id: str | None = None
    """Which camera this frame came from (ADR-011's Phase 1 seam).

    Each camera is its own `VideoSource` stream, so this is set once per source and stamped on
    every frame it yields. `None` means the source was not told — the honest state for a
    single-camera clip nobody labelled.

    Free-form on purpose. M7 uses the same words as `storage.manifest.Role` ("face_on",
    "down_the_line"), but `capture` must not import `storage`: modules depend on `contracts`
    and never on each other (ADR-008).
    """


@runtime_checkable
class VideoSource(Protocol):
    """A source of video frames. Implementations are context managers."""

    def __enter__(self) -> VideoSource: ...
    def __exit__(self, *exc: object) -> None: ...

    @property
    def fps(self) -> float: ...

    @property
    def camera_id(self) -> str | None: ...

    def frames(self) -> Iterator[Frame]:
        """Yield frames in order until the source is exhausted."""
        ...
