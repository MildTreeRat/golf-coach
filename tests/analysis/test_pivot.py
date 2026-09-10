"""The pivot producer and the rule checks over it. [M17 P3, P4]

Three halves, and the file grew the third one. The first is the index space — that
`PIVOT_SAMPLES` observations come back and that the middle one really is the top of the backswing,
which is `tests/contracts/test_pivots.py`'s odd-sample-count pin seen from the producer's side.
The second is the refusals: `pivot_observations` returns `None` rather than a partial path, and
every gate that produces one is reachable from the synthetic fixture.

The orientation assertions are the ones with teeth. A collapsed line reads `None` and never an
angle, and the reason it must is that `atan2` is worst-conditioned exactly where the projected
shoulder line vanishes — so a check reading degrees off it would fire hardest on the cleanest
turns (ADR-029 addendum §4).

The third half is P4's: each check fires on the fault it is named for and stays quiet on the other
two faults, the whole panel is unchanged by mirroring the golfer, and the refusals a check may
report are drawn from `MEASUREMENT_REASONS` rather than invented.
"""

from __future__ import annotations

import math

import pytest
from conftest import make_swing

from golf_coach.analysis.phases import LEAD_WRIST
from golf_coach.analysis.pivot import (
    MAX_UNORIENTED,
    PIVOT_CHECKS,
    pivot_observations,
)
from golf_coach.contracts.keypoints import FrameKeypoints, PoseLandmark
from golf_coach.contracts.pivots import (
    BACKSWING_SPAN,
    DOWNSWING_SPAN,
    PIVOT_MEASUREMENT_REGISTRY,
    PIVOT_SAMPLES,
    FrameOfReference,
    PivotObservation,
)
from golf_coach.contracts.unscored import MEASUREMENT_REASONS, UnscoredReason

# `make_swing(30, 10)`'s own three instants, as frame indices: eight frames of address dwell, a
# thirty-frame rise whose last frame *is* the top, then a ten-frame descent ending at impact.
# Passed explicitly rather than routed through `segment_phases`, because the tests below need the
# top anchor to be an exact frame — `anchors_from_phases` returns the midpoint of the TRANSITION
# window, which brackets it and lands between frames. `test_the_fixture_still_tops_out_where_this
# _file_says` is what notices if a fixture retune moves them.
_ADDRESS, _TOP, _IMPACT = 8.0, 37.0, 47.0
_ANCHORS = (_ADDRESS, _TOP, _IMPACT)

# The turn `_turn_profile` gives the fixture's shoulders, and the hitch `_turned_shoulders` can
# break it with. 40 degrees is a projected face-on shoulder line at the top rather than an
# anatomical turn — it only has to be big and clean enough that reading it as a fault would be
# obvious. The hitch is held for several frames because the resampling lands ~1.45 frames apart
# through the backswing, so a one-frame spike would be interpolated into two half-sized ones.
_TURN_DEG = 40.0
_HITCH_DEG = 25.0
_HITCH_FRAMES = 6


def _observations(
    swing: list[FrameKeypoints],
    anchors: tuple[float, float, float] = _ANCHORS,
    view: FrameOfReference = FrameOfReference.IMAGE_PLANE_FACE_ON,
):
    return pivot_observations(swing, anchors, frame_of_reference=view)


def _flatten_shoulders(swing: list[FrameKeypoints], lo: int, hi: int) -> list[FrameKeypoints]:
    """Collapse the shoulder line onto one point across an inclusive frame span.

    What a face-on camera sees at the top of a full turn: the trail shoulder swings behind the
    lead one and the line the golfer rotates about has no width left to measure.
    """
    for frame in swing[lo : hi + 1]:
        frame.landmarks[PoseLandmark.RIGHT_SHOULDER] = frame.landmarks[
            PoseLandmark.LEFT_SHOULDER
        ].model_copy()
    return swing


def _blind(swing: list[FrameKeypoints], landmark: PoseLandmark) -> list[FrameKeypoints]:
    """Drop one landmark below the confidence gate for the whole clip."""
    for frame in swing:
        frame.landmarks[landmark] = frame.landmarks[landmark].model_copy(
            update={"visibility": 0.0}
        )
    return swing


def test_the_fixture_still_tops_out_where_this_file_says() -> None:
    """Guards the three constants above against a retune of `make_swing`."""
    swing = make_swing(30, 10)
    peak = min(range(len(swing)), key=lambda i: swing[i].landmarks[LEAD_WRIST].y)

    assert peak == int(_TOP)
    assert _IMPACT < len(swing) - 1


