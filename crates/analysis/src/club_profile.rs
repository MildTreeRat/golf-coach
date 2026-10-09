//! One golfer's history, cut by club. `analysis/club_profile.py`. [M9 P15; M36 P12]
//!
//! [`contracts::club_profile`] declared the shape a per-club answer comes back in. This fills it, and
//! **it is deliberately short**: every statistic was already sealed by the step allowed to make it
//! (the guard in [`crate::baseline`], the discriminator in [`crate::dispersion`], the honest counting
//! in `crates/storage`'s reader). What is left is the assembly, and if this module ever grows past
//! that, something downstream is being reimplemented one layer up.
//!
//! # Narrowing first is the whole correctness argument
//!
//! Every club's numbers come from [`CareerCorpus::narrowed_to`] and never from the corpus itself.
//! Handing [`build_baseline`] the whole bag would let a club hit twice clear the five-sample floor on
//! the *bag's* `n`: a confident mean carry for a 7 iron, built out of eight driver shots. That is the
//! printed-`n`-describes-a-different-set failure the narrowing recomputes its counts to prevent, one
//! layer further out, and it is why the filter is called per club rather than the swing list sliced
//! here.
//!
//! The filter is on the contract because `analysis` may depend on `contracts` alone (ADR-008). The
//! Python pins that by parsing this module's imports, since `from golf_coach.storage.corpus import
//! narrow_to` is one keystroke away; here the cargo edge is the pin, as `analysis` has no `storage`
//! dependency to reach through.
//!
//! # Declared and hit are two questions, so the union is the answer
//!
//! A club gets a profile if it has been hit **or** is in the bag. Profiling only what was hit loses
//! the new 5 wood with no shots on it; profiling only the declared bag loses the club that has left
//! it and still has real history.
//!
//! # A bag change is caveated, never split
//!
//! `BagEntry.recorded_at` is the only thing on disk that can see a club being replaced, and a club
//! replaced under the same name pools two physical clubs into one carry average. This says so in a
//! sentence and leaves every statistic standing. Splitting the history at that date instead *is*
//! ADR-024's deferred bag-entry versioning, which `contracts::bag` is explicit is not promised:
//! nothing reads `Bag.retired`, and a seam inferred from one timestamp would silently halve an `n`
//! career mode spent learning to count honestly. So the loose reading wins, for
//! `SESSION_DRIFT_FACTOR`'s reason: an entry recorded late for a club that never changed is the
//! likelier case, and being wrong about it has to cost a sentence rather than a verdict.
//!
//! # A mishit is subtracted upstream and only *named* here
//!
//! [`contracts::career::CorpusSwing::artifact_key`] already withheld the topped shot's carry and
//! total from the pool (ADR-028), so the mean here is already right; [`mishit_caveats`] adds the
//! sentence saying a mean was built from fewer shots than `n_shots`, because a number that shrank
//! with nothing explaining it is indistinguishable from a bug.
//!
//! # Where the port reads differently, each invisible to every caller
//!
//! - The instants are [`contracts::Timestamp`]'s order (an aware `datetime`'s), and the caveat's date
//!   is [`contracts::Timestamp::date_ymd`], `%Y-%m-%d` **in the entry's own offset**: the career
//!   family's `bag-declared` dates a 23:30−05:00 entry the 20th, where UTC says the 21st.
//! - The counts are `len()`s here and `i64` fields on [`ClubProfile`], cast once where they meet
//!   (M36 P6's finding).

use std::collections::BTreeSet;

use contracts::bag::{Bag, BagEntry};
use contracts::career::{CareerCorpus, CorpusSwing, Narrowing};
use contracts::club::ClubId;
use contracts::club_profile::{BagProfile, ClubProfile};

use crate::baseline::build_baseline;
use crate::dispersion::build_dispersion;

