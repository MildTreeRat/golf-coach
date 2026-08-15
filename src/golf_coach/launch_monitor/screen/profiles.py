"""Device profiles — the only device-specific knowledge in the screen pipeline. [ADR-014]

A profile says *what to look for* (tile labels, and which `ShotData` field each feeds) and
*how to read a sign* off the qualifier the device prints beside a number — HD Golf writes
`1.6 ° O>I` and `1.1 ° Closed` where the contract wants signed degrees. It deliberately
carries no pixel coordinates: hard-coded ROIs break the moment you stand somewhere else to
take the photo, so `parser` locates cells from the labels' own geometry instead.

Data ships inside the package (`profiles.json`) and is read via `importlib.resources`,
the same pattern as the benchmark store (`analysis/benchmarks/store.py`), so adding a
second launch monitor is a data change rather than a code change.
"""

from __future__ import annotations

import json
import re
from difflib import SequenceMatcher
from functools import lru_cache
from importlib import resources

from pydantic import BaseModel, Field

_PROFILES_FILE = "profiles.json"

# Two labels are "the same" above this similarity. 0.8 absorbs the usual OCR damage
# ("Bounce & Roll" -> "Bounce &Roll", "Carry" -> "Carny") without letting "Club Speed"
# match "Club Path" (0.62) or "Spin" match "Spin Axis" (0.62 after normalization).
_LABEL_MATCH_THRESHOLD = 0.8

_NON_ALNUM = re.compile(r"[^A-Z0-9]+")


def normalize_label(text: str) -> str:
    """Uppercase, strip punctuation/whitespace — the form labels are compared in."""
    return _NON_ALNUM.sub(" ", text.upper()).strip()


class SignRule(BaseModel):
    """Qualifier tokens that set the sign of an otherwise-unsigned on-screen number."""

    tokens: list[str]
    sign: int = Field(description="+1 or -1, applied to the magnitude read from the tile.")


class ProfileField(BaseModel):
    """One tile on the screen."""

    label: str
    aliases: list[str] = Field(default_factory=list)
    target: str | None = Field(
        default=None,
        description="ShotData field this fills. None = locate it for layout only, don't store.",
    )
    kind: str = Field(default="number", description="'number' or 'text'.")
    unit: str | None = None
    sign_tokens: list[SignRule] = Field(default_factory=list)
    printed_sign: int | None = Field(
        default=None,
        description=(
            "Set when the device prints this tile's sign in the digits ('-9.3 °') rather "
            "than as a word. +1 = the printed polarity already matches the ShotData "
            "contract, -1 = it is inverted and must be negated. Left None, an explicitly "
            "signed number is still reported as sign-unknown, because a sign we cannot "
            "attribute to a stated convention is a guess."
        ),
    )

    def matches(self, text: str) -> float:
        """Similarity of `text` to this field's label or any alias, 0-1."""
        candidate = normalize_label(text)
        if not candidate:
            return 0.0
        best = 0.0
        for name in (self.label, *self.aliases):
            ratio = SequenceMatcher(None, candidate, normalize_label(name)).ratio()
            best = max(best, ratio)
        return best


class DeviceProfile(BaseModel):
    """Everything the parser needs to read one make of launch-monitor screen."""

    device: str
    title: str
    fields: list[ProfileField]
    value_span_ratio: float = Field(
        default=3.5,
        description=(
            "How far below a label its value can sit, as a multiple of the label's own "
            "height. Covers a stacked number + unit + qualifier (e.g. '1.1 °' / 'Closed')."
        ),
    )
    blank_markers: list[str] = Field(
        default_factory=lambda: ["---"],
        description="Cell contents meaning 'this device didn't measure it' -> None.",
    )

    @property
    def stored_fields(self) -> list[ProfileField]:
        """Fields that actually feed a `ShotData` attribute."""
        return [f for f in self.fields if f.target is not None]

    def field_for(self, text: str) -> ProfileField | None:
        """The field whose label best matches `text`, or None if nothing is close enough."""
        best: ProfileField | None = None
        best_score = _LABEL_MATCH_THRESHOLD
        for field in self.fields:
            score = field.matches(text)
            if score >= best_score:
                best, best_score = field, score
        return best


class _ProfileFile(BaseModel):
    profiles: list[DeviceProfile]


@lru_cache(maxsize=1)
def _load_profiles() -> dict[str, DeviceProfile]:
    raw = resources.files(__package__).joinpath(_PROFILES_FILE).read_text(encoding="utf-8")
    parsed = _ProfileFile.model_validate(json.loads(raw))
    return {p.device: p for p in parsed.profiles}


def load_profile(device: str = "hd_golf") -> DeviceProfile:
    """Load a device profile by name. Raises `KeyError` for an unknown device."""
    profiles = _load_profiles()
    if device not in profiles:
        known = ", ".join(sorted(profiles))
        raise KeyError(f"Unknown launch-monitor profile {device!r}. Known profiles: {known}")
    return profiles[device]


def available_profiles() -> list[str]:
    """Device names this build can parse."""
    return sorted(_load_profiles())