def test_the_ideal_swing_resamples_to_the_registered_sample_count() -> None:
    """`PIVOT_SAMPLES` observations, every line readable, every point placed."""
    observations = _observations(make_swing(30, 10))

    assert observations is not None
    assert len(observations) == PIVOT_SAMPLES
    assert all(o.shoulder_line is not None for o in observations)
    assert all(o.hip_line is not None for o in observations)
    assert all(o.club_head is None for o in observations), "M17 detects no club"


def test_the_frame_of_reference_is_the_one_the_caller_declared() -> None:
    """It is a field on every observation because two views are two instruments, never blended."""
    for view in (FrameOfReference.IMAGE_PLANE_FACE_ON, FrameOfReference.IMAGE_PLANE_DTL):
        observations = _observations(make_swing(30, 10), view=view)
        assert observations is not None
        assert all(o.frame_of_reference is view for o in observations)


def test_one_producer_serves_both_views_without_branching_on_them() -> None:
    """The same frames read as either view give the same numbers — the view only names them.

    The seam, from the producer's side: nothing here is view-specific, which is why
    `analysis/engine.py` needs one helper rather than two and why a `CALIBRATED_3D` source can
    replace this one without the checks noticing.
    """
    face_on = _observations(make_swing(30, 10), view=FrameOfReference.IMAGE_PLANE_FACE_ON)
    dtl = _observations(make_swing(30, 10), view=FrameOfReference.IMAGE_PLANE_DTL)

    assert face_on is not None and dtl is not None
    assert [o.hip for o in face_on] == [o.hip for o in dtl]
    assert [o.shoulder_line for o in face_on] == [o.shoulder_line for o in dtl]


def test_the_pose_producer_cannot_claim_to_be_calibrated() -> None:
    """A wiring bug, not a data condition: the caller passes a literal (`contracts.spec_for`)."""
    with pytest.raises(ValueError, match="calibrated"):
        _observations(make_swing(30, 10), view=FrameOfReference.CALIBRATED_3D)


def test_the_middle_sample_is_the_top_of_the_backswing() -> None:
    """The odd-sample-count contract, from the data's side rather than the arithmetic's.

    The fixture's hands are highest at exactly one frame, and that frame is the top anchor. With an
    even `PIVOT_SAMPLES` no sample would land on it, the highest observation would sit either side
    of the middle, and `BACKSWING_SPAN[-1]` would be an ordinary mid-backswing instant that the
    reversal checks then split the swing at.
    """
    observations = _observations(make_swing(30, 10))
    assert observations is not None

    hands = [o.hands for o in observations]
    assert all(hand is not None for hand in hands), "the fixture's wrists are readable throughout"

    # Image coordinates: the smallest `y` is the highest the hands go.
    highest = min(range(PIVOT_SAMPLES), key=lambda i: hands[i][1])  # type: ignore[index]
    assert highest == BACKSWING_SPAN[-1] == DOWNSWING_SPAN[0]


def test_the_address_hip_centre_is_the_origin() -> None:
    """Every coordinate is measured from one fixed point, and that point is sample 0's hip centre.

    Fixed, not per-sample. A per-sample hip origin is what `build_trajectory` means by
    hip-relative, and it would put the hip at `(0, 0)` in every observation — zeroing the travel
    `hip_axis_drift` and `hip_path_jitter` exist to measure.
    """
    swayed = _observations(make_swing(30, 10, hip_sway=0.08))
    assert swayed is not None

    assert swayed[0].hip == pytest.approx((0.0, 0.0))
    assert max(abs(o.hip[0]) for o in swayed) > 0.1, "a swaying hip must move off its own origin"


def test_the_scale_is_one_ruler_and_it_is_the_shoulder_width() -> None:
    """The fixture's shoulders span a known width, so the normalised geometry is predictable.

    The shoulder centre sits above the hip centre by 0.2 in frame units against a 0.16 shoulder
    span, so it lands ~1.25 shoulder widths above the origin. Pinning a *derived* distance rather
    than the scale itself is what makes this fail if the ruler is ever swapped for a different
    quantity.
    """
    observations = _observations(make_swing(30, 10))
    assert observations is not None

    assert observations[0].shoulder[1] == pytest.approx((0.4 - 0.6) / 0.16, rel=1e-3)


