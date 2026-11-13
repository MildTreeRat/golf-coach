//! `analysis::drill`: which strikes count, inside a circle at and either side of its edge, the
//! baseline's start level, and a ladder block's step at the middle and both ends of a table (M46
//! P14); then the pattern, each suggestion's trigger firing and not at its edge, a recentre block's
//! goal and each differential call's edge (P15).
//!
//! **No Python twin and no vector**, as `analysis::strike`'s rules have none: each test holds one rule
//! to the M46 plan's calls, C10 (what counts, inside, a block's step), C11 (the baseline), C12 (the
//! pattern), C13 (the suggestions) and C14 (the calls), with ADR-037 clause 8 read as the plan's P1
//! finding 6 reads it and the widest start level the user's (P1 finding 5). The
//! strikes are a 7 iron's, a `mid_iron`, and every one passes `Validate`, so no rule here is held on a
//! strike the store would refuse. The tables are read from `ladder_radii` rather than copied, except
//! where a worked case names its radii, and that case asserts them first: P29's floor removes the
//! tightest radii (the plan's C9), and these tests should move with it rather than pin today's.

use analysis::drill::{
    call_made, counted, inside, judge_block, judge_differential, judge_recentre, level_or_widest,
    off_centre, pattern, start_level, suggestion, BlockResult, DifferentialResult, LadderStep,
    OffCentre, Pattern, RecentreResult,
};
use contracts::club::{category_of, ClubId};
use contracts::drill::{
    crop_file, ladder_radii, strike_id, ClubDrillState, DrillStage, LevelEntry, StrikeCall,
    StrikeSource, Suggestion, TapeNotRead, TapeStrike, RECENTRE_FRACTION,
};
use contracts::face_impact::TAPE_READER_VERSION;
use contracts::intent::ClubCategory;
use contracts::{ContractError, Timestamp, Validate};

const INSTALL: &str = "3fa2";

/// Every category with a ladder. The putter and `all` have none, and are refused below.
const LADDERED: [ClubCategory; 7] = [
    ClubCategory::Driver,
    ClubCategory::Wood,
    ClubCategory::Hybrid,
    ClubCategory::LongIron,
    ClubCategory::MidIron,
    ClubCategory::ShortIron,
    ClubCategory::Wedge,
];

fn radii() -> &'static [f64] {
    ladder_radii(ClubCategory::MidIron)
}

fn tightest() -> usize {
    radii().len() - 1
}

// --- The strikes ----------------------------------------------------------------------------------

/// A 7 iron's strike numbered `n` (below 3,600), in `stage` and judged against `radius_mm`, captured
/// `n` seconds after 07:00, before anything says where it landed.
fn shell(n: i64, stage: DrillStage, radius_mm: Option<f64>) -> TapeStrike {
    TapeStrike {
        strike_id: strike_id(INSTALL, n),
        player_id: "aaron".into(),
        club: ClubId::SevenIron,
        category: ClubCategory::MidIron,
        label_type: "strikepro_iron".into(),
        label_id: "3fa2-L1".into(),
        n_on_label: n,
        captured_at: Timestamp::parse(&format!("2026-11-13T07:{:02}:{:02}Z", n / 60, n % 60))
            .unwrap(),
        stage,
        level_radius_mm: radius_mm,
        call: None,
        offset_h_mm: None,
        offset_v_mm: None,
        source: None,
        not_read: None,
        reader_version: TAPE_READER_VERSION,
        crop: None,
    }
}

fn on_ladder(n: i64, level: usize) -> TapeStrike {
    shell(n, DrillStage::Ladder, Some(radii()[level]))
}

fn in_baseline(n: i64) -> TapeStrike {
    shell(n, DrillStage::Baseline, None)
}

/// The store keeps only a strike that validates, so a fixture that did not would be testing a rule
/// on a strike nothing could write.
fn checked(strike: TapeStrike) -> TapeStrike {
    strike.validate().unwrap_or_else(|e| panic!("{e}"));
    strike
}

fn placed(shell: TapeStrike, source: StrikeSource, h: f64, v: f64) -> TapeStrike {
    let crop = crop_file(&shell.strike_id);
    checked(TapeStrike {
        offset_h_mm: Some(h),
        offset_v_mm: Some(v),
        source: Some(source),
        crop: Some(crop),
        ..shell
    })
}

fn read(shell: TapeStrike, h: f64, v: f64) -> TapeStrike {
    placed(shell, StrikeSource::Read, h, v)
}

fn picked(shell: TapeStrike, h: f64, v: f64) -> TapeStrike {
    placed(shell, StrikeSource::Picked, h, v)
}

fn declared(shell: TapeStrike) -> TapeStrike {
    checked(TapeStrike {
        source: Some(StrikeSource::DeclaredOffTape),
        ..shell
    })
}

fn not_read(shell: TapeStrike, reason: TapeNotRead) -> TapeStrike {
    checked(TapeStrike {
        not_read: Some(reason),
        ..shell
    })
}

/// Half the circle's radius, toward the heel and high.
fn well_inside(shell: TapeStrike, radius: f64) -> TapeStrike {
    read(shell, -0.3 * radius, 0.4 * radius)
}

/// A millimetre past the circle, toward the toe and low.
fn just_outside(shell: TapeStrike, radius: f64) -> TapeStrike {
    read(shell, 0.6 * (radius + 1.0), -0.8 * (radius + 1.0))
}

/// A ladder block of ten read strikes at `level`, numbered 1 to 10: `n_inside` inside its circle,
/// then the rest outside it.
fn ladder_block(level: usize, n_inside: usize) -> Vec<TapeStrike> {
    let radius = radii()[level];
    (1..=10)
        .map(|n| {
            if n as usize <= n_inside {
                well_inside(on_ladder(n, level), radius)
            } else {
                just_outside(on_ladder(n, level), radius)
            }
        })
        .collect()
}

fn judged(
    inside: usize,
    step: LadderStep,
    new_level: usize,
    at_tightest_in: bool,
    at_widest_out: bool,
) -> BlockResult {
    BlockResult {
        inside,
        counted: 10,
        step,
        new_level,
        at_tightest_in,
        at_widest_out,
    }
}

// --- Inside, and what counts ------------------------------------------------------------------------

