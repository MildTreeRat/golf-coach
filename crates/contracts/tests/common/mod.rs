//! What the many-shot families' runners share: where the vectors are, how one is read, and how two
//! values are said to differ. [M36 P6, from `stores.rs`'s P5 helpers]
//!
//! Lifted out when `career.rs` became the second runner over `spec/vectors/{storage,career}/`, on
//! `crates/screen/tests/common`'s precedent, so the two cannot disagree about which files a family
//! holds or how a `.json.gz` is opened.

// Each test binary that includes this uses part of it.
#![allow(dead_code)]

use std::collections::BTreeSet;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use flate2::read::GzDecoder;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;

pub fn vectors_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../spec/vectors")
        .canonicalize()
        .expect("spec/vectors/ is missing. It is committed, so restore it from git")
}

/// Every vector under `family`, sorted, so a report reads in the same order on every machine.
pub fn vector_files(family: &str) -> Vec<PathBuf> {
    fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
        for entry in fs::read_dir(dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())) {
            let path = entry.expect("a directory entry").path();
            let name = path.to_string_lossy();
            if path.is_dir() {
                collect(&path, out);
            } else if name.ends_with(".json") || name.ends_with(".json.gz") {
                out.push(path);
            }
        }
    }
    let mut out = Vec::new();
    collect(&vectors_dir().join(family), &mut out);
    out.sort();
    assert!(!out.is_empty(), "no vectors under spec/vectors/{family}");
    out
}

pub fn read_vector(path: &Path) -> Value {
    let bytes = fs::read(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let text = if path.extension().is_some_and(|ext| ext == "gz") {
        let mut text = String::new();
        GzDecoder::new(bytes.as_slice())
            .read_to_string(&mut text)
            .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        text
    } else {
        String::from_utf8(bytes).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
    };
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// Where two values differ, as key paths. A whole-value `assert_eq!` prints two bags in full to
/// say that one timestamp moved.
pub fn differences(written: &Value, given: &Value, path: &str, out: &mut Vec<String>) {
    match (written, given) {
        (Value::Object(w), Value::Object(g)) => {
            let keys: BTreeSet<&String> = w.keys().chain(g.keys()).collect();
            for key in keys {
                let here = format!("{path}.{key}");
                match (w.get(key), g.get(key)) {
                    (Some(w), Some(g)) => differences(w, g, &here, out),
                    (Some(_), None) => out.push(format!("{here}: written, not given")),
                    (None, Some(_)) => out.push(format!("{here}: given, not written")),
                    (None, None) => unreachable!(),
                }
            }
        }
        (Value::Array(w), Value::Array(g)) if w.len() == g.len() => {
            for (i, (w, g)) in w.iter().zip(g).enumerate() {
                differences(w, g, &format!("{path}[{i}]"), out);
            }
        }
        _ if written == given => {}
        _ => out.push(format!("{path}: written {written}, given {given}")),
    }
}

/// Read `given` as a `T` and write it back: the differences, each prefixed by `at`, or the refusal.
/// The parsed value comes back too, for a caller that goes on to ask it something.
pub fn round_trip<T: DeserializeOwned + Serialize>(
    at: &str,
    given: &Value,
    failures: &mut Vec<String>,
) -> Option<T> {
    match serde_json::from_value::<T>(given.clone()) {
        Err(e) => {
            failures.push(format!("{at}: refused ({e})"));
            None
        }
        Ok(parsed) => {
            let written = serde_json::to_value(&parsed).expect("serialize");
            let mut out = Vec::new();
            differences(&written, given, "", &mut out);
            failures.extend(out.into_iter().map(|d| format!("{at}: {d}")));
            Some(parsed)
        }
    }
}
