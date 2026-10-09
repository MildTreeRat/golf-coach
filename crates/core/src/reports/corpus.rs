//! `scripts/career_corpus.py`'s report: the honest-`n` counter for one golfer's career corpus.
//! [M36 P15]
//!
//! What `read_corpus` found: how many swing directories were read, how many of them are actually
//! distinct swings, and the sample size behind each metric. **The gap between those first two
//! numbers is the reason the report exists**: a baseline built by counting directories would report
//! a golfer's numbers as many times as their clips were re-uploaded, and repeating one measurement
//! does not just inflate `n`, it drives the variance toward zero, which is the quantity career mode
//! is built to read.
//!
//! # The outdated paragraph, under `COMPARABLE_FROM`
//!
//! The script prints `N distinct swing(s) analyzed by an older engine (version …; M is installed)`
//! with its own `ANALYSIS_VERSION` as `M`. Here `M` is the caller's `installed`: the career vector's
//! `input.versions.installed` in the family (16 on every Python-recorded case), and
//! `contracts::swing::ANALYSIS_VERSION` in the verb. **The sentence stays frozen Python's** after
//! `career_version` 1 (M36 P14) made the corpus exclude below `COMPARABLE_FROM` instead, because it
//! is still true of every swing it lists: an excluded swing is older than the oldest comparable
//! engine, so older than the installed one too, and "not comparable with a swing analyzed today" is
//! now the rule itself rather than an approximation of it. Each exclusion's own `detail`, printed by
//! `--verbose`, names the line it fell under.

use std::collections::{BTreeMap, BTreeSet};

use contracts::career::CareerCorpus;

use super::{header, plural, Printed};

