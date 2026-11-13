//! `storage::drill_store`: the impact drill's history on disk. [M46 P17]
//!
//! **No vector and no Python twin** (the module doc), so this is the store's gate, on a scratch root
//! per test. It pins what the M46 plan's P17 asks for: C7's layout exactly, a strike and a state
//! round tripping, a strike id already held refused with the file untouched, a torn last line skipped
//! and counted and not swallowing the next strike, and two clubs kept apart. Beside those, what the
//! store refuses before writing anything, the strict state reader and its write guard, and the lock,
//! which is what makes an id's check and its append one step when two calls race.

use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Barrier;

use contracts::club::{category_of, ClubId};
use contracts::drill::{
    crop_file, label_id, strike_id, AppliedLabel, ClubDrillState, DrillStage, DrillState,
    LevelEntry, StrikeSource, Suggestion, TapeNotRead, TapeStrike,
};
use contracts::face_impact::TAPE_READER_VERSION;
use contracts::Timestamp;
use storage::drill_store::{DrillStore, StrikeLog};
use storage::StoreError;

const INSTALL: &str = "a1b2";
const PLAYER: &str = "aaron";

fn when() -> Timestamp {
    Timestamp::parse("2026-11-13T08:00:00Z").unwrap()
}

/// The `n`th strike, read in the baseline on `club`.
fn strike(n: i64, club: ClubId) -> TapeStrike {
    let id = strike_id(INSTALL, n);
    TapeStrike {
        crop: Some(crop_file(&id)),
        strike_id: id,
        player_id: PLAYER.into(),
        club,
        category: category_of(club),
        label_type: "strikepro_iron".into(),
        label_id: label_id(INSTALL, 1),
        n_on_label: n,
        captured_at: when(),
        stage: DrillStage::Baseline,
        level_radius_mm: None,
        call: None,
        offset_h_mm: Some(1.5 * n as f64),
        offset_v_mm: Some(-0.25),
        source: Some(StrikeSource::Read),
        not_read: None,
        reader_version: TAPE_READER_VERSION,
    }
}

/// The `n`th strike, not read: no position, no source and no crop.
fn not_read(n: i64, club: ClubId) -> TapeStrike {
    TapeStrike {
        offset_h_mm: None,
        offset_v_mm: None,
        source: None,
        not_read: Some(TapeNotRead::NoLabel),
        crop: None,
        ..strike(n, club)
    }
}

/// A seven iron on the ladder mid-block with a suggestion waiting, and a driver in its first
/// baseline: every field of a club's state, present and absent.
fn state() -> DrillState {
    let mut clubs = BTreeMap::new();
    clubs.insert(
        ClubId::SevenIron,
        ClubDrillState {
            stage: DrillStage::Ladder,
            level: Some(2),
            block: vec![strike_id(INSTALL, 11)],
            suggestion: Some(Suggestion::Recentre),
            history: vec![LevelEntry {
                at: when(),
                from: None,
                to: Some(2),
                block: (1..=10).map(|n| strike_id(INSTALL, n)).collect(),
            }],
            label: Some(AppliedLabel {
                label_id: label_id(INSTALL, 2),
                n_on_label: 3,
            }),
        },
    );
    clubs.insert(
        ClubId::Driver,
        ClubDrillState {
            stage: DrillStage::Baseline,
            level: None,
            block: Vec::new(),
            suggestion: None,
            history: Vec::new(),
            label: None,
        },
    );
    DrillState {
        player_id: PLAYER.into(),
        strikes_issued: 11,
        labels_issued: 2,
        clubs,
        updated_at: when(),
    }
}

fn open(root: &Path) -> DrillStore {
    DrillStore::open(root, PLAYER).unwrap()
}

