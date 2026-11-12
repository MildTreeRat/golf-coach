"""The coach-feedback flow: the paragraph through the real stdio route, checked by code. [M43 P17]

What these tests hold:

- the flow's cases are the generator's, each carrying the brief the user approved in `cases.md`,
  and the flow imports nothing the coach route does not already;
- each of the four checks passes a paragraph true to its brief and fails the violation it names:
  a number not in the brief, a dropped caveat, a claim about what is not measured, a reversed
  verdict (the plan's Done when), on hand-built views and on the approved cases;
- `run` coaches through `golf-mcp` with the run's model and client, refuses a brief nobody
  approved before any request, and reads swings safely from the runner's threads;
- the runner finds the flow by name and writes its rows, a refusal included;
- every served paragraph is also judged (`coach_judge`, P18), blind to the model under test and
  apart from the checks, and a refusal is not judged.

Hand-built views run on the base install. The rest needs the built server and skips by name
without it (`tests/conftest.py::golf_mcp`).
"""

from __future__ import annotations

import json
import runpy
import shutil
import subprocess
import sys
from collections.abc import Mapping
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
from types import SimpleNamespace
from typing import Any

import pytest

from golf_coach.config import REPO_ROOT
from golf_coach.evals import coach_cases, coach_feedback, coach_judge, runner
from golf_coach.evals.coach_feedback import (
    BRIEFS_FILE,
    CASES_DIR,
    CASES_FILE,
    CHECKS,
    FLOW,
    SESSIONS,
    Coached,
    CoachFeedbackFlow,
    SwingCase,
    approved_briefs,
    check_numbers,
    grade_paragraph,
    signed_checkpoints,
    unmeasured_terms,
)
from golf_coach.evals.runner import AttemptFailed, Case, CaseContext, RecordingClient
from golf_coach.feedback.coach import SYSTEM_PROMPT, GolfMcpServer, brief_from_view, read_swing

MODEL = "claude-opus-5"
JUDGE_IDS = [metric.id for metric in coach_judge.METRICS]
STATE = REPO_ROOT / ".claude" / "hillclimb" / "coach_feedback" / "_state.json"
#: `api/pipeline.py::_band_estimator_note`'s sentence on a real stored swing (P16's finding).
BAND_ESTIMATOR_NOTE = (
    "this swing was measured by mediapipe:heavy, but the benchmark bands were cut from "
    "mediapipe:lite; percentiles near a band edge may move by a few points"
)


@pytest.fixture(scope="module")
def cases() -> list[Case]:
    return FLOW.load_cases()


def _case(cases: list[Case], case_id: str) -> Case:
    return next(case for case in cases if case.id == case_id)


# --------------------------------------------------------------------------- the flow's shape


def test_the_runner_finds_the_flow_by_its_module_name() -> None:
    assert isinstance(coach_feedback.FLOW, runner.Flow)
    assert FLOW.judge_model == coach_judge.JUDGE_MODEL == "claude-opus-5-5"
    # The code checks lead and the judge's columns follow them (ADR-036 §8).
    assert [metric.id for metric in FLOW.metrics] == ["all_checks", *CHECKS, *JUDGE_IDS]
    assert CHECKS == ("numbers_in_brief", "caveats_carried", "nothing_unmeasured", "verdict_kept")
    # The headline is the first binary metric, and every one here is binary.
    assert runner.headline(FLOW.metrics).id == "all_checks"
    assert [metric.always for metric in FLOW.metrics] == [True] + [False] * 9


def test_the_cases_are_the_generators_each_with_the_brief_the_user_approved(
    cases: list[Case],
) -> None:
    made = coach_cases.load_cases(coach_cases.CASES)
    assert [case.id for case in cases] == [case.id for case in made]
    approved = approved_briefs((CASES_DIR / BRIEFS_FILE).read_text(encoding="utf-8"))
    for case, twin in zip(cases, made, strict=True):
        assert case.tags == twin.tags
        assert isinstance(case.input, SwingCase)
        assert (case.input.session_id, case.input.swing_id) == (twin.session_id, twin.swing_id)
        assert case.input.brief == approved[case.id]
        assert case.input.brief.startswith(f"SWING {twin.swing_id} (session {twin.session_id})")
        assert twin.about in case.prompt
    # The flow names the generator's paths again rather than importing the recorder (P15).
    assert CASES_DIR == coach_cases.CASES_DIR
    assert (CASES_FILE.name, BRIEFS_FILE, SESSIONS) == (
        coach_cases.CASES.name,
        coach_cases.CASES_MD,
        coach_cases.SESSIONS,
    )


def _copy_cases(tmp_path: Path) -> Path:
    for name in (CASES_FILE, BRIEFS_FILE):
        shutil.copyfile(CASES_DIR / name, tmp_path / name)
    return tmp_path


