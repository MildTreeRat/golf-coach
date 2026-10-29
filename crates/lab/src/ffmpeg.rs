//! One clip's audio as mono PCM, and when its video starts: `audio/ffmpeg.py`. [M29 P21]
//!
//! The twin of `FfmpegAudioSource.read` and `video_start_seconds`, with frozen Python's ffmpeg
//! arguments verbatim. The arguments are the design: ffmpeg applies each track's edit list, which
//! is how the 2112 samples of AAC priming most face-on tracks carry are skipped on the track that
//! has them and not on the one that does not (docs/M11_ACOUSTIC_SYNC.md §E2), and `showinfo` over
//! a passthrough decode is how the empty edit that starts four down-the-line clips' video 105–125
//! ms late is read off the decoder rather than re-derived from the atoms. Python's docstrings carry
//! the measurements behind each flag; they are not repeated here.
//!
//! **Which ffmpeg differs, and so does what it decodes.** Python runs the static 7.1 the
//! `imageio-ffmpeg` wheel ships. This runs the one [`Ffmpeg::find`] finds, `GOLF_FFMPEG` and then
//! `PATH` (the M29 plan's D17), through `ocr::decode::find_ffmpeg`, the lab's one finder; on this
//! box that is 9.0.2. Measured over the 30 stored clips with these arguments (the plan's P21
//! findings), the two do not decode the same waveform:
//!
//! - **9.0.2 ends each audio track where its edit list ends**, on all 30 (its sample count is the
//!   track's edit duration, `ffprobe`'s `duration_ts`). 7.1 decodes every AAC frame after the
//!   priming whole, so on 26 of the 30 it runs 16 to 976 samples longer. The four that agree are
//!   the four down-the-line clips with the 19200 video timescale and the late video (§E2).
//! - Over the samples both decode, 27 of the 30 are bit-identical, and each of the other three
//!   (2026-08-23's down-the-line clips 3, 7 and 8) differs in one sample by one level.
//!
//! In a stored `audio.json` that moves three floats and nothing the engine reads: `duration_s`,
//! and each strike's `confidence` and `prominence`, by at most a relative 1.25e-3 over the 30,
//! while every strike's sample, frame and rank, and the clip's fps and video start, come out
//! exactly. `tests/strikes.rs` gates this ffmpeg, the one the lab runs: those three to a relative
//! 2e-3 and everything else exactly, which is the user's answer when P21 stopped on the
//! difference. `.venv`'s 7.1 is not a fallback: it leaves with the `audio` extra in M40, and it
//! decodes the bay's HEIC photo as one tile (D17).
//!
//! **No `camera_id` on [`AudioClip`]**, where Python's carries one: `audio_for` passes the role in
//! and never reads it back, and nothing else here decodes audio.

use std::fmt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// `_TARGET_RATE`: one rate for everything downstream. Every clip in the corpus is already 48 kHz
/// stereo `mp4a` (§E1), so this resamples nothing today and pins the contract for footage that
/// arrives otherwise.
pub const TARGET_RATE: u32 = 48_000;

/// `_NO_STREAM_MARKERS`: what ffmpeg says on stderr when `-map` selects a stream the file does not
/// have, matched lowercased because the capitalisation has moved between releases. Both releases
/// on this box say the first (`Stream map '' matches no streams.`).
const NO_STREAM_MARKERS: [&str; 2] = ["matches no streams", "does not contain any stream"];

/// One clip's audio as mono 16-bit PCM, after the container's edit list has been applied.
/// `audio/source.py::AudioClip`.
#[derive(Debug, Clone, PartialEq)]
pub struct AudioClip {
    /// Signed 16-bit, one channel, at `sample_rate` Hz.
    pub samples: Vec<i16>,
    pub sample_rate: u32,
    /// Which audio stream was decoded: face-on clips carry two `soun` tracks with different edit
    /// offsets, so a sample index means something only beside its stream.
    pub stream_index: u32,
}

impl AudioClip {
    /// The decoded waveform's length in seconds, 0.0 for a rate-less decode: `duration_s`.
    /// Derived from the sample count, so it carries the edit list's effect, which the container's
    /// own `duration` field does not.
    pub fn duration_s(&self) -> f64 {
        if self.sample_rate == 0 {
            return 0.0;
        }
        self.samples.len() as f64 / f64::from(self.sample_rate)
    }
}

