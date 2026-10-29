//! `golf-lab flight`: fly a golf ball from launch conditions typed in, re-fly the validation gate,
//! or fly every stored shot joined to the swing it arrived with. `scripts/simulate_flight.py`.
//! [M29 P28]
//!
//! **Four modes, exactly one at a time**, as the script has them: typed conditions fly what nobody
//! hit (the what-if), `--gate` flies [`VALIDATION_SHOTS`] beside the carries HD Golf printed,
//! `--shots` reads the corpus and flies what it can, and `--shot` is one of those in full. The
//! stored modes are a join ([`read_flight_inputs`]): a shot carries no club, the swing it arrived
//! with does, and the two meet on the photo's sha256. A spin the solve produced is named as one on
//! every row it reaches, with the cap it sits under, so an inferred spin never wears a measured
//! label.
//!
//! Every number, sentence and refusal comes from the crates that already hold it: the integrator,
//! the gate and the sample from `analysis::flight`, the what-if from `FlightModel::at_altitude`, the
//! solve from `analysis::flight_infer`, the caveats from `analysis::flight_caveats` and the printed
//! comparison from `analysis::flight_measure` (ported at P14 for `golf-mcp`, which reads them too),
//! and each reason's summary from `contracts::unscored`. What is here is the script's layout and
//! the few sentences that are about this report and nothing else.
//!
//! Exit codes are the script's: 0 when a ball flew, 2 when none did or the request was unusable.
//! **Over stored shots a refusal is the finding**, so `--shots` exits 0 whatever it finds, and its
//! one 2 is a `--shot` id nothing on disk carries.
//!
//! # Where the text differs from the script's (the M29 plan's C8)
//!
//! Measured at P28 against the frozen script over `data/` and on typed conditions, byte for byte
//! but these, each declared in the plan's P28 findings:
//!
//! - **An empty shot store names `golf-lab import-photos`**, where the script names
//!   `scripts/import_shot_screens.py`, which M29 P34 deleted.
//! - **Where the script raises, this answers.** A step the integrator refuses reaches
//!   `gate_comparisons` and the spin solve, neither of which the script guards, so `--gate --step 0`
//!   is a traceback there and `No flight: <why>` with exit 2 here, as the typed mode already says
//!   it. Over stored shots a solve that refuses is that shot's row, `refused by the integrator`
//!   with the reason, which is what the script's own `_Resolved.error` exists for: a refusal
//!   escaping mid-corpus would print a traceback in the middle of a read.
//! - **Three directory flags of its own**, `--shots-dir`, `--sessions-dir` and `--golfers-dir`,
//!   as `golf-lab analyze` has them; the script reads only `settings`. Their defaults are the same
//!   variables, then `data/processed/`.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::io::{self, Write};
use std::path::PathBuf;

use analysis::benchmarks::flight_model::{
    flight_dataset_info, load_flight_model, FlightModel, SourceNote,
};
use analysis::flight::{
    gate_comparisons, simulate_flight, FlightResult, GateComparison, LaunchConditions,
    GATE_AGREEMENT_FRACTION, VALIDATION_SHOTS,
};
use analysis::flight_caveats::{
    altitude_caveat, caveats_for, clamp_caveat, gate_caveat, planar_caveat,
};
use analysis::flight_infer::{flight_for_shot, infer_spin_axis, ShotFlight, SpinSource};
use analysis::flight_measure::{
    circular_carry_note, compare_to_printed, signed_percent, FlownShot,
};
use contracts::intent::TargetShape;
use contracts::unscored::{spec_for, UnscoredReason};
use contracts::Timestamp;
use golf_core::textwrap::fill;
use storage::flight_inputs::{read_flight_inputs, FlightInputs, LoftGap};
use storage::shot_store::ShotStore;

use crate::analyze::wire_name;
use crate::import::spelled;

/// 72 columns, the width `club_profile.py` prints at, as the script says: two dev CLIs disagreeing
/// about how wide a terminal is would be a small thing done twice.
pub const WIDTH: usize = 72;

/// The script's `_M_S_TO_MPH`, evaluated as it is: the reciprocal first, then one multiply per
/// point, so the `mph` column rounds from the same double.
const M_S_TO_MPH: f64 = 1.0 / 0.44704;

/// Which of the four things to fly.
#[derive(Debug, Clone, PartialEq)]
pub enum Mode {
    /// `--ball-speed`, `--launch-angle` and `--spin`, with both lateral fields.
    Typed(LaunchConditions),
    /// `--gate`.
    Gate,
    /// `--shots`.
    Shots,
    /// `--shot SHOT_ID`.
    Shot(String),
}

/// One run of the verb, its defaults already resolved.
#[derive(Debug, Clone, PartialEq)]
pub struct Request {
    pub mode: Mode,
    /// Metres. Zero leaves the committed air alone, as the script's truthiness test does: the
    /// what-if is bit-identical there anyway, and not asking for it keeps the air's name the
    /// published one rather than a generated one.
    pub altitude_m: f64,
    pub step_s: f64,
    /// Signed because the script's is argparse's `int`: any nonzero value prints the table, and a
    /// count below two is the landing alone (`FlightResult::sample`).
    pub points: i64,
    pub verbose: bool,
    pub shots_dir: PathBuf,
    pub sessions_dir: PathBuf,
    pub golfers_dir: PathBuf,
}

