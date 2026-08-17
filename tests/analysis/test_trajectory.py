"""The trajectory model: the shared feature builder, and the placement it feeds.

Like `test_joint.py` and `test_distributions.py`, these run against the **real** committed
`trajectory_model_v1.json` rather than a fixture — it ships inside the wheel, and the failure worth
catching is someone re-deriving it into a shape the loader cannot read.

They also run on a base install with no extras, which is the whole point of the split: the basis is
*fitted* by `scripts/golfdb/derive_trajectory_model.py` under the `research` extra and *evaluated*
here by stdlib arithmetic (ADR-022).
"""

from __future__ import annotations

import math

import pytest
from conftest import make_swing

from golf_coach.analysis.benchmarks import (
    load_trajectory_model,
    trajectory_dataset_info,
    trajectory_placement_for,
)
from golf_coach.analysis.benchmarks.trajectory import (
    DOWN_THE_LINE,
    FACE_ON,
    placement_from_anchors,
)
from golf_coach.analysis.phases import segment_phases
from golf_coach.analysis.trajectory import (
    LANDMARK_INDEX,
    anchors_from_phases,
    build_trajectory,
    sample_positions,
)
from golf_coach.contracts.swing import PhaseSegment, SwingPhase


def _phases(swing):
    return segment_phases(swing)


def test_dataset_provenance_is_present() -> None:
    info = trajectory_dataset_info()
    assert info.name == "GolfDB"
    assert "McNally" in info.citation
    # The licensing posture must travel with the artifact, not live only in an ADR (ADR-012 §2).
    assert "aggregate" in info.license_note.lower()


def test_the_artifact_shapes_agree() -> None:
    model = load_trajectory_model()
    assert model.dimensions == model.steps * len(model.landmarks) * len(model.axes)
    assert len(model.mean) == model.dimensions
    assert len(model.basis) == model.components
    assert len(model.scale) == model.components
    assert all(len(row) == model.dimensions for row in model.basis)
    assert all(s > 0 for s in model.scale)


def test_every_landmark_the_artifact_names_is_one_we_can_find() -> None:
    """The artifact stores landmark *names*; the builder maps them to indices.

    A name the map does not know would make `build_trajectory` return None for every swing, so the
    model would be silently inert rather than wrong — the worst failure mode of the two.
    """
    for name in load_trajectory_model().landmarks:
        assert name in LANDMARK_INDEX


def test_the_basis_is_orthonormal() -> None:
    """PCA rows must stay unit-length and mutually perpendicular or the projection is not one."""
    basis = load_trajectory_model().basis
    for i, row in enumerate(basis):
        assert math.isclose(sum(v * v for v in row), 1.0, abs_tol=1e-3)
        for other in basis[i + 1 :]:
            assert abs(sum(a * b for a, b in zip(row, other, strict=True))) < 1e-3


def test_population_size_is_recorded_and_plausible() -> None:
    model = load_trajectory_model()
    assert model.n >= 300
    assert model.n_players >= 100
    assert 0.0 < model.explained_variance <= 1.0


def test_the_artifact_carries_its_own_validation() -> None:
    """Both exceedance figures ship inside the file, because Q is *not* calibrated.

    A consumer reading Q without knowing it over-flags golfers the basis never saw would describe a
    different person as an abnormal swing. That fact belongs next to the numbers, not in a doc.
    """
    exceedance = load_trajectory_model().leave_one_player_out_exceedance
    assert exceedance["target"] == 0.10
    assert 0.05 < exceedance["t2"] < 0.15
    assert exceedance["q"] > exceedance["target"]


# ------------------------------------------------------------------ the shared feature builder


def test_sample_positions_land_on_the_anchors() -> None:
    """Event time is the contract: the first, middle and last samples are the anchors themselves."""
    positions = sample_positions((10.0, 30.0, 50.0), 41)
    assert positions[0] == pytest.approx(10.0)
    assert positions[20] == pytest.approx(30.0)
    assert positions[-1] == pytest.approx(50.0)


def test_sample_positions_absorb_slow_motion() -> None:
    """A clip at twice the frame rate yields samples at twice the frame indices — same swing."""
    real = sample_positions((0.0, 20.0, 40.0), 21)
    slow = sample_positions((0.0, 40.0, 80.0), 21)
    assert [2 * p for p in real] == pytest.approx(slow)


