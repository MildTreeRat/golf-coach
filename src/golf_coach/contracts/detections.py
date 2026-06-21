"""Club/ball detection contracts (produced by the `detection` module).

The `detection` module runs YOLOv8 + a tracker and maps the raw output into these
shapes. Consumers (analysis, overlays) depend only on this — never on Ultralytics.
"""

from __future__ import annotations

from enum import StrEnum

from pydantic import BaseModel, Field


class ObjectClass(StrEnum):
    """The objects we train YOLOv8 to detect (ADR-005)."""

    CLUB_HEAD = "club_head"
    BALL = "ball"


class BoundingBox(BaseModel):
    """Axis-aligned box in normalized image coordinates (xyxy, each in [0, 1])."""

    x1: float = Field(ge=0.0, le=1.0)
    y1: float = Field(ge=0.0, le=1.0)
    x2: float = Field(ge=0.0, le=1.0)
    y2: float = Field(ge=0.0, le=1.0)

    @property
    def center(self) -> tuple[float, float]:
        """Box center — this is the point we string together into the club-path arc."""
        return ((self.x1 + self.x2) / 2.0, (self.y1 + self.y2) / 2.0)


class Detection(BaseModel):
    """A single detected object in a frame."""

    object_class: ObjectClass
    bbox: BoundingBox
    confidence: float = Field(ge=0.0, le=1.0)
    # Assigned by the tracker (ByteTrack); links the same object across frames so we
    # can connect club-head centers into a path. None before tracking is applied.
    track_id: int | None = None


class FrameDetections(BaseModel):
    """All detections for one video frame."""

    frame_index: int = Field(ge=0)
    timestamp_ms: float = Field(ge=0.0)
    detections: list[Detection] = Field(default_factory=list)

    def of(self, object_class: ObjectClass) -> list[Detection]:
        """All detections of a given class in this frame."""
        return [d for d in self.detections if d.object_class == object_class]
