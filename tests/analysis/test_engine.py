"""End-to-end: analyze_swing turns a synthetic swing into a scored SwingResult."""

from __future__ import annotations

from datetime import UTC, datetime

from golf_coach.analysis import analyze_swing
from golf_coach.contracts.golfer import Handedness
from golf_coach.contracts.intent import PracticeGoal, PracticeMode
from golf_coach.contracts.keypoints import FrameKeypoints
from golf_coach.contracts.placements import (
    FACE_ON,
    PLACEMENTS_BY_NAME,
    POPULATION_PLACEMENT_REGISTRY,
    placement_names,
)
from golf_coach.contracts.shot import ShotData, ShotProvenance, ShotSource

#: The `Measurement.source` every population placement carries. Consumers partition on the registry
#: rather than on this string, but a placement recorded under a different source is one the caveats
#: describe and no reader can find.
_POPULATION = "population:golfdb"


def test_analyze_swing_produces_tempo_checkpoint_and_score(swing: list[FrameKeypoints]) -> None:
    result = analyze_swing("swing-1", "session-1", swing)

    assert result.swing_id == "swing-1"
    assert len(result.phases) == 6

    names = {cp.name for cp in result.checkpoint_scores}
    assert "tempo" in names

    # Fundamentals: overall == mechanics, outcome axis untouched.
    assert result.intent is not None
    assert result.intent.mode is PracticeMode.FUNDAMENTALS
    assert result.outcome_score is None
    assert result.mechanics_score == result.overall_score
    assert 0.0 <= result.overall_score <= 100.0
    assert result.overall_score > 0.0  # an ideal-tempo swing scores well


def test_analyze_swing_defaults_intent_to_fundamentals(swing: list[FrameKeypoints]) -> None:
    explicit = analyze_swing("s", "sess", swing, intent=PracticeGoal())
    implied = analyze_swing("s", "sess", swing)
    assert explicit.overall_score == implied.overall_score


# ------------------------------------------------------- the launch-monitor half [M9 P8]


def test_a_swing_with_a_shot_carries_both_distances(swing: list[FrameKeypoints]) -> None:
    """The registry-to-artifact path, end to end: `SHOT_MEASUREMENTS` -> `SwingResult`.

    `analysis/shot_measure.py` is only worth editing if the engine picks the row up, and the engine
    walks the registry rather than naming metrics — so this is the pin that a new row actually
    reaches a stored `analysis.json`. The source string is asserted because `storage.corpus` keys
    its honest sample counts off it: a distance recorded under `pose:face_on` would be counted per
    face-on clip rather than per shot photo (`contracts/career.py`, "two dedupe keys").
    """
    shot = ShotData(
        shot_id="shot-1",
        session_id="session-1",
        timestamp=datetime(2026, 8, 21, tzinfo=UTC),
        source=ShotSource.SCREEN,
        carry_distance=125.6,
        total_distance=131.0,
        provenance=ShotProvenance(device="hd_golf", parse_confidence=0.95),
    )

    result = analyze_swing("swing-1", "session-1", swing, shot=shot)
    distances = {m.name: m for m in result.measurements if m.name.endswith("_yds")}

    assert set(distances) == {"carry_distance_yds", "total_distance_yds"}
    assert distances["carry_distance_yds"].value == 125.6
    assert distances["total_distance_yds"].value == 131.0
    for measurement in distances.values():
        assert measurement.unit == "yards"
        assert measurement.source == "launch_monitor:hd_golf"


def test_a_swing_with_no_shot_records_no_distance(swing: list[FrameKeypoints]) -> None:
    """The direction that matters: a face-on clip alone never invents a carry.

    Same shape as the down-the-line assertion below — there was no launch monitor in the room, so
    the honest output is nothing, not a zero that pools into a mean.
    """
    result = analyze_swing("swing-1", "session-1", swing)

    assert not [m for m in result.measurements if m.name.endswith("_yds")]


# ------------------------------------------------------- the population placements [M8.3]
#
# `contracts/placements.py` is what `caveats.py` derives its placement prose from, and prose
# derived from a registry that has drifted from the engine is worse than prose that was typed:
# every assertion about it still passes. These two are the only pins that compare the registry
# against a real `analyze_swing` call.


def test_the_engine_emits_exactly_the_placements_the_registry_describes(
    swing: list[FrameKeypoints],
) -> None:
    """A face-on swing produces the face-on placements and neither down-the-line one.

    The second half is the load-bearing direction: a swing with no rear clip must not report a
    down-the-line placement, because there was no rear camera to measure one from.
    """
    result = analyze_swing("swing-1", "session-1", swing, handedness=Handedness.RIGHT)

    emitted = {m.name for m in result.measurements if m.source == _POPULATION}
    assert emitted, "no population placement was recorded on a swing the models can read"
    assert emitted <= set(placement_names()), (
        f"the engine emits {emitted - set(placement_names())} under {_POPULATION}, which the "
        "registry does not describe — so the caveats say nothing about it"
    )
    assert emitted == {s.name for s in POPULATION_PLACEMENT_REGISTRY if s.view == FACE_ON}


def test_every_emitted_placement_carries_its_registered_unit_and_a_detail(
    swing: list[FrameKeypoints],
) -> None:
    """Unit comes off the spec in `engine._placements`; this proves it still does.

    `detail` is asserted non-empty because for a placement it is not provenance but meaning — it
    carries the percentile and the population size, and `mcp/query.py` ships it for that reason.
    """
    result = analyze_swing("swing-1", "session-1", swing, handedness=Handedness.RIGHT)

    for measurement in result.measurements:
        spec = PLACEMENTS_BY_NAME.get(measurement.name)
        if spec is None:
            continue
        assert measurement.unit == spec.unit
        assert measurement.source == _POPULATION
        assert measurement.detail, f"{spec.name} carries no detail — its meaning is in that string"
