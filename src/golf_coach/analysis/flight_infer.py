"""The two launch conditions the screen did not print, and what it takes to stand in. [M15 P9]

`analysis/flight.py` flies a ball from five numbers. The HD Golf screen prints three of them on
every shot, the fourth (spin) on two of thirteen, and the fifth (spin axis) on the same two — and
ADR-027 §Decisions 3 and 5 are the two answers to that. This module is both of them, and it is the
first place in M15 where a refusal is a *result* rather than an exception:

- **spin** comes from `spin_solve.solve_spin_from_carry`, which returns up to two candidates, and
  §Decision 3 gives loft the job of choosing between them. That is loft's entire involvement in
  ball flight — an involvement in the *inference*, not in the flight, which is why a club bent 2°
  strong changes the inferred spin and not the path the ball takes.
- **the spin axis** comes from the measured `spin_axis` where the screen printed one, and
  otherwise from `Shot Type` and face-to-path.

Everything that cannot be resolved comes back as an `UnscoredReason` from `contracts/unscored.py`,
all four of them with `refilming_helps` false, because a launch monitor withholding a number is not
a camera problem and a golfer must never be sent back to the bay over one.

## What this module found, and it is mostly about what is *not* on disk

**Every refusal below fires on the whole corpus today.** Not one of the thirteen stored shots
carries a `club`, so no loft resolves, so the branch is never chosen; and eleven of the thirteen
carry no `spin_axis`, so the axis falls to a fallback that this phase found cannot produce one. The
code is built and silent, in the same way M9's per-club answers are built and silent, and the same
bay session unlocks both — a club tag on the upload page is what turns the spin half on.

**§Decision 5's second branch resolves the sign and not the magnitude, and that is a correction to
the ADR.** Face-to-path says *which way* the ball curved and agrees with the `Shot Type` tile on
eleven of the twelve shots that carry both. It cannot say *how far* the axis was tilted, and the
two shots where both quantities are on disk are what rule out calibrating it: 10.9° of face-to-path
against a 2.5° axis on one, 13.2° against 9.3° on the other — the same quantity, three times the
tilt per degree. The direction of that disagreement is the part that matters. The second shot spins
*more* (8,100 rpm against 5,991), and more backspin under the same sidespin tilts the axis *less*,
so the two readings are not merely scattered — they are ordered the wrong way round for any
monotone relation between face-to-path and axis tilt. A magnitude fitted to `n = 2` pointing the
wrong way is an invented number, so the fallback reports the direction, withholds the axis, and the
flight is drawn in the vertical plane with `landing_offline` unscored — which is what §Decision 5's
*third* branch already prescribed.

## Why the loft prior needs no spin table

The branch question looks like it needs a loft-to-spin model and does not. It is one bit: **does
this club spin the ball faster than the peak-carry spin?** — and the peak is measured per shot by
`carry_window`, not assumed. On the three two-branch shots on disk it sits at 2,402, 2,555 and
2,781 rpm, which is driver spin. Every club with more loft than a driver spins the ball harder than
that, so for every club this repo has ever seen the answer is the falling branch; only a driver
puts its own spin near enough the peak for the two candidates to straddle it, and there the prior
refuses rather than guessing.

So the one constant here is a loft, placed in the gap between a driver and a fairway wood, and the
answer is invariant to where in that gap it sits. `tests/analysis/test_flight_infer.py` pins that
invariance, which is the honest form of "this number is coarse and the margin is enormous".

## What the number is not, said once more where it is produced

`spin_solve.py` says it and ADR-027 §Decision 3 says it first: an inferred spin is *"the spin our
integrator needs in order to agree with the simulator"*. This module is where that stops being a
docstring and becomes a value with a label attached, so it reports two things beside every number
it produces and neither is optional:

- **the cap.** Every inferred spin is bounded above by `CarryWindow.high_plateau_min_rpm`
  (5,103-5,734 rpm across the shots on disk), because above it carry stops depending on spin at
  all. A `flight_spin_rpm` shown without the cap reads as a low-spin diagnosis of the golfer.
- **the gate, and not merely its percentage.** `GATE_AGREEMENT_FRACTION` is ±2.59%, and quoting it
  as "accurate to 2.6%" is the reading M15 P4 exists to prevent: the model *ranks the two
  validation shots backwards*, so that figure is the size of what the model cannot see.
  `gate_ordering` measures the inversion rather than restating it.

`honest_test()` is the third: it re-solves the two shots whose real spin is on disk as if it never
had been, and reports one refusal and one answer 46.8% low.

Stdlib + `contracts` only, and no numpy or scipy (ADR-008, ADR-027 §Decision 1); the pin is in
`tests/api/test_pipeline_imports.py`.
"""

