"""The club-spec flow and its grader, offline. [M43 P12]

Nothing here makes a request: the route runs against a fake client, and the grader against specs
built in the test. The three checks the plan's P12 asks for are here by name. The answer key fed
back scores 100% accuracy and 0% invented; an all-blank answer scores 0 coverage and 0 invented;
and a hand-built mixed case matches grades worked by hand. A share with nothing to divide by is
absent from the grade, and the tests pin where. The oracle and the null then go through the whole
runner (`runner.main` on a fake client answering from the key, with the committed `_state.json`),
which is where an absent metric has to be accepted and averaged over the cases that hold it.
"""

from __future__ import annotations

import json
import re
import runpy
import shutil
import sys
from pathlib import Path
from types import SimpleNamespace
from typing import Any

import pytest

from golf_coach.config import REPO_ROOT
from golf_coach.contracts.club import ClubId
from golf_coach.contracts.club_spec import ClubSpec
from golf_coach.evals import club_spec, runner
from golf_coach.evals.club_spec import (
    FLOW,
    KEY,
    Answer,
    ClubSpecFlow,
    Outcome,
    SetCase,
    classify,
    field_results,
    grade_specs,
)
from golf_coach.evals.runner import AttemptFailed, Case, CaseContext, RecordingClient

MODEL = "claude-opus-5"
FIELDS = ("loft_deg", "lie_deg", "length_in", "offset_mm", "bounce_deg")
#: The real sets whose maker printed all five fields for every slot: no invented rate is defined.
EVERY_FIELD_PRINTED = {"ping-i230", "pxg-0311-xp-gen7", "wilson-staff-model-cb"}


@pytest.fixture(scope="module")
def cases() -> list[Case]:
    return FLOW.load_cases()


def _asked(case: Case) -> SetCase:
    assert isinstance(case.input, SetCase)
    return case.input


def _oracle(asked: SetCase) -> list[ClubSpec]:
    """The answer key fed back as the model's output: every published value, every blank kept."""
    return [
        ClubSpec(
            club=slot,
            make=asked.make,
            model=asked.model,
            model_year=asked.model_year,
            **asked.published[slot],
        )
        for slot in asked.slots
    ]


# --------------------------------------------------------------------------- the flow's shape


def test_the_runner_finds_the_flow_by_its_module_name() -> None:
    assert isinstance(club_spec.FLOW, runner.Flow)
    assert FLOW.judge_model is None
    assert [metric.id for metric in FLOW.metrics] == [
        "invented",
        "accuracy",
        "coverage",
        "abstention",
    ]
    # Every metric is a float, so the report's headline is the first: the invented rate.
    assert runner.headline(FLOW.metrics).id == "invented"
    assert FLOW.metrics[0].better == "lower"
    assert all(metric.better == "higher" for metric in FLOW.metrics[1:])
    assert all(0 < len(metric.label) <= runner.LABEL_LIMIT for metric in FLOW.metrics)
    assert all(metric.scale == 1.0 for metric in FLOW.metrics)


def test_every_verified_set_in_the_key_loads_as_one_case(cases: list[Case]) -> None:
    doc = json.loads(KEY.read_text(encoding="utf-8"))
    assert [case.id for case in cases] == [raw["id"] for raw in doc["cases"]]
    for case, raw in zip(cases, doc["cases"], strict=True):
        asked = _asked(case)
        assert asked.make == raw["make"]
        assert asked.model == raw["model"]
        assert asked.model_year == raw["model_year"]
        assert [slot.value for slot in asked.slots] == raw["slots"]
        assert asked.fields == FIELDS
        assert asked.fabricated == ("fabricated" in raw["tags"])
        assert case.tags == tuple(raw["tags"])
        assert case.meta["slots"] * len(FIELDS) == (
            case.meta["published"] + case.meta["unpublished"]
        )
        assert raw["make"] in case.prompt
        assert raw["model"] in case.prompt
    # P11's three made-up sets, and the driver whose loft is in its model text.
    assert sum(_asked(case).fabricated for case in cases) == 3
    driver = next(case for case in cases if case.id == "taylormade-r9-460")
    assert _asked(driver).slots == (ClubId.DRIVER,)
    assert "model year not given" in driver.prompt