/// Why a clip's audio, or its video timebase, could not be read. Python raises three exception
/// types for these, and `audio_for` answers each with its own note.
#[derive(Debug, Clone, PartialEq)]
pub enum AudioError {
    /// `FileNotFoundError`: the clip is not on disk.
    Missing(PathBuf),
    /// `NoAudioTrackError`: the clip opened and has no audio stream at that index, which is a fact
    /// about the footage (it cannot be anchored on the strike), not a broken tool. Measured 0 of
    /// 30 in this corpus, which is why `tests/ffmpeg.rs` makes such a clip rather than waiting for
    /// one, as Python's `tests/audio/test_ffmpeg_source.py` does.
    NoAudioTrack { name: String, stream_index: u32 },
    /// `OSError`: ffmpeg could not be run, exited non-zero for another reason, or handed back a
    /// partial sample. The message is Python's where Python has one.
    Failed(String),
}

impl fmt::Display for AudioError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AudioError::Missing(path) => write!(f, "Video file not found: {}", path.display()),
            AudioError::NoAudioTrack { name, stream_index } => {
                write!(f, "{name} has no audio stream at index {stream_index}")
            }
            AudioError::Failed(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for AudioError {}

/// No ffmpeg anywhere it was looked for: the Rust form of Python's "can this machine listen?".
///
/// Python asks two questions there (is the `audio` extra installed, has `golf-trigger` been
/// built), because its decoder and its detector are both outside the interpreter. Here the
/// detector is linked in, so ffmpeg is the one thing a machine can lack, and `audio_for` answers
/// its absence on the same branch Python answers either of those on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FfmpegMissing {
    /// Every place looked, in order, as `ocr::decode::find_ffmpeg` reports them.
    pub tried: Vec<String>,
}

impl fmt::Display for FfmpegMissing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "no ffmpeg was found (tried {}); install ffmpeg or set {} to its path",
            self.tried.join(", "),
            ocr::decode::FFMPEG_ENV
        )
    }
}

impl std::error::Error for FfmpegMissing {}

/// An ffmpeg binary to run. Found once per run and handed to every view, so a machine without one
/// is told once which places were tried.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ffmpeg {
    path: PathBuf,
}

impl Ffmpeg {
    /// `GOLF_FFMPEG`, then `PATH` (D17), through `ocr::decode::find_ffmpeg`.
    pub fn find() -> Result<Ffmpeg, FfmpegMissing> {
        ocr::decode::find_ffmpeg()
            .map(|path| Ffmpeg { path })
            .map_err(|tried| FfmpegMissing { tried })
    }