/// How a run ended: an exit code, or the one usage error that can only be found once the store
/// has been read (a `--shot` id nothing carries), which the binary prints with the usage, as
/// argparse's `parser.error` does, and exits 2.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ended {
    Exit(u8),
    Usage(String),
}

/// The script's `main` after the arguments: the header, the model, the mode, the provenance.
pub fn flight(request: &Request, out: &mut dyn Write) -> io::Result<Ended> {
    writeln!(out)?;
    writeln!(out, "Ball flight — simulated, not measured")?;
    writeln!(out, "{}", "-".repeat(WIDTH))?;

    let committed = load_flight_model();
    // The air is a block of the model and the model is already a parameter, so altitude is a
    // different model rather than a different call (M15 P6).
    let altitude_model;
    let model = if request.altitude_m != 0.0 {
        match committed.at_altitude(request.altitude_m) {
            Ok(model) => {
                altitude_model = model;
                &altitude_model
            }
            Err(refused) => {
                writeln!(out, "No flight: {refused}")?;
                return Ok(Ended::Exit(2));
            }
        }
    } else {
        committed
    };

    match &request.mode {
        Mode::Shots | Mode::Shot(_) => {
            let rows = read_flight_inputs(
                ShotStore::new(&request.shots_dir).all(),
                &request.sessions_dir,
                &request.golfers_dir,
            );
            if rows.is_empty() {
                writeln!(out)?;
                let empty = format!(
                    "No parsed shots in {}. Shots arrive by `golf-lab import-photos` or with an \
                     uploaded bundle, and the store is keyed by the photo's sha256 — an empty one \
                     is a repo nobody has imported a screen into yet, not an error.",
                    spelled(&request.shots_dir).display()
                );
                writeln!(out, "{}", wrap(&empty, "  "))?;
                return Ok(Ended::Exit(0));
            }
            if let Mode::Shot(wanted) = &request.mode {
                let count = rows.len();
                // Only the one asked for is flown: the script resolves all of them first, and the
                // report is the same either way, because nothing it prints reads another row.
                let Some(row) = rows.into_iter().find(|row| row.shot.shot_id == *wanted) else {
                    return Ok(Ended::Usage(format!(
                        "flight: error: no stored shot has the id {}; --shots prints the {count} \
                         that do",
                        pyfmt::str_repr(wanted)
                    )));
                };
                let resolved = resolve(row, model, request.step_s);
                report_shot(out, &resolved, model, request.points, request.altitude_m)?;
            } else {
                let resolved: Vec<Resolved> = rows
                    .into_iter()
                    .map(|row| resolve(row, model, request.step_s))
                    .collect();
                report_stored(out, &resolved)?;
                report_stored_findings(out, &resolved, request.altitude_m)?;
            }
        }
        Mode::Gate => {
            if request.altitude_m != 0.0 {
                writeln!(out)?;
                let what_if = format!(
                    "Flown at {} m. The gate is a sea-level gate, so what follows is a what-if \
                     and not a validation: HD Golf printed these carries for a ball hit indoors \
                     near sea level, and an altitude chosen because the agreement improved would \
                     be fitting the atmosphere to the residual of a held coefficient (ADR-027's \
                     2026-09-05e addendum).",
                    pyfmt::g(request.altitude_m)
                );
                writeln!(out, "{}", wrap(&what_if, "  "))?;
            }
            match gate_comparisons(model, request.step_s) {
                Ok(comparisons) => report_gate(out, &comparisons)?,
                Err(refused) => {
                    writeln!(out, "No flight: {}", refused.message)?;
                    return Ok(Ended::Exit(2));
                }
            }
        }
        Mode::Typed(launch) => {
            // `simulate_flight` refuses rather than guessing, and its doc names this as the
            // boundary that owns the check: a ball at or below the horizontal rolls.
            let result = match simulate_flight(launch, model, request.step_s) {
                Ok(result) => result,
                Err(refused) => {
                    writeln!(out, "No flight: {}", refused.message)?;
                    return Ok(Ended::Exit(2));
                }
            };
            report_flight(out, &result, model, request.points)?;
            let mut said = vec![clamp_caveat(&result, model), gate_caveat()];
            if request.altitude_m != 0.0 {
                said.push(altitude_caveat(request.altitude_m));
            }
            report_caveats(out, &said)?;
        }
    }

    if request.verbose {
        report_provenance(out, model)?;
    }
    writeln!(out)?;
    Ok(Ended::Exit(0))
}

// --------------------------------------------------------------------- the script's helpers

