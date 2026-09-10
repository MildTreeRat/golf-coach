"""End-to-end: analyze_swing turns a synthetic swing into a scored SwingResult."""

from __future__ import annotations

from datetime import UTC, datetime

from golf_coach.analysis import analyze_swing
from golf_coach.contracts.career import MODEL_SOURCE_PREFIX
from golf_coach.contracts.checkpoints import checkpoint_names
from golf_coach.contracts.golfer import Handedness
from golf_coach.contracts.intent import PracticeGoal, PracticeMode
from golf_coach.contracts.keypoints import FrameKeypoints
from golf_coach.contracts.pivots import (
    PIVOT_MEASUREMENT_REGISTRY,
    PIVOTS_BY_NAME,
)
from golf_coach.contracts.placements import (
    FACE_ON,
    PLACEMENTS_BY_NAME,
    POPULATION_PLACEMENT_REGISTRY,
    placement_names,
)
from golf_coach.contracts.shot import ShotData, ShotProvenance, ShotSource
from golf_coach.contracts.unscored import UnscoredReason

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


# ------------------------------------------------------- the pivot paths [M17 P5]
#
# The half `tests/contracts/test_pivots.py` could not assert: that the ten registry rows are rows
# the engine actually emits. A registry naming a metric nothing computes is internally consistent
# and wrong about the data, and until this section existed nothing in the suite could tell.


def test_the_engine_emits_exactly_the_face_on_pivots_the_registry_describes(
    swing: list[FrameKeypoints],
) -> None:
    """Five rows in registry order, and not one of the `_dtl` five.

    The second half is the load-bearing direction, as it is for the placements above: there was no
    rear camera in this call, so a down-the-line rotation number would be a reading of a clip that
    does not exist. Order is asserted rather than membership because `contracts/pivots.py` says the
    registry's order is the recorded order and `feedback/coach.py` renders them in it.
    """
    result = analyze_swing("swing-1", "session-1", swing, handedness=Handedness.RIGHT)

    emitted = [m.name for m in result.measurements if m.name in PIVOTS_BY_NAME]

    assert emitted == [s.name for s in PIVOT_MEASUREMENT_REGISTRY if s.view == FACE_ON]
    assert not [name for name in emitted if name.endswith("_dtl")]


def test_every_emitted_pivot_carries_its_registered_unit_detail_and_the_face_on_source(
    swing: list[FrameKeypoints],
) -> None:
    """Unit and detail come off the spec in `engine._pivot_measurements`; this proves it still does.

    The source is asserted for the reason the distances' and the flight's are, and here it decides
    something the others do not: `CorpusSwing.artifact_key` keys a `pose:`-prefixed row on the
    face-on clip's hash, which is exactly right for these five and exactly wrong for their `_dtl`
    partners — `contracts/career.py::POSE_DTL_SOURCE` is the branch that separates them, and a
    face-on row that quietly acquired that source would pool into no baseline at all.
    """
    result = analyze_swing("swing-1", "session-1", swing, handedness=Handedness.RIGHT)

    pivots = [m for m in result.measurements if m.name in PIVOTS_BY_NAME]
    assert pivots, "no pivot row was recorded on a swing the producer can read"

    for measurement in pivots:
        spec = PIVOTS_BY_NAME[measurement.name]
        assert measurement.unit == spec.unit
        assert measurement.detail == spec.detail
        assert measurement.source == "pose:face_on"


def test_a_pivot_is_measured_and_never_judged(swing: list[FrameKeypoints]) -> None:
    """ADR-010 §2's firewall, at the one boundary that could breach it.

    A rotation checkpoint needs a calibrated instrument and a population and M17 has neither, so
    these ten ship as `Measurement`s. Nothing here is in `CHECKPOINT_REGISTRY`, nothing reaches
    `overall_score`, and a refused check leaves an *absence* rather than an `unscored` row —
    `unscored` is a list of checkpoints, and a swing judged on fewer fundamentals is what it means.
    """
    result = analyze_swing("swing-1", "session-1", swing, handedness=Handedness.RIGHT)

    assert set(PIVOTS_BY_NAME).isdisjoint(checkpoint_names())
    assert not [c for c in result.checkpoint_scores if c.name in PIVOTS_BY_NAME]
    assert not [entry for entry in result.unscored if entry.name in PIVOTS_BY_NAME]


def test_an_unsegmentable_swing_records_no_pivot_rather_than_a_flat_one(
    swing: list[FrameKeypoints],
) -> None:
    """A still golfer has no three instants to resample between, and zero is not the answer.

    Three static frames give `segment_phases` nothing to find, so `anchors_from_phases` refuses and
    the pivot paths never get built. Every check here is an excursion or a reversal *magnitude*, so
    a swing recorded as all-zeroes would read as the cleanest turn in the corpus rather than as the
    absence it is — the same hazard `_placements` avoids by returning None on the same input.
    """
    still = [swing[0].model_copy(update={"frame_index": i}) for i in range(3)]

    result = analyze_swing("swing-1", "session-1", still)

    assert not [m for m in result.measurements if m.name in PIVOTS_BY_NAME]


