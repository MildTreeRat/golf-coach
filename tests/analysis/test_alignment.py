"""Two-view alignment: one swing, two differently-configured phones, one shared tau axis.

Everything here is synthetic — the point of a normalized swing-time axis is that it can be
reasoned about exactly, so these are equalities and bounds rather than eyeballed video. The
fixtures reuse `make_swing` from conftest, which is the same builder `test_phases.py` runs on.
"""

from __future__ import annotations

import pytest
from conftest import _ADDRESS_FRAMES, make_swing

from golf_coach.analysis.alignment import (
    _BACKSWING_AGREEMENT_S,
    _MAX_WARP_SPEED_ERROR,
    _TEMPO_AGREEMENT,
    align_swings,
    anchors_from_keypoints,
    frame_of_tau,
    map_frame,
    pair_frames,
    tau_of_frame,
    warp_speeds,
    with_measured_impact,
)
from golf_coach.analysis.phases import _STRIKE_TOLERANCE_S, candidate_downswings
from golf_coach.analysis.smoothing import smooth_keypoints
from golf_coach.contracts.alignment import (
    TAU_IMPACT,
    TAU_TOP,
    AlignmentQuality,
    ClipAlignment,
    SwingAnchors,
)
from golf_coach.contracts.keypoints import ClipMetadata, FrameKeypoints


def _clip(keypoints: list[FrameKeypoints], fps: float) -> ClipMetadata:
    return ClipMetadata(fps=fps, width=1080, height=1920, frame_count=len(keypoints))


def _anchored(keypoints: list[FrameKeypoints], fps: float) -> SwingAnchors:
    anchors = anchors_from_keypoints(keypoints, clip=_clip(keypoints, fps))
    assert anchors is not None
    return anchors


def _concat(*clips: list[FrameKeypoints]) -> list[FrameKeypoints]:
    """Join clips end to end, renumbering frames and timestamps — a multi-swing recording."""
    joined: list[FrameKeypoints] = []
    for clip in clips:
        for frame in clip:
            index = len(joined)
            joined.append(
                frame.model_copy(update={"frame_index": index, "timestamp_ms": index * 10.0})
            )
    return joined


def _backswing_in_downswings(clip: ClipAlignment) -> float:
    """How long the warp thinks the backswing is, measured in that clip's own downswings."""
    return (clip.anchors.top - clip.warp_motion_start) / clip.anchors.downswing_frames


def _backswing_seconds(clip: ClipAlignment) -> float:
    """The same span in real time — which is what the two panels have to share, not the ratio."""
    assert clip.anchors.fps is not None
    return (clip.anchors.top - clip.warp_motion_start) / clip.anchors.fps


def _prepend_still(clip: list[FrameKeypoints], frames: int) -> list[FrameKeypoints]:
    """Hold the opening frame for longer — a phone that started rolling earlier."""
    return _concat([clip[0]] * frames, clip)


# A swing filmed at 60fps and the *same* swing at 240fps: every phase four times as many frames,
# **including the address dwell**. That last part is not cosmetic. `_motion_start` requires a quiet
# run of a quarter of the clip's own downswing (`_MOTION_STALL_FRACTION`), so at 4x the frame rate
# it needs 4x the still frames before the takeaway — which a real golfer standing over the ball for
# a second amply provides, and a fixture that scaled only the swing would not. Leaving it at 8 made
# the 240fps clip fall back to an estimated motion start and quietly demoted this whole comparison
# to TOP_IMPACT, testing the fallback instead of the thing it was written to test.
@pytest.fixture
def slow_clip() -> list[FrameKeypoints]:
    return make_swing(20, 8, followthrough_frames=8)


@pytest.fixture
def fast_clip() -> list[FrameKeypoints]:
    return _prepend_still(make_swing(80, 32, followthrough_frames=32), 24)


def test_the_same_swing_at_60_and_240_fps_aligns_on_its_instants(
    slow_clip: list[FrameKeypoints], fast_clip: list[FrameKeypoints]
) -> None:
    """The central claim: a 4x frame-rate difference is absorbed, not merely tolerated."""
    a = _anchored(slow_clip, 60.0)
    b = _anchored(fast_clip, 240.0)
    alignment = align_swings(a, b)

    assert alignment.quality is AlignmentQuality.FULL

    # The top of one clip must land on the top of the other, and impact on impact. This is the
    # whole feature; a frame of slack covers the detector's own rounding at each end.
    assert abs(map_frame(alignment, a.top, source="a") - b.top) <= 1
    assert abs(map_frame(alignment, a.impact, source="a") - b.impact) <= 1
    assert abs(map_frame(alignment, b.top, source="b") - a.top) <= 1
    assert abs(map_frame(alignment, b.impact, source="b") - a.impact) <= 1


def test_midway_through_the_downswing_maps_proportionally(
    slow_clip: list[FrameKeypoints], fast_clip: list[FrameKeypoints]
) -> None:
    """Between anchors the map is linear, so half a downswing in one clip is half in the other."""
    a = _anchored(slow_clip, 60.0)
    b = _anchored(fast_clip, 240.0)
    alignment = align_swings(a, b)

    halfway_a = a.top + a.downswing_frames // 2
    mapped = map_frame(alignment, halfway_a, source="a")
    expected = b.top + b.downswing_frames / 2
    # Two frames of the 240fps clip is half a frame of the 60fps one — the tolerance has to be
    # stated in the faster clip's units or it silently demands sub-frame accuracy from the slower.
    assert abs(mapped - expected) <= 2


def test_tau_round_trips_through_frames(slow_clip: list[FrameKeypoints]) -> None:
    a = _anchored(slow_clip, 60.0)
    for tau in (-0.5, 0.0, 0.25, 1.0, 1.5, 2.0, 3.0):
        assert tau_of_frame(a, frame_of_tau(a, tau)) == pytest.approx(tau)


def test_the_anchors_sit_at_their_defining_tau(slow_clip: list[FrameKeypoints]) -> None:
    a = _anchored(slow_clip, 60.0)
    assert tau_of_frame(a, a.motion_start) == pytest.approx(0.0)
    assert tau_of_frame(a, a.top) == pytest.approx(TAU_TOP)
    assert tau_of_frame(a, a.impact) == pytest.approx(TAU_IMPACT)


def test_past_impact_runs_at_the_downswing_rate(slow_clip: list[FrameKeypoints]) -> None:
    """The follow-through has no anchor, so it extrapolates at the last rate actually measured."""
    a = _anchored(slow_clip, 60.0)
    assert frame_of_tau(a, TAU_IMPACT + 1.0) == pytest.approx(a.impact + a.downswing_frames)


def test_the_mapping_never_goes_backwards(
    slow_clip: list[FrameKeypoints], fast_clip: list[FrameKeypoints]
) -> None:
    """Monotonicity is what lets the renderer stream both clips instead of buffering them."""
    alignment = align_swings(_anchored(slow_clip, 60.0), _anchored(fast_clip, 240.0))
    mapped = [map_frame(alignment, f, source="a") for f in range(len(slow_clip))]
    assert all(later >= earlier for earlier, later in zip(mapped, mapped[1:], strict=False))