/// `_wrap`: one paragraph, wrapped at [`WIDTH`] and hanging-indented under its bullet. The
/// whitespace is Python's `split()`'s and `" ".join`'s, so a sentence written across source lines
/// arrives as one line before `fill` breaks it.
pub fn wrap(body: &str, bullet: &str) -> String {
    let joined = pyfmt::split(body).collect::<Vec<_>>().join(" ");
    fill(&joined, WIDTH, bullet, &" ".repeat(bullet.chars().count()))
}

/// `_row`: a label, a right-aligned value, and an optional note after three spaces.
fn row(label: &str, value: &str, note: &str) -> String {
    let line = format!("  {label:<16}{value:>14}");
    if note.is_empty() {
        line
    } else {
        format!("{line}   {note}")
    }
}

/// Python's `f"{x:+g}"`: [`pyfmt::g`] with the sign always written. CPython renders the digits and
/// inserts the sign after, so `-0.0` stays `-0` and a NaN is `+nan`, as [`pyfmt::signed_fixed`]
/// has it for `.Nf`.
pub fn signed_g(x: f64) -> String {
    let rendered = pyfmt::g(x);
    if rendered.starts_with('-') {
        rendered
    } else {
        format!("+{rendered}")
    }
}

/// `f"{timestamp:%Y-%m-%d %H:%M}"`: the minute in the value's **own** offset, as `strftime` reads
/// an aware `datetime`, beside [`Timestamp::date_ymd`]'s date in the same offset.
pub fn local_minute(timestamp: &Timestamp) -> String {
    const MICROS_PER_MINUTE: i64 = 60_000_000;
    const MINUTES_PER_DAY: i64 = 24 * 60;
    let local = timestamp.utc_micros() + i64::from(timestamp.offset_minutes()) * MICROS_PER_MINUTE;
    let minute = local.rem_euclid(MINUTES_PER_DAY * MICROS_PER_MINUTE) / MICROS_PER_MINUTE;
    format!(
        "{} {:02}:{:02}",
        timestamp.date_ymd(),
        minute / 60,
        minute % 60
    )
}

/// `LaunchConditions`' `repr`, a `NamedTuple`'s: the `_gate_from_disk` sentence prints both
/// readings whole when they differ, so a reader sees which field moved.
fn launch_repr(launch: Option<&LaunchConditions>) -> String {
    match launch {
        None => "None".to_string(),
        Some(launch) => format!(
            "LaunchConditions(ball_speed_mph={}, launch_angle_deg={}, spin_rpm={}, \
             launch_direction_deg={}, spin_axis_deg={})",
            pyfmt::repr(launch.ball_speed_mph),
            pyfmt::repr(launch.launch_angle_deg),
            pyfmt::repr(launch.spin_rpm),
            pyfmt::repr(launch.launch_direction_deg),
            pyfmt::repr(launch.spin_axis_deg)
        ),
    }
}

// --------------------------------------------------------------------- the flight

/// `_report_flight`: the launch and the air, then one landing point read in two frames.
fn report_flight(
    out: &mut dyn Write,
    result: &FlightResult,
    model: &FlightModel,
    points: i64,
) -> io::Result<()> {
    let launch = &result.launch;
    let air = &model.atmosphere;
    writeln!(out)?;
    // Whole rpm: the root search stops at half a revolution, so two decimals on a solved spin
    // would claim a precision the inverse problem does not have.
    writeln!(
        out,
        "  launch    {} mph, {} deg up, {} rpm, start {} deg, axis {} deg",
        pyfmt::g(launch.ball_speed_mph),
        pyfmt::g(launch.launch_angle_deg),
        pyfmt::fixed(launch.spin_rpm, 0),
        signed_g(launch.launch_direction_deg),
        signed_g(launch.spin_axis_deg)
    )?;
    writeln!(
        out,
        "  air       {}, {} kg/m3, {} K",
        air.name,
        pyfmt::fixed(air.density_kg_m3, 4),
        pyfmt::fixed(air.temperature_k, 2)
    )?;
    writeln!(out)?;

    let yards = |value: f64| format!("{} yd", pyfmt::fixed(value, 2));
    let signed_yards = |value: f64| format!("{} yd", pyfmt::signed_fixed(value, 2));
    let lines = [
        row("carry", &yards(result.carry_yds()), "along the launch line"),
        row(
            "offline",
            &signed_yards(result.landing_offline_yds()),
            "from the target line",
        ),
        row(
            "curvature",
            &signed_yards(result.curvature_yds()),
            "from the launch line",
        ),
        row("apex", &yards(result.apex_yds()), ""),
        row(
            "hang time",
            &format!("{} s", pyfmt::fixed(result.flight_time_s, 2)),
            "",
        ),
        row(
            "descent",
            &format!("{} deg", pyfmt::fixed(result.descent_angle_deg, 1)),
            "",
        ),
        row(
            "spin at landing",
            &format!("{} rpm", pyfmt::fixed(result.landing().spin_rpm, 0)),
            "",
        ),
        row(
            "spin ratio",
            &format!(
                "{} - {}",
                pyfmt::fixed(result.spin_ratio_min, 3),
                pyfmt::fixed(result.spin_ratio_max, 3)
            ),
            &format!(
                "{} of {} points held",
                result.clamped_points,
                result.points.len()
            ),
        ),
    ];
    for line in lines {
        writeln!(out, "{line}")?;
    }

    if points != 0 {
        report_points(out, result, points)?;
    }
    Ok(())
}