def test_a_collapsed_shoulder_line_reads_none_rather_than_an_angle() -> None:
    """The line vanishes at the top; the observation says so instead of reporting jitter.

    The path itself survives — the shoulder *centre* is still a point, the hips are untouched, and
    the ruler is a median over the samples rather than a mean, so a handful of collapsed frames
    cannot take the whole swing down with them.
    """
    swing = _flatten_shoulders(make_swing(30, 10), 30, 44)
    observations = _observations(swing)

    assert observations is not None
    assert observations[BACKSWING_SPAN[-1]].shoulder_line is None
    assert observations[0].shoulder_line is not None
    assert all(o.hip_line is not None for o in observations)


def test_a_swing_with_no_shoulder_width_at_all_refuses() -> None:
    """The ruler gate fires first: with no scale there is no normalised path to report at all."""
    assert _observations(_flatten_shoulders(make_swing(30, 10), 0, 55)) is None


def test_an_unreadable_landmark_refuses_rather_than_inventing_a_path() -> None:
    """Hips below the confidence gate for the whole clip are not bridgeable — `MAX_MISSING`."""
    assert _observations(_blind(make_swing(30, 10), PoseLandmark.LEFT_HIP)) is None


def test_unreadable_wrists_cost_the_hands_and_nothing_else() -> None:
    """⚠️ The defect M17 P5's first corpus run found, pinned. [M17 P5]

    The hands are drawn and measured by nothing, and they are also the worst-tracked of the three
    points — M14 P3 put face-on hand tracking at 0.63-0.68 over a whole clip against 1.00 over the
    address window. Under one shared `_interpolate_gaps` call they took the swing down with them:
    44-61% of the resampled samples on the fifteen stored swings had no readable wrist pair while
    the shoulder and hip midpoints read on **every** one, so every face-on swing in the corpus
    recorded no rotation number at all — a landmark nothing measures vetoing the five metrics that
    ignore it.

    The assertion that matters is the second one. A test that only checked `hands is None` would
    pass on a producer that had quietly stopped placing the other two as well.
    """
    blind_hands = _blind(make_swing(30, 10), PoseLandmark.LEFT_WRIST)
    blind_hands = _blind(blind_hands, PoseLandmark.RIGHT_WRIST)

    observations = _observations(blind_hands)
    intact = _observations(make_swing(30, 10))

    assert observations is not None and intact is not None
    assert all(o.hands is None for o in observations)
    assert [o.shoulder for o in observations] == [o.shoulder for o in intact]
    assert [o.hip for o in observations] == [o.hip for o in intact]


def test_collapsed_or_disordered_anchors_refuse() -> None:
    """A window that did not open means detection failed; resampling it divides by zero.

    Checked here and not left to the caller because the down-the-line caller builds its three
    anchors from a `SwingAnchors` by hand — `anchors_from_phases` is the only path that has
    already made this check.
    """
    swing = make_swing(30, 10)

    assert _observations(swing, (_TOP, _TOP, _IMPACT)) is None
    assert _observations(swing, (_IMPACT, _TOP, _ADDRESS)) is None


def test_anchors_outside_the_clip_refuse() -> None:
    """Sampling past the last frame would hold the final pose and call it a swing."""
    swing = make_swing(30, 10)

    assert _observations(swing, (-1.0, _TOP, _IMPACT)) is None
    assert _observations(swing, (_ADDRESS, _TOP, float(len(swing)))) is None


def test_too_few_frames_refuse() -> None:
    assert _observations(make_swing(30, 10)[:1], (0.0, 0.5, 1.0)) is None


# ------------------------------------------------------------------------------ the rule checks


def _scores(swing: list[FrameKeypoints]) -> dict[str, float | None]:
    """Every check's value over one swing, keyed by check name. `None` where it refused."""
    observations = _observations(swing)
    assert observations is not None
    return {name: check(observations).value for name, check in PIVOT_CHECKS.items()}


def _measured(swing: list[FrameKeypoints]) -> dict[str, float]:
    """`_scores`, having asserted that nothing refused — for the tests that do arithmetic."""
    scores = _scores(swing)
    assert all(value is not None for value in scores.values())
    return {name: value for name, value in scores.items() if value is not None}


