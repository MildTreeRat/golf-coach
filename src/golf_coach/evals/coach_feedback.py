"""The coach-feedback eval: the coaching paragraph, checked by code against its brief. [M43 P17]

    python -m golf_coach.evals.runner --flow coach_feedback --model claude-opus-5 [--reps 2]

Each case is one synthetic swing stored under `evals/coach_feedback/sessions/` (the plan's P15 and
P16), coached through the real stdio entry, `feedback.coach.coach_through_golf_mcp`, with the run's
model and the runner's recording client, over a `golf-mcp` started on those sessions. That is the
route `golf-lab analyze` spawns, so what is graded is the paragraph a golfer would be served.

## The inputs are the approved ones

The user approved the sixteen briefs in `cases.md` as written (P16), as the club-spec key's sets
were verified one by one. So before the paid request the flow reads the swing through `golf-mcp`,
renders it as the route does (`brief_from_view`), and refuses the attempt as a `harness_error`, with
no request made, when that is not the approved brief. A stale or rebuilt server, an edited session
or a change to the rendering would otherwise have a model coach from inputs nobody approved. After
the request it checks the brief the route actually sent, the same way. Because of those two checks,
`cases.md` is the only input file the harness sha needs beside `cases.json`: an upstream change
either leaves every brief as approved or stops each attempt before it costs anything.

`golf-mcp` is read twice per attempt (here, then inside the route), each a fresh stdio session of a
few tens of milliseconds. Each read starts its own server, so attempts on the runner's threads share
nothing (P7's question, pinned in `tests/evals/test_coach_feedback.py`). Its stderr banner goes to a
scratch file, quoted only when a read fails, so it does not bury the runner's per-attempt lines.

## The four code checks

ADR-036 §8: these are the eval's primary metric, and the judge (`coach_judge`, P18) is secondary.
They measure whether the paragraph is **faithful to its brief**, not whether it is good coaching. A
paragraph that says nothing passes all four; whether it names the top fault and gives something to
try is the judge's rubric.

Each check fails only on a violation it can name, and its explanation names it (the number, the
words, the sentence), so the pilot's grading review (P20) can audit every fail. That costs recall: a
paraphrase the patterns below do not know is not caught, and reading for those is the judge's job.
The patterns are derived from the case wherever the case says the thing (a checkpoint's name and its
rule-based reading, the shot's labels, the prompt's own list of what is not measured), and the few
word lists that cannot be derived are module constants with their reasons.

1. **`numbers_in_brief`**: every number the paragraph writes in digits is in the brief, within the
   rounding the paragraph wrote it at: `0.3` stands for the brief's `0.28`, and `0.275` does not.
   Signs are ignored here (the brief prints a club path as `-1.5` and a paragraph rightly says "1.5
   degrees out-to-in"; quoting a raw negative is the caveats' business). A worked number, a
   difference or a conversion to seconds, is not in the brief and fails, because the prompt says
   never to state a measurement the brief does not hold. A whole number is good to half a unit, so
   one below ten nearly always rounds from something in a brief (its session date holds a 7): an
   invented "7-iron" passes, since no brief names the club.
2. **`caveats_carried`**: each caveat this case's view carries, wherever the paragraph touches the
   thing it qualifies. What the paragraph leaves alone asks nothing of it: a paragraph that never
   mentions a ball-flight row drops no caveat (P15). The caveats come from the served view, never
   from a fixed list, so a real stored brief's band-estimator note (NOTES, P16) asks nothing:
   - a shot flagged `needs_review`, used (a printed label, the launch monitor, or a number only the
     shot block holds), and nowhere said to be uncertain;
   - an unscored checkpoint named in a sentence that does not say it was not measured (sentence by
     sentence, because "your finish was balanced" is a claim whatever a later sentence admits);
   - advice to film or photograph again when every unscored entry says that would not help
     (`refilming_helps`) and the shot is not flagged;
   - the views called synchronized when the alignment carries its caveat;
   - an uncalibrated placement (a Q) used with no word about calibration or its T2;
   - interim rotation readings used with nothing saying they are interim;
   - the raw signed value quoted for the checkpoint the prompt says is recorded as negative, which
     the prompt says never to quote.
3. **`nothing_unmeasured`**: no sentence claims one of the things the system prompt says are not
   measured, the list read off the prompt the route sent ("X, Y and Z are NOT measured here"), not
   retyped. A sentence that says it was not measured is not a claim. A term the brief itself prints
   as a reading is exempt: the launch monitor prints `club path`, which is the shot's number and not
   the pose's.
4. **`verdict_kept`**: no sentence reverses the rule-based verdict, praising a checkpoint that
   missed its band or calling one that passed a fault. Only a sentence naming exactly one checkpoint
   and holding no negation is read, and only praise with no fault word, or a fault with no praise,
   counts. `interim-pivot-dtl` misses `hip_sway` on its low side, so a paragraph calling the small
   hip travel quiet or good fails here (less is not better, P16).

**`all_checks`**, the headline (the runner's first binary metric), is 1 when a paragraph was served
and it passes all four.

## The judge

After the checks, every served paragraph goes to `coach_judge.judge` through `ctx.judge_client`,
with the brief and the system prompt the route sent and never `ctx.model`, so the judge is blind to
which model wrote it (ADR-036 §8). Its five columns (`judge_all`, then one per rubric criterion)
follow the code checks', so the headline stays `all_checks`. Neither side reads the other: a
paragraph that fails a check is still judged, and the judge's verdict moves no check. The checks
run first because they cost nothing, so a prompt the checks cannot read stops the attempt before
the judge is paid for.

## What the route's answers become

- **A paragraph**: the four checks and `all_checks`, then the judge's five.
- **Truncation** (`max_tokens`): the runner writes the row as `truncated` and never grades it, so
  the coach's served-with-an-admission paragraph is not checked (P7).
- **A refusal**: a graded row (eval-audit.md §2), `all_checks` 0 and the four checks absent, which
  their `always=False` allows: there is no paragraph to check, and a stand-in would move their means
  (the club-spec flow's rule, M43 P12). The judge is not called, and its five are absent too. The
  runner marks it `meta.failure_class: "refusal"`.
- **The judge's refusal, or an answer that is not a verdict**: an error line, never a zero
  (`coach_judge`), and a resume grades the case again.
- **No text**: nothing to grade, and the model's doing, so `genuine_failure`.
- **An SDK error**: the runner reads it off the recorded call, as for the club-spec flow.
- **No `golf-mcp`, no analysis, or not the approved brief**: `harness_error`, before any request.

## What it imports

ADR-036 §1: the standard library, the runner, the judge, `config.py` and the coach route's own
public pieces (a survivor). Never `coach_cases`, the recorder, which imports the frozen engine
(P15), so the case paths are named again here and `tests/evals/test_coach_feedback.py` checks the
two lists of cases agree.
"""

