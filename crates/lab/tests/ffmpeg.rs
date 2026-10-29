//! `ffmpeg.rs` against clips this file makes: the twin of `tests/audio/test_ffmpeg_source.py`.
//! [M29 P21]
//!
//! The stored corpus reaches two of the decoder's answers and never the rest: 0 of its 30 clips
//! has no audio track, and none is asked for a stream it lacks, so those branches are pinned here
//! on synthetic clips rather than left to be met for the first time on a bay recording. The
//! fixtures are Python's, built by the same ffmpeg the lab runs (`Ffmpeg::find`).
//!
//! Ignored, because they need an ffmpeg. A fixture that cannot be built **fails** rather than
//! skipping as Python's does: an ignored test is run on purpose, and a pass with nothing built
//! would say the branch was checked when it was not.

use std::path::Path;
use std::process::Command;

use golf_lab::ffmpeg::{AudioError, Ffmpeg, TARGET_RATE};

fn ffmpeg() -> Ffmpeg {
    Ffmpeg::find().unwrap_or_else(|missing| panic!("{missing}"))
}

/// `_ffmpeg`: one quiet, overwriting run, which must succeed.
fn make(ffmpeg: &Ffmpeg, args: &[&str]) {
    let out = Command::new(ffmpeg.path())
        .args(["-y", "-hide_banner", "-loglevel", "error"])
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("could not run {}: {e}", ffmpeg.path().display()));
    assert!(
        out.status.success(),
        "a fixture could not be built ({args:?}): {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn s(path: &Path) -> &str {
    path.to_str().unwrap()
}

/// `_write_tone`: an AAC-in-MOV stereo tone at a rate other than the target, so every decode here
/// also resamples and downmixes. Encoding through AAC is what gives the file a real edit list.
fn write_tone(ffmpeg: &Ffmpeg, path: &Path, seconds: f64) {
    let source = format!("sine=frequency=440:sample_rate=44100:duration={seconds}");
    make(
        ffmpeg,
        &[
            "-f",
            "lavfi",
            "-i",
            &source,
            "-ac",
            "2",
            "-c:a",
            "aac",
            s(path),
        ],
    );
}

/// `_write_silent_video`: a video track and no audio track at all.
fn write_silent_video(ffmpeg: &Ffmpeg, path: &Path) {
    make(
        ffmpeg,
        &[
            "-f",
            "lavfi",
            "-i",
            "color=c=black:s=64x48:d=0.5:r=30",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
            s(path),
        ],
    );
}

