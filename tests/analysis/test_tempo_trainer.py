"""Building a metronome from the tour's durations. [ADR-023]

Runs against the **shipped** `golfdb_v1.json` rather than a fixture, deliberately: the thing most
worth pinning is that the numbers a golfer practices to come out of the committed corpus, and a
stubbed distribution would pin the arithmetic while letting the wiring rot.
"""

from __future__ import annotations

import re
from datetime import UTC, datetime

import pytest

from golf_coach.analysis import tempo_trainer
from golf_coach.analysis.benchmarks.distributions import load_distribution
from golf_coach.analysis.phases import POSSIBLE_DOWNSWING_S
from golf_coach.analysis.tempo_trainer import (
    build_career_tempo,
    build_tempo_plan,
    build_tempo_plan_for,
)
from golf_coach.api.app import _STATIC_DIR
from golf_coach.contracts.baseline import BaselineClaim, minimum_n
from golf_coach.contracts.career import CareerCorpus, CorpusSwing
from golf_coach.contracts.swing import Measurement, PhaseSegment, SwingPhase
from golf_coach.contracts.tempo import BeatRole, TempoAnchor, TempoPattern


def _phases(backswing_ms: float, downswing_ms: float) -> list[PhaseSegment]:
    """A segmentation that yields exactly these two durations through `tempo_timings`.

    `tempo_timings` reads motion start, the *center* of the transition window, and impact — so the
    transition is built symmetrically around the top rather than starting at it.
    """
    top = backswing_ms
    impact = top + downswing_ms
    return [
        PhaseSegment(phase=SwingPhase.ADDRESS, start_frame=0, end_frame=1,
                     start_ms=0.0, end_ms=0.0, detected=True),
        PhaseSegment(phase=SwingPhase.BACKSWING, start_frame=1, end_frame=2,
                     start_ms=0.0, end_ms=top - 10, detected=True),
        PhaseSegment(phase=SwingPhase.TRANSITION, start_frame=2, end_frame=3,
                     start_ms=top - 10, end_ms=top + 10, detected=True),
        PhaseSegment(phase=SwingPhase.DOWNSWING, start_frame=3, end_frame=4,
                     start_ms=top + 10, end_ms=impact, detected=True),
        PhaseSegment(phase=SwingPhase.IMPACT, start_frame=4, end_frame=5,
                     start_ms=impact, end_ms=impact + 20, detected=True),
        PhaseSegment(phase=SwingPhase.FOLLOW_THROUGH, start_frame=5, end_frame=6,
                     start_ms=impact + 20, end_ms=impact + 500, detected=True),
    ]


def test_both_patterns_are_built_and_the_grid_is_the_default() -> None:
    """Order is load-bearing — the page plays `patterns[0]` before anyone touches the toggle."""
    plan = build_tempo_plan([])

    assert plan is not None
    assert [p.mode for p in plan.patterns] == [TempoPattern.GRID, TempoPattern.CUES]
    assert plan.default_pattern.mode is TempoPattern.GRID


def test_the_grid_ticks_evenly_and_its_ratio_is_exactly_an_integer() -> None:
    """What the snap buys, and what it costs, in one assertion each.

    The tick count is *derived* from the tour medians. It resolves to 3 today, which is why the
    assertion is `is_integer()` and a spacing check rather than the literal 3 — writing 3 here
    would pin an output as though it were a constant, and it would stop being true the day the
    corpus is re-derived.
    """
    grid = build_tempo_plan([]).patterns[0]

    gaps = [b.at_ms - a.at_ms for a, b in zip(grid.beats, grid.beats[1:], strict=False)]
    assert gaps == pytest.approx([gaps[0]] * len(gaps))
    assert grid.ratio.is_integer()
    assert grid.ratio == pytest.approx(grid.backswing_ms / grid.downswing_ms)


def test_the_cues_pattern_keeps_the_unrounded_quotient_of_the_tour_medians() -> None:
    """The half the grid gives up. If these two agreed, one of the patterns would be pointless."""
    backswing = load_distribution("backswing_ms")
    downswing = load_distribution("downswing_ms")
    grid, cues = build_tempo_plan([]).patterns

    assert cues.ratio == pytest.approx(backswing.p50 / downswing.p50)
    assert cues.backswing_ms == pytest.approx(backswing.p50)
    assert cues.downswing_ms == pytest.approx(downswing.p50)
    assert cues.ratio != pytest.approx(grid.ratio)


