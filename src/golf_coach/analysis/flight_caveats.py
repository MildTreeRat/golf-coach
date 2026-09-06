"""What a simulated flight may not be quoted without. [M15 P14]

Four sentences, and every surface that draws a flight owes all four of them. They were written in
`scripts/simulate_flight.py` (M15 P7) because a dev CLI was the only surface there was; M15 P14
gave the flight an HTTP route and a second reader, and load-bearing prose that exists twice is
prose that goes stale in one place. `contracts/caveats.py` is the precedent and its docstring is
the argument — it lives one layer down for exactly this reason, and the only thing keeping these
out of it is that they read `FlightResult`, `FlightModel` and `ShotFlight`, which are `analysis`
shapes that `contracts` may not import.

**Every number in them is derived, never typed** (`docs/CODE_STANDARDS.md` R4). The clamp
threshold comes off `AeroTable`, the agreement off `GATE_AGREEMENT_FRACTION`, the spin cap off the
`CarryWindow` that shot measured. A re-sourced coefficient table moves all three and the prose
follows it; a retyped one would go quietly stale while still reading as true.

## They compose, and the composition is the shared part

`caveats_for` is what a surface should call, not the four builders individually. *Which* caveats a
flight owes is a rule about the flight — clamped or not, planar or not, solved spin or printed —
and a second surface re-deriving that rule off the same fields is a second place it can be derived
wrongly. The CLI's terminal wrapping stays in the CLI: these return one normalised paragraph each,
with no line breaks and no indent, because a JSON payload and a 72-column terminal want different
shapes of the same sentence and only one of them is prose.

Stdlib + `analysis` shapes only, and no `contracts` import at all — these describe a flight, not a
`SwingResult`.
"""

from __future__ import annotations

from golf_coach.analysis.benchmarks import FlightModel
from golf_coach.analysis.flight import GATE_AGREEMENT_FRACTION, FlightResult
from golf_coach.analysis.flight_infer import ShotFlight, SpinSource

_M_TO_FEET = 1.0 / 0.3048


def _said(body: str) -> str:
    """One paragraph, with the source's own line breaks taken out and none put back.

    The caveats are written as triple-quoted prose so they stay readable here, which means they
    arrive carrying this file's indentation. `contracts/caveats.py` normalises the same way and
    then wraps; these deliberately stop before the wrap, because the terminal is one consumer of
    two and the other one is JSON.
    """
    return " ".join(body.split())


def clamp_caveat(result: FlightResult, model: FlightModel) -> str:
    """Whether this flight read the published table or was flown off the end of it."""
    table = model.coefficients
    if result.fully_clamped:
        return _said(
            f"""every point of this flight sat above the published table, which stops at a spin
            ratio of {table.spin_ratio_max:g} and was measured on a *driver*. Its end row was held
            rather than extended, so lift and drag were one constant pair the whole way — and since
            spin reaches the flight only through the spin ratio and the spin ratio only through
            those two coefficients, the spin above the clamp does not reach this carry at all. The
            same ball at 5,200 rpm and at 30,000 rpm flies exactly this far."""
        )
    if result.clamped_points:
        return _said(
            f"""{result.clamped_points} of {len(result.points)} points fell outside the
            published table ({table.spin_ratio_min:g} to {table.spin_ratio_max:g}) and read a held
            end row rather than a measured one. Over that part of the flight, spin has no route
            into the answer."""
        )
    return _said(
        f"""every point read a measured row of the table ({table.spin_ratio_min:g} to
        {table.spin_ratio_max:g}), which one shot on disk manages and no other — and that shot is
        the one whose spin was *solved* rather than printed. The two go together rather than
        coinciding: a carry the solve can invert is a carry still responding to spin, and spin
        still reaching the answer is exactly a flight below the clamp."""
    )


def gate_caveat() -> str:
    """The agreement and the thing the agreement hides, which are never quoted apart."""
    return _said(
        f"""the model agrees with HD Golf to +/-{GATE_AGREEMENT_FRACTION:.2%} on the two shots
        where a full launch-condition set and an independent carry are both on disk (`--gate`) —
        while ranking those two shots backwards. That percentage is therefore the size of what this
        model cannot see, not a calibration error, and a carry from here is not "accurate to
        {GATE_AGREEMENT_FRACTION:.1%}"."""
    )


def planar_caveat(count: int = 1) -> str:
    """What a flight with no spin axis does and does not say about where the ball finished."""
    subject = f"{count} of these flights are" if count > 1 else "this flight is"
    return _said(
        f"""{subject} drawn in the vertical plane, because nothing on the screen fixes how far
        the spin axis was tilted (ADR-027 §Decision 5's third branch). The curvature is then zero
        by construction and the landing offline reduces to `carry * sin(start line)` — which is
        `shot_measure`'s `start_line_offline_yds` reached by a much longer route, and not a landing
        point. The curve is missing from it, and on a shot the screen called a fade the curve is
        most of the lateral miss."""
    )


def inferred_caveat(resolved: ShotFlight) -> str:
    """What an inferred spin is, said where the number is printed rather than in a footnote."""
    if resolved.spin is None:  # pragma: no cover - only called on an inferred flight
        return ""
    window = resolved.spin.window
    return _said(
        f"""the spin on this flight was not measured. It is the spin this integrator needs in
        order to fly the carry HD Golf printed, capped at {window.high_plateau_min_rpm:.0f} rpm
        where carry stops responding to spin at all, and chosen between two candidates by the
        club's loft. Both shots whose real spin is on disk sit above their own cap, and the one
        the solve answers on comes back 47% low — so this is a path-drawing device and not a spin
        rate the golfer produced."""
    )


def altitude_caveat(altitude_m: float) -> str:
    """That the gate was measured at sea level and this flight was not flown there."""
    return _said(
        f"""the gate above is a *sea-level* gate: HD Golf printed its carries for a ball hit
        indoors near sea level, and this flight was flown at {altitude_m:g} m
        ({altitude_m * _M_TO_FEET:.0f} ft). The agreement measured there does not transfer, and an
        altitude picked because the agreement improved would be fitting the atmosphere to the
        residual of a held coefficient (ADR-027's 2026-09-05e addendum)."""
    )


def caveats_for(
    result: FlightResult,
    resolved: ShotFlight,
    model: FlightModel,
    *,
    altitude_m: float = 0.0,
) -> tuple[str, ...]:
    """Everything one flown shot has to be read with, in the order a reader needs them.

    The clamp first because it is about this flight, the gate second because it is about the
    model, then the two that are only sometimes owed. The order is part of the shared rule: a
    surface that led with the gate would be answering "how good is this model" before "what is
    this line", and the first question a reader has is the second one.
    """
    said = [clamp_caveat(result, model), gate_caveat()]
    if not resolved.curve_is_drawn:
        said.append(planar_caveat())
    if resolved.spin_source is SpinSource.INFERRED:
        said.append(inferred_caveat(resolved))
    if altitude_m:
        said.append(altitude_caveat(altitude_m))
    return tuple(said)
