//! `graded_profile`, and the gate on what it can say. [M38 P11]
//!
//! **The gate is the first two tests.** The phone's Dart spells no sentence about a grade (rule 8), so
//! every string the Profile tab shows comes from this view, and every string in this view has to appear
//! in what `golf-core profile` prints for the same corpus. So the view cannot say what the lab's report
//! does not, and a sentence reworded on one side alone stops the build.
//!
//! **What the gate cannot see is what the rest of the file is for.** A substring test passes a true
//! sentence on the wrong club, or a strength filed as a weakness. So [`every_field_sits_where_the_vector_says`]
//! holds the view's structure to each case's hand-worked `expected`, and the named pins below that hold the
//! states a screen has to tell apart.
//!
//! **Everything read off the six captures here is provisional: pasted copies** (rule 12). They are
//! chat-pasted JPEGs and not the originals, by M33's decision of 2026-11-04. Those files are read and never
//! written (rule 5), and a capture is always copied into the scratch root first.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::thread;

use app::api::history::{delete_shot, set_shot_club, set_shot_mishit, MishitVerdict};
use app::api::profile::{
    graded_profile, BlendView, ClubView, FindingView, NumberRow, PlayerView, ProfileView,
    ShapeView, ShareView, TopicView,
};
use app::api::record::{record_photo, PhotoIn, RecordStatus, ShotCard};
use app::api::review::{correct_shot, FieldEdit};
use app::api::setup::{
    create_golfer, get_state, init_state, set_bag_slot, set_state, AppState, Handedness,
    PracticeMode,
};
use app::{engine_versions, profile_view, Dirs};
use contracts::bag::Bag;
use contracts::career::CareerCorpus;
use contracts::club::parse_club;
use contracts::club_profile::BagProfile;
use contracts::strike::{Topic, RATE_MINIMUM_N};
use golf_core::profile::{run_profile, run_profile_and_bag_profile};
use golf_core::reports::club_profile::club_profile as club_report;
use golf_core::reports::profile::{criterion, profile as report, NOTHING_LISTED};
use golf_core::reports::{answer, DataDirs, Request, Verb};
use serde_json::{json, Value};
use storage::bag_store::BagStore;
use storage::corpus::read_corpus;
use tempfile::TempDir;

const CAPTURES: [&str; 6] = [
    "IMG_2889",
    "IMG_2891",
    "IMG_2892",
    "IMG_2893",
    "IMG_2894",
    "2026-10-07-1",
];
const TAKEN: &str = "2026-11-04T20:42:00-05:00";
const NOW: &str = "2026-11-04T20:43:00-05:00";
const LATER: &str = "2026-11-04T20:44:00-05:00";
/// How many saves a read is raced against, `tests/history.rs`' count.
const SAVES: u32 = 60;

// ---------------------------------------------------------------------------------- the vectors

fn family() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec/vectors/profile")
}

