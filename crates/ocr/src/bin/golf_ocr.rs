//! `golf-ocr` — one photo of the launch-monitor screen through the Rust reader. [M29 P7]
//!
//! ```text
//! golf-ocr read <photo> [--json | --boxes] [--device D] [--session S] [--min-confidence X]
//!               [--models DIR]
//! ```
//!
//! The photo is read by [`ocr::read_photo`] (decode, the screen found and rectified, the rotation
//! vote over the recognizer) and its shot parsed by [`screen::read`], the reader the phone calls
//! with VisionKit's boxes, so this is `scripts/import_shot_screens.py --dry-run` for one photo
//! with nothing stored. **It writes nothing.** The lab's verbs that store a read are `golf-lab`'s
//! (the M29 plan's P24), and this is the reader's own seam, as `golf-core parse-screen` is the
//! parser's.
//!
//! The shot is stamped as `import_screen` stamps one: `shot_id` is the session and the photo's
//! digest's first twelve hex digits, the timestamp is the photo's modification time
//! ([`ocr::screen_timestamp`]), and `image_path` is the path as given. The defaults are frozen
//! Python's: `--device` `hd_golf` and `--min-confidence` 0.6 are `config.py`'s
//! `launch_monitor_profile` and `ocr_min_confidence`, and `--session` is
//! `import_shot_screens.py`'s `screens-<today, UTC>`. The models are `data/models/ocr/` under the
//! repository, found from the source as `golf-core`'s data directories are, unless `--models`
//! names another.
//!
//! `--json` prints the `ShotData` [`screen::read`] makes, or `null` for a read of nothing (exit 0,
//! as `parse-screen` answers a failed read); without it, a short report. A photo that cannot be
//! decoded, a missing or mismatched model and an unknown device each exit 1 with the cause's own
//! message; a malformed command line exits 2.
//!
//! `--boxes` prints what the parser would have been given instead of what it made: `{"device",
//! "boxes", "notes"}` as pretty JSON, the prepared screen's boxes (each with its glyph colour,
//! `rgb`) and notes, in the shape a screen vector's `input` holds them, so a read can be pasted
//! into a hand-worked vector and its boxes looked at by eye (M42 P2). The device still matters
//! there: the rotation vote scores each turn by the device's labels. No shot is parsed, so
//! `--session` and `--min-confidence` change nothing, and `--json` with it is refused, as two
//! answers to what is printed.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use contracts::shot::ShotData;
use contracts::Timestamp;
use ocr::models::Models;
use ocr::PreparedScreen;
use screen::profile::load_profile;
use screen::{ScreenInput, TextBox};
use serde::Serialize;
use sha2::{Digest, Sha256};

const USAGE: &str = "usage: golf-ocr read <photo> [--json | --boxes] [--device D] [--session S] \
                     [--min-confidence X] [--models DIR]";

/// `config.py`'s `launch_monitor_profile` default.
const DEFAULT_DEVICE: &str = "hd_golf";
/// `config.py`'s `ocr_min_confidence` default: below it a read that passed every check is still
/// flagged for review.
const DEFAULT_MIN_CONFIDENCE: f64 = 0.6;

struct Request {
    photo: PathBuf,
    json: bool,
    boxes: bool,
    device: String,
    session: Option<String>,
    min_confidence: f64,
    models: PathBuf,
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("read") => {}
        Some("--help" | "-h") | None => {
            println!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        Some(other) => return usage(&format!("unknown command {other:?}")),
    }
    let request = match parse(&args[1..]) {
        Ok(request) => request,
        Err(problem) => return usage(&problem),
    };
    match read(&request) {
        Ok(()) => ExitCode::SUCCESS,
        Err(problem) => {
            eprintln!("golf-ocr read: {problem}");
            ExitCode::FAILURE
        }
    }
}

fn usage(problem: &str) -> ExitCode {
    eprintln!("golf-ocr: {problem}\n{USAGE}");
    ExitCode::from(2)
}

fn parse(args: &[String]) -> Result<Request, String> {
    let mut photo = None;
    let mut request = Request {
        photo: PathBuf::new(),
        json: false,
        boxes: false,
        device: DEFAULT_DEVICE.to_string(),
        session: None,
        min_confidence: DEFAULT_MIN_CONFIDENCE,
        models: repo_root().join(ocr::DEFAULT_DIR),
    };
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        let mut value = |flag: &str| {
            args.next()
                .cloned()
                .ok_or_else(|| format!("{flag} needs a value"))
        };
        match arg.as_str() {
            "--json" => request.json = true,
            "--boxes" => request.boxes = true,
            "--device" => request.device = value("--device")?,
            "--session" => request.session = Some(value("--session")?),
            "--min-confidence" => {
                let text = value("--min-confidence")?;
                request.min_confidence = text
                    .parse()
                    .map_err(|_| format!("--min-confidence {text:?} is not a number"))?;
            }
            "--models" => request.models = PathBuf::from(value("--models")?),
            flag if flag.starts_with("--") => return Err(format!("unknown flag {flag}")),
            path if photo.is_none() => photo = Some(PathBuf::from(path)),
            extra => return Err(format!("one photo at a time, and {extra:?} is a second")),
        }
    }
    request.photo = photo.ok_or("read needs a photo")?;
    if request.json && request.boxes {
        return Err("--json prints the shot and --boxes what it was read from; give one".into());
    }
    Ok(request)
}