def test_the_tolerances_cover_exactly_the_keys_fields() -> None:
    doc = json.loads(KEY.read_text(encoding="utf-8"))
    assert set(club_spec.TOLERANCE) == set(doc["fields"])
    assert set(club_spec.TOLERANCE) <= set(ClubSpec.model_fields)


def _key_with(tmp_path: Path, change: Any) -> ClubSpecFlow:
    doc = json.loads(KEY.read_text(encoding="utf-8"))
    change(doc)
    path = tmp_path / "cases.json"
    path.write_text(json.dumps(doc), encoding="utf-8")
    return ClubSpecFlow(path)


def test_a_set_the_user_has_not_verified_is_refused_not_graded(tmp_path: Path) -> None:
    def unverify(doc: dict[str, Any]) -> None:
        doc["cases"][3]["verified"] = None

    with pytest.raises(ValueError, match="no `verified` date"):
        _key_with(tmp_path, unverify).load_cases()


def test_a_key_field_with_no_tolerance_is_refused(tmp_path: Path) -> None:
    def add_field(doc: dict[str, Any]) -> None:
        doc["fields"].append("swing_weight")

    with pytest.raises(ValueError, match="TOLERANCE covers"):
        _key_with(tmp_path, add_field).load_cases()


# --------------------------------------------------------------------------- the plan's checks


def test_the_answer_key_fed_back_scores_full_accuracy_and_nothing_invented(
    cases: list[Case],
) -> None:
    for case in cases:
        asked = _asked(case)
        scores = grade_specs(asked, _oracle(asked)).scores
        if asked.fabricated:
            assert scores == {"invented": 0.0, "abstention": 1.0}, case.id
        elif case.id in EVERY_FIELD_PRINTED:
            assert scores == {"accuracy": 1.0, "coverage": 1.0}, case.id
        else:
            assert scores == {"invented": 0.0, "accuracy": 1.0, "coverage": 1.0}, case.id


def test_an_all_blank_answer_covers_nothing_and_invents_nothing(cases: list[Case]) -> None:
    for case in cases:
        asked = _asked(case)
        # The route's two spellings of "nothing": no specs, and a spec per slot holding only its
        # identity (every field null). They must grade alike.
        identity_only = [
            ClubSpec(club=slot, make=asked.make, model=asked.model, model_year=asked.model_year)
            for slot in asked.slots
        ]
        for specs in ((), identity_only):
            scores = grade_specs(asked, specs).scores
            # No published value was answered, so accuracy has nothing to divide by anywhere.
            if asked.fabricated:
                assert scores == {"invented": 0.0, "abstention": 1.0}, case.id
            elif case.id in EVERY_FIELD_PRINTED:
                assert scores == {"coverage": 0.0}, case.id
            else:
                assert scores == {"invented": 0.0, "coverage": 0.0}, case.id


def _mixed() -> SetCase:
    return SetCase(
        make="Example",
        model="X1",
        model_year=2024,
        slots=(ClubId.SEVEN_IRON, ClubId.EIGHT_IRON),
        published={
            ClubId.SEVEN_IRON: {
                "loft_deg": 31.0,
                "lie_deg": 62.5,
                "length_in": 37.0,
                "offset_mm": None,
                "bounce_deg": None,
            },
            ClubId.EIGHT_IRON: {
                "loft_deg": 35.0,
                "lie_deg": 63.0,
                "length_in": 36.5,
                "offset_mm": 2.5,
                "bounce_deg": None,
            },
        },
        fields=FIELDS,
        fabricated=False,
    )


