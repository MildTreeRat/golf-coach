//! The bag, in the shapes an MCP tool returns. `src/golf_coach/mcp/club.py`. [M29 P13]
//!
//! [`crate::career`]'s sibling, one cut further in: that module answers "what does this golfer
//! usually do", this one "what does this golfer do **with this club**". Everything underneath is
//! already built and sealed — `analysis::club_profile::build_bag_profile` narrows the corpus per
//! club and hands each slice to career mode's guard — so nothing here averages, gates or judges.
//! What is here is the wire shape, the two misses, and the notes that say what a silence means.
//!
//! # Five silences, and they need different answers
//!
//! From a distance all five read as "no numbers", and a model handed the wrong one gives a confident
//! wrong instruction, so each is its own sentence (pinned below, as `test_club_tools.py` pins it):
//!
//! 1. **No club profiles at all** ([`NOTHING_TAGGED`]): nothing on disk names a club, and the fix
//!    is tagging, not hitting balls.
//! 2. **A club whose every figure is withheld** ([`not_enough_on_this_club`]): the guard wants more
//!    shots on that club, and hitting balls is the fix.
//! 3. **A club never hit and not in the bag** ([`NEVER_HIT`]): an answer, not a miss.
//! 4. **A club in the bag never hit** ([`IN_BAG_NEVER_HIT`]): the state of every club the day it is
//!    declared, which "withheld" would misreport as a refusal.
//! 5. **A club whose swings carry no measurement** ([`NO_MEASUREMENTS`]): not about the club.
//!
//! # Why [`ClubMetric`] is not `career::MetricProfile`
//!
//! `MetricProfile` carries the tour join, and there is no per-club tour join to put in it:
//! `ranges.json` holds `club_category: "all"` rows only (ADR-024's second trap). Leaving those fields
//! at their defaults would ship `tour_standing: "withheld"` on every metric of every club, a refusal
//! of a claim nobody could make. So the baseline and dispersion halves are mirrored field for field
//! and the tour block is not here, which is Python's call and its reason.
//!
//! # The corpus is Rust's, as the career tools' is
//!
//! Read through [`career::versions`], so these views count what [`crate::career`]'s count: a swing
//! is outdated only below `COMPARABLE_FROM`, a photo-only entry is admitted (so it is one of a
//! club's `n_swings`, and one of its `n_shots`), and a drill is in no count. No recorded case
//! reaches any of the three (every recorded tree was written by frozen Python), so no recorded club
//! answer moved.
//!
//! **A drill is as if deleted from disk here, and no sentence is reworded for it**, unlike the two
//! of `crate::career`'s that name it. ADR-024's M35 addendum is that every aggregate and every report
//! over a corpus holding a drill equals the one with the drill deleted, and `golf-core club-profile`
//! prints a club hit only in drills as "never hit and not in the bag" under that rule; these notes
//! follow it. The career sentences were different in kind: each enumerates every reason a sample can
//! be missing, so leaving the drill out made them deny one. A per-club drill sentence would also
//! need the club of an excluded swing, which `ExcludedSwing` does not carry.
//!
//! Neither this nor [`crate::career`] touches the protocol; [`crate::tools`] puts a view on the wire.

use std::path::Path;

use analysis::club_profile::build_bag_profile;
use contracts::bag::BagEntry;
use contracts::baseline::MetricBaseline;
use contracts::club::{category_of, parse_club, ClubId};
use contracts::club_profile::{BagProfile, ClubProfile};
use contracts::dispersion::MetricDispersion;
use serde::Serialize;
use storage::bag_store::BagStore;
use storage::corpus::read_corpus;

use crate::career::{self, Refusal, SessionPoint, THE_UNBLOCK};
use crate::query::{self, NotFound};

/// Silence (1). Repeated in the payload rather than left to `READING_A_BAG`, because a briefing is
/// read once on connect and a note is read at the moment the answer is being written.
///
/// **Its last clause is Rust's** [M29 P32]: frozen Python's ends on [`NOTHING_TAGGED_FROZEN_TAIL`],
/// the frozen server's retag control and route, which the M29 plan's D11 closed; this ends on
/// [`NOTHING_TAGGED_TAIL`], the verb whose name that route's refusal gives. Declared at
/// `get_bag_profile`'s `note` in `crate::family`, as P9's precedent for `flag_mishit.py` is.
pub const NOTHING_TAGGED: &str = "No swing on record names a club, so there is nothing to profile \
                                  per club yet. This is NOT a sample-size refusal and a bay session \
                                  does not fix it on its own — the club tag is newer than these \
                                  swings, and they stay uncountable per club until each one is \
                                  tagged. Do not answer this by telling the golfer to hit more \
                                  balls. New swings get a club at upload; a swing already stored is \
                                  retagged one at a time, with golf-core set-club --session \
                                  <session> --swing <swing> --club <club>.";

