//! The impact drill judged: which strikes count, which landed inside a circle, the level a baseline
//! starts the ladder at, and the step a block of ten earns (M46 P14). Then where a club's strikes
//! land and how widely (the pattern), whether a recentre block met its goal, whether a differential
//! strike made its call, and what the drill suggests after each judged block (P15).
//!
//! **No Python twin, and none is coming**, for `contracts::drill`'s reason: the drill was designed
//! after the pivot (ADR-034), and frozen Python never gains it (ADR-035 clause 4). Nothing records an
//! oracle for it either, so the tests hold each rule to the M46 plan's calls (C10 to C14) and
//! ADR-037 rather than to recorded answers.
//!
//! # Judged here, stored nowhere
//!
//! A strike stores where it landed and the radius it was judged against, never whether it landed
//! inside (C8), because a stored verdict could disagree with its own numbers. So [`inside`] is
//! derived each time it is asked, as `TapeStrike::counts` is, and a block's inside count and step are
//! a [`BlockResult`] its caller keeps only as levels (`LevelEntry`'s `from` and `to`). A block's
//! inside count can always be worked again from its strikes.
//!
//! # `None` is "cannot judge", and a declaration is a miss
//!
//! [`inside`] answers `None` for a strike that was not read: it has no position and does not count
//! (C10), so it is neither in nor out. A strike **declared off the tape** has no position either, but
//! it answers `Some(false)` at every radius, because the golfer said the ball missed the label and
//! every table is trimmed at its label's inscribed radius (`contracts::drill`'s module doc), so no
//! circle reaches past the tape (C6). The same split as `crate::strike`'s: a `false` is a judged miss,
//! and the two are different values so that nothing can count one as the other.
//!
//! # Every edge is inclusive
//!
//! A strike exactly on the circle is inside, as `CENTERED_RADIUS_MM`'s rule is (C10). A block with
//! exactly [`MOVE_IN_AT`] inside moves in, one with exactly [`MOVE_OUT_AT`] moves out, and a circle
//! holding exactly [`BASELINE_HOLDS`] of the baseline can be its start level. `f64::hypot` rather than
//! `pyfmt::hypot`, for `crate::strike::is_centered`'s reason: there is no recorded answer to match.
//!
//! # A block is exactly ten counted strikes, or it is refused
//!
//! The steps are counts, not shares, so nine strikes judged by them would read three misses as a
//! failed block that one more strike could have saved. [`judge_block`] and [`start_level`] refuse
//! anything but [`BLOCK_SIZE`] counted strikes, and a strike that was not read may sit among them
//! without making up the tenth. They refuse a level past its table and a category with no ladder too.
//! A validated `DrillState` holds neither, so each refusal is a caller's bug, named as one rather
//! than an index past the end of a table.
//!
//! # The table's two ends, and what C11 leaves to its caller
//!
//! At the tightest level a block that earns a move in stays, and at the widest one that earns a move
//! out stays. [`BlockResult`] says which, because the second is what C13's differential trigger counts
//! (two in a row, P15), and a `LevelEntry` with `from == to` cannot tell a held block from a middling
//! one.
//!
//! [`start_level`] answers `None` when even the widest circle holds fewer than [`BASELINE_HOLDS`].
//! C11 names no level then, only C13's trigger, and **the user named it** (2026-11-13, the M46
//! plan's P1 finding 5): such a club starts at the widest circle, which is C14's `R` until the
//! ladder moves it. [`level_or_widest`] is that rule, spelled once, and `start_level` still answers
//! `None` so that the trigger stays visible to [`suggestion`].
//!
//! # The pattern: where the strikes sit, apart from how widely
//!
//! [`pattern`] takes a club's last [`PATTERN_WINDOW`] positioned strikes in the baseline, ladder and
//! recentre stages (C12). A differential strike was aimed off centre on purpose, so it never enters,
//! and neither does a strike with no position: one declared off the tape landed somewhere unknown.
//! Below [`RATE_MINIMUM_N`] the pattern is absent rather than estimated (ADR-010 §2), the floor
//! `analysis::strike`'s rates keep. The centre and its two intervals are `stats`' (`mean_and_sd`,
//! `mean_ci`), and both medians are `stats::percentile`, so a centre here is computed the way every
//! other mean in the crate is.
//!
//! # The suggestions are worked again from the record each time
//!
//! [`suggestion`] reads the club's history and its stored strikes, never its pending suggestion, so a
//! declined one is offered again after every block for which it still holds and "not now" is never
//! remembered (C13). A `LevelEntry` stores only levels and strike ids (P6), and one at the widest with
//! `from == to` cannot tell three inside from six, so each block a trigger reads is judged again from
//! its strikes by the same function that judged it first.
//!
//! **Recentre outranks differential practice** when both hold after a ladder block or a baseline.
//! Differential practice fires on the circle's own count, and ADR-037 clause 8 says why that count
//! cannot tell a tight pattern sitting off centre from a centred one that is too wide; the recentre
//! trigger can, so when it fires the count's failure is at least partly the offset, and the pattern
//! is moved before its size is worked on.

