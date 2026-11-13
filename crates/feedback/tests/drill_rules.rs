//! The impact drill's words, each pinned whole for a named input. [M46 P16]
//!
//! Nothing records an oracle for these strings, so each test states the line or sentence a facade
//! would get for an input it could send, singular and plural both where a count is printed. Two
//! sweeps close the file: no number in any of them is printed past a tenth, and lines and sentences
//! keep to their kinds (the module doc of `feedback::drill_rules`).
//!
//! The inputs stand in for `analysis::drill`'s results, which this crate cannot reach (the edge
//! `tests/profile_rules.rs` pins); each test names the result its numbers come from.

use contracts::club::ClubId;
use contracts::drill::{
    crop_file, ladder_radii, strike_id, DrillStage, StrikeCall, StrikeSource, TapeNotRead,
    TapeStrike,
};
use contracts::face_impact::TAPE_READER_VERSION;
use contracts::intent::ClubCategory;
use contracts::{Timestamp, Validate};
use feedback::drill_rules::{
    baseline_progress, baseline_tip, call_name, call_verdict, differential_progress,
    differential_result, differential_suggestion, fresh_baseline_suggestion, heat_map_needs,
    inside_verdict, ladder_progress, ladder_result, level_line, offset_line, pattern_centre_line,
    pattern_needs, position_line, r50_line, radius_text, recentre_progress, recentre_result,
    recentre_suggestion, return_to_ladder_suggestion, spread_line, stage_name, DifferentialWhy,
    DEAD_CENTRE_MM, NO_LABEL_FITS, NO_LEVEL_YET,
};

const INSTALL: &str = "3fa2";

const EVERY_CATEGORY: [ClubCategory; 9] = [
    ClubCategory::Driver,
    ClubCategory::Wood,
    ClubCategory::Hybrid,
    ClubCategory::LongIron,
    ClubCategory::MidIron,
    ClubCategory::ShortIron,
    ClubCategory::Wedge,
    ClubCategory::Putter,
    ClubCategory::All,
];

const EVERY_REASON: [TapeNotRead; 5] = [
    TapeNotRead::NoLabel,
    TapeNotRead::NoOrientation,
    TapeNotRead::NoNewMark,
    TapeNotRead::MarkClipped,
    TapeNotRead::DecodeFailed,
];

/// A ladder strike on the 7 iron at its widest circle, with no position and no source yet.
fn shell() -> TapeStrike {
    TapeStrike {
        strike_id: strike_id(INSTALL, 1),
        player_id: "aaron".into(),
        club: ClubId::SevenIron,
        category: ClubCategory::MidIron,
        label_type: "strikepro_iron".into(),
        label_id: "3fa2-L1".into(),
        n_on_label: 1,
        captured_at: Timestamp::parse("2026-11-13T07:00:01Z").unwrap(),
        stage: DrillStage::Ladder,
        level_radius_mm: Some(ladder_radii(ClubCategory::MidIron)[0]),
        call: None,
        offset_h_mm: None,
        offset_v_mm: None,
        source: None,
        not_read: None,
        reader_version: TAPE_READER_VERSION,
        crop: None,
    }
}

/// Only a strike that validates can be stored, so only one is shown.
fn checked(strike: TapeStrike) -> TapeStrike {
    strike.validate().unwrap_or_else(|e| panic!("{e}"));
    strike
}

fn placed(source: StrikeSource, h: f64, v: f64) -> TapeStrike {
    let shell = shell();
    let crop = crop_file(&shell.strike_id);
    checked(TapeStrike {
        offset_h_mm: Some(h),
        offset_v_mm: Some(v),
        source: Some(source),
        crop: Some(crop),
        ..shell
    })
}

fn declared() -> TapeStrike {
    checked(TapeStrike {
        source: Some(StrikeSource::DeclaredOffTape),
        ..shell()
    })
}

fn not_read(reason: TapeNotRead) -> TapeStrike {
    checked(TapeStrike {
        not_read: Some(reason),
        ..shell()
    })
}

