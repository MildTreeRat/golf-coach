"""The call log: one JSON line per Claude API request, with what it cost (ADR-036 §2–§4). [M43 P2]

Every route that calls Claude (`clubs/lookup.py::_look_up`, `feedback/coach.py::_write`,
`feedback/conversation.py::_mirror`) calls `record` once per request, at the one place the request
is made. Each call appends one line to `llm_calls.jsonl` under `settings.processed_dir`, which is
git-ignored because it records the user's own use.

**Metadata only.** A line holds the models, the token counts, the cost, the latency, the stop
reason, the outcome and the ids, and never a prompt, a response's text or a key. `record` reads the
response for its `.model`, `.usage`, `.stop_reason`, `._request_id` and the *types* of its content
blocks, and nothing else.

**Logging never changes an outcome** (§4). `record` never raises: anything that goes wrong in it,
a full disk or a response shaped unlike the SDK's, is swallowed with one line on stderr, so a
paragraph never becomes a note and a note never becomes a crash because the log could not be
written.

**Cost is priced from measured `usage`** with `llm_prices.json`, which carries its source and the
date it was checked (ADR-022). A model the table does not know is recorded with `cost_usd: null`
and one warning on stderr, never a zero, because a zero would be summed into a total as if the call
had been free. A request that failed carries no usage, so it is unpriced too.

**Imports nothing from `anthropic`.** `feedback/coach.py` imports this module, and it must still
import clean without the `llm` extra (`tests/api/test_pipeline_imports.py`). The response is read
by attribute, so any object shaped like the SDK's `Message` will do, and so will a test's fake.

**The summary [M43 P6].** `python -m golf_coach.llm_log summary [--since DATE] [--origin
app|eval|all]` reads the log back and prints, per origin, route and requested model: the calls and
their outcomes, the failure rate, latency p50/p95, the tokens, the cache-read share, and the total
and per-call cost. `--origin` is `app` unless asked (§3), so an eval never bills real use. What
each number counts:

- **failed** is a call whose outcome served nothing. What is served depends on the route:
  `truncated` serves a paragraph with an admission on `coach` and `conversation`, and nothing on
  `club_lookup` (`_SERVED`). `exhausted`, the conversation's "ask again", is a failure. On
  `conversation` a call is one request, so a turn's tool-use requests count as `ok` calls, and
  the turns line gives the per-question view: a turn is every line with one `turn_id`, and its
  outcome is its last iteration's.
- **latency** is over the requests the API answered (a line with `usage`). A request that raised
  is a call and an outcome, but the time to an error is not the model's latency. The percentiles
  are nearest-rank, so each is a latency that was measured.
- **cache-read share** is cache reads over the whole prompt: input, cache reads and cache writes.
- **cost** sums the logged `cost_usd` of the priced calls, and the per-call cost divides by those
  calls. A call with `usage` and no cost is **unpriced**: counted, named by the model that answered
  it, and never summed as a zero (§2). A request that raised has no usage, was not billed, and is
  neither priced nor unpriced.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
import threading
from collections import Counter
from collections.abc import Iterable, Mapping, Sequence
from dataclasses import dataclass
from datetime import UTC, datetime
from functools import lru_cache
from pathlib import Path
from typing import Any, Literal

from golf_coach.config import settings

Route = Literal["club_lookup", "coach", "conversation"]

LOG_FILE = "llm_calls.jsonl"
DEFAULT_ORIGIN = "app"

_PRICES_FILE = "llm_prices.json"
_PER_MILLION = 1_000_000
_RATES = ("input", "output", "cache_read", "cache_write_5m", "cache_write_1h")

# `eval:<flow>/<variant>`, where a variant is a run directory build-eval.md names: `baseline` or
# `v<N>`. Anything else is a runner wiring bug, and `set_origin` raises on it.
_EVAL_ORIGIN = re.compile(r"eval:[a-z][a-z0-9_]*/(baseline|v[0-9]+)")

# Block types that are the model asking for a tool. The conversation's tools are client tools
# (`tool_use`); a server tool is still a tool call, so it counts the same.
_TOOL_BLOCKS = frozenset({"tool_use", "server_tool_use"})

_origin = DEFAULT_ORIGIN
_write_lock = threading.Lock()
_warned_unpriced: set[str] = set()


def set_origin(label: str) -> None:
    """Tag every line this process writes from now on (ADR-036 §3).

    Process-wide, because one eval run is one process and one origin. `app` is the default and
    what real use writes; the runner sets `eval:<flow>/<variant>` before its first call, so a
    summary of real use never counts what an eval spent.
    """
    global _origin
    if label != DEFAULT_ORIGIN and not _EVAL_ORIGIN.fullmatch(label):
        raise ValueError(
            f"origin must be {DEFAULT_ORIGIN!r} or 'eval:<flow>/<variant>' (variant 'baseline' "
            f"or 'v<N>'), not {label!r}"
        )
    _origin = label


def origin() -> str:
    """The tag the next line will carry."""
    return _origin


def record(
    *,
    route: Route,
    requested_model: str,
    latency_s: float,
    response: Any = None,
    outcome: str = "ok",
    error: BaseException | None = None,
    turn_id: str | None = None,
    iteration: int | None = None,
    path: Path | None = None,
) -> None:
    """Append one line for one API request. Never raises.

    - `response`: what `messages.create` returned (or one message the tool runner yielded), or
      `None` when the request raised.
    - `outcome`: `ok`, or the kind of note the route turned this request into.
    - `error`: the exception the request raised, if it did. Its class name is recorded, and its
      `request_id` when the SDK attached one.
    - `turn_id` and `iteration`: the conversation loop's, `None` on the other routes.
    - `path`: the log file. `None` means `settings.processed_dir / "llm_calls.jsonl"`, read at
      call time.
    """
    try:
        line = _line(
            route=route,
            requested_model=requested_model,
            latency_s=latency_s,
            response=response,
            outcome=outcome,
            error=error,
            turn_id=turn_id,
            iteration=iteration,
        )
        text = json.dumps(line, separators=(",", ":")) + "\n"
        target = path if path is not None else _log_path()
        # A lock, because the API server runs route calls on worker threads and an append of one
        # line is not promised to be atomic on Windows. Two interleaved halves would be two
        # unreadable lines.
        with _write_lock:
            target.parent.mkdir(parents=True, exist_ok=True)
            with target.open("a", encoding="utf-8") as handle:
                handle.write(text)
    except Exception as exc:  # §4: observation never changes the outcome it observes
        _warn(f"the call was not logged ({type(exc).__name__}: {exc})")


def cost_usd(model: str, usage: dict[str, int]) -> float | None:
    """What `usage` cost on `model`, or `None` when the price table does not know the model.

    `usage` is a line's `usage` object. The cache-write total is priced at the 5-minute rate, which
    is the only duration a route asks for; `record` prices a 1-hour split from the response itself.
    """
    return _price(
        model,
        input_tokens=usage["input_tokens"],
        output_tokens=usage["output_tokens"],
        cache_read=usage["cache_read_input_tokens"],
        cache_write_5m=usage["cache_creation_input_tokens"],
        cache_write_1h=0,
    )


def _line(
    *,
    route: Route,
    requested_model: str,
    latency_s: float,
    response: Any,
    outcome: str,
    error: BaseException | None,
    turn_id: str | None,
    iteration: int | None,
) -> dict[str, Any]:
    served_model = _text(getattr(response, "model", None))
    tokens = _tokens(getattr(response, "usage", None))
    usage: dict[str, int] | None = None
    cost: float | None = None
    if tokens is not None:
        input_tokens, output_tokens, cache_read, write_5m, write_1h = tokens
        usage = {
            "input_tokens": input_tokens,
            "output_tokens": output_tokens,
            "cache_read_input_tokens": cache_read,
            "cache_creation_input_tokens": write_5m + write_1h,
        }
        # The served model is the one billed. A response with no `model` (a fake, in practice) is
        # priced as the model that was asked for.
        cost = _price(
            served_model or requested_model,
            input_tokens=input_tokens,
            output_tokens=output_tokens,
            cache_read=cache_read,
            cache_write_5m=write_5m,
            cache_write_1h=write_1h,
        )
    return {
        "ts": datetime.now(UTC).isoformat(timespec="milliseconds").replace("+00:00", "Z"),
        "origin": _origin,
        "route": route,
        "requested_model": requested_model,
        "served_model": served_model,
        "usage": usage,
        "cost_usd": cost,
        "latency_s": round(float(latency_s), 6),
        "stop_reason": _text(getattr(response, "stop_reason", None)),
        "outcome": outcome,
        "tool_calls": _tool_calls(response) if response is not None else None,
        "turn_id": turn_id,
        "iteration": iteration,
        "error_class": type(error).__name__ if error is not None else None,
        "request_id": _text(getattr(response, "_request_id", None))
        or _text(getattr(error, "request_id", None)),
    }


def _tokens(usage: Any) -> tuple[int, int, int, int, int] | None:
    """(input, output, cache read, 5-minute write, 1-hour write), or `None` with no real usage.

    `input_tokens` and `output_tokens` must be integers or there is no usage at all: a fake whose
    attributes are mocks would otherwise be priced at a fake zero. The cache fields are `None` on
    the SDK when nothing was cached, and that is a real zero.
    """
    input_tokens = _count(getattr(usage, "input_tokens", None))
    output_tokens = _count(getattr(usage, "output_tokens", None))
    if input_tokens is None or output_tokens is None:
        return None
    cache_read = _count(getattr(usage, "cache_read_input_tokens", None)) or 0
    cache_write = _count(getattr(usage, "cache_creation_input_tokens", None)) or 0
    # The split by duration, when the API sends it. The 1-hour writes are priced at their own rate
    # and the rest of the total at the 5-minute one, so a total without a split prices as 5-minute.
    split = getattr(usage, "cache_creation", None)
    write_1h = min(_count(getattr(split, "ephemeral_1h_input_tokens", None)) or 0, cache_write)
    return input_tokens, output_tokens, cache_read, cache_write - write_1h, write_1h


def _price(
    model: str,
    *,
    input_tokens: int,
    output_tokens: int,
    cache_read: int,
    cache_write_5m: int,
    cache_write_1h: int,
) -> float | None:
    rates = _prices().get(model)
    if rates is None:
        if model not in _warned_unpriced:
            _warned_unpriced.add(model)
            _warn(f"no price for model {model!r} in {_PRICES_FILE}; its calls log cost_usd: null")
        return None
    tier = rates.get("long_prompt")
    # A long-prompt tier (Haiku 5.5's) is chosen by the whole prompt, cache reads and writes
    # included, and each request is priced on its own (the pricing page, "Long context pricing").
    prompt = input_tokens + cache_read + cache_write_5m + cache_write_1h
    if isinstance(tier, dict) and prompt > tier["above_prompt_tokens"]:
        rates = tier
    total = (
        input_tokens * rates["input"]
        + output_tokens * rates["output"]
        + cache_read * rates["cache_read"]
        + cache_write_5m * rates["cache_write_5m"]
        + cache_write_1h * rates["cache_write_1h"]
    )
    return round(total / _PER_MILLION, 10)


@lru_cache(maxsize=1)
def _prices() -> dict[str, dict[str, Any]]:
    """The packaged price table's `models`, read once. Committed data, so a bad file is a bug.

    Loaded the way `clubs/catalogue.py` loads `club_catalogue.json`: a path beside this module and
    `json.loads`. Cached, unlike the catalogue, because nothing writes this file while a process
    runs.
    """
    payload = json.loads(_prices_path().read_text(encoding="utf-8"))
    models: dict[str, dict[str, Any]] = payload["models"]
    return models


def _prices_path() -> Path:
    return Path(__file__).with_name(_PRICES_FILE)


def _log_path() -> Path:
    """Where the log lives. A function so the test suite can point it at a throwaway file."""
    return Path(settings.processed_dir) / LOG_FILE


def _tool_calls(response: Any) -> int:
    content = getattr(response, "content", None)
    if not isinstance(content, list | tuple):
        return 0
    return sum(1 for block in content if _block_type(block) in _TOOL_BLOCKS)


def _block_type(block: Any) -> Any:
    return block.get("type") if isinstance(block, dict) else getattr(block, "type", None)


def _count(value: Any) -> int | None:
    # `bool` is an `int`, and `True` tokens is a fake's mistake, not a count.
    return value if isinstance(value, int) and not isinstance(value, bool) else None


def _text(value: Any) -> str | None:
    return value if isinstance(value, str) and value else None


def _warn(message: str) -> None:
    """One line on stderr, whatever the message held."""
    print(f"llm_log: {' '.join(message.split())}", file=sys.stderr)


# --- the summary [M43 P6] ----------------------------------------------------------------------

OriginFilter = Literal["app", "eval", "all"]

#: The outcomes that put an answer in front of the golfer, per route; every other outcome failed.
#: `truncated` is served with an admission on the coach and the conversation, and serves nothing on
#: the lookup (M43 P4). A route this table does not know serves only `ok`.
_SERVED: dict[str, frozenset[str]] = {
    "club_lookup": frozenset({"ok"}),
    "coach": frozenset({"ok", "truncated"}),
    "conversation": frozenset({"ok", "truncated"}),
}
_ONLY_OK = frozenset({"ok"})

_USAGE_KEYS = (
    "input_tokens",
    "output_tokens",
    "cache_read_input_tokens",
    "cache_creation_input_tokens",
)


@dataclass(frozen=True)
class Summary:
    """One origin, route and requested model: what its calls did, how long, and what they cost.

    The module docstring says what each count counts. `outcomes` runs from the commonest kind down.
    `unpriced` and `served_by_other` map a model to its number of calls.
    """

    origin: str
    route: str
    model: str
    calls: int
    outcomes: dict[str, int]
    failed: int
    answered: int
    latency_p50_s: float | None
    latency_p95_s: float | None
    input_tokens: int
    output_tokens: int
    cache_read_tokens: int
    cache_write_tokens: int
    priced: int
    cost_usd: float
    unpriced: dict[str, int]
    served_by_other: dict[str, int]
    turns: int
    failed_turns: int

    @property
    def failure_rate(self) -> float:
        return self.failed / self.calls

    @property
    def cache_read_share(self) -> float | None:
        """Cache reads over the whole prompt, or `None` when no request was answered."""
        prompt = self.input_tokens + self.cache_read_tokens + self.cache_write_tokens
        return self.cache_read_tokens / prompt if prompt else None

    @property
    def cost_per_call_usd(self) -> float | None:
        """The mean cost of a priced call, or `None` when none was priced, never a zero."""
        return self.cost_usd / self.priced if self.priced else None


def read_log(path: Path) -> tuple[list[dict[str, Any]], int]:
    """The log's lines, and how many were skipped as unreadable.

    `record` writes whole lines under a lock, so an unreadable one was edited by hand or cut short
    by a crash. It is skipped and counted, and never stops the summary of every other line.
    """
    lines: list[dict[str, Any]] = []
    skipped = 0
    for raw in path.read_text(encoding="utf-8", errors="replace").splitlines():
        if not raw.strip():
            continue
        try:
            line = json.loads(raw)
        except ValueError:
            skipped += 1
            continue
        if _readable(line):
            lines.append(line)
        else:
            skipped += 1
    return lines, skipped


def summarize(
    lines: Iterable[Mapping[str, Any]],
    *,
    origin: OriginFilter = "app",
    since: datetime | None = None,
) -> list[Summary]:
    """One `Summary` per origin, route and requested model, sorted, so `app` comes first.

    `origin` keeps `app` (real use), every `eval:…`, or `all`. `since` keeps the lines at or after
    it, and a naive time is UTC, as the log's are. The requested model is the group, because it is
    the one the route chose; a call another model answered is priced as that model and named.
    """
    if since is not None and since.tzinfo is None:
        since = since.replace(tzinfo=UTC)
    groups: dict[tuple[str, str, str], list[Mapping[str, Any]]] = {}
    for line in lines:
        if not _kept(line["origin"], origin):
            continue
        when = _when(line["ts"])
        if since is not None and (when is None or when < since):
            continue
        key = (line["origin"], line["route"], line["requested_model"])
        groups.setdefault(key, []).append(line)
    return [_summary(*key, members) for key, members in sorted(groups.items())]


def render(
    summaries: Sequence[Summary],
    *,
    path: Path,
    origin: OriginFilter,
    since: datetime | None,
) -> str:
    """The summary as plain ASCII text, one block per group and a total line."""
    scope = f"origin {origin}" + (f", since {since.isoformat()}" if since is not None else "")
    if not summaries:
        return f"No calls in {path} match: {scope}."
    out = [f"Claude API calls in {path}: {scope}.", ""]
    for summary in summaries:
        out.extend(_block(summary))
        out.append("")
    out.append(_total(summaries))
    return "\n".join(out)


def main(argv: Sequence[str] | None = None) -> int:
    """`python -m golf_coach.llm_log summary`: what the logged calls did and cost (ADR-036 §3)."""
    parser = argparse.ArgumentParser(
        prog="python -m golf_coach.llm_log",
        description="Read the Claude call log back.",
    )
    commands = parser.add_subparsers(dest="command", required=True)
    summary = commands.add_parser(
        "summary", help="Calls, outcomes, latency, tokens and cost, per route and model."
    )
    summary.add_argument(
        "--since",
        type=_since,
        metavar="DATE",
        help="Only calls at or after this: YYYY-MM-DD or an ISO time, UTC unless it names a zone.",
    )
    summary.add_argument(
        "--origin",
        choices=("app", "eval", "all"),
        default="app",
        help="app (real use, the default), eval (every eval run) or all.",
    )
    args = parser.parse_args(argv)

    path = _log_path()
    if not path.exists():
        print(f"No calls logged yet: {path} does not exist.")
        return 0
    lines, skipped = read_log(path)
    if skipped:
        _warn(f"skipped {skipped} unreadable line(s) in {path}")
    summaries = summarize(lines, origin=args.origin, since=args.since)
    print(render(summaries, path=path, origin=args.origin, since=args.since))
    return 0


def _summary(origin_: str, route: str, model: str, members: list[Mapping[str, Any]]) -> Summary:
    served = _SERVED.get(route, _ONLY_OK)
    outcomes = Counter(str(line["outcome"]) for line in members)
    answered = [line for line in members if line.get("usage") is not None]
    latencies = sorted(float(line["latency_s"]) for line in answered)
    tokens = {key: sum(int(line["usage"][key]) for line in answered) for key in _USAGE_KEYS}
    priced = [float(line["cost_usd"]) for line in members if line.get("cost_usd") is not None]
    unpriced = Counter(
        str(line.get("served_model") or model) for line in answered if line.get("cost_usd") is None
    )
    served_by_other = Counter(
        str(line["served_model"])
        for line in members
        if line.get("served_model") is not None and line["served_model"] != model
    )
    turns = _turn_outcomes(members)
    return Summary(
        origin=origin_,
        route=route,
        model=model,
        calls=len(members),
        outcomes=dict(sorted(outcomes.items(), key=lambda item: (-item[1], item[0]))),
        failed=sum(count for kind, count in outcomes.items() if kind not in served),
        answered=len(answered),
        latency_p50_s=_nearest_rank(latencies, 50),
        latency_p95_s=_nearest_rank(latencies, 95),
        input_tokens=tokens["input_tokens"],
        output_tokens=tokens["output_tokens"],
        cache_read_tokens=tokens["cache_read_input_tokens"],
        cache_write_tokens=tokens["cache_creation_input_tokens"],
        priced=len(priced),
        cost_usd=sum(priced),
        unpriced=dict(sorted(unpriced.items())),
        served_by_other=dict(sorted(served_by_other.items())),
        turns=len(turns),
        failed_turns=sum(1 for kind in turns if kind not in served),
    )


def _turn_outcomes(members: Iterable[Mapping[str, Any]]) -> list[str]:
    """Each conversation turn's outcome: its last iteration's (M43 P5). Empty off the loop."""
    last: dict[str, tuple[int, str]] = {}
    for line in members:
        turn_id = line.get("turn_id")
        if turn_id is None:
            continue
        iteration = line.get("iteration") or 0
        if turn_id not in last or iteration >= last[turn_id][0]:
            last[turn_id] = (iteration, str(line["outcome"]))
    return [kind for _, kind in last.values()]


def _nearest_rank(ordered: Sequence[float], percent: int) -> float | None:
    """The smallest sorted value with `percent`% of the values at or below it: one that happened."""
    if not ordered:
        return None
    rank = -(-percent * len(ordered) // 100)  # the ceiling, in integers
    return ordered[max(rank, 1) - 1]


def _block(summary: Summary) -> list[str]:
    served = ", ".join(sorted(_SERVED.get(summary.route, _ONLY_OK)))
    outcomes = ", ".join(f"{kind} {count}" for kind, count in summary.outcomes.items())
    share = summary.cache_read_share
    rows = [
        f"{summary.route} / {summary.model}  [{summary.origin}]",
        f"  calls      {summary.calls}: {outcomes}",
        f"  failed     {summary.failed} of {summary.calls} ({summary.failure_rate:.1%});"
        f" served on {summary.route}: {served}",
        f"  latency    {_latency(summary)}",
        f"  tokens     in {summary.input_tokens:,}, out {summary.output_tokens:,},"
        f" cache read {summary.cache_read_tokens:,}, cache write {summary.cache_write_tokens:,}",
        "  cache      "
        + (f"{share:.1%} of prompt tokens read from the cache" if share is not None else "-"),
        f"  cost       {_cost(summary)}",
    ]
    if summary.unpriced:
        rows.append(
            f"  unpriced   {sum(summary.unpriced.values())} ({_named(summary.unpriced)}),"
            " not in the cost"
        )
    if summary.turns:
        rows.append(
            f"  turns      {summary.turns}, failed {summary.failed_turns}"
            f" ({summary.failed_turns / summary.turns:.1%})"
        )
    if summary.served_by_other:
        rows.append(f"  served by another model: {_named(summary.served_by_other)}")
    return rows


def _latency(summary: Summary) -> str:
    if summary.latency_p50_s is None or summary.latency_p95_s is None:
        return "none answered"
    return (
        f"p50 {summary.latency_p50_s:.2f} s, p95 {summary.latency_p95_s:.2f} s,"
        f" over {summary.answered} answered"
    )


def _cost(summary: Summary) -> str:
    per_call = summary.cost_per_call_usd
    if per_call is None:
        return "none priced"
    return f"${summary.cost_usd:.6f} total, ${per_call:.6f} per call, over {summary.priced} priced"


def _total(summaries: Sequence[Summary]) -> str:
    unpriced: Counter[str] = Counter()
    for summary in summaries:
        unpriced.update(summary.unpriced)
    text = (
        f"Total: {sum(s.calls for s in summaries)} calls,"
        f" ${sum(s.cost_usd for s in summaries):.6f} over {sum(s.priced for s in summaries)} priced"
    )
    if unpriced:
        text += f"; {sum(unpriced.values())} unpriced ({_named(unpriced)}), not in the total"
    return text + "."


def _named(counts: Mapping[str, int]) -> str:
    return ", ".join(f"{name} {count}" for name, count in sorted(counts.items()))


def _kept(line_origin: str, wanted: OriginFilter) -> bool:
    if wanted == "all":
        return True
    if wanted == "eval":
        return line_origin.startswith("eval:")
    return line_origin == DEFAULT_ORIGIN


def _readable(line: Any) -> bool:
    """Whether a parsed line has every field the summary reads, each of the type `record` writes."""
    if not isinstance(line, dict):
        return False
    if not all(_text(line.get(key)) for key in ("origin", "route", "requested_model", "outcome")):
        return False
    usage = line.get("usage")
    return (
        _when(line.get("ts")) is not None
        and _number(line.get("latency_s"))
        and (line.get("cost_usd") is None or _number(line["cost_usd"]))
        and (
            usage is None
            or (
                isinstance(usage, dict)
                and all(_count(usage.get(key)) is not None for key in _USAGE_KEYS)
            )
        )
        and (line.get("served_model") is None or _text(line["served_model"]) is not None)
        and (line.get("turn_id") is None or _text(line["turn_id"]) is not None)
        and (line.get("iteration") is None or _count(line["iteration"]) is not None)
    )


def _when(value: Any) -> datetime | None:
    """An ISO date or time as an aware datetime, UTC when it names no zone; `None` if not one."""
    if not isinstance(value, str):
        return None
    try:
        moment = datetime.fromisoformat(value)
    except ValueError:
        return None
    return moment if moment.tzinfo is not None else moment.replace(tzinfo=UTC)


def _since(value: str) -> datetime:
    moment = _when(value)
    if moment is None:
        raise argparse.ArgumentTypeError(f"not a date or an ISO time: {value!r}")
    return moment


def _number(value: Any) -> bool:
    return isinstance(value, int | float) and not isinstance(value, bool)


if __name__ == "__main__":
    raise SystemExit(main())