/// Every club this golfer has hit or declared, with what its history supports saying.
///
/// An empty corpus and no bag is an empty [`BagProfile`] rather than an error: "no swings and no
/// clubs declared yet" is a real answer and the first one every golfer has. `bag` is optional
/// because a golfer who has never declared one still has shot history worth profiling; it costs them
/// the loft and nothing else, and `None` is not a degraded mode.
pub fn build_bag_profile(corpus: &CareerCorpus, bag: Option<&Bag>) -> BagProfile {
    let hit: BTreeSet<ClubId> = corpus
        .swings
        .iter()
        .filter_map(|swing| swing.club)
        .collect();
    let declared = |club: &ClubId| bag.is_some_and(|bag| bag.entries.contains_key(club));

    // Walked off `ClubId::ALL`, never sorted and never taken from the map's order. Declaration order
    // *is* bag order, and `BagProfile`'s validator is what goes red if this is forgotten:
    // alphabetically a 3 wood lands between the hybrids, which is not a bag anyone recognises.
    let clubs = ClubId::ALL
        .into_iter()
        .filter(|club| hit.contains(club) || declared(club));

    BagProfile {
        player_id: corpus.player_id.clone(),
        clubs: clubs.map(|club| profile_for(corpus, club, bag)).collect(),
        // Both read off the **whole** corpus. A narrowed one reports 0 by construction, so taking
        // either from inside the loop would erase a number that describes history no profile above
        // it can see.
        untagged_swings: count(corpus.untagged_swings()),
        mishits_excluded: count(corpus.mishit_shots()),
    }
}

// ------------------------------------------------------------------------------------- internals

/// One club: the physical club, the evidence, and what that evidence supports.
///
/// Private until something needs one club without the bag around it: [`BagProfile::profile_for`] is
/// the lookup the verb's `--club 7i` uses, so a second public entry point would be an abstraction
/// with no caller. `caveats` only ever qualifies; nothing here withholds a statistic on the strength
/// of a bag entry's date.
fn profile_for(corpus: &CareerCorpus, club: ClubId, bag: Option<&Bag>) -> ClubProfile {
    let narrowed = corpus.narrowed_to(&Narrowing {
        club: Some(club),
        ..Narrowing::default()
    });

    // The live entry only: `Bag.retired` is deliberately not consulted. The shelf is *retention*,
    // not the bag-entry versioning ADR-024 defers, so a club that has left the bag keeps every
    // statistic and loses only its loft, which is the split `clubs_used` / `clubs_declared` express.
    let entry = bag.and_then(|bag| bag.entries.get(&club));

    let mut caveats = bag_changed_caveats(entry, &narrowed.swings);
    caveats.extend(mishit_caveats(&narrowed));

    ClubProfile {
        club,
        bag_entry: entry.cloned(),
        // Exact while `Bag` pins `entries` to live clubs only. It stops being exact the day
        // something hands `bag_entry` a retired stint, which is the divergence `ClubProfile.in_bag`
        // documents, and this line is where that has to be answered.
        in_bag: entry.is_some(),
        n_swings: count(narrowed.distinct_swings()),
        n_shots: count(narrowed.distinct_shots()),
        n_sessions: count(narrowed.distinct_sessions()),
        metrics: build_baseline(&narrowed).metrics,
        // A *second* baseline over the same narrowed corpus, and not waste to optimise away:
        // `build_dispersion` takes a corpus on purpose, so its guarded statistics and its raw
        // per-session samples provably describe the same read. Handing it one built here would be
        // the seam through which one club's spread could be paired with another's sessions.
        dispersion: build_dispersion(&narrowed).metrics,
        mishits: count(narrowed.mishit_shots()),
        mishit_refs: narrowed.mishit_refs(),
        mishits_unconfirmed: count(narrowed.mishit_shots_unconfirmed()),
        caveats,
    }
}

