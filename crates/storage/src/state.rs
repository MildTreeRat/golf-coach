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
//! What stays behind in `api/state.py`, because only the API calls it: `resolve_tempo_plan` (the
//! results page's) and `now()` (the clock is an argument here, as the crate doc says).
//!
//! # The placement and pivot resolvers [M29 P10]
//!
//! [`resolve_placements`] and [`resolve_pivots`] stayed behind at M36 for the same reason, and
//! came across when `golf-mcp`'s `get_swing` needed them: a stored placement or pivot row turned
//! into one that carries what makes it readable. They sit here, beside [`load_analysis`], for
//! Python's reason (the tolerant readers for `analysis.json` sit together) and so that one
//! definition serves every Rust reader of a stored swing, as Python's serves the MCP view, the
//! results page and the coaching brief. Each returns a typed row where Python returns a dict; the
//! MCP view wraps it in its own field order, as `PlacementView(**row)` does.
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

/// Write a swing's `analysis.json`: the serialized result, as `golf_core::serialize` drops it, in
/// `model_dump_json(indent=2)`'s layout. [M29 P26]
///
/// **Atomically, where `analyze_swing_dir`'s `write_text` is not**: the results page and `golf-mcp`
/// read this file while a re-run rewrites it, and a reader that met half a file would serve a swing
/// as unanalyzed. The `.tmp` beside it is the stores' (`write_atomically`), and a rename is all a
/// reader ever sees. It does not create `swing_dir`, for [`save_shot_analysis`]'s reason: a result
/// is about a directory that already holds a manifest.
///
/// It takes the JSON rather than the typed result because the drop is the shell's, not this crate's
/// (`golf_core::serialize`), and what frozen Python can read is the caller's to check before it
/// writes (`golf_lab::analyze`'s guard on the reasons, the M29 plan's C6).
pub fn save_analysis(analysis: &Value, swing_dir: &Path) -> io::Result<()> {
    if !swing_dir.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("no swing directory at {}", swing_dir.display()),
        ));
    }
    crate::write_atomically(analysis, &swing_dir.join(ANALYSIS_NAME))
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

