//! The pose cache over every stored bundle, and one forced run of the real worker. [M29 P22]
//!
//! Frozen Python posed every stored clip and wrote every `{role}.keypoints.json`
//! (`api/pipeline.py::keypoints_for`), and the two languages share those files until M40 (ADR-035
//! clause 4). So the lab's cache rule must read each as current, and its sidecar must pose a clip
//! into the file Python's would have written. Two gates:
//!
//! 1. **Every stored view is a cache hit, and nothing is spawned.** Each bundle's manifest and
//!    keypoints are copied into a scratch directory beside an empty stand-in for each clip, so a
//!    view the rule failed to read as current would reach the sidecar (and fail on the stand-in)
//!    rather than stop quietly at "the clip is missing". The sidecar is the lab's own,
//!    [`Sidecar::configured`], and [`Sidecar::started`] must still be `false` at the end. Each hit
//!    must return the stored file, say Python's one log line, add no note and write nothing. Each
//!    bundle's stored `analysis.json` must carry [`band_estimator_note`]'s sentence for that
//!    estimator, which is what ties the ported note to the artifacts Python wrote.
//! 2. **One forced run on the smallest stored clip**, in a scratch copy, through the real worker
//!    (`python -m golf_coach.pose.worker` under `.venv`, MediaPipe `heavy`). The frame count, the
//!    clip block, the estimator and each frame's landmark count must equal the stored file's, and
//!    the run must say Python's log lines with no note. **The landmark differences are recorded,
//!    not bounded** (the M29 plan's P22 Done line): the largest coordinate difference and how many
//!    frames differ at all are printed, with whether the written file equals the stored one as
//!    JSON values, and the plan's P22 finding carries the numbers.
//!
//! Both need `data/processed/sessions/`, which is gitignored, and the second needs the `vision`
//! extra and `data/models/pose_landmarker_heavy.task`, so both are ignored. Neither passes with
//! nothing to read: a missing directory or a bundle-less one fails, and a machine that cannot pose
//! fails the second on the note it gets instead of landmarks.

use std::fs;
use std::path::{Path, PathBuf};

use contracts::keypoints::KeypointsFile;
use golf_lab::pose::{band_estimator_note, keypoints_for, Sidecar};
use serde_json::Value;
use storage::keypoints_io::{keypoints_path, load_keypoints};
use storage::manifest::{load_manifest, manifest_path, Role, SwingManifest};

const ROLES: [Role; 2] = [Role::FaceOn, Role::DownTheLine];

fn sessions_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/processed/sessions")
}

/// Every directory under `sessions` holding a manifest, sorted.
fn swing_dirs(sessions: &Path) -> Vec<PathBuf> {
    assert!(
        sessions.is_dir(),
        "{} is not a directory: this gate reads the stored bundles",
        sessions.display()
    );
    let mut found = Vec::new();
    let mut session_dirs: Vec<PathBuf> = fs::read_dir(sessions)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.is_dir())
        .collect();
    session_dirs.sort();
    for session in session_dirs {
        let mut swings: Vec<PathBuf> = fs::read_dir(&session)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| manifest_path(path).is_file())
            .collect();
        swings.sort();
        found.extend(swings);
    }
    assert!(!found.is_empty(), "no bundle under {}", sessions.display());
    found
}

fn manifest_of(swing_dir: &Path) -> SwingManifest {
    load_manifest(&manifest_path(swing_dir))
        .unwrap_or_else(|| panic!("{}: a stored manifest", swing_dir.display()))
}

