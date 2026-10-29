//! What each tool is for, in the words a model picks it by. `contracts/tool_descriptions.py`.
//! [M29 P9]
//!
//! A model picks a tool by reading its description and nothing else, so these carry when to call it
//! (in the user's words), what a miss looks like, and for the per-golfer tools the refusals, said
//! again at the point of use. Python's module doc is the long form. Plain strings, as there: none
//! states a count or a band, which is what keeps this file out of the drift `caveats` guards against.
//!
//! The eleven Python has are its strings, written as `concat!` of the same fragments so a line here
//! can be held against a line there; `tests/surface.rs` compares them with what frozen Python's
//! server sent. Ten are Python's exactly, and one sentence of `GET_CLUB_PROFILE` is Rust's,
//! declared there (M29 P15).
//! `GET_STRIKE_PROFILE` has no Python twin (M37's graded profile has no Python behind it) and is not
//! in the recorded surface.

pub const LIST_SESSIONS: &str = concat!(
    "List golf sessions, newest first, with each swing's score and lead coaching point. ",
    "Call this first when the user asks about their golf generally, refers to a session ",
    "by day ('yesterday', 'last time'), or asks what data exists — it is the cheapest way ",
    "to find the session and swing ids every other tool needs. Returns one entry per ",
    "session with its swings; a swing whose status is not 'done' has no score yet, and ",
    "'stale' means the video was re-uploaded after the result was computed."
);

pub const GET_SWING: &str = concat!(
    "Get one swing's full analysis: every scored checkpoint with the tour band it was ",
    "judged against and the percentile it sits at, the ranked coaching tips, any ",
    "checkpoints that could not be measured, and the launch-monitor shot if one was ",
    "attached. It also returns `population`: where the swing as a whole sits against a ",
    "corpus of tour swings, which is context rather than a verdict — nothing in it has a ",
    "band and unusual is not bad. Call this whenever the user asks how a specific swing ",
    "went or what to work ",
    "on — it is the only tool that returns mechanics. Needs a session_id and swing_id ",
    "from list_sessions; if no such swing exists it says so rather than returning data."
);

pub const GET_SESSION_SUMMARY: &str = concat!(
    "Aggregate one session across both axes: mean/best/worst score, how often each ",
    "checkpoint passed, which checkpoints went unmeasured, and averages over the ",
    "launch-monitor shots attached to that session's swings. Call this when the user asks ",
    "how a whole session or day went, rather than fetching every swing individually. ",
    "Shots flagged as uncertain are counted but excluded from the averages, and so are the ",
    "carry and total distance of a swing the golfer confirmed was a mishit (only that ",
    "manual verdict, not the automatic per-club rule get_bag_profile uses). A session ",
    "with nothing analyzed yet returns zero counts, which is a real answer; only a ",
    "session id that does not exist at all reports a miss."
);

pub const GET_RECENT_SHOTS: &str = concat!(
    "Get the most recent launch-monitor shots, newest first, across every configured ",
    "source. Call this when the user asks about ball flight, distances, club speed or ",
    "strike quality without naming a specific swing. Each shot carries needs_review and ",
    "parse_confidence, because these numbers are read off a photograph of the simulator ",
    "screen by OCR — treat a flagged shot's figures as uncertain rather than as fact."
);

pub const GET_SHOT_BY_ID: &str = concat!(
    "Get one launch-monitor shot by its id, with the full set of metrics and the ",
    "confidence of the OCR parse that produced them. Call this to follow up on a specific ",
    "shot surfaced by get_recent_shots or attached to a swing by get_swing. If no shot ",
    "has that id it says so rather than returning data."
);

pub const GET_GOLFER_PROFILE: &str = concat!(
    "Get one golfer's own baseline: their typical value and spread for each measured ",
    "metric, whether their miss is repeatable or scattered, and where their typical value ",
    "sits in the tour population. Call this whenever the user asks what they usually do, ",
    "what their tendency is, whether something is improving, or why a fault keeps ",
    "happening — it is the only tool that judges a golfer against themselves rather than ",
    "against tour bands. Takes a name as typed ('Aaron') or a stored id ('aaron'). ",
    "IMPORTANT: any figure the sample size cannot support is absent rather than flagged, ",
    "and the `withheld` list says what it is waiting for. Report those refusals as ",
    "refusals; do not compute the missing figure from the per-session counts beside it."
);

