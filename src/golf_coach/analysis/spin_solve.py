"""Spin solved backwards out of a printed carry — the inverse of `analysis/flight.py`. [M15 P8]

Eleven of the thirteen shots on disk record no spin at all: the bay's screen profile prints
`Spin:` with nothing under it, and only the two 2026-08-10 reference shots carry a number
(ADR-027 §Context 2). This module is what stands in — given ball speed, launch angle and the carry
the simulator printed beside them, it looks for the spin rate that makes the integrator agree.

**What comes back is not a measurement of spin, and the strength of that sentence is the whole
point of the module.** ADR-027 §Decision 3 wrote it down as the milestone's weakest claim before
any of this ran; the 2026-09-05b addendum then ran the only honest test available — the two shots
where a real spin *is* on disk — and found one refusal and one answer 47% low. What survives is
what §Decision 3 said the number was for: **drawing a path whose carry matches the measured
carry**. It is a path-drawing device. It must never be shown to a golfer as their spin rate, and
M15 P9 owns the labelling that keeps that true.

## Why this is a separate module from the integrator

`flight.py` is the forward model and this is the inverse problem over it, called as a black box —
it holds no physics, no constant of its own, and no second copy of anything in `flight_model.py`.
Keeping them apart is also what keeps the forward model's own property obvious: `gate_comparisons`
re-flies rather than remembering, and so does every number here.

## The shape of the curve being inverted, which is not the shape the ADR assumed

Carry against launch spin, at fixed ball speed and launch angle, is **five segments**:

    low plateau ──▁▂▃ rising ▄▅ peak ▅▄ falling ▃▂▁── high plateau
    (flat)                                            (flat)

Both plateaus are the coefficient table being clamped, and they are flat for the reason the
2026-09-05b addendum found: `w` reaches the flight only through `S = wR/v`, and `S` only through
`Cl` and `Cd`. Hold those and spin has left the problem entirely — carry is *bit-identical* at
5,200 and 30,000 rpm on the reference shot. The high plateau starts where the whole flight launches
at or above the table's last row; the low one ends where the flight first climbs into its first row
(`S` rises through a flight, so a low-spin ball can start under the table and enter it later).

That geometry gives the seven cases in `SpinSolveCase`, which are ADR-027 §Decision 4's five with
two of them split by what the 2026-09-05c addendum measured: the plateau case is two different
plateaus, and the two-solution case has a sub-band between them where only one branch reaches.

The whole range is **6.8-23.6 yd wide** across the eleven spin-less shots, and the *two-branch*
part of it — peak down to the high plateau, the only band where a loft prior has anything to choose
between — is **1.3-10.7 yd** of that. The second figure is the one ADR-027's 2026-09-05b addendum
quotes as "the window", measured before P4 found the low plateau underneath it. Either way a
printed carry a yard out lands in a different case, and *which case* is most of the honest answer.

## What this module deliberately does not do

It does not pick a branch — ADR-027 §Decision 3 gives that job to the loft prior, and M15 P9 has
it, along with the refusal reasons from `contracts/unscored.py`. It does not read anything off
disk; M15 P10 hands it real shots through the tolerant readers. And it never invents a
representative from an infinite set: on either plateau the honest answer is the case itself, not a
spin chosen from a flat interval (`docs/CODE_STANDARDS.md` R7).

Stdlib only, like everything in `analysis/` (ADR-008).
"""

from __future__ import annotations

from collections.abc import Callable
from enum import StrEnum
from typing import NamedTuple

from golf_coach.analysis.benchmarks.flight_model import FlightModel, load_flight_model
from golf_coach.analysis.flight import (
    DEFAULT_STEP_S,
    FlightResult,
    LaunchConditions,
    simulate_flight,
)

#: The spin the calibration flight is flown at, purely to read a launch spin ratio off it. Any
#: positive value works — `S = wR/v` is linear in `w` at a fixed ball speed, so one flight fixes
#: the whole scale — and this one is a real 7-iron number so a failure prints something plausible.
_CALIBRATION_RPM = 5000.0

#: How close two spins have to be before the search stops splitting them. Half an rpm is four
#: orders below the 47% error the solve was measured to make on the one shot where the truth is
#: known, so the iteration count here is not what limits the answer.
_ROOT_TOLERANCE_RPM = 0.5
#: Bisection halves the bracket each pass, so this covers a 6,500 rpm range down to the tolerance
#: above with room to spare. Exhausting it returns the midpoint rather than raising: the bracket is
#: already known to hold a root, so the last iterate is a worse answer and not a wrong one.
_ROOT_ITERATIONS = 24

