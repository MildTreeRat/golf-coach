"""The eval runner: one flow's cases on one model, graded, in build-eval.md's contract. [M43 P7, P8]

    python -m golf_coach.evals.runner --flow club_spec --model claude-opus-5 \\
        [--variant baseline|v<N>] [--reps N] [--limit K] [--concurrency N] [--timeout-s S] \\
        [--runs-dir DIR] [--approve-harness]

A run is a command the user approves and never a test (ADR-036 §5): every case is a paid request.
The default `pytest` runs this module on fake flows and fake clients (`tests/evals/`).

## What it writes

Under `<runs dir>/<flow>/<variant>/`, where the runs dir is `.claude/hillclimb/` at the repo root
(ADR-036 §9) and the variant is exactly `baseline` or `v<N>`. The report builder ignores any other
name, and a `v<N>` is refused without a `change.md` in its directory.

- **`results.jsonl`**, one row per scored (case, rep), appended as each attempt finishes so a crash
  keeps what is done. A row has `prompt_id`, `rep`, `prompt`, `tags` (`tags[0]` is the report's
  grouping), `meta`, `model` (the response's, never the config's), `usage`, `stop_reason`,
  `status` (`ok`, or `truncated` at `max_tokens`, which is not graded and stays out of the means),
  `latency_s`, `tool_calls` and `grade` (a `{metric_id: number}` dict, with `explanation` beside it
  when the flow gives one). **A grade holds every declared metric except one declared
  `always=False`, which it may leave out** where the share has nothing to divide by on that case
  (the club-spec flow's, M43 P12, the user's call): absent and never a stand-in number, which would
  move every mean it joined. That departs from build-eval.md's "every id on every case's grade"
  for those metrics only, and the summary and the lite report average the rows that hold one, so
  each metric's case count is its own. `meta` carries the case's own meta plus `cost_usd` (priced
  through `llm_log`'s table, the judge's part included) and `retries` (the backoff's, below), and
  `failure_class: "refusal"` when the model refused. A refusal is a graded outcome, not an error
  (eval-audit.md §2). A flow that grades with a model adds `judge_model`, `judge_usage`,
  `meta.judge_cost_usd` and `meta.judge_retries`.
- **`traces/<id>_rep<k>.json`**, the conversation in SCHEMA.md's turn format (`system`, `user`,
  `assistant` with `thinking`, `tool_call`, `tool_result`), built from the requests the flow made.
  It is written before its row, so every row has its trace.
- **`errors.jsonl`**, one line per failed attempt, never a row in `results.jsonl`: a row would hold
  the (case, rep) slot, so resume would never re-run it, and it would score plumbing as the model's
  failure. A line has `failure_class`, `error`, `retries`, `judge_retries`, `latency_s`, and
  `model`, `usage` and `cost_usd` when a request completed (billed-but-failed spend is still
  spend). A served dated snapshot is priced as the model requested, as on a row.

It reads `<runs dir>/<flow>/_state.json`, the hillclimb loop's state (eval-hillclimb.md), and writes
it only to record an approved harness (below).

**Resume** is at the (case, rep) key. A rerun skips every pair with a row and re-runs every pair
without one, so an errored attempt runs again and its old error line stays (the file is
append-only, and a later row supersedes it).

## How a flow plugs in

A flow is a `Flow` exposed as `FLOW` by `golf_coach.evals.<flow>`. It loads its `Case`s, runs one
(`run`, which calls the real route with `client=ctx.client` and `llm_model=ctx.model`), and grades
the output (`grade`, returning a `Grade`). It declares its `metrics` in the order the report wants
them: the headline is the first `binary` metric, else the first.

`ctx.client` is a `RecordingClient` over the runner's client, one per attempt, and that is how the
runner measures a route that returns only its answer or a note. It keeps every
`messages.create` request and its response or exception, and re-raises the exception unchanged, so
the route meets exactly what it would have met. The row's `model`, `usage`, `stop_reason`,
`latency_s` and trace come from those recorded calls, not from the flow. A grader that calls a
model uses `ctx.judge_client` and declares `judge_model`.

## Failure classes

What the runner decides itself:

- `serving_error`: a recorded request raised a 408, 409, 429 or 5xx (a 529 is overloaded), or a
  connection error or timeout. The route turned it into a note, and the runner reads the cause off
  the call instead of the note's wording.
- `harness_error`: any other request error (auth, a bad request, a missing model); a flow or grader
  that raised; a grade that does not match the declared metrics; a measurement that is missing or
  zero.
- `serving_substitution`: the response's model is not the one requested. A documented
  alias-to-snapshot resolution (`foo` served as `foo-20260101`) passes, as in `runner-scaffold.mjs`.

- `timeout`: the attempt passed its wall-clock ceiling (below).

What a flow raises as `AttemptFailed(cls, message)`: `refusal` (for a grader that cannot score, as
the judge's refusal in ADR-036 §8), `genuine_failure` (no scorable output, and the model's doing)
and the classes above.

## Retries [M43 P8]

A `serving_error` is a provider's passing trouble, so the try is sent again after a jittered
backoff, as `runner-scaffold.mjs` does it: `min(60, 2**k)` seconds times a jitter of 0.5 to 1.5
before the k-th retry (k from 0), and five tries at most. A route turns a 429 into a note, so the
backoff keys on the exception the recorder kept, which is that note's cause: `rate_limited` (429),
`overloaded` (529), `connection_failed`, and an `api_error` with a 408, 409 or 5xx. No other note is
retried. The client has `max_retries=0`, so these are the only retries, and every one is counted.

**Scoring is strict per attempt.** A graded output is final: no case is re-run for its grade, so a
refusal, a truncation or a wrong answer stands as it came. Only a try that produced nothing is sent
again, and only when none of its requests was answered, so a retry never throws billed work away.
A serving error after an answered request is an error line carrying that spend, and a resume re-runs
it. The row's grade, usage, trace and latency all come from the one try that answered, and
`meta.retries` counts the tries sent before it. The judge backs off on its own, counted in
`judge_retries`, and retrying the judge never re-sends the app's request.

## The ceiling [M43 P8]

`--timeout-s` (1800 unless given; 0 for none) is a hard wall-clock ceiling on each (case, rep): the
app's tries, the backoff between them and the grading, together. When it passes, the attempt is an
`errors.jsonl` line of class `timeout`, never a zero, carrying what was billed by then, and its slot
goes to the next case. A thread cannot be stopped, so a request in flight runs on in the background
and whatever it returns is dropped. The attempt sends nothing more: no retry and no judge request.
A route's own call-log line for that request still lands, if it finishes before the process exits.
A retry whose backoff would end past the ceiling is not started, and the attempt is the serving
error it met.

## The harness gate [M43 P8]

A sha256 over the harness, computed by `runner-scaffold.mjs`'s rule: this file, the flow's module,
any Python lockfile beside this file or at the repo root, and every path in `_state.json`'s
`harness_paths` (relative to the repo root). The files are taken in order of their repo-relative
name, and each adds its name, a NUL, its bytes and a NUL. A listed path that cannot be read is
skipped with a warning. When `_state.json` has no `harness_sha`, or a different one, the run is
refused (exit 2) until it is run once with `--approve-harness`, which records the new sha and goes
on. **That flag is the user's, never an agent's** (ADR-036 §6): passing it says a person has read
the change. It is a change detector, not a security boundary. It covers no installed package,
interpreter or SDK version, and no file it is not given.

The runner also refuses a `_state.json` whose `metrics` do not list the flow's metric ids in the
flow's order, with the same kind wherever one is given. The report draws `_state.json`'s columns,
so a mismatch would leave a column blank, and the headline is the first binary one.

**A zero is a bug until proven otherwise** (build-eval.md, "Trusting a zero"). An answered request
with no usage, zero prompt or output tokens, a cost that is not above `$0`, a `0.0` latency or no
`model` on its response is not written as a row. Its attempt is a `harness_error` and the run
**stops**: no new case starts, and the exit code is 3. A metric that is constant across every
scored row is a warning in the summary rather than a stop, because an oracle or a null run is
exactly that.

## The key

The runner builds `anthropic.Anthropic(max_retries=0)`, so every retry is the runner's and visible.
That resolves `ANTHROPIC_API_KEY`, `ANTHROPIC_AUTH_TOKEN` or an `ant auth` profile, as the SDK
does. When it finds none, the runner falls back to `settings.anthropic_api_key`
(`GOLF_ANTHROPIC_API_KEY`) and unwraps it here, at the entry point (ADR-036 §5, ADR-019's M43
addendum). Each route is handed the client through `client=`, so its own key gate never runs.

## The summary

The headline metric's mean over cases (each case the mean of its `ok` reps, the report builder's
way), with a 95% interval: Wilson's score interval for a binary metric, and the normal one over
the per-case means otherwise. Then the other metrics, the row and error counts by class, the backoff
retries, and the measured cost (`not measured` when a row could not be priced, never `$0`).

## Exit codes

0 every attempt scored; 1 at least one attempt failed; 2 refused before any request (bad
arguments, an unknown flow, a model with no price, a `v<N>` without `change.md`, a `_state.json`
that does not parse or whose metrics are not the flow's, a harness not approved, bad cases, no
credential); 3 stopped on a zero.
"""