from __future__ import annotations

from enum import StrEnum
from typing import NamedTuple

from golf_coach.analysis.benchmarks.flight_model import FlightModel
from golf_coach.analysis.flight import (
    DEFAULT_STEP_S,
    VALIDATION_SHOTS,
    LaunchConditions,
    gate_comparisons,
    simulate_flight,
)
from golf_coach.analysis.shot_measure import (
    measure_ball_speed,
    measure_carry_distance,
    measure_face_to_path,
    measure_launch_angle,
    measure_start_line,
    normalize_shot_shape,
)
from golf_coach.analysis.spin_solve import (
    CarryWindow,
    SpinSolution,
    SpinSolveCase,
    UnspunLaunch,
    solve_spin_from_carry,
)
from golf_coach.contracts.golfer import Handedness
from golf_coach.contracts.intent import TargetShape
from golf_coach.contracts.shot import ShotData
from golf_coach.contracts.unscored import UnscoredReason

#: The loft above which a club spins the ball faster than any peak-carry spin this model produces,
#: so the falling branch is the answer and the rising one is not.
#:
#: **Placed in a gap in the equipment rather than fitted to anything.** `carry_window` measures the
#: peak per shot, and on every two-branch shot on disk it lands at 2,402-2,781 rpm — a driver's own
#: spin. A driver is therefore the only club whose two candidates genuinely straddle it; a fairway
#: wood and everything above spins harder than the peak on any published club average, so the
#: falling branch is theirs. Drivers are built to about 12° and fairway woods start at about 15°,
#: and this sits between, so no real club is near enough the threshold for its exact value to
#: matter. `test_flight_infer.py` pins that: the branch chosen is the same anywhere in 12.0-15.0.
#:
#: It is also, today, **never exercised on real data** — no shot on disk carries a club, so no loft
#: reaches this comparison. Which is the honest status of the whole prior: argued, pinned against
#: synthetic lofts, and unproven on a shot anyone hit.
LOFT_FLOOR_FOR_THE_FALLING_BRANCH_DEG = 13.0


class SpinAxisSource(StrEnum):
    """Where a resolved spin axis came from. ADR-027 §Decision 5's order, as far as it reaches.

    There is no `FACE_TO_PATH` member, and its absence is this phase's finding rather than an
    omission — see the module docstring. Face-to-path resolves the *direction* of the curve, which
    `InferredSpinAxis.curve_direction` reports whether or not an axis came out of it, and a
    direction is not an axis.
    """

    #: The `spin_axis` tile, flipped out of the contract's golfer-relative sign into the
    #: integrator's geometric one. The only source that has ever produced a number.
    MEASURED = "measured"


