//! The whole `flight` stage, against `spec/vectors/stages/`. [M22 P8, P8b]
//!
//! The **seventh and last** stage to acquire a runner, and it arrived in two halves: P8 ported the
//! integrator and the spin solve, P8b the resolution and the join above them. Four gates:
//!
//! | gate | committed input | committed answer | vectors |
//! |---|---|---|---|
//! | [`the_integrator_reproduces_every_committed_flight`] | `flown.resolved.launch` | `flown.flight` — the whole path | the 5 that flew |
//! | [`the_carry_window_reproduces_every_committed_solve`] | `…spin.solution.window.launch` | `…spin.solution.window` | the 11 that solved |
//! | [`the_spin_solve_reproduces_every_committed_case`] | `window.launch` + `target_carry_yds` | `case`, `rising_rpm`, `falling_rpm` | the 11 that solved |
//! | [`the_resolution_conforms_on_every_committed_shot`] | the **engine** vector's shot, loft and handedness | everything else `flown` holds, and `unscored` | all 15 |
//!
//! **That the stage records a function's input beside its output is what makes the first three
//! possible**, and it is worth naming because P7 found the shape it does *not* cover: a stage records
//! a call's inputs and its output, never the lines of the caller between them. Here it works in the
//! port's favour twice over — the integrator can be gated without the inference chain above it, and
//! the solve without the loft prior that reads it.
//!
//! **The fourth gate is that property used up.** `fly_shot` *is* the caller, so its input is not in
//! the stage at all and has to come from the engine vector — which is why this file, alone among the
//! seven runners, went five gates without reading one. The two ends meeting is what makes
//! `resolved.launch` a checked answer rather than a shared assumption: a port that built the wrong
//! `UnspunLaunch` would pass the first three cleanly.
//!
//! # What the coverage actually is, measured
//!
//! Only **5 of the 15** corpus vectors produce a flight at all: the four whose spin the screen
//! printed (`2026-08-07-aaron1-1`, `2026-08-09-2`, both 2026-08-10 shots) and one whose spin was
//! inferred (`2026-08-23-4`). The other ten refuse before the integrator is reached — `above_peak`,
//! `between_plateaus` or `no_club_loft` — so ten of the fifteen gate the **solve** and never the
//! flight. No synthetic vector carries a shot, so this whole file is the corpus half.
//!
//! The fourth gate's own coverage is thirteen distinct sentences across the fifteen, and what it
//! misses is in `flight_infer`'s module doc: no left-handed shot, no printed axis without a golfer,
//! and no loft anywhere near [`analysis::flight_infer::LOFT_FLOOR_FOR_THE_FALLING_BRANCH_DEG`] — all
//! five lofts in `spec/` are 30.5°.
//!
//! Four of those five flights are **fully clamped** — 905, 905, 905 and 949 points, every one of them
//! on a held end row of a table measured on a driver — and the fifth, `2026-08-23-4`, is the only
//! flight in `spec/` that reads an interpolated row, on all 919 of its points. So the committed set
//! covers both sides of [`AeroTable::coefficients_for`]'s branch, which is not something it was
//! designed to do.
//!
//! # The rules
//!
//! `docs/CONFORMANCE.md` §3's: floats within `RTOL = 1e-9`, and **bools and ints exactly**.
//! `clamped` is a bool on every one of the 4,583 points compared here, and `clamped_points` is an
//! int, so the coefficient branch is compared exactly while the arithmetic that feeds it is
//! compared within tolerance — which is the right way round, because a flipped clamp is a different
//! flight and not a different last bit.
//!
//! And then a census beside the tolerance, which is P8's other finding:
//! [`the_committed_flights_are_reproduced_to_the_bit`] counts how many of the 41,287 floats are
//! bit-identical rather than merely close. It is **100%**, and it took two corrections to get there
//! that `RTOL = 1e-9` was structurally unable to report — one in the port (`pyfmt::hypot`) and one in
//! how this file reads its own oracle (`serde_json`'s `float_roundtrip`). A tolerance six orders
//! above a ulp cannot tell a converged integration from a systematically wrong primitive; a census
//! can.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use analysis::benchmarks::flight_model::{load_flight_model, AeroTable};
use analysis::flight::{simulate_flight, LaunchConditions, DEFAULT_STEP_S};
use analysis::flight_measure::{flight_unscored, fly_shot, FlownShot};
use analysis::spin_solve::{carry_window, solve_spin_from_carry, UnspunLaunch};
use contracts::golfer::Handedness;
use contracts::shot::ShotData;
use flate2::read::GzDecoder;
use serde::Deserialize;