def test_a_hand_built_mixed_case_matches_grades_worked_by_hand() -> None:
    asked = _mixed()
    specs = [
        # 7i: loft 0.5 off (right, at the edge), lie 1.0 off (wrong), length blank (missed),
        # offset 0 where none is published (invented), bounce blank (abstained).
        ClubSpec(club="7i", loft_deg=31.5, lie_deg=63.5, offset_mm=0.0),
        # 8i: loft 0.4 off (right), lie exact (right), length 0.26 off (wrong, just outside the
        # quarter inch), offset 0.25 off (right, at the edge), bounce blank (abstained).
        ClubSpec(club="8i", loft_deg=34.6, lie_deg=63.0, length_in=36.76, offset_mm=2.75),
    ]
    assert [r.outcome for r in field_results(asked, specs)] == [
        Outcome.RIGHT,
        Outcome.WRONG,
        Outcome.MISSED,
        Outcome.INVENTED,
        Outcome.ABSTAINED,
        Outcome.RIGHT,
        Outcome.RIGHT,
        Outcome.WRONG,
        Outcome.RIGHT,
        Outcome.ABSTAINED,
    ]
    # By hand: 4 right, 2 wrong, 1 missed, 1 invented, 2 abstained, over 10 fields.
    grade = grade_specs(asked, specs)
    assert grade.scores == pytest.approx({"invented": 1 / 3, "accuracy": 4 / 6, "coverage": 6 / 7})
    assert grade.explanation == {
        "invented": "1 of 3 unpublished fields filled in: 7i offset_mm 0",
        "accuracy": "4 of 6 answered published values within tolerance: 7i lie_deg 63.5 "
        "(published 62.5), 8i length_in 36.76 (published 36.5)",
        "coverage": "6 of 7 published values answered: 7i length_in",
    }


def test_a_made_up_model_scores_abstention_over_every_field() -> None:
    asked = SetCase(
        make="Example",
        model="Never Made",
        model_year=None,
        slots=(ClubId.SEVEN_IRON,),
        published={ClubId.SEVEN_IRON: dict.fromkeys(FIELDS)},
        fields=FIELDS,
        fabricated=True,
    )
    grade = grade_specs(asked, [ClubSpec(club="7i", loft_deg=30.0)])
    assert grade.scores == {"invented": 1 / 5, "abstention": 4 / 5}
    assert grade.explanation is not None
    assert grade.explanation["abstention"] == "4 of 5 fields left blank on a made-up model"


def test_a_zero_where_the_maker_printed_nothing_is_invented(cases: list[Case]) -> None:
    """P11's question: a driver's offset or bounce given as 0, where the R9 460's page printed none.

    A zero is a number, and nobody published it, so it is graded as any other invented figure.
    """
    driver = _asked(next(case for case in cases if case.id == "taylormade-r9-460"))
    published = driver.published[ClubId.DRIVER]
    assert published["offset_mm"] is None
    assert published["bounce_deg"] is None
    spec = _oracle(driver)[0].model_copy(update={"offset_mm": 0.0, "bounce_deg": 0.0})
    scores = grade_specs(driver, [spec]).scores
    assert scores == {"invented": 1.0, "accuracy": 1.0, "coverage": 1.0}


@pytest.mark.parametrize(
    ("field", "published", "answered", "outcome"),
    [
        ("loft_deg", 31.0, 30.5, Outcome.RIGHT),
        ("loft_deg", 31.0, 30.49, Outcome.WRONG),
        ("length_in", 38.75, 38.5, Outcome.RIGHT),
        # Not exactly 0.25 in binary floating point, and still at the edge.
        ("offset_mm", 3.16, 2.91, Outcome.RIGHT),
        ("offset_mm", 3.16, 2.9, Outcome.WRONG),
        ("bounce_deg", 6.0, 0.0, Outcome.WRONG),
        ("bounce_deg", None, None, Outcome.ABSTAINED),
        ("bounce_deg", None, 0.0, Outcome.INVENTED),
        ("lie_deg", 62.0, None, Outcome.MISSED),
    ],
)
def test_classify(
    field: str, published: float | None, answered: float | None, outcome: Outcome
) -> None:
    assert classify(field, published, answered) is outcome


# --------------------------------------------------------------------------- through the route


