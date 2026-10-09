//! The career family against `crates/analysis`. [M36 P11, P12; the runner lifted at P13]
//!
//! `spec/vectors/career/` was recorded once from frozen Python (M36 P3) by
//! `conformance_vectors.py::_run_career`: a corpus and a bag in, and every aggregate the four career
//! scripts print out (`expected.{baseline, dispersion, standing, bag_profile}`), beside the derived
//! counts `model_dump` leaves out (`expected.properties`) and the scripts' report text
//! (`expected.reports`). This runs the same input through the Rust builders and compares under
//! `docs/CONFORMANCE.md` §3's rules, through [`golf_core::compare`], the comparator
//! `golf-core rerecord` re-records with.
//!
//! # What runs, and what is gated elsewhere
//!
//! P11 ported `build_baseline` and P12 the other three, so every aggregate and every derived count
//! is checked on every vector, the real one included ([`CHECKED`]). [`DEFERRED`] is empty and stays
//! declared: a key the recorder grows lands there with the phase that ports its builder, never past a
//! blanket ignore. The report text is not this gate's ([`ELSEWHERE`]).
//! [`every_expected_key_is_checked_or_deferred_by_name`] fails if a vector carries a key none of the
//! three lists names, so a new key cannot go unchecked silently.
//!
//! The runner is [`run_career`], one document per vector in `expected`'s own shape less the
//! carried reports. It was this file's until P13 lifted it into `golf_core::career_family`, so that
//! `golf-core rerecord` and this gate run one definition; [`built`] is it, after the version check
//! a gate makes and a re-record must not.
//!
//! # An adopted corpus is its storage vector's answer
//!
//! The storage family's corpus cases each have a career twin whose corpus is that case's answer,
//! copied (`career_family`'s module doc). The re-record derives the copy from its source on every
//! career run; [`every_adopted_corpus_is_its_storage_vectors_answer`] holds the committed families
//! to it, so a copy that drifted fails here before a re-record has to say so.
//!
//! # The builder's promise, which no shape holds
//!
//! `contracts::baseline` holds a recorded baseline to its floors (P6), but "a ready claim's
//! statistics are present" is something only the builder can promise: `MetricBaseline` would
//! deserialize a ready `center` with no mean. [`the_builder_withholds_exactly_what_it_refuses`]
//! holds every baseline Rust builds from the family to it, both ways, the per-club ones included.
//!
//! # And the bits, on the target the family was recorded on
//!
//! §3's `RTOL` is sized for a different summation order, so the gate cannot see whether the port
//! sums the way CPython does. [`every_aggregate_float_is_frozen_pythons_to_the_bit`] can, and its doc
//! carries the controls that showed the gate could not.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use analysis::baseline::build_baseline;
use analysis::club_profile::build_bag_profile;
use contracts::bag::Bag;
use contracts::baseline::{BaselineClaim, MetricBaseline};
use contracts::career::{CareerCorpus, CAREER_VERSION};
use flate2::read::GzDecoder;
use golf_core::career_family::{
    adopted_from, differences, run_career, AGGREGATES, CARRIED, RENDERED, RUN_CAREER,
};
use serde_json::Value;

/// The keys of `expected` (and of `expected.properties`) this gate checks: the four aggregates, and
/// `properties`, which holds their derived counts.
const CHECKED: [&str; 5] = [
    "baseline",
    "dispersion",
    "standing",
    "bag_profile",
    "properties",
];

/// Keys this gate will check and does not yet, each with the phase that ports its builder. Empty
/// since P12, and kept so that the next key has a named place to wait in.
const DEFERRED: [(&str, &str); 0] = [];

/// Keys that are gated, but not here.
const ELSEWHERE: [(&str, &str); 1] = [(
    "reports",
    "the five scripts' text, gated to the character by crates/core/tests/reports.rs (P15 the \
     three career scripts, P16 club_profile and flag_mishit)",
)];

fn vectors_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../spec/vectors")
        .canonicalize()
        .expect("spec/vectors/ is committed")
}

