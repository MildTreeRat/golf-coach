//! The `mcp` family's runner: one tool case's `input` in, `golf-mcp`'s answer to every call out, in
//! `expected.results`' shape. [M29 P9]
//!
//! `spec/vectors/mcp/tools/` and `real/` were recorded once from frozen Python
//! (`conformance_vectors.py::_run_mcp`, M29 P8), and a case is a file tree, the server's three
//! roots inside it, and a list of calls. [`run_case`] lays the tree out in a scratch directory as
//! `_materialise` does, builds the server over it as `_mcp_server` does (`golfers: null` is the
//! registry-less shape), and answers each call in order under its own clock ([`Clock`]). The gate
//! (`tests/tools.rs`) compares those answers with the recorded ones, or skips a tool by name until
//! the phase that lands its body.
//!
//! # The answer's shape
//!
//! `{content, structuredContent, isError}`, as the wire keys them, with `null` where the wire
//! leaves one out (P8's finding 3), so an answer here is one the recorded case can be held to as a
//! value. A protocol error is not a recorded answer: the recorder refused any, so the runner
//! returns it as `Err` for the gate to fail on by name.
//!
//! # What `golf-mcp` answers differently on purpose, by path [M29 P11, P12, P14, P17, P32, P34]
//!
//! [`DECLARED`] is the list, and [`as_declared`] applies it: a recorded answer rewritten by each
//! declared rule at its path, which is what the gate then holds Rust's answer to, under the same
//! comparison as every other path. So a declaration does not loosen the comparison at a path; it
//! states what the value there must become, and anything else there, or anywhere outside the
//! declared paths, still fails. A declaration that moves no recorded answer is stale, and the gate
//! fails that too.
//!
//! Here rather than in the gate because the list is the family's, not one test's. The surface's
//! five differences are declared in `tests/surface.rs`, beside the one test that reads them. The
//! tool cases are walked a second time, by the career re-record (the plan's P15), and that walk
//! must apply the list the gate applies. Nothing may depend on `golf-mcp`, so `golf-core` cannot
//! hold the list for both; this library can, and [`recorded_as_ours`] is the one place either
//! reads it through.
//!
//! # Three oracles, and which answers a declaration reaches [M29 P15]
//!
//! A Python-recorded case (`tools/`, `real/`) is frozen Python's answer, so [`DECLARED`] is applied
//! to it. A hand-worked case (`hand/`, P11 on) and a case recorded once by Rust (`rust/`, P15) are
//! already this server's answers ([`OURS_ALREADY`]), so nothing is applied to them: a declaration
//! that reached one would be rewriting `golf-mcp`'s answer into another of its own.
//!
//! # The runner as a process: [`answer_case`] [M29 P15]
//!
//! `golf-core rerecord` walks this family after the profile family under a career declaration,
//! and it cannot call this crate, which depends on it. So `golf-mcp answer-case` is this module
//! behind stdin and stdout: one case in, [`answer_case`]'s `{answers, recorded}` out, and the
//! recorder gates, applies and ledgers with its own rules, as it does for every other family. The
//! same verb is what recorded `rust/`'s cases once: a case with no `expected` answers with
//! `recorded: null`.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use serde_json::{json, Value};

use contracts::checkpoints::CHECKPOINT_REGISTRY;
use contracts::unscored::UnscoredReason;

use crate::server::{Clock, Dirs, GolfMcp};
use crate::{career, club, flight, query};

/// One recorded call: the tool, its arguments, and the `now` it was made at.
#[derive(Debug, Clone, PartialEq)]
pub struct Call {
    pub tool: String,
    pub arguments: serde_json::Map<String, Value>,
    pub now: Option<String>,
}

/// The calls of a case's `input`, in order.
pub fn calls(input: &Value) -> Vec<Call> {
    input["calls"]
        .as_array()
        .expect("input.calls is a list")
        .iter()
        .map(|call| Call {
            tool: call["tool"]
                .as_str()
                .expect("a call names its tool")
                .to_string(),
            arguments: call["arguments"]
                .as_object()
                .cloned()
                .expect("a call's arguments are an object"),
            now: call["now"].as_str().map(str::to_string),
        })
        .collect()
}

