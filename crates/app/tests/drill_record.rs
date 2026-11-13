//! `record_tape_photo`, `pick_mark` and `declare_off_tape`, driven as Dart drives them: a raw RGBA
//! file under `.app/tmp/`, strings in and a card out, against a scratch data root. [M46 P18]
//!
//! Two kinds of photo, because a drawn one costs a second or more to read in a debug build:
//!
//! - **Drawn**, by P9 to P13's renderer, included from `crates/impact/tests/tape_scene` by path so that
//!   there is one renderer, and read by the real reader: one test per outcome the reader gives, each
//!   checking what was stored, the crop, the club's last crop and the photo's deletion.
//! - **Stood in**: a tiny raw file and a reader that answers what the test says, through
//!   `app::drill::record_tape_photo_with`, for what does not depend on the pixels: the counting, a
//!   block judged at its tenth, two marks over a block's end, the waiting strikes and the refusals.

#[path = "../../impact/tests/tape_scene/mod.rs"]
mod tape_scene;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use app::api::drill::{
    declare_off_tape, pick_mark, record_tape_photo, DrillCall, TapeIn, TapeOutcome, TapeStatus,
};
use app::api::setup::{create_golfer, init_state, Handedness};
use app::drill::{record_tape_photo_with, Reading, PENDING_DIR};
use app::Dirs;
use contracts::club::ClubId;
use contracts::drill::{
    ClubDrillState, DrillStage, DrillState, StrikeCall, StrikeSource, Suggestion, TapeNotRead,
    TapeStrike,
};
use contracts::Timestamp;
use feedback::drill_rules::{baseline_tip, differential_result, ladder_result, NO_LABEL_FITS};
use impact::tape::{
    self, Crop, CropGrid, FoundLabel, LabelType, Orientation, TapeMark, TapePick, TapeReading,
    TapeRefusal, CROP_PX_PER_MM, STRIKEPRO_IRON,
};
use impact::Frame;
use storage::drill_store::DrillStore;
use tape_scene::{bottom_at, framed, truth, Hand, Mark, Scene, Shaft};
use tempfile::TempDir;

const INSTALL: &str = "a1b2";
const TAKEN: &str = "2026-11-13T09:00:00-05:00";
const NOW: &str = "2026-11-13T09:00:05-05:00";

/// Six places on the iron label, printed-frame millimetres, 12.6 mm or more apart and clear of the
/// red ring: `tape_diff.rs`'s run for the iron.
const PLACES: [[f64; 2]; 6] = [
    [-24.0, 3.0],
    [-11.0, -5.0],
    [1.0, 3.0],
    [13.0, -5.0],
    [25.0, 2.0],
    [-12.0, 9.0],
];

/// How far a drawn mark may be read from where it was drawn: P11's bound, which P12 and P13 kept.
const WITHIN_MM: f64 = 0.2;

/// A scratch data root with an install id and a right-handed golfer, as first-run setup leaves it.
struct Phone {
    _dir: TempDir,
    root: String,
}

static PHOTOS: AtomicUsize = AtomicUsize::new(0);

impl Phone {
    fn new() -> Phone {
        let dir = tempfile::tempdir().expect("a scratch directory");
        let root = dir.path().join("processed").display().to_string();
        init_state(root.clone(), INSTALL.to_string()).expect("an install id");
        create_golfer(
            root.clone(),
            "Aaron".to_string(),
            Handedness::Right,
            NOW.to_string(),
        )
        .expect("a golfer");
        Phone { _dir: dir, root }
    }

    fn tmp(&self) -> PathBuf {
        Dirs::of(&self.root).app.join("tmp")
    }

    /// `frame` as Swift writes it: raw RGBA under `.app/tmp/`, alpha opaque.
    fn raw(&self, frame: &Frame) -> String {
        fs::create_dir_all(self.tmp()).expect("tmp");
        let n = PHOTOS.fetch_add(1, Ordering::Relaxed);
        let path = self.tmp().join(format!("photo-{n}.rgba"));
        let rgba: Vec<u8> = frame
            .rgb
            .chunks(3)
            .flat_map(|pixel| [pixel[0], pixel[1], pixel[2], 255])
            .collect();
        fs::write(&path, rgba).expect("a raw photo");
        path.display().to_string()
    }

    fn input(&self, frame: &Frame, club: &str, call: Option<DrillCall>) -> TapeIn {
        TapeIn {
            root: self.root.clone(),
            player_id: "aaron".to_string(),
            club: club.to_string(),
            raw_path: self.raw(frame),
            width: frame.width as i64,
            height: frame.height as i64,
            captured_at: TAKEN.to_string(),
            now: NOW.to_string(),
            call,
        }
    }

