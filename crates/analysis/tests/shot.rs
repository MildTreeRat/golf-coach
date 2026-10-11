//! `analyze_shot`, a shot analysed with no clip beside it, against the committed corpus. [M35 P2]
//!
//! **No Python twin and no vector family of its own yet.** The shot family
//! (`spec/vectors/shot/`) arrives in M35 P4 and P5, gated through `crates/core`'s `run_shot` seam so
//! the screen's refusals are exercised end to end. What this file holds is the property that family
//! is built on, the M35 plan's call 2 pinned as **the Decision-12 pin**: over every committed engine
//! vector carrying a shot, `analyze_shot` produces exactly the shot and flight rows that vector
//! recorded for its whole swing, positionally, and exactly the flight refusals at the tail of its
//! `unscored`. So adding or removing video can never move a shot number. Those answers are the
//! oracle's (Python-recorded, re-recorded by Rust where `provenance.rerecords` says so), which
//! makes this a check against committed numbers rather than against `analyze_swing` run beside it.
//!
//! The rest are the function's own rules, on shots built here or taken from the corpus: a reading
//! the screen did not print reaches nothing, a shown-only one stays on the shot and reaches nothing,
//! an undeclared device is refused, and the screen's refusals come first in the order given.
//!
//! # The rules
//!
//! `docs/CONFORMANCE.md` §3's: floats within `RTOL`, strings exactly, lists positionally. The
//! readers are duplicated from `tests/flight.rs` rather than shared, for the reason that file gives.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use analysis::engine::{analyze_shot, ShotRequest};
use contracts::checkpoints::checkpoint_names;
use contracts::golfer::Handedness;
use contracts::intent::{PracticeGoal, PracticeMode};
use contracts::shot::ShotData;
use contracts::shot_result::ShotResult;
use contracts::swing::{Measurement, ANALYSIS_VERSION};
use contracts::unscored::{UnscoredCheckpoint, UnscoredReason};
use flate2::read::GzDecoder;
use serde::Deserialize;
use serde_json::{json, Value};

/// `docs/CONFORMANCE.md` §3's float rule, and the same constants the sibling gates use.
const RTOL: f64 = 1e-9;
const ATOL: f64 = 1e-12;

/// The two `Measurement.source` families a shot produces: `launch_monitor:{device}` for the shot's
/// rows, and the flight model's for the flight's (`analysis::flight_measure::FLIGHT_SOURCE`).
const SHOT_SOURCE_PREFIX: &str = "launch_monitor:";
const FLIGHT_SOURCE: &str = analysis::flight_measure::FLIGHT_SOURCE;

#[derive(Deserialize)]
struct EngineVector {
    id: String,
    input: EngineInput,
    expected: Expected,
}

#[derive(Deserialize)]
struct EngineInput {
    swing_id: String,
    session_id: String,
    #[serde(default)]
    shot: Option<ShotData>,
    #[serde(default)]
    intent: Option<PracticeGoal>,
    #[serde(default)]
    loft_deg: Option<f64>,
    #[serde(default)]
    handedness: Option<Handedness>,
}

#[derive(Deserialize)]
struct Expected {
    swing: ExpectedSwing,
    notes: Vec<String>,
}

#[derive(Deserialize)]
struct ExpectedSwing {
    measurements: Vec<Measurement>,
    unscored: Vec<UnscoredCheckpoint>,
}

fn spec_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crates/analysis has a grandparent")
        .join("spec")
        .join("vectors")
}

fn read_json(path: &Path) -> String {
    let bytes = fs::read(path).unwrap_or_else(|e| panic!("read {path:?}: {e}"));
    if path.extension().is_some_and(|e| e == "gz") {
        let mut text = String::new();
        GzDecoder::new(&bytes[..])
            .read_to_string(&mut text)
            .unwrap_or_else(|e| panic!("gunzip {path:?}: {e}"));
        text
    } else {
        String::from_utf8(bytes).unwrap_or_else(|e| panic!("utf-8 {path:?}: {e}"))
    }
}

/// Every committed engine vector in one half of `spec/vectors/`, sorted so a failure names a
/// stable first offender.
fn engine_vectors(half: &str) -> Vec<EngineVector> {
    let root = spec_dir().join(half);
    let mut paths: Vec<PathBuf> = fs::read_dir(&root)
        .unwrap_or_else(|e| panic!("list {root:?}: {e}"))
        .map(|entry| entry.expect("a readable directory entry").path())
        .collect();
    paths.sort();
    assert!(!paths.is_empty(), "no engine vectors under {root:?}");
    paths
        .into_iter()
        .map(|path| {
            serde_json::from_str(&read_json(&path))
                .unwrap_or_else(|e| panic!("parse {path:?}: {e}"))
        })
        .collect()
}