def test_a_case_with_no_approved_brief_is_refused(tmp_path: Path) -> None:
    folder = _copy_cases(tmp_path)
    markdown = (folder / BRIEFS_FILE).read_text(encoding="utf-8")
    (folder / BRIEFS_FILE).write_text(
        markdown.replace("## `clean-swing`", "## `renamed`"), encoding="utf-8"
    )
    with pytest.raises(ValueError, match="no brief for case 'clean-swing'"):
        CoachFeedbackFlow(folder).load_cases()


def test_a_brief_for_no_case_is_refused(tmp_path: Path) -> None:
    folder = _copy_cases(tmp_path)
    doc = json.loads((folder / CASES_FILE).read_text(encoding="utf-8"))
    doc["cases"] = doc["cases"][1:]
    (folder / CASES_FILE).write_text(json.dumps(doc), encoding="utf-8")
    with pytest.raises(ValueError, match=r"briefs for no case: \['clean-swing'\]"):
        CoachFeedbackFlow(folder).load_cases()


def test_the_flow_adds_nothing_to_what_the_coach_route_imports() -> None:
    """ADR-036 §1: the flow reaches the frozen lab only through the survivors it calls."""
    script = (
        "import sys\n"
        "import golf_coach.config, golf_coach.evals.runner, golf_coach.feedback.coach\n"
        "before = set(sys.modules)\n"
        "import golf_coach.evals.coach_feedback\n"
        "print(sorted(m for m in set(sys.modules) - before if m.startswith('golf_coach')))\n"
    )
    found = subprocess.run(
        [sys.executable, "-c", script], capture_output=True, text=True, check=True, cwd=REPO_ROOT
    )
    assert found.stdout.strip() == (
        "['golf_coach.evals.coach_feedback', 'golf_coach.evals.coach_judge']"
    )


def test_the_committed_state_declares_the_metrics_and_the_approved_inputs_as_harness() -> None:
    state = json.loads(STATE.read_text(encoding="utf-8"))
    assert state["metrics"] == [
        {"id": m.id, "kind": m.kind, "label": m.label, "better": m.better} for m in FLOW.metrics
    ]
    runner._check_state_metrics(state, FLOW.metrics, STATE)
    # cases.md holds every approved brief, and `run` refuses any other: so it stands in for the
    # sessions and golf-mcp. The route under test is not listed (P8's finding). The judge is the
    # grader, so its module is, where the runner would not hash it on its own (P17's finding).
    assert runner.harness_digest(state, FLOW).hashed == [
        "evals/coach_feedback/cases.json",
        "evals/coach_feedback/cases.md",
        "src/golf_coach/evals/coach_feedback.py",
        "src/golf_coach/evals/coach_judge.py",
        "src/golf_coach/evals/runner.py",
    ]
    # Nothing is asserted about `harness_sha`, as the club-spec twin asserts nothing. Until M43
    # P20 this line held it absent, and the user's `--approve-harness` in P20 recorded it, which
    # left the pin red from then until P23 found it. Absent or present, it is the user's record
    # (ADR-036 §6), and the runner's gate is what compares it with the files. A pin on its value
    # would turn every harness edit into a red suite that only the user's flag could clear.


# --------------------------------------------------------------------------- what the prompt says


def test_the_unmeasured_list_and_the_signed_checkpoint_are_read_off_the_prompt() -> None:
    assert unmeasured_terms(SYSTEM_PROMPT) == (
        "spine angle",
        "hip rotation",
        "swing plane",
        "club path",
    )
    assert signed_checkpoints(SYSTEM_PROMPT) == ("head_stays_back",)


def test_a_prompt_that_names_nothing_unmeasured_is_a_harness_error() -> None:
    with pytest.raises(AttemptFailed) as failed:
        unmeasured_terms("You are a golf coach.")
    assert failed.value.failure_class == "harness_error"


# --------------------------------------------------------------------------- numbers


@pytest.mark.parametrize(
    ("paragraph", "passed"),
    [
        ("Your hips slide 0.28 shoulder-widths, past 0.21.", True),
        ("About 0.3 shoulder-widths of slide.", True),  # 0.28 at one decimal
        ("Your tempo is 2.7 to 1, or about 3 to 1.", True),
        ("More than 90% of 1,399 tour swings; the 90th percentile.", True),
        ("A path of 1.5 degrees out-to-in.", True),  # the brief prints -1.5; signs are not read
        ("Your 151-yard carry.", True),
        ("Your hips slide 0.275 shoulder-widths.", False),  # more precision than the brief has
        ("Your hips slide 0.35 shoulder-widths.", False),
        ("That is 0.07 past the limit.", False),  # 0.28 - 0.21: worked, not in the brief
        ("Your downswing took 0.15 seconds.", False),  # 150 ms converted
        ("Your 165-yard carry.", False),
        # A whole number below ten nearly always rounds from something in a brief (the session's
        # date alone holds a 7), so an invented one is rarely caught: the docstring says so.
        ("With your 7-iron.", True),
    ],
)
def test_every_number_must_be_in_the_brief_at_the_paragraphs_rounding(
    cases: list[Case], paragraph: str, passed: bool
) -> None:
    brief = _case(cases, "clean-swing").input.brief
    result = check_numbers(paragraph, brief)
    assert result.passed is passed, result.explanation


