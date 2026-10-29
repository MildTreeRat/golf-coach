//! What the text says: each detected box cut out front-on, read by the recognition model, and
//! the model's output decoded to a string and a score. [M29 P5]
//!
//! The twin of three pieces of PaddleX 3.7.2 as frozen PaddleOCR runs them:
//!
//! 1. [`crop`]: `CropByPolys(det_box_type="quad")` (`pipelines/components/common/
//!    crop_image_regions.py`), the box's `minAreaRect` with its corners in `get_mini_boxes`'
//!    order, a bicubic perspective warp with a replicated border
//!    ([`crate::cv::warp::warp_perspective_cubic`]) to the box's longer edges, and a quarter turn
//!    anticlockwise (`np.rot90`) when the crop is at least [`TALL`] times taller than wide;
//! 2. [`rec_input`]: the text-recognition predictor's `OCRReisizeNormImg` and `ToBatch`
//!    (`models/text_recognition/processors.py`): each crop resized (OpenCV's bilinear) to the
//!    model's height and its own width ratio, scaled to -1..1 **in BGR order**, because
//!    `ReadImage(format="BGR")` passes frozen Python's BGR array through untouched, as it does for
//!    the detector (M29 P4), and zero-padded on the right to the widest crop in its batch;
//! 3. [`ctc_decode`]: `CTCLabelDecode`, greedy: the most likely class at each step, repeats
//!    collapsed, the blank dropped, and the score the mean of the kept steps' probabilities, as
//!    NumPy sums a `float32` mean.
//!
//! [`recognize`] runs the three as the OCR pipeline does (`pipelines/ocr/pipeline.py`): the crops
//! sorted by width over height, stably, and read in batches of the manifest's `batch_size`, so
//! **a crop's batch-mates are its neighbours in that order** and its padding is theirs. That is
//! mirrored rather than simplified (the M29 plan's P1 finding 11 left it to this phase): the model
//! reads the padded columns too, and one step in eight of its output is theirs.
//!
//! What comes after recognition in the pipeline (the score threshold, and the boxes kept beside
//! their texts) is [`crate::recognizer::recognize`], P6's seam, which also turns crops that read
//! upside down ([`crate::cls`]) before they reach [`recognize`].
//!
//! # How close it lands (M29 P5, the 13 bay photos)
//!
//! Fed frozen Python's boxes (`tests/fixtures/det_boxes.json`, in reading order) on this crate's
//! working image of each photo, against PaddleX's crops and rec model on frozen Python's
//! (`tests/fixtures/rec_texts.json`), and pinned by `tests/rec.rs`:
//!
//! - on the twelve photos that are not HEIC, **every crop is PaddleX's pixel for pixel and every
//!   batch's input tensor bit for bit**. One box of 2026-08-23-10 needed `minAreaRect`'s caliper
//!   chosen as OpenCV 4.10 chooses it ([`crate::cv::rect`]), without which its corners were an
//!   ulp off and its crop a rounding off;
//! - fed frozen Paddle's own steps, the decode gives **its texts and scores to the bit**;
//! - through `ort`, **every one of the 607 texts is Paddle's**, and on the twelve every score is
//!   within 2.3e-5 of Paddle's: the runtimes' probabilities differ by up to 3.4e-4 a step, and no
//!   step's most likely class moves;
//! - on the HEIC, whose working image comes from ffmpeg's decode (the plan's P3 finding), no crop
//!   is PaddleX's but every text still is, and its scores move by up to 0.026.

use image::{imageops, RgbImage};

use crate::cv::contours::Point;
use crate::cv::resize::resize_linear;
use crate::cv::warp::{get_perspective_transform, invert3, warp_perspective_cubic};
use crate::det::mini_box;
use crate::models::{Manifest, Model, ModelError, Models, Rec, Tensor};

/// `get_rotate_crop_image`'s turn: a crop whose height over width is at least this is read
/// sideways, so it is turned a quarter anticlockwise first.
pub const TALL: f64 = 1.5;

/// `OCRReisizeNormImg.max_imgW`: no crop is resized wider than this, however long its text.
pub const MAX_WIDTH: usize = 3200;

