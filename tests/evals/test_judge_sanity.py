"""The coach judge's sanity checks, offline. [M43 P19]

What these tests hold, with fake clients and no request made:

- the three probes are the plan's: an empty paragraph, "I don't know", and a confident paragraph
  about another case's swing, whose numbers the code check finds in that case's brief and not in
  the brief it is judged against;
- `main` judges each probe `--reps` times through the judge's own request, one row per answer, and
  exits 0 only when every row failed, 1 when the judge passed one;
- a refused answer and an SDK error are error lines that hold no slot, so a rerun pays for exactly
  what is missing and nothing already on disk;
- an answered request with a zero measure stops the run (a zero is a bug until proven otherwise);
- `read_inputs` reads `CASE`'s approved brief and the route's instructions through `golf-mcp`, and
  refuses before any request when the brief served is not the approved one.
"""

from __future__ import annotations

import json
from pathlib import Path
from types import SimpleNamespace
from typing import Any

import pytest

from golf_coach.evals import judge_sanity, runner
from golf_coach.evals.coach_feedback import (
    BRIEFS_FILE,
    CASES_DIR,
    approved_briefs,
    check_numbers,
    unmeasured_terms,
)
from golf_coach.evals.coach_judge import CRITERIA, JUDGE_MODEL, judge_request
from golf_coach.evals.judge_sanity import (
    CASE,
    OTHER,
    OTHER_SWING_PARAGRAPH,
    PROBES,
    Inputs,
    Refused,
    main,
    read_inputs,
)
from golf_coach.feedback.coach import Brief

INPUTS = Inputs(instructions="THE COACH'S INSTRUCTIONS", brief="SWING 1 (session 2026-07-01)")
KEYS = [criterion.key for criterion in CRITERIA]


def _verdict(*passed: str) -> str:
    return json.dumps(
        {key: {"explanation": f"{key}: read", "passed": key in passed} for key in KEYS}
    )


def _response(
    text: str | None, *, stop_reason: str = "end_turn", output_tokens: int = 900
) -> SimpleNamespace:
    content = [SimpleNamespace(type="text", text=text)] if text is not None else []
    return SimpleNamespace(
        model=JUDGE_MODEL,
        stop_reason=stop_reason,
        stop_details=None,
        content=content,
        usage=SimpleNamespace(
            input_tokens=1800,
            output_tokens=output_tokens,
            cache_read_input_tokens=3000,
            cache_creation_input_tokens=0,
        ),
        _request_id="req_fake",
    )


class _Client:
    """A fake SDK client: each request kept, and answered by `answer(request)`."""

    def __init__(self, answer: Any) -> None:
        self.messages = self
        self.requests: list[dict[str, Any]] = []
        self._answer = answer

    def create(self, **request: Any) -> Any:
        self.requests.append(request)
        result = self._answer(request)
        if isinstance(result, BaseException):
            raise result
        return result


def _paragraph(request: dict[str, Any]) -> str:
    return str(request["messages"][0]["content"][1]["text"])


def _failing(request: dict[str, Any]) -> Any:
    return _response(_verdict())


def _rows(out: Path, name: str) -> list[dict[str, Any]]:
    path = out / name
    if not path.is_file():
        return []
    return [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines()]


@pytest.fixture
def offline(monkeypatch: pytest.MonkeyPatch) -> None:
    """`read_inputs` without `golf-mcp`: the inputs are the test's."""
    monkeypatch.setattr(judge_sanity, "read_inputs", lambda *args, **kwargs: INPUTS)


# --------------------------------------------------------------------------- the probes


def test_the_probes_are_an_empty_paragraph_i_dont_know_and_another_swing() -> None:
    assert [probe.id for probe in PROBES] == ["empty", "dont_know", "other_swing"]
    assert PROBES[0].paragraph == ""
    assert PROBES[1].paragraph == "I don't know."
    assert PROBES[2].paragraph == OTHER_SWING_PARAGRAPH


