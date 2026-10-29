//! The ball-flight constants — published measurements, read, never fitted. [M22 P8]
//!
//! The port of `benchmarks/flight_model.py`. Everything [`crate::flight`] needs and cannot derive:
//! the ball's mass and diameter, an air state, a spin-decay rate, and a table of lift and drag
//! coefficients against spin ratio `S = wR/v`. Nothing here was fitted — these are somebody else's
//! wind-tunnel and test-range measurements, which is why
//! [ADR-027](../../../../docs/decisions/027-ball-flight-simulation.md) §Decision 2 forbids a
//! `scripts/` fitting stage for it and why the artifact carries four citations rather than one.
//!
//! It ships before the integrator here for the same reason it does in Python: an integrator written
//! first has to carry a constant to run at all, and a constant that works is a constant nobody
//! removes.
//!
//! # The ported surface is seven numbers and eight rows, and that is the phase's first finding
//!
//! `flight_model.py` is 416 lines. What `conformance.py::run_vector` reaches of it is
//! `gravity_m_s2`, the ball's `mass_kg` and `diameter_m`, the atmosphere's `density_kg_m3`, the
//! spin decay's `rate_per_s`, and the eight coefficient rows' `spin_ratio` and four `cl`/`cd`
//! columns. Everything else in that file is either **provenance** — a `SourceNote` on every
//! block, `FlightDatasetInfo` over the whole artifact — or the **altitude what-if**,
//! `AtmosphereProfile` and `FlightModel.at_altitude`, whose only caller is
//! `scripts/simulate_flight.py --altitude`.
//!
//! So none of it came across at M22, on the rule [`super`] set for `dataset_info()` and
//! [`super::store`] for `source_date`: a port of a function no committed vector can gate is a
//! second implementation with no oracle.
//!
//! **M29 P28 brought the provenance and the what-if**, because `golf-lab flight` is
//! `simulate_flight.py`'s twin and prints both: [`SourceNote`] on every block and
//! [`FlightDatasetInfo`] for `--verbose`, the atmosphere's name, temperature and pressure for the
//! report's `air` line, and [`AtmosphereProfile`] with [`FlightModel::at_altitude`] for
//! `--altitude`. Their oracle is that verb's text against the frozen script's (the M29 plan's C8),
//! and the unit tests below hold the four numbers at four altitudes to frozen Python's `repr`, to
//! the bit. Still not here: `Atmosphere.reynolds_for`, `AeroRow.reynolds`, `AeroTable`'s three
//! prose fields and `SpinDecay.kind`, which are on disk and read by nothing either verb calls. serde
//! ignores unknown keys, so leaving them out of the structs costs nothing and states the reach
//! honestly.
//!
//! **The Reynolds column staying unparsed is worth one extra sentence**, because it is not dead
//! weight in the artifact: `coefficients.rows` is Table 3 of US 7,156,757 B2, eight points along a
//! *driver* trajectory where each spin ratio is paired with one Reynolds number, and reading the
//! table against spin ratio alone imports that pairing (ADR-027 §Decision 1). The column is stored
//! so a later two-dimensional read does not have to re-source the table. When that read arrives it
//! arrives in both languages.
//!
//! # Every shot in this repo flies entirely above the table
//!
//! A 7 iron at 90.7 mph and 5991 rpm launches at `S = 0.330` against a published ceiling of 0.284,
//! and `S` climbs through a flight as the ball slows faster than the spin decays. So
//! [`AeroCoefficients::clamped`] is true for the whole path on four of the five corpus flights, and
//! `crates/analysis/tests/flight.rs` gates that as a fact rather than trusting it. Clamping is the
//! deliberate choice over linear extrapolation — Smits and Smith (1994) measured out to `S = 1.4`
//! and report lift *saturating* — but it is still an extrapolation, which is what the flag says.

use std::sync::OnceLock;

use serde::Deserialize;

/// ADR-032 §5: one copy on disk, read from the Python package path at compile time. The third
/// artifact to cross this way, after `ranges.json` and the two trajectory bases.
const FLIGHT_MODEL_JSON: &str =
    include_str!("../../../../src/golf_coach/analysis/benchmarks/flight_model_v1.json");

