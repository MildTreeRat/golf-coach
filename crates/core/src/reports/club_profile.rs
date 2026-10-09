//! `scripts/club_profile.py`'s report: how far one golfer hits each club, and where their history
//! refuses to say. [M36 P16]
//!
//! The fourth career-shaped report. The other three ask one corpus three questions over the whole
//! bag; this one cuts the same corpus by club, which is the only cut on which a distance means
//! anything: a mean carry pooled over a driver and a wedge describes nobody's shot. Per club, a
//! baseline and a miss shape are one row of one table, so this report merges what
//! `career_baseline` and `career_dispersion` each print half of, rather than asking for two
//! commands to read one club.
//!
//! **The empty state is a finding, not a blank page.** A club with no swings and no bag entry gets
//! no row at all, so a corpus whose swings name no club prints [`report_empty`]'s paragraph: how many
//! untagged swings there are, and where a tag comes from. A golfer's name followed by nothing would
//! read as a bug in the verb when the finding is that nothing on disk names a club.
//!
//! This report nests one level deeper than the career ones (a metric sits inside a club, which sits
//! inside a golfer), so its label column takes an indent, and every labelled paragraph wraps at
//! `94 - indent - 10` (`_print_block`).

use contracts::bag::BagEntry;
use contracts::baseline::{BaselineClaim, MetricBaseline};
use contracts::club::ClubId;
use contracts::club_profile::{BagProfile, ClubProfile};
use contracts::dispersion::{Finding, MetricDispersion};

use super::{fmt, header, plural, wrap, Printed};

/// How each finding prints, copied with its reasoning by the script from `career_dispersion.py`:
/// `NotEstablished` is not "no", it is "there is enough data to ask and the data does not settle
/// it", and a column reading "no" beside a real number would be read as a clean bill.
fn finding_label(finding: Finding) -> &'static str {
    match finding {
        Finding::Established => "yes",
        Finding::NotEstablished => "cannot tell",
        Finding::Withheld => "withheld",
    }
}

/// Width of the label column, from `career_dispersion.py`. "untagged" is the longest here.
const LABEL_WIDTH: usize = 10;

/// The line a labelled paragraph fills, indent and label column included: `_print_block` wraps its
/// text at `94 - indent - LABEL_WIDTH`.
const LINE_WIDTH: usize = 94;

/// The metric blocks' indent, `_label` and `_print_block`'s default.
const METRIC_INDENT: usize = 6;

/// `_plural`: the count, the noun, and an `s` unless the count is one.
fn counted(count: i64, noun: &str) -> String {
    format!("{count} {noun}{}", plural(count))
}

/// The label column at an indent, so every row in a block lines up (`_label`).
fn label(label: &str, indent: usize) -> String {
    format!("{}{label:<LABEL_WIDTH$}", " ".repeat(indent))
}

/// A labelled paragraph, wrapped and hanging-indented under its label (`_print_block`). The reasons
/// here are sentences, not fragments, because a refusal has to say what it is waiting for, and
/// several run past 200 characters.
fn print_block(out: &mut Printed, name: &str, text: &str, indent: usize) {
    for (i, line) in wrap(Some(text), LINE_WIDTH - indent - LABEL_WIDTH)
        .iter()
        .enumerate()
    {
        out.line(label(if i == 0 { name } else { "" }, indent) + line);
    }
}

// ----------------------------------------------------------------------------------- one metric

