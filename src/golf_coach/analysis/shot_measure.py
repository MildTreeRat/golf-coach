"""Derived launch-monitor quantities. Measure, never judge. [M6.5 — data infrastructure]

`ShotData` carries what the HD Golf screen printed. This module computes the things that are
*derived* from those numbers and worth recording in their own right — chiefly **face-to-path**,
which the screen does not print but which is the quantity that actually explains the ball's
curvature.

Nothing here scores anything, and nothing here needs a reference population. That is the point:
face-to-path relates to shape by ball-flight physics, not by a tour distribution, so it is
measurable today whereas carry and spin *norms* are blocked on data nobody has acquired
(ADR-010 §4 names TrackMan / Arccos as the eventual source). Norms, not the numbers: carry itself is
measured here as of M9 and judged by nothing, which is that sentence working rather than an
exception to it.

## What arrived late, and why it could not arrive earlier

**`carry_distance_yds` and `total_distance_yds`.** These were readable off `ShotData` from the day
the OCR worked, and were left out until M9 anyway. A carry pooled across clubs is not a weak
statistic, it is a meaningless one — a mean over a driver and a sand wedge describes nobody's shot,
and its spread is mostly which clubs happened to be hit. **The club tag is what makes distance
poolable at all**, so distance enters the corpus with M9's `SwingManifest.club` and not with M6.5,
which is when everything else in this module was written.

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


def measure_carry_distance(shot: ShotData) -> float | None:
    """Carry, in yards, exactly as the screen printed it. No arithmetic at all.

    A pass-through is still worth a registry row: what `measurements` holds is what
    `storage.corpus` pools and what `analysis.baseline` builds a personal mean from, and a field
    that stays on `ShotData` reaches none of that. The module docstring above says why this one
    waited for a club tag.
    """
    return shot.carry_distance


def measure_total_distance(shot: ShotData) -> float | None:
    """Carry plus roll, in yards, as printed.

    Kept beside carry rather than folded into it because the two answer different questions and
    the gap between them is the ground, not the swing — a number that moves with a mat, a fairway
    setting and a season. `ShotData.bounce_and_roll` already prints that gap, so it is not measured
    here: recording `total - carry` beside both is a third number free to disagree with them.
    """
    return shot.total_distance


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
    # The two distances are **pooled whole-bag** by everything that reads them until
    # `storage.corpus.narrow_to(club=)` lands (M9 P13), which is what makes a mean carry mean
    # anything. Nothing false ships in the meantime — `contracts.baseline.DEFAULT_MINIMUM_N`
    # refuses a center below 5 samples — but the guard that saves this is a sample count, not an
    # argument about clubs, so it would go on being satisfied by a mixed bag.
    "carry_distance_yds": (
        measure_carry_distance,
        "yards",
        "carry as printed by the launch monitor; meaningful per club, not across a bag",
    ),
    "total_distance_yds": (
        measure_total_distance,
        "yards",
        "carry plus roll as printed; the gap to carry is the ground, not the swing",
    ),
}
