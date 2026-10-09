//! The career scripts' report text, and the verbs that print it. [M36 P15–P16]
//!
//! `spec/vectors/career/` recorded each script's `_report` stdout over every case, plain and
//! `--verbose` (`expected.reports`, the M36 plan's call 3), `club_profile --club` once per club the
//! case names, and `flag_mishit --list`. This holds every report of a script
//! [`golf_core::career_family::run_career`] renders ([`RENDERED`]) to that text **to the character**:
//! no tolerance, because a report is prose a golfer reads, and a digit or a space out of place is the
//! failure. Through `run_career`, the runner `golf-core rerecord` records with, so the text gated
//! here is the text a re-record writes. A carried script ([`CARRIED`]) would wait for the phase named
//! in [`DEFERRED`]; since P16 there is none.
//!
//! Then the verbs' `main`, which no recorded report holds (P3's recorder captured `_report` alone):
//! who a verb reports on, its refusals, the trailing blank line, `--json`, and that a verb over a
//! sessions tree prints the renderer's answer for the corpus that tree reads as. And `flag-mishit`'s
//! write, on a scratch copy of a vector's tree, never on `data/`: the manifest it writes reads back
//! through the storage family's readers, and its verdict is one frozen Python's `MishitVerdict`
//! accepts (ADR-035 clause 4: the frozen lab must keep reading what Rust writes).

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use analysis::club_profile::build_bag_profile;
use contracts::career::CareerCorpus;
use contracts::club::ClubId;
use contracts::mishit::MishitVerdict;
use contracts::swing::{ANALYSIS_VERSION, COMPARABLE_FROM};
use contracts::Timestamp;
use flate2::read::GzDecoder;
use golf_core::career_family::{run_career, script_of, CARRIED, RENDERED};
use golf_core::reports::club_profile::club_profile;
use golf_core::reports::corpus::career_corpus;
use golf_core::reports::flag_mishit::{flag_mishit, flag_mishit_list, MishitAction};
use golf_core::reports::{answer, targets, DataDirs, Request, Targets, Verb};
use serde_json::Value;
use storage::corpus::{read_corpus, EngineVersions};
use storage::golfer_store::GolferStore;
use storage::manifest::{load_manifest, manifest_path};

/// Each carried script, with the phase that renders it. Empty since P16 rendered `club_profile`
/// and `flag_mishit`; a script added to the recorder before its renderer exists is named here.
const DEFERRED: [(&str, &str); 0] = [];

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

/// Every vector in one family's directory, sorted by id. Duplicated from `tests/career.rs`, for
/// `engine.rs`'s reason: integration tests are separate binaries.
fn family_vectors(family: &str) -> Vec<(String, Value)> {
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
    collect(&vectors_dir().join(family), &mut paths);
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
    assert!(
        !vectors.is_empty(),
        "no vectors under spec/vectors/{family}"
    );
    vectors
}

/// Where two texts first part, by line, so a failure names the line a reader would look at.
fn first_difference(recorded: &str, ours: &str) -> String {
    let (mut theirs, mut mine) = (recorded.split('\n'), ours.split('\n'));
    for line in 1.. {
        match (theirs.next(), mine.next()) {
            (Some(a), Some(b)) if a == b => continue,
            (None, None) => return "identical".to_string(),
            (a, b) => return format!("line {line}: recorded {a:?}, rendered {b:?}"),
        }
    }
    unreachable!("the loop returns")
}

