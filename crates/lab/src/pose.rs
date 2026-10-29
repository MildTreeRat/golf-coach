//! One view's landmarks, from cache when the clip behind them has not changed: `api/pipeline.py`'s
//! `keypoints_for`, `_older_pose_note` and `_band_estimator_note`, with `pose/estimator.py`'s
//! `pose_estimator_name`. [M29 P22]
//!
//! **`golf-lab` is `crates/pose`'s first caller.** M23 built the sidecar wired to nothing, and the
//! lab is where a swing's pose now comes from: Python's `estimate_pose` in-process is replaced by a
//! job to `python -m golf_coach.pose.worker` through [`pose::Pool`], which is the one boundary
//! ADR-035 clause 1 keeps in Python (ADR-033's last addendum names this crate as its caller).
//!
//! # The cache rule is Python's
//!
//! A `{role}.keypoints.json` is read rather than posed again when it was measured from **this
//! clip** (its `clip.source_sha256` against the manifest's) **by this estimator** (its
//! `pose_estimator` against [`pose_estimator_name`] of the configured variant). A file measured by
//! another estimator, or by an unrecorded one, is posed again, and kept with a note when this
//! machine cannot pose. Frozen Python reads and re-poses these same files until M40 (ADR-035 clause
//! 4), so every decision, log line and note is Python's, and a lab run reads the same in either
//! language.
//!
//! # A run that hits the cache spawns nothing
//!
//! [`Sidecar`] starts its pool on the first view that needs posing, not when it is made. Every
//! stored view is a cache hit, so a re-analysis of the stored bundles starts no interpreter at all:
//! `tests/keypoints.rs` holds that to all of them with [`Sidecar::started`]. The pool is
//! `crates/pose`'s [`PoolConfig`] defaults (the M29 plan's C3), and this module poses one view at a
//! time in Python's order, so the second worker of the default two idles; submitting both views at
//! once would use it, at the cost of Python's log order (the plan's P22 finding, for P25).
//!
//! # Where Python raises, and where it notes
//!
//! - **"Can this machine pose?"** is Python's `ImportError` branch, and here it is the pool failing
//!   to start: no interpreter answered, the `vision` extra or the model is absent, the variant is
//!   unknown, or the protocols disagree. Its `why` is [`PoseError`]'s text, which names what was
//!   looked for, where Python's names the extra. It is asked once per [`Sidecar`] and remembered,
//!   because a failed start can cost a handshake timeout and the second view would pay it again.
//! - **A job that fails is Python's raise**: `FileVideoSource` refusing a file, or `estimate_pose`
//!   throwing out of MediaPipe. It is [`KeypointsError::Pose`] and stops the bundle, after
//!   `crates/pose`'s one retry where clause 5 allows it. The exception is `clip_empty`, which is
//!   Python's `if not frames` and is a note.
//! - **A broken cache stops the bundle**, as `load_keypoints` raising does there, and so does a
//!   failed write.
//!
//! # What the file holds
//!
//! The worker's landmarks and clip block, with `source_sha256` the manifest's and `pose_estimator`
//! the configured one: what Python's `keypoints_for` stamps, so the next run's cache check reads
//! the same key in both languages. The worker's own values are equal on every intact bundle (it
//! hashes the bytes on disk, which the upload route hashed as they arrived: 30 of 30 in ADR-033's
//! 2026-09-26 addendum, and it serves the variant it is started with). Where the worker's hash
//! disagrees with the manifest's, that is noted, because Python never hashes the clip and could not
//! see it; where its estimator disagrees, the landmarks are refused, because an artifact stamped
//! with an instrument that did not measure it is the lie the cache key exists to prevent.
//!
//! The writer is `pose::writer::write`, `save_keypoints`' twin (ADR-033 clause 8, measured
//! value-equal over all 30 stored files in M23 P7), and like Python's it is not atomic.
//!
//! # The variant
//!
//! `GOLF_POSE_MODEL_VARIANT`, `settings.pose_model_variant`'s environment name, else `config.py`'s
//! default, which a test reads out of `config.py` so the two cannot drift. `.env` is not read, as
//! `golf-core`'s `DataDirs` does not read it: it holds no pose setting today, and a dotenv parser is
//! a dependency for a file that cannot move this answer. An empty variable counts as unset, the
//! rule `DataDirs` keeps. **The variant is not validated here**: `pose/estimator.py::resolve_variant`
//! owns the list (clause 2), and a second copy is a second thing to forget. A typo reaches the lab as
//! the worker's `bad_variant` refusal, so every view keeps its older pose with a note naming it,
//! where Python raises before posing anything. No stored file can carry a misspelt estimator, so a
//! typo is never a cache hit.