/// One metric for one club: what it says, what it will not say, and what it is evidence for
/// (`_report_metric`).
fn report_metric(
    out: &mut Printed,
    baseline: &MetricBaseline,
    dispersion: Option<&MetricDispersion>,
    verbose: bool,
) {
    let unit = baseline.unit.as_str();
    out.line(format!("\n    {}  ({unit})", baseline.name));
    out.line(format!(
        "      n = {} over {}",
        baseline.n,
        counted(baseline.n_sessions, "session")
    ));

    // Not calls to `supports()`: a withheld claim leaves no number behind, so "is there a number"
    // and "may it be shown" are the same question.
    if let (Some(mean), Some(median)) = (baseline.mean, baseline.median) {
        let mut line = label("center", METRIC_INDENT) + &fmt(mean, unit);
        if let Some(ci) = &baseline.mean_ci {
            line += &format!("   95% CI {} .. {}", fmt(ci.low, unit), fmt(ci.high, unit));
        }
        out.line(format!("{line}   (median {})", fmt(median, unit)));
    }

    if let (Some(sd), Some(minimum), Some(maximum)) =
        (baseline.sd, baseline.minimum, baseline.maximum)
    {
        let mut line = label("spread", METRIC_INDENT) + &format!("sd {}", fmt(sd, unit));
        if let Some(ci) = &baseline.sd_ci {
            line += &format!("   95% CI {} .. {}", fmt(ci.low, unit), fmt(ci.high, unit));
        }
        out.line(format!(
            "{line}   (range {} .. {})",
            fmt(minimum, unit),
            fmt(maximum, unit)
        ));
    }

    if baseline.supports(BaselineClaim::Trend) {
        out.line(label("trend", METRIC_INDENT) + "per session:");
        for session in &baseline.sessions {
            let mean = session
                .mean
                .map_or_else(|| "—".to_string(), |mean| fmt(mean, unit));
            out.line(
                label("", METRIC_INDENT)
                    + &format!("  {}  n={}  mean {mean}", session.session_id, session.n),
            );
        }
    }

    if let Some(dispersion) = dispersion {
        report_findings(out, dispersion);
    }

    // These sentences explain the absence of everything above them, so they follow it. The
    // baseline's refusals before the dispersion's `unavailable`, as `career_dispersion` orders
    // `waiting` and `blocked`: the two need opposite responses, and the one no amount of swinging
    // fixes reads best as the last word.
    for refusal in &baseline.withheld {
        print_block(out, refusal.claim.as_str(), &refusal.reason, METRIC_INDENT);
    }
    if let Some(dispersion) = dispersion {
        for reason in &dispersion.unavailable {
            print_block(out, "blocked", reason, METRIC_INDENT);
        }
    }

    if verbose && !baseline.supports(BaselineClaim::Trend) {
        // The evidence behind the trend refusal. Counts only: a per-session mean is itself a claim,
        // and it stays sealed with the rest of them.
        out.line(label("", METRIC_INDENT) + "sessions so far:");
        for session in &baseline.sessions {
            out.line(
                label("", METRIC_INDENT) + &format!("  {}  n={}", session.session_id, session.n),
            );
        }
    }
}

/// The bias/scatter pair, a repeatable miss against a scattered one (`_report_findings`).
///
/// **`metric.withheld` is deliberately not printed.** The dispersion builder's carried refusals
/// *filter* the baseline's own, so every sentence in that list is already printed verbatim by
/// [`report_metric`], and rendering both would say each thing twice. Nothing is hidden: the labels
/// still read "withheld", and `unavailable`, which says something different, is printed by the
/// caller after the refusals.
fn report_findings(out: &mut Printed, metric: &MetricDispersion) {
    let unit = metric.unit.as_str();
    let mut line = label("finding", METRIC_INDENT)
        + &format!(
            "bias {}    scatter {}",
            finding_label(metric.bias),
            finding_label(metric.scatter)
        );
    if let (Some(target), Some(tolerance)) = (metric.target, metric.tolerance) {
        line += &format!(
            "    target {} +/- {}",
            fmt(target, unit),
            fmt(tolerance, unit)
        );
    }
    out.line(line);

    // Only worth a row when it is not the center restated: every target in the table today is
    // either 0.0 or absent. `and metric.target` is truthiness, so a target of zero (either sign)
    // prints nothing, and so does an absent one.
    if let (Some(offset), Some(target)) = (metric.offset, metric.target) {
        if target != 0.0 {
            out.line(label("", METRIC_INDENT) + &format!("off target by {}", fmt(offset, unit)));
        }
    }
    if let Some(within) = metric.within_session_sd {
        out.line(label("", METRIC_INDENT) + &format!("within-session sd {}", fmt(within, unit)));
    }
    if let Some(pattern) = metric.pattern {
        out.line(label("pattern", METRIC_INDENT) + &pattern.as_str().replace('_', " "));
    }
    if let Some(points_at) = metric.points_at.as_deref().filter(|text| !text.is_empty()) {
        print_block(out, "check", points_at, METRIC_INDENT);
    }
    for caveat in &metric.caveats {
        print_block(out, "caveat", caveat, METRIC_INDENT);
    }
}