/// Every case under `spec/vectors/profile/<sub>/`, sorted, so a new case is gated the moment it is
/// committed. Empty fails here, since a walk that found nothing would pass everything.
fn cases(sub: &str) -> Vec<(String, Value)> {
    let dir = family().join(sub);
    let mut paths: Vec<PathBuf> = fs::read_dir(&dir)
        .unwrap_or_else(|failed| panic!("{}: {failed}", dir.display()))
        .map(|entry| entry.expect("an entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .collect();
    paths.sort();
    assert!(!paths.is_empty(), "{} holds no case", dir.display());
    paths
        .into_iter()
        .map(|path| {
            let text = fs::read_to_string(&path).expect("a committed case");
            let vector = serde_json::from_str(&text).expect("a committed case's JSON");
            (path.display().to_string(), vector)
        })
        .collect()
}

fn hand_case(name: &str) -> Value {
    let path = family().join("hand").join(format!("{name}.json"));
    let text = fs::read_to_string(&path).unwrap_or_else(|f| panic!("{}: {f}", path.display()));
    serde_json::from_str(&text).expect("a committed case's JSON")
}

fn input_of(vector: &Value) -> (CareerCorpus, Option<Bag>) {
    let corpus = serde_json::from_value(vector["input"]["corpus"].clone()).expect("a corpus");
    let bag = serde_json::from_value(vector["input"]["bag"].clone()).expect("a bag");
    (corpus, bag)
}

fn view_of(vector: &Value) -> ProfileView {
    let (corpus, bag) = input_of(vector);
    profile_view(&corpus, bag.as_ref()).expect("a profile that validates")
}

fn hand_view(name: &str) -> ProfileView {
    view_of(&hand_case(name))
}

/// `golf-core profile`'s text for a corpus, under a name no sentence here prints.
fn report_of(corpus: &CareerCorpus, bag: Option<&Bag>) -> String {
    let profile = run_profile(corpus, bag).expect("a profile that validates");
    report(&profile, "Golfer", None)
}

fn empty_corpus() -> CareerCorpus {
    serde_json::from_value(json!({"player_id": "nobody"})).expect("an empty corpus reads")
}

/// A case with two clubs and untagged shots, with every club taken off, which is the one state no
/// committed case reaches: shots on record and no club to list.
fn corpus_naming_no_club() -> CareerCorpus {
    let mut vector = hand_case("player-pools-two-clubs-and-untagged");
    for swing in vector["input"]["corpus"]["swings"]
        .as_array_mut()
        .expect("the case's swings")
    {
        swing["club"] = Value::Null;
    }
    input_of(&vector).0
}

// ------------------------------------------------------------------------------------- the gate

/// Every display string in a view, by the field holding it.
type Strings = BTreeMap<&'static str, Vec<String>>;

fn put(into: &mut Strings, field: &'static str, text: &str) {
    into.entry(field).or_default().push(text.to_string());
}

fn put_all<'a>(
    into: &mut Strings,
    field: &'static str,
    texts: impl IntoIterator<Item = &'a String>,
) {
    for text in texts {
        put(into, field, text);
    }
}

fn share_strings(into: &mut Strings, fields: (&'static str, &'static str), share: &ShareView) {
    let ShareView { percent, text } = share;
    put_all(into, fields.0, percent);
    put(into, fields.1, text);
}

/// **Every view is destructured whole, with no `..`**, so a field added to one stops this file compiling
/// until someone says whether it is a display string. That is the only thing that makes "every" true.
fn topic_strings(into: &mut Strings, topic: &TopicView) {
    let TopicView {
        name,
        criterion,
        grade,
        unscored,
        shape,
    } = topic;
    put(into, "topic.name", name);
    put(into, "topic.criterion", criterion);
    share_strings(into, ("topic.grade.percent", "topic.grade.text"), grade);
    put_all(into, "topic.unscored", unscored);
    if let Some(ShapeView {
        hit_shape,
        on_line_projected: _,
        landings,
        no_landing,
    }) = shape
    {
        share_strings(into, ("shape.hit.percent", "shape.hit.text"), hit_shape);
        put_all(into, "shape.landings", landings);
        put_all(into, "shape.no_landing", no_landing);
    }
}

fn blend_strings(into: &mut Strings, blend: Option<&BlendView>) {
    if let Some(BlendView {
        mean,
        averaged,
        excluded,
        text,
    }) = blend
    {
        put(into, "blend.mean", mean);
        put_all(into, "blend.averaged", averaged);
        put_all(into, "blend.excluded", excluded);
        put(into, "blend.text", text);
    }
}

fn finding_strings(into: &mut Strings, field: (&'static str, &'static str), list: &[FindingView]) {
    for FindingView { metric, text } in list {
        put(into, field.0, metric);
        put(into, field.1, text);
    }
}

fn strings_of(view: &ProfileView) -> Strings {
    let ProfileView {
        player_id,
        preamble,
        no_clubs,
        clubs,
        player,
    } = view;
    let mut into = Strings::new();
    put(&mut into, "player_id", player_id);
    put(&mut into, "preamble", preamble);
    put_all(&mut into, "no_clubs", no_clubs);
    for ClubView {
        club,
        n_shots: _,
        shots,
        numbers,
        topics,
        no_topics,
        blend,
        strengths,
        weaknesses,
        nothing_listed,
    } in clubs
    {
        put(&mut into, "club", club);
        put(&mut into, "club.shots", shots);
        for NumberRow {
            name,
            n: _,
            // `golf-core club-profile`'s and not this report's, which prints no mean, so it is held to
            // that verb's line by `every_center_is_club_profiles_own_line_for_each_case` instead.
            center: _,
            spread,
            withheld: _,
        } in numbers
        {
            put(&mut into, "number.name", name);
            put(&mut into, "number.spread", spread);
        }
        for topic in topics {
            topic_strings(&mut into, topic);
        }
        put_all(&mut into, "no_topics", no_topics);
        blend_strings(&mut into, blend.as_ref());
        finding_strings(&mut into, ("strength.metric", "strength.text"), strengths);
        finding_strings(&mut into, ("weakness.metric", "weakness.text"), weaknesses);
        put_all(&mut into, "nothing_listed", nothing_listed);
    }
    if let Some(PlayerView {
        basis,
        n_shots: _,
        shots,
        topics,
        no_topics,
        blend,
        strengths,
        weaknesses,
        nothing_listed,
    }) = player
    {
        put(&mut into, "player.basis", basis);
        put(&mut into, "player.shots", shots);
        for topic in topics {
            topic_strings(&mut into, topic);
        }
        put_all(&mut into, "no_topics", no_topics);
        blend_strings(&mut into, blend.as_ref());
        finding_strings(&mut into, ("strength.metric", "strength.text"), strengths);
        finding_strings(&mut into, ("weakness.metric", "weakness.text"), weaknesses);
        put_all(&mut into, "nothing_listed", nothing_listed);
    }
    into
}

/// The report wraps at 94 columns and indents its continuation lines, so whitespace is not its words.
fn flattened(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn held_to_report(view: &ProfileView, text: &str, case: &str) {
    let flat = flattened(text);
    let strings = strings_of(view);
    let missing: Vec<(&str, &String)> = strings
        .iter()
        .flat_map(|(field, strings)| strings.iter().map(move |string| (*field, string)))
        .filter(|(_, string)| !flat.contains(&flattened(string)))
        .collect();
    assert!(
        missing.is_empty(),
        "{case}: the view says what the report does not: {missing:#?}\nThe report:\n{text}"
    );
}

/// **The gate**, as the Done line names it: for each hand case, every display string in the view appears,
/// whitespace flattened, in `reports::profile::profile()`'s text for that case.
#[test]
fn every_string_in_the_view_is_in_the_reports_text_for_each_hand_case() {
    for (case, vector) in cases("hand") {
        let (corpus, bag) = input_of(&vector);
        held_to_report(&view_of(&vector), &report_of(&corpus, bag.as_ref()), &case);
    }
}

/// The same held to the **committed** text of the recorded cases, the real golfer's among them, so what
/// the phone says is held to what the lab's report has already been recorded saying.
#[test]
fn every_string_in_the_view_is_in_the_recorded_report_for_each_corpus_case() {
    for (case, vector) in cases("corpus") {
        let text = vector["expected"]["report"]
            .as_str()
            .unwrap_or_else(|| panic!("{case}: a corpus case records its report"));
        held_to_report(&view_of(&vector), text, &case);
    }
}

/// The two states no committed case reaches, whose sentence is the report's too.
#[test]
fn a_profile_with_no_club_says_why_in_the_reports_words() {
    let nobody = empty_corpus();
    let view = profile_view(&nobody, None).expect("an empty profile");
    held_to_report(&view, &report_of(&nobody, None), "no shots");
    let said = view.no_clubs.expect("nothing on record says so");
    assert!(said.contains("nothing to grade yet"), "{said}");
    assert_eq!(view.player, None);

    let untagged = corpus_naming_no_club();
    let view = profile_view(&untagged, None).expect("a profile of the player alone");
    held_to_report(&view, &report_of(&untagged, None), "no club named");
    let said = view.no_clubs.expect("no club named says so");
    assert!(said.contains("only the player's grades follow"), "{said}");
    assert!(view.clubs.is_empty());
    let player = view.player.expect("the player's grades follow");
    assert!(player.n_shots > 0);
    assert!(!player.topics.is_empty());
}

/// A field nothing ever fills is a field the gate never read. So every one has to hold a string somewhere
/// in what the gate walks, and this is what says a new case or a removed one has not opened a hole.
#[test]
fn the_gate_reads_every_field_at_least_once() {
    let mut seen = Strings::new();
    let views = cases("hand")
        .iter()
        .chain(cases("corpus").iter())
        .map(|(_, vector)| view_of(vector))
        .chain([
            profile_view(&empty_corpus(), None).expect("an empty profile"),
            profile_view(&corpus_naming_no_club(), None).expect("the player alone"),
        ])
        .collect::<Vec<_>>();
    for view in &views {
        for (field, strings) in strings_of(view) {
            seen.entry(field).or_default().extend(strings);
        }
    }
    for field in [
        "player_id",
        "preamble",
        "no_clubs",
        "club",
        "club.shots",
        "number.name",
        "number.spread",
        "topic.name",
        "topic.criterion",
        "topic.grade.percent",
        "topic.grade.text",
        "topic.unscored",
        "shape.hit.percent",
        "shape.hit.text",
        "shape.landings",
        "shape.no_landing",
        "no_topics",
        "blend.mean",
        "blend.averaged",
        "blend.excluded",
        "blend.text",
        "strength.metric",
        "strength.text",
        "weakness.metric",
        "weakness.text",
        "nothing_listed",
        "player.basis",
        "player.shots",
    ] {
        assert!(seen.contains_key(field), "nothing fills {field}");
    }
    assert_eq!(seen.len(), 28, "{:?}", seen.keys());
}

// ------------------------------------------------------------------- the mean, held to club-profile

/// The `center` row's text in each metric's block of `golf-core club-profile`'s text for one club, by
/// metric. A metric's block runs from its `    name  (unit)` head to the next line at that indent,
/// and its `center` row is `      center    ` and then the text, or a withheld refusal under the same
/// label, which no mean can be read as since only the printed one carries its median.
fn centers_printed(text: &str) -> BTreeMap<String, Vec<String>> {
    let mut blocks: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut metric: Option<String> = None;
    for line in text.lines() {
        if let Some(head) = line
            .strip_prefix("    ")
            .filter(|rest| !rest.starts_with(' '))
        {
            metric = head.split_whitespace().next().map(str::to_string);
            blocks
                .entry(metric.clone().expect("a head names something"))
                .or_default();
        } else if let (Some(metric), Some(center)) =
            (&metric, line.strip_prefix("      center    "))
        {
            if center.contains("   (median ") {
                blocks
                    .entry(metric.clone())
                    .or_default()
                    .push(center.to_string());
            }
        }
    }
    blocks
}

/// Every club's means in the view, each against the line `golf-core club-profile` prints for that club
/// and metric: the same text when the view has one, and no mean printed when it has none. Counts what
/// it saw of each, so a caller can say both states were reached.
fn held_to_club_profile(
    view: &ProfileView,
    printed: impl Fn(&str) -> String,
    case: &str,
) -> (usize, usize) {
    let (mut shown, mut withheld) = (0, 0);
    for club in &view.clubs {
        let text = printed(&club.club);
        let centers = centers_printed(&text);
        for row in &club.numbers {
            let printed = centers.get(&row.name).unwrap_or_else(|| {
                panic!(
                    "{case}: club-profile prints no {} for {}:\n{text}",
                    row.name, club.club
                )
            });
            match &row.center {
                Some(center) => {
                    shown += 1;
                    assert_eq!(
                        printed,
                        std::slice::from_ref(center),
                        "{case}: {} {}",
                        club.club,
                        row.name
                    );
                }
                None => {
                    withheld += 1;
                    assert!(
                        printed.is_empty(),
                        "{case}: {} {}: {printed:?}",
                        club.club,
                        row.name
                    );
                }
            }
        }
    }
    (shown, withheld)
}

/// `golf-core club-profile`'s text for one club of a corpus, as the verb prints it after `--club`.
fn club_text(bag_profile: &BagProfile, club: &str) -> String {
    let club = parse_club(club).expect("a view's club is a club");
    club_report(bag_profile, "Golfer", false, Some(club))
}

/// **The mean's gate** (the user's decision of 2026-11-05, on P11's finding 5): `NumberRow.center` is
/// `golf-core club-profile`'s `center` row for the same club and metric, word for word, over every hand
/// and recorded case, and absent exactly where that verb prints none. Both states are reached, so the
/// gate is not passing on one alone.
#[test]
fn every_center_is_club_profiles_own_line_for_each_case() {
    let (mut shown, mut withheld) = (0, 0);
    for (case, vector) in cases("hand").into_iter().chain(cases("corpus")) {
        let (corpus, bag) = input_of(&vector);
        let (_, bag_profile) =
            run_profile_and_bag_profile(&corpus, bag.as_ref()).expect("a profile that validates");
        let seen = held_to_club_profile(
            &view_of(&vector),
            |club| club_text(&bag_profile, club),
            &case,
        );
        shown += seen.0;
        withheld += seen.1;
    }
    assert!(shown > 0, "no case holds a mean");
    assert!(withheld > 0, "no case withholds one");
}

/// The case the decision was for: a club whose spread still waits for its 10 samples shows its mean,
/// which clears at 5 (`contracts::baseline::DEFAULT_MINIMUM_N`), and carry has one though its
/// dispersion's `center` is empty, since carry has no target.
#[test]
fn a_mean_shows_before_its_spread_does() {
    let view = hand_view("player-pools-two-clubs-and-untagged");
    let driver = &view.clubs[0];
    assert_eq!(driver.club, "driver");
    let carry = number(driver, "carry_distance_yds");
    assert!(carry.withheld, "{carry:?}");
    let center = carry
        .center
        .as_deref()
        .expect("a mean at 5 samples or more");
    assert!(center.contains("   95% CI "), "{center}");
    assert!(
        center.ends_with(')') && center.contains("(median "),
        "{center}"
    );
}

/// Below its own floor a mean is absent, never a 0, and `spread` is what says it is withheld.
#[test]
fn a_mean_below_its_floor_is_absent() {
    let mut thin = hand_case("player-pools-two-clubs-and-untagged");
    let swings = thin["input"]["corpus"]["swings"]
        .as_array_mut()
        .expect("the case's swings");
    let mut kept = 0;
    swings.retain(|swing| {
        let driver = swing["club"] == "driver";
        kept += usize::from(driver);
        !driver || kept <= 2
    });
    let view = view_of(&thin);
    let driver = view
        .clubs
        .iter()
        .find(|club| club.club == "driver")
        .expect("the driver keeps two shots");
    for row in &driver.numbers {
        assert!(row.n < 5, "{row:?}");
        assert_eq!(row.center, None, "{row:?}");
        assert!(row.withheld, "{row:?}");
    }
}

// ---------------------------------------------------------------------- the structure, by the case

/// A share's numbers as the hand worked them, in the view's text, and a percent exactly when it is graded.
fn share_matches(view: &ShareView, expected: &Value, case: &str) {
    if let Some(rate) = expected.get("graded") {
        let counts = format!("({} of {})", rate["successes"], rate["n"]);
        assert!(view.text.contains(&counts), "{case}: {}", view.text);
        let percent = view.percent.as_deref().expect("a graded share has one");
        assert!(view.text.starts_with(percent), "{case}: {}", view.text);
    } else {
        let withheld = &expected["withheld"];
        let waits = format!(
            "{} of the {} shots a share needs",
            withheld["have_n"], withheld["need_n"]
        );
        assert!(view.text.starts_with("withheld, "), "{case}: {}", view.text);
        assert!(view.text.ends_with(&waits), "{case}: {}", view.text);
        assert_eq!(view.percent, None, "{case}");
    }
}

fn topics_match(view: &[TopicView], expected: &Value, case: &str) {
    let expected = expected.as_array().cloned().unwrap_or_default();
    assert_eq!(view.len(), expected.len(), "{case}: how many topics");
    for (topic, want) in view.iter().zip(&expected) {
        let name = want["topic"].as_str().expect("a name").replace('_', " ");
        assert_eq!(topic.name, name, "{case}");
        assert!(topic.grade.text.contains(&topic.criterion), "{case}");
        share_matches(&topic.grade, &want["grade"], case);
        let unscored = want["unscored"].as_array().map_or(0, Vec::len);
        assert_eq!(topic.unscored.len(), unscored, "{case}: {name}'s unscored");
        match (&topic.shape, want.get("shape")) {
            (None, None) => {}
            (Some(shape), Some(shares)) => {
                share_matches(&shape.hit_shape, &shares["hit_shape"], case);
                assert_eq!(
                    shape.on_line_projected,
                    shares["projected"].as_i64() > Some(0),
                    "{case}: {name}"
                );
                let landed = shares["printed"].as_i64().unwrap_or(0)
                    + shares["projected"].as_i64().unwrap_or(0);
                assert_eq!(shape.landings.is_some(), landed > 0, "{case}: {name}");
                let no_landing = shares["landing_unscored"].as_array().map_or(0, Vec::len);
                assert_eq!(shape.no_landing.len(), no_landing, "{case}: {name}");
            }
            (shape, shares) => panic!("{case}: {name}: {shape:?} against {shares:?}"),
        }
    }
}

fn blend_matches(view: Option<&BlendView>, expected: Option<&Value>, case: &str) {
    let names = |list: &Value| -> Vec<String> {
        list.as_array()
            .map(|names| {
                names
                    .iter()
                    .map(|name| name.as_str().expect("a name").replace('_', " "))
                    .collect()
            })
            .unwrap_or_default()
    };
    match (view, expected.filter(|blend| !blend.is_null())) {
        (None, None) => {}
        (Some(blend), Some(want)) => {
            assert_eq!(blend.averaged, names(&want["averaged"]), "{case}");
            assert_eq!(blend.excluded, names(&want["excluded"]), "{case}");
            assert!(
                blend.text.starts_with(&blend.mean),
                "{case}: {}",
                blend.text
            );
        }
        (blend, want) => panic!("{case}: {blend:?} against {want:?}"),
    }
}

fn list_matches(strengths: &[FindingView], weaknesses: &[FindingView], want: &Value, case: &str) {
    let want = want.as_array().cloned().unwrap_or_default();
    let side = |weak: bool| -> Vec<&str> {
        want.iter()
            .filter(|finding| (finding["kind"] != json!("strength")) == weak)
            .map(|finding| finding["metric"].as_str().expect("a metric"))
            .collect()
    };
    let names = |list: &[FindingView]| -> Vec<String> {
        list.iter().map(|finding| finding.metric.clone()).collect()
    };
    assert_eq!(names(strengths), side(false), "{case}: the strengths");
    assert_eq!(names(weaknesses), side(true), "{case}: the weaknesses");
    assert!(
        strengths.iter().all(|f| f.text.starts_with("strength")),
        "{case}"
    );
    assert!(
        weaknesses.iter().all(|f| f.text.starts_with("weakness")),
        "{case}"
    );
}

/// What a substring gate cannot see: a true sentence on the wrong club, a share in the other share's
/// place, a strength filed as a weakness. Each is held to the case's hand-worked `expected`.
#[test]
fn every_field_sits_where_the_vector_says() {
    for (case, vector) in cases("hand") {
        let view = view_of(&vector);
        let profile = &vector["expected"]["profile"];
        assert_eq!(
            view.player_id,
            profile["player_id"].as_str().expect("an id"),
            "{case}"
        );
        let clubs = profile["clubs"].as_array().expect("the clubs");
        assert_eq!(view.clubs.len(), clubs.len(), "{case}: how many clubs");
        for (club, want) in view.clubs.iter().zip(clubs) {
            let grades = &want["grades"];
            assert_eq!(
                club.club,
                grades["club"].as_str().expect("a club"),
                "{case}"
            );
            assert_eq!(Some(club.n_shots), grades["n_shots"].as_i64(), "{case}");
            topics_match(&club.topics, &grades["topics"], &case);
            blend_matches(club.blend.as_ref(), grades.get("blend"), &case);
            list_matches(&club.strengths, &club.weaknesses, &want["list"], &case);
            let empty = want["list"].as_array().is_none_or(Vec::is_empty);
            assert_eq!(club.nothing_listed.is_some(), empty, "{case}");

            let strike = &grades["strike"];
            let mut numbers: Vec<String> = strike
                .get("ball_speed")
                .map(|baseline| baseline["name"].as_str().expect("a name").to_string())
                .into_iter()
                .collect();
            numbers.extend(
                strike["dispersion"]
                    .as_object()
                    .into_iter()
                    .flatten()
                    .map(|(k, _)| k.clone()),
            );
            let names: Vec<&String> = club.numbers.iter().map(|row| &row.name).collect();
            assert_eq!(
                names,
                numbers.iter().collect::<Vec<_>>(),
                "{case}: the numbers"
            );
            for row in &club.numbers {
                let held = strike["dispersion"]
                    .get(&row.name)
                    .or_else(|| strike.get("ball_speed"))
                    .expect("the row's own");
                assert_eq!(Some(row.n), held["n"].as_i64(), "{case}: {}", row.name);
                assert_eq!(row.withheld, held["sd"].is_null(), "{case}: {}", row.name);
            }
        }
        let player = &profile["player"];
        let grades = &player["grades"];
        let view_player = view.player.as_ref().expect("every hand case holds shots");
        assert_eq!(
            Some(view_player.n_shots),
            grades["n_shots"].as_i64(),
            "{case}"
        );
        topics_match(&view_player.topics, &grades["topics"], &case);
        blend_matches(view_player.blend.as_ref(), grades.get("blend"), &case);
        list_matches(
            &view_player.strengths,
            &view_player.weaknesses,
            &player["list"],
            &case,
        );
    }
}

// ------------------------------------------------------------------------------ the named states

fn player_of(view: &ProfileView) -> &PlayerView {
    view.player.as_ref().expect("this case holds shots")
}

fn topic<'a>(topics: &'a [TopicView], name: &str) -> &'a TopicView {
    topics
        .iter()
        .find(|topic| topic.name == name)
        .unwrap_or_else(|| panic!("no topic {name} in {topics:#?}"))
}

