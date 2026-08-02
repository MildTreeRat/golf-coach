"""Phase segmentation: a synthetic swing yields the six phases, in order, contiguous."""

from __future__ import annotations

from conftest import _ADDRESS_FRAMES, make_swing

from golf_coach.analysis.phases import _rising_runs, segment_phases
from golf_coach.analysis.smoothing import smooth_keypoints
from golf_coach.contracts.keypoints import FrameKeypoints
from golf_coach.contracts.swing import SwingPhase


def test_six_phases_in_canonical_order(swing: list[FrameKeypoints]) -> None:
    phases = segment_phases(swing)
    assert [p.phase for p in phases] == [
        SwingPhase.ADDRESS,
        SwingPhase.BACKSWING,
        SwingPhase.TRANSITION,
        SwingPhase.DOWNSWING,
        SwingPhase.IMPACT,
        SwingPhase.FOLLOW_THROUGH,
    ]


def test_segments_are_contiguous_and_monotonic(swing: list[FrameKeypoints]) -> None:
    phases = segment_phases(swing)
    # Each segment is well-formed and the chain covers the whole clip without gaps.
    for segment in phases:
        assert segment.start_frame <= segment.end_frame
        assert segment.start_ms <= segment.end_ms
    # Sliding pairwise window (intentionally uneven lengths), so strict=False.
    for earlier, later in zip(phases, phases[1:], strict=False):
        assert earlier.end_frame == later.start_frame
    assert phases[0].start_frame == 0
    assert phases[-1].end_frame == len(swing) - 1


def test_too_short_clip_returns_no_phases() -> None:
    assert segment_phases([]) == []


def test_top_detected_near_true_top_on_smoothed_swing() -> None:
    # make_swing(30, 10): 8 address frames + 30-frame backswing => the top (min wrist-y)
    # sits at ~frame 37. Smoothing (as engine does) must keep the detected top close to it.
    smoothed = smooth_keypoints(make_swing(30, 10))
    phases = segment_phases(smoothed)
    transition = next(p for p in phases if p.phase is SwingPhase.TRANSITION)
    top = (transition.start_frame + transition.end_frame) // 2
    assert 34 <= top <= 41


def test_top_survives_a_finish_higher_than_the_top() -> None:
    """The regression that GolfDB exposed. [M4-REF]

    A full tour finish leaves the hands **higher** than they ever were at the top of the backswing,
    so "top = global minimum of wrist y" locates the *finish* instead. Measured against GolfDB's
    hand-annotated events, the old rule was a median of 26 frames late and wrong on 80% of tour
    clips — and no clip we owned could have shown it, because none of them finish that high.

    Here the finish (`0.05`) is well above the top (`_TOP_Y == 0.15`). The detected top must stay
    on the backswing, not jump to the end of the clip.
    """
    smoothed = smooth_keypoints(make_swing(30, 10, finish_y=0.05))
    phases = segment_phases(smoothed)
    transition = next(p for p in phases if p.phase is SwingPhase.TRANSITION)
    top = (transition.start_frame + transition.end_frame) // 2
    assert 34 <= top <= 41, f"top landed at {top} — likely the finish, not the top of backswing"

    # And the swing must still segment sanely around it, rather than collapsing to a sliver.
    impact = next(p for p in phases if p.phase is SwingPhase.IMPACT)
    assert top < impact.start_frame < len(smoothed) - 1


def test_rising_runs_ignore_untracked_frames() -> None:
    """Held-forward values must not manufacture a descent. [M4-REF]

    `_lead_wrist_xy` carries the last confident position through dim frames to keep the series
    continuous. That is right for smoothing and wrong for finding extrema: the held stretch is not
    evidence of where the hands went. Here the untracked middle sits far below the tracked frames,
    which without the confidence mask would read as by far the largest descent in the clip.
    """
    ys = [0.50, 0.40, 0.30, 0.95, 0.95, 0.95, 0.32, 0.40, 0.50]
    confident = [True, True, True, False, False, False, True, True, True]

    runs = _rising_runs(ys, confident)

    assert runs, "the genuine 0.30 -> 0.50 descent should still be found"
    assert max(rise for rise, _, _ in runs) < 0.30, (
        "a run spanning the untracked plateau was scored — the 0.30 -> 0.95 jump is an artefact "
        "of holding the last good value, not a descent of the hands"
    )
    assert all(not 3 <= end <= 5 for _, _, end in runs)


