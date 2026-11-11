//! How a corpus becomes what the Profile tab shows. [M38 P11]
//!
//! Outside `api` because the codegen binds what is `pub` there (rule 7). This grades nothing and words
//! nothing: the grades and the list are `golf_core::profile::run_profile`'s (C23), and every sentence is
//! a helper of `golf_core::reports::profile`, the one `golf-core profile` itself prints through, but a
//! club's mean, which is `golf_core::reports::club_profile`'s (below).
//!
//! # Why [`profile_view`] is `pub` at the crate root
//!
//! The gate in `tests/profile.rs` runs it over `spec/vectors/profile/`, whose inputs are corpora and not
//! stores, and a test is outside the crate. That is `parse_now`'s footing: public, outside `api`, and so
//! not something Dart can call.
//!
//! # The one number from another report
//!
//! A club's mean is `golf-core club-profile`'s `center` row, through its own helper
//! (`reports::club_profile::center_text`), read off the `BagProfile` that `run_profile` builds its
//! grades from ([`run_profile_and_bag_profile`]). `golf-core profile` prints no mean, and the graded
//! profile carries none for carry, whose dispersion has no target and so no `center`. Added at the
//! user's decision on M38 P11's finding 5, so the tab can say how far a club goes once its mean clears
//! the floor, which comes before its spread's.
//!
//! # What the report prints and the view leaves out, each on purpose
//!
//! - **The impact map and its lean.** Only a measured face-impact offset fills one (ADR-034's M37 and
//!   M42 addenda), which comes off a clip's replay, and the phone keeps stills only (D4). So nothing the
//!   phone stores can fill it, and the strike-location topic it goes with cannot exist here either.
//!   That topic still maps if a future build meets one, as every topic does.
//! - **The player's raw numbers**, by the plan's Do line. What the player's strike profile holds is
//!   club-independent, and the player's list already names what is established in it.
//! - **The sentence for a missing blend.** A missing blend is no card at all (rule 8), and the report's
//!   sentence would be one more thing to read beside nothing.
//!
//! # The bag is read and moves nothing today
//!
//! `run_profile` takes it, `golf-core profile` passes it, and so this does: a grade that came to read it
//! would otherwise differ between the phone and the lab with nothing to say so. Nothing in
//! `analysis::strike` or `feedback::profile_rules` reads a declared entry today, so a club in the bag with
//! no shots is simply no club of this profile, and dropping the bag here fails no test (M38 P11's finding).
//!
//! # One moment
//!
//! The stores are read under `crate::record::one_writer()`, as `crate::history`'s reads are, **though not
//! for its reason**. The corpus reads manifests and analyses, each replaced whole, and not the shot store,
//! whose in-place write is what tears. What is left for the lock to stop is a read landing between two
//! writes: a retag analyses first and stores second, so in between a read pools the new club's flight under
//! the old club's name. The next read is right either way, and no test here can see the difference
//! (`tests/profile.rs`' race says so). It is held because it costs one wait and leaves every read of the
//! stores on this phone under one rule. It is let go before the grading, which reads no file. **A writer
//! must not call [`graded`] while it holds that lock**, for the reason `crate::history` gives.

use contracts::bag::Bag;
use contracts::career::CareerCorpus;
use contracts::club_profile::ClubProfile;
use contracts::strike::{Blend, ProfileFinding, Share, TopicGrade};
use golf_core::profile::run_profile_and_bag_profile;
use golf_core::reports::club_profile::center_text;
use golf_core::reports::profile::{
    blend_text, club_shots, criterion, finding_text, landings_text, no_clubs_note, no_landing,
    not_judged, percent, player_shots, preamble, shape_rule, share_text, spread_rows, topic_name,
    NOTHING_LISTED, NO_TOPICS, PLAYER_BASIS,
};
use storage::bag_store::BagStore;
use storage::corpus::read_corpus;

use crate::api::profile::{
    BlendView, ClubView, FindingView, NumberRow, PlayerView, ProfileView, ShapeView, ShareView,
    TopicView,
};
use crate::record::one_writer;
use crate::{engine_versions, Dirs};

/// `graded_profile`'s body: the module doc's one moment, then the view.
pub(crate) fn graded(root: &str, player_id: &str) -> Result<ProfileView, String> {
    let dirs = Dirs::of(root);
    let (corpus, bag) = {
        let _one_moment = one_writer();
        (
            read_corpus(&dirs.sessions, player_id, engine_versions()),
            BagStore::new(&dirs.golfers).get(player_id),
        )
    };
    profile_view(&corpus, bag.as_ref())
}

