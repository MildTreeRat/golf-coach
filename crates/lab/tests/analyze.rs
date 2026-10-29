//! `golf_lab::analyze::analyze_swing_dir` on scratch bundles: the steps after the engine, in
//! `api/pipeline.py`'s order, the two artifacts it writes and what frozen Python's readers need of
//! them, coaching folded in from a coach that answers in the test, and the C6 pin. [M29 P26]
//!
//! Every case runs over `tests/bundle/mod.rs`' scratch bundle, served entirely from caches, so no
//! case poses, decodes or reads a photo, and no case spawns Python or spends a coaching call: the
//! coach is [`Answering`]. The real entry's parse is `golf_lab::coach`'s unit tests, and its one live
//! run is the M29 plan's P26 finding. `golf_lab::report` is held to the frozen script's text there
//! too, on stored bundles; here, the lines a scratch bundle reaches.

mod bundle;

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use bundle::{fixture, read_json, sidecar, NoReads, Spec, ViewSpec, SESSION, SWING};
use contracts::feedback::CoachingProvenance;
use contracts::swing::{SwingBundleResult, ANALYSIS_VERSION};
use contracts::unscored::{UnscoredCheckpoint, UnscoredReason, UNSCORED_REASONS};
use contracts::Timestamp;
use golf_lab::analyze::{
    analyze_swing_dir, resolve_swing_dir, wire_name, write_analysis, AnalyzeError, Outcome,
    UNREADABLE_BY_FROZEN_PYTHON,
};
use golf_lab::coach::{Coach, CoachAnswer, CoachRequest};
use golf_lab::ffmpeg::FfmpegMissing;
use golf_lab::import::spelled;
use golf_lab::pipeline::{assemble, Assembly, PipelineOptions, Tools};
use golf_lab::report::{print_report, print_swings};
use serde_json::Value;
use storage::manifest::{load_manifest, manifest_path};
use storage::state::{input_hashes, load_state, save_state, AnalysisState, Status, ANALYSIS_NAME};

/// When the fake clock starts, and how far each reading moves it.
const T0_MICROS: i64 = 1_792_000_000_000_000;
const TICK_MICROS: i64 = 1_500_000;

const PARAGRAPH: &str = "Tempo first: take it back longer, and the head-behind-the-hips \
                         separation that is just under the tour range tends to follow on its own.";

/// A clock that moves a tick and a half-second per reading, so the state's stamps and duration are
/// known.
fn clock() -> impl FnMut() -> Timestamp {
    let mut readings = 0;
    move || {
        readings += 1;
        Timestamp::from_utc_micros(T0_MICROS + (readings - 1) * TICK_MICROS)
    }
}

/// A coach that answers what the case tells it to, and keeps what it was asked and what the file it
/// would read held at that moment.
struct Answering {
    answer: Option<CoachAnswer>,
    asked: Vec<(PathBuf, PathBuf, String, String)>,
    saw: Vec<Value>,
}

impl Answering {
    fn with(answer: CoachAnswer) -> Self {
        Answering {
            answer: Some(answer),
            asked: Vec::new(),
            saw: Vec::new(),
        }
    }

    /// A coach that fails the case if asked.
    fn never() -> Self {
        Answering {
            answer: None,
            asked: Vec::new(),
            saw: Vec::new(),
        }
    }
}

impl Coach for Answering {
    fn coach(&mut self, request: &CoachRequest<'_>) -> CoachAnswer {
        let answer = self
            .answer
            .clone()
            .expect("a case with coaching off asked the coach");
        self.asked.push((
            request.sessions_dir.to_path_buf(),
            request.shots_dir.to_path_buf(),
            request.session_id.to_string(),
            request.swing_id.to_string(),
        ));
        // What `golf-mcp`'s `get_swing` would read at this moment.
        let file = request
            .sessions_dir
            .join(request.session_id)
            .join(request.swing_id)
            .join(ANALYSIS_NAME);
        self.saw
            .push(serde_json::from_slice(&fs::read(file).expect("written before asking")).unwrap());
        answer
    }
}

fn written() -> CoachAnswer {
    CoachAnswer::Written {
        text: PARAGRAPH.to_string(),
        provenance: CoachingProvenance {
            model: "claude-test".to_string(),
            generated_at: Timestamp::parse("2026-10-25T09:00:00Z").unwrap(),
            input_digest: Some("ab".repeat(32)),
        },
    }
}

