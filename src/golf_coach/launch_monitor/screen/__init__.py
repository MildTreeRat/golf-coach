"""Screen-capture shot ingestion — read shot metrics off a photo of the simulator. [ADR-014]

The HD Golf simulator has no data export, so the shot numbers are recovered from a
photograph of its SHOT DATA screen. The pipeline is:

    photo -> preprocess (rectify + orient) -> TextRecognizer (OCR) -> parser -> validate
          -> store (JSON cache, keyed by image hash) -> ScreenShotDataSource

The split is deliberate. `recognizer`, `profiles`, `parser`, and `validate` are pure —
stdlib + pydantic only, no pixels — so the sign conventions and physics checks (the parts
that silently corrupt a session when wrong) are unit-testable on the base install. Only
`preprocess` (OpenCV, `vision` extra) and `paddle` (`ocr` extra) touch images (ADR-008).

`TextRecognizer` is a Protocol, so the OCR engine is swappable: if local OCR proves too
fragile on glare and off-axis photos, a vision-model adapter drops in behind it without
touching parsing, validation, caching, or the source.
"""

from golf_coach.launch_monitor.screen.importer import (
    MissingOCRExtra,
    build_recognizer,
    import_screen,
)
from golf_coach.launch_monitor.screen.parser import ParsedShot, parse_screen, to_shot_data
from golf_coach.launch_monitor.screen.profiles import DeviceProfile, load_profile
from golf_coach.launch_monitor.screen.recognizer import TextBox, TextRecognizer
from golf_coach.launch_monitor.screen.source import ScreenShotDataSource
from golf_coach.launch_monitor.screen.store import ShotStore
from golf_coach.launch_monitor.screen.validate import validate_parse

__all__ = [
    "TextBox",
    "TextRecognizer",
    "DeviceProfile",
    "load_profile",
    "ParsedShot",
    "parse_screen",
    "to_shot_data",
    "validate_parse",
    "ShotStore",
    "ScreenShotDataSource",
    "import_screen",
    "build_recognizer",
    "MissingOCRExtra",
]
