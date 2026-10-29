//! The standing caveats every reader of this data has to carry. `contracts/caveats.py`. [M29 P9]
//!
//! The text exists because an LLM presents whatever it is handed as fact, and every number this
//! repo knows to be provisional — a clamped percentile, an unmeasurable checkpoint, an OCR'd shot, a
//! simulated flight — has to arrive with its uncertainty attached. Python's module doc says why the
//! counts and names are derived rather than typed (it claimed five checkpoints through M6.5 while
//! six ran); the same holds here, so every count and list below is read off `contracts`' registries
//! as Python's is read off its own, and wrapped by [`crate::textwrap::fill`] at Python's width.
//!
//! **Since M29 this is the copy that reaches a model** (the plan's D10): `golf-mcp` sends it as the
//! connect-time `instructions` and serves the named sentences as the `golf-coach://caveats`
//! resource, and P16–P17 move the LLM side to read them there. Python's copy is frozen (ADR-035
//! clause 4) and keeps its wording; this one departs from it once, on purpose, and says where.
//!
//! # The one departure: a photo worth retaking (the plan's C13)
//!
//! Rust's [`UNSCORED_REASONS`] has two rows frozen Python's has not, `printed_blank` and `misread`,
//! the photo-side reasons (M32). Derived over Rust's table, Python's sentence would list
//! `printed_blank` among the "not capture problems … the footage is not what went wrong" and then
//! call `misread` "a clip worth taking again", which tells a golfer to re-film a swing because a
//! photo of the screen was unreadable. [`unscored_reasons_bullet`] therefore says the photo as well
//! as the footage, and names the photo-side reasons a sharper photo may fix. [`Wording::Frozen`] is
//! Python's sentence over Python's rows, kept runnable because it is what the recorded surface
//! holds and what `tests/surface.rs` declares the difference against.
//!
//! # And one verb: the bag's mishit bullet [M29 P34]
//!
//! `READING_A_BAG`'s mishit bullet tells a model that one `flag_mishit.py --clear` puts a shot back
//! in the average. M29 P34 deleted that script, and `golf-core flag-mishit` is what records a
//! verdict now, so [`mishit_exclusion_bag_bullet`] names the verb under [`Wording::Rust`] and the
//! script under [`Wording::Frozen`], which is what the recorded surface holds. The bullet ships
//! only on the registry shape, so the registry-less briefing departs in the one bullet above.

use contracts::checkpoints::CHECKPOINT_REGISTRY;
use contracts::mishit::MISHIT_EXCLUDED_METRICS;
use contracts::pivots::PIVOT_MEASUREMENT_REGISTRY;
use contracts::placements::POPULATION_PLACEMENT_REGISTRY;
use contracts::unscored::{UnscoredReason, UNSCORED_REASONS};

use crate::textwrap::fill;

/// `_WIDTH`: the hand-written blocks are at this width, so the derived ones are wrapped to it.
const WIDTH: usize = 95;

const COUNT_WORDS: [&str; 13] = [
    "no", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten", "eleven",
    "twelve",
];

/// `_count_word`: `6` -> "six", and the digits past twelve. Public for `tests/docs.rs`, which spells
/// the tool count the way the prose it pins does. [M29 P15]
pub fn count_word(n: usize) -> String {
    COUNT_WORDS
        .get(n)
        .map_or_else(|| n.to_string(), |w| (*w).to_string())
}

/// `_and_list`: "a", "a and b", "a, b and c", with no serial comma.
fn and_list(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [one] => one.clone(),
        [head @ .., last] => format!("{} and {last}", head.join(", ")),
    }
}

fn ticked(name: &str) -> String {
    format!("`{name}`")
}

/// A bullet: wrapped at the width with `- ` before its first line and two spaces before the rest.
fn bullet(text: &str) -> String {
    fill(text, WIDTH, "- ", "  ")
}

fn checkpoint_count() -> String {
    count_word(CHECKPOINT_REGISTRY.len())
}

