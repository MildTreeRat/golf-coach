//! Every bag and golfer the two M36 families hold, read into this crate and written back. [M36 P5]
//!
//! `spec/vectors/storage/` and `spec/vectors/career/` were recorded from frozen Python (M36 P2 and
//! P3), and they are the only bags and golfers on record that pydantic wrote: the bag and golfer
//! stores' files before and after each op, every bag and golfer an op returned, the bags `bag.save`
//! was handed, and each career case's `input.bag`. No store is ported yet (that is P8 and P9), so
//! this asks the one question P5 can be wrong about, as `round_trip.rs` asks it of the engine
//! family: **does every field survive the crossing?**
//!
//! - A **whole** bag or golfer pydantic wrote must read, and write back as the same value. Each one
//!   is a `model_dump` (or a `model_dump_json` file), so it carries every key, and the comparison is
//!   exact: no arithmetic happened, so a float goes out with the bits it came in with, and a
//!   timestamp comes back in pydantic's spelling with its own offset kept (`bag-declared` holds a
//!   `-05:00` and a `+05:30` entry).
//! - A file frozen Python **refuses** must be refused here too. [`REFUSED`] names each one, measured
//!   at M36 P5 with `model_validate_json`, and where the refusal is the contract's own validator it
//!   carries the message, which is what the bag store's write guard shows a golfer.
//! - A **partial** entry an op was handed (`bag.set_entry`'s `args.entry`, written with only the keys
//!   its caller knew) must read and write back as pydantic's dump of it: every key given, at its
//!   value, and every other key at its blank. That is what the dump was, measured at P5.
//!
//! What a store *does* with these shapes is not asked here: that is `crates/core/tests/storage.rs`,
//! from P8.

mod common;

use std::collections::BTreeSet;

use common::{differences, read_vector, round_trip, vector_files};
use contracts::bag::{Bag, BagEntry};
use contracts::golfer::Golfer;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;

/// One row of [`REFUSED`].
type Refused = (
    &'static str,
    &'static str,
    &'static [&'static str],
    Option<&'static str>,
);

/// The sides of a vector a file can sit on.
const BOTH: &[&str] = &["input", "expected"];
const INPUT: &[&str] = &["input"];

/// Every bag and golfer file in the two families that frozen Python refuses to read, as `(vector
/// id, file name, the sides it is refused on, the validator's message)`. The message is `None` where
/// the refusal is pydantic's own (not JSON, a required key missing), which no validator here words.
///
/// A file sits in a vector's `input.files` and again in its `expected.files` unless an op wrote
/// over it, which is why the sides are named: `golfer.get_or_create` writes a fresh record over
/// `corrupt.golfer.json` (P2 finding 4), so `expected` holds a golfer both languages read. [`walk`]
/// fails on an entry it never met, so this list cannot outlive the files it names.
const REFUSED: [Refused; 9] = [
    ("storage/stores/bag-reads", "bob.bag.json", BOTH, None),
    (
        "storage/stores/bag-reads",
        "carol.bag.json",
        BOTH,
        Some("entry filed under 7i declares itself pw"),
    ),
    ("storage/stores/bag-reads", "dave.bag.json", BOTH, None),
    (
        "storage/stores/bag-reads",
        "erin.bag.json",
        BOTH,
        Some("player_id 'Erin' is not a slug (expected ^[a-z0-9]+(?:-[a-z0-9]+)*$)"),
    ),
    (
        "storage/stores/bag-reads",
        "frank.bag.json",
        BOTH,
        Some("entry in slot 7i is in the bag but carries retired_at"),
    ),
    (
        "storage/stores/bag-write-guard",
        "aaron.bag.json",
        BOTH,
        None,
    ),
    (
        "storage/stores/bag-write-guard",
        "carol.bag.json",
        BOTH,
        Some("entry filed under 7i declares itself pw"),
    ),
    (
        "storage/stores/golfer-reads",
        "badid.golfer.json",
        BOTH,
        Some("player_id 'Bad Id' is not a slug (expected ^[a-z0-9]+(?:-[a-z0-9]+)*$)"),
    ),
    (
        "storage/stores/golfer-reads",
        "corrupt.golfer.json",
        INPUT,
        None,
    ),
];

/// The partial-entry rule: every key given comes back at its value (recursively, so a nested
/// provenance may gain its `notes`), and every key not given comes back blank — `null`, or `""` for
/// the free-text fields.
fn widened(written: &Value, given: &Value, path: &str, out: &mut Vec<String>) {
    match (written, given) {
        (Value::Object(w), Value::Object(g)) => {
            for (key, value) in w {
                let here = format!("{path}.{key}");
                match g.get(key) {
                    Some(given) => widened(value, given, &here, out),
                    None if value.is_null() || value.as_str() == Some("") => {}
                    None => out.push(format!("{here}: not given, written {value}")),
                }
            }
            for key in g.keys().filter(|key| !w.contains_key(*key)) {
                out.push(format!("{path}.{key}: given, not written"));
            }
        }
        _ => differences(written, given, path, out),
    }
}

/// What the walk met, so a walk that met nothing cannot pass.
#[derive(Default)]
struct Seen {
    bag_files: usize,
    golfer_files: usize,
    bag_answers: usize,
    golfer_answers: usize,
    saved_bags: usize,
    entry_args: usize,
    career_bags: usize,
    refused: BTreeSet<(String, String, String)>,
}

struct Walk {
    seen: Seen,
    failures: Vec<String>,
}

impl Walk {
    /// A whole shape pydantic wrote: it reads, and writes back as the same value.
    fn whole<T: DeserializeOwned + Serialize>(&mut self, at: &str, given: &Value) {
        round_trip::<T>(at, given, &mut self.failures);
    }

