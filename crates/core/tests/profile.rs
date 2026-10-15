//! The profile family (`spec/vectors/profile/`) against `golf_core::profile::run_profile`: every
//! case read, its counts held to its own swings, and its answer compared. [M37 P5; the comparison
//! P9's; the corpus cases P11's]
//!
//! **No Python twin, and no Python oracle.** M37 is the first analysis written Rust first with
//! nothing behind it (`contracts::strike`'s module doc), and the family has two sub-families:
//!
//! - **`hand/`** (P5): every case worked by hand (`provenance.oracle: "hand"`) from the M37 plan's
//!   decisions and calls, before the code that must meet it existed. Each `note` shows its working,
//!   and the shared `source` names the three kinds of number a case carries rather than works: the
//!   club's ball-speed baseline and dispersions (`analysis::club_profile::build_bag_profile`'s), the
//!   player's (the whole corpus' `analysis::dispersion::build_dispersion`), and heel/toe's mean
//!   offset and its interval (`analysis::stats`). Those are gated by the career family already; the
//!   grades are what the hand decides.
//! - **`corpus/`** (P11): one case per career case whose corpus carries a shot, the real golfer's
//!   among them, its `bag`, `corpus` and `display_name` adopted from that career case and its
//!   `expected` written once by `run_profile_case` (`provenance.oracle: "rust"`, the plan's call
//!   18). Nothing independent stands behind one, so they pin stability, and the `profile` report's
//!   text, which no hand case carries.
//!
//! A hand case is `{career_version, id, input: {corpus, bag}, expected: {profile}, provenance}`: the
//! corpus `crates/storage::read_corpus` would hand `golf-core profile`, and the `GradedProfile`
//! `golf_core::profile::run_profile` must answer from it. A corpus case's `input` adds
//! `display_name` and its `expected` adds `report`. The family ages on `CAREER_VERSION` (the plan's
//! call 1), as the storage and career families do.
//!
//! # What is checked, and in what order
//!
//! Before any answer is asked for, every case reads into its types through their `Validate`
//! ([`every_profile_case_reads_into_its_types`]), which holds a profile's parts to each other (a
//! blend to its graded topics, heel/toe to the grid, a shape's shares to its judged count), and
//! every count a case asserts is held to the swings in its own corpus
//! ([`every_case_counts_its_own_swings`]), so a hand-worked case cannot claim a shot its corpus does
//! not hold. Those two are what checked the cases from P5 until `run_profile` existed, and they stay
//! because they tell a slip in the working from a builder that disagrees.
//!
//! **The gate** is [`every_profile_case_is_run_profiles_answer`]: every case in the family, run
//! through `golf_core::profile_family::run_profile_case` (the runner `golf-core rerecord` shares
//! from P11) and compared under `docs/CONFORMANCE.md` §3's rules, the report's text included. It
//! walks the directory, so a new case is compared the moment it is committed. §3's `RTOL` cannot see
//! a last bit, and the plan's P5 finding 8 asks for the comparison **by value**, so
//! [`every_profile_is_run_profiles_by_value`] holds the typed profiles equal as well, on the target
//! the cases were worked on (its doc says why only there).
//!
//! **The corpus cases are another family's inputs**, so two more pins hold them to it:
//! [`every_adopted_input_is_its_career_cases`], the career gate's adoption pin one link further, and
//! [`every_career_case_carrying_a_shot_has_its_corpus_case`], the census both ways.
//!
//! # What the cases cover, as a checked fact
//!
//! [`the_hand_cases_reach_every_rule_the_plan_names`] counts what the family reaches (each topic
//! graded, a withheld share, an absent blend, a printed, a projected and a missing landing, each
//! kind of list entry and an empty list, an untagged shot), so a case edited away from its purpose
//! fails by name rather than leaving a hole.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Read as _;
use std::path::{Path, PathBuf};