def test_a_number_glued_to_a_word_is_not_a_quoted_number(cases: list[Case]) -> None:
    brief = _case(cases, "clean-swing").input.brief
    assert check_numbers("Read the T2 for a 3w or a 4h.", brief).explanation == (
        "the paragraph quotes no number"
    )


# --------------------------------------------------------------------------- hand-built views


def _checkpoint(
    name: str, observed: float, passed: bool, lead: str, *, one_sided: bool = False
) -> dict[str, Any]:
    return {
        "name": name,
        "observed": observed,
        "expected_low": 0.14,
        "expected_high": 0.5,
        "passed": passed,
        "score": 1.0 if passed else 0.6,
        "percentile": 50.0,
        "population_n": 458,
        "message": f"{lead} - {abs(observed)} shoulder-widths.",
        "one_sided": one_sided,
    }


def _view(**changes: Any) -> dict[str, Any]:
    """A `get_swing` view with the keys `brief_from_view` reads: one miss, two passes, no shot."""
    view: dict[str, Any] = {
        "session_id": "2026-01-01",
        "swing_id": "1",
        "overall_score": 80.0,
        "mechanics_score": 80.0,
        "outcome_score": None,
        "checkpoints": [
            _checkpoint("hip_shift_at_top", 0.28, False, "Hip slide going back", one_sided=True),
            _checkpoint("hip_sway", 0.38, True, "Good lower-body movement"),
            _checkpoint("head_stays_back", -0.25, True, "Head stayed behind the ball"),
        ],
        "unscored": [],
        "population": [],
        "rotation": [],
        "headline": "Work on hip shift at top first.",
        "tips": [{"checkpoint": "hip_shift_at_top", "severity": "minor", "text": "Hip slide."}],
        "shot": None,
        "alignment_quality": None,
        "alignment_caveat": None,
        "notes": [],
    }
    view.update(changes)
    return view


def _failing(paragraph: str, view: Mapping[str, Any]) -> dict[str, str]:
    """The checks that fail, with their explanations: empty when the paragraph passes all four."""
    grade = grade_paragraph(paragraph, brief=brief_from_view(view), system=SYSTEM_PROMPT, view=view)
    assert grade.explanation is not None
    assert grade.scores["all_checks"] == float(all(grade.scores[m] for m in CHECKS))
    return {m: grade.explanation[m] for m in CHECKS if grade.scores[m] == 0.0}


GOOD = (
    "Work on your hip slide going back: 0.28 shoulder-widths to the top is too much. Feel the "
    "trail hip turn behind you rather than slide. Your hip sway into the ball is good."
)


def test_a_paragraph_true_to_its_brief_passes_every_check() -> None:
    grade = grade_paragraph(
        GOOD, brief=brief_from_view(_view()), system=SYSTEM_PROMPT, view=_view()
    )
    assert grade.scores == dict.fromkeys(["all_checks", *CHECKS], 1.0)
    assert grade.explanation is not None
    assert grade.explanation["all_checks"] == "every check passed"


@pytest.mark.parametrize(
    "paragraph",
    [
        "Your hip shift at the top is great.",
        "Your hip slide going back looks solid.",  # named by its reading's lead
    ],
)
def test_praising_a_checkpoint_that_missed_reverses_the_verdict(paragraph: str) -> None:
    failing = _failing(paragraph, _view())
    assert list(failing) == ["verdict_kept"]
    assert "hip_shift_at_top missed its band" in failing["verdict_kept"]


def test_calling_a_passed_checkpoint_a_fault_reverses_the_verdict() -> None:
    failing = _failing("Your hip sway needs work.", _view())
    assert failing == {
        "verdict_kept": 'hip_sway passed, and "Your hip sway needs work." calls it a fault '
        "(needs work)"
    }


def test_less_is_not_better_on_a_two_sided_checkpoint_missed_low() -> None:
    """`interim-pivot-dtl`'s trap (P16): hip_sway missed on its low side, so quiet is not good."""
    low = _view(checkpoints=[_checkpoint("hip_sway", 0.11, False, "Little hip movement")])
    assert list(_failing("Your hip sway is nice and quiet.", low)) == ["verdict_kept"]
    assert _failing("Your hip sway is too small: 0.11 is under the tour range.", low) == {}


@pytest.mark.parametrize(
    "paragraph",
    [
        "Your hip sway is fine and your hip shift at top is too much.",  # two named
        "Your hip shift at top is not great.",  # negated
        "Your hip shift at top looks fine but is too much.",  # praise and fault together
    ],
)
def test_a_sentence_the_verdict_check_cannot_attribute_is_not_read(paragraph: str) -> None:
    assert _failing(paragraph, _view()) == {}


