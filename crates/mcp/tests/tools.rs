//! The tool cases' gate: every call `spec/vectors/mcp/` holds, answered by `golf-mcp` over the same
//! tree, and held to the committed answer. [M29 P9]
//!
//! **Every call is compared** (M29 P15). P9 registered all twelve tools and P10–P15 landed their
//! bodies; until then a tool with no body was skipped by name, never by a blanket ignore, and its
//! calls had to answer that it was not ported. P15 landed the last, and a row's body is no longer
//! optional (`golf_mcp::tools`), so there is nothing left to skip.
//!
//! # What a comparison is (the plan's P8 finding 4)
//!
//! An answer is `{content, structuredContent, isError}`. `isError` is compared exactly and
//! `structuredContent` as a value under `golf_core::compare`'s rules (§3's tolerance): the
//! committed one has its keys sorted by the writer, so text order means nothing. Each `content`
//! block is pydantic's `indent=2` JSON of one item, so its text is parsed and compared as a value
//! too, beside its `type`.
//!
//! # Three oracles [M29 P11, P15]
//!
//! A Python-recorded answer (`tools/`, `real/`) is first rewritten by `golf_mcp::family::DECLARED`,
//! the departures `golf-mcp` makes on purpose, each a rule at a path; the comparison is then the
//! same as everywhere else, so a declared path is held to what the rule makes of the recorded
//! value, not skipped. Each declaration must move at least one recorded answer, or it is stale and
//! this fails. The count each one moved is printed. `hand/` holds what no recorded answer reaches
//! (`oracle: "hand"`), and `rust/` the strike profile's answers over recorded trees, written once by
//! `golf-mcp answer-case` (`oracle: "rust"`, P15): both are already Rust's, so no declaration is
//! applied to them. `golf_mcp::family::recorded_as_ours` is that rule, and the career re-record
//! reads the same function through the binary.
//!
//! # What the strike cases are held to besides their answers [M29 P15]
//!
//! A `rust/` case's tree is copied whole from the recorded case its `provenance.source` names, and
//! is held equal to it here, so the copy cannot drift from what frozen Python answered over. And
//! every strike profile any case answers is held to `golf-core profile --json` over the same tree,
//! the verb's element for the same name, so the tool and the verb read one corpus and one bag.

use std::collections::BTreeMap;
use std::io::Read;
use std::path::PathBuf;

use golf_core::compare::compare;
use golf_core::reports::{answer as report, DataDirs, Request, Verb};
use golf_mcp::career::versions;
use golf_mcp::family::{
    calls, materialise, recorded_as_ours, run_case, Scratch, DECLARED, OURS_ALREADY,
};
use golf_mcp::tools;
use serde_json::Value;

fn family() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../spec/vectors/mcp")
}

/// The cases in one sub-family's directory, plain and gzipped, sorted.
fn listed(dir: &str) -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(family().join(dir))
        .unwrap_or_else(|e| panic!("spec/vectors/mcp/{dir}/: {e}"))
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            let name = path.to_string_lossy();
            name.ends_with(".json") || name.ends_with(".json.gz")
        })
        .collect();
    paths.sort();
    paths
}

fn read_case(path: &std::path::Path) -> Value {
    let bytes = std::fs::read(path).expect("a case");
    let text = if path.extension().is_some_and(|e| e == "gz") {
        let mut text = String::new();
        flate2::read::GzDecoder::new(bytes.as_slice())
            .read_to_string(&mut text)
            .expect("gzipped JSON");
        text
    } else {
        String::from_utf8(bytes).expect("UTF-8")
    };
    serde_json::from_str(&text).expect("JSON")
}

/// Every tool case: the Python-recorded `tools/` and `real/`, then the hand-worked `hand/` (P11
/// on) and the Rust-recorded `rust/` (P15). The surface case is `tests/surface.rs`'.
fn cases() -> Vec<(String, Value)> {
    ["tools", "real", "hand", "rust"]
        .into_iter()
        .flat_map(listed)
        .map(|path| {
            let case = read_case(&path);
            (case["id"].as_str().unwrap().to_string(), case)
        })
        .collect()
}