/// A reason's wire name, `no_band`: serde's spelling, which is Python's enum value.
fn wire(reason: UnscoredReason) -> String {
    match serde_json::to_value(reason) {
        Ok(serde_json::Value::String(name)) => name,
        other => unreachable!("a reason serializes as its name, not {other:?}"),
    }
}

/// Whether `reason` is one of the photo-side rows, the ones Rust's table has and frozen Python's
/// has not. Derived from the table's own layout rule (`contracts::unscored`'s
/// [`UNSCORED_REASONS`] doc): those rows come last, after `Unrecorded`, so Python's rows stay an
/// unbroken prefix. `photo_side_rows_are_the_two_m32_added` pins which they are today.
pub fn is_photo_side(reason: UnscoredReason) -> bool {
    let at = |wanted: UnscoredReason| {
        UNSCORED_REASONS
            .iter()
            .position(|(r, _)| *r == wanted)
            .expect("every reason has a row")
    };
    at(reason) > at(UnscoredReason::Unrecorded)
}

/// `TWO_AXES`: what the two scores mean and why they are not interchangeable (ADR-009).
pub fn two_axes() -> String {
    let labels: Vec<&str> = CHECKPOINT_REGISTRY.iter().map(|s| s.label).collect();
    fill(
        &format!(
            "Two independent axes, and they answer different questions. **Mechanics** come from \
             pose analysis of a face-on video: {} checkpoints ({}) scored against bands derived \
             from hand-annotated tour swings, plus the percentile the swing sits at in that \
             population. **Outcome** comes from the launch monitor: club and ball speed, launch, \
             spin, carry. A swing can score well on one and badly on the other; say which you are \
             talking about.",
            checkpoint_count(),
            labels.join(", "),
        ),
        WIDTH,
        "",
        "",
    )
}

/// `ONLY_CHECKPOINTS_ARE_JUDGED`: the clause bounding what a reader may judge, for a consumer to
/// compose its own sentence around (Python's `mcp/query.py` and `feedback/conversation.py` do).
pub fn only_checkpoints_are_judged() -> String {
    format!(
        "the {} entries in `checkpoints` are the only things on this swing anyone has a reference \
         population for",
        checkpoint_count()
    )
}

/// `PLACEMENTS_ARE_NOT_SCORES`: the same bound for the population placements.
pub fn placements_are_not_scores() -> String {
    format!(
        "the {} entries in `population` are distances from a population of tour professionals \
         rather than scores — unusual is not bad, no band exists for any of them, and none of them \
         is included in `overall_score`",
        count_word(POPULATION_PLACEMENT_REGISTRY.len())
    )
}

/// `PIVOTS_ARE_INTERIM`: the same bound for the pivot measurements.
pub fn pivots_are_interim() -> String {
    format!(
        "the {} entries in `rotation` are an interim, uncalibrated 2-D-per-view reading of the \
         swing's rotation — foreshortened, never blended or compared across the two camera views, \
         not judged and not part of `overall_score` — until fiducial calibration lands",
        count_word(PIVOT_MEASUREMENT_REGISTRY.len())
    )
}

/// `ALIGNMENT_CAVEAT`: attached to an alignment tier below FULL, with the tier's own `summary`
/// clause put where `{summary}` is, as Python's `str.format` does.
pub const ALIGNMENT_CAVEAT: &str = "The two camera views were {summary}, not on every instant. \
     Frame correspondence is exact at those anchors and interpolated between them, so do not \
     describe the side-by-side video as synchronized throughout, and do not read fine timing \
     differences off it.";

