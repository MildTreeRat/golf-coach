//! Deterministic bay audio, shared by the tests in [`crate::stream`] and [`crate::clip`]. [M20 P4]
//!
//! **No RNG anywhere.** `tests/audio/test_impact.py` synthesized its clips from a seeded numpy
//! generator, which is exactly why `docs/CONFORMANCE.md` §5 had to file detection as uncovered: a
//! seeded generator does not reproduce in another language, so there was no oracle a port could be
//! held to. Anything this crate generates has to be reproducible from the source alone.
//!
//! It lives in its own module rather than in whichever test module wrote it first because two now
//! need it, and a second copy of a fixture is a second thing that drifts — the room tone here has
//! to be the *same* room tone the thresholds in [`crate::stream`] were exercised against, or the
//! cutting tests are measuring a different bay.

pub const RATE: u32 = 48_000;

/// Room tone with broadband cracks dropped into it.
pub fn bay(seconds: f64, strikes_at: &[f64]) -> Vec<i16> {
    let n = (seconds * f64::from(RATE)) as usize;
    let mut out: Vec<i16> = (0..n)
        .map(|i| {
            // Two incommensurable tones plus a slow beat: broadband enough to have a floor,
            // quiet enough that a strike is obvious, and periodic so it is easy to reason
            // about.
            let t = i as f64 / f64::from(RATE);
            let hum = (t * 120.0 * std::f64::consts::TAU).sin() * 180.0;
            let hiss = ((i * 7919) % 211) as f64 - 105.0;
            (hum + hiss) as i16
        })
        .collect();
    for at in strikes_at {
        let start = (at * f64::from(RATE)) as usize;
        // A short broadband burst with a fast decay — a crack, not a tone.
        for k in 0..(RATE as usize / 100) {
            if start + k >= n {
                break;
            }
            let decay = (-(k as f64) / 300.0).exp();
            let click = (((start + k) * 31) % 2003) as f64 - 1001.0;
            out[start + k] = (out[start + k] as f64 + click * 12.0 * decay) as i16;
        }
    }
    out
}

/// The same room tone with a *whoosh* rather than a crack: a practice swing.
///
/// Broadband like a strike and far quieter, with its attack spread over 250 ms instead of
/// arriving in one window. Both halves of that matter — flux measures how much each bin *grew*
/// since the previous window, so a swell is small in the quantity the detector actually reads even
/// before its amplitude is taken into account.
///
/// **Measured against this room tone**, by sweeping the bar until it stops firing: this whoosh
/// peaks at **z = 32.5** and the crack [`bay`] drops in the same tone clears **3000**. So the
/// fixture is not balanced on the threshold — it sits at less than half of
/// [`crate::stream::MIN_TRIGGER_Z`], with the thing it must be told apart from two orders of
/// magnitude away. [`crate::clip::tests::a_practice_swing_cuts_nothing`] is what reads it.
pub fn whoosh(seconds: f64, at: f64) -> Vec<i16> {
    let mut out = bay(seconds, &[]);
    let start = (at * f64::from(RATE)) as usize;
    let span = RATE as usize / 4;
    for k in 0..span {
        if start + k >= out.len() {
            break;
        }
        let shape = (std::f64::consts::PI * k as f64 / span as f64).sin();
        let air = (((start + k) * 31) % 2003) as f64 - 1001.0;
        out[start + k] = (out[start + k] as f64 + air * 0.05 * shape) as i16;
    }
    out
}

/// A 200 ms ramp at each end, so joining two of these makes no click.
pub fn faded(mut samples: Vec<i16>) -> Vec<i16> {
    let ramp = RATE as usize / 5;
    let n = samples.len();
    for i in 0..ramp.min(n / 2) {
        let gain = i as f64 / ramp as f64;
        samples[i] = (samples[i] as f64 * gain) as i16;
        samples[n - 1 - i] = (samples[n - 1 - i] as f64 * gain) as i16;
    }
    samples
}
