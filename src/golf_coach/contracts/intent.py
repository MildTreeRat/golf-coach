"""Practice-intent contracts (consumed by `analysis` scoring). [M4-PoC]

Per [ADR-009](../../docs/decisions/009-swing-scoring-model.md): judging a swing is
meaningless without knowing what the golfer was *trying* to do. `PracticeGoal` carries that
intent into `analyze_swing`, where the `mode` selects a scoring policy and (later) the
target shape / club parameterize the expected checkpoint ranges.

This module imports nothing from the rest of `contracts` — intent is an input, so keeping it
dependency-free means `swing.py` can import `PracticeGoal` without a cycle.
"""

from __future__ import annotations

from enum import StrEnum

from pydantic import BaseModel


class PracticeMode(StrEnum):
    """What the golfer is practicing — selects a scoring policy (ADR-009).

    Only ``FUNDAMENTALS`` is implemented in the PoC; the rest are named seams that
    ``scoring.policy_for`` rejects with ``NotImplementedError`` until full M4.
    """

    FUNDAMENTALS = "fundamentals"  # grade mechanics only; outcome informational
    SHOT_SHAPING = "shot_shaping"  # grade outcome-vs-intended-shape (full M4)
    PERFORMANCE = "performance"  # grade result vs club/skill benchmarks (full M4)
    DRILL = "drill"  # spotlight one checkpoint (full M4)


class TargetShape(StrEnum):
    """Intended ball flight — parameterizes outcome ranges in full M4."""

    STRAIGHT = "straight"
    DRAW = "draw"
    FADE = "fade"


class ClubCategory(StrEnum):
    """Club family used to key benchmark rows (ADR-010 §3).

    ``ALL`` is the club-independent fallback — tempo and posture key on it.
    """

    DRIVER = "driver"
    WOOD = "wood"
    HYBRID = "hybrid"
    LONG_IRON = "long_iron"
    MID_IRON = "mid_iron"
    SHORT_IRON = "short_iron"
    WEDGE = "wedge"
    PUTTER = "putter"
    ALL = "all"


class PlayerProfile(BaseModel):
    """Who is swinging — keys skill-parameterized benchmark rows (ADR-010 §3).

    Deliberately thin for the PoC: only ``skill_level``. Height / physical limits are
    deferred to future mechanics ranges.
    """

    skill_level: str = "all"


class PracticeGoal(BaseModel):
    """The golfer's intent for a swing (ADR-009 §Concepts).

    Selected at session level, overridable per shot. Defaults describe the PoC case:
    Fundamentals mode, no declared shape, club-agnostic.
    """

    mode: PracticeMode = PracticeMode.FUNDAMENTALS
    target_shape: TargetShape | None = None
    club: ClubCategory = ClubCategory.ALL
    focus_checkpoint: str | None = None