#[test]
fn inside_is_hypot_at_most_the_radius_with_the_edge_inside_on_either_side_of_centre() {
    // The premise: the platform's hypot is exact on a 6-8-10 triangle, so the edge below is the edge.
    assert_eq!(6.0_f64.hypot(8.0), 10.0);
    for (n, (h, v)) in [(6.0, 8.0), (-6.0, -8.0), (10.0, 0.0), (0.0, -10.0)]
        .into_iter()
        .enumerate()
    {
        let strike = read(on_ladder(n as i64 + 1, 2), h, v);
        assert_eq!(
            inside(&strike, 10.0),
            Some(true),
            "({h}, {v}) on the circle"
        );
        assert_eq!(inside(&strike, 9.999), Some(false), "({h}, {v}) past it");
    }
}

#[test]
fn a_picked_strike_is_judged_where_it_was_tapped_as_a_read_one_is() {
    let strike = picked(on_ladder(1, 2), -6.0, 8.0);
    assert_eq!(inside(&strike, 10.0), Some(true));
    assert_eq!(inside(&strike, 9.999), Some(false));
}

#[test]
fn a_strike_declared_off_the_tape_is_outside_every_circle_of_every_table() {
    let off = declared(on_ladder(1, 0));
    for category in LADDERED {
        for &radius in ladder_radii(category) {
            assert_eq!(
                inside(&off, radius),
                Some(false),
                "{} at {radius} mm",
                category.as_str()
            );
        }
    }
}

#[test]
fn a_strike_not_read_is_neither_inside_nor_outside() {
    for (n, reason) in [
        TapeNotRead::NoLabel,
        TapeNotRead::NoOrientation,
        TapeNotRead::NoNewMark,
        TapeNotRead::MarkClipped,
        TapeNotRead::DecodeFailed,
        TapeNotRead::NotPicked,
    ]
    .into_iter()
    .enumerate()
    {
        let strike = not_read(on_ladder(n as i64 + 1, 0), reason);
        assert_eq!(inside(&strike, radii()[0]), None, "{reason:?}");
    }
}

#[test]
fn counted_keeps_read_picked_and_declared_strikes_in_order_and_drops_the_not_read() {
    let strikes = [
        read(on_ladder(1, 2), 1.0, 1.0),
        not_read(on_ladder(2, 2), TapeNotRead::NoNewMark),
        picked(on_ladder(3, 2), -2.0, 0.5),
        declared(on_ladder(4, 2)),
        not_read(on_ladder(5, 2), TapeNotRead::MarkClipped),
    ];
    let ids: Vec<&str> = counted(&strikes)
        .iter()
        .map(|strike| strike.strike_id.as_str())
        .collect();
    assert_eq!(ids, ["3fa2-1", "3fa2-3", "3fa2-4"]);
}

// --- A ladder block ---------------------------------------------------------------------------------

#[test]
fn seven_inside_moves_in_a_level_and_so_does_ten() {
    for n_inside in [7, 10] {
        assert_eq!(
            judge_block(&ladder_block(2, n_inside), 2, ClubCategory::MidIron),
            Ok(judged(n_inside, LadderStep::In, 3, false, false)),
            "{n_inside} inside"
        );
    }
}

#[test]
fn three_inside_moves_out_a_level_and_so_does_none() {
    for n_inside in [3, 0] {
        assert_eq!(
            judge_block(&ladder_block(2, n_inside), 2, ClubCategory::MidIron),
            Ok(judged(n_inside, LadderStep::Out, 1, false, false)),
            "{n_inside} inside"
        );
    }
}

#[test]
fn four_to_six_inside_stays() {
    for n_inside in 4..=6 {
        assert_eq!(
            judge_block(&ladder_block(2, n_inside), 2, ClubCategory::MidIron),
            Ok(judged(n_inside, LadderStep::Stay, 2, false, false)),
            "{n_inside} inside"
        );
    }
}

#[test]
fn seven_inside_at_the_tightest_level_stays_and_says_so() {
    let level = tightest();
    assert_eq!(
        judge_block(&ladder_block(level, 7), level, ClubCategory::MidIron),
        Ok(judged(7, LadderStep::Stay, level, true, false))
    );
    // A block at the tightest that earns nothing says nothing.
    assert_eq!(
        judge_block(&ladder_block(level, 5), level, ClubCategory::MidIron),
        Ok(judged(5, LadderStep::Stay, level, false, false))
    );
}

#[test]
fn three_inside_at_the_widest_level_stays_and_says_so() {
    for n_inside in [3, 0] {
        assert_eq!(
            judge_block(&ladder_block(0, n_inside), 0, ClubCategory::MidIron),
            Ok(judged(n_inside, LadderStep::Stay, 0, false, true)),
            "{n_inside} inside"
        );
    }
    assert_eq!(
        judge_block(&ladder_block(0, 5), 0, ClubCategory::MidIron),
        Ok(judged(5, LadderStep::Stay, 0, false, false))
    );
}

#[test]
fn moving_in_from_the_widest_and_out_from_the_tightest_are_ordinary_steps() {
    assert_eq!(
        judge_block(&ladder_block(0, 7), 0, ClubCategory::MidIron),
        Ok(judged(7, LadderStep::In, 1, false, false))
    );
    let level = tightest();
    assert_eq!(
        judge_block(&ladder_block(level, 3), level, ClubCategory::MidIron),
        Ok(judged(3, LadderStep::Out, level - 1, false, false))
    );
}

#[test]
fn a_strike_not_read_is_not_counted_and_cannot_make_up_the_tenth() {
    let mut block = ladder_block(2, 7);
    block.insert(3, not_read(on_ladder(11, 2), TapeNotRead::NoNewMark));
    block.push(not_read(on_ladder(12, 2), TapeNotRead::NoLabel));
    assert_eq!(
        judge_block(&block, 2, ClubCategory::MidIron),
        Ok(judged(7, LadderStep::In, 3, false, false))
    );

    let mut short = ladder_block(2, 7);
    short.pop();
    short.push(not_read(on_ladder(13, 2), TapeNotRead::NoOrientation));
    let refused = judge_block(&short, 2, ClubCategory::MidIron).unwrap_err();
    assert_eq!(refused.field, "judge_block.block");
}

