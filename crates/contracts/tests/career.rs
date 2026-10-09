//! Every corpus and baseline the two M36 families hold, read into this crate and asked what frozen
//! Python answered. [M36 P6]
//!
//! `spec/vectors/storage/corpus/` ends each case in a `CareerCorpus` (`read_corpus`'s answer) and in
//! the corpus narrowed once per `input.narrowings` entry; `spec/vectors/career/` starts each case
//! from one and records the `PersonalBaseline` built from it. Neither the reader nor the builder is
//! ported yet (P10, P11), so this asks what the *shapes* can be wrong about:
//!
//! - **Does every field survive the crossing?** Each corpus and baseline reads and writes back as
//!   the same value, exactly: no arithmetic happened, so every float keeps its bits and every
//!   timestamp its spelling.
//! - **Do the rules on the shape answer as Python's did?** The derived counts a report prints and
//!   `model_dump` drops (`expected.properties`, P1 finding 3, and the baseline's two under P3's), each
//!   narrowing (`CareerCorpus::narrowed_to` over the recorded corpus is the recorded narrowed corpus,
//!   counts recomputed), `count_metrics` over a corpus's swings (its own `metric_counts` and
//!   `unknown_sources`, which is how `read_corpus` produced them), and the guard's floor tables
//!   (every claim the career family withheld names the floor `minimum_n` and `minimum_sessions` give,
//!   and every claim it made clears them).
//!
//! What builds a corpus or a baseline is not asked here: that is `crates/core/tests/{storage,
//! career}.rs`, from P10 and P11.

mod common;

use common::{differences, read_vector, round_trip, vector_files};
use contracts::baseline::{
    minimum_n, minimum_sessions, BaselineClaim, MetricBaseline, PersonalBaseline, DEFAULT_MINIMUM_N,
};
use contracts::career::{count_metrics, CareerCorpus, Narrowing, CAREER_VERSION};
use serde_json::{json, Value};

/// The counts `scripts/conformance_vectors.py::_corpus_answer` records beside a corpus.
fn corpus_properties(corpus: &CareerCorpus) -> Value {
    json!({
        "distinct_swings": corpus.distinct_swings(),
        "distinct_shots": corpus.distinct_shots(),
        "distinct_sessions": corpus.distinct_sessions(),
        "untagged_swings": corpus.untagged_swings(),
        "duplicates_collapsed": corpus.duplicates_collapsed(),
        "shot_conflicts": corpus.shot_conflicts(),
        "mishit_shots": corpus.mishit_shots(),
        "mishit_refs": corpus.mishit_refs(),
        "mishit_shots_unconfirmed": corpus.mishit_shots_unconfirmed(),
    })
}

/// What the walks met, so a walk that met nothing cannot pass.
#[derive(Default)]
struct Seen {
    corpora: usize,
    narrowings: usize,
    with_unknown_sources: usize,
    with_mishits: usize,
    baselines: usize,
    metric_baselines: usize,
    club_metric_baselines: usize,
    ready: usize,
    withheld: usize,
    withheld_on_sessions: usize,
    overridden_floors: usize,
}

fn compare(at: &str, written: &Value, given: &Value, failures: &mut Vec<String>) {
    let mut out = Vec::new();
    differences(written, given, "", &mut out);
    failures.extend(out.into_iter().map(|d| format!("{at}: {d}")));
}

/// The rules a corpus carries, asked of one Python recorded. Its `properties` are checked by the
/// caller where the vector holds them.
fn corpus_rules(at: &str, corpus: &CareerCorpus, seen: &mut Seen, failures: &mut Vec<String>) {
    seen.corpora += 1;
    seen.with_unknown_sources += usize::from(!corpus.unknown_sources.is_empty());
    seen.with_mishits += usize::from(corpus.mishit_shots() > 0);

    let (counts, unknown) = count_metrics(&corpus.swings);
    if counts != corpus.metric_counts || unknown != corpus.unknown_sources {
        failures.push(format!(
            "{at}: count_metrics answers {counts:?} and {unknown:?}, the corpus holds {:?} and {:?}",
            corpus.metric_counts, corpus.unknown_sources
        ));
    }
    // `read_corpus` computes the four recomputed fields with the expressions `narrowed_to` uses, so
    // a corpus read off disk is a fixed point of the empty narrowing.
    let unnarrowed = corpus.narrowed_to(&Narrowing::default());
    compare(
        &format!("{at} narrowed_to(no narrowing)"),
        &serde_json::to_value(&unnarrowed).unwrap(),
        &serde_json::to_value(corpus).unwrap(),
        failures,
    );
}

