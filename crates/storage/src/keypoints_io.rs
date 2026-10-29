//! Reading `{role}.keypoints.json`. `storage/keypoints_io.py`'s `load_keypoints`. [M29 P22]
//!
//! The one place that knows the on-disk shapes of pose output, as Python's is. Its caller is the
//! lab's pose cache (`api/pipeline.py::keypoints_for`, ported in M29 P22 as `golf_lab::pose`), which
//! reads a cached file here before deciding whether the clip behind it must be posed again.
//!
//! **Two shapes, as there.** The enveloped one, a [`KeypointsFile`] with the clip it came from, is
//! what every stored bundle holds. The legacy one is a bare array of frames, which
//! `scripts/run_pose.py` and the GolfDB extractor wrote before M7 Phase 1, and it reads as a
//! [`KeypointsFile`] with `clip: None` and no estimator, which is accurate: those files do not say
//! what clip measured them, so the cache rule reads one as another clip's and poses again.
//!
//! **Strict, not tolerant**, as [`crate::audio_io`] is and for the same reason: Python's
//! `load_keypoints` raises on a file it cannot read and `keypoints_for` lets it raise, so a broken
//! cache stops the bundle rather than being posed over. A missing or unreadable file is
//! [`StoreError::Io`]; text that is not JSON, or a value `contracts` refuses, is
//! [`StoreError::Unreadable`] in serde's words; JSON that is neither an object nor an array is
//! [`StoreError::Refused`] with Python's own message.
//!
//! **No writer here.** ADR-033 clause 8 has the sidecar's caller write the file, and
//! `pose::writer::write` is that writer, `save_keypoints`' twin with its measured equality to the 30
//! stored files (M23 P7). A second writer in this crate would be a second spelling of the artifact,
//! and `storage` cannot reach `pose` to share one (it depends on `contracts` and `pyfmt` alone).
//!
//! **One parse, not two.** The shape is told from the first byte that is not JSON whitespace, and
//! the file is parsed straight into its shape, where `json.loads` builds a tree and pydantic
//! validates it. A stored file is up to 14 MB of landmarks, and a `Value` tree of one is several
//! times that in memory for a dispatch one byte decides. Only a file that is neither shape is
//! parsed as a `Value`, to name what it held.

use std::fs;
use std::path::{Path, PathBuf};

use contracts::keypoints::{FrameKeypoints, KeypointsFile};
use serde_json::Value;

use crate::manifest::Role;
use crate::StoreError;

/// `swing_dir / f"{role.value}.keypoints.json"`, the name `keypoints_for` caches a view's pose
/// under. `pose::keypoints_path` names the same file from a free-form `camera_id`, which is the
/// sidecar's vocabulary; this takes the swing directory's.
pub fn keypoints_path(swing_dir: &Path, role: Role) -> PathBuf {
    swing_dir.join(format!("{}.keypoints.json", role.as_str()))
}

/// Load a keypoints file in either the enveloped or the legacy bare-array shape.
///
/// A legacy file comes back with `clip: None`, as Python's does. Every frame and landmark is held to
/// `contracts`' bounds on the way in, which is pydantic's validation.
pub fn load_keypoints(path: &Path) -> Result<KeypointsFile, StoreError> {
    let bytes = fs::read(path)?;
    let unreadable = |reason: String| StoreError::Unreadable {
        path: path.to_path_buf(),
        reason,
    };
    let first = bytes
        .iter()
        .find(|byte| !matches!(byte, b' ' | b'\t' | b'\n' | b'\r'));
    match first {
        Some(b'{') => serde_json::from_slice(&bytes).map_err(|e| unreadable(e.to_string())),
        Some(b'[') => {
            let frames: Vec<FrameKeypoints> =
                serde_json::from_slice(&bytes).map_err(|e| unreadable(e.to_string()))?;
            Ok(KeypointsFile {
                clip: None,
                frames,
                pose_estimator: None,
            })
        }
        _ => {
            let payload: Value =
                serde_json::from_slice(&bytes).map_err(|e| unreadable(e.to_string()))?;
            Err(StoreError::Refused(format!(
                "{}: expected a keypoints object or a bare frame array, got {}",
                path.display(),
                crate::python_type_name(&payload)
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FRAME: &str = r#"{"frame_index": 0, "timestamp_ms": 0.0,
        "landmarks": [{"x": 0.5, "y": 0.25, "z": -0.125, "visibility": 0.75}],
        "camera_id": "face_on"}"#;

    fn written(dir: &Path, text: &str) -> PathBuf {
        let path = keypoints_path(dir, Role::FaceOn);
        fs::write(&path, text).unwrap();
        path
    }

    #[test]
    fn the_enveloped_shape_reads_with_its_clip_and_estimator() {
        let dir = tempfile::tempdir().unwrap();
        let path = written(
            dir.path(),
            &format!(
                r#"{{"clip": {{"fps": 59.96, "width": 2160, "height": 3840, "frame_count": 1,
                   "source_sha256": "2183"}}, "frames": [{FRAME}],
                   "pose_estimator": "mediapipe:heavy"}}"#
            ),
        );
        assert!(path.ends_with("face_on.keypoints.json"));
        let read = load_keypoints(&path).unwrap();
        let clip = read.clip.expect("the envelope's clip");
        assert_eq!(clip.source_sha256.as_deref(), Some("2183"));
        assert_eq!(clip.frame_count, Some(1));
        assert_eq!(read.pose_estimator.as_deref(), Some("mediapipe:heavy"));
        assert_eq!(read.frames[0].landmarks[0].z, -0.125);
    }

    #[test]
    fn a_bare_array_is_the_legacy_shape_with_no_clip() {
        let dir = tempfile::tempdir().unwrap();
        let read = load_keypoints(&written(dir.path(), &format!("\n  [{FRAME}]"))).unwrap();
        assert_eq!(read.clip, None);
        assert_eq!(read.pose_estimator, None);
        assert_eq!(read.frames.len(), 1);
    }

    #[test]
    fn a_file_that_is_neither_shape_is_refused_with_pythons_message() {
        let dir = tempfile::tempdir().unwrap();
        let path = written(dir.path(), "\"frames\"");
        match load_keypoints(&path) {
            Err(StoreError::Refused(message)) => assert_eq!(
                message,
                format!(
                    "{}: expected a keypoints object or a bare frame array, got str",
                    path.display()
                )
            ),
            other => panic!("expected a refusal, got {other:?}"),
        }
    }

    #[test]
    fn a_broken_or_out_of_bounds_file_is_unreadable_and_a_missing_one_is_io() {
        let dir = tempfile::tempdir().unwrap();
        let truncated = written(dir.path(), r#"{"frames": [{"frame_index": 0"#);
        assert!(matches!(
            load_keypoints(&truncated),
            Err(StoreError::Unreadable { .. })
        ));
        // `visibility` above 1, which pydantic refuses and `contracts` refuses with it.
        let over = written(
            dir.path(),
            r#"[{"frame_index": 0, "timestamp_ms": 0.0,
                "landmarks": [{"x": 0.5, "y": 0.5, "visibility": 1.5}]}]"#,
        );
        assert!(matches!(
            load_keypoints(&over),
            Err(StoreError::Unreadable { .. })
        ));
        assert!(matches!(
            load_keypoints(&dir.path().join("absent.keypoints.json")),
            Err(StoreError::Io(_))
        ));
    }
}