/// **The gate**: every report of every [`RENDERED`] script, on every career vector, the real one
/// included, is the recorded text to the character; and `run_career` renders no report the vector
/// did not record.
#[test]
fn every_rendered_report_is_the_recorded_text_to_the_character() {
    let mut failures = Vec::new();
    let mut checked = 0;
    for (id, vector) in family_vectors("career") {
        let built = run_career(&vector).unwrap_or_else(|e| panic!("{e}"));
        let recorded = vector["expected"]["reports"]
            .as_object()
            .unwrap_or_else(|| panic!("{id}: no expected.reports"));
        for (key, text) in recorded {
            let script =
                script_of(key).unwrap_or_else(|| panic!("{id}: no script printed {key:?}"));
            if !RENDERED.contains(&script) {
                continue;
            }
            let text = text.as_str().expect("a report is a string");
            match built["reports"][key].as_str() {
                Some(ours) if ours == text => checked += 1,
                Some(ours) => {
                    failures.push(format!("{id}: {key}: {}", first_difference(text, ours)))
                }
                None => failures.push(format!("{id}: {key}: not rendered")),
            }
        }
        for key in built["reports"]
            .as_object()
            .expect("rendered reports")
            .keys()
        {
            assert!(
                recorded.contains_key(key),
                "{id}: {key} is rendered and not recorded"
            );
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    // At least nine reports a vector (three career scripts and `club_profile`, each plain and
    // verbose, and the listing), so a run that checked a handful passed by skipping the rest.
    assert!(checked > 9 * 50, "only {checked} reports checked");
    println!("{checked} reports identical");
}

/// Every report key a vector records belongs to a rendered script or to a carried one waiting on a
/// named phase, and the carried scripts are exactly the deferred ones: a report cannot sit unchecked
/// without a phase that owns it.
#[test]
fn every_report_is_rendered_or_deferred_to_a_named_phase() {
    let deferred: Vec<&str> = DEFERRED.iter().map(|(script, _)| *script).collect();
    assert_eq!(
        deferred, CARRIED,
        "the carried scripts and the deferred ones"
    );
    let mut seen = std::collections::BTreeSet::new();
    for (id, vector) in family_vectors("career") {
        for key in vector["expected"]["reports"]
            .as_object()
            .expect("expected.reports")
            .keys()
        {
            let script =
                script_of(key).unwrap_or_else(|| panic!("{id}: no script printed {key:?}"));
            seen.insert(script);
        }
    }
    for script in RENDERED.iter().chain(&CARRIED) {
        assert!(
            seen.contains(script),
            "{script} is named and no vector has its report"
        );
    }
    println!("deferred: {DEFERRED:?}");
}

// ---------------------------------------------------------------------------------- the verbs

/// A scratch directory under the system temp dir, removed on drop. `tempfile` is not one of this
/// crate's dependencies (the M36 plan keeps it a dev-dependency of `storage` alone, P13 finding 6).
struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "golf-core-reports-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).expect("a scratch directory");
        Scratch(path)
    }

    fn write(&self, relative: &str, text: &str) {
        let path = self.0.join(relative);
        fs::create_dir_all(path.parent().expect("a parent")).expect("parent directories");
        fs::write(&path, text).expect("a scratch file");
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

const GOLFER: &str = r#"{"player_id": "aaron", "display_name": "Aaron", "handedness": "right",
    "created_at": "2026-08-07T15:04:05.123456Z"}"#;

/// The hand-worked case read under exactly the verbs' engine generations, `{17, 14}` (M36 P14).
const HAND_CASE: &str = "storage/hand/versions-either-side.json";

fn verbs_versions() -> EngineVersions {
    EngineVersions {
        installed: ANALYSIS_VERSION,
        comparable_from: COMPARABLE_FROM,
    }
}

/// A scratch tree holding [`HAND_CASE`]'s sessions and one registered golfer, `aaron`.
fn hand_tree() -> (Scratch, DataDirs, Value) {
    let vector = read_vector(&vectors_dir().join(HAND_CASE));
    assert_eq!(
        vector["input"]["versions"],
        serde_json::json!({"installed": ANALYSIS_VERSION, "comparable_from": COMPARABLE_FROM}),
        "{HAND_CASE} is no longer read under the verbs' versions; pick another case"
    );
    let scratch = Scratch::new();
    for (relative, text) in vector["input"]["files"].as_object().expect("input.files") {
        scratch.write(
            &format!("sessions/{relative}"),
            text.as_str().expect("a file's text"),
        );
    }
    scratch.write("golfers/aaron.golfer.json", GOLFER);
    let dirs = DataDirs {
        sessions: scratch.0.join("sessions"),
        golfers: scratch.0.join("golfers"),
    };
    (scratch, dirs, vector)
}

fn request(dirs: &DataDirs, name: Option<&str>, player_id: Option<&str>) -> Request {
    Request {
        name: name.map(str::to_string),
        player_id: player_id.map(str::to_string),
        verbose: false,
        club: None,
        json: false,
        dirs: dirs.clone(),
    }
}

/// The scripts' `main` up to the loop: a name is slugged, an id is taken as given, an unregistered
/// id is still read under `(not registered)`, an empty string is absent, and a name with no usable
/// characters and an empty registry are the two refusals.
#[test]
fn a_verb_reports_on_whom_the_scripts_would() {
    let (scratch, dirs, _) = hand_tree();
    let golfers = GolferStore::new(&dirs.golfers);
    let aaron = Targets::Golfers(vec![("aaron".into(), "Aaron".into())]);
    assert_eq!(targets(&golfers, Some("  AARON "), None), aaron);
    assert_eq!(targets(&golfers, None, Some("aaron")), aaron);
    assert_eq!(targets(&golfers, Some("Aaron"), Some("")), aaron);
    assert_eq!(
        targets(&golfers, Some(""), Some("")),
        aaron,
        "both empty lists the registry"
    );
    assert_eq!(
        targets(&golfers, Some("nobody"), Some("aaron")),
        aaron,
        "the id wins"
    );
    assert_eq!(
        targets(&golfers, None, Some("nobody")),
        Targets::Golfers(vec![("nobody".into(), "nobody (not registered)".into())])
    );
    assert_eq!(
        targets(&golfers, Some("!!!"), None),
        Targets::NoUsableId("!!!".into())
    );
    let empty = GolferStore::new(scratch.0.join("no-golfers-here"));
    assert_eq!(targets(&empty, None, None), Targets::NoneRegistered);
}

/// A verb over a sessions tree prints the renderer's report for the corpus that tree reads as, and
/// then the script's trailing `print()`: over [`HAND_CASE`]'s tree, that corpus is the case's
/// `expected.corpus`, which the storage family gates.
#[test]
fn a_verb_prints_the_report_of_the_corpus_its_tree_reads_as_and_one_blank_line() {
    let (_scratch, dirs, vector) = hand_tree();
    let corpus: CareerCorpus =
        serde_json::from_value(vector["expected"]["corpus"].clone()).expect("a corpus");
    for verbose in [false, true] {
        let request = Request {
            verbose,
            ..request(&dirs, Some("Aaron"), None)
        };
        let answered = answer(Verb::CareerCorpus, &request, verbs_versions());
        let report = career_corpus(&corpus, "Aaron", ANALYSIS_VERSION, verbose);
        assert_eq!(answered.stdout, format!("{report}\n"));
        assert_eq!((answered.stderr.as_str(), answered.code), ("", 0));
    }
    // The corpus reaches the outdated paragraph, so the installed engine it names is the verbs'.
    let answered = answer(
        Verb::CareerCorpus,
        &request(&dirs, None, None),
        verbs_versions(),
    );
    assert!(
        answered
            .stdout
            .contains(&format!("(version 13; {ANALYSIS_VERSION} is installed)")),
        "{}",
        answered.stdout
    );
}

/// The two refusals, as the scripts print them, and `--json`'s versions of them: stdout stays JSON.
#[test]
fn the_refusals_print_what_the_scripts_print_and_json_keeps_stdout_json() {
    let (scratch, dirs, _) = hand_tree();
    for verb in Verb::ALL {
        let refused = answer(verb, &request(&dirs, Some("!!!"), None), verbs_versions());
        assert_eq!(
            (refused.stdout.as_str(), refused.code),
            ("'!!!' has no usable characters for an id.\n", 2),
            "{verb:?}"
        );

        let empty = DataDirs {
            golfers: scratch.0.join("no-golfers-here"),
            ..dirs.clone()
        };
        let none = answer(verb, &request(&empty, None, None), verbs_versions());
        assert_eq!(
            none.stdout,
            format!("No golfers registered in {}.\n", empty.golfers.display()),
            "{verb:?}"
        );
        assert_eq!(none.code, 0);

        let json = |dirs: &DataDirs, name: Option<&str>| {
            answer(
                verb,
                &Request {
                    json: true,
                    ..request(dirs, name, None)
                },
                verbs_versions(),
            )
        };
        let refused = json(&dirs, Some("!!!"));
        assert_eq!(refused.stdout, "");
        assert_eq!(refused.code, 2);
        assert!(refused.stderr.contains("no usable characters"));
        let none = json(&empty, None);
        assert_eq!((none.stdout.as_str(), none.code), ("[]\n", 0));
        assert!(none.stderr.starts_with("No golfers registered"));
    }
}

/// `--json` prints one array, an object per golfer, holding the display name and each aggregate the
/// report was rendered from under the career family's name for it.
#[test]
fn json_prints_the_aggregates_each_report_was_rendered_from() {
    let (_scratch, dirs, vector) = hand_tree();
    for (verb, keys) in [
        (Verb::CareerCorpus, &["corpus"][..]),
        (Verb::CareerBaseline, &["baseline", "standing"][..]),
        (Verb::CareerDispersion, &["dispersion"][..]),
        (Verb::ClubProfile, &["bag_profile"][..]),
    ] {
        let answered = answer(
            verb,
            &Request {
                json: true,
                ..request(&dirs, None, Some("aaron"))
            },
            verbs_versions(),
        );
        assert_eq!(answered.code, 0);
        let printed: Value = serde_json::from_str(&answered.stdout).expect("stdout is JSON");
        let entry = &printed.as_array().expect("an array")[..];
        assert_eq!(entry.len(), 1, "{verb:?}");
        let mut expected_keys = vec!["display_name"];
        expected_keys.extend(keys);
        let mut found: Vec<&str> = entry[0]
            .as_object()
            .expect("an object")
            .keys()
            .map(String::as_str)
            .collect();
        found.sort_unstable();
        expected_keys.sort_unstable();
        assert_eq!(found, expected_keys, "{verb:?}");
        assert_eq!(entry[0]["display_name"], "Aaron");
        for key in keys {
            assert_eq!(entry[0][key]["player_id"], "aaron", "{verb:?} {key}");
        }
        if verb == Verb::CareerCorpus {
            assert_eq!(
                entry[0]["corpus"]["metric_counts"],
                vector["expected"]["corpus"]["metric_counts"]
            );
        }
    }
}

/// The binary end to end: the subcommand dispatches to [`answer`] under this build's engine
/// generations, the data-directory flags reach it in both spellings, an unknown flag exits 2 as
/// argparse does, and `--help` exits 0.
#[test]
fn the_binary_runs_each_verb_over_its_flags() {
    let (_scratch, dirs, _) = hand_tree();
    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_golf-core"))
            .args(args)
            .env_remove("GOLF_SESSIONS_DIR")
            .env_remove("GOLF_GOLFERS_DIR")
            .output()
            .expect("golf-core runs")
    };
    let sessions = dirs.sessions.to_string_lossy().to_string();
    let golfers = dirs.golfers.to_string_lossy().to_string();
    for verb in Verb::ALL {
        let expected = answer(
            verb,
            &Request {
                verbose: true,
                ..request(&dirs, Some("Aaron"), None)
            },
            verbs_versions(),
        );
        let output = run(&[
            verb.name(),
            "--name",
            "Aaron",
            "--verbose",
            "--sessions-dir",
            &sessions,
            &format!("--golfers-dir={golfers}"),
        ]);
        assert_eq!(output.status.code(), Some(0), "{verb:?}");
        assert_eq!(
            String::from_utf8_lossy(&output.stdout),
            expected.stdout,
            "{verb:?}"
        );

        let refused = run(&[verb.name(), "--colour"]);
        assert_eq!(refused.status.code(), Some(2), "{verb:?}");
        assert!(refused.stdout.is_empty());
        assert!(String::from_utf8_lossy(&refused.stderr).contains("unrecognized arguments"));

        let missing = run(&[verb.name(), "--name"]);
        assert_eq!(missing.status.code(), Some(2), "{verb:?}");
        assert!(String::from_utf8_lossy(&missing.stderr).contains("expected one argument"));

        let help = run(&[verb.name(), "--help"]);
        assert_eq!(help.status.code(), Some(0), "{verb:?}");
        assert!(String::from_utf8_lossy(&help.stdout).contains(verb.name()));

        // `--club` is `club_profile.py`'s flag alone; the other scripts' argparse refuses it.
        let club = run(&[
            verb.name(),
            "--club",
            "7i",
            "--sessions-dir",
            &sessions,
            "--golfers-dir",
            &golfers,
        ]);
        let expected = if verb == Verb::ClubProfile {
            Some(0)
        } else {
            Some(2)
        };
        assert_eq!(club.status.code(), expected, "{verb:?} --club");
    }
}