use std::collections::HashMap;

use contracts::club::{category_of, ClubId};
use contracts::drill::{
    ladder_radii, ClubDrillState, DrillStage, LevelEntry, StrikeCall, StrikeSource, Suggestion,
    TapeStrike, BASELINE_HOLDS, BLOCK_SIZE, DIFFERENTIAL_AFTER_BLOCKS, FRESH_BASELINE_AT,
    MOVE_IN_AT, MOVE_OUT_AT, PATTERN_WINDOW, RECENTRE_FRACTION,
};
use contracts::intent::ClubCategory;
use contracts::strike::RATE_MINIMUM_N;
use contracts::ContractError;

use crate::stats;

// A block can earn only one step while the two thresholds cannot both hold, and a baseline can name
// a start level only while a circle can hold enough. A revision of the constants that broke either
// would let `judge_block`'s first arm win silently, or leave every baseline without a level, so it
// fails to compile instead.
const _: () = assert!(MOVE_OUT_AT < MOVE_IN_AT && MOVE_IN_AT <= BLOCK_SIZE);
const _: () = assert!(BASELINE_HOLDS <= BLOCK_SIZE);
// The same for P15's: a differential block that could never make enough calls would never suggest a
// fresh baseline, a window narrower than the floor would never hold a pattern, and a trigger over
// no blocks would fire on an empty history.
const _: () = assert!(FRESH_BASELINE_AT <= BLOCK_SIZE);
const _: () = assert!(RATE_MINIMUM_N > 0 && PATTERN_WINDOW >= RATE_MINIMUM_N as usize);
const _: () = assert!(DIFFERENTIAL_AFTER_BLOCKS > 0);

/// [`RATE_MINIMUM_N`] as a count, which the assertion above shows is positive.
const PATTERN_MINIMUM: usize = RATE_MINIMUM_N as usize;

/// The widest level, index 0 of every table.
const WIDEST: usize = 0;

/// The strikes that count toward a block or a baseline, in the order given (C10): read, picked or
/// declared off the tape. A strike that was not read is stored and never counted.
pub fn counted(strikes: &[TapeStrike]) -> Vec<&TapeStrike> {
    strikes.iter().filter(|strike| strike.counts()).collect()
}

/// Did the strike land inside a circle of `radius_mm` about the label's centre (C10)? Inside is
/// `hypot(h, v) ≤ radius`, **inclusive**, as `CENTERED_RADIUS_MM`'s rule is.
///
/// `Some(false)` at every radius for a strike declared off the tape (C6), and `None` for a strike
/// that was not read, which has no position and does not count (the module doc). A `match` on the
/// source, so that a source added is a compile error here until someone decides where it lands.
pub fn inside(strike: &TapeStrike, radius_mm: f64) -> Option<bool> {
    match strike.source {
        Some(StrikeSource::Read | StrikeSource::Picked) => {
            strike.position().map(|(h, v)| h.hypot(v) <= radius_mm)
        }
        Some(StrikeSource::DeclaredOffTape) => Some(false),
        None => None,
    }
}

/// The level a club's baseline starts its ladder at (C11): the **tightest** circle in `category`'s
/// table holding at least [`BASELINE_HOLDS`] of the baseline's [`BLOCK_SIZE`] counted strikes, as an
/// index into [`ladder_radii`] (0 is the widest).
///
/// `Ok(None)` when even the widest circle holds fewer, which is C13's differential trigger, and names
/// no level (the module doc). Refuses a baseline of anything but [`BLOCK_SIZE`] counted strikes and a
/// category with no ladder.
pub fn start_level(
    baseline: &[TapeStrike],
    category: ClubCategory,
) -> Result<Option<usize>, ContractError> {
    let radii = table("start_level", category)?;
    let strikes = full_block("start_level.baseline", baseline)?;
    // Widest first and strictly narrowing, so the last circle holding enough is the tightest one.
    Ok(radii
        .iter()
        .rposition(|&radius| inside_count(&strikes, radius) >= BASELINE_HOLDS))
}

