//! The impact drill judged: which strikes count, which landed inside a circle, the level a baseline
//! starts the ladder at, and the step a block of ten earns (M46 P14). P15 adds the pattern, the
//! suggestions and a differential call's verdict.
//!
//! **No Python twin, and none is coming**, for `contracts::drill`'s reason: the drill was designed
//! after the pivot (ADR-034), and frozen Python never gains it (ADR-035 clause 4). Nothing records an
//! oracle for it either, so the tests hold each rule to the M46 plan's calls (C10, C11) and ADR-037
//! rather than to recorded answers.
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
//! C11 names no level then, only C13's trigger. Which radius the club is judged against after such a
//! baseline is not this module's to choose: the M46 plan's P1 finding 5 leaves it to the user.

use contracts::drill::{
    ladder_radii, StrikeSource, TapeStrike, BASELINE_HOLDS, BLOCK_SIZE, MOVE_IN_AT, MOVE_OUT_AT,
};
use contracts::intent::ClubCategory;
use contracts::ContractError;

// A block can earn only one step while the two thresholds cannot both hold, and a baseline can name
// a start level only while a circle can hold enough. A revision of the constants that broke either
// would let `judge_block`'s first arm win silently, or leave every baseline without a level, so it
// fails to compile instead.
const _: () = assert!(MOVE_OUT_AT < MOVE_IN_AT && MOVE_IN_AT <= BLOCK_SIZE);
const _: () = assert!(BASELINE_HOLDS <= BLOCK_SIZE);

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
    let radii = table("judge_block", category)?;
    let Some(&radius) = radii.get(level) else {
        return Err(refuse(
            "judge_block.level",
            format!(
                "level {level} is past the {} table's {} levels",
                category.as_str(),
                radii.len()
            ),
        ));
    };
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