/// The vector's own request, as the shell would build it for the same shot with no clip.
fn request_for<'a>(vector: &'a EngineVector, shot: &'a ShotData) -> ShotRequest<'a> {
    ShotRequest {
        swing_id: &vector.input.swing_id,
        session_id: &vector.input.session_id,
        shot,
        intent: vector.input.intent.as_ref(),
        handedness: vector.input.handedness,
        loft_deg: vector.input.loft_deg,
        tiles: &[],
        photo_sha256: shot
            .provenance
            .as_ref()
            .and_then(|provenance| provenance.image_sha256.as_deref()),
    }
}

fn float_differs(got: f64, want: f64) -> bool {
    !matches!(
        (got - want).abs().partial_cmp(&(ATOL + RTOL * want.abs())),
        Some(std::cmp::Ordering::Less | std::cmp::Ordering::Equal)
    )
}

/// The rows positionally: every string exactly, the value within `RTOL`.
fn compare_rows(id: &str, got: &[Measurement], want: &[&Measurement], out: &mut Vec<String>) {
    if got.len() != want.len() {
        out.push(format!(
            "{id}: {} shot and flight rows, the vector recorded {}: {:?} != {:?}",
            got.len(),
            want.len(),
            got.iter().map(|row| &row.name).collect::<Vec<_>>(),
            want.iter().map(|row| &row.name).collect::<Vec<_>>(),
        ));
        return;
    }
    for (at, (got, want)) in got.iter().zip(want).enumerate() {
        let strings = |row: &Measurement| {
            (
                row.name.clone(),
                row.unit.clone(),
                row.source.clone(),
                row.detail.clone(),
            )
        };
        if strings(got) != strings(want) {
            out.push(format!(
                "{id}: row {at}: {:?} != {:?}",
                strings(got),
                strings(want)
            ));
        }
        if float_differs(got.value, want.value) {
            out.push(format!(
                "{id}: row {at} {}: {} != {}",
                want.name, got.value, want.value
            ));
        }
    }
}

/// **The Decision-12 pin** (the M35 plan's call 2): the shot alone produces the shot half of the
/// swing that carried it, on every committed vector with a shot.
///
/// - The **rows** are the vector's `launch_monitor:` and `model:flight_v1` rows, in recorded order.
///   Nothing between them is skipped, since `measurements` writes the shot group and then the
///   flight group last.
/// - The **refusals** are the vector's `unscored` tail after its last checkpoint entry, which is
///   where `analyze_swing` appends the flight's. Taken by registry name rather than by counting, so
///   a vector whose flight refused nothing contributes an empty tail and is still compared.
/// - The **notes** are the vector's needs-review note, when it has one, and nothing else.
#[test]
fn the_shot_alone_produces_every_committed_shot_and_flight_row() {
    let registry = checkpoint_names();
    let mut differences: Vec<String> = Vec::new();
    let mut compared = 0;

    let corpus = engine_vectors("corpus");
    let corpus_count = corpus.len();
    for vector in corpus.iter().chain(&engine_vectors("synthetic")) {
        let id = &vector.id;
        let Some(shot) = vector.input.shot.as_ref() else {
            assert!(
                !id.starts_with("corpus/"),
                "{id}: a corpus vector with no shot"
            );
            continue;
        };
        compared += 1;
        let result = analyze_shot(&request_for(vector, shot))
            .unwrap_or_else(|e| panic!("{id}: refused: {e}"));

        let recorded: Vec<&Measurement> = vector
            .expected
            .swing
            .measurements
            .iter()
            .filter(|row| row.source.starts_with(SHOT_SOURCE_PREFIX) || row.source == FLIGHT_SOURCE)
            .collect();
        compare_rows(id, &result.measurements, &recorded, &mut differences);

        let unscored = &vector.expected.swing.unscored;
        let tail_at = unscored
            .iter()
            .rposition(|entry| registry.contains(&entry.name.as_str()))
            .map_or(0, |last| last + 1);
        if result.unscored != unscored[tail_at..] {
            differences.push(format!(
                "{id}: unscored {:?} != the vector's flight tail {:?}",
                result.unscored,
                &unscored[tail_at..]
            ));
        }

        let reviewed: Vec<&String> = vector
            .expected
            .notes
            .iter()
            .filter(|note| note.starts_with("shot data needs review"))
            .collect();
        if result.notes.iter().collect::<Vec<_>>() != reviewed {
            differences.push(format!("{id}: notes {:?} != {reviewed:?}", result.notes));
        }
    }

    assert!(
        differences.is_empty(),
        "{} difference(s) across {compared} committed shots:\n{}",
        differences.len(),
        differences.join("\n")
    );
    assert_eq!(
        compared, corpus_count,
        "every corpus vector carries a shot and no synthetic one does"
    );
    println!("shot: {compared} committed shots produce their swing's shot and flight rows");
}

