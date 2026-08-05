"""PaddleOCRRecognizer — the local, offline OCR adapter. [ADR-014]

PaddleOCR over Tesseract for two reasons that matter here: it installs from pip with no
separate system binary (this is a Windows box), and its detection stage copes better with
the low-contrast, unevenly-lit text that a photographed screen produces.

Requires the `ocr` extra (`pip install -e '.[ocr]'`), so this module is imported directly
rather than re-exported from the package `__init__` — the parser and the shot source must
stay installable without it (ADR-008).

**On the result shape.** PaddleOCR changed its public API between 2.x (`.ocr()` returning
nested lists) and 3.x (`.predict()` returning dicts), and both are in the wild on PyPI
right now. Rather than pin a version and break on the user's next `pip install`, the
adapter normalizes whichever shape it gets — the `TextRecognizer` port is a list of
`TextBox`, and keeping that promise is this module's whole job.
"""

from __future__ import annotations

from typing import Any

from golf_coach.launch_monitor.screen.recognizer import TextBox


class PaddleOCRRecognizer:
    """Implements the `TextRecognizer` port using a local PaddleOCR model."""

    def __init__(self, lang: str = "en", engine: Any | None = None) -> None:
        """`engine` is injectable so tests can drive the normalization without the model."""
        self._lang = lang
        self._engine = engine

    @property
    def engine(self) -> Any:
        """The PaddleOCR model, loaded on first use (construction downloads weights)."""
        if self._engine is None:
            try:
                from paddleocr import PaddleOCR
            except ImportError as exc:  # pragma: no cover - depends on the install
                raise ImportError(
                    "PaddleOCR is not installed. Install the OCR extra with: "
                    "pip install -e '.[ocr]'"
                ) from exc

            # use_angle_cls corrects per-line 180° flips; whole-image orientation is
            # handled upstream in preprocess by the label-hit-rate vote.
            self._engine = PaddleOCR(use_angle_cls=True, lang=self._lang, show_log=False)
        return self._engine

    def recognize(self, image: Any) -> list[TextBox]:
        raw = self._run(image)
        return [box for entry in raw if (box := _to_text_box(entry)) is not None]

    def _run(self, image: Any) -> list[Any]:
        engine = self.engine
        if hasattr(engine, "predict"):  # PaddleOCR 3.x
            return _flatten(engine.predict(image))
        return _flatten(engine.ocr(image, cls=True))  # PaddleOCR 2.x


def _flatten(result: Any) -> list[Any]:
    """PaddleOCR wraps per-image results in an outer list; unwrap the single image."""
    if not result:
        return []
    if isinstance(result, dict):
        return _from_dict_result(result)
    if isinstance(result, list) and len(result) == 1 and isinstance(result[0], dict):
        return _from_dict_result(result[0])
    if isinstance(result, list) and len(result) == 1 and isinstance(result[0], list):
        return result[0]
    return list(result)


def _from_dict_result(result: dict[str, Any]) -> list[Any]:
    """PaddleOCR 3.x returns parallel lists of polygons, texts, and scores."""
    polygons = result.get("dt_polys") or result.get("rec_polys") or []
    texts = result.get("rec_texts") or []
    scores = result.get("rec_scores") or []
    entries: list[Any] = []
    for index, text in enumerate(texts):
        polygon = polygons[index] if index < len(polygons) else None
        score = scores[index] if index < len(scores) else 1.0
        if polygon is not None:
            entries.append([polygon, (text, score)])
    return entries


def _to_text_box(entry: Any) -> TextBox | None:
    """Normalize one `[polygon, (text, score)]` entry into a `TextBox`."""
    try:
        polygon, recognition = entry[0], entry[1]
        text, confidence = recognition[0], float(recognition[1])
        xs = [float(point[0]) for point in polygon]
        ys = [float(point[1]) for point in polygon]
    except (TypeError, IndexError, ValueError):
        return None

    if not text or not xs or not ys:
        return None

    left, right = min(xs), max(xs)
    top, bottom = min(ys), max(ys)
    return TextBox(
        text=str(text),
        x=left,
        y=top,
        width=right - left,
        height=bottom - top,
        confidence=confidence,
    )
