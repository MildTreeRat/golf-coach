"""Picking the real swing out of a clip that contains several. [M7 Phase 4]

`segment_phases` takes the earliest major descent, which is right for the single-swing GolfDB
corpus and wrong for a bay clip — on all four real bay clips it picks a *setup move*. The rule
under test filters candidates by downswing duration and then takes the last, and that is not
cosmetic: the window decides which frames get **scored**, so an unwindowed clip is graded on
the wrong swing.

Everything here is synthetic. `make_swing` builds one swing at a chosen tempo; joining several
end to end with `_concat` makes a recording with rehearsals in it, and because the builder is
exact we can say precisely which one the rule must land on.
"""

from __future__ import annotations

from conftest import make_swing

from golf_coach.analysis.phases import (
    _MATCH_TOLERANCE_S,
    _MIN_ADDRESS_LEAD_S,
    _PLAUSIBLE_DOWNSWING_S,
    _WINDOW_LEAD,
    _WINDOW_TRAIL,
    TRAIL_WRIST,
    candidate_downswings,
    segment_phases,
    select_matching_swing,
    select_swing,
    window_around,
)
from golf_coach.analysis.smoothing import smooth_keypoints
from golf_coach.contracts.keypoints import FrameKeypoints, PoseLandmark
from golf_coach.contracts.swing import SwingPhase

# The fixture builder emits one frame per 10 ms, so a clip made from it is 100 fps.
_FPS = 100.0

# Downswing frame counts that land inside / outside the plausible band at 100 fps.
_LOW, _HIGH = _PLAUSIBLE_DOWNSWING_S
_REAL = 25  # 0.25 s — a real downswing
_SLOW = 60  # 0.60 s — a setup move or rehearsal, above the band


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


def _smoothed(keypoints: list[FrameKeypoints]) -> list[FrameKeypoints]:
    return smooth_keypoints(keypoints)


def _prepend_still(clip: list[FrameKeypoints], frames: int) -> list[FrameKeypoints]:
    """Pad the front with copies of the first frame — a golfer standing over the ball."""
    return _concat([clip[0]] * frames, clip)


def test_band_constants_bracket_the_synthetic_durations() -> None:
    """Guard the fixture: these tests only mean anything if _REAL is in band and _SLOW is not."""
    assert _LOW <= _REAL / _FPS <= _HIGH
    assert _SLOW / _FPS > _HIGH


def test_no_fps_declines_rather_than_guessing() -> None:
    """A duration in seconds is meaningless without a frame rate, and legacy files have none."""
    assert select_swing(_smoothed(make_swing()), fps=None) is None
    assert select_swing(_smoothed(make_swing()), fps=0.0) is None


def test_picks_the_real_swing_over_an_earlier_rehearsal() -> None:
    """A slow rehearsal first, the real swing second — the earliest-descent rule gets this wrong."""
    clip = _concat(
        make_swing(backswing_frames=80, downswing_frames=_SLOW),
        make_swing(backswing_frames=70, downswing_frames=_REAL),
    )
    choice = select_swing(_smoothed(clip), fps=_FPS)

    assert choice is not None
    duration = (choice.downswing.impact - choice.downswing.top) / _FPS
    assert _LOW <= duration <= _HIGH, "the rehearsal's duration must not have been chosen"
    # The real swing is in the back half of the recording.
    assert choice.downswing.top > len(clip) // 2


def test_takes_the_last_plausible_swing_not_the_first() -> None:
    """Nobody takes a practice swing *after* hitting the ball, so later beats earlier."""
    clip = _concat(
        make_swing(backswing_frames=70, downswing_frames=_REAL),
        make_swing(backswing_frames=70, downswing_frames=_REAL),
    )
    choice = select_swing(_smoothed(clip), fps=_FPS)

    assert choice is not None
    plausible = [
        swing
        for swing in choice.candidates
        if _LOW <= (swing.impact - swing.top) / _FPS <= _HIGH
    ]
    assert len(plausible) > 1, "this clip must offer a genuine choice for the rule to make"
    assert choice.downswing == plausible[-1]
    assert "took the last" in choice.reason