/// `_LESS_IS_NOT_BETTER`.
fn less_is_not_better() -> String {
    let two_sided: Vec<String> = CHECKPOINT_REGISTRY
        .iter()
        .filter(|s| !s.one_sided)
        .map(|s| ticked(s.name))
        .collect();
    let one_sided: Vec<&str> = CHECKPOINT_REGISTRY
        .iter()
        .filter(|s| s.one_sided)
        .map(|s| s.label)
        .collect();
    bullet(&format!(
        "**Less is not better on every checkpoint.** {} are two-sided (`one_sided` is false): too \
         little hip travel fails just as too much does, because some lateral hip movement is the \
         weight shift a swing needs, and a head that hangs back through impact fails as a head \
         that drifts forward with the hips does. On the one-sided checkpoints — {} — below the \
         band is the good side. Read `one_sided` before describing a low number as a strength.",
        and_list(&two_sided),
        one_sided.join(", "),
    ))
}

/// Which wording of the two declared bullets: this module's, or frozen Python's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wording {
    /// Over every row of Rust's table, saying the photo as well as the footage (C13), and naming
    /// `golf-core flag-mishit` where a verdict is recorded (P34).
    Rust,
    /// `_UNSCORED_REASONS_PROSE` exactly: Python's sentence over Python's rows, the photo-side ones
    /// left out; and `_MISHIT_EXCLUSION_BAG` naming `flag_mishit.py`. What
    /// `spec/vectors/mcp/surface/server.json` recorded.
    Frozen,
}

/// The `unscored` reasons bullet of [`reading_this_data_honestly`], in either wording.
///
/// Which reasons are not the golfer's capture is derived from `refilming_helps`, as Python derives
/// it, because a hand-written list would be wrong the first time a reason is added.
pub fn unscored_reasons_bullet(wording: Wording) -> String {
    let rows = UNSCORED_REASONS
        .iter()
        .filter(|(reason, _)| wording == Wording::Rust || !is_photo_side(*reason));
    let not_capture: Vec<String> = rows
        .clone()
        .filter(|(_, spec)| !spec.refilming_helps)
        .map(|(reason, _)| ticked(&wire(*reason)))
        .collect();
    let retake_photo: Vec<String> = rows
        .filter(|(reason, spec)| spec.refilming_helps && is_photo_side(*reason))
        .map(|(reason, _)| ticked(&wire(*reason)))
        .collect();
    let not_capture = and_list(&not_capture);
    let text = match (wording, retake_photo.as_slice()) {
        (Wording::Frozen, _) | (Wording::Rust, []) => format!(
            "Each entry in `unscored` carries a `reason`, and it decides what to say next. \
             {not_capture} are not capture problems — the footage is not what went wrong, so a \
             better clip would not change the answer and you must never respond to one of those \
             by telling the golfer to re-film, steady the camera or shoot the swing again. Every \
             other reason is a clip worth taking again. The `refilming_helps` flag says which is \
             which; do not infer it from the checkpoint name."
        ),
        (Wording::Rust, photos) => {
            let (verb, noun) = if photos.len() == 1 {
                ("is", "a photo")
            } else {
                ("are", "photos")
            };
            format!(
                "Each entry in `unscored` carries a `reason`, and it decides what to say next. \
                 {not_capture} are not capture problems — neither the footage nor the photo of the \
                 screen is what went wrong, so a better clip or photo would not change the answer \
                 and you must never respond to one of those by telling the golfer to re-film, \
                 steady the camera, shoot the swing again or photograph the screen again. Every \
                 other reason is worth capturing again: {} {verb} {noun} of the screen worth \
                 retaking, sharper and square-on, and every other one is a clip worth taking \
                 again. The `refilming_helps` flag says which is which; do not infer it from the \
                 checkpoint name.",
                and_list(photos)
            )
        }
    };
    bullet(&text)
}

/// `_PLACEMENTS_ARE_NOT_SCORES`, the bullet.
fn placements_bullet() -> String {
    let names: Vec<String> = POPULATION_PLACEMENT_REGISTRY
        .iter()
        .map(|s| ticked(s.name))
        .collect();
    bullet(&format!(
        "**The `population:golfdb` entries are placements, not scores.** {} say how far this \
         swing sits from a population of tour professionals — as a combination of metrics, and as \
         a motion. Every clip behind them is a tour swing, so these models know the shape of \
         swings that work and nothing whatever about swings that do not: a large value means \
         **unusual**, which is a reason to look and never a fault to report. No band exists for \
         any of them, none is part of `overall_score`, and their percentiles clamp at 10 and 90 \
         exactly as the checkpoint ones do. Naming what a swing does unusually is useful; calling \
         it wrong is not supported by anything behind these numbers.",
        and_list(&names)
    ))
}