def test_an_unscored_checkpoint_must_be_said_to_be_unscored_where_it_is_named() -> None:
    view = _view(
        unscored=[
            {
                "name": "finish_balance",
                "reason": "too_few_frames",
                "why": "the window held fewer usable frames than the measurement needs",
                "refilming_helps": True,
                "family": "checkpoint",
            }
        ]
    )
    failing = _failing("Your finish balance looked solid.", view)
    assert list(failing) == ["caveats_carried"]
    assert "finish_balance is unscored" in failing["caveats_carried"]
    # Sentence by sentence: a later admission does not undo the claim.
    assert list(
        _failing("Your finish was balanced. Finish balance was not scored, though.", view)
    ) == ["caveats_carried"]
    assert _failing("Finish balance wasn't scored, so film the whole follow-through.", view) == {}


def _not_a_capture_problem(**changes: Any) -> dict[str, Any]:
    entry = {
        "name": "head_stays_back",
        "reason": "no_handedness",
        "why": "no golfer is attributed",
        "refilming_helps": False,
        "family": "checkpoint",
    }
    return _view(unscored=[{**entry, **changes}])


@pytest.mark.parametrize(
    "paragraph",
    [
        "Head stays back could not be scored, so film the swing again.",
        "Try a steadier camera next time.",
        "Re-record it from further back.",
    ],
)
def test_advice_to_film_again_drops_the_caveat_when_filming_would_not_help(paragraph: str) -> None:
    failing = _failing(paragraph, _not_a_capture_problem())
    assert list(failing) == ["caveats_carried"]
    assert (
        "no unscored entry here is a capture problem (no_handedness)"
        in (failing["caveats_carried"])
    )


def test_filming_again_is_fine_to_rule_out_or_to_advise_when_it_would_help() -> None:
    assert _failing("Filming again would not change that.", _not_a_capture_problem()) == {}
    helps = _not_a_capture_problem(reason="low_confidence", refilming_helps=True)
    assert _failing("Film the swing again in better light.", helps) == {}


def test_a_partly_aligned_pair_must_not_be_called_synchronized() -> None:
    view = _view(alignment_quality="top_impact", alignment_caveat="aligned on top and impact")
    assert list(_failing("The two views are in sync.", view)) == ["caveats_carried"]
    assert _failing("The views are not synchronized throughout.", view) == {}
    assert _failing("The two views are in sync.", _view(alignment_quality="full")) == {}


def test_an_uncalibrated_placement_is_never_read_alone() -> None:
    rows = [
        {
            "name": name,
            "view": "face-on",
            "calibrated": calibrated,
            "value": value,
            "unit": unit,
            "detail": "detail",
        }
        for name, calibrated, value, unit in (
            ("tour_trajectory_t2", True, 5.76, "sd_units"),
            ("tour_trajectory_q", False, 21.1, "shoulder_widths"),
        )
    ]
    view = _view(population=rows)
    failing = _failing("Your Q of 21.1 says the motion is far from tour shape.", view)
    assert list(failing) == ["caveats_carried"]
    assert "tour_trajectory_q is not calibrated" in failing["caveats_carried"]
    assert _failing("Your T2 of 5.76 is unusual, and the Q of 21.1 beside it is too.", view) == {}


def test_rotation_readings_must_be_called_interim() -> None:
    row = {
        "name": "pivot_hip_axis_drift_norm",
        "view": "face-on",
        "value": 0.31,
        "unit": "shoulder_widths",
        "interim_reason": "no reference population",
    }
    view = _view(rotation=[row])
    assert list(_failing("Your rotation reading of 0.31 is large.", view)) == ["caveats_carried"]
    assert _failing("The rotation reading of 0.31 is interim, so read it loosely.", view) == {}


def test_the_raw_signed_value_the_prompt_forbids_is_never_quoted() -> None:
    failing = _failing("Your head stays back reads -0.25.", _view())
    assert list(failing) == ["caveats_carried"]
    assert (
        "head_stays_back's raw value is quoted with its sign (-0.25)"
        in (failing["caveats_carried"])
    )
    assert _failing("Your head stays 0.25 shoulder-widths behind your hips.", _view()) == {}
    # A range's hyphen is not a sign.
    assert _failing("The tour range is 0.14-0.5 shoulder-widths; 0.25 sits in it.", _view()) == {}


def _flagged() -> dict[str, Any]:
    return _view(
        shot={
            "needs_review": True,
            "parse_confidence": 0.53,
            "warnings": ["smash_factor 1.87 is outside the plausible range"],
            "metrics": {"ball_speed": 113.0, "smash_factor": 1.87, "carry_distance": 151.0},
        }
    )


def test_a_flagged_shot_used_must_be_called_uncertain() -> None:
    failing = _failing("Your smash factor of 1.87 is excellent.", _flagged())
    assert list(failing) == ["caveats_carried"]
    assert (
        "(smash factor, 1.87) without saying the reading is uncertain"
        in (failing["caveats_carried"])
    )
    assert _failing("The launch monitor read 113 mph.", _flagged()) != {}
    assert (
        _failing("I wouldn't trust that 1.87 smash factor; the read was flagged.", _flagged()) == {}
    )
    # A paragraph that leaves the shot alone quotes nothing as fact.
    assert _failing(GOOD, _flagged()) == {}


