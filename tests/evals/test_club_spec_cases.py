"""The club-spec answer key's own shape, checked offline. [M43 P9]

`evals/club_spec/cases.json` is what P12's grader scores the set lookup against, and the user
verifies it entry by entry in P11 (ADR-036 §7). A value with no URL behind it, a slot that is not
a `ClubId`, or an offset converted wrongly from the inches a maker printed would be graded against
without complaint. So each of those is a failure here rather than a wrong score later. So is a set
with no `verified` date: P11's yes is what makes the key an answer key (Decision 2), and P12's
grader would score against an unreviewed set exactly as readily as a reviewed one. [M43 P11]

`problems` is the whole check. It returns every fault it finds, so one run names them all, and the
tests below it break a good key on purpose to prove each rule catches what it says it does.

`cases.md` is what the user reads in that review, so it is **rendered from `cases.json`** and
compared. Reviewing a table that has drifted from the key would verify the wrong numbers. After an
edit to `cases.json`, rewrite it with:

    .venv/Scripts/python.exe tests/evals/test_club_spec_cases.py
"""

from __future__ import annotations

import copy
import json
import re
import sys
from datetime import date
from typing import Any

import pytest

from golf_coach.config import REPO_ROOT
from golf_coach.contracts.club import ClubId
from golf_coach.contracts.club_spec import ClubSpec

KEY_DIR = REPO_ROOT / "evals" / "club_spec"
CASES_JSON = KEY_DIR / "cases.json"
CASES_MD = KEY_DIR / "cases.md"

#: The `ClubSpec` fields the key holds a published value (or a published blank) for, in the order
#: `cases.md` shows them. A field outside this list is not graded, because nothing here says what
#: its right answer is.
FIELDS = ("loft_deg", "lie_deg", "length_in", "offset_mm", "bounce_deg")

#: The unit each field's value is stored in, and the units a source may print it in.
FIELD_UNIT = {
    "loft_deg": "deg",
    "lie_deg": "deg",
    "length_in": "in",
    "offset_mm": "mm",
    "bounce_deg": "deg",
}
PRINTED_UNITS = {
    "loft_deg": {"deg"},
    "lie_deg": {"deg"},
    "length_in": {"in"},
    "offset_mm": {"mm", "in"},
    "bounce_deg": {"deg"},
}
MM_PER_IN = 25.4

#: A loose sanity range per field, in the stored unit. A value outside it is a transcription slip,
#: such as a lie typed into the loft column, rather than a club.
PLAUSIBLE = {
    "loft_deg": (5.0, 70.0),
    "lie_deg": (50.0, 72.0),
    "length_in": (32.0, 48.0),
    "offset_mm": (0.0, 15.0),
    "bounce_deg": (0.0, 20.0),
}

#: `own-bag` may sit beside an age tag. Every case carries exactly one of the three age tags.
AGE_TAGS = ("current", "older", "fabricated")
KNOWN_TAGS = {"own-bag", *AGE_TAGS}

#: Where a value may be read from (ADR-036 §7). A retailer only when the maker's page is gone.
#: `absence-search` is the search that shows a made-up model does not exist (P10).
SOURCE_KINDS = {"maker", "maker-archive", "retailer", "retailer-archive", "absence-search"}
ARCHIVED_KINDS = {"maker-archive", "retailer-archive"}

DOC_KEYS = {"about", "fields", "cases"}
CASE_KEYS = {
    "id",
    "tags",
    "make",
    "model",
    "model_year",
    "model_year_source",
    "slots",
    "slot_labels",
    "sources",
    "notes",
    "verified",
    "answers",
}
SOURCE_KEYS = {"kind", "url", "archive_url", "read_on", "where"}
ENTRY_KEYS = {"value", "published", "unit", "source"}

FIELD_HEADINGS = {
    "loft_deg": "Loft (°)",
    "lie_deg": "Lie (°)",
    "length_in": "Length (in)",
    "offset_mm": "Offset (mm)",
    "bounce_deg": "Bounce (°)",
}

_SAFE_ID = re.compile(r"[a-z0-9][a-z0-9-]*")
_ARCHIVE = re.compile(r"https://web\.archive\.org/web/(\d{14})/(.+)")
_PUBLISHED = re.compile(r'(\d+(?:\.\d+)?|\.\d+)(?:\s+(\d+)/(\d+))?\s*(?:°|"|mm|in)?')


