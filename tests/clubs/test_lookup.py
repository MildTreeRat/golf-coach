"""The lookup: a null stays a null, and the identity stays the golfer's.

Two failures shape almost every pin here, and neither shows up as a crash:

- **a refusal that becomes a number.** The model is asked to answer `null` where it is not sure,
  and every step between that null and `ClubSpec` is a place it could quietly become `0.0` — a
  `true` in a numeric field (Python says `isinstance(True, int)`), a bare `NaN` (Python's `json`
  accepts the literal), a missing key, the word "unknown". Each is pinned, because a blank renders
  blank on the form and a zero renders as a club that would be unplayable;
- **an identity that drifts.** `make`, `model` and `model_year` are echoed from the request and
  never taken from the answer, because they are the catalogue key. A model that returns "Titleist
  Golf" for "Titleist" would have its answer stored under a key the next identical lookup misses,
  and the catalogue would call the API forever while looking exactly like it was working.

Every test runs on the base install with an injected client: no network, no `llm` extra. That the
module *imports* without `anthropic` is pinned where the other extras boundaries live,
`tests/api/test_pipeline_imports.py`.
"""

from __future__ import annotations

import json
import math
from typing import Any

import pytest

from golf_coach.clubs.lookup import _GIVEN_FIELDS as GIVEN_FIELDS
from golf_coach.clubs.lookup import (
    MAX_TOKENS,
    ClubLookupOutcome,
    look_up_club,
    look_up_set,
)
from golf_coach.contracts.club import ClubId
from golf_coach.contracts.club_spec import ClubSpec, ShaftFlex, ShaftMaterial

LLM_MODEL = "claude-opus-5"

#: Every field of a 7 iron the model is asked for, all of them refused. The starting point for most
#: pins here: a test overrides the one field it is about, so a stray zero has nowhere to hide.
_ALL_NULL: dict[str, Any] = {
    "club": "7i",
    "head_type": "",
    "set_composition": "",
    "loft_deg": None,
    "lie_deg": None,
    "bounce_deg": None,
    "grind": "",
    "offset_mm": None,
    "face_angle_deg": None,
    "head_weight_g": None,
    "adjustable_hosel": None,
    "loft_range_deg": None,
    "shaft_model": "",
    "shaft_material": None,
    "shaft_flex": None,
    "shaft_weight_g": None,
    "shaft_torque_deg": None,
    "shaft_kick_point": "",
    "length_in": None,
    "swing_weight": "",
    "total_weight_g": None,
    "grip": "",
    "cor": None,
    "moi_g_cm2": None,
    "usga_conforming": None,
    "notes": "",
}


def _row(**overrides: Any) -> dict[str, Any]:
    return {**_ALL_NULL, **overrides}


class _FakeMessages:
    """Records every request and returns whatever the test wants back."""

    def __init__(self, response: object | Exception) -> None:
        self.response = response
        self.calls: list[dict] = []

    def create(self, **kwargs: Any) -> object:
        self.calls.append(kwargs)
        if isinstance(self.response, Exception):
            raise self.response
        return self.response


class _FakeClient:
    def __init__(self, response: object | Exception) -> None:
        self.messages = _FakeMessages(response)


class _Block:
    def __init__(self, text: str, type_: str = "text") -> None:
        self.text = text
        self.type = type_


class _Response:
    def __init__(self, text: str, *, stop_reason: str = "end_turn", thinking: str = "") -> None:
        blocks = [_Block(thinking, "thinking")] if thinking else []
        self.content = [*blocks, _Block(text)]
        self.stop_reason = stop_reason
        self.model = LLM_MODEL


def _answering(*rows: dict[str, Any], **kwargs: Any) -> _FakeClient:
    return _FakeClient(_Response(json.dumps({"clubs": list(rows)}), **kwargs))


def _t150(client: _FakeClient, club: ClubId = ClubId.SEVEN_IRON) -> ClubLookupOutcome:
    return look_up_club("Titleist", "T150", 2023, club, llm_model=LLM_MODEL, client=client)


# --------------------------------------------------------------------------- the schema


