//! Acoustic contracts: the ball strikes heard in one clip. `contracts/audio.py`. [M29 P20]
//!
//! The shape of `{role}.audio.json`, the strike cache `api/pipeline.py::audio_for` writes beside
//! each clip and `golf-lab` writes from M29 P21 on. It is as narrow as Python's on purpose: an
//! integer **sample index**, a confidence and a prominence per strike, plus what decode produced
//! them. No waveform crosses it.
//!
//! **Not `crates/trigger`'s `Strike`.** That crate has a Python caller (`api/pipeline.py` spawns
//! `golf-trigger`) and no edge to this one, and its `Strike` is what the detector hands back, with
//! no bounds and a `usize` sample. This is what a stored file holds, with pydantic's bounds as
//! runtime checks, so a file frozen Python would refuse is refused here too. The lab maps the one
//! to the other when it stamps a detection (P21).
//!
//! **Integers are `i64` with a bound, not `u64`**, as `keypoints::FrameKeypoints.frame_index` is: a
//! negative `sample` is then refused by [`Validate`] naming the field, the way pydantic's `ge=0`
//! names it, rather than by serde as a type error that names nothing.
//!
//! **What is written leaves every `None` out**, as `storage/audio_io.py::save_audio` dumps with
//! `exclude_none`: a strike whose frame is unknown carries no `"frame": null` line. That includes
//! `clip` itself. Absent and `null` both read back as `None`, in both languages.
//!
//! Its gate is the schema frozen Python exports, `spec/schemas/audio_file.schema.json`, which stays
//! pydantic's because no shape moves (`tests/python_schemas.rs` holds these structs to it), and every
//! file stored under `data/` on this box, which `crates/lab/tests/audio.rs` round-trips.

use serde::{Deserialize, Serialize};

use crate::{each, ge, gt, le, nested, ContractError, Validate};

/// Which generation of the strike detector a stored `{role}.audio.json` was detected by.
/// `contracts/audio.py::AUDIO_DETECTOR_VERSION`.
///
/// **It is frozen Python's number until M40, and `tests/python_schemas.rs` reads it out of
/// `contracts/audio.py` to say so.** `audio_for` re-detects any cache whose version differs from
/// its own, so a Rust bump would make the frozen lab re-detect and overwrite every file Rust wrote,
/// which ADR-035 clause 4 (the frozen lab reads what Rust writes) rules out while both run.
///
/// 0 → 1 (2026-08-30) introduced the field; 1 → 2 (2026-08-30) is `detect_strikes`' quarter-of-the-
/// loudest floor and the `video_start_s` re-probe. Python's comment carries the measurements.
pub const AUDIO_DETECTOR_VERSION: i64 = 2;

/// One transient the detector found, ranked among its siblings by `prominence`.
///
/// A candidate, not a verdict: the bay makes four transients per shot (club-ball contact, the club
/// on the mat, the ball on the screen, the simulator's own flight audio), and the screen strike is
/// often the louder of the first pair (docs/M11_ACOUSTIC_SYNC.md §E5). So the file keeps them all
/// and the caller decides which was the ball.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Self")]
pub struct AudioStrike {
    /// Offset into the decoded mono waveform, in samples, after the container's edit list has been
    /// applied. The measurement; everything else is derived from it.
    pub sample: i64,
    /// How sure the detector is this is a real onset, in `[0, 1]`.
    pub confidence: f64,
    /// How far this onset stands above its local background, in the detector's own units. Ranks
    /// candidates within one clip; `confidence` is the number to compare across clips.
    pub prominence: f64,
    /// The video frame this strike lands on, derived by the caller holding the clip's fps and its
    /// `video_start_s`. `None` means unknown, never frame 0 (ADR-010 §2): nobody knew the fps, or
    /// the strike sounded before the first frame the decoder hands back.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame: Option<i64>,
}

crate::validated!(AudioStrike);

impl Validate for AudioStrike {
    fn validate(&self) -> Result<(), ContractError> {
        ge("AudioStrike.sample", self.sample, 0)?;
        ge("AudioStrike.confidence", self.confidence, 0.0)?;
        le("AudioStrike.confidence", self.confidence, 1.0)?;
        ge("AudioStrike.prominence", self.prominence, 0.0)?;
        ge("AudioStrike.frame", self.frame, 0)
    }
}