/// A club's level, or the **widest** when it has none: the user's rule for a baseline whose widest
/// circle holds fewer than [`BASELINE_HOLDS`] (2026-11-13, the M46 plan's P1 finding 5). The club
/// starts there, and that radius is C14's `R` until the ladder moves it, so differential practice can
/// be accepted and judged after such a baseline. The two alternatives turned down were re-running the
/// baseline until the circle holds enough, and the widest marked provisional.
///
/// So `level_or_widest(start_level(baseline, category)?)` is the level a judged baseline leaves its
/// club at, and [`suggestion`] reads a stored level, or a history entry's `from`, through this too.
pub fn level_or_widest(level: Option<usize>) -> usize {
    level.unwrap_or(WIDEST)
}

/// Which way a judged block moves the club's circle (D8). `In` is a tighter circle, a level further
/// into the table, and `Out` a wider one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LadderStep {
    In,
    Out,
    Stay,
}

/// One ladder block judged against its level's circle (C10).
///
/// **Not stored.** Its caller writes the level change into the club's history (`LevelEntry`), and
/// everything else here can be worked again from the block's strikes (the module doc).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockResult {
    /// The block's counted strikes inside the circle.
    pub inside: usize,
    /// The block's counted strikes, which is [`BLOCK_SIZE`]: a block of any other count is refused.
    pub counted: usize,
    /// The step taken, which is `Stay` at a table's end even where the count earned a move.
    pub step: LadderStep,
    /// The club's level after the block, an index into [`ladder_radii`].
    pub new_level: usize,
    /// The block earned a move in, at the tightest level, so it stayed.
    pub at_tightest_in: bool,
    /// The block earned a move out (at most [`MOVE_OUT_AT`] inside), at the widest level, so it
    /// stayed. Two of these in a row is C13's differential trigger (P15).
    pub at_widest_out: bool,
}

/// Judge one ladder block of [`BLOCK_SIZE`] counted strikes against the circle of `level` in
/// `category`'s table (C10, D8): at least [`MOVE_IN_AT`] inside moves in a level, at most
/// [`MOVE_OUT_AT`] moves out, and anything between stays. At either end of the table a move off it
/// stays and says so ([`BlockResult`]).
///
/// Refuses a block of anything but [`BLOCK_SIZE`] counted strikes, a level past the table and a
/// category with no ladder (the module doc).
pub fn judge_block(
    block: &[TapeStrike],
    level: usize,
    category: ClubCategory,
) -> Result<BlockResult, ContractError> {
    let (radii, radius) = level_radius("judge_block", level, category)?;
    let strikes = full_block("judge_block.block", block)?;
    let inside = inside_count(&strikes, radius);
    let tightest = radii.len() - 1;
    let (step, new_level, at_tightest_in, at_widest_out) = if inside >= MOVE_IN_AT {
        if level < tightest {
            (LadderStep::In, level + 1, false, false)
        } else {
            (LadderStep::Stay, level, true, false)
        }
    } else if inside <= MOVE_OUT_AT {
        if level > 0 {
            (LadderStep::Out, level - 1, false, false)
        } else {
            (LadderStep::Stay, level, false, true)
        }
    } else {
        (LadderStep::Stay, level, false, false)
    };
    Ok(BlockResult {
        inside,
        counted: strikes.len(),
        step,
        new_level,
        at_tightest_in,
        at_widest_out,
    })
}

// --- The pattern (C12) ------------------------------------------------------------------------------

