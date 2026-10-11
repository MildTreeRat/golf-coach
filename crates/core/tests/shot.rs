//! The shot family (`spec/vectors/shot/`) through the `run_shot` seam and its verb. [M35 P4]
//!
//! **No Python twin, and no Python oracle.** Every case is worked by hand (`provenance.oracle:
//! "hand"`, the M35 plan's call 7), and its `note` names where each number came from: a corpus
//! vector's Python-recorded rows wherever the shot's analysed fields are that vector's, and Rust's,
//! named value by value, where no Python vector holds them. So `expected` is held to the seam under
//! `docs/CONFORMANCE.md` §3's rules, through [`golf_core::compare`], with floats within its `RTOL`:
//! a value copied from a corpus vector is Python's bits, and Rust reaches it within that tolerance
//! rather than to the bit (the M35 plan's P2 finding 4).
//!
//! The seam is the gate rather than `analysis::engine::analyze_shot`, because the screen's refusals
//! are `screen::tile_refusals`' and only the shell holds both crates (ADR-008 as cargo edges). Then
//! the verb, as `tests/parse_screen.rs` holds `parse-screen`: every case piped through
//! `golf-core run-shot` comes back as exactly the value [`golf_core::run_shot`] returns, so the verb
//! adds nothing between stdin and stdout but the pipe. Then its own edges, which no library test can
//! see.
//!
//! # What the cases cover, as a checked fact
//!
//! [`the_cases_reach_every_row_of_the_tile_table_and_each_named_gap`] counts what the family
//! reaches, so a case edited away from its purpose fails by name rather than leaving a hole: every
//! row of call 3's table, a direct feed that refuses no tile, a printed shown-only tile left blank,
//! the flight's `no_handedness`, `no_club_loft` and `no_launch_conditions` refusals, and ADR-014's
//! needs-review note, which no corpus vector reaches (the M35 plan's P2 finding 4).
//!
//! [`every_corpus_vector_carrying_a_shot_has_its_case`] holds the other half of the family to the
//! corpus: one `corpus-<vector>` case per committed corpus vector with a shot, whose shot is that
//! vector's (P5). Its rows are then the swing's twice over, once here within `RTOL` and once in
//! `crates/analysis/tests/shot.rs`' Decision-12 pin.

use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use contracts::intent::PracticeMode;
use contracts::swing::ANALYSIS_VERSION;
use flate2::read::GzDecoder;
use golf_core::compare::{compare, DifferenceKind};
use golf_core::{run_shot, ShotVector};
use serde_json::{json, Value};

fn shot_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec/vectors/shot")
}

/// Every case as `(file name, its parsed document)`, sorted so a failure names a stable first
/// offender.
fn cases() -> Vec<(String, Value)> {
    let dir = shot_dir();
    let mut found: Vec<(String, Value)> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("read {dir:?}: {e}"))
        .map(|entry| {
            let path = entry.expect("a directory entry").path();
            let name = path
                .file_name()
                .expect("a named file")
                .to_string_lossy()
                .into_owned();
            let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path:?}: {e}"));
            let value =
                serde_json::from_str(&text).unwrap_or_else(|e| panic!("parse {path:?}: {e}"));
            (name, value)
        })
        .collect();
    found.sort_by(|a, b| a.0.cmp(&b.0));
    assert!(!found.is_empty(), "no shot cases under {dir:?}");
    found
}

fn case(name: &str) -> Value {
    cases()
        .into_iter()
        .find(|(file, _)| file == name)
        .unwrap_or_else(|| panic!("no shot case {name}"))
        .1
}

/// What the seam makes of a case's `input`, serialized as the verb writes it.
fn seam(document: &Value, name: &str) -> Value {
    let vector: ShotVector = serde_json::from_value(document.clone())
        .unwrap_or_else(|e| panic!("{name}: input does not parse into ShotInput: {e}"));
    let result =
        run_shot(&vector.input).unwrap_or_else(|e| panic!("{name}: run_shot refused: {e}"));
    serde_json::to_value(result).expect("a ShotResult serializes")
}

/// **Every case, through the seam, against its hand-worked `expected`, in one report.**
///
/// Each file says what it is before it is run: the family's kind, the hand oracle with a note, and
/// the engine generation its numbers are, since the family is re-recorded under `ANALYSIS_VERSION`
/// (the M35 plan's call 7) and a case recorded by another engine certifies retracted answers.
#[test]
fn every_case_conforms_through_the_seam() {
    let cases = cases();
    let mut differences = Vec::new();
    for (name, document) in &cases {
        let provenance = &document["provenance"];
        assert_eq!(provenance["kind"], json!("shot"), "{name}: kind");
        assert_eq!(provenance["oracle"], json!("hand"), "{name}: oracle");
        assert!(
            provenance["note"]
                .as_str()
                .is_some_and(|note| !note.is_empty()),
            "{name}: a hand-worked case says where its numbers came from"
        );
        assert_eq!(
            document["id"],
            json!(format!("shot/{}", name.trim_end_matches(".json"))),
            "{name}: id"
        );
        assert_eq!(
            document["analysis_version"].as_i64(),
            Some(ANALYSIS_VERSION),
            "{name}: recorded at v{} against a port claiming v{ANALYSIS_VERSION}; re-record the \
             family in the change that bumped it",
            document["analysis_version"]
        );
        differences.extend(
            compare(&document["expected"], &seam(document, name))
                .iter()
                .map(|difference| format!("{name}: {difference}")),
        );
    }
    assert!(
        differences.is_empty(),
        "{} difference(s) across {} shot cases:\n{}",
        differences.len(),
        cases.len(),
        differences.join("\n")
    );
}