def test_the_other_swing_paragraph_quotes_the_other_case_and_not_this_one() -> None:
    """Every number in it is the other swing's, and some are nowhere in `CASE`'s brief."""
    briefs = approved_briefs((CASES_DIR / BRIEFS_FILE).read_text(encoding="utf-8"))
    assert check_numbers(OTHER_SWING_PARAGRAPH, briefs[OTHER]).passed
    against_case = check_numbers(OTHER_SWING_PARAGRAPH, briefs[CASE])
    assert not against_case.passed
    assert "228" in against_case.explanation and "143" in against_case.explanation


# --------------------------------------------------------------------------- the run


@pytest.mark.usefixtures("offline")
def test_each_probe_is_judged_twice_and_every_failure_exits_0(tmp_path: Path) -> None:
    client = _Client(_failing)
    assert main(["--out", str(tmp_path)], client=client) == 0

    # Probe by probe, one request at a time, each exactly the judge's own request.
    assert all(
        request
        == judge_request(
            instructions=INPUTS.instructions,
            brief=INPUTS.brief,
            paragraph=PROBES[index // 2].paragraph,
        )
        for index, request in enumerate(client.requests)
    )
    rows = _rows(tmp_path, judge_sanity.RESULTS)
    assert [(row["prompt_id"], row["rep"]) for row in rows] == [
        (probe.id, rep) for probe in PROBES for rep in range(2)
    ]
    for row in rows:
        assert row["case_id"] == CASE
        assert row["failed"] and row["failed_every_criterion"]
        assert row["judge_model"] == JUDGE_MODEL
        assert row["judge_usage"]["output_tokens"] == 900
        # 1,800 in at $4/M, 3,000 read from the cache at $0.20/M and 900 out at $20/M.
        assert row["cost_usd"] == pytest.approx(0.0072 + 0.0006 + 0.018)
        assert row["latency_s"] > 0
        assert row["request_id"] == "req_fake"
    assert not (tmp_path / judge_sanity.ERRORS).exists()


@pytest.mark.usefixtures("offline")
def test_a_rerun_pays_for_nothing_already_on_disk(tmp_path: Path) -> None:
    assert main(["--out", str(tmp_path), "--reps", "1"], client=_Client(_failing)) == 0
    again = _Client(_failing)
    assert main(["--out", str(tmp_path), "--reps", "2"], client=again) == 0
    assert [_paragraph(request).count("I don't know.") for request in again.requests] == [0, 1, 0]
    assert len(_rows(tmp_path, judge_sanity.RESULTS)) == 6


@pytest.mark.usefixtures("offline")
def test_a_probe_the_judge_passes_exits_1(tmp_path: Path) -> None:
    def answer(request: dict[str, Any]) -> Any:
        if "228 yards" in _paragraph(request):
            return _response(_verdict(*KEYS))
        return _response(_verdict())

    assert main(["--out", str(tmp_path)], client=_Client(answer)) == 1
    passed = [row for row in _rows(tmp_path, judge_sanity.RESULTS) if not row["failed"]]
    assert [row["prompt_id"] for row in passed] == ["other_swing", "other_swing"]


@pytest.mark.usefixtures("offline")
def test_one_criterion_passed_fails_the_probe_but_not_every_criterion(tmp_path: Path) -> None:
    """The plan's Done when is `judge_all` 0; P18's second choice asks for every criterion too."""

    def answer(request: dict[str, Any]) -> Any:
        if "228 yards" in _paragraph(request):
            return _response(_verdict("actionable"))
        return _response(_verdict())

    assert main(["--out", str(tmp_path)], client=_Client(answer)) == 0
    rows = _rows(tmp_path, judge_sanity.RESULTS)
    assert all(row["failed"] for row in rows)
    assert [row["prompt_id"] for row in rows if not row["failed_every_criterion"]] == [
        "other_swing",
        "other_swing",
    ]


@pytest.mark.usefixtures("offline")
def test_a_refusal_is_an_error_line_holding_no_slot_and_a_rerun_fills_it(tmp_path: Path) -> None:
    refused = {"first": True}

    def answer(request: dict[str, Any]) -> Any:
        if refused.pop("first", False):
            return _response(None, stop_reason="refusal", output_tokens=40)
        return _response(_verdict())

    assert main(["--out", str(tmp_path)], client=_Client(answer)) == 2
    (line,) = _rows(tmp_path, judge_sanity.ERRORS)
    assert (line["prompt_id"], line["rep"], line["failure_class"]) == ("empty", 0, "refusal")
    # Billed but failed is still spend (build-eval.md): the line carries what it cost.
    assert line["cost_usd"] > 0 and line["judge_usage"]["output_tokens"] == 40
    assert len(_rows(tmp_path, judge_sanity.RESULTS)) == 5

    again = _Client(_failing)
    assert main(["--out", str(tmp_path)], client=again) == 0
    assert [_paragraph(request) for request in again.requests] == [
        _paragraph(
            judge_request(instructions=INPUTS.instructions, brief=INPUTS.brief, paragraph="")
        )
    ]


@pytest.mark.usefixtures("offline")
def test_an_sdk_error_is_an_error_line_named_by_its_class(tmp_path: Path) -> None:
    class OverloadedError(Exception):
        status_code = 529

    def answer(request: dict[str, Any]) -> Any:
        return OverloadedError("overloaded")

    assert main(["--out", str(tmp_path), "--reps", "1"], client=_Client(answer)) == 2
    lines = _rows(tmp_path, judge_sanity.ERRORS)
    assert [line["failure_class"] for line in lines] == ["serving_error"] * 3
    assert all(line["error_class"] == "OverloadedError" for line in lines)
    assert all("cost_usd" not in line for line in lines)


@pytest.mark.usefixtures("offline")
def test_an_answered_request_with_no_output_tokens_stops_the_run(tmp_path: Path) -> None:
    def answer(request: dict[str, Any]) -> Any:
        return _response(_verdict(), output_tokens=0)

    with pytest.raises(runner._Unmeasured, match="zero prompt or output tokens"):
        main(["--out", str(tmp_path)], client=_Client(answer))
    assert _rows(tmp_path, judge_sanity.RESULTS) == []


@pytest.mark.usefixtures("offline")
def test_a_dry_run_makes_no_request_and_writes_nothing(tmp_path: Path) -> None:
    def answer(request: dict[str, Any]) -> Any:
        raise AssertionError("a dry run made a request")

    out = tmp_path / "runs"
    assert main(["--out", str(out), "--dry-run"], client=_Client(answer)) == 0
    assert not out.exists()


# --------------------------------------------------------------------------- the inputs


def test_the_inputs_are_the_approved_brief_and_the_routes_instructions(golf_mcp: Path) -> None:
    inputs = read_inputs(golf_mcp=golf_mcp)
    briefs = approved_briefs((CASES_DIR / BRIEFS_FILE).read_text(encoding="utf-8"))
    assert inputs.brief == briefs[CASE]
    # The route's prompt, as P17's checks read it: the not-measured list is there.
    assert unmeasured_terms(inputs.instructions)


def test_a_brief_that_is_not_the_approved_one_is_refused_before_any_request(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    caveats = {"TWO_AXES": "two axes.", "READING_THIS_DATA_HONESTLY": "read honestly."}
    monkeypatch.setattr(
        judge_sanity,
        "brief_through_golf_mcp",
        lambda *args, **kwargs: Brief(text="SWING 1, rendered differently", caveats=caveats),
    )
    with pytest.raises(Refused, match="not the one approved"):
        read_inputs()


def test_no_brief_from_golf_mcp_is_refused_with_its_reason(tmp_path: Path) -> None:
    assert main(["--golf-mcp", str(tmp_path / "golf-mcp.exe"), "--out", str(tmp_path)]) == 2
    with pytest.raises(Refused, match="not built"):
        read_inputs(golf_mcp=tmp_path / "golf-mcp.exe")
