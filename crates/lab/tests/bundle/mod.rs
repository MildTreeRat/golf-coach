//! A scratch swing bundle and the stores beside it, served entirely from caches: what
//! `tests/assemble.rs` (P25) and `tests/analyze.rs` (P26) both run the pipeline over. [M29 P25]
//!
//! Every view is served from a cache the fixture writes (keypoints by the configured estimator,
//! strikes by this detector, both for the manifest's clip), and the shot from a current stored
//! shot, so nothing run over it poses, decodes or reads a photo: the [`sidecar`] points at no
//! worker, ffmpeg is missing and [`NoReads`] refuses to be asked. The clips are flat, so each view's
//! swing selection declines. Moved out of `tests/assemble.rs` at P26, unchanged but for `pub`.

// Each test binary that includes this uses its own part of it.
#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

use contracts::audio::{AudioClipMetadata, AudioFile, AudioStrike, AUDIO_DETECTOR_VERSION};
use contracts::keypoints::{ClipMetadata, FrameKeypoints, KeypointsFile, Landmark};
use contracts::shot::{ShotData, SCREEN_PARSER_VERSION};
use golf_lab::audio::frames_derived;
use golf_lab::pipeline::PipelineOptions;
use golf_lab::pose::Sidecar;
use golf_lab::shot::{PhotoReader, Prepared, ShotOptions};
use pose::{Config, PoolConfig};
use screen::profile::DeviceProfile;
use serde_json::{json, Value};
use storage::audio_io::{audio_path, save_audio};
use storage::bag_store::BagStore;
use storage::keypoints_io::keypoints_path;
use storage::manifest::Role;
use storage::shot_store::{hash_image, ShotStore};

pub const SESSION: &str = "2026-08-23";
pub const SWING: &str = "7";
pub const FACE_SHA: &str = "fa00000000000000000000000000000000000000000000000000000000000001";
pub const DTL_SHA: &str = "d700000000000000000000000000000000000000000000000000000000000002";
pub const PHOTO: &[u8] = b"a stand-in for the bay photo, which no case here reads";
pub const FPS: f64 = 60.0;
pub const FRAMES: usize = 30;
pub const RATE: i64 = 48_000;

pub fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub fn read_json(relative: &str) -> Value {
    serde_json::from_slice(&fs::read(repo().join(relative)).unwrap()).unwrap()
}

/// A reader that fails the case if asked anything: every shot here is served from the store.
pub struct NoReads;

impl PhotoReader for NoReads {
    fn ready(&mut self) -> Result<(), String> {
        panic!("a case asked the photo reader whether it is ready")
    }

    fn prepare(&mut self, photo: &Path, _: &DeviceProfile) -> Result<Prepared, String> {
        panic!("a case read {}", photo.display())
    }
}

/// A sidecar under the configured estimator whose worker is a script that does not exist, so a
/// case that reached it would fail on the refusal rather than pose anything.
pub fn sidecar() -> Sidecar {
    Sidecar::new(
        "heavy",
        PoolConfig {
            worker: Config {
                worker_args: vec![repo()
                    .join("crates/lab/tests/no-such-worker.py")
                    .display()
                    .to_string()],
                ..Config::default()
            },
            size: 1,
            queue_bound: 1,
        },
    )
}

/// What a view's fixture holds: its clip in the manifest, its keypoints cached, and the samples of
/// its cached strikes, `None` for a view with no audio cache.
#[derive(Clone)]
pub struct ViewSpec {
    pub keypoints: bool,
    pub strikes: Option<Vec<i64>>,
}

impl ViewSpec {
    pub fn heard(samples: &[i64]) -> Self {
        ViewSpec {
            keypoints: true,
            strikes: Some(samples.to_vec()),
        }
    }
}

/// One scratch bundle and the stores beside it.
pub struct Fixture {
    pub _root: tempfile::TempDir,
    pub swing_dir: PathBuf,
    pub options: PipelineOptions,
}

pub struct Spec {
    pub face_on: Option<ViewSpec>,
    pub down_the_line: Option<ViewSpec>,
    pub photo: bool,
    pub player: Option<&'static str>,
    pub club: Option<&'static str>,
}

impl Default for Spec {
    /// A complete bundle by `aaron`, hit with his `7i`: a strike heard face-on, none down the line.
    fn default() -> Self {
        Spec {
            face_on: Some(ViewSpec::heard(&[12_000])),
            down_the_line: Some(ViewSpec::heard(&[])),
            photo: true,
            player: Some("aaron"),
            club: Some("7i"),
        }
    }
}

pub fn role_entry(role: Role, filename: &str, sha: &str, kind: &str) -> Value {
    json!({
        "role": role.as_str(), "filename": filename, "content_sha256": sha,
        "original_filename": filename, "content_type": kind, "size_bytes": 1,
        "received_at": "2026-08-23T05:29:50.000000Z", "warnings": []
    })
}