use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

use ::pose::protocol::{FailureReason, Job};
use ::pose::worker::Outcome;
use ::pose::{Pool, PoolConfig, PoseError};
use contracts::keypoints::{ClipMetadata, KeypointsFile};
use storage::keypoints_io::{keypoints_path, load_keypoints};
use storage::manifest::{Role, SwingManifest};
use storage::StoreError;

use crate::file_name;

/// `settings.pose_model_variant`'s environment variable, by pydantic-settings' `GOLF_` prefix.
pub const POSE_VARIANT_ENV: &str = "GOLF_POSE_MODEL_VARIANT";

/// `config.py`'s default for `pose_model_variant`, which the bundle uploads are measured by.
/// `tests::the_default_variant_is_config_pys` reads it out of `config.py`.
pub const DEFAULT_POSE_VARIANT: &str = "heavy";

/// The variant this run poses with: [`POSE_VARIANT_ENV`], else [`DEFAULT_POSE_VARIANT`]. `env` is
/// asked by name, so a test can answer for the process environment.
pub fn configured_variant(env: impl Fn(&str) -> Option<String>) -> String {
    env(POSE_VARIANT_ENV)
        .filter(|variant| !variant.is_empty())
        .unwrap_or_else(|| DEFAULT_POSE_VARIANT.to_string())
}

/// `"mediapipe:heavy"`, the name a stored artifact records its measuring instrument by:
/// `pose/estimator.py::pose_estimator_name`, and the worker's handshake `estimator`.
pub fn pose_estimator_name(variant: &str) -> String {
    format!("mediapipe:{variant}")
}

/// The pose sidecar as the lab holds it: a [`Pool`] started on first use, serving one variant.
///
/// Dropping it shuts the pool down, which is [`Pool`]'s own `Drop`.
pub struct Sidecar {
    config: PoolConfig,
    variant: String,
    /// `None` until a view needs posing; then the pool, or why it would not start.
    pool: Option<Result<Pool, String>>,
    jobs: u64,
}

impl Sidecar {
    /// The lab's sidecar: the configured variant, over `crates/pose`'s defaults (the plan's C3).
    pub fn configured() -> Self {
        Self::new(
            configured_variant(|name| std::env::var(name).ok()),
            PoolConfig::default(),
        )
    }

    /// A sidecar serving `variant` over `config`. The variant is set on the workers' process
    /// (`Config::variant`) and on every job, so the worker poses with the bundle this run's cache
    /// key names rather than with whatever its own environment says.
    pub fn new(variant: impl Into<String>, mut config: PoolConfig) -> Self {
        let variant = variant.into();
        config.worker.variant = Some(variant.clone());
        Self {
            config,
            variant,
            pool: None,
            jobs: 0,
        }
    }

    pub fn variant(&self) -> &str {
        &self.variant
    }

    /// `pose_estimator_name()` for this sidecar's variant: what a cached file must say to be read.
    pub fn estimator(&self) -> String {
        pose_estimator_name(&self.variant)
    }

    /// Whether a pool was ever asked to start, successfully or not. A run of cache hits answers
    /// `false`, which is how the gate says no interpreter was spawned.
    pub fn started(&self) -> bool {
        self.pool.is_some()
    }

    /// Start the pool if it has not been, and say whether this machine can pose.
    fn start(&mut self) -> Result<(), String> {
        let config = &self.config;
        match self
            .pool
            .get_or_insert_with(|| Pool::new(config).map_err(|error| error.to_string()))
        {
            Ok(_) => Ok(()),
            Err(why) => Err(why.clone()),
        }
    }

    /// Pose one clip and wait for it. Only called after [`Self::start`] answered `Ok`.
    fn run(&mut self, clip: &Path, camera_id: &str) -> Result<Outcome, PoseError> {
        self.jobs += 1;
        let Some(Ok(pool)) = &self.pool else {
            return Err(PoseError::ShutDown);
        };
        // Absolute, because `Job::clip_path` is specified absolute (clause 2) and the worker runs
        // with the repository root as its working directory rather than this process's.
        let job = Job {
            job_id: format!("golf-lab-{}", self.jobs),
            clip_path: std::path::absolute(clip)?,
            frame_range: None,
            // `FileVideoSource(video_path, camera_id=role.value)`: every frame carries the role.
            camera_id: Some(camera_id.to_string()),
            pose_model_variant: Some(self.variant.clone()),
        };
        pool.submit(job)?;
        // `recv` and not `recv_timeout`, as `golf-pose run` waits: clause 6's deadlines are the
        // worker's, and the pool reports a passed one as a `WorkTimeout`.
        match pool.results().recv() {
            Ok(completed) => completed.result,
            Err(_) => Err(PoseError::ShutDown),
        }
    }
}