// ------------------------------------------------------------------------------------- one club

/// One club's block (`_report_club`): its physical club, the evidence, the caveats, then a metric
/// block for each metric, or why there is none.
fn report_club(out: &mut Printed, profile: &ClubProfile, verbose: bool) {
    out.line(format!(
        "\n  {}  ({})",
        profile.club.as_str(),
        profile.category().as_str().replace('_', " ")
    ));
    print_bag_entry(out, profile.bag_entry.as_ref());

    // All three counters, never one: a 7 iron filmed six times with two shot-screen photos is six
    // swings of history and a carry ceiling of two, because every launch-monitor claim dedupes on
    // the photo's hash (`contracts::club_profile`'s doc).
    let mut evidence = format!(
        "{}, {}, {}",
        counted(profile.n_swings, "swing"),
        counted(profile.n_shots, "shot photo"),
        counted(profile.n_sessions, "session")
    );
    if profile.n_shots < profile.n_swings {
        evidence += " — so every distance and launch number below is capped at the shot count";
    }
    print_block(out, "evidence", &evidence, 4);

    for caveat in &profile.caveats {
        print_block(out, "caveat", caveat, 4);
    }

    if profile.metrics.is_empty() {
        print_block(out, "history", no_metrics_reason(profile), 4);
        return;
    }

    // `ClubProfile.metrics` is a `BTreeMap`, which is the order Python's dict holds: the builder
    // inserts sorted by name, as the baseline's does.
    for (name, baseline) in &profile.metrics {
        report_metric(out, baseline, profile.dispersion.get(name), verbose);
    }
}

/// Why a profile exists with nothing in it, and the two cases need opposite responses
/// (`_no_metrics_reason`). A declared club nobody has hit is the state every club is in the day it
/// is added to the bag; a club with swings and no measurement is a pipeline problem, which
/// `career_corpus` explains.
///
/// The sentences name `python scripts/career_corpus.py` because that is what the script printed and
/// what the career family recorded; `golf-core career-corpus` answers the same question, and the
/// text moves when a declared re-record moves it, not here.
fn no_metrics_reason(profile: &ClubProfile) -> &'static str {
    if profile.n_swings == 0 {
        return "In the bag, nothing hit with it yet. No statistics, and that is not an error — the \
                profile is here so the club is visible before it has any history.";
    }
    "These swings carry no measurement. Run `python scripts/career_corpus.py` for why — the \
     commonest causes are swings that were never analyzed, and swings analyzed by an older engine."
}

/// The physical club in this slot, or what its absence costs (`_print_bag_entry`).
///
/// An undeclared entry is spelled out rather than left blank because both halves need saying
/// together: it takes nothing away from the distance statistics, and it makes every loft or fitting
/// question about this club refuse (ADR-024 §2). A blank cell would read as a zero loft.
///
/// The entry's line goes through the wrap, which splits on whitespace, so the script's two spaces
/// before `(declared …)` print as one, here as there.
fn print_bag_entry(out: &mut Printed, entry: Option<&BagEntry>) {
    let Some(entry) = entry else {
        print_block(
            out,
            "club",
            "No bag entry declared. Every statistic below is unaffected; a loft or fitting \
             question about this club has to refuse. Look the club up and confirm it to fill \
             this in (M12).",
            4,
        );
        return;
    };

    let spec = &entry.spec;
    let described = [spec.make.as_str(), spec.model.as_str()]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    let mut line = if described.is_empty() {
        "make and model not recorded".to_string()
    } else {
        described
    };
    match spec.loft_deg {
        Some(loft) => line += &format!(", {} deg loft", pyfmt::g(loft)),
        None => line += ", loft not recorded",
    }
    if !spec.shaft_model.is_empty() {
        line += &format!(", {} shaft", spec.shaft_model);
    }
    if let Some(length) = spec.length_in {
        line += &format!(", {} in", pyfmt::g(length));
    }
    // `f"{recorded_at:%Y-%m-%d}"` is the date in the timestamp's own offset, never converted to UTC
    // first (P3's `bag-declared` dates a 23:30-05:00 entry by its own calendar).
    line += &format!("  (declared {})", entry.recorded_at.date_ymd());
    print_block(out, "club", &line, 4);
}

