//! Where the text is: the detection model run on the working image, and its probability map
//! turned into four-corner boxes. [M29 P4]
//!
//! The twin of PaddleX 3.7.2's text-detection predictor as frozen PaddleOCR runs it
//! (`paddlex/inference/models/text_detection/`, `TextDetRunnerPredictor` with
//! `DetResizeForTest`, `NormalizeImage` and `DBPostProcess` from `processors.py`), with the
//! constants the OCR pipeline hands it (`crates/ocr/models.json`'s `det` entry, the M29 plan's P1
//! finding 10, confirmed here against the live pipeline's `get_text_det_params`):
//!
//! 1. [`det_input`]: the working image resized so each side is a multiple of 32 (OpenCV's
//!    bilinear, [`crate::cv::resize::resize_linear`]), then normalised per channel in `f32` **in
//!    BGR order**, because frozen `preprocess.py` holds BGR and PaddleX's `ReadImage(format="BGR")`
//!    passes an array through untouched, and laid out NCHW;
//! 2. the `det` model (`crate::models`), one probability per pixel;
//! 3. [`db_postprocess`]: the map thresholded, every border traced (`RETR_LIST`), each one's
//!    rotated bounding box scored by the mean probability under it, grown by the unclip
//!    ([`crate::clipper`]), boxed again, and scaled back to the working image's pixels.
//!
//! [`sort_quad_boxes`] is the OCR pipeline's own reading order (`SortQuadBoxes`), applied to the
//! detector's boxes before anything is cropped; [`crate::recognizer`] starts from it (P6).
//!
//! # How close it lands (M29 P4, the 13 bay photos)
//!
//! Measured on frozen Python's own working images, with PaddleX's det model run alone, and
//! recorded in the plan's P4 finding and pinned by `tests/det.rs`:
//!
//! - **the input tensor is PaddleX's bit for bit** on every photo;
//! - **the postprocess is PaddleX's exactly**: fed frozen Paddle's own probability maps, it gives
//!   the same boxes, the same scores to the last bit and the same reading order on all 13;
//! - **what is left is the runtime.** ONNX Runtime's map differs from Paddle's by at most about
//!   1e-3 on the same input. Python's onnxruntime 1.28 gives `ort`'s map exactly at the default
//!   optimisation level these sessions use, and with optimisation basic or off it moves by about
//!   4e-6 and lands no nearer Paddle's, so it is the runtime and not this crate's settings. That
//!   tips a single pixel across `thresh` on four photos and reshapes one small box by 2 px on one
//!   of them (2026-08-23-11). Box counts agree on every photo, and every other box is identical.
//!   Nor is any box near `box_thresh` (0.6): every kept box scores at least 0.64 and every
//!   candidate dropped for its score at most 0.53, so a runtime's last bits cannot keep a box on
//!   one side that the other drops.

use image::RgbImage;

use crate::clipper::{offset_round, to_clipper_point};
use crate::cv::contours::{find_list_contours, polygon_area, polygon_perimeter, Point};
use crate::cv::draw::{fill_poly, mean_masked};
use crate::cv::rect::{box_points, min_area_rect};
use crate::cv::resize::resize_linear;
use crate::models::{
    DbPostprocess, DetResize, Manifest, Model, ModelError, Models, Normalize, Tensor,
};

/// `DBPostProcess.min_size`: a box whose shorter side is under this is dropped before scoring,
/// and one under this plus 2 after the unclip.
const MIN_SIZE: f64 = 3.0;

/// What `DetResizeForTest` did to the image: the size it was given and the ratios it applied,
/// `np.array([src_h, src_w, ratio_h, ratio_w])`. The postprocess scales boxes back by the size
/// alone; the ratios are kept because PaddleX returns them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DetShape {
    pub src_height: usize,
    pub src_width: usize,
    pub ratio_h: f64,
    pub ratio_w: f64,
}

/// One detected text box: four corners in the working image's pixels, in `get_mini_boxes`'
/// order (top-left, top-right, bottom-right, bottom-left of the rotated box, by the rule that
/// sorts its corners by `x`), and the mean probability under the box before it was grown.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DetBox {
    pub points: [[i32; 2]; 4],
    pub score: f64,
}

/// `resize_image_type0`'s `limit_type`.
#[derive(Clone, Copy)]
enum Limit {
    Min,
    Max,
    ResizeLong,
}

