"""Rectification and orientation, without needing an OCR engine.

`preprocess` is the stage that decides whether OCR gets a clean, upright grid of tiles or
a skewed photo of a room, so it is worth testing on its own. A stub recognizer stands in
for the real engine: it "reads" the profile's labels only when the image is the right way
up, which is exactly the signal the rotation search votes on.

Needs the `vision` extra for OpenCV, not the `ocr` extra — the OCR engine is never called.
"""

from __future__ import annotations

import importlib.util

import pytest

from golf_coach.launch_monitor.screen.profiles import load_profile
from golf_coach.launch_monitor.screen.recognizer import TextBox

pytestmark = pytest.mark.skipif(
    importlib.util.find_spec("cv2") is None, reason="needs the `vision` extra (OpenCV)"
)

# A corner marker painted into the true top-left. After a rotation it is somewhere else,
# which is how the stub recognizer knows the image is not upright.
_MARKER = (0, 0, 255)


@pytest.fixture
def np():
    return pytest.importorskip("numpy")


@pytest.fixture
def cv2():
    return pytest.importorskip("cv2")


def _photo(np, cv2, *, screen_fraction: float = 0.6, rotate: int = 0):
    """A dark 'screen' rectangle on a light 'room' background, optionally rotated."""
    image = np.full((900, 1200, 3), 210, dtype=np.uint8)  # the room
    height, width = image.shape[:2]
    margin_x = int(width * (1 - screen_fraction) / 2)
    margin_y = int(height * (1 - screen_fraction) / 2)
    cv2.rectangle(
        image,
        (margin_x, margin_y),
        (width - margin_x, height - margin_y),
        (30, 30, 30),
        thickness=-1,
    )
    cv2.rectangle(image, (margin_x, margin_y), (margin_x + 40, margin_y + 40), _MARKER, -1)
    if rotate:
        image = cv2.rotate(
            image,
            {90: cv2.ROTATE_90_CLOCKWISE, 180: cv2.ROTATE_180, 270: cv2.ROTATE_90_COUNTERCLOCKWISE}[
                rotate
            ],
        )
    return image


class _OrientationSensitiveRecognizer:
    """Returns the profile's labels only when the marker is back in the top-left."""

    def __init__(self, profile) -> None:
        self._profile = profile
        self.calls = 0

    def recognize(self, image) -> list[TextBox]:
        self.calls += 1
        top_left = image[: image.shape[0] // 3, : image.shape[1] // 3]
        if not (top_left[..., 2] > 200).any():  # the red marker is not up here
            return []
        return [
            TextBox(field.label, 10.0 + index * 60, 10.0, 50.0, 12.0, 0.9)
            for index, field in enumerate(self._profile.stored_fields)
        ]


def test_finds_the_screen_outline_in_a_photo(np, cv2) -> None:
    from golf_coach.launch_monitor.screen.preprocess import find_screen_quad, warp_to_rect

    image = _photo(np, cv2)

    quad = find_screen_quad(image)

    assert quad is not None
    warped = warp_to_rect(image, quad)
    # The crop is the screen, not the room: the light background is gone.
    assert warped.mean() < image.mean()
    assert 0.4 < (warped.shape[1] / image.shape[1]) < 0.85


def test_a_blank_frame_yields_no_quad_rather_than_a_wrong_one(np, cv2) -> None:
    """No crop is recoverable; a fabricated crop warps the tiles into nonsense."""
    from golf_coach.launch_monitor.screen.preprocess import find_screen_quad

    assert find_screen_quad(np.full((600, 800, 3), 128, dtype=np.uint8)) is None


def test_a_contour_tracing_the_whole_frame_is_rejected(np, cv2) -> None:
    from golf_coach.launch_monitor.screen.preprocess import find_screen_quad

    # A "screen" filling 99% of the frame is indistinguishable from the photo border.
    assert find_screen_quad(_photo(np, cv2, screen_fraction=0.995)) is None


@pytest.mark.parametrize("rotation", [0, 90, 180, 270])
def test_orientation_is_recovered_from_the_content(np, cv2, rotation: int) -> None:
    from golf_coach.launch_monitor.screen.preprocess import prepare_screen

    profile = load_profile("hd_golf")
    recognizer = _OrientationSensitiveRecognizer(profile)

    prepared = prepare_screen(_photo(np, cv2, rotate=rotation), recognizer, profile)

    assert prepared.label_ratio == 1.0
    assert len(prepared.boxes) == len(profile.stored_fields)
    if rotation:
        assert any("rotated" in note for note in prepared.notes)


def test_the_upright_case_short_circuits_the_rotation_search(np, cv2) -> None:
    """An upright photo must not pay for four OCR passes."""
    from golf_coach.launch_monitor.screen.preprocess import prepare_screen

    profile = load_profile("hd_golf")
    recognizer = _OrientationSensitiveRecognizer(profile)

    prepare_screen(_photo(np, cv2), recognizer, profile)

    assert recognizer.calls == 1


def test_an_unreadable_screen_is_reported_not_hidden(np, cv2) -> None:
    from golf_coach.launch_monitor.screen.preprocess import prepare_screen

    class _Blind:
        def recognize(self, image) -> list[TextBox]:
            return []

    prepared = prepare_screen(_photo(np, cv2), _Blind(), load_profile("hd_golf"))

    assert prepared.label_ratio == 0.0
    assert any("legible" in note for note in prepared.notes)
