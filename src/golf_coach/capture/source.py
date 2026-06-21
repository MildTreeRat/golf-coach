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
    """One captured frame plus its timing metadata."""

    index: int
    timestamp_ms: float
    image: np.ndarray[Any, Any]  # BGR HxWx3 (annotation only; numpy not imported at runtime)


@runtime_checkable
class VideoSource(Protocol):
    """A source of video frames. Implementations are context managers."""

    def __enter__(self) -> VideoSource: ...
    def __exit__(self, *exc: object) -> None: ...

    @property
    def fps(self) -> float: ...

    def frames(self) -> Iterator[Frame]:
        """Yield frames in order until the source is exhausted."""
        ...