/// Where one block of the artifact came from. `SourceNote`. [M29 P28]
///
/// Required on every block, for Python's reason: a block only exists because something published
/// it, and a constant that cannot say what would make the file's whole claim (that none of this was
/// fitted here) unverifiable one block at a time.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct SourceNote {
    pub citation: String,
    pub url: String,
    pub note: String,
    /// Only the coefficient block and the profile carry one: what a stored, unevaluated column is
    /// for.
    #[serde(default)]
    pub reynolds_note: Option<String>,
}

/// The ball the integrator flies: conforming mass and diameter.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct BallSpec {
    pub mass_kg: f64,
    pub diameter_m: f64,
    pub source: SourceNote,
}

impl BallSpec {
    pub fn radius_m(&self) -> f64 {
        self.diameter_m / 2.0
    }

    /// The reference area both coefficients are defined against.
    ///
    /// `r * r` where the Python writes `self.radius_m**2`. CPython's `**` on a float exponent of 2
    /// goes through `pow`, which every libm this runs on special-cases to one multiply; a multiply
    /// is also the only correctly-rounded answer available, so the two cannot differ by more than
    /// the multiply already does, which is zero.
    pub fn frontal_area_m2(&self) -> f64 {
        std::f64::consts::PI * self.radius_m() * self.radius_m()
    }
}

/// The air the ball flies through — one air state, not a law for generating them.
///
/// The committed instance is ISO 2533 at sea level, which is where every shot on disk was hit.
/// [`AtmosphereProfile`] is what moves it somewhere else. Only the density reaches the flight; the
/// name and temperature reach `golf-lab flight`'s `air` line, and the viscosity and pressure are
/// carried so an atmosphere generated at altitude is the whole air state Python's is.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Atmosphere {
    pub name: String,
    pub density_kg_m3: f64,
    pub kinematic_viscosity_m2_s: f64,
    pub temperature_k: f64,
    pub pressure_pa: f64,
    pub source: SourceNote,
}

/// An altitude outside the layer [`AtmosphereProfile`] transcribes: Python's `ValueError`.
/// [M29 P28]
///
/// R8's raising side rather than a golfer-facing refusal, for `simulate_flight`'s reason: a caller
/// asking for 30 km has a units bug, not a golf course. A value rather than a panic because
/// `golf-lab flight --altitude` takes the number from a person, and the script prints it as
/// `No flight: <message>` with exit 2.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AltitudeOutsideLayer {
    /// `str(exc)`, exactly as Python's would read.
    pub message: String,
}

impl std::fmt::Display for AltitudeOutsideLayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for AltitudeOutsideLayer {}

/// How the air changes with altitude: ISO 2533's lowest layer, written as ratios.
/// `AtmosphereProfile`. [M29 P28]
///
/// **Ratios against the committed sea-level block rather than the standard's three absolute
/// formulas**, which is Python's one design decision in the class: the committed row is
/// self-consistent in density and rounded to five figures in viscosity, so an absolute evaluation at
/// zero altitude would hand back an air state subtly unlike the default. Ratios return all four
/// numbers **to the bit** at zero, by construction, which a test below pins. Only `name` and
/// `source` differ there, on purpose: an air state that was generated says which law generated it.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct AtmosphereProfile {
    pub name: String,
    pub lapse_rate_k_per_m: f64,
    pub gas_constant_j_kg_k: f64,
    /// Stored and evaluated by nothing, here as in Python: the ratios need only `sutherland_s_k`.
    pub sutherland_beta_kg_m_s_sqrt_k: f64,
    pub sutherland_s_k: f64,
    /// The layer's own domain, not a judgement about golf.
    pub altitude_min_m: f64,
    pub altitude_max_m: f64,
    pub source: SourceNote,
}

impl AtmosphereProfile {
    /// `g/(L*R)`, about 5.2559. Derived rather than stored, because it is derived.
    pub fn pressure_exponent(&self, gravity_m_s2: f64) -> f64 {
        gravity_m_s2 / (self.lapse_rate_k_per_m * self.gas_constant_j_kg_k)
    }