#[test]
fn two_marks_on_one_photo_are_two_counted_strikes_each_judged_alone() {
    let level = 2;
    let radius = radii()[level];
    // Five inside and three outside, then one photo's two new marks: the same moment on the same
    // label, one inside the circle and one past it.
    let mut block = ladder_block(level, 5);
    block.truncate(8);
    let first = well_inside(on_ladder(9, level), radius);
    let second = just_outside(
        TapeStrike {
            captured_at: first.captured_at,
            ..on_ladder(10, level)
        },
        radius,
    );
    block.extend([first.clone(), second]);
    assert_eq!(
        judge_block(&block, level, ClubCategory::MidIron),
        Ok(judged(6, LadderStep::Stay, level, false, false))
    );

    // Both inside, and the block that stayed moves in.
    block.pop();
    block.push(well_inside(
        TapeStrike {
            captured_at: first.captured_at,
            ..on_ladder(10, level)
        },
        radius,
    ));
    assert_eq!(
        judge_block(&block, level, ClubCategory::MidIron),
        Ok(judged(7, LadderStep::In, level + 1, false, false))
    );
}

#[test]
fn a_strike_declared_off_the_tape_counts_toward_the_block_as_a_miss() {
    let level = 2;
    let radius = radii()[level];
    let block = |n_inside: i64| -> Vec<TapeStrike> {
        (1..=10)
            .map(|n| {
                if n <= n_inside {
                    well_inside(on_ladder(n, level), radius)
                } else {
                    declared(on_ladder(n, level))
                }
            })
            .collect()
    };
    // Seven read inside and three declared: ten counted, so the block is judged, and moves in.
    assert_eq!(
        judge_block(&block(7), level, ClubCategory::MidIron),
        Ok(judged(7, LadderStep::In, level + 1, false, false))
    );
    // Six and four: each declaration is a miss, so it stays.
    assert_eq!(
        judge_block(&block(6), level, ClubCategory::MidIron),
        Ok(judged(6, LadderStep::Stay, level, false, false))
    );
}

#[test]
fn a_block_of_anything_but_ten_counted_strikes_is_refused() {
    let nine = &ladder_block(2, 7)[..9];
    assert_eq!(
        judge_block(nine, 2, ClubCategory::MidIron)
            .unwrap_err()
            .field,
        "judge_block.block"
    );
    let mut eleven = ladder_block(2, 7);
    eleven.push(well_inside(on_ladder(11, 2), radii()[2]));
    assert_eq!(
        judge_block(&eleven, 2, ClubCategory::MidIron)
            .unwrap_err()
            .field,
        "judge_block.block"
    );
}

#[test]
fn a_level_past_the_table_and_a_category_with_no_ladder_are_refused() {
    let past = radii().len();
    assert_eq!(
        judge_block(&ladder_block(0, 7), past, ClubCategory::MidIron)
            .unwrap_err()
            .field,
        "judge_block.level"
    );
    for category in [ClubCategory::Putter, ClubCategory::All] {
        assert_eq!(
            judge_block(&ladder_block(0, 7), 0, category)
                .unwrap_err()
                .field,
            "judge_block.category",
            "{}",
            category.as_str()
        );
    }
}

// --- The baseline -----------------------------------------------------------------------------------

/// The worked baseline. Distances from centre: 2, 4, 6, 8.5, 10 (on the edge), 11, 13, 15, 15 mm, and
/// one declared off the tape. So the 14 mm circle holds 7, 12 holds 6, 10 holds 5 and 9 holds 4.
fn worked_baseline() -> Vec<TapeStrike> {
    vec![
        read(in_baseline(1), 2.0, 0.0),
        read(in_baseline(2), 0.0, -4.0),
        read(in_baseline(3), -6.0, 0.0),
        read(in_baseline(4), -5.1, 6.8),
        read(in_baseline(5), 6.0, 8.0),
        read(in_baseline(6), 11.0, 0.0),
        read(in_baseline(7), -12.0, 5.0),
        read(in_baseline(8), 9.0, -12.0),
        picked(in_baseline(9), -9.0, 12.0),
        declared(in_baseline(10)),
    ]
}

#[test]
fn the_start_level_is_the_tightest_circle_holding_five_of_the_ten() {
    assert_eq!(radii()[..4], [14.0, 12.0, 10.0, 9.0], "the case's premise");
    assert_eq!(
        start_level(&worked_baseline(), ClubCategory::MidIron),
        Ok(Some(2))
    );
}

#[test]
fn a_strike_on_the_edge_is_inside_the_circle_it_completes_and_a_hair_past_it_is_not() {
    // The worked baseline's fifth strike sits on the 10 mm circle and completes its five. Moved a
    // hair out, the circle holds four, and the level is the 12 mm one.
    let mut baseline = worked_baseline();
    baseline[4] = read(in_baseline(5), 6.0, 8.000_001);
    assert_eq!(start_level(&baseline, ClubCategory::MidIron), Ok(Some(1)));
}

#[test]
fn the_start_level_is_read_from_the_categorys_table() {
    // The same ten against the driver's table: 10 mm is its tightest circle, and it holds five.
    let driver = ladder_radii(ClubCategory::Driver);
    assert_eq!(driver.last(), Some(&10.0), "the case's premise");
    assert_eq!(
        start_level(&worked_baseline(), ClubCategory::Driver),
        Ok(Some(driver.len() - 1))
    );
}

#[test]
fn fewer_than_five_inside_the_widest_circle_names_no_start_level_and_five_starts_at_it() {
    let widest = radii()[0];
    let baseline = |n_inside: i64| -> Vec<TapeStrike> {
        (1..=10)
            .map(|n| match n {
                // On the widest circle's edge, toward the toe.
                n if n <= n_inside => read(in_baseline(n), widest, 0.0),
                n if n <= 7 => just_outside(in_baseline(n), widest),
                n => declared(in_baseline(n)),
            })
            .collect()
    };
    assert_eq!(start_level(&baseline(4), ClubCategory::MidIron), Ok(None));
    assert_eq!(start_level(&baseline(0), ClubCategory::MidIron), Ok(None));
    assert_eq!(
        start_level(&baseline(5), ClubCategory::MidIron),
        Ok(Some(0))
    );
}

#[test]
fn a_baseline_inside_every_circle_starts_at_the_tightest_level() {
    let baseline: Vec<TapeStrike> = (1..=10).map(|n| read(in_baseline(n), 1.0, -1.0)).collect();
    assert_eq!(
        start_level(&baseline, ClubCategory::MidIron),
        Ok(Some(tightest()))
    );
}

