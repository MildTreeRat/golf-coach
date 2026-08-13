"""Population placement on checkpoints: percentiles, their limits, and the scoring firewall.

`ranges.json` says *whether* a swing is in band; `golfdb_v1.json` says *where in the tour
population* it sits. These tests guard the seam between the two — above all that the second never
leaks into the first (ADR-010 §2).
"""

from __future__ import annotations

import pytest
from conftest import _ADDRESS_FRAMES, make_swing

from golf_coach.analysis.checkpoints import (
    evaluate_finish_balance,
    evaluate_head_sway,
    evaluate_tempo,
)
from golf_coach.analysis.checkpoints.mechanics import _placement_clause
from golf_coach.analysis.engine import analyze_swing
from golf_coach.analysis.phases import segment_phases
from golf_coach.analysis.smoothing import smooth_keypoints
from golf_coach.contracts.golfer import Handedness


def _all_three(head_sway: float = 0.0, finish_drift: float = 0.0):
    """Every mechanics checkpoint on one synthetic swing, mirroring the engine's order."""
    smoothed = smooth_keypoints(make_swing(30, 10, head_sway=head_sway, finish_drift=finish_drift))
    phases = segment_phases(smoothed)
    return [
        evaluate_tempo(phases),
        evaluate_head_sway(smoothed, phases),
        evaluate_finish_balance(smoothed, phases),
    ]


def test_every_checkpoint_reports_where_it_sits_in_the_population() -> None:
    for cp in _all_three():
        assert cp is not None
        assert cp.percentile is not None, f"{cp.name} resolved no distribution"
        assert 10.0 <= cp.percentile <= 90.0
        assert cp.population_n is not None and cp.population_n > 0


def test_one_sided_flag_matches_the_shape_of_the_band() -> None:
    """`one_sided` must agree with the band it was cut from, or `_tail_distance` inverts."""
    tempo, sway, balance = _all_three()
    assert tempo is not None and sway is not None and balance is not None

    assert tempo.one_sided is False
    assert tempo.expected_low is not None and tempo.expected_low > 0.0

    for cp in (sway, balance):
        assert cp.one_sided is True, f"{cp.name} should be lower-is-better"
        assert cp.expected_low == 0.0


def test_percentile_never_moves_the_score_or_the_verdict(monkeypatch: pytest.MonkeyPatch) -> None:
    """The ADR-010 §2 firewall: scoring reads `ranges.json` and nothing else.

    Blinding the evaluators to the distributions must leave `score` and `passed` bit-identical —
    if this ever fails, the reference population has crept onto the scoring path and every band
    has quietly changed meaning.
    """
    with_percentiles = _all_three(head_sway=0.5, finish_drift=0.4)

    monkeypatch.setattr(
        "golf_coach.analysis.checkpoints.mechanics.load_distribution", lambda *a, **k: None
    )
    blinded = _all_three(head_sway=0.5, finish_drift=0.4)

    for before, after in zip(with_percentiles, blinded, strict=True):
        assert before is not None and after is not None
        assert before.score == after.score
        assert before.passed == after.passed
        assert before.observed == after.observed
        assert after.percentile is None and after.population_n is None


def test_percentile_saturates_at_the_band_edge() -> None:
    """A failing checkpoint pins at the rail — which is why severity stays on `score`.

    The bands *are* the reference p10/p90 (ADR-012) and `percentile_of` clamps there, so a swing
    that misses by a hair and one that misses by triple both report 90. `feedback.rules` relies on
    this being true when it ranks failures by `score` instead.
    """
    mild = _all_three(finish_drift=0.15)[2]  # observed ~0.32 against a 0.29 ceiling
    gross = _all_three(finish_drift=0.20)[2]  # observed ~0.42
    assert mild is not None and gross is not None

    assert mild.passed is False and gross.passed is False
    assert mild.percentile == gross.percentile == 90.0
    assert gross.score < mild.score, "score must still separate what the percentile cannot"


def test_percentile_separates_two_checkpoints_that_both_scored_perfectly() -> None:
    """The converse, and the reason percentiles were wired in at all.

    Every passing checkpoint scores exactly 1.0, so `score` cannot rank them. Two swings well
    inside the band are indistinguishable by score and clearly ordered by percentile — this is
    what lets `feedback.rules` say which in-band checkpoint is closest to becoming a fault.
    """
    steady = _all_three(finish_drift=0.05)[2]
    drifty = _all_three(finish_drift=0.12)[2]
    assert steady is not None and drifty is not None

    assert steady.passed and drifty.passed
    assert steady.score == drifty.score == 1.0
    assert steady.percentile is not None and drifty.percentile is not None
    assert steady.percentile < drifty.percentile