/// A grade below the floor says what it waits for and shows no number, so a screen has nothing to set
/// large and nothing to read as 0.
#[test]
fn a_grade_below_the_floor_is_withheld_in_rusts_words_with_no_number() {
    let view = hand_view("ungraded-named-impossible-absent");
    let grade = &topic(&view.clubs[0].topics, "consistency").grade;
    assert_eq!(grade.percent, None);
    assert_eq!(
        grade.text,
        format!(
            "withheld, {}: {} of the {RATE_MINIMUM_N} shots a share needs",
            criterion(Topic::Consistency),
            view.clubs[0].n_shots
        )
    );
    assert!(view.clubs[0].n_shots < RATE_MINIMUM_N);
}

/// A blend says which topics it averaged and which it left out, by name, so "100%" cannot read as a grade
/// on everything. Here the draw is withheld and the consistency is not.
#[test]
fn a_blend_names_what_it_averaged_and_what_it_left_out() {
    let view = hand_view("draw-on-line-withheld");
    for blend in [&view.clubs[0].blend, &player_of(&view).blend] {
        let blend = blend.as_ref().expect("consistency is graded");
        assert_eq!(blend.mean, "100%");
        assert_eq!(blend.averaged, ["consistency"]);
        assert_eq!(blend.excluded, ["draw"]);
        assert_eq!(
            blend.text,
            "100%, the equal-weight mean of consistency; left out while withheld: draw"
        );
    }
}

