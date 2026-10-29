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
//! **Since M35 P6 the slice also holds the corpus vector's shot case**, `shot/corpus-2026-08-23-11`,
//! because an engine declaration re-records the shot family too and a run over a `spec/` with no
//! `vectors/shot/` is refused. Its real differences are made on the copy as the engine pair's are,
//! and written back in the case's own layout (LF, an indent of 4), so the writer's rule that a file
//! keeps its layout stays under test on the one family that is not `_write_json`'s.
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
//! **Since M37 P11 they copy the profile family too**, because a career declaration re-records it
//! after the career family, and a run over a `spec/` with no `vectors/profile/` is refused. Its
//! corpus cases adopt career cases' inputs, so an older value is carried one link further.
//!
//! **Since M29 P15 they copy the mcp family too**, the fourth a career declaration re-records, and
//! its walk runs the `golf-mcp` binary (`golf_core::rerecord::golf_mcp_binary`): beside
//! `golf-core`, or one up from cargo's `deps/` for the runs here that call [`plan`] in process. A
//! workspace `cargo test` builds it before any test runs; `cargo test -p golf-core` alone does not,
//! and these tests then fail on the walk's refusal naming the build, never pass without the family.
//!
//! **Since M29 P19 every slice also holds two selection cases and one mcp case**, because an engine
//! declaration re-records the selection family after the shot family and then the mcp family, and a
//! run over a `spec/` with either directory missing is refused. The two selection cases are a rules
//! case and a windows case, whose `_auto_windows` answer is built from the pick ahead of it (until
//! M29 P25 it was carried as committed); the mcp case is one with flight-model calls, the reason an
//! engine run walks that family.
//! The sub-families they leave empty are created empty, so the walks find every directory they
//! name. So **every engine run here spawns `golf-mcp` too**, and `cargo test -p golf-core` alone
//! fails them on the refusal, as it fails the career runs.

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
    plan, run_screen, Family, Refused, CAREER, CAREER_DOCUMENTS, GOLF_MCP_ENV, MCP, MCP_DOCUMENTS,
    PROFILE, PROFILE_DOCUMENTS, SCREEN_DOCUMENTS, SELECTION, SELECTION_DOCUMENTS, STORAGE,
    STORAGE_DOCUMENTS,
};
use screen::ScreenInput;
use serde_json::{json, Value};

const SYNTHETIC: &str = "synthetic/tempo-too-quick.json";
const CORPUS: &str = "corpus/2026-08-23-11.json.gz";
const SYNTHETIC_ID: &str = "synthetic/tempo-too-quick";
const CORPUS_ID: &str = "corpus/2026-08-23-11";

/// The corpus vector's shot case, the shot family's one file in every slice. [M35 P6]
const SHOT_CASE: &str = "shot/corpus-2026-08-23-11.json";
const SHOT_ID: &str = "shot/corpus-2026-08-23-11";

/// The shot case's first row, the path the shot tests move on the copy: its own spelling, which no
/// engine or stage document has, so a declaration naming it can match in the shot family alone.
const SHOT_ROW: &str = "expected.measurements[0].value";

/// The selection cases in every slice: a rules case, and a windows case whose second call,
/// `_auto_windows`, is built from its first, a `_pick_swing`. Every call in both is answered since
/// M29 P25, which ported `_auto_windows`. [M29 P19]
const RULES_CASE: &str = "selection/rules/fast-downswing.json.gz";
const RULES_ID: &str = "selection/rules/fast-downswing";
const WINDOWS_CASE: &str = "selection/windows/face-on-only.json.gz";
const WINDOWS_ID: &str = "selection/windows/face-on-only";

/// The first call's reason in either selection case: a `select_swing` answer in the rules case, a
/// `_pick_swing` answer in the windows case, which that case's `_auto_windows` is built from.
const FIRST_REASON: &str = "expected.results[0].reason";

