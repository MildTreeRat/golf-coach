//! `golf-lab flight`, held to `scripts/simulate_flight.py`'s text. [M29 P28]
//!
//! **`flight/expected/` is the frozen script's stdout, recorded once at M29 P28** (the plan's C8),
//! over the same arguments and, for the stored modes, over `flight/`'s fixture tree: five shots,
//! four trimmed from `data/processed/shots/` (both gate screens, a solved spin and a carry no spin
//! reaches) and one with no ball speed; four manifests holding three of their photos (one photo
//! twice, the earlier arrival in a `-05:00` offset), so the second gate screen and the shot with no
//! ball speed are held by no swing; and the golfer, with a bag declaring only the 7 iron. That
//! covers both signs of the error column and the gate's "do not match" sentence, whose other
//! branch the ignored case over `data/` asks.
//! The script read the tree through `GOLF_SHOTS_DIR`, `GOLF_SESSIONS_DIR` and `GOLF_GOLFERS_DIR`;
//! this reads it through the verb's own flags, and once through the same variables.
//!
//! The text differs from the script's only where `golf_lab::flight`'s module doc says, and each of
//! those is a case below. The whole run over `data/` was measured the same way, byte for byte on 54
//! argument sets; the ignored case at the foot re-asks the parts of it that survive P29's rewrite.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("flight")
}

/// The binary over `args`, with every directory variable cleared so a developer's shell cannot
/// point a case at a real store.
fn golf_lab(args: &[&str], env: &[(&str, &Path)]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_golf-lab"));
    command
        .arg("flight")
        .args(args)
        .env_remove("GOLF_SHOTS_DIR")
        .env_remove("GOLF_SESSIONS_DIR")
        .env_remove("GOLF_GOLFERS_DIR");
    for (name, value) in env {
        command.env(name, value);
    }
    command.output().expect("golf-lab runs")
}

/// The binary over the fixture tree, through the verb's own directory flags.
fn over_fixture(args: &[&str]) -> Output {
    let root = fixture();
    let dirs = ["shots", "sessions", "golfers"].map(|leaf| root.join(leaf));
    let mut all: Vec<&str> = args.to_vec();
    for (flag, dir) in ["--shots-dir", "--sessions-dir", "--golfers-dir"]
        .iter()
        .zip(&dirs)
    {
        all.push(flag);
        all.push(dir.to_str().expect("the fixture's path is UTF-8"));
    }
    golf_lab(&all, &[])
}

/// The script's recorded stdout. Checked out with whatever line endings git gives a `.txt`, so they
/// are normalized; the binary writes `\n`.
fn expected(name: &str) -> String {
    let path = fixture().join("expected").join(format!("{name}.txt"));
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{}: {error}", path.display()))
        .replace("\r\n", "\n")
}

fn stdout(out: &Output) -> String {
    String::from_utf8(out.stdout.clone()).expect("the report is UTF-8")
}

fn stderr(out: &Output) -> String {
    String::from_utf8(out.stderr.clone()).expect("the refusal is UTF-8")
}

/// The header every run that got past its arguments prints first.
const HEADER: &str =
    "\nBall flight — simulated, not measured\n------------------------------------------------------------------------\n";

#[test]
fn the_typed_report_is_the_script_s() {
    let out = golf_lab(
        &[
            "--ball-speed",
            "90.7",
            "--launch-angle",
            "20.9",
            "--spin",
            "5991",
            "--launch-direction",
            "-5.3",
            "--spin-axis",
            "2.5",
            "--points",
            "12",
        ],
        &[],
    );
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    assert_eq!(stdout(&out), expected("typed"));
}