use contracts::bag::Bag;
use contracts::career::{CareerCorpus, CorpusSwing, CAREER_VERSION};
use contracts::club::ClubId;
use contracts::strike::{
    FindingKind, GradedProfile, ProfileFinding, Share, Topic, TopicGrade, RATE_MINIMUM_N,
};
use contracts::unscored::UnscoredReason;
use flate2::read::GzDecoder;
use golf_core::profile::run_profile;
use golf_core::profile_family::{adopted_from, differences, run_profile_case, ADOPTED, KIND};
use serde_json::Value;

/// The sub-family worked by hand (P5), whose oracle is `"hand"`.
const HAND: &str = "profile/hand/";
/// The sub-family Rust recorded (P11), whose oracle is `"rust"`, each case adopting a career case's
/// input.
const CORPUS: &str = "profile/corpus/";

fn vectors_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../spec/vectors")
        .canonicalize()
        .expect("spec/vectors/ is committed")
}

/// Every case in the family, sorted by id, each with the path it was read from. Duplicated from
/// `tests/career.rs` for `engine.rs`'s reason: integration tests are separate binaries.
fn profile_cases() -> Vec<(String, PathBuf, Value)> {
    fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
        for entry in fs::read_dir(dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())) {
            let path = entry.expect("a directory entry").path();
            if path.is_dir() {
                collect(&path, out);
            } else if path.extension().is_some_and(|ext| ext == "json") {
                out.push(path);
            }
        }
    }
    let root = vectors_dir().join(KIND);
    let mut paths = Vec::new();
    collect(&root, &mut paths);
    let mut cases: Vec<(String, PathBuf, Value)> = paths
        .into_iter()
        .map(|path| {
            let text =
                fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            let vector: Value =
                serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            let id = vector["id"]
                .as_str()
                .expect("a case carries its id")
                .to_string();
            (id, path, vector)
        })
        .collect();
    cases.sort_by(|a, b| a.0.cmp(&b.0));
    assert!(!cases.is_empty(), "no case under spec/vectors/{KIND}");
    cases
}

/// One case, read into its types: the corpus, the bag, and the expected profile, each through its
/// `Validate`.
struct Case {
    id: String,
    corpus: CareerCorpus,
    bag: Option<Bag>,
    profile: GradedProfile,
}

fn read_case(id: &str, path: &Path, vector: &Value) -> Case {
    let relative = path
        .strip_prefix(vectors_dir())
        .expect("under spec/vectors")
        .with_extension("")
        .to_string_lossy()
        .replace('\\', "/");
    assert_eq!(id, relative, "{}: the id is not its path", path.display());
    assert_eq!(
        vector["career_version"].as_i64(),
        Some(CAREER_VERSION),
        "{id}: born at career_version {} against a port at {CAREER_VERSION}",
        vector["career_version"]
    );
    let provenance = &vector["provenance"];
    assert_eq!(provenance["kind"].as_str(), Some(KIND), "{id}: kind");
    for key in ["note", "source"] {
        let text = provenance[key].as_str().unwrap_or("");
        assert!(!text.is_empty(), "{id}: provenance.{key} is empty");
        // `conformance.py list` prints every note to a console that may encode as cp1252.
        assert!(text.is_ascii(), "{id}: provenance.{key} is not ASCII");
    }
    let keys = |value: &Value| -> BTreeSet<String> {
        value
            .as_object()
            .map(|object| object.keys().cloned().collect())
            .unwrap_or_default()
    };
    let set =
        |names: &[&str]| -> BTreeSet<String> { names.iter().map(|n| n.to_string()).collect() };
    // Each sub-family's shape and oracle, and a sub-family nobody placed is refused rather than read
    // as either: `golf-core rerecord` walks exactly these two (`PROFILE_DOCUMENTS`).
    let adopted = adopted_from(vector).unwrap_or_else(|why| panic!("{why}"));
    let (oracle, input, expected) = if id.starts_with(HAND) {
        assert_eq!(adopted, None, "{id}: a hand case adopts nothing");
        ("hand", set(&["bag", "corpus"]), set(&["profile"]))
    } else if id.starts_with(CORPUS) {
        assert!(
            adopted.is_some(),
            "{id}: a corpus case adopts a career case's input, named first in provenance.source"
        );
        ("rust", set(&ADOPTED), set(&["profile", "report"]))
    } else {
        panic!("{id}: not under {HAND} or {CORPUS}, the family's two sub-families");
    };
    assert_eq!(provenance["oracle"].as_str(), Some(oracle), "{id}: oracle");
    assert_eq!(keys(&vector["input"]), input, "{id}: input");
    assert_eq!(keys(&vector["expected"]), expected, "{id}: expected");
    let corpus: CareerCorpus = serde_json::from_value(vector["input"]["corpus"].clone())
        .unwrap_or_else(|e| panic!("{id}: input.corpus: {e}"));
    let bag: Option<Bag> = serde_json::from_value(vector["input"]["bag"].clone())
        .unwrap_or_else(|e| panic!("{id}: input.bag: {e}"));
    let profile: GradedProfile = serde_json::from_value(vector["expected"]["profile"].clone())
        .unwrap_or_else(|e| panic!("{id}: expected.profile: {e}"));
    // Written as `GradedProfile` writes itself, so the gate's comparison of two documents is not a
    // comparison of two spellings: no `null` where the struct omits a key, no key it does not
    // write.
    let rewritten = serde_json::to_value(&profile).expect("a GradedProfile serializes");
    assert_eq!(
        rewritten, vector["expected"]["profile"],
        "{id}: expected.profile is not spelled as GradedProfile writes it"
    );
    Case {
        id: id.to_string(),
        corpus,
        bag,
        profile,
    }
}

