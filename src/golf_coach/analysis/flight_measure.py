"""The six numbers a simulated flight contributes, and the two ways it declines to. [M15 P11]

`flight_infer.flight_for_shot` resolves a stored shot as far as the screen and the bag allow and
**stops at the launch conditions**; `flight.simulate_flight` flies them. This module is the join,
and it is the layer that decides what of the result becomes a `Measurement` — which is a narrower
question than what a viewer may draw, and the reason the two are not the same function.

It also owns **which printed number each simulated one may be read against** (`compare_to_printed`,
M15 P16), because that is the same registry question as the `flight_` prefix asked from the other
side: two names for one quantity from two provenances. The surfaces render the answer; none of them
derives it.

ADR-027 §Decision 6 names the six: `flight_carry_yds`, `flight_apex_yds`,
`flight_descent_angle_deg`, `flight_time_s`, `flight_landing_offline_yds` and `flight_spin_rpm`,
under the source `model:flight_v1` — the fourth, after `pose:face_on`, `launch_monitor:*` and
`population:golfdb`.

## Distinct names, and the rule underneath them

**`baseline.pooled_samples` groups by name**, so a predicted carry sharing `carry_distance_yds`
would build a personal mean over a mixture of a measurement and a model output. The `flight_`
prefix is what prevents that, and ADR-022's 2026-08-17a addendum set the precedent when the
down-the-line trajectory model got its own names.

The rule that prefix encodes is **one provenance per name**, and this phase found two places where
holding to it costs a number rather than buying one:

- **`flight_landing_offline_yds` is recorded only when the curve was actually drawn.** With the
  spin axis unresolved the flight is flown in the vertical plane and its landing offline comes out
  as `carry * sin(start line)` to the last bit — which is `shot_measure.measure_start_line_offline`,
  recorded since M9, reached by forty flights instead of one sine. M15 P10 measured that identity
  on the corpus (1e-14 structurally, 2.6e-5 yd on the real shot). Recording it anyway would put two
  names on one quantity, which is the pooling hazard §Decision 6 exists to prevent arriving from
  the direction it did not anticipate. `ShotFlight.curve_is_drawn` is the bit that says when.
- **`flight_spin_rpm` is recorded only when the spin was inferred.** ⚠️ This is a correction to
  §Decision 6, which named the measurement without distinguishing the two spins that can fill it.
  A spin the screen printed is a reading off a launch monitor; a solved one is *the spin this
  integrator needs in order to agree with HD Golf's carry* (`spin_solve`'s own words), which is a
  different kind of quantity wearing the same unit — `SpinSource` exists to say so. Pooling the two
  under one name is precisely what the `flight_` prefix was introduced to stop, one layer in: on
  the corpus today it would average a measured 5,991 rpm with a solved 2,924 rpm and print the
  result as one golfer's spin rate. The measured spin is not lost — it is on `ShotData.spin_rate`,
  where the launch monitor put it, and `ShotFlight.spin_rpm` is what a viewer reads to say what the
  line was drawn with. It simply is not a *model* output, so it does not enter under a `model:`
  source.

The other four are model outputs however the spin arrived, so they record on every flight.

## The two refusals, and why they are two entries and not six

A refused flight withholds five names at once, for one cause. It is reported as **one**
`UnscoredCheckpoint` named `flight_carry_yds` — the quantity the model exists to produce — because
`mcp.query.get_session_summary` counts unscored entries by name, and one missing flight counted six
times would report a session as six times more broken than it is. The cause is in `detail` and the
`SpinSolveCase` it landed on travels with it.

The axis is the second entry and genuinely separate: **a refused axis does not stop the flight**
(ADR-027 §Decision 5's third branch), so the other five record and only
`flight_landing_offline_yds` goes missing. That asymmetry is why `ShotFlight` carries `reason` and
`axis.reason` as two fields, and it is the reason there are two entries rather than one.

**Nothing here is a checkpoint** and nothing enters `overall_score` — no band, no `ranges.json`
row, no `CHECKPOINT_REGISTRY` entry (§Decision 6). `UnscoredCheckpoint` is reused because the
absence being *named* is the whole point of that shape, and `contracts.unscored.INFERENCE_REASONS`
is the family M15 P9 added for exactly this. What that reuse costs is two consumer sentences that
assumed every entry was a checkpoint excluded from a score, and `feedback/rules.py` and
`feedback/coach.py` are corrected in this phase rather than left to say something false.

## Why this catches `ValueError`

`simulate_flight` raises on launch conditions there is no flight to integrate from, and names the
caller as the boundary that owns the check. `flight_for_shot` owns two of the five guards — a
missing ball speed or launch angle, and a launch angle at or below the horizontal — and the other
three (a negative spin, a launch direction or a spin axis past a quarter turn) can still arrive
from a parse rather than from a swing. Reaching `analyze_swing` they would take the *whole* swing
down, discarding pose that had already run and every checkpoint that had already scored — the same
failure `api.pipeline._shot_for` was widened to prevent, one layer in. So they are caught here and
returned as a stated refusal. Nothing about a launch monitor's tile justifies losing the swing.

Stdlib + `contracts` only, no numpy (ADR-008, ADR-027 §Decision 1).
"""