def load() -> dict[str, Any]:
    return json.loads(CASES_JSON.read_text(encoding="utf-8"))


def published_number(text: str) -> float | None:
    """The number a published cell states: `"38 7/8\\""` is 38.875, `".220\\""` (PXG) is 0.22."""
    match = _PUBLISHED.fullmatch(text.strip())
    if match is None:
        return None
    number = float(match.group(1))
    if match.group(2):
        number += int(match.group(2)) / int(match.group(3))
    return number


def stored_value(field: str, number: float, unit: str) -> float:
    """What a published number becomes in the field's own unit, as the key stores it."""
    if FIELD_UNIT[field] == "mm" and unit == "in":
        return round(number * MM_PER_IN, 2)
    return number


# --- the check -------------------------------------------------------------------------------


def problems(doc: dict[str, Any], today: date | None = None) -> list[str]:
    """Every way `doc` falls short of an answer key P12 may grade against. Empty when it is one."""
    today = today or date.today()
    found: list[str] = []
    if set(doc) != DOC_KEYS:
        found.append(f"top-level keys are {sorted(doc)}, expected {sorted(DOC_KEYS)}")
    if tuple(doc.get("fields", ())) != FIELDS:
        found.append(f"fields are {doc.get('fields')}, expected {list(FIELDS)}")
    unknown_fields = set(FIELDS) - set(ClubSpec.model_fields)
    if unknown_fields:
        found.append(f"{sorted(unknown_fields)} are not ClubSpec fields")
    cases = doc.get("cases")
    if not isinstance(cases, list) or not cases:
        return [*found, "there are no cases"]

    seen: set[str] = set()
    for case in cases:
        found += _case_problems(case, today)
        folded = str(case.get("id", "")).lower()
        if folded in seen:
            found.append(f"case id {case.get('id')!r} is used twice (ignoring case)")
        seen.add(folded)
    return found


def _case_problems(case: dict[str, Any], today: date) -> list[str]:
    cid = case.get("id")
    where = f"case {cid!r}"
    found: list[str] = []
    if set(case) != CASE_KEYS:
        # Closed on purpose: the repo is public, and ADR-036 §7 lets the user's bag in as
        # specifications only. A key nobody planned for is where a player id would ride in.
        found.append(f"{where}: keys are {sorted(case)}, expected {sorted(CASE_KEYS)}")
        return found
    if not isinstance(cid, str) or not _SAFE_ID.fullmatch(cid):
        found.append(f"{where}: the id is not lowercase letters, digits and hyphens")
    for name in ("make", "model", "notes"):
        if not isinstance(case[name], str) or not case[name].strip():
            found.append(f"{where}: {name} is empty")

    tags = case["tags"]
    if not isinstance(tags, list) or len(set(tags)) != len(tags) or not set(tags) <= KNOWN_TAGS:
        found.append(f"{where}: tags {tags} are not distinct known tags {sorted(KNOWN_TAGS)}")
        tags = []
    ages = [tag for tag in tags if tag in AGE_TAGS]
    if len(ages) != 1:
        found.append(f"{where}: has age tags {ages}, expected exactly one of {AGE_TAGS}")
    fabricated = "fabricated" in tags
    if fabricated and "own-bag" in tags:
        found.append(f"{where}: a made-up model cannot be in the user's bag")

    sources = case["sources"]
    if not isinstance(sources, dict) or not sources:
        return [*found, f"{where}: has no sources"]
    for sid, source in sources.items():
        found += _source_problems(f"{where} source {sid!r}", source, today)
    used: set[str] = set()
    found += _verified_problems(where, case["verified"], sources, today)

    year, year_source = case["model_year"], case["model_year_source"]
    if year is None:
        if year_source is not None:
            found.append(f"{where}: a model_year_source with no model_year")
    elif not isinstance(year, int) or not 1950 <= year <= today.year + 1:
        found.append(f"{where}: model_year {year!r} is not a plausible year")
    elif year_source not in sources:
        found.append(f"{where}: model_year {year} names no source the case lists")
    if year_source is not None:
        used.add(year_source)

    slots = case["slots"]
    order = list(ClubId)
    try:
        clubs = [ClubId(slot) for slot in slots]
    except (TypeError, ValueError):
        found.append(f"{where}: slots {slots} are not all ClubId values")
        clubs = []
    if not slots or len(set(slots)) != len(slots):
        found.append(f"{where}: slots are empty or repeat")
    elif clubs and [order.index(club) for club in clubs] != sorted(order.index(c) for c in clubs):
        found.append(f"{where}: slots are not in bag order")

    labels = case["slot_labels"]
    if not isinstance(labels, dict) or list(labels) != list(slots):
        found.append(f"{where}: slot_labels do not name exactly the slots, in their order")
    elif not all(isinstance(label, str) and label.strip() for label in labels.values()):
        found.append(f"{where}: a slot label is empty")

    answers = case["answers"]
    if not isinstance(answers, dict) or list(answers) != list(slots):
        return [*found, f"{where}: answers do not cover exactly the slots, in their order"]
    for slot, row in answers.items():
        if not isinstance(row, dict) or tuple(row) != FIELDS:
            found.append(f"{where} {slot}: answers {list(row)} are not exactly {list(FIELDS)}")
            continue
        for field, entry in row.items():
            found += _entry_problems(f"{where} {slot} {field}", field, entry, sources, fabricated)
            if isinstance(entry, dict) and entry.get("source") in sources:
                used.add(entry["source"])

    unused = set(sources) - used
    if unused:
        found.append(f"{where}: sources {sorted(unused)} back no value")
    if fabricated:
        kinds = {source.get("kind") for source in sources.values() if isinstance(source, dict)}
        if kinds != {"absence-search"}:
            found.append(f"{where}: a made-up model is sourced only by absence searches")
    return found


