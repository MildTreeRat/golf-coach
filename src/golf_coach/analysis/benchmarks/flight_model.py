"""The ball-flight constants — published measurements, read, never fitted. [M15 P2]

`analysis/flight.py` will integrate a golf ball from its launch conditions (M15 P3). Everything
that integrator needs and cannot derive lives here as committed data: the ball's mass and
diameter, an atmosphere and the law that moves it to another altitude, a spin-decay rate, and a
table of lift and drag coefficients against spin ratio `S = wR/v`.

This is [ADR-022](../../../../docs/decisions/022-learned-artifacts-as-committed-data.md)'s
division applied the way `clubs/catalogue.py` applies it — data committed as JSON with provenance,
and a dictionary read inside the package — and **not** the way `joint.py` applies it. Nothing here
was fitted. These are somebody else's wind-tunnel and test-range measurements, so there is no
`scripts/` fitting stage under the `research` extra and none should be written
([ADR-027](../../../../docs/decisions/027-ball-flight-simulation.md) §Decision 2). The five blocks
of the artifact carry four different citations — the atmosphere and its altitude profile are the
zero row and the lowest layer of one standard — which is why provenance sits per block rather
than once at the top.

## It ships before the integrator, and that ordering is the point

An integrator written first has to carry a constant to run at all, and a constant that works is a
constant nobody removes. So this phase lands first and `flight.py` will have nowhere to put a
fallback (ADR-027 §Decision 2).

## The table is a driver's flight, and our shots are irons — which is a measured problem

`coefficients.rows` is Table 3 of US 7,156,757 B2: eight points along a **driver** trajectory,
where the ball slows as its spin ratio rises, so each spin ratio is paired with one Reynolds
number and the pair runs 0.085/230,000 down to 0.284/69,000. Reading it against spin ratio alone
imports that pairing, which is the simplification ADR-027 §Decision 1 chose; the Reynolds column
is stored anyway so a later two-dimensional read does not have to re-source the table.

The consequence was measured rather than assumed, and it is larger than the simplification:
**both validation shots fly entirely above the table.** A 7 iron at 90.7 mph of ball speed and
5991 rpm launches at `S = 0.330`, and 90.5 mph with 8100 rpm launches at `S = 0.447` — against a
published ceiling of 0.284, and `S` climbs further through the flight as the ball slows faster
than the spin decays. Every step of both flights therefore reads the clamp, not the table.

A throwaway RK4 over exactly these constants returned 122.4 yd against HD Golf's 125.6 and
124.1 yd against its 121.0 — about 2.5% out on each, with **opposite signs**, which is what a
flattened coefficient looks like: with `Cl` and `Cd` pinned at the last row, the only route spin
still has into carry is the Magnus term's own `w`, so the model cannot fully tell 5991 rpm from
8100 rpm. That agreement is close enough to be encouraging and was produced by a constant pair
rather than by the table, and P4 — the gate — has to say both of those things or it will read as a
validation of data that was never consulted.

Clamping is the deliberate choice over linear extrapolation. Smits and Smith (1994) measured out
to `S = 1.4` and report lift *saturating* rather than continuing to climb, so holding the last row
is closer to the physics than extending its slope, and it can never return an unbounded
coefficient. It is still an extrapolation, so `AeroCoefficients.clamped` says when it happened and
callers are expected to surface it.

Stdlib + pydantic, `contracts`-free (ADR-008). No numpy: M15 P6's subprocess pin in
`tests/api/test_pipeline_imports.py` covers `analysis.flight`, which reaches this module, so an
import here fails there.
"""

from __future__ import annotations

import json
import math
from functools import lru_cache
from importlib import resources

from pydantic import BaseModel

_MODEL_FILE = "flight_model_v1.json"


class SourceNote(BaseModel):
    """Where one block of the artifact came from.

    Required on every block, for `CatalogueRow.provenance`'s reason: a block only exists because
    something published it, and a constant that cannot say what would make the file's whole claim —
    that none of this was fitted here — unverifiable one block at a time.
    """

    citation: str
    url: str
    note: str
    #: Only the coefficient block has one; it records what the stored Reynolds column is for.
    reynolds_note: str | None = None


class BallSpec(BaseModel):
    """The ball the integrator flies: conforming mass and diameter."""

    mass_kg: float
    diameter_m: float
    source: SourceNote

    @property
    def radius_m(self) -> float:
        return self.diameter_m / 2.0

    @property
    def frontal_area_m2(self) -> float:
        """The reference area both coefficients are defined against."""
        return math.pi * self.radius_m**2