from __future__ import annotations

from collections.abc import Callable
from math import cos, radians
from typing import NamedTuple

from golf_coach.analysis.benchmarks.flight_model import FlightModel
from golf_coach.analysis.flight import (
    DEFAULT_STEP_S,
    GATE_AGREEMENT_FRACTION,
    FlightResult,
    simulate_flight,
)
from golf_coach.analysis.flight_infer import ShotFlight, SpinSource, flight_for_shot
from golf_coach.analysis.shot_measure import SHOT_MEASUREMENTS
from golf_coach.contracts.golfer import Handedness
from golf_coach.contracts.shot import ShotData
from golf_coach.contracts.unscored import UnscoredCheckpoint, UnscoredReason

#: `Measurement.source` for everything this module produces. The fourth provenance in the repo, and
#: the first that is not a reading of anything — `pose:`, `launch_monitor:` and `population:` all
#: name an instrument or a corpus, and this one names a *model*, versioned with the artifact it
#: evaluates (`benchmarks/flight_model_v1.json`) so a re-sourced coefficient table gets a new name
#: rather than silently changing what the old one meant.
#:
#: **The prefix has a consequence one layer out**, and M15 P12 is where it was settled:
#: `contracts.career.CorpusSwing.artifact_key` is the single definition of the dedupe rule, and it
#: now maps `model:` onto the **shot photo's** identity — a flight is integrated from the tile's
#: launch conditions and nothing the body did, so two swings sharing one photo are one flight, and
#: a parse flagged under ADR-014 takes the flight down with it. P11 shipped ahead of that on the
#: argument that the `swing:{ref}` fallback can only over-count, which was true of the *dedupe* and
#: wrong about the refusal: an unregistered prefix has no flagged-parse rule in it, so a flight
#: simulated off a suspect tile counted as a sample while the `carry_distance_yds` printed beside
#: it did not.
#:
#: ⚠️ ADR-027 §Decision 6 and this milestone's roadmap both put `artifact_key` in
#: `storage/corpus.py`. It is on `CorpusSwing` in `contracts/career.py` and always has been.
FLIGHT_SOURCE = "model:flight_v1"


class FlownShot(NamedTuple):
    """One stored shot flown, or the reason it was not — with the resolution kept either way.

    `resolved` survives a refusal on purpose. `ShotFlight.spin` carries which of
    `spin_solve.SpinSolveCase`'s seven shapes the carry landed on and the cap the answer would have
    sat under, and that is most of what a reader wants when there is no number: a printed carry two
    yards above the peak and one sitting on the high plateau are entirely different findings about
    the shot, and `reason` collapses both onto `carry_unreachable`.
    """

    #: `None` only when resolving the launch conditions itself raised — a parse this model cannot
    #: fly. Present on every ordinary refusal.
    resolved: ShotFlight | None
    #: The flight, or `None` when `reason` says why there is none.
    flight: FlightResult | None
    #: One of `contracts.unscored.INFERENCE_REASONS`. Never `SPIN_AXIS_UNRESOLVED`, which is a
    #: flight drawn straight rather than a flight refused — see `axis_reason`.
    reason: UnscoredReason | None
    detail: str

    @property
    def flew(self) -> bool:
        return self.flight is not None

    @property
    def curve_is_drawn(self) -> bool:
        """Whether this flight bends, and so whether its landing point is a landing point."""
        return self.resolved is not None and self.resolved.curve_is_drawn


