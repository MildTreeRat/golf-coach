"""Dev CLI: fly a golf ball from launch conditions you type. [M15 P7]

Usage:
    python scripts/simulate_flight.py --ball-speed 90.7 --launch-angle 20.9 --spin 5991
    python scripts/simulate_flight.py ... --launch-direction -5.3 --spin-axis 2.5
    python scripts/simulate_flight.py ... --altitude 1609   # the Denver what-if, in metres
    python scripts/simulate_flight.py ... --points 12       # sample the path itself
    python scripts/simulate_flight.py --gate                # the two shots the gate is made of
    python scripts/simulate_flight.py --shots               # every shot on disk, flown or refused
    python scripts/simulate_flight.py --shot 2026-08-23-4   # one of them, in full
    python scripts/simulate_flight.py ... --verbose         # the model's provenance, per block

**Typed conditions and stored shots are two modes, not one.** `--ball-speed ...` flies conditions
nobody hit, which is the whole use of a what-if; `--shots` reads the corpus through the tolerant
readers that already exist and flies what it can. M15 P10 added the second and kept them apart,
because a what-if that quietly filled in a missing spin would be M15 P8's solver wearing a disguise
— here a solved spin is named as one on every row it produced, with the cap it sits under.

**Reading the corpus is a join, and the join is what found the club.** A shot carries no club and
never will; the *swing* it arrived with does, and `storage/flight_inputs.py` matches the two on the
shot photo's sha256. ADR-027's 2026-09-05g addendum reported that no shot on disk resolves a loft;
eleven of the thirteen are attached to a swing that names a club, and what is actually missing is a
declared loft on the bag entry that tag points at.

`--gate` is that same rule rather than an exception to it: `VALIDATION_SHOTS` is a constant in
`analysis/flight.py`, hand-typed from the two 2026-08-10 screens, so flying it here reads no
artifact either. It exists because every later surface has to quote the agreement *and* what the
agreement conceals (ADR-027's 2026-09-05c addendum), and this is the first place a human can see
both instead of reading the percentage and stopping.

Exits 0 when a ball flew and 2 when none did — argparse's own code for a usage error, and
`simulate_flight` refuses the same class of thing: a launch angle at the horizontal is a shot that
rolls, and roll is out of scope. There is deliberately no exit 1 for "flew, but flagged", which is
`analyze_bundle.py`'s middle code. Every iron in this repo's corpus flies its whole path on a held
end row of a driver's coefficient table, so a flagged exit would fire on essentially every real
shot and would mean nothing by the second run. The clamp is said in words instead, every time.

**Over stored shots that rule inverts, and deliberately: there a refusal is the finding.** Ten of
the thirteen shots on disk cannot be flown at all, and exiting 2 on the corpus's own state would
report the repo as broken every time it is read honestly. So `--shots` exits 0 whatever it finds,
and the one exit 2 it has is a `--shot` id nothing on disk carries, which really is a usage error.
"""

from __future__ import annotations

import argparse
import sys
import textwrap
from typing import NamedTuple

from golf_coach.analysis.benchmarks import (
    FlightModel,
    SourceNote,
    flight_dataset_info,
    load_flight_model,
)
from golf_coach.analysis.flight import (
    DEFAULT_STEP_S,
    GATE_AGREEMENT_FRACTION,
    VALIDATION_SHOTS,
    FlightResult,
    GateComparison,
    LaunchConditions,
    gate_comparisons,
    simulate_flight,
)
from golf_coach.analysis.flight_caveats import (
    altitude_caveat,
    caveats_for,
    clamp_caveat,
    gate_caveat,
    planar_caveat,
)
from golf_coach.analysis.flight_infer import ShotFlight, SpinSource, flight_for_shot
from golf_coach.analysis.flight_measure import (
    FlownShot,
    circular_carry_note,
    compare_to_printed,
)
from golf_coach.config import settings
from golf_coach.contracts.unscored import UNSCORED_REASONS, UnscoredReason
from golf_coach.launch_monitor.screen.store import ShotStore
from golf_coach.storage.flight_inputs import FlightInputs, LoftGap, read_flight_inputs

#: 72 columns, the width `club_profile.py` prints at — two dev CLIs disagreeing about how wide a
#: terminal is would be a small thing done twice.
_WIDTH = 72

_M_S_TO_MPH = 1.0 / 0.44704


