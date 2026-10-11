//! `analysis.state.json` — what the worker did to a swing, and whether it is still true — and the
//! tolerant readers for `analysis.json` itself. `api/state.py`'s storage half. [M36 P8]
//!
//! A **sidecar**, not a field on the manifest: the manifest records what arrived from a phone, this
//! records what a machine later derived from it, so an analysis run never rewrites ingestion truth,
//! and a corrupt or deleted state file costs a re-analysis rather than a swing.
//!
//! Two jobs beyond bookkeeping. **Knowing when a result went stale**: `inputs` is the role → hash map
//! as it stood when the run started, so re-uploading a clip under the same role changes a hash and
//! the result stops [matching](AnalysisState::matches) its manifest. **Keeping the status poll
//! cheap**: `score` and `headline` are denormalised copies, so a page polling every swing of the day
//! does not parse every `analysis.json` to draw one number.
//!
//! The readers for `analysis.json` sit beside the state's, as in Python, because they answer one
//! question from two sides: is what we recorded about this swing still true? They read the raw JSON
//! rather than a typed result, so an artifact whose shape has moved on stays readable.
//!
//! What stays behind in `api/state.py`, because only the API calls it: `resolve_tempo_plan`,
//! `resolve_placements`, `resolve_pivots` and `_measurement_entries` (the results page's), and
//! `now()` (the clock is an argument here, as the crate doc says).
//!
//! # `shot_analysis.json`, which has no Python twin [M35 P8]
//!
//! A photo entry's result, a [`ShotResult`] (the M35 plan's call 1), sits beside the swing's two
//! artifacts and answers the same question for an entry with no clip, so its reader and writer live
//! here too. **One artifact per kind**: a photo entry is read from this file and never from an
//! `analysis.json` beside it, and a swing from `analysis.json` and never from this (call 6). It is
//! also **its own staleness record** — `ShotResult.photo_sha256` against the manifest's photo, in
//! [`shot_analysis_matches`] — so Rust never writes frozen Python's `analysis.state.json` for a
//! shot, and the frozen server's status page never sees a state it did not write.
//!
//! **Read typed, where `analysis.json` is read raw**, because the reason for reading that one raw
//! does not hold: frozen Python's results page renders whatever the pipeline wrote, and nothing of
//! Python's reads this file at all. So it is held to the contract whole, and a file holding a
//! version that is not an integer (`true`, `16.0`) is no result, where [`stored_analysis_version`]
//! reads the same value in an `analysis.json` as version 0. Both stay outside the counts once the
//! corpus reads this file (M35 P10), the first as `not_analyzed` and the second as `outdated`. A
//! missing `analysis_version` is the contract's default, 0, so it reads as older than every engine
//! with no special case here, and a missing `photo_sha256` reads as stale the same way (M35 P1's
//! finding 4).

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use contracts::shot_result::ShotResult;
use contracts::swing::ANALYSIS_VERSION;
use contracts::{Timestamp, Validate};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::manifest::{Role, SwingManifest};
use crate::StoreError;

/// `STATE_NAME`.
pub const STATE_NAME: &str = "analysis.state.json";

/// `api/pipeline.py::ANALYSIS_NAME`, where the pipeline writes a swing's result.
pub const ANALYSIS_NAME: &str = "analysis.json";

/// Where a photo entry's [`ShotResult`] is written. Rust's alone (the module doc). [M35 P8]
pub const SHOT_ANALYSIS_NAME: &str = "shot_analysis.json";

/// `Literal["queued", "running", "done", "failed"]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Queued,
    Running,
    Done,
    Failed,
}

impl Status {
    /// Every status, in declaration order. `tests/python_schemas.rs` holds it to the schema.
    pub const ALL: [Status; 4] = [
        Status::Queued,
        Status::Running,
        Status::Done,
        Status::Failed,
    ];
}

