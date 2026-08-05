"""Photo -> a screen image OCR can actually read. [ADR-014]

This is the stage that makes local OCR viable on a phone snap of a simulator screen, and
it exists because of three specific things in real photos of this rig:

  - **Perspective.** You stand beside the monitor, so the screen is a trapezoid. Text
    baselines are not horizontal, which wrecks line grouping. Fixed → detect the screen's
    quadrilateral and warp it back to a rectangle.
  - **Orientation.** `cv2.imread` applies the EXIF orientation tag, which covers a phone
    photo straight off the camera roll. It does *not* cover an image whose metadata was
    stripped in transit (re-encodes, screenshots, chat apps) or a frame decoded from video,
    where EXIF does not exist at all — and those arrive silently sideways. Fixed → try all
    four rotations and keep whichever one the device profile's own labels are actually
    found in. Letting the *content* vote means orientation is right whether or not the
    metadata survived.
  - **Glare and reflections.** Ceiling lights blow out patches of the screen and the room
    reflects across the tiles. Fixed → CLAHE on the lightness channel, which lifts local
    contrast inside the dark tiles without amplifying the bright reflections.

Requires the `vision` extra for OpenCV, so it is imported directly rather than re-exported
from the package `__init__` — importing the parser must stay dependency-free (ADR-008).
"""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import Any

import cv2
import numpy as np

from golf_coach.launch_monitor.screen.profiles import DeviceProfile
from golf_coach.launch_monitor.screen.recognizer import TextBox, TextRecognizer

# The rectified screen is normalized to this width before OCR. Big enough that the tile
# values stay legible, small enough that four rotation attempts stay quick.
_TARGET_WIDTH = 1600

# Outline detection runs on a downscaled copy. This is not just for speed: the morphology
# kernel that bridges gaps in the bezel edge is a fixed pixel size, so on a 5712px phone
# photo it is far too small to close anything and the screen contour comes back shattered.
# Normalizing the width first is what makes the kernel meaningful.
_DETECT_WIDTH = 1000

# A candidate screen quad must cover this much of the frame: enough that a picture frame
# in the background never wins, capped so a contour that merely traced the photo's own
# border is rejected rather than "found".
_MIN_QUAD_AREA_RATIO = 0.15
_MAX_QUAD_AREA_RATIO = 0.92

# The 4-gon must actually resemble the contour it came from — an approximation that
# balloons past this multiple came from a sprawling edge fragment, not a screen.
_MAX_QUAD_FILL_RATIO = 1.6

# Edge/close settings to try in order. The dark HD Golf UI against a lit room needs
# different thresholds depending on how much glare there is, and one photo's best
# setting is another's worst — so try a few rather than tune for one picture.
_CANNY_PRESETS = ((30, 120), (10, 60))
_CLOSE_SIZES = (5, 15, 25)
_APPROX_EPSILONS = (0.02, 0.04, 0.01)

# Rotations to try, best-guess first: upright, then upside-down (the common phone flip),
# then the two sideways cases.
_ROTATIONS = (0, 180, 90, 270)

# Stop the rotation search early once this fraction of the profile's labels are found —
# no orientation other than the right one gets close to this.
_GOOD_ENOUGH_LABEL_RATIO = 0.6


@dataclass(frozen=True)
class PreparedScreen:
    """A rectified, correctly-oriented screen and the text found on it."""

    image: Any
    boxes: list[TextBox]
    rotation: int
    rectified: bool
    label_ratio: float
    notes: list[str]


def load_image(path: Path | str) -> Any:
    """Read an image file as BGR. Raises rather than returning None on failure."""
    image = cv2.imread(str(path), cv2.IMREAD_COLOR)
    if image is None:
        raise OSError(f"Could not read image: {path}")
    return image