def test_anchors_come_out_in_order() -> None:
    anchors = anchors_from_phases(_phases(make_swing()))
    assert anchors is not None
    assert anchors[0] < anchors[1] < anchors[2]


def test_anchors_refuse_when_a_phase_is_missing() -> None:
    partial = [
        PhaseSegment(
            phase=SwingPhase.ADDRESS, start_frame=0, end_frame=5, start_ms=0.0, end_ms=50.0
        )
    ]
    assert anchors_from_phases(partial) is None


def test_anchors_refuse_a_collapsed_window() -> None:
    """Non-increasing anchors mean detection failed; resampling them would divide by zero."""
    flat = [
        PhaseSegment(phase=p, start_frame=7, end_frame=7, start_ms=0.0, end_ms=0.0)
        for p in (SwingPhase.ADDRESS, SwingPhase.TRANSITION, SwingPhase.IMPACT)
    ]
    assert anchors_from_phases(flat) is None


def test_the_vector_has_the_length_the_artifact_expects() -> None:
    model = load_trajectory_model()
    swing = make_swing()
    anchors = anchors_from_phases(_phases(swing))
    assert anchors is not None
    vector = build_trajectory(swing, anchors, model.steps, model.landmarks, model.axes)
    assert vector is not None
    assert len(vector) == model.dimensions


def test_mirroring_twice_is_the_identity() -> None:
    """The fold has to be an involution, or a left-handed corpus drifts a little on every pass."""
    model = load_trajectory_model()
    swing = make_swing()
    anchors = anchors_from_phases(_phases(swing))
    assert anchors is not None

    once = build_trajectory(
        swing, anchors, model.steps, model.landmarks, model.axes, mirror=True
    )
    plain = build_trajectory(swing, anchors, model.steps, model.landmarks, model.axes)
    assert once is not None and plain is not None
    assert once != plain, "mirroring a swing should change it"

    # Mirroring is a swap plus a sign flip on x, so applying the same rule to the mirrored vector
    # must return the original. Done via the model's own landmark order so the test cannot pass by
    # agreeing with a private helper about that order.
    from golf_coach.analysis.trajectory import _mirror

    per_axis = len(model.axes)
    columns = [
        [once[t * len(model.landmarks) * per_axis + c] for t in range(model.steps)]
        for c in range(len(model.landmarks) * per_axis)
    ]
    back = _mirror(columns, model.landmarks, model.axes)
    flat = [
        back[i * per_axis + j][t]
        for t in range(model.steps)
        for i in range(len(model.landmarks))
        for j in range(per_axis)
    ]
    assert flat == pytest.approx(plain)


# ------------------------------------------------------------------------------- the placement


def test_a_synthetic_swing_places_without_error() -> None:
    swing = make_swing()
    placement = trajectory_placement_for(swing, _phases(swing))
    assert placement is not None
    assert placement.t2 >= 0.0
    assert placement.q >= 0.0
    assert 10.0 <= placement.t2_percentile <= 90.0
    assert 10.0 <= placement.q_percentile <= 90.0


def test_residual_shares_sum_to_one_and_name_the_interval() -> None:
    model = load_trajectory_model()
    swing = make_swing()
    placement = trajectory_placement_for(swing, _phases(swing))
    assert placement is not None
    assert math.isclose(sum(placement.residual_by_interval.values()), 1.0, abs_tol=1e-3)
    # One key per gap between anchors — this is the "when" the scalar model cannot express.
    assert len(placement.residual_by_interval) == len(model.events) - 1


def test_placement_refuses_when_phases_are_unusable() -> None:
    """No number beats a wrong one (ADR-010 §2)."""
    assert trajectory_placement_for(make_swing(), []) is None


def test_a_wilder_swing_is_never_placed_closer_than_a_tamer_one() -> None:
    """More head sway must not *reduce* the distance from the tour shape."""
    tame = make_swing(head_sway=0.0)
    wild = make_swing(head_sway=0.30)
    tame_placement = trajectory_placement_for(tame, _phases(tame))
    wild_placement = trajectory_placement_for(wild, _phases(wild))
    assert tame_placement is not None and wild_placement is not None
    assert wild_placement.t2 >= tame_placement.t2