/// Why a view's pose stopped the bundle, as Python's raise would have.
#[derive(Debug)]
pub enum KeypointsError {
    /// The cached file could not be read: `load_keypoints` raising.
    Store(StoreError),
    /// The sidecar was asked and did not pose the clip: `FileVideoSource` or `estimate_pose`
    /// raising, or a worker reporting a failure `crates/pose` would not retry or retried once.
    Pose { clip: PathBuf, detail: String },
    /// The landmarks could not be written: `save_keypoints` raising.
    Write { path: PathBuf, error: io::Error },
}

impl fmt::Display for KeypointsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KeypointsError::Store(error) => write!(f, "{error}"),
            KeypointsError::Pose { clip, detail } => {
                write!(f, "pose failed on {}: {detail}", clip.display())
            }
            KeypointsError::Write { path, error } => {
                write!(f, "writing {}: {error}", path.display())
            }
        }
    }
}

impl std::error::Error for KeypointsError {}

impl From<StoreError> for KeypointsError {
    fn from(error: StoreError) -> Self {
        KeypointsError::Store(error)
    }
}

/// Pose output for one view, from cache when the clip behind it has not changed: `keypoints_for`.
/// Writes `{role}.keypoints.json` when it poses.
///
/// `Ok(None)` when there is nothing to pose: no such role, the clip missing from disk, a machine
/// that cannot pose, or a clip that decoded no frames, each but the first with a note. The face-on
/// view answering `None` is the caller's to refuse, as `analyze_swing_dir` refuses it.
pub fn keypoints_for(
    swing_dir: &Path,
    manifest: &SwingManifest,
    role: Role,
    force: bool,
    sidecar: &mut Sidecar,
    log: &mut dyn FnMut(&str),
    notes: &mut Vec<String>,
) -> Result<Option<KeypointsFile>, KeypointsError> {
    let Some(role_file) = manifest.roles.get(&role) else {
        return Ok(None);
    };
    let name = role.as_str();
    let estimator = sidecar.estimator();
    let cache_path = keypoints_path(swing_dir, role);

    // Same clip, another estimator: worth re-running, and worth keeping if this machine cannot.
    let mut superseded: Option<KeypointsFile> = None;
    if cache_path.exists() && !force {
        let cached = load_keypoints(&cache_path)?;
        let same_clip = cached
            .clip
            .as_ref()
            .and_then(|clip| clip.source_sha256.as_deref())
            == Some(role_file.content_sha256.as_str());
        if !same_clip {
            log(&format!(
                "  {name}: cached keypoints are for a different clip, re-running pose"
            ));
        } else if cached.pose_estimator.as_deref() != Some(estimator.as_str()) {
            log(&format!(
                "  {name}: cached pose is from {}, re-running with {estimator}",
                measured_by(&cached)
            ));
            superseded = Some(cached);
        } else {
            log(&format!(
                "  {name}: {} frames from cache",
                cached.frames.len()
            ));
            return Ok(Some(cached));
        }
    }

    let video_path = swing_dir.join(&role_file.filename);
    if !video_path.exists() {
        // A cached run whose clip has since been deleted is still a measurement, and losing it
        // would cost the whole bundle when this is the face-on view.
        if let Some(older) = superseded {
            log(&format!(
                "  {name}: {} is gone, keeping the older pose run",
                file_name(&video_path)
            ));
            notes.push(older_pose_note(
                role,
                &older,
                &estimator,
                "the clip is not on disk",
            ));
            return Ok(Some(older));
        }
        log(&format!(
            "  {name}: {} is missing from the swing directory",
            file_name(&video_path)
        ));
        notes.push(format!(
            "the {name} clip is recorded in the manifest but missing from disk"
        ));
        return Ok(None);
    }

    if let Err(why) = sidecar.start() {
        // Older landmarks beat none: the swing is still scoreable, by an instrument this machine
        // cannot reproduce, and the note says so.
        if let Some(older) = superseded {
            log(&format!(
                "  {name}: cannot re-run pose ({why}), keeping the older run"
            ));
            notes.push(older_pose_note(role, &older, &estimator, &why));
            return Ok(Some(older));
        }
        log(&format!("  {name}: no pose — {why}"));
        notes.push(format!("no pose for {name}: {why}"));
        return Ok(None);
    }

    log(&format!(
        "  {name}: running pose over {} ...",
        role_file.original_filename
    ));
    let refused = |detail: String| KeypointsError::Pose {
        clip: video_path.clone(),
        detail,
    };
    let (posed, claimed) = match sidecar.run(&video_path, name) {
        Ok(Outcome::Done {
            keypoints,
            claimed_frames,
        }) => (keypoints, claimed_frames),
        Ok(Outcome::Failed {
            reason: FailureReason::ClipEmpty,
            ..
        }) => return Ok(decoded_nothing(name, log, notes)),
        // `{reason:?}`, as `golf-pose run` prints it: the variant name is one underscore-transform
        // from clause 5's wire spelling, and the protocol grows no `Display` for an error line.
        Ok(Outcome::Failed { reason, detail }) => {
            return Err(refused(format!("{reason:?} — {detail}")))
        }
        Err(error) => return Err(refused(error.to_string())),
    };
    if posed.frames.is_empty() {
        return Ok(decoded_nothing(name, log, notes));
    }
    if let Some(theirs) = posed
        .pose_estimator
        .as_deref()
        .filter(|theirs| *theirs != estimator)
    {
        return Err(refused(format!(
            "the pose sidecar answered with landmarks from {theirs}, not the configured {estimator}"
        )));
    }

    let decoded = posed.frames.len();
    if claimed != 0 && claimed != decoded as i64 {
        // A file that opens cleanly and then decodes short produces a plausible-looking analysis of
        // an incomplete swing (docs/M7_TWO_PHONE_SPIKE.md, Q2), so it is a note and not a log line.
        let message = format!(
            "the {name} clip claims {claimed} frames but only {decoded} decoded — this analysis \
             may be of an incomplete swing"
        );
        log(&format!("  {name}: warning - {message}"));
        notes.push(message);
    }
    let clip = posed.clip.unwrap_or(ClipMetadata {
        fps: None,
        width: None,
        height: None,
        frame_count: None,
        source_sha256: None,
    });
    if let Some(theirs) = clip
        .source_sha256
        .as_deref()
        .filter(|theirs| *theirs != role_file.content_sha256)
    {
        let message = format!(
            "the {name} clip on disk hashes to {theirs}, not to the {} its manifest records, so \
             these landmarks may not be of the clip this swing was uploaded with",
            role_file.content_sha256
        );
        log(&format!("  {name}: warning - {message}"));
        notes.push(message);
    }

    let keypoints = KeypointsFile {
        clip: Some(ClipMetadata {
            frame_count: Some(decoded as i64),
            source_sha256: Some(role_file.content_sha256.clone()),
            ..clip
        }),
        frames: posed.frames,
        pose_estimator: Some(estimator),
    };
    ::pose::writer::write(&keypoints, &cache_path).map_err(|error| KeypointsError::Write {
        path: cache_path.clone(),
        error,
    })?;
    log(&format!(
        "  {name}: {decoded} frames -> {}",
        file_name(&cache_path)
    ));
    Ok(Some(keypoints))
}