from __future__ import annotations

import itertools
import json
import re
import tempfile
from collections.abc import Iterable, Mapping, Sequence
from dataclasses import dataclass
from pathlib import Path
from typing import Any, TextIO

from golf_coach.config import REPO_ROOT
from golf_coach.evals import coach_judge
from golf_coach.evals.runner import AttemptFailed, Case, CaseContext, Flow, Grade, Metric
from golf_coach.feedback.coach import (
    FLIGHT_FAMILY,
    GolfMcpServer,
    brief_from_view,
    coach_through_golf_mcp,
    default_golf_mcp,
    is_analyzed,
    read_swing,
)

CASES_DIR = REPO_ROOT / "evals" / "coach_feedback"
#: A path rather than a bare name, as `coach_cases.CASES` and `club_spec.KEY` are: an eval's input
#: is read by Python alone, so `test_every_on_disk_artifact_has_a_schema` has nothing to classify.
CASES_FILE = Path("cases.json")
BRIEFS_FILE = "cases.md"
SESSIONS = "sessions"

#: The code checks, the primary metric. The four may be absent: on a refusal there is no paragraph
#: to check (the docstring).
CODE_METRICS: tuple[Metric, ...] = (
    Metric("all_checks", "All checks", "binary"),
    Metric("numbers_in_brief", "Numbers", "binary", always=False),
    Metric("caveats_carried", "Caveats", "binary", always=False),
    Metric("nothing_unmeasured", "No unmeasured", "binary", always=False),
    Metric("verdict_kept", "Verdict kept", "binary", always=False),
)
CHECKS = tuple(metric.id for metric in CODE_METRICS[1:])
#: Then the judge's, secondary (ADR-036 §8), so `all_checks` stays the report's headline.
METRICS: tuple[Metric, ...] = (*CODE_METRICS, *coach_judge.METRICS)

#: The brief's heading over the launch monitor's numbers (`brief_from_view`).
SHOT_HEADING = "LAUNCH MONITOR SHOT"

#: How long a quoted sentence may run in an explanation before it is cut.
_QUOTE_LIMIT = 140


@dataclass(frozen=True)
class SwingCase:
    """One case: where its swing is stored, and the brief the user approved for it (`cases.md`)."""

    session_id: str
    swing_id: str
    brief: str


@dataclass(frozen=True)
class Coached:
    """What the route served for one case, and exactly what it was served from."""

    #: The paragraph, or None when the route answered with a note.
    text: str | None
    note: str | None
    stop_reason: str | None
    #: The user message and the system prompt the route sent, read off the recorded request.
    brief: str
    system: str
    #: `get_swing`'s answer the brief was rendered from: the caveats' and the verdict's source.
    view: Mapping[str, Any]


@dataclass(frozen=True)
class CheckResult:
    """One check's verdict on one paragraph, and the sentence saying why."""

    passed: bool
    explanation: str