// --- The reading card ---------------------------------------------------------------------------

#[test]
fn the_offset_line_names_each_axis_by_its_side() {
    assert_eq!(offset_line(8.4, 2.1), "8.4 mm toe, 2.1 mm high");
    assert_eq!(offset_line(-8.4, -2.1), "8.4 mm heel, 2.1 mm low");
    assert_eq!(offset_line(8.44, -12.96), "8.4 mm toe, 13.0 mm low");
}

#[test]
fn dead_centre_is_within_half_a_millimetre_on_both_axes_and_the_edge_is_inside() {
    assert_eq!(DEAD_CENTRE_MM, 0.5);
    for (h, v) in [(0.0, 0.0), (0.5, -0.5), (-0.5, 0.5), (0.04, -0.04)] {
        assert_eq!(offset_line(h, v), "dead centre", "({h}, {v})");
    }
    assert_eq!(offset_line(0.51, 0.0), "0.5 mm toe");
    assert_eq!(offset_line(0.0, -0.51), "0.5 mm low");
}

#[test]
fn an_axis_within_dead_centre_is_left_out_rather_than_given_a_side() {
    assert_eq!(offset_line(8.4, 0.3), "8.4 mm toe");
    assert_eq!(offset_line(-0.2, 2.1), "2.1 mm high");
    // What the rule exists for: a tenth that rounds to nothing is never printed with a side.
    assert_eq!(offset_line(-0.04, -6.0), "6.0 mm low");
    assert!(!offset_line(0.04, 6.0).contains("0.0"));
}

#[test]
fn the_position_line_says_how_the_position_was_got() {
    assert_eq!(
        position_line(&placed(StrikeSource::Read, 8.4, 2.1)),
        "8.4 mm toe, 2.1 mm high"
    );
    assert_eq!(
        position_line(&placed(StrikeSource::Picked, -3.0, 0.2)),
        "3.0 mm heel (picked)"
    );
    assert_eq!(
        position_line(&placed(StrikeSource::Picked, 0.1, 0.2)),
        "dead centre (picked)"
    );
    assert_eq!(position_line(&declared()), "off the tape");
    for reason in EVERY_REASON {
        assert_eq!(position_line(&not_read(reason)), "not read", "{reason:?}");
    }
}

#[test]
fn the_verdicts_name_the_circle_and_the_call() {
    assert_eq!(inside_verdict(true, 12.0), "Inside 12 mm");
    assert_eq!(inside_verdict(false, 12.0), "Outside 12 mm");
    assert_eq!(call_name(StrikeCall::Heel), "Heel");
    assert_eq!(call_name(StrikeCall::Centre), "Centre");
    assert_eq!(call_name(StrikeCall::Toe), "Toe");
    assert_eq!(call_verdict(StrikeCall::Heel, true), "Heel call: made");
    assert_eq!(
        call_verdict(StrikeCall::Centre, false),
        "Centre call: missed"
    );
    assert_eq!(call_verdict(StrikeCall::Toe, true), "Toe call: made");
}

// --- Where the club is -----------------------------------------------------------------------------

#[test]
fn each_stage_has_its_name() {
    assert_eq!(stage_name(DrillStage::Baseline), "Baseline");
    assert_eq!(stage_name(DrillStage::Ladder), "Ladder");
    assert_eq!(stage_name(DrillStage::Recentre), "Recentring");
    assert_eq!(
        stage_name(DrillStage::Differential),
        "Differential practice"
    );
}

/// The widest radii are asserted as numbers because P29's floor removes only the tightest, and the
/// table's length is read from the table so that the floor moves the line rather than breaking it.
#[test]
fn the_level_line_counts_from_one_at_the_widest() {
    let driver = ladder_radii(ClubCategory::Driver);
    assert_eq!(driver[0], 22.0);
    assert_eq!(
        level_line(0, ClubCategory::Driver).as_deref(),
        Some(format!("Level 1 of {}: 22 mm", driver.len()).as_str())
    );
    let mid = ladder_radii(ClubCategory::MidIron);
    assert_eq!(mid[1], 12.0);
    assert_eq!(
        level_line(1, ClubCategory::MidIron).as_deref(),
        Some(format!("Level 2 of {}: 12 mm", mid.len()).as_str())
    );
    let tightest = driver.len() - 1;
    assert_eq!(
        level_line(tightest, ClubCategory::Driver),
        Some(format!(
            "Level {n} of {n}: {}",
            radius_text(driver[tightest]),
            n = driver.len()
        ))
    );
}