def _wrap(body: str, *, bullet: str = "  - ") -> str:
    """One paragraph, wrapped and hanging-indented under its bullet.

    The caveats are written as triple-quoted prose so they stay readable in the source, which
    means they arrive with the source's own line breaks in them; `split()` throws those away
    before `fill` puts the right ones in. `contracts/caveats.py` composes its sentences the same
    way for the same reason.
    """
    return textwrap.fill(
        " ".join(body.split()),
        width=_WIDTH,
        initial_indent=bullet,
        subsequent_indent=" " * len(bullet),
    )


def _row(label: str, value: str, note: str = "") -> str:
    line = f"  {label:<16}{value:>14}"
    return f"{line}   {note}" if note else line


def _signed(value: float, places: int = 2) -> str:
    """An explicit `+`, because every lateral number here is signed and the sign is the meaning."""
    return f"{value:+.{places}f}"


# --------------------------------------------------------------------- the flight


def _report_flight(result: FlightResult, model: FlightModel, *, points: int) -> None:
    launch = result.launch
    air = model.atmosphere
    print()
    print(
        f"  launch    {launch.ball_speed_mph:g} mph, {launch.launch_angle_deg:g} deg up, "
        # Whole rpm: the root search stops at half a revolution and a solved spin printed to two
        # decimals would claim a precision the inverse problem does not have.
        f"{launch.spin_rpm:.0f} rpm, start {launch.launch_direction_deg:+g} deg, "
        f"axis {launch.spin_axis_deg:+g} deg"
    )
    print(f"  air       {air.name}, {air.density_kg_m3:.4f} kg/m3, {air.temperature_k:.2f} K")
    print()

    # Carry, offline and curvature are one landing point read in two frames — `FlightResult`
    # carries the identity between them — so they print together and each says which frame.
    print(_row("carry", f"{result.carry_yds:.2f} yd", "along the launch line"))
    print(_row("offline", f"{_signed(result.landing_offline_yds)} yd", "from the target line"))
    print(_row("curvature", f"{_signed(result.curvature_yds)} yd", "from the launch line"))
    print(_row("apex", f"{result.apex_yds:.2f} yd"))
    print(_row("hang time", f"{result.flight_time_s:.2f} s"))
    print(_row("descent", f"{result.descent_angle_deg:.1f} deg"))
    print(_row("spin at landing", f"{result.landing.spin_rpm:.0f} rpm"))
    print(
        _row(
            "spin ratio",
            f"{result.spin_ratio_min:.3f} - {result.spin_ratio_max:.3f}",
            f"{result.clamped_points} of {len(result.points)} points held",
        )
    )

    if points:
        _report_points(result, points)


def _report_points(result: FlightResult, wanted: int) -> None:
    """A sample of the path — the launch, the landing, and evenly spaced indices between.

    The sampling rule is `FlightResult.sample`'s, which [M15 P14] moved there when the flight
    route needed the same polyline this table prints.
    """
    print()
    print(f"  {'t s':>6}{'down yd':>10}{'up yd':>9}{'right yd':>10}{'mph':>8}{'S':>8}  coeff")
    for p in result.sample(wanted):
        print(
            f"  {p.t_s:>6.2f}{p.x_yds:>10.2f}{p.y_yds:>9.2f}{p.z_yds:>10.2f}"
            f"{p.speed_m_s * _M_S_TO_MPH:>8.1f}{p.spin_ratio:>8.3f}"
            f"  {'held' if p.clamped else 'read'}"
        )


# --------------------------------------------------------------------- what has to be said


def _report_caveats(said: list[str]) -> None:
    print()
    print("  Read it with these:")
    for caveat in said:
        print(_wrap(caveat))


# --------------------------------------------------------------------- provenance


def _report_provenance(model: FlightModel) -> None:
    """Per block, because the five blocks have four sources (ADR-027 §Decision 2)."""
    info = flight_dataset_info()
    blocks: list[tuple[str, SourceNote]] = [
        ("ball", model.ball.source),
        ("atmosphere", model.atmosphere.source),
        ("atmosphere_profile", model.atmosphere_profile.source),
        ("spin_decay", model.spin_decay.source),
        ("coefficients", model.coefficients.source),
    ]

    print()
    print(f"  {info.name}, assembled {info.assembled_on} by {info.assembled_by}")
    print(f"  fitted: {str(info.fitted).lower()}")
    for name, source in blocks:
        print()
        print(f"  {name}")
        print(_wrap(source.citation, bullet="    "))
        print(_wrap(source.url, bullet="    "))