/// An absent topic is in neither of a blend's lists, and nowhere else: it is not a withheld one. The fade
/// exists on no shot of this case.
#[test]
fn an_absent_topic_is_named_nowhere() {
    let view = hand_view("draw-on-line-withheld");
    let every = strings_of(&view);
    for (field, strings) in &every {
        for string in strings {
            assert!(!string.contains("fade"), "{field}: {string}");
        }
    }
}

/// With nothing graded there is no blend, for the club or the player, and nothing to print a 0 from.
#[test]
fn a_blend_is_absent_when_no_topic_is_graded() {
    let view = hand_view("ungraded-named-impossible-absent");
    assert_eq!(view.clubs[0].blend, None);
    assert_eq!(player_of(&view).blend, None);
    let every = strings_of(&view);
    assert!(!every.contains_key("blend.mean"));
    assert!(!every.contains_key("topic.grade.percent"));
}

/// ADR-034 §5.3: a landing the flight model projected is labelled as projected. The screen reads the flag,
/// and the word is what it shows.
#[test]
fn a_projected_landing_says_projected() {
    let view = hand_view("left-handed-fade-and-draw-projected");
    for name in ["fade", "draw"] {
        let shape = topic(&view.clubs[0].topics, name)
            .shape
            .as_ref()
            .expect("a shape topic");
        assert!(shape.on_line_projected, "{name}");
        let landings = shape.landings.as_deref().expect("every landing projected");
        assert!(landings.contains("projected"), "{landings}");
        assert!(landings.contains("not printed"), "{landings}");
    }
}