/// The report `career_corpus._report(corpus, display_name, verbose=…)` prints. `installed` is the
/// engine generation the outdated paragraph names (the module doc).
pub fn career_corpus(
    corpus: &CareerCorpus,
    display_name: &str,
    installed: i64,
    verbose: bool,
) -> String {
    let mut out = Printed::default();
    header(&mut out, display_name, &corpus.player_id);
    let dirs = corpus.swing_dirs_seen;
    let sessions = corpus.sessions_scanned;
    let swings = corpus.distinct_swings() as i64;
    let shots = corpus.distinct_shots() as i64;
    out.line(format!(
        "  {dirs} swing director{} across {sessions} session{}  ->  {swings} distinct swing{}, \
         {shots} distinct shot{}",
        if dirs == 1 { "y" } else { "ies" },
        plural(sessions),
        plural(swings),
        plural(shots),
    ));

    let collapsed = corpus.duplicates_collapsed();
    if collapsed > 0 {
        out.line(format!("\n  Collapsed {collapsed} re-upload(s):"));
        for swing in corpus.swings.iter().filter(|s| !s.duplicates.is_empty()) {
            out.line(format!(
                "    face_on {}  kept {}  <- {}",
                prefix(&swing.face_on_sha256),
                swing.swing_ref(),
                swing.duplicates.join(", ")
            ));
        }
    }

    let conflicts = corpus.shot_conflicts();
    if conflicts > 0 {
        out.line(format!(
            "\n  {conflicts} swing(s) whose re-uploads disagree on the shot photo:"
        ));
        for swing in corpus
            .swings
            .iter()
            .filter(|s| !s.conflicting_shots.is_empty())
        {
            let others: Vec<&str> = swing.conflicting_shots.iter().map(|s| prefix(s)).collect();
            // `(swing.shot_sha256 or "none")[:12]`: falsy, so an empty hash prints "none" too.
            let kept = match swing.shot_sha256.as_deref() {
                Some(sha) if !sha.is_empty() => prefix(sha),
                _ => "none",
            };
            out.line(format!(
                "    {}  kept {kept}, also saw {}",
                swing.swing_ref(),
                others.join(", ")
            ));
        }
        out.line("    One swing has one ball flight, so one of these is misattached. Not counted.");
    }

    out.line("\n  Honest n per metric:");
    if corpus.metric_counts.is_empty() {
        out.line("    (none — no distinct swing carries a measurement yet)");
    } else {
        let width = corpus
            .metric_counts
            .keys()
            .map(|name| name.chars().count())
            .max()
            .unwrap_or_default();
        for (name, count) in &corpus.metric_counts {
            out.line(format!("    {name:<width$}  n = {count}"));
        }
    }

    if corpus.outdated_swings != 0 {
        let versions: BTreeSet<i64> = corpus
            .swings
            .iter()
            .filter(|s| s.outdated)
            .map(|s| s.analysis_version)
            .collect();
        let seen: Vec<String> = versions.iter().map(i64::to_string).collect();
        out.line(format!(
            "\n  {} distinct swing(s) analyzed by an older engine (version {}; {installed} is \
             installed).",
            corpus.outdated_swings,
            seen.join(", ")
        ));
        out.line("    Their numbers are not comparable with a swing analyzed today, so they are");
        out.line("    reported rather than pooled. Repair with: python scripts/reanalyze.py");
    }

    if corpus.analyzed_without_measurements != 0 {
        out.line(format!(
            "\n  {} current-engine swing(s) carrying no measurement at all.",
            corpus.analyzed_without_measurements
        ));
        out.line("    Every metric returned None on them — a measurement failure, not an old");
        out.line("    artifact. Their checkpoint `observed` values are deliberately not read as");
        out.line("    measurements: it would mix two derivation paths under one metric name.");
    }

    let untagged = corpus.untagged_swings();
    if untagged > 0 {
        out.line(format!(
            "\n  {untagged} distinct swing(s) naming no club (of {swings})."
        ));
        out.line("    They still count toward every metric above and toward every whole-bag");
        out.line("    number — the club was never an input to measuring head sway — and are");
        out.line("    absent only from per-club views. Repair them one at a time on the upload");
        out.line("    page: a session has many clubs, so there is no bulk backfill (ADR-024 §5).");
    }

    if !corpus.unknown_sources.is_empty() {
        out.line(format!(
            "\n  Unrecognised measurement sources: {}",
            corpus.unknown_sources.join(", ")
        ));
        out.line(
            "    Counted per swing rather than per artifact — check the dedupe key still fits.",
        );
    }

    if !corpus.excluded.is_empty() {
        out.line(format!(
            "\n  Contributing no sample ({}):",
            corpus.excluded.len()
        ));
        // `sorted(by_reason.items())`: by the reason's value, which is ASCII, so byte order is
        // Python's code-point order; each reason's refs in the corpus's own order.
        let mut by_reason: BTreeMap<&str, Vec<String>> = BTreeMap::new();
        for entry in &corpus.excluded {
            by_reason
                .entry(entry.reason.as_str())
                .or_default()
                .push(entry.swing_ref());
        }
        for (reason, refs) in &by_reason {
            out.line(format!(
                "    {reason:<14} {:>3}  {}",
                refs.len(),
                refs.join(", ")
            ));
        }
        if verbose {
            out.line("");
            for entry in &corpus.excluded {
                out.line(format!(
                    "    {}: {} — {}",
                    entry.swing_ref(),
                    entry.reason.as_str(),
                    entry.detail
                ));
            }
        }
    }

    if corpus.other_golfers != 0 {
        out.line(format!(
            "\n  ({} swing(s) on disk belong to someone else.)",
            corpus.other_golfers
        ));
    }
    out.into_string()
}

/// `sha[:12]`, by code point.
fn prefix(sha: &str) -> &str {
    sha.char_indices()
        .nth(12)
        .map_or(sha, |(cut, _)| &sha[..cut])
}

#[cfg(test)]
mod tests {
    use super::prefix;

    #[test]
    fn a_prefix_is_twelve_code_points_or_the_whole() {
        assert_eq!(prefix("0123456789abcdef"), "0123456789ab");
        assert_eq!(prefix("short"), "short");
        assert_eq!(prefix("ééééééééééééé"), "éééééééééééé");
    }
}
