"""Derived launch-monitor quantities. Measure, never judge. [M6.5 — data infrastructure]

`ShotData` carries what the HD Golf screen printed. This module computes the things that are
*derived* from those numbers and worth recording in their own right — chiefly **face-to-path**,
which the screen does not print but which is the quantity that actually explains the ball's
curvature.

Nothing here scores anything, and nothing here needs a reference population. That is the point:
face-to-path relates to shape by ball-flight physics, not by a tour distribution, so it is
measurable today whereas carry and spin norms are blocked on data nobody has acquired
(ADR-010 §4 names TrackMan / Arccos as the eventual source).

## What is deliberately excluded, and why

**`smash_factor` and `club_head_speed`.** Every shot on disk reads a smash factor of 0.89-1.00 —
ball speed *below* club speed, which a real strike cannot produce — and one reads 159.5 mph of club
speed for 195.9 yards of carry. The OCR is faithful: `launch_monitor/screen/validate.py` checks
`ball_speed / club_head_speed == printed smash` and it passes on all three, so the simulator itself
is printing these. Recording them as measurements would invite a later derivation step to cut a
band from a device artifact, which is a wrong number with a provenance string attached.

**`spin_axis`.** Its sign is unresolved and contradicts `contracts/shot.py`, which documents
`+ = fade`: both fades on disk carry a *negative* value. The parser warns it stored an
uninterpreted magnitude because the screen prints no direction word. Face-to-path answers the same
question with signs that demonstrably work.

**Dispersion, and anything else needing more than one shot.** There is one shot per session on
disk. A spread over n=1 is not a spread.

ADR-008: `analysis` is a pure functional core and must not import `launch_monitor`. Everything here
reads `ShotData` from `contracts` and computes from its fields.
"""

from __future__ import annotations

from golf_coach.contracts.intent import TargetShape
from golf_coach.contracts.shot import ShotData

#: Tokens the HD Golf screen uses in its free-text `Shot Type` tile. `contracts/shot.py` leaves
#: this vocabulary un-normalized at the ingest boundary on purpose — "normalizing them belongs in
#: analysis" — and this is that step.
#:
#: **Order is load-bearing: curvature words are checked before centering words.** The screen prints
#: compound labels like `"CENTER SLIGHT FADE"`, where `CENTER` describes the *start line* and
#: `FADE` the *curve*. Matching `CENTER` first classified a real recorded fade as STRAIGHT.
_SHAPE_TOKENS: tuple[tuple[str, TargetShape], ...] = (
    # Curvature — these decide the shape.
    ("DRAW", TargetShape.DRAW),
    ("HOOK", TargetShape.DRAW),
    ("FADE", TargetShape.FADE),
    ("SLICE", TargetShape.FADE),
    # Only reached when nothing curved.
    ("STRAIGHT", TargetShape.STRAIGHT),
    ("CENTER", TargetShape.STRAIGHT),
)


def measure_face_to_path(shot: ShotData) -> float | None:
    """`club_face_angle - club_path`, in degrees. The quantity that explains curvature.

    Positive means the face is **open to the path** (a fade/slice shape for a right-handed
    golfer); negative means closed to it (draw/hook). Zero is a straight shot regardless of where
    either number sits individually, which is exactly why the difference is the observable and
    neither angle is on its own — ADR-009 §Concepts names face-to-path for this reason.

    On the three shots recorded so far it agrees with the simulator's own shape verdict every
    time: +13.2 on a FADE, +10.9 on a CENTER SLIGHT FADE, -11.8 on a DRAW. That agreement is worth
    something specific — it is an independent check that the OCR resolved *both* signs correctly,
    since the two fields carry different sign vocabularies (`OPEN`/`CLOSED` against `I>O`/`O>I`).

    Returns None if either angle is missing.
    """
    if shot.club_face_angle is None or shot.club_path is None:
        return None
    return shot.club_face_angle - shot.club_path


def measure_start_line(shot: ShotData) -> float | None:
    """Initial horizontal launch direction, in degrees; positive is right of target.

    Where the ball *started*, as opposed to how it curved afterwards. Kept separate from
    face-to-path because the two together are what distinguish a pull-fade from a push-fade —
    the same curvature off a different start line is a different miss and a different fix.
    """
    return shot.launch_direction


def normalize_shot_shape(shot: ShotData) -> TargetShape | None:
    """Map the simulator's free-text shape ("CENTER SLIGHT FADE") onto `TargetShape`.

    Coarse by design: the sim's qualifiers ("SLIGHT") describe magnitude, and magnitude is what
    `measure_face_to_path` reports in degrees. This answers only *which way did it curve*, which is
    the part that has to line up with an intended shape later.

    A compound label resolves to its **curvature** word, not its start-line word — "CENTER SLIGHT
    FADE" is a fade that started centered, and reading it as STRAIGHT loses the only part of the
    label this function exists to extract. See `_SHAPE_TOKENS` for the ordering that enforces it.

    Returns None when nothing matches rather than guessing STRAIGHT — an unrecognized vocabulary is
    a fact worth surfacing, not a default worth inventing.
    """
    if not shot.shot_type:
        return None
    text = shot.shot_type.upper()
    for token, shape in _SHAPE_TOKENS:
        if token in text:
            return shape
    return None


#: Name -> (function, unit, detail). Mirrors `measure.POSE_MEASUREMENTS` so the engine can build
#: both families the same way. Only numeric measurements live here; `normalize_shot_shape` returns
#: a category and is consumed directly.
SHOT_MEASUREMENTS = {
    "face_to_path_deg": (
        measure_face_to_path,
        "degrees",
        "club face angle minus club path; + is face open to path (fade shape)",
    ),
    "start_line_deg": (
        measure_start_line,
        "degrees",
        "initial horizontal launch direction; + is right of target",
    ),
}