def test_reversing_the_clips_inverts_the_alignment(
    slow_clip: list[FrameKeypoints], fast_clip: list[FrameKeypoints]
) -> None:
    a = _anchored(slow_clip, 60.0)
    b = _anchored(fast_clip, 240.0)
    forward = align_swings(a, b)
    backward = align_swings(b, a)

    assert forward.quality is backward.quality
    # Same correspondence, whichever way round the arguments went.
    assert map_frame(forward, a.top, source="a") == map_frame(backward, a.top, source="b")


def test_an_undetected_motion_start_drops_to_top_and_impact() -> None:
    """A clip that never settles must not be aligned as if its address were known.

    This is the `detected=False` path `test_phases.py` pins: the wrist moves from the first frame,
    so `segment_phases` returns a bounded *estimate*. It is fine for placing a window and wrong as
    a shared anchor, because two clips guessing separately do not guess the same.
    """
    moving = make_swing(20, 14, takeaway_frames=60)[_ADDRESS_FRAMES:]
    settled = make_swing(20, 14, takeaway_frames=60)

    a = _anchored(moving, 60.0)
    b = _anchored(settled, 60.0)
    assert not a.motion_start_detected

    alignment = align_swings(a, b)

    assert alignment.quality is AlignmentQuality.TOP_IMPACT
    assert any("motion start" in note for note in alignment.notes)
    # Top and impact still align exactly — dropping the soft anchor costs the backswing, not these.
    assert abs(map_frame(alignment, a.top, source="a") - b.top) <= 1
    assert abs(map_frame(alignment, a.impact, source="a") - b.impact) <= 1


def test_both_clips_take_the_same_fallback_duration_not_the_same_ratio() -> None:
    """Degrade symmetrically or not at all — and "symmetrically" is measured in seconds. [M10 P2]

    A shared *ratio* is not symmetry. Two views of one swing routinely disagree about the downswing
    by 10-40%, and the tour-median fallback multiplies that gap by 3.5 before it reaches the screen:
    on the 24-vs-19-frame pair below the old per-clip rule opened the render with the two panels
    0.28s apart at tau=0, which is the drift docs/M10_ALIGNMENT_ACCURACY.md §B1 measured on disk.
    """
    a = SwingAnchors(
        motion_start=100, top=200, impact=224, frame_count=400, fps=59.94, camera_id="face_on",
        motion_start_detected=False,
    )
    b = SwingAnchors(
        motion_start=300, top=400, impact=419, frame_count=700, fps=59.94,
        camera_id="down_the_line",
    )
    # The downswings disagree by 21%, under `_DOWNSWING_AGREEMENT`, so the tops stand and this is
    # the TOP_IMPACT branch under test rather than the IMPACT_ONLY one.
    alignment = align_swings(a, b)
    assert alignment.quality is AlignmentQuality.TOP_IMPACT

    assert alignment.a is not None and alignment.b is not None
    # Neither clip's warp uses its own detected motion start; both use the same substituted rule.
    assert alignment.b.warp_motion_start != alignment.b.anchors.motion_start
    # Within a frame of each other in real time. The ratios now differ, and that is the point.
    assert _backswing_seconds(alignment.a) == pytest.approx(
        _backswing_seconds(alignment.b), abs=1 / 59.94
    )
    assert _backswing_in_downswings(alignment.a) != pytest.approx(
        _backswing_in_downswings(alignment.b), abs=0.2
    )


def test_a_clip_without_fps_still_falls_back_on_its_own_downswing() -> None:
    """No fps means no duration to share, so the pre-top region degrades in ratio as it always did.

    Reported, not raised (ADR-013): a clip whose container never gave up a frame rate is a worse
    alignment, not an error. This is the path `_shared_motion_starts` deliberately leaves alone.
    """
    from golf_coach.analysis.phases import _FALLBACK_TEMPO_RATIO

    a = SwingAnchors(
        motion_start=100, top=200, impact=224, frame_count=400, camera_id="face_on",
        motion_start_detected=False,
    )
    b = SwingAnchors(
        motion_start=300, top=400, impact=419, frame_count=700, fps=59.94,
        camera_id="down_the_line",
    )
    alignment = align_swings(a, b)

    assert alignment.quality is AlignmentQuality.TOP_IMPACT
    assert alignment.a is not None and alignment.b is not None
    for clip in (alignment.a, alignment.b):
        expected = clip.anchors.top - round(
            _FALLBACK_TEMPO_RATIO * clip.anchors.downswing_frames
        )
        assert clip.warp_motion_start == expected


def test_agreeing_tempo_does_not_rescue_backswings_that_disagree_in_seconds() -> None:
    """The ratio check's blind spot, taken from the bundle it was found on. [M10 P3]

    Session 2026-08-23/9 shipped as `full` while the two panels were 0.233s apart at the takeaway —
    14 frames of daylight at 60fps, which is what the complaint was about. Its tempo ratios are
    4.06 and 4.25, closer together than any other pair on disk, because both views mismeasured the
    downswing in the same direction and the ratio divided the error back out.

    Session 8 is the control below: 14 times closer in real time, and its ratios are 1.89 against
    2.60. Ranking those two pairs by tempo agreement puts them in the wrong order, which is why
    seconds and not a ratio.
    """
    a = SwingAnchors(
        motion_start=469, top=534, impact=550, frame_count=726, fps=59.975, camera_id="face_on"
    )
    b = SwingAnchors(
        motion_start=246, top=297, impact=309, frame_count=450, fps=59.960,
        camera_id="down_the_line",
    )
    assert a.tempo_ratio is not None and b.tempo_ratio is not None
    # The ratios agree comfortably — this pair is refused on time alone, not on tempo.
    assert abs(a.tempo_ratio - b.tempo_ratio) / b.tempo_ratio < _TEMPO_AGREEMENT
    gap = (a.top - a.motion_start) / 59.975 - (b.top - b.motion_start) / 59.960
    assert gap > _BACKSWING_AGREEMENT_S

    alignment = align_swings(a, b)

    assert alignment.quality is AlignmentQuality.TOP_IMPACT
    note = next(n for n in alignment.notes if "backswings are" in n)
    assert "tempo ratios agree" in note
    # The downswings match (0.267s and 0.200s), so the takeaway is the boundary to doubt.
    assert "down_the_line is finding its motion start late" in note
    assert "DIFFERENT swings" not in note


def test_backswings_that_agree_in_seconds_keep_the_soft_anchor() -> None:
    """The honest pair, from session 2026-08-23/8 — and it is the one with the *worst* tempo gap.

    0.884s against 0.867s of backswing, one frame apart, on ratios of 1.89 and 2.60. A tighter
    `_TEMPO_AGREEMENT` would throw this pair away to catch session 9 above, and still miss it.
    """
    a = SwingAnchors(
        motion_start=452, top=505, impact=533, frame_count=700, fps=59.975, camera_id="face_on"
    )
    b = SwingAnchors(
        motion_start=300, top=352, impact=372, frame_count=600, fps=59.960,
        camera_id="down_the_line",
    )
    assert a.tempo_ratio is not None and b.tempo_ratio is not None
    assert abs(a.tempo_ratio - b.tempo_ratio) / b.tempo_ratio < _TEMPO_AGREEMENT

    alignment = align_swings(a, b)

    assert alignment.quality is AlignmentQuality.FULL
    assert alignment.a is not None and alignment.b is not None
    assert alignment.a.warp_motion_start == a.motion_start
    assert alignment.b.warp_motion_start == b.motion_start