def test_a_lone_candidate_wins_however_long_its_downswing_is() -> None:
    """The band exists to choose *between* candidates; with one there is nothing to choose.

    This is the 30 fps case: the 2026-08-07 bundle's face-on view holds a single swing whose
    downswing measures 0.60 s — above a band derived from 60 fps footage. Declining there threw
    the window away and scored the swing with the clip's dead air mixed in.
    """
    clip = make_swing(backswing_frames=80, downswing_frames=_SLOW)
    assert len(candidate_downswings(_smoothed(clip), min_fraction=0.45)) == 1

    choice = select_swing(_smoothed(clip), fps=_FPS)

    assert choice is not None
    assert choice.downswing.top > 0
    assert "nothing to choose between" in choice.reason


def test_declines_when_several_candidates_and_none_are_plausible() -> None:
    """Two rehearsals and no swing: say so rather than pick one of them.

    Note the candidates are not all *slow*. Joining two clips leaves a short artifact at the
    seam — the hands dropping from one clip's finish to the next one's address — which is a
    descent like any other, and lands below the band rather than above it.
    """
    clip = _concat(
        make_swing(backswing_frames=80, downswing_frames=_SLOW),
        make_swing(backswing_frames=80, downswing_frames=_SLOW),
    )
    candidates = candidate_downswings(_smoothed(clip), min_fraction=0.45)
    assert len(candidates) > 1, "this clip must offer more than one candidate"
    assert not any(_LOW <= (c.impact - c.top) / _FPS <= _HIGH for c in candidates)

    assert select_swing(_smoothed(clip), fps=_FPS) is None


def test_window_contains_the_swing_with_room_for_takeaway_and_finish() -> None:
    """The window must hold the whole swing, not just the descent.

    A lead of only a couple of downswings lands *inside* the backswing, which leaves motion
    start undetected — that drops the tempo checkpoint and degrades the alignment. The lead is
    sized from the tour-median backswing (~3.5 downswings) for exactly this reason.
    """
    # Padded at the front so the window is not clamped by the start of the clip, which is the
    # only thing allowed to shorten the lead.
    clip = _prepend_still(make_swing(backswing_frames=70, downswing_frames=_REAL), 200)
    choice = select_swing(_smoothed(clip), fps=_FPS)
    assert choice is not None

    start, end = choice.window
    assert choice.window == window_around(choice.downswing, fps=_FPS)
    assert start > 0, "the pad must leave the lead unclamped for this to test anything"
    assert end > choice.downswing.impact

    downswing = choice.downswing.impact - choice.downswing.top
    lead = choice.downswing.top - start
    assert lead >= 3.5 * downswing, "must reach back past a tour-tempo backswing"


def _dim_lead_wrist(clip: list[FrameKeypoints]) -> list[FrameKeypoints]:
    """Drop the lead wrist below `_MIN_VISIBILITY` everywhere — the down-the-line view.

    From behind the golfer the lead wrist is the **far** arm, occluded by the torso through the
    top and tracked in 39% of frames (`phases.TRAIL_WRIST`). The fixture animates both wrists
    together, so blanking the lead one's visibility is the only part of that view a synthetic
    clip can model — and it is the part selection depends on.
    """
    dimmed: list[FrameKeypoints] = []
    for frame in clip:
        landmarks = list(frame.landmarks)
        lead = landmarks[PoseLandmark.LEFT_WRIST]
        landmarks[PoseLandmark.LEFT_WRIST] = lead.model_copy(update={"visibility": 0.0})
        dimmed.append(frame.model_copy(update={"landmarks": landmarks}))
    return dimmed


def test_the_trail_wrist_finds_a_swing_the_lead_wrist_cannot_see() -> None:
    """The down-the-line case: one landmark is occluded and the other is not.

    This also pins `_wrist_confident`, which took a `wrist` argument and then read `_LEAD_WRIST`
    in its body. With that bug the trail-wrist call still masks every frame by the *lead* wrist's
    visibility, finds no confidently-tracked run, and declines exactly as the lead-wrist call
    does — so this test fails on the threading alone.
    """
    clip = _smoothed(_dim_lead_wrist(_prepend_still(make_swing(downswing_frames=_REAL), 40)))

    assert select_swing(clip, fps=_FPS) is None, "the lead wrist must be unreadable here"

    choice = select_swing(clip, fps=_FPS, wrist=TRAIL_WRIST)

    assert choice is not None
    assert _LOW <= (choice.downswing.impact - choice.downswing.top) / _FPS <= _HIGH


