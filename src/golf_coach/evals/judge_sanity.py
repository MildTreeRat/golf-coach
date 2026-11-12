"""The coach judge's three sanity checks, run before it grades anything real. [M43 P19]

build-eval.md asks this of any LLM judge, and ADR-036 §8 makes it the coach pilot's precondition:
an empty paragraph, "I don't know", and a confident paragraph about a different swing must each
fail. `coach_judge`'s rubric goes further and says a non-answer fails **every** criterion (P18's
second choice), because otherwise saying nothing would pass `no_invented_claims` and a degenerate
policy would score. So each row records both: `failed` (`judge_all` is 0, the plan's Done when) and
`failed_every_criterion`.

## What is judged

Each probe is one paragraph judged against one approved case's brief (`CASE`), with the coach's
instructions exactly as the stdio route sends them: the caveat sentences read off `golf-mcp` and
joined by `coaching_system_prompt`, as `coach_through_golf_mcp` joins them. The frozen in-process
`coach.SYSTEM_PROMPT` is not used (P18's finding). The brief is read through `golf-mcp` too, and
refused unless it is the one approved in `cases.md`, so nothing is paid for on a brief nobody
approved.

The third probe is written by hand, not by a model. It is good coaching for `OTHER` (a
left-hander's driver whose head drifts forward), in that swing's numbers and faults, judged against
`CASE`'s brief. Its suggestions promise nothing, so read on its own it could pass `actionable` and
`no_invented_claims`. It fails those two only if the judge applies the rubric's "plainly about a
different swing" rule, which is the thing being checked.

## Not a runner flow

The runner refuses a flow that made no app request (`_measured`), and a flow would need its own
`_state.json` and the user's `--approve-harness` (P18's finding). A probe has nothing under test
before the judge, so this module makes the one request through the runner's recorder and measures
it with the runner's own checks: the served model is the judge, and usage, cost and latency are
above zero, or the run stops (a zero is a bug until proven otherwise). The requests go one at a
time, so the first writes the cache and the rest read it.

## What it writes

`.claude/hillclimb/coach_feedback/_judge_sanity/`, whose leading underscore keeps the report
builder from reading it as a variant. `results.jsonl` holds one row per answered judge request and
`errors.jsonl` one line per failed one (a refusal, an answer that is not a verdict, an SDK error),
carrying whatever it was billed. Resume is at (probe, rep): a probe with a row is not paid for
again, and an error line does not hold its slot. The judge writes no call-log line (P18), so the
rows' `cost_usd` and the errors' are the whole record of the spend.

    python -m golf_coach.evals.judge_sanity [--reps 2] [--dry-run]

Exit 0 when every probe has its rows and each failed, 1 when the judge passed one, and 2 when a row
is missing or the run was refused before any request.
"""

from __future__ import annotations

import argparse
import tempfile
from collections.abc import Callable, Sequence
from dataclasses import dataclass
from datetime import UTC, datetime
from pathlib import Path
from typing import Any

from golf_coach.evals import coach_judge, runner
from golf_coach.evals.coach_feedback import CASES_DIR, SESSIONS, CoachFeedbackFlow, SwingCase
from golf_coach.evals.runner import AttemptFailed, RecordingClient
from golf_coach.feedback.coach import (
    PROMPT_CAVEATS,
    GolfMcpServer,
    brief_through_golf_mcp,
    coaching_system_prompt,
    default_golf_mcp,
)

#: The case whose brief every probe is judged against: the first one, everything measured, and a
#: headline (hip shift at top) that the third probe does not name.
CASE = "clean-swing"
#: The case the third probe is written about. Its numbers (228 yards of carry, 143 mph) and its
#: faults (the head drifting forward, head sway) are not `CASE`'s, and `CASE` missed the tempo it
#: praises.
OTHER = "clean-left-driver"
REPS = 2
OUT_DIR = runner.RUNS_DIR / "coach_feedback" / "_judge_sanity"
RESULTS = "results.jsonl"
ERRORS = "errors.jsonl"


@dataclass(frozen=True)
class Probe:
    """One paragraph the judge must fail, and what it is, for the summary."""

    id: str
    paragraph: str
    about: str


#: Written by hand from `OTHER`'s approved brief in `cases.md`, in the coach's own format (one
#: paragraph, no markdown, at most two things to try), and confident throughout.
OTHER_SWING_PARAGRAPH = (
    "Good strike with the driver, 228 yards of carry off 143 mph of ball speed, and your tempo is "
    "right where it belongs at 2.9 to 1, with a finish you held beautifully. The thing to work on "
    "first is your head drifting toward the target through impact: you had only 0.25 "
    "shoulder-widths of separation between head and hips by impact, and your head moved 0.50 "
    "shoulder-widths sideways on the way down, more than at least 90% of tour swings. On your next "
    "few swings, pick a spot on the back of the ball and feel your head stay behind it until the "
    "ball is gone, and rehearse that first with a few slow half swings, letting your hips turn "
    "through while your head stays put."
)

