//! Whole-clip strike detection: the question `detect_strikes` answers, and the oracle the online
//! detector in `stream` is measured against.
//!
//! Ported from `src/golf_coach/audio/impact.py::detect_strikes`. Every constant below carries the
//! measurement it came from, because that measurement is the only reason any of them is the
//! number it is — a threshold whose evidence has been dropped is a threshold nobody can revise.
//!
//! **Thresholds are clip-relative, per ADR-013.** Every decision here is expressed against the
//! clip's own flux floor (a robust z-score), never against an absolute amplitude: a phone two
//! metres from the mat and one six metres away record the same swing at very different levels,
//! and an absolute threshold would silently mean something different on each.

use crate::flux::{envelope, Geometry};
use crate::strike::Strike;

/// How far above the clip's flux floor a peak must stand to be listed at all, in robust z
/// (median absolute deviations from the median).
///
/// Measured over the 22 clips of the 2026-08-23 session: the ball and screen strikes land at
/// z = 113-599, while 20 s of pure room tone — and of amplitude-modulated noise standing in for a
/// voice — never exceeds z = 4.4. So this floor is an order of magnitude below every real strike
/// and roughly twice the loudest thing an empty bay produces. It is set low on purpose: this
/// function's job is to hand over candidates, not to pick one.
pub const MIN_PROMINENCE_Z: f64 = 8.0;

/// Peaks closer together than this are one transient seen twice.
///
/// A real strike's flux does not arrive as a spike but as a burst that stays elevated for tens of
/// milliseconds, and 50 ms is the largest suppression that still keeps the ball and the screen
/// apart: `docs/M11_ACOUSTIC_SYNC.md` §E5 measures that gap at 85-145 ms. The deliberate cost is
/// that the club-mat transient, 15-20 ms after the ball, is absorbed into the ball's candidate
/// rather than listed separately — the right trade, because no consumer has a question that needs
/// the mat.
pub const MIN_SEPARATION_S: f64 = 0.050;

/// The z at which `confidence` reads 0.5.
///
/// Chosen from the same 22-clip measurement so the numbers it produces are readable rather than
/// arbitrary: the real strikes (z = 113-599) land at 0.85-0.97, the listing floor above lands at
/// 0.29, and the loudest peak in an empty bay near 0.18.
pub const CONFIDENCE_HALF_Z: f64 = 20.0;

/// A second floor, and the one that decides which transient a caller anchors on.
///
/// [`MIN_PROMINENCE_Z`] separates a real onset from room tone; this separates the shot's own
/// transients from the quiet ones around them, as a fraction of the loudest transient in the same
/// clip. It exists because the rule downstream is *earliest wins*
/// (`analysis/alignment.py::with_measured_impact`): the ball is the first sound a shot makes, so
/// anything audible ahead of it is taken instead of it, and a candidate 2-3 frames early moves an
/// anchor a whole milestone is built on.
///
/// Measured over all 30 cached clips on 2026-08-30. Relative prominence of the precursors runs
/// 0.02-0.10 and the ball never falls below 0.61 — so this sits 2.5x above the loudest thing it
/// drops and 2.4x below the quietest thing it keeps, with a clear order of magnitude between
/// those two populations and nothing at all in between.
pub const MIN_RELATIVE_PROMINENCE: f64 = 0.25;

/// Scale factor turning a median absolute deviation into a standard deviation on normal data.
const MAD_TO_SIGMA: f64 = 1.4826;

/// Every transient in one decoded clip, loudest first.
///
/// Ordering is by `prominence`, descending — the ranking, not the timeline. Sort by `sample` for
/// time order.
///
/// `frame` is left `None` on every strike: this function is handed a waveform and a sample rate
/// and has never seen the video, so it cannot know the fps. The caller holding the manifest fills
/// it in.
///
/// Returns an empty list for a clip with no transient above the floor — which is a *result*, and
/// the one M11 P5 reads: a rehearsal swing makes no crack. It is also what a clip too short to
/// hold two analysis windows returns, and a clip of digital silence, where there is no floor to
/// measure anything against.
pub fn detect_strikes(samples: &[i16], rate: u32) -> Vec<Strike> {
    let (env, geometry) = envelope(samples, rate);
    detect_in_envelope(&env, rate, geometry)
}

