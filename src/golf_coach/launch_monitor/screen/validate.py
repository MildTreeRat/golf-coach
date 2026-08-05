"""Physics cross-checks on a parsed screen. Pure — no pixels, no I/O. [ADR-014]

OCR does not fail loudly. A dropped digit turns a 128.1-yard carry into 28.1, which is a
perfectly well-formed number that will quietly skew every session average it lands in.
The defence is that the screen prints *redundant* metrics, and the redundancy is only
self-consistent if the parse was right:

    smash factor  ==  ball speed / club speed
    shot distance ==  carry + bounce & roll

Both hold exactly on real screens (101.5 / 88.6 = 1.15; 128.1 + 23.4 = 151.5), so a
mismatch is evidence about the *parse*, not about the shot.

Range checks are the same idea at coarser grain — they exist to catch an inserted or
dropped digit, so the bounds are deliberately loose. This module never judges whether a
shot was *good*, only whether the numbers were read correctly: an odd-but-consistent
reading (a 0.89 smash factor) passes, because the parser faithfully reported what the
device displayed.
"""

from __future__ import annotations

from dataclasses import replace

from golf_coach.launch_monitor.screen.parser import ParsedShot

# Relative slack on the smash-factor identity. The screen rounds smash to 2dp, which is
# worth ~0.4% on a typical shot; 3% leaves room for that plus rounding on both speeds.
_SMASH_TOLERANCE = 0.03

# Slack on carry + bounce == total, in yards and as a fraction, whichever is larger.
_DISTANCE_TOLERANCE_YARDS = 1.0
_DISTANCE_TOLERANCE_RATIO = 0.015

# Deliberately wide: these catch "142.1 read as 1421", not an unusual swing.
_PLAUSIBLE_RANGES: dict[str, tuple[float, float]] = {
    "club_head_speed": (20.0, 200.0),
    "ball_speed": (20.0, 250.0),
    "smash_factor": (0.4, 1.7),
    "launch_angle": (-15.0, 70.0),
    "launch_direction": (-45.0, 45.0),
    "club_path": (-30.0, 30.0),
    "club_face_angle": (-30.0, 30.0),
    "spin_rate": (0.0, 15000.0),
    "spin_axis": (-45.0, 45.0),
    "carry_distance": (0.0, 450.0),
    "total_distance": (0.0, 500.0),
    "bounce_and_roll": (0.0, 150.0),
}

# How much a failed check costs. A broken identity is the strongest signal available that
# the parse is wrong, so it outweighs a merely out-of-range value.
_IDENTITY_PENALTY = 0.25
_RANGE_PENALTY = 0.15


def validate_parse(parsed: ParsedShot, min_confidence: float = 0.6) -> ParsedShot:
    """Cross-check a parse, returning a copy with warnings, confidence, and review flag.

    `needs_review` is set when any cross-check fails *or* confidence lands below
    `min_confidence` — the two catch different failures, so either alone is enough.
    """
    warnings = list(parsed.warnings)
    penalty = 0.0

    for message in _identity_failures(parsed.values):
        warnings.append(message)
        penalty += _IDENTITY_PENALTY

    for message in _range_failures(parsed.values):
        warnings.append(message)
        penalty += _RANGE_PENALTY

    confidence = max(0.0, min(1.0, parsed.confidence - penalty))
    return replace(
        parsed,
        confidence=confidence,
        warnings=warnings,
        needs_review=penalty > 0.0 or confidence < min_confidence,
    )


def _identity_failures(values: dict[str, float | str]) -> list[str]:
    """Checks that only hold when every metric involved was read correctly."""
    failures: list[str] = []

    ball = _number(values, "ball_speed")
    club = _number(values, "club_head_speed")
    smash = _number(values, "smash_factor")
    if ball is not None and club is not None and smash is not None and club > 0:
        expected = ball / club
        if abs(expected - smash) > _SMASH_TOLERANCE * max(expected, smash, 1e-6):
            failures.append(
                f"smash factor {smash:g} does not match ball speed / club speed "
                f"({ball:g} / {club:g} = {expected:.3f}) - one of the three was misread"
            )

    carry = _number(values, "carry_distance")
    roll = _number(values, "bounce_and_roll")
    total = _number(values, "total_distance")
    if carry is not None and roll is not None and total is not None:
        expected_total = carry + roll
        tolerance = max(
            _DISTANCE_TOLERANCE_YARDS, _DISTANCE_TOLERANCE_RATIO * max(abs(total), 1e-6)
        )
        if abs(expected_total - total) > tolerance:
            failures.append(
                f"shot distance {total:g} does not match carry + bounce & roll "
                f"({carry:g} + {roll:g} = {expected_total:g}) - one of the three was misread"
            )

    return failures


def _range_failures(values: dict[str, float | str]) -> list[str]:
    failures: list[str] = []
    for name, (low, high) in _PLAUSIBLE_RANGES.items():
        value = _number(values, name)
        if value is None:
            continue
        if not low <= value <= high:
            failures.append(
                f"{name} {value:g} is outside the plausible range {low:g}-{high:g} - "
                "likely a dropped or extra digit"
            )
    return failures


def _number(values: dict[str, float | str], name: str) -> float | None:
    value = values.get(name)
    return float(value) if isinstance(value, (int, float)) else None