/// `_report_points`: [`FlightResult::sample`]'s points, launch and landing always among them.
fn report_points(out: &mut dyn Write, result: &FlightResult, wanted: i64) -> io::Result<()> {
    writeln!(out)?;
    writeln!(
        out,
        "  {:>6}{:>10}{:>9}{:>10}{:>8}{:>8}  coeff",
        "t s", "down yd", "up yd", "right yd", "mph", "S"
    )?;
    for point in result.sample(wanted) {
        writeln!(
            out,
            "  {:>6}{:>10}{:>9}{:>10}{:>8}{:>8}  {}",
            pyfmt::fixed(point.t_s, 2),
            pyfmt::fixed(point.x_yds(), 2),
            pyfmt::fixed(point.y_yds(), 2),
            pyfmt::fixed(point.z_yds(), 2),
            pyfmt::fixed(point.speed_m_s() * M_S_TO_MPH, 1),
            pyfmt::fixed(point.spin_ratio, 3),
            if point.clamped { "held" } else { "read" }
        )?;
    }
    Ok(())
}

/// `_report_caveats`: what has to be said beside the numbers, one wrapped bullet each.
fn report_caveats(out: &mut dyn Write, said: &[String]) -> io::Result<()> {
    writeln!(out)?;
    writeln!(out, "  Read it with these:")?;
    for caveat in said {
        writeln!(out, "{}", wrap(caveat, "  - "))?;
    }
    Ok(())
}

/// `_report_provenance`: per block, because the five blocks have four sources (ADR-027 §Decision
/// 2). The atmosphere's is the profile's once an altitude generated it.
fn report_provenance(out: &mut dyn Write, model: &FlightModel) -> io::Result<()> {
    let info = flight_dataset_info();
    let blocks: [(&str, &SourceNote); 5] = [
        ("ball", &model.ball.source),
        ("atmosphere", &model.atmosphere.source),
        ("atmosphere_profile", &model.atmosphere_profile.source),
        ("spin_decay", &model.spin_decay.source),
        ("coefficients", &model.coefficients.source),
    ];
    writeln!(out)?;
    writeln!(
        out,
        "  {}, assembled {} by {}",
        info.name, info.assembled_on, info.assembled_by
    )?;
    writeln!(out, "  fitted: {}", info.fitted)?;
    for (name, source) in blocks {
        writeln!(out)?;
        writeln!(out, "  {name}")?;
        writeln!(out, "{}", wrap(&source.citation, "    "))?;
        writeln!(out, "{}", wrap(&source.url, "    "))?;
    }
    Ok(())
}

// --------------------------------------------------------------------- the gate

/// `_report_gate`: both validation shots beside HD Golf's carries, then the ordering, which no
/// per-shot tolerance can report.
fn report_gate(out: &mut dyn Write, comparisons: &[GateComparison]) -> io::Result<()> {
    writeln!(out)?;
    writeln!(
        out,
        "  {:<14}{:>10}{:>10}{:>10}{:>9}",
        "shot", "HD Golf", "model", "error yd", "error"
    )?;
    for c in comparisons {
        writeln!(
            out,
            "  {:<14}{:>10}{:>10}{:>10}{:>9}  {}",
            c.shot.shot_id,
            pyfmt::fixed(c.shot.simulator_carry_yds, 1),
            pyfmt::fixed(c.flight.carry_yds(), 2),
            pyfmt::signed_fixed(c.error_yds(), 2),
            signed_percent(c.error_fraction(), 2),
            if c.agrees() { "ok" } else { "OUT" }
        )?;
    }

    // `VALIDATION_SHOTS` is ordered, and the first shot is the lower-spin one.
    let (first, second) = (&comparisons[0], &comparisons[1]);
    let measured_gap = first.shot.simulator_carry_yds - second.shot.simulator_carry_yds;
    let model_gap = first.flight.carry_yds() - second.flight.carry_yds();
    let inverted = measured_gap * model_gap < 0.0;
    let further = |gap: f64| if gap > 0.0 { "further" } else { "shorter" };
    writeln!(out)?;
    let ordering = format!(
        "HD Golf has {} — the lower spin of the two — flying {} yd {} than {}; this model has it \
         {} yd {}. Each shot passes on its own, and the ordering between them is {} — which no \
         per-shot tolerance can see. Read GATE_AGREEMENT_FRACTION ({}) as the size of that, not \
         as an accuracy.",
        first.shot.shot_id,
        pyfmt::fixed(measured_gap.abs(), 1),
        further(measured_gap),
        second.shot.shot_id,
        pyfmt::fixed(model_gap.abs(), 2),
        further(model_gap),
        if inverted { "inverted" } else { "right" },
        pyfmt::percent(GATE_AGREEMENT_FRACTION, 2)
    );
    writeln!(out, "{}", wrap(&ordering, "  "))?;
    if comparisons.iter().all(|c| c.flight.fully_clamped()) {
        writeln!(out)?;
        writeln!(
            out,
            "{}",
            wrap(
                "Both flights are clamped end to end, so this is the agreement between HD Golf \
                 and a held constant pair — not evidence about the published table, which \
                 neither flight read a single row of.",
                "  "
            )
        )?;
    }
    Ok(())
}