def test_a_pair_without_fps_is_still_judged_on_the_ratio_alone() -> None:
    """No frame rate, no duration to compare — so the seconds check does not run. [M10 P3]

    Reported, not raised (ADR-013). A clip whose container never gave up a frame rate gets the
    weaker of the two cross-checks rather than an error, and keeps the soft anchor it would have
    kept before this check existed. Same anchors as the session-9 case above, fps dropped.
    """
    a = SwingAnchors(motion_start=469, top=534, impact=550, frame_count=726, camera_id="face_on")
    b = SwingAnchors(
        motion_start=246, top=297, impact=309, frame_count=450, camera_id="down_the_line"
    )

    alignment = align_swings(a, b)

    assert alignment.quality is AlignmentQuality.FULL
    assert not any("backswings are" in note for note in alignment.notes)


def test_disagreeing_tempo_refuses_the_soft_anchor() -> None:
    """Frame rate cancels out of a ratio, so two views of ONE swing cannot disagree on tempo.

    When they do, something is wrong that no amount of interpolation fixes — most likely the two
    clips locked onto *different* swings, which is exactly the practice-swing failure mode. Better
    a lower quality tier and a loud note than a confident, plausible, wrong video.
    """
    a = SwingAnchors(motion_start=0, top=60, impact=75, frame_count=200)   # 4.0 : 1
    b = SwingAnchors(motion_start=45, top=60, impact=75, frame_count=200)  # 1.0 : 1
    assert a.tempo_ratio is not None and b.tempo_ratio is not None
    assert abs(a.tempo_ratio - b.tempo_ratio) / a.tempo_ratio > _TEMPO_AGREEMENT

    alignment = align_swings(a, b)

    assert alignment.quality is AlignmentQuality.TOP_IMPACT
    assert any("tempo ratios disagree" in note for note in alignment.notes)


def test_agreeing_downswings_blame_the_takeaway_not_a_second_swing() -> None:
    """The denominator says which half of the ratio is wrong. [M7]

    "Check for a practice swing" is the right advice when the two views disagree about the
    downswing too — the tops are then on different events. It is the wrong advice when the
    downswings match to the millisecond and only the ratio differs, which is what the first fixed
    bay pair actually looks like: same motion, one late takeaway boundary. Sending the reader off
    to hunt a practice swing that isn't there is a worse failure than the degraded tier itself.
    """
    a = _DISAGREEING_A.model_copy(update={"motion_start": 636, "top": 694})  # 2.42 : 1
    alignment = align_swings(a, _DISAGREEING_B)  # b is 1.50 : 1, same 24-frame downswing

    assert alignment.quality is AlignmentQuality.TOP_IMPACT
    note = next(n for n in alignment.notes if "tempo ratios disagree" in n)
    assert "downswings agree" in note
    assert "down_the_line is finding its motion start late" in note
    assert "DIFFERENT swings" not in note


def test_disagreeing_downswings_still_warn_about_a_practice_swing() -> None:
    """The other branch: different downswings mean the tops are on different events.

    Both ratios have to stay above `MIN_PLAUSIBLE_TEMPO` or the collapse check refuses the soft
    anchor first and the tempo comparison never runs.
    """
    a = SwingAnchors(motion_start=0, top=60, impact=75, frame_count=200, fps=60.0)  # 4.0:1, 0.25s
    b = SwingAnchors(motion_start=0, top=90, impact=135, frame_count=200, fps=60.0)  # 2.0:1, 0.75s

    alignment = align_swings(a, b)

    note = next(n for n in alignment.notes if "tempo ratios disagree" in n)
    assert "DIFFERENT swings" in note


def test_a_backswing_shorter_than_its_downswing_is_refused() -> None:
    """The failure real phone footage actually produced. [M7 Phase 2]

    `_motion_start` walks back from the top for the last *quiet* stretch of wrist speed — and a
    golfer who pauses at the top hands it one immediately, so the boundary lands a frame or two
    below the top and the "backswing" measures near zero. `phases.py` reports `detected=True`,
    correctly: from inside one clip nothing looks wrong. Measured on the first real bay pair, the
    two views came out at 0.43 and 0.04 downswings of backswing.

    No golf swing has a backswing shorter than its downswing, so this needs no reference
    distribution to reject — and rejecting it is what stops a garbage anchor from being averaged
    into the warp as if it were evidence.
    """
    collapsed = SwingAnchors(motion_start=698, top=704, impact=718, frame_count=926)
    healthy = SwingAnchors(motion_start=1300, top=1551, impact=1574, frame_count=2472)
    assert collapsed.tempo_ratio is not None and collapsed.tempo_ratio < 1.0

    alignment = align_swings(collapsed, healthy)

    assert alignment.quality is AlignmentQuality.TOP_IMPACT
    assert any("collapsed onto the top" in note for note in alignment.notes)
    # Refused for one clip means substituted in BOTH, or the two panels drift apart.
    assert alignment.a is not None and alignment.b is not None
    assert alignment.a.warp_motion_start != collapsed.motion_start
    assert alignment.b.warp_motion_start != healthy.motion_start
    # The anchors that ARE trustworthy still line up exactly.
    assert abs(map_frame(alignment, collapsed.impact, source="a") - healthy.impact) <= 1
    assert abs(map_frame(alignment, collapsed.top, source="a") - healthy.top) <= 1


def _panel_speeds(alignment, count_a: int, count_b: int) -> tuple[float, float]:
    """How fast each panel runs against real time, given the render schedule.

    1.0 means the panel advances through its own footage at the rate it was filmed. Anything else
    is the warp resampling that view onto the other's timeline — the thing a viewer sees as one
    camera running fast.
    """
    schedule = pair_frames(alignment, count_a, count_b, reference="a")
    first, last = schedule[0], schedule[-1]
    fps_a = alignment.a.anchors.fps
    fps_b = alignment.b.anchors.fps
    output_seconds = len(schedule) / fps_a
    return (
        ((last.frame_a - first.frame_a) / fps_a) / output_seconds,
        ((last.frame_b - first.frame_b) / fps_b) / output_seconds,
    )


# The anchors the first real bay pair actually produced, before `_DRAWDOWN_FLOOR` landed: the two
# views measured the same physical downswing as 14 and 24 frames at the same ~60 fps.
_DISAGREEING_A = SwingAnchors(
    motion_start=698, top=704, impact=718, frame_count=926, fps=59.92, camera_id="face_on"
)
_DISAGREEING_B = SwingAnchors(
    motion_start=1514, top=1550, impact=1574, frame_count=2472, fps=59.96,
    camera_id="down_the_line",
)


