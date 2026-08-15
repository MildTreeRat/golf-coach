"""Synthetic HD Golf screens, built from the two real reference photos.

The parser only ever sees `TextBox`es, so a screen can be reconstructed in pure Python:
these fixtures lay out the same tile grid, with the same strings the OCR engine reads off
`data/raw/shot_screens/`, at plausible coordinates. That keeps the tests that guard the
sign conventions and the physics checks — the parts that silently corrupt a session when
wrong — runnable on the base install, with no OCR engine and no images (ADR-008).

*The same strings* is load-bearing, and was once only nearly true: the title default read
`SHOT DATA Player 1` where PaddleOCR returns the wide-tracked banner as one token,
`SHOTDATA`. The title check was a substring test, so it passed here and failed on every
real photo the repo had ever parsed. A fixture that is kinder than the OCR engine tests
the parser against a screen that does not exist — so when one of these strings and the
engine disagree, the engine is right.
"""

from __future__ import annotations

import pytest

from golf_coach.launch_monitor.screen.profiles import DeviceProfile, load_profile
from golf_coach.launch_monitor.screen.recognizer import TextBox

# Tile grid geometry. Only the relationships matter — the parser derives every cell from
# the labels it finds, so the absolute scale here is arbitrary.
TILE_WIDTH = 160.0
LABEL_HEIGHT = 20.0
VALUE_HEIGHT = 24.0
ROW_PITCH = 150.0
FIRST_LABEL_Y = 100.0
VALUE_GAP = 15.0
LINE_PITCH = 30.0

Row = list[tuple[str, list[str]]]


def build_screen(
    rows: list[Row], *, title: str = "SHOTDATA Aaron", confidence: float = 0.95
) -> list[TextBox]:
    """Lay out `rows` of (label, value lines) as located text, label above value."""
    boxes = [TextBox(title, 10.0, 40.0, 220.0, LABEL_HEIGHT, confidence)]
    for row_index, row in enumerate(rows):
        label_y = FIRST_LABEL_Y + row_index * ROW_PITCH
        for column, (label, lines) in enumerate(row):
            boxes.append(_centered(label, column, label_y, LABEL_HEIGHT, 7.0, confidence))
            for line_index, line in enumerate(lines):
                y = label_y + LABEL_HEIGHT + VALUE_GAP + line_index * LINE_PITCH
                boxes.append(_centered(line, column, y, VALUE_HEIGHT, 9.0, confidence))
    return boxes


def _centered(
    text: str, column: int, y: float, height: float, char_width: float, confidence: float
) -> TextBox:
    width = min(len(text) * char_width, TILE_WIDTH - 20.0)
    x = column * TILE_WIDTH + (TILE_WIDTH - width) / 2
    return TextBox(text, x, y, width, height, confidence)


# The two reference photos in data/raw/shot_screens/, transcribed tile by tile.
SCREEN_2738: list[Row] = [
    [
        ("Shot Distance", ["151.5", "yds"]),
        ("Carry", ["128.1", "yds"]),
        ("Bounce & Roll", ["23.4", "yds"]),
        ("Ball Speed", ["101.5", "mph"]),
        ("Launch Angle", ["10.2 °"]),
        ("Club Speed", ["88.6", "mph"]),
        ("Club Path", ["1.6 ° O>I"]),
        ("Custom", []),
    ],
    [
        ("Club Face Angle", ["1.1 °", "Closed"]),
        ("Spin", ["---"]),
        ("Smash Factor", ["1.15"]),
        ("Impact Position", ["CENTER"]),
        ("Shot Type", ["SLIGHT", "DRAW"]),
        ("Horizontal Angle", ["2.6 ° L"]),
        ("Spin Axis", ["---"]),
    ],
]

SCREEN_2739: list[Row] = [
    [
        ("Shot Distance", ["226.3", "yds"]),
        ("Carry", ["195.9", "yds"]),
        ("Bounce & Roll", ["30.4", "yds"]),
        ("Ball Speed", ["142.1", "mph"]),
        ("Launch Angle", ["5.4 °"]),
        ("Club Speed", ["159.5", "mph"]),
        ("Club Path", ["6.8 ° I>O"]),
        ("Custom", []),
    ],
    [
        ("Club Face Angle", ["5.0 °", "Closed"]),
        ("Spin", ["---"]),
        ("Smash Factor", ["0.89"]),
        ("Impact Position", ["CENTER"]),
        ("Shot Type", ["DRAW"]),
        ("Horizontal Angle", ["1.8 ° R"]),
        ("Spin Axis", ["---"]),
    ],
]


@pytest.fixture
def profile() -> DeviceProfile:
    return load_profile("hd_golf")