// --------------------------------------------------------------------- stored shots

/// `_Resolved`: one stored shot, what it was joined to, what stood in for its spin, and where it
/// landed.
#[derive(Debug, Clone, PartialEq)]
pub struct Resolved {
    pub inputs: FlightInputs,
    pub flight: ShotFlight,
    pub result: Option<FlightResult>,
    /// What the integrator refused, on a condition [`flight_for_shot`] does not pre-check. `None`
    /// on every shot on disk.
    pub error: Option<String>,
}

/// `_resolve_stored`'s loop body: the launch conditions resolved, then flown if there are any.
///
/// **The solve's own refusal is this shot's row**, where the script lets it escape: the solve flies
/// the ball dozens of times, so a step the integrator refuses surfaces from inside it. The row then
/// carries the axis alone and the integrator's words, and reads as `refused by the integrator`, the
/// label the script gives an `error` it caught.
pub fn resolve(inputs: FlightInputs, model: &FlightModel, step_s: f64) -> Resolved {
    let shot = &inputs.shot;
    let flight = match flight_for_shot(shot, inputs.loft_deg(), inputs.handedness, model, step_s) {
        Ok(flight) => flight,
        Err(refused) => {
            let axis = infer_spin_axis(shot, inputs.handedness);
            return Resolved {
                flight: ShotFlight {
                    launch: None,
                    spin_source: None,
                    spin: None,
                    axis,
                    reason: None,
                    detail: String::new(),
                },
                result: None,
                error: Some(refused.message),
                inputs,
            };
        }
    };
    let (result, error) = match &flight.launch {
        None => (None, None),
        Some(launch) => match simulate_flight(launch, model, step_s) {
            Ok(result) => (Some(result), None),
            Err(refused) => (None, Some(refused.message)),
        },
    };
    Resolved {
        inputs,
        flight,
        result,
        error,
    }
}

/// `r.error or fallback`, with Python's truthiness: an empty message is no message.
fn error_or<'a>(error: &'a Option<String>, fallback: &'a str) -> &'a str {
    error
        .as_deref()
        .filter(|text| !text.is_empty())
        .unwrap_or(fallback)
}

/// `_club_of`.
fn club_of(inputs: &FlightInputs) -> &'static str {
    inputs.club.map_or("-", |club| club.as_str())
}

/// `_carry_column`: the printed carry, or a dash where the screen printed none.
fn carry_column(resolved: &Resolved) -> String {
    match resolved.inputs.shot.carry_distance {
        Some(printed) => format!("{:>9}", pyfmt::fixed(printed, 1)),
        None => format!("{:>9}", "-"),
    }
}