def test_disagreeing_downswings_fall_back_to_real_time() -> None:
    """Two views that disagree about the downswing must not be forced to meet at the top. [M7]

    This is the failure the side-by-side made visible before the phase fix landed. The warp pins
    both clips to tau=1 and tau=2, so when one view measures the downswing at 0.234s and the other
    at 0.400s, the only way to satisfy both is to *resample* the longer one — the down-the-line
    panel played at 1.69x and started a full second out of step with face-on.

    Speed is the wrong thing to spend on a disagreement about instants. A viewer cannot see that
    the top anchor is two frames off, but they can absolutely see one camera running fast, and it
    misrepresents the two things the video exists to show. So both panels revert to their own
    native rate, pinned at impact, and the tier says the top is no longer an anchor.
    """
    alignment = align_swings(_DISAGREEING_A, _DISAGREEING_B)

    assert alignment.quality is AlignmentQuality.IMPACT_ONLY
    assert any("downswing durations disagree" in note for note in alignment.notes)

    speed_a, speed_b = _panel_speeds(alignment, 926, 2472)
    assert speed_a == pytest.approx(1.0, abs=0.05)
    assert speed_b == pytest.approx(1.0, abs=0.05), (
        f"the down-the-line panel is replayed at {speed_b:.2f}x — the warp is still resampling it"
    )


def test_the_rigid_fallback_pins_the_panels_at_impact() -> None:
    """Impact is the one instant both views locate to a median of a single frame."""
    alignment = align_swings(_DISAGREEING_A, _DISAGREEING_B)
    mapped = map_frame(alignment, _DISAGREEING_A.impact, source="a")
    assert abs(mapped - _DISAGREEING_B.impact) <= 1


def test_agreeing_downswings_keep_the_warp() -> None:
    """The guard must not fire on healthy input — degrading has a real cost. [M7]

    These are the same two clips after `phases._DRAWDOWN_FLOOR` fixed the face-on top: the views
    now measure the downswing at 24 frames apiece, so the detected tops stand as anchors.
    """
    a = _DISAGREEING_A.model_copy(update={"motion_start": 636, "top": 694})
    alignment = align_swings(a, _DISAGREEING_B)

    assert alignment.quality is not AlignmentQuality.IMPACT_ONLY
    assert alignment.a is not None and alignment.b is not None
    assert alignment.a.warp_top is None and alignment.b.warp_top is None
    assert alignment.a.top == a.top


def test_the_rigid_fallback_tolerates_two_honestly_different_frame_rates() -> None:
    """A 30fps phone beside a 60fps one is an ordinary pairing, not a broken clock.

    Worth pinning because the obvious guard here — refuse when the two reported rates differ — is
    wrong, and was written that way first. Each clip converts the shared duration through its own
    fps, so different rates are exactly what the arithmetic is for.
    """
    b = _DISAGREEING_B.model_copy(update={"fps": 29.97})
    alignment = align_swings(_DISAGREEING_A, b)

    assert alignment.quality is AlignmentQuality.IMPACT_ONLY
    speed_a, speed_b = _panel_speeds(alignment, 926, 2472)
    assert speed_a == pytest.approx(1.0, abs=0.05)
    assert speed_b == pytest.approx(1.0, abs=0.05)


def test_an_impossible_reference_is_imposed_rather_than_replayed_fast() -> None:
    """The inversion of the rule this used to pin, and the whole of the 2026-08-30 fix.

    An unbelievable reference duration used to veto the correction, leaving both detected tops in
    place. That reads as caution and is not: `pair_frames` has no way to express two disagreeing
    tops except as playback speed, so declining shipped four bundles replaying a panel at 1.48x to
    3.11x. Holding both panels to a duration nobody can swing costs the top banner a few frames;
    declining costs the viewer the tempo the side-by-side exists to show.

    What the veto was protecting is not carried by the warp at all - `top_late_by` carries it, on
    its own evidence - so nothing about the diagnosis changes here.
    """
    a = _DISAGREEING_A.model_copy(update={"top": 712})  # 6 frames ~ 0.10s, below the plausible band
    alignment = align_swings(a, _DISAGREEING_B)

    assert alignment.quality is AlignmentQuality.IMPACT_ONLY
    assert alignment.a is not None and alignment.b is not None
    assert alignment.a.warp_top is not None and alignment.b.warp_top is not None
    # The band is still read, and still reported - it just no longer decides anything.
    assert any("not a downswing any golfer makes" in note for note in alignment.notes)
    # The point of imposing it: neither panel is replayed at a speed its camera never shot.
    assert warp_speeds(alignment)["downswing"] == pytest.approx(1.0, abs=0.01)


def test_manual_anchors_are_not_a_second_code_path(slow_clip: list[FrameKeypoints]) -> None:
    """An overridden anchor must produce exactly what the detector's would, given the same numbers.

    Down-the-line detection is M7's biggest open risk, so the override has to be trustworthy
    rather than a bolted-on special case.
    """
    detected = _anchored(slow_clip, 60.0)
    by_hand = SwingAnchors(
        motion_start=detected.motion_start,
        top=detected.top,
        impact=detected.impact,
        frame_count=detected.frame_count,
        fps=detected.fps,
    )
    other = _anchored(make_swing(40, 16, followthrough_frames=16), 120.0)

    assert align_swings(detected, other).model_dump() == align_swings(by_hand, other).model_dump()


def test_a_clip_too_short_to_segment_yields_no_anchors() -> None:
    assert anchors_from_keypoints([]) is None
    assert anchors_from_keypoints(make_swing(1, 1)[:4]) is None


def test_an_unalignable_swing_is_reported_not_raised() -> None:
    """`align_swings` never sees a degenerate anchor set — the model refuses to build one."""
    with pytest.raises(ValueError, match="impact"):
        SwingAnchors(motion_start=0, top=50, impact=50)
    with pytest.raises(ValueError, match="motion_start"):
        SwingAnchors(motion_start=60, top=50, impact=70)


# --- multi-swing clips: the practice-swing problem -----------------------------------------


def test_candidate_downswings_finds_every_swing_in_the_clip() -> None:
    """A practice swing before the real one is two descents, and both must be visible.

    `segment_phases` takes the *earliest* major descent — right for a trimmed single-swing clip,
    and wrong for a phone recording where the golfer rehearses first. It cannot tell them apart,
    so the remedy is to show the caller what is there rather than to guess better.

    Note the **third** descent this finds, between the two swings: after the finish the hands come
    back *down* to address for the next swing, and in a wrist-y trace that is a descent like any
    other. It is smaller than either real downswing, which is what lets a reader rank it — but it
    is real, and any rule that assumed "N descents means N swings" would be wrong by one per swing.
    """
    two_swings = _concat(make_swing(20, 8), make_swing(20, 8))
    swings = candidate_downswings(smooth_keypoints(two_swings), min_fraction=0.45)

    assert len(swings) >= 2
    assert swings == sorted(swings, key=lambda s: s.top), "earliest first"

    # The two real downswings are the two deepest descents; the return-to-address between them is
    # shallower than both.
    deepest = sorted(swings, key=lambda s: s.rise, reverse=True)[:2]
    assert min(s.rise for s in deepest) > max(
        (s.rise for s in swings if s not in deepest), default=0.0
    )

    # Earliest first, and the first is the one segment_phases would have chosen on its own.
    first = anchors_from_keypoints(two_swings)
    assert first is not None
    assert abs(first.top - swings[0].top) <= 3


