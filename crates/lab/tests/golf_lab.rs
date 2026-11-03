//! The binary's argument handling, before any verb exists. [M29 P20]
//!
//! What every later verb inherits: help on stdout with 0, a missing or unknown verb on stderr with
//! 1, as `golf-core` answers both. And since P26, `analyze`'s own: its refusals, each exit 2, and
//! one run over `tests/bundle/mod.rs`' scratch bundle, served from caches, with coaching off. Since
//! P27, `reanalyze`'s and `analyze-keypoints`' refusals, a dry run, and one report over the scratch
//! bundle's keypoints. Since P30, `build-vectors`' refusals and a dry run over an empty store, whose
//! default root is the committed `spec/vectors/` and so must refuse the real corpus by name. Since
//! M42 P13, `read-impact`'s refusals, and a clip that is not there named on stdout with exit 1. Since
//! M42 P14b, `import-video`'s refusals, a device that declares nothing from the replay first.

mod bundle;

use std::process::{Command, Output};

use bundle::{fixture, Spec, FRAMES, SESSION, SWING};

fn golf_lab(args: &[&str]) -> Output {
    golf_lab_with(args, &[])
}

/// The binary with `env` set over the process's, and `GOLF_COACHING_ENABLED` cleared unless a case
/// sets it, so a developer's shell cannot turn a case's coaching on.
fn golf_lab_with(args: &[&str], env: &[(&str, &str)]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_golf-lab"));
    command.args(args).env_remove("GOLF_COACHING_ENABLED");
    for (name, value) in env {
        command.env(name, value);
    }
    command.output().expect("golf-lab runs")
}

#[test]
fn help_is_the_usage_on_stdout() {
    for flag in ["--help", "-h", "help"] {
        let out = golf_lab(&[flag]);
        assert_eq!(out.status.code(), Some(0), "{flag}");
        let stdout = String::from_utf8(out.stdout).unwrap();
        assert!(stdout.starts_with("usage: golf-lab"), "{flag}: {stdout}");
        assert!(out.stderr.is_empty(), "{flag}");
    }
}

#[test]
fn a_missing_or_unknown_verb_is_named_on_stderr() {
    for (args, problem) in [
        (&[][..], "golf-lab: no command given"),
        (&["analyse"][..], "golf-lab: unknown command \"analyse\""),
        (
            &["--help", "extra"][..],
            "golf-lab: unknown command \"--help\"",
        ),
    ] {
        let out = golf_lab(args);
        assert_eq!(out.status.code(), Some(1), "{args:?}");
        assert!(out.stdout.is_empty(), "{args:?}");
        let stderr = String::from_utf8(out.stderr).unwrap();
        assert!(stderr.starts_with(problem), "{args:?}: {stderr}");
        assert!(stderr.contains("usage: golf-lab"), "{args:?}");
    }
}

/// Each refusal names the flag and exits 2, the script's code for no result, before anything runs.
#[test]
fn analyze_refuses_what_it_cannot_run() {
    let empty = tempfile::tempdir().unwrap();
    let sessions = empty.path().to_string_lossy().into_owned();
    for (args, env, problem) in [
        (
            &["analyze"][..],
            &[][..],
            "analyze: error: the following arguments are required: swing",
        ),
        (
            &["analyze", "a/1", "--tau", "-0.4:3.0"][..],
            &[][..],
            "analyze: argument --tau: golf-lab renders no side-by-side video",
        ),
        (
            &["analyze", "a/1", "--window-face-on", "676"][..],
            &[][..],
            "analyze: argument --window-face-on: window must look like START:END, got '676'",
        ),
        (
            &["analyze", "a/1", "--club", "spoon"][..],
            &[][..],
            "analyze: argument --club: invalid ClubCategory value: 'spoon'",
        ),
        (
            &["analyze", "a/1"][..],
            &[("GOLF_COACHING_ENABLED", "maybe")][..],
            "analyze: error: GOLF_COACHING_ENABLED=\"maybe\" is not a bool",
        ),
    ] {
        let out = golf_lab_with(args, env);
        assert_eq!(out.status.code(), Some(2), "{args:?}");
        let stderr = String::from_utf8(out.stderr).unwrap();
        assert!(
            stderr.starts_with(&format!("golf-lab: {problem}")),
            "{args:?}: {stderr}"
        );
    }

    let out = golf_lab(&["analyze", "nowhere/1", "--sessions-dir", &sessions]);
    assert_eq!(out.status.code(), Some(2));
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(
        stderr.starts_with("error: no swing bundle at 'nowhere/1' (looked in "),
        "{stderr}"
    );
    assert!(out.stdout.is_empty());

    let out = golf_lab(&["analyze", "--help"]);
    assert_eq!(out.status.code(), Some(0));
    assert!(String::from_utf8(out.stdout)
        .unwrap()
        .contains("golf-lab analyze SWING"));
}

