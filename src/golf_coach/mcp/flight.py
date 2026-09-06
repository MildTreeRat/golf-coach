"""The flight of the shot that was just hit, for a reader with no canvas. [M15 P17]

The eleventh tool, and the fourth surface over ADR-027's model after the CLI, the route and the
page. What is different here is that nothing can be *shown*: a model cannot be handed a polyline
and asked to look at it, so this sends the six numbers, where each of them came from and every
sentence they have to be read with — and no path at all. `DEFAULT_PATH_POINTS` has no counterpart
in this module for that reason.

**It re-flies rather than reading `analysis.json`, for M15 P14's reason and one of its own.** The
route re-flies because the artifact stores no path and a path is what a viewer draws; this has no
path to want. It re-flies because two of a flight's inputs — the club's declared loft and the
golfer's handedness — live in *editable* artifacts, so the flight recorded when the engine ran and
the flight the same shot has today can be two different answers. P14 found that, and found that
nothing on disk can see it. `differs_from_recorded` is that difference made visible: the stored
numbers read back through `query.get_swing` and set beside the flown ones, with the sentence that
says which of the two causes is in play. It is the first surface in this repo that can say a bag
edit has moved a stored result.

**The resolution is the route's, and the direction of it is this phase's finding.** A shot borrows
two things from the swing it was hit with — the club's declared loft and the golfer's handedness —
and `storage/flight_inputs.loft_for_club` is the half of M15 P10's join that exists for a caller
holding the manifest, which this is. What it deliberately does *not* use is `read_flight_inputs`:
that answers "which swing was this shot hit on" and names one survivor per photo, which is right
for counting a shot once and wrong from this side. See `_shot_for`.

**A refusal is an answer, exactly as it is a 200 on the route.** Most shots on disk cannot be
flown, and a tool that reported those as a miss would teach a model that this repo is broken.
`NotFound` is for a swing id nothing carries. A swing with no shot screen, a screen nobody has
read, and a carry the model cannot reach are three different sentences that all arrive with
`flew: false` — and only the third carries `unscored`, because that is the one the engine recorded
too.

**No `anthropic` and no MCP SDK here**, the same as `query.py`: this is the half that reads, and
`server.py` and `runner_tools.py` are the two adapters over it.
"""

from __future__ import annotations

from pathlib import Path

from pydantic import BaseModel, Field

from golf_coach.analysis.benchmarks import load_flight_model
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
from golf_coach.contracts.shot import ShotData
from golf_coach.contracts.swing import ANALYSIS_VERSION
from golf_coach.launch_monitor.source import ShotDataSource
from golf_coach.mcp import query
from golf_coach.mcp.query import NotFound, SimulatedView, missing_swing
from golf_coach.storage.bundle_store import SwingBundleStore
from golf_coach.storage.flight_inputs import loft_for_club
from golf_coach.storage.golfer_store import GolferStore
from golf_coach.storage.manifest import Role


class FlightRefusalView(BaseModel):
    """One thing this flight could not produce, with the case it fell into.

    Deliberately not `query.UnscoredView`. That shape drops `detail`, which is right for a
    checkpoint — "no confident ear frames in the impact window" is provenance for a derivation
    step — and wrong here, where the detail *is* the finding: `127.0 yd, case two_branches` and
    `121.8 yd, case above_peak` are two entirely different facts about a shot, and both collapse
    onto `carry_unreachable` in the reason.
    """

    name: str
    reason: str = Field(
        description="Stable token from `contracts.unscored.UnscoredReason` — branch on this."
    )
    why: str = Field(description="What went wrong, as a clause that can be read to a golfer.")
    detail: str = Field(
        description=(
            "Which case the spin solve landed on, and the numbers behind it. This is what tells "
            "two refusals under one reason apart."
        )
    )
    refilming_helps: bool = Field(
        description=(
            "Always false on a flight. Nothing here is a capture problem — the video was never "
            "an input to the ball flight — so never answer one of these by telling the golfer to "
            "film the swing again."
        )
    )


