//! The recognition gate: the crops, tensors and readings, held to PaddleX's (M29 P5).
//!
//! Two references, both recorded once by scratch scripts calling frozen Python's own libraries:
//!
//! - `tests/fixtures/rec_reference.json` — answers on inputs small enough to commit, so the
//!   always-run suite holds the port without the photos or the models: PaddleX's
//!   `CropByPolys(det_box_type="quad")` on the synthetic scene `rectify/scene.png` for 60 boxes
//!   (rotated rectangles at any angle, a quarter of them tall enough to be turned, many past the
//!   scene's edge, and some sets of any four points), its text-recognition preprocessing on those
//!   crops in the pipeline's order and batches, and the decode of some real crops' steps, as
//!   frozen Paddle's rec model scored them on the bay photos.
//! - `tests/fixtures/rec_texts.json` — frozen Python on the 13 bay photos: PaddleX's crops of the
//!   detector's boxes (`det_boxes.json`'s `sorted`, the reading order) on frozen `preprocess.py`'s
//!   working image, each crop's size and digest, each batch's input-tensor digest, and the text and
//!   score PaddleX's rec model read, through the live predictor with the pipeline's batches. The
//!   photos and the models are gitignored, so that gate is an ignored test that fails, rather than
//!   passes, when either is missing (the M29 plan's rules and C2).
//!
//! What the photo gate holds the port to is in its own doc comment.

use std::fs;
use std::path::{Path, PathBuf};

use image::RgbImage;
use ocr::cv::resize::resize_linear;
use ocr::models::{Manifest, Models};
use ocr::rec::{batches, crop, decode_steps, rec_input, recognize};
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

fn box_of(value: &Value) -> [[i32; 2]; 4] {
    std::array::from_fn(|i| {
        [
            value[i][0].as_i64().unwrap() as i32,
            value[i][1].as_i64().unwrap() as i32,
        ]
    })
}

/// The dictionary as far as the decode cases reach it: the fixture's entries at their classes,
/// and every other entry empty, at the manifest's length. The real dictionary is gitignored beside
/// the models, so this lets the decode run on every checkout; a case that reached an entry the
/// fixture left out would lose that character and fail, never pass. The photo gate checks the
/// fixture's entries against the real dictionary.
fn fixture_characters(decode: &Value) -> Vec<String> {
    let mut characters = vec![String::new(); Manifest::committed().models.rec.dict.entries];
    for (class, character) in decode["characters"].as_object().unwrap() {
        let class: usize = class.parse().unwrap();
        characters[class - 1] = character.as_str().unwrap().to_owned();
    }
    characters
}

#[test]
fn the_scenes_crops_are_paddlexs_pixel_for_pixel() {
    let scene = scene();
    let cases = fixture("rec_reference.json")["crops"].clone();
    let cases = cases.as_array().unwrap();
    assert_eq!(cases.len(), 60);
    let mut turned = 0;
    for (i, case) in cases.iter().enumerate() {
        let cut = crop(&scene, &box_of(&case["points"]));
        assert_eq!(
            (cut.width() as usize, cut.height() as usize),
            (usize_of(&case["width"]), usize_of(&case["height"])),
            "crop {i}"
        );
        assert_eq!(
            hex(cut.as_raw()),
            case["rgb_sha256"].as_str().unwrap(),
            "crop {i}"
        );
        turned += usize::from(case["turned"].as_bool().unwrap());
    }
    // The fixture's turned crops must stay in it, or the quarter turn goes untested.
    assert!(turned >= 10, "only {turned} turned crops");
}

#[test]
fn the_scenes_batches_are_paddlexs_input_tensors_bit_for_bit() {
    let scene = scene();
    let reference = fixture("rec_reference.json");
    let crops: Vec<RgbImage> = reference["crops"]
        .as_array()
        .unwrap()
        .iter()
        .map(|case| crop(&scene, &box_of(&case["points"])))
        .collect();
    let rec = &Manifest::committed().models.rec;
    let (width, height) = (scene.width() as usize, scene.height() as usize);
    let alone = |w: usize, h: usize| {
        RgbImage::from_raw(
            w as u32,
            h as u32,
            resize_linear(scene.as_raw(), width, height, 3, w, h),
        )
        .unwrap()
    };
    let batches = reference["batches"].as_array().unwrap();
    assert!(batches.len() >= 12);
    for (i, batch) in batches.iter().enumerate() {
        let members = batch["members"].as_array().unwrap();
        // The two crops recorded alone: past the 3200 px cap, and exactly the model's ratio.
        let single;
        let images: Vec<&RgbImage> = match members[0].as_str() {
            Some("wide") => {
                single = alone(width * 16, 24);
                vec![&single]
            }
            Some("model_ratio") => {
                single = alone(320, 48);
                vec![&single]
            }
            _ => members.iter().map(|m| &crops[usize_of(m)]).collect(),
        };
        let input = rec_input(&images, rec);
        let want: Vec<usize> = batch["shape"]
            .as_array()
            .unwrap()
            .iter()
            .map(usize_of)
            .collect();
        assert_eq!(input.shape, want, "batch {i}");
        assert_eq!(
            f32_digest(&input.data),
            batch["sha256"].as_str().unwrap(),
            "batch {i}"
        );
    }
}

