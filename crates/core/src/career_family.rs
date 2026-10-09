//! The career family's runner: one vector under `spec/vectors/career/` in, every aggregate Rust
//! builds from its input out, in `expected`'s shape. [M36 P11–P12, lifted into the library at P13]
//!
//! `spec/vectors/career/` was recorded once from frozen Python (M36 P3) by
//! `conformance_vectors.py::_run_career`: a corpus and a bag in, and every aggregate the four career
//! scripts print out (`expected.{baseline, dispersion, standing, bag_profile}`), beside the derived
//! counts `model_dump` leaves out (`expected.properties`) and the scripts' report text
//! (`expected.reports`). [`run_career`] is the same input through `crates/analysis`'s builders.
//!
//! # One definition, two callers
//!
//! `tests/career.rs` is the family's gate and `golf-core rerecord` is its recorder (the M36 plan's
//! P13), and both call [`run_career`] and judge its answer with [`crate::compare`]. Until P13 it lived
//! in the gate.
//!
//! # The reports: all five rendered, none carried
//!
//! `expected.reports` is the five scripts' stdout, keyed by script (`career_corpus`,
//! `career_corpus_verbose`, `club_profile_7i`, `flag_mishit_list` …). [`run_career`] renders the
//! reports of every script in [`RENDERED`], through `crate::reports` (M36 P15 the first three, P16
//! the last two), and a re-record copies the reports of every script in [`CARRIED`] from the
//! committed vector unchanged ([`carry`]). Carried rather than left out of the comparison: the
//! re-record starts from [`run_career`]'s document and adds the carried reports back, so a report the
//! recorder grows that is neither rendered nor carried is a removed key to the comparator, which no
//! career declaration can pass. Since P16 [`CARRIED`] is empty and the carry moves nothing; it stays
//! as the place a sixth script's report would wait, named, until a renderer runs it.
//!
//! # An adopted corpus is another vector's answer
//!
//! Most career cases build their corpus by hand, but the storage family's corpus cases each have a
//! career twin whose `input.corpus` is that case's `expected.corpus`, copied whole (P3's
//! `storage-<case>`, and the real golfer), and P3's recorder asserted the two equal. So the career
//! family's input moves when the storage family's answer does. [`adopted_from`] reads which storage
//! vector a case adopted, and `golf-core rerecord` derives the copy from that vector *as the run will
//! write it*: a copy that drifted from its source, or a source whose answer moved, is a difference at
//! `input.corpus…` that the declaration has to name, in the same run that moves the source. That is
//! what keeps the two families from drifting apart silently once Rust re-records them (the M36 plan's
//! call 2), and `tests/career.rs` holds the committed families to it.

use serde_json::{json, Map, Value};

use analysis::baseline::build_baseline;
use analysis::club_profile::build_bag_profile;
use analysis::comparison::build_standing;
use analysis::dispersion::build_dispersion;
use contracts::bag::Bag;
use contracts::career::CareerCorpus;
use contracts::club::ClubId;
use contracts::club_profile::ClubProfile;

use crate::compare::compare;
use crate::reports::baseline::career_baseline;
use crate::reports::club_profile::club_profile;
use crate::reports::corpus::career_corpus;
use crate::reports::dispersion::career_dispersion;
use crate::reports::flag_mishit::flag_mishit_list;

/// `provenance.recorded_by` on every career vector.
pub const RUN_CAREER: &str = "scripts/conformance_vectors.py::_run_career";

/// The aggregates [`run_career`] answers, each a key of `expected` and of `expected.properties`.
pub const AGGREGATES: [&str; 4] = ["baseline", "dispersion", "standing", "bag_profile"];

/// The scripts whose reports [`run_career`] renders into `expected.reports` (the module doc). A
/// report's key is its script's name, or the name, `_` and a suffix ([`script_of`]). [M36 P15–P16]
pub const RENDERED: [&str; 5] = [
    "career_corpus",
    "career_baseline",
    "career_dispersion",
    "club_profile",
    "flag_mishit",
];