# ------------------------------------------------------- the simulated flight [M15 P11]


def _flyable(**fields) -> ShotData:
    """`2026-08-23-4` as the screen printed it — the one shot on disk whose spin the solve names."""
    base = dict(
        shot_id="shot-1",
        session_id="session-1",
        timestamp=datetime(2026, 8, 23, tzinfo=UTC),
        source=ShotSource.SCREEN,
        ball_speed=89.8,
        launch_angle=22.4,
        launch_direction=2.8,
        carry_distance=126.1,
        provenance=ShotProvenance(device="hd_golf", parse_confidence=0.95),
    )
    return ShotData(**{**base, **fields})


def test_a_swing_with_a_loft_carries_the_simulated_flight(swing: list[FrameKeypoints]) -> None:
    """The second registry-to-artifact pin: `FLIGHT_MEASUREMENTS` -> `SwingResult`.

    The source is asserted for the reason the distances' is: `CorpusSwing.artifact_key` partitions
    on the prefix, and `model:` is the fourth one — registered in M15 P12, where it keys on the
    shot photo the launch conditions were read off. A source string edited here without editing
    that dispatch is a flight silently counted per swing again, so the prefix is asserted against
    the constant the corpus reader actually tests rather than against a literal.
    """
    result = analyze_swing(
        "swing-1",
        "session-1",
        swing,
        shot=_flyable(),
        handedness=Handedness.RIGHT,
        loft_deg=30.5,
    )
    flight = {m.name: m for m in result.measurements if m.name.startswith("flight_")}

    assert set(flight) == {
        "flight_carry_yds",
        "flight_apex_yds",
        "flight_descent_angle_deg",
        "flight_time_s",
        "flight_spin_rpm",
    }
    for measurement in flight.values():
        assert measurement.source == "model:flight_v1"
        assert measurement.source.startswith(MODEL_SOURCE_PREFIX)
    # No axis on the screen, so the curve is not drawn and the landing point is withheld — it would
    # be `start_line_offline_yds` under a second name (ADR-027's 2026-09-05i addendum).
    assert "flight_landing_offline_yds" not in flight
    assert [entry.name for entry in result.unscored if entry.name.startswith("flight_")] == [
        "flight_landing_offline_yds"
    ]


def test_no_loft_costs_the_flight_and_nothing_else(swing: list[FrameKeypoints]) -> None:
    """The corpus's own state on six of the thirteen shots, and it must not touch the score.

    `overall_score` is a mean over the checkpoints that scored. Nothing in the ball flight is a
    checkpoint, so a swing that could not be flown has to grade identically to one that could.
    """
    flown = analyze_swing(
        "s", "sess", swing, shot=_flyable(), handedness=Handedness.RIGHT, loft_deg=30.5
    )
    grounded = analyze_swing(
        "s", "sess", swing, shot=_flyable(), handedness=Handedness.RIGHT, loft_deg=None
    )

    assert grounded.overall_score == flown.overall_score
    assert not [m for m in grounded.measurements if m.name.startswith("flight_")]
    refusals = [entry for entry in grounded.unscored if entry.name.startswith("flight_")]
    assert [entry.name for entry in refusals] == ["flight_carry_yds"]
    assert refusals[0].reason is UnscoredReason.NO_CLUB_LOFT


def test_a_swing_with_no_shot_attempts_no_flight(swing: list[FrameKeypoints]) -> None:
    """No launch conditions were ever going to arrive, so a refusal here would be noise.

    The pipeline gates its "tag a club" note the same way, and for the same reason: a repair for a
    thing that was not attempted is worse than silence.
    """
    result = analyze_swing("s", "sess", swing, handedness=Handedness.RIGHT, loft_deg=30.5)

    assert not [m for m in result.measurements if m.name.startswith("flight_")]
    assert not [entry for entry in result.unscored if entry.name.startswith("flight_")]


def test_a_printed_spin_flies_without_any_loft_at_all(swing: list[FrameKeypoints]) -> None:
    """⚠️ The fact `api/pipeline`'s loft note is gated on, pinned where it is actually true.

    Loft picks a branch of the *spin solve*, so a shot whose screen printed a spin never reaches
    for it. The two 2026-08-10 swings on disk are exactly that — no club tagged, and they fly — and
    narrating the loft gap up front told them to go and tag a club so their ball flight could be
    simulated, printed beside the simulated ball flight. `NO_CLUB_LOFT` is the engine saying the
    loft was reached for and missing, and it is the only evidence that repair is a repair.
    """
    result = analyze_swing(
        "s",
        "sess",
        swing,
        shot=_flyable(spin_rate=5991.0, spin_axis=2.5),
        handedness=Handedness.RIGHT,
        loft_deg=None,
    )

    assert [m.name for m in result.measurements if m.name == "flight_carry_yds"]
    assert not [
        entry for entry in result.unscored if entry.reason is UnscoredReason.NO_CLUB_LOFT
    ]