#[test]
fn a_baseline_strike_not_read_is_not_counted_and_the_baseline_waits_for_ten() {
    let mut baseline = worked_baseline();
    baseline.insert(2, not_read(in_baseline(11), TapeNotRead::NoNewMark));
    assert_eq!(start_level(&baseline, ClubCategory::MidIron), Ok(Some(2)));

    baseline.pop();
    assert_eq!(
        start_level(&baseline, ClubCategory::MidIron)
            .unwrap_err()
            .field,
        "start_level.baseline"
    );
    let mut eleven = worked_baseline();
    eleven.push(read(in_baseline(12), 0.0, 0.0));
    assert_eq!(
        start_level(&eleven, ClubCategory::MidIron)
            .unwrap_err()
            .field,
        "start_level.baseline"
    );
}

#[test]
fn a_baseline_for_a_category_with_no_ladder_is_refused() {
    for category in [ClubCategory::Putter, ClubCategory::All] {
        assert_eq!(
            start_level(&worked_baseline(), category).unwrap_err().field,
            "start_level.category",
            "{}",
            category.as_str()
        );
    }
}

// --- P15: the fixtures --------------------------------------------------------------------------------

const MID: ClubCategory = ClubCategory::MidIron;

/// Where a strike in a played block landed: read at `(h, v)`, or declared off the tape.
#[derive(Debug, Clone, Copy)]
enum Spot {
    At(f64, f64),
    Off,
}

use Spot::{At, Off};

fn land(shell: TapeStrike, spot: Spot) -> TapeStrike {
    match spot {
        At(h, v) => read(shell, h, v),
        Off => declared(shell),
    }
}

/// One block of a 7 iron's strikes, numbered from `first`, in `stage` at `level` (none in the
/// baseline), landing at `spots`.
fn played(first: i64, stage: DrillStage, level: Option<usize>, spots: &[Spot]) -> Vec<TapeStrike> {
    spots
        .iter()
        .zip(first..)
        .map(|(&spot, n)| land(shell(n, stage, level.map(|level| radii()[level])), spot))
        .collect()
}

/// One differential block at `level`, numbered from `first`, each strike called and landing.
fn called(first: i64, level: usize, calls: &[(StrikeCall, Spot)]) -> Vec<TapeStrike> {
    calls
        .iter()
        .zip(first..)
        .map(|(&(call, spot), n)| {
            let shell = shell(n, DrillStage::Differential, Some(radii()[level]));
            land(
                TapeStrike {
                    call: Some(call),
                    ..shell
                },
                spot,
            )
        })
        .collect()
}

fn spots(at: &[(f64, f64)]) -> Vec<Spot> {
    at.iter().map(|&(h, v)| At(h, v)).collect()
}

/// Ten spots, alternating `a` and `b`.
fn alternating(a: (f64, f64), b: (f64, f64)) -> Vec<Spot> {
    (0..10)
        .map(|i| {
            if i % 2 == 0 {
                At(a.0, a.1)
            } else {
                At(b.0, b.1)
            }
        })
        .collect()
}

/// Ten spots whose mean is `(2, −1)` exactly, each 5 mm from it: the worked pattern.
const AROUND: [(f64, f64); 10] = [
    (5.0, 3.0),
    (-1.0, -5.0),
    (6.0, -4.0),
    (-2.0, 2.0),
    (7.0, -1.0),
    (-3.0, -1.0),
    (2.0, 4.0),
    (2.0, -6.0),
    (5.0, -5.0),
    (-1.0, 3.0),
];

/// Strikes near centre, the first three balancing, so a block of them moves no pattern off centre.
const NEAR: [Spot; 7] = [
    At(0.0, 0.0),
    At(1.0, 1.0),
    At(-1.0, -1.0),
    At(1.0, -1.0),
    At(-1.0, 1.0),
    At(2.0, 0.0),
    At(-2.0, 0.0),
];

/// A ladder block at `level`, numbered from `first`: `n_inside` strikes near centre and the rest
/// declared off the tape, so it fails without moving the pattern off centre.
fn failing(first: i64, level: usize, n_inside: usize) -> Vec<TapeStrike> {
    let spots: Vec<Spot> = (0..10)
        .map(|i| if i < n_inside { NEAR[i] } else { Off })
        .collect();
    played(first, DrillStage::Ladder, Some(level), &spots)
}

/// Six on a 13 mm ring, balanced.
const RING: [Spot; 6] = [
    At(13.0, 0.0),
    At(-13.0, 0.0),
    At(0.0, 13.0),
    At(0.0, -13.0),
    At(5.0, 12.0),
    At(-5.0, -12.0),
];

/// A baseline with `n_on_ring` strikes on the 13 mm ring, inside the widest circle (14 mm) and
/// outside the next (12 mm), and the rest declared off the tape.
fn ring_baseline(n_on_ring: usize) -> Vec<TapeStrike> {
    let spots: Vec<Spot> = (0..10)
        .map(|i| if i < n_on_ring { RING[i] } else { Off })
        .collect();
    played(1, DrillStage::Baseline, None, &spots)
}

/// The history entry the facade writes for a judged block.
fn entry(block: &[TapeStrike], from: Option<usize>, to: Option<usize>) -> LevelEntry {
    LevelEntry {
        at: block.last().expect("a block").captured_at,
        from,
        to,
        block: counted(block)
            .iter()
            .map(|strike| strike.strike_id.clone())
            .collect(),
    }
}

/// A 7 iron's drill as the facade would leave it after each judged block: its strikes, oldest
/// first, and the club's state.
#[derive(Clone)]
struct Drill {
    strikes: Vec<TapeStrike>,
    state: ClubDrillState,
}

impl Drill {
    /// Before the first baseline.
    fn new() -> Self {
        Drill {
            strikes: vec![],
            state: ClubDrillState {
                stage: DrillStage::Baseline,
                level: None,
                block: vec![],
                suggestion: None,
                history: vec![],
                label: None,
            },
        }
    }

    /// On the ladder at `level`, with no judged block yet.
    fn at(level: usize) -> Self {
        let mut drill = Drill::new();
        drill.state.stage = DrillStage::Ladder;
        drill.state.level = Some(level);
        drill
    }