/// The what-if paragraph, one shot out of tolerance at altitude, and the provenance with the
/// atmosphere's citation the profile's.
#[test]
fn the_gate_at_altitude_with_its_provenance_is_the_script_s() {
    let out = golf_lab(&["--gate", "--altitude", "1609", "--verbose"], &[]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    assert_eq!(stdout(&out), expected("gate"));
}

#[test]
fn every_stored_report_is_the_script_s() {
    for (name, args) in [
        ("shots", &["--shots"][..]),
        ("shot-measured", &["--shot", "2026-08-10-1"][..]),
        (
            "shot-solved",
            &["--shot", "2026-08-23-4", "--points", "4"][..],
        ),
        ("shot-unreachable", &["--shot", "2026-08-23-11"][..]),
        ("shot-loose", &["--shot", "2026-08-24-1"][..]),
    ] {
        let out = over_fixture(args);
        assert_eq!(out.status.code(), Some(0), "{name}: {}", stderr(&out));
        assert_eq!(stdout(&out), expected(name), "{name}");
    }
}

/// The variables the script reads are the defaults the flags override.
#[test]
fn the_directories_default_to_the_script_s_variables() {
    let root = fixture();
    let (shots, sessions, golfers) = (
        root.join("shots"),
        root.join("sessions"),
        root.join("golfers"),
    );
    let out = golf_lab(
        &["--shots"],
        &[
            ("GOLF_SHOTS_DIR", &shots),
            ("GOLF_SESSIONS_DIR", &sessions),
            ("GOLF_GOLFERS_DIR", &golfers),
        ],
    );
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    assert_eq!(stdout(&out), expected("shots"));
}

/// Each refusal before anything runs: argparse's words where the script's are argparse's, exit 2,
/// nothing on stdout.
#[test]
fn a_request_it_cannot_run_is_refused_before_the_header() {
    for (args, problem) in [
        (
            &[][..],
            "golf-lab: flight: error: --ball-speed, --launch-angle and --spin are all required (or \
             --gate, --shots, --shot)",
        ),
        (
            &["--ball-speed", "90"][..],
            "golf-lab: flight: error: --ball-speed, --launch-angle and --spin are required together",
        ),
        (
            &["--gate", "--shots"][..],
            "golf-lab: flight: error: typed conditions, --gate, --shots and --shot are four modes; \
             pick one.",
        ),
        (
            &["--ball-speed", "90", "--shot", "x"][..],
            "golf-lab: flight: error: typed conditions, --gate, --shots and --shot are four modes",
        ),
        (
            &["--ball-speed", "x", "--launch-angle", "20", "--spin", "5000"][..],
            "golf-lab: flight: argument --ball-speed: invalid float value: 'x'",
        ),
        (
            &["--gate", "--points", "2.5"][..],
            "golf-lab: flight: argument --points: invalid int value: '2.5'",
        ),
        (
            &["--gate", "--altitude"][..],
            "golf-lab: flight: argument --altitude: expected one argument",
        ),
        (
            &["--gate", "--verbose=yes"][..],
            "golf-lab: flight: argument --verbose: ignored explicit argument \"yes\"",
        ),
        (
            &["--gate", "--tau", "1"][..],
            "golf-lab: flight: unrecognized arguments: --tau",
        ),
        (
            &["--gate", "extra"][..],
            "golf-lab: flight: unrecognized arguments: extra",
        ),
    ] {
        let out = golf_lab(args, &[]);
        assert_eq!(out.status.code(), Some(2), "{args:?}");
        assert!(out.stdout.is_empty(), "{args:?}");
        let said = stderr(&out);
        assert!(said.starts_with(problem), "{args:?}: {said}");
        assert!(said.contains("golf-lab flight (--gate"), "{args:?}");
    }
}

/// `--help` is the usage on stdout with 0, as every `golf-lab` verb prints it.
#[test]
fn help_is_the_usage() {
    let out = golf_lab(&["--gate", "--help"], &[]);
    assert_eq!(out.status.code(), Some(0));
    assert!(stdout(&out).starts_with("usage: golf-lab"));
}

/// What the integrator or the profile refuses is a sentence after the header, exit 2, in the
/// script's words. The gate's is the script's traceback answered (the module doc).
#[test]
fn no_flight_is_said_on_stdout_with_exit_2() {
    for (args, said) in [
        (
            &[
                "--ball-speed",
                "90",
                "--launch-angle",
                "0",
                "--spin",
                "5000",
            ][..],
            "No flight: launch angle must be above the horizontal and below the vertical to leave \
             the ground, got 0.0 deg\n",
        ),
        (
            &["--ball-speed", "90", "--launch-angle", "10", "--spin", "-1"][..],
            "No flight: spin rate must not be negative, got -1.0 rpm\n",
        ),
        (
            &["--gate", "--altitude", "20000"][..],
            "No flight: altitude 20000.0 m is outside isa_troposphere (-2000.0 to 11000.0 m), \
             where the lapse rate this profile carries is not the standard's\n",
        ),
        (
            &["--gate", "--step", "0"][..],
            "No flight: step must be positive, got 0.0 s\n",
        ),
    ] {
        let out = golf_lab(args, &[]);
        assert_eq!(out.status.code(), Some(2), "{args:?}");
        assert_eq!(stdout(&out), format!("{HEADER}{said}"), "{args:?}");
        assert!(out.stderr.is_empty(), "{args:?}");
    }
}

/// A `--shot` nothing carries is argparse's error after the header, counting what the store holds.
#[test]
fn a_shot_id_nothing_carries_is_a_usage_error() {
    let out = over_fixture(&["--shot", "nope"]);
    assert_eq!(out.status.code(), Some(2));
    assert_eq!(stdout(&out), HEADER);
    assert!(stderr(&out).starts_with(
        "golf-lab: flight: error: no stored shot has the id 'nope'; --shots prints the 5 that do\n"
    ));
}

/// An empty store is a state, exit 0, and the sentence names the Rust import, where the script's
/// names `scripts/import_shot_screens.py`. It returns before the provenance, as the script does.
#[test]
fn an_empty_store_says_how_shots_arrive() {
    let empty = tempfile::tempdir().unwrap();
    let store = empty.path().to_str().unwrap();
    let out = golf_lab(&["--shots", "--verbose", "--shots-dir", store], &[]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    let text = stdout(&out);
    // The block's layout up to `in` cannot depend on the store's path, so it stays exact. What
    // follows wraps by that path's length: a long temp path (macOS's) goes to the next line whole,
    // which ends this one on `in` with no space after it, so the next word is asked of the flat text.
    assert!(
        text.starts_with(&format!("{HEADER}\n  No parsed shots in")),
        "{text:?}"
    );
    let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let flat_header = HEADER.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        flat.starts_with(&format!("{flat_header} No parsed shots in ")),
        "{flat}"
    );
    assert!(flat.contains(
        "Shots arrive by `golf-lab import-photos` or with an uploaded bundle, and the store is \
         keyed by the photo's sha256 — an empty one is a repo nobody has imported a screen into \
         yet, not an error."
    ));
    assert!(!text.contains("fitted:"), "{text}");
    assert!(text.ends_with("not an error.\n"), "{text:?}");
}

/// The solve's refusal is the shot's row, where the script's escapes as a traceback: every shot
/// on a step the integrator refuses is `refused by the integrator`, and the run exits 0.
#[test]
fn a_refused_step_over_stored_shots_is_each_shot_s_row() {
    let out = over_fixture(&["--shots", "--step", "0"]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    let text = stdout(&out);
    assert!(text.contains("  Not flown (5)\n"), "{text}");
    // Both gate screens fly on a printed spin, so they are refused by `simulate_flight`, and the
    // two that need a solve by the solve: four of five, all in the integrator's words.
    assert_eq!(text.matches("refused by the integrator").count(), 4);
    assert_eq!(
        text.matches("        step must be positive, got 0.0 s\n")
            .count(),
        4
    );
    // The shot with no ball speed is refused before anything is flown, for its own reason.
    assert!(text.contains("    2026-08-24-1  -     no_launch_conditions\n"));
}

/// Over `data/`: what the frozen script printed there and P29's rewrite does not move, the gate's
/// two screens matching the typed pair and the photo three swings share.
#[test]
#[ignore = "needs data/processed (the shot store, the sessions and the golfers)"]
fn the_corpus_on_disk_reads_as_the_script_read_it() {
    let out = golf_lab(&["--shots"], &[]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    let flat = stdout(&out)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    assert!(flat.contains("stored shots, each joined to the swing it arrived with:"));
    assert!(flat.contains(
        "the 2 shots the gate is made of are in the shot store too, and the launch conditions \
         read off disk match the hand-typed VALIDATION_SHOTS to the digit"
    ));

    let out = golf_lab(&["--shot", "2026-08-10-1"], &[]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    let flat = stdout(&out)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    assert!(flat.contains("swing 2026-08-07-aaron1/1"));
    assert!(flat.contains("the same photo is attached to 2026-08-09/2, 2026-08-10/1 as well"));
}