class InferredSpin(NamedTuple):
    """The spin the flight will be drawn with, or the reason there is none — and the cap either way.

    `solution` is always present, including on a refusal, because *which* of
    `spin_solve.SpinSolveCase`'s seven shapes the carry landed on is most of the answer: a printed
    carry two yards above the peak and one sitting exactly on the high plateau both come back
    without a number and are entirely different findings about the shot. `reason` collapses those
    seven onto the four things a reader can *do*, and `detail` keeps the case.
    """

    spin_rpm: float | None
    #: `None` exactly when `spin_rpm` is not. One of `contracts.unscored.INFERENCE_REASONS`, or
    #: `NO_HANDEDNESS` — never a reason whose `refilming_helps` is true.
    reason: UnscoredReason | None
    detail: str
    solution: SpinSolution

    @property
    def window(self) -> CarryWindow:
        return self.solution.window

    @property
    def cap_rpm(self) -> float:
        """The most spin this carry could ever have named, and **it must be reported beside the
        number, not instead of it**.

        Above this the whole flight launches past the coefficient table's last row, the held end
        row makes carry independent of spin, and the inverse problem has nothing left to invert.
        Both shots whose real spin is known sit above their own cap — 5,991 and 8,100 rpm against
        5,154 and 5,143 — which is why one of them is refused and the other comes back 46.8% low.
        """
        return self.window.high_plateau_min_rpm

    @property
    def at_cap(self) -> bool:
        """Whether the number produced is the cap itself, to within the root search's tolerance.

        Never true on this corpus, and worth surfacing rather than assuming: an answer sitting on
        the cap is the solve saying "at least this much and I cannot see further", which is a
        different sentence from an answer in the interior.
        """
        return self.spin_rpm is not None and self.spin_rpm >= self.cap_rpm - 1.0


class InferredSpinAxis(NamedTuple):
    """How far the spin axis was tilted, or the reason it is unknown — plus which way it curved.

    The two halves are separate on purpose. `spin_axis_deg` is refused far more often than
    `curve_direction` is, and a consumer that has to draw a straight flight still wants to be able
    to say *"this was a fade and the model is not drawing the curve"* rather than saying nothing.
    """

    #: Geometric, in `LaunchConditions.spin_axis_deg`'s sign — **positive curves the ball right** —
    #: which is the contract's `+ = fade` only for a right-handed golfer. That flip is what
    #: `LaunchConditions` names M15 P9 as the owner of, and it is the reason a missing handedness
    #: refuses here rather than defaulting: `contracts/dispersion.py` is the precedent, where a
    #: camera-relative sign meeting a mixed-handedness corpus read every left-handed golfer as a
    #: gross fault.
    spin_axis_deg: float | None
    source: SpinAxisSource | None
    reason: UnscoredReason | None
    detail: str
    #: Which way the ball curved, golfer-relative, from whichever evidence was available — the
    #: measured axis where there is one, else the sign of face-to-path. Independent of whether an
    #: axis magnitude resolved.
    curve_direction: TargetShape | None
    #: What the `Shot Type` tile said, through `shot_measure.normalize_shot_shape`. The
    #: cross-check, and the reason ADR-027 §Decision 5 could resolve a sign ADR-014 could not:
    #: no *numeric* tile carries the direction and a *text* one does.
    screen_shape: TargetShape | None

    @property
    def sign_disagrees(self) -> bool:
        """Whether the derived direction contradicts the word on the screen.

        **A warning and never a silent overwrite** (ADR-027 §Decision 5). A disagreement is
        information about the parse, and resolving it quietly is how ADR-014's original sign
        inversion survived as long as it did. `None` on either side is not a disagreement — there
        is simply nothing to compare.
        """
        if self.curve_direction is None or self.screen_shape is None:
            return False
        return self.curve_direction is not self.screen_shape


