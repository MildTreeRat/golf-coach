"""Claude recalls what a club is, and is asked to say so only where it is sure. [M12 P4]

`catalogue.py` answers the second lookup of a club. This module answers the first. It is the only
place in this repo that produces a `ClubSpec` out of nothing but a make, a model and a slot, and
everything in it is arranged around the one failure that would make the whole milestone worse than
the blank form it replaces: **a specification that is confidently wrong.**

A blank lie angle is a lie angle nobody knows, and it renders blank on the page where the golfer
can type it in. A hallucinated lie angle looks exactly like a right one, is stored with the same
provenance shape, is confirmed by a golfer who has no way to check it, and then becomes the input
to every model this milestone exists to enable. So the prompt asks for `null` wherever the model is
not confident, the response schema requires every field and makes the numeric ones nullable so a
refusal has to be written down, and the parse **keeps the null** — a `None` here is a refusal and
must never arrive downstream as a `0.0`. That is ADR-010 §2 ("no score beats a wrong one") at a new
boundary, and [ADR-026](../../../docs/decisions/026-club-specification-lookup.md) §6 is the
decision.

What that costs in schema gymnastics is recorded on `_KINDS`, because it took four rejected
requests to find and none of it is guessable from the docs. `_spec_from` reads a missing key and an
explicit `null` identically regardless, so both spellings of the refusal arrive as the same `None`.

## Structure, and why it mirrors `feedback/coach.py` without importing it

The split is the one `feedback/coach.py` already holds against the `llm` extra: `anthropic` is
imported **inside** `_sdk()` so this module imports clean on a base install, a `client=` seam lets
every test assert request and parse shape with no network, and an expected failure — no extra, no
key, an API error, a refusal — returns an outcome carrying a `note` rather than raising.

It is a structural mirror and not a shared one. `clubs/` may not import `feedback/` (ADR-008), so
`_sdk`, `_text_from` and `_note_for` are deliberately duplicated here in trimmed form. The
duplication is worth naming: what is copied is the *prose* for an SDK failure, and the cost of the
two copies drifting is a differently-worded sentence rather than a wrong number. If a third caller
ever needs it, the answer is the one `contracts/caveats.py` already took — lift the text into
`contracts/`, which is where this repo puts prose two modules need and ADR-008 forbids them to
share any other way — not a fourth copy.

## The wire shape is derived from `ClubSpec`, not written out beside it

`_SPEC_FIELDS` is built by walking `ClubSpec.model_fields`, so the JSON schema the model is given
and the coercion that reads its answer both come off the contract. A hand-written field list here
would be the R4/R6 failure this repo already fixed once (`mcp/query.py`'s `_METRIC_FIELDS`), and it
would fail in the quietest possible way: a field added to `ClubSpec` would simply never be asked
for, would come back blank for every club, and would be diagnosed months later by someone wondering
why nothing ever fills it in.

An annotation `_KINDS` has no entry for raises at import. That is R8's wiring-bug half — a new
field *type* is a decision to make once, not a club silently missing a number.

Stdlib + pydantic, `contracts` only (ADR-008). The `llm` extra, `settings.anthropic_api_key` and
`settings.coaching_model` are reused; this milestone adds no configuration.
"""

from __future__ import annotations

import json
import math
from collections.abc import Callable, Iterable
from dataclasses import dataclass
from datetime import UTC, datetime
from typing import Any

from pydantic import ValidationError

from golf_coach.contracts.club import ClubId, parse_club
from golf_coach.contracts.club_spec import (
    ClubSpec,
    ShaftFlex,
    ShaftMaterial,
    SpecProvenance,
    parse_shaft_flex,
    parse_shaft_material,
)

#: Four times `coach.py`'s ceiling, because the two shapes fail differently. A truncated coaching
#: paragraph is still a paragraph and is served with an admission; a truncated JSON object is not
#: JSON and the whole lookup is lost. A full set is fourteen slots of two dozen fields, and
#: `max_tokens` bounds thinking and output together on this model, so this is sized for the largest
#: request the page can make rather than the typical one.
MAX_TOKENS = 16000

#: `coach.py` runs at `low` because it summarises numbers it was handed. This asks the model to
#: produce numbers from its own memory and, harder, to know which of them it does not actually
#: know — a calibration judgement, and the one thing here worth spending on. A lookup happens once
#: per club in a bag's lifetime and is then kept by `catalogue.remember`, so the cost of the high
#: rung is paid once and the cost of a wrong loft is paid on every shot after it.
EFFORT = "high"