def _turn_profile(count: int, *, hitch_at: int | None = None) -> list[float]:
    """Shoulder-line angle per frame: square at address, 40 degrees at the top, square at impact.

    The fixture's shoulders are nailed to one horizontal line, which is what makes the ideal swing
    score exact zeros below — good as a pin and useless as a subject. This turns them, so the two
    reversal checks have a real turn to be monotone about.

    `hitch_at` drops the line back by `_HITCH_DEG` for `_HITCH_FRAMES` and then resumes the ramp:
    a golfer whose shoulders unwind mid-backswing and re-turn. The step back is one against-the-turn
    move between adjacent frames; the step out of it runs *with* the turn and must not count.
    """
    angles = []
    for i in range(count):
        if i <= _ADDRESS:
            angles.append(0.0)
        elif i <= _TOP:
            angles.append(_TURN_DEG * (i - _ADDRESS) / (_TOP - _ADDRESS))
        elif i <= _IMPACT:
            angles.append(_TURN_DEG * (1.0 - (i - _TOP) / (_IMPACT - _TOP)))
        else:
            angles.append(0.0)
    if hitch_at is not None:
        for i in range(hitch_at, min(hitch_at + _HITCH_FRAMES, count)):
            angles[i] -= _HITCH_DEG
    return angles


def _turned_shoulders(swing: list[FrameKeypoints], angles: list[float]) -> list[FrameKeypoints]:
    """Rotate each frame's shoulder pair about its own midpoint by that frame's angle.

    About the midpoint, so the shoulder *centre* never moves: a turn must not leak into
    `shoulder_axis_drift`, or the isolation these tests claim would be an artifact of the fixture.
    """
    for frame, angle in zip(swing, angles, strict=True):
        left = frame.landmarks[PoseLandmark.LEFT_SHOULDER]
        right = frame.landmarks[PoseLandmark.RIGHT_SHOULDER]
        mid_x, mid_y = (left.x + right.x) / 2, (left.y + right.y) / 2
        half = (right.x - left.x) / 2
        dx, dy = half * math.cos(math.radians(angle)), half * math.sin(math.radians(angle))
        frame.landmarks[PoseLandmark.LEFT_SHOULDER] = left.model_copy(
            update={"x": mid_x - dx, "y": mid_y - dy}
        )
        frame.landmarks[PoseLandmark.RIGHT_SHOULDER] = right.model_copy(
            update={"x": mid_x + dx, "y": mid_y + dy}
        )
    return swing


def _jittered_hips(swing: list[FrameKeypoints], amplitude: float) -> list[FrameKeypoints]:
    """Shake both hips sideways by `+/-amplitude` on alternating frames.

    A path that is rough without going anywhere: the excursion stays about one amplitude while the
    sample-to-sample bend is four times it, which is how the roughness check can be told apart from
    the drift one at all.
    """
    for frame in swing:
        shift = amplitude if frame.frame_index % 2 else -amplitude
        for landmark in (PoseLandmark.LEFT_HIP, PoseLandmark.RIGHT_HIP):
            hip = frame.landmarks[landmark]
            frame.landmarks[landmark] = hip.model_copy(update={"x": hip.x + shift})
    return swing


def _mirrored(swing: list[FrameKeypoints]) -> list[FrameKeypoints]:
    """The same swing seen from the other side: every `x` reflected, no landmark identity changed.

    What a face-on camera does to a left-handed golfer, and the reason `analysis/engine.py`'s
    docstring warns about it. Reflection negates every signed quantity the checks look at — an
    excursion's direction, a bend's direction, a turn's direction — so an unsigned panel must not
    move by a digit.
    """
    for frame in swing:
        for index, landmark in enumerate(frame.landmarks):
            frame.landmarks[index] = landmark.model_copy(update={"x": 1.0 - landmark.x})
    return swing


def _wonky(hitch_at: int = 22) -> list[FrameKeypoints]:
    """One synthetic carrying all three faults at once — the mirroring subject."""
    swing = make_swing(30, 10, hip_sway=0.08)
    _turned_shoulders(swing, _turn_profile(len(swing), hitch_at=hitch_at))
    return _jittered_hips(swing, 0.004)


def _flat(observations: list[PivotObservation], span: range) -> float:
    """The fraction of one span whose shoulder line has collapsed."""
    return sum(1 for i in span if observations[i].shoulder_line is None) / len(span)


def _still(
    index: int, view: FrameOfReference = FrameOfReference.IMAGE_PLANE_FACE_ON
) -> PivotObservation:
    """One hand-built observation, a shoulder width further along `x` per index.

    Hand-built rather than sliced off a real path, because the shapes these guard against are ones
    `pivot_observations` cannot produce — a path of one sample, or two instruments in one list.
    """
    return PivotObservation(
        shoulder=(float(index), 1.0),
        hip=(float(index), 0.0),
        hands=(float(index), 0.5),
        shoulder_line=(1.0, 0.0),
        hip_line=(1.0, 0.0),
        frame_of_reference=view,
    )


