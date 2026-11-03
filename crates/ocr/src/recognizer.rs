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
//!    `f64`, the score as the confidence, and a reading with no text dropped;
//! 6. each kept box's glyph colour measured on the image it was found in ([`glyph_rgb`]), which
//!    no step of frozen Python's has (M42 P2): Full Swing lights one letter of a sign pair, and
//!    only the colour says which.
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
        .map(|mut found| {
            found.rgb = glyph_rgb(image, found.x, found.y, found.width, found.height);
            found
        })
        .collect())
}

/// `paddle.py`'s `_to_text_box`: the polygon's bounding rectangle, and the reading's score as the
/// confidence; `None` for a reading with no text, which the adapter drops. The adapter measures no
/// colour, so `rgb` is left `None` here and [`recognize`] fills it from the image, which
/// `_to_text_box`'s arguments do not include.
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
        rgb: None,
    })
}

/// A box's glyph colour, [`TextBox::rgb`]'s definition exactly (the M42 plan's call 4): the mean
/// red, green and blue of the brightest quarter of the box's pixels in `image`. No Python twin:
/// frozen Python's boxes carry no colour. [M42 P2]
///
/// The box is clipped to the image first (`x0 = floor(max(x, 0))`, `x1 = min(ceil(x + width),
/// W)`, likewise y), so a box the detector let run off an edge is measured on what is inside it,
/// and one wholly outside, or with no width or height, holds no pixel and is `None`. The pixels are
/// ranked by `r + g + b` ascending with a stable sort over row-major order (top row first, left to
/// right), so two pixels equally bright keep that order and the answer is fixed; the brightest
/// quarter is the ranks from `n * 3 / 4` (integer division) to the end, never empty once `n` is at
/// least one.
///
/// Why the brightest quarter and not the mean: a letter's strokes cover a fraction of its box and
/// the dark ground the rest, so a box's mean is mostly the ground's. On Full Swing's grid photos
/// the strokes filled at least a quarter of every sign letter's box, and the brightest quarter
/// measured a lit letter at blue ÷ min(red, green) 0.32–0.47 and an unlit one at 0.94–0.98 (the
/// M42 plan's finding 4). The sums are of integers below 2^53, so they are exact in `f64` and the
/// order they are added in cannot move a bit.
pub fn glyph_rgb(image: &RgbImage, x: f64, y: f64, width: f64, height: f64) -> Option<[f64; 3]> {
    let (x0, x1) = clipped(x, width, image.width())?;
    let (y0, y1) = clipped(y, height, image.height())?;
    let mut pixels: Vec<[u8; 3]> = (y0..y1)
        .flat_map(|row| (x0..x1).map(move |column| image.get_pixel(column, row).0))
        .collect();
    // `sort_by_key` is stable, which is the tie rule.
    pixels.sort_by_key(|[r, g, b]| u32::from(*r) + u32::from(*g) + u32::from(*b));
    let brightest = &pixels[pixels.len() * 3 / 4..];
    let mut sum = [0.0f64; 3];
    for pixel in brightest {
        for (total, channel) in sum.iter_mut().zip(pixel) {
            *total += f64::from(*channel);
        }
    }
    let n = brightest.len() as f64;
    Some(sum.map(|total| total / n))
}

