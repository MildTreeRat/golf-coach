"""What can be held about `flight.html` without a browser. [M15 P15]

The static pages have no build step and no test runner — `docs/REFACTOR_LEDGER.md`'s 2026-08-13
row declined a toolchain for `api/static/` — so this file does not try to verify that the canvas
draws. `tests/api/test_career_page.py` set the shape: pin the failures that are *silent*, and
leave layout to a human with a phone.

Three of those failures are live here.

**A second copy of a registry.** `FLIGHT_MEASUREMENTS` is the one list of what a flight produces
(ADR-027 §Decision 6) and `FLIGHT_SOURCE` the one name they are filed under. A page that typed
either would go stale the next time the list grew, silently — which is exactly how five
population placements shipped with prose naming none of them (`tests/api/test_results.py`).

**A vocabulary written down twice.** The refusal reasons belong to `contracts/unscored.py` and
the solve cases to `analysis/spin_solve.py`. A page holding its own spellings can print a
sentence about a reason the code no longer returns.

**"Film it again" said to a golfer whose footage was fine.** Every reason a flight can return has
`refilming_helps` false. The MCP server's reading rules name this one explicitly and say the flag
decides it, never the reason's name — so the page has to branch on the flag.

**M15 P16 added a fourth, and it is the sharpest of them: a comparison that is not one.** Setting a
simulated number beside a printed one is what this page now does, and on this corpus *neither* of
the two pairs is a check — one is the spin solve's own input read back, and the other is where the
ball started against where it finished. The pairing, the flag and the sentence are all
`analysis/flight_measure.compare_to_printed`'s, so what is pinned here is that the page renders
them rather than deriving anything, and never prints the two numbers without the sentence.
"""

from __future__ import annotations

import re
from pathlib import Path

import pytest
from fastapi.testclient import TestClient

from golf_coach.analysis.flight_measure import (
    _COMPARISON_PAIRS,
    FLIGHT_MEASUREMENTS,
    FLIGHT_SOURCE,
)
from golf_coach.analysis.spin_solve import SpinSolveCase
from golf_coach.api.app import _STATIC_DIR, create_app
from golf_coach.contracts.unscored import INFERENCE_REASONS
from golf_coach.storage.bundle_store import SwingBundleStore

_PAGE = _STATIC_DIR / "flight.html"
_RESULTS = _STATIC_DIR / "results.html"


@pytest.fixture()
def client(tmp_path: Path) -> TestClient:
    # worker=None: nothing here uploads, and a running consumer would only add threads.
    return TestClient(
        create_app(store=SwingBundleStore(tmp_path), token=None, worker=None)
    )


def _without_comments(source: str) -> str:
    """`tests/api/test_results.py`'s stripper, for the same reason it has one.

    The page's comments quote the vocabularies these tests forbid it to *use* — naming the thing
    you are explaining is how a comment explains it — and a pin that could not tell an example
    from a hard-code would push those explanations out of the file. Line comments match at the
    start of a line only, so a `//` inside a URL or a template literal survives.
    """
    without_blocks = re.sub(r"/\*.*?\*/", "", source, flags=re.DOTALL)
    return "\n".join(
        line for line in without_blocks.splitlines() if not line.lstrip().startswith("//")
    )


def test_the_flight_page_is_served(client: TestClient) -> None:
    """The static mount is open by design (ADR-016): a page has to load before it can ask."""
    res = client.get("/flight.html")

    assert res.status_code == 200
    assert "Ball flight" in res.text


def test_the_page_holds_no_second_copy_of_the_measurement_registry() -> None:
    """The six names are derived from the payload, never typed.

    `label()` strips the `flight_` prefix and the unit suffix off whatever the route sends, so a
    seventh measurement renders the day it ships rather than the day someone remembers this file.
    """
    page = _without_comments(_PAGE.read_text(encoding="utf-8"))

    for name in FLIGHT_MEASUREMENTS:
        assert name not in page, (
            f"flight.html names {name}; it must render whatever `measurements` carries instead"
        )
    assert FLIGHT_SOURCE not in page, (
        "flight.html types the model's source string; the route sends it as `source` so the page "
        "and the stored artifact cannot come to say different words"
    )