def infer_spin(
    launch: UnspunLaunch,
    target_carry_yds: float,
    loft_deg: float | None,
    *,
    model: FlightModel | None = None,
    step_s: float = DEFAULT_STEP_S,
    window: CarryWindow | None = None,
) -> InferredSpin:
    """Solve the carry for a spin and let the club's loft choose the branch.

    `target_carry_yds` is the carry the launch monitor printed. It is a required positional rather
    than something read off a `ShotData` here, because a shot with no printed carry has nothing to
    solve and that absence belongs to the caller — M15 P10 is what reads real shots.

    Pass `window` when the caller already measured one; it costs about forty flights and both the
    solve and any report of the cap want the same one.
    """
    solution = solve_spin_from_carry(
        launch, target_carry_yds, model=model, step_s=step_s, window=window
    )
    case = solution.case
    where = f"{target_carry_yds:.1f} yd, case {case.value}"

    if case in (SpinSolveCase.ABOVE_PEAK, SpinSolveCase.BELOW_FLOOR):
        edge = "peak" if case is SpinSolveCase.ABOVE_PEAK else "floor"
        edge_yds = solution.window.peak_yds if edge == "peak" else solution.window.floor_yds
        return InferredSpin(
            None,
            UnscoredReason.CARRY_UNREACHABLE,
            f"{where}: {edge_yds:.2f} yd is the {edge} of what these launch conditions can fly",
            solution,
        )

    if case in (SpinSolveCase.ON_LOW_PLATEAU, SpinSolveCase.ON_HIGH_PLATEAU):
        # Infinitely many spins fly exactly this carry. Choosing a representative out of a flat
        # interval is the one invention `spin_solve` is written not to make (R7), and the loft
        # prior cannot rescue it either — a prior narrows a set of two, not a continuum.
        end = "low" if case is SpinSolveCase.ON_LOW_PLATEAU else "high"
        return InferredSpin(
            None,
            UnscoredReason.SPIN_NOT_RECOVERABLE,
            f"{where}: every spin on the {end} plateau flies this carry, so none of them "
            "is the answer",
            solution,
        )

    if case is SpinSolveCase.AT_PEAK:
        # One spin, on a knife edge, and no branch to choose — so this is the one answer loft is
        # not needed for. It has never occurred on a real shot.
        return InferredSpin(solution.window.peak_rpm, None, f"{where}: the peak itself", solution)

    if case is SpinSolveCase.BETWEEN_PLATEAUS:
        return _between_plateaus(solution, loft_deg, where)

    return _two_branches(solution, loft_deg, where)


def _between_plateaus(
    solution: SpinSolution, loft_deg: float | None, where: str
) -> InferredSpin:
    """The unique answer, refused — and **argued from where it sits, not from the case's name**.

    ADR-027's 2026-09-05c addendum asked for a blanket refusal here on the ground that the band
    lies at spins no 7 iron produces, and its 2026-09-05f successor found a shot where it does not:
    on `2026-08-10-1` the band runs 1,129-1,538 rpm and on `2026-08-23-2` it runs 1,323-4,572 with
    the answer at 2,307, an ordinary number. So the argument is made against the peak instead.

    A unique answer here is on the *rising* branch by construction — the falling branch never comes
    down this far, which is what made it unique — so it is below `peak_rpm`. The loft prior says a
    club above the loft floor spins *above* `peak_rpm`. The two cannot both be right, and the prior
    is the one built on a club rather than on a printed carry, so the answer is refused. It comes
    out as a refusal in every case, but for a stated reason that a driver, or a differently-shaped
    curve, would not satisfy.
    """
    answer = solution.rising_rpm
    peak_rpm = solution.window.peak_rpm
    if loft_deg is None:
        return InferredSpin(
            None,
            UnscoredReason.NO_CLUB_LOFT,
            f"{where}: one spin flies this carry, and no loft is on record to say whether "
            f"{'it' if answer is None else f'{answer:.0f} rpm'} is a spin this club makes",
            solution,
        )
    if loft_deg >= LOFT_FLOOR_FOR_THE_FALLING_BRANCH_DEG and answer is not None:
        return InferredSpin(
            None,
            UnscoredReason.SPIN_NOT_RECOVERABLE,
            f"{where}: the only spin that flies it is {answer:.0f} rpm, below the {peak_rpm:.0f} "
            f"rpm peak, and a {loft_deg:.1f} deg club spins above the peak",
            solution,
        )
    return InferredSpin(
        None,
        UnscoredReason.SPIN_NOT_RECOVERABLE,
        f"{where}: a {loft_deg:.1f} deg club spins about as fast as the {peak_rpm:.0f} rpm peak, "
        "so nothing here says which side of it this shot was on",
        solution,
    )


