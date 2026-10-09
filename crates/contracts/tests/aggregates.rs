//! Every dispersion, standing and bag profile the career family holds, read into this crate, and
//! every prose table those shapes carry held to frozen Python's word for word. [M36 P7]
//!
//! `spec/vectors/career/` records, per case, the `GolferDispersion`, `GolferStanding` and
//! `BagProfile` frozen Python built from the case's corpus, with the derived values `model_dump`
//! drops (`expected.properties`). The builders are not ported yet (P12), so this asks what the
//! *shapes* can be wrong about:
//!
//! - **Does every field survive the crossing?** Each aggregate reads and writes back as the same
//!   value, exactly, and answers its derived values (`patterns_established`, `placements`,
//!   `clubs_used`, `clubs_declared`, each club's `category`) as Python did.
//! - **Is every sentence the tables hold the one Python holds?** The reports print them, so a
//!   paraphrase is a report that prints something else. Two independent pins: every reading,
//!   target, tolerance and refusal the family *recorded* is the one these tables give, and every
//!   table entry is met by at least one recording; then every table, provenance included (which
//!   no aggregate carries), equals `tests/data/python_tables.json`, extracted from the Python
//!   modules at P7. The sentence *frames* around a table entry (`"no target for {name}: …"`) are
//!   `analysis`'s, so this matches the entry, not the frame.

mod common;

use std::collections::BTreeSet;

use common::{differences, read_vector, round_trip, vector_files};
use contracts::club_profile::{BagProfile, ClubProfile};
use contracts::comparison::{
    no_population_reason, tour_comparison_blocked, GolferStanding, MetricComparison, Standing,
    NO_LAUNCH_MONITOR_POPULATION, NO_MODEL_POPULATION, NO_POPULATION, SPREAD_NOT_COMPARABLE,
    STANDING_READING, TOUR_COMPARISON_BLOCKED,
};
use contracts::dispersion::{
    target_for, DispersionPattern, Finding, GolferDispersion, MetricDispersion, METRIC_TARGETS,
    PATTERN_READING, SCATTER_ONLY_READING, SESSION_DRIFT_FACTOR,
};
use serde_json::{json, Value};

const PYTHON_TABLES: &str = include_str!("data/python_tables.json");

fn compare(at: &str, written: &Value, given: &Value, failures: &mut Vec<String>) {
    let mut out = Vec::new();
    differences(written, given, "", &mut out);
    failures.extend(out.into_iter().map(|d| format!("{at}: {d}")));
}

fn club_ids(profiles: &[&ClubProfile]) -> Value {
    json!(profiles
        .iter()
        .map(|profile| profile.club.as_str())
        .collect::<Vec<_>>())
}

#[test]
fn every_aggregate_python_recorded_reads_writes_back_and_answers_as_it_did() {
    let (mut dispersions, mut standings, mut bags, mut clubs) = (0, 0, 0, 0);
    let mut failures = Vec::new();

    for path in vector_files("career") {
        let vector = read_vector(&path);
        let id = vector["id"].as_str().expect("a vector id");
        let (expected, properties) = (&vector["expected"], &vector["expected"]["properties"]);

        let at = format!("{id} dispersion");
        if let Some(dispersion) =
            round_trip::<GolferDispersion>(&at, &expected["dispersion"], &mut failures)
        {
            dispersions += 1;
            let derived = json!({
                "patterns_established": dispersion.patterns_established(),
                "nothing_established": dispersion.nothing_established(),
            });
            compare(&at, &derived, &properties["dispersion"], &mut failures);
        }

        let at = format!("{id} standing");
        if let Some(standing) =
            round_trip::<GolferStanding>(&at, &expected["standing"], &mut failures)
        {
            standings += 1;
            let derived = json!({
                "placements": standing.placements(),
                "nothing_placed": standing.nothing_placed(),
            });
            compare(&at, &derived, &properties["standing"], &mut failures);
        }

        let at = format!("{id} bag_profile");
        if let Some(profile) =
            round_trip::<BagProfile>(&at, &expected["bag_profile"], &mut failures)
        {
            bags += 1;
            clubs += profile.clubs.len();
            let categories: serde_json::Map<String, Value> = profile
                .clubs
                .iter()
                .map(|club| {
                    (
                        club.club.as_str().to_string(),
                        json!(club.category().as_str()),
                    )
                })
                .collect();
            let derived = json!({
                "clubs_used": club_ids(&profile.clubs_used()),
                "clubs_declared": club_ids(&profile.clubs_declared()),
                "categories": categories,
            });
            compare(&at, &derived, &properties["bag_profile"], &mut failures);
            for club in &profile.clubs {
                if profile.profile_for(club.club) != Some(club) {
                    failures.push(format!("{at}: profile_for({}) finds another", club.club));
                }
            }
        }
    }

    for (what, count) in [
        ("dispersions", dispersions),
        ("standings", standings),
        ("bag profiles", bags),
        ("club profiles", clubs),
    ] {
        assert!(
            count > 0,
            "the walk met no {what}, so it proves nothing about them"
        );
    }
    assert!(
        failures.is_empty(),
        "{} differences between crates/contracts and what frozen Python recorded:\n  {}",
        failures.len(),
        failures.join("\n  ")
    );
}

