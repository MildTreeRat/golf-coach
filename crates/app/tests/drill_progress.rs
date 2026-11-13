//! `drill_progress` and `category_progress`, and the geometry the reading card gained, driven as Dart
//! drives them against a scratch data root. [M46 P20]
//!
//! Every photo is stood in through `app::drill::record_tape_photo_with`, as `drill_actions.rs` does: a
//! tiny raw file and a reader that answers where the test says the mark landed. What is pinned here
//! is what the views draw and say: a club's view at 0, 9 and 10 or more strikes, the heat map's peak,
//! a category over two clubs, differential strikes drawn and kept out of the pattern, and every shape
//! in the crop's frame.

use std::collections::BTreeMap;
use std::fs;
use std::io::Write as _;
use std::sync::atomic::{AtomicUsize, Ordering};

use analysis::drill::pattern;
use app::api::drill::{
    category_progress, drill_progress, pick_mark, set_stage, ClubLevelRow, DrillCall,
    DrillStage as StageView, HistoryRow, LabelCircle, LabelPoint, ProgressView, TapeIn,
    TapeOutcome, TapeStatus,
};
use app::api::setup::{create_golfer, init_state, set_bag_slot, Handedness};
use app::drill::{record_tape_photo_with, Reading, HEAT_CELL_MM};
use app::Dirs;
use contracts::club::ClubId;
use contracts::drill::{ClubDrillState, DrillStage, DrillState, TapeStrike};
use contracts::intent::ClubCategory;
use contracts::Timestamp;
use feedback::drill_rules::{
    heat_map_needs, level_line, pattern_centre_line, pattern_needs, r50_line, spread_line,
    HEEL_SIDE, NO_LABEL_FITS, NO_LEVEL_YET, TOE_SIDE,
};
use impact::tape::{
    self, Crop, CropGrid, FoundLabel, LabelType, Orientation, TapeMark, TapePick, TapeReading,
    CROP_PX_PER_MM, STRIKEPRO_IRON,
};
use impact::Frame;
use storage::drill_store::DrillStore;
use tempfile::TempDir;

const INSTALL: &str = "a1b2";
const NOW: &str = "2026-11-13T09:00:05-05:00";

/// The 7 and 6 irons' table, `mid_iron`'s: 14, 12, 10, 9, 8, 7 and 6 mm.
const MID: ClubCategory = ClubCategory::MidIron;

/// Ten baseline strikes along the heel-toe axis, `drill_actions.rs`' baseline: the club starts at its
/// third level, 10 mm.
const BASELINE: [f64; 10] = [3.0, -4.0, 5.0, -8.5, 9.5, -11.0, 11.5, -13.0, 15.0, -20.0];

/// A tight pattern centred exactly on (6, 0), 6 mm toward the toe.
const TOE: [(f64, f64); 2] = [(5.5, 0.5), (6.5, -0.5)];

/// A tight pattern on centre, exactly (0, 0).
const CENTRED: [(f64, f64); 2] = [(0.5, -0.5), (-0.5, 0.5)];

/// A tolerance for shares of the label worked through two different products of the same numbers.
const CLOSE: f64 = 1e-12;

/// A scratch data root with an install id, one golfer and a bag, as setup leaves it.
struct Phone {
    _dir: TempDir,
    root: String,
}

static PHOTOS: AtomicUsize = AtomicUsize::new(0);

