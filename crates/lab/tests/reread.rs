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
//!
//! # A re-read of a shot a person corrected (M38 P8)
//!
//! The rest of this file is the M38 plan's C12, which needs no `data/` and runs in `cargo test`. Each
//! shot starts where a phone leaves it: a photo entry opened, read and analysed, then reviewed through
//! P7's own `golf_core::session::correct_shot`, so what is carried is what a correction really writes
//! and not a test's idea of it. The reader replays P10's hand-worked `full-swing-grid` boxes, changed
//! where a later parser has to disagree with the first read. Every case runs both of C12's re-reads
//! ([`BOTH`]), and the ones that write through [`shot_for`] have a twin through [`import_photo`], since
//! those are the two writers. `tests/shot_lookup.rs` holds what is not about a correction.

mod common;

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use contracts::capability::analysed_view;
use contracts::club::ClubId;
use contracts::golfer::Handedness;
use contracts::shot::{parse_is_current, ShotData, SCREEN_PARSER_VERSION};
use contracts::swing::{ANALYSIS_VERSION, COMPARABLE_FROM};
use contracts::Timestamp;
use golf_core::session::{
    analyse_entry, correct_shot, open_photo_entry, Duplicates, NewPhotoEntry, Outcome, ShotDirs,
    ShotEdit,
};
use golf_lab::read_impact::Outcome as ImpactOutcome;
use golf_lab::shot::{
    import_photo, record_impact_outcome, shot_for, Imported, NewImport, OcrReader, Origin,
    PhotoReader, Prepared, ShotFor, ShotOptions,
};
use screen::profile::{load_profile, DeviceProfile};
use screen::ScreenInput;
use serde_json::Value;
use storage::bundle_store::SwingBundleStore;
use storage::corpus::EngineVersions;
use storage::golfer_store::GolferStore;
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

// ----------------------------------------------------------- a person's correction (M38 P8, C12)

const PHOTO: &[u8] = b"a stand-in for Full Swing's screen, which only the reader would open";
const SESSION: &str = "2026-11-04-phone-a1b2";
const SWING: &str = "1";

/// A reader answering with P10's hand-worked `full-swing-grid` boxes, however a test changed them.
struct Replay(Prepared);

impl Replay {
    fn first_read() -> Self {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../spec/vectors/screen/hand/full-swing-grid.json");
        let document: Value = serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
        let input: ScreenInput = serde_json::from_value(document["input"].clone()).unwrap();
        Replay(Prepared {
            boxes: input.boxes,
            notes: input.notes,
        })
    }

    /// The same photo as a later parser reads it: each `(text, as)` box's text replaced, and every
    /// one of them found, so a test cannot quietly stop disagreeing.
    fn reading(changed: &[(&str, &str)]) -> Self {
        let mut reader = Self::first_read();
        for (text, read_as) in changed {
            let found = reader.0.boxes.iter_mut().find(|found| found.text == *text);
            found.expect("a box the vector holds").text = read_as.to_string();
        }
        reader
    }

    /// The new read that does not locate a tile: its label's box is gone.
    fn without(label: &str) -> Self {
        let mut reader = Self::first_read();
        let before = reader.0.boxes.len();
        reader.0.boxes.retain(|found| found.text != label);
        assert!(reader.0.boxes.len() < before, "a box the vector holds");
        reader
    }

    fn nothing() -> Self {
        Replay(Prepared {
            boxes: Vec::new(),
            notes: Vec::new(),
        })
    }
}

impl PhotoReader for Replay {
    fn ready(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn prepare(&mut self, _photo: &Path, _profile: &DeviceProfile) -> Result<Prepared, String> {
        Ok(self.0.clone())
    }
}

/// The two re-reads C12 names.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Reread {
    /// An older parser's shot, which any run re-reads keeping its identity.
    Stale,
    /// `--force` over a current shot, which is read and stamped as a new import.
    Force,
}