#[test]
fn no_level_line_past_the_table_or_for_a_category_with_no_ladder() {
    let driver = ladder_radii(ClubCategory::Driver);
    assert_eq!(level_line(driver.len(), ClubCategory::Driver), None);
    assert_eq!(level_line(0, ClubCategory::Putter), None);
    assert_eq!(level_line(0, ClubCategory::All), None);
    assert_eq!(NO_LEVEL_YET, "No level yet: the baseline sets it");
    assert_eq!(NO_LABEL_FITS, "No STRIKEPRO label fits this club");
}

#[test]
fn every_radius_prints_as_its_table_holds_it() {
    for category in EVERY_CATEGORY {
        for &radius in ladder_radii(category) {
            let text = radius_text(radius);
            let number = text.strip_suffix(" mm").expect("a radius ends in mm");
            assert_eq!(number.parse::<f64>().unwrap(), radius, "{category:?}");
            assert!(!number.ends_with(".0"), "{text}");
        }
    }
}

// --- The block in progress ----------------------------------------------------------------------------

#[test]
fn the_progress_lines_count_toward_the_block() {
    assert_eq!(baseline_progress(6), "Baseline: 6 of 10");
    assert_eq!(baseline_progress(0), "Baseline: 0 of 10");
    assert_eq!(ladder_progress(6, 4), "Block: 6 of 10, 4 inside");
    assert_eq!(ladder_progress(1, 1), "Block: 1 of 10, 1 inside");
    assert_eq!(recentre_progress(6), "Recentring block: 6 of 10");
}

#[test]
fn the_differential_line_counts_calls_singular_and_plural() {
    assert_eq!(
        differential_progress(6, 4),
        "Differential block: 6 of 10, 4 calls made"
    );
    assert_eq!(
        differential_progress(3, 1),
        "Differential block: 3 of 10, 1 call made"
    );
    assert_eq!(
        differential_progress(0, 0),
        "Differential block: 0 of 10, 0 calls made"
    );
}

// --- A judged block -----------------------------------------------------------------------------------

#[test]
fn the_baseline_tip_says_where_half_landed_and_the_target() {
    // The plan's own example, with R50 printed to a tenth as every measured distance is.
    assert_eq!(
        baseline_tip(Some(14.0), 12.0, true),
        "Half your strikes landed within 14.0 mm. Your target: 7 of 10 inside 12 mm."
    );
}

/// `start_level` answered `None`, so the club starts at the widest (P1 finding 5), and the tip
/// says why.
#[test]
fn a_baseline_no_circle_held_starts_at_the_widest_and_says_so() {
    assert_eq!(
        baseline_tip(Some(24.27), 22.0, false),
        "Half your strikes landed within 24.3 mm. No circle held 5 of your 10 strikes, so you \
         start at the widest. Your target: 7 of 10 inside 22 mm."
    );
}

/// P15 finding 4: a baseline's median cannot always be placed when a strike was declared off the
/// tape, and the tip says so rather than printing one.
#[test]
fn a_baseline_with_no_r50_says_a_strike_was_off_the_tape() {
    assert_eq!(
        baseline_tip(None, 12.0, true),
        "Your baseline had a strike off the tape, so how close half your strikes landed is not \
         known. Your target: 7 of 10 inside 12 mm."
    );
    assert_eq!(
        baseline_tip(None, 22.0, false),
        "Your baseline had a strike off the tape, so how close half your strikes landed is not \
         known. No circle held 5 of your 10 strikes, so you start at the widest. Your target: 7 \
         of 10 inside 22 mm."
    );
}