class CoachFeedbackFlow(Flow):
    """The `coach_feedback` flow: one coaching paragraph per case, checked by code and judged."""

    metrics = METRICS
    judge_model = coach_judge.JUDGE_MODEL

    def __init__(self, cases_dir: Path = CASES_DIR, *, golf_mcp: Path | None = None) -> None:
        self.cases_dir = cases_dir
        #: None means `default_golf_mcp()`, read at each run, so `GOLF_MCP` set after import counts.
        self.golf_mcp = golf_mcp

    def load_cases(self) -> list[Case]:
        doc = json.loads((self.cases_dir / CASES_FILE).read_text(encoding="utf-8"))
        approved = approved_briefs((self.cases_dir / BRIEFS_FILE).read_text(encoding="utf-8"))
        cases: list[Case] = []
        for raw in doc["cases"]:
            case_id = raw["id"]
            brief = approved.get(case_id)
            if brief is None:
                raise ValueError(
                    f"{self.cases_dir / BRIEFS_FILE} has no brief for case {case_id!r}. The user "
                    "approves the briefs as cases.md shows them (M43 P16), so regenerate it "
                    "(python -m golf_coach.evals.coach_cases) and have it approved"
                )
            swing = f"{raw['session_id']}/{raw['swing_id']}"
            cases.append(
                Case(
                    id=case_id,
                    prompt=f"Swing {swing}: {raw['about']}",
                    tags=tuple(raw["tags"]),
                    input=SwingCase(raw["session_id"], raw["swing_id"], brief),
                    meta={"swing": swing},
                )
            )
        stray = sorted(set(approved) - {case.id for case in cases})
        if stray:
            raise ValueError(f"{self.cases_dir / BRIEFS_FILE} has briefs for no case: {stray}")
        return cases

    def run(self, case: Case, ctx: CaseContext) -> Coached:
        asked = _swing_case(case)
        binary = self.golf_mcp or default_golf_mcp()
        with tempfile.TemporaryDirectory(
            prefix="coach-eval-", ignore_cleanup_errors=True
        ) as scratch_dir:
            scratch = Path(scratch_dir)
            (scratch / "shots").mkdir()
            server = GolfMcpServer(
                binary=binary,
                sessions_dir=self.cases_dir / SESSIONS,
                shots_dir=scratch / "shots",
            )
            with (scratch / "golf-mcp.log").open("w+", encoding="utf-8") as errlog:
                view = _approved_view(server, asked, errlog)
                answer = coach_through_golf_mcp(
                    asked.session_id,
                    asked.swing_id,
                    server=server,
                    model=ctx.model,
                    client=ctx.client,
                    errlog=errlog,
                )
                if not ctx.client.calls:
                    raise AttemptFailed(
                        "harness_error",
                        f"the coach route made no request: {answer.get('note')}{_tail(errlog)}",
                    )
        call = ctx.client.calls[-1]
        sent_brief, system = _sent(call.request)
        if sent_brief != asked.brief:
            raise AttemptFailed(
                "harness_error",
                f"the route sent {_swing(asked)} a brief that is not the approved one: "
                + _first_difference(sent_brief, asked.brief),
            )
        text = answer.get("text")
        note = answer.get("note")
        return Coached(
            text=text if isinstance(text, str) else None,
            note=note if isinstance(note, str) else None,
            stop_reason=getattr(call.response, "stop_reason", None),
            brief=sent_brief,
            system=system,
            view=view,
        )

    def grade(self, case: Case, output: Any, ctx: CaseContext) -> Grade:
        if not isinstance(output, Coached):
            raise AttemptFailed("harness_error", f"the flow returned {type(output).__name__}")
        if output.text is None:
            if output.stop_reason == "refusal":
                return Grade(
                    scores={"all_checks": 0.0},
                    explanation={
                        "all_checks": "no paragraph: the model declined to coach this swing, so "
                        "there was nothing to check"
                    },
                )
            raise AttemptFailed("genuine_failure", f"the route served no paragraph: {output.note}")
        checked = grade_paragraph(
            output.text, brief=output.brief, system=output.system, view=output.view
        )
        # What the coach was sent and what it wrote, and never `ctx.model`: blind (ADR-036 §8).
        judged = coach_judge.judge(
            ctx.judge_client, instructions=output.system, brief=output.brief, paragraph=output.text
        )
        return Grade(
            scores={**checked.scores, **judged.scores},
            explanation={**(checked.explanation or {}), **(judged.explanation or {})},
        )


# --- the checks ---------------------------------------------------------------------------------


def grade_paragraph(paragraph: str, *, brief: str, system: str, view: Mapping[str, Any]) -> Grade:
    """All four checks on one paragraph, and `all_checks` over them."""
    results = check_paragraph(paragraph, brief=brief, system=system, view=view)
    passed = all(result.passed for result in results.values())
    failing = [metric for metric, result in results.items() if not result.passed]
    scores = {"all_checks": 1.0 if passed else 0.0}
    scores.update({metric: 1.0 if result.passed else 0.0 for metric, result in results.items()})
    explanation = {
        "all_checks": "every check passed" if passed else f"failed: {', '.join(failing)}"
    }
    explanation.update({metric: result.explanation for metric, result in results.items()})
    return Grade(scores=scores, explanation=explanation)


