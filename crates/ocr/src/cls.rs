//! Which way up each line of text is: every crop classified as upright or upside down, and the
//! upside-down ones turned before they are read. [M29 P6]
//!
//! The twin of PaddleX 3.7.2's textline-orientation step as frozen PaddleOCR runs it
//! (`use_textline_orientation=True` in `paddle.py`; `pipelines/ocr/pipeline.py`, between the crop
//! and recognition):
//!
//! 1. [`cls_input`]: the image-classification predictor (`ClasRunnerPredictor`,
//!    `models/image_classification/predictor.py`) on each crop: `ReadImage(format="RGB")`, which
//!    turns frozen Python's BGR crop into RGB, so **tensor channel 0 is red** here where the
//!    detector's and the recogniser's are blue; `Resize` to the manifest's 160x80 with OpenCV's
//!    bilinear, or not at all when the crop is already that size; `NormalizeImage`, folded as the
//!    detector's is ([`crate::models::Normalize::folded`]); CHW, and stacked;
//! 2. the `textline` model (`crate::models`), two probabilities per crop, `0_degree` and
//!    `180_degree`;
//! 3. [`top_class`]: `Topk` with `topk=1`, the last of a stable ascending `argsort`, so an exact tie
//!    is read as upside down;
//! 4. [`turn_half`]: the pipeline's `rotate_image(crop, 180)`, which is not a flip (its doc says
//!    what it is).
//!
//! [`classify`] runs the crops in the order they were cut, in batches of the manifest's
//! `batch_size`, as the predictor's batch sampler cuts the pipeline's list. Unlike recognition's,
//! a batch here pads nothing, because every crop is resized to the same size, so a crop's input
//! does not depend on its batch-mates; the batches are kept because the runtime's arithmetic may.
//! There is no threshold: whichever class is likelier wins.
//!
//! # How close it lands (M29 P6, the 13 bay photos, pinned by `tests/cls.rs`)
//!
//! **No crop of the 13 is upside down**: frozen `recognize` turned none, and nor does this. So the
//! photo gate also classifies every crop flipped, and Paddle reads about a third of those as upside
//! down; short numbers and symmetric text read either way up. Fed frozen Python's boxes, on the
//! twelve photos that are not HEIC every input tensor is PaddleX's bit for bit, upright and
//! flipped, every class is Paddle's, and every probability is within 1.2e-5 of it, which is the
//! runtime. On the HEIC, whose crops are not PaddleX's (ffmpeg's decode), every upright class is
//! still Paddle's, and six of its 47 flipped crops, each one the model is unsure of, take the
//! other class.

use image::{Rgb, RgbImage};

use crate::cv::resize::resize_linear;
use crate::models::{Manifest, Model, ModelError, Models, Tensor, Textline};

/// One crop's orientation: the class `Topk` picks (0 upright, 1 upside down, by the manifest's
/// `labels`) and the model's two probabilities, as the runtime gave them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Orientation {
    pub class: usize,
    pub probabilities: [f32; 2],
}

impl Orientation {
    /// Whether the pipeline turns this crop before reading it.
    pub fn upside_down(&self) -> bool {
        self.class == 1
    }
}

/// The labels the pipeline's `rotate_image` call assumes: class `i` is a turn of `i * 180`
/// degrees, and it asserts that a class is 0 or 1.
const LABELS: [&str; 2] = ["0_degree", "180_degree"];

fn checked(textline: &Textline) {
    // `tests/cls.rs` reads the committed manifest, so another value is a bug in it, not an input.
    let normalize = &textline.normalize;
    if normalize.img_mode != "RGB" || !normalize.order.is_empty() || textline.labels != LABELS {
        panic!(
            "crates/ocr/models.json: textline img_mode {:?}, order {:?} and labels {:?} are not the \
             RGB, HWC and two orientations PaddleX's classifier and pipeline read",
            normalize.img_mode, normalize.order, textline.labels
        );
    }
}

/// The textline model's input for one batch, `[n, 3, 80, 160]`: each crop resized as 8-bit to
/// the manifest's size (OpenCV's bilinear; `cv2.resize` is skipped for a crop already that size,
/// which [`resize_linear`] mirrors by copying), then each value `f32(pixel) * alpha + beta` in RGB
/// order.
///
/// A crop exactly twice the size each way (320x160) would be an exact halving, which OpenCV turns
/// into `INTER_AREA`'s fast path and [`resize_linear`] into [`crate::cv::resize::resize_area`]'s
/// general one, P2's named gap. No crop of the 13 bay photos is that size (M29 P6).
pub fn cls_input(crops: &[&RgbImage], textline: &Textline) -> Tensor {
    checked(textline);
    assert!(!crops.is_empty(), "a batch holds at least one crop");
    let (width, height) = (
        textline.resize.width as usize,
        textline.resize.height as usize,
    );
    let (alpha, beta) = textline.normalize.folded();
    let plane = width * height;
    let mut data = vec![0f32; crops.len() * 3 * plane];
    for (n, crop) in crops.iter().enumerate() {
        let pixels = resize_linear(
            crop.as_raw(),
            crop.width() as usize,
            crop.height() as usize,
            3,
            width,
            height,
        );
        let base = n * 3 * plane;
        for (i, pixel) in pixels.chunks_exact(3).enumerate() {
            for c in 0..3 {
                data[base + c * plane + i] = f32::from(pixel[c]) * alpha[c] + beta[c];
            }
        }
    }
    Tensor {
        shape: vec![crops.len(), 3, height, width],
        data,
    }
}

