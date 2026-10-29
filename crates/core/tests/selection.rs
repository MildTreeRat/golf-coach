//! The selection family (`spec/vectors/selection/`) against
//! `golf_core::selection_family::run_selection`: every call answered and compared, **exactly**.
//! [M29 P19]
//!
//! Frozen Python recorded the family once (M29 P18, the plan's decision 14): `analysis/phases.py`'s
//! clip-choosing rules and `api/pipeline.py`'s `_pick_swing` and `_auto_windows`, over the tests'
//! clips (`rules/`, `windows/`) and the 15 stored bundles (`corpus/`). It is the only committed gate
//! on which descent the engine is handed: the corpus vectors take their windows from the stored
//! `analysis.json`, so they say what was scored and not how it was chosen.
//!
//! **By value, not under §3's tolerance alone.** Every answer is a few frame indices, a `rise`
//! that is one subtraction of two smoothed wrist heights, and a sentence; the rules are a straight
//! port in the Python's order of operations, so nothing here should land on another last bit, and
//! [`every_selection_call_is_run_selections_answer_exactly`] holds them to that (P18's finding 8).
//! A difference inside `RTOL` would be a real finding rather than noise, because a `rise` decides
//! whether a descent is a candidate at all.
//!
//! **Every call is answered since M29 P25**, `_auto_windows` included: the pipeline's ordering
//! around its picks is `golf_core::auto_windows`, which `crates/lab`'s bundle assembly calls, so
//! `selection_family::NOT_YET` is empty and the gate asserts that nothing is skipped. Its picks are
//! gated on their own as well, because the recorder lists each one as a `_pick_swing` call
//! immediately ahead of it, which [`every_auto_windows_pick_is_the_pick_call_ahead_of_it`] holds,
//! and [`the_family_reaches_every_sentence_auto_windows_writes`] holds that the cases reach every
//! note and decline it can write.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Read as _;
use std::path::{Path, PathBuf};

use contracts::swing::ANALYSIS_VERSION;
use flate2::read::GzDecoder;
use golf_core::compare::compare;
use golf_core::selection_family::{not_yet, run_selection, KIND, NOT_YET};
use serde_json::{json, Value};

/// The sub-families, as the recorder writes them and `golf-core rerecord` walks them.
const SUB_FAMILIES: [&str; 3] = ["rules", "windows", "corpus"];

fn vectors_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../spec/vectors")
        .canonicalize()
        .expect("spec/vectors/ is committed")
}

/// Every case in the family, sorted by id. Every case is gzipped (P18), and every entry of the
/// family's directory is one of [`SUB_FAMILIES`], so a sub-family added later is placed on purpose.
fn selection_cases() -> Vec<(String, Value)> {
    let root = vectors_dir().join(KIND);
    let mut cases = Vec::new();
    for entry in fs::read_dir(&root).unwrap_or_else(|e| panic!("{}: {e}", root.display())) {
        let dir = entry.expect("an entry").path();
        let name = dir.file_name().and_then(|n| n.to_str()).unwrap_or_default();
        assert!(
            dir.is_dir() && SUB_FAMILIES.contains(&name),
            "{} is not a selection sub-family",
            dir.display()
        );
        for file in fs::read_dir(&dir).expect("list") {
            let path = file.expect("an entry").path();
            assert!(
                path.to_string_lossy().ends_with(".json.gz"),
                "{}: every selection case is gzipped",
                path.display()
            );
            let mut text = String::new();
            GzDecoder::new(&fs::read(&path).expect("read")[..])
                .read_to_string(&mut text)
                .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            let case: Value =
                serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            let id = case["id"]
                .as_str()
                .expect("a case carries its id")
                .to_string();
            assert!(
                id.starts_with(&format!("{KIND}/{name}/")),
                "{id} is filed under {name}/"
            );
            cases.push((id, case));
        }
    }
    cases.sort_by(|a, b| a.0.cmp(&b.0));
    assert!(!cases.is_empty(), "no case under spec/vectors/{KIND}");
    cases
}

