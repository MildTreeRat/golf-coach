"""Two-view alignment contracts (produced by `analysis.alignment`). [M7 Phase 2]

Two hand-held phones film one swing and agree on nothing: not the frame rate, not the clip
length, not the moment recording started. What they *do* both see is the swing itself, so the
correspondence between them is expressed on a **normalized swing-time axis** rather than a clock:

    tau = 0  motion start        tau = 1  top of backswing        tau = 2  impact

`tau` is dimensionless. One unit is one backswing between 0 and 1, and one downswing between 1
and 2; past impact it keeps running at the downswing rate. That is what makes a 60 fps clip and a
240 fps slo-mo clip of the same swing describe the same numbers — the same reason `tempo_ratio`
survives a frame-rate change, and the same clip-relative principle as ADR-013.

**These shapes deliberately do not promise more than the detector delivers.** `AlignmentQuality`
exists so a consumer can say *"aligned on impact only"* out loud instead of rendering two panels
side by side and letting the viewer assume frame accuracy nobody measured. Anchor reliability from
the repo's own bake-off (docs/M4_POSE_BAKEOFF.md): impact a median of 1 frame, top 2 frames,
motion start 7 frames with 40% of clips over 10 — which is why motion start is a *soft* anchor
here and the other two are not.

Pydantic + stdlib only, like every other contract (ADR-008).
"""

from __future__ import annotations

from enum import StrEnum

from pydantic import BaseModel, Field, model_validator

# The three fixed points of the axis. Named because they appear in both the warp and its inverse,
# and a bare 2.0 in that arithmetic is unreadable.
TAU_MOTION_START = 0.0
TAU_TOP = 1.0
TAU_IMPACT = 2.0


class AlignmentQuality(StrEnum):
    """How much of the swing the two clips were actually aligned *on*.

    Ordered from most to least evidence. The value is what a UI should surface — rendering a
    side-by-side video implies frame correspondence everywhere, and only `FULL` earns that.
    """

    FULL = "full"
    TOP_IMPACT = "top_impact"
    IMPACT_ONLY = "impact_only"
    UNALIGNED = "unaligned"

    @property
    def summary(self) -> str:
        """One human-readable clause, for a HUD line or a results page."""
        return _QUALITY_SUMMARY[self]

    @property
    def is_aligned(self) -> bool:
        return self is not AlignmentQuality.UNALIGNED


_QUALITY_SUMMARY: dict[AlignmentQuality, str] = {
    AlignmentQuality.FULL: "aligned on motion start, top and impact",
    AlignmentQuality.TOP_IMPACT: "aligned on top and impact",
    AlignmentQuality.IMPACT_ONLY: "aligned on impact only",
    AlignmentQuality.UNALIGNED: "not aligned",
}


class SwingAnchors(BaseModel):
    """The three swing instants one clip contributes to the warp, as frame indices.

    Built either from `segment_phases()` output (`analysis.alignment.anchors_from_phases`) or by
    hand when a detector has to be overridden. Both routes produce this same shape on purpose:
    the manual path is a *parameter* of the alignment, not a second code path through it.
    """

    motion_start: int = Field(ge=0)
    top: int = Field(ge=0)
    impact: int = Field(ge=0)

    motion_start_detected: bool = Field(
        default=True,
        description=(
            "False when `motion_start` is `segment_phases`' bounded estimate rather than something "
            "found in the signal (ADR-013). It is still the best available number — it just may "
            "not be used as a shared anchor, because two clips guessing separately do not agree."
        ),
    )

    camera_id: str | None = Field(
        default=None, description="Which view this clip is, when the keypoints recorded it."
    )
    frame_count: int | None = Field(
        default=None, ge=0, description="Clip length, used to clamp mapped indices in range."
    )
    fps: float | None = Field(
        default=None,
        gt=0.0,
        description=(
            "Container-reported frame rate. Nothing in the warp needs it — that is the whole "
            "point of a normalized axis — except the degenerate `IMPACT_ONLY` case, which has no "
            "second anchor to derive a rate from. Treat it with the suspicion "
            "docs/M7_TWO_PHONE_SPIKE.md Q3 earns it."
        ),
    )

    @model_validator(mode="after")
    def _ordered(self) -> SwingAnchors:
        """A swing goes motion start -> top -> impact, and impact is strictly after the top.

        `motion_start == top` is tolerated (a clip that opens mid-takeaway), but a zero-length
        downswing is not: it is the denominator of the entire tau axis.
        """
        if self.motion_start > self.top:
            raise ValueError(
                f"motion_start ({self.motion_start}) must not be after top ({self.top})"
            )
        if self.impact <= self.top:
            raise ValueError(f"impact ({self.impact}) must be after top ({self.top})")
        return self

    @property
    def downswing_frames(self) -> int:
        """Frames from the top to impact — the clip's own time base (ADR-013)."""
        return self.impact - self.top

    @property
    def backswing_frames(self) -> int:
        return self.top - self.motion_start

    @property
    def tempo_ratio(self) -> float | None:
        """Backswing:downswing, or None when there is no backswing to measure.

        Frame rate cancels, so two clips of the *same* swing must agree on this however
        differently the phones were configured. That is what makes it usable as a cross-check on
        the soft anchor rather than merely a checkpoint metric.
        """
        if self.backswing_frames <= 0:
            return None
        return self.backswing_frames / self.downswing_frames


class ClipAlignment(BaseModel):
    """One clip's place on the shared tau axis."""

    anchors: SwingAnchors = Field(description="The instants as detected or supplied.")

    warp_motion_start: int = Field(
        ge=0,
        description=(
            "The frame the warp actually pins to tau=0. Equal to `anchors.motion_start` when the "
            "soft anchor was accepted; otherwise the tour-median estimate substituted into *both* "
            "clips, so a degraded pre-top region degrades identically in each rather than in one "
            "panel only."
        ),
    )

    tau_start: float = Field(description="tau at frame 0 — negative when the clip rolls early.")
    tau_end: float = Field(description="tau at the last frame.")


class SwingAlignment(BaseModel):
    """The correspondence between two clips of one swing, plus how much to trust it.

    `a` and `b` are None only when the swing could not be aligned at all, which is reported rather
    than raised: a detector that fails should say so (ADR-013).
    """

    a: ClipAlignment | None = None
    b: ClipAlignment | None = None

    quality: AlignmentQuality = AlignmentQuality.UNALIGNED

    notes: list[str] = Field(
        default_factory=list,
        description=(
            "Why the quality is what it is, in order of discovery — e.g. 'down_the_line motion "
            "start was estimated, not detected'. A bare enum says the alignment degraded; these "
            "say what to fix."
        ),
    )

    overlap: tuple[float, float] | None = Field(
        default=None,
        description=(
            "The tau range both clips actually cover. Rendering outside it would mean holding one "
            "panel frozen on its first or last frame while the other moves, which looks like a "
            "sync failure and is really just one phone that started rolling later."
        ),
    )