/// `_PLACEMENTS_PER_VIEW`.
fn placements_per_view() -> String {
    let uncalibrated: Vec<String> = POPULATION_PLACEMENT_REGISTRY
        .iter()
        .filter(|s| !s.calibrated)
        .map(|s| ticked(s.name))
        .collect();
    bullet(&format!(
        "{} are **not calibrated**, and the others are. A Q is the part of the motion the tour \
         basis cannot represent at all, and it over-flags any golfer that basis never saw — which \
         is every golfer using this system. It also cannot tell an unusual swing from a \
         mis-detected one, so a large Q beside an ordinary T2 more often means the swing was read \
         badly than swung badly. Read each Q beside its own T2, never alone. And the face-on and \
         down-the-line numbers come from two bases fitted to two cameras: each is compared \
         against its own population and never against the other, so a difference between the two \
         is not a finding about the swing. A down-the-line placement is still not spine angle, \
         swing plane or club path: those live in the depth direction one camera cannot resolve, \
         and none of them is measured here.",
        and_list(&uncalibrated)
    ))
}

/// `_PIVOTS_ARE_INTERIM_PROSE`.
fn pivots_bullet() -> String {
    let names: Vec<String> = PIVOT_MEASUREMENT_REGISTRY
        .iter()
        .map(|s| ticked(s.name))
        .collect();
    bullet(&format!(
        "**The `rotation` entries are an interim reading, not a score.** {} say how far the \
         shoulder and hip centres travelled off their address position and how the shoulder line \
         turned, read off one uncalibrated phone's image plane — a 2-D projection of a 3-D turn, \
         foreshortened by wherever the camera stood. The face-on and down-the-line numbers are \
         two different instruments and must never be blended or compared to each other. None has \
         a band, none contributes to `overall_score`, and career mode reports them unavailable \
         rather than treating the camera's own noise as a repeatability statistic. Fiducial \
         calibration is what promotes one from interim to judged; each row's own `detail` says \
         what is still missing until then.",
        and_list(&names)
    ))
}

/// `_ONLY_THESE_FUNDAMENTALS`.
fn only_these_fundamentals() -> String {
    bullet(&format!(
        "Only {} fundamentals are measured, all from the face-on view, and all of them are \
         lateral (side-to-side) or timing quantities. Spine angle, hip **rotation**, swing plane \
         and club path are NOT measured here — they need a second calibrated view or club \
         detection, neither of which exists yet. The hip checkpoints measure how far the hips \
         travel sideways, never how far they turn, and never in which direction. Do not infer \
         them from what is available.",
        checkpoint_count()
    ))
}

/// `_MISHIT_HEAD`: the excluded metrics named from `MISHIT_EXCLUDED_METRICS`, sorted as Python
/// sorts its frozenset.
fn mishit_head() -> String {
    let mut names: Vec<&str> = MISHIT_EXCLUDED_METRICS.to_vec();
    names.sort_unstable();
    let names: Vec<String> = names.into_iter().map(ticked).collect();
    format!(
        "A **mishit** — a shot carrying far below that club's own median, a top or a duff — is \
         held out of {} and no other metric: the same shot still counts toward ball speed, \
         launch, start line and every pose checkpoint, because the swing was real and only its \
         distance is meaningless.",
        and_list(&names)
    )
}

/// `_MISHIT_EXCLUSION_HISTORY`.
fn mishit_exclusion_history() -> String {
    bullet(&format!(
        "{} `mishits_excluded` counts them across the whole history, and while it is above zero \
         those two distance metrics carry a caveat that names the count. This is not a \
         sample-size refusal — the golfer confirmed the shot or the per-club rule flagged it, and \
         more swings do not change it.",
        mishit_head()
    ))
}

