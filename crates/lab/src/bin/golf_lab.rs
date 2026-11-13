//! `golf-lab` — the lab CLI. The scripts in front of `api/pipeline.py`. [M29 P20]
//!
//! ```text
//! golf-lab import-photos [PATHS...] [--session S] [--out DIR] [--device D]
//!                        [--min-confidence X] [--force] [--dry-run] [--models DIR]
//! golf-lab import-photos [PATHS...] --into-session S --player P --club C
//!                        [--sessions-dir D] [--golfers-dir D] [--out DIR] [--device D]
//!                        [--min-confidence X] [--force] [--dry-run] [--models DIR]
//! golf-lab import-video CLIP... [--session S] [--out DIR] [--device D] [--min-confidence X]
//!                       [--force] [--dry-run] [--models DIR]
//! golf-lab import-video CLIP... --into-session S --player P --club C
//!                       [--sessions-dir D] [--golfers-dir D] [--out DIR] [--device D]
//!                       [--min-confidence X] [--force] [--dry-run] [--models DIR]
//! golf-lab analyze SWING [--sessions-dir D] [--shots-dir D] [--golfers-dir D]
//!                  [--window-face-on S:E] [--window-dtl S:E] [--no-auto-window] [--list-swings]
//!                  [--no-video] [--no-coaching] [--force-pose] [--force-ocr] [--skip-ocr]
//!                  [--club C] [--min-confidence X] [--models DIR]
//! golf-lab reanalyze [SWINGS...] [--sessions-dir D] [--shots-dir D] [--golfers-dir D]
//!                    [--player P] [--all] [--dry-run] [--coaching] [--verbose] [--models DIR]
//! golf-lab analyze-keypoints KEYPOINTS
//! golf-lab flight --ball-speed MPH --launch-angle DEG --spin RPM [--launch-direction DEG]
//!                 [--spin-axis DEG] [--altitude M] [--step S] [--points N] [--verbose]
//! golf-lab flight (--gate | --shots | --shot SHOT_ID) [--altitude M] [--step S] [--points N]
//!                 [--verbose] [--shots-dir D] [--sessions-dir D] [--golfers-dir D]
//! golf-lab build-vectors [--sessions-dir D] [--golfers-dir D] [--out DIR] [--dry-run]
//! golf-lab import-export EXPORT [--sessions-dir D] [--shots-dir D] [--golfers-dir D] [--dry-run]
//! golf-lab read-impact CLIP... [--json] [--review DIR]
//! golf-lab read-tape PHOTO... --label T [--sequence] [--review DIR]
//! golf-lab --help
//! ```
//!
//! Every verb lands in a shell that parses and refuses as `golf-core` does: a verb's flags are its
//! own, `--help` (or `-h`, or `help`) prints the usage to stdout and exits 0, and a missing or
//! unknown verb prints what was wrong and the usage to stderr and exits 1, `golf-core`'s `usage`.
//! Flags a verb refuses exit 2, as `golf-core`'s career verbs do for argparse's reason: a caller of
//! the script saw 2.
//!
//! # `import-photos` (P24)
//!
//! `scripts/import_shot_screens.py`, whose flags it keeps: `PATHS` (photos and directories; none is
//! `GOLF_SHOT_SCREENS_DIR`, else `data/raw/shot_screens`), `--session`, `--device` and
//! `--min-confidence` (defaulting to `GOLF_LAUNCH_MONITOR_PROFILE` and `GOLF_OCR_MIN_CONFIDENCE`,
//! else `config.py`'s), `--out` (the shot store: `GOLF_SHOTS_DIR`, else `data/processed/shots`),
//! `--force` and `--dry-run`. Its own: `--into-session S --player P --club C`, all three or none,
//! which files each photo as a photo entry and analyses it (`golf_lab::import`'s module doc), with
//! `--sessions-dir` and `--golfers-dir` resolved as `golf-core`'s verbs resolve them, and
//! `--models`, the ONNX directory (`data/models/ocr/` by default), as `golf-ocr read` takes it.
//! `--session` names what a store-only import files its shots under, so it does not go with
//! `--into-session`, which names the session itself.
//!
//! # `analyze` (P26)
//!
//! `scripts/analyze_bundle.py`, whose flags it keeps: `SWING` (`SESSION/SWING` in the store or a
//! swing directory), `--sessions-dir` and `--shots-dir` (`GOLF_SESSIONS_DIR` and `GOLF_SHOTS_DIR`,
//! else `data/processed/`'s), the two windows, `--no-auto-window`, `--list-swings`, `--no-coaching`
//! (coaching is otherwise `GOLF_COACHING_ENABLED`'s, on by default: `golf_lab::coach`), the three
//! `--force-`/`--skip-` switches, `--club` and `--min-confidence`. Its own, as `import-photos` has
//! them: `--golfers-dir` (`GOLF_GOLFERS_DIR`) and `--models`. Exit codes are the script's: 0 clean,
//! 1 a result with something flagged (the shot needs review, a role is missing), 2 no result.
//!
//! **What differs, each declared in the M29 plan's P26 findings:**
//!
//! - no render (C4): `--no-video` is accepted and changes nothing, and `--tau`, which only sets the
//!   render's range, is refused;
//! - where the script raises, this answers in a sentence and exits 2, the script's code for no
//!   result: a window that is not `START:END` (the script's `ArgumentTypeError` escapes `main`), and
//!   any error the pipeline raises;
//! - `not measured:` prints the unscored names, where the script's `join` raises
//!   (`golf_lab::report`'s doc).
//!
//! # `reanalyze` (P27)
//!
//! `scripts/reanalyze.py`, whose flags it keeps: `SWINGS` (`SESSION/SWING` refs or swing
//! directories; none is every swing that needs it), `--sessions-dir` and `--shots-dir`, `--player`,
//! `--all`, `--dry-run`, `--coaching` (off by default, and on only where `GOLF_COACHING_ENABLED`
//! allows it, as the script's `defaults.coaching and args.coaching`: C5) and `--verbose`. Its own,
//! as `analyze` has them: `--golfers-dir` and `--models`. Exit codes are the script's: 0 clean, 1
//! something wants a human, 2 a named target could not be found. `--video` is refused (C4), and
//! what else differs is `golf_lab::reanalyze`'s module doc.
//!
//! # `analyze-keypoints` (P27)
//!
//! `scripts/analyze_swing.py`: one keypoints file, either shape, analysed with no bundle, and its
//! report. A missing file is the script's `error: <path> not found` with exit 1, and so is one that
//! will not read, where the script's traceback exits 1. `--overlay` is refused (C4), and what else
//! differs is `golf_lab::analyze_keypoints`' module doc.
//!
//! # `flight` (P28)
//!
//! `scripts/simulate_flight.py`, whose flags it keeps: the typed conditions (`--ball-speed`,
//! `--launch-angle` and `--spin` together, with `--launch-direction` and `--spin-axis` defaulting
//! to 0), `--gate`, `--shots` and `--shot`, exactly one of the four, and `--altitude`, `--step`,
//! `--points` and `--verbose`. Its own, as `analyze` has them: `--shots-dir`, `--sessions-dir` and
//! `--golfers-dir`, which the stored modes join (`GOLF_SHOTS_DIR`, `GOLF_SESSIONS_DIR` and
//! `GOLF_GOLFERS_DIR`, else `data/processed/`'s). Exit codes are the script's: 0 when a ball flew,
//! and over stored shots whatever they hold; 2 when none flew, for a request that names no mode or
//! two, and for a `--shot` id nothing on disk carries. What differs is `golf_lab::flight`'s module
//! doc.
//!
//! # `build-vectors` (P30)
//!
//! `scripts/conformance_vectors.py`'s builders over `data/`, which `conformance.py regenerate` ran:
//! a corpus vector and its stage vector per stored swing at this engine, and the real storage
//! corpus, recorded by Rust and verified before anything is written (`golf_lab::build_vectors`'
//! module doc). `--sessions-dir` and `--golfers-dir` are resolved as the other verbs resolve them,
//! and `--out` is the vectors root, `spec/vectors/` by default. A vector is written only where no
//! file is: one already there is refused by name and left as it is, since `golf-core rerecord` is
//! the only thing that rewrites a committed vector (the M29 plan's D19). `--dry-run` builds and
//! verifies all of them and says which it would write. Exit codes: 0 every vector written (or
//! writable), 1 one or more refused and the rest written, 2 nothing written because the build
//! stopped, or a write failed.
//!
//! # `import-export` (P31)
//!
//! No script: the phone's export, `EXPORT` a zip archive or a directory of the stored layout itself
//! (`sessions/`, `shots/`, `golfers/`; the M29 plan's D16), merged into the stores
//! (`golf_lab::import_export`'s module doc). `--sessions-dir`, `--shots-dir` and `--golfers-dir` are
//! resolved as the other verbs resolve them. New files are copied, identical ones skipped, a shot
//! and a photo entry whose photo is stored already are left as stored, as are a bag and a golfer of
//! the same id and handedness (M38's D6), and any other file that differs from what is stored
//! refuses the whole import, naming each (C11). `--dry-run` judges every file
//! and writes nothing. Exit codes: 0 imported (or importable), 1 refused on a conflict, 2 the export
//! is not a stored layout this reads, or a write failed part way.
//!
//! # `read-impact` (M42 P13)
//!
//! No script: the impact offset off Full Swing's overhead replay, read from each `CLIP` (a Live
//! Photo's motion part, `.mov` in any case; a still is refused, naming `import-photos`) and printed
//! one line a clip, or with `--json` one object a line (`golf_lab::read_impact`'s module doc).
//! `--review DIR` writes the frames each reading came from as PNGs under `DIR/<stem>/`. Nothing is
//! stored. Exit codes: 0 every clip read, 1 any refused or not decoded (or no ffmpeg), 2 usage.
//!
//! # `import-video` (M42 P14b)
//!
//! No script: a Live Photo's motion part imported as a shot of its own, the second import method
//! (`golf_lab::import`'s module doc). Each `CLIP`, or each `.mov` one level inside a directory, gives
//! a shot holding the grid's numbers off one frame and the impact offset or its named refusal off
//! the replay. It keeps `import-photos`' flags and exit codes, but takes no default directory: the
//! M42 plan's call 14 names its clips, and the default directory holds `IMG_2888.MOV`, the club-view
//! layout no profile reads. A device that declares nothing from the replay exits 2 before anything
//! is read (`--device full_swing`, or `GOLF_LAUNCH_MONITOR_PROFILE`; the default is `hd_golf`). No
//! ffmpeg is a clip that cannot be decoded, for every clip, as with `read-impact`.
//!
//! # `read-tape` (M46 P8)
//!
//! No script: where on the face, read off each `PHOTO` of a STRIKEPRO label on the club and
//! printed as one JSON object a line (`golf_lab::read_tape`'s module doc). `--label` names the label
//! type every photo is read as, by its id (`strikepro_driver`, `strikepro_iron` or
//! `strikepro_hybrid`). **It is required**, where the M46 plan's C23 bracketed it, because no
//! default is safe: the driver and iron labels' boxes are within 1% of one aspect (90.5 × 49.6 and
//! 73 × 39.9 mm), so a photo read as the wrong one could pass the finder's aspect check and be read
//! at the wrong scale, a wrong reading where a refusal was owed. `--sequence` reads each photo
//! against the crop of the last one placed before it, and `--review DIR` writes each photo's review
//! images under `DIR/<stem>/`. Nothing is stored. A photo that is not there is refused before any
//! is read, since with `--sequence` it would change what every later photo is read against; one
//! that is there and will not decode is its own `decode_failed` line. Exit codes: 0 every photo
//! read, 1 any needing a pick or not read, 2 usage or a photo not there.

