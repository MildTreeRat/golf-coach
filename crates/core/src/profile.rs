//! A golfer's graded profile in one call: each club's grades and list, then the player's. [M37 P9]
//!
//! **No Python twin, and none is coming.** Like `contracts::strike`, `analysis::strike` and
//! `feedback::profile_rules`, which it joins, this is M37's and Rust's alone: frozen Python gains
//! nothing (ADR-035 clause 4), and what it must answer is worked by hand in
//! `spec/vectors/profile/hand/` rather than recorded from an oracle.
//!
//! # Why the shell joins them
//!
//! The grades are `analysis`' (`strike::build_grades`) and the strengths and weaknesses list is
//! `feedback`'s (`profile_rules::rank_profile` and `rank_player`), and ADR-008 as a cargo edge keeps
//! each from reaching the other (`crates/feedback/Cargo.toml`). The two do not feed each other
//! either (ADR-034 §5.7): the grade says how often a club's shots were good, and the list says what
//! its dispersion established. So, as with `build_feedback` beside `analyze_swing_bundle` in
//! [`crate::run`], the crate above both makes the calls (the M37 plan's call 13).
//!
//! # The three steps, and why each reads what it reads
//!
//! 1. `build_bag_profile(corpus, bag)`: each club's baseline and dispersion, which the grades copy
//!    (ball speed, and the three strike dispersions) and the club's list ranks. One build serves
//!    both, so a club's grades and its list cannot read two different spreads. Since M38 P26
//!    [`run_profile_and_bag_profile`] hands the same build to the phone, for the means it shows.
//! 2. `build_grades(corpus, &bag_profile)`: every club with a shot photo, in bag order, then the
//!    player's grades, which judge the whole corpus once (the plan's P7 finding 2). A club the bag
//!    declares and the golfer never photographed is profiled by step 1 and is no club of this
//!    profile (P5 finding 5).
//! 3. The lists: each graded club's from its own `ClubProfile`, and the player's from
//!    `build_dispersion` over the **whole** corpus, kept to club-independent metrics by
//!    `rank_player`. `build_player_grades` builds that same dispersion for the player's strike
//!    numbers and keeps only what it shows (P7 finding 4), so it is built a second time here rather
//!    than threaded out: the same function over the same corpus gives the same bits, and a builder
//!    that returned a dispersion it does not grade would hand the phone a second spelling of it.
//!
//! The grades carry no player id, so the profile takes the corpus's.
//!
//! # The answer is validated once, whole
//!
//! `GradedProfile`'s `Validate` holds every part of a profile to the others: a blend to its graded
//! topics, heel/toe to its grid, a list to its own ordering and scope rules, the clubs to bag order
//! (P1 finding 11). Each builder promises all of it; [`run_profile`] checks it in one pass before
//! anything reads the answer, and a refusal is an `Err` rather than a panic so that `golf-core
//! profile` over `data/` reports a builder bug instead of aborting on one. Nothing here re-decides a
//! grade or a finding.

use analysis::club_profile::build_bag_profile;
use analysis::dispersion::build_dispersion;
use analysis::strike::build_grades;
use contracts::bag::Bag;
use contracts::career::CareerCorpus;
use contracts::club_profile::BagProfile;
use contracts::strike::{GradedClub, GradedPlayer, GradedProfile};
use contracts::{ContractError, Validate};
use feedback::profile_rules::{rank_player, rank_profile};

/// Every graded club with its list, then the player's grades with theirs, for one golfer's corpus
/// and declared bag (`None` for a golfer with no bag). The module doc gives the three steps.
///
/// `Err` only when the assembled profile fails its own `Validate`, which the builders promise it
/// cannot: a refusal is a bug in one of them, named by the field it broke.
pub fn run_profile(
    corpus: &CareerCorpus,
    bag: Option<&Bag>,
) -> Result<GradedProfile, ContractError> {
    run_profile_and_bag_profile(corpus, bag).map(|(profile, _)| profile)
}

/// [`run_profile`], and the `BagProfile` its step 1 built, for a caller that shows a number that
/// profile holds and the graded one does not. [M38 P26]
///
/// The phone's Profile tab is that caller: it shows each club's mean carry as `golf-core
/// club-profile` prints it, from the club's baseline, and the graded profile carries a dispersion
/// for carry whose `center` stays empty, since carry has no target to be biased from
/// (`contracts::dispersion::METRIC_TARGETS`). Handing out the one build, rather than the phone
/// calling `build_bag_profile` again, keeps the number beside a grade from the same build the grade
/// read, and keeps `analysis` off the phone's crate's edges. The `BagProfile` is not validated
/// here: it is the career builder's answer, which `golf-core club-profile` prints as it stands.
pub fn run_profile_and_bag_profile(
    corpus: &CareerCorpus,
    bag: Option<&Bag>,
) -> Result<(GradedProfile, BagProfile), ContractError> {
    let bag_profile = build_bag_profile(corpus, bag);
    let grades = build_grades(corpus, &bag_profile);
    let clubs = grades
        .clubs
        .into_iter()
        .map(|club| {
            // `build_grades` grades only the `BagProfile`'s own clubs, so the lookup cannot miss.
            let profile = bag_profile
                .profile_for(club.club)
                .expect("build_grades grades only the clubs its BagProfile holds");
            GradedClub {
                list: rank_profile(profile),
                grades: club,
            }
        })
        .collect();
    let profile = GradedProfile {
        player_id: corpus.player_id.clone(),
        clubs,
        player: GradedPlayer {
            grades: grades.player,
            list: rank_player(&build_dispersion(corpus)),
        },
    };
    profile.validate()?;
    Ok((profile, bag_profile))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_corpus() -> CareerCorpus {
        serde_json::from_value(serde_json::json!({"player_id": "nobody"}))
            .expect("an empty corpus reads")
    }

    /// A golfer with no swings has a profile, and it is empty rather than refused: no club, no
    /// topic, no blend, nothing listed. The verb prints "nothing to grade" from this rather than an
    /// error.
    #[test]
    fn an_empty_corpus_answers_an_empty_profile() {
        let profile = run_profile(&empty_corpus(), None).expect("an empty profile validates");
        assert_eq!(profile.player_id, "nobody");
        assert!(profile.clubs.is_empty());
        assert_eq!(profile.player.grades.n_shots, 0);
        assert!(profile.player.grades.topics.is_empty());
        assert!(profile.player.grades.blend.is_none());
        assert!(profile.player.list.is_empty());
    }
}