# --------------------------------------------------------------------- the gate


def _report_gate(comparisons: tuple[GateComparison, ...]) -> None:
    """Both validation shots, beside the carries the simulator printed for them."""
    print()
    print(f"  {'shot':<14}{'HD Golf':>10}{'model':>10}{'error yd':>10}{'error':>9}")
    for c in comparisons:
        print(
            f"  {c.shot.shot_id:<14}{c.shot.simulator_carry_yds:>10.1f}"
            f"{c.flight.carry_yds:>10.2f}{c.error_yds:>+10.2f}{c.error_fraction:>+9.2%}"
            f"  {'ok' if c.agrees else 'OUT'}"
        )

    # The ordering, measured here rather than quoted: it is the finding P4 exists for, and the one
    # thing a per-shot tolerance cannot report. `>` because `VALIDATION_SHOTS` is ordered and the
    # first shot is the lower-spin one.
    first, second = comparisons[0], comparisons[1]
    measured_gap = first.shot.simulator_carry_yds - second.shot.simulator_carry_yds
    model_gap = first.flight.carry_yds - second.flight.carry_yds
    inverted = measured_gap * model_gap < 0
    print()
    print(
        _wrap(
            f"""HD Golf has {first.shot.shot_id} — the lower spin of the two — flying
            {abs(measured_gap):.1f} yd {'further' if measured_gap > 0 else 'shorter'} than
            {second.shot.shot_id}; this model has it {abs(model_gap):.2f} yd
            {'further' if model_gap > 0 else 'shorter'}. Each shot passes on its own, and the
            ordering between them is {'inverted' if inverted else 'right'} — which no per-shot
            tolerance can see. Read GATE_AGREEMENT_FRACTION
            ({GATE_AGREEMENT_FRACTION:.2%}) as the size of that, not as an accuracy.""",
            bullet="  ",
        )
    )
    if all(c.flight.fully_clamped for c in comparisons):
        print()
        print(
            _wrap(
                """Both flights are clamped end to end, so this is the agreement between HD Golf
                and a held constant pair — not evidence about the published table, which neither
                flight read a single row of.""",
                bullet="  ",
            )
        )


# --------------------------------------------------------------------- stored shots


class _Resolved(NamedTuple):
    """One stored shot: what it was joined to, what stood in for its spin, and where it landed."""

    inputs: FlightInputs
    flight: ShotFlight
    result: FlightResult | None
    #: What `simulate_flight` refused, on the conditions `flight_for_shot` does not pre-check for
    #: it. `None` on every shot on disk; kept because the integrator owns those guards, and a CLI
    #: that let one escape would print a traceback in the middle of a corpus read.
    error: str | None


def _resolve_stored(*, model: FlightModel, step_s: float) -> list[_Resolved]:
    """Every parsed shot on disk, joined to its swing and flown as far as it can be.

    **Two existing tolerant readers and no third one.** `ShotStore.all()` skips a file it cannot
    parse, and `read_flight_inputs` treats a missing swing, club tag, bag entry and declared loft
    as four different states rather than as one error. `scripts/` is where the two meet, because
    `storage/` may not import `launch_monitor/` and neither may import the other (ADR-008) — which
    is why the join takes its shots as an argument instead of opening the store itself.
    """
    rows = read_flight_inputs(
        ShotStore(settings.shots_dir).all(),
        sessions_dir=settings.sessions_dir,
        golfers_dir=settings.golfers_dir,
    )
    resolved: list[_Resolved] = []
    for row in rows:
        flight = flight_for_shot(
            row.shot,
            loft_deg=row.loft_deg,
            handedness=row.handedness,
            model=model,
            step_s=step_s,
        )
        result: FlightResult | None = None
        error: str | None = None
        if flight.launch is not None:
            try:
                result = simulate_flight(flight.launch, model=model, step_s=step_s)
            except ValueError as exc:
                error = str(exc)
        resolved.append(_Resolved(row, flight, result, error))
    return resolved


def _club_of(row: FlightInputs) -> str:
    return row.club.value if row.club is not None else "-"


def _carry_column(r: _Resolved) -> str:
    printed = r.inputs.shot.carry_distance
    return f"{printed:>9.1f}" if printed is not None else f"{'-':>9}"