def test_every_club_spec_field_is_asked_for() -> None:
    """The one pin that catches a field added to `ClubSpec` and never wired up here.

    Derived from the contract and not from the module under test: `ClubSpec` is the registry, and a
    field added there has to show up in the request or it comes back blank for every club forever,
    with nothing to distinguish that from a model that never knew it (R6).
    """
    client = _answering(_row())
    _t150(client)

    schema = client.messages.calls[0]["output_config"]["format"]["schema"]
    asked = set(schema["properties"]["clubs"]["items"]["properties"])

    assert set(ClubSpec.model_fields) - GIVEN_FIELDS <= asked
    # And the four the caller owns are not asked for — except `club`, which is how a set says
    # which slot it is describing.
    assert "club" in asked
    assert {"make", "model", "model_year"} & asked == set()


#: Three of the four ceilings the API's structured-output compiler enforces on one schema — the
#: fourth, `minItems`, is on the array's shape and is pinned just below. All were found by real
#: 400s during M12 P4, and none is guessable from the request that fails:
#:
#: - at most 16 parameters with union types (``limit: 16 parameters with unions``);
#: - at most 24 optional parameters (``too many optional parameters (25) … limit: 24``);
#: - and a bare ``Schema is too complex.`` that is really about optionality, not size — the same
#:   24 fields compile in 20s when all are required and are rejected after 180s when 15 are not.
#:
#: Pinned here because a schema is only validated by the API, every test in this file injects a
#: client, and a `ClubSpec` that grows past a ceiling would otherwise 400 in front of a golfer.
_UNION_LIMIT = 16
_OPTIONAL_LIMIT = 24


def test_every_field_is_required_so_a_refusal_has_to_be_written_down() -> None:
    """Required *and* nullable is the trick that makes an omission impossible.

    Without `required`, a model that lost interest could drop the tail of an object and the missing
    fields would be indistinguishable from fields it deliberately refused. Required forces a `null`
    or an empty string on purpose — and, per `_KINDS`, it is also the only shape the compiler
    accepts for a field list this long.
    """
    client = _answering(_row())
    _t150(client)

    item = client.messages.calls[0]["output_config"]["format"]["schema"]["properties"]["clubs"][
        "items"
    ]

    assert set(item["required"]) == set(item["properties"])
    # Not merely conventional: the API rejects an object schema that omits it.
    assert item["additionalProperties"] is False


def test_nothing_is_optional_because_optionality_is_what_the_compiler_pays_for() -> None:
    """The counter-intuitive one, and the reason it is pinned rather than left to hold by luck."""
    client = _answering(_row())
    _t150(client)

    item = client.messages.calls[0]["output_config"]["format"]["schema"]["properties"]["clubs"][
        "items"
    ]
    optional = set(item["properties"]) - set(item["required"])

    assert optional == set(), (
        f"{sorted(optional)} optional parameters — the limit is {_OPTIONAL_LIMIT}, and well "
        "before it the schema is rejected as too complex"
    )


def test_the_union_count_stays_inside_the_limit() -> None:
    """Fifteen of a permitted sixteen. **One field of margin**, which is why this is a test.

    A sixteenth nullable non-string field added to `ClubSpec` must fail here, loudly, rather than
    400 every lookup in front of a golfer. The two shaft enums cross as plain strings for exactly
    this reason — they are text, `""` is already their blank, and un-nulling them is what bought
    the margin that exists.
    """
    client = _answering(_row())
    _t150(client)

    properties = client.messages.calls[0]["output_config"]["format"]["schema"]["properties"][
        "clubs"
    ]["items"]["properties"]

    unions = [
        name
        for name, prop in properties.items()
        if isinstance(prop.get("type"), list) or "anyOf" in prop
    ]
    assert len(unions) <= _UNION_LIMIT, (
        f"{len(unions)} union-typed parameters exceeds the API's limit of {_UNION_LIMIT} — "
        f"every lookup would 400. Unions: {sorted(unions)}"
    )
    # And a number really is refusable, which is the whole point of spending the margin on them.
    assert {"loft_deg", "lie_deg", "adjustable_hosel", "loft_range_deg"} <= set(unions)