class _Block:
    def __init__(self, text: str) -> None:
        self.type = "text"
        self.text = text


class _Response:
    def __init__(self, text: str, stop_reason: str = "end_turn") -> None:
        self.content = [_Block(text)]
        self.stop_reason = stop_reason
        self.model = MODEL


class _FakeMessages:
    def __init__(self, response: _Response) -> None:
        self.response = response
        self.requests: list[dict[str, Any]] = []

    def create(self, **request: Any) -> _Response:
        self.requests.append(request)
        return self.response


class _FakeClient:
    def __init__(self, response: _Response) -> None:
        self.messages = _FakeMessages(response)


def _ctx(response: _Response) -> tuple[CaseContext, _FakeClient]:
    fake = _FakeClient(response)
    ctx = CaseContext(
        model=MODEL,
        variant="baseline",
        rep=0,
        client=RecordingClient(fake),
        judge_client=RecordingClient(fake),
    )
    return ctx, fake


def _as_json(specs: list[ClubSpec]) -> str:
    """The specs as the model writes them: every field the schema asks for, blanks as null."""
    rows = []
    for spec in specs:
        row: dict[str, Any] = {"club": spec.club.value, "notes": ""}
        for field in FIELDS:
            row[field] = getattr(spec, field)
        rows.append(row)
    return json.dumps({"clubs": rows})


def _case(cases: list[Case], case_id: str) -> Case:
    return next(case for case in cases if case.id == case_id)


def test_run_calls_the_set_lookup_with_the_runs_model_and_client(cases: list[Case]) -> None:
    case = _case(cases, "takomo-201-mkii")
    asked = _asked(case)
    ctx, fake = _ctx(_Response(_as_json(_oracle(asked))))
    answer = FLOW.run(case, ctx)

    assert isinstance(answer, Answer)
    assert answer.stop_reason == "end_turn"
    (request,) = fake.messages.requests
    assert request["model"] == MODEL
    assert len(ctx.client.calls) == 1
    brief = request["messages"][0]["content"]
    assert "make: Takomo" in brief
    assert "model: 201 MKII" in brief
    assert "model year: not given" in brief
    # One request for the whole set, in the key's slot order (ADR-026 §6).
    schema = request["output_config"]["format"]["schema"]
    slots = schema["properties"]["clubs"]["items"]["properties"]["club"]["enum"]
    assert slots == [slot.value for slot in asked.slots]
    assert FLOW.grade(case, answer, ctx) == grade_specs(asked, _oracle(asked))


def test_a_refusal_is_graded_as_an_answer_with_every_field_blank(cases: list[Case]) -> None:
    for case_id, expected in (
        ("takomo-201-mkii", {"invented": 0.0, "coverage": 0.0}),
        ("titleist-t275", {"invented": 0.0, "abstention": 1.0}),
    ):
        case = _case(cases, case_id)
        ctx, _ = _ctx(_Response(_as_json([]), stop_reason="refusal"))
        answer = FLOW.run(case, ctx)
        assert answer.stop_reason == "refusal"
        assert answer.outcome.specs == ()
        assert FLOW.grade(case, answer, ctx).scores == expected


@pytest.mark.parametrize("text", ["the 7 iron is about 31 degrees", '{"irons": []}'])
def test_an_answer_with_nothing_to_grade_is_a_genuine_failure_not_a_zero(
    cases: list[Case], text: str
) -> None:
    case = _case(cases, "takomo-201-mkii")
    ctx, _ = _ctx(_Response(text))
    answer = FLOW.run(case, ctx)
    with pytest.raises(AttemptFailed) as failed:
        FLOW.grade(case, answer, ctx)
    assert failed.value.failure_class == "genuine_failure"


# --------------------------------------------------------------------------- through the runner

STATE = REPO_ROOT / ".claude" / "hillclimb" / "club_spec" / "_state.json"


