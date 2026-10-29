//! One view's strikes, from cache when the clip behind them has not changed: `api/pipeline.py`'s
//! `audio_for`, `_frames_derived` and `_frame_of`. [M29 P21]
//!
//! The cache rule is Python's, decision for decision, because frozen Python reads and re-detects
//! these same files until M40 (ADR-035 clause 4). A `{role}.audio.json` is read rather than
//! re-detected when it was detected from **this clip** (its `source_sha256` against the
//! manifest's) **by this detector** (`AUDIO_DETECTOR_VERSION`, which `contracts`' own pin holds to
//! frozen Python's). A cache from an older detector is re-detected, and kept, with a note saying
//! so, when this machine cannot re-detect. The log lines and the notes are Python's text, so a lab
//! run reads the same in either language.
//!
//! **Detection is in-process, so one of Python's failures has no twin.** Python spawns
//! `golf-trigger` and answers a binary that fails mid-run with a note and the older detection;
//! here `trigger::detect_strikes` is a pure function over the waveform and cannot fail. "Can this
//! machine listen?" is therefore one question, whether an ffmpeg was found ([`FfmpegMissing`]),
//! asked before the decode as Python asks its two.
//!
//! **A broken cache stops the bundle**, as it does in Python: `audio_for` calls `load_audio` with
//! no `try`, and so does `analyze_swing_dir`. [`audio_for`] answers it as `Err`, and a failed write
//! the same way. Every other way a view can have no strikes is `Ok(None)` and a note.
//!
//! **`fps` is the caller's**, as there: the detector never sees the video, so a frame index is
//! derived, and the caller holding the pose's `ClipMetadata.fps` is the one that knows the rate.

use std::path::Path;

use contracts::audio::{AudioClipMetadata, AudioFile, AudioStrike, AUDIO_DETECTOR_VERSION};
use storage::audio_io::{audio_path, load_audio, save_audio};
use storage::manifest::{Role, SwingManifest};
use storage::StoreError;

use crate::ffmpeg::{AudioError, Ffmpeg, FfmpegMissing};
use crate::file_name;

/// `audio_for`'s keyword arguments.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct AudioOptions {
    /// The fps this view's pose was decoded at, which turns a sample index into a frame index in
    /// this clip's numbering. `None` leaves every strike's `frame` unset, which is honest.
    pub fps: Option<f64>,
    /// Re-detect even when the cache is current (`--force-audio`).
    pub force: bool,
}

