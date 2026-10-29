//! `golf_lab::reanalyze` on scratch bundles and photo entries: each form of "out of date" in the
//! script's order and words, the walk, the filters, the runs and their one-line diffs, the
//! stale-video line, and idempotence. [M29 P27]
//!
//! Every run is over `tests/bundle/mod.rs`' scratch bundle, served from caches: nothing is posed,
//! decoded or read, and the coach refuses to be asked, since coaching is off unless `--coaching`
//! (the plan's C5). The text is held to frozen `reanalyze.py`'s on stored bundles in the M29 plan's
//! P27 findings; here, the lines a scratch bundle reaches.

mod bundle;

use std::fs;
use std::path::{Path, PathBuf};

use bundle::{fixture, role_entry, sidecar, NoReads, Spec, SESSION, SWING};
use contracts::swing::ANALYSIS_VERSION;
use contracts::Timestamp;
use golf_lab::coach::{Coach, CoachAnswer, CoachRequest};
use golf_lab::ffmpeg::FfmpegMissing;
use golf_lab::pipeline::{PipelineOptions, Tools};
use golf_lab::reanalyze::{
    collect, reanalyze, DuplicatesByRoot, Request, BY_REQUEST, NEVER_ANALYZED, REUPLOADED,
};
use serde_json::{json, Value};
use storage::manifest::Role;
use storage::state::{save_state, AnalysisState, Status, ANALYSIS_NAME, SHOT_ANALYSIS_NAME};

/// A coach that fails the case if asked: no run here has coaching on.
struct Never;

impl Coach for Never {
    fn coach(&mut self, _: &CoachRequest<'_>) -> CoachAnswer {
        panic!("a reanalyze run with coaching off asked the coach")
    }
}

fn request(sessions: &Path, options: &PipelineOptions) -> Request {
    let mut options = options.clone();
    options.coaching = false;
    Request {
        refs: Vec::new(),
        sessions_dir: sessions.to_path_buf(),
        player: None,
        everything: false,
        dry_run: false,
        verbose: false,
        options,
    }
}

/// One invocation: its exit code, stdout and stderr.
fn run(request: &Request) -> (u8, String, String) {
    let missing = FfmpegMissing { tried: Vec::new() };
    let mut sidecar = sidecar();
    let mut reader = NoReads;
    let mut tools = Tools {
        ffmpeg: Err(&missing),
        sidecar: &mut sidecar,
        reader: &mut reader,
    };
    let mut micros = 1_792_000_000_000_000;
    let mut clock = move || {
        micros += 1_500_000;
        Timestamp::from_utc_micros(micros)
    };
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = reanalyze(
        request, &mut tools, &mut Never, &mut clock, &mut out, &mut err,
    )
    .unwrap();
    assert!(!sidecar.started(), "a case reached the pose sidecar");
    (
        code,
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap(),
    )
}

fn sessions_of(swing_dir: &Path) -> PathBuf {
    swing_dir.parent().unwrap().parent().unwrap().to_path_buf()
}

fn read(path: &Path) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

fn write(path: &Path, value: &Value) {
    fs::write(path, serde_json::to_vec_pretty(value).unwrap()).unwrap();
}

/// A swing directory holding only a manifest: what the walk and the reasons read.
fn manifest_only(
    sessions: &Path,
    swing: &str,
    roles: &[(Role, &str)],
    player: Option<&str>,
) -> PathBuf {
    let dir = sessions.join(SESSION).join(swing);
    fs::create_dir_all(&dir).unwrap();
    let roles: serde_json::Map<String, Value> = roles
        .iter()
        .map(|(role, sha)| {
            let name = format!("{}.{}", role.as_str(), &sha[..12]);
            (
                role.as_str().to_string(),
                role_entry(*role, &name, sha, "video/quicktime"),
            )
        })
        .collect();
    let mut manifest = json!({
        "swing_id": swing, "session_id": SESSION,
        "created_at": "2026-08-23T05:29:40.000000Z", "updated_at": "2026-08-23T05:29:50.000000Z",
        "roles": roles,
    });
    if let Some(player) = player {
        manifest["player_id"] = json!(player);
    }
    write(&dir.join("manifest.json"), &manifest);
    dir
}

const CLIP: &str = "c1000000000000000000000000000000000000000000000000000000000000c1";