pub const GET_SHOT_TRENDS: &str = concat!(
    "Get one metric's per-session history for one golfer, oldest first, to answer whether ",
    "it has moved. Covers the pose metrics (tempo, head sway, hip sway, finish balance and ",
    "the rest) as well as the launch-monitor ones, despite the name. Call this for ",
    "'is my tempo getting better', 'has my face angle changed since last month'. Optional ",
    "`days` limits the window; omit it for the whole history. IMPORTANT: `ready` is false ",
    "until there are enough swings AND enough separate sessions for a trend to mean ",
    "anything, and while it is false every per-session mean is absent and no direction may ",
    "be described — the session counts are the evidence for the refusal, not a series."
);

pub const COMPARE_SESSIONS: &str = concat!(
    "Hold two of one golfer's sessions against each other: each session's mean score, and ",
    "each metric's sample count and mean in both. Call this for 'was today better than ",
    "last time' or 'compare Tuesday to Thursday'. Needs two session ids from ",
    "list_sessions. IMPORTANT: a per-metric mean is present only when that session on its ",
    "own carried enough swings to support one, so at small sample sizes the honest answer ",
    "is a pair of counts and no delta. No difference here is tested for significance, and ",
    "a session can score well while contributing no samples at all — read the `note`."
);

pub const GET_BAG_PROFILE: &str = concat!(
    "Get one golfer's whole bag: every club they have hit or declared, how far they hit it, how ",
    "repeatable that is, and the physical club recorded in each slot. Call this when the user ",
    "asks about their distances, their gapping, which club they hit how far, or what is in the ",
    "bag. Takes a name as typed ('Aaron') or a stored id ('aaron'). IMPORTANT: two different ",
    "silences live in this payload and they need opposite answers. An empty club list means no ",
    "swing on record names a club at all — the fix is tagging swings, NOT hitting more balls, so ",
    "do not answer it by telling the golfer to go and practise. A club that is listed but ",
    "withholds a figure has too few shots for that claim; `withheld` says what it is waiting for, ",
    "and that one does mean go and hit some. Report either as a refusal and never compute the ",
    "missing figure from the counts sitting beside it."
);

/// Python's sentence about the two counts, which Rust's corpus made false: a photo-only entry is one
/// of a club's `n_swings` with no clip (ADR-024's M35 addendum, the plan's P13 finding 5). The
/// surface's fourth declared difference (`tests/surface.rs`) is this fragment replaced by
/// [`CLUB_PROFILE_N_SWINGS`] in the one description, and nothing else. [M29 P15]
pub const CLUB_PROFILE_N_SWINGS_FROZEN: &str = "while `n_swings` counts clips.";

/// Rust's spelling of that fragment, which [`GET_CLUB_PROFILE`] carries once.
pub const CLUB_PROFILE_N_SWINGS: &str = "while `n_swings` counts swings, with or without a clip.";

/// Python's description but for the two counts' fragment ([`CLUB_PROFILE_N_SWINGS_FROZEN`]). The
/// line holding it is split in two here, so every other line still holds against Python's.
pub const GET_CLUB_PROFILE: &str = concat!(
    "Get one club of one golfer's: carry and total distance, start line, ball speed and launch ",
    "angle, each with its own sample count and its own refusal, plus the make, model and loft ",
    "declared for the club in that slot. Call this for 'how far do I hit my seven iron', 'what is ",
    "my gap wedge carrying', or anything about one club rather than the whole bag. The club is ",
    "free text — '7i', '7 iron', 'seven iron', 'driver', 'pw' all reach the same club; 'wedge' ",
    "and 'iron' name a category rather than a club and are refused rather than guessed at. ",
    "IMPORTANT: read these fields as they are named. `start_line_offline_yds` is where the ball ",
    "STARTED, in yards, projected out to the carry — it is not where the ball landed and the ",
    "curve is not in it; the curve is `face_to_path_deg`, in degrees. `n_shots` caps every ",
    "distance and launch figure, because those are read off a photograph of the simulator screen ",
    "while `n_swings` counts swings, with or without a clip. Nothing here says how far a golfer ",
    "should hit a club, so a ",
    "distance never earns a bias finding — only a repeatability one. And a club with no bag entry ",
    "has no loft on record: refuse loft and fitting questions about it rather than filling the gap ",
    "yourself. A loft that IS on record is the manufacturer's published figure unless the golfer ",
    "says they measured it, so it describes the model and not a club that has been bent."
);