def test_the_cues_pattern_is_three_beats_and_the_grid_is_more() -> None:
    grid, cues = build_tempo_plan([]).patterns

    assert [b.role for b in cues.beats] == [BeatRole.TAKEAWAY, BeatRole.TOP, BeatRole.IMPACT]
    assert BeatRole.SUBDIVISION in {b.role for b in grid.beats}
    assert len(grid.beats) > len(cues.beats)


def test_the_patterns_are_never_pre_scaled_by_the_pace() -> None:
    """One place applies a pace, and it is the renderer.

    The builder used to take a `pace` and multiply the beats by it *while* the page multiplied
    again by its slider — a double application waiting for the first caller to pass one. Nothing
    ever did, so it never bit. The parameter is gone rather than documented, and this is what
    stops it coming back: whatever the fitted pace, the stored beats are the tour reference.
    """
    fitted = build_tempo_plan(_phases(backswing_ms=1150.0, downswing_ms=400.0))
    plain = build_tempo_plan([])

    assert fitted.pace != pytest.approx(1.0), "this fixture must be fitted, or it proves nothing"
    for a, b in zip(plain.patterns, fitted.patterns, strict=True):
        assert [x.at_ms for x in a.beats] == pytest.approx([x.at_ms for x in b.beats])
    assert fitted.patterns[1].downswing_ms == pytest.approx(load_distribution("downswing_ms").p50)


def test_the_observed_side_is_present_when_tempo_was_timeable() -> None:
    plan = build_tempo_plan(_phases(backswing_ms=901.0, downswing_ms=384.0))

    assert plan.observed_backswing_ms == pytest.approx(901.0)
    assert plan.observed_downswing_ms == pytest.approx(384.0)


def test_the_observed_side_is_absent_when_tempo_could_not_be_measured() -> None:
    """An unsegmented swing still gets a metronome — the targets do not depend on the golfer.

    This is the case the trainer exists for as much as any: the tempo checkpoint went unscored,
    so there is nothing to compare against, and the tour's durations are no less true.
    """
    plan = build_tempo_plan([])

    assert plan is not None
    assert plan.observed_backswing_ms is None
    assert plan.observed_downswing_ms is None


def test_an_estimated_backswing_boundary_leaves_the_observed_side_empty() -> None:
    """`detected=False` means the boundary came from an assumed tempo (ADR-013).

    Reporting it back as this golfer's measured tempo would echo the assumption at them as an
    observation — and next to a metronome, it would be an assumption they then practice to. It
    also means such a swing is never *anchored* to, which is the more important consequence.
    """
    phases = _phases(backswing_ms=901.0, downswing_ms=384.0)
    phases[1] = phases[1].model_copy(update={"detected": False})

    plan = build_tempo_plan(phases)

    assert plan.observed_backswing_ms is None
    assert plan.observed_downswing_ms is None
    assert plan.anchored is False


def test_the_plan_refuses_when_a_reference_distribution_is_missing(monkeypatch) -> None:
    """No fallback constant, on purpose: a golfer would practice to an invented tempo.

    ADR-010 §2 for a checkpoint that cannot be measured, applied to a target that cannot be
    resolved — and the stakes are higher here, because a wrong verdict is read once and a wrong
    metronome is rehearsed.
    """
    monkeypatch.setattr(tempo_trainer, "load_distribution", lambda *a, **k: None)

    assert build_tempo_plan([]) is None


def test_every_pattern_says_where_its_numbers_came_from() -> None:
    """Provenance travels with the numbers rather than being looked up by whoever renders them."""
    for pattern in build_tempo_plan([]).patterns:
        assert "GolfDB" in pattern.source
        assert "n=" in pattern.source


# --------------------------------------------------------------------- anchoring to the golfer
#
# The trainer's answer to "a 70 mph swing and a 100 mph swing have different tempos". They do —
# LPGA against PGA, driver only and one vote per golfer, the backswing runs 1001 ms against 834
# and the downswing 267 against 234. Club is *not* that axis (between-club sd 6.9 ms), which is
# why the first version of this module targeted the tour median for everyone. See ADR-023's
# 2026-08-20 addendum. Anchoring reads a duration the golfer actually swung instead of inferring a
# speed we cannot measure — every stored shot's smash factor is below 1.0, so there is no usable
# club-head speed.
#
# **Which duration is the 2026-09-02 addendum: the downswing.** It is the half a golfer feels, so
# it is the given; the backswing is the half they can change, so it is what the plan prescribes.
# These tests were written against the opposite anchor and read as their own change log.
#
# The fit is carried by `pace` rather than baked into the beats, so these assert on what a golfer
# actually *hears*: the stored time multiplied by the plan's pace.


