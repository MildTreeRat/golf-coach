//! `golf-core rerecord` against a copy of a slice of `spec/`. [M32 P5]
//!
//! The rules are unit-tested in `golf_core::rerecord` over in-memory documents; this is the run over
//! files — the walk, the join, the atomicity, the typo and version guards, and what lands on disk.
//! Each test copies one synthetic and one corpus engine vector and their two stage vectors into a
//! directory of its own under the system temp dir (`crates/pose/tests/writer.rs`' precedent), so
//! nothing here can touch the committed `spec/`.
//!
//! **A real difference without changing the engine** is made by editing the *committed copy*: a
//! version set one below this build's, or a key removed from `expected` that Rust's answer still
//! has. Rust's answer then differs from the copy in exactly the way a re-record would see, and the
//! test declares it — or does not. Every edit is relative to [`ANALYSIS_VERSION`], so the tests mean
//! the same thing after M32 P8 moves it.
//!
//! The two picked are the smallest of their halves, because every run here is a debug build running
//! the engine on them.

use std::collections::BTreeMap;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use contracts::swing::ANALYSIS_VERSION;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use golf_core::compare::compare;
use golf_core::rerecord::{plan, Refused};
use serde_json::{json, Value};

const SYNTHETIC: &str = "synthetic/tempo-too-quick.json";
const CORPUS: &str = "corpus/2026-08-23-11.json.gz";
const SYNTHETIC_ID: &str = "synthetic/tempo-too-quick";
const CORPUS_ID: &str = "corpus/2026-08-23-11";

/// The key the tests remove from the corpus copy's answer. Nothing the compose check reads, so a
/// declared re-add exercises the gate and the ledger without also moving a composed value.
const SCORE: &str = "expected.swing.overall_score";

fn committed_spec() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("spec")
}

fn read(path: &Path) -> Value {
    let bytes = fs::read(path).unwrap_or_else(|e| panic!("read {path:?}: {e}"));
    let text = if path.extension().is_some_and(|e| e == "gz") {
        let mut text = String::new();
        GzDecoder::new(&bytes[..])
            .read_to_string(&mut text)
            .unwrap_or_else(|e| panic!("gunzip {path:?}: {e}"));
        text
    } else {
        String::from_utf8(bytes).unwrap_or_else(|e| panic!("utf-8 {path:?}: {e}"))
    };
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("parse {path:?}: {e}"))
}

/// One test's copy of the slice, under `{root}/repo/spec/`, removed when the test ends.
struct Slice {
    root: PathBuf,
}