/// And a printed one does not, since the label is only worth anything if it can be absent.
#[test]
fn a_printed_landing_does_not_say_projected() {
    let view = hand_view("straight-edge-printed-landing");
    let shape = topic(&view.clubs[0].topics, "straight")
        .shape
        .as_ref()
        .expect("a shape topic");
    assert!(!shape.on_line_projected);
    assert_eq!(
        shape.landings.as_deref(),
        Some(format!("landings: {} printed", view.clubs[0].n_shots).as_str())
    );
    let strings = strings_of(&view);
    assert!(
        strings.values().flatten().all(|s| !s.contains("projected")),
        "{strings:#?}"
    );
}

/// Both shares of a shape topic can stand apart: here hitting the draw is graded while the on-line share
/// is withheld, because most shots have no landing, and each has its own number or its own lack of one.
#[test]
fn a_shape_topic_carries_both_shares_each_in_its_own_state() {
    let view = hand_view("draw-on-line-withheld");
    let draw = topic(&view.clubs[0].topics, "draw");
    assert_eq!(draw.grade.percent, None);
    assert!(
        draw.grade.text.starts_with("withheld, "),
        "{}",
        draw.grade.text
    );
    let shape = draw.shape.as_ref().expect("a shape topic");
    assert!(shape.hit_shape.percent.is_some(), "{shape:?}");
    assert!(shape.on_line_projected);
    assert_eq!(shape.no_landing.len(), 3, "{:?}", shape.no_landing);
    assert!(shape
        .no_landing
        .iter()
        .all(|s| s.contains("out of the on-line share")));
    assert_eq!(draw.unscored.len(), 1, "{:?}", draw.unscored);
    assert!(
        draw.unscored[0].contains("not judged"),
        "{}",
        draw.unscored[0]
    );
    assert_eq!(topic(&view.clubs[0].topics, "consistency").shape, None);
}