/// How [`NOTHING_TAGGED`] ends in frozen Python's `club.py`, and in every case it recorded.
pub const NOTHING_TAGGED_FROZEN_TAIL: &str = "from the change control on its row in the upload \
                                              page's swing list or with POST \
                                              /api/sessions/<session>/swings/<swing>/club.";

/// How [`NOTHING_TAGGED`] ends here.
pub const NOTHING_TAGGED_TAIL: &str =
    "with golf-core set-club --session <session> --swing <swing> --club <club>.";

/// Silence (2)'s first half, the part only true per club. [`not_enough_on_this_club`] puts
/// [`THE_UNBLOCK`] after it, as Python's constant concatenates `career.THE_UNBLOCK`: the floors are
/// career mode's own, not a second set.
pub const NOT_ENOUGH_HEAD: &str = "Every claim about this club is withheld. The floors are the \
                                   same ones the whole-bag history faces, applied to this club's \
                                   share of it, so a bag hit across a dozen clubs reaches them far \
                                   more slowly than the bag as a whole does.";

/// Silence (2), whole: `NOT_ENOUGH_ON_THIS_CLUB`. A function because a `const` cannot concatenate
/// another `const`, and a second copy of the unblock sentence is a second thing that drifts.
pub fn not_enough_on_this_club() -> String {
    format!("{NOT_ENOUGH_HEAD} {THE_UNBLOCK}")
}

/// Silence (3). Deliberately not a [`NotFound`]: the club is real, the golfer is real, and "you
/// have no record of hitting it" is the answer. [`BagProfile::profile_for`] answering `None` is what
/// this reads off.
pub const NEVER_HIT: &str = "No swing on record was hit with this club and it is not in the \
                             declared bag, so there is no history to report and nothing has been \
                             refused. That is an answer rather than a miss. If the golfer does hit \
                             this club, tagging the swing is what puts it here.";

/// Silence (4), and the reason it is not (2): a club declared today has refused nothing.
pub const IN_BAG_NEVER_HIT: &str = "This club is in the bag and nothing has been hit with it yet, \
                                    so there are no statistics and nothing has been refused. That \
                                    is the state every club is in the day it is declared, and the \
                                    profile exists so the club is visible before it has any \
                                    history.";

/// Silence (5). Points at `get_golfer_profile` rather than a CLI, because a model has the tool and
/// not the shell.
pub const NO_MEASUREMENTS: &str = "These swings carry no measurement at all, so there is nothing \
                                   here to refuse and nothing to report. That is not about the \
                                   club: the commonest causes are swings that were never analyzed \
                                   and swings analyzed by an older engine. Call get_golfer_profile \
                                   to see which metrics have samples at all.";

/// Appended whenever untagged swings sit beside a bag that does have clubs in it: the partial case
/// of (1), because a bag looks complete when most of a golfer's swings are simply missing from it.
pub const SOME_SWINGS_UNTAGGED: &str = "Some swings on record name no club (see \
                                        `untagged_swings`), so nothing above counts them. They are \
                                        not lost — every whole-bag figure get_golfer_profile \
                                        reports still includes them — but no per-club number can \
                                        be cut from them until each one is tagged.";

// -------------------------------------------------------------------------------------------------
// The views, each in its pydantic model's field order
// -------------------------------------------------------------------------------------------------

/// One metric for one club: what its history supports, and every refusal in the gaps. The baseline
/// and dispersion halves of `career::MetricProfile`, and no tour block (the module doc).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ClubMetric {
    pub name: String,
    pub unit: String,
    pub source: String,
    /// Distinct contributing artifacts for this club, never the bag's.
    pub n: i64,
    pub n_sessions: i64,

    // What this club's own history says (`contracts::baseline`).
    pub center: Option<f64>,
    pub center_ci_low: Option<f64>,
    pub center_ci_high: Option<f64>,
    pub median: Option<f64>,
    pub sd: Option<f64>,
    pub sd_ci_low: Option<f64>,
    pub sd_ci_high: Option<f64>,
    pub minimum: Option<f64>,
    pub maximum: Option<f64>,

    // What the shape of the miss points at (`contracts::dispersion`).
    pub bias: String,
    pub scatter: String,
    pub pattern: Option<String>,
    pub points_at: Option<String>,
    pub target: Option<f64>,
    pub tolerance: Option<f64>,
    pub within_session_sd: Option<f64>,

    pub sessions: Vec<SessionPoint>,
    pub caveats: Vec<String>,
    /// Refused for want of data. More shots on this club fix these.
    pub withheld: Vec<Refusal>,
    /// Refused for want of a reference. More shots do not fix these.
    pub unavailable: Vec<String>,
}