/// One crop's reading: the text and `CTCLabelDecode`'s score, a `float32` mean widened as
/// Python's `float` widens it, or exactly 0 when no step survived the decode.
#[derive(Clone, Debug, PartialEq)]
pub struct Recognized {
    pub text: String,
    pub score: f64,
}

/// `CropByPolys.get_minarea_rect_crop`: the box `points` (four corners in `image`'s pixels) cut
/// out and warped front-on. Its size is the rotated rectangle's longer pair of edges each way,
/// measured as NumPy measures a `float32` difference and truncated, as
/// [`crate::rectify::warp_to_rect`] measures the screen.
///
/// A side that truncates to zero is OpenCV's `dsize.empty()`, where `warpPerspective` makes an
/// image the size of the source instead, through a transform that has collapsed to zero; that is
/// mirrored rather than refused, so this returns what frozen Python would recognise. The detector
/// cannot produce such a box: it drops any whose shorter side is under 5 px after the unclip.
pub fn crop(image: &RgbImage, points: &[[i32; 2]; 4]) -> RgbImage {
    let contour = points.map(|[x, y]| Point { x, y });
    let (corners, _) = mini_box(&contour);
    let edge = |a: [f32; 2], b: [f32; 2]| {
        let (dx, dy) = (a[0] - b[0], a[1] - b[1]);
        (dx * dx + dy * dy).sqrt()
    };
    // Python's `max(a, b)` keeps `a` unless `b` is larger.
    let longer = |a: f32, b: f32| if b > a { b } else { a };
    let [a, b, c, d] = corners;
    let width = longer(edge(a, b), edge(c, d)) as usize;
    let height = longer(edge(a, d), edge(b, c)) as usize;
    let (w, h) = (width as f32, height as f32);
    let target = [[0.0, 0.0], [w, 0.0], [w, h], [0.0, h]];
    let inverse = invert3(&get_perspective_transform(&corners, &target));
    let (dst_width, dst_height) = if width == 0 || height == 0 {
        (image.width() as usize, image.height() as usize)
    } else {
        (width, height)
    };
    let pixels = warp_perspective_cubic(
        image.as_raw(),
        image.width() as usize,
        image.height() as usize,
        &inverse,
        dst_width,
        dst_height,
    );
    let cropped = RgbImage::from_raw(dst_width as u32, dst_height as u32, pixels)
        .expect("a buffer of width * height * 3 bytes");
    if dst_height as f64 / dst_width as f64 >= TALL {
        // `np.rot90`: a quarter anticlockwise, the right-hand column becoming the top row.
        imageops::rotate270(&cropped)
    } else {
        cropped
    }
}

/// `OCRReisizeNormImg.resize_norm_img`'s two widths for a `width`x`height` crop: the width it is
/// resized to and the width its own tensor is padded to before batching. The ratio is the crop's
/// own or the model's (320 / 48), whichever is wider; a crop narrower than the model's ratio is
/// resized to the ceiling of its own width at the model's height and padded to 320, and a wider
/// one is resized to the truncation and not padded, up to [`MAX_WIDTH`].
pub fn rec_widths(width: usize, height: usize, rec: &Rec) -> (usize, usize) {
    let [_, img_h, img_w] = rec.image_shape.map(|v| v as usize);
    let max_wh_ratio = (img_w as f64 / img_h as f64).max(width as f64 / height as f64);
    let padded = (img_h as f64 * max_wh_ratio) as usize;
    if padded > MAX_WIDTH {
        return (MAX_WIDTH, MAX_WIDTH);
    }
    let ratio = width as f64 / height as f64;
    let ceiling = (img_h as f64 * ratio).ceil();
    let resized = if ceiling > padded as f64 {
        padded
    } else {
        ceiling as usize
    };
    (resized, padded)
}