fn cases() -> Vec<Case> {
    profile_cases()
        .iter()
        .map(|(id, path, vector)| read_case(id, path, vector))
        .collect()
}

/// A swing whose shot a topic may judge: counting toward metrics, not flagged, holding a shot
/// (the M37 plan's call 8).
fn judgeable(swing: &CorpusSwing) -> bool {
    swing.counts_toward_metrics() && !swing.shot_needs_review && swing.shot.is_some()
}

fn declared(swing: &CorpusSwing, topic: Topic) -> bool {
    topic.shape().is_some_and(|shape| {
        swing
            .intent
            .as_ref()
            .is_some_and(|goal| goal.target_shape == Some(shape))
    })
}

/// The shots a topic could have judged or left unscored among these swings: every judgeable one,
/// and for a shape only those declared it.
fn could_judge(swings: &[&CorpusSwing], topic: Topic) -> i64 {
    swings
        .iter()
        .filter(|swing| judgeable(swing) && (topic.shape().is_none() || declared(swing, topic)))
        .count() as i64
}

fn photos(swings: &[&CorpusSwing]) -> i64 {
    swings
        .iter()
        .filter_map(|swing| swing.shot_sha256.as_deref())
        .collect::<BTreeSet<_>>()
        .len() as i64
}

fn unscored_total(grade: &TopicGrade) -> i64 {
    grade.unscored.iter().map(|count| count.n).sum()
}

// ------------------------------------------------------------------------------------------ gates