class LaunchView(BaseModel):
    """The five conditions the ball left on, whichever way each of them was arrived at."""

    ball_speed_mph: float
    launch_angle_deg: float
    spin_rpm: float
    launch_direction_deg: float
    spin_axis_deg: float = Field(
        description=(
            "Zero here is a flight drawn flat as often as it is an axis measured at zero — "
            "`axis.spin_axis_deg` is the field that tells those apart."
        )
    )


class SpinView(BaseModel):
    """The spin, its provenance and the cap it sits under — never one of them without the others.

    `source` is the field that matters, and it is ADR-027 §Decision 3 in one word: a `measured`
    spin is a launch monitor's reading, an `inferred` one is *the spin this integrator needs in
    order to agree with the carry HD Golf printed*. Same unit, different kind of quantity.
    """

    rpm: float | None
    source: str | None = Field(description="`measured` (off the screen) or `inferred` (solved).")
    case: str | None = Field(
        default=None,
        description=(
            "Which of `analysis.spin_solve.SpinSolveCase`'s shapes the printed carry landed on. "
            "Present whenever a solve ran, including when it refused — that is most of the "
            "answer when there is no number."
        ),
    )
    cap_rpm: float | None = Field(
        default=None,
        description=(
            "Above this the carry stops responding to spin at all, so the solve cannot see past "
            "it. It bounds any claim made about an inferred number."
        ),
    )
    at_cap: bool | None = None
    detail: str = ""


class AxisView(BaseModel):
    """How far the ball was tilted, which way it curved, and whether those two agree."""

    spin_axis_deg: float | None = Field(
        description=(
            "None means the flight was drawn in the vertical plane. It does not mean the shot "
            "was straight."
        )
    )
    source: str | None = None
    curve_direction: str | None = Field(
        default=None,
        description=(
            "Which way it bent, golfer-relative. Resolved far more often than the magnitude is, "
            "so a flat-drawn flight can still be described as the fade it was."
        ),
    )
    screen_shape: str | None = Field(
        default=None, description="What the `Shot Type` tile on the screen said."
    )
    sign_disagrees: bool = Field(
        description=(
            "True when the direction derived from face-to-path contradicts the word on the "
            "screen. A warning and never a correction (ADR-027 §Decision 5) — say that the two "
            "readings disagree rather than picking one."
        )
    )
    curve_is_drawn: bool = Field(
        description=(
            "False when the axis was refused. `flight_landing_offline_yds` is withheld in that "
            "case, because a flat flight's landing offline is the start line projected out and "
            "would be one number under two names."
        )
    )
    why: str = Field(default="", description="Why the axis is missing, when it is.")


class ComparisonView(BaseModel):
    """One simulated number beside the printed one it is about — and whether that is a check.

    **Neither of the two pairs this corpus can produce is a validation, for two different
    reasons** (M15 P16), so `comparable` and `reading` are not decoration: the numbers alone read
    as a check that does not exist. Where the spin was solved, the printed carry was the solve's
    own input and the flight reproduces it to the root search's tolerance. The offline pair is
    where the ball *started* against where the model has it finishing, and that gap is the curve
    rather than an error.
    """

    simulated_name: str
    measured_name: str
    quantity: str
    simulated: float
    measured: float
    difference: float
    comparable: bool = Field(
        description="False means the difference cannot be read as agreement or disagreement."
    )
    reading: str = Field(
        description="What the difference is, in words. Never quote the numbers without it."
    )


class RecordedGap(BaseModel):
    """One of the six where the stored artifact and today's flight do not say the same thing."""

    name: str
    recorded: float | None = Field(
        description="What `analysis.json` holds. None means the artifact has no such row."
    )
    flown: float | None = Field(
        description="What the same shot flies now. None means it is refused now."
    )
    difference: float | None = None


