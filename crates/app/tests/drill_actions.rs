//! `drill_clubs`, `start_drill`, `answer_suggestion` and `set_stage`, driven as Dart drives them,
//! against a scratch data root: one club walked through the whole drill, a stage changed mid-block,
//! and what each action refuses. [M46 P19]
//!
//! Every photo is stood in, through `app::drill::record_tape_photo_with`: a tiny raw file and a reader
//! that answers where the test says the mark landed. What is pinned here is the drill's course, not
//! the pixels, which `drill_record.rs` reads drawn.

use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};

use app::api::drill::{
    answer_suggestion, drill_clubs, pick_mark, set_stage, start_drill, DrillCall, DrillClubView,
    DrillStage as StageView, DrillView, TapeIn, TapeOutcome,
};
use app::api::setup::{create_golfer, init_state, set_bag_slot, Handedness};
use app::drill::{record_tape_photo_with, Reading};
use app::Dirs;
use contracts::club::ClubId;
use contracts::drill::{
    ClubDrillState, DrillStage, DrillState, StrikeCall, Suggestion, TapeNotRead, TapeStrike,
};
use contracts::intent::ClubCategory;
use contracts::Timestamp;
use feedback::drill_rules::{
    baseline_progress, baseline_tip, differential_progress, differential_result,
    differential_suggestion, fresh_baseline_suggestion, ladder_progress, ladder_result, level_line,
    recentre_progress, recentre_result, recentre_suggestion, return_to_ladder_suggestion,
    stage_name, DifferentialWhy, NO_LABEL_FITS, NO_LEVEL_YET,
};
use impact::tape::{
    self, Crop, CropGrid, FoundLabel, LabelType, Orientation, TapeMark, TapePick, TapeReading,
    CROP_PX_PER_MM, STRIKEPRO_IRON,
};
use impact::Frame;
use storage::drill_store::DrillStore;
use tempfile::TempDir;

const INSTALL: &str = "a1b2";
const TAKEN: &str = "2026-11-13T09:00:00-05:00";
const NOW: &str = "2026-11-13T09:00:05-05:00";

/// The 7 iron's table, `mid_iron`'s: 14, 12, 10, 9, 8, 7 and 6 mm.
const MID: ClubCategory = ClubCategory::MidIron;

/// Ten baseline strikes along the heel-toe axis, centred and wide: the 10 mm circle holds five and
/// the 9 mm circle four, so the club starts at its third level (`drill_record.rs`' baseline). Their
/// median distance is 10.25 mm.
const BASELINE: [f64; 10] = [3.0, -4.0, 5.0, -8.5, 9.5, -11.0, 11.5, -13.0, 15.0, -20.0];

/// A tight pattern 6 mm toward the toe, alternating so that its centre is exactly (6, 0): inside
/// every circle from 8 mm out.
const TOE: [(f64, f64); 2] = [(5.5, 0.5), (6.5, -0.5)];

/// A tight pattern on centre, exactly (0, 0).
const CENTRED: [(f64, f64); 2] = [(0.5, -0.5), (-0.5, 0.5)];

/// A wide pattern on centre, outside every circle of the table: each block moves out.
const WIDE: [(f64, f64); 2] = [(20.0, 0.0), (-20.0, 0.0)];

/// A scratch data root with an install id, a right-handed golfer and a bag, as setup leaves it.
struct Phone {
    _dir: TempDir,
    root: String,
}

static PHOTOS: AtomicUsize = AtomicUsize::new(0);

impl Phone {
    fn new(bag: &[&str]) -> Phone {
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
        for club in bag {
            set_bag_slot(
                root.clone(),
                "aaron".into(),
                club.to_string(),
                None,
                NOW.into(),
            )
            .expect("a bag slot");
        }
        Phone { _dir: dir, root }
    }