@pytest.mark.parametrize(
    "paragraph",
    [
        "Your swing plane is too steep.",
        "Keep your spine angle through impact.",
        "Rotate your hips harder through the ball.",  # "hip rotation", inflected and reordered
        "The plane of your swing looks upright.",
    ],
)
def test_a_claim_about_what_the_prompt_says_is_not_measured_fails(paragraph: str) -> None:
    failing = _failing(paragraph, _view())
    assert list(failing) == ["nothing_unmeasured"]


def test_saying_it_was_not_measured_is_no_claim_and_a_printed_reading_is_exempt() -> None:
    assert _failing("Swing plane isn't measured here, so I can't speak to it.", _view()) == {}
    assert _failing("Your club path is 1.5 degrees out-to-in.", _view()) != {}  # nothing printed
    printed = _view(
        shot={
            "needs_review": False,
            "parse_confidence": 0.97,
            "warnings": [],
            "metrics": {"club_path": -1.5},
        }
    )
    grade = grade_paragraph(
        "Your club path is 1.5 degrees out-to-in.",
        brief=brief_from_view(printed),
        system=SYSTEM_PROMPT,
        view=printed,
    )
    assert grade.scores["nothing_unmeasured"] == 1.0
    assert grade.explanation is not None
    assert "club path exempt" in grade.explanation["nothing_unmeasured"]


def test_a_real_stored_briefs_band_estimator_note_asks_nothing_of_the_paragraph() -> None:
    """P16: no case carries the note, so the caveats must come from the view, not a fixed list."""
    view = _view(notes=[BAND_ESTIMATOR_NOTE])
    assert BAND_ESTIMATOR_NOTE in brief_from_view(view)
    assert _failing(GOOD, view) == {}


# --------------------------------------------------------------------------- what is graded


def _ctx(client: Any) -> CaseContext:
    return CaseContext(
        model=MODEL,
        variant="baseline",
        rep=0,
        client=RecordingClient(client),
        judge_client=RecordingClient(client),
    )


def _coached(text: str | None, stop_reason: str, note: str | None = None) -> Coached:
    view = _view()
    return Coached(
        text=text,
        note=note,
        stop_reason=stop_reason,
        brief=brief_from_view(view),
        system=SYSTEM_PROMPT,
        view=view,
    )


def test_a_refusal_is_a_failed_headline_with_nothing_else_checked_or_judged(
    cases: list[Case],
) -> None:
    ctx = _ctx(None)
    grade = FLOW.grade(cases[0], _coached(None, "refusal", "declined"), ctx)
    assert grade.scores == {"all_checks": 0.0}
    assert ctx.judge_client.calls == []


def test_no_text_is_a_genuine_failure_not_a_zero(cases: list[Case]) -> None:
    ctx = _ctx(None)
    with pytest.raises(AttemptFailed) as failed:
        FLOW.grade(cases[0], _coached(None, "end_turn", "no text"), ctx)
    assert failed.value.failure_class == "genuine_failure"
    assert ctx.judge_client.calls == []


def _verdict(**failed: str) -> str:
    """A judge's answer: every criterion passed but those named, failed for the reason given."""
    return json.dumps(
        {
            criterion.key: {
                "explanation": failed.get(criterion.key, f"{criterion.key} holds"),
                "passed": criterion.key not in failed,
            }
            for criterion in coach_judge.CRITERIA
        }
    )


def test_a_paragraph_is_graded_on_all_four_checks_and_by_the_judge(cases: list[Case]) -> None:
    client = _Coach(verdict=_verdict(top_fault="it leads with the hip sway"))
    coached = _coached(GOOD, "end_turn")
    grade = FLOW.grade(cases[0], coached, _ctx(client))

    assert grade.scores == {
        **dict.fromkeys(["all_checks", *CHECKS], 1.0),
        **dict.fromkeys(JUDGE_IDS, 1.0),
        "judge_all": 0.0,
        "judge_top_fault": 0.0,
    }
    assert grade.explanation is not None
    assert grade.explanation["all_checks"] == "every check passed"
    assert grade.explanation["judge_top_fault"] == "it leads with the hip sway"
    assert grade.explanation["judge_all"] == "failed: top_fault"
    # One judge request, built from what the coach was sent and what it wrote.
    (judged,) = client.judged
    assert judged == coach_judge.judge_request(
        instructions=coached.system, brief=coached.brief, paragraph=GOOD
    )
    assert client.requests == []