def _heard(plan, mode: int = 1) -> tuple[float, float]:
    """`(backswing, downswing)` in milliseconds as played. `CUES` by default — the exact pattern."""
    pattern = plan.patterns[mode]
    return pattern.backswing_ms * plan.pace, pattern.downswing_ms * plan.pace


def test_a_slower_golfer_gets_a_slower_target_and_a_quicker_one_gets_quicker() -> None:
    """The whole point: one target for every golfer hands a slow swinger a fast swinger's swing."""
    slow = build_tempo_plan(_phases(backswing_ms=1001.0, downswing_ms=300.0))
    quick = build_tempo_plan(_phases(backswing_ms=750.0, downswing_ms=250.0))

    assert slow.anchored and quick.anchored
    assert slow.anchor_downswing_ms == pytest.approx(300.0)
    assert quick.anchor_downswing_ms == pytest.approx(250.0)
    # The prescribed halves follow the anchors rather than the golfers' own backswings, which is
    # the milestone: neither 1001 nor 750 appears anywhere in the target.
    assert slow.anchor_backswing_ms > quick.anchor_backswing_ms
    assert slow.pace > 1.0 > quick.pace

    for mode in (0, 1):
        slow_back, slow_down = _heard(slow, mode)
        quick_back, quick_down = _heard(quick, mode)
        assert slow_back > quick_back and slow_down > quick_down


def test_the_pace_is_the_anchor_against_the_tour_median() -> None:
    """What the golfer's slider opens at, and the number it means.

    A pace of 1.12 says "played 12% slower than the tour median *downswing*" — legible only because
    the patterns stay at that median. It is also what the page pre-sets the control to, and the
    reason M13 P3 has to widen the control: this divisor's range is far wider than the backswing's.
    """
    median = load_distribution("downswing_ms").p50
    plan = build_tempo_plan(_phases(backswing_ms=1001.0, downswing_ms=300.0))

    assert plan.pace == pytest.approx(300.0 / median)
    assert build_tempo_plan([]).pace == pytest.approx(1.0)


def test_anchoring_moves_what_is_heard_and_never_the_ratio() -> None:
    """It is the anchor's *length* that follows the golfer, never the shape.

    The ratio is what the tempo checkpoint judges, so the drill has to keep teaching the tour's —
    a trainer that also personalised the ratio would coach the golfer toward their own fault. It
    is pace-invariant by construction, which is the point of scaling both halves.
    """
    default = build_tempo_plan([])
    slow = build_tempo_plan(_phases(backswing_ms=1150.0, downswing_ms=400.0))

    for plain, fitted in zip(default.patterns, slow.patterns, strict=True):
        assert fitted.ratio == pytest.approx(plain.ratio)
    for mode in (0, 1):
        assert _heard(slow, mode)[1] > _heard(default, mode)[1]


def test_the_target_is_the_tour_ratio_applied_to_the_golfers_own_downswing() -> None:
    """`CUES` is the exact pattern, so the arithmetic is checkable on what it plays.

    Read the played downswing back and it is the golfer's own to the millisecond; the backswing is
    the one being asked for, and it is deliberately *not* the 1001 ms they swung — that difference
    is the whole instruction. `anchor_backswing_ms` carries it so no page has to multiply.
    """
    tour_ratio = load_distribution("backswing_ms").p50 / load_distribution("downswing_ms").p50
    plan = build_tempo_plan(_phases(1001.0, 300.0))
    backswing, downswing = _heard(plan)

    assert downswing == pytest.approx(300.0)
    assert backswing == pytest.approx(300.0 * tour_ratio)
    assert plan.anchor_backswing_ms == pytest.approx(backswing)
    assert plan.anchor_backswing_ms != pytest.approx(1001.0)


