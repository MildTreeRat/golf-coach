//! The storage family's runner: one vector under `spec/vectors/storage/` in, Rust's answer to it
//! out, in `expected`'s shape. [M36 P8–P10, lifted into the library at P13]
//!
//! `spec/vectors/storage/` was recorded once from frozen Python (M36 P1–P2), and a case is a tree
//! and either a sequence of store calls on it (`bundle/`, `stores/`, recorded by
//! `conformance_vectors.py::_run_ops`) or one `read_corpus` over it (`corpus/`, by `_run_corpus`).
//! [`run_storage`] runs each kind the way its recorder did, picked by the vector's
//! `provenance.recorded_by` — or, on a hand-worked case under `hand/` (M36 P14), by the
//! `provenance.worked_against` that names the same definition ([`StorageCase::of`]):
//!
//! - an **op case**: `input.files` laid out as a scratch root, every store opened on it, each op in
//!   order, the root read back after the last;
//! - a **corpus case**: `input.files` laid out as a scratch sessions root, `read_corpus` over it for
//!   `input.player_id` under `input.versions`, then `narrow_to` once per entry of
//!   `input.narrowings` over that one corpus. The answer is each corpus as it serializes, beside the
//!   nine counts `CareerCorpus` derives (`properties`), which the dump leaves out and every report
//!   prints.
//!
//! # One definition, two callers
//!
//! `tests/storage.rs` is the family's gate and `golf-core rerecord` is its recorder (the M36 plan's
//! P13), and both call [`run_storage`] and judge its answer with [`crate::compare`]. Until P13 the
//! runner lived in the gate; it moved here so that a re-record writes what the gate checks and
//! nothing else. Lifting it made `storage` a normal dependency of this crate.
//!
//! # The answer is the family's spelling of Rust's
//!
//! Two things make a store's answer comparable with what frozen Python recorded, and both are part
//! of the answer rather than of the comparison, so the gate and the re-record cannot apply them
//! differently:
//!
//! - **A shot is on the keys frozen Python wrote** (M36 P2's finding 6, settled at P8). Rust's
//!   `ShotData` is M32's wider shape and writes ten keys frozen Python's has no field for. Each must
//!   be at its default, which is what a shot parsed by frozen Python's rules holds, and is then
//!   dropped from `shot.get` and `shot.all` answers. A Rust-only key at any other value stays, so it
//!   is an added key to the comparator. `crates/storage::shot_store`'s doc says why the store does
//!   not leave them out instead.
//! - **A file whose recorded text is JSON is the recorded text wherever the values agree** (the M36
//!   plan's call 1). What pins a store's write is the value pydantic serialized, timestamps included
//!   since they are strings, and not its key order, indentation, float spelling or line endings. So
//!   [`run_storage`] keeps the committed text of every JSON file whose value Rust wrote too, under
//!   §3's rules and with a shot file on frozen Python's keys, as an in-tolerance float keeps
//!   Python's bits through a re-record. A file whose value moved is Rust's text, a move of the whole
//!   file to the comparator, and [`differences`] says where inside it. Anything else is compared as
//!   text.
//!
//! **A move inside a written file cannot be declared yet.** Its path is
//! `expected.files.<session>/<swing>/manifest.json`, and a ledger path cannot name a key holding a
//! `.` (`golf_core::rerecord`'s module doc, "What a declaration can name"), so a re-record that meets
//! one refuses it. Nothing in M36 moves a store's write; the first change that does (M35's manifest
//! key) is the one that builds the escape.
//!
//! # What a refusal is, and what a panic is
//!
//! A store's refusal is an answer: `expected.results` holds each op's `{"returned": value}` or
//! `{"raised": {type, message}}`, and [`StoreError`] maps onto the second by the Python exception
//! each variant stands for. A raise with `message: null` is pydantic's own refusal, whose wording no
//! port is asked to match. A message naming a file names it under the scratch root, which the
//! recorder spelled `<root>/` and so does this. A model argument (`bag.save`'s bag,
//! `bag.set_entry`'s entry) is read as the recorder's `model_validate` reads it, so a validator's
//! refusal there is an answer too.
//!
//! **A vector the runner cannot read is not an answer, and panics**, naming the place: an op the
//! recorder's `_op_table` does not name, an argument of the wrong type, a store write failing on the
//! disk. That is the gate's behaviour, kept, because it is one definition. A re-record that meets one
//! stops before it writes, since `rerecord::plan` writes nothing. The one refusal returned as an
//! `Err` is a vector recorded by a runner this module does not have, which is a question about the
//! family rather than a broken case in it.
//!
//! # The scratch root
//!
//! Each case runs on a directory of its own under the system temp dir, removed when the case ends
//! ([`Scratch`]), as the recorder's `tempfile.TemporaryDirectory` is. Hand-rolled rather than
//! `tempfile`'s, because the M36 plan names `tempfile` a dev-dependency only, and a recorder that
//! needed it would make it the binary's.

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use contracts::bag::{Bag, BagEntry};
use contracts::career::{CareerCorpus, Narrowing};
use contracts::club::ClubId;
use contracts::golfer::Handedness;
use contracts::mishit::MishitVerdict;
use contracts::shot::ShotData;
use contracts::{Timestamp, Validate};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::{json, Map, Value};
use storage::bag_store::BagStore;
use storage::bundle_store::{SwingBundleStore, Upload};
use storage::corpus::{narrow_to, read_corpus, EngineVersions};
use storage::golfer_store::{slugify, GolferStore};
use storage::manifest::Role;
use storage::shot_store::{ShotStore, SUFFIX as SHOT_SUFFIX};
use storage::StoreError;