use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use analysis::flight::{LaunchConditions, DEFAULT_STEP_S};
use contracts::intent::ClubCategory;
use contracts::Timestamp;
use golf_core::reports::{data_dir, repo_root};
use golf_lab::analyze::{analyze_swing_dir, resolve_swing_dir};
use golf_lab::analyze_keypoints::{analyze_keypoints, print_keypoints_report};
use golf_lab::build_vectors::{build_vectors, Request as BuildRequest};
use golf_lab::coach::{coaching_enabled, Entry};
use golf_lab::ffmpeg::Ffmpeg;
use golf_lab::flight::{flight, Ended, Mode, Request as FlightRequest};
use golf_lab::import::{
    import_photos, import_video, shot_screens_dir, spelled, IntoSession, Request,
};
use golf_lab::import_export::{import_export, Request as ExportRequest};
use golf_lab::pipeline::{PipelineOptions, Tools, VIEWS};
use golf_lab::pose::{keypoints_for, Sidecar};
use golf_lab::read_impact::{is_clip, read_impact_clips, Request as ImpactRequest};
use golf_lab::read_tape::{missing_photo, read_tape_photos, Request as TapeRequest};
use golf_lab::reanalyze::{reanalyze, Request as ReanalyzeRequest};
use golf_lab::report::{print_report, print_swings};
use golf_lab::shot::{OcrReader, ShotOptions};
use storage::manifest::{load_manifest, manifest_path, Role};