def _two_branches(solution: SpinSolution, loft_deg: float | None, where: str) -> InferredSpin:
    """Two spins fly the carry, and this is the only place in M15 where loft does anything.

    The choice is a single comparison against the peak, and the module docstring argues why that
    needs no loft-to-spin model: the peak is measured, it lands at driver spin, and every club with
    more loft than a driver beats it.
    """
    rising, falling = solution.rising_rpm, solution.falling_rpm
    peak_rpm = solution.window.peak_rpm
    if rising is None or falling is None:  # pragma: no cover - the case is defined by both existing
        raise RuntimeError(f"{solution.case} without two branches: {solution}")

    if loft_deg is None:
        return InferredSpin(
            None,
            UnscoredReason.NO_CLUB_LOFT,
            f"{where}: {rising:.0f} and {falling:.0f} rpm both fly this carry and no loft is on "
            "record to choose between them",
            solution,
        )
    if loft_deg < LOFT_FLOOR_FOR_THE_FALLING_BRANCH_DEG:
        # A driver's own spin sits at the peak, so its two candidates straddle it and the prior has
        # nothing to say. Refusing is the whole of ADR-010 §2 applied to an inference.
        return InferredSpin(
            None,
            UnscoredReason.SPIN_NOT_RECOVERABLE,
            f"{where}: a {loft_deg:.1f} deg club spins about as fast as the {peak_rpm:.0f} rpm "
            f"peak, so neither {rising:.0f} nor {falling:.0f} rpm is the one it was",
            solution,
        )
    return InferredSpin(
        falling,
        None,
        f"{where}: the falling branch, because a {loft_deg:.1f} deg club spins above the "
        f"{peak_rpm:.0f} rpm peak; {rising:.0f} rpm is the branch not taken",
        solution,
    )


def infer_spin_axis(shot: ShotData, handedness: Handedness | None) -> InferredSpinAxis:
    """Resolve the spin axis, and the direction of the curve, in ADR-027 §Decision 5's order.

    The measured `spin_axis` first, flipped into the integrator's geometric sign; then face-to-path,
    which resolves the direction and — this phase's finding — not the magnitude; then nothing.

    `handedness` is `Golfer.handedness` and is required rather than defaulted, because the flip it
    controls is exactly the one ADR-014's addendum got wrong for a milestone.
    """
    screen_shape = normalize_shot_shape(shot)
    face_to_path = measure_face_to_path(shot)
    direction = _direction_of(shot.spin_axis, face_to_path)

    if shot.spin_axis is None:
        detail = (
            f"no spin axis printed; face-to-path {face_to_path:+.1f} deg gives the direction and "
            "no magnitude"
            if face_to_path is not None
            else "no spin axis printed, and no face angle to take a direction from either"
        )
        return InferredSpinAxis(
            None, None, UnscoredReason.SPIN_AXIS_UNRESOLVED, detail, direction, screen_shape
        )

    if handedness is None:
        return InferredSpinAxis(
            None,
            None,
            UnscoredReason.NO_HANDEDNESS,
            f"a {shot.spin_axis:+.1f} deg axis was printed, but '+ = fade' is a right-handed "
            "reading and no golfer is attributed to this shot",
            direction,
            screen_shape,
        )

    # `+ = fade` is `+ = right` for a right-handed golfer and `+ = left` for a left-handed one;
    # the integrator's sign is geometric and knows nothing about either.
    geometric = shot.spin_axis if handedness is Handedness.RIGHT else -shot.spin_axis
    return InferredSpinAxis(
        geometric,
        SpinAxisSource.MEASURED,
        None,
        f"{shot.spin_axis:+.1f} deg printed, read {handedness.value}-handed",
        direction,
        screen_shape,
    )


def _direction_of(spin_axis: float | None, face_to_path: float | None) -> TargetShape | None:
    """Which way the ball curved, golfer-relative, from whichever evidence exists.

    Both inputs carry the same sign convention for this purpose — `ShotData.spin_axis` is `+ = fade`
    and a positive face-to-path is a face open to the path, which is a fade — so this is a sign
    read and not a conversion. Exact zero is `STRAIGHT` rather than a coin toss; nothing on disk
    is exactly zero and a shot that were would deserve the honest label.
    """
    value = spin_axis if spin_axis is not None else face_to_path
    if value is None:
        return None
    if value > 0.0:
        return TargetShape.FADE
    if value < 0.0:
        return TargetShape.DRAW
    return TargetShape.STRAIGHT