def test_the_default_landmark_is_the_lead_wrist() -> None:
    """Every stored window and every band was produced on the lead wrist; the default holds it.

    P5 changes the down-the-line call sites — `api.pipeline._auto_window` and
    `scripts/align_swings.py` — not the default. Pinned on both entry points because they are
    threaded separately and a default that drifted on only one of them would be invisible until
    a face-on window moved.
    """
    clip = _smoothed(_prepend_still(make_swing(downswing_frames=_REAL), 40))

    assert candidate_downswings(clip, min_fraction=0.45) == candidate_downswings(
        clip, min_fraction=0.45, wrist=PoseLandmark.LEFT_WRIST
    )
    assert select_swing(clip, fps=_FPS) == select_swing(
        clip, fps=_FPS, wrist=PoseLandmark.LEFT_WRIST
    )


def _windowed_backswing(
    clip: list[FrameKeypoints], window: tuple[int, int]
) -> tuple[bool, float, int]:
    """Segment the windowed clip and report `(motion start detected, backswing seconds, top)`.

    The top comes back in *absolute* frames so a window that moved can be told from one that only
    grew at the front — the property `_WINDOW_LEAD`'s comment claims and the address floor must
    not cost.
    """
    start, end = window
    segments = {segment.phase: segment for segment in segment_phases(clip[start:end])}
    backswing = segments[SwingPhase.BACKSWING]
    return (
        backswing.detected,
        (backswing.end_frame - backswing.start_frame) / _FPS,
        segments[SwingPhase.DOWNSWING].start_frame + start,
    )


def test_a_fast_downswing_still_leaves_a_quiet_address_in_the_window() -> None:
    """M10 §A4: the lead is floored in seconds, because the address it must hold is real time.

    `_WINDOW_LEAD * downswing` shrinks exactly where it must not. This swing's backswing is 110
    frames and its measured downswing 21, so five downswings reach back 105 frames — *inside* the
    backswing. `_motion_start` then finds no quiet run, the ADDRESS and BACKSWING segments come
    back `detected=False`, and the alignment falls to the estimate that multiplies that same short
    downswing by 3.5. Sessions 2026-08-23/2, /4 and /5 are this shape.
    """
    clip = _smoothed(_prepend_still(make_swing(backswing_frames=110, downswing_frames=20), 250))
    choice = select_swing(clip, fps=_FPS)
    assert choice is not None

    downswing = choice.downswing.impact - choice.downswing.top
    assert _WINDOW_LEAD * downswing < 110, "the fixture must under-reach for this to test anything"

    unfloored = window_around(choice.downswing)
    was_detected, _, top_before = _windowed_backswing(clip, unfloored)
    assert not was_detected, "without the floor this geometry loses the motion start"

    is_detected, backswing_s, top_after = _windowed_backswing(clip, choice.window)
    assert is_detected
    assert 0.8 <= backswing_s <= 1.2, "the recovered backswing must be a plausible one"
    assert top_after == top_before, "the floor may only add address, never move the swing"


def test_the_address_floor_binds_only_where_the_lead_is_short() -> None:
    """A slow downswing already reaches back far enough, and must be left alone.

    Nine of the fourteen stored face-on clips are this case — a 0.38-0.42 s downswing puts
    `5 * downswing` at 1.9-2.1 s, well past `_MIN_ADDRESS_LEAD_S`. The fixture is that shape.
    Widening those windows would be free room for a practice swing to enter, which is what the
    `_WINDOW_LEAD = 7` alternative was rejected for.
    """
    clip = _smoothed(_prepend_still(make_swing(backswing_frames=95, downswing_frames=38), 250))
    choice = select_swing(clip, fps=_FPS)
    assert choice is not None

    downswing = choice.downswing.impact - choice.downswing.top
    assert _WINDOW_LEAD * downswing > _MIN_ADDRESS_LEAD_S * _FPS
    assert choice.window == window_around(choice.downswing), "the floor must not have bound"
    assert _windowed_backswing(clip, choice.window)[0]