/// Every file under `root`, relative and spelled with `/`, sorted.
fn files_under(root: &Path) -> Vec<String> {
    fn walk(root: &Path, dir: &Path, into: &mut Vec<String>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(root, &path, into);
            } else {
                let relative = path.strip_prefix(root).unwrap();
                let parts: Vec<_> = relative.iter().map(|p| p.to_str().unwrap()).collect();
                into.push(parts.join("/"));
            }
        }
    }
    let mut files = Vec::new();
    if root.exists() {
        walk(root, root, &mut files);
    }
    files.sort();
    files
}

/// Append raw text to `strikes.jsonl`, as a torn write or a hand edit would leave it.
fn append_raw(store: &DrillStore, text: &str) {
    fs::create_dir_all(store.dir()).unwrap();
    fs::File::options()
        .create(true)
        .append(true)
        .open(store.strikes_path())
        .unwrap()
        .write_all(text.as_bytes())
        .unwrap();
}

fn ids(log: &StrikeLog) -> Vec<&str> {
    log.strikes.iter().map(|s| s.strike_id.as_str()).collect()
}

// --- The layout -----------------------------------------------------------------------------------

#[test]
fn the_layout_is_c7s_paths_exactly_and_nothing_is_made_until_a_write() {
    let root = tempfile::tempdir().unwrap();
    let store = open(root.path());
    let dir = root
        .path()
        .join("drills")
        .join("aaron")
        .join("impact")
        .join("strike_tape");
    assert_eq!(store.dir(), dir);
    assert_eq!(store.strikes_path(), dir.join("strikes.jsonl"));
    assert_eq!(store.state_path(), dir.join("state.json"));
    assert_eq!(
        store.last_path(ClubId::SevenIron),
        dir.join("last").join("7i.rgb")
    );
    // Byte for byte, so the crop's path is spelled with the platform's separator throughout.
    assert_eq!(
        store.crop_path("a1b2-7").as_os_str(),
        dir.join("crops").join("a1b2-7.jpg").as_os_str()
    );
    assert!(files_under(root.path()).is_empty());

    store.append_strike(&strike(1, ClubId::SevenIron)).unwrap();
    store.put_state(&state()).unwrap();
    let crop = store.put_crop("a1b2-1", b"\xff\xd8 a jpeg").unwrap();
    store.put_last(ClubId::SevenIron, &[1, 2, 3]).unwrap();
    assert_eq!(crop, store.crop_path("a1b2-1"));
    // Every write is where C7 puts it, and none leaves its `.tmp` behind.
    assert_eq!(
        files_under(root.path()),
        [
            "drills/aaron/impact/strike_tape/crops/a1b2-1.jpg",
            "drills/aaron/impact/strike_tape/last/7i.rgb",
            "drills/aaron/impact/strike_tape/state.json",
            "drills/aaron/impact/strike_tape/strikes.jsonl",
        ]
    );
}

#[test]
fn a_player_id_that_is_not_a_slug_opens_no_store() {
    let root = tempfile::tempdir().unwrap();
    for bad in ["../aaron", "Dave Smith", "", "aaron/../dave"] {
        assert!(
            matches!(
                DrillStore::open(root.path(), bad),
                Err(StoreError::Invalid(_))
            ),
            "{bad:?}"
        );
    }
}

// --- The strikes ----------------------------------------------------------------------------------

#[test]
fn strikes_round_trip_in_the_order_appended_with_an_absent_value_as_an_absent_key() {
    let root = tempfile::tempdir().unwrap();
    let store = open(root.path());
    assert_eq!(
        store.all_strikes().unwrap(),
        StrikeLog::default(),
        "no file is an empty log"
    );

    let declared = TapeStrike {
        offset_h_mm: None,
        offset_v_mm: None,
        source: Some(StrikeSource::DeclaredOffTape),
        crop: None,
        ..strike(3, ClubId::SevenIron)
    };
    let written = [
        strike(1, ClubId::SevenIron),
        not_read(2, ClubId::SevenIron),
        declared,
    ];
    for one in &written {
        store.append_strike(one).unwrap();
    }
    let log = store.strikes(ClubId::SevenIron).unwrap();
    assert_eq!(log.strikes, written);
    assert_eq!(log.skipped, 0);

    // One compact object a line, each ending in a newline, and a strike not read holds no key for
    // what it does not have.
    let text = fs::read_to_string(store.strikes_path()).unwrap();
    let lines: Vec<&str> = text.split_terminator('\n').collect();
    assert_eq!(lines.len(), 3);
    assert!(text.ends_with('\n'));
    let second: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
    for absent in [
        "offset_h_mm",
        "offset_v_mm",
        "source",
        "crop",
        "call",
        "level_radius_mm",
    ] {
        assert!(second.get(absent).is_none(), "{absent}");
    }
    assert_eq!(second["not_read"], "no_label");
}

