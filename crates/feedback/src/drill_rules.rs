//! The impact drill's words: every line and sentence the drill shows, from a reading card's offset
//! to a suggestion and why it fired. [M46 P16]
//!
//! **No Python twin, and none is coming**, for `contracts::drill`'s reason: the drill was designed
//! after the pivot (ADR-034), and frozen Python never gains it (ADR-035 clause 4). Nothing records
//! an oracle for these strings either, so `tests/drill_rules.rs` pins each one for a named input.
//!
//! # Why the words are here
//!
//! The app's Dart holds layout and no golf, so Rust hands every display string (the M46 plan's rule
//! 8, which is M38's). The words sit here, beside the profile's list, rather than in the app's facade
//! (`crates/app`'s `drill`, P18 to P20), so that every sentence the drill can show is in one file
//! and the facade only chooses which to show. They are not in `analysis` because the engine judges
//! and this crate speaks (ADR-008, as a cargo edge: `Cargo.toml` says why, and
//! `tests/profile_rules.rs` pins it).
//!
//! # Every verdict arrives decided
//!
//! This crate cannot see `analysis`, so the facade copies `analysis::drill`'s results across as plain
//! numbers and flags: a block's `inside` and `counted`, the radii before and after, the table-end
//! flags, a recentre block's mean and whether it met its goal, the calls made, a pattern's centre and
//! which axes are off. **Nothing here re-decides any of them.** No function compares an offset with a
//! radius, counts a block or chooses a suggestion; it reads the answer it is given and says it. From
//! `contracts::drill` it reads only the vocabulary, the constants a target is stated in, and the
//! tables a level is named from, which are shapes rather than judgments.
//!
//! **The not-read sentences are not here.** They are `TapeNotRead::sentence()`, beside their reasons
//! and their `retake_helps` flags (P6), so a reason and its words cannot drift apart. A strike that
//! was not read shows as "not read" in [`position_line`], as every absent value in the app does.
//!
//! # Two kinds of number, two formatters
//!
//! - A **measured** distance (an offset, a mean, R50, a spread, the recentre goal worked from a
//!   radius) prints to a tenth of a millimetre through `pyfmt::fixed`, as the plan's "8.4 mm toe"
//!   does. A tenth is the plan's precision, and it is a display precision: P29 is the first to
//!   measure what the reader resolves.
//! - A **table radius** prints through `pyfmt::g`, as the table holds it, so a circle is "12 mm" and
//!   never "12.0 mm". Every radius is a value in `ladder_radii`, set by hand, so `g`'s six
//!   significant digits print it whole.
//!
//! So no sentence carries a raw float, which the tests sweep for.
//!
//! # Lines and sentences
//!
//! A **line** is a fragment a card or a list shows beside other things ("8.4 mm toe, 2.1 mm high",
//! "Inside 12 mm", "Block: 6 of 10, 4 inside") and has no full stop. A **sentence** is a tip, a
//! block's result, a suggestion or a view's absence, and ends with one. The tests hold each function
//! to its kind.
//!
//! # The sign of nothing
//!
//! Within [`DEAD_CENTRE_MM`] of the label's centre on an axis, [`offset_line`] names no direction on
//! that axis, and within it on both the strike is "dead centre". Below that, a tenth rounds toward
//! 0.0, and "0.0 mm heel" would print a side the reading does not support, which is ADR-010 §2's rule
//! applied to a word. One threshold for both cases, so the line never drops an axis it would print a
//! number for on the other.
//!
//! # Words
//!
//! The ladder's idea is the challenge-point framework's, and the pattern and differential ideas are
//! Adam Young's emphasis (the M46 plan's C22, ADR-037 clause 10). No text, drill name or table is
//! copied from his materials, and the words are plain: "Try differential practice: hit the spot I
//! call." Each target is stated in the constants that set it, so a revised [`MOVE_IN_AT`] or
//! [`BASELINE_HOLDS`] changes the sentence with it rather than leaving "half" or "7" behind.

use contracts::drill::{
    ladder_radii, DrillStage, StrikeCall, StrikeSource, TapeStrike, BASELINE_HOLDS, BLOCK_SIZE,
    DIFFERENTIAL_AFTER_BLOCKS, FRESH_BASELINE_AT, MOVE_IN_AT, MOVE_OUT_AT, RECENTRE_FRACTION,
};
use contracts::intent::ClubCategory;
use contracts::strike::RATE_MINIMUM_N;