/// `docs/CONFORMANCE.md` §3's float rule, and the same constants the five sibling gates use.
const RTOL: f64 = 1e-9;
const ATOL: f64 = 1e-12;

#[derive(Deserialize)]
struct StageVector {
    id: String,
    provenance: Provenance,
    stages: Stages,
}

#[derive(Deserialize)]
struct Provenance {
    derived_from: String,
}

#[derive(Deserialize)]
struct Stages {
    flight: FlightStage,
}

#[derive(Deserialize)]
struct FlightStage {
    flown: Option<Flown>,
    /// `flight_unscored(flown)`, verbatim — the one or two entries `analyze_swing` appends to
    /// `SwingResult.unscored`. P8b's.
    unscored: Vec<RecordedUnscored>,
}

#[derive(Deserialize)]
struct RecordedUnscored {
    name: String,
    reason: String,
    detail: String,
}

#[derive(Deserialize)]
struct Flown {
    resolved: Option<Resolved>,
    flight: Option<RecordedFlight>,
    reason: Option<String>,
    detail: String,
}

#[derive(Deserialize)]
struct Resolved {
    launch: Option<RecordedLaunch>,
    spin: Option<RecordedSpin>,
    spin_source: Option<String>,
    axis: RecordedAxis,
    reason: Option<String>,
    detail: String,
}

/// `flight_infer.InferredSpinAxis`, all six fields. P8b's whole first half.
#[derive(Deserialize)]
struct RecordedAxis {
    spin_axis_deg: Option<f64>,
    source: Option<String>,
    reason: Option<String>,
    detail: String,
    curve_direction: Option<String>,
    screen_shape: Option<String>,
}

/// `flight_infer.InferredSpin`. P8 owned exactly one field, the `solution` underneath; the wrapper's
/// own `spin_rpm`, `reason` and `detail` are the loft prior's answer and are P8b's.
#[derive(Deserialize)]
struct RecordedSpin {
    solution: Option<RecordedSolution>,
    spin_rpm: Option<f64>,
    reason: Option<String>,
    detail: String,
}

/// One engine vector, of which this file reads the three arguments `fly_shot` takes.
#[derive(Deserialize)]
struct EngineVector {
    input: EngineInput,
}

#[derive(Deserialize)]
struct EngineInput {
    #[serde(default)]
    shot: Option<ShotData>,
    #[serde(default)]
    loft_deg: Option<f64>,
    #[serde(default)]
    handedness: Option<Handedness>,
}

#[derive(Deserialize)]
struct RecordedSolution {
    case: String,
    target_carry_yds: f64,
    window: RecordedWindow,
    rising_rpm: Option<f64>,
    falling_rpm: Option<f64>,
}

#[derive(Deserialize)]
struct RecordedWindow {
    launch: RecordedUnspun,
    low_plateau_yds: f64,
    low_plateau_max_rpm: f64,
    peak_yds: f64,
    peak_rpm: f64,
    high_plateau_yds: f64,
    high_plateau_min_rpm: f64,
}

#[derive(Deserialize)]
struct RecordedUnspun {
    ball_speed_mph: f64,
    launch_angle_deg: f64,
    launch_direction_deg: f64,
    spin_axis_deg: f64,
}

#[derive(Deserialize)]
struct RecordedLaunch {
    ball_speed_mph: f64,
    launch_angle_deg: f64,
    spin_rpm: f64,
    launch_direction_deg: f64,
    spin_axis_deg: f64,
}

#[derive(Deserialize)]
struct RecordedFlight {
    points: Vec<RecordedPoint>,
    carry_m: f64,
    curvature_m: f64,
    landing_offline_m: f64,
    apex_m: f64,
    flight_time_s: f64,
    descent_angle_deg: f64,
    clamped_points: usize,
    spin_ratio_min: f64,
    spin_ratio_max: f64,
}

#[derive(Deserialize)]
struct RecordedPoint {
    t_s: f64,
    x_m: f64,
    y_m: f64,
    z_m: f64,
    vx_m_s: f64,
    vy_m_s: f64,
    vz_m_s: f64,
    spin_rpm: f64,
    spin_ratio: f64,
    clamped: bool,
}

fn spec_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crates/analysis has a grandparent")
        .join("spec")
        .join("vectors")
}