const BOTH: [Reread; 2] = [Reread::Stale, Reread::Force];

fn typed(field: &str, printed: &str) -> ShotEdit {
    ShotEdit {
        field: field.to_string(),
        printed: Some(printed.to_string()),
        letter: None,
    }
}

fn cleared(field: &str) -> ShotEdit {
    ShotEdit {
        field: field.to_string(),
        printed: None,
        letter: None,
    }
}

fn versions() -> EngineVersions {
    EngineVersions {
        installed: ANALYSIS_VERSION,
        comparable_from: COMPARABLE_FROM,
    }
}

fn at(text: &str) -> Timestamp {
    Timestamp::parse(text).unwrap()
}

fn new_import() -> NewImport<'static> {
    NewImport {
        session_id: SESSION,
        shot_id: Some("2026-11-04-phone-a1b2-1"),
    }
}

/// A photo entry as a phone leaves it, in a scratch root.
struct Entry {
    _root: tempfile::TempDir,
    dirs: ShotDirs,
    manifest: SwingManifest,
    swing_dir: PathBuf,
    photo: PathBuf,
    digest: String,
}

impl Entry {
    /// Opened, read, analysed, and reviewed with `edits`, where an empty list is "reviewed, nothing
    /// to change" and leaves the map empty.
    fn reviewed(edits: &[ShotEdit]) -> Self {
        let root = tempfile::tempdir().unwrap();
        let dirs = ShotDirs {
            sessions: root.path().join("sessions"),
            golfers: root.path().join("golfers"),
            shots: root.path().join("shots"),
        };
        let opened = at("2026-11-04T20:00:00+00:00");
        GolferStore::new(&dirs.golfers)
            .get_or_create("Aaron", Handedness::Right, opened)
            .unwrap();
        let digest = hash_image(PHOTO);
        let manifest = open_photo_entry(
            &SwingBundleStore::new(&dirs.sessions),
            &NewPhotoEntry {
                session_id: SESSION,
                digest: &digest,
                bytes: PHOTO,
                original_filename: "IMG_2893.HEIC",
                modified: SystemTime::UNIX_EPOCH + Duration::from_secs(1_793_000_000),
                player_id: "aaron",
                club: ClubId::SevenIron,
                intent: None,
                floor: 0,
            },
            opened,
        )
        .unwrap();
        assert_eq!(manifest.swing_id, SWING);
        let swing_dir = dirs.sessions.join(SESSION).join(SWING);
        let photo = swing_dir.join(&manifest.roles[&Role::ShotScreen].filename);
        let entry = Entry {
            _root: root,
            dirs,
            manifest,
            swing_dir,
            photo,
            digest,
        };
        let first = entry.import(&entry.options(), &mut Replay::first_read());
        assert_eq!(first.status(), "parsed");
        let mut duplicates = Duplicates::new(&entry.dirs.sessions, versions());
        assert!(matches!(
            analyse_entry(&entry.manifest, &entry.dirs, &mut duplicates),
            Outcome::Written(_)
        ));
        entry.correct(edits).unwrap();
        entry
    }

    fn correct(&self, edits: &[ShotEdit]) -> Result<ShotData, String> {
        correct_shot(
            &self.dirs,
            SESSION,
            SWING,
            edits,
            versions(),
            at("2026-11-04T20:30:00+00:00"),
        )
    }

    fn options(&self) -> ShotOptions {
        ShotOptions {
            device: "full_swing".to_string(),
            ..ShotOptions::new(&self.dirs.shots)
        }
    }

    /// The options for `reread`, with the store made ready for it. Stale is by hand, since rule 4
    /// holds `SCREEN_PARSER_VERSION` still: this is what a bump makes of every stored shot.
    fn options_for(&self, reread: Reread) -> ShotOptions {
        match reread {
            Reread::Stale => {
                let mut stored = self.stored();
                stored.provenance.as_mut().unwrap().parser_version = 0;
                self.store().put(&stored).unwrap();
                self.options()
            }
            Reread::Force => ShotOptions {
                force_ocr: true,
                ..self.options()
            },
        }
    }

