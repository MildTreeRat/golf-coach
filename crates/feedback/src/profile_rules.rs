//! The strengths and weaknesses list beside a profile's grades: ADR-034 §5.7 as code. [M37 P8]
//!
//! **No Python twin, and none is coming.** Frozen Python never had a `rank_profile` (the M37 plan's
//! finding 1: the M31 design called it "unchanged in rule", and the rule is all that existed), it
//! gains nothing now (ADR-035 clause 4), and the list's kinds, order and ratios are worked by hand
//! in `spec/vectors/profile/hand/` rather than recorded from an oracle. So, like
//! `contracts::strike`, there is nothing here to mirror one for one. The rule is ADR-034 §5.7's,
//! and the M37 plan's call 12 is how it reads the numbers.
//!
//! # The grade says how often; the list says what is established
//!
//! The two do not feed each other (ADR-034 §5.7): the grades are `analysis::strike`'s shares of good
//! shots, and the list is built here from the career layer's dispersion, which already decided every
//! bias and scatter by an interval against a tolerance (`analysis::dispersion`). This module
//! **re-decides nothing**: a weakness is the dispersion's own established finding, and a strength is
//! the dispersion's own two intervals read the other way. `crates/core` joins the two halves, because
//! this crate may not import `analysis` (ADR-008 as a cargo edge; `Cargo.toml` says why, and
//! `tests/profile_rules.rs` pins it).
//!
//! # What may list
//!
//! A row lists only when it is a **launch-monitor** row (`source` under
//! [`LAUNCH_MONITOR_SOURCE_PREFIX`]) with a **stat scope** and a **registered tolerance**. Each of the
//! three keeps something specific off:
//!
//! - The scope ([`scope_of`]) keeps **mechanics** off (Decision 12), because no pose metric has one.
//!   A pose row can establish a bias as readily as a shot row can — `head_sway_norm` and
//!   `finish_balance_norm` both do in the hand case `the-list-beside-a-drill-and-pose`, in the 7
//!   iron's own dispersion and the whole corpus' — and the scope table, not an absence of data, is
//!   what keeps them out. The scope is also what the player's list filters on.
//! - The source keeps a **shot number from another instrument** off, whatever its name. Today no row
//!   reaches it that the scope has not already refused, and it is not redundant: every shot
//!   tolerance in `METRIC_TARGETS` is a judgment about the OCR path (its `JUDGED_*` provenance), so
//!   a `start_line_deg` read off video, or a pose metric given a scope tomorrow (tempo is arguably
//!   club-independent), would be ranked against an error term nobody derived for it. Mechanics then
//!   stays off by its source.
//! - The tolerance is what a ratio divides by. The `flight_*` rows have a scope and no tolerance
//!   (`METRIC_TARGETS` leaves them out on purpose: five re-report measured rows through a model and
//!   one is solved, not measured), so a projection never lists as a fact about the golfer.
//!
//! A row with a tolerance and **no target** (carry, total, ball speed, launch) can still be a
//! scatter weakness and can never be a strength: a strength asserts closeness to a target, and how
//! far a golfer should hit a 7 iron is not a number this repo has.
//!
//! # A weakness, and how it ranks
//!
//! An **established** bias or scatter, ranked by its point estimate against the tolerance:
//! `|offset| / tolerance` for a bias and `sd / tolerance` for scatter. Both exceed 1 whenever the
//! finding is established, since the interval that established it sits clear of the tolerance and
//! holds the estimate. Dividing by the tolerance is what lets a two-degree face and a five-yard carry
//! share one list: each is measured in multiples of what this instrument's error explains. The
//! interval decided *whether*; the estimate says *how much*, and ranking on an interval bound would
//! rank by sample size as much as by the miss.
//!
//! # A strength needs both halves
//!
//! A metric **with a target** whose mean's 95% interval lies inside `target ± tolerance`
//! (**inclusive**) *and* whose SD's upper bound sits **strictly** below the tolerance. Its ratio is
//! the nearer of the two to failing, `max(|center_ci end − target|, sd_ci.high) / tolerance`: at
//! most 1 (exactly 1 only with an interval end on the band's edge), and the smaller the stronger.
//! The two edges are the hand cases' as worked (the M37 plan's P5 finding 6), and each is pinned
//! from both sides in the tests. They cannot collide with a weakness: an established bias has its
//! whole interval strictly outside the band, and established scatter has its SD's *lower* bound
//! strictly above the tolerance, so no metric is both, which `contracts::strike`'s list check holds.
//!
//! **"Nothing established" is never a strength.** `DispersionPattern::NothingEstablished` means the
//! intervals straddle the tolerance, which is consistent with a real miss and with none, and the
//! containment test above refuses a straddle. A metric withheld for want of `n` has no intervals at
//! all, and lists nothing.
//!
//! # The order
//!
//! Weaknesses first, largest ratio first, ties in `METRIC_TARGETS` order and then bias before
//! scatter; then strengths, smallest ratio first, ties in `METRIC_TARGETS` order. The sort decides
//! every position from the ratio, the table and the kind, so the map's own (alphabetical) order
//! reaches nothing. Ratios compare by `total_cmp`, a total order, so no position is left to the
//! order the metrics arrived in.
//!
//! # The player's list
//!
//! [`rank_player`] reads the **whole corpus'** dispersion, kept to club-independent metrics: a carry
//! "scatter" pooled across a driver and a wedge is the bag, not the golfer. It takes the
//! `GolferDispersion` alone and no `PersonalBaseline`, because every number the rule reads is on the
//! dispersion already: `center_ci` and `sd_ci` are the baseline's `mean_ci` and `sd_ci`, carried and
//! never recomputed, so a second argument would be a second spelling of the same two intervals,
//! free to disagree with the first.