/// One corpus and bag as the view, through `run_profile`. Not a screen's to call: see the module doc.
///
/// `Err` is `run_profile`'s, a profile that fails its own check.
pub fn profile_view(corpus: &CareerCorpus, bag: Option<&Bag>) -> Result<ProfileView, String> {
    let (profile, bag_profile) = run_profile_and_bag_profile(corpus, bag).map_err(|refused| {
        format!(
            "the graded profile failed its own check, {refused}. That is a bug in a builder, not a \
             fact about the golfer."
        )
    })?;
    let no_clubs = no_clubs_note(&profile).map(str::to_string);
    let clubs = profile
        .clubs
        .iter()
        .map(|graded| {
            let grades = &graded.grades;
            let (strengths, weaknesses, nothing_listed) = list_of(&graded.list);
            // `run_profile` grades only its `BagProfile`'s own clubs, so the lookup cannot miss.
            let club_profile = bag_profile
                .profile_for(grades.club)
                .expect("run_profile grades only the clubs its BagProfile holds");
            ClubView {
                club: grades.club.as_str().to_string(),
                n_shots: grades.n_shots,
                shots: club_shots(grades.n_shots),
                numbers: spread_rows(&grades.strike)
                    .into_iter()
                    .map(|spread| NumberRow {
                        center: center_of(club_profile, &spread.name),
                        name: spread.name,
                        n: spread.n,
                        spread: spread.text,
                        withheld: spread.withheld,
                    })
                    .collect(),
                topics: topics_of(&grades.topics),
                no_topics: no_topics(&grades.topics),
                blend: grades.blend.as_ref().map(blend_of),
                strengths,
                weaknesses,
                nothing_listed,
            }
        })
        .collect();
    let grades = &profile.player.grades;
    let player = (grades.n_shots > 0).then(|| {
        let (strengths, weaknesses, nothing_listed) = list_of(&profile.player.list);
        PlayerView {
            basis: PLAYER_BASIS.to_string(),
            n_shots: grades.n_shots,
            shots: player_shots(grades),
            topics: topics_of(&grades.topics),
            no_topics: no_topics(&grades.topics),
            blend: grades.blend.as_ref().map(blend_of),
            strengths,
            weaknesses,
            nothing_listed,
        }
    });
    Ok(ProfileView {
        player_id: profile.player_id,
        preamble: preamble(),
        no_clubs,
        clubs,
        player,
    })
}

/// A number's mean as `golf-core club-profile` prints it for this club, or `None` while it is
/// withheld. Every number row is a metric of the club's own baselines, since the grades copy each one
/// from them, so a row with no baseline would be a builder's bug, and it shows no mean rather than a
/// guessed one.
fn center_of(club: &ClubProfile, metric: &str) -> Option<String> {
    club.metrics.get(metric).and_then(center_text)
}

fn no_topics(topics: &[TopicGrade]) -> Option<String> {
    topics.is_empty().then(|| NO_TOPICS.to_string())
}

fn topics_of(topics: &[TopicGrade]) -> Vec<TopicView> {
    topics.iter().map(topic_of).collect()
}

fn topic_of(grade: &TopicGrade) -> TopicView {
    let what = criterion(grade.topic);
    TopicView {
        name: topic_name(grade.topic),
        grade: share_of(&grade.grade, &what),
        criterion: what,
        unscored: not_judged(&grade.unscored),
        shape: grade.shape.as_ref().map(|shape| ShapeView {
            hit_shape: share_of(&shape.hit_shape, &shape_rule(grade.topic)),
            on_line_projected: shape.projected > 0,
            landings: landings_text(shape),
            no_landing: no_landing(&shape.landing_unscored),
        }),
    }
}

/// One share as the report words it, its percentage split out for the screen to set large.
fn share_of(share: &Share, what: &str) -> ShareView {
    ShareView {
        percent: share.graded().map(|rate| percent(rate.share)),
        text: share_text(share, what),
    }
}

fn blend_of(blend: &Blend) -> BlendView {
    let names = |topics: &[contracts::strike::Topic]| -> Vec<String> {
        topics.iter().map(|topic| topic_name(*topic)).collect()
    };
    BlendView {
        mean: percent(blend.mean),
        averaged: names(&blend.averaged),
        excluded: names(&blend.excluded),
        text: blend_text(blend),
    }
}

/// A list split by what each entry is, each side keeping the list's own order, with what to say if
/// the whole list is empty.
fn list_of(list: &[ProfileFinding]) -> (Vec<FindingView>, Vec<FindingView>, Option<String>) {
    let side = |weakness: bool| -> Vec<FindingView> {
        list.iter()
            .filter(|finding| finding.kind.is_weakness() == weakness)
            .map(|finding| FindingView {
                metric: finding.metric.clone(),
                text: finding_text(finding),
            })
            .collect()
    };
    let nothing = list.is_empty().then(|| NOTHING_LISTED.to_string());
    (side(false), side(true), nothing)
}
