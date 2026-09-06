"""The eleventh tool: what it flies, what it refuses, and what it can see that nothing else can.

Three things are pinned here and nowhere else.

**The resolution direction.** `simulate_flight` finds a swing's shot through the photo the manifest
names, not through `read_flight_inputs`' shot-to-swing join. The corpus has one screen photo
attached to three swings, and that join names one survivor — so the join would tell two of the
three that their screen had never been read while `get_swing` returned a recorded flight for them
in the same conversation. `test_every_swing_sharing_one_photo_flies` is that case.

**The refusals are answers.** Most shots on file cannot be flown, and the tool has four different
silences — no photo, an unread photo, a refused flight, and a swing that does not exist. Only the
last is a miss.

**The M15 P14 seam.** Two of a flight's inputs live in editable artifacts, so a stored result and
today's flight can disagree with nothing on disk able to notice. This tool reads both, and the
tests below drive the two causes apart: a current artifact means an input was edited, an older one
means the engine moved.

Built from hand-written manifests and a stub source, like the rest of `tests/mcp/`: the point is
what the tool reads, so nothing here runs pose, OCR or a video decoder. The launch conditions are
`analysis/flight.py`'s own `VALIDATION_SHOTS`, so a re-sourced coefficient table moves these
numbers exactly where it moves the gate's.
"""

from __future__ import annotations

import json
from collections.abc import Iterator
from datetime import UTC, datetime
from pathlib import Path

import pytest

from golf_coach.analysis.flight import VALIDATION_SHOTS
from golf_coach.analysis.flight_measure import FLIGHT_MEASUREMENTS, FLIGHT_SOURCE
from golf_coach.contracts.club import ClubId
from golf_coach.contracts.shot import ShotData, ShotProvenance, ShotSource
from golf_coach.contracts.swing import ANALYSIS_VERSION
from golf_coach.mcp import query
from golf_coach.mcp.flight import (
    NO_SHOT_SCREEN,
    UNREAD_SHOT_SCREEN,
    flight_for_swing,
    missing_swing_flight,
)
from golf_coach.storage.manifest import Role


#: The digest `tests/mcp/conftest.py::make_manifest` writes for a shot screen. The join key, and
#: the reason a test shot has to carry a `ShotProvenance` rather than only launch numbers.
def _photo(session_id: str, swing_id: str) -> str:
    return f"sha-{session_id}-{swing_id}-{Role.SHOT_SCREEN.value}"


def _shot(
    photo_sha256: str,
    *,
    shot_id: str = "shot-1",
    spin_rate: float | None = None,
    carry_distance: float | None = None,
    **fields: float,
) -> ShotData:
    """One parsed screen, filed under the photo a manifest points at."""
    reference = VALIDATION_SHOTS[0].launch
    base = {
        "ball_speed": reference.ball_speed_mph,
        "launch_angle": reference.launch_angle_deg,
        "launch_direction": reference.launch_direction_deg,
        "spin_axis": reference.spin_axis_deg,
    }
    return ShotData(
        shot_id=shot_id,
        session_id="2026-08-10",
        source=ShotSource.SCREEN,
        timestamp=datetime(2026, 8, 10, 12, tzinfo=UTC),
        spin_rate=spin_rate,
        carry_distance=carry_distance,
        provenance=ShotProvenance(
            device="hd_golf", parse_confidence=0.9, image_sha256=photo_sha256
        ),
        **{**base, **fields},
    )


class _Source:
    """A `ShotDataSource` over a fixed list. `stream()` is the only method this tool uses."""

    def __init__(self, *shots: ShotData) -> None:
        self._shots = list(shots)

    def recent(self, count: int) -> list[ShotData]:
        return self._shots[:count]

    def stream(self) -> Iterator[ShotData]:
        return iter(self._shots)


@pytest.fixture
def flown_source() -> _Source:
    """The gate's first shot, with the spin its screen printed — so nothing is solved."""
    return _Source(
        _shot(
            _photo("2026-08-10", "1"),
            shot_id="2026-08-10-1",
            spin_rate=VALIDATION_SHOTS[0].launch.spin_rpm,
            carry_distance=VALIDATION_SHOTS[0].simulator_carry_yds,
        )
    )