// ------------------------------------------------------------------------------ club-profile

/// `club-profile` over a tree prints the renderer's report for the bag profile that tree reads as,
/// with the golfer's bag from the golfers directory, then one blank line; `--club` takes any
/// spelling `parse_club` takes and narrows the report to that club; text that is not a club is
/// refused **before** the golfer is looked up, as the script orders it.
#[test]
fn club_profile_prints_the_bag_profile_its_tree_reads_as() {
    let (scratch, dirs, vector) = hand_tree();
    let corpus: CareerCorpus =
        serde_json::from_value(vector["expected"]["corpus"].clone()).expect("a corpus");
    // A bag beside the golfer, so the bag-entry block is the declared one.
    scratch.write(
        "golfers/aaron.bag.json",
        r#"{"player_id": "aaron", "entries": {"7i": {"club": "7i", "make": "Mizuno",
            "model": "JPX 923", "loft_deg": 30.5, "length_in": 37.25,
            "recorded_at": "2026-08-01T23:30:00-05:00"}},
            "updated_at": "2026-08-01T23:30:00-05:00"}"#,
    );
    let bag = storage::bag_store::BagStore::new(&dirs.golfers)
        .get("aaron")
        .expect("the bag reads");
    let profile = build_bag_profile(&corpus, Some(&bag));

    for (club, parsed) in [
        (None, None),
        (Some(""), None),
        (Some("seven iron"), Some(ClubId::SevenIron)),
        (Some("pw"), Some(ClubId::PitchingWedge)),
    ] {
        let answered = answer(
            Verb::ClubProfile,
            &Request {
                club: club.map(str::to_string),
                verbose: true,
                ..request(&dirs, Some("Aaron"), None)
            },
            verbs_versions(),
        );
        let report = club_profile(&profile, "Aaron", true, parsed);
        assert_eq!(answered.stdout, format!("{report}\n"), "{club:?}");
        assert_eq!((answered.stderr.as_str(), answered.code), ("", 0));
    }
    let seven = club_profile(&profile, "Aaron", false, Some(ClubId::SevenIron));
    assert!(
        seven.contains("Mizuno JPX 923, 30.5 deg loft, 37.25 in (declared 2026-08-01)"),
        "{seven}"
    );

    for json in [false, true] {
        let refused = answer(
            Verb::ClubProfile,
            &Request {
                club: Some("wedge".into()),
                json,
                ..request(&dirs, Some("!!!"), None)
            },
            verbs_versions(),
        );
        let text = "'wedge' is not a club. Try '7i', '7 iron', 'driver' or 'pw'.\n'wedge' and \
                    'iron' name a category rather than a club, so neither is accepted.\n";
        let printed = if json {
            &refused.stderr
        } else {
            &refused.stdout
        };
        assert_eq!((printed.as_str(), refused.code), (text, 2), "json {json}");
    }
}

