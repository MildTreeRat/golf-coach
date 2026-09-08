"""Claims about this repo that a machine can check, checked by a machine.

This project already had near-maximal documentation discipline — a four-tier trust system, ADRs
with addenda where reality corrected the original call, an append-only WORKLOG, a routing map —
**and it drifted anyway, on the highest-stakes sentence in the repo.** `contracts/caveats.py` told
every MCP client and every coaching call that the system measured *five* checkpoints while six
shipped, for the whole of M6.5. Nothing caught it because nothing could: it was prose.

So the rule this file encodes is not "write better docs". It is: **a claim with a mechanical source
of truth gets asserted against that source, and a claim without one gets deleted from prose.** Both
halves matter. Asserting a volatile number (a test count, a file's line count) just moves the
staleness into a test failure; those numbers are removed from the docs instead, and the assertions
here guard that they stay out.

What is deliberately *not* asserted: the many true past-tense statements about the panel having sat
at three checkpoints. `WORKLOG.md` is append-only history and the milestone docs record what was
built when — a blanket grep for "three checkpoints" would fail on forty correct sentences. Only
present-tense claims about what ships today are mechanized here.
"""

from __future__ import annotations

import re
import subprocess
from pathlib import Path

import pytest

from golf_coach.contracts.checkpoints import CHECKPOINT_REGISTRY
from golf_coach.contracts.placements import POPULATION_PLACEMENT_REGISTRY
from golf_coach.contracts.swing import ANALYSIS_VERSION

REPO = Path(__file__).resolve().parent.parent
DOCS = REPO / "docs"
DECISIONS = DOCS / "decisions"

#: Tiers named in `docs/README.md`'s legend table. `SUPERSEDED` is spelled differently by
#: convention — archived docs lead with the word itself rather than "Tier:".
TIERS = {"AS-BUILT", "TARGET", "REFERENCE", "FOUNDING", "SUPERSEDED"}

# Both banner spellings in use: `> **Tier: AS-BUILT.**` and `**Tier**: AS-BUILT — ...`, plus the
# archive's `> **SUPERSEDED — historical record.**`, which names the tier without the word "Tier".
_BANNER = re.compile(r"^>?\s*\*\*(?:Tier\*{0,2}:\s*)?([A-Z][A-Z-]+)", re.MULTILINE)
_ADDENDUM = re.compile(r"^#+ *Addendum", re.MULTILINE)