def check_paragraph(
    paragraph: str, *, brief: str, system: str, view: Mapping[str, Any]
) -> dict[str, CheckResult]:
    """Each check's result, keyed by its metric id, in `CHECKS` order."""
    return {
        "numbers_in_brief": check_numbers(paragraph, brief),
        "caveats_carried": check_caveats(paragraph, brief=brief, system=system, view=view),
        "nothing_unmeasured": check_unmeasured(paragraph, brief=brief, system=system),
        "verdict_kept": check_verdict(paragraph, view),
    }


def check_numbers(paragraph: str, brief: str) -> CheckResult:
    """Every number written in digits is one the brief holds, at the paragraph's own rounding."""
    known = [quoted.value for quoted in numbers_in(brief)]
    quoted = numbers_in(paragraph)
    stray = [q for q in quoted if not any(_within(q, value) for value in known)]
    if stray:
        listed = "; ".join(
            f"{q.text} (in {_quote(_sentence_at(paragraph, q.start))})" for q in stray
        )
        return CheckResult(
            False, f"{len(stray)} of {len(quoted)} numbers not in the brief: {listed}"
        )
    if not quoted:
        return CheckResult(True, "the paragraph quotes no number")
    return CheckResult(
        True,
        f"all {len(quoted)} numbers are in the brief: {', '.join(q.text for q in quoted)}",
    )


def check_caveats(
    paragraph: str, *, brief: str, system: str, view: Mapping[str, Any]
) -> CheckResult:
    """Every caveat the view carries, wherever the paragraph uses what it qualifies."""
    dropped: list[str] = []
    carried: list[str] = []
    said = sentences(paragraph)
    shot = view.get("shot") if isinstance(view.get("shot"), Mapping) else None
    flagged = bool(shot and shot.get("needs_review"))
    unscored = _items(view.get("unscored"))

    if flagged:
        used = _shot_uses(paragraph, brief)
        if used and not _UNCERTAIN.search(paragraph):
            dropped.append(
                f"the shot is flagged needs_review, and the paragraph uses it ({', '.join(used)}) "
                "without saying the reading is uncertain"
            )
        elif used:
            carried.append("the flagged shot is called uncertain")

    for entry in unscored:
        name = str(entry.get("name", ""))
        if not name or entry.get("family") == FLIGHT_FAMILY:
            continue
        pattern = _name_pattern(name)
        for sentence in said:
            if pattern.search(sentence):
                if _says_not_measured(sentence):
                    carried.append(f"{name} is said to be unscored")
                else:
                    dropped.append(f"{name} is unscored, and {_quote(sentence)} does not say so")

    capture_helps = any(entry.get("refilming_helps") for entry in unscored)
    if unscored and not capture_helps and not flagged:
        for sentence in said:
            if _asserted(_REFILM, sentence):
                reasons = sorted({str(entry.get("reason")) for entry in unscored})
                dropped.append(
                    f"{_quote(sentence)} advises capturing again, and no unscored entry here is a "
                    f"capture problem ({', '.join(reasons)})"
                )

    if view.get("alignment_caveat"):
        for sentence in said:
            if _asserted(_SYNCED, sentence):
                dropped.append(
                    f"the alignment is {view.get('alignment_quality')} and carries its caveat, "
                    f"and {_quote(sentence)} calls the views synchronized"
                )

    for row in _items(view.get("population")):
        if row.get("calibrated") is not False:
            continue
        name = str(row.get("name", ""))
        uses = [s for s in said if _name_pattern(name).search(s) or _Q.search(s)]
        if uses and not _CALIBRATION.search(paragraph):
            dropped.append(
                f"{name} is not calibrated, and {_quote(uses[0])} reads it with no word about "
                "calibration or its T2"
            )
        elif uses:
            carried.append(f"{name} is read beside its calibration")

    rotation = _items(view.get("rotation"))
    if rotation:
        names = [re.escape(str(row.get("name", ""))) for row in rotation if row.get("name")]
        raw = re.compile("|".join(names), re.IGNORECASE) if names else None
        uses = [s for s in said if _ROTATION.search(s) or (raw is not None and raw.search(s))]
        if uses and not (_INTERIM.search(paragraph) or any(_says_not_measured(s) for s in uses)):
            dropped.append(
                f"the rotation rows are interim, and {_quote(uses[0])} uses them without saying so"
            )
        elif uses:
            carried.append("the rotation rows are called interim")

    for name in signed_checkpoints(system):
        for checkpoint in _items(view.get("checkpoints")):
            observed = _number(checkpoint.get("observed"))
            if checkpoint.get("name") != name or observed is None or observed >= 0:
                continue
            for quoted in _signed_numbers(paragraph):
                if _within(quoted, observed):
                    dropped.append(
                        f"{name}'s raw value is quoted with its sign ({quoted.text}), which the "
                        "prompt says never to quote"
                    )

    if dropped:
        return CheckResult(False, "; ".join(dropped))
    if carried:
        return CheckResult(True, "carried: " + "; ".join(dict.fromkeys(carried)))
    return CheckResult(True, "the paragraph touches nothing a caveat in this brief qualifies")