/// One committed shot, by vector id: `2026-08-23-4` holds both shown-only fields, a loft, and a
/// flight whose axis was refused, so most of what follows has something to see on it.
fn corpus_vector(id: &str) -> EngineVector {
    engine_vectors("corpus")
        .into_iter()
        .find(|vector| vector.id == format!("corpus/{id}"))
        .unwrap_or_else(|| panic!("no corpus vector {id}"))
}

fn shot_of(vector: &EngineVector) -> ShotData {
    vector
        .input
        .shot
        .clone()
        .expect("a corpus vector carries a shot")
}

/// A shot whose provenance is edited through its JSON, so the shape's validator runs on the way
/// back in.
fn with_provenance(shot: &ShotData, edit: impl FnOnce(&mut Value)) -> ShotData {
    let mut written = serde_json::to_value(shot).expect("serialize");
    edit(&mut written["provenance"]);
    serde_json::from_value(written).expect("still a valid shot")
}

/// **A shown-only field holding a value stays on `ShotResult.shot` and reaches no measurement.**
/// The stored shot keeps its club speed and smash, because the phone displays them, and giving them
/// values no strike produces moves nothing measured, flown or refused.
#[test]
fn a_shown_only_field_is_kept_on_the_shot_and_reaches_no_number() {
    let vector = corpus_vector("2026-08-23-4");
    let shot = shot_of(&vector);
    assert!(shot.club_head_speed.is_some() && shot.smash_factor.is_some());

    let result = analyze_shot(&request_for(&vector, &shot)).unwrap();
    assert_eq!(
        result.shot, shot,
        "the shot is stored as printed, not as the view"
    );

    let mut wild = shot.clone();
    wild.club_head_speed = Some(250.0);
    wild.smash_factor = Some(2.4);
    let moved = analyze_shot(&request_for(&vector, &wild)).unwrap();
    assert_eq!(moved.measurements, result.measurements);
    assert_eq!(moved.unscored, result.unscored);
    assert_eq!(moved.shot.club_head_speed, Some(250.0));
}

/// **A reading the screen did not print reaches nothing**, even when the shot holds a value for it.
/// This is the view having teeth: a stamped shot whose parse did not locate the carry tile measures
/// no carry and cannot solve a spin from one, where the same values on an unstamped shot do both.
#[test]
fn a_reading_the_screen_did_not_print_reaches_no_number() {
    let vector = corpus_vector("2026-08-23-4");
    let shot = shot_of(&vector);
    let held = [
        "total_distance",
        "ball_speed",
        "launch_angle",
        "club_path",
        "club_face_angle",
        "impact_position",
        "shot_type",
        "launch_direction",
    ];
    let stamped = with_provenance(&shot, |provenance| {
        provenance["parser_version"] = json!(1);
        provenance["fields_present"] = json!(held);
    });
    assert!(stamped.carry_distance.is_some(), "the value is held");

    let full = analyze_shot(&request_for(&vector, &shot)).unwrap();
    let cut = analyze_shot(&request_for(&vector, &stamped)).unwrap();
    let names = |result: &ShotResult| -> Vec<String> {
        result
            .measurements
            .iter()
            .map(|row| row.name.clone())
            .collect()
    };
    assert!(names(&full).contains(&"carry_distance_yds".to_string()));
    for carried in ["carry_distance_yds", "start_line_offline_yds"] {
        assert!(!names(&cut).contains(&carried.to_string()), "{carried}");
    }
    assert!(
        !cut.measurements
            .iter()
            .any(|row| row.source == FLIGHT_SOURCE),
        "a flight solved from a carry the screen did not print"
    );
    // The unprinted carry is not a refusal of the screen's either: `tiles` is the shell's, and an
    // unprinted field is never blank. What the flight refuses is its own business.
    assert!(cut
        .unscored
        .iter()
        .all(|entry| entry.name.starts_with("flight_")));
}