fn read_vector(path: &Path) -> Value {
    let bytes = fs::read(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let text = if path.extension().is_some_and(|ext| ext == "gz") {
        let mut text = String::new();
        GzDecoder::new(bytes.as_slice())
            .read_to_string(&mut text)
            .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        text
    } else {
        String::from_utf8(bytes).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
    };
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// Every vector in the career family, sorted by id.
fn career_vectors() -> Vec<(String, Value)> {
    family_vectors("career")
}

/// Every vector in one family's directory, sorted by id. Duplicated from `tests/storage.rs` (and
/// `crates/contracts/tests/common/`), for `engine.rs`'s reason: integration tests are separate
/// binaries.
fn family_vectors(family: &str) -> Vec<(String, Value)> {
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
    collect(&vectors_dir().join(family), &mut paths);
    let mut vectors: Vec<(String, Value)> = paths
        .iter()
        .map(|path| {
            let vector = read_vector(path);
            let id = vector["id"]
                .as_str()
                .expect("a vector carries its id")
                .to_string();
            (id, vector)
        })
        .collect();
    vectors.sort_by(|a, b| a.0.cmp(&b.0));
    assert!(
        !vectors.is_empty(),
        "no vectors under spec/vectors/{family}"
    );
    vectors
}

/// The checks a gate makes before it runs a vector: the version key, which the re-record moves and
/// so [`run_career`] does not read, and the recorder.
fn checked(id: &str, vector: &Value) {
    assert_eq!(
        vector["career_version"].as_i64(),
        Some(CAREER_VERSION),
        "{id}: recorded at career_version {} against a port at {CAREER_VERSION}; re-record with \
         `golf-core rerecord` in the change that bumped it",
        vector["career_version"]
    );
    assert_eq!(
        vector["provenance"]["recorded_by"].as_str(),
        Some(RUN_CAREER),
        "{id}: recorded by a runner this gate does not have"
    );
}

/// The vector's corpus, read as `CareerCorpus` reads it, after [`checked`].
fn corpus_of(id: &str, vector: &Value) -> CareerCorpus {
    checked(id, vector);
    serde_json::from_value(vector["input"]["corpus"].clone())
        .unwrap_or_else(|e| panic!("{id}: input.corpus: {e}"))
}

/// The vector's declared bag, `None` where the recorder wrote `null` (a golfer with no bag).
fn bag_of(id: &str, vector: &Value) -> Option<Bag> {
    serde_json::from_value(vector["input"]["bag"].clone())
        .unwrap_or_else(|e| panic!("{id}: input.bag: {e}"))
}

/// [`run_career`] on one vector, after [`checked`].
fn built(id: &str, vector: &Value) -> Value {
    checked(id, vector);
    run_career(vector).unwrap_or_else(|why| panic!("{why}"))
}

// ------------------------------------------------------------------------------------------ gates

/// **The gate**: every career vector's input, built into all four aggregates by Rust and compared in
/// full with what frozen Python built from it, in one report.
#[test]
fn every_career_vector_builds_its_aggregates() {
    let mut found = Vec::new();
    let (mut ids, mut metrics, mut clubs) = (Vec::new(), 0, 0);
    for (id, vector) in career_vectors() {
        let document = built(&id, &vector);
        metrics += document["baseline"]["metrics"]
            .as_object()
            .map_or(0, |m| m.len());
        clubs += document["bag_profile"]["clubs"]
            .as_array()
            .map_or(0, Vec::len);
        found.extend(
            differences(&vector["expected"], &document)
                .into_iter()
                .map(|d| format!("{id}: {d}")),
        );
        ids.push(id);
    }
    assert!(
        ids.iter().any(|id| id.starts_with("career/real/")),
        "the real case is missing from the family"
    );
    assert!(
        found.is_empty(),
        "{} difference(s) across {} vectors:\n{}",
        found.len(),
        ids.len(),
        found.join("\n")
    );
    println!(
        "{} vectors conform on {AGGREGATES:?} ({metrics} metrics, {clubs} club profiles): {}",
        ids.len(),
        ids.join(", ")
    );
}

/// What a claim's verdict owes the fields it gates: ready means every statistic it opens is there,
/// withheld means none is. Empty is the pass.
fn promise_breaches(metric: &MetricBaseline) -> Vec<String> {
    let mut out = Vec::new();
    let mut walked: Vec<BaselineClaim> = metric.ready.clone();
    walked.extend(metric.withheld.iter().map(|w| w.claim));
    walked.sort_by_key(|claim| *claim as usize);
    if walked != BaselineClaim::ALL {
        out.push(format!(
            "ready {:?} and withheld do not partition the claims",
            metric.ready
        ));
    }
    let session_means = metric.sessions.iter().map(|s| s.mean.is_some());
    for claim in BaselineClaim::ALL {
        let present: Vec<bool> = match claim {
            BaselineClaim::Center => vec![
                metric.mean.is_some(),
                metric.mean_ci.is_some(),
                metric.median.is_some(),
            ],
            BaselineClaim::Spread => vec![
                metric.sd.is_some(),
                metric.sd_ci.is_some(),
                metric.minimum.is_some(),
                metric.maximum.is_some(),
            ],
            BaselineClaim::Trend => session_means.clone().collect(),
        };
        let ready = metric.supports(claim);
        if present.iter().any(|&p| p != ready) {
            out.push(format!(
                "{claim:?} is {} but its statistics read {present:?}",
                if ready { "ready" } else { "withheld" }
            ));
        }
    }
    out
}

/// **The builder's promise**, on every metric of every baseline Rust builds from the family, the
/// golfer's and each club's: `ready` and `withheld` partition the claims, a ready claim's statistics
/// are all present (a trend's are the per-session means), and a withheld claim's are all absent.
#[test]
fn the_builder_withholds_exactly_what_it_refuses() {
    let mut breaches = Vec::new();
    let (mut ready, mut withheld) = (0, 0);
    for (id, vector) in career_vectors() {
        let corpus = corpus_of(&id, &vector);
        let profile = build_bag_profile(&corpus, bag_of(&id, &vector).as_ref());
        let golfer = build_baseline(&corpus).metrics;
        let per_club = profile.clubs.iter().flat_map(|club| {
            club.metrics
                .iter()
                .map(move |(name, metric)| (format!("{}: {name}", club.club), metric))
        });
        let every = golfer
            .iter()
            .map(|(name, metric)| (name.clone(), metric))
            .chain(per_club);
        for (name, metric) in every {
            ready += metric.ready.len();
            withheld += metric.withheld.len();
            breaches.extend(
                promise_breaches(metric)
                    .into_iter()
                    .map(|b| format!("{id}: {name}: {b}")),
            );
        }
    }
    // The family has to reach both sides for the walk to hold anything.
    assert!(
        ready > 0 && withheld > 0,
        "{ready} ready, {withheld} withheld"
    );
    assert!(breaches.is_empty(), "{}", breaches.join("\n"));
}

/// Every key a vector's `expected` (and its `properties`) carries is checked here, deferred to a
/// named phase, or gated elsewhere; every name in those lists is a key the family carries; and
/// every aggregate [`built`] answers is one [`CHECKED`] names, so nothing is built and then ignored.
#[test]
fn every_expected_key_is_checked_or_deferred_by_name() {
    let named = |key: &str| {
        CHECKED.contains(&key)
            || DEFERRED.iter().any(|(k, _)| *k == key)
            || ELSEWHERE.iter().any(|(k, _)| *k == key)
    };
    let mut seen = std::collections::BTreeSet::new();
    for (id, vector) in career_vectors() {
        let expected = vector["expected"].as_object().expect("an expected object");
        let properties = expected["properties"]
            .as_object()
            .expect("a properties object");
        for key in expected.keys().chain(properties.keys()) {
            assert!(
                named(key),
                "{id}: expected key {key:?} is checked by nothing"
            );
            seen.insert(key.clone());
        }
    }
    for key in CHECKED
        .iter()
        .chain(DEFERRED.iter().map(|(k, _)| k))
        .chain(ELSEWHERE.iter().map(|(k, _)| k))
    {
        assert!(seen.contains(*key), "{key:?} is named but no vector has it");
    }
    for key in AGGREGATES {
        assert!(CHECKED.contains(&key), "{key:?} is built and not checked");
    }
    // `reports` holds every script's text, rendered by the runner or carried by the re-record, and
    // this gate compares neither, so it has to be named as gated somewhere else.
    assert!(
        RENDERED.len() + CARRIED.len() == 5 && ELSEWHERE.iter().any(|(k, _)| *k == "reports"),
        "the five scripts' reports are gated nowhere"
    );
    println!("deferred: {DEFERRED:?}; elsewhere: {ELSEWHERE:?}");
}

/// Every float in two documents that is not the same f64, by path: the bits, not `RTOL`.
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
                out.push(format!("{path}: {e:?} recorded, {a:?} built"));
            }
        }
        _ => {}
    }
}