/// Each form the script names, plus the parser's, in the script's order and words, and a current
/// result left out of the walk but listed by `--all` and by name.
#[test]
fn each_form_of_out_of_date_is_named_in_the_scripts_words() {
    let root = tempfile::tempdir().unwrap();
    let sessions = root.path().join("sessions");
    let face = [(Role::FaceOn, CLIP)];
    let current = json!({"analysis_version": ANALYSIS_VERSION});

    manifest_only(&sessions, "1", &face, Some("aaron"));
    let reuploaded = manifest_only(&sessions, "2", &face, Some("aaron"));
    write(
        &reuploaded.join(ANALYSIS_NAME),
        &json!({"analysis_version": 16}),
    );
    let mut state = AnalysisState::new(Status::Done);
    state
        .inputs
        .insert("face_on".into(), "an older upload".into());
    save_state(&state, &reuploaded).unwrap();
    let outdated = manifest_only(&sessions, "3", &face, Some("aaron"));
    write(
        &outdated.join(ANALYSIS_NAME),
        &json!({"analysis_version": 16}),
    );
    let older_parse = manifest_only(&sessions, "4", &face, Some("aaron"));
    let mut parsed = current.clone();
    parsed["swing"] = json!({"shot": {"shot_id": "x", "provenance": {"parser_version": 0}}});
    write(&older_parse.join(ANALYSIS_NAME), &parsed);
    let up_to_date = manifest_only(&sessions, "5", &face, Some("aaron"));
    write(&up_to_date.join(ANALYSIS_NAME), &current);
    // A shot no parser read (no provenance) is current, as `parse_is_current` has it.
    let fed = manifest_only(&sessions, "6", &face, None);
    let mut direct = current.clone();
    direct["swing"] = json!({"shot": {"shot_id": "y", "provenance": null}});
    write(&fed.join(ANALYSIS_NAME), &direct);
    // A matching state does not make an outdated result current.
    let matching = manifest_only(&sessions, "10", &face, Some("aaron"));
    write(
        &matching.join(ANALYSIS_NAME),
        &json!({"analysis_version": "17"}),
    );
    let mut state = AnalysisState::new(Status::Done);
    state.inputs.insert("face_on".into(), CLIP.into());
    save_state(&state, &matching).unwrap();

    let options = PipelineOptions::new(
        golf_lab::shot::ShotOptions::new(root.path().join("shots")),
        root.path().join("golfers"),
    );
    let base = request(&sessions, &options);
    let listed = |request: &Request| -> Vec<(String, String)> {
        collect(request, &mut DuplicatesByRoot::default())
            .0
            .iter()
            .map(|target| (target.reference(), target.reason.clone()))
            .collect()
    };
    let at = |swing: &str| format!("{SESSION}/{swing}");
    let engine = |stored: i64| {
        format!("analyzed by engine version {stored}, {ANALYSIS_VERSION} is installed")
    };
    let parser = "its shot was read by an unversioned screen parser, screen parser 1 is installed";

    // The walk: swings by number, so `10` after `5`; the current two left out.
    assert_eq!(
        listed(&base),
        vec![
            (at("1"), NEVER_ANALYZED.to_string()),
            (at("2"), REUPLOADED.to_string()),
            (at("3"), engine(16)),
            (at("4"), parser.to_string()),
            (at("10"), engine(0)),
        ]
    );

    let mut all = base.clone();
    all.everything = true;
    let everything = listed(&all);
    assert_eq!(everything.len(), 7);
    assert_eq!(everything[4], (at("5"), BY_REQUEST.to_string()));
    assert_eq!(everything[5], (at("6"), BY_REQUEST.to_string()));

    let mut named = base.clone();
    named.refs = vec![at("5"), at("3"), at("5")];
    assert_eq!(
        listed(&named),
        vec![
            (at("5"), BY_REQUEST.to_string()),
            (at("3"), engine(16)),
            (at("5"), BY_REQUEST.to_string()),
        ],
        "named targets run in the order given, current or not, as often as named"
    );

    let mut player = all.clone();
    player.player = Some("aaron".into());
    assert!(
        !listed(&player)
            .iter()
            .any(|(reference, _)| *reference == at("6")),
        "--player leaves out a swing nobody is attributed"
    );
    player.player = Some("Aaron".into());
    assert!(listed(&player).is_empty(), "by stored id, exactly");
}