from __future__ import annotations

import argparse
import hashlib
import importlib
import inspect
import json
import math
import os
import queue
import random
import re
import sys
import threading
from abc import ABC, abstractmethod
from collections import Counter, deque
from collections.abc import Callable, Mapping, Sequence
from dataclasses import dataclass, field, replace
from pathlib import Path
from time import perf_counter
from types import ModuleType
from typing import Any, Literal, TypeVar, get_args

from golf_coach import llm_log
from golf_coach.config import REPO_ROOT, settings

RUNS_DIR = REPO_ROOT / ".claude" / "hillclimb"
RESULTS_FILE = "results.jsonl"
ERRORS_FILE = "errors.jsonl"
CHANGE_FILE = "change.md"
TRACES_DIR = "traces"
#: The hillclimb loop's state, in the claude-api skill's format (eval-hillclimb.md), not ours.
STATE_FILE = "_state.json"
#: The full viewer's legend truncates a longer label (build-eval.md Step 3).
LABEL_LIMIT = 14

#: The scaffold's backoff: five tries, `min(60, 2**k)` s before retry k, times a 0.5-1.5 jitter.
MAX_TRIES = 5
BACKOFF_BASE_S = 1.0
BACKOFF_CAP_S = 60.0
#: The scaffold's default ceiling on one (case, rep), in seconds; 0 means none.
DEFAULT_TIMEOUT_S = 1800.0
#: Python's lockfiles. Dependency drift is harness drift too (runner-scaffold.mjs).
LOCKFILES = ("uv.lock", "poetry.lock", "pdm.lock", "Pipfile.lock", "pylock.toml")
#: How long the main thread waits at most between looks at the ceilings (Ctrl-C stays live).
_POLL_S = 1.0

EXIT_OK = 0
EXIT_FAILED = 1
EXIT_REFUSED = 2
EXIT_STOPPED = 3

FailureClass = Literal[
    "serving_error",
    "harness_error",
    "serving_substitution",
    "refusal",
    "timeout",
    "genuine_failure",
]
FAILURE_CLASSES: frozenset[str] = frozenset(get_args(FailureClass))
MetricKind = Literal["binary", "float", "judge"]

_FLOW_NAME = re.compile(r"[a-z][a-z0-9_]*")
_VARIANT = re.compile(r"baseline|v[1-9][0-9]*")
_METRIC_ID = re.compile(r"[a-z][a-z0-9_]*")
#: A case id is a row key and a file name, so it is checked rather than rewritten.
_CASE_ID = re.compile(r"[A-Za-z0-9][A-Za-z0-9._-]{0,119}")
#: What a served model may add to the requested one: a dated snapshot, as the scaffold allows.
_SNAPSHOT = re.compile(r"[-@](?:[0-9]{8}|[0-9]{4}-[0-9]{2}-[0-9]{2})")
_ALIAS_SUFFIX = re.compile(r"-latest$|-0$")
_CONTROL = re.compile(r"[\x00-\x1f\x7f-\x9f]")
_ERROR_LIMIT = 500
_Z95 = 1.959963984540054
_USAGE_KEYS = (
    "input_tokens",
    "output_tokens",
    "cache_read_input_tokens",
    "cache_creation_input_tokens",
)
_NO_USAGE = dict.fromkeys(_USAGE_KEYS, 0)
_TOOL_BLOCKS = frozenset({"tool_use", "server_tool_use"})
#: Status codes a provider returns for a moment and not for good (the SDK's own retry set).
_TRANSIENT_STATUS = frozenset({408, 409, 429})

_T = TypeVar("_T")
Stage = Literal["app", "judge"]


# --- what a flow declares and returns -------------------------------------------------------


@dataclass(frozen=True)
class Metric:
    """One column of the report. `id` is the key in a row's `grade` (SCHEMA.md `Metric`).

    `always=False` lets a grade leave the metric out, for a share that has nothing to divide by on
    some cases (the club-spec flow's invented rate on a set whose maker printed every field). It is
    this runner's, not SCHEMA.md's, so it never reaches `_state.json`.
    """

    id: str
    label: str
    kind: MetricKind
    better: Literal["higher", "lower"] = "higher"
    scale: float | None = None
    always: bool = True


@dataclass(frozen=True)
class Case:
    """One input. `prompt` is the text the report shows; `input` is whatever the flow needs."""

    id: str
    prompt: str
    tags: tuple[str, ...]
    input: Any = None
    meta: Mapping[str, Any] = field(default_factory=dict)


@dataclass(frozen=True)
class Grade:
    """Every declared metric's score for one output, and an optional reason per metric."""

    scores: Mapping[str, float]
    explanation: Mapping[str, str] | None = None


class AttemptFailed(Exception):
    """An attempt with no scorable output. It becomes an `errors.jsonl` line, never a zero."""

    def __init__(
        self, failure_class: FailureClass, message: str, *, error_class: str | None = None
    ) -> None:
        if failure_class not in FAILURE_CLASSES:
            raise ValueError(f"unknown failure class {failure_class!r}")
        super().__init__(message)
        self.failure_class: FailureClass = failure_class
        self.error_class = error_class


@dataclass
class Call:
    """One `messages.create` request, and its response or the exception it raised."""

    request: dict[str, Any]
    response: Any = None
    error: BaseException | None = None
    latency_s: float = 0.0


class RecordingClient:
    """What a flow hands a route as `client=`: the runner's client, with each request kept.

    Only `messages.create` exists on it. A route that reached for anything else would make a
    request the row could not account for, so it fails with an `AttributeError` instead.
    """

    def __init__(self, inner: Any) -> None:
        self._inner = inner
        self.calls: list[Call] = []
        self.messages = _RecordingMessages(self)


class _RecordingMessages:
    def __init__(self, owner: RecordingClient) -> None:
        self._owner = owner

    def create(self, **request: Any) -> Any:
        call = Call(request=request)
        self._owner.calls.append(call)
        started = perf_counter()
        try:
            call.response = self._owner._inner.messages.create(**request)
        except Exception as exc:
            call.error = exc
            raise
        finally:
            call.latency_s = perf_counter() - started
        return call.response


@dataclass(frozen=True)
class CaseContext:
    """What one attempt gives the flow: the run's model and two recorders over one client."""

    model: str
    variant: str
    rep: int
    client: RecordingClient
    judge_client: RecordingClient


class Flow(ABC):
    """One eval: its cases, how to run one, how to grade one, and the metrics it reports."""

    #: In report order. The headline is the first `binary` metric, else the first.
    metrics: Sequence[Metric] = ()
    #: The model a grader calls through `ctx.judge_client`, or None for code-only grading.
    judge_model: str | None = None

    @abstractmethod
    def load_cases(self) -> Sequence[Case]:
        """Every case, in a stable order (`--limit` takes the first K)."""

    @abstractmethod
    def run(self, case: Case, ctx: CaseContext) -> Any:
        """Call the real route with `client=ctx.client` and `ctx.model`; return what it gave."""

    @abstractmethod
    def grade(self, case: Case, output: Any, ctx: CaseContext) -> Grade:
        """Score `output` on every declared metric. Raise `AttemptFailed` when it cannot."""


# --- the command ----------------------------------------------------------------------------


@dataclass(frozen=True)
class Options:
    flow: str
    model: str
    variant: str = "baseline"
    reps: int = 1
    limit: int | None = None
    concurrency: int = 4
    runs_dir: Path = RUNS_DIR
    timeout_s: float = DEFAULT_TIMEOUT_S
    #: The user's flag, never an agent's (ADR-036 §6).
    approve_harness: bool = False

    @property
    def label(self) -> str:
        return f"{self.flow}/{self.variant}"