/// The recognition model's input for one batch, `[n, 3, 48, W]`, `W` the widest of the crops'
/// padded widths ([`rec_widths`]). Each crop is resized as 8-bit (OpenCV's bilinear), then each
/// value is `(f32(pixel) / 255 - 0.5) / 0.5`, the three steps rounded separately as NumPy's
/// `float32` division and in-place `-=` and `/=` round them, with channel 0 blue; the rest of the
/// row is zero.
pub fn rec_input(crops: &[&RgbImage], rec: &Rec) -> Tensor {
    let [channels, img_h, _] = rec.image_shape.map(|v| v as usize);
    if channels != 3 || rec.img_mode != "BGR" {
        // `tests/rec.rs` reads the committed manifest, so another value is a bug in it.
        panic!(
            "crates/ocr/models.json: rec image_shape {:?} and img_mode {:?} are not the 3-channel BGR PaddleX reads",
            rec.image_shape, rec.img_mode
        );
    }
    assert!(!crops.is_empty(), "a batch holds at least one crop");
    let sized: Vec<(usize, usize)> = crops
        .iter()
        .map(|crop| {
            // `validate_text_rec_image_array`: PaddleX raises on an empty crop, and the pipeline
            // has already dropped any.
            assert!(crop.width() > 0 && crop.height() > 0, "an empty crop");
            rec_widths(crop.width() as usize, crop.height() as usize, rec)
        })
        .collect();
    let batch_width = sized.iter().map(|&(_, padded)| padded).max().unwrap_or(0);
    let plane = img_h * batch_width;
    let mut data = vec![0f32; crops.len() * 3 * plane];
    for (n, (crop, &(resized, _))) in crops.iter().zip(&sized).enumerate() {
        let pixels = resize_linear(
            crop.as_raw(),
            crop.width() as usize,
            crop.height() as usize,
            3,
            resized,
            img_h,
        );
        let base = n * 3 * plane;
        for y in 0..img_h {
            for x in 0..resized {
                let pixel = &pixels[(y * resized + x) * 3..(y * resized + x) * 3 + 3];
                for c in 0..3 {
                    // RGB in, BGR out: tensor channel 0 is blue.
                    let value = f32::from(pixel[2 - c]) / 255.0;
                    data[base + c * plane + y * batch_width + x] = (value - 0.5) / 0.5;
                }
            }
        }
    }
    Tensor {
        shape: vec![crops.len(), 3, img_h, batch_width],
        data,
    }
}

/// NumPy's `add.reduce` on a contiguous `float32` array (`FLOAT_pairwise_sum`): under 8 values a
/// running sum from zero; up to 128, eight running sums combined pairwise and the remainder added
/// one by one; beyond that the two halves (the first rounded down to a multiple of 8) summed so
/// and added. Checked against `np.mean` on 20,000 random arrays of 1 to 400 values (M29 P5).
fn pairwise_sum(values: &[f32]) -> f32 {
    let n = values.len();
    if n < 8 {
        values.iter().fold(0.0, |sum, &v| sum + v)
    } else if n <= 128 {
        let mut r: [f32; 8] = std::array::from_fn(|j| values[j]);
        let whole = n - n % 8;
        for block in values[8..whole].chunks_exact(8) {
            for (sum, &v) in r.iter_mut().zip(block) {
                *sum += v;
            }
        }
        let mut sum = ((r[0] + r[1]) + (r[2] + r[3])) + ((r[4] + r[5]) + (r[6] + r[7]));
        for &v in &values[whole..] {
            sum += v;
        }
        sum
    } else {
        let half = n / 2;
        let half = half - half % 8;
        pairwise_sum(&values[..half]) + pairwise_sum(&values[half..])
    }
}

/// `np.mean` of a `float32` array: the `float32` sum divided by the count in `f64`, then rounded
/// back to `float32` (`_mean`'s `ret.dtype.type(ret / rcount)`).
fn mean_f32(values: &[f32]) -> f32 {
    (f64::from(pairwise_sum(values)) / values.len() as f64) as f32
}

