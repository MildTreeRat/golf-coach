"""The drawable flight behind `GET .../flight`, derived at read time. [M15 P14]

`analysis.json` already carries the six numbers `flight_measure.FLIGHT_MEASUREMENTS` reads off a
flight — but it carries no *path*, and a path is the whole of what a viewer draws. Storing nine
hundred points per swing to avoid re-integrating them would be storing a derivation that costs
about ten milliseconds, so this re-flies instead. `resolve_tempo_plan` is the precedent on the
swing-detail route: derived beside the artifact, never folded into it.

**Re-flying is also the honest read rather than merely the cheap one.** The bag is editable. A 3
wood whose loft is typed in this afternoon changes what the same shot's flight *is*, and a route
serving the stored numbers would keep drawing the refusal until someone re-analysed the swing. So
the artifact records what was true when the engine ran, and this says what is true now — which is
why the two can disagree and why only one of them is a measurement of anything.

## What it serves, and why it is more than the six numbers

M15 P16 is a viewer-honesty phase, so it must be a *rendering* phase: everything it has to show
has to be in this payload or it becomes a second route change. Hence the spin's source beside the
spin, the axis's refusal beside the curve, `curve_is_drawn`, the shot's own printed numbers for
the measured landing point to sit beside the simulated one, and `caveats` — composed by
`analysis/flight_caveats.caveats_for`, which is also what the CLI prints, so the two surfaces
cannot come to say different things about one flight.

⚠️ **P16 needed one thing more than that, and it was not a rendering decision.** Setting a
simulated number beside a printed one means deciding *which* printed one and *whether the gap is
an error* — a registry question, and on this corpus the answer is "neither of the two pairs is a
check" for two different reasons. `analysis/flight_measure.compare_to_printed` is where that is
decided, and `comparison` is it read here; a page pairing `flight_carry_yds` with
`carry_distance_yds` in JavaScript would have been the third copy of two registries this route
exists to keep singular.

Both registries are read rather than retyped: `FLIGHT_MEASUREMENTS` for the six simulated numbers
(so this route and the stored artifact cannot disagree about what a flight produces) and
`SHOT_MEASUREMENTS` for the printed ones (so "measured" here means exactly what it means
everywhere else in the repo).

No `fastapi` import, which is what keeps this testable on a base install and off
`tests/api/test_pipeline_imports.py`'s list of things to worry about. The route in `app.py` is the
thin part: it resolves the manifest, the shot, the loft and the handedness, and hands them here.
"""

from __future__ import annotations

from golf_coach.analysis.benchmarks import FlightModel, load_flight_model
from golf_coach.analysis.flight import FlightResult
from golf_coach.analysis.flight_caveats import caveats_for
from golf_coach.analysis.flight_infer import ShotFlight
from golf_coach.analysis.flight_measure import (
    FLIGHT_MEASUREMENTS,
    FLIGHT_SOURCE,
    FlownShot,
    compare_to_printed,
    flight_unscored,
    fly_shot,
)
from golf_coach.analysis.shot_measure import SHOT_MEASUREMENTS
from golf_coach.contracts.club import ClubId
from golf_coach.contracts.golfer import Handedness
from golf_coach.contracts.shot import ShotData
from golf_coach.contracts.unscored import UnscoredReason

#: How many points of the path to send. The integrator takes about nine hundred steps at
#: `DEFAULT_STEP_S`, which is more polyline than any canvas resolves and about ten times the JSON;
#: 120 segments are smooth at any plausible page width. `FlightResult.sample` keeps the launch and
#: the solved landing whatever this is, so the two points a viewer measures off are never the ones
#: dropped.
DEFAULT_PATH_POINTS = 120

_M_S_TO_MPH = 1.0 / 0.44704