def _report_stored(resolved: list[_Resolved]) -> None:
    """The corpus in three blocks: measured spin, solved spin, and not flown."""
    measured = [r for r in resolved if r.flight.spin_source is SpinSource.MEASURED]
    inferred = [r for r in resolved if r.flight.spin_source is SpinSource.INFERRED]
    refused = [r for r in resolved if r.result is None]

    print()
    summary = (
        f"{len(resolved)} stored shots, each joined to the swing it arrived with: "
        f"{len(measured)} flown on a spin the screen printed, {len(inferred)} on a spin solved "
        f"from the printed carry, {len(refused)} not flown."
    )
    print(_wrap(summary, bullet="  "))

    if measured:
        print()
        print("  Spin measured off the screen")
        print(
            f"    {'shot':<14}{'club':<6}{'mph':>6}{'deg':>6}{'rpm':>7}"
            f"{'printed':>9}{'flown':>8}{'error':>8}"
        )
        for r in measured:
            launch, result = r.flight.launch, r.result
            if launch is None or result is None:
                continue
            printed = r.inputs.shot.carry_distance
            error = (
                f"{(result.carry_yds - printed) / printed:+.2%}"
                if printed is not None and printed > 0.0
                else "-"
            )
            print(
                f"    {r.inputs.shot.shot_id:<14}{_club_of(r.inputs):<6}"
                f"{launch.ball_speed_mph:>6.1f}{launch.launch_angle_deg:>6.1f}"
                f"{launch.spin_rpm:>7.0f}{_carry_column(r)}"
                f"{result.carry_yds:>8.2f}{error:>8}"
            )

    if inferred:
        print()
        print("  Spin solved from the printed carry")
        print(
            f"    {'shot':<14}{'club':<6}{'mph':>6}{'deg':>6}{'rpm':>7}"
            f"{'printed':>9}{'flown':>8}{'cap':>8}"
        )
        for r in inferred:
            launch, result, spin = r.flight.launch, r.result, r.flight.spin
            if launch is None or result is None or spin is None:
                continue
            print(
                f"    {r.inputs.shot.shot_id:<14}{_club_of(r.inputs):<6}"
                f"{launch.ball_speed_mph:>6.1f}{launch.launch_angle_deg:>6.1f}"
                f"{launch.spin_rpm:>7.0f}{_carry_column(r)}"
                f"{result.carry_yds:>8.2f}{spin.cap_rpm:>8.0f}"
            )
        print()
        # Composed in `analysis/flight_measure.py` rather than here, because M15 P16 put the same
        # claim on a web page one row at a time and this CLI is no longer its only reader. The cap
        # sentence stays local: it is about this table's last column and nothing else has one.
        print(_wrap(circular_carry_note(), bullet="    - "))
        print(
            _wrap(
                """`cap` is the spin above which carry stops responding to spin at all — an
                answer near it is a floor, not a reading.""",
                bullet="    - ",
            )
        )

    if refused:
        print()
        print(f"  Not flown ({len(refused)})")
        for r in refused:
            reason = r.flight.reason
            label = reason.value if reason is not None else "refused by the integrator"
            print(f"    {r.inputs.shot.shot_id:<14}{_club_of(r.inputs):<6}{label}")
            print(_wrap(r.error or r.flight.detail, bullet="        "))
        _report_reason_legend(refused)


def _report_reason_legend(refused: list[_Resolved]) -> None:
    """What each reason above means, in `contracts/unscored.py`'s own words.

    Read out of `UNSCORED_REASONS` rather than restated here, because that table is what the
    results page, the MCP payload and the coaching brief all render (R4). A second wording of
    `carry_unreachable` in a CLI is a second thing to keep in step with the golfer-facing one.
    """
    seen = {r.flight.reason for r in refused if r.flight.reason is not None}
    if not seen:
        return
    print()
    for reason in sorted(seen, key=lambda item: item.value):
        spec = UNSCORED_REASONS[reason]
        print(_wrap(f"{reason.value}: {spec.summary}", bullet="    - "))