# ---------------------------------------------------------------------------------------------
# One stored shot, resolved into conditions the integrator can fly
# ---------------------------------------------------------------------------------------------


class SpinSource(StrEnum):
    """Where the spin a flight is drawn with came from — and the two are not interchangeable.

    A measured spin is a reading off the screen. An inferred one is *the spin this integrator
    needs in order to agree with HD Golf's carry*, which is a different kind of quantity wearing
    the same unit, so nothing may show one without saying which it had.
    """

    MEASURED = "measured"
    INFERRED = "inferred"


class ShotFlight(NamedTuple):
    """One stored shot, resolved as far as the screen and the bag allow — or the reason it is not.

    The unit M15 P10's CLI prints and M15 P11's pipeline measures. It stops at the launch
    conditions rather than flying them, and that is deliberate: the caller chooses the model (the
    altitude what-if is a different `FlightModel`, not a different call), and `simulate_flight`
    owns the physical guards on what may be flown. What this resolves is only the two fields the
    screen did not print.

    **The axis refusal does not stop the flight, and the spin refusal does.** With no axis the ball
    is flown in the vertical plane and its landing offline is withheld — ADR-027 §Decision 5's
    third branch — so `reason` stays `None` and `axis.reason` carries the refusal. That asymmetry
    is the whole reason the two are separate fields.
    """

    #: Ready to fly, or `None` when `reason` says why not. `spin_axis_deg` is zero on it whenever
    #: `axis.spin_axis_deg` is `None`, which is a flight drawn planar and not an axis of zero
    #: measured.
    launch: LaunchConditions | None
    spin_source: SpinSource | None
    #: The solve, whenever one ran — including when it refused, because `InferredSpin.solution`
    #: carries which of the seven cases the carry landed on and the cap the answer sits under.
    #: `None` only when the screen printed a spin, or when there was nothing to solve from.
    spin: InferredSpin | None
    axis: InferredSpinAxis
    #: Why there is no flight. One of `contracts.unscored.INFERENCE_REASONS`; never
    #: `SPIN_AXIS_UNRESOLVED`, which is a flight drawn straight rather than a flight refused.
    reason: UnscoredReason | None
    detail: str

    @property
    def spin_rpm(self) -> float | None:
        """The spin the flight is drawn with, whichever way it was arrived at."""
        return self.launch.spin_rpm if self.launch is not None else None

    @property
    def curve_is_drawn(self) -> bool:
        """Whether this flight bends — and so whether its landing point is a landing point.

        False whenever the axis was refused. **A planar flight's `landing_offline_yds` is not new
        information**: with the axis at zero it comes out as `carry * sin(start line)` to the last
        bit, which is `shot_measure.measure_start_line_offline` — a quantity this repo has recorded
        since M9 — reached by forty flights instead of one sine. M15 P10 measured that identity on
        the corpus and it is why ADR-027 §Decision 6's `flight_landing_offline_yds` must not be
        recorded on a shot whose axis is unresolved: two names for one number is exactly the
        pooling hazard that section exists to prevent.
        """
        return self.axis.spin_axis_deg is not None


