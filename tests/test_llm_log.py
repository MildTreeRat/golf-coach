"""The call log: one line per request, priced from measured usage, and never in the route's way.

ADR-036 §2–§4. Each pin guards a failure nothing else would notice:

- a cost that is quietly wrong (a cache read at the input rate, Haiku 5.5's long-prompt tier
  missed, the requested model priced when another one answered) still sums to a believable total,
  so the rates are pinned here **by hand, from the pricing page as read on 2026-11-09**, and never
  read back out of `llm_prices.json`, which would make each test agree with itself;
- an unknown model priced at zero is a free call in every summary. It must be `null`, with one
  warning;
- a log write that raises would turn a coaching paragraph into a 500, which is the one thing an
  observer must never do;
- an eval's calls logged as `app` would bill an experiment to real use;
- `import anthropic` here would break the `vision`-only install `feedback/coach.py` promises;
- a summary that sums an unpriced call as zero, counts a served `truncated` paragraph as a failure
  (or a lookup's `truncated` as served), or averages a 429's latency into the model's, reads as a
  believable number and is wrong. So the summary is checked against a fixture log written out by
  hand, line by line, with every number in the test worked by hand from it (M43 P6).
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
from datetime import UTC, datetime
from pathlib import Path
from types import SimpleNamespace
from typing import Any
from unittest.mock import MagicMock

import pytest

from golf_coach import llm_log
from golf_coach.config import settings

#: The real `_log_path`, taken at import, before `tests/conftest.py::_throwaway_llm_log` points it
#: at a throwaway file for each test (M43 P3). The default-path test puts it back.
_REAL_LOG_PATH = llm_log._log_path

LINE_KEYS = {
    "ts",
    "origin",
    "route",
    "requested_model",
    "served_model",
    "usage",
    "cost_usd",
    "latency_s",
    "stop_reason",
    "outcome",
    "tool_calls",
    "turn_id",
    "iteration",
    "error_class",
    "request_id",
}


@pytest.fixture(autouse=True)
def _fresh_process_state(monkeypatch: pytest.MonkeyPatch) -> None:
    """Origin and the once-per-model warning are process-wide, so each test starts from none."""
    monkeypatch.setattr(llm_log, "_origin", llm_log.DEFAULT_ORIGIN)
    monkeypatch.setattr(llm_log, "_warned_unpriced", set())


def _usage(
    input_tokens: int,
    output_tokens: int,
    cache_read: int | None = None,
    cache_write: int | None = None,
    cache_creation: Any = None,
) -> SimpleNamespace:
    return SimpleNamespace(
        input_tokens=input_tokens,
        output_tokens=output_tokens,
        cache_read_input_tokens=cache_read,
        cache_creation_input_tokens=cache_write,
        cache_creation=cache_creation,
    )


def _response(
    model: str | None,
    usage: Any,
    *,
    stop_reason: str = "end_turn",
    content: list[Any] | None = None,
    request_id: str | None = "req_01",
) -> SimpleNamespace:
    return SimpleNamespace(
        model=model,
        usage=usage,
        stop_reason=stop_reason,
        content=content if content is not None else [SimpleNamespace(type="text", text="hi")],
        _request_id=request_id,
    )


def _lines(path: Path) -> list[dict[str, Any]]:
    return [json.loads(raw) for raw in path.read_text(encoding="utf-8").splitlines()]


def _one(path: Path) -> dict[str, Any]:
    lines = _lines(path)
    assert len(lines) == 1
    return lines[0]


# --- cost --------------------------------------------------------------------------------------

# (model, input, output, cache read, 5-minute cache write), USD per million tokens, copied by hand
# from https://platform.claude.com/docs/en/about-claude/pricing on 2026-11-09.
PAGE_RATES = [
    ("claude-opus-5", 5.0, 25.0, 0.50, 6.25),
    ("claude-opus-5-5", 4.0, 20.0, 0.20, 5.0),
    ("claude-sonnet-5-5", 2.0, 10.0, 0.10, 2.5),
    ("claude-haiku-5-5", 0.10, 0.50, 0.01, 0.125),
]


@pytest.mark.parametrize(("model", "rate_in", "rate_out", "rate_read", "rate_write"), PAGE_RATES)
def test_each_model_is_priced_with_its_cache_reads_and_writes_at_their_own_rates(
    tmp_path: Path, model: str, rate_in: float, rate_out: float, rate_read: float, rate_write: float
) -> None:
    log = tmp_path / "llm_calls.jsonl"
    usage = _usage(2_000, 500, cache_read=10_000, cache_write=3_000)

    llm_log.record(
        route="coach",
        requested_model=model,
        latency_s=1.5,
        response=_response(model, usage),
        path=log,
    )

    line = _one(log)
    expected = (2_000 * rate_in + 500 * rate_out + 10_000 * rate_read + 3_000 * rate_write) / 1e6
    assert line["cost_usd"] == pytest.approx(expected, rel=1e-9)
    assert line["usage"] == {
        "input_tokens": 2_000,
        "output_tokens": 500,
        "cache_read_input_tokens": 10_000,
        "cache_creation_input_tokens": 3_000,
    }


def test_one_hour_cache_writes_are_priced_at_the_one_hour_rate(tmp_path: Path) -> None:
    """Opus 5.5 writes $5 for five minutes and $8 for an hour; the split comes from the response."""
    log = tmp_path / "llm_calls.jsonl"
    split = SimpleNamespace(ephemeral_5m_input_tokens=600, ephemeral_1h_input_tokens=400)
    usage = _usage(100, 10, cache_write=1_000, cache_creation=split)

    llm_log.record(
        route="coach",
        requested_model="claude-opus-5-5",
        latency_s=1.0,
        response=_response("claude-opus-5-5", usage),
        path=log,
    )

    line = _one(log)
    assert line["cost_usd"] == pytest.approx((100 * 4 + 10 * 20 + 600 * 5 + 400 * 8) / 1e6)
    assert line["usage"]["cache_creation_input_tokens"] == 1_000


@pytest.mark.parametrize(
    ("input_tokens", "cache_read", "expected"),
    [
        # 100,000 is "up to 100,000": the base rates.
        (100_000, 0, (100_000 * 0.10 + 1_000 * 0.50) / 1e6),
        # One token over: every token of the request at the long-prompt rates.
        (100_001, 0, (100_001 * 0.50 + 1_000 * 2.50) / 1e6),
        # Cache reads count toward the prompt's length, so this is over too.
        (1_000, 99_500, (1_000 * 0.50 + 99_500 * 0.05 + 1_000 * 2.50) / 1e6),
    ],
)
def test_haiku_prices_a_prompt_over_100k_tokens_at_its_long_prompt_tier(
    tmp_path: Path, input_tokens: int, cache_read: int, expected: float
) -> None:
    log = tmp_path / "llm_calls.jsonl"
    usage = _usage(input_tokens, 1_000, cache_read=cache_read)

    llm_log.record(
        route="club_lookup",
        requested_model="claude-haiku-5-5",
        latency_s=0.8,
        response=_response("claude-haiku-5-5", usage),
        path=log,
    )

    assert _one(log)["cost_usd"] == pytest.approx(expected, rel=1e-9)


def test_the_served_model_is_the_one_priced(tmp_path: Path) -> None:
    """The served model is the one billed, and a substitution is recorded rather than hidden."""
    log = tmp_path / "llm_calls.jsonl"

    llm_log.record(
        route="coach",
        requested_model="claude-opus-5",
        latency_s=2.0,
        response=_response("claude-haiku-5-5", _usage(1_000, 100)),
        path=log,
    )

    line = _one(log)
    assert line["requested_model"] == "claude-opus-5"
    assert line["served_model"] == "claude-haiku-5-5"
    assert line["cost_usd"] == pytest.approx((1_000 * 0.10 + 100 * 0.50) / 1e6)


def test_an_unknown_model_is_recorded_unpriced_with_one_warning_and_never_as_zero(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    log = tmp_path / "llm_calls.jsonl"
    for _ in range(2):
        llm_log.record(
            route="coach",
            requested_model="claude-opus-9",
            latency_s=1.0,
            response=_response("claude-opus-9", _usage(1_000, 100)),
            path=log,
        )

    lines = _lines(log)
    assert [line["cost_usd"] for line in lines] == [None, None]
    assert '"cost_usd":null' in log.read_text(encoding="utf-8")
    assert lines[0]["usage"]["input_tokens"] == 1_000  # the tokens are kept, so it can be re-priced
    err = capsys.readouterr().err.splitlines()
    assert len(err) == 1, "one warning per unknown model, not one per call"
    assert "claude-opus-9" in err[0]


def test_cost_usd_prices_a_lines_usage_from_the_same_table() -> None:
    usage = {
        "input_tokens": 2_000,
        "output_tokens": 500,
        "cache_read_input_tokens": 10_000,
        "cache_creation_input_tokens": 3_000,
    }

    assert llm_log.cost_usd("claude-opus-5", usage) == pytest.approx(0.04625)
    assert llm_log.cost_usd("claude-opus-9", usage) is None


# --- the line ----------------------------------------------------------------------------------


def test_a_line_carries_every_field_and_no_text(tmp_path: Path) -> None:
    """ADR-036 §2: metadata only. No prompt, no response text, and the ids it needs."""
    log = tmp_path / "llm_calls.jsonl"
    content = [
        SimpleNamespace(type="thinking", thinking="SECRET-THOUGHT"),
        SimpleNamespace(type="text", text="SECRET-PARAGRAPH"),
    ]

    llm_log.record(
        route="coach",
        requested_model="claude-opus-5",
        latency_s=3.25,
        response=_response(
            "claude-opus-5", _usage(1_200, 300), content=content, request_id="req_abc"
        ),
        path=log,
    )

    raw = log.read_text(encoding="utf-8")
    assert "SECRET" not in raw
    line = _one(log)
    assert set(line) == LINE_KEYS
    assert line["ts"].endswith("Z")
    assert line["origin"] == "app"
    assert line["route"] == "coach"
    assert line["latency_s"] == 3.25
    assert line["stop_reason"] == "end_turn"
    assert line["outcome"] == "ok"
    assert line["tool_calls"] == 0
    assert line["turn_id"] is None and line["iteration"] is None
    assert line["error_class"] is None
    assert line["request_id"] == "req_abc"


def test_the_conversation_loop_logs_its_turn_iteration_and_tool_calls(tmp_path: Path) -> None:
    log = tmp_path / "llm_calls.jsonl"
    content = [
        SimpleNamespace(type="text", text="Let me look."),
        SimpleNamespace(type="tool_use", id="t1", name="swing", input={}),
        {"type": "tool_use", "id": "t2", "name": "session", "input": {}},
    ]

    for iteration in (1, 2):
        llm_log.record(
            route="conversation",
            requested_model="claude-opus-5",
            latency_s=1.0,
            response=_response("claude-opus-5", _usage(500, 50), content=content),
            turn_id="turn-7",
            iteration=iteration,
            path=log,
        )

    lines = _lines(log)
    assert [(ln["turn_id"], ln["iteration"]) for ln in lines] == [("turn-7", 1), ("turn-7", 2)]
    assert [ln["tool_calls"] for ln in lines] == [2, 2]


def test_a_failed_request_records_its_error_class_and_request_id_unpriced(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    class RateLimitError(Exception):
        request_id = "req_err"

    log = tmp_path / "llm_calls.jsonl"

    llm_log.record(
        route="club_lookup",
        requested_model="claude-opus-5",
        latency_s=0.4,
        error=RateLimitError("429"),
        outcome="rate_limited",
        path=log,
    )

    line = _one(log)
    assert line["error_class"] == "RateLimitError"
    assert line["request_id"] == "req_err"
    assert line["outcome"] == "rate_limited"
    assert line["usage"] is None and line["cost_usd"] is None
    assert line["served_model"] is None and line["tool_calls"] is None
    assert capsys.readouterr().err == "", "no usage is not an unknown model"


def test_a_mock_response_is_logged_without_usage_rather_than_priced_at_zero(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    """The route tests hand in fakes. Their attributes are mocks, and none may become a number."""
    log = tmp_path / "llm_calls.jsonl"

    llm_log.record(
        route="coach",
        requested_model="claude-opus-5",
        latency_s=0.1,
        response=MagicMock(),
        path=log,
    )

    line = _one(log)
    assert line["usage"] is None and line["cost_usd"] is None
    assert line["served_model"] is None and line["request_id"] is None
    assert line["tool_calls"] == 0
    assert capsys.readouterr().err == ""


def test_an_sdk_message_is_read_by_its_real_field_names(tmp_path: Path) -> None:
    """The fakes above are shaped by hand, so one real `Message` proves the names are the SDK's."""
    anthropic = pytest.importorskip("anthropic")
    message = anthropic.types.Message.model_validate(
        {
            "id": "msg_01",
            "type": "message",
            "role": "assistant",
            "model": "claude-sonnet-5-5",
            "content": [
                {"type": "text", "text": "hello"},
                {"type": "tool_use", "id": "toolu_01", "name": "swing", "input": {}},
            ],
            "stop_reason": "tool_use",
            "stop_sequence": None,
            "usage": {
                "input_tokens": 1_000,
                "output_tokens": 200,
                "cache_read_input_tokens": 4_000,
                "cache_creation_input_tokens": 2_000,
                "cache_creation": {
                    "ephemeral_5m_input_tokens": 1_500,
                    "ephemeral_1h_input_tokens": 500,
                },
            },
        }
    )
    message._request_id = "req_real"
    log = tmp_path / "llm_calls.jsonl"

    llm_log.record(
        route="conversation",
        requested_model="claude-sonnet-5-5",
        latency_s=2.0,
        response=message,
        path=log,
    )

    line = _one(log)
    assert line["served_model"] == "claude-sonnet-5-5"
    assert line["stop_reason"] == "tool_use"
    assert line["tool_calls"] == 1
    assert line["request_id"] == "req_real"
    assert line["cost_usd"] == pytest.approx(
        (1_000 * 2 + 200 * 10 + 4_000 * 0.10 + 1_500 * 2.5 + 500 * 4) / 1e6
    )