/// One run, its log, and the sidecar's word that nothing was posed.
fn analyze(
    swing_dir: &Path,
    options: &PipelineOptions,
    coach: &mut dyn Coach,
) -> (Result<Outcome, AnalyzeError>, Vec<String>) {
    let missing = FfmpegMissing { tried: Vec::new() };
    let mut sidecar = sidecar();
    let mut reader = NoReads;
    let mut log = Vec::new();
    let outcome = analyze_swing_dir(
        swing_dir,
        options,
        &mut Tools {
            ffmpeg: Err(&missing),
            sidecar: &mut sidecar,
            reader: &mut reader,
        },
        coach,
        &mut clock(),
        &mut |line| log.push(line.to_string()),
    );
    assert!(!sidecar.started(), "a case reached the pose sidecar");
    (outcome, log)
}

/// What the engine says on its own over this bundle, and the notes the assembly adds after it:
/// the two halves of the stored order.
fn engine_and_pipeline_notes(
    swing_dir: &Path,
    options: &PipelineOptions,
) -> (Vec<String>, Vec<String>) {
    let missing = FfmpegMissing { tried: Vec::new() };
    let mut sidecar = sidecar();
    let mut reader = NoReads;
    let Assembly::Ready(assembled) = assemble(
        swing_dir,
        options,
        &mut Tools {
            ffmpeg: Err(&missing),
            sidecar: &mut sidecar,
            reader: &mut reader,
        },
        &mut |_| {},
    )
    .unwrap() else {
        panic!("the bundle did not assemble");
    };
    let engine = golf_core::run_bundle(&assembled.input).notes;
    (engine, assembled.notes)
}

fn read(path: &Path) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

fn names_in(dir: &Path) -> BTreeSet<String> {
    fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect()
}

/// A state the frozen worker left: its `queued_at`, and a render this run does not replace.
fn previous_state(swing_dir: &Path, video: &str) -> AnalysisState {
    let manifest = load_manifest(&manifest_path(swing_dir)).unwrap();
    let state = AnalysisState {
        inputs: input_hashes(&manifest),
        queued_at: Some(Timestamp::parse("2026-08-23T04:49:54.758464Z").unwrap()),
        video: Some(video.to_string()),
        video_codec: Some("avc1".to_string()),
        score: Some(12.5),
        ..AnalysisState::new(Status::Done)
    };
    save_state(&state, swing_dir).unwrap();
    state
}

/// The whole run on a complete bundle: Python's order after the engine, the artifact the coach reads
/// before it answers and the one written after, and the state beside it.
#[test]
fn a_bundle_is_analysed_coached_written_and_recorded_in_pythons_order() {
    let bundle = fixture(Spec::default());
    let dir = &bundle.swing_dir;
    previous_state(dir, "aligned.mp4");
    fs::write(dir.join("aligned.mp4"), b"").unwrap();
    let (engine_notes, pipeline_notes) = engine_and_pipeline_notes(dir, &bundle.options);
    let mut coach = Answering::with(written());

    let (outcome, log) = analyze(dir, &bundle.options, &mut coach);
    let outcome = outcome.unwrap();
    let analysis_path = dir.join(ANALYSIS_NAME);
    assert_eq!(
        outcome.analysis_path.as_deref(),
        Some(analysis_path.as_path())
    );
    assert_eq!((outcome.flagged, outcome.error.as_deref()), (false, None));
    let result = outcome.result.as_ref().expect("a result");

    // After the assembly's lines: coaching, then the write, and nothing between.
    let tail = &log[log
        .iter()
        .position(|l| l == "\nCoaching:")
        .expect("coached")..];
    assert_eq!(
        tail,
        [
            "\nCoaching:".to_string(),
            format!(
                "  claude-test wrote {} characters",
                PARAGRAPH.chars().count()
            ),
            format!("\nWrote {}", spelled(&analysis_path).display()),
        ]
    );

    // The engine's notes, then the pipeline's: the order every stored artifact holds.
    let mut notes = engine_notes.clone();
    notes.extend(pipeline_notes);
    assert!(!engine_notes.is_empty() && notes.len() > engine_notes.len());
    assert_eq!(result.notes, notes);

    // Asked once, where `golf-mcp` finds the swing, with every note in and no paragraph yet.
    let sessions = std::path::absolute(dir.parent().unwrap().parent().unwrap()).unwrap();
    let shots = std::path::absolute(&bundle.options.shot.shots_dir).unwrap();
    assert_eq!(
        coach.asked,
        [(sessions, shots, SESSION.to_string(), SWING.to_string())]
    );
    assert_eq!(coach.saw[0]["notes"], serde_json::json!(notes));
    assert_eq!(coach.saw[0]["feedback"]["coaching_text"], Value::Null);

    // The artifact: the result as `serialize` drops it, with the paragraph folded in.
    let stored = read(&analysis_path);
    assert_eq!(stored, golf_core::serialize(result));
    assert!(stored["swing"].get("keypoints").is_none());
    assert!(stored["swing"].get("detections").is_none());
    assert_eq!(stored["analysis_version"], ANALYSIS_VERSION);
    assert_eq!(stored["feedback"]["coaching_text"], PARAGRAPH);
    assert_eq!(stored["feedback"]["coaching"]["model"], "claude-test");
    let typed: SwingBundleResult = serde_json::from_value(stored).unwrap();
    let mut dropped = result.clone();
    dropped.swing.keypoints.clear();
    dropped.swing.detections.clear();
    assert_eq!(
        typed, dropped,
        "the artifact reads back as the result, less the two streams"
    );

    // The state: this run's stamps and summary, the worker's `queued_at` and the standing render.
    let state = load_state(dir).unwrap();
    let manifest = load_manifest(&manifest_path(dir)).unwrap();
    assert_eq!(state.status, Status::Done);
    assert!(state.matches(&manifest));
    assert_eq!(
        state.queued_at,
        Some(Timestamp::parse("2026-08-23T04:49:54.758464Z").unwrap())
    );
    assert_eq!(
        state.started_at,
        Some(Timestamp::from_utc_micros(T0_MICROS))
    );
    assert_eq!(
        state.completed_at,
        Some(Timestamp::from_utc_micros(T0_MICROS + TICK_MICROS))
    );
    assert_eq!(state.duration_seconds, Some(1.5));
    assert_eq!(
        (state.video.as_deref(), state.video_codec.as_deref()),
        (Some("aligned.mp4"), Some("avc1"))
    );
    assert_eq!(state.score, Some(result.swing.overall_score));
    assert_eq!(
        state.headline,
        result.feedback.as_ref().unwrap().headline.clone()
    );
    assert_eq!((state.error, state.partial), (None, false));
    assert!(state.missing_roles.is_empty());
    assert!(
        !names_in(dir).iter().any(|name| name.ends_with(".tmp")),
        "no write was left half done"
    );
}