/// Where a club's strikes land and how widely, over C12's window (the module doc). Millimetres in
/// the label's frame, + toward the toe and + toward the top line (C4).
///
/// **Not stored**, as [`BlockResult`] is not: it is worked again from the strikes whenever it is
/// asked for.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pattern {
    /// The mean position, `(h, v)`.
    pub centre: (f64, f64),
    /// Each mean's 95% interval, `(low, high)`: heel to toe first, then high to low
    /// ([`stats::mean_ci`]).
    pub centre_ci: ((f64, f64), (f64, f64)),
    /// R50: the median distance from face centre, the label's printed centre (D2).
    pub r50: f64,
    /// The median distance from the pattern's own centre: how big the pattern is, wherever it sits.
    pub spread: f64,
    /// The positioned strikes it was taken over, from [`RATE_MINIMUM_N`] to [`PATTERN_WINDOW`].
    pub n: usize,
}

/// The pattern of `strikes`, oldest first (C12): their last [`PATTERN_WINDOW`] positioned strikes in
/// the baseline, ladder and recentre stages. `None` below [`RATE_MINIMUM_N`] of them, never an
/// estimate (the module doc).
///
/// Takes whatever strikes it is given, so a club's progress passes the club's and its category's
/// roll-up passes its clubs' (D10). [`suggestion`] passes one club's, and filters them itself.
pub fn pattern<'a>(strikes: impl IntoIterator<Item = &'a TapeStrike>) -> Option<Pattern> {
    let positions: Vec<(f64, f64)> = strikes
        .into_iter()
        .filter(|strike| in_pattern(strike.stage))
        .filter_map(TapeStrike::position)
        .collect();
    let window = &positions[positions.len().saturating_sub(PATTERN_WINDOW)..];
    if window.len() < PATTERN_MINIMUM {
        return None;
    }
    let h: Vec<f64> = window.iter().map(|&(h, _)| h).collect();
    let v: Vec<f64> = window.iter().map(|&(_, v)| v).collect();
    let (mean_h, _) = stats::mean_and_sd(&h)?;
    let (mean_v, _) = stats::mean_and_sd(&v)?;
    let from_face: Vec<f64> = window.iter().map(|&(h, v)| h.hypot(v)).collect();
    let from_centre: Vec<f64> = window
        .iter()
        .map(|&(h, v)| (h - mean_h).hypot(v - mean_v))
        .collect();
    Some(Pattern {
        centre: (mean_h, mean_v),
        centre_ci: (stats::mean_ci(&h)?, stats::mean_ci(&v)?),
        r50: stats::percentile(&from_face, 0.5)?,
        spread: stats::percentile(&from_centre, 0.5)?,
        n: window.len(),
    })
}

/// Which stages a pattern is taken over (C12). A `match` with no wildcard, so a stage added is a
/// compile error here until someone decides whether its strikes were aimed at centre.
fn in_pattern(stage: DrillStage) -> bool {
    match stage {
        DrillStage::Baseline | DrillStage::Ladder | DrillStage::Recentre => true,
        DrillStage::Differential => false,
    }
}

/// The axes on which a pattern's centre is clearly off (C13): **on that one axis**, the 95% interval
/// excludes 0 **and** the mean is more than [`RECENTRE_FRACTION`] of the radius from 0. Both on the
/// same axis, as ADR-037 clause 8 reads it (the M46 plan's P1 finding 6), so a sure but small lean on
/// one axis and a large but unsure one on the other do not add up to a suggestion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OffCentre {
    /// Toward the heel or the toe.
    pub heel_toe: bool,
    /// Toward the top line or the leading edge.
    pub high_low: bool,
}

impl OffCentre {
    /// Off on either axis, which is the recentre trigger.
    pub fn any(self) -> bool {
        self.heel_toe || self.high_low
    }
}

/// Whether `pattern`'s centre is clearly off, against the circle of `radius_mm` ([`OffCentre`]).
/// "More than" is strict, so a mean exactly a third of the radius out is not off; an interval
/// touching 0 does not exclude it.
pub fn off_centre(pattern: &Pattern, radius_mm: f64) -> OffCentre {
    let limit = RECENTRE_FRACTION * radius_mm;
    let off = |mean: f64, (low, high): (f64, f64)| (low > 0.0 || high < 0.0) && mean.abs() > limit;
    OffCentre {
        heel_toe: off(pattern.centre.0, pattern.centre_ci.0),
        high_low: off(pattern.centre.1, pattern.centre_ci.1),
    }
}

// --- A recentre block (C13) ---------------------------------------------------------------------------