use crate::compare::{compare, DifferenceKind};

/// `provenance.recorded_by` on an op case (`bundle/`, `stores/`).
pub const RUN_OPS: &str = "scripts/conformance_vectors.py::_run_ops";

/// `provenance.recorded_by` on a corpus case (`corpus/`), and `provenance.worked_against` on a
/// hand-worked one (`hand/`).
pub const RUN_CORPUS: &str = "scripts/conformance_vectors.py::_run_corpus";

/// `provenance.oracle` on a vector a person worked out rather than a program recorded. [M36 P14]
pub const HAND_ORACLE: &str = "hand";

/// Which recorder a storage vector names, and so which half of [`run_storage`] answers it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageCase {
    /// [`RUN_OPS`]: store calls on a tree, `expected` is `{results, files}`.
    Ops,
    /// [`RUN_CORPUS`]: one `read_corpus` and its narrowings, `expected` is
    /// `{corpus, properties, narrowed}`.
    Corpus,
}

impl StorageCase {
    /// The case `vector` is, by the runner its provenance names; an `Err` naming that runner if it
    /// is neither of the two.
    ///
    /// **A hand-worked vector names the definition it was worked against, in
    /// `provenance.worked_against`, and not a recorder** (M36 P14, P13's finding 9 left the choice
    /// open). Nothing recorded `spec/vectors/storage/hand/`: a person worked each answer out, as
    /// M34's screen cases were, so `recorded_by` there would claim a provenance the file does not
    /// have, and a reader of `recorded_by` — frozen Python's pins, `conformance.py list` — would
    /// count a hand case among the Python-recorded ones. The key a vector is read by follows its
    /// `oracle`, so neither kind can borrow the other's.
    pub fn of(vector: &Value) -> Result<Self, String> {
        let provenance = &vector["provenance"];
        let key = if provenance["oracle"] == HAND_ORACLE {
            "worked_against"
        } else {
            "recorded_by"
        };
        match provenance[key].as_str() {
            Some(RUN_OPS) => Ok(Self::Ops),
            Some(RUN_CORPUS) => Ok(Self::Corpus),
            other => Err(format!(
                "{}: {} {other:?}, a runner the storage family does not have ({RUN_OPS} or \
                 {RUN_CORPUS})",
                vector["id"].as_str().unwrap_or("<a vector with no id>"),
                key.replace('_', " "),
            )),
        }
    }
}

/// **The runner**: Rust's answer to one committed storage vector, in `expected`'s shape and in the
/// family's spelling (the module doc): a shot on frozen Python's keys, and each JSON file's committed
/// text wherever Rust wrote the same value.
///
/// Reads `vector`'s `input`, and its `expected` only for the spelling of the files. `Err` only for a
/// vector recorded by a runner this module does not have; a case it cannot read panics (the module
/// doc says why).
pub fn run_storage(vector: &Value) -> Result<Value, String> {
    match StorageCase::of(vector)? {
        StorageCase::Corpus => Ok(run_corpus(&vector["input"])),
        StorageCase::Ops => {
            let mut answer = run_ops(&vector["input"]);
            in_the_recorded_spelling(&vector["expected"]["files"], &mut answer["files"]);
            Ok(answer)
        }
    }
}