/// **Every float the builders produce is frozen Python's to the bit**, on the target the family was
/// recorded on (CPython 3.13, x86_64 Windows, the UCRT's `pow`), across all four aggregates. Keys,
/// refusals and every other value are [`every_career_vector_builds_its_aggregates`]'s; this walks
/// only the floats both sides have, and prints how many each aggregate holds.
///
/// The gate above compares within §3's `RTOL`, which is sized for a different summation order, and
/// so cannot see the arithmetic choices `analysis` makes. Measured at P11: summing with a plain fold
/// instead of CPython's compensated `sum()` moved 203 of the family's 1,286 baseline floats in a bit
/// and passed the gate. Every one of those bits is a `.Nf` in a report that could land on the other
/// side of a half. The `**` choice is held by `stats`' own pin instead, since this family is blind to
/// it: the product passed here as well, all 1,286 bit-identical. P12's controls on `dispersion`'s
/// within-session spread are in the plan's findings. Held to the target because another libm may
/// answer `pow`'s last bit differently, and CPython over it would.
#[cfg(all(target_os = "windows", target_env = "msvc"))]
#[test]
fn every_aggregate_float_is_frozen_pythons_to_the_bit() {
    let mut floats = [0usize; AGGREGATES.len()];
    let mut out = Vec::new();
    for (id, vector) in career_vectors() {
        let document = built(&id, &vector);
        for (slot, key) in AGGREGATES.iter().enumerate() {
            let mut here = Vec::new();
            bit_differences(&vector["expected"][key], &document[key], key, &mut here);
            floats[slot] += count_floats(&vector["expected"][key]);
            out.extend(here.into_iter().map(|d| format!("{id}: {d}")));
        }
    }
    let total: usize = floats.iter().sum();
    assert!(
        floats.iter().all(|&n| n > 0),
        "an aggregate holds no float to compare: {:?}",
        AGGREGATES.iter().zip(floats).collect::<Vec<_>>()
    );
    assert!(
        out.is_empty(),
        "{} of {total} floats differ in a bit:\n{}",
        out.len(),
        out.join("\n")
    );
    println!(
        "{total} floats bit-identical: {:?}",
        AGGREGATES.iter().zip(floats).collect::<Vec<_>>()
    );
}