class _Refused(Exception):
    """A reason not to start. Raised before any request, so nothing is spent."""


class _Unmeasured(Exception):
    """A measurement missing or zero on an answered request: a runner bug, so the run stops."""


def main(
    argv: Sequence[str] | None = None,
    *,
    flows: Mapping[str, Flow] | None = None,
    client: Any = None,
) -> int:
    """Run the command. `flows` and `client` replace the flow lookup and the SDK, for tests."""
    options = _parse(argv)
    try:
        return run(options, flows=flows, client=client)
    except _Refused as refused:
        _say(f"refusing to run: {refused}")
        return EXIT_REFUSED
    except OSError as exc:
        _say(f"stopped: {exc}. Rows already written are kept, and a rerun resumes after them")
        return EXIT_FAILED


def run(options: Options, *, flows: Mapping[str, Flow] | None = None, client: Any = None) -> int:
    """Run `options`' flow and print its summary. Returns the exit code."""
    flow = _flow(options.flow, flows)
    _check_metrics(flow.metrics)
    for model in (options.model, flow.judge_model):
        if model is not None and llm_log.cost_usd(model, _NO_USAGE) is None:
            raise _Refused(
                f"{model!r} has no price in llm_prices.json, so no row could carry a measured cost"
            )
    paths = _Paths(options.runs_dir / options.flow / options.variant)
    _prepare(paths, options.variant)
    state = _read_state(paths.state)
    _check_state_metrics(state, flow.metrics, paths.state)
    _check_harness(paths.state, state, flow, approve=options.approve_harness)
    _make(paths)
    cases = _cases(flow)
    if options.limit is not None:
        cases = cases[: options.limit]
    finished = _finished(paths.results)
    tasks = [
        (case, rep)
        for case in cases
        for rep in range(options.reps)
        if (case.id, rep) not in finished
    ]
    _say(f"[{options.label}] {len(tasks)} of {len(cases) * options.reps} (case, rep) to run")

    failed, stopped = 0, False
    if tasks:
        if client is None:
            client, source = _build_client(_sdk())
            _say(f"[{options.label}] credential: {source}")
        previous = llm_log.origin()
        # Before the first request: every line the routes log from here is this run's (§3).
        llm_log.set_origin(f"eval:{options.flow}/{options.variant}")
        try:
            failed, stopped = _execute(flow, tasks, options, client, paths)
        finally:
            llm_log.set_origin(previous)

    summary = summarize(paths.variant_dir, flow.metrics)
    for line in render_summary(options.label, options.model, summary):
        print(line)
    for metric_id in summary.constant:
        _say(
            f"warning: {metric_id} is the same on every scored row. A constant metric is a bug "
            "until proven otherwise; an oracle or a null run is the expected exception"
        )
    if stopped:
        return EXIT_STOPPED
    return EXIT_FAILED if failed else EXIT_OK


def _parse(argv: Sequence[str] | None) -> Options:
    parser = argparse.ArgumentParser(
        prog="python -m golf_coach.evals.runner",
        description="Run one eval flow on one model (M43, ADR-036 §5). Every case is a paid call.",
    )
    parser.add_argument("--flow", required=True, type=_matching(_FLOW_NAME, "a flow name"))
    parser.add_argument("--model", required=True)
    parser.add_argument(
        "--variant", default="baseline", type=_matching(_VARIANT, "'baseline' or 'v<N>'")
    )
    parser.add_argument("--reps", type=_positive, default=1)
    parser.add_argument("--limit", type=_positive, default=None, help="the first K cases only")
    parser.add_argument("--concurrency", type=_positive, default=4)
    parser.add_argument(
        "--timeout-s",
        type=_seconds,
        default=DEFAULT_TIMEOUT_S,
        help="the wall-clock ceiling on one (case, rep), in seconds; 0 for none",
    )
    parser.add_argument("--runs-dir", type=Path, default=RUNS_DIR)
    parser.add_argument(
        "--approve-harness",
        action="store_true",
        help="the user's flag, never an agent's: record the harness sha after reading the change",
    )
    args = parser.parse_args(argv)
    return Options(
        flow=args.flow,
        model=args.model,
        variant=args.variant,
        reps=args.reps,
        limit=args.limit,
        concurrency=args.concurrency,
        runs_dir=args.runs_dir,
        timeout_s=args.timeout_s,
        approve_harness=args.approve_harness,
    )


def _matching(pattern: re.Pattern[str], what: str) -> Any:
    def check(value: str) -> str:
        if not pattern.fullmatch(value):
            raise argparse.ArgumentTypeError(f"must be {what}, got {value!r}")
        return value

    return check


def _positive(value: str) -> int:
    try:
        number = int(value)
    except ValueError:
        number = 0
    if number < 1:
        raise argparse.ArgumentTypeError(f"must be a whole number of at least 1, got {value!r}")
    return number


def _seconds(value: str) -> float:
    try:
        number = float(value)
    except ValueError:
        number = -1.0
    # A wait longer than the platform's maximum raises, so it is refused here, before any request.
    if not math.isfinite(number) or not 0 <= number <= threading.TIMEOUT_MAX:
        raise argparse.ArgumentTypeError(f"must be 0 (no ceiling) or more seconds, got {value!r}")
    return number


# --- before the first request ---------------------------------------------------------------


def _flow(name: str, flows: Mapping[str, Flow] | None) -> Flow:
    if flows is not None:
        found: Any = flows.get(name)
    else:
        module_name = f"golf_coach.evals.{name}"
        try:
            module = importlib.import_module(module_name)
        except ModuleNotFoundError as exc:
            if exc.name != module_name:
                raise
            raise _Refused(f"there is no flow named {name!r} ({module_name})") from exc
        found = getattr(module, "FLOW", None)
    if not isinstance(found, Flow):
        raise _Refused(f"there is no flow named {name!r} (no FLOW that is a runner.Flow)")
    return found


def _check_metrics(metrics: Sequence[Metric]) -> None:
    if not metrics:
        raise _Refused("the flow declares no metrics")
    seen: set[str] = set()
    for metric in metrics:
        if not _METRIC_ID.fullmatch(metric.id) or metric.id in seen:
            raise _Refused(f"metric id {metric.id!r} is not a unique snake_case id")
        if not 0 < len(metric.label) <= LABEL_LIMIT:
            raise _Refused(f"metric {metric.id!r}'s label must be 1-{LABEL_LIMIT} characters")
        if metric.kind not in get_args(MetricKind):
            raise _Refused(f"metric {metric.id!r} has unknown kind {metric.kind!r}")
        seen.add(metric.id)


@dataclass(frozen=True)
class _Paths:
    variant_dir: Path

    @property
    def results(self) -> Path:
        return self.variant_dir / RESULTS_FILE

    @property
    def errors(self) -> Path:
        return self.variant_dir / ERRORS_FILE

    @property
    def traces(self) -> Path:
        return self.variant_dir / TRACES_DIR

    @property
    def state(self) -> Path:
        return self.variant_dir.parent / STATE_FILE

    def trace(self, case_id: str, rep: int) -> Path:
        return self.traces / f"{case_id}_rep{rep}.json"


def _prepare(paths: _Paths, variant: str) -> None:
    """Refuse a planted link or a `v<N>` with no rationale."""
    flow_dir = paths.variant_dir.parent
    for path in (
        flow_dir,
        paths.state,
        paths.variant_dir,
        paths.traces,
        paths.results,
        paths.errors,
    ):
        if path.is_symlink():
            raise _Refused(f"{path} is a symlink; the run directory must hold plain files")
    if variant != "baseline":
        change = paths.variant_dir / CHANGE_FILE
        if not change.is_file() or not change.read_text(encoding="utf-8").strip():
            raise _Refused(
                f"{change} is missing or empty. A {variant} run needs its change.md, whose first "
                "line says what changed (build-eval.md)"
            )


def _make(paths: _Paths) -> None:
    """The directories, and a newline after a torn last line."""
    paths.traces.mkdir(parents=True, exist_ok=True)
    for path in (paths.results, paths.errors):
        _end_with_newline(path)


# --- _state.json and the harness gate --------------------------------------------------------


def _read_state(path: Path) -> dict[str, Any]:
    """The loop's state, read-only here. Absent is an empty state; unparseable is refused."""
    if not path.exists():
        return {}
    try:
        state = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError) as exc:
        # The parser's message is not echoed: it can quote the file's bytes.
        raise _Refused(f"{path} is not valid JSON ({type(exc).__name__}); fix it first") from exc
    if not isinstance(state, dict):
        raise _Refused(f"{path} holds {type(state).__name__}, not a JSON object")
    return state