def check_unmeasured(paragraph: str, *, brief: str, system: str) -> CheckResult:
    """No sentence claims what the prompt says is not measured, unless the brief prints it."""
    terms = unmeasured_terms(system)
    exempt = [term for term in terms if _printed(term, brief)]
    checked = [term for term in terms if term not in exempt]
    claims: list[str] = []
    for term in checked:
        pattern = _phrase(term, gap=2, any_order=True)
        for sentence in sentences(paragraph):
            if pattern.search(sentence) and not _says_not_measured(sentence):
                claims.append(f"{term}: {_quote(sentence)}")
    if claims:
        return CheckResult(False, "claims what is not measured: " + "; ".join(claims))
    note = (
        f" ({', '.join(exempt)} exempt: the brief prints it as a launch-monitor reading)"
        if exempt
        else ""
    )
    return CheckResult(True, f"claims none of {', '.join(checked)}{note}")


def check_verdict(paragraph: str, view: Mapping[str, Any]) -> CheckResult:
    """No sentence praises a checkpoint that missed, or calls one that passed a fault."""
    scored = [
        (str(c.get("name")), bool(c.get("passed")), _checkpoint_patterns(c))
        for c in _items(view.get("checkpoints"))
        if isinstance(c.get("passed"), bool) and c.get("name")
    ]
    reversals: list[str] = []
    read = 0
    for sentence in sentences(paragraph):
        named = [(name, passed) for name, passed, patterns in scored if _any(patterns, sentence)]
        if len(named) != 1 or _VERDICT_NEGATION.search(sentence):
            continue
        read += 1
        (name, passed), (praise, fault) = named[0], _praise_and_fault(sentence)
        if not passed and praise and not fault:
            reversals.append(
                f"{name} missed its band, and {_quote(sentence)} praises it ({', '.join(praise)})"
            )
        elif passed and fault and not praise:
            reversals.append(
                f"{name} passed, and {_quote(sentence)} calls it a fault ({', '.join(fault)})"
            )
    if reversals:
        return CheckResult(False, "; ".join(reversals))
    return CheckResult(True, f"no reversal in the {read} sentence(s) naming exactly one checkpoint")


# --- what the checks read off the prompt and the brief ------------------------------------------

#: "Spine angle, hip **rotation**, swing plane and club path are NOT measured here": the prompt's
#: own list (`ONLY_THESE_FUNDAMENTALS` in the caveats `golf-mcp` serves).
_NOT_MEASURED_HERE = re.compile(r"([^.:;]*?)\s+are\s+not\s+measured\s+here", re.IGNORECASE)
#: "`head_stays_back` is recorded as a **negative** number": the checkpoint never quoted raw.
_RECORDED_NEGATIVE = re.compile(r"`(\w+)` is recorded as a negative number", re.IGNORECASE)


def unmeasured_terms(system: str) -> tuple[str, ...]:
    """What the system prompt says is not measured, lower-cased, in its order. Raises on none."""
    flat = " ".join(system.split()).replace("**", "")
    terms: list[str] = []
    for clause in _NOT_MEASURED_HERE.findall(flat):
        terms += [part.strip().lower() for part in re.split(r",\s*|\s+and\s+|\s+or\s+", clause)]
    terms = [term for term in terms if term]
    if not terms:
        raise AttemptFailed(
            "harness_error",
            "the system prompt names nothing as not measured ('... are NOT measured here'), so "
            "there is no list to check against",
        )
    return tuple(dict.fromkeys(terms))


def signed_checkpoints(system: str) -> tuple[str, ...]:
    """The checkpoints the prompt says are recorded as negative numbers, never quoted raw."""
    flat = " ".join(system.split()).replace("**", "")
    return tuple(dict.fromkeys(_RECORDED_NEGATIVE.findall(flat)))


def _printed(term: str, brief: str) -> bool:
    """Whether the brief prints `term` as a reading of its own (`club path: -1.5 degrees`)."""
    return re.search(rf"^{re.escape(term)}:", brief, re.IGNORECASE | re.MULTILINE) is not None


def _blocks(brief: str) -> list[list[str]]:
    """The brief's blank-line-separated blocks, each a list of lines (its heading first)."""
    return [block.strip("\n").splitlines() for block in brief.split("\n\n") if block.strip()]


def _shot_uses(paragraph: str, brief: str) -> list[str]:
    """How the paragraph uses the shot: a label, the launch monitor, or a number only it holds."""
    blocks = _blocks(brief)
    shot = next((block for block in blocks if block[0] == SHOT_HEADING), [])
    rest = "\n".join("\n".join(block) for block in blocks if block[0] != SHOT_HEADING)
    labels = [
        line.split(":", 1)[0]
        for line in shot[1:]
        if ":" in line and not line.startswith((" ", "needs_review"))
    ]
    uses = [label for label in labels if _phrase(label, gap=1).search(paragraph)]
    if _LAUNCH_MONITOR.search(paragraph):
        uses.append("the launch monitor")
    shot_numbers = [q.value for q in numbers_in("\n".join(shot))]
    other_numbers = [q.value for q in numbers_in(rest)]
    uses += [
        q.text
        for q in numbers_in(paragraph)
        if any(_within(q, v) for v in shot_numbers)
        and not any(_within(q, v) for v in other_numbers)
    ]
    return list(dict.fromkeys(uses))