/// Say when some of these swings predate the bag entry they are being pooled under.
///
/// **It adds a sentence and removes nothing**: a loose judgment that only ever caveats is one whose
/// failure costs a sentence, where the same judgment wired to a refusal would delete real history
/// every time an entry was simply recorded late.
///
/// **Two sentences, because two different things are true.** With swings on both sides of the date,
/// one average holds two physical clubs and the mixed form says so, with the proportion, since "2 of
/// 40" deflates as later history accumulates and "2 of 4" does not. With every swing on the early
/// side nothing is pooled; what is in doubt is whether the entry's make, model and loft describe the
/// club that hit any of it, and that is the *common* case, since a golfer declaring a bag declares
/// every club at once.
///
/// Strictly `<`: an entry recorded in the same instant as a swing *was* the club that hit it. No
/// grace window: there is no "how much later is suspicious" judgment to make, only "later at all".
/// Both forms count **swings**, never "shots", because `n_shots` beside them counts photos.
fn bag_changed_caveats(entry: Option<&BagEntry>, swings: &[CorpusSwing]) -> Vec<String> {
    // A club with no declared entry has nothing to be inconsistent with, and a declared club with no
    // swings is pooling nothing yet.
    let Some(entry) = entry else {
        return Vec::new();
    };
    if swings.is_empty() {
        return Vec::new();
    }

    let earlier = swings
        .iter()
        .filter(|swing| swing.captured_at < entry.recorded_at)
        .count();
    if earlier == 0 {
        return Vec::new();
    }

    let date = entry.recorded_at.date_ymd();

    if earlier == swings.len() {
        let subject = if swings.len() == 1 {
            "The single swing behind these numbers predates".to_string()
        } else {
            format!("All {} swings behind these numbers predate", swings.len())
        };
        // No pronoun in the tail on purpose: "the one that hit them" needs "it" in the singular,
        // and a sentence that has to agree with a count is one that will one day not.
        return vec![format!(
            "{subject} the bag entry recorded on {date}, so the make, model and loft recorded \
             there may describe a different club."
        )];
    }

    vec![format!(
        "{earlier} of the {} swings behind these numbers were hit before the bag entry recorded on \
         {date}. A club replaced under the same name would put two different clubs in one average, \
         and these numbers pool both.",
        swings.len()
    )]
}

/// Say how many of this club's shots were set aside as mishits, and how many await a verdict.
///
/// Shaped like [`bag_changed_caveats`]: one sentence, appended, removing nothing (the samples were
/// withheld upstream). "N of M" against `distinct_shots`, so a reader sees how much of the distance
/// history it touches. No threshold constant: the 0.50 lives once, in `contracts::mishit`.
fn mishit_caveats(narrowed: &CareerCorpus) -> Vec<String> {
    let total = narrowed.mishit_shots();
    if total == 0 {
        return Vec::new();
    }

    let shots = narrowed.distinct_shots();
    let head = if total == 1 {
        format!("1 of the {shots} shots on this club was")
    } else {
        format!("{total} of the {shots} shots on this club were")
    };
    let mut sentence = format!(
        "{head} set aside as a mishit — a top or duff carrying far below this club's median — and \
         left out of the carry and total-distance averages; every other metric still counts them."
    );

    let unconfirmed = narrowed.mishit_shots_unconfirmed();
    if unconfirmed > 0 {
        let verb = if unconfirmed == 1 { "was" } else { "were" };
        sentence.push_str(&format!(
            " {unconfirmed} {verb} flagged automatically and {verb} not yet confirmed or cleared."
        ));
    }
    vec![sentence]
}

/// A `len()` as the `i64` a contract field holds.
fn count(len: usize) -> i64 {
    i64::try_from(len).expect("a count fits in an i64")
}

#[cfg(test)]
mod tests {
    //! `tests/analysis/test_club_profile_builder.py`, ported case for case. Two properties carry
    //! nearly all the weight: **every number comes from the narrowed corpus** (a per-club mean built
    //! from the whole bag's `n` looks like a working feature), and **hit and declared stay two
    //! questions**. Corpora and bags are built from the contracts directly, no disk.

    use super::*;
    use contracts::baseline::BaselineClaim;
    use contracts::intent::ClubCategory;
    use contracts::mishit::MishitVerdict;
    use contracts::swing::Measurement;
    use contracts::Timestamp;
    use serde_json::json;

    const CARRY: &str = "carry_distance_yds";
    const LM: &str = "launch_monitor:hd_golf";

    fn at(day: u32) -> Timestamp {
        Timestamp::parse(&format!("2026-08-{day:02}T12:00:00Z")).expect("a valid timestamp")
    }

    /// The overrides Python's `_swing` takes as keywords; the defaults give every swing its own
    /// clip, photo and session.
    #[derive(Default)]
    struct Swing {
        carry: Option<f64>,
        face_on: Option<&'static str>,
        shot: Option<&'static str>,
        auto_mishit: bool,
        mishit: Option<MishitVerdict>,
    }