/// One recentre block judged by its own mean (C13). **Not stored**, as [`BlockResult`] is not. The
/// level does not move in the recentre stage: meeting the goal only suggests the ladder again, at the
/// same level.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RecentreResult {
    /// The block's counted strikes, which is [`BLOCK_SIZE`].
    pub counted: usize,
    /// The block's mean position `(h, v)`, or `None` when one of its counted strikes was declared off
    /// the tape and so has no position.
    pub mean: Option<(f64, f64)>,
    /// The mean is within [`RECENTRE_FRACTION`] of the radius on both axes. False with no mean.
    pub goal_met: bool,
}

/// Judge one recentre block of [`BLOCK_SIZE`] counted strikes against the circle of `level` in
/// `category`'s table (C13): its goal is its own mean within [`RECENTRE_FRACTION`] of the radius.
///
/// **Within is per axis and inclusive**: each axis's mean at most a third of the radius from 0. That
/// is the recentre trigger's own test turned round (ADR-037 clause 8 reads it per axis), so a block
/// that meets the goal cannot, by its mean, be off centre on either axis; a test on the mean's
/// distance would leave means that neither meet the goal nor trigger.
///
/// **A strike declared off the tape leaves the mean unknown**, so the goal is not met. It counts
/// toward the block (C10) and landed somewhere off the label, on a side nobody saw, and a mean taken
/// over the other nine would be a guess at the tenth (ADR-010 §2). The pattern leaves such a strike
/// out instead, because C12 defines it over positioned strikes, and a window is a description where
/// this is a verdict.
///
/// Refuses as [`judge_block`] does: a block of anything but [`BLOCK_SIZE`] counted strikes, a level
/// past the table and a category with no ladder.
pub fn judge_recentre(
    block: &[TapeStrike],
    level: usize,
    category: ClubCategory,
) -> Result<RecentreResult, ContractError> {
    let (_, radius) = level_radius("judge_recentre", level, category)?;
    let strikes = full_block("judge_recentre.block", block)?;
    let positions: Option<Vec<(f64, f64)>> =
        strikes.iter().map(|strike| strike.position()).collect();
    let mean = positions.and_then(|positions| {
        let h: Vec<f64> = positions.iter().map(|&(h, _)| h).collect();
        let v: Vec<f64> = positions.iter().map(|&(_, v)| v).collect();
        Some((stats::mean_and_sd(&h)?.0, stats::mean_and_sd(&v)?.0))
    });
    let limit = RECENTRE_FRACTION * radius;
    Ok(RecentreResult {
        counted: strikes.len(),
        mean,
        goal_met: mean.is_some_and(|(h, v)| h.abs() <= limit && v.abs() <= limit),
    })
}

// --- Differential judging (C14) ---------------------------------------------------------------------

/// Did the strike make `call`, against the club's current radius `radius_mm` (C14)? A **centre**
/// call is made when `hypot(h, v) ≤ R`, a **heel** call when `h ≤ −R`, and a **toe** call when
/// `h ≥ +R`. Every edge inclusive, as [`inside`]'s is, and the heel and toe calls read `h` alone,
/// since high and low are not called in M46.
///
/// `Some(false)` for a strike declared off the tape, which makes no call, and `None` for one that
/// was not read, as [`inside`] answers. A `match` on the source and on the call, so a source or a call
/// added is a compile error here until someone decides how it is judged.
pub fn call_made(strike: &TapeStrike, call: StrikeCall, radius_mm: f64) -> Option<bool> {
    match strike.source {
        Some(StrikeSource::Read | StrikeSource::Picked) => {
            strike.position().map(|(h, v)| match call {
                StrikeCall::Centre => h.hypot(v) <= radius_mm,
                StrikeCall::Heel => h <= -radius_mm,
                StrikeCall::Toe => h >= radius_mm,
            })
        }
        Some(StrikeSource::DeclaredOffTape) => Some(false),
        None => None,
    }
}

/// One differential block judged by its calls (C13, C14). **Not stored**, as [`BlockResult`] is
/// not. The level does not move in the differential stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DifferentialResult {
    /// The block's counted strikes that made their call.
    pub made: usize,
    /// The block's counted strikes, which is [`BLOCK_SIZE`].
    pub counted: usize,
    /// At least [`FRESH_BASELINE_AT`] calls made, which suggests a fresh baseline (C13).
    pub fresh_baseline: bool,
}