/// A scratch directory under the system's temporary one, removed when dropped. Hand-rolled, as
/// `golf_core::storage_family`'s is, rather than a `tempfile` dependency for one directory.
pub struct Scratch(PathBuf);

impl Scratch {
    pub fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        loop {
            let n = NEXT.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!("golf-mcp-{}-{n}", std::process::id()));
            if fs::create_dir(&path).is_ok() {
                return Scratch(path);
            }
        }
    }

    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Default for Scratch {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// `_materialise`: lay `input.files` out under `root`. A key ending `/` is a directory and carries
/// no text; every other is a file, written as the vector's UTF-8 bytes so no line ending is
/// substituted into it.
pub fn materialise(files: &Value, root: &Path) {
    let Some(files) = files.as_object() else {
        assert!(files.is_null(), "input.files is an object or null");
        return;
    };
    for (rel, text) in files {
        let parts: Vec<&str> = rel.trim_end_matches('/').split('/').collect();
        assert!(
            !rel.starts_with('/') && !parts.iter().any(|p| p.is_empty() || *p == ".."),
            "{rel:?} is not a path inside the root"
        );
        let target = parts
            .iter()
            .fold(root.to_path_buf(), |p, part| p.join(part));
        if rel.ends_with('/') {
            assert_eq!(text, "", "{rel:?} names a directory and carries text");
            fs::create_dir_all(&target).expect("a directory");
        } else {
            fs::create_dir_all(target.parent().expect("a parent")).expect("a parent directory");
            fs::write(&target, text.as_str().expect("a file's text").as_bytes()).expect("a file");
        }
    }
}

/// The server a case's `input.roots` names, over `root`.
pub fn server_over(input: &Value, root: &Path) -> GolfMcp {
    let roots = &input["roots"];
    let dir = |key: &str| roots[key].as_str().map(|rel| root.join(rel));
    GolfMcp::new(Dirs {
        sessions: dir("sessions").expect("roots.sessions"),
        shots: dir("shots").expect("roots.shots"),
        golfers: dir("golfers"),
    })
}

/// Every call of the case answered, in order, each as `{content, structuredContent, isError}`, or
/// the protocol error's message for a call that raised one.
pub fn run_case(input: &Value) -> Vec<Result<Value, String>> {
    let scratch = Scratch::new();
    materialise(&input["files"], scratch.path());
    let server = server_over(input, scratch.path());
    calls(input)
        .iter()
        .map(|call| {
            let clock = call.now.clone().map_or(Clock::Unset, Clock::At);
            server
                .call(&call.tool, &call.arguments, &clock)
                .map(|result| {
                    let wire = serde_json::to_value(&result).expect("a result serializes");
                    json!({
                        "content": wire.get("content").cloned().unwrap_or(Value::Null),
                        "structuredContent":
                            wire.get("structuredContent").cloned().unwrap_or(Value::Null),
                        "isError": wire.get("isError").cloned().unwrap_or(Value::Null),
                    })
                })
                .map_err(|error| format!("{}: {}", call.tool, error.message))
        })
        .collect()
}

// -------------------------------------------------------------------------------------------------
// The declared departures [M29 P11]
// -------------------------------------------------------------------------------------------------

/// One deliberate departure from frozen Python's answers.
pub struct Declared {
    /// The tool whose answers it reaches.
    pub tool: &'static str,
    /// Keys from one answered item (a view, or one element of a list a tool returns) down to the
    /// value the rule rewrites. A key missing or `null` on the way leaves that item as recorded: a
    /// miss has no `warnings`, and a swing with no shot has no `shot.warnings`.
    pub path: &'static [&'static str],
    /// What the recorded value becomes in Rust's answer, given that value and the whole recorded
    /// item it sits in, for a rule that turns on a sibling (P14's): `(recorded, item)`.
    pub rule: fn(&Value, &Value) -> Value,
    /// Why this is not a regression, for the gate's report.
    pub why: &'static str,
}

/// `screen::golfer_warnings` over a recorded list of warnings. Anything not a list of strings is
/// left as it is, for the gate to fail on.
fn golfer_warnings(recorded: &Value, _item: &Value) -> Value {
    let Some(lines) = recorded
        .as_array()
        .and_then(|items| items.iter().map(Value::as_str).collect::<Option<Vec<_>>>())
    else {
        return recorded.clone();
    };
    let lines: Vec<String> = lines.into_iter().map(str::to_string).collect();
    Value::from(screen::golfer_warnings(&lines))
}

