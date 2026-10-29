//! One swing bundle in, the engine's input out: `api/pipeline.py::analyze_swing_dir` up to its
//! `analyze_swing_bundle` call, with `_handedness_for`, `_loft_for` and `loft_remedy`. [M29 P25]
//!
//! **Python's order, step for step, and every log line and note in Python's words**, because frozen
//! Python reads what this writes until M40 (ADR-035 clause 4) and a lab run should read the same in
//! either language: the bundle line and the missing roles, the band note, pose per view
//! ([`keypoints_for`]), the strikes per view ([`audio_for`]), the swing selection
//! ([`golf_core::auto_windows`], the one definition the selection family gates), the shot
//! ([`shot_for`]), the shot-without-a-strike note, the golfer's handedness and the club's loft. What
//! comes after the engine call (the loft note it gates, the feedback, coaching, the writers and
//! `analysis.state.json`) is the M29 plan's P26, in [`crate::analyze`], and [`Assembled`] carries
//! what it needs.
//!
//! **The input is a [`VectorInput`]**, `golf_core::run`'s: an engine vector's `input` is exactly the
//! arguments the pipeline hands `analyze_swing_bundle`, so one type serves the vectors and the lab,
//! and [`corpus_input`] cuts it as `conformance_vectors.py::_corpus_vector` cuts a bundle into a
//! committed corpus vector. `tests/inputs.rs` is the M29 plan's D20 gate on that: every stored
//! bundle assembled, cut, and held to its committed corpus vector's input.
//!
//! # Departures
//!
//! - **No render** (the plan's C4), so no `tau_range`, and `PipelineOptions`' `render_video` has no
//!   twin.
//! - **The intent is Python's, `PracticeGoal(club=options.club)`, and not the manifest's.** Since
//!   M35 a manifest may carry an `intent` (`golf-core set-intent`), which frozen Python ignores and
//!   `golf_core::run_shot` reads for a photo entry. A swing bundle's goal stays the CLI's club
//!   category, `all` by default, because the category picks the bands a checkpoint is judged by, and
//!   every committed corpus vector carries `all` (D20's gate would refuse anything else).
//! - **The dry run is the shot's alone** ([`ShotOptions::dry_run`]): pose and audio write only on a
//!   miss, and the stored bundles are all hits, which `tests/inputs.rs` holds by hashing its scratch
//!   copy before and after.
//! - **The machine's instruments are the caller's** ([`Tools`]): one ffmpeg lookup, one pose sidecar
//!   and one OCR reader per run rather than per bundle, so a re-run over every bundle opens the
//!   models and starts the pool once at most (the plan's P21–P23 findings).

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use contracts::club::ClubId;
use contracts::golfer::Handedness;
use contracts::intent::{ClubCategory, PracticeGoal};
use contracts::keypoints::{ClipMetadata, KeypointsFile};
use contracts::unscored::{UnscoredCheckpoint, UnscoredReason};
use golf_core::auto_windows::{auto_windows, View};
use golf_core::VectorInput;
use serde_json::{json, Map, Value};
use storage::flight_inputs::{loft_for_club, LoftGap};
use storage::golfer_store::GolferStore;
use storage::manifest::{load_manifest, manifest_path, Role, SwingManifest};
use storage::StoreError;

use crate::audio::{audio_for, AudioOptions};
use crate::ffmpeg::{Ffmpeg, FfmpegMissing};
use crate::import::spelled;
use crate::pose::{band_estimator_note, keypoints_for, KeypointsError, Sidecar};
use crate::shot::{shot_for, Origin, PhotoReader, ShotOptions};

/// `VIEWS`: the two clips, in the order every step visits them, with the label a sentence names
/// each by.
pub const VIEWS: [(Role, &str); 2] = [
    (Role::FaceOn, "face-on"),
    (Role::DownTheLine, "down-the-line"),
];

