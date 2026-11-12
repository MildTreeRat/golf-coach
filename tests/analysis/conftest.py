"""Synthetic-swing fixtures for the analysis spine (no MediaPipe, no clips).

The builder is `golf_coach.evals.synthetic_swing.make_swing`, whose docstring says what it models
and why. It lived here until M43 P15 moved it, verbatim, so that the coach-feedback eval's case
generator could import it (`tests/` is not a package). Re-exported here, with the constants
`tests/analysis/` reads, so every `from conftest import make_swing` is unchanged and
`tests/analysis/test_conftest.py` still pins the placement it always pinned.
"""

from __future__ import annotations

import pytest

from golf_coach.contracts.keypoints import FrameKeypoints
from golf_coach.evals.synthetic_swing import (
    _ADDRESS_FRAMES,
    _ADDRESS_Y,
    _GRIP_OFFSET_X,
    _GRIP_OFFSET_Y,
    _HIP_BASE_X,
    _HIP_HALF_SPAN,
    _INDEX_FAN,
    _SHOULDER_LEFT_X,
    _SHOULDER_Y,
    make_swing,
)

__all__ = [
    "_ADDRESS_FRAMES",
    "_ADDRESS_Y",
    "_GRIP_OFFSET_X",
    "_GRIP_OFFSET_Y",
    "_HIP_BASE_X",
    "_HIP_HALF_SPAN",
    "_INDEX_FAN",
    "_SHOULDER_LEFT_X",
    "_SHOULDER_Y",
    "make_swing",
]


@pytest.fixture
def swing() -> list[FrameKeypoints]:
    """A ~3:1 (ideal-tempo) synthetic swing: steady head, held finish."""
    return make_swing()