/// `_report_stored`: the corpus in three blocks, measured spin, solved spin, and not flown.
///
/// The counts are the script's, quirk included: a shot whose spin was measured and whose flight the
/// integrator then refused is counted in both the first block's number and the third's, and listed
/// in the third only.
fn report_stored(out: &mut dyn Write, resolved: &[Resolved]) -> io::Result<()> {
    let source = |wanted: SpinSource| {
        resolved
            .iter()
            .filter(move |r| r.flight.spin_source == Some(wanted))
    };
    let measured: Vec<&Resolved> = source(SpinSource::Measured).collect();
    let inferred: Vec<&Resolved> = source(SpinSource::Inferred).collect();
    let refused: Vec<&Resolved> = resolved.iter().filter(|r| r.result.is_none()).collect();

    writeln!(out)?;
    let summary = format!(
        "{} stored shots, each joined to the swing it arrived with: {} flown on a spin the screen \
         printed, {} on a spin solved from the printed carry, {} not flown.",
        resolved.len(),
        measured.len(),
        inferred.len(),
        refused.len()
    );
    writeln!(out, "{}", wrap(&summary, "  "))?;

    if !measured.is_empty() {
        writeln!(out)?;
        writeln!(out, "  Spin measured off the screen")?;
        writeln!(
            out,
            "    {:<14}{:<6}{:>6}{:>6}{:>7}{:>9}{:>8}{:>8}",
            "shot", "club", "mph", "deg", "rpm", "printed", "flown", "error"
        )?;
        for r in measured {
            let (Some(launch), Some(result)) = (&r.flight.launch, &r.result) else {
                continue;
            };
            let error = match r.inputs.shot.carry_distance {
                Some(printed) if printed > 0.0 => {
                    signed_percent((result.carry_yds() - printed) / printed, 2)
                }
                _ => "-".to_string(),
            };
            writeln!(
                out,
                "    {:<14}{:<6}{:>6}{:>6}{:>7}{}{:>8}{:>8}",
                r.inputs.shot.shot_id,
                club_of(&r.inputs),
                pyfmt::fixed(launch.ball_speed_mph, 1),
                pyfmt::fixed(launch.launch_angle_deg, 1),
                pyfmt::fixed(launch.spin_rpm, 0),
                carry_column(r),
                pyfmt::fixed(result.carry_yds(), 2),
                error
            )?;
        }
    }

    if !inferred.is_empty() {
        writeln!(out)?;
        writeln!(out, "  Spin solved from the printed carry")?;
        writeln!(
            out,
            "    {:<14}{:<6}{:>6}{:>6}{:>7}{:>9}{:>8}{:>8}",
            "shot", "club", "mph", "deg", "rpm", "printed", "flown", "cap"
        )?;
        for r in inferred {
            let (Some(launch), Some(result), Some(spin)) =
                (&r.flight.launch, &r.result, &r.flight.spin)
            else {
                continue;
            };
            writeln!(
                out,
                "    {:<14}{:<6}{:>6}{:>6}{:>7}{}{:>8}{:>8}",
                r.inputs.shot.shot_id,
                club_of(&r.inputs),
                pyfmt::fixed(launch.ball_speed_mph, 1),
                pyfmt::fixed(launch.launch_angle_deg, 1),
                pyfmt::fixed(launch.spin_rpm, 0),
                carry_column(r),
                pyfmt::fixed(result.carry_yds(), 2),
                pyfmt::fixed(spin.cap_rpm(), 0)
            )?;
        }
        writeln!(out)?;
        // The column form of `circular_carry_note`, composed in `flight_measure` because the same
        // claim reaches a reader one row at a time elsewhere. The cap sentence is about this
        // table's last column and nothing else has one, so it stays here, as in the script.
        writeln!(out, "{}", wrap(&circular_carry_note(None), "    - "))?;
        writeln!(
            out,
            "{}",
            wrap(
                "`cap` is the spin above which carry stops responding to spin at all — an answer \
                 near it is a floor, not a reading.",
                "    - "
            )
        )?;
    }

    if !refused.is_empty() {
        writeln!(out)?;
        writeln!(out, "  Not flown ({})", refused.len())?;
        for r in &refused {
            let label = r
                .flight
                .reason
                .map_or_else(|| "refused by the integrator".to_string(), wire_name);
            writeln!(
                out,
                "    {:<14}{:<6}{label}",
                r.inputs.shot.shot_id,
                club_of(&r.inputs)
            )?;
            writeln!(
                out,
                "{}",
                wrap(error_or(&r.error, &r.flight.detail), "        ")
            )?;
        }
        report_reason_legend(out, &refused)?;
    }
    Ok(())
}

/// `_report_reason_legend`: what each reason above means, in `contracts::unscored`'s own words,
/// sorted by name.
fn report_reason_legend(out: &mut dyn Write, refused: &[&Resolved]) -> io::Result<()> {
    let seen: BTreeMap<String, UnscoredReason> = refused
        .iter()
        .filter_map(|r| r.flight.reason)
        .map(|reason| (wire_name(reason), reason))
        .collect();
    if seen.is_empty() {
        return Ok(());
    }
    writeln!(out)?;
    for (name, reason) in seen {
        let summary = spec_for(reason).summary;
        writeln!(out, "{}", wrap(&format!("{name}: {summary}"), "    - "))?;
    }
    Ok(())
}

/// `_report_shot`: one stored shot in full, the join, the two stand-ins, and the flight if there
/// was one.
fn report_shot(
    out: &mut dyn Write,
    r: &Resolved,
    model: &FlightModel,
    points: i64,
    altitude_m: f64,
) -> io::Result<()> {
    let row = &r.inputs;
    let shot = &row.shot;
    writeln!(out)?;
    writeln!(
        out,
        "  shot      {}, {}, {}",
        shot.shot_id,
        shot.source.as_str(),
        local_minute(&shot.timestamp)
    )?;
    writeln!(
        out,
        "  swing     {}",
        row.swing_ref
            .as_deref()
            .filter(|reference| !reference.is_empty())
            .unwrap_or("no swing on disk carries this photo")
    )?;
    if !row.also_attached_to.is_empty() {
        let also = format!(
            "the same photo is attached to {} as well; the earliest arrival is the one read here, \
             which is `storage/corpus.py`'s rule for a re-uploaded bundle.",
            row.also_attached_to.join(", ")
        );
        writeln!(out, "{}", wrap(&also, &" ".repeat(12)))?;
    }
    let hand = row.handedness.map_or("-", |hand| hand.as_str());
    let player = row
        .player_id
        .as_deref()
        .filter(|id| !id.is_empty())
        .unwrap_or("-");
    writeln!(out, "  golfer    {player}, {hand}-handed")?;
    let loft = match row.loft {
        Ok(loft_deg) => format!("{} deg", pyfmt::g(loft_deg)),
        Err(gap) => format!("none ({})", LoftGap::as_str(gap)),
    };
    writeln!(out, "  club      {}, loft {loft}", club_of(row))?;

    writeln!(out)?;
    writeln!(
        out,
        "{}",
        wrap(&format!("spin: {}", r.flight.detail), "  - ")
    )?;
    writeln!(
        out,
        "{}",
        wrap(&format!("axis: {}", r.flight.axis.detail), "  - ")
    )?;
    if r.flight.axis.sign_disagrees() {
        let disagrees = format!(
            "the direction derived here ({}) contradicts the `Shot Type` tile ({}). Flagged and \
             never overwritten (ADR-027 §Decision 5).",
            shape_name(r.flight.axis.curve_direction),
            shape_name(r.flight.axis.screen_shape)
        );
        writeln!(out, "{}", wrap(&disagrees, "  ! "))?;
    }

    let Some(result) = &r.result else {
        writeln!(out)?;
        writeln!(
            out,
            "{}",
            wrap(error_or(&r.error, "No flight, for the reason above."), "  ")
        )?;
        return Ok(());
    };

    report_flight(out, result, model, points)?;
    report_comparison(out, r, result)?;
    // Which caveats a flown shot owes is a rule about the flight, composed in `flight_caveats` for
    // every surface that shows one.
    report_caveats(out, &caveats_for(result, &r.flight, model, altitude_m))
}