def test_the_judge_never_learns_which_model_wrote_the_paragraph(cases: list[Case]) -> None:
    """ADR-036 §8: blind. The same paragraph from two models is one and the same judge request."""
    requests = []
    for model in ("claude-opus-5", "claude-haiku-5-5"):
        client = _Coach()
        ctx = CaseContext(
            model=model,
            variant="baseline",
            rep=0,
            client=RecordingClient(client),
            judge_client=RecordingClient(client),
        )
        FLOW.grade(cases[0], _coached(GOOD, "end_turn"), ctx)
        (request,) = client.judged
        requests.append(request)
    assert requests[0] == requests[1]
    sent = json.dumps({key: value for key, value in requests[0].items() if key != "model"})
    for name in ("claude", "opus", "sonnet", "haiku"):
        assert name not in sent.lower()


def test_the_checks_and_the_judge_do_not_read_each_other(cases: list[Case]) -> None:
    """A paragraph failing a check is still judged, and a failing judge moves no check."""
    wrong = "Work on your hip slide going back: 0.35 shoulder-widths to the top is too much."
    client = _Coach(verdict=_verdict(accurate="0.35 is not in the brief"))
    grade = FLOW.grade(cases[0], _coached(wrong, "end_turn"), _ctx(client))
    assert len(client.judged) == 1
    assert grade.scores["numbers_in_brief"] == 0.0
    assert grade.scores["judge_accurate"] == 0.0
    assert grade.scores["verdict_kept"] == 1.0
    assert grade.scores["judge_actionable"] == 1.0


def test_an_unbuilt_golf_mcp_is_a_harness_error_before_any_request(
    tmp_path: Path, cases: list[Case]
) -> None:
    flow = CoachFeedbackFlow(golf_mcp=tmp_path / "golf-mcp.exe")
    client = _Coach()
    with pytest.raises(AttemptFailed) as failed:
        flow.run(cases[0], _ctx(client))
    assert failed.value.failure_class == "harness_error"
    assert client.requests == []


# --------------------------------------------------------------------------- through golf-mcp


def _response(text: str, stop_reason: str = "end_turn", model: str = MODEL) -> SimpleNamespace:
    """A response the route reads and the runner measures: the model, text, and non-zero usage."""
    usage = SimpleNamespace(
        input_tokens=4000,
        output_tokens=300,
        cache_read_input_tokens=None,
        cache_creation_input_tokens=None,
    )
    content = [SimpleNamespace(type="text", text=text)] if text else []
    return SimpleNamespace(model=model, stop_reason=stop_reason, content=content, usage=usage)


class _Coach:
    """A fake SDK client: one paragraph for every brief, or a refusal for the briefs named.

    It is the judge too, as the runner's one client is: a request for the judge's model is kept
    in `judged`, apart from the coaching `requests`, and answered with `verdict`.
    """

    def __init__(
        self,
        text: str = "Keep making this swing.",
        refuse: tuple[str, ...] = (),
        verdict: str | None = None,
    ) -> None:
        self.messages = self
        self.requests: list[dict[str, Any]] = []
        self.judged: list[dict[str, Any]] = []
        self._text = text
        self._refuse = refuse
        self._verdict = verdict if verdict is not None else _verdict()

    def create(self, **request: Any) -> SimpleNamespace:
        if request["model"] == coach_judge.JUDGE_MODEL:
            self.judged.append(request)
            return _response(self._verdict, model=coach_judge.JUDGE_MODEL)
        self.requests.append(request)
        brief = request["messages"][0]["content"]
        if any(brief.startswith(f"SWING {swing} (session") for swing in self._refuse):
            return _response("", stop_reason="refusal")
        return _response(self._text)


def _served(golf_mcp: Path, case: Case, shots: Path) -> dict[str, Any]:
    assert isinstance(case.input, SwingCase)
    server = GolfMcpServer(binary=golf_mcp, sessions_dir=CASES_DIR / SESSIONS, shots_dir=shots)
    with (shots.parent / f"{case.id}.log").open("w", encoding="utf-8") as errlog:
        read = read_swing(server, case.input.session_id, case.input.swing_id, errlog=errlog)
    assert read.view is not None
    return read.view


def test_every_case_is_served_as_its_approved_brief_from_the_runners_threads(
    golf_mcp: Path, tmp_path: Path, cases: list[Case]
) -> None:
    """P7's question: each read starts its own `golf-mcp`, so four threads at once share nothing.

    Eight cases, two rounds of the runner's default four: `test_coach_cases.py` serves all sixteen
    one at a time already.
    """
    (tmp_path / "shots").mkdir()
    picked = cases[:8]
    with ThreadPoolExecutor(max_workers=4) as pool:
        views = list(pool.map(lambda case: _served(golf_mcp, case, tmp_path / "shots"), picked))
    for case, view in zip(picked, views, strict=True):
        assert isinstance(case.input, SwingCase)
        assert brief_from_view(view) == case.input.brief, case.id