def test_the_committed_state_declares_the_flows_metrics_and_the_key_as_harness() -> None:
    state = json.loads(STATE.read_text(encoding="utf-8"))
    # The report draws these columns, and the runner refuses any list that is not the flow's.
    assert state["metrics"] == [
        {"id": m.id, "kind": m.kind, "label": m.label, "better": m.better, "scale": m.scale}
        for m in FLOW.metrics
    ]
    runner._check_state_metrics(state, FLOW.metrics, STATE)
    # The key is grader data, so an edit to it stops the next run for the user's approval. The
    # route under test is not listed: a hillclimb round edits it (P8's finding).
    assert state["harness_paths"] == ["evals/club_spec/cases.json"]
    hashed = runner.harness_digest(state, FLOW).hashed
    assert hashed == [
        "evals/club_spec/cases.json",
        "src/golf_coach/evals/club_spec.py",
        "src/golf_coach/evals/runner.py",
    ]
    # Every row is priced from llm_prices.json, so a second price table would only drift.
    assert "prices" not in state


def _usage_response(text: str) -> SimpleNamespace:
    """A response the runner can measure: the requested model, and usage that is not zero."""
    usage = SimpleNamespace(
        input_tokens=1500,
        output_tokens=400,
        cache_read_input_tokens=None,
        cache_creation_input_tokens=None,
    )
    content = [SimpleNamespace(type="text", text=text)]
    return SimpleNamespace(model=MODEL, stop_reason="end_turn", content=content, usage=usage)


class _KeyClient:
    """Answers each set's request from the key: the published values, or every field blank."""

    def __init__(self, cases: list[Case], *, blank: bool) -> None:
        self.messages = self
        self.requests: list[dict[str, Any]] = []
        self._by_name = {(_asked(c).make, _asked(c).model): _asked(c) for c in cases}
        assert len(self._by_name) == len(cases), "two sets share a make and model"
        self._blank = blank

    def create(self, **request: Any) -> SimpleNamespace:
        self.requests.append(request)
        brief = request["messages"][0]["content"]
        make = re.search(r"^make: (.*)$", brief, re.MULTILINE)
        model = re.search(r"^model: (.*)$", brief, re.MULTILINE)
        assert make is not None and model is not None
        asked = self._by_name[(make.group(1), model.group(1))]
        specs = _oracle(asked)
        if self._blank:
            specs = [ClubSpec(club=spec.club) for spec in specs]
        return _usage_response(_as_json(specs))


def _run_through_the_runner(tmp_path: Path, client: _KeyClient) -> int:
    """`runner.main` on the real flow, with the committed `_state.json` copied beside the run.

    `--approve-harness` here approves the committed harness in a temporary directory, which is the
    test's to do; on a real run that flag is the user's alone (ADR-036 §6).
    """
    flow_dir = tmp_path / "club_spec"
    flow_dir.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(STATE, flow_dir / "_state.json")
    return runner.main(
        ["--flow", "club_spec", "--model", MODEL, "--runs-dir", str(tmp_path)]
        + ["--concurrency", "2", "--approve-harness"],
        flows={"club_spec": FLOW},
        client=client,
    )


def _rows(tmp_path: Path) -> dict[str, dict[str, Any]]:
    path = tmp_path / "club_spec" / "baseline" / "results.jsonl"
    rows = [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines() if line]
    return {row["prompt_id"]: row for row in rows}


def test_the_oracle_through_the_runner_invents_nothing_and_gets_every_value_right(
    tmp_path: Path, cases: list[Case], capsys: pytest.CaptureFixture[str]
) -> None:
    """eval-audit.md §2, the oracle: the key fed back, through the whole runner."""
    client = _KeyClient(cases, blank=False)
    assert _run_through_the_runner(tmp_path, client) == runner.EXIT_OK
    assert len(client.requests) == len(cases)

    rows = _rows(tmp_path)
    assert set(rows) == {case.id for case in cases}
    for case in cases:
        row = rows[case.id]
        assert row["status"] == "ok"
        assert row["grade"] == grade_specs(_asked(case), _oracle(_asked(case))).scores
        assert row["meta"]["cost_usd"] > 0
        assert row["meta"]["published"] + row["meta"]["unpublished"] == 5 * row["meta"]["slots"]
    assert not (tmp_path / "club_spec" / "baseline" / "errors.jsonl").exists()

    out = capsys.readouterr().out
    real, made_up = len(cases) - 3, 3
    with_invented = len(cases) - len(EVERY_FIELD_PRINTED)
    assert f"Invented 0.000 (95% CI 0.000 to 0.000, {with_invented} cases)" in out
    assert f"also Accuracy 1.000 (95% CI 1.000 to 1.000, {real} cases)" in out
    assert f"also Coverage 1.000 (95% CI 1.000 to 1.000, {real} cases)" in out
    assert f"also Made-up blank 1.000 (95% CI 1.000 to 1.000, {made_up} cases)" in out