/// **Decision 1 on one case**: the drill, re-run under `FUNDAMENTALS`, differs at `intent.mode`
/// and nowhere else. Its `expected` already holds that, since its rows are a vector recorded under
/// `FUNDAMENTALS`; this says it without the reader having to know which vector.
#[test]
fn the_drill_differs_from_fundamentals_in_its_mode_alone() {
    let drill = case("drill-intent.json");
    assert_eq!(drill["input"]["intent"]["mode"], json!("drill"));
    let mut fundamentals = drill.clone();
    fundamentals["input"]["intent"]["mode"] = json!("fundamentals");

    let differences = compare(
        &seam(&fundamentals, "fundamentals"),
        &seam(&drill, "drill-intent.json"),
    );
    assert_eq!(differences.len(), 1, "{differences:?}");
    assert_eq!(differences[0].path, "intent.mode");
    assert!(matches!(differences[0].kind, DifferenceKind::Moved { .. }));
    let mode: PracticeMode =
        serde_json::from_value(drill["expected"]["intent"]["mode"].clone()).expect("a mode");
    assert!(!mode.is_tracked(), "the case is the untracked mode");
}

/// What the family reaches, counted, so a case cannot drift off its purpose silently.
#[test]
fn the_cases_reach_every_row_of_the_tile_table_and_each_named_gap() {
    let mut details: Vec<(String, String)> = Vec::new();
    let mut direct_feed_without_refusals = 0;
    let mut blank_shown_only = 0;
    let mut needs_review_noted = 0;
    for (name, document) in cases() {
        let shot = &document["input"]["shot"];
        let unscored = document["expected"]["unscored"]
            .as_array()
            .unwrap_or_else(|| panic!("{name}: unscored is a list"));
        let notes = document["expected"]["notes"]
            .as_array()
            .unwrap_or_else(|| panic!("{name}: notes is a list"));
        if notes.iter().any(|note| {
            note.as_str()
                .is_some_and(|n| n.starts_with("shot data needs review"))
        }) {
            needs_review_noted += 1;
        }
        for entry in unscored {
            details.push((
                entry["reason"].as_str().expect("a reason").to_string(),
                entry["detail"].as_str().expect("a detail").to_string(),
            ));
        }
        let provenance = shot.get("provenance").filter(|p| !p.is_null());
        if provenance.is_none() && !unscored.iter().any(|e| !is_flight(e)) {
            direct_feed_without_refusals += 1;
        }
        // A printed tile holding nothing whose field is shown only: in `fields_present`, `""`
        // under its label, and refused by nothing.
        if let Some(provenance) = provenance {
            let printed = provenance["fields_present"]
                .as_array()
                .cloned()
                .unwrap_or_default();
            if printed.contains(&json!("smash_factor"))
                && provenance["raw_fields"]["Smash Factor"] == json!("")
                && !unscored.iter().any(|e| e["name"] == json!("smash_factor"))
            {
                blank_shown_only += 1;
            }
        }
    }
    let reached = |reason: &str, words: &str| {
        details
            .iter()
            .any(|(r, detail)| r == reason && detail.contains(words))
    };
    for (reason, words) in [
        ("misread", "located, but its label tied with another tile's"),
        ("printed_blank", "nothing under the label"),
        ("printed_blank", "the screen's blank"),
        ("misread", "no value could be read from it"),
        ("no_handedness", "no golfer is attributed to this shot"),
        ("no_club_loft", "no loft is on record"),
        ("no_launch_conditions", "no carry to solve one from either"),
    ] {
        assert!(
            reached(reason, words),
            "no case reaches {reason}: {words:?}"
        );
    }
    assert!(direct_feed_without_refusals >= 1, "no direct feed");
    assert!(blank_shown_only >= 1, "no blank shown-only tile");
    assert!(
        needs_review_noted >= 1,
        "no case notes a shot needing review"
    );
}

/// Every committed corpus vector as `(its stem, its document)`, gunzipped and sorted.
fn corpus_vectors() -> Vec<(String, Value)> {
    let dir = shot_dir().join("../corpus");
    let mut found: Vec<(String, Value)> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("read {dir:?}: {e}"))
        .map(|entry| {
            let path = entry.expect("a directory entry").path();
            let stem = path
                .file_name()
                .expect("a named file")
                .to_string_lossy()
                .trim_end_matches(".json.gz")
                .to_string();
            let bytes = fs::read(&path).unwrap_or_else(|e| panic!("read {path:?}: {e}"));
            let mut text = String::new();
            GzDecoder::new(&bytes[..])
                .read_to_string(&mut text)
                .unwrap_or_else(|e| panic!("gunzip {path:?}: {e}"));
            let value =
                serde_json::from_str(&text).unwrap_or_else(|e| panic!("parse {path:?}: {e}"));
            (stem, value)
        })
        .collect();
    found.sort_by(|a, b| a.0.cmp(&b.0));
    assert!(!found.is_empty(), "no corpus vectors under {dir:?}");
    found
}