/// C5's switch off: the coach is never asked, nothing is logged for it, and no paragraph or note.
#[test]
fn with_coaching_off_the_coach_is_never_asked() {
    let bundle = fixture(Spec::default());
    let options = PipelineOptions {
        coaching: false,
        ..bundle.options.clone()
    };
    let (outcome, log) = analyze(&bundle.swing_dir, &options, &mut Answering::never());
    let result = outcome.unwrap().result.unwrap();
    assert!(!log.iter().any(|line| line == "\nCoaching:"), "{log:?}");
    assert_eq!(
        log.last().unwrap(),
        &format!(
            "\nWrote {}",
            spelled(&bundle.swing_dir.join(ANALYSIS_NAME)).display()
        )
    );
    assert_eq!(result.feedback.as_ref().unwrap().coaching_text, None);
    let (engine, pipeline) = engine_and_pipeline_notes(&bundle.swing_dir, &options);
    assert_eq!(result.notes, [engine, pipeline].concat());
}

/// A refusal is the last note and the last coaching line, as Python appends `coaching.note`.
#[test]
fn a_coaching_refusal_is_the_last_note() {
    let bundle = fixture(Spec::default());
    let note = "no written coaching: no Anthropic API key is configured, so the coaching call was \
                skipped (set GOLF_ANTHROPIC_API_KEY). The scores and ranked tips above are \
                unaffected.";
    let mut coach = Answering::with(CoachAnswer::Note(note.to_string()));
    let (outcome, log) = analyze(&bundle.swing_dir, &bundle.options, &mut coach);
    let result = outcome.unwrap().result.unwrap();
    assert_eq!(result.notes.last().map(String::as_str), Some(note));
    let coaching = log.iter().position(|l| l == "\nCoaching:").unwrap();
    assert_eq!(log[coaching + 1], format!("  {note}"));
    assert_eq!(result.feedback.as_ref().unwrap().coaching, None);
    assert_eq!(
        read(&bundle.swing_dir.join(ANALYSIS_NAME))["notes"],
        serde_json::json!(result.notes)
    );
}