def test_the_null_through_the_runner_covers_nothing_and_invents_nothing(
    tmp_path: Path, cases: list[Case], capsys: pytest.CaptureFixture[str]
) -> None:
    """eval-audit.md §2, the null: every slot answered and every field left blank."""
    assert _run_through_the_runner(tmp_path, _KeyClient(cases, blank=True)) == runner.EXIT_OK

    rows = _rows(tmp_path)
    assert len(rows) == len(cases)
    assert all("accuracy" not in row["grade"] for row in rows.values())
    out = capsys.readouterr().out
    real = len(cases) - 3
    with_invented = len(cases) - len(EVERY_FIELD_PRINTED)
    assert f"Invented 0.000 (95% CI 0.000 to 0.000, {with_invented} cases)" in out
    # Nothing published was answered, so accuracy has nothing to divide by on any case.
    assert "also Accuracy: no scored rows" in out
    assert f"also Coverage 0.000 (95% CI 0.000 to 0.000, {real} cases)" in out
    assert "also Made-up blank 1.000 (95% CI 1.000 to 1.000, 3 cases)" in out


# runpy says the module is already imported, and that second copy is the case under test.
@pytest.mark.filterwarnings("ignore:'golf_coach.evals.runner' found in sys.modules:RuntimeWarning")
def test_the_command_line_entry_runs_the_flow_by_name_through_every_case(
    tmp_path: Path, cases: list[Case], monkeypatch: pytest.MonkeyPatch
) -> None:
    """`python -m golf_coach.evals.runner --flow club_spec`, as the user types it, on a fake SDK.

    Run that way, the runner's file is `__main__`: a second copy of the module, whose classes no
    flow derives from. The user's first pilot was refused by name, before any request (M43 P13).
    `runpy` runs the file as `__main__` here too, and the fake SDK takes the run past the flow
    lookup into the cases. There, `Case`, `Grade` and `AttemptFailed` are checked by class as well.
    `test_runner.py` runs the real subprocess as far as the harness gate, which is as far as it can
    go without a request.
    """
    picked = cases[:2]
    client = _KeyClient(picked, blank=False)
    # What `_build_client` reads as a credential the SDK resolved for itself.
    client.api_key = "not-a-key"
    monkeypatch.setattr(runner, "_sdk", lambda: SimpleNamespace(Anthropic=lambda **_: client))
    flow_dir = tmp_path / "club_spec"
    flow_dir.mkdir()
    shutil.copyfile(STATE, flow_dir / "_state.json")
    argv = ["--flow", "club_spec", "--model", MODEL, "--runs-dir", str(tmp_path)]
    monkeypatch.setattr(sys, "argv", ["runner", *argv, "--limit", "2", "--approve-harness"])

    with pytest.raises(SystemExit) as exited:
        runpy.run_module("golf_coach.evals.runner", run_name="__main__", alter_sys=True)

    assert exited.value.code == runner.EXIT_OK
    assert len(client.requests) == len(picked)
    rows = _rows(tmp_path)
    assert set(rows) == {case.id for case in picked}
    for case in picked:
        assert rows[case.id]["status"] == "ok"
        assert rows[case.id]["grade"] == grade_specs(_asked(case), _oracle(_asked(case))).scores
    assert not (flow_dir / "baseline" / "errors.jsonl").exists()