@pytest.mark.parametrize("downswing_ms", [60.0, 700.0])
def test_a_downswing_outside_the_tour_range_is_anchored_to_anyway_and_flagged(
    downswing_ms: float,
) -> None:
    """The deleted guard, and the thing that replaced it. [ADR-023 addendum 2026-09-02]

    Its predecessor refused an out-of-range *backswing*, because such a backswing is plausibly the
    fault and a drill built on it rehearses the error. That does not carry over: the downswing is
    the given, never the fault, so substituting the tour median for it would hand the golfer a
    target fitted to nobody while claiming a fit — the exact state M13 exists to leave.

    So the plan anchors and *reports*: the notice reads the bool, the snap control reads the
    None-ness of the pace, and the golfer decides. A mis-segmented 60 ms downswing is caught by the
    same sentence rather than by a silent substitution.
    """
    reference = load_distribution("downswing_ms")
    plan = build_tempo_plan(_phases(backswing_ms=900.0, downswing_ms=downswing_ms))

    assert plan.anchored is True
    assert plan.anchor_downswing_ms == pytest.approx(downswing_ms)
    assert plan.downswing_in_tour_range is False

    edge = reference.p10 if downswing_ms < reference.p10 else reference.p90
    assert plan.in_range_pace == pytest.approx(edge / reference.p50), (
        "the snap has to land on the nearest edge, which is the side the golfer fell off"
    )


def test_a_downswing_inside_the_tour_range_is_offered_nothing_to_snap_to() -> None:
    """The other half of the offer, and the reason `in_range_pace` is optional rather than always
    populated: a control that renders on a value it should ignore is a control that moves a golfer
    off their own measured downswing for no reason either surface can see.
    """
    plan = build_tempo_plan(_phases(backswing_ms=900.0, downswing_ms=267.0))

    assert plan.downswing_in_tour_range is True
    assert plan.in_range_pace is None


def test_every_fitted_pace_is_reachable_on_the_pages_slider() -> None:
    """The control has to be able to open where the fitter put it.

    The anchor is the golfer's own downswing with nothing left to reject it — M13 P2 deleted the
    guard — so the paces the fitter can produce are exactly the downswings the segmenter will hand
    it, over the tour median it divides by. That range is `phases.POSSIBLE_DOWNSWING_S`. A slider
    narrower than it clamps the opening value and quietly plays a tempo other than the one the
    page's own text claims, silently, because both numbers look fine.

    This read `backswing_ms`'s p10-p90 until P3, which was the old guard's range written in the
    old anchor's units: still green the moment P2 made the fit reach 144%, because it was measuring
    a distribution the fitter had stopped consulting. A pin sourced from the deleted rule is worse
    than no pin, and that is what re-sourcing it fixes.

    The bounds are read out of `tempo.js` rather than restated here, so a fit that outgrows the
    control fails this instead of shipping. They live there with the rest of the trainer's
    mechanism because career mode grew a second control: two pairs of bounds is two ways for this
    to be true of one page and false of the other.

    What it does not prove: `POSSIBLE_DOWNSWING_S` is the window `phases.py` admits *on its own*,
    and it will admit a descent outside it when the other view vouches for it (`phases.py`'s
    matching route), while `tempo_timings` measures top-to-impact from the centre of the transition
    window rather than the start of the descent. So this is the designed range, not a hard bound on
    `plan.pace` — which is why `wire()` still clamps the opening value as a backstop.
    """
    page = (_STATIC_DIR / "tempo.js").read_text(encoding="utf-8")
    control = re.search(r'id="tempoPace"[^>]*', page).group(0)
    low = int(re.search(r'min="(\d+)"', control).group(1))
    high = int(re.search(r'max="(\d+)"', control).group(1))

    downswing = load_distribution("downswing_ms")
    possible_low_ms, possible_high_ms = (1000 * seconds for seconds in POSSIBLE_DOWNSWING_S)
    reachable_low = 100 * possible_low_ms / downswing.p50
    reachable_high = 100 * possible_high_ms / downswing.p50

    assert low <= reachable_low and reachable_high <= high, (
        f"the fitter can produce {reachable_low:.0f}%-{reachable_high:.0f}%; the pace control is "
        f"min={low} max={high} and cannot reach it"
    )