/// The sample count decoded from sample zero with the edit list switched off:
/// `_decode_ignoring_the_edit_list`.
fn naive_sample_count(ffmpeg: &Ffmpeg, path: &Path) -> usize {
    let out = Command::new(ffmpeg.path())
        .args(["-y", "-hide_banner", "-loglevel", "error"])
        .args(["-ignore_editlist", "1", "-i", s(path)])
        .args(["-map", "0:a:0", "-vn", "-ac", "1", "-ar", "48000"])
        .args(["-f", "s16le", "-acodec", "pcm_s16le", "-"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    out.stdout.len() / 2
}

#[test]
#[ignore = "needs an ffmpeg (GOLF_FFMPEG or PATH)"]
fn a_stereo_tone_decodes_to_mono_pcm_at_the_target_rate() {
    let ffmpeg = ffmpeg();
    let dir = tempfile::tempdir().unwrap();
    let clip = dir.path().join("tone.mov");
    write_tone(&ffmpeg, &clip, 1.0);

    let decoded = ffmpeg.decode_audio(&clip, 0).unwrap();
    assert_eq!(decoded.sample_rate, TARGET_RATE);
    assert_eq!(decoded.stream_index, 0);
    // AAC pads both ends, so the decode runs a few tens of ms past the tone: the rate and the
    // channel count are the point, not the duration.
    assert!(
        (decoded.duration_s() - 1.0).abs() <= 0.1,
        "{}",
        decoded.duration_s()
    );
    let peak = decoded
        .samples
        .iter()
        .map(|&v| i32::from(v).abs())
        .max()
        .unwrap();
    assert!(peak > 1000, "a tone, not silence: peak {peak}");
}

#[test]
#[ignore = "needs an ffmpeg (GOLF_FFMPEG or PATH)"]
fn the_containers_edit_list_is_applied() {
    // The behaviour the arguments exist for (docs/M11_ACOUSTIC_SYNC.md §E2): the AAC priming is
    // recorded as an edit-list skip, and a decode from sample zero would inherit it as a leading
    // silence. The synthetic offset is smaller than the corpus's 2112 samples, so this pins a
    // direction and a floor.
    let ffmpeg = ffmpeg();
    let dir = tempfile::tempdir().unwrap();
    let clip = dir.path().join("tone.mov");
    write_tone(&ffmpeg, &clip, 1.0);

    let decoded = ffmpeg.decode_audio(&clip, 0).unwrap().samples.len();
    let naive = naive_sample_count(&ffmpeg, &clip);
    assert!(
        naive > decoded,
        "the edit list was not applied: {naive} against {decoded}"
    );
    assert!(
        (naive - decoded) as f64 >= 0.010 * f64::from(TARGET_RATE),
        "{naive} against {decoded}: under 10 ms trimmed"
    );
}

#[test]
#[ignore = "needs an ffmpeg (GOLF_FFMPEG or PATH)"]
fn the_stream_asked_for_is_the_one_decoded() {
    // Face-on clips carry two audio streams, so "the audio track" is not well defined.
    let ffmpeg = ffmpeg();
    let dir = tempfile::tempdir().unwrap();
    let (a, b, clip) = (
        dir.path().join("a.mov"),
        dir.path().join("b.mov"),
        dir.path().join("two_streams.mov"),
    );
    write_tone(&ffmpeg, &a, 1.0);
    write_tone(&ffmpeg, &b, 0.4);
    make(
        &ffmpeg,
        &[
            "-i",
            s(&a),
            "-i",
            s(&b),
            "-map",
            "0:a:0",
            "-map",
            "1:a:0",
            "-c",
            "copy",
            s(&clip),
        ],
    );

    let first = ffmpeg.decode_audio(&clip, 0).unwrap();
    let second = ffmpeg.decode_audio(&clip, 1).unwrap();
    assert_eq!((first.stream_index, second.stream_index), (0, 1));
    assert!(second.duration_s() < first.duration_s());
}

#[test]
#[ignore = "needs an ffmpeg (GOLF_FFMPEG or PATH)"]
fn no_audio_track_is_told_apart_from_a_broken_decode() {
    // Different answers downstream: an unanchorable bundle against a broken tool (ADR-010 §2).
    let ffmpeg = ffmpeg();
    let dir = tempfile::tempdir().unwrap();
    let silent = dir.path().join("silent.mov");
    write_silent_video(&ffmpeg, &silent);
    let tone = dir.path().join("tone.mov");
    write_tone(&ffmpeg, &tone, 0.3);

    assert_eq!(
        ffmpeg.decode_audio(&silent, 0),
        Err(AudioError::NoAudioTrack {
            name: "silent.mov".into(),
            stream_index: 0
        })
    );
    let absent = ffmpeg.decode_audio(&tone, 7).unwrap_err();
    assert_eq!(
        absent,
        AudioError::NoAudioTrack {
            name: "tone.mov".into(),
            stream_index: 7
        }
    );
    assert_eq!(
        absent.to_string(),
        "tone.mov has no audio stream at index 7"
    );

    let garbage = dir.path().join("garbage.mov");
    std::fs::write(&garbage, b"not a clip").unwrap();
    let broken = ffmpeg.decode_audio(&garbage, 0).unwrap_err();
    assert!(
        matches!(&broken, AudioError::Failed(m) if m.starts_with("ffmpeg could not decode audio from ")),
        "{broken:?}"
    );
}

#[test]
#[ignore = "needs an ffmpeg (GOLF_FFMPEG or PATH)"]
fn a_video_that_starts_late_reports_the_offset_and_one_that_does_not_reports_zero() {
    // Python's `_write_video_starting_late`: `-itsoffset` on the video input makes the MOV muxer
    // write the leading empty edit four of the corpus's down-the-line clips carry (§E2, 105–125
    // ms there). What is pinned is that the offset is seen, because the failure it guards is a
    // silent zero.
    let ffmpeg = ffmpeg();
    let dir = tempfile::tempdir().unwrap();
    let (video, audio, late) = (
        dir.path().join("late.video.mp4"),
        dir.path().join("late.audio.mov"),
        dir.path().join("late_video.mov"),
    );
    write_silent_video(&ffmpeg, &video);
    write_tone(&ffmpeg, &audio, 1.0);
    make(
        &ffmpeg,
        &[
            "-itsoffset",
            "0.1",
            "-i",
            s(&video),
            "-i",
            s(&audio),
            "-map",
            "0:v:0",
            "-map",
            "1:a:0",
            "-c",
            "copy",
            s(&late),
        ],
    );

    let start = ffmpeg.video_start_seconds(&late, 0).unwrap().unwrap();
    assert!((start - 0.1).abs() <= 0.005, "{start}");
    // A measured 0.0, which is what lets a missing value mean "nobody looked".
    assert_eq!(ffmpeg.video_start_seconds(&video, 0), Ok(Some(0.0)));
}

#[test]
#[ignore = "needs an ffmpeg (GOLF_FFMPEG or PATH)"]
fn a_clip_with_no_video_track_has_no_video_start() {
    // None, not zero: there are no frames for a sample index to land on (ADR-010 §2).
    let ffmpeg = ffmpeg();
    let dir = tempfile::tempdir().unwrap();
    let tone = dir.path().join("tone.mov");
    write_tone(&ffmpeg, &tone, 0.3);
    assert_eq!(ffmpeg.video_start_seconds(&tone, 0), Ok(None));
}