#: The plan's Done when, on the approved cases: one good paragraph and four bad ones per case.
REAL = {
    "clean-swing": (
        "The one thing to work on is your hips sliding away from the target on the way back: "
        "they travel 0.28 shoulder-widths by the top, more than at least 90% of tour swings, where "
        "you want to stay under 0.21. That slide is probably tied to your quick tempo, 2.7 to 1, "
        "because a rushed backswing gets there by drifting rather than turning. Try slow "
        "rehearsals with your trail hip staying over your trail foot. Your head stability and "
        "balanced finish are inside the tour range, and the launch monitor had you carrying it "
        "151 yards on a 1.5 degree out-to-in club path."
    ),
    "no-handedness": (
        "Work on the hip slide going back first: 0.21 shoulder-widths to the top puts you right "
        "at the edge of the tour range. Head stays back wasn't scored because no golfer is "
        "attributed to this swing, which filming again would not fix; naming who swung would."
    ),
    "interim-pivot-dtl": (
        "Work on your hip sway: at 0.11 shoulder-widths your hips barely move toward the target, "
        "under the 0.14 the tour range starts at, and on this checkpoint less is not better."
    ),
    "flagged-shot": (
        "Start with the hip slide on the way back: 0.24 shoulder-widths to the top is past the "
        "0.21 most tour players stay under. I'd ignore the launch monitor numbers on this one, "
        "since the reading was flagged and a 1.87 smash factor is not possible."
    ),
    "partial-alignment": (
        "Your hips slide 0.23 shoulder-widths going back, just past the tour limit of 0.21. The "
        "down-the-line clip only lines up with this one at the top and at impact, so don't read "
        "the timing between them too closely."
    ),
}

BAD = [
    ("clean-swing", "Your hips slide 0.35 shoulder-widths going back.", "numbers_in_brief"),
    ("flagged-shot", "Your 1.87 smash factor shows a great strike.", "caveats_carried"),
    ("no-handedness", "Head stays back was unscored, so film it again.", "caveats_carried"),
    ("partial-alignment", "The two views are synchronized throughout.", "caveats_carried"),
    ("clean-swing", "Your hip slide is flattening your swing plane.", "nothing_unmeasured"),
    ("interim-pivot-dtl", "Your hip sway is nice and quiet at 0.11.", "verdict_kept"),
    ("nothing-to-fix", "Your tempo needs work, so take it back longer.", "verdict_kept"),
]


@pytest.fixture
def real_views(golf_mcp: Path, tmp_path: Path, cases: list[Case]) -> dict[str, dict[str, Any]]:
    (tmp_path / "shots").mkdir()
    wanted = {*REAL, *(case_id for case_id, _, _ in BAD)}
    picked = [case for case in cases if case.id in wanted]
    with ThreadPoolExecutor(max_workers=4) as pool:
        views = pool.map(lambda case: _served(golf_mcp, case, tmp_path / "shots"), picked)
        return {case.id: view for case, view in zip(picked, views, strict=True)}


def _real_failing(
    case: Case, view: Mapping[str, Any], paragraph: str, system: str = SYSTEM_PROMPT
) -> list[str]:
    assert isinstance(case.input, SwingCase)
    grade = grade_paragraph(paragraph, brief=case.input.brief, system=system, view=view)
    return [metric for metric in CHECKS if grade.scores[metric] == 0.0]


def test_on_the_approved_cases_good_paragraphs_pass_and_each_bad_one_fails_its_check(
    real_views: dict[str, dict[str, Any]], cases: list[Case]
) -> None:
    for case_id, paragraph in REAL.items():
        assert _real_failing(_case(cases, case_id), real_views[case_id], paragraph) == [], case_id
    for case_id, paragraph, check in BAD:
        failing = _real_failing(_case(cases, case_id), real_views[case_id], paragraph)
        assert failing == [check], (case_id, paragraph)


def test_run_coaches_through_golf_mcp_with_the_runs_model_and_client(
    golf_mcp: Path, cases: list[Case]
) -> None:
    case = _case(cases, "clean-swing")
    assert isinstance(case.input, SwingCase)
    client = _Coach(REAL["clean-swing"])
    ctx = _ctx(client)
    coached = CoachFeedbackFlow(golf_mcp=golf_mcp).run(case, ctx)

    (request,) = client.requests
    assert request["model"] == MODEL
    assert request["messages"] == [{"role": "user", "content": case.input.brief}]
    assert coached.brief == case.input.brief
    assert coached.text == REAL["clean-swing"]
    assert coached.stop_reason == "end_turn"
    # The system prompt is golf-mcp's caveats, with the list the unmeasured check reads.
    assert unmeasured_terms(coached.system) == unmeasured_terms(SYSTEM_PROMPT)
    assert brief_from_view(coached.view) == case.input.brief
    grade = FLOW.grade(case, coached, ctx)
    assert grade.scores == dict.fromkeys(["all_checks", *CHECKS, *JUDGE_IDS], 1.0)
    # The judge reads the instructions the route sent, the approved brief and the paragraph.
    (judged,) = client.judged
    assert judged == coach_judge.judge_request(
        instructions=coached.system, brief=case.input.brief, paragraph=REAL["clean-swing"]
    )