const USAGE: &str = "usage: golf-lab import-photos [PATHS...] [--session S] [--out DIR] \
                     [--device D] [--min-confidence X] [--force] [--dry-run] [--models DIR]\n       \
                     golf-lab import-photos [PATHS...] --into-session S --player P --club C \
                     [--sessions-dir D] [--golfers-dir D] [--out DIR] [--device D] \
                     [--min-confidence X] [--force] [--dry-run] [--models DIR]\n       \
                     golf-lab import-video CLIP... [--session S] [--out DIR] [--device D] \
                     [--min-confidence X] [--force] [--dry-run] [--models DIR]\n       \
                     golf-lab import-video CLIP... --into-session S --player P --club C \
                     [--sessions-dir D] [--golfers-dir D] [--out DIR] [--device D] \
                     [--min-confidence X] [--force] [--dry-run] [--models DIR]\n       \
                     golf-lab analyze SWING [--sessions-dir D] [--shots-dir D] [--golfers-dir D] \
                     [--window-face-on S:E] [--window-dtl S:E] [--no-auto-window] [--list-swings] \
                     [--no-video] [--no-coaching] [--force-pose] [--force-ocr] [--skip-ocr] \
                     [--club C] [--min-confidence X] [--models DIR]\n       \
                     golf-lab reanalyze [SWINGS...] [--sessions-dir D] [--shots-dir D] \
                     [--golfers-dir D] [--player P] [--all] [--dry-run] [--coaching] [--verbose] \
                     [--models DIR]\n       \
                     golf-lab analyze-keypoints KEYPOINTS\n       \
                     golf-lab flight --ball-speed MPH --launch-angle DEG --spin RPM \
                     [--launch-direction DEG] [--spin-axis DEG] [--altitude M] [--step S] \
                     [--points N] [--verbose]\n       \
                     golf-lab flight (--gate | --shots | --shot SHOT_ID) [--altitude M] \
                     [--step S] [--points N] [--verbose] [--shots-dir D] [--sessions-dir D] \
                     [--golfers-dir D]\n       \
                     golf-lab build-vectors [--sessions-dir D] [--golfers-dir D] [--out DIR] \
                     [--dry-run]\n       \
                     golf-lab import-export EXPORT [--sessions-dir D] [--shots-dir D] \
                     [--golfers-dir D] [--dry-run]\n       \
                     golf-lab read-impact CLIP... [--json] [--review DIR]
                            golf-lab read-tape PHOTO... --label T [--sequence] [--review DIR]";

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("--help" | "-h" | "help") if args.next().is_none() => {
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
        Some("import-photos") => match parse_import("import-photos", args) {
            Ok(Some((request, models))) => import(&request, models),
            Ok(None) => {
                println!("{USAGE}");
                ExitCode::SUCCESS
            }
            Err(problem) => {
                usage(&problem);
                ExitCode::from(2)
            }
        },
        Some("import-video") => match parse_import("import-video", args) {
            Ok(Some((request, models))) => run_import_video(&request, models),
            Ok(None) => {
                println!("{USAGE}");
                ExitCode::SUCCESS
            }
            Err(problem) => {
                usage(&problem);
                ExitCode::from(2)
            }
        },
        Some("analyze") => match parse_analyze(args) {
            Ok(Some(request)) => analyze(request),
            Ok(None) => {
                println!("{USAGE}");
                ExitCode::SUCCESS
            }
            Err(problem) => {
                usage(&problem);
                ExitCode::from(2)
            }
        },
        Some("reanalyze") => match parse_reanalyze(args) {
            Ok(Some((request, models))) => run_reanalyze(&request, models),
            Ok(None) => {
                println!("{USAGE}");
                ExitCode::SUCCESS
            }
            Err(problem) => {
                usage(&problem);
                ExitCode::from(2)
            }
        },
        Some("analyze-keypoints") => match parse_keypoints(args) {
            Ok(Some(path)) => run_keypoints(&path),
            Ok(None) => {
                println!("{USAGE}");
                ExitCode::SUCCESS
            }
            Err(problem) => {
                usage(&problem);
                ExitCode::from(2)
            }
        },
        Some("flight") => match parse_flight(args) {
            Ok(Some(request)) => run_flight(&request),
            Ok(None) => {
                println!("{USAGE}");
                ExitCode::SUCCESS
            }
            Err(problem) => {
                usage(&problem);
                ExitCode::from(2)
            }
        },
        Some("build-vectors") => match parse_build_vectors(args) {
            Ok(Some(request)) => run_build_vectors(&request),
            Ok(None) => {
                println!("{USAGE}");
                ExitCode::SUCCESS
            }
            Err(problem) => {
                usage(&problem);
                ExitCode::from(2)
            }
        },
        Some("import-export") => match parse_import_export(args) {
            Ok(Some(request)) => run_import_export(&request),
            Ok(None) => {
                println!("{USAGE}");
                ExitCode::SUCCESS
            }
            Err(problem) => {
                usage(&problem);
                ExitCode::from(2)
            }
        },
        Some("read-impact") => match parse_read_impact(args) {
            Ok(Some(request)) => run_read_impact(&request),
            Ok(None) => {
                println!("{USAGE}");
                ExitCode::SUCCESS
            }
            Err(problem) => {
                usage(&problem);
                ExitCode::from(2)
            }
        },
        Some("read-tape") => match parse_read_tape(args) {
            Ok(Some(request)) => run_read_tape(&request),
            Ok(None) => {
                println!("{USAGE}");
                ExitCode::SUCCESS
            }
            Err(problem) => {
                usage(&problem);
                ExitCode::from(2)
            }
        },
        Some(other) => usage(&format!("unknown command {other:?}")),
        None => usage("no command given"),
    }
}

fn usage(problem: &str) -> ExitCode {
    eprintln!("golf-lab: {problem}");
    eprintln!("{USAGE}");
    ExitCode::FAILURE
}

/// `--flag=value` as argparse reads it, for a long flag; anything else is one argument.
fn split_flag(arg: &str) -> (String, Option<String>) {
    match arg.split_once('=') {
        Some((flag, value)) if flag.starts_with("--") => {
            (flag.to_string(), Some(value.to_string()))
        }
        _ => (arg.to_string(), None),
    }
}