def test_the_check_table_is_exactly_what_the_registry_names() -> None:
    """Both directions. A spec naming a missing check and a check nothing names are one gap.

    Keyed by `PivotMeasurementSpec.check` and never by metric name: ten rows name five checks,
    because a face-on spec and its `_dtl` partner share an implementation that does not know which
    camera it is reading.
    """
    named = {spec.check for spec in PIVOT_MEASUREMENT_REGISTRY}

    assert set(PIVOT_CHECKS) == named
    assert len(PIVOT_CHECKS) == 5 and len(PIVOT_MEASUREMENT_REGISTRY) == 2 * len(PIVOT_CHECKS)


def test_the_ideal_swing_is_clean_on_every_check() -> None:
    """Nothing slides, nothing shakes and nothing turns back, so every number is zero.

    Exactly zero rather than small, because the fixture's shoulders are nailed to one line and its
    hips are steady by construction. That makes this a pin on the checks' *arithmetic* — an
    off-by-one or a first difference where a second belongs shows up here — and it is the fault
    cases below that give them teeth.
    """
    assert _scores(make_swing(30, 10)) == dict.fromkeys(PIVOT_CHECKS, 0.0)


def test_a_full_monotone_turn_is_not_a_reversal() -> None:
    """Forty degrees each way and both reversal checks read zero.

    The hazard ADR-029's addendum §4 raised, from the check's side: a turn is the *signal*, and a
    rule that mistook a large clean one for a fault would fire hardest on the best swings.
    """
    swing = make_swing(30, 10)
    scores = _scores(_turned_shoulders(swing, _turn_profile(len(swing))))

    assert scores["shoulder_reversal_backswing"] == 0.0
    assert scores["shoulder_reversal_downswing"] == 0.0
    assert scores["shoulder_axis_drift"] == pytest.approx(0.0, abs=1e-9), "turn is not drift"


def test_a_mid_backswing_hitch_fires_the_backswing_reversal_alone() -> None:
    """The window that holds the fault reports it; the other three checks do not move."""
    swing = make_swing(30, 10)
    scores = _scores(_turned_shoulders(swing, _turn_profile(len(swing), hitch_at=22)))

    assert scores["shoulder_reversal_backswing"] is not None
    assert scores["shoulder_reversal_backswing"] > 10.0
    assert scores["shoulder_reversal_downswing"] == 0.0
    assert scores["hip_axis_drift"] == 0.0
    assert scores["hip_path_jitter"] == 0.0


def test_a_lateral_hip_slide_fires_the_drift_alone() -> None:
    """A hip sway of half a shoulder width reads as half a shoulder width.

    The arithmetic is pinned rather than the direction: the fixture's 0.08 slide against its 0.16
    shoulder span is 0.5, and a ruler swapped for the segment length or a mean would move it.
    """
    scores = _scores(make_swing(30, 10, hip_sway=0.08))

    assert scores["hip_axis_drift"] == pytest.approx(0.5, rel=1e-3)
    assert scores["shoulder_axis_drift"] == 0.0
    # Not zero, and the reason is the resampling rather than the fixture: this hip slides at one
    # constant speed in *frames*, and event time reads the backswing at ~1.45 frames per sample
    # against the downswing's ~0.5 — so the speed changes at the top and the path bends there. Two
    # orders of magnitude below a real shake, which is why it is documented as a floor and not
    # subtracted out (`pivot.hip_path_jitter`).
    assert scores["hip_path_jitter"] == pytest.approx(3.1e-4, rel=0.05)


def test_a_shake_in_place_and_a_slide_are_told_apart() -> None:
    """The two hip metrics are not one measurement twice, and neither is clean of the other.

    A shake has to register *some* excursion — it is one, briefly — and a slide has to register
    some bend, for the event-time reason above. So the claim that can honestly be made is the
    ratio, and it is an order of magnitude in each direction: the shake is ~85x rougher, the slide
    ~10x further travelled.

    The shake's own drift is exact arithmetic worth pinning: the path alternates about its address
    sample, which sits at one extreme, so the excursion is the full peak-to-peak `2 * 0.004 / 0.16`
    rather than the amplitude. The roughness is not pinned that way — an alternation at one sample
    per frame is aliased by a backswing read at 1.45 frames per sample and smoothed by a downswing
    read at 0.5, so its magnitude is a property of the tempo as much as of the shake.
    """
    slide = _measured(make_swing(30, 10, hip_sway=0.08))
    shake = _measured(_jittered_hips(make_swing(30, 10), 0.004))

    assert shake["hip_axis_drift"] == pytest.approx(0.05, rel=1e-3)
    assert shake["hip_path_jitter"] > 20 * slide["hip_path_jitter"]
    assert slide["hip_axis_drift"] > 8 * shake["hip_axis_drift"]


