"""Coaching: the brief carries every caveat, and a failed call costs the swing nothing.

The assertions that matter here are not "does it call the API" — they are "does a provisional
number arrive labelled as one". A caveat silently dropped from the brief is exactly how an LLM
starts stating this repo's uncertain numbers as fact, and it would never show up as a crash.

Runs on the base install: `build_brief` is pure, and `generate_coaching` takes an injected client.
"""

from __future__ import annotations

from datetime import UTC, datetime

import pytest

from golf_coach.contracts.alignment import (
    AlignmentQuality,
    ClipAlignment,
    SwingAlignment,
    SwingAnchors,
)
from golf_coach.contracts.feedback import FeedbackPayload, Severity, Tip
from golf_coach.contracts.shot import ShotData, ShotProvenance, ShotSource
from golf_coach.contracts.swing import (
    CheckpointScore,
    Measurement,
    SwingBundleResult,
    SwingResult,
)
from golf_coach.feedback.coach import (
    CoachingOutcome,
    build_brief,
    generate_coaching,
)

MODEL = "claude-opus-5"


# --------------------------------------------------------------------------- fixtures


def _checkpoint(
    name: str = "tempo",
    *,
    passed: bool = True,
    percentile: float | None = 62.0,
    one_sided: bool = False,
) -> CheckpointScore:
    return CheckpointScore(
        name=name,
        score=1.0 if passed else 0.4,
        passed=passed,
        observed=2.42,
        expected_low=2.72,
        expected_high=4.71,
        message=f"{name} reading.",
        percentile=percentile,
        population_n=1399 if percentile is not None else None,
        one_sided=one_sided,
    )


def _placements() -> list[Measurement]:
    """One calibrated and one uncalibrated placement, with the real values from `2026-08-10/2`.

    The `q_dtl` figure is the interesting one and is why these are not round numbers: 11.06 is the
    swing whose down-the-line clip segments to a one-frame backswing, so the largest placement on
    the swing is a mis-detected anchor rather than anything the golfer did.
    """
    return [
        Measurement(
            name="tour_trajectory_t2",
            value=2.9099,
            unit="sd_units",
            source="population:golfdb",
            detail="distance from the tour swing shape inside the fitted subspace; more unusual "
            "than 82.3% of 415 tour swings",
        ),
        Measurement(
            name="tour_trajectory_q_dtl",
            value=11.055,
            unit="shoulder_widths",
            source="population:golfdb",
            detail="down-the-line: residual off that basis; most of it falls in address->top. "
            "NOT calibrated, same caveat as the face-on Q",
        ),
    ]


def _bundle(
    *,
    checkpoints: list[CheckpointScore] | None = None,
    unscored: list[str] | None = None,
    shot: ShotData | None = None,
    alignment: SwingAlignment | None = None,
    notes: list[str] | None = None,
    feedback: FeedbackPayload | None = None,
    measurements: list[Measurement] | None = None,
) -> SwingBundleResult:
    swing = SwingResult(
        swing_id="2",
        session_id="2026-08-10",
        checkpoint_scores=checkpoints if checkpoints is not None else [_checkpoint()],
        unscored=unscored or [],
        measurements=measurements if measurements is not None else _placements(),
        overall_score=94.9,
        mechanics_score=94.9,
        shot=shot,
    )
    return SwingBundleResult(
        swing_id="2",
        session_id="2026-08-10",
        swing=swing,
        alignment=alignment,
        notes=notes or [],
        feedback=feedback,
    )


def _flagged_shot() -> ShotData:
    return ShotData(
        shot_id="2026-08-10-2",
        session_id="2026-08-10",
        timestamp=datetime(2026, 8, 10, 15, 0, tzinfo=UTC),
        source=ShotSource.SCREEN,
        carry_distance=168.0,
        ball_speed=131.0,
        provenance=ShotProvenance(
            device="hd_golf",
            parse_confidence=0.42,
            needs_review=True,
            warnings=["smash factor 1.63 is above the physical maximum"],
        ),
    )