    /// This binary, unchecked: a path that is not one is refused when it is run, by name.
    pub fn at(path: impl Into<PathBuf>) -> Ffmpeg {
        Ffmpeg { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Decode stream `stream_index`'s audio to mono PCM at [`TARGET_RATE`]:
    /// `FfmpegAudioSource(path, stream_index=...).read()`.
    pub fn decode_audio(&self, clip: &Path, stream_index: u32) -> Result<AudioClip, AudioError> {
        if !clip.exists() {
            return Err(AudioError::Missing(clip.to_path_buf()));
        }
        let output = self.run(
            Command::new(&self.path)
                .args(["-hide_banner", "-nostdin", "-loglevel", "error", "-i"])
                .arg(clip)
                .args([
                    // Explicitly the Nth audio stream: face-on clips carry the AAC track and an
                    // `apac` one, and which comes out without `-map` is ffmpeg's heuristic.
                    "-map",
                    &format!("0:a:{stream_index}"),
                    "-vn",
                    "-ac",
                    "1",
                    "-ar",
                    &TARGET_RATE.to_string(),
                    "-f",
                    "s16le",
                    "-acodec",
                    "pcm_s16le",
                    // `-ignore_editlist` is absent on purpose: applying the edit list is the point.
                    "-",
                ]),
        )?;
        if !output.status.success() {
            let message = stderr_text(&output);
            return Err(if is_no_stream(&message) {
                AudioError::NoAudioTrack {
                    name: file_name(clip),
                    stream_index,
                }
            } else {
                AudioError::Failed(format!(
                    "ffmpeg could not decode audio from {}: {message}",
                    clip.display()
                ))
            });
        }
        let payload = output.stdout;
        // A truncated tail means ffmpeg was cut off mid-sample: a decode failure wearing a success
        // exit code.
        if payload.len() % 2 != 0 {
            return Err(AudioError::Failed(format!(
                "ffmpeg returned {} bytes of 16-bit PCM for {}, which is not a whole number of \
                 samples",
                payload.len(),
                file_name(clip)
            )));
        }
        // `-f s16le` is little-endian by name, so this reads it as such on any host, where
        // Python's `array("h")` is native and byteswaps on a big-endian one.
        let samples = payload
            .chunks_exact(2)
            .map(|pair| i16::from_le_bytes([pair[0], pair[1]]))
            .collect();
        Ok(AudioClip {
            samples,
            sample_rate: TARGET_RATE,
            stream_index,
        })
    }

    /// Presentation time of the first frame a decoder hands back from this clip:
    /// `video_start_seconds`. Almost always 0.0; 0.105 to 0.125 s on the four down-the-line clips
    /// whose containers carry a leading empty edit, measured the same by 7.1 and 9.0.2.
    ///
    /// `None` when the clip has no such video stream, and when ffmpeg prints no timestamp at all:
    /// nobody measured it, which is never zero. `-fps_mode passthrough` matters, because ffmpeg's
    /// default output mode duplicates the first frame to fill the empty edit, and a timestamp read
    /// off that padding would be 0.0 and useless.
    pub fn video_start_seconds(
        &self,
        clip: &Path,
        stream_index: u32,
    ) -> Result<Option<f64>, AudioError> {
        if !clip.exists() {
            return Err(AudioError::Missing(clip.to_path_buf()));
        }
        let output = self.run(
            Command::new(&self.path)
                // `showinfo` logs at info level, so this is the one call here that cannot be quiet.
                .args(["-hide_banner", "-nostdin", "-loglevel", "info", "-i"])
                .arg(clip)
                .args([
                    "-map",
                    &format!("0:v:{stream_index}"),
                    "-fps_mode",
                    "passthrough",
                    "-vf",
                    "showinfo",
                    // One frame is the whole measurement, and the first frame is a keyframe.
                    "-frames:v",
                    "1",
                    "-f",
                    "null",
                    "-",
                ]),
        )?;
        if !output.status.success() {
            let message = stderr_text(&output);
            if is_no_stream(&message) {
                return Ok(None);
            }
            return Err(AudioError::Failed(format!(
                "ffmpeg could not read the video timebase of {}: {message}",
                clip.display()
            )));
        }
        Ok(first_pts_time(&output.stderr))
    }

    /// Run one ffmpeg command to completion, both streams captured, as `subprocess.run` with
    /// `capture_output=True` does. A binary that cannot be started is an `OSError` in Python too.
    fn run(&self, command: &mut Command) -> Result<Output, AudioError> {
        command.output().map_err(|error| {
            AudioError::Failed(format!("could not run {}: {error}", self.path.display()))
        })
    }
}

/// `stderr.decode("utf-8", errors="replace").strip()`.
fn stderr_text(output: &Output) -> String {
    pyfmt::strip(&String::from_utf8_lossy(&output.stderr)).to_string()
}

fn is_no_stream(message: &str) -> bool {
    let haystack = pyfmt::lower(message);
    NO_STREAM_MARKERS
        .iter()
        .any(|marker| haystack.contains(marker))
}

/// `Path.name`, for the two messages Python words with it.
fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// The first `pts_time:` on `showinfo`'s stderr, as `_PTS_TIME.search` finds it:
/// `rb"pts_time:\s*([0-9]+\.?[0-9]*)"`, so digits, then an optional point and more digits, after
/// any ASCII whitespace. An occurrence the pattern does not match there (a negative or `NOPTS`
/// time) is passed over for the next, as the regex search passes over it.
fn first_pts_time(stderr: &[u8]) -> Option<f64> {
    const KEY: &[u8] = b"pts_time:";
    let digits_from = |mut at: usize| {
        while stderr.get(at).is_some_and(u8::is_ascii_digit) {
            at += 1;
        }
        at
    };
    let mut from = 0;
    while let Some(found) = stderr[from..]
        .windows(KEY.len())
        .position(|window| window == KEY)
    {
        let mut at = from + found + KEY.len();
        // Python's bytes `\s`: space, \t, \n, \r, \f and \v.
        while stderr
            .get(at)
            .is_some_and(|&b| matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c))
        {
            at += 1;
        }
        let start = at;
        let mut end = digits_from(start);
        if end > start {
            if stderr.get(end) == Some(&b'.') {
                end = digits_from(end + 1);
            }
            // ASCII digits and a point, so always UTF-8, and `"12."` parses as `float()` reads it.
            return std::str::from_utf8(&stderr[start..end]).ok()?.parse().ok();
        }
        from += found + 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_timestamp_showinfo_prints_is_read() {
        let line = b"[Parsed_showinfo_0 @ 0x1] n:   0 pts:   1260 pts_time:0.105   duration:  10";
        assert_eq!(first_pts_time(line), Some(0.105));
        assert_eq!(first_pts_time(b"pts_time:0 duration"), Some(0.0));
        assert_eq!(first_pts_time(b"pts_time:\t 12. x"), Some(12.0));
        assert_eq!(
            first_pts_time(b"pts_time:0.1 n:1 pts_time:0.2"),
            Some(0.1),
            "the first wins"
        );
    }

    #[test]
    fn a_timestamp_the_pattern_does_not_match_is_passed_over() {
        assert_eq!(
            first_pts_time(b"pts_time:NOPTS pts_time:0.125"),
            Some(0.125)
        );
        assert_eq!(
            first_pts_time(b"pts_time:-0.5 x"),
            None,
            "no sign in the pattern"
        );
        assert_eq!(first_pts_time(b"pts_time:.5"), None, "a digit comes first");
        assert_eq!(first_pts_time(b""), None);
        assert_eq!(first_pts_time(b"pts_time:"), None);
    }

    #[test]
    fn a_missing_stream_is_told_from_a_broken_decode() {
        assert!(is_no_stream(
            "Stream map '' matches no streams.\nTo ignore this, add ..."
        ));
        assert!(is_no_stream("Output file DOES NOT CONTAIN ANY STREAM"));
        assert!(!is_no_stream("Invalid data found when processing input"));
    }

    #[test]
    fn a_missing_clip_is_refused_before_ffmpeg_is_run() {
        let ffmpeg = Ffmpeg::at("no-such-ffmpeg");
        let clip = Path::new("no-such-clip.MOV");
        let missing = AudioError::Missing(clip.to_path_buf());
        assert_eq!(ffmpeg.decode_audio(clip, 0), Err(missing.clone()));
        assert_eq!(ffmpeg.video_start_seconds(clip, 0), Err(missing.clone()));
        assert_eq!(
            missing.to_string(),
            "Video file not found: no-such-clip.MOV"
        );
    }

    #[test]
    fn an_ffmpeg_that_cannot_be_run_is_a_named_failure() {
        let dir = tempfile::tempdir().unwrap();
        let clip = dir.path().join("face_on.MOV");
        std::fs::write(&clip, b"not a clip").unwrap();
        let ffmpeg = Ffmpeg::at(dir.path().join("no-ffmpeg-here.exe"));
        for error in [
            ffmpeg.decode_audio(&clip, 0).unwrap_err(),
            ffmpeg.video_start_seconds(&clip, 0).unwrap_err(),
        ] {
            let AudioError::Failed(message) = error else {
                panic!("{error:?}");
            };
            assert!(message.starts_with("could not run "), "{message}");
            assert!(message.contains("no-ffmpeg-here.exe"), "{message}");
        }
    }

    #[test]
    fn duration_is_the_decoded_length() {
        let clip = AudioClip {
            samples: vec![0; 72_000],
            sample_rate: TARGET_RATE,
            stream_index: 0,
        };
        assert_eq!(clip.duration_s(), 1.5);
        assert_eq!(
            AudioClip {
                sample_rate: 0,
                ..clip
            }
            .duration_s(),
            0.0
        );
    }

    #[test]
    fn a_missing_ffmpeg_says_where_it_looked_and_how_to_point_at_one() {
        let missing = FfmpegMissing {
            tried: vec![
                "GOLF_FFMPEG (not set)".into(),
                "every PATH directory".into(),
            ],
        };
        assert_eq!(
            missing.to_string(),
            "no ffmpeg was found (tried GOLF_FFMPEG (not set), every PATH directory); install \
             ffmpeg or set GOLF_FFMPEG to its path"
        );
    }
}