/// One club of one golfer's: the physical club, the evidence, and what it supports saying.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ClubView {
    pub player_id: String,
    /// The canonical club id.
    pub club: String,
    /// Derived from the club through `category_of`, never stored beside it.
    pub category: String,
    pub in_bag: bool,
    /// The declared club in this slot, written flat as Python's subclass is.
    pub bag_entry: Option<BagEntry>,
    pub n_swings: i64,
    /// The ceiling on every distance and launch figure.
    pub n_shots: i64,
    pub n_sessions: i64,
    pub mishits: i64,
    pub mishit_refs: Vec<String>,
    pub mishits_unconfirmed: i64,
    pub metrics: Vec<ClubMetric>,
    pub caveats: Vec<String>,
    /// True when the guard refused every claim on every metric.
    pub nothing_sayable: bool,
    /// Which silence this is, or empty.
    pub note: String,
}

/// Every club one golfer has hit or declared, in bag order.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BagView {
    pub player_id: String,
    pub display_name: String,
    pub clubs: Vec<ClubView>,
    pub clubs_used: usize,
    pub clubs_declared: usize,
    pub untagged_swings: i64,
    pub mishits_excluded: i64,
    /// True when no club has been hit or declared at all, which is not a sample-size refusal.
    pub nothing_tagged: bool,
    pub note: String,
}

// -------------------------------------------------------------------------------------------------
// The misses
// -------------------------------------------------------------------------------------------------
//
// The golfer misses mirror `crate::career`'s, with a hint of their own each, and the club miss is
// this module's.

/// `missing_golfer_bag`.
pub fn missing_golfer_bag(player: &str) -> NotFound {
    query::missing(
        format!(
            "No golfer matching {} is registered.",
            pyfmt::str_repr(player)
        ),
        "Names are matched loosely (case and punctuation are folded), so this means nobody by \
         that name has swings on file yet. Call get_golfer_profile to see who is.",
    )
}

/// `missing_golfer_club`.
pub fn missing_golfer_club(player: &str) -> NotFound {
    query::missing(
        format!(
            "No golfer matching {} is registered.",
            pyfmt::str_repr(player)
        ),
        "Call get_bag_profile with the name to see whether they have a bag on file at all.",
    )
}

/// `unknown_club`: not a club, refused rather than resolved to the nearest thing, which is
/// `parse_club`'s whole posture (ADR-024 §5).
pub fn unknown_club(text: &str) -> NotFound {
    query::missing(
        format!("{} is not a club.", pyfmt::str_repr(text)),
        "Clubs are named like '7i', '7 iron', 'seven iron', 'driver' or 'pw'. Note that 'wedge' \
         and 'iron' name a category rather than a club, so neither is accepted: ask which one, or \
         call get_bag_profile for every club this golfer has.",
    )
}

// -------------------------------------------------------------------------------------------------
// The two readers
// -------------------------------------------------------------------------------------------------

/// `bag_profile`: one golfer's whole bag, club by club. `None` if the golfer is unknown.
///
/// Every club is narrowed out of one corpus by `build_bag_profile`, so each club's counts describe
/// that club's swings.
pub fn bag_profile(sessions_dir: &Path, golfers_dir: &Path, player: &str) -> Option<BagView> {
    let golfer = career::resolve_golfer(golfers_dir, player)?;
    let profile = bag(sessions_dir, golfers_dir, &golfer.player_id);
    let clubs: Vec<ClubView> = profile
        .clubs
        .iter()
        .map(|one| club_view(&golfer.player_id, one))
        .collect();
    let note = bag_note(&clubs, profile.untagged_swings);
    Some(BagView {
        player_id: golfer.player_id,
        display_name: golfer.display_name,
        clubs_used: profile.clubs_used().len(),
        clubs_declared: profile.clubs_declared().len(),
        untagged_swings: profile.untagged_swings,
        mishits_excluded: profile.mishits_excluded,
        nothing_tagged: clubs.is_empty(),
        note,
        clubs,
    })
}