def test_the_unanchored_plan_is_exactly_what_shipped_before_anchoring() -> None:
    """Anchoring is a generalization, not a change of default.

    "Unanchored" now means one thing only — no downswing was measured — since M13 P2 deleted the
    guard that could also refuse a measured one. In that state the anchor is the tour median, the
    pace is 1.0, and the arithmetic collapses back onto the original: `GRID` keeps the tour
    downswing exactly and `CUES` both medians.
    """
    backswing = load_distribution("backswing_ms")
    downswing = load_distribution("downswing_ms")
    plan = build_tempo_plan([])
    grid, cues = plan.patterns

    assert plan.pace == pytest.approx(1.0)
    assert cues.backswing_ms == pytest.approx(backswing.p50)
    assert cues.downswing_ms == pytest.approx(downswing.p50)
    assert grid.downswing_ms == pytest.approx(downswing.p50)


def test_the_source_prose_says_which_downswing_the_target_was_built_on() -> None:
    fitted = build_tempo_plan(_phases(backswing_ms=1001.0, downswing_ms=300.0))
    defaulted = build_tempo_plan([])

    assert "your own downswing" in fitted.patterns[0].source
    assert "tour median downswing" in defaulted.patterns[0].source


# ------------------------------------------------------------------ career scope [ADR-023 add.]
#
# What is pinned here is the *layering* — measurements print, claims are withheld, the anchor
# degrades in one direction — and not the statistics under it, which are `test_baseline.py`'s.
# Corpora are built by constructing the contracts directly, the same posture that file takes: a
# guard failure and a reader failure must not be able to look alike.


def _tempo_swing(
    index: int, *, ratio: float, backswing: float | None = None, downswing: float | None = None,
) -> CorpusSwing:
    """One distinct swing carrying a tempo reading, and optionally the two halves behind it."""
    values: dict[str, tuple[str, float]] = {"tempo_ratio": ("ratio", ratio)}
    if backswing is not None:
        values["backswing_ms"] = ("ms", backswing)
    if downswing is not None:
        values["downswing_ms"] = ("ms", downswing)
    return CorpusSwing(
        player_id="aaron",
        session_id=f"2026-08-{index:02d}",
        swing_id=str(index),
        captured_at=datetime(2026, 8, index, 12, tzinfo=UTC),
        face_on_sha256=f"clip-{index}",
        analyzed=True,
        measurements=[
            Measurement(name=name, value=value, unit=unit, source="pose:face_on", detail="test")
            for name, (unit, value) in values.items()
        ],
    )


def _tempo_corpus(*swings: CorpusSwing) -> CareerCorpus:
    return CareerCorpus(player_id="aaron", swings=list(swings))


def test_every_reading_is_printed_while_the_mean_over_them_is_withheld() -> None:
    """The property the whole career view rests on, and the reason it ships useful at n=2.

    A measurement asserts nothing about the golfer, so `swings` is never gated; a mean asserts a
    tendency, so it is. Collapsing the two would either print a baseline over two swings — the one
    thing career mode exists to prevent — or refuse a golfer numbers sitting in their own
    artifacts.
    """
    tempo = build_career_tempo(_tempo_corpus(
        _tempo_swing(7, ratio=2.42, backswing=968.0, downswing=400.6),
        _tempo_swing(10, ratio=2.35, backswing=901.2, downswing=383.9),
    ))

    assert [s.ratio for s in tempo.swings] == [2.42, 2.35]
    assert tempo.latest.ratio == 2.35
    assert tempo.typical_ratio is None
    assert tempo.typical_backswing_ms is None
    assert tempo.withheld, "an absent mean must arrive with the floor it is waiting for"


def test_the_typical_value_lands_once_the_center_guard_lifts() -> None:
    """The other half of the same property: the refusal has to actually be liftable.

    `tempo_ratio` carries an override (8, against the default 5) because it is the noisiest metric
    in the panel, so the floor is read from `minimum_n` rather than written here — a floor revised
    by a real bay session should move this test's input, not break it.
    """
    n = minimum_n("tempo_ratio", BaselineClaim.CENTER)
    tempo = build_career_tempo(_tempo_corpus(*(
        _tempo_swing(i, ratio=2.4, backswing=900.0, downswing=375.0) for i in range(1, n + 1)
    )))

    assert tempo.typical_ratio == pytest.approx(2.4)
    assert tempo.typical_backswing_ms == pytest.approx(900.0)
    # The mean downswing is what the anchor is selected on since M13 P4; the mean backswing rides
    # along as the other half of the "yours" reading and no longer decides anything.
    assert tempo.typical_downswing_ms == pytest.approx(375.0)
    assert tempo.anchor is TempoAnchor.CAREER_MEAN
    assert tempo.plan.anchor_downswing_ms == pytest.approx(375.0)