fn limit(name: &str) -> Limit {
    match name {
        "min" => Limit::Min,
        "max" => Limit::Max,
        "resize_long" => Limit::ResizeLong,
        // PaddleX raises `not support limit type`. The manifest is committed and
        // `tests/det.rs` reads it, so another value is a bug in the manifest, not an input.
        other => panic!(
            "crates/ocr/models.json: det resize limit_type {other:?} is not one PaddleX knows"
        ),
    }
}

/// `resize_image_type0`, the pipeline's resize: the target size of an `h`x`w` image (already
/// padded, as [`det_input`] pads it) as `(height, width)`, and the ratios. Sides are scaled by the
/// limit, capped by `max_side_limit`, then rounded half to even to a multiple of 32 (Python's
/// `round`), never below 32.
pub fn det_size(h: usize, w: usize, resize: &DetResize) -> (usize, usize, f64, f64) {
    let side = f64::from(resize.limit_side_len);
    let ratio = match limit(&resize.limit_type) {
        Limit::Max if h.max(w) as f64 > side => side / (if h > w { h } else { w }) as f64,
        Limit::Min if (h.min(w) as f64) < side => side / (if h < w { h } else { w }) as f64,
        Limit::Max | Limit::Min => 1.0,
        Limit::ResizeLong => side / h.max(w) as f64,
    };
    let mut resize_h = (h as f64 * ratio) as usize;
    let mut resize_w = (w as f64 * ratio) as usize;
    let max_side = resize.max_side_limit as usize;
    if resize_h.max(resize_w) > max_side {
        let cap = max_side as f64 / resize_h.max(resize_w) as f64;
        resize_h = (resize_h as f64 * cap) as usize;
        resize_w = (resize_w as f64 * cap) as usize;
    }
    let to_32 = |v: usize| (((v as f64 / 32.0).round_ties_even() * 32.0) as usize).max(32);
    let (resize_h, resize_w) = (to_32(resize_h), to_32(resize_w));
    if (resize_h, resize_w) == (h, w) {
        return (h, w, 1.0, 1.0);
    }
    (
        resize_h,
        resize_w,
        resize_h as f64 / h as f64,
        resize_w as f64 / w as f64,
    )
}

/// The detector's input tensor, `[1, 3, H, W]`, and what the resize did. The image is resized
/// as 8-bit, then each channel is `f32(pixel) * f32(scale / std) + f32(-mean / std)`, the two
/// steps rounded separately as NumPy's in-place `*=` and `+=` round them, with channel 0 blue.
pub fn det_input(
    image: &RgbImage,
    resize: &DetResize,
    normalize: &Normalize,
) -> (Tensor, DetShape) {
    let (src_width, src_height) = (image.width() as usize, image.height() as usize);
    let mut pixels = image.as_raw().clone();
    let (mut width, mut height) = (src_width, src_height);
    // `image_padding`: an image whose sides sum under 64 is first padded with black to at least
    // 32x32, and the padded size is what the resize sees. The bay's working images are 1600 px
    // wide, so this is PaddleX's path ported rather than one the photos take.
    if height + width < 64 {
        let (pad_h, pad_w) = (height.max(32), width.max(32));
        let mut padded = vec![0u8; pad_h * pad_w * 3];
        for y in 0..height {
            padded[y * pad_w * 3..(y * pad_w + width) * 3]
                .copy_from_slice(&pixels[y * width * 3..(y + 1) * width * 3]);
        }
        (pixels, width, height) = (padded, pad_w, pad_h);
    }
    let (resize_h, resize_w, ratio_h, ratio_w) = det_size(height, width, resize);
    if (resize_h, resize_w) != (height, width) {
        pixels = resize_linear(&pixels, width, height, 3, resize_w, resize_h);
    }
    if normalize.img_mode != "BGR" {
        panic!(
            "crates/ocr/models.json: det img_mode {:?} is not the BGR PaddleX reads",
            normalize.img_mode
        );
    }
    let (alpha, beta) = normalize.folded();
    let plane = resize_h * resize_w;
    let mut data = vec![0f32; 3 * plane];
    for (i, pixel) in pixels.chunks_exact(3).enumerate() {
        // RGB in, BGR out: tensor channel 0 is blue.
        for c in 0..3 {
            data[c * plane + i] = f32::from(pixel[2 - c]) * alpha[c] + beta[c];
        }
    }
    (
        Tensor {
            shape: vec![1, 3, resize_h, resize_w],
            data,
        },
        DetShape {
            src_height,
            src_width,
            ratio_h,
            ratio_w,
        },
    )
}

