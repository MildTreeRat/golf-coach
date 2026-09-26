//! The live trigger: hearing the ball strike while it is still happening. [M20 P2]
//!
//! The first code in this crate with no Python counterpart. [`crate::offline`] is handed a clip
//! that already exists and may look at all of it; this is handed a microphone and has to decide,
//! with bounded latency and no view of the future, that a shot just happened — so that a session
//! can cut ten seconds of video around something it has only just heard.
//!
//! # These two detectors are not doing the same job
//!
//! It is tempting to read this as "`detect_strikes`, streamed", and that reading produces the
//! wrong thresholds. The offline detector's job is to *list candidates for an anchor*: it is
//! deliberately permissive ([`crate::offline::MIN_PROMINENCE_Z`] is 8.0, roughly twice the
//! loudest thing an empty bay produces) and it leans on whole-clip statistics to throw away the
//! quiet precursors that would otherwise win the earliest-wins rule in
//! `analysis/alignment.py::with_measured_impact`.
//!
//! This one's job is **segmentation**: did a shot happen, and where do I cut. A false positive
//! here costs a clip, a pose run and a row in the golfer's history; a false negative costs the
//! swing entirely. So it fires on a much higher bar and it does not try to resolve which of a
//! shot's four transients was the ball — that question is answered *afterwards*, when the cut
//! clip goes through [`crate::offline::detect_strikes`] with whole-clip statistics available.
//!
//! **That split is what makes the missing whole-clip floor a non-problem.**
//! [`crate::offline::MIN_RELATIVE_PROMINENCE`] is a fraction of the loudest transient in the
//! clip, which cannot be known until the clip is over — so no streaming detector can apply it.
//! It does not need to: the precursor problem it exists to solve is an *alignment* problem, and
//! alignment happens offline on a clip this detector has already cut.
//!
//! # What is clip-relative when there is no clip
//!
//! ADR-013's principle — thresholds live in the signal's own units, never in absolute ones,
//! because a phone two metres from the mat and one six metres away record the same swing at very
//! different levels — still holds and now has to be applied to a *moving window*. The floor is a
//! rolling median and median-absolute-deviation over the last [`FLOOR_WINDOW_S`] seconds of flux,
//! excluding the hop being judged. Robust rather than a mean and standard deviation, for the
//! reason the offline detector gives: a handful of enormous transients is exactly what would
//! inflate an sd and then hide itself behind it.

use std::collections::VecDeque;

use crate::flux::{Geometry, Stft};
use crate::offline::CONFIDENCE_HALF_Z;

/// How much recent flux the rolling floor reads.
///
/// Long enough that a shot occupies a small fraction of it — the ball, the mat, the screen and
/// the simulator's own audio together span under half a second, which is under 10% of this, well
/// inside a median's 50% breakdown point — and short enough to follow a bay that gets louder when
/// the group next door arrives. A floor computed over the whole session would be the thing
/// ADR-013 forbids in slow motion: one number meaning different things at different times.
pub const FLOOR_WINDOW_S: f64 = 5.0;

/// How much history must exist before anything is allowed to fire.
///
/// A median over three hops is not a floor, it is an accident. Without this the detector fires on
/// the first loud thing after startup — including the click of the capture device opening, which
/// is the single most reliable false positive available.
pub const WARMUP_S: f64 = 1.0;