/// The verb end to end on a scratch bundle: the run's log, the report, exit 0, and the two
/// artifacts; then `--list-swings`, which writes nothing.
#[test]
fn analyze_runs_a_scratch_bundle_and_lists_its_swings() {
    let bundle = fixture(Spec::default());
    let sessions = bundle.swing_dir.parent().unwrap().parent().unwrap();
    let dirs = [
        "--sessions-dir".to_string(),
        sessions.to_string_lossy().into_owned(),
        "--shots-dir".to_string(),
        bundle.options.shot.shots_dir.to_string_lossy().into_owned(),
        "--golfers-dir".to_string(),
        bundle.options.golfers_dir.to_string_lossy().into_owned(),
    ];
    let pair = format!("{SESSION}/{SWING}");
    let mut args: Vec<&str> = vec!["analyze", &pair, "--list-swings"];
    args.extend(dirs.iter().map(String::as_str));
    let out = golf_lab(&args);
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        stdout.contains(&format!(
            "
face-on: {FRAMES} frames, 0 candidate descent(s)
"
        )),
        "{stdout}"
    );
    assert!(
        !bundle.swing_dir.join("analysis.json").exists(),
        "a listing writes nothing"
    );

    args[2] = "--no-coaching";
    let out = golf_lab(&args);
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert_eq!(out.status.code(), Some(0), "{stdout}");
    assert!(stdout.starts_with(&format!(
        "
Bundle {SESSION}/{SWING}  [complete]
"
    )));
    assert!(!stdout.contains(
        "
Coaching:
"
    ));
    assert!(stdout.contains(
        "
Wrote "
    ));
    assert!(stdout.contains(&format!(
        "
Swing {SWING}  (session {SESSION})
Overall score: "
    )));
    assert!(
        stdout.ends_with(
            "

"
        ),
        "the script's closing print()"
    );
    assert!(bundle.swing_dir.join("analysis.json").is_file());
    assert!(bundle.swing_dir.join("analysis.state.json").is_file());
}

/// `reanalyze`'s refusals, each exit 2 before anything runs, then a dry run over an empty store.
#[test]
fn reanalyze_refuses_what_it_cannot_run_and_lists_nothing_in_an_empty_store() {
    let empty = tempfile::tempdir().unwrap();
    let sessions = empty.path().to_string_lossy().into_owned();
    for (args, env, problem) in [
        (
            &["reanalyze", "--video"][..],
            &[][..],
            "reanalyze: argument --video: golf-lab renders no side-by-side video",
        ),
        (
            &["reanalyze", "--all=yes"][..],
            &[][..],
            "reanalyze: argument --all: ignored explicit argument \"yes\"",
        ),
        (
            &["reanalyze", "--player"][..],
            &[][..],
            "reanalyze: argument --player: expected one argument",
        ),
        (
            &["reanalyze", "--tau", "1"][..],
            &[][..],
            "reanalyze: unrecognized arguments: --tau",
        ),
        (
            &["reanalyze"][..],
            &[("GOLF_COACHING_ENABLED", "maybe")][..],
            "reanalyze: error: GOLF_COACHING_ENABLED=\"maybe\" is not a bool",
        ),
    ] {
        let out = golf_lab_with(args, env);
        assert_eq!(out.status.code(), Some(2), "{args:?}");
        let stderr = String::from_utf8(out.stderr).unwrap();
        assert!(
            stderr.starts_with(&format!("golf-lab: {problem}")),
            "{args:?}: {stderr}"
        );
    }

    let out = golf_lab(&["reanalyze", "nowhere/1", "--sessions-dir", &sessions]);
    assert_eq!(out.status.code(), Some(2));
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(
        stderr.starts_with("error: no swing bundle at 'nowhere/1' (looked in "),
        "{stderr}"
    );

    let out = golf_lab(&["reanalyze", "--dry-run", "--sessions-dir", &sessions]);
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.starts_with("Nothing to re-analyze in "), "{stdout}");
    assert!(
        stdout.ends_with(" — every stored result is current.\n"),
        "{stdout}"
    );

    let out = golf_lab(&["reanalyze", "--help"]);
    assert_eq!(out.status.code(), Some(0));
    assert!(String::from_utf8(out.stdout)
        .unwrap()
        .contains("golf-lab reanalyze [SWINGS...]"));
}

