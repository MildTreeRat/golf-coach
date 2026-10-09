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
//!
//! **The screen family's tests copy all of `spec/vectors/screen/`** [M34 P7], because reading a
//! screen costs nothing next to running the engine, and a run over every document is the run M34
//! P10 made. Their real difference is the shape-only declaration (the M34 plan's call 7): the ten
//! keys Rust's `ShotData` gained in M32, and the version. **Since M34 P8 the copy is given this
//! build's answer first**, because from P8 the shipping parser no longer gave frozen Python's (the
//! tie rule, the `Impact Position V` tile, the stamp) while the committed documents kept frozen
//! Python's until P10 re-recorded them. [`Slice::age_the_screen_documents`] answers each copy through
//! [`run_screen`], which is what P10 committed, and then takes the ten keys out and the version
//! back, so the shape-only change is the whole difference on either side of P10, and on a later
//! parser version's. These tests are about the verb; which answers are committed is
//! `crates/screen`'s tests' business.
//!
//! **The storage and career families' tests copy both families whole** [M36 P13], for the screen
//! family's reason: the whole of both runs in well under a second, and a career run is always both.
//! Their real differences are made on the copies, by moving `career_version` back one (the shape
//! of P14's re-record) or by giving a corpus case and its career twin an older sentence than the
//! reader writes, which is how a re-record meets an adopted corpus that moved with its source.

