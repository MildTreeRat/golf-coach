"""The club-spec eval: the set lookup, graded field by field against a published key. [M43 P12]

    python -m golf_coach.evals.runner --flow club_spec --model claude-opus-5 [--reps 2]

Each case is one `clubs.lookup.look_up_set` call, the route the bag page makes, run with the run's
model through the runner's recording client. What is graded is the route's outcome, the specs the
golfer would be shown, and not the model's raw JSON: a row the route drops (a slot nobody asked
for, a spec that did not hold together) is a blank here because it is a blank on the page.

The answer key is `evals/club_spec/cases.json`, gathered from makers' spec pages with a URL and a
date per value and verified by the user set by set (ADR-036 §7, the plan's P9 to P11). It is never
`clubs/club_catalogue.json`, which holds Claude's own past lookups and so is the thing under test.
A case without the user's `verified` date is refused at load, not graded: an unreviewed set would
be scored against exactly as readily as a reviewed one.

## Grading, per field

Only the key's five `fields` are graded (loft, lie, length, offset, bounce). The lookup asks for
every `ClubSpec` field, and the key says nothing about the rest, so a filled swing weight is
neither right nor invented by it. Each graded field of each slot is one of five outcomes:

- `right`: a value was published and the answer is within `TOLERANCE` of it;
- `wrong`: a value was published and the answer is outside it;
- `missed`: a value was published and the answer is blank;
- `abstained`: nothing was published and the answer is blank, which is the right answer;
- `invented`: nothing was published and the answer holds a number.

**A zero is a number.** A driver's offset or bounce that the maker never printed and the model
gives as `0` is `invented`, the same as any other figure nobody published. The route keeps a `0.0`
as a value and never as a blank (`lookup.py`'s docstring), so the bag would store "no offset" as a
fact with the lookup's provenance on it. Whether a driver has offset is a claim, and the key holds
no source for it. (P11 asked for this ruling; the key does not change for it.)

## The metrics, and what each is a share of

In report order, every one a per-case share on a 0 to 1 scale:

1. `invented`, the headline (lower is better): invented over the fields with no published value.
2. `accuracy`: right over the published values the answer filled in. An invention is not in it,
   because `invented` counts that; scoring one mistake in two columns would weigh it twice.
3. `coverage`: the published values the answer filled in, right or wrong, over all of them.
4. `abstention`: on a made-up model (tag `fabricated`) only, abstained over every graded field.
   There every field is unpublished, so it is `1 - invented` on that case. It is a column of its
   own because `invented`'s mean mixes real sets and made-up ones.

**A share with nothing to divide by is absent from the grade, never a stand-in number.** A made-up
set has no published value, so no accuracy or coverage; a set that prints every field (three of
the real ones do) has no unpublished field, so no invented rate; a real set has no abstention; and
an answer that filled nothing published has no accuracy. A stand-in would move every mean it
joined: `accuracy = 1` on an all-blank answer would score the null run perfect, and `coverage = 0`
on a made-up set would mark the oracle down for leaving it blank. So all four are declared
`always=False`, which lets the runner's grade check pass a grade without them (the user's call at
P12, departing from build-eval.md's "every id on every case's grade" for these four only). The
runner's summary and the report builder average only the values a row has, so each metric's mean
is over the cases that define it, and its case count says how many.

The report averages per case, so a one-club driver weighs what a ten-club iron set does. Each row's
`meta` carries the case's `published` and `unpublished` counts, so a pooled rate over fields can be
worked out from the rows, and each metric's `explanation` names the fields behind its share.

## What the route's notes become

- **An SDK error** (rate limit, overload, connection, auth): the runner reads it off the recorded
  call, before grading, as a `serving_error` (backed off and retried) or a `harness_error`.
- **Truncation** (`max_tokens`): the runner writes the row as `truncated` and never grades it.
- **A refusal**: graded, as an answer with every field blank, and the runner marks the row's
  `meta.failure_class` as `refusal` (eval-audit.md §2: a refusal is the model's answer). On a
  made-up model that is a perfect abstention, which it is.
- **An unreadable answer, or one naming no clubs**: nothing to grade, and the model's doing, so the
  attempt fails as `genuine_failure` and is an `errors.jsonl` line rather than a zero.

## Tolerances

`TOLERANCE` holds the plan's proposal for loft, lie and length, plus this module's for offset and
bounce. P13's pilot is where the user confirms or changes them. An answer exactly at the edge is
right.
"""

from __future__ import annotations

import json
from collections import Counter
from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from enum import StrEnum
from pathlib import Path
from typing import Any

from golf_coach.clubs.lookup import ClubLookupOutcome, look_up_set
from golf_coach.config import REPO_ROOT
from golf_coach.contracts.club import ClubId
from golf_coach.contracts.club_spec import ClubSpec
from golf_coach.evals.runner import AttemptFailed, Case, CaseContext, Flow, Grade, Metric