#[test]
fn a_named_target_that_resolves_to_nothing_stops_the_run_with_2() {
    let bundle = fixture(Spec::default());
    let sessions = sessions_of(&bundle.swing_dir);
    let mut named = request(&sessions, &bundle.options);
    named.refs = vec![
        format!("{SESSION}/{SWING}"),
        "nowhere/1".into(),
        "it's".into(),
    ];
    let (code, out, err) = run(&named);
    assert_eq!(code, 2);
    assert!(out.is_empty(), "{out}");
    let looked = golf_lab::import::spelled(&sessions).display().to_string();
    assert_eq!(
        err,
        format!(
            "error: no swing bundle at 'nowhere/1' (looked in {looked})\n\
             error: no swing bundle at \"it's\" (looked in {looked})\n"
        )
    );
    assert!(
        !bundle.swing_dir.join(ANALYSIS_NAME).exists(),
        "nothing ran"
    );
}

/// A swing end to end: listed, run, diffed, then reported current; then made outdated under a
/// standing render whose anchors the run moves.
#[test]
fn a_swing_is_rerun_diffed_and_then_current() {
    let bundle = fixture(Spec::default());
    let sessions = sessions_of(&bundle.swing_dir);
    let walk = request(&sessions, &bundle.options);

    let mut dry = walk.clone();
    dry.dry_run = true;
    let (code, out, _) = run(&dry);
    assert_eq!(code, 0);
    assert_eq!(
        out,
        format!(
            "1 swing(s) to re-analyze (engine version {ANALYSIS_VERSION}):\n  \
             {SESSION}/{SWING}             never analyzed\n\n--dry-run: nothing was changed.\n"
        )
    );
    assert!(!bundle.swing_dir.join(ANALYSIS_NAME).exists());

    let (code, out, err) = run(&walk);
    assert_eq!((code, err.as_str()), (0, ""), "{out}");
    assert!(
        out.contains(&format!(
            "\n{SESSION}/{SWING}  (never analyzed)\n  version 0 -> {ANALYSIS_VERSION} | score -- -> "
        )),
        "{out}"
    );
    assert!(out.ends_with("\nRe-analyzed 1/1 swing(s).\n"), "{out}");
    assert!(
        !out.contains("Bundle "),
        "the pipeline's narration is --verbose's"
    );
    let written = read(&bundle.swing_dir.join(ANALYSIS_NAME));
    assert_eq!(written["analysis_version"], json!(ANALYSIS_VERSION));

    let (code, out, _) = run(&walk);
    assert_eq!(code, 0);
    assert_eq!(
        out,
        format!(
            "Nothing to re-analyze in {} — every stored result is current.\n",
            golf_lab::import::spelled(&sessions).display()
        ),
        "idempotent: the second run has nothing to do"
    );

    let mut verbose = walk.clone();
    verbose.everything = true;
    verbose.verbose = true;
    let (code, out, _) = run(&verbose);
    assert_eq!(code, 0);
    assert!(
        out.contains(&format!("\n{SESSION}/{SWING}  ({BY_REQUEST})\n\nBundle ")),
        "{out}"
    );
    assert!(out.contains("\n  no change\n"), "{out}");

    // Outdated, with anchors the run will move under a render it does not redraw.
    let mut stale = written.clone();
    stale["analysis_version"] = json!(16);
    stale["alignment"] = json!({"a": {"anchors": {"motion_start": 1, "top": 2, "impact": 3}}});
    write(&bundle.swing_dir.join(ANALYSIS_NAME), &stale);
    fs::write(bundle.swing_dir.join("aligned.mp4"), b"").unwrap();
    let (code, out, _) = run(&walk);
    assert_eq!(code, 1, "a stale video wants a human: {out}");
    assert!(
        out.contains(&format!(
            "  version 16 -> {ANALYSIS_VERSION} | anchors ((1, 2, 3), (None,)) -> "
        )),
        "{out}"
    );
    assert!(
        out.contains(
            "\n  ! the alignment anchors moved, so aligned.mp4 no longer matches this result — and \
             golf-lab renders no side-by-side video to replace it (the M29 plan's C4)\n"
        ),
        "{out}"
    );
}

/// A run that fails is a `FAILED` line, the walk goes on, and the exit is 1.
#[test]
fn a_failed_run_is_named_and_the_walk_goes_on() {
    let bundle = fixture(Spec {
        face_on: None,
        down_the_line: None,
        photo: false,
        ..Spec::default()
    });
    let sessions = sessions_of(&bundle.swing_dir);
    let (code, out, _) = run(&request(&sessions, &bundle.options));
    assert_eq!(code, 1, "{out}");
    assert!(
        out.contains(&format!(
            "\n{SESSION}/{SWING}  (never analyzed)\n  FAILED: "
        )),
        "{out}"
    );
    assert!(
        out.ends_with("\nRe-analyzed 0/1 swing(s).\n1 failed — see above.\n"),
        "{out}"
    );
}