fn read_json(path: &Path) -> String {
    let bytes = fs::read(path).unwrap_or_else(|e| panic!("read {path:?}: {e}"));
    if path.extension().is_some_and(|e| e == "gz") {
        let mut text = String::new();
        GzDecoder::new(&bytes[..])
            .read_to_string(&mut text)
            .unwrap_or_else(|e| panic!("gunzip {path:?}: {e}"));
        text
    } else {
        String::from_utf8(bytes).unwrap_or_else(|e| panic!("utf-8 {path:?}: {e}"))
    }
}

/// Every committed stage vector, sorted so a failure names a stable first offender.
///
/// The engine vector is *not* read here, unlike in the five sibling gates: everything this file
/// needs was recorded inside the `flight` stage itself, which is the property the module doc calls
/// the phase's luck.
fn stage_vectors() -> Vec<StageVector> {
    let root = spec_dir().join("stages");
    let mut paths: Vec<PathBuf> = ["synthetic", "corpus"]
        .iter()
        .flat_map(|half| {
            fs::read_dir(root.join(half))
                .unwrap_or_else(|e| panic!("list {:?}: {e}", root.join(half)))
                .map(|entry| entry.expect("a readable directory entry").path())
                .collect::<Vec<_>>()
        })
        .collect();
    paths.sort();
    assert!(!paths.is_empty(), "no stage vectors under {root:?}");
    paths
        .into_iter()
        .map(|path| {
            serde_json::from_str(&read_json(&path))
                .unwrap_or_else(|e| panic!("parse {path:?}: {e}"))
        })
        .collect()
}

fn float_differs(got: f64, want: f64) -> bool {
    !matches!(
        (got - want).abs().partial_cmp(&(ATOL + RTOL * want.abs())),
        Some(std::cmp::Ordering::Less | std::cmp::Ordering::Equal)
    )
}

fn check(out: &mut Vec<String>, id: &str, what: &str, got: f64, want: f64) {
    if float_differs(got, want) {
        out.push(format!("{id}: {what} {got} != {want}"));
    }
}

/// Every one of the ~4,500 committed flight points, and the nine numbers read off the path.
///
/// **The whole path rather than its summary**, which is the stage vector's own choice: M22 P1 kept
/// `flight`'s 919-point path where it dropped four of `smoothed`'s six fields, because the Flutter
/// shell draws it once `api/` retires. It also means a divergence is located — a wrong Magnus sign
/// shows up in the first fifty points rather than as a carry three yards out.
#[test]
fn the_integrator_reproduces_every_committed_flight() {
    let model = load_flight_model();
    let mut differences: Vec<String> = Vec::new();
    let mut flown_count = 0;

    for vector in stage_vectors() {
        let id = &vector.id;
        let Some(flown) = vector.stages.flight.flown.as_ref() else {
            continue;
        };
        let (Some(resolved), Some(want)) = (flown.resolved.as_ref(), flown.flight.as_ref()) else {
            continue;
        };
        let Some(launch) = resolved.launch.as_ref() else {
            // A refusal recorded its resolution and no flight. Ten of the fifteen are here, and
            // `the_stage_refuses_ten_of_fifteen_before_the_integrator` pins that count.
            continue;
        };
        flown_count += 1;

        let got = simulate_flight(
            &LaunchConditions {
                ball_speed_mph: launch.ball_speed_mph,
                launch_angle_deg: launch.launch_angle_deg,
                spin_rpm: launch.spin_rpm,
                launch_direction_deg: launch.launch_direction_deg,
                spin_axis_deg: launch.spin_axis_deg,
            },
            model,
            DEFAULT_STEP_S,
        )
        .unwrap_or_else(|e| panic!("{id}: the committed launch refused: {}", e.message));

        if got.points.len() != want.points.len() {
            differences.push(format!(
                "{id}: {} points against {}",
                got.points.len(),
                want.points.len()
            ));
            continue;
        }
        for (at, (got, want)) in got.points.iter().zip(&want.points).enumerate() {
            for (field, a, b) in [
                ("t_s", got.t_s, want.t_s),
                ("x_m", got.x_m, want.x_m),
                ("y_m", got.y_m, want.y_m),
                ("z_m", got.z_m, want.z_m),
                ("vx_m_s", got.vx_m_s, want.vx_m_s),
                ("vy_m_s", got.vy_m_s, want.vy_m_s),
                ("vz_m_s", got.vz_m_s, want.vz_m_s),
                ("spin_rpm", got.spin_rpm, want.spin_rpm),
                ("spin_ratio", got.spin_ratio, want.spin_ratio),
            ] {
                check(&mut differences, id, &format!("points[{at}].{field}"), a, b);
            }
            if got.clamped != want.clamped {
                differences.push(format!(
                    "{id}: points[{at}].clamped {} != {}",
                    got.clamped, want.clamped
                ));
            }
        }

        for (field, a, b) in [
            ("carry_m", got.carry_m, want.carry_m),
            ("curvature_m", got.curvature_m, want.curvature_m),
            (
                "landing_offline_m",
                got.landing_offline_m,
                want.landing_offline_m,
            ),
            ("apex_m", got.apex_m, want.apex_m),
            ("flight_time_s", got.flight_time_s, want.flight_time_s),
            (
                "descent_angle_deg",
                got.descent_angle_deg,
                want.descent_angle_deg,
            ),
            ("spin_ratio_min", got.spin_ratio_min, want.spin_ratio_min),
            ("spin_ratio_max", got.spin_ratio_max, want.spin_ratio_max),
        ] {
            check(&mut differences, id, field, a, b);
        }
        if got.clamped_points != want.clamped_points {
            differences.push(format!(
                "{id}: clamped_points {} != {}",
                got.clamped_points, want.clamped_points
            ));
        }
    }

    assert!(
        differences.is_empty(),
        "{} difference(s) across {flown_count} committed flights:\n{}",
        differences.len(),
        differences.join("\n")
    );
    assert_eq!(
        flown_count, 5,
        "five of the fifteen corpus vectors fly; the count is the coverage this gate has"
    );
    println!("flight: {flown_count} committed flights conform");
}