impl Phone {
    fn new(hand: Handedness, bag: &[&str]) -> Phone {
        let dir = tempfile::tempdir().expect("a scratch directory");
        let root = dir.path().join("processed").display().to_string();
        init_state(root.clone(), INSTALL.to_string()).expect("an install id");
        create_golfer(root.clone(), "Aaron".to_string(), hand, NOW.to_string()).expect("a golfer");
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

    /// One photo of `club`, read as `reading` says, taken one second after the last.
    fn photo(&self, club: &str, call: Option<DrillCall>, reading: Reading) -> TapeOutcome {
        let n = PHOTOS.fetch_add(1, Ordering::Relaxed);
        let tmp = Dirs::of(&self.root).app.join("tmp");
        fs::create_dir_all(&tmp).expect("tmp");
        let raw = tmp.join(format!("photo-{n}.rgba"));
        fs::write(&raw, [200u8; 4 * 2 * 4]).expect("a raw photo");
        let input = TapeIn {
            root: self.root.clone(),
            player_id: "aaron".into(),
            club: club.into(),
            raw_path: raw.display().to_string(),
            width: 4,
            height: 2,
            captured_at: format!(
                "2026-11-13T{:02}:{:02}:{:02}-05:00",
                8 + n / 3600,
                n / 60 % 60,
                n % 60
            ),
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

    /// `n` photos of `club`, the `i`th landing at `at(i)`.
    fn photos(&self, club: &str, n: usize, at: impl Fn(usize) -> (f64, f64)) {
        for i in 0..n {
            self.photo(club, None, read(at(i), SIDES));
        }
    }

    fn club(&self, club: &str) -> ProgressView {
        drill_progress(self.root.clone(), "aaron".into(), club.into())
            .unwrap_or_else(|refused| panic!("{refused}"))
    }

    fn category(&self, category: &str) -> ProgressView {
        category_progress(self.root.clone(), "aaron".into(), category.into())
            .unwrap_or_else(|refused| panic!("{refused}"))
    }

    /// The 7 iron on the ladder at `level`, as an earlier session would have left it.
    fn seed_ladder(&self, level: usize) {
        let mut clubs = BTreeMap::new();
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
        DrillStore::open(&self.root, "aaron")
            .expect("a drill store")
            .put_state(&DrillState {
                player_id: "aaron".into(),
                strikes_issued: 0,
                labels_issued: 0,
                clubs,
                updated_at: Timestamp::parse(NOW).unwrap(),
            })
            .expect("a seeded state");
    }

    fn strikes(&self) -> Vec<TapeStrike> {
        DrillStore::open(&self.root, "aaron")
            .expect("a drill store")
            .all_strikes()
            .expect("readable strikes")
            .strikes
    }
}

/// The iron label found, with a plain crop.
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

fn iron_grid() -> CropGrid {
    CropGrid::new(&STRIKEPRO_IRON, CROP_PX_PER_MM)
}

/// A right-handed club with its label logo-up: the toe at the printed left.
const SIDES: Orientation = Orientation {
    toe_printed_right: false,
    top_line_printed_top: true,
};

/// A right-handed club with its label logo-down: the toe at the printed right, the top line at the
/// printed bottom.
const UPSIDE_DOWN: Orientation = Orientation {
    toe_printed_right: true,
    top_line_printed_top: false,
};

fn read((h, v): (f64, f64), sides: Orientation) -> Reading {
    Reading {
        outcome: tape::TapeOutcome::Read(TapeReading {
            found: found(),
            orientation: sides,
            new_label: false,
            marks: vec![TapeMark {
                centroid_px: [0.0, 0.0],
                offset_h_mm: h,
                offset_v_mm: v,
                area_mm2: 78.5,
            }],
        }),
        sides: Some(sides),
    }
}

fn baseline_at(n: usize) -> (f64, f64) {
    (BASELINE[n], 0.0)
}

fn alternating(pair: [(f64, f64); 2]) -> impl Fn(usize) -> (f64, f64) {
    move |n| pair[n % 2]
}

/// The iron label's printed-frame millimetres as a share of its crop, worked by hand from its
/// measured box: 73.0 by 39.9 mm at 10 px/mm, the cross 36.6 mm from the left and 24.8 mm down.
fn on_iron([x, y]: [f64; 2]) -> LabelPoint {
    LabelPoint {
        x: (x + 36.6) * 10.0 / 730.0,
        y: (24.8 - y) * 10.0 / 399.0,
    }
}

fn assert_near(at: LabelPoint, expected: LabelPoint, what: &str) {
    assert!(
        (at.x - expected.x).abs() < CLOSE && (at.y - expected.y).abs() < CLOSE,
        "{what}: {at:?} is not {expected:?}"
    );
}

/// A circle of `radius_mm` about the cross, worked by hand.
fn about_cross(radius_mm: f64) -> LabelCircle {
    let cross = on_iron([0.0, 0.0]);
    LabelCircle {
        x: cross.x,
        y: cross.y,
        rx: radius_mm * 10.0 / 730.0,
        ry: radius_mm * 10.0 / 399.0,
    }
}

fn assert_circle(circle: Option<LabelCircle>, expected: LabelCircle, what: &str) {
    let circle = circle.unwrap_or_else(|| panic!("{what}: no circle"));
    assert_near(
        LabelPoint {
            x: circle.x,
            y: circle.y,
        },
        LabelPoint {
            x: expected.x,
            y: expected.y,
        },
        what,
    );
    assert!(
        (circle.rx - expected.rx).abs() < CLOSE && (circle.ry - expected.ry).abs() < CLOSE,
        "{what}: {circle:?} is not {expected:?}"
    );
}

fn peak(view: &ProgressView) -> f64 {
    view.heat_map
        .as_ref()
        .expect("a heat map")
        .rows
        .iter()
        .flatten()
        .copied()
        .fold(f64::MIN, f64::max)
}

// --- A club's view -------------------------------------------------------------------------------------

/// Nothing, then nine baseline strikes, then the tenth and a ladder block: the pattern and the heat
/// map are absent with how many they need until the tenth placed strike, and shown from it; the dots
/// are every strike; and the circle, the level line and the history follow the judged blocks.
#[test]
fn a_club_shows_its_pattern_and_heat_map_from_its_tenth_placed_strike() {
    let phone = Phone::new(Handedness::Right, &["7i"]);
    let empty = phone.club("7i");
    assert_eq!(
        empty,
        ProgressView {
            title: "7i".into(),
            label: empty.label.clone(),
            left_side: TOE_SIDE.into(),
            right_side: HEEL_SIDE.into(),
            circle: None,
            level_line: Some(NO_LEVEL_YET.into()),
            dots: Vec::new(),
            pattern: None,
            pattern_needs: pattern_needs(0),
            heat_map: None,
            heat_map_needs: heat_map_needs(0),
            history: Vec::new(),
            clubs: Vec::new(),
            record_note: None,
        }
    );
    assert_eq!(
        empty.pattern_needs.as_deref(),
        Some("The pattern needs 10 more strikes placed on the label.")
    );
    assert!(
        !DrillStore::open(&phone.root, "aaron")
            .unwrap()
            .state_path()
            .exists(),
        "a view writes nothing"
    );

    phone.photos("7i", 9, baseline_at);
    let nine = phone.club("7i");
    assert_eq!(nine.dots.len(), 9);
    assert_eq!((nine.pattern.clone(), nine.heat_map.clone()), (None, None));
    assert_eq!(
        nine.pattern_needs.as_deref(),
        Some("The pattern needs 1 more strike placed on the label.")
    );
    assert_eq!(nine.heat_map_needs, heat_map_needs(9));
    assert_eq!((nine.circle, nine.history.len()), (None, 0));
    // The first strike, 3 mm toward the toe: on the left of the cross for a right-handed golfer.
    assert_near(nine.dots[0].at, on_iron([-3.0, 0.0]), "the first dot");
    assert_eq!(nine.dots[0].stage, StageView::Baseline);

    phone.photos("7i", 1, |_| baseline_at(9));
    let ten = phone.club("7i");
    let found = pattern(&phone.strikes()).expect("a pattern of ten");
    let shown = ten.pattern.clone().expect("the pattern, at ten");
    assert_eq!(
        (
            shown.centre_line.as_str(),
            shown.spread_line.as_str(),
            shown.r50_line.as_str()
        ),
        (
            pattern_centre_line(found.centre).as_str(),
            spread_line(found.spread).as_str(),
            r50_line(found.r50).as_str()
        )
    );
    assert_near(
        shown.centre,
        on_iron([-found.centre.0, found.centre.1]),
        "the pattern's centre",
    );
    assert_eq!(
        (ten.pattern_needs.clone(), ten.heat_map_needs.clone()),
        (None, None)
    );
    assert_eq!(peak(&ten), 1.0);
    // The baseline set the third level, 10 mm.
    assert_eq!(ten.level_line, level_line(2, MID));
    assert_circle(ten.circle, about_cross(10.0), "the club's circle");
    assert_eq!(
        ten.history,
        [HistoryRow {
            date: "2026-11-13".into(),
            stage: Some("Baseline".into()),
            from: "no level".into(),
            to: "10 mm".into(),
        }]
    );

    // A ladder block all inside 10 mm moves the club in, to 9 mm.
    phone.photos("7i", 10, alternating(CENTRED));
    let twenty = phone.club("7i");
    assert_eq!(twenty.dots.len(), 20);
    assert_circle(
        twenty.circle,
        about_cross(9.0),
        "the circle after a move in",
    );
    assert_eq!(
        twenty.history[1],
        HistoryRow {
            date: "2026-11-13".into(),
            stage: Some("Ladder".into()),
            from: "10 mm".into(),
            to: "9 mm".into(),
        }
    );
    assert_eq!(
        twenty.pattern.expect("a pattern").r50_line,
        r50_line(pattern(&phone.strikes()).unwrap().r50)
    );
}

/// Ten strikes 6 mm toward the toe: the densest cell is exactly 1, it is the cell they landed in,
/// every other is below it and none below 0, and the grid is the label's box in 2 mm cells.
#[test]
fn the_heat_map_peaks_at_exactly_one_where_the_strikes_landed() {
    let phone = Phone::new(Handedness::Right, &["7i"]);
    phone.photos("7i", 10, alternating(TOE));
    let view = phone.club("7i");
    let heat = view.heat_map.expect("a heat map at ten");
    // 73.0 by 39.9 mm in 2 mm cells: 37 across and 20 down, the last of each running past the box.
    assert_eq!(heat.rows.len(), 20);
    assert!(heat.rows.iter().all(|row| row.len() == 37));
    assert!((heat.cell_width - HEAT_CELL_MM * 10.0 / 730.0).abs() < CLOSE);
    assert!((heat.cell_height - HEAT_CELL_MM * 10.0 / 399.0).abs() < CLOSE);
    // The pattern sits at printed (-6, 0) for a right-handed golfer: 306 px across and 248 px down,
    // which is column 15 and row 12.
    assert_eq!(heat.rows[12][15], 1.0);
    for (r, row) in heat.rows.iter().enumerate() {
        for (c, &value) in row.iter().enumerate() {
            assert!((0.0..=1.0).contains(&value), "[{r}][{c}] = {value}");
            if (r, c) != (12, 15) {
                assert!(value < 1.0, "[{r}][{c}] = {value}");
            }
        }
    }
}

/// A left-handed golfer's view is the mirror of a right-handed one's: the toe on the right, and the
/// same strike drawn there.
#[test]
fn a_left_handed_golfer_sees_the_toe_on_the_right() {
    let phone = Phone::new(Handedness::Left, &["7i"]);
    let left_handed = Orientation {
        toe_printed_right: true,
        top_line_printed_top: true,
    };
    phone.photo("7i", None, read((3.0, 2.0), left_handed));
    let view = phone.club("7i");
    assert_eq!(
        (view.left_side.as_str(), view.right_side.as_str()),
        (HEEL_SIDE, TOE_SIDE)
    );
    assert_near(
        view.dots[0].at,
        on_iron([3.0, 2.0]),
        "a left-handed toe strike",
    );
}

/// Ten baseline strikes centred on (6, 0), then differential practice with every call landing 12 mm
/// toward the heel: those are drawn, marked with their stage, and move neither the pattern nor the
/// heat map, nor what either needs.
#[test]
fn differential_strikes_are_dots_but_not_in_the_pattern_or_the_heat_map() {
    let phone = Phone::new(Handedness::Right, &["7i"]);
    phone.photos("7i", 10, alternating(TOE));
    let before = phone.club("7i");
    set_stage(
        phone.root.clone(),
        "aaron".into(),
        "7i".into(),
        StageView::Differential,
        NOW.into(),
    )
    .expect("differential practice, after a baseline");
    for _ in 0..3 {
        phone.photo("7i", Some(DrillCall::Heel), read((-12.0, 0.0), SIDES));
    }
    let after = phone.club("7i");
    assert_eq!(after.dots.len(), 13);
    assert_eq!(&after.dots[..10], &before.dots[..]);
    for dot in &after.dots[10..] {
        assert_eq!(dot.stage, StageView::Differential);
        assert_near(dot.at, on_iron([12.0, 0.0]), "a heel strike");
    }
    let pattern = after.pattern.clone().expect("the baseline's pattern");
    assert_eq!(pattern.centre_line, pattern_centre_line((6.0, 0.0)));
    assert_eq!(after.pattern, before.pattern);
    assert_eq!(after.heat_map, before.heat_map);
}

/// Three differential strikes and then nine on the ladder: twelve dots, and the pattern and the heat
/// map each still one strike short, because only the nine were aimed at centre.
#[test]
fn only_strikes_aimed_at_centre_count_toward_the_pattern_and_the_heat_map() {
    let phone = Phone::new(Handedness::Right, &["7i"]);
    phone.seed_ladder(2);
    let stage = |stage| {
        set_stage(
            phone.root.clone(),
            "aaron".into(),
            "7i".into(),
            stage,
            NOW.into(),
        )
        .expect("a stage")
    };
    stage(StageView::Differential);
    for _ in 0..3 {
        phone.photo("7i", Some(DrillCall::Toe), read((12.0, 0.0), SIDES));
    }
    stage(StageView::Ladder);
    phone.photos("7i", 9, alternating(CENTRED));
    let view = phone.club("7i");
    assert_eq!(view.dots.len(), 12);
    assert_eq!((view.pattern, view.heat_map), (None, None));
    assert_eq!(view.pattern_needs, pattern_needs(9));
    assert_eq!(view.heat_map_needs, heat_map_needs(9));
}

// --- A category's view ---------------------------------------------------------------------------------

/// Five strikes on each of two mid irons, interleaved, and one on the 8 iron, a short iron: the mid
/// irons' view pools the ten in the order they were taken, shows the pattern neither club has alone,
/// lists both clubs in bag order, and has no circle, level line or history of its own.
#[test]
fn a_category_pools_its_clubs_strikes_and_lists_each_clubs_level() {
    let phone = Phone::new(Handedness::Right, &["6i", "7i", "8i"]);
    for n in 0..10 {
        let club = if n % 2 == 0 { "7i" } else { "6i" };
        phone.photo(club, None, read(baseline_at(n), SIDES));
    }
    phone.photo("8i", None, read((1.0, 1.0), SIDES));

    assert_eq!(phone.club("7i").pattern, None);
    assert_eq!(phone.club("6i").pattern, None);
    let mid = phone.category("mid_iron");
    assert_eq!(mid.title, "Mid irons");
    assert_eq!(mid.dots.len(), 10);
    for (n, dot) in mid.dots.iter().enumerate() {
        assert_near(
            dot.at,
            on_iron([-BASELINE[n], 0.0]),
            "a pooled dot, in photo order",
        );
    }
    let mid_strikes: Vec<TapeStrike> = phone
        .strikes()
        .into_iter()
        .filter(|strike| strike.club.as_str() != "8i")
        .collect();
    assert_eq!(
        mid.pattern.as_ref().expect("the pool's pattern").r50_line,
        r50_line(pattern(&mid_strikes).unwrap().r50)
    );
    assert_eq!(peak(&mid), 1.0);
    assert_eq!(
        mid.clubs,
        [
            ClubLevelRow {
                club: "6i".into(),
                stage_name: "Baseline".into(),
                level_line: NO_LEVEL_YET.into(),
            },
            ClubLevelRow {
                club: "7i".into(),
                stage_name: "Baseline".into(),
                level_line: NO_LEVEL_YET.into(),
            },
        ]
    );
    assert_eq!(
        (mid.circle, mid.level_line.clone(), mid.history.len()),
        (None, None, 0)
    );

    let short = phone.category("short_iron");
    assert_eq!(
        (short.title.as_str(), short.dots.len(), short.clubs.len()),
        ("Short irons", 1, 1)
    );
    assert_eq!(short.pattern_needs, pattern_needs(1));
}

/// A 7 iron strike waits for a tap while a 6 iron photo is stored, and is stored after it when the
/// tap comes: the pool draws the 7 iron's first, because its photo was taken first.
#[test]
fn a_pool_is_in_the_order_its_photos_were_taken_not_the_order_they_were_stored() {
    let phone = Phone::new(Handedness::Right, &["6i", "7i"]);
    let waiting = phone.photo(
        "7i",
        None,
        Reading {
            outcome: tape::TapeOutcome::NeedsPick(TapePick {
                found: found(),
                orientation: SIDES,
            }),
            sides: Some(SIDES),
        },
    );
    phone.photo("6i", None, read((-5.0, 0.0), SIDES));
    pick_mark(
        phone.root.clone(),
        waiting.pending_id.expect("a waiting strike"),
        0.25,
        0.6,
        NOW.into(),
    )
    .expect("the pick");
    let stored: Vec<String> = phone.strikes().iter().map(|s| s.club.to_string()).collect();
    assert_eq!(stored, ["6i", "7i"], "stored in the order resolved");
    let mid = phone.category("mid_iron");
    assert_near(
        mid.dots[0].at,
        LabelPoint { x: 0.25, y: 0.6 },
        "the 7 iron's, first",
    );
    assert_near(mid.dots[1].at, on_iron([5.0, 0.0]), "the 6 iron's, second");
}

// --- The card's geometry -------------------------------------------------------------------------------

/// A read strike's dot is where its mark sits on the crop, through the photo's own sides: on a label
/// stuck logo-down that is the mirror of where the progress view draws it. The circle is the one the
/// strike was hit at, and the baseline has none.
#[test]
fn the_card_draws_each_dot_on_the_crop_and_the_circle_it_was_hit_at() {
    let phone = Phone::new(Handedness::Right, &["7i"]);
    let first = phone.photo("7i", None, read((3.0, 2.0), UPSIDE_DOWN));
    assert_near(
        first.strikes[0].dot.expect("a dot"),
        on_iron([3.0, -2.0]),
        "a logo-down strike on its crop",
    );
    assert_near(
        phone.club("7i").dots[0].at,
        on_iron([-3.0, 2.0]),
        "the same strike in the view",
    );
    assert_eq!(first.circle, None, "the baseline has no circle");
    let label = &first.label;
    assert_eq!(label.label_type, "strikepro_iron");
    assert!((label.aspect - 730.0 / 399.0).abs() < CLOSE);
    assert_eq!(label.outline.len(), STRIKEPRO_IRON.outline_mm.len());
    assert_near(
        label.outline[0],
        on_iron(STRIKEPRO_IRON.outline_mm[0]),
        "the outline",
    );
    assert_near(label.cross, on_iron([0.0, 0.0]), "the cross");

    phone.photos("7i", 9, |n| baseline_at(n + 1));
    let ladder = phone.photo("7i", None, read((1.0, 1.0), SIDES));
    assert_circle(ladder.circle, about_cross(10.0), "the card's circle");
}

/// A pick's dot is the tap: the frame a tap is read in is the frame a dot is drawn in.
#[test]
fn a_picked_strikes_dot_is_where_the_golfer_tapped() {
    let phone = Phone::new(Handedness::Right, &["7i"]);
    let waiting = phone.photo(
        "7i",
        None,
        Reading {
            outcome: tape::TapeOutcome::NeedsPick(TapePick {
                found: found(),
                orientation: SIDES,
            }),
            sides: Some(SIDES),
        },
    );
    assert_eq!(waiting.status, TapeStatus::NeedsPick);
    assert!(waiting.strikes.is_empty());
    let picked = pick_mark(
        phone.root.clone(),
        waiting.pending_id.expect("a waiting strike"),
        0.25,
        0.6,
        NOW.into(),
    )
    .expect("the pick");
    assert_near(
        picked.strikes[0].dot.expect("a dot"),
        LabelPoint { x: 0.25, y: 0.6 },
        "the tap",
    );
}

// --- What a view says, and refuses ---------------------------------------------------------------------

/// A line of the record that cannot be read is left out, and the view says so.
#[test]
fn a_view_says_when_the_record_held_a_line_it_could_not_read() {
    let phone = Phone::new(Handedness::Right, &["7i"]);
    phone.photos("7i", 2, baseline_at);
    let path = DrillStore::open(&phone.root, "aaron")
        .unwrap()
        .strikes_path();
    let mut file = fs::OpenOptions::new().append(true).open(path).unwrap();
    writeln!(file, "{{\"strike_id\": \"a1b2-9").unwrap();
    let view = phone.club("7i");
    assert_eq!(view.dots.len(), 2);
    assert_eq!(
        view.record_note.as_deref(),
        Some("1 line of the drill's record could not be read and is left out.")
    );
    assert_eq!(phone.category("mid_iron").record_note, view.record_note);
}

/// A club or category no label fits, a category that is not one, a golfer not on the phone, and a
/// state that cannot be read, each refused.
#[test]
fn a_view_is_refused_where_no_label_fits_or_the_record_cannot_be_read() {
    let phone = Phone::new(Handedness::Right, &["7i"]);
    let refusal = |result: Result<ProgressView, String>| result.expect_err("refused");
    assert!(refusal(drill_progress(
        phone.root.clone(),
        "aaron".into(),
        "putter".into()
    ))
    .contains(NO_LABEL_FITS));
    assert!(refusal(category_progress(
        phone.root.clone(),
        "aaron".into(),
        "putter".into()
    ))
    .contains(NO_LABEL_FITS));
    assert_eq!(
        refusal(category_progress(
            phone.root.clone(),
            "aaron".into(),
            "irons".into()
        )),
        "'irons' is not a club category"
    );
    assert!(refusal(drill_progress(
        phone.root.clone(),
        "nobody".into(),
        "7i".into()
    ))
    .contains("is not on this phone"));

    phone.photos("7i", 1, baseline_at);
    let state = DrillStore::open(&phone.root, "aaron").unwrap().state_path();
    fs::write(state, "{ not json").unwrap();
    assert!(refusal(drill_progress(
        phone.root.clone(),
        "aaron".into(),
        "7i".into()
    ))
    .starts_with("The drill's record on this phone cannot be read"));
}