/// The scripts whose reports [`run_career`] does not render, which a re-record copies from the
/// committed vector unchanged ([`carry`]). Empty since P16 rendered the last two.
pub const CARRIED: [&str; 0] = [];

/// The script, of [`RENDERED`] and [`CARRIED`], whose report sits at `key` in `expected.reports`:
/// the key is the script's name (`career_corpus`) or the name, `_` and a suffix
/// (`career_corpus_verbose`, `club_profile_7i`, `flag_mishit_list`). `None` for a key no script
/// printed. No script's name is another's followed by `_`, so at most one matches.
pub fn script_of(key: &str) -> Option<&'static str> {
    RENDERED.iter().chain(&CARRIED).copied().find(|script| {
        key.strip_prefix(script)
            .is_some_and(|rest| rest.is_empty() || rest.starts_with('_'))
    })
}

/// Copy every [`CARRIED`] script's report from a committed vector's `expected` into `ours`, the
/// document [`run_career`] answered, so the re-record neither moves nor drops a report no renderer
/// runs yet (the module doc).
pub fn carry(ours: &mut Value, committed: &Value) {
    let Some(reports) = committed["reports"].as_object() else {
        return;
    };
    for (key, text) in reports {
        if script_of(key).is_some_and(|script| CARRIED.contains(&script)) {
            ours["reports"][key] = text.clone();
        }
    }
}

/// Where an adopted corpus's source is named: `provenance.source` begins with the storage vector's
/// path under the repository, as P3's recorder wrote it.
const ADOPTED_PREFIX: &str = "spec/vectors/";

