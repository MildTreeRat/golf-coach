"""Turn recognized text boxes into a shot. Pure — no pixels, no I/O. [ADR-014]

The HD Golf screen is a grid of tiles, each a label with its value stacked underneath
("Carry" / "128.1" / "yds"). Rather than hard-code where those tiles sit — which breaks
the moment the photo is framed differently — this module *finds* the labels, groups them
into rows, derives each tile's column from its neighbours, and reads whatever text falls
in the cell below. A profile supplies the labels and the sign rules; the geometry is
worked out per image.

The two things worth being careful about, because getting either wrong produces a number
that is plausible and false:

  - **Signs.** The screen prints magnitudes with a word: `1.6 ° O>I`, `1.1 ° Closed`,
    `2.6 ° L`. The contract wants signed degrees (+ = in-to-out, + = open, + = right), so
    the qualifier is what decides the sign. A missing qualifier is reported, not guessed.
  - **Blanks.** `---` means the device did not measure it. That must become `None`, never 0.
"""

from __future__ import annotations

import re
from dataclasses import dataclass, field
from datetime import datetime

from golf_coach.contracts.shot import ShotData, ShotProvenance, ShotSource
from golf_coach.launch_monitor.screen.profiles import (
    DeviceProfile,
    ProfileField,
    normalize_label,
)
from golf_coach.launch_monitor.screen.recognizer import TextBox

# Two labels belong to the same row when their centers sit within this fraction of the
# taller one's height. Generous enough for OCR jitter, tight enough that the two label
# rows of the HD Golf grid never merge.
_ROW_TOLERANCE = 0.7

# Within a cell, boxes whose centers are closer together than this fraction of the label
# height count as the same line of text, and so read left-to-right.
_LINE_BUCKET = 0.6

# A number, optionally signed, with either a decimal point or thousands separators.
_NUMBER = re.compile(r"[-+]?\d[\d,]*(?:\.\d+)?")
_THOUSANDS = re.compile(r"(?<=\d),(?=\d{3}\b)")
# What survives into sign-token matching: letters, digits, and the '>' of "O>I".
_SIGN_NOISE = re.compile(r"[^A-Z0-9>]")

# Weights for the three independent things that can go wrong: we found the wrong screen,
# we found the labels but not their values, or the OCR engine itself was unsure.
_W_LABELS, _W_VALUES, _W_OCR = 0.35, 0.35, 0.30

# A profile field paired with the box its label was recognized in.
_Located = tuple["ProfileField", TextBox]


@dataclass
class ParsedShot:
    """One screen's worth of extracted metrics, with everything needed to audit it."""

    device: str
    values: dict[str, float | str] = field(default_factory=dict)
    raw_fields: dict[str, str] = field(default_factory=dict)
    confidence: float = 0.0
    warnings: list[str] = field(default_factory=list)
    # Set by `validate_parse`, not by parsing: the parser reports what it saw, the
    # validator decides whether to trust it.
    needs_review: bool = False

    @property
    def is_empty(self) -> bool:
        return not self.values