/// `club_profile`: one club of one golfer's. `None` is an unknown **golfer**, whose hint differs
/// per tool, so the body builds that miss; `Err` is an unknown **club**, whose hint does not.
///
/// The golfer is resolved before the club is parsed, as Python's is, so a bad name and a bad club
/// together answer the golfer's miss. A club that parses with neither history nor a bag entry is
/// neither miss: it is an answer carrying [`NEVER_HIT`].
pub fn club_profile(
    sessions_dir: &Path,
    golfers_dir: &Path,
    player: &str,
    club: &str,
) -> Option<Result<ClubView, NotFound>> {
    let golfer = career::resolve_golfer(golfers_dir, player)?;
    let Some(parsed) = parse_club(club) else {
        return Some(Err(unknown_club(club)));
    };
    let profile = bag(sessions_dir, golfers_dir, &golfer.player_id);
    Some(Ok(match profile.profile_for(parsed) {
        Some(one) => club_view(&golfer.player_id, one),
        None => never_hit(&golfer.player_id, parsed),
    }))
}

// -------------------------------------------------------------------------------------------------
// Internals
// -------------------------------------------------------------------------------------------------

/// `_bag`: the corpus, the bag and the builder, as `golf-core club-profile` runs them. The bag lives
/// in the golfer directory, so there is no second path; a golfer with no declared bag reads as
/// `None`, which costs the loft and nothing else.
fn bag(sessions_dir: &Path, golfers_dir: &Path, player_id: &str) -> BagProfile {
    let corpus = read_corpus(sessions_dir, player_id, career::versions());
    build_bag_profile(&corpus, BagStore::new(golfers_dir).get(player_id).as_ref())
}

/// `_club_view`. The metrics come off a map sorted by name, which is Python's `sorted(items())`.
fn club_view(player_id: &str, profile: &ClubProfile) -> ClubView {
    let metrics = profile
        .metrics
        .iter()
        .map(|(name, baseline)| metric(baseline, profile.dispersion.get(name)))
        .collect();
    // `PersonalBaseline.nothing_sayable`'s own rule over this club's metrics, derived rather than
    // restated so the two surfaces cannot disagree about what "ready" means.
    let nothing_sayable = !profile
        .metrics
        .values()
        .any(|baseline| !baseline.ready.is_empty());
    ClubView {
        player_id: player_id.to_string(),
        club: profile.club.as_str().to_string(),
        category: profile.category().as_str().to_string(),
        in_bag: profile.in_bag,
        bag_entry: profile.bag_entry.clone(),
        n_swings: profile.n_swings,
        n_shots: profile.n_shots,
        n_sessions: profile.n_sessions,
        mishits: profile.mishits,
        mishit_refs: profile.mishit_refs.clone(),
        mishits_unconfirmed: profile.mishits_unconfirmed,
        metrics,
        caveats: profile.caveats.clone(),
        nothing_sayable,
        note: club_note(profile, nothing_sayable),
    }
}

/// `_club_note`: which silence this club is in, or none. **"No swings" is asked first**, because a
/// club with none has refused nothing, and `nothing_sayable` is true there and means something else;
/// reading it first is how a newly declared club gets reported as a refusal.
fn club_note(profile: &ClubProfile, nothing_sayable: bool) -> String {
    if profile.n_swings == 0 {
        IN_BAG_NEVER_HIT.to_string()
    } else if profile.metrics.is_empty() {
        NO_MEASUREMENTS.to_string()
    } else if nothing_sayable {
        not_enough_on_this_club()
    } else {
        String::new()
    }
}

/// `_bag_note`: the bag-level silence. An empty list and a list of refusals are the same shape and
/// opposite instructions, and both can apply beside untagged swings, so the untagged sentence is
/// appended to the unblock rather than replacing it.
fn bag_note(clubs: &[ClubView], untagged: i64) -> String {
    if clubs.is_empty() {
        return NOTHING_TAGGED.to_string();
    }
    let note = if clubs.iter().all(|one| one.nothing_sayable) {
        THE_UNBLOCK
    } else {
        ""
    };
    match (untagged != 0, note.is_empty()) {
        (false, _) => note.to_string(),
        (true, true) => SOME_SWINGS_UNTAGGED.to_string(),
        (true, false) => format!("{note} {SOME_SWINGS_UNTAGGED}"),
    }
}