#: The peak's *location* barely matters — carry is stationary there, so 2 rpm of slop moves the
#: peak carry by well under a thousandth of a yard — but its *value* decides whether a printed
#: carry is reachable at all, which is why the search is run to a tolerance rather than a fixed
#: pass count.
_PEAK_TOLERANCE_RPM = 2.0
_PEAK_ITERATIONS = 40

#: Half of the last digit HD Golf prints. The carry tiles round to 1 dp, so a target within this of
#: a plateau value is *indistinguishable from it at the precision it was read*, and treating it as
#: a solvable point would be inventing a spin out of the rounding. `analysis/shot_measure.py` and
#: M15 P4's precision sweep both work from the same 1 dp reading.
_PRINTED_PRECISION_YDS = 0.05


class UnspunLaunch(NamedTuple):
    """`LaunchConditions` with the one field this module is looking for left out.

    A separate shape rather than a `LaunchConditions` whose `spin_rpm` is ignored, because an
    ignored field is a trap: every caller here holds launch conditions that genuinely have no spin
    on them, and a solve that quietly overwrote a spin the caller *did* pass would be impossible to
    see at the call site.
    """

    ball_speed_mph: float
    launch_angle_deg: float
    #: Both lateral fields carry `LaunchConditions`' signs and defaults exactly — see it for what
    #: `+` means and for why the spin axis is not the contract's `spin_axis` sign.
    launch_direction_deg: float = 0.0
    spin_axis_deg: float = 0.0

    @classmethod
    def from_launch(cls, launch: LaunchConditions) -> UnspunLaunch:
        """Drop the spin off a full set of launch conditions.

        Used to re-solve the two validation shots as if their spin had never been printed, which is
        the only honest test ADR-027 §Decision 3 can be given.
        """
        return cls(
            launch.ball_speed_mph,
            launch.launch_angle_deg,
            launch.launch_direction_deg,
            launch.spin_axis_deg,
        )

    def at_spin(self, spin_rpm: float) -> LaunchConditions:
        """Put a candidate spin back on, giving something the integrator can fly."""
        return LaunchConditions(
            self.ball_speed_mph,
            self.launch_angle_deg,
            spin_rpm,
            self.launch_direction_deg,
            self.spin_axis_deg,
        )


class CarryWindow(NamedTuple):
    """Every carry these launch conditions can produce, and the spins at the edges of that set.

    This is the module's real product. The solve returns a spin only in three of its seven cases,
    but the window is measured every time and says *why* — a printed carry 2 yd above the peak and
    one sitting exactly on the high plateau both come back without a number, and they are entirely
    different findings about the shot.

    Read `low_plateau_yds` and `high_plateau_yds` as values of the flat segments, and the two rpm
    fields as the edges of the flat intervals that produce them: every spin at or below
    `low_plateau_max_rpm` flies `low_plateau_yds`, and every spin at or above
    `high_plateau_min_rpm` flies `high_plateau_yds`.
    """

    launch: UnspunLaunch
    #: What the ball flies when the spin is too low for the coefficient table to see it at all.
    #: Not zero-lift: the first row is *held*, so this is a real flight with `Cl = 0.141` on it.
    low_plateau_yds: float
    #: The most spin that still flies `low_plateau_yds` — the point where the flight first climbs
    #: into the table's measured rows on its way down.
    low_plateau_max_rpm: float
    peak_yds: float
    peak_rpm: float
    #: What the ball flies once every step of the flight is above the table's last row. Every iron
    #: in this repo's corpus is here for its whole path (ADR-027's 2026-09-05 addendum).
    high_plateau_yds: float
    #: The least spin that flies `high_plateau_yds` — analytic, not searched: the launch spin ratio
    #: is the smallest one in a flight, so it alone decides whether the whole path is clamped.
    #: **This is also the cap on every inferred spin**, and M15 P9 has to report it: an answer here
    #: means the carry stopped responding, not that the golfer spun the ball this fast.
    high_plateau_min_rpm: float

    @property
    def floor_yds(self) -> float:
        """The shortest carry these conditions can fly, whichever plateau it falls on.

        The 2026-09-05c addendum's correction, as code: the bottom of the range is the *low*-spin
        clamp on every shot measured so far, and not — as the addendum before it assumed — the
        high-spin one that the falling branch runs into.
        """
        return min(self.low_plateau_yds, self.high_plateau_yds)

    @property
    def width_yds(self) -> float:
        """Floor to peak: the room a printed carry has to be wrong in before the case changes."""
        return self.peak_yds - self.floor_yds