/// Where `ours` departs from `want`, as the module doc compares them.
fn departures(want: &Value, ours: &Value) -> Vec<String> {
    let mut out = Vec::new();
    if want["isError"] != ours["isError"] {
        out.push(format!(
            "isError {} != {}",
            ours["isError"], want["isError"]
        ));
    }
    for d in compare(&want["structuredContent"], &ours["structuredContent"]) {
        out.push(format!("structuredContent: {d:?}"));
    }
    let (want_blocks, our_blocks) = (
        want["content"].as_array().cloned().unwrap_or_default(),
        ours["content"].as_array().cloned().unwrap_or_default(),
    );
    if want_blocks.len() != our_blocks.len() {
        out.push(format!(
            "content: {} blocks, recorded {}",
            our_blocks.len(),
            want_blocks.len()
        ));
    }
    for (i, (w, o)) in want_blocks.iter().zip(&our_blocks).enumerate() {
        if w["type"] != o["type"] {
            out.push(format!("content[{i}].type {} != {}", o["type"], w["type"]));
        }
        let parse = |block: &Value| -> Value {
            serde_json::from_str(block["text"].as_str().unwrap_or("null")).unwrap_or(Value::Null)
        };
        for d in compare(&parse(w), &parse(o)) {
            out.push(format!("content[{i}]: {d:?}"));
        }
    }
    out
}

#[test]
fn every_committed_call_is_answered_as_committed() {
    let mut by_oracle: BTreeMap<String, usize> = BTreeMap::new();
    let mut by_tool: BTreeMap<String, usize> = BTreeMap::new();
    let mut failures = Vec::new();
    let mut moved = vec![0usize; DECLARED.len()];
    for (id, case) in cases() {
        assert_eq!(case["provenance"]["kind"], "mcp", "{id}");
        let oracle = case["provenance"]["oracle"].as_str().unwrap_or_default();
        assert!(
            oracle == "python" || OURS_ALREADY.contains(&oracle),
            "{id}: oracle {oracle:?}"
        );
        let wanted = recorded_as_ours(&case)
            .unwrap_or_else(|e| panic!("{id}: {e}"))
            .unwrap_or_else(|| panic!("{id}: no expected.results"));
        let answers = run_case(&case["input"]);
        let calls = calls(&case["input"]);
        assert_eq!(answers.len(), wanted.len(), "{id}: one answer per call");
        for (i, ((call, answer), (want, which))) in
            calls.iter().zip(&answers).zip(&wanted).enumerate()
        {
            assert!(tools::spec(&call.tool).is_some(), "{id}[{i}]: no such tool");
            let ours = match answer {
                Ok(ours) => ours,
                Err(error) => {
                    panic!("{id}[{i}]: a protocol error, which nothing recorded: {error}")
                }
            };
            for index in which {
                moved[*index] += 1;
            }
            *by_oracle.entry(oracle.to_string()).or_default() += 1;
            *by_tool.entry(call.tool.clone()).or_default() += 1;
            let found = departures(want, ours);
            if !found.is_empty() {
                failures.push(format!("{id}[{i}] {}: {}", call.tool, found.join("; ")));
            }
        }
    }
    for (declared, n) in DECLARED.iter().zip(&moved) {
        eprintln!(
            "declared at {} {}: {n} recorded answers moved",
            declared.tool,
            declared.path.join(".")
        );
    }
    eprintln!("compared by oracle: {by_oracle:?}; by tool: {by_tool:?}");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    for (declared, n) in DECLARED.iter().zip(&moved) {
        assert!(
            *n > 0,
            "{} at {} moved no recorded answer, so its declaration is stale: {}",
            declared.tool,
            declared.path.join("."),
            declared.why
        );
    }
    // Every tool the table lists is called by some case, so none is listed and held to nothing.
    for spec in &tools::TOOLS {
        assert!(
            by_tool.contains_key(spec.name),
            "no committed case calls {}",
            spec.name
        );
    }
    for oracle in ["python", "hand", "rust"] {
        assert!(by_oracle.contains_key(oracle), "no {oracle} call was read");
    }
}

/// The recorded case a `rust/` case copied its tree from: `provenance.source` begins with its path
/// under the repository, as a profile corpus case names its career case.
fn source_of(id: &str, case: &Value) -> PathBuf {
    let source = case["provenance"]["source"].as_str().unwrap_or_default();
    let path = source
        .split([',', ' '])
        .next()
        .and_then(|first| first.strip_prefix("spec/vectors/mcp/"))
        .unwrap_or_else(|| panic!("{id}: provenance.source names no mcp case first: {source:?}"));
    family().join(path)
}