    fn swing(index: u32, club: Option<ClubId>, over: Swing) -> CorpusSwing {
        CorpusSwing {
            player_id: "aaron".into(),
            session_id: format!("2026-08-{index:02}"),
            swing_id: index.to_string(),
            captured_at: at(index),
            face_on_sha256: over
                .face_on
                .map_or_else(|| format!("clip-{index}"), String::from),
            shot_sha256: Some(
                over.shot
                    .map_or_else(|| format!("photo-{index}"), String::from),
            ),
            club,
            measurements: over
                .carry
                .map(|value| Measurement {
                    name: CARRY.into(),
                    value,
                    unit: "yards".into(),
                    source: LM.into(),
                    detail: "test".into(),
                })
                .into_iter()
                .collect(),
            duplicates: Vec::new(),
            conflicting_shots: Vec::new(),
            analyzed: true,
            stale: false,
            analysis_version: 0,
            outdated: false,
            shot_needs_review: false,
            auto_mishit: over.auto_mishit,
            manual_mishit: over.mishit,
            missing_roles: Vec::new(),
        }
    }

    fn carrying(index: u32, club: ClubId, carry: f64) -> CorpusSwing {
        swing(
            index,
            Some(club),
            Swing {
                carry: Some(carry),
                ..Swing::default()
            },
        )
    }

    fn corpus(swings: Vec<CorpusSwing>) -> CareerCorpus {
        serde_json::from_value(json!({"player_id": "aaron"}))
            .map(|empty: CareerCorpus| CareerCorpus { swings, ..empty })
            .expect("an empty corpus parses")
    }

    /// `n` distinct swings of one club, each its own session, clip and shot photo.
    fn hits(club: ClubId, n: u32, start: u32, carry: f64) -> Vec<CorpusSwing> {
        (start..start + n)
            .map(|i| carrying(i, club, carry + f64::from(i)))
            .collect()
    }

    fn sevens(n: u32) -> Vec<CorpusSwing> {
        hits(ClubId::SevenIron, n, 1, 150.0)
    }

    /// A declared bag, every entry recorded on day `recorded` (1 by default, before every swing
    /// `hits` builds, so a test that says nothing about the bag's age earns no caveat).
    fn bag(clubs: &[ClubId], retired: &[ClubId], recorded: u32) -> Bag {
        let when = at(recorded).to_string();
        let entries: serde_json::Map<String, serde_json::Value> = clubs
            .iter()
            .map(|club| (club.to_string(), json!({"club": club, "recorded_at": when})))
            .collect();
        let retired: Vec<serde_json::Value> = retired
            .iter()
            .map(|club| json!({"club": club, "recorded_at": when, "retired_at": when}))
            .collect();
        serde_json::from_value(json!({
            "player_id": "aaron",
            "entries": entries,
            "retired": retired,
            "updated_at": when,
        }))
        .expect("a valid bag")
    }

    fn declared(clubs: &[ClubId]) -> Bag {
        bag(clubs, &[], 1)
    }

    fn clubs_of(profiles: &[&ClubProfile]) -> Vec<ClubId> {
        profiles.iter().map(|p| p.club).collect()
    }

    // --- the guard, per club -------------------------------------------------------------------

    /// The whole milestone in one assertion, and the counts are the half that matters: both clubs
    /// sit in one corpus of eight shots, comfortably over the floor, so a builder that handed
    /// `build_baseline` the corpus would give the driver a confident mean off two shots.
    #[test]
    fn a_club_with_six_shots_states_a_mean_and_one_with_two_refuses() {
        let mut swings = sevens(6);
        swings.extend(hits(ClubId::Driver, 2, 20, 260.0));
        let profile = build_bag_profile(&corpus(swings), None);
        let irons = profile.profile_for(ClubId::SevenIron).unwrap();
        let driver = profile.profile_for(ClubId::Driver).unwrap();

        assert_eq!((irons.n_swings, irons.n_shots), (6, 6));
        assert!(irons.metrics[CARRY].supports(BaselineClaim::Center));
        assert_eq!(irons.metrics[CARRY].n, 6);
        assert!(irons.metrics[CARRY].mean.is_some());

        assert_eq!((driver.n_swings, driver.n_shots), (2, 2));
        assert_eq!(driver.metrics[CARRY].n, 2);
        assert_eq!(
            driver.metrics[CARRY].mean, None,
            "a mean off two shots is the bug this milestone has"
        );
        let refusal = driver.metrics[CARRY]
            .withheld
            .iter()
            .find(|r| r.claim == BaselineClaim::Center)
            .unwrap();
        assert_eq!((refusal.have_n, refusal.need_n), (2, 5));
    }

