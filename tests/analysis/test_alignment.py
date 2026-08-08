"""Two-view alignment: one swing, two differently-configured phones, one shared tau axis.

Everything here is synthetic — the point of a normalized swing-time axis is that it can be
reasoned about exactly, so these are equalities and bounds rather than eyeballed video. The
fixtures reuse `make_swing` from conftest, which is the same builder `test_phases.py` runs on.
"""

from __future__ import annotations

import pytest
from conftest import _ADDRESS_FRAMES, make_swing

from golf_coach.analysis.alignment import (
    _TEMPO_AGREEMENT,
    align_swings,
    anchors_from_keypoints,
    frame_of_tau,
    map_frame,
    pair_frames,
    tau_of_frame,
)
from golf_coach.analysis.phases import candidate_downswings
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


def test_both_clips_take_the_same_fallback_when_the_soft_anchor_is_dropped() -> None:
    """Degrade symmetrically or not at all — one panel drifting is worse than both drifting."""
    moving = make_swing(20, 14, takeaway_frames=60)[_ADDRESS_FRAMES:]
    settled = make_swing(20, 14, takeaway_frames=60)
    alignment = align_swings(_anchored(moving, 60.0), _anchored(settled, 60.0))

    assert alignment.a is not None and alignment.b is not None
    # Neither clip's warp uses its own detected motion start; both use the same substituted rule.
    assert alignment.b.warp_motion_start != alignment.b.anchors.motion_start
    assert _backswing_in_downswings(alignment.a) == pytest.approx(
        _backswing_in_downswings(alignment.b), abs=0.2
    )


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