    /// Sutherland's law as a ratio, `(T/T0)**1.5 * (T0+S)/(T+S)`, for dynamic viscosity.
    ///
    /// `powf` where Python writes `**`: both are the platform's C `pow` on a positive base, which is
    /// what the `repr`s in the tests confirm to the bit. The products and the division keep Python's
    /// left-to-right order.
    pub fn viscosity_ratio(&self, temperature_k: f64, base_temperature_k: f64) -> f64 {
        let s = self.sutherland_s_k;
        (temperature_k / base_temperature_k).powf(1.5) * (base_temperature_k + s)
            / (temperature_k + s)
    }

    /// The air at `altitude_m`, anchored on `base` as the standard's zero row. `at_altitude`.
    ///
    /// Refuses outside the layer this profile transcribes, a NaN included, as Python's chained
    /// comparison does. The generated air state cites the profile that generated it, so a flight
    /// can always say where its atmosphere came from.
    pub fn at_altitude(
        &self,
        base: &Atmosphere,
        altitude_m: f64,
        gravity_m_s2: f64,
    ) -> Result<Atmosphere, AltitudeOutsideLayer> {
        if !(self.altitude_min_m <= altitude_m && altitude_m <= self.altitude_max_m) {
            return Err(AltitudeOutsideLayer {
                message: format!(
                    "altitude {} m is outside {} ({} to {} m), where the lapse rate this profile \
                     carries is not the standard's",
                    pyfmt::repr(altitude_m),
                    self.name,
                    pyfmt::repr(self.altitude_min_m),
                    pyfmt::repr(self.altitude_max_m)
                ),
            });
        }

        let temperature_ratio = 1.0 - self.lapse_rate_k_per_m * altitude_m / base.temperature_k;
        let temperature_k = base.temperature_k * temperature_ratio;
        let exponent = self.pressure_exponent(gravity_m_s2);
        // Density carries `exponent - 1` because `rho = p/(R*T)` and the `T` in the denominator is
        // itself the ratio: the pressure drop divided by the temperature drop.
        let density_ratio = temperature_ratio.powf(exponent - 1.0);
        Ok(Atmosphere {
            name: format!("{}_{}m", self.name, pyfmt::g(altitude_m)),
            density_kg_m3: base.density_kg_m3 * density_ratio,
            kinematic_viscosity_m2_s: base.kinematic_viscosity_m2_s
                * self.viscosity_ratio(temperature_k, base.temperature_k)
                / density_ratio,
            temperature_k,
            pressure_pa: base.pressure_pa * temperature_ratio.powf(exponent),
            source: self.source.clone(),
        })
    }
}

/// Exponential spin-down at a fixed fractional rate.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct SpinDecay {
    pub rate_per_s: f64,
    pub source: SourceNote,
}

impl SpinDecay {
    /// One rate for every speed and spin ratio, which is the simplification worth naming.
    ///
    /// Smits and Smith (1994) measured decay as a function of both. The 4%/s figure in the artifact
    /// is the one Lyu et al. used for a driver trajectory, and a 4.5 s iron flight loses ~17% of
    /// its spin under it — enough to matter to the carry, not enough that the shape of the decay
    /// law dominates the answer.
    pub fn spin_after(&self, initial_rad_s: f64, seconds: f64) -> f64 {
        initial_rad_s * (-self.rate_per_s * seconds).exp()
    }
}

/// One measured point: a spin ratio and the two seam orientations the source reports separately.
///
/// `pp` and `ph` are kept apart rather than pre-averaged so the artifact stays a faithful
/// transcription of the published table — a stored average is a number no source can be checked
/// against. `reynolds` is on disk and not here; the module doc says why.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct AeroRow {
    pub spin_ratio: f64,
    pub cl_pp: f64,
    pub cd_pp: f64,
    pub cl_ph: f64,
    pub cd_ph: f64,
}

impl AeroRow {
    pub fn cl(&self) -> f64 {
        (self.cl_pp + self.cl_ph) / 2.0
    }

    pub fn cd(&self) -> f64 {
        (self.cd_pp + self.cd_ph) / 2.0
    }
}