/// The strikes in one view's clip: `audio_for`. Writes `{role}.audio.json` when it detects, and when
/// it derives frames a cached file did not have.
///
/// `Ok(None)` when there is nothing to detect in: no such role, the clip missing from disk, no
/// ffmpeg, no audio stream in the container, or a decode that failed, each with a note. **An empty
/// `strikes` list is not one of those**: it is a result, the one a rehearsal swing gives.
pub fn audio_for(
    swing_dir: &Path,
    manifest: &SwingManifest,
    role: Role,
    options: AudioOptions,
    ffmpeg: Result<&Ffmpeg, &FfmpegMissing>,
    log: &mut dyn FnMut(&str),
    notes: &mut Vec<String>,
) -> Result<Option<AudioFile>, StoreError> {
    let Some(role_file) = manifest.roles.get(&role) else {
        return Ok(None);
    };
    let name = role.as_str();
    let fps = options.fps;
    let cache_path = audio_path(swing_dir, role);
    let cache_name = file_name(&cache_path);

    // Same clip, older detector: worth re-detecting, and worth keeping if this machine cannot.
    let mut superseded: Option<AudioFile> = None;
    if cache_path.exists() && !options.force {
        let cached = load_audio(&cache_path)?;
        let same_clip = cached
            .clip
            .as_ref()
            .and_then(|clip| clip.source_sha256.as_deref())
            == Some(role_file.content_sha256.as_str());
        if !same_clip {
            log(&format!(
                "  {name}: cached audio is for a different clip, re-detecting"
            ));
        } else if cached.detector_version != AUDIO_DETECTOR_VERSION {
            // A cache the current detector would disagree with, which the sha cannot see: it keys
            // on the clip, and the clip has not changed.
            log(&format!(
                "  {name}: cached audio is from detector v{}, re-detecting at \
                 v{AUDIO_DETECTOR_VERSION}",
                cached.detector_version
            ));
            superseded = Some(cached);
        } else {
            log(&format!(
                "  {name}: {} strikes from cache",
                cached.strikes.len()
            ));
            return Ok(Some(match frames_derived(&cached, fps) {
                // Detected by a caller that did not know the fps, and this one does. Re-deriving
                // beats re-detecting, since the sample index is the measurement, and writing it
                // back gives the next reader the frames with the fps beside them.
                Some(framed) => {
                    save_audio(&framed, &cache_path)?;
                    log(&format!(
                        "  {name}: frames derived at {} fps -> {cache_name}",
                        fps.map(pyfmt::repr).unwrap_or_default()
                    ));
                    framed
                }
                None => cached,
            }));
        }
    }

    let video_path = swing_dir.join(&role_file.filename);
    if !video_path.exists() {
        log(&format!(
            "  {name}: {} is missing from the swing directory",
            file_name(&video_path)
        ));
        notes.push(format!(
            "the {name} clip is recorded in the manifest but missing from disk"
        ));
        return Ok(None);
    }

    let ffmpeg = match ffmpeg {
        Ok(ffmpeg) => ffmpeg,
        Err(why) => {
            // An older detection beats none: a stored strike list is a measurement, where the pose
            // impact it would otherwise fall back to is an estimate. Said out loud, with the
            // reason, so a reader comparing this run with one that could listen sees why.
            if let Some(older) = superseded {
                log(&format!(
                    "  {name}: cannot re-detect ({why}), keeping the older detection"
                ));
                notes.push(format!(
                    "the {name} strikes were found by an older detector (v{}) and this machine \
                     cannot re-run it: {why}",
                    older.detector_version
                ));
                return Ok(Some(frames_derived(&older, fps).unwrap_or(older)));
            }
            log(&format!("  {name}: no strike detection — {why}"));
            notes.push(format!("no audio for {name}: {why}"));
            return Ok(None);
        }
    };

    log(&format!(
        "  {name}: decoding audio from {} ...",
        role_file.original_filename
    ));
    let clip = match ffmpeg.decode_audio(&video_path, 0) {
        Ok(clip) => clip,
        Err(AudioError::NoAudioTrack { .. }) => {
            // A fact about the footage, not a broken tool: this bundle cannot be anchored on the
            // strike, which is worth recording rather than falling through.
            log(&format!("  {name}: the clip carries no audio track"));
            notes.push(format!(
                "the {name} clip has no audio track, so it cannot be anchored on the ball strike"
            ));
            return Ok(None);
        }
        Err(error) => {
            log(&format!("  {name}: audio could not be decoded — {error}"));
            notes.push(format!(
                "the {name} clip's audio could not be decoded, so no strike was detected in it"
            ));
            return Ok(None);
        }
    };

    // The second clock (M11 P10), measured while the clip is at hand. A probe that fails is a
    // note, not a failed decode: the strikes are still the measurement.
    let video_start_s = match ffmpeg.video_start_seconds(&video_path, 0) {
        Ok(start) => {
            if let Some(start) = start.filter(|&start| start != 0.0) {
                log(&format!(
                    "  {name}: video starts {} ms after the audio",
                    pyfmt::fixed(start * 1000.0, 0)
                ));
            }
            start
        }
        Err(error) => {
            log(&format!(
                "  {name}: could not read the video timebase — {error}"
            ));
            notes.push(format!(
                "the {name} clip's video timebase could not be read, so its strike frames assume \
                 the video starts where the audio does"
            ));
            None
        }
    };

    let strikes: Vec<AudioStrike> = trigger::detect_strikes(&clip.samples, clip.sample_rate)
        .into_iter()
        .map(|strike| AudioStrike {
            sample: strike.sample as i64,
            confidence: strike.confidence,
            prominence: strike.prominence,
            // The detector has never seen the video; `frames_derived` below fills this in.
            frame: None,
        })
        .collect();
    let detected = AudioFile {
        clip: Some(AudioClipMetadata {
            sample_rate: Some(i64::from(clip.sample_rate)),
            // The decoded length, which carries the edit list's effect (§E2).
            duration_s: Some(clip.duration_s()),
            source_sha256: Some(role_file.content_sha256.clone()),
            stream_index: Some(i64::from(clip.stream_index)),
            fps: None,
            video_start_s,
        }),
        strikes,
        detector_version: AUDIO_DETECTOR_VERSION,
    };
    let audio = frames_derived(&detected, fps).unwrap_or(detected);
    save_audio(&audio, &cache_path)?;
    log(&format!(
        "  {name}: {} strikes -> {cache_name}",
        audio.strikes.len()
    ));
    Ok(Some(audio))
}