/// `_MISHIT_EXCLUSION_BAG`, in either wording: the verdict is recorded by the script frozen Python
/// names or by the verb that replaced it (the module doc).
///
/// Rust's says "clearing a borderline shot with" where Python's says "one … on a borderline shot"
/// for a reason of layout: at this width Python's word order puts the line break inside
/// `flag-mishit`, where `fill` breaks a hyphenated word as Python's `textwrap` does, and a command a
/// model may quote should not reach it as `flag- mishit`. The unit test below holds the command
/// whole on one line, so a registry change that moves the break is caught.
pub fn mishit_exclusion_bag_bullet(wording: Wording) -> String {
    let verdict = match wording {
        Wording::Rust => {
            "so clearing a borderline shot with `golf-core flag-mishit --clear` puts it back in \
             the average."
        }
        Wording::Frozen => {
            "so one `flag_mishit.py --clear` on a borderline shot puts it back in the average."
        }
    };
    bullet(&format!(
        "{} `mishits` counts them for the club, `mishit_refs` names every held-out shot, \
         `mishits_excluded` is the bag-wide total, and `mishits_unconfirmed` is how many the \
         automatic rule flagged that the golfer has not yet confirmed or cleared. The verdict \
         overrides the rule either way, {verdict}",
        mishit_head()
    ))
}

/// `READING_THIS_DATA_HONESTLY` in either wording of its `unscored` bullet: the provisional-data
/// warnings, one per way this repo can be honestly wrong.
pub fn reading_this_data_honestly_worded(wording: Wording) -> String {
    format!(
        "Reading this data honestly:\n\
         \n\
         - `unscored` lists checkpoints that could not be scored, not ones that failed. They are\n  \
         excluded from `overall_score` rather than counted as zero, so a swing with an unscored\n  \
         checkpoint was judged on fewer fundamentals than one without. Say so if it comes up.\n\
         {unscored}\n\
         - `percentile` is informational and clamps at the band edges, so a failing checkpoint reports\n  \
         about 90 (or about 10, when it missed on the low side of a two-sided metric) whether it missed\n  \
         narrowly or hugely. A low percentile is therefore not by itself good news — read `passed` for\n  \
         the verdict and rank severity by `score`, never by percentile.\n\
         {less}\n\
         - `head_stays_back` is recorded as a **negative** number, and that is a frame of reference rather\n  \
         than a fault: its band lives in a right-handed camera frame where more negative means more\n  \
         head-behind-hips separation. Never read its sign as bad news, and never quote the raw value to a\n  \
         golfer — the checkpoint's own `message` states the magnitude in the right direction. This is also\n  \
         the one checkpoint that can be missing because nobody said who swung: when a swing has no golfer\n  \
         attributed, it appears in `unscored` rather than being scored on a guess.\n\
         {placements}\n\
         {per_view}\n\
         {pivots}\n\
         - `needs_review` on a shot means its numbers were read off a photograph by OCR and the parse\n  \
         was flagged. Do not quote a flagged shot's figures as fact; say the reading is uncertain.\n\
         - `alignment_caveat`, when present, means the two camera views were anchored on some swing\n  \
         instants and interpolated between them. Do not describe the side-by-side video as\n  \
         synchronized throughout, and do not read fine timing differences off it.\n\
         {fundamentals}\n\
         \n\
         Only a handful of sessions and shots are recorded so far. Do not describe a trend or a pattern\n\
         from the numbers in front of you — there is not enough of them for one, and any statement about\n\
         change over time has to come from something that counted the swings behind it.",
        unscored = unscored_reasons_bullet(wording),
        less = less_is_not_better(),
        placements = placements_bullet(),
        per_view = placements_per_view(),
        pivots = pivots_bullet(),
        fundamentals = only_these_fundamentals(),
    )
}