def _verified_problems(
    where: str, verified: Any, sources: dict[str, Any], today: date
) -> list[str]:
    """The date the user said yes to this set in P11, which cannot come before its reading."""
    try:
        on = date.fromisoformat(verified)
    except (TypeError, ValueError):
        return [f"{where}: verified {verified!r} is not the date the user verified the set"]
    if on > today:
        return [f"{where}: verified on {on}, which is in the future"]
    read: list[date] = []
    for source in sources.values():
        try:
            read.append(date.fromisoformat(source["read_on"]))
        except (TypeError, ValueError, KeyError):
            continue  # `_source_problems` names a bad `read_on`; this rule does not repeat it
    if read and on < max(read):
        return [f"{where}: verified on {on}, before its source was read on {max(read)}"]
    return []


def _source_problems(where: str, source: Any, today: date) -> list[str]:
    if not isinstance(source, dict) or set(source) != SOURCE_KEYS:
        return [f"{where}: keys are not {sorted(SOURCE_KEYS)}"]
    found: list[str] = []
    kind, url, archive = source["kind"], source["url"], source["archive_url"]
    if kind not in SOURCE_KINDS:
        found.append(f"{where}: kind {kind!r} is not one of {sorted(SOURCE_KINDS)}")
    if not isinstance(url, str) or not url.startswith("https://") or " " in url:
        found.append(f"{where}: url {url!r} is not an https URL")
    if kind in ARCHIVED_KINDS:
        match = _ARCHIVE.fullmatch(archive) if isinstance(archive, str) else None
        if match is None or match.group(2) != url:
            found.append(f"{where}: an archived source needs the Wayback copy of its own url")
    elif archive is not None:
        found.append(f"{where}: archive_url is set on a {kind!r} source")
    try:
        read_on = date.fromisoformat(source["read_on"])
    except (TypeError, ValueError):
        found.append(f"{where}: read_on {source['read_on']!r} is not a date")
    else:
        if read_on > today:
            found.append(f"{where}: read_on {read_on} is in the future")
    if not isinstance(source["where"], str) or not source["where"].strip():
        found.append(f"{where}: says nothing about where on the page the values are")
    return found


