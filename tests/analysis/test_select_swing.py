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
    _PLAUSIBLE_DOWNSWING_S,
    candidate_downswings,
    select_swing,
    window_around,
)
from golf_coach.analysis.smoothing import smooth_keypoints
from golf_coach.contracts.keypoints import FrameKeypoints

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
    assert choice.window == window_around(choice.downswing)
    assert start > 0, "the pad must leave the lead unclamped for this to test anything"
    assert end > choice.downswing.impact

    downswing = choice.downswing.impact - choice.downswing.top
    lead = choice.downswing.top - start
    assert lead >= 3.5 * downswing, "must reach back past a tour-tempo backswing"