def test_a_window_selects_the_real_swing_in_absolute_frame_numbers() -> None:
    """Windowing must return indices in the ORIGINAL clip's coordinates, not the slice's.

    Every frame number leaving this module is used to index a video, so an off-by-a-window error
    would render the wrong part of the clip while looking entirely plausible.
    """
    practice = make_swing(20, 8)
    real = make_swing(20, 8)
    two_swings = _concat(practice, real)
    boundary = len(practice)

    windowed = anchors_from_keypoints(two_swings, window=(boundary, len(two_swings)))
    alone = anchors_from_keypoints(real)
    assert windowed is not None and alone is not None

    # The second swing's instants, expressed in the joined clip's frame numbering.
    assert windowed.top >= boundary
    assert abs(windowed.top - (alone.top + boundary)) <= 2
    assert abs(windowed.impact - (alone.impact + boundary)) <= 2


def test_windowed_anchors_align_against_an_untrimmed_clip() -> None:
    """The end-to-end shape of the fix: one phone caught the practice swing, the other didn't."""
    real = make_swing(20, 8)
    with_practice = _concat(make_swing(20, 8), real)

    a = anchors_from_keypoints(real, clip=_clip(real, 60.0))
    b = anchors_from_keypoints(
        with_practice, clip=_clip(with_practice, 60.0), window=(len(real), len(with_practice))
    )
    assert a is not None and b is not None

    alignment = align_swings(a, b)
    assert alignment.quality is AlignmentQuality.FULL
    assert abs(map_frame(alignment, a.impact, source="a") - b.impact) <= 1


def test_pair_frames_gives_one_tau_per_output_frame() -> None:
    """The render schedule is the seam between working out the correspondence and drawing it.

    Every entry carries a single tau used for BOTH panels, which is what makes the banners land
    simultaneously by construction rather than by coincidence — a renderer that did its own warp
    arithmetic per panel could drift.
    """
    real = make_swing(20, 8)
    padded = _prepend_still(real, 30)

    a = _anchored(real, 60.0)
    b = _anchored(padded, 60.0)
    alignment = align_swings(a, b)

    schedule = pair_frames(alignment, len(real), len(padded))
    assert schedule

    # The reference clip advances one frame at a time; the follower never goes backwards.
    assert [entry.frame_a for entry in schedule] == sorted({e.frame_a for e in schedule})
    assert all(
        later.frame_b >= earlier.frame_b
        for earlier, later in zip(schedule, schedule[1:], strict=False)
    ), "the warp is monotone, which is what lets both clips stream"

    # Both indices stay inside their own clip, so a renderer can index without checking.
    assert all(0 <= entry.frame_a < len(real) for entry in schedule)
    assert all(0 <= entry.frame_b < len(padded) for entry in schedule)

    # tau increases with the reference frame, and impact lands where the anchors say it does.
    taus = [entry.tau for entry in schedule]
    assert taus == sorted(taus)
    at_impact = min(schedule, key=lambda entry: abs(entry.tau - TAU_IMPACT))
    assert abs(at_impact.frame_a - a.impact) <= 1
    assert abs(at_impact.frame_b - b.impact) <= 1


def test_pair_frames_is_empty_without_an_alignment() -> None:
    """Reported, not raised: nothing to map through means no schedule (ADR-013)."""
    from golf_coach.contracts.alignment import SwingAlignment

    assert pair_frames(SwingAlignment(), 10, 10) == []


def test_pair_frames_rejects_an_unknown_reference() -> None:
    real = make_swing(20, 8)
    alignment = align_swings(_anchored(real, 60.0), _anchored(real, 60.0))
    with pytest.raises(ValueError, match="reference must be"):
        pair_frames(alignment, len(real), len(real), reference="c")


# --- the ball strike as a measured anchor ---------------------------------------------- [M11 P6]
#
# Every number below is taken from docs/M11_ACOUSTIC_SYNC.md rather than invented: §E4's four
# offending bundles, where the down-the-line pose impact runs 5.7-7.5 frames early while face-on is
# right to within a frame, and §E5's bay, where the ball hitting the impact screen 85-145 ms later
# is the *louder* transient on every clip measured. The second fact is why these tests care so much
# about which of two candidates wins.


def _pair(*, impact_a: int = 224, impact_b: int = 419) -> tuple[SwingAnchors, SwingAnchors]:
    """Two views of one swing whose anchors agree, so any tier change is the strike's doing."""
    a = SwingAnchors(
        motion_start=100, top=200, impact=impact_a, frame_count=400, fps=59.94,
        camera_id="face_on",
    )
    b = SwingAnchors(
        motion_start=295, top=395, impact=impact_b, frame_count=700, fps=59.94,
        camera_id="down_the_line",
    )
    return a, b


def test_an_anchor_with_no_strike_is_left_inferred() -> None:
    """Nobody listened, and the detector heard nothing, are both "no measurement".

    Neither may come back claiming `impact_measured` at the pose estimate: not measured is not the
    same as measured and agreeing (ADR-010 §2). The two differ one level up, in `api/pipeline.py`,
    which notes the silence — down here they are the same absence.
    """
    anchors, _ = _pair()
    for strikes in (None, []):
        measured = with_measured_impact(anchors, strikes)
        assert measured == anchors
        assert not measured.impact_measured


def test_the_early_down_the_line_impact_is_pulled_onto_the_strike() -> None:
    """§E4's finding, in the shape it was measured: pose 7 frames early, the crack where it is."""
    _, dtl = _pair()
    measured = with_measured_impact(dtl, [dtl.impact + 7])

    assert measured.impact == dtl.impact + 7
    assert measured.impact_measured
    # The correction lands entirely in the downswing — nothing else about the clip moved.
    assert measured.top == dtl.top and measured.motion_start == dtl.motion_start
    assert measured.downswing_frames == dtl.downswing_frames + 7


def test_the_screen_strike_never_wins_over_the_ball() -> None:
    """§E5's trap, both ways round: the loud one is second, and second must never win.

    The bay makes the ball, the mat, then the impact screen 85-145 ms later — 5 to 9 frames at 60
    fps, and 3-4x louder in onset flux. No window can separate those, because the correction this
    exists to make (§E4, up to 7.5 frames) is *longer* than the gap it would have to exclude. Only
    the ordering can, so it is the ordering that is pinned here rather than the window width.
    """
    _, dtl = _pair()
    screen = round(0.145 * 59.94)  # the widest ball-to-screen gap §E5 measured

    # Pose impact already right — the case where "take the loudest peak" takes the screen strike.
    on_time = with_measured_impact(dtl, [dtl.impact, dtl.impact + screen])
    assert on_time.impact == dtl.impact

    # And pose impact 7 frames early, where both candidates sit after the estimate.
    early = with_measured_impact(dtl, [dtl.impact + 7, dtl.impact + 7 + screen])
    assert early.impact == dtl.impact + 7
    assert early.impact_measured


def test_a_transient_outside_the_window_is_not_this_swing() -> None:
    """A crack from the next bay, or the golfer's next shot, is not evidence about this impact."""
    _, dtl = _pair()
    outside = round(_STRIKE_TOLERANCE_S * 59.94) + 1

    measured = with_measured_impact(dtl, [dtl.impact + outside])

    assert measured == dtl
    assert not measured.impact_measured


