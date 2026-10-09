//! `scripts/career_dispersion.py`'s report: what the *shape* of a golfer's miss points at.
//! [M36 P15]
//!
//! The third of the three career reports, and they answer three different questions about one
//! corpus: `career_corpus` prints the honest `n`, `career_baseline` what that `n` buys, and this what
//! the numbers are *evidence for*, a repeatable miss (look at what is fixed before the swing) against
//! a scattered one (look at timing and release). Every refusal says which of the two kinds it is,
//! waiting for swings or waiting for a band, so the report doubles as a worklist.

use contracts::dispersion::{target_for, Finding, GolferDispersion, MetricDispersion};

use super::{fmt, header, plural, wrap, Printed};

/// How each finding prints (`_FINDING_LABEL`). `NotEstablished` is deliberately not "no": it is the
/// answer "there is enough data to ask and the data does not settle it", and a column reading "no"
/// beside a mean of 2.5 degrees would be read as a clean bill.
fn finding_label(finding: Finding) -> &'static str {
    match finding {
        Finding::Established => "yes",
        Finding::NotEstablished => "cannot tell",
        Finding::Withheld => "withheld",
    }
}

/// Width of the label column. "tolerance" is the longest and needs a gap after it: at 9 it butted
/// straight up against the value and printed "tolerance2".
const LABEL_WIDTH: usize = 10;

/// The wrap width of a labelled paragraph (`_wrap`'s default).
const WRAP_WIDTH: usize = 78;

/// The label column, so every row in a metric block lines up at the same column (`_label`).
fn label(label: &str) -> String {
    format!("    {label:<LABEL_WIDTH$}")
}

/// A labelled paragraph, wrapped and hanging-indented under its label (`_print_block`). The reasons
/// here are sentences, not fragments, because a refusal has to say what it is waiting for, and
/// several run past 200 characters.
fn print_block(out: &mut Printed, name: &str, text: &str) {
    for (i, line) in wrap(Some(text), WRAP_WIDTH).iter().enumerate() {
        out.line(format!("{}{line}", label(if i == 0 { name } else { "" })));
    }
}

/// One metric's block (`_report_metric`).
fn report_metric(out: &mut Printed, metric: &MetricDispersion, verbose: bool) {
    let unit = metric.unit.as_str();
    out.line(format!("\n  {}  ({unit})", metric.name));

    let mut head = format!(
        "    n = {} over {} session{}",
        metric.n,
        metric.n_sessions,
        plural(metric.n_sessions)
    );
    if let (Some(target), Some(tolerance)) = (metric.target, metric.tolerance) {
        head += &format!(
            "    target {} +/- {}",
            fmt(target, unit),
            fmt(tolerance, unit)
        );
    }
    out.line(head);

    // As in the baseline report, these conditions are not calls to a "may I show this" predicate:
    // a finding that was never established leaves no number behind.
    let mut bias = label("bias") + finding_label(metric.bias);
    if let Some(center) = metric.center {
        bias += &format!("    center {}", fmt(center, unit));
        if let Some(ci) = &metric.center_ci {
            bias += &format!(" (95% CI {} .. {})", fmt(ci.low, unit), fmt(ci.high, unit));
        }
        // Only worth a column when it is not the center restated: every target in the table today
        // is either 0.0 or absent, so this stays quiet until a nonzero one is declared. `and
        // metric.target` is truthiness, so a target of zero (either sign) prints nothing.
        if let (Some(offset), Some(target)) = (metric.offset, metric.target) {
            if target != 0.0 {
                bias += &format!("    off target by {}", fmt(offset, unit));
            }
        }
    }
    out.line(bias);

    let mut scatter = label("scatter") + finding_label(metric.scatter);
    if let Some(sd) = metric.sd {
        scatter += &format!("    sd {}", fmt(sd, unit));
        if let Some(ci) = &metric.sd_ci {
            scatter += &format!(" (95% CI {} .. {})", fmt(ci.low, unit), fmt(ci.high, unit));
        }
    }
    if let Some(within) = metric.within_session_sd {
        scatter += &format!("    within-session {}", fmt(within, unit));
    }
    out.line(scatter);

    if let Some(pattern) = metric.pattern {
        out.line(label("pattern") + &pattern.as_str().replace('_', " "));
    }
    for line in wrap(metric.points_at.as_deref(), WRAP_WIDTH) {
        out.line(label("") + &line);
    }
    for caveat in &metric.caveats {
        print_block(out, "caveat", caveat);
    }

    for refusal in &metric.withheld {
        print_block(out, "waiting", &refusal.reason);
    }
    for reason in &metric.unavailable {
        print_block(out, "blocked", reason);
    }

    if verbose && metric.tolerance.is_some() {
        if let Some(declared) = target_for(&metric.name) {
            print_block(
                out,
                "tolerance",
                &format!("{} — {}", pyfmt::g(declared.tolerance), declared.provenance),
            );
        }
    }
}

/// The report `career_dispersion._report(dispersion, display_name, verbose=…)` prints.
pub fn career_dispersion(
    dispersion: &GolferDispersion,
    display_name: &str,
    verbose: bool,
) -> String {
    let mut out = Printed::default();
    header(&mut out, display_name, &dispersion.player_id);

    if dispersion.metrics.is_empty() {
        out.line("  No measurement on any swing yet — nothing to read a miss-shape from.");
        out.line("  Check `python scripts/career_corpus.py` for why.");
        return out.into_string();
    }

    out.line(format!(
        "  Built from {} distinct swing{} across {} session{}.",
        dispersion.built_from_swings,
        plural(dispersion.built_from_swings),
        dispersion.built_from_sessions,
        plural(dispersion.built_from_sessions)
    ));

    let nothing_established = dispersion.nothing_established();
    if nothing_established {
        // One `print` holding a newline, as the script's does.
        out.line(
            "\n  No metric can be asked yet. A repeatable miss and a scattered one are the same\n  \
             two numbers until there are enough of them to tell apart.",
        );
    }

    for metric in dispersion.metrics.values() {
        report_metric(&mut out, metric, verbose);
    }

    if nothing_established {
        out.line("\n  The unblock is a bay session: 20-30 swings with shots attached.");
    }
    out.into_string()
}
