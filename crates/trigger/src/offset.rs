//! How far apart two clips' clocks are, without naming a single transient. [M20 P6]
//!
//! Ported from `src/golf_coach/audio/impact.py::offset_between`, the other half of that file and
//! the *robust* half. `detect_strikes` says when the transients are and is the hard question — a
//! bay produces four per shot and the ball is not reliably the loudest. This asks nothing of the
//! kind: every sound in the bay reaches both phones, so cross-correlating the two flux envelopes
//! recovers the offset even when no single event in the pattern can be named. That is what let
//! M11 P6's synchronization be built without M11 P5 existing.
//!
//! Nothing in this repo calls it today — it was built as M11's diagnostic and used from the
//! corpus tools — and it is ported rather than retired because M21's two-camera work wants it and
//! because it is the evidence behind M11's "which half was lying" finding. It is covered by the
//! same vectors as the rest: `spec/vectors/audio/` records the offset for each of the fifteen
//! two-view swings on the face-on vector, against its named sibling.

use rustfft::num_complex::Complex;
use rustfft::FftPlanner;

use crate::flux::Geometry;

/// How well the two envelopes must line up before an offset is reported at all.
///
/// Measured whole-clip over the eleven 2026-08-23 bundles: the winning lag scores
/// r = 0.838-0.923 against a best rival of 0.125-0.672. An offset that cannot be trusted is not
/// reported (ADR-010 §2).
pub const MIN_CORRELATION: f64 = 0.45;

/// By how much the winning lag must beat the best rival elsewhere in the clip.
pub const MIN_MARGIN: f64 = 0.10;

/// How far from the winning lag a peak has to be before it counts as a *rival* rather than as the
/// shoulder of the same match.
///
/// Wider than §E5's 85-145 ms ball-to-screen gap on purpose: that gap puts a real secondary peak
/// either side of every correct answer — M11 P0 saw it at ±6-8 frames in the correlation function
/// — and a guard narrower than it would make every honest offset look ambiguous against its own
/// alias.
pub const RIVAL_GUARD_S: f64 = 0.25;

/// A lag whose overlap is a sliver of one clip can correlate on almost nothing.
///
/// Requiring half of the shorter envelope keeps the comparison a comparison; the one-second floor
/// is for the degenerate case where "half" is still too little to mean anything.
pub const MIN_OVERLAP_FRACTION: f64 = 0.5;
pub const MIN_OVERLAP_S: f64 = 1.0;

/// How far apart two clips' clocks are, with the evidence that says so.
///
/// Richer than the bare `f64` this was planned to return, and for a reason ADR-010 §2 makes
/// structural: the caller has to be able to *write down* why it trusted or declined an offset,
/// and a lone number can say neither how good the match was nor how close the runner-up came.
/// The two correlations are the whole audit trail.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClipOffset {
    /// `t_a - t_b` for any event visible in both clips: add it to a time in `b` to get the same
    /// instant on `a`'s clock. Positive means `b` was started earlier, so the event lands later
    /// in `a`'s own timeline.
    pub seconds: f64,
    /// Pearson correlation of the two flux envelopes at the winning lag, over their overlap.
    pub r: f64,
    /// The best correlation at any lag more than [`RIVAL_GUARD_S`] away — how alone the winner is.
    ///
    /// A high `r` with a `runner_up_r` right behind it is not a strong match; it is an ambiguous
    /// one.
    pub runner_up_r: f64,
}