/// The half of detection that reads statistics rather than audio.
///
/// Split out because it is separately checkable: `spec/vectors/audio/` ships each clip's whole
/// flux envelope, which is a thousandth of the bytes its waveform would be, and this is the
/// function those vectors call. Windowing the *waveform* instead would not work — the median, the
/// MAD and `MIN_RELATIVE_PROMINENCE` are all taken over the whole clip, so an excerpt has
/// different statistics and a different answer.
pub fn detect_in_envelope(env: &[f64], rate: u32, geometry: Geometry) -> Vec<Strike> {
    if env.is_empty() {
        return Vec::new();
    }
    let centre = median(env);
    // Median absolute deviation, robust rather than a plain standard deviation because the thing
    // being measured — a handful of enormous transients — is precisely what would inflate an sd
    // and then hide itself behind it.
    let deviations: Vec<f64> = env.iter().map(|v| (v - centre).abs()).collect();
    let scale = median(&deviations) * MAD_TO_SIGMA;
    if scale <= 0.0 {
        // Digital silence, or a flux envelope flat enough to have no floor. There is no
        // clip-relative threshold to apply and an absolute one would mean something different on
        // every clip (ADR-013), so decline rather than invent one.
        return Vec::new();
    }

    let z: Vec<f64> = env.iter().map(|v| (v - centre) / scale).collect();
    let separation = ((MIN_SEPARATION_S * f64::from(rate) / geometry.hop as f64)
        .round_ties_even()
        .max(1.0)) as usize;
    // Both floors are read off the same envelope and they are floors on different things: `z` is
    // "louder than this clip's own noise", prominence is "loud against this clip's own loudest".
    // A clip whose peak barely clears the noise has no meaningful ratio to take, so the relative
    // floor is only meaningful once the absolute one has been passed — which is the order below.
    let loudest = env.iter().copied().fold(f64::NEG_INFINITY, f64::max) - centre;
    let floor = loudest * MIN_RELATIVE_PROMINENCE;

    let mut order: Vec<usize> = (0..env.len()).collect();
    // Descending by z. The reference is `np.argsort(z)[::-1]`, whose default sort is *not* stable,
    // so numpy's own tie order is unspecified and there is nothing to match on a tie; ties are
    // broken here by ascending index so that this implementation is at least deterministic. Exact
    // equality between two envelope values needs identical float sums over 513 bins and does not
    // occur anywhere in the committed corpus — if it ever does, it is a finding, not a rounding
    // difference, and it belongs in `docs/CONFORMANCE.md` §3.
    order.sort_by(|&a, &b| z[b].partial_cmp(&z[a]).unwrap().then(a.cmp(&b)));

    let mut strikes = Vec::new();
    let mut chosen: Vec<usize> = Vec::new();
    for peak in order {
        if z[peak] < MIN_PROMINENCE_Z {
            break;
        }
        if env[peak] - centre < floor {
            // Ordered loudest-first, so everything after this is quieter still. `break` rather
            // than `continue` rests on that ordering exactly as the line above does; it is not a
            // different rule.
            break;
        }
        if chosen
            .iter()
            .any(|&other| peak.abs_diff(other) < separation)
        {
            continue;
        }
        chosen.push(peak);
        strikes.push(Strike {
            // The first sample of the analysis window whose new energy produced this rise.
            // Calibrated against the raw waveform on all 22 clips of the 2026-08-23 session: this
            // convention runs +18 ms late on average (sd 25 ms) against where the amplitude
            // actually starts climbing, most of that spread being the crudeness of the reference
            // rather than of the estimate. The bias is a property of the window, so it is the
            // same in both views of a bundle and cancels in any comparison between them.
            sample: (peak + 1) * geometry.hop,
            confidence: z[peak] / (z[peak] + CONFIDENCE_HALF_Z),
            prominence: env[peak] - centre,
            frame: None,
        });
    }
    strikes
}