/// What audio was decoded, recorded beside the strikes found in it.
///
/// Every field optional, as `keypoints::ClipMetadata`'s are: facts about a decode, and a strike
/// list can exist without them. `None` means unknown, never zero.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(remote = "Self")]
pub struct AudioClipMetadata {
    /// Samples per second of the waveform the strikes index into.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sample_rate: Option<i64>,
    /// Length of the waveform actually decoded, edit list included, which the container's own
    /// duration field does not account for (§E2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_s: Option<f64>,
    /// Content hash of the source video: the cache key `keypoints::ClipMetadata` uses too, so a
    /// re-uploaded clip invalidates both.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_sha256: Option<String>,
    /// Which audio stream was decoded. Face-on clips here carry two `soun` tracks with different
    /// edit offsets (§E2), so a sample index means something only beside its stream.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stream_index: Option<i64>,
    /// The frame rate every `AudioStrike.frame` on this file was derived under, so a stored frame
    /// index can be re-derived and checked.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fps: Option<f64>,
    /// Presentation time of the first frame the video decoder hands back, subtracted from every
    /// sample time before it becomes a frame. Almost always 0.0; 0.105–0.125 s on four down-the-line
    /// clips whose containers carry a leading empty edit (`audio/ffmpeg.py`'s
    /// `video_start_seconds`). `None` means nobody measured it, which is not a measured zero.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub video_start_s: Option<f64>,
}

crate::validated!(AudioClipMetadata);

impl Validate for AudioClipMetadata {
    fn validate(&self) -> Result<(), ContractError> {
        gt("AudioClipMetadata.sample_rate", self.sample_rate, 0)?;
        ge("AudioClipMetadata.duration_s", self.duration_s, 0.0)?;
        ge("AudioClipMetadata.stream_index", self.stream_index, 0)?;
        gt("AudioClipMetadata.fps", self.fps, 0.0)?;
        ge("AudioClipMetadata.video_start_s", self.video_start_s, 0.0)
    }
}

/// The on-disk shape of a `{role}.audio.json`: the strikes plus what decode produced them.
///
/// Read and written through `storage::audio_io`, as Python's goes through `storage/audio_io.py`, so
/// one place knows the file layout.
///
/// **An empty `strikes` list is a result, not a gap.** The file exists only once detection has run,
/// so empty means the detector ran and heard no transient, which is what a rehearsal swing sounds
/// like. "Never analysed" is the absence of the file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Self")]
pub struct AudioFile {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clip: Option<AudioClipMetadata>,
    pub strikes: Vec<AudioStrike>,
    /// Which generation of the detector found these ([`AUDIO_DETECTOR_VERSION`]). **Defaults to 0,
    /// not to the current version**, for `SwingBundleResult.analysis_version`'s reason: a file
    /// written before the field existed must read as older than current. No bound, as Python's has
    /// none: a negative version is merely not current.
    #[serde(default)]
    pub detector_version: i64,
}

crate::validated!(AudioFile);