def flight_for_shot(
    shot: ShotData,
    *,
    loft_deg: float | None,
    handedness: Handedness | None,
    model: FlightModel | None = None,
    step_s: float = DEFAULT_STEP_S,
) -> ShotFlight:
    """Turn one stored shot into launch conditions, solving for the spin where the screen has none.

    `loft_deg` and `handedness` are required keywords with no defaults, because both have a
    plausible-looking wrong answer — a 7 iron's loft and a right-handed golfer — and both come
    from a different artifact than the shot does. `storage/flight_inputs.py` is what resolves them
    off disk; passing `None` for either is the corpus's own state on most shots and produces a
    stated refusal rather than a guess.

    Every field is read through `shot_measure`'s extractors rather than off `ShotData` directly,
    so there is one definition of which tile is the carry and which is the start line
    (`measure_start_line_offline` sets the precedent).
    """
    axis = infer_spin_axis(shot, handedness)
    # A refused axis is flown planar, not refused: ADR-027 §Decision 5's third branch. The zero
    # here is `LaunchConditions`' own default arriving explicitly, and `offline_is_drawn` is what
    # stops a reader taking the resulting zero offline for a measurement.
    axis_deg = axis.spin_axis_deg if axis.spin_axis_deg is not None else 0.0

    ball_speed = measure_ball_speed(shot)
    launch_angle = measure_launch_angle(shot)
    missing = [
        name
        for name, value in (("ball speed", ball_speed), ("launch angle", launch_angle))
        if value is None
    ]
    if ball_speed is None or launch_angle is None:
        return ShotFlight(
            None,
            None,
            None,
            axis,
            UnscoredReason.NO_LAUNCH_CONDITIONS,
            f"the screen printed no {' and no '.join(missing)}",
        )
    if not 0.0 < launch_angle < 90.0:
        # `simulate_flight`'s docstring names this check as the caller's to own: a ball at or below
        # the horizontal rolls, and roll is out of scope. Refused here, where there is a shot to
        # name, rather than raised out of the integrator two frames later.
        return ShotFlight(
            None,
            None,
            None,
            axis,
            UnscoredReason.NO_LAUNCH_CONDITIONS,
            f"{launch_angle:.1f} deg of launch angle is a ball that rolls rather than flies",
        )

    start_line = measure_start_line(shot) or 0.0
    unspun = UnspunLaunch(ball_speed, launch_angle, start_line, axis_deg)

    if shot.spin_rate is not None:
        return ShotFlight(
            unspun.at_spin(shot.spin_rate),
            SpinSource.MEASURED,
            None,
            axis,
            None,
            f"{shot.spin_rate:.0f} rpm printed on the screen",
        )

    carry = measure_carry_distance(shot)
    if carry is None:
        return ShotFlight(
            None,
            None,
            None,
            axis,
            UnscoredReason.NO_LAUNCH_CONDITIONS,
            "no spin printed, and no carry to solve one from either",
        )

    spin = infer_spin(unspun, carry, loft_deg, model=model, step_s=step_s)
    if spin.spin_rpm is None:
        return ShotFlight(None, None, spin, axis, spin.reason, spin.detail)
    return ShotFlight(
        unspun.at_spin(spin.spin_rpm), SpinSource.INFERRED, spin, axis, None, spin.detail
    )


# ---------------------------------------------------------------------------------------------
# The two things every inferred spin has to be reported beside
# ---------------------------------------------------------------------------------------------


class HonestTestResult(NamedTuple):
    """One validation shot re-solved as if its spin had never been printed.

    ADR-027 §Decision 3 asked for this comparison the day a shot carried both a measured spin and a
    printed carry, and its own *Deferred, by choice* section thought that day was still coming. It
    was 2026-08-10.
    """

    shot_id: str
    measured_spin_rpm: float
    inferred: InferredSpin

    @property
    def error_fraction(self) -> float | None:
        """Signed, against the measured spin. `None` where the solve refused - half of them."""
        if self.inferred.spin_rpm is None:
            return None
        return (self.inferred.spin_rpm - self.measured_spin_rpm) / self.measured_spin_rpm

    @property
    def measured_above_cap(self) -> bool:
        """Whether the real spin was past the point where carry stops depending on spin.

        True on both shots, and it is the *mechanism* rather than the size of the error: no solve
        of any quality recovers a spin the carry has stopped responding to.
        """
        return self.measured_spin_rpm > self.inferred.cap_rpm


def honest_test(
    *,
    loft_deg: float | None = None,
    model: FlightModel | None = None,
    step_s: float = DEFAULT_STEP_S,
) -> tuple[HonestTestResult, ...]:
    """Run the solve against the only two shots whose real spin is on disk.

    Re-run rather than remembered, exactly as `gate_comparisons` is: a re-sourced coefficient table
    moves these numbers, and a stored copy would keep reading true after the model beneath it moved.

    `loft_deg` defaults to `None`, which is the corpus's own state — no shot on disk carries a club
    — and makes the two-branch shot refuse for want of a loft rather than for want of physics. Pass
    the 7 iron's 30.5° to see what the prior does with it: the falling branch, 46.8% low.
    """
    results = []
    for shot in VALIDATION_SHOTS:
        launch = UnspunLaunch.from_launch(shot.launch)
        results.append(
            HonestTestResult(
                shot.shot_id,
                shot.launch.spin_rpm,
                infer_spin(
                    launch,
                    shot.simulator_carry_yds,
                    loft_deg,
                    model=model,
                    step_s=step_s,
                ),
            )
        )
    return tuple(results)


