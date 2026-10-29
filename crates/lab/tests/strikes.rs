//! Forced detection over every stored clip reproduces the strike cache frozen Python wrote.
//! [M29 P21]
//!
//! The plan's P21 gate. Each stored bundle's manifest and clips are copied into a scratch swing
//! directory, never touched in place, and each view goes through [`audio_for`] twice:
//!
//! 1. **unforced, over a copy of the stored `{role}.audio.json`**, which must be a cache hit that
//!    writes nothing and answers the stored file exactly, frames included, because the fps handed
//!    in is the one the stored frames were derived under (the view's `keypoints.json`, as
//!    `analyze_swing_dir` hands it in);
//! 2. **forced**, which decodes the clip through the ffmpeg [`Ffmpeg::find`] finds, probes its
//!    video start and detects, and must reproduce the stored file.
//!
//! **Which ffmpeg it gates is the lab's**: `GOLF_FFMPEG`, then `PATH` (the plan's D17), which on
//! this box is `PATH`'s 9.0.2. The stored files were written by frozen Python through `.venv`'s
//! 7.1, and the two do not decode the same waveform: 9.0.2 ends each audio track where its edit
//! list ends, and 7.1 decoded 16 to 976 samples past it on 26 of the 30 stored clips
//! (`ffmpeg.rs`'s module doc, and the plan's P21 findings for the measurement).
//!
//! So "reproduce" is held at two strengths, as the user chose when P21 stopped on this gate:
//!
//! - **Exactly**: every strike's sample, frame and rank (the list's order, with its length), the
//!   clip's sample rate, sha, stream, fps and video start, and the detector version. Nothing the
//!   engine reads is in the other list: `analyze_swing_dir` passes on `strike.frame` alone.
//! - **To a relative [`DECODER_RTOL`]**, named here because they are the three that move with the
//!   waveform's tail: `duration_s`, which is the decoded length, and each strike's `confidence` and
//!   `prominence`, which the detector measures against the whole clip's median flux (and
//!   `confidence` against its spread too), so a longer tail moves both. Under 9.0.2 the measured
//!   worst were 1.25e-3, 6.1e-4 and 2.5e-5, on 26 views, 55 strikes and 46 strikes.
//!
//! The bound is the decoder's, not the port's. Under 7.1 (`GOLF_FFMPEG` at `.venv`'s binary) this
//! gate passed 30 of 30 with every float inside the audio family's tolerance
//! (`crates/trigger/tests/conformance.rs`'s 1e-9), worst relative 2.1e-15. That run is recorded
//! once in the plan's P21 findings and is not part of the gate, because 7.1 leaves with the
//! `audio` extra in M40.
//!
//! It needs `data/processed/sessions/` (gitignored) and an ffmpeg, so it is ignored, and it refuses
//! to pass with nothing to read.

use std::fs;
use std::path::{Path, PathBuf};

use contracts::audio::AudioFile;
use golf_lab::audio::{audio_for, AudioOptions};
use golf_lab::ffmpeg::Ffmpeg;
use storage::audio_io::{audio_path, load_audio};
use storage::manifest::{load_manifest, manifest_path, Role};

/// The bound on `duration_s`, `confidence` and `prominence` under the lab's ffmpeg: the measured
/// worst over the 30 (1.25e-3, a `duration_s`) with headroom, and nothing looser.
const DECODER_RTOL: f64 = 2e-3;
/// The floor for a value of zero, the audio family's (`crates/trigger/tests/conformance.rs`).
const ATOL: f64 = 1e-12;

fn sessions_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/processed/sessions")
}

/// Every directory under `sessions` holding a manifest, sorted.
fn swing_dirs(sessions: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut sessions_seen: Vec<PathBuf> = fs::read_dir(sessions)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.is_dir())
        .collect();
    sessions_seen.sort();
    for session in sessions_seen {
        let mut swings: Vec<PathBuf> = fs::read_dir(&session)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| manifest_path(path).is_file())
            .collect();
        swings.sort();
        found.extend(swings);
    }
    found
}

/// `KeypointsFile.clip.fps`, the rate `analyze_swing_dir` hands `audio_for`.
fn pose_fps(swing_dir: &Path, role: Role) -> Option<f64> {
    let path = swing_dir.join(format!("{}.keypoints.json", role.as_str()));
    let keypoints: serde_json::Value = serde_json::from_slice(&fs::read(path).ok()?).ok()?;
    keypoints["clip"]["fps"].as_f64()
}

fn close(actual: f64, expected: f64) -> bool {
    (actual - expected).abs() <= ATOL + DECODER_RTOL * expected.abs()
}

fn relative(actual: f64, expected: f64) -> f64 {
    if expected == 0.0 {
        actual.abs()
    } else {
        ((actual - expected) / expected).abs()
    }
}