def test_a_transient_before_the_top_is_refused_rather_than_clamped() -> None:
    """A live branch, not a defensive one — the window outruns a fast downswing all by itself.

    M10's four offenders measure 0.183-0.267 s of face-on downswing, and the strike window is
    0.20 s wide, so on the fastest of them the window opens *before* the top. `SwingAnchors`
    requires impact strictly after it, and `with_measured_impact` skips such a candidate rather
    than clamping it onto the top: a transient that early is a different event, not a mismeasured
    impact.
    """
    face_on, _ = _pair(impact_a=211)  # 11 frames of downswing, 0.18 s at 59.94
    assert face_on.impact - face_on.top < round(_STRIKE_TOLERANCE_S * 59.94)

    measured = with_measured_impact(face_on, [face_on.top - 1])

    assert measured == face_on
    assert not measured.impact_measured


def test_a_strike_past_the_end_of_the_clip_is_refused() -> None:
    """The audio outlives the video on some containers; an impact off the end of it is not one."""
    face_on, _ = _pair()
    beyond = face_on.frame_count
    assert beyond is not None

    measured = with_measured_impact(
        face_on.model_copy(update={"impact": beyond - 2}), [beyond + 1]
    )

    assert not measured.impact_measured


def test_without_fps_there_is_no_window_to_size() -> None:
    """The tolerance is in seconds and the strikes are in frames; no rate, no comparison.

    Reported, not raised (ADR-013) — the same shape as `_shared_motion_starts`' fps guard, which is
    the other place a clip with no frame rate simply gets less.
    """
    anchors = SwingAnchors(motion_start=100, top=200, impact=224, frame_count=400)

    measured = with_measured_impact(anchors, [224])

    assert measured == anchors
    assert not measured.impact_measured


def test_both_views_on_a_heard_strike_are_synchronized() -> None:
    """The tier this milestone exists to produce: a real shared clock, not a shared inference."""
    a, b = _pair()
    alignment = align_swings(
        with_measured_impact(a, [a.impact]), with_measured_impact(b, [b.impact])
    )

    assert alignment.quality is AlignmentQuality.SYNCHRONIZED
    assert alignment.quality.summary == "synchronized on the ball strike"
    assert not alignment.quality.is_degraded


def test_one_view_hearing_the_strike_is_not_a_shared_clock() -> None:
    """Half a pair earns a note and no tier — a phone across the bay hears less, ordinarily."""
    a, b = _pair()
    alignment = align_swings(with_measured_impact(a, [a.impact]), b)

    assert alignment.quality is AlignmentQuality.FULL
    assert any("face_on" in note and "down_the_line" in note for note in alignment.notes)


def test_a_measured_impact_outranks_a_refused_top() -> None:
    """The inversion §Design names: `impact_only` must not be the label on the best anchor here.

    The downswings disagree far past `_DOWNSWING_AGREEMENT`, so `_shared_tops` refuses the detected
    tops and the pose ladder bottoms out at `IMPACT_ONLY` — the tier whose *whole reason* for being
    the worst one is that its single anchor was a guess. Here it is not a guess, so the label goes
    up. The warp underneath still holds both panels to one duration back from impact rather than
    replaying either fast; which duration is P7's question, pinned below.
    """
    a = SwingAnchors(
        motion_start=100, top=200, impact=213, frame_count=400, fps=59.94, camera_id="face_on",
    )
    b = SwingAnchors(
        motion_start=295, top=395, impact=419, frame_count=700, fps=59.94,
        camera_id="down_the_line",
    )
    assert align_swings(a, b).quality is AlignmentQuality.IMPACT_ONLY

    alignment = align_swings(
        with_measured_impact(a, [a.impact]), with_measured_impact(b, [b.impact])
    )

    assert alignment.quality is AlignmentQuality.SYNCHRONIZED
    assert alignment.a is not None and alignment.b is not None
    assert alignment.a.warp_top is not None and alignment.b.warp_top is not None
    assert any("downswing durations disagree" in note for note in alignment.notes)


def test_every_tier_has_a_summary_and_only_the_top_two_are_undegraded() -> None:
    """`is_degraded` and `summary` are total over the enum — a new member cannot be half-added."""
    for quality in AlignmentQuality:
        assert quality.summary
        assert quality.is_degraded is (
            quality not in {AlignmentQuality.SYNCHRONIZED, AlignmentQuality.FULL}
        )


def test_anchors_written_before_m11_read_back_as_inferred() -> None:
    """The default is False so an artifact on disk cannot claim a measurement nobody took."""
    stored = {"motion_start": 100, "top": 200, "impact": 224}

    assert not SwingAnchors.model_validate(stored).impact_measured


# --- arbitrating the late top ---------------------------------------------------------- [M11 P7]
#
# The anchors below are the ones on disk, read out of `analysis.json` for 2026-08-23 bundles 2 and
# 4 — two of the five that carry M10's handoff defect (docs/M11_ACOUSTIC_SYNC.md §E3). §E4 measured
# both bundles' pose impacts as right in *both* views to within a frame, which is why the strikes
# here are `[anchors.impact]`: that is exactly what P9's re-run will produce for them.


def _bundle_2() -> tuple[SwingAnchors, SwingAnchors]:
    """Face-on reads 0.217s of downswing where down-the-line reads 0.384s of the same swing."""
    a = SwingAnchors(
        motion_start=910, top=974, impact=987, frame_count=1100, fps=59.9747,
        camera_id="face_on",
    )
    b = SwingAnchors(
        motion_start=1273, top=1274, impact=1297, frame_count=1400, fps=59.9602,
        camera_id="down_the_line",
    )
    return a, b


def _bundle_4() -> tuple[SwingAnchors, SwingAnchors]:
    """0.200s against 0.484s — the same defect, with a reference too long to be a downswing."""
    a = SwingAnchors(
        motion_start=672, top=745, impact=757, frame_count=900, fps=59.9739,
        camera_id="face_on",
    )
    b = SwingAnchors(
        motion_start=1225, top=1275, impact=1304, frame_count=1400, fps=59.9588,
        camera_id="down_the_line",
    )
    return a, b


def _heard(pair: tuple[SwingAnchors, SwingAnchors]) -> tuple[SwingAnchors, SwingAnchors]:
    """Both views anchored on the strike each one heard — the shared clock P6 built."""
    a, b = pair
    return with_measured_impact(a, [a.impact]), with_measured_impact(b, [b.impact])


def test_the_late_top_is_the_shorter_downswing_once_both_views_heard_the_strike() -> None:
    """M10's handoff defect, decided rather than described.

    Both tops used to be dragged onto the face-on duration, which on these bundles is the *wrong*
    one — face-on is the view whose descent fragmented. With tau=2 pinned to one sound in both
    clips the two downswings measure one interval in real time, so the shorter is the late top and
    the reference flips to the other view.
    """
    a, b = _heard(_bundle_2())
    alignment = align_swings(a, b)

    assert alignment.a is not None and alignment.b is not None
    # Face-on's top moves 10 frames earlier, onto down-the-line's 0.384s; down-the-line keeps the
    # top it detected, because the reference is its own duration.
    assert alignment.a.warp_top == 964
    assert alignment.b.warp_top == b.top


