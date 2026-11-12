"""What the two stdio routes' tests share: a stored swing for the built `golf-mcp` to read.

[M29 P16–P17] Both `conversation.py`'s route and `coach.py`'s JSON entry are tested against the real
server rather than a fake one, because what they must hold is what `golf-mcp` actually answers.
The server itself is `tests/conftest.py`'s `golf_mcp`, and the writer is
`golf_coach.evals.coach_cases.store_swing`: both lived here until M43 P15 moved them, so the coach
eval's cases are written and served exactly as these tests' swings are (`tests/` is not a package).
"""

from __future__ import annotations

from collections.abc import Callable
from pathlib import Path

import pytest

from golf_coach.contracts.swing import SwingBundleResult
from golf_coach.evals.coach_cases import store_swing as _store


@pytest.fixture
def store_swing() -> Callable[[Path, SwingBundleResult], Path]:
    """Write one analyzed swing under a sessions directory, for `golf-mcp` to read back."""
    return _store