/// The loft repair lands once the engine has reached for the loft and missed it, after the
/// pipeline's notes and logged under its own heading, as `analyze_swing_dir` gates it.
#[test]
fn a_loft_the_engine_missed_is_the_repair_after_the_pipeline_s_notes() {
    let bundle = fixture(Spec {
        club: Some("3w"),
        ..Spec::default()
    });
    let options = PipelineOptions {
        coaching: false,
        ..bundle.options.clone()
    };
    let (outcome, log) = analyze(&bundle.swing_dir, &options, &mut Answering::never());
    let result = outcome.unwrap().result.unwrap();
    assert!(
        result
            .swing
            .unscored
            .iter()
            .any(|entry| entry.reason == UnscoredReason::NoClubLoft),
        "the fixture's shot prints no spin, so its flight needs the loft: {:?}",
        result.swing.unscored
    );
    let remedy = "club 3w is not in this golfer's bag, so its loft is unknown and the ball flight \
                  could not be simulated — add the club on the bag page";
    let (engine, pipeline) = engine_and_pipeline_notes(&bundle.swing_dir, &options);
    assert_eq!(
        result.notes,
        [engine, pipeline, vec![remedy.to_string()]].concat()
    );
    assert!(
        log.contains(&format!("\nBall flight:\n  {remedy}")),
        "{log:?}"
    );
}

/// No face-on view: Python's failed outcome, recorded as a failed run, with no artifact written and
/// the coach never asked. A render whose file is gone is not carried forward.
#[test]
fn no_face_on_view_records_a_failed_run_and_writes_no_analysis() {
    let bundle = fixture(Spec {
        face_on: Some(ViewSpec {
            keypoints: false,
            strikes: None,
        }),
        ..Spec::default()
    });
    let dir = &bundle.swing_dir;
    previous_state(dir, "aligned.mp4");
    let (outcome, _) = analyze(dir, &bundle.options, &mut Answering::never());
    let outcome = outcome.unwrap();
    let error = "no usable face-on view — that is the view every checkpoint is measured from, so \
                 there is nothing to score";
    assert_eq!(outcome.error.as_deref(), Some(error));
    assert!(outcome.result.is_none() && outcome.analysis_path.is_none());
    assert!(!dir.join(ANALYSIS_NAME).exists());
    let state = load_state(dir).unwrap();
    assert_eq!(state.status, Status::Failed);
    assert_eq!(state.error.as_deref(), Some(error));
    assert_eq!((state.score, state.headline), (None, None));
    assert_eq!(
        (state.video, state.video_codec),
        (None, None),
        "aligned.mp4 is not on disk, so it is not re-advertised"
    );
    assert!(state.queued_at.is_some());
}

/// No manifest: nothing to key a state on, so nothing is written at all.
#[test]
fn a_directory_with_no_manifest_writes_nothing() {
    let bundle = fixture(Spec::default());
    let empty = bundle.swing_dir.parent().unwrap().join("8");
    fs::create_dir_all(&empty).unwrap();
    let (outcome, log) = analyze(&empty, &bundle.options, &mut Answering::never());
    let outcome = outcome.unwrap();
    assert!(outcome
        .error
        .unwrap()
        .ends_with(" has no readable manifest"));
    assert!(log.is_empty());
    assert!(names_in(&empty).is_empty());
}

/// A role that never arrived flags the run and marks the state partial.
#[test]
fn a_missing_role_flags_the_run_and_the_state_says_partial() {
    let bundle = fixture(Spec {
        down_the_line: None,
        ..Spec::default()
    });
    let options = PipelineOptions {
        coaching: false,
        ..bundle.options.clone()
    };
    let (outcome, _) = analyze(&bundle.swing_dir, &options, &mut Answering::never());
    let outcome = outcome.unwrap();
    assert!(outcome.flagged);
    let state = load_state(&bundle.swing_dir).unwrap();
    assert!(state.partial);
    assert_eq!(state.missing_roles, ["down_the_line"]);
    assert_eq!(state.status, Status::Done);
}

/// **C6's pin.** The reasons the writer refuses are exactly the ones frozen Python's closed
/// `UnscoredReason` lacks, read off the committed `swing_bundle_result` schema, so a reason added
/// to Rust and not to the refusal fails here rather than in a Python reader.
#[test]
fn the_reasons_the_writer_refuses_are_the_ones_frozen_python_lacks() {
    let rust: BTreeSet<String> = UNSCORED_REASONS
        .iter()
        .map(|(reason, _)| wire_name(*reason))
        .collect();
    let python: BTreeSet<String> = read_json("spec/schemas/swing_bundle_result.schema.json")
        ["$defs"]["UnscoredReason"]["enum"]
        .as_array()
        .expect("the schema names frozen Python's reasons")
        .iter()
        .map(|name| name.as_str().unwrap().to_string())
        .collect();
    let refused: BTreeSet<String> = UNREADABLE_BY_FROZEN_PYTHON
        .into_iter()
        .map(wire_name)
        .collect();
    assert!(
        python.is_subset(&rust),
        "a Python reason Rust does not name"
    );
    assert_eq!(
        rust.difference(&python).cloned().collect::<BTreeSet<_>>(),
        refused
    );
}