class FlightView(BaseModel):
    """One stored swing's shot, flown — or every reason it was not, in the same shape.

    Every number under `simulated` is a model output. Nothing in this payload was measured off a
    ball except what `comparison` names as `measured`, and `source` sits on the view so that can
    be said once rather than per row.
    """

    session_id: str
    swing_id: str
    shot_id: str | None = None
    club: str | None = None
    flew: bool = Field(
        description=(
            "False is a finding about the shot rather than a fault: most shots on file cannot be "
            "flown, because the screen printed no spin and no spin reaches the carry beside it."
        )
    )
    source: str = Field(
        default=FLIGHT_SOURCE,
        description=(
            "The provenance every number in `simulated` carries. It is versioned with the "
            "coefficient table the model evaluates, so a re-sourced table gets a new name rather "
            "than quietly changing what this one meant."
        ),
    )
    simulated: list[SimulatedView] = Field(
        default_factory=list,
        description=(
            "The flight, as the numbers the engine records. SIMULATED — say so whenever you "
            "quote one. None of them was measured and no launch monitor printed them."
        ),
    )
    reason: str | None = Field(
        default=None, description="Why there is no flight. None when there is one."
    )
    detail: str = ""
    unscored: list[FlightRefusalView] = Field(
        default_factory=list,
        description=(
            "What this shot's `analysis.json` records under the same refusal. A refused flight is "
            "one entry and never six, and it is not a checkpoint that failed: no score moved, "
            "because a flight was never in `overall_score` to be excluded from it."
        ),
    )
    launch: LaunchView | None = None
    spin: SpinView | None = None
    axis: AxisView | None = None
    comparison: list[ComparisonView] = Field(
        default_factory=list,
        description=(
            "The printed numbers this flight has a counterpart for. The launch monitor's own "
            "figures are `get_shot_by_id`'s answer; what is here is the pairing, which is the "
            "part that is not obvious."
        ),
    )
    caveats: list[str] = Field(
        default_factory=list,
        description=(
            "Everything this flight has to be read with, in the order a reader needs them. Not "
            "optional and not a footnote: the first of them says the model was run outside the "
            "range its coefficients were published for, which is true of every shot on file."
        ),
    )
    differs_from_recorded: list[RecordedGap] = Field(
        default_factory=list,
        description=(
            "Where this flight and the one stored in `analysis.json` disagree. Normally empty; "
            "when it is not, `recorded_reading` says why, and the answer is never that the "
            "golfer swung differently."
        ),
    )
    recorded_reading: str = ""


def missing_swing_flight(session_id: str, swing_id: str) -> NotFound:
    """A swing id nothing on disk carries — the one miss this tool has.

    `query.missing_swing`'s own sentence, and not a second wording of it: a wrong id has to read
    identically whichever tool it was typed into, which is the reason the `missing_*` helpers live
    in `query.py` at all.
    """
    return missing_swing(session_id, swing_id)


#: Said instead of a flight when the swing arrived without a shot screen, and when the screen it
#: did arrive with has not been read. Two different repairs, and neither of them is a re-film:
#: ball flight is simulated from what the launch monitor printed, and the video is not an input to
#: any of it. The route says the same two things as its two 404s (`api/app.py::swing_flight`);
#: here they are answers, because a tool with no status code has to carry the difference in words.
NO_SHOT_SCREEN = (
    "This swing has no shot-screen photo, so there are no launch conditions to fly. Ball flight "
    "is simulated from what the launch monitor printed, never from the video."
)
UNREAD_SHOT_SCREEN = (
    "This swing's shot screen has not been read yet, so its launch conditions are not on file. "
    "Analyzing the swing reads it, or it can be imported with scripts/import_shot_screens.py."
)