#[test]
#[ignore = "needs data/processed/sessions/, the stored bundles (gitignored)"]
fn every_stored_view_is_a_cache_hit_and_nothing_is_spawned() {
    let sessions = sessions_dir();
    let mut sidecar = Sidecar::configured();
    let estimator = sidecar.estimator();
    let band_note = band_estimator_note(&estimator);
    eprintln!("configured estimator: {estimator}; band note: {band_note:?}");

    let mut views = 0;
    let mut failures = Vec::new();
    for swing_dir in swing_dirs(&sessions) {
        let label = swing_dir
            .strip_prefix(&sessions)
            .unwrap()
            .display()
            .to_string();
        let manifest = manifest_of(&swing_dir);
        let scratch = tempfile::tempdir().unwrap();
        fs::copy(manifest_path(&swing_dir), manifest_path(scratch.path())).unwrap();
        for role in ROLES {
            let Some(role_file) = manifest.roles.get(&role) else {
                continue;
            };
            let view = format!("{label} {}", role.as_str());
            views += 1;
            let stored_path = keypoints_path(&swing_dir, role);
            let cache = keypoints_path(scratch.path(), role);
            fs::copy(&stored_path, &cache).unwrap();
            // An empty stand-in, so a miss reaches the sidecar instead of the missing-clip branch.
            fs::write(scratch.path().join(&role_file.filename), b"").unwrap();
            let stored = load_keypoints(&stored_path).unwrap();
            let before = fs::read(&cache).unwrap();

            let mut log = Vec::new();
            let mut notes = Vec::new();
            let result = keypoints_for(
                scratch.path(),
                &manifest,
                role,
                false,
                &mut sidecar,
                &mut |line: &str| log.push(line.to_string()),
                &mut notes,
            );
            match result {
                Ok(Some(hit)) if hit == stored => {}
                other => failures.push(format!("{view}: not the stored file: {other:?}")),
            }
            let said = format!(
                "  {}: {} frames from cache",
                role.as_str(),
                stored.frames.len()
            );
            if log != [said] || !notes.is_empty() {
                failures.push(format!("{view}: said {log:?}, noted {notes:?}"));
            }
            if fs::read(&cache).unwrap() != before {
                failures.push(format!("{view}: the cache was rewritten"));
            }
        }

        // The band note, held to the artifact frozen Python wrote with the same estimator.
        let analysis: Value =
            serde_json::from_slice(&fs::read(swing_dir.join("analysis.json")).unwrap()).unwrap();
        let stored_notes: Vec<&str> = analysis["notes"]
            .as_array()
            .map(|notes| notes.iter().filter_map(Value::as_str).collect())
            .unwrap_or_default();
        let carries = |note: &str| stored_notes.contains(&note);
        match &band_note {
            Some(note) if !carries(note) => {
                failures.push(format!("{label}: analysis.json lacks the band note"))
            }
            None if stored_notes
                .iter()
                .any(|n| n.contains("benchmark bands were cut")) =>
            {
                failures.push(format!(
                    "{label}: analysis.json has a band note this run does not"
                ))
            }
            _ => {}
        }
    }
    assert!(
        !sidecar.started(),
        "a cache check started the pose sidecar: {failures:?}"
    );
    assert!(
        failures.is_empty(),
        "{} of {views} stored views:\n{}",
        failures.len(),
        failures.join("\n")
    );
    eprintln!("{views} stored views are cache hits; no worker was spawned");
}

/// The smallest clip any stored manifest records, by its size on disk.
fn smallest_stored_clip(sessions: &Path) -> (PathBuf, Role, u64) {
    let mut smallest: Option<(PathBuf, Role, u64)> = None;
    for swing_dir in swing_dirs(sessions) {
        let manifest = manifest_of(&swing_dir);
        for role in ROLES {
            let Some(role_file) = manifest.roles.get(&role) else {
                continue;
            };
            let Ok(meta) = fs::metadata(swing_dir.join(&role_file.filename)) else {
                continue;
            };
            if smallest
                .as_ref()
                .is_none_or(|(_, _, size)| meta.len() < *size)
            {
                smallest = Some((swing_dir.clone(), role, meta.len()));
            }
        }
    }
    smallest.expect("a stored clip")
}

/// The largest coordinate difference between two frames' landmarks, and whether any differs.
fn landmark_difference(posed: &KeypointsFile, stored: &KeypointsFile) -> (f64, usize, usize) {
    let mut worst = 0.0f64;
    let mut frames_differing = 0;
    let mut values_differing = 0;
    for (a, b) in posed.frames.iter().zip(&stored.frames) {
        let mut differs = false;
        for (x, y) in a.landmarks.iter().zip(&b.landmarks) {
            for (p, q) in [
                (x.x, y.x),
                (x.y, y.y),
                (x.z, y.z),
                (x.visibility, y.visibility),
            ] {
                if p != q {
                    differs = true;
                    values_differing += 1;
                    worst = worst.max((p - q).abs());
                }
            }
        }
        frames_differing += usize::from(differs);
    }
    (worst, frames_differing, values_differing)
}

