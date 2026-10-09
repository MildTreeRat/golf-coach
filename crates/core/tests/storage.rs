//! The storage family against `crates/storage`. [M36 P8, P9, P10; the runner lifted at P13]
//!
//! `spec/vectors/storage/` was recorded once from frozen Python (M36 P1–P2) and is the oracle for
//! the stores: a case is a tree and either a sequence of store calls on it (`bundle/`, `stores/`,
//! recorded by `conformance_vectors.py::_run_ops`) or one `read_corpus` over it (`corpus/`, by
//! `_run_corpus`). Each vector goes through [`run_storage`], which runs each kind the way its
//! recorder did, picked by the vector's `provenance.recorded_by`, and its answer is compared with
//! `expected` under `docs/CONFORMANCE.md` §3's rules, through [`golf_core::compare`].
//!
//! # One definition with the re-record
//!
//! The runner was this file's until P13 lifted it into `golf_core::storage_family`, because
//! `golf-core rerecord` re-records this family with it: the gate and the recorder now run one
//! definition and judge its answer with one comparator. How an op's answer and a store's written
//! files are made comparable with what frozen Python recorded — a shot on frozen Python's keys, a
//! JSON file in the recorded text wherever its value agrees — is that module's doc, because it is
//! part of the answer rather than of this comparison. [`differences`] is [`compare`]'s verdict, the
//! same one the re-record gates on, with each moved file explained.
//!
//! # Every vector runs
//!
//! P8 ported the golfer store, the shot store and `slugify`, P9 the bag store and the bundle store,
//! and P10 `read_corpus`, so nothing in the family waits on a later phase and nothing is skipped. A
//! vector recorded by a runner this gate does not have fails it rather than being passed over.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use contracts::career::CAREER_VERSION;
use flate2::read::GzDecoder;
use golf_core::compare::compare;
use golf_core::storage_family::{differences, run_storage, StorageCase};
use serde::Serialize;
use serde_json::{Map, Value};
use storage::manifest::SwingManifest;
use storage::state::AnalysisState;

fn vectors_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../spec/vectors")
        .canonicalize()
        .expect("spec/vectors/ is committed")
}