    /// Passing the narrowed corpus to one builder and the whole one to the other type-checks, runs,
    /// and reports one club's mean beside the whole bag's spread.
    #[test]
    fn the_dispersion_is_built_over_the_same_narrowing_as_the_baseline() {
        let mut swings = sevens(6);
        swings.extend(hits(ClubId::Driver, 2, 20, 260.0));
        let profile = build_bag_profile(&corpus(swings), None);
        let irons = profile.profile_for(ClubId::SevenIron).unwrap();
        assert_eq!(irons.dispersion[CARRY].n, 6);
        assert_eq!(irons.metrics[CARRY].n, 6);
        assert_eq!(
            irons.dispersion[CARRY].n_sessions,
            irons.metrics[CARRY].n_sessions
        );
    }

    /// One 7 iron filmed three times with two photos is three swings of history and a carry ceiling
    /// of two, because every launch-monitor claim dedupes on the photo.
    #[test]
    fn swings_and_shots_are_counted_separately_and_the_photos_are_the_ceiling() {
        let with = |index, carry, face_on, shot| {
            swing(
                index,
                Some(ClubId::SevenIron),
                Swing {
                    carry: Some(carry),
                    face_on: Some(face_on),
                    shot: Some(shot),
                    ..Swing::default()
                },
            )
        };
        let profile = build_bag_profile(
            &corpus(vec![
                with(1, 150.0, "clip-1", "photo-a"),
                with(2, 150.0, "clip-2", "photo-a"),
                with(3, 160.0, "clip-3", "photo-b"),
            ]),
            None,
        );
        let irons = profile.profile_for(ClubId::SevenIron).unwrap();
        assert_eq!((irons.n_swings, irons.n_shots, irons.n_sessions), (3, 2, 3));
        assert_eq!(
            irons.metrics[CARRY].n, 2,
            "the carry counts photos, not clips"
        );
    }

    // --- hit, declared, or both ----------------------------------------------------------------

    /// Real history and nothing declared: a builder that profiled only declared clubs would return
    /// an empty page.
    #[test]
    fn an_empty_bag_still_profiles_every_club_that_has_shots() {
        let mut swings = hits(ClubId::SevenIron, 2, 1, 150.0);
        swings.extend(hits(ClubId::PitchingWedge, 1, 20, 150.0));
        let corpus = corpus(swings);
        let empty = declared(&[]);
        for bag in [None, Some(&empty)] {
            let profile = build_bag_profile(&corpus, bag);
            let order = [ClubId::SevenIron, ClubId::PitchingWedge];
            assert_eq!(
                profile.clubs.iter().map(|p| p.club).collect::<Vec<_>>(),
                order
            );
            assert_eq!(clubs_of(&profile.clubs_used()), order);
            assert!(profile.clubs_declared().is_empty());
        }
    }

    /// "You have not hit your driver yet" is a real answer, and it reaches the page as a club with
    /// no numbers rather than as an absence.
    #[test]
    fn a_declared_but_unhit_club_appears_with_no_history_and_every_claim_withheld() {
        let profile = build_bag_profile(&corpus(Vec::new()), Some(&declared(&[ClubId::Driver])));
        let driver = profile.profile_for(ClubId::Driver).unwrap();
        assert!(driver.in_bag);
        assert!(driver.bag_entry.is_some());
        assert_eq!(
            (driver.n_swings, driver.n_shots, driver.n_sessions),
            (0, 0, 0)
        );
        assert!(driver.metrics.is_empty() && driver.dispersion.is_empty());
        assert!(profile.clubs_used().is_empty());
        assert_eq!(clubs_of(&profile.clubs_declared()), [ClubId::Driver]);
    }