// ------------------------------------------------------------------------------- flag-mishit

/// A fixed clock, so the stamp a write leaves can be checked.
fn fixed_now() -> Timestamp {
    "2026-10-08T20:30:00.250000Z".parse().expect("a timestamp")
}

/// The manifest's `mishit` as written, held to the enum frozen Python exported into
/// `spec/schemas/swing_manifest.schema.json`: absent or `null` is "no verdict", and a string must
/// be one of `MishitVerdict`'s members, which is what `load_manifest` in frozen Python reads.
fn assert_python_reads_the_verdict(manifest: &Path) {
    let schema: Value = serde_json::from_str(
        &fs::read_to_string(vectors_dir().join("../schemas/swing_manifest.schema.json"))
            .expect("the committed manifest schema"),
    )
    .expect("the schema is JSON");
    let members = schema["$defs"]["MishitVerdict"]["enum"]
        .as_array()
        .expect("MishitVerdict's members");
    let written: Value =
        serde_json::from_str(&fs::read_to_string(manifest).expect("the manifest")).expect("JSON");
    match &written["mishit"] {
        Value::Null => {}
        verdict => assert!(
            members.contains(verdict),
            "{verdict} is not one of {members:?}"
        ),
    }
}

/// **`flag-mishit`'s write path, on a scratch copy of a vector's tree** (the M36 plan's P16 gate):
/// each verdict and `--auto` is written through `SwingBundleStore::set_mishit` with the clock passed
/// in, reads back through `load_manifest` and through `read_corpus` (the storage family's
/// readers), leaves a value frozen Python's `MishitVerdict` accepts, and touches no other file.
#[test]
fn flag_mishit_writes_a_verdict_both_languages_read_back() {
    let (scratch, dirs, vector) = hand_tree();
    let reference = "2026-08-10/2";
    let manifest = manifest_path(&dirs.sessions.join("2026-08-10").join("2"));
    let untouched: Vec<(String, String)> = vector["input"]["files"]
        .as_object()
        .expect("input.files")
        .iter()
        .filter(|(relative, _)| relative.as_str() != "2026-08-10/2/manifest.json")
        .map(|(relative, text)| (relative.clone(), text.as_str().expect("text").to_string()))
        .collect();

    for (verdict, said) in [
        (Some(MishitVerdict::Confirmed), "confirmed"),
        (Some(MishitVerdict::Cleared), "cleared"),
        (None, "auto (no verdict -- the rule decides)"),
    ] {
        let flagged = flag_mishit(
            &MishitAction::Flag {
                reference: reference.into(),
                verdict,
            },
            &dirs,
            verbs_versions(),
            fixed_now(),
        );
        assert_eq!(
            (
                flagged.stdout.as_str(),
                flagged.stderr.as_str(),
                flagged.code
            ),
            (format!("{reference}: mishit = {said}\n").as_str(), "", 0)
        );
        let read = load_manifest(&manifest).expect("the written manifest reads back");
        assert_eq!(read.mishit, verdict);
        assert_eq!(read.updated_at.to_string(), fixed_now().to_string());
        assert_python_reads_the_verdict(&manifest);

        let corpus = read_corpus(&dirs.sessions, "aaron", verbs_versions());
        let swing = corpus
            .swings
            .iter()
            .find(|swing| swing.session_id == "2026-08-10" && swing.swing_id == "2")
            .expect("the flagged swing is pooled");
        assert_eq!(swing.manual_mishit, verdict);
    }
    for (relative, text) in &untouched {
        let now =
            fs::read_to_string(dirs.sessions.join(relative)).expect("the file is still there");
        assert_eq!(&now, text, "{relative} was written");
    }

    // The refusals, printed on stdout and exiting 2 as the script's do, and writing nothing.
    for (reference, printed) in [
        (
            "2026-08-10",
            "'2026-08-10' is not a SESSION/SWING reference, e.g. 2026-08-10/3\n",
        ),
        (
            "2026-08-10/",
            "'2026-08-10/' is not a SESSION/SWING reference, e.g. 2026-08-10/3\n",
        ),
        (
            "a/b/c",
            "'a/b/c' is not a SESSION/SWING reference, e.g. 2026-08-10/3\n",
        ),
        ("2026-08-10/9", "no swing 2026-08-10/9\n"),
    ] {
        let refused = flag_mishit(
            &MishitAction::Flag {
                reference: reference.into(),
                verdict: Some(MishitVerdict::Confirmed),
            },
            &dirs,
            verbs_versions(),
            fixed_now(),
        );
        assert_eq!((refused.stdout.as_str(), refused.code), (printed, 2));
    }
    assert!(!scratch.0.join("sessions/2026-08-10/9").exists());
}