// ----------------------------------------------------------------------------------- one golfer

/// The report `club_profile._report(profile, display_name, verbose=…, club=…)` prints: every club
/// with history or a declared entry, or the one `club` names, then the swings no club counts.
pub fn club_profile(
    profile: &BagProfile,
    display_name: &str,
    verbose: bool,
    club: Option<ClubId>,
) -> String {
    let mut out = Printed::default();
    header(&mut out, display_name, &profile.player_id);

    if let Some(club) = club {
        // `BagProfile::profile_for`, never a hand-rolled scan, so this and every other consumer
        // agree about what "no such club" means.
        match profile.profile_for(club) {
            None => out.line(format!(
                "  {}: never hit and not in the bag — there is no history to report.",
                club.as_str()
            )),
            Some(one) => report_club(&mut out, one, verbose),
        }
        print_untagged(&mut out, profile);
        return out.into_string();
    }

    if profile.clubs.is_empty() {
        report_empty(&mut out, profile);
        return out.into_string();
    }

    out.line(format!(
        "  {} with history or a declared bag entry: {} hit, {} in the bag.",
        counted(count(profile.clubs.len()), "club"),
        profile.clubs_used().len(),
        profile.clubs_declared().len()
    ));
    for one in &profile.clubs {
        report_club(&mut out, one, verbose);
    }
    print_untagged(&mut out, profile);
    out.into_string()
}

/// A list's length as the `i64` the counters are, for [`counted`].
fn count(len: usize) -> i64 {
    i64::try_from(len).expect("a bag holds fewer clubs than i64::MAX")
}

/// No club hit and none declared (`_report_empty`). Its own branch because "no clubs" and "clubs
/// that cannot say anything yet" look alike from a distance and need opposite responses: a table of
/// refusals means go and hit balls, an empty bag profile means nothing on disk names a club at all,
/// which no bay session fixes on its own. So it carries the untagged count and where a tag comes
/// from.
fn report_empty(out: &mut Printed, profile: &BagProfile) {
    out.line("  No club has been hit or declared — there is nothing to profile yet.");

    if profile.untagged_swings != 0 {
        // Labelled with the word the populated path uses: this *is* `print_untagged`'s number,
        // printed here because the empty state returns before reaching it.
        print_block(
            out,
            "untagged",
            &format!(
                "All {} on record name no club. That is real history, and no per-club number can \
                 be cut from it until each one is tagged.",
                counted(profile.untagged_swings, "swing")
            ),
            2,
        );
    } else {
        out.line("  No swings on record either.");
    }

    print_block(
        out,
        "next",
        "New swings get a club at upload — `python scripts/run_server.py`, then the picker on the \
         upload page. A swing already stored is retagged one at a time, from the change control on \
         its row in that page's swing list or with POST \
         /api/sessions/<session>/swings/<swing>/club.",
        2,
    );
}

/// The history no club profile above can see, printed last and never folded into a club
/// (`_print_untagged`). `untagged_swings` is read off the whole corpus, so it is the one number here
/// every per-club narrowing reports as zero by construction; leaving it out would make a bag look
/// complete when most of the golfer's swings are missing from it.
fn print_untagged(out: &mut Printed, profile: &BagProfile) {
    if profile.untagged_swings == 0 {
        return;
    }
    out.line("");
    print_block(
        out,
        "untagged",
        &format!(
            "{} on record name no club, so nothing above counts them. Retag one at a time, from the \
             change control on the swing's row in the upload page or with POST \
             /api/sessions/<session>/swings/<swing>/club.",
            counted(profile.untagged_swings, "swing")
        ),
        2,
    );
}