def test_the_anchor_falls_back_to_the_latest_swing_before_the_tour_median() -> None:
    """The middle rung, and the reason `TempoAnchor` has three values rather than a bool.

    A golfer whose mean is withheld still has a measured downswing, and one measured swing is a
    better target than a population median. It is not a mean smuggled past the guard — it is one
    swing's measurement used as a target, exactly as the results page has done since ADR-023.
    """
    tempo = build_career_tempo(_tempo_corpus(
        _tempo_swing(7, ratio=2.42, backswing=968.0, downswing=400.6),
        _tempo_swing(10, ratio=2.35, backswing=901.2, downswing=383.9),
    ))

    assert tempo.anchor is TempoAnchor.LATEST_SWING
    assert tempo.plan.anchored is True
    assert tempo.plan.anchor_downswing_ms == pytest.approx(383.9)


def test_a_ratio_with_no_halves_behind_it_still_counts_as_history() -> None:
    """`backswing_ms` and `downswing_ms` are a later measurement than the ratio.

    An artifact from an engine that wrote only `tempo_ratio` has a readable tempo and no durations.
    Dropping it would shorten the history for a reason no reader could see; anchoring on it is
    impossible, so the target falls back to the tour median and says so.
    """
    tempo = build_career_tempo(_tempo_corpus(_tempo_swing(7, ratio=2.42)))

    assert [s.ratio for s in tempo.swings] == [2.42]
    assert tempo.swings[0].backswing_ms is None
    assert tempo.anchor is TempoAnchor.TOUR_MEDIAN
    assert tempo.plan.anchored is False


def test_the_halves_are_joined_by_swing_and_never_zipped_positionally() -> None:
    """The bug this join exists to make impossible.

    Three parallel lists paired by index put one swing's ratio beside another's durations the first
    time the metrics cover different swings — and every number still looks plausible.
    """
    tempo = build_career_tempo(_tempo_corpus(
        _tempo_swing(7, ratio=2.42),
        _tempo_swing(10, ratio=2.35, backswing=901.2, downswing=383.9),
    ))

    by_ref = {s.swing_ref: s for s in tempo.swings}
    assert by_ref["2026-08-07/7"].backswing_ms is None
    assert by_ref["2026-08-10/10"].backswing_ms == pytest.approx(901.2)


def test_a_backswing_with_no_downswing_behind_it_anchors_nothing() -> None:
    """The half selected on is the half fitted to, so a lone backswing is not a target. [M13 P4]

    The two halves are separate measurements joined by `swing_ref`, so a swing can carry one and
    not the other — that asymmetry is the whole reason `_tempo_swings` joins rather than zips. A
    backswing alone used to select `LATEST_SWING` while the builder, reading the downswing, fell
    back to the tour median; the view then said "matched to your own swing" over a drill built on
    a population. Both now read the downswing, so this swing degrades to the median in one place.
    """
    tempo = build_career_tempo(_tempo_corpus(_tempo_swing(7, ratio=2.42, backswing=968.0)))

    assert tempo.anchor is TempoAnchor.TOUR_MEDIAN
    assert tempo.plan.anchored is False
    # The reading itself is untouched — the anchor governs the target, not the measurement.
    assert tempo.swings[0].backswing_ms == pytest.approx(968.0)


def test_a_downswing_with_no_backswing_behind_it_is_still_a_target() -> None:
    """The other side of that asymmetry, and it goes the other way. [M13 P4]

    The anchor needs one number and it is the downswing. A swing whose backswing could not be read
    still says how hard the golfer swung, which is the whole input to the fit — the prescription
    lands on the backswing rather than coming from it. `observed_backswing_ms` stays `None` so the
    page's "yours 968 / 400" line simply does not print, rather than pairing a real half with a
    borrowed one.
    """
    tempo = build_career_tempo(_tempo_corpus(_tempo_swing(7, ratio=2.42, downswing=383.9)))

    assert tempo.anchor is TempoAnchor.LATEST_SWING
    assert tempo.plan.anchored is True
    assert tempo.plan.anchor_downswing_ms == pytest.approx(383.9)
    assert tempo.plan.observed_backswing_ms is None