#[test]
fn a_strike_id_already_held_is_refused_and_the_file_is_untouched() {
    let root = tempfile::tempdir().unwrap();
    let store = open(root.path());
    store.append_strike(&strike(1, ClubId::SevenIron)).unwrap();
    let before = fs::read(store.strikes_path()).unwrap();

    // The same id on another club, with another position: still the same id.
    let again = TapeStrike {
        offset_h_mm: Some(-9.0),
        ..strike(1, ClubId::Driver)
    };
    match store.append_strike(&again) {
        Err(StoreError::Refused(message)) => {
            assert_eq!(
                message,
                "strike 'a1b2-1' is already stored; refusing to store it twice"
            );
        }
        other => panic!("expected a refusal, got {other:?}"),
    }
    assert_eq!(fs::read(store.strikes_path()).unwrap(), before);
    assert_eq!(ids(&store.all_strikes().unwrap()), ["a1b2-1"]);
}

#[test]
fn a_torn_last_line_is_skipped_and_counted_and_the_next_strike_starts_a_line_of_its_own() {
    let root = tempfile::tempdir().unwrap();
    let store = open(root.path());
    store.append_strike(&strike(1, ClubId::SevenIron)).unwrap();
    store.append_strike(&strike(2, ClubId::SevenIron)).unwrap();
    // A write of strike 3 cut off mid-line: no closing brace and no newline.
    let whole = serde_json::to_string(&strike(3, ClubId::SevenIron)).unwrap();
    append_raw(&store, &whole[..whole.len() / 2]);

    let log = store.strikes(ClubId::SevenIron).unwrap();
    assert_eq!(ids(&log), ["a1b2-1", "a1b2-2"]);
    assert_eq!(log.skipped, 1);

    // A torn line holds no id, so strike 3 is not already held, and it lands on a line of its own
    // rather than being glued to the fragment, where both would be lost.
    store.append_strike(&strike(3, ClubId::SevenIron)).unwrap();
    let log = store.strikes(ClubId::SevenIron).unwrap();
    assert_eq!(ids(&log), ["a1b2-1", "a1b2-2", "a1b2-3"]);
    assert_eq!(log.strikes[2], strike(3, ClubId::SevenIron));
    assert_eq!(log.skipped, 1);
    let text = fs::read_to_string(store.strikes_path()).unwrap();
    assert_eq!(text.split_terminator('\n').count(), 4);
}

#[test]
fn a_refused_line_another_golfers_and_a_second_id_are_skipped_and_counted_and_the_first_kept() {
    let root = tempfile::tempdir().unwrap();
    let store = open(root.path());
    let line = |strike: &TapeStrike| serde_json::to_string(strike).unwrap();
    let first = strike(1, ClubId::SevenIron);
    let mut unversioned = serde_json::to_value(strike(2, ClubId::SevenIron)).unwrap();
    unversioned["reader_version"] = 0.into();
    let dave = TapeStrike {
        player_id: "dave".into(),
        ..strike(3, ClubId::SevenIron)
    };
    let second_of_first = TapeStrike {
        offset_h_mm: Some(-9.0),
        ..first.clone()
    };
    append_raw(
        &store,
        &[
            line(&first),
            "not json".into(),
            unversioned.to_string(),
            line(&dave),
            String::new(),
            "   ".into(),
            line(&second_of_first),
            String::new(),
        ]
        .join("\n"),
    );

    let log = store.all_strikes().unwrap();
    assert_eq!(log.strikes, [first]);
    // Blank lines are newlines, not strikes, and are not counted.
    assert_eq!(log.skipped, 4);
}