/// **The runner**: every aggregate Rust builds from one career vector's input, in `expected`'s shape
/// less the [`CARRIED`] reports — the four aggregates as `model_dump(mode="json")` writes them,
/// `properties`, the derived counts each one answers (`_run_career`'s, key for key), and `reports`,
/// each [`RENDERED`] script's text as the recorder captured it (`_career_reports`): the three career
/// reports and `club_profile` plain and `_verbose`, `club_profile_<club>` plain for each of
/// `input.clubs`, and `flag_mishit_list`.
///
/// Reads `input.corpus` and `input.bag` for the aggregates, and `input.display_name`,
/// `input.versions.installed` (the corpus report names the installed engine) and `input.clubs` for
/// the reports. **`flag_mishit_list` is the bag profile built with no bag**, as `_list` builds it,
/// where `club_profile` is the profile with the case's bag: a club declared and never hit has a
/// `club_profile` block and no listing row. Does not read `career_version`, which is the gate's to
/// check and the re-record's to move. `Err` for a vector recorded by another runner or an input the
/// shapes refuse.
pub fn run_career(vector: &Value) -> Result<Value, String> {
    let id = vector["id"].as_str().unwrap_or("<a vector with no id>");
    if vector["provenance"]["recorded_by"].as_str() != Some(RUN_CAREER) {
        return Err(format!(
            "{id}: recorded by {:?}, a runner the career family does not have ({RUN_CAREER})",
            vector["provenance"]["recorded_by"]
        ));
    }
    let corpus: CareerCorpus = serde_json::from_value(vector["input"]["corpus"].clone())
        .map_err(|e| format!("{id}: input.corpus: {e}"))?;
    let bag: Option<Bag> = serde_json::from_value(vector["input"]["bag"].clone())
        .map_err(|e| format!("{id}: input.bag: {e}"))?;
    let display_name = vector["input"]["display_name"]
        .as_str()
        .ok_or_else(|| format!("{id}: input.display_name is not a string"))?;
    let installed = vector["input"]["versions"]["installed"]
        .as_i64()
        .ok_or_else(|| format!("{id}: input.versions.installed is not an integer"))?;
    let clubs: Vec<ClubId> = serde_json::from_value(vector["input"]["clubs"].clone())
        .map_err(|e| format!("{id}: input.clubs: {e}"))?;

    let baseline = build_baseline(&corpus);
    let dispersion = build_dispersion(&corpus);
    let standing = build_standing(&corpus);
    let bag_profile = build_bag_profile(&corpus, bag.as_ref());

    let categories: Map<String, Value> = bag_profile
        .clubs
        .iter()
        .map(|club| {
            (
                club.club.as_str().to_string(),
                json!(club.category().as_str()),
            )
        })
        .collect();
    let mut reports = Map::new();
    for (tail, verbose) in [("", false), ("_verbose", true)] {
        reports.insert(
            format!("career_corpus{tail}"),
            json!(career_corpus(&corpus, display_name, installed, verbose)),
        );
        reports.insert(
            format!("career_baseline{tail}"),
            json!(career_baseline(&baseline, &standing, display_name, verbose)),
        );
        reports.insert(
            format!("career_dispersion{tail}"),
            json!(career_dispersion(&dispersion, display_name, verbose)),
        );
        reports.insert(
            format!("club_profile{tail}"),
            json!(club_profile(&bag_profile, display_name, verbose, None)),
        );
    }
    for club in clubs {
        reports.insert(
            format!("club_profile_{club}"),
            json!(club_profile(&bag_profile, display_name, false, Some(club))),
        );
    }
    reports.insert(
        "flag_mishit_list".to_string(),
        json!(flag_mishit_list(
            &corpus.player_id,
            &build_bag_profile(&corpus, None)
        )),
    );
    Ok(json!({
        "baseline": baseline,
        "dispersion": dispersion,
        "standing": standing,
        "bag_profile": bag_profile,
        "properties": {
            "baseline": {
                "claims_ready": baseline.claims_ready(),
                "nothing_sayable": baseline.nothing_sayable(),
            },
            "dispersion": {
                "patterns_established": dispersion.patterns_established(),
                "nothing_established": dispersion.nothing_established(),
            },
            "standing": {
                "placements": standing.placements(),
                "nothing_placed": standing.nothing_placed(),
            },
            "bag_profile": {
                "clubs_used": club_ids(&bag_profile.clubs_used()),
                "clubs_declared": club_ids(&bag_profile.clubs_declared()),
                "categories": categories,
            },
        },
        "reports": reports,
    }))
}

fn club_ids(profiles: &[&ClubProfile]) -> Value {
    json!(profiles
        .iter()
        .map(|profile| profile.club.as_str())
        .collect::<Vec<_>>())
}

/// Every difference between what a vector recorded and what [`run_career`] answers, aggregate by
/// aggregate and then each aggregate's `properties`, every path rooted at `expected`. `reports` is
/// not compared here: its text is `tests/reports.rs`'s, to the character.
pub fn differences(expected: &Value, built: &Value) -> Vec<String> {
    let mut out = Vec::new();
    for key in AGGREGATES {
        out.extend(
            compare(&expected[key], &built[key])
                .iter()
                .map(|d| format!("{key}.{d}")),
        );
        out.extend(
            compare(&expected["properties"][key], &built["properties"][key])
                .iter()
                .map(|d| format!("properties.{key}.{d}")),
        );
    }
    out
}