/// A lift and drag coefficient for one spin ratio, and whether the table actually covered it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AeroCoefficients {
    pub spin_ratio: f64,
    pub cl: f64,
    pub cd: f64,
    /// True when the request fell outside the measured range and the nearest end row was held.
    /// Not a failure and not a refusal — but it is the difference between a read and an
    /// extrapolation, and on every shot stored so far it is true for the whole flight.
    pub clamped: bool,
}

/// The measured coefficient rows, ascending in spin ratio, with the rule for reading them.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct AeroTable {
    pub source: SourceNote,
    pub rows: Vec<AeroRow>,
}

impl AeroTable {
    pub fn spin_ratio_min(&self) -> f64 {
        self.rows[0].spin_ratio
    }

    pub fn spin_ratio_max(&self) -> f64 {
        self.rows[self.rows.len() - 1].spin_ratio
    }

    /// Linear interpolation between measured rows; the end rows held beyond them.
    ///
    /// Held rather than extended: see the module doc. A negative spin ratio is not meaningful here
    /// and clamps to the first row like any other under-range value — the integrator's own sign
    /// handling is where a top-spun ball belongs, not this lookup.
    ///
    /// **The trailing fall-through is unreachable while the rows ascend**, which the loader checks,
    /// and it is kept rather than replaced by a panic for the Python's reason: a data defect should
    /// not take down a flight mid-integration, and the loader is where it gets caught.
    pub fn coefficients_for(&self, spin_ratio: f64) -> AeroCoefficients {
        if spin_ratio <= self.spin_ratio_min() {
            let first = &self.rows[0];
            return AeroCoefficients {
                spin_ratio,
                cl: first.cl(),
                cd: first.cd(),
                clamped: true,
            };
        }
        if spin_ratio >= self.spin_ratio_max() {
            let last = &self.rows[self.rows.len() - 1];
            return AeroCoefficients {
                spin_ratio,
                cl: last.cl(),
                cd: last.cd(),
                clamped: true,
            };
        }

        for pair in self.rows.windows(2) {
            let (low, high) = (&pair[0], &pair[1]);
            if low.spin_ratio <= spin_ratio && spin_ratio <= high.spin_ratio {
                let span = (spin_ratio - low.spin_ratio) / (high.spin_ratio - low.spin_ratio);
                return AeroCoefficients {
                    spin_ratio,
                    cl: low.cl() + span * (high.cl() - low.cl()),
                    cd: low.cd() + span * (high.cd() - low.cd()),
                    clamped: false,
                };
            }
        }
        let last = &self.rows[self.rows.len() - 1];
        AeroCoefficients {
            spin_ratio,
            cl: last.cl(),
            cd: last.cd(),
            clamped: true,
        }
    }
}

/// Everything [`crate::flight`] needs and cannot derive.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct FlightModel {
    pub gravity_m_s2: f64,
    pub ball: BallSpec,
    pub atmosphere: Atmosphere,
    pub atmosphere_profile: AtmosphereProfile,
    pub spin_decay: SpinDecay,
    pub coefficients: AeroTable,
}

impl FlightModel {
    /// This model with its air swapped for the air at `altitude_m`: the what-if. `at_altitude`.
    /// [M29 P28]
    ///
    /// A whole model rather than a loose atmosphere, because [`crate::flight::simulate_flight`]
    /// already takes a model and an altitude beside it would be a second way to say one thing. The
    /// committed model is never touched: this clones it and swaps one block, as `model_copy` does.
    /// At zero altitude the returned model flies a bit-identical flight, which is the profile's
    /// construction and not luck.
    pub fn at_altitude(&self, altitude_m: f64) -> Result<FlightModel, AltitudeOutsideLayer> {
        let atmosphere =
            self.atmosphere_profile
                .at_altitude(&self.atmosphere, altitude_m, self.gravity_m_s2)?;
        Ok(FlightModel {
            atmosphere,
            ..self.clone()
        })
    }

    /// `S = wR/v`, or `None` at a standstill.
    ///
    /// `None` rather than infinity: a ball with no speed has no aerodynamic coefficient to look up,
    /// and returning a number for it would put a division by zero one step further from where it
    /// happened.
    pub fn spin_ratio(&self, speed_m_s: f64, spin_rad_s: f64) -> Option<f64> {
        if speed_m_s <= 0.0 {
            return None;
        }
        Some(spin_rad_s * self.ball.radius_m() / speed_m_s)
    }