def fly_shot(
    shot: ShotData,
    *,
    loft_deg: float | None,
    handedness: Handedness | None,
    model: FlightModel | None = None,
    step_s: float = DEFAULT_STEP_S,
) -> FlownShot:
    """Resolve one stored shot's launch conditions and fly them, or say why neither happened.

    `loft_deg` and `handedness` are required keywords with no defaults for the reason
    `flight_for_shot` gives: both have a plausible-looking wrong answer — a 7 iron's loft and a
    right-handed golfer — and both come from a different artifact than the shot does.
    `storage/flight_inputs.py` is what resolves them off disk, and `api/pipeline.py` is what hands
    them here.
    """
    try:
        resolved = flight_for_shot(
            shot, loft_deg=loft_deg, handedness=handedness, model=model, step_s=step_s
        )
    except ValueError as exc:
        # The solve flies the ball dozens of times looking for a carry, so an unflyable start line
        # or spin axis raises from in there rather than from the `simulate_flight` below.
        return FlownShot(None, None, UnscoredReason.NO_LAUNCH_CONDITIONS, str(exc))

    if resolved.launch is None:
        assert resolved.reason is not None, "a shot with no launch conditions must say why"
        return FlownShot(resolved, None, resolved.reason, resolved.detail)

    try:
        flight = simulate_flight(resolved.launch, model=model, step_s=step_s)
    except ValueError as exc:
        return FlownShot(resolved, None, UnscoredReason.NO_LAUNCH_CONDITIONS, str(exc))
    return FlownShot(resolved, flight, None, resolved.detail)


def _carry(flown: FlownShot) -> float | None:
    return flown.flight.carry_yds if flown.flight is not None else None


def _apex(flown: FlownShot) -> float | None:
    return flown.flight.apex_yds if flown.flight is not None else None


def _descent_angle(flown: FlownShot) -> float | None:
    return flown.flight.descent_angle_deg if flown.flight is not None else None


def _flight_time(flown: FlownShot) -> float | None:
    return flown.flight.flight_time_s if flown.flight is not None else None


def _landing_offline(flown: FlownShot) -> float | None:
    """Where the ball **finished**, and only when the model actually bent it.

    The module docstring says why a planar flight's offline is withheld rather than recorded: it is
    `start_line_offline_yds` again, to the last bit, under a second name.
    """
    if flown.flight is None or not flown.curve_is_drawn:
        return None
    return flown.flight.landing_offline_yds


def _spin(flown: FlownShot) -> float | None:
    """The solved spin, and never a printed one — the module docstring says why.

    Reads `spin_source` rather than `ShotData.spin_rate` so there is one definition of which spin
    the flight was drawn with, and it is the one `flight_infer` chose.
    """
    if flown.flight is None or flown.resolved is None:
        return None
    if flown.resolved.spin_source is not SpinSource.INFERRED:
        return None
    return flown.resolved.spin_rpm


#: Name -> (function, unit, detail). The same shape as `shot_measure.SHOT_MEASUREMENTS` and
#: `measure.POSE_MEASUREMENTS`, so `analysis.engine` builds all three families the same way — the
#: difference is only what the function reads, because six numbers off one flight must not fly the
#: ball six times.
#:
#: **Every detail string says the number is simulated**, in the string itself rather than by
#: relying on the `flight_` prefix or the source. `analysis.baseline` averages by name, unit and
#: detail without knowing what a source is, and `mcp.query` prints the detail beside the value —
#: so this is the one line about a model output a reader is guaranteed to see.
#:
#: None of them is judged: no band, no `ranges.json` row, no `CHECKPOINT_REGISTRY` entry
#: (ADR-027 §Decision 6), and no `contracts.dispersion.METRIC_TARGETS` row either — see the block
#: comment beside that table for why a model output must not draw a scatter finding.
FLIGHT_MEASUREMENTS: dict[str, tuple[Callable[[FlownShot], float | None], str, str]] = {
    "flight_carry_yds": (
        _carry,
        "yards",
        "SIMULATED carry from the printed launch conditions; not the launch monitor's own carry, "
        "which is carry_distance_yds",
    ),
    "flight_apex_yds": (
        _apex,
        "yards",
        "SIMULATED peak height above the tee; the screen prints no apex tile at all",
    ),
    "flight_descent_angle_deg": (
        _descent_angle,
        "degrees",
        "SIMULATED angle below the horizontal at landing, read off the landing velocity",
    ),
    # Seconds rather than the `ms` the two pose durations use, which is the name ADR-027 gave it and
    # is also what keeps it out of `measure.FPS_DEPENDENT_MEASUREMENTS` — that set is derived from
    # the unit `ms`, and a hang time integrated by a model has no frame rate to be dependent on.
    "flight_time_s": (
        _flight_time,
        "seconds",
        "SIMULATED hang time from launch to landing",
    ),
    "flight_landing_offline_yds": (
        _landing_offline,
        "yards",
        "SIMULATED yards right (+) or left of target the ball FINISHED; recorded only when the "
        "spin axis resolved, so unlike start_line_offline_yds this is a landing point",
    ),
    "flight_spin_rpm": (
        _spin,
        "rpm",
        "SOLVED backspin - the spin this model needs to agree with the printed carry, capped and "
        "recorded only when the screen printed none; a printed spin is on the shot, not here",
    ),
}