/// Within this many millimetres of the label's centre on an axis, [`offset_line`] names no
/// direction on it, and within it on both the line is "dead centre" (the module doc). **The M46
/// plan's P16 section's, provisional, and display only**: no judgment reads it. What revises it is
/// P29's measured reading error, since a centre finer than the reader resolves is a claim the line
/// cannot back.
pub const DEAD_CENTRE_MM: f64 = 0.5;

/// The level line for a club whose first baseline has not set a level yet.
pub const NO_LEVEL_YET: &str = "No level yet: the baseline sets it";

/// The line for a club whose category takes no label (the putter, the M46 plan's P3 finding 8),
/// which is listed and cannot start (P19).
pub const NO_LABEL_FITS: &str = "No STRIKEPRO label fits this club";

// "Your last 2 blocks each had" is plural by construction: a row of one is not a row.
const _: () = assert!(DIFFERENTIAL_AFTER_BLOCKS >= 2);

// --- The reading card ------------------------------------------------------------------------------

/// Where a strike landed, in the label's frame (C4: + toward the toe, + toward the top line):
/// "8.4 mm toe, 2.1 mm high". An axis within [`DEAD_CENTRE_MM`] of centre is left out ("8.4 mm toe"),
/// and both within it is "dead centre". The edge is inclusive.
pub fn offset_line(h_mm: f64, v_mm: f64) -> String {
    let sides: Vec<String> = [side(h_mm, "toe", "heel"), side(v_mm, "high", "low")]
        .into_iter()
        .flatten()
        .collect();
    if sides.is_empty() {
        "dead centre".to_string()
    } else {
        sides.join(", ")
    }
}

/// One axis of the offset line, or `None` within [`DEAD_CENTRE_MM`] of centre.
fn side(offset_mm: f64, plus: &str, minus: &str) -> Option<String> {
    if offset_mm.abs() <= DEAD_CENTRE_MM {
        return None;
    }
    let word = if offset_mm > 0.0 { plus } else { minus };
    Some(format!("{} mm {word}", mm(offset_mm.abs())))
}

/// A stored strike's position line: the [`offset_line`] for a read strike, the same marked
/// "(picked)" for one the golfer placed, since a pick is a declaration and is shown as one (ADR-037
/// clause 4), "off the tape" for one declared off it, and "not read" for one that was not read.
pub fn position_line(strike: &TapeStrike) -> String {
    match (strike.source, strike.position()) {
        (Some(StrikeSource::Read), Some((h, v))) => offset_line(h, v),
        (Some(StrikeSource::Picked), Some((h, v))) => format!("{} (picked)", offset_line(h, v)),
        (Some(StrikeSource::DeclaredOffTape), _) => "off the tape".to_string(),
        // A read or picked strike always holds a position (`TapeStrike`'s check), so only a strike
        // that was not read lands here.
        (Some(StrikeSource::Read | StrikeSource::Picked), None) | (None, _) => {
            "not read".to_string()
        }
    }
}

/// A strike judged against its circle (C20): "Inside 12 mm" or "Outside 12 mm". `inside` is
/// `analysis::drill::inside`'s answer, so a strike declared off the tape arrives as outside.
pub fn inside_verdict(inside: bool, radius_mm: f64) -> String {
    let word = if inside { "Inside" } else { "Outside" };
    format!("{word} {}", radius_text(radius_mm))
}

/// The spot a differential strike was called to, as the app shows it.
pub const fn call_name(call: StrikeCall) -> &'static str {
    match call {
        StrikeCall::Heel => "Heel",
        StrikeCall::Centre => "Centre",
        StrikeCall::Toe => "Toe",
    }
}

/// A differential strike judged against its call (C14): "Toe call: made" or "Toe call: missed".
/// `made` is `analysis::drill::call_made`'s answer at the club's current radius.
pub fn call_verdict(call: StrikeCall, made: bool) -> String {
    let word = if made { "made" } else { "missed" };
    format!("{} call: {word}", call_name(call))
}

// --- Where the club is ---------------------------------------------------------------------------------

/// A stage as the app names it.
pub const fn stage_name(stage: DrillStage) -> &'static str {
    match stage {
        DrillStage::Baseline => "Baseline",
        DrillStage::Ladder => "Ladder",
        DrillStage::Recentre => "Recentring",
        DrillStage::Differential => "Differential practice",
    }
}

