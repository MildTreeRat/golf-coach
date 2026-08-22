"""The tool descriptions, held to what makes a model route on them. [ADR-020]

These run on a base install — no SDK, no server. The cross-check that both adapters actually
*carry* these strings needs the `llm` extra and lives in `tests/mcp/test_runner_tools.py`; what is
here is everything provable about the prose itself.

Worth stating why the substance assertions are duplicated from `tests/mcp/test_server.py`: that
module could only ever see the descriptions the MCP server declares, and since ADR-020 there is a
second adapter. Asserting the property at the source covers both, and keeps covering a third.
"""

from __future__ import annotations

import re

from golf_coach.contracts.tool_descriptions import (
    CAREER_TOOL_NAMES,
    CLUB_TOOL_NAMES,
    COMPARE_SESSIONS,
    GET_BAG_PROFILE,
    GET_CLUB_PROFILE,
    GET_GOLFER_PROFILE,
    GET_RECENT_SHOTS,
    GET_SESSION_SUMMARY,
    GET_SHOT_BY_ID,
    GET_SHOT_TRENDS,
    GET_SWING,
    LIST_SESSIONS,
    REGISTRY_TOOL_NAMES,
    TOOL_DESCRIPTIONS,
)


def test_the_mapping_covers_every_constant_and_nothing_else() -> None:
    """A constant absent from the mapping is a tool no adapter can find by name."""
    declared = {
        LIST_SESSIONS,
        GET_SWING,
        GET_SESSION_SUMMARY,
        GET_RECENT_SHOTS,
        GET_SHOT_BY_ID,
        GET_GOLFER_PROFILE,
        GET_SHOT_TRENDS,
        COMPARE_SESSIONS,
        GET_BAG_PROFILE,
        GET_CLUB_PROFILE,
    }

    assert set(TOOL_DESCRIPTIONS.values()) == declared
    assert len(TOOL_DESCRIPTIONS) == len(declared), "two tools share one description"


def test_the_career_names_are_real_tools() -> None:
    """`CAREER_TOOL_NAMES` names the three that judge a golfer against their own history.

    A typo here removes nothing and silently offers a tool that answers "unknown golfer" to every
    name — the state `build_server`'s docstring says is worse than not offering it at all.
    """
    assert CAREER_TOOL_NAMES <= set(TOOL_DESCRIPTIONS)
    assert len(CAREER_TOOL_NAMES) == 3


def test_the_registry_gate_is_the_union_of_both_families() -> None:
    """[M9 P18] The gate is "a golfer registry exists", and five tools sit behind it now.

    Kept as three names rather than one because each says something different and true: which
    tools are career mode's, which are the per-club cut of it, and which need the registry. Only
    the union is the gate, and asserting it here is what stops a club tool being added to the
    server while the adapters' "absent without a registry" test still subtracts three names.
    """
    assert CLUB_TOOL_NAMES <= set(TOOL_DESCRIPTIONS)
    assert len(CLUB_TOOL_NAMES) == 2
    assert not (CLUB_TOOL_NAMES & CAREER_TOOL_NAMES), "a tool cannot be in both families"
    assert REGISTRY_TOOL_NAMES == CAREER_TOOL_NAMES | CLUB_TOOL_NAMES


def test_every_description_says_when_to_call_it() -> None:
    """A thin description is the most common cause of a tool never being called.

    The threshold is the one `tests/mcp/test_server.py` has always applied, moved to the source so
    it covers the runner tools too.
    """
    for name, text in TOOL_DESCRIPTIONS.items():
        assert len(text) > 150, f"{name}'s description is too thin to route on"
        assert "Call this" in text, f"{name} never says when to call it"


def test_the_registry_descriptions_carry_their_refusal() -> None:
    """The one warning that cannot be left to the connect-time briefing alone.

    Career mode withholds any figure the sample size cannot support, and the withheld figure sits
    next to the counts that justify the refusal — trivially reconstructible arithmetic, right
    there in the payload. `caveats.READING_A_PERSONAL_HISTORY` says so once at the top of a
    conversation; these say it again at the point of use, which is where it gets ignored.
    """
    for name in sorted(REGISTRY_TOOL_NAMES):
        assert "IMPORTANT" in TOOL_DESCRIPTIONS[name], (
            f"{name} lost the refusal warning — at n=2 every one of its claims is withheld, and "
            "nothing else in the payload stops a model computing the missing figure itself"
        )


def test_no_description_states_a_band_or_a_measurement() -> None:
    """Bands live in `ranges.json` and nowhere else — a number here is a snapshot that will rot.

    Fires on a bare decimal or a degrees/mph/rpm figure. Plain integers are allowed: several
    descriptions legitimately say "two session ids", and `get_shot_trends` names a `days` window.
    """
    number = re.compile(r"\b\d+\.\d+\b|\b\d+\s*(?:°|deg|degrees|mph|rpm|yards)\b", re.IGNORECASE)

    for name, text in TOOL_DESCRIPTIONS.items():
        found = number.search(text)
        assert found is None, (
            f"{name}'s description states '{found.group(0)}' — benchmark bands and measurements "
            "live in ranges.json, and a number quoted here is a snapshot that goes stale silently"
        )


def test_the_club_description_names_what_it_is_easy_to_misread() -> None:
    """[M9 P18] Four sentences that have to be in the routing text, not only in the payload.

    Each one is a misreading this repo can predict and a model cannot catch for itself: an offline
    figure that is a *start line* projected to the carry rather than a landing point, a curve that
    lives in a different field entirely, a shot count that caps every distance, and a loft that does
    not exist for a club nobody has declared. A description is what a model reads before deciding
    what the numbers mean.
    """
    bag, one = GET_BAG_PROFILE, GET_CLUB_PROFILE

    assert "start_line_offline_yds" in one and "STARTED" in one
    assert "not where the ball landed" in one
    assert "face_to_path_deg" in one
    assert "n_shots" in one
    assert "no loft on record" in one

    # The P17 finding, in the tool that renders it: an empty bag is not a sample-size refusal, and
    # answering it with "go and practise" is the specific wrong answer to guard against.
    assert "names a club" in bag
    assert "NOT hitting more balls" in bag