/// `_measurement_entries`: `swing.measurements` off a stored artifact, its object entries only, or
/// nothing for anything that is not that shape.
fn measurement_entries(
    analysis: Option<&Map<String, Value>>,
) -> impl Iterator<Item = &Map<String, Value>> {
    analysis
        .and_then(|analysis| analysis.get("swing"))
        .and_then(Value::as_object)
        .and_then(|swing| swing.get("measurements"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_object)
}

/// A stored entry's `name` and `value` when the name is a string and the value a number, as the two
/// resolvers both require. `isinstance(value, int | float)` excluding `bool`: JSON's `true` is a
/// bool to `serde_json` before it could be a number, so the exclusion is the type itself here.
fn named_value(entry: &Map<String, Value>) -> Option<(&str, f64)> {
    let name = entry.get("name")?.as_str()?;
    let value = entry.get("value")?.as_f64()?;
    Some((name, value))
}

/// The stored entry's unit when it names one, else the registry's, so a row from an artifact that
/// omitted it still renders. Python's `unit if isinstance(unit, str) and unit else spec.unit`.
fn unit_or(entry: &Map<String, Value>, registered: &str) -> String {
    match entry.get("unit").and_then(Value::as_str) {
        Some(unit) if !unit.is_empty() => unit.to_string(),
        _ => registered.to_string(),
    }
}

/// The stored entry's detail when it is a string, else empty.
fn detail_of(entry: &Map<String, Value>) -> String {
    entry
        .get("detail")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

/// One population placement on a stored swing, with the two registry facts that decide how it may
/// be read. [M29 P10]
///
/// **Unit and detail come off the stored entry, view and calibrated off the registry.** An artifact
/// an older engine wrote is read back here, and the two halves differ in what they are: the first
/// pair is what that engine recorded, the second is what this repository knows about the model.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedPlacement {
    pub name: String,
    pub value: f64,
    pub unit: String,
    pub detail: String,
    pub view: &'static str,
    pub calibrated: bool,
}

/// `resolve_placements`: the population placements on a stored swing, in the artifact's order.
///
/// Membership is [`contracts::placements::placement_by_name`] and never a `tour_` prefix test: a
/// prefix would silently reclassify a future name in the direction that loses the caveat. An entry
/// with no string name or no numeric value is skipped, as Python skips it.
pub fn resolve_placements(analysis: Option<&Map<String, Value>>) -> Vec<ResolvedPlacement> {
    measurement_entries(analysis)
        .filter_map(|entry| {
            let (name, value) = named_value(entry)?;
            let spec = contracts::placements::placement_by_name(name)?;
            Some(ResolvedPlacement {
                name: name.to_string(),
                value,
                unit: unit_or(entry, spec.unit),
                detail: detail_of(entry),
                view: spec.view,
                calibrated: spec.calibrated,
            })
        })
        .collect()
}

/// One rotation measurement on a stored swing, with the sentence that says it is interim.
/// [`ResolvedPlacement`]'s sibling, split the same way between entry and registry. [M29 P10]
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedPivot {
    pub name: String,
    pub value: f64,
    pub unit: String,
    pub detail: String,
    pub view: &'static str,
    pub interim_reason: &'static str,
}

/// `resolve_pivots`: the pivot rows on a stored swing, in the artifact's order. Membership is
/// [`contracts::pivots::pivot_by_name`], never a `pivot_` prefix, for [`resolve_placements`]'
/// reason.
pub fn resolve_pivots(analysis: Option<&Map<String, Value>>) -> Vec<ResolvedPivot> {
    measurement_entries(analysis)
        .filter_map(|entry| {
            let (name, value) = named_value(entry)?;
            let spec = contracts::pivots::pivot_by_name(name)?;
            Some(ResolvedPivot {
                name: name.to_string(),
                value,
                unit: unit_or(entry, spec.unit),
                detail: detail_of(entry),
                view: spec.view,
                interim_reason: spec.interim_reason,
            })
        })
        .collect()
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

    /// The writer [`load_analysis`] reads back, replacing what was there, with no `.tmp` left beside
    /// it; and no directory made for a swing that has none. [M29 P26]
    #[test]
    fn an_analysis_round_trips_and_needs_its_swing_directory() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(ANALYSIS_NAME), "{\"analysis_version\": 3}").unwrap();
        let written = json!({"swing_id": "2", "analysis_version": 17, "notes": ["a — b"]});
        save_analysis(&written, dir.path()).unwrap();
        let read = load_analysis(dir.path()).expect("the written file reads back");
        assert_eq!(Value::Object(read), written);
        let names: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(names, [ANALYSIS_NAME], "only the artifact is left");

        let missing = dir.path().join("2026-10-09").join("9");
        let err = save_analysis(&written, &missing).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
        assert!(!missing.exists(), "no swing directory is made for a result");
    }

    /// The resolvers' tolerance, as `api/state.py`'s reads it: membership by registry, a bool or
    /// a missing value skipped, the stored unit when it names one and the registry's when it does
    /// not, a non-string detail read as empty, and a non-object entry or swing ignored. The
    /// `mcp` family's `swing-views` holds one well-formed row of each; these are the rest.
    #[test]
    fn the_resolvers_keep_registered_rows_and_read_the_rest_tolerantly() {
        let Value::Object(analysis) = json!({"swing": {"measurements": [
            {"name": "tour_joint_distance", "value": 3, "unit": "", "detail": 7},
            {"name": "tour_trajectory_q", "value": true},
            {"name": "tour_trajectory_t2", "unit": "sigma"},
            "tour_joint_distance",
            {"name": "head_sway_norm", "value": 0.12},
            {"name": "pivot_hip_axis_drift_norm", "value": 0.42, "unit": "widths", "detail": "d"},
        ]}}) else {
            unreachable!()
        };
        let placements = resolve_placements(Some(&analysis));
        assert_eq!(placements.len(), 1, "{placements:?}");
        let placement = &placements[0];
        assert_eq!(placement.value, 3.0);
        assert_eq!(
            placement.unit,
            contracts::placements::spec_for("tour_joint_distance").unit
        );
        assert_eq!(placement.detail, "");
        let pivots = resolve_pivots(Some(&analysis));
        assert_eq!(pivots.len(), 1, "{pivots:?}");
        assert_eq!(
            (pivots[0].unit.as_str(), pivots[0].detail.as_str()),
            ("widths", "d")
        );
        assert_eq!(
            pivots[0].interim_reason,
            contracts::pivots::spec_for("pivot_hip_axis_drift_norm").interim_reason
        );

        let Value::Object(odd) = json!({"swing": {"measurements": {"name": "x"}}}) else {
            unreachable!()
        };
        assert!(resolve_placements(Some(&odd)).is_empty());
        assert!(resolve_pivots(None).is_empty());
    }
}