def test_the_arbitrated_note_names_the_late_view_and_what_it_does_to_tempo() -> None:
    """Reported, never substituted — and the number the reader is looking at is the tempo.

    `SwingResult` keeps the 4.92:1 the face-on phases produced; this note is what says the
    denominator behind it is 10 frames short and what the ratio reads without that error.
    """
    alignment = align_swings(*_heard(_bundle_2()))

    note = next(n for n in alignment.notes if "downswing durations disagree" in n)
    assert "face_on's is 10 frames late" in note
    assert "moves face_on's top to frame 964" in note
    assert "reads 2.35:1 rather than 4.92:1" in note


def test_without_a_shared_clock_the_reference_is_still_the_face_on_view() -> None:
    """The pre-M11 behaviour, unchanged where the evidence to change it is absent.

    Same two clips, neither anchored on sound. Two inferred impacts cannot settle which top is
    wrong, so `_shared_tops` falls back to the view the detector was tuned on — and on this bundle
    that is demonstrably the late one. Holding the pair to a duration that is probably wrong still
    beats replaying one panel fast; it is the arbitration that is new, not the fallback.
    """
    a, b = _bundle_2()
    alignment = align_swings(a, b)

    assert alignment.a is not None and alignment.b is not None
    assert alignment.a.warp_top == a.top  # face-on's own 0.217s, imposed on both
    note = next(n for n in alignment.notes if "downswing durations disagree" in n)
    assert "one view's top is wrong" in note
    assert "frames late" not in note


def test_an_arbitrated_reference_that_is_no_downswing_is_imposed_with_a_caution() -> None:
    """Knowing which top is wrong is not the same as knowing where the right one is - so say so.

    Bundle 4's shape with its down-the-line downswing stretched to 0.917s — past
    `phases.POSSIBLE_DOWNSWING_S`, which no golfer's downswing reaches. §E4 says this family's
    down-the-line impact anchor is the *early* kind, so P6 pushing tau=2 later only lengthens it
    further. That reference really is suspect. It is imposed regardless, because the alternative on
    this bundle was a down-the-line panel replayed at 3.11x, and the note carries the doubt instead
    of the warp carrying it as speed.

    Stretched rather than taken as stored: bundle 4's real 0.484s sat outside the 0.45 ceiling of
    the day and sits comfortably inside the widened band, which is the 2026-08-30 change working —
    0.484s *is* a downswing. The branch still exists for a reference that is not one.
    """
    a, b = _bundle_4()
    b = b.model_copy(update={"impact": b.top + 55})  # 0.917s of "downswing"
    alignment = align_swings(*_heard((a, b)))

    assert alignment.a is not None and alignment.b is not None
    assert alignment.a.warp_top is not None and alignment.b.warp_top is not None
    note = next(n for n in alignment.notes if "downswing durations disagree" in n)
    assert "face_on's is 43 frames late" in note
    assert "not a downswing any golfer makes" in note
    assert "top banner is likely wrong in both panels" in note


def test_one_strike_in_both_clips_rules_out_two_different_swings() -> None:
    """The note P0 found to be false on every bundle that carries it.

    Bundles 4, 7 and 9 all report *"Most likely the two clips are showing DIFFERENT swings"*, and
    on all three §E4 cross-correlates the two views' audio to a single strike at r = 0.76-0.83.
    They are one swing filmed twice with a bad boundary in one view — which is now sayable, because
    a strike heard in both clips is the evidence that rules the alternative out.
    """
    a, b = _bundle_4()
    assert "DIFFERENT swings" in next(
        n for n in align_swings(a, b).notes if "tempo ratios disagree" in n
    )

    alignment = align_swings(*_heard((a, b)))

    note = next(n for n in alignment.notes if "tempo ratios disagree" in n)
    assert "DIFFERENT swings" not in note
    assert "one swing filmed twice" in note
    assert "face_on's top is the late one, by 17 frames" in note


def test_half_a_pair_is_not_enough_to_arbitrate() -> None:
    """One microphone is not a shared clock — the same rule the tier already applies.

    Worth its own pin because the arbitration is the more tempting place to relax it: the late view
    here *is* the one without a measurement, so "trust the view that heard something" would give
    the right answer on this bundle and the wrong one the moment the deaf phone is the sound view.
    """
    a, b = _bundle_2()
    alignment = align_swings(a, with_measured_impact(b, [b.impact]))

    assert alignment.a is not None
    assert alignment.a.warp_top == a.top
    assert not any("frames late" in note for note in alignment.notes)



# --- the late top as a finding a consumer can read ------------------------------------- [M11 P8]
#
# P7 put the arbitration in the notes, where only a human can read it. These pin the machine-
# readable half: `ClipAlignment.top_late_by`, which is what `analysis.engine` withdraws a
# checkpoint on. The distinction that matters throughout is diagnosis against correction —
# `warp_top` is what the warp *did*, `top_late_by` is what the shared clock *knows*.


def test_the_late_top_is_recorded_on_the_clip_that_carries_it() -> None:
    """The finding lands on one clip, not on the pair — a pair has two tops and one is fine."""
    alignment = align_swings(*_heard(_bundle_2()))

    assert alignment.a is not None and alignment.b is not None
    assert alignment.a.top_late_by == 10
    assert alignment.a.top_is_late
    # The sound view keeps the top it detected, so nothing about it was contradicted. Pinned
    # because "the tops disagree" is a symmetric fact and this one deliberately is not.
    assert alignment.b.top_late_by is None
    assert not alignment.b.top_is_late


def test_a_correction_that_does_not_fit_the_clip_is_still_a_contradicted_top() -> None:
    """The case the whole field exists for, and the one `warp_top` cannot express.

    The warp can still decline - not on plausibility any more, but on *fit*: a reference measuring
    back past frame 0 has no top to pin in that clip. Reading the correction as the finding would
    make such a bundle indistinguishable from one where the two views agreed, and this family is
    precisely the one shipping a `tempo` of 6.08:1 that M10 P10 called not coaching truth.

    Declining is safe here only because `pair_frames` guards the render independently. Before that
    guard existed every route out of `_shared_tops` returning None shipped a speed-up, which is
    what made the plausibility veto harmful rather than merely cautious.
    """
    # Face-on's whole clip is shorter than down-the-line's downswing, so 0.484s back from its
    # impact lands before its first frame and there is no shared top to impose.
    a, b = _bundle_4()
    a = a.model_copy(update={"motion_start": 3, "top": 12, "impact": 24})
    alignment = align_swings(*_heard((a, b)))

    assert alignment.a is not None and alignment.b is not None
    assert alignment.a.warp_top is None and alignment.b.warp_top is None
    assert any("falls outside one of the clips" in note for note in alignment.notes)
    assert alignment.a.top_is_late
    assert alignment.b.top_late_by is None


def test_a_pair_with_no_shared_clock_claims_no_late_top() -> None:
    """Same two clips, neither anchored on sound: the disagreement is real and undecidable.

    The pre-M11 fallback still holds both panels to the face-on duration — but it does so on a tie
    -break, not on evidence, so nothing may be recorded as *contradicted*. A `top_late_by` set here
    would have `analysis.engine` withdraw a score on the strength of a guess (ADR-010 §2).
    """
    alignment = align_swings(*_bundle_2())

    assert alignment.a is not None and alignment.b is not None
    assert alignment.a.top_late_by is None and alignment.b.top_late_by is None