/// `analyze-keypoints`: its refusals, the script's `not found`, and the report over the scratch
/// bundle's flat face-on keypoints, which segment to nothing and so score nothing.
#[test]
fn analyze_keypoints_reports_one_file_and_refuses_an_overlay() {
    for (args, problem) in [
        (
            &["analyze-keypoints"][..],
            "analyze-keypoints: error: the following arguments are required: keypoints",
        ),
        (
            &["analyze-keypoints", "a.json", "--overlay", "a.mov"][..],
            "analyze-keypoints: argument --overlay: golf-lab renders no annotated clip",
        ),
        (
            &["analyze-keypoints", "a.json", "b.json"][..],
            "analyze-keypoints: unrecognized arguments: b.json",
        ),
    ] {
        let out = golf_lab(args);
        assert_eq!(out.status.code(), Some(2), "{args:?}");
        let stderr = String::from_utf8(out.stderr).unwrap();
        assert!(
            stderr.starts_with(&format!("golf-lab: {problem}")),
            "{args:?}: {stderr}"
        );
    }

    let empty = tempfile::tempdir().unwrap();
    let absent = empty.path().join("absent.keypoints.json");
    let out = golf_lab(&["analyze-keypoints", &absent.to_string_lossy()]);
    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.starts_with("error: ") && stderr.ends_with("absent.keypoints.json not found\n"));

    let bundle = fixture(Spec::default());
    let keypoints = bundle.swing_dir.join("face_on.keypoints.json");
    let out = golf_lab(&["analyze-keypoints", &keypoints.to_string_lossy()]);
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert_eq!(out.status.code(), Some(0), "{stdout}");
    assert!(
        stdout.starts_with(&format!(
            "\nSwing: face_on   frames: {FRAMES}\nOverall score: "
        )),
        "{stdout}"
    );
    assert!(stdout.contains("\nDetected instants (frame): "), "{stdout}");
    assert!(stdout.contains("\nCheckpoints:\n"), "{stdout}");
    assert!(stdout.contains("\n  not measured: "), "{stdout}");
    assert!(
        stdout.contains("\nTips (most actionable first):\n"),
        "{stdout}"
    );
    assert!(stdout.ends_with("\n\n"), "the script's closing print()");
}

/// `build-vectors`' refusals, each exit 2 before anything runs; then a dry run over an empty store
/// into the default root, the committed `spec/vectors/`, which builds only the real corpus, finds
/// it committed and refuses it by name with exit 1, writing nothing.
#[test]
fn build_vectors_refuses_what_it_cannot_run_and_a_committed_vector_by_name() {
    for (args, problem) in [
        (
            &["build-vectors", "--dry-run=yes"][..],
            "build-vectors: argument --dry-run: ignored explicit argument \"yes\"",
        ),
        (
            &["build-vectors", "--out"][..],
            "build-vectors: argument --out: expected one argument",
        ),
        (
            &["build-vectors", "--force"][..],
            "build-vectors: unrecognized arguments: --force",
        ),
        (
            &["build-vectors", "corpus"][..],
            "build-vectors: unrecognized arguments: corpus",
        ),
    ] {
        let out = golf_lab(args);
        assert_eq!(out.status.code(), Some(2), "{args:?}");
        let stderr = String::from_utf8(out.stderr).unwrap();
        assert!(
            stderr.starts_with(&format!("golf-lab: {problem}")),
            "{args:?}: {stderr}"
        );
    }

    let empty = tempfile::tempdir().unwrap();
    let sessions = empty.path().to_string_lossy().into_owned();
    let out = golf_lab(&["build-vectors", "--dry-run", "--sessions-dir", &sessions]);
    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(
        stderr.starts_with("refused: storage/corpus/real is already at "),
        "{stderr}"
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        stdout.starts_with("Would write 0 of 1 vectors under "),
        "{stdout}"
    );

    let out = golf_lab(&["build-vectors", "--help"]);
    assert_eq!(out.status.code(), Some(0));
    assert!(String::from_utf8(out.stdout)
        .unwrap()
        .contains("golf-lab build-vectors [--sessions-dir D]"));
}