def _entry_problems(
    where: str, field: str, entry: Any, sources: dict[str, Any], fabricated: bool
) -> list[str]:
    if not isinstance(entry, dict) or set(entry) != ENTRY_KEYS:
        return [f"{where}: keys are not {sorted(ENTRY_KEYS)}"]
    found: list[str] = []
    if entry["source"] not in sources:
        found.append(f"{where}: source {entry['source']!r} is not one the case lists")
    value, published, unit = entry["value"], entry["published"], entry["unit"]
    if value is None:
        # Published-blank: the source's table does not print the field. Nothing else is set,
        # so a blank cannot carry a stray number that a reader mistakes for the answer.
        if published is not None or unit is not None:
            found.append(f"{where}: a blank value carries published text or a unit")
        return found
    if fabricated:
        found.append(f"{where}: a made-up model has a published value")
    if isinstance(value, bool) or not isinstance(value, int | float):
        return [*found, f"{where}: value {value!r} is not a number"]
    low, high = PLAUSIBLE[field]
    if not low <= value <= high:
        found.append(f"{where}: {value} is outside {low}..{high}")
    if unit not in PRINTED_UNITS[field]:
        return [*found, f"{where}: unit {unit!r} is not one {field} is printed in"]
    number = published_number(published) if isinstance(published, str) else None
    if number is None:
        return [*found, f"{where}: published text {published!r} states no number"]
    expected = stored_value(field, number, unit)
    if abs(expected - value) > 1e-9:
        found.append(f"{where}: published {published!r} ({unit}) is {expected}, not {value}")
    return found


# --- cases.md --------------------------------------------------------------------------------


def _number(value: float) -> str:
    return f"{value:g}"


def _cell(field: str, entry: dict[str, Any]) -> str:
    """The published text as printed, then `= value` when a reader could not see the value in it.

    That is a figure printed in another unit, or a fraction: `0.17" = 4.32`, `38 7/8" = 38.875`.
    A bare number gets its unit named, since the maker put the unit in the column heading.
    """
    if entry["value"] is None:
        return "blank"
    published = str(entry["published"])
    if entry["unit"] == FIELD_UNIT[field] and "/" not in published:
        return published
    marked = published.rstrip().endswith(('"', "°", "mm", "in"))
    shown = published if marked else f"{published} {entry['unit']}"
    return f"{shown} = {_number(entry['value'])}"


def _source_line(sid: str, source: dict[str, Any]) -> str:
    line = f"- **{sid}** ({source['kind']}): <{source['url']}>"
    if source["archive_url"]:
        line += f", read from the archived copy <{source['archive_url']}>"
    return f"{line}, on {source['read_on']}. On the page: {source['where']}."