def _checkpoint_patterns(checkpoint: Mapping[str, Any]) -> list[re.Pattern[str]]:
    """How a sentence names a checkpoint: its name, or the lead of its rule-based reading."""
    patterns = [_name_pattern(str(checkpoint.get("name", "")))]
    message = checkpoint.get("message")
    if isinstance(message, str) and " - " in message:
        patterns.append(_name_pattern(message.split(" - ", 1)[0]))
    return patterns


# --- approved briefs ----------------------------------------------------------------------------

_MD_CASE = re.compile(r"^## `(?P<id>[^`]+)`\n(?P<body>.*?)(?=^## `|\Z)", re.MULTILINE | re.DOTALL)
_MD_BRIEF = re.compile(r"^The brief:\n\n```text\n(?P<brief>.*?)\n```$", re.MULTILINE | re.DOTALL)


def approved_briefs(markdown: str) -> dict[str, str]:
    """Each case's brief as `cases.md` shows it, by case id: what the user approved (P16)."""
    briefs: dict[str, str] = {}
    for case in _MD_CASE.finditer(markdown):
        found = _MD_BRIEF.search(case.group("body"))
        if found is not None:
            briefs[case.group("id")] = found.group("brief")
    return briefs


def _approved_view(server: GolfMcpServer, asked: SwingCase, errlog: TextIO) -> Mapping[str, Any]:
    """`get_swing`'s view of the case's swing, refused unless it renders as the approved brief."""
    try:
        read = read_swing(server, asked.session_id, asked.swing_id, errlog=errlog)
    except Exception as exc:  # not installed, not built, will not start, or no view: as the route
        raise AttemptFailed(
            "harness_error",
            f"golf-mcp could not serve {_swing(asked)} ({_innermost(exc)}){_tail(errlog)}",
        ) from exc
    if read.view is None or not is_analyzed(read.view):
        raise AttemptFailed(
            "harness_error",
            f"golf-mcp holds no analysis for {_swing(asked)}: {read.miss or 'no analysis on file'}",
        )
    try:
        rendered = brief_from_view(read.view)
    except ValueError as exc:
        raise AttemptFailed("harness_error", f"{_swing(asked)}: {exc}") from exc
    if rendered != asked.brief:
        raise AttemptFailed(
            "harness_error",
            f"golf-mcp serves {_swing(asked)} a brief that is not the one approved in cases.md, "
            "so no request was made: "
            + _first_difference(rendered, asked.brief)
            + ". Rebuild golf-mcp (cargo build --release -p golf-mcp), or regenerate the cases "
            "and have them approved again",
        )
    return read.view


def _sent(request: Mapping[str, Any]) -> tuple[str, str]:
    """The brief and the system prompt one recorded coaching request carried."""
    messages = request.get("messages") or []
    content = messages[0].get("content") if messages else None
    system = request.get("system")
    if isinstance(system, list):
        system = "\n".join(
            str(block.get("text", "")) for block in system if isinstance(block, dict)
        )
    if not isinstance(content, str) or not isinstance(system, str):
        raise AttemptFailed("harness_error", "the coaching request carried no brief or no system")
    return content, system


def _first_difference(got: str, approved: str) -> str:
    for number, (one, other) in enumerate(
        itertools.zip_longest(got.splitlines(), approved.splitlines(), fillvalue="<end>"), 1
    ):
        if one != other:
            return f"line {number} reads {_quote(one)} where the approved one reads {_quote(other)}"
    return "the two differ only in line endings"


def _swing(asked: SwingCase) -> str:
    return f"swing {asked.session_id}/{asked.swing_id}"


def _swing_case(case: Case) -> SwingCase:
    if not isinstance(case.input, SwingCase):
        raise AttemptFailed("harness_error", f"case {case.id!r} carries no SwingCase")
    return case.input


def _tail(errlog: TextIO) -> str:
    """The end of `golf-mcp`'s stderr, for a message about a read that failed."""
    try:
        errlog.flush()
        errlog.seek(0)
        said = " ".join(errlog.read().split())
    except (OSError, ValueError):
        return ""
    return f"; golf-mcp said: {said[-300:]}" if said else ""


def _innermost(exc: BaseException) -> str:
    """The one error worth naming, out of the exception groups anyio wraps it in."""
    while isinstance(exc, BaseExceptionGroup) and exc.exceptions:
        exc = exc.exceptions[0]
    return f"{type(exc).__name__}: {exc}"


# --- reading text -------------------------------------------------------------------------------

#: A number written in digits: thousands commas, a decimal part, an ordinal suffix. Never one glued
#: to a word (`T2`, `7i`), and a minus is not part of it (a range's hyphen would read as one).
_NUMBER = re.compile(
    r"(?<![\w.,])(?P<whole>\d{1,3}(?:,\d{3})+|\d+)(?:\.(?P<part>\d+))?(?:st|nd|rd|th)?(?!\w)"
)
#: A number written with its sign, which the caveats read: never a range's hyphen after a digit.
_SIGNED = re.compile(
    r"(?<![\w.])(?:[-−]\s?|minus\s+|negative\s+)(?P<whole>\d+)(?:\.(?P<part>\d+))?(?!\w)",
    re.IGNORECASE,
)
#: Where a sentence turns from one clause to the next: punctuation, a dash, or a joining word.
_CLAUSE = re.compile(r"[,;:—–]|\s-\s|\b(?:so|but|because|since|although|though)\b")
_SENTENCE_END = re.compile(r"(?<=[.!?])[\"'”’)\]]*\s+|\n+")