use std::cmp::Ordering;

use contracts::career::LAUNCH_MONITOR_SOURCE_PREFIX;
use contracts::club_profile::ClubProfile;
use contracts::dispersion::{Finding, GolferDispersion, MetricDispersion, METRIC_TARGETS};
use contracts::strike::{is_club_independent, scope_of, FindingKind, ProfileFinding};

/// One club's list, read off its `ClubProfile.dispersion`: club-only rows included, since yards
/// and mph mean something within one club.
pub fn rank_profile(profile: &ClubProfile) -> Vec<ProfileFinding> {
    rank(profile.dispersion.values())
}

/// The player's list, read off a whole-corpus dispersion (`analysis::dispersion::build_dispersion`
/// over the corpus `analysis::strike::build_player_grades` grades) and kept to club-independent
/// metrics.
pub fn rank_player(dispersion: &GolferDispersion) -> Vec<ProfileFinding> {
    rank(
        dispersion
            .metrics
            .values()
            .filter(|metric| is_club_independent(&metric.name)),
    )
}

/// A finding with what the sort needs beside it: the metric's row in `METRIC_TARGETS`.
struct Ranked {
    finding: ProfileFinding,
    row: usize,
}

fn rank<'a>(metrics: impl Iterator<Item = &'a MetricDispersion>) -> Vec<ProfileFinding> {
    let mut weaknesses = Vec::new();
    let mut strengths = Vec::new();
    for metric in metrics {
        let (Some(row), Some(tolerance)) = (listable_row(metric), metric.tolerance) else {
            continue;
        };
        let mut push = |kind, ratio| {
            let ranked = Ranked {
                finding: ProfileFinding {
                    metric: metric.name.clone(),
                    kind,
                    ratio,
                },
                row,
            };
            if kind == FindingKind::Strength {
                strengths.push(ranked);
            } else {
                weaknesses.push(ranked);
            }
        };
        if metric.bias == Finding::Established {
            if let Some(offset) = metric.offset {
                push(FindingKind::Bias, offset.abs() / tolerance);
            }
        }
        if metric.scatter == Finding::Established {
            if let Some(sd) = metric.sd {
                push(FindingKind::Scatter, sd / tolerance);
            }
        }
        if let Some(ratio) = strength_ratio(metric, tolerance) {
            push(FindingKind::Strength, ratio);
        }
    }
    weaknesses.sort_by(|a, b| {
        b.finding
            .ratio
            .total_cmp(&a.finding.ratio)
            .then(a.row.cmp(&b.row))
            .then(weakness_order(a.finding.kind, b.finding.kind))
    });
    strengths.sort_by(|a, b| {
        a.finding
            .ratio
            .total_cmp(&b.finding.ratio)
            .then(a.row.cmp(&b.row))
    });
    weaknesses
        .into_iter()
        .chain(strengths)
        .map(|ranked| ranked.finding)
        .collect()
}

/// The metric's `METRIC_TARGETS` row, when it may list at all: a launch-monitor row with a scope
/// and a registered tolerance (the module doc's three filters). `ProfileFinding`'s own check asks
/// for the scope and the registration too, so no metric this admits is refused there for its name.
fn listable_row(metric: &MetricDispersion) -> Option<usize> {
    if !metric.source.starts_with(LAUNCH_MONITOR_SOURCE_PREFIX) || scope_of(&metric.name).is_none()
    {
        return None;
    }
    METRIC_TARGETS
        .iter()
        .position(|row| row.metric == metric.name)
}

/// Bias before scatter on one metric, at an equal ratio.
fn weakness_order(a: FindingKind, b: FindingKind) -> Ordering {
    let place = |kind| match kind {
        FindingKind::Bias => 0,
        FindingKind::Scatter => 1,
        FindingKind::Strength => 2,
    };
    place(a).cmp(&place(b))
}

/// The strength's ratio, when both halves are established; `None` otherwise, which includes every
/// metric with no target and every metric withheld for want of `n`.
///
/// Every comparison here is one a NaN fails, so a NaN anywhere is no strength rather than a
/// strength with an unrankable ratio.
fn strength_ratio(metric: &MetricDispersion, tolerance: f64) -> Option<f64> {
    let (target, center_ci, sd_ci) = (
        metric.target?,
        metric.center_ci.as_ref()?,
        metric.sd_ci.as_ref()?,
    );
    let inside = center_ci.low >= target - tolerance && center_ci.high <= target + tolerance;
    let tight = sd_ci.high < tolerance;
    if !(inside && tight) {
        return None;
    }
    let nearest_to_failing = (center_ci.low - target)
        .abs()
        .max((center_ci.high - target).abs())
        .max(sd_ci.high);
    Some(nearest_to_failing / tolerance)
}