/// `get_mini_boxes`: `minAreaRect` of the points, its corners sorted by `x` (stably, as Python's
/// `sorted`), then the left pair and the right pair each put top first, giving top-left,
/// top-right, bottom-right, bottom-left; and the box's shorter side.
pub(crate) fn mini_box(points: &[Point]) -> ([[f32; 2]; 4], f64) {
    let rect = min_area_rect(points);
    let mut corners = box_points(&rect);
    // `partial_cmp` rather than `total_cmp`: Python's `<` holds -0.0 and 0.0 equal, so the
    // stable sort keeps them in `boxPoints`' order.
    corners.sort_by(|a, b| a[0].partial_cmp(&b[0]).unwrap_or(std::cmp::Ordering::Equal));
    let (first, fourth) = if corners[1][1] > corners[0][1] {
        (0, 1)
    } else {
        (1, 0)
    };
    let (second, third) = if corners[3][1] > corners[2][1] {
        (2, 3)
    } else {
        (3, 2)
    };
    let ordered = [
        corners[first],
        corners[second],
        corners[third],
        corners[fourth],
    ];
    let side = f64::from(rect.size[0]).min(f64::from(rect.size[1]));
    (ordered, side)
}

/// `box_score_fast`: the mean of the map under the box, over the box's bounding rectangle
/// clipped to the map, with the box filled by `fillPoly` after its corners are moved into that
/// rectangle (in `f32`) and truncated to integers (`astype(np.int32)`).
fn box_score(pred: &[f32], width: usize, height: usize, corners: &[[f32; 2]; 4]) -> f64 {
    let clip = |v: f64, size: usize| v.min(size as f64 - 1.0).max(0.0) as usize;
    let xs = corners.map(|c| c[0]);
    let ys = corners.map(|c| c[1]);
    let min = |v: [f32; 4]| v.iter().copied().fold(f32::INFINITY, f32::min);
    let max = |v: [f32; 4]| v.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let xmin = clip(f64::from(min(xs)).floor(), width);
    let xmax = clip(f64::from(max(xs)).ceil(), width);
    let ymin = clip(f64::from(min(ys)).floor(), height);
    let ymax = clip(f64::from(max(ys)).ceil(), height);
    let (mask_w, mask_h) = (xmax - xmin + 1, ymax - ymin + 1);
    let mut mask = vec![0u8; mask_w * mask_h];
    let local = corners.map(|c| [(c[0] - xmin as f32) as i32, (c[1] - ymin as f32) as i32]);
    fill_poly(&mut mask, mask_w, mask_h, &local, 1);
    mean_masked(pred, width, [xmin, ymin], [mask_w, mask_h], &mask)
}

/// `unclip`: the box grown by `area * ratio / perimeter` with round corners, its corners first
/// truncated to integers as pyclipper takes them.
fn unclip(corners: &[[f32; 2]; 4], ratio: f64) -> Vec<Point> {
    let distance = polygon_area(corners) * ratio / polygon_perimeter(corners);
    let path = corners.map(to_clipper_point);
    // `minAreaRect` reads pyclipper's 64-bit answer as `int32` (`cv2` casts an `int64` array).
    offset_round(&path, distance)
        .into_iter()
        .map(|p| Point {
            x: p[0] as i32,
            y: p[1] as i32,
        })
        .collect()
}