def test_without_fps_the_lead_is_the_old_arithmetic_exactly() -> None:
    """A clip whose frame rate was never recorded degrades rather than raising (ADR-013).

    Legacy `*.keypoints.json` files carry `clip=None`, and `scripts/align_swings.py --list-swings`
    prints a window for them. Seconds mean nothing there, so the floor cannot apply and the lead
    stays `_WINDOW_LEAD` downswings — the same no-fps path `alignment`'s `TOP_IMPACT` fallback
    keeps.
    """
    clip = _smoothed(_prepend_still(make_swing(backswing_frames=110, downswing_frames=20), 250))
    swing = candidate_downswings(clip, min_fraction=0.45)[0]

    downswing = swing.impact - swing.top
    assert window_around(swing) == (
        swing.top - _WINDOW_LEAD * downswing,
        swing.impact + _WINDOW_TRAIL * downswing,
    )
    assert window_around(swing, fps=None) == window_around(swing)
    assert window_around(swing, fps=_FPS) != window_around(swing), (
        "this fixture must be one the floor binds on, or the pin is vacuous"
    )


# The cross-view rule (M10 P7). Frame counts below are what the builder is *asked* for; the measured
# descent differs by a frame or two, so every assertion reads the duration back off the candidate
# rather than assuming it.
_LATE = 16  # a short descent after impact — in band, and nothing like a 0.25 s reference
_PAST_BAND = 48  # measures ~0.46 s: past the band, the shape sessions 3 and 6 have on DTL
_REFERENCE = 0.25  # what the face-on view measured for the swing being matched


def _durations(clip: list[FrameKeypoints]) -> list[float]:
    return [
        (swing.impact - swing.top) / _FPS
        for swing in candidate_downswings(clip, min_fraction=0.45)
    ]


def _two_swings(first: int, second: int) -> list[FrameKeypoints]:
    return _smoothed(
        _concat(
            make_swing(backswing_frames=70, downswing_frames=first),
            make_swing(backswing_frames=70, downswing_frames=second),
        )
    )


def test_the_match_fixtures_sit_where_the_cross_view_tests_claim() -> None:
    """Guard the fixtures: these tests mean nothing unless the durations land as described."""
    nearest = _durations(_two_swings(_REAL, _LATE))
    assert any(abs(d - _REFERENCE) < 0.02 for d in nearest), "one descent must be the reference's"
    late = nearest[-1]  # candidates come earliest-first, so the post-impact decoy is last
    assert _LOW <= late <= _HIGH and abs(late - _REFERENCE) <= _MATCH_TOLERANCE_S, (
        "the decoy must survive rule 1 on both branches, so that rule 2 is what rejects it"
    )
    assert abs(late - _REFERENCE) > 0.05, "and it must sit measurably farther from the reference"

    outside = _durations(_two_swings(_PAST_BAND, _SLOW))[0]
    assert outside > _HIGH, "the leading descent must miss the band"
    assert outside - _HIGH < 0.05, "but only just — session 3 misses it by 0.017 s"


def test_the_reference_picks_the_nearest_candidate_not_the_last() -> None:
    """M10 §A2, and the whole point of the phase.

    The down-the-line phone keeps rolling on the busy side of the bay, so the clip holds a descent
    *after* impact. `select_swing` is right to take the last when a clip is judged alone — nobody
    rehearses after hitting the ball — and that is exactly what goes wrong here: the late descent
    is plausible on its own and is not the swing the other camera filmed.

    The decoy here is admitted by *both* halves of rule 1 — it is in band and it is inside
    `_MATCH_TOLERANCE_S` of the reference — so the only thing that can reject it is rule 2. That is
    the harder half of the phase: a tolerance wide enough to rescue sessions 3 and 6 is also wide
    enough to admit their decoys.
    """
    clip = _two_swings(_REAL, _LATE)

    alone = select_swing(clip, fps=_FPS)
    assert alone is not None
    assert "took the last" in alone.reason

    matched = select_matching_swing(clip, fps=_FPS, reference_downswing_s=_REFERENCE)
    assert matched is not None
    assert matched.downswing != alone.downswing, "the independent rule must take the other one"
    assert matched.downswing.top < alone.downswing.top, "the swing is the earlier descent here"
    assert abs((matched.downswing.impact - matched.downswing.top) / _FPS - _REFERENCE) < 0.02
    # The reason names what it judged against, the way `select_swing`'s reasons do.
    assert f"{_REFERENCE:.2f}s reference" in matched.reason