/// A `TargetShape` as an f-string prints a `StrEnum`: its value, or `None`. Through its own
/// `Serialize`, as [`wire_name`] reads a reason's, so no name is retyped here.
fn shape_name(shape: Option<TargetShape>) -> String {
    match shape.map(serde_json::to_value) {
        None => "None".to_string(),
        Some(Ok(serde_json::Value::String(name))) => name,
        other => unreachable!("a shape serializes to its name, not {other:?}"),
    }
}

/// `_report_comparison`: the simulated numbers this screen printed a counterpart for, and what
/// each gap is, by `compare_to_printed`'s rule, the same one `golf-mcp`'s `simulate_flight` serves.
fn report_comparison(out: &mut dyn Write, r: &Resolved, result: &FlightResult) -> io::Result<()> {
    let flown = FlownShot {
        resolved: Some(r.flight.clone()),
        flight: Some(result.clone()),
        reason: None,
        detail: r.flight.detail.clone(),
    };
    let rows = compare_to_printed(&flown, &r.inputs.shot);
    if rows.is_empty() {
        return Ok(());
    }
    writeln!(out)?;
    writeln!(out, "  Beside what the screen printed")?;
    writeln!(
        out,
        "    {:<27}{:>10}{:>10}{:>8}",
        "measurement", "simulated", "printed", "apart"
    )?;
    for row in &rows {
        writeln!(
            out,
            "    {:<27}{:>10}{:>10}{:>8}",
            row.simulated_name,
            pyfmt::fixed(row.simulated, 2),
            pyfmt::fixed(row.measured, 2),
            pyfmt::signed_fixed(row.difference, 2)
        )?;
    }
    writeln!(out)?;
    for row in &rows {
        let reading = format!("{}: {}", row.simulated_name, row.reading);
        writeln!(out, "{}", wrap(&reading, "    - "))?;
    }
    Ok(())
}

/// `_report_stored_findings`: what the corpus itself says, counted at run time rather than quoted.
fn report_stored_findings(
    out: &mut dyn Write,
    resolved: &[Resolved],
    altitude_m: f64,
) -> io::Result<()> {
    let mut said: Vec<String> = Vec::new();

    let tagged: Vec<&Resolved> = resolved
        .iter()
        .filter(|r| r.inputs.club.is_some())
        .collect();
    if !tagged.is_empty() {
        let with_loft = tagged
            .iter()
            .filter(|r| r.inputs.loft_deg().is_some())
            .count();
        said.push(format!(
            "{} of {} shots reach a club tag through the swing they arrived with, and {with_loft} \
             of those reach a declared loft. The club is on the *swing* and never on the shot, so \
             nothing reading `ShotData` alone can see it — which is what ADR-027's 2026-09-05g \
             addendum was measuring when it reported none.",
            tagged.len(),
            resolved.len()
        ));
    }

    let no_entry: Vec<&Resolved> = resolved
        .iter()
        .filter(|r| r.inputs.loft_gap() == Some(LoftGap::NoBagEntry))
        .collect();
    if !no_entry.is_empty() {
        let slots: BTreeSet<&str> = no_entry.iter().map(|r| club_of(&r.inputs)).collect();
        let blocked = no_entry
            .iter()
            .filter(|r| r.flight.reason == Some(UnscoredReason::NoClubLoft))
            .count();
        said.push(format!(
            "{} shots name a club with no bag entry ({}), so the loft that would pick the branch \
             has never been declared. That is the bag page and not a bay session: {blocked} of \
             them are otherwise solvable and would name a spin the moment the club is looked up.",
            no_entry.len(),
            slots.into_iter().collect::<Vec<_>>().join(", ")
        ));
    }

    let planar = resolved
        .iter()
        .filter(|r| r.result.is_some() && !r.flight.curve_is_drawn())
        .count();
    if planar > 0 {
        said.push(planar_caveat(planar));
    }

    for r in resolved.iter().filter(|r| r.flight.axis.sign_disagrees()) {
        said.push(format!(
            "{} disagrees with itself about which way it curved: the screen printed `{}` and the \
             face-to-path behind it is {}. Flagged rather than resolved — quietly picking one is \
             how ADR-014's sign inversion survived a milestone.",
            r.inputs.shot.shot_id,
            r.inputs.shot.shot_type.as_deref().unwrap_or("None"),
            shape_name(r.flight.axis.curve_direction)
        ));
    }

    said.push(gate_from_disk(resolved));
    said.push(gate_caveat());
    if altitude_m != 0.0 {
        said.push(altitude_caveat(altitude_m));
    }
    report_caveats(out, &said)
}

