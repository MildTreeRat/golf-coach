//! This crate against `spec/vectors/audio/`.
//!
//! The Rust half of M19's suite. `scripts/conformance.py check` runs the swing-bundle vectors and
//! defers these ones here, because the implementation under test is this crate: ADR-030 §1 gives
//! strike detection to Rust and M20 retires the Python detector, so after the delete there is no
//! Python left to run them against.
//!
//! **What a vector contains, and why it is in two layers.** Windowing audio is lossy where
//! windowing keypoints was not. The corpus vectors ship the scored *slice* of a keypoint file
//! because `phases.select_swing` picks that window before the engine sees it. Detection has no
//! such property: the median, the MAD and `MIN_RELATIVE_PROMINENCE` are all taken over the
//! **whole** clip, so an excerpt has different statistics and a different answer — and shipping
//! every waveform would cost 68 MB against the 4.3 MB these do. So:
//!
//! - `input.envelope` is the whole clip's flux envelope and `expected.strikes` is what the
//!   reference found in it. The statistics, at real clip length.
//! - `input.excerpts` are short full-rate waveforms and `expected.excerpt_envelopes` is the flux
//!   envelope of each. The FFT, the Hann window, and the `(peak + 1) * hop` convention.
//!
//! Matching both is matching `detect_strikes` on the whole clip. The one thing neither layer
//! reaches is a signal long enough to cross the Python reference's 4096-frame block boundary
//! (20.5 s), and that is covered by a property rather than by bytes: this implementation has no
//! block structure at all, and `flux::tests::envelope_is_independent_of_chunking` asserts that a
//! 25-second signal agrees with its own parts.
//!
//! **The tolerances are `docs/CONFORMANCE.md` §3's**, unchanged: integers and the strike count
//! exactly, floats at `|a - b| <= ATOL + RTOL * |expected|`. That is sized for a different
//! *summation order*, which is exactly what separates `rustfft` from numpy's pocketfft and a
//! sequential sum from numpy's pairwise one — not for a different rule.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use flate2::read::GzDecoder;
use serde::Deserialize;
use trigger::flux::Geometry;
use trigger::{detect_in_envelope, envelope, offset_between, Strike};

/// `docs/CONFORMANCE.md` §3. Roughly six orders of magnitude above f64 epsilon and six below
/// anything this detector would call a difference.
const RTOL: f64 = 1e-9;
const ATOL: f64 = 1e-12;

#[derive(Deserialize)]
struct Vector {
    id: String,
    detector_version: u32,
    provenance: Provenance,
    input: Input,
    expected: Expected,
}

#[derive(Deserialize)]
struct Provenance {
    /// The vector id holding this one's other view, when there is one.
    #[serde(default)]
    offset_against: Option<String>,
}

#[derive(Deserialize)]
struct Input {
    rate: u32,
    envelope: Vec<f64>,
    excerpts: Vec<Excerpt>,
}

#[derive(Deserialize)]
struct Excerpt {
    #[allow(dead_code)]
    start_sample: usize,
    samples: Vec<i16>,
}

#[derive(Deserialize)]
struct Expected {
    hop: usize,
    window: usize,
    strikes: Vec<Strike>,
    excerpt_envelopes: Vec<Vec<f64>>,
    /// What `offset_between` made of this swing's two clips. Present on the face-on vector of a
    /// two-view swing and absent everywhere else; `None` means the reference *declined* the pair,
    /// which a port has to reproduce as a refusal rather than as a number (ADR-010 §2).
    #[serde(default)]
    offset: Option<Option<ExpectedOffset>>,
}

#[derive(Deserialize)]
struct ExpectedOffset {
    seconds: f64,
    r: f64,
    runner_up_r: f64,
}

/// `contracts/audio.py::AUDIO_DETECTOR_VERSION`. A vector recorded by a different generation of
/// the detector is not a conformance failure — it is a different question — so it is reported as
/// stale rather than diffed, the same way `conformance.py check` treats a stale engine version.
const AUDIO_DETECTOR_VERSION: u32 = 2;

fn spec_dir() -> PathBuf {
    // `CARGO_MANIFEST_DIR` is `crates/trigger`; the spec is two levels up, beside `pyproject.toml`.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../spec/vectors/audio")
        .canonicalize()
        .expect("spec/vectors/audio is missing — run `python scripts/conformance.py regenerate`")
}