#: Asked of the caller and never of the model. `make`, `model` and `model_year` are echoed back
#: onto every spec exactly as they were typed, which is not politeness — it is what keeps
#: `catalogue.key_for` able to find the row again. A model that helpfully returned "Titleist Golf"
#: for "Titleist", or established a year the golfer left blank, would produce a row stored under a
#: key the next identical lookup misses, and the catalogue would call the API forever while looking
#: like it was working. Anything the model wants to say about the generation goes in `notes`.
#:
#: `club` is excluded for a different reason: it *is* asked for, because a set call has to say
#: which slot each object describes, but the value that lands on the spec is the requested slot and
#: never the model's echo.
_GIVEN_FIELDS = frozenset({"club", "make", "model", "model_year"})


@dataclass(frozen=True)
class ClubLookupOutcome:
    """What one lookup produced. An empty `specs` means it did not happen, and `note` says why.

    Mirrors `CoachingOutcome`: an expected failure is a value and not an exception, because the
    caller's job is to show it to the golfer and carry on. The page still renders — with an empty
    form to type into, which is exactly the M9 state this milestone is improving on and not a
    regression from it.

    `provenance` is one block for the whole outcome rather than one per spec, because a set is one
    call, at one moment, from one model. `ClubSpec` carries no provenance field of its own by
    design (ADR-026 §4); `BagEntry` is where it lands once a golfer confirms.
    """

    specs: tuple[ClubSpec, ...] = ()
    provenance: SpecProvenance | None = None
    #: Why there is nothing at all — phrased for the golfer looking at the form, not for a log.
    #: What was odd about a *partial* answer goes in `provenance.notes` instead, because it travels
    #: with the specification it qualifies rather than with the attempt.
    note: str | None = None


@dataclass(frozen=True)
class _FieldKind:
    """How one annotation is asked for over the wire, and how the answer is read back.

    One record rather than two dicts keyed by the same annotations (R5). The schema and the parse
    are two halves of a single decision about a type — a `float | None` is asked for as an optional
    number *because* an optional number is what `_as_float` will accept — and splitting them into
    parallel tables is how the two come to disagree.
    """

    json_schema: dict[str, Any]
    coerce: Callable[[object], Any]


def _as_str(value: object) -> str:
    """Text, or the blank. A `null` in a string field is "unknown", which is `""` on `ClubSpec`."""
    return value if isinstance(value, str) else ""


def _as_float(value: object) -> float | None:
    """A finite number, or `None`. Anything that is not a number is `None` and never a zero.

    Three guards, each against a value that would otherwise arrive looking like a measurement:

    - **`bool` is rejected before `int`**, because `isinstance(True, int)` is true in Python and a
      `true` in a numeric field would become `1.0` — a lie angle of one degree, indistinguishable
      from a real reading;
    - **non-finite is rejected**, because `json.loads` accepts the bare `NaN` and `Infinity`
      literals by default, and a NaN loft compares false against every band it is ever put in
      while printing as a number;
    - **a clean numeric string is accepted**, since `"30.5"` is a number written as text and
      reading it is a parse, not a guess. `"unknown"`, `"N/A"`, `"~30"` and `""` are not numbers
      and are `None` — the refusal `parse_club` makes, for the same asymmetry.
    """
    if isinstance(value, bool):
        return None
    if isinstance(value, int | float):
        number = float(value)
    elif isinstance(value, str):
        try:
            number = float(value.strip())
        except ValueError:
            return None
    else:
        return None
    return number if math.isfinite(number) else None


def _as_bool(value: object) -> bool | None:
    """A real boolean, or `None`. `None` is not `False` — see `ClubSpec.adjustable_hosel`."""
    return value if isinstance(value, bool) else None


def _as_pair(value: object) -> tuple[float, float] | None:
    """Two finite numbers, or `None`. A one-ended or three-ended range is not a range."""
    if not isinstance(value, list) or len(value) != 2:
        return None
    low, high = (_as_float(item) for item in value)
    return None if low is None or high is None else (low, high)