/// `_gate_from_disk`: whether the gate's hand-typed launch conditions match the ones the OCR
/// stored for the same two screens, which is the only check [`VALIDATION_SHOTS`] has.
///
/// Keyed by shot id with the **last** row winning, as the script's dict comprehension keeps it.
fn gate_from_disk(resolved: &[Resolved]) -> String {
    let stored: HashMap<&str, &Resolved> = resolved
        .iter()
        .map(|r| (r.inputs.shot.shot_id.as_str(), r))
        .collect();
    let checked: Vec<_> = VALIDATION_SHOTS
        .iter()
        .filter_map(|shot| stored.get(shot.shot_id).map(|r| (shot, *r)))
        .collect();
    if checked.is_empty() {
        return "neither shot the gate is made of is in the shot store, so the hand-typed \
                VALIDATION_SHOTS could not be checked against what the OCR read."
            .to_string();
    }
    let differences: Vec<String> = checked
        .iter()
        .filter(|(shot, r)| r.flight.launch != Some(shot.launch))
        .map(|(shot, r)| {
            format!(
                "{} (typed {}, stored {})",
                shot.shot_id,
                launch_repr(Some(&shot.launch)),
                launch_repr(r.flight.launch.as_ref())
            )
        })
        .collect();
    if !differences.is_empty() {
        return format!(
            "the gate's typed launch conditions do not match what the OCR stored for the same \
             screens: {}. One of the two readings is wrong, and every number the gate has ever \
             printed rests on the typed pair.",
            differences.join("; ")
        );
    }
    format!(
        "the {} shots the gate is made of are in the shot store too, and the launch conditions \
         read off disk match the hand-typed VALIDATION_SHOTS to the digit, including both lateral \
         fields. That constant has been unverifiable since M15 P4; this is the check, and it comes \
         from reading the corpus rather than from trusting it.",
        checked.len()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// CPython's `f"{x:+g}"` on each, printed off the frozen interpreter at M29 P28.
    #[test]
    fn signed_g_writes_python_s_sign() {
        for (x, python) in [
            (0.0, "+0"),
            (-0.0, "-0"),
            (3.0, "+3"),
            (-20.0, "-20"),
            (-5.3, "-5.3"),
            (2.5, "+2.5"),
            (1e-5, "+1e-05"),
            (f64::NAN, "+nan"),
            (f64::NEG_INFINITY, "-inf"),
        ] {
            assert_eq!(signed_g(x), python, "{x}");
        }
    }

    /// The minute in the stored offset, not in UTC: `04:30-05:00` is 09:30Z and prints 04:30, and
    /// a negative offset that crosses midnight keeps the local date.
    #[test]
    fn local_minute_reads_the_value_s_own_offset() {
        for (stored, python) in [
            ("2026-08-23T04:30:12Z", "2026-08-23 04:30"),
            ("2026-08-23T04:30:59-05:00", "2026-08-23 04:30"),
            ("2026-08-23T23:59:00-05:00", "2026-08-23 23:59"),
            ("2026-08-24T00:00:00+05:30", "2026-08-24 00:00"),
        ] {
            let timestamp = Timestamp::parse(stored).unwrap();
            assert_eq!(local_minute(&timestamp), python, "{stored}");
        }
    }

    /// `_wrap`: the source's line breaks dropped, then `fill` at 72 under a hanging indent.
    #[test]
    fn wrap_joins_the_source_s_lines_and_hangs_the_rest() {
        let text =
            "  `cap` is the spin above which carry stops responding\n   to spin at all — an \
                    answer near it is a floor, not a reading.  ";
        assert_eq!(
            wrap(text, "    - "),
            "    - `cap` is the spin above which carry stops responding to spin at\n      all — \
             an answer near it is a floor, not a reading."
        );
        assert_eq!(wrap("", "  "), "");
    }

    /// A `NamedTuple`'s `repr`, each float as `repr` writes it, and `None` for a shot with no
    /// launch conditions.
    #[test]
    fn the_launch_repr_is_the_named_tuple_s() {
        assert_eq!(
            launch_repr(Some(&VALIDATION_SHOTS[0].launch)),
            "LaunchConditions(ball_speed_mph=90.7, launch_angle_deg=20.9, spin_rpm=5991.0, \
             launch_direction_deg=-5.3, spin_axis_deg=2.5)"
        );
        assert_eq!(launch_repr(None), "None");
    }
}