/// Every difference between a recorded `expected` and [`run_storage`]'s answer: [`compare`]'s, in
/// its order, which is exactly what a re-record gates on — each followed, when it is a written JSON
/// file that moved, by where inside the file, indented.
pub fn differences(expected: &Value, answer: &Value) -> Vec<String> {
    let mut out = Vec::new();
    for difference in compare(expected, answer) {
        out.push(difference.to_string());
        if let (
            Some(name),
            DifferenceKind::Moved {
                expected: Value::String(recorded),
                actual: Value::String(left),
            },
        ) = (difference.path.strip_prefix("files."), &difference.kind)
        {
            // A text file's move is the comparator's sentence already.
            if serde_json::from_str::<Value>(recorded).is_err() {
                continue;
            }
            out.extend(
                file_differences(name, recorded, left)
                    .into_iter()
                    .map(|inside| format!("  {inside}")),
            );
        }
    }
    out
}

// ---------------------------------------------------------------------------------- the op cases

/// Every store, opened on the one root, as `_open_stores` opens them.
struct Stores {
    bundle: SwingBundleStore,
    bags: BagStore,
    golfers: GolferStore,
    shots: ShotStore,
}

/// `_run_ops(input) -> expected`, with each shot answer on frozen Python's keys.
fn run_ops(input: &Value) -> Value {
    let scratch = Scratch::new();
    let root = scratch.path().join("root");
    materialise(&input["files"], &root);
    let stores = Stores {
        bundle: SwingBundleStore::new(&root),
        bags: BagStore::new(&root),
        golfers: GolferStore::new(&root),
        shots: ShotStore::new(&root),
    };
    let results: Vec<Value> = input["ops"]
        .as_array()
        .expect("ops")
        .iter()
        .map(|op| {
            let mut result = match run_op(&stores, &root, op) {
                Ok(returned) => json!({ "returned": returned }),
                Err(raised) => json!({ "raised": raised }),
            };
            python_shaped_answer(op["op"].as_str().unwrap_or_default(), &mut result);
            result
        })
        .collect();
    let files = if root.exists() {
        read_tree(&root)
    } else {
        Value::Null
    };
    json!({ "results": results, "files": files })
}