def render(doc: dict[str, Any]) -> str:
    """`cases.md` for `doc`: a summary, then one table per set, every value with its source."""
    cases = doc["cases"]
    real = [c for c in cases if "fabricated" not in c["tags"]]
    entries = [e for c in real for row in c["answers"].values() for e in row.values()]
    filled = sum(1 for e in entries if e["value"] is not None)
    made_up = sum(len(c["slots"]) * len(FIELDS) for c in cases if c not in real)
    out = [
        "# Club-spec answer key",
        "",
        "<!-- Rendered from cases.json by tests/evals/test_club_spec_cases.py, which fails when "
        "this file and cases.json differ. Edit cases.json, then run "
        "`.venv/Scripts/python.exe tests/evals/test_club_spec_cases.py`. -->",
        "",
        "What the club-spec eval grades the set lookup against (M43, "
        "[ADR-036 §7](../../docs/decisions/036-llm-evaluation-and-observability.md)). Every value "
        "was read from the page named under its table, on the date given there. Nothing here came "
        "from `club_catalogue.json` or from a model. **blank** means the page's table does not "
        "print that field, so the right answer is a blank. A set tagged `fabricated` is a made-up "
        "model: every field is blank, and its source is the search that found no such model. A "
        "cell such as `0.15 in = 3.81` is a "
        "figure the page printed in another unit, converted to the column's (25.4 mm to the inch), "
        'and `38 7/8" = 38.875` is a fraction written out.',
        "",
        "Slots follow the repo's `ClubId`. A maker's AW or GW is the `gw` slot, and a wedge "
        "labelled only by the slot's initial (PXG's G, S and L) is `gw`, `sw` or `lw`. A wedge the "
        "maker labels only W, UW, W2 or DW is left out, because naming its slot would be a guess.",
        "",
        f"{len(cases)} sets, {sum(len(c['slots']) for c in cases)} slots, {filled} published "
        f"values, {len(entries) - filled} published blanks, and {made_up} blanks in the "
        f"{len(cases) - len(real)} made-up sets.",
        "",
        "| Case | Tags | Make | Model | Year | Slots |",
        "|---|---|---|---|---|---|",
    ]
    for case in cases:
        year = case["model_year"] if case["model_year"] is not None else "not given"
        out.append(
            f"| `{case['id']}` | {', '.join(case['tags'])} | {case['make']} | {case['model']} | "
            f"{year} | {', '.join(case['slots'])} |"
        )
    for case in cases:
        year = case["model_year"] if case["model_year"] is not None else "not given"
        named = case["model_year_source"]
        year_source = f" (from **{named}**)" if named else ""
        out += [
            "",
            f"## `{case['id']}`: {case['make']} {case['model']}",
            "",
            f"Tags: {', '.join(case['tags'])}. Model year: {year}{year_source}. "
            f"Verified by the user: {case['verified']}.",
            "",
            "| Slot | Maker's label | " + " | ".join(FIELD_HEADINGS[f] for f in FIELDS) + " |",
            "|---|---|" + "---|" * len(FIELDS),
        ]
        for slot in case["slots"]:
            row = case["answers"][slot]
            cells = " | ".join(_cell(field, row[field]) for field in FIELDS)
            out.append(f"| {slot} | {case['slot_labels'][slot]} | {cells} |")
        sources = sorted({row[f]["source"] for row in case["answers"].values() for f in FIELDS})
        if len(sources) > 1:
            out += ["", "Each value's source, where a set has more than one:", ""]
            for slot in case["slots"]:
                row = case["answers"][slot]
                out.append(f"- {slot}: " + ", ".join(f"{f} {row[f]['source']}" for f in FIELDS))
        out += ["", "Sources:", ""]
        out += [_source_line(sid, source) for sid, source in case["sources"].items()]
        out += ["", f"Notes: {case['notes']}"]
    return "\n".join(out) + "\n"


# --- the tests -------------------------------------------------------------------------------


def test_the_answer_key_has_no_problems() -> None:
    found = problems(load())
    assert not found, "\n".join(found)


def test_cases_md_is_rendered_from_cases_json() -> None:
    assert CASES_MD.read_text(encoding="utf-8") == render(load()), (
        "cases.md is not what cases.json renders to. Run "
        "`.venv/Scripts/python.exe tests/evals/test_club_spec_cases.py` to rewrite it."
    )


def test_the_key_has_the_users_bag_both_ages_of_real_set_and_made_up_ones() -> None:
    tags = {tag for case in load()["cases"] for tag in case["tags"]}
    assert {"own-bag", "current", "older", "fabricated"} <= tags


def _first(doc: dict[str, Any], tag: str) -> dict[str, Any]:
    return next(case for case in doc["cases"] if tag in case["tags"])


def _filled(case: dict[str, Any]) -> tuple[str, str]:
    """The first slot and field of `case` holding a published value."""
    return next(
        (slot, field)
        for slot, row in case["answers"].items()
        for field, entry in row.items()
        if entry["value"] is not None
    )


def _break_value(doc: dict[str, Any]) -> None:
    case = _first(doc, "current")
    slot, field = _filled(case)
    case["answers"][slot][field]["value"] += 1.0


def _break_conversion(doc: dict[str, Any]) -> None:
    for case in doc["cases"]:
        for row in case["answers"].values():
            entry = row["offset_mm"]
            if entry["unit"] == "in":
                entry["value"] = float(published_number(entry["published"]) or 0)
                return
    raise AssertionError("no offset in the key is printed in inches")


def _drop_url(doc: dict[str, Any]) -> None:
    next(iter(_first(doc, "current")["sources"].values()))["url"] = ""


def _drop_date(doc: dict[str, Any]) -> None:
    next(iter(_first(doc, "current")["sources"].values()))["read_on"] = "last week"


def _dangling_source(doc: dict[str, Any]) -> None:
    case = _first(doc, "current")
    slot, field = _filled(case)
    case["answers"][slot][field]["source"] = "nowhere"