/// A circle's radius as the table holds it: "12 mm" (the module doc's second formatter).
pub fn radius_text(radius_mm: f64) -> String {
    format!("{} mm", pyfmt::g(radius_mm))
}

/// A club's level line: "Level 2 of 6: 19 mm". Counted from 1 at the widest circle, so climbing the
/// ladder raises the number. `None` for a level past the category's table or a category with no
/// ladder, which a validated `DrillState` holds neither of. A club with no level yet shows
/// [`NO_LEVEL_YET`].
pub fn level_line(level: usize, category: ClubCategory) -> Option<String> {
    let radii = ladder_radii(category);
    let radius = radii.get(level)?;
    Some(format!(
        "Level {} of {}: {}",
        level + 1,
        radii.len(),
        radius_text(*radius)
    ))
}

// --- The block in progress -------------------------------------------------------------------------------

/// The baseline so far: "Baseline: 6 of 10". It aims at centre with no circle, so nothing is inside
/// yet.
pub fn baseline_progress(counted: usize) -> String {
    format!("Baseline: {counted} of {BLOCK_SIZE}")
}

/// A ladder block so far (C20): "Block: 6 of 10, 4 inside".
pub fn ladder_progress(counted: usize, inside: usize) -> String {
    format!("Block: {counted} of {BLOCK_SIZE}, {inside} inside")
}

/// A recentre block so far: "Recentring block: 6 of 10". Its goal is its own centre, which is judged
/// only at its tenth strike, so nothing else is counted on the way.
pub fn recentre_progress(counted: usize) -> String {
    format!("Recentring block: {counted} of {BLOCK_SIZE}")
}

/// A differential block so far: "Differential block: 6 of 10, 4 calls made", and "1 call made".
pub fn differential_progress(counted: usize, made: usize) -> String {
    format!(
        "Differential block: {counted} of {BLOCK_SIZE}, {} made",
        plural(made, "call", "calls")
    )
}

// --- A judged block ---------------------------------------------------------------------------------

/// The target a ladder level sets: "Your target: 7 of 10 inside 12 mm."
fn target(radius_mm: f64) -> String {
    format!(
        "Your target: {MOVE_IN_AT} of {BLOCK_SIZE} inside {}.",
        radius_text(radius_mm)
    )
}

/// The tip a judged baseline gives (C11): how close half its strikes landed, and the start level's
/// target. "Half your strikes landed within 14.0 mm. Your target: 7 of 10 inside 12 mm."
///
/// - `r50_mm` is the baseline's R50, or `None` when its median cannot be placed, which ten counted
///   strikes allow only when one was declared off the tape (the M46 plan's P15 finding 4). It is
///   **not** always `analysis::drill::pattern`'s `r50`, which needs ten placed strikes; the facade
///   works it, and says how.
/// - `start_radius_mm` is the radius of the level the baseline leaves the club at,
///   `level_or_widest(start_level(..))`.
/// - `start_found` is whether `start_level` named a level. When it did not, the club starts at the
///   widest circle (the user's decision, P1 finding 5) and the tip says why.
pub fn baseline_tip(r50_mm: Option<f64>, start_radius_mm: f64, start_found: bool) -> String {
    let half = match r50_mm {
        Some(r50) => format!("Half your strikes landed within {} mm.", mm(r50)),
        None => {
            "Your baseline had a strike off the tape, so how close half your strikes landed is \
                 not known."
                .to_string()
        }
    };
    let start = if start_found {
        String::new()
    } else {
        format!(
            " No circle held {BASELINE_HOLDS} of your {BLOCK_SIZE} strikes, so you start at the \
             widest."
        )
    };
    format!("{half}{start} {}", target(start_radius_mm))
}