def prepare_screen(
    image: Any, recognizer: TextRecognizer, profile: DeviceProfile
) -> PreparedScreen:
    """Rectify, orient, and OCR one photo of a launch-monitor screen."""
    notes: list[str] = []

    quad = find_screen_quad(image)
    if quad is None:
        notes.append("screen outline not found - parsing the photo uncropped")
        working = _resize(image)
        rectified = False
    else:
        working = _resize(warp_to_rect(image, quad))
        rectified = True

    working = enhance(working)

    best: tuple[float, int, list[TextBox]] = (-1.0, 0, [])
    for rotation in _ROTATIONS:
        candidate = _rotate(working, rotation)
        boxes = recognizer.recognize(candidate)
        ratio = _label_ratio(boxes, profile)
        if ratio > best[0]:
            best = (ratio, rotation, boxes)
        if ratio >= _GOOD_ENOUGH_LABEL_RATIO:
            break

    ratio, rotation, boxes = best
    if rotation:
        notes.append(f"image was rotated {rotation} degrees - corrected")
    if ratio < _GOOD_ENOUGH_LABEL_RATIO:
        notes.append(
            f"only {ratio:.0%} of the {profile.device} labels were legible at the best "
            "orientation - the parse below is unreliable"
        )

    return PreparedScreen(
        image=_rotate(working, rotation),
        boxes=boxes,
        rotation=rotation,
        rectified=rectified,
        label_ratio=ratio,
        notes=notes,
    )


def find_screen_quad(image: Any) -> Any | None:
    """Locate the monitor's outline, as four corners in the original image's coordinates.

    Returns `None` when nothing convincing is found — the caller then parses the photo
    uncropped, which is worse but not broken. Guessing a wrong quad *is* broken, because
    it warps the tiles into nonsense, so every candidate has to clear a size and shape
    check before it is accepted.
    """
    scale, small = _downscale_for_detection(image)
    gray = cv2.cvtColor(small, cv2.COLOR_BGR2GRAY)
    # Bilateral blur smooths the screen's own content (tile borders, the trajectory plot)
    # while keeping the strong bezel edge we actually want to trace.
    smoothed = cv2.bilateralFilter(gray, 9, 75, 75)
    frame_area = float(small.shape[0] * small.shape[1])

    fallback: Any | None = None
    for low, high in _CANNY_PRESETS:
        edges = cv2.Canny(smoothed, low, high)
        for close_size in _CLOSE_SIZES:
            kernel = cv2.getStructuringElement(cv2.MORPH_RECT, (close_size, close_size))
            closed = cv2.morphologyEx(edges, cv2.MORPH_CLOSE, kernel)
            contours, _ = cv2.findContours(closed, cv2.RETR_EXTERNAL, cv2.CHAIN_APPROX_SIMPLE)

            for contour in sorted(contours, key=cv2.contourArea, reverse=True)[:5]:
                area = cv2.contourArea(contour)
                if area < frame_area * _MIN_QUAD_AREA_RATIO:
                    break  # sorted, so every remaining contour is smaller still
                quad = _as_quad(contour, area, frame_area)
                if quad is not None:
                    return _order_quad(quad / scale)
                if fallback is None:
                    fallback = _as_rotated_rect(contour, frame_area)

    # Nothing approximated cleanly to four corners. A rotated bounding box of the biggest
    # plausible contour still beats no crop at all for a screen that is only mildly tilted.
    return _order_quad(fallback / scale) if fallback is not None else None


def _as_quad(contour: Any, area: float, frame_area: float) -> Any | None:
    """Reduce a contour to four corners, if it plausibly is a screen outline."""
    perimeter = cv2.arcLength(contour, True)
    for epsilon in _APPROX_EPSILONS:
        approx = cv2.approxPolyDP(contour, epsilon * perimeter, True)
        if len(approx) != 4 or not cv2.isContourConvex(approx):
            continue
        quad_area = cv2.contourArea(approx)
        if not _plausible_area(quad_area, frame_area):
            continue
        # A quad far larger than the contour it approximates came from an open edge
        # fragment (an L of bezel and desk, say) rather than a closed screen outline.
        if area > 0 and quad_area / area > _MAX_QUAD_FILL_RATIO:
            continue
        return approx.reshape(4, 2).astype(np.float32)
    return None


