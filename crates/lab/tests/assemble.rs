//! `golf_lab::pipeline::assemble` on scratch bundles: the branches of `analyze_swing_dir` up to the
//! engine call that the stored bundles do not reach, each with Python's log lines and notes, and
//! `corpus_input`'s cut on an input small enough to work by hand. [M29 P25]
//!
//! Every case runs over `tests/bundle/mod.rs`' scratch bundle, served entirely from caches, so no
//! case here poses, decodes or reads a photo, and each checks the sidecar never started. The
//! selection itself is the selection family's to gate (`crates/core/tests/selection.rs` answers
//! every `_auto_windows` call by value); the clips here are flat, so each view declines, which is
//! the one selection outcome that also writes a note. `tests/inputs.rs` runs the same assembly
//! over the stored bundles.

mod bundle;

use std::path::Path;

use bundle::{
    fixture, flat_keypoints, sidecar, NoReads, Spec, ViewSpec, DTL_SHA, FACE_SHA, FRAMES, SESSION,
    SWING,
};
use contracts::golfer::Handedness;
use contracts::intent::{ClubCategory, PracticeGoal};
use contracts::unscored::{UnscoredCheckpoint, UnscoredReason};
use golf_core::VectorInput;
use golf_lab::ffmpeg::FfmpegMissing;
use golf_lab::pipeline::{
    assemble, corpus_input, Assembled, Assembly, PipelineOptions, Refusal, Tools,
};
use golf_lab::pose::band_estimator_note;
use golf_lab::shot::{Origin, ShotOptions};
use serde_json::{json, Value};
use storage::manifest::{Role, SwingManifest};

/// One assembly, its log, and the sidecar's word that nothing was posed.
fn run(swing_dir: &Path, options: &PipelineOptions) -> (Assembly, Vec<String>) {
    let missing = FfmpegMissing { tried: Vec::new() };
    let mut sidecar = sidecar();
    let mut reader = NoReads;
    let mut log = Vec::new();
    let assembly = assemble(
        swing_dir,
        options,
        &mut Tools {
            ffmpeg: Err(&missing),
            sidecar: &mut sidecar,
            reader: &mut reader,
        },
        &mut |line| log.push(line.to_string()),
    )
    .unwrap();
    assert!(!sidecar.started(), "a case reached the pose sidecar");
    (assembly, log)
}

fn ready(assembly: Assembly) -> Assembled {
    match assembly {
        Assembly::Ready(assembled) => *assembled,
        Assembly::Refused(refusal) => panic!("refused: {refusal:?}"),
    }
}

fn band_note() -> String {
    band_estimator_note("mediapipe:heavy").expect("the bands were cut by another estimator")
}

/// `_narrate_choice`'s decline, for a flat clip read at an fps.
fn declined(label: &str, landmark: &str) -> (String, String) {
    (
        format!(
            "  {label}: could not pick a swing (no plausible downswing, on the {landmark}) — using \
             the whole clip. Run --list-swings and pass a window if this clip holds more than one \
             swing"
        ),
        format!(
            "could not pick a swing in the {label} view (no plausible downswing, reading the \
             {landmark}) — the whole clip was scored, so these numbers describe every motion in it, \
             not one swing"
        ),
    )
}

const NO_STRIKE_DTL: &str = "no ball strike was heard in the down-the-line clip, so its swing was \
                             picked on movement alone — if this was a rehearsal, the numbers \
                             describe a swing that never hit a ball";

#[test]
fn a_directory_with_no_manifest_is_refused_with_nothing_to_record() {
    let dir = tempfile::tempdir().unwrap();
    let options = PipelineOptions::new(ShotOptions::new(dir.path().join("shots")), dir.path());
    let (assembly, log) = run(dir.path(), &options);
    let Assembly::Refused(refusal) = assembly else {
        panic!("assembled a directory with no manifest");
    };
    let Refusal {
        manifest,
        missing,
        error,
    } = *refusal;
    assert_eq!(manifest, None);
    assert!(missing.is_empty());
    assert!(error.ends_with(" has no readable manifest"), "{error}");
    assert!(log.is_empty(), "{log:?}");
}