def flight_unscored(flown: FlownShot) -> list[UnscoredCheckpoint]:
    """The one or two entries naming what this flight could not produce, and why.

    Empty when the flight flew and its curve was drawn, which on the corpus today is no shot at
    all. See the module docstring for why a refused flight is one entry under `flight_carry_yds`
    and not five.
    """
    if flown.reason is not None:
        return [
            UnscoredCheckpoint(
                name="flight_carry_yds",
                reason=flown.reason,
                # The other four names are withheld by the same cause; saying so here is what stops
                # a reader taking their absence for a second, unexplained failure.
                detail=f"{flown.detail} - no flight, so none of the flight_* measurements recorded",
            )
        ]
    if flown.resolved is not None and flown.resolved.axis.reason is not None:
        return [
            UnscoredCheckpoint(
                name="flight_landing_offline_yds",
                reason=flown.resolved.axis.reason,
                detail=flown.resolved.axis.detail,
            )
        ]
    return []


#: Which simulated name may be set beside which printed one. [M15 P16]
#:
#: Two pairs and no third, because the launch monitor prints two of the six quantities this model
#: produces and neither of the two is a straight check. **Reading a simulated number against a
#: printed one is the whole of what a viewer-honesty phase is for**, and it is a registry question
#: rather than a rendering one: which two names describe the same quantity from two provenances is
#: the same question `FLIGHT_MEASUREMENTS`'s `flight_` prefix answers from the other side. A page
#: pairing them in JavaScript would be holding a third copy of both registries — the copy
#: `tests/api/test_flight_page.py` exists to forbid — and a terminal doing it separately would be a
#: fourth.
#:
#: **The four missing pairs are a fact about the screen, not an omission.** `flight_apex_yds` has
#: `ShotData.apex_height` opposite it and no shot on disk carries one, so the model's apex is
#: unfalsifiable here; `flight_descent_angle_deg` and `flight_time_s` have no tile at all; and
#: `flight_spin_rpm` is structurally unpairable — it records *only* a solved spin, so a shot with a
#: printed spin to check it against is exactly a shot where it is not recorded (`_spin` above).
#:
#: The third element is **which of the two drawing axes the quantity lives on**, and it is here
#: rather than on the page for the same reason the pairing is: down range and offline are facts
#: about the numbers, not about a canvas, and a surface that decided them from a name would be
#: pattern-matching on `_yds`. It is what lets a viewer put the printed number where the ball
#: would have been — on the ground line at the carry, and at the start line projected out beside
#: the landing point the model bent to.
_COMPARISON_PAIRS = (
    ("flight_carry_yds", "carry_distance_yds", "down_range"),
    ("flight_landing_offline_yds", "start_line_offline_yds", "offline"),
)


class PrintedComparison(NamedTuple):
    """One simulated number beside the printed one it is about, and whether that is a check."""

    simulated_name: str
    measured_name: str
    #: `"down_range"` or `"offline"` — see `_COMPARISON_PAIRS`.
    quantity: str
    simulated: float
    #: What the launch monitor printed, read through `SHOT_MEASUREMENTS` so "measured" here means
    #: what it means everywhere else in the repo.
    measured: float
    #: `simulated - measured`. A number, and `comparable` is what says whether it is an *error*.
    difference: float
    #: False when the difference cannot be read as agreement or disagreement — because the printed
    #: number was an input to the simulation, or because the two are not the same quantity. Both
    #: cases are live on the corpus today and neither is rare.
    comparable: bool
    #: What the difference is, in words, with its numbers derived here rather than typed
    #: (`docs/CODE_STANDARDS.md` R4). This is the sentence a surface must print beside the row; the
    #: row without it is two numbers that look like a validation.
    reading: str