def test_the_career_anchor_is_read_back_off_the_plan_and_never_predicted() -> None:
    """`anchor` reports the builder's decision; it does not forecast it.

    Since M13 P4 the read-back cannot change the answer — the branch selects on a `downswing_ms`
    and `_anchor_downswing` anchors on any downswing it is given, the range guard that could once
    refuse one having gone with P2. What is pinned here is that the reporting still *goes through*
    the plan: every selection agrees with what the builder decided, which is the property that
    would break first if a second copy of the anchor rule appeared in this branch.
    """
    corpora = {
        "career mean": _tempo_corpus(*(
            _tempo_swing(i, ratio=2.4, backswing=900.0, downswing=375.0)
            for i in range(1, minimum_n("tempo_ratio", BaselineClaim.CENTER) + 1)
        )),
        "latest swing": _tempo_corpus(_tempo_swing(7, ratio=2.42, downswing=400.6)),
        "backswing only": _tempo_corpus(_tempo_swing(7, ratio=2.42, backswing=968.0)),
        "ratio only": _tempo_corpus(_tempo_swing(7, ratio=2.42)),
        "no swings": _tempo_corpus(),
    }

    for name, corpus in corpora.items():
        tempo = build_career_tempo(corpus)
        anchored = tempo.anchor is not TempoAnchor.TOUR_MEDIAN
        assert tempo.plan.anchored is anchored, f"{name}: anchor and plan disagree"


def test_an_empty_corpus_is_an_empty_history_and_still_offers_the_tour_target() -> None:
    """The first true answer about every golfer, and it is not an error.

    The trainer still builds: a golfer with no swings on file can practice the tour tempo, and
    refusing them the metronome as well as the history would be a refusal nobody asked for.
    """
    tempo = build_career_tempo(_tempo_corpus())

    assert tempo.swings == ()
    assert tempo.latest is None
    assert tempo.typical_ratio is None
    assert tempo.anchor is TempoAnchor.TOUR_MEDIAN
    assert tempo.plan is not None


def test_the_refusals_are_deduplicated_and_strictest_first() -> None:
    """Three metrics, two distinct floors, and an order a reader can act on.

    `backswing_ms` and `downswing_ms` share a floor and always share an `n`, so an undeduplicated
    list prints one sentence twice and reads as two problems. The strictest is first because it is
    the one gating the headline ratio, and the sentences differ only in a number.
    """
    tempo = build_career_tempo(_tempo_corpus(
        _tempo_swing(7, ratio=2.42, backswing=968.0, downswing=400.6),
    ))

    reasons = [claim.reason for claim in tempo.withheld]
    assert len(reasons) == len(set(reasons))
    assert [claim.need_n for claim in tempo.withheld] == sorted(
        (claim.need_n for claim in tempo.withheld), reverse=True
    )
    assert tempo.withheld[0].need_n == minimum_n("tempo_ratio", BaselineClaim.CENTER)


def test_both_scopes_go_through_one_builder() -> None:
    """The reason `build_tempo_plan_for` was split out rather than copied.

    A career target fitted to a downswing and a per-swing target fitted to the same downswing are
    the same plan. Two builders would be free to disagree about which half a golfer's target
    follows — silently, since both would look right.
    """
    from_phases = build_tempo_plan(_phases(backswing_ms=901.2, downswing_ms=383.9))
    from_durations = build_tempo_plan_for(
        observed_backswing_ms=901.2, observed_downswing_ms=383.9
    )

    # Field by field rather than `==`: `_phases` reconstructs the durations through the transition
    # midpoint, so the two arrive a float ulp apart and an equality would be testing arithmetic
    # noise instead of the guard.
    assert from_phases.anchored is from_durations.anchored
    assert from_phases.anchor_downswing_ms == pytest.approx(from_durations.anchor_downswing_ms)
    assert from_phases.anchor_backswing_ms == pytest.approx(from_durations.anchor_backswing_ms)
    assert from_phases.pace == pytest.approx(from_durations.pace)
    assert [p.model_dump() for p in from_phases.patterns] == [
        p.model_dump() for p in from_durations.patterns
    ]
