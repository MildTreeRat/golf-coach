//! `golf-trigger` — strike detection over raw PCM on stdin. [M20 P3]
//!
//! ```text
//! golf-trigger detect --rate 48000 < clip.s16le    # offline: the AudioFile the pipeline stores
//! golf-trigger replay --rate 48000 --shots 1260000 < clip.s16le      # online, scored
//! golf-trigger replay --rate 48000 --shots 1260000 --min-z 24        # the same, different bar
//! ```
//!
//! **Raw PCM rather than a video path, and that is deliberate for now.** ADR-030 §1 gives Rust
//! clip cutting and therefore decoding, but `audio/ffmpeg.py` is 269 lines carrying real
//! subtleties — container edit lists, two `soun` tracks on the face-on clips with different
//! offsets, the `video_start_seconds` probe — and it reaches ffmpeg through the static binary the
//! `imageio-ffmpeg` wheel ships rather than through a system install. Porting decode is the next
//! thing to move and not this milestone's, so the seam is drawn where the subtlety is not: Python
//! decodes, this detects.
//!
//! The reply on stdout is JSON. For `detect` it is an `AudioFile` exactly as
//! `spec/schemas/audio_file.schema.json` describes it, so `api/pipeline.py` can parse it straight
//! into the model it already writes.

use std::io::{self, Read, Write};
use std::process::ExitCode;

use trigger::flux::Geometry;
use trigger::{detect_strikes, OnlineDetector, Strike, Trigger};

/// How close a trigger must land to a shot's first transient to count as having found it.
///
/// Generous on purpose: this scores *segmentation*, and a trigger 300 ms late still cuts a clip
/// with the swing in it. Where the strike actually was is the offline detector's question, asked
/// afterwards on the clip this cut.
const MATCH_WINDOW_S: f64 = 0.5;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let command = args.first().map(String::as_str).unwrap_or("");
    let rate: u32 = match flag(&args, "--rate").and_then(|v| v.parse().ok()) {
        Some(r) if r > 0 => r,
        _ => {
            eprintln!("--rate is required and must be a positive integer");
            return ExitCode::from(2);
        }
    };
    let samples = match read_pcm() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("could not read s16le PCM from stdin: {e}");
            return ExitCode::from(2);
        }
    };

    let out = match command {
        "detect" => detect(&samples, rate),
        "replay" => {
            let min_z = flag(&args, "--min-z").and_then(|v| v.parse().ok());
            let shots: Vec<usize> = flag(&args, "--shots")
                .map(|v| v.split(',').filter_map(|x| x.trim().parse().ok()).collect())
                .unwrap_or_default();
            replay(&samples, rate, min_z, &shots)
        }
        other => {
            eprintln!("unknown command {other:?}; expected `detect` or `replay`");
            return ExitCode::from(2);
        }
    };
    let mut stdout = io::stdout().lock();
    let _ = writeln!(stdout, "{out}");
    ExitCode::SUCCESS
}

fn flag<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .map(String::as_str)
}

/// s16le, little-endian, mono — what `audio/ffmpeg.py` already decodes to.
fn read_pcm() -> io::Result<Vec<i16>> {
    let mut bytes = Vec::new();
    io::stdin().lock().read_to_end(&mut bytes)?;
    if bytes.len() % 2 != 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "{} bytes is not a whole number of 16-bit samples",
                bytes.len()
            ),
        ));
    }
    Ok(bytes
        .chunks_exact(2)
        .map(|p| i16::from_le_bytes([p[0], p[1]]))
        .collect())
}

/// An `AudioFile` body: what `api/pipeline.py` stores as `{role}.audio.json`.
///
/// `clip` and `detector_version` are the caller's to fill — it is the caller that knows the
/// source hash, the stream index and the video timebase, and `contracts/audio.py` is explicit
/// that a detector handed a waveform and a rate knows none of those. Emitting a half-invented
/// `clip` block here would be a detector claiming to know what stream it was given.
fn detect(samples: &[i16], rate: u32) -> String {
    let strikes = detect_strikes(samples, rate);
    format!(
        "{{\"strikes\":{},\"sample_rate\":{rate},\"sample_count\":{}}}",
        json_strikes(&strikes),
        samples.len()
    )
}

fn json_strikes(strikes: &[Strike]) -> String {
    let rows: Vec<String> = strikes
        .iter()
        .map(|s| {
            format!(
                "{{\"sample\":{},\"confidence\":{},\"prominence\":{},\"frame\":null}}",
                s.sample, s.confidence, s.prominence
            )
        })
        .collect();
    format!("[{}]", rows.join(","))
}

/// Run the online detector over the samples and score it against ground truth handed in.
///
/// **`--shots` is the caller's to supply, and that is not laziness.** The first attempt derived
/// the ground truth here, by clustering `detect_strikes`' output into bursts — and it was wrong
/// in a way worth recording: `detect_strikes` is ground truth for *transients*, not for *shots*.
/// On `2026-08-07-aaron1/1 down_the_line` it lists a transient at 0.37 s with a third of the real
/// strike's prominence — the golfer setting up, not a second swing — and clustering scored that
/// as a shot the detector had missed. Deciding which transient was a ball is the shell's job
/// (`analysis/alignment.py::with_measured_impact` does it against the pose impact), exactly as
/// `contracts/audio.py` says deciding a strike's *frame* is. So the harness reads the impact the
/// pipeline actually anchored on, out of artifacts already on disk, and passes it in.
fn replay(samples: &[i16], rate: u32, min_z: Option<f64>, shots: &[usize]) -> String {
    let mut detector = match min_z {
        Some(z) => OnlineDetector::with_threshold(rate, z),
        None => OnlineDetector::new(rate),
    };
    // One push with everything: `stream::tests::the_block_size_does_not_change_the_answer` pins
    // that this is the same answer a live capture device's chunking would produce.
    let triggers = detector.push(samples);

    let window = (MATCH_WINDOW_S * f64::from(rate)) as i64;
    let mut matched = vec![false; shots.len()];
    let mut latencies: Vec<i64> = Vec::new();
    let mut spurious = 0usize;
    for trigger in &triggers {
        // Nearest unclaimed shot within the window; a second trigger on the same shot is a false
        // positive, not a second detection, which is what the refractory period exists to prevent.
        let hit = shots
            .iter()
            .enumerate()
            .filter(|(i, _)| !matched[*i])
            .map(|(i, &onset)| (i, trigger.sample as i64 - onset as i64))
            .filter(|(_, delta)| delta.abs() <= window)
            .min_by_key(|(_, delta)| delta.abs());
        match hit {
            Some((i, delta)) => {
                matched[i] = true;
                latencies.push(delta);
            }
            None => spurious += 1,
        }
    }

    let found = matched.iter().filter(|m| **m).count();
    format!(
        "{{\"shots\":{},\"triggers\":{},\"found\":{found},\"missed\":{},\"spurious\":{spurious},\
         \"latency_samples\":{:?},\"rate\":{rate},\"hop\":{},\"detail\":{}}}",
        shots.len(),
        triggers.len(),
        shots.len() - found,
        latencies,
        Geometry::for_rate(rate).hop,
        json_triggers(&triggers),
    )
}

fn json_triggers(triggers: &[Trigger]) -> String {
    let rows: Vec<String> = triggers
        .iter()
        .map(|t| {
            format!(
                "{{\"sample\":{},\"z\":{},\"confidence\":{}}}",
                t.sample, t.z, t.confidence
            )
        })
        .collect();
    format!("[{}]", rows.join(","))
}
