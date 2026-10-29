//! `golf_lab::build_vectors`: the builder that reads `data/`, on a scratch bundle and on the store.
//! [M29 P30]
//!
//! The cases on `tests/bundle/mod.rs`' scratch bundle, analysed by `golf_lab::analyze` first so its
//! `analysis.json` is the lab's own, hold the verb to the M29 plan's D19: every vector written is one
//! its family's runner answers as recorded, a second build refuses each by name and leaves its
//! bytes, a dry run writes nothing, a stale artifact is skipped, and a build that fails its verify
//! writes nothing at all.
//!
//! The ignored gate is P30's Done line: every stored bundle built into scratch and held to the
//! committed corpus and stage vectors, the re-read shot's declared paths aside, and the real
//! storage corpus's differences from the committed one each placed in a kind the plan's P30
//! finding explains.

mod bundle;
mod common;

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Read as _;
use std::path::{Path, PathBuf};

use bundle::{fixture, sidecar, Fixture, NoReads, Spec, SESSION, SWING};
use common::{rust_only_shot_keys, DECLARED_VALUES, REREAD_TEXT};
use contracts::career::CAREER_VERSION;
use contracts::shot::SCREEN_PARSER_VERSION;
use contracts::swing::{ANALYSIS_VERSION, COMPARABLE_FROM};
use contracts::Timestamp;
use golf_core::compare::{compare, DifferenceKind};
use golf_core::stages::{run_stages, verify_compose};
use golf_core::storage_family::{run_storage, StorageCase, RUN_CORPUS};
use golf_core::{run, VectorInput};
use golf_lab::analyze::analyze_swing_dir;
use golf_lab::build_vectors::{build_vectors, Request, REAL_PLAYER, RECORDED_BY};
use golf_lab::coach::{Coach, CoachAnswer, CoachRequest};
use golf_lab::ffmpeg::FfmpegMissing;
use golf_lab::pipeline::{PipelineOptions, Tools};
use serde_json::{json, Value};

const NAME: &str = "2026-08-23-7";

/// What each case builds over the scratch bundle, by path under the vectors root.
const BUILT: [&str; 3] = [
    "corpus/2026-08-23-7.json.gz",
    "stages/corpus/2026-08-23-7.json.gz",
    "storage/corpus/real.json.gz",
];

/// A coach that fails the case if asked: every run here has coaching off.
struct Never;

impl Coach for Never {
    fn coach(&mut self, _: &CoachRequest<'_>) -> CoachAnswer {
        panic!("a case with coaching off asked the coach")
    }
}

/// The scratch bundle, analysed by the lab with a window on each view, so the cut moves frames,
/// strikes and phases and the verify has an offset to add back.
fn analysed() -> Fixture {
    let bundle = fixture(Spec::default());
    let options = PipelineOptions {
        coaching: false,
        window_face_on: Some((5, 25)),
        window_down_the_line: Some((2, 28)),
        ..bundle.options.clone()
    };
    let missing = FfmpegMissing { tried: Vec::new() };
    let (mut sidecar, mut reader) = (sidecar(), NoReads);
    let outcome = analyze_swing_dir(
        &bundle.swing_dir,
        &options,
        &mut Tools {
            ffmpeg: Err(&missing),
            sidecar: &mut sidecar,
            reader: &mut reader,
        },
        &mut Never,
        &mut || Timestamp::from_utc_micros(1_792_000_000_000_000),
        &mut |_| {},
    )
    .unwrap();
    assert!(outcome.analysis_path.is_some(), "{:?}", outcome.error);
    bundle
}