/// Each side keeps the list's order, largest miss first, and a list of nothing says so rather than offering
/// a strength: "nothing established" is never one (ADR-034 §5.7).
#[test]
fn the_list_is_split_into_weaknesses_and_strengths_and_an_empty_one_says_so() {
    let view = hand_view("the-list-beside-a-drill-and-pose");
    let names = |list: &[FindingView]| -> Vec<String> {
        list.iter().map(|finding| finding.metric.clone()).collect()
    };
    let (driver, seven) = (&view.clubs[0], &view.clubs[1]);
    assert_eq!(seven.club, "7i");
    assert_eq!(
        names(&seven.weaknesses),
        ["carry_distance_yds", "face_to_path_deg"]
    );
    assert_eq!(
        names(&seven.strengths),
        ["start_line_offline_yds", "start_line_deg"]
    );
    assert_eq!(seven.nothing_listed, None);
    assert_eq!(driver.club, "driver");
    assert!(driver.strengths.is_empty() && driver.weaknesses.is_empty());
    assert_eq!(driver.nothing_listed.as_deref(), Some(NOTHING_LISTED));
}

/// A club with shots and no topic says so, in place of a header over a blank. Only the recorded cases
/// reach it: shots that print the inputs of no topic.
#[test]
fn no_topic_is_said_exactly_when_there_are_none() {
    let mut said = 0;
    for (case, vector) in cases("hand").into_iter().chain(cases("corpus")) {
        let view = view_of(&vector);
        for club in &view.clubs {
            assert_eq!(club.no_topics.is_some(), club.topics.is_empty(), "{case}");
            said += usize::from(club.no_topics.is_some());
        }
        if let Some(player) = &view.player {
            assert_eq!(
                player.no_topics.is_some(),
                player.topics.is_empty(),
                "{case}"
            );
        }
    }
    assert!(said > 0, "no committed case holds a club with no topic");
}

/// A sentence naming a verb the phone does not have is advice nobody on a phone can take, which `crate::history`
/// had to rewrite for the corpus's. Here the gate forbids rewriting one, so this is what says none has arrived.
#[test]
fn no_sentence_names_a_verb_the_phone_does_not_have() {
    for (case, vector) in cases("hand").into_iter().chain(cases("corpus")) {
        let strings = strings_of(&view_of(&vector));
        for (field, strings) in &strings {
            for string in strings {
                for verb in ["golf-core", "golf-lab", "python", "scripts/", "--"] {
                    assert!(!string.contains(verb), "{case}: {field}: {string}");
                }
            }
        }
    }
}

/// Carry, ball speed and the rest, with n and spread. A topped shot leaves the carry's `n` short and ball
/// speed's whole, so each row has to state its own: this is the two counters `contracts::club_profile`
/// keeps apart.
#[test]
fn a_clubs_numbers_are_each_with_its_own_n_and_its_spread() {
    let view = hand_view("consistency-mishit-and-outliers");
    let numbers = &view.clubs[0].numbers;
    let row = |name: &str| -> &NumberRow {
        numbers
            .iter()
            .find(|row| row.name == name)
            .unwrap_or_else(|| panic!("no {name} in {numbers:#?}"))
    };
    assert_eq!(numbers[0].name, "ball_speed_mph");
    assert_eq!(row("ball_speed_mph").n, 12);
    assert_eq!(row("carry_distance_yds").n, 10);
    for number in numbers {
        assert_eq!(
            number.withheld,
            number.spread.starts_with("withheld: "),
            "{number:?}"
        );
        if !number.withheld {
            assert!(number.spread.starts_with("sd "), "{number:?}");
            assert!(
                number.spread.ends_with(&format!("n = {}", number.n)),
                "{number:?}"
            );
        }
    }
    assert!(!row("carry_distance_yds").withheld);
}

/// A number under its floor is a refusal, and says what it waits for, in place of a spread that would read
/// as measured.
#[test]
fn a_number_below_its_floor_is_withheld_and_says_why() {
    let view = hand_view("ungraded-named-impossible-absent");
    let carry = view.clubs[0]
        .numbers
        .iter()
        .find(|row| row.name == "carry_distance_yds")
        .expect("carry is held");
    assert!(carry.withheld, "{carry:?}");
    assert!(carry.spread.contains("needs 10 samples"), "{carry:?}");
    assert!(
        carry.spread.contains(&format!("(there are {})", carry.n)),
        "{carry:?}"
    );
}