/// The lab's merge reads a phone's files as bytes (P21), and the store's own rules are what it reads
/// them by: the same strikes and the same count skipped as the file read in place, and the strict
/// state reader's answer, refusals included. A store opened at the `drills/` directory is the store
/// opened at the root above it.
#[test]
fn another_copys_bytes_read_by_the_stores_own_rules() {
    let root = tempfile::tempdir().unwrap();
    let store = open(root.path());
    assert_eq!(
        DrillStore::in_drills(root.path().join("drills"), PLAYER)
            .unwrap()
            .dir(),
        store.dir()
    );
    assert!(matches!(
        DrillStore::in_drills(root.path(), "../aaron"),
        Err(StoreError::Invalid(_))
    ));

    store.append_strike(&strike(1, ClubId::SevenIron)).unwrap();
    store.append_strike(&strike(2, ClubId::Driver)).unwrap();
    let dave = TapeStrike {
        player_id: "dave".into(),
        ..strike(3, ClubId::SevenIron)
    };
    append_raw(
        &store,
        &format!(
            "{}\n{{\"strike_id\": ",
            serde_json::to_string(&dave).unwrap()
        ),
    );
    store.put_state(&state()).unwrap();

    // Read from bytes held anywhere, here a copy outside the store's directory.
    let copy = |path: &Path| fs::read(path).unwrap();
    let in_place = store.all_strikes().unwrap();
    assert_eq!(in_place.skipped, 2, "dave's line and the torn one");
    assert_eq!(store.parse_strikes(&copy(&store.strikes_path())), in_place);
    assert_eq!(
        store.parse_state(&copy(&store.state_path())),
        Ok(store.state().unwrap().unwrap())
    );

    let theirs = DrillStore::open(root.path(), "dave").unwrap();
    assert_eq!(
        ids(&theirs.parse_strikes(&copy(&store.strikes_path()))),
        ["a1b2-3"]
    );
    assert_eq!(
        theirs.parse_state(&copy(&store.state_path())),
        Err("it holds the drill of 'aaron', not of 'dave'".to_string())
    );
    assert!(store.parse_state(b"{\"player_id\": \"aaron\"").is_err());
}

#[test]
fn two_clubs_are_kept_apart() {
    let root = tempfile::tempdir().unwrap();
    let store = open(root.path());
    for (n, club) in [
        (1, ClubId::SevenIron),
        (2, ClubId::Driver),
        (3, ClubId::SevenIron),
        (4, ClubId::Driver),
        (5, ClubId::SevenIron),
    ] {
        store.append_strike(&strike(n, club)).unwrap();
    }
    append_raw(&store, "{\"strike_id\":");
    let iron = store.strikes(ClubId::SevenIron).unwrap();
    let driver = store.strikes(ClubId::Driver).unwrap();
    assert_eq!(ids(&iron), ["a1b2-1", "a1b2-3", "a1b2-5"]);
    assert_eq!(ids(&driver), ["a1b2-2", "a1b2-4"]);
    assert!(store
        .strikes(ClubId::PitchingWedge)
        .unwrap()
        .strikes
        .is_empty());
    // A torn line names no club, so each club's read counts it.
    assert_eq!((iron.skipped, driver.skipped), (1, 1));
    assert_eq!(
        ids(&store.all_strikes().unwrap()),
        ["a1b2-1", "a1b2-2", "a1b2-3", "a1b2-4", "a1b2-5"]
    );

    store.put_last(ClubId::SevenIron, &[7; 6]).unwrap();
    store.put_last(ClubId::Driver, &[1; 9]).unwrap();
    assert_eq!(store.last(ClubId::SevenIron).unwrap(), Some(vec![7; 6]));
    assert_eq!(store.last(ClubId::Driver).unwrap(), Some(vec![1; 9]));
    assert_eq!(store.last(ClubId::PitchingWedge).unwrap(), None);

    store.put_state(&state()).unwrap();
    let read = store.state().unwrap().unwrap();
    assert_eq!(
        read.clubs[&ClubId::SevenIron],
        state().clubs[&ClubId::SevenIron]
    );
    assert_eq!(read.clubs[&ClubId::Driver], state().clubs[&ClubId::Driver]);

    // And two golfers: another's drill is another directory.
    let dave = DrillStore::open(root.path(), "dave").unwrap();
    assert!(dave.all_strikes().unwrap().strikes.is_empty());
    assert_eq!(dave.state().unwrap(), None);
    assert_eq!(dave.last(ClubId::SevenIron).unwrap(), None);
}