@dataclass(frozen=True)
class _Cell:
    """A located tile: its label box and the text found underneath it."""

    profile_field: ProfileField
    label_box: TextBox
    value_boxes: tuple[TextBox, ...]

    @property
    def text(self) -> str:
        """The cell's value text, read line by line, left to right."""
        bucket = max(self.label_box.height * _LINE_BUCKET, 1e-6)
        ordered = sorted(self.value_boxes, key=lambda b: (int(b.center_y // bucket), b.center_x))
        return " ".join(b.text.strip() for b in ordered if b.text.strip())


def parse_screen(boxes: list[TextBox], profile: DeviceProfile) -> ParsedShot:
    """Extract shot metrics from one screen's recognized text."""
    parsed = ParsedShot(device=profile.device)

    labels = _find_labels(boxes, profile)
    if not labels:
        parsed.warnings.append(
            f"no {profile.device} tile labels recognized - wrong screen, or the crop failed"
        )
        return parsed

    if not _has_title(boxes, profile) and _title_band_was_captured(boxes, labels):
        parsed.warnings.append(
            f"screen title {profile.title!r} not found - is this the right page?"
        )

    label_box_ids = {id(box) for _, box in labels}
    cells = _build_cells(labels, boxes, label_box_ids, profile)

    used: list[TextBox] = []
    for cell in cells:
        used.append(cell.label_box)
        used.extend(cell.value_boxes)
        _read_cell(cell, profile, parsed)

    for missing in _missing_labels(cells, profile):
        parsed.warnings.append(f"no tile found for {missing!r}")

    parsed.confidence = _score(cells, used, profile)
    return parsed


def to_shot_data(
    parsed: ParsedShot,
    *,
    shot_id: str,
    session_id: str,
    timestamp: datetime,
    image_sha256: str | None = None,
    image_path: str | None = None,
) -> ShotData:
    """Assemble a `ShotData`, carrying the parse audit trail in `provenance`."""
    return ShotData(
        shot_id=shot_id,
        session_id=session_id,
        timestamp=timestamp,
        source=ShotSource.SCREEN,
        provenance=ShotProvenance(
            device=parsed.device,
            parse_confidence=round(parsed.confidence, 3),
            needs_review=parsed.needs_review,
            warnings=list(parsed.warnings),
            image_sha256=image_sha256,
            image_path=image_path,
            raw_fields=dict(parsed.raw_fields),
        ),
        **parsed.values,
    )


# --- locating tiles -------------------------------------------------------------------


def _has_title(boxes: list[TextBox], profile: DeviceProfile) -> bool:
    """Is the screen's title anywhere on it? Compared with the spaces taken out.

    HD Golf sets the title in wide-tracked capitals, and PaddleOCR returns it as the
    single token `SHOTDATA` — so a substring test against `SHOT DATA` failed on every
    real photo the repo has ever parsed, while passing on the synthetic fixtures, whose
    title string was written by hand *with* the space. The letters are what identify the
    page; the kerning the OCR infers between them is not evidence about anything.
    """
    want = _despace(profile.title)
    return any(want in _despace(box.text) for box in boxes)


def _despace(text: str) -> str:
    return normalize_label(text).replace(" ", "")


def _title_band_was_captured(boxes: list[TextBox], labels: list[_Located]) -> bool:
    """Was there anything above the top row of tiles for the title to be *in*?

    The title sits above the grid, and `preprocess.rectify` crops to the screen — which
    on the reference photos lands below it. A title missing from a frame that starts at
    the first tile row is a fact about the crop, not about the page, and warning on it
    means warning on every correctly-parsed photo. So the check answers "we looked where
    it would be and it was not there", which is the only form of it that is evidence.
    """
    top_of_grid = min(box.y for _, box in labels)
    return any(box.bottom <= top_of_grid for box in boxes)


def _find_labels(boxes: list[TextBox], profile: DeviceProfile) -> list[_Located]:
    """Best box for each profile label. A label the OCR saw twice resolves to one tile."""
    best: dict[str, tuple[float, TextBox]] = {}
    for box in boxes:
        matched = profile.field_for(box.text)
        if matched is None:
            continue
        score = matched.matches(box.text)
        current = best.get(matched.label)
        if current is None or score > current[0]:
            best[matched.label] = (score, box)

    by_label = {f.label: f for f in profile.fields}
    return [(by_label[label], box) for label, (_, box) in best.items()]


def _group_rows(labels: list[_Located]) -> list[list[_Located]]:
    """Cluster labels into screen rows by vertical position."""
    rows: list[list[_Located]] = []
    for item in sorted(labels, key=lambda pair: pair[1].center_y):
        _, box = item
        placed = False
        for row in rows:
            reference = row[-1][1]
            tolerance = max(reference.height, box.height) * _ROW_TOLERANCE
            if abs(box.center_y - reference.center_y) <= tolerance:
                row.append(item)
                placed = True
                break
        if not placed:
            rows.append([item])
    for row in rows:
        row.sort(key=lambda pair: pair[1].center_x)
    return rows


def _build_cells(
    labels: list[_Located],
    boxes: list[TextBox],
    label_box_ids: set[int],
    profile: DeviceProfile,
) -> list[_Cell]:
    """Derive each tile's bounds from its neighbours, then collect the text inside it."""
    rows = _group_rows(labels)
    cells: list[_Cell] = []

    for row_index, row in enumerate(rows):
        next_row = rows[row_index + 1] if row_index + 1 < len(rows) else ()
        next_row_top = min((b.y for _, b in next_row), default=None)
        for col_index, (profile_field, label_box) in enumerate(row):
            # Column: halfway to the neighbouring tiles, so a wide value can overhang its
            # label without being claimed by the tile next door.
            left = (
                (row[col_index - 1][1].right + label_box.x) / 2
                if col_index > 0
                else label_box.x - label_box.width
            )
            right = (
                (label_box.right + row[col_index + 1][1].x) / 2
                if col_index + 1 < len(row)
                else label_box.right + label_box.width
            )
            # Rows: down to the next row of labels, but never further than a value could
            # plausibly stack (number + unit + qualifier).
            bottom = label_box.bottom + label_box.height * profile.value_span_ratio
            if next_row_top is not None:
                bottom = min(bottom, next_row_top)

            inside = tuple(
                box
                for box in boxes
                if id(box) not in label_box_ids
                and left <= box.center_x <= right
                and label_box.bottom <= box.center_y <= bottom
            )
            cells.append(_Cell(profile_field, label_box, inside))
    return cells


def _missing_labels(cells: list[_Cell], profile: DeviceProfile) -> list[str]:
    found = {cell.profile_field.label for cell in cells}
    return [f.label for f in profile.stored_fields if f.label not in found]


# --- reading a tile -------------------------------------------------------------------


def _read_cell(cell: _Cell, profile: DeviceProfile, parsed: ParsedShot) -> None:
    profile_field = cell.profile_field
    if profile_field.target is None:  # boundary-only tile (e.g. the settings gear)
        return

    text = cell.text
    parsed.raw_fields[profile_field.label] = text

    if not text:
        parsed.warnings.append(f"{profile_field.label}: no value text under the label")
        return
    if _is_blank(text, profile):
        return  # the device reported nothing here — a correct read of an absent metric

    if profile_field.kind == "text":
        parsed.values[profile_field.target] = " ".join(text.upper().split())
        return

    number = _first_number(text)
    if number is None:
        parsed.warnings.append(f"{profile_field.label}: could not read a number from {text!r}")
        return

    magnitude, matched_text = number
    if profile_field.sign_tokens or profile_field.printed_sign is not None:
        sign = _sign_from(text.replace(matched_text, " ", 1), profile_field)
        if sign is not None:
            # A direction word beats a printed sign: it states the convention in the
            # device's own words, where a bare '-' only states one if we already know
            # which way the device points it.
            magnitude = abs(magnitude) * sign
        elif matched_text[0] in "+-" and profile_field.printed_sign is not None:
            # The tile printed its own sign, and the profile records how that polarity
            # relates to the contract. Nothing is unknown here, so nothing is warned.
            magnitude *= profile_field.printed_sign
        else:
            parsed.warnings.append(
                f"{profile_field.label}: no direction word in {text!r} - "
                "sign unknown, storing the magnitude as printed"
            )
    parsed.values[profile_field.target] = magnitude


def _is_blank(text: str, profile: DeviceProfile) -> bool:
    compact = re.sub(r"\s+", "", text)
    return any(compact == re.sub(r"\s+", "", marker) for marker in profile.blank_markers)


def _first_number(text: str) -> tuple[float, str] | None:
    """First number in `text`, with the exact substring it came from."""
    match = _NUMBER.search(text)
    if match is None:
        return None
    raw = match.group()
    try:
        return float(_THOUSANDS.sub("", raw).replace(",", ".")), raw
    except ValueError:
        return None


def _sign_from(residual: str, profile_field: ProfileField) -> int | None:
    """Read the direction word left over after the number, e.g. 'O>I' or 'Closed'."""
    compact = _SIGN_NOISE.sub("", residual.upper())
    if not compact:
        return None
    for rule in profile_field.sign_tokens:
        for token in rule.tokens:
            wanted = _SIGN_NOISE.sub("", token.upper())
            if not wanted:
                continue
            # Single letters ('L'/'R') must stand alone, or 'L' would match 'CLOSED'.
            hit = compact == wanted if len(wanted) == 1 else wanted in compact
            if hit:
                return rule.sign
    return None


# --- confidence -----------------------------------------------------------------------


def _score(cells: list[_Cell], used: list[TextBox], profile: DeviceProfile) -> float:
    """Blend three independent failure modes into one 0-1 trust score.

    Kept separate on purpose: a clean OCR pass on the wrong screen, and a right screen the
    engine could barely read, are both untrustworthy for different reasons, and averaging
    coverage with engine confidence is what catches each of them.
    """
    expected = len(profile.stored_fields)
    if expected == 0:
        return 0.0

    found_labels = {c.profile_field.label for c in cells if c.profile_field.target is not None}
    read_values = sum(1 for c in cells if c.profile_field.target is not None and c.text)
    ocr = [b.confidence for b in used]

    label_coverage = len(found_labels) / expected
    value_coverage = min(read_values / expected, 1.0)
    ocr_confidence = sum(ocr) / len(ocr) if ocr else 0.0

    score = _W_LABELS * label_coverage + _W_VALUES * value_coverage + _W_OCR * ocr_confidence
    return max(0.0, min(1.0, score))