/// `np.median`: the middle value, or the mean of the two middle ones.
///
/// Sorting a copy rather than selecting, because the envelopes here are tens of thousands of
/// values and the clarity is worth more than the asymptotics. NaN would panic on the comparison,
/// which is the honest outcome — a NaN in a flux envelope means the decode produced something
/// that is not audio, and a median that silently ordered around it would hide that.
fn median(values: &[f64]) -> f64 {
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).expect("flux envelope contains NaN"));
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
    use crate::flux::Geometry;

    fn flat(value: f64, n: usize) -> Vec<f64> {
        vec![value; n]
    }

    /// Room tone with a floor that actually varies.
    ///
    /// A *flat* background has a median absolute deviation of zero, and the detector declines on
    /// that by design — there is no clip-relative threshold to take, and ADR-013 forbids
    /// inventing an absolute one. Which means a perfectly flat fixture tests the refusal path and
    /// never reaches the peak-picking below it, and the first draft of these tests asserted
    /// against an empty list without noticing.
    fn background(n: usize) -> Vec<f64> {
        (0..n).map(|i| 1.0 + (i % 7) as f64 * 0.1).collect()
    }

    #[test]
    fn median_of_an_even_count_is_the_mean_of_the_middle_pair() {
        assert_eq!(median(&[1.0, 2.0, 3.0, 4.0]), 2.5);
        assert_eq!(median(&[3.0, 1.0, 4.0, 2.0]), 2.5);
        assert_eq!(median(&[1.0, 2.0, 3.0]), 2.0);
    }

    /// A flat envelope has no floor to measure against, so nothing is reported.
    #[test]
    fn digital_silence_declines_rather_than_guessing() {
        let geometry = Geometry::for_rate(48_000);
        assert!(detect_in_envelope(&flat(0.0, 500), 48_000, geometry).is_empty());
        assert!(detect_in_envelope(&flat(7.5, 500), 48_000, geometry).is_empty());
        assert!(detect_in_envelope(&[], 48_000, geometry).is_empty());
    }

    /// Two peaks inside the suppression window are one transient, and the quieter one goes.
    #[test]
    fn peaks_closer_than_the_separation_collapse_to_one() {
        let geometry = Geometry::for_rate(48_000);
        let mut env = background(500);
        env[200] = 1000.0;
        env[204] = 900.0; // 4 hops = 20 ms, inside the 50 ms suppression
        env[300] = 950.0; // 100 hops away, its own transient
        let found = detect_in_envelope(&env, 48_000, geometry);
        let samples: Vec<usize> = found.iter().map(|s| s.sample).collect();
        assert_eq!(samples, vec![201 * geometry.hop, 301 * geometry.hop]);
    }

    /// The relative floor drops a quiet precursor that the z floor alone would have listed.
    #[test]
    fn a_quiet_precursor_is_dropped_by_the_relative_floor() {
        let geometry = Geometry::for_rate(48_000);
        let mut env = background(500);
        env[100] = 50.0; // z is enormous against a flat floor, but 5% of the loudest
        env[300] = 1000.0;
        let found = detect_in_envelope(&env, 48_000, geometry);
        assert_eq!(found.len(), 1, "the precursor should not be listed");
        assert_eq!(found[0].sample, 301 * geometry.hop);
    }

    /// Detection never invents a frame number.
    #[test]
    fn frames_are_left_unknown() {
        let geometry = Geometry::for_rate(48_000);
        let mut env = background(500);
        env[250] = 1000.0;
        assert!(detect_in_envelope(&env, 48_000, geometry)
            .iter()
            .all(|s| s.frame.is_none()));
    }
}