#[test]
fn a_strike_the_store_cannot_take_writes_nothing() {
    let root = tempfile::tempdir().unwrap();
    let store = open(root.path());

    let neither = TapeStrike {
        not_read: None,
        ..not_read(1, ClubId::SevenIron)
    };
    assert!(matches!(
        store.append_strike(&neither),
        Err(StoreError::Invalid(_))
    ));

    let dave = TapeStrike {
        player_id: "dave".into(),
        ..strike(1, ClubId::SevenIron)
    };
    match store.append_strike(&dave) {
        Err(StoreError::Refused(message)) => assert_eq!(
            message,
            "a strike of 'dave' does not belong in the drill of 'aaron'"
        ),
        other => panic!("expected a refusal, got {other:?}"),
    }

    // The contract holds the crop to the id, so an id that climbs out of `crops/` is consistent
    // there and refused here.
    for bad in ["../a1b2-1", "a1b2/1", "a1b2.1", ""] {
        let climbing = TapeStrike {
            strike_id: bad.into(),
            crop: Some(crop_file(bad)),
            ..strike(1, ClubId::SevenIron)
        };
        assert!(
            matches!(store.append_strike(&climbing), Err(StoreError::Refused(_))),
            "{bad:?}"
        );
        assert!(
            matches!(store.put_crop(bad, b"jpeg"), Err(StoreError::Refused(_))),
            "{bad:?}"
        );
    }
    assert!(files_under(root.path()).is_empty());
}

// --- The state ------------------------------------------------------------------------------------

#[test]
fn the_state_round_trips_and_is_absent_before_the_first_write() {
    let root = tempfile::tempdir().unwrap();
    let store = open(root.path());
    assert_eq!(store.state().unwrap(), None);
    store.put_state(&state()).unwrap();
    assert_eq!(store.state().unwrap(), Some(state()));

    let mut later = state();
    later.strikes_issued = 12;
    later.clubs.remove(&ClubId::Driver);
    store.put_state(&later).unwrap();
    assert_eq!(store.state().unwrap(), Some(later));
}

#[test]
fn an_unreadable_state_is_an_error_and_is_never_written_over() {
    let root = tempfile::tempdir().unwrap();
    let store = open(root.path());
    fs::create_dir_all(store.dir()).unwrap();
    fs::write(
        store.state_path(),
        "{\"player_id\": \"aaron\", \"strikes_iss",
    )
    .unwrap();
    let before = fs::read(store.state_path()).unwrap();

    assert!(matches!(store.state(), Err(StoreError::Unreadable { .. })));
    match store.put_state(&state()) {
        Err(StoreError::Refused(message)) => {
            assert!(message.starts_with("drill state at "), "{message}");
            assert!(
                message.ends_with("exists but cannot be read; refusing to overwrite it"),
                "{message}"
            );
        }
        other => panic!("expected a refusal, got {other:?}"),
    }
    assert_eq!(fs::read(store.state_path()).unwrap(), before);

    // Another golfer's state in this golfer's directory reads as nobody's, and is not replaced.
    let dave = DrillState {
        player_id: "dave".into(),
        ..state()
    };
    fs::write(store.state_path(), serde_json::to_string(&dave).unwrap()).unwrap();
    assert!(matches!(store.state(), Err(StoreError::Unreadable { .. })));
    assert!(matches!(
        store.put_state(&state()),
        Err(StoreError::Refused(_))
    ));
}