/// Each input is a `BlockResult` `judge_block` can give, with its levels' radii.
#[test]
fn a_ladder_result_names_its_step_and_the_next_target() {
    assert_eq!(
        ladder_result(8, 10, 14.0, 12.0, false, false),
        "8 of 10 inside 14 mm. Moving in a level. Your target: 7 of 10 inside 12 mm."
    );
    assert_eq!(
        ladder_result(3, 10, 12.0, 14.0, false, false),
        "3 of 10 inside 12 mm. Moving out a level. Your target: 7 of 10 inside 14 mm."
    );
    assert_eq!(
        ladder_result(5, 10, 12.0, 12.0, false, false),
        "5 of 10 inside 12 mm. The level stays. Your target: 7 of 10 inside 12 mm."
    );
    assert_eq!(
        ladder_result(1, 10, 12.0, 14.0, false, false),
        "1 of 10 inside 12 mm. Moving out a level. Your target: 7 of 10 inside 14 mm."
    );
}

#[test]
fn a_ladder_result_at_a_table_end_says_why_the_level_stayed() {
    assert_eq!(
        ladder_result(9, 10, 10.0, 10.0, true, false),
        "9 of 10 inside 10 mm. That is the tightest circle, so the level stays. Your target: 7 of \
         10 inside 10 mm."
    );
    assert_eq!(
        ladder_result(2, 10, 22.0, 22.0, false, true),
        "2 of 10 inside 22 mm. That is the widest circle, so the level stays. Your target: 7 of \
         10 inside 22 mm."
    );
}

/// `RecentreResult`'s three shapes. The goal is a third of the radius, worked and printed to a
/// tenth: 4.0 mm at 12, 4.7 mm at 14.
#[test]
fn a_recentre_result_says_where_the_block_centred_or_why_it_cannot() {
    assert_eq!(
        recentre_result(Some((1.2, 0.3)), true, 12.0),
        "Block centre: 1.2 mm toe. Goal met: within 4.0 mm of the label's centre on both axes."
    );
    assert_eq!(
        recentre_result(Some((5.3, -1.0)), false, 14.0),
        "Block centre: 5.3 mm toe, 1.0 mm low. The goal is within 4.7 mm of the label's centre on \
         both axes."
    );
    assert_eq!(
        recentre_result(None, false, 12.0),
        "Block centre: not known, because the block had a strike off the tape. The goal is within \
         4.0 mm of the label's centre on both axes."
    );
}

#[test]
fn a_differential_result_counts_its_calls_against_the_target() {
    assert_eq!(
        differential_result(4, 10, false),
        "4 of 10 calls made. Your target: 7 of 10."
    );
    assert_eq!(
        differential_result(8, 10, true),
        "8 of 10 calls made. Target met: 7 of 10."
    );
    assert_eq!(
        differential_result(1, 10, false),
        "1 of 10 calls made. Your target: 7 of 10."
    );
}

// --- The suggestions --------------------------------------------------------------------------------

/// `centre` is a `Pattern::centre` and the flags are `OffCentre`'s: each axis it found off is named
/// with its side, and an axis it did not find off is not, whatever its mean.
#[test]
fn the_recentre_suggestion_names_each_off_axis_and_only_those() {
    assert_eq!(
        recentre_suggestion((6.04, 2.0), true, false),
        "Your strikes sit 6.0 mm toward the toe. Move the pattern before shrinking it."
    );
    assert_eq!(
        recentre_suggestion((-6.0, 2.0), true, false),
        "Your strikes sit 6.0 mm toward the heel. Move the pattern before shrinking it."
    );
    assert_eq!(
        recentre_suggestion((3.0, 4.2), false, true),
        "Your strikes sit 4.2 mm high. Move the pattern before shrinking it."
    );
    assert_eq!(
        recentre_suggestion((0.2, -4.2), false, true),
        "Your strikes sit 4.2 mm low. Move the pattern before shrinking it."
    );
    assert_eq!(
        recentre_suggestion((-6.0, -4.2), true, true),
        "Your strikes sit 6.0 mm toward the heel and 4.2 mm low. Move the pattern before shrinking \
         it."
    );
    assert_eq!(
        recentre_suggestion((1.0, 1.0), false, false),
        "Your strikes sit off centre. Move the pattern before shrinking it."
    );
}