fn vectors() -> Vec<Vector> {
    let mut paths: Vec<PathBuf> = fs::read_dir(spec_dir())
        .expect("cannot read spec/vectors/audio")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "gz"))
        .collect();
    // Discovered rather than listed, and sorted so a failure reports in a stable order — the same
    // choice `conformance.vector_paths` makes, for the same reason: a vector that exists on disk
    // and in no index is a vector nothing runs.
    paths.sort();
    assert!(!paths.is_empty(), "no audio vectors under {:?}", spec_dir());
    paths
        .iter()
        .map(|p| {
            let mut text = String::new();
            GzDecoder::new(fs::File::open(p).expect("open vector"))
                .read_to_string(&mut text)
                .expect("decompress vector");
            serde_json::from_str(&text).unwrap_or_else(|e| panic!("parse {p:?}: {e}"))
        })
        .collect()
}

fn close(actual: f64, expected: f64) -> bool {
    (actual - expected).abs() <= ATOL + RTOL * expected.abs()
}

/// How far apart two values are, as a fraction of the expected one.
///
/// Reported rather than only thresholded, because "it passed" says nothing about headroom. The
/// interesting question for this port is not whether `rustfft` agrees with numpy's pocketfft but
/// by how much — that margin is what says whether the tolerance is generous or whether the next
/// constant change will land on top of it.
fn relative(actual: f64, expected: f64) -> f64 {
    if expected == 0.0 {
        (actual - expected).abs()
    } else {
        (actual - expected).abs() / expected.abs()
    }
}

/// Every difference, not the first: one report that names four drifting bins is worth more than
/// four runs that each name one.
fn compare_envelope(actual: &[f64], expected: &[f64], what: &str, worst: &mut f64) -> Vec<String> {
    if actual.len() != expected.len() {
        return vec![format!(
            "{what}: {} values, expected {}",
            actual.len(),
            expected.len()
        )];
    }
    for (a, e) in actual.iter().zip(expected) {
        *worst = worst.max(relative(*a, *e));
    }
    actual
        .iter()
        .zip(expected)
        .enumerate()
        .filter(|(_, (a, e))| !close(**a, **e))
        .map(|(i, (a, e))| format!("{what}[{i}]: {a} != {e}"))
        .take(8)
        .collect()
}

/// A refusal compares equal to nothing but a refusal — `docs/CONFORMANCE.md` §3, ADR-010 §2.
///
/// An implementation that names a number where this one declined has turned "the two clips could
/// not be lined up" into an offset, and every frame it aligns afterwards inherits that.
fn compare_offset(
    actual: Option<trigger::ClipOffset>,
    expected: Option<&ExpectedOffset>,
    worst: &mut f64,
) -> Vec<String> {
    match (actual, expected) {
        (None, None) => Vec::new(),
        (Some(a), None) => vec![format!("offset: got {a:?}, expected a refusal")],
        (None, Some(e)) => vec![format!("offset: refused, expected {:+}s", e.seconds)],
        (Some(a), Some(e)) => {
            *worst = worst
                .max(relative(a.seconds, e.seconds))
                .max(relative(a.r, e.r))
                .max(relative(a.runner_up_r, e.runner_up_r));
            [
                ("seconds", a.seconds, e.seconds),
                ("r", a.r, e.r),
                ("runner_up_r", a.runner_up_r, e.runner_up_r),
            ]
            .into_iter()
            .filter(|(_, got, want)| !close(*got, *want))
            .map(|(field, got, want)| format!("offset.{field}: {got} != {want}"))
            .collect()
        }
    }
}

fn compare_strikes(actual: &[Strike], expected: &[Strike], worst: &mut f64) -> Vec<String> {
    if actual.len() != expected.len() {
        return vec![format!(
            "strikes: found {}, expected {} (samples {:?} vs {:?})",
            actual.len(),
            expected.len(),
            actual.iter().map(|s| s.sample).collect::<Vec<_>>(),
            expected.iter().map(|s| s.sample).collect::<Vec<_>>(),
        )];
    }
    let mut out = Vec::new();
    for (i, (a, e)) in actual.iter().zip(expected).enumerate() {
        *worst = worst
            .max(relative(a.confidence, e.confidence))
            .max(relative(a.prominence, e.prominence));
        // The sample index is an integer and is compared exactly. It is the measurement; a
        // tolerance on it would be a tolerance on *which hop the ball was in*, and the float
        // tolerance above exists for summation order, not for that.
        if a.sample != e.sample {
            out.push(format!("strike[{i}].sample: {} != {}", a.sample, e.sample));
        }
        if !close(a.confidence, e.confidence) {
            out.push(format!(
                "strike[{i}].confidence: {} != {}",
                a.confidence, e.confidence
            ));
        }
        if !close(a.prominence, e.prominence) {
            out.push(format!(
                "strike[{i}].prominence: {} != {}",
                a.prominence, e.prominence
            ));
        }
        // `None` compares equal to nothing but `None` — ADR-010 §2 applied to a frame index. A
        // port filling in 0 where the reference declines has turned "unknown" into "frame zero".
        if a.frame != e.frame {
            out.push(format!("strike[{i}].frame: {:?} != {:?}", a.frame, e.frame));
        }
    }
    out
}