/// `import-video`'s refusals, each exit 2 before a model or a clip is touched. `--device hd_golf`
/// answers on a machine with no ffmpeg and no models too, naming the device that has a replay to
/// read, and so does the default device, which is `hd_golf`. `tests/import_video.rs` drives the rest
/// against a fake decoder (M42 P14b).
#[test]
fn import_video_refuses_a_device_with_nothing_from_the_replay_and_wants_a_clip() {
    let empty = tempfile::tempdir().unwrap();
    let absent = empty.path().join("IMG_0000.MOV");
    let absent = absent.to_string_lossy();
    let refused = "error: device 'hd_golf' declares no field read from an impact video, so a clip \
                   of it cannot be imported; full_swing does\n";
    for args in [
        &["import-video", "--device", "hd_golf", &absent][..],
        &["import-video", &absent][..],
    ] {
        let out = golf_lab_with(args, &[("GOLF_LAUNCH_MONITOR_PROFILE", "")]);
        assert_eq!(out.status.code(), Some(2), "{args:?}");
        assert!(out.stdout.is_empty(), "{args:?}");
        assert_eq!(String::from_utf8(out.stderr).unwrap(), refused, "{args:?}");
    }

    for (args, problem) in [
        (
            &["import-video", "--device", "full_swing"][..],
            "import-video: the following arguments are required: CLIP",
        ),
        (
            &["import-video", "a.mov", "--club", "7i"][..],
            "import-video: error: --into-session S, --player P and --club C go together",
        ),
        (
            &["import-video", "a.mov", "--width", "960"][..],
            "import-video: unrecognized arguments: --width",
        ),
    ] {
        let out = golf_lab(args);
        assert_eq!(out.status.code(), Some(2), "{args:?}");
        assert!(out.stdout.is_empty(), "{args:?}");
        let stderr = String::from_utf8(out.stderr).unwrap();
        assert!(
            stderr.starts_with(&format!("golf-lab: {problem}\n")),
            "{args:?}: {stderr}"
        );
    }

    let out = golf_lab(&["import-video", "--help"]);
    assert_eq!(out.status.code(), Some(0));
    assert!(String::from_utf8(out.stdout)
        .unwrap()
        .contains("golf-lab import-video CLIP..."));

    // A models directory with no models is the reader's refusal, exit 1, before any clip is decoded,
    // as `import-photos` answers it.
    let out = golf_lab(&[
        "import-video",
        "--device",
        "full_swing",
        "--models",
        &empty.path().to_string_lossy(),
        &absent,
    ]);
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stdout.is_empty());
}

/// `read-impact`'s refusals, each exit 2 before anything is decoded, a still among them naming the
/// verb that reads one; then a clip that is not there, which is that clip's line on stdout and exit
/// 1, as a clip ffmpeg cannot decode is (M42 P13). The real clips are not run here: they are
/// `data/`'s, and the plan's P13 findings hold what the reader made of them.
#[test]
fn read_impact_refuses_a_still_and_names_a_clip_it_could_not_decode() {
    for (args, problem) in [
        (
            &["read-impact"][..],
            "read-impact: the following arguments are required: CLIP",
        ),
        (
            &["read-impact", "IMG_2889.MOV", "IMG_2889.HEIC"][..],
            "read-impact: IMG_2889.HEIC is not a Live Photo clip (.mov): a photo of the screen is \
             read by golf-lab import-photos",
        ),
        (
            &["read-impact", "a.mov", "--review"][..],
            "read-impact: argument --review: expected one argument",
        ),
        (
            &["read-impact", "a.mov", "--json=yes"][..],
            "read-impact: argument --json: ignored explicit argument \"yes\"",
        ),
        (
            &["read-impact", "a.mov", "--width", "960"][..],
            "read-impact: unrecognized arguments: --width",
        ),
    ] {
        let out = golf_lab(args);
        assert_eq!(out.status.code(), Some(2), "{args:?}");
        assert!(out.stdout.is_empty(), "{args:?}");
        let stderr = String::from_utf8(out.stderr).unwrap();
        assert!(
            stderr.starts_with(&format!("golf-lab: {problem}\n")),
            "{args:?}: {stderr}"
        );
    }

    // `GOLF_FFMPEG` names a file that is there and is no ffmpeg, so the case is the same on a box
    // with none: the clip's absence is found before anything is run.
    let empty = tempfile::tempdir().unwrap();
    let absent = empty.path().join("IMG_0000.MOV");
    let out = golf_lab_with(
        &["read-impact", "--json", &absent.to_string_lossy()],
        &[("GOLF_FFMPEG", env!("CARGO_BIN_EXE_golf-lab"))],
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert_eq!(out.status.code(), Some(1), "{stdout}");
    assert!(
        stdout.starts_with(
            "{\"clip\":\"IMG_0000.MOV\",\"error\":\"could not be decoded: Video file not found: "
        ),
        "{stdout}"
    );
    assert_eq!(stdout.lines().count(), 1, "{stdout}");
}
