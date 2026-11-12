"""The eval runner on a fake flow and a fake client: no request leaves the machine. [M43 P7, P8]

The fake flow asks six arithmetic questions the way a route asks Claude: through `ctx.client`, and
with an SDK error turned into a note rather than raised. The fake client stands in for
`anthropic.Anthropic`, so each test sets what the "model" answers, which model serves it, what it
costs and how it fails. Everything below goes through `runner.main`, the whole runner, except where
a test names the one function it pins.

`run_eval` passes `--approve-harness` unless told not to. Here that approves a fake flow in a
temporary directory, which is the gate's own test; on a real run the flag is the user's alone.
"""

from __future__ import annotations

import hashlib
import json
import re
import subprocess
import sys
import threading
import time
from collections.abc import Callable
from dataclasses import replace
from pathlib import Path
from types import SimpleNamespace
from typing import Any

import pytest
from pydantic import SecretStr

from golf_coach import llm_log
from golf_coach.config import REPO_ROOT, settings
from golf_coach.evals import runner
from golf_coach.evals.runner import AttemptFailed, Case, CaseContext, Flow, Grade, Metric

MODEL = "claude-opus-5"
JUDGE = "claude-opus-5-5"
SYSTEM = "Answer with the number only."

#: case id -> (question, answer, tag)
QUESTIONS = {
    "add": ("What is 2 + 3?", "5", "sums"),
    "subtract": ("What is 9 - 4?", "5", "sums"),
    "multiply": ("What is 6 x 7?", "42", "products"),
    "square": ("What is 8 squared?", "64", "products"),
    "halve": ("What is half of 30?", "15", "fractions"),
    "third": ("What is a third of 27?", "9", "fractions"),
}
ANSWERS = {question: answer for question, answer, _ in QUESTIONS.values()}


# --- the fakes -------------------------------------------------------------------------------


def response(
    text: str,
    *,
    model: str = MODEL,
    stop_reason: str = "end_turn",
    input_tokens: int | None = 120,
    output_tokens: int | None = 8,
    thinking: str | None = None,
    usage: bool = True,
) -> SimpleNamespace:
    content: list[Any] = []
    if thinking is not None:
        content.append(SimpleNamespace(type="thinking", thinking=thinking))
    content.append(SimpleNamespace(type="text", text=text))
    tokens = SimpleNamespace(
        input_tokens=input_tokens,
        output_tokens=output_tokens,
        cache_read_input_tokens=None,
        cache_creation_input_tokens=None,
    )
    return SimpleNamespace(
        model=model, stop_reason=stop_reason, content=content, usage=tokens if usage else None
    )


class FakeClient:
    """Stands in for `anthropic.Anthropic`. `answer(request)` returns a response or raises."""

    def __init__(self, answer: Callable[[dict[str, Any]], Any], *, delay: float = 0.0) -> None:
        self.messages = self
        self.requests: list[dict[str, Any]] = []
        self._answer = answer
        self._delay = delay
        self._lock = threading.Lock()
        self._in_flight = 0
        self.peak = 0

    def create(self, **request: Any) -> Any:
        with self._lock:
            self.requests.append(request)
            self._in_flight += 1
            self.peak = max(self.peak, self._in_flight)
        try:
            if self._delay:
                time.sleep(self._delay)
            return self._answer(request)
        finally:
            with self._lock:
                self._in_flight -= 1


def question(request: dict[str, Any]) -> str:
    content: str = request["messages"][-1]["content"]
    return content


def oracle(**kwargs: Any) -> FakeClient:
    return FakeClient(lambda request: response(ANSWERS[question(request)], **kwargs))


def failing_on(prompt: str, error: Exception, **kwargs: Any) -> FakeClient:
    def answer(request: dict[str, Any]) -> Any:
        if question(request) == prompt:
            raise error
        return response(ANSWERS[question(request)], **kwargs)

    return FakeClient(answer)


class RateLimitError(Exception):
    status_code = 429


class OverloadedError(Exception):
    status_code = 529


class AuthenticationError(Exception):
    status_code = 401


class APIConnectionError(Exception):
    pass


class APITimeoutError(APIConnectionError):
    pass


class Arithmetic(Flow):
    """A fake flow that runs as a route does: an SDK error comes back as a note."""

    metrics = (
        Metric("correct", "Correct", "binary"),
        Metric("chars", "Answer length", "float", better="lower", scale=100),
    )

    def __init__(self) -> None:
        self.origins: list[str] = []
        self.graded: list[str] = []

    def load_cases(self) -> list[Case]:
        return [
            Case(
                id=case_id,
                prompt=prompt,
                tags=(tag, "arithmetic"),
                input={"answer": answer},
                meta={"source": "hand-written"},
            )
            for case_id, (prompt, answer, tag) in QUESTIONS.items()
        ]

    def run(self, case: Case, ctx: CaseContext) -> dict[str, str]:
        self.origins.append(llm_log.origin())
        try:
            answer = ctx.client.messages.create(
                model=ctx.model,
                max_tokens=64,
                system=SYSTEM,
                messages=[{"role": "user", "content": case.prompt}],
            )
        except Exception as exc:
            return {"note": f"Unavailable ({type(exc).__name__})."}
        return {"text": "".join(b.text for b in answer.content if b.type == "text")}

    def grade(self, case: Case, output: dict[str, str], ctx: CaseContext) -> Grade:
        self.graded.append(case.id)
        text = output.get("text", "")
        correct = float(text.strip() == case.input["answer"])
        return Grade(
            scores={"correct": correct, "chars": float(len(text))},
            explanation={"correct": f"expected {case.input['answer']}"},
        )


def run_eval(
    runs_dir: Path,
    client: Any,
    *args: str,
    flow: Flow | None = None,
    approve: bool = True,
) -> int:
    return runner.main(
        ["--flow", "fake", "--model", MODEL, "--runs-dir", str(runs_dir), "--concurrency", "1"]
        + (["--approve-harness"] if approve else [])
        + list(args),
        flows={"fake": flow or Arithmetic()},
        client=client,
    )


REAL_BACKOFF = runner._backoff_delay


@pytest.fixture(autouse=True)
def backoffs(monkeypatch: pytest.MonkeyPatch) -> list[int]:
    """No real sleeping: every backoff is a millisecond, and the retry it came before is kept."""
    asked: list[int] = []

    def delay(retry: int) -> float:
        asked.append(retry)
        return 0.001

    monkeypatch.setattr(runner, "_backoff_delay", delay)
    return asked


def lines(path: Path) -> list[dict[str, Any]]:
    if not path.exists():
        return []
    return [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines() if line]


def baseline(runs_dir: Path) -> Path:
    return runs_dir / "fake" / "baseline"


# --- the oracle and the null -----------------------------------------------------------------