def flight_view(
    shot: ShotData,
    *,
    club: ClubId | None,
    loft_deg: float | None,
    loft_remedy: str | None,
    handedness: Handedness | None,
    points: int = DEFAULT_PATH_POINTS,
    model: FlightModel | None = None,
) -> dict:
    """One stored shot flown for a viewer — or every reason it was not, in the same shape.

    A refusal is not an error and does not become one here: ten of the thirteen shots on disk
    cannot be flown, and a route that raised on them would report the corpus's own state as a
    server fault. `flew` is the field to branch on; `reason`, `detail` and `unscored` say what
    happened, and the resolution survives either way — `spin` still carries which of the seven
    `SpinSolveCase` shapes the printed carry landed on, which is most of what a reader wants when
    there is no number.

    `loft_remedy` is passed in rather than derived because it names a *page* ("add the club on the
    bag page"), and which pages exist is the shell's knowledge — `api.pipeline.loft_remedy` is the
    one definition of those four sentences and the analysis pipeline prints the same ones.
    """
    flown = fly_shot(shot, loft_deg=loft_deg, handedness=handedness, model=model)
    resolved = flown.resolved

    return {
        "shot": {
            "shot_id": shot.shot_id,
            "source": shot.source.value,
            "timestamp": shot.timestamp.isoformat(),
            "club": club.value if club is not None else None,
            "loft_deg": loft_deg,
            # Sent only once the flight has actually asked for a loft and gone without, which is
            # `api/pipeline.py::_loft_for`'s rule arriving at the second shell that needs it: a
            # shot whose screen printed a spin never reaches for a loft, and telling that golfer
            # to go and fill in the bag page would be a repair for a problem they do not have.
            "loft_remedy": (
                loft_remedy if flown.reason is UnscoredReason.NO_CLUB_LOFT else None
            ),
            "handedness": handedness.value if handedness is not None else None,
            "shot_type": shot.shot_type,
        },
        "flew": flown.flew,
        "reason": flown.reason.value if flown.reason is not None else None,
        "detail": flown.detail,
        # The name every simulated number on this page is filed under, sent so the page can label
        # the block once rather than repeating "SIMULATED" per row (ADR-027 §Decision 6).
        "source": FLIGHT_SOURCE,
        "measurements": _simulated(flown),
        "unscored": [
            {
                "name": entry.name,
                "reason": entry.reason.value,
                "detail": entry.detail,
                # The flag decides what a page may tell the golfer to do, and it is never to be
                # inferred from the name: every reason a flight produces has it false, so nothing
                # here may end in "film it again".
                "refilming_helps": entry.spec.refilming_helps,
            }
            for entry in flight_unscored(flown)
        ],
        "launch": _launch(resolved),
        "spin": _spin(resolved),
        "axis": _axis(resolved),
        "path": _path(flown.flight, points),
        "measured": _measured(shot),
        # The two of those the model also produces, paired where the pairing is decided
        # (`flight_measure._COMPARISON_PAIRS`) rather than here or on the page. Empty on a refused
        # flight, and neither row on this corpus is a validation — `reading` is the sentence that
        # says which kind of non-validation each one is, and a surface printing the numbers without
        # it has drawn a check that does not exist.
        "comparison": [row._asdict() for row in compare_to_printed(flown, shot)],
        # A refused flight owes none of them: every one of the four is a sentence about a line
        # that was drawn, and there is no line. What it owes instead is `reason` and `detail`.
        "caveats": list(
            caveats_for(flown.flight, resolved, model or load_flight_model())
            if flown.flight is not None and resolved is not None
            else ()
        ),
    }


def _simulated(flown: FlownShot) -> list[dict]:
    """The six, read through the registry the stored artifact is written from.

    Absent names are dropped rather than sent as null. Two of the six record conditionally and the
    conditions are the subject of `unscored` — `flight_landing_offline_yds` where the curve was
    never drawn, `flight_spin_rpm` where the screen printed the spin — so a null here would be a
    second, quieter way of saying something the payload already says out loud.
    """
    rows = []
    for name, (read, unit, detail) in FLIGHT_MEASUREMENTS.items():
        value = read(flown)
        if value is not None:
            rows.append({"name": name, "value": value, "unit": unit, "detail": detail})
    return rows


def _measured(shot: ShotData) -> list[dict]:
    """What the launch monitor actually printed, in the repo's own vocabulary.

    Here so that M15 P16 can draw the measured landing point beside the simulated one:
    `carry_distance_yds` and `start_line_offline_yds` are that point, and the second of them is
    the number a planar flight's landing offline collapses to — which is the reason the payload
    withholds `flight_landing_offline_yds` rather than sending it twice under two names.
    """
    rows = []
    for name, (read, unit, detail) in SHOT_MEASUREMENTS.items():
        value = read(shot)
        if value is not None:
            rows.append({"name": name, "value": value, "unit": unit, "detail": detail})
    return rows