def test_the_array_carries_no_min_items() -> None:
    """The same subset rejects any `minItems` but 0 or 1; `_as_pair` holds the length instead."""
    client = _answering(_row())
    _t150(client)

    properties = client.messages.calls[0]["output_config"]["format"]["schema"]["properties"][
        "clubs"
    ]["items"]["properties"]

    assert "minItems" not in properties["loft_range_deg"]
    assert "maxItems" not in properties["loft_range_deg"]


def test_the_slot_enum_is_exactly_what_was_asked_for() -> None:
    client = _answering()
    look_up_set(
        "Titleist", "T150", 2023, [ClubId.SEVEN_IRON, ClubId.EIGHT_IRON],
        llm_model=LLM_MODEL, client=client,
    )

    schema = client.messages.calls[0]["output_config"]["format"]["schema"]
    assert schema["properties"]["clubs"]["items"]["properties"]["club"]["enum"] == ["7i", "8i"]


def test_request_shape_matches_what_opus_5_accepts() -> None:
    """The removed parameters are a 400, not a warning — pin their absence."""
    client = _answering(_row())
    _t150(client)

    kwargs = client.messages.calls[0]
    assert kwargs["model"] == LLM_MODEL
    assert kwargs["thinking"] == {"type": "adaptive"}
    assert kwargs["output_config"]["effort"] == "high"
    assert kwargs["output_config"]["format"]["type"] == "json_schema"
    for removed in ("temperature", "top_p", "top_k", "budget_tokens"):
        assert removed not in kwargs
    # A truncated JSON object is not JSON, so this ceiling has to fit the largest set the page can
    # ask for — it cannot be sized to the typical single club.
    assert kwargs["max_tokens"] == MAX_TOKENS >= 8192


def test_the_brief_says_the_year_is_unknown_rather_than_omitting_it() -> None:
    """An omitted year reads as "any year"; this has to read as "the golfer did not say"."""
    client = _answering(_row())
    look_up_club("Titleist", "T150", None, ClubId.SEVEN_IRON, llm_model=LLM_MODEL, client=client)

    brief = client.messages.calls[0]["messages"][0]["content"]
    assert "model year: not given" in brief
    assert "slots: 7i" in brief


def test_the_prompt_carries_the_closed_vocabularies_it_expects_back() -> None:
    """Derived from the enums, so a member added to either is offered without editing prose."""
    client = _answering(_row())
    _t150(client)

    prompt = client.messages.calls[0]["system"]
    for member in (*ShaftMaterial, *ShaftFlex):
        assert member.value in prompt


# --------------------------------------------------------------------------- a null stays a null


def test_a_refused_spec_is_none_and_never_zero() -> None:
    outcome = _t150(_answering(_row()))

    (spec,) = outcome.specs
    assert spec.loft_deg is None
    assert spec.lie_deg is None
    assert spec.offset_mm is None
    assert spec.adjustable_hosel is None
    assert spec.usga_conforming is None
    assert spec.loft_range_deg is None
    assert spec.swing_weight == ""


def test_a_missing_key_reads_the_same_as_an_explicit_null() -> None:
    """Both mean "the model did not answer", so neither may become a number."""
    outcome = _t150(_answering({"club": "7i"}))

    (spec,) = outcome.specs
    assert spec.loft_deg is None
    assert spec.grind == ""
    assert spec.club is ClubId.SEVEN_IRON


def test_a_boolean_in_a_numeric_field_is_not_a_one() -> None:
    """`isinstance(True, int)` is true in Python, so `true` would otherwise be a 1.0 lie angle."""
    outcome = _t150(_answering(_row(lie_deg=True, loft_deg=False)))

    (spec,) = outcome.specs
    assert spec.lie_deg is None
    assert spec.loft_deg is None


@pytest.mark.parametrize("literal", ["NaN", "Infinity", "-Infinity"])
def test_a_non_finite_number_is_refused(literal: str) -> None:
    """Python's `json` parses the bare literals, and a NaN loft fails every band while printing."""
    text = json.dumps({"clubs": [_row()]}).replace('"loft_deg": null', f'"loft_deg": {literal}')
    outcome = _t150(_FakeClient(_Response(text)))

    (spec,) = outcome.specs
    assert spec.loft_deg is None or not math.isfinite(spec.loft_deg)
    assert spec.loft_deg is None