/// The carry-against-spin curve: five numbers per vector, each of them worth ~40 flights.
///
/// Gated separately from the solve above it because the window is measured on **every** shot that
/// reaches the inference, including the ten that then refuse. It is the module's real product —
/// a printed carry two yards above the peak and one sitting on the high plateau are entirely
/// different findings about the shot, and the case alone cannot tell them apart.
#[test]
fn the_carry_window_reproduces_every_committed_solve() {
    let model = load_flight_model();
    let mut differences: Vec<String> = Vec::new();
    let mut solved = 0;

    for (id, solution) in committed_solutions() {
        solved += 1;
        let want = &solution.window;
        let got = carry_window(&unspun(&want.launch), model, DEFAULT_STEP_S)
            .unwrap_or_else(|e| panic!("{id}: the committed window refused: {}", e.message));
        for (field, a, b) in [
            ("low_plateau_yds", got.low_plateau_yds, want.low_plateau_yds),
            (
                "low_plateau_max_rpm",
                got.low_plateau_max_rpm,
                want.low_plateau_max_rpm,
            ),
            ("peak_yds", got.peak_yds, want.peak_yds),
            ("peak_rpm", got.peak_rpm, want.peak_rpm),
            (
                "high_plateau_yds",
                got.high_plateau_yds,
                want.high_plateau_yds,
            ),
            (
                "high_plateau_min_rpm",
                got.high_plateau_min_rpm,
                want.high_plateau_min_rpm,
            ),
        ] {
            check(&mut differences, &id, &format!("window.{field}"), a, b);
        }
    }

    assert!(
        differences.is_empty(),
        "{} difference(s) across {solved} committed windows:\n{}",
        differences.len(),
        differences.join("\n")
    );
    assert_eq!(solved, 11, "eleven of the fifteen reach the spin solve");
    println!("flight: {solved} committed carry windows conform");
}

/// Which segment of the curve the printed carry landed on, and the spins at that point.
///
/// **The case is compared as a string and exactly**, because it is the part of the answer that
/// survives having no number: `docs/CONFORMANCE.md` §3 puts it on the exact side of the line, and
/// `SpinSolveCase` is a `StrEnum` in the Python precisely so the case outlives the rpm.
#[test]
fn the_spin_solve_reproduces_every_committed_case() {
    let model = load_flight_model();
    let mut differences: Vec<String> = Vec::new();
    let mut seen: Vec<String> = Vec::new();

    for (id, want) in committed_solutions() {
        let launch = unspun(&want.window.launch);
        // The window is re-measured rather than handed in, which is the path `flight_infer` takes:
        // it calls `solve_spin_from_carry` with no `window=` and lets it measure its own.
        let got =
            solve_spin_from_carry(&launch, want.target_carry_yds, model, DEFAULT_STEP_S, None)
                .unwrap_or_else(|e| panic!("{id}: the committed solve refused: {}", e.message));

        if got.case.as_str() != want.case {
            differences.push(format!(
                "{id}: case {:?} != {:?}",
                got.case.as_str(),
                want.case
            ));
        }
        seen.push(want.case.clone());
        check(
            &mut differences,
            &id,
            "target_carry_yds",
            got.target_carry_yds,
            want.target_carry_yds,
        );
        for (field, a, b) in [
            ("rising_rpm", got.rising_rpm, want.rising_rpm),
            ("falling_rpm", got.falling_rpm, want.falling_rpm),
        ] {
            match (a, b) {
                (Some(a), Some(b)) => check(&mut differences, &id, field, a, b),
                (None, None) => {}
                (a, b) => differences.push(format!("{id}: {field} {a:?} != {b:?}")),
            }
        }
    }

    assert!(
        differences.is_empty(),
        "{} difference(s):\n{}",
        differences.len(),
        differences.join("\n")
    );
    seen.sort();
    seen.dedup();
    println!("flight: the committed solves cover {seen:?}");
}