def _enum_coercer(parse: Callable[[str], Any]) -> Callable[[object], Any]:
    """Route a closed vocabulary through its parser in `contracts/club_spec.py`, and nowhere else.

    The model is *asked* for the canonical spelling by the system prompt, and this still refuses
    anything the contract does not recognise rather than trusting the string it was given. That is
    P2's lesson at the other boundary: `api/app.py` parses a typed flex through the same function
    so `"S"` has one meaning, and a second parse site here would be a second place for a graphite
    shaft to end up filed as steel.
    """
    return lambda value: parse(value) if isinstance(value, str) else None


#: The Python annotation -> how it crosses the wire. Keyed by the annotation object itself, which
#: compares and hashes structurally, so `ClubSpec`'s own declaration is what selects the row.
#:
#: **Every field is `required`, and a number refuses by being `null`.** That combination reads like
#: a contradiction and is the point: `required` means the model cannot quietly drop the tail of an
#: object it lost interest in, and nullable means the only way to satisfy that is to write `null`
#: on purpose. An omission and a refusal are then not the same event.
#:
#: Getting there cost four 400s from the real API, and the shape is bounded on three sides at once:
#:
#: - **`minItems` other than 0 or 1 is rejected outright**, so the pair's length is `_as_pair`'s
#:   job rather than the schema's;
#: - **at most 16 union-typed parameters.** `ClubSpec` has 17 optional fields, so the two shaft
#:   enums cross as plain (non-null) strings — they are text, and `""` is already their blank —
#:   leaving the 12 floats, 2 bools and 1 array at fifteen unions. **One under the limit**, which
#:   is why `tests/clubs/test_lookup.py` pins the count: a sixteenth nullable non-string field on
#:   `ClubSpec` must fail there rather than 400 in front of a golfer;
#: - **at most 24 optional parameters** — and, separately, an all-but-unexplained `Schema is too
#:   complex.` once enough of them are optional. That one is worth recording precisely, because it
#:   is the opposite of the intuition: the same 24 fields compile in **20 seconds** when every one
#:   is `required`, and take **180 seconds to be rejected** when fifteen are optional. It is
#:   optionality that the grammar compiler pays for, not the number of fields. Making everything
#:   required is therefore not a workaround for the complexity limit; it is what the limit is
#:   asking for, and it happens to be the design that was wanted anyway.
#:
#: The two enums cross as plain strings, not as a JSON `enum`. The vocabulary is in the prompt and
#: the refusal is in `parse_shaft_*`, which already accepts the letter codes printed on a shaft
#: band; pinning the wire to the canonical spellings would make an `"S"` a schema violation rather
#: than a flex, and would put a second copy of the vocabulary here.
_KINDS: dict[Any, _FieldKind] = {
    str: _FieldKind({"type": "string"}, _as_str),
    float | None: _FieldKind({"type": ["number", "null"]}, _as_float),
    bool | None: _FieldKind({"type": ["boolean", "null"]}, _as_bool),
    ShaftMaterial | None: _FieldKind({"type": "string"}, _enum_coercer(parse_shaft_material)),
    ShaftFlex | None: _FieldKind({"type": "string"}, _enum_coercer(parse_shaft_flex)),
    tuple[float, float] | None: _FieldKind(
        {"type": ["array", "null"], "items": {"type": "number"}}, _as_pair
    ),
}


def _spec_fields() -> dict[str, _FieldKind]:
    """Every `ClubSpec` field the model is asked for, walked off the contract (R6).

    Raises at import when a field's annotation has no `_KINDS` row. That is R8's wiring-bug half,
    and it is the whole reason this is derived rather than listed: the alternative failure is
    silent — a new field is simply never requested, comes back blank for every club, and looks
    exactly like a model that never knew it.
    """
    fields: dict[str, _FieldKind] = {}
    for name, model_field in ClubSpec.model_fields.items():
        if name in _GIVEN_FIELDS:
            continue
        kind = _KINDS.get(model_field.annotation)
        if kind is None:
            raise ValueError(
                f"ClubSpec.{name} is annotated {model_field.annotation!r}, which clubs/lookup.py "
                "has no wire shape for — add a _KINDS row, or the field is never looked up"
            )
        fields[name] = kind
    return fields


_SPEC_FIELDS = _spec_fields()

#: The model's own account of what it was unsure about, kept verbatim in `SpecProvenance.notes`.
#: Named apart from the spec fields because it is not one: it describes the answer rather than the
#: club, and it is what makes a blank beside it legible instead of merely empty.
_NOTES_KEY = "notes"