# --------------------------------------------------------------------------- the happy path


def test_a_shot_with_a_printed_spin_flies(sessions_dir: Path, flown_source: _Source) -> None:
    view = flight_for_swing(sessions_dir, flown_source, "2026-08-10", "1")

    assert view is not None
    assert view.flew is True
    assert view.reason is None
    assert view.shot_id == "2026-08-10-1"
    assert view.spin is not None and view.spin.source == "measured"


def test_the_numbers_come_from_the_registry_and_carry_their_provenance(
    sessions_dir: Path, flown_source: _Source
) -> None:
    """No second copy of `FLIGHT_MEASUREMENTS` here, and no bare floats leaving the tool.

    The names are the registry's, and every row carries the unit, the source and the sentence the
    engine stored it with — which is the whole difference between this payload and the six floats
    `get_swing` used to hand a model (M15 P17).
    """
    view = flight_for_swing(sessions_dir, flown_source, "2026-08-10", "1")

    assert view is not None
    assert {row.name for row in view.simulated} <= set(FLIGHT_MEASUREMENTS)
    assert view.simulated, "a flight that flew records something"
    for row in view.simulated:
        assert row.source == FLIGHT_SOURCE
        assert row.unit and row.detail
        assert row.detail == FLIGHT_MEASUREMENTS[row.name][2]
    assert view.source == FLIGHT_SOURCE


def test_a_flight_that_flew_carries_its_caveats(sessions_dir: Path, flown_source: _Source) -> None:
    """Composed by `flight_caveats.caveats_for`, so the CLI, the page and this cannot diverge."""
    view = flight_for_swing(sessions_dir, flown_source, "2026-08-10", "1")

    assert view is not None
    assert view.caveats, "every flight on this corpus owes at least the clamp and the gate"


def test_the_printed_carry_is_paired_but_never_alone(
    sessions_dir: Path, flown_source: _Source
) -> None:
    """M15 P16's rule, on the surface that talks: the numbers never travel without `reading`."""
    view = flight_for_swing(sessions_dir, flown_source, "2026-08-10", "1")

    assert view is not None
    assert view.comparison
    for row in view.comparison:
        assert row.reading


# --------------------------------------------------------------- the finding: which direction


def test_every_swing_sharing_one_photo_flies(
    sessions_dir: Path, swing_writer, analysis_factory
) -> None:
    """⚠️ M15 P17's finding, pinned as a regression.

    `storage/flight_inputs.read_flight_inputs` answers "which swing was this shot hit on" and
    names one survivor per photo — earliest arrival wins, which is right for counting a shot once
    and wrong from this side. Three swings on the real corpus share one screen photo, and under
    that join two of them would report `UNREAD_SHOT_SCREEN` while `get_swing` returned their
    recorded flight in the same conversation.
    """
    photo = _photo("2026-08-10", "1")
    for session_id, swing_id in (("2026-08-11", "1"), ("2026-08-12", "1")):
        swing_writer(
            sessions_dir,
            session_id,
            swing_id,
            analysis=analysis_factory(session_id, swing_id),
        )
        # The same photograph, uploaded again — which is content-addressed to the same digest.
        path = sessions_dir / session_id / swing_id / "manifest.json"
        manifest = json.loads(path.read_text(encoding="utf-8"))
        manifest["roles"][Role.SHOT_SCREEN.value]["content_sha256"] = photo
        path.write_text(json.dumps(manifest), encoding="utf-8")

    source = _Source(
        _shot(
            photo,
            shot_id="2026-08-10-1",
            spin_rate=VALIDATION_SHOTS[0].launch.spin_rpm,
            carry_distance=VALIDATION_SHOTS[0].simulator_carry_yds,
        )
    )

    flew = [
        flight_for_swing(sessions_dir, source, session_id, swing_id)
        for session_id, swing_id in (("2026-08-10", "1"), ("2026-08-11", "1"), ("2026-08-12", "1"))
    ]
    assert [view is not None and view.flew for view in flew] == [True, True, True]


# --------------------------------------------------------------------------- the four silences