    /// The retired shelf is retention, not versioning, so it is not read here (ADR-024): history
    /// stays, loft goes.
    #[test]
    fn a_club_that_has_left_the_bag_keeps_its_history_and_loses_only_its_loft() {
        let shelf = bag(&[ClubId::Driver], &[ClubId::SevenIron], 1);
        let profile = build_bag_profile(&corpus(sevens(2)), Some(&shelf));
        let irons = profile.profile_for(ClubId::SevenIron).unwrap();
        assert!(!irons.in_bag);
        assert!(
            irons.bag_entry.is_none(),
            "the shelf is not a fallback for the current entry"
        );
        assert_eq!(irons.n_swings, 2);
        assert_eq!(clubs_of(&profile.clubs_used()), [ClubId::SevenIron]);
        assert_eq!(clubs_of(&profile.clubs_declared()), [ClubId::Driver]);
    }

    /// Insertion (`pw 3w driver 7i`), alphabetical (`3w 7i driver pw`) and bag order (`driver 3w 7i
    /// pw`) all differ over these four, so a builder that took the order from the map or sorted
    /// fails visibly.
    #[test]
    fn clubs_come_back_in_canonical_bag_order_whatever_order_the_bag_declared_them() {
        let bag = declared(&[
            ClubId::PitchingWedge,
            ClubId::ThreeWood,
            ClubId::Driver,
            ClubId::SevenIron,
        ]);
        let profile = build_bag_profile(&corpus(Vec::new()), Some(&bag));
        assert_eq!(
            profile.clubs.iter().map(|p| p.club).collect::<Vec<_>>(),
            [
                ClubId::Driver,
                ClubId::ThreeWood,
                ClubId::SevenIron,
                ClubId::PitchingWedge
            ]
        );
        assert_eq!(
            profile
                .clubs
                .iter()
                .map(ClubProfile::category)
                .collect::<Vec<_>>(),
            [
                ClubCategory::Driver,
                ClubCategory::Wood,
                ClubCategory::MidIron,
                ClubCategory::Wedge
            ]
        );
    }

    /// The history no club profile can see, kept where a reader will find it.
    #[test]
    fn untagged_swings_reach_no_profile_and_are_reported_once() {
        let mut swings = sevens(2);
        swings.push(swing(
            9,
            None,
            Swing {
                carry: Some(140.0),
                ..Swing::default()
            },
        ));
        let corpus = corpus(swings);
        let profile = build_bag_profile(&corpus, None);
        assert_eq!(
            profile.clubs.iter().map(|p| p.club).collect::<Vec<_>>(),
            [ClubId::SevenIron]
        );
        assert_eq!(profile.profile_for(ClubId::SevenIron).unwrap().n_swings, 2);
        assert_eq!(profile.untagged_swings, 1);
        assert_eq!(
            build_baseline(&corpus).metrics[CARRY].n,
            3,
            "untagged shots still count whole-bag"
        );
    }

    #[test]
    fn no_swings_and_no_bag_is_an_empty_profile_not_an_error() {
        let profile = build_bag_profile(&corpus(Vec::new()), None);
        assert_eq!(profile.player_id, "aaron");
        assert!(profile.clubs.is_empty());
        assert_eq!(profile.untagged_swings, 0);
        assert!(profile.profile_for(ClubId::Driver).is_none());
    }

    // --- the bag-changed caveat ----------------------------------------------------------------

    fn caveats(corpus: &CareerCorpus, bag: Option<&Bag>, club: ClubId) -> Vec<String> {
        build_bag_profile(corpus, bag)
            .profile_for(club)
            .expect("a profile for the club")
            .caveats
            .clone()
    }

    fn seven_bag(recorded: u32) -> Bag {
        bag(&[ClubId::SevenIron], &[], recorded)
    }

    /// The clean case, and the one the caveat must not cry wolf on.
    #[test]
    fn an_entry_recorded_before_every_swing_says_nothing() {
        let corpus = corpus(hits(ClubId::SevenIron, 3, 5, 150.0));
        assert!(caveats(&corpus, Some(&seven_bag(2)), ClubId::SevenIron).is_empty());
    }