/// One swing's analysis lifecycle. Written at every transition, read by every poll.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalysisState {
    pub status: Status,
    /// Role → `content_sha256` at the moment the run started. The staleness key.
    #[serde(default)]
    pub inputs: BTreeMap<String, String>,
    #[serde(default)]
    pub queued_at: Option<Timestamp>,
    #[serde(default)]
    pub started_at: Option<Timestamp>,
    #[serde(default)]
    pub completed_at: Option<Timestamp>,
    #[serde(default)]
    pub duration_seconds: Option<f64>,
    /// Present only when `status` is `failed`: the message a human needs, not a traceback.
    #[serde(default)]
    pub error: Option<String>,
    /// True when the bundle was analyzed without all three roles, always through an explicit
    /// "analyze anyway", never automatically.
    #[serde(default)]
    pub partial: bool,
    #[serde(default)]
    pub missing_roles: Vec<String>,
    /// The rendered video's filename, relative to the swing directory; `None` when none was made.
    #[serde(default)]
    pub video: Option<String>,
    /// `avc1` plays in a browser and `mp4v` does not, which the results page warns about.
    #[serde(default)]
    pub video_codec: Option<String>,
    #[serde(default)]
    pub score: Option<f64>,
    #[serde(default)]
    pub headline: Option<String>,
}

impl AnalysisState {
    /// A state at `status` with every other field at pydantic's default, which is what
    /// `AnalysisState(status=…)` builds.
    pub fn new(status: Status) -> Self {
        AnalysisState {
            status,
            inputs: BTreeMap::new(),
            queued_at: None,
            started_at: None,
            completed_at: None,
            duration_seconds: None,
            error: None,
            partial: false,
            missing_roles: Vec::new(),
            video: None,
            video_codec: None,
            score: None,
            headline: None,
        }
    }

    /// Does this state still describe the files currently in the swing directory?
    pub fn matches(&self, manifest: &SwingManifest) -> bool {
        self.inputs == input_hashes(manifest)
    }
}

/// The role → sha256 map that identifies exactly which bytes an analysis ran over, keyed by each
/// role's wire name. Python builds it from `sorted(manifest.roles.items())`, which orders a
/// `StrEnum` by its value; a `BTreeMap` of the wire names is that order, and the comparison in
/// [`AnalysisState::matches`] reads no order at all.
pub fn input_hashes(manifest: &SwingManifest) -> BTreeMap<String, String> {
    manifest
        .roles
        .iter()
        .map(|(role, file)| (role.as_str().to_string(), file.content_sha256.clone()))
        .collect()
}

pub fn state_path(swing_dir: &Path) -> PathBuf {
    swing_dir.join(STATE_NAME)
}

/// The recorded state, or `None` if there is none or it cannot be read, mirroring
/// [`crate::manifest::load_manifest`]: a half-written or older-schema state file should cost a
/// re-analysis, not an error on the status page.
///
/// **A readable state with no `inputs` reads as an empty map, so it matches no manifest that has a
/// role** and the swing is stale (P1's finding 5, `unreadable-state`).
pub fn load_state(swing_dir: &Path) -> Option<AnalysisState> {
    let bytes = fs::read(state_path(swing_dir)).ok()?;
    serde_json::from_slice(&bytes).ok()
}

/// Write atomically: the status route reads this file while the worker rewrites it.
pub fn save_state(state: &AnalysisState, swing_dir: &Path) -> io::Result<()> {
    fs::create_dir_all(swing_dir)?;
    crate::write_atomically(state, &state_path(swing_dir))
}

/// The stored `analysis.json` as plain JSON, or `None` when it is missing, unreadable, not JSON, or
/// not an object.
///
/// Deliberately not read into a typed result: the results page renders whatever the pipeline wrote,
/// and a schema change should not make an old result unservable. The crate doc lists what
/// `json.loads` accepts that this does not.
pub fn load_analysis(swing_dir: &Path) -> Option<Map<String, Value>> {
    let bytes = fs::read(swing_dir.join(ANALYSIS_NAME)).ok()?;
    match serde_json::from_slice(&bytes).ok()? {
        Value::Object(analysis) => Some(analysis),
        _ => None,
    }
}