def test_a_refused_flight_is_an_answer_and_never_a_miss(sessions_dir: Path) -> None:
    """A printed carry no spin can reach. The commonest state on the real corpus."""
    source = _Source(
        _shot(_photo("2026-08-10", "1"), shot_id="2026-08-10-1", carry_distance=400.0)
    )

    view = flight_for_swing(sessions_dir, source, "2026-08-10", "1")

    assert view is not None
    assert view.flew is False
    assert view.reason is not None
    assert view.simulated == []
    assert view.detail, "a refusal says which case it fell into"


def test_a_refusal_is_one_entry_and_never_asks_for_another_clip(sessions_dir: Path) -> None:
    """The flight is filed under one name, and nothing about it is a capture problem.

    Six entries would report a session as five times more broken than it is — `get_session_summary`
    counts unscored entries by name — and a `refilming_helps` that were true anywhere here would
    send a golfer back to the bay for a number the video was never an input to.
    """
    source = _Source(
        _shot(_photo("2026-08-10", "1"), shot_id="2026-08-10-1", carry_distance=400.0)
    )

    view = flight_for_swing(sessions_dir, source, "2026-08-10", "1")

    assert view is not None
    assert len(view.unscored) == 1
    assert all(entry.refilming_helps is False for entry in view.unscored)
    assert all(entry.detail for entry in view.unscored)


def test_a_swing_with_no_shot_screen_says_so_without_mentioning_the_video(
    sessions_dir: Path, swing_writer, analysis_factory
) -> None:
    swing_writer(
        sessions_dir,
        "2026-08-13",
        "1",
        roles=(Role.FACE_ON,),
        analysis=analysis_factory("2026-08-13", "1"),
    )

    view = flight_for_swing(sessions_dir, _Source(), "2026-08-13", "1")

    assert view is not None
    assert view.flew is False
    assert view.detail == NO_SHOT_SCREEN
    assert view.shot_id is None


def test_an_unread_shot_screen_names_what_reads_it(sessions_dir: Path) -> None:
    """A photo on disk with no parse filed under it. No OCR runs here to fix that."""
    view = flight_for_swing(sessions_dir, _Source(), "2026-08-10", "1")

    assert view is not None
    assert view.detail == UNREAD_SHOT_SCREEN


def test_a_swing_that_does_not_exist_is_the_only_miss(sessions_dir: Path) -> None:
    assert flight_for_swing(sessions_dir, _Source(), "2026-08-10", "99") is None
    assert missing_swing_flight("2026-08-10", "99") == query.missing_swing("2026-08-10", "99")


# ------------------------------------------------------------------- the P14 seam, made visible


def _with_recorded_flight(analysis: dict, values: dict[str, float], *, version: int) -> dict:
    """An artifact carrying a stored flight, in the shape and rounding `engine.py` writes."""
    analysis["analysis_version"] = version
    analysis["swing"]["measurements"] = [
        {
            "name": name,
            "value": round(value, 4),
            "unit": FLIGHT_MEASUREMENTS[name][1],
            "source": FLIGHT_SOURCE,
            "detail": FLIGHT_MEASUREMENTS[name][2],
        }
        for name, value in values.items()
    ]
    return analysis


def _flown_values(sessions_dir: Path, source: _Source, session_id: str, swing_id: str) -> dict:
    """What this shot flies today — so a fixture records what the engine would have recorded."""
    view = flight_for_swing(sessions_dir, source, session_id, swing_id)
    assert view is not None
    return {row.name: row.value for row in view.simulated}


def _reference_source(session_id: str, swing_id: str) -> _Source:
    """The gate's first shot, filed under one swing's photo."""
    return _Source(
        _shot(
            _photo(session_id, swing_id),
            shot_id="2026-08-10-1",
            spin_rate=VALIDATION_SHOTS[0].launch.spin_rpm,
            carry_distance=VALIDATION_SHOTS[0].simulator_carry_yds,
        )
    )