/// The player pools two clubs and the shots naming none, in bag order for the clubs, and says the untagged
/// ones count toward some topics and not others.
#[test]
fn the_player_counts_the_shots_naming_no_club_and_says_so() {
    let view = hand_view("player-pools-two-clubs-and-untagged");
    let clubs: Vec<&str> = view.clubs.iter().map(|club| club.club.as_str()).collect();
    assert_eq!(clubs, ["driver", "7i"]);
    let player = player_of(&view);
    assert_eq!(player.n_shots, 16);
    assert!(
        player.shots.contains("4 naming no club"),
        "{}",
        player.shots
    );
    assert!(
        player.basis.contains("its own club's criterion"),
        "{}",
        player.basis
    );
}

// ----------------------------------------------------------------------------- through the stores

fn spike() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spikes/vision-ocr")
}

/// A scratch data root with a golfer and a club chosen, as first-run setup leaves it.
struct Phone {
    dir: TempDir,
    root: String,
}

impl Phone {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("a scratch directory");
        let root = dir.path().join("processed").display().to_string();
        init_state(root.clone(), "a1b2".to_string()).expect("an install id");
        create_golfer(
            root.clone(),
            "Aaron".to_string(),
            Handedness::Right,
            NOW.to_string(),
        )
        .expect("a golfer");
        let phone = Phone { dir, root };
        phone.choose(|state| {
            state.player_id = Some("aaron".to_string());
            state.club = Some("7i".to_string());
        });
        phone
    }

    fn choose(&self, change: impl FnOnce(&mut AppState)) {
        let mut state = get_state(self.root.clone());
        change(&mut state);
        set_state(self.root.clone(), state).expect("a state set_state takes");
    }

    fn feed(&self, id: &str) -> ShotCard {
        let tmp = self.dir.path().join("tmp");
        fs::create_dir_all(&tmp).expect("tmp");
        let to = tmp.join(format!("{id}.jpg"));
        fs::copy(spike().join("captures").join(format!("{id}.jpg")), &to).expect("a capture");
        let boxes = spike().join("boxes/C0/raw").join(format!("{id}.json"));
        let outcome = record_photo(PhotoIn {
            root: self.root.clone(),
            path: to.display().to_string(),
            boxes_json: fs::read_to_string(boxes).expect("the capture's boxes"),
            captured_at: TAKEN.to_string(),
            now: NOW.to_string(),
            challenge_call: None,
        })
        .expect("a recorded photo");
        assert_ne!(outcome.status, RecordStatus::Failed, "{id}");
        outcome.card.expect("a card")
    }

    fn feed_all(&self) -> Vec<ShotCard> {
        CAPTURES.iter().map(|id| self.feed(id)).collect()
    }

    fn profile(&self) -> ProfileView {
        graded_profile(self.root.clone(), "aaron".to_string()).expect("a profile")
    }
}

fn number<'a>(club: &'a ClubView, name: &str) -> &'a NumberRow {
    club.numbers
        .iter()
        .find(|row| row.name == name)
        .unwrap_or_else(|| panic!("no {name} in {:#?}", club.numbers))
}

/// "No golfer data means an empty profile, not a refusal." The first screen anyone sees, and no id here can
/// reach outside the data root or be refused for its spelling: the id only filters and names a bag file.
#[test]
fn no_golfer_data_is_an_empty_profile_and_never_a_refusal() {
    let phone = Phone::new();
    let scratch = tempfile::tempdir().expect("a scratch directory");
    let never_set_up = scratch.path().join("processed").display().to_string();
    for (root, player_id) in [
        (phone.root.as_str(), "aaron"),
        (phone.root.as_str(), "nobody"),
        (phone.root.as_str(), ""),
        (phone.root.as_str(), "../../etc"),
        (never_set_up.as_str(), "aaron"),
    ] {
        let view = graded_profile(root.to_string(), player_id.to_string())
            .unwrap_or_else(|refused| panic!("{player_id:?}: {refused}"));
        assert_eq!(view.player_id, player_id);
        assert!(view.clubs.is_empty(), "{view:?}");
        assert_eq!(view.player, None, "{view:?}");
        assert!(view.no_clubs.is_some());
    }
}

/// The six captures all the way: read from their boxes, stored, read back through `read_corpus` as `golf-core
/// profile` reads them, and held to that verb's text. **With six shots of one club every grade is withheld
/// and no blend exists, and that is the right answer**, not a failure to grade. provisional: pasted copies.
#[test]
fn the_six_captures_read_as_golf_core_profile_reads_them() {
    let phone = Phone::new();
    phone.feed_all();
    let view = phone.profile();

    let dirs = Dirs::of(&phone.root);
    let corpus = read_corpus(&dirs.sessions, "aaron", engine_versions());
    let bag = BagStore::new(&dirs.golfers).get("aaron");
    assert_eq!(
        view,
        profile_view(&corpus, bag.as_ref()).expect("the same profile")
    );
    held_to_report(&view, &report_of(&corpus, bag.as_ref()), "the six captures");

    assert_eq!(view.player_id, "aaron");
    assert_eq!(view.clubs.len(), 1);
    let club = &view.clubs[0];
    assert_eq!(club.club, "7i");
    assert_eq!(club.n_shots, 6);
    assert_eq!(club.shots, "6 tracked shot photos");
    assert!(!club.topics.is_empty());
    assert!(
        club.topics
            .iter()
            .all(|topic| topic.grade.percent.is_none()),
        "{club:#?}"
    );
    assert_eq!(club.blend, None);
    assert_eq!(number(club, "carry_distance_yds").n, 6);
    assert_eq!(number(club, "ball_speed_mph").n, 6);
    assert_eq!(player_of(&view).n_shots, 6);

    // Every spread waits for 10 samples, and every mean is through at 6, held to what the verb itself
    // prints over the same stores: the 7 iron's carry is on the tab (P26's decision).
    let request = Request {
        name: None,
        player_id: Some("aaron".to_string()),
        verbose: false,
        club: Some("7i".to_string()),
        json: false,
        dirs: DataDirs {
            sessions: dirs.sessions.clone(),
            golfers: dirs.golfers.clone(),
        },
    };
    let verb = answer(Verb::ClubProfile, &request, engine_versions());
    assert_eq!(verb.code, 0, "{}", verb.stderr);
    let (shown, withheld) =
        held_to_club_profile(&view, |_| verb.stdout.clone(), "the six captures");
    assert_eq!((shown, withheld), (club.numbers.len(), 0), "{club:#?}");
    assert!(club.numbers.iter().all(|row| row.withheld), "{club:#?}");
}