/// **Three of the seven cases are gated by the corpus and four are not**, which is the coverage
/// finding this phase owes the next one.
///
/// The eleven committed solves land on `above_peak`, `between_plateaus` and `two_branches` only.
/// `at_peak`, `on_high_plateau`, `on_low_plateau` and `below_floor` ship against
/// `spin_solve`'s own unit tests — and `below_floor` in particular is the case ADR-027 §Decision 4
/// did not know existed, so it is the one a reader is most likely to assume is covered. Recorded as
/// an assertion rather than a comment so the day a vector reaches a fourth case, this fails **with
/// the case in the message** and the answer is that the ladder has become gated.
#[test]
fn four_of_the_seven_solve_cases_reach_no_committed_answer() {
    let mut seen: Vec<String> = committed_solutions()
        .into_iter()
        .map(|(_, s)| s.case)
        .collect();
    seen.sort();
    seen.dedup();
    assert_eq!(
        seen,
        vec!["above_peak", "between_plateaus", "two_branches"],
        "the committed cases moved"
    );
}

/// Ten of the fifteen refuse before the integrator, and four of the five that fly are **fully
/// clamped for their whole path**. Both are properties of the corpus rather than of the port, and
/// both are what the coverage of the gates above actually is.
///
/// The fifth flight, `2026-08-23-4`, is the only one in `spec/` that reads an **interpolated**
/// coefficient row — 0 clamped points of 919 — so the committed set covers both sides of
/// `AeroTable::coefficients_for`'s branch. Nothing designed that; it is worth pinning before a
/// re-recorded corpus loses it.
#[test]
fn the_committed_flights_cover_both_sides_of_the_coefficient_clamp() {
    let table: &AeroTable = &load_flight_model().coefficients;
    let mut refused = 0;
    let mut fully_clamped = 0;
    let mut unclamped = 0;

    for vector in stage_vectors() {
        let Some(flown) = vector.stages.flight.flown.as_ref() else {
            continue;
        };
        match flown.flight.as_ref() {
            None => refused += 1,
            Some(flight) => {
                if flight.clamped_points == flight.points.len() {
                    fully_clamped += 1;
                    // Above the table's ceiling for every point, which is the sentence ADR-027's
                    // 2026-09-05 addendum requires beside any quoted agreement with HD Golf.
                    assert!(
                        flight.spin_ratio_min > table.spin_ratio_max(),
                        "{}: clamped throughout but S starts at {} against a {} ceiling",
                        vector.id,
                        flight.spin_ratio_min,
                        table.spin_ratio_max()
                    );
                } else {
                    assert_eq!(flight.clamped_points, 0, "{}: partly clamped", vector.id);
                    unclamped += 1;
                }
            }
        }
    }
    assert_eq!((refused, fully_clamped, unclamped), (10, 4, 1));
}