def _read(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def _flat(text: str) -> str:
    """Collapse whitespace, so a phrase that `textwrap.fill` broke across lines still matches.

    The caveat blocks are composed with `fill`, so "hip shift at top" is as likely to arrive with a
    newline in the middle of it as not. Asserting on the wrapped form would make this suite fail
    whenever a label's length changed, which is not the thing being checked.
    """
    return " ".join(text.split())


def _strip_fences(text: str) -> str:
    """Blank out fenced code blocks, keeping line numbering intact.

    `docs/ARCHITECTURE.md`'s "The commands, precisely" block is a shell transcript full of `#`
    comments, which any heading parser reads as forty headings if it does not do this.
    """
    out, fenced = [], False
    for line in text.splitlines():
        if line.lstrip().startswith("```"):
            fenced = not fenced
            out.append("")
            continue
        out.append("" if fenced else line)
    return "\n".join(out)


def _living_docs() -> list[Path]:
    """The `docs/*.md` files the map is expected to route to — the map itself excepted."""
    return sorted(p for p in DOCS.glob("*.md") if p.name != "README.md")


#: A milestone phase list announces itself with `**Status: ... N/M phases ...**`. Matching on the
#: banner rather than on a filename pattern is what keeps the pin from needing an edit per
#: milestone; `M4_FUNDAMENTALS_PANEL.md` has a `**Status:` line and no phase count, and is
#: correctly not a phase doc.
_PHASE_STATUS = re.compile(r"\*\*Status: [^*]*?(\d+)/(\d+) phases")


def _phase_docs() -> list[Path]:
    """The docs that state their own phase count, discovered rather than listed."""
    return [p for p in _living_docs() if _PHASE_STATUS.search(_read(p))]


# --------------------------------------------------------------------- the shipped prose


def test_the_caveats_name_every_checkpoint_that_ships() -> None:
    """The one that would have caught the M6.5 bug.

    `TWO_AXES` goes verbatim into `feedback/coach.py`'s system prompt and `mcp/server.py`'s
    instructions. A checkpoint missing from it is the product telling a model a false fact about
    itself on every swing — and the model has no way to know better.
    """
    from golf_coach.contracts.caveats import TWO_AXES

    two_axes = _flat(TWO_AXES)
    for spec in CHECKPOINT_REGISTRY:
        assert spec.label in two_axes, (
            f"{spec.name} ships but TWO_AXES never names it — the coaching prompt and every MCP "
            "client are being told the panel is smaller than it is"
        )


def test_the_caveats_state_the_real_checkpoint_count() -> None:
    """Both the count and the word for it, since the prose spells the number out."""
    from golf_coach.contracts.caveats import READING_THIS_DATA_HONESTLY, TWO_AXES, _count_word

    word = _count_word(len(CHECKPOINT_REGISTRY))

    assert f"{word} checkpoints" in _flat(TWO_AXES)
    assert f"Only {word} fundamentals are measured" in _flat(READING_THIS_DATA_HONESTLY)


def test_the_sided_split_in_the_caveats_matches_the_registry() -> None:
    """"Less is not better" is the warning most likely to be read backwards, so it is derived.

    A one-sided checkpoint named as two-sided would teach a model that a low number is a failure
    when it is the good side, which is precisely the misreading the bullet exists to prevent.
    """
    from golf_coach.contracts.caveats import READING_THIS_DATA_HONESTLY as honestly

    bullet = next(b for b in _flat(honestly).split("- ") if "Less is not better" in b)
    head, marker, tail = bullet.partition("On the one-sided checkpoints")
    assert marker, "the sided-split bullet no longer names the one-sided checkpoints"

    for spec in CHECKPOINT_REGISTRY:
        if spec.one_sided:
            assert spec.label in tail, f"{spec.name} is one-sided but is not listed as one"
        else:
            assert f"`{spec.name}`" in head, f"{spec.name} is two-sided but is not listed as one"


def test_the_caveats_name_every_reason_that_cannot_be_fixed_by_re_filming() -> None:
    """The half of the unscored prose that reverses if it goes stale.

    This bullet tells every coaching model which causes must never be answered with "shoot the
    clip again". A reason missing from it gets the generic capture advice, which sends a golfer
    back to the bay over something they could fix from the results page — the exact failure the
    old `_UNSCORED_REMEDY` heuristic was written to avoid and could only avoid for one checkpoint.

    Derived from `UNSCORED_REASONS`, so the pin is on the derivation rather than on the wording.
    """
    from golf_coach.contracts.caveats import READING_THIS_DATA_HONESTLY as honestly
    from golf_coach.contracts.unscored import UNSCORED_REASONS

    prose = _flat(honestly)
    for reason, spec in UNSCORED_REASONS.items():
        if not spec.refilming_helps:
            assert f"`{reason.value}`" in prose, (
                f"{reason.value} cannot be fixed by re-filming, but the caveats never say so — "
                "a model reading them will give the capture advice anyway"
            )


def test_the_caveats_do_not_name_a_reason_re_filming_does_fix() -> None:
    """The other direction, and the one that fails quietly.

    A capture problem listed as "not a capture problem" tells a model to withhold the one piece of
    advice that would have worked. Same bullet, opposite error, and nothing else would catch it.
    """
    from golf_coach.contracts.caveats import READING_THIS_DATA_HONESTLY as honestly
    from golf_coach.contracts.unscored import UNSCORED_REASONS

    bullet = next(b for b in _flat(honestly).split("- ") if "not capture problems" in b)
    for reason, spec in UNSCORED_REASONS.items():
        if spec.refilming_helps:
            assert f"`{reason.value}`" not in bullet, (
                f"{reason.value} is a capture problem but is listed among the ones that are not"
            )


def test_the_mishit_bullet_names_every_excluded_metric() -> None:
    """ADR-028 §4: the mishit-exclusion prose is derived from `MISHIT_EXCLUDED_METRICS`.

    Both per-golfer briefings carry the bullet, and it names the metrics a mishit is held out of.
    A bullet naming carry but not total would tell a model the total-distance average still counts
    a topped shot — the drift the derivation exists to prevent, in the channel that ships inside
    `mcp/server.py`'s connect-time instructions.
    """
    from golf_coach.contracts.caveats import READING_A_BAG, READING_A_PERSONAL_HISTORY
    from golf_coach.contracts.mishit import MISHIT_EXCLUDED_METRICS

    for name, block in (
        ("READING_A_BAG", READING_A_BAG),
        ("READING_A_PERSONAL_HISTORY", READING_A_PERSONAL_HISTORY),
    ):
        flat = _flat(block)
        assert "**mishit**" in flat, f"{name} carries no mishit bullet"
        for metric in MISHIT_EXCLUDED_METRICS:
            assert f"`{metric}`" in flat, (
                f"{name}'s mishit bullet does not name `{metric}` — derive the names from "
                "MISHIT_EXCLUDED_METRICS so the prose cannot drift from what artifact_key withholds"
            )


def test_the_caveats_name_every_population_placement_that_ships() -> None:
    """The M6.5 bug's shape, in the channel M8 opened.

    Five placements shipped onto `SwingResult.measurements` and the caveats said nothing about any
    of them, so an MCP client received a distance from a tour population as a bare float under a
    field description promising there was no percentile. A placement missing from this prose is a
    number a model will read as a score, because that is what a large number next to a golf swing
    looks like.
    """
    from golf_coach.contracts.caveats import READING_THIS_DATA_HONESTLY as honestly

    flat = _flat(honestly)
    for spec in POPULATION_PLACEMENT_REGISTRY:
        assert f"`{spec.name}`" in flat, (
            f"{spec.name} ships on every swing but the caveats never name it — every MCP client "
            "and every coaching call is being handed it with no way to read it"
        )


def test_the_uncalibrated_placements_are_named_as_uncalibrated() -> None:
    """Derived, because this is the half of the placement prose that reverses if it goes stale.

    A calibrated placement described as uncalibrated wastes a finding. An **un**calibrated one
    described as calibrated is a mis-detected anchor reported to a golfer as the most unusual thing
    about their swing — which is exactly what `tour_trajectory_q_dtl` is on three of the four
    stored swings.
    """
    from golf_coach.contracts.caveats import READING_THIS_DATA_HONESTLY as honestly

    bullet = next(b for b in _flat(honestly).split("- ") if "not calibrated" in b)
    for spec in POPULATION_PLACEMENT_REGISTRY:
        named = f"`{spec.name}`" in bullet
        assert named is not spec.calibrated, (
            f"{spec.name} is calibrated={spec.calibrated} but the uncalibrated-placement bullet "
            f"{'names' if named else 'does not name'} it"
        )


# --------------------------------------------------------------------- the other prose channel


def _mcp_field_descriptions() -> list[tuple[str, str]]:
    """Every `description=` on a view model the MCP tools return, as (where, text).

    Pydantic ships these inside each tool's `outputSchema`, so they reach a model the same way
    `mcp/server.py`'s instructions do — they are prose about the panel that deriving
    `contracts/caveats.py` does not cover, because nothing was looking at them.

    `mcp/server.py` is left out: it imports the MCP SDK at module scope, and this suite runs on
    the base install. Its instructions are the channel the caveats tests above already guard, and
    since ADR-020 its *tool* descriptions are `_mcp_tool_descriptions()` below.
    """
    import inspect

    from pydantic import BaseModel

    from golf_coach.mcp import career, club, query

    found: list[tuple[str, str]] = []
    for module in (query, career, club):
        for name, obj in vars(module).items():
            if not (inspect.isclass(obj) and issubclass(obj, BaseModel)):
                continue
            # Defined here, not merely imported into scope: `ShotData`'s own descriptions belong
            # to `contracts/` and are checked where they live.
            if obj.__module__ != module.__name__:
                continue
            for field_name, field in obj.model_fields.items():
                if field.description:
                    found.append((f"{name}.{field_name}", field.description))
    return found


def _mcp_tool_descriptions() -> list[tuple[str, str]]:
    """Every tool description, as (where, text). [ADR-020]

    The *third* channel this repo's prose reaches a model through, after the connect-time
    instructions and the output schemas above — and the one a model reads to decide whether to
    call a tool at all. It became scannable when ADR-020 moved these out of `mcp/server.py` (which
    needs the SDK) into `contracts/tool_descriptions.py` (which needs nothing), so the gap named
    in the docstring above is now closed rather than merely explained.
    """
    from golf_coach.contracts.tool_descriptions import TOOL_DESCRIPTIONS

    return [(f"tool {name}", text) for name, text in sorted(TOOL_DESCRIPTIONS.items())]


def test_no_mcp_field_description_miscounts_the_panel() -> None:
    """The M6.5 bug in its second channel: `SwingView.measurements` said "the three entries in
    `checkpoints`" while six shipped, so a model was told half the panel had no reference
    population and could not be judged.

    A scan rather than an assertion on the one field that was wrong. The failure was never that
    *that* sentence was wrong — it was that nothing was reading any of them.
    """
    from golf_coach.contracts.caveats import _COUNT_WORDS, _count_word

    word = _count_word(len(CHECKPOINT_REGISTRY))
    # Only fires on an actual number word, so "fewer fundamentals" and "the three files" — both
    # present and both correct — are not false positives.
    claim = re.compile(
        rf"\b({'|'.join(_COUNT_WORDS)}|\d+) +(?:entries in `checkpoints`|checkpoints|fundamentals)",
        re.IGNORECASE,
    )

    for where, text in _mcp_field_descriptions() + _mcp_tool_descriptions():
        stated = claim.search(_flat(text))
        if stated and stated.group(1).lower() != word:
            pytest.fail(
                f"{where} says '{stated.group(0)}'; {len(CHECKPOINT_REGISTRY)} checkpoints ship. "
                "That description ships to every MCP client inside the tool's outputSchema."
            )


def test_the_measurements_description_derives_its_count() -> None:
    """Correct-but-typed is the state the bug started from, so pin the derivation itself."""
    from golf_coach.contracts.caveats import ONLY_CHECKPOINTS_ARE_JUDGED
    from golf_coach.mcp.query import SwingView

    description = SwingView.model_fields["measurements"].description or ""

    assert ONLY_CHECKPOINTS_ARE_JUDGED in _flat(description), (
        "SwingView.measurements no longer interpolates caveats.ONLY_CHECKPOINTS_ARE_JUDGED — "
        "stating the panel size in its own words is what went stale last time"
    )


def test_the_population_description_derives_its_caveat() -> None:
    """Same pin for the field that carries the placements, and it ships in the same outputSchema."""
    from golf_coach.contracts.caveats import PLACEMENTS_ARE_NOT_SCORES
    from golf_coach.mcp.query import SwingView

    description = SwingView.model_fields["population"].description or ""

    assert PLACEMENTS_ARE_NOT_SCORES in _flat(description), (
        "SwingView.population no longer interpolates caveats.PLACEMENTS_ARE_NOT_SCORES — the "
        "count of placements is stated in its own words again"
    )


# --------------------------------------------------------------------- the architecture doc


def test_architecture_describes_every_checkpoint_that_ships() -> None:
    """`ARCHITECTURE.md` is tier AS-BUILT — "trust its numbers" — so its panel table must be true.

    It carried "Five checkpoints" and a five-row table for a milestone after the sixth shipped.
    """
    text = _read(DOCS / "ARCHITECTURE.md")

    for spec in CHECKPOINT_REGISTRY:
        assert f"`{spec.name}`" in text, (
            f"{spec.name} ships but ARCHITECTURE.md never mentions it, and that document is "
            "tier AS-BUILT"
        )


def test_architecture_states_the_real_checkpoint_count() -> None:
    """The sentence and the diagram node that both said "five" while six shipped.

    Asserted by exact phrasing on purpose: if someone rewords these, this test fails and they have
    to look at it, which is the correct outcome for the two places an AS-BUILT document tells a
    reader how big the panel is.
    """
    from golf_coach.contracts.caveats import _count_word

    text = _read(DOCS / "ARCHITECTURE.md")
    n = len(CHECKPOINT_REGISTRY)

    prose = re.search(r"\b(\w+) checkpoints, all from a single face-on camera", text)
    assert prose, "ARCHITECTURE.md no longer states the panel size in §3 — reword the test with it"
    assert prose.group(1).lower() == _count_word(n), (
        f"ARCHITECTURE.md §3 says '{prose.group(1)} checkpoints'; {n} ship"
    )

    node = re.search(r"CK\[\"(\d+) checkpoints", text)
    assert node, "ARCHITECTURE.md's §1 pipeline diagram no longer labels the checkpoint node"
    assert int(node.group(1)) == n, (
        f"the §1 diagram says {node.group(1)} checkpoints; {n} ship"
    )


def test_architectures_checkpoint_table_has_a_row_per_checkpoint() -> None:
    rows = [
        line
        for line in _read(DOCS / "ARCHITECTURE.md").splitlines()
        if line.startswith("| `") and "|" in line[3:]
    ]
    named = {spec.name for spec in CHECKPOINT_REGISTRY}
    charted = {line.split("`")[1] for line in rows} & named

    assert charted == named, (
        f"ARCHITECTURE.md's checkpoint table is missing {sorted(named - charted)}"
    )


# --------------------------------------------------------------------- the M9 surface

# The three pins below were added by M9 P20, and the reason is worth stating: this suite was
# **fully green** for the whole of M9 while `ARCHITECTURE.md` §4 described a repo that had not
# existed since P4. Fifteen consecutive phases wrote a note in `docs/M9_PLAYER_TRACKING.md` saying
# so, and no test could see any of them. "Run the doc-truth suite first and work from its failures"
# is only a method where the suite covers the surface being changed — so P20 extended the cover
# rather than just fixing the prose.


def test_the_docs_state_the_real_mcp_tool_count() -> None:
    """M9 P18 added two MCP tools and left six sentences saying eight.

    Pinned on one phrase shape — "the same N tools" — rather than on every "N tools" in the repo,
    because a subset count is a legitimate thing to write: `mcp/club.py` really does hold two. What
    is not legitimate is a sentence claiming to describe the *whole* surface with a stale number,
    and that sentence has one recognisable form in all three places it appears.

    `WORKLOG.md` is deliberately not read. It is append-only history, and a dated record of what a
    handshake advertised in August is not a claim about today — the same exemption the module
    docstring gives every true past-tense sentence about the three-checkpoint panel.
    """
    from golf_coach.contracts.caveats import _count_word
    from golf_coach.contracts.tool_descriptions import TOOL_DESCRIPTIONS

    expected = _count_word(len(TOOL_DESCRIPTIONS))
    sources = [
        DOCS / "ARCHITECTURE.md",
        REPO / "ROADMAP.md",
        REPO / "scripts" / "ask_swing.py",
        *sorted(DECISIONS.glob("*.md")),
    ]

    found: list[str] = []
    for path in sources:
        for word in re.findall(r"the same (\w+) tools", _flat(_read(path))):
            found.append(f"{path.relative_to(REPO)}: 'the same {word} tools'")
            assert word == expected, (
                f"{path.relative_to(REPO)} says 'the same {word} tools'; "
                f"{len(TOOL_DESCRIPTIONS)} are registered in TOOL_DESCRIPTIONS"
            )

    assert found, (
        "no document states the MCP tool count in the pinned form any more — either the phrasing "
        "moved, in which case reword this test with it, or the claim was dropped and this pin is "
        "now guarding nothing"
    )


def test_architecture_lists_every_api_route() -> None:
    """There was no route table anywhere in this repo, so M9's seven endpoints landed in silence.

    The decorators are parsed out of `api/app.py`'s **source text** rather than by importing it.
    Importing would pull in `fastapi`, and this suite runs on a base install where the `api` extra
    is absent — the same constraint `tests/api/test_pipeline_imports.py` exists to hold, and the
    same technique the `comparison.py` boundary test uses for its own reason.
    """
    source = _read(REPO / "src" / "golf_coach" / "api" / "app.py")
    routes = set(re.findall(r'@app\.(?:get|post|put|delete)\(\s*"(/api/[^"]+)"', source))

    assert routes, "no routes parsed out of api/app.py — the decorator spelling changed"

    table = _read(DOCS / "ARCHITECTURE.md")
    missing = sorted(route for route in routes if f"`{route}`" not in table)

    assert not missing, (
        "ARCHITECTURE.md §1's route table does not list "
        + ", ".join(missing)
        + " — that document is tier AS-BUILT, so its route table is the answer to "
        "'what does the server serve' and an unlisted endpoint is invisible"
    )


@pytest.mark.parametrize("doc", _phase_docs(), ids=lambda p: p.name)
def test_the_map_and_each_phase_doc_agree_on_the_phase_count(doc: Path) -> None:
    """Both sides drifted independently during M9: the map said "start at P8" at 19/20 built.

    A phase doc's own status line and its row in the map are two hand-maintained counts of one
    thing, which is exactly the shape that goes stale. Pinned to each other rather than to a
    literal, so closing a future milestone the same way needs no edit here.

    **This was M9-only until M10 P9, and the M9-only form was actively harmful.** It gathered every
    `N/M phases built` in the whole map into one set and compared it against M9's status line — so
    the moment a second milestone stated its count honestly, the *correct* edit failed the test.
    M10's row survived at "0/10 built" only by omitting the word "phases", which is a test shaping
    a document rather than checking it. Scoped per doc, and discovered rather than listed, so M11
    needs no edit here either.
    """
    stated = _PHASE_STATUS.search(_read(doc))
    assert stated, f"{doc.name} no longer states 'N/M phases' in its status line"

    rows = [
        line
        for line in _read(DOCS / "README.md").splitlines()
        if line.startswith("|") and f"({doc.name})" in line
    ]
    assert rows, f"{doc.name} states a phase count but has no row in docs/README.md to agree with"

    counts = {m for row in rows for m in re.findall(r"(\d+)/(\d+) phases built", row)}
    assert counts, (
        f"{doc.name} says {stated.group(1)}/{stated.group(2)} phases built; its docs/README.md "
        "row states no count at all, so the map can go stale without anything noticing"
    )
    assert counts == {stated.groups()}, (
        f"{doc.name} says {stated.group(1)}/{stated.group(2)} phases; "
        f"docs/README.md says {sorted(counts)}"
    )


# --------------------------------------------------------------------- the map's own counts


def test_the_documentation_map_counts_the_documents_correctly() -> None:
    """`docs/README.md:8` writes out the command that checks this. It was asking to be a test.

    `.claude/` is excluded on both sides. The agent and slash-command definitions there are
    markdown, but they are harness configuration rather than documentation: the map's job is to
    route a *reader*, and it routes to none of them. Counting them would make the number answer a
    different question than the sentence above it asks.
    """
    listed = subprocess.run(
        ["git", "ls-files", "*.md", ":!.claude"],
        cwd=REPO,
        capture_output=True,
        text=True,
        check=True,
    )
    actual = len([line for line in listed.stdout.splitlines() if line.strip()])

    text = _read(DOCS / "README.md")
    claimed = int(re.search(r"^(\d+) markdown documents", text, re.MULTILINE).group(1))

    assert claimed == actual, (
        f"docs/README.md claims {claimed} markdown documents; git tracks {actual}"
    )


def test_the_map_counts_the_addenda_correctly() -> None:
    """The other command `docs/README.md` writes inline (line 70), as an assertion.

    The addenda are where reality corrected an ADR's original call, so an undercount is how a
    reader misses the correction and acts on the superseded decision.
    """
    actual = sum(len(_ADDENDUM.findall(_read(p))) for p in sorted(DECISIONS.glob("*.md")))

    text = _read(DOCS / "README.md")
    claimed = int(re.search(r"(\d+) addenda between them", text).group(1))

    assert claimed == actual, f"docs/README.md claims {claimed} addenda; there are {actual}"


def test_each_adr_row_states_that_adrs_own_addendum_count() -> None:
    """The per-row half — this is what caught ADR-010 sitting at 4 after gaining a 5th."""
    rows = re.findall(
        r"^\| \[(\d{3})\]\(decisions/([^)]+)\).*\|([^|]*)\|\s*$",
        _read(DOCS / "README.md"),
        re.MULTILINE,
    )
    assert rows, "no ADR rows parsed out of docs/README.md — has the table format changed?"

    for number, filename, addenda_cell in rows:
        actual = len(_ADDENDUM.findall(_read(DECISIONS / filename)))
        stated = re.search(r"\*\*(\d+)\*\*", addenda_cell)
        claimed = int(stated.group(1)) if stated else 0

        assert claimed == actual, (
            f"ADR-{number} row claims {claimed} addenda; {filename} has {actual}"
        )


#: `**eleven addenda**` / `**Three addenda at the foot**` — an ADR counting its own corrections in
#: its Status section. Words rather than digits, which is how both ADRs that do this write it, and
#: `\s+` rather than a space because ADR-027 wraps between the two: the first version of this pin
#: required a literal space, matched nothing there, and passed over the very error it was written
#: for.
_SELF_COUNT = re.compile(r"\*\*(\w+)\s+addenda", re.IGNORECASE)
_IN_WORDS = {
    word: value
    for value, word in enumerate(
        "zero one two three four five six seven eight nine ten eleven twelve thirteen fourteen "
        "fifteen sixteen seventeen eighteen nineteen twenty".split()
    )
}


def test_an_adr_that_counts_its_own_addenda_counts_them_correctly() -> None:
    """The map's per-row count has a pin; an ADR's own header did not, and had gone stale.

    [M15 P15]

    `test_each_adr_row_states_that_adrs_own_addendum_count` reads `docs/README.md`, so an ADR that
    states the number in its *own* Status section states it in the one place nothing checks — and
    ADR-027 was found here saying "eleven addenda" over twelve of them, with the bullet list under
    it stopping a phase short. That header is the first thing a reader of a long ADR sees and it
    is exactly the reader who most needs the corrections, so an undercount there points them at a
    decision reality has already moved.

    Only ADRs that make the claim are checked. Not making it is fine — most do not.
    """
    for path in sorted(DECISIONS.glob("*.md")):
        text = _read(path)
        stated = _SELF_COUNT.search(text)
        if stated is None:
            continue
        word = stated.group(1).lower()
        assert word in _IN_WORDS, (
            f"{path.name} counts its addenda as '{word}', which this pin cannot read — spell it "
            "in words up to twenty, or take the count out of the prose"
        )
        actual = len(_ADDENDUM.findall(text))
        assert _IN_WORDS[word] == actual, (
            f"{path.name} says it has {word} ({_IN_WORDS[word]}) addenda; it has {actual}"
        )


def test_the_version_ledger_documents_the_installed_version() -> None:
    """`ANALYSIS_VERSION` and the comment block above it move together, or neither moves.

    That block is the only place that says what an older artifact is *missing* versus what it
    **disagrees** about, and the difference decides whether a stored swing may be compared against
    a fresh one or has to be re-run first. A bump with no entry is this file's own failure mode —
    prose that has quietly stopped describing the code — and nothing else in the suite would see
    it: every test here reads the constant, not the paragraph explaining it. [M15 P13]
    """
    text = _read(REPO / "src" / "golf_coach" / "contracts" / "swing.py")
    documented = {int(n) for n in re.findall(r"^#:\s*\d+ -> (\d+) \(", text, re.MULTILINE)}
    documented |= {int(n) for n in re.findall(r"^#:\s+(\d+)\s\s+\S", text, re.MULTILINE)}

    assert documented, (
        "no version ledger parsed out of contracts/swing.py — has its format changed?"
    )

    missing = sorted(set(range(1, ANALYSIS_VERSION + 1)) - documented)
    assert not missing, (
        f"ANALYSIS_VERSION is {ANALYSIS_VERSION}; the ledger in contracts/swing.py documents no "
        f"entry for {missing} — say what a stored artifact from the older engine is missing or "
        "disagrees about, in the same edit as the bump"
    )


# --------------------------------------------------------------------- tiers and routing


@pytest.mark.parametrize("doc", _living_docs(), ids=lambda p: p.name)
def test_every_living_doc_declares_a_tier(doc: Path) -> None:
    """The tier is how a reader calibrates trust before reading, so it is not optional.

    `CLAUDE.md`'s reading ladder leans on this: AS-BUILT means trust it, TARGET means verify
    against code, REFERENCE means read the design and not the numbers. A doc with no banner gets
    read at whatever depth the reader guesses.
    """
    head = "\n".join(_read(doc).splitlines()[:8])
    found = {m for m in _BANNER.findall(head) if m in TIERS}

    assert found, f"{doc.name} declares no tier in its first 8 lines (one of {sorted(TIERS)})"


@pytest.mark.parametrize("doc", _living_docs(), ids=lambda p: p.name)
def test_every_living_doc_is_routed_to_from_the_map(doc: Path) -> None:
    """A doc nothing links to is a doc nobody reads — and the map is the only index there is."""
    assert f"({doc.name})" in _read(DOCS / "README.md"), (
        f"{doc.name} is not listed in docs/README.md, so nothing routes a reader to it"
    )


@pytest.mark.parametrize("doc", _living_docs(), ids=lambda p: p.name)
def test_the_map_agrees_with_each_doc_about_its_tier(doc: Path) -> None:
    """Both sides declare the tier, so both sides can drift. Pin them to each other."""
    # Table rows only. Several docs are also named in the "Start here" prose, which mentions the
    # tier in passing and would otherwise be matched first and skipped.
    row = next(
        (
            line
            for line in _read(DOCS / "README.md").splitlines()
            if line.startswith("|") and f"({doc.name})" in line
        ),
        None,
    )
    if row is None or row.count("|") < 3:
        pytest.skip(f"{doc.name} is referenced in prose rather than a tier table row")

    declared = {m for m in _BANNER.findall("\n".join(_read(doc).splitlines()[:8])) if m in TIERS}
    stated = row.split("|")[2].strip()

    assert stated in declared, (
        f"docs/README.md files {doc.name} as {stated}, but the document declares "
        f"{sorted(declared)}"
    )


def test_claude_md_routes_only_to_files_that_exist() -> None:
    """`CLAUDE.md` is loaded into every session, so a path it names is read without checking.

    This is what stops the routing table rotting into pointing at moved or renamed docs — the
    failure mode that would send a cold session hunting through the repo, which is the entire cost
    the file exists to avoid.
    """
    claude = REPO / "CLAUDE.md"
    if not claude.exists():
        pytest.skip("CLAUDE.md does not exist yet")

    text = _strip_fences(_read(claude))
    # Only backticked *paths* — a bare `ranges.json` in prose names a file without claiming to
    # route to it, and resolving those against the repo root would be a false failure.
    #
    # Package-relative shorthand (`analysis/engine.py`) is the form the docs already use, so it
    # resolves against `src/golf_coach/` as well as the repo root. Both are real references; what
    # this test is for is the one that resolves against neither.
    missing = [
        ref
        for ref in re.findall(r"`([\w./-]+\.(?:md|py|json|toml))`", text)
        if "/" in ref
        and not (REPO / ref).exists()
        and not (REPO / "src" / "golf_coach" / ref).exists()
    ]

    assert not missing, f"CLAUDE.md names files that do not exist: {missing}"


# --------------------------------------------------------------------- volatile numbers


def test_volatile_counts_stay_out_of_prose() -> None:
    """Numbers that move every session are deleted rather than asserted.

    A test count or a file's line count cannot be acted on by a reader — "883 lines, grep it"
    and "long, grep it" carry the same instruction — and pinning them here would just convert
    doc drift into a failing test on every unrelated commit. The useful signal survives; the
    digits do not.
    """
    offenders: list[str] = []
    patterns = {
        "ROADMAP.md": r"\b\d{3,4} tests\b",
        "docs/ARCHITECTURE.md": r"\b\d{3,4} passed\b",
        "docs/README.md": r"\b\d{3,4} lines\b",
    }

    for name, pattern in patterns.items():
        for n, line in enumerate(_read(REPO / name).splitlines(), 1):
            if re.search(pattern, line):
                offenders.append(f"{name}:{n}: {line.strip()}")

    assert not offenders, (
        "a volatile count came back into prose — delete the number, keep the point:\n"
        + "\n".join(offenders)
    )