    fn store(&self) -> ShotStore {
        ShotStore::new(&self.dirs.shots)
    }

    fn stored(&self) -> ShotData {
        self.store().get(&self.digest).unwrap().unwrap()
    }

    fn stored_bytes(&self) -> Vec<u8> {
        fs::read(self.store().path_for(&self.digest)).unwrap()
    }

    /// `golf-lab analyze` and `reanalyze`'s writer.
    fn run(&self, options: &ShotOptions, reader: &mut Replay) -> (ShotFor, Vec<String>) {
        let mut lines = Vec::new();
        let answer = shot_for(
            &self.swing_dir,
            &self.manifest,
            options,
            reader,
            &mut |line| lines.push(line.to_string()),
        )
        .unwrap();
        (answer, lines)
    }

    /// `golf-lab import-photos` and `import-video`'s writer.
    fn import(&self, options: &ShotOptions, reader: &mut Replay) -> Imported {
        import_photo(
            &self.photo,
            new_import(),
            load_profile("full_swing").unwrap(),
            options,
            reader,
        )
        .unwrap()
    }
}

fn corrections(shot: &ShotData) -> Vec<(&str, &str)> {
    let provenance = shot.provenance.as_ref().unwrap();
    let map = provenance.corrections.iter();
    map.map(|(field, read)| (field.as_str(), read.as_str()))
        .collect()
}

/// How many times the shot lists `field`.
fn listed(shot: &ShotData, field: &str) -> usize {
    let provenance = shot.provenance.as_ref().unwrap();
    let present = provenance.fields_present.as_ref().unwrap();
    present.iter().filter(|found| *found == field).count()
}