    /// The coefficients for one instant of a flight, or `None` where the spin ratio is not.
    pub fn coefficients_at(&self, speed_m_s: f64, spin_rad_s: f64) -> Option<AeroCoefficients> {
        let ratio = self.spin_ratio(speed_m_s, spin_rad_s)?;
        Some(self.coefficients.coefficients_for(ratio))
    }
}

/// Provenance for the artifact as a whole; the per-block citations live on the blocks.
/// `FlightDatasetInfo`. [M29 P28]
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct FlightDatasetInfo {
    pub name: String,
    pub kind: String,
    pub assembled_on: String,
    pub assembled_by: String,
    /// Pinned false. The day something here *is* fitted, ADR-022's fitting-stage rules apply.
    pub fitted: bool,
    pub license_note: String,
    pub note: String,
}

#[derive(Deserialize)]
struct ModelFile {
    dataset: FlightDatasetInfo,
    model: FlightModel,
}

/// Python's `@lru_cache(maxsize=1)` on `_load`, which here is a parse that happens once.
///
/// The two load-time checks come across with it, and they are not ceremony: an out-of-order row
/// does not raise anywhere downstream — it silently returns the wrong coefficient for a real
/// flight, through the fall-through [`AeroTable::coefficients_for`] keeps for exactly that reason.
/// A panic rather than Python's `ValueError` because the bytes are in the binary: there is no
/// runtime input that could make this fail, so it is a build-time fact stated at first use.
fn loaded() -> &'static ModelFile {
    static FILE: OnceLock<ModelFile> = OnceLock::new();
    FILE.get_or_init(|| {
        let parsed: ModelFile = serde_json::from_str(FLIGHT_MODEL_JSON)
            .expect("flight_model_v1.json ships in this crate and parses");
        let ratios: Vec<f64> = parsed
            .model
            .coefficients
            .rows
            .iter()
            .map(|row| row.spin_ratio)
            .collect();
        assert!(
            ratios.len() >= 2,
            "flight_model_v1.json: a coefficient table needs at least two rows to read"
        );
        assert!(
            ratios.windows(2).all(|pair| pair[0] < pair[1]),
            "flight_model_v1.json: coefficient rows must ascend in spin_ratio"
        );
        parsed
    })
}

/// Provenance of the published constants this model is assembled from. `flight_dataset_info`.
/// [M29 P28]
pub fn flight_dataset_info() -> &'static FlightDatasetInfo {
    &loaded().dataset
}