/// **A `rust/` case's tree is its source's**: the roots and the files, equal as values to the
/// Python-recorded case it names, so the strike profile is answered over exactly what frozen
/// Python answered every other tool over, and a copy cannot drift. [M29 P15]
#[test]
fn every_rust_case_reads_the_tree_of_the_recorded_case_it_names() {
    let rust = listed("rust");
    assert!(!rust.is_empty(), "spec/vectors/mcp/rust/ is empty");
    for path in rust {
        let case = read_case(&path);
        let id = case["id"].as_str().unwrap().to_string();
        assert_eq!(case["provenance"]["oracle"], "rust", "{id}");
        let source = read_case(&source_of(&id, &case));
        assert_eq!(source["provenance"]["oracle"], "python", "{id}'s source");
        for key in ["roots", "files"] {
            assert_eq!(
                case["input"][key], source["input"][key],
                "{id}: input.{key} is not its source's"
            );
        }
        for call in calls(&case["input"]) {
            assert_eq!(
                call.tool, "get_strike_profile",
                "{id}: a rust case pins one tool"
            );
        }
    }
}

/// **The tool and the verb read one corpus and one bag**: every strike profile a case answers is
/// `golf-core profile --json`'s element for the same name over the same tree, value for value. A
/// miss has no element to compare, and the verb prints a `(not registered)` golfer instead.
/// [M29 P15]
#[test]
fn every_strike_profile_answered_is_the_profile_verbs_json() {
    let mut compared = 0;
    for (id, case) in cases() {
        let strike: Vec<_> = calls(&case["input"])
            .into_iter()
            .enumerate()
            .filter(|(_, call)| call.tool == "get_strike_profile")
            .collect();
        if strike.is_empty() {
            continue;
        }
        let answers = run_case(&case["input"]);
        let scratch = Scratch::new();
        materialise(&case["input"]["files"], scratch.path());
        let roots = &case["input"]["roots"];
        let dir = |key: &str| scratch.path().join(roots[key].as_str().expect("a root"));
        for (i, call) in strike {
            let answer = answers[i].as_ref().expect("an answer");
            let result = &answer["structuredContent"]["result"];
            if result["found"] == false {
                continue;
            }
            let printed = report(
                Verb::Profile,
                &Request {
                    name: Some(call.arguments["player"].as_str().unwrap().to_string()),
                    player_id: None,
                    verbose: false,
                    club: None,
                    json: true,
                    dirs: DataDirs {
                        sessions: dir("sessions"),
                        golfers: dir("golfers"),
                    },
                },
                versions(),
            );
            assert_eq!(printed.code, 0, "{id}[{i}]: {}", printed.stderr);
            let verb: Value = serde_json::from_str(&printed.stdout).expect("the verb's JSON");
            assert_eq!(verb.as_array().map(Vec::len), Some(1), "{id}[{i}]");
            assert_eq!(
                &verb[0], result,
                "{id}[{i}]: the tool and the verb disagree"
            );
            compared += 1;
        }
    }
    assert!(compared > 0, "no strike profile was answered");
}

/// **The real golfer's strike profile is the profile family's**: `rust/real-data`, answered over
/// `real/data`'s snapshot of `data/`, equals `profile/corpus/real-aaron`'s profile, recorded from the
/// storage family's slimmer snapshot of the same captures (M37 P11). The two snapshots keep
/// different parts of each analysis, and nothing a grade reads differs between them; a re-record
/// under a career declaration moves both in one walk, so the two cannot part without this failing.
/// [M29 P15]
#[test]
fn the_real_strike_profile_is_the_profile_familys_real_aaron() {
    let ours = read_case(&family().join("rust/real-data.json.gz"));
    let theirs = read_case(&family().join("../profile/corpus/real-aaron.json"));
    let answered = &ours["expected"]["results"][0]["structuredContent"]["result"];
    assert_eq!(answered["display_name"], theirs["input"]["display_name"]);
    assert_eq!(answered["profile"], theirs["expected"]["profile"]);
}