/// `BaseRecLabelDecode.decode` with `is_remove_duplicate=True` on one crop's steps: `indices` the
/// most likely class at each step and `probabilities` its probability. A step is kept when its
/// class differs from the step before's and is not the blank (class 0); the text is the kept
/// classes' characters (`characters` is the dictionary from class 1, and the class after it is
/// the space `use_space_char` adds), and the score their probabilities' mean, or 0 when none was
/// kept (`conf_list = [0]`).
pub fn decode_steps(indices: &[usize], probabilities: &[f32], characters: &[String]) -> Recognized {
    assert_eq!(indices.len(), probabilities.len());
    let space = characters.len() + 1;
    let mut text = String::new();
    let mut kept = Vec::new();
    for (t, (&class, &probability)) in indices.iter().zip(probabilities).enumerate() {
        if class == 0 || (t > 0 && class == indices[t - 1]) {
            continue;
        }
        if class == space {
            text.push(' ');
        } else {
            text.push_str(&characters[class - 1]);
        }
        kept.push(probability);
    }
    let score = if kept.is_empty() {
        0.0
    } else {
        f64::from(mean_f32(&kept))
    };
    Recognized { text, score }
}

/// `CTCLabelDecode` on the model's output `[n, steps, classes]`: at each step the most likely
/// class (the first of equal maxima, as `argmax` picks) and its probability, then
/// [`decode_steps`] per crop, in batch order.
pub fn ctc_decode(output: &Tensor, characters: &[String]) -> Vec<Recognized> {
    let [n, steps, classes] = output.shape[..] else {
        panic!(
            "the rec model's output is [n, steps, classes], not {:?}",
            output.shape
        );
    };
    assert_eq!(
        classes,
        characters.len() + 2,
        "the rec model's classes are blank, the dictionary and the space"
    );
    (0..n)
        .map(|b| {
            let (indices, probabilities): (Vec<usize>, Vec<f32>) = (0..steps)
                .map(|t| {
                    let at = (b * steps + t) * classes;
                    let row = &output.data[at..at + classes];
                    let mut best = 0;
                    for (class, &p) in row.iter().enumerate().skip(1) {
                        if p > row[best] {
                            best = class;
                        }
                    }
                    (best, row[best])
                })
                .unzip();
            decode_steps(&indices, &probabilities, characters)
        })
        .collect()
}

/// The pipeline's batches for crops of these `(width, height)` sizes, none of them empty: the
/// crops sorted by width over height, stably, and taken `batch_size` at a time in that order.
/// Each batch is its members' indices into `sizes`, in the order the model sees them.
///
/// `pipelines/ocr/pipeline.py` sorts by `sub_img_ratio`, `w / float(h)`, with Python's `sorted`,
/// which is stable; a ratio here is finite and positive, so `total_cmp` orders exactly as `<`
/// does and the stable `sort_by` keeps equal ratios in box order, as Python's does. The batches
/// are the predictor's `batch_sampler` cutting that list, so a crop's batch-mates, and so its
/// padding, are decided here.
pub fn batches(sizes: &[(u32, u32)], batch_size: usize) -> Vec<Vec<usize>> {
    assert!(batch_size > 0, "a batch holds at least one crop");
    let ratio = |i: usize| f64::from(sizes[i].0) / f64::from(sizes[i].1);
    let mut order: Vec<usize> = (0..sizes.len()).collect();
    order.sort_by(|&a, &b| ratio(a).total_cmp(&ratio(b)));
    order.chunks(batch_size).map(<[usize]>::to_vec).collect()
}