/// Judge one differential block of [`BLOCK_SIZE`] counted strikes, each against its own stored
/// call, at the circle of `level` in `category`'s table (C14). The radius comes from the table as
/// [`judge_block`]'s does; the call can only come from the strike, which stores what was called.
///
/// Refuses as [`judge_block`] does, and also a counted strike that names no call, which no
/// differential strike that validates can be.
pub fn judge_differential(
    block: &[TapeStrike],
    level: usize,
    category: ClubCategory,
) -> Result<DifferentialResult, ContractError> {
    let (_, radius) = level_radius("judge_differential", level, category)?;
    let strikes = full_block("judge_differential.block", block)?;
    let mut made = 0;
    for strike in &strikes {
        let Some(call) = strike.call else {
            return Err(refuse(
                "judge_differential.block",
                format!(
                    "strike {} names no call, so it is not a differential strike",
                    strike.strike_id
                ),
            ));
        };
        if call_made(strike, call, radius) == Some(true) {
            made += 1;
        }
    }
    Ok(DifferentialResult {
        made,
        counted: strikes.len(),
        fresh_baseline: made >= FRESH_BASELINE_AT,
    })
}

// --- The suggestions (C13) --------------------------------------------------------------------------

/// What the drill suggests `club` do next, after the last block in `state`'s history was judged
/// (C13, D11), or `None`. By the stage that block was played in, read from its strikes:
///
/// - **a baseline or a ladder block**: [`Suggestion::Recentre`] when the club's [`pattern`] is
///   [`off_centre`] against its current circle, else [`Suggestion::Differential`] after a baseline
///   that named no start level, or after [`DIFFERENTIAL_AFTER_BLOCKS`] ladder blocks in a row that
///   each earned a move out at the widest level ([`BlockResult::at_widest_out`]). Recentre outranks
///   differential practice (the module doc);
/// - **a recentre block**: [`Suggestion::ReturnToLadder`] when it met its goal ([`judge_recentre`]);
/// - **a differential block**: [`Suggestion::FreshBaseline`] when it made enough of its calls
///   ([`judge_differential`]).
///
/// `strikes` is the drill's stored strikes, any clubs', oldest first; only `club`'s are read. The
/// club's current circle is `state.level`'s, through [`level_or_widest`]. **`state.suggestion` is
/// never read**, so a declined suggestion is offered again after every block for which it holds
/// (the module doc). "In a row" is the history's last entries: a recentre, differential or baseline
/// entry between two ladder blocks breaks it, and an abandoned block, which leaves no entry, does not.
///
/// Refuses a history entry naming a strike that is not among the club's, an entry of no strikes or
/// of more than one stage, and whatever the judgment it re-runs refuses.
pub fn suggestion(
    club: ClubId,
    state: &ClubDrillState,
    strikes: &[TapeStrike],
) -> Result<Option<Suggestion>, ContractError> {
    let Some(last) = state.history.last() else {
        return Ok(None);
    };
    let category = category_of(club);
    let own: Vec<&TapeStrike> = strikes
        .iter()
        .filter(|strike| strike.club == club)
        .collect();
    let by_id: HashMap<&str, &TapeStrike> = own
        .iter()
        .map(|strike| (strike.strike_id.as_str(), *strike))
        .collect();
    let (stage, block) = entry_block(&by_id, last)?;
    let off_centre_now = || -> Result<bool, ContractError> {
        let (_, radius) = level_radius("suggestion", level_or_widest(state.level), category)?;
        Ok(pattern(own.iter().copied()).is_some_and(|pattern| off_centre(&pattern, radius).any()))
    };
    // The differential trigger is worked first in both arms, so that a malformed history is refused
    // whether or not the recentre trigger would have answered before reaching it.
    let answer = match stage {
        DrillStage::Baseline => {
            let differential = start_level(&block, category)?.is_none();
            first_of(off_centre_now()?, differential)
        }
        DrillStage::Ladder => {
            let differential = widest_out_in_a_row(&by_id, &state.history, category)?;
            first_of(off_centre_now()?, differential)
        }
        DrillStage::Recentre => judge_recentre(&block, level_or_widest(last.from), category)?
            .goal_met
            .then_some(Suggestion::ReturnToLadder),
        DrillStage::Differential => {
            judge_differential(&block, level_or_widest(last.from), category)?
                .fresh_baseline
                .then_some(Suggestion::FreshBaseline)
        }
    };
    Ok(answer)
}