def test_the_page_spells_out_no_refusal_reason_of_its_own() -> None:
    """`reason` is respaced for display and never rewritten, so nothing here can go stale."""
    page = _without_comments(_PAGE.read_text(encoding="utf-8"))

    for reason in INFERENCE_REASONS:
        assert reason.value not in page, (
            f"flight.html names the {reason.value} reason; the route sends `reason` and the "
            "sentence that goes with it in `detail`"
        )
    # The same rule one layer in. Which of the seven shapes the printed carry landed on is most
    # of the answer when there is no number (`FlownShot.resolved` survives a refusal for exactly
    # that), so the refusal banner prints the case — and prints whichever one arrives.
    for case in SpinSolveCase:
        assert case.value not in page, (
            f"flight.html names the {case.value} solve case; it must render `spin.case`"
        )


def test_the_page_decides_the_refilming_sentence_on_the_flag() -> None:
    """Never on the reason's name. Every flight reason has the flag false, so nothing here says
    it today — but the branch is what keeps that true when a reason that *is* a capture problem
    reaches this payload."""
    page = _without_comments(_PAGE.read_text(encoding="utf-8"))

    assert "refilming_helps" in page
    assert "re-film" not in page and "film it again" not in page


def test_the_page_draws_the_clamped_part_differently() -> None:
    """ADR-027's 2026-09-05b addendum, at the surface.

    Above the published table's last row the coefficients are held, so spin has no route into the
    carry at all. The payload carries `clamped` per point precisely so the drawing can say which
    part of the line that was; one confident stroke for both would present the extrapolation as
    the measured half.
    """
    page = _PAGE.read_text(encoding="utf-8")

    assert "clamped" in page
    assert "setLineDash" in page
    assert "fully_clamped" in page


def test_the_plan_view_prints_the_stretch_it_drew_with() -> None:
    """The offline axis is exaggerated and the factor is the caption's whole job.

    A reader who misses it is reading a wild hook off a two-yard drift. The factor is chosen from
    a list of round numbers for the same reason: "×7.3" is not something anyone can hold a flight
    in their head against.
    """
    page = _PAGE.read_text(encoding="utf-8")

    assert "planScale" in page
    assert "offline stretched" in page
    assert "EXAGGERATIONS" in page


def test_the_page_says_something_when_the_route_is_not_there() -> None:
    """Three causes of a 404 and only one is a broken install, so the body picks the sentence.

    `StaticFiles` reads from disk per request, so a running server serves this page the moment it
    is written and can still 404 the route it calls — library.html shipped stuck on "Loading..."
    against exactly that. The route's own two 404s carry a `detail` written for a human.
    """
    page = _PAGE.read_text(encoding="utf-8")

    assert "res.status === 404" in page
    assert "run_server.py" in page          # names the repair, not just the failure
    assert "res.status === 401" in page
    assert "!res.ok" in page                # and every other non-OK status reports too


def test_the_flight_page_and_the_swing_page_link_to_each_other() -> None:
    """A page reachable only by typing its URL is the state the library page was built to end.

    The link is offered on every swing whose screen has been read, including the ten of thirteen
    that cannot be flown: the refusal is a finding about the shot — a carry above the model's own
    peak, a club whose loft nobody has declared — and hiding the link on those would hide the
    shots with something to say.
    """
    page = _PAGE.read_text(encoding="utf-8")
    results = _RESULTS.read_text(encoding="utf-8")

    assert "/flight.html?session=" in results
    assert "/results.html?session=" in page
    assert "/api/sessions/" in page and "/flight`" in page


# --------------------------------------------------------------- what P16 added [M15 P16]


def test_the_page_pairs_no_printed_number_with_a_simulated_one_of_its_own() -> None:
    """Which two names are one quantity is a registry question, answered in `analysis/`.

    A page matching `flight_carry_yds` to `carry_distance_yds` in JavaScript would be the third
    copy of two registries — and the pairing it would be copying is the one that decides whether a
    difference is an error at all. `quantity` is what places the printed mark on the drawing, so
    the page never has to know what either number is called.
    """
    page = _without_comments(_PAGE.read_text(encoding="utf-8"))

    # The paired half of `SHOT_MEASUREMENTS` and not all of it: `ball_speed_mph` and
    # `launch_angle_deg` are also the payload's own launch-condition fields, and the page reads
    # those by name because they are what the ball left on rather than something to compare.
    for _simulated, measured, _quantity in _COMPARISON_PAIRS:
        assert measured not in page, (
            f"flight.html names the printed measurement {measured}; the route pairs it in "
            "`compare_to_printed` and sends the row"
        )
    assert "quantity" in page
    assert "down_range" in page and "offline" in page