/// `READING_THIS_DATA_HONESTLY`, as `golf-mcp` sends it: [`Wording::Rust`].
pub fn reading_this_data_honestly() -> String {
    reading_this_data_honestly_worded(Wording::Rust)
}

/// `READING_A_PERSONAL_HISTORY`: the discipline the career tools need, and only they do — a figure
/// the sample cannot support is absent on purpose, and must not be reconstructed.
pub fn reading_a_personal_history() -> String {
    format!(
        "Reading one golfer's own history:\n\
         \n\
         - A figure the sample size cannot support is **absent, not flagged**. `null` here means the guard\n  \
         refused the claim, and `withheld` says what it is waiting for. Report the refusal in words. Do\n  \
         NOT reconstruct the missing figure from the per-session counts sitting next to it — those counts\n  \
         are the evidence for the refusal, not a series to average.\n\
         - `not_established` is not \"no\" and not \"you are fine\". It means there is enough data to ask and\n  \
         the data does not settle it. An interval straddling a threshold is consistent with a real effect\n  \
         and with none.\n\
         - `bias` and `scatter` are two separate findings and the contrast between them is the whole point:\n  \
         a miss that repeats at the same size points at something set before the swing, a miss that moves\n  \
         points at timing and release. Never collapse the pair into one verdict, and never name a specific\n  \
         cause — grip, wrist angle, release and clubface are not measured here.\n\
         - A tour band describes a population of professionals, and `population_players` says how large it\n  \
         is for that metric — it is not the same for all of them. `outside` is a distance from that\n  \
         population, NOT a fault: holding an amateur to a tour p10-p90 fails everyone forever, which is\n  \
         why this system compares a golfer to themselves first. Read `outside_by` for the direction; on a\n  \
         one-sided magnitude like head sway or finish balance, below the band is the good side.\n\
         - `n` counts distinct swings, not swing directories. A session can score perfectly well and still\n  \
         contribute no samples, because its clips were re-uploads of a swing already counted elsewhere.\n\
         {}",
        mishit_exclusion_history()
    )
}

/// `READING_A_SIMULATED_FLIGHT`: how to read a number nothing measured. Prose rather than derived,
/// for Python's reason: the rule is about a *kind* of number, not about today's flight rows.
pub const READING_A_SIMULATED_FLIGHT: &str = "\
Reading a simulated ball flight:

- **Nothing in it was measured.** The flight is integrated from the launch conditions the
  simulator printed, so the carry, apex, hang time, descent angle and landing offline are all a
  model's output. Say \"simulated\" whenever you quote one, never average one with a printed
  figure, and never present one as something the launch monitor recorded.
- **The spin is often solved rather than read.** Where the screen printed no spin it is recovered
  backwards from the printed carry — it is the spin this model needs in order to agree with that
  number. Where that happened, the simulated carry matching the printed one is not a check of
  anything: it is the input read back out. `spin.source` says which case you are in, and
  `comparison` carries the sentence for it.
- **A refused flight is a fact about the shot and never about the clip.** A carry no spin can
  reach, a club whose loft nobody declared, a screen that printed neither a spin nor a carry to
  solve from — those are the reasons, and re-filming the swing fixes none of them. Say what is
  missing, and where it is a declared loft say that the repair is a bag entry rather than a range
  session.
- **The landing offline is not the printed one.** What the screen prints is where the ball
  *started*, projected out to the carry; the simulated number is where the model has it finishing.
  The difference between them is the curve, and it is an error in neither. Where the curve could
  not be resolved the flight is drawn flat and that number is withheld rather than guessed.
- **The apex, hang time and descent angle have no printed counterpart at all**, so nothing on file
  can check them. Quote them as what the model says and never describe one as confirmed.
- `caveats` is not a footnote. Read it before quoting any figure and pass on what it says — the
  first of them is about the range the model's coefficients were published for, and it applies to
  every shot on file.";

/// `READING_A_BAG`: what the per-club tools add, beside [`reading_a_personal_history`] and never
/// instead of it. [`Wording::Rust`], as `golf-mcp` sends it.
pub fn reading_a_bag() -> String {
    reading_a_bag_worded(Wording::Rust)
}