/// `audio` with every strike's frame worked out under `fps`: `_frames_derived`. **`None` means
/// `audio` itself**, where Python returns the same object, which is how [`audio_for`] tells
/// "already derived" from "just derived" and writes only the second.
///
/// Derived, never guessed: with no fps, no sample rate, or a list already carrying this fps's
/// frames, nothing is invented and nothing recorded is thrown away, so a caller that does not know
/// the rate cannot erase the frames a caller that did wrote (ADR-010 §2). The result is compared
/// rather than assumed current, because `frame = None` has two meanings: unknown, and a strike
/// ahead of the first decoded frame, which keeps `None` forever and must not be re-derived and
/// re-saved on every read. The detector version is carried, never re-stamped: arithmetic on a
/// sample index does not turn an older detector's list into this one's.
pub fn frames_derived(audio: &AudioFile, fps: Option<f64>) -> Option<AudioFile> {
    let fps = fps.filter(|&fps| fps > 0.0)?;
    let clip = audio.clip.as_ref()?;
    let rate = clip.sample_rate.filter(|&rate| rate != 0)?;
    // `video_start_s or 0.0`: absent is the assumption every reader made before the field existed.
    let start = clip.video_start_s.unwrap_or(0.0);
    let framed: Vec<AudioStrike> = audio
        .strikes
        .iter()
        .map(|strike| AudioStrike {
            frame: frame_of(strike.sample, rate, fps, start),
            ..strike.clone()
        })
        .collect();
    if clip.fps == Some(fps) && framed == audio.strikes {
        return None;
    }
    Some(AudioFile {
        clip: Some(AudioClipMetadata {
            fps: Some(fps),
            ..clip.clone()
        }),
        strikes: framed,
        detector_version: audio.detector_version,
    })
}