/// A photo entry (C7): re-run through `run-shot --session`'s own code into `shot_analysis.json`,
/// each form of its staleness named, and current afterwards.
#[test]
fn a_photo_entry_is_rerun_into_its_shot_analysis() {
    let bundle = fixture(Spec {
        face_on: None,
        down_the_line: None,
        ..Spec::default()
    });
    let sessions = sessions_of(&bundle.swing_dir);
    let walk = request(&sessions, &bundle.options);
    let file = bundle.swing_dir.join(SHOT_ANALYSIS_NAME);

    let (code, out, err) = run(&walk);
    assert_eq!((code, err.as_str()), (0, ""), "{out}");
    let written = read(&file);
    let measured = written["measurements"].as_array().unwrap().len();
    assert!(measured > 0);
    assert!(
        out.contains(&format!(
            "\n{SESSION}/{SWING}  (never analyzed)\n  version 0 -> {ANALYSIS_VERSION} | \
             measurements 0 -> {measured}\n"
        )),
        "{out}"
    );
    assert!(
        !bundle.swing_dir.join(ANALYSIS_NAME).exists(),
        "one artifact per kind"
    );
    assert!(run(&walk).1.starts_with("Nothing to re-analyze in "));

    let reason_after = |edit: &dyn Fn(&mut Value)| {
        let mut changed = written.clone();
        edit(&mut changed);
        write(&file, &changed);
        let mut dry = walk.clone();
        dry.dry_run = true;
        let listed = collect(&dry, &mut DuplicatesByRoot::default()).0;
        assert_eq!(listed.len(), 1);
        listed[0].reason.clone()
    };
    assert_eq!(
        reason_after(&|v| v["photo_sha256"] = json!("ab".repeat(32))),
        REUPLOADED
    );
    assert_eq!(
        reason_after(&|v| v["analysis_version"] = json!(16)),
        format!("analyzed by engine version 16, {ANALYSIS_VERSION} is installed")
    );
    assert_eq!(
        reason_after(&|v| v["shot"]["provenance"]["parser_version"] = json!(0)),
        "its shot was read by an unversioned screen parser, screen parser 1 is installed"
    );

    // The run re-attaches the store's current shot, and nothing it measures moves.
    let (code, out, _) = run(&walk);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("\n  no change\n"), "{out}");
    assert_eq!(read(&file), written);
}

/// A photo entry the corpus counts as a duplicate is never out of date, and a run by name prints
/// the corpus's sentence.
#[test]
fn a_duplicate_photo_entry_is_left_out_and_skipped_by_name() {
    let bundle = fixture(Spec {
        face_on: None,
        down_the_line: None,
        ..Spec::default()
    });
    let sessions = sessions_of(&bundle.swing_dir);
    // A filmed swing by the same golfer holding the same photo: the copy the corpus keeps.
    let manifest = read(&bundle.swing_dir.join("manifest.json"));
    let photo = manifest["roles"]["shot_screen"].clone();
    let filmed = manifest_only(&sessions, "3", &[(Role::FaceOn, CLIP)], Some("aaron"));
    let mut filmed_manifest = read(&filmed.join("manifest.json"));
    filmed_manifest["roles"]["shot_screen"] = photo;
    write(&filmed.join("manifest.json"), &filmed_manifest);

    let mut dry = request(&sessions, &bundle.options);
    dry.dry_run = true;
    let (_, out, _) = run(&dry);
    assert!(out.contains(&format!("  {SESSION}/3 ")), "{out}");
    assert!(!out.contains(&format!("{SESSION}/{SWING}")), "{out}");

    let mut named = request(&sessions, &bundle.options);
    named.refs = vec![format!("{SESSION}/{SWING}")];
    let (code, out, _) = run(&named);
    assert_eq!(code, 0, "{out}");
    assert!(
        out.contains(&format!(
            "\n{SESSION}/{SWING}  ({BY_REQUEST})\n  skipped: shot photo "
        )),
        "{out}"
    );
    assert!(!bundle.swing_dir.join(SHOT_ANALYSIS_NAME).exists());
}