@pytest.mark.parametrize(
    ("sent", "expected"),
    [("30.5", 30.5), (" 30.5 ", 30.5), ("unknown", None), ("~30", None), ("", None), ("30°", None)],
)
def test_a_numeric_string_is_read_and_a_hedge_is_not(sent: str, expected: float | None) -> None:
    """Reading `"30.5"` is a parse; reading `"~30"` would be a guess."""
    outcome = _t150(_answering(_row(loft_deg=sent)))

    assert outcome.specs[0].loft_deg == expected


def test_the_two_vocabularies_go_through_the_contracts_parsers() -> None:
    """"S" is a flex here for the same reason it is one in the save route — one parse site."""
    outcome = _t150(_answering(_row(shaft_flex="S", shaft_material="Carbon Fiber")))

    (spec,) = outcome.specs
    assert spec.shaft_flex is ShaftFlex.STIFF
    assert spec.shaft_material is ShaftMaterial.GRAPHITE


def test_an_unrecognised_flex_is_blank_rather_than_nudged() -> None:
    outcome = _t150(_answering(_row(shaft_flex="regular-ish", shaft_material="titanium")))

    (spec,) = outcome.specs
    assert spec.shaft_flex is None
    assert spec.shaft_material is None


def test_a_real_spec_survives_intact() -> None:
    outcome = _t150(
        _answering(
            _row(
                loft_deg=30.5,
                lie_deg=62.0,
                offset_mm=0.0,
                shaft_flex="stiff",
                shaft_material="steel",
                shaft_weight_g=120.0,
                length_in=37.0,
                swing_weight="D2",
                adjustable_hosel=False,
                notes="Lofts are the 2023 build.",
            )
        )
    )

    (spec,) = outcome.specs
    assert spec.loft_deg == 30.5
    assert spec.lie_deg == 62.0
    # A blade's genuine zero offset, which must not read as "unknown".
    assert spec.offset_mm == 0.0
    assert spec.adjustable_hosel is False
    assert spec.swing_weight == "D2"
    assert outcome.provenance is not None
    assert "Lofts are the 2023 build." in outcome.provenance.notes


def test_a_malformed_loft_range_costs_the_numbers_and_keeps_the_club() -> None:
    """`ClubSpec` refuses a reversed range, and the refusal must not lose the whole lookup."""
    outcome = _t150(_answering(_row(loft_deg=9.0, loft_range_deg=[10.75, 7.25])))

    (spec,) = outcome.specs
    assert spec.club is ClubId.SEVEN_IRON
    assert spec.make == "Titleist"
    assert spec.loft_deg is None
    assert outcome.provenance is not None
    assert "did not hold together" in outcome.provenance.notes


@pytest.mark.parametrize("sent", [[9.0], [7.25, 9.0, 10.75], "7.25-10.75", [7.25, "high"]])
def test_a_range_that_is_not_two_numbers_is_refused(sent: object) -> None:
    outcome = _t150(_answering(_row(loft_range_deg=sent)))

    assert outcome.specs[0].loft_range_deg is None


# --------------------------------------------------------------------------- the identity


def test_the_identity_comes_from_the_request_not_the_answer() -> None:
    """The catalogue key is built from these three, so a helpful correction is a permanent miss."""
    corrected = {"make": "Titleist Golf", "model": "T-150", "model_year": 2024}
    outcome = _t150(_answering(_row(club="7i") | corrected))

    (spec,) = outcome.specs
    assert spec.make == "Titleist"
    assert spec.model == "T150"
    assert spec.model_year == 2023


def test_a_club_nobody_asked_about_is_dropped() -> None:
    outcome = _t150(_answering(_row(club="8i"), _row(club="7i", loft_deg=30.5)))

    (spec,) = outcome.specs
    assert spec.club is ClubId.SEVEN_IRON
    assert spec.loft_deg == 30.5


def test_an_unparseable_club_string_is_dropped_rather_than_nudged() -> None:
    """`parse_club` is the only place text becomes a `ClubId`, here as everywhere else."""
    outcome = _t150(_answering(_row(club="seven iron-ish", loft_deg=30.5)))

    (spec,) = outcome.specs
    assert spec.loft_deg is None
    assert outcome.provenance is not None
    assert "nothing was returned" in outcome.provenance.notes