def _partial_alignment() -> SwingAlignment:
    anchors = SwingAnchors(motion_start=10, top=60, impact=84)
    clip = ClipAlignment(anchors=anchors, warp_motion_start=10, tau_start=-0.2, tau_end=2.6)
    return SwingAlignment(
        a=clip,
        b=clip,
        quality=AlignmentQuality.TOP_IMPACT,
        notes=["down_the_line motion start was estimated, not detected"],
    )


class _FakeMessages:
    """Records the request and returns whatever the test wants back."""

    def __init__(self, response: object | Exception) -> None:
        self.response = response
        self.kwargs: dict = {}

    def create(self, **kwargs):
        self.kwargs = kwargs
        if isinstance(self.response, Exception):
            raise self.response
        return self.response


class _FakeClient:
    def __init__(self, response: object | Exception) -> None:
        self.messages = _FakeMessages(response)


class _Block:
    def __init__(self, text: str, type_: str = "text") -> None:
        self.text = text
        self.type = type_


class _Response:
    def __init__(self, blocks: list[_Block], *, stop_reason: str = "end_turn") -> None:
        self.content = blocks
        self.stop_reason = stop_reason
        self.model = MODEL


# --------------------------------------------------------------------------- the brief


@pytest.mark.parametrize(
    ("percentile", "expected"),
    [(90.0, "percentile 90"), (10.0, "percentile 10"), (56.3, "percentile 56")],
)
def test_round_percentiles_are_not_mangled(percentile: float, expected: str) -> None:
    """A percentile of 90 must not render as 9.

    Regression: the first cut trimmed trailing zeros unconditionally, so every round percentile
    lost its last digit — 90 became 9, 10 became 1. Plausible, wrong, and handed straight to a
    model that would have stated it as fact.
    """
    brief = build_brief(_bundle(checkpoints=[_checkpoint(percentile=percentile)]))
    assert expected in brief


def test_brief_carries_the_checkpoint_band_and_percentile() -> None:
    brief = build_brief(_bundle())

    assert "tempo" in brief
    assert "2.72 to 4.71" in brief
    assert "percentile 62 of 1399" in brief


def test_percentile_says_which_direction_is_a_fault() -> None:
    """`one_sided` changes what a percentile *means*, so the brief has to spell it out."""
    two_sided = build_brief(_bundle(checkpoints=[_checkpoint("tempo", one_sided=False)]))
    one_sided = build_brief(
        _bundle(checkpoints=[_checkpoint("head_sway", one_sided=True)])
    )

    assert "both extremes are faults" in two_sided
    assert "lower is better" in one_sided


def test_unscored_is_named_and_explained_not_dropped() -> None:
    brief = build_brief(_bundle(unscored=["tempo"]))

    assert "unscored" in brief
    assert "tempo" in brief
    # The distinction the whole field exists for: dropped from the mean, not scored zero.
    assert "rather than counted as zero" in brief


def test_unscored_says_none_rather_than_going_silent() -> None:
    """An absent section reads as 'nothing to say'; this has to read as 'all three measured'."""
    assert "unscored: none" in build_brief(_bundle())


def test_a_flagged_shot_arrives_flagged() -> None:
    brief = build_brief(_bundle(shot=_flagged_shot()))

    assert "needs_review: yes" in brief
    assert "0.42" in brief
    assert "smash factor 1.63 is above the physical maximum" in brief
    # And the numbers themselves are still there — flagged, not withheld.
    assert "carry: 168" in brief


def test_an_unflagged_shot_says_so() -> None:
    shot = _flagged_shot()
    shot.provenance.needs_review = False
    assert "needs_review: no" in build_brief(_bundle(shot=shot))


def test_partial_alignment_gets_the_caveat_sentence() -> None:
    brief = build_brief(_bundle(alignment=_partial_alignment()))

    assert "alignment_caveat:" in brief
    assert "aligned on top and impact" in brief
    assert "do not read fine timing differences off it" in brief
    assert "down_the_line motion start was estimated" in brief