/// A flat clip: every landmark still, so the hands never descend and each view's pick declines.
pub fn flat_keypoints(sha: &str) -> KeypointsFile {
    let landmark = Landmark {
        x: 0.5,
        y: 0.5,
        z: 0.0,
        visibility: 0.9,
    };
    KeypointsFile {
        clip: Some(ClipMetadata {
            fps: Some(FPS),
            width: Some(2160),
            height: Some(3840),
            frame_count: Some(FRAMES as i64),
            source_sha256: Some(sha.to_string()),
        }),
        frames: (0..FRAMES)
            .map(|index| FrameKeypoints {
                frame_index: index as i64,
                timestamp_ms: index as f64 * 1000.0 / FPS,
                landmarks: vec![landmark.clone(); contracts::keypoints::NUM_POSE_LANDMARKS],
                camera_id: None,
            })
            .collect(),
        pose_estimator: Some("mediapipe:heavy".to_string()),
    }
}

/// A strike cache for the clip `sha`, by this detector, its frames derived at [`FPS`] so a read at
/// that rate is a clean hit.
pub fn audio(sha: &str, samples: &[i64]) -> AudioFile {
    let detected = AudioFile {
        clip: Some(AudioClipMetadata {
            sample_rate: Some(RATE),
            duration_s: Some(FRAMES as f64 / FPS),
            source_sha256: Some(sha.to_string()),
            stream_index: Some(0),
            fps: None,
            video_start_s: Some(0.0),
        }),
        strikes: samples
            .iter()
            .map(|&sample| AudioStrike {
                sample,
                confidence: 0.9,
                prominence: 100.0,
                frame: None,
            })
            .collect(),
        detector_version: AUDIO_DETECTOR_VERSION,
    };
    frames_derived(&detected, Some(FPS)).unwrap_or(detected)
}

pub fn fixture(spec: Spec) -> Fixture {
    let root = tempfile::tempdir().unwrap();
    let swing_dir = root.path().join("sessions").join(SESSION).join(SWING);
    fs::create_dir_all(&swing_dir).unwrap();
    let mut roles = serde_json::Map::new();
    for (role, sha, view) in [
        (Role::FaceOn, FACE_SHA, &spec.face_on),
        (Role::DownTheLine, DTL_SHA, &spec.down_the_line),
    ] {
        let Some(view) = view else {
            continue;
        };
        let clip = format!("{}.{}.MOV", role.as_str(), &sha[..12]);
        roles.insert(
            role.as_str().into(),
            role_entry(role, &clip, sha, "video/quicktime"),
        );
        // The stand-in is written only beside a cache, so a view without one is a missing clip.
        if view.keypoints {
            fs::write(swing_dir.join(&clip), b"").unwrap();
            pose::writer::write(&flat_keypoints(sha), &keypoints_path(&swing_dir, role)).unwrap();
        }
        if let Some(samples) = &view.strikes {
            save_audio(&audio(sha, samples), &audio_path(&swing_dir, role)).unwrap();
        }
    }

    let shots = root.path().join("shots");
    if spec.photo {
        let sha = hash_image(PHOTO);
        let photo = format!("shot_screen.{}.jpg", &sha[..12]);
        fs::write(swing_dir.join(&photo), PHOTO).unwrap();
        roles.insert(
            "shot_screen".into(),
            role_entry(Role::ShotScreen, &photo, &sha, "image/jpeg"),
        );
        let mut shot =
            read_json("spec/vectors/screen/corpus/2026-08-23-2.json")["expected"]["shot"].clone();
        shot["provenance"]["image_sha256"] = json!(sha);
        shot["provenance"]["parser_version"] = json!(SCREEN_PARSER_VERSION);
        let shot: ShotData = serde_json::from_value(shot).unwrap();
        ShotStore::new(&shots).put(&shot).unwrap();
    }

    let mut manifest = json!({
        "swing_id": SWING, "session_id": SESSION,
        "created_at": "2026-08-23T05:29:40.000000Z", "updated_at": "2026-08-23T05:29:50.000000Z",
        "roles": roles,
    });
    if let Some(player) = spec.player {
        manifest["player_id"] = json!(player);
    }
    if let Some(club) = spec.club {
        manifest["club"] = json!(club);
    }
    fs::write(
        swing_dir.join("manifest.json"),
        serde_json::to_string(&manifest).unwrap(),
    )
    .unwrap();

    let golfers = root.path().join("golfers");
    fs::create_dir_all(&golfers).unwrap();
    fs::write(
        golfers.join("aaron.golfer.json"),
        r#"{"player_id": "aaron", "display_name": "Aaron", "handedness": "right",
            "created_at": "2026-08-07T15:04:05.123456Z"}"#,
    )
    .unwrap();
    let bags = read_json("spec/vectors/storage/stores/bag-reads.json");
    fs::write(
        BagStore::new(&golfers).path_for("aaron"),
        bags["input"]["files"]["aaron.bag.json"].as_str().unwrap(),
    )
    .unwrap();

    Fixture {
        options: PipelineOptions::new(ShotOptions::new(shots), golfers),
        swing_dir,
        _root: root,
    }
}