SYSTEM_PROMPT = f"""\
You are looking up the published specification of one slot of one golf club model — the numbers a
manufacturer prints for a club as it leaves the factory. This is recall about a manufactured
object. It is not an estimate, and it is not advice.

The single rule that matters: **do not report any value you are not confident of.** Answer `null`
for a number you do not know and an empty string for a text field you do not know. Every field is
required, so a blank is something you write down deliberately, and it is the right answer far more
often than a value is. A blank is visible to the golfer and they can fill it in; a wrong number is
invisible, gets confirmed by someone with no way to check it, and is then used as fact. You are
never penalised here for a blank, and an object of nulls and empty strings with a `notes` line is a
good answer when that is what you actually know.

That rule forbids, specifically:

- interpolating a value between two slots you do know;
- carrying a value across from a different model, a different generation, or a similar club;
- deriving one specification from another — a lie angle from a length, a bounce from a grind;
- reporting a number you half-remember. If you would not bet on the digit, it is null.

How to answer:

- Report the manufacturer's **stock** specification for the club as sold. If a club is commonly
  built to something other than stock, say so in `notes` rather than reporting the common build.
- Units are in the field names: `_deg` degrees, `_mm` millimetres, `_g` grams, `_in` inches,
  `_g_cm2` grams per square centimetre. `swing_weight` is the letter-and-digit code, e.g. "D2".
- `shaft_material` is one of: {", ".join(ShaftMaterial)}.
- `shaft_flex` is one of: {", ".join(ShaftFlex)}. The letter codes are understood too.
- `loft_deg` is the loft the club is set to; `loft_range_deg` is the adjustable range as exactly
  two numbers, [min, max], and is null for a club whose hosel does not move.
- Use `notes` for one sentence on anything you were unsure of, which generation you took the
  specification from, or why a field beside it is blank. Leave it empty when there is nothing to
  say. Do not restate the make, model or year — they were given to you and are recorded already.

When several slots are requested, answer them together and keep them consistent: the loft, length
and lie of a set form a progression. A slot you are unsure of is still null, and never filled with
the value that progression would predict."""


def look_up_club(
    make: str,
    model: str,
    model_year: int | None,
    club: ClubId,
    *,
    llm_model: str,
    api_key: str | None = None,
    client: Any | None = None,
) -> ClubLookupOutcome:
    """One slot of one model, looked up. Never raises for an expected failure.

    The positional four are `catalogue.lookup`'s signature exactly, so the cache hit and the cache
    miss are called the same way and the route composing them (P5) has nothing to reorder.

    `llm_model` and not `model`, which is the collision this milestone was always going to have:
    `model` is a club's model name in `contracts/club_spec.py`, in the catalogue key and on this
    call, so the Anthropic model id is the one that takes the qualifier. Reusing `model` for the
    more famous meaning would make `look_up_club(make, model=...)` mean two different things
    depending on which file the reader arrived from.
    """
    return _look_up(make, model, model_year, (club,), llm_model, api_key, client)


def look_up_set(
    make: str,
    model: str,
    model_year: int | None,
    slots: Iterable[ClubId],
    *,
    llm_model: str,
    api_key: str | None = None,
    client: Any | None = None,
) -> ClubLookupOutcome:
    """Every requested slot of one model, in **one** call, returning one spec per slot.

    One call and not one per slot, because a set's lofts, lies and lengths are a progression and
    seven independent answers are seven chances to break it (ADR-026 §6). The model sees the whole
    set it is describing, which is also what lets it leave one slot blank without that slot quietly
    inheriting the shape of the six around it.

    Duplicate slots collapse and the order is the caller's. A slot the model returned nothing for
    comes back as a spec carrying its identity and nothing else — the honest shape, and the one
    P6's form renders as a row of blanks to type into rather than as a club nobody asked about.
    """
    requested = tuple(dict.fromkeys(slots))
    if not requested:
        # A wiring bug would raise (R8); an empty picker is a data condition, and a page asking for
        # nothing should cost nothing rather than a 500.
        return ClubLookupOutcome(note="no clubs were requested, so nothing was looked up.")
    return _look_up(make, model, model_year, requested, llm_model, api_key, client)