class GateOrdering(NamedTuple):
    """The gate's disagreement stated as an ordering rather than as a percentage.

    `GATE_AGREEMENT_FRACTION` is ±2.59% and reporting an inferred spin as "accurate to 2.6%" is the
    reading M15 P4 exists to prevent: the two shots pass individually while the model ranks them
    backwards. HD Golf has the lower-spin shot flying further; this model has it shorter. No
    per-shot tolerance can catch that, so a caveat built on the percentage alone says the opposite
    of what the gate found.
    """

    #: Shot one minus shot two, as HD Golf printed them. Positive: the simulator has the
    #: lower-spin shot flying further.
    simulator_gap_yds: float
    #: The same difference as this model flies it, with the recorded spin axes on.
    model_gap_yds: float
    #: And as M15 P4 flew it, before the axes were in the integrator. Kept because the difference
    #: between the two is the only measurement anyone has of how much of the inversion is the axis.
    planar_model_gap_yds: float

    @property
    def inverted(self) -> bool:
        return (self.simulator_gap_yds > 0.0) != (self.model_gap_yds > 0.0)

    @property
    def residual_yds(self) -> float:
        """What the model still cannot see, in yards: the two gaps' distance apart.

        **This is the number to report beside an inferred spin**, not the percentage. It is 5.63 yd
        on a pair of shots 4.6 yd apart — larger than the effect it is failing to reproduce.
        """
        return self.simulator_gap_yds - self.model_gap_yds

    @property
    def planar_residual_yds(self) -> float:
        return self.simulator_gap_yds - self.planar_model_gap_yds

    @property
    def axis_share(self) -> float:
        """The fraction of the inversion that the recorded spin axis closed. About a tenth.

        M15 P5 measured it and ADR-027's 2026-09-05d addendum drew the conclusion: P4's reading of
        ±2.5% as "the size of the missing *spin* effect" was an over-attribution, because a tenth
        of it was never spin's to explain. The other nine tenths remain unattributed, and M15 P6
        ruled out the atmosphere as the free parameter that would close them.
        """
        return (self.planar_residual_yds - self.residual_yds) / self.planar_residual_yds


def gate_ordering(
    *, model: FlightModel | None = None, step_s: float = DEFAULT_STEP_S
) -> GateOrdering:
    """Measure the inversion, re-flying both shots twice — as recorded, and planar.

    The planar flight is built by dropping the two lateral fields off the recorded conditions
    rather than by quoting M15 P4's numbers, so a re-sourced table moves both halves together and
    `axis_share` stays a measurement of the axis rather than of the gap between two eras.

    Two shots exactly, and the ordering of two is the weakest possible statement of it. It is also
    the only one available: ADR-027 §Context 2 says there will not be a third until the bay's
    screen profile starts printing spin.
    """
    flown = gate_comparisons(model=model, step_s=step_s)
    if len(flown) != 2:  # pragma: no cover - VALIDATION_SHOTS is a two-row constant
        raise RuntimeError(f"the ordering is defined over two shots, got {len(flown)}")
    first, second = flown

    def planar(launch: LaunchConditions) -> float:
        stripped = LaunchConditions(
            launch.ball_speed_mph, launch.launch_angle_deg, launch.spin_rpm
        )
        return simulate_flight(stripped, model=model, step_s=step_s).carry_yds

    return GateOrdering(
        simulator_gap_yds=first.shot.simulator_carry_yds - second.shot.simulator_carry_yds,
        model_gap_yds=first.flight.carry_yds - second.flight.carry_yds,
        planar_model_gap_yds=planar(first.shot.launch) - planar(second.shot.launch),
    )