def _as_rotated_rect(contour: Any, frame_area: float) -> Any | None:
    rect = cv2.minAreaRect(contour)
    if not _plausible_area(rect[1][0] * rect[1][1], frame_area):
        return None
    return cv2.boxPoints(rect).astype(np.float32)


def _plausible_area(area: float, frame_area: float) -> bool:
    return _MIN_QUAD_AREA_RATIO <= area / frame_area <= _MAX_QUAD_AREA_RATIO


def _downscale_for_detection(image: Any) -> tuple[float, Any]:
    """Copy of `image` at the detection width, plus the scale that was applied."""
    height, width = image.shape[:2]
    if width <= _DETECT_WIDTH:
        return 1.0, image
    scale = _DETECT_WIDTH / width
    resized = cv2.resize(
        image, (_DETECT_WIDTH, max(int(height * scale), 1)), interpolation=cv2.INTER_AREA
    )
    return scale, resized


def warp_to_rect(image: Any, quad: Any) -> Any:
    """Perspective-correct the region inside `quad` to a front-on rectangle."""
    top_left, top_right, bottom_right, bottom_left = quad
    top_edge = np.linalg.norm(top_right - top_left)
    bottom_edge = np.linalg.norm(bottom_right - bottom_left)
    left_edge = np.linalg.norm(bottom_left - top_left)
    right_edge = np.linalg.norm(bottom_right - top_right)
    # The far edge of a tilted screen is foreshortened, so take the nearer (longer) one.
    width = max(int(max(top_edge, bottom_edge)), 1)
    height = max(int(max(left_edge, right_edge)), 1)

    target = np.array(
        [[0, 0], [width - 1, 0], [width - 1, height - 1], [0, height - 1]], dtype=np.float32
    )
    transform = cv2.getPerspectiveTransform(quad, target)
    return cv2.warpPerspective(image, transform, (width, height))


def enhance(image: Any) -> Any:
    """Lift local contrast so text inside dark tiles survives ceiling glare."""
    lab = cv2.cvtColor(image, cv2.COLOR_BGR2LAB)
    lightness, a_channel, b_channel = cv2.split(lab)
    clahe = cv2.createCLAHE(clipLimit=2.0, tileGridSize=(8, 8))
    merged = cv2.merge((clahe.apply(lightness), a_channel, b_channel))
    return cv2.cvtColor(merged, cv2.COLOR_LAB2BGR)


def _label_ratio(boxes: list[TextBox], profile: DeviceProfile) -> float:
    """Fraction of the profile's stored labels present — the orientation vote."""
    expected = profile.stored_fields
    if not expected:
        return 0.0
    found = {
        matched.label
        for box in boxes
        if (matched := profile.field_for(box.text)) is not None and matched.target is not None
    }
    return len(found) / len(expected)


def _resize(image: Any) -> Any:
    height, width = image.shape[:2]
    if width <= _TARGET_WIDTH:
        return image
    scale = _TARGET_WIDTH / width
    return cv2.resize(
        image, (_TARGET_WIDTH, max(int(height * scale), 1)), interpolation=cv2.INTER_AREA
    )


def _rotate(image: Any, degrees: int) -> Any:
    if degrees == 90:
        return cv2.rotate(image, cv2.ROTATE_90_CLOCKWISE)
    if degrees == 180:
        return cv2.rotate(image, cv2.ROTATE_180)
    if degrees == 270:
        return cv2.rotate(image, cv2.ROTATE_90_COUNTERCLOCKWISE)
    return image


def _order_quad(points: Any) -> Any:
    """Order four corners as top-left, top-right, bottom-right, bottom-left."""
    ordered = np.zeros((4, 2), dtype=np.float32)
    coordinate_sum = points.sum(axis=1)
    coordinate_diff = np.diff(points, axis=1).ravel()
    ordered[0] = points[np.argmin(coordinate_sum)]  # smallest x+y
    ordered[2] = points[np.argmax(coordinate_sum)]  # largest  x+y
    ordered[1] = points[np.argmin(coordinate_diff)]  # smallest y-x
    ordered[3] = points[np.argmax(coordinate_diff)]  # largest  y-x
    return ordered