def _check_state_metrics(state: Mapping[str, Any], metrics: Sequence[Metric], path: Path) -> None:
    """`_state.json`'s `metrics`, when given, are the report's columns: they must be the flow's."""
    declared = state.get("metrics")
    if declared is None:
        return
    ids: list[Any] | None = None
    if isinstance(declared, list):
        ids = [
            entry
            if isinstance(entry, str)
            else (entry.get("id") if isinstance(entry, dict) else None)
            for entry in declared
        ]
    expected = [metric.id for metric in metrics]
    kinds_differ = ids == expected and any(
        isinstance(entry, dict) and entry.get("kind") not in (None, metric.kind)
        for entry, metric in zip(declared, metrics, strict=True)
    )
    if ids != expected or kinds_differ:
        flow_metrics = [f"{metric.id} ({metric.kind})" for metric in metrics]
        raise _Refused(
            f"{path}'s metrics are not the flow's {flow_metrics} in that order. The report draws "
            "_state.json's columns, so the rest would be blank, and its headline is the first "
            "binary one"
        )


@dataclass(frozen=True)
class HarnessDigest:
    """The harness sha and what went into it, by repo-relative name."""

    sha: str
    hashed: list[str]
    skipped: list[str]
    lockfiles: list[str]


def harness_digest(state: Mapping[str, Any], flow: Flow) -> HarnessDigest:
    """`runner-scaffold.mjs`'s sha: each file's name, NUL, bytes, NUL, in name order.

    The files are this module, the flow's module (the scaffold's runner holds its flow), any
    Python lockfile beside this module or at the repo root, and `state["harness_paths"]`, each
    relative to the repo root unless absolute.
    """
    listed = state.get("harness_paths", [])
    if not isinstance(listed, list) or not all(isinstance(p, str) and p for p in listed):
        raise _Refused("_state.json's harness_paths must be a list of paths")
    runner_file = Path(__file__).resolve()
    try:
        flow_file = Path(inspect.getfile(type(flow))).resolve()
    except (TypeError, OSError) as exc:
        raise _Refused("the flow's module has no source file, so no harness sha") from exc
    lockfiles = {
        (folder / name).resolve()
        for folder in (runner_file.parent, REPO_ROOT)
        for name in LOCKFILES
        if (folder / name).is_file()
    }
    files = {runner_file, flow_file, *lockfiles}
    files.update((REPO_ROOT / p).resolve() for p in listed)
    digest = hashlib.sha256()
    hashed: list[str] = []
    skipped: list[str] = []
    for name, path in sorted((_harness_name(path), path) for path in files):
        try:
            data = path.read_bytes()
        except OSError:
            if path == runner_file:
                raise
            skipped.append(name)
            continue
        digest.update(name.encode("utf-8") + b"\0" + data + b"\0")
        hashed.append(name)
    return HarnessDigest(
        sha=digest.hexdigest(),
        hashed=hashed,
        skipped=skipped,
        lockfiles=sorted(_harness_name(path) for path in lockfiles),
    )


def _harness_name(path: Path) -> str:
    try:
        return Path(os.path.relpath(path, REPO_ROOT)).as_posix()
    except ValueError:  # another drive, on Windows
        return path.as_posix()


def _check_harness(path: Path, state: dict[str, Any], flow: Flow, *, approve: bool) -> None:
    """Go on when the harness is the approved one, or record it on the user's approval."""
    digest = harness_digest(state, flow)
    for name in digest.skipped:
        _say(f"warning: harness path {name} is not readable, so it is not in the sha")
    approved = state.get("harness_sha")
    if approved == digest.sha:
        return
    if not digest.lockfiles:
        _say(
            "note: no lockfile beside the runner or at the repo root, so a dependency change is "
            "outside the harness sha"
        )
    files = ", ".join(digest.hashed)
    if approve:
        _write_state(path, {**state, "harness_sha": digest.sha})
        _say(
            f"harness approved: sha256 {digest.sha[:12]} over {len(digest.hashed)} file(s), "
            f"recorded in {path}"
        )
        return
    if approved is None:
        raise _Refused(
            f"no approved harness sha in {path} (computed {digest.sha[:12]} over: {files}). "
            "Review the harness, then run once with --approve-harness to record it. That flag is "
            "the user's, never an agent's"
        )
    raise _Refused(
        f"the harness changed since it was last approved (files: {files}); approved "
        f"{str(approved)[:12]}, now {digest.sha[:12]}. Re-run with --approve-harness after "
        "reviewing the diff. That flag is the user's, never an agent's"
    )


def _write_state(path: Path, state: Mapping[str, Any]) -> None:
    """Whole or not at all: a torn `_state.json` would refuse every later run."""
    path.parent.mkdir(parents=True, exist_ok=True)
    staged = path.with_name(path.name + ".tmp")
    if staged.is_symlink():
        raise _Refused(f"{staged} is a symlink; the run directory must hold plain files")
    staged.write_text(
        json.dumps(state, indent=2, ensure_ascii=False) + "\n", encoding="utf-8", newline="\n"
    )
    os.replace(staged, path)


def _cases(flow: Flow) -> list[Case]:
    cases = list(flow.load_cases())
    if not cases:
        raise _Refused("the flow loaded no cases")
    seen: dict[str, str] = {}
    for case in cases:
        if not isinstance(case, Case):
            raise _Refused(f"the flow loaded {type(case).__name__}, not a Case")
        if not _CASE_ID.fullmatch(case.id):
            raise _Refused(
                f"case id {case.id!r} is not a file-safe id (letters, digits, '.', '_', '-')"
            )
        # Lower-cased because Windows and macOS file systems would merge the two traces.
        twin = seen.get(case.id.lower())
        if twin is not None:
            raise _Refused(f"case ids {twin!r} and {case.id!r} collide")
        seen[case.id.lower()] = case.id
        if not isinstance(case.prompt, str) or not case.prompt.strip():
            raise _Refused(f"case {case.id!r} has no prompt text")
        tags = case.tags
        if (
            not isinstance(tags, tuple | list)
            or not tags
            or not all(isinstance(tag, str) and tag for tag in tags)
        ):
            raise _Refused(f"case {case.id!r} needs at least one tag; tags[0] groups the report")
        try:
            json.dumps(dict(case.meta))
        except (TypeError, ValueError) as exc:
            raise _Refused(f"case {case.id!r} has meta that is not JSON: {exc}") from exc
    return cases


def _finished(results: Path) -> set[tuple[str, int]]:
    """Every (case, rep) with a row. An unreadable line holds no slot."""
    done: set[tuple[str, int]] = set()
    for row in _rows(results):
        prompt_id, rep = row.get("prompt_id"), row.get("rep")
        if isinstance(prompt_id, str) and isinstance(rep, int):
            done.add((prompt_id, rep))
    return done


def _sdk() -> ModuleType:
    try:
        return importlib.import_module("anthropic")
    except ImportError as exc:
        raise _Refused("the `llm` extra is not installed (pip install -e .[llm])") from exc


def _build_client(sdk: Any) -> tuple[Any, str]:
    """The run's client, with no retries of its own, and where its credential came from.

    The SDK resolves its own environment and an `ant auth` profile first. Only when it finds
    nothing is the app's key unwrapped, here at the entry point (ADR-019's M43 addendum).
    """
    client = sdk.Anthropic(max_retries=0)
    if getattr(client, "api_key", None):
        return client, "ANTHROPIC_API_KEY"
    if getattr(client, "auth_token", None):
        return client, "ANTHROPIC_AUTH_TOKEN"
    if getattr(client, "credentials", None) is not None:
        return client, "an ant auth profile"
    key = settings.anthropic_api_key
    if key is None:
        raise _Refused(
            "no API credential: set ANTHROPIC_API_KEY or GOLF_ANTHROPIC_API_KEY, or sign in with "
            "`ant auth login`"
        )
    return sdk.Anthropic(api_key=key.get_secret_value(), max_retries=0), "GOLF_ANTHROPIC_API_KEY"


# --- the attempts ---------------------------------------------------------------------------


@dataclass(frozen=True)
class _Spend:
    """What a list of calls measured: the served model, summed usage, cost and request time."""

    model: str | None
    usage: dict[str, int] | None
    cost_usd: float | None
    latency_s: float
    stop_reason: str | None
    tool_calls: int