    /// `block` judged at the club's level, as the facade judges it: a baseline sets the level
    /// (through `level_or_widest`, the user's rule), a ladder block steps it, and a recentre or a
    /// differential block keeps it. The club leaves a baseline for the ladder and stays in any
    /// other stage.
    fn then(mut self, block: Vec<TapeStrike>) -> Self {
        let from = self.state.level;
        let stage = block[0].stage;
        let to = match stage {
            DrillStage::Baseline => level_or_widest(start_level(&block, MID).unwrap()),
            DrillStage::Ladder => {
                judge_block(&block, level_or_widest(from), MID)
                    .unwrap()
                    .new_level
            }
            DrillStage::Recentre | DrillStage::Differential => level_or_widest(from),
        };
        if stage != DrillStage::Baseline {
            for strike in &block {
                assert_eq!(
                    strike.level_radius_mm,
                    Some(radii()[level_or_widest(from)]),
                    "the fixture plays {} at the club's level",
                    strike.strike_id
                );
            }
        }
        self.state.history.push(entry(&block, from, Some(to)));
        self.state.level = Some(to);
        self.state.stage = match stage {
            DrillStage::Baseline => DrillStage::Ladder,
            other => other,
        };
        self.strikes.extend(block);
        self.state.validate().unwrap_or_else(|e| panic!("{e}"));
        self
    }

    /// Strikes stored and never judged: a block abandoned by a change of stage, or one in progress.
    fn abandoned(mut self, strikes: Vec<TapeStrike>) -> Self {
        self.strikes.extend(strikes);
        self
    }

    fn suggests(&self) -> Option<Suggestion> {
        suggestion(ClubId::SevenIron, &self.state, &self.strikes).unwrap_or_else(|e| panic!("{e}"))
    }

    fn pattern(&self) -> Pattern {
        pattern(&self.strikes).expect("a pattern")
    }
}

fn close(got: (f64, f64), want: (f64, f64)) {
    assert!(
        (got.0 - want.0).abs() < 1e-9 && (got.1 - want.1).abs() < 1e-9,
        "{got:?} against {want:?}"
    );
}

// --- The pattern (C12) ----------------------------------------------------------------------------

#[test]
fn the_pattern_is_the_mean_its_intervals_r50_and_spread_worked_by_hand() {
    assert_eq!(
        3.0_f64.hypot(4.0),
        5.0,
        "the premise: every spot is exactly 5 mm out"
    );
    let p = pattern(&played(1, DrillStage::Ladder, Some(0), &spots(&AROUND))).unwrap();
    assert_eq!(p.n, 10);
    assert_eq!(p.centre, (2.0, -1.0));
    // Every spot is 5 mm from (2, −1), so the median distance from the pattern's own centre is 5.
    assert_eq!(p.spread, 5.0);
    // From face centre the ten are √8, √10, √10, √20, √26, √34, √40, √50, √50 and √52 mm out, and
    // the median of ten is halfway between the fifth and the sixth.
    assert!((p.r50 - (26.0_f64.sqrt() + 34.0_f64.sqrt()) / 2.0).abs() < 1e-12);
    // t(0.975, 9) is 2.262, and the squared deviations from the mean sum to 118 on h and 132 on v.
    let half = |squares: f64| 2.262 * (squares / 9.0).sqrt() / 10.0_f64.sqrt();
    close(p.centre_ci.0, (2.0 - half(118.0), 2.0 + half(118.0)));
    close(p.centre_ci.1, (-1.0 - half(132.0), -1.0 + half(132.0)));
}

#[test]
fn no_pattern_below_ten_positioned_strikes_and_one_at_ten() {
    let mut strikes = played(1, DrillStage::Ladder, Some(0), &spots(&AROUND[..9]));
    // Counted but placed nowhere, stored but not counted, and aimed off centre: none is a
    // positioned strike of the pattern's stages.
    strikes.push(declared(on_ladder(10, 0)));
    strikes.push(not_read(on_ladder(11, 0), TapeNotRead::NoNewMark));
    strikes.extend(called(12, 0, &[(StrikeCall::Toe, At(-1.0, 3.0)); 5]));
    assert_eq!(pattern(&strikes), None);

    strikes.push(read(on_ladder(17, 0), -1.0, 3.0));
    let p = pattern(&strikes).unwrap();
    assert_eq!((p.n, p.centre, p.spread), (10, (2.0, -1.0), 5.0));
}

#[test]
fn the_pattern_is_the_last_twenty_positioned_strikes_of_the_baseline_ladder_and_recentre_stages() {
    // Five far toward the toe, then the worked ten twice, the second time in the recentre stage.
    let mut strikes = played(1, DrillStage::Baseline, None, &[At(12.0, 6.0); 5]);
    strikes.extend(played(6, DrillStage::Ladder, Some(0), &spots(&AROUND)));
    strikes.extend(played(16, DrillStage::Recentre, Some(1), &spots(&AROUND)));
    let p = pattern(&strikes).unwrap();
    assert_eq!((p.n, p.centre, p.spread), (20, (2.0, -1.0), 5.0));
    // One fewer of the worked ones, and the window reaches back to a far one.
    let p = pattern(&strikes[..24]).unwrap();
    assert_eq!(p.n, 20);
    assert!(p.centre.0 > 2.0, "{p:?}");
}

#[test]
fn differential_strikes_never_enter_the_pattern() {
    let mut strikes = played(1, DrillStage::Ladder, Some(0), &spots(&AROUND));
    strikes.extend(called(11, 0, &[(StrikeCall::Toe, At(13.0, 0.0)); 10]));
    let p = pattern(&strikes).unwrap();
    assert_eq!((p.n, p.centre), (10, (2.0, -1.0)));
    // Nine aimed at centre and ten at calls hold no pattern at all.
    assert_eq!(pattern(&strikes[1..]), None);
}

// --- Recentre (C13) -------------------------------------------------------------------------------

