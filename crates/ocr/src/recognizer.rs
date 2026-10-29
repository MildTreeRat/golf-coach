//! The `recognize` seam: a working image in, the located text the screen parser reads out. The
//! `TextRecognizer` port's Rust twin. [M29 P6]
//!
//! Frozen Python reaches its parser through a port, `recognizer.py`'s `TextRecognizer`, whose one
//! method turns an image into `TextBox`es, and `paddle.py`'s `PaddleOCRRecognizer` is the adapter
//! behind it. [`recognize`] is that adapter's twin, built from this crate's stages in the order
//! PaddleX's OCR pipeline runs them (`pipelines/ocr/pipeline.py`), and it returns
//! `crates/screen`'s [`TextBox`], so what comes out goes straight into `screen::read` as frozen
//! Python's boxes go into `parse_screen`:
//!
//! 1. [`crate::det::detect`], then [`crate::det::sort_quad_boxes`], the reading order;
//! 2. [`crate::rec::crop`] on each box;
//! 3. [`crate::cls::classify`] on the crops in that order, and [`crate::cls::turn_half`] on each
//!    one it reads as upside down;
//! 4. [`crate::rec::recognize`], which sorts and batches the crops itself;
//! 5. a reading kept when its score is at least the manifest's `score_thresh`, and its box turned
//!    into a [`TextBox`] as `paddle.py`'s `_to_text_box` does: the polygon's bounding rectangle in
//!    `f64`, the score as the confidence, and a reading with no text dropped.
//!
//! # What it does not do: D5's two skipped models
//!
//! Frozen `paddle.py` sets only `use_textline_orientation` and `enable_mkldnn`, so PaddleOCR's
//! document preprocessor runs too: a document-orientation classifier and UVDoc's unwarping, ahead
//! of detection. The M29 plan's D5 leaves both out, because the screen is already turned upright by
//! the rotation vote and warped front-on by [`crate::rectify`]. So frozen `recognize` detects on
//! UVDoc's image and this detects on the working image itself, and their boxes are not the same
//! boxes.
//!
//! # How close it lands (M29 P6, the 13 bay photos, pinned by `tests/recognizer.rs`)
//!
//! - **Against frozen PaddleOCR with D5's three models** (`paddle.py`'s construction with the
//!   document preprocessor off), the reader this ports: on the twelve photos that are not HEIC,
//!   the same boxes in the same order and **every text identical**, one box 2 px off (the
//!   detector's, P4) and every confidence within 2.3e-5. On the HEIC, every text identical. On all
//!   13 the two readings parse identically.
//! - **Against frozen `recognize` with all five**, the boxes the screen vectors hold: UVDoc moves
//!   text by up to 170 px and splits it into boxes differently, so this finds up to 8 boxes more
//!   on a photo (mostly the menu bar). The parser locates the same tiles on all 13 and reads the
//!   same values on 12. On 2026-08-10-1 the five-model reading loses the `V` of `Impact Position
//!   V`, so frozen Python's parse sees two labels tie and reads neither tile, and this reads the
//!   `V`, so the parse reads `Impact Position` as `CENTER`, which is what the photo shows. On
//!   seven more photos the text under a tile differs by a degree sign or a unit, and no value
//!   moves. The user chose (2026-10-21, the M29 plan's P7) to keep this reader as it is and name
//!   that one value in the OCR gate (`tests/read.rs`), rather than add UVDoc to reproduce the
//!   misread.
//!
//! # Two of the pipeline's steps that cannot fire, mirrored by argument
//!
//! The pipeline drops a box whose crop is empty, and [`crate::rec::crop`] never returns one (a side
//! that truncates to zero takes the source's size, as `warpPerspective` does). It keeps a reading
//! when `rec_score >= text_rec_score_thresh`, and with the manifest's 0.0 that keeps every reading,
//! because a score is a mean of probabilities, or exactly 0. Both are written out anyway, so a
//! manifest that raised the threshold would be honoured rather than ignored.
//!
//! **One place this is not `paddle.py` to the letter.** `_from_dict_result` pairs the `i`-th kept
//! text with the `i`-th *detected* polygon (`dt_polys`), not the `i`-th kept one (`rec_polys`).
//! The two are the same list whenever the threshold drops nothing, which at 0.0 it cannot; a
//! threshold that dropped a reading would shift every later text onto its neighbour's box. Here a
//! reading keeps its own box, which is frozen Python's answer at every threshold it can be given.

use image::RgbImage;
use screen::TextBox;

use crate::cls::{classify, turn_half};
use crate::det::{detect, sort_quad_boxes, DetBox};
use crate::models::{Manifest, ModelError, Models};
use crate::rec::{self, crop, Recognized};

/// Every run of text found on `image`, the working image frozen `prepare_screen` hands its
/// recognizer (rectified, shrunk to 1600 px, enhanced and turned by the vote), in the pipeline's
/// reading order, in `image`'s pixels.
pub fn recognize(models: &mut Models, image: &RgbImage) -> Result<Vec<TextBox>, ModelError> {
    let boxes = sort_quad_boxes(&detect(models, image)?);
    if boxes.is_empty() {
        // The pipeline skips an image with no boxes before it crops anything.
        return Ok(Vec::new());
    }
    let mut crops: Vec<RgbImage> = boxes.iter().map(|b| crop(image, &b.points)).collect();
    let orientations = classify(models, &crops)?;
    for (cut, orientation) in crops.iter_mut().zip(orientations) {
        if orientation.upside_down() {
            *cut = turn_half(cut);
        }
    }
    let readings = rec::recognize(models, &crops)?;
    let threshold = Manifest::committed().models.rec.score_thresh;
    Ok(boxes
        .iter()
        .zip(readings)
        .filter(|(_, reading)| reading.score >= threshold)
        .filter_map(|(found, reading)| text_box(found, reading))
        .collect())
}

/// `paddle.py`'s `_to_text_box`: the polygon's bounding rectangle, and the reading's score as the
/// confidence; `None` for a reading with no text, which the adapter drops.
pub fn text_box(found: &DetBox, reading: Recognized) -> Option<TextBox> {
    if reading.text.is_empty() {
        return None;
    }
    let xs = found.points.map(|[x, _]| f64::from(x));
    let ys = found.points.map(|[_, y]| f64::from(y));
    let (left, right) = (min(&xs), max(&xs));
    let (top, bottom) = (min(&ys), max(&ys));
    Some(TextBox {
        text: reading.text,
        x: left,
        y: top,
        width: right - left,
        height: bottom - top,
        confidence: reading.score,
    })
}

fn min(values: &[f64; 4]) -> f64 {
    values.iter().copied().fold(f64::INFINITY, f64::min)
}

fn max(values: &[f64; 4]) -> f64 {
    values.iter().copied().fold(f64::NEG_INFINITY, f64::max)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_box_is_its_polygons_bounding_rectangle_and_an_empty_reading_is_dropped() {
        let found = DetBox {
            points: [[12, 40], [90, 38], [91, 60], [11, 62]],
            score: 0.9,
        };
        let reading = |text: &str| Recognized {
            text: text.into(),
            score: 0.75,
        };
        let text_box = text_box(&found, reading("Carry")).unwrap();
        assert_eq!(
            text_box,
            TextBox {
                text: "Carry".into(),
                x: 11.0,
                y: 38.0,
                width: 80.0,
                height: 24.0,
                confidence: 0.75,
            }
        );
        assert_eq!(super::text_box(&found, reading("")), None);
    }
}