/// One call, `_op_table`'s entry for it. A refusal is an answer; an `OSError` is not, because frozen
/// Python's recorder stops on anything but a `ValueError`, so no vector holds one.
fn run_op(stores: &Stores, root: &Path, op: &Value) -> Result<Value, Value> {
    let args = &op["args"];
    let text = |key: &str| {
        args[key]
            .as_str()
            .unwrap_or_else(|| panic!("{op}: args.{key} is not a string"))
    };
    let now = || {
        Timestamp::parse(
            op["now"]
                .as_str()
                .expect("an op that may stamp carries a `now`"),
        )
        .expect("a recorded `now` parses")
    };
    let refused = |error: StoreError| raised(error, root);
    let wrote = "a store write succeeds";
    let (bundle, bags) = (&stores.bundle, &stores.bags);
    let name = op["op"].as_str().expect("an op name");
    match name {
        "bundle.list_session_ids" => Ok(json!(bundle.list_session_ids())),
        "bundle.get_session" => Ok(dumped(&bundle.get_session(text("session_id")))),
        "bundle.get_swing" => Ok(dumped(
            &bundle.get_swing(text("session_id"), text("swing_id")),
        )),
        "bundle.current_session_id" => {
            // `args.now` is the call's argument; when it is absent the store reads the clock, which
            // the recorder froze at the op's `now`.
            let at = match args["now"].as_str() {
                Some(given) => Timestamp::parse(given).expect("a given `now` parses"),
                None => now(),
            };
            Ok(json!(bundle.current_session_id(at)))
        }
        "bundle.assign_from_path" => {
            let tmp_path = text("tmp_path")
                .split('/')
                .fold(root.to_path_buf(), |path, part| path.join(part));
            let upload = Upload {
                session_id: text("session_id"),
                role: arg::<Role>(args, "role"),
                tmp_path: &tmp_path,
                digest: text("digest"),
                original_filename: text("original_filename"),
                content_type: text("content_type"),
                size_bytes: args["size_bytes"].as_i64().expect("a size"),
                swing_id: args["swing_id"].as_str(),
                player_id: args["player_id"].as_str(),
                club: arg::<Option<ClubId>>(args, "club"),
            };
            Ok(dumped(
                &bundle.assign_from_path(&upload, now()).expect(wrote),
            ))
        }
        "bundle.attribute_unlabeled" => Ok(json!(bundle
            .attribute_unlabeled(text("session_id"), text("player_id"), now())
            .expect(wrote))),
        "bundle.set_player" => Ok(dumped(
            &bundle
                .set_player(
                    text("session_id"),
                    text("swing_id"),
                    text("player_id"),
                    now(),
                )
                .expect(wrote),
        )),
        "bundle.set_club" => Ok(dumped(
            &bundle
                .set_club(
                    text("session_id"),
                    text("swing_id"),
                    arg::<ClubId>(args, "club"),
                    now(),
                )
                .expect(wrote),
        )),
        "bundle.set_mishit" => Ok(dumped(
            &bundle
                .set_mishit(
                    text("session_id"),
                    text("swing_id"),
                    arg::<Option<MishitVerdict>>(args, "verdict"),
                    now(),
                )
                .expect(wrote),
        )),
        "bundle.delete_swing" => Ok(json!(bundle
            .delete_swing(text("session_id"), text("swing_id"))
            .expect(wrote))),
        "bag.get" => Ok(dumped(&bags.get(text("player_id")))),
        "bag.save" => {
            let bag = model(Bag::deserialize(&args["bag"]), root)?;
            bags.save(&bag).map(|()| Value::Null).map_err(refused)
        }
        "bag.set_entry" => {
            let entry = model(BagEntry::deserialize(&args["entry"]), root)?;
            bags.set_entry(text("player_id"), &entry, now())
                .map(|bag| dumped(&bag))
                .map_err(refused)
        }
        "bag.remove_entry" => bags
            .remove_entry(text("player_id"), arg::<ClubId>(args, "club"), now())
            .map(|bag| dumped(&bag))
            .map_err(refused),
        "bag.restore_entry" => bags
            .restore_entry(text("player_id"), arg::<ClubId>(args, "club"), now())
            .map(|bag| dumped(&bag))
            .map_err(refused),
        "golfer.get" => Ok(dumped(&stores.golfers.get(text("player_id")))),
        "golfer.list_all" => Ok(dumped(&stores.golfers.list_all())),
        "golfer.get_or_create" => stores
            .golfers
            .get_or_create(text("name"), arg::<Handedness>(args, "handedness"), now())
            .map(|golfer| dumped(&golfer))
            .map_err(refused),
        "shot.put" => match serde_json::from_value::<ShotData>(args["shot"].clone()) {
            Err(_) => Err(json!({ "type": "ValidationError", "message": null })),
            Ok(shot) => {
                let path = stores.shots.put(&shot).expect(wrote);
                Ok(json!(relative(&path, root)))
            }
        },
        "shot.get" => stores
            .shots
            .get(text("key"))
            .map(|shot| dumped(&shot))
            .map_err(refused),
        "shot.has" => Ok(json!(stores.shots.has(text("key")))),
        "shot.all" => Ok(dumped(&stores.shots.all())),
        "slugify" => Ok(json!(slugify(text("name")))),
        other => panic!("{other} is not an op the recorder's `_op_table` names"),
    }
}

// ------------------------------------------------------------------------------ the corpus cases

/// `_run_corpus(input) -> expected`. Rust's reader takes the engine generations as an argument
/// where frozen Python's reads its own, so `input.versions` is passed as it was recorded; the
/// recorder refused any pair but its own `{16, 16}`.
fn run_corpus(input: &Value) -> Value {
    let scratch = Scratch::new();
    let root = scratch.path().join("sessions");
    materialise(&input["files"], &root);
    let player_id = input["player_id"].as_str().expect("a player id");
    let corpus = read_corpus(&root, player_id, arg::<EngineVersions>(input, "versions"));
    let narrowed: Map<String, Value> = input["narrowings"]
        .as_object()
        .expect("narrowings")
        .iter()
        .map(|(name, args)| {
            let narrowing: Narrowing = serde_json::from_value(args.clone())
                .unwrap_or_else(|e| panic!("narrowings.{name} = {args}: {e}"));
            (name.clone(), corpus_answer(&narrow_to(&corpus, &narrowing)))
        })
        .collect();
    let mut answer = corpus_answer(&corpus);
    answer["narrowed"] = Value::Object(narrowed);
    answer
}