def _report_shot(
    r: _Resolved, model: FlightModel, *, points: int, altitude_m: float = 0.0
) -> None:
    """One stored shot in full — the join, the two stand-ins, and the flight if there was one."""
    row = r.inputs
    shot = row.shot
    print()
    print(f"  shot      {shot.shot_id}, {shot.source.value}, {shot.timestamp:%Y-%m-%d %H:%M}")
    print(f"  swing     {row.swing_ref or 'no swing on disk carries this photo'}")
    if row.also_attached_to:
        print(
            _wrap(
                f"""the same photo is attached to {', '.join(row.also_attached_to)} as well; the
                earliest arrival is the one read here, which is `storage/corpus.py`'s rule for a
                re-uploaded bundle.""",
                bullet="            ",
            )
        )
    hand = row.handedness.value if row.handedness is not None else "-"
    print(f"  golfer    {row.player_id or '-'}, {hand}-handed")
    loft = f"{row.loft_deg:g} deg" if row.loft_deg is not None else f"none ({row.loft_gap})"
    print(f"  club      {_club_of(row)}, loft {loft}")

    print()
    print(_wrap(f"spin: {r.flight.detail}", bullet="  - "))
    print(_wrap(f"axis: {r.flight.axis.detail}", bullet="  - "))
    if r.flight.axis.sign_disagrees:
        print(
            _wrap(
                f"""the direction derived here ({r.flight.axis.curve_direction}) contradicts the
                `Shot Type` tile ({r.flight.axis.screen_shape}). Flagged and never overwritten
                (ADR-027 §Decision 5).""",
                bullet="  ! ",
            )
        )

    if r.result is None:
        print()
        print(_wrap(r.error or "No flight, for the reason above.", bullet="  "))
        return

    _report_flight(r.result, model, points=points)
    _report_comparison(r)
    # Composed there rather than here: which caveats a flown shot owes is a rule about the flight,
    # and M15 P14 gave that rule a second reader (`api/app.py`'s flight route). Two surfaces
    # deriving it separately off the same fields is two places it can be derived wrongly.
    _report_caveats(list(caveats_for(r.result, r.flight, model, altitude_m=altitude_m)))


def _report_comparison(r: _Resolved) -> None:
    """The simulated numbers this screen printed a counterpart for, and what each gap is.

    **Which number pairs with which, and whether the gap is an error, is decided in
    `analysis/flight_measure.compare_to_printed`** — the same rule the flight route serves to
    `flight.html` (M15 P16). Two surfaces deriving a comparison off two registries is two places
    it can be derived wrongly, and on this corpus neither of the two pairs is a validation: one is
    the solve's own input read back, and the other is not the same quantity twice.

    `FlownShot` is assembled here rather than by calling `fly_shot`, because this CLI resolves and
    flies in two steps on purpose — it reports an integrator error separately from a refusal — and
    past the guard above both halves exist. This is those two halves in the shape the registries
    read, not a third way of producing them.
    """
    if r.result is None:
        return
    rows = compare_to_printed(FlownShot(r.flight, r.result, None, r.flight.detail), r.inputs.shot)
    if not rows:
        return
    print()
    print("  Beside what the screen printed")
    print(f"    {'measurement':<27}{'simulated':>10}{'printed':>10}{'apart':>8}")
    for row in rows:
        # No "is this a check" column. The answer is a sentence on both of the two pairs — one is
        # the solve's own input read back, the other is a different quantity — and a column would
        # have to say it in two words, which is how a reader ends up reading the numbers instead.
        print(
            f"    {row.simulated_name:<27}{row.simulated:>10.2f}"
            f"{row.measured:>10.2f}{row.difference:>+8.2f}"
        )
    print()
    for row in rows:
        print(_wrap(f"{row.simulated_name}: {row.reading}", bullet="    - "))


