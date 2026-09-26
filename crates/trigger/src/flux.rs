//! Half-wave-rectified spectral flux: the envelope everything else in this crate reads.
//!
//! Ported from `src/golf_coach/audio/impact.py::_flux_envelope`, and the reasoning there is the
//! reasoning here. Flux — how much each frequency bin *grew* since the previous window — rather
//! than plain energy, because it answers the question actually being asked. A ball strike is a
//! sudden broadband arrival, and it is the arrival that marks the instant; loudness alone peaks
//! somewhere in the middle of the ringing that follows, and where in it depends on the room, the
//! microphone and how far away the phone was standing.
//!
//! **This implementation has no block structure and the Python one does.** `impact.py` computes
//! the STFT in blocks of 4096 frames to keep peak allocation flat — the whole-clip matrix would
//! be 134 MB on the corpus's longest clip — and then has to stitch the one difference that
//! straddles each block boundary back in by hand. Holding two magnitude spectra and walking
//! forward costs nothing here and removes the boundary along with the stitch, so the two agree by
//! construction rather than by a fixup. `envelope_is_independent_of_chunking` below is what says
//! so out loud.

use rustfft::num_complex::Complex;
use rustfft::{Fft, FftPlanner};
use std::sync::Arc;

/// Hop between analysis windows, in seconds.
///
/// 5 ms is the hop `docs/M11_ACOUSTIC_SYNC.md` §E4-E5 measured the corpus with, kept so the
/// numbers in that document and in this crate describe the same signal. It is a third of a frame
/// at 60 fps, which is finer than any consumer needs. A shorter one was tried and rejected: at a
/// 2.5 ms hop over a 256-sample window the envelope of a real strike breaks into a dozen
/// jittering sub-peaks (measured on `2026-08-23/2` face-on) and the ball-versus-screen structure
/// this module exists to expose stops being legible.
pub const HOP_S: f64 = 0.005;

/// Analysis window length, in seconds — 1024 samples at the corpus's 48 kHz.
///
/// Expressed in seconds rather than samples so a differently-sampled clip gets the same *time*
/// resolution rather than the same array shape.
pub const WINDOW_S: f64 = 1024.0 / 48_000.0;

/// The geometry a sample rate implies: how far apart windows sit, and how long each one is.
///
/// Derived rather than passed in, because getting this derivation right is part of what a port
/// has to prove. `round` here is Python's, which breaks a tie to the **even** neighbour — and the
/// tie is reachable: `44_100 * 0.005` is exactly 220.5, so Rust's `f64::round`
/// (half-away-from-zero) would give 221 where the reference gives 220, and every sample index in
/// the clip would then be wrong by a growing multiple of one hop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Geometry {
    pub hop: usize,
    pub window: usize,
}

impl Geometry {
    pub fn for_rate(rate: u32) -> Self {
        let rate = f64::from(rate);
        Self {
            hop: (rate * HOP_S).round_ties_even().max(1.0) as usize,
            window: (rate * WINDOW_S).round_ties_even().max(2.0) as usize,
        }
    }
}

/// `np.hanning(m)`, evaluated the way numpy evaluates it.
///
/// Not `0.5 - 0.5*cos(2*pi*i/(m-1))`, which is the same function and not the same arithmetic.
/// numpy computes `0.5 + 0.5*cos(pi*n/(m-1))` over `n = 1-m, 3-m, ..., m-1`, and the two differ
/// in the last bit of most coefficients. One bit of taper is far inside the tolerance the vectors
/// are compared at, so this is not load-bearing — it is written this way because "the same
/// formula rearranged" is exactly the kind of difference that is invisible until it is the only
/// thing left to explain a failure.
///
/// This is also the symmetric window, not the periodic one most DSP crates hand you by default.
fn hann(m: usize) -> Vec<f64> {
    let denominator = (m - 1) as f64;
    (0..m)
        .map(|i| {
            let n = 1.0 - m as f64 + 2.0 * i as f64;
            0.5 + 0.5 * (std::f64::consts::PI * n / denominator).cos()
        })
        .collect()
}

/// One value per hop: how much broadband energy arrived since the previous window.
///
/// Returns an empty envelope when the clip holds fewer than two analysis windows — there is no
/// "since the previous window" to measure — which is a result and not an error, and is one of the
/// two ways `detect_strikes` legitimately finds nothing.
pub fn envelope(samples: &[i16], rate: u32) -> (Vec<f64>, Geometry) {
    let geometry = Geometry::for_rate(rate);
    (envelope_with(samples, geometry), geometry)
}

fn envelope_with(samples: &[i16], geometry: Geometry) -> Vec<f64> {
    let Geometry { hop, window } = geometry;
    let frames = if samples.len() >= window {
        1 + (samples.len() - window) / hop
    } else {
        0
    };
    if frames < 2 {
        return Vec::new();
    }

    let stft = Stft::new(geometry);
    let mut previous = stft.magnitude(&samples[..window]);
    let mut out = Vec::with_capacity(frames - 1);
    for frame in 1..frames {
        let start = frame * hop;
        let current = stft.magnitude(&samples[start..start + window]);
        out.push(Stft::flux(&current, &previous));
        previous = current;
    }
    out
}