/// The repository, found from this crate's source as `golf-core`'s `repo_root` finds it, so the
/// models are found from any working directory.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crates/ocr sits two directories below the repository root")
        .to_path_buf()
}

/// What `--boxes` prints: a screen vector's `input` up to its identity keys, in that order. A
/// struct rather than `json!`, whose map (no `preserve_order` in this workspace) sorts the keys.
#[derive(Serialize)]
struct ReadFrom<'a> {
    device: &'a str,
    boxes: &'a [TextBox],
    notes: &'a [String],
}

fn read(request: &Request) -> Result<(), String> {
    let profile = load_profile(&request.device).map_err(|unknown| unknown.to_string())?;
    let bytes = std::fs::read(&request.photo)
        .map_err(|e| format!("could not read image {}: {e}", request.photo.display()))?;
    let mut models = Models::open(&request.models).map_err(|e| e.to_string())?;
    let prepared =
        ocr::read_photo(&mut models, &request.photo, profile).map_err(|e| e.to_string())?;
    if request.boxes {
        let read_from = ReadFrom {
            device: &profile.device,
            boxes: &prepared.boxes,
            notes: &prepared.notes,
        };
        let text = serde_json::to_string_pretty(&read_from).map_err(|e| e.to_string())?;
        println!("{text}");
        return Ok(());
    }

    // `hash_image`, the shot store's key (`crates/storage`'s `shot_store::hash_image`): SHA-256 in
    // lowercase hex. Computed here rather than through `storage`, an edge this crate does not need.
    let digest = format!("{:x}", Sha256::digest(&bytes));
    let timestamp = ocr::screen_timestamp(&request.photo)
        .map_err(|e| format!("{}: {e}", request.photo.display()))?;

    let session_id = request
        .session
        .clone()
        .unwrap_or_else(|| format!("screens-{}", Timestamp::now_utc().date_ymd()));
    let input = ScreenInput {
        device: profile.device.clone(),
        boxes: prepared.boxes.clone(),
        notes: prepared.notes.clone(),
        shot_id: format!("{session_id}-{}", &digest[..12]),
        session_id,
        timestamp,
        image_sha256: Some(digest),
        image_path: Some(request.photo.display().to_string()),
        min_confidence: request.min_confidence,
    };
    let shot = screen::read(&input).map_err(|unknown| unknown.to_string())?;
    if request.json {
        let text = serde_json::to_string_pretty(&shot).map_err(|e| e.to_string())?;
        println!("{text}");
    } else {
        print!("{}", report(&request.photo, &prepared, shot.as_ref()));
    }
    Ok(())
}

/// The short report: how the photo was read, then the shot's values and its review state.
fn report(photo: &Path, prepared: &PreparedScreen, shot: Option<&ShotData>) -> String {
    let mut out = format!("{}\n", photo.display());
    let cut = match &prepared.quad {
        Some(_) => "the screen found and rectified",
        None => "read uncropped",
    };
    out += &format!(
        "  {cut}, turned {} degrees, {} boxes, {} of the labels legible\n",
        prepared.rotation.degrees(),
        prepared.boxes.len(),
        pyfmt::percent(prepared.label_ratio, 0)
    );
    let Some(shot) = shot else {
        out += "  nothing read: no value on this screen parsed\n";
        return out;
    };
    let record = serde_json::to_value(shot).expect("a ShotData serializes");
    let fields = record.as_object().expect("a ShotData is an object");
    for (key, value) in fields {
        if value.is_null() || is_identity(key) {
            continue;
        }
        out += &format!("  {key:<20} {value}\n");
    }
    if let Some(provenance) = &shot.provenance {
        out += &format!(
            "  confidence {}{}\n",
            pyfmt::fixed(provenance.parse_confidence, 3),
            if provenance.needs_review {
                ", NEEDS REVIEW"
            } else {
                ""
            }
        );
        for warning in &provenance.warnings {
            out += &format!("    - {warning}\n");
        }
    }
    out
}

fn is_identity(key: &str) -> bool {
    matches!(
        key,
        "shot_id" | "session_id" | "timestamp" | "source" | "provenance"
    )
}