def _archive_without_copy(doc: dict[str, Any]) -> None:
    for case in doc["cases"]:
        for source in case["sources"].values():
            if source["kind"] in ARCHIVED_KINDS:
                source["archive_url"] = None
                return
    raise AssertionError("no source in the key is archived")


def _unknown_tag(doc: dict[str, Any]) -> None:
    _first(doc, "current")["tags"].append("vintage")


def _two_ages(doc: dict[str, Any]) -> None:
    _first(doc, "current")["tags"].append("older")


def _not_a_slot(doc: dict[str, Any]) -> None:
    case = _first(doc, "current")
    case["slots"][0] = "aw"


def _out_of_order(doc: dict[str, Any]) -> None:
    case = _first(doc, "current")
    case["slots"].reverse()


def _missing_field(doc: dict[str, Any]) -> None:
    row = next(iter(_first(doc, "current")["answers"].values()))
    del row["bounce_deg"]


def _blank_with_text(doc: dict[str, Any]) -> None:
    for case in doc["cases"]:
        for row in case["answers"].values():
            for entry in row.values():
                if entry["value"] is None:
                    entry["published"] = "n/a"
                    return
    raise AssertionError("the key has no published blank")


def _personal_key(doc: dict[str, Any]) -> None:
    _first(doc, "own-bag")["player_id"] = "someone"


def _year_without_source(doc: dict[str, Any]) -> None:
    case = next(c for c in doc["cases"] if c["model_year"] is not None)
    case["model_year_source"] = None


def _implausible(doc: dict[str, Any]) -> None:
    case = _first(doc, "current")
    entry = next(iter(case["answers"].values()))["lie_deg"]
    entry["value"], entry["published"], entry["unit"] = 6.2, "6.2°", "deg"


def _unverified(doc: dict[str, Any]) -> None:
    _first(doc, "older")["verified"] = None


def _verified_before_it_was_read(doc: dict[str, Any]) -> None:
    _first(doc, "current")["verified"] = "2026-01-01"


def _fabricated_with_a_value(doc: dict[str, Any]) -> None:
    case = copy.deepcopy(_first(doc, "current"))
    case["id"], case["tags"] = "made-up-model", ["fabricated"]
    case["model_year"] = case["model_year_source"] = None
    case["sources"] = {
        "search": {
            "kind": "absence-search",
            "url": "https://duckduckgo.com/html/?q=made+up+model",
            "archive_url": None,
            "read_on": "2026-11-09",
            "where": "no result names the model",
        }
    }
    for row in case["answers"].values():
        for field in FIELDS:
            row[field] = {"value": None, "published": None, "unit": None, "source": "search"}
    assert not problems({**doc, "cases": [case]}), "a well-formed made-up case must pass"
    row = next(iter(case["answers"].values()))
    row["loft_deg"] = {"value": 30.0, "published": "30°", "unit": "deg", "source": "search"}
    doc["cases"].append(case)


@pytest.mark.parametrize(
    "breakage",
    [
        _break_value,
        _break_conversion,
        _drop_url,
        _drop_date,
        _dangling_source,
        _archive_without_copy,
        _unknown_tag,
        _two_ages,
        _not_a_slot,
        _out_of_order,
        _missing_field,
        _blank_with_text,
        _personal_key,
        _year_without_source,
        _implausible,
        _fabricated_with_a_value,
        _unverified,
        _verified_before_it_was_read,
    ],
    ids=lambda breakage: breakage.__name__.strip("_"),
)
def test_each_rule_catches_the_fault_it_names(breakage: Any) -> None:
    doc = load()
    assert not problems(doc)
    breakage(doc)
    assert problems(doc), f"{breakage.__name__} broke the key and nothing noticed"


def test_a_published_cell_reads_as_the_number_it_states() -> None:
    assert published_number('38 7/8"') == 38.875
    assert published_number("3.5mm") == 3.5
    assert published_number("30.5°") == 30.5
    assert published_number("0.142") == 0.142
    assert published_number('.220"') == 0.22
    assert published_number("about 30") is None
    assert stored_value("offset_mm", 0.15, "in") == 3.81


if __name__ == "__main__":
    CASES_MD.write_text(render(load()), encoding="utf-8", newline="\n")
    sys.stdout.write(f"wrote {CASES_MD.relative_to(REPO_ROOT).as_posix()}\n")