#[test]
fn recentre_is_suggested_past_a_third_of_the_radius_on_an_axis_and_not_at_it() {
    assert_eq!(
        RECENTRE_FRACTION * radii()[1],
        4.0,
        "the premise: a third of 12 mm is 4 mm exactly"
    );
    // A ladder block at the widest with every strike inside moves the club in to 12 mm. One axis
    // alternates about its mean, and the other's mean is 0.
    for (a, b, off) in [
        ((3.0, 1.0), (5.0, -1.0), None),
        ((3.001, 1.0), (5.001, -1.0), Some((true, false))),
        ((-3.0, 1.0), (-5.0, -1.0), None),
        ((-3.001, 1.0), (-5.001, -1.0), Some((true, false))),
        ((1.0, 3.0), (-1.0, 5.0), None),
        ((1.0, 3.001), (-1.0, 5.001), Some((false, true))),
        ((1.0, -3.0), (-1.0, -5.0), None),
        ((1.0, -3.001), (-1.0, -5.001), Some((false, true))),
    ] {
        let drill = Drill::at(0).then(played(1, DrillStage::Ladder, Some(0), &alternating(a, b)));
        assert_eq!(drill.state.level, Some(1), "the premise: {a:?} {b:?}");
        let (heel_toe, high_low) = off.unwrap_or((false, false));
        assert_eq!(
            off_centre(&drill.pattern(), 12.0),
            OffCentre { heel_toe, high_low },
            "{a:?} {b:?}"
        );
        assert_eq!(
            drill.suggests(),
            off.map(|_| Suggestion::Recentre),
            "{a:?} {b:?}"
        );
    }
}

#[test]
fn an_interval_holding_zero_suggests_nothing_however_far_the_mean_nor_does_a_sure_lean_on_the_other_axis(
) {
    // h alternates 13 and −3: a mean of 5 mm, past a third of 12, but spread so wide that its
    // interval holds 0.
    let wide = Drill::at(0).then(played(
        1,
        DrillStage::Ladder,
        Some(0),
        &alternating((13.0, 1.0), (-3.0, -1.0)),
    ));
    let p = wide.pattern();
    let ((h_low, _), _) = p.centre_ci;
    assert!(p.centre.0 > 4.0 && h_low < 0.0, "the premise: {p:?}");
    assert_eq!(wide.suggests(), None);

    // v alternates 1 and 2: its interval excludes 0, but 1.5 mm is well inside a third of 12. Each
    // axis holds one of the two conditions and neither holds both (P1 finding 6).
    let lean = Drill::at(0).then(played(
        1,
        DrillStage::Ladder,
        Some(0),
        &alternating((13.0, 1.0), (-3.0, 2.0)),
    ));
    let p = lean.pattern();
    let (_, (v_low, _)) = p.centre_ci;
    assert!(v_low > 0.0 && p.centre.1 < 4.0, "the premise: {p:?}");
    assert_eq!(
        off_centre(&p, 12.0),
        OffCentre {
            heel_toe: false,
            high_low: false
        }
    );
    assert_eq!(lean.suggests(), None);
}

#[test]
fn a_recentre_block_meets_its_goal_with_each_mean_at_a_third_of_the_radius_and_not_past_it() {
    // At 12 mm the goal is each axis's mean within 4 mm.
    for (a, b, met) in [
        ((3.0, 1.0), (5.0, -1.0), true),
        ((3.001, 1.0), (5.001, -1.0), false),
        ((-3.0, 1.0), (-5.0, -1.0), true),
        ((-3.001, 1.0), (-5.001, -1.0), false),
        ((1.0, 3.0), (-1.0, 5.0), true),
        ((1.0, -3.001), (-1.0, -5.001), false),
    ] {
        let block = played(1, DrillStage::Recentre, Some(1), &alternating(a, b));
        let judged = judge_recentre(&block, 1, MID).unwrap();
        assert_eq!((judged.counted, judged.goal_met), (10, met), "{a:?} {b:?}");
        let drill = Drill::at(1).then(block);
        assert_eq!(
            drill.suggests(),
            met.then_some(Suggestion::ReturnToLadder),
            "{a:?} {b:?}"
        );
        // A block that missed its goal is off centre by its own pattern, and the recentre stage
        // does not suggest recentring again.
        assert_eq!(
            off_centre(&drill.pattern(), 12.0).any(),
            !met,
            "{a:?} {b:?}"
        );
    }
    assert_eq!(
        judge_recentre(
            &played(
                1,
                DrillStage::Recentre,
                Some(1),
                &alternating((3.0, 1.0), (5.0, -1.0))
            ),
            1,
            MID
        ),
        Ok(RecentreResult {
            counted: 10,
            mean: Some((4.0, 0.0)),
            goal_met: true
        })
    );
}

#[test]
fn a_recentre_block_with_a_strike_off_the_tape_has_no_mean_and_does_not_meet_its_goal() {
    let mut spots = alternating((0.5, 0.0), (-0.5, 0.0));
    spots[9] = Off;
    let block = played(1, DrillStage::Recentre, Some(1), &spots);
    assert_eq!(
        judge_recentre(&block, 1, MID),
        Ok(RecentreResult {
            counted: 10,
            mean: None,
            goal_met: false
        })
    );
    assert_eq!(Drill::at(1).then(block).suggests(), None);
}

// --- Differential practice (C13) ------------------------------------------------------------------

#[test]
fn differential_is_suggested_after_two_ladder_blocks_in_a_row_that_failed_at_the_widest() {
    let once = Drill::new().then(ring_baseline(6)).then(failing(11, 0, 3));
    assert_eq!(
        once.state.level,
        Some(0),
        "the premise: the ring starts at the widest"
    );
    assert_eq!(once.suggests(), None);
    assert_eq!(
        once.clone().then(failing(21, 0, 3)).suggests(),
        Some(Suggestion::Differential)
    );
    // Four inside is a block that stays, not one that earned a move out.
    assert_eq!(once.then(failing(21, 0, 4)).suggests(), None);
}

#[test]
fn a_block_that_moved_out_to_the_widest_did_not_fail_at_it() {
    let drill = Drill::at(1).then(failing(1, 1, 3));
    assert_eq!(drill.state.level, Some(0), "the premise: it moved out");
    let drill = drill.then(failing(11, 0, 3));
    assert_eq!(drill.suggests(), None);
    assert_eq!(
        drill.then(failing(21, 0, 3)).suggests(),
        Some(Suggestion::Differential)
    );
}

#[test]
fn a_judged_recentre_block_breaks_the_row_and_an_abandoned_block_does_not() {
    let recentring = played(
        11,
        DrillStage::Recentre,
        Some(0),
        &alternating((1.0, 0.0), (-1.0, 0.0)),
    );
    let broken = Drill::at(0)
        .then(failing(1, 0, 3))
        .then(recentring)
        .then(failing(21, 0, 3));
    assert_eq!(broken.suggests(), None);
    assert_eq!(
        broken.then(failing(31, 0, 3)).suggests(),
        Some(Suggestion::Differential)
    );

    let abandoned = Drill::at(0)
        .then(failing(1, 0, 3))
        .abandoned(played(
            11,
            DrillStage::Recentre,
            Some(0),
            &[At(1.0, 0.0), At(-1.0, 0.0)],
        ))
        .then(failing(21, 0, 3));
    assert_eq!(abandoned.suggests(), Some(Suggestion::Differential));
}