/// `READING_A_BAG` in either wording of its mishit bullet.
pub fn reading_a_bag_worded(wording: Wording) -> String {
    format!(
        "Reading one golfer's bag, per club:\n\
         \n\
         - **An empty bag profile is not a refusal.** No clubs listed means no swing on record names a club\n  \
         — the club tag is newer than the swings. Say that, and say the fix is tagging those swings; do\n  \
         NOT answer it by telling the golfer to hit more balls. A club that *is* listed and withholds a\n  \
         figure is the other case, and that one is fixed by shots on that club.\n\
         - `n_swings` counts clips and `n_shots` counts photographs of the simulator screen, so `n_shots` is\n  \
         the ceiling on every distance and launch figure — carry, total, offline, ball speed and launch\n  \
         angle all come off that screen. A club filmed often and photographed rarely has plenty of swing\n  \
         history and almost no distance history.\n\
         - `start_line_offline_yds` is where the ball **started**, projected out to the carry — not where it\n  \
         landed. The simulator prints no landing offset, so nothing here knows it. The curve is a separate\n  \
         number, `face_to_path_deg`, in degrees.\n\
         - Nothing says how far a golfer *should* hit a given club, so a distance never earns a bias\n  \
         finding; only a repeatability one. `unavailable` carries that reason, and more swings never fix\n  \
         it.\n\
         - A club with no bag entry has no loft, make or model on record. That costs the loft and nothing\n  \
         else — its distance statistics are perfectly readable — but a loft, gapping-by-loft or fitting\n  \
         question about it must refuse rather than be answered from a specification nobody confirmed. A\n  \
         loft that is on record is the manufacturer's published one unless the golfer measured it\n  \
         (ADR-026 §1), so it describes the model rather than a club that has since been bent. Where a bag\n  \
         entry exists and a caveat says swings predate it, the numbers may pool two physical clubs under\n  \
         one name.\n\
         {}",
        mishit_exclusion_bag_bullet(wording)
    )
}

/// `_OPENING` in `mcp/server.py`: the one line about this server rather than the data.
const OPENING: &str = "Swing analysis and launch-monitor data from a home golf simulator.";

/// `instructions(career_tools=…)`: the connect-time briefing, matched to the tools offered.
///
/// The two per-golfer blocks ship only when a golfer registry is configured, because both are about
/// reading the refusals of tools a registry-less server does not offer; the flight block ships
/// always, because a flight reaches a model through `get_swing` too. `wording` is
/// [`Wording::Rust`] for what `golf-mcp` sends, and [`Wording::Frozen`] for the declaration's
/// other side.
pub fn instructions_worded(registry: bool, wording: Wording) -> String {
    let mut blocks = vec![
        OPENING.to_string(),
        two_axes(),
        reading_this_data_honestly_worded(wording),
        READING_A_SIMULATED_FLIGHT.to_string(),
    ];
    if registry {
        blocks.push(reading_a_personal_history());
        blocks.push(reading_a_bag_worded(wording));
    }
    format!("{}\n", blocks.join("\n\n"))
}

/// The briefing `golf-mcp` sends on connect.
pub fn instructions(registry: bool) -> String {
    instructions_worded(registry, Wording::Rust)
}