    /// A file's text: refused when [`REFUSED`] says frozen Python refuses it, else a whole shape.
    fn file<T: DeserializeOwned + Serialize>(
        &mut self,
        id: &str,
        side: &str,
        name: &str,
        text: &str,
    ) {
        let at = format!("{id} {side}.files[{name}]");
        let refusal = REFUSED.iter().find(|(vector, file, sides, _)| {
            *vector == id && *file == name && sides.contains(&side)
        });
        match (refusal, serde_json::from_str::<T>(text)) {
            (Some((_, _, _, message)), Err(e)) => {
                self.seen
                    .refused
                    .insert((id.to_string(), name.to_string(), side.to_string()));
                if let Some(message) = message {
                    let e = e.to_string();
                    if !e.contains(message) {
                        self.failures.push(format!(
                            "{at}: refused, but not with Python's {message:?}: {e}"
                        ));
                    }
                }
            }
            (Some(_), Ok(_)) => self.failures.push(format!(
                "{at}: frozen Python refuses this file, and Rust read it"
            )),
            (None, _) => {
                let given: Value = serde_json::from_str(text).unwrap_or_else(|e| {
                    panic!("{at}: frozen Python read this, so it is JSON: {e}")
                });
                self.whole::<T>(&at, &given);
            }
        }
    }

    fn files(&mut self, id: &str, side: &str, files: Option<&Value>) {
        let Some(files) = files.and_then(Value::as_object) else {
            return;
        };
        for (name, text) in files {
            let text = text.as_str().expect("a file's text");
            if name.ends_with(".bag.json") {
                self.seen.bag_files += 1;
                self.file::<Bag>(id, side, name, text);
            } else if name.ends_with(".golfer.json") {
                self.seen.golfer_files += 1;
                self.file::<Golfer>(id, side, name, text);
            }
        }
    }

    fn ops(&mut self, id: &str, input: &Value, expected: &Value) {
        let (Some(ops), Some(results)) = (input["ops"].as_array(), expected["results"].as_array())
        else {
            return;
        };
        for (i, (op, result)) in ops.iter().zip(results).enumerate() {
            let name = op["op"].as_str().expect("an op name");
            let args = &op["args"];
            let at = format!("{id} ops[{i}] {name}");
            if name.starts_with("bag.") {
                if let Some(entry) = args.get("entry") {
                    self.seen.entry_args += 1;
                    match serde_json::from_value::<BagEntry>(entry.clone()) {
                        Err(e) => self
                            .failures
                            .push(format!("{at} args.entry: refused ({e})")),
                        Ok(parsed) => {
                            let written = serde_json::to_value(&parsed).expect("serialize");
                            let mut out = Vec::new();
                            widened(&written, entry, "", &mut out);
                            self.failures
                                .extend(out.into_iter().map(|d| format!("{at} args.entry: {d}")));
                        }
                    }
                }
                if let Some(bag) = args.get("bag") {
                    self.seen.saved_bags += 1;
                    self.whole::<Bag>(&format!("{at} args.bag"), bag);
                }
            }
            let Some(returned) = result.get("returned") else {
                continue;
            };
            if name.starts_with("bag.") && returned.is_object() {
                self.seen.bag_answers += 1;
                self.whole::<Bag>(&format!("{at} returned"), returned);
            } else if name.starts_with("golfer.") {
                let answers = match returned {
                    Value::Array(list) => list.iter().collect(),
                    Value::Object(_) => vec![returned],
                    _ => vec![],
                };
                for (j, golfer) in answers.into_iter().enumerate() {
                    self.seen.golfer_answers += 1;
                    self.whole::<Golfer>(&format!("{at} returned[{j}]"), golfer);
                }
            }
        }
    }
}

fn walk() -> Walk {
    let mut walk = Walk {
        seen: Seen::default(),
        failures: Vec::new(),
    };
    for family in ["storage", "career"] {
        for path in vector_files(family) {
            let vector = read_vector(&path);
            let id = vector["id"].as_str().expect("a vector id").to_string();
            let (input, expected) = (&vector["input"], &vector["expected"]);
            walk.files(&id, "input", input.get("files"));
            walk.files(&id, "expected", expected.get("files"));
            walk.ops(&id, input, expected);
            if let Some(bag) = input.get("bag").filter(|bag| !bag.is_null()) {
                walk.seen.career_bags += 1;
                walk.whole::<Bag>(&format!("{id} input.bag"), bag);
            }
        }
    }
    walk
}

#[test]
fn every_bag_and_golfer_python_wrote_reads_and_writes_back_as_the_same_value() {
    let Walk { seen, failures } = walk();

    for (what, count) in [
        ("bag files", seen.bag_files),
        ("golfer files", seen.golfer_files),
        ("bags an op returned", seen.bag_answers),
        ("golfers an op returned", seen.golfer_answers),
        ("bags `bag.save` was handed", seen.saved_bags),
        ("entries `bag.set_entry` was handed", seen.entry_args),
        ("career cases' bags", seen.career_bags),
    ] {
        assert!(
            count > 0,
            "the walk met no {what}, so it proves nothing about them"
        );
    }
    let named: BTreeSet<(String, String, String)> = REFUSED
        .iter()
        .flat_map(|(id, name, sides, _)| {
            sides
                .iter()
                .map(|side| (id.to_string(), name.to_string(), side.to_string()))
        })
        .collect();
    let unmet: Vec<_> = named.difference(&seen.refused).collect();
    assert!(
        unmet.is_empty(),
        "REFUSED names files the walk never refused, so the list is stale: {unmet:?}"
    );
    assert!(
        failures.is_empty(),
        "{} differences between crates/contracts and what frozen Python wrote:\n  {}",
        failures.len(),
        failures.join("\n  ")
    );
}