use std::collections::BTreeMap;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use contracts::career::CAREER_VERSION;
use contracts::shot::SCREEN_PARSER_VERSION;
use contracts::swing::ANALYSIS_VERSION;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use golf_core::compare::compare;
use golf_core::rerecord::{
    plan, run_screen, Family, Refused, CAREER, CAREER_DOCUMENTS, SCREEN_DOCUMENTS, STORAGE,
    STORAGE_DOCUMENTS,
};
use screen::ScreenInput;
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

    /// [`Slice::new`], plus the whole committed screen family, every sub-family in it.
    fn with_screen(name: &str) -> Self {
        let slice = Self::new(name);
        copy_tree(
            &committed_spec().join("vectors").join("screen"),
            &slice.vector("screen"),
        );
        slice
    }

    /// [`Slice::new`], plus the whole committed storage and career families. [M36 P13]
    fn with_career(name: &str) -> Self {
        let slice = Self::new(name);
        for family in [STORAGE, CAREER] {
            copy_tree(
                &committed_spec().join("vectors").join(family),
                &slice.vector(family),
            );
        }
        slice
    }

    /// A career declaration, `spec/declarations/{name}.json`: the career families' version key in
    /// place of the engine's. [M36 P13]
    fn declare_career(&self, name: &str, version: i64, moved: &[&str]) -> PathBuf {
        let path = self.declare(name, 0, &[], moved);
        let mut declaration = read(&path);
        let fields = declaration.as_object_mut().expect("a declaration");
        fields.remove("analysis_version");
        fields.insert("career_version".to_string(), json!(version));
        fs::write(&path, declaration.to_string()).expect("write a declaration");
        path
    }

    /// Every storage and career document in the slice, by path: the files a career run reads.
    fn career_documents(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        for (family, halves) in [
            (STORAGE, &STORAGE_DOCUMENTS[..]),
            (CAREER, &CAREER_DOCUMENTS[..]),
        ] {
            for half in halves {
                for entry in fs::read_dir(self.vector(family).join(half)).expect("list") {
                    paths.push(entry.expect("an entry").path());
                }
            }
        }
        paths.sort();
        paths
    }

    /// Edit a committed copy in place and write it back as the committed files are written: CRLF
    /// with a trailing newline when plain, LF inside a gzip. Unlike [`Slice::edit`], so a re-record
    /// of the edited copy is held to the writer's line-ending rule.
    fn edit_like_committed(&self, path: &Path, change: impl FnOnce(&mut Value)) {
        let mut document = read(path);
        change(&mut document);
        let text = serde_json::to_string_pretty(&document).expect("serializes");
        if path.extension().is_some_and(|e| e == "gz") {
            let mut encoder = GzEncoder::new(Vec::new(), Compression::fast());
            encoder.write_all(text.as_bytes()).expect("gzip");
            fs::write(path, encoder.finish().expect("gzip")).expect("write");
        } else {
            fs::write(path, format!("{text}\n").replace('\n', "\r\n")).expect("write");
        }
    }

    /// A screen declaration, `spec/declarations/{name}.json`: the screen family's version key in
    /// place of the engine's.
    fn declare_screen(&self, name: &str, version: i64, added: &[&str], moved: &[&str]) -> PathBuf {
        let path = self.declare(name, 0, added, moved);
        let mut declaration = read(&path);
        let fields = declaration.as_object_mut().expect("a declaration");
        fields.remove("analysis_version");
        fields.insert("screen_parser_version".to_string(), json!(version));
        fs::write(&path, declaration.to_string()).expect("write a declaration");
        path
    }

    /// Every screen document in the slice, by path: the files a screen run reads.
    fn screen_documents(&self) -> Vec<PathBuf> {
        let mut paths: Vec<PathBuf> = SCREEN_DOCUMENTS
            .iter()
            .flat_map(|half| {
                fs::read_dir(self.vector("screen").join(half))
                    .expect("list")
                    .map(|entry| entry.expect("an entry").path())
            })
            .collect();
        paths.sort();
        paths
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

    /// Every screen document answered by this build and then put back to frozen Python's shape:
    /// `expected` is [`run_screen`]'s, one parser version back, without `keys` in its shot, and with
    /// no ledger. The shape-only change is then the whole difference between a copy and this build,
    /// whether or not the committed documents have been re-recorded (the module doc says why the
    /// answer is this build's since M34 P8). Written CRLF, as the committed files are, so the
    /// writer's line-ending rule stays under test.
    fn age_the_screen_documents(&self, keys: &[String]) {
        for path in self.screen_documents() {
            let mut document = read(&path);
            let input: ScreenInput = serde_json::from_value(document["input"].clone())
                .unwrap_or_else(|e| panic!("{path:?}: input: {e}"));
            document["expected"] = run_screen(&input).unwrap_or_else(|e| panic!("{path:?}: {e}"));
            document["screen_parser_version"] = json!(SCREEN_PARSER_VERSION - 1);
            if let Some(provenance) = document["provenance"].as_object_mut() {
                provenance.remove("rerecords");
            }
            if document["expected"]["shot"].is_object() {
                for key in keys {
                    let (parent, last) = key.rsplit_once('.').expect("a nested key");
                    if let Some(parent) = document
                        .pointer_mut(&pointer(parent))
                        .and_then(Value::as_object_mut)
                    {
                        parent.remove(last);
                    }
                }
            }
            let text = serde_json::to_string_pretty(&document).expect("serializes");
            fs::write(&path, format!("{text}\n").replace('\n', "\r\n")).expect("write");
        }
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

fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).expect("mkdir");
    for entry in fs::read_dir(from).unwrap_or_else(|e| panic!("list {from:?}: {e}")) {
        let path = entry.expect("an entry").path();
        let target = to.join(path.file_name().expect("a name"));
        if path.is_dir() {
            copy_tree(&path, &target);
        } else {
            fs::copy(&path, &target).unwrap_or_else(|e| panic!("copy {path:?}: {e}"));
        }
    }
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

// ------------------------------------------------------------------ the screen family [M34 P7]

/// `spec/declarations/v17.json`'s `added`, re-rooted from the engine vectors' shot to a screen
/// document's: the ten keys Rust's `ShotData` has and frozen Python's records lack (the M34 plan's
/// call 7), spelt as `spec/declarations/screen-v1.json` added them, and read from `v17.json` rather
/// than written out, so this test and M32's declaration cannot spell the ten differently.
fn m32_shot_keys() -> Vec<String> {
    let v17 = read(&committed_spec().join("declarations").join("v17.json"));
    let keys: Vec<String> = v17["added"]
        .as_array()
        .expect("v17 lists what it added")
        .iter()
        .map(|added| {
            let added = added.as_str().expect("a declared path is a string");
            let key = added
                .strip_prefix("expected.swing.shot.")
                .unwrap_or_else(|| panic!("v17 added {added:?}, which is not on the shot"));
            format!("expected.shot.{key}")
        })
        .collect();
    assert!(!keys.is_empty(), "v17.json adds no shot key");
    keys
}

/// The shape-only screen declaration: M32's keys added, and the version moved from frozen Python's
/// unversioned 0 to this build's — the move every re-record makes, as M32's moved
/// `analysis_version`. A real difference on a slice after [`Slice::age_the_screen_documents`].
fn declare_shape_only(slice: &Slice, name: &str, keys: &[String]) -> PathBuf {
    let keys: Vec<&str> = keys.iter().map(String::as_str).collect();
    slice.declare_screen(
        name,
        SCREEN_PARSER_VERSION,
        &keys,
        &["screen_parser_version"],
    )
}

/// `/a/b` for `a.b`, for the keys here, none of which holds a `.`, `/` or `~`.
fn pointer(path: &str) -> String {
    format!("/{}", path.replace('.', "/"))
}

/// **The screen family re-records behind M32's gate**: an undeclared key is refused everywhere at
/// once and nothing is written; the shape-only declaration then lands exactly its keys and the
/// version on every document, each ledgered under `screen_parser_version`, every other value and
/// every file outside the screen documents left byte for byte; and a second run writes nothing.
#[test]
fn a_screen_shape_only_declaration_lands_exactly_its_keys_and_a_second_run_writes_nothing() {
    let slice = Slice::with_screen("screen-shape");
    let keys = m32_shot_keys();
    slice.age_the_screen_documents(&keys);
    let documents = slice.screen_documents();
    let before = slice.snapshot();

    // One key left out: refused on every document that has a shot, and nowhere written.
    let short = declare_shape_only(&slice, "screen-short", &keys[..keys.len() - 1]);
    let left_out = &keys[keys.len() - 1];
    let with_a_shot = documents
        .iter()
        .filter(|path| !read(path)["expected"]["shot"].is_null())
        .count();
    assert!(
        with_a_shot > 0 && with_a_shot < documents.len(),
        "the family should hold shots and at least one failed read"
    );
    match plan(&slice.spec(), &short) {
        Err(Refused::Gate(refusals)) => {
            assert_eq!(refusals.len(), with_a_shot, "{refusals:#?}");
            assert!(
                refusals
                    .iter()
                    .all(|r| r.starts_with("screen/") && r.contains(left_out.as_str())),
                "{refusals:#?}"
            );
        }
        other => panic!("expected the gate to refuse, got {other:?}"),
    }
    fs::remove_file(&short).expect("remove the short declaration");
    assert_eq!(slice.snapshot(), before, "a refused run wrote something");

    let declaration = declare_shape_only(&slice, "screen-shape", &keys);
    let run = plan(&slice.spec(), &declaration).expect("the shape-only change is declared");
    assert_eq!(run.vectors().len(), documents.len());
    let committed: BTreeMap<PathBuf, Value> = documents
        .iter()
        .map(|path| (path.clone(), read(path)))
        .collect();
    for vector in run.vectors() {
        assert_eq!(vector.family(), Family::Screen, "{}", vector.id());
        let added: Vec<&str> = vector
            .applied()
            .added()
            .iter()
            .map(|p| p.as_str())
            .collect();
        let moved: Vec<&str> = vector
            .applied()
            .moved()
            .iter()
            .map(|p| p.as_str())
            .collect();
        let had_a_shot = !committed[vector.path()]["expected"]["shot"].is_null();
        let want: Vec<&str> = if had_a_shot {
            keys.iter().map(String::as_str).collect()
        } else {
            Vec::new()
        };
        assert_eq!(added, want, "{}", vector.id());
        assert_eq!(moved, ["screen_parser_version"], "{}", vector.id());
    }
    assert_eq!(run.write().expect("writes"), documents.len());

    // Outside the screen documents nothing moved: the engine and stage slice, `units/`, and the
    // declarations directory are the bytes they were, and no staged file was left behind.
    let after = slice.snapshot();
    assert_eq!(
        after
            .keys()
            .filter(|path| !path.ends_with("screen-shape.json"))
            .collect::<Vec<_>>(),
        before.keys().collect::<Vec<_>>(),
        "the run added or removed a file"
    );
    for (path, bytes) in &before {
        if !documents.contains(path) {
            assert_eq!(
                &after[path], bytes,
                "{path:?} is not a screen document and changed"
            );
        }
    }

    for vector in run.changed() {
        let path = vector.path();
        let written = read(path);
        assert_eq!(
            &written,
            vector.written().expect("planned"),
            "{}",
            vector.id()
        );
        let bytes = &after[path];
        assert!(
            bytes.ends_with(b"}\r\n") && !text(bytes).replace("\r\n", "").contains('\n'),
            "{}: the committed CRLF was not kept",
            vector.id()
        );

        // Each ledger lists what matched in that file, under the screen family's version key.
        let entries = written["provenance"]["rerecords"]
            .as_array()
            .expect("a ledger");
        assert_eq!(entries.len(), 1, "{}", vector.id());
        let had_a_shot = !committed[path]["expected"]["shot"].is_null();
        assert_eq!(
            entries[0],
            json!({
                "screen_parser_version": SCREEN_PARSER_VERSION,
                "by": "golf-core rerecord",
                "declaration": "spec/declarations/screen-shape.json",
                "added": if had_a_shot { json!(keys) } else { json!([]) },
                "moved": ["screen_parser_version"],
                "removed": [],
            }),
            "{}",
            vector.id()
        );
        assert_eq!(
            written["provenance"]["oracle"],
            committed[path]["provenance"]["oracle"],
            "{}: the re-record keeps the oracle it found",
            vector.id()
        );

        // Undo the declared paths and the ledger, and what is left is the document as aged.
        let mut undone = written.clone();
        undone["provenance"] = committed[path]["provenance"].clone();
        undone["screen_parser_version"] = committed[path]["screen_parser_version"].clone();
        if had_a_shot {
            for key in &keys {
                let (parent, last) = key.rsplit_once('.').expect("a nested key");
                undone
                    .pointer_mut(&pointer(parent))
                    .and_then(Value::as_object_mut)
                    .expect("the key's parent")
                    .remove(last)
                    .unwrap_or_else(|| panic!("{}: {key} did not land", vector.id()));
            }
        }
        assert_eq!(
            undone,
            committed[path],
            "{}: an undeclared value moved",
            vector.id()
        );
    }

    // The second run: nothing differs, every declared path is in a ledger, nothing is written.
    let second = plan(&slice.spec(), &declaration).expect("a second run is a no-op, not a refusal");
    assert_eq!(second.changed().count(), 0);
    assert_eq!(second.write().expect("writes nothing"), 0);
    assert_eq!(slice.snapshot(), after, "a second run wrote something");
}

/// **A screen declaration's `removed` lets exactly the named key go, in the one document that has
/// it** (M34 P10, the user's answer to the plan's P7 finding 3). One aged copy is given a
/// `raw_fields` key this build's answer lacks — the shape the tie rule's withheld tiles take. Without
/// the removal declared the run refuses on that key alone; with it, the key is gone from that file
/// and only that file's ledger lists it, every other ledger says `removed: []`, and a second run is
/// a no-op that the typo guard accepts from the ledger.
#[test]
fn a_screen_removal_lands_where_it_is_named_and_nowhere_else() {
    let slice = Slice::with_screen("screen-removed");
    let keys = m32_shot_keys();
    slice.age_the_screen_documents(&keys);
    let stray = "expected.parsed.raw_fields.Impact Position";
    let target = slice
        .screen_documents()
        .into_iter()
        .find(|path| {
            let document = read(path);
            document["expected"]["shot"].is_object()
                && document["expected"]["parsed"]["raw_fields"]
                    .get("Impact Position")
                    .is_none()
        })
        .expect("a document whose answer has no `Impact Position` raw field");
    let mut document = read(&target);
    document["expected"]["parsed"]["raw_fields"]["Impact Position"] = json!("HEEL");
    let text = serde_json::to_string_pretty(&document).expect("serializes");
    fs::write(&target, format!("{text}\n").replace('\n', "\r\n")).expect("write");
    let target_id = document["id"].as_str().expect("an id").to_string();

    let unnamed = declare_shape_only(&slice, "screen-unnamed", &keys);
    match plan(&slice.spec(), &unnamed) {
        Err(Refused::Gate(refusals)) => {
            assert_eq!(refusals.len(), 1, "{refusals:#?}");
            assert!(
                refusals[0].starts_with(&format!("{target_id}: {stray}"))
                    && refusals[0].contains("`removed`"),
                "{}",
                refusals[0]
            );
        }
        other => panic!("expected the gate to refuse the unnamed removal, got {other:?}"),
    }
    fs::remove_file(&unnamed).expect("remove the declaration");

    let declaration = declare_shape_only(&slice, "screen-removing", &keys);
    let mut file = read(&declaration);
    file["removed"] = json!([stray]);
    fs::write(&declaration, file.to_string()).expect("write a declaration");

    let run = plan(&slice.spec(), &declaration).expect("the removal is named");
    run.write().expect("writes");
    for path in slice.screen_documents() {
        let written = read(&path);
        let entry = &written["provenance"]["rerecords"][0];
        if path == target {
            assert!(
                written["expected"]["parsed"]["raw_fields"]
                    .get("Impact Position")
                    .is_none(),
                "the named key is still there"
            );
            assert_eq!(entry["removed"], json!([stray]), "{target_id}");
        } else {
            assert_eq!(entry["removed"], json!([]), "{path:?}");
        }
    }

    let after = slice.snapshot();
    let second = plan(&slice.spec(), &declaration).expect("a second run is a no-op, not a refusal");
    assert_eq!(second.changed().count(), 0);
    assert_eq!(second.write().expect("writes nothing"), 0);
    assert_eq!(slice.snapshot(), after, "a second run wrote something");
}

/// **A run reads its own family and nothing else** (call 6). With every engine and stage vector in
/// the slice made unreadable, a screen run still passes, so it never opened one; with a screen
/// document unreadable and a sub-family nobody placed beside it, an engine run passes and writes
/// its own files alone.
#[test]
fn a_screen_run_reads_no_engine_vector_and_an_engine_run_reads_no_screen_vector() {
    let unreadable = b"not a vector";

    let slice = Slice::with_screen("screen-only");
    for relative in [SYNTHETIC, CORPUS] {
        for family in ["", "stages/"] {
            fs::write(slice.vector(&format!("{family}{relative}")), unreadable).expect("write");
        }
    }
    let keys = m32_shot_keys();
    slice.age_the_screen_documents(&keys);
    let declaration = declare_shape_only(&slice, "screen-shape", &keys);
    let run = plan(&slice.spec(), &declaration).expect("a screen run never reads an engine vector");
    assert!(run.vectors().iter().all(|v| v.family() == Family::Screen));
    assert_eq!(run.changed().count(), slice.screen_documents().len());

    let slice = Slice::with_screen("engine-only");
    slice.age_the_synthetic_pair();
    let first = slice.screen_documents()[0].clone();
    fs::write(&first, unreadable).expect("write");
    fs::create_dir_all(slice.vector("screen/unplaced")).expect("mkdir");
    let screen_before: BTreeMap<PathBuf, Vec<u8>> = slice
        .snapshot()
        .into_iter()
        .filter(|(path, _)| path.starts_with(slice.vector("screen")))
        .collect();
    let declaration = slice.declare(
        "versions",
        ANALYSIS_VERSION,
        &[],
        &["analysis_version", "expected.analysis_version"],
    );
    let run = plan(&slice.spec(), &declaration).expect("an engine run never reads a screen vector");
    assert!(run.vectors().iter().all(|v| v.family() != Family::Screen));
    assert_eq!(run.write().expect("writes"), 2);
    let screen_after: BTreeMap<PathBuf, Vec<u8>> = slice
        .snapshot()
        .into_iter()
        .filter(|(path, _)| path.starts_with(slice.vector("screen")))
        .collect();
    assert_eq!(
        screen_after, screen_before,
        "an engine run wrote a screen file"
    );
}

/// The version guard reads the screen family's own constant, and before a vector is read: the spec
/// here does not exist.
#[test]
fn a_screen_declaration_at_another_version_is_refused_before_anything_is_read() {
    let slice = Slice::new("screen-version");
    let declaration = slice.declare_screen("screen-next", SCREEN_PARSER_VERSION + 1, &[], &[]);
    match plan(&slice.root.join("no-such-spec"), &declaration) {
        Err(Refused::Declaration(why)) => {
            assert!(why.contains("call 6"), "{why}");
            assert!(
                why.contains(&format!("this screen parser is at {SCREEN_PARSER_VERSION}")),
                "{why}"
            );
        }
        other => panic!("expected the version guard, got {other:?}"),
    }
}

/// A directory under `vectors/screen/` that is neither re-recorded nor deliberately unread stops
/// the walk, and so does a loose file: a sub-family is placed in one list or the other on purpose,
/// never skipped because nobody told the re-record about it.
#[test]
fn a_screen_sub_family_nobody_placed_stops_the_walk() {
    for (name, make) in [
        ("screen-unplaced-dir", "unplaced/"),
        ("screen-unplaced-file", "README.json"),
    ] {
        let slice = Slice::with_screen(name);
        let target = slice.vector("screen").join(make.trim_end_matches('/'));
        if make.ends_with('/') {
            fs::create_dir_all(&target).expect("mkdir");
        } else {
            fs::write(&target, "{}").expect("write");
        }
        let declaration = declare_shape_only(&slice, "screen-shape", &m32_shot_keys());
        let before = slice.snapshot();
        match plan(&slice.spec(), &declaration) {
            Err(Refused::Files(why)) => {
                assert!(
                    why.contains(make.trim_end_matches('/')) && why.contains("SCREEN_DOCUMENTS"),
                    "{why}"
                );
            }
            other => panic!("{make}: expected the walk to stop, got {other:?}"),
        }
        assert_eq!(slice.snapshot(), before);
    }
}

// --------------------------------------------------- the storage and career families [M36 P13]

const OUTDATED_CASE: &str = "storage/corpus/stale-and-outdated.json";
const OUTDATED_TWIN: &str = "career/synthetic/storage-stale-and-outdated.json";
const OUTDATED_TWIN_ID: &str = "career/synthetic/storage-stale-and-outdated";
/// The outdated swing's exclusion in the case's answer, and the same entry in its twin's copy.
const OUTDATED_DETAIL: &str = "expected.corpus.excluded[1].detail";
const ADOPTED_DETAIL: &str = "input.corpus.excluded[1].detail";
const OLDER_SENTENCE: &str = "an older sentence than the reader writes";

/// How many storage and how many career documents the slice holds, for the report's count line.
fn career_counts(slice: &Slice) -> (usize, usize) {
    let documents = slice.career_documents();
    let storage = slice.vector(STORAGE);
    let in_storage = documents.iter().filter(|p| p.starts_with(&storage)).count();
    (in_storage, documents.len() - in_storage)
}

/// **An empty declaration at this build's career version finds nothing and writes nothing**, dry
/// or not: the committed families are what this build answers, the op cases' written files
/// included, whose committed spelling the runner keeps wherever the values agree.
#[test]
fn an_empty_career_declaration_finds_nothing_and_writes_nothing() {
    let slice = Slice::with_career("career-empty");
    let declaration = slice.declare_career("career-empty", CAREER_VERSION, &[]);
    let (storage, career) = career_counts(&slice);
    assert!(storage > 0 && career > 0);
    let before = slice.snapshot();

    for extra in [&["--dry-run"][..], &[][..]] {
        let output = slice.verb(&declaration, extra);
        let stdout = text(&output.stdout);
        assert!(output.status.success(), "{}", text(&output.stderr));
        let count = format!(
            "{} vectors run ({storage} storage, {career} career); 0 changed; 0 files written",
            storage + career
        );
        for expected in [
            format!("a declaration at career_version {CAREER_VERSION}, 0 added and 0 moved"),
            "nothing differs from the committed vectors".to_string(),
            count,
        ] {
            assert!(
                stdout.contains(&expected),
                "{extra:?}: missing {expected:?} in:\n{stdout}"
            );
        }
        assert_eq!(slice.snapshot(), before, "{extra:?} wrote something");
    }
}

/// The version guard reads the career families' own constant, and before a vector is read: the spec
/// here does not exist.
#[test]
fn a_career_declaration_at_another_version_is_refused_before_anything_is_read() {
    let slice = Slice::new("career-version");
    let declaration = slice.declare_career("career-next", CAREER_VERSION + 1, &[]);
    match plan(&slice.root.join("no-such-spec"), &declaration) {
        Err(Refused::Declaration(why)) => {
            assert!(why.contains("call 6"), "{why}");
            assert!(
                why.contains(&format!("this many-shot layer is at {CAREER_VERSION}")),
                "{why}"
            );
        }
        other => panic!("expected the version guard, got {other:?}"),
    }
}

/// **A career re-record moves both families behind the gate**: with every copy recorded one
/// version back, an empty declaration is refused on every document at once and writes nothing; the
/// version declared, it lands on every storage and career document alone, each ledgered under
/// `career_version` with no `removed` list, every other value and every other family's file left
/// byte for byte, the committed line endings kept; and a second run writes nothing. P14's re-record
/// is this move plus the sentences it declares.
///
/// Aging a copy also drops its ledger, as `age_the_screen_documents` does, so each file's entry is
/// the only one: since `career-v1.json` ran (M36 P14) the committed vectors carry its entry, and a
/// run appends to a ledger rather than replacing it.
#[test]
fn a_career_version_move_lands_on_both_families_and_a_second_run_writes_nothing() {
    let slice = Slice::with_career("career-move");
    let documents = slice.career_documents();
    for path in &documents {
        slice.edit_like_committed(path, |v| {
            v["career_version"] = json!(CAREER_VERSION - 1);
            if let Some(provenance) = v["provenance"].as_object_mut() {
                provenance.remove("rerecords");
            }
        });
    }
    let before = slice.snapshot();

    let empty = slice.declare_career("career-empty", CAREER_VERSION, &[]);
    match plan(&slice.spec(), &empty) {
        Err(Refused::Gate(refusals)) => {
            assert_eq!(refusals.len(), documents.len(), "{refusals:#?}");
            assert!(
                refusals
                    .iter()
                    .all(|r| r.contains(": career_version: ") && r.contains("`moved`")),
                "{refusals:#?}"
            );
        }
        other => panic!("expected the gate to refuse, got {other:?}"),
    }
    fs::remove_file(&empty).expect("remove the empty declaration");
    assert_eq!(slice.snapshot(), before, "a refused run wrote something");

    let declaration = slice.declare_career("career-move", CAREER_VERSION, &["career_version"]);
    let run = plan(&slice.spec(), &declaration).expect("the version move is declared");
    let families: Vec<Family> = run.vectors().iter().map(|v| v.family()).collect();
    let storage = families.iter().filter(|f| **f == Family::Storage).count();
    assert!(storage > 0 && storage < families.len());
    assert!(
        families[..storage].iter().all(|f| *f == Family::Storage)
            && families[storage..].iter().all(|f| *f == Family::Career),
        "the storage family first, then the career family"
    );
    for vector in run.vectors() {
        let moved: Vec<&str> = vector
            .applied()
            .moved()
            .iter()
            .map(|p| p.as_str())
            .collect();
        assert_eq!(moved, ["career_version"], "{}", vector.id());
        assert!(vector.applied().added().is_empty(), "{}", vector.id());
    }
    let aged: BTreeMap<PathBuf, Value> = documents.iter().map(|p| (p.clone(), read(p))).collect();
    assert_eq!(run.write().expect("writes"), documents.len());

    let after = slice.snapshot();
    for (path, bytes) in &before {
        if !documents.contains(path) {
            assert_eq!(
                &after[path], bytes,
                "{path:?} is not a career document and changed"
            );
        }
    }
    for path in &documents {
        let written = read(path);
        let bytes = &after[path];
        if path.extension().is_some_and(|e| e == "gz") {
            assert!(
                !text(&decompressed(bytes)).contains('\r'),
                "{path:?}: CRLF in a gzip"
            );
        } else {
            assert!(
                bytes.ends_with(b"}\r\n") && !text(bytes).replace("\r\n", "").contains('\n'),
                "{path:?}: the committed CRLF was not kept"
            );
        }
        assert_eq!(
            written["provenance"]["rerecords"],
            json!([{
                "career_version": CAREER_VERSION,
                "by": "golf-core rerecord",
                "declaration": "spec/declarations/career-move.json",
                "added": [],
                "moved": ["career_version"],
            }]),
            "{path:?}"
        );
        let mut undone = written.clone();
        undone["provenance"] = aged[path]["provenance"].clone();
        undone["career_version"] = aged[path]["career_version"].clone();
        assert_eq!(undone, aged[path], "{path:?}: an undeclared value moved");
    }

    let second = plan(&slice.spec(), &declaration).expect("a second run is a no-op, not a refusal");
    assert_eq!(second.changed().count(), 0);
    assert_eq!(second.write().expect("writes nothing"), 0);
    assert_eq!(slice.snapshot(), after, "a second run wrote something");
}

fn decompressed(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    GzDecoder::new(bytes).read_to_end(&mut out).expect("a gzip");
    out
}

/// **An adopted corpus moves with its storage vector, and only where the declaration says so.** A
/// corpus case and its career twin are both given an older exclusion sentence than the reader
/// writes, as two vectors recorded under an older rule would hold. Declaring the case's sentence
/// alone is refused on the twin's copy, which the run derived from the case as it will write it;
/// declaring both lands both, and the twin's aggregates, which never read the sentence, stay as
/// they were. And a twin whose copy drifted from an unchanged case is refused by an empty
/// declaration: the two families cannot come apart silently.
#[test]
fn an_adopted_corpus_moves_with_its_storage_vector_and_only_where_declared() {
    let slice = Slice::with_career("career-adopted");
    let (case, twin) = (slice.vector(OUTDATED_CASE), slice.vector(OUTDATED_TWIN));
    let reader_writes = read(&case)["expected"]["corpus"]["excluded"][1]["detail"].clone();
    assert!(
        reader_writes
            .as_str()
            .is_some_and(|s| s.contains("engine version")),
        "the case's second exclusion is the outdated swing: {reader_writes}"
    );
    slice.edit_like_committed(&case, |v| {
        v["expected"]["corpus"]["excluded"][1]["detail"] = json!(OLDER_SENTENCE);
    });
    slice.edit_like_committed(&twin, |v| {
        v["input"]["corpus"]["excluded"][1]["detail"] = json!(OLDER_SENTENCE);
    });
    let before = slice.snapshot();

    let half = slice.declare_career("career-half", CAREER_VERSION, &[OUTDATED_DETAIL]);
    match plan(&slice.spec(), &half) {
        Err(Refused::Gate(refusals)) => {
            assert_eq!(refusals.len(), 1, "{refusals:#?}");
            assert!(
                refusals[0].starts_with(&format!("{OUTDATED_TWIN_ID}: {ADOPTED_DETAIL}: ")),
                "{}",
                refusals[0]
            );
        }
        other => panic!("expected the twin's copy to be refused, got {other:?}"),
    }
    fs::remove_file(&half).expect("remove the half declaration");
    assert_eq!(slice.snapshot(), before, "a refused run wrote something");

    let both = slice.declare_career(
        "career-both",
        CAREER_VERSION,
        &[OUTDATED_DETAIL, ADOPTED_DETAIL],
    );
    let twin_expected = read(&twin)["expected"].clone();
    let run = plan(&slice.spec(), &both).expect("both are declared");
    let changed: Vec<(&str, Vec<&str>)> = run
        .changed()
        .map(|v| {
            let moved = v.applied().moved().iter().map(|p| p.as_str()).collect();
            (v.id(), moved)
        })
        .collect();
    assert_eq!(
        changed,
        [
            ("storage/corpus/stale-and-outdated", vec![OUTDATED_DETAIL]),
            (OUTDATED_TWIN_ID, vec![ADOPTED_DETAIL]),
        ]
    );
    assert_eq!(run.write().expect("writes"), 2);
    assert_eq!(
        read(&case)["expected"]["corpus"]["excluded"][1]["detail"],
        reader_writes
    );
    let written_twin = read(&twin);
    assert_eq!(
        written_twin["input"]["corpus"],
        read(&case)["expected"]["corpus"],
        "the twin's copy is its case's answer again"
    );
    assert_eq!(
        written_twin["expected"], twin_expected,
        "the twin's answer moved"
    );

    let after = slice.snapshot();
    let second = plan(&slice.spec(), &both).expect("a second run is a no-op, not a refusal");
    assert_eq!(second.changed().count(), 0);
    assert_eq!(slice.snapshot(), after);

    let slice = Slice::with_career("career-drifted");
    let twin = slice.vector(OUTDATED_TWIN);
    slice.edit_like_committed(&twin, |v| {
        v["input"]["corpus"]["excluded"][1]["detail"] = json!(OLDER_SENTENCE);
    });
    let empty = slice.declare_career("career-empty", CAREER_VERSION, &[]);
    match plan(&slice.spec(), &empty) {
        Err(Refused::Gate(refusals)) => {
            assert_eq!(refusals.len(), 1, "{refusals:#?}");
            assert!(
                refusals[0].starts_with(&format!("{OUTDATED_TWIN_ID}: {ADOPTED_DETAIL}: ")),
                "{}",
                refusals[0]
            );
        }
        other => panic!("expected the drifted copy to be refused, got {other:?}"),
    }
}

/// **A career run reads its own families and nothing else**, and neither of the others reads one of
/// its files. With every engine, stage and screen document unreadable, a career run still passes;
/// with every storage and career document unreadable and a sub-family nobody placed beside them, an
/// engine run and a screen run pass and write their own files alone.
#[test]
fn a_career_run_reads_no_engine_or_screen_vector_and_neither_reads_a_career_one() {
    let unreadable = b"not a vector";

    let slice = Slice::with_screen("career-only");
    for family in [STORAGE, CAREER] {
        copy_tree(
            &committed_spec().join("vectors").join(family),
            &slice.vector(family),
        );
    }
    for path in slice.snapshot().into_keys() {
        let career = [STORAGE, CAREER]
            .iter()
            .any(|family| path.starts_with(slice.vector(family)));
        if !career && path.starts_with(slice.spec().join("vectors")) {
            fs::write(&path, unreadable).expect("write");
        }
    }
    let declaration = slice.declare_career("career-empty", CAREER_VERSION, &[]);
    let run = plan(&slice.spec(), &declaration).expect("a career run never reads another family");
    assert!(run
        .vectors()
        .iter()
        .all(|v| matches!(v.family(), Family::Storage | Family::Career)));
    assert_eq!(run.vectors().len(), slice.career_documents().len());

    let slice = Slice::with_screen("not-career");
    for family in [STORAGE, CAREER] {
        copy_tree(
            &committed_spec().join("vectors").join(family),
            &slice.vector(family),
        );
    }
    for path in slice.career_documents() {
        fs::write(&path, unreadable).expect("write");
    }
    fs::create_dir_all(slice.vector("storage/unplaced")).expect("mkdir");
    slice.age_the_synthetic_pair();
    let keys = m32_shot_keys();
    slice.age_the_screen_documents(&keys);
    let career_before: BTreeMap<PathBuf, Vec<u8>> = slice
        .snapshot()
        .into_iter()
        .filter(|(path, _)| {
            [STORAGE, CAREER]
                .iter()
                .any(|family| path.starts_with(slice.vector(family)))
        })
        .collect();

    let engine = slice.declare(
        "versions",
        ANALYSIS_VERSION,
        &[],
        &["analysis_version", "expected.analysis_version"],
    );
    let run = plan(&slice.spec(), &engine).expect("an engine run never reads a career vector");
    assert_eq!(run.write().expect("writes"), 2);
    let screen = declare_shape_only(&slice, "screen-shape", &keys);
    let run = plan(&slice.spec(), &screen).expect("a screen run never reads a career vector");
    assert!(run.vectors().iter().all(|v| v.family() == Family::Screen));
    run.write().expect("writes");

    let career_after: BTreeMap<PathBuf, Vec<u8>> = slice
        .snapshot()
        .into_iter()
        .filter(|(path, _)| {
            [STORAGE, CAREER]
                .iter()
                .any(|family| path.starts_with(slice.vector(family)))
        })
        .collect();
    assert_eq!(
        career_after, career_before,
        "another family's run wrote a career file"
    );
}

/// A directory under `vectors/storage/` or `vectors/career/` that the re-record does not place stops
/// the walk, and so does a loose file; and a storage vector naming a runner the family does not
/// have stops it too, rather than panicking.
#[test]
fn a_career_sub_family_nobody_placed_or_a_runner_nobody_has_stops_the_walk() {
    for (name, make, list) in [
        (
            "career-unplaced-dir",
            "storage/unplaced/",
            "STORAGE_DOCUMENTS",
        ),
        (
            "career-unplaced-file",
            "career/README.json",
            "CAREER_DOCUMENTS",
        ),
    ] {
        let slice = Slice::with_career(name);
        let target = slice.vector(make.trim_end_matches('/'));
        if make.ends_with('/') {
            fs::create_dir_all(&target).expect("mkdir");
        } else {
            fs::write(&target, "{}").expect("write");
        }
        let declaration = slice.declare_career("career-empty", CAREER_VERSION, &[]);
        let before = slice.snapshot();
        match plan(&slice.spec(), &declaration) {
            Err(Refused::Files(why)) => {
                let leaf = make
                    .trim_end_matches('/')
                    .rsplit('/')
                    .next()
                    .expect("a name");
                assert!(why.contains(leaf) && why.contains(list), "{why}");
            }
            other => panic!("{make}: expected the walk to stop, got {other:?}"),
        }
        assert_eq!(slice.snapshot(), before);
    }

    let slice = Slice::with_career("career-unknown-runner");
    slice.edit_like_committed(&slice.vector(OUTDATED_CASE), |v| {
        v["provenance"]["recorded_by"] = json!("by hand");
    });
    let declaration = slice.declare_career("career-empty", CAREER_VERSION, &[]);
    match plan(&slice.spec(), &declaration) {
        Err(Refused::Files(why)) => {
            assert!(
                why.contains("a runner the storage family does not have"),
                "{why}"
            );
        }
        other => panic!("expected the walk to stop, got {other:?}"),
    }
}
