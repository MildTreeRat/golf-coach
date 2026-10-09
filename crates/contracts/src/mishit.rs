//! When a shot's carry is a gross mishit, and the golfer's own verdict on it.
//! `contracts/mishit.py`. [M36 P5]
//!
//! Career mode pools every tagged shot's carry into one guarded mean (ADR-024). A topped 7 iron
//! that carries 20 yards is one of those samples, and it drags "your 7 iron carries 150" down by a
//! shot the golfer already knows was a mistake. This is the rule that spots one and the verdict
//! that overrules the rule either way. ADR-028 is the why.
//!
//! **"Miss" is not the word.** `dispersion` uses *miss* for the shape of a golfer's error
//! distribution; this is *mishit*, and the two never share a name.
//!
//! **Why a fraction of the median, and not the flight model.** A genuine top prints a low ball
//! speed *and* a short carry, and they agree: nothing in the spin solve flags a carry consistent
//! with its own launch, however far it is from what the golfer meant. The only thing that says a
//! 20-yard 7 iron was a mistake is that this golfer's *other* 7 irons carry 150. So the rule is
//! relative to the club's own history, uses the median (robust to the very tops it is looking for),
//! and is gated at the count below which there is no history to be an outlier of.

use serde::{Deserialize, Serialize};

/// The golfer's own verdict on a shot, set through the per-swing repair route.
///
/// `None` on a manifest — the common case — means no verdict, and the automatic rule decides. A
/// verdict overrides that rule in either direction and is never inferred.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MishitVerdict {
    /// Yes, a mishit — hold it out of the distance averages.
    Confirmed,
    /// No, a real shot — count it, whatever the automatic rule thinks.
    Cleared,
}

impl MishitVerdict {
    /// Both verdicts, in declaration order. `tests/python_schemas.rs` holds it to the enum pydantic
    /// exported into `swing_manifest.schema.json`.
    pub const ALL: [MishitVerdict; 2] = [MishitVerdict::Confirmed, MishitVerdict::Cleared];

    /// The wire name.
    pub const fn as_str(self) -> &'static str {
        match self {
            MishitVerdict::Confirmed => "confirmed",
            MishitVerdict::Cleared => "cleared",
        }
    }
}

/// The measurements a mishit is withheld from, and the only ones. Ball speed, launch, start line,
/// face-to-path and every pose checkpoint still count a mishit shot: the swing was real, only its
/// distance is meaningless. A `frozenset` in Python, so the order here is nobody's; a caller that
/// prints the names sorts them, as `caveats.py` does.
pub const MISHIT_EXCLUDED_METRICS: [&str; 2] = ["carry_distance_yds", "total_distance_yds"];

/// Distinct carry samples a club needs before the automatic rule flags any of its shots.
///
/// The CENTER-claim floor of `contracts/baseline.py`'s default table, reused deliberately — below
/// it a club has no established carry for a short shot to be an outlier *of*. A literal in Python
/// too; the baseline's table lands at M36 P6, which is where the two can be held equal.
pub const MISHIT_MIN_CLEAN_SHOTS: usize = 5;

/// How far below a club's own median carry a shot has to fall to be an automatic mishit. A
/// judgment, and the one number in M16 a bay session is expected to move.
pub const MISHIT_CARRY_FRACTION: f64 = 0.50;

/// Why [`MISHIT_CARRY_FRACTION`] is 0.50, verbatim from the Python constant.
pub const MISHIT_CARRY_FRACTION_PROVENANCE: &str = "judgment, 2026-09-07 (M16 P1): half a club's own median carry is a topped or bladed shot, not the low edge of normal dispersion - a 7 iron that carries 150 does not carry 75 on a real strike. Deliberately loose: it removes only shots no average should contain and leaves heavier contact (a 90-yard chunk off a 150-yard club) for a manual verdict. No instrument-error evidence stands behind 0.50; revise from a bay session with labelled tops and duffs";

/// Carry below which a shot of this club is a gross mishit, or `None` with too few samples.
///
/// `carries` is the club's pooled carry set — one value per distinct shot photo, the set the
/// baseline means over — so the median is the one the golfer sees, and one or two tops barely move
/// it, which is the whole reason it is the median and not the mean.
pub fn mishit_carry_floor(carries: &[f64]) -> Option<f64> {
    if carries.len() < MISHIT_MIN_CLEAN_SHOTS {
        return None;
    }
    Some(MISHIT_CARRY_FRACTION * median(carries))
}

/// `statistics.median`: sort, then the middle value, or the mean of the middle two as
/// `(low + high) / 2` — Python's own arithmetic, so an even count rounds as it does.
///
/// A stable sort on `<`, as `sorted` is, so values that compare equal (`-0.0` and `0.0`) keep their
/// order and the middle one is the same element Python picks. No NaN reaches it: a carry is a
/// bounded `ShotData` field.
fn median(values: &[f64]) -> f64 {
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = sorted.len();
    if n % 2 == 1 {
        sorted[n / 2]
    } else {
        (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn too_few_samples_is_no_floor_not_a_low_one() {
        let carries = [155.0, 20.0, 160.0, 158.0];
        assert!(carries.len() < MISHIT_MIN_CLEAN_SHOTS);
        assert_eq!(mishit_carry_floor(&carries), None);
    }

    #[test]
    fn the_floor_is_half_the_median() {
        let carries = [155.0, 160.0, 158.0, 162.0, 159.0];
        assert_eq!(
            mishit_carry_floor(&carries),
            Some(MISHIT_CARRY_FRACTION * 159.0)
        );
    }

    #[test]
    fn a_gross_top_is_below_the_floor_and_a_low_normal_shot_is_not() {
        let floor = mishit_carry_floor(&[155.0, 160.0, 158.0, 162.0, 159.0]).unwrap();
        assert!(20.0 < floor);
        assert!(140.0 > floor);
    }

    #[test]
    fn the_median_does_not_chase_the_tops_it_is_looking_for() {
        let floor = mishit_carry_floor(&[150.0, 152.0, 148.0, 151.0, 20.0]).unwrap();
        assert_eq!(floor, MISHIT_CARRY_FRACTION * 150.0);
        assert!(20.0 < floor);
        assert!(130.0 > floor);
    }

    /// An even count averages the middle two, which P1's finding 5 recorded as a case the corpus
    /// reaches; and the sum is Python's `a + b` then `/ 2`, not `a / 2 + b / 2`.
    #[test]
    fn an_even_count_averages_the_middle_two() {
        let carries = [140.0, 150.0, 151.0, 160.0, 20.0, 155.0];
        assert_eq!(median(&carries), (150.0 + 151.0) / 2.0);
        assert_eq!(
            mishit_carry_floor(&carries),
            Some(MISHIT_CARRY_FRACTION * 150.5)
        );
    }

    #[test]
    fn excluded_metrics_are_carry_and_total_only() {
        let mut names = MISHIT_EXCLUDED_METRICS;
        names.sort();
        assert_eq!(names, ["carry_distance_yds", "total_distance_yds"]);
    }

    #[test]
    fn verdict_values_round_trip() {
        for verdict in [MishitVerdict::Confirmed, MishitVerdict::Cleared] {
            let wire = serde_json::to_string(&verdict).unwrap();
            assert_eq!(wire, format!("\"{}\"", verdict.as_str()));
            assert_eq!(
                serde_json::from_str::<MishitVerdict>(&wire).unwrap(),
                verdict
            );
        }
    }
}
