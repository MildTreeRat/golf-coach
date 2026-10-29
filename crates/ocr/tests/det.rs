//! The text-detection gate: the boxes the detector finds, held to PaddleX's (M29 P4).
//!
//! Two references, both recorded once by scratch scripts calling frozen Python's own libraries:
//!
//! - `tests/fixtures/det_reference.json` — answers on inputs small enough to commit, so the
//!   always-run suite holds the port without the photos or the models: PaddleX's input tensor for
//!   the synthetic screen `rectify/scene.png`; `cv2.resize`'s bilinear on it at sizes that enlarge,
//!   shrink and leave a ragged row; `cv2.fillPoly` on random four-gons, half of them rotated boxes
//!   and half any four points, many past the mask's edge; `cv2.findContours(RETR_LIST)` on random
//!   blobs, holes and all; pyclipper's unclip on boxes the detector could make, at any angle and
//!   axis-aligned; and `DBPostProcess` with the OCR pipeline's
//!   arguments on a crop of a real probability map (`det/pred_crop.png`), boxes, scores and the
//!   pipeline's reading order.
//! - `tests/fixtures/det_boxes.json` — frozen Python on the 13 bay photos: PaddleX's det model run
//!   alone, through the live predictor, on frozen `preprocess.py`'s working image, each photo's
//!   input-tensor digest and the boxes and scores it returned. The photos and the models are
//!   gitignored, so that gate is an ignored test that fails, rather than passes, when either is
//!   missing (the M29 plan's rules and C2).
//!
//! What the photo gate holds the port to is in its own doc comment.

use std::fs;
use std::path::{Path, PathBuf};

use image::{GrayImage, RgbImage};
use ocr::clipper::{offset_round, to_clipper_point};
use ocr::cv::contours::find_list_contours;
use ocr::cv::draw::fill_poly;
use ocr::cv::resize::resize_linear;
use ocr::det::{db_postprocess, det_input, detect, sort_quad_boxes, DetBox, DetShape};
use ocr::models::{Manifest, Models};
use serde_json::Value;
use sha2::{Digest, Sha256};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

fn fixture(name: &str) -> Value {
    serde_json::from_str(&fs::read_to_string(fixtures().join(name)).unwrap()).unwrap()
}

fn hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn f32_digest(values: &[f32]) -> String {
    let bytes: Vec<u8> = values.iter().flat_map(|v| v.to_le_bytes()).collect();
    hex(&bytes)
}

fn scene() -> RgbImage {
    image::open(fixtures().join("rectify").join("scene.png"))
        .unwrap()
        .to_rgb8()
}

fn usize_of(value: &Value) -> usize {
    value.as_u64().unwrap() as usize
}

fn points_of(value: &Value) -> Vec<[i64; 2]> {
    value
        .as_array()
        .unwrap()
        .iter()
        .map(|p| [p[0].as_i64().unwrap(), p[1].as_i64().unwrap()])
        .collect()
}

fn boxes_of(value: &Value) -> Vec<[[i32; 2]; 4]> {
    value
        .as_array()
        .unwrap()
        .iter()
        .map(|b| {
            let corners = points_of(b);
            std::array::from_fn(|i| [corners[i][0] as i32, corners[i][1] as i32])
        })
        .collect()
}

#[test]
fn the_scene_becomes_paddlexs_input_tensor_bit_for_bit() {
    let reference = &fixture("det_reference.json")["scene_input"];
    let det = &Manifest::committed().models.det;
    let (input, shape) = det_input(&scene(), &det.resize, &det.normalize);
    let want_shape: Vec<usize> = reference["shape"]
        .as_array()
        .unwrap()
        .iter()
        .map(usize_of)
        .collect();
    assert_eq!(input.shape, want_shape);
    let img_shape: Vec<f64> = reference["img_shape"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_f64().unwrap())
        .collect();
    assert_eq!(
        [
            shape.src_height as f64,
            shape.src_width as f64,
            shape.ratio_h,
            shape.ratio_w
        ],
        img_shape[..]
    );
    assert_eq!(
        f32_digest(&input.data),
        reference["sha256"].as_str().unwrap()
    );
}

