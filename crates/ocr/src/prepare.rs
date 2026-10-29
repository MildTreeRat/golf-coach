//! The reader end to end: a photo in, the boxes and notes `crates/screen` reads a shot from out.
//! `preprocess.py::prepare_screen`'s twin, and the pixel half of `importer.py::import_screen`.
//! [M29 P7]
//!
//! [`prepare_screen`] is `prepare_screen`: [`crate::rectify::working_image`] (the screen found,
//! warped front-on, shrunk and enhanced, or the photo read uncropped with a note saying so), then
//! the rotation vote, [`crate::recognize`] at each turn in [`Rotation::VOTE_ORDER`] until one
//! carries [`GOOD_ENOUGH_LABEL_RATIO`] of the profile's labels, and the notes worded as frozen
//! Python words them. [`read_photo`] puts [`crate::decode::load_image`] in front of it, and is the
//! crate's entry: what `golf-ocr read`, and from P23 the lab's shot re-read and `golf-lab
//! import-photos`, call.
//!
//! It stops where `import_screen` hands over to the parser, at a [`PreparedScreen`], whose `boxes`
//! and `notes` are a [`screen::ScreenInput`]'s. The parse, the checks and the record are
//! [`screen::read`]'s, so a shot read from a photo here and one read from the phone's VisionKit
//! boxes go through the one reader. [`screen_timestamp`] is the one other thing `import_screen`
//! takes from the photo itself, its time.
//!
//! # The vote is written against a recognizer, not the models
//!
//! [`vote`] takes the recognizer as a closure, as frozen `prepare_screen` takes a
//! `TextRecognizer`, so its rules (the early stop, the first best kept on a tie, the notes) are
//! unit-tested here with no model loaded. [`prepare_screen`] passes it [`crate::recognize`].
//!
//! # How close it lands (M29 P7, the 13 bay photos)
//!
//! `tests/read.rs` is D7's gate: each photo read here, its shot parsed by [`screen::read`], and
//! compared with the shot the committed screen vector parsed from frozen PaddleOCR's boxes. Every
//! photo's shot agrees on D7's terms but one value, which the gate names: 2026-08-10-1's
//! `impact_position`, read here as the photo shows it and withheld by the vector's misread label
//! (`src/recognizer.rs`'s doc). The M29 plan's P7 finding has the rest.

use std::fmt;
use std::fs;
use std::io;
use std::path::Path;
use std::time::{Duration, UNIX_EPOCH};

use contracts::Timestamp;
use image::RgbImage;
use screen::orient::label_ratio;
use screen::profile::DeviceProfile;
use screen::TextBox;

use crate::decode::{load_image, DecodeError};
use crate::models::{ModelError, Models};
use crate::quad::ScreenQuad;
use crate::recognizer::recognize;
use crate::rectify::{rotate, working_image, Rotation, Working};

/// The vote stops at the first turn carrying this fraction of the profile's stored labels
/// (`_GOOD_ENOUGH_LABEL_RATIO`): no orientation but the right one gets close to it. A best turn
/// below it is read anyway, with a note that the parse is unreliable.
pub const GOOD_ENOUGH_LABEL_RATIO: f64 = 0.6;

/// `prepare_screen`'s note when no screen outline was found, verbatim: it goes first in the
/// shot's warnings, because it explains the rest.
pub const UNCROPPED_NOTE: &str = "screen outline not found - parsing the photo uncropped";

/// A rectified, upright screen and the text found on it: `PreparedScreen`.
#[derive(Clone, Debug)]
pub struct PreparedScreen {
    /// The working image at the vote's turn, the one `boxes` are in the pixels of.
    pub image: RgbImage,
    pub boxes: Vec<TextBox>,
    pub rotation: Rotation,
    /// The screen's outline the photo was cut by, or `None` where it was read uncropped. Python
    /// keeps only `rectified`, which is [`PreparedScreen::rectified`]; the quad says which of
    /// `find_screen_quad`'s two answers it was.
    pub quad: Option<ScreenQuad>,
    /// The fraction of the profile's stored labels `boxes` carries, the vote's score.
    pub label_ratio: f64,
    /// What preprocessing noticed, in `prepare_screen`'s order and words.
    pub notes: Vec<String>,
}