/// `import-photos`' flags, argparse's way as `golf-core`'s verbs read theirs, then the defaults
/// resolved against the process environment, for it or for `import-video`, which keeps them.
/// `Ok(None)` is `--help`.
fn parse_import(
    verb: &str,
    mut args: impl Iterator<Item = String>,
) -> Result<Option<(Request, Option<PathBuf>)>, String> {
    let mut paths = Vec::new();
    let (mut session, mut device, mut out, mut min_confidence) = (None, None, None, None);
    let (mut into, mut player, mut club) = (None, None, None);
    let (mut sessions, mut golfers, mut models) = (None, None, None);
    let (mut force, mut dry_run) = (false, false);
    while let Some(arg) = args.next() {
        let (flag, inline) = split_flag(&arg);
        let mut value = |flag: &str| {
            inline
                .clone()
                .or_else(|| args.next())
                .ok_or_else(|| format!("{verb}: argument {flag}: expected one argument"))
        };
        match flag.as_str() {
            "--session" => session = Some(value("--session")?),
            "--device" => device = Some(value("--device")?),
            "--out" => out = Some(PathBuf::from(value("--out")?)),
            "--min-confidence" => {
                let text = value("--min-confidence")?;
                min_confidence = Some(text.trim().parse::<f64>().map_err(|_| {
                    format!("{verb}: argument --min-confidence: invalid float value: '{text}'")
                })?);
            }
            "--into-session" => into = Some(value("--into-session")?),
            "--player" => player = Some(value("--player")?),
            "--club" => club = Some(value("--club")?),
            "--sessions-dir" => sessions = Some(PathBuf::from(value("--sessions-dir")?)),
            "--golfers-dir" => golfers = Some(PathBuf::from(value("--golfers-dir")?)),
            "--models" => models = Some(PathBuf::from(value("--models")?)),
            "--force" | "--dry-run" | "-h" | "--help" if inline.is_some() => {
                return Err(format!(
                    "{verb}: argument {flag}: ignored explicit argument {:?}",
                    inline.unwrap_or_default()
                ))
            }
            "--force" => force = true,
            "--dry-run" => dry_run = true,
            "-h" | "--help" => return Ok(None),
            other if other.starts_with('-') && other != "-" => {
                return Err(format!("{verb}: unrecognized arguments: {other}"))
            }
            _ => paths.push(PathBuf::from(arg)),
        }
    }

    let env = |var: &str| std::env::var(var).ok();
    let into = match (into, player, club) {
        (None, None, None) => {
            if sessions.is_some() || golfers.is_some() {
                return Err(format!(
                    "{verb}: error: --sessions-dir and --golfers-dir go with --into-session"
                ));
            }
            None
        }
        (Some(_), _, _) if session.is_some() => {
            return Err(format!(
                "{verb}: error: argument --session: not allowed with argument --into-session, \
                 which names the session the photos are filed into"
            ))
        }
        (Some(session_id), Some(player), Some(club)) => Some(IntoSession {
            session_id,
            player,
            club,
            sessions_dir: data_dir(sessions, "GOLF_SESSIONS_DIR", "sessions", &env),
            golfers_dir: data_dir(golfers, "GOLF_GOLFERS_DIR", "golfers", &env),
        }),
        _ => {
            return Err(format!(
                "{verb}: error: --into-session S, --player P and --club C go together"
            ))
        }
    };
    let mut options = ShotOptions::configured(data_dir(out, "GOLF_SHOTS_DIR", "shots", &env), env)
        .map_err(|problem| format!("{verb}: error: {problem}"))?;
    if let Some(device) = device {
        options.device = device;
    }
    if let Some(min_confidence) = min_confidence {
        options.min_confidence = min_confidence;
    }
    options.force_ocr = force;
    options.dry_run = dry_run;
    if paths.is_empty() {
        if verb == "import-video" {
            return Err(format!(
                "{verb}: the following arguments are required: CLIP"
            ));
        }
        paths.push(shot_screens_dir(env));
    }
    Ok(Some((
        Request {
            paths,
            session,
            options,
            into,
        },
        models,
    )))
}

/// Run `import-photos` with the clock read now and the reader over `models` (or the repository's).
fn import(request: &Request, models: Option<PathBuf>) -> ExitCode {
    let mut reader = models.map_or_else(OcrReader::configured, OcrReader::new);
    let (mut out, mut err) = (io::stdout(), io::stderr());
    match import_photos(
        request,
        &mut reader,
        Timestamp::now_utc(),
        &mut out,
        &mut err,
    ) {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("golf-lab import-photos: writing the report: {error}");
            ExitCode::FAILURE
        }
    }
}

/// Run `import-video`, as [`import`] runs `import-photos`, with ffmpeg found once. Its absence is
/// each clip's failure rather than the verb's, so a device refused for its declaration exits 2 on a
/// machine with no ffmpeg too.
fn run_import_video(request: &Request, models: Option<PathBuf>) -> ExitCode {
    let mut reader = models.map_or_else(OcrReader::configured, OcrReader::new);
    let (mut out, mut err) = (io::stdout(), io::stderr());
    let ffmpeg = Ffmpeg::find();
    let mut motion = |clip: &std::path::Path, width: Option<u32>| match &ffmpeg {
        Ok(ffmpeg) => ffmpeg.decode_frames(clip, width),
        Err(missing) => Err(missing.to_string()),
    };
    match import_video(
        request,
        &mut motion,
        &mut reader,
        Timestamp::now_utc(),
        &mut out,
        &mut err,
    ) {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("golf-lab import-video: writing the report: {error}");
            ExitCode::FAILURE
        }
    }
}

/// `analyze`'s request: the bundle named, the options it runs under, and the two things that are
/// not pipeline options.
struct AnalyzeRequest {
    swing: String,
    sessions_dir: PathBuf,
    options: PipelineOptions,
    list_swings: bool,
    models: Option<PathBuf>,
}

/// `analyze`'s switches, in the order [`parse_analyze`] destructures them.
const ANALYZE_SWITCHES: [&str; 7] = [
    "--no-auto-window",
    "--list-swings",
    "--no-video",
    "--no-coaching",
    "--force-pose",
    "--force-ocr",
    "--skip-ocr",
];

/// `analyze`'s flags, argparse's way, then the defaults resolved against the process environment.
/// `Ok(None)` is `--help`.
fn parse_analyze(mut args: impl Iterator<Item = String>) -> Result<Option<AnalyzeRequest>, String> {
    const VERB: &str = "analyze";
    let mut swing: Option<String> = None;
    let (mut sessions, mut shots, mut golfers, mut models) = (None, None, None, None);
    let (mut window_face_on, mut window_dtl) = (None, None);
    let (mut club, mut min_confidence) = (None, None);
    let mut switches = [false; ANALYZE_SWITCHES.len()];
    while let Some(arg) = args.next() {
        let (flag, inline) = split_flag(&arg);
        let mut value = |flag: &str| {
            inline
                .clone()
                .or_else(|| args.next())
                .ok_or_else(|| format!("{VERB}: argument {flag}: expected one argument"))
        };
        match flag.as_str() {
            "--sessions-dir" => sessions = Some(PathBuf::from(value("--sessions-dir")?)),
            "--shots-dir" => shots = Some(PathBuf::from(value("--shots-dir")?)),
            "--golfers-dir" => golfers = Some(PathBuf::from(value("--golfers-dir")?)),
            "--models" => models = Some(PathBuf::from(value("--models")?)),
            "--window-face-on" => {
                window_face_on = Some(window("--window-face-on", &value("--window-face-on")?)?)
            }
            "--window-dtl" => window_dtl = Some(window("--window-dtl", &value("--window-dtl")?)?),
            "--club" => {
                let text = value("--club")?;
                let named = serde_json::Value::String(text.clone());
                club = Some(serde_json::from_value::<ClubCategory>(named).map_err(|_| {
                    format!("{VERB}: argument --club: invalid ClubCategory value: '{text}'")
                })?);
            }
            "--min-confidence" => {
                let text = value("--min-confidence")?;
                min_confidence = Some(text.trim().parse::<f64>().map_err(|_| {
                    format!("{VERB}: argument --min-confidence: invalid float value: '{text}'")
                })?);
            }
            "--tau" => {
                value("--tau")?;
                return Err(format!(
                    "{VERB}: argument --tau: golf-lab renders no side-by-side video (the M29 \
                     plan's C4), so there is no render range to set"
                ));
            }
            switch
                if ANALYZE_SWITCHES.contains(&switch) || switch == "-h" || switch == "--help" =>
            {
                if let Some(inline) = inline {
                    return Err(format!(
                        "{VERB}: argument {switch}: ignored explicit argument {inline:?}"
                    ));
                }
                match ANALYZE_SWITCHES.iter().position(|known| *known == switch) {
                    Some(index) => switches[index] = true,
                    None => return Ok(None),
                }
            }
            other if other.starts_with('-') && other != "-" => {
                return Err(format!("{VERB}: unrecognized arguments: {other}"))
            }
            _ if swing.is_none() => swing = Some(arg),
            _ => return Err(format!("{VERB}: unrecognized arguments: {arg}")),
        }
    }
    let Some(swing) = swing else {
        return Err(format!(
            "{VERB}: error: the following arguments are required: swing"
        ));
    };
    // `--no-video` is read and dropped: no run renders (C4).
    let [no_auto_window, list_swings, _no_video, no_coaching, force_pose, force_ocr, skip_ocr] =
        switches;

    let env = |var: &str| std::env::var(var).ok();
    let mut shot = ShotOptions::configured(data_dir(shots, "GOLF_SHOTS_DIR", "shots", &env), env)
        .map_err(|problem| format!("{VERB}: error: {problem}"))?;
    if let Some(min_confidence) = min_confidence {
        shot.min_confidence = min_confidence;
    }
    shot.force_ocr = force_ocr;
    shot.skip_ocr = skip_ocr;
    let coaching = coaching_enabled(env).map_err(|problem| format!("{VERB}: error: {problem}"))?;
    let mut options =
        PipelineOptions::new(shot, data_dir(golfers, "GOLF_GOLFERS_DIR", "golfers", &env));
    options.club = club.unwrap_or(ClubCategory::All);
    options.window_face_on = window_face_on;
    options.window_down_the_line = window_dtl;
    options.auto_window = !no_auto_window;
    options.force_pose = force_pose;
    options.coaching = coaching && !no_coaching;
    Ok(Some(AnalyzeRequest {
        swing,
        sessions_dir: data_dir(sessions, "GOLF_SESSIONS_DIR", "sessions", &env),
        options,
        list_swings,
        models,
    }))
}