/// `if not frames`: a container that opened and decoded nothing.
fn decoded_nothing(
    name: &str,
    log: &mut dyn FnMut(&str),
    notes: &mut Vec<String>,
) -> Option<KeypointsFile> {
    log(&format!("  {name}: no frames decoded"));
    notes.push(format!("the {name} clip decoded no frames at all"));
    None
}

/// `superseded.pose_estimator or 'an unrecorded estimator'`: Python's `or` reads an empty string as
/// absent too.
fn measured_by(keypoints: &KeypointsFile) -> &str {
    keypoints
        .pose_estimator
        .as_deref()
        .filter(|estimator| !estimator.is_empty())
        .unwrap_or("an unrecorded estimator")
}

/// Why a result carries landmarks the configured estimator did not produce: `_older_pose_note`.
///
/// Said rather than substituted quietly, because a reader comparing this result with one measured
/// by the configured estimator has to be able to see where the difference came from.
pub fn older_pose_note(
    role: Role,
    superseded: &KeypointsFile,
    estimator: &str,
    because: &str,
) -> String {
    format!(
        "the {} landmarks were measured by {} rather than by the configured {estimator}, and pose \
         could not be re-run because {because}",
        role.as_str(),
        measured_by(superseded)
    )
}

/// Say so when the swing and the bands it is judged against were measured differently:
/// `_band_estimator_note`, against the estimators `golfdb_v1.json`'s `dataset` block names.
///
/// ADR-012 §4: a band is comparable to a swing measured the same way, because the two sides share
/// an estimator's bias and it cancels, and that fails when they diverge. A note and not a refusal,
/// because the bands are still the best reference there is. Called once per bundle by the caller
/// (`analyze_swing_dir`), never per view.
pub fn band_estimator_note(estimator: &str) -> Option<String> {
    band_estimator_note_against(
        estimator,
        analysis::benchmarks::distributions::dataset_pose_estimators(),
    )
}

