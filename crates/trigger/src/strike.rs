//! What crosses out of this crate: an integer sample index, a confidence, and a prominence.
//!
//! The Rust side of `src/golf_coach/contracts/audio.py::AudioStrike`, and deliberately as narrow
//! as that one is — no waveform, no envelope, no spectrogram. What the rest of the system gets to
//! know about a ball strike is *when* it happened and *how sure* the detector is.
//!
//! The shape is pinned by `spec/schemas/audio_file.schema.json`, so this is not a free choice:
//! the JSON these serialize to is read by `storage.audio_io` and by anything else that opens a
//! `{role}.audio.json`.

use serde::{Deserialize, Serialize};

/// One transient found in a clip, ranked among its siblings by `prominence`.
///
/// **A strike is a candidate, not a verdict, and the first entry is not the ball.** A simulator
/// bay produces four transients per shot — club-ball contact, the club hitting the mat 15-20 ms
/// later, the ball hitting the impact screen 85-145 ms after that, and the simulator's own
/// ball-flight audio a moment later — and across the 22 clips of the 2026-08-23 session the
/// screen strike outranks the ball on roughly half of them. Any rule of the form "take the
/// loudest peak" is therefore wrong about half the time, which is why detection returns the list
/// and lets the caller decide.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Strike {
    /// Offset into the decoded mono waveform, in samples, after the container's edit list has
    /// been applied. This is the measurement; everything else here is derived from it.
    pub sample: usize,
    /// How sure the detector is this is a real onset, normalized to `[0, 1]`.
    pub confidence: f64,
    /// How far this onset stands above its local background, in the detector's own units —
    /// unbounded, and not comparable across detectors. Ranks candidates *within* one clip;
    /// `confidence` is the number to compare across clips.
    pub prominence: f64,
    /// The video frame this strike lands on, or `None` when it could not be worked out.
    ///
    /// Derived, never measured. Detection is handed a waveform and a sample rate and has never
    /// seen the video, so it cannot know the fps; the caller holding the clip's fps *and* its
    /// `video_start_s` fills this in. `None` means unknown — a strike whose frame could not be
    /// worked out is not frame 0 (ADR-010 §2).
    #[serde(default)]
    pub frame: Option<usize>,
}