@dataclass(frozen=True)
class _Scored:
    case_id: str
    rep: int
    row: dict[str, Any]
    transcript: list[dict[str, Any]]


@dataclass(frozen=True)
class _Failed:
    case_id: str
    rep: int
    row: dict[str, Any]
    stops: bool = False


_Outcome = _Scored | _Failed | None


@dataclass(eq=False)
class _Attempt:
    """One (case, rep): every try it sent, its retries, its ceiling, and who settles it.

    The worker thread runs it and the main thread watches its ceiling. Exactly one of them settles
    it (`settle`): the worker with its outcome, or the main thread with a timeout. The other's
    outcome is dropped.
    """

    case: Case
    rep: int
    model: str
    judge_model: str | None
    variant: str
    started: float
    deadline: float | None
    app_clients: list[RecordingClient] = field(default_factory=list)
    judge_clients: list[RecordingClient] = field(default_factory=list)
    retries: int = 0
    judge_retries: int = 0
    #: Set when the ceiling took the attempt: it sends nothing more.
    abandoned: threading.Event = field(default_factory=threading.Event)
    _lock: threading.Lock = field(default_factory=threading.Lock)
    _settled: bool = False

    def settle(self) -> bool:
        """Claim the attempt's one outcome. False when the other thread already has."""
        with self._lock:
            if self._settled:
                return False
            self._settled = True
            return True

    def expire(self) -> bool:
        """The ceiling's claim: true when it took the attempt before the worker finished."""
        if not self.settle():
            return False
        self.abandoned.set()
        return True

    def expired(self) -> bool:
        return self.abandoned.is_set() or (
            self.deadline is not None and perf_counter() >= self.deadline
        )

    def overdue(self, now: float) -> bool:
        return self.deadline is not None and now >= self.deadline

    def context(self, client: Any) -> CaseContext:
        """A fresh pair of recorders for a new try of the app."""
        ctx = CaseContext(
            model=self.model,
            variant=self.variant,
            rep=self.rep,
            client=RecordingClient(client),
            judge_client=RecordingClient(client),
        )
        self.app_clients.append(ctx.client)
        self.judge_clients.append(ctx.judge_client)
        return ctx

    def rejudged(self, ctx: CaseContext, client: Any) -> CaseContext:
        """The same app try, with a fresh recorder for a new try of the judge."""
        judged = replace(ctx, judge_client=RecordingClient(client))
        self.judge_clients.append(judged.judge_client)
        return judged

    def calls(self, stage: Stage) -> list[Call]:
        clients = list(self.app_clients if stage == "app" else self.judge_clients)
        return [call for client in clients for call in list(client.calls)]


def _execute(
    flow: Flow,
    tasks: Sequence[tuple[Case, int]],
    options: Options,
    client: Any,
    paths: _Paths,
) -> tuple[int, bool]:
    """Run the attempts on daemon threads, at most `concurrency` at once, and write each one.

    Only this thread writes, so a row and its trace never interleave with another attempt's. It
    also keeps the ceilings: an attempt past its own is written as a timeout and its slot goes to
    the next case. Its thread is a daemon, so a request that never returns cannot hold the run open.
    """
    stop = threading.Event()
    settled: queue.Queue[tuple[_Attempt, _Outcome]] = queue.Queue()
    waiting = deque(tasks)
    running: list[_Attempt] = []
    failed, stopped, finished = 0, False, 0

    def written(outcome: _Scored | _Failed) -> None:
        nonlocal failed, stopped, finished
        finished += 1
        if isinstance(outcome, _Scored):
            outcome = _write_scored(paths, outcome)
        progress = f"[{options.label}] {finished}/{len(tasks)} {outcome.case_id} rep{outcome.rep}"
        if isinstance(outcome, _Scored):
            _say(f"{progress} {outcome.row['status']}")
            return
        failed += 1
        _append(paths.errors, outcome.row)
        _say(f"{progress} FAILED ({outcome.row['failure_class']}): {outcome.row['error']}")
        if outcome.stops and not stopped:
            stopped = True
            stop.set()
            _say(
                f"[{options.label}] STOPPED: a zero or missing measurement on an answered "
                "request is a runner bug until proven otherwise. No new case starts."
            )

    try:
        while waiting or running:
            while waiting and len(running) < options.concurrency and not stop.is_set():
                case, rep = waiting.popleft()
                running.append(_start(flow, case, rep, options, client, stop, settled))
            if not running:
                break  # stopped, with cases left that never start
            try:
                attempt, outcome = settled.get(timeout=_wait(running))
            except queue.Empty:
                now = perf_counter()
                for overdue in [one for one in running if one.overdue(now)]:
                    if overdue.expire():
                        running.remove(overdue)
                        written(_timed_out(overdue, options.timeout_s))
                continue
            running.remove(attempt)
            if outcome is not None:  # None: it never started, because the run had stopped
                written(outcome)
    except BaseException:
        stop.set()
        for attempt in running:
            attempt.abandoned.set()
        raise
    return failed, stopped


def _start(
    flow: Flow,
    case: Case,
    rep: int,
    options: Options,
    client: Any,
    stop: threading.Event,
    settled: queue.Queue[tuple[_Attempt, _Outcome]],
) -> _Attempt:
    started = perf_counter()
    attempt = _Attempt(
        case=case,
        rep=rep,
        model=options.model,
        judge_model=flow.judge_model,
        variant=options.variant,
        started=started,
        deadline=started + options.timeout_s if options.timeout_s > 0 else None,
    )
    threading.Thread(
        target=_run_attempt,
        args=(flow, attempt, client, stop, settled),
        name=f"eval-{case.id}-rep{rep}",
        daemon=True,
    ).start()
    return attempt


def _wait(running: Sequence[_Attempt]) -> float:
    """Until the nearest ceiling, and never longer than `_POLL_S`."""
    deadlines = [attempt.deadline for attempt in running if attempt.deadline is not None]
    if not deadlines:
        return _POLL_S
    return min(_POLL_S, max(0.0, min(deadlines) - perf_counter()))


def _run_attempt(
    flow: Flow,
    attempt: _Attempt,
    client: Any,
    stop: threading.Event,
    settled: queue.Queue[tuple[_Attempt, _Outcome]],
) -> None:
    """A worker thread's life: one attempt, its outcome handed over unless the ceiling took it."""
    outcome: _Outcome = None
    try:
        outcome = _attempt(flow, attempt, client, stop)
    except BaseException as exc:
        # Never leave the main thread waiting on an attempt that died.
        outcome = _failed(attempt, "harness_error", f"the attempt died: {type(exc).__name__}")
        raise
    finally:
        if attempt.settle():
            settled.put((attempt, outcome))


def _write_scored(paths: _Paths, scored: _Scored) -> _Scored | _Failed:
    """The trace first, then the row, so a row on disk always has its trace."""
    trace = paths.trace(scored.case_id, scored.rep)
    try:
        if trace.is_symlink():
            raise OSError(f"{trace} is a symlink")
        trace.write_text(
            json.dumps(scored.transcript, indent=2, ensure_ascii=False) + "\n",
            encoding="utf-8",
            newline="\n",
        )
    except OSError as exc:
        row = {
            key: scored.row[key]
            for key in ("prompt_id", "rep", "model", "usage", "latency_s")
            if key in scored.row
        }
        meta = scored.row["meta"]
        row.update(
            failure_class="harness_error",
            error=_one_line(f"writing the trace failed: {exc}"),
            retries=meta["retries"],
            judge_retries=meta.get("judge_retries", 0),
            cost_usd=meta["cost_usd"],
        )
        return _Failed(case_id=scored.case_id, rep=scored.rep, row=row)
    _append(paths.results, scored.row)
    return scored


def _attempt(flow: Flow, attempt: _Attempt, client: Any, stop: threading.Event) -> _Outcome:
    if stop.is_set():
        return None
    try:
        return _scored(flow, attempt, client)
    except _Unmeasured as exc:
        # Set here and not by the writer, which can fall behind: no attempt starts after this.
        stop.set()
        return _failed(attempt, "harness_error", str(exc), stops=True)
    except AttemptFailed as exc:
        return _failed(attempt, exc.failure_class, str(exc), error_class=exc.error_class)
    except Exception as exc:
        return _failed(attempt, "harness_error", f"the runner raised {type(exc).__name__}: {exc}")