#[test]
fn differential_is_suggested_after_a_baseline_whose_widest_circle_holds_four_and_not_five() {
    assert_eq!(start_level(&ring_baseline(4), MID), Ok(None));
    assert_eq!(level_or_widest(None), 0);
    assert_eq!(level_or_widest(Some(3)), 3);
    // The club starts at the widest (the user's rule), and is offered differential practice.
    let four = Drill::new().then(ring_baseline(4));
    assert_eq!(four.state.level, Some(0));
    assert_eq!(four.suggests(), Some(Suggestion::Differential));
    let five = Drill::new().then(ring_baseline(5));
    assert_eq!(five.state.level, Some(0));
    assert_eq!(five.suggests(), None);

    // A club left in the baseline with no level, which validates, is read at the widest too.
    let mut unmapped = four.state.clone();
    unmapped.stage = DrillStage::Baseline;
    unmapped.level = None;
    unmapped.history[0].to = None;
    unmapped.validate().unwrap();
    assert_eq!(
        suggestion(ClubId::SevenIron, &unmapped, &four.strikes),
        Ok(Some(Suggestion::Differential))
    );
}

#[test]
fn recentre_outranks_differential_practice_when_both_hold() {
    // A tight pattern 15 mm toward the toe: outside even the widest circle, so a baseline of it
    // names no start level and ladder blocks of it fail at the widest, and clearly off centre.
    let toward_toe =
        |first, stage, level| played(first, stage, level, &alternating((14.5, 0.5), (15.5, -0.5)));
    let baseline = toward_toe(1, DrillStage::Baseline, None);
    assert_eq!(start_level(&baseline, MID), Ok(None));
    assert_eq!(
        Drill::new().then(baseline).suggests(),
        Some(Suggestion::Recentre)
    );

    let block = toward_toe(21, DrillStage::Ladder, Some(0));
    assert!(judge_block(&block, 0, MID).unwrap().at_widest_out);
    let drill = Drill::new()
        .then(ring_baseline(6))
        .then(toward_toe(11, DrillStage::Ladder, Some(0)))
        .then(block);
    assert_eq!(drill.suggests(), Some(Suggestion::Recentre));
}

#[test]
fn a_differential_block_of_seven_calls_made_suggests_a_fresh_baseline_and_six_does_not() {
    use StrikeCall::{Centre, Heel, Toe};
    assert_eq!(radii()[3], 9.0, "the premise");
    // Seven made, four of them on an edge, then three missed, one by declaring the ball off the
    // tape.
    let mut calls = [
        (Centre, At(0.0, 0.0)),
        (Centre, At(0.0, 9.0)),
        (Heel, At(-9.0, 0.0)),
        (Heel, At(-9.0, 4.0)),
        (Toe, At(9.0, 0.0)),
        (Toe, At(11.0, -3.0)),
        (Centre, At(1.0, -2.0)),
        (Heel, At(-8.999, 0.0)),
        (Toe, At(0.0, 0.0)),
        (Centre, Off),
    ];
    // Before it, a ladder block that stayed at 9 mm and left the pattern off toward the toe, which a
    // differential block does not answer with recentring.
    let drill = Drill::at(3).then(played(
        1,
        DrillStage::Ladder,
        Some(3),
        &alternating((5.0, 0.0), (9.5, 0.0)),
    ));
    assert_eq!(drill.state.level, Some(3), "the premise: it stayed");
    assert!(off_centre(&drill.pattern(), 9.0).any(), "the premise");

    let seven = called(11, 3, &calls);
    assert_eq!(
        judge_differential(&seven, 3, MID),
        Ok(DifferentialResult {
            made: 7,
            counted: 10,
            fresh_baseline: true
        })
    );
    assert_eq!(
        drill.clone().then(seven).suggests(),
        Some(Suggestion::FreshBaseline)
    );

    calls[6] = (Centre, At(9.0, 0.001));
    let six = called(11, 3, &calls);
    assert_eq!(judge_differential(&six, 3, MID).unwrap().made, 6);
    assert_eq!(drill.then(six).suggests(), None);
}

#[test]
fn each_call_is_made_at_its_edge_and_missed_just_short_of_it() {
    use StrikeCall::{Centre, Heel, Toe};
    let at = |h, v| called(1, 0, &[(Centre, At(h, v))]).remove(0);
    let edges = [
        // A centre call: hypot(6, 8) is 10.
        (at(6.0, 8.0), Centre, 10.0, true),
        (at(6.0, 8.0), Centre, 9.999, false),
        (at(-10.0, 0.0), Centre, 10.0, true),
        // A heel call is h ≤ −R, whatever v.
        (at(-10.0, 0.0), Heel, 10.0, true),
        (at(-10.0, 7.0), Heel, 10.0, true),
        (at(-9.999, 0.0), Heel, 10.0, false),
        (at(10.0, 0.0), Heel, 10.0, false),
        // A toe call is h ≥ +R, whatever v.
        (at(10.0, 0.0), Toe, 10.0, true),
        (at(10.0, -7.0), Toe, 10.0, true),
        (at(9.999, 0.0), Toe, 10.0, false),
        (at(0.0, 0.0), Toe, 10.0, false),
    ];
    for (strike, call, radius, made) in &edges {
        assert_eq!(
            call_made(strike, *call, *radius),
            Some(*made),
            "{call:?} at {:?} against {radius}",
            strike.position()
        );
    }
    // A picked strike is judged where it was tapped.
    let tapped = picked(
        TapeStrike {
            call: Some(Heel),
            ..shell(2, DrillStage::Differential, Some(10.0))
        },
        -10.0,
        0.0,
    );
    assert_eq!(call_made(&tapped, Heel, 10.0), Some(true));
    // Off the tape makes no call, and not read is neither made nor missed.
    let off = called(3, 0, &[(Centre, Off)]).remove(0);
    let unread = not_read(
        TapeStrike {
            call: Some(Centre),
            ..shell(4, DrillStage::Differential, Some(10.0))
        },
        TapeNotRead::NoNewMark,
    );
    for call in [Centre, Heel, Toe] {
        assert_eq!(call_made(&off, call, 10.0), Some(false), "{call:?}");
        assert_eq!(call_made(&unread, call, 10.0), None, "{call:?}");
    }
}