def test_unmeasurable_checkpoint_is_named_rather_than_dropped_silently() -> None:
    """Tempo drops on an estimated address boundary (ADR-013) — the result must say so."""
    # No address dwell, so the wrist never settles and motion start is an estimate.
    moving = make_swing(20, 14, takeaway_frames=60)[_ADDRESS_FRAMES:]
    result = analyze_swing(swing_id="s", session_id="sess", keypoints=moving)

    assert "tempo" in result.unscored
    assert all(cp.name != "tempo" for cp in result.checkpoint_scores)
    # The rest still score, and the overall stays a mean over what survived (not a penalty).
    assert result.checkpoint_scores
    assert result.overall_score == pytest.approx(
        sum(cp.score for cp in result.checkpoint_scores) / len(result.checkpoint_scores) * 100.0
    )


def test_a_fully_measured_swing_leaves_nothing_unscored() -> None:
    result = analyze_swing(
        swing_id="s",
        session_id="sess",
        keypoints=make_swing(30, 10),
        handedness=Handedness.RIGHT,
    )
    assert result.unscored == []
    assert len(result.checkpoint_scores) == 6

    # `make_swing`'s default body is rigid — head and hips hold perfectly still, which no golfer
    # does — so every spatial metric measures exactly 0.0. That splits the panel cleanly along
    # `one_sided`: a one-sided band starts at zero and passes a motionless body, while both
    # two-sided bands (hip_sway, head_stays_back) require real movement and fail it. Measured and
    # failed is the opposite of unmeasured, which is what this test is actually about, and the
    # split is asserted by *shape* rather than by name so a sixth checkpoint cannot be added
    # without deciding which side of it lands on.
    by_name = {cp.name: cp for cp in result.checkpoint_scores}
    assert {name for name, cp in by_name.items() if not cp.passed} == {
        "hip_sway",
        "head_stays_back",
    }
    # Restated as the shape rule it follows from: on a motionless body a spatial checkpoint passes
    # exactly when its band is one-sided. Tempo is excluded because it is two-sided and passes —
    # it reads durations, not positions, so a rigid body has nothing to do with it.
    assert all(cp.passed is cp.one_sided for cp in by_name.values() if cp.name != "tempo")


def test_head_stays_back_is_unscored_when_nobody_said_who_swung() -> None:
    """The one checkpoint whose sign depends on identity refuses rather than assumes.

    A face-on camera mirrors a left-handed swing, so scoring this without knowing the golfer means
    reading half of them as a gross fault. Dropping it is visible — `unscored` names it — where a
    wrong guess would look exactly like a measurement.
    """
    keypoints = make_swing(30, 10)
    anonymous = analyze_swing(swing_id="s", session_id="sess", keypoints=keypoints)

    assert anonymous.unscored == ["head_stays_back"]
    assert len(anonymous.checkpoint_scores) == 5

    # The other five are untouched by the missing identity: same scores, same verdicts.
    known = analyze_swing(
        swing_id="s", session_id="sess", keypoints=keypoints, handedness=Handedness.RIGHT
    )
    shared = {cp.name: cp.score for cp in known.checkpoint_scores if cp.name != "head_stays_back"}
    assert {cp.name: cp.score for cp in anonymous.checkpoint_scores} == shared


def test_handedness_mirrors_the_head_stays_back_reading() -> None:
    """Same frames, opposite handedness: the observed value flips sign and nothing else moves.

    This is the property the whole seam exists for. The measurement is camera-relative, so the two
    golfers described here are the *same* body position seen from the two sides — and the checkpoint
    has to normalize them onto one frame before the band can mean anything.
    """
    keypoints = make_swing(30, 10)
    right = analyze_swing(
        swing_id="s", session_id="sess", keypoints=keypoints, handedness=Handedness.RIGHT
    )
    left = analyze_swing(
        swing_id="s", session_id="sess", keypoints=keypoints, handedness=Handedness.LEFT
    )

    r = next(cp for cp in right.checkpoint_scores if cp.name == "head_stays_back")
    left_score = next(cp for cp in left.checkpoint_scores if cp.name == "head_stays_back")
    assert left_score.observed == pytest.approx(-r.observed)

    # The raw measurement stays in the camera frame for both — normalizing is the judge's job, and
    # a stored measurement that silently depended on handedness could not be re-derived later.
    raw = {m.name: m.value for m in right.measurements}
    assert raw["head_hip_gain_norm"] == pytest.approx(
        next(m.value for m in left.measurements if m.name == "head_hip_gain_norm")
    )


@pytest.mark.parametrize(
    ("pct", "expected"),
    [
        (50.0, "under 50% of 100 tour swings"),  # exactly median → below branch, no rail
        (25.0, "under 75% of 100 tour swings"),
        (75.0, "over 75% of 100 tour swings"),
        (10.0, "under at least 90% of 100 tour swings"),  # clamped rail
        (90.0, "over at least 90% of 100 tour swings"),  # clamped rail
    ],
)
def test_placement_clause_names_the_share_it_beats(pct: float, expected: str) -> None:
    """The share quoted is of the population on the *other* side of the observation."""
    assert _placement_clause(pct, 100, "under", "over") == expected