/// How far apart two recordings of the same moment are, or `None` if it cannot be told.
///
/// **`max_lag_s` bounds the search, and a caller with any prior at all should pass one.** Measured
/// whole-clip over the eleven 2026-08-23 bundles, this recovers the offset to within 0-20 ms
/// (0-1.2 frames at 60 fps) on ten of them — and gets bundle 8 confidently, decisively *wrong*:
/// its 80-second down-the-line clip holds two shots, and the face-on clip's single shot matches
/// the second of them at r = 0.923 where the correct answer scores 0.672. The margin rule below
/// does not catch that and no margin rule can, because both answers are real matches; only a
/// bound on the search does.
///
/// Takes envelopes rather than waveforms, where the Python reference took waveforms and computed
/// them. Both halves of this crate work that way — see [`crate::offline::detect_in_envelope`] —
/// because the envelope is what the committed vectors can afford to carry, and a function that
/// computed its own would be unreachable from them.
pub fn offset_between(
    a: &[f64],
    b: &[f64],
    geometry: Geometry,
    rate: u32,
    max_lag_s: Option<f64>,
) -> Option<ClipOffset> {
    if a.is_empty() || b.is_empty() {
        return None;
    }
    let hop_s = geometry.hop as f64 / f64::from(rate);
    let (lags, mut r) = correlation_by_lag(a, b, min_overlap(a, b, hop_s));

    if let Some(limit) = max_lag_s {
        for (lag, value) in lags.iter().zip(r.iter_mut()) {
            if (lag.unsigned_abs() as f64) * hop_s > limit {
                *value = f64::NAN;
            }
        }
    }

    let winner = argmax(&r)?;
    let best = r[winner];

    let guard = ((RIVAL_GUARD_S / hop_s).round_ties_even().max(1.0)) as usize;
    let lo = winner.saturating_sub(guard);
    let hi = (winner + guard + 1).min(r.len());
    for value in &mut r[lo..hi] {
        *value = f64::NAN;
    }
    let runner_up = argmax(&r).map_or(0.0, |i| r[i]);

    if best < MIN_CORRELATION || best - runner_up < MIN_MARGIN {
        return None;
    }
    Some(ClipOffset {
        seconds: lags[winner] as f64 * hop_s,
        r: best,
        runner_up_r: runner_up,
    })
}

/// `np.nanargmax`: the index of the largest finite value, or `None` if there are none.
fn argmax(values: &[f64]) -> Option<usize> {
    values
        .iter()
        .enumerate()
        .filter(|(_, v)| v.is_finite())
        // `>` rather than `>=` so the *first* maximum wins, which is what numpy's argmax does.
        .fold(None, |best: Option<(usize, f64)>, (i, &v)| match best {
            Some((_, b)) if b >= v => best,
            _ => Some((i, v)),
        })
        .map(|(i, _)| i)
}

/// How many hops two envelopes must share before a lag between them is worth scoring.
fn min_overlap(a: &[f64], b: &[f64], hop_s: f64) -> usize {
    let by_time = (MIN_OVERLAP_S / hop_s).round_ties_even();
    let by_share = (MIN_OVERLAP_FRACTION * a.len().min(b.len()) as f64).round_ties_even();
    by_time.max(by_share) as usize
}

/// Pearson correlation of `a` against `b` at every lag, NaN where the overlap is too small.
///
/// Pearson over the *overlap at each lag*, not one global normalization, because the two clips
/// are different lengths and start at different moments: a lag that lines up ten seconds of
/// shared bay noise and one that lines up half a second of it would otherwise be scored on the
/// same scale. Prefix sums make the per-lag means and variances exact at no extra pass over the
/// data, so this stays one FFT and a handful of subtractions.
///
/// Lag `k` means an event at index `i` in `a` sits at index `i - k` in `b`.
fn correlation_by_lag(a: &[f64], b: &[f64], min_overlap: usize) -> (Vec<i64>, Vec<f64>) {
    let (n, m) = (a.len(), b.len());
    let size = (n + m).next_power_of_two();

    // numpy's `irfft` divides by the transform length and rustfft does not, so the products come
    // out scaled by `size` without this. It is the one normalization difference between the two
    // libraries and it is silent: every correlation would still be a correlation, just of a
    // covariance that no longer matches the variances it is divided by.
    let scale = 1.0 / size as f64;
    let mut planner = FftPlanner::<f64>::new();
    let forward = planner.plan_fft_forward(size);
    let inverse = planner.plan_fft_inverse(size);

    let mut spectrum_a = padded(a, size);
    let mut spectrum_b = padded(b, size);
    forward.process(&mut spectrum_a);
    forward.process(&mut spectrum_b);
    let mut circular: Vec<Complex<f64>> = spectrum_a
        .iter()
        .zip(&spectrum_b)
        .map(|(x, y)| x * y.conj())
        .collect();
    inverse.process(&mut circular);

    // The inverse transform wraps the negative lags around the end of the buffer; unwrap them
    // onto a plain axis, exactly as the reference's concatenation does.
    let lags: Vec<i64> = (-((m as i64) - 1)..(n as i64)).collect();
    let mut products: Vec<f64> = Vec::with_capacity(lags.len());
    products.extend(circular[size - (m - 1)..].iter().map(|c| c.re * scale));
    products.extend(circular[..n].iter().map(|c| c.re * scale));

    let (sum_a, sum_aa) = prefix(a);
    let (sum_b, sum_bb) = prefix(b);

    let r = lags
        .iter()
        .zip(&products)
        .map(|(&lag, &product)| {
            let lo_a = lag.max(0) as usize;
            let hi_a = ((m as i64 + lag).min(n as i64)).max(lag.max(0)) as usize;
            let count = hi_a - lo_a;
            if count < min_overlap {
                return f64::NAN;
            }
            let (lo_b, hi_b) = ((lo_a as i64 - lag) as usize, (hi_a as i64 - lag) as usize);
            let safe = count.max(1) as f64;

            let (total_a, total_aa) = (sum_a[hi_a] - sum_a[lo_a], sum_aa[hi_a] - sum_aa[lo_a]);
            let (total_b, total_bb) = (sum_b[hi_b] - sum_b[lo_b], sum_bb[hi_b] - sum_bb[lo_b]);
            let covariance = product - total_a * total_b / safe;
            let spread = ((total_aa - total_a * total_a / safe).max(0.0)
                * (total_bb - total_b * total_b / safe).max(0.0))
            .sqrt();
            if spread > 0.0 {
                covariance / spread
            } else {
                0.0
            }
        })
        .collect();
    (lags, r)
}