def _launch(resolved: ShotFlight | None) -> dict | None:
    """The five conditions the ball left on, whichever way each of them was arrived at."""
    if resolved is None or resolved.launch is None:
        return None
    launch = resolved.launch
    return {
        "ball_speed_mph": launch.ball_speed_mph,
        "launch_angle_deg": launch.launch_angle_deg,
        "spin_rpm": launch.spin_rpm,
        "launch_direction_deg": launch.launch_direction_deg,
        # Zero here is a flight drawn planar as often as it is an axis measured at zero, and
        # `axis.spin_axis_deg` is the field that tells the two apart. Sent anyway because it is
        # what was integrated.
        "spin_axis_deg": launch.spin_axis_deg,
    }


def _spin(resolved: ShotFlight | None) -> dict | None:
    """The spin, its provenance, and the cap it sits under — never one without the others.

    `source` is the field that matters: a printed spin is a launch monitor's reading and a solved
    one is *the spin this integrator needs in order to agree with HD Golf's carry*, which is a
    different kind of quantity in the same unit. `cap_rpm` travels with an inferred number because
    `InferredSpin.cap_rpm`'s docstring requires it to — above the cap carry stops responding to
    spin at all, so the cap is what bounds the claim.
    """
    if resolved is None:
        return None
    inferred = resolved.spin
    return {
        "rpm": resolved.spin_rpm,
        "source": resolved.spin_source.value if resolved.spin_source is not None else None,
        "detail": resolved.detail,
        # Present whenever a solve ran, including where it refused: which of the seven cases the
        # printed carry landed on is most of the answer when there is no number.
        "case": inferred.solution.case.value if inferred is not None else None,
        "cap_rpm": inferred.cap_rpm if inferred is not None else None,
        "at_cap": inferred.at_cap if inferred is not None else None,
    }


def _axis(resolved: ShotFlight | None) -> dict | None:
    """How far the ball was tilted, which way it curved, and whether those two agree.

    `sign_disagrees` is a flag and never a correction (ADR-027 §Decision 5): the screen's own
    word and the direction derived from face-to-path contradict each other on one shot on disk,
    and quietly picking one is how ADR-014's sign inversion survived a milestone.
    """
    if resolved is None:
        return None
    axis = resolved.axis
    return {
        "spin_axis_deg": axis.spin_axis_deg,
        "source": axis.source.value if axis.source is not None else None,
        "reason": axis.reason.value if axis.reason is not None else None,
        "detail": axis.detail,
        "curve_direction": axis.curve_direction.value if axis.curve_direction is not None else None,
        "screen_shape": axis.screen_shape.value if axis.screen_shape is not None else None,
        "sign_disagrees": axis.sign_disagrees,
        # The bit that says whether the landing point is a landing point. False draws a straight
        # line, and a straight line's offline is the start line projected out — not where the ball
        # finished.
        "curve_is_drawn": resolved.curve_is_drawn,
    }


def _path(result: FlightResult | None, points: int) -> dict | None:
    """The polyline, plus what a reader has to know about which rows the table was read for.

    `clamped` is per point rather than once for the flight because it is not constant along one:
    the spin ratio climbs as the ball sheds speed faster than it sheds spin, so a flight can start
    inside the published table and leave it. A page that draws the extrapolated part differently
    needs the flag where the extrapolation is.
    """
    if result is None:
        return None
    sampled = result.sample(points)
    return {
        "points": [
            {
                "t_s": p.t_s,
                "x_yds": p.x_yds,
                "y_yds": p.y_yds,
                "z_yds": p.z_yds,
                "speed_mph": p.speed_m_s * _M_S_TO_MPH,
                "spin_rpm": p.spin_rpm,
                "spin_ratio": p.spin_ratio,
                "clamped": p.clamped,
            }
            for p in sampled
        ],
        "sampled": len(sampled),
        "steps": len(result.points),
        "clamped_points": result.clamped_points,
        "fully_clamped": result.fully_clamped,
        "spin_ratio_min": result.spin_ratio_min,
        "spin_ratio_max": result.spin_ratio_max,
    }