/// Python records the failed run when the manifest is there, so the refusal carries it.
#[test]
fn a_bundle_whose_face_on_clip_is_gone_is_refused_after_pose() {
    let bundle = fixture(Spec {
        face_on: Some(ViewSpec {
            keypoints: false,
            strikes: None,
        }),
        ..Spec::default()
    });
    let (assembly, log) = run(&bundle.swing_dir, &bundle.options);
    let Assembly::Refused(refusal) = assembly else {
        panic!("assembled a bundle with no face-on view");
    };
    assert_eq!(
        refusal.error,
        "no usable face-on view — that is the view every checkpoint is measured from, so there is \
         nothing to score"
    );
    let manifest: SwingManifest = refusal.manifest.expect("the refusal carries the manifest");
    assert_eq!(manifest.swing_id, SWING);
    assert!(refusal.missing.is_empty());
    assert_eq!(log[0], format!("\nBundle {SESSION}/{SWING}  [complete]"));
    assert_eq!(log[1], "\nPose:");
    assert!(!log.iter().any(|line| line == "\nAudio:"), "{log:?}");
}

/// The whole order on a complete bundle: Python's headings and lines, the notes in encounter order,
/// and every argument the engine is handed.
#[test]
fn a_complete_bundle_assembles_in_pythons_order() {
    let bundle = fixture(Spec::default());
    let (assembly, log) = run(&bundle.swing_dir, &bundle.options);
    let assembled = ready(assembly);
    let (face_line, face_note) = declined("face-on", "lead wrist");
    let (dtl_line, dtl_note) = declined("down-the-line", "trail wrist");
    assert_eq!(
        log,
        [
            format!("\nBundle {SESSION}/{SWING}  [complete]"),
            "\nPose:".to_string(),
            format!("  face_on: {FRAMES} frames from cache"),
            format!("  down_the_line: {FRAMES} frames from cache"),
            "\nAudio:".to_string(),
            "  face_on: 1 strikes from cache".to_string(),
            "  down_the_line: 0 strikes from cache".to_string(),
            "  down_the_line: no ball strike in this clip".to_string(),
            "\nSwing selection:".to_string(),
            face_line,
            dtl_line,
            "\nShot data:".to_string(),
            "  shot_screen: 2026-08-23-2 from the shot store".to_string(),
        ]
    );
    assert_eq!(
        assembled.notes,
        [band_note(), NO_STRIKE_DTL.to_string(), face_note, dtl_note]
    );
    assert!(assembled.missing.is_empty());
    assert_eq!(assembled.shot_origin, Origin::Stored);

    let input = &assembled.input;
    assert_eq!(
        (input.session_id.as_str(), input.swing_id.as_str()),
        (SESSION, SWING)
    );
    assert_eq!(input.face_on.frames.len(), FRAMES);
    assert_eq!(
        input.down_the_line.as_ref().map(|k| k.frames.len()),
        Some(FRAMES)
    );
    // 12 000 samples at 48 kHz is a quarter of a second, frame 15 at 60 fps.
    assert_eq!(input.face_on_strikes, Some(vec![15]));
    assert_eq!(input.down_the_line_strikes, Some(Vec::new()));
    assert_eq!(
        (input.face_on_window, input.down_the_line_window),
        (None, None)
    );
    assert_eq!(input.handedness, Some(Handedness::Right));
    assert_eq!(
        input.loft_deg,
        Some(34.0),
        "the storage family's bag lofts the 7i"
    );
    assert_eq!(assembled.loft_remedy, None);
    assert_eq!(input.intent, Some(PracticeGoal::default()));
    assert_eq!(
        input.shot.as_ref().map(|shot| shot.shot_id.as_str()),
        Some("2026-08-23-2")
    );
}