def _look_up(
    make: str,
    model: str,
    model_year: int | None,
    slots: tuple[ClubId, ...],
    llm_model: str,
    api_key: str | None,
    client: Any | None,
) -> ClubLookupOutcome:
    """The one request both public calls make. A single club is a set of one.

    Kept as one function because the two differ only in how many slots they ask for. Two copies
    would be two places for the refusal rules, the truncation branch and the slot matching to
    drift, and the single-club path is the one that runs most often.
    """
    sdk = _sdk()
    if client is None:
        # The extra before the key, for `coach.py`'s reason: with no `anthropic` installed there is
        # nothing to authenticate against, so answering that install with "no API key" sends the
        # reader to `.env` — where the key is sitting, perfectly valid — instead of to pip.
        if sdk is None:
            return ClubLookupOutcome(
                note=(
                    "no specifications looked up: the `llm` extra is not installed "
                    "(pip install -e '.[llm]'). You can still type the club in by hand."
                )
            )
        if not api_key:
            return ClubLookupOutcome(
                note=(
                    "no specifications looked up: no Anthropic API key is configured, so the "
                    "lookup was skipped (set GOLF_ANTHROPIC_API_KEY). You can still type the club "
                    "in by hand."
                )
            )
        client = sdk.Anthropic(api_key=api_key)

    try:
        response = client.messages.create(
            model=llm_model,
            max_tokens=MAX_TOKENS,
            # Thinking is on by default on Opus 5; saying so explicitly keeps this request honest
            # if the default ever moves. `budget_tokens`, temperature, top_p and top_k are all
            # removed on this model and 400 — none of them belong here.
            thinking={"type": "adaptive"},
            output_config={
                "effort": EFFORT,
                # The schema is what stops the answer being prose, and what stops the model
                # inventing a field: `additionalProperties` is false and `club` is an enum of the
                # slots asked for, so an answer can be short but never off-topic.
                "format": {"type": "json_schema", "schema": _response_schema(slots)},
            },
            # No `cache_control`, unlike `coach.py`. This prompt is well under the 512-token floor
            # Opus 5 will cache at, and a club is looked up once in a bag's life and then served
            # from the catalogue, so there is no second identical request to hit a cache anyway.
            system=SYSTEM_PROMPT,
            messages=[{"role": "user", "content": _brief(make, model, model_year, slots)}],
        )
    except Exception as exc:  # narrowed in `_note_for`; the SDK may be absent entirely
        return ClubLookupOutcome(note=_note_for(exc, sdk))

    stop_reason = getattr(response, "stop_reason", None)
    if stop_reason == "refusal":
        return ClubLookupOutcome(
            note="no specifications looked up: the model declined to answer for this club."
        )
    if stop_reason == "max_tokens":
        # Nothing to salvage and nothing to serve half of: a JSON object cut off mid-key is not
        # JSON. So this is a refusal with a reason, where `coach.py` can admit truncation and still
        # serve the prose above it.
        return ClubLookupOutcome(
            note=(
                "no specifications looked up: the answer was cut off before it finished. "
                "Try fewer clubs at once."
            )
        )

    try:
        payload = json.loads(_text_from(response))
    except ValueError:
        # `output_config.format` makes this unreachable against the real API and reachable against
        # everything else, which is exactly when it must not raise.
        return ClubLookupOutcome(
            note="no specifications looked up: the model's answer was not readable."
        )

    rows = payload.get("clubs") if isinstance(payload, dict) else None
    if not isinstance(rows, list):
        return ClubLookupOutcome(
            note="no specifications looked up: the model's answer named no clubs."
        )

    matched, notes = _by_slot(rows, slots)
    specs: list[ClubSpec] = []
    for slot in slots:
        row = matched.get(slot)
        if row is None:
            notes.append(f"{slot.value}: nothing was returned for this slot.")
        spec, trouble = _spec_from(row or {}, slot, make, model, model_year)
        specs.append(spec)
        notes += trouble

    return ClubLookupOutcome(
        specs=tuple(specs),
        provenance=SpecProvenance(
            # The model that answered, not the one that was asked, so a server-side substitution is
            # recorded rather than hidden — `coach.py`'s `CoachingProvenance` reads the same field.
            source=f"llm:{getattr(response, 'model', None) or llm_model}",
            retrieved_at=datetime.now(UTC),
            notes=" ".join(notes),
        ),
    )