def test_mirroring_the_golfer_moves_no_number() -> None:
    """The unsigned property, on a swing where all five checks have something to say.

    This is the pin that makes `handedness` safely absent from every signature. A signed pivot
    metric added later breaks it here first, which is the point: the alternative is discovering it
    in `analysis/engine.py`, where the face-on call site is the one that gets forgotten.
    """
    upright = _scores(_wonky())
    mirrored = _scores(_mirrored(_wonky()))

    assert all(value is not None for value in upright.values())
    for name, value in upright.items():
        assert mirrored[name] == pytest.approx(value, rel=1e-9, abs=1e-12), name


def test_a_collapsed_shoulder_line_refuses_its_window_rather_than_reporting_an_angle() -> None:
    """One swing, two windows, two answers — which is what the coverage floor is for.

    The whole backswing collapses, so that window is refused; the downswing loses only the sample
    at the top and is measured. Pairing across the collapse instead would have charged one interval
    with the entire turn that happened during it.
    """
    swing = _flatten_shoulders(make_swing(30, 10), int(_ADDRESS), int(_TOP))
    observations = _observations(swing)
    assert observations is not None

    assert _flat(observations, BACKSWING_SPAN) > MAX_UNORIENTED
    assert _flat(observations, DOWNSWING_SPAN) < MAX_UNORIENTED

    backswing = PIVOT_CHECKS["shoulder_reversal_backswing"](observations)
    downswing = PIVOT_CHECKS["shoulder_reversal_downswing"](observations)

    assert backswing.value is None
    assert backswing.reason is UnscoredReason.LANDMARKS_UNCONFIDENT
    assert backswing.detail, "a reason with no window named is half an answer"
    assert downswing.value is not None


def test_too_few_samples_refuse_rather_than_reporting_zero() -> None:
    """An empty path is not a still one. The floors differ, so each check states its own."""
    one = [_still(0)]

    for name, check in PIVOT_CHECKS.items():
        outcome = check(one)
        assert outcome.value is None, name
        assert outcome.reason is UnscoredReason.TOO_FEW_FRAMES, name

    # Two samples are an excursion but not yet a bend: roughness needs a middle sample.
    two = [_still(0), _still(1)]
    assert PIVOT_CHECKS["hip_axis_drift"](two).value == pytest.approx(1.0)
    assert PIVOT_CHECKS["hip_path_jitter"](two).reason is UnscoredReason.TOO_FEW_FRAMES


def test_every_refusal_a_check_makes_is_one_a_measurement_may_report() -> None:
    """`analysis/pivot.py` measures and never judges, and this is what makes that checkable.

    Same guard `tests/analysis/test_measure.py` puts on the measure functions: a `NO_BAND` out of
    here would mean the split between measuring and judging had quietly closed again.
    """
    collapsed = _observations(_flatten_shoulders(make_swing(30, 10), int(_ADDRESS), int(_TOP)))
    assert collapsed is not None

    outcomes = [check([_still(0)]) for check in PIVOT_CHECKS.values()]
    outcomes += [check(collapsed) for check in PIVOT_CHECKS.values()]

    for outcome in outcomes:
        if outcome.value is None:
            assert outcome.reason in MEASUREMENT_REASONS


def test_two_frames_of_reference_in_one_path_are_a_wiring_bug() -> None:
    """Two instruments, never blended. Raised for `pivot_observations`' CALIBRATED_3D reason:
    nothing that comes out of a producer can look like this, so a caller assembled it."""
    mixed = [_still(0), _still(1, view=FrameOfReference.IMAGE_PLANE_DTL)]

    # Every check, not only the ones that compare orientations: the guard is about the space the
    # coordinates live in, and all five read coordinates.
    for check in PIVOT_CHECKS.values():
        with pytest.raises(ValueError, match="frames of reference"):
            check(mixed)