#: The words that undo a claim. `no` and `nothing` are in it for the caveats, where "no need to film
#: again" and "nothing measured it" are not advice or claims.
_NEGATION = re.compile(r"\b(?:not|no|never|nor|cannot|without|nothing)\b|n['’]t\b", re.IGNORECASE)
#: The verdict reads praise like "no wasted motion", so there `no` and `nothing` do not negate.
_VERDICT_NEGATION = re.compile(
    r"\b(?:not|never|nor|cannot|without|hardly|barely)\b|n['’]t\b", re.IGNORECASE
)
#: What a sentence says was not measured, beside a negation: "it wasn't measured", "I can't see it".
_MEASURED = re.compile(
    r"\b(?:measur\w*|scor\w*|track\w*|captur\w*|record\w*|judg\w*|detect\w*|assess\w*|read\w*"
    r"|see|seen|tell|data|numbers?|information|available|visible)\b",
    re.IGNORECASE,
)
_UNMEASURED = re.compile(
    r"\b(?:unscored|unmeasured|untracked|unmeasurable|missing|excluded|left out)\b", re.IGNORECASE
)
#: Saying a flagged reading is uncertain, in the words a coach would use for it.
_UNCERTAIN = re.compile(
    r"\b(?:uncertain\w*|unsure|misread\w*|mis-read\w*|unreliable|flagged|review|doubt\w*"
    r"|questionable|suspect|garbled|caution\w*|grain of salt|(?:may|might|could) be (?:wrong|off)"
    r"|not (?:sure|reliable|trustworthy)|(?:can['’]t|cannot|don['’]t|wouldn['’]t"
    r"|won['’]t|shouldn['’]t) (?:trust|rely))\b",
    re.IGNORECASE,
)
_LAUNCH_MONITOR = re.compile(r"\blaunch[- ]monitor\b", re.IGNORECASE)
#: Advice to capture again: the prompt's "re-film, steady the camera, shoot the swing again or
#: photograph the screen again", and the ways a coach says it.
_REFILM = re.compile(
    r"\bre-?(?:film|record|shoot|take|photograph|capture)\w*"
    r"|\b(?:film|record|shoot|photograph)\w*\b(?:\W+\w+){0,4}?\W+again\b"
    r"|\b(?:another|a new|a fresh|a second|a better|a steadier|a sharper|a clearer)\s+"
    r"(?:clip|video|recording|photo|picture|take|camera|angle)\b"
    r"|\bstead(?:y|i)\w*\s+(?:the\s+|your\s+)?(?:camera|phone)\b",
    re.IGNORECASE,
)
_SYNCED = re.compile(r"\bsynchroni[sz]\w*|\bin[- ]sync\b|\bsync(?:ed|s)?\b", re.IGNORECASE)
#: A Q by its letter, as the brief and the prompt write it (capital, a word on its own).
_Q = re.compile(r"(?<![\w-])Q(?![\w-])")
_CALIBRATION = re.compile(r"calibrat|\bt2\b", re.IGNORECASE)
_ROTATION = re.compile(
    r"\b(?:rotation|pivot\w*|shoulder\s+(?:turn|line|axis|rotation|reversal)\w*|axis\s+drift"
    r"|hip\s+axis)\b",
    re.IGNORECASE,
)
_INTERIM = re.compile(
    r"\b(?:interim|provisional|rough\w*|approximat\w*|uncalibrated|not calibrated|tentative"
    r"|preliminary|indicative|2-?d|two-dimensional|no band|not (?:scored|judged|part of))\b",
    re.IGNORECASE,
)
#: Praise and fault, for the verdict. Praise is matched first and taken out, so "nothing to fix"
#: is not read as a "fix". `quiet`, `steady` and `stable` are praise of small movement, which is
#: exactly the reading that reverses a two-sided checkpoint missed on its low side.
_PRAISE = re.compile(
    r"\b(?:good|great|solid|nice|excellent|perfect|ideal|fine|strong|strengths?|steady|stable"
    r"|quiet|tidy|textbook|spot[- ]on|smooth|well done|right where"
    r"|(?:inside|within) (?:the |your )?(?:tour )?(?:band|range)|passe[sd]"
    r"|no (?:problems?|issues?|faults?|concerns?)|nothing to (?:fix|change|work on))\b",
    re.IGNORECASE,
)
_FAULT = re.compile(
    r"\b(?:faults?|problems?|issues?|mistakes?|flaws?|leaks?|poor|bad"
    r"|too (?:much|far|quick|fast|slow|little|early|late|big|small|long|short)"
    r"|miss(?:es|ed)?|(?:outside|under|below|over|above|past|beyond) (?:the |your )?(?:tour )?"
    r"(?:band|range)|needs? (?:work|fixing|attention)|work on|fix\w*|improv\w*|tighten\w*"
    r"|cut down|reduc\w*)\b",
    re.IGNORECASE,
)
#: Words a name or a reading's lead carries that say nothing about which checkpoint it is.
_STOP = frozenset({"a", "an", "and", "at", "of", "the", "to", "in", "on", "by"})


