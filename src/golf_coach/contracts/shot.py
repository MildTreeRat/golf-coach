"""Launch-monitor shot data contract (produced by the `launch_monitor` module).

This is the single shape both the analysis engine and Claude (via MCP) consume.
Critically, it is identical whether the data came from the real Garmin R10, a photo of
an HD Golf screen, or the mock source — the `source` field is the only thing that
differs (ADR-007). Swapping hardware only changes which adapter fills this in, never
the consumers (ADR-006).

Field set and units come from ADR-004 (R10 schema). Every metric is optional because
not all sources/clubs report every field — the HD Golf screen, for instance, leaves
spin blank (ADR-014).
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
    SCREEN = "screen"  # parsed from a launch-monitor screen image/frame (ADR-014)


class ShotProvenance(BaseModel):
    """How a shot was obtained, for sources that *infer* rather than *receive* metrics.

    A shot read off a photograph is a measurement of a measurement: OCR can drop a digit
    and produce a number that is wrong but perfectly plausible. Carrying the confidence,
    the physics-check warnings, and the raw on-screen text alongside the metrics is what
    makes a bad parse auditable after the fact instead of silently poisoning a session's
    scores (ADR-014). Sources that receive metrics directly (R10, mock) leave this `None`.
    """

    device: str = Field(description="Device profile that parsed this, e.g. 'hd_golf'.")
    parse_confidence: float = Field(ge=0.0, le=1.0, description="0-1; 1 = fully trusted.")
    needs_review: bool = Field(
        default=False,
        description="True when confidence fell below threshold or a physics check failed.",
    )
    warnings: list[str] = Field(
        default_factory=list, description="Human-readable reasons this parse is suspect."
    )
    image_sha256: str | None = Field(default=None, description="Content hash of the source image.")
    image_path: str | None = Field(default=None, description="Where the image was read from.")
    raw_fields: dict[str, str] = Field(
        default_factory=dict,
        description="On-screen label -> raw recognized text, exactly as read.",
    )


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
    bounce_and_roll: float | None = Field(default=None, description="yards, total - carry")
    apex_height: float | None = Field(default=None, description="yards")

    # Categorical read-outs. Kept as free text rather than enums: these are the source's
    # own vocabulary (HD Golf says "SLIGHT DRAW", the R10 will say something else), and
    # normalizing them belongs in analysis, not at the ingest boundary.
    shot_type: str | None = Field(default=None, description="e.g. 'DRAW', 'SLIGHT FADE'")
    impact_position: str | None = Field(default=None, description="e.g. 'CENTER', 'TOE'")

    # Set only by sources that infer metrics (screen capture); None for direct feeds.
    provenance: ShotProvenance | None = None