class Atmosphere(BaseModel):
    """The air the ball flies through — one air state, not a law for generating them.

    The committed instance is ISO 2533 at sea level, which is where every shot on disk was hit.
    `AtmosphereProfile` is what moves it somewhere else (M15 P6).
    """

    name: str
    density_kg_m3: float
    kinematic_viscosity_m2_s: float
    temperature_k: float
    pressure_pa: float
    source: SourceNote

    def reynolds_for(self, speed_m_s: float, diameter_m: float) -> float:
        """`Re = vD/nu` — not read by the coefficient lookup, which is indexed on spin ratio.

        It exists because the stored table pairs a Reynolds number with each spin ratio, so
        anything checking whether a flight is inside the conditions those coefficients were
        measured at needs to be able to compute one.
        """
        return speed_m_s * diameter_m / self.kinematic_viscosity_m2_s


class AtmosphereProfile(BaseModel):
    """How the air changes with altitude — ISO 2533's lowest layer, written as ratios. [M15 P6]

    The standard atmosphere is normally quoted as three absolute formulas: temperature falls
    linearly at the lapse rate, pressure follows `p0 * (T/T0)**(g/(L*R))`, and density is
    `p/(R*T)`. **This is written as ratios against the committed sea-level block instead**, and
    that is the one design decision in the class.

    An absolute evaluation would not reproduce the block it is supposed to extend, and the reason
    is worth the paragraph because it is only *half* true. Density it reproduces almost exactly —
    `p0/(R*T0)` is `1.22500002` against the committed `1.225`, a relative 1.5e-8, because ISO 2533
    picked its gas constant to make that sea-level triple self-consistent. **Viscosity it does
    not**: Sutherland at 288.15 K gives `1.4607186e-5` against the committed `1.4607e-5`, a
    relative 1.3e-5, because the standard publishes that one rounded to five figures. So the
    committed row is self-consistent in one quantity and rounded in another, and an absolute
    profile evaluated at zero altitude would hand back an atmosphere subtly unlike the default in
    whichever of the two happens to be rounded today.

    Ratios do not care which. Every one of the four numbers comes back **to the bit** at zero
    altitude — and so does the whole flight flown through it, point for point, which is the
    property the tests pin — and it holds by construction rather than by the committed numbers
    happening to agree with each other. Only `name` and `source` differ there, and they differ on
    purpose: an air state that was generated should say which law generated it rather than pass
    itself off as the published row.

    The Sutherland relation gives the dynamic viscosity, and the ratio form needs only `S`. The
    kinematic viscosity the integrator's Reynolds helper reads is `mu/rho`, so it *rises* with
    altitude — the air thins faster than it stiffens. Nothing in the flight reads it today, which
    is exactly why it is worth getting right rather than leaving at its sea-level value: the day a
    two-dimensional coefficient read arrives (the stored Reynolds column is there for it), a
    frozen viscosity would be a silent error rather than a missing feature.
    """

    name: str
    lapse_rate_k_per_m: float
    gas_constant_j_kg_k: float
    #: Stored, evaluated by nothing — see the artifact's own note. The ratios need only `S`.
    sutherland_beta_kg_m_s_sqrt_k: float
    sutherland_s_k: float
    #: The layer's own domain, not a judgement about golf. Outside it the lapse rate is a
    #: different number and this arithmetic is simply the wrong standard.
    altitude_min_m: float
    altitude_max_m: float
    source: SourceNote

    def pressure_exponent(self, gravity_m_s2: float) -> float:
        """`g/(L*R)` — about 5.2559. Derived rather than stored, because it is derived."""
        return gravity_m_s2 / (self.lapse_rate_k_per_m * self.gas_constant_j_kg_k)

    def viscosity_ratio(self, temperature_k: float, base_temperature_k: float) -> float:
        """Sutherland's law as a ratio: `(T/T0)**1.5 * (T0+S)/(T+S)`, for dynamic viscosity."""
        s = self.sutherland_s_k
        return (temperature_k / base_temperature_k) ** 1.5 * (base_temperature_k + s) / (
            temperature_k + s
        )

    def at_altitude(
        self, base: Atmosphere, altitude_m: float, gravity_m_s2: float
    ) -> Atmosphere:
        """The air at `altitude_m`, anchored on `base` as the standard's zero row.

        Raises outside the layer this profile transcribes. That is the raising side of
        `docs/CODE_STANDARDS.md` R8 rather than a refusal, for `simulate_flight`'s reason: a
        caller asking for 30 km has a units bug, not a golf course, and there is no sentence to
        say to a golfer about it.
        """
        if not self.altitude_min_m <= altitude_m <= self.altitude_max_m:
            raise ValueError(
                f"altitude {altitude_m} m is outside {self.name} "
                f"({self.altitude_min_m} to {self.altitude_max_m} m), where the lapse rate this "
                "profile carries is not the standard's"
            )

        temperature_ratio = 1.0 - self.lapse_rate_k_per_m * altitude_m / base.temperature_k
        temperature_k = base.temperature_k * temperature_ratio
        exponent = self.pressure_exponent(gravity_m_s2)
        # Density carries `exponent - 1` because `rho = p/(R*T)` and the `T` in the denominator
        # is itself the ratio: the pressure drop divided by the temperature drop.
        density_ratio = temperature_ratio ** (exponent - 1.0)
        return Atmosphere(
            name=f"{self.name}_{altitude_m:g}m",
            density_kg_m3=base.density_kg_m3 * density_ratio,
            kinematic_viscosity_m2_s=base.kinematic_viscosity_m2_s
            * self.viscosity_ratio(temperature_k, base.temperature_k)
            / density_ratio,
            temperature_k=temperature_k,
            pressure_pa=base.pressure_pa * temperature_ratio**exponent,
            # The generated air state cites the profile that generated it, so a flight can always
            # say where its atmosphere came from — which is the whole point of provenance per
            # block, and an altitude atmosphere is the first one in this repo nobody published.
            source=self.source,
        )