def _scored(flow: Flow, attempt: _Attempt, client: Any) -> _Scored:
    case = attempt.case

    def app_try(index: int) -> tuple[CaseContext, Any, _Spend]:
        ctx = attempt.context(client)
        try:
            output = flow.run(case, ctx)
        except Exception as exc:
            # A failed request is the cause, whatever the flow then made of it.
            _raise_for_failed_calls(ctx.client.calls, "the request")
            if isinstance(exc, AttemptFailed):
                raise
            raise AttemptFailed(
                "harness_error", f"the flow raised {type(exc).__name__}: {exc}"
            ) from exc
        return ctx, output, _measured(ctx.client.calls, attempt.model, "the flow")

    ctx, output, app = _with_backoff(attempt, "app", app_try)
    status = "truncated" if app.stop_reason == "max_tokens" else "ok"

    # A clipped answer is counted and shown but never graded or averaged (build-eval.md).
    grade, judge = Grade(scores={}), None
    if status == "ok":

        def judge_try(index: int) -> tuple[Grade, _Spend | None]:
            judged = ctx if index == 0 else attempt.rejudged(ctx, client)
            return _graded(flow, case, output, judged)

        grade, judge = _with_backoff(attempt, "judge", judge_try)

    return _Scored(
        case_id=case.id,
        rep=attempt.rep,
        row=_row(case, attempt, app, judge, status, grade),
        transcript=_transcript(ctx.client.calls),
    )


def _graded(flow: Flow, case: Case, output: Any, ctx: CaseContext) -> tuple[Grade, _Spend | None]:
    try:
        grade = flow.grade(case, output, ctx)
    except Exception as exc:
        _raise_for_failed_calls(ctx.judge_client.calls, "the judge's request")
        if isinstance(exc, AttemptFailed):
            raise
        raise AttemptFailed(
            "harness_error", f"the grader raised {type(exc).__name__}: {exc}"
        ) from exc
    judge: _Spend | None = None
    if ctx.judge_client.calls:
        if flow.judge_model is None:
            raise AttemptFailed(
                "harness_error", "the grader made a request, but the flow declares no judge_model"
            )
        # Before the grade is read: a grade made around a failed judge request is not one.
        _raise_for_failed_calls(ctx.judge_client.calls, "the judge's request")
    _check_grade(grade, flow.metrics)
    if ctx.judge_client.calls:
        assert flow.judge_model is not None
        judge = _measured(ctx.judge_client.calls, flow.judge_model, "the grader")
    return grade, judge


def _with_backoff(attempt: _Attempt, stage: Stage, one_try: Callable[[int], _T]) -> _T:
    """`one_try(k)` for k = 0, 1, ..., backing off between tries on a serving error.

    A try is sent again only when it raised a `serving_error` and none of its requests was
    answered, so no billed work is thrown away, and only while a try is left and the backoff ends
    inside the ceiling. The retry count is the attempt's, kept apart for the app and the judge.
    """
    what = "the request" if stage == "app" else "the judge's request"
    index = 0
    while True:
        if attempt.expired():
            raise AttemptFailed("timeout", f"the wall-clock ceiling passed before {what}")
        try:
            return one_try(index)
        except AttemptFailed as exc:
            clients = attempt.app_clients if stage == "app" else attempt.judge_clients
            answered = any(call.response is not None for call in clients[-1].calls)
            if exc.failure_class != "serving_error" or answered or index + 1 >= MAX_TRIES:
                raise
            delay = _backoff_delay(index)
            if attempt.deadline is not None and perf_counter() + delay >= attempt.deadline:
                raise
            if attempt.abandoned.wait(delay):
                raise AttemptFailed(
                    "timeout", f"the wall-clock ceiling passed while backing off before {what}"
                ) from exc
        index += 1
        if stage == "app":
            attempt.retries = index
        else:
            attempt.judge_retries = index


def _backoff_delay(retry: int) -> float:
    """Seconds before retry `retry` (from 0): `min(60, 2**retry)`, times a 0.5-1.5 jitter."""
    return min(BACKOFF_CAP_S, BACKOFF_BASE_S * 2**retry) * (0.5 + random.random())


def _timed_out(attempt: _Attempt, timeout_s: float) -> _Failed:
    return _failed(
        attempt,
        "timeout",
        f"passed its {timeout_s:g} s wall-clock ceiling. A request still in flight was abandoned, "
        "not cancelled, and nothing it returns is kept",
    )


def _measured(calls: Sequence[Call], requested: str, who: str) -> _Spend:
    """The calls' spend, after every check a successful row has to pass."""
    if not calls:
        raise AttemptFailed("harness_error", f"{who} made no request through its client")
    _raise_for_failed_calls(calls, "the request")
    for call in calls:
        served = getattr(call.response, "model", None)
        if not isinstance(served, str) or not served:
            raise _Unmeasured(f"{who}'s response names no model")
        if not _same_model(requested, served):
            raise AttemptFailed(
                "serving_substitution",
                f"served model {served!r} is not the requested {requested!r}",
            )
    # Priced as the model asked for: it passed the check above, so it is that model or a dated
    # snapshot of it, which the price table does not list separately.
    spend = _spend(calls, requested=requested)
    if spend.usage is None:
        raise _Unmeasured(f"{who}'s response carried no usage")
    prompt = (
        spend.usage["input_tokens"]
        + spend.usage["cache_read_input_tokens"]
        + spend.usage["cache_creation_input_tokens"]
    )
    if prompt <= 0 or spend.usage["output_tokens"] <= 0:
        raise _Unmeasured(f"{who}'s usage has zero prompt or output tokens: {spend.usage}")
    if spend.cost_usd is None or spend.cost_usd <= 0:
        raise _Unmeasured(f"{who}'s cost measured {spend.cost_usd!r}, not above $0")
    if spend.latency_s <= 0:
        raise _Unmeasured(f"{who}'s latency measured {spend.latency_s!r} s")
    return spend


def _raise_for_failed_calls(calls: Sequence[Call], what: str) -> None:
    for call in calls:
        if call.error is not None:
            name = type(call.error).__name__
            raise AttemptFailed(
                _failure_class_of(call.error),
                f"{what} failed: {name}: {call.error}",
                error_class=name,
            )


def _failure_class_of(error: BaseException) -> FailureClass:
    """A provider's passing trouble, or ours. Read off the exception, not the route's note."""
    status = getattr(error, "status_code", None)
    if isinstance(status, int) and not isinstance(status, bool):
        transient = status in _TRANSIENT_STATUS or status >= 500
        return "serving_error" if transient else "harness_error"
    # The SDK's connection errors, read by name so this module need not import the SDK.
    names = {klass.__name__ for klass in type(error).__mro__}
    if names & {"APIConnectionError", "APITimeoutError"} or isinstance(
        error, TimeoutError | ConnectionError
    ):
        return "serving_error"
    return "harness_error"


def _same_model(requested: str, served: str) -> bool:
    if served == requested:
        return True
    base = _ALIAS_SUFFIX.sub("", requested)
    return served.startswith(base) and _SNAPSHOT.fullmatch(served[len(base) :]) is not None


def _spend(calls: Sequence[Call], *, requested: str | None) -> _Spend:
    """Summed over the answered calls.

    Each call is priced as the `requested` model when it served that model or a dated snapshot of
    it (the table lists no snapshots), and as the model it names otherwise.
    """
    answered = [call for call in calls if call.response is not None]
    latency = sum(call.latency_s for call in calls)
    if not answered:
        return _Spend(None, None, None, latency, None, 0)
    last = answered[-1].response
    usages = [_usage(call.response) for call in answered]
    usage: dict[str, int] | None = None
    cost: float | None = None
    if all(one is not None for one in usages):
        counted = [one for one in usages if one is not None]
        usage = {key: sum(one[key] for one in counted) for key in _USAGE_KEYS}
        costs = [
            llm_log.cost_usd(_priced_as(requested, call.response), one)
            for call, one in zip(answered, counted, strict=True)
        ]
        if all(one is not None for one in costs):
            cost = round(sum(one for one in costs if one is not None), 10)
    return _Spend(
        model=_text(getattr(last, "model", None)),
        usage=usage,
        cost_usd=cost,
        latency_s=latency,
        stop_reason=_text(getattr(last, "stop_reason", None)),
        tool_calls=sum(_tool_calls(call.response) for call in answered),
    )


def _priced_as(requested: str | None, response: Any) -> str:
    served = _text(getattr(response, "model", None))
    if requested is not None and (served is None or _same_model(requested, served)):
        return requested
    return served or ""