def test_two_views_that_agree_record_no_late_top() -> None:
    """The ordinary case: both heard the strike, both read one downswing, nothing to settle."""
    a, b = _heard(_pair())
    alignment = align_swings(a, b)

    assert alignment.quality is AlignmentQuality.SYNCHRONIZED
    assert alignment.a is not None and alignment.b is not None
    assert alignment.a.top_late_by is None and alignment.b.top_late_by is None


def test_a_clip_alignment_written_before_m11_reads_back_as_uncontradicted() -> None:
    """Every stored bundle predates the field, and none of them was arbitrated.

    The same back-compatibility shape as `SwingAnchors.impact_measured` and `warp_top`: the default
    has to mean "nobody looked", and here that is indistinguishable from "nothing was wrong" only
    because a top nobody checked is a top nothing contradicts.
    """
    stored = ClipAlignment.model_validate(
        {
            "anchors": {"motion_start": 10, "top": 40, "impact": 60},
            "warp_motion_start": 10,
            "tau_start": -0.33,
            "tau_end": 2.5,
        }
    )

    assert stored.top_late_by is None
    assert not stored.top_is_late


# --- never at a speed the camera did not shoot ------------------------------------- [2026-08-30]
#
# The complaint that started this: on `2026-08-23/9` the down-the-line panel plays at 1.00x to the
# top and 2.08x from the top onward. Nothing was wrong with the clip - the warp was resampling it
# to make two disagreeing tops meet at one impact. These pin the two layers that stop it:
# `_shared_tops` repairs the anchor where it can, and `pair_frames` refuses the speed regardless.


def test_warp_speeds_reads_one_for_a_pair_that_agrees() -> None:
    """The measurement the guard stands on, on a pair with nothing wrong with it."""
    speeds = warp_speeds(align_swings(*_bundle_8()))

    assert speeds["downswing"] == pytest.approx(1.0, abs=_MAX_WARP_SPEED_ERROR)
    assert speeds["backswing"] == pytest.approx(1.0, abs=_MAX_WARP_SPEED_ERROR)


def test_warp_speeds_declines_without_a_frame_rate() -> None:
    """A speed is a rate in real time and there is none without fps - empty, never 1.0.

    ADR-010 §2 in miniature: "not measured" and "measured at native rate" are different claims, and
    a guard that read an empty dict as agreement would wave through exactly the clips it cannot
    see. `pair_frames` leans on this - it warps unguarded rather than rigidly when fps is missing.
    """
    a, b = _bundle_9()
    assert warp_speeds(align_swings(a, b.model_copy(update={"fps": None}))) == {}


def test_session_9_no_longer_replays_the_follower_at_double_speed() -> None:
    """The bundle the complaint came from, at its real stored anchors.

    Face-on reads 13 frames of downswing where down-the-line reads 27 of the same swing, and both
    impacts are pinned to the strike each phone heard. The old veto refused the correction because
    0.4503s missed the duration bound of the day by 0.3 ms, and the render then covered
    down-the-line's 27 frames in face-on's 13.
    """
    alignment = align_swings(*_heard(_bundle_9()))

    speeds = warp_speeds(alignment)
    assert speeds["downswing"] == pytest.approx(1.0, abs=0.01)
    assert speeds["backswing"] == pytest.approx(1.0, abs=0.01)


def test_the_schedule_advances_the_follower_at_its_native_rate() -> None:
    """The claim as a viewer meets it: frames on a schedule, not durations in a model.

    Sampled across the top, because that is where the old warp changed gear - one rate below tau=1
    and another above it. A single rate over the whole schedule is what "nothing was sped up" means.
    """
    alignment = align_swings(*_heard(_bundle_9()))
    schedule = pair_frames(alignment, 726, 450, reference="a")

    assert schedule
    below = [step for step in schedule if step.tau < TAU_TOP]
    above = [step for step in schedule if TAU_TOP <= step.tau <= TAU_IMPACT]
    assert len(below) > 2 and len(above) > 2
    for segment in (below, above):
        advanced = segment[-1].frame_b - segment[0].frame_b
        drove = segment[-1].frame_a - segment[0].frame_a
        assert advanced / drove == pytest.approx(1.0, abs=0.1)


def test_a_pre_top_disagreement_the_repair_never_sees_is_still_refused_as_speed() -> None:
    """The half `_shared_tops` structurally cannot reach, and the reason the guard exists at all.

    That rule only ever moves a *top*, so it is blind to a pair whose downswings agree and whose
    backswings do not - which an accepted soft anchor allows up to `_BACKSWING_AGREEMENT_S` of.
    `2026-08-23/10` is this shape on disk: 0.95x through the downswing, which nobody would notice,
    and 0.83x before the top, which is six frames of daylight by the takeaway.
    """
    a, b = _bundle_10()
    alignment = align_swings(a, b)

    # The repair declines - the downswings are within `_DOWNSWING_AGREEMENT` of each other.
    assert alignment.a is not None and alignment.b is not None
    assert alignment.a.warp_top is None and alignment.b.warp_top is None
    assert warp_speeds(alignment)["backswing"] < 1.0 - _MAX_WARP_SPEED_ERROR

    # The guard does not, and the rendered schedule comes out at native rate anyway.
    schedule = pair_frames(alignment, a.frame_count or 900, b.frame_count or 900, reference="a")
    assert schedule
    advanced = schedule[-1].frame_b - schedule[0].frame_b
    drove = schedule[-1].frame_a - schedule[0].frame_a
    assert advanced / drove == pytest.approx(1.0, abs=_MAX_WARP_SPEED_ERROR)


def _bundle_8() -> tuple[SwingAnchors, SwingAnchors]:
    """Two views of one swing that actually agree - 28 frames of downswing in each."""
    a = SwingAnchors(
        motion_start=452, top=505, impact=533, frame_count=700, fps=59.975, camera_id="face_on"
    )
    b = SwingAnchors(
        motion_start=300, top=353, impact=381, frame_count=600, fps=59.960,
        camera_id="down_the_line",
    )
    return a, b


def _bundle_9() -> tuple[SwingAnchors, SwingAnchors]:
    """The stored anchors of `2026-08-23/9`, the bundle the 2.08x render came from."""
    a = SwingAnchors(
        motion_start=469, top=534, impact=547, frame_count=726, fps=59.97521685254027,
        camera_id="face_on",
    )
    b = SwingAnchors(
        motion_start=244, top=297, impact=324, frame_count=450, fps=59.9602911978822,
        camera_id="down_the_line",
    )
    return a, b


def _bundle_10() -> tuple[SwingAnchors, SwingAnchors]:
    """Downswings that agree and backswings that do not - the guard's own case."""
    a = SwingAnchors(
        motion_start=700, top=752, impact=773, frame_count=900, fps=59.96, camera_id="face_on"
    )
    b = SwingAnchors(
        motion_start=1200, top=1243, impact=1263, frame_count=1400, fps=59.96,
        camera_id="down_the_line",
    )
    return a, b