#[test]
fn bilinear_resizes_match_opencv() {
    let source = scene();
    let (width, height) = (source.width() as usize, source.height() as usize);
    let red: Vec<u8> = source.pixels().map(|p| p[0]).collect();
    let cases = fixture("det_reference.json")["resize"].clone();
    assert!(cases.as_array().unwrap().len() >= 9);
    for case in cases.as_array().unwrap() {
        let (channels, dw, dh) = (
            usize_of(&case["channels"]),
            usize_of(&case["width"]),
            usize_of(&case["height"]),
        );
        let src = if channels == 3 {
            source.as_raw().as_slice()
        } else {
            red.as_slice()
        };
        let out = resize_linear(src, width, height, channels, dw, dh);
        assert_eq!(
            hex(&out),
            case["sha256"].as_str().unwrap(),
            "{channels} channel(s) to {dw}x{dh}"
        );
    }
}

#[test]
fn polygons_fill_as_opencv_fills_them() {
    let cases = fixture("det_reference.json")["fill"].clone();
    let cases = cases.as_array().unwrap();
    assert!(cases.len() >= 200);
    // A port that filled nothing, or everything, could not pass: the cases are both.
    assert!(cases.iter().any(|c| c["filled"] == 0));
    assert!(cases
        .iter()
        .any(|c| usize_of(&c["filled"]) == usize_of(&c["width"]) * usize_of(&c["height"])));
    for (i, case) in cases.iter().enumerate() {
        let (w, h) = (usize_of(&case["width"]), usize_of(&case["height"]));
        let points: Vec<[i32; 2]> = points_of(&case["points"])
            .iter()
            .map(|p| [p[0] as i32, p[1] as i32])
            .collect();
        let mut mask = vec![0u8; w * h];
        fill_poly(&mut mask, w, h, &points, 1);
        assert_eq!(
            hex(&mask),
            case["sha256"].as_str().unwrap(),
            "case {i}: {w}x{h} {points:?}"
        );
    }
}

#[test]
fn list_contours_match_opencv_holes_and_all() {
    let cases = fixture("det_reference.json")["contours"].clone();
    let cases = cases.as_array().unwrap();
    let mut total = 0;
    for (i, case) in cases.iter().enumerate() {
        let (w, h) = (usize_of(&case["width"]), usize_of(&case["height"]));
        let bitmap: Vec<u8> = case["rows"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|row| {
                row.as_str()
                    .unwrap()
                    .bytes()
                    .map(|b| if b == b'1' { 255 } else { 0 })
                    .collect::<Vec<u8>>()
            })
            .collect();
        let found: Vec<Vec<[i64; 2]>> = find_list_contours(&bitmap, w, h)
            .iter()
            .map(|c| c.iter().map(|p| [i64::from(p.x), i64::from(p.y)]).collect())
            .collect();
        let want: Vec<Vec<[i64; 2]>> = case["contours"]
            .as_array()
            .unwrap()
            .iter()
            .map(points_of)
            .collect();
        total += want.len();
        assert_eq!(found, want, "case {i}");
    }
    assert!(total > cases.len() * 2, "the blobs have holes and islands");
}

#[test]
fn the_unclip_gives_pyclippers_vertices_in_its_order() {
    let cases = fixture("det_reference.json")["offset"].clone();
    for (i, case) in cases.as_array().unwrap().iter().enumerate() {
        let path: Vec<[i64; 2]> = case["points"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| {
                to_clipper_point([p[0].as_f64().unwrap() as f32, p[1].as_f64().unwrap() as f32])
            })
            .collect();
        let grown = offset_round(&path, case["distance"].as_f64().unwrap());
        assert_eq!(grown, points_of(&case["vertices"]), "case {i}");
    }
}