/// A judged ladder block (D8): the count against the circle it was hit at, the step, and the next
/// target. "8 of 10 inside 14 mm. Moving in a level. Your target: 7 of 10 inside 12 mm."
///
/// Every input is `analysis::drill::BlockResult`'s, with the radii of its levels before and after.
/// The step is read off the two radii, which is what the judgment left the club at (a tighter circle
/// is a move in), and the two table-end flags say why an earned move stayed.
pub fn ladder_result(
    inside: usize,
    counted: usize,
    from_mm: f64,
    to_mm: f64,
    at_tightest_in: bool,
    at_widest_out: bool,
) -> String {
    let step = if to_mm < from_mm {
        "Moving in a level."
    } else if to_mm > from_mm {
        "Moving out a level."
    } else if at_tightest_in {
        "That is the tightest circle, so the level stays."
    } else if at_widest_out {
        "That is the widest circle, so the level stays."
    } else {
        "The level stays."
    };
    format!(
        "{inside} of {counted} inside {}. {step} {}",
        radius_text(from_mm),
        target(to_mm)
    )
}

/// The recentre goal, worked from the club's radius: a block's centre within [`RECENTRE_FRACTION`]
/// of it on both axes (C13, judged per axis, P15 finding 2).
fn recentre_goal(radius_mm: f64) -> String {
    format!(
        "within {} mm of the label's centre on both axes",
        mm(RECENTRE_FRACTION * radius_mm)
    )
}

/// A judged recentre block (C13): where it centred, and its goal.
/// "Block centre: 1.2 mm toe. Goal met: within 4.0 mm of the label's centre on both axes."
///
/// `mean` and `goal_met` are `analysis::drill::RecentreResult`'s. A `mean` of `None` means the block
/// had a strike declared off the tape, and the sentence says so rather than printing a centre
/// (P15 finding 3); `judge_recentre` never meets the goal without a mean.
pub fn recentre_result(mean: Option<(f64, f64)>, goal_met: bool, radius_mm: f64) -> String {
    let centre = match mean {
        Some((h, v)) => format!("Block centre: {}.", offset_line(h, v)),
        None => "Block centre: not known, because the block had a strike off the tape.".to_string(),
    };
    let goal = if goal_met && mean.is_some() {
        format!("Goal met: {}.", recentre_goal(radius_mm))
    } else {
        format!("The goal is {}.", recentre_goal(radius_mm))
    };
    format!("{centre} {goal}")
}

/// A judged differential block (C13, C14): the calls made, and the target that suggests a fresh
/// baseline. "4 of 10 calls made. Your target: 7 of 10." Every input is
/// `analysis::drill::DifferentialResult`'s.
pub fn differential_result(made: usize, counted: usize, fresh_baseline: bool) -> String {
    let target = if fresh_baseline {
        "Target met"
    } else {
        "Your target"
    };
    format!("{made} of {counted} calls made. {target}: {FRESH_BASELINE_AT} of {BLOCK_SIZE}.")
}

// --- The suggestions --------------------------------------------------------------------------------

/// Which of C13's two triggers suggested differential practice. `contracts::drill::Suggestion` does
/// not say, and the sentence differs, so the facade names it (P15 finding 3) from the stage of the
/// block just judged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DifferentialWhy {
    /// A baseline whose widest circle held fewer than [`BASELINE_HOLDS`] of its strikes.
    Baseline,
    /// [`DIFFERENTIAL_AFTER_BLOCKS`] ladder blocks in a row with [`MOVE_OUT_AT`] or fewer inside the
    /// widest circle.
    AtWidest,
}

/// The recentre suggestion, with why it fired: "Your strikes sit 6.0 mm toward the toe. Move the
/// pattern before shrinking it."
///
/// `centre` is the pattern's (`analysis::drill::Pattern::centre`), and `heel_toe` and `high_low` are
/// `OffCentre`'s axes. **Each off axis is named, and only those**: an axis the trigger did not find
/// clearly off is not part of why it fired, and both off is both named, heel to toe first.
pub fn recentre_suggestion(centre: (f64, f64), heel_toe: bool, high_low: bool) -> String {
    let (h, v) = centre;
    let mut sides = Vec::new();
    if heel_toe {
        let word = if h > 0.0 { "toe" } else { "heel" };
        sides.push(format!("{} mm toward the {word}", mm(h.abs())));
    }
    if high_low {
        let word = if v > 0.0 { "high" } else { "low" };
        sides.push(format!("{} mm {word}", mm(v.abs())));
    }
    let why = if sides.is_empty() {
        // Never sent by a facade that read `OffCentre::any()`, and still true if it were.
        "Your strikes sit off centre.".to_string()
    } else {
        format!("Your strikes sit {}.", sides.join(" and "))
    };
    format!("{why} Move the pattern before shrinking it.")
}