/// How far above the rolling floor a hop must stand to trigger, in robust z.
///
/// **Measured by M20 P3** over all 30 stored clips — 11.9 minutes of bay audio — with
/// `scripts/trigger_replay.py`. Against the impact the pipeline actually anchored on, the two
/// populations are:
///
/// | | z |
/// |---|---|
/// | real strikes (30) | **137.5** to 647, median 300 |
/// | false positives (148) | up to **111.3** |
///
/// They separate by 1.24x with nothing in between, and 70 sits **2.0x below the quietest real
/// strike** rather than in the middle of that gap. The asymmetry is deliberate and it is the
/// whole argument for this number: a false negative is a swing the golfer made and the app never
/// saw, and nothing downstream can recover it; a false positive costs a clip and a pose run and
/// is then *filtered* — the cut clip goes through [`crate::offline::detect_strikes`], which has
/// the whole-clip statistics this detector cannot have. So the margin is spent on recall. Thirty
/// clips from one bay, one golfer and one phone is not a sample to fit a threshold tightly to.
///
/// At this bar, recall is 30/30 and two genuine false positives survive in 11.9 minutes.
///
/// **One apparent false positive is not one**, and finding that out is what fixed the measurement:
/// the loudest survivor at any bar is `2026-08-23/8 down_the_line` at 66.66 s, z = 189. That clip
/// is 80 seconds long and holds a **second shot** the corpus never labelled — `detect_strikes`
/// gives it 16.82M prominence against the anchored shot's 19.35M, with a second transient 120 ms
/// behind it, which is the ball-to-screen gap `docs/M11_ACOUSTIC_SYNC.md` §E5 measures at
/// 85-145 ms. Counted as a false positive it made the two populations overlap; counted correctly
/// it is the detector finding a swing the ground truth did not know about.
///
/// **What this cannot say** is the false-positive rate in an empty bay, because no such recording
/// exists — every clip on disk was recorded around a swing. That is M20 P4's continuous recording,
/// and until it runs this number is measured against the wrong background.
pub const MIN_TRIGGER_Z: f64 = 70.0;

/// How long the detector keeps watching after the bar is crossed, before naming the instant.
///
/// **This exists because the first draft did not have it, and M20 P3 measured what that cost.**
/// Without it the detector fires on the first hop over the bar and starts its refractory period
/// there — so on `2026-08-23/3 face_on` at a bar of z = 8 it fired at 10.820 s on the rising edge
/// at z = 8.2, and the refractory then swallowed the actual strike 35 ms later at **z = 245**.
/// The consequence is worse than it sounds: lowering the threshold made the *timing* worse, not
/// just the precision, and the reported `z` described the edge of a transient rather than the
/// transient. Recall hid it, because a trigger 35 ms early still lands inside any sane match
/// window.
///
/// Holding for 50 ms and reporting the loudest hop in that span decouples the instant from the
/// threshold: the same strike now lands on the same sample whether the bar is 8 or 140. 50 ms is
/// the offline detector's [`crate::offline::MIN_SEPARATION_S`] and for the same reason — it is
/// long enough to absorb the club hitting the mat 15-20 ms after the ball, and short enough to
/// stay clear of the impact screen at 85-145 ms. It is paid as latency, once, per shot.
pub const PEAK_HOLD_S: f64 = 0.050;

/// How long after a trigger the detector stays silent.
///
/// **Longer than the offline separation, and in the opposite direction.**
/// [`crate::offline::MIN_SEPARATION_S`] is 50 ms precisely so that the ball and the screen stay
/// *apart* — the offline detector wants both, because the caller has to be able to see that there
/// were two. This detector wants one trigger per shot, so it has to swallow the whole burst: the
/// club hitting the mat 15-20 ms after the ball, the impact screen 85-145 ms after that, and the
/// simulator's own ball-flight audio a moment later. 250 ms covers all four with room to spare
/// and is still far shorter than the ten seconds between two real swings.
pub const REFRACTORY_S: f64 = 0.250;

/// A live trigger: a shot, heard.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Trigger {
    /// Absolute sample index of the analysis window whose new energy fired this.
    ///
    /// The same convention [`crate::strike::Strike::sample`] uses — the first sample of that
    /// window — so a trigger and a strike found in the clip it cut are measured against the same
    /// origin and can be compared without a fudge factor.
    pub sample: usize,
    /// How far above the rolling floor the flux stood, in robust z.
    pub z: f64,
    /// `z / (z + CONFIDENCE_HALF_Z)`, the offline detector's mapping.
    ///
    /// Shared deliberately: these numbers are read side by side during replay, and two
    /// confidences on different scales would make that comparison quietly meaningless.
    pub confidence: f64,
}