/// `DBPostProcess` with `box_type="quad"`, `score_mode="fast"` and no dilation, on one
/// probability map `width`x`height` (row-major): the boxes in the order their borders were
/// traced, scaled to `shape`'s source size.
pub fn db_postprocess(
    pred: &[f32],
    width: usize,
    height: usize,
    shape: &DetShape,
    params: &DbPostprocess,
) -> Vec<DetBox> {
    assert_eq!(pred.len(), width * height);
    // `pred > thresh`: NumPy compares a `float32` array with a Python float in `float32`.
    let thresh = params.thresh as f32;
    let bitmap: Vec<u8> = pred
        .iter()
        .map(|&p| if p > thresh { 255 } else { 0 })
        .collect();
    let dest_width = shape.src_width as f64;
    let dest_height = shape.src_height as f64;
    let width_scale = dest_width / width as f64;
    let height_scale = dest_height / height as f64;

    let mut boxes = Vec::new();
    for contour in find_list_contours(&bitmap, width, height)
        .iter()
        .take(params.max_candidates)
    {
        let (corners, side) = mini_box(contour);
        if side < MIN_SIZE {
            continue;
        }
        let score = box_score(pred, width, height, &corners);
        if params.box_thresh > score {
            continue;
        }
        let grown = unclip(&corners, params.unclip_ratio);
        let (corners, side) = mini_box(&grown);
        if side < MIN_SIZE + 2.0 {
            continue;
        }
        // `max(0, min(round(c * scale), dest))`: a `float32` corner times a `float64` scale is a
        // `float64` product, rounded half to even. The clamp is to the size, not the last pixel.
        let scale = |v: f32, by: f64, dest: f64| {
            (f64::from(v) * by).round_ties_even().min(dest).max(0.0) as i32
        };
        let points = corners.map(|c| {
            [
                scale(c[0], width_scale, dest_width),
                scale(c[1], height_scale, dest_height),
            ]
        });
        boxes.push(DetBox { points, score });
    }
    boxes
}

/// `SortQuadBoxes`: by the first corner's `y` then `x` (stably), then one insertion pass that
/// moves a box left past its neighbours while their first corners are under 10 px apart in `y`
/// and it is further left. The OCR pipeline's reading order.
pub fn sort_quad_boxes(boxes: &[DetBox]) -> Vec<DetBox> {
    let mut sorted = boxes.to_vec();
    sorted.sort_by_key(|b| (b.points[0][1], b.points[0][0]));
    for i in 0..sorted.len().saturating_sub(1) {
        for j in (0..=i).rev() {
            let (a, b) = (sorted[j].points[0], sorted[j + 1].points[0]);
            if (b[1] - a[1]).abs() < 10 && b[0] < a[0] {
                sorted.swap(j, j + 1);
            } else {
                break;
            }
        }
    }
    sorted
}

/// The detector on one working image: its boxes in the order the detector returns them (pass
/// them through [`sort_quad_boxes`] for the pipeline's reading order).
pub fn detect(models: &mut Models, image: &RgbImage) -> Result<Vec<DetBox>, ModelError> {
    let det = &Manifest::committed().models.det;
    let (input, shape) = det_input(image, &det.resize, &det.normalize);
    let output = models.run(Model::Det, input)?;
    let (height, width) = (output.shape[2], output.shape[3]);
    Ok(db_postprocess(
        &output.data,
        width,
        height,
        &shape,
        &det.postprocess,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn committed_resize() -> &'static DetResize {
        &Manifest::committed().models.det.resize
    }

    #[test]
    fn the_working_image_sizes_round_half_to_even_onto_multiples_of_32() {
        // 1093 / 32 = 34.16 → 34 → 1088; 1200 / 32 = 37.5 → 38 (half to even) → 1216.
        assert_eq!(
            det_size(1093, 1600, committed_resize()),
            (1088, 1600, 1088.0 / 1093.0, 1.0)
        );
        assert_eq!(
            det_size(1200, 1600, committed_resize()),
            (1216, 1600, 1216.0 / 1200.0, 1.0)
        );
        assert_eq!(
            det_size(1088, 1600, committed_resize()),
            (1088, 1600, 1.0, 1.0)
        );
    }

    #[test]
    fn a_small_image_is_scaled_up_to_the_limit_on_its_short_side() {
        // limit_side_len 64, "min": a 32x48 image scales by 64 / 32 to 64x96.
        assert_eq!(det_size(32, 48, committed_resize()), (64, 96, 2.0, 2.0));
    }

    #[test]
    fn boxes_sort_top_to_bottom_then_left_to_right_within_a_line() {
        let at = |x: i32, y: i32| DetBox {
            points: [[x, y], [x + 5, y], [x + 5, y + 5], [x, y + 5]],
            score: 1.0,
        };
        // Two boxes on one line 4 px apart in y, the right one higher; a box on the next line.
        let sorted = sort_quad_boxes(&[at(100, 52), at(10, 56), at(50, 20), at(0, 90)]);
        let firsts: Vec<[i32; 2]> = sorted.iter().map(|b| b.points[0]).collect();
        assert_eq!(firsts, vec![[50, 20], [10, 56], [100, 52], [0, 90]]);
    }
}