/// The mcp case in every slice: one whose calls reach the flight model, the reason an engine run
/// walks the mcp family. [M29 P19]
const MCP_CASE: &str = "mcp/tools/flight-registry.json";

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
        let shot = root
            .join("repo")
            .join("spec")
            .join("vectors")
            .join(SHOT_CASE);
        fs::create_dir_all(shot.parent().expect("a parent")).expect("mkdir");
        fs::copy(committed_spec().join("vectors").join(SHOT_CASE), &shot)
            .unwrap_or_else(|e| panic!("copy {SHOT_CASE}: {e}"));
        fs::create_dir_all(root.join("repo").join("spec").join("declarations")).expect("mkdir");
        let slice = Self { root };
        // Every sub-family an engine run walks after the shot family, each present, two of them
        // holding a case and the rest empty. [M29 P19]
        for (family, halves) in [
            (SELECTION, &SELECTION_DOCUMENTS[..]),
            (MCP, &MCP_DOCUMENTS[..]),
        ] {
            for half in halves {
                fs::create_dir_all(slice.vector(family).join(half)).expect("mkdir");
            }
        }
        for relative in [RULES_CASE, WINDOWS_CASE, MCP_CASE] {
            fs::copy(
                committed_spec().join("vectors").join(relative),
                slice.vector(relative),
            )
            .unwrap_or_else(|e| panic!("copy {relative}: {e}"));
        }
        slice
    }

    /// Edit the shot case's copy in place and write it back as the case was committed: LF, an
    /// indent of 4, a trailing newline (the M35 plan's P4 finding 4). [M35 P6]
    fn edit_the_shot_case(&self, change: impl FnOnce(&mut Value)) {
        let path = self.vector(SHOT_CASE);
        let mut document = read(&path);
        change(&mut document);
        fs::write(&path, format!("{}\n", hand_layout(&document))).expect("write");
    }

    /// The shot case recorded one engine version back, as [`Slice::age_the_synthetic_pair`] ages
    /// the engine pair: the same two paths, which one declaration covers in both families.
    fn age_the_shot_case(&self) {
        self.edit_the_shot_case(|v| {
            v["analysis_version"] = json!(ANALYSIS_VERSION - 1);
            v["expected"]["analysis_version"] = json!(ANALYSIS_VERSION - 1);
        });
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

    /// [`Slice::new`], plus the whole committed storage, career, profile and mcp families. [M36 P13,
    /// the mcp family M29 P15]
    fn with_career(name: &str) -> Self {
        let slice = Self::new(name);
        slice.copy_the_career_families();
        slice
    }

    /// Copy the committed storage, career and profile families into the slice: every family a
    /// career declaration re-records. [M35 P9, the profile family M37 P11]
    fn copy_the_career_families(&self) {
        for family in CAREER_FAMILIES {
            copy_tree(
                &committed_spec().join("vectors").join(family),
                &self.vector(family),
            );
        }
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

    /// Every storage, career, profile and mcp document in the slice, by path: the files a career run
    /// reads, which leaves out `mcp/surface/`.
    fn career_documents(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        for (family, halves) in [
            (STORAGE, &STORAGE_DOCUMENTS[..]),
            (CAREER, &CAREER_DOCUMENTS[..]),
            (PROFILE, &PROFILE_DOCUMENTS[..]),
            (MCP, &MCP_DOCUMENTS[..]),
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

/// A document at an indent of 4, the shot family's, with no trailing newline.
fn hand_layout(document: &Value) -> String {
    let mut out = Vec::new();
    let formatter = serde_json::ser::PrettyFormatter::with_indent(b"    ");
    serde::Serialize::serialize(
        document,
        &mut serde_json::Serializer::with_formatter(&mut out, formatter),
    )
    .expect("serializes");
    String::from_utf8(out).expect("utf-8")
}

/// The written shot case is in the layout it was committed in: LF throughout and an indent of 4.
fn assert_in_the_hand_layout(path: &Path) {
    let written = text(&fs::read(path).expect("read"));
    assert!(!written.contains('\r'), "{path:?}: a CR in an LF file");
    assert!(
        written
            .lines()
            .nth(1)
            .is_some_and(|line| line.starts_with("    \"") && !line.starts_with("     ")),
        "{path:?}: not at an indent of 4:\n{}",
        &written[..written.len().min(200)]
    );
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
    assert_eq!(
        run.vectors().len(),
        8,
        "two engine, two stage, the shot case, two selection cases and the mcp case"
    );
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
///
/// **An engine declaration counts the shot family, and re-records it with the engine's** (M35
/// P6): the shot case is aged beside the synthetic pair, and the one declaration of the two version
/// paths lands on all three, the case written back in its own layout.
#[test]
fn a_dry_run_reports_and_writes_nothing() {
    let slice = Slice::new("dry-run");
    slice.age_the_synthetic_pair();
    slice.age_the_shot_case();
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
        "\nshot/corpus-2026-08-23-11\n",
        moved.as_str(),
        "8 vectors run (2 engine, 2 stage, 1 shot, 2 selection, 1 mcp); 3 changed; 0 files written",
        "the mcp family was answered by ",
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

    // And without the flag, the same declaration writes the three it reported, the shot case in
    // the layout it was committed in and at this build's version.
    let output = slice.verb(&declaration, &[]);
    assert!(output.status.success(), "{}", text(&output.stderr));
    assert!(text(&output.stdout).contains("3 changed; 3 files written"));
    let shot = read(&slice.vector(SHOT_CASE));
    assert_eq!(shot["analysis_version"], json!(ANALYSIS_VERSION));
    assert_eq!(
        shot["expected"]["analysis_version"],
        json!(ANALYSIS_VERSION)
    );
    assert_eq!(
        shot["provenance"]["oracle"],
        json!("hand"),
        "the ledger keeps the oracle"
    );
    assert_in_the_hand_layout(&slice.vector(SHOT_CASE));
}

/// **A declaration naming a shot path re-records that path and nothing else** (M35 P6). The shot
/// copy's first row is moved away from what Rust computes: undeclared, the run is refused at that
/// path and writes nothing; declared, it lands there alone, in the case's file alone, with a
/// ledger naming it and the hand oracle kept — and a second run writes nothing.
#[test]
fn a_shot_path_declared_lands_there_alone_and_a_second_run_writes_nothing() {
    let slice = Slice::new("shot-path");
    slice.edit_the_shot_case(|v| {
        v["expected"]["measurements"][0]["value"] = json!(99.0);
    });
    let edited = read(&slice.vector(SHOT_CASE));
    let undeclared = slice.declare("nothing", ANALYSIS_VERSION, &[], &[]);
    let declaration = slice.declare("shot-row", ANALYSIS_VERSION, &[], &[SHOT_ROW]);
    let before = slice.snapshot();

    match plan(&slice.spec(), &undeclared) {
        Err(Refused::Gate(refusals)) => {
            assert_eq!(refusals.len(), 1, "{refusals:?}");
            assert!(
                refusals[0].starts_with(&format!("{SHOT_ID}: {SHOT_ROW}"))
                    && refusals[0].contains("`moved`"),
                "{}",
                refusals[0]
            );
        }
        other => panic!("expected the gate to refuse the shot row, got {other:?}"),
    }
    assert_eq!(slice.snapshot(), before);

    let run = plan(&slice.spec(), &declaration).expect("the shot row is declared");
    let changed: Vec<(&str, Family)> = run.changed().map(|v| (v.id(), v.family())).collect();
    assert_eq!(changed, [(SHOT_ID, Family::Shot)]);
    assert_eq!(run.write().expect("writes"), 1);

    let after = slice.snapshot();
    let moved: Vec<&PathBuf> = before
        .keys()
        .filter(|path| before.get(*path) != after.get(*path))
        .collect();
    assert_eq!(
        moved,
        [&slice.vector(SHOT_CASE)],
        "a file other than the shot case moved"
    );
    assert_eq!(after.len(), before.len(), "a file appeared or went");
    assert_in_the_hand_layout(&slice.vector(SHOT_CASE));

    let landed = read(&slice.vector(SHOT_CASE));
    assert_eq!(landed["provenance"]["oracle"], json!("hand"));
    assert_eq!(
        landed["provenance"]["rerecords"],
        json!([{
            "analysis_version": ANALYSIS_VERSION,
            "by": "golf-core rerecord",
            "declaration": "spec/declarations/shot-row.json",
            "added": [],
            "moved": [SHOT_ROW],
        }])
    );
    // Rust's value, inside the tolerance of the one the case was committed with.
    let committed = read(&committed_spec().join("vectors").join(SHOT_CASE));
    assert!(compare(
        &committed["expected"]["measurements"][0]["value"],
        &landed["expected"]["measurements"][0]["value"]
    )
    .is_empty());
    // And every undeclared value is the copy's: undo the row and the ledger, and the copy is back.
    let mut undone = landed.clone();
    undone["expected"]["measurements"][0]["value"] = json!(99.0);
    undone["provenance"] = edited["provenance"].clone();
    assert_eq!(undone, edited, "an undeclared value moved");

    let second = plan(&slice.spec(), &declaration).expect("a second run is a no-op, not a refusal");
    assert_eq!(second.changed().count(), 0);
    assert_eq!(second.write().expect("writes nothing"), 0);
    assert_eq!(slice.snapshot(), after, "a second run wrote something");
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
    for relative in [SHOT_CASE, RULES_CASE, WINDOWS_CASE, MCP_CASE] {
        fs::write(slice.vector(relative), unreadable).expect("write");
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

/// The families a career declaration re-records, in the order it walks them. [M37 P11, the mcp
/// family M29 P15]
const CAREER_FAMILIES: [&str; 4] = [STORAGE, CAREER, PROFILE, MCP];

const OUTDATED_CASE: &str = "storage/corpus/stale-and-outdated.json";
const OUTDATED_TWIN: &str = "career/synthetic/storage-stale-and-outdated.json";
const OUTDATED_TWIN_ID: &str = "career/synthetic/storage-stale-and-outdated";
/// The outdated swing's exclusion in the case's answer, and the same entry in its twin's copy.
const OUTDATED_DETAIL: &str = "expected.corpus.excluded[1].detail";
const ADOPTED_DETAIL: &str = "input.corpus.excluded[1].detail";
const OLDER_SENTENCE: &str = "an older sentence than the reader writes";

/// How many storage, career, profile and mcp documents the slice holds, for the report's count
/// line.
fn career_counts(slice: &Slice) -> [usize; 4] {
    let documents = slice.career_documents();
    CAREER_FAMILIES.map(|family| {
        let dir = slice.vector(family);
        documents.iter().filter(|p| p.starts_with(&dir)).count()
    })
}

/// **An empty declaration at this build's career version finds nothing and writes nothing**, dry
/// or not: the committed families are what this build answers, the op cases' written files
/// included, whose committed spelling the runner keeps wherever the values agree.
#[test]
fn an_empty_career_declaration_finds_nothing_and_writes_nothing() {
    let slice = Slice::with_career("career-empty");
    let declaration = slice.declare_career("career-empty", CAREER_VERSION, &[]);
    let [storage, career, profile, mcp] = career_counts(&slice);
    assert!(storage > 0 && career > 0 && profile > 0 && mcp > 0);
    let before = slice.snapshot();

    for extra in [&["--dry-run"][..], &[][..]] {
        let output = slice.verb(&declaration, extra);
        let stdout = text(&output.stdout);
        assert!(output.status.success(), "{}", text(&output.stderr));
        let count = format!(
            "{} vectors run ({storage} storage, {career} career, {profile} profile, {mcp} mcp); 0 \
             changed; 0 files written",
            storage + career + profile + mcp
        );
        for expected in [
            format!("a declaration at career_version {CAREER_VERSION}, 0 added and 0 moved"),
            "nothing differs from the committed vectors".to_string(),
            "the mcp family was answered by ".to_string(),
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

/// **A career re-record moves its three families behind the gate**: with every copy recorded one
/// version back, an empty declaration is refused on every document at once and writes nothing; the
/// version declared, it lands on every storage, career and profile document alone, each ledgered
/// under `career_version` with no `removed` list and its oracle kept (`"hand"` and `"rust"` on the
/// profile family's two halves), every other value and every other family's file left byte for
/// byte, the committed line endings kept; and a second run writes nothing. P14's re-record is this
/// move plus the sentences it declares.
///
/// Aging a copy also drops its ledger, as `age_the_screen_documents` does, so each file's entry is
/// the only one: since `career-v1.json` ran (M36 P14) the committed vectors carry its entry, and a
/// run appends to a ledger rather than replacing it.
#[test]
fn a_career_version_move_lands_on_its_three_families_and_a_second_run_writes_nothing() {
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
    let mut walked = families.clone();
    walked.dedup();
    assert_eq!(
        walked,
        [
            Family::Storage,
            Family::Career,
            Family::Profile,
            Family::Mcp
        ],
        "the storage family first, then the career family, then the profile family, then mcp"
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
    let oracle_of = |path: &Path| -> &'static str {
        let (profile, mcp) = (slice.vector(PROFILE), slice.vector(MCP));
        if path.starts_with(profile.join("corpus")) || path.starts_with(mcp.join("rust")) {
            "rust"
        } else if path.starts_with(profile.join("hand"))
            || path.starts_with(slice.vector("storage/hand"))
            || path.starts_with(mcp.join("hand"))
        {
            "hand"
        } else {
            "python"
        }
    };

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
        assert_eq!(
            written["provenance"]["oracle"],
            json!(oracle_of(path)),
            "{path:?}: the oracle"
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

/// **A career run with no `golf-mcp` to run refuses, by name, and writes nothing** (the M29 plan's
/// rule that a tool is found, never assumed). Through the verb, so [`GOLF_MCP_ENV`] is set on the
/// child alone and no test running beside this one sees it. [M29 P15]
#[test]
fn a_career_run_with_no_golf_mcp_refuses_naming_it_and_writes_nothing() {
    let slice = Slice::with_career("career-no-mcp");
    let declaration = slice.declare_career("career-empty", CAREER_VERSION, &[]);
    let before = slice.snapshot();
    let missing = slice.root.join("no-such-golf-mcp.exe");
    let output = Command::new(env!("CARGO_BIN_EXE_golf-core"))
        .arg("rerecord")
        .arg("--declare")
        .arg(&declaration)
        .arg("--spec")
        .arg(slice.spec())
        .arg("--dry-run")
        .env(GOLF_MCP_ENV, &missing)
        .output()
        .expect("the binary runs");
    assert!(!output.status.success());
    let stderr = text(&output.stderr);
    assert!(
        stderr.contains(GOLF_MCP_ENV)
            && stderr.contains("no-such-golf-mcp.exe")
            && stderr.contains("cargo build --release -p golf-mcp"),
        "{stderr}"
    );
    assert!(text(&output.stdout).is_empty(), "{}", text(&output.stdout));
    assert_eq!(slice.snapshot(), before);
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
/// its files. With every engine, stage, shot, selection and screen document unreadable, a career
/// run still passes; with every storage, career and profile document unreadable and a sub-family
/// nobody placed beside them, an engine run and a screen run pass and write their own files alone.
///
/// **The mcp family is the one both keys walk** (M29 P19: an engine run walks it after the
/// selection family, `walk_mcp`'s doc), so it is left readable for the engine run, which is held
/// to writing none of it: the committed answers are this build's.
#[test]
fn a_career_run_reads_no_engine_or_screen_vector_and_neither_reads_a_career_one() {
    let unreadable = b"not a vector";

    let slice = Slice::with_screen("career-only");
    slice.copy_the_career_families();
    for path in slice.snapshot().into_keys() {
        let career = CAREER_FAMILIES
            .iter()
            .any(|family| path.starts_with(slice.vector(family)));
        if !career && path.starts_with(slice.spec().join("vectors")) {
            fs::write(&path, unreadable).expect("write");
        }
    }
    let declaration = slice.declare_career("career-empty", CAREER_VERSION, &[]);
    let run = plan(&slice.spec(), &declaration).expect("a career run never reads another family");
    assert!(run.vectors().iter().all(|v| matches!(
        v.family(),
        Family::Storage | Family::Career | Family::Profile | Family::Mcp
    )));
    assert_eq!(run.vectors().len(), slice.career_documents().len());

    let slice = Slice::with_screen("not-career");
    slice.copy_the_career_families();
    for path in slice.career_documents() {
        if !path.starts_with(slice.vector(MCP)) {
            fs::write(&path, unreadable).expect("write");
        }
    }
    fs::create_dir_all(slice.vector("storage/unplaced")).expect("mkdir");
    slice.age_the_synthetic_pair();
    let keys = m32_shot_keys();
    slice.age_the_screen_documents(&keys);
    let career_before: BTreeMap<PathBuf, Vec<u8>> = slice
        .snapshot()
        .into_iter()
        .filter(|(path, _)| {
            CAREER_FAMILIES
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
            CAREER_FAMILIES
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
        (
            "profile-unplaced-dir",
            "profile/unplaced/",
            "PROFILE_DOCUMENTS",
        ),
        ("mcp-unplaced-dir", "mcp/unplaced/", "MCP_DOCUMENTS"),
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

/// A storage corpus case, its career twin and the profile corpus case that adopts the twin: the
/// three links of one adopted input. [M37 P11]
const EVERY_SOURCE_CASE: &str = "storage/corpus/every-source.json";
const EVERY_SOURCE_TWIN: &str = "career/synthetic/storage-every-source.json";
const EVERY_SOURCE_PROFILE: &str = "profile/corpus/storage-every-source.json";
const EVERY_SOURCE_TWIN_ID: &str = "career/synthetic/storage-every-source";
const EVERY_SOURCE_PROFILE_ID: &str = "profile/corpus/storage-every-source";
/// A value of the corpus that no aggregate, grade or report reads, so moving it moves a copy and
/// nothing computed from one.
const SHOT_ID_ANSWER: &str = "expected.corpus.swings[0].shot.shot_id";
const SHOT_ID_ADOPTED: &str = "input.corpus.swings[0].shot.shot_id";
const OLDER_SHOT_ID: &str = "an older shot id than the reader writes";

/// **An adopted input moves along the whole chain, and only where the declaration says so** (M37
/// P11). A storage corpus case, its career twin and the profile corpus case that adopts the twin
/// are all given an older shot id than the reader writes, as three vectors recorded under an older
/// rule would hold. Declaring the storage case's path alone is refused at the twin's copy, and only
/// there: a refused career vector is passed on as committed, so the profile case, whose copy agrees
/// with the committed twin, is not refused a second time for the same missing path. The chain stops
/// at its first broken link. The twin and the profile case spell the copy at one path, so declaring
/// it lands both, and all three move, with no answer moving. And a profile case whose copy drifted
/// from an unchanged career case is refused by an empty declaration, at the key that drifted.
#[test]
fn an_adopted_profile_input_moves_with_its_career_case_and_only_where_declared() {
    let slice = Slice::with_career("profile-adopted");
    let (case, twin, profile) = (
        slice.vector(EVERY_SOURCE_CASE),
        slice.vector(EVERY_SOURCE_TWIN),
        slice.vector(EVERY_SOURCE_PROFILE),
    );
    let reader_writes = read(&case)["expected"]["corpus"]["swings"][0]["shot"]["shot_id"].clone();
    assert!(reader_writes.is_string(), "{reader_writes}");
    slice.edit_like_committed(&case, |v| {
        v["expected"]["corpus"]["swings"][0]["shot"]["shot_id"] = json!(OLDER_SHOT_ID);
    });
    for copy in [&twin, &profile] {
        slice.edit_like_committed(copy, |v| {
            v["input"]["corpus"]["swings"][0]["shot"]["shot_id"] = json!(OLDER_SHOT_ID);
        });
    }
    let before = slice.snapshot();

    let half = slice.declare_career("career-half", CAREER_VERSION, &[SHOT_ID_ANSWER]);
    match plan(&slice.spec(), &half) {
        Err(Refused::Gate(refusals)) => {
            assert_eq!(refusals.len(), 1, "{refusals:#?}");
            assert!(
                refusals[0].starts_with(&format!("{EVERY_SOURCE_TWIN_ID}: {SHOT_ID_ADOPTED}: ")),
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
        &[SHOT_ID_ANSWER, SHOT_ID_ADOPTED],
    );
    let expected_before = [&twin, &profile].map(|path| read(path)["expected"].clone());
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
            ("storage/corpus/every-source", vec![SHOT_ID_ANSWER]),
            (EVERY_SOURCE_TWIN_ID, vec![SHOT_ID_ADOPTED]),
            (EVERY_SOURCE_PROFILE_ID, vec![SHOT_ID_ADOPTED]),
        ]
    );
    assert_eq!(run.write().expect("writes"), 3);
    let written_twin = read(&twin);
    let written_profile = read(&profile);
    assert_eq!(
        written_twin["input"]["corpus"],
        read(&case)["expected"]["corpus"],
        "the twin's copy is its case's answer again"
    );
    assert_eq!(
        written_profile["input"]["corpus"], written_twin["input"]["corpus"],
        "the profile case's copy is its career case's again"
    );
    assert_eq!(
        written_profile["input"]["corpus"]["swings"][0]["shot"]["shot_id"],
        reader_writes
    );
    assert_eq!(
        [&written_twin, &written_profile].map(|v| v["expected"].clone()),
        expected_before,
        "an answer moved"
    );
    assert_eq!(written_profile["provenance"]["oracle"], json!("rust"));

    let after = slice.snapshot();
    let second = plan(&slice.spec(), &both).expect("a second run is a no-op, not a refusal");
    assert_eq!(second.changed().count(), 0);
    assert_eq!(slice.snapshot(), after);

    let slice = Slice::with_career("profile-drifted");
    slice.edit_like_committed(&slice.vector(EVERY_SOURCE_PROFILE), |v| {
        v["input"]["display_name"] = json!("someone else");
    });
    let empty = slice.declare_career("career-empty", CAREER_VERSION, &[]);
    match plan(&slice.spec(), &empty) {
        Err(Refused::Gate(refusals)) => {
            assert_eq!(refusals.len(), 1, "{refusals:#?}");
            assert!(
                refusals[0]
                    .starts_with(&format!("{EVERY_SOURCE_PROFILE_ID}: input.display_name: ")),
                "{}",
                refusals[0]
            );
        }
        other => panic!("expected the drifted copy to be refused, got {other:?}"),
    }
}

// ------------------------------------------------------------------- the shot family [M35 P6]

/// **A screen or career run never opens `shot/`, and an engine run always does** (M35 P6). With the
/// shot case unreadable and a sub-family nobody placed beside it, a screen run and a career run
/// pass and leave both as they were; an engine run over the same slice stops on the stray entry,
/// and then on the unreadable case, so the setup is one that a run reading `shot/` could not miss.
#[test]
fn a_screen_or_career_run_never_opens_the_shot_family() {
    let slice = Slice::with_screen("not-shot");
    slice.copy_the_career_families();
    fs::write(slice.vector(SHOT_CASE), b"not a vector").expect("write");
    fs::create_dir_all(slice.vector("shot/unplaced")).expect("mkdir");
    let shot_files = |slice: &Slice| -> BTreeMap<PathBuf, Vec<u8>> {
        slice
            .snapshot()
            .into_iter()
            .filter(|(path, _)| path.starts_with(slice.vector("shot")))
            .collect()
    };
    let shot_before = shot_files(&slice);

    let keys = m32_shot_keys();
    slice.age_the_screen_documents(&keys);
    let screen = declare_shape_only(&slice, "screen-shape", &keys);
    let run = plan(&slice.spec(), &screen).expect("a screen run never opens shot/");
    assert!(run.vectors().iter().all(|v| v.family() == Family::Screen));
    run.write().expect("writes");
    let career = slice.declare_career("career-empty", CAREER_VERSION, &[]);
    let run = plan(&slice.spec(), &career).expect("a career run never opens shot/");
    assert!(run.vectors().iter().all(|v| matches!(
        v.family(),
        Family::Storage | Family::Career | Family::Profile | Family::Mcp
    )));
    assert_eq!(
        shot_files(&slice),
        shot_before,
        "another family's run wrote a shot file"
    );
    assert!(slice.vector("shot/unplaced").is_dir());

    let engine = slice.declare("nothing", ANALYSIS_VERSION, &[], &[]);
    match plan(&slice.spec(), &engine) {
        Err(Refused::Files(why)) => {
            assert!(
                why.contains("unplaced") && why.contains("is not a shot case"),
                "{why}"
            );
        }
        other => panic!("expected the shot walk to stop on the stray entry, got {other:?}"),
    }
    fs::remove_dir(slice.vector("shot/unplaced")).expect("rmdir");
    match plan(&slice.spec(), &engine) {
        Err(Refused::Files(why)) => {
            assert!(
                why.contains("corpus-2026-08-23-11.json") && why.starts_with("parsing"),
                "{why}"
            );
        }
        other => panic!("expected the shot walk to stop on the unreadable case, got {other:?}"),
    }
}

// ------------------------------------------- the selection family, and mcp under the engine [M29 P19]

/// Edit a gzipped copy in place and write it back as the recorder and the re-record write it: LF,
/// an indent of 2.
fn edit_gzipped(path: &Path, change: impl FnOnce(&mut Value)) {
    let mut document = read(path);
    change(&mut document);
    let text = serde_json::to_string_pretty(&document).expect("serializes");
    let mut encoder = GzEncoder::new(Vec::new(), Compression::fast());
    encoder.write_all(text.as_bytes()).expect("gzip");
    fs::write(path, encoder.finish().expect("gzip")).expect("write");
}

/// **An engine run walks the selection family after the shot family, and the mcp family last**,
/// and the report counts both and names the `golf-mcp` it ran.
#[test]
fn an_engine_run_walks_selection_and_then_mcp_after_the_shot_family() {
    let slice = Slice::new("engine-order");
    let declaration = slice.declare("nothing", ANALYSIS_VERSION, &[], &[]);
    let run = plan(&slice.spec(), &declaration).expect("the slice is what this build answers");
    let mut walked: Vec<Family> = run.vectors().iter().map(|v| v.family()).collect();
    walked.dedup();
    assert_eq!(
        walked,
        [
            Family::Engine,
            Family::Stage,
            Family::Shot,
            Family::Selection,
            Family::Mcp
        ]
    );
    let ids: Vec<&str> = run
        .vectors()
        .iter()
        .filter(|v| v.family() == Family::Selection)
        .map(|v| v.id())
        .collect();
    assert_eq!(ids, [RULES_ID, WINDOWS_ID], "SELECTION_DOCUMENTS' order");
    let report = run.report(true, 0);
    assert!(
        report.contains("8 vectors run (2 engine, 2 stage, 1 shot, 2 selection, 1 mcp); 0 changed")
            && report.contains("the mcp family was answered by "),
        "{report}"
    );
}

/// **A selection case moves behind the gate**: recorded one engine version back, an empty
/// declaration is refused at its version alone; the version declared, it lands on that case alone,
/// in its gzip's layout, ledgered with its oracle kept and every other value as it was — and a second
/// run writes nothing. This is the move the committed family made under `v17.json` (the user's
/// decision at the P18/P19 boundary).
#[test]
fn a_selection_version_move_lands_on_the_case_alone_and_a_second_run_writes_nothing() {
    let slice = Slice::new("selection-move");
    let path = slice.vector(RULES_CASE);
    edit_gzipped(&path, |v| {
        v["analysis_version"] = json!(ANALYSIS_VERSION - 1);
        v["provenance"]
            .as_object_mut()
            .expect("a provenance")
            .remove("rerecords");
    });
    let aged = read(&path);
    let empty = slice.declare("nothing", ANALYSIS_VERSION, &[], &[]);
    match plan(&slice.spec(), &empty) {
        Err(Refused::Gate(refusals)) => {
            assert_eq!(refusals.len(), 1, "{refusals:?}");
            assert!(
                refusals[0].starts_with(&format!("{RULES_ID}: analysis_version")),
                "{}",
                refusals[0]
            );
        }
        other => panic!("expected the gate to refuse the aged case, got {other:?}"),
    }

    let declaration = slice.declare("version", ANALYSIS_VERSION, &[], &["analysis_version"]);
    let before = slice.snapshot();
    let run = plan(&slice.spec(), &declaration).expect("the version is declared");
    let changed: Vec<(&str, Family)> = run.changed().map(|v| (v.id(), v.family())).collect();
    assert_eq!(changed, [(RULES_ID, Family::Selection)]);
    assert_eq!(run.write().expect("writes"), 1);
    let after = slice.snapshot();
    let moved: Vec<&PathBuf> = before
        .keys()
        .filter(|p| before.get(*p) != after.get(*p))
        .collect();
    assert_eq!(moved, [&path], "a file other than the selection case moved");
    assert!(
        !text(&decompressed(&after[&path])).contains('\r'),
        "CRLF in a gzip"
    );

    let landed = read(&path);
    assert_eq!(landed["analysis_version"], json!(ANALYSIS_VERSION));
    assert_eq!(landed["provenance"]["oracle"], json!("python"));
    assert_eq!(
        landed["provenance"]["rerecords"],
        json!([{
            "analysis_version": ANALYSIS_VERSION,
            "by": "golf-core rerecord",
            "declaration": "spec/declarations/version.json",
            "added": [],
            "moved": ["analysis_version"],
        }])
    );
    let mut undone = landed.clone();
    undone["analysis_version"] = aged["analysis_version"].clone();
    undone["provenance"] = aged["provenance"].clone();
    assert_eq!(undone, aged, "an undeclared value moved");

    let second = plan(&slice.spec(), &declaration).expect("a second run is a no-op");
    assert_eq!(second.changed().count(), 0);
    assert_eq!(second.write().expect("writes nothing"), 0);
    assert_eq!(slice.snapshot(), after, "a second run wrote something");
}

/// **A moved pick moves the windows chosen from it, and the declaration must say so.** Both cases'
/// first reason is given an older sentence than the rules write, as a rule change would leave them.
/// In the rules case the declared path lands as any declared move does. In the windows case the
/// `_auto_windows` answer logs that pick's reason, so the older sentence is in its log line too, and
/// since M29 P25 Rust answers that call rather than carrying it: the same declaration is refused by
/// the gate at the log line it did not name, nothing is written, and a declaration naming both
/// lands both. Before P25 the carry's guard refused this case whatever was declared.
#[test]
fn a_moved_pick_moves_the_windows_chosen_from_it() {
    const OLDER: &str = "an older sentence than the rules write";
    const PICK_LOG: &str = "expected.results[1].log[0]";
    let slice = Slice::new("selection-moved-pick");
    let declaration = slice.declare("first-reason", ANALYSIS_VERSION, &[], &[FIRST_REASON]);

    edit_gzipped(&slice.vector(RULES_CASE), |v| {
        v["expected"]["results"][0]["reason"] = json!(OLDER);
    });
    let run = plan(&slice.spec(), &declaration).expect("the rules case's move is declared");
    let changed: Vec<&str> = run.changed().map(|v| v.id()).collect();
    assert_eq!(changed, [RULES_ID]);
    assert_eq!(run.write().expect("writes"), 1);
    let landed = read(&slice.vector(RULES_CASE));
    assert_ne!(landed["expected"]["results"][0]["reason"], json!(OLDER));

    edit_gzipped(&slice.vector(WINDOWS_CASE), |v| {
        v["expected"]["results"][0]["reason"] = json!(OLDER);
        v["expected"]["results"][1]["log"][0] = json!(format!("  face-on: {OLDER}"));
    });
    let before = slice.snapshot();
    match plan(&slice.spec(), &declaration) {
        Err(Refused::Gate(refusals)) => {
            assert_eq!(refusals.len(), 1, "{refusals:?}");
            assert!(
                refusals[0].starts_with(&format!("{WINDOWS_ID}: {PICK_LOG}"))
                    && refusals[0].contains("`moved`"),
                "{}",
                refusals[0]
            );
        }
        other => panic!("expected the gate to refuse the undeclared log line, got {other:?}"),
    }
    assert_eq!(slice.snapshot(), before, "a refused run wrote something");

    let both = slice.declare(
        "first-reason-and-log",
        ANALYSIS_VERSION,
        &[],
        &[FIRST_REASON, PICK_LOG],
    );
    let run = plan(&slice.spec(), &both).expect("both moves are declared");
    let changed: Vec<&str> = run.changed().map(|v| v.id()).collect();
    assert_eq!(changed, [WINDOWS_ID]);
    assert_eq!(run.write().expect("writes"), 1);
    let landed = read(&slice.vector(WINDOWS_CASE));
    let reason = landed["expected"]["results"][0]["reason"]
        .as_str()
        .expect("a reason");
    assert_ne!(reason, OLDER);
    assert_eq!(
        landed["expected"]["results"][1]["log"][0],
        json!(format!("  face-on: {reason}")),
        "the log line landed with the pick it narrates"
    );
}

/// A directory under `vectors/selection/` the re-record does not place stops an engine run, and so
/// does a loose file, as under `vectors/screen/`.
#[test]
fn a_selection_sub_family_nobody_placed_stops_the_walk() {
    for (name, make) in [
        ("selection-unplaced-dir", "selection/unplaced/"),
        ("selection-unplaced-file", "selection/README.json"),
    ] {
        let slice = Slice::new(name);
        let target = slice.vector(make.trim_end_matches('/'));
        if make.ends_with('/') {
            fs::create_dir_all(&target).expect("mkdir");
        } else {
            fs::write(&target, "{}").expect("write");
        }
        let declaration = slice.declare("nothing", ANALYSIS_VERSION, &[], &[]);
        match plan(&slice.spec(), &declaration) {
            Err(Refused::Files(why)) => {
                let leaf = make
                    .trim_end_matches('/')
                    .rsplit('/')
                    .next()
                    .expect("a name");
                assert!(
                    why.contains(leaf) && why.contains("SELECTION_DOCUMENTS"),
                    "{why}"
                );
            }
            other => panic!("{make}: expected the walk to stop, got {other:?}"),
        }
    }
}

/// **An engine run with no `golf-mcp` to run refuses, by name, and writes nothing**, as a career
/// run does: since M29 P19 an engine declaration walks the mcp family too.
#[test]
fn an_engine_run_with_no_golf_mcp_refuses_naming_it_and_writes_nothing() {
    let slice = Slice::new("engine-no-mcp");
    slice.age_the_synthetic_pair();
    let declaration = slice.declare(
        "versions",
        ANALYSIS_VERSION,
        &[],
        &["analysis_version", "expected.analysis_version"],
    );
    let before = slice.snapshot();
    let missing = slice.root.join("no-such-golf-mcp.exe");
    let output = Command::new(env!("CARGO_BIN_EXE_golf-core"))
        .arg("rerecord")
        .arg("--declare")
        .arg(&declaration)
        .arg("--spec")
        .arg(slice.spec())
        .env(GOLF_MCP_ENV, &missing)
        .output()
        .expect("the binary runs");
    assert!(!output.status.success());
    let stderr = text(&output.stderr);
    assert!(
        stderr.contains(GOLF_MCP_ENV) && stderr.contains("no-such-golf-mcp.exe"),
        "{stderr}"
    );
    assert_eq!(slice.snapshot(), before);
}

// ------------------------------------------------------------------ the oracle pin [M37 P11]

/// The oracles a committed vector may name (ADR-032's twelfth addendum's three kinds): frozen
/// Python recorded it, a person worked it, or Rust's own run recorded it with nothing independent
/// behind it (the M37 plan's call 18).
const ORACLES: [&str; 3] = ["python", "hand", "rust"];

/// The families whose oracle is named here rather than in their files: `audio` and `format`
/// predate `provenance.oracle` (CONFORMANCE §2, "Who recorded each value"). Both are Python's.
/// The deleted `audio/impact.py` recorded the audio family (M20), and the format family is
/// CPython's own output, written by `conformance_vectors.build_format`.
///
/// The user's answer to the M37 plan's P11 finding 3 (option b). Writing the key into the 43 files
/// would have changed two families M37's rules hold still. It would also have changed the frozen
/// `_format_vector`, because `test_the_format_table_regenerates_byte_identically` compares every
/// format file with that function's output. So the table names the oracle once, and the files keep
/// their bytes. A family listed here whose files start carrying the key fails the pin, so the
/// table and the files can never both claim it.
///
/// Every family after these two writes its oracle into each file, and so is not a row here: the
/// `mcp` family (M29 P8) is one, recorded once from frozen Python with `provenance.oracle:
/// "python"` in every case, beside the hand and Rust cases M29 P9–P15 add to it. The `selection`
/// family (M29 P18) is another, recorded once from frozen Python in the same way.
const ORACLE_BY_FAMILY: [(&str, &str); 2] = [("audio", "python"), ("format", "python")];

/// Just enough of a vector to read its provenance; every other key is skipped unparsed, which is
/// what keeps this walk over every committed vector cheap.
#[derive(serde::Deserialize)]
struct Header {
    id: Option<String>,
    provenance: Option<Value>,
}

/// **Every committed vector names its oracle, and every ledger entry its declaration** (§M37's
/// oracle pin, the M37 plan's P11). The first half is ADR-034 clause 9 and ADR-032's twelfth
/// addendum: a family recorded by Python, worked by hand and recorded by Rust each mean something
/// different under a re-record, so the kind is written down. The second is ADR-032's fourteenth
/// addendum's option: a `provenance.rerecords` entry is what says which values are Rust's, so each
/// names the declaration that licensed it, that declaration is committed, and the entry carries the
/// declaration's own version key and value, with `by` naming the verb.
///
/// Read from the committed `spec/`, not a slice, because the claim is about what is committed.
#[test]
fn every_committed_vector_names_its_oracle_and_every_ledger_entry_its_declaration() {
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
    let spec = committed_spec();
    let vectors = spec.join("vectors");
    let mut paths = Vec::new();
    collect(&vectors, &mut paths);
    paths.sort();

    let version_keys = [
        "analysis_version",
        "screen_parser_version",
        "career_version",
    ];
    let mut declarations: BTreeMap<String, (String, Value)> = BTreeMap::new();
    let mut found = Vec::new();
    let mut by_family: BTreeMap<String, BTreeMap<String, usize>> = BTreeMap::new();
    let mut entries = 0;
    for path in &paths {
        let relative = path
            .strip_prefix(&vectors)
            .expect("under spec/vectors")
            .to_string_lossy()
            .replace('\\', "/");
        let family = relative.split('/').next().expect("a family").to_string();
        let bytes = fs::read(path).unwrap_or_else(|e| panic!("{relative}: {e}"));
        let text = if relative.ends_with(".gz") {
            text(&decompressed(&bytes))
        } else {
            text(&bytes)
        };
        let header: Header =
            serde_json::from_str(&text).unwrap_or_else(|e| panic!("{relative}: {e}"));
        let id = header.id.unwrap_or_else(|| relative.clone());
        let provenance = header.provenance.unwrap_or(Value::Null);
        let oracle = provenance["oracle"].as_str();

        if let Some(&(_, named)) = ORACLE_BY_FAMILY.iter().find(|(f, _)| *f == family) {
            if oracle.is_some() {
                found.push(format!(
                    "{id}: provenance.oracle is {oracle:?}, and ORACLE_BY_FAMILY already names \
                     `{family}`'s: one of the two must go"
                ));
            }
            *by_family
                .entry(family)
                .or_default()
                .entry(named.to_string())
                .or_default() += 1;
        } else {
            match oracle {
                Some(oracle) if ORACLES.contains(&oracle) => {
                    *by_family
                        .entry(family)
                        .or_default()
                        .entry(oracle.to_string())
                        .or_default() += 1;
                }
                other => found.push(format!(
                    "{id}: provenance.oracle is {other:?}, not one of {ORACLES:?}"
                )),
            }
        }

        for entry in provenance["rerecords"].as_array().into_iter().flatten() {
            entries += 1;
            let Some(declaration) = entry["declaration"].as_str() else {
                found.push(format!(
                    "{id}: a ledger entry names no declaration: {entry}"
                ));
                continue;
            };
            if entry["by"] != "golf-core rerecord" {
                found.push(format!("{id}: a ledger entry is not the verb's: {entry}"));
            }
            let (key, value) = declarations
                .entry(declaration.to_string())
                .or_insert_with(|| {
                    let file = spec.parent().expect("spec has a parent").join(declaration);
                    let committed = read(&file);
                    let keys: Vec<&str> = version_keys
                        .iter()
                        .copied()
                        .filter(|key| committed.get(*key).is_some())
                        .collect();
                    assert_eq!(keys.len(), 1, "{declaration}: version keys {keys:?}");
                    (keys[0].to_string(), committed[keys[0]].clone())
                });
            if &entry[key.as_str()] != value {
                found.push(format!(
                    "{id}: a ledger entry naming {declaration} records {key} {}, and the \
                     declaration is at {value}",
                    entry[key.as_str()]
                ));
            }
        }
    }
    // The table is held to the same three kinds as the files, and to families that exist, so a
    // stale row (a family renamed or retired) fails here rather than vouching for nothing.
    for (family, named) in ORACLE_BY_FAMILY {
        if !ORACLES.contains(&named) {
            found.push(format!(
                "ORACLE_BY_FAMILY names `{family}`'s oracle {named:?}, not one of {ORACLES:?}"
            ));
        }
        if !by_family.contains_key(family) {
            found.push(format!(
                "ORACLE_BY_FAMILY names `{family}`, and no committed vector is in it"
            ));
        }
    }
    assert!(
        found.is_empty(),
        "{} vectors break the oracle pin:\n{}",
        found.len(),
        found.join("\n")
    );
    assert!(
        entries > 0 && !declarations.is_empty(),
        "no ledger entry was read"
    );
    println!(
        "{} vectors, {entries} ledger entries naming {} declarations; by family: {by_family:?}",
        paths.len(),
        declarations.len()
    );
}