/// Flux in, triggers out, with bounded latency and no view of the future.
///
/// Feed it whatever the capture device hands over — the block size is the caller's business and
/// changes nothing about the result, which is what [`tests::the_block_size_does_not_change_the_answer`]
/// asserts. Every trigger is reported on the `push` that contains its onset, so the latency is one
/// analysis window plus one hop (26 ms at 48 kHz) plus whatever the caller's own block size adds.
pub struct OnlineDetector {
    geometry: Geometry,
    stft: Stft,
    /// Samples not yet consumed into an analysis window, and the absolute index of `buffer[0]`.
    buffer: Vec<i16>,
    base: usize,
    cursor: usize,
    previous: Option<Vec<f64>>,
    /// How many analysis frames have been transformed. Frame `k`'s flux is reported at absolute
    /// sample `k * hop`, which is the offline `(peak + 1) * hop` written from the other side.
    frames: usize,
    history: VecDeque<f64>,
    floor_capacity: usize,
    warmup_hops: usize,
    peak_hold_hops: usize,
    refractory_hops: usize,
    silent_until: usize,
    /// Nothing may fire before this hop: set at startup and again whenever the floor goes
    /// degenerate, which are the same situation seen twice.
    deaf_until: usize,
    holding: Option<Peak>,
    min_z: f64,
}

/// The loudest hop seen since the bar was crossed, and where it was.
#[derive(Debug, Clone, Copy)]
struct Peak {
    hop: usize,
    z: f64,
}

/// What the rolling floor makes of one hop.
///
/// Three outcomes rather than an `Option`, because "quieter than the bar" and "there is no bar"
/// are different events and only one of them restarts the warm-up.
#[derive(Debug, Clone, Copy)]
enum Verdict {
    Over(f64),
    Under,
    NoFloor,
}

impl OnlineDetector {
    pub fn new(rate: u32) -> Self {
        Self::with_threshold(rate, MIN_TRIGGER_Z)
    }

    /// The same detector with a different bar, for the sweep M20 P3 runs.
    ///
    /// Public because the threshold is the one constant here that has not been measured on real
    /// continuous audio yet, and a sweep that had to edit a `const` to move it would be a sweep
    /// nobody re-runs.
    pub fn with_threshold(rate: u32, min_z: f64) -> Self {
        let geometry = Geometry::for_rate(rate);
        let hops_per_second = f64::from(rate) / geometry.hop as f64;
        Self {
            geometry,
            stft: Stft::new(geometry),
            buffer: Vec::new(),
            base: 0,
            cursor: 0,
            previous: None,
            frames: 0,
            history: VecDeque::new(),
            floor_capacity: (FLOOR_WINDOW_S * hops_per_second) as usize,
            warmup_hops: (WARMUP_S * hops_per_second) as usize,
            peak_hold_hops: ((PEAK_HOLD_S * hops_per_second) as usize).max(1),
            refractory_hops: (REFRACTORY_S * hops_per_second) as usize,
            silent_until: 0,
            deaf_until: (WARMUP_S * hops_per_second) as usize,
            holding: None,
            min_z: rate_check(min_z),
        }
    }

    /// Consume a block of samples and return whatever fired inside it.
    pub fn push(&mut self, samples: &[i16]) -> Vec<Trigger> {
        self.buffer.extend_from_slice(samples);
        let Geometry { hop, window } = self.geometry;

        let mut fired = Vec::new();
        while self.cursor + window <= self.buffer.len() {
            let current = self
                .stft
                .magnitude(&self.buffer[self.cursor..self.cursor + window]);
            if let Some(previous) = &self.previous {
                let flux = Stft::flux(&current, previous);
                if let Some(trigger) = self.observe(flux) {
                    fired.push(trigger);
                }
            }
            self.previous = Some(current);
            self.cursor += hop;
            self.frames += 1;
        }

        // Drop what can never be read again. Only whole hops, so `base` stays a multiple of the
        // hop and absolute sample indices keep meaning what they mean.
        if self.cursor > window {
            let drop_to = self.cursor - window;
            self.buffer.drain(..drop_to);
            self.base += drop_to;
            self.cursor = window;
        }
        fired
    }

