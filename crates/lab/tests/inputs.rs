//! The M29 plan's D20 input gate: every stored bundle assembled by the lab, in a dry run, and its
//! engine input held to the committed corpus vector's. [M29 P25, P29]
//!
//! `spec/vectors/corpus/` was cut from these same bundles by `conformance_vectors.py::_corpus_vector`
//! out of what frozen Python's pipeline handed its engine: the stored windows, the stored shot, the
//! strikes on disk, and the golfer and loft resolved as the pipeline resolves them. So the lab's
//! [`assemble`], cut by [`corpus_input`], had to give the same input before P29 let it write
//! anything, and this was that check. P29 then wrote `data/`, and the user's (b) there made the gate
//! check the store as P29 leaves it: the same input, now with the shot served from the store P29
//! re-read into, so no photo is read. The committed vector still carries frozen Python's shot, so
//! the shot's declared paths below are seen as before. Per bundle:
//!
//! - **every input key but the shot is equal by value**, frames, strikes, intent, handedness and
//!   loft alike, and so are the vector's `frame_offset` and `source_clip_frames`: the windows the
//!   lab's selection chose are the windows Python's chose;
//! - **the shot differs only at its declared paths**, which are what the re-read every stored shot
//!   got (frozen Python's were unstamped, so [`shot_for`](golf_lab::shot) read each one again, in a
//!   dry run until P29 and into the store at P29): `common::DECLARED_VALUES`, seen on every bundle
//!   whose shot they name; the ten keys `spec/declarations/v17.json` added to the engine's echo of
//!   a shot, which frozen Python's input lacks, each at the default the v17 re-record wrote into
//!   that echo but for the stamp (`parser_version`, `fields_present`), which is seen on every
//!   bundle; and the re-read's provenance text (warnings, `raw_fields`, `parse_confidence`), each
//!   change of which `tests/reread.rs` held to a kind the plan lists until P29 (the M29 plan's P23
//!   finding 3 is that measurement). Identity, device, photo and review flag are not among them,
//!   so they are held equal;
//! - **the notes are the stored `analysis.json`'s tail**: the committed engine notes, then the
//!   assembly's in order, then the loft repair exactly when the committed result reports
//!   `no_club_loft` ([`golf_lab::pipeline::Assembled::loft_note`]), which is the order P26 writes;
//! - **the selection's log lines are the selection family's**: what the bundle logged under
//!   `Swing selection:` is the committed `selection/corpus/` case's `_auto_windows` log.
//!
//! The bundles are copied into a scratch directory first, every clip an empty stand-in, so a view
//! the cache rules failed to read as current would reach the sidecar or ffmpeg rather than stop at a
//! missing clip, and nothing under `data/` could be written whatever went wrong. The copy is hashed
//! before and after: the dry run must write nothing, pose must start no worker, and the reader must
//! read no photo, since every stored shot is current (until P29 it read each bundle's photo once).
//!
//! Needs `data/processed/` (gitignored), as P29 wrote it, so it is ignored. The reader is the lab's
//! own, so a shot wrongly read again fails here by count, and, were `data/models/ocr/` absent, by
//! the note the lookup would leave among the notes. It cannot pass with nothing to read: every
//! committed corpus vector must be reached by a stored bundle and every stored bundle must have
//! one, and a declared path never seen fails.

mod common;

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Read as _;
use std::path::{Path, PathBuf};

use contracts::shot::SCREEN_PARSER_VERSION;
use contracts::unscored::UnscoredCheckpoint;
use golf_core::compare::{compare, DifferenceKind};
use golf_lab::ffmpeg::Ffmpeg;
use golf_lab::pipeline::{assemble, corpus_input, Assembly, PipelineOptions, Tools};
use golf_lab::pose::Sidecar;
use golf_lab::shot::{OcrReader, ShotOptions};
use serde_json::{json, Value};
use storage::manifest::{load_manifest, manifest_path, Role};
use storage::shot_store::hash_image;

use common::{rust_only_shot_keys, swing_dirs, DECLARED_VALUES, REREAD_TEXT};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read_gzipped(path: &Path) -> Value {
    let bytes = fs::read(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let mut text = String::new();
    flate2::read::GzDecoder::new(&bytes[..])
        .read_to_string(&mut text)
        .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    serde_json::from_str(&text).unwrap()
}

/// Copy what the assembly reads of `processed` into `scratch`: every file of every session but the
/// clips, each clip an empty stand-in under its manifest name, and the shot store and the golfer
/// registry whole.
fn scratch_copy(processed: &Path, scratch: &Path) {
    fn copy_tree(from: &Path, to: &Path) {
        fs::create_dir_all(to).unwrap();
        for entry in fs::read_dir(from).unwrap() {
            let path = entry.unwrap().path();
            let target = to.join(path.file_name().unwrap());
            if path.is_dir() {
                copy_tree(&path, &target);
                continue;
            }
            let suffix = path
                .extension()
                .map(|s| s.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            if suffix != "mov" && suffix != "mp4" {
                fs::copy(&path, &target).unwrap();
            }
        }
    }
    for tree in ["sessions", "shots", "golfers"] {
        copy_tree(&processed.join(tree), &scratch.join(tree));
    }
    for swing_dir in swing_dirs(&scratch.join("sessions")) {
        let manifest = load_manifest(&manifest_path(&swing_dir)).unwrap();
        for role in [Role::FaceOn, Role::DownTheLine] {
            if let Some(file) = manifest.roles.get(&role) {
                fs::write(swing_dir.join(&file.filename), b"").unwrap();
            }
        }
    }
}

/// Every file under `root`, by path, as its digest.
fn digests(root: &Path) -> BTreeMap<PathBuf, String> {
    let mut out = BTreeMap::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
            } else {
                out.insert(path.clone(), hash_image(&fs::read(&path).unwrap()));
            }
        }
    }
    out
}