/// **How much of the committed answer this port reproduces to the bit**, which is the measurement
/// that found P8's portability edge and the one a tolerance is structurally unable to report.
///
/// Measured 2026-09-25 on Windows with `serde_json`'s `float_roundtrip`: **100.0%** — all 41,287
/// floats across the five flights, every point and every scalar, bit-identical to CPython's. Two
/// things had to be true for that, and neither was true when this gate was first written:
///
/// 1. `pyfmt::hypot` rather than a chained [`f64::hypot`]. With the chain, 4,149 values differed by
///    one ulp — **9.2%** — and one of them flipped a `clamped`.
/// 2. `serde_json`'s `float_roundtrip` feature. Without it the *oracle* was read one ulp off on some
///    17-digit literals (`"0.030000000000000002"` parsed back as `0.03`), so a bit-exact port could
///    not have shown as one. `Cargo.toml` carries the measurement.
///
/// **The floor is 99% rather than 100%, and that is deliberate.** `exp`, `cos`, `sin` and `atan2` are
/// the platform's libm on both sides, so a different target may legitimately move the last bit of a
/// long integration; `docs/CONFORMANCE.md` §3's `RTOL = 1e-9` remains the contract, and the gates
/// above are read under it. What 99% pins is the class of error a tolerance hides — a systematically
/// wrong primitive, which is what one ulp in `hypot` was.
#[test]
fn the_committed_flights_are_reproduced_to_the_bit() {
    let model = load_flight_model();
    let mut exact = 0usize;
    let mut total = 0usize;

    for vector in stage_vectors() {
        let Some(flown) = vector.stages.flight.flown.as_ref() else {
            continue;
        };
        let (Some(resolved), Some(want)) = (flown.resolved.as_ref(), flown.flight.as_ref()) else {
            continue;
        };
        let Some(launch) = resolved.launch.as_ref() else {
            continue;
        };
        let got = simulate_flight(
            &LaunchConditions {
                ball_speed_mph: launch.ball_speed_mph,
                launch_angle_deg: launch.launch_angle_deg,
                spin_rpm: launch.spin_rpm,
                launch_direction_deg: launch.launch_direction_deg,
                spin_axis_deg: launch.spin_axis_deg,
            },
            model,
            DEFAULT_STEP_S,
        )
        .expect("the committed launch flies");

        let mut census = |a: f64, b: f64| {
            total += 1;
            if a.to_bits() == b.to_bits() {
                exact += 1;
            }
        };
        for (got, want) in got.points.iter().zip(&want.points) {
            census(got.t_s, want.t_s);
            census(got.x_m, want.x_m);
            census(got.y_m, want.y_m);
            census(got.z_m, want.z_m);
            census(got.vx_m_s, want.vx_m_s);
            census(got.vy_m_s, want.vy_m_s);
            census(got.vz_m_s, want.vz_m_s);
            census(got.spin_rpm, want.spin_rpm);
            census(got.spin_ratio, want.spin_ratio);
        }
        census(got.carry_m, want.carry_m);
        census(got.curvature_m, want.curvature_m);
        census(got.landing_offline_m, want.landing_offline_m);
        census(got.apex_m, want.apex_m);
        census(got.flight_time_s, want.flight_time_s);
        census(got.descent_angle_deg, want.descent_angle_deg);
        census(got.spin_ratio_min, want.spin_ratio_min);
        census(got.spin_ratio_max, want.spin_ratio_max);
    }

    let share = exact as f64 / total as f64;
    println!(
        "flight: {exact} of {total} floats bit-exact ({:.4}%)",
        share * 100.0
    );
    assert!(
        share >= 0.99,
        "only {exact} of {total} floats are bit-exact ({:.2}%) — one ulp in a shared primitive \
         looks exactly like this",
        share * 100.0
    );
}

/// **The whole `flight` stage from its caller's end**, on all twenty-one vectors. [M22 P8b]
///
/// The three gates above each hand a committed *intermediate* to the function under it — a recorded
/// `LaunchConditions` to the integrator, a recorded `UnspunLaunch` to the window and the solve. This
/// one starts where `analyze_swing` starts: the shot, the loft and the handedness off the **engine**
/// vector, through `fly_shot`, and every field of the recorded `flown` compared on the way out.
///
/// That is the shape P7 found a per-stage gate is structurally bad at: a stage records a call's
/// inputs and its output and never the lines of the caller between them. Here the caller *is* the
/// thing being ported, so the inputs have to come from outside the stage — and the two ends meeting
/// is what makes `resolved.launch` an answer rather than an assumption. A port whose
/// `flight_for_shot` built the wrong `UnspunLaunch` would pass all three gates above and fail here.
///
/// What it compares: `resolved.launch` and its five numbers, `spin_source`, the `InferredSpin`
/// wrapper's `spin_rpm`/`reason`/`detail`, all six `InferredSpinAxis` fields, `flown.reason`,
/// `flown.detail`, and the `flight_unscored` list. Every one of those but the numbers is a **string
/// or an enum**, so §3 compares them byte for byte — thirteen distinct sentences across the fifteen.
#[test]
fn the_resolution_conforms_on_every_committed_shot() {
    let model = load_flight_model();
    let mut differences: Vec<String> = Vec::new();
    let mut resolved_count = 0;

    for (stage, engine) in stage_vectors_with_inputs() {
        let id = &stage.id;
        let Some(shot) = engine.input.shot.as_ref() else {
            assert!(
                stage.stages.flight.flown.is_none(),
                "{id}: no shot on the input and a flight recorded anyway"
            );
            assert!(stage.stages.flight.unscored.is_empty(), "{id}");
            continue;
        };
        let want = stage
            .stages
            .flight
            .flown
            .as_ref()
            .unwrap_or_else(|| panic!("{id}: a shot on the input and no flight recorded"));
        resolved_count += 1;

        let got = fly_shot(
            shot,
            engine.input.loft_deg,
            engine.input.handedness,
            model,
            DEFAULT_STEP_S,
        );
        compare_flown(id, &got, want, &mut differences);
        compare_unscored(id, &got, &stage.stages.flight.unscored, &mut differences);
    }

    assert!(
        differences.is_empty(),
        "{} difference(s) across {resolved_count} committed resolutions:\n{}",
        differences.len(),
        differences.join("\n")
    );
    assert_eq!(
        resolved_count, 15,
        "all fifteen corpus vectors carry a shot and none of the six synthetic ones does"
    );
    println!("flight: {resolved_count} committed resolutions conform");
}