/// Which decoded frame a sample index lands on, or `None` when it precedes the first: `_frame_of`.
///
/// A floor, not a truncation: a frame index answers "which frame was being exposed", and `int()`
/// would put a strike 0.4 frames before the video started on frame 0, hiding the case this answers
/// `None` for. The video's start is subtracted first, which is the whole of M11 P10: a sample index
/// is on the presentation clock, a frame index counts from the first frame the decoder hands back.
pub fn frame_of(sample: i64, rate: i64, fps: f64, video_start_s: f64) -> Option<i64> {
    let frame = ((sample as f64 / rate as f64 - video_start_s) * fps).floor();
    (frame >= 0.0).then_some(frame as i64)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    const CLIP_SHA: &str = "b2a5e44ef3d72e71137e7775738596999867d67a10e5d5c0fd567273e6ee3aeb";

    fn manifest() -> SwingManifest {
        serde_json::from_value(serde_json::json!({
            "swing_id": "9", "session_id": "2026-08-23",
            "created_at": "2026-08-23T04:50:20.421608Z",
            "updated_at": "2026-08-23T04:50:29.716126Z",
            "roles": {"down_the_line": {
                "role": "down_the_line", "filename": "down_the_line.b2a5e44ef3d7.MOV",
                "content_sha256": CLIP_SHA, "original_filename": "shot-9-back.MOV",
                "content_type": "video/quicktime", "size_bytes": 51105875,
                "received_at": "2026-08-23T04:50:25.978185Z", "warnings": []}}
        }))
        .unwrap()
    }

    /// Bundle 9's stored down-the-line file, whose video starts 105 ms after its audio.
    fn stored(fps: Option<f64>, frames: [Option<i64>; 2], version: i64) -> AudioFile {
        let strike = |sample, confidence, prominence, frame| AudioStrike {
            sample,
            confidence,
            prominence,
            frame,
        };
        AudioFile {
            clip: Some(AudioClipMetadata {
                sample_rate: Some(48_000),
                duration_s: Some(7.573333333333333),
                source_sha256: Some(CLIP_SHA.into()),
                stream_index: Some(0),
                fps,
                video_start_s: Some(0.105),
            }),
            strikes: vec![
                strike(262_320, 0.9554449193033062, 19232199.970034547, frames[0]),
                strike(267_600, 0.951155176636159, 17464388.25427588, frames[1]),
            ],
            detector_version: version,
        }
    }

    const FPS: f64 = 59.9602911978822;

    struct Run {
        result: Result<Option<AudioFile>, StoreError>,
        log: Vec<String>,
        notes: Vec<String>,
    }

    fn run(dir: &Path, options: AudioOptions, ffmpeg: Result<&Ffmpeg, &FfmpegMissing>) -> Run {
        let mut log = Vec::new();
        let mut notes = Vec::new();
        let result = audio_for(
            dir,
            &manifest(),
            Role::DownTheLine,
            options,
            ffmpeg,
            &mut |line: &str| log.push(line.to_string()),
            &mut notes,
        );
        Run { result, log, notes }
    }

    fn no_ffmpeg() -> FfmpegMissing {
        FfmpegMissing {
            tried: vec![
                "GOLF_FFMPEG (not set)".into(),
                "every PATH directory".into(),
            ],
        }
    }

    #[test]
    fn a_frame_is_the_floor_after_the_video_start_and_none_before_it() {
        // Bundle 9's down-the-line ball strike: frame 321 under the 0.105 s empty edit, and 327
        // without it, the 6.3-frame error M11 P10 removed.
        assert_eq!(frame_of(262_320, 48_000, FPS, 0.105), Some(321));
        assert_eq!(frame_of(262_320, 48_000, FPS, 0.0), Some(327));
        assert_eq!(frame_of(0, 48_000, 60.0, 0.0), Some(0));
        // 0.4 frames before the first decoded one: `int()` would say 0.
        assert_eq!(frame_of(0, 48_000, 60.0, 0.4 / 60.0), None);
        assert_eq!(frame_of(799, 48_000, 60.0, 0.0), Some(0));
        assert_eq!(frame_of(801, 48_000, 60.0, 0.0), Some(1));
    }

    #[test]
    fn frames_are_derived_only_where_something_is_added() {
        let underived = stored(None, [None, None], 2);
        let derived = frames_derived(&underived, Some(FPS)).expect("frames to add");
        assert_eq!(derived, stored(Some(FPS), [Some(321), Some(327)], 2));
        assert_eq!(frames_derived(&derived, Some(FPS)), None, "already derived");

        for fps in [None, Some(0.0), Some(-1.0)] {
            assert_eq!(
                frames_derived(&derived, fps),
                None,
                "{fps:?} erases nothing"
            );
        }
        let mut no_rate = underived.clone();
        no_rate.clip.as_mut().unwrap().sample_rate = None;
        assert_eq!(frames_derived(&no_rate, Some(FPS)), None);
        no_rate.clip.as_mut().unwrap().sample_rate = Some(0);
        assert_eq!(frames_derived(&no_rate, Some(FPS)), None);
        let no_clip = AudioFile {
            clip: None,
            ..underived.clone()
        };
        assert_eq!(frames_derived(&no_clip, Some(FPS)), None);

        // A different fps re-derives, and the version is carried, not re-stamped.
        let older = stored(Some(30.0), [Some(160), Some(163)], 1);
        let again = frames_derived(&older, Some(FPS)).unwrap();
        assert_eq!(again, stored(Some(FPS), [Some(321), Some(327)], 1));
    }

    #[test]
    fn a_strike_before_the_first_frame_keeps_none_and_is_not_re_derived_forever() {
        let mut early = stored(None, [None, None], 2);
        // Between the two strikes, 5.465 s and 5.575 s into the audio.
        early.clip.as_mut().unwrap().video_start_s = Some(5.5);
        let derived = frames_derived(&early, Some(FPS)).unwrap();
        assert_eq!(
            derived.strikes.iter().map(|s| s.frame).collect::<Vec<_>>(),
            [None, Some(4)]
        );
        assert_eq!(frames_derived(&derived, Some(FPS)), None);
    }

    #[test]
    fn no_such_role_is_nothing_to_detect_in() {
        let dir = tempfile::tempdir().unwrap();
        let mut log = Vec::new();
        let mut notes = Vec::new();
        let result = audio_for(
            dir.path(),
            &manifest(),
            Role::FaceOn,
            AudioOptions::default(),
            Err(&no_ffmpeg()),
            &mut |line: &str| log.push(line.to_string()),
            &mut notes,
        );
        assert_eq!(result.unwrap(), None);
        assert!(log.is_empty() && notes.is_empty());
    }

    #[test]
    fn a_current_cache_is_read_and_written_back_only_when_frames_are_added() {
        let dir = tempfile::tempdir().unwrap();
        let path = audio_path(dir.path(), Role::DownTheLine);
        save_audio(&stored(None, [None, None], 2), &path).unwrap();
        let options = AudioOptions {
            fps: Some(FPS),
            force: false,
        };

        // No clip on disk and no ffmpeg: a cache hit needs neither.
        let first = run(dir.path(), options, Err(&no_ffmpeg()));
        let expected = stored(Some(FPS), [Some(321), Some(327)], 2);
        assert_eq!(first.result.unwrap(), Some(expected.clone()));
        assert_eq!(
            first.log,
            [
                "  down_the_line: 2 strikes from cache".to_string(),
                "  down_the_line: frames derived at 59.9602911978822 fps -> \
                 down_the_line.audio.json"
                    .to_string(),
            ]
        );
        assert!(first.notes.is_empty());
        assert_eq!(load_audio(&path).unwrap(), expected, "written back");

        let modified = fs::metadata(&path).unwrap().modified().unwrap();
        let second = run(dir.path(), options, Err(&no_ffmpeg()));
        assert_eq!(second.result.unwrap(), Some(expected));
        assert_eq!(second.log, ["  down_the_line: 2 strikes from cache"]);
        assert_eq!(fs::metadata(&path).unwrap().modified().unwrap(), modified);
    }

    #[test]
    fn an_older_detection_is_kept_with_a_note_when_this_machine_cannot_listen() {
        let dir = tempfile::tempdir().unwrap();
        let path = audio_path(dir.path(), Role::DownTheLine);
        save_audio(&stored(None, [None, None], 1), &path).unwrap();
        // The clip is on disk; it is ffmpeg that is missing.
        fs::write(dir.path().join("down_the_line.b2a5e44ef3d7.MOV"), b"x").unwrap();
        let options = AudioOptions {
            fps: Some(FPS),
            force: false,
        };
        let done = run(dir.path(), options, Err(&no_ffmpeg()));
        assert_eq!(
            done.result.unwrap(),
            Some(stored(Some(FPS), [Some(321), Some(327)], 1)),
            "framed, and still v1"
        );
        let why = no_ffmpeg().to_string();
        assert_eq!(
            done.log,
            [
                "  down_the_line: cached audio is from detector v1, re-detecting at v2".to_string(),
                format!("  down_the_line: cannot re-detect ({why}), keeping the older detection"),
            ]
        );
        assert_eq!(
            done.notes,
            [format!(
                "the down_the_line strikes were found by an older detector (v1) and this machine \
                 cannot re-run it: {why}"
            )]
        );
        assert_eq!(
            load_audio(&path).unwrap(),
            stored(None, [None, None], 1),
            "an older detection kept is not re-saved"
        );
    }

    #[test]
    fn with_no_ffmpeg_and_nothing_older_the_view_has_no_audio() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("down_the_line.b2a5e44ef3d7.MOV"), b"x").unwrap();
        let done = run(dir.path(), AudioOptions::default(), Err(&no_ffmpeg()));
        assert_eq!(done.result.unwrap(), None);
        let why = no_ffmpeg().to_string();
        assert_eq!(
            done.log,
            [format!("  down_the_line: no strike detection — {why}")]
        );
        assert_eq!(done.notes, [format!("no audio for down_the_line: {why}")]);
    }

    #[test]
    fn a_cache_for_another_clip_is_re_detected_and_a_missing_clip_is_a_note() {
        let dir = tempfile::tempdir().unwrap();
        let mut other = stored(None, [None, None], 2);
        other.clip.as_mut().unwrap().source_sha256 = Some("0".repeat(64));
        save_audio(&other, &audio_path(dir.path(), Role::DownTheLine)).unwrap();
        let done = run(dir.path(), AudioOptions::default(), Err(&no_ffmpeg()));
        assert_eq!(done.result.unwrap(), None);
        assert_eq!(
            done.log,
            [
                "  down_the_line: cached audio is for a different clip, re-detecting",
                "  down_the_line: down_the_line.b2a5e44ef3d7.MOV is missing from the swing \
                 directory",
            ]
        );
        assert_eq!(
            done.notes,
            ["the down_the_line clip is recorded in the manifest but missing from disk"]
        );
    }

    #[test]
    fn force_skips_a_current_cache() {
        let dir = tempfile::tempdir().unwrap();
        save_audio(
            &stored(None, [None, None], 2),
            &audio_path(dir.path(), Role::DownTheLine),
        )
        .unwrap();
        let options = AudioOptions {
            fps: None,
            force: true,
        };
        let done = run(dir.path(), options, Err(&no_ffmpeg()));
        assert_eq!(
            done.result.unwrap(),
            None,
            "no clip on disk to re-detect from"
        );
        assert_eq!(
            done.notes,
            ["the down_the_line clip is recorded in the manifest but missing from disk"]
        );
    }

    #[test]
    fn a_decode_that_fails_is_a_note_and_not_an_error() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("down_the_line.b2a5e44ef3d7.MOV"), b"x").unwrap();
        let ffmpeg = Ffmpeg::at(dir.path().join("no-ffmpeg-here.exe"));
        let done = run(dir.path(), AudioOptions::default(), Ok(&ffmpeg));
        assert_eq!(done.result.unwrap(), None);
        assert_eq!(done.log.len(), 2, "{:?}", done.log);
        assert_eq!(
            done.log[0],
            "  down_the_line: decoding audio from shot-9-back.MOV ..."
        );
        assert!(
            done.log[1].starts_with("  down_the_line: audio could not be decoded — could not run "),
            "{}",
            done.log[1]
        );
        assert_eq!(
            done.notes,
            ["the down_the_line clip's audio could not be decoded, so no strike was detected in it"]
        );
    }

    #[test]
    fn a_broken_cache_stops_the_bundle() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(audio_path(dir.path(), Role::DownTheLine), b"{not json").unwrap();
        let done = run(dir.path(), AudioOptions::default(), Err(&no_ffmpeg()));
        assert!(
            matches!(done.result, Err(StoreError::Unreadable { .. })),
            "{:?}",
            done.result
        );
    }
}
