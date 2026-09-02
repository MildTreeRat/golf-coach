"""Shared test fixtures.

Sample clips and other binary fixtures live under tests/fixtures/ (gitignored if large).
Most tests should be able to run with only the base install (no vision/ML deps), because
the `contracts` seam and the mock sources have no heavy dependencies.
"""

from collections.abc import Iterator
from pathlib import Path

import pytest

from golf_coach.clubs import catalogue


@pytest.fixture(autouse=True)
def _throwaway_club_catalogue(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> Iterator[Path]:
    """Point the club catalogue at a throwaway file for **every** test in the suite. [M12 P5]

    `catalogue.remember` writes `club_catalogue.json` beside its own module, so a test that
    triggers a write without redirecting the path edits the repo's committed data and passes while
    doing it. That is not hypothetical: P5 gave the bag save route a `remember` call, and the first
    run of `tests/api/test_bag_route.py` after it committed a Titleist T150 row to the shipped
    catalogue — a test fixture's club, provenanced `"typed"`, in the file the package ships.

    Repo-wide rather than per-module because the set of modules that can reach a write is no longer
    enumerable by eye: any test exercising the bag route writes, and so will P6's. The cost of
    forgetting is a dirty working tree nobody attributes to a test run, and the only symptom of
    *not* noticing is a catalogue row that then answers every lookup of that club.

    `tests/clubs/test_catalogue.py` keeps its own fixture and its own reasons; setting the same
    attribute twice is harmless, and its `test_the_shipped_catalogue_parses…` deliberately points
    back at the real file, which still works because it re-patches after this one.
    """
    path = tmp_path / "club_catalogue.json"
    monkeypatch.setattr(catalogue, "_catalogue_path", lambda: path)
    yield path