/// Every field of a `FlownShot` bar the path, which the three gates above already own.
fn compare_flown(id: &str, got: &FlownShot, want: &Flown, out: &mut Vec<String>) {
    string(
        out,
        id,
        "reason",
        got.reason.map(reason_name),
        want.reason.as_deref(),
    );
    string(out, id, "detail", Some(&got.detail), Some(&want.detail));

    let (Some(got), Some(want)) = (got.resolved.as_ref(), want.resolved.as_ref()) else {
        if got.resolved.is_some() != want.resolved.is_some() {
            out.push(format!("{id}: resolved present/absent disagrees"));
        }
        return;
    };

    // The resolution's own two, which are *not* the flown shot's: on a refusal they are the same
    // strings, and on an unflyable parse they are not — which is the case `resolved` is `None` for.
    string(
        out,
        id,
        "resolved.reason",
        got.reason.map(reason_name),
        want.reason.as_deref(),
    );
    string(
        out,
        id,
        "resolved.detail",
        Some(&got.detail),
        Some(&want.detail),
    );
    string(
        out,
        id,
        "resolved.spin_source",
        got.spin_source.map(|s| s.as_str()),
        want.spin_source.as_deref(),
    );

    let axis = &got.axis;
    let wanted = &want.axis;
    string(
        out,
        id,
        "axis.detail",
        Some(&axis.detail),
        Some(&wanted.detail),
    );
    string(
        out,
        id,
        "axis.reason",
        axis.reason.map(reason_name),
        wanted.reason.as_deref(),
    );
    string(
        out,
        id,
        "axis.source",
        axis.source.map(|s| s.as_str()),
        wanted.source.as_deref(),
    );
    string(
        out,
        id,
        "axis.curve_direction",
        axis.curve_direction.map(shape_name),
        wanted.curve_direction.as_deref(),
    );
    string(
        out,
        id,
        "axis.screen_shape",
        axis.screen_shape.map(shape_name),
        wanted.screen_shape.as_deref(),
    );
    optional(
        out,
        id,
        "axis.spin_axis_deg",
        axis.spin_axis_deg,
        wanted.spin_axis_deg,
    );

    match (got.spin.as_ref(), want.spin.as_ref()) {
        (None, None) => {}
        (Some(got), Some(want)) => {
            optional(out, id, "spin.spin_rpm", got.spin_rpm, want.spin_rpm);
            string(
                out,
                id,
                "spin.reason",
                got.reason.map(reason_name),
                want.reason.as_deref(),
            );
            string(
                out,
                id,
                "spin.detail",
                Some(&got.detail),
                Some(&want.detail),
            );
        }
        (a, b) => out.push(format!(
            "{id}: spin {} != {}",
            a.map_or("absent", |_| "present"),
            b.map_or("absent", |_| "present")
        )),
    }

    match (got.launch.as_ref(), want.launch.as_ref()) {
        (None, None) => {}
        (Some(got), Some(want)) => {
            for (field, a, b) in [
                ("ball_speed_mph", got.ball_speed_mph, want.ball_speed_mph),
                (
                    "launch_angle_deg",
                    got.launch_angle_deg,
                    want.launch_angle_deg,
                ),
                ("spin_rpm", got.spin_rpm, want.spin_rpm),
                (
                    "launch_direction_deg",
                    got.launch_direction_deg,
                    want.launch_direction_deg,
                ),
                ("spin_axis_deg", got.spin_axis_deg, want.spin_axis_deg),
            ] {
                check(out, id, &format!("launch.{field}"), a, b);
            }
        }
        (a, b) => out.push(format!(
            "{id}: launch {} != {}",
            a.map_or("absent", |_| "present"),
            b.map_or("absent", |_| "present")
        )),
    }
}