def flight_for_swing(
    sessions_dir: Path,
    shot_source: ShotDataSource,
    session_id: str,
    swing_id: str,
    *,
    golfers_dir: Path | None = None,
) -> FlightView | None:
    """Fly the shot this swing was hit with, or say which of the silences applies.

    `None` is returned for one thing only — no such swing — and `missing_swing_flight` is the
    sentence that goes with it. Everything else is a `FlightView`, including every refusal.

    `golfers_dir` is optional here rather than gating the tool, unlike the career and club tools:
    a shot whose screen printed its spin needs neither a bag nor a handedness, which is exactly
    the two 2026-08-10 reference shots. Without a registry the loft resolves to `NO_BAG_ENTRY` and
    the axis refuses, which is a flat flight rather than no flight.
    """
    manifest = SwingBundleStore(sessions_dir).get_swing(session_id, swing_id)
    if manifest is None:
        return None

    view = FlightView(
        session_id=session_id,
        swing_id=swing_id,
        club=manifest.club.value if manifest.club is not None else None,
        flew=False,
    )

    role_file = manifest.roles.get(Role.SHOT_SCREEN)
    if role_file is None:
        view.detail = NO_SHOT_SCREEN
        return view

    shot = _shot_for(shot_source, role_file.content_sha256)
    if shot is None:
        view.detail = UNREAD_SHOT_SCREEN
        return view

    # The two things a shot borrows from the swing it was hit with (M15 P10), resolved from the
    # manifest already in hand rather than through `read_flight_inputs`'s photo-sha256 join — see
    # `_shot_for` for why that direction is the wrong one here. This is `api/app.py::swing_flight`'s
    # resolution, and `loft_for_club` exists because the route needed exactly this half of it.
    loft_deg, _gap = loft_for_club(manifest.player_id, manifest.club, golfers_dir=golfers_dir)
    golfer = (
        GolferStore(golfers_dir).get(manifest.player_id)
        if golfers_dir is not None and manifest.player_id
        else None
    )
    flown = fly_shot(
        shot,
        loft_deg=loft_deg,
        handedness=golfer.handedness if golfer is not None else None,
    )
    _fill(view, shot, flown)
    _compare_to_recorded(view, sessions_dir, session_id, swing_id)
    return view


def _shot_for(shot_source: ShotDataSource, photo_sha256: str) -> ShotData | None:
    """The parse filed under this swing's shot photo, or None if nothing has read it.

    ⚠️ **Not `read_flight_inputs`, and that is M15 P17's finding.** That function answers "which
    swing was this shot hit on", so where one photo is attached to several swings it names a single
    survivor — earliest arrival wins, which is the right rule for counting a shot once. Asked from
    this side the same rule is wrong: the corpus has one screen photo attached to three swings (an
    upload path tested three times over one physical swing), and the join names one of them, so the
    other two would be told their screen had never been read while `get_swing` was returning the
    recorded flight for them in the same conversation. The direction matters, so this resolves the
    photo the manifest names instead.

    A linear scan of `stream()`, for `query.get_shot`'s reason one field over: the port offers no
    lookup, and the screen store is keyed by image hash — which is the key here, so this is that
    index read the only way the port exposes it. No OCR runs; an unread photo is a `None`, which is
    the answer and not an error.
    """
    for shot in shot_source.stream():
        provenance = shot.provenance
        if provenance is not None and provenance.image_sha256 == photo_sha256:
            return shot
    return None


def _fill(view: FlightView, shot: ShotData, flown: FlownShot) -> None:
    """Fill one view from a flight that ran, refused, or raised on its launch conditions."""
    resolved = flown.resolved
    view.shot_id = shot.shot_id
    view.flew = flown.flew
    view.reason = flown.reason.value if flown.reason is not None else None
    view.detail = flown.detail
    view.simulated = _simulated(flown)
    view.unscored = [
        FlightRefusalView(
            name=entry.name,
            reason=entry.reason.value,
            why=entry.spec.summary,
            detail=entry.detail,
            refilming_helps=entry.spec.refilming_helps,
        )
        for entry in flight_unscored(flown)
    ]
    view.launch = _launch(resolved)
    view.spin = _spin(resolved)
    view.axis = _axis(resolved)
    view.comparison = [ComparisonView(**row._asdict()) for row in compare_to_printed(flown, shot)]
    # A refused flight owes none of the caveats: every one of the four is a sentence about a line
    # that was drawn, and there is no line. What it owes instead is `reason` and `detail`.
    if flown.flight is not None and resolved is not None:
        view.caveats = list(caveats_for(flown.flight, resolved, load_flight_model()))


def _simulated(flown: FlownShot) -> list[SimulatedView]:
    """The six, read through the registry the stored artifact is written from.

    Absent names are dropped rather than sent as null, which is `api/flight_view._simulated`'s
    rule for the identical reason: two of the six record conditionally and the conditions are the
    subject of `unscored`, so a null here would be a second, quieter way of saying it.
    """
    rows = []
    for name, (read, unit, detail) in FLIGHT_MEASUREMENTS.items():
        value = read(flown)
        if value is not None:
            rows.append(
                SimulatedView(
                    name=name, value=value, unit=unit, source=FLIGHT_SOURCE, detail=detail
                )
            )
    return rows