    /// Four driver swings also predate the entry and are pooled under another name: a builder handing
    /// the helper `corpus.swings` writes a caveat about eight swings onto a club that has four.
    #[test]
    fn an_entry_recorded_mid_history_names_the_date_and_how_many_predate_it() {
        let mut swings = hits(ClubId::SevenIron, 4, 1, 150.0);
        swings.extend(hits(ClubId::Driver, 4, 20, 260.0));
        let found = caveats(&corpus(swings), Some(&seven_bag(3)), ClubId::SevenIron);
        assert_eq!(
            found,
            [
                "2 of the 4 swings behind these numbers were hit before the bag entry recorded on \
              2026-08-03. A club replaced under the same name would put two different clubs in one \
              average, and these numbers pool both."
            ]
        );
    }

    /// The common case once a golfer declares a bag after months of range sessions: nothing is
    /// pooled, and what is in doubt is the entry, not the average.
    #[test]
    fn an_entry_recorded_after_every_swing_doubts_the_club_not_the_pooling() {
        let found = caveats(&corpus(sevens(4)), Some(&seven_bag(9)), ClubId::SevenIron);
        assert_eq!(
            found,
            ["All 4 swings behind these numbers predate the bag entry recorded on 2026-08-09, so \
              the make, model and loft recorded there may describe a different club."]
        );
        assert!(
            !found[0].contains("pool"),
            "nothing is pooled when every swing is on the same side"
        );
    }

    /// Plain English on the smallest history there is, which is where a naive count reads worst.
    #[test]
    fn the_sentence_does_not_say_one_of_these_1_swings() {
        let caveat = &caveats(&corpus(sevens(1)), Some(&seven_bag(5)), ClubId::SevenIron)[0];
        assert!(
            caveat.contains("The single swing behind these numbers predates"),
            "{caveat}"
        );
        assert!(!caveat.contains("1 swings"), "{caveat}");
    }

    /// The `<` / `<=` boundary: off by one, every bag declared during a session caveats it.
    #[test]
    fn a_swing_hit_in_the_same_instant_the_entry_was_recorded_does_not_predate_it() {
        let corpus = corpus(hits(ClubId::SevenIron, 3, 4, 150.0));
        assert!(caveats(&corpus, Some(&seven_bag(4)), ClubId::SevenIron).is_empty());
    }

    /// The date is the entry's own: 23:30−05:00 on the 3rd is 04:30Z on the 4th, so only swing 3
    /// (12:00Z on the 3rd) predates it, and the sentence dates it the 3rd where UTC says the 4th.
    #[test]
    fn the_entrys_own_offset_names_the_date() {
        let mut entry = seven_bag(1);
        let recorded = Timestamp::parse("2026-08-03T23:30:00-05:00").unwrap();
        entry
            .entries
            .get_mut(&ClubId::SevenIron)
            .unwrap()
            .recorded_at = recorded;
        let found = caveats(
            &corpus(hits(ClubId::SevenIron, 3, 3, 150.0)),
            Some(&entry),
            ClubId::SevenIron,
        );
        assert_eq!(found.len(), 1);
        assert!(found[0].starts_with("1 of the 3 swings"), "{}", found[0]);
        assert!(found[0].contains("recorded on 2026-08-03."), "{}", found[0]);
    }

    /// No entry, no date, no caveat, for an undeclared club and a retired one alike: the retired one
    /// must not borrow the shelf's `recorded_at`.
    #[test]
    fn a_club_with_no_bag_entry_has_nothing_to_be_inconsistent_with() {
        let corpus = corpus(hits(ClubId::SevenIron, 3, 5, 150.0));
        assert!(caveats(&corpus, None, ClubId::SevenIron).is_empty());
        let shelf = bag(&[], &[ClubId::SevenIron], 9);
        assert!(caveats(&corpus, Some(&shelf), ClubId::SevenIron).is_empty());
    }

    /// The empty-`swings` path, and the guard that stops the helper reaching for an empty list.
    #[test]
    fn a_declared_but_unhit_club_is_pooling_nothing_and_so_says_nothing() {
        let bag = bag(&[ClubId::Driver], &[], 9);
        assert!(caveats(&corpus(Vec::new()), Some(&bag), ClubId::Driver).is_empty());
    }