/// `Topk(topk=1)`'s class for one crop's probabilities: `argsort(axis=1)[:, -1:]`, the last index
/// of an ascending sort. NumPy sorts a row this short by insertion, which is stable, so equal
/// probabilities keep their order and the **last** of equal maxima wins (checked in NumPy 2.3.5:
/// `[0.5, 0.5]` gives class 1, M29 P6). A NaN, which `argsort` puts last, cannot come out of the
/// model's softmax on a finite input, and is not given a meaning here.
pub fn top_class(probabilities: &[f32]) -> usize {
    let mut best = 0;
    for (class, &p) in probabilities.iter().enumerate().skip(1) {
        if p >= probabilities[best] {
            best = class;
        }
    }
    best
}

/// The textline model on `crops`, in their order and batches of the manifest's `batch_size`, one
/// [`Orientation`] per crop.
pub fn classify(models: &mut Models, crops: &[RgbImage]) -> Result<Vec<Orientation>, ModelError> {
    let textline = &Manifest::committed().models.textline;
    let mut orientations = Vec::with_capacity(crops.len());
    for batch in crops.chunks(textline.batch_size) {
        let members: Vec<&RgbImage> = batch.iter().collect();
        let output = models.run(Model::Textline, cls_input(&members, textline))?;
        assert_eq!(
            output.shape,
            [batch.len(), LABELS.len()],
            "the textline model's output is [n, classes]"
        );
        for row in output.data.chunks_exact(LABELS.len()) {
            orientations.push(Orientation {
                class: top_class(row),
                probabilities: [row[0], row[1]],
            });
        }
    }
    Ok(orientations)
}

/// The pipeline's `rotate_image(crop, 180)` (`pipelines/components/common/warp_image.py`): a
/// `cv2.warpAffine` by `getRotationMatrix2D((w / 2, h / 2), 180, 1)`, bicubic, into a canvas of the
/// same size with OpenCV's default black border.
///
/// **That is not a flip.** It turns about the point `(w / 2, h / 2)`, the corner between the four
/// middle pixels, where a flip turns about `((w - 1) / 2, (h - 1) / 2)`, their centre. So output
/// pixel `(x, y)` samples the source at `(w - x, h - y)`, one pixel past a flip's `(w - 1 - x,
/// h - 1 - y)`. The matrix's `sin(pi)` of 1.2e-16 is far below what OpenCV's 10-bit fixed-point
/// coordinates resolve, so every sample lands on a whole pixel, where the bicubic weights are
/// exactly `0, 1, 0, 0` and copy it. Row 0 and column 0 sample row `h` and column `w`, outside the
/// crop, and are black. Checked against `rotate_image` on 3,064 random images, every size from 1x1
/// to 8x8 among them and IPP on and off: the same bytes on all of them (M29 P6).
pub fn turn_half(crop: &RgbImage) -> RgbImage {
    let (width, height) = crop.dimensions();
    let mut turned = RgbImage::from_pixel(width, height, Rgb([0, 0, 0]));
    for y in 1..height {
        for x in 1..width {
            turned.put_pixel(x, y, *crop.get_pixel(width - x, height - y));
        }
    }
    turned
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_likelier_class_wins_and_a_tie_reads_upside_down() {
        assert_eq!(top_class(&[0.9, 0.1]), 0);
        assert_eq!(top_class(&[0.1, 0.9]), 1);
        assert_eq!(top_class(&[0.5, 0.5]), 1);
    }

    #[test]
    fn a_half_turn_is_a_flip_shifted_one_pixel_onto_a_black_edge() {
        // 3x2, rows 10 20 30 and 40 50 60. Output (x, y) is the source at (3 - x, 2 - y): row 0
        // and column 0 are black, (1, 1) is the source's (2, 1) and (2, 1) its (1, 1). A flip
        // would have given 60 50 40 over 30 20 10.
        let pixel = |v: u8| [v, v + 1, v + 2];
        let source: Vec<u8> = [10, 20, 30, 40, 50, 60]
            .iter()
            .flat_map(|&v| pixel(v))
            .collect();
        let crop = RgbImage::from_raw(3, 2, source).unwrap();
        let turned = turn_half(&crop);
        let values: Vec<u8> = turned.pixels().map(|p| p[0]).collect();
        assert_eq!(values, vec![0, 0, 0, 0, 60, 50]);
        assert_eq!(turned.get_pixel(1, 1).0, pixel(60));
        let one = RgbImage::from_pixel(1, 1, Rgb([9, 9, 9]));
        assert_eq!(turn_half(&one).get_pixel(0, 0).0, [0, 0, 0]);
    }
}
