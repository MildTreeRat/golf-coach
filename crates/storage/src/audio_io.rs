//! Reading and writing `{role}.audio.json`. `storage/audio_io.py`. [M29 P20]
//!
//! The one place that knows the on-disk shape of a clip's strikes, as Python's is: every reader
//! comes through [`load_audio`] and every writer through [`save_audio`], rather than parsing
//! [`AudioFile`] itself. The lab's strike cache (`api/pipeline.py::audio_for`, ported in M29 P21)
//! is its caller.
//!
//! **Strict, not tolerant**, where [`crate::state`]'s readers answer `None`: Python's `load_audio`
//! raises on a file it cannot read, and `audio_for` lets it raise, so a broken cache stops the
//! bundle rather than being re-detected over. This answers the same file with the same verdict:
//!
//! - a missing or unreadable file is [`StoreError::Io`] (`read_bytes`' `OSError`);
//! - text that is not JSON, or an object pydantic would refuse, is [`StoreError::Unreadable`]
//!   (`JSONDecodeError`, `ValidationError`), in serde's words, which nothing compares;
//! - JSON that is not an object is [`StoreError::Refused`] with Python's own message, the one
//!   `isinstance` branch the module doc says a future legacy shape would be absorbed beside.
//!
//! Every verdict but one is Python's: a bare `NaN` or `Infinity`, which `json.loads` reads as a
//! float and so refuses as not an object, is not JSON to `serde_json` and is `Unreadable` here,
//! the laxness the crate doc says this crate refuses. Both refuse it, and no writer writes it.
//!
//! **No bare-array form**, as there is none in Python: audio was born enveloped (M11 P2), so a
//! legacy shape here would be one invented out of symmetry with `keypoints_io.py`.
//!
//! **The write is atomic, where Python's is not.** `save_audio` there is a `write_bytes`; here it
//! is [`crate::write_atomically`], a `.tmp` beside the file and a rename, as every store of this
//! crate writes. The value written is the same, and the difference is only what a reader sees if
//! the writer dies mid-file: a half-written cache, which the strict reader above would refuse and
//! stop the bundle on, against the old file whole. The `.tmp` is `{role}.audio.tmp`, which no
//! reader of a swing directory globs for.

use std::fs;
use std::path::{Path, PathBuf};

use contracts::audio::AudioFile;
use contracts::Validate;
use serde_json::Value;

use crate::manifest::Role;
use crate::StoreError;

/// `swing_dir / f"{role.value}.audio.json"`, the name `audio_for` caches a view's strikes under.
pub fn audio_path(swing_dir: &Path, role: Role) -> PathBuf {
    swing_dir.join(format!("{}.audio.json", role.as_str()))
}

/// Load an audio file. `clip` is `None` when the decode was not recorded, and an empty `strikes`
/// is an answer, not a missing one: the file only exists once detection has run.
pub fn load_audio(path: &Path) -> Result<AudioFile, StoreError> {
    let bytes = fs::read(path)?;
    let unreadable = |reason: String| StoreError::Unreadable {
        path: path.to_path_buf(),
        reason,
    };
    let payload: Value = serde_json::from_slice(&bytes).map_err(|e| unreadable(e.to_string()))?;
    if !payload.is_object() {
        return Err(StoreError::Refused(format!(
            "{}: expected an audio object, got {}",
            path.display(),
            crate::python_type_name(&payload)
        )));
    }
    serde_json::from_value(payload).map_err(|e| unreadable(e.to_string()))
}