/// Bag order, which the profile's own `Validate` holds: an alphabetised bag reads as a bug. And a club in the
/// bag with nothing hit is no club of this profile, which P26's selector has to say from `bag_slots`.
/// provisional: pasted copies.
#[test]
fn clubs_come_in_bag_order_and_a_club_never_hit_is_absent() {
    let phone = Phone::new();
    set_bag_slot(
        phone.root.clone(),
        "aaron".to_string(),
        "3w".to_string(),
        Some(15.0),
        NOW.to_string(),
    )
    .expect("a bag slot");
    phone.feed("IMG_2889");
    phone.choose(|state| state.club = Some("driver".to_string()));
    phone.feed("IMG_2891");
    let clubs: Vec<String> = phone
        .profile()
        .clubs
        .into_iter()
        .map(|club| club.club)
        .collect();
    assert_eq!(clubs, ["driver", "7i"]);
}

/// A drill counts in no number, for its club or the player, and another golfer's shot is not this one's.
/// provisional: pasted copies.
#[test]
fn a_drill_and_another_golfers_shot_count_for_nothing_here() {
    let phone = Phone::new();
    phone.feed("IMG_2889");
    phone.choose(|state| state.mode = PracticeMode::Drill);
    phone.feed("IMG_2891");
    phone.choose(|state| {
        state.mode = PracticeMode::Fundamentals;
    });
    create_golfer(
        phone.root.clone(),
        "Beth".to_string(),
        Handedness::Right,
        NOW.to_string(),
    )
    .expect("a golfer");
    phone.choose(|state| state.player_id = Some("beth".to_string()));
    phone.feed("IMG_2892");

    let view = phone.profile();
    assert_eq!(view.clubs.len(), 1);
    assert_eq!(view.clubs[0].n_shots, 1);
    assert_eq!(player_of(&view).n_shots, 1);
    let beth = graded_profile(phone.root.clone(), "beth".to_string()).expect("a profile");
    assert_eq!(player_of(&beth).n_shots, 1);
}

/// What a person does on a shot's detail screen reaches the profile through the stores, each by the rule it
/// is meant to: a verdict takes a carry out and leaves the shot counted, a retag moves it, a cleared value
/// shortens that number alone, and a delete takes the shot away. provisional: pasted copies.
#[test]
fn a_persons_changes_to_a_shot_reach_the_numbers() {
    let phone = Phone::new();
    let cards = phone.feed_all();
    let root = || phone.root.clone();

    set_shot_mishit(
        root(),
        cards[0].session_id.clone(),
        cards[0].swing_id.clone(),
        Some(MishitVerdict::Confirmed),
        LATER.to_string(),
    )
    .expect("a verdict");
    let club = phone.profile().clubs.remove(0);
    assert_eq!(club.n_shots, 6);
    assert_eq!(number(&club, "carry_distance_yds").n, 5);
    assert_eq!(number(&club, "ball_speed_mph").n, 6);

    correct_shot(
        root(),
        cards[1].session_id.clone(),
        cards[1].swing_id.clone(),
        vec![FieldEdit {
            field: "ball_speed".to_string(),
            printed: None,
            letter: None,
        }],
        LATER.to_string(),
    )
    .expect("a cleared value");
    let club = phone.profile().clubs.remove(0);
    assert_eq!(number(&club, "ball_speed_mph").n, 5);
    assert_eq!(club.n_shots, 6);

    set_shot_club(
        root(),
        cards[2].session_id.clone(),
        cards[2].swing_id.clone(),
        "6i".to_string(),
        LATER.to_string(),
    )
    .expect("a retag");
    let view = phone.profile();
    let shots: Vec<(&str, i64)> = view
        .clubs
        .iter()
        .map(|c| (c.club.as_str(), c.n_shots))
        .collect();
    assert_eq!(shots, [("6i", 1), ("7i", 5)]);
    assert_eq!(player_of(&view).n_shots, 6);

    delete_shot(
        root(),
        cards[3].session_id.clone(),
        cards[3].swing_id.clone(),
    )
    .expect("a delete");
    let view = phone.profile();
    assert_eq!(view.clubs[1].n_shots, 4);
    assert_eq!(player_of(&view).n_shots, 5);
}

/// A profile taken while a save is landing counts every shot. **This does not show the lock matters**: with
/// it out of `crate::profile::graded` this passed 6 runs of 6, because the corpus reads no file a save
/// tears (that module's doc). What it pins is the outcome, whichever way the lock goes. provisional: pasted
/// copies.
#[test]
fn a_profile_taken_during_a_save_still_counts_every_shot() {
    let phone = Phone::new();
    let cards = phone.feed_all();
    let root = phone.root.clone();
    let (session_id, swing_id) = (cards[0].session_id.clone(), cards[0].swing_id.clone());
    let saver = thread::spawn(move || {
        for n in 0..SAVES {
            correct_shot(
                root.clone(),
                session_id.clone(),
                swing_id.clone(),
                vec![FieldEdit {
                    field: "carry_distance".to_string(),
                    printed: Some(format!("{}", 130 + n % 2)),
                    letter: None,
                }],
                LATER.to_string(),
            )
            .expect("a saved review");
        }
    });
    while !saver.is_finished() {
        let view = phone.profile();
        assert_eq!(player_of(&view).n_shots, 6, "{view:#?}");
        assert_eq!(
            number(&view.clubs[0], "carry_distance_yds").n,
            6,
            "{view:#?}"
        );
    }
    saver.join().expect("the saver");
}