/// Which table entries the recorded sentences met, so an entry no recording reaches is named.
#[derive(Default)]
struct Met {
    patterns: BTreeSet<&'static str>,
    scatter_only: usize,
    registered: BTreeSet<&'static str>,
    reasons: BTreeSet<&'static str>,
    unregistered: usize,
    standings: BTreeSet<&'static str>,
    populations: BTreeSet<&'static str>,
    blocked: BTreeSet<String>,
    spread: usize,
}

const NOT_REGISTERED: &str = "is not registered in METRIC_TARGETS";

fn dispersion_sentences(
    at: &str,
    metric: &MetricDispersion,
    met: &mut Met,
    fail: &mut Vec<String>,
) {
    let mut fail = |problem: String| fail.push(format!("{at}: {problem}"));
    let name = metric.name.as_str();

    match target_for(name) {
        None => {
            met.unregistered += 1;
            if (metric.target, metric.tolerance) != (None, None) {
                fail("unregistered, and carries a target or a tolerance".into());
            }
            if metric.bias != Finding::Withheld || metric.scatter != Finding::Withheld {
                fail("unregistered, and a finding was asked".into());
            }
            let frame = format!("{name} {NOT_REGISTERED}");
            if !metric
                .unavailable
                .iter()
                .any(|reason| reason.starts_with(&frame))
            {
                fail(format!(
                    "unregistered, and no refusal names {NOT_REGISTERED}"
                ));
            }
        }
        Some(row) => {
            met.registered.insert(row.metric);
            if metric.target != row.target || metric.tolerance != Some(row.tolerance) {
                fail(format!(
                    "recorded target {:?} and tolerance {:?}, the table says {:?} and {}",
                    metric.target, metric.tolerance, row.target, row.tolerance
                ));
            }
            let reason = format!("no target for {name}: {}", row.no_target_reason);
            let says_why = metric.unavailable.iter().filter(|r| **r == reason).count();
            match (row.target, says_why) {
                (None, 1) => {
                    met.reasons.insert(row.metric);
                }
                (Some(_), 0) => {}
                _ => fail(format!(
                    "the table's no-target reason appears {says_why} times; recorded {:?}",
                    metric.unavailable
                )),
            }
        }
    }
    for reason in &metric.unavailable {
        let known = reason.starts_with("no target for ") || reason.contains(NOT_REGISTERED);
        if !known {
            fail(format!("a refusal no table accounts for: {reason:?}"));
        }
    }

    match (metric.pattern, metric.points_at.as_deref()) {
        (Some(pattern), Some(points_at)) if points_at == pattern.reading() => {
            met.patterns.insert(pattern.as_str());
        }
        (None, Some(SCATTER_ONLY_READING)) => met.scatter_only += 1,
        (None, None) => {}
        (pattern, points_at) => fail(format!(
            "pattern {pattern:?} points at a sentence no table holds: {points_at:?}"
        )),
    }
}