/// The value at a comparator path inside `document` (`a.b[2].c`), for the few paths this reads.
fn at<'v>(document: &'v Value, path: &str) -> &'v Value {
    path.split('.').fold(document, |value, key| &value[key])
}

#[test]
#[ignore = "needs data/processed/ (gitignored), as the M29 plan's P29 wrote it"]
fn every_stored_bundle_assembles_its_committed_corpus_input_but_the_re_read_shot() {
    let processed = repo().join("data/processed");
    let scratch = tempfile::tempdir().unwrap();
    scratch_copy(&processed, scratch.path());
    let before = digests(scratch.path());

    let rust_only = rust_only_shot_keys();
    let corpus_dir = repo().join("spec/vectors/corpus");
    let committed_names: BTreeSet<String> = fs::read_dir(&corpus_dir)
        .unwrap()
        .filter_map(|entry| {
            let name = entry.unwrap().file_name().to_string_lossy().into_owned();
            name.strip_suffix(".json.gz").map(str::to_string)
        })
        .collect();

    let ffmpeg = Ffmpeg::find();
    let mut sidecar = Sidecar::configured();
    let mut reader = OcrReader::configured();
    let shots = ShotOptions {
        dry_run: true,
        ..ShotOptions::new(scratch.path().join("shots"))
    };
    let options = PipelineOptions::new(shots, scratch.path().join("golfers"));

    let mut failures: Vec<String> = Vec::new();
    let mut reached = BTreeSet::new();
    let mut seen = vec![0usize; DECLARED_VALUES.len()];
    for swing_dir in swing_dirs(&scratch.path().join("sessions")) {
        let mut log: Vec<String> = Vec::new();
        let mut tools = Tools {
            ffmpeg: ffmpeg.as_ref(),
            sidecar: &mut sidecar,
            reader: &mut reader,
        };
        let assembly = assemble(&swing_dir, &options, &mut tools, &mut |line| {
            log.push(line.to_string())
        })
        .unwrap_or_else(|e| panic!("{}: {e}", swing_dir.display()));
        let Assembly::Ready(assembled) = assembly else {
            panic!("{}: refused: {assembly:?}", swing_dir.display());
        };
        let name = format!(
            "{}-{}",
            assembled.manifest.session_id, assembled.manifest.swing_id
        );
        let mut fail = |what: String| failures.push(format!("{name}: {what}"));
        reached.insert(name.clone());
        let Some(committed) = committed_names
            .contains(&name)
            .then(|| read_gzipped(&corpus_dir.join(format!("{name}.json.gz"))))
        else {
            fail("no committed corpus vector".to_string());
            continue;
        };
        let cut = corpus_input(&assembled.input);

        // Everything but the shot, by value.
        let (want, ours) = (&committed["input"], &cut.input);
        let keys = |v: &Value| {
            v.as_object()
                .unwrap()
                .keys()
                .cloned()
                .collect::<BTreeSet<_>>()
        };
        if keys(want) != keys(ours) {
            fail(format!(
                "input keys {:?} against {:?}",
                keys(ours),
                keys(want)
            ));
        }
        for key in keys(want).into_iter().filter(|key| key != "shot") {
            if want[&key] != ours[&key] {
                let differences = compare(&want[&key], &ours[&key]);
                fail(format!(
                    "input.{key} differs: {:?}",
                    &differences[..differences.len().min(3)]
                ));
            }
        }
        for (block, ours) in [
            ("frame_offset", json!(cut.frame_offset)),
            ("source_clip_frames", json!(cut.source_clip_frames)),
        ] {
            if committed["provenance"][block] != ours {
                fail(format!(
                    "provenance.{block} {ours} against {}",
                    committed["provenance"][block]
                ));
            }
        }

        // The shot, at its declared paths only.
        let (stored_shot, shot) = (&want["shot"], &ours["shot"]);
        let shot_id = stored_shot["shot_id"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        let mut stamped = false;
        let mut seen_here = BTreeSet::new();
        for difference in compare(stored_shot, shot) {
            let path = difference.path.as_str();
            match &difference.kind {
                DifferenceKind::AddedKey if rust_only.contains(path) => {
                    let value = at(shot, path);
                    let at_default = match path {
                        "provenance.parser_version" => {
                            stamped = *value == json!(SCREEN_PARSER_VERSION);
                            stamped
                        }
                        "provenance.fields_present" => {
                            value.as_array().is_some_and(|tiles| !tiles.is_empty())
                        }
                        // The rest at the default the v17 re-record wrote into the engine's echo of
                        // this same shot, rather than at a default restated here.
                        _ => value == at(&committed["expected"]["swing"]["shot"], path),
                    };
                    if !at_default {
                        fail(format!("shot.{path} added as {value}"));
                    }
                }
                DifferenceKind::Moved { expected, actual } => {
                    let declared = DECLARED_VALUES.iter().position(|d| {
                        d.shot == shot_id
                            && d.key == path
                            && serde_json::from_str::<Value>(d.stored).unwrap() == *expected
                            && serde_json::from_str::<Value>(d.reread).unwrap() == *actual
                    });
                    match declared {
                        Some(index) => {
                            seen[index] += 1;
                            seen_here.insert(index);
                        }
                        None if REREAD_TEXT.iter().any(|text| path.starts_with(text)) => {}
                        None => fail(format!("shot.{path} {expected} -> {actual}, not declared")),
                    }
                }
                _ if REREAD_TEXT.iter().any(|text| path.starts_with(text)) => {}
                _ => fail(format!("shot: {difference}, not declared")),
            }
        }
        if !stamped {
            fail("the shot is not stamped by this parser".to_string());
        }
        let named_here: BTreeSet<usize> = DECLARED_VALUES
            .iter()
            .enumerate()
            .filter(|(_, d)| d.shot == shot_id)
            .map(|(index, _)| index)
            .collect();
        if seen_here != named_here {
            fail(format!(
                "declared values {named_here:?} name shot {shot_id}, and {seen_here:?} were seen"
            ));
        }

        // The notes, as the stored artifact holds them.
        let stored: Value = serde_json::from_slice(
            &fs::read(processed.join(format!(
                "sessions/{}/{}/analysis.json",
                assembled.manifest.session_id, assembled.manifest.swing_id
            )))
            .unwrap(),
        )
        .unwrap();
        let unscored: Vec<UnscoredCheckpoint> =
            serde_json::from_value(committed["expected"]["swing"]["unscored"].clone()).unwrap();
        let mut notes: Vec<Value> = committed["expected"]["notes"].as_array().unwrap().clone();
        notes.extend(assembled.notes.iter().map(|note| json!(note)));
        notes.extend(assembled.loft_note(&unscored).map(|note| json!(note)));
        if stored["notes"] != Value::Array(notes.clone()) {
            fail(format!(
                "notes {notes:?} against the stored {}",
                stored["notes"]
            ));
        }

        // The selection's log lines are the selection family's.
        let heading = |text: &str| log.iter().position(|line| line == text);
        let (Some(from), Some(to)) = (heading("\nSwing selection:"), heading("\nShot data:"))
        else {
            fail(format!("no selection or shot heading in {log:?}"));
            continue;
        };
        let case =
            read_gzipped(&repo().join(format!("spec/vectors/selection/corpus/{name}.json.gz")));
        let calls = case["input"]["calls"].as_array().unwrap();
        let index = calls
            .iter()
            .position(|call| call["fn"] == "_auto_windows")
            .expect("a corpus case ends in its _auto_windows call");
        let recorded = &case["expected"]["results"][index]["log"];
        if json!(log[from + 1..to]) != *recorded {
            fail(format!(
                "selection logged {:?}, the family recorded {recorded}",
                &log[from + 1..to]
            ));
        }
        println!(
            "{name:<20} windows {:?} {:?}; shot {shot_id} declared {seen_here:?}; {} notes",
            assembled.input.face_on_window,
            assembled.input.down_the_line_window,
            assembled.notes.len()
        );
    }

    if reached != committed_names {
        failures.push(format!(
            "the stored bundles reach {reached:?}, and the committed corpus is {committed_names:?}"
        ));
    }
    for (declared, count) in DECLARED_VALUES.iter().zip(&seen) {
        if *count == 0 {
            failures.push(format!(
                "{} {} {} -> {} was never seen: DECLARED_VALUES is out of date",
                declared.shot, declared.key, declared.stored, declared.reread
            ));
        }
    }
    println!(
        "{} bundles, {} photos read, declared values seen {seen:?}",
        reached.len(),
        reader.reads()
    );
    assert!(!sidecar.started(), "a stored view reached the pose sidecar");
    assert_eq!(
        reader.reads(),
        0,
        "a photo was read: every stored shot is current since P29"
    );
    assert_eq!(
        digests(scratch.path()),
        before,
        "the dry run wrote into its scratch copy"
    );
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