class SpinDecay(BaseModel):
    """Exponential spin-down at a fixed fractional rate."""

    kind: str
    rate_per_s: float
    source: SourceNote

    def spin_after(self, initial_rad_s: float, seconds: float) -> float:
        """One rate for every speed and spin ratio, which is the simplification worth naming.

        Smits and Smith (1994) measured decay as a function of both. The 4%/s figure here is the
        one Lyu et al. used for a driver trajectory, and a 4.5 s iron flight loses ~17% of its
        spin under it — enough to matter to the carry, not enough that the shape of the decay law
        dominates the answer.
        """
        return initial_rad_s * math.exp(-self.rate_per_s * seconds)


class AeroRow(BaseModel):
    """One measured point: a spin ratio, the Reynolds number it was paired with, two orientations.

    `pp` and `ph` are the two seam orientations the source reports separately. They are kept apart
    rather than pre-averaged so the artifact stays a faithful transcription of the published table
    — a stored average is a number no source can be checked against.
    """

    spin_ratio: float
    reynolds: float
    cl_pp: float
    cd_pp: float
    cl_ph: float
    cd_ph: float

    @property
    def cl(self) -> float:
        return (self.cl_pp + self.cl_ph) / 2.0

    @property
    def cd(self) -> float:
        return (self.cd_pp + self.cd_ph) / 2.0


class AeroCoefficients(BaseModel):
    """A lift and drag coefficient for one spin ratio, and whether the table actually covered it."""

    spin_ratio: float
    cl: float
    cd: float
    #: True when the request fell outside the measured range and the nearest end row was held.
    #: Not a failure and not a refusal — but it is the difference between a read and an
    #: extrapolation, and on every shot stored so far it is true for the whole flight.
    clamped: bool


class AeroTable(BaseModel):
    """The measured coefficient rows, ascending in spin ratio, with the rule for reading them."""

    index: str
    orientation_rule: str
    out_of_range: str
    source: SourceNote
    rows: list[AeroRow]

    @property
    def spin_ratio_min(self) -> float:
        return self.rows[0].spin_ratio

    @property
    def spin_ratio_max(self) -> float:
        return self.rows[-1].spin_ratio

    def coefficients_for(self, spin_ratio: float) -> AeroCoefficients:
        """Linear interpolation between measured rows; the end rows held beyond them.

        Held rather than extended: see the module docstring. A negative spin ratio is not
        meaningful here and clamps to the first row like any other under-range value — the
        integrator's own sign handling is where a top-spin shot belongs, not this lookup.
        """
        if spin_ratio <= self.spin_ratio_min:
            first = self.rows[0]
            return AeroCoefficients(
                spin_ratio=spin_ratio, cl=first.cl, cd=first.cd, clamped=True
            )
        if spin_ratio >= self.spin_ratio_max:
            last = self.rows[-1]
            return AeroCoefficients(spin_ratio=spin_ratio, cl=last.cl, cd=last.cd, clamped=True)

        for low, high in zip(self.rows, self.rows[1:], strict=False):
            if low.spin_ratio <= spin_ratio <= high.spin_ratio:
                span = (spin_ratio - low.spin_ratio) / (high.spin_ratio - low.spin_ratio)
                return AeroCoefficients(
                    spin_ratio=spin_ratio,
                    cl=low.cl + span * (high.cl - low.cl),
                    cd=low.cd + span * (high.cd - low.cd),
                    clamped=False,
                )
        # Unreachable while the rows are sorted, which `FlightModel` checks on load. Falling
        # through to the last row rather than raising keeps a data defect from taking down a
        # flight mid-integration; the loader is where that defect gets caught.
        last = self.rows[-1]
        return AeroCoefficients(spin_ratio=spin_ratio, cl=last.cl, cd=last.cd, clamped=True)


