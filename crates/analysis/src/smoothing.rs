//! `analysis/smoothing.py`. Landmark temporal smoothing. [M22 P4]
//!
//! MediaPipe landmark trajectories jitter frame to frame, and the phase instants tempo depends on
//! are read straight off those trajectories, so a few pixels of noise can move the detected top by
//! several frames. This denoises the timeline *once*, up front, so [`crate::phases`] and every
//! measurement downstream read one stable signal.
//!
//! The filter is a small centered moving average, **visibility-weighted**: each smoothed coordinate
//! is the mean of its neighbours within a half-window, weighting every sample by MediaPipe's
//! `visibility` so a low-confidence landmark contributes little. Only `x`/`y` are denoised.
//!
//! # The two things a port gets wrong here
//!
//! The arithmetic is a sum of products in frame order and the sum order is the answer: `RTOL = 1e-9`
//! in `docs/CONFORMANCE.md` §3 is sized for exactly this, a float added up in a different sequence,
//! so the loop below accumulates left to right the way the Python does rather than reaching for
//! anything cleverer.
//!
//! And the output **drops `camera_id`**. The Python builds its `FrameKeypoints` without one, so a
//! smoothed timeline has no camera on it even when the input did — which is why `conformance.py`'s
//! `run_stages` reads the camera off the *unsmoothed* frames. It is faithfully reproduced here
//! rather than quietly improved: `alignment.py` (P7) reads that field, and a port that carried it
//! through would hand P7 a camera id where Python hands it a `None`.

use contracts::keypoints::{FrameKeypoints, Landmark, NUM_POSE_LANDMARKS};

/// Total window (frames) of the centered moving average. Odd so it is symmetric about the frame
/// being smoothed; 5 -> two neighbours each side. Small: enough to kill single-frame jitter without
/// blurring the genuine motion the swing traces in ~10-40 frames.
pub const DEFAULT_WINDOW: usize = 5;

/// A sample dimmer than this still counts, but a window whose *total* weight falls below this is
/// treated as "no confident samples" and the raw coordinate is kept rather than one invented.
const MIN_TOTAL_WEIGHT: f64 = 1e-6;