    /// One photo of the 7 iron, read as `reading` says, from a raw file Swift would have written.
    fn photo(&self, call: Option<DrillCall>, reading: Reading) -> TapeOutcome {
        let tmp = Dirs::of(&self.root).app.join("tmp");
        fs::create_dir_all(&tmp).expect("tmp");
        let raw = tmp.join(format!(
            "photo-{}.rgba",
            PHOTOS.fetch_add(1, Ordering::Relaxed)
        ));
        fs::write(&raw, [200u8; 4 * 2 * 4]).expect("a raw photo");
        let input = TapeIn {
            root: self.root.clone(),
            player_id: "aaron".into(),
            club: "7i".into(),
            raw_path: raw.display().to_string(),
            width: 4,
            height: 2,
            captured_at: TAKEN.into(),
            now: NOW.into(),
            call,
        };
        record_tape_photo_with(input, &mut |_: &Frame,
                                            _: Option<&Frame>,
                                            _: &LabelType,
                                            _| {
            reading.clone()
        })
        .unwrap_or_else(|refused| panic!("{refused}"))
    }

    /// Ten photos, the `n`th landing at `at(n)` with the call `call(n)`: one block, judged at its
    /// tenth. Answers the tenth's card, after checking that none of the first nine finished a block.
    fn block(
        &self,
        at: impl Fn(usize) -> (f64, f64),
        call: impl Fn(usize) -> Option<DrillCall>,
    ) -> TapeOutcome {
        for n in 0..9 {
            let card = self.photo(call(n), read(at(n)));
            assert_eq!(
                (card.block_result, card.suggestion),
                (None, None),
                "photo {n}"
            );
        }
        let last = self.photo(call(9), read(at(9)));
        assert!(last.block_result.is_some(), "the tenth judges the block");
        last
    }

    fn page(&self) -> DrillView {
        start_drill(self.root.clone(), "aaron".into(), "7i".into(), NOW.into())
            .unwrap_or_else(|refused| panic!("{refused}"))
    }

    fn answer(&self, accept: bool) -> DrillView {
        answer_suggestion(
            self.root.clone(),
            "aaron".into(),
            "7i".into(),
            accept,
            NOW.into(),
        )
        .unwrap_or_else(|refused| panic!("{refused}"))
    }

    fn set(&self, stage: StageView) -> Result<DrillView, String> {
        set_stage(
            self.root.clone(),
            "aaron".into(),
            "7i".into(),
            stage,
            NOW.into(),
        )
    }

    fn store(&self) -> DrillStore {
        DrillStore::open(&self.root, "aaron").expect("a drill store")
    }