    /// One drawn photo of the 7 iron, read by the real reader.
    fn shoot(&self, scene: &Scene) -> (TapeIn, TapeOutcome) {
        let input = self.input(&scene.draw(), "7i", None);
        let outcome =
            record_tape_photo(input.clone()).unwrap_or_else(|refused| panic!("{refused}"));
        (input, outcome)
    }

    /// One stood-in photo: the reader answers `reading`, whatever the pixels.
    fn stand_in(&self, club: &str, call: Option<DrillCall>, reading: Reading) -> TapeOutcome {
        self.try_stand_in(club, call, reading)
            .unwrap_or_else(|refused| panic!("{refused}"))
    }

    fn try_stand_in(
        &self,
        club: &str,
        call: Option<DrillCall>,
        reading: Reading,
    ) -> Result<TapeOutcome, String> {
        let input = self.input(&tiny(), club, call);
        record_tape_photo_with(input, &mut |_: &Frame,
                                            _: Option<&Frame>,
                                            _: &LabelType,
                                            _| {
            reading.clone()
        })
    }

    fn store(&self) -> DrillStore {
        DrillStore::open(&self.root, "aaron").expect("a drill store")
    }

    fn state(&self) -> DrillState {
        self.store()
            .state()
            .expect("a readable state")
            .expect("a stored state")
    }

    fn club(&self, club: ClubId) -> ClubDrillState {
        self.state().clubs[&club].clone()
    }

    fn strikes(&self) -> Vec<TapeStrike> {
        let log = self.store().all_strikes().expect("readable strikes");
        assert_eq!(log.skipped, 0);
        log.strikes
    }

    fn strike(&self, id: &str) -> TapeStrike {
        self.strikes()
            .into_iter()
            .find(|strike| strike.strike_id == id)
            .unwrap_or_else(|| panic!("no strike {id}"))
    }

    fn last(&self) -> Option<Vec<u8>> {
        self.store()
            .last(ClubId::SevenIron)
            .expect("a readable last crop")
    }

    fn pending(&self, id: &str) -> [PathBuf; 2] {
        let dir = Dirs::of(&self.root).app.join(PENDING_DIR);
        [
            dir.join(format!("{id}.json")),
            dir.join(format!("{id}.jpg")),
        ]
    }

    /// Puts the 7 iron where a test starts it, as an earlier session would have left it.
    fn seed(&self, club: ClubDrillState, strikes_issued: i64) {
        let mut clubs = std::collections::BTreeMap::new();
        clubs.insert(ClubId::SevenIron, club);
        self.store()
            .put_state(&DrillState {
                player_id: "aaron".into(),
                strikes_issued,
                labels_issued: 1,
                clubs,
                updated_at: Timestamp::parse(NOW).unwrap(),
            })
            .expect("a seeded state");
    }
}

fn tiny() -> Frame {
    Frame {
        width: 4,
        height: 2,
        rgb: vec![200; 4 * 2 * 3],
    }
}

/// The iron label, framed face-on at 200 mm on a right-handed club, logo up, with marks at `at`.
fn iron(at: &[[f64; 2]], turn: [f64; 3]) -> Scene {
    let mut scene = framed(&STRIKEPRO_IRON, Hand::Right, true, turn, 200.0);
    scene.marks = at
        .iter()
        .map(|&centre_mm| Mark {
            centre_mm,
            radius_mm: 5.0,
        })
        .collect();
    scene
}

fn iron_grid() -> CropGrid {
    CropGrid::new(&STRIKEPRO_IRON, CROP_PX_PER_MM)
}

