"""Skeleton overlay rendering. [M1]

Draws a `FrameKeypoints` skeleton onto a BGR frame for visual accuracy review. Works purely
off our own contract (no MediaPipe): it denormalizes landmark coordinates back to pixels and
connects them with a small bone list, drawing a dot at every landmark that list touches — the
dot set is derived from the bones so the two cannot disagree. This proves `FrameKeypoints`
carries everything a consumer needs to visualize a pose. OpenCV is imported lazily so
importing this module stays cheap.

Over that skeleton it draws the rotation picture [M17 P1]: the shoulder line and the hip line as
emphasised axes, and a marker at the three points a swing turns about — those two lines' midpoints
and the hands. That is a picture only; the numbers read off the same three points live in
`analysis/pivot.py` and are measured from the keypoints, never from these pixels.
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Any

from golf_coach.contracts.keypoints import FrameKeypoints, PoseLandmark

if TYPE_CHECKING:
    import numpy as np

# Minimal skeleton topology: which landmarks to connect with a bone. Enough to read body
# posture through a swing without drawing all of MediaPipe's edges.
_BONES: tuple[tuple[PoseLandmark, PoseLandmark], ...] = (
    # arms
    (PoseLandmark.LEFT_SHOULDER, PoseLandmark.LEFT_ELBOW),
    (PoseLandmark.LEFT_ELBOW, PoseLandmark.LEFT_WRIST),
    (PoseLandmark.RIGHT_SHOULDER, PoseLandmark.RIGHT_ELBOW),
    (PoseLandmark.RIGHT_ELBOW, PoseLandmark.RIGHT_WRIST),
    # shoulders + torso
    (PoseLandmark.LEFT_SHOULDER, PoseLandmark.RIGHT_SHOULDER),
    (PoseLandmark.LEFT_SHOULDER, PoseLandmark.LEFT_HIP),
    (PoseLandmark.RIGHT_SHOULDER, PoseLandmark.RIGHT_HIP),
    (PoseLandmark.LEFT_HIP, PoseLandmark.RIGHT_HIP),
    # legs
    (PoseLandmark.LEFT_HIP, PoseLandmark.LEFT_KNEE),
    (PoseLandmark.LEFT_KNEE, PoseLandmark.LEFT_ANKLE),
    (PoseLandmark.RIGHT_HIP, PoseLandmark.RIGHT_KNEE),
    (PoseLandmark.RIGHT_KNEE, PoseLandmark.RIGHT_ANKLE),
    # head — exactly one bone, and it is the quantity `evaluate_head_sway` scores: the midpoint
    # of these two ears is the head centre (`measure.head_center_points`, whose docstring records
    # why the ears and not the nose — the nose swings with head rotation and read a stable head
    # as 1.18 shoulder-widths of sway). Drawing the pair joined puts the measured thing on screen.
    # It is not decorative; do not tidy it away as a redundant line.
    (PoseLandmark.LEFT_EAR, PoseLandmark.RIGHT_EAR),
    # hands — a three-spoke fan per side. Nothing reads landmarks 17-22 yet (M14 P3 is the
    # reliability screen that decides whether anything ever will), so this draws structure rather
    # than a measurement: a fan can be reviewed by eye for whether the hands are tracked, and six
    # loose specks could not be. That review is the prerequisite for trusting P3's numbers.
    (PoseLandmark.LEFT_WRIST, PoseLandmark.LEFT_INDEX),
    (PoseLandmark.LEFT_WRIST, PoseLandmark.LEFT_PINKY),
    (PoseLandmark.LEFT_WRIST, PoseLandmark.LEFT_THUMB),
    (PoseLandmark.RIGHT_WRIST, PoseLandmark.RIGHT_INDEX),
    (PoseLandmark.RIGHT_WRIST, PoseLandmark.RIGHT_PINKY),
    (PoseLandmark.RIGHT_WRIST, PoseLandmark.RIGHT_THUMB),
)

# Which landmarks get a dot — *derived* from `_BONES`, never listed separately. The two lists
# used to be independent (the dot loop walked all 33 of `PoseLandmark`) and they drifted: the
# overlay rendered eleven head dots — four eye points, two mouth corners, two ears, the nose,
# plus two heels and two foot indices below — that no bone touched and no checkpoint measured.
# Deriving is the same rule CLAUDE.md states for the checkpoint panel: derive membership, never
# restate it. Sorted so the draw order is landmark order and a render is reproducible. [M14 P1]
_JOINTS: tuple[PoseLandmark, ...] = tuple(sorted({lm for bone in _BONES for lm in bone}))

# The two segments a turn is visible in, drawn a second time over the bones that already join
# those same pairs. The overdraw is deliberate: the shoulder segment and the hip segment are two of
# twenty-one identical white bones, and the whole of M17 is asking a golfer to watch those two
# rotate. Emphasis is the point — the same reason the ear bone above is drawn. Not decorative; do
# not tidy either away as a redundant line. [M17 P1]
#
# The tempting tidy-up — move these pairs out of `_BONES` so nothing is drawn twice — would
# re-derive `_JOINTS` and silently drop the shoulder and hip dots. Leave `_BONES` alone; the guide
# line is thicker and covers the bone anyway.
_GUIDE_LINES: tuple[tuple[PoseLandmark, PoseLandmark], ...] = (
    (PoseLandmark.LEFT_SHOULDER, PoseLandmark.RIGHT_SHOULDER),
    (PoseLandmark.LEFT_HIP, PoseLandmark.RIGHT_HIP),
)

# The pivot points: each pair's midpoint gets a marker. The first two are *derived* from
# `_GUIDE_LINES` — the centres those lines turn about, which is what makes them the same two pairs
# and not a second list that can drift. The third is the hands, drawn here and deliberately
# unmeasured: M14 P3 put face-on hand tracking at 0.63-0.68 over a whole clip, so a through-swing
# hand number would report the tracker rather than the golfer (ADR-029). Seeing them is still worth
# it — the hands are what a golfer recognises the swing by. [M17 P1]
_PIVOT_POINTS: tuple[tuple[PoseLandmark, PoseLandmark], ...] = (
    *_GUIDE_LINES,
    (PoseLandmark.LEFT_WRIST, PoseLandmark.RIGHT_WRIST),
)

# How far past each endpoint a guide line runs, as a fraction of that segment's own length. A line
# stopping at the joints reads as one more bone; one that overshoots reads as an axis. Scaled by the
# segment rather than by the frame so it shrinks with the golfer and cannot run off a close crop.
# Applied as a proportion of the segment vector, with no normalisation, so a collapsed line — both
# landmarks on one pixel, which is the face-on shoulder line at the top of the swing — extends to
# itself instead of dividing by zero. That collapse is the honest picture, and it is the same
# degeneracy `contracts/pivots.py` will answer with a `None` orientation.
_GUIDE_EXTENSION = 0.25

# A landmark dimmer than this is treated as not-confidently-seen and skipped.
_MIN_VISIBILITY = 0.5

_JOINT_COLOR = (0, 255, 0)  # BGR green
_BONE_COLOR = (255, 255, 255)  # BGR white
_GUIDE_COLOR = (255, 255, 0)  # BGR cyan — the rotation axes
_PIVOT_COLOR = (255, 0, 255)  # BGR magenta — the points they turn about


def draw_skeleton(
    image: np.ndarray[Any, Any], keypoints: FrameKeypoints
) -> np.ndarray[Any, Any]:
    """Draw the skeleton for one frame onto a copy of `image` (BGR). Returns the copy."""
    import cv2

    canvas = image.copy()
    height, width = canvas.shape[:2]

    def pixel(which: PoseLandmark) -> tuple[int, int] | None:
        lm = keypoints.landmark(which)
        if lm.visibility < _MIN_VISIBILITY:
            return None
        return int(lm.x * width), int(lm.y * height)

    # Bones first so joints render on top.
    for a, b in _BONES:
        pa, pb = pixel(a), pixel(b)
        if pa is not None and pb is not None:
            cv2.line(canvas, pa, pb, _BONE_COLOR, 2)

    # The rotation axes, over the bones they repeat and under the joints. Both endpoints must be
    # confidently seen: half a shoulder line is a line at an invented angle.
    for a, b in _GUIDE_LINES:
        pa, pb = pixel(a), pixel(b)
        if pa is None or pb is None:
            continue
        dx = int((pb[0] - pa[0]) * _GUIDE_EXTENSION)
        dy = int((pb[1] - pa[1]) * _GUIDE_EXTENSION)
        start = (pa[0] - dx, pa[1] - dy)
        end = (pb[0] + dx, pb[1] + dy)
        cv2.line(canvas, start, end, _GUIDE_COLOR, 3)

    for landmark in _JOINTS:
        p = pixel(landmark)
        if p is not None:
            cv2.circle(canvas, p, 3, _JOINT_COLOR, -1)

    # The pivot markers last, so a centre is never hidden under the joint or bone it sits between.
    for a, b in _PIVOT_POINTS:
        pa, pb = pixel(a), pixel(b)
        if pa is None or pb is None:
            continue
        cv2.circle(canvas, ((pa[0] + pb[0]) // 2, (pa[1] + pb[1]) // 2), 5, _PIVOT_COLOR, -1)

    return canvas


_BANNER_COLOR = (0, 165, 255)  # BGR amber — phase-instant marker
_HUD_BG = (0, 0, 0)  # BGR black backdrop for legibility
_HUD_TEXT = (255, 255, 255)  # BGR white


def annotate_frame(
    image: np.ndarray[Any, Any],
    keypoints: FrameKeypoints,
    banner: str | None = None,
    hud_lines: tuple[str, ...] = (),
) -> np.ndarray[Any, Any]:
    """Skeleton overlay plus an optional phase banner and a corner score HUD (verification).

    `banner` (e.g. "TOP OF BACKSWING") is stamped across the top on the detected instant
    frames so you can eyeball that the segmentation landed on the right moment; `hud_lines`
    render as a stacked caption in the lower-left. Both draw on a copy — the input is not
    mutated. Used by `scripts/analyze_swing.py` as the no-hardware accuracy check.
    """
    import cv2

    canvas = draw_skeleton(image, keypoints)
    height, width = canvas.shape[:2]

    if banner:
        cv2.rectangle(canvas, (0, 0), (width, 34), _HUD_BG, -1)
        cv2.putText(
            canvas, banner, (10, 24), cv2.FONT_HERSHEY_SIMPLEX, 0.7, _BANNER_COLOR, 2, cv2.LINE_AA
        )

    if hud_lines:
        line_h = 20
        y0 = height - line_h * len(hud_lines) - 8
        cv2.rectangle(canvas, (0, y0 - 6), (width, height), _HUD_BG, -1)
        for i, line in enumerate(hud_lines):
            y = y0 + line_h * (i + 1)
            cv2.putText(
                canvas, line, (10, y), cv2.FONT_HERSHEY_SIMPLEX, 0.5, _HUD_TEXT, 1, cv2.LINE_AA
            )

    return canvas