class SpinSolveCase(StrEnum):
    """Which of the curve's segments the target carry landed on, and how many spins produce it.

    ADR-027 §Decision 4's five cases, with two of them split by what M15 P4 measured — the plateau
    case is two different plateaus with different meanings, and the two-solution case has a band
    between the plateaus where only one branch reaches down that far.

    A `StrEnum` because M15 P9 stores this beside the answer and M15 P14 serves it: the case is
    the part of the result that stays true even when there is no number.
    """

    #: Two spins fly this carry, one either side of the peak. ADR-027 §Decision 3's loft prior is
    #: what chooses; with no loft on record there is no branch rule and no answer.
    TWO_BRANCHES = "two_branches"

    #: Exactly one spin flies it, because the target sits between the two plateau values and the
    #: falling branch never comes down this far. **Unique is not automatically the good news it
    #: looks like**: on the reference shot the band runs 1,129-1,538 rpm, which is not a spin a
    #: 7 iron produces, and ADR-027's 2026-09-05c addendum asks for a refusal there on exactly that
    #: ground. But the band is not always implausible — on `2026-08-23-2`, launched at 3.4°, it
    #: runs 1,323-4,572 rpm and the answer lands at 2,307. So the refusal M15 P9 owes this case has
    #: to be argued from where the answer sits, not from the case's name.
    BETWEEN_PLATEAUS = "between_plateaus"

    #: The target is at the peak, to within the precision it was printed at — one spin, on a knife
    #: edge, and the answer is the peak itself rather than a branch.
    AT_PEAK = "at_peak"

    #: Infinitely many: every spin at or above `high_plateau_min_rpm` flies exactly this carry. The
    #: common case for a well-struck iron, and the reason a carry cannot recover an iron's spin.
    ON_HIGH_PLATEAU = "on_high_plateau"

    #: Infinitely many at the other end: every spin from zero up to `low_plateau_max_rpm`. Nothing
    #: in that interval is a plausible struck-iron spin, which is the difference between this and
    #: the case above.
    ON_LOW_PLATEAU = "on_low_plateau"

    #: No spin flies the ball this far. A finding rather than a failure — but read it beside the
    #: model's own ~2.6% disagreement with HD Golf before calling it an OCR fault, because seven of
    #: the eleven printed carries clear the peak by less than that (2026-09-05b addendum).
    ABOVE_PEAK = "above_peak"

    #: No spin flies the ball this *short* — the opposite refusal, and the one ADR-027 §Decision 4
    #: did not know existed, because it assumed carry falls without bound past the peak. It floors
    #: instead. This is what happens to `2026-08-10-2`, whose printed 121.0 yd is below anything
    #: these launch conditions can fly.
    BELOW_FLOOR = "below_floor"


class SpinSolution(NamedTuple):
    """What the carry could and could not say about the spin.

    The case is always present; the two spins are present only where they exist. Nothing here is
    filled in with a plausible value when it is absent (`docs/CODE_STANDARDS.md` R7) — in
    particular neither plateau case returns a representative spin, because choosing one out of a
    flat interval is exactly the invention this module is written not to make.
    """

    case: SpinSolveCase
    target_carry_yds: float
    window: CarryWindow
    #: The low-spin answer, below the peak. Present for `TWO_BRANCHES`, for `BETWEEN_PLATEAUS` when
    #: the low plateau is the lower of the two, and for `AT_PEAK`, where both branches meet.
    rising_rpm: float | None
    #: The high-spin answer, above the peak — the branch a struck iron is actually on, and the one
    #: capped by `window.high_plateau_min_rpm`.
    falling_rpm: float | None

    @property
    def candidates(self) -> tuple[float, ...]:
        """The distinct spins that fly the target carry.

        Empty where none does — and equally where infinitely many do.

        `AT_PEAK` sets both branches to the same value on purpose, so it comes back as the one
        answer it is rather than as two.
        """
        found = [rpm for rpm in (self.rising_rpm, self.falling_rpm) if rpm is not None]
        if len(found) == 2 and found[0] == found[1]:
            return (found[0],)
        return tuple(found)