def test_the_oracle_scores_every_case_and_the_null_none(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    """eval-audit.md §2: a harness that cannot tell a right answer from none is broken."""
    assert run_eval(tmp_path / "oracle", oracle()) == runner.EXIT_OK
    shown = capsys.readouterr().out
    rows = lines(baseline(tmp_path / "oracle") / "results.jsonl")
    assert len(rows) == 6
    assert {row["grade"]["correct"] for row in rows} == {1.0}
    assert "fake/baseline on claude-opus-5: Correct 100.0% (95% CI 61.0% to 100.0%, 6 cases)" in (
        shown
    )

    assert run_eval(tmp_path / "null", FakeClient(lambda request: response(""))) == 0
    captured = capsys.readouterr()
    rows = lines(baseline(tmp_path / "null") / "results.jsonl")
    assert len(rows) == 6
    assert {row["grade"]["correct"] for row in rows} == {0.0}
    assert "Correct 0.0% (95% CI 0.0% to 39.0%, 6 cases)" in captured.out
    # A constant metric is said out loud, and does not fail an oracle or a null run.
    assert "warning: correct is the same on every scored row" in captured.err


def test_a_row_carries_every_field_the_report_and_the_spend_need(tmp_path: Path) -> None:
    snapshot = "claude-opus-5-20261101"
    assert run_eval(tmp_path, oracle(model=snapshot), "--limit", "1") == 0

    (row,) = lines(baseline(tmp_path) / "results.jsonl")
    usage = {
        "input_tokens": 120,
        "output_tokens": 8,
        "cache_read_input_tokens": 0,
        "cache_creation_input_tokens": 0,
    }
    expected_cost = llm_log.cost_usd(MODEL, usage)
    assert expected_cost is not None and expected_cost > 0
    assert row["prompt_id"] == "add"
    assert row["rep"] == 0
    assert row["prompt"] == "What is 2 + 3?"
    assert row["tags"] == ["sums", "arithmetic"]
    # The response's model, not the one asked for: a snapshot of it passes the check.
    assert row["model"] == snapshot
    assert row["usage"] == usage
    assert row["stop_reason"] == "end_turn"
    assert row["status"] == "ok"
    assert row["latency_s"] > 0
    assert row["tool_calls"] == 0
    assert row["grade"] == {"correct": 1.0, "chars": 1.0}
    assert row["explanation"] == {"correct": "expected 5"}
    assert row["meta"] == {"source": "hand-written", "cost_usd": expected_cost, "retries": 0}
    assert "judge_model" not in row


def test_the_trace_is_schemas_turn_format(tmp_path: Path) -> None:
    client = oracle(thinking="Two and three make five.")
    assert run_eval(tmp_path, client, "--limit", "1") == 0

    trace = baseline(tmp_path) / "traces" / "add_rep0.json"
    assert json.loads(trace.read_text(encoding="utf-8")) == [
        {"role": "system", "content": SYSTEM},
        {"role": "user", "content": "What is 2 + 3?"},
        {"role": "assistant", "content": "5", "thinking": "Two and three make five."},
    ]


def test_a_tool_loop_traces_each_call_and_result_as_its_own_turn() -> None:
    tool_use = SimpleNamespace(
        type="tool_use", id="t1", name="get_swing", input={"session": "s1", "swing": 2}
    )
    first = SimpleNamespace(
        model=MODEL,
        stop_reason="tool_use",
        content=[SimpleNamespace(type="thinking", thinking="Need the swing."), tool_use],
    )
    second = response("Your tempo is 3.1:1.")
    question_ = {"role": "user", "content": "How was swing 2?"}
    calls = [
        runner.Call(
            request={"system": [{"type": "text", "text": "Coach."}], "messages": [question_]},
            response=first,
        ),
        runner.Call(
            request={
                "messages": [
                    question_,
                    {"role": "assistant", "content": first.content},
                    {
                        "role": "user",
                        "content": [{"type": "tool_result", "tool_use_id": "t1", "content": "3.1"}],
                    },
                ]
            },
            response=second,
        ),
    ]

    assert runner._transcript(calls) == [
        {"role": "system", "content": "Coach."},
        {"role": "user", "content": "How was swing 2?"},
        {
            "role": "tool_call",
            "name": "get_swing",
            "content": '{\n  "session": "s1",\n  "swing": 2\n}',
            "thinking": "Need the swing.",
        },
        {"role": "tool_result", "content": "3.1"},
        {"role": "assistant", "content": "Your tempo is 3.1:1."},
    ]


def test_each_rep_gets_its_own_row_and_trace_and_limit_takes_the_first_cases(
    tmp_path: Path,
) -> None:
    assert run_eval(tmp_path, oracle(), "--reps", "2", "--limit", "2") == 0

    rows = lines(baseline(tmp_path) / "results.jsonl")
    assert sorted((row["prompt_id"], row["rep"]) for row in rows) == [
        ("add", 0),
        ("add", 1),
        ("subtract", 0),
        ("subtract", 1),
    ]
    traces = sorted(path.name for path in (baseline(tmp_path) / "traces").iterdir())
    assert traces == ["add_rep0.json", "add_rep1.json", "subtract_rep0.json", "subtract_rep1.json"]


def test_concurrency_is_bounded_by_the_flag(tmp_path: Path) -> None:
    client = FakeClient(lambda request: response(ANSWERS[question(request)]), delay=0.05)
    code = runner.main(
        ["--flow", "fake", "--model", MODEL, "--runs-dir", str(tmp_path), "--concurrency", "3"]
        + ["--approve-harness"],
        flows={"fake": Arithmetic()},
        client=client,
    )
    assert code == 0
    assert len(lines(baseline(tmp_path) / "results.jsonl")) == 6
    assert 2 <= client.peak <= 3


def test_the_runs_calls_are_tagged_with_its_origin_and_the_tag_is_put_back(
    tmp_path: Path,
) -> None:
    flow = Arithmetic()
    assert run_eval(tmp_path, oracle(), "--limit", "2", flow=flow) == 0
    assert flow.origins == ["eval:fake/baseline"] * 2
    assert llm_log.origin() == "app"


# --- failures are error lines, never zeros ---------------------------------------------------


@pytest.mark.parametrize(
    ("error", "failure_class"),
    [
        (RateLimitError("429 rate limited"), "serving_error"),
        (OverloadedError("529 overloaded"), "serving_error"),
        (APIConnectionError("connection reset"), "serving_error"),
        (APITimeoutError("timed out"), "serving_error"),
        (AuthenticationError("401 invalid x-api-key"), "harness_error"),
    ],
)
def test_an_induced_api_error_is_an_error_line_and_not_a_zero(
    tmp_path: Path,
    capsys: pytest.CaptureFixture[str],
    error: Exception,
    failure_class: str,
) -> None:
    """The route turned the error into a note; the runner reads the cause off the call.

    A serving error is sent again, five tries in all (P8), and still fails here, because the fake
    fails every time. Anything else is sent once.
    """
    client = failing_on("What is 2 + 3?", error)
    assert run_eval(tmp_path, client) == runner.EXIT_FAILED
    tries = runner.MAX_TRIES if failure_class == "serving_error" else 1
    assert [question(request) for request in client.requests].count("What is 2 + 3?") == tries

    rows = lines(baseline(tmp_path) / "results.jsonl")
    assert "add" not in {row["prompt_id"] for row in rows}
    assert len(rows) == 5
    (failed,) = lines(baseline(tmp_path) / "errors.jsonl")
    assert failed["prompt_id"] == "add"
    assert failed["rep"] == 0
    assert failed["failure_class"] == failure_class
    assert failed["error_class"] == type(error).__name__
    assert failed["retries"] == tries - 1
    assert failed["judge_retries"] == 0
    assert failed["latency_s"] > 0
    # The call never completed, so nothing was billed and nothing is carried.
    assert "usage" not in failed and "model" not in failed
    out = capsys.readouterr().out
    assert "Correct 100.0% (95% CI 56.6% to 100.0%, 5 cases)" in out
    assert f"1 failed attempts ({failure_class} 1), 1 (case, rep) unscored" in out
    assert f"{tries - 1} backoff retries" in out


def test_resume_skips_finished_rows_and_reruns_errored_ones(
    tmp_path: Path, capsys: pytest.CaptureFixture[str], monkeypatch: pytest.MonkeyPatch
) -> None:
    assert run_eval(tmp_path, failing_on("What is 2 + 3?", RateLimitError("429"))) == 1
    capsys.readouterr()

    again = oracle()
    assert run_eval(tmp_path, again) == 0
    assert [question(request) for request in again.requests] == ["What is 2 + 3?"]
    rows = lines(baseline(tmp_path) / "results.jsonl")
    keys = [(row["prompt_id"], row["rep"]) for row in rows]
    assert sorted(keys) == sorted((case_id, 0) for case_id in QUESTIONS)
    # Append-only: the old error line stays, and the row now on disk supersedes it.
    assert len(lines(baseline(tmp_path) / "errors.jsonl")) == 1
    assert "1 failed attempts (serving_error 1), 0 (case, rep) unscored" in capsys.readouterr().out

    # Nothing left to run, so no client is built at all.
    def no_sdk() -> Any:
        raise AssertionError("a finished run built a client")

    monkeypatch.setattr(runner, "_sdk", no_sdk)
    assert run_eval(tmp_path, None) == 0
    assert "0 of 6 (case, rep) to run" in capsys.readouterr().err


def test_a_torn_last_line_holds_no_slot_and_is_not_glued_to_the_next_row(tmp_path: Path) -> None:
    variant = baseline(tmp_path)
    variant.mkdir(parents=True)
    whole = {"prompt_id": "add", "rep": 0, "status": "ok", "grade": {"correct": 1.0, "chars": 1.0}}
    torn = '{"prompt_id": "subtract", "rep": 0, "sta'
    (variant / "results.jsonl").write_text(json.dumps(whole) + "\n" + torn, encoding="utf-8")

    client = oracle()
    assert run_eval(tmp_path, client) == 0
    assert "What is 2 + 3?" not in [question(request) for request in client.requests]
    assert len(client.requests) == 5
    text = (variant / "results.jsonl").read_text(encoding="utf-8").splitlines()
    assert text[1] == torn
    assert len(text) == 7
    assert all(json.loads(line)["prompt_id"] for line in text[2:])


def test_a_served_model_mismatch_is_an_error_line_carrying_what_was_billed(
    tmp_path: Path,
) -> None:
    assert run_eval(tmp_path, oracle(model="claude-sonnet-5-5"), "--limit", "2") == 1

    assert lines(baseline(tmp_path) / "results.jsonl") == []
    failed = lines(baseline(tmp_path) / "errors.jsonl")
    assert len(failed) == 2
    usage = {
        "input_tokens": 120,
        "output_tokens": 8,
        "cache_read_input_tokens": 0,
        "cache_creation_input_tokens": 0,
    }
    for line in failed:
        assert line["failure_class"] == "serving_substitution"
        assert line["model"] == "claude-sonnet-5-5"
        assert line["usage"] == usage
        # Billed as the model that answered: failed spend is still spend.
        assert line["cost_usd"] == llm_log.cost_usd("claude-sonnet-5-5", usage)


@pytest.mark.parametrize(
    ("requested", "served", "same"),
    [
        ("claude-opus-5", "claude-opus-5", True),
        ("claude-opus-5", "claude-opus-5-20261101", True),
        ("claude-opus-5", "claude-opus-5@20261101", True),
        ("claude-opus-5", "claude-opus-5-2026-11-01", True),
        ("claude-opus-5-latest", "claude-opus-5-20261101", True),
        ("claude-opus-5", "claude-opus-5-5", False),
        ("claude-opus-5", "claude-sonnet-5-5", False),
        ("claude-opus-5-20261101", "claude-opus-5-20261201", False),
        ("claude-opus-5-latest", "claude-opus-5", False),
    ],
)
def test_the_served_model_check_allows_only_a_dated_snapshot(
    requested: str, served: str, same: bool
) -> None:
    assert runner._same_model(requested, served) is same


@pytest.mark.parametrize(
    ("client_kwargs", "patch"),
    [
        ({"output_tokens": 0}, None),
        ({"input_tokens": 0}, None),
        ({"usage": False}, None),
        ({"model": ""}, None),
        ({}, "latency"),
        ({}, "cost"),
    ],
    ids=["no-output", "no-prompt", "no-usage", "no-model", "zero-latency", "zero-cost"],
)
def test_a_zero_on_an_answered_request_stops_the_run_loudly(
    tmp_path: Path,
    capsys: pytest.CaptureFixture[str],
    monkeypatch: pytest.MonkeyPatch,
    client_kwargs: dict[str, Any],
    patch: str | None,
) -> None:
    """build-eval.md, "Trusting a zero": raise, don't write the row."""
    if patch == "latency":
        monkeypatch.setattr(runner, "perf_counter", lambda: 100.0)
    if patch == "cost":
        monkeypatch.setattr(llm_log, "cost_usd", lambda model, usage: 0.0)
    client = oracle(**client_kwargs)

    assert run_eval(tmp_path, client) == runner.EXIT_STOPPED
    assert lines(baseline(tmp_path) / "results.jsonl") == []
    (failed,) = lines(baseline(tmp_path) / "errors.jsonl")
    assert failed["failure_class"] == "harness_error"
    # One worker, so the stop came before any second case started.
    assert len(client.requests) == 1
    assert "STOPPED" in capsys.readouterr().err


def test_a_truncated_answer_is_counted_but_never_graded_or_averaged(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    def answer(request: dict[str, Any]) -> Any:
        if question(request) == "What is 6 x 7?":
            return response("4", stop_reason="max_tokens")
        return response(ANSWERS[question(request)])

    flow = Arithmetic()
    assert run_eval(tmp_path, FakeClient(answer), flow=flow) == 0

    rows = {row["prompt_id"]: row for row in lines(baseline(tmp_path) / "results.jsonl")}
    assert rows["multiply"]["status"] == "truncated"
    assert rows["multiply"]["grade"] == {}
    assert rows["multiply"]["usage"]["output_tokens"] == 8
    assert "multiply" not in flow.graded
    out = capsys.readouterr().out
    assert "Correct 100.0% (95% CI 56.6% to 100.0%, 5 cases)" in out
    assert "5 scored and 1 truncated rows" in out


def test_a_refusal_is_a_graded_row_marked_as_one(tmp_path: Path) -> None:
    """eval-audit.md §2: a refusal is the model's answer, so it is scored, not dropped."""

    def answer(request: dict[str, Any]) -> Any:
        if question(request) == "What is 2 + 3?":
            return response("", stop_reason="refusal")
        return response(ANSWERS[question(request)])

    assert run_eval(tmp_path, FakeClient(answer)) == 0

    rows = {row["prompt_id"]: row for row in lines(baseline(tmp_path) / "results.jsonl")}
    assert rows["add"]["status"] == "ok"
    assert rows["add"]["stop_reason"] == "refusal"
    assert rows["add"]["meta"]["failure_class"] == "refusal"
    assert rows["add"]["grade"]["correct"] == 0.0
    assert "failure_class" not in rows["subtract"]["meta"]


class Unreadable(Arithmetic):
    """A flow that, like the lookup on unparseable JSON, has nothing to grade."""

    def run(self, case: Case, ctx: CaseContext) -> dict[str, str]:
        output = super().run(case, ctx)
        if case.id == "add":
            raise AttemptFailed("genuine_failure", "the answer did not parse")
        return output


def test_a_flow_that_cannot_score_says_why_and_keeps_the_spend(tmp_path: Path) -> None:
    assert run_eval(tmp_path, oracle(), flow=Unreadable()) == 1

    (failed,) = lines(baseline(tmp_path) / "errors.jsonl")
    assert failed["prompt_id"] == "add"
    assert failed["failure_class"] == "genuine_failure"
    assert failed["error"] == "the answer did not parse"
    assert failed["model"] == MODEL
    assert failed["usage"]["output_tokens"] == 8
    assert failed["cost_usd"] > 0


class BadGrades(Arithmetic):
    def __init__(self, grade: Callable[[], Grade]) -> None:
        super().__init__()
        self._grade = grade

    def grade(self, case: Case, output: dict[str, str], ctx: CaseContext) -> Grade:
        return self._grade()


def _raises() -> Grade:
    raise KeyError("answer")


@pytest.mark.parametrize(
    "grade",
    [
        lambda: Grade(scores={"correct": 1.0}),
        lambda: Grade(scores={"correct": 1.0, "chars": 1.0, "extra": 1.0}),
        lambda: Grade(scores={"correct": float("nan"), "chars": 1.0}),
        lambda: Grade(scores={"correct": 0.5, "chars": 1.0}),
        lambda: Grade(scores={"correct": 1.0, "chars": 1.0}, explanation={"tone": "?"}),
        _raises,
    ],
    ids=["missing", "extra", "nan", "half-binary", "stray-explanation", "grader-raised"],
)
def test_a_grader_that_crashes_or_mis_declares_is_a_harness_error(
    tmp_path: Path, grade: Callable[[], Grade]
) -> None:
    assert run_eval(tmp_path, oracle(), "--limit", "1", flow=BadGrades(grade)) == 1

    assert lines(baseline(tmp_path) / "results.jsonl") == []
    (failed,) = lines(baseline(tmp_path) / "errors.jsonl")
    assert failed["failure_class"] == "harness_error"
    assert failed["usage"]["input_tokens"] == 120


#: `chars` may be absent from a grade; `correct` may not. [M43 P12]
SPARSE = (
    Metric("correct", "Correct", "binary"),
    Metric("chars", "Answer length", "float", better="lower", scale=100, always=False),
)
NONE_REQUIRED = tuple(replace(metric, always=False) for metric in SPARSE)


class Sparse(Arithmetic):
    """Leaves `chars` out on the two products, as a share with nothing to divide by would be."""

    metrics = SPARSE

    def grade(self, case: Case, output: dict[str, str], ctx: CaseContext) -> Grade:
        full = super().grade(case, output, ctx)
        if case.tags[0] != "products":
            return full
        return Grade(scores={"correct": full.scores["correct"]}, explanation=full.explanation)


def test_a_metric_declared_always_false_may_be_left_out_and_is_averaged_where_held(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    """The club-spec flow's case: absent, never a stand-in, and each metric counts its own cases."""
    assert run_eval(tmp_path, oracle(), flow=Sparse()) == runner.EXIT_OK

    rows = {row["prompt_id"]: row for row in lines(baseline(tmp_path) / "results.jsonl")}
    assert len(rows) == 6
    assert lines(baseline(tmp_path) / "errors.jsonl") == []
    assert {case for case, row in rows.items() if "chars" not in row["grade"]} == {
        "multiply",
        "square",
    }
    shown = capsys.readouterr().out
    assert "Correct 100.0% (95% CI 61.0% to 100.0%, 6 cases)" in shown
    # Lengths 1, 1, 2 and 1 over the four cases holding one: mean 1.25, sd 0.5, half-width 0.49.
    assert "  also Answer length 1.250 (95% CI 0.760 to 1.740, 4 cases)" in shown


@pytest.mark.parametrize(
    ("metrics", "grade", "error"),
    [
        (SPARSE, Grade(scores={"chars": 1.0}), "of which only ['chars'] may be left out"),
        (SPARSE, Grade(scores={"correct": 1.0, "chars": float("inf")}), "not a number"),
        (
            SPARSE,
            Grade(scores={"correct": 1.0}, explanation={"chars": "1 character"}),
            "explains a metric it does not score",
        ),
        (NONE_REQUIRED, Grade(scores={}), "blank row"),
    ],
    ids=["required-left-out", "present-not-finite", "explains-an-absent-score", "blank-row"],
)
def test_only_a_metric_declared_always_false_may_be_left_out(
    tmp_path: Path, metrics: tuple[Metric, ...], grade: Grade, error: str
) -> None:
    flow = BadGrades(lambda: grade)
    flow.metrics = metrics
    assert run_eval(tmp_path, oracle(), "--limit", "1", flow=flow) == 1

    assert lines(baseline(tmp_path) / "results.jsonl") == []
    (failed,) = lines(baseline(tmp_path) / "errors.jsonl")
    assert failed["failure_class"] == "harness_error"
    assert error in failed["error"]


class Silent(Arithmetic):
    def run(self, case: Case, ctx: CaseContext) -> dict[str, str]:
        return {"text": case.input["answer"]}


def test_a_flow_that_made_no_request_measured_nothing(tmp_path: Path) -> None:
    client = oracle()
    assert run_eval(tmp_path, client, "--limit", "1", flow=Silent()) == 1
    (failed,) = lines(baseline(tmp_path) / "errors.jsonl")
    assert failed["failure_class"] == "harness_error"
    assert "made no request" in failed["error"]


# --- a grader that calls a model -------------------------------------------------------------


class Judged(Arithmetic):
    judge_model: str | None = JUDGE

    def __init__(self, *, refuse: bool = False) -> None:
        super().__init__()
        self._refuse = refuse

    def grade(self, case: Case, output: dict[str, str], ctx: CaseContext) -> Grade:
        verdict = ctx.judge_client.messages.create(
            model=JUDGE,
            max_tokens=32,
            messages=[{"role": "user", "content": f"Grade: {output.get('text', '')}"}],
        )
        if verdict.stop_reason == "refusal" or self._refuse:
            raise AttemptFailed("refusal", "the judge refused")
        return super().grade(case, output, ctx)


def app_and_judge(request: dict[str, Any]) -> Any:
    if request["model"] == JUDGE:
        return response("pass", model=JUDGE, input_tokens=300, output_tokens=20)
    return response(ANSWERS[question(request)])


def test_a_judges_spend_is_measured_apart_and_kept_out_of_the_trace(tmp_path: Path) -> None:
    assert run_eval(tmp_path, FakeClient(app_and_judge), "--limit", "1", flow=Judged()) == 0

    (row,) = lines(baseline(tmp_path) / "results.jsonl")
    judge_usage = {
        "input_tokens": 300,
        "output_tokens": 20,
        "cache_read_input_tokens": 0,
        "cache_creation_input_tokens": 0,
    }
    judge_cost = llm_log.cost_usd(JUDGE, judge_usage)
    app_cost = llm_log.cost_usd(MODEL, row["usage"])
    assert judge_cost is not None and app_cost is not None
    assert row["model"] == MODEL
    assert row["judge_model"] == JUDGE
    assert row["judge_usage"] == judge_usage
    assert row["meta"]["judge_cost_usd"] == judge_cost
    assert row["meta"]["cost_usd"] == round(app_cost + judge_cost, 10)
    trace = json.loads((baseline(tmp_path) / "traces" / "add_rep0.json").read_text("utf-8"))
    assert [turn["role"] for turn in trace] == ["system", "user", "assistant"]


def test_a_judge_refusal_is_an_error_line_with_both_spends(tmp_path: Path) -> None:
    """ADR-036 §8: no fallbacks on the judge, so its refusal is an error line, not a zero."""
    flow = Judged(refuse=True)
    assert run_eval(tmp_path, FakeClient(app_and_judge), "--limit", "1", flow=flow) == 1

    (failed,) = lines(baseline(tmp_path) / "errors.jsonl")
    assert failed["failure_class"] == "refusal"
    assert failed["model"] == MODEL
    assert failed["judge_model"] == JUDGE
    assert failed["judge_usage"]["output_tokens"] == 20
    assert failed["judge_cost_usd"] > 0


def test_a_grader_calling_a_model_it_did_not_declare_is_a_harness_error(tmp_path: Path) -> None:
    class Undeclared(Judged):
        judge_model = None

    assert run_eval(tmp_path, FakeClient(app_and_judge), "--limit", "1", flow=Undeclared()) == 1
    (failed,) = lines(baseline(tmp_path) / "errors.jsonl")
    assert failed["failure_class"] == "harness_error"
    assert "declares no judge_model" in failed["error"]


# --- refused before any request --------------------------------------------------------------


@pytest.mark.parametrize("variant", ["v1-better-prompt", "variant_a", "v0", "Baseline"])
def test_a_variant_is_named_baseline_or_v_n_exactly(tmp_path: Path, variant: str) -> None:
    """The report builder silently skips any other directory name (build-eval.md)."""
    with pytest.raises(SystemExit) as exited:
        run_eval(tmp_path, oracle(), "--variant", variant)
    assert exited.value.code == runner.EXIT_REFUSED


def test_a_v_n_run_needs_its_change_md(tmp_path: Path) -> None:
    client = oracle()
    assert run_eval(tmp_path, client, "--variant", "v1") == runner.EXIT_REFUSED
    assert client.requests == []
    assert not (tmp_path / "fake" / "v1").exists()

    (tmp_path / "fake" / "v1").mkdir(parents=True)
    (tmp_path / "fake" / "v1" / "change.md").write_text("# Shorter system prompt\n", "utf-8")
    flow = Arithmetic()
    assert run_eval(tmp_path, client, "--variant", "v1", "--limit", "1", flow=flow) == 0
    assert len(lines(tmp_path / "fake" / "v1" / "results.jsonl")) == 1
    assert flow.origins == ["eval:fake/v1"]


class Declared(Arithmetic):
    def __init__(self, *, metrics: Any = None, cases: Any = None) -> None:
        super().__init__()
        if metrics is not None:
            self.metrics = metrics
        self._cases = cases

    def load_cases(self) -> list[Case]:
        return self._cases if self._cases is not None else super().load_cases()


def _case(case_id: str = "one", **kwargs: Any) -> Case:
    fields: dict[str, Any] = {"prompt": "What is 2 + 3?", "tags": ("sums",), "input": {}}
    fields.update(kwargs)
    return Case(id=case_id, **fields)


@pytest.mark.parametrize(
    "flow",
    [
        Declared(metrics=()),
        Declared(metrics=(Metric("correct", "Correct answers", "binary"),)),
        Declared(metrics=(Metric("correct", "C", "binary"), Metric("correct", "D", "float"))),
        Declared(metrics=(Metric("Correct", "C", "binary"),)),
        Declared(cases=[]),
        Declared(cases=[_case("a/b")]),
        Declared(cases=[_case("Same"), _case("same")]),
        Declared(cases=[_case(tags=())]),
        Declared(cases=[_case(tags="sums")]),
        Declared(cases=[_case(prompt=" ")]),
        Declared(cases=[_case(meta={"when": object()})]),
    ],
    ids=[
        "no-metrics",
        "long-label",
        "duplicate-metric",
        "metric-id-case",
        "no-cases",
        "unsafe-id",
        "case-twins",
        "no-tags",
        "tags-a-string",
        "blank-prompt",
        "meta-not-json",
    ],
)
def test_a_bad_declaration_is_refused_before_any_request(tmp_path: Path, flow: Flow) -> None:
    client = oracle()
    assert run_eval(tmp_path, client, flow=flow) == runner.EXIT_REFUSED
    assert client.requests == []


def test_a_model_with_no_price_is_refused_before_any_request(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    client = oracle()
    code = runner.main(
        ["--flow", "fake", "--model", "claude-unknown-1", "--runs-dir", str(tmp_path)],
        flows={"fake": Arithmetic()},
        client=client,
    )
    assert code == runner.EXIT_REFUSED
    assert client.requests == []
    assert "no price in llm_prices.json" in capsys.readouterr().err


def test_an_unknown_flow_is_refused_from_the_command_line(tmp_path: Path) -> None:
    """The real `python -m` path: the flow is looked up as `golf_coach.evals.<flow>`."""
    done = subprocess.run(
        [
            sys.executable,
            "-m",
            "golf_coach.evals.runner",
            "--flow",
            "no_such_flow",
            "--model",
            MODEL,
            "--runs-dir",
            str(tmp_path),
        ],
        capture_output=True,
        text=True,
        cwd=REPO_ROOT,
        timeout=120,
    )
    assert done.returncode == runner.EXIT_REFUSED
    assert "there is no flow named 'no_such_flow'" in done.stderr
    assert not (tmp_path / "no_such_flow").exists()


def test_a_real_flow_is_found_by_name_from_the_command_line(tmp_path: Path) -> None:
    """The real `python -m` path finds `club_spec` by name, and stops at the harness gate.

    Run that way, the runner's file is `__main__`: a second copy of the module, whose `Flow` no
    flow derives from. The user's first club-spec pilot was refused here by name, before any request
    (M43 P13). The fake flows above are passed to `runner.main` directly, so none of them could see
    it. `test_club_spec_flow.py` runs the same entry through every case on a fake SDK.

    The state is the committed one with any approved sha dropped, so the gate refuses before a
    client is built. That holds after the user approves the real harness, and no request can leave.
    """
    committed = REPO_ROOT / ".claude" / "hillclimb" / "club_spec" / "_state.json"
    state = json.loads(committed.read_text(encoding="utf-8"))
    state.pop("harness_sha", None)
    flow_dir = tmp_path / "club_spec"
    flow_dir.mkdir()
    (flow_dir / "_state.json").write_text(json.dumps(state), encoding="utf-8")
    done = subprocess.run(
        [sys.executable, "-m", "golf_coach.evals.runner", "--flow", "club_spec"]
        + ["--model", MODEL, "--runs-dir", str(tmp_path)],
        capture_output=True,
        text=True,
        cwd=REPO_ROOT,
        timeout=120,
    )
    assert done.returncode == runner.EXIT_REFUSED
    assert "no flow named" not in done.stderr
    # The gate is after the flow, its metrics and their state check: so the flow was found by name.
    assert "no approved harness sha" in done.stderr
    assert [path.name for path in flow_dir.iterdir()] == ["_state.json"]


# --- the client ------------------------------------------------------------------------------


class FakeSdk:
    """`anthropic` as `_build_client` sees it: `Anthropic(**kwargs)` resolving `resolved`."""

    def __init__(self, resolved: dict[str, Any]) -> None:
        self.built: list[dict[str, Any]] = []
        self._resolved = resolved

    def Anthropic(self, **kwargs: Any) -> SimpleNamespace:  # noqa: N802 - the SDK's name
        self.built.append(kwargs)
        fields = {"api_key": None, "auth_token": None, "credentials": None}
        fields.update(self._resolved if "api_key" not in kwargs else {"api_key": kwargs["api_key"]})
        return SimpleNamespace(max_retries=kwargs["max_retries"], **fields)


@pytest.mark.parametrize(
    ("resolved", "source"),
    [
        ({"api_key": "sk-from-env"}, "ANTHROPIC_API_KEY"),
        ({"auth_token": "token"}, "ANTHROPIC_AUTH_TOKEN"),
        ({"credentials": object()}, "an ant auth profile"),
    ],
)
def test_the_client_takes_what_the_sdk_resolves_first(
    monkeypatch: pytest.MonkeyPatch, resolved: dict[str, Any], source: str
) -> None:
    monkeypatch.setattr(settings, "anthropic_api_key", SecretStr("sk-golf"))
    sdk = FakeSdk(resolved)
    client, found = runner._build_client(sdk)
    assert found == source
    # No retries of the SDK's own: every retry is the runner's, and visible.
    assert sdk.built == [{"max_retries": 0}]
    assert client.max_retries == 0


def test_the_client_falls_back_to_the_apps_key(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(settings, "anthropic_api_key", SecretStr("sk-golf"))
    sdk = FakeSdk({})
    client, found = runner._build_client(sdk)
    assert found == "GOLF_ANTHROPIC_API_KEY"
    assert sdk.built == [{"max_retries": 0}, {"api_key": "sk-golf", "max_retries": 0}]
    assert client.api_key == "sk-golf"


def test_no_credential_anywhere_is_refused_before_any_request(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    monkeypatch.setattr(settings, "anthropic_api_key", None)
    monkeypatch.setattr(runner, "_sdk", lambda: FakeSdk({}))
    assert run_eval(tmp_path, None) == runner.EXIT_REFUSED
    assert "no API credential" in capsys.readouterr().err
    assert lines(baseline(tmp_path) / "results.jsonl") == []


# --- the summary -----------------------------------------------------------------------------


def test_the_summary_averages_reps_within_a_case_then_cases(tmp_path: Path) -> None:
    """Worked by hand. Wilson, n = 4 cases, p = 0.75: centre 0.6275, half-width 0.3269."""
    metrics = (Metric("correct", "Correct", "binary"), Metric("chars", "Length", "float"))
    rows = [
        # case a: reps 1 and 1 -> 1.0; case b: 1 -> 1.0; case c: 1 and 0 -> 0.5; case d: 0.5 alone
        ("a", 0, 1.0, 1.0),
        ("a", 1, 1.0, 1.0),
        ("b", 0, 1.0, 2.0),
        ("c", 0, 1.0, 3.0),
        ("c", 1, 0.0, 3.0),
        ("d", 0, 0.0, 4.0),
        ("d", 1, 1.0, 4.0),
    ]
    variant = tmp_path / "flow" / "baseline"
    variant.mkdir(parents=True)
    with (variant / "results.jsonl").open("w", encoding="utf-8") as handle:
        for prompt_id, rep, correct, chars in rows:
            row = {
                "prompt_id": prompt_id,
                "rep": rep,
                "status": "ok",
                "grade": {"correct": correct, "chars": chars},
                "meta": {"cost_usd": 0.25},
            }
            handle.write(json.dumps(row) + "\n")
        truncated = {"prompt_id": "a", "rep": 2, "status": "truncated", "grade": {}, "meta": {}}
        handle.write(json.dumps(truncated | {"meta": {"cost_usd": 0.5}}) + "\n")
    errors = [
        {"prompt_id": "e", "rep": 0, "failure_class": "serving_error"},
        {"prompt_id": "c", "rep": 1, "failure_class": "serving_error"},
        {
            "prompt_id": "f",
            "rep": 0,
            "failure_class": "serving_substitution",
            "usage": {"input_tokens": 1},
            "cost_usd": 0.125,
        },
    ]
    (variant / "errors.jsonl").write_text(
        "".join(json.dumps(line) + "\n" for line in errors), encoding="utf-8"
    )

    summary = runner.summarize(variant, metrics)

    correct, chars = summary.metrics
    # Per case: a 1.0, b 1.0, c 0.5, d 0.5 -> 0.75 over four cases.
    assert correct.mean == pytest.approx(0.75)
    assert correct.low == pytest.approx(0.3006, abs=1e-4)
    assert correct.high == pytest.approx(0.9544, abs=1e-4)
    assert correct.cases == 4
    # Per case: 1, 2, 3, 4 -> 2.5, sd sqrt(5/3), half-width 1.96 * 1.2910 / 2 = 1.2652.
    assert chars.mean == pytest.approx(2.5)
    assert chars.low == pytest.approx(1.2348, abs=1e-4)
    assert chars.high == pytest.approx(3.7652, abs=1e-4)
    assert summary.ok == 7
    assert summary.truncated == 1
    assert summary.failed_attempts == 3
    assert summary.failures_by_class == {"serving_error": 2, "serving_substitution": 1}
    # c rep 1 has a row now, so only e and f are still unscored.
    assert summary.unscored == 2
    # Seven rows at $0.25, the truncated one at $0.50, and the billed error line at $0.125.
    assert summary.cost_usd == pytest.approx(7 * 0.25 + 0.5 + 0.125)
    assert summary.unpriced == 0
    assert summary.constant == []

    shown = runner.render_summary("flow/baseline", MODEL, summary)
    assert (
        shown[0] == "flow/baseline on claude-opus-5: Correct 75.0% (95% CI 30.1% to 95.4%, 4 cases)"
    )
    assert shown[1] == "  also Length 2.500 (95% CI 1.235 to 3.765, 4 cases)"


def test_the_headline_is_the_first_binary_metric() -> None:
    metrics = (
        Metric("length", "Length", "float"),
        Metric("correct", "Correct", "binary"),
        Metric("sharp", "Sharp", "binary"),
    )
    assert runner.headline(metrics).id == "correct"
    assert runner.headline(metrics[:1]).id == "length"


# --- backoff [M43 P8] ------------------------------------------------------------------------


def flaky(failures: dict[str, list[Exception]]) -> FakeClient:
    """Raises each listed error in turn for its question, then answers it."""

    def answer(request: dict[str, Any]) -> Any:
        waiting = failures.get(question(request))
        if waiting:
            raise waiting.pop(0)
        return response(ANSWERS[question(request)])

    return FakeClient(answer)


def test_two_transient_notes_then_an_answer_are_an_ok_row_with_two_retries(
    tmp_path: Path,
    capsys: pytest.CaptureFixture[str],
    monkeypatch: pytest.MonkeyPatch,
    backoffs: list[int],
) -> None:
    """build-eval.md Step 3: back off with jitter, and keep the retry count with the attempt."""
    real_delay = runner._backoff_delay

    def a_tenth(retry: int) -> float:
        real_delay(retry)
        return 0.1

    monkeypatch.setattr(runner, "_backoff_delay", a_tenth)
    client = flaky({"What is 2 + 3?": [RateLimitError("429"), OverloadedError("529")]})
    assert run_eval(tmp_path, client, "--limit", "2") == runner.EXIT_OK

    rows = {row["prompt_id"]: row for row in lines(baseline(tmp_path) / "results.jsonl")}
    assert [question(request) for request in client.requests].count("What is 2 + 3?") == 3
    assert backoffs == [0, 1]
    assert rows["add"]["status"] == "ok"
    assert rows["add"]["grade"]["correct"] == 1.0
    assert rows["add"]["meta"]["retries"] == 2
    assert rows["subtract"]["meta"]["retries"] == 0
    # Strict per attempt: the usage, the latency and the trace are the answered try's alone.
    assert rows["add"]["usage"]["input_tokens"] == 120
    assert rows["add"]["latency_s"] < 0.1
    trace = json.loads((baseline(tmp_path) / "traces" / "add_rep0.json").read_text("utf-8"))
    assert [turn["role"] for turn in trace] == ["system", "user", "assistant"]
    assert lines(baseline(tmp_path) / "errors.jsonl") == []
    assert "0 failed attempts, 0 (case, rep) unscored; 2 backoff retries" in capsys.readouterr().out


def test_the_backoff_doubles_from_a_second_to_a_minute_with_jitter(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    """The scaffold's: `min(60 s, 2**k s)` before retry k, times a jitter of 0.5 to 1.5."""
    monkeypatch.setattr(runner.random, "random", lambda: 0.5)
    assert [REAL_BACKOFF(retry) for retry in range(8)] == [1, 2, 4, 8, 16, 32, 60, 60]
    monkeypatch.setattr(runner.random, "random", lambda: 0.0)
    assert REAL_BACKOFF(3) == 4.0
    monkeypatch.setattr(runner.random, "random", lambda: 0.999)
    assert REAL_BACKOFF(3) == pytest.approx(8 * 1.499)


class TwoRequests(Arithmetic):
    """A flow that asks twice, so its second request can fail after the first was billed."""

    def run(self, case: Case, ctx: CaseContext) -> dict[str, str]:
        output = super().run(case, ctx)
        try:
            ctx.client.messages.create(
                model=ctx.model,
                max_tokens=64,
                system=SYSTEM,
                messages=[{"role": "user", "content": f"Check: {case.prompt}"}],
            )
        except Exception as exc:
            return {"note": f"Unavailable ({type(exc).__name__})."}
        return output


def test_a_try_with_an_answered_request_is_never_sent_again(tmp_path: Path) -> None:
    """A retry would bill the answered request twice. The spend stays on the error line."""

    def answer(request: dict[str, Any]) -> Any:
        asked = question(request)
        if asked == "Check: What is 2 + 3?":
            raise RateLimitError("429")
        return response(ANSWERS.get(asked, "ok"))

    client = FakeClient(answer)
    assert run_eval(tmp_path, client, "--limit", "1", flow=TwoRequests()) == runner.EXIT_FAILED

    assert len(client.requests) == 2
    (failed,) = lines(baseline(tmp_path) / "errors.jsonl")
    assert failed["failure_class"] == "serving_error"
    assert failed["retries"] == 0
    assert failed["usage"]["input_tokens"] == 120
    assert failed["cost_usd"] > 0


def test_the_judge_backs_off_on_its_own_and_never_re_sends_the_app(tmp_path: Path) -> None:
    limited = [RateLimitError("429")]

    def answer(request: dict[str, Any]) -> Any:
        if request["model"] == JUDGE and limited:
            raise limited.pop()
        return app_and_judge(request)

    client = FakeClient(answer)
    assert run_eval(tmp_path, client, "--limit", "1", flow=Judged()) == runner.EXIT_OK

    assert [request["model"] for request in client.requests] == [MODEL, JUDGE, JUDGE]
    (row,) = lines(baseline(tmp_path) / "results.jsonl")
    assert row["meta"]["retries"] == 0
    assert row["meta"]["judge_retries"] == 1
    assert row["judge_usage"]["input_tokens"] == 300


# --- the ceiling [M43 P8] --------------------------------------------------------------------


def test_a_slow_case_becomes_a_timeout_line_and_gives_up_its_slot(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    """build-eval.md Step 3: a hard ceiling on the whole case, recorded as a timeout, not a zero."""
    release = threading.Event()

    def answer(request: dict[str, Any]) -> Any:
        if request["model"] == MODEL and question(request) == "What is 2 + 3?":
            release.wait(10)  # a connection that never answers, until the test lets it go
        return app_and_judge(request)

    client = FakeClient(answer)
    started = time.perf_counter()
    try:
        code = run_eval(tmp_path, client, "--timeout-s", "0.3", flow=Judged())
        elapsed = time.perf_counter() - started
    finally:
        release.set()

    assert code == runner.EXIT_FAILED
    # One worker, and the hung case did not keep it: the other five ran in its place.
    assert elapsed < 5
    scored = sorted(row["prompt_id"] for row in lines(baseline(tmp_path) / "results.jsonl"))
    assert scored == sorted(set(QUESTIONS) - {"add"})
    (failed,) = lines(baseline(tmp_path) / "errors.jsonl")
    assert failed["prompt_id"] == "add"
    assert failed["failure_class"] == "timeout"
    assert "0.3 s wall-clock ceiling" in failed["error"]
    assert failed["latency_s"] >= 0.3
    assert "usage" not in failed  # nothing had answered, so nothing was billed
    assert "1 failed attempts (timeout 1), 1 (case, rep) unscored" in capsys.readouterr().out

    # Let go, the abandoned attempt sends nothing more (no judge request) and writes nothing.
    for thread in threading.enumerate():
        if thread.name == "eval-add-rep0":
            thread.join(10)
            assert not thread.is_alive()
    assert sum(1 for request in client.requests if request["model"] == JUDGE) == 5
    assert len(lines(baseline(tmp_path) / "results.jsonl")) == 5
    assert len(lines(baseline(tmp_path) / "errors.jsonl")) == 1


def test_a_retry_whose_backoff_would_end_past_the_ceiling_is_not_sent(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    monkeypatch.setattr(runner, "_backoff_delay", lambda retry: 30.0)
    client = failing_on("What is 2 + 3?", RateLimitError("429"))
    started = time.perf_counter()
    code = run_eval(tmp_path, client, "--limit", "1", "--timeout-s", "5")

    assert code == runner.EXIT_FAILED
    assert time.perf_counter() - started < 5
    assert len(client.requests) == 1
    (failed,) = lines(baseline(tmp_path) / "errors.jsonl")
    # What the attempt met, not a timeout: the ceiling never passed.
    assert failed["failure_class"] == "serving_error"
    assert failed["retries"] == 0


def test_no_ceiling_is_zero_seconds(tmp_path: Path) -> None:
    assert run_eval(tmp_path, oracle(), "--limit", "1", "--timeout-s", "0") == runner.EXIT_OK
    assert len(lines(baseline(tmp_path) / "results.jsonl")) == 1


@pytest.mark.parametrize("value", ["-1", "nan", "inf", "soon", "1e15"])
def test_the_ceiling_is_zero_or_more_seconds(tmp_path: Path, value: str) -> None:
    with pytest.raises(SystemExit) as exited:
        run_eval(tmp_path, oracle(), "--timeout-s", value)
    assert exited.value.code == runner.EXIT_REFUSED


# --- the harness gate [M43 P8] ---------------------------------------------------------------


def state_file(runs_dir: Path) -> Path:
    return runs_dir / "fake" / "_state.json"


def test_a_first_run_is_refused_until_the_harness_is_approved(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    """ADR-036 §6: exit 2 with nothing spent, until the user passes --approve-harness."""
    client = oracle()
    assert run_eval(tmp_path, client, "--limit", "1", approve=False) == runner.EXIT_REFUSED
    assert client.requests == []
    err = capsys.readouterr().err
    assert "no approved harness sha" in err
    assert "src/golf_coach/evals/runner.py, tests/evals/test_runner.py" in err
    assert "the user's, never an agent's" in err
    assert not (tmp_path / "fake").exists()

    assert run_eval(tmp_path, client, "--limit", "1") == runner.EXIT_OK
    state = json.loads(state_file(tmp_path).read_text(encoding="utf-8"))
    assert set(state) == {"harness_sha"}
    assert re.fullmatch(r"[0-9a-f]{64}", state["harness_sha"])
    assert "harness approved: sha256" in capsys.readouterr().err

    # Approved once, it runs without the flag.
    assert run_eval(tmp_path, client, "--limit", "2", approve=False) == runner.EXIT_OK
    assert len(client.requests) == 2


def test_a_changed_harness_file_exits_2_until_approved_again(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    grader = tmp_path / "grader.py"
    grader.write_text("THRESHOLD = 1\n", encoding="utf-8")
    runs = tmp_path / "runs"
    state_file(runs).parent.mkdir(parents=True)
    metrics = [
        {"id": "correct", "kind": "binary", "label": "Right answer"},
        {"id": "chars", "kind": "float", "better": "lower"},
    ]
    state_file(runs).write_text(
        json.dumps({"harness_paths": [str(grader)], "metrics": metrics}), encoding="utf-8"
    )
    client = oracle()
    assert run_eval(runs, client, "--limit", "1") == runner.EXIT_OK
    approved = json.loads(state_file(runs).read_text(encoding="utf-8"))["harness_sha"]

    grader.write_text("THRESHOLD = 2\n", encoding="utf-8")
    assert run_eval(runs, client, "--limit", "2", approve=False) == runner.EXIT_REFUSED
    assert len(client.requests) == 1
    assert "the harness changed since it was last approved" in capsys.readouterr().err

    assert run_eval(runs, client, "--limit", "2") == runner.EXIT_OK
    assert len(client.requests) == 2
    state = json.loads(state_file(runs).read_text(encoding="utf-8"))
    assert state["harness_sha"] != approved
    # The approval writes the sha and nothing else: the loop's own keys are kept as they were.
    assert state["harness_paths"] == [str(grader)]
    assert state["metrics"] == metrics


def test_the_sha_is_the_scaffolds_rule_over_the_runner_the_flow_and_the_listed_paths(
    tmp_path: Path,
) -> None:
    """runner-scaffold.mjs: sha256 of name, NUL, bytes, NUL per file, in name order."""
    listed = tmp_path / "grader.py"
    listed.write_bytes(b"def grade(): ...\n")
    missing = tmp_path / "gone.py"
    digest = runner.harness_digest({"harness_paths": [str(listed), str(missing)]}, Arithmetic())

    files = {Path(runner.__file__).resolve(), Path(__file__).resolve(), listed.resolve()}
    files |= {(REPO_ROOT / name).resolve() for name in digest.lockfiles}
    named = sorted((runner._harness_name(path), path) for path in files)
    expected = hashlib.sha256()
    for name, path in named:
        expected.update(name.encode("utf-8") + b"\0" + path.read_bytes() + b"\0")
    assert digest.sha == expected.hexdigest()
    assert digest.hashed == [name for name, _ in named]
    assert {"src/golf_coach/evals/runner.py", "tests/evals/test_runner.py"} <= set(digest.hashed)
    # A listed path that cannot be read is said and skipped, as the scaffold does.
    assert digest.skipped == [runner._harness_name(missing.resolve())]


@pytest.mark.parametrize(
    "text",
    [
        "{not json",
        "[1, 2]",
        '{"harness_paths": "grader.py"}',
        '{"harness_paths": [3]}',
        '{"metrics": "correct"}',
        '{"metrics": ["correct"]}',
        '{"metrics": ["chars", "correct"]}',
        '{"metrics": [{"id": "correct", "kind": "float"}, {"id": "chars"}]}',
    ],
    ids=[
        "not-json",
        "not-an-object",
        "paths-a-string",
        "path-a-number",
        "metrics-a-string",
        "a-metric-missing",
        "metrics-out-of-order",
        "a-kind-differs",
    ],
)
def test_a_state_file_that_would_mislead_is_refused_even_with_approval(
    tmp_path: Path, text: str
) -> None:
    """A wrong `metrics` list draws blank columns and the wrong headline in the report."""
    state_file(tmp_path).parent.mkdir(parents=True)
    state_file(tmp_path).write_text(text, encoding="utf-8")
    client = oracle()
    assert run_eval(tmp_path, client) == runner.EXIT_REFUSED
    assert client.requests == []
    assert state_file(tmp_path).read_text(encoding="utf-8") == text