def _report_stored_findings(resolved: list[_Resolved], *, altitude_m: float = 0.0) -> None:
    """What the corpus itself says, counted at run time rather than quoted (R4)."""
    said: list[str] = []

    tagged = [r for r in resolved if r.inputs.club is not None]
    if tagged:
        with_loft = sum(1 for r in tagged if r.inputs.loft_deg is not None)
        said.append(
            f"""{len(tagged)} of {len(resolved)} shots reach a club tag through the swing they
            arrived with, and {with_loft} of those reach a declared loft. The club is on the
            *swing* and never on the shot, so nothing reading `ShotData` alone can see it — which
            is what ADR-027's 2026-09-05g addendum was measuring when it reported none."""
        )

    no_entry = [r for r in resolved if r.inputs.loft_gap is LoftGap.NO_BAG_ENTRY]
    if no_entry:
        slots = sorted({_club_of(r.inputs) for r in no_entry})
        blocked = sum(1 for r in no_entry if r.flight.reason is UnscoredReason.NO_CLUB_LOFT)
        said.append(
            f"""{len(no_entry)} shots name a club with no bag entry ({', '.join(slots)}), so the
            loft that would pick the branch has never been declared. That is the bag page and not
            a bay session: {blocked} of them are otherwise solvable and would name a spin the
            moment the club is looked up."""
        )

    planar = [r for r in resolved if r.result is not None and not r.flight.curve_is_drawn]
    if planar:
        said.append(planar_caveat(len(planar)))

    for r in (r for r in resolved if r.flight.axis.sign_disagrees):
        said.append(
            f"""{r.inputs.shot.shot_id} disagrees with itself about which way it curved: the screen
            printed `{r.inputs.shot.shot_type}` and the face-to-path behind it is
            {r.flight.axis.curve_direction}. Flagged rather than resolved — quietly picking one is
            how ADR-014's sign inversion survived a milestone."""
        )

    said.append(_gate_from_disk(resolved))
    said.append(gate_caveat())
    if altitude_m:
        said.append(altitude_caveat(altitude_m))
    _report_caveats(said)


def _gate_from_disk(resolved: list[_Resolved]) -> str:
    """Whether the gate's hand-typed launch conditions match the ones the OCR stored.

    `VALIDATION_SHOTS` was typed off two screens by hand in M15 P4 and has been this model's only
    external check ever since. The same two screens were also parsed into the shot store, so the
    comparison costs nothing the moment anything reads the corpus — and it is the first time that
    constant has been checkable rather than merely stated.
    """
    stored = {r.inputs.shot.shot_id: r for r in resolved}
    checked = [(shot, stored[shot.shot_id]) for shot in VALIDATION_SHOTS if shot.shot_id in stored]
    if not checked:
        return """neither shot the gate is made of is in the shot store, so the hand-typed
            VALIDATION_SHOTS could not be checked against what the OCR read."""
    differences = [
        f"{shot.shot_id} (typed {shot.launch}, stored {r.flight.launch})"
        for shot, r in checked
        if r.flight.launch != shot.launch
    ]
    if differences:
        return f"""the gate's typed launch conditions do not match what the OCR stored for the
            same screens: {'; '.join(differences)}. One of the two readings is wrong, and every
            number the gate has ever printed rests on the typed pair."""
    return f"""the {len(checked)} shots the gate is made of are in the shot store too, and the
        launch conditions read off disk match the hand-typed VALIDATION_SHOTS to the digit,
        including both lateral fields. That constant has been unverifiable since M15 P4; this is
        the check, and it comes from reading the corpus rather than from trusting it."""


# --------------------------------------------------------------------- the CLI


def _build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Fly a golf ball from launch conditions, or re-fly the validation gate.",
        epilog=(
            "Four modes and exactly one at a time: typed conditions, --gate, --shots, --shot. "
            "Angles are degrees, and + is right of the target line for both lateral fields."
        ),
    )
    parser.add_argument("--ball-speed", type=float, help="mph, as the screen prints it")
    parser.add_argument("--launch-angle", type=float, help="degrees above the horizontal")
    parser.add_argument("--spin", type=float, help="backspin, rpm")
    parser.add_argument(
        "--launch-direction",
        type=float,
        default=0.0,
        help="degrees off the target line at launch, + = right (default 0)",
    )
    parser.add_argument(
        "--spin-axis",
        type=float,
        default=0.0,
        help=(
            "degrees of spin-axis tilt, + = curves right (default 0). Geometric, and deliberately "
            "not ShotData.spin_axis's fade-positive sign: that flip needs a handedness, and "
            "M15 P9 owns it"
        ),
    )
    parser.add_argument(
        "--altitude",
        type=float,
        default=0.0,
        help="metres above sea level; swaps the air for ISO 2533's at that height (default 0)",
    )
    parser.add_argument(
        "--step",
        type=float,
        default=DEFAULT_STEP_S,
        help=f"RK4 step, seconds (default {DEFAULT_STEP_S})",
    )
    parser.add_argument(
        "--points", type=int, default=0, help="print this many samples of the path (default none)"
    )
    parser.add_argument(
        "--gate",
        action="store_true",
        help="fly the two 2026-08-10 shots against the carries HD Golf printed for them",
    )
    parser.add_argument(
        "--shots",
        action="store_true",
        help="read every parsed shot on disk, joined to its swing, and fly what can be flown",
    )
    parser.add_argument(
        "--shot",
        metavar="SHOT_ID",
        help="one stored shot in full, by the id --shots prints (e.g. 2026-08-23-4)",
    )
    parser.add_argument(
        "--verbose", action="store_true", help="print the model's provenance, block by block"
    )
    return parser