#[test]
#[ignore = "needs data/processed/sessions/ (gitignored), the vision extra and \
            data/models/pose_landmarker_heavy.task; poses one clip, about a minute"]
fn a_forced_run_on_the_smallest_stored_clip_matches_its_stored_keypoints() {
    let sessions = sessions_dir();
    let (swing_dir, role, size) = smallest_stored_clip(&sessions);
    let manifest = manifest_of(&swing_dir);
    let role_file = &manifest.roles[&role];
    let label = format!(
        "{} {}",
        swing_dir.strip_prefix(&sessions).unwrap().display(),
        role.as_str()
    );
    eprintln!(
        "smallest stored clip: {label}, {} ({size} bytes)",
        role_file.filename
    );

    let scratch = tempfile::tempdir().unwrap();
    fs::copy(manifest_path(&swing_dir), manifest_path(scratch.path())).unwrap();
    fs::copy(
        swing_dir.join(&role_file.filename),
        scratch.path().join(&role_file.filename),
    )
    .unwrap();
    let stored_path = keypoints_path(&swing_dir, role);
    let stored = load_keypoints(&stored_path).unwrap();

    let mut sidecar = Sidecar::configured();
    let started = std::time::Instant::now();
    let mut log = Vec::new();
    let mut notes = Vec::new();
    let posed = keypoints_for(
        scratch.path(),
        &manifest,
        role,
        true,
        &mut sidecar,
        &mut |line: &str| log.push(line.to_string()),
        &mut notes,
    )
    .unwrap_or_else(|error| panic!("{label}: {error}"))
    .unwrap_or_else(|| panic!("{label}: no landmarks: {log:?} {notes:?}"));
    let elapsed = started.elapsed().as_secs_f64();

    let name = role.as_str();
    assert_eq!(
        log,
        [
            format!(
                "  {name}: running pose over {} ...",
                role_file.original_filename
            ),
            format!(
                "  {name}: {} frames -> {name}.keypoints.json",
                stored.frames.len()
            ),
        ]
    );
    assert!(notes.is_empty(), "{notes:?}");
    assert_eq!(posed.frames.len(), stored.frames.len(), "frame count");
    assert_eq!(posed.clip, stored.clip, "the clip block");
    assert_eq!(posed.pose_estimator, stored.pose_estimator);
    for (a, b) in posed.frames.iter().zip(&stored.frames) {
        assert_eq!(
            (a.frame_index, a.landmarks.len(), a.camera_id.as_deref()),
            (b.frame_index, b.landmarks.len(), b.camera_id.as_deref()),
            "frame {}",
            b.frame_index
        );
    }
    let written = keypoints_path(scratch.path(), role);
    assert_eq!(
        load_keypoints(&written).unwrap(),
        posed,
        "the file written is the one returned"
    );

    // Recorded, not bounded: the P22 finding carries these numbers.
    let (worst, frames_differing, values_differing) = landmark_difference(&posed, &stored);
    let timestamps_differing = posed
        .frames
        .iter()
        .zip(&stored.frames)
        .filter(|(a, b)| a.timestamp_ms != b.timestamp_ms)
        .count();
    let as_values =
        |path: &Path| -> Value { serde_json::from_slice(&fs::read(path).unwrap()).unwrap() };
    let same_json = as_values(&written) == as_values(&stored_path);
    eprintln!(
        "{label}: {} frames posed in {elapsed:.1}s; {frames_differing} frames and \
         {values_differing} landmark values differ, the largest by {worst:e}; \
         {timestamps_differing} timestamps differ; written JSON equals the stored file's as \
         values: {same_json}",
        posed.frames.len()
    );

    // And the file the lab wrote is one its own cache rule reads as current.
    let mut log = Vec::new();
    let mut notes = Vec::new();
    let hit = keypoints_for(
        scratch.path(),
        &manifest,
        role,
        false,
        &mut sidecar,
        &mut |line: &str| log.push(line.to_string()),
        &mut notes,
    )
    .unwrap();
    assert_eq!(hit.as_ref(), Some(&posed));
    assert_eq!(
        log,
        [format!(
            "  {name}: {} frames from cache",
            posed.frames.len()
        )]
    );
}
