//! The shot lookup over every stored bundle, once `data/` is Rust's: each stored shot stamped by
//! this parser and served from the store, with nothing read and nothing written. [M29 P23, P29]
//!
//! P23 built this gate as a dry-run re-read: frozen Python wrote every
//! `data/processed/shots/*.shot.json` unstamped (`parser_version` 0), so the lab's lookup read
//! each as stale, re-read its photo, and the gate held the re-read to the stored shot, value by
//! value and provenance line by line. P29 then wrote those re-reads (`golf-lab reanalyze --all`),
//! which left nothing stale to re-read, and the user's (b) at P29 made the gate check the store as
//! P29 leaves it rather than the backup of the store before it. **The comparison with frozen
//! Python's reads is not a gate any more**: it is the M29 plan's P23 and P25 findings, measured
//! before the write, and P29's step 3, measured across it.
//!
//! Per bundle, [`shot_for`] runs over the real store and the real bundles with the lab's own reader
//! ([`OcrReader::configured`]), as a dry run so that whatever went wrong it could not write, and:
//!
//! - **the stored shot is stamped by this parser**: [`parse_is_current`], `parser_version` exactly
//!   [`SCREEN_PARSER_VERSION`], and `fields_present` the bay layout's tiles, `Impact Position V`
//!   among them and `Bounce and Roll` not;
//! - **it is served from the store**: [`Origin::Stored`], the store's shot unchanged, no note, and
//!   the one log line `_shot_for` writes for a current shot;
//! - **the values P29 was declared to move have moved**: each of `common::DECLARED_VALUES` is
//!   the store's value on every bundle whose shot it names (`tests/inputs.rs` holds the same list
//!   against the committed corpus, from the other side).
//!
//! Then the reader read no photo, and the store's files hash as they did before.
//!
//! Needs `data/processed/` (gitignored), so it is ignored. It cannot pass with nothing to read: a
//! missing store or bundle directory fails, a bundle with no shot fails, and a declared value never
//! seen fails. The reader is built as the lab builds it, so a lookup that wrongly asked for a read
//! fails here by its origin whether or not the models are present, and by the count when they are.

mod common;

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use contracts::shot::{parse_is_current, SCREEN_PARSER_VERSION};
use golf_lab::shot::{shot_for, OcrReader, Origin, ShotOptions};
use serde_json::Value;
use storage::manifest::{load_manifest, manifest_path, Role, SwingManifest};
use storage::shot_store::{hash_image, ShotStore};

use common::{swing_dirs, DECLARED_VALUES};

fn processed() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/processed")
}

/// Every file in the store, by name, as its digest.
fn store_files(shots: &Path) -> BTreeMap<String, String> {
    assert!(shots.is_dir(), "{} is not a directory", shots.display());
    fs::read_dir(shots)
        .unwrap()
        .map(|entry| {
            let path = entry.unwrap().path();
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            (name, hash_image(&fs::read(&path).unwrap()))
        })
        .collect()
}

#[test]
#[ignore = "needs data/processed/ (gitignored), as the M29 plan's P29 wrote it"]
fn every_stored_shot_is_stamped_by_this_parser_and_served_from_the_store() {
    let processed = processed();
    let shots_dir = processed.join("shots");
    let before = store_files(&shots_dir);
    let store = ShotStore::new(&shots_dir);
    let options = ShotOptions {
        dry_run: true,
        ..ShotOptions::new(&shots_dir)
    };
    let mut reader = OcrReader::configured();
    let mut failures: Vec<String> = Vec::new();
    let mut seen = vec![0usize; DECLARED_VALUES.len()];
    let mut photos = BTreeSet::new();
    let mut bundles = 0;

    for swing_dir in swing_dirs(&processed.join("sessions")) {
        let manifest: SwingManifest = load_manifest(&manifest_path(&swing_dir)).unwrap();
        let bundle = format!("{}/{}", manifest.session_id, manifest.swing_id);
        let role = manifest
            .roles
            .get(&Role::ShotScreen)
            .unwrap_or_else(|| panic!("{bundle}: every stored bundle has a shot photo"));
        let sha = role.content_sha256.clone();
        photos.insert(sha.clone());
        bundles += 1;
        let stored = store
            .get(&sha)
            .unwrap()
            .unwrap_or_else(|| panic!("{bundle}: the store holds no shot for {sha}"));
        let mut fail =
            |what: String| failures.push(format!("{bundle} ({}): {what}", stored.shot_id));

        // Stamped by this parser.
        let provenance = stored.provenance.as_ref();
        if !parse_is_current(&stored)
            || provenance.map(|p| p.parser_version) != Some(SCREEN_PARSER_VERSION)
        {
            fail(format!(
                "not stamped by screen parser {SCREEN_PARSER_VERSION}: {:?}",
                provenance.map(|p| p.parser_version)
            ));
        }
        match provenance.and_then(|p| p.fields_present.as_ref()) {
            Some(present)
                if present.iter().any(|f| f == "impact_position_v")
                    && !present.iter().any(|f| f == "bounce_and_roll") => {}
            other => fail(format!(
                "fields_present {other:?}, not the bay layout stamped"
            )),
        }

        // Served from the store, with its one line.
        let mut lines = Vec::new();
        let answer = shot_for(&swing_dir, &manifest, &options, &mut reader, &mut |line| {
            lines.push(line.to_string())
        })
        .unwrap();
        if !matches!(answer.origin, Origin::Stored) {
            fail(format!("not served from the store: {:?}", answer.origin));
        }
        let want_log = [format!(
            "  shot_screen: {} from the shot store",
            stored.shot_id
        )];
        if lines != want_log {
            fail(format!("log {lines:?}"));
        }
        if answer.note.is_some() {
            fail(format!("note {:?}", answer.note));
        }
        if answer.shot.as_ref() != Some(&stored) {
            fail("the shot served is not the store's".to_string());
        }

        // The declared values, as the re-read left them.
        let values = serde_json::to_value(&stored).unwrap();
        for (index, declared) in DECLARED_VALUES.iter().enumerate() {
            if declared.shot != stored.shot_id {
                continue;
            }
            let want: Value = serde_json::from_str(declared.reread).unwrap();
            if values[declared.key] == want {
                seen[index] += 1;
            } else {
                fail(format!(
                    "{} is {}, and the re-read P29 wrote reads {want} (frozen Python's read {})",
                    declared.key, values[declared.key], declared.stored
                ));
            }
        }
        println!(
            "{bundle:<20} {:<14} parser {:?}; {} tiles present",
            stored.shot_id,
            provenance.map(|p| p.parser_version),
            provenance
                .and_then(|p| p.fields_present.as_ref())
                .map_or(0, Vec::len),
        );
    }

    for (declared, count) in DECLARED_VALUES.iter().zip(&seen) {
        if *count == 0 {
            failures.push(format!(
                "{} {} {} was never seen: DECLARED_VALUES is out of date",
                declared.shot, declared.key, declared.reread
            ));
        }
    }
    println!(
        "{bundles} bundles, {} photos, {} read; declared values seen {seen:?}",
        photos.len(),
        reader.reads()
    );
    assert_eq!(reader.reads(), 0, "a current stored shot was read again");
    assert_eq!(
        store_files(&shots_dir),
        before,
        "the dry run wrote into the store"
    );
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