fn standing_sentences(at: &str, metric: &MetricComparison, met: &mut Met, fail: &mut Vec<String>) {
    let mut fail = |problem: String| fail.push(format!("{at}: {problem}"));
    let name = metric.name.as_str();

    if metric.reading.as_deref() != metric.standing.reading() {
        fail(format!(
            "{:?} reads {:?}, the table says {:?}",
            metric.standing,
            metric.reading,
            metric.standing.reading()
        ));
    } else if metric.standing != Standing::Withheld {
        met.standings.insert(metric.standing.as_str());
    }

    for reason in &metric.unavailable {
        if *reason == no_population_reason(name, &metric.source) {
            let sentence = &reason[name.len() + 2..];
            for (label, table) in [
                ("launch monitor", NO_LAUNCH_MONITOR_POPULATION),
                ("model", NO_MODEL_POPULATION),
                ("generic", NO_POPULATION),
            ] {
                if sentence == table {
                    met.populations.insert(label);
                }
            }
        } else if tour_comparison_blocked(name).is_some_and(|blocked| reason.ends_with(blocked)) {
            met.blocked.insert(name.to_string());
        } else if reason.starts_with(name) && reason.ends_with(SPREAD_NOT_COMPARABLE) {
            met.spread += 1;
        } else {
            fail(format!("a refusal no table accounts for: {reason:?}"));
        }
    }
}

#[test]
fn every_sentence_the_career_family_recorded_is_one_these_tables_hold() {
    let mut met = Met::default();
    let mut failures = Vec::new();

    for path in vector_files("career") {
        let vector = read_vector(&path);
        let id = vector["id"].as_str().expect("a vector id");
        let expected = &vector["expected"];

        let dispersion: GolferDispersion =
            serde_json::from_value(expected["dispersion"].clone()).expect("a dispersion");
        for (name, metric) in &dispersion.metrics {
            let at = format!("{id} dispersion[{name}]");
            dispersion_sentences(&at, metric, &mut met, &mut failures);
        }
        let profile: BagProfile =
            serde_json::from_value(expected["bag_profile"].clone()).expect("a bag profile");
        for club in &profile.clubs {
            for (name, metric) in &club.dispersion {
                let at = format!("{id} bag_profile[{}].dispersion[{name}]", club.club);
                dispersion_sentences(&at, metric, &mut met, &mut failures);
            }
        }
        let standing: GolferStanding =
            serde_json::from_value(expected["standing"].clone()).expect("a standing");
        for (name, metric) in &standing.metrics {
            let at = format!("{id} standing[{name}]");
            standing_sentences(&at, metric, &mut met, &mut failures);
        }
    }
    assert!(
        failures.is_empty(),
        "{} recorded sentences these tables do not hold:\n  {}",
        failures.len(),
        failures.join("\n  ")
    );

    // Every entry was met by a recording, so each one above was asked rather than passed by absence.
    let mut unmet = Vec::new();
    for (pattern, _) in PATTERN_READING {
        if !met.patterns.contains(pattern.as_str()) {
            unmet.push(format!("PATTERN_READING[{}]", pattern.as_str()));
        }
    }
    for row in &METRIC_TARGETS {
        if !met.registered.contains(row.metric) {
            unmet.push(format!(
                "METRIC_TARGETS[{}]'s target and tolerance",
                row.metric
            ));
        }
        if row.target.is_none() && !met.reasons.contains(row.metric) {
            unmet.push(format!("METRIC_TARGETS[{}].no_target_reason", row.metric));
        }
    }
    for (standing, _) in STANDING_READING {
        if !met.standings.contains(standing.as_str()) {
            unmet.push(format!("STANDING_READING[{}]", standing.as_str()));
        }
    }
    for label in ["launch monitor", "model", "generic"] {
        if !met.populations.contains(label) {
            unmet.push(format!("the {label} population sentence"));
        }
    }
    for (metric, _) in TOUR_COMPARISON_BLOCKED {
        if !met.blocked.contains(metric) {
            unmet.push(format!("TOUR_COMPARISON_BLOCKED[{metric}]"));
        }
    }
    for (what, count) in [
        ("SCATTER_ONLY_READING", met.scatter_only),
        ("SPREAD_NOT_COMPARABLE", met.spread),
        ("an unregistered metric", met.unregistered),
    ] {
        if count == 0 {
            unmet.push(what.to_string());
        }
    }
    assert!(
        unmet.is_empty(),
        "no career vector recorded these, so nothing here asked them:\n  {}",
        unmet.join("\n  ")
    );
}