def test_full_alignment_gets_no_caveat() -> None:
    anchors = SwingAnchors(motion_start=10, top=60, impact=84)
    clip = ClipAlignment(anchors=anchors, warp_motion_start=10, tau_start=0.0, tau_end=2.5)
    full = SwingAlignment(a=clip, b=clip, quality=AlignmentQuality.FULL)

    assert "alignment_caveat:" not in build_brief(_bundle(alignment=full))


def test_pipeline_notes_survive_verbatim() -> None:
    note = "could not pick a swing in the down-the-line view — the whole clip was scored"
    assert note in build_brief(_bundle(notes=[note]))


def test_the_rule_based_verdict_is_included_so_the_model_elaborates_on_it() -> None:
    feedback = FeedbackPayload(
        swing_id="2",
        overall_score=94.9,
        headline="Work on tempo first.",
        tips=[Tip(checkpoint="tempo", text="Tempo is quick.", severity=Severity.MINOR)],
    )
    brief = build_brief(_bundle(feedback=feedback))

    assert "Work on tempo first." in brief
    assert "Tempo is quick." in brief


def test_brief_excludes_the_heavy_streams() -> None:
    """Keypoints would dominate the prompt and no coach reasons from raw landmarks."""
    brief = build_brief(_bundle())
    assert "keypoints" not in brief.lower()
    assert len(brief) < 4000


# ------------------------------------------------------- the population placements [M8.3]


def test_the_placements_reach_the_brief_at_all() -> None:
    """They did not, for the whole of M8: five were recorded and `build_brief` rendered none.

    The models were fitted, validated and surfaced onto `measurements`, and the coaching call —
    the only channel that speaks to a golfer — never saw them.
    """
    brief = build_brief(_bundle())

    assert "tour_trajectory_t2" in brief
    assert "2.91" in brief


def test_a_placement_carries_its_detail_because_that_is_where_its_meaning_is() -> None:
    """The value alone is unreadable: the percentile and the population size live in `detail`."""
    brief = build_brief(_bundle())

    assert "82.3% of 415 tour swings" in brief


def test_an_uncalibrated_placement_says_so_on_its_own_line() -> None:
    """The caveat block warns about this too, three hundred words above the number.

    A model applies a general warning to the numbers it remembers, so the flag rides on the line
    that carries the value — the same reason `_checkpoint_line` repeats `one_sided`. Without it
    the largest number in this brief is a broken anchor presented as the swing's headline.
    """
    brief = build_brief(_bundle())

    q_line = next(line for line in brief.splitlines() if "tour_trajectory_q_dtl" in line)
    assert "NOT calibrated" in q_line
    assert "down-the-line view" in q_line

    t2_line = next(line for line in brief.splitlines() if "tour_trajectory_t2 " in line)
    assert "NOT calibrated" not in t2_line
    assert "face-on view" in t2_line


def test_a_swing_with_no_placement_says_nothing_rather_than_going_silent() -> None:
    """Same discipline as `unscored`: an absent section reads as one nobody looked at."""
    brief = build_brief(_bundle(measurements=[]))

    assert "POPULATION PLACEMENT" in brief
    assert "Say nothing about where it sits." in brief


def test_plain_measurements_are_not_rendered_as_placements() -> None:
    """`measurements` also holds pose metrics the checkpoints above already judged.

    Repeating them here would print every number in the brief twice under two names, which is how
    a model comes to describe one reading as two findings.
    """
    pose = Measurement(
        name="head_sway_norm",
        value=0.1234,
        unit="shoulder_widths",
        source="pose:face_on",
        detail="address window -> impact window",
    )
    brief = build_brief(_bundle(measurements=[pose]))

    assert "head_sway_norm" not in brief
    assert "Say nothing about where it sits." in brief


# --------------------------------------------------------------------------- the call