impl PreparedScreen {
    /// `PreparedScreen.rectified`.
    pub fn rectified(&self) -> bool {
        self.quad.is_some()
    }
}

/// What the vote chose: the turn, its boxes and their score, and the notes it adds.
#[derive(Clone, Debug, PartialEq)]
pub struct Vote {
    pub rotation: Rotation,
    pub boxes: Vec<TextBox>,
    pub label_ratio: f64,
    pub notes: Vec<String>,
}

/// `prepare_screen`'s rotation vote over `working`, with `recognize` as its recognizer.
///
/// Each turn in [`Rotation::VOTE_ORDER`] is recognized and scored by `label_ratio`, best guess
/// first, until one reaches [`GOOD_ENOUGH_LABEL_RATIO`]. The best is kept with a strict `>`, so on
/// a tie the earlier turn wins, and the starting best of `-1.0` is beaten by any first score. The
/// notes are Python's: the turn, when it is not upright, then the legibility warning, when even the
/// best is below the bar, with the percentage as `{ratio:.0%}` prints it.
pub fn vote<E>(
    working: &RgbImage,
    profile: &DeviceProfile,
    mut recognize: impl FnMut(&RgbImage) -> Result<Vec<TextBox>, E>,
) -> Result<Vote, E> {
    let mut best: (f64, Rotation, Vec<TextBox>) = (-1.0, Rotation::Upright, Vec::new());
    for rotation in Rotation::VOTE_ORDER {
        let boxes = recognize(&rotate(working, rotation))?;
        let ratio = label_ratio(&boxes, profile);
        let good_enough = ratio >= GOOD_ENOUGH_LABEL_RATIO;
        if ratio > best.0 {
            best = (ratio, rotation, boxes);
        }
        if good_enough {
            break;
        }
    }
    let (ratio, rotation, boxes) = best;
    let mut notes = Vec::new();
    if rotation != Rotation::Upright {
        notes.push(format!(
            "image was rotated {} degrees - corrected",
            rotation.degrees()
        ));
    }
    if ratio < GOOD_ENOUGH_LABEL_RATIO {
        notes.push(format!(
            "only {} of the {} labels were legible at the best orientation - the parse below is \
             unreliable",
            pyfmt::percent(ratio, 0),
            profile.device
        ));
    }
    Ok(Vote {
        rotation,
        boxes,
        label_ratio: ratio,
        notes,
    })
}

/// `prepare_screen`: rectify, orient and recognize one decoded photo of a launch-monitor screen.
pub fn prepare_screen(
    models: &mut Models,
    photo: &RgbImage,
    profile: &DeviceProfile,
) -> Result<PreparedScreen, ModelError> {
    let Working {
        image: working,
        quad,
    } = working_image(photo);
    let mut notes = Vec::new();
    if quad.is_none() {
        notes.push(UNCROPPED_NOTE.to_string());
    }
    let chosen = vote(&working, profile, |image| recognize(models, image))?;
    notes.extend(chosen.notes);
    Ok(PreparedScreen {
        image: rotate(&working, chosen.rotation),
        boxes: chosen.boxes,
        rotation: chosen.rotation,
        quad,
        label_ratio: chosen.label_ratio,
        notes,
    })
}

/// Why a photo could not be read. Each passes its cause's message through unchanged, so a
/// progressive JPEG's refusal says to re-save it (the M29 plan's P2 finding 19) and a missing
/// model says how the models are made, with nothing wrapped around either.
#[derive(Debug)]
pub enum ReadError {
    Decode(DecodeError),
    Model(ModelError),
}

impl fmt::Display for ReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReadError::Decode(error) => error.fmt(f),
            ReadError::Model(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for ReadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ReadError::Decode(error) => Some(error),
            ReadError::Model(error) => Some(error),
        }
    }
}

impl From<DecodeError> for ReadError {
    fn from(error: DecodeError) -> Self {
        ReadError::Decode(error)
    }
}

impl From<ModelError> for ReadError {
    fn from(error: ModelError) -> Self {
        ReadError::Model(error)
    }
}

/// The crate's entry: one photo of a launch-monitor screen, decoded, rectified, oriented and
/// recognized, ready for [`screen::read`]. `import_screen`'s `prepare_screen(load_image(path), …)`.
pub fn read_photo(
    models: &mut Models,
    path: &Path,
    profile: &DeviceProfile,
) -> Result<PreparedScreen, ReadError> {
    let photo = load_image(path)?;
    Ok(prepare_screen(models, &photo, profile)?)
}