def test_motion_start_includes_horizontal_takeaway() -> None:
    # A 20-frame near-horizontal takeaway (lead wrist sliding sideways at address height) sits
    # between the 8-frame address dwell and the vertical rise (~frame 28+). A wrist-*height* rule
    # can't see that sideways move and anchors motion-start at the vertical-rise start, collapsing
    # the backswing; the 2D-speed rule must anchor it back near the takeaway start (~frame 8).
    smoothed = smooth_keypoints(make_swing(20, 13, takeaway_frames=20))
    phases = segment_phases(smoothed)
    backswing = next(p for p in phases if p.phase is SwingPhase.BACKSWING)
    assert backswing.start_frame <= _ADDRESS_FRAMES + 4  # not the ~frame-28 vertical-rise start


def _address_end(keypoints: list[FrameKeypoints]) -> int:
    phases = segment_phases(smooth_keypoints(keypoints))
    return next(p for p in phases if p.phase is SwingPhase.ADDRESS).end_frame


def test_motion_start_is_frame_rate_invariant() -> None:
    """The same swing at 3x the frame rate must still put the boundary at the end of the dwell.

    The rule's quiet run is a fraction of the clip's own downswing (ADR-013), so stretching every
    phase by a common factor must not move the boundary relative to the swing. The superseded
    fixed 4-frame stall was the one fps-dependent absolute in this path.

    Note what this can and cannot show: the fixture's setup is *perfectly* still, so both the old
    and new rules find the dwell here. The failure this guards against needs a noisy setup and a
    gradual takeaway, which is measured on the 461-clip GolfDB corpus by
    `scripts/golfdb/tune_address.py`, not in a unit test. What it does lock down is that the
    boundary tracks the swing rather than the frame count.
    """
    fast = make_swing(20, 8, takeaway_frames=6)
    slow = make_swing(60, 24, takeaway_frames=18)

    # Address dwell is a fixed 8 frames in both, so the boundary should land at its end either
    # way rather than drifting into the (3x longer) takeaway.
    assert _address_end(fast) <= _ADDRESS_FRAMES
    assert _address_end(slow) <= _ADDRESS_FRAMES


def test_quiet_run_scales_with_the_downswing() -> None:
    """A longer downswing demands a longer stretch of stillness before calling it the takeaway."""
    from golf_coach.analysis.phases import _MOTION_STALL_FRACTION, _MOTION_STALL_MIN_FRAMES

    assert max(_MOTION_STALL_MIN_FRAMES, round(_MOTION_STALL_FRACTION * 8)) == 2
    assert max(_MOTION_STALL_MIN_FRAMES, round(_MOTION_STALL_FRACTION * 40)) == 10


def test_detected_flag_is_true_when_the_wrist_settles(swing: list[FrameKeypoints]) -> None:
    phases = segment_phases(smooth_keypoints(swing))
    by_phase = {p.phase: p for p in phases}
    assert by_phase[SwingPhase.ADDRESS].detected
    assert by_phase[SwingPhase.BACKSWING].detected
    # Every other segment is unconditionally detected — the flag only describes this one boundary.
    assert all(p.detected for p in phases)


def test_never_settling_falls_back_to_a_bounded_estimate() -> None:
    """A clip with no still setup must not answer frame 0, and must admit it guessed.

    `make_swing(takeaway_frames=...)` with no address dwell is the degenerate case: the wrist is
    moving from the first frame, so no quiet run exists. The superseded rule returned 0, which on
    GolfDB's median 59 frames of pre-roll cost those clips a median of 31 frames. The estimate has
    to be bounded *and* flagged, because it is derived from an assumed tempo ratio and so is
    circular for anything that divides by it.
    """
    from golf_coach.analysis.phases import _FALLBACK_TEMPO_RATIO

    # A long takeaway with the address dwell stripped: the wrist moves from the first frame, so
    # no quiet run exists anywhere. The takeaway is long enough that the prior lands well clear of
    # zero, which is what makes this a test of the *estimate* rather than of the clamp.
    moving = make_swing(20, 14, takeaway_frames=60)[_ADDRESS_FRAMES:]
    phases = segment_phases(smooth_keypoints(moving))
    by_phase = {p.phase: p for p in phases}
    address = by_phase[SwingPhase.ADDRESS]

    assert not address.detected
    assert not by_phase[SwingPhase.BACKSWING].detected

    transition = by_phase[SwingPhase.TRANSITION]
    top = (transition.start_frame + transition.end_frame) // 2
    impact = by_phase[SwingPhase.IMPACT].start_frame
    expected = max(0, top - round(_FALLBACK_TEMPO_RATIO * max(impact - top, 1)))
    assert address.end_frame == expected
    assert address.end_frame > 0, "the bounded estimate, not the frame-0 answer it replaced"