def test_happy_path_sets_text_and_provenance() -> None:
    client = _FakeClient(_Response([_Block("Your tempo is quick.")]))

    outcome = generate_coaching(_bundle(), model=MODEL, client=client)

    assert outcome.text == "Your tempo is quick."
    assert outcome.provenance is not None
    assert outcome.provenance.model == MODEL
    assert outcome.provenance.input_digest
    assert outcome.note is None


def test_request_shape_matches_what_opus_5_accepts() -> None:
    """The four removed parameters are a 400, not a warning — pin their absence."""
    client = _FakeClient(_Response([_Block("ok")]))
    generate_coaching(_bundle(), model=MODEL, client=client)

    kwargs = client.messages.kwargs
    assert kwargs["model"] == MODEL
    assert kwargs["thinking"] == {"type": "adaptive"}
    assert kwargs["output_config"] == {"effort": "low"}
    for removed in ("temperature", "top_p", "top_k", "budget_tokens"):
        assert removed not in kwargs
    # max_tokens bounds thinking *and* text together on this model, so it cannot be tight.
    assert kwargs["max_tokens"] >= 2048


def test_thinking_blocks_are_not_mistaken_for_the_answer() -> None:
    client = _FakeClient(
        _Response([_Block("internal", type_="thinking"), _Block("The real answer.")])
    )
    assert generate_coaching(_bundle(), model=MODEL, client=client).text == "The real answer."


def test_digest_changes_when_the_swing_does() -> None:
    """A stored result should be able to tell that the numbers moved underneath it."""
    client = _FakeClient(_Response([_Block("ok")]))
    first = generate_coaching(_bundle(), model=MODEL, client=client)
    second = generate_coaching(_bundle(unscored=["tempo"]), model=MODEL, client=client)

    assert first.provenance.input_digest != second.provenance.input_digest


def test_missing_api_key_is_a_note_not_an_exception() -> None:
    outcome = generate_coaching(_bundle(), model=MODEL, api_key=None)

    assert outcome.text is None
    assert outcome.note is not None
    assert "no Anthropic API key" in outcome.note


def test_a_refusal_produces_no_text() -> None:
    client = _FakeClient(_Response([_Block("...")], stop_reason="refusal"))
    outcome = generate_coaching(_bundle(), model=MODEL, client=client)

    assert outcome.text is None
    assert "declined" in outcome.note


def test_an_empty_response_produces_no_text() -> None:
    client = _FakeClient(_Response([]))
    outcome = generate_coaching(_bundle(), model=MODEL, client=client)

    assert outcome.text is None
    assert outcome.note is not None


def test_truncation_is_admitted_rather_than_served_silently() -> None:
    client = _FakeClient(_Response([_Block("Your tempo is")], stop_reason="max_tokens"))
    outcome = generate_coaching(_bundle(), model=MODEL, client=client)

    assert outcome.text is not None
    assert "cut off" in outcome.text


def test_an_api_error_degrades_to_a_note() -> None:
    client = _FakeClient(RuntimeError("connection reset"))
    outcome = generate_coaching(_bundle(), model=MODEL, client=client)

    assert outcome.text is None
    assert outcome.provenance is None
    assert "connection reset" in outcome.note


@pytest.mark.parametrize("error_name", ["NotFoundError", "RateLimitError"])
def test_sdk_errors_are_named_specifically_when_the_sdk_is_installed(error_name: str) -> None:
    """A stale `coaching_model` and a rate limit want different words on the results page."""
    anthropic = pytest.importorskip("anthropic")
    exc_type = getattr(anthropic, error_name)
    # These constructors want a response/body; build the cheapest instance that type-checks.
    exc = exc_type.__new__(exc_type)
    Exception.__init__(exc, "boom")

    outcome = generate_coaching(_bundle(), model=MODEL, client=_FakeClient(exc))

    assert outcome.text is None
    expected = "model id" if error_name == "NotFoundError" else "rate limited"
    assert expected in outcome.note


def test_outcome_defaults_are_the_absent_case() -> None:
    assert CoachingOutcome().text is None
    assert CoachingOutcome().provenance is None