const FILTERED: &str = "a shot's warnings go through M34's golfer-facing filter, so the \
                        `no tile found for` layout line is dropped and every other line kept in \
                        order (the program plan's §M29; `golf_mcp::query`'s module doc)";

/// `get_shot_trends`' "no sample" note, as Rust's corpus words it: the reasons frozen Python's
/// corpus had for a missing sample, and the drill after them (`crate::career::DRILL_IN_THE_NOTE`).
/// A note that does not carry Python's list is left as it is, for the gate to fail on.
fn drill_in_the_note(recorded: &Value, _item: &Value) -> Value {
    match recorded.as_str() {
        Some(note) if note.contains(career::FROZEN_NOTE_REASONS) => Value::String(note.replace(
            career::FROZEN_NOTE_REASONS,
            &format!("stale, {}", career::DRILL_IN_THE_NOTE),
        )),
        _ => recorded.clone(),
    }
}

/// `simulate_flight`'s `recorded_reading`, as Rust reads the artifact's generation
/// (`crate::flight::recorded_reading`): a stale reading becomes the edited one **when the artifact
/// recorded a flight number**, which a gap shows as a non-null `recorded`. [M29 P14]
///
/// The answer carries no version, so the rule reads the one thing in it that dates the artifact:
/// only an engine of 15 or later records a flight (14 -> 15 added the family), and frozen Python
/// recorded every tree at 16 or earlier, so such an artifact is inside `COMPARABLE_FROM` to this
/// engine and reads as current here. A stale reading with no recorded number is a swing with no
/// readable version or no flight at all, which Rust reads as stale too, and is left as recorded.
/// **Self-checking**: a recorded artifact this misdates would read differently in Rust's answer,
/// and the gate would fail on it.
fn comparable_reads_as_current(recorded: &Value, item: &Value) -> Value {
    let recorded_a_flight = item["differs_from_recorded"]
        .as_array()
        .is_some_and(|gaps| gaps.iter().any(|gap| gap["recorded"].is_number()));
    match recorded.as_str() {
        Some(reading) if reading == flight::STALE_READING && recorded_a_flight => {
            Value::String(flight::EDITED_READING.to_string())
        }
        _ => recorded.clone(),
    }
}

const COMPARABLE: &str = "an artifact from COMPARABLE_FROM to this engine reads as current, \
                          since none of its numbers disagrees with today's, so a gap there is an \
                          edited input and not the engine, where Python reads every engine but \
                          its own as stale (`golf_mcp::flight::recorded_reading`)";

/// Each element of a recorded `get_swing`'s `checkpoints`, with the `one_sided` key Rust's view adds
/// (`golf_mcp::query`'s module doc). [M29 P17]
///
/// The view reads it off the stored score, and the recorded answer does not carry the stored
/// file, so the rule reads the registry the score was judged under: every recorded artifact writes
/// the registry's sidedness (measured over every `tools/` and `real/` tree, 2026-10-22). **Self-
/// checking**: an artifact that wrote another would answer differently from this, and the gate
/// would fail on it. A name the registry does not hold is `null`, as a score that does not say is.
fn sidedness_from_the_registry(recorded: &Value, _item: &Value) -> Value {
    each_object(recorded, |view| {
        let name = view.get("name").and_then(Value::as_str);
        let sided = CHECKPOINT_REGISTRY
            .iter()
            .find(|spec| Some(spec.name) == name)
            .map_or(Value::Null, |spec| Value::Bool(spec.one_sided));
        view.insert("one_sided".to_string(), sided);
    })
}

/// Each element of a recorded `get_swing`'s `unscored`, with the `family` key Rust's view adds,
/// read off its `reason` as the view reads it: a token this build does not know is `unrecorded`,
/// so a checkpoint. [M29 P17]
fn family_from_the_reason(recorded: &Value, _item: &Value) -> Value {
    each_object(recorded, |view| {
        let reason = view
            .get("reason")
            .cloned()
            .and_then(|token| serde_json::from_value(token).ok())
            .unwrap_or(UnscoredReason::Unrecorded);
        view.insert("family".to_string(), Value::from(query::family_of(reason)));
    })
}