    /// One flux value, judged against the floor the ones before it imply.
    fn observe(&mut self, flux: f64) -> Option<Trigger> {
        // The floor is read **before** this value joins the history, so a strike cannot raise its
        // own bar. Over a five-second window one hop moves a median almost not at all, but
        // "almost not at all" is not a rule anyone can reason about and this is.
        let verdict = self.judge(flux);

        self.history.push_back(flux);
        if self.history.len() > self.floor_capacity {
            self.history.pop_front();
        }

        // Frame `self.frames` produced this flux against frame `self.frames - 1`.
        let hop_index = self.frames;
        match verdict {
            // No floor worth the name. Start the warm-up clock again rather than merely
            // declining this hop: a window that has just been full of digital silence has a
            // near-zero scale, so the *first* sound after it divides by almost nothing and
            // reports an enormous z whatever it was. Measured by M20 P3 — replaying the corpus as
            // one stream with silence between the clips fired on 20 of the 29 seams, and this is
            // the whole of that number. Recovering from silence and starting up are the same
            // situation, so they get the same answer.
            Verdict::NoFloor => {
                self.deaf_until = hop_index + self.warmup_hops;
                self.holding = None;
            }
            Verdict::Over(z) if hop_index >= self.deaf_until && hop_index >= self.silent_until => {
                self.hold(hop_index, z);
            }
            _ => {}
        }
        // Checked on every hop, including ones that did not clear the bar: the burst's window
        // closes on the clock, not on the next loud thing to arrive.
        self.release(hop_index)
    }

    /// Accept a hop that cleared the bar, or fold it into the burst already being held.
    fn hold(&mut self, hop_index: usize, z: f64) -> Option<Trigger> {
        match &mut self.holding {
            Some(peak) if z > peak.z => {
                peak.z = z;
                peak.hop = hop_index;
            }
            Some(_) => {}
            None => self.holding = Some(Peak { hop: hop_index, z }),
        }
        None
    }

    /// Emit the held burst once its window has closed.
    fn release(&mut self, hop_index: usize) -> Option<Trigger> {
        let peak = self
            .holding
            .take_if(|p| hop_index >= p.hop + self.peak_hold_hops)?;
        // The refractory period runs from the *peak*, not from the crossing that opened the hold,
        // so a burst with a slow attack does not get a shorter silence than a sharp one.
        self.silent_until = peak.hop + self.refractory_hops;
        Some(Trigger {
            sample: peak.hop * self.geometry.hop,
            z: peak.z,
            confidence: peak.z / (peak.z + CONFIDENCE_HALF_Z),
        })
    }

    /// What the rolling floor makes of one flux value.
    fn judge(&self, flux: f64) -> Verdict {
        if self.history.len() < self.warmup_hops.max(2) {
            return Verdict::NoFloor;
        }
        let centre = median(self.history.iter().copied());
        let scale = median(self.history.iter().map(|v| (v - centre).abs())) * MAD_TO_SIGMA;
        if scale <= 0.0 {
            // Digital silence. There is no clip-relative threshold to apply and an absolute one
            // would mean something different on every device (ADR-013). A real microphone never
            // produces this; a muted or disconnected one always does, which is the case worth
            // declining rather than firing on.
            return Verdict::NoFloor;
        }
        let z = (flux - centre) / scale;
        if z >= self.min_z {
            Verdict::Over(z)
        } else {
            Verdict::Under
        }
    }

    /// How many samples have been consumed into analysis windows so far.
    pub fn position(&self) -> usize {
        self.base + self.cursor
    }
}

/// Same constant as [`crate::offline`], and it has to stay the same one.
const MAD_TO_SIGMA: f64 = 1.4826;

fn rate_check(min_z: f64) -> f64 {
    assert!(
        min_z > 0.0,
        "a trigger threshold must be positive, got {min_z}"
    );
    min_z
}