fn count_floats(value: &Value) -> usize {
    match value {
        Value::Object(map) => map.values().map(count_floats).sum(),
        Value::Array(items) => items.iter().map(count_floats).sum(),
        Value::Number(n) => usize::from(n.is_f64()),
        _ => 0,
    }
}

/// **Every adopted corpus is its storage vector's answer, exactly** (the M36 plan's call 2):
/// each career vector whose `provenance.source` names a storage corpus case carries that case's
/// `expected.corpus` as its `input.corpus`, value for value, so the two families agree before any
/// re-record derives one from the other. The real golfer is one of them.
#[test]
fn every_adopted_corpus_is_its_storage_vectors_answer() {
    let storage: std::collections::BTreeMap<String, Value> =
        family_vectors("storage").into_iter().collect();
    let mut adopted = Vec::new();
    for (id, vector) in career_vectors() {
        let Some(source) = adopted_from(&vector).unwrap_or_else(|why| panic!("{why}")) else {
            continue;
        };
        let answer = &storage
            .get(&source)
            .unwrap_or_else(|| panic!("{id} adopts {source}, which is not a storage vector"))
            ["expected"]["corpus"];
        assert!(
            answer.is_object(),
            "{id} adopts {source}, which is not a corpus case"
        );
        assert!(
            &vector["input"]["corpus"] == answer,
            "{id}: input.corpus is not {source}'s expected.corpus"
        );
        adopted.push(id);
    }
    assert!(
        adopted.iter().any(|id| id.starts_with("career/real/")),
        "the real golfer adopts no corpus: {adopted:?}"
    );
    println!("{} career vectors adopt a storage corpus", adopted.len());
}