/// A recorded list with `add` applied to each object in it. Anything that is not a list of objects
/// is left as it is, for the gate to fail on.
fn each_object(recorded: &Value, add: impl Fn(&mut serde_json::Map<String, Value>)) -> Value {
    let Some(items) = recorded.as_array() else {
        return recorded.clone();
    };
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        let Some(view) = item.as_object() else {
            return recorded.clone();
        };
        let mut view = view.clone();
        add(&mut view);
        out.push(Value::Object(view));
    }
    Value::Array(out)
}

const SIDEDNESS: &str = "a checkpoint's view carries the stored score's `one_sided`, which the \
                         coaching brief prints beside every percentile and the briefing tells \
                         every client to read, where Python's view has no such key \
                         (`golf_mcp::query`'s module doc)";

const FAMILY: &str = "an unscored entry's view says whether it is a checkpoint or a flight number, \
                      which was never in `overall_score`, so the coaching brief can tell them apart \
                      without a copy of the reason table, where Python's view has no such key \
                      (`golf_mcp::query`'s module doc)";

/// `get_bag_profile`'s note when no swing names a club, ending on the verb that replaced the closed
/// retag route rather than on the route (`crate::club::NOTHING_TAGGED`). [M29 P32] A note that does
/// not end on frozen Python's tail is left as it is, for the gate to fail on.
fn retag_through_golf_core(recorded: &Value, _item: &Value) -> Value {
    match recorded.as_str() {
        Some(note) if note.ends_with(club::NOTHING_TAGGED_FROZEN_TAIL) => {
            Value::String(note.replace(club::NOTHING_TAGGED_FROZEN_TAIL, club::NOTHING_TAGGED_TAIL))
        }
        _ => recorded.clone(),
    }
}

const RETAG: &str = "the note for a bag no swing tags names `golf-core set-club`, since the M29 \
                     plan's D11 closed the frozen server's retag route it named and that route \
                     now refuses naming the verb (`golf_mcp::club::NOTHING_TAGGED`)";

/// `simulate_flight`'s `detail` for a screen not read yet, ending on the verb that replaced the
/// deleted importer rather than on the importer (`crate::flight::UNREAD_SHOT_SCREEN`). [M29 P34]
/// Any other detail, or one that does not end on frozen Python's tail, is left as it is: the rule
/// moves only the sentence it names, and the gate fails anything else.
fn import_through_golf_lab(recorded: &Value, _item: &Value) -> Value {
    match recorded.as_str() {
        Some(detail) if detail.ends_with(flight::UNREAD_SHOT_SCREEN_FROZEN_TAIL) => {
            Value::String(detail.replace(
                flight::UNREAD_SHOT_SCREEN_FROZEN_TAIL,
                flight::UNREAD_SHOT_SCREEN_TAIL,
            ))
        }
        _ => recorded.clone(),
    }
}

const IMPORT: &str = "the detail for a shot screen not read yet names `golf-lab import-photos`, \
                      since M29 P34 deleted the `scripts/import_shot_screens.py` it named \
                      (`golf_mcp::flight::UNREAD_SHOT_SCREEN`)";

const DRILLS: &str =
    "Rust's corpus leaves a drill out of every count (ADR-024's M35 addendum), so \
                      the note naming every reason a metric has no sample names that one too, \
                      where Python's would deny it (`golf_mcp::career`'s module doc)";