def test_a_descent_just_outside_the_band_is_admitted_when_it_matches_the_reference() -> None:
    """M10 §A3: `2026-08-10/2` misses the band by 0.0003 s and session 3 by 0.017 s.

    Both decline today with the real swing sitting right there in the candidate list, because the
    band is the only evidence available. Another camera measuring the same descent is better
    evidence than the band, and it is evidence the band cannot see.
    """
    clip = _two_swings(_PAST_BAND, _SLOW)
    reference = _durations(clip)[0] - 0.04  # the other view, four frames short of this one

    assert select_swing(clip, fps=_FPS) is None, "the band alone must decline this clip"

    matched = select_matching_swing(clip, fps=_FPS, reference_downswing_s=reference)
    assert matched is not None
    assert matched.downswing == candidate_downswings(clip, min_fraction=0.45)[0]
    assert (matched.downswing.impact - matched.downswing.top) / _FPS > _HIGH
    assert "outside the usual" in matched.reason
    assert f"{reference:.2f}s reference" in matched.reason


def test_declines_when_nothing_is_plausible_or_near_the_reference() -> None:
    """Never guess (ADR-013). Two rehearsals and a seam artifact, and no swing among them."""
    clip = _two_swings(_SLOW, _SLOW)
    durations = _durations(clip)
    assert len(durations) > 1, "the lone-candidate escape must not be what is under test here"
    assert not any(
        _LOW <= d <= _HIGH or abs(d - _REFERENCE) <= _MATCH_TOLERANCE_S for d in durations
    )

    assert select_matching_swing(clip, fps=_FPS, reference_downswing_s=_REFERENCE) is None


def test_a_lone_candidate_still_wins_when_it_matches_nothing() -> None:
    """`2026-08-23/4`: one down-the-line descent, 0.484 s, against a 0.200 s face-on reference.

    Neither the band nor the reference admits it, and declining would throw away the window P5 won
    that bundle — the swing would be scored with 30 s of bay footage mixed in. With one candidate
    there is nothing to choose between, so both rules keep it.
    """
    clip = _smoothed(make_swing(backswing_frames=80, downswing_frames=_SLOW))
    only = candidate_downswings(clip, min_fraction=0.45)
    assert len(only) == 1
    duration = (only[0].impact - only[0].top) / _FPS
    assert duration > _HIGH and abs(duration - 0.20) > _MATCH_TOLERANCE_S

    matched = select_matching_swing(clip, fps=_FPS, reference_downswing_s=0.20)
    assert matched is not None
    assert matched.downswing == only[0]
    assert "nothing to choose between" in matched.reason
    assert matched == select_swing(clip, fps=_FPS), "one candidate is one answer, from either rule"


def test_no_fps_and_no_reference_both_decline() -> None:
    """A duration in seconds needs a frame rate, and this rule needs something to match against.

    The second case is the one a caller meets in practice: when the reference view declined there
    is no measured swing to match, and the answer is to judge this view on its own with
    `select_swing` — not to match against a reference of zero.
    """
    clip = _two_swings(_REAL, _LATE)
    assert select_matching_swing(clip, fps=None, reference_downswing_s=_REFERENCE) is None
    assert select_matching_swing(clip, fps=0.0, reference_downswing_s=_REFERENCE) is None
    assert select_matching_swing(clip, fps=_FPS, reference_downswing_s=0.0) is None