#[test]
fn the_postprocess_matches_paddlex_on_a_real_map() {
    let reference = &fixture("det_reference.json")["postprocess"];
    let map: GrayImage = image::open(fixtures().join("det").join("pred_crop.png"))
        .unwrap()
        .to_luma8();
    let (width, height) = (map.width() as usize, map.height() as usize);
    let pred: Vec<f32> = map.as_raw().iter().map(|&k| f32::from(k) / 255.0).collect();
    let source = &reference["source"];
    let shape = DetShape {
        src_height: usize_of(&source[0]),
        src_width: usize_of(&source[1]),
        ratio_h: 0.0,
        ratio_w: 0.0,
    };
    let params = &Manifest::committed().models.det.postprocess;
    let boxes = db_postprocess(&pred, width, height, &shape, params);
    let want = boxes_of(&reference["boxes"]);
    assert!(want.len() >= 5, "the crop holds several boxes");
    let got: Vec<[[i32; 2]; 4]> = boxes.iter().map(|b| b.points).collect();
    assert_eq!(got, want);
    // OpenCV's own masked mean, which this ports; `cv2.mean` with IPP on lands within a few ulps.
    for (b, (ipp_off, ipp_on)) in boxes.iter().zip(
        reference["scores_ipp_off"]
            .as_array()
            .unwrap()
            .iter()
            .zip(reference["scores"].as_array().unwrap()),
    ) {
        assert_eq!(b.score, ipp_off.as_f64().unwrap());
        assert!((b.score - ipp_on.as_f64().unwrap()).abs() < 1e-12);
    }
    let sorted: Vec<[[i32; 2]; 4]> = sort_quad_boxes(&boxes).iter().map(|b| b.points).collect();
    assert_eq!(sorted, boxes_of(&reference["sorted"]));
}

#[test]
fn the_bay_photos_boxes_sort_into_the_pipelines_reading_order() {
    // Frozen Python's own boxes, so this holds `SortQuadBoxes` on real layouts without the
    // photos: rows of tiles whose first corners sit a few pixels apart.
    let photos = fixture("det_boxes.json")["photos"].clone();
    let photos = photos.as_array().unwrap();
    assert_eq!(photos.len(), 13);
    let mut reordered = 0;
    for photo in photos {
        let boxes: Vec<DetBox> = boxes_of(&photo["boxes"])
            .into_iter()
            .map(|points| DetBox { points, score: 1.0 })
            .collect();
        let sorted: Vec<[[i32; 2]; 4]> = sort_quad_boxes(&boxes).iter().map(|b| b.points).collect();
        let want = boxes_of(&photo["sorted"]);
        assert_eq!(sorted, want, "{}", photo["id"]);
        reordered += usize::from(want != boxes_of(&photo["boxes"]));
    }
    assert_eq!(
        reordered,
        photos.len(),
        "the detector's own order is never the reading order"
    );
}

/// How far a corner may land from frozen Python's on a bay photo, in the working image's pixels.
const CORNER_TOLERANCE_PX: i32 = 2;

/// How many boxes, across the twelve photos held to the bound, may differ from Python's at all.
const MOVED_BOXES_ALLOWED: usize = 1;

/// How far a corner may land from frozen Python's on the HEIC, in reading order. A guard, not
/// the bound: 3 px is the worst measured (M29 P4), and the gate's doc says why the HEIC is apart.
const HEIC_CORNER_GUARD_PX: i32 = 3;