/// Every declared departure, by tool and path. P11's three are one rule reaching the three places
/// a shot's view is answered: the two shot tools, and `get_swing`'s embedded `shot`. The fourth
/// place, `get_session_summary`, answers no warnings, only counts and averages, so the filter does
/// not reach it. P12's one is the trend's note, and P14's the flight's reading of a stored one.
/// P17's two are keys `get_swing`'s view gained for the coaching brief, and they reach no other
/// tool, because no other answer embeds a swing's view. P32's is the bag note naming the verb that
/// replaced a closed route; `get_club_profile` answers no such note, so it reaches one tool. P34's
/// is the flight's detail naming the verb that replaced a deleted script; `get_swing` embeds no
/// flight, so it reaches one tool too.
pub static DECLARED: [Declared; 9] = [
    Declared {
        tool: "get_recent_shots",
        path: &["warnings"],
        rule: golfer_warnings,
        why: FILTERED,
    },
    Declared {
        tool: "get_shot_by_id",
        path: &["warnings"],
        rule: golfer_warnings,
        why: FILTERED,
    },
    Declared {
        tool: "get_swing",
        path: &["shot", "warnings"],
        rule: golfer_warnings,
        why: FILTERED,
    },
    Declared {
        tool: "get_shot_trends",
        path: &["note"],
        rule: drill_in_the_note,
        why: DRILLS,
    },
    Declared {
        tool: "simulate_flight",
        path: &["recorded_reading"],
        rule: comparable_reads_as_current,
        why: COMPARABLE,
    },
    Declared {
        tool: "get_swing",
        path: &["checkpoints"],
        rule: sidedness_from_the_registry,
        why: SIDEDNESS,
    },
    Declared {
        tool: "get_swing",
        path: &["unscored"],
        rule: family_from_the_reason,
        why: FAMILY,
    },
    Declared {
        tool: "get_bag_profile",
        path: &["note"],
        rule: retag_through_golf_core,
        why: RETAG,
    },
    Declared {
        tool: "simulate_flight",
        path: &["detail"],
        rule: import_through_golf_lab,
        why: IMPORT,
    },
];

/// The oracles whose committed answers are already `golf-mcp`'s (the module doc), so no
/// declaration is applied to them. `python` is the third, and every declaration reaches it.
pub const OURS_ALREADY: [&str; 2] = ["hand", "rust"];

/// One committed answer as `golf-mcp` answers it on purpose, and the indices into [`DECLARED`] of
/// the declarations that moved it there.
pub type AsOurs = (Value, Vec<usize>);

/// A case's committed answers as `golf-mcp` answers them on purpose, one per call, each with the
/// indices into [`DECLARED`] of the declarations that moved it: a Python-recorded case through
/// [`as_declared`], a hand or Rust case as committed. `Ok(None)` for a case with no
/// `expected.results` (one being recorded), and `Err` for an oracle the family does not have or
/// results that are not one per call.
pub fn recorded_as_ours(case: &Value) -> Result<Option<Vec<AsOurs>>, String> {
    let Some(results) = case["expected"]["results"].as_array() else {
        return Ok(None);
    };
    let oracle = case["provenance"]["oracle"].as_str().unwrap_or_default();
    let calls = calls(&case["input"]);
    if calls.len() != results.len() {
        return Err(format!(
            "{} calls and {} recorded results: a case answers each call once",
            calls.len(),
            results.len()
        ));
    }
    let declared = match oracle {
        "python" => true,
        other if OURS_ALREADY.contains(&other) => false,
        other => {
            return Err(format!(
                "provenance.oracle is {other:?}, and an mcp case is recorded by \"python\", or is \
                 already golf-mcp's: {OURS_ALREADY:?}"
            ))
        }
    };
    Ok(Some(
        calls
            .iter()
            .zip(results)
            .map(|(call, want)| {
                if declared {
                    as_declared(&call.tool, want)
                } else {
                    (want.clone(), Vec::new())
                }
            })
            .collect(),
    ))
}

/// The family's runner as one value, what `golf-mcp answer-case` prints: `answers`, [`run_case`]'s
/// answer to every call in order (a protocol error as `{"protocol_error": message}`, which nothing
/// recorded), and `recorded`, [`recorded_as_ours`]' committed answers without their indices, or
/// `null` for a case with no `expected`. `Err` for a case that is not one: no `input`, or what
/// [`recorded_as_ours`] refuses.
pub fn answer_case(case: &Value) -> Result<Value, String> {
    if !case["input"].is_object() {
        return Err("no `input` object: a case is a tree, its roots and its calls".to_string());
    }
    let recorded = recorded_as_ours(case)?
        .map(|answers| answers.into_iter().map(|(answer, _)| answer).collect());
    let answers: Value = run_case(&case["input"])
        .into_iter()
        .map(|answer| answer.unwrap_or_else(|error| json!({ "protocol_error": error })))
        .collect();
    Ok(json!({
        "answers": answers,
        "recorded": recorded.unwrap_or(Value::Null),
    }))
}