/// `_corpus_answer`: the corpus as it serializes, and the counts it derives, which the dump
/// leaves out.
fn corpus_answer(corpus: &CareerCorpus) -> Value {
    json!({
        "corpus": dumped(corpus),
        "properties": {
            "distinct_swings": corpus.distinct_swings(),
            "distinct_shots": corpus.distinct_shots(),
            "distinct_sessions": corpus.distinct_sessions(),
            "untagged_swings": corpus.untagged_swings(),
            "duplicates_collapsed": corpus.duplicates_collapsed(),
            "shot_conflicts": corpus.shot_conflicts(),
            "mishit_shots": corpus.mishit_shots(),
            "mishit_refs": corpus.mishit_refs(),
            "mishit_shots_unconfirmed": corpus.mishit_shots_unconfirmed(),
        },
    })
}

// --------------------------------------------------------------------------------- the plumbing

/// An enum argument, which travels as its value (`_op_table`'s docstring).
fn arg<T: DeserializeOwned>(args: &Value, key: &str) -> T {
    serde_json::from_value(args[key].clone())
        .unwrap_or_else(|e| panic!("args.{key} = {}: {e}", args[key]))
}

/// A model argument as `Model.model_validate` reads it: refused by its shape with no message, by
/// its own validators with theirs. `deserialize` is each contract's unvalidated read (the
/// `#[serde(remote = "Self")]` half), so a validator's refusal keeps its `ContractError` rather than
/// being folded into serde's text, which is how a store's own refusal reads too.
fn model<T: Validate>(parsed: Result<T, serde_json::Error>, root: &Path) -> Result<T, Value> {
    let value = parsed.map_err(|_| json!({ "type": "ValidationError", "message": null }))?;
    value
        .validate()
        .map_err(|error| raised(StoreError::Invalid(error), root))?;
    Ok(value)
}

/// `_raised`: the Python exception's class, and its message where this repo wrote it, with the
/// scratch root spelled `<root>/` as the recorder spelled it.
fn raised(error: StoreError, root: &Path) -> Value {
    match error {
        StoreError::Refused(message) => {
            let scratch = format!("{}{}", root.display(), std::path::MAIN_SEPARATOR);
            json!({ "type": "ValueError", "message": message.replace(&scratch, "<root>/") })
        }
        StoreError::Invalid(error) => {
            json!({ "type": "ValidationError", "message": error.problem })
        }
        StoreError::Unreadable { .. } => json!({ "type": "ValidationError", "message": null }),
        StoreError::Io(error) => panic!("an OSError, which no recorded op raises: {error}"),
    }
}

fn dumped<T: Serialize>(value: &T) -> Value {
    serde_json::to_value(value).expect("a store's answer serializes")
}