/// Every case reads into its types through their `Validate`, and is spelled as those types write
/// themselves.
#[test]
fn every_profile_case_reads_into_its_types() {
    let read = cases();
    println!(
        "{} profile cases read: {}",
        read.len(),
        read.iter()
            .map(|case| case.id.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );
}

/// Every count a case asserts, held to the swings in its own corpus: which clubs are listed, each
/// owner's shots, the untagged shots, and no topic judging (or leaving unscored) more shots than
/// the corpus gives it. The player's club-bound topics are the clubs' sums exactly (call 9), and its
/// club-free ones exceed the clubs' only by untagged shots.
#[test]
fn every_case_counts_its_own_swings() {
    for case in cases() {
        let id = &case.id;
        let swings: Vec<&CorpusSwing> = case.corpus.swings.iter().collect();
        assert_eq!(
            case.profile.player_id, case.corpus.player_id,
            "{id}: player"
        );

        let mut by_club: BTreeMap<ClubId, Vec<&CorpusSwing>> = BTreeMap::new();
        for swing in &swings {
            if let (Some(club), Some(_)) = (swing.club, swing.shot_sha256.as_ref()) {
                by_club.entry(club).or_default().push(swing);
            }
        }
        let listed: Vec<ClubId> = case
            .profile
            .clubs
            .iter()
            .map(|club| club.grades.club)
            .collect();
        assert_eq!(
            listed,
            by_club.keys().copied().collect::<Vec<_>>(),
            "{id}: the clubs listed are not the clubs with a tracked shot"
        );

        let mut clubs_judged: BTreeMap<Topic, i64> = BTreeMap::new();
        let mut clubs_unscored: BTreeMap<Topic, i64> = BTreeMap::new();
        for graded in &case.profile.clubs {
            let grades = &graded.grades;
            let own = &by_club[&grades.club];
            assert_eq!(grades.n_shots, photos(own), "{id}: {} n_shots", grades.club);
            for topic in &grades.topics {
                let room = could_judge(own, topic.topic);
                assert!(
                    topic.judged + unscored_total(topic) <= room,
                    "{id}: {} {} judged {} and left {} unscored of {room} shots it could judge",
                    grades.club,
                    topic.topic.as_str(),
                    topic.judged,
                    unscored_total(topic)
                );
                *clubs_judged.entry(topic.topic).or_default() += topic.judged;
                *clubs_unscored.entry(topic.topic).or_default() += unscored_total(topic);
            }
            if let (Some(impacts), Some(strike)) = (
                &grades.strike.impacts,
                grades
                    .topics
                    .iter()
                    .find(|t| t.topic == Topic::StrikeLocation),
            ) {
                assert_eq!(
                    impacts.heel_toe.n, strike.judged,
                    "{id}: {} heel/toe counts other shots than strike location judged",
                    grades.club
                );
            }
        }

        let untagged: Vec<&CorpusSwing> = swings
            .iter()
            .copied()
            .filter(|swing| swing.club.is_none() && swing.shot_sha256.is_some())
            .collect();
        let player = &case.profile.player.grades;
        assert_eq!(player.n_shots, photos(&swings), "{id}: player n_shots");
        assert_eq!(player.untagged_shots, photos(&untagged), "{id}: untagged");
        for topic in &player.topics {
            let from_clubs = clubs_judged.get(&topic.topic).copied().unwrap_or(0);
            let unscored_clubs = clubs_unscored.get(&topic.topic).copied().unwrap_or(0);
            if topic.topic.needs_a_club() {
                assert_eq!(
                    (topic.judged, unscored_total(topic)),
                    (from_clubs, unscored_clubs),
                    "{id}: the player's {} is not its clubs' pooled",
                    topic.topic.as_str()
                );
            } else {
                let extra = topic.judged + unscored_total(topic) - from_clubs - unscored_clubs;
                assert!(
                    (0..=could_judge(&untagged, topic.topic)).contains(&extra),
                    "{id}: the player's {} holds {extra} shots beyond its clubs', and the \
                     untagged shots cannot give it them",
                    topic.topic.as_str()
                );
            }
        }
        if let (Some(impacts), Some(strike)) = (
            &player.strike.impacts,
            player
                .topics
                .iter()
                .find(|t| t.topic == Topic::StrikeLocation),
        ) {
            assert_eq!(impacts.heel_toe.n, strike.judged, "{id}: player heel/toe");
        }
    }
}

/// **The gate**: every case's input through `run_profile_case`, the runner the re-record shares,
/// compared with the case's `expected` under §3's rules, every difference in one report. The
/// version and the spelling are [`read_case`]'s, checked first.
#[test]
fn every_profile_case_is_run_profiles_answer() {
    let mut found = Vec::new();
    let mut ids = Vec::new();
    for (id, path, vector) in profile_cases() {
        read_case(&id, &path, &vector);
        let built = run_profile_case(&vector).unwrap_or_else(|why| panic!("{why}"));
        found.extend(
            differences(&vector["expected"], &built)
                .into_iter()
                .map(|d| format!("{id}: {d}")),
        );
        ids.push(id);
    }
    assert!(
        found.is_empty(),
        "{} differences:\n{}",
        found.len(),
        found.join("\n")
    );
    println!("{} profile cases answered: {}", ids.len(), ids.join(", "));
}

/// The career family's vectors, by id, read as `tests/career.rs` reads them (plain or gzipped), for
/// the corpus cases that adopt one. Duplicated for `engine.rs`'s reason: integration tests are
/// separate binaries.
fn career_cases() -> BTreeMap<String, Value> {
    fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
        for entry in fs::read_dir(dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())) {
            let path = entry.expect("a directory entry").path();
            let name = path.to_string_lossy();
            if path.is_dir() {
                collect(&path, out);
            } else if name.ends_with(".json") || name.ends_with(".json.gz") {
                out.push(path);
            }
        }
    }
    let mut paths = Vec::new();
    collect(&vectors_dir().join("career"), &mut paths);
    let cases: BTreeMap<String, Value> = paths
        .iter()
        .map(|path| {
            let bytes = fs::read(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            let text = if path.extension().is_some_and(|ext| ext == "gz") {
                let mut text = String::new();
                GzDecoder::new(&bytes[..])
                    .read_to_string(&mut text)
                    .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
                text
            } else {
                String::from_utf8(bytes).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
            };
            let vector: Value =
                serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            let id = vector["id"].as_str().expect("an id").to_string();
            (id, vector)
        })
        .collect();
    assert!(!cases.is_empty(), "no career vector");
    cases
}