/// `_parse_window`: `676:760` is `(676, 760)`, each half read as Python's `int` reads it, with
/// surrounding whitespace and a sign allowed.
fn window(flag: &str, text: &str) -> Result<(i64, i64), String> {
    let (start, end) = text.split_once(':').unwrap_or((text, ""));
    match (start.trim().parse(), end.trim().parse()) {
        (Ok(start), Ok(end)) => Ok((start, end)),
        _ => Err(format!(
            "analyze: argument {flag}: window must look like START:END, got {}",
            pyfmt::str_repr(text)
        )),
    }
}

/// Run `analyze`: resolve the bundle, then list its swings or analyse it, and print what the script
/// prints, on the clock read now and the machine's instruments.
fn analyze(request: AnalyzeRequest) -> ExitCode {
    let Some(swing_dir) = resolve_swing_dir(&request.swing, &request.sessions_dir) else {
        eprintln!(
            "error: no swing bundle at {} (looked in {})",
            pyfmt::str_repr(&request.swing),
            spelled(&request.sessions_dir).display()
        );
        return ExitCode::from(2);
    };
    let mut sidecar = Sidecar::configured();
    let mut log = |line: &str| println!("{line}");

    if request.list_swings {
        return list_swings(&swing_dir, request.options.force_pose, &mut sidecar);
    }

    let ffmpeg = Ffmpeg::find();
    let mut reader = request
        .models
        .map_or_else(OcrReader::configured, OcrReader::new);
    let mut coach = Entry::configured();
    let mut clock = Timestamp::now_utc;
    let mut tools = Tools {
        ffmpeg: ffmpeg.as_ref(),
        sidecar: &mut sidecar,
        reader: &mut reader,
    };
    let outcome = match analyze_swing_dir(
        &swing_dir,
        &request.options,
        &mut tools,
        &mut coach,
        &mut clock,
        &mut log,
    ) {
        Ok(outcome) => outcome,
        Err(error) => {
            eprintln!("error: {error}");
            return ExitCode::from(2);
        }
    };
    let Some(result) = &outcome.result else {
        eprintln!("error: {}", outcome.error.as_deref().unwrap_or_default());
        return ExitCode::from(2);
    };
    let mut out = io::stdout().lock();
    if let Err(error) = print_report(result, &mut out).and_then(|()| writeln!(out)) {
        eprintln!("golf-lab analyze: writing the report: {error}");
        return ExitCode::FAILURE;
    }
    ExitCode::from(if outcome.flagged { 1 } else { 0 })
}

/// `_list_swings`: the candidate descents in each view, then stop. A view is posed only on a cache
/// miss, as the script's `keypoints_for` call poses it, and its notes are dropped, as the script
/// passes none.
fn list_swings(swing_dir: &std::path::Path, force_pose: bool, sidecar: &mut Sidecar) -> ExitCode {
    let Some(manifest) = load_manifest(&manifest_path(swing_dir)) else {
        eprintln!(
            "error: {} has no readable manifest",
            spelled(swing_dir).display()
        );
        return ExitCode::from(2);
    };
    let mut log = |line: &str| println!("{line}");
    for (role, label) in VIEWS {
        let keypoints = match keypoints_for(
            swing_dir,
            &manifest,
            role,
            force_pose,
            sidecar,
            &mut log,
            &mut Vec::new(),
        ) {
            Ok(keypoints) => keypoints,
            Err(error) => {
                eprintln!("error: {error}");
                return ExitCode::from(2);
            }
        };
        let Some(keypoints) = keypoints else {
            continue;
        };
        let flag = if role == Role::FaceOn {
            "--window-face-on"
        } else {
            "--window-dtl"
        };
        if let Err(error) = print_swings(label, flag, &keypoints, &mut io::stdout().lock()) {
            eprintln!("golf-lab analyze: writing the listing: {error}");
            return ExitCode::FAILURE;
        }
    }
    println!();
    ExitCode::SUCCESS
}

/// `reanalyze`'s switches, in the order [`parse_reanalyze`] destructures them.
const REANALYZE_SWITCHES: [&str; 4] = ["--all", "--dry-run", "--coaching", "--verbose"];