impl Validate for AudioFile {
    fn validate(&self) -> Result<(), ContractError> {
        nested("AudioFile.clip", self.clip.as_ref())?;
        each("AudioFile.strikes", &self.strikes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    fn strike() -> Value {
        json!({"sample": 1260000, "confidence": 0.94, "prominence": 19836910.0, "frame": 1573})
    }

    fn file() -> Value {
        json!({
            "clip": {
                "sample_rate": 48000, "duration_s": 41.236, "source_sha256": "e19f",
                "stream_index": 0, "fps": 59.958766220641145, "video_start_s": 0.0,
            },
            "strikes": [strike()],
            "detector_version": 2,
        })
    }

    fn refusal(value: Value) -> String {
        serde_json::from_value::<AudioFile>(value)
            .unwrap_err()
            .to_string()
    }

    #[test]
    fn a_full_file_reads_and_writes_back_the_same_value() {
        let parsed: AudioFile = serde_json::from_value(file()).unwrap();
        assert_eq!(serde_json::to_value(&parsed).unwrap(), file());
    }

    #[test]
    fn every_strike_bound_is_refused_by_name() {
        for (key, value, problem) in [
            ("sample", json!(-1), "AudioStrike.sample: -1 is not >= 0"),
            (
                "confidence",
                json!(-0.1),
                "AudioStrike.confidence: -0.1 is not >= 0",
            ),
            (
                "confidence",
                json!(1.5),
                "AudioStrike.confidence: 1.5 is not <= 1",
            ),
            (
                "prominence",
                json!(-2.0),
                "AudioStrike.prominence: -2 is not >= 0",
            ),
            ("frame", json!(-3), "AudioStrike.frame: -3 is not >= 0"),
        ] {
            let mut bad = file();
            bad["strikes"][0][key] = value;
            // The strike validates in its own `Deserialize`, before the file sees it, so a parse
            // names the strike's field without the file's path; `validate` on a built file adds it.
            let error = refusal(bad);
            assert!(error.contains(problem), "{key}: {error}");
        }
    }

    #[test]
    fn every_clip_bound_is_refused_by_name() {
        for (key, value, problem) in [
            ("sample_rate", json!(0), "sample_rate: 0 is not > 0"),
            ("duration_s", json!(-1.0), "duration_s: -1 is not >= 0"),
            ("stream_index", json!(-1), "stream_index: -1 is not >= 0"),
            ("fps", json!(0.0), "fps: 0 is not > 0"),
            (
                "video_start_s",
                json!(-0.1),
                "video_start_s: -0.1 is not >= 0",
            ),
        ] {
            let mut bad = file();
            bad["clip"][key] = value;
            let error = refusal(bad);
            assert!(
                error.contains(&format!("AudioClipMetadata.{problem}")),
                "{key}: {error}"
            );
        }
    }

    #[test]
    fn the_bounds_admit_their_edges() {
        // `ge` is inclusive and `gt` is not: a confidence of exactly 0 or 1, a zero sample, a zero
        // stream and a zero video start all read, as pydantic reads them.
        let mut edges = file();
        edges["strikes"][0] =
            json!({"sample": 0, "confidence": 1.0, "prominence": 0.0, "frame": 0});
        edges["clip"]["duration_s"] = json!(0.0);
        assert!(serde_json::from_value::<AudioFile>(edges).is_ok());
        let mut zero = file();
        zero["strikes"][0]["confidence"] = json!(0.0);
        assert!(serde_json::from_value::<AudioFile>(zero).is_ok());
    }

    #[test]
    fn a_minimal_file_reads_with_pythons_defaults() {
        let parsed: AudioFile = serde_json::from_value(json!({"strikes": []})).unwrap();
        assert_eq!(parsed.clip, None);
        assert!(parsed.strikes.is_empty());
        // 0, not the current version: a legacy file reads as stale.
        assert_eq!(parsed.detector_version, 0);
        assert!(serde_json::from_value::<AudioFile>(json!({})).is_err());
    }

    #[test]
    fn every_none_is_left_out_of_what_is_written() {
        let quiet = AudioFile {
            clip: Some(AudioClipMetadata {
                sample_rate: Some(48000),
                ..AudioClipMetadata::default()
            }),
            strikes: vec![AudioStrike {
                sample: 10,
                confidence: 0.5,
                prominence: 1.0,
                frame: None,
            }],
            detector_version: AUDIO_DETECTOR_VERSION,
        };
        assert_eq!(
            serde_json::to_value(&quiet).unwrap(),
            json!({
                "clip": {"sample_rate": 48000},
                "strikes": [{"sample": 10, "confidence": 0.5, "prominence": 1.0}],
                "detector_version": 2,
            })
        );
        let bare = AudioFile {
            clip: None,
            ..quiet
        };
        assert!(serde_json::to_value(&bare).unwrap().get("clip").is_none());
    }

    #[test]
    fn a_built_file_names_the_path_to_what_it_refuses() {
        let mut built: AudioFile = serde_json::from_value(file()).unwrap();
        built.strikes[0].sample = -1;
        assert_eq!(
            built.validate().unwrap_err().to_string(),
            "AudioFile.strikes[0] -> AudioStrike.sample: -1 is not >= 0"
        );
        built.strikes[0].sample = 0;
        built.clip.as_mut().unwrap().fps = Some(-60.0);
        assert_eq!(
            built.validate().unwrap_err().to_string(),
            "AudioFile.clip -> AudioClipMetadata.fps: -60 is not > 0"
        );
    }

    #[test]
    fn nan_is_refused_rather_than_compared() {
        let strike = AudioStrike {
            sample: 0,
            confidence: f64::NAN,
            prominence: 0.0,
            frame: None,
        };
        assert!(strike.validate().is_err());
    }
}