def test_the_page_never_prints_the_two_numbers_without_the_reading() -> None:
    """The row without its sentence is a validation the corpus cannot support.

    `+0.001 yd` between a simulated carry and a printed one is the most convincing thing on this
    page, and it is the solve reproducing its own input. `comparable` is the flag that says which
    kind of row this is, and `reading` is the sentence — the page decides on the flag and prints
    the sentence, and writes neither.
    """
    page = _without_comments(_PAGE.read_text(encoding="utf-8"))

    assert "row.reading" in page
    assert "row.comparable" in page
    assert "by construction" not in page, (
        "flight.html writes its own version of `circular_carry_note`; the route sends it"
    )


def test_the_page_renders_the_caveats_it_is_sent_and_writes_none() -> None:
    """`caveats_for` composes them and their order is part of that rule, not a layout choice."""
    page = _without_comments(_PAGE.read_text(encoding="utf-8"))

    assert "payload.caveats" in page
    for owned in ("spin ratio of", "ranking those two shots backwards", "vertical plane"):
        assert owned not in page, (
            f"flight.html holds its own copy of a caveat ({owned!r}); `caveats_for` owns them and "
            "the CLI prints the same strings"
        )


def test_a_planar_flight_is_drawn_without_a_landing_point() -> None:
    """The end of a straight line is not where the ball finished, and the page must not say it is.

    With the axis unresolved the flight is flown in the vertical plane and
    `flight_landing_offline_yds` is withheld from the artifact for that reason. A filled landing
    mark on the plan view would be the page asserting the number the pipeline refused to record.
    """
    page = _PAGE.read_text(encoding="utf-8")

    assert "curve_is_drawn" in page
    assert "planarNote" in page
    assert "axis.detail" in page, "the refusal's own sentence, not a second one written here"


def test_the_tracer_replays_and_can_be_asked_not_to() -> None:
    """The perspective panel is the one moving thing on any page in this repo.

    Two failures it has to not have. A listener added on every `draw()` would replay the shot once
    per resize the page has ever had, so the button is bound by assignment; and an animation left
    running while a second one starts is a flicker nobody can debug off a screenshot, so a redraw
    cancels first. `prefers-reduced-motion` gets the finished line rather than a faster one --
    a shorter animation is still an animation.
    """
    page = _PAGE.read_text(encoding="utf-8")

    assert "requestAnimationFrame" in page
    assert "cancelAnimationFrame" in page, "a redraw must cancel the frame already queued"
    assert "prefers-reduced-motion" in page
    assert 'id="replay"' in page
    assert "replay.onclick" in page, (
        "addEventListener would stack one replay per redraw; the binding has to be an assignment"
    )


def test_the_perspective_panel_says_nothing_may_be_measured_off_it() -> None:
    """It is the only drawing here with a camera in it, and a camera foreshortens.

    The two panels under it carry a scale caption at full opacity for the opposite reason -- they
    *can* be read, once the reader knows what they were stretched by. This one cannot be read at
    all, and the caption is where that is said. `.scale` is the class both use, so the warning
    sits in the same place and at the same weight in all three.
    """
    page = _PAGE.read_text(encoding="utf-8")

    assert 'id="tracer"' in page
    assert "nothing measurable" in page


def test_the_tracer_draws_no_landing_the_plan_view_would_refuse() -> None:
    """The same rule as `test_a_planar_flight_is_drawn_without_a_landing_point`, one panel up.

    A perspective view is the easiest place to assert a landing by accident, because a ring on the
    turf reads as a place rather than as a number. `tracerLanding` is the only thing that draws one
    and the only thing that names an offline in words, and both are behind `curve_is_drawn`.
    """
    page = _PAGE.read_text(encoding="utf-8")

    body = page.split("function tracerLanding(")[1].split("\nfunction ")[0]
    assert "curve_is_drawn" in body, "the ring and the label must both sit behind the flag"
    guard = body.index("curve_is_drawn")
    assert body.index("ring(") > guard, "the landing ring is drawn before the flag is checked"


def test_the_sign_disagreement_is_shown_outside_the_launch_block() -> None:
    """⚠️ The one shot on disk carrying the flag is a refusal, so it has no launch conditions.

    Rendered from inside `launchBlock` — where a flag about the spin axis naturally belongs — it
    would never have appeared on the only swing that has ever set it. ADR-027 §Decision 5 says
    flagged and never overwritten, and a flag nothing renders is neither.
    """
    page = _without_comments(_PAGE.read_text(encoding="utf-8"))

    assert "sign_disagrees" in page
    render = page.split("function render()")[1]
    assert "signNote()" in render.split("}")[0], (
        "signNote is called from launchBlock or not at all; it has to be rendered beside the "
        "refusal too"
    )