def _brief(make: str, model: str, model_year: int | None, slots: tuple[ClubId, ...]) -> str:
    """What was asked for — the identity fields, and deliberately nothing else.

    The year is stated as unknown rather than omitted, because an omitted line reads as "any year"
    and this has to read as "the golfer did not tell us". That is the model's cue to name the
    generation it answered from in `notes` instead of silently picking the newest.
    """
    year = str(model_year) if model_year is not None else "not given"
    return (
        f"make: {make}\n"
        f"model: {model}\n"
        f"model year: {year}\n"
        f"slots: {', '.join(slot.value for slot in slots)}"
    )


def _response_schema(slots: tuple[ClubId, ...]) -> dict[str, Any]:
    """The JSON schema the answer is constrained to, built from `ClubSpec` and the asked-for slots.

    **Everything is `required`**, and a refusal is written rather than omitted: `null` for a
    number, `""` for a text field. That is what makes a blank a deliberate act instead of an
    ambiguity — a model that dropped the tail of an object and one that declined four lie angles
    would otherwise be indistinguishable. It is also, per `_KINDS`, the only shape the API's
    grammar compiler will accept for a field list this long.

    `additionalProperties` is `False` and is not optional: the API rejects an object schema that
    leaves it out. Between the two, the model may refuse anything it was asked and answer nothing
    it was not.

    `club` is an `enum` of exactly the requested slots, so the model cannot answer about a club
    nobody asked for — the schema doing at this boundary what `parse_club` does at every other.
    """
    properties: dict[str, Any] = {
        "club": {"type": "string", "enum": [slot.value for slot in slots]},
        **{name: dict(kind.json_schema) for name, kind in _SPEC_FIELDS.items()},
        _NOTES_KEY: {"type": "string"},
    }
    return {
        "type": "object",
        "additionalProperties": False,
        "required": ["clubs"],
        "properties": {
            "clubs": {
                "type": "array",
                "items": {
                    "type": "object",
                    "additionalProperties": False,
                    "required": list(properties),
                    "properties": properties,
                },
            }
        },
    }


def _by_slot(
    rows: list[Any], slots: tuple[ClubId, ...]
) -> tuple[dict[ClubId, dict[str, Any]], list[str]]:
    """Returned objects to the slots that were asked for, plus anything odd about the mapping.

    Every `club` string goes through **`parse_club`** (`club.py:185`), so this is not a second
    place where text becomes a `ClubId`. A row naming a club nobody requested is dropped rather
    than added: the point of asking for a fixed set of slots is that the answer is about them.

    A second row for a slot already answered keeps the **first**, and says so. There is no
    principled way to choose between two answers for one club — the model has contradicted itself
    — and taking the last would be arbitrary in a way that looks decisive.
    """
    matched: dict[ClubId, dict[str, Any]] = {}
    notes: list[str] = []
    wanted = set(slots)
    for row in rows:
        if not isinstance(row, dict):
            continue
        raw = row.get("club")
        club = parse_club(raw) if isinstance(raw, str) else None
        if club is None or club not in wanted:
            continue
        if club in matched:
            notes.append(f"{club.value}: two answers came back and the first was kept.")
            continue
        matched[club] = row
    return matched, notes


def _spec_from(
    row: dict[str, Any],
    club: ClubId,
    make: str,
    model: str,
    model_year: int | None,
) -> tuple[ClubSpec, list[str]]:
    """One returned object to a `ClubSpec`, with the identity taken from the request.

    Nothing here fills a blank. Every field goes through its `_KINDS` coercion, every coercion
    returns absence rather than a substitute, and a missing key coerces exactly as an explicit
    `null` does — both mean the model did not answer, and neither may become a zero.

    A `ValidationError` costs the numbers and keeps the identity. The one rule on `ClubSpec` that
    can still fail after coercion is the cross-field `loft_range_deg` ordering, and pydantic
    reports a model-level failure without naming a field, so there is nothing to selectively drop.
    Dropping `loft_range_deg` by name was the alternative and is rejected: it would put the
    contract's only cross-field rule in a second file, where it would go on working after the rule
    there changed.
    """
    values = {name: kind.coerce(row.get(name)) for name, kind in _SPEC_FIELDS.items()}
    identity: dict[str, Any] = {
        "club": club,
        "make": make,
        "model": model,
        "model_year": model_year,
    }
    try:
        spec = ClubSpec(**identity, **values)
    except ValidationError:
        return (
            ClubSpec(**identity),
            [f"{club.value}: the specification did not hold together and was not kept."],
        )

    note = row.get(_NOTES_KEY)
    if isinstance(note, str) and note.strip():
        return spec, [f"{club.value}: {note.strip()}"]
    return spec, []