/// `flag-mishit --list` is `_list`'s report for each golfer, over the bag profile built with **no
/// bag**, with no trailing blank line (the script's `main` returns `_list` directly); `--name` slugs,
/// and a name no record answers is refused, a name that slugs to nothing included.
#[test]
fn flag_mishit_lists_each_golfers_tally() {
    let (_scratch, dirs, vector) = hand_tree();
    let corpus: CareerCorpus =
        serde_json::from_value(vector["expected"]["corpus"].clone()).expect("a corpus");
    let listing = flag_mishit_list("aaron", &build_bag_profile(&corpus, None));
    for name in [None, Some(""), Some("  AARON ")] {
        let listed = flag_mishit(
            &MishitAction::List {
                name: name.map(str::to_string),
            },
            &dirs,
            verbs_versions(),
            fixed_now(),
        );
        assert_eq!(
            (listed.stdout.as_str(), listed.code),
            (listing.as_str(), 0),
            "{name:?}"
        );
    }
    for (name, printed) in [
        ("nobody", "no golfer 'nobody'\n"),
        ("!!!", "no golfer ''\n"),
    ] {
        let refused = flag_mishit(
            &MishitAction::List {
                name: Some(name.into()),
            },
            &dirs,
            verbs_versions(),
            fixed_now(),
        );
        assert_eq!((refused.stdout.as_str(), refused.code), (printed, 2));
    }
}

