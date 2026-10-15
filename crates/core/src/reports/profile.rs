//! `golf-core profile`'s report: a golfer's graded profile, each club's grades and list, then the
//! player's. [M37 P10]
//!
//! **No Python twin, and no script behind it.** The other renderers in [`super`] port a script's
//! `_report` and are held to the text it printed; this one renders [`GradedProfile`], which is
//! M37's and Rust's alone (`crate::profile`'s doc), so its wording is its own and is pinned by the
//! profile family's corpus cases once M37 P11 records them beside the profile they render.
//!
//! # What prints, and what does not
//!
//! The three states `contracts::strike` spells three ways print three ways:
//!
//! - **graded**: the share as a percentage, what it is a share of, `successes of n` and the Wilson
//!   interval. A shape topic adds "hit the shape" beneath and says where its landings came from,
//!   **naming a projected landing as projected** (ADR-034 §5.3);
//! - **withheld**: the word, the shots it was judged on and the floor it waits for, in words, so a
//!   withheld grade says what would lift it, and the blend names it as left out;
//! - **absent**: nothing. A topic a club's shots cannot be judged on gets no line, no name and no
//!   blend entry, exactly as the profile carries no entry for it. A club with no topic at all says
//!   so in one line, because a club header over a blank would read as a bug in the verb (the
//!   `club_profile` report's rule: the empty state is a finding).
//!
//! Every shot a topic could not judge is counted under its reason, in the reason's own clause from
//! `contracts::unscored::spec_for`, so the sentence cannot drift from the one the tip, the brief and
//! the MCP view print.
//!
//! **Mechanics prints nowhere here** (Decision 12): the profile carries no pose number, so there is
//! none to print, and the preamble says the grades are the launch monitor's alone.
//!
//! # Formatting
//!
//! Percentages are [`pyfmt::fixed`] at no places, which is half-even on an exact half, as every
//! other report here rounds; a measurement is [`super::fmt`] by its unit, as `club_profile` prints
//! one; a constant (the 10 mm radius, the 9-yard tolerance) is [`pyfmt::g`], so `9.0` reads `9`.
//! There is no Python text to match, so these are choices, and they are the other reports' choices
//! so that one golfer's reports read alike.

use contracts::baseline::{minimum_n, BaselineClaim, Interval, MetricBaseline, WithheldClaim};
use contracts::club::{category_of, ClubId};
use contracts::dispersion::{target_for, MetricDispersion};
use contracts::strike::{
    Blend, FaceColumn, FaceRow, FindingKind, GradedClub, GradedProfile, ImpactMap, OfflineBasis,
    ProfileFinding, Share, StrikeProfile, Topic, TopicGrade, UnscoredCount, CENTERED_RADIUS_MM,
    CONSISTENCY_FRACTION, ON_LINE_TOLERANCE_ROW, RATE_MINIMUM_N, STRAIGHT_TOLERANCE_ROW,
};
use contracts::unscored::spec_for;

use super::{fmt, header, plural, wrap, Printed};

/// Width of the label column ("topics", "spread", "list"), as `club_profile`'s.
const LABEL_WIDTH: usize = 10;

/// Width of the key column beside it: a topic, a metric, or a grid row. `start_line_offline_yds`,
/// the longest metric the list can name, is 22.
const KEY_WIDTH: usize = 24;

/// The line every block fills, indent and both columns included, as `club_profile`'s 94.
const LINE_WIDTH: usize = 94;

/// A club's (and the player's) blocks sit under its header at this indent.
const BLOCK_INDENT: usize = 4;

/// `count noun(s)`.
fn counted(count: i64, noun: &str) -> String {
    format!("{count} {noun}{}", plural(count))
}

/// A row: the label column, the key column, and text wrapped beneath the text column, every
/// continuation line hanging under the text rather than under the label.
fn row(out: &mut Printed, label: &str, key: &str, text: &str) {
    let indent = " ".repeat(BLOCK_INDENT);
    let lines = wrap(
        Some(text),
        LINE_WIDTH - BLOCK_INDENT - LABEL_WIDTH - KEY_WIDTH,
    );
    for (i, line) in lines.iter().enumerate() {
        let (label, key) = if i == 0 { (label, key) } else { ("", "") };
        out.line(format!(
            "{indent}{label:<LABEL_WIDTH$}{key:<KEY_WIDTH$}{line}"
        ));
    }
}