/// `_never_hit`: a real club with no record of it. The category through `category_of`, the one
/// mapping, as `ClubProfile::category` derives it; there is simply no profile to ask.
fn never_hit(player_id: &str, club: ClubId) -> ClubView {
    ClubView {
        player_id: player_id.to_string(),
        club: club.as_str().to_string(),
        category: category_of(club).as_str().to_string(),
        in_bag: false,
        bag_entry: None,
        n_swings: 0,
        n_shots: 0,
        n_sessions: 0,
        mishits: 0,
        mishit_refs: Vec::new(),
        mishits_unconfirmed: 0,
        metrics: Vec::new(),
        caveats: Vec::new(),
        nothing_sayable: true,
        note: NEVER_HIT.to_string(),
    }
}

/// `_metric`: two layers about one metric, flattened. `withheld` is the baseline's alone, because
/// the dispersion layer's is a filter of it and carrying both would say each thing twice;
/// `unavailable` is the dispersion layer's deduplicated keeping the first, as `dict.fromkeys` does.
fn metric(baseline: &MetricBaseline, dispersion: Option<&MetricDispersion>) -> ClubMetric {
    let mut unavailable: Vec<String> = Vec::new();
    for reason in dispersion
        .map(|d| d.unavailable.as_slice())
        .unwrap_or_default()
    {
        if !unavailable.contains(reason) {
            unavailable.push(reason.clone());
        }
    }
    ClubMetric {
        name: baseline.name.clone(),
        unit: baseline.unit.clone(),
        source: baseline.source.clone(),
        n: baseline.n,
        n_sessions: baseline.n_sessions,
        center: baseline.mean,
        center_ci_low: career::low(baseline.mean_ci.as_ref()),
        center_ci_high: career::high(baseline.mean_ci.as_ref()),
        median: baseline.median,
        sd: baseline.sd,
        sd_ci_low: career::low(baseline.sd_ci.as_ref()),
        sd_ci_high: career::high(baseline.sd_ci.as_ref()),
        minimum: baseline.minimum,
        maximum: baseline.maximum,
        bias: dispersion
            .map_or("withheld", |d| d.bias.as_str())
            .to_string(),
        scatter: dispersion
            .map_or("withheld", |d| d.scatter.as_str())
            .to_string(),
        pattern: dispersion
            .and_then(|d| d.pattern)
            .map(|p| p.as_str().to_string()),
        points_at: dispersion.and_then(|d| d.points_at.clone()),
        target: dispersion.and_then(|d| d.target),
        tolerance: dispersion.and_then(|d| d.tolerance),
        within_session_sd: dispersion.and_then(|d| d.within_session_sd),
        sessions: career::points(baseline),
        caveats: dispersion.map(|d| d.caveats.clone()).unwrap_or_default(),
        withheld: baseline.withheld.iter().map(career::refusal).collect(),
        unavailable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    /// `test_every_silence_is_its_own_sentence`: five states that look alike in a payload, plus the
    /// partial untagged one, and two collapsing into one string would leave every recorded case
    /// green and the tools wrong.
    #[test]
    fn every_silence_is_its_own_sentence() {
        let not_enough = not_enough_on_this_club();
        let notes: BTreeSet<&str> = [
            NOTHING_TAGGED,
            not_enough.as_str(),
            NEVER_HIT,
            IN_BAG_NEVER_HIT,
            NO_MEASUREMENTS,
            SOME_SWINGS_UNTAGGED,
        ]
        .into_iter()
        .collect();
        assert_eq!(notes.len(), 6);
        assert!(not_enough.ends_with(THE_UNBLOCK));
        assert!(NOTHING_TAGGED.ends_with(NOTHING_TAGGED_TAIL));
        assert!(!NOTHING_TAGGED.contains("/api/"), "it names a closed route");
        for note in notes {
            assert!(!note.contains("  ") && !note.ends_with(' '), "{note}");
        }
    }

    /// The bag note's four arms: an empty bag, a bag that says something, one refusing everything,
    /// and the untagged sentence after either of the last two.
    #[test]
    fn the_bag_note_appends_the_untagged_sentence_to_the_unblock() {
        let view = |sayable: bool| ClubView {
            nothing_sayable: !sayable,
            ..never_hit("aaron", ClubId::SevenIron)
        };
        assert_eq!(bag_note(&[], 3), NOTHING_TAGGED);
        assert_eq!(bag_note(&[view(true), view(false)], 0), "");
        assert_eq!(bag_note(&[view(false)], 0), THE_UNBLOCK);
        assert_eq!(bag_note(&[view(true)], 2), SOME_SWINGS_UNTAGGED);
        assert_eq!(
            bag_note(&[view(false)], 2),
            format!("{THE_UNBLOCK} {SOME_SWINGS_UNTAGGED}")
        );
    }
}