/// What differs between a re-detection and the stored file, worst float difference beside it.
fn compare(detected: &AudioFile, stored: &AudioFile, worst: &mut f64) -> Vec<String> {
    let mut diffs = Vec::new();
    let (Some(got), Some(want)) = (&detected.clip, &stored.clip) else {
        return vec![format!(
            "clip: {:?} against {:?}",
            detected.clip, stored.clip
        )];
    };
    let exact = [
        (
            "sample_rate",
            format!("{:?}", got.sample_rate),
            format!("{:?}", want.sample_rate),
        ),
        (
            "source_sha256",
            format!("{:?}", got.source_sha256),
            format!("{:?}", want.source_sha256),
        ),
        (
            "stream_index",
            format!("{:?}", got.stream_index),
            format!("{:?}", want.stream_index),
        ),
        ("fps", format!("{:?}", got.fps), format!("{:?}", want.fps)),
        (
            "video_start_s",
            format!("{:?}", got.video_start_s),
            format!("{:?}", want.video_start_s),
        ),
        (
            "detector_version",
            detected.detector_version.to_string(),
            stored.detector_version.to_string(),
        ),
        (
            "strike count",
            detected.strikes.len().to_string(),
            stored.strikes.len().to_string(),
        ),
    ];
    for (field, got, want) in exact {
        if got != want {
            diffs.push(format!("{field}: {got} against {want}"));
        }
    }
    let mut floats = vec![(
        "duration_s".to_string(),
        got.duration_s.unwrap_or(f64::NAN),
        want.duration_s.unwrap_or(f64::NAN),
    )];
    for (rank, (a, e)) in detected.strikes.iter().zip(&stored.strikes).enumerate() {
        if (a.sample, a.frame) != (e.sample, e.frame) {
            diffs.push(format!(
                "strike {rank}: sample {} frame {:?} against sample {} frame {:?}",
                a.sample, a.frame, e.sample, e.frame
            ));
        }
        floats.push((
            format!("strike {rank} confidence"),
            a.confidence,
            e.confidence,
        ));
        floats.push((
            format!("strike {rank} prominence"),
            a.prominence,
            e.prominence,
        ));
    }
    for (field, a, e) in floats {
        *worst = worst.max(relative(a, e));
        if !close(a, e) {
            diffs.push(format!(
                "{field}: {a:?} against {e:?} (relative {:.3e})",
                relative(a, e)
            ));
        }
    }
    diffs
}

#[test]
#[ignore = "needs data/processed/sessions/ (the stored bundles, gitignored) and an ffmpeg"]
fn forced_detection_over_every_stored_clip_reproduces_the_stored_strikes() {
    let sessions = sessions_dir();
    assert!(
        sessions.is_dir(),
        "{} is not a directory: this gate reads the stored bundles",
        sessions.display()
    );
    let ffmpeg = Ffmpeg::find().unwrap_or_else(|missing| panic!("{missing}"));
    let version = std::process::Command::new(ffmpeg.path())
        .arg("-version")
        .output()
        .map(|out| {
            String::from_utf8_lossy(&out.stdout)
                .lines()
                .next()
                .unwrap_or("")
                .to_string()
        })
        .unwrap_or_default();
    eprintln!("ffmpeg: {} ({version})", ffmpeg.path().display());

    let mut views = 0;
    let mut worst = 0.0f64;
    let mut failures = Vec::new();
    for swing_dir in swing_dirs(&sessions) {
        let label = swing_dir
            .strip_prefix(&sessions)
            .unwrap()
            .display()
            .to_string();
        let manifest = load_manifest(&manifest_path(&swing_dir)).expect("a stored manifest");
        let scratch = tempfile::tempdir().unwrap();
        fs::copy(manifest_path(&swing_dir), manifest_path(scratch.path())).unwrap();
        for role in [Role::FaceOn, Role::DownTheLine] {
            let Some(role_file) = manifest.roles.get(&role) else {
                continue;
            };
            let stored_path = audio_path(&swing_dir, role);
            let stored = load_audio(&stored_path)
                .unwrap_or_else(|e| panic!("{label} {}: {e}", role.as_str()));
            fs::copy(
                swing_dir.join(&role_file.filename),
                scratch.path().join(&role_file.filename),
            )
            .unwrap();
            let cache = audio_path(scratch.path(), role);
            fs::copy(&stored_path, &cache).unwrap();
            let fps = pose_fps(&swing_dir, role);
            let view = format!("{label} {}", role.as_str());
            views += 1;

            // 1. The cache hit: the stored file, exactly, and nothing written.
            let before = fs::read(&cache).unwrap();
            let mut log = Vec::new();
            let mut notes = Vec::new();
            let hit = audio_for(
                scratch.path(),
                &manifest,
                role,
                AudioOptions { fps, force: false },
                Ok(&ffmpeg),
                &mut |line: &str| log.push(line.to_string()),
                &mut notes,
            )
            .unwrap();
            if hit.as_ref() != Some(&stored) || fs::read(&cache).unwrap() != before {
                failures.push(format!("{view}: not a clean cache hit: {log:?}"));
            }
            if log
                != [format!(
                    "  {}: {} strikes from cache",
                    role.as_str(),
                    stored.strikes.len()
                )]
                || !notes.is_empty()
            {
                failures.push(format!("{view}: cache hit said {log:?} {notes:?}"));
            }

            // 2. Forced: decoded, probed and detected again.
            let mut log = Vec::new();
            let mut notes = Vec::new();
            let detected = audio_for(
                scratch.path(),
                &manifest,
                role,
                AudioOptions { fps, force: true },
                Ok(&ffmpeg),
                &mut |line: &str| log.push(line.to_string()),
                &mut notes,
            )
            .unwrap();
            let Some(detected) = detected else {
                failures.push(format!("{view}: nothing detected: {log:?} {notes:?}"));
                continue;
            };
            if !notes.is_empty() {
                failures.push(format!("{view}: notes {notes:?}"));
            }
            if load_audio(&cache).unwrap() != detected {
                failures.push(format!("{view}: the file written is not the one returned"));
            }
            let diffs = compare(&detected, &stored, &mut worst);
            if !diffs.is_empty() {
                failures.push(format!("{view}:\n    {}", diffs.join("\n    ")));
            }
            eprintln!("{view}: {}", log.join(" | "));
        }
    }
    assert!(views > 0, "no stored views under {}", sessions.display());
    eprintln!(
        "{views} views re-detected; worst relative float difference {worst:.3e} \
         (DECODER_RTOL {DECODER_RTOL:.0e})"
    );
    assert!(
        failures.is_empty(),
        "{} of {views} views:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