PROBES: tuple[Probe, ...] = (
    Probe("empty", "", "an empty paragraph"),
    Probe("dont_know", "I don't know.", '"I don\'t know."'),
    Probe("other_swing", OTHER_SWING_PARAGRAPH, f"a confident paragraph about {OTHER}"),
)


@dataclass(frozen=True)
class Inputs:
    """What every probe is judged with: the coach's instructions and `CASE`'s approved brief."""

    instructions: str
    brief: str


class Refused(Exception):
    """A reason not to start. Raised before any request, so nothing is spent."""


def read_inputs(cases_dir: Path = CASES_DIR, *, golf_mcp: Path | None = None) -> Inputs:
    """`CASE`'s brief and the coach's instructions, read through `golf-mcp` as the route reads them.

    Raises `Refused` when `golf-mcp` cannot serve the brief, or serves one that is not the brief
    approved in `cases.md`.
    """
    asked = _swing_case(cases_dir, CASE)
    binary = golf_mcp or default_golf_mcp()
    with tempfile.TemporaryDirectory(
        prefix="judge-sanity-", ignore_cleanup_errors=True
    ) as scratch_dir:
        scratch = Path(scratch_dir)
        (scratch / "shots").mkdir()
        server = GolfMcpServer(
            binary=binary, sessions_dir=cases_dir / SESSIONS, shots_dir=scratch / "shots"
        )
        with (scratch / "golf-mcp.log").open("w+", encoding="utf-8") as errlog:
            brief = brief_through_golf_mcp(
                server, asked.session_id, asked.swing_id, caveats=PROMPT_CAVEATS, errlog=errlog
            )
            errlog.seek(0)
            said = errlog.read().strip()
    if brief.text is None:
        tail = f" (golf-mcp said: {said[-400:]})" if said else ""
        raise Refused(f"golf-mcp served no brief for {CASE}: {brief.refusal}{tail}")
    if brief.text != asked.brief:
        raise Refused(
            f"golf-mcp serves {CASE} a brief that is not the one approved in cases.md, so no "
            "request was made. Rebuild golf-mcp (cargo build --release -p golf-mcp), or regenerate "
            "the cases and have them approved again"
        )
    instructions = coaching_system_prompt(*(brief.caveats[name] for name in PROMPT_CAVEATS))
    return Inputs(instructions=instructions, brief=brief.text)


def judge_probes(
    client: Any,
    inputs: Inputs,
    *,
    out_dir: Path = OUT_DIR,
    reps: int = REPS,
    probes: Sequence[Probe] = PROBES,
    say: Callable[[str], None] = print,
) -> int:
    """Each probe judged `reps` times, one request at a time; returns how many requests were made.

    A (probe, rep) with a row is skipped. Each answer is appended as it lands, to `results.jsonl`
    when it is a verdict and to `errors.jsonl` when it is not.
    """
    out_dir.mkdir(parents=True, exist_ok=True)
    results, errors = out_dir / RESULTS, out_dir / ERRORS
    runner._end_with_newline(results)
    runner._end_with_newline(errors)
    done = runner._finished(results)
    made = 0
    for probe in probes:
        for rep in range(reps):
            if (probe.id, rep) in done:
                continue
            row = _judged(client, inputs, probe, rep)
            made += 1
            if "grade" in row:
                runner._append(results, row)
                say(f"{probe.id} rep {rep}: judge_all {row['grade']['judge_all']:g}")
            else:
                runner._append(errors, row)
                say(f"{probe.id} rep {rep}: {row['failure_class']}: {row['error']}")
    return made


def _judged(client: Any, inputs: Inputs, probe: Probe, rep: int) -> dict[str, Any]:
    """One judge request for one probe: a row with a grade, or an error line saying why not.

    `runner._Unmeasured` is not caught: an answered request with a zero measure is a bug, and the
    run stops rather than write it down.
    """
    recorder = RecordingClient(client)
    base: dict[str, Any] = {
        "prompt_id": probe.id,
        "rep": rep,
        "case_id": CASE,
        "judged_at": datetime.now(UTC).isoformat(timespec="seconds"),
    }
    try:
        grade = coach_judge.judge(
            recorder,
            instructions=inputs.instructions,
            brief=inputs.brief,
            paragraph=probe.paragraph,
        )
        spend = runner._measured(recorder.calls, coach_judge.JUDGE_MODEL, "the judge")
    except AttemptFailed as exc:
        return _error(base, recorder, exc.failure_class, str(exc), exc.error_class)
    except runner._Unmeasured:
        raise
    except Exception as exc:  # an SDK error, which the judge raises unchanged
        name = type(exc).__name__
        return _error(base, recorder, runner._failure_class_of(exc), f"{name}: {exc}", name)
    request_id = getattr(recorder.calls[-1].response, "_request_id", None)
    scores = grade.scores
    return {
        **base,
        "paragraph": probe.paragraph,
        "judge_model": spend.model,
        "judge_usage": spend.usage,
        "cost_usd": spend.cost_usd,
        "latency_s": round(spend.latency_s, 6),
        "stop_reason": spend.stop_reason,
        "request_id": request_id if isinstance(request_id, str) else None,
        "grade": scores,
        "explanation": grade.explanation,
        "failed": scores["judge_all"] == 0.0,
        "failed_every_criterion": all(
            scores[criterion.metric_id] == 0.0 for criterion in coach_judge.CRITERIA
        ),
    }