def main(argv: list[str] | None = None) -> int:
    parser = _build_parser()
    args = parser.parse_args(argv)

    typed = (args.ball_speed, args.launch_angle, args.spin)
    any_typed = any(value is not None for value in typed)
    modes = sum((args.gate, args.shots, args.shot is not None, any_typed))
    if modes > 1:
        parser.error(
            "typed conditions, --gate, --shots and --shot are four modes; pick one. The stored "
            "modes fly what the screen recorded and the typed one flies what you ask for, and "
            "silently merging them is how an inferred spin would end up wearing a measured label"
        )
    if modes == 0:
        parser.error(
            "--ball-speed, --launch-angle and --spin are all required (or --gate, --shots, --shot)"
        )
    if any_typed and any(value is None for value in typed):
        parser.error("--ball-speed, --launch-angle and --spin are required together")

    print()
    print("Ball flight — simulated, not measured")
    print("-" * _WIDTH)

    model = load_flight_model()
    try:
        # The air is a block of the model and the model is already a parameter, so altitude is a
        # different model rather than a different call (M15 P6). A zero altitude is left alone
        # rather than round-tripped: `at_altitude(0.0)` is bit-identical by construction, and not
        # calling it keeps the default model's `source` saying "published" rather than "generated".
        if args.altitude:
            model = model.at_altitude(args.altitude)
    except ValueError as exc:
        print(f"No flight: {exc}")
        return 2

    if args.shots or args.shot is not None:
        resolved = _resolve_stored(model=model, step_s=args.step)
        if not resolved:
            print()
            print(
                _wrap(
                    f"""No parsed shots in {settings.shots_dir}. Shots arrive by
                    `scripts/import_shot_screens.py` or with an uploaded bundle, and the store is
                    keyed by the photo's sha256 — an empty one is a repo nobody has imported a
                    screen into yet, not an error.""",
                    bullet="  ",
                )
            )
            return 0
        if args.shot is not None:
            match = next((r for r in resolved if r.inputs.shot.shot_id == args.shot), None)
            if match is None:
                parser.error(
                    f"no stored shot has the id {args.shot!r}; --shots prints the "
                    f"{len(resolved)} that do"
                )
            else:
                _report_shot(match, model, points=args.points, altitude_m=args.altitude)
        else:
            _report_stored(resolved)
            _report_stored_findings(resolved, altitude_m=args.altitude)
    elif args.gate:
        if args.altitude:
            print()
            print(
                _wrap(
                    f"""Flown at {args.altitude:g} m. The gate is a sea-level gate, so what
                    follows is a what-if and not a validation: HD Golf printed these carries for a
                    ball hit indoors near sea level, and an altitude chosen because the agreement
                    improved would be fitting the atmosphere to the residual of a held coefficient
                    (ADR-027's 2026-09-05e addendum).""",
                    bullet="  ",
                )
            )
        _report_gate(gate_comparisons(model=model, step_s=args.step))
    else:
        launch = LaunchConditions(
            ball_speed_mph=args.ball_speed,
            launch_angle_deg=args.launch_angle,
            spin_rpm=args.spin,
            launch_direction_deg=args.launch_direction,
            spin_axis_deg=args.spin_axis,
        )
        try:
            result = simulate_flight(launch, model=model, step_s=args.step)
        except ValueError as exc:
            # `simulate_flight` raises rather than refusing, and its docstring names this as the
            # boundary that owns the check: a ball at or below the horizontal rolls, and there is
            # no sentence about a swing to be said about it.
            print(f"No flight: {exc}")
            return 2

        _report_flight(result, model, points=args.points)
        caveats = [clamp_caveat(result, model), gate_caveat()]
        if args.altitude:
            caveats.append(altitude_caveat(args.altitude))
        _report_caveats(caveats)

    if args.verbose:
        _report_provenance(model)

    print()
    return 0


if __name__ == "__main__":
    sys.exit(main())