def test_a_brief_nobody_approved_is_refused_before_any_request(
    golf_mcp: Path, tmp_path: Path
) -> None:
    folder = _copy_cases(tmp_path)
    shutil.copytree(CASES_DIR / SESSIONS, folder / SESSIONS)
    markdown = (folder / BRIEFS_FILE).read_text(encoding="utf-8")
    (folder / BRIEFS_FILE).write_text(
        markdown.replace("overall_score: 94 out of 100", "overall_score: 95 out of 100", 1),
        encoding="utf-8",
    )
    flow = CoachFeedbackFlow(folder, golf_mcp=golf_mcp)
    case = _case(flow.load_cases(), "clean-swing")
    client = _Coach()
    with pytest.raises(AttemptFailed) as failed:
        flow.run(case, _ctx(client))
    assert failed.value.failure_class == "harness_error"
    assert "not the one approved in cases.md, so no request was made" in str(failed.value)
    assert "line 3" in str(failed.value)
    assert client.requests == []


# --------------------------------------------------------------------------- through the runner


def _rows(runs: Path) -> dict[str, dict[str, Any]]:
    path = runs / "coach_feedback" / "baseline" / "results.jsonl"
    rows = [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines() if line]
    return {row["prompt_id"]: row for row in rows}


def _runs_dir(tmp_path: Path) -> Path:
    runs = tmp_path / "runs"
    (runs / "coach_feedback").mkdir(parents=True)
    shutil.copyfile(STATE, runs / "coach_feedback" / "_state.json")
    return runs


def test_the_runner_writes_a_row_per_case_and_grades_a_refusal_as_a_failed_headline(
    golf_mcp: Path, tmp_path: Path, cases: list[Case]
) -> None:
    """`--approve-harness` approves the committed harness in a scratch directory: the test's to do
    here, and the user's alone on a real run (ADR-036 §6)."""
    runs = _runs_dir(tmp_path)
    client = _Coach(refuse=("2",))  # swing 2 is `unscored-finish`
    flows = {"coach_feedback": CoachFeedbackFlow(golf_mcp=golf_mcp)}
    argv = ["--flow", "coach_feedback", "--model", MODEL, "--runs-dir", str(runs), "--limit", "4"]
    assert runner.main([*argv, "--approve-harness"], flows=flows, client=client) == runner.EXIT_OK
    assert len(client.requests) == 4
    assert len(client.judged) == 3  # the refusal has no paragraph to judge

    rows = _rows(runs)
    assert set(rows) == {case.id for case in cases[:4]}
    refused = rows["unscored-finish"]
    assert refused["status"] == "ok"
    assert refused["meta"]["failure_class"] == "refusal"
    assert refused["grade"] == {"all_checks": 0.0}
    assert "judge_model" not in refused
    for case_id, row in rows.items():
        assert row["meta"]["cost_usd"] > 0
        if case_id != "unscored-finish":
            # A paragraph that says nothing passes every code check: usefulness is the judge's,
            # and this fake judge passes everything.
            assert row["grade"] == dict.fromkeys(["all_checks", *CHECKS, *JUDGE_IDS], 1.0)
            assert row["judge_model"] == coach_judge.JUDGE_MODEL
            assert 0 < row["meta"]["judge_cost_usd"] < row["meta"]["cost_usd"]
    assert not (runs / "coach_feedback" / "baseline" / "errors.jsonl").exists()


# runpy says the module is already imported, and that second copy is the case under test.
@pytest.mark.filterwarnings("ignore:'golf_coach.evals.runner' found in sys.modules:RuntimeWarning")
def test_the_command_line_entry_runs_the_flow_by_name(
    golf_mcp: Path, tmp_path: Path, cases: list[Case], monkeypatch: pytest.MonkeyPatch
) -> None:
    """`python -m golf_coach.evals.runner --flow coach_feedback`, as the user types it (P13's
    finding: a test passing `flows=` cannot see an entry-point fault). `GOLF_MCP` is how the
    command finds the server, read at each run rather than at import."""
    client = _Coach(REAL["clean-swing"])
    client.api_key = "not-a-key"  # what `_build_client` reads as a credential the SDK resolved
    monkeypatch.setattr(runner, "_sdk", lambda: SimpleNamespace(Anthropic=lambda **_: client))
    monkeypatch.setenv("GOLF_MCP", str(golf_mcp))
    runs = _runs_dir(tmp_path)
    argv = ["--flow", "coach_feedback", "--model", MODEL, "--runs-dir", str(runs)]
    monkeypatch.setattr(sys, "argv", ["runner", *argv, "--limit", "2", "--approve-harness"])

    with pytest.raises(SystemExit) as exited:
        runpy.run_module("golf_coach.evals.runner", run_name="__main__", alter_sys=True)

    assert exited.value.code == runner.EXIT_OK
    assert len(client.requests) == 2
    assert len(client.judged) == 2
    rows = _rows(runs)
    assert set(rows) == {case.id for case in cases[:2]}
    assert rows["clean-swing"]["grade"]["all_checks"] == 1.0
    assert rows["clean-swing"]["grade"]["judge_all"] == 1.0
