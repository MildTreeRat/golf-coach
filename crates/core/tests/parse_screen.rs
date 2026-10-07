//! `golf-core parse-screen` against the committed screen documents. [M34 P6]
//!
//! `crates/screen/tests/hand.rs` holds [`screen::read`] to every hand-worked shot, and
//! `crates/screen/tests/read.rs` to every shot frozen Python recorded and M34 P10 re-recorded. This
//! holds the verb to the library: every document, piped through the binary, comes back as
//! exactly the value `read` returns for it, so the verb passes every shot the library does, and adds
//! nothing between stdin and stdout but the pipe. Exactly means equal parsed values with no
//! tolerance, since no arithmetic separates the two: the only crossing is `serde_json` out and back,
//! which keeps an `f64`'s bits (`float_roundtrip`).
//!
//! Then the verb's own edges, which no library test can see: a failed read is `null` on exit 0, a
//! bare `input` reads as the whole file does, and an unknown device is exit 1 with nothing on stdout.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use screen::ScreenInput;
use serde_json::{json, Value};

/// The screen sub-families shaped as documents. `units/` tables are not screens.
const DOCUMENTS: [&str; 4] = ["corpus", "reference", "synthetic", "hand"];

/// The one document frozen Python could not read, `screen/synthetic/unreadable`'s file.
const UNREADABLE: &str = "synthetic/unreadable.json";

fn screen_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec/vectors/screen")
}

/// Every document as `(path relative to the screen family, its text)`, sorted.
fn documents() -> Vec<(String, String)> {
    let mut found = Vec::new();
    for family in DOCUMENTS {
        let dir = screen_dir().join(family);
        for entry in fs::read_dir(&dir).unwrap_or_else(|e| panic!("read {dir:?}: {e}")) {
            let path = entry.expect("a directory entry").path();
            let name = path.file_name().expect("a named file").to_string_lossy();
            let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path:?}: {e}"));
            found.push((format!("{family}/{name}"), text));
        }
    }
    found.sort();
    assert!(
        !found.is_empty(),
        "no screen documents under {:?}",
        screen_dir()
    );
    found
}

fn document(relative: &str) -> String {
    let path = screen_dir().join(relative);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path:?}: {e}"))
}

/// The verb with `stdin` piped in. The child reads stdin to EOF before it writes a byte, so writing
/// all of it first and then collecting the output cannot fill a pipe both ends are waiting on.
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

fn parse_screen(stdin: &str) -> Output {
    verb(&["parse-screen"], stdin)
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

/// What the library makes of a document, serialized as the verb would.
fn library(text: &str) -> Value {
    let vector: Value = serde_json::from_str(text).expect("a committed document parses");
    let input: ScreenInput =
        serde_json::from_value(vector["input"].clone()).expect("a document's input reads");
    let shot = screen::read(&input).expect("every document names a shipped profile");
    serde_json::to_value(shot).expect("a read serializes")
}

#[test]
fn every_document_through_the_verb_is_the_librarys_read() {
    let mut failures = Vec::new();
    let documents = documents();
    for (name, text) in &documents {
        let got = stdout_json(&parse_screen(text), name);
        if got != library(text) {
            failures.push(name.clone());
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} documents differ between the verb and screen::read: {failures:?}",
        failures.len(),
        documents.len()
    );
}

/// `import_screen`'s `failed` is an answer: `null`, and a clean exit.
#[test]
fn a_failed_read_is_null_on_a_clean_exit() {
    let output = parse_screen(&document(UNREADABLE));
    assert_eq!(stdout_json(&output, UNREADABLE), Value::Null);
}

/// The bare `input` and the whole file are the same request, as they are for `run`.
#[test]
fn the_bare_input_reads_as_the_whole_file_does() {
    let (name, text) = documents()
        .into_iter()
        .find(|(name, _)| name.starts_with("corpus/"))
        .expect("a corpus document");
    let vector: Value = serde_json::from_str(&text).expect("a committed document parses");
    let bare = vector["input"].to_string();

    let whole = parse_screen(&text);
    let from_bare = parse_screen(&bare);
    assert_eq!(stdout_json(&from_bare, &name), stdout_json(&whole, &name));
    assert_ne!(
        stdout_json(&whole, &name),
        Value::Null,
        "{name} reads as a shot"
    );
}

/// A device no profile names is the caller's mistake, not a failed read: exit 1, nothing on stdout,
/// and Python's words for it on stderr.
#[test]
fn an_unknown_device_exits_one_with_nothing_on_stdout() {
    let mut vector: Value = serde_json::from_str(&document(UNREADABLE)).expect("parses");
    vector["input"]["device"] = json!("no_such_device");
    let output = parse_screen(&vector.to_string());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success());
    assert!(
        output.stdout.is_empty(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(
        stderr.contains("Unknown launch-monitor profile 'no_such_device'"),
        "{stderr}"
    );
}

/// Neither a screen vector nor its input, and the verb says both ways it tried.
#[test]
fn stdin_that_is_not_a_screen_is_refused() {
    let output = parse_screen(r#"{"input": {"swing_id": "an engine vector"}}"#);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(
        stderr.contains("as a vector:") && stderr.contains("as a vector input:"),
        "{stderr}"
    );
}

#[test]
fn the_verb_takes_no_arguments() {
    let output = verb(&["parse-screen", "--spec", "x"], "");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success());
    assert!(
        stderr.contains("usage:") && stderr.contains("parse-screen"),
        "{stderr}"
    );
}