/// **Every adopted input is its career case's, exactly** (the M37 plan's P11, the career gate's
/// `every_adopted_corpus_is_its_storage_vectors_answer` one link further): each corpus case carries
/// the career case it names as its `bag`, `corpus` and `display_name`, value for value, so the two
/// families agree before any re-record derives one from the other. Since a career twin's corpus is
/// its storage case's answer, the chain storage → career → profile is held link by link.
#[test]
fn every_adopted_input_is_its_career_cases() {
    let career = career_cases();
    let mut adopted = Vec::new();
    for (id, _, vector) in profile_cases() {
        let Some(source) = adopted_from(&vector).unwrap_or_else(|why| panic!("{why}")) else {
            continue;
        };
        let twin = career
            .get(&source)
            .unwrap_or_else(|| panic!("{id} adopts {source}, which is not a career vector"));
        for key in ADOPTED {
            assert!(
                vector["input"][key] == twin["input"][key],
                "{id}: input.{key} is not {source}'s"
            );
        }
        adopted.push(id);
    }
    assert!(
        adopted.iter().any(|id| id.starts_with(CORPUS)),
        "no profile case adopts a career case"
    );
    println!("{} profile cases adopt a career case", adopted.len());
}

/// **The census, both ways**: a career case gets a profile corpus case exactly when its corpus
/// carries a shot (the M37 plan's P11: "one per career case with a corpus worth grading"), so a
/// career case that gains one is graded the day it is committed, and no profile case outlives the
/// career case it copies. A corpus with no shot answers an empty profile, which the hand cases and
/// `golf_core::profile`'s own test already reach. Each career case is adopted once at most.
#[test]
fn every_career_case_carrying_a_shot_has_its_corpus_case() {
    let carrying: BTreeSet<String> = career_cases()
        .into_iter()
        .filter(|(_, vector)| {
            vector["input"]["corpus"]["swings"]
                .as_array()
                .is_some_and(|swings| swings.iter().any(|swing| swing["shot"].is_object()))
        })
        .map(|(id, _)| id)
        .collect();
    assert!(
        carrying.iter().any(|id| id.starts_with("career/real/")),
        "the real golfer's corpus carries no shot: {carrying:?}"
    );
    let mut adopted = BTreeSet::new();
    for (id, _, vector) in profile_cases() {
        if let Some(source) = adopted_from(&vector).unwrap_or_else(|why| panic!("{why}")) {
            assert!(
                adopted.insert(source.clone()),
                "{id}: {source} is adopted twice"
            );
        }
    }
    assert_eq!(
        adopted, carrying,
        "the career cases the profile corpus cases adopt are not those carrying a shot"
    );
}