/// When the photo was taken, as frozen Python stamps a shot read from it:
/// `importer.py::screen_timestamp`, `datetime.fromtimestamp(path.stat().st_mtime, tz=UTC)`.
///
/// **A float's microseconds, not the file time's.** CPython makes `st_mtime` as `sec + nsec *
/// 1e-9` in a double, and `fromtimestamp` rounds that double's fraction to the microsecond half
/// to even, carrying into the second. At today's 1.7e9 seconds a double is about 0.24 us coarse,
/// so this is not a truncation of the file's 100 ns time: `.828488500` stamps `.828489`, and
/// `.999999900` the next whole second. [`micros_since_epoch`] is that arithmetic, held to
/// CPython 3.13's answers by its unit test. An mtime before 1970 is refused rather than signed.
pub fn screen_timestamp(path: &Path) -> io::Result<Timestamp> {
    let since = fs::metadata(path)?
        .modified()?
        .duration_since(UNIX_EPOCH)
        .map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{} was modified before 1970", path.display()),
            )
        })?;
    Ok(Timestamp::from_utc_micros(micros_since_epoch(since)))
}

/// `fromtimestamp`'s microseconds for a file time `since` the epoch (`_PyTime_DoubleToDenominator`
/// at `_PyTime_ROUND_HALF_EVEN`).
fn micros_since_epoch(since: Duration) -> i64 {
    let seconds = since.as_secs() as f64 + f64::from(since.subsec_nanos()) * 1e-9;
    let mut whole = seconds.trunc();
    let mut micros = pyfmt::round_half_even((seconds - whole) * 1e6);
    if micros >= 1e6 {
        micros -= 1e6;
        whole += 1.0;
    }
    whole as i64 * 1_000_000 + micros as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use screen::profile::load_profile;
    use std::cell::Cell;

    fn boxes(texts: &[&str]) -> Vec<TextBox> {
        texts
            .iter()
            .map(|text| TextBox {
                text: (*text).to_string(),
                x: 0.0,
                y: 0.0,
                width: 1.0,
                height: 1.0,
                confidence: 1.0,
            })
            .collect()
    }

    /// The first `n` of the profile's stored labels, as one box each.
    fn labels(profile: &DeviceProfile, n: usize) -> Vec<TextBox> {
        let stored: Vec<&str> = profile
            .stored_fields()
            .iter()
            .map(|field| field.label.as_str())
            .collect();
        boxes(&stored[..n])
    }

    /// A recognizer that answers the `i`-th turn it is shown with `answers[i]`, counting in `calls`.
    fn scripted<'a>(
        answers: &'a [Vec<TextBox>],
        calls: &'a Cell<usize>,
    ) -> impl FnMut(&RgbImage) -> Result<Vec<TextBox>, ()> + 'a {
        move |_| {
            let i = calls.get();
            calls.set(i + 1);
            Ok(answers[i].clone())
        }
    }

    #[test]
    fn an_upright_screen_is_read_once_and_noted_nowhere() {
        let profile = load_profile("hd_golf").unwrap();
        let stored = profile.stored_fields().len();
        let (answers, calls) = (vec![labels(profile, stored)], Cell::new(0));
        let chosen = vote(&RgbImage::new(4, 2), profile, scripted(&answers, &calls)).unwrap();
        assert_eq!(calls.get(), 1);
        assert_eq!(chosen.rotation, Rotation::Upright);
        assert_eq!(chosen.label_ratio, 1.0);
        assert!(chosen.notes.is_empty());
    }

    /// The vote stops at the first turn good enough, not the best, and notes the turn.
    #[test]
    fn the_vote_stops_at_the_first_turn_good_enough_and_notes_it() {
        let profile = load_profile("hd_golf").unwrap();
        let stored = profile.stored_fields().len();
        let enough = (stored as f64 * GOOD_ENOUGH_LABEL_RATIO).ceil() as usize;
        let calls = Cell::new(0);
        let answers = vec![
            labels(profile, 1),
            labels(profile, 2),
            labels(profile, enough),
            labels(profile, stored),
        ];
        let chosen = vote(&RgbImage::new(4, 2), profile, scripted(&answers, &calls)).unwrap();
        assert_eq!(calls.get(), 3, "0, 180 and 90 tried, 270 not");
        assert_eq!(chosen.rotation, Rotation::Clockwise);
        assert_eq!(chosen.boxes, labels(profile, enough));
        assert_eq!(
            chosen.notes,
            vec!["image was rotated 90 degrees - corrected"]
        );
    }

    /// Below the bar everywhere: all four tried, the first of the equal best kept, and both notes,
    /// the percentage as Python's `.0%` prints it (3 of hd_golf's 15 is `20%`).
    #[test]
    fn a_screen_illegible_at_every_turn_keeps_the_first_best_and_says_so() {
        let profile = load_profile("hd_golf").unwrap();
        let stored = profile.stored_fields().len();
        let calls = Cell::new(0);
        let answers = vec![
            labels(profile, 1),
            labels(profile, 3),
            labels(profile, 2),
            labels(profile, 3),
        ];
        let chosen = vote(&RgbImage::new(4, 2), profile, scripted(&answers, &calls)).unwrap();
        assert_eq!(calls.get(), 4);
        assert_eq!(
            chosen.rotation,
            Rotation::Half,
            "the tie goes to the earlier turn"
        );
        assert_eq!(chosen.label_ratio, 3.0 / stored as f64);
        assert_eq!(
            chosen.notes,
            vec![
                "image was rotated 180 degrees - corrected".to_string(),
                format!(
                    "only {} of the hd_golf labels were legible at the best orientation - the \
                     parse below is unreliable",
                    pyfmt::percent(3.0 / stored as f64, 0)
                ),
            ]
        );
        assert_eq!(stored, 15, "the note's percentage below is worked for 15");
        assert_eq!(pyfmt::percent(3.0 / 15.0, 0), "20%");
    }

    /// Nothing legible at all is still a vote: upright, a ratio of 0, and the warning.
    #[test]
    fn a_screen_with_no_text_is_read_upright_at_zero() {
        let profile = load_profile("hd_golf").unwrap();
        let (answers, calls) = (vec![Vec::new(); 4], Cell::new(0));
        let chosen = vote(&RgbImage::new(4, 2), profile, scripted(&answers, &calls)).unwrap();
        assert_eq!(calls.get(), 4);
        assert_eq!(chosen.rotation, Rotation::Upright);
        assert_eq!(chosen.label_ratio, 0.0);
        assert_eq!(
            chosen.notes,
            vec![
                "only 0% of the hd_golf labels were legible at the best orientation - the parse \
                 below is unreliable"
            ]
        );
    }

    #[test]
    fn a_recognizer_failure_stops_the_vote() {
        let profile = load_profile("hd_golf").unwrap();
        let failed = vote(&RgbImage::new(4, 2), profile, |_| {
            Err::<Vec<TextBox>, _>("no model")
        });
        assert_eq!(failed, Err("no model"));
    }

    /// CPython 3.13's `datetime.fromtimestamp(sec + nsec * 1e-9, tz=UTC)` on each, recorded in M29
    /// P7: the float's rounding, not the file time's truncation (`.828488500` stamps `.828489`),
    /// a half microsecond the double cannot hold (`500` ns stamps nothing), and a fraction that
    /// rounds into the next second.
    #[test]
    fn a_file_time_stamps_as_cpython_stamps_it() {
        let cases = [
            (1_786_000_000, 0, "2026-08-06T07:06:40Z"),
            (1_786_000_000, 828_488_300, "2026-08-06T07:06:40.828488Z"),
            (1_786_000_000, 999_999_900, "2026-08-06T07:06:41Z"),
            (1_786_000_000, 500, "2026-08-06T07:06:40Z"),
            (1_786_000_000, 1_500, "2026-08-06T07:06:40.000001Z"),
            (1_754_789_926, 828_488_500, "2025-08-10T01:38:46.828489Z"),
            (1_754_789_926, 123_456_700, "2025-08-10T01:38:46.123457Z"),
        ];
        for (seconds, nanos, expected) in cases {
            let micros = micros_since_epoch(Duration::new(seconds, nanos));
            assert_eq!(
                Timestamp::from_utc_micros(micros).to_string(),
                expected,
                "{seconds}.{nanos:09}"
            );
        }
    }
}
