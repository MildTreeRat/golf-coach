//! Body pose contracts. `contracts/keypoints.py`. [M22 P2]
//!
//! These mirror the MediaPipe Pose output: 33 landmarks per frame. Nothing here depends on
//! MediaPipe — and under ADR-030 nothing in Rust ever will, because pose is the one thing that
//! stays Python (§2's sidecar). This crate holds the shape its landmarks arrive in.

use serde::{Deserialize, Serialize};

use crate::{each, ge, gt, nested, ContractError, Validate};

/// The 33 MediaPipe Pose landmarks, in MediaPipe's canonical index order.
///
/// Use these names instead of magic indices. The wrist landmarks matter most for golf — they
/// anchor the club shaft, and `phases.py` segments the swing on one of them.
///
/// Never serialized: a frame's landmarks are a positional list and this is how it is indexed.
/// `#[repr(u8)]` so `PoseLandmark::LeftWrist as usize` is the index, matching the Python
/// `IntEnum`'s one use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum PoseLandmark {
    Nose = 0,
    LeftEyeInner = 1,
    LeftEye = 2,
    LeftEyeOuter = 3,
    RightEyeInner = 4,
    RightEye = 5,
    RightEyeOuter = 6,
    LeftEar = 7,
    RightEar = 8,
    MouthLeft = 9,
    MouthRight = 10,
    LeftShoulder = 11,
    RightShoulder = 12,
    LeftElbow = 13,
    RightElbow = 14,
    LeftWrist = 15,
    RightWrist = 16,
    LeftPinky = 17,
    RightPinky = 18,
    LeftIndex = 19,
    RightIndex = 20,
    LeftThumb = 21,
    RightThumb = 22,
    LeftHip = 23,
    RightHip = 24,
    LeftKnee = 25,
    RightKnee = 26,
    LeftAnkle = 27,
    RightAnkle = 28,
    LeftHeel = 29,
    RightHeel = 30,
    LeftFootIndex = 31,
    RightFootIndex = 32,
}

pub const NUM_POSE_LANDMARKS: usize = 33;

/// A single body landmark in normalized image coordinates.
///
/// x/y are normalized to `[0, 1]` relative to image width/height (MediaPipe convention). z is depth
/// relative to the hips (roughly normalized, smaller = closer to camera). `visibility` is
/// MediaPipe's confidence the landmark is present and not occluded.
///
/// **x and y are deliberately unbounded**, exactly as in the Python: MediaPipe reports landmarks
/// outside the frame rather than clamping them, and a golfer whose hands leave the top of a phone
/// video is an ordinary clip rather than a malformed one.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Self")]
pub struct Landmark {
    pub x: f64,
    pub y: f64,
    #[serde(default)]
    pub z: f64,
    #[serde(default)]
    pub visibility: f64,
}

crate::validated!(Landmark);

impl Validate for Landmark {
    fn validate(&self) -> Result<(), ContractError> {
        ge("Landmark.visibility", self.visibility, 0.0)?;
        crate::le("Landmark.visibility", self.visibility, 1.0)
    }
}

/// All body landmarks for one video frame.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Self")]
pub struct FrameKeypoints {
    pub frame_index: i64,
    /// Milliseconds from start of clip.
    pub timestamp_ms: f64,
    /// Exactly [`NUM_POSE_LANDMARKS`] landmarks, indexed by [`PoseLandmark`].
    pub landmarks: Vec<Landmark>,
    /// Which camera produced this frame (ADR-011). `None` means not recorded — the state of every
    /// file written before M7 Phase 1. Free-form by design: the value is whatever the source was
    /// told it is.
    #[serde(default)]
    pub camera_id: Option<String>,
}

crate::validated!(FrameKeypoints);

impl FrameKeypoints {
    /// Convenience accessor, mirroring the Python `landmark()`.
    pub fn landmark(&self, which: PoseLandmark) -> &Landmark {
        &self.landmarks[which as usize]
    }
}

impl Validate for FrameKeypoints {
    fn validate(&self) -> Result<(), ContractError> {
        ge("FrameKeypoints.frame_index", self.frame_index, 0)?;
        ge("FrameKeypoints.timestamp_ms", self.timestamp_ms, 0.0)?;
        // The count is a *description* in the Python (`Field(description=…)`) and not a bound, so
        // it is not one here either. Corpus clips really do carry frames MediaPipe found nobody in,
        // and rejecting them would refuse artifacts this repo has on disk.
        each("FrameKeypoints.landmarks", &self.landmarks)
    }
}