pub const SIMULATE_FLIGHT: &str = concat!(
    "Fly the ball for one swing's shot: the carry, apex, hang time, descent angle and how far ",
    "offline it finishes, integrated from the launch conditions the simulator printed. Call this ",
    "when the user asks what a shot did in the air — how high, how far it carried, whether it ",
    "curved — rather than what the screen printed, which is get_shot_by_id. **Every number it ",
    "returns is SIMULATED and none of them was measured**; say so, and never average one with a ",
    "printed figure or present it as what the launch monitor recorded. Most shots cannot be ",
    "flown: the screen prints no spin on them, and the spin is solved backwards from the printed ",
    "carry where that is possible and refused where it is not. A refusal comes back with ",
    "`flew: false` and the reason, which is a fact about the shot and not a fault — never answer ",
    "one by telling the golfer to film the swing again, because the video is not an input to any ",
    "of this. Read `caveats` before quoting any number: the model is run outside the range its ",
    "coefficients were published for on every shot on file. Needs a session_id and swing_id from ",
    "list_sessions; if no such swing exists it says so rather than returning data."
);

/// The twelfth tool, M37's graded profile, which no Python server offers. Written at M29 P15 against
/// the answer it describes (`golf_mcp::strike`, `contracts::strike`'s module doc): each of the
/// three states a topic can be in is named here, because a model that reads an absent topic or a
/// withheld share as a poor grade gives the golfer a confident wrong instruction, and the payload
/// carries no note of its own (it is `golf-core profile --json`'s shape, whole).
pub const GET_STRIKE_PROFILE: &str = concat!(
    "Get one golfer's graded strike profile, read from their launch-monitor shots: for each club ",
    "with a shot on record, the ball-speed baseline, the carry, start-line and face-to-path ",
    "dispersion, a grade per strike topic, the blend of those grades, and the ranked weaknesses ",
    "and strengths; then the same for the golfer across every club, each shot judged against its ",
    "own club. Call this when the user asks what they do well or badly at impact, how good their ",
    "strike is with a club, or what to work on from their shot data rather than from a swing ",
    "video, which get_swing answers. Takes a name as typed ('Aaron') or a stored id ('aaron'). A ",
    "grade is the share of a club's shots that met the topic's criterion, with its 95% interval; ",
    "a blend averages the graded topics alone. IMPORTANT: read a missing grade for what it is. A ",
    "topic absent from `topics` cannot be graded from what the screen prints (strike location ",
    "needs a measured face-impact offset, never the printed CENTER, HEEL or TOE word, and a shape ",
    "needs shots practised as that shape), so it is not a weakness. A withheld grade has too few ",
    "shots and says how many it has and needs. A blend is absent when nothing is graded, and a ",
    "golfer with no shot photo has no clubs and no topics at all. Report each as exactly that, ",
    "never as a poor grade, and never fill in a grade, share or blend yourself. Shots hit as ",
    "drills count toward nothing here."
);

#[cfg(test)]
mod tests {
    use super::*;

    /// The fragments join with a space, as Python's adjacent literals do: no run of two and none
    /// missing at a seam, which a dropped trailing space in one fragment would show as.
    #[test]
    fn no_fragment_seam_lost_or_doubled_a_space() {
        for text in [
            LIST_SESSIONS,
            GET_SWING,
            GET_SESSION_SUMMARY,
            GET_RECENT_SHOTS,
            GET_SHOT_BY_ID,
            GET_GOLFER_PROFILE,
            GET_SHOT_TRENDS,
            COMPARE_SESSIONS,
            GET_BAG_PROFILE,
            GET_CLUB_PROFILE,
            SIMULATE_FLIGHT,
            GET_STRIKE_PROFILE,
        ] {
            assert!(!text.contains("  "), "{text}");
            assert!(!text.ends_with(' '), "{text}");
        }
    }

    /// The fourth declared difference is the one fragment, carried once. [M29 P15]
    #[test]
    fn the_club_profile_carries_rusts_count_fragment_once_and_not_pythons() {
        assert_eq!(GET_CLUB_PROFILE.matches(CLUB_PROFILE_N_SWINGS).count(), 1);
        assert!(!GET_CLUB_PROFILE.contains(CLUB_PROFILE_N_SWINGS_FROZEN));
    }
}