/// Which generation of the engine wrote this artifact; 0 for anything before versioning.
///
/// Read off the raw JSON, for [`load_analysis`]'s reason. Anything but a non-negative integer is 0
/// — "old enough that I cannot tell" and "definitely old" want the same repair — and that is
/// Python's test exactly: `true` is a `bool` before it is an `int`, and `16.0` is a float, so both
/// read as 0, as do `"16"`, `-1` and a missing key (the storage family's `outdated-versions`).
///
/// An integer past `i64` reads as `i64::MAX`, which is "newer than anything" as Python's unbounded
/// `int` is; one past `u64` is a float to `serde_json` and so reads as 0 (the crate doc).
pub fn stored_analysis_version(analysis: Option<&Map<String, Value>>) -> i64 {
    match analysis.and_then(|analysis| analysis.get("analysis_version")) {
        Some(Value::Number(number)) => match (number.as_i64(), number.as_u64()) {
            (Some(version), _) if version >= 0 => version,
            (None, Some(_)) => i64::MAX,
            _ => 0,
        },
        _ => 0,
    }
}

/// Was this written by an engine older than `version`? A missing analysis is **not** — it is
/// absent, which is a different repair with a different report (`NOT_ANALYZED`).
///
/// [`is_outdated`] asks it of this build's engine. The corpus asks it of the version its caller
/// names, so an engine bump never moves a recorded corpus (the M36 plan's call 7).
pub fn is_older_than(analysis: Option<&Map<String, Value>>, version: i64) -> bool {
    analysis.is_some() && stored_analysis_version(analysis) < version
}

/// `is_outdated`: was this written by an engine older than the one installed now? The question a
/// re-analysis picks its targets by — *is there a newer engine to run* — which is why it reads
/// [`ANALYSIS_VERSION`], this build's engine.
pub fn is_outdated(analysis: Option<&Map<String, Value>>) -> bool {
    is_older_than(analysis, ANALYSIS_VERSION)
}

pub fn shot_analysis_path(swing_dir: &Path) -> PathBuf {
    swing_dir.join(SHOT_ANALYSIS_NAME)
}

/// The stored [`ShotResult`], or `None` when it is missing, unreadable, not JSON, or not one the
/// contract accepts — a missing required key, a reason no enum names, a bound broken anywhere in the
/// tree. Tolerant as [`load_state`] is, for its reason: a half-written or older-shape result should
/// cost a re-analysis, not an error. The module doc says why this one is read typed.
pub fn load_shot_analysis(swing_dir: &Path) -> Option<ShotResult> {
    let bytes = fs::read(shot_analysis_path(swing_dir)).ok()?;
    serde_json::from_slice(&bytes).ok()
}

/// Write a photo entry's result atomically (a `.tmp` beside it, then a rename), as the stores
/// write: a corpus read can run while `golf-core run-shot --session` rewrites one.
///
/// **It validates first, as [`crate::bag_store::BagStore::save`] does**: a [`ShotResult`] can be
/// assembled invalid in Rust, and writing one would leave a file [`load_shot_analysis`] answers
/// `None` from, so it is [`StoreError::Invalid`] and nothing is written.
///
/// **It does not create `swing_dir`**, where [`save_state`] does as Python's does. A result is about
/// a swing directory that already holds a manifest, so a missing one is a caller's mistake, and
/// creating it would leave a swing directory with a result and no manifest, which every reader here
/// skips. It is [`StoreError::Io`] instead.
pub fn save_shot_analysis(result: &ShotResult, swing_dir: &Path) -> Result<(), StoreError> {
    result.validate().map_err(StoreError::Invalid)?;
    crate::write_atomically(result, &shot_analysis_path(swing_dir))?;
    Ok(())
}