/// **And by value** (the M37 plan's P5 finding 8): the `GradedProfile` [`run_profile`] builds from
/// each case equals the case's own, typed, every float to the bit. The gate above compares within
/// §3's `RTOL`, which is sized for another summation order and would pass a Wilson bound or a blend
/// summed in another order; the hand cases were worked op for op so that it cannot be (P5 finding
/// 4), and only an exact comparison holds them to that.
///
/// Held to the target the cases were worked on (x86_64 Windows, MSVC), as the career family's
/// `every_aggregate_float_is_frozen_pythons_to_the_bit` is: the numbers a case carries rather than
/// works (the dispersions, the ball-speed baseline, heel/toe's interval) were computed by running
/// Rust once, through `analysis::stats`' `pow`, which is the C library's and may answer a last bit
/// otherwise on another libm.
#[cfg(all(target_os = "windows", target_env = "msvc"))]
#[test]
fn every_profile_is_run_profiles_by_value() {
    let mut out = Vec::new();
    let mut floats = 0;
    for case in cases() {
        let built = run_profile(&case.corpus, case.bag.as_ref())
            .unwrap_or_else(|e| panic!("{}: {e}", case.id));
        let (expected, actual) = (
            serde_json::to_value(&case.profile).expect("a GradedProfile serializes"),
            serde_json::to_value(&built).expect("a GradedProfile serializes"),
        );
        floats += count_floats(&expected);
        if built != case.profile {
            let mut here = Vec::new();
            bit_differences(&expected, &actual, "profile", &mut here);
            if here.is_empty() {
                here.push("differs, though no float does: see the gate's report".to_string());
            }
            out.extend(here.into_iter().map(|d| format!("{}: {d}", case.id)));
        }
    }
    assert!(floats > 0, "the family holds no float to compare");
    assert!(
        out.is_empty(),
        "{} differences by value:\n{}",
        out.len(),
        out.join("\n")
    );
    println!("{floats} floats bit-identical");
}

/// Every float in two documents that is not the same f64, by path: the bits, not `RTOL`.
/// Duplicated from `tests/career.rs`, for `engine.rs`'s reason: integration tests are separate
/// binaries.
#[cfg(all(target_os = "windows", target_env = "msvc"))]
fn bit_differences(expected: &Value, actual: &Value, path: &str, out: &mut Vec<String>) {
    match (expected, actual) {
        (Value::Object(e), Value::Object(a)) => {
            for (key, value) in e {
                if let Some(other) = a.get(key) {
                    bit_differences(value, other, &format!("{path}.{key}"), out);
                }
            }
        }
        (Value::Array(e), Value::Array(a)) => {
            for (i, (value, other)) in e.iter().zip(a).enumerate() {
                bit_differences(value, other, &format!("{path}[{i}]"), out);
            }
        }
        (Value::Number(e), Value::Number(a)) if e.is_f64() || a.is_f64() => {
            let (e, a) = (e.as_f64(), a.as_f64());
            if e.map(f64::to_bits) != a.map(f64::to_bits) {
                out.push(format!("{path}: {e:?} worked, {a:?} built"));
            }
        }
        _ => {}
    }
}

#[cfg(all(target_os = "windows", target_env = "msvc"))]
fn count_floats(value: &Value) -> usize {
    match value {
        Value::Object(map) => map.values().map(count_floats).sum(),
        Value::Array(items) => items.iter().map(count_floats).sum(),
        Value::Number(n) => usize::from(n.is_f64()),
        _ => 0,
    }
}