/// **An undeclared device is refused**, as `capability::printed_on` refuses it: guessing what an
/// R10 prints would either hide a stat the golfer can see or grade one they cannot.
#[test]
fn an_undeclared_device_is_refused() {
    let vector = corpus_vector("2026-08-23-4");
    let r10 = with_provenance(&shot_of(&vector), |provenance| {
        provenance["device"] = json!("r10");
    });
    let refused = analyze_shot(&request_for(&vector, &r10)).unwrap_err();
    assert_eq!(refused.field, "DeviceCapability.device");
    assert!(refused.problem.contains("\"r10\""), "{refused}");
}

/// **The screen's refusals come first in `unscored`, in the order given**, then the flight's. Given
/// here out of `devices.json`'s tile order on purpose (`spin_rate` is declared before
/// `spin_axis`), so a function that sorted them would fail rather than agree by accident: ordering
/// them is `screen::tile_refusals`' job (the M35 plan's call 3), and this function only places them.
#[test]
fn the_screens_refusals_come_first_in_the_order_given() {
    let vector = corpus_vector("2026-08-23-4");
    let shot = shot_of(&vector);
    let tiles = [
        UnscoredCheckpoint {
            name: "spin_axis".to_string(),
            reason: UnscoredReason::Misread,
            detail: "illustrative".to_string(),
        },
        UnscoredCheckpoint {
            name: "spin_rate".to_string(),
            reason: UnscoredReason::PrintedBlank,
            detail: "illustrative".to_string(),
        },
    ];
    let alone = analyze_shot(&request_for(&vector, &shot)).unwrap();
    assert!(
        !alone.unscored.is_empty(),
        "the flight refuses something on this shot"
    );

    let mut request = request_for(&vector, &shot);
    request.tiles = &tiles;
    let result = analyze_shot(&request).unwrap();
    assert_eq!(result.unscored[..tiles.len()], tiles);
    assert_eq!(result.unscored[tiles.len()..], alone.unscored[..]);
    assert_eq!(
        result.measurements, alone.measurements,
        "a refusal moves no number"
    );
}

/// The result says which engine produced it, from which photo, under which intent, and the intent
/// moves no number (the M35 plan's decision 1). It also round-trips as `shot_analysis.json` will.
#[test]
fn the_result_records_its_engine_photo_and_intent_and_the_intent_moves_nothing() {
    let vector = corpus_vector("2026-08-23-4");
    let shot = shot_of(&vector);
    let drill = PracticeGoal {
        mode: PracticeMode::Drill,
        ..PracticeGoal::default()
    };
    let mut request = request_for(&vector, &shot);
    request.intent = Some(&drill);
    request.photo_sha256 = Some("f09d9e34");
    let result = analyze_shot(&request).unwrap();

    assert_eq!(result.analysis_version, ANALYSIS_VERSION);
    assert_eq!(result.photo_sha256.as_deref(), Some("f09d9e34"));
    assert_eq!(result.intent, drill);
    assert_eq!(result.swing_id, vector.input.swing_id);
    assert_eq!(result.session_id, vector.input.session_id);

    request.intent = None;
    let defaulted = analyze_shot(&request).unwrap();
    assert_eq!(
        defaulted.intent,
        PracticeGoal::default(),
        "None is Fundamentals"
    );
    assert_eq!(defaulted.measurements, result.measurements);
    assert_eq!(defaulted.unscored, result.unscored);

    let written = serde_json::to_string(&result).expect("serialize");
    let back: ShotResult = serde_json::from_str(&written).expect("read back");
    assert_eq!(back, result);
}

/// A doubtful parse is named in the sentence a swing's bundle uses for it, and a trusted one says
/// nothing. `engine.rs`'s unit test pins the same sentence on the bundle; the two calls share one
/// function, so the two tests are what hold it to both callers.
#[test]
fn a_shot_that_needs_review_is_named_in_the_notes() {
    let vector = corpus_vector("2026-08-23-4");
    let trusted = shot_of(&vector);
    assert!(analyze_shot(&request_for(&vector, &trusted))
        .unwrap()
        .notes
        .is_empty());

    let doubtful = with_provenance(&trusted, |provenance| {
        provenance["needs_review"] = json!(true);
        provenance["parse_confidence"] = json!(0.425);
    });
    assert_eq!(
        analyze_shot(&request_for(&vector, &doubtful))
            .unwrap()
            .notes,
        [
            "shot data needs review (parse confidence 0.42) — the numbers below were read off a \
          photograph and at least one check on them failed (ADR-014)"
        ]
    );
}