/// Apply one declaration to one answered item, in place. `true` when it changed the item.
fn rewrite(item: &mut Value, declared: &Declared) -> bool {
    let (last, parents) = declared
        .path
        .split_last()
        .expect("a declared path is not empty");
    let recorded_item = item.clone();
    let mut at = item;
    for key in parents {
        match at.get_mut(*key) {
            Some(next) if !next.is_null() => at = next,
            _ => return false,
        }
    }
    let Some(slot) = at.get_mut(*last) else {
        return false;
    };
    let rewritten = (declared.rule)(slot, &recorded_item);
    let changed = rewritten != *slot;
    *slot = rewritten;
    changed
}

/// The recorded answer `want` to one call of `tool`, as `golf-mcp` answers it on purpose: every
/// [`DECLARED`] rule for that tool applied at its path, in `structuredContent.result` (each element,
/// when the tool returns a list) and in each `content` block's parsed text. Answers the rewritten
/// answer and the indices into [`DECLARED`] of the declarations that changed it.
///
/// A rewritten block's text is the rewritten item serialized again; the gate compares blocks as
/// parsed values, so its spelling is not what is held.
pub fn as_declared(tool: &str, want: &Value) -> (Value, Vec<usize>) {
    let mut answer = want.clone();
    let mut moved = Vec::new();
    for (index, declared) in DECLARED.iter().enumerate() {
        if declared.tool != tool {
            continue;
        }
        let mut changed = false;
        if let Some(result) = answer
            .get_mut("structuredContent")
            .and_then(|content| content.get_mut("result"))
        {
            match result {
                Value::Array(items) => {
                    for item in items {
                        changed |= rewrite(item, declared);
                    }
                }
                item => changed |= rewrite(item, declared),
            }
        }
        if let Some(blocks) = answer.get_mut("content").and_then(Value::as_array_mut) {
            for block in blocks {
                let Some(mut item) = block["text"]
                    .as_str()
                    .and_then(|text| serde_json::from_str::<Value>(text).ok())
                else {
                    continue;
                };
                if rewrite(&mut item, declared) {
                    changed = true;
                    block["text"] = Value::String(
                        serde_json::to_string_pretty(&item).expect("a parsed block serializes"),
                    );
                }
            }
        }
        if changed {
            moved.push(index);
        }
    }
    (answer, moved)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A declaration rewrites what its path names, in both halves of an answer, and leaves a miss
    /// (no `warnings`) and a swing with no shot as recorded.
    #[test]
    fn a_declaration_rewrites_its_path_and_nothing_else() {
        let view = json!({"shot_id": "s", "warnings": [
            "no tile found for 'Bounce & Roll'", "Spin: no value text under the label"
        ]});
        let answer = |result: Value, blocks: Vec<&Value>| {
            json!({
                "content": blocks.iter().map(|b| json!({
                    "type": "text", "text": serde_json::to_string_pretty(b).unwrap()
                })).collect::<Vec<_>>(),
                "structuredContent": {"result": result},
                "isError": false,
            })
        };
        let recorded = answer(json!([view.clone(), view.clone()]), vec![&view, &view]);
        let (ours, moved) = as_declared("get_recent_shots", &recorded);
        assert_eq!(moved, [0]);
        let kept = json!(["Spin: no value text under the label"]);
        assert_eq!(ours["structuredContent"]["result"][1]["warnings"], kept);
        let block: Value =
            serde_json::from_str(ours["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(block["warnings"], kept);
        assert_eq!(block["shot_id"], "s");

        let miss = json!({"found": false, "message": "No shot with id 'x'."});
        let recorded = answer(miss.clone(), vec![&miss]);
        assert_eq!(as_declared("get_shot_by_id", &recorded), (recorded, vec![]));
        let bare = json!({"swing_id": "1", "shot": null});
        let recorded = answer(bare.clone(), vec![&bare]);
        assert_eq!(as_declared("get_swing", &recorded), (recorded, vec![]));
        // A tool nothing is declared for is answered as recorded, even where the path exists.
        let recorded = answer(view.clone(), vec![&view]);
        assert_eq!(as_declared("list_sessions", &recorded), (recorded, vec![]));
    }
}