/// What each re-read's answer says it was, which C12 leaves as it was: only a stale shot has an
/// older shot to replace, and `--force` is still a new import.
fn assert_origin(reread: Reread, origin: &Origin) {
    match (reread, origin) {
        (
            Reread::Stale,
            Origin::Read {
                replacing: Some(_),
                written: Some(_),
            },
        )
        | (
            Reread::Force,
            Origin::Read {
                replacing: None,
                written: Some(_),
            },
        ) => {}
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_re_read_keeps_a_persons_value_and_its_entry_and_every_other_field_takes_the_new_read() {
    for reread in BOTH {
        let entry = Entry::reviewed(&[typed("carry_distance", "130.5")]);
        let options = entry.options_for(reread);
        // The new read disagrees with the person, and with the first read on a field nobody touched.
        let mut reader = Replay::reading(&[("131", "132"), ("102.5", "103.5")]);
        let (answer, _) = entry.run(&options, &mut reader);
        assert_origin(reread, &answer.origin);
        let shot = answer.shot.unwrap();

        assert_eq!(shot.carry_distance, Some(130.5), "{reread:?}");
        assert_eq!(
            corrections(&shot),
            [("carry_distance", "131.0")],
            "{reread:?}: the entry is what the first read said, and not the new read's 132"
        );
        assert_eq!(shot.ball_speed, Some(103.5), "{reread:?}");
        assert_eq!(listed(&shot, "carry_distance"), 1, "{reread:?}");
        let provenance = shot.provenance.as_ref().unwrap();
        assert_eq!(provenance.parser_version, SCREEN_PARSER_VERSION);
        assert_eq!(
            provenance.raw_fields["Carry Yd"], "132",
            "{reread:?}: what the new read saw stays the new read's"
        );
        assert_eq!(
            entry.stored(),
            shot,
            "{reread:?}: the store holds the answer"
        );
    }
}

#[test]
fn import_photos_re_reads_keep_it_too() {
    for (reread, status) in [(Reread::Stale, "re-read"), (Reread::Force, "parsed")] {
        let entry = Entry::reviewed(&[typed("carry_distance", "130.5")]);
        let options = entry.options_for(reread);
        let mut reader = Replay::reading(&[("131", "132"), ("102.5", "103.5")]);
        let imported = entry.import(&options, &mut reader);
        assert_eq!(imported.status(), status);
        let shot = imported.shot().unwrap();
        assert_eq!(shot.carry_distance, Some(130.5), "{reread:?}");
        assert_eq!(corrections(shot), [("carry_distance", "131.0")]);
        assert_eq!(shot.ball_speed, Some(103.5), "{reread:?}");
        assert_eq!(&entry.stored(), shot, "{reread:?}");
    }
}

/// `sidespin` is one of the two keys a `None` does not serialize (`ShotData`'s `WRITTEN_WHEN_SET`), so
/// a clear of it is a key that is absent from the stored shot rather than a null.
#[test]
fn a_cleared_tile_stays_cleared_with_the_number_it_held_in_the_map() {
    for reread in BOTH {
        let entry = Entry::reviewed(&[cleared("carry_distance"), cleared("sidespin")]);
        let options = entry.options_for(reread);
        let (answer, _) = entry.run(&options, &mut Replay::first_read());
        let shot = answer.shot.unwrap();
        assert_eq!(shot.carry_distance, None, "{reread:?}");
        assert_eq!(shot.sidespin, None, "{reread:?}");
        assert_eq!(
            corrections(&shot),
            [("carry_distance", "131.0"), ("sidespin", "-4199.0")],
            "{reread:?}"
        );
        assert_eq!(
            shot.ball_speed,
            Some(102.5),
            "{reread:?}: the read still reads"
        );
        assert_eq!(listed(&shot, "carry_distance"), 1, "{reread:?}");
        assert_eq!(listed(&shot, "sidespin"), 1, "{reread:?}");
        assert_eq!(entry.stored(), shot, "{reread:?}");
    }
}

/// P7's finding 7: `analysed_view` blanks a reading outside `fields_present`, so a value carried with
/// no place in the list would be stored and then ignored.
#[test]
fn a_value_typed_for_a_tile_the_new_read_did_not_locate_stays_listed_and_is_analysed() {
    for reread in BOTH {
        // The premise, on a shot nobody corrected: this read does not locate the tile.
        let control = Entry::reviewed(&[]);
        let options = control.options_for(reread);
        let (answer, _) = control.run(&options, &mut Replay::without("CARRYYD"));
        let unlocated = answer.shot.unwrap();
        assert_eq!(unlocated.carry_distance, None, "{reread:?}");
        assert_eq!(listed(&unlocated, "carry_distance"), 0, "{reread:?}");

        let entry = Entry::reviewed(&[typed("carry_distance", "130.5")]);
        let options = entry.options_for(reread);
        let (answer, _) = entry.run(&options, &mut Replay::without("CARRYYD"));
        let shot = answer.shot.unwrap();
        assert_eq!(listed(&shot, "carry_distance"), 1, "{reread:?}");
        assert_eq!(corrections(&shot), [("carry_distance", "131.0")]);
        assert_eq!(
            analysed_view(&shot).unwrap().carry_distance,
            Some(130.5),
            "{reread:?}"
        );
    }
}

/// A person cleared what the first read located, so the same shot goes on saying the tile is there and
/// holds nothing, whatever a later parser could find. The shot's answer does not depend on the re-read.
#[test]
fn a_cleared_tile_the_new_read_did_not_locate_stays_listed() {
    for reread in BOTH {
        let entry = Entry::reviewed(&[cleared("carry_distance")]);
        let options = entry.options_for(reread);
        let (answer, _) = entry.run(&options, &mut Replay::without("CARRYYD"));
        let shot = answer.shot.unwrap();
        assert_eq!(shot.carry_distance, None, "{reread:?}");
        assert_eq!(listed(&shot, "carry_distance"), 1, "{reread:?}");
        assert_eq!(corrections(&shot), [("carry_distance", "131.0")]);
    }
}

/// `fields_present: None` is not recorded, so nothing says the value's tile was located, and the
/// value is still what a person typed for it.
#[test]
fn a_value_on_a_shot_with_no_list_is_listed_on_the_new_read() {
    let entry = Entry::reviewed(&[typed("carry_distance", "130.5")]);
    let options = entry.options_for(Reread::Stale);
    let mut older = entry.stored();
    older.provenance.as_mut().unwrap().fields_present = None;
    entry.store().put(&older).unwrap();

    let (answer, _) = entry.run(&options, &mut Replay::without("CARRYYD"));
    let shot = answer.shot.unwrap();
    assert_eq!(listed(&shot, "carry_distance"), 1);
    assert_eq!(analysed_view(&shot).unwrap().carry_distance, Some(130.5));
}

/// The map is the audit of the first read, and a parser coming round to the person's answer does not
/// make the first read something else.
#[test]
fn a_new_read_agreeing_with_the_person_leaves_the_entry() {
    for reread in BOTH {
        let entry = Entry::reviewed(&[typed("carry_distance", "130.5")]);
        let options = entry.options_for(reread);
        let (answer, _) = entry.run(&options, &mut Replay::reading(&[("131", "130.5")]));
        let shot = answer.shot.unwrap();
        assert_eq!(shot.carry_distance, Some(130.5), "{reread:?}");
        assert_eq!(
            corrections(&shot),
            [("carry_distance", "131.0")],
            "{reread:?}"
        );
    }
}

/// C12's last sentence: the review answered the first read's question, and this is a new one.
#[test]
fn a_new_flag_from_the_new_read_stands_beside_the_kept_value() {
    for reread in BOTH {
        let entry = Entry::reviewed(&[typed("carry_distance", "130.5")]);
        let before = entry.stored();
        let reviewed = before.provenance.as_ref().unwrap();
        assert!(!reviewed.needs_review && reviewed.warnings.is_empty());

        let options = entry.options_for(reread);
        let (answer, _) = entry.run(&options, &mut Replay::reading(&[("1.08", "2.08")]));
        let shot = answer.shot.unwrap();
        let provenance = shot.provenance.as_ref().unwrap();
        assert!(provenance.needs_review, "{reread:?}");
        assert!(
            provenance
                .warnings
                .iter()
                .any(|w| w.contains("smash factor 2.08")),
            "{reread:?}: {:?}",
            provenance.warnings
        );
        assert_eq!(shot.carry_distance, Some(130.5), "{reread:?}");
        assert_eq!(
            corrections(&shot),
            [("carry_distance", "131.0")],
            "{reread:?}"
        );
    }
}

/// `carry_impact_video` and `carry_corrections` run one after the other on a stale re-read, and both
/// reach `fields_present`.
#[test]
fn a_stale_re_read_keeps_a_correction_and_the_replays_offset_together() {
    let entry = Entry::reviewed(&[typed("carry_distance", "130.5")]);
    let options = entry.options_for(Reread::Stale);
    let mut older = entry.stored();
    record_impact_outcome(
        &mut older,
        &ImpactOutcome::Read(impact::ImpactReading {
            offset_h_mm: 4.2,
            contact_frame: 6,
            frames: 9,
            ball_center: [480.0, 360.0],
            ball_diameter_px: 24.0,
            heel: [430.0, 330.0],
            toe: [480.0, 360.0],
        }),
    );
    entry.store().put(&older).unwrap();

    let (answer, _) = entry.run(&options, &mut Replay::first_read());
    let shot = answer.shot.unwrap();
    assert_eq!(shot.carry_distance, Some(130.5));
    assert_eq!(shot.impact_offset_h, Some(4.2));
    assert_eq!(listed(&shot, "carry_distance"), 1);
    assert_eq!(listed(&shot, "impact_offset_h"), 1);
    assert_eq!(corrections(&shot), [("carry_distance", "131.0")]);
}

/// Today's output, worked from what it is: `screen::read`'s answer over the stale shot's identity,
/// stored. Both writers, to the byte, with the same lines.
#[test]
fn a_stale_re_read_of_a_shot_with_an_empty_map_moves_no_byte() {
    let changed: &[(&str, &str)] = &[("131", "132"), ("102.5", "103.5")];
    for by_import in [false, true] {
        let entry = Entry::reviewed(&[]);
        let options = entry.options_for(Reread::Stale);
        let stale = entry.stored();
        assert!(corrections(&stale).is_empty());
        let by_hand = screen::read(&ScreenInput {
            device: "full_swing".to_string(),
            boxes: Replay::reading(changed).0.boxes,
            notes: Vec::new(),
            shot_id: stale.shot_id.clone(),
            session_id: stale.session_id.clone(),
            timestamp: stale.timestamp,
            image_sha256: Some(entry.digest.clone()),
            image_path: stale.provenance.as_ref().unwrap().image_path.clone(),
            min_confidence: options.min_confidence,
        })
        .unwrap()
        .unwrap();
        let scratch = tempfile::tempdir().unwrap();
        let want = fs::read(ShotStore::new(scratch.path()).put(&by_hand).unwrap()).unwrap();

        if by_import {
            let imported = entry.import(&options, &mut Replay::reading(changed));
            assert_eq!(imported.status(), "re-read");
        } else {
            let (_, lines) = entry.run(&options, &mut Replay::reading(changed));
            assert_eq!(lines.last().unwrap(), "  shot_screen: parsed");
        }
        assert_eq!(entry.stored_bytes(), want, "by_import {by_import}");
    }
}

/// Today's `--force` does not look at the store, so its output is the output over an empty one.
#[test]
fn a_force_re_read_of_a_shot_with_an_empty_map_moves_no_byte() {
    let changed: &[(&str, &str)] = &[("131", "132"), ("102.5", "103.5")];
    for by_import in [false, true] {
        let entry = Entry::reviewed(&[]);
        let options = entry.options_for(Reread::Force);
        let read = |entry: &Entry| {
            let mut reader = Replay::reading(changed);
            if by_import {
                let imported = entry.import(&options, &mut reader);
                (imported.status().to_string(), Vec::new(), None)
            } else {
                let (answer, lines) = entry.run(&options, &mut reader);
                (format!("{:?}", answer.origin), lines, answer.note)
            }
        };
        fs::remove_file(entry.store().path_for(&entry.digest)).unwrap();
        let over_nothing = read(&entry);
        let want = entry.stored_bytes();

        let mut held = entry.stored();
        held.carry_distance = Some(99.9);
        entry.store().put(&held).unwrap();
        assert_eq!(read(&entry), over_nothing, "by_import {by_import}");
        assert_eq!(entry.stored_bytes(), want, "by_import {by_import}");
    }
}

/// Today's `--force` is how such a file gets replaced, as it never read it, so looking for corrections
/// must not make it stop the run as the lookup's `get` stops a run without `--force`.
#[test]
fn a_force_re_read_over_a_file_that_is_not_a_shot_replaces_it_as_it_did() {
    let entry = Entry::reviewed(&[]);
    let options = entry.options_for(Reread::Force);
    entry.run(&options, &mut Replay::first_read());
    let want = entry.stored_bytes();

    fs::write(entry.store().path_for(&entry.digest), "{not json").unwrap();
    let (answer, _) = entry.run(&options, &mut Replay::first_read());
    assert!(answer.shot.is_some());
    assert_eq!(entry.stored_bytes(), want);

    fs::write(entry.store().path_for(&entry.digest), "{not json").unwrap();
    let imported = entry.import(&options, &mut Replay::first_read());
    assert_eq!(imported.status(), "parsed");
    assert_eq!(entry.stored_bytes(), want);
}

/// The store holds one shot per photo, so a correction belongs to the bytes it was made on. A photo
/// that no longer hashes to its manifest's sha256 is not that photo (`shot.rs`' module doc).
#[test]
fn a_correction_follows_the_photos_bytes_and_not_its_manifest() {
    for reread in BOTH {
        let entry = Entry::reviewed(&[typed("carry_distance", "130.5")]);
        let options = entry.options_for(reread);
        let manifests = entry.stored_bytes();
        fs::write(&entry.photo, b"some other photo's bytes").unwrap();

        let (answer, _) = entry.run(&options, &mut Replay::first_read());
        let shot = answer.shot.unwrap();
        assert_eq!(shot.carry_distance, Some(131.0), "{reread:?}");
        assert!(corrections(&shot).is_empty(), "{reread:?}");
        assert!(answer.note.unwrap().contains("hashes to"), "{reread:?}");
        assert_eq!(
            entry.stored_bytes(),
            manifests,
            "{reread:?}: another photo's"
        );

        // Now a correction on those bytes' own shot, which is the one to keep.
        let mut own = shot;
        own.carry_distance = Some(140.0);
        let provenance = own.provenance.as_mut().unwrap();
        provenance
            .corrections
            .insert("carry_distance".to_string(), "131.0".to_string());
        if reread == Reread::Stale {
            provenance.parser_version = 0;
        }
        entry.store().put(&own).unwrap();
        let (answer, _) = entry.run(&options, &mut Replay::first_read());
        let shot = answer.shot.unwrap();
        assert_eq!(shot.carry_distance, Some(140.0), "{reread:?}");
        assert_eq!(
            corrections(&shot),
            [("carry_distance", "131.0")],
            "{reread:?}"
        );
    }
}

/// A read of nothing writes nothing (`shot.rs`' module doc), so the correction is still in the store
/// for the next read that does read something.
#[test]
fn a_re_read_of_nothing_leaves_the_corrected_shot_in_the_store() {
    for reread in BOTH {
        let entry = Entry::reviewed(&[typed("carry_distance", "130.5")]);
        let options = entry.options_for(reread);
        let before = entry.stored_bytes();
        let (answer, _) = entry.run(&options, &mut Replay::nothing());
        assert_eq!(answer.shot, None, "{reread:?}");
        assert_eq!(entry.stored_bytes(), before, "{reread:?}");
    }
}

#[test]
fn a_dry_run_answers_with_the_kept_value_and_writes_nothing() {
    for reread in BOTH {
        let entry = Entry::reviewed(&[typed("carry_distance", "130.5")]);
        let options = ShotOptions {
            dry_run: true,
            ..entry.options_for(reread)
        };
        let before = entry.stored_bytes();
        let (answer, _) = entry.run(&options, &mut Replay::reading(&[("131", "132")]));
        let shot = answer.shot.unwrap();
        assert_eq!(shot.carry_distance, Some(130.5), "{reread:?}");
        assert_eq!(
            corrections(&shot),
            [("carry_distance", "131.0")],
            "{reread:?}"
        );
        assert_eq!(entry.stored_bytes(), before, "{reread:?}");
    }
}

/// C11's restore still works on what a re-read leaves: the entry it carried is the first read's, so
/// typing that number back removes it. The value it lands on is the person's and not the new read's.
#[test]
fn a_carried_entry_is_still_removed_by_typing_the_first_read_back() {
    for reread in BOTH {
        let entry = Entry::reviewed(&[typed("carry_distance", "130.5")]);
        let options = entry.options_for(reread);
        let (answer, _) = entry.run(&options, &mut Replay::reading(&[("131", "132")]));
        assert_eq!(answer.shot.unwrap().carry_distance, Some(130.5));

        let restored = entry.correct(&[typed("carry_distance", "131")]).unwrap();
        assert_eq!(restored.carry_distance, Some(131.0), "{reread:?}");
        assert!(corrections(&restored).is_empty(), "{reread:?}");
    }
}