/// What a run may vary: `PipelineOptions`, less the render (C4), plus the golfer registry
/// `settings.golfers_dir` names in Python. `coaching` is read by [`crate::analyze`] alone (P26).
#[derive(Debug, Clone, PartialEq)]
pub struct PipelineOptions {
    /// The shot store, the device, the confidence floor, `--force-ocr`, `--skip-ocr` and the dry run.
    pub shot: ShotOptions,
    /// Where the golfer registry and the bags are: `_handedness_for` and `_loft_for` read it.
    pub golfers_dir: PathBuf,
    /// `--club`: the goal's club category, which picks the bands a checkpoint is judged by.
    pub club: ClubCategory,
    pub window_face_on: Option<(i64, i64)>,
    pub window_down_the_line: Option<(i64, i64)>,
    /// Pick each view's swing (`--no-auto-window` turns it off). Windows given by hand win either way.
    pub auto_window: bool,
    pub force_pose: bool,
    pub force_audio: bool,
    /// Ask for the written coaching paragraph (C5): `settings.coaching_enabled`, on by default, and
    /// off under `--no-coaching` ([`crate::coach::coaching_enabled`] reads the setting).
    pub coaching: bool,
}

impl PipelineOptions {
    /// Python's defaults over the two stores, no flag set.
    pub fn new(shot: ShotOptions, golfers_dir: impl Into<PathBuf>) -> Self {
        PipelineOptions {
            shot,
            golfers_dir: golfers_dir.into(),
            club: ClubCategory::All,
            window_face_on: None,
            window_down_the_line: None,
            auto_window: true,
            force_pose: false,
            force_audio: false,
            coaching: true,
        }
    }
}

/// The machine's instruments, borrowed for one bundle: what can decode audio, what can pose, and
/// what can read a photo. The caller owns them across a run, and can ask each afterwards what it did
/// (`Sidecar::started`, `OcrReader::reads`).
pub struct Tools<'a> {
    pub ffmpeg: Result<&'a Ffmpeg, &'a FfmpegMissing>,
    pub sidecar: &'a mut Sidecar,
    pub reader: &'a mut dyn PhotoReader,
}

/// Why a bundle stopped where Python's would have raised: a broken pose or strike cache, a pose job
/// that failed, or a stored shot that is not a shot. Every expected failure is an [`Assembly`].
#[derive(Debug)]
pub enum PipelineError {
    Keypoints(KeypointsError),
    Store(StoreError),
}