class FlightModel(BaseModel):
    """Everything `analysis/flight.py` needs and cannot derive."""

    kind: str
    gravity_m_s2: float
    ball: BallSpec
    atmosphere: Atmosphere
    atmosphere_profile: AtmosphereProfile
    spin_decay: SpinDecay
    coefficients: AeroTable

    def at_altitude(self, altitude_m: float) -> FlightModel:
        """This model with its air swapped for the air at `altitude_m` — the what-if. [M15 P6]

        A whole model rather than a loose atmosphere, because `simulate_flight` already takes a
        `model` and adding an `altitude_m` beside it would be a second way to say one thing. The
        caller writes `simulate_flight(launch, model=load_flight_model().at_altitude(1609))` and
        the integrator does not learn a new argument.

        The cached default is never mutated: `model_copy` returns a new object and the atmosphere
        it is given is freshly constructed. At zero altitude the returned model flies a
        bit-identical flight — see `AtmosphereProfile` for why that is construction and not luck.
        """
        return self.model_copy(
            update={
                "atmosphere": self.atmosphere_profile.at_altitude(
                    self.atmosphere, altitude_m, self.gravity_m_s2
                )
            }
        )

    def spin_ratio(self, speed_m_s: float, spin_rad_s: float) -> float | None:
        """`S = wR/v`, or `None` at a standstill.

        `None` rather than infinity: a ball with no speed has no aerodynamic coefficient to look
        up, and returning a number for it would put a division by zero one step further from where
        it happened.
        """
        if speed_m_s <= 0.0:
            return None
        return spin_rad_s * self.ball.radius_m / speed_m_s

    def coefficients_at(self, speed_m_s: float, spin_rad_s: float) -> AeroCoefficients | None:
        """The coefficients for one instant of a flight, or `None` where the spin ratio is not."""
        ratio = self.spin_ratio(speed_m_s, spin_rad_s)
        if ratio is None:
            return None
        return self.coefficients.coefficients_for(ratio)


class FlightDatasetInfo(BaseModel):
    """Provenance for the artifact as a whole — the per-block citations live on the blocks."""

    name: str
    kind: str
    assembled_on: str
    assembled_by: str
    #: Pinned false and asserted by the tests. The day something here *is* fitted, ADR-022's
    #: fitting-stage rules apply and this file stops being the `clubs/catalogue.py` shape.
    fitted: bool
    license_note: str
    note: str


@lru_cache(maxsize=1)
def _load() -> tuple[FlightDatasetInfo, FlightModel]:
    raw = resources.files(__package__).joinpath(_MODEL_FILE).read_text(encoding="utf-8")
    payload = json.loads(raw)
    model = FlightModel.model_validate(payload["model"])

    rows = model.coefficients.rows
    if len(rows) < 2:
        raise ValueError(f"{_MODEL_FILE}: a coefficient table needs at least two rows to read")
    ratios = [row.spin_ratio for row in rows]
    if ratios != sorted(ratios) or len(set(ratios)) != len(ratios):
        # Checked on load rather than trusted, because an out-of-order row does not raise
        # anywhere downstream — it silently returns the wrong coefficient for a real flight.
        raise ValueError(f"{_MODEL_FILE}: coefficient rows must ascend in spin_ratio")

    return FlightDatasetInfo.model_validate(payload["dataset"]), model


def flight_dataset_info() -> FlightDatasetInfo:
    """Provenance of the published constants this model is assembled from."""
    return _load()[0]


def load_flight_model() -> FlightModel:
    """The ball, the air, the spin decay and the coefficient table."""
    return _load()[1]


def coefficients_for(spin_ratio: float) -> AeroCoefficients:
    """Convenience: lift and drag at one spin ratio."""
    return load_flight_model().coefficients.coefficients_for(spin_ratio)