/// **One case per corpus vector with a shot, and its shot is that vector's** (the M35 plan's P5).
///
/// By name, `corpus-<vector>.json`, and by content: the case's `input.shot` is the vector's
/// `input.shot` unedited, and its swing and session are the vector's, so the case is that swing's
/// shot through the seam and nothing else. A corpus vector added without its case fails here
/// rather than leaving the shot family a swing behind.
#[test]
fn every_corpus_vector_carrying_a_shot_has_its_case() {
    let cases = cases();
    let mut missing = Vec::new();
    let mut checked = 0;
    for (stem, vector) in corpus_vectors() {
        let shot = &vector["input"]["shot"];
        if shot.is_null() {
            continue;
        }
        let file = format!("corpus-{stem}.json");
        let Some((_, document)) = cases.iter().find(|(name, _)| *name == file) else {
            missing.push(file);
            continue;
        };
        let input = &document["input"];
        assert_eq!(&input["shot"], shot, "{file}: the shot is the vector's");
        for key in ["swing_id", "session_id"] {
            assert_eq!(input[key], vector["input"][key], "{file}: {key}");
        }
        checked += 1;
    }
    assert!(
        missing.is_empty(),
        "corpus vectors with no shot case: {missing:?}"
    );
    assert!(checked > 0, "no corpus vector carries a shot");
}

fn is_flight(entry: &Value) -> bool {
    entry["name"]
        .as_str()
        .is_some_and(|n| n.starts_with("flight_"))
}

/// The verb with `stdin` piped in, as `tests/parse_screen.rs` runs `parse-screen`: the child reads
/// stdin to EOF before it writes a byte.
fn verb(args: &[&str], stdin: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_golf-core"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the binary runs");
    child
        .stdin
        .take()
        .expect("a piped stdin")
        .write_all(stdin.as_bytes())
        .expect("write stdin");
    child.wait_with_output().expect("the binary exits")
}

fn run_shot_verb(stdin: &str) -> Output {
    verb(&["run-shot"], stdin)
}

fn stdout_json(output: &Output, what: &str) -> Value {
    assert!(
        output.status.success(),
        "{what}: exit {:?}: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap_or_else(|e| panic!("{what}: stdout: {e}"))
}

/// Exactly the seam's value, with no tolerance: nothing but `serde_json` out and back separates
/// the two, and that keeps an `f64`'s bits.
#[test]
fn every_case_through_the_verb_is_the_seams_answer() {
    let cases = cases();
    let mut failures = Vec::new();
    for (name, document) in &cases {
        let got = stdout_json(&run_shot_verb(&document.to_string()), name);
        if got != seam(document, name) {
            failures.push(name.clone());
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} cases differ between the verb and run_shot: {failures:?}",
        failures.len(),
        cases.len()
    );
}

/// The bare `input` and the whole file are the same request, as they are for `run`.
#[test]
fn the_bare_input_reads_as_the_whole_file_does() {
    let document = case("stamped-tiles.json");
    let whole = run_shot_verb(&document.to_string());
    let bare = run_shot_verb(&document["input"].to_string());
    assert_eq!(stdout_json(&bare, "bare"), stdout_json(&whole, "whole"));
}

/// A device nobody declared is the caller's mistake: exit 1, nothing on stdout, and the refusal
/// naming the device on stderr.
#[test]
fn an_undeclared_device_exits_one_with_nothing_on_stdout() {
    let mut document = case("stamped-tiles.json");
    document["input"]["shot"]["provenance"]["device"] = json!("no_such_device");
    let output = run_shot_verb(&document.to_string());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(
        output.stdout.is_empty(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(
        stderr.contains("\"no_such_device\" is not a device devices.json declares"),
        "{stderr}"
    );
}

/// An engine vector is not a shot: neither form reads, and the verb says both ways it tried.
#[test]
fn stdin_that_is_not_a_shot_is_refused() {
    let output = run_shot_verb(r#"{"input": {"swing_id": "1", "session_id": "s", "face_on": {}}}"#);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(
        stderr.contains("as a vector:") && stderr.contains("as a vector input:"),
        "{stderr}"
    );
}

/// Any argument selects the session mode (M35 P11, `tests/verbs.rs`), so a flag that mode does not
/// take is a usage refusal, exit 2 as argparse's, and stdin is never read.
#[test]
fn a_flag_the_session_mode_does_not_take_is_a_usage_refusal() {
    let output = verb(&["run-shot", "--spec", "x"], "");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(2), "{stderr}");
    assert!(output.stdout.is_empty());
    assert!(
        stderr.contains("unrecognized arguments: --spec")
            && stderr.contains("usage:")
            && stderr.contains("run-shot --session"),
        "{stderr}"
    );
}