#[test]
fn the_differential_suggestion_says_which_trigger_fired() {
    assert_eq!(
        differential_suggestion(DifferentialWhy::Baseline, 22.0),
        "Fewer than 5 of your 10 baseline strikes landed inside the widest circle, 22 mm. Try \
         differential practice: hit the spot I call."
    );
    assert_eq!(
        differential_suggestion(DifferentialWhy::AtWidest, 14.0),
        "Your last 2 blocks each had 3 or fewer of 10 inside the widest circle, 14 mm. Try \
         differential practice: hit the spot I call."
    );
}

#[test]
fn the_return_and_fresh_baseline_suggestions_state_what_comes_next() {
    assert_eq!(
        return_to_ladder_suggestion(12.0),
        "This block's centre was within 4.0 mm of the label's centre on both axes. Go back to the \
         ladder. Your target: 7 of 10 inside 12 mm."
    );
    assert_eq!(
        fresh_baseline_suggestion(8, 10),
        "You made 8 of 10 calls. Take a fresh baseline: 10 strikes aimed at centre, and the ladder \
         starts again from it."
    );
}

// --- The progress view --------------------------------------------------------------------------------

#[test]
fn the_pattern_lines_print_its_centre_spread_and_r50() {
    assert_eq!(
        pattern_centre_line((3.2, -1.0)),
        "Pattern centre: 3.2 mm toe, 1.0 mm low"
    );
    assert_eq!(
        pattern_centre_line((0.3, -0.1)),
        "Pattern centre: dead centre"
    );
    assert_eq!(
        spread_line(6.54),
        "Spread: half within 6.5 mm of the pattern's centre"
    );
    assert_eq!(
        r50_line(9.4),
        "Half your strikes within 9.4 mm of the label's centre"
    );
}

#[test]
fn the_pattern_and_heat_map_say_how_many_more_they_need_until_they_show() {
    assert_eq!(
        pattern_needs(0).as_deref(),
        Some("The pattern needs 10 more strikes placed on the label.")
    );
    assert_eq!(
        pattern_needs(9).as_deref(),
        Some("The pattern needs 1 more strike placed on the label.")
    );
    assert_eq!(pattern_needs(10), None);
    assert_eq!(pattern_needs(25), None);
    assert_eq!(
        heat_map_needs(4).as_deref(),
        Some("The heat map needs 6 more strikes placed on the label.")
    );
    assert_eq!(
        heat_map_needs(9).as_deref(),
        Some("The heat map needs 1 more strike placed on the label.")
    );
    assert_eq!(heat_map_needs(10), None);
}

// --- The sweeps -------------------------------------------------------------------------------------

/// Every line, with inputs whose expansions run long, and every radius of every table.
fn every_line() -> Vec<String> {
    let awkward = [
        8.444_444_4,
        -2.155_555_5,
        14.0 / 3.0,
        -0.666_666_7,
        23.999_99,
    ];
    let mut lines = vec![NO_LEVEL_YET.to_string(), NO_LABEL_FITS.to_string()];
    for &h in &awkward {
        for &v in &awkward {
            lines.push(offset_line(h, v));
            lines.push(pattern_centre_line((h, v)));
            lines.push(position_line(&placed(StrikeSource::Read, h, v)));
            lines.push(position_line(&placed(StrikeSource::Picked, h, v)));
        }
        lines.push(spread_line(h.abs()));
        lines.push(r50_line(h.abs()));
    }
    for category in EVERY_CATEGORY {
        for (level, &radius) in ladder_radii(category).iter().enumerate() {
            lines.push(level_line(level, category).unwrap());
            lines.push(inside_verdict(true, radius));
            lines.push(inside_verdict(false, radius));
        }
    }
    for call in [StrikeCall::Heel, StrikeCall::Centre, StrikeCall::Toe] {
        lines.push(call_verdict(call, true));
        lines.push(call_verdict(call, false));
    }
    for stage in [
        DrillStage::Baseline,
        DrillStage::Ladder,
        DrillStage::Recentre,
        DrillStage::Differential,
    ] {
        lines.push(stage_name(stage).to_string());
    }
    lines.extend([
        position_line(&declared()),
        position_line(&not_read(TapeNotRead::NoLabel)),
        baseline_progress(6),
        ladder_progress(6, 4),
        recentre_progress(6),
        differential_progress(6, 1),
    ]);
    lines
}