def carry_window(
    launch: UnspunLaunch,
    *,
    model: FlightModel | None = None,
    step_s: float = DEFAULT_STEP_S,
) -> CarryWindow:
    """Measure the whole carry-against-spin curve for one set of launch conditions.

    Costs about forty flights, which is why it is a separate function: a caller that wants both the
    window and a solve against it (M15 P10's CLI reports both) should measure it once and hand it
    back to `solve_spin_from_carry`.

    Raises whatever `simulate_flight` raises for launch conditions there is no flight to integrate
    from — a launch angle at or below the horizontal is the caller's check to have made, and
    `flight.py` says why that is a raise and not a refusal.
    """
    model = load_flight_model() if model is None else model

    def fly(spin_rpm: float) -> FlightResult:
        return simulate_flight(launch.at_spin(spin_rpm), model=model, step_s=step_s)

    def carry(spin_rpm: float) -> float:
        return fly(spin_rpm).carry_yds

    # The launch spin ratio is `wR/v` with the launch speed in it, so at a fixed ball speed it is
    # simply proportional to the spin. One flight therefore fixes the scale exactly, and no unit
    # conversion is re-typed here from `flight.py` — the constants stay in one place (R4).
    calibration = fly(_CALIBRATION_RPM)
    ratio_per_rpm = calibration.points[0].spin_ratio / _CALIBRATION_RPM
    high_plateau_min_rpm = model.coefficients.spin_ratio_max / ratio_per_rpm

    low_plateau_yds = carry(0.0)
    high_plateau_yds = carry(high_plateau_min_rpm)
    low_plateau_max_rpm = _low_plateau_shoulder(
        fly, model.coefficients.spin_ratio_min, high_plateau_min_rpm
    )

    peak_rpm, peak_yds = _peak(carry, low_plateau_max_rpm, high_plateau_min_rpm)
    # The interior search assumes one hump between the shoulders; the plateau values are known
    # exactly, so taking the best of the three costs nothing and means launch conditions whose
    # carry only ever rises come back with a peak at the end rather than with a golden-section
    # artefact somewhere in the middle. **This is not defensive**: `2026-08-23-2` is on disk and
    # does exactly that, and its peak is its high plateau to the last bit — so it has no falling
    # branch at all and no two-solution band.
    if low_plateau_yds >= peak_yds:
        peak_rpm, peak_yds = low_plateau_max_rpm, low_plateau_yds
    if high_plateau_yds >= peak_yds:
        peak_rpm, peak_yds = high_plateau_min_rpm, high_plateau_yds

    return CarryWindow(
        launch=launch,
        low_plateau_yds=low_plateau_yds,
        low_plateau_max_rpm=low_plateau_max_rpm,
        peak_yds=peak_yds,
        peak_rpm=peak_rpm,
        high_plateau_yds=high_plateau_yds,
        high_plateau_min_rpm=high_plateau_min_rpm,
    )


def solve_spin_from_carry(
    launch: UnspunLaunch,
    target_carry_yds: float,
    *,
    model: FlightModel | None = None,
    step_s: float = DEFAULT_STEP_S,
    window: CarryWindow | None = None,
) -> SpinSolution:
    """Find the spin — or the spins, or neither — that fly `target_carry_yds` from these conditions.

    The target is the carry the launch monitor printed, which is the output of *its* flight model.
    Solving against it fits this model to that one rather than to reality, and ADR-027 §Decision 3
    is on the record about it: what comes back is "the spin our integrator needs in order to agree
    with the simulator" and is not a measurement of spin.
    """
    model = load_flight_model() if model is None else model
    if window is None:
        window = carry_window(launch, model=model, step_s=step_s)

    def carry(spin_rpm: float) -> float:
        return simulate_flight(launch.at_spin(spin_rpm), model=model, step_s=step_s).carry_yds

    if target_carry_yds > window.peak_yds + _PRINTED_PRECISION_YDS:
        return SpinSolution(SpinSolveCase.ABOVE_PEAK, target_carry_yds, window, None, None)
    if target_carry_yds > window.peak_yds:
        # Above the peak but by less than the printed carry's own last digit. The two branches have
        # collapsed onto each other here, so the peak is the answer rather than either of them.
        return SpinSolution(
            SpinSolveCase.AT_PEAK, target_carry_yds, window, window.peak_rpm, window.peak_rpm
        )
    if abs(target_carry_yds - window.low_plateau_yds) <= _PRINTED_PRECISION_YDS:
        return SpinSolution(SpinSolveCase.ON_LOW_PLATEAU, target_carry_yds, window, None, None)
    if abs(target_carry_yds - window.high_plateau_yds) <= _PRINTED_PRECISION_YDS:
        return SpinSolution(SpinSolveCase.ON_HIGH_PLATEAU, target_carry_yds, window, None, None)
    if target_carry_yds < window.floor_yds:
        return SpinSolution(SpinSolveCase.BELOW_FLOOR, target_carry_yds, window, None, None)

    rising_rpm = _root(carry, window.low_plateau_max_rpm, window.peak_rpm, target_carry_yds)
    falling_rpm = _root(carry, window.peak_rpm, window.high_plateau_min_rpm, target_carry_yds)
    found = [rpm for rpm in (rising_rpm, falling_rpm) if rpm is not None]
    if not found:
        # The target is inside the window the same window said it was inside, and neither branch
        # brackets it. That is the curve not having the shape measured above, which is a wiring bug
        # in this module rather than anything about the shot (R8).
        raise RuntimeError(
            f"{target_carry_yds} yd sits inside {window.floor_yds}-{window.peak_yds} yd for "
            f"{launch}, but neither branch brackets it - the carry curve is not the shape "
            "carry_window measured"
        )
    case = SpinSolveCase.TWO_BRANCHES if len(found) == 2 else SpinSolveCase.BETWEEN_PLATEAUS
    return SpinSolution(case, target_carry_yds, window, rising_rpm, falling_rpm)