KEY = REPO_ROOT / "evals" / "club_spec" / "cases.json"

#: How far an answer may sit from the published figure and still be right, in the field's unit.
#: Loft, lie and length are the plan's proposal. Offset's quarter millimetre is about a hundredth
#: of an inch, and about half the step between neighbouring irons' offsets, as the quarter inch is
#: for length; it covers P9's inch conversion (rounded to 0.01 mm) many times over. Bounce is in
#: degrees, as loft and lie are. P13 confirms or changes all five.
TOLERANCE: Mapping[str, float] = {
    "loft_deg": 0.5,
    "lie_deg": 0.5,
    "length_in": 0.25,
    "offset_mm": 0.25,
    "bounce_deg": 0.5,
}

#: Float noise on a difference that is exactly the tolerance (3.16 - 2.91 is not quite 0.25).
_SLACK = 1e-9

#: The tag the answer key gives a made-up model, whose right answer is blank throughout.
FABRICATED = "fabricated"

#: Each may be absent from a grade, where its share has nothing to divide by (`rates`).
METRICS: tuple[Metric, ...] = (
    Metric("invented", "Invented", "float", better="lower", scale=1.0, always=False),
    Metric("accuracy", "Accuracy", "float", scale=1.0, always=False),
    Metric("coverage", "Coverage", "float", scale=1.0, always=False),
    Metric("abstention", "Made-up blank", "float", scale=1.0, always=False),
)


class Outcome(StrEnum):
    """What one graded field of one slot came to."""

    RIGHT = "right"
    WRONG = "wrong"
    MISSED = "missed"
    ABSTAINED = "abstained"
    INVENTED = "invented"


@dataclass(frozen=True)
class SetCase:
    """One case's input to the lookup, and the published values it is graded against."""

    make: str
    model: str
    model_year: int | None
    slots: tuple[ClubId, ...]
    #: `published[slot][field]`, in the field's unit; `None` where the source printed nothing.
    published: Mapping[ClubId, Mapping[str, float | None]]
    fields: tuple[str, ...]
    fabricated: bool


@dataclass(frozen=True)
class Answer:
    """What the route served, and why its last request stopped (a refusal is graded as blank)."""

    outcome: ClubLookupOutcome
    stop_reason: str | None


@dataclass(frozen=True)
class FieldResult:
    slot: ClubId
    field: str
    outcome: Outcome
    published: float | None
    answered: float | None


class ClubSpecFlow(Flow):
    """The `club_spec` flow: one set lookup per case, graded by code (no judge)."""

    metrics = METRICS
    judge_model = None

    def __init__(self, key: Path = KEY) -> None:
        self.key = key

    def load_cases(self) -> list[Case]:
        doc = json.loads(self.key.read_text(encoding="utf-8"))
        fields = tuple(doc["fields"])
        if set(fields) != set(TOLERANCE) or len(fields) != len(TOLERANCE):
            raise ValueError(
                f"{self.key} grades {list(fields)}, and TOLERANCE covers {list(TOLERANCE)}. "
                "A field with no tolerance cannot be graded, and one with no key has no answer"
            )
        unknown = set(fields) - set(ClubSpec.model_fields)
        if unknown:
            raise ValueError(f"{self.key} grades {sorted(unknown)}, which ClubSpec does not have")
        return [_case(raw, fields, self.key) for raw in doc["cases"]]

    def run(self, case: Case, ctx: CaseContext) -> Answer:
        asked = _set_case(case)
        outcome = look_up_set(
            asked.make,
            asked.model,
            asked.model_year,
            asked.slots,
            llm_model=ctx.model,
            client=ctx.client,
        )
        # Read off the recorded response, as the route's `_read` does, and not off the note's
        # wording, which can be reworded without the outcome changing.
        last = ctx.client.calls[-1].response if ctx.client.calls else None
        return Answer(outcome=outcome, stop_reason=getattr(last, "stop_reason", None))

    def grade(self, case: Case, output: Any, ctx: CaseContext) -> Grade:
        asked = _set_case(case)
        if not isinstance(output, Answer):
            raise AttemptFailed("harness_error", f"the flow returned {type(output).__name__}")
        if not output.outcome.specs and output.stop_reason != "refusal":
            raise AttemptFailed(
                "genuine_failure",
                f"the lookup served no specifications: {output.outcome.note}",
            )
        return grade_specs(asked, output.outcome.specs)


def grade_specs(asked: SetCase, specs: Sequence[ClubSpec]) -> Grade:
    """Every graded field's outcome, reduced to the case's shares. An empty `specs` is all blank."""
    results = field_results(asked, specs)
    shares = rates(results, fabricated=asked.fabricated)
    explanation = _explain(results, shares)
    return Grade(
        scores={metric: value for metric, value in shares.items() if value is not None},
        explanation=explanation,
    )


