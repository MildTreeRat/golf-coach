"""The one thing about `career.html` a test can hold: its field list. [M12 P6]

The bag page is hand-written HTML with no build step and no test runner — `docs/REFACTOR_LEDGER.md`
declined a toolchain here — so its *behaviour* is verified by driving it, and this file does not
try. What it pins is the failure the milestone named as most likely to happen anyway
(`docs/M12_CLUB_SPECS.md` §Risks): `ClubSpec`, `BagEntryRequest` and the page's `SPEC_FIELDS` are
three surfaces over one field list, and two of them are hand-written.

The shape of that failure is silent and it has happened here before, one layer in: `mcp/query.py`'s
`_METRIC_FIELDS` went stale against its contract and answered 200 while dropping the field nobody
had added to it. A field added to `ClubSpec` and missing from the page is the same defect at the
surface a golfer types into — the route accepts the save, the value has nowhere to be typed, and
nothing anywhere reports a problem.

So this parses the constant out of the page with a regex, which is coarse and is the point: a
parser that understood JavaScript would be a build step, and the constant is a literal list of
literal three-tuples precisely so that reading it needs neither.
"""

from __future__ import annotations

import re
from pathlib import Path

from golf_coach.contracts.club_spec import ClubSpec

_PAGE = Path(__file__).resolve().parents[2] / "src/golf_coach/api/static/career.html"

#: `["loft_deg", "loft °", "number"],` — a field row. The group rows around them
#: (`["head", [`) do not match, which is what keeps the two levels apart without a real parser.
_FIELD = re.compile(r'\["([a-z_0-9]+)", "[^"]*", "(text|number|int|bool|range)"\]')

#: Not on the form and each for its own reason, all three recorded in the page's own comment:
#: `club` is the route's path segment, and the store owns the clock.
_NOT_ON_THE_FORM = {"club"}


def _spec_fields() -> dict[str, str]:
    """The page's `SPEC_FIELDS`, as field name -> control type."""
    page = _PAGE.read_text(encoding="utf-8")
    start = page.index("const SPEC_FIELDS = [")
    end = page.index("\n];", start)
    return {name: kind for name, kind in _FIELD.findall(page[start:end])}


def test_the_page_edits_every_spec_field_and_the_one_it_must_not() -> None:
    """The regression this file exists for. See the module docstring."""
    assert set(_spec_fields()) == set(ClubSpec.model_fields) - _NOT_ON_THE_FORM


def test_every_field_gets_the_control_its_type_needs() -> None:
    """A control that cannot express the field's type loses values silently.

    Three of these matter and the rest are one rule. A `bool | None` on a checkbox cannot say "not
    recorded" — it would save every unknown hosel as fixed. `model_year` on a `step="0.1"` input
    accepts 2023.5 and earns a 422 from an `int` field. And `loft_range_deg` is a *pair*: one input
    would post a string where the contract wants two floats.
    """
    fields = _spec_fields()
    assert fields["adjustable_hosel"] == "bool"
    assert fields["usga_conforming"] == "bool"
    assert fields["model_year"] == "int"
    assert fields["loft_range_deg"] == "range"

    text = {name for name, kind in fields.items() if kind == "text"}
    # Every `str` field on the contract, and only those — `""` is its undeclared value, where a
    # number's is `None`, and a text input is the only control here that produces one.
    assert text == {
        name for name, field in ClubSpec.model_fields.items()
        if field.annotation is str
    } | {"shaft_material", "shaft_flex"}


def test_the_page_holds_no_second_copy_of_a_closed_vocabulary() -> None:
    """`shaft_flex` and `shaft_material` are typed, not picked, and that is deliberate.

    `GET /api/clubs` exists so `static/index.html` never holds a second copy of the club taxonomy
    (`tests/api/test_uploads.py:537`); the same argument covers the two vocabularies M12 closed.
    A <select> here would be the members written down twice, free to drift from
    `contracts/club_spec.py` — and it buys nothing, because the save route already refuses a
    spelling `parse_shaft_flex` does not know, by name, in a sentence.

    A future phase that does want pickers has an answer that is not this one: serve the members
    from a route, the way the club list already is.
    """
    page = _PAGE.read_text(encoding="utf-8")
    assert "xx_stiff" not in page
    assert "multi_material" not in page