/// The `flag-mishit` binary: a flag and a listing end to end, and argparse's refusal (no verdict,
/// or two) exiting 2 with stdout empty.
#[test]
fn the_binary_flags_and_lists() {
    let (_scratch, dirs, _) = hand_tree();
    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_golf-core"))
            .arg("flag-mishit")
            .args(args)
            .arg(format!("--sessions-dir={}", dirs.sessions.display()))
            .arg("--golfers-dir")
            .arg(&dirs.golfers)
            .env_remove("GOLF_SESSIONS_DIR")
            .env_remove("GOLF_GOLFERS_DIR")
            .output()
            .expect("golf-core runs")
    };
    let flagged = run(&["2026-08-10/3", "--confirm"]);
    assert_eq!(flagged.status.code(), Some(0));
    assert_eq!(
        String::from_utf8_lossy(&flagged.stdout),
        "2026-08-10/3: mishit = confirmed\n"
    );
    let read = load_manifest(&manifest_path(&dirs.sessions.join("2026-08-10").join("3")))
        .expect("the manifest reads back");
    assert_eq!(read.mishit, Some(MishitVerdict::Confirmed));

    let listed = run(&["--list", "--name", "Aaron"]);
    assert_eq!(listed.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&listed.stdout).starts_with("\naaron -- "));

    for args in [
        &["2026-08-10/3"][..],
        &["--confirm"][..],
        &["2026-08-10/3", "--confirm", "--clear"][..],
        &["2026-08-10/3", "2026-08-10/4", "--auto"][..],
        &["--colour"][..],
    ] {
        let refused = run(args);
        assert_eq!(refused.status.code(), Some(2), "{args:?}");
        assert!(refused.stdout.is_empty(), "{args:?}");
    }
    let help = Command::new(env!("CARGO_BIN_EXE_golf-core"))
        .args(["flag-mishit", "--help"])
        .output()
        .expect("golf-core runs");
    assert_eq!(help.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&help.stdout).contains("flag-mishit"));
}