/// The detection gate on the 13 bay photos.
///
/// Each photo is decoded by this crate and cut by frozen Python's own quad from
/// `screen_quads.json` (as `tests/rectify.rs` cuts it), so the stage under test is detection
/// alone. On the twelve photos whose working image is identical to frozen Python's
/// (`tests/rectify.rs`), held to **the bound**:
///
/// - **the same input tensor, bit for bit**: the resize and the normalisation are exact;
/// - **the same number of boxes, in the detector's own order**, every corner within
///   [`CORNER_TOLERANCE_PX`], and no more than [`MOVED_BOXES_ALLOWED`] box across the twelve not
///   identical.
///
/// Why any box may move at all (M29 P4): fed frozen Paddle's own probability maps, the
/// postprocess gives Paddle's boxes and scores exactly on all 13, and PaddleX's postprocess fed
/// this crate's maps gives this crate's boxes. The difference is the runtime. ONNX Runtime and
/// Paddle disagree on these maps by up to about 1e-3 on the same input (Python's onnxruntime
/// 1.28 reproduces `ort`'s map exactly at the default optimisation level, and basic or none
/// moves it by about 4e-6 and no nearer Paddle's, so it is not this crate's session settings),
/// which tips a pixel across the 0.3 threshold on four photos. On one, 2026-08-23-11, that
/// reshapes a small box near the screen's foot by 2 px.
///
/// **The HEIC (2026-08-23-1) gets a guard instead**, as `tests/rectify.rs` gives it one. Its
/// working image here comes from ffmpeg's decode, which is darker than libheif's by up to two
/// levels (the M29 plan's P2 and P3 findings), so its input is not Python's and the bound cannot
/// be its measure. Fed frozen Python's own working image, as the P4 reference was, its boxes are
/// Python's exactly. From this crate's decode the count is the same, and in **reading order**
/// ([`sort_quad_boxes`]) every corner is within [`HEIC_CORNER_GUARD_PX`]: 16 of its 47 boxes move,
/// 11 of them by 1 px. The detector's own order is not held, because a pixel's change at the top
/// of two blobs on nearly the same row swaps which one the scan meets first. Whether those
/// pixels change what the parser reads is the M29 plan's P7 gate's question.
#[test]
#[ignore = "needs the 13 bay photos in data/processed/, ffmpeg 7.1 or later (GOLF_FFMPEG or PATH) and the ONNX models in data/models/ocr/"]
fn every_bay_photo_detects_its_text_as_frozen_python_does() {
    let root = repo_root();
    let mut models = Models::open(&root.join(ocr::DEFAULT_DIR)).unwrap();
    let det = &Manifest::committed().models.det;
    let mut failures = Vec::new();
    let mut moved = 0;
    let mut heif_photos = 0;
    for photo in fixture("det_boxes.json")["photos"].as_array().unwrap() {
        let id = photo["id"].as_str().unwrap();
        let path = root.join(photo["image_path"].as_str().unwrap());
        let image = ocr::decode::load_image(&path).unwrap_or_else(|e| panic!("{id}: {e}"));
        let quad = &photo["quad"];
        let shrunk = if quad.is_null() {
            ocr::rectify::resize(&image)
        } else {
            let corners: [[f32; 2]; 4] = std::array::from_fn(|i| {
                [
                    quad[i][0].as_f64().unwrap() as f32,
                    quad[i][1].as_f64().unwrap() as f32,
                ]
            });
            ocr::rectify::resize(&ocr::rectify::warp_to_rect(&image, &corners))
        };
        let working = ocr::rectify::enhance(&shrunk);
        let (input, _) = det_input(&working, &det.resize, &det.normalize);
        let identical_input = f32_digest(&input.data) == photo["x_sha256"].as_str().unwrap();
        let boxes = detect(&mut models, &working).unwrap();
        let here = if ocr::decode::is_heif(&path) {
            heif_photos += 1;
            let sorted = sort_quad_boxes(&boxes);
            check_boxes(
                id,
                &sorted,
                &photo["sorted"],
                HEIC_CORNER_GUARD_PX,
                &mut failures,
            )
        } else {
            if !identical_input {
                failures.push(format!("{id}: the input tensor differs from PaddleX's"));
            }
            let here = check_boxes(
                id,
                &boxes,
                &photo["boxes"],
                CORNER_TOLERANCE_PX,
                &mut failures,
            );
            moved += here;
            here
        };
        eprintln!(
            "{id}: {} boxes, {here} not identical, input identical: {identical_input}",
            boxes.len()
        );
    }
    // The guard is the HEIC's only check, so a fixture that lost it must not pass in silence.
    assert_eq!(heif_photos, 1, "the bay photos hold one HEIC");
    if moved > MOVED_BOXES_ALLOWED {
        failures.push(format!(
            "{moved} boxes differ from Python's, more than the {MOVED_BOXES_ALLOWED} allowed"
        ));
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Records a failure for a different box count or a corner past `tolerance`, and returns how
/// many boxes are not identical.
fn check_boxes(
    id: &str,
    boxes: &[DetBox],
    want: &Value,
    tolerance: i32,
    failures: &mut Vec<String>,
) -> usize {
    let want = boxes_of(want);
    if boxes.len() != want.len() {
        failures.push(format!(
            "{id}: {} boxes, Python {}",
            boxes.len(),
            want.len()
        ));
        return 0;
    }
    let mut moved = 0;
    for (i, (got, want)) in boxes.iter().zip(&want).enumerate() {
        let off = got
            .points
            .iter()
            .zip(want)
            .flat_map(|(g, w)| [(g[0] - w[0]).abs(), (g[1] - w[1]).abs()])
            .max()
            .unwrap();
        moved += usize::from(off > 0);
        if off > tolerance {
            failures.push(format!(
                "{id}: box {i} {:?} is {off} px from Python's {want:?}",
                got.points
            ));
        }
    }
    moved
}
