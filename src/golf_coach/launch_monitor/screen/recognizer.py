"""TextRecognizer port: the OCR interface the screen parser reads through.

Adapters:
  - PaddleOCRRecognizer (screen/paddle.py) — local, offline OCR   [needs the `ocr` extra]

Keeping this a Protocol is what makes the engine choice reversible. The parser only ever
sees `TextBox`es, so swapping PaddleOCR for Tesseract — or for a vision model that returns
boxes — changes one adapter and nothing downstream (ADR-008, ADR-014).

Coordinates are pixels in the *rectified* screen image (origin top-left), which is what
`preprocess` hands over. The parser only ever compares boxes to each other, so the absolute
scale is irrelevant — a 900px-wide crop and a 4000px-wide one parse identically.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any, Protocol, runtime_checkable


@dataclass(frozen=True)
class TextBox:
    """One run of recognized text and where it sat on the screen."""

    text: str
    x: float
    y: float
    width: float
    height: float
    confidence: float = 1.0

    @property
    def right(self) -> float:
        return self.x + self.width

    @property
    def bottom(self) -> float:
        return self.y + self.height

    @property
    def center_x(self) -> float:
        return self.x + self.width / 2

    @property
    def center_y(self) -> float:
        return self.y + self.height / 2


@runtime_checkable
class TextRecognizer(Protocol):
    """Turns an image into located text runs."""

    def recognize(self, image: Any) -> list[TextBox]:
        """Return every text run found in `image` (a BGR HxWx3 numpy array)."""
        ...