/// The median of an iterator of finite floats.
///
/// Collected and sorted rather than selected, for the reason [`crate::offline`] gives: the window
/// is a thousand values two hundred times a second, which is nothing, and clarity is worth more
/// than the asymptotics here.
fn median(values: impl Iterator<Item = f64>) -> f64 {
    let mut sorted: Vec<f64> = values.collect();
    sorted.sort_by(|a, b| a.partial_cmp(b).expect("flux is NaN"));
    let n = sorted.len();
    if n % 2 == 1 {
        sorted[n / 2]
    } else {
        (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::testing::{bay, faded, RATE};

    #[test]
    fn a_strike_in_room_tone_fires_once() {
        let audio = bay(6.0, &[4.0]);
        let mut detector = OnlineDetector::new(RATE);
        let fired = detector.push(&audio);
        assert_eq!(fired.len(), 1, "expected one trigger, got {fired:?}");
        let at = fired[0].sample as f64 / f64::from(RATE);
        assert!(
            (at - 4.0).abs() < 0.05,
            "fired at {at:.3}s, expected within 50 ms of 4.000s"
        );
    }

    /// Room tone alone must produce nothing. This is the false-positive floor.
    #[test]
    fn an_empty_bay_is_silent() {
        let mut detector = OnlineDetector::new(RATE);
        assert!(detector.push(&bay(20.0, &[])).is_empty());
    }

    /// The whole burst a single shot makes is one trigger, not four.
    #[test]
    fn the_refractory_period_swallows_the_mat_and_the_screen() {
        // Ball, then the mat 18 ms later, then the impact screen 120 ms after the ball — the
        // spacing docs/M11_ACOUSTIC_SYNC.md §E5 measured.
        let audio = bay(8.0, &[4.0, 4.018, 4.120]);
        let mut detector = OnlineDetector::new(RATE);
        let fired = detector.push(&audio);
        assert_eq!(fired.len(), 1, "one shot is one trigger, got {fired:?}");
    }

    /// Two shots ten seconds apart are two triggers.
    #[test]
    fn two_swings_are_two_triggers() {
        let audio = bay(20.0, &[4.0, 14.0]);
        let mut detector = OnlineDetector::new(RATE);
        let fired = detector.push(&audio);
        assert_eq!(fired.len(), 2, "got {fired:?}");
        let gap = (fired[1].sample - fired[0].sample) as f64 / f64::from(RATE);
        assert!((gap - 10.0).abs() < 0.05, "gap was {gap:.3}s");
    }

    /// How the caller chops up the stream is the caller's business.
    ///
    /// The property that makes a replay harness meaningful: if the answer moved with the block
    /// size, replaying a file in 4096-sample chunks would measure the chunking rather than the
    /// detector, and no number it produced would transfer to a live microphone.
    #[test]
    fn the_block_size_does_not_change_the_answer() {
        let audio = bay(12.0, &[3.0, 8.5]);
        let whole = OnlineDetector::new(RATE).push(&audio);
        for block in [1, 97, 1024, 4096, 48_000] {
            let mut detector = OnlineDetector::new(RATE);
            let mut fired = Vec::new();
            for chunk in audio.chunks(block) {
                fired.extend(detector.push(chunk));
            }
            assert_eq!(fired, whole, "block size {block} changed the answer");
        }
    }

    /// Nothing fires before there is enough history to have a floor.
    #[test]
    fn a_strike_inside_the_warmup_is_not_reported() {
        let audio = bay(6.0, &[0.3]);
        let mut detector = OnlineDetector::new(RATE);
        assert!(detector.push(&audio).is_empty());
    }

    /// A muted or disconnected device declines rather than firing.
    #[test]
    fn digital_silence_declines() {
        let mut detector = OnlineDetector::new(RATE);
        assert!(detector.push(&vec![0i16; RATE as usize * 10]).is_empty());
    }

    /// Coming back from silence does not fire on the first thing it hears.
    ///
    /// The regression M20 P3 found by replaying the corpus as one stream with eight seconds of
    /// digital silence between the clips: the floor window fills with zeros, its scale collapses,
    /// and the next sound divides by almost nothing and reports an enormous z whatever it was.
    /// It fired on 20 of the 29 seams. Restarting the warm-up whenever the floor goes degenerate
    /// took that to zero and left the real strikes untouched — 30/30 either way.
    #[test]
    fn the_floor_rebuilds_after_silence_before_anything_fires() {
        // Faded into and out of the silence. A hard cut from tone to digital zero is a step
        // discontinuity, and truncating a tone splatters broadband energy across the spectrum —
        // so the detector fires on it, correctly, and the first version of this test was
        // measuring its own fixture. Real audio does not stop instantaneously.
        let mut stream = faded(bay(6.0, &[4.0]));
        stream.extend(std::iter::repeat_n(0i16, RATE as usize * 8));
        stream.extend(faded(bay(6.0, &[4.0])));

        let mut detector = OnlineDetector::new(RATE);
        let fired = detector.push(&stream);
        assert_eq!(fired.len(), 2, "one per real strike, got {fired:?}");
        for (trigger, expected) in fired.iter().zip([4.0, 18.0]) {
            let at = trigger.sample as f64 / f64::from(RATE);
            assert!(
                (at - expected).abs() < 0.05,
                "fired at {at:.3}s, expected {expected:.3}s"
            );
        }
    }
}