def _error(
    base: dict[str, Any],
    recorder: RecordingClient,
    failure_class: str,
    message: str,
    error_class: str | None,
) -> dict[str, Any]:
    """The error line, with what the request was billed when it was answered at all."""
    row: dict[str, Any] = {**base, "failure_class": failure_class, "error": message}
    if error_class is not None:
        row["error_class"] = error_class
    spend = runner._spend(recorder.calls, requested=coach_judge.JUDGE_MODEL)
    if spend.usage is not None:
        row.update(judge_model=spend.model, judge_usage=spend.usage, cost_usd=spend.cost_usd)
    row["latency_s"] = round(spend.latency_s, 6)
    return row


def report(
    out_dir: Path = OUT_DIR,
    *,
    reps: int = REPS,
    probes: Sequence[Probe] = PROBES,
    say: Callable[[str], None] = print,
) -> int:
    """Every probe's rows as a table, the measured spend, and the exit code (the module doc's)."""
    rows = {(row.get("prompt_id"), row.get("rep")): row for row in runner._rows(out_dir / RESULTS)}
    errors = runner._rows(out_dir / ERRORS)
    columns = ["judge_all", *(criterion.metric_id for criterion in coach_judge.CRITERIA)]
    say(f"judged against {CASE}'s brief by {coach_judge.JUDGE_MODEL}, {reps} reps per probe")
    say(f"{'probe':<12} rep  " + "  ".join(name.removeprefix("judge_") for name in columns))
    missing = passed = 0
    costs: list[float] = []
    for probe in probes:
        for rep in range(reps):
            row = rows.get((probe.id, rep))
            if row is None:
                missing += 1
                say(f"{probe.id:<12} {rep:<3}  no row")
                continue
            scores = row["grade"]
            passed += 0 if row["failed"] else 1
            costs.append(float(row["cost_usd"]))
            cells = "  ".join(
                f"{scores[name]:<{len(name.removeprefix('judge_'))}g}" for name in columns
            )
            spent = f"${row['cost_usd']:.4f}  {row['latency_s']:.1f} s"
            say(f"{probe.id:<12} {rep:<3}  {cells}  {spent}")
    billed = [float(line["cost_usd"]) for line in errors if line.get("cost_usd") is not None]
    if costs:
        say(
            f"{len(costs)} rows, ${sum(costs):.4f} in all, ${sum(costs) / len(costs):.4f} a "
            "judge call"
        )
    if errors:
        say(f"{len(errors)} error lines, ${sum(billed):.4f} billed on them")
    if passed:
        say(f"FAILED: the judge passed {passed} of the probes it must fail")
        return 1
    if missing:
        say(f"{missing} probe rows missing: run again to judge them")
        return 2
    return 0


def main(argv: Sequence[str] | None = None, *, client: Any = None) -> int:
    parser = argparse.ArgumentParser(
        prog="python -m golf_coach.evals.judge_sanity",
        description="Judge the three paragraphs the coach judge must fail (M43 P19).",
    )
    parser.add_argument("--reps", type=runner._positive, default=REPS)
    parser.add_argument("--cases-dir", type=Path, default=CASES_DIR)
    parser.add_argument("--out", type=Path, default=OUT_DIR)
    parser.add_argument(
        "--golf-mcp", type=Path, default=None, help="default: GOLF_MCP, else target/release"
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="read the inputs and build each request, and make none",
    )
    args = parser.parse_args(argv)
    try:
        inputs = read_inputs(args.cases_dir, golf_mcp=args.golf_mcp)
    except Refused as exc:
        print(f"refused: {exc}")
        return 2
    if args.dry_run:
        for probe in PROBES:
            request = coach_judge.judge_request(
                instructions=inputs.instructions, brief=inputs.brief, paragraph=probe.paragraph
            )
            cached, fresh = request["messages"][0]["content"]
            print(
                f"{probe.id}: {len(request['system']) + len(cached['text'])} characters before "
                f"the cache breakpoint and {len(fresh['text'])} after it; no request made"
            )
        return 0
    if client is None:
        try:
            client, source = runner._build_client(runner._sdk())
        except runner._Refused as exc:
            print(f"refused: {exc}")
            return 2
        print(f"credential: {source}")
    judge_probes(client, inputs, out_dir=args.out, reps=args.reps)
    return report(args.out, reps=args.reps)


def _swing_case(cases_dir: Path, case_id: str) -> SwingCase:
    for case in CoachFeedbackFlow(cases_dir).load_cases():
        if case.id == case_id:
            asked = case.input
            assert isinstance(asked, SwingCase)
            return asked
    raise Refused(f"{cases_dir} has no case {case_id!r}")


if __name__ == "__main__":
    raise SystemExit(main())