/// The id of the storage vector whose `expected.corpus` this career vector's `input.corpus` is (the
/// module doc, "An adopted corpus is another vector's answer"), or `None` for a corpus built by hand.
///
/// Read from `provenance.source`, which P3's recorder began with the source's path for every adopted
/// case — `spec/vectors/storage/corpus/stale.json, expected.corpus` — and with prose for every other.
/// So a source that begins with a vector path is one, and the path up to the first `,` or space,
/// less `spec/vectors/` and its suffix, is the source's id. `Err` for a source that begins with the
/// prefix and names no storage corpus case, which is a vector this rule would misread.
pub fn adopted_from(vector: &Value) -> Result<Option<String>, String> {
    let id = vector["id"].as_str().unwrap_or("<a vector with no id>");
    let Some(rest) = vector["provenance"]["source"]
        .as_str()
        .and_then(|source| source.strip_prefix(ADOPTED_PREFIX))
    else {
        return Ok(None);
    };
    let path = rest.split([',', ' ']).next().unwrap_or_default();
    let source = path
        .strip_suffix(".json.gz")
        .or_else(|| path.strip_suffix(".json"))
        .filter(|source| source.starts_with("storage/corpus/"))
        .ok_or_else(|| {
            format!(
                "{id}: provenance.source names {ADOPTED_PREFIX}{path}, which is not a storage \
                 corpus case's file"
            )
        })?;
    Ok(Some(source.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sourced(source: &str) -> Value {
        json!({"id": "career/synthetic/x", "provenance": {"source": source}})
    }

    /// Both spellings P3's recorder wrote, a hand-built corpus, and a path that is not a storage
    /// corpus case.
    #[test]
    fn an_adopted_corpus_names_its_storage_vector() {
        assert_eq!(
            adopted_from(&sourced(
                "spec/vectors/storage/corpus/stale.json, expected.corpus"
            )),
            Ok(Some("storage/corpus/stale".to_string()))
        );
        assert_eq!(
            adopted_from(&sourced(
                "spec/vectors/storage/corpus/real.json.gz expected.corpus, and data/processed/golfers"
            )),
            Ok(Some("storage/corpus/real".to_string()))
        );
        assert_eq!(
            adopted_from(&sourced(
                "CorpusSwing built by scripts/conformance_vectors.py, as tests/analysis/ does"
            )),
            Ok(None)
        );
        assert_eq!(adopted_from(&json!({"provenance": {}})), Ok(None));
        for wrong in [
            "spec/vectors/storage/bundle/repairs.json, expected.files",
            "spec/vectors/career/synthetic/thin.json",
            "spec/vectors/storage/corpus/stale.txt",
        ] {
            let refused = adopted_from(&sourced(wrong)).expect_err(wrong);
            assert!(refused.contains("not a storage corpus case"), "{refused}");
        }
    }

    /// A report key belongs to the script it begins with, whole or followed by `_`, and nothing
    /// else does.
    #[test]
    fn a_report_key_names_its_script() {
        assert_eq!(script_of("career_corpus"), Some("career_corpus"));
        assert_eq!(script_of("career_corpus_verbose"), Some("career_corpus"));
        assert_eq!(script_of("club_profile_7i"), Some("club_profile"));
        assert_eq!(script_of("flag_mishit_list"), Some("flag_mishit"));
        for stray in ["career_corpusx", "career", "flag", "reports", ""] {
            assert_eq!(script_of(stray), None, "{stray:?}");
        }
    }

    /// Only a carried script's report is copied, and since P16 none is: a rendered one keeps the
    /// runner's text, a rendered key the runner lacks stays absent, and a key no script printed is
    /// left out, each for the comparator to report.
    #[test]
    fn carry_copies_the_carried_reports_alone() {
        let committed = json!({"reports": {
            "career_corpus": "committed",
            "club_profile_7i": "committed too",
            "flag_mishit_list": "and this",
            "stray": "dropped",
        }});
        let mut ours = json!({"reports": {"career_corpus": "rendered"}});
        carry(&mut ours, &committed);
        assert_eq!(ours, json!({"reports": {"career_corpus": "rendered"}}));
    }

    #[test]
    fn a_runner_this_module_does_not_have_is_an_err() {
        let vector = json!({"id": "career/x", "provenance": {"recorded_by": "by hand"}});
        let refused = run_career(&vector).expect_err("an unknown recorder");
        assert!(refused.starts_with("career/x: recorded by"), "{refused}");
    }
}