def _sdk() -> Any | None:
    """The `anthropic` module, or `None` when the `llm` extra is not installed.

    The single place this module touches the extra, imported inside the function so
    `tests/api/test_pipeline_imports.py` keeps passing and the offline CLIs keep running on a base
    install. `feedback/coach.py:311` holds the same seam for the same reason; ADR-008 is why this
    is a copy rather than an import.
    """
    try:
        import anthropic
    except ImportError:
        return None
    return anthropic


def _text_from(response: object) -> str:
    """Concatenate the text blocks, skipping thinking blocks.

    Still needed with a JSON schema in force: the schema constrains what the *text* blocks say and
    does not remove the thinking block in front of them, and a thinking block concatenated into the
    JSON is a parse failure that would read as the model having answered badly.
    """
    blocks = getattr(response, "content", None) or []
    parts = [
        block.text
        for block in blocks
        if getattr(block, "type", None) == "text" and getattr(block, "text", "")
    ]
    return "\n".join(parts).strip()


#: Long enough for the API's own sentence, short enough that a note stays a note.
_MAX_DETAIL = 200


def _note_for(exc: Exception, anthropic: Any | None, *, prefix: str = "no club lookup: ") -> str:
    """An SDK exception to the sentence the golfer should read beside the empty form.

    Most specific first, per the SDK's own guidance, and the branches are `coach.py`'s because the
    failures are the same failures — a stale model id and a rate limit want different words, and
    collapsing both into "an error occurred" is how a bad `coaching_model` goes unnoticed for a
    month.

    The status branch keeps the API's own explanation for the reason `coach.py` records: a
    malformed request and an exhausted credit balance arrive as the same 400, and only the body
    says which. Everything is read through `getattr`, so a diagnostic cannot raise while explaining
    a failure.
    """
    if anthropic is not None:
        if isinstance(exc, anthropic.NotFoundError):  # type: ignore[attr-defined]
            return (
                f"{prefix}the API rejected the configured model id. Check `coaching_model` in "
                f"config.py ({exc})."
            )
        if isinstance(exc, anthropic.AuthenticationError):  # type: ignore[attr-defined]
            return f"{prefix}the configured Anthropic API key was rejected."
        if isinstance(exc, anthropic.RateLimitError):  # type: ignore[attr-defined]
            return f"{prefix}the request was rate limited. Try again in a moment."
        if isinstance(exc, anthropic.OverloadedError):  # type: ignore[attr-defined]
            # 529 is the API being busy and says nothing about this request. Named apart from the
            # status branch below so it reads as "wait", which is the whole of the fix.
            return f"{prefix}the API is overloaded right now. Try again in a moment."
        if isinstance(exc, anthropic.APIStatusError):  # type: ignore[attr-defined]
            status = getattr(exc, "status_code", None)
            code = str(status) if status is not None else "an error"
            detail = _api_detail(exc)
            if detail:
                return f"{prefix}the API returned {code}: {detail}"
            return f"{prefix}the API returned {code}."
        if isinstance(exc, anthropic.APIConnectionError):  # type: ignore[attr-defined]
            return f"{prefix}could not reach the API. You can still type the club in by hand."
    return f"{prefix}the request failed ({type(exc).__name__}: {exc})."


def _api_detail(exc: Exception) -> str:
    """The API's own sentence about a failure, or `""`.

    `APIStatusError.message` is assembled by the SDK as ``Error code: 400 - {…whole body dict…}``,
    which is accurate and unreadable; the parsed body carries the same sentence on its own.
    """
    body = getattr(exc, "body", None)
    if isinstance(body, dict):
        error = body.get("error")
        if isinstance(error, dict) and isinstance(error.get("message"), str):
            detail: str = error["message"].strip()
            if detail:
                return detail[:_MAX_DETAIL]
    message = getattr(exc, "message", None)
    if isinstance(message, str) and message.strip():
        return message.strip()[:_MAX_DETAIL]
    return ""