fn read_vector(path: &Path) -> Value {
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

/// Every vector in the storage family, sorted by id. Duplicated from `crates/contracts/tests/
/// common/`, for `engine.rs`'s reason: integration tests are separate binaries.
fn storage_vectors() -> Vec<(String, Value)> {
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
    let mut paths = Vec::new();
    collect(&vectors_dir().join("storage"), &mut paths);
    let mut vectors: Vec<(String, Value)> = paths
        .iter()
        .map(|path| {
            let vector = read_vector(path);
            let id = vector["id"]
                .as_str()
                .expect("a vector carries its id")
                .to_string();
            (id, vector)
        })
        .collect();
    vectors.sort_by(|a, b| a.0.cmp(&b.0));
    assert!(!vectors.is_empty(), "no vectors under spec/vectors/storage");
    vectors
}

fn op_names(vector: &Value) -> Vec<&str> {
    vector["input"]["ops"]
        .as_array()
        .map(|ops| {
            ops.iter()
                .map(|op| op["op"].as_str().expect("an op name"))
                .collect()
        })
        .unwrap_or_default()
}

// ------------------------------------------------------------------------------------------ gates

/// **The gate**: every storage vector, run by the definition its recorder used, and compared in
/// full, in one report.
#[test]
fn every_storage_vector_conforms() {
    let mut found = Vec::new();
    let (mut ops, mut corpora) = (Vec::new(), Vec::new());
    for (id, vector) in storage_vectors() {
        assert_eq!(
            vector["career_version"].as_i64(),
            Some(CAREER_VERSION),
            "{id}: recorded at career_version {} against a port at {CAREER_VERSION}; re-record              with `golf-core rerecord` in the change that bumped it",
            vector["career_version"]
        );
        match StorageCase::of(&vector) {
            Ok(StorageCase::Ops) => ops.push(id.clone()),
            Ok(StorageCase::Corpus) => corpora.push(id.clone()),
            Err(why) => panic!("{why}"),
        }
        let answer = run_storage(&vector).expect("a recorder the runner has");
        found.extend(
            differences(&vector["expected"], &answer)
                .into_iter()
                .map(|d| format!("{id}: {d}")),
        );
    }
    assert!(
        !ops.is_empty() && !corpora.is_empty(),
        "the family lost a kind: {} op cases, {} corpus cases",
        ops.len(),
        corpora.len()
    );
    assert!(
        found.is_empty(),
        "{} difference(s) across {} vectors:
{}",
        found.len(),
        ops.len() + corpora.len(),
        found.join(
            "
"
        )
    );
    println!(
        "{} op cases and {} corpus cases conform: {}",
        ops.len(),
        corpora.len(),
        ops.iter()
            .chain(&corpora)
            .cloned()
            .collect::<Vec<_>>()
            .join(", ")
    );
}

/// **Every manifest and analysis state frozen Python read, read here and written back as the same
/// value.** The gate above runs every manifest through the bundle store since P9, but only the
/// manifests its ops reach; this is what holds `SwingManifest` and `AnalysisState` to every file on
/// the far side of the shape:
///
/// - every manifest a bundle op answered — `get_swing`, `get_session` and the three `set_*` repairs
///   — which is a manifest pydantic read and dumped;
/// - every `manifest.json` and `analysis.state.json` in the real corpus's tree, which is `data/`'s
///   own files as frozen Python wrote them.
///
/// **A file on disk may predate a field**, and the real manifests do: every one predates `mishit`,
/// and the earliest predate `club` too. That is the manifest's no-migration guarantee. Writing one
/// back adds the field at its default, exactly as pydantic's dump of the same file would, so a key
/// the file lacks is accepted when what Rust writes for it is a blank (`null`, `false`, `[]`, `{}`),
/// and reported when it is anything else.
#[test]
fn every_manifest_and_state_frozen_python_read_round_trips() {
    fn round_trip<T: serde::de::DeserializeOwned + Serialize>(
        at: &str,
        given: &Value,
    ) -> Vec<String> {
        match serde_json::from_value::<T>(given.clone()) {
            Err(e) => vec![format!("{at}: refused ({e})")],
            Ok(parsed) => {
                let mut written = serde_json::to_value(&parsed).expect("serializes");
                drop_blank_additions(&mut written, given);
                compare(given, &written)
                    .iter()
                    .map(|d| format!("{at}: {d}"))
                    .collect()
            }
        }
    }

    /// Remove every key `written` has and `given` lacks whose value is a blank, at any depth.
    fn drop_blank_additions(written: &mut Value, given: &Value) {
        match (written, given) {
            (Value::Object(written), Value::Object(given)) => {
                written.retain(|key, value| {
                    given.contains_key(key)
                        || !matches!(value, Value::Null | Value::Bool(false))
                            && !value.as_array().is_some_and(Vec::is_empty)
                            && !value.as_object().is_some_and(Map::is_empty)
                });
                for (key, value) in written.iter_mut() {
                    if let Some(given) = given.get(key) {
                        drop_blank_additions(value, given);
                    }
                }
            }
            (Value::Array(written), Value::Array(given)) => {
                for (written, given) in written.iter_mut().zip(given) {
                    drop_blank_additions(written, given);
                }
            }
            _ => {}
        }
    }

    let mut problems = Vec::new();
    let mut manifests = 0;
    let mut states = 0;
    for (id, vector) in storage_vectors() {
        if id.starts_with("storage/bundle/") {
            let ops = op_names(&vector);
            let results = vector["expected"]["results"].as_array().expect("results");
            for (i, (op, result)) in ops.iter().zip(results).enumerate() {
                if !matches!(
                    *op,
                    "bundle.get_swing"
                        | "bundle.get_session"
                        | "bundle.set_player"
                        | "bundle.set_club"
                        | "bundle.set_mishit"
                ) {
                    continue;
                }
                let answered: Vec<&Value> = match &result["returned"] {
                    Value::Array(many) => many.iter().collect(),
                    Value::Null => Vec::new(),
                    one => vec![one],
                };
                for manifest in answered {
                    manifests += 1;
                    problems.extend(round_trip::<SwingManifest>(
                        &format!("{id}: results[{i}] ({op})"),
                        manifest,
                    ));
                }
            }
        }
        if id == "storage/corpus/real" {
            for (name, text) in vector["input"]["files"].as_object().expect("files") {
                let parsed =
                    || serde_json::from_str::<Value>(text.as_str().expect("text")).expect("JSON");
                let at = format!("{id}: files[{name}]");
                if name.ends_with("/manifest.json") {
                    manifests += 1;
                    problems.extend(round_trip::<SwingManifest>(&at, &parsed()));
                } else if name.ends_with("/analysis.state.json") {
                    states += 1;
                    problems.extend(round_trip::<AnalysisState>(&at, &parsed()));
                }
            }
        }
    }
    assert!(
        manifests > 0 && states > 0,
        "the walk found nothing to read"
    );
    assert!(
        problems.is_empty(),
        "{} problem(s) over {manifests} manifests and {states} states:\n{}",
        problems.len(),
        problems.join("\n")
    );
    println!("{manifests} manifests and {states} analysis states round-trip");
}