/// The ball, the air, the spin decay and the coefficient table. `load_flight_model`.
pub fn load_flight_model() -> &'static FlightModel {
    &loaded().model
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The seven numbers the flight reads, against the artifact as committed. A `const` would be
    /// the second copy ADR-032 §5 exists to prevent, so these are asserted rather than declared.
    #[test]
    fn the_committed_constants_are_the_ones_the_flight_reads() {
        let model = load_flight_model();
        assert_eq!(model.gravity_m_s2, 9.80665);
        assert_eq!(model.ball.mass_kg, 0.04593);
        assert_eq!(model.ball.diameter_m, 0.04267);
        assert_eq!(model.atmosphere.density_kg_m3, 1.225);
        assert_eq!(model.spin_decay.rate_per_s, 0.04);
        assert_eq!(model.coefficients.rows.len(), 8);
        assert_eq!(model.coefficients.spin_ratio_min(), 0.085);
        assert_eq!(model.coefficients.spin_ratio_max(), 0.284);
    }

    /// The two orientations average, and the average is computed rather than stored.
    #[test]
    fn a_row_averages_its_two_seam_orientations() {
        let row = &load_flight_model().coefficients.rows[0];
        assert_eq!(row.cl(), (0.144 + 0.138) / 2.0);
        assert_eq!(row.cd(), (0.219 + 0.217) / 2.0);
    }

    /// Both ends hold, and say so. This is the branch every corpus flight takes.
    #[test]
    fn beyond_either_end_the_row_is_held_and_flagged() {
        let table = &load_flight_model().coefficients;
        let under = table.coefficients_for(0.01);
        assert!(under.clamped);
        assert_eq!(under.cl, table.rows[0].cl());
        let over = table.coefficients_for(0.45);
        assert!(over.clamped);
        assert_eq!(over.cl, table.rows[7].cl());
        // A negative ratio clamps low like any other under-range value rather than raising.
        assert_eq!(table.coefficients_for(-1.0).cl, table.rows[0].cl());
        // The end ratios themselves are clamped, not interpolated — `<=` and `>=` in the Python.
        assert!(table.coefficients_for(0.085).clamped);
        assert!(table.coefficients_for(0.284).clamped);
    }

    /// Between two rows the read is linear and is *not* flagged, which is the only way `clamped`
    /// can come back false and the reason one corpus flight reports 0 clamped points.
    #[test]
    fn between_two_rows_the_read_is_linear_and_unflagged() {
        let table = &load_flight_model().coefficients;
        let (low, high) = (&table.rows[0], &table.rows[1]);
        let mid = (low.spin_ratio + high.spin_ratio) / 2.0;
        let got = table.coefficients_for(mid);
        assert!(!got.clamped);
        let span = (mid - low.spin_ratio) / (high.spin_ratio - low.spin_ratio);
        assert_eq!(got.cl, low.cl() + span * (high.cl() - low.cl()));
        assert_eq!(got.cd, low.cd() + span * (high.cd() - low.cd()));
    }

    /// A standstill has no spin ratio, which is what stops a division by zero reaching the table.
    #[test]
    fn a_standstill_has_no_spin_ratio_and_no_coefficients() {
        let model = load_flight_model();
        assert_eq!(model.spin_ratio(0.0, 600.0), None);
        assert_eq!(model.spin_ratio(-1.0, 600.0), None);
        assert!(model.coefficients_at(0.0, 600.0).is_none());
        assert_eq!(
            model.spin_ratio(40.0, 600.0),
            Some(600.0 * model.ball.radius_m() / 40.0)
        );
    }

    /// The reference area, and the one place a `**2` had to be read rather than transcribed.
    #[test]
    fn the_frontal_area_is_pi_r_squared() {
        let ball = &load_flight_model().ball;
        assert_eq!(ball.radius_m(), 0.04267 / 2.0);
        assert_eq!(
            ball.frontal_area_m2(),
            std::f64::consts::PI * (0.04267 / 2.0f64).powi(2)
        );
    }

    /// The four numbers at four altitudes, each to frozen Python's `repr` (printed at M29 P28 off
    /// `load_flight_model().at_altitude(a).atmosphere`): Denver, the Dead Sea's half metre, and both
    /// edges of the layer. To the bit, so `powf` is CPython's `**` here.
    // Every literal here is CPython's `repr` and is the answer being compared, so clippy's
    // truncation advice would rewrite the test's subject, as `pyfmt`'s `hypot` test says.
    #[allow(clippy::excessive_precision)]
    #[test]
    fn the_profile_reproduces_frozen_python_at_every_altitude_it_was_asked() {
        let model = load_flight_model();
        let cases: [(f64, &str, [f64; 4]); 4] = [
            (
                1609.0,
                "isa_troposphere_1609m",
                [
                    1.0466549285466333,
                    1.6609563581519225e-05,
                    277.69149999999996,
                    83431.10757561972,
                ],
            ),
            (
                -390.5,
                "isa_troposphere_-390.5m",
                [
                    1.271587093556052,
                    1.4167954746052686e-05,
                    290.68825,
                    106104.91172907004,
                ],
            ),
            (
                11000.0,
                "isa_troposphere_11000m",
                [
                    0.36391764271731925,
                    3.906364562252669e-05,
                    216.64999999999998,
                    22632.040095007793,
                ],
            ),
            (
                -2000.0,
                "isa_troposphere_-2000m",
                [
                    1.4780761390172505,
                    1.252584071453698e-05,
                    301.15,
                    127773.73012293251,
                ],
            ),
        ];
        for (altitude, name, [density, viscosity, temperature, pressure]) in cases {
            let air = model.at_altitude(altitude).unwrap().atmosphere;
            assert_eq!(air.name, name);
            assert_eq!(air.density_kg_m3, density, "{name}");
            assert_eq!(air.kinematic_viscosity_m2_s, viscosity, "{name}");
            assert_eq!(air.temperature_k, temperature, "{name}");
            assert_eq!(air.pressure_pa, pressure, "{name}");
            assert_eq!(air.source, model.atmosphere_profile.source);
        }
        assert_eq!(
            model
                .atmosphere_profile
                .pressure_exponent(model.gravity_m_s2),
            5.255879812716677
        );
    }

    /// `test_the_profile_at_zero_altitude_returns_the_committed_air_to_the_bit`: the ratios' whole
    /// point. Only the name and the source say the air was generated.
    #[test]
    fn the_profile_at_zero_altitude_returns_the_committed_air_to_the_bit() {
        let model = load_flight_model();
        let at_zero = model.at_altitude(0.0).unwrap();
        let (sea, zero) = (&model.atmosphere, &at_zero.atmosphere);
        assert_eq!(zero.density_kg_m3, sea.density_kg_m3);
        assert_eq!(zero.kinematic_viscosity_m2_s, sea.kinematic_viscosity_m2_s);
        assert_eq!(zero.temperature_k, sea.temperature_k);
        assert_eq!(zero.pressure_pa, sea.pressure_pa);
        assert_eq!(zero.name, "isa_troposphere_0m");
        assert_ne!(zero.source, sea.source);
        // Every other block is the committed one, and the committed model is untouched.
        assert_eq!(at_zero.ball, model.ball);
        assert_eq!(at_zero.coefficients, model.coefficients);
        assert_eq!(at_zero.spin_decay, model.spin_decay);
        assert_eq!(at_zero.atmosphere_profile, model.atmosphere_profile);
        assert_eq!(load_flight_model().atmosphere.name, "isa_sea_level");
    }

    /// `test_an_altitude_outside_the_layer_raises_rather_than_extrapolating`, in Python's words
    /// (printed off the frozen module), a NaN included: the chained comparison is false for it.
    #[test]
    fn an_altitude_outside_the_layer_is_refused_in_python_s_words() {
        let model = load_flight_model();
        let words = |altitude: f64| model.at_altitude(altitude).unwrap_err().message;
        assert_eq!(
            words(20000.0),
            "altitude 20000.0 m is outside isa_troposphere (-2000.0 to 11000.0 m), where the \
             lapse rate this profile carries is not the standard's"
        );
        assert_eq!(
            words(f64::NAN),
            "altitude nan m is outside isa_troposphere (-2000.0 to 11000.0 m), where the lapse \
             rate this profile carries is not the standard's"
        );
        assert!(model.at_altitude(-2000.1).is_err());
    }

    /// The dataset block `--verbose` prints, and the one claim the artifact makes about itself.
    #[test]
    fn the_dataset_says_nothing_was_fitted_and_every_block_cites_a_source() {
        let info = flight_dataset_info();
        assert_eq!(info.name, "flight_model_v1");
        assert_eq!(info.assembled_on, "2026-09-05");
        assert_eq!(info.assembled_by, "M15 P2");
        assert!(!info.fitted);
        let model = load_flight_model();
        for source in [
            &model.ball.source,
            &model.atmosphere.source,
            &model.atmosphere_profile.source,
            &model.spin_decay.source,
            &model.coefficients.source,
        ] {
            assert!(!source.citation.is_empty() && source.url.starts_with("https://"));
        }
    }

    /// 4%/s, compounding — and against `exp` rather than a series, because the Python's is `exp`.
    #[test]
    fn the_spin_decays_exponentially_against_absolute_time() {
        let decay = &load_flight_model().spin_decay;
        assert_eq!(decay.spin_after(100.0, 0.0), 100.0);
        assert_eq!(decay.spin_after(100.0, 1.0), 100.0 * (-0.04f64).exp());
        // A 4.5 s iron flight loses about 17%, which is the figure the doc quotes.
        let left = decay.spin_after(1.0, 4.5);
        assert!((0.82..0.84).contains(&left), "{left} left after 4.5 s");
    }
}