/// After a baseline or a ladder block: recentre if it holds, else differential practice if it holds.
fn first_of(recentre: bool, differential: bool) -> Option<Suggestion> {
    if recentre {
        Some(Suggestion::Recentre)
    } else if differential {
        Some(Suggestion::Differential)
    } else {
        None
    }
}

/// The last [`DIFFERENTIAL_AFTER_BLOCKS`] history entries are all ladder blocks that earned a move
/// out at the widest level, each judged again from its strikes at the level it was played at.
fn widest_out_in_a_row(
    by_id: &HashMap<&str, &TapeStrike>,
    history: &[LevelEntry],
    category: ClubCategory,
) -> Result<bool, ContractError> {
    if history.len() < DIFFERENTIAL_AFTER_BLOCKS {
        return Ok(false);
    }
    for entry in &history[history.len() - DIFFERENTIAL_AFTER_BLOCKS..] {
        let (stage, block) = entry_block(by_id, entry)?;
        if stage != DrillStage::Ladder
            || !judge_block(&block, level_or_widest(entry.from), category)?.at_widest_out
        {
            return Ok(false);
        }
    }
    Ok(true)
}

/// A history entry's strikes, in its order, and the one stage they were played in.
fn entry_block(
    by_id: &HashMap<&str, &TapeStrike>,
    entry: &LevelEntry,
) -> Result<(DrillStage, Vec<TapeStrike>), ContractError> {
    let block = entry
        .block
        .iter()
        .map(|id| {
            by_id
                .get(id.as_str())
                .map(|strike| (*strike).clone())
                .ok_or_else(|| {
                    refuse(
                        "suggestion.history",
                        format!("strike {id} is not among the club's strikes"),
                    )
                })
        })
        .collect::<Result<Vec<TapeStrike>, ContractError>>()?;
    let Some(stage) = block.first().map(|strike| strike.stage) else {
        return Err(refuse("suggestion.history", "an entry names no strikes"));
    };
    if let Some(other) = block.iter().find(|strike| strike.stage != stage) {
        return Err(refuse(
            "suggestion.history",
            format!(
                "strike {} was played in another stage than its block's first, and a stage change \
                 abandons a block",
                other.strike_id
            ),
        ));
    }
    Ok((stage, block))
}

// --- Shared -------------------------------------------------------------------------------------------

/// The category's table, or a refusal for a category with no ladder (the putter, and `all`).
fn table(owner: &str, category: ClubCategory) -> Result<&'static [f64], ContractError> {
    let radii = ladder_radii(category);
    if radii.is_empty() {
        return Err(refuse(
            &format!("{owner}.category"),
            format!("a {} has no ladder", category.as_str()),
        ));
    }
    Ok(radii)
}

/// The category's table and the radius of `level` in it, or a refusal for a category with no
/// ladder or a level past the table.
fn level_radius(
    owner: &str,
    level: usize,
    category: ClubCategory,
) -> Result<(&'static [f64], f64), ContractError> {
    let radii = table(owner, category)?;
    let Some(&radius) = radii.get(level) else {
        return Err(refuse(
            &format!("{owner}.level"),
            format!(
                "level {level} is past the {} table's {} levels",
                category.as_str(),
                radii.len()
            ),
        ));
    };
    Ok((radii, radius))
}

/// The counted strikes, refused unless there are exactly [`BLOCK_SIZE`] of them (the module doc).
fn full_block<'a>(
    field: &str,
    strikes: &'a [TapeStrike],
) -> Result<Vec<&'a TapeStrike>, ContractError> {
    let counted = counted(strikes);
    if counted.len() != BLOCK_SIZE {
        return Err(refuse(
            field,
            format!(
                "a block is judged at exactly {BLOCK_SIZE} counted strikes (got {})",
                counted.len()
            ),
        ));
    }
    Ok(counted)
}

/// How many of `strikes` landed inside `radius_mm`. Every one is counted, so none answers `None`.
fn inside_count(strikes: &[&TapeStrike], radius_mm: f64) -> usize {
    strikes
        .iter()
        .filter(|strike| inside(strike, radius_mm) == Some(true))
        .count()
}

fn refuse(field: &str, problem: impl Into<String>) -> ContractError {
    ContractError {
        field: field.to_string(),
        problem: problem.into(),
    }
}