/// One tapered short-time transform, planned once and reused.
///
/// Shared by the whole-clip path above and the live one in [`crate::stream`], because they must
/// not be allowed to differ. The offline detector is the online detector's ground truth — its
/// precision and recall are measured against what the offline one found on the same audio — and
/// that comparison means nothing if the two are reading different signals. One copy of the
/// transform is what makes it a comparison rather than a coincidence.
pub(crate) struct Stft {
    taper: Vec<f64>,
    fft: Arc<dyn Fft<f64>>,
}

impl Stft {
    pub(crate) fn new(geometry: Geometry) -> Self {
        Self {
            taper: hann(geometry.window),
            fft: FftPlanner::<f64>::new().plan_fft_forward(geometry.window),
        }
    }

    /// The magnitude spectrum of one tapered window, positive frequencies only.
    ///
    /// `rfft`'s output, reproduced from a full complex transform: `rustfft` has no real-input
    /// plan, and a real signal's spectrum is conjugate-symmetric, so bins `0..=window/2` are the
    /// whole of it. The discarded half costs a little time and no accuracy.
    pub(crate) fn magnitude(&self, frame: &[i16]) -> Vec<f64> {
        let mut buffer: Vec<Complex<f64>> = self
            .taper
            .iter()
            .zip(frame)
            .map(|(t, s)| Complex::new(f64::from(*s) * t, 0.0))
            .collect();
        self.fft.process(&mut buffer);
        // `norm` is `hypot`, which is what numpy's `abs` on a complex128 array is too. The naive
        // `(re*re + im*im).sqrt()` is a different function near the extremes of the exponent
        // range and there is no reason to hand-roll it.
        buffer[..=self.taper.len() / 2]
            .iter()
            .map(|bin| bin.norm())
            .collect()
    }

    /// Half-wave-rectified flux between two consecutive magnitude spectra.
    ///
    /// Only *growth* counts. A bin that fell is a sound ending, and the instant being located is
    /// a sound starting.
    pub(crate) fn flux(current: &[f64], previous: &[f64]) -> f64 {
        current
            .iter()
            .zip(previous)
            .map(|(now, before)| (now - before).max(0.0))
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A click at a known offset lands on the hop that contains it.
    #[test]
    fn a_transient_raises_the_envelope_where_it_happens() {
        let rate = 48_000;
        let mut samples = vec![0i16; rate as usize];
        for (i, s) in samples.iter_mut().enumerate() {
            // Quiet, deterministic room tone, so the floor is not exactly zero.
            *s = ((i % 97) as i16 - 48) * 2;
        }
        samples[rate as usize / 2] = 20_000;
        let (env, geometry) = envelope(&samples, rate);
        let peak = env
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .unwrap()
            .0;
        let landed = (peak + 1) * geometry.hop;
        assert!(
            landed.abs_diff(rate as usize / 2) <= geometry.window,
            "click at {} reported at {landed}",
            rate / 2
        );
    }

    /// The envelope does not depend on how the work is divided up.
    ///
    /// The property the Python reference needs a hand-written stitch to hold: it computes the
    /// STFT in blocks of 4096 frames — 20.5 s at the hop above — and has to add back the one
    /// difference that straddles each boundary. This implementation walks forward holding two
    /// spectra, so there is no boundary to get wrong, and the assertion is that a signal longer
    /// than a Python block produces the same values as the concatenation of its parts implies.
    /// That is what makes the committed vectors' short excerpts sufficient evidence for an
    /// eighty-second clip.
    #[test]
    fn envelope_is_independent_of_chunking() {
        let rate = 48_000;
        let long: Vec<i16> = (0..rate as usize * 25)
            .map(|i| (((i * 7919) % 4001) as i16) - 2000)
            .collect();
        let (whole, geometry) = envelope(&long, rate);
        assert!(whole.len() > 4096, "signal must exceed one Python block");

        // Recompute a window of the middle independently; the overlapping hops must agree
        // exactly, not to a tolerance — same inputs, same order of operations.
        let offset_hops = 3000;
        let start = offset_hops * geometry.hop;
        let piece = &long[start..start + geometry.window + geometry.hop * 200];
        let (part, _) = envelope(piece, rate);
        for (i, value) in part.iter().enumerate() {
            assert_eq!(*value, whole[offset_hops + i], "hop {i} disagrees");
        }
    }

    /// Fewer than two analysis windows is an empty result, not an error.
    #[test]
    fn a_clip_too_short_to_hold_two_windows_has_no_envelope() {
        let rate = 48_000;
        let geometry = Geometry::for_rate(rate);
        assert!(envelope(&vec![0i16; geometry.window], rate).0.is_empty());
        assert!(envelope(&[], rate).0.is_empty());
    }

    /// The tie `f64::round` gets wrong, pinned.
    #[test]
    fn the_hop_at_44_1_khz_breaks_its_tie_to_even() {
        // 44_100 * 0.005 == 220.5 exactly. Python's round gives 220; half-away-from-zero gives
        // 221, and every sample index downstream would then drift by a multiple of one hop.
        assert_eq!(Geometry::for_rate(44_100).hop, 220);
        assert_eq!(Geometry::for_rate(48_000).hop, 240);
        assert_eq!(Geometry::for_rate(48_000).window, 1024);
    }
}
