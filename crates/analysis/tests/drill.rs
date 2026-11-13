//! `analysis::drill`: which strikes count, inside a circle at and either side of its edge, the
//! baseline's start level, and a ladder block's step at the middle and both ends of a table (M46
//! P14).
//!
//! **No Python twin and no vector**, as `analysis::strike`'s rules have none: each test holds one rule
//! to the M46 plan's calls, C10 (what counts, inside, a block's step) and C11 (the baseline). The
//! strikes are a 7 iron's, a `mid_iron`, and every one passes `Validate`, so no rule here is held on a
//! strike the store would refuse. The tables are read from `ladder_radii` rather than copied, except
//! where a worked case names its radii, and that case asserts them first: P29's floor removes the
//! tightest radii (the plan's C9), and these tests should move with it rather than pin today's.

use analysis::drill::{counted, inside, judge_block, start_level, BlockResult, LadderStep};
use contracts::club::ClubId;
use contracts::drill::{
    crop_file, ladder_radii, strike_id, DrillStage, StrikeSource, TapeNotRead, TapeStrike,
};
use contracts::face_impact::TAPE_READER_VERSION;
use contracts::intent::ClubCategory;
use contracts::{Timestamp, Validate};

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

/// A 7 iron's strike numbered `n` (at most 59), in `stage` and judged against `radius_mm`, captured
/// `n` seconds into the minute, before anything says where it landed.
fn shell(n: i64, stage: DrillStage, radius_mm: Option<f64>) -> TapeStrike {
    TapeStrike {
        strike_id: strike_id(INSTALL, n),
        player_id: "aaron".into(),
        club: ClubId::SevenIron,
        category: ClubCategory::MidIron,
        label_type: "strikepro_iron".into(),
        label_id: "3fa2-L1".into(),
        n_on_label: n,
        captured_at: Timestamp::parse(&format!("2026-11-13T07:30:{n:02}Z")).unwrap(),
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