def test_placement_is_not_reachable_from_the_scoring_path() -> None:
    """The firewall, written while it is still trivially true.

    A population placement must never become an input to a score (ADR-010 §2) — the same rule
    `test_population.py` pins for percentiles and `test_joint.py` for the joint model.
    """
    import golf_coach.analysis.checkpoints.mechanics as mechanics
    import golf_coach.analysis.scoring as scoring

    for module in (mechanics, scoring):
        public = {name for name in dir(module) if not name.startswith("_")}
        assert "trajectory" not in public, (
            f"{module.__name__} has grown a reference to the trajectory model"
        )


# ------------------------------------------------------------------ the two views are two models


def test_both_views_load_and_are_distinct_models() -> None:
    face = load_trajectory_model(FACE_ON)
    dtl = load_trajectory_model(DOWN_THE_LINE)
    assert face.landmarks != dtl.landmarks
    assert face.n > 0 and dtl.n > 0


def test_the_down_the_line_model_drops_the_lead_arm() -> None:
    """The measured reason the two views cannot share a landmark list.

    From behind, the lead arm crosses the body and the torso hides it — elbow and wrist track in
    under half of frames (M4_POSE_BAKEOFF §Phase G). Handing the face-on list to a down-the-line
    fit discards 72% of the corpus, so this is not a stylistic difference and must not drift back.
    """
    dtl = load_trajectory_model(DOWN_THE_LINE)
    assert "left_elbow" not in dtl.landmarks
    assert "left_wrist" not in dtl.landmarks
    # The trail arm is what survives, and the feet take the freed slots.
    assert "right_wrist" in dtl.landmarks
    assert "left_ankle" in dtl.landmarks


def test_every_landmark_either_model_names_is_one_we_can_find() -> None:
    """A name absent from `LANDMARK_INDEX` makes `build_trajectory` return None for *every* swing.

    That is silent rather than loud — it is how the first down-the-line fit produced 0 usable
    clips — so the map has to cover the union of both views' lists, not whichever came first.
    """
    for view in (FACE_ON, DOWN_THE_LINE):
        for name in load_trajectory_model(view).landmarks:
            assert name in LANDMARK_INDEX, f"{name} ({view}) is not in LANDMARK_INDEX"


def test_an_unknown_view_raises_rather_than_falling_back() -> None:
    with pytest.raises(KeyError):
        load_trajectory_model("worms-eye")


def test_placement_from_anchors_matches_the_phase_path() -> None:
    """The two entry points must agree, or the bundle scores a different swing than it aligns.

    `analyze_swing_bundle` reuses the anchors it already computed rather than segmenting a second
    time; that is only sound while both routes build the same vector.
    """
    swing = make_swing()
    phases = _phases(swing)
    anchors = anchors_from_phases(phases)
    assert anchors is not None

    model = load_trajectory_model(FACE_ON)
    via_phases = model.placement_for(swing, phases)
    via_anchors = placement_from_anchors(swing, anchors, view=FACE_ON)
    assert via_phases is not None and via_anchors is not None
    assert via_anchors.t2 == pytest.approx(via_phases.t2)
    assert via_anchors.q == pytest.approx(via_phases.q)


def test_the_trajectory_anchors_agree_with_the_alignment_anchors() -> None:
    """Two functions read the same three instants off the same phases; pin them to each other.

    `alignment.anchors_from_phases` builds the warp's anchors and `trajectory.anchors_from_phases`
    builds the model's. They are independent implementations of one rule, and the bundle path
    feeds one into the other — if they drift, the trajectory is read off different frames than the
    warp pins to, which nothing else would catch.
    """
    from golf_coach.analysis.alignment import anchors_from_phases as alignment_anchors

    phases = _phases(make_swing())
    mine = anchors_from_phases(phases)
    theirs = alignment_anchors(phases)
    assert mine is not None and theirs is not None
    assert int(mine[0]) == theirs.motion_start
    assert int(mine[1]) == theirs.top
    assert int(mine[2]) == theirs.impact