/// The refusal itself: a result naming a photo-side reason writes nothing.
#[test]
fn a_result_naming_a_photo_side_reason_is_not_written() {
    let bundle = fixture(Spec::default());
    let options = PipelineOptions {
        coaching: false,
        ..bundle.options.clone()
    };
    let (outcome, _) = analyze(&bundle.swing_dir, &options, &mut Answering::never());
    let mut result = outcome.unwrap().result.unwrap();
    let elsewhere = tempfile::tempdir().unwrap();
    for reason in UNREADABLE_BY_FROZEN_PYTHON {
        result.swing.unscored.push(UnscoredCheckpoint {
            name: "ball_speed".to_string(),
            reason,
            detail: String::new(),
        });
        let error = write_analysis(&result, elsewhere.path()).unwrap_err();
        assert!(
            matches!(error, AnalyzeError::UnreadableReason { reason: refused, .. } if refused == UNREADABLE_BY_FROZEN_PYTHON[0]),
            "{error}"
        );
        assert!(error.to_string().contains("printed_blank"), "{error}");
        assert!(names_in(elsewhere.path()).is_empty());
    }
}

/// `resolve_swing_dir`: a directory holding a manifest, or `SESSION/SWING` in either separator.
#[test]
fn a_swing_is_named_by_its_directory_or_its_pair() {
    let bundle = fixture(Spec::default());
    let sessions = bundle.swing_dir.parent().unwrap().parent().unwrap();
    let dir = bundle.swing_dir.to_string_lossy().into_owned();
    assert_eq!(
        resolve_swing_dir(&dir, Path::new("nowhere")),
        Some(bundle.swing_dir.clone())
    );
    for pair in [format!("{SESSION}/{SWING}"), format!("{SESSION}\\{SWING}")] {
        assert_eq!(
            resolve_swing_dir(&pair, sessions),
            Some(sessions.join(SESSION).join(SWING))
        );
    }
    assert_eq!(resolve_swing_dir(&format!("{SESSION}/9"), sessions), None);
    assert_eq!(resolve_swing_dir(SESSION, sessions), None);
    assert_eq!(
        resolve_swing_dir(&format!("a/{SESSION}/{SWING}"), sessions),
        None
    );
}

/// The report's lines a scratch bundle reaches: the unscored names where the script's `join`
/// raises, the shot line, and the paragraph under its attribution, wrapped at 88.
#[test]
fn the_report_names_what_was_not_measured_and_wraps_the_paragraph() {
    let bundle = fixture(Spec::default());
    let mut coach = Answering::with(written());
    let (outcome, _) = analyze(&bundle.swing_dir, &bundle.options, &mut coach);
    let result = outcome.unwrap().result.unwrap();
    let mut out = Vec::new();
    print_report(&result, &mut out).unwrap();
    let text = String::from_utf8(out).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines[1], format!("Swing {SWING}  (session {SESSION})"));
    let names: Vec<&str> = result
        .swing
        .unscored
        .iter()
        .map(|u| u.name.as_str())
        .collect();
    assert!(!names.is_empty(), "a flat clip measures little");
    assert!(
        lines.contains(&format!("  not measured: {}", names.join(", ")).as_str()),
        "{text}"
    );
    assert!(lines
        .iter()
        .any(|line| line.starts_with("  2026-08-23-2   source=screen  conf=")));
    let heading = lines
        .iter()
        .position(|line| *line == "Coach (claude-test, written from the numbers above):")
        .expect("the paragraph is attributed");
    let paragraph: Vec<&str> = lines[heading + 1..].to_vec();
    assert_eq!(
        paragraph,
        golf_core::textwrap::wrap(PARAGRAPH, 88, "", "")
            .iter()
            .map(|line| format!("  {line}"))
            .collect::<Vec<_>>()
    );
    assert!(paragraph.len() > 1 && paragraph.iter().all(|line| line.len() <= 90));
}

/// `--list-swings` on a flat clip: the view's frame count, and no descent.
#[test]
fn a_flat_clip_lists_no_descent() {
    let keypoints = bundle::flat_keypoints(bundle::FACE_SHA);
    let mut out = Vec::new();
    print_swings("face-on", "--window-face-on", &keypoints, &mut out).unwrap();
    assert_eq!(
        String::from_utf8(out).unwrap(),
        format!(
            "\nface-on: {} frames, 0 candidate descent(s)\n  (none - no detectable descent of the \
             hands)\n",
            bundle::FRAMES
        )
    );
}