/// The JPEG's size, from its baseline frame header, so a crop is checked without a decoder.
fn jpeg_size(path: &Path) -> (usize, usize) {
    let bytes = fs::read(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    assert_eq!(&bytes[..2], [0xFF, 0xD8], "{} is no JPEG", path.display());
    assert_eq!(&bytes[bytes.len() - 2..], [0xFF, 0xD9]);
    let at = bytes
        .windows(2)
        .position(|pair| pair == [0xFF, 0xC0])
        .expect("a baseline frame header");
    let height = u16::from_be_bytes([bytes[at + 5], bytes[at + 6]]) as usize;
    let width = u16::from_be_bytes([bytes[at + 7], bytes[at + 8]]) as usize;
    (width, height)
}

fn crop_of(phone: &Phone, id: &str) -> PathBuf {
    phone.store().crop_path(id)
}

/// The drawn place `at` in the reading frame, and how far `strike` was read from it.
fn off_by(scene: &Scene, strike: &TapeStrike, at: [f64; 2]) -> f64 {
    let [h, v] = truth(scene).to_reading_mm(at);
    let (read_h, read_v) = strike.position().expect("a position");
    (read_h - h).hypot(read_v - v)
}

// --- Drawn photos, the real reader ------------------------------------------------------------------

/// A fresh label read, then a photo missed, then a photo with nothing new declared off the tape:
/// four strikes, the counters behind each, and what each photo left on disk.
#[test]
fn a_fresh_label_then_two_new_marks_then_none_declared_off_the_tape() {
    let phone = Phone::new();
    let first = iron(&PLACES[..1], [0.0, 0.0, 0.0]);
    let (input, outcome) = phone.shoot(&first);
    assert_eq!(outcome.status, TapeStatus::Read, "{outcome:?}");
    assert!(!Path::new(&input.raw_path).exists(), "the photo stays");
    let one = phone.strike("a1b2-1");
    assert_eq!(one.source, Some(StrikeSource::Read));
    assert!(off_by(&first, &one, PLACES[0]) <= WITHIN_MM);
    assert_eq!((one.label_id.as_str(), one.n_on_label), ("a1b2-L1", 1));
    assert_eq!(
        (one.stage, one.level_radius_mm, one.call),
        (DrillStage::Baseline, None, None)
    );
    assert_eq!(one.label_type, "strikepro_iron");
    assert_eq!(one.reader_version, 1);
    let grid = iron_grid();
    assert_eq!(
        jpeg_size(&crop_of(&phone, "a1b2-1")),
        (grid.width, grid.height)
    );
    assert_eq!(
        outcome.crop_path,
        Some(crop_of(&phone, "a1b2-1").display().to_string())
    );
    let reference = phone.last().expect("the club's last crop");
    assert_eq!(reference.len(), grid.width * grid.height * 3);
    assert_eq!(outcome.strikes.len(), 1);
    assert_eq!(
        outcome.strikes[0].verdict, None,
        "the baseline has no circle"
    );
    assert_eq!(outcome.block_line, "Baseline: 1 of 10");
    assert_eq!(
        (outcome.message.as_deref(), outcome.retake_helps),
        (None, false)
    );
    let state = phone.state();
    assert_eq!((state.strikes_issued, state.labels_issued), (1, 1));
    assert_eq!(state.clubs[&ClubId::SevenIron].block, ["a1b2-1"]);

    // A photo missed: both new marks read, each its own strike, one place further on the label.
    let second = iron(&PLACES[..3], [6.0, -8.0, 4.0]);
    let (_, outcome) = phone.shoot(&second);
    assert_eq!(outcome.status, TapeStatus::Read, "{outcome:?}");
    let ids: Vec<&str> = outcome
        .strikes
        .iter()
        .map(|s| s.strike_id.as_str())
        .collect();
    assert_eq!(ids, ["a1b2-2", "a1b2-3"]);
    let (two, three) = (phone.strike("a1b2-2"), phone.strike("a1b2-3"));
    assert_eq!(
        [
            (two.label_id.as_str(), two.n_on_label),
            (three.label_id.as_str(), three.n_on_label)
        ],
        [("a1b2-L1", 2), ("a1b2-L1", 3)]
    );
    for at in [PLACES[1], PLACES[2]] {
        let nearest = [&two, &three]
            .iter()
            .map(|strike| off_by(&second, strike, at))
            .fold(f64::INFINITY, f64::min);
        assert!(nearest <= WITHIN_MM, "{at:?} read {nearest} mm off");
    }
    assert_eq!(outcome.block_line, "Baseline: 3 of 10");
    let after_two = phone.last().expect("the club's last crop");
    assert_ne!(after_two, reference, "the reference advanced");

    // Nothing new: the strike waits, counted nowhere, and the reference stays.
    let (input, outcome) = phone.shoot(&iron(&PLACES[..3], [-10.0, 5.0, -6.0]));
    assert_eq!(outcome.status, TapeStatus::NoNewMark, "{outcome:?}");
    assert!(!Path::new(&input.raw_path).exists());
    assert_eq!(outcome.pending_id.as_deref(), Some("a1b2-4"));
    assert_eq!(
        outcome.message.as_deref(),
        Some(TapeNotRead::NoNewMark.sentence())
    );
    assert!(outcome.retake_helps);
    assert!(outcome.strikes.is_empty());
    let waiting = PathBuf::from(outcome.crop_path.expect("the crop to tap"));
    assert_eq!(jpeg_size(&waiting), (grid.width, grid.height));
    assert_eq!(phone.strikes().len(), 3);
    assert_eq!(phone.last(), Some(after_two.clone()));
    assert_eq!(
        phone.state().strikes_issued,
        4,
        "the waiting strike's id is spent"
    );

    // Missed the tape: counted, no position, the crop kept, and the label's place not taken.
    let outcome = declare_off_tape(phone.root.clone(), "a1b2-4".into(), NOW.into()).unwrap();
    assert_eq!(outcome.status, TapeStatus::OffTape);
    assert_eq!(outcome.strikes[0].position, "off the tape");
    let four = phone.strike("a1b2-4");
    assert_eq!(four.source, Some(StrikeSource::DeclaredOffTape));
    assert_eq!((four.position(), four.not_read), (None, None));
    assert_eq!((four.label_id.as_str(), four.n_on_label), ("a1b2-L1", 4));
    assert!(crop_of(&phone, "a1b2-4").is_file());
    assert!(phone.pending("a1b2-4").iter().all(|path| !path.exists()));
    let club = phone.club(ClubId::SevenIron);
    assert_eq!(club.block, ["a1b2-1", "a1b2-2", "a1b2-3", "a1b2-4"]);
    assert_eq!(club.label.map(|label| label.n_on_label), Some(3));
    assert_eq!(outcome.block_line, "Baseline: 4 of 10");
    assert_eq!(phone.last(), Some(after_two));
}

/// Two marks on a label the club has no crop of cannot be told apart. The crop waits for a tap, the
/// reference and the label's place advance at once, and the tap lands where it was made.
#[test]
fn two_marks_on_a_fresh_label_wait_for_a_tap_that_is_stored_where_it_was_made() {
    let phone = Phone::new();
    let scene = iron(&PLACES[..2], [0.0, 0.0, 0.0]);
    let (input, outcome) = phone.shoot(&scene);
    assert_eq!(outcome.status, TapeStatus::NeedsPick, "{outcome:?}");
    assert!(!Path::new(&input.raw_path).exists());
    assert_eq!(outcome.pending_id.as_deref(), Some("a1b2-1"));
    assert_eq!(
        (outcome.message.as_deref(), outcome.retake_helps),
        (None, false)
    );
    assert!(phone.strikes().is_empty());
    assert!(phone.pending("a1b2-1").iter().all(|path| path.is_file()));
    assert!(
        phone.last().is_some(),
        "the mark is on the label either way"
    );
    let state = phone.state();
    assert_eq!((state.strikes_issued, state.labels_issued), (1, 1));

    let grid = iron_grid();
    let [px, py] = grid.to_crop_px(PLACES[1]);
    let outcome = pick_mark(
        phone.root.clone(),
        "a1b2-1".into(),
        px / grid.width as f64,
        py / grid.height as f64,
        NOW.into(),
    )
    .unwrap();
    assert_eq!(outcome.status, TapeStatus::Picked);
    let picked = phone.strike("a1b2-1");
    assert_eq!(picked.source, Some(StrikeSource::Picked));
    assert!(off_by(&scene, &picked, PLACES[1]) < 1e-9, "{picked:?}");
    assert!(outcome.strikes[0].position.ends_with("(picked)"));
    assert_eq!(
        (picked.label_id.as_str(), picked.n_on_label),
        ("a1b2-L1", 1)
    );
    assert!(crop_of(&phone, "a1b2-1").is_file());
    assert!(phone.pending("a1b2-1").iter().all(|path| !path.exists()));
    assert_eq!(phone.club(ClubId::SevenIron).block, ["a1b2-1"]);
    assert!(pick_mark(phone.root.clone(), "a1b2-1".into(), 0.5, 0.5, NOW.into()).is_err());
}

/// A mark the label's edge cuts is stored not read, with its crop, and never counted. Its mark is on
/// the label, so the reference and the label's place advance.
#[test]
fn a_clipped_mark_is_stored_not_read_and_advances_the_reference() {
    let phone = Phone::new();
    let edge = [17.0, bottom_at(&STRIKEPRO_IRON, 17.0) + 1.0];
    let (input, outcome) = phone.shoot(&iron(&[edge], [0.0, 0.0, 0.0]));
    assert_eq!(outcome.status, TapeStatus::NotRead, "{outcome:?}");
    assert!(!Path::new(&input.raw_path).exists());
    assert_eq!(
        outcome.message.as_deref(),
        Some(TapeNotRead::MarkClipped.sentence())
    );
    assert!(!outcome.retake_helps);
    assert_eq!(outcome.strikes[0].position, "not read");
    let clipped = phone.strike("a1b2-1");
    assert_eq!(clipped.not_read, Some(TapeNotRead::MarkClipped));
    assert!(crop_of(&phone, "a1b2-1").is_file());
    assert!(phone.last().is_some());
    let club = phone.club(ClubId::SevenIron);
    assert!(club.block.is_empty());
    assert_eq!(club.label.map(|label| label.n_on_label), Some(1));
    assert_eq!(outcome.block_line, "Baseline: 0 of 10");
}

/// No label found: the strike is stored not read with no crop, the photo is kept as a JPEG in
/// `.app/failed/` for the finder, and nothing on the label moves.
#[test]
fn a_photo_with_no_label_is_kept_as_a_jpeg_and_its_strike_stored_not_read() {
    let phone = Phone::new();
    let mut scene = iron(&[], [0.0, 0.0, 0.0]);
    scene.no_label = true;
    let (input, outcome) = phone.shoot(&scene);
    assert_eq!(outcome.status, TapeStatus::NotRead, "{outcome:?}");
    assert!(!Path::new(&input.raw_path).exists());
    assert_eq!(outcome.crop_path, None);
    let kept = Dirs::of(&phone.root).app.join("failed/tape-a1b2-1.jpg");
    assert_eq!(
        jpeg_size(&kept),
        (input.width as usize, input.height as usize)
    );
    let strike = phone.strike("a1b2-1");
    assert_eq!(
        (strike.not_read, strike.crop.as_deref()),
        (Some(TapeNotRead::NoLabel), None)
    );
    // The place the unseen mark would take, with nothing issued.
    assert_eq!(
        (strike.label_id.as_str(), strike.n_on_label),
        ("a1b2-L1", 1)
    );
    let state = phone.state();
    assert_eq!((state.strikes_issued, state.labels_issued), (1, 0));
    assert_eq!(state.clubs[&ClubId::SevenIron].label, None);
    assert_eq!(phone.last(), None);
}

/// No shaft, no sides: stored not read with the label's crop, and the reference stays, so a retake
/// framing the hosel still shows the strike.
#[test]
fn a_photo_with_no_shaft_is_stored_not_read_with_its_crop_and_leaves_the_reference() {
    let phone = Phone::new();
    let mut scene = iron(&PLACES[..1], [0.0, 0.0, 0.0]);
    scene.shaft = Shaft::Missing;
    let (_, outcome) = phone.shoot(&scene);
    assert_eq!(outcome.status, TapeStatus::NotRead, "{outcome:?}");
    assert_eq!(
        outcome.message.as_deref(),
        Some(TapeNotRead::NoOrientation.sentence())
    );
    assert!(outcome.retake_helps);
    assert_eq!(
        phone.strike("a1b2-1").not_read,
        Some(TapeNotRead::NoOrientation)
    );
    assert!(crop_of(&phone, "a1b2-1").is_file());
    assert_eq!(phone.last(), None);
    assert_eq!(phone.state().labels_issued, 0);
}

/// A raw file that is not the size it was said to be never reaches the reader.
#[test]
fn a_raw_file_of_the_wrong_size_is_stored_as_not_decoded() {
    let phone = Phone::new();
    let mut input = phone.input(&tiny(), "7i", None);
    input.width = 5;
    let outcome = record_tape_photo(input.clone()).unwrap();
    assert_eq!(outcome.status, TapeStatus::NotRead);
    assert!(!Path::new(&input.raw_path).exists());
    let strike = phone.strike("a1b2-1");
    assert_eq!(
        (strike.not_read, strike.crop),
        (Some(TapeNotRead::DecodeFailed), None)
    );
}

// --- Stood-in photos ----------------------------------------------------------------------------------

/// The iron label found, with a plain crop: what a stood-in reading carries.
fn found() -> FoundLabel {
    let grid = iron_grid();
    let pixels = grid.width * grid.height;
    FoundLabel {
        corners: [[10.0, 10.0], [90.0, 10.0], [90.0, 50.0], [10.0, 50.0]],
        crop: Crop {
            grid,
            frame: Frame {
                width: grid.width,
                height: grid.height,
                rgb: vec![236; pixels * 3],
            },
            outside: vec![false; pixels],
        },
    }
}

/// A right-handed club with its label logo-up.
const SIDES: Orientation = Orientation {
    toe_printed_right: false,
    top_line_printed_top: true,
};

fn read(marks: &[(f64, f64)]) -> Reading {
    Reading {
        outcome: tape::TapeOutcome::Read(TapeReading {
            found: found(),
            orientation: SIDES,
            new_label: false,
            marks: marks
                .iter()
                .map(|&(h, v)| TapeMark {
                    centroid_px: [0.0, 0.0],
                    offset_h_mm: h,
                    offset_v_mm: v,
                    area_mm2: 78.5,
                })
                .collect(),
        }),
        sides: Some(SIDES),
    }
}

fn needs_pick() -> Reading {
    Reading {
        outcome: tape::TapeOutcome::NeedsPick(TapePick {
            found: found(),
            orientation: SIDES,
        }),
        sides: Some(SIDES),
    }
}

fn refused(reason: TapeRefusal) -> Reading {
    let past_the_finder = !matches!(reason, TapeRefusal::NoLabel | TapeRefusal::DecodeFailed);
    Reading {
        outcome: tape::TapeOutcome::NotRead {
            reason,
            found: past_the_finder.then(found),
        },
        sides: (reason == TapeRefusal::NoNewMark).then_some(SIDES),
    }
}

/// Ten baseline distances, alternating heel and toe so that the pattern sits on centre: the 10 mm
/// circle holds five of them and the 9 mm circle four, so the mid-iron table starts the club at
/// its third level.
const BASELINE: [f64; 10] = [3.0, -4.0, 5.0, -8.5, 9.5, -11.0, 11.5, -13.0, 15.0, -20.0];

#[test]
fn a_baseline_judged_at_its_tenth_strike_sets_the_level_and_moves_the_club_to_the_ladder() {
    let phone = Phone::new();
    for (n, h) in BASELINE.iter().enumerate() {
        let outcome = phone.stand_in("7i", None, read(&[(*h, 0.0)]));
        if n < 9 {
            assert_eq!(outcome.block_line, format!("Baseline: {} of 10", n + 1));
            assert_eq!(outcome.block_result, None);
        } else {
            // R50 is the median distance: 9.5 and 11.0 are the fifth and sixth.
            assert_eq!(
                outcome.block_result,
                Some(baseline_tip(Some(10.25), 10.0, true))
            );
            assert_eq!(outcome.block_line, "Block: 0 of 10, 0 inside");
        }
    }
    let club = phone.club(ClubId::SevenIron);
    assert_eq!((club.stage, club.level), (DrillStage::Ladder, Some(2)));
    assert!(club.block.is_empty());
    assert_eq!(club.history.len(), 1);
    let entry = &club.history[0];
    assert_eq!((entry.from, entry.to), (None, Some(2)));
    assert_eq!(entry.block.len(), 10);
    assert_eq!(entry.at, Timestamp::parse(NOW).unwrap());
    assert_eq!(
        club.suggestion, None,
        "a centred baseline with a level suggests nothing"
    );

    let outcome = phone.stand_in("7i", None, read(&[(2.0, 0.0)]));
    let ladder = phone.strike("a1b2-11");
    assert_eq!(
        (ladder.stage, ladder.level_radius_mm),
        (DrillStage::Ladder, Some(10.0))
    );
    assert_eq!(outcome.strikes[0].verdict.as_deref(), Some("Inside 10 mm"));
    assert_eq!(outcome.block_line, "Block: 1 of 10, 1 inside");
}

/// C10 read literally (P14 finding 4): the first mark is the baseline's tenth, and the second opens
/// the ladder's first block, stamped with the circle the judgment left.
#[test]
fn two_marks_on_a_baseline_of_nine_close_it_and_open_the_next_block() {
    let phone = Phone::new();
    for h in &BASELINE[..9] {
        phone.stand_in("7i", None, read(&[(*h, 0.0)]));
    }
    let outcome = phone.stand_in("7i", None, read(&[(BASELINE[9], 0.0), (1.0, 1.0)]));
    assert_eq!(outcome.strikes.len(), 2);
    assert!(outcome.block_result.is_some());
    let (tenth, next) = (phone.strike("a1b2-10"), phone.strike("a1b2-11"));
    assert_eq!(
        (tenth.stage, tenth.level_radius_mm),
        (DrillStage::Baseline, None)
    );
    assert_eq!(
        (next.stage, next.level_radius_mm),
        (DrillStage::Ladder, Some(10.0))
    );
    let club = phone.club(ClubId::SevenIron);
    assert_eq!(
        club.history[0].block.last().map(String::as_str),
        Some("a1b2-10")
    );
    assert_eq!(club.block, ["a1b2-11"]);
    assert_eq!(outcome.block_line, "Block: 1 of 10, 1 inside");
}

fn at_level(stage: DrillStage, level: usize) -> ClubDrillState {
    ClubDrillState {
        stage,
        level: Some(level),
        block: Vec::new(),
        suggestion: None,
        history: Vec::new(),
        label: None,
    }
}

#[test]
fn a_ladder_block_with_seven_inside_moves_in_a_level() {
    let phone = Phone::new();
    phone.seed(at_level(DrillStage::Ladder, 2), 0);
    let mut outcome = None;
    for n in 0..10 {
        let h = if n < 7 { 2.0 } else { 12.0 };
        outcome = Some(phone.stand_in("7i", None, read(&[(h, 0.0)])));
    }
    let outcome = outcome.unwrap();
    assert_eq!(
        outcome.block_result,
        Some(ladder_result(7, 10, 10.0, 9.0, false, false))
    );
    let club = phone.club(ClubId::SevenIron);
    assert_eq!(club.level, Some(3));
    assert_eq!(
        (club.history[0].from, club.history[0].to),
        (Some(2), Some(3))
    );
}

/// In differential practice every photo carries its call, a strike is judged by it, and seven made
/// suggest a fresh baseline. Outside it, a call is refused, and so is a missing one inside it.
#[test]
fn a_differential_block_is_judged_by_its_calls_and_a_call_belongs_only_to_one() {
    let phone = Phone::new();
    let refused = phone.try_stand_in("7i", Some(DrillCall::Toe), read(&[(1.0, 0.0)]));
    assert!(refused.unwrap_err().contains("differential"));
    assert!(phone.store().all_strikes().unwrap().strikes.is_empty());

    phone.seed(at_level(DrillStage::Differential, 2), 0);
    let missing = phone.try_stand_in("7i", None, read(&[(1.0, 0.0)]));
    assert!(missing.unwrap_err().contains("call"));
    let mut outcome = None;
    for n in 0..10 {
        let h = if n < 7 { 12.0 } else { 1.0 };
        outcome = Some(phone.stand_in("7i", Some(DrillCall::Toe), read(&[(h, 0.0)])));
        if n == 0 {
            let card = &outcome.as_ref().unwrap().strikes[0];
            assert_eq!(card.verdict.as_deref(), Some("Toe call: made"));
        }
    }
    let outcome = outcome.unwrap();
    assert_eq!(outcome.block_result, Some(differential_result(7, 10, true)));
    let strike = phone.strike("a1b2-1");
    assert_eq!(strike.call, Some(StrikeCall::Toe));
    let club = phone.club(ClubId::SevenIron);
    assert_eq!(club.suggestion, Some(Suggestion::FreshBaseline));
    assert_eq!(club.level, Some(2));
}

/// The club's next photo stores a waiting strike as it stands: a retake after nothing new leaves
/// `no_new_mark`, and a pick nobody made leaves `not_picked`.
#[test]
fn the_next_photo_stores_a_waiting_strike_as_not_read() {
    let phone = Phone::new();
    let waiting = phone.stand_in("7i", None, refused(TapeRefusal::NoNewMark));
    assert_eq!(waiting.status, TapeStatus::NoNewMark);
    assert!(phone.pending("a1b2-1").iter().all(|path| path.is_file()));
    let retake = phone.stand_in("7i", None, read(&[(1.0, 0.0)]));
    assert_eq!(retake.strikes[0].strike_id, "a1b2-2");
    let discarded = phone.strike("a1b2-1");
    assert_eq!(discarded.not_read, Some(TapeNotRead::NoNewMark));
    assert!(crop_of(&phone, "a1b2-1").is_file());
    assert!(phone.pending("a1b2-1").iter().all(|path| !path.exists()));

    let pick = phone.stand_in("7i", None, needs_pick());
    assert_eq!(pick.pending_id.as_deref(), Some("a1b2-3"));
    phone.stand_in("7i", None, read(&[(2.0, 0.0)]));
    assert_eq!(
        phone.strike("a1b2-3").not_read,
        Some(TapeNotRead::NotPicked)
    );
    assert_eq!(phone.club(ClubId::SevenIron).block, ["a1b2-2", "a1b2-4"]);
    assert!(pick_mark(phone.root.clone(), "a1b2-3".into(), 0.5, 0.5, NOW.into()).is_err());
}

/// A tap on a `no_new_mark` is a strike landing on an old mark (P12 finding 9): it takes the
/// label's next place then, and not before.
#[test]
fn a_tap_on_a_strike_with_nothing_new_takes_the_labels_next_place() {
    let phone = Phone::new();
    phone.stand_in("7i", None, read(&[(1.0, 0.0)]));
    phone.stand_in("7i", None, refused(TapeRefusal::NoNewMark));
    assert_eq!(
        phone.club(ClubId::SevenIron).label.map(|l| l.n_on_label),
        Some(1)
    );
    let grid = iron_grid();
    let [px, py] = grid.to_crop_px([-6.0, 2.0]);
    let outcome = pick_mark(
        phone.root.clone(),
        "a1b2-2".into(),
        px / grid.width as f64,
        py / grid.height as f64,
        NOW.into(),
    )
    .unwrap();
    assert_eq!(outcome.status, TapeStatus::Picked);
    let picked = phone.strike("a1b2-2");
    // Printed left is the toe on a right-handed club, logo up.
    let (h, v) = picked.position().unwrap();
    assert!((h - 6.0).abs() < 1e-9 && (v - 2.0).abs() < 1e-9, "{h} {v}");
    assert_eq!(picked.n_on_label, 2);
    assert_eq!(
        phone.club(ClubId::SevenIron).label.map(|l| l.n_on_label),
        Some(2)
    );
}

/// Every reason the reader gives is stored under its own wire name, as `golf-lab read-tape` maps it.
/// `no_new_mark` waits, and is stored by the next photo.
#[test]
fn every_reason_the_reader_gives_is_stored_by_its_own_name() {
    for reason in TapeRefusal::ALL {
        let phone = Phone::new();
        let outcome = phone.stand_in("7i", None, refused(reason));
        if reason == TapeRefusal::NoNewMark {
            assert_eq!(outcome.status, TapeStatus::NoNewMark);
            phone.stand_in("7i", None, read(&[(1.0, 0.0)]));
        } else {
            assert_eq!(outcome.status, TapeStatus::NotRead, "{}", reason.as_str());
        }
        let stored = phone.strike("a1b2-1");
        let name = serde_json::to_value(stored.not_read.expect("a reason")).unwrap();
        assert_eq!(name, reason.as_str());
    }
}

#[test]
fn the_declarations_refuse_what_they_cannot_mean() {
    let phone = Phone::new();
    phone.stand_in("7i", None, needs_pick());
    let declared = declare_off_tape(phone.root.clone(), "a1b2-1".into(), NOW.into());
    assert!(declared.unwrap_err().contains("Tap the new mark"));
    assert!(phone.pending("a1b2-1").iter().all(|path| path.is_file()));
    for (x, y) in [(-0.1, 0.5), (0.5, 1.5), (f64::NAN, 0.5)] {
        assert!(pick_mark(phone.root.clone(), "a1b2-1".into(), x, y, NOW.into()).is_err());
    }
    for id in ["a1b2-9", "../a1b2-1", ""] {
        assert!(pick_mark(phone.root.clone(), id.into(), 0.5, 0.5, NOW.into()).is_err());
    }
    assert!(pick_mark(
        phone.root.clone(),
        "a1b2-1".into(),
        0.5,
        0.5,
        "2026-11-13T09:00:05".into()
    )
    .is_err());
    assert!(phone.strikes().is_empty());
}

/// One way a photo's input is wrong.
type Change = fn(&mut TapeIn);

/// Refused before anything is written, and the photo is gone all the same, since past the path's
/// check it is Rust's (C16).
#[test]
fn a_refused_photo_stores_nothing_and_is_removed() {
    let phone = Phone::new();
    let refusals: [(&str, Change); 4] = [
        ("putter", |input| input.club = "putter".into()),
        ("golfer", |input| input.player_id = "nobody".into()),
        ("offset", |input| input.now = "2026-11-13T09:00:05".into()),
        ("club", |input| input.club = "7 iron".into()),
    ];
    for (name, change) in refusals {
        let mut input = phone.input(&tiny(), "7i", None);
        change(&mut input);
        let refused = record_tape_photo_with(
            input.clone(),
            &mut |_: &Frame, _: Option<&Frame>, _: &LabelType, _| {
                panic!("{name}: the reader is not reached")
            },
        );
        let message = refused.expect_err(name);
        if name == "putter" {
            assert!(message.starts_with(NO_LABEL_FITS), "{message}");
        }
        assert!(!Path::new(&input.raw_path).exists(), "{name}");
    }
    assert!(!Path::new(&phone.root).join("drills").exists());
}

/// A path anywhere but `.app/tmp/` is refused and left alone, so a caller's bug cannot delete a
/// store's file.
#[test]
fn a_photo_outside_the_apps_tmp_is_refused_and_left_where_it_is() {
    let phone = Phone::new();
    let elsewhere = Path::new(&phone.root).join("photo.rgba");
    fs::write(&elsewhere, vec![255; 4 * 2 * 4]).unwrap();
    let mut input = phone.input(&tiny(), "7i", None);
    for path in [
        elsewhere.display().to_string(),
        phone
            .tmp()
            .join("..")
            .join("photo.rgba")
            .display()
            .to_string(),
    ] {
        input.raw_path = path.clone();
        assert!(record_tape_photo(input.clone()).is_err(), "{path}");
    }
    assert!(elsewhere.is_file());
}

/// A drill record that cannot be read is never read as a fresh one (P17 finding 4): the photo is
/// refused with a sentence and the record is left as it was.
#[test]
fn an_unreadable_drill_record_refuses_the_photo_and_is_left_alone() {
    let phone = Phone::new();
    let store = phone.store();
    fs::create_dir_all(store.dir()).unwrap();
    fs::write(store.state_path(), "{ torn").unwrap();
    let refused = phone.try_stand_in("7i", None, read(&[(1.0, 0.0)]));
    assert!(refused
        .unwrap_err()
        .starts_with("The drill's record on this phone cannot be read"));
    assert_eq!(fs::read_to_string(store.state_path()).unwrap(), "{ torn");
    assert!(!store.strikes_path().exists());
}

/// The stood-in sides are a right-handed club's with its label logo-up, as the drawn ones are.
#[test]
fn the_stood_in_sides_are_the_drawn_scenes() {
    assert_eq!(truth(&iron(&[], [0.0, 0.0, 0.0])), SIDES);
}