/// What the source clip was, recorded alongside the pose extracted from it.
///
/// Every field is optional: these are facts about a video file, and a frame list can legitimately
/// exist without one (synthetic fixtures, a corpus whose videos are long gone). `None` means
/// unknown, never zero.
///
/// **`fps` is the container's claim, not measured real time.** It is `CAP_PROP_FPS` as reported,
/// which iPhone slo-mo is known to stretch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Self")]
pub struct ClipMetadata {
    /// Container-reported frames/second.
    #[serde(default)]
    pub fps: Option<f64>,
    #[serde(default)]
    pub width: Option<i64>,
    #[serde(default)]
    pub height: Option<i64>,
    /// Frames actually DECODED, which is the honest number — a container can claim more frames than
    /// it yields, and a silently-short decode is the failure mode this records.
    #[serde(default)]
    pub frame_count: Option<i64>,
    /// Content hash of the source video, tying pose back to its clip.
    #[serde(default)]
    pub source_sha256: Option<String>,
}

crate::validated!(ClipMetadata);

impl Validate for ClipMetadata {
    fn validate(&self) -> Result<(), ContractError> {
        gt("ClipMetadata.fps", self.fps, 0.0)?;
        gt("ClipMetadata.width", self.width, 0)?;
        gt("ClipMetadata.height", self.height, 0)?;
        ge("ClipMetadata.frame_count", self.frame_count, 0)
    }
}

/// The on-disk shape of a `*.keypoints.json`: the frames plus what clip they came from.
///
/// Files written before M7 Phase 1 are a bare JSON array of frames with no envelope. The tolerant
/// reader for those is `storage/keypoints_io.py`, which is lab code — [ADR-032 §8](../../../docs/decisions/032-the-rust-core.md)
/// keeps storage in Python for M22, so this type reads the enveloped form only. The vectors are all
/// enveloped, which is what the round-trip gate checks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Self")]
pub struct KeypointsFile {
    #[serde(default)]
    pub clip: Option<ClipMetadata>,
    pub frames: Vec<FrameKeypoints>,
    /// Which pose estimator measured these landmarks, e.g. `mediapipe:heavy`. **`None` means
    /// unknown, not lite**: every file written before this field existed reads `None`, and the
    /// reader re-runs pose on one rather than guessing which bundle produced it. Landmarks from two
    /// variants are not interchangeable — they move the phase instants and therefore every score
    /// built on them.
    #[serde(default)]
    pub pose_estimator: Option<String>,
}

crate::validated!(KeypointsFile);

impl Validate for KeypointsFile {
    fn validate(&self) -> Result<(), ContractError> {
        nested("KeypointsFile.clip", self.clip.as_ref())?;
        each("KeypointsFile.frames", &self.frames)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn landmark_indices_match_mediapipe() {
        assert_eq!(PoseLandmark::Nose as usize, 0);
        assert_eq!(PoseLandmark::LeftWrist as usize, 15);
        assert_eq!(
            PoseLandmark::RightFootIndex as usize,
            NUM_POSE_LANDMARKS - 1
        );
    }

    #[test]
    fn visibility_is_bounded_at_both_ends() {
        let over = r#"{"x":0.5,"y":0.5,"visibility":1.5}"#;
        let under = r#"{"x":0.5,"y":0.5,"visibility":-0.1}"#;
        assert!(serde_json::from_str::<Landmark>(over).is_err());
        assert!(serde_json::from_str::<Landmark>(under).is_err());
    }

    #[test]
    fn a_bad_landmark_deep_in_a_file_names_its_path() {
        let file = KeypointsFile {
            clip: None,
            pose_estimator: None,
            frames: vec![FrameKeypoints {
                frame_index: 0,
                timestamp_ms: 0.0,
                camera_id: None,
                landmarks: vec![Landmark {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                    visibility: 2.0,
                }],
            }],
        };
        let err = file.validate().unwrap_err();
        assert_eq!(
            err.field,
            "KeypointsFile.frames[0] -> FrameKeypoints.landmarks[0] -> Landmark.visibility"
        );
    }

    #[test]
    fn a_negative_frame_index_is_refused() {
        let json = r#"{"frame_index":-1,"timestamp_ms":0.0,"landmarks":[]}"#;
        assert!(serde_json::from_str::<FrameKeypoints>(json).is_err());
    }

    #[test]
    fn clip_metadata_refuses_a_zero_frame_rate_and_allows_zero_frames() {
        assert!(serde_json::from_str::<ClipMetadata>(r#"{"fps":0.0}"#).is_err());
        assert!(serde_json::from_str::<ClipMetadata>(r#"{"frame_count":0}"#).is_ok());
    }
}