/// Every sentence, with the same awkward inputs, and every radius of every table as a target.
fn every_sentence() -> Vec<String> {
    let awkward = [
        8.444_444_4,
        -2.155_555_5,
        14.0 / 3.0,
        -0.666_666_7,
        23.999_99,
    ];
    let mut sentences = Vec::new();
    for category in EVERY_CATEGORY {
        let radii = ladder_radii(category);
        for (level, &radius) in radii.iter().enumerate() {
            let wider = radii[level.saturating_sub(1)];
            sentences.push(baseline_tip(Some(awkward[0]), radius, level > 0));
            sentences.push(baseline_tip(None, radius, true));
            sentences.push(ladder_result(4, 10, wider, radius, false, false));
            sentences.push(ladder_result(9, 10, radius, radius, true, false));
            sentences.push(ladder_result(0, 10, radius, radius, false, true));
            sentences.push(recentre_result(None, false, radius));
            sentences.push(return_to_ladder_suggestion(radius));
            for why in [DifferentialWhy::Baseline, DifferentialWhy::AtWidest] {
                sentences.push(differential_suggestion(why, radius));
            }
            for &h in &awkward {
                for &v in &awkward {
                    sentences.push(recentre_result(Some((h, v)), false, radius));
                    sentences.push(recentre_result(Some((h, v)), true, radius));
                }
            }
        }
    }
    for &h in &awkward {
        for &v in &awkward {
            for (heel_toe, high_low) in [(true, false), (false, true), (true, true)] {
                sentences.push(recentre_suggestion((h, v), heel_toe, high_low));
            }
        }
    }
    sentences.extend([
        differential_result(4, 10, false),
        differential_result(8, 10, true),
        fresh_baseline_suggestion(8, 10),
        pattern_needs(3).unwrap(),
        heat_map_needs(9).unwrap(),
    ]);
    sentences.extend(
        EVERY_REASON
            .iter()
            .map(|reason| reason.sentence().to_string()),
    );
    sentences
}

/// The decimal places of every number in `text`: a run of digits and points, read from its last
/// point. A full stop after a whole number ("12 mm." does not reach here, "of 10." does) is a point
/// with no digits after it.
fn decimals(text: &str) -> Vec<(String, usize)> {
    let mut found = Vec::new();
    let mut run = String::new();
    for c in text.chars().chain(std::iter::once(' ')) {
        if c.is_ascii_digit() || (c == '.' && !run.is_empty()) {
            run.push(c);
        } else if !run.is_empty() {
            let places = run.rfind('.').map_or(0, |point| run.len() - point - 1);
            found.push((std::mem::take(&mut run), places));
        }
    }
    found
}

#[test]
fn no_line_or_sentence_carries_a_number_past_a_tenth() {
    for text in every_line().iter().chain(every_sentence().iter()) {
        for (number, places) in decimals(text) {
            assert!(places <= 1, "{number} in {text:?}");
        }
        assert!(!text.contains("nan") && !text.contains("inf"), "{text:?}");
    }
}

#[test]
fn lines_have_no_full_stop_and_sentences_end_with_one() {
    for line in every_line() {
        assert!(!line.ends_with('.'), "{line:?}");
    }
    for sentence in every_sentence() {
        assert!(sentence.ends_with('.'), "{sentence:?}");
    }
}
