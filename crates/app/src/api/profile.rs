//! What the Profile tab shows, as plain data. [M38 P11]
//!
//! The shapes Dart sees and one one-line call (rule 7). `crate::profile` holds how they are filled.
//! **Every sentence here is `golf-core profile`'s own**, and `tests/profile.rs` fails on one that is
//! not, so the phone cannot say what the lab's report does not.
//!
//! The one exception is [`NumberRow::center`], which is `golf-core club-profile`'s, since `golf-core
//! profile` prints no mean, and `tests/profile.rs` holds it to that verb's line instead.
//!
//! Three states, spelled three ways, as the report spells them (ADR-034 §5.5): a **graded** share has a
//! `percent`; a **withheld** one has `None` there and says what it waits for in `text`; and an **absent**
//! topic is not in `topics` at all, nor named anywhere. So `None` is never a 0, and nothing here needs
//! Dart to work out which it is holding.

/// One golfer's graded profile. A golfer with nothing recorded answers this too, with no clubs and no
/// player, and `no_clubs` saying so.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileView {
    pub player_id: String,
    /// What a grade is and what it leaves out, for the head of the tab.
    pub preamble: String,
    /// Why no club is listed, or `None` if one is. It covers both nothing recorded at all and shots
    /// none of which name a club, the second with the player's grades still below it.
    pub no_clubs: Option<String>,
    /// One per club with a tracked shot, in bag order, and only those: a club in the bag with no shots
    /// is in `bag_slots` and not here.
    pub clubs: Vec<ClubView>,
    /// `None` with no tracked shot at all, where `golf-core profile` prints `no_clubs` and stops, since
    /// "these shots" and "0 across the bag" would be sentences about nothing.
    pub player: Option<PlayerView>,
}

/// One club's numbers, grades and list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClubView {
    /// `ClubId`'s own spelling, `7i`.
    pub club: String,
    /// Tracked shot photos of this club, the evidence every topic is judged from. Never gated.
    pub n_shots: i64,
    /// `n_shots` in words.
    pub shots: String,
    /// Ball speed, then carry and the other strike numbers the shots print, by name. A number no shot
    /// holds has no row.
    pub numbers: Vec<NumberRow>,
    /// The topics that exist for this club, in topic order. A topic not here is absent.
    pub topics: Vec<TopicView>,
    /// What to say in place of `topics` when there are none, and `None` otherwise.
    pub no_topics: Option<String>,
    /// The club blend. `None` when no topic is graded: there is no blend, and it is never 0.
    pub blend: Option<BlendView>,
    /// What is established as good. Never "nothing established": that is `nothing_listed`.
    pub strengths: Vec<FindingView>,
    /// What is established as a miss, the largest first.
    pub weaknesses: Vec<FindingView>,
    /// What to say when both lists are empty, and `None` otherwise.
    pub nothing_listed: Option<String>,
}

/// The player's grades: every club's shots, each judged by its own club's criterion. A club's view less
/// its numbers, because a driver-plus-wedge average is not a number about anybody (ADR-034 §4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerView {
    /// What these grades pool, which is not what a club's do.
    pub basis: String,
    /// Tracked shot photos across the bag, including any that name no club.
    pub n_shots: i64,
    /// `n_shots` in words, naming the shots with no club and what they count toward.
    pub shots: String,
    /// As [`ClubView::topics`].
    pub topics: Vec<TopicView>,
    /// As [`ClubView::no_topics`].
    pub no_topics: Option<String>,
    /// The player blend. `None` when no topic is graded, as [`ClubView::blend`].
    pub blend: Option<BlendView>,
    pub strengths: Vec<FindingView>,
    pub weaknesses: Vec<FindingView>,
    /// As [`ClubView::nothing_listed`].
    pub nothing_listed: Option<String>,
}

/// One raw number beside the grades.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NumberRow {
    /// The contract's name, `carry_distance_yds`, as the report's key column prints it. No label table
    /// exists for these, and one added here would say what the report does not.
    pub name: String,
    /// How many shots stand behind it.
    pub n: i64,
    /// The mean as `golf-core club-profile` prints it on its `center` row, `152.4   95% CI 149.0 ..
    /// 155.8   (median 153.0)`, in the metric's own unit. `None` until the mean clears its floor,
    /// which is lower than the spread's, so a club's carry shows here while `spread` still waits.
    /// **Held to that verb's text and not `golf-core profile`'s**, which prints no mean: the one
    /// field here from another report (the user's decision of 2026-11-05, on M38 P11's finding 5).
    pub center: Option<String>,
    /// `sd 4.2 yards, 95% CI 3.1 .. 6.4, n = 12`, or `withheld: ` and why.
    pub spread: String,
    /// True when `spread` is the refusal, so no SD is shown.
    pub withheld: bool,
}

/// One topic, for one club or for the player.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TopicView {
    /// `low point`, as a reader says it.
    pub name: String,
    /// What the grade is a share of, `low point ahead of the ball`.
    pub criterion: String,
    /// The topic's grade. On a shape topic this is the on-line share: hit the shape and finished on
    /// line, the one that enters the blend.
    pub grade: ShareView,
    /// Shots the topic was printed for and could not judge, one sentence a reason. **Not misses**: a
    /// blank or misread input is unscored for this topic and in no share (ADR-010 §2).
    pub unscored: Vec<String>,
    /// Present on a shape topic and only there.
    pub shape: Option<ShapeView>,
}

/// One share of shots, or why it is not shown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShareView {
    /// `58%`. `None` is withheld, and is never 0.
    pub percent: Option<String>,
    /// With a percentage, what of, how many and the interval. Withheld, what it waits for.
    pub text: String,
}

/// What a shape topic shows beside its grade (ADR-034 §5.3), so a golfer can tell the shaping failing
/// from the aiming.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShapeView {
    /// The other share, hit the shape wherever it finished. Never in a blend.
    pub hit_shape: ShareView,
    /// At least one landing behind the on-line share came from the flight model and was not printed.
    /// `landings` then says `projected`, and the screen must show that it is.
    pub on_line_projected: bool,
    /// Where the landings came from, or `None` if the on-line share rests on none.
    pub landings: Option<String>,
    /// Judged shots with no landing, one sentence a reason. In `hit_shape` and out of the on-line share.
    pub no_landing: Vec<String>,
}

/// A blend of a club's or the player's graded topics. No interval, because none has been derived.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlendView {
    /// `58%`.
    pub mean: String,
    /// The graded topics it averaged, by name, in topic order.
    pub averaged: Vec<String>,
    /// The withheld topics it left out, by name, in topic order. An absent topic is in neither list.
    pub excluded: Vec<String>,
    /// All of that as the report's sentence.
    pub text: String,
}

/// One entry on a strengths or weaknesses list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FindingView {
    /// The contract's name, as [`NumberRow::name`].
    pub metric: String,
    /// What is established about it, and by how much.
    pub text: String,
}

/// One golfer's graded profile, each club in bag order and then the player. A golfer with no shots, or
/// an id nobody holds, answers an empty profile. `Err` is only a profile that fails its own check, which
/// is a bug in a builder and not a fact about the golfer.
pub fn graded_profile(root: String, player_id: String) -> Result<ProfileView, String> {
    crate::profile::graded(&root, &player_id)
}