    fn club(&self) -> ClubDrillState {
        self.store()
            .state()
            .expect("a readable state")
            .expect("a stored state")
            .clubs[&ClubId::SevenIron]
            .clone()
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

    /// The 7 iron on the ladder at `level`, as an earlier session would have left it.
    fn seed_ladder(&self, level: usize) {
        let mut clubs = std::collections::BTreeMap::new();
        clubs.insert(
            ClubId::SevenIron,
            ClubDrillState {
                stage: DrillStage::Ladder,
                level: Some(level),
                block: Vec::new(),
                suggestion: None,
                history: Vec::new(),
                label: None,
            },
        );
        self.store()
            .put_state(&DrillState {
                player_id: "aaron".into(),
                strikes_issued: 0,
                labels_issued: 0,
                clubs,
                updated_at: Timestamp::parse(NOW).unwrap(),
            })
            .expect("a seeded state");
    }
}

/// The iron label found, with a plain crop.
fn found() -> FoundLabel {
    let grid = CropGrid::new(&STRIKEPRO_IRON, CROP_PX_PER_MM);
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

fn read((h, v): (f64, f64)) -> Reading {
    Reading {
        outcome: tape::TapeOutcome::Read(TapeReading {
            found: found(),
            orientation: SIDES,
            new_label: false,
            marks: vec![TapeMark {
                centroid_px: [0.0, 0.0],
                offset_h_mm: h,
                offset_v_mm: v,
                area_mm2: 78.5,
            }],
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

fn no_call(_: usize) -> Option<DrillCall> {
    None
}

/// The `n`th of a block alternating between two places.
fn alternating(pair: [(f64, f64); 2]) -> impl Fn(usize) -> (f64, f64) {
    move |n| pair[n % 2]
}

fn baseline_at(n: usize) -> (f64, f64) {
    (BASELINE[n], 0.0)
}

/// The page the 7 iron should show: `level` as its level line, and the rest as given.
fn page(
    stage: DrillStage,
    level: Option<usize>,
    block_line: String,
    suggestion: Option<String>,
) -> DrillView {
    let view = match stage {
        DrillStage::Baseline => StageView::Baseline,
        DrillStage::Ladder => StageView::Ladder,
        DrillStage::Recentre => StageView::Recentre,
        DrillStage::Differential => StageView::Differential,
    };
    DrillView {
        club: "7i".into(),
        stage: view,
        stage_name: stage_name(stage).into(),
        level_line: level.map_or(NO_LEVEL_YET.to_string(), |level| {
            level_line(level, MID).expect("a level on the table")
        }),
        block_line,
        suggestion,
        call_wanted: stage == DrillStage::Differential,
    }
}

/// The level each judged block left the club at, oldest first.
fn levels(phone: &Phone) -> Vec<Option<usize>> {
    phone.club().history.iter().map(|entry| entry.to).collect()
}

// --- The whole drill ----------------------------------------------------------------------------------

/// One club through the cycle the section names: baseline, then the ladder, a recentre suggestion
/// declined and offered again after the next block, recentring, the ladder again, differential
/// practice, and a fresh baseline that the ladder starts from. At each step the page, the card's
/// suggestion and the stored state agree.
#[test]
fn a_club_walks_the_whole_drill_from_baseline_to_a_fresh_baseline() {
    let phone = Phone::new(&["7i"]);

    // Opened: the start of a baseline, and nothing written.
    assert_eq!(
        phone.page(),
        page(DrillStage::Baseline, None, baseline_progress(0), None)
    );
    assert!(!phone.store().state_path().exists());

    // The baseline sets the third level, 10 mm, and suggests nothing.
    let card = phone.block(baseline_at, no_call);
    assert_eq!(
        card.block_result,
        Some(baseline_tip(Some(10.25), 10.0, true))
    );
    assert_eq!(card.suggestion, None);
    assert_eq!(
        phone.page(),
        page(DrillStage::Ladder, Some(2), ladder_progress(0, 0), None)
    );

    // Ten tight toe strikes inside 10 mm move the club in. Half the window is still the centred
    // baseline, so the pattern's centre, 2.4 mm toe, is under a third of 9 mm.
    let card = phone.block(alternating(TOE), no_call);
    assert_eq!(
        card.block_result,
        Some(ladder_result(10, 10, 10.0, 9.0, false, false))
    );
    assert_eq!(card.suggestion, None);

    // Ten more, and the window is all toe: centred 6.0 mm out, more than a third of 8 mm.
    let recentre = recentre_suggestion((6.0, 0.0), true, false);
    let card = phone.block(alternating(TOE), no_call);
    assert_eq!(card.suggestion.as_ref(), Some(&recentre));
    assert_eq!(phone.club().suggestion, Some(Suggestion::Recentre));
    assert_eq!(
        phone.page(),
        page(
            DrillStage::Ladder,
            Some(4),
            ladder_progress(0, 0),
            Some(recentre.clone())
        )
    );

    // Declined: cleared, and nothing else moves.
    assert_eq!(
        phone.answer(false),
        page(DrillStage::Ladder, Some(4), ladder_progress(0, 0), None)
    );
    assert_eq!(phone.club().suggestion, None);

    // Offered again after the next block, because it still holds.
    let card = phone.block(alternating(TOE), no_call);
    assert_eq!(card.suggestion.as_ref(), Some(&recentre));
    assert_eq!(phone.page().suggestion, Some(recentre));

    // Accepted: recentring, at the level the ladder reached.
    assert_eq!(
        phone.answer(true),
        page(DrillStage::Recentre, Some(5), recentre_progress(0), None)
    );

    // A centred recentre block meets its goal, and suggests the ladder again.
    let back = return_to_ladder_suggestion(7.0);
    let card = phone.block(alternating(CENTRED), no_call);
    assert_eq!(
        card.block_result,
        Some(recentre_result(Some((0.0, 0.0)), true, 7.0))
    );
    assert_eq!(card.suggestion.as_ref(), Some(&back));
    assert_eq!(
        phone.answer(true),
        page(DrillStage::Ladder, Some(5), ladder_progress(0, 0), None)
    );

    // Wide blocks move the club out to the widest circle. The one that arrives there was not at
    // it, so only the two after it make the row that suggests differential practice.
    for _ in 0..6 {
        assert_eq!(phone.block(alternating(WIDE), no_call).suggestion, None);
    }
    let differential = differential_suggestion(DifferentialWhy::AtWidest, 14.0);
    let card = phone.block(alternating(WIDE), no_call);
    assert_eq!(
        card.block_result,
        Some(ladder_result(0, 10, 14.0, 14.0, false, true))
    );
    assert_eq!(card.suggestion.as_ref(), Some(&differential));
    assert_eq!(
        levels(&phone)[5..],
        [4, 3, 2, 1, 0, 0, 0].map(Some),
        "after the recentre block"
    );

    // Accepted: calls wanted, judged against the widest circle.
    assert_eq!(
        phone.answer(true),
        page(
            DrillStage::Differential,
            Some(0),
            differential_progress(0, 0),
            None
        )
    );

    // Eight of ten toe calls made at 16 mm toe, and two heel calls missed there.
    let call = |n: usize| {
        Some(if n < 8 {
            DrillCall::Toe
        } else {
            DrillCall::Heel
        })
    };
    let card = phone.block(|_| (16.0, 0.0), call);
    assert_eq!(card.block_result, Some(differential_result(8, 10, true)));
    assert_eq!(card.suggestion, Some(fresh_baseline_suggestion(8, 10)));
    assert_eq!(phone.strike("a1b2-121").call, Some(StrikeCall::Toe));

    // A fresh baseline keeps the level until it is judged.
    assert_eq!(
        phone.answer(true),
        page(DrillStage::Baseline, Some(0), baseline_progress(0), None)
    );

    // And the ladder starts from it.
    let card = phone.block(baseline_at, no_call);
    assert_eq!(
        card.block_result,
        Some(baseline_tip(Some(10.25), 10.0, true))
    );
    assert_eq!(
        phone.page(),
        page(DrillStage::Ladder, Some(2), ladder_progress(0, 0), None)
    );
    let club = phone.club();
    let last = club.history.last().expect("the fresh baseline's entry");
    assert_eq!((last.from, last.to), (Some(0), Some(2)));
    assert_eq!(club.history.len(), 14);
    assert_eq!(phone.strikes().len(), 140);

    let listed = drill_clubs(phone.root.clone(), "aaron".into()).expect("the clubs");
    assert_eq!(listed[0].stage, Some(StageView::Ladder));
    assert_eq!(listed[0].line, level_line(2, MID).unwrap());
}

/// A baseline whose widest circle holds fewer than half its strikes leaves the club at the widest
/// (the user's rule, the plan's P1 finding 5) and suggests differential practice for that reason,
/// which can be accepted and is judged at the widest circle.
#[test]
fn a_baseline_that_misses_the_widest_circle_suggests_differential_practice_there() {
    let phone = Phone::new(&["7i"]);
    let card = phone.block(alternating(WIDE), no_call);
    assert_eq!(
        card.block_result,
        Some(baseline_tip(Some(20.0), 14.0, false))
    );
    let why = differential_suggestion(DifferentialWhy::Baseline, 14.0);
    assert_eq!(card.suggestion.as_ref(), Some(&why));
    assert_eq!(
        phone.page(),
        page(
            DrillStage::Ladder,
            Some(0),
            ladder_progress(0, 0),
            Some(why.clone())
        )
    );

    // The stage the club is in changes nothing, the suggestion included.
    assert_eq!(phone.set(StageView::Ladder).unwrap().suggestion, Some(why));

    assert!(phone.answer(true).call_wanted);
    let card = phone.photo(Some(DrillCall::Toe), read((16.0, 0.0)));
    assert_eq!(card.strikes[0].verdict.as_deref(), Some("Toe call: made"));
    assert_eq!(phone.strike("a1b2-11").level_radius_mm, Some(14.0));
}

/// The suggestion's words are worked again from the record each time the page is asked for, and
/// still describe the block that raised it, however many strikes have landed since without an answer.
#[test]
fn a_waiting_suggestion_keeps_the_words_of_the_block_that_raised_it() {
    let phone = Phone::new(&["7i"]);
    phone.block(baseline_at, no_call);
    phone.block(alternating(TOE), no_call);
    let card = phone.block(alternating(TOE), no_call);
    let recentre = recentre_suggestion((6.0, 0.0), true, false);
    assert_eq!(card.suggestion.as_ref(), Some(&recentre));

    // Five heel strikes would put the pattern's centre at 3.0 mm toe.
    for _ in 0..5 {
        let card = phone.photo(None, read((-6.0, 0.0)));
        assert_eq!(card.suggestion, None, "a block in progress raises nothing");
    }
    let page = phone.page();
    assert_eq!(page.suggestion, Some(recentre));
    assert_eq!(page.block_line, ladder_progress(5, 5));
}

// --- The golfer's own change of stage -------------------------------------------------------------------

/// Changing stage mid-block abandons the block: its strikes stay stored and are never judged, the
/// waiting strike is stored first, and the next strike opens a block of the new stage.
#[test]
fn a_stage_change_mid_block_abandons_the_block_and_stores_the_waiting_strike() {
    let phone = Phone::new(&["7i"]);
    phone.seed_ladder(2);
    for _ in 0..4 {
        phone.photo(None, read((2.0, 0.0)));
    }
    let waiting = phone.photo(None, needs_pick());
    assert_eq!(waiting.pending_id.as_deref(), Some("a1b2-5"));
    assert_eq!(waiting.block_line, ladder_progress(4, 4));

    assert_eq!(
        phone.set(StageView::Recentre),
        Ok(page(
            DrillStage::Recentre,
            Some(2),
            recentre_progress(0),
            None
        ))
    );
    let club = phone.club();
    assert!(club.block.is_empty());
    assert!(
        club.history.is_empty(),
        "an abandoned block is never judged"
    );
    let strikes = phone.strikes();
    assert_eq!(strikes.len(), 5, "the block's strikes stay stored");
    assert!(strikes[..4]
        .iter()
        .all(|strike| strike.stage == DrillStage::Ladder));
    assert_eq!(strikes[4].not_read, Some(TapeNotRead::NotPicked));
    assert!(pick_mark(phone.root.clone(), "a1b2-5".into(), 0.5, 0.5, NOW.into()).is_err());

    // The next strike is the recentre block's first, against the same circle.
    phone.photo(None, read((1.0, 0.0)));
    let next = phone.strike("a1b2-6");
    assert_eq!(
        (next.stage, next.level_radius_mm),
        (DrillStage::Recentre, Some(10.0))
    );
    assert_eq!(phone.club().block, ["a1b2-6"]);

    // The stage it is in changes nothing and writes nothing.
    let state = fs::read(phone.store().state_path()).unwrap();
    assert_eq!(
        phone.set(StageView::Recentre).unwrap().block_line,
        recentre_progress(1)
    );
    assert_eq!(fs::read(phone.store().state_path()).unwrap(), state);

    // The block judged at its tenth holds the recentre strikes alone.
    for _ in 0..9 {
        phone.photo(None, read((0.5, 0.0)));
    }
    let club = phone.club();
    assert_eq!(club.history.len(), 1);
    let ids: Vec<String> = (6..=15).map(|n| format!("a1b2-{n}")).collect();
    assert_eq!(club.history[0].block, ids);
    assert_eq!(
        (club.history[0].from, club.history[0].to),
        (Some(2), Some(2))
    );
}

/// The golfer's own stage change clears a waiting suggestion, which is then not there to answer.
#[test]
fn a_stage_change_clears_a_waiting_suggestion() {
    let phone = Phone::new(&["7i"]);
    phone.block(alternating(WIDE), no_call);
    assert_eq!(phone.club().suggestion, Some(Suggestion::Differential));
    let page = phone.set(StageView::Recentre).unwrap();
    assert_eq!((page.stage, page.suggestion), (StageView::Recentre, None));
    assert_eq!(phone.club().suggestion, None);
    let refused = answer_suggestion(
        phone.root.clone(),
        "aaron".into(),
        "7i".into(),
        true,
        NOW.into(),
    );
    assert!(refused.unwrap_err().contains("No suggestion"));
}

/// Before its first baseline sets a level, a club cannot leave the baseline, and the refusal writes
/// nothing. A baseline that was judged always leaves a level, so this is the only such case.
#[test]
fn a_club_cannot_leave_its_first_baseline_before_it_sets_a_level() {
    let phone = Phone::new(&["7i"]);
    for stage in [
        StageView::Ladder,
        StageView::Recentre,
        StageView::Differential,
    ] {
        let refused = phone.set(stage).expect_err("no level yet");
        assert!(refused.contains("baseline"), "{refused}");
    }
    assert!(!phone.store().state_path().exists());
    assert_eq!(
        phone.set(StageView::Baseline),
        Ok(page(DrillStage::Baseline, None, baseline_progress(0), None))
    );
    assert!(!phone.store().state_path().exists());
}

// --- The clubs ---------------------------------------------------------------------------------------

/// Every bag club in bag order, with its category and label; the putter is listed and cannot start;
/// and a club is listed as started once it has a strike, not once it is opened.
#[test]
fn drill_clubs_lists_the_bag_with_the_putter_unable_to_start() {
    let phone = Phone::new(&["putter", "7i", "driver"]);
    let fresh = |club: &str, category: &str, label: &str| DrillClubView {
        club: club.into(),
        category: category.into(),
        label_type: Some(label.into()),
        can_start: true,
        stage: None,
        stage_name: None,
        line: NO_LEVEL_YET.into(),
    };
    let putter = DrillClubView {
        club: "putter".into(),
        category: "putter".into(),
        label_type: None,
        can_start: false,
        stage: None,
        stage_name: None,
        line: NO_LABEL_FITS.into(),
    };
    let listed = || drill_clubs(phone.root.clone(), "aaron".into()).expect("the clubs");
    assert_eq!(
        listed(),
        [
            fresh("driver", "driver", "strikepro_driver"),
            fresh("7i", "mid_iron", "strikepro_iron"),
            putter.clone(),
        ]
    );

    phone.page();
    assert_eq!(listed()[1].stage, None, "opening a club writes nothing");
    phone.photo(None, read((1.0, 0.0)));
    assert_eq!(
        listed()[1],
        DrillClubView {
            stage: Some(StageView::Baseline),
            stage_name: Some("Baseline".into()),
            ..fresh("7i", "mid_iron", "strikepro_iron")
        }
    );
    assert_eq!(listed()[2], putter);

    let refused = start_drill(
        phone.root.clone(),
        "aaron".into(),
        "putter".into(),
        NOW.into(),
    );
    assert!(refused.unwrap_err().contains(NO_LABEL_FITS));
    let refused = set_stage(
        phone.root.clone(),
        "aaron".into(),
        "putter".into(),
        StageView::Baseline,
        NOW.into(),
    );
    assert!(refused.unwrap_err().contains(NO_LABEL_FITS));
}

#[test]
fn a_golfer_with_no_bag_has_no_clubs_to_drill() {
    let phone = Phone::new(&[]);
    assert_eq!(
        drill_clubs(phone.root.clone(), "aaron".into()),
        Ok(Vec::new())
    );
}

/// A drill record that cannot be read is never shown as no drill (the plan's P17 finding 4).
#[test]
fn an_unreadable_drill_record_is_refused_by_every_action() {
    let phone = Phone::new(&["7i"]);
    let state = phone.store().state_path();
    fs::create_dir_all(state.parent().unwrap()).unwrap();
    fs::write(&state, b"{ torn").unwrap();
    assert!(drill_clubs(phone.root.clone(), "aaron".into()).is_err());
    assert!(start_drill(phone.root.clone(), "aaron".into(), "7i".into(), NOW.into()).is_err());
    assert!(phone.set(StageView::Ladder).is_err());
    assert_eq!(fs::read(&state).unwrap(), b"{ torn");
}
