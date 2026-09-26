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
//! So none of it is here, on the rule [`super`] set for `dataset_info()` and
//! [`super::store`] for `source_date`: a port of a function no committed vector can gate is a
//! second implementation with no oracle. The same goes for `Atmosphere.reynolds_for`,
//! `AeroRow.reynolds`, `AeroTable`'s three prose fields and `SpinDecay.kind` — all on disk, all
//! read by nothing the engine calls. serde ignores unknown keys, so leaving them out of the structs
//! costs nothing and states the reach honestly.
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

/// The ball the integrator flies: conforming mass and diameter.
#[derive(Debug, Clone, Deserialize)]
pub struct BallSpec {
    pub mass_kg: f64,
    pub diameter_m: f64,
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
/// The committed instance is ISO 2533 at sea level, which is where every shot on disk was hit. The
/// law that moves it somewhere else is `AtmosphereProfile` and is not ported — see the module doc.
/// Only the density reaches the flight; the other three fields are on disk for the Reynolds helper
/// nothing calls.
#[derive(Debug, Clone, Deserialize)]
pub struct Atmosphere {
    pub density_kg_m3: f64,
}

/// Exponential spin-down at a fixed fractional rate.
#[derive(Debug, Clone, Deserialize)]
pub struct SpinDecay {
    pub rate_per_s: f64,
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
#[derive(Debug, Clone, Deserialize)]
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
#[derive(Debug, Clone, Deserialize)]
pub struct AeroTable {
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
#[derive(Debug, Clone, Deserialize)]
pub struct FlightModel {
    pub gravity_m_s2: f64,
    pub ball: BallSpec,
    pub atmosphere: Atmosphere,
    pub spin_decay: SpinDecay,
    pub coefficients: AeroTable,
}

impl FlightModel {
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

#[derive(Deserialize)]
struct ModelFile {
    model: FlightModel,
}

/// Python's `@lru_cache(maxsize=1)` on `_load`, which here is a parse that happens once.
///
/// The two load-time checks come across with it, and they are not ceremony: an out-of-order row
/// does not raise anywhere downstream — it silently returns the wrong coefficient for a real
/// flight, through the fall-through [`AeroTable::coefficients_for`] keeps for exactly that reason.
/// A panic rather than Python's `ValueError` because the bytes are in the binary: there is no
/// runtime input that could make this fail, so it is a build-time fact stated at first use.
pub fn load_flight_model() -> &'static FlightModel {
    static MODEL: OnceLock<FlightModel> = OnceLock::new();
    MODEL.get_or_init(|| {
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
        parsed.model
    })
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
