//! Writing the `{camera_id}.keypoints.json` a worker's landmarks belong in. [M23 P6]
//!
//! ADR-033 clause 8: the worker takes a path and returns landmarks, and **the caller writes the
//! file** — the same division `crates/capture` already has, where capture owns the directory and the
//! detector only detects. This module is that caller's half, and it is the first Rust in this repo
//! that writes an artifact at all (`golf-core run` writes to stdout; nothing else writes anywhere).
//!
//! # What it has to reproduce, and what it deliberately does not
//!
//! `storage/keypoints_io.py::save_keypoints` writes `model_dump_json(indent=2, exclude_none=True)`.
//! Three properties of that matter here, and the third is not the one the plan for this phase
//! expected:
//!
//! - **An absent value is an absent key, not a null.** That docstring gives the reason — a clip with
//!   no `camera_id` would otherwise carry several hundred `"camera_id": null` lines saying nothing —
//!   and [`to_json`] reproduces it, so a file this writes and a file `save_keypoints` writes have the
//!   same *keys* and not merely the same values.
//! - **Two-space indent, and no trailing newline.** `model_dump_json` returns no newline and
//!   `save_keypoints` adds none; the committed corpus files end on their closing brace. Matched
//!   because it is free, not because anything reads it: see below.
//!
//! The third property is the one **ADR-033 measurement 4 gets wrong**, and P6 measured it the other
//! way. That measurement says the stored files *"carry full CPython float repr"*, so a Rust writer's
//! floats could never match them. **They do not carry it**: `model_dump_json` serializes inside **pydantic-core,
//! which is Rust and uses `serde_json`** — so `save_keypoints` has been writing ryu's float form all
//! along. `1.8422693756292574e-05` is written `0.000018422693756292574` by pydantic and
//! `1.8422693756292574e-05` by `json.dumps`, and the committed
//! `2026-08-23/11/face_on.keypoints.json` spells it the first way. It contains **zero** exponent-form
//! literals despite holding four values small enough for CPython to write one.
//!
//! So the two measurements that look like they disagree are about two different writers, which is the
//! whole resolution: ADR-032's second addendum found 77 exponent-form floats in `spec/vectors/`, and
//! those are written by `conformance.py`'s `json.dumps` — CPython. The *keypoints artifact* has a Rust
//! writer on both sides of the boundary and always did.
//!
//! **What that buys, measured over the whole corpus** (P7, 30 clips; P6 measured one): the files this
//! writes and the files the Python pipeline wrote are the same byte length — **239,214,827 bytes on
//! each side** — and once the trailing comma is stripped their sorted lines are identical on
//! **30/30**. Every float literal in 239 MB is spelled the same way by both writers. The comma needs
//! stripping because key *order* is the one real difference, for the reason below: a different key is
//! last in each object, so the comma moves with it (`"width": 2160` against `"width": 2160,`). It
//! still does not make the comparison a byte comparison:
//! ADR-033 clause 8 calls for a structural one, nothing compares these files as text (the pose cache
//! keys on the **clip's** sha256, never the keypoints file's), and a claim resting on pydantic-core's
//! choice of float formatter is a claim that a dependency bump can retract.
//!
//! # Why the nulls are dropped here rather than on the shared struct
//!
//! The plan for this phase proposed `skip_serializing_if = "Option::is_none"` on
//! `contracts::keypoints`' nullable fields, and pre-authorised this module as the fix if that broke a
//! `contracts` round trip. **It breaks it, measured: 397 dropped keys** — 391 `camera_id` and 6
//! `clip.source_sha256` across the six synthetic vectors, which carry those two spelled `null` where
//! the corpus vectors carry values. `crates/contracts/tests/round_trip.rs` distinguishes an absent
//! key from an explicit `null` on purpose (M22 P2 caught its own harness flattening the two), so a
//! shared struct that dropped them would rewrite every one of those vectors while passing.
//!
//! The two demands are therefore real and opposed: the vector round trip needs `null` in and `null`
//! out, and this writer needs `None` to vanish. Serializing through a [`Value`] and dropping the
//! nulls afterwards satisfies both and leaves the shared shape alone.
//!
//! **A mirror struct per shape, with `skip_serializing_if` on its own fields, was the alternative and
//! was declined**: three structs and eleven field names copied out of `contracts`, which is a second
//! copy of a shape that a field added upstream would be silently dropped by. The [`Value`] walk works
//! from whatever `Serialize` emits, so it cannot fall behind the struct it serializes.
//!
//! **The cost of that choice, stated rather than discovered, and it is the last difference left.**
//! `serde_json::Map` is a `BTreeMap` without the `preserve_order` feature, so the keys come out
//! **alphabetical** where pydantic's come out in field-declaration order — `camera_id` before
//! `frame_index` in a frame, `frame_count` before `width` in the envelope. Since the floats agree,
//! this is now the *only* thing standing between the two files and byte equality, and turning on
//! `preserve_order` would close it: an `IndexMap` in insertion order is the declaration order.
//!
//! **Declined here anyway, and deliberately left to an ADR rather than taken by a phase.** It is a
//! workspace-wide swap of every `Value`'s map type, made for a property ADR-033 clause 8 states this
//! repo does not need — and byte equality of an artifact nothing hashes is the sort of guarantee that
//! gets quietly depended on and then broken by a dependency bump. If a later milestone wants it, the
//! argument belongs in an addendum to clause 8 beside the measurement above, not in this module.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use contracts::keypoints::KeypointsFile;
use serde_json::Value;