/// The loft repair is resolved before the engine and said only on its word.
#[test]
fn a_club_the_bag_lacks_is_a_repair_the_engine_must_ask_for() {
    let bundle = fixture(Spec {
        club: Some("3w"),
        ..Spec::default()
    });
    let assembled = ready(run(&bundle.swing_dir, &bundle.options).0);
    assert_eq!(assembled.input.loft_deg, None);
    let remedy = "club 3w is not in this golfer's bag, so its loft is unknown and the ball flight \
                  could not be simulated — add the club on the bag page";
    assert_eq!(assembled.loft_remedy.as_deref(), Some(remedy));
    assert!(
        !assembled.notes.iter().any(|note| note == remedy),
        "the repair is not a note before the engine has asked"
    );
    let missed = UnscoredCheckpoint {
        name: "ball_flight".to_string(),
        reason: UnscoredReason::NoClubLoft,
        detail: String::new(),
    };
    assert_eq!(assembled.loft_note(&[missed]), Some(remedy));
    assert_eq!(assembled.loft_note(&[]), None);

    // No shot, no loft looked for: the remedy belongs to the flight a photo would have fed.
    let photoless = fixture(Spec {
        club: Some("3w"),
        photo: false,
        ..Spec::default()
    });
    let (assembly, log) = run(&photoless.swing_dir, &photoless.options);
    let assembled = ready(assembly);
    assert_eq!(
        (assembled.input.loft_deg, assembled.loft_remedy),
        (None, None)
    );
    let note = "no shot-screen photo in this bundle — no launch-monitor data to attach";
    assert!(log.contains(&format!("  {note}")), "{log:?}");
    assert_eq!(assembled.notes.last().map(String::as_str), Some(note));
    assert_eq!(assembled.missing, [Role::ShotScreen]);
}

#[test]
fn a_shot_with_no_strike_in_either_clip_is_noted_after_selection() {
    let bundle = fixture(Spec {
        face_on: Some(ViewSpec::heard(&[])),
        ..Spec::default()
    });
    let assembled = ready(run(&bundle.swing_dir, &bundle.options).0);
    assert_eq!(
        assembled.notes.last().map(String::as_str),
        Some(
            "the simulator recorded a shot for this swing but no clip in the bundle carries a ball \
             strike — the footage and the shot data may not describe the same swing"
        )
    );
    assert_eq!(assembled.notes.len(), 6, "{:#?}", assembled.notes);

    // Nobody listened: no strike lists at all, so nothing disagrees with the shot.
    let unheard = fixture(Spec {
        face_on: Some(ViewSpec {
            keypoints: true,
            strikes: None,
        }),
        down_the_line: Some(ViewSpec {
            keypoints: true,
            strikes: None,
        }),
        ..Spec::default()
    });
    let assembled = ready(run(&unheard.swing_dir, &unheard.options).0);
    assert_eq!(
        (
            &assembled.input.face_on_strikes,
            &assembled.input.down_the_line_strikes
        ),
        (&None, &None)
    );
    assert!(
        !assembled
            .notes
            .iter()
            .any(|note| note.starts_with("the simulator recorded a shot")),
        "{:#?}",
        assembled.notes
    );
}

#[test]
fn a_swing_nobody_is_attributed_to_or_an_unregistered_golfer_says_which() {
    for (player, said) in [
        (
            None,
            "no golfer is attributed to this swing, so the head_stays_back checkpoint could not be \
             scored — its sign depends on which side the golfer swings from"
                .to_string(),
        ),
        (
            Some("nobody"),
            "golfer 'nobody' is not in the registry, so the head_stays_back checkpoint could not be \
             scored — its sign depends on which side the golfer swings from"
                .to_string(),
        ),
    ] {
        let bundle = fixture(Spec {
            player,
            ..Spec::default()
        });
        let (assembly, log) = run(&bundle.swing_dir, &bundle.options);
        let assembled = ready(assembly);
        assert_eq!(assembled.input.handedness, None);
        assert_eq!(assembled.notes.last(), Some(&said));
        assert_eq!(log.last(), Some(&format!("\nGolfer:\n  {said}")));
    }
}