/// A new, temporally smoothed copy of a keypoint timeline.
///
/// `frame_index`, `timestamp_ms` and each landmark's `z`/`visibility` are preserved exactly — that
/// pass-through is what `spec/vectors/stages/`'s `smoothed` stage asserts rather than records, and
/// it is what makes that stage 2.3 MB smaller than the input it is taken from. A `window <= 1` (or a
/// timeline too short to smooth) returns an equivalent copy unchanged.
pub fn smooth_keypoints(keypoints: &[FrameKeypoints], window: usize) -> Vec<FrameKeypoints> {
    let n = keypoints.len();
    if n == 0 || window <= 1 {
        // The Python's `model_copy(deep=True)`, which keeps `camera_id` — this branch copies rather
        // than rebuilding, so unlike the smoothing path below it does not drop the field. The
        // asymmetry is the Python's and is preserved: an unsmoothable clip is handed back as it
        // arrived.
        return keypoints.to_vec();
    }

    let half = window / 2;
    let mut smoothed = Vec::with_capacity(n);
    for (i, frame) in keypoints.iter().enumerate() {
        let lo = i.saturating_sub(half);
        let hi = (i + half + 1).min(n);
        let mut landmarks = Vec::with_capacity(NUM_POSE_LANDMARKS);
        for lm_index in 0..NUM_POSE_LANDMARKS {
            let original = &frame.landmarks[lm_index];
            let (mut sx, mut sy, mut weight) = (0.0, 0.0, 0.0);
            for sample in keypoints[lo..hi].iter().map(|f| &f.landmarks[lm_index]) {
                let w = sample.visibility;
                sx += sample.x * w;
                sy += sample.y * w;
                weight += w;
            }
            if weight < MIN_TOTAL_WEIGHT {
                // No confident neighbours — trust the raw value over a fabricated average.
                landmarks.push(original.clone());
            } else {
                landmarks.push(Landmark {
                    x: sx / weight,
                    y: sy / weight,
                    z: original.z,
                    visibility: original.visibility,
                });
            }
        }
        smoothed.push(FrameKeypoints {
            frame_index: frame.frame_index,
            timestamp_ms: frame.timestamp_ms,
            landmarks,
            camera_id: None,
        });
    }
    smoothed
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(index: i64, x: f64, y: f64, visibility: f64) -> FrameKeypoints {
        FrameKeypoints {
            frame_index: index,
            timestamp_ms: index as f64 * 10.0,
            camera_id: Some("cam".to_string()),
            landmarks: vec![
                Landmark {
                    x,
                    y,
                    z: 0.25,
                    visibility,
                };
                NUM_POSE_LANDMARKS
            ],
        }
    }

    #[test]
    fn a_window_of_one_returns_the_timeline_untouched() {
        let frames = vec![frame(0, 0.1, 0.2, 1.0), frame(1, 0.9, 0.8, 1.0)];
        assert_eq!(smooth_keypoints(&frames, 1), frames);
        assert_eq!(smooth_keypoints(&frames, 0), frames);
    }

    #[test]
    fn an_empty_timeline_smooths_to_an_empty_one() {
        assert!(smooth_keypoints(&[], DEFAULT_WINDOW).is_empty());
    }

    #[test]
    fn a_spike_is_pulled_toward_its_neighbours() {
        let frames: Vec<_> = [0.0, 0.0, 1.0, 0.0, 0.0]
            .iter()
            .enumerate()
            .map(|(i, &x)| frame(i as i64, x, 0.0, 1.0))
            .collect();
        let out = smooth_keypoints(&frames, DEFAULT_WINDOW);
        assert_eq!(
            out[2].landmarks[0].x, 0.2,
            "mean of five, one of which is 1.0"
        );
    }

    /// The weighting is the point of the filter: a dim neighbour must not drag a confident frame.
    #[test]
    fn visibility_weights_the_average() {
        let frames = vec![
            frame(0, 1.0, 0.0, 0.01),
            frame(1, 0.0, 0.0, 1.0),
            frame(2, 1.0, 0.0, 0.01),
        ];
        let x = smooth_keypoints(&frames, 3)[1].landmarks[0].x;
        assert!(
            x < 0.02,
            "a 0.01-visibility pair should barely move it, got {x}"
        );
    }

    #[test]
    fn a_window_with_no_confident_samples_keeps_the_raw_coordinate() {
        let frames = vec![frame(0, 0.42, 0.24, 0.0), frame(1, 0.99, 0.11, 0.0)];
        let out = smooth_keypoints(&frames, 3);
        assert_eq!(out[0].landmarks[0].x, 0.42);
        assert_eq!(out[1].landmarks[0].y, 0.11);
    }

    #[test]
    fn z_and_visibility_and_the_timestamps_pass_through() {
        let frames = vec![frame(7, 0.1, 0.2, 0.75), frame(8, 0.3, 0.4, 0.75)];
        let out = smooth_keypoints(&frames, DEFAULT_WINDOW);
        assert_eq!(out[0].frame_index, 7);
        assert_eq!(out[1].timestamp_ms, 80.0);
        assert_eq!(out[0].landmarks[0].z, 0.25);
        assert_eq!(out[0].landmarks[0].visibility, 0.75);
    }

    /// The faithfully-reproduced wart. See the module doc.
    #[test]
    fn smoothing_drops_the_camera_id_and_the_short_path_does_not() {
        let frames = vec![frame(0, 0.1, 0.2, 1.0), frame(1, 0.3, 0.4, 1.0)];
        assert_eq!(smooth_keypoints(&frames, DEFAULT_WINDOW)[0].camera_id, None);
        assert_eq!(
            smooth_keypoints(&frames, 1)[0].camera_id,
            Some("cam".to_string())
        );
    }
}