/// `reanalyze`'s flags, argparse's way, then the defaults resolved against the process environment.
/// `Ok(None)` is `--help`.
fn parse_reanalyze(
    mut args: impl Iterator<Item = String>,
) -> Result<Option<(ReanalyzeRequest, Option<PathBuf>)>, String> {
    const VERB: &str = "reanalyze";
    let mut refs = Vec::new();
    let (mut sessions, mut shots, mut golfers, mut models, mut player) =
        (None, None, None, None, None);
    let mut switches = [false; REANALYZE_SWITCHES.len()];
    while let Some(arg) = args.next() {
        let (flag, inline) = split_flag(&arg);
        let mut value = |flag: &str| {
            inline
                .clone()
                .or_else(|| args.next())
                .ok_or_else(|| format!("{VERB}: argument {flag}: expected one argument"))
        };
        match flag.as_str() {
            "--sessions-dir" => sessions = Some(PathBuf::from(value("--sessions-dir")?)),
            "--shots-dir" => shots = Some(PathBuf::from(value("--shots-dir")?)),
            "--golfers-dir" => golfers = Some(PathBuf::from(value("--golfers-dir")?)),
            "--models" => models = Some(PathBuf::from(value("--models")?)),
            "--player" => player = Some(value("--player")?),
            "--video" => {
                return Err(format!(
                    "{VERB}: argument --video: golf-lab renders no side-by-side video (the M29 \
                     plan's C4), so there is no aligned.mp4 to re-render"
                ))
            }
            switch
                if REANALYZE_SWITCHES.contains(&switch) || switch == "-h" || switch == "--help" =>
            {
                if let Some(inline) = inline {
                    return Err(format!(
                        "{VERB}: argument {switch}: ignored explicit argument {inline:?}"
                    ));
                }
                match REANALYZE_SWITCHES.iter().position(|known| *known == switch) {
                    Some(index) => switches[index] = true,
                    None => return Ok(None),
                }
            }
            other if other.starts_with('-') && other != "-" => {
                return Err(format!("{VERB}: unrecognized arguments: {other}"))
            }
            _ => refs.push(arg),
        }
    }
    let [everything, dry_run, coaching, verbose] = switches;

    let env = |var: &str| std::env::var(var).ok();
    let shot = ShotOptions::configured(data_dir(shots, "GOLF_SHOTS_DIR", "shots", &env), env)
        .map_err(|problem| format!("{VERB}: error: {problem}"))?;
    let enabled = coaching_enabled(env).map_err(|problem| format!("{VERB}: error: {problem}"))?;
    let mut options =
        PipelineOptions::new(shot, data_dir(golfers, "GOLF_GOLFERS_DIR", "golfers", &env));
    options.coaching = enabled && coaching;
    Ok(Some((
        ReanalyzeRequest {
            refs,
            sessions_dir: data_dir(sessions, "GOLF_SESSIONS_DIR", "sessions", &env),
            player,
            everything,
            dry_run,
            verbose,
            options,
        },
        models,
    )))
}

/// Run `reanalyze` on the clock read now and the machine's instruments, built once for the walk.
fn run_reanalyze(request: &ReanalyzeRequest, models: Option<PathBuf>) -> ExitCode {
    let ffmpeg = Ffmpeg::find();
    let mut sidecar = Sidecar::configured();
    let mut reader = models.map_or_else(OcrReader::configured, OcrReader::new);
    let mut coach = Entry::configured();
    let mut clock = Timestamp::now_utc;
    let mut tools = Tools {
        ffmpeg: ffmpeg.as_ref(),
        sidecar: &mut sidecar,
        reader: &mut reader,
    };
    let (mut out, mut err) = (io::stdout().lock(), io::stderr());
    match reanalyze(
        request, &mut tools, &mut coach, &mut clock, &mut out, &mut err,
    ) {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("golf-lab reanalyze: writing the report: {error}");
            ExitCode::FAILURE
        }
    }
}

/// `analyze-keypoints`' one path. `Ok(None)` is `--help`.
fn parse_keypoints(args: impl Iterator<Item = String>) -> Result<Option<PathBuf>, String> {
    const VERB: &str = "analyze-keypoints";
    let mut path = None;
    for arg in args {
        let (flag, _) = split_flag(&arg);
        match flag.as_str() {
            "-h" | "--help" => return Ok(None),
            "--overlay" => {
                return Err(format!(
                    "{VERB}: argument --overlay: golf-lab renders no annotated clip (the M29 \
                     plan's C4), so there is no overlay to draw"
                ))
            }
            other if other.starts_with('-') && other != "-" => {
                return Err(format!("{VERB}: unrecognized arguments: {other}"))
            }
            _ if path.is_none() => path = Some(PathBuf::from(arg)),
            _ => return Err(format!("{VERB}: unrecognized arguments: {arg}")),
        }
    }
    path.map(Some)
        .ok_or_else(|| format!("{VERB}: error: the following arguments are required: keypoints"))
}