// --- What a suggestion reads ----------------------------------------------------------------------

#[test]
fn a_declined_suggestion_is_offered_again_after_each_block_it_still_holds_for() {
    let failed = Drill::new()
        .then(ring_baseline(6))
        .then(failing(11, 0, 3))
        .then(failing(21, 0, 3));
    assert_eq!(failed.suggests(), Some(Suggestion::Differential));
    // Pending or declined, the answer is the same, because the state's suggestion is never read.
    let mut pending = failed.state.clone();
    pending.suggestion = Some(Suggestion::Recentre);
    assert_eq!(
        suggestion(ClubId::SevenIron, &pending, &failed.strikes),
        Ok(Some(Suggestion::Differential))
    );
    // Declined, the golfer plays on, fails again, and is offered it again.
    assert_eq!(
        failed.then(failing(31, 0, 3)).suggests(),
        Some(Suggestion::Differential)
    );

    // Recentring the same way: declined at 12 mm, and offered again at 10 mm after the next block.
    let off = |first, level| {
        played(
            first,
            DrillStage::Ladder,
            Some(level),
            &alternating((3.001, 1.0), (5.001, -1.0)),
        )
    };
    let drill = Drill::at(0).then(off(1, 0));
    assert_eq!(drill.suggests(), Some(Suggestion::Recentre));
    let drill = drill.then(off(11, 1));
    assert_eq!(drill.state.level, Some(2), "the premise: it moved in again");
    assert_eq!(drill.suggests(), Some(Suggestion::Recentre));
}

#[test]
fn nothing_is_suggested_before_a_block_is_judged() {
    assert_eq!(Drill::new().suggests(), None);
    let in_progress = Drill::new().abandoned(played(1, DrillStage::Baseline, None, &[Off; 9]));
    assert_eq!(in_progress.suggests(), None);
}

#[test]
fn only_the_clubs_own_strikes_are_read_and_a_history_naming_any_other_is_refused() {
    let drill = Drill::at(0).then(played(
        1,
        DrillStage::Ladder,
        Some(0),
        &alternating((3.0, 1.0), (5.0, -1.0)),
    ));
    assert_eq!(drill.suggests(), None);
    // An 8 iron's ten, far toward the toe, which would put the 7 iron's pattern off centre if they
    // were read with it.
    let eight: Vec<TapeStrike> = played(101, DrillStage::Ladder, Some(0), &[At(12.0, 0.0); 10])
        .into_iter()
        .map(|strike| {
            checked(TapeStrike {
                club: ClubId::EightIron,
                category: category_of(ClubId::EightIron),
                ..strike
            })
        })
        .collect();
    let mut strikes = drill.strikes.clone();
    strikes.extend(eight.clone());
    assert!(
        off_centre(&pattern(&strikes).unwrap(), 12.0).any(),
        "the premise"
    );
    assert_eq!(
        suggestion(ClubId::SevenIron, &drill.state, &strikes),
        Ok(None)
    );

    // An entry naming the 8 iron's strikes, or a strike that is not stored, is refused.
    let mut state = drill.state.clone();
    state.history.push(entry(&eight, Some(1), Some(1)));
    assert_eq!(
        suggestion(ClubId::SevenIron, &state, &strikes)
            .unwrap_err()
            .field,
        "suggestion.history"
    );
    assert_eq!(
        suggestion(ClubId::SevenIron, &drill.state, &drill.strikes[1..])
            .unwrap_err()
            .field,
        "suggestion.history"
    );
    // So is an entry of no strikes, and one whose strikes were played in two stages.
    let mut empty = drill.state.clone();
    empty.history.push(LevelEntry {
        block: vec![],
        ..entry(&drill.strikes, Some(1), Some(1))
    });
    assert_eq!(
        suggestion(ClubId::SevenIron, &empty, &drill.strikes)
            .unwrap_err()
            .field,
        "suggestion.history"
    );
    let mut mixed = played(201, DrillStage::Ladder, Some(1), &[At(0.0, 0.0); 9]);
    mixed.extend(played(210, DrillStage::Recentre, Some(1), &[At(0.0, 0.0)]));
    let mut state = drill.state.clone();
    state.history.push(entry(&mixed, Some(1), Some(1)));
    let mut strikes = drill.strikes.clone();
    strikes.extend(mixed);
    assert_eq!(
        suggestion(ClubId::SevenIron, &state, &strikes)
            .unwrap_err()
            .field,
        "suggestion.history"
    );
}

type Judge = fn(&[TapeStrike], usize, ClubCategory) -> Result<(), ContractError>;

/// `judge`, which reports refusals under `owner`, refuses as `judge_block` does: nine counted
/// strikes, a level past the table and a category with no ladder.
fn refused_as_a_ladder_block_is(owner: &str, judge: Judge, block: &[TapeStrike]) {
    assert_eq!(
        judge(&block[..9], 1, MID).unwrap_err().field,
        format!("{owner}.block")
    );
    assert_eq!(
        judge(block, radii().len(), MID).unwrap_err().field,
        format!("{owner}.level")
    );
    for category in [ClubCategory::Putter, ClubCategory::All] {
        assert_eq!(
            judge(block, 0, category).unwrap_err().field,
            format!("{owner}.category")
        );
    }
}

#[test]
fn a_recentre_or_differential_block_is_refused_as_a_ladder_block_is() {
    refused_as_a_ladder_block_is(
        "judge_recentre",
        |block, level, category| judge_recentre(block, level, category).map(|_| ()),
        &played(1, DrillStage::Recentre, Some(1), &[At(0.0, 0.0); 10]),
    );
    let differential = called(1, 1, &[(StrikeCall::Centre, At(0.0, 0.0)); 10]);
    refused_as_a_ladder_block_is(
        "judge_differential",
        |block, level, category| judge_differential(block, level, category).map(|_| ()),
        &differential,
    );
    // And a differential block holding a strike that was never called.
    let mut uncalled = differential[..9].to_vec();
    uncalled.extend(played(11, DrillStage::Ladder, Some(1), &[At(0.0, 0.0)]));
    assert_eq!(
        judge_differential(&uncalled, 1, MID).unwrap_err().field,
        "judge_differential.block"
    );
}
