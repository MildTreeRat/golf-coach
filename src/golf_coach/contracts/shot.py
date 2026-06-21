"""Launch-monitor shot data contract (produced by the `launch_monitor` module).

This is the single shape both the analysis engine and Claude (via MCP) consume.
Critically, it is identical whether the data came from the real Garmin R10 or the
mock source — the `source` field is the only thing that differs (ADR-007). Swapping
hardware only changes which adapter fills this in, never the consumers (ADR-006).

Field set and units come from ADR-004 (R10 schema). Every metric is optional because
not all sources/clubs report every field.
"""

from __future__ import annotations

from datetime import datetime
from enum import StrEnum

from pydantic import BaseModel, Field


class ShotSource(StrEnum):
    """Where a ShotData record came from — for provenance and debugging."""

    MOCK = "mock"
    R10 = "r10"
    IMPORT = "import"  # bulk export / historical JSON


class ShotData(BaseModel):
    """Metrics for a single shot. Units in field descriptions (ADR-004)."""

    shot_id: str
    session_id: str
    timestamp: datetime
    source: ShotSource = ShotSource.MOCK

    # Club metrics — what the camera-observed mechanics should correlate with.
    club_head_speed: float | None = Field(default=None, description="mph")
    club_face_angle: float | None = Field(default=None, description="degrees, + = open")
    club_path: float | None = Field(default=None, description="degrees, + = in-to-out")

    # Ball metrics.
    ball_speed: float | None = Field(default=None, description="mph")
    launch_angle: float | None = Field(default=None, description="degrees, vertical")
    launch_direction: float | None = Field(default=None, description="degrees, + = right")
    spin_rate: float | None = Field(default=None, description="rpm")
    spin_axis: float | None = Field(default=None, description="degrees, + = fade")
    smash_factor: float | None = Field(default=None, description="ball_speed / club_head_speed")

    # Flight estimates.
    carry_distance: float | None = Field(default=None, description="yards")
    total_distance: float | None = Field(default=None, description="yards")
    apex_height: float | None = Field(default=None, description="yards")