/// Run `analyze-keypoints`: the script's `not found`, then the engine and the report.
fn run_keypoints(path: &std::path::Path) -> ExitCode {
    if !path.exists() {
        eprintln!("error: {} not found", spelled(path).display());
        return ExitCode::FAILURE;
    }
    let result = match analyze_keypoints(path) {
        Ok(result) => result,
        Err(error) => {
            eprintln!("error: {}: {error}", spelled(path).display());
            return ExitCode::FAILURE;
        }
    };
    if let Err(error) = print_keypoints_report(&result, &mut io::stdout().lock()) {
        eprintln!("golf-lab analyze-keypoints: writing the report: {error}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

/// `flight`'s switches, in the order [`parse_flight`] destructures them.
const FLIGHT_SWITCHES: [&str; 3] = ["--gate", "--shots", "--verbose"];

/// `flight`'s flags, argparse's way, then the script's three mode checks in its order, then the
/// directories resolved against the process environment. `Ok(None)` is `--help`.
fn parse_flight(mut args: impl Iterator<Item = String>) -> Result<Option<FlightRequest>, String> {
    const VERB: &str = "flight";
    let float = |flag: &str, text: String| {
        text.trim()
            .parse::<f64>()
            .map_err(|_| format!("{VERB}: argument {flag}: invalid float value: '{text}'"))
    };
    let (mut ball_speed, mut launch_angle, mut spin) = (None, None, None);
    let (mut launch_direction, mut spin_axis, mut altitude) = (0.0, 0.0, 0.0);
    let (mut step, mut points, mut shot) = (DEFAULT_STEP_S, 0_i64, None);
    let (mut shots_dir, mut sessions, mut golfers) = (None, None, None);
    let mut switches = [false; FLIGHT_SWITCHES.len()];
    while let Some(arg) = args.next() {
        let (flag, inline) = split_flag(&arg);
        let mut value = |flag: &str| {
            inline
                .clone()
                .or_else(|| args.next())
                .ok_or_else(|| format!("{VERB}: argument {flag}: expected one argument"))
        };
        match flag.as_str() {
            "--ball-speed" => ball_speed = Some(float("--ball-speed", value("--ball-speed")?)?),
            "--launch-angle" => {
                launch_angle = Some(float("--launch-angle", value("--launch-angle")?)?)
            }
            "--spin" => spin = Some(float("--spin", value("--spin")?)?),
            "--launch-direction" => {
                launch_direction = float("--launch-direction", value("--launch-direction")?)?
            }
            "--spin-axis" => spin_axis = float("--spin-axis", value("--spin-axis")?)?,
            "--altitude" => altitude = float("--altitude", value("--altitude")?)?,
            "--step" => step = float("--step", value("--step")?)?,
            "--points" => {
                let text = value("--points")?;
                points = text.trim().parse::<i64>().map_err(|_| {
                    format!("{VERB}: argument --points: invalid int value: '{text}'")
                })?;
            }
            "--shot" => shot = Some(value("--shot")?),
            "--shots-dir" => shots_dir = Some(PathBuf::from(value("--shots-dir")?)),
            "--sessions-dir" => sessions = Some(PathBuf::from(value("--sessions-dir")?)),
            "--golfers-dir" => golfers = Some(PathBuf::from(value("--golfers-dir")?)),
            switch if FLIGHT_SWITCHES.contains(&switch) || switch == "-h" || switch == "--help" => {
                if let Some(inline) = inline {
                    return Err(format!(
                        "{VERB}: argument {switch}: ignored explicit argument {inline:?}"
                    ));
                }
                match FLIGHT_SWITCHES.iter().position(|known| *known == switch) {
                    Some(index) => switches[index] = true,
                    None => return Ok(None),
                }
            }
            other if other.starts_with('-') && other != "-" => {
                return Err(format!("{VERB}: unrecognized arguments: {other}"))
            }
            _ => return Err(format!("{VERB}: unrecognized arguments: {arg}")),
        }
    }
    let [gate, shots, verbose] = switches;

    // The script's three checks, in its order and its words: two modes is refused before none, and
    // a partial typed set only once it is the one mode asked for.
    let typed = [ball_speed, launch_angle, spin];
    let any_typed = typed.iter().any(Option::is_some);
    let modes = [gate, shots, shot.is_some(), any_typed]
        .iter()
        .filter(|asked| **asked)
        .count();
    if modes > 1 {
        return Err(format!(
            "{VERB}: error: typed conditions, --gate, --shots and --shot are four modes; pick one. \
             The stored modes fly what the screen recorded and the typed one flies what you ask \
             for, and silently merging them is how an inferred spin would end up wearing a \
             measured label"
        ));
    }
    if modes == 0 {
        return Err(format!(
            "{VERB}: error: --ball-speed, --launch-angle and --spin are all required (or --gate, \
             --shots, --shot)"
        ));
    }
    let mode = match (typed, shot) {
        ([Some(ball_speed_mph), Some(launch_angle_deg), Some(spin_rpm)], _) => {
            Mode::Typed(LaunchConditions {
                ball_speed_mph,
                launch_angle_deg,
                spin_rpm,
                launch_direction_deg: launch_direction,
                spin_axis_deg: spin_axis,
            })
        }
        _ if any_typed => {
            return Err(format!(
                "{VERB}: error: --ball-speed, --launch-angle and --spin are required together"
            ))
        }
        (_, Some(shot)) => Mode::Shot(shot),
        _ if shots => Mode::Shots,
        _ => Mode::Gate,
    };

    let env = |var: &str| std::env::var(var).ok();
    Ok(Some(FlightRequest {
        mode,
        altitude_m: altitude,
        step_s: step,
        points,
        verbose,
        shots_dir: data_dir(shots_dir, "GOLF_SHOTS_DIR", "shots", &env),
        sessions_dir: data_dir(sessions, "GOLF_SESSIONS_DIR", "sessions", &env),
        golfers_dir: data_dir(golfers, "GOLF_GOLFERS_DIR", "golfers", &env),
    }))
}

/// Run `flight` and print what the script prints. A `--shot` nothing carries is argparse's error,
/// after the header, as the script's is.
fn run_flight(request: &FlightRequest) -> ExitCode {
    let mut out = io::stdout().lock();
    match flight(request, &mut out) {
        Ok(Ended::Exit(code)) => ExitCode::from(code),
        Ok(Ended::Usage(problem)) => {
            drop(out);
            usage(&problem);
            ExitCode::from(2)
        }
        Err(error) => {
            eprintln!("golf-lab flight: writing the report: {error}");
            ExitCode::FAILURE
        }
    }
}

/// `build-vectors`' flags, argparse's way, then the directories resolved against the process
/// environment and the vectors root against the repository. `Ok(None)` is `--help`.
fn parse_build_vectors(
    mut args: impl Iterator<Item = String>,
) -> Result<Option<BuildRequest>, String> {
    const VERB: &str = "build-vectors";
    let (mut sessions, mut golfers, mut out) = (None, None, None);
    let mut dry_run = false;
    while let Some(arg) = args.next() {
        let (flag, inline) = split_flag(&arg);
        let mut value = |flag: &str| {
            inline
                .clone()
                .or_else(|| args.next())
                .ok_or_else(|| format!("{VERB}: argument {flag}: expected one argument"))
        };
        match flag.as_str() {
            "--sessions-dir" => sessions = Some(PathBuf::from(value("--sessions-dir")?)),
            "--golfers-dir" => golfers = Some(PathBuf::from(value("--golfers-dir")?)),
            "--out" => out = Some(PathBuf::from(value("--out")?)),
            "--dry-run" | "-h" | "--help" if inline.is_some() => {
                return Err(format!(
                    "{VERB}: argument {flag}: ignored explicit argument {:?}",
                    inline.unwrap_or_default()
                ))
            }
            "--dry-run" => dry_run = true,
            "-h" | "--help" => return Ok(None),
            other if other.starts_with('-') && other != "-" => {
                return Err(format!("{VERB}: unrecognized arguments: {other}"))
            }
            _ => return Err(format!("{VERB}: unrecognized arguments: {arg}")),
        }
    }
    let env = |var: &str| std::env::var(var).ok();
    Ok(Some(BuildRequest {
        sessions_dir: data_dir(sessions, "GOLF_SESSIONS_DIR", "sessions", &env),
        golfers_dir: data_dir(golfers, "GOLF_GOLFERS_DIR", "golfers", &env),
        out: out.unwrap_or_else(|| repo_root().join("spec").join("vectors")),
        dry_run,
    }))
}

/// Run `build-vectors`: what was written to stdout, what was skipped or refused to stderr.
fn run_build_vectors(request: &BuildRequest) -> ExitCode {
    let (mut out, mut err) = (io::stdout().lock(), io::stderr());
    match build_vectors(request, &mut out, &mut err) {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("golf-lab build-vectors: writing the report: {error}");
            ExitCode::FAILURE
        }
    }
}

/// `import-export`'s flags, argparse's way, then the three stores resolved against the process
/// environment. `Ok(None)` is `--help`.
fn parse_import_export(
    mut args: impl Iterator<Item = String>,
) -> Result<Option<ExportRequest>, String> {
    const VERB: &str = "import-export";
    let (mut export, mut sessions, mut shots, mut golfers) = (None, None, None, None);
    let mut dry_run = false;
    while let Some(arg) = args.next() {
        let (flag, inline) = split_flag(&arg);
        let mut value = |flag: &str| {
            inline
                .clone()
                .or_else(|| args.next())
                .ok_or_else(|| format!("{VERB}: argument {flag}: expected one argument"))
        };
        match flag.as_str() {
            "--sessions-dir" => sessions = Some(PathBuf::from(value("--sessions-dir")?)),
            "--shots-dir" => shots = Some(PathBuf::from(value("--shots-dir")?)),
            "--golfers-dir" => golfers = Some(PathBuf::from(value("--golfers-dir")?)),
            "--dry-run" | "-h" | "--help" if inline.is_some() => {
                return Err(format!(
                    "{VERB}: argument {flag}: ignored explicit argument {:?}",
                    inline.unwrap_or_default()
                ))
            }
            "--dry-run" => dry_run = true,
            "-h" | "--help" => return Ok(None),
            other if other.starts_with('-') && other != "-" => {
                return Err(format!("{VERB}: unrecognized arguments: {other}"))
            }
            _ if export.is_none() => export = Some(PathBuf::from(&arg)),
            _ => return Err(format!("{VERB}: unrecognized arguments: {arg}")),
        }
    }
    let Some(export) = export else {
        return Err(format!(
            "{VERB}: the following arguments are required: EXPORT"
        ));
    };
    let env = |var: &str| std::env::var(var).ok();
    Ok(Some(ExportRequest {
        export,
        sessions_dir: data_dir(sessions, "GOLF_SESSIONS_DIR", "sessions", &env),
        shots_dir: data_dir(shots, "GOLF_SHOTS_DIR", "shots", &env),
        golfers_dir: data_dir(golfers, "GOLF_GOLFERS_DIR", "golfers", &env),
        dry_run,
    }))
}

/// Run `import-export`: the report to stdout, every refusal to stderr.
fn run_import_export(request: &ExportRequest) -> ExitCode {
    let (mut out, mut err) = (io::stdout().lock(), io::stderr());
    match import_export(request, &mut out, &mut err) {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("golf-lab import-export: writing the report: {error}");
            ExitCode::FAILURE
        }
    }
}

/// `read-impact`'s clips and flags, argparse's way. A still among the clips is refused here, naming
/// the verb that reads one, so nothing is decoded before the usage error. `Ok(None)` is `--help`.
fn parse_read_impact(
    mut args: impl Iterator<Item = String>,
) -> Result<Option<ImpactRequest>, String> {
    const VERB: &str = "read-impact";
    let (mut clips, mut json, mut review) = (Vec::new(), false, None);
    while let Some(arg) = args.next() {
        let (flag, inline) = split_flag(&arg);
        match flag.as_str() {
            "--review" => {
                review = Some(PathBuf::from(inline.or_else(|| args.next()).ok_or_else(
                    || format!("{VERB}: argument --review: expected one argument"),
                )?))
            }
            "--json" | "-h" | "--help" if inline.is_some() => {
                return Err(format!(
                    "{VERB}: argument {flag}: ignored explicit argument {:?}",
                    inline.unwrap_or_default()
                ))
            }
            "--json" => json = true,
            "-h" | "--help" => return Ok(None),
            other if other.starts_with('-') && other != "-" => {
                return Err(format!("{VERB}: unrecognized arguments: {other}"))
            }
            _ => {
                let clip = PathBuf::from(&arg);
                if !is_clip(&clip) {
                    return Err(format!(
                        "{VERB}: {arg} is not a Live Photo clip (.mov): a photo of the screen is \
                         read by golf-lab import-photos"
                    ));
                }
                clips.push(clip);
            }
        }
    }
    if clips.is_empty() {
        return Err(format!(
            "{VERB}: the following arguments are required: CLIP"
        ));
    }
    Ok(Some(ImpactRequest {
        clips,
        json,
        review,
    }))
}

/// Run `read-impact`: ffmpeg found once, then one line a clip on stdout. No ffmpeg is a clip that
/// cannot be decoded, for every clip, so it exits 1 rather than 2.
fn run_read_impact(request: &ImpactRequest) -> ExitCode {
    let mut ffmpeg = match Ffmpeg::find() {
        Ok(ffmpeg) => ffmpeg,
        Err(missing) => {
            eprintln!("golf-lab read-impact: {missing}");
            return ExitCode::FAILURE;
        }
    };
    match read_impact_clips(request, &mut ffmpeg, &mut io::stdout().lock()) {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("golf-lab read-impact: writing the report or the review: {error}");
            ExitCode::FAILURE
        }
    }
}