def test_two_answers_for_one_slot_keep_the_first_and_say_so() -> None:
    """The model has contradicted itself; taking the last would be arbitrary but look decisive."""
    outcome = _t150(_answering(_row(loft_deg=30.5), _row(loft_deg=34.0)))

    assert outcome.specs[0].loft_deg == 30.5
    assert outcome.provenance is not None
    assert "two answers came back" in outcome.provenance.notes


# --------------------------------------------------------------------------- the set


def test_a_set_is_one_request_and_one_spec_per_slot() -> None:
    """One call, because a set's lofts are a progression and N calls are N chances to break it."""
    slots = [ClubId.FIVE_IRON, ClubId.SIX_IRON, ClubId.SEVEN_IRON]
    client = _answering(
        _row(club="5i", loft_deg=24.0),
        _row(club="6i", loft_deg=27.0),
        _row(club="7i", loft_deg=30.5),
    )

    outcome = look_up_set("Titleist", "T150", 2023, slots, llm_model=LLM_MODEL, client=client)

    assert len(client.messages.calls) == 1
    assert [spec.club for spec in outcome.specs] == slots
    assert [spec.loft_deg for spec in outcome.specs] == [24.0, 27.0, 30.5]


def test_a_slot_the_model_skipped_still_comes_back_as_a_row_of_blanks() -> None:
    """The honest shape: a slot nobody answered is a form to type into, not a club that vanished."""
    slots = [ClubId.SEVEN_IRON, ClubId.PITCHING_WEDGE]
    client = _answering(_row(club="7i", loft_deg=30.5))

    outcome = look_up_set("Titleist", "T150", 2023, slots, llm_model=LLM_MODEL, client=client)

    assert [spec.club for spec in outcome.specs] == slots
    assert outcome.specs[1].loft_deg is None
    assert outcome.specs[1].make == "Titleist"
    assert outcome.provenance is not None
    assert "pw: nothing was returned for this slot." in outcome.provenance.notes


def test_duplicate_slots_collapse_and_order_is_the_callers() -> None:
    client = _answering(_row(club="8i"), _row(club="7i"))

    outcome = look_up_set(
        "Titleist", "T150", 2023,
        [ClubId.EIGHT_IRON, ClubId.SEVEN_IRON, ClubId.EIGHT_IRON],
        llm_model=LLM_MODEL, client=client,
    )

    assert [spec.club for spec in outcome.specs] == [ClubId.EIGHT_IRON, ClubId.SEVEN_IRON]


def test_an_empty_set_costs_nothing_and_does_not_call() -> None:
    client = _answering()

    outcome = look_up_set("Titleist", "T150", 2023, [], llm_model=LLM_MODEL, client=client)

    assert client.messages.calls == []
    assert outcome.specs == ()
    assert outcome.note is not None


def test_per_slot_notes_are_kept_and_named_by_slot() -> None:
    client = _answering(
        _row(club="7i", notes="Loft is the 2023 build."),
        _row(club="8i", notes="Not confident about the lie."),
    )

    outcome = look_up_set(
        "Titleist", "T150", 2023, [ClubId.SEVEN_IRON, ClubId.EIGHT_IRON],
        llm_model=LLM_MODEL, client=client,
    )

    assert outcome.provenance is not None
    assert "7i: Loft is the 2023 build." in outcome.provenance.notes
    assert "8i: Not confident about the lie." in outcome.provenance.notes


# --------------------------------------------------------------------------- the failures


def test_provenance_names_the_model_that_answered() -> None:
    outcome = _t150(_answering(_row()))

    assert outcome.provenance is not None
    assert outcome.provenance.source == f"llm:{LLM_MODEL}"
    assert outcome.provenance.retrieved_at is not None


def test_missing_api_key_is_a_note_not_an_exception() -> None:
    """Needs the SDK present, because the extra is checked first and would answer instead."""
    pytest.importorskip("anthropic", reason="the missing-key note is only reached with the SDK")

    outcome = look_up_club(
        "Titleist", "T150", 2023, ClubId.SEVEN_IRON, llm_model=LLM_MODEL, api_key=None
    )

    assert outcome.specs == ()
    assert outcome.note is not None
    assert "no Anthropic API key" in outcome.note