def compare_to_printed(flown: FlownShot, shot: ShotData) -> tuple[PrintedComparison, ...]:
    """Every simulated number this shot printed a counterpart for, with what the gap means.

    **The carry is a check on exactly half of the flights this corpus can produce.** Where the
    screen printed a spin, the flight was flown on it and the printed carry is an independent
    measurement — that is the gate, per shot. Where the spin was *solved*, the printed carry is the
    solve's own input: the flown carry reproduces it to the root search's tolerance, and a column
    of near-zeros there would read as a validation of the thing that was assumed. That sentence is
    `circular_carry_note` below, and the CLI has printed it since M15 P7 — this is the same
    function, not a second wording of it.

    **The offline pair is never a check, and its difference is not an error.** The screen prints no
    offline tile at all — `start_line_offline_yds` is the start line projected out to the carry,
    which is where the ball *started*, while `flight_landing_offline_yds` is where the model has it
    finishing. The gap between them is the curve, and the curve is the part the launch monitor
    never measured.

    Empty for a refused flight: there is no simulated number to set anything beside, and `reason`
    is what that shot owes a reader instead.
    """
    if flown.flight is None:
        return ()
    rows: list[PrintedComparison] = []
    for simulated_name, measured_name, quantity in _COMPARISON_PAIRS:
        read_simulated = FLIGHT_MEASUREMENTS[simulated_name][0]
        read_measured = SHOT_MEASUREMENTS[measured_name][0]
        simulated, measured = read_simulated(flown), read_measured(shot)
        if simulated is None or measured is None:
            continue
        comparable, reading = _reading(simulated_name, flown, simulated, measured)
        rows.append(
            PrintedComparison(
                simulated_name=simulated_name,
                measured_name=measured_name,
                quantity=quantity,
                simulated=simulated,
                measured=measured,
                difference=simulated - measured,
                comparable=comparable,
                reading=reading,
            )
        )
    return tuple(rows)


def _reading(
    simulated_name: str, flown: FlownShot, simulated: float, measured: float
) -> tuple[bool, str]:
    """The sentence for one pair, and whether the difference in it is an error at all."""
    difference = simulated - measured
    if simulated_name == "flight_landing_offline_yds":
        assert flown.flight is not None, "an offline row needs the flight it was read off"
        # Split rather than attributed whole to the curve, because it is not the curve: the two
        # numbers are read off *different carries*, so the start line leans the carry row's own
        # disagreement into this one. `FlightResult.landing_offline_m` is the identity being
        # inverted here — `carry * sin(direction) + curvature * cos(direction)` — and the second
        # term is what is left once the first is taken out, which makes the split exact rather
        # than approximately right.
        direction = flown.flight.launch.launch_direction_deg
        bend = flown.flight.curvature_yds * cos(radians(direction))
        return False, _said(
            f"""not an error and not a comparison: the printed number is where the ball
            *started*, projected out to the carry, and the simulated one is where this model has
            it finishing. Most of the gap is the bend the screen never measured, {bend:+.2f} yd
            of it; the remaining {difference - bend:+.2f} yd is the carry row above leaning on a
            start line {direction:g} deg off target, and it is not a second finding."""
        )
    printed_spin = flown.resolved is not None and flown.resolved.spin_source is SpinSource.MEASURED
    if not printed_spin:
        return False, circular_carry_note(difference)
    return True, _said(
        f"""the screen printed the spin this was flown on, so the carry beside it is an
        independent measurement and this difference is the model's error on this shot:
        {difference:+.2f} yd, {difference / measured:+.2%} against a gate measured at
        +/-{GATE_AGREEMENT_FRACTION:.2%}."""
    )


def circular_carry_note(difference: float | None = None) -> str:
    """Why a solved flight's carry is not a check — one sentence, for a row or for a column.

    `scripts/simulate_flight.py` has printed this since M15 P7, under the `--shots` table of
    solved flights, and M15 P16 put the same claim on a web page one row at a time. Which is two
    surfaces owning one load-bearing sentence, and `analysis/flight_caveats.py`'s docstring is the
    standing argument against that: the CLI is where these were written because a dev CLI was the
    only surface there was, and prose that exists twice goes stale in one place.

    `difference` is what separates the two shapes. Given one, this is a single flight's row and
    the residual is quoted, because a reader looking at `+0.001 yd` deserves to be told what that
    number is before deciding it is agreement. Given none, this is a whole column of them.
    """
    claim = """the printed carry is the *input* to the spin solve, so the flown carry reproduces
        it by construction"""
    if difference is None:
        return _said(
            f"""there is no error column here and there cannot be one: {claim}, and a column of
            zeros would read as a validation of the thing that was assumed."""
        )
    return _said(
        f"""{claim} ({difference:+.3f} yd, which is the root search's own tolerance and not an
        agreement). Nothing here checks the model; what the solve produced is a spin, and its cap
        is what bounds that."""
    )


def _said(body: str) -> str:
    """One paragraph, the source's own line breaks taken out and none put back.

    `flight_caveats._said`'s rule and its reason: a JSON payload and a 72-column terminal want
    different shapes of the same sentence, and only one of them is prose. Not imported from there
    because importing a private helper across modules to save four lines is how two modules come
    to share a name they never agreed on.
    """
    return " ".join(body.split())