fn relative(path: &Path, root: &Path) -> String {
    let rel = path
        .strip_prefix(root)
        .expect("a store writes under its root");
    rel.components()
        .map(|part| part.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

/// `_materialise`: `null` leaves the root absent, `{}` makes it empty, a key ending in `/` is a
/// directory, and every file is written as the UTF-8 of its text, byte for byte.
fn materialise(files: &Value, root: &Path) {
    let Some(files) = files.as_object() else {
        assert!(files.is_null(), "input.files is an object or null");
        return;
    };
    fs::create_dir_all(root).expect("the root");
    for (rel, text) in files {
        let parts: Vec<&str> = rel.trim_end_matches('/').split('/').collect();
        assert!(
            !rel.starts_with('/') && !parts.iter().any(|p| p.is_empty() || *p == ".."),
            "{rel:?} is not a path inside the root"
        );
        let target = parts
            .iter()
            .fold(root.to_path_buf(), |path, part| path.join(part));
        if rel.ends_with('/') {
            fs::create_dir_all(&target).expect("a directory");
        } else {
            fs::create_dir_all(target.parent().expect("a parent")).expect("a parent directory");
            fs::write(&target, text.as_str().expect("a file's text")).expect("a file");
        }
    }
}

/// `_read_tree`: every file as text with `\r\n` normalised, and an empty directory as its `/` key.
fn read_tree(root: &Path) -> Value {
    fn walk(root: &Path, dir: &Path, out: &mut Map<String, Value>) {
        for entry in fs::read_dir(dir).expect("a readable directory") {
            let path = entry.expect("an entry").path();
            let rel = relative(&path, root);
            if path.is_dir() {
                if fs::read_dir(&path).expect("a directory").next().is_none() {
                    out.insert(format!("{rel}/"), json!(""));
                }
                walk(root, &path, out);
            } else {
                let text = String::from_utf8(fs::read(&path).expect("a file"))
                    .expect("every file a store writes is UTF-8");
                out.insert(rel, json!(text.replace("\r\n", "\n")));
            }
        }
    }
    let mut out = Map::new();
    walk(root, root, &mut out);
    Value::Object(out)
}

/// A directory of the runner's own under the system temp dir, removed when dropped (the module doc,
/// "The scratch root"). Named by the process and a counter, so the gate's parallel tests and two
/// processes at once each get their own; one left behind by a dead process that had this one's id
/// is stepped over rather than reused.
struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let parent = std::env::temp_dir();
        loop {
            let n = NEXT.fetch_add(1, Ordering::Relaxed);
            let path = parent.join(format!("golf-core-storage-{}-{n}", std::process::id()));
            match fs::create_dir(&path) {
                Ok(()) => return Self(path),
                Err(e) if e.kind() == ErrorKind::AlreadyExists => {}
                Err(e) => panic!("a scratch directory under {}: {e}", parent.display()),
            }
        }
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

// ------------------------------------------------------------------------- the family's spelling

/// The keys Rust's `ShotData` writes and frozen Python's has no field for, at the value a shot
/// parsed by frozen Python's rules holds: M32's seven, then the provenance's three.
const RUST_ONLY_SHOT_KEYS: [&str; 7] = [
    "attack_angle",
    "dynamic_loft",
    "low_point",
    "impact_offset_h",
    "impact_offset_v",
    "impact_position_v",
    "carry_offline",
];

fn rust_only_provenance_keys() -> [(&'static str, Value); 3] {
    [
        ("parser_version", json!(0)),
        ("fields_present", Value::Null),
        ("corrections", json!({})),
    ]
}

/// A Rust-written shot on frozen Python's key set: each Rust-only key dropped when it is at its
/// default, and kept (to be reported as added) when it is not.
fn python_shaped_shot(shot: &mut Value) {
    let Some(object) = shot.as_object_mut() else {
        return;
    };
    for key in RUST_ONLY_SHOT_KEYS {
        if object.get(key) == Some(&Value::Null) {
            object.remove(key);
        }
    }
    if let Some(Value::Object(provenance)) = object.get_mut("provenance") {
        for (key, default) in rust_only_provenance_keys() {
            if provenance.get(key) == Some(&default) {
                provenance.remove(key);
            }
        }
    }
}

fn python_shaped_answer(op: &str, result: &mut Value) {
    if !matches!(op, "shot.get" | "shot.all") {
        return;
    }
    match result.get_mut("returned") {
        Some(Value::Array(shots)) => shots.iter_mut().for_each(python_shaped_shot),
        Some(shot) => python_shaped_shot(shot),
        None => {}
    }
}

/// Give every file Rust left the committed text wherever [`file_differences`] finds nothing between
/// the two — one definition of "the same file" for the gate and the re-record.
fn in_the_recorded_spelling(recorded: &Value, left: &mut Value) {
    let (Some(recorded), Some(left)) = (recorded.as_object(), left.as_object_mut()) else {
        return;
    };
    for (name, text) in left.iter_mut() {
        if let (Some(Value::String(was)), Value::String(now)) = (recorded.get(name), &*text) {
            if was != now && file_differences(name, was, now).is_empty() {
                *text = json!(was);
            }
        }
    }
}

/// What differs between a file's recorded text and the text Rust left, by the module doc's rule:
/// JSON compared as a value (a shot on frozen Python's keys), anything else as text. Empty is the
/// same file.
fn file_differences(name: &str, recorded: &str, left: &str) -> Vec<String> {
    let Ok(recorded) = serde_json::from_str::<Value>(recorded) else {
        return if recorded == left {
            Vec::new()
        } else {
            vec![format!("text {left:?}, recorded {recorded:?}")]
        };
    };
    match serde_json::from_str::<Value>(left) {
        Err(e) => vec![format!("recorded JSON, Rust left text ({e})")],
        Ok(mut left) => {
            if name.ends_with(SHOT_SUFFIX) {
                python_shaped_shot(&mut left);
            }
            compare(&recorded, &left)
                .iter()
                .map(ToString::to_string)
                .collect()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A JSON file Rust wrote in another spelling keeps the committed text; one whose value moved is
    /// Rust's text, and [`differences`] says where inside it; a text file is compared as text.
    #[test]
    fn a_file_keeps_the_recorded_text_exactly_where_its_value_agrees() {
        let recorded = json!({
            "a.json": "{\n  \"x\": 1.0,\n  \"y\": \"z\"\n}",
            "b.json": "{\"x\": 1}",
            "c.txt": "one",
            "d.txt": "two",
        });
        let mut left = json!({
            "a.json": "{\"y\":\"z\",\"x\":1.0}",
            "b.json": "{\"x\": 2}",
            "c.txt": "one",
            "d.txt": "three",
        });
        in_the_recorded_spelling(&recorded, &mut left);
        assert_eq!(
            left["a.json"], recorded["a.json"],
            "the same value, respelled"
        );
        assert_eq!(
            left["b.json"],
            json!("{\"x\": 2}"),
            "a moved value is Rust's"
        );
        assert_eq!(left["d.txt"], json!("three"));

        let expected = json!({"files": recorded});
        let answer = json!({"files": left});
        let found = differences(&expected, &answer);
        assert_eq!(found.len(), 3, "{found:#?}");
        assert!(found[0].starts_with("files.b.json: "), "{}", found[0]);
        assert_eq!(found[1], "  x: 1 != 2");
        assert!(found[2].starts_with("files.d.txt: "), "{}", found[2]);
    }

    /// A shot file is the same file when the only keys Rust adds are its own at their defaults, and
    /// a different one when one of them holds a value.
    #[test]
    fn a_shot_file_is_compared_on_frozen_pythons_keys() {
        let name = format!("abc{SHOT_SUFFIX}");
        let recorded = r#"{"shot_id": "1", "provenance": {"device": "d"}}"#;
        let at_defaults = r#"{"shot_id": "1", "attack_angle": null,
            "provenance": {"device": "d", "parser_version": 0, "corrections": {}}}"#;
        let with_a_value = r#"{"shot_id": "1", "attack_angle": -3.0,
            "provenance": {"device": "d"}}"#;
        assert!(file_differences(&name, recorded, at_defaults).is_empty());
        assert_eq!(file_differences(&name, recorded, with_a_value).len(), 1);
        assert_eq!(
            file_differences("abc.json", recorded, at_defaults).len(),
            3,
            "only a shot file is shaped"
        );
    }

    /// The scratch root is the runner's own and does not outlive it.
    #[test]
    fn a_scratch_root_is_unique_and_removed() {
        let (one, two) = (Scratch::new(), Scratch::new());
        assert_ne!(one.path(), two.path());
        let kept = one.path().to_path_buf();
        assert!(kept.is_dir());
        fs::write(kept.join("f"), "x").expect("write into the scratch root");
        drop(one);
        assert!(!kept.exists(), "the scratch root outlived its case");
    }

    #[test]
    fn a_runner_this_module_does_not_have_is_an_err() {
        let vector = json!({"id": "storage/x/y", "provenance": {"recorded_by": "by hand"}});
        let refused = StorageCase::of(&vector).expect_err("an unknown recorder");
        assert!(
            refused.starts_with("storage/x/y: recorded by Some(\"by hand\")"),
            "{refused}"
        );
        assert!(run_storage(&vector).is_err());
    }

    /// A hand-worked case is read by `worked_against` and by nothing else: naming the corpus runner
    /// there makes it a corpus case, and a hand case that names a recorder instead is refused, so a
    /// hand case cannot pass for a recorded one, nor a recorded one for a hand case. [M36 P14]
    #[test]
    fn a_hand_case_names_the_runner_it_was_worked_against() {
        let hand = json!({"id": "storage/hand/x", "provenance": {
            "oracle": HAND_ORACLE, "worked_against": RUN_CORPUS}});
        assert_eq!(StorageCase::of(&hand), Ok(StorageCase::Corpus));

        let claims_a_recorder = json!({"id": "storage/hand/y", "provenance": {
            "oracle": HAND_ORACLE, "recorded_by": RUN_CORPUS}});
        let refused = StorageCase::of(&claims_a_recorder).expect_err("no worked_against");
        assert!(
            refused.starts_with("storage/hand/y: worked against None"),
            "{refused}"
        );

        let recorded_naming_neither = json!({"id": "storage/corpus/z", "provenance": {
            "oracle": "python", "worked_against": RUN_CORPUS}});
        assert!(StorageCase::of(&recorded_naming_neither).is_err());
    }
}