fn strings<const N: usize>(names: [&str; N]) -> Value {
    json!(names.as_slice())
}

#[test]
fn every_table_is_frozen_pythons_word_for_word() {
    let python: Value = serde_json::from_str(PYTHON_TABLES).expect("python_tables.json");
    let (dispersion, comparison) = (&python["dispersion"], &python["comparison"]);
    let mut failures = Vec::new();
    let mut check =
        |at: &str, ours: Value, theirs: &Value| compare(at, &ours, theirs, &mut failures);

    check(
        "Finding",
        strings(Finding::ALL.map(Finding::as_str)),
        &dispersion["Finding"],
    );
    check(
        "DispersionPattern",
        strings(DispersionPattern::ALL.map(DispersionPattern::as_str)),
        &dispersion["DispersionPattern"],
    );
    let readings: serde_json::Map<String, Value> = PATTERN_READING
        .iter()
        .map(|(pattern, sentence)| (pattern.as_str().to_string(), json!(sentence)))
        .collect();
    check(
        "PATTERN_READING",
        json!(readings),
        &dispersion["PATTERN_READING"],
    );
    check(
        "SCATTER_ONLY_READING",
        json!(SCATTER_ONLY_READING),
        &dispersion["SCATTER_ONLY_READING"],
    );
    // In declaration order, `key` beside `metric` because Python's dict holds both.
    let targets: Vec<Value> = METRIC_TARGETS
        .iter()
        .map(|row| {
            json!({
                "key": row.metric,
                "metric": row.metric,
                "target": row.target,
                "tolerance": row.tolerance,
                "provenance": row.provenance,
                "no_target_reason": row.no_target_reason,
            })
        })
        .collect();
    check(
        "METRIC_TARGETS",
        json!(targets),
        &dispersion["METRIC_TARGETS"],
    );
    check(
        "SESSION_DRIFT_FACTOR",
        json!(SESSION_DRIFT_FACTOR),
        &dispersion["SESSION_DRIFT_FACTOR"],
    );

    check(
        "Standing",
        strings(Standing::ALL.map(Standing::as_str)),
        &comparison["Standing"],
    );
    let readings: serde_json::Map<String, Value> = STANDING_READING
        .iter()
        .map(|(standing, sentence)| (standing.as_str().to_string(), json!(sentence)))
        .collect();
    check(
        "STANDING_READING",
        json!(readings),
        &comparison["STANDING_READING"],
    );
    let blocked: serde_json::Map<String, Value> = TOUR_COMPARISON_BLOCKED
        .iter()
        .map(|(metric, reason)| (metric.to_string(), json!(reason)))
        .collect();
    check(
        "TOUR_COMPARISON_BLOCKED",
        json!(blocked),
        &comparison["TOUR_COMPARISON_BLOCKED"],
    );
    for (name, ours) in [
        ("NO_LAUNCH_MONITOR_POPULATION", NO_LAUNCH_MONITOR_POPULATION),
        ("NO_MODEL_POPULATION", NO_MODEL_POPULATION),
        ("NO_POPULATION", NO_POPULATION),
        ("SPREAD_NOT_COMPARABLE", SPREAD_NOT_COMPARABLE),
    ] {
        check(name, json!(ours), &comparison[name]);
    }
    let reasons = comparison["no_population_reason"]
        .as_object()
        .expect("no_population_reason");
    assert!(!reasons.is_empty());
    for (source, theirs) in reasons {
        check(
            &format!("no_population_reason(\"m\", {source:?})"),
            json!(no_population_reason("m", source)),
            theirs,
        );
    }

    assert!(
        failures.is_empty(),
        "{} differences from frozen Python's tables:\n  {}",
        failures.len(),
        failures.join("\n  ")
    );
}