@dataclass(frozen=True)
class Quoted:
    """A number as written: its text, its value (unsigned), its decimals, and where it starts."""

    text: str
    value: float
    decimals: int
    start: int


def numbers_in(text: str) -> list[Quoted]:
    return [_quoted(match) for match in _NUMBER.finditer(text)]


def _signed_numbers(text: str) -> list[Quoted]:
    return [_quoted(match) for match in _SIGNED.finditer(text)]


def _quoted(match: re.Match[str]) -> Quoted:
    whole, part = match.group("whole").replace(",", ""), match.group("part")
    value = float(f"{whole}.{part}" if part else whole)
    return Quoted(match.group(0), value, len(part) if part else 0, match.start())


def _within(quoted: Quoted, value: float) -> bool:
    """Whether `value` rounds to what was written, at the decimals it was written with."""
    return abs(abs(value) - quoted.value) <= 0.5 * 10.0**-quoted.decimals + 1e-9


def sentences(text: str) -> list[str]:
    return [part.strip() for part in _SENTENCE_END.split(text) if part.strip()]


def _sentence_at(text: str, index: int) -> str:
    start = max(text.rfind(mark, 0, index) for mark in (". ", "! ", "? ", "\n"))
    ends = [found for mark in (". ", "! ", "? ", "\n") if (found := text.find(mark, index)) >= 0]
    return text[start + 1 if start >= 0 else 0 : min(ends) + 1 if ends else len(text)].strip()


def _quote(sentence: str) -> str:
    cut = sentence if len(sentence) <= _QUOTE_LIMIT else sentence[: _QUOTE_LIMIT - 3] + "..."
    return f'"{cut}"'


def _asserted(pattern: re.Pattern[str], sentence: str) -> bool:
    """Whether a clause of `sentence` matches `pattern` with no negation in that clause.

    By clause, because "it could not be scored, so film it again" is advice to film again, and a
    negation anywhere in the sentence would excuse it.
    """
    return any(
        pattern.search(clause) and not _NEGATION.search(clause)
        for clause in _CLAUSE.split(sentence)
    )


def _says_not_measured(sentence: str) -> bool:
    return bool(
        _UNMEASURED.search(sentence) or (_NEGATION.search(sentence) and _MEASURED.search(sentence))
    )


def _praise_and_fault(sentence: str) -> tuple[list[str], list[str]]:
    praise = [match.group(0) for match in _PRAISE.finditer(sentence)]
    rest = _PRAISE.sub(" ", sentence)
    return praise, [match.group(0) for match in _FAULT.finditer(rest)]


def _word(word: str) -> str:
    """One word, and the forms a coach inflects it into: `rotation` also reads `rotate`."""
    word = word.lower()
    if len(word) <= 3:
        return re.escape(word) + "s?"
    if word.endswith("ion") and len(word) > 5:
        return re.escape(word[:-3]) + r"(?:ion\w*|e\w*|ing|ed)"
    if word.endswith("e"):
        return re.escape(word[:-1]) + r"(?:e\w*|ing|ed)"
    if word.endswith("s") and len(word) > 4:
        word = word[:-1]
    return re.escape(word) + r"\w*"


def _phrase(text: str, *, gap: int, any_order: bool = False) -> re.Pattern[str]:
    """A phrase's words in order (or any order), each inflected, with up to `gap` words between."""
    words = [w for w in re.split(r"[\W_]+", text.lower()) if w]
    content = [w for w in words if w not in _STOP] or words
    between = rf"\W+(?:\w+\W+){{0,{gap}}}"
    orders: Iterable[Sequence[str]] = itertools.permutations(content) if any_order else (content,)
    body = "|".join(between.join(_word(w) for w in order) for order in orders)
    return re.compile(rf"(?<!\w)(?:{body})(?!\w)", re.IGNORECASE)


def _name_pattern(name: str) -> re.Pattern[str]:
    """A checkpoint's or reading's name in a sentence: in order, each stop word dropped a gap."""
    words = [w for w in re.split(r"[\W_]+", name.lower()) if w]
    return _phrase(name, gap=1 + sum(w in _STOP for w in words))


def _any(patterns: Sequence[re.Pattern[str]], sentence: str) -> bool:
    return any(pattern.search(sentence) for pattern in patterns)


def _items(value: Any) -> list[Mapping[str, Any]]:
    return [item for item in value if isinstance(item, Mapping)] if isinstance(value, list) else []


def _number(value: Any) -> float | None:
    if isinstance(value, bool) or not isinstance(value, int | float):
        return None
    return float(value)


FLOW = CoachFeedbackFlow()