def test_a_missing_extra_is_not_reported_as_a_missing_key(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    """With both absent only one can be named, and naming the key sends you to the wrong file."""
    monkeypatch.setattr("golf_coach.clubs.lookup._sdk", lambda: None)

    outcome = look_up_club(
        "Titleist", "T150", 2023, ClubId.SEVEN_IRON, llm_model=LLM_MODEL, api_key=None
    )

    assert outcome.note is not None
    assert "llm" in outcome.note
    assert "API key" not in outcome.note


def test_a_refusal_produces_no_specs() -> None:
    outcome = _t150(_answering(_row(), stop_reason="refusal"))

    assert outcome.specs == ()
    assert outcome.note is not None
    assert "declined" in outcome.note


def test_truncation_is_a_refusal_here_rather_than_an_admission() -> None:
    """A cut-off paragraph is still a paragraph; a cut-off JSON object is nothing at all."""
    cut_off = _Response('{"clubs": [{"club": "7i", "lo', stop_reason="max_tokens")
    outcome = _t150(_FakeClient(cut_off))

    assert outcome.specs == ()
    assert outcome.note is not None
    assert "cut off" in outcome.note


def test_unreadable_text_degrades_to_a_note() -> None:
    outcome = _t150(_FakeClient(_Response("I could not find that club.")))

    assert outcome.specs == ()
    assert outcome.note is not None
    assert "not readable" in outcome.note


@pytest.mark.parametrize("payload", ['{"clubs": {}}', '{"clubs": null}', "[]", '{"specs": []}'])
def test_an_answer_that_names_no_clubs_degrades_to_a_note(payload: str) -> None:
    outcome = _t150(_FakeClient(_Response(payload)))

    assert outcome.specs == ()
    assert outcome.note is not None
    assert "named no clubs" in outcome.note


def test_a_thinking_block_is_not_parsed_as_the_answer() -> None:
    client = _FakeClient(
        _Response(json.dumps({"clubs": [_row(loft_deg=30.5)]}), thinking="Let me recall the T150.")
    )

    assert _t150(client).specs[0].loft_deg == 30.5


def test_an_api_error_degrades_to_a_note() -> None:
    outcome = _t150(_FakeClient(RuntimeError("boom")))

    assert outcome.specs == ()
    assert outcome.note is not None
    assert "RuntimeError" in outcome.note


@pytest.mark.parametrize(
    ("error_name", "expected"),
    [
        ("NotFoundError", "rejected the configured model id"),
        ("AuthenticationError", "key was rejected"),
        ("RateLimitError", "rate limited"),
        ("OverloadedError", "overloaded"),
        ("APIConnectionError", "could not reach the API"),
    ],
)
def test_sdk_errors_are_named_specifically_when_the_sdk_is_installed(
    error_name: str, expected: str
) -> None:
    """A stale model id and a rate limit want different words; "an error occurred" hides both."""
    anthropic = pytest.importorskip("anthropic", reason="needs the `llm` extra to be meaningful")

    exc = getattr(anthropic, error_name).__new__(getattr(anthropic, error_name))
    outcome = _t150(_FakeClient(exc))

    assert outcome.note is not None
    assert expected in outcome.note


def test_a_status_error_keeps_the_apis_own_explanation() -> None:
    """A malformed request and an exhausted balance are both 400; only the body says which."""
    anthropic = pytest.importorskip("anthropic", reason="needs the `llm` extra to be meaningful")

    exc = anthropic.APIStatusError.__new__(anthropic.APIStatusError)
    exc.status_code = 400
    exc.body = {"error": {"message": "Your credit balance is too low to access the API."}}

    outcome = _t150(_FakeClient(exc))

    assert outcome.note is not None
    assert "credit balance is too low" in outcome.note


def test_outcome_defaults_are_the_absent_case() -> None:
    outcome = ClubLookupOutcome()

    assert outcome.specs == ()
    assert outcome.provenance is None
    assert outcome.note is None