/// [`band_estimator_note`] over a stated corpus, so a test can name one. An empty `measured_by` is
/// Python's `None` (no note); an explicit `[]` in the file would read the same here where Python
/// would write a sentence naming nobody, and no artifact holds one.
pub fn band_estimator_note_against(estimator: &str, measured_by: &[String]) -> Option<String> {
    if measured_by.is_empty() || measured_by.iter().any(|corpus| corpus == estimator) {
        return None;
    }
    Some(format!(
        "this swing was measured by {estimator}, but the benchmark bands were cut from swings \
         measured by {} — the two instruments do not cancel, so read the scores as approximate \
         until the corpus is re-derived (ADR-012 §4)",
        measured_by.join(" and ")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keypoints(estimator: Option<&str>) -> KeypointsFile {
        KeypointsFile {
            clip: None,
            frames: Vec::new(),
            pose_estimator: estimator.map(str::to_string),
        }
    }

    #[test]
    fn the_default_variant_is_config_pys() {
        // Frozen Python's `Settings.pose_model_variant` default, read from the source rather than
        // copied, so the lab and the frozen server key the same cache until M40.
        let config = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../src/golf_coach/config.py");
        let text = std::fs::read_to_string(&config).unwrap();
        let line = text
            .lines()
            .map(str::trim)
            .find(|line| line.starts_with("pose_model_variant: str = "))
            .expect("config.py declares pose_model_variant");
        assert_eq!(
            line,
            format!("pose_model_variant: str = \"{DEFAULT_POSE_VARIANT}\"")
        );
    }

    #[test]
    fn the_variant_is_the_environments_else_the_default_and_empty_is_unset() {
        let set = |value: &'static str| {
            move |name: &str| (name == POSE_VARIANT_ENV).then(|| value.to_string())
        };
        assert_eq!(configured_variant(set("lite")), "lite");
        assert_eq!(configured_variant(set("")), DEFAULT_POSE_VARIANT);
        assert_eq!(configured_variant(|_| None), DEFAULT_POSE_VARIANT);
        assert_eq!(pose_estimator_name("lite"), "mediapipe:lite");
        let sidecar = Sidecar::new("full", PoolConfig::default());
        assert_eq!(sidecar.estimator(), "mediapipe:full");
        assert_eq!(sidecar.config.worker.variant.as_deref(), Some("full"));
        assert!(!sidecar.started(), "made, not started");
    }

    #[test]
    fn the_older_pose_note_is_pythons_and_names_an_unrecorded_estimator() {
        assert_eq!(
            older_pose_note(
                Role::DownTheLine,
                &keypoints(Some("mediapipe:lite")),
                "mediapipe:heavy",
                "the clip is not on disk"
            ),
            "the down_the_line landmarks were measured by mediapipe:lite rather than by the \
             configured mediapipe:heavy, and pose could not be re-run because the clip is not on \
             disk"
        );
        for unrecorded in [None, Some("")] {
            assert!(
                older_pose_note(Role::FaceOn, &keypoints(unrecorded), "mediapipe:heavy", "x")
                    .starts_with(
                        "the face_on landmarks were measured by an unrecorded estimator rather"
                    )
            );
        }
    }

    #[test]
    fn the_band_note_names_every_corpus_estimator_and_is_silent_on_a_match() {
        let corpus = |names: &[&str]| names.iter().map(|n| n.to_string()).collect::<Vec<_>>();
        assert_eq!(
            band_estimator_note_against("mediapipe:heavy", &corpus(&["mediapipe:lite"])).unwrap(),
            "this swing was measured by mediapipe:heavy, but the benchmark bands were cut from \
             swings measured by mediapipe:lite — the two instruments do not cancel, so read the \
             scores as approximate until the corpus is re-derived (ADR-012 §4)"
        );
        assert!(band_estimator_note_against(
            "mediapipe:heavy",
            &corpus(&["mediapipe:lite", "mediapipe:full"])
        )
        .unwrap()
        .contains("measured by mediapipe:lite and mediapipe:full —"));
        assert_eq!(
            band_estimator_note_against(
                "mediapipe:lite",
                &corpus(&["mediapipe:heavy", "mediapipe:lite"])
            ),
            None
        );
        assert_eq!(band_estimator_note_against("mediapipe:heavy", &[]), None);
    }
}