/// Every named sentence, by its Python constant's name, in `contracts/caveats.py`'s order: what the
/// `golf-coach://caveats` resource serves (the plan's D10), so the LLM side reads the prose here
/// and keeps no copy. `ALIGNMENT_CAVEAT` keeps its `{summary}` for the reader to fill.
pub fn named_sentences() -> Vec<(&'static str, String)> {
    vec![
        ("TWO_AXES", two_axes()),
        ("ONLY_CHECKPOINTS_ARE_JUDGED", only_checkpoints_are_judged()),
        ("PLACEMENTS_ARE_NOT_SCORES", placements_are_not_scores()),
        ("PIVOTS_ARE_INTERIM", pivots_are_interim()),
        ("ALIGNMENT_CAVEAT", ALIGNMENT_CAVEAT.to_string()),
        ("READING_THIS_DATA_HONESTLY", reading_this_data_honestly()),
        ("READING_A_PERSONAL_HISTORY", reading_a_personal_history()),
        (
            "READING_A_SIMULATED_FLIGHT",
            READING_A_SIMULATED_FLIGHT.to_string(),
        ),
        ("READING_A_BAG", reading_a_bag()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn photo_side_rows_are_the_two_m32_added() {
        let photo: Vec<UnscoredReason> = UNSCORED_REASONS
            .iter()
            .map(|(r, _)| *r)
            .filter(|r| is_photo_side(*r))
            .collect();
        assert_eq!(
            photo,
            [UnscoredReason::PrintedBlank, UnscoredReason::Misread]
        );
    }

    #[test]
    fn the_rust_bullet_sends_misread_to_the_photo_and_printed_blank_nowhere() {
        let bullet = unscored_reasons_bullet(Wording::Rust).replace("\n  ", " ");
        let (not_capture, rest) = bullet.split_once(" are not capture problems").unwrap();
        assert!(not_capture.contains("`printed_blank`"), "{bullet}");
        assert!(!not_capture.contains("`misread`"), "{bullet}");
        assert!(
            rest.contains("`misread` is a photo of the screen worth retaking"),
            "{bullet}"
        );
        assert!(!bullet.contains("Every other reason is a clip"), "{bullet}");
    }

    #[test]
    fn the_frozen_bullet_names_only_pythons_rows() {
        let bullet = unscored_reasons_bullet(Wording::Frozen);
        assert!(!bullet.contains("printed_blank") && !bullet.contains("misread"));
        assert!(bullet.contains("Every other reason is a clip worth taking again."));
    }

    #[test]
    fn the_bag_bullet_names_the_verb_and_the_frozen_one_the_script() {
        let wrapped = mishit_exclusion_bag_bullet(Wording::Rust);
        assert!(
            wrapped
                .lines()
                .any(|line| line.contains("`golf-core flag-mishit --clear`")),
            "the command is broken across lines:\n{wrapped}"
        );
        let ours = wrapped.replace("\n  ", " ");
        let frozen = mishit_exclusion_bag_bullet(Wording::Frozen).replace("\n  ", " ");
        assert!(
            ours.contains("clearing a borderline shot with `golf-core flag-mishit --clear` puts"),
            "{ours}"
        );
        assert!(!ours.contains("flag_mishit.py"), "{ours}");
        assert!(
            frozen.contains("one `flag_mishit.py --clear` on a borderline shot"),
            "{frozen}"
        );
        assert!(reading_a_bag().contains(&mishit_exclusion_bag_bullet(Wording::Rust)));
    }

    #[test]
    fn the_counts_are_the_registries() {
        assert!(two_axes().contains(&format!("{} checkpoints", checkpoint_count())));
        assert_eq!(count_word(6), "six");
        assert_eq!(count_word(13), "13");
        assert_eq!(
            and_list(&["a".into(), "b".into(), "c".into()]),
            "a, b and c"
        );
    }

    #[test]
    fn the_resource_names_every_sentence_once_and_the_briefing_holds_its_blocks() {
        let named = named_sentences();
        let mut names: Vec<&str> = named.iter().map(|(n, _)| *n).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), named.len());
        let briefing = instructions(true);
        for (name, text) in &named {
            let in_briefing = matches!(
                *name,
                "TWO_AXES"
                    | "READING_THIS_DATA_HONESTLY"
                    | "READING_A_PERSONAL_HISTORY"
                    | "READING_A_SIMULATED_FLIGHT"
                    | "READING_A_BAG"
            );
            assert_eq!(briefing.contains(text.as_str()), in_briefing, "{name}");
        }
        assert!(!instructions(false).contains(&reading_a_bag()));
    }
}