def _low_plateau_shoulder(
    fly: Callable[[float], FlightResult],
    table_spin_ratio_min: float,
    high_plateau_min_rpm: float,
) -> float:
    """The most spin whose flight never once climbs into the table's measured rows.

    Bisected on the *mechanism* rather than on the carry. The two agree — the plateau is flat
    because the coefficients are held — but `spin_ratio_max` rises with spin by construction, while
    a carry that happens to be flat over some other interval would fool an equality test. `S` is
    largest at landing, so the whole flight is under the table exactly when the landing point is.
    """
    low, high = 0.0, high_plateau_min_rpm
    for _ in range(_ROOT_ITERATIONS):
        if high - low <= _ROOT_TOLERANCE_RPM:
            break
        mid = 0.5 * (low + high)
        result = fly(mid)
        if result.spin_ratio_max <= table_spin_ratio_min:
            low = mid
        else:
            high = mid
    return low


def _peak(
    carry: Callable[[float], float], low_rpm: float, high_rpm: float
) -> tuple[float, float]:
    """Golden-section search for the top of the hump between the two plateaus.

    Golden section rather than a derivative or a sampled sweep: carry has no closed form here, each
    evaluation is a whole flight, and a sweep fine enough to place the peak to a couple of rpm
    would cost thousands of them.

    It assumes one hump between the shoulders, which is true of every normally-launched shot on
    disk and **false of at least one real one** — `2026-08-23-2`, launched at 3.4°, rises all the
    way to the high plateau and never falls. Golden section walks to that edge rather than
    inventing an interior maximum, and `carry_window` then takes the best of the three known
    points, so the caller and not this helper is where that case is closed.
    """
    #: 1/phi, the ratio that lets each pass reuse one of the previous pass's two evaluations.
    ratio = 0.6180339887498949
    a, b = low_rpm, high_rpm
    c, d = b - ratio * (b - a), a + ratio * (b - a)
    fc, fd = carry(c), carry(d)
    for _ in range(_PEAK_ITERATIONS):
        if b - a <= _PEAK_TOLERANCE_RPM:
            break
        if fc > fd:
            b, d, fd = d, c, fc
            c = b - ratio * (b - a)
            fc = carry(c)
        else:
            a, c, fc = c, d, fd
            d = a + ratio * (b - a)
            fd = carry(d)
    peak_rpm = 0.5 * (a + b)
    return peak_rpm, carry(peak_rpm)


def _root(
    carry: Callable[[float], float], a_rpm: float, b_rpm: float, target_yds: float
) -> float | None:
    """Bisect for the spin that flies `target_yds`, on a branch known to be monotone.

    Orientation-free — it reads the sign at each end rather than being told which way the branch
    runs — so the same helper serves the rising and the falling branch, and neither call site has a
    flag on it saying which (`docs/CODE_STANDARDS.md` R14).

    Returns `None` when the bracket does not straddle the target. That is the ordinary way a branch
    reports that it never reaches the carry being asked for, and it is what separates
    `BETWEEN_PLATEAUS` from `TWO_BRANCHES`.
    """
    a, b = a_rpm, b_rpm
    fa, fb = carry(a) - target_yds, carry(b) - target_yds
    if fa == 0.0:
        return a
    if fb == 0.0:
        return b
    if (fa > 0.0) == (fb > 0.0):
        return None
    for _ in range(_ROOT_ITERATIONS):
        if b - a <= _ROOT_TOLERANCE_RPM:
            break
        mid = 0.5 * (a + b)
        fm = carry(mid) - target_yds
        if (fm > 0.0) == (fa > 0.0):
            a, fa = mid, fm
        else:
            b, fb = mid, fm
    return 0.5 * (a + b)