impl Slice {
    fn new(name: &str) -> Self {
        let root =
            std::env::temp_dir().join(format!("golf-core-rerecord-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        for relative in [SYNTHETIC, CORPUS] {
            for family in ["", "stages/"] {
                let relative = format!("{family}{relative}");
                let to = root
                    .join("repo")
                    .join("spec")
                    .join("vectors")
                    .join(&relative);
                fs::create_dir_all(to.parent().expect("a parent")).expect("mkdir");
                fs::copy(committed_spec().join("vectors").join(&relative), &to)
                    .unwrap_or_else(|e| panic!("copy {relative}: {e}"));
            }
        }
        fs::create_dir_all(root.join("repo").join("spec").join("declarations")).expect("mkdir");
        Self { root }
    }

    /// `{root}/repo/spec`: one level down, so `{root}` is a place outside the "repository" to put
    /// a declaration in.
    fn spec(&self) -> PathBuf {
        self.root.join("repo").join("spec")
    }

    fn vector(&self, relative: &str) -> PathBuf {
        self.spec().join("vectors").join(relative)
    }

    /// Write `spec/declarations/{name}.json` and return its path.
    fn declare(&self, name: &str, version: i64, added: &[&str], moved: &[&str]) -> PathBuf {
        let path = self
            .spec()
            .join("declarations")
            .join(format!("{name}.json"));
        let declaration = json!({
            "analysis_version": version,
            "note": format!("tests/rerecord.rs: {name}"),
            "added": added,
            "moved": moved,
        });
        fs::write(&path, declaration.to_string()).expect("write a declaration");
        path
    }

    /// Edit a committed copy in place. Written plainly — the copy's own text is not under test.
    fn edit(&self, relative: &str, change: impl FnOnce(&mut Value)) {
        let path = self.vector(relative);
        let mut document = read(&path);
        change(&mut document);
        let text = serde_json::to_string_pretty(&document).expect("serializes");
        if relative.ends_with(".gz") {
            let mut encoder = GzEncoder::new(Vec::new(), Compression::fast());
            encoder.write_all(text.as_bytes()).expect("gzip");
            fs::write(&path, encoder.finish().expect("gzip")).expect("write");
        } else {
            fs::write(&path, text).expect("write");
        }
    }

    /// The engine vector and its stage twin recorded one version back, so moving both versions is a
    /// real difference.
    fn age_the_synthetic_pair(&self) {
        self.edit(SYNTHETIC, |v| {
            v["analysis_version"] = json!(ANALYSIS_VERSION - 1);
            v["expected"]["analysis_version"] = json!(ANALYSIS_VERSION - 1);
        });
        self.edit(&format!("stages/{SYNTHETIC}"), |v| {
            v["analysis_version"] = json!(ANALYSIS_VERSION - 1);
        });
    }

    fn drop_the_corpus_score(&self) {
        self.edit(CORPUS, |v| {
            v["expected"]["swing"]
                .as_object_mut()
                .expect("a swing")
                .remove("overall_score")
                .expect("the corpus answer has an overall score");
        });
    }

    /// Every file under the slice's `spec/`, bytes and all — so "wrote nothing" is checked on the
    /// bytes, and a staged file left behind shows up as a new path.
    fn snapshot(&self) -> BTreeMap<PathBuf, Vec<u8>> {
        fn walk(dir: &Path, into: &mut BTreeMap<PathBuf, Vec<u8>>) {
            for entry in fs::read_dir(dir).expect("list") {
                let path = entry.expect("an entry").path();
                if path.is_dir() {
                    walk(&path, into);
                } else {
                    into.insert(path.clone(), fs::read(&path).expect("read"));
                }
            }
        }
        let mut files = BTreeMap::new();
        walk(&self.spec(), &mut files);
        files
    }

    /// The `golf-core` binary, run on this slice.
    fn verb(&self, declaration: &Path, extra: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_golf-core"))
            .arg("rerecord")
            .arg("--declare")
            .arg(declaration)
            .arg("--spec")
            .arg(self.spec())
            .args(extra)
            .output()
            .expect("the binary runs")
    }
}

impl Drop for Slice {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// **An undeclared difference anywhere writes nothing anywhere** (call 7) — checked through the
/// binary, because [`plan`] never writes and the property is the verb's.
///
/// The synthetic pair's version move is declared and would land on its own; the corpus copy's
/// missing score is not. The run must refuse, name the undeclared path, and leave every byte —
/// the synthetic pair's included — as it was.
#[test]
fn an_undeclared_difference_fails_and_writes_nothing_anywhere() {
    let slice = Slice::new("undeclared");
    slice.age_the_synthetic_pair();
    slice.drop_the_corpus_score();
    let declaration = slice.declare(
        "versions-only",
        ANALYSIS_VERSION,
        &[],
        &["analysis_version", "expected.analysis_version"],
    );
    let before = slice.snapshot();

    let output = slice.verb(&declaration, &[]);
    let stderr = text(&output.stderr);
    assert!(!output.status.success(), "a refused run exits non-zero");
    assert!(
        stderr.contains(&format!("{CORPUS_ID}: {SCORE}")) && stderr.contains("`added`"),
        "the refusal names the vector and the path:\n{stderr}"
    );
    assert!(stderr.contains("nothing was written anywhere"), "{stderr}");
    assert_eq!(slice.snapshot(), before, "a refused run wrote something");

    match plan(&slice.spec(), &declaration) {
        Err(Refused::Gate(refusals)) => assert_eq!(refusals.len(), 1, "{refusals:?}"),
        other => panic!("expected the gate to refuse, got {other:?}"),
    }
}

/// **A declared change lands, reads back as it was planned, carries its ledger, keeps every other
/// value — and a second run writes nothing.**
#[test]
fn a_declared_change_lands_reads_back_and_a_second_run_writes_nothing() {
    let slice = Slice::new("declared");
    slice.age_the_synthetic_pair();
    slice.drop_the_corpus_score();
    let committed: BTreeMap<&str, Value> =
        [SYNTHETIC, CORPUS, "stages/synthetic/tempo-too-quick.json"]
            .into_iter()
            .map(|relative| (relative, read(&slice.vector(relative))))
            .collect();
    let declaration = slice.declare(
        "score-and-versions",
        ANALYSIS_VERSION,
        &[SCORE],
        &["analysis_version", "expected.analysis_version"],
    );

    let run = plan(&slice.spec(), &declaration).expect("everything that differs is declared");
    let changed: Vec<&str> = run.changed().map(|v| v.id()).collect();
    assert_eq!(
        changed,
        [SYNTHETIC_ID, CORPUS_ID, "stages/synthetic/tempo-too-quick"],
        "engine family first, synthetic half before corpus; the corpus stage vector is untouched"
    );
    assert_eq!(run.vectors().len(), 4);
    assert_eq!(run.write().expect("writes"), 3);

    // Each file reads back as the document that was planned for it.
    for vector in run.changed() {
        assert_eq!(
            &read(vector.path()),
            vector.written().expect("a changed vector has a document"),
            "{}: read back differently",
            vector.id()
        );
    }

    // Each ledger lists what matched in *that* file, and names the declaration repo-relatively.
    let ledger_of = |relative: &str| {
        let document = read(&slice.vector(relative));
        let entries = document["provenance"]["rerecords"]
            .as_array()
            .unwrap_or_else(|| panic!("{relative}: no ledger"))
            .clone();
        assert!(
            document["provenance"]["oracle"].is_string(),
            "{relative}: no oracle"
        );
        entries.last().expect("an entry").clone()
    };
    let entry = |added: &[&str], moved: &[&str]| {
        json!({
            "analysis_version": ANALYSIS_VERSION,
            "by": "golf-core rerecord",
            "declaration": "spec/declarations/score-and-versions.json",
            "added": added,
            "moved": moved,
        })
    };
    assert_eq!(ledger_of(CORPUS), entry(&[SCORE], &[]));
    assert_eq!(
        ledger_of(SYNTHETIC),
        entry(&[], &["analysis_version", "expected.analysis_version"])
    );
    assert_eq!(
        ledger_of("stages/synthetic/tempo-too-quick.json"),
        entry(&[], &["analysis_version"])
    );

    // Every undeclared value is the committed one: undo the declared paths and the provenance, and
    // what is left is the copy as it was, exactly — `Value`'s `==` compares floats as f64s.
    for (relative, before) in &committed {
        let mut undone = read(&slice.vector(relative));
        undone["provenance"] = before["provenance"].clone();
        if *relative == CORPUS {
            undone["expected"]["swing"]
                .as_object_mut()
                .expect("a swing")
                .remove("overall_score")
                .expect("the score landed");
        } else {
            undone["analysis_version"] = before["analysis_version"].clone();
            if undone.get("expected").is_some() {
                undone["expected"]["analysis_version"] =
                    before["expected"]["analysis_version"].clone();
            }
        }
        assert_eq!(&undone, before, "{relative}: an undeclared value moved");
    }
    // And the score that landed is Rust's answer, inside the tolerance of the one Python recorded.
    let recorded = read(&committed_spec().join("vectors").join(CORPUS));
    let landed = read(&slice.vector(CORPUS));
    assert!(compare(
        &recorded["expected"]["swing"]["overall_score"],
        &landed["expected"]["swing"]["overall_score"]
    )
    .is_empty());

    // The second run: nothing differs, every declared path is already in a ledger, nothing is
    // written — and it is a success, not a typo-guard refusal.
    let after_first = slice.snapshot();
    let second = plan(&slice.spec(), &declaration).expect("a second run is a no-op, not a refusal");
    assert_eq!(second.changed().count(), 0);
    assert_eq!(second.write().expect("writes nothing"), 0);
    assert_eq!(
        slice.snapshot(),
        after_first,
        "a second run wrote something"
    );
}

/// `--dry-run` prints the review — every applied path, with its value before and after — and
/// writes nothing.
#[test]
fn a_dry_run_reports_and_writes_nothing() {
    let slice = Slice::new("dry-run");
    slice.age_the_synthetic_pair();
    let declaration = slice.declare(
        "versions",
        ANALYSIS_VERSION,
        &[],
        &["analysis_version", "expected.analysis_version"],
    );
    let before = slice.snapshot();

    let output = slice.verb(&declaration, &["--dry-run"]);
    let stdout = text(&output.stdout);
    assert!(output.status.success(), "{}", text(&output.stderr));
    let moved = format!(
        "  moved  analysis_version: {} -> {ANALYSIS_VERSION}",
        ANALYSIS_VERSION - 1
    );
    for expected in [
        "--dry-run: nothing is written",
        "\nsynthetic/tempo-too-quick\n",
        "\nstages/synthetic/tempo-too-quick\n",
        moved.as_str(),
        "4 vectors run (2 engine, 2 stage); 2 changed; 0 files written",
    ] {
        assert!(
            stdout.contains(expected),
            "missing {expected:?} in:\n{stdout}"
        );
    }
    assert!(
        !stdout.contains(CORPUS_ID),
        "an unchanged vector is reported:\n{stdout}"
    );
    assert_eq!(slice.snapshot(), before, "a dry run wrote something");

    // And without the flag, the same declaration writes the two it reported.
    let output = slice.verb(&declaration, &[]);
    assert!(output.status.success(), "{}", text(&output.stderr));
    assert!(text(&output.stdout).contains("2 changed; 2 files written"));
}

/// Call 5: a declared path that matches nothing anywhere fails the run, so a misspelling cannot
/// "declare" nothing and pass.
#[test]
fn an_unmatched_declaration_entry_fails_the_run() {
    let slice = Slice::new("unmatched");
    let declaration = slice.declare(
        "misspelled",
        ANALYSIS_VERSION,
        &["expected.swing.overall_scroe"],
        &[],
    );
    let before = slice.snapshot();
    match plan(&slice.spec(), &declaration) {
        Err(Refused::Unmatched(paths)) => {
            assert_eq!(paths.len(), 1, "{paths:?}");
            assert!(
                paths[0].contains("expected.swing.overall_scroe") && paths[0].contains("call 5"),
                "{}",
                paths[0]
            );
        }
        other => panic!("expected the typo guard to refuse, got {other:?}"),
    }
    let output = slice.verb(&declaration, &[]);
    assert!(!output.status.success());
    assert_eq!(slice.snapshot(), before);
}

/// Call 6, and before a vector is read: the spec here does not exist, so a guard that ran after the
/// walk would fail on the walk instead.
#[test]
fn a_declaration_at_another_version_is_refused_before_anything_is_read() {
    let slice = Slice::new("version");
    let declaration = slice.declare("next", ANALYSIS_VERSION + 1, &[], &[]);
    match plan(&slice.root.join("no-such-spec"), &declaration) {
        Err(Refused::Declaration(why)) => {
            assert!(why.contains("call 6"), "{why}");
            assert!(
                why.contains(&format!("this engine is at {ANALYSIS_VERSION}")),
                "{why}"
            );
        }
        other => panic!("expected the version guard, got {other:?}"),
    }
}

/// Where a declaration may sit. Outside the repository it can drive a run that writes nothing — the
/// empty declaration M32 P5 ran over the real `spec/` from the session scratchpad — but not one that
/// would write a ledger naming it; and under `spec/vectors/` it is refused outright, because
/// `conformance.py` would read it as a vector (decision 2).
#[test]
fn a_declaration_is_named_by_its_place_in_the_repository() {
    let slice = Slice::new("place");
    let outside = slice.root.join("outside.json");

    fs::write(
        &outside,
        json!({"analysis_version": ANALYSIS_VERSION, "note": "", "added": [], "moved": []})
            .to_string(),
    )
    .expect("write");
    let quiet = plan(&slice.spec(), &outside).expect("nothing to write, so nothing to name");
    assert_eq!(quiet.changed().count(), 0);

    slice.age_the_synthetic_pair();
    fs::write(
        &outside,
        json!({
            "analysis_version": ANALYSIS_VERSION,
            "note": "",
            "added": [],
            "moved": ["analysis_version", "expected.analysis_version"],
        })
        .to_string(),
    )
    .expect("write");
    match plan(&slice.spec(), &outside) {
        Err(Refused::Declaration(why)) => assert!(why.contains("spec/declarations/"), "{why}"),
        other => panic!("expected a refusal to write an unnameable ledger, got {other:?}"),
    }

    let among_vectors = slice.vector("v17.json");
    fs::copy(&outside, &among_vectors).expect("copy");
    match plan(&slice.spec(), &among_vectors) {
        Err(Refused::Declaration(why)) => assert!(why.contains("under spec/vectors/"), "{why}"),
        other => panic!("expected a declaration among the vectors to be refused, got {other:?}"),
    }
}

/// The verb refuses what it does not understand, before it reads anything.
#[test]
fn the_verb_refuses_arguments_it_does_not_understand() {
    for (args, says) in [
        (&["rerecord"][..], "--declare"),
        (&["rerecord", "--declare"][..], "needs a value"),
        (
            &["rerecord", "--declare", "a", "--declare", "b"][..],
            "given twice",
        ),
        (
            &["rerecord", "--declare", "a", "--dry-run", "--dry-run"][..],
            "given twice",
        ),
        (
            &["rerecord", "--declare", "a", "--force"][..],
            "unknown argument",
        ),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_golf-core"))
            .args(args)
            .output()
            .expect("the binary runs");
        let stderr = text(&output.stderr);
        assert!(!output.status.success(), "{args:?} succeeded");
        assert!(
            stderr.contains(says) && stderr.contains("usage:"),
            "{args:?}: {stderr}"
        );
    }
}