#[test]
fn a_state_the_store_cannot_take_writes_nothing() {
    let root = tempfile::tempdir().unwrap();
    let store = open(root.path());

    // The putter has no ladder, so the contract refuses a state holding one.
    let mut putter = state();
    let driver = putter.clubs[&ClubId::Driver].clone();
    putter.clubs.insert(ClubId::Putter, driver);
    assert!(matches!(
        store.put_state(&putter),
        Err(StoreError::Invalid(_))
    ));

    let dave = DrillState {
        player_id: "dave".into(),
        ..state()
    };
    match store.put_state(&dave) {
        Err(StoreError::Refused(message)) => assert_eq!(
            message,
            "a state of 'dave' does not belong in the drill of 'aaron'"
        ),
        other => panic!("expected a refusal, got {other:?}"),
    }
    assert!(files_under(root.path()).is_empty());
}

// --- The crops and the last crop ------------------------------------------------------------------

#[test]
fn a_crop_and_a_last_crop_round_trip_byte_for_byte_and_are_replaced_whole() {
    let root = tempfile::tempdir().unwrap();
    let store = open(root.path());

    let jpeg: Vec<u8> = (0..=255).collect();
    let path = store.put_crop("a1b2-1", &jpeg).unwrap();
    assert_eq!(fs::read(&path).unwrap(), jpeg);

    assert_eq!(store.last(ClubId::SevenIron).unwrap(), None);
    let rgb: Vec<u8> = (0..3 * 40 * 20).map(|i| (i % 251) as u8).collect();
    store.put_last(ClubId::SevenIron, &rgb).unwrap();
    assert_eq!(store.last(ClubId::SevenIron).unwrap(), Some(rgb));
    store.put_last(ClubId::SevenIron, &[9, 9, 9]).unwrap();
    assert_eq!(store.last(ClubId::SevenIron).unwrap(), Some(vec![9, 9, 9]));
}

// --- The lock -------------------------------------------------------------------------------------

#[test]
fn racing_appends_store_each_id_once() {
    let root = tempfile::tempdir().unwrap();
    let (racers, strikes) = (8_usize, 12_i64);
    let start = Barrier::new(racers);
    let refused = AtomicUsize::new(0);
    std::thread::scope(|scope| {
        for racer in 0..racers {
            let (root, start, refused) = (root.path(), &start, &refused);
            scope.spawn(move || {
                // A store per call, as the facade opens one, so only a lock shared by every store
                // in the process keeps an id's check and its append together. Every racer offers
                // every strike, each from its own starting point, so they contend on every id.
                start.wait();
                for k in 0..strikes {
                    let n = (k + racer as i64) % strikes + 1;
                    let club = [ClubId::SevenIron, ClubId::Driver][racer % 2];
                    match open(root).append_strike(&strike(n, club)) {
                        Ok(()) => {}
                        Err(StoreError::Refused(_)) => {
                            refused.fetch_add(1, Ordering::SeqCst);
                        }
                        Err(other) => panic!("{other:?}"),
                    }
                }
            });
        }
    });
    let store = open(root.path());
    let log = store.all_strikes().unwrap();
    // A duplicate that slipped past the check would be a second line with an id, which a read skips
    // and counts, so `skipped` is what catches a lost race, and the raw line count backs it up.
    assert_eq!(log.skipped, 0);
    let text = fs::read_to_string(store.strikes_path()).unwrap();
    assert_eq!(text.split_terminator('\n').count(), strikes as usize);
    let mut held = ids(&log);
    held.sort_unstable();
    let mut expected: Vec<String> = (1..=strikes).map(|n| strike_id(INSTALL, n)).collect();
    expected.sort_unstable();
    assert_eq!(held, expected);
    assert_eq!(
        refused.load(Ordering::SeqCst),
        racers * strikes as usize - strikes as usize
    );
}