/// `read-tape`'s photos and flags, argparse's way. `--label` is checked against the label types
/// here, so nothing is decoded before the usage error. `Ok(None)` is `--help`.
fn parse_read_tape(mut args: impl Iterator<Item = String>) -> Result<Option<TapeRequest>, String> {
    const VERB: &str = "read-tape";
    let (mut photos, mut sequence, mut label, mut review) = (Vec::new(), false, None, None);
    while let Some(arg) = args.next() {
        let (flag, inline) = split_flag(&arg);
        match flag.as_str() {
            "--label" => {
                let id = inline
                    .or_else(|| args.next())
                    .ok_or_else(|| format!("{VERB}: argument --label: expected one argument"))?;
                label = Some(impact::tape::label_type(&id).ok_or_else(|| {
                    let choices: Vec<String> = impact::tape::LABEL_TYPES
                        .iter()
                        .map(|label| format!("'{}'", label.id))
                        .collect();
                    format!(
                        "{VERB}: argument --label: invalid choice: '{id}' (choose from {})",
                        choices.join(", ")
                    )
                })?);
            }
            "--review" => {
                review = Some(PathBuf::from(inline.or_else(|| args.next()).ok_or_else(
                    || format!("{VERB}: argument --review: expected one argument"),
                )?))
            }
            "--sequence" | "-h" | "--help" if inline.is_some() => {
                return Err(format!(
                    "{VERB}: argument {flag}: ignored explicit argument {:?}",
                    inline.unwrap_or_default()
                ))
            }
            "--sequence" => sequence = true,
            "-h" | "--help" => return Ok(None),
            other if other.starts_with('-') && other != "-" => {
                return Err(format!("{VERB}: unrecognized arguments: {other}"))
            }
            _ => photos.push(PathBuf::from(&arg)),
        }
    }
    let missing: Vec<&str> = [(photos.is_empty(), "PHOTO"), (label.is_none(), "--label")]
        .into_iter()
        .filter_map(|(absent, name)| absent.then_some(name))
        .collect();
    let Some(label) = label.filter(|_| missing.is_empty()) else {
        return Err(format!(
            "{VERB}: the following arguments are required: {}",
            missing.join(", ")
        ));
    };
    Ok(Some(TapeRequest {
        photos,
        sequence,
        label,
        review,
    }))
}

/// Run `read-tape`: every photo checked to be there, then one line a photo on stdout.
fn run_read_tape(request: &TapeRequest) -> ExitCode {
    if let Some(photo) = missing_photo(request) {
        eprintln!(
            "golf-lab read-tape: {} not found, so no photo was read",
            spelled(photo).display()
        );
        return ExitCode::from(2);
    }
    match read_tape_photos(request, &mut io::stdout().lock()) {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("golf-lab read-tape: writing the report or the review: {error}");
            ExitCode::FAILURE
        }
    }
}