/// A row with no key column: the text starts right after the label and wraps beneath itself.
fn labelled(out: &mut Printed, label: &str, text: &str) {
    let indent = " ".repeat(BLOCK_INDENT);
    let lines = wrap(Some(text), LINE_WIDTH - BLOCK_INDENT - LABEL_WIDTH);
    for (i, line) in lines.iter().enumerate() {
        let label = if i == 0 { label } else { "" };
        out.line(format!("{indent}{label:<LABEL_WIDTH$}{line}"));
    }
}

/// A paragraph at the header's indent, wrapped to the same line.
fn paragraph(out: &mut Printed, text: &str) {
    for line in wrap(Some(text), LINE_WIDTH - 2) {
        out.line(format!("  {line}"));
    }
}

/// A share as a whole percentage. `+ 0.0` turns a `-0.0` into `0.0`, so a Wilson bound computed as
/// a negative zero cannot print as `-0%`.
fn percent(share: f64) -> String {
    format!("{}%", pyfmt::fixed(share * 100.0 + 0.0, 0))
}

/// "a, b and c", the way a sentence lists names.
fn listed(names: &[&str]) -> String {
    match names {
        [] => String::new(),
        [one] => (*one).to_string(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

/// A topic's name as a reader says it.
fn topic_name(topic: Topic) -> String {
    topic.as_str().replace('_', " ")
}

/// A `METRIC_TARGETS` row's tolerance, which `contracts::strike` names rather than copies.
fn tolerance_of(row: &str) -> f64 {
    target_for(row)
        .unwrap_or_else(|| panic!("{row} is a METRIC_TARGETS row, which tests/strike.rs holds"))
        .tolerance
}

// ------------------------------------------------------------------------------------ the topics

/// What a topic's grade is a share of, as a clause after the percentage.
fn criterion(topic: Topic) -> String {
    match topic {
        Topic::StrikeLocation => {
            format!(
                "centered, within {} mm of face center",
                pyfmt::g(CENTERED_RADIUS_MM)
            )
        }
        Topic::Consistency => format!(
            "within {}% of its club's typical carry and line",
            pyfmt::g(CONSISTENCY_FRACTION * 100.0)
        ),
        Topic::LowPoint => "low point ahead of the ball".to_string(),
        Topic::Fade | Topic::Draw | Topic::Straight => format!(
            "{} and finished within {} yd of the line",
            hit(topic),
            pyfmt::g(tolerance_of(ON_LINE_TOLERANCE_ROW))
        ),
    }
}

/// "hit the fade", "hit the draw", "hit it straight".
fn hit(topic: Topic) -> String {
    match topic {
        Topic::Straight => "hit it straight".to_string(),
        shape => format!("hit the {}", shape.as_str()),
    }
}

/// What the "hit the shape" share counts, face to path against the straight tolerance, for either
/// hand: the sign is the golfer's, not the camera's (the M37 plan's finding 5).
fn shape_rule(topic: Topic) -> String {
    let tolerance = pyfmt::g(tolerance_of(STRAIGHT_TOLERANCE_ROW));
    match topic {
        Topic::Fade => format!("{}, face to path beyond +{tolerance} deg", hit(topic)),
        Topic::Draw => format!("{}, face to path beyond -{tolerance} deg", hit(topic)),
        _ => format!("{}, face to path within +/-{tolerance} deg", hit(topic)),
    }
}

/// One share in words: graded with its interval, or withheld with what it waits for.
fn share_text(share: &Share, what: &str) -> String {
    match share {
        Share::Graded(rate) => format!(
            "{} {what} ({} of {}), 95% CI {} .. {}",
            percent(rate.share),
            rate.successes,
            rate.n,
            percent(rate.interval.low),
            percent(rate.interval.high)
        ),
        Share::Withheld(withheld) => format!(
            "withheld, {what}: {} of the {} shots a share needs",
            withheld.have_n, withheld.need_n
        ),
    }
}

/// Shots a topic could not judge, each reason in its own clause.
fn unscored_rows(out: &mut Printed, counts: &[UnscoredCount], what: &str) {
    for count in counts {
        row(
            out,
            "",
            "",
            &format!(
                "{} {what}: {}",
                counted(count.n, "shot"),
                spec_for(count.reason).summary
            ),
        );
    }
}

/// One topic's lines: its grade, and for a shape the other share and where the landings came from.
fn report_topic(out: &mut Printed, label: &str, grade: &TopicGrade) {
    let name = topic_name(grade.topic);
    // The grade's own `n` is printed, never `judged`: on a shape topic a judged shot with no landing
    // is out of the grade's denominator, and the landing rows below count it.
    row(
        out,
        label,
        &name,
        &share_text(&grade.grade, &criterion(grade.topic)),
    );
    if let Some(shape) = &grade.shape {
        row(
            out,
            "",
            "",
            &share_text(&shape.hit_shape, &shape_rule(grade.topic)),
        );
        let landed = shape.printed + shape.projected;
        if landed > 0 {
            let mut text = format!("landings: {} printed", shape.printed);
            if shape.projected > 0 {
                text += &format!(
                    ", {} {} by the flight model and not printed",
                    shape.projected,
                    OfflineBasis::Projected.as_str()
                );
            }
            row(out, "", "", &text);
        }
        unscored_rows(
            out,
            &shape.landing_unscored,
            "with no landing, so out of the on-line share",
        );
    }
    unscored_rows(out, &grade.unscored, "not judged");
}

/// The topics block, or the one line a club with none of them gets.
fn report_topics(out: &mut Printed, topics: &[TopicGrade]) {
    if topics.is_empty() {
        labelled(
            out,
            "topics",
            "none: these shots print the inputs of no topic, so there is nothing to grade",
        );
        return;
    }
    for (i, grade) in topics.iter().enumerate() {
        report_topic(out, if i == 0 { "topics" } else { "" }, grade);
    }
}

/// The blend, or why there is none. Nothing at all when there are no topics, which the topics block
/// has already said.
fn report_blend(out: &mut Printed, topics: &[TopicGrade], blend: Option<&Blend>) {
    let names = |list: &[Topic]| -> String {
        let names: Vec<String> = list.iter().map(|topic| topic_name(*topic)).collect();
        listed(&names.iter().map(String::as_str).collect::<Vec<_>>())
    };
    match blend {
        Some(blend) => {
            let mut text = format!(
                "{}, the equal-weight mean of {}",
                percent(blend.mean),
                names(&blend.averaged)
            );
            if !blend.excluded.is_empty() {
                text += &format!("; left out while withheld: {}", names(&blend.excluded));
            }
            labelled(out, "blend", &text);
        }
        None if !topics.is_empty() => labelled(
            out,
            "blend",
            "none: no topic is graded yet, and a blend of nothing is not a zero",
        ),
        None => {}
    }
}

// --------------------------------------------------------------------------------- the strike

/// The grid's row names: the three heights, then the row for shots whose height was not printed.
fn grid_row_name(vertical: Option<FaceRow>) -> &'static str {
    match vertical {
        Some(FaceRow::High) => "high",
        Some(FaceRow::Center) => "center",
        Some(FaceRow::Low) => "low",
        None => "no height",
    }
}

/// Where on the face, as the picture a golfer would draw: high at the top, heel to toe across, and
/// the height-not-printed row last. A row prints when any shot could land in it (every height row
/// once a height is printed at all), so an empty cell reads as a zero rather than a gap.
fn report_impacts(out: &mut Printed, impacts: &ImpactMap) {
    let lean = &impacts.heel_toe;
    if lean.n == 0 {
        labelled(out, "impact", "no shot held a face-impact offset");
        return;
    }
    labelled(
        out,
        "impact",
        &format!(
            "{} with an offset, binned at {} mm (+ = toe, + = high)",
            counted(lean.n, "shot"),
            pyfmt::g(CENTERED_RADIUS_MM)
        ),
    );
    let cell = |vertical: Option<FaceRow>, horizontal: FaceColumn| -> i64 {
        impacts
            .grid
            .iter()
            .filter(|cell| cell.vertical == vertical && cell.horizontal == horizontal)
            .map(|cell| cell.n)
            .sum()
    };
    let indent = " ".repeat(BLOCK_INDENT + LABEL_WIDTH);
    out.line(format!(
        "{indent}{:<12}{:>6}{:>8}{:>6}",
        "", "heel", "center", "toe"
    ));
    let mut rows = Vec::new();
    if impacts.grid.iter().any(|cell| cell.vertical.is_some()) {
        rows.extend([
            Some(FaceRow::High),
            Some(FaceRow::Center),
            Some(FaceRow::Low),
        ]);
    }
    if impacts.grid.iter().any(|cell| cell.vertical.is_none()) {
        rows.push(None);
    }
    for vertical in rows {
        out.line(format!(
            "{indent}{:<12}{:>6}{:>8}{:>6}",
            grid_row_name(vertical),
            cell(vertical, FaceColumn::Heel),
            cell(vertical, FaceColumn::Center),
            cell(vertical, FaceColumn::Toe)
        ));
    }

    let mut text = format!(
        "beyond {} mm, {} toward the heel and {} toward the toe",
        pyfmt::g(CENTERED_RADIUS_MM),
        lean.heel,
        lean.toe
    );
    match (lean.mean_offset_mm, &lean.mean_offset_ci) {
        (Some(mean), Some(ci)) => {
            text += &format!(
                "; mean offset {} mm (+ = toe), 95% CI {} .. {}",
                pyfmt::fixed(mean, 1),
                pyfmt::fixed(ci.low, 1),
                pyfmt::fixed(ci.high, 1)
            );
        }
        _ => {
            // The floor the contract holds the mean to, read rather than restated.
            let floor = minimum_n(Topic::StrikeLocation.inputs()[0], BaselineClaim::Center);
            text += &format!("; no mean offset is shown below {floor} shots");
        }
    }
    labelled(out, "lean", &text);
}

/// A spread in words: the SD with its interval and `n`, or the refusal standing in for it.
fn spread_text(
    unit: &str,
    n: i64,
    sd: Option<f64>,
    sd_ci: Option<&Interval>,
    withheld: &[WithheldClaim],
    unavailable: &[String],
) -> String {
    match sd {
        Some(sd) => {
            let mut text = format!("sd {} {unit}", fmt(sd, unit));
            if let Some(ci) = sd_ci {
                text += &format!(", 95% CI {} .. {}", fmt(ci.low, unit), fmt(ci.high, unit));
            }
            text + &format!(", n = {n}")
        }
        None => {
            let reason = withheld
                .iter()
                .find(|claim| claim.claim == BaselineClaim::Spread)
                .map(|claim| claim.reason.clone())
                .or_else(|| unavailable.first().cloned())
                .unwrap_or_else(|| format!("no spread is shown, n = {n}"));
            format!("withheld: {reason}")
        }
    }
}

/// The raw numbers beside the grades: the ball-speed spread and the strike dispersions' spreads,
/// each carried from the club's profile and never recomputed. Their bias and scatter verdicts are
/// the list's to print, so they are not repeated here.
fn report_spread(out: &mut Printed, strike: &StrikeProfile) {
    let mut rows: Vec<(&str, String)> = Vec::new();
    if let Some(baseline) = &strike.ball_speed {
        rows.push((baseline.name.as_str(), baseline_spread(baseline)));
    }
    for (name, dispersion) in &strike.dispersion {
        rows.push((name.as_str(), dispersion_spread(dispersion)));
    }
    for (i, (key, text)) in rows.iter().enumerate() {
        row(out, if i == 0 { "spread" } else { "" }, key, text);
    }
}

fn baseline_spread(baseline: &MetricBaseline) -> String {
    spread_text(
        &baseline.unit,
        baseline.n,
        baseline.sd,
        baseline.sd_ci.as_ref(),
        &baseline.withheld,
        &[],
    )
}

fn dispersion_spread(dispersion: &MetricDispersion) -> String {
    spread_text(
        &dispersion.unit,
        dispersion.n,
        dispersion.sd,
        dispersion.sd_ci.as_ref(),
        &dispersion.withheld,
        &dispersion.unavailable,
    )
}

// ------------------------------------------------------------------------------------ the list

/// One entry, with its ratio in words: how many tolerances a weakness misses by, or how close to
/// failing a strength's nearer bound sits.
fn finding_text(finding: &ProfileFinding) -> String {
    let row = target_for(&finding.metric);
    let tolerance = row.map_or_else(String::new, |row| pyfmt::g(row.tolerance));
    let ratio = pyfmt::fixed(finding.ratio, 2);
    match finding.kind {
        FindingKind::Bias => format!(
            "weakness, bias: the mean is off target by {ratio} times the tolerance of {tolerance}"
        ),
        FindingKind::Scatter => {
            format!("weakness, scatter: the sd is {ratio} times the tolerance of {tolerance}")
        }
        FindingKind::Strength => {
            let target = row
                .and_then(|row| row.target)
                .map_or_else(String::new, pyfmt::g);
            format!(
                "strength: established inside {target} +/- {tolerance}, its nearer bound at \
                 {ratio} of the tolerance"
            )
        }
    }
}

fn report_list(out: &mut Printed, list: &[ProfileFinding]) {
    if list.is_empty() {
        labelled(out, "list", "nothing is established either way yet");
        return;
    }
    for (i, finding) in list.iter().enumerate() {
        row(
            out,
            if i == 0 { "list" } else { "" },
            &finding.metric,
            &finding_text(finding),
        );
    }
}

// ------------------------------------------------------------------------------------ one block

/// The strike profile, the topics, the blend and the list: a club's block and the player's alike.
fn report_body(
    out: &mut Printed,
    strike: &StrikeProfile,
    topics: &[TopicGrade],
    blend: Option<&Blend>,
    list: &[ProfileFinding],
) {
    if let Some(impacts) = &strike.impacts {
        report_impacts(out, impacts);
    }
    report_spread(out, strike);
    report_topics(out, topics);
    report_blend(out, topics, blend);
    report_list(out, list);
}

fn report_club(out: &mut Printed, club: &GradedClub) {
    let grades = &club.grades;
    out.line(format!(
        "\n  {}  ({})",
        grades.club.as_str(),
        category_of(grades.club).as_str().replace('_', " ")
    ));
    labelled(out, "shots", &counted(grades.n_shots, "tracked shot photo"));
    report_body(
        out,
        &grades.strike,
        &grades.topics,
        grades.blend.as_ref(),
        &club.list,
    );
}

fn report_player(out: &mut Printed, profile: &GradedProfile) {
    let grades = &profile.player.grades;
    out.line("\n  all clubs  (each shot judged by its own club's criterion)");
    let mut text = format!(
        "{} across the bag",
        counted(grades.n_shots, "tracked shot photo")
    );
    if grades.untagged_shots > 0 {
        text += &format!(
            ", {} naming no club, which count toward strike location and the shapes, whose \
             criteria need no club, and toward nothing else",
            grades.untagged_shots
        );
    }
    labelled(out, "shots", &text);
    report_body(
        out,
        &grades.strike,
        &grades.topics,
        grades.blend.as_ref(),
        &profile.player.list,
    );
}

// ---------------------------------------------------------------------------------- one golfer

/// The report `golf-core profile` prints for one golfer: every graded club in bag order, then the
/// player's grades, or with `club` that club's block alone. The verb's trailing blank line is the
/// verb's, as for the other reports.
pub fn profile(profile: &GradedProfile, display_name: &str, club: Option<ClubId>) -> String {
    let mut out = Printed::default();
    header(&mut out, display_name, &profile.player_id);
    paragraph(
        &mut out,
        &format!(
            "Each grade is the share of tracked shots that met its topic, with a 95% interval, \
             and is withheld below {RATE_MINIMUM_N} judged shots. Launch-monitor numbers only: \
             mechanics is not graded here."
        ),
    );

    if let Some(club) = club {
        match profile
            .clubs
            .iter()
            .find(|graded| graded.grades.club == club)
        {
            Some(graded) => report_club(&mut out, graded),
            None => out.line(format!(
                "\n  {}: no tracked shot photo of this club, so there is nothing to grade.",
                club.as_str()
            )),
        }
        return out.into_string();
    }

    let player = &profile.player.grades;
    if profile.clubs.is_empty() && player.n_shots == 0 {
        out.line("\n  No tracked shot photo on record, so there is nothing to grade yet.");
        return out.into_string();
    }
    if profile.clubs.is_empty() {
        out.line("\n  No tracked shot photo names a club, so only the player's grades follow.");
    }
    for graded in &profile.clubs {
        report_club(&mut out, graded);
    }
    report_player(&mut out, profile);
    out.into_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_percentage_is_whole_and_never_negative_zero() {
        assert_eq!(percent(0.5833333333333334), "58%");
        assert_eq!(percent(1.0), "100%");
        assert_eq!(percent(-0.0), "0%");
        // Half-even on an exact half, as every report here rounds.
        assert_eq!(percent(0.125), "12%");
    }

    #[test]
    fn names_are_listed_as_a_sentence_lists_them() {
        assert_eq!(listed(&[]), "");
        assert_eq!(listed(&["fade"]), "fade");
        assert_eq!(listed(&["fade", "draw"]), "fade and draw");
        assert_eq!(
            listed(&["consistency", "fade", "draw"]),
            "consistency, fade and draw"
        );
    }

    /// The constants a criterion quotes are read, so the words move when the rows do.
    #[test]
    fn each_criterion_quotes_its_constant() {
        assert_eq!(
            criterion(Topic::StrikeLocation),
            "centered, within 10 mm of face center"
        );
        assert_eq!(
            criterion(Topic::Consistency),
            "within 10% of its club's typical carry and line"
        );
        assert_eq!(
            criterion(Topic::Straight),
            "hit it straight and finished within 9 yd of the line"
        );
        assert_eq!(
            shape_rule(Topic::Draw),
            "hit the draw, face to path beyond -2 deg"
        );
    }
}