#[test]
fn a_missing_view_is_said_and_noted_and_selection_reads_the_other() {
    let bundle = fixture(Spec {
        down_the_line: None,
        ..Spec::default()
    });
    let (assembly, log) = run(&bundle.swing_dir, &bundle.options);
    let assembled = ready(assembly);
    assert_eq!(log[0], format!("\nBundle {SESSION}/{SWING}  [collecting]"));
    assert_eq!(log[1], "  missing: down_the_line");
    let (face_line, face_note) = declined("face-on", "lead wrist");
    assert_eq!(
        assembled.notes,
        [
            "analyzed without down_the_line — the result is thinner than a complete bundle's"
                .to_string(),
            band_note(),
            face_note,
        ]
    );
    assert!(log.contains(&face_line));
    assert_eq!(assembled.missing, [Role::DownTheLine]);
    assert!(assembled.input.down_the_line.is_none());
    assert_eq!(assembled.input.down_the_line_strikes, None);
}

#[test]
fn windows_given_with_auto_window_off_reach_the_engine_unpicked() {
    let bundle = fixture(Spec::default());
    let options = PipelineOptions {
        window_face_on: Some((3, 20)),
        window_down_the_line: Some((5, 25)),
        auto_window: false,
        club: ClubCategory::MidIron,
        ..bundle.options.clone()
    };
    let (assembly, log) = run(&bundle.swing_dir, &options);
    let assembled = ready(assembly);
    assert!(
        !log.iter().any(|line| line == "\nSwing selection:"),
        "{log:?}"
    );
    assert_eq!(
        (
            assembled.input.face_on_window,
            assembled.input.down_the_line_window
        ),
        (Some((3, 20)), Some((5, 25)))
    );
    assert_eq!(assembled.notes, [band_note(), NO_STRIKE_DTL.to_string()]);
    assert_eq!(
        assembled.input.intent.map(|goal| goal.club),
        Some(ClubCategory::MidIron),
        "`--club` is the goal's category"
    );
}

/// `_corpus_vector`'s cut, worked by hand: a ten-frame clip windowed on `[2, 6)`.
#[test]
fn the_corpus_cut_slices_restates_and_shifts_as_corpus_vector_does() {
    let mut face_on = flat_keypoints(FACE_SHA);
    face_on.frames.truncate(10);
    let input = VectorInput {
        swing_id: SWING.to_string(),
        session_id: SESSION.to_string(),
        face_on,
        down_the_line: Some(flat_keypoints(DTL_SHA)),
        shot: None,
        intent: Some(PracticeGoal::default()),
        face_on_window: Some((2, 6)),
        down_the_line_window: None,
        // 1 is before the window and 6 is its end, so only 2 and 5 are kept, moved by 2.
        face_on_strikes: Some(vec![5, 1, 2, 6]),
        // Heard, but nothing that a cut keeps, which a vector writes as null as it writes none.
        down_the_line_strikes: Some(Vec::new()),
        handedness: Some(Handedness::Left),
        loft_deg: Some(30.5),
    };
    let cut = corpus_input(&input);
    let given = &cut.input;
    assert_eq!(
        given.as_object().unwrap().keys().collect::<Vec<_>>(),
        [
            "down_the_line",
            "down_the_line_strikes",
            "face_on",
            "face_on_strikes",
            "handedness",
            "intent",
            "loft_deg",
            "session_id",
            "shot",
            "swing_id"
        ]
    );
    assert_eq!(given["face_on_strikes"], json!([3, 0]));
    assert_eq!(given["down_the_line_strikes"], Value::Null);
    let frames = given["face_on"]["frames"].as_array().unwrap();
    assert_eq!(
        frames
            .iter()
            .map(|f| f["frame_index"].clone())
            .collect::<Vec<_>>(),
        [json!(2), json!(3), json!(4), json!(5)],
        "the slice keeps the whole clip's frame numbers"
    );
    assert_eq!(given["face_on"]["clip"]["frame_count"], json!(4));
    assert_eq!(given["down_the_line"]["clip"]["frame_count"], json!(FRAMES));
    assert_eq!(given["face_on"]["pose_estimator"], json!("mediapipe:heavy"));
    assert_eq!(given["handedness"], json!("left"));
    assert_eq!(given["shot"], Value::Null);
    assert_eq!(given["intent"]["club"], json!("all"));
    assert_eq!(
        json!(cut.frame_offset),
        json!({"face_on": 2, "down_the_line": 0})
    );
    assert_eq!(
        json!(cut.source_clip_frames),
        json!({"face_on": 10, "down_the_line": FRAMES})
    );
}