#[test]
fn frozen_paddles_steps_decode_to_its_texts_and_scores() {
    let decode = fixture("rec_reference.json")["decode"].clone();
    let characters = fixture_characters(&decode);
    let cases = decode["cases"].as_array().unwrap();
    assert!(cases.len() >= 10);
    for case in cases {
        let at = format!("{} box {}", case["photo"], case["box"]);
        let indices: Vec<usize> = case["indices"]
            .as_array()
            .unwrap()
            .iter()
            .map(usize_of)
            .collect();
        // Each probability is a `float32` widened by `float()`, so narrowing it back is exact.
        let probabilities: Vec<f32> = case["probabilities"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| p.as_f64().unwrap() as f32)
            .collect();
        let reading = decode_steps(&indices, &probabilities, &characters);
        assert_eq!(reading.text, case["text"].as_str().unwrap(), "{at}");
        assert_eq!(reading.score, case["score"].as_f64().unwrap(), "{at}");
    }
}

#[test]
fn the_bay_photos_crops_batch_as_the_pipeline_batches_them() {
    let rec = &Manifest::committed().models.rec;
    let photos = fixture("rec_texts.json")["photos"].clone();
    let photos = photos.as_array().unwrap();
    assert_eq!(photos.len(), 13);
    for photo in photos {
        let id = photo["id"].as_str().unwrap();
        let sizes: Vec<(u32, u32)> = photo["crops"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| (usize_of(&c["width"]) as u32, usize_of(&c["height"]) as u32))
            .collect();
        let want: Vec<Vec<usize>> = photo["batches"]
            .as_array()
            .unwrap()
            .iter()
            .map(|b| {
                b["members"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(usize_of)
                    .collect()
            })
            .collect();
        assert_eq!(batches(&sizes, rec.batch_size), want, "{id}");
    }
}

/// How far a score may land from frozen Paddle's on a box of the twelve photos held to the
/// bound: the worst measured, 2.3e-5 (2026-08-23-4), rounded up (M29 P5).
const SCORE_TOLERANCE: f64 = 3e-5;

/// How far a score may land from frozen Paddle's on the HEIC's boxes. A guard, not the bound: the
/// worst measured, 0.026, rounded up (M29 P5), and the gate's doc says why the HEIC is apart.
const HEIC_SCORE_GUARD: f64 = 0.03;

/// The recognition gate on the 13 bay photos.
///
/// Each photo is decoded by this crate and cut by frozen Python's quad, as `tests/det.rs` cuts
/// it, and each of frozen Python's own boxes (`det_boxes.json`'s `sorted`, the reading order) is
/// cropped from that working image, so the stage under test is recognition alone, fed the boxes
/// PaddleX was fed. On the twelve photos whose working image is frozen Python's
/// (`tests/rectify.rs`), held to **the bound**:
///
/// - **every crop PaddleX's, pixel for pixel**, and the pipeline's batches with **every input
///   tensor bit for bit**;
/// - **every text Paddle's**, and every score within [`SCORE_TOLERANCE`].
///
/// Why a score may move at all (M29 P5): fed frozen Paddle's own steps, the decode gives its
/// texts and scores to the bit (`frozen_paddles_steps_decode_to_its_texts_and_scores`), so the
/// difference is the runtime. ONNX Runtime's probabilities differ from Paddle's by up to 3.4e-4 a
/// step on these inputs, no step's most likely class changes on the twelve, and a score is a
/// mean of those probabilities.
///
/// **The HEIC (2026-08-23-1) gets a guard instead**, as `tests/rectify.rs` and `tests/det.rs`
/// give it one. Its working image here comes from ffmpeg's decode, darker than libheif's by up to
/// two levels (the M29 plan's P2 and P3 findings), so none of its crops is PaddleX's and the
/// bound cannot be its measure. Its crops' sizes, and so its batches, are still Python's, because
/// a crop's size is its box's. Every text is still Paddle's. The different pixels move a step's
/// probability by up to 0.14, and the most likely class at five steps (each time between the
/// blank and the character beside it) without changing a text, and the worst score moves by 0.026
/// ("Ball Speed"), which [`HEIC_SCORE_GUARD`] holds.
/// Whether the HEIC's own boxes read the same is the M29 plan's P7 gate's question.
#[test]
#[ignore = "needs the 13 bay photos in data/processed/, ffmpeg 7.1 or later (GOLF_FFMPEG or PATH) and the ONNX models in data/models/ocr/"]
fn every_bay_photos_boxes_read_as_frozen_paddle_reads_them() {
    let root = repo_root();
    let mut models = Models::open(&root.join(ocr::DEFAULT_DIR)).unwrap();
    let rec = &Manifest::committed().models.rec;
    // The decode cases' dictionary entries are the real dictionary's.
    let decode = fixture("rec_reference.json")["decode"].clone();
    for (class, character) in decode["characters"].as_object().unwrap() {
        let class: usize = class.parse().unwrap();
        assert_eq!(
            models.characters()[class - 1],
            character.as_str().unwrap(),
            "class {class}"
        );
    }
    let det = fixture("det_boxes.json");
    let texts = fixture("rec_texts.json");
    let (det, texts) = (
        det["photos"].as_array().unwrap(),
        texts["photos"].as_array().unwrap(),
    );
    assert_eq!(det.len(), texts.len());
    let mut failures = Vec::new();
    let mut heif_photos = 0;
    for (photo, want) in det.iter().zip(texts) {
        let id = photo["id"].as_str().unwrap();
        assert_eq!(id, want["id"].as_str().unwrap());
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
        let heif = ocr::decode::is_heif(&path);
        heif_photos += usize::from(heif);
        let crops: Vec<RgbImage> = photo["sorted"]
            .as_array()
            .unwrap()
            .iter()
            .map(|points| crop(&working, &box_of(points)))
            .collect();
        let want_crops = want["crops"].as_array().unwrap();
        assert_eq!(crops.len(), want_crops.len(), "{id}");
        // A crop's size is its box's alone, so even the HEIC's are Python's, and so are its
        // batches.
        let sizes: Vec<(u32, u32)> = crops.iter().map(RgbImage::dimensions).collect();
        for (i, (cut, case)) in crops.iter().zip(want_crops).enumerate() {
            let size = (
                usize_of(&case["width"]) as u32,
                usize_of(&case["height"]) as u32,
            );
            if cut.dimensions() != size {
                failures.push(format!(
                    "{id} box {i}: crop {:?}, Python {size:?}",
                    cut.dimensions()
                ));
            } else if !heif && hex(cut.as_raw()) != case["rgb_sha256"].as_str().unwrap() {
                failures.push(format!(
                    "{id} box {i}: the crop's pixels differ from PaddleX's"
                ));
            }
        }
        let want_batches = want["batches"].as_array().unwrap();
        let members: Vec<Vec<usize>> = want_batches
            .iter()
            .map(|b| {
                b["members"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(usize_of)
                    .collect()
            })
            .collect();
        if batches(&sizes, rec.batch_size) != members {
            failures.push(format!("{id}: the batches differ from the pipeline's"));
        }
        if !heif {
            for (k, (batch, case)) in members.iter().zip(want_batches).enumerate() {
                let images: Vec<&RgbImage> = batch.iter().map(|&i| &crops[i]).collect();
                if f32_digest(&rec_input(&images, rec).data) != case["sha256"].as_str().unwrap() {
                    failures.push(format!(
                        "{id} batch {k}: the input tensor differs from PaddleX's"
                    ));
                }
            }
        }
        let readings = recognize(&mut models, &crops).unwrap();
        let (mut differ, mut worst) = (0, 0f64);
        let tolerance = if heif {
            HEIC_SCORE_GUARD
        } else {
            SCORE_TOLERANCE
        };
        for (i, (reading, case)) in readings
            .iter()
            .zip(want["results"].as_array().unwrap())
            .enumerate()
        {
            let (text, score) = (
                case["text"].as_str().unwrap(),
                case["score"].as_f64().unwrap(),
            );
            if reading.text != text {
                differ += 1;
                failures.push(format!(
                    "{id} box {i}: read {:?}, Paddle {text:?}",
                    reading.text
                ));
            }
            let off = (reading.score - score).abs();
            worst = worst.max(off);
            if off > tolerance {
                failures.push(format!(
                    "{id} box {i}: score {} vs Paddle's {score}",
                    reading.score
                ));
            }
        }
        eprintln!(
            "{id}: {} boxes, {differ} texts differ, worst score difference {worst:e}",
            crops.len()
        );
    }
    // The HEIC is held to its texts and scores alone, so a fixture that lost it must not pass in
    // silence.
    assert_eq!(heif_photos, 1, "the bay photos hold one HEIC");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