/// What the hand cases reach, across the family, so that a case edited away from its purpose fails
/// by name (the M37 plan's P5 list).
#[test]
fn the_hand_cases_reach_every_rule_the_plan_names() {
    let read = cases();
    let hand: Vec<&Case> = read
        .iter()
        .filter(|case| case.id.starts_with("profile/hand/"))
        .collect();
    let owners = || {
        hand.iter().flat_map(|case| {
            case.profile
                .clubs
                .iter()
                .map(|club| (&club.grades.topics, club.grades.blend.is_some(), &club.list))
                .chain(std::iter::once((
                    &case.profile.player.grades.topics,
                    case.profile.player.grades.blend.is_some(),
                    &case.profile.player.list,
                )))
        })
    };
    let topics = || owners().flat_map(|(topics, _, _)| topics.iter());
    let lists = || owners().map(|(_, _, list)| list);

    let mut missing = Vec::new();
    let mut need = |what: &str, held: bool| {
        if !held {
            missing.push(what.to_string());
        }
    };
    for topic in Topic::ALL {
        need(
            &format!("{} graded", topic.as_str()),
            topics().any(|grade| grade.topic == topic && grade.grade.graded().is_some()),
        );
    }
    need(
        "a share withheld below the floor",
        topics().any(|grade| matches!(grade.grade, Share::Withheld(_))),
    );
    need(
        "a share withheld one shot short of the floor",
        topics()
            .any(|grade| grade.grade.n() == RATE_MINIMUM_N - 1 && grade.grade.graded().is_none()),
    );
    need(
        "an owner with topics and no blend",
        owners().any(|(topics, blend, _)| !topics.is_empty() && !blend),
    );
    need(
        "a blend excluding a withheld topic",
        hand.iter().any(|case| {
            case.profile
                .clubs
                .iter()
                .filter_map(|club| club.grades.blend.as_ref())
                .chain(case.profile.player.grades.blend.as_ref())
                .any(|blend| !blend.excluded.is_empty())
        }),
    );
    let shapes = || topics().filter_map(|grade| grade.shape.as_ref());
    need(
        "a printed landing",
        shapes().any(|shares| shares.printed > 0),
    );
    need(
        "a projected landing",
        shapes().any(|shares| shares.projected > 0),
    );
    need(
        "a shape whose on-line grade is withheld while hit-the-shape grades",
        topics().any(|grade| {
            grade.grade.graded().is_none()
                && grade
                    .shape
                    .as_ref()
                    .is_some_and(|shares| shares.hit_shape.graded().is_some())
        }),
    );
    for reason in [
        UnscoredReason::SpinAxisUnresolved,
        UnscoredReason::CarryUnreachable,
        UnscoredReason::Unrecorded,
    ] {
        need(
            &format!("a landing missing for {reason:?}"),
            shapes().any(|shares| shares.landing_unscored.iter().any(|c| c.reason == reason)),
        );
    }
    for reason in [UnscoredReason::PrintedBlank, UnscoredReason::Misread] {
        need(
            &format!("an input printed blank and unscored for {reason:?}"),
            topics().any(|grade| grade.unscored.iter().any(|c| c.reason == reason)),
        );
    }
    let kinds = |kind: FindingKind| lists().any(|list| list.iter().any(|f| f.kind == kind));
    need("a bias weakness", kinds(FindingKind::Bias));
    need("a scatter weakness", kinds(FindingKind::Scatter));
    need("a strength", kinds(FindingKind::Strength));
    need(
        "an empty list",
        lists().any(|list: &Vec<ProfileFinding>| list.is_empty()),
    );
    need(
        "an untagged shot",
        hand.iter()
            .any(|case| case.profile.player.grades.untagged_shots > 0),
    );
    need(
        "a drill the corpus left out",
        hand.iter().any(|case| {
            case.corpus
                .excluded
                .iter()
                .any(|excluded| excluded.reason == contracts::career::ExclusionReason::Untracked)
        }),
    );
    need(
        "a swing carrying pose rows",
        hand.iter().any(|case| {
            case.corpus.swings.iter().any(|swing| {
                swing
                    .measurements
                    .iter()
                    .any(|row| row.source.starts_with("pose:"))
            })
        }),
    );
    assert!(
        missing.is_empty(),
        "the hand cases no longer reach: {}",
        missing.join("; ")
    );
}
