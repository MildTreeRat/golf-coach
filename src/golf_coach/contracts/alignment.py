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
from typing import NamedTuple

from pydantic import BaseModel, Field, computed_field, model_validator

# The three fixed points of the axis. Named because they appear in both the warp and its inverse,
# and a bare 2.0 in that arithmetic is unreadable.
TAU_MOTION_START = 0.0
TAU_TOP = 1.0
TAU_IMPACT = 2.0


class AlignmentQuality(StrEnum):
    """How much of the swing the two clips were actually aligned *on*.

    Ordered from most to least evidence. The value is what a UI should surface — rendering a
    side-by-side video implies frame correspondence everywhere, and only `FULL` earns that.

    **`SYNCHRONIZED` is a different kind of claim from the three below it** (M11 P6). Those count
    *inferred* anchors: three pose estimates agree, or two, or one. `SYNCHRONIZED` says the tau=2
    anchor was **heard** in both clips — the ball strike reaches both microphones, so it is the one
    instant the two recordings share on a real clock rather than by inference. That is why it sits
    above `FULL` instead of beside it, and why it overwrites the count: one measured anchor is
    better evidence than three estimated ones (docs/M11_ACOUSTIC_SYNC.md §E4 found four bundles
    where a `full`/`impact_only` pair had its down-the-line impact 5.7-7.5 frames wrong).

    The count is not lost when that happens — every anchor `analysis.alignment` refuses appends its
    own note, so what the tier stops carrying, `SwingAlignment.notes` still says out loud.
    """

    SYNCHRONIZED = "synchronized"
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

    @property
    def is_degraded(self) -> bool:
        """True when the warp is standing on less than the best evidence available to it.

        **Deliberately not the same test as the one that gates `caveats.ALIGNMENT_CAVEAT`**, which
        is `is not FULL` at each of its two call sites and stays that way. The two ask different
        questions. This one asks *did something go wrong* — and a pair anchored on a heard ball
        strike is the opposite of that, so a `SYNCHRONIZED` result must not be reported as
        degraded. The caveat asks *may a reader trust every instant*, and the answer there is still
        no: `SYNCHRONIZED` pins tau=2 to a measured event and interpolates between anchors exactly
        as `FULL` does, so "the two views were synchronized on the ball strike, **not on every
        instant**" is the sentence a reader of a synchronized pair needs, not one to suppress.
        """
        return self not in _UNDEGRADED


_QUALITY_SUMMARY: dict[AlignmentQuality, str] = {
    AlignmentQuality.SYNCHRONIZED: "synchronized on the ball strike",
    AlignmentQuality.FULL: "aligned on motion start, top and impact",
    AlignmentQuality.TOP_IMPACT: "aligned on top and impact",
    AlignmentQuality.IMPACT_ONLY: "aligned on impact only",
    AlignmentQuality.UNALIGNED: "not aligned",
}

# The tiers `is_degraded` answers False for. A frozenset rather than a chain of `is not`
# comparisons at each caller, so the membership lives in one place next to the summaries it has to
# stay consistent with.
_UNDEGRADED = frozenset({AlignmentQuality.SYNCHRONIZED, AlignmentQuality.FULL})


class FramePairing(NamedTuple):
    """One output frame: the swing instant it shows, and the frame of each clip showing it.

    The seam between working out the correspondence and drawing it. `analysis.alignment` produces
    a schedule of these from a `SwingAlignment`; a renderer consumes it and never does warp
    arithmetic of its own — which is what lets the two live in different modules (ADR-008) and
    what makes the correspondence testable without decoding a single frame.

    One `tau` per output frame, shared by both panels. That is the whole reason the ADDRESS / TOP
    / IMPACT banners land simultaneously by construction rather than by coincidence.
    """

    tau: float
    frame_a: int
    frame_b: int


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

    impact_measured: bool = Field(
        default=False,
        description=(
            "True when `impact` was pinned to a ball strike **heard** in this clip rather than "
            "inferred from pose (`analysis.alignment.with_measured_impact`, M11 P6). It is the one "
            "anchor that can be measured: the crack reaches both phones, so two clips carrying it "
            "share a real clock and the pair reports `AlignmentQuality.SYNCHRONIZED`. "
            "Defaults False so every artifact written before M11 reads back as what it was — "
            "inferred — rather than silently claiming a measurement nobody took, the same way "
            "`ClipAlignment.warp_top` was added."
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
            "Container-reported frame rate. Nothing in the *warp* needs it — that is the whole "
            "point of a normalized axis — but everything that has to compare the two clips in real "
            "seconds does: the shared-duration fallbacks, and the strike window "
            "`with_measured_impact` converts from seconds into this clip's frames. Treat it with "
            "the suspicion docs/M7_TWO_PHONE_SPIKE.md Q3 earns it."
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

    warp_top: int | None = Field(
        default=None,
        ge=0,
        description=(
            "The frame the warp pins to tau=1, when that is *not* `anchors.top`. Only the "
            "IMPACT_ONLY tier sets it: there the two views disagree about how long the downswing "
            "lasted, so rather than resample one panel onto the other's timeline both clips take "
            "the same duration measured back from impact and each advances at its own native "
            "rate. None on every other tier — and on artifacts written before the tier existed, "
            "which is why this is optional rather than required. Read it through `.top`."
        ),
    )

    top_late_by: int | None = Field(
        default=None,
        gt=0,
        description=(
            "How many of **this clip's** frames its detected top sits past the real one, when a "
            "shared clock can say (M11 P7/P8). None means either that the tops agree or that "
            "nothing could arbitrate them; it never means zero. Set only on a pair that both heard "
            "the ball strike: with tau=2 pinned to one sound the two downswings measure one "
            "interval in real time, so the shorter one is the *late* top rather than merely the "
            "disagreeing one (`analysis.alignment._arbitrate_tops`). "
            "**This is the diagnosis, and `warp_top` is the correction — they are separate on "
            "purpose.** The warp only moves a top when the duration it would be held to is a "
            "possible downswing (`phases._PLAUSIBLE_DOWNSWING_S`), so a bundle can carry a top "
            "known to be late that the warp declined to move; reporting only the corrected ones "
            "would leave every declined case looking sound. `analysis.engine` reads *this* field, "
            "not `warp_top`, when it retires a checkpoint timed from a contradicted instant."
        ),
    )

    tau_start: float = Field(description="tau at frame 0 — negative when the clip rolls early.")
    tau_end: float = Field(description="tau at the last frame.")

    @property
    def top(self) -> int:
        """The frame tau=1 resolves to: the override when there is one, else the detected top."""
        return self.anchors.top if self.warp_top is None else self.warp_top

    @property
    def top_is_late(self) -> bool:
        """Whether a shared clock contradicted this top at all. `top_late_by` says how far.

        A property so no consumer writes `is not None` against a field whose None means "nothing
        could decide" rather than "nothing was wrong" — two readings a bare comparison invites
        mixing up.
        """
        return self.top_late_by is not None


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

    @computed_field  # type: ignore[prop-decorator]
    @property
    def quality_summary(self) -> str:
        """`quality.summary`, carried in the serialized payload.

        Serialized rather than left to the reader because the alternative is every consumer
        keeping its own copy of the enum-to-sentence mapping — and the results page, which had
        exactly that job, instead printed the bare enum value (`top_impact`) at a viewer for two
        milestones.
        """
        return self.quality.summary