def test_each_request_appends_its_own_line(tmp_path: Path) -> None:
    log = tmp_path / "nested" / "llm_calls.jsonl"  # the directory is made on the first write

    for route in ("club_lookup", "coach"):
        llm_log.record(
            route=route,  # type: ignore[arg-type]
            requested_model="claude-opus-5",
            latency_s=1.0,
            response=_response("claude-opus-5", _usage(10, 10)),
            path=log,
        )

    assert [line["route"] for line in _lines(log)] == ["club_lookup", "coach"]


def test_the_default_log_is_llm_calls_jsonl_under_the_processed_dir(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    # A directory of its own, so the suite-wide throwaway log (`tmp_path / "llm_calls.jsonl"`)
    # cannot satisfy this by coincidence if the real function were not put back.
    monkeypatch.setattr(llm_log, "_log_path", _REAL_LOG_PATH)
    monkeypatch.setattr(settings, "processed_dir", tmp_path / "processed")

    llm_log.record(route="coach", requested_model="claude-opus-5", latency_s=1.0)

    assert (tmp_path / "processed" / "llm_calls.jsonl").exists()


def test_a_failed_write_is_swallowed_with_one_line_on_stderr(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    """§4: the route that called `record` must get back exactly what it would have without it."""
    blocker = tmp_path / "not-a-directory"
    blocker.write_text("", encoding="utf-8")

    result = llm_log.record(
        route="coach",
        requested_model="claude-opus-5",
        latency_s=1.0,
        response=_response("claude-opus-5", _usage(10, 10)),
        path=blocker / "llm_calls.jsonl",
    )

    assert result is None
    err = capsys.readouterr().err.splitlines()
    assert len(err) == 1
    assert err[0].startswith("llm_log: the call was not logged")


# --- origin ------------------------------------------------------------------------------------


def test_the_origin_is_app_until_the_runner_tags_the_process(tmp_path: Path) -> None:
    log = tmp_path / "llm_calls.jsonl"

    llm_log.record(route="coach", requested_model="claude-opus-5", latency_s=1.0, path=log)
    llm_log.set_origin("eval:club_spec/baseline")
    llm_log.record(route="club_lookup", requested_model="claude-opus-5", latency_s=1.0, path=log)
    llm_log.set_origin("eval:coach_feedback/v3")
    llm_log.record(route="coach", requested_model="claude-opus-5", latency_s=1.0, path=log)
    llm_log.set_origin("app")
    llm_log.record(route="coach", requested_model="claude-opus-5", latency_s=1.0, path=log)

    assert [line["origin"] for line in _lines(log)] == [
        "app",
        "eval:club_spec/baseline",
        "eval:coach_feedback/v3",
        "app",
    ]


@pytest.mark.parametrize(
    "label",
    [
        "",
        "eval",
        "eval:club_spec",
        "eval:club_spec/",
        "eval:club_spec/v",
        "eval:Club/baseline",
        "eval:club_spec/pilot",
        "test",
        "APP",
    ],
)
def test_an_origin_that_is_neither_app_nor_an_eval_variant_is_a_wiring_bug(label: str) -> None:
    with pytest.raises(ValueError):
        llm_log.set_origin(label)
    assert llm_log.origin() == "app"


# --- the price table and the import boundary ---------------------------------------------------


def test_the_shipped_price_table_names_its_source_and_prices_every_model_m43_compares() -> None:
    """ADR-022: committed data carries its source and the date it was checked."""
    table = json.loads(
        (Path(llm_log.__file__).with_name("llm_prices.json")).read_text(encoding="utf-8")
    )

    assert table["source"] == "https://platform.claude.com/docs/en/about-claude/pricing"
    assert len(table["checked"]) == 10 and table["checked"][4] == "-"
    rates = ("input", "output", "cache_read", "cache_write_5m", "cache_write_1h")
    for model, _, _, _, _ in PAGE_RATES:
        entry = table["models"][model]
        assert all(isinstance(entry[r], float) and entry[r] > 0 for r in rates), model
    tier = table["models"]["claude-haiku-5-5"]["long_prompt"]
    assert tier["above_prompt_tokens"] == 100_000
    assert all(tier[r] > table["models"]["claude-haiku-5-5"][r] for r in rates)


def test_importing_the_call_log_does_not_import_anthropic() -> None:
    """ADR-036 §1: `feedback/coach.py` imports this, and must import clean without the extra.

    A subprocess for `test_pipeline_imports.py`'s reason: this process imported anthropic already.
    """
    code = "import golf_coach.llm_log, sys; print('anthropic' in sys.modules)"

    out = subprocess.run([sys.executable, "-c", code], capture_output=True, text=True, check=True)

    assert out.stdout.strip() == "False"


# --- the summary [M43 P6] ----------------------------------------------------------------------


def _fixture_line(
    route: str,
    model: str,
    *,
    latency: float,
    cost: float | None,
    usage: tuple[int, int, int, int] | None = (1_000, 100, 0, 0),
    outcome: str = "ok",
    served: str | None = "",
    stop_reason: str | None = "end_turn",
    origin: str = "app",
    ts: str = "2026-11-05T12:00:00.000Z",
    turn_id: str | None = None,
    iteration: int | None = None,
) -> dict[str, Any]:
    """One log line, written out the way `record` writes it. `served=""` means the model asked."""
    answered = usage is not None
    return {
        "ts": ts,
        "origin": origin,
        "route": route,
        "requested_model": model,
        "served_model": (served or model) if answered else None,
        "usage": dict(zip(llm_log._USAGE_KEYS, usage, strict=True)) if usage else None,
        "cost_usd": cost,
        "latency_s": latency,
        "stop_reason": stop_reason if answered else None,
        "outcome": outcome,
        "tool_calls": 0 if answered else None,
        "turn_id": turn_id,
        "iteration": iteration,
        "error_class": None if answered else "APIError",
        "request_id": "req_x",
    }


def _conversation(turn: str, iteration: int, latency: float, **kwargs: Any) -> dict[str, Any]:
    # Opus 5 at 1,000 in and 100 out: (1,000 x $5 + 100 x $25) / 1M = $0.0075.
    kwargs.setdefault("cost", 0.0075)
    kwargs.setdefault("stop_reason", "tool_use" if iteration == 1 else "end_turn")
    return _fixture_line(
        "conversation",
        "claude-opus-5",
        latency=latency,
        turn_id=turn,
        iteration=iteration,
        **kwargs,
    )


# Every cost is (tokens x rate) summed and over a million, at the rates in `PAGE_RATES`.
FIXTURE_LOG = [
    # coach / claude-opus-5: a truncated paragraph is served on the coach, and the 429 has no
    # usage, so it is a failed call but no latency and no cost.
    _fixture_line("coach", "claude-opus-5", latency=2.0, usage=(1_000, 200, 0, 0), cost=0.01),
    # 1,000 x 5 + 400 x 25 + 3,000 cache read x 0.50 = 16,500
    _fixture_line("coach", "claude-opus-5", latency=4.0, usage=(1_000, 400, 3_000, 0), cost=0.0165),
    _fixture_line(
        "coach",
        "claude-opus-5",
        latency=6.0,
        usage=(1_000, 2_000, 0, 0),
        cost=0.055,
        outcome="truncated",
        stop_reason="max_tokens",
    ),
    _fixture_line(
        "coach", "claude-opus-5", latency=0.5, usage=None, cost=None, outcome="rate_limited"
    ),
    # club_lookup / claude-opus-5: the lookup's `truncated` served nothing, so it failed.
    # 2,000 x 5 + 1,000 x 25 + 1,000 cache write x 6.25 = 41,250
    _fixture_line(
        "club_lookup", "claude-opus-5", latency=10.0, usage=(2_000, 1_000, 0, 1_000), cost=0.04125
    ),
    _fixture_line(
        "club_lookup",
        "claude-opus-5",
        latency=30.0,
        usage=(2_000, 4_000, 0, 0),
        cost=0.11,
        outcome="truncated",
        stop_reason="max_tokens",
    ),
    # conversation / claude-opus-5: three turns, interleaved as two worker threads would write
    # them. Turn a answers; b runs out of steps; c's second request raises.
    _conversation("a", 1, 1.0),
    _conversation("b", 1, 3.0),
    _conversation("a", 2, 2.0),
    _conversation("b", 2, 4.0, outcome="exhausted", stop_reason="tool_use"),
    _conversation("c", 1, 5.0),
    _conversation("c", 2, 0.2, usage=None, cost=None, outcome="connection_failed"),
    # coach / claude-opus-9: a model the price table does not know. Two calls are unpriced, and
    # the 529 is not one of them, because it was never billed.
    _fixture_line("coach", "claude-opus-9", latency=1.0, cost=None),
    _fixture_line("coach", "claude-opus-9", latency=3.0, cost=None),
    _fixture_line(
        "coach", "claude-opus-9", latency=0.1, usage=None, cost=None, outcome="overloaded"
    ),
    # club_lookup / claude-sonnet-5-5: one answered by Sonnet 5.5 (2,000 + 10,000 = 12,000), one
    # by a model the table does not know, which is named both ways.
    _fixture_line(
        "club_lookup", "claude-sonnet-5-5", latency=7.0, usage=(1_000, 1_000, 0, 0), cost=0.012
    ),
    _fixture_line(
        "club_lookup",
        "claude-sonnet-5-5",
        latency=8.0,
        usage=(1_000, 1_000, 0, 0),
        cost=None,
        served="claude-sonnet-9",
    ),
    # Two eval runs, on Haiku 5.5: 1,000 x 0.10 + 100 x 0.50 = 150, and twice that.
    _fixture_line(
        "club_lookup",
        "claude-haiku-5-5",
        latency=1.5,
        cost=0.00015,
        origin="eval:club_spec/baseline",
    ),
    _fixture_line(
        "coach",
        "claude-haiku-5-5",
        latency=2.5,
        usage=(2_000, 200, 0, 0),
        cost=0.0003,
        origin="eval:coach_feedback/v1",
    ),
]

# What each `app` group must come to, worked by hand from the lines above. Latency is nearest-rank
# over the answered calls: p50 is the ceil(n/2)-th smallest and p95 the ceil(0.95n)-th.
EXPECTED_APP = {
    ("club_lookup", "claude-opus-5"): {
        "calls": 2,
        "outcomes": {"ok": 1, "truncated": 1},
        "failed": 1,
        "failure_rate": 0.5,
        "answered": 2,
        "latency_p50_s": 10.0,  # [10, 30]: the 1st
        "latency_p95_s": 30.0,  # the 2nd
        "input_tokens": 4_000,
        "output_tokens": 5_000,
        "cache_read_tokens": 0,
        "cache_write_tokens": 1_000,
        "cache_read_share": 0.0,
        "priced": 2,
        "cost_usd": 0.15125,
        "cost_per_call_usd": 0.075625,
        "unpriced": {},
        "served_by_other": {},
        "turns": 0,
        "failed_turns": 0,
    },
    ("club_lookup", "claude-sonnet-5-5"): {
        "calls": 2,
        "outcomes": {"ok": 2},
        "failed": 0,
        "failure_rate": 0.0,
        "answered": 2,
        "latency_p50_s": 7.0,
        "latency_p95_s": 8.0,
        "input_tokens": 2_000,
        "output_tokens": 2_000,
        "cache_read_tokens": 0,
        "cache_write_tokens": 0,
        "cache_read_share": 0.0,
        "priced": 1,
        "cost_usd": 0.012,
        "cost_per_call_usd": 0.012,  # over the one priced call, not the two
        "unpriced": {"claude-sonnet-9": 1},
        "served_by_other": {"claude-sonnet-9": 1},
        "turns": 0,
        "failed_turns": 0,
    },
    ("coach", "claude-opus-5"): {
        "calls": 4,
        "outcomes": {"ok": 2, "rate_limited": 1, "truncated": 1},
        "failed": 1,  # the 429; the truncated paragraph was served
        "failure_rate": 0.25,
        "answered": 3,
        "latency_p50_s": 4.0,  # [2, 4, 6]: the 2nd; the 429's 0.5 s is not in it
        "latency_p95_s": 6.0,  # the 3rd
        "input_tokens": 3_000,
        "output_tokens": 2_600,
        "cache_read_tokens": 3_000,
        "cache_write_tokens": 0,
        "cache_read_share": 0.5,  # 3,000 of 3,000 + 3,000 + 0
        "priced": 3,
        "cost_usd": 0.0815,  # 0.01 + 0.0165 + 0.055
        "cost_per_call_usd": 0.0815 / 3,
        "unpriced": {},
        "served_by_other": {},
        "turns": 0,
        "failed_turns": 0,
    },
    ("coach", "claude-opus-9"): {
        "calls": 3,
        "outcomes": {"ok": 2, "overloaded": 1},
        "failed": 1,
        "failure_rate": 1 / 3,
        "answered": 2,
        "latency_p50_s": 1.0,
        "latency_p95_s": 3.0,
        "input_tokens": 2_000,
        "output_tokens": 200,
        "cache_read_tokens": 0,
        "cache_write_tokens": 0,
        "cache_read_share": 0.0,
        "priced": 0,
        "cost_usd": 0.0,  # nothing priced: the two unpriced calls are not in it as zeros
        "cost_per_call_usd": None,
        "unpriced": {"claude-opus-9": 2},
        "served_by_other": {},
        "turns": 0,
        "failed_turns": 0,
    },
    ("conversation", "claude-opus-5"): {
        "calls": 6,
        "outcomes": {"ok": 4, "connection_failed": 1, "exhausted": 1},
        "failed": 2,  # per request, the tool-use `ok`s are in the denominator
        "failure_rate": 2 / 6,
        "answered": 5,
        "latency_p50_s": 3.0,  # [1, 2, 3, 4, 5]: the 3rd
        "latency_p95_s": 5.0,  # the 5th
        "input_tokens": 5_000,
        "output_tokens": 500,
        "cache_read_tokens": 0,
        "cache_write_tokens": 0,
        "cache_read_share": 0.0,
        "priced": 5,
        "cost_usd": 0.0375,
        "cost_per_call_usd": 0.0075,
        "unpriced": {},
        "served_by_other": {},
        "turns": 3,
        "failed_turns": 2,  # b ended exhausted and c on a raised request; a answered
    },
}


def _as_dict(summary: llm_log.Summary) -> dict[str, Any]:
    names = next(iter(EXPECTED_APP.values()))
    return {name: getattr(summary, name) for name in names}


def _write_log(path: Path, lines: list[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    text = "".join(json.dumps(line, separators=(",", ":")) + "\n" for line in lines)
    path.write_text(text, encoding="utf-8")


def test_the_summary_of_a_fixture_log_matches_the_numbers_worked_by_hand() -> None:
    summaries = llm_log.summarize(FIXTURE_LOG)

    assert [(s.origin, s.route, s.model) for s in summaries] == [
        ("app", route, model) for route, model in EXPECTED_APP
    ], "app only, by default, and sorted"
    for summary in summaries:
        expected = dict(EXPECTED_APP[(summary.route, summary.model)])
        actual = _as_dict(summary)
        group = (summary.route, summary.model)
        # `approx` takes no nested mapping, so the three counts by name are compared exactly.
        for name in ("outcomes", "unpriced", "served_by_other"):
            assert actual.pop(name) == expected.pop(name), (group, name)
        assert actual == pytest.approx(expected), group
        assert list(summary.outcomes) == list(EXPECTED_APP[group]["outcomes"]), "commonest first"


def test_the_origin_filter_keeps_real_use_unless_eval_or_all_is_asked_for() -> None:
    evals = llm_log.summarize(FIXTURE_LOG, origin="eval")
    everything = llm_log.summarize(FIXTURE_LOG, origin="all")

    assert [(s.origin, s.route, s.model, s.calls, s.cost_usd) for s in evals] == [
        ("eval:club_spec/baseline", "club_lookup", "claude-haiku-5-5", 1, 0.00015),
        ("eval:coach_feedback/v1", "coach", "claude-haiku-5-5", 1, 0.0003),
    ]
    # One answered call: it is its own p50 and p95.
    assert (evals[0].latency_p50_s, evals[0].latency_p95_s) == (1.5, 1.5)
    # Each origin stays its own group under `all`, so an eval never folds into real use.
    assert len(everything) == len(EXPECTED_APP) + 2
    assert sum(s.calls for s in everything) == len(FIXTURE_LOG)


def test_since_keeps_the_calls_at_or_after_it_read_as_utc() -> None:
    lines = [
        _fixture_line("coach", "claude-opus-5", latency=1.0, cost=0.0075, ts=ts)
        for ts in ("2026-10-31T23:59:59.999Z", "2026-11-01T00:00:00.000Z", "2026-11-02T08:00:00Z")
    ]

    def calls(since: datetime | None) -> int:
        return sum(s.calls for s in llm_log.summarize(lines, since=since))

    assert calls(None) == 3
    assert calls(datetime(2026, 11, 1)) == 2, "a naive time is UTC, and the boundary is kept"
    assert calls(datetime.fromisoformat("2026-11-01T01:00:00+01:00")) == 2
    assert calls(datetime(2026, 11, 3, tzinfo=UTC)) == 0


def test_the_command_prints_each_group_and_a_total_that_names_what_it_left_out(
    capsys: pytest.CaptureFixture[str],
) -> None:
    _write_log(llm_log._log_path(), FIXTURE_LOG)

    assert llm_log.main(["summary"]) == 0

    out = capsys.readouterr().out
    assert out.isascii(), "no console code page may mangle it"
    lines = out.splitlines()
    coach = lines.index("coach / claude-opus-5  [app]")
    assert lines[coach + 1 : coach + 8] == [
        "  calls      4: ok 2, rate_limited 1, truncated 1",
        "  failed     1 of 4 (25.0%); served on coach: ok, truncated",
        "  latency    p50 4.00 s, p95 6.00 s, over 3 answered",
        "  tokens     in 3,000, out 2,600, cache read 3,000, cache write 0",
        "  cache      50.0% of prompt tokens read from the cache",
        "  cost       $0.081500 total, $0.027167 per call, over 3 priced",
        "",
    ]
    assert "  failed     1 of 2 (50.0%); served on club_lookup: ok" in lines
    assert "  cost       none priced" in lines
    assert "  unpriced   2 (claude-opus-9 2), not in the cost" in lines
    assert "  turns      3, failed 2 (66.7%)" in lines
    assert "  served by another model: claude-sonnet-9 1" in lines
    # 0.15125 + 0.012 + 0.0815 + 0.0375 over 2 + 1 + 3 + 5 priced calls; no eval line in it.
    assert lines[-1] == (
        "Total: 17 calls, $0.282250 over 11 priced; 3 unpriced"
        " (claude-opus-9 2, claude-sonnet-9 1), not in the total."
    )
    assert "eval:" not in out


def test_the_command_reads_eval_runs_and_a_since_when_asked(
    capsys: pytest.CaptureFixture[str],
) -> None:
    _write_log(llm_log._log_path(), FIXTURE_LOG)

    assert llm_log.main(["summary", "--origin", "eval", "--since", "2026-11-01"]) == 0

    lines = capsys.readouterr().out.splitlines()
    assert lines[0].endswith(": origin eval, since 2026-11-01T00:00:00+00:00.")
    assert "club_lookup / claude-haiku-5-5  [eval:club_spec/baseline]" in lines
    assert lines[-1] == "Total: 2 calls, $0.000450 over 2 priced."


def test_unreadable_lines_are_skipped_and_counted_on_stderr_and_the_rest_summed(
    capsys: pytest.CaptureFixture[str],
) -> None:
    log = llm_log._log_path()
    good = _fixture_line("coach", "claude-opus-5", latency=2.0, cost=0.0075)
    no_outcome = {key: value for key, value in good.items() if key != "outcome"}
    _write_log(log, [good, no_outcome, {**good, "latency_s": "2.0"}, {**good, "ts": "Tuesday"}])
    with log.open("a", encoding="utf-8") as handle:
        handle.write("[1, 2]\n\n" + json.dumps(good)[:40] + "\n")

    assert llm_log.main(["summary"]) == 0

    captured = capsys.readouterr()
    assert captured.err.splitlines() == [f"llm_log: skipped 5 unreadable line(s) in {log}"]
    assert "  calls      1: ok 1" in captured.out.splitlines()


def test_every_line_record_writes_is_one_the_summary_reads(tmp_path: Path) -> None:
    """The writer and the reader agree on the line, including its null-usage and loop shapes."""
    log = tmp_path / "llm_calls.jsonl"
    llm_log.record(
        route="coach",
        requested_model="claude-opus-5",
        latency_s=1.0,
        response=_response("claude-opus-5", _usage(100, 10, cache_read=5)),
        path=log,
    )
    llm_log.record(
        route="conversation",
        requested_model="claude-opus-5",
        latency_s=0.3,
        error=RuntimeError("boom"),
        outcome="request_failed",
        turn_id="t",
        iteration=2,
        path=log,
    )
    llm_log.record(
        route="coach",
        requested_model="claude-opus-5",
        latency_s=0.1,
        response=MagicMock(),
        path=log,
    )

    lines, skipped = llm_log.read_log(log)

    assert (len(lines), skipped) == (3, 0)
    assert llm_log.summarize(lines)[0].priced == 1


def test_no_log_yet_and_nothing_matching_are_each_said_plainly(
    capsys: pytest.CaptureFixture[str],
) -> None:
    assert llm_log.main(["summary"]) == 0
    assert capsys.readouterr().out.startswith("No calls logged yet: ")

    _write_log(llm_log._log_path(), FIXTURE_LOG[:1])
    assert llm_log.main(["summary", "--origin", "eval"]) == 0
    assert capsys.readouterr().out.strip().endswith("match: origin eval.")


@pytest.mark.parametrize(
    "argv", [["summary", "--since", "last week"], ["summary", "--origin", "x"]]
)
def test_a_since_or_origin_it_cannot_read_is_a_usage_error(argv: list[str]) -> None:
    with pytest.raises(SystemExit) as exit_:
        llm_log.main(argv)
    assert exit_.value.code == 2


def test_python_m_golf_coach_llm_log_summary_runs(tmp_path: Path) -> None:
    """The documented command, end to end, on a log under `GOLF_PROCESSED_DIR`."""
    _write_log(tmp_path / "llm_calls.jsonl", FIXTURE_LOG)
    env = {**os.environ, "GOLF_PROCESSED_DIR": str(tmp_path)}

    out = subprocess.run(
        [sys.executable, "-m", "golf_coach.llm_log", "summary", "--origin", "all"],
        capture_output=True,
        text=True,
        env=env,
        check=True,
    )

    assert out.stderr == ""
    assert out.stdout.splitlines()[-1].startswith(f"Total: {len(FIXTURE_LOG)} calls, $0.282700 ")