def test_an_agreeing_artifact_reports_no_gap(
    sessions_dir: Path, flown_source: _Source, swing_writer, analysis_factory
) -> None:
    """The normal state, and the one that has to stay quiet or the flag means nothing."""
    source = _reference_source("2026-08-14", "1")
    swing_writer(sessions_dir, "2026-08-14", "1", analysis=analysis_factory("2026-08-14", "1"))
    flown = _flown_values(sessions_dir, source, "2026-08-14", "1")

    swing_writer(
        sessions_dir,
        "2026-08-14",
        "1",
        analysis=_with_recorded_flight(
            analysis_factory("2026-08-14", "1"), flown, version=ANALYSIS_VERSION
        ),
    )

    view = flight_for_swing(sessions_dir, source, "2026-08-14", "1")

    assert view is not None and view.flew
    assert view.differs_from_recorded == []
    assert view.recorded_reading == ""


def test_a_current_artifact_that_disagrees_reads_as_an_edited_input(
    sessions_dir: Path, swing_writer, analysis_factory
) -> None:
    """⚠️ The seam M15 P14 found and could not see from anywhere.

    The artifact is on the current engine, so the difference cannot be the engine: an input that
    lives outside `analysis.json` — the club's declared loft, the golfer's handedness — moved after
    the swing was analyzed, and neither the version check nor the upload hashes can tell.
    """
    source = _reference_source("2026-08-15", "1")
    swing_writer(
        sessions_dir, "2026-08-15", "1", club=ClubId.SEVEN_IRON, analysis=analysis_factory("x", "1")
    )
    recorded = _flown_values(sessions_dir, source, "2026-08-15", "1") | {"flight_carry_yds": 99.0}

    swing_writer(
        sessions_dir,
        "2026-08-15",
        "1",
        club=ClubId.SEVEN_IRON,
        analysis=_with_recorded_flight(
            analysis_factory("2026-08-15", "1"), recorded, version=ANALYSIS_VERSION
        ),
    )

    view = flight_for_swing(sessions_dir, source, "2026-08-15", "1")

    assert view is not None
    # One row moved, so one row is reported: the other four agree and stay quiet.
    gaps = {gap.name: gap for gap in view.differs_from_recorded}
    assert set(gaps) == {"flight_carry_yds"}
    assert gaps["flight_carry_yds"].recorded == 99.0
    assert gaps["flight_carry_yds"].flown is not None
    assert gaps["flight_carry_yds"].difference is not None
    assert "edited" in view.recorded_reading
    assert "re-analyz" in view.recorded_reading.lower()


def test_an_older_artifact_that_disagrees_reads_as_the_engine(
    sessions_dir: Path, swing_writer, analysis_factory
) -> None:
    """Same gap, different sentence — and telling a golfer their bag was edited would be wrong."""
    source = _reference_source("2026-08-16", "1")
    swing_writer(sessions_dir, "2026-08-16", "1", analysis=analysis_factory("2026-08-16", "1"))
    recorded = _flown_values(sessions_dir, source, "2026-08-16", "1") | {"flight_carry_yds": 99.0}

    swing_writer(
        sessions_dir,
        "2026-08-16",
        "1",
        analysis=_with_recorded_flight(
            analysis_factory("2026-08-16", "1"), recorded, version=ANALYSIS_VERSION - 1
        ),
    )

    view = flight_for_swing(sessions_dir, source, "2026-08-16", "1")

    assert view is not None
    assert view.differs_from_recorded
    assert "older engine" in view.recorded_reading


def test_a_refused_flight_over_a_recorded_one_is_still_a_gap(
    sessions_dir: Path, swing_writer, analysis_factory
) -> None:
    """The direction that matters most: a stored number the model would no longer produce.

    A loft that was declared when the engine ran and has since been cleared reads exactly like
    this, and reporting nothing would leave `get_swing` quoting a carry this tool refuses.
    """
    swing_writer(
        sessions_dir,
        "2026-08-17",
        "1",
        analysis=_with_recorded_flight(
            analysis_factory("2026-08-17", "1"),
            {"flight_carry_yds": 122.0},
            version=ANALYSIS_VERSION,
        ),
    )
    source = _Source(
        _shot(_photo("2026-08-17", "1"), shot_id="2026-08-10-1", carry_distance=400.0)
    )

    view = flight_for_swing(sessions_dir, source, "2026-08-17", "1")

    assert view is not None and view.flew is False
    gaps = {gap.name: gap for gap in view.differs_from_recorded}
    assert gaps["flight_carry_yds"].flown is None
    assert gaps["flight_carry_yds"].recorded == 122.0