def field_results(asked: SetCase, specs: Sequence[ClubSpec]) -> tuple[FieldResult, ...]:
    """One result per graded field of every requested slot, in slot then field order."""
    served = {spec.club: spec for spec in specs}
    results: list[FieldResult] = []
    for slot in asked.slots:
        spec = served.get(slot)
        for field in asked.fields:
            published = asked.published[slot][field]
            answered = getattr(spec, field) if spec is not None else None
            results.append(
                FieldResult(slot, field, classify(field, published, answered), published, answered)
            )
    return tuple(results)


def classify(field: str, published: float | None, answered: float | None) -> Outcome:
    if published is None:
        return Outcome.ABSTAINED if answered is None else Outcome.INVENTED
    if answered is None:
        return Outcome.MISSED
    within = abs(answered - published) <= TOLERANCE[field] + _SLACK
    return Outcome.RIGHT if within else Outcome.WRONG


def rates(results: Sequence[FieldResult], *, fabricated: bool) -> dict[str, float | None]:
    """The four shares, in `METRICS` order, each `None` where it has nothing to divide by."""
    n = Counter(result.outcome for result in results)
    unpublished = n[Outcome.ABSTAINED] + n[Outcome.INVENTED]
    answered = n[Outcome.RIGHT] + n[Outcome.WRONG]
    published = answered + n[Outcome.MISSED]
    return {
        "invented": _share(n[Outcome.INVENTED], unpublished),
        "accuracy": _share(n[Outcome.RIGHT], answered),
        "coverage": _share(answered, published),
        "abstention": _share(n[Outcome.ABSTAINED], len(results)) if fabricated else None,
    }


def _share(part: int, whole: int) -> float | None:
    return part / whole if whole else None


def _explain(results: Sequence[FieldResult], shares: Mapping[str, float | None]) -> dict[str, str]:
    """A sentence per present metric, naming the fields behind it, for the user's grading review."""
    by = {outcome: [r for r in results if r.outcome is outcome] for outcome in Outcome}
    unpublished = len(by[Outcome.ABSTAINED]) + len(by[Outcome.INVENTED])
    answered = len(by[Outcome.RIGHT]) + len(by[Outcome.WRONG])
    published = answered + len(by[Outcome.MISSED])
    text = {
        "invented": f"{len(by[Outcome.INVENTED])} of {unpublished} unpublished fields filled in"
        + _listed(by[Outcome.INVENTED], answer=True),
        "accuracy": f"{len(by[Outcome.RIGHT])} of {answered} answered published values within "
        "tolerance" + _listed(by[Outcome.WRONG], answer=True, published=True),
        "coverage": f"{answered} of {published} published values answered"
        + _listed(by[Outcome.MISSED]),
        "abstention": f"{len(by[Outcome.ABSTAINED])} of {len(results)} fields left blank on a "
        "made-up model",
    }
    return {metric: text[metric] for metric, value in shares.items() if value is not None}


def _listed(
    results: Sequence[FieldResult], *, answer: bool = False, published: bool = False
) -> str:
    if not results:
        return ""
    parts = []
    for r in results:
        part = f"{r.slot.value} {r.field}"
        if answer:
            part += f" {r.answered:g}"
        if published:
            part += f" (published {r.published:g})"
        parts.append(part)
    return ": " + ", ".join(parts)


def _set_case(case: Case) -> SetCase:
    if not isinstance(case.input, SetCase):
        raise AttemptFailed("harness_error", f"case {case.id!r} carries no SetCase")
    return case.input


def _case(raw: Mapping[str, Any], fields: tuple[str, ...], key: Path) -> Case:
    case_id = raw["id"]
    if not raw.get("verified"):
        raise ValueError(
            f"{key}: case {case_id!r} has no `verified` date. The user's review is what makes a "
            "set an answer key (ADR-036 §7), so an unreviewed one is not graded against"
        )
    slots = tuple(ClubId(slot) for slot in raw["slots"])
    published: dict[ClubId, dict[str, float | None]] = {}
    for slot in slots:
        entries = raw["answers"][slot.value]
        published[slot] = {
            field: None if entries[field]["value"] is None else float(entries[field]["value"])
            for field in fields
        }
    tags = tuple(raw["tags"])
    year = raw["model_year"]
    counts = Counter(value is None for row in published.values() for value in row.values())
    asked = SetCase(
        make=raw["make"],
        model=raw["model"],
        model_year=year,
        slots=slots,
        published=published,
        fields=fields,
        fabricated=FABRICATED in tags,
    )
    return Case(
        id=case_id,
        prompt=(
            f"{asked.make} {asked.model}, model year {year if year is not None else 'not given'}: "
            + ", ".join(slot.value for slot in slots)
        ),
        tags=tags,
        input=asked,
        meta={"slots": len(slots), "published": counts[False], "unpublished": counts[True]},
    )


FLOW = ClubSpecFlow()
