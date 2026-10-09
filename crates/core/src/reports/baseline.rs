//! `scripts/career_baseline.py`'s report: what one golfer's own history supports saying, and what it
//! refuses to. [M36 P15]
//!
//! `career_corpus` prints the honest `n`; this prints what that `n` buys. Every refusal names what
//! it is waiting for, so the report doubles as a worklist. The `vs tour` row belongs here rather than
//! in a report of its own because a standing is a statement about the *center*, which is what this
//! report is about.

use contracts::baseline::{BaselineClaim, MetricBaseline, PersonalBaseline};
use contracts::comparison::{GolferStanding, MetricComparison, Standing};

use super::{fmt, header, plural, Printed};

/// How each standing prints, the script's `_STANDING_LABEL` and `_standing_label`. Two deliberate
/// choices, the second found by rendering a corpus that could actually speak:
///
/// - `straddles` is not "borderline". The interval crosses the band edge, so which side the golfer
///   is on is unresolved rather than nearly decided.
/// - `outside` is never printed bare, because it reads as a fault and for half the panel it is the
///   opposite. `finish_balance_norm` sits on a one-sided `[0, high]` band, so a center below p10 is
///   *better balanced* than the tour population, and "outside tour range" rendered that identically
///   to a center above p90. Naming the side is factual where the bare word is a verdict.
fn standing_label(standing: &MetricComparison) -> &'static str {
    match standing.standing {
        Standing::Outside => {
            if standing.outside_by.is_some_and(|by| by < 0.0) {
                "below tour range"
            } else {
                "above tour range"
            }
        }
        Standing::Inside => "inside tour range",
        Standing::Straddles => "cannot tell",
        Standing::Withheld => "withheld",
    }
}

/// Where the center sits in the tour population, or why it cannot be placed (`_report_standing`).
///
/// Prints the band whenever one exists, even while the standing itself is withheld: "we have a
/// reference for this and are waiting on your `n`" and "there is no reference and never will be
/// from this data" are the two situations, and the band is what tells them apart at a glance.
fn report_standing(out: &mut Printed, standing: &MetricComparison, unit: &str) {
    let mut line = format!("    vs tour  {}", standing_label(standing));

    if let (Some(low), Some(high)) = (standing.band_low, standing.band_high) {
        line += &format!("   tour {} .. {}", fmt(low, unit), fmt(high, unit));
        if let Some(players) = standing.population_players {
            line += &format!(" ({players} players)");
        }
    }

    if let Some(percentile) = standing.percentile {
        // "at least" at the rails: `percentile_of` clamps at the stored quantiles, so a 90 there is
        // a floor on how extreme the center is rather than its rank.
        let qualifier = if standing.percentile_clamped {
            "at least "
        } else {
            ""
        };
        line += &format!("   {qualifier}{}th pct", pyfmt::fixed(percentile, 0));
    }

    if let Some(by) = standing.outside_by {
        let sign = if by > 0.0 { "+" } else { "" };
        line += &format!("   {sign}{} past the edge", fmt(by, unit));
    }

    out.line(line);

    for reason in &standing.unavailable {
        out.line(format!("    {:<8} blocked  — {reason}", ""));
    }
}

/// One metric's block (`_report_metric`).
fn report_metric(
    out: &mut Printed,
    metric: &MetricBaseline,
    standing: Option<&MetricComparison>,
    verbose: bool,
) {
    let unit = metric.unit.as_str();
    out.line(format!("\n  {}  ({unit})", metric.name));
    out.line(format!(
        "    n = {} over {} session{}",
        metric.n,
        metric.n_sessions,
        plural(metric.n_sessions)
    ));

    // Not a call to `supports()`: a withheld claim leaves no number behind, so "is there a number"
    // and "may it be shown" are the same question, which is the property the contract exists to
    // give every consumer, this one included.
    if let (Some(mean), Some(median)) = (metric.mean, metric.median) {
        let mut line = format!("    center   {}", fmt(mean, unit));
        if let Some(ci) = &metric.mean_ci {
            line += &format!("   95% CI {} .. {}", fmt(ci.low, unit), fmt(ci.high, unit));
        }
        out.line(format!("{line}   (median {})", fmt(median, unit)));
    }

    if let (Some(sd), Some(minimum), Some(maximum)) = (metric.sd, metric.minimum, metric.maximum) {
        let mut line = format!("    spread   sd {}", fmt(sd, unit));
        if let Some(ci) = &metric.sd_ci {
            line += &format!("   95% CI {} .. {}", fmt(ci.low, unit), fmt(ci.high, unit));
        }
        out.line(format!(
            "{line}   (range {} .. {})",
            fmt(minimum, unit),
            fmt(maximum, unit)
        ));
    }

    if metric.supports(BaselineClaim::Trend) {
        out.line("    trend    per session:");
        for session in &metric.sessions {
            let mean = session
                .mean
                .map_or_else(|| "—".to_string(), |mean| fmt(mean, unit));
            out.line(format!(
                "               {}  n={}  mean {mean}",
                session.session_id, session.n
            ));
        }
    }

    for refusal in &metric.withheld {
        out.line(format!(
            "    {:<8} withheld — {}",
            refusal.claim.as_str(),
            refusal.reason
        ));
    }

    // Last, because a standing is derived from the center: printing it above the row that says
    // there is no center reads as the two being unrelated.
    if let Some(standing) = standing {
        report_standing(out, standing, unit);
    }

    if verbose && !metric.supports(BaselineClaim::Trend) {
        // The evidence behind the trend refusal. Counts only: the per-session mean is itself a
        // claim, and it stays sealed with the rest of them.
        out.line("             sessions so far:");
        for session in &metric.sessions {
            out.line(format!(
                "               {}  n={}",
                session.session_id, session.n
            ));
        }
    }
}

/// The report `career_baseline._report(baseline, standing, display_name, verbose=…)` prints.
pub fn career_baseline(
    baseline: &PersonalBaseline,
    standing: &GolferStanding,
    display_name: &str,
    verbose: bool,
) -> String {
    let mut out = Printed::default();
    header(&mut out, display_name, &baseline.player_id);

    if baseline.metrics.is_empty() {
        out.line("  No measurement on any swing yet — nothing to build a baseline from.");
        out.line("  Check `python scripts/career_corpus.py` for why: the commonest causes are");
        out.line("  swings that were never analyzed, and swings analyzed by an older engine.");
        return out.into_string();
    }

    out.line(format!(
        "  Built from {} distinct swing{} across {} session{}.",
        baseline.built_from_swings,
        plural(baseline.built_from_swings),
        baseline.built_from_sessions,
        plural(baseline.built_from_sessions)
    ));

    let nothing_sayable = baseline.nothing_sayable();
    if nothing_sayable {
        out.line("\n  Nothing is sayable yet. Every claim below is refused, with what it needs.");
    }

    // `PersonalBaseline.metrics` is a `BTreeMap`, which is the order Python's dict holds: the
    // builder inserts sorted by name (M36 P6).
    for metric in baseline.metrics.values() {
        report_metric(
            &mut out,
            metric,
            standing.metrics.get(&metric.name),
            verbose,
        );
    }

    if nothing_sayable {
        out.line("\n  The unblock is a bay session: 20-30 swings with shots attached.");
    }
    out.into_string()
}