/// Write an audio file, creating its directory as Python's `mkdir(parents=True)` does, with every
/// `None` left out (`exclude_none`, which [`AudioFile`]'s serialization carries).
///
/// **It validates first**, as [`crate::state::save_shot_analysis`] does: an [`AudioFile`] can be
/// assembled out of bounds in Rust where pydantic could not construct one, and writing it would
/// leave a cache [`load_audio`] refuses. It is [`StoreError::Invalid`] and nothing is written.
pub fn save_audio(audio: &AudioFile, path: &Path) -> Result<(), StoreError> {
    audio.validate().map_err(StoreError::Invalid)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    crate::write_atomically(audio, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use contracts::audio::{AudioClipMetadata, AudioStrike, AUDIO_DETECTOR_VERSION};

    fn audio() -> AudioFile {
        AudioFile {
            clip: Some(AudioClipMetadata {
                sample_rate: Some(48000),
                duration_s: Some(41.236),
                source_sha256: Some("e19f".into()),
                stream_index: Some(0),
                fps: Some(59.958766220641145),
                video_start_s: Some(0.0),
            }),
            strikes: vec![
                AudioStrike {
                    sample: 1260000,
                    confidence: 0.9436282670018675,
                    prominence: 19836910.03461857,
                    frame: Some(1573),
                },
                AudioStrike {
                    sample: 17760,
                    confidence: 0.8463355781675366,
                    prominence: 6526851.622130356,
                    frame: None,
                },
            ],
            detector_version: AUDIO_DETECTOR_VERSION,
        }
    }

    #[test]
    fn a_file_round_trips_and_leaves_no_tmp() {
        let dir = tempfile::tempdir().unwrap();
        let path = audio_path(&dir.path().join("2026-08-07/1"), Role::FaceOn);
        save_audio(&audio(), &path).unwrap();
        assert_eq!(load_audio(&path).unwrap(), audio());
        assert!(path.ends_with("2026-08-07/1/face_on.audio.json"));
        assert!(!path.with_extension("tmp").exists());
        let written: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(written["strikes"][1].get("frame"), None, "exclude_none");
    }

    #[test]
    fn an_empty_strike_list_is_an_answer() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("down_the_line.audio.json");
        fs::write(&path, r#"{"strikes": []}"#).unwrap();
        let read = load_audio(&path).unwrap();
        assert!(read.strikes.is_empty());
        assert_eq!((read.clip, read.detector_version), (None, 0));
    }

    #[test]
    fn json_that_is_not_an_object_is_refused_with_pythons_message() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("face_on.audio.json");
        for (text, name) in [
            ("[]", "list"),
            ("\"x\"", "str"),
            ("3", "int"),
            ("3.5", "float"),
            ("true", "bool"),
            ("null", "NoneType"),
        ] {
            fs::write(&path, text).unwrap();
            match load_audio(&path) {
                Err(StoreError::Refused(message)) => assert_eq!(
                    message,
                    format!("{}: expected an audio object, got {name}", path.display())
                ),
                other => panic!("{text}: {other:?}"),
            }
        }
    }

    #[test]
    fn a_missing_file_bad_json_and_a_broken_bound_each_fail_as_python_raises() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("face_on.audio.json");
        assert!(matches!(load_audio(&path), Err(StoreError::Io(_))));
        for text in [
            "{",
            // `json.loads` reads this as a float, so Python's verdict is the not-an-object
            // refusal; here it is not JSON at all (the module doc).
            "NaN",
            r#"{"strikes": [{"sample": -1, "confidence": 0.5, "prominence": 1.0}]}"#,
            r#"{"clip": {"fps": 0}, "strikes": []}"#,
            r#"{"clip": null}"#,
        ] {
            fs::write(&path, text).unwrap();
            assert!(
                matches!(load_audio(&path), Err(StoreError::Unreadable { .. })),
                "{text}"
            );
        }
    }

    #[test]
    fn an_out_of_bounds_file_is_never_written() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("face_on.audio.json");
        let mut bad = audio();
        bad.strikes[0].confidence = 1.5;
        match save_audio(&bad, &path) {
            Err(StoreError::Invalid(error)) => assert_eq!(
                error.to_string(),
                "AudioFile.strikes[0] -> AudioStrike.confidence: 1.5 is not <= 1"
            ),
            other => panic!("{other:?}"),
        }
        assert!(!path.exists() && !path.with_extension("tmp").exists());
    }

    #[test]
    fn unknown_keys_are_ignored_as_pydantic_ignores_them() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("face_on.audio.json");
        fs::write(
            &path,
            r#"{"strikes": [], "detector_version": 2, "later": 1}"#,
        )
        .unwrap();
        assert_eq!(load_audio(&path).unwrap().detector_version, 2);
    }
}
