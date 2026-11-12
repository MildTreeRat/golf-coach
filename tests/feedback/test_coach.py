"""Coaching: the brief carries every caveat, and a failed call costs the swing nothing.

The assertions that matter here are not "does it call the API" — they are "does a provisional
number arrive labelled as one". A caveat silently dropped from the brief is exactly how an LLM
starts stating this repo's uncertain numbers as fact, and it would never show up as a crash.

Runs on the base install: `build_brief` is pure, and `generate_coaching` takes an injected client.
"""

from __future__ import annotations

import hashlib
import json
import subprocess
import sys
from datetime import UTC, datetime
from pathlib import Path
from types import SimpleNamespace
from typing import Any

import pytest

from golf_coach import llm_log
from golf_coach.contracts.alignment import (
    AlignmentQuality,
    ClipAlignment,
    SwingAlignment,
    SwingAnchors,
)
from golf_coach.contracts.feedback import CoachingProvenance, FeedbackPayload, Severity, Tip
from golf_coach.contracts.shot import ShotData, ShotProvenance, ShotSource
from golf_coach.contracts.swing import (
    CheckpointScore,
    Measurement,
    SwingBundleResult,
    SwingResult,
)
from golf_coach.contracts.unscored import UnscoredCheckpoint, UnscoredReason
from golf_coach.feedback.coach import (
    BUILD_GOLF_MCP,
    CoachingOutcome,
    GolfMcpServer,
    _spelled,
    brief_from_view,
    brief_through_golf_mcp,
    build_brief,
    coach_through_golf_mcp,
    generate_coaching,
    is_analyzed,
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


# ------------------------------------------------------- the pivot (rotation) readings [M17 P6]


def _pivots() -> list[Measurement]:
    """One face-on and one down-the-line pivot row — both interim, for different reasons."""
    return [
        Measurement(
            name="pivot_hip_axis_drift_norm",
            value=0.5821,
            unit="shoulder_widths",
            source="pose:face_on",
            detail="peak lateral excursion of the hip centre from its address x",
        ),
        Measurement(
            name="pivot_shoulder_reversal_backswing_deg_dtl",
            value=2.9,
            unit="degrees",
            source="pose:down_the_line",
            detail="largest against-the-turn move of the shoulder line within the backswing",
        ),
    ]


def test_the_pivot_rows_reach_the_brief_labelled_interim() -> None:
    """One instrument short of a placement — no reference population at all — and the flag has to
    ride on the line carrying the value, `_placement_lines`'s reason for stamping `calibrated`."""
    brief = build_brief(_bundle(measurements=_pivots()))

    hip_line = next(line for line in brief.splitlines() if "pivot_hip_axis_drift_norm" in line)
    assert "interim" in hip_line
    assert "face-on view" in hip_line

    dtl_line = next(
        line for line in brief.splitlines() if "pivot_shoulder_reversal_backswing_deg_dtl" in line
    )
    assert "interim" in dtl_line
    assert "down-the-line view" in dtl_line


def test_a_pivot_row_carries_its_interim_reason_not_just_the_flag() -> None:
    """`interim` alone says nothing about what would clear it; the reason is the next line."""
    brief = build_brief(_bundle(measurements=_pivots()))

    assert "Fiducial calibration clears it." in brief


def test_a_swing_with_no_pivot_rows_says_nothing_rather_than_going_silent() -> None:
    """Same discipline as the placement section: an absent block reads as one nobody looked at."""
    brief = build_brief(_bundle(measurements=[]))

    assert "ROTATION" in brief
    assert "Say nothing about the golfer's turn." in brief


def test_plain_measurements_are_not_rendered_as_pivots() -> None:
    """`measurements` also holds pose metrics the checkpoints above already judged."""
    pose = Measurement(
        name="head_sway_norm",
        value=0.1234,
        unit="shoulder_widths",
        source="pose:face_on",
        detail="address window -> impact window",
    )
    brief = build_brief(_bundle(measurements=[pose]))

    assert "head_sway_norm" not in brief
    assert "Say nothing about the golfer's turn." in brief


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
    """Needs the SDK present, because the extra is now checked first and would answer instead."""
    pytest.importorskip("anthropic", reason="the missing-key note is only reached with the SDK")

    outcome = generate_coaching(_bundle(), model=MODEL, api_key=None)

    assert outcome.text is None
    assert outcome.note is not None
    assert "no Anthropic API key" in outcome.note


def test_a_missing_extra_is_not_reported_as_a_missing_key(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    """With both absent, only one can be named, and naming the key sends you to the wrong file.

    The order used to be the other way round, so an install without `.[llm]` reported "no
    Anthropic API key is configured" — sending a reader to `.env`, where the key is sitting
    perfectly valid, for a problem that pip fixes.
    """
    monkeypatch.setattr("golf_coach.feedback.coach._sdk", lambda: None)

    outcome = generate_coaching(_bundle(), model=MODEL, api_key=None)

    assert outcome.note is not None
    assert "llm" in outcome.note
    assert "API key" not in outcome.note


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


def test_an_overloaded_api_reads_as_wait_rather_than_as_a_bad_request() -> None:
    """529 says nothing about the request, so the note has to say "try again" and not "400"."""
    anthropic = pytest.importorskip("anthropic")
    exc = anthropic.OverloadedError.__new__(anthropic.OverloadedError)
    Exception.__init__(exc, "boom")

    outcome = generate_coaching(_bundle(), model=MODEL, client=_FakeClient(exc))

    assert outcome.note is not None
    assert "overloaded" in outcome.note
    assert "try again" in outcome.note


def test_a_status_error_keeps_the_apis_own_explanation() -> None:
    """A bare status is a dead end on the one code that most needs explaining.

    An exhausted credit balance and a malformed request are both 400, so "the API returned 400."
    sends you to read request-building code when the answer was billing. The body says which.
    """
    anthropic = pytest.importorskip("anthropic")
    exc = anthropic.BadRequestError.__new__(anthropic.BadRequestError)
    Exception.__init__(exc, "boom")
    exc.status_code = 400
    exc.body = {
        "type": "error",
        "error": {
            "type": "invalid_request_error",
            "message": "Your credit balance is too low to access the Anthropic API.",
        },
    }

    outcome = generate_coaching(_bundle(), model=MODEL, client=_FakeClient(exc))

    assert outcome.note is not None
    assert "400" in outcome.note
    assert "credit balance is too low" in outcome.note


def test_a_status_error_with_no_readable_body_still_names_the_status() -> None:
    """The detail is a bonus, not a dependency — an unfamiliar body must not lose the code."""
    anthropic = pytest.importorskip("anthropic")
    exc = anthropic.ConflictError.__new__(anthropic.ConflictError)
    Exception.__init__(exc, "")
    exc.status_code = 409
    exc.body = None

    outcome = generate_coaching(_bundle(), model=MODEL, client=_FakeClient(exc))

    assert outcome.note is not None
    assert "409" in outcome.note


def test_outcome_defaults_are_the_absent_case() -> None:
    assert CoachingOutcome().text is None
    assert CoachingOutcome().provenance is None


# ------------------------------------------------------- the stdio route [M29 P17]
#
# `brief_from_view` over `golf-mcp`'s `get_swing`, and `python -m golf_coach.feedback.coach`. The
# tests that need the server take `conftest.py`'s `golf_mcp`, which skips by name when none is
# built; the renderer's own rules are pinned on hand-written views, with no server.


def _full_bundle() -> SwingBundleResult:
    """A result reaching every line of the brief, stored as the pipeline stores one.

    Both families of `unscored`, both sidednesses, placements and pivots, a verdict, a flagged shot
    carrying the screen-layout warning `golf-mcp` filters, and a partial alignment whose note the
    swing's notes also carry as `alignment: …`, as both engines copy it.
    """
    alignment = _partial_alignment()
    bundle = _bundle(
        checkpoints=[_checkpoint(), _checkpoint("head_sway", passed=False, one_sided=True)],
        measurements=[*_placements(), *_pivots()],
        shot=_flagged_shot().model_copy(
            update={
                "provenance": ShotProvenance(
                    device="hd_golf",
                    parse_confidence=0.42,
                    needs_review=True,
                    warnings=[
                        "no tile found for 'Bounce & Roll'",
                        "smash factor 1.63 is above the physical maximum",
                    ],
                )
            }
        ),
        alignment=alignment,
        notes=[*(f"alignment: {note}" for note in alignment.notes), "pose ran on the heavy model"],
        feedback=FeedbackPayload(
            swing_id="2",
            overall_score=94.9,
            headline="Work on tempo.",
            tips=[Tip(checkpoint="tempo", text="Slow the backswing.", severity=Severity.MAJOR)],
        ),
    )
    bundle.swing.unscored = [
        UnscoredCheckpoint(name="hip_sway", reason=UnscoredReason.NO_BAND),
        UnscoredCheckpoint(name="flight_carry_yds", reason=UnscoredReason.CARRY_UNREACHABLE),
    ]
    return bundle


def _as_the_view_renders_it(brief: str, bundle: SwingBundleResult) -> str:
    """`build_brief`'s text with `brief_from_view`'s documented differences applied, and no other.

    The four in its docstring that a stored result can reach: the device, the layout warning, the
    alignment summary, and the alignment notes the swing's own notes already carry.
    """
    alignment = bundle.alignment
    assert alignment is not None
    renamed = brief.replace(
        "photo of the hd_golf screen", "photo of the launch monitor's screen"
    ).replace(f" ({alignment.quality_summary})", "")
    dropped = {"  warning: no tile found for 'Bounce & Roll'"}
    dropped |= {f"  note: {note}" for note in alignment.notes}
    return "\n".join(line for line in renamed.splitlines() if line not in dropped)


def _golf_mcp_over(binary: Path, root: Path) -> GolfMcpServer:
    return GolfMcpServer(binary=binary, sessions_dir=root / "sessions", shots_dir=root / "shots")


def test_the_view_brief_is_build_brief_line_for_line_but_the_four_named(
    golf_mcp: Path, tmp_path: Path, store_swing: Any
) -> None:
    """The whole rendering, through the real server: every other line is `build_brief`'s exactly.

    This is what makes `brief_from_view` a port rather than a second brief. It also holds the two
    keys `golf-mcp` added for it: the sidedness on both percentiles, and the flight refusal under
    its own heading rather than among the checkpoints excluded from `overall_score`.
    """
    bundle = _full_bundle()
    store_swing(tmp_path / "sessions", bundle)
    (tmp_path / "shots").mkdir()

    brief = brief_through_golf_mcp(_golf_mcp_over(golf_mcp, tmp_path), "2026-08-10", "2")

    assert brief.refusal is None and brief.text is not None
    assert brief.text == _as_the_view_renders_it(build_brief(bundle), bundle)
    assert "(lower is better)" in brief.text and "(both extremes are faults)" in brief.text
    flight_heading = brief.text.index("simulated ball flight")
    assert brief.text.index("- hip_sway: ") < flight_heading < brief.text.index("- flight_carry")


def _view(**fields: Any) -> dict[str, Any]:
    """A `get_swing` answer as `golf-mcp` gives one, with only what a test sets filled."""
    return {"swing_id": "2", "session_id": "2026-08-10", "status": "done", **fields}


@pytest.mark.parametrize(
    ("one_sided", "said"),
    [(True, " (lower is better)"), (False, " (both extremes are faults)"), (None, "")],
)
def test_a_percentile_says_its_direction_only_when_the_score_does(
    one_sided: bool | None, said: str
) -> None:
    """`null` is a score that does not say, and a guessed direction is wrong half the time."""
    checkpoint = {"name": "tempo", "score": 1.0, "passed": True, "percentile": 62.0}
    brief = brief_from_view(
        _view(overall_score=90.0, checkpoints=[{**checkpoint, "one_sided": one_sided}])
    )

    assert "  percentile 62 tour swings" + said + "\n" in brief


def test_a_flight_refusal_is_never_called_a_checkpoint_excluded_from_the_score() -> None:
    entry = {"name": "flight_carry_yds", "why": "no carry fits.", "family": "flight"}

    brief = brief_from_view(_view(overall_score=90.0, unscored=[entry]))

    assert "unscored: none - every checkpoint was scored on this swing" in brief
    assert "never part of overall_score):\n- flight_carry_yds: no carry fits." in brief


@pytest.mark.parametrize(
    ("view", "key"),
    [
        (_view(checkpoints=[{"name": "tempo", "score": 1.0}]), "one_sided"),
        (_view(unscored=[{"name": "tempo", "reason": "no_band", "why": "x"}]), "family"),
    ],
)
def test_a_view_from_a_server_older_than_its_keys_is_refused(view: dict, key: str) -> None:
    """A stale build would silently drop a direction or mislabel a refusal, so it is named."""
    with pytest.raises(ValueError, match=f"no `{key}`.*{BUILD_GOLF_MCP}"):
        brief_from_view(view)


def test_a_view_with_only_its_identity_is_not_analyzed() -> None:
    """`SwingView::bare`: what `golf-mcp` answers for a swing with no `analysis.json`."""
    bare = _view(
        club=None,
        mishit=None,
        checkpoints=[],
        tips=[],
        unscored=[],
        measurements={},
        simulated=[],
        population=[],
        rotation=[],
        notes=[],
    )

    assert not is_analyzed(bare)
    assert is_analyzed({**bare, "overall_score": 0.0})
    assert is_analyzed({**bare, "unscored": [{"name": "tempo"}]})


def test_the_entry_writes_the_paragraph_from_golf_mcps_view_and_caveats(
    golf_mcp: Path, tmp_path: Path, store_swing: Any
) -> None:
    """The request is `generate_coaching`'s, with the brief from the view and the prompt from Rust.

    Rust's briefing is the only one that says a misread photo is worth retaking (the M29 plan's
    C13), so finding it is finding that the caveats came over the pipe and not from a Python copy.
    The provenance is `CoachingProvenance` as `analysis.json` stores it, round trip and all.
    """
    store_swing(tmp_path / "sessions", _full_bundle())
    (tmp_path / "shots").mkdir()
    client = _FakeClient(_Response([_Block("Your tempo is quick.")]))
    server = _golf_mcp_over(golf_mcp, tmp_path)

    answer = coach_through_golf_mcp("2026-08-10", "2", server=server, model=MODEL, client=client)

    brief = brief_through_golf_mcp(server, "2026-08-10", "2").text
    assert brief is not None
    assert answer["text"] == "Your tempo is quick."
    provenance = answer["provenance"]
    assert provenance["model"] == MODEL
    assert provenance["input_digest"] == hashlib.sha256(brief.encode("utf-8")).hexdigest()
    assert CoachingProvenance.model_validate(provenance).model_dump(mode="json") == provenance
    kwargs = client.messages.kwargs
    assert kwargs["messages"] == [{"role": "user", "content": brief}]
    assert kwargs["thinking"] == {"type": "adaptive"}
    assert kwargs["output_config"] == {"effort": "low"}
    system = kwargs["system"][0]["text"]
    assert "`misread` is a photo of the screen worth retaking" in " ".join(system.split())
    assert system.startswith("You are a golf coach reading one swing")
    assert system.endswith('Address the golfer as "you". Be direct and encouraging; do not pad.')


def test_the_entry_turns_a_miss_and_a_missing_server_into_notes(
    golf_mcp: Path, tmp_path: Path
) -> None:
    (tmp_path / "sessions").mkdir()
    (tmp_path / "shots").mkdir()
    client = _FakeClient(_Response([_Block("never asked")]))

    miss = coach_through_golf_mcp(
        "2026-08-10", "9", server=_golf_mcp_over(golf_mcp, tmp_path), model=MODEL, client=client
    )
    unbuilt = coach_through_golf_mcp(
        "2026-08-10",
        "9",
        server=_golf_mcp_over(tmp_path / "golf-mcp", tmp_path),
        model=MODEL,
        client=client,
    )

    assert miss == {
        "note": (
            "no written coaching: No swing '9' in session '2026-08-10'. "
            "Call list_sessions to see which sessions and swing ids exist."
        )
    }
    assert set(unbuilt) == {"note"} and BUILD_GOLF_MCP in unbuilt["note"]
    assert client.messages.kwargs == {}, "the model was asked with nothing to coach from"


def test_a_missing_key_is_refused_before_golf_mcp_is_looked_for(tmp_path: Path) -> None:
    pytest.importorskip("anthropic", reason="the missing-key note is only reached with the SDK")

    answer = coach_through_golf_mcp(
        "2026-08-10", "2", server=_golf_mcp_over(tmp_path / "golf-mcp", tmp_path), model=MODEL
    )

    assert "no Anthropic API key" in answer["note"]
    assert BUILD_GOLF_MCP not in answer["note"]


@pytest.mark.parametrize(
    "instant",
    [
        datetime(2026, 10, 22, 8, 23, tzinfo=UTC),
        datetime(2026, 10, 22, 8, 23, 0, 120, tzinfo=UTC),
        datetime(2026, 10, 22, 8, 23, 5, 300000, tzinfo=UTC),
    ],
)
def test_the_time_is_spelled_as_pydantic_spells_it(instant: datetime) -> None:
    """The entry writes no pydantic shape, and `analysis.json` holds pydantic's spelling."""
    stored = CoachingProvenance(model=MODEL, generated_at=instant).model_dump(mode="json")

    assert _spelled(instant) == stored["generated_at"]


def test_the_module_runs_and_prints_one_json_object(tmp_path: Path) -> None:
    """`python -m golf_coach.feedback.coach`, as `golf-lab analyze` spawns it: exit 2 with a note.

    With no server at the path given it cannot write, whatever key `.env` holds, so this makes no
    API call. Stdout is the one JSON object and nothing else, for the caller to parse.
    """
    run = subprocess.run(
        [
            sys.executable,
            "-m",
            "golf_coach.feedback.coach",
            "--session",
            "2026-08-10",
            "--swing",
            "2",
            "--golf-mcp",
            str(tmp_path / "golf-mcp"),
            "--sessions-dir",
            str(tmp_path),
        ],
        capture_output=True,
        text=True,
        encoding="utf-8",
        check=False,
    )

    assert run.returncode == 2, run.stderr
    answer = json.loads(run.stdout)
    assert set(answer) == {"note"} and answer["note"].startswith("no written coaching: ")
    assert run.stdout.isascii()


# --------------------------------------------------------------------------- the call log [M43 P4]
#
# One line per request, on both routes and every path that made one, and none on a path that did
# not (ADR-036 §4). `_write` is the one place both routes reach, so it is the one place that logs.
# The log is `tests/conftest.py::_throwaway_llm_log`'s file, so nothing here reaches the real
# `llm_calls.jsonl`.


def _logged() -> list[dict[str, Any]]:
    path = llm_log._log_path()
    if not path.exists():
        return []
    return [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines()]


def test_a_paragraph_logs_one_line() -> None:
    generate_coaching(_bundle(), model=MODEL, client=_FakeClient(_Response([_Block("ok")])))

    [line] = _logged()
    assert line["route"] == "coach"
    assert line["requested_model"] == MODEL
    assert line["served_model"] == MODEL
    assert line["stop_reason"] == "end_turn"
    assert line["outcome"] == "ok"
    assert line["error_class"] is None
    assert line["origin"] == llm_log.DEFAULT_ORIGIN
    assert line["turn_id"] is None and line["iteration"] is None
    assert line["tool_calls"] == 0


def test_a_session_of_swings_logs_a_line_per_swing() -> None:
    client = _FakeClient(_Response([_Block("ok")]))
    generate_coaching(_bundle(), model=MODEL, client=client)
    generate_coaching(_bundle(unscored=["tempo"]), model=MODEL, client=client)

    assert [line["outcome"] for line in _logged()] == ["ok", "ok"]


@pytest.mark.parametrize(
    ("response", "kind", "served"),
    [
        (_Response([_Block("...")], stop_reason="refusal"), "refusal", False),
        (_Response([]), "no_text", False),
        # Thinking can spend the whole budget, which is a truncation with nothing to serve.
        (_Response([_Block("hm", type_="thinking")], stop_reason="max_tokens"), "no_text", False),
        # Unlike the lookup's `truncated`, this one is served, with its admission.
        (_Response([_Block("Your tempo is")], stop_reason="max_tokens"), "truncated", True),
    ],
)
def test_each_answer_logs_one_line_with_its_kind(
    response: _Response, kind: str, served: bool
) -> None:
    outcome = generate_coaching(_bundle(), model=MODEL, client=_FakeClient(response))

    assert (outcome.text is not None) is served
    [line] = _logged()
    assert line["outcome"] == kind
    assert line["stop_reason"] == response.stop_reason
    assert line["error_class"] is None


def test_a_failed_request_logs_one_line_with_its_error_and_no_usage() -> None:
    outcome = generate_coaching(
        _bundle(), model=MODEL, client=_FakeClient(RuntimeError("connection reset"))
    )

    assert outcome.note is not None
    [line] = _logged()
    assert line["outcome"] == "request_failed"
    assert line["error_class"] == "RuntimeError"
    assert line["served_model"] is None
    assert line["usage"] is None
    assert line["cost_usd"] is None


@pytest.mark.parametrize(
    ("error_name", "kind"),
    [
        ("NotFoundError", "model_not_found"),
        ("AuthenticationError", "auth_failed"),
        ("RateLimitError", "rate_limited"),
        ("OverloadedError", "overloaded"),
        ("InternalServerError", "api_error"),
        ("BadRequestError", "api_error"),
        ("APIStatusError", "api_error"),
        ("APIConnectionError", "connection_failed"),
        ("APITimeoutError", "connection_failed"),
    ],
)
def test_sdk_errors_log_the_kind_of_note_they_became(error_name: str, kind: str) -> None:
    """A 429 and a 529 are subclasses of `APIStatusError`, and must not log as one (M43 P8)."""
    anthropic = pytest.importorskip("anthropic", reason="needs the `llm` extra to be meaningful")

    exc = getattr(anthropic, error_name).__new__(getattr(anthropic, error_name))
    Exception.__init__(exc, "boom")
    generate_coaching(_bundle(), model=MODEL, client=_FakeClient(exc))

    [line] = _logged()
    assert line["outcome"] == kind
    assert line["error_class"] == error_name


def test_no_key_logs_nothing_because_nothing_was_sent() -> None:
    pytest.importorskip("anthropic", reason="the missing-key note is only reached with the SDK")

    generate_coaching(_bundle(), model=MODEL, api_key=None)

    assert _logged() == []


def test_a_missing_extra_logs_nothing(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr("golf_coach.feedback.coach._sdk", lambda: None)

    generate_coaching(_bundle(), model=MODEL, api_key=None)

    assert _logged() == []


@pytest.mark.parametrize("fails", [False, True])
def test_latency_is_the_request_and_nothing_around_it(
    fails: bool, monkeypatch: pytest.MonkeyPatch
) -> None:
    """The clock moves only inside `create`, so a clock started after it would log a zero."""
    clock = SimpleNamespace(now=100.0)
    monkeypatch.setattr("golf_coach.feedback.coach.perf_counter", lambda: clock.now)
    client = _FakeClient(RuntimeError("boom") if fails else _Response([_Block("ok")]))
    create = client.messages.create

    def slow_create(**kwargs: Any) -> object:
        clock.now += 2.5
        return create(**kwargs)

    client.messages.create = slow_create  # type: ignore[method-assign]
    generate_coaching(_bundle(), model=MODEL, client=client)

    [line] = _logged()
    assert line["latency_s"] == pytest.approx(2.5)


def test_the_response_is_handed_to_the_log_whole() -> None:
    """Usage, the served model and the request id come off the response, so the line is priced.

    The coaching prompt is cached, so the line carries a cache read and a cache write. Claude Opus
    5 at $5 in, $25 out, $0.50 a cache read and $6.25 a 5-minute cache write per million tokens,
    the pricing page as read on 2026-11-09: 200 in, 400 out, 2,000 read and 1,000 written is
    $0.001 + $0.01 + $0.001 + $0.00625.
    """
    response = _Response([_Block("Your tempo is quick.")])
    response.usage = SimpleNamespace(  # type: ignore[attr-defined]
        input_tokens=200,
        output_tokens=400,
        cache_read_input_tokens=2000,
        cache_creation_input_tokens=1000,
        cache_creation=None,
    )
    response._request_id = "req_coach"  # type: ignore[attr-defined]

    generate_coaching(_bundle(), model=MODEL, client=_FakeClient(response))

    [line] = _logged()
    assert line["usage"] == {
        "input_tokens": 200,
        "output_tokens": 400,
        "cache_read_input_tokens": 2000,
        "cache_creation_input_tokens": 1000,
    }
    assert line["cost_usd"] == pytest.approx(0.01825)
    assert line["request_id"] == "req_coach"


def test_a_log_that_cannot_be_written_changes_nothing(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    """§4: the golfer gets the same paragraph whether or not the line could be written."""
    blocker = tmp_path / "not-a-directory"
    blocker.write_text("", encoding="utf-8")
    monkeypatch.setattr(llm_log, "_log_path", lambda: blocker / "llm_calls.jsonl")

    outcome = generate_coaching(
        _bundle(), model=MODEL, client=_FakeClient(_Response([_Block("Your tempo is quick.")]))
    )

    assert outcome.text == "Your tempo is quick."
    assert outcome.note is None
    assert capsys.readouterr().err.startswith("llm_log: ")


def test_the_stdio_route_logs_its_one_request(
    golf_mcp: Path, tmp_path: Path, store_swing: Any
) -> None:
    """The second route reaches `_write` too, so its request is logged in the same one place."""
    store_swing(tmp_path / "sessions", _full_bundle())
    (tmp_path / "shots").mkdir()
    client = _FakeClient(_Response([_Block("Your tempo is quick.")]))

    answer = coach_through_golf_mcp(
        "2026-08-10", "2", server=_golf_mcp_over(golf_mcp, tmp_path), model=MODEL, client=client
    )

    assert answer["text"] == "Your tempo is quick."
    [line] = _logged()
    assert line["route"] == "coach"
    assert line["outcome"] == "ok"


def test_a_swing_with_nothing_to_coach_from_logs_nothing(golf_mcp: Path, tmp_path: Path) -> None:
    (tmp_path / "sessions").mkdir()
    (tmp_path / "shots").mkdir()
    client = _FakeClient(_Response([_Block("never asked")]))

    coach_through_golf_mcp(
        "2026-08-10", "9", server=_golf_mcp_over(golf_mcp, tmp_path), model=MODEL, client=client
    )

    assert _logged() == []


def test_an_unbuilt_server_and_a_missing_key_log_nothing(tmp_path: Path) -> None:
    unbuilt = _golf_mcp_over(tmp_path / "golf-mcp", tmp_path)

    coach_through_golf_mcp(
        "2026-08-10", "2", server=unbuilt, model=MODEL, client=_FakeClient(_Response([]))
    )
    coach_through_golf_mcp("2026-08-10", "2", server=unbuilt, model=MODEL, api_key=None)

    assert _logged() == []