/// What a keypoints artifact is called, after the camera that produced it.
///
/// A constant rather than a format string at two sites, because `api/pipeline.py` and
/// `data/README.md` both name this shape and a third spelling of it is a third thing to keep in step.
pub const KEYPOINTS_SUFFIX: &str = ".keypoints.json";

/// Where one camera's landmarks go inside a swing directory: `{camera_id}.keypoints.json`.
///
/// `camera_id` and not a `Role`, deliberately. The field is free-form by design in
/// `contracts::keypoints` — "whatever the source was told it is" — and the corpus's two values,
/// `face_on` and `down_the_line`, are the file names on disk today. A `Role` enum belongs to whoever
/// owns the swing directory, which under ADR-033 clause 8 is M24 and not this crate.
pub fn keypoints_path(dir: &Path, camera_id: &str) -> PathBuf {
    dir.join(format!("{camera_id}{KEYPOINTS_SUFFIX}"))
}

/// The JSON `save_keypoints` would have written for these landmarks: indent 2, no key for a `None`.
///
/// Returns a `String` rather than writing as it serializes, and that is the failure mode talking: a
/// serialization error part-way through a streaming write leaves a truncated file where the previous
/// one was, and this way the only thing that can fail after the first byte is the filesystem.
pub fn to_json(keypoints: &KeypointsFile) -> Result<String, serde_json::Error> {
    let mut value = serde_json::to_value(keypoints)?;
    drop_nulls(&mut value);
    serde_json::to_string_pretty(&value)
}

/// pydantic's `exclude_none=True`, over a parsed value: a key whose value is null is not written.
///
/// Recursive and applied everywhere, exactly as `exclude_none` is — including inside `landmarks`,
/// where nothing is nullable today, because a writer that only knew about the three fields that are
/// nullable *now* is the drift this module's doc declines a mirror struct to avoid.
///
/// A null *inside an array* is left alone, which is also what pydantic does: an array's elements are
/// values rather than keys, and dropping one would shift every index after it.
fn drop_nulls(value: &mut Value) {
    match value {
        Value::Object(map) => {
            map.retain(|_, field| !field.is_null());
            for field in map.values_mut() {
                drop_nulls(field);
            }
        }
        Value::Array(items) => {
            for item in items {
                drop_nulls(item);
            }
        }
        _ => {}
    }
}