def _usage(response: Any) -> dict[str, int] | None:
    usage = getattr(response, "usage", None)
    input_tokens = _count(getattr(usage, "input_tokens", None))
    output_tokens = _count(getattr(usage, "output_tokens", None))
    if input_tokens is None or output_tokens is None:
        return None
    return {
        "input_tokens": input_tokens,
        "output_tokens": output_tokens,
        # `None` on the SDK when nothing was cached, which is a real zero.
        "cache_read_input_tokens": _count(getattr(usage, "cache_read_input_tokens", None)) or 0,
        "cache_creation_input_tokens": _count(getattr(usage, "cache_creation_input_tokens", None))
        or 0,
    }


def _check_grade(grade: Grade, metrics: Sequence[Metric]) -> None:
    """Every declared metric, a finite number each, and nothing else: a blank column is a bug.

    A metric declared `always=False` may be left out, and only that: a present value is checked as
    any other is, and a grade with no score at all is a blank row, so it is refused too.
    """
    if not isinstance(grade, Grade):
        raise AttemptFailed("harness_error", f"the grader returned {type(grade).__name__}")
    declared = {metric.id: metric for metric in metrics}
    required = {metric.id for metric in metrics if metric.always}
    if not required <= set(grade.scores) <= set(declared):
        optional = sorted(set(declared) - required)
        raise AttemptFailed(
            "harness_error",
            f"the grade's metrics {sorted(grade.scores)} are not the declared {sorted(declared)}"
            + (f", of which only {optional} may be left out" if optional else ""),
        )
    if not grade.scores:
        raise AttemptFailed("harness_error", "the grade scores no metric, which is a blank row")
    for metric_id, value in grade.scores.items():
        if not isinstance(value, int | float) or not math.isfinite(value):
            raise AttemptFailed("harness_error", f"{metric_id} scored {value!r}, not a number")
        if declared[metric_id].kind == "binary" and value not in (0, 1):
            raise AttemptFailed("harness_error", f"binary {metric_id} scored {value!r}")
    if grade.explanation is not None and not set(grade.explanation) <= set(grade.scores):
        # An explanation beside an absent score would be a reason shown for a blank cell.
        raise AttemptFailed("harness_error", "the grade explains a metric it does not score")


def _row(
    case: Case,
    attempt: _Attempt,
    app: _Spend,
    judge: _Spend | None,
    status: str,
    grade: Grade,
) -> dict[str, Any]:
    assert app.cost_usd is not None  # _measured refused anything else
    meta = dict(case.meta)
    total = app.cost_usd
    if judge is not None:
        assert judge.cost_usd is not None
        total += judge.cost_usd
    meta["cost_usd"] = round(total, 10)
    if judge is not None:
        meta["judge_cost_usd"] = judge.cost_usd
    meta["retries"] = attempt.retries
    if judge is not None or attempt.judge_retries:
        meta["judge_retries"] = attempt.judge_retries
    if app.stop_reason == "refusal":
        meta["failure_class"] = "refusal"
    row: dict[str, Any] = {
        "prompt_id": case.id,
        "rep": attempt.rep,
        "prompt": case.prompt,
        "tags": list(case.tags),
        "meta": meta,
        "model": app.model,
        "usage": app.usage,
        "stop_reason": app.stop_reason,
        "status": status,
    }
    if judge is not None:
        row["judge_model"] = judge.model
        row["judge_usage"] = judge.usage
    row["latency_s"] = app.latency_s
    row["tool_calls"] = app.tool_calls
    row["grade"] = {metric_id: float(value) for metric_id, value in grade.scores.items()}
    if grade.explanation:
        row["explanation"] = dict(grade.explanation)
    return row


def _failed(
    attempt: _Attempt,
    failure_class: FailureClass,
    message: str,
    *,
    error_class: str | None = None,
    stops: bool = False,
) -> _Failed:
    """The error line, carrying whatever every try of the attempt was billed for."""
    row: dict[str, Any] = {
        "prompt_id": attempt.case.id,
        "rep": attempt.rep,
        "failure_class": failure_class,
        "error": _one_line(message),
        "retries": attempt.retries,
        "judge_retries": attempt.judge_retries,
    }
    if error_class is not None:
        row["error_class"] = error_class
    app = _spend(attempt.calls("app"), requested=attempt.model)
    if app.usage is not None:
        row.update(model=app.model, usage=app.usage, cost_usd=app.cost_usd)
    judge = _spend(attempt.calls("judge"), requested=attempt.judge_model)
    if judge.usage is not None:
        row.update(judge_model=judge.model, judge_usage=judge.usage, judge_cost_usd=judge.cost_usd)
    row["latency_s"] = round(perf_counter() - attempt.started, 6)
    return _Failed(case_id=attempt.case.id, rep=attempt.rep, row=row, stops=stops)


# --- the trace ------------------------------------------------------------------------------


def _transcript(calls: Sequence[Call]) -> list[dict[str, Any]]:
    """SCHEMA.md's `Turn[]`: the system prompt, then each request's new messages and its answer."""
    turns: list[dict[str, Any]] = []
    shown = 0
    for index, call in enumerate(calls):
        if index == 0:
            system = _plain_text(call.request.get("system"))
            if system:
                turns.append({"role": "system", "content": system})
        messages = call.request.get("messages") or []
        for message in messages[shown:]:
            turns.extend(_message_turns(message))
        # The answer becomes the next request's last message, so it is not shown twice.
        shown = len(messages) + 1
        if call.response is not None:
            turns.extend(_answer_turns(call.response))
        elif call.error is not None:
            failure = f"(the request failed: {type(call.error).__name__})"
            turns.append({"role": "assistant", "content": failure})
    return turns


def _message_turns(message: Any) -> list[dict[str, Any]]:
    role = _field(message, "role")
    content = _field(message, "content")
    if isinstance(content, str):
        return [{"role": "user" if role == "user" else "assistant", "content": content}]
    if role == "assistant":
        return _answer_turns(message)
    turns: list[dict[str, Any]] = []
    text: list[str] = []
    for block in content or []:
        kind = _field(block, "type")
        if kind == "tool_result":
            turns.append({"role": "tool_result", "content": _plain_text(_field(block, "content"))})
        elif kind == "text":
            text.append(str(_field(block, "text") or ""))
        else:
            text.append(f"[{kind}]")
    if text:
        turns.insert(0, {"role": "user", "content": "\n\n".join(text)})
    return turns


def _answer_turns(response: Any) -> list[dict[str, Any]]:
    turns: list[dict[str, Any]] = []
    thinking: list[str] = []
    for block in _field(response, "content") or []:
        kind = _field(block, "type")
        if kind == "thinking":
            thinking.append(str(_field(block, "thinking") or ""))
        elif kind == "text":
            turn: dict[str, Any] = {"role": "assistant", "content": str(_field(block, "text"))}
            if turns and turns[-1]["role"] == "assistant" and not thinking:
                turns[-1]["content"] += turn["content"]
                continue
            turns.append(_with_thinking(turn, thinking))
        elif kind in _TOOL_BLOCKS:
            arguments = json.dumps(_field(block, "input"), indent=2, ensure_ascii=False)
            turn = {"role": "tool_call", "name": str(_field(block, "name")), "content": arguments}
            turns.append(_with_thinking(turn, thinking))
    if thinking or not turns:
        turns.append(_with_thinking({"role": "assistant", "content": ""}, thinking))
    return turns


def _with_thinking(turn: dict[str, Any], thinking: list[str]) -> dict[str, Any]:
    if thinking:
        turn["thinking"] = "\n\n".join(thinking)
        thinking.clear()
    return turn


def _plain_text(value: Any) -> str:
    if value is None:
        return ""
    if isinstance(value, str):
        return value
    if isinstance(value, list | tuple):
        parts = [
            str(_field(block, "text"))
            if _field(block, "type") == "text"
            else f"[{_field(block, 'type')}]"
            for block in value
        ]
        return "\n\n".join(parts)
    return json.dumps(value, indent=2, ensure_ascii=False, default=str)


def _field(value: Any, name: str) -> Any:
    return value.get(name) if isinstance(value, dict) else getattr(value, name, None)


def _tool_calls(response: Any) -> int:
    content = _field(response, "content")
    if not isinstance(content, list | tuple):
        return 0
    return sum(1 for block in content if _field(block, "type") in _TOOL_BLOCKS)


# --- the summary ----------------------------------------------------------------------------