/// The `unscored` entries, compared as a list: name, reason and detail, in the recorded order.
///
/// **The order is the answer as much as the contents are.** `analyze_swing` appends these to the
/// checkpoint loop's own list and §3 compares `swing.unscored` positionally, so a port emitting the
/// axis entry before the carry one would read identically per entry and be wrong.
fn compare_unscored(id: &str, got: &FlownShot, want: &[RecordedUnscored], out: &mut Vec<String>) {
    let got = flight_unscored(got);
    if got.len() != want.len() {
        out.push(format!(
            "{id}: unscored {:?} != {:?}",
            got.iter().map(|e| &e.name).collect::<Vec<_>>(),
            want.iter().map(|e| &e.name).collect::<Vec<_>>()
        ));
        return;
    }
    for (at, (got, want)) in got.iter().zip(want).enumerate() {
        for (field, a, b) in [
            ("name", got.name.as_str(), want.name.as_str()),
            ("reason", reason_name(got.reason), want.reason.as_str()),
            ("detail", got.detail.as_str(), want.detail.as_str()),
        ] {
            if a != b {
                out.push(format!("{id}: unscored[{at}].{field} {a:?} != {b:?}"));
            }
        }
    }
}

/// One optional string field under §3's exact rule, with a trail into the payload on a difference.
fn string(out: &mut Vec<String>, id: &str, what: &str, got: Option<&str>, want: Option<&str>) {
    if got != want {
        out.push(format!("{id}: {what} {got:?} != {want:?}"));
    }
}

fn optional(out: &mut Vec<String>, id: &str, what: &str, got: Option<f64>, want: Option<f64>) {
    match (got, want) {
        (Some(a), Some(b)) => check(out, id, what, a, b),
        (None, None) => {}
        (a, b) => out.push(format!("{id}: {what} {a:?} != {b:?}")),
    }
}

/// The wire name of an `UnscoredReason`, which the crate exposes only through serde.
///
/// A `to_value` rather than a second table: `contracts` has no `as_str` on this enum, and adding one
/// here would be the second spelling of eleven names that `docs/CONFORMANCE.md` §3 compares exactly.
fn reason_name(reason: contracts::unscored::UnscoredReason) -> &'static str {
    match serde_json::to_value(reason).expect("a reason serializes") {
        serde_json::Value::String(name) => Box::leak(name.into_boxed_str()),
        other => panic!("a reason serialized as {other}"),
    }
}

fn shape_name(shape: contracts::intent::TargetShape) -> &'static str {
    match shape {
        contracts::intent::TargetShape::Straight => "straight",
        contracts::intent::TargetShape::Draw => "draw",
        contracts::intent::TargetShape::Fade => "fade",
    }
}

/// Every committed stage vector with the engine vector it was derived from.
///
/// The five gates in the sibling files each duplicate this, for the reason they give: integration
/// tests are separate binaries and a crate whose only job is to be imported by six of them is more
/// structure than forty lines of reading earns. It arrives in *this* file only with P8b — P8's three
/// gates read nothing outside the `flight` stage, which was the phase's luck and is no longer true.
fn stage_vectors_with_inputs() -> Vec<(StageVector, EngineVector)> {
    stage_vectors()
        .into_iter()
        .map(|stage| {
            let mut path = spec_dir().join(&stage.provenance.derived_from);
            path.set_extension("json");
            if !path.exists() {
                path.set_extension("json.gz");
            }
            let engine: EngineVector = serde_json::from_str(&read_json(&path))
                .unwrap_or_else(|e| panic!("parse {path:?}: {e}"));
            (stage, engine)
        })
        .collect()
}

fn unspun(launch: &RecordedUnspun) -> UnspunLaunch {
    UnspunLaunch {
        ball_speed_mph: launch.ball_speed_mph,
        launch_angle_deg: launch.launch_angle_deg,
        launch_direction_deg: launch.launch_direction_deg,
        spin_axis_deg: launch.spin_axis_deg,
    }
}

/// Every committed `SpinSolution`, with the vector id it came from.
fn committed_solutions() -> Vec<(String, RecordedSolution)> {
    stage_vectors()
        .into_iter()
        .filter_map(|vector| {
            let solution = vector.stages.flight.flown?.resolved?.spin?.solution?;
            Some((vector.id, solution))
        })
        .collect()
}