/// One case's calls and its recorded answers, side by side.
fn calls_and_results(case: &Value) -> (&[Value], &[Value]) {
    let calls = case["input"]["calls"].as_array().expect("calls");
    let results = case["expected"]["results"].as_array().expect("results");
    assert_eq!(
        calls.len(),
        results.len(),
        "{}: a result per call",
        case["id"]
    );
    (calls, results)
}

/// **The gate.** Every call the runner answers is the recorded answer, by value — and under §3's
/// comparison too, so a failure names its path — and every call it does not answer is one
/// [`NOT_YET`] names, counted.
#[test]
fn every_selection_call_is_run_selections_answer_exactly() {
    let mut failures = Vec::new();
    let mut answered: BTreeMap<String, usize> = BTreeMap::new();
    let mut skipped: BTreeMap<String, usize> = BTreeMap::new();
    let cases = selection_cases();
    for (id, case) in &cases {
        let (calls, results) = calls_and_results(case);
        let answers = run_selection(&case["input"])
            .unwrap_or_else(|e| panic!("{id}: the runner refused: {e}"));
        assert_eq!(answers.len(), calls.len(), "{id}: an answer per call");
        for (index, ((call, recorded), answer)) in
            calls.iter().zip(results).zip(&answers).enumerate()
        {
            let name = call["fn"]
                .as_str()
                .expect("a call names its fn")
                .to_string();
            let Some(answer) = answer else {
                assert!(
                    not_yet(&name).is_some(),
                    "{id}: call {index} ({name}) was not answered, and NOT_YET does not name it"
                );
                *skipped.entry(name).or_default() += 1;
                continue;
            };
            *answered.entry(name.clone()).or_default() += 1;
            let differences = compare(recorded, answer);
            if !differences.is_empty() || answer != recorded {
                failures.push(format!(
                    "{id}: call {index} ({name}): {} under §3, {}",
                    differences.len(),
                    if differences.is_empty() {
                        "and differs below RTOL".to_string()
                    } else {
                        format!("{differences:?}")
                    }
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} calls differ:\n{}",
        failures.len(),
        failures.join("\n")
    );
    let names: BTreeSet<&str> = skipped.keys().map(String::as_str).collect();
    let not_yet_names: BTreeSet<&str> = NOT_YET.iter().map(|&(name, _)| name).collect();
    assert_eq!(
        names, not_yet_names,
        "every call NOT_YET names is still in the family, so the skip is not vacuous"
    );
    println!(
        "{} cases; answered exactly {answered:?}; skipped by name {skipped:?} until {:?}",
        cases.len(),
        NOT_YET
    );
}

/// Every case is a frozen-Python record at this build's engine version: recorded at v16 by P18 and
/// moved to v17 by `golf-core rerecord` under `v17.json`, with the ledger saying so (the user's
/// decision at the P18/P19 boundary). A case at another version is one the re-record has not
/// walked.
#[test]
fn every_selection_case_is_a_python_record_at_this_engine_version() {
    for (id, case) in selection_cases() {
        assert_eq!(case["provenance"]["kind"], json!(KIND), "{id}");
        assert_eq!(case["provenance"]["oracle"], json!("python"), "{id}");
        assert_eq!(
            case["analysis_version"],
            json!(ANALYSIS_VERSION),
            "{id}: not at this build's engine version"
        );
        let ledger = case["provenance"]["rerecords"]
            .as_array()
            .unwrap_or_else(|| panic!("{id}: a version above the recorder's needs a ledger"));
        assert!(
            ledger
                .iter()
                .any(|entry| entry["analysis_version"] == json!(ANALYSIS_VERSION)
                    && entry["moved"]
                        .as_array()
                        .is_some_and(|moved| moved.contains(&json!("analysis_version")))),
            "{id}: no ledger entry moved its version to {ANALYSIS_VERSION}"
        );
    }
}

/// The encoding's promise (P18's finding 4): each `_auto_windows` answer's `picks` are exactly the
/// `_pick_swing` calls immediately ahead of it, in order. That is what makes the picks gated here
/// on their own, and what P25 leans on to hold the lab's ordering around them.
#[test]
fn every_auto_windows_pick_is_the_pick_call_ahead_of_it() {
    let mut seen = 0;
    for (id, case) in selection_cases() {
        let (calls, results) = calls_and_results(&case);
        for (index, call) in calls.iter().enumerate() {
            if call["fn"] != "_auto_windows" {
                continue;
            }
            seen += 1;
            let picks = results[index]["picks"].as_array().expect("picks");
            let ahead = &calls[index - picks.len()..index];
            for (pick, call) in picks.iter().zip(ahead) {
                let mut arguments = call.clone();
                arguments.as_object_mut().expect("a call").remove("fn");
                assert_eq!(call["fn"], "_pick_swing", "{id}: call {index}");
                assert_eq!(pick, &arguments, "{id}: call {index}'s pick");
            }
        }
    }
    assert!(seen > 0, "no _auto_windows call to hold");
}

/// **What the family reaches, as a checked fact**, so a case edited away from its purpose fails by
/// name. Each branch of both rules is a reason's shape (or a decline), and P18's finding 5 says the
/// stored bundles reach only some of them: the decline, the lone-candidate escape and the matching
/// rule's "outside the band" reach the gate through `rules/` and `windows/` alone.
#[test]
fn the_family_reaches_every_branch_of_both_rules() {
    let mut reached: BTreeMap<&str, usize> = BTreeMap::new();
    for (_, case) in selection_cases() {
        let (calls, results) = calls_and_results(&case);
        for (call, result) in calls.iter().zip(results) {
            let name = call["fn"].as_str().expect("a fn");
            if name == "window_around" {
                let branch = if call["fps"].is_null() {
                    "window: no fps"
                } else {
                    "window: fps"
                };
                *reached.entry(branch).or_default() += 1;
                continue;
            }
            if !matches!(
                name,
                "select_swing" | "select_matching_swing" | "_pick_swing"
            ) {
                continue;
            }
            let Some(reason) = result["reason"].as_str() else {
                *reached.entry("a decline").or_default() += 1;
                continue;
            };
            let mut branch = |shape: &'static str| *reached.entry(shape).or_default() += 1;
            if reason.contains("ending at a ball strike") {
                branch("a strike narrowed the field");
            }
            if reason.starts_with("1 descent ends at a ball strike, and its") {
                branch("lone, struck");
            } else if reason.starts_with("1 descent, and its") {
                branch("lone, nothing to choose between");
            } else if reason.contains(", 1 with a plausible downswing (") {
                branch("alone: one plausible");
            } else if reason.contains("took the last") {
                branch("alone: took the last");
            } else if reason.contains("in band, off by") {
                branch("matched: in band");
            } else if reason.contains("outside the usual 0.12-0.8s band, off by") {
                branch("matched: vouched for outside the band");
            } else {
                panic!("a reason of no known shape: {reason}");
            }
        }
    }
    for branch in [
        "window: no fps",
        "window: fps",
        "a decline",
        "a strike narrowed the field",
        "lone, struck",
        "lone, nothing to choose between",
        "alone: one plausible",
        "alone: took the last",
        "matched: in band",
        "matched: vouched for outside the band",
    ] {
        assert!(
            reached.get(branch).is_some_and(|&n| n > 0),
            "no call reaches {branch:?}: {reached:?}"
        );
    }
    println!("{reached:?}");
}

/// **What the `_auto_windows` answers reach**, as [`the_family_reaches_every_branch_of_both_rules`]
/// holds the rules' (M29 P25): each sentence `golf_core::auto_windows` can append is a branch of
/// the ordering, and P18's finding 5 says no stored bundle reaches a decline or the reverse match,
/// so `windows/` is the only gate on them and a case edited away from one fails here by name.
#[test]
fn the_family_reaches_every_sentence_auto_windows_writes() {
    let mut reached: BTreeMap<&str, usize> = BTreeMap::new();
    for (_, case) in selection_cases() {
        let (calls, results) = calls_and_results(&case);
        for (call, result) in calls.iter().zip(results) {
            if call["fn"] != "_auto_windows" {
                continue;
            }
            for note in result["notes"].as_array().expect("notes") {
                let note = note.as_str().expect("a note is a sentence");
                let branch = if note.starts_with("could not pick a swing in the face-on view") {
                    "face-on declined"
                } else if note.starts_with("could not pick a swing in the down-the-line view") {
                    "down the line declined"
                } else if note.starts_with("the face-on swing could not be picked from that clip") {
                    "face-on matched in reverse"
                } else if note.contains("(the face-on window was given rather than measured)") {
                    "down the line alone: face-on given"
                } else if note.contains("(the face-on view has no measured downswing") {
                    "down the line alone: nothing measured"
                } else {
                    panic!("a note of no known shape: {note}")
                };
                *reached.entry(branch).or_default() += 1;
                for why in [
                    "(the keypoints file records no fps,",
                    "(no plausible downswing,",
                ] {
                    if note.contains(why) {
                        *reached.entry(why).or_default() += 1;
                    }
                }
            }
            if call["window_face_on"].is_array() {
                *reached.entry("face-on window given").or_default() += 1;
            }
            if call["window_dtl"].is_array() {
                *reached.entry("down-the-line window given").or_default() += 1;
            }
            if call["views"]["down_the_line"].is_null() {
                *reached.entry("no down-the-line view").or_default() += 1;
            }
        }
    }
    for branch in [
        "face-on declined",
        "down the line declined",
        "face-on matched in reverse",
        "down the line alone: face-on given",
        "down the line alone: nothing measured",
        "(the keypoints file records no fps,",
        "(no plausible downswing,",
        "face-on window given",
        "down-the-line window given",
        "no down-the-line view",
    ] {
        assert!(
            reached.get(branch).is_some_and(|&n| n > 0),
            "no _auto_windows call reaches {branch:?}: {reached:?}"
        );
    }
    println!("{reached:?}");
}

/// The runner refuses what it cannot read, by name, rather than answering a call it misread.
#[test]
fn the_runner_refuses_what_it_cannot_read() {
    let clip = |tracks: Value| json!({"fps": 60.0, "frames": 2, "tracks": tracks});
    let wrist = json!({"LEFT_WRIST": {"y": [0.5, 0.5], "visibility": [1.0, 1.0]}});
    let rule = json!({
        "fn": "select_swing", "clip": "c", "fps": 60.0, "wrist": "LEFT_WRIST",
        "strike_frames": null,
    });
    let input = |clips: Value, call: Value| json!({"clips": clips, "calls": [call]});
    let mut extra = rule.clone();
    extra["window"] = json!([0, 1]);
    for (given, says) in [
        (
            input(
                json!({"c": clip(wrist.clone())}),
                json!({"fn": "segment_phases"}),
            ),
            "no selection call is named \"segment_phases\"",
        ),
        (
            input(json!({"d": clip(wrist.clone())}), rule.clone()),
            "no clip is named \"c\"",
        ),
        (
            input(
                json!({"c": clip(json!({"NOSE": {"y": [0.5, 0.5], "visibility": [1.0, 1.0]}}))}),
                rule.clone(),
            ),
            "\"NOSE\" is not a wrist",
        ),
        (
            input(
                json!({"c": clip(json!({"LEFT_WRIST": {"y": [0.5], "visibility": [1.0, 1.0]}}))}),
                rule.clone(),
            ),
            "not 2",
        ),
        (
            input(json!({"c": clip(wrist.clone())}), extra),
            "unknown field `window`",
        ),
    ] {
        let refused = run_selection(&given).expect_err(says);
        assert!(refused.contains(says), "{refused}");
    }
    let answered = run_selection(&input(json!({"c": clip(wrist)}), rule)).expect("reads");
    assert_eq!(answered, [Some(Value::Null)], "two frames hold no swing");
}