fn sessions_dir(bundle: &Fixture) -> PathBuf {
    bundle
        .swing_dir
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn request(bundle: &Fixture, out: &Path, dry_run: bool) -> Request {
    Request {
        sessions_dir: sessions_dir(bundle),
        golfers_dir: bundle.options.golfers_dir.clone(),
        out: out.to_path_buf(),
        dry_run,
    }
}

/// One run of the verb: its exit code, stdout and stderr.
fn build(request: &Request) -> (u8, String, String) {
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = build_vectors(request, &mut out, &mut err).unwrap();
    (
        code,
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap(),
    )
}

fn gunzipped(path: &Path) -> String {
    let bytes = fs::read(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let mut text = String::new();
    flate2::read::GzDecoder::new(&bytes[..])
        .read_to_string(&mut text)
        .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    text
}

fn read_gzipped(path: &Path) -> Value {
    serde_json::from_str(&gunzipped(path)).unwrap()
}

/// Every file under `root`, by path, as its bytes.
fn files(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut out = BTreeMap::new();
    if !root.exists() {
        return out;
    }
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
            } else {
                out.insert(path.clone(), fs::read(&path).unwrap());
            }
        }
    }
    out
}

// ------------------------------------------------------------------------- the scratch bundle

/// **Each vector written is one its family answers as recorded**: the corpus vector cut to the
/// windows and run back to its own `expected`, its stage vector recorded and composing onto it, and
/// the real corpus answered by the storage family's runner as written. Each is laid out as
/// `_write_json` wrote a vector, and says its oracle is Rust's.
#[test]
fn a_bundle_builds_three_vectors_each_family_answers_as_recorded() {
    let bundle = analysed();
    let out = tempfile::tempdir().unwrap();
    let (code, stdout, stderr) = build(&request(&bundle, out.path(), false));
    assert_eq!((code, stderr.as_str()), (0, ""), "{stdout}");
    for path in BUILT {
        assert!(stdout.contains(&format!("  wrote {path}\n")), "{stdout}");
    }
    assert!(
        stdout.ends_with("Wrote 3 of 3 vectors under ")
            || stdout.contains("Wrote 3 of 3 vectors under "),
        "{stdout}"
    );
    assert_eq!(files(out.path()).len(), 3);

    // The corpus vector: the cut, and the engine's answer to it.
    let engine = read_gzipped(&out.path().join(BUILT[0]));
    assert_eq!(engine["id"], json!(format!("corpus/{NAME}")));
    assert_eq!(engine["analysis_version"], json!(ANALYSIS_VERSION));
    let provenance = &engine["provenance"];
    assert_eq!(
        (
            &provenance["kind"],
            &provenance["oracle"],
            &provenance["recorded_by"]
        ),
        (&json!("corpus"), &json!("rust"), &json!(RECORDED_BY))
    );
    assert_eq!(
        provenance["frame_offset"],
        json!({"face_on": 5, "down_the_line": 2})
    );
    assert_eq!(
        provenance["source_clip_frames"],
        json!({"face_on": 30, "down_the_line": 30})
    );
    let input = &engine["input"];
    assert_eq!(input["face_on"]["frames"].as_array().unwrap().len(), 20);
    assert_eq!(input["face_on"]["clip"]["frame_count"], json!(20));
    assert_eq!(
        input["down_the_line"]["frames"].as_array().unwrap().len(),
        26
    );
    // The strike heard at frame 15 of the whole clip is frame 10 of the window; none down the line.
    assert_eq!(input["face_on_strikes"], json!([10]));
    assert_eq!(input["down_the_line_strikes"], Value::Null);
    assert_eq!(
        (&input["handedness"], input["loft_deg"].is_number()),
        (&json!("right"), true)
    );
    assert!(
        input.get("face_on_window").is_none(),
        "the slice is the window"
    );
    let given: VectorInput = serde_json::from_value(input.clone()).unwrap();
    let differences = compare(&engine["expected"], &run(&given));
    assert!(differences.is_empty(), "{differences:?}");

    // Its stage vector.
    let stages = read_gzipped(&out.path().join(BUILT[1]));
    assert_eq!(stages["id"], json!(format!("stages/corpus/{NAME}")));
    assert_eq!(
        stages["provenance"]["derived_from"],
        json!(format!("corpus/{NAME}"))
    );
    assert_eq!(stages["provenance"]["oracle"], json!("rust"));
    let differences = compare(&stages["stages"], &run_stages(&given));
    assert!(differences.is_empty(), "{differences:?}");
    verify_compose(&stages["stages"], &engine).unwrap();

    // The real corpus, read by the definition it names and answered as written.
    let real = read_gzipped(&out.path().join(BUILT[2]));
    assert_eq!(real["id"], json!("storage/corpus/real"));
    assert_eq!(real["career_version"], json!(CAREER_VERSION));
    assert_eq!(StorageCase::of(&real), Ok(StorageCase::Corpus));
    assert_eq!(real["provenance"]["worked_against"], json!(RUN_CORPUS));
    assert_eq!(real["input"]["player_id"], json!(REAL_PLAYER));
    assert_eq!(
        real["input"]["versions"],
        json!({"installed": ANALYSIS_VERSION, "comparable_from": COMPARABLE_FROM})
    );
    assert_eq!(
        real["input"]["narrowings"],
        json!({"club-7i": {"since": null, "sessions": null, "club": "7i"}})
    );
    let files_carried: BTreeSet<&str> = real["input"]["files"]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(
        files_carried,
        BTreeSet::from([
            "2026-08-23/7/analysis.json",
            "2026-08-23/7/analysis.state.json",
            "2026-08-23/7/manifest.json",
        ])
    );
    let slim: Value = serde_json::from_str(
        real["input"]["files"]["2026-08-23/7/analysis.json"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    let swing_keys: BTreeSet<&str> = slim["swing"]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(
        swing_keys,
        BTreeSet::from(["measurements", "shot", "unscored"])
    );
    assert_eq!(
        real["expected"]["corpus"]["swings"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let differences = compare(&real["expected"], &run_storage(&real).unwrap());
    assert!(differences.is_empty(), "{differences:?}");

    // `_write_json`'s layout: the uncompressed text indented by 2, with no trailing newline in a gzip.
    let text = gunzipped(&out.path().join(BUILT[0]));
    assert!(
        text.lines().nth(1).unwrap().starts_with("  \""),
        "{}",
        &text[..60]
    );
    assert!(!text.ends_with('\n'));
}

/// **A second build refuses every vector already there, by name, and leaves its bytes** (D19), the
/// rest written, exit 1. A file put there by anything else is refused as a committed one is.
#[test]
fn a_vector_already_there_is_refused_by_name_and_left_as_it_is() {
    let bundle = analysed();
    let out = tempfile::tempdir().unwrap();
    assert_eq!(build(&request(&bundle, out.path(), false)).0, 0);
    let first = files(out.path());

    let (code, stdout, stderr) = build(&request(&bundle, out.path(), false));
    assert_eq!(code, 1, "{stdout}{stderr}");
    for (path, id) in BUILT.iter().zip([
        format!("corpus/{NAME}"),
        format!("stages/corpus/{NAME}"),
        "storage/corpus/real".to_string(),
    ]) {
        let at = out.path().join(path);
        assert!(
            stderr.contains(&format!(
                "refused: {id} is already at {}, and golf-core rerecord is the only thing that \
                 rewrites a committed vector\n",
                golf_lab::import::spelled(&at).display()
            )),
            "{stderr}"
        );
    }
    assert!(stdout.contains("Wrote 0 of 3 vectors"), "{stdout}");
    assert!(stdout.contains("3 refused, 0 failed"), "{stdout}");
    assert_eq!(files(out.path()), first, "a refused vector was rewritten");

    // One stands where the stage vector would go, and only that one is refused.
    let fresh = tempfile::tempdir().unwrap();
    let squatter = fresh.path().join(BUILT[1]);
    fs::create_dir_all(squatter.parent().unwrap()).unwrap();
    fs::write(&squatter, b"committed").unwrap();
    let (code, stdout, stderr) = build(&request(&bundle, fresh.path(), false));
    assert_eq!(code, 1, "{stdout}{stderr}");
    assert_eq!(stderr.matches("refused: ").count(), 1, "{stderr}");
    assert!(
        stderr.contains(&format!("refused: stages/corpus/{NAME} ")),
        "{stderr}"
    );
    assert_eq!(fs::read(&squatter).unwrap(), b"committed");
    let written = files(fresh.path());
    assert_eq!(written.len(), 3);
    for path in [BUILT[0], BUILT[2]] {
        assert_eq!(
            written[&fresh.path().join(path)],
            first[&out.path().join(path)]
        );
    }
}

/// **A dry run builds and verifies everything and writes nothing**, saying what it would write and
/// what it would refuse, with the codes a real run would give.
#[test]
fn a_dry_run_writes_nothing_and_says_what_it_would() {
    let bundle = analysed();
    let out = tempfile::tempdir().unwrap();
    let root = out.path().join("vectors");
    let (code, stdout, stderr) = build(&request(&bundle, &root, true));
    assert_eq!((code, stderr.as_str()), (0, ""), "{stdout}");
    for path in BUILT {
        assert!(
            stdout.contains(&format!("  would write {path}\n")),
            "{stdout}"
        );
    }
    assert!(stdout.contains("Would write 3 of 3 vectors"), "{stdout}");
    assert!(!root.exists(), "a dry run made the vectors root");

    assert_eq!(
        build(&request(&bundle, out.path().join("real").as_path(), false)).0,
        0
    );
    let before = files(out.path());
    let (code, _, stderr) = build(&request(&bundle, out.path().join("real").as_path(), true));
    assert_eq!(code, 1);
    assert_eq!(stderr.matches("refused: ").count(), 3, "{stderr}");
    assert_eq!(files(out.path()), before);
}

/// **An artifact at another engine version is skipped with `_corpus`' line**, and the real corpus is
/// still built over it, since the corpus reader reads every generation; a missing sessions directory
/// is refused, and nothing is written.
#[test]
fn a_stale_artifact_is_skipped_and_a_missing_sessions_dir_is_refused() {
    let bundle = analysed();
    let analysis = bundle.swing_dir.join("analysis.json");
    let mut stored: Value = serde_json::from_slice(&fs::read(&analysis).unwrap()).unwrap();
    stored["analysis_version"] = json!(ANALYSIS_VERSION - 1);
    fs::write(&analysis, serde_json::to_string(&stored).unwrap()).unwrap();

    let out = tempfile::tempdir().unwrap();
    let (code, stdout, stderr) = build(&request(&bundle, out.path(), false));
    assert_eq!(code, 0, "{stdout}{stderr}");
    assert_eq!(
        stderr,
        format!(
            "  skip {SWING}: stored at v{}, engine is v{ANALYSIS_VERSION} — run golf-lab \
             reanalyze\n",
            ANALYSIS_VERSION - 1
        )
    );
    let written: Vec<PathBuf> = files(out.path()).into_keys().collect();
    assert_eq!(written, [out.path().join(BUILT[2])]);

    let gone = out.path().join("no-sessions-here");
    let (code, stdout, stderr) = build(&Request {
        sessions_dir: gone.clone(),
        ..request(&bundle, &out.path().join("other"), false)
    });
    assert_eq!((code, stdout.as_str()), (2, ""));
    assert_eq!(
        stderr,
        format!(
            "golf-lab build-vectors: {} is missing, and the real corpus is read from it; nothing \
             was written\n",
            golf_lab::import::spelled(&gone).display()
        )
    );
    assert!(!out.path().join("other").exists());
}

/// **A vector that does not reproduce its stored analysis stops the build, and nothing is
/// written** (`_verify_against_stored`): here a refusal the engine never made, appended to the
/// stored `unscored`, and a stored phase moved a frame.
#[test]
fn a_vector_that_does_not_reproduce_its_artifact_writes_nothing() {
    let bundle = analysed();
    let analysis = bundle.swing_dir.join("analysis.json");
    let original = fs::read(&analysis).unwrap();
    let stored: Value = serde_json::from_slice(&original).unwrap();

    let mut refused = stored.clone();
    refused["swing"]["unscored"]
        .as_array_mut()
        .unwrap()
        .push(json!({"name": "tempo", "reason": "no_band"}));
    let mut moved = stored.clone();
    let phases = moved["swing"]["phases"].as_array_mut().unwrap();
    assert!(!phases.is_empty(), "the scratch swing has no phase to move");
    phases[0]["end_frame"] = json!(phases[0]["end_frame"].as_i64().unwrap() + 1);

    for (tampered, at) in [(refused, "swing.unscored"), (moved, "swing.phases")] {
        fs::write(&analysis, serde_json::to_string(&tampered).unwrap()).unwrap();
        let out = tempfile::tempdir().unwrap();
        let (code, stdout, stderr) = build(&request(&bundle, out.path(), false));
        assert_eq!((code, stdout.as_str()), (2, ""), "{stderr}");
        assert!(
            stderr.starts_with(&format!(
                "golf-lab build-vectors: corpus/{NAME}: the windowed vector does not reproduce \
                 the stored analysis at {at} — "
            )),
            "{stderr}"
        );
        assert!(stderr.ends_with("; nothing was written\n"), "{stderr}");
        assert!(files(out.path()).is_empty());
    }
    fs::write(&analysis, original).unwrap();
}

// ------------------------------------------------------------------------------------ the store

/// Whether one difference inside a stored shot is one the re-read is declared to move (P25's
/// `tests/inputs.rs` rule): a `DECLARED_VALUES` move, counted into `seen`; the re-read's provenance
/// text; the stamp, at this parser and with tiles present; or a key Rust's `ShotData` adds, at its
/// default, which is what a shot frozen Python parsed holds.
fn declared_in_shot(
    shot_id: &str,
    path: &str,
    kind: &DifferenceKind,
    built: &Value,
    rust_only: &BTreeSet<String>,
    seen: &mut [usize],
) -> bool {
    let at = path
        .split('.')
        .fold(built, |value, key| value.get(key).unwrap_or(&Value::Null));
    if let DifferenceKind::Moved { expected, actual } = kind {
        if let Some(index) = DECLARED_VALUES.iter().position(|d| {
            d.shot == shot_id
                && d.key == path
                && serde_json::from_str::<Value>(d.stored).unwrap() == *expected
                && serde_json::from_str::<Value>(d.reread).unwrap() == *actual
        }) {
            seen[index] += 1;
            return true;
        }
    }
    if REREAD_TEXT.iter().any(|text| path.starts_with(text)) {
        return true;
    }
    match path {
        "provenance.parser_version" => *at == json!(SCREEN_PARSER_VERSION),
        "provenance.fields_present" => at.as_array().is_some_and(|tiles| !tiles.is_empty()),
        _ => {
            matches!(kind, DifferenceKind::AddedKey)
                && rust_only.contains(path)
                && (at.is_null() || *at == json!({}))
        }
    }
}

/// Every difference between two shots, each either declared or named in `undeclared`.
fn shot_differences(
    committed: &Value,
    built: &Value,
    rust_only: &BTreeSet<String>,
    seen: &mut [usize],
    undeclared: &mut Vec<String>,
    where_: &str,
) {
    let shot_id = committed["shot_id"].as_str().unwrap_or_default();
    for difference in compare(committed, built) {
        if !declared_in_shot(
            shot_id,
            difference.path.as_str(),
            &difference.kind,
            built,
            rust_only,
            seen,
        ) {
            undeclared.push(format!("{where_}: shot {shot_id}: {difference}"));
        }
    }
}

/// Whether a comparator path is `key` or inside it, and not a sibling that merely starts with the
/// same letters (`shot_unscored`, `shot_needs_review`).
fn within(path: &str, key: &str) -> bool {
    path == key
        || path
            .strip_prefix(key)
            .is_some_and(|rest| rest.starts_with('.'))
}

fn require_every_declared_value_seen(seen: &[usize], family: &str, undeclared: &mut Vec<String>) {
    for (declared, count) in DECLARED_VALUES.iter().zip(seen) {
        if *count == 0 {
            undeclared.push(format!(
                "{family}: {} {} {} -> {} was never seen",
                declared.shot, declared.key, declared.stored, declared.reread
            ));
        }
    }
}

/// **P30's Done line**: every stored bundle built into scratch, and what was built held to what is
/// committed. Each corpus vector's input and answer are the committed one's but for the shot's
/// declared paths (the re-read P29 wrote into the store, the committed vectors still carrying frozen
/// Python's shot), and its provenance names Rust; each stage vector's intermediates are the
/// committed ones exactly, under §3's rules. The real storage corpus differs from the committed one
/// in exactly the kinds the M29 plan's P30 finding explains, each counted here: Rust's engine pair,
/// P29's re-run (each `analysis_version` and state file's times), Rust's slim (`swing.unscored`, and
/// the `shot_unscored` the reader lifts from it), and the re-read shot.
#[test]
#[ignore = "needs data/processed/ (gitignored), as the M29 plan's P29 wrote it"]
fn every_stored_bundle_builds_its_committed_vectors_but_the_re_read_shot() {
    // The verb's own root, with no `..` in it, so each note spells its path from the repository.
    let repo = golf_core::reports::repo_root();
    let processed = repo.join("data").join("processed");
    let committed_root = repo.join("spec/vectors");
    let out = tempfile::tempdir().unwrap();
    let (code, stdout, stderr) = build(&Request {
        sessions_dir: processed.join("sessions"),
        golfers_dir: processed.join("golfers"),
        out: out.path().to_path_buf(),
        dry_run: false,
    });
    println!("{}", stdout.lines().last().unwrap_or_default());
    assert_eq!((code, stderr.as_str()), (0, ""), "{stdout}");

    let rust_only = rust_only_shot_keys();
    let mut undeclared = Vec::new();
    let names: BTreeSet<String> = fs::read_dir(committed_root.join("corpus"))
        .unwrap()
        .filter_map(|entry| {
            let name = entry.unwrap().file_name().to_string_lossy().into_owned();
            name.strip_suffix(".json.gz").map(str::to_string)
        })
        .collect();
    let built_names: BTreeSet<String> = fs::read_dir(out.path().join("corpus"))
        .unwrap()
        .filter_map(|entry| {
            let name = entry.unwrap().file_name().to_string_lossy().into_owned();
            name.strip_suffix(".json.gz").map(str::to_string)
        })
        .collect();
    assert_eq!(
        built_names, names,
        "the store and the committed corpus name different swings"
    );

    // The corpus and stage vectors.
    let (mut seen_input, mut seen_expected) = (
        vec![0; DECLARED_VALUES.len()],
        vec![0; DECLARED_VALUES.len()],
    );
    for name in &names {
        let file = format!("{name}.json.gz");
        let committed = read_gzipped(&committed_root.join("corpus").join(&file));
        let built = read_gzipped(&out.path().join("corpus").join(&file));
        let here = format!("corpus/{name}");
        for key in ["id", "analysis_version"] {
            if committed[key] != built[key] {
                undeclared.push(format!(
                    "{here}: {key} {} -> {}",
                    committed[key], built[key]
                ));
            }
        }
        for key in ["kind", "note", "frame_offset", "source_clip_frames"] {
            if committed["provenance"][key] != built["provenance"][key] {
                undeclared.push(format!("{here}: provenance.{key} differs"));
            }
        }
        assert_eq!(built["provenance"]["oracle"], json!("rust"), "{here}");

        let (want, ours) = (&committed["input"], &built["input"]);
        for difference in compare(&json!({"input": want}), &json!({"input": ours})) {
            if !within(difference.path.as_str(), "input.shot") {
                undeclared.push(format!("{here}: {difference}"));
            }
        }
        shot_differences(
            &want["shot"],
            &ours["shot"],
            &rust_only,
            &mut seen_input,
            &mut undeclared,
            &format!("{here} input"),
        );
        for difference in compare(&committed["expected"], &built["expected"]) {
            if !within(difference.path.as_str(), "swing.shot") {
                undeclared.push(format!("{here} expected: {difference}"));
            }
        }
        shot_differences(
            &committed["expected"]["swing"]["shot"],
            &built["expected"]["swing"]["shot"],
            &rust_only,
            &mut seen_expected,
            &mut undeclared,
            &format!("{here} expected"),
        );

        let committed = read_gzipped(&committed_root.join("stages/corpus").join(&file));
        let built = read_gzipped(&out.path().join("stages/corpus").join(&file));
        for key in ["id", "analysis_version"] {
            if committed[key] != built[key] {
                undeclared.push(format!("stages/{here}: {key} differs"));
            }
        }
        if committed["provenance"]["derived_from"] != built["provenance"]["derived_from"] {
            undeclared.push(format!("stages/{here}: derived_from differs"));
        }
        for difference in compare(&committed["stages"], &built["stages"]) {
            undeclared.push(format!("stages/{here}: {difference}"));
        }
    }
    require_every_declared_value_seen(&seen_input, "corpus inputs", &mut undeclared);
    require_every_declared_value_seen(&seen_expected, "corpus answers", &mut undeclared);

    // The real storage corpus, kind by kind.
    let committed = read_gzipped(&committed_root.join("storage/corpus/real.json.gz"));
    let built = read_gzipped(&out.path().join("storage/corpus/real.json.gz"));
    let mut kinds: BTreeMap<&str, usize> = BTreeMap::new();
    let (want, ours) = (&committed["input"], &built["input"]);
    for key in ["player_id", "narrowings"] {
        if want[key] != ours[key] {
            undeclared.push(format!("storage/corpus/real: input.{key} differs"));
        }
    }
    assert_eq!(
        (&want["versions"], &ours["versions"]),
        (
            &json!({"installed": ANALYSIS_VERSION - 1, "comparable_from": ANALYSIS_VERSION - 1}),
            &json!({"installed": ANALYSIS_VERSION, "comparable_from": COMPARABLE_FROM})
        ),
        "the engine pairs"
    );
    let (want_files, our_files) = (
        want["files"].as_object().unwrap(),
        ours["files"].as_object().unwrap(),
    );
    assert_eq!(
        want_files.keys().collect::<Vec<_>>(),
        our_files.keys().collect::<Vec<_>>(),
        "the trees carry different files"
    );
    let mut seen_files = vec![0; DECLARED_VALUES.len()];
    for (path, text) in want_files {
        let ours = &our_files[path];
        if text == ours {
            continue;
        }
        let here = format!("storage/corpus/real: input.files[{path}]");
        let parse = |text: &Value| serde_json::from_str::<Value>(text.as_str().unwrap()).unwrap();
        let (text, ours) = (parse(text), parse(ours));
        if path.ends_with("/analysis.state.json") {
            for difference in compare(&text, &ours) {
                match difference.path.as_str() {
                    "started_at" | "completed_at" | "duration_seconds" => {
                        *kinds.entry("state file: P29's run times").or_default() += 1
                    }
                    _ => undeclared.push(format!("{here}: {difference}")),
                }
            }
        } else if path.ends_with("/analysis.json") {
            for difference in compare(&text, &ours) {
                let at = difference.path.as_str();
                match &difference.kind {
                    DifferenceKind::Moved { expected, actual }
                        if at == "analysis_version"
                            && *expected == json!(ANALYSIS_VERSION - 1)
                            && *actual == json!(ANALYSIS_VERSION) =>
                    {
                        *kinds
                            .entry("slim: analysis_version, P29's re-run")
                            .or_default() += 1
                    }
                    DifferenceKind::AddedKey if at == "swing.unscored" => {
                        *kinds.entry("slim: swing.unscored, Rust's").or_default() += 1
                    }
                    _ if within(at, "swing.shot") => {}
                    _ => undeclared.push(format!("{here}: {difference}")),
                }
            }
            shot_differences(
                &text["swing"]["shot"],
                &ours["swing"]["shot"],
                &rust_only,
                &mut seen_files,
                &mut undeclared,
                &here,
            );
        } else {
            undeclared.push(format!("{here}: the text differs"));
        }
    }
    require_every_declared_value_seen(&seen_files, "the real tree", &mut undeclared);

    // Its answer: every corpus, the whole and each narrowing, swing by swing.
    let mut seen_answer = vec![0; DECLARED_VALUES.len()];
    // `_corpus_answer`'s `{corpus, properties}`, for the whole corpus at `expected`'s root and for
    // each narrowing under `expected.narrowed`.
    let answer = |document: &Value, narrowing: Option<&str>| match narrowing {
        None => document["expected"].clone(),
        Some(name) => document["expected"]["narrowed"][name].clone(),
    };
    let mut narrowings: Vec<Option<String>> = vec![None];
    narrowings.extend(
        committed["expected"]["narrowed"]
            .as_object()
            .unwrap()
            .keys()
            .map(|name| Some(name.clone())),
    );
    for narrowing in &narrowings {
        let label = narrowing.as_deref().map_or("corpus".to_string(), |name| {
            format!("narrowed.{name}.corpus")
        });
        let (mut want, mut ours) = (
            answer(&committed, narrowing.as_deref()),
            answer(&built, narrowing.as_deref()),
        );
        let (want_swings, our_swings) = (
            want["corpus"]["swings"].as_array().unwrap().clone(),
            ours["corpus"]["swings"].as_array().unwrap().clone(),
        );
        assert_eq!(want_swings.len(), our_swings.len(), "{label}: swings");
        for (index, (want, ours)) in want_swings.iter().zip(&our_swings).enumerate() {
            let here = format!("storage/corpus/real: {label}.swings[{index}]");
            for difference in compare(want, ours) {
                let path = difference.path.as_str();
                match &difference.kind {
                    DifferenceKind::Moved { expected, actual }
                        if path == "analysis_version"
                            && *expected == json!(ANALYSIS_VERSION - 1)
                            && *actual == json!(ANALYSIS_VERSION) =>
                    {
                        *kinds
                            .entry("answer: analysis_version, P29's re-run")
                            .or_default() += 1
                    }
                    DifferenceKind::AddedKey if path == "shot_unscored" => {
                        *kinds
                            .entry("answer: shot_unscored, lifted from Rust's slim")
                            .or_default() += 1
                    }
                    _ if within(path, "shot") => {}
                    _ => undeclared.push(format!("{here}: {difference}")),
                }
            }
            shot_differences(
                &want["shot"],
                &ours["shot"],
                &rust_only,
                &mut seen_answer,
                &mut undeclared,
                &here,
            );
        }
        // The rest of the answer, the derived counts and the narrowings' own included, is equal.
        want["corpus"]["swings"] = Value::Null;
        ours["corpus"]["swings"] = Value::Null;
        if narrowing.is_none() {
            want["narrowed"] = Value::Null;
            ours["narrowed"] = Value::Null;
        }
        for difference in compare(&want, &ours) {
            undeclared.push(format!("storage/corpus/real: {label}: {difference}"));
        }
    }
    require_every_declared_value_seen(&seen_answer, "the real answer", &mut undeclared);

    println!(
        "{} corpus and stage vectors; declared values seen: inputs {seen_input:?}, answers \
         {seen_expected:?}, real tree {seen_files:?}, real answer {seen_answer:?}",
        names.len()
    );
    for (kind, count) in &kinds {
        println!("  storage/corpus/real, {kind}: {count}");
    }
    assert!(undeclared.is_empty(), "{}", undeclared.join("\n"));
    assert_eq!(
        kinds.keys().copied().collect::<Vec<_>>(),
        [
            "answer: analysis_version, P29's re-run",
            "answer: shot_unscored, lifted from Rust's slim",
            "slim: analysis_version, P29's re-run",
            "slim: swing.unscored, Rust's",
            "state file: P29's run times",
        ],
        "a kind of difference the plan's P30 finding explains was not seen"
    );
}

/// Every name the scratch bundle is filed under, so the constants above stay that fixture's.
#[test]
fn the_scratch_names_are_the_fixture_s() {
    assert_eq!(NAME, format!("{SESSION}-{SWING}"));
    assert!(BUILT.iter().all(|path| path.ends_with(".json.gz")));
}