/// The differential suggestion, with which trigger fired and the widest circle it was judged at.
/// "Your last 2 blocks each had 3 or fewer of 10 inside the widest circle, 22 mm. Try differential
/// practice: hit the spot I call."
pub fn differential_suggestion(why: DifferentialWhy, widest_mm: f64) -> String {
    let because = match why {
        DifferentialWhy::Baseline => format!(
            "Fewer than {BASELINE_HOLDS} of your {BLOCK_SIZE} baseline strikes landed inside the \
             widest circle, {}.",
            radius_text(widest_mm)
        ),
        DifferentialWhy::AtWidest => format!(
            "Your last {DIFFERENTIAL_AFTER_BLOCKS} blocks each had {MOVE_OUT_AT} or fewer of \
             {BLOCK_SIZE} inside the widest circle, {}.",
            radius_text(widest_mm)
        ),
    };
    format!("{because} Try differential practice: hit the spot I call.")
}

/// The suggestion a recentre block that met its goal gives: back to the ladder at the same level.
/// "This block's centre was within 4.0 mm of the label's centre on both axes. Go back to the
/// ladder. Your target: 7 of 10 inside 12 mm."
pub fn return_to_ladder_suggestion(radius_mm: f64) -> String {
    format!(
        "This block's centre was {}. Go back to the ladder. {}",
        recentre_goal(radius_mm),
        target(radius_mm)
    )
}

/// The suggestion a differential block that made enough calls gives: a fresh baseline, and the
/// ladder from it. `made` and `counted` are `DifferentialResult`'s.
pub fn fresh_baseline_suggestion(made: usize, counted: usize) -> String {
    format!(
        "You made {made} of {counted} calls. Take a fresh baseline: {BLOCK_SIZE} strikes aimed at \
         centre, and the ladder starts again from it."
    )
}

// --- The progress view ---------------------------------------------------------------------------------

/// Where a pattern sits: "Pattern centre: 3.2 mm toe, 1.0 mm low". `centre` is
/// `analysis::drill::Pattern::centre`.
pub fn pattern_centre_line(centre: (f64, f64)) -> String {
    format!("Pattern centre: {}", offset_line(centre.0, centre.1))
}

/// How big a pattern is, wherever it sits: "Spread: half within 6.5 mm of the pattern's centre".
/// `spread_mm` is `Pattern::spread`.
pub fn spread_line(spread_mm: f64) -> String {
    format!(
        "Spread: half within {} mm of the pattern's centre",
        mm(spread_mm)
    )
}

/// How close to the label's centre half the strikes land, R50 in golfer's words: "Half your strikes
/// within 9.4 mm of the label's centre".
pub fn r50_line(r50_mm: f64) -> String {
    format!(
        "Half your strikes within {} mm of the label's centre",
        mm(r50_mm)
    )
}

/// What the pattern still needs, or `None` once it has [`RATE_MINIMUM_N`] placed strikes and is
/// shown. `placed` is how many strikes `analysis::drill::pattern` would take: read or picked, in the
/// baseline, ladder and recentre stages. "The pattern needs 6 more strikes placed on the label."
pub fn pattern_needs(placed: usize) -> Option<String> {
    needs("The pattern", placed)
}

/// What the heat map still needs, or `None` once it is shown (C21: at [`RATE_MINIMUM_N`] placed
/// strikes, as the pattern is). "The heat map needs 1 more strike placed on the label."
pub fn heat_map_needs(placed: usize) -> Option<String> {
    needs("The heat map", placed)
}

fn needs(what: &str, placed: usize) -> Option<String> {
    // A constant of 10, so the conversion cannot fail; `strike` keeps it `i64` because it
    // compares with a count of rows there.
    let minimum = usize::try_from(RATE_MINIMUM_N).unwrap_or(usize::MAX);
    let more = minimum.checked_sub(placed).filter(|more| *more > 0)?;
    Some(format!(
        "{what} needs {} placed on the label.",
        plural(more, "more strike", "more strikes")
    ))
}

// --- Helpers ---------------------------------------------------------------------------------------

/// A measured distance to a tenth of a millimetre, the module doc's first formatter.
fn mm(distance_mm: f64) -> String {
    pyfmt::fixed(distance_mm, 1)
}

/// `n` with its noun, singular at exactly one.
fn plural(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}