#[test]
fn every_audio_vector_conforms() {
    let vectors = vectors();
    let by_id: std::collections::HashMap<&str, &Vector> =
        vectors.iter().map(|v| (v.id.as_str(), v)).collect();
    let mut failures = Vec::new();
    let mut worst = 0.0f64;
    let mut strikes_seen = 0usize;
    let mut offsets_seen = 0usize;

    for vector in &vectors {
        assert_eq!(
            vector.detector_version, AUDIO_DETECTOR_VERSION,
            "{}: recorded by detector v{}, this build is v{AUDIO_DETECTOR_VERSION}",
            vector.id, vector.detector_version
        );
        let mut diffs = Vec::new();

        // The geometry a port has to derive rather than be told.
        let geometry = Geometry::for_rate(vector.input.rate);
        if geometry.hop != vector.expected.hop || geometry.window != vector.expected.window {
            diffs.push(format!(
                "geometry: hop {} window {}, expected hop {} window {}",
                geometry.hop, geometry.window, vector.expected.hop, vector.expected.window
            ));
        }

        // Layer A — the statistics, over the whole clip's envelope.
        let found = detect_in_envelope(&vector.input.envelope, vector.input.rate, geometry);
        strikes_seen += vector.expected.strikes.len();
        diffs.extend(compare_strikes(
            &found,
            &vector.expected.strikes,
            &mut worst,
        ));

        // Layer B — the transform, over short full-rate excerpts.
        for (i, excerpt) in vector.input.excerpts.iter().enumerate() {
            let (actual, _) = envelope(&excerpt.samples, vector.input.rate);
            diffs.extend(compare_envelope(
                &actual,
                &vector.expected.excerpt_envelopes[i],
                &format!("excerpt[{i}]"),
                &mut worst,
            ));
        }

        // The other half of `audio/impact.py`: two views of one swing, correlated. The sibling's
        // envelope is already committed as its own vector's input, so this costs no extra bytes.
        if let Some(expected) = &vector.expected.offset {
            let sibling = vector
                .provenance
                .offset_against
                .as_ref()
                .and_then(|id| by_id.get(id.as_str()))
                .unwrap_or_else(|| panic!("{}: names a sibling that is not committed", vector.id));
            let actual = offset_between(
                &vector.input.envelope,
                &sibling.input.envelope,
                geometry,
                vector.input.rate,
                None,
            );
            offsets_seen += 1;
            diffs.extend(compare_offset(actual, expected.as_ref(), &mut worst));
        }

        if !diffs.is_empty() {
            failures.push(format!(
                "FAIL {}\n        {}",
                vector.id,
                diffs.join("\n        ")
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "{}/{} audio vectors conform\n{}",
        vectors.len() - failures.len(),
        vectors.len(),
        failures.join("\n")
    );
    // The headroom, not just the verdict. `rustfft` and numpy's pocketfft are different
    // algorithms and a sequential sum is not numpy's pairwise one, so some disagreement is
    // expected; what matters is that it stays where a reassociation lives and nowhere near where
    // a different measurement would.
    println!(
        "{} audio vectors conform: {strikes_seen} strikes, {offsets_seen} clip offsets;          worst relative difference {worst:.3e} (RTOL {RTOL:.0e})",
        vectors.len()
    );
}

/// The composition argument the two layers rest on, checked rather than assumed.
///
/// Layer A is handed an envelope and layer B checks that envelopes are computed correctly. That
/// only adds up to "`detect_strikes` is correct" if the whole-clip path really is
/// `envelope` followed by `detect_in_envelope` — so this runs the excerpts through both entry
/// points and requires the same answer.
#[test]
fn the_whole_clip_path_is_its_two_halves() {
    for vector in vectors() {
        let geometry = Geometry::for_rate(vector.input.rate);
        for excerpt in &vector.input.excerpts {
            let direct = trigger::detect_strikes(&excerpt.samples, vector.input.rate);
            let (env, _) = envelope(&excerpt.samples, vector.input.rate);
            let composed = detect_in_envelope(&env, vector.input.rate, geometry);
            assert_eq!(
                direct, composed,
                "{}: the two entry points disagree",
                vector.id
            );
        }
    }
}
