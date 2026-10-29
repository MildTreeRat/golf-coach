//! Every stored `{role}.audio.json` round-trips through `storage::audio_io`. [M29 P20]
//!
//! The strike cache's gate, and the lab's because the lab reads and writes these files from P21
//! on. Frozen Python wrote every one of them (`api/pipeline.py::audio_for`), and ADR-035 clause 4
//! has the two languages share them until M40, so Rust must read each and write back the same
//! value: [`load_audio`] reads it, [`save_audio`] writes it into a scratch copy, and the written
//! JSON is held **structurally** to the stored JSON, every key and value as `json.loads` would see
//! it. Bytes are not compared, because `serde_json` and pydantic spell floats differently (the
//! `contracts` crate doc's "one thing the gate cannot ask for").
//!
//! It needs `data/processed/sessions/`, which is gitignored, so it is ignored, and it refuses to
//! pass with nothing to read: a missing directory or one holding no audio file fails.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;
use storage::audio_io::{load_audio, save_audio};

fn sessions_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/processed/sessions")
}

/// Every `*.audio.json` under `dir`, sorted, as paths relative to it.
fn audio_files(dir: &Path, under: &Path, found: &mut Vec<PathBuf>) {
    let mut entries: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .map(|entry| entry.expect("an entry").path())
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            audio_files(&path, under, found);
        } else if path
            .file_name()
            .is_some_and(|name| name.to_string_lossy().ends_with(".audio.json"))
        {
            found.push(path.strip_prefix(under).expect("under").to_path_buf());
        }
    }
}

#[test]
#[ignore = "needs data/processed/sessions/, the stored bundles (gitignored)"]
fn every_stored_audio_file_round_trips() {
    let sessions = sessions_dir();
    assert!(
        sessions.is_dir(),
        "{} is not a directory: this gate reads the stored bundles",
        sessions.display()
    );
    let mut found = Vec::new();
    audio_files(&sessions, &sessions, &mut found);
    assert!(
        !found.is_empty(),
        "no *.audio.json under {}",
        sessions.display()
    );

    let scratch = tempfile::tempdir().unwrap();
    let mut failures = Vec::new();
    for relative in &found {
        let stored_path = sessions.join(relative);
        let stored: Value = serde_json::from_slice(&fs::read(&stored_path).unwrap()).unwrap();
        let read = match load_audio(&stored_path) {
            Ok(read) => read,
            Err(error) => {
                failures.push(format!("{}: refused: {error}", relative.display()));
                continue;
            }
        };
        let copy = scratch.path().join(relative);
        save_audio(&read, &copy).unwrap();
        let written: Value = serde_json::from_slice(&fs::read(&copy).unwrap()).unwrap();
        if written != stored {
            failures.push(format!(
                "{}: wrote {written} where the store holds {stored}",
                relative.display()
            ));
        }
        if load_audio(&copy).unwrap() != read {
            failures.push(format!("{}: reads back differently", relative.display()));
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} stored audio files:\n{}",
        failures.len(),
        found.len(),
        failures.join("\n")
    );
    eprintln!("{} stored audio files round-trip", found.len());
}