impl fmt::Display for PipelineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PipelineError::Keypoints(error) => write!(f, "{error}"),
            PipelineError::Store(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for PipelineError {}

impl From<KeypointsError> for PipelineError {
    fn from(error: KeypointsError) -> Self {
        PipelineError::Keypoints(error)
    }
}

impl From<StoreError> for PipelineError {
    fn from(error: StoreError) -> Self {
        PipelineError::Store(error)
    }
}

/// What [`assemble`] made of one bundle.
#[derive(Debug, Clone)]
pub enum Assembly {
    /// Nothing to score: Python's `PipelineOutcome(error=…)`.
    Refused(Box<Refusal>),
    /// The engine's input, and what the steps after the engine need.
    Ready(Box<Assembled>),
}

/// A bundle with nothing to score. `manifest` is `None` when there was none to read, and then no
/// state is written for it (Python's reason: no `inputs` to key one on); with one, P26 records the
/// failed run in `analysis.state.json`, as `analyze_swing_dir` does for a missing face-on view.
#[derive(Debug, Clone, PartialEq)]
pub struct Refusal {
    pub manifest: Option<SwingManifest>,
    pub missing: Vec<Role>,
    pub error: String,
}

/// One bundle assembled for the engine.
#[derive(Debug, Clone)]
pub struct Assembled {
    pub manifest: SwingManifest,
    /// The roles the bundle never received, which flag the run.
    pub missing: Vec<Role>,
    /// `analyze_swing_bundle`'s arguments.
    pub input: VectorInput,
    /// Every note the steps before the engine wrote, in encounter order. `analyze_swing_dir`
    /// appends them after the engine's own, and the stored `analysis.json` files hold that order.
    pub notes: Vec<String>,
    /// `_loft_for`'s candidate repair, appended only once the engine has reported `no_club_loft`
    /// ([`Assembled::loft_note`]).
    pub loft_remedy: Option<String>,
    /// Where the shot came from, for a caller that reports it.
    pub shot_origin: Origin,
}

impl Assembled {
    /// The loft repair, **only when the engine reached for the loft and missed it**: a shot whose
    /// screen printed a spin flies without one, and telling it to go and tag a club beside its
    /// simulated flight was the bug Python's gate exists for (`analyze_swing_dir`'s comment).
    pub fn loft_note(&self, unscored: &[UnscoredCheckpoint]) -> Option<&str> {
        let missed = unscored
            .iter()
            .any(|entry| entry.reason == UnscoredReason::NoClubLoft);
        self.loft_remedy.as_deref().filter(|_| missed)
    }
}

/// One bundle, assembled for the engine: `analyze_swing_dir` up to `analyze_swing_bundle`.
///
/// `Ok(Assembly::Refused)` for the two expected failures Python returns an outcome for (no readable
/// manifest, no usable face-on view); `Err` where Python raises.
pub fn assemble(
    swing_dir: &Path,
    options: &PipelineOptions,
    tools: &mut Tools<'_>,
    log: &mut dyn FnMut(&str),
) -> Result<Assembly, PipelineError> {
    let Some(manifest) = load_manifest(&manifest_path(swing_dir)) else {
        return Ok(Assembly::Refused(Box::new(Refusal {
            manifest: None,
            missing: Vec::new(),
            error: format!("{} has no readable manifest", spelled(swing_dir).display()),
        })));
    };

    let missing = manifest.missing_roles();
    log(&format!(
        "\nBundle {}/{}  [{}]",
        manifest.session_id,
        manifest.swing_id,
        manifest.status().as_str()
    ));
    let missing_names = missing
        .iter()
        .map(|role| role.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    if !missing.is_empty() {
        log(&format!("  missing: {missing_names}"));
    }

    let mut notes: Vec<String> = Vec::new();
    if !missing.is_empty() {
        notes.push(format!(
            "analyzed without {missing_names} — the result is thinner than a complete bundle's"
        ));
    }
    if let Some(note) = band_estimator_note(&tools.sidecar.estimator()) {
        notes.push(note);
    }

    log("\nPose:");
    let mut face_on: Option<KeypointsFile> = None;
    let mut down_the_line: Option<KeypointsFile> = None;
    for (role, label) in VIEWS {
        let keypoints = keypoints_for(
            swing_dir,
            &manifest,
            role,
            options.force_pose,
            tools.sidecar,
            log,
            &mut notes,
        )?;
        match (role, keypoints) {
            (Role::FaceOn, Some(keypoints)) => face_on = Some(keypoints),
            (Role::FaceOn, None) => {
                return Ok(Assembly::Refused(Box::new(Refusal {
                    manifest: Some(manifest),
                    missing,
                    error: format!(
                        "no usable {label} view — that is the view every checkpoint is measured \
                         from, so there is nothing to score"
                    ),
                })));
            }
            (_, keypoints) => down_the_line = keypoints,
        }
    }
    let face_on = face_on.expect("the face-on view was posed or the bundle was refused");

    // Unconditional, as Python's is since M11 P6: the engine pins each view's impact to the strike
    // that view heard, whether its window was found or given.
    log("\nAudio:");
    let mut face_on_strikes: Option<Vec<i64>> = None;
    let mut down_the_line_strikes: Option<Vec<i64>> = None;
    for (role, label) in VIEWS {
        let (view, strikes) = match role {
            Role::FaceOn => (Some(&face_on), &mut face_on_strikes),
            _ => (down_the_line.as_ref(), &mut down_the_line_strikes),
        };
        let Some(view) = view else {
            continue;
        };
        // The fps this view's pose was decoded at, which makes a sample index a frame index in this
        // clip's own numbering.
        let audio = audio_for(
            swing_dir,
            &manifest,
            role,
            AudioOptions {
                fps: clip_fps(view),
                force: options.force_audio,
            },
            tools.ffmpeg,
            log,
            &mut notes,
        )?;
        let Some(audio) = audio else {
            continue;
        };
        *strikes = Some(
            audio
                .strikes
                .iter()
                .filter_map(|strike| strike.frame)
                .collect(),
        );
        if audio.strikes.is_empty() {
            // The one result that is a finding rather than a gap: the detector ran and heard
            // nothing above the clip's own floor, which does not decline the window.
            log(&format!("  {}: no ball strike in this clip", role.as_str()));
            notes.push(format!(
                "no ball strike was heard in the {label} clip, so its swing was picked on movement \
                 alone — if this was a rehearsal, the numbers describe a swing that never hit a ball"
            ));
        }
    }

    let mut face_on_window = options.window_face_on;
    let mut down_the_line_window = options.window_down_the_line;
    if options.auto_window {
        log("\nSwing selection:");
        let chosen = auto_windows(
            view_of(&face_on, &face_on_strikes),
            down_the_line
                .as_ref()
                .map(|keypoints| view_of(keypoints, &down_the_line_strikes)),
            face_on_window,
            down_the_line_window,
            log,
            &mut notes,
        );
        face_on_window = chosen.face_on;
        down_the_line_window = chosen.down_the_line;
    }

    log("\nShot data:");
    let shot = shot_for(swing_dir, &manifest, &options.shot, tools.reader, log)?;
    if let Some(note) = &shot.note {
        log(&format!("  {note}"));
        notes.push(note.clone());
    }

    // Two independent witnesses to one event disagreeing: a recorded ball and no crack in either
    // clip most likely means the footage and the shot are not the same swing. Python's
    // `strikes and not any(strikes.values())`: some view was listened to, and none heard a strike
    // with a frame.
    let heard: Vec<&Vec<i64>> = [&face_on_strikes, &down_the_line_strikes]
        .into_iter()
        .flatten()
        .collect();
    if shot.shot.is_some() && !heard.is_empty() && heard.iter().all(|frames| frames.is_empty()) {
        notes.push(
            "the simulator recorded a shot for this swing but no clip in the bundle carries a ball \
             strike — the footage and the shot data may not describe the same swing"
                .to_string(),
        );
    }

    let (handedness, handedness_note) = handedness_for(&manifest, &options.golfers_dir);
    if let Some(note) = handedness_note {
        log(&format!("\nGolfer:\n  {note}"));
        notes.push(note);
    }

    // Resolved before the analysis and narrated after it (`Assembled::loft_note`).
    let (loft_deg, loft_remedy) = if shot.shot.is_some() {
        loft_for(&manifest, &options.golfers_dir)
    } else {
        (None, None)
    };

    let input = VectorInput {
        swing_id: manifest.swing_id.clone(),
        session_id: manifest.session_id.clone(),
        face_on,
        down_the_line,
        shot: shot.shot,
        intent: Some(PracticeGoal {
            club: options.club,
            ..PracticeGoal::default()
        }),
        face_on_window,
        down_the_line_window,
        face_on_strikes,
        down_the_line_strikes,
        handedness,
        loft_deg,
    };
    Ok(Assembly::Ready(Box::new(Assembled {
        manifest,
        missing,
        input,
        notes,
        loft_remedy,
        shot_origin: shot.origin,
    })))
}

/// `keypoints.clip.fps if keypoints.clip else None`.
fn clip_fps(keypoints: &KeypointsFile) -> Option<f64> {
    keypoints.clip.as_ref().and_then(|clip| clip.fps)
}

/// What selection reads of one posed view.
fn view_of<'a>(keypoints: &'a KeypointsFile, strikes: &'a Option<Vec<i64>>) -> View<'a> {
    View {
        frames: &keypoints.frames,
        fps: clip_fps(keypoints),
        strikes: strikes.as_deref(),
    }
}

/// `_handedness_for`: the swinging golfer's handedness, or `(None, how it went missing)`.
///
/// The note is not a duplicate of `unscored`'s `no_handedness`, and the difference is the layer: the
/// engine knows only that no handedness reached it, and whether nobody was attributed or the
/// attributed golfer is not in the registry is the shell's knowledge, with two different repairs.
/// `player_id is None` and not truthiness, as Python tests it, so an empty id is looked up and
/// missed.
pub fn handedness_for(
    manifest: &SwingManifest,
    golfers_dir: &Path,
) -> (Option<Handedness>, Option<String>) {
    let Some(player_id) = manifest.player_id.as_deref() else {
        return (
            None,
            Some(
                "no golfer is attributed to this swing, so the head_stays_back checkpoint could \
                 not be scored — its sign depends on which side the golfer swings from"
                    .to_string(),
            ),
        );
    };
    match GolferStore::new(golfers_dir).get(player_id) {
        Some(golfer) => (Some(golfer.handedness), None),
        None => (
            None,
            Some(format!(
                "golfer {} is not in the registry, so the head_stays_back checkpoint could not be \
                 scored — its sign depends on which side the golfer swings from",
                pyfmt::str_repr(player_id)
            )),
        ),
    }
}

/// `_loft_for`: the declared loft of the club this swing was hit with, or the repair a missing one
/// wants, which is a candidate the caller appends only on the engine's word ([`Assembled::loft_note`]).
pub fn loft_for(manifest: &SwingManifest, golfers_dir: &Path) -> (Option<f64>, Option<String>) {
    match loft_for_club(
        manifest.player_id.as_deref(),
        manifest.club,
        Some(golfers_dir),
    ) {
        Ok(loft) => (Some(loft), None),
        Err(gap) => (None, loft_remedy(Some(gap), manifest.club)),
    }
}

/// `loft_remedy`: the minutes of work that would give this swing a loft, named per gap. Two
/// sentences name the frozen server's bag page, as Python's do, because its bag routes stay open
/// (the M29 plan's D11); `golf_core::session`'s twin for a photo entry names `golf-core run-shot`
/// instead. [`LoftGap::NoSwing`] has none, as `remedy.get` has none: both callers are holding the
/// swing.
///
/// **[`LoftGap::NoClubTag`]'s is Rust's** [M29 P34]. Python's says to tag one on the results page
/// and re-analyze, and both of those controls now refuse (D11), naming `golf-core set-club` and
/// `golf-lab analyze`, so this names the two verbs. P26 held it to the script's text, and the
/// script went at P34. No committed vector holds it, and no `analysis.json` in `data/` (checked
/// 2026-10-29).
pub fn loft_remedy(gap: Option<LoftGap>, club: Option<ClubId>) -> Option<String> {
    // `f"club {club}"` over a `ClubId | None`, so a missing club prints Python's `None`.
    let club = club.map_or("None", ClubId::as_str);
    match gap? {
        LoftGap::NoClubTag => Some(
            "no club is tagged on this swing, so its ball flight could not be simulated — tag one \
             with golf-core set-club and re-analyze with golf-lab analyze"
                .to_string(),
        ),
        LoftGap::NoBagEntry => Some(format!(
            "club {club} is not in this golfer's bag, so its loft is unknown and the ball flight \
             could not be simulated — add the club on the bag page"
        )),
        LoftGap::NoDeclaredLoft => Some(format!(
            "the bag entry for {club} carries no loft, so the ball flight could not be simulated — \
             the club's book loft on the bag page is enough"
        )),
        LoftGap::NoSwing => None,
    }
}

/// One bundle's engine input as a committed corpus vector holds it, with the vector's two
/// provenance blocks: `conformance_vectors.py::_corpus_vector`'s cut.
#[derive(Debug, Clone, PartialEq)]
pub struct CorpusInput {
    /// The vector's `input`.
    pub input: Value,
    /// `provenance.frame_offset`: where each view's window starts in its whole clip.
    pub frame_offset: BTreeMap<String, i64>,
    /// `provenance.source_clip_frames`: each whole clip's length.
    pub source_clip_frames: BTreeMap<String, usize>,
}

/// `_corpus_vector`'s cut of an engine input: each view sliced to its window, the slice's
/// `frame_count` restated as its own length, and its strikes moved into the slice's numbering, a
/// strike outside the window dropped rather than clamped, and none at all written `null`. The
/// windows themselves are not written: the slice is the window.
///
/// The slice is Python's (`clip.frames[start:end]`, an index past either end clamped and a negative
/// one counted from the end), and the offset and the strike test use the window as given, as
/// Python's do. P30's builder writes new corpus vectors through this.
pub fn corpus_input(input: &VectorInput) -> CorpusInput {
    let mut given = Map::new();
    let value = |v: Result<Value, serde_json::Error>| v.expect("an engine input serializes");
    given.insert("swing_id".into(), json!(input.swing_id));
    given.insert("session_id".into(), json!(input.session_id));
    given.insert("shot".into(), value(serde_json::to_value(&input.shot)));
    given.insert("intent".into(), value(serde_json::to_value(&input.intent)));
    given.insert(
        "handedness".into(),
        value(serde_json::to_value(input.handedness)),
    );
    given.insert("loft_deg".into(), json!(input.loft_deg));

    let mut frame_offset = BTreeMap::new();
    let mut source_clip_frames = BTreeMap::new();
    let views = [
        (
            "face_on",
            Some(&input.face_on),
            input.face_on_window,
            &input.face_on_strikes,
        ),
        (
            "down_the_line",
            input.down_the_line.as_ref(),
            input.down_the_line_window,
            &input.down_the_line_strikes,
        ),
    ];
    for (role, file, window, strikes) in views {
        let Some(file) = file else {
            continue;
        };
        let length = file.frames.len() as i64;
        let (start, end) = window.unwrap_or((0, length));
        let (from, to) = python_slice(length, start, end);
        let frames = file.frames[from..to].to_vec();
        let mut clip = file.clip.clone().unwrap_or(ClipMetadata {
            fps: None,
            width: None,
            height: None,
            frame_count: None,
            source_sha256: None,
        });
        clip.frame_count = Some(frames.len() as i64);
        let sliced = KeypointsFile {
            clip: Some(clip),
            frames,
            pose_estimator: file.pose_estimator.clone(),
        };
        given.insert(role.into(), value(serde_json::to_value(&sliced)));
        let kept: Vec<i64> = strikes
            .as_deref()
            .unwrap_or_default()
            .iter()
            .filter(|&&strike| start <= strike && strike < end)
            .map(|strike| strike - start)
            .collect();
        given.insert(
            format!("{role}_strikes"),
            if kept.is_empty() {
                Value::Null
            } else {
                json!(kept)
            },
        );
        frame_offset.insert(role.to_string(), start);
        source_clip_frames.insert(role.to_string(), file.frames.len());
    }
    CorpusInput {
        input: Value::Object(given),
        frame_offset,
        source_clip_frames,
    }
}

/// `sequence[start:end]`'s bounds over a sequence of `length`: an index past either end clamped,
/// a negative one counted from the end, and an end before the start an empty slice.
fn python_slice(length: i64, start: i64, end: i64) -> (usize, usize) {
    let bound = |index: i64| {
        if index < 0 {
            (index + length).max(0)
        } else {
            index.min(length)
        }
    };
    let (from, to) = (bound(start), bound(end));
    (from as usize, to.max(from) as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_slice_is_pythons() {
        // `list(range(10))[a:b]` for each pair, measured in CPython.
        for (start, end, want) in [
            (2, 5, (2, 5)),
            (0, 10, (0, 10)),
            (3, 99, (3, 10)),
            (7, 3, (7, 7)),
            (-3, 10, (7, 10)),
            (0, -2, (0, 8)),
            (-99, 4, (0, 4)),
            (12, 15, (10, 10)),
        ] {
            assert_eq!(python_slice(10, start, end), want, "[{start}:{end}]");
        }
    }

    #[test]
    fn each_gap_names_its_repair_and_no_swing_names_none() {
        let club = Some(ClubId::ThreeWood);
        assert!(loft_remedy(Some(LoftGap::NoClubTag), club)
            .unwrap()
            .starts_with("no club is tagged on this swing"));
        assert_eq!(
            loft_remedy(Some(LoftGap::NoBagEntry), club).unwrap(),
            "club 3w is not in this golfer's bag, so its loft is unknown and the ball flight could \
             not be simulated — add the club on the bag page"
        );
        assert!(loft_remedy(Some(LoftGap::NoDeclaredLoft), club)
            .unwrap()
            .starts_with("the bag entry for 3w carries no loft"));
        assert_eq!(loft_remedy(Some(LoftGap::NoSwing), club), None);
        assert_eq!(loft_remedy(None, club), None);
        assert!(loft_remedy(Some(LoftGap::NoBagEntry), None)
            .unwrap()
            .starts_with("club None is not"));
    }
}