fn padded(values: &[f64], size: usize) -> Vec<Complex<f64>> {
    let mut out: Vec<Complex<f64>> = values.iter().map(|v| Complex::new(*v, 0.0)).collect();
    out.resize(size, Complex::new(0.0, 0.0));
    out
}

/// Running sums of the values and of their squares, both with a leading zero.
fn prefix(values: &[f64]) -> (Vec<f64>, Vec<f64>) {
    let mut sum = Vec::with_capacity(values.len() + 1);
    let mut sum_sq = Vec::with_capacity(values.len() + 1);
    sum.push(0.0);
    sum_sq.push(0.0);
    for v in values {
        sum.push(sum.last().unwrap() + v);
        sum_sq.push(sum_sq.last().unwrap() + v * v);
    }
    (sum, sum_sq)
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: u32 = 48_000;

    /// A pattern against a delayed copy of itself recovers the delay.
    #[test]
    fn a_shifted_copy_reports_its_shift() {
        let geometry = Geometry::for_rate(RATE);
        // Aperiodic on purpose. A pattern with a period correlates with itself at every
        // multiple of that period, so the runner-up sits right behind the winner and the margin
        // rule declines the pair — correctly. The first version of this test used a 700-hop
        // period and was asserting that a genuinely ambiguous signal produced an answer.
        let mut base = vec![0.0f64; 4000];
        for (i, v) in base.iter_mut().enumerate() {
            *v = (i % 13) as f64;
        }
        for at in [137usize, 401, 1129, 1723, 2591, 3067, 3449] {
            base[at] = 1000.0;
        }
        // `b` starts 200 hops before `a`, so an event in `a` sits 200 later in `b` — lag -200.
        let a = base[200..].to_vec();
        let b = base.clone();
        let offset = offset_between(&a, &b, geometry, RATE, None).expect("a clear match");
        let hop_s = geometry.hop as f64 / f64::from(RATE);
        assert!(
            (offset.seconds - (-200.0 * hop_s)).abs() < hop_s,
            "reported {:+.4}s, expected {:+.4}s",
            offset.seconds,
            -200.0 * hop_s
        );
        assert!(offset.r > 0.9, "r was {}", offset.r);
    }

    /// Two envelopes with nothing in common are declined, not guessed at.
    #[test]
    fn unrelated_envelopes_are_declined() {
        let geometry = Geometry::for_rate(RATE);
        let a: Vec<f64> = (0..3000).map(|i| ((i * 7919) % 101) as f64).collect();
        let b: Vec<f64> = (0..3000).map(|i| ((i * 104_729) % 97) as f64).collect();
        assert!(offset_between(&a, &b, geometry, RATE, None).is_none());
    }

    #[test]
    fn an_empty_envelope_has_no_offset() {
        let geometry = Geometry::for_rate(RATE);
        assert!(offset_between(&[], &[1.0, 2.0], geometry, RATE, None).is_none());
        assert!(offset_between(&[1.0, 2.0], &[], geometry, RATE, None).is_none());
    }
}