/// Does this result still describe the photo in the swing directory? The photo entry's
/// [`AnalysisState::matches`]: the photo it was computed from against the manifest's shot screen.
///
/// **No photo on either side is not a match**, where two empty maps match in
/// [`AnalysisState::matches`]: a photo entry is defined by its photo, so a result that names none
/// cannot be shown to describe it (`ShotResult.photo_sha256`'s "`None` reads as stale"), and a
/// manifest that has lost its photo is no photo entry for it to describe.
pub fn shot_analysis_matches(result: &ShotResult, manifest: &SwingManifest) -> bool {
    match (
        result.photo_sha256.as_deref(),
        manifest.roles.get(&Role::ShotScreen),
    ) {
        (Some(computed_from), Some(photo)) => computed_from == photo.content_sha256,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::RoleFile;
    use serde_json::json;

    fn analysis(version: Value) -> Map<String, Value> {
        let Value::Object(map) = json!({"analysis_version": version}) else {
            unreachable!()
        };
        map
    }

    /// The storage family's `outdated-versions`, as frozen Python read each.
    #[test]
    fn only_a_non_negative_integer_is_a_version() {
        for (raw, version) in [
            (json!(16), 16),
            (json!(0), 0),
            (json!(17), 17),
            (json!(true), 0),
            (json!(16.0), 0),
            (json!("16"), 0),
            (json!(-1), 0),
            (json!(null), 0),
            (json!(u64::MAX), i64::MAX),
        ] {
            assert_eq!(
                stored_analysis_version(Some(&analysis(raw.clone()))),
                version,
                "{raw}"
            );
        }
        assert_eq!(stored_analysis_version(Some(&Map::new())), 0);
        assert_eq!(stored_analysis_version(None), 0);
    }

    #[test]
    fn a_missing_analysis_is_absent_not_outdated() {
        assert!(!is_outdated(None));
        assert!(!is_older_than(None, 99));
        assert!(is_older_than(Some(&analysis(json!(16))), 17));
        assert!(!is_older_than(Some(&analysis(json!(17))), 17));
        assert!(is_older_than(Some(&Map::new()), 1), "unversioned is 0");
    }

    #[test]
    fn load_analysis_answers_only_an_object() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(load_analysis(dir.path()), None);
        for (text, readable) in [
            (r#"{"analysis_version": 16}"#, true),
            ("[1, 2]", false),
            ("{not json", false),
            (r#"{"score": NaN}"#, false),
        ] {
            fs::write(dir.path().join(ANALYSIS_NAME), text).unwrap();
            assert_eq!(load_analysis(dir.path()).is_some(), readable, "{text}");
        }
    }

    #[test]
    fn a_state_round_trips_and_matches_its_manifest_by_hashes_alone() {
        let dir = tempfile::tempdir().unwrap();
        let noon: Timestamp = "2026-08-06T12:00:00Z".parse().unwrap();
        let file = |role: Role, digest: &str| RoleFile {
            role,
            filename: format!("{}.{digest}.mov", role.as_str()),
            content_sha256: digest.to_string(),
            original_filename: "clip.mov".to_string(),
            content_type: "video/quicktime".to_string(),
            size_bytes: 4,
            received_at: noon,
            warnings: vec![],
        };
        let mut manifest = SwingManifest {
            swing_id: "1".to_string(),
            session_id: "2026-08-06".to_string(),
            created_at: noon,
            updated_at: noon,
            roles: [
                (Role::ShotScreen, file(Role::ShotScreen, "c")),
                (Role::FaceOn, file(Role::FaceOn, "a")),
            ]
            .into_iter()
            .collect(),
            player_id: None,
            club: None,
            mishit: None,
            intent: None,
        };

        let mut state = AnalysisState::new(Status::Done);
        state.inputs = input_hashes(&manifest);
        state.completed_at = Some(noon);
        state.score = Some(0.75);
        save_state(&state, dir.path()).unwrap();
        let read = load_state(dir.path()).unwrap();
        assert_eq!(read, state);
        assert!(read.matches(&manifest));
        assert_eq!(
            read.inputs.keys().collect::<Vec<_>>(),
            ["face_on", "shot_screen"]
        );

        manifest
            .roles
            .get_mut(&Role::FaceOn)
            .unwrap()
            .content_sha256 = "a2".into();
        assert!(!read.matches(&manifest), "a re-upload stales the result");
    }

    #[test]
    fn a_state_with_no_inputs_reads_and_is_stale() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(state_path(dir.path()), r#"{"status": "done"}"#).unwrap();
        let state = load_state(dir.path()).unwrap();
        assert!(state.inputs.is_empty());
        fs::write(state_path(dir.path()), r#"{"status": "finished"}"#).unwrap();
        assert_eq!(load_state(dir.path()), None, "a status no Literal names");
        fs::write(
            state_path(dir.path()),
            r#"{"status": "done", "partial": null}"#,
        )
        .unwrap();
        assert_eq!(
            load_state(dir.path()),
            None,
            "partial is a bool, not optional"
        );
    }

    /// A photo entry's result as `golf-core run-shot --session` writes one: a stored HD Golf shot,
    /// unstamped, with one shot row and the loft gap's flight refusal.
    fn shot_result() -> Value {
        json!({
            "swing_id": "2",
            "session_id": "2026-10-09",
            "analysis_version": 17,
            "photo_sha256": "photo-a",
            "intent": {"mode": "drill"},
            "shot": {
                "shot_id": "s1",
                "session_id": "2026-10-09",
                "timestamp": "2026-10-09T12:00:00Z",
                "source": "screen",
                "ball_speed": 112.5,
                "carry_distance": 161.0,
                "provenance": {
                    "device": "hd_golf",
                    "parse_confidence": 0.9,
                    "image_sha256": "photo-a"
                }
            },
            "measurements": [{
                "name": "shot_ball_speed",
                "value": 112.5,
                "unit": "mph",
                "source": "launch_monitor:hd_golf",
                "detail": "as printed"
            }],
            "unscored": [{
                "name": "flight_carry_yds",
                "reason": "no_club_loft",
                "detail": "no loft"
            }],
            "notes": ["no club is tagged on this shot"]
        })
    }

    fn photo_entry(photo: Option<&str>) -> SwingManifest {
        let noon: Timestamp = "2026-10-09T12:00:00Z".parse().unwrap();
        SwingManifest {
            swing_id: "2".to_string(),
            session_id: "2026-10-09".to_string(),
            created_at: noon,
            updated_at: noon,
            roles: photo
                .map(|digest| {
                    let file = RoleFile {
                        role: Role::ShotScreen,
                        filename: format!("shot_screen.{digest}.jpg"),
                        content_sha256: digest.to_string(),
                        original_filename: "screen.jpg".to_string(),
                        content_type: "image/jpeg".to_string(),
                        size_bytes: 4,
                        received_at: noon,
                        warnings: vec![],
                    };
                    (Role::ShotScreen, file)
                })
                .into_iter()
                .collect(),
            player_id: Some("aaron".to_string()),
            club: None,
            mishit: None,
            intent: None,
        }
    }

    #[test]
    fn a_shot_result_round_trips_through_the_file_and_leaves_no_tmp() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(load_shot_analysis(dir.path()), None, "nothing written yet");
        let result: ShotResult = serde_json::from_value(shot_result()).unwrap();
        save_shot_analysis(&result, dir.path()).unwrap();
        assert_eq!(load_shot_analysis(dir.path()), Some(result.clone()));
        assert!(!dir.path().join("shot_analysis.tmp").exists());

        // A second write of the same result is the same bytes, which is what lets a re-run of the
        // verb that writes it be checked by comparing files.
        let first = fs::read(shot_analysis_path(dir.path())).unwrap();
        save_shot_analysis(&result, dir.path()).unwrap();
        assert_eq!(fs::read(shot_analysis_path(dir.path())).unwrap(), first);
    }

    /// Every way a file can fail the contract reads as no result, never as an error.
    #[test]
    fn an_unreadable_or_invalid_shot_result_is_none() {
        let dir = tempfile::tempdir().unwrap();
        let path = shot_analysis_path(dir.path());
        let mut no_shot = shot_result();
        no_shot.as_object_mut().unwrap().remove("shot");
        let mut bad_confidence = shot_result();
        bad_confidence["shot"]["provenance"]["parse_confidence"] = json!(1.5);
        let mut unknown_reason = shot_result();
        unknown_reason["unscored"][0]["reason"] = json!("smudged");
        let mut bool_version = shot_result();
        bool_version["analysis_version"] = json!(true);
        let mut float_version = shot_result();
        float_version["analysis_version"] = json!(17.0);
        let nan = shot_result().to_string().replace("112.5", "NaN");

        for (label, text) in [
            ("not json", "{not json".to_string()),
            ("an array", "[1, 2]".to_string()),
            ("no shot", no_shot.to_string()),
            ("a broken bound", bad_confidence.to_string()),
            ("a reason no enum names", unknown_reason.to_string()),
            ("a bool version", bool_version.to_string()),
            ("a float version", float_version.to_string()),
            ("NaN", nan),
        ] {
            fs::write(&path, text).unwrap();
            assert_eq!(load_shot_analysis(dir.path()), None, "{label}");
        }
    }

    /// The defaults are the tolerant reading: no version is older than every engine, and no photo
    /// hash describes no photo.
    #[test]
    fn a_result_missing_its_version_and_photo_reads_as_outdated_and_stale() {
        let dir = tempfile::tempdir().unwrap();
        let mut bare = shot_result();
        let object = bare.as_object_mut().unwrap();
        object.remove("analysis_version");
        object.remove("photo_sha256");
        fs::write(shot_analysis_path(dir.path()), bare.to_string()).unwrap();
        let read = load_shot_analysis(dir.path()).expect("the required keys are there");
        assert_eq!(read.analysis_version, 0);
        assert!(!shot_analysis_matches(&read, &photo_entry(Some("photo-a"))));
    }

    #[test]
    fn a_result_matches_only_the_photo_it_was_computed_from() {
        let result: ShotResult = serde_json::from_value(shot_result()).unwrap();
        assert!(shot_analysis_matches(
            &result,
            &photo_entry(Some("photo-a"))
        ));
        assert!(
            !shot_analysis_matches(&result, &photo_entry(Some("photo-b"))),
            "a re-uploaded photo stales the result"
        );
        assert!(
            !shot_analysis_matches(&result, &photo_entry(None)),
            "a manifest with no photo is described by no result"
        );
        let unnamed = ShotResult {
            photo_sha256: None,
            ..result
        };
        assert!(!shot_analysis_matches(&unnamed, &photo_entry(None)));
    }

    #[test]
    fn an_invalid_result_or_a_missing_directory_writes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let mut result: ShotResult = serde_json::from_value(shot_result()).unwrap();
        result.shot.provenance.as_mut().unwrap().parse_confidence = 1.5;
        let err = save_shot_analysis(&result, dir.path()).unwrap_err();
        assert!(matches!(err, StoreError::Invalid(_)), "{err}");
        assert!(!shot_analysis_path(dir.path()).exists());

        let valid: ShotResult = serde_json::from_value(shot_result()).unwrap();
        let missing = dir.path().join("2026-10-09").join("9");
        let err = save_shot_analysis(&valid, &missing).unwrap_err();
        assert!(matches!(err, StoreError::Io(_)), "{err}");
        assert!(!missing.exists(), "no swing directory is made for a result");
    }
}