    /// The load-bearing one: a caveat must not be able to delete history. Six shots clear the floor,
    /// and an entry recorded midway is the likelier case of a golfer declaring late.
    #[test]
    fn the_caveat_qualifies_the_statistics_and_never_withholds_them() {
        let profile = build_bag_profile(&corpus(sevens(6)), Some(&seven_bag(4)));
        let irons = profile.profile_for(ClubId::SevenIron).unwrap();
        assert_eq!(irons.caveats.len(), 1);
        assert!(irons.metrics[CARRY].supports(BaselineClaim::Center));
        assert!(irons.metrics[CARRY].mean.is_some());
        assert_eq!(
            irons.metrics[CARRY].n, 6,
            "the caveat must not narrow what the mean is built from"
        );
    }

    // --- the mishit caveat ---------------------------------------------------------------------

    fn topped(index: u32, club: ClubId, carry: f64, over: Swing) -> CorpusSwing {
        swing(
            index,
            Some(club),
            Swing {
                carry: Some(carry),
                ..over
            },
        )
    }

    fn auto() -> Swing {
        Swing {
            auto_mishit: true,
            ..Swing::default()
        }
    }

    #[test]
    fn a_mishit_is_counted_named_and_left_out_of_the_carry_mean_only() {
        let mut swings = sevens(6); // carries 151..156
        swings.push(topped(9, ClubId::SevenIron, 20.0, auto()));
        let profile = build_bag_profile(&corpus(swings), None);
        let irons = profile.profile_for(ClubId::SevenIron).unwrap();

        assert_eq!(irons.n_shots, 7, "the top is still a shot");
        assert_eq!(irons.mishits, 1);
        assert_eq!(irons.mishit_refs, ["2026-08-09/9"]);
        assert_eq!(irons.mishits_unconfirmed, 1);
        assert_eq!(irons.metrics[CARRY].n, 6, "but not a carry sample");
        assert_eq!(
            irons.caveats,
            [
                "1 of the 7 shots on this club was set aside as a mishit — a top or duff carrying \
              far below this club's median — and left out of the carry and total-distance \
              averages; every other metric still counts them. 1 was flagged automatically and was \
              not yet confirmed or cleared."
            ]
        );
    }

    #[test]
    fn a_confirmed_mishit_reads_as_ruled_on_not_waiting() {
        let mut swings = sevens(6);
        swings.push(topped(
            9,
            ClubId::SevenIron,
            110.0,
            Swing {
                mishit: Some(MishitVerdict::Confirmed),
                ..Swing::default()
            },
        ));
        let profile = build_bag_profile(&corpus(swings), None);
        let irons = profile.profile_for(ClubId::SevenIron).unwrap();
        assert_eq!((irons.mishits, irons.mishits_unconfirmed), (1, 0));
        assert_eq!(irons.metrics[CARRY].n, 6);
        assert!(!irons
            .caveats
            .iter()
            .any(|c| c.contains("not yet confirmed")));
    }

    /// Two of each, so the plural forms are the ones read.
    #[test]
    fn the_plural_forms_agree_with_their_counts() {
        let mut swings = sevens(6);
        swings.push(topped(9, ClubId::SevenIron, 20.0, auto()));
        swings.push(topped(10, ClubId::SevenIron, 25.0, auto()));
        let profile = build_bag_profile(&corpus(swings), None);
        let caveat = &profile.profile_for(ClubId::SevenIron).unwrap().caveats[0];
        assert!(
            caveat.starts_with("2 of the 8 shots on this club were set aside"),
            "{caveat}"
        );
        assert!(
            caveat
                .ends_with(" 2 were flagged automatically and were not yet confirmed or cleared."),
            "{caveat}"
        );
    }

    #[test]
    fn the_bag_profile_sums_mishits_across_clubs() {
        let mut swings = sevens(6);
        swings.push(topped(9, ClubId::SevenIron, 20.0, auto()));
        swings.extend(hits(ClubId::Driver, 6, 13, 250.0));
        swings.push(topped(20, ClubId::Driver, 30.0, auto()));
        assert_eq!(build_bag_profile(&corpus(swings), None).mishits_excluded, 2);
    }

    #[test]
    fn no_mishit_no_caveat_and_zero_counts() {
        let profile = build_bag_profile(&corpus(sevens(6)), None);
        let irons = profile.profile_for(ClubId::SevenIron).unwrap();
        assert_eq!((irons.mishits, irons.mishits_unconfirmed), (0, 0));
        assert!(irons.mishit_refs.is_empty());
        assert!(!irons.caveats.iter().any(|c| c.contains("mishit")));
    }
}