@dataclass(frozen=True)
class MetricSummary:
    """A metric's mean over cases (each the mean of its `ok` reps), and its 95% interval."""

    metric: Metric
    mean: float | None
    low: float | None
    high: float | None
    cases: int


@dataclass(frozen=True)
class VariantSummary:
    """A variant directory as the report will see it: every row on disk, not just this run's."""

    metrics: list[MetricSummary]  # the headline first
    ok: int
    truncated: int
    failed_attempts: int
    failures_by_class: dict[str, int]
    unscored: int  # (case, rep) pairs with an error line and no row
    retries: int  # backoff retries, the app's and the judge's, on rows and error lines
    cost_usd: float
    unpriced: int  # rows or error lines with usage but no cost
    constant: list[str]  # metric ids with one value across two or more scored rows


def headline(metrics: Sequence[Metric]) -> Metric:
    """The report's headline: the first binary metric, else the first (SCHEMA.md)."""
    return next((metric for metric in metrics if metric.kind == "binary"), metrics[0])


def summarize(variant_dir: Path, metrics: Sequence[Metric]) -> VariantSummary:
    rows = _rows(variant_dir / RESULTS_FILE)
    errors = _rows(variant_dir / ERRORS_FILE)
    scored = [row for row in rows if row.get("status", "ok") == "ok"]
    first = headline(metrics)
    ordered = [first, *[metric for metric in metrics if metric is not first]]

    summaries: list[MetricSummary] = []
    constant: list[str] = []
    for metric in ordered:
        per_case: dict[str, list[float]] = {}
        for row in scored:
            value = (row.get("grade") or {}).get(metric.id)
            if isinstance(value, int | float) and not isinstance(value, bool):
                per_case.setdefault(str(row.get("prompt_id")), []).append(float(value))
        means = [sum(values) / len(values) for values in per_case.values()]
        summaries.append(_interval(metric, means))
        values = [value for values in per_case.values() for value in values]
        if len(values) >= 2 and len(set(values)) == 1:
            constant.append(metric.id)

    keys = {(row.get("prompt_id"), row.get("rep")) for row in rows}
    unscored = {(e.get("prompt_id"), e.get("rep")) for e in errors} - keys
    retries = sum(_retries(row.get("meta") or {}) for row in rows)
    retries += sum(_retries(error) for error in errors)
    cost, unpriced = 0.0, 0
    for row in rows:
        value = (row.get("meta") or {}).get("cost_usd")
        if isinstance(value, int | float):
            cost += value
        else:
            unpriced += 1
    for error in errors:
        if error.get("usage") is None:
            continue
        value = error.get("cost_usd")
        if isinstance(value, int | float):
            cost += value + (error.get("judge_cost_usd") or 0)
        else:
            unpriced += 1
    return VariantSummary(
        metrics=summaries,
        ok=len(scored),
        truncated=sum(1 for row in rows if row.get("status") == "truncated"),
        failed_attempts=len(errors),
        failures_by_class=dict(Counter(str(e.get("failure_class")) for e in errors)),
        unscored=len(unscored),
        retries=retries,
        cost_usd=round(cost, 10),
        unpriced=unpriced,
        constant=constant,
    )


def _retries(record: Mapping[str, Any]) -> int:
    counts = (record.get("retries"), record.get("judge_retries"))
    return sum(count for count in counts if isinstance(count, int) and not isinstance(count, bool))


def _interval(metric: Metric, means: Sequence[float]) -> MetricSummary:
    count = len(means)
    if count == 0:
        return MetricSummary(metric, None, None, None, 0)
    mean = sum(means) / count
    if metric.kind == "binary":
        # Wilson's score interval over cases: unlike the normal one, it does not collapse to a
        # point at 0% or 100%, which is where an oracle and a null land.
        z2 = _Z95**2
        centre = (mean + z2 / (2 * count)) / (1 + z2 / count)
        half = _Z95 * math.sqrt(mean * (1 - mean) / count + z2 / (4 * count**2)) / (1 + z2 / count)
        return MetricSummary(metric, mean, max(0.0, centre - half), min(1.0, centre + half), count)
    if count < 2:
        return MetricSummary(metric, mean, None, None, count)
    sd = math.sqrt(sum((value - mean) ** 2 for value in means) / (count - 1))
    half = _Z95 * sd / math.sqrt(count)
    return MetricSummary(metric, mean, mean - half, mean + half, count)


def render_summary(label: str, model: str, summary: VariantSummary) -> list[str]:
    """The summary lines: the headline with its interval first."""
    first, *others = summary.metrics
    lines = [f"{label} on {model}: {_scored_metric(first)}"]
    lines.extend(f"  also {_scored_metric(other)}" for other in others)
    failures = ", ".join(f"{cls} {n}" for cls, n in sorted(summary.failures_by_class.items()))
    tally = f"  {summary.ok} scored and {summary.truncated} truncated rows; "
    tally += f"{summary.failed_attempts} failed attempts"
    tally += f" ({failures})" if failures else ""
    tally += f", {summary.unscored} (case, rep) unscored; {summary.retries} backoff retries"
    lines.append(tally)
    cost = f"  cost ${summary.cost_usd:.4f} measured"
    if summary.unpriced:
        cost += f", and not measured for {summary.unpriced} row(s) with no price"
    lines.append(cost)
    return lines


def _scored_metric(summary: MetricSummary) -> str:
    name = summary.metric.label
    if summary.mean is None:
        return f"{name}: no scored rows"

    def shown(value: float) -> str:
        if summary.metric.kind == "binary":
            return f"{100 * value:.1f}%"
        return f"{value:.3f}"

    text = f"{name} {shown(summary.mean)}"
    if summary.low is not None and summary.high is not None:
        text += f" (95% CI {shown(summary.low)} to {shown(summary.high)}"
    else:
        text += " (no interval"
    return text + f", {summary.cases} case{'s' if summary.cases != 1 else ''})"


# --- files ----------------------------------------------------------------------------------


def _rows(path: Path) -> list[dict[str, Any]]:
    """Every readable JSON object in a JSONL file. A torn or foreign line is skipped."""
    if not path.is_file():
        return []
    rows: list[dict[str, Any]] = []
    for line in path.read_text(encoding="utf-8").splitlines():
        if not line.strip():
            continue
        try:
            row = json.loads(line)
        except ValueError:
            continue
        if isinstance(row, dict):
            rows.append(row)
    return rows


def _append(path: Path, row: Mapping[str, Any]) -> None:
    if path.is_symlink():
        raise OSError(f"refusing to append to {path}: it is a symlink")
    with path.open("a", encoding="utf-8", newline="\n") as handle:
        handle.write(json.dumps(row, ensure_ascii=False) + "\n")


def _end_with_newline(path: Path) -> None:
    """A crash can leave a torn last line; the next row must not be glued onto it."""
    if not path.is_file() or path.stat().st_size == 0:
        return
    with path.open("rb") as handle:
        handle.seek(-1, 2)
        last = handle.read(1)
    if last != b"\n":
        with path.open("ab") as handle:
            handle.write(b"\n")


def _say(message: str) -> None:
    """One line on stderr. Case ids and error text can carry a model's bytes, so no controls."""
    print(_CONTROL.sub("", " ".join(message.split())), file=sys.stderr)


def _one_line(message: str) -> str:
    text = _CONTROL.sub("", " ".join(message.split()))
    return text if len(text) <= _ERROR_LIMIT else text[: _ERROR_LIMIT - 3] + "..."


def _count(value: Any) -> int | None:
    # `bool` is an `int`, and `True` tokens is a fake's mistake, not a count.
    return value if isinstance(value, int) and not isinstance(value, bool) else None


def _text(value: Any) -> str | None:
    return value if isinstance(value, str) and value else None


if __name__ == "__main__":
    # Run as `python -m golf_coach.evals.runner`, this file is `__main__`: a second copy of the
    # module, beside the `golf_coach.evals.runner` that every flow imports and subclasses. Its own
    # `main` would check each flow's objects against its own classes, which no flow derives from:
    # `_flow`'s `isinstance(found, Flow)` refused `club_spec` by name on the user's first pilot
    # (M43 P13, before any request), and past it `Case`, `Grade` and `except AttemptFailed` would
    # have failed every case the same way. So the command runs the importable module's `main`,
    # whose classes are the ones the flows share. `tests/evals/test_runner.py` runs this entry.
    sys.exit(importlib.import_module("golf_coach.evals.runner").main())