/// One axis of a box clipped to an image `extent` pixels long, as `[start, end)` pixel indices:
/// `floor(max(start, 0))` to `min(ceil(start + length), extent)`, or `None` when that is empty.
fn clipped(start: f64, length: f64, extent: u32) -> Option<(u32, u32)> {
    let low = start.max(0.0).floor();
    let high = (start + length).ceil().min(f64::from(extent));
    if low >= high {
        return None;
    }
    // Both are whole numbers in `[0, extent]` here, so the casts are exact.
    Some((low as u32, high as u32))
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
                rgb: None,
            }
        );
        assert_eq!(super::text_box(&found, reading("")), None);
    }

    use image::Rgb;

    fn filled(width: u32, height: u32, colour: [u8; 3]) -> RgbImage {
        RgbImage::from_pixel(width, height, Rgb(colour))
    }

    /// A lit letter as Full Swing prints one: thin yellow strokes on a dark tile, a darker
    /// antialiased fringe, and white text just past the box's right edge. The strokes fill 252 of
    /// the box's 800 pixels, more than its brightest quarter, so the quarter is all stroke; the
    /// box's plain mean would be mostly the ground's.
    #[test]
    fn a_dark_box_holding_yellow_strokes_reads_yellow() {
        let yellow = [241, 223, 91];
        let mut image = filled(60, 30, [30, 30, 35]);
        for y in 6..24 {
            for x in 14..20 {
                image.put_pixel(x, y, Rgb(yellow));
            }
            image.put_pixel(20, y, Rgb([120, 110, 45]));
        }
        for y in 6..12 {
            for x in 20..44 {
                image.put_pixel(x, y, Rgb(yellow));
            }
        }
        // Column 50 is `ceil(10 + 40)`, the first one past the box.
        for y in 0..30 {
            image.put_pixel(50, y, Rgb([255, 255, 255]));
        }
        let [r, g, b] = glyph_rgb(&image, 10.0, 5.0, 40.0, 20.0).unwrap();
        for (got, want) in [r, g, b].iter().zip(yellow) {
            assert!((got - f64::from(want)).abs() <= 1.0, "{:?}", [r, g, b]);
        }
        // What the lit rule (call 4) will read it as: blue well under the smaller of red and green.
        assert!(b / r.min(g) < 0.47);
    }

    #[test]
    fn a_box_with_no_pixel_in_the_image_has_no_colour() {
        let image = filled(10, 10, [200, 200, 200]);
        for (x, y, width, height) in [
            (10.0, 0.0, 5.0, 5.0), // right of the image
            (-8.0, 0.0, 5.0, 5.0), // left of it, ending at -3
            (0.0, 12.0, 5.0, 5.0), // below it
            (0.0, -6.0, 5.0, 6.0), // above it, ending at row 0 exclusive
            (3.0, 3.0, 0.0, 4.0),  // no width
            (3.0, 3.0, 4.0, -1.0), // a negative height
        ] {
            assert_eq!(
                glyph_rgb(&image, x, y, width, height),
                None,
                "{:?}",
                (x, y, width, height)
            );
        }
        // One pixel is a quarter of itself.
        assert_eq!(glyph_rgb(&image, 9.0, 9.0, 1.0, 1.0), Some([200.0; 3]));
    }

    /// A box run off the top-left corner: `floor(max(-3.5, 0)) = 0` to `min(ceil(3.5), 10) = 4`
    /// across and `floor(6.5) = 6` to `min(ceil(16.5), 10) = 10` down, sixteen pixels graded 0 to
    /// 150 by 10. Everything outside them is brighter than all sixteen, so a clip one pixel wider
    /// in any direction would move the answer off the brightest four's mean, 135.
    #[test]
    fn a_box_straddling_the_edge_is_clipped_to_it() {
        let mut image = filled(10, 10, [250, 250, 250]);
        for y in 6..10 {
            for x in 0..4 {
                let level = u8::try_from(((y - 6) * 4 + x) * 10).unwrap();
                image.put_pixel(x, y, Rgb([level; 3]));
            }
        }
        assert_eq!(
            glyph_rgb(&image, -3.5, 6.5, 7.0, 10.0),
            Some([135.0, 135.0, 135.0])
        );
    }

    /// Eight pixels all summing to 300: the brightest quarter is ranks 6 and 7, and a stable sort
    /// over row-major order makes those the bottom row's last two, whose mean is pinned. Any other
    /// order of equals answers something else.
    #[test]
    fn equally_bright_pixels_keep_their_row_major_order() {
        let rows = [
            [[100, 100, 100], [200, 50, 50], [50, 200, 50], [50, 50, 200]],
            [[0, 150, 150], [150, 0, 150], [150, 150, 0], [30, 60, 210]],
        ];
        let mut image = filled(4, 2, [0, 0, 0]);
        for (y, row) in (0u32..).zip(rows) {
            for (x, pixel) in (0u32..).zip(row) {
                image.put_pixel(x, y, Rgb(pixel));
            }
        }
        assert_eq!(
            glyph_rgb(&image, 0.0, 0.0, 4.0, 2.0),
            Some([90.0, 105.0, 105.0])
        );
    }
}