/// Write these landmarks to `path`, creating its parent directory if it is missing.
///
/// Mirrors `save_keypoints` down to the `mkdir parents=True`: a swing directory that does not exist
/// yet is the ordinary case for the first artifact written into it.
///
/// **Not written to a temporary file and renamed**, and that matches the Python it replaces rather
/// than merely being simpler. A process killed mid-write leaves invalid JSON, which
/// `load_keypoints` refuses loudly; the failure a rename would protect against is a *silently short*
/// file, and JSON cannot be silently short — its last byte is a brace. Whoever owns the swing
/// directory (M24) is the right place for an atomic-publish rule if one is ever wanted, because it
/// would have to cover the clip and the analysis too.
pub fn write(keypoints: &KeypointsFile, path: &Path) -> io::Result<()> {
    let json = to_json(keypoints).map_err(io::Error::other)?;
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, json.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use contracts::keypoints::{ClipMetadata, FrameKeypoints, Landmark};

    fn frame(camera_id: Option<&str>) -> FrameKeypoints {
        FrameKeypoints {
            frame_index: 0,
            timestamp_ms: 0.0,
            landmarks: vec![Landmark {
                x: 0.5,
                y: 0.25,
                z: -0.125,
                visibility: 0.5,
            }],
            camera_id: camera_id.map(str::to_string),
        }
    }

    #[test]
    fn an_absent_value_is_an_absent_key_and_never_a_null() {
        let bare = KeypointsFile {
            clip: None,
            frames: vec![frame(None)],
            pose_estimator: None,
        };
        let json = to_json(&bare).expect("serialize");
        assert!(!json.contains("null"), "{json}");
        assert!(!json.contains("clip"), "{json}");
        assert!(!json.contains("camera_id"), "{json}");
        assert!(!json.contains("pose_estimator"), "{json}");
        // And the frames are still there — a writer that dropped the whole envelope would also pass
        // the three assertions above.
        assert!(json.contains("\"frames\""), "{json}");
    }

    #[test]
    fn a_present_value_is_written_even_when_a_sibling_is_absent() {
        // `ClipMetadata` with one field filled is the shape that catches an `exclude_none` applied at
        // the wrong depth: drop the parent and `fps` goes with it.
        let partial = KeypointsFile {
            clip: Some(ClipMetadata {
                fps: Some(59.9651365485183),
                width: None,
                height: None,
                frame_count: Some(344),
                source_sha256: None,
            }),
            frames: vec![frame(Some("face_on"))],
            pose_estimator: Some("mediapipe:heavy".to_string()),
        };
        let json = to_json(&partial).expect("serialize");
        assert!(!json.contains("null"), "{json}");
        assert!(json.contains("\"fps\": 59.9651365485183"), "{json}");
        assert!(json.contains("\"frame_count\": 344"), "{json}");
        assert!(!json.contains("width"), "{json}");
        assert!(json.contains("\"camera_id\": \"face_on\""), "{json}");
    }

    #[test]
    fn the_indent_is_two_spaces_and_there_is_no_trailing_newline() {
        // `save_keypoints` writes `model_dump_json(indent=2)`, which returns no newline, and the
        // committed corpus files end on their closing brace.
        let json = to_json(&KeypointsFile {
            clip: None,
            frames: vec![frame(None)],
            pose_estimator: None,
        })
        .expect("serialize");
        assert!(json.starts_with("{\n  \"frames\": [\n    {\n"), "{json}");
        assert!(json.ends_with('}'), "ends {:?}", &json[json.len() - 8..]);
    }

    #[test]
    fn the_artifact_is_named_after_the_camera() {
        let path = keypoints_path(Path::new("sessions/2026-08-23/11"), "face_on");
        assert_eq!(path.file_name().unwrap(), "face_on.keypoints.json");
    }

    #[test]
    fn writing_creates_the_swing_directory_and_the_file_reads_back_equal() {
        let dir = std::env::temp_dir().join(format!("golf-pose-p6-{}", std::process::id()));
        // A nested child, so the `create_dir_all` is doing more than one level.
        let target = keypoints_path(&dir.join("11"), "face_on");
        let written = KeypointsFile {
            clip: Some(ClipMetadata {
                fps: Some(59.9651365485183),
                width: Some(2160),
                height: Some(3840),
                frame_count: Some(1),
                source_sha256: Some("21831919bc67".to_string()),
            }),
            frames: vec![frame(Some("face_on"))],
            pose_estimator: Some("mediapipe:heavy".to_string()),
        };
        write(&written, &target).expect("write");
        let text = fs::read_to_string(&target).expect("read back");
        let parsed: KeypointsFile = serde_json::from_str(&text).expect("parse back");
        assert_eq!(parsed, written);
        let _ = fs::remove_dir_all(&dir);
    }
}