def _launch(resolved: ShotFlight | None) -> LaunchView | None:
    if resolved is None or resolved.launch is None:
        return None
    launch = resolved.launch
    return LaunchView(
        ball_speed_mph=launch.ball_speed_mph,
        launch_angle_deg=launch.launch_angle_deg,
        spin_rpm=launch.spin_rpm,
        launch_direction_deg=launch.launch_direction_deg,
        spin_axis_deg=launch.spin_axis_deg,
    )


def _spin(resolved: ShotFlight | None) -> SpinView | None:
    if resolved is None:
        return None
    inferred = resolved.spin
    return SpinView(
        rpm=resolved.spin_rpm,
        source=resolved.spin_source.value if resolved.spin_source is not None else None,
        case=inferred.solution.case.value if inferred is not None else None,
        cap_rpm=inferred.cap_rpm if inferred is not None else None,
        at_cap=inferred.at_cap if inferred is not None else None,
        detail=resolved.detail,
    )


def _axis(resolved: ShotFlight | None) -> AxisView | None:
    if resolved is None:
        return None
    axis = resolved.axis
    return AxisView(
        spin_axis_deg=axis.spin_axis_deg,
        source=axis.source.value if axis.source is not None else None,
        curve_direction=axis.curve_direction.value if axis.curve_direction is not None else None,
        screen_shape=axis.screen_shape.value if axis.screen_shape is not None else None,
        sign_disagrees=axis.sign_disagrees,
        curve_is_drawn=resolved.curve_is_drawn,
        why=axis.detail if axis.spin_axis_deg is None else "",
    )


#: Two causes, and a reader has to be told which one is in play before being told a number moved.
#: A stale artifact is repaired by re-analysing the swing; a current one that disagrees is the
#: M15 P14 seam — an input that lives in an editable artifact was edited after the engine ran, and
#: no version changed because none of the *uploads* did.
_STALE_READING = (
    "This artifact was written by an older engine, so the difference is the engine and not the "
    "shot. Re-analyze the swing to bring the recorded flight up to date."
)
_EDITED_READING = (
    "This artifact is current, so the difference is an input that lives outside it: the club's "
    "declared loft or the golfer's handedness was edited after the swing was analyzed. Nothing "
    "about the swing changed, and re-analyzing it is what makes the stored numbers agree again."
)


def _compare_to_recorded(
    view: FlightView, sessions_dir: Path, session_id: str, swing_id: str
) -> None:
    """Set today's flight beside the one `analysis.json` holds, and say why they can differ.

    ⚠️ **This is the seam M15 P14 found and could not see.** Two of a flight's inputs live in
    editable artifacts — the bag's declared loft and the golfer's handedness — so declaring a 3
    wood's loft this afternoon changes what a stored shot flies without changing anything the
    staleness check can detect: `is_outdated` compares engine versions and `AnalysisState.inputs`
    hashes the *uploads*. Every other surface re-flies and shows one answer. This one reads both.

    The stored numbers come back through `query.get_swing` rather than through a second reader of
    `analysis.json`: that function already owns the tolerant read and, since P17, the split that
    keeps a simulated number out of `measurements`. Reading the artifact again here would be a
    second thing to keep in step with the artifact's shape.
    """
    stored = query.get_swing(sessions_dir, session_id, swing_id)
    if stored is None:
        return
    recorded = {row.name: row.value for row in stored.simulated}
    flown = {row.name: row.value for row in view.simulated}

    gaps: list[RecordedGap] = []
    for name in FLIGHT_MEASUREMENTS:
        here, there = flown.get(name), recorded.get(name)
        if here is None and there is None:
            continue
        # Rounded to what the artifact stores: `engine.py` writes four decimals and this flies at
        # full precision, so an exact comparison would report every number as moved on every swing.
        if here is not None and there is not None and round(here, 4) == round(there, 4):
            continue
        gaps.append(
            RecordedGap(
                name=name,
                recorded=there,
                flown=here,
                difference=(
                    round(here - there, 4) if here is not None and there is not None else None
                ),
            )
        )
    if not gaps:
        return
    view.differs_from_recorded = gaps
    view.recorded_reading = (
        _EDITED_READING if stored.analysis_version == ANALYSIS_VERSION else _STALE_READING
    )