#[test]
fn every_corpus_python_recorded_reads_writes_back_and_answers_as_it_did() {
    let mut seen = Seen::default();
    let mut failures = Vec::new();

    for path in vector_files("storage/corpus") {
        let vector = read_vector(&path);
        let id = vector["id"].as_str().expect("a vector id");
        let (input, expected) = (&vector["input"], &vector["expected"]);

        let Some(corpus) =
            round_trip::<CareerCorpus>(&format!("{id} corpus"), &expected["corpus"], &mut failures)
        else {
            continue;
        };
        corpus_rules(id, &corpus, &mut seen, &mut failures);
        compare(
            &format!("{id} properties"),
            &corpus_properties(&corpus),
            &expected["properties"],
            &mut failures,
        );

        let narrowings = input["narrowings"].as_object().expect("input.narrowings");
        for (name, args) in narrowings {
            seen.narrowings += 1;
            let at = format!("{id} narrowed[{name}]");
            let narrowing: Narrowing = serde_json::from_value(args.clone())
                .unwrap_or_else(|e| panic!("{at}: the narrowing does not read: {e}"));
            let narrowed = corpus.narrowed_to(&narrowing);
            let recorded = &expected["narrowed"][name];
            compare(
                &format!("{at} corpus"),
                &serde_json::to_value(&narrowed).unwrap(),
                &recorded["corpus"],
                &mut failures,
            );
            compare(
                &format!("{at} properties"),
                &corpus_properties(&narrowed),
                &recorded["properties"],
                &mut failures,
            );
        }
    }

    for path in vector_files("career") {
        let vector = read_vector(&path);
        let id = vector["id"].as_str().expect("a vector id");
        let at = format!("{id} input.corpus");
        if let Some(corpus) =
            round_trip::<CareerCorpus>(&at, &vector["input"]["corpus"], &mut failures)
        {
            corpus_rules(&at, &corpus, &mut seen, &mut failures);
        }
    }

    for (what, count) in [
        ("corpora", seen.corpora),
        ("narrowings", seen.narrowings),
        ("corpora with an unknown source", seen.with_unknown_sources),
        ("corpora with a mishit", seen.with_mishits),
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

/// The guard's promise, asked of one metric baseline Python built: each claim is ready or withheld,
/// never both and never neither, each list in declaration order; a withheld claim names the floors
/// the tables give and falls short of one; a ready claim clears both; and a withheld claim's
/// statistics are absent.
fn claim_rules(at: &str, metric: &MetricBaseline, seen: &mut Seen, failures: &mut Vec<String>) {
    let mut fail = |problem: String| failures.push(format!("{at}: {problem}"));
    let withheld: Vec<BaselineClaim> = metric.withheld.iter().map(|w| w.claim).collect();
    let in_order = |claims: &[BaselineClaim]| {
        BaselineClaim::ALL
            .iter()
            .filter(|claim| claims.contains(claim))
            .copied()
            .collect::<Vec<_>>()
            == claims
    };
    let partition = BaselineClaim::ALL
        .iter()
        .all(|claim| metric.ready.contains(claim) != withheld.contains(claim));
    if !partition || !in_order(&metric.ready) || !in_order(&withheld) {
        fail(format!(
            "ready {:?} and withheld {withheld:?} are not a partition of the claims in order",
            metric.ready
        ));
    }

    for &claim in &metric.ready {
        seen.ready += 1;
        let (need_n, need_sessions) = (minimum_n(&metric.name, claim), minimum_sessions(claim));
        if metric.n < need_n || metric.n_sessions < need_sessions {
            fail(format!(
                "{claim:?} is ready at n={} over {} sessions, under the floor {need_n}/{need_sessions}",
                metric.n, metric.n_sessions
            ));
        }
    }
    for refusal in &metric.withheld {
        seen.withheld += 1;
        seen.withheld_on_sessions += usize::from(refusal.have_sessions < refusal.need_sessions);
        let claim = refusal.claim;
        let floors = (minimum_n(&metric.name, claim), minimum_sessions(claim));
        if (refusal.need_n, refusal.need_sessions) != floors {
            fail(format!(
                "{claim:?} withheld needing {}/{}, the tables say {floors:?}",
                refusal.need_n, refusal.need_sessions
            ));
        }
        if (refusal.have_n, refusal.have_sessions) != (metric.n, metric.n_sessions) {
            fail(format!(
                "{claim:?} withheld with another n than the metric's"
            ));
        }
        if refusal.have_n >= refusal.need_n && refusal.have_sessions >= refusal.need_sessions {
            fail(format!("{claim:?} withheld though it clears its floors"));
        }
    }
    if floors_overridden(&metric.name) {
        seen.overridden_floors += 1;
    }

    let absent = |claim, present: bool| !(withheld.contains(&claim) && present);
    let center = metric.mean.is_some() || metric.mean_ci.is_some() || metric.median.is_some();
    let spread = metric.sd.is_some()
        || metric.sd_ci.is_some()
        || metric.minimum.is_some()
        || metric.maximum.is_some();
    let trend = metric.sessions.iter().any(|session| session.mean.is_some());
    for (claim, present) in [
        (BaselineClaim::Center, center),
        (BaselineClaim::Spread, spread),
        (BaselineClaim::Trend, trend),
    ] {
        if !absent(claim, present) {
            fail(format!(
                "{claim:?} is withheld and a statistic it gates is present"
            ));
        }
    }
}

fn floors_overridden(metric: &str) -> bool {
    BaselineClaim::ALL
        .iter()
        .any(|&claim| minimum_n(metric, claim) != DEFAULT_MINIMUM_N[claim as usize].1)
}

#[test]
fn every_baseline_python_recorded_reads_writes_back_and_holds_to_the_floors() {
    let mut seen = Seen::default();
    let mut failures = Vec::new();

    for path in vector_files("career") {
        let vector = read_vector(&path);
        let id = vector["id"].as_str().expect("a vector id");
        let expected = &vector["expected"];

        if let Some(baseline) = round_trip::<PersonalBaseline>(
            &format!("{id} baseline"),
            &expected["baseline"],
            &mut failures,
        ) {
            seen.baselines += 1;
            compare(
                &format!("{id} properties.baseline"),
                &json!({
                    "claims_ready": baseline.claims_ready(),
                    "nothing_sayable": baseline.nothing_sayable(),
                }),
                &expected["properties"]["baseline"],
                &mut failures,
            );
            for (name, metric) in &baseline.metrics {
                seen.metric_baselines += 1;
                let at = format!("{id} baseline.metrics[{name}]");
                if *name != metric.name {
                    failures.push(format!("{at}: filed under another name"));
                }
                claim_rules(&at, metric, &mut seen, &mut failures);
            }
        }

        // `ClubProfile` is P7's, but the per-club baselines inside it are this phase's shape, built
        // by the same guard over a club-narrowed corpus, so they are asked the same questions.
        let clubs = expected["bag_profile"]["clubs"].as_array();
        for club in clubs.into_iter().flatten() {
            let club_id = club["club"].as_str().expect("a club id");
            let metrics = club["metrics"].as_object().expect("a club's metrics");
            for (name, given) in metrics {
                seen.club_metric_baselines += 1;
                let at = format!("{id} bag_profile[{club_id}].metrics[{name}]");
                if let Some(metric) = round_trip::<MetricBaseline>(&at, given, &mut failures) {
                    claim_rules(&at, &metric, &mut seen, &mut failures);
                }
            }
        }
    }

    for (what, count) in [
        ("baselines", seen.baselines),
        ("metric baselines", seen.metric_baselines),
        ("per-club metric baselines", seen.club_metric_baselines),
        ("ready claims", seen.ready),
        ("withheld claims", seen.withheld),
        ("claims withheld on sessions", seen.withheld_on_sessions),
        ("metrics with overridden floors", seen.overridden_floors),
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

/// The two families age on [`CAREER_VERSION`] together (the M36 plan's decision 8), so a bump fails
/// here until `golf-core rerecord` has re-recorded both, as `ANALYSIS_VERSION`'s pin does for the
/// engine family.
#[test]
fn every_vector_in_both_families_is_at_this_career_version() {
    let mut stale = Vec::new();
    for family in ["storage", "career"] {
        for path in vector_files(family) {
            let vector = read_vector(&path);
            if vector["career_version"] != json!(CAREER_VERSION) {
                stale.push(format!(
                    "{} at {}",
                    vector["id"].as_str().unwrap_or("?"),
                    vector["career_version"]
                ));
            }
        }
    }
    assert!(
        stale.is_empty(),
        "CAREER_VERSION is {CAREER_VERSION}; re-record these with a career-v{CAREER_VERSION}.json \
         declaration:\n  {}",
        stale.join("\n  ")
    );
}