/// The OCR pipeline's recognition step on `crops` (none of them empty): read in [`batches`] of
/// the manifest's `batch_size`, each padded to its widest crop ([`rec_input`]) and decoded
/// ([`ctc_decode`]); the readings come back in `crops`' order.
pub fn recognize(models: &mut Models, crops: &[RgbImage]) -> Result<Vec<Recognized>, ModelError> {
    let rec = &Manifest::committed().models.rec;
    let sizes: Vec<(u32, u32)> = crops.iter().map(RgbImage::dimensions).collect();
    let mut readings: Vec<Option<Recognized>> = vec![None; crops.len()];
    for batch in &batches(&sizes, rec.batch_size) {
        let members: Vec<&RgbImage> = batch.iter().map(|&i| &crops[i]).collect();
        let output = models.run(Model::Rec, rec_input(&members, rec))?;
        for (&i, reading) in batch.iter().zip(ctc_decode(&output, models.characters())) {
            readings[i] = Some(reading);
        }
    }
    Ok(readings
        .into_iter()
        .map(|reading| reading.expect("every crop is in one batch"))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec() -> &'static Rec {
        &Manifest::committed().models.rec
    }

    #[test]
    fn a_narrow_crop_is_resized_by_its_ceiling_and_padded_to_the_models_width() {
        // 100x30: 48 * 100 / 30 = 160 exactly, padded to 320.
        assert_eq!(rec_widths(100, 30, rec()), (160, 320));
        // 101x30: 161.6 rounds up to 162.
        assert_eq!(rec_widths(101, 30, rec()), (162, 320));
    }

    #[test]
    fn a_wide_crop_is_resized_to_the_truncation_and_never_past_the_cap() {
        // 500x30: 48 * 500 / 30 = 800 exactly.
        assert_eq!(rec_widths(500, 30, rec()), (800, 800));
        // 333x20: 799.2 truncates to 799, and so does the resize.
        assert_eq!(rec_widths(333, 20, rec()), (799, 799));
        assert_eq!(rec_widths(5000, 30, rec()), (MAX_WIDTH, MAX_WIDTH));
    }

    #[test]
    fn numpys_float32_sum_is_pairwise() {
        // 0.1f32 nine times: a running sum gives 0.9000001, NumPy's 0.90000004 (`v.sum()`).
        let values = [0.1f32; 9];
        assert_eq!(values.iter().fold(0.0f32, |s, &v| s + v), 0.900_000_1);
        assert_eq!(pairwise_sum(&values), 0.900_000_04);
    }

    #[test]
    fn the_decode_collapses_repeats_and_drops_the_blank() {
        let characters: Vec<String> = ["a", "b"].iter().map(|s| s.to_string()).collect();
        // a a _ a b b _ <space> : "aab " from the steps kept at 0, 3, 4 and 7.
        let indices = [1, 1, 0, 1, 2, 2, 0, 3];
        let probabilities = [0.5, 0.9, 0.9, 0.25, 0.75, 0.1, 0.9, 1.0];
        let reading = decode_steps(&indices, &probabilities, &characters);
        assert_eq!(reading.text, "aab ");
        assert_eq!(reading.score, 0.625);
        let blank = decode_steps(&[0, 0], &[0.9, 0.8], &characters);
        assert_eq!(
            blank,
            Recognized {
                text: String::new(),
                score: 0.0
            }
        );
    }

    #[test]
    fn the_output_decodes_by_the_first_maximum_per_crop_in_batch_order() {
        let characters: Vec<String> = ["a", "b"].iter().map(|s| s.to_string()).collect();
        // Two crops, two steps, four classes (blank, a, b, space). Crop 0's first step ties a
        // and b at 0.4, and `argmax` keeps the first; its second step is the blank. Crop 1 reads
        // b then the space.
        let output = Tensor {
            shape: vec![2, 2, 4],
            data: vec![
                0.2, 0.4, 0.4, 0.0, //
                0.7, 0.1, 0.1, 0.1, //
                0.1, 0.1, 0.5, 0.3, //
                0.0, 0.0, 0.25, 0.75,
            ],
        };
        let readings = ctc_decode(&output, &characters);
        assert_eq!(
            readings[0],
            Recognized {
                text: "a".into(),
                score: f64::from(0.4f32)
            }
        );
        assert_eq!(readings[1].text, "b ");
        assert_eq!(readings[1].score, 0.625);
    }

    #[test]
    fn crops_batch_by_width_over_height_and_ties_keep_box_order() {
        // Ratios 4, 0.5, 2, 2, 1: sorted 0.5, 1, 2, 2, 4, the two 2s in box order.
        let sizes = [(40, 10), (5, 10), (20, 10), (40, 20), (10, 10)];
        assert_eq!(batches(&sizes, 2), vec![vec![1, 4], vec![2, 3], vec![0]]);
    }
}
