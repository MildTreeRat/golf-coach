//! The rectification gate: the image the recognizer sees, held to OpenCV's (M29 P3).
//!
//! Two references, both recorded once by scratch scripts calling `cv2` and frozen
//! `launch_monitor/screen/preprocess.py`:
//!
//! - `tests/fixtures/rectify_reference.json` — OpenCV's answers on inputs small enough to commit,
//!   so the always-run suite holds the port without the photos: the 8-bit L\*a\*b\* conversion
//!   both ways on **every** input (2^24 each), and `warp_to_rect`, `enhance` and `_rotate` on a
//!   seeded synthetic screen (`tests/fixtures/rectify/scene.png`), cut to sizes that take each of
//!   CLAHE's padding branches and with a quad that reaches past the image.
//! - `tests/fixtures/screen_working.json` — frozen Python's working image on the 13 bay photos
//!   (`prepare_screen` up to its rotation vote), stage by stage. The photos are gitignored, so that
//!   gate is an ignored test that fails, rather than passes, when they are missing (the M29 plan's
//!   rules and C2).
//!
//! What the photo gate holds the port to:
//!
//! - **the same size** at every stage on every photo, HEIC included;
//! - **the same pixels** at every stage on the twelve photos whose decode is identical
//!   (`ocr::decode`). P3 measured it identical on the HEIC too when fed frozen Python's own decoded
//!   pixels, so the port adds nothing there;
//! - **on the HEIC, each channel's mean within [`HEIC_MEAN_TOLERANCE`]** of Python's, and no
//!   more. Its decode differs by up to 2 levels (ffmpeg's colour conversion against libheif's, P2),
//!   always darker, and the warp and CLAHE spread that: against Python's working image P3 measured
//!   a mean absolute difference of 1.10 levels, a 99th percentile of 4 and a maximum of 17. Whether
//!   the recognizer can tell is the OCR gate's to say (P7), and the pixels to measure it against
//!   are too large to commit, so this check is a guard against a gross break, not the bound.

use std::fs;
use std::path::{Path, PathBuf};

use image::RgbImage;
use ocr::cv::lab::{lab_to_rgb, rgb_to_lab};
use ocr::decode::{is_heif, load_image};
use ocr::rectify::{enhance, resize, rotate, warp_to_rect, Rotation};
use serde_json::Value;
use sha2::{Digest, Sha256};

/// How far each channel's mean may drift on the HEIC, in levels: one level, the decode's own
/// largest bias rounded up. ffmpeg's decoded green is 0.84 of a level darker than libheif's on
/// average (red 0.45, blue 0.42, no value lighter; M29 P3), and after CLAHE the working image's
/// green mean sits 0.57 below Python's. A first guess of 0.5, made before that was measured, failed
/// on it. The module doc says why this is a guard and not the bound.
const HEIC_MEAN_TOLERANCE: f64 = 1.0;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

fn fixture(name: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name);
    serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap()
}

fn hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn size(image: &RgbImage) -> (u64, u64) {
    (u64::from(image.width()), u64::from(image.height()))
}

/// `None` when `image` is the expected one, every pixel; otherwise what differs.
fn mismatch(image: &RgbImage, expected: &Value) -> Option<String> {
    let want = (
        expected["width"].as_u64().unwrap(),
        expected["height"].as_u64().unwrap(),
    );
    if size(image) != want {
        return Some(format!("size {:?}, OpenCV {want:?}", size(image)));
    }
    (hex(image.as_raw()) != expected["rgb_sha256"].as_str().unwrap())
        .then(|| "pixels differ from OpenCV's".to_string())
}

fn channel_means(image: &RgbImage) -> [f64; 3] {
    let mut sums = [0u64; 3];
    for pixel in image.pixels() {
        for (sum, value) in sums.iter_mut().zip(pixel.0) {
            *sum += u64::from(value);
        }
    }
    let n = (image.width() * image.height()) as f64;
    sums.map(|s| s as f64 / n)
}

/// The digest of `convert` over every 8-bit triple, in the fixture's order.
fn digest_of_every_input(convert: fn([u8; 3]) -> [u8; 3]) -> String {
    let mut hasher = Sha256::new();
    let mut row = Vec::with_capacity(3 * 4096);
    for high in 0..4096u32 {
        row.clear();
        for low in 0..4096u32 {
            let i = high << 12 | low;
            row.extend(convert([(i >> 16) as u8, (i >> 8) as u8, i as u8]));
        }
        hasher.update(&row);
    }
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

// Exhaustive rather than sampled: one cube-root table entry in 3,072 was what moved 17,445
// colours (`ocr::cv::lab`'s `softfloat_cbrt`), and no sample would have been sure to reach it.
// One test per direction, so the harness runs the two side by side (about 9 s each in a debug
// build).

#[test]
fn every_rgb_input_converts_to_lab_as_opencv_does() {
    let every = &fixture("rectify_reference.json")["every_input"];
    assert_eq!(
        digest_of_every_input(rgb_to_lab),
        every["rgb_to_lab_sha256"].as_str().unwrap()
    );
}

#[test]
fn every_lab_input_converts_to_rgb_as_opencv_does() {
    let every = &fixture("rectify_reference.json")["every_input"];
    assert_eq!(
        digest_of_every_input(lab_to_rgb),
        every["lab_to_rgb_sha256"].as_str().unwrap()
    );
}

#[test]
fn the_synthetic_screen_rectifies_as_opencv_does() {
    let reference = fixture("rectify_reference.json");
    let scene_ref = &reference["scene"];
    let scene_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(scene_ref["path"].as_str().unwrap());
    let scene = load_image(&scene_path).unwrap();
    assert_eq!(
        mismatch(&scene, scene_ref),
        None,
        "the scene does not decode to the pixels its answers were recorded from"
    );

    let cases = reference["cases"].as_array().unwrap();
    let ops: Vec<&str> = cases.iter().map(|c| c["op"].as_str().unwrap()).collect();
    // Each stage is exercised, so a fixture that lost one cannot pass vacuously.
    for op in ["enhance", "warp_to_rect", "rotate"] {
        assert!(ops.contains(&op), "no {op} case");
    }

    let mut failures = Vec::new();
    for case in cases {
        let name = case["name"].as_str().unwrap();
        let output = match case["op"].as_str().unwrap() {
            "enhance" => {
                let c: Vec<u32> = case["crop"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_u64().unwrap() as u32)
                    .collect();
                let crop = image::imageops::crop_imm(&scene, c[0], c[1], c[2], c[3]).to_image();
                enhance(&crop)
            }
            "warp_to_rect" => {
                let corners: Vec<[f32; 2]> = case["quad"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|p| [p[0].as_f64().unwrap() as f32, p[1].as_f64().unwrap() as f32])
                    .collect();
                warp_to_rect(&scene, &[corners[0], corners[1], corners[2], corners[3]])
            }
            "rotate" => {
                let degrees = case["degrees"].as_u64().unwrap();
                let rotation = Rotation::VOTE_ORDER
                    .into_iter()
                    .find(|r| u64::from(r.degrees()) == degrees)
                    .unwrap();
                rotate(&scene, rotation)
            }
            other => panic!("{name}: unknown op {other}"),
        };
        if let Some(why) = mismatch(&output, &case["expected"]) {
            failures.push(format!("{name}: {why}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn the_photo_reference_covers_both_branches_and_the_heic() {
    let reference = fixture("screen_working.json");
    let photos = reference["photos"].as_array().unwrap();
    assert_eq!(photos.len(), 13);
    let warped = photos.iter().filter(|p| !p["warped"].is_null()).count();
    // prepare_screen warps a found screen and reads the rest uncropped; both are covered.
    assert!(warped > 0 && warped < photos.len(), "{warped} warped");
    assert!(photos
        .iter()
        .any(|p| p["image_path"].as_str().unwrap().ends_with(".HEIC")));
}

#[test]
#[ignore = "needs the 13 bay photos in data/processed/ and ffmpeg 7.1 or later (GOLF_FFMPEG or PATH)"]
fn every_bay_photo_rectifies_as_frozen_python_does() {
    let root = repo_root();
    // Each photo is cut by Python's own quad (`screen_quads.json`), which `tests/quad.rs` holds the
    // finder to, so this gate measures rectification alone; P7's gate runs the two together.
    let quads = fixture("screen_quads.json");
    let mut failures = Vec::new();
    for photo in fixture("screen_working.json")["photos"].as_array().unwrap() {
        let id = photo["id"].as_str().unwrap();
        let path = root.join(photo["image_path"].as_str().unwrap());
        let decoded = load_image(&path).unwrap_or_else(|e| panic!("{id}: {e}"));
        let quad = quads["photos"]
            .as_array()
            .unwrap()
            .iter()
            .find(|q| q["id"] == id)
            .unwrap_or_else(|| panic!("{id}: not in screen_quads.json"))["quad"]
            .clone();
        let heic = is_heif(&path);

        let mut stages: Vec<(&str, RgbImage)> = Vec::new();
        let base = if quad.is_null() {
            decoded
        } else {
            let corners: Vec<[f32; 2]> = quad
                .as_array()
                .unwrap()
                .iter()
                .map(|p| [p[0].as_f64().unwrap() as f32, p[1].as_f64().unwrap() as f32])
                .collect();
            let warped = warp_to_rect(&decoded, &[corners[0], corners[1], corners[2], corners[3]]);
            stages.push(("warped", warped.clone()));
            warped
        };
        let resized = resize(&base);
        let enhanced = enhance(&resized);
        stages.push(("resized", resized));
        stages.push(("enhanced", enhanced.clone()));

        if photo["warped"].is_null() != quad.is_null() {
            failures.push(format!(
                "{id}: the two fixtures disagree on whether it is warped"
            ));
        }
        for (stage, image) in &stages {
            let expected = &photo[*stage];
            if heic {
                let want = (
                    expected["width"].as_u64().unwrap(),
                    expected["height"].as_u64().unwrap(),
                );
                if size(image) != want {
                    failures.push(format!(
                        "{id} {stage}: size {:?}, Python {want:?}",
                        size(image)
                    ));
                }
            } else if let Some(why) = mismatch(image, expected) {
                failures.push(format!("{id} {stage}: {why}"));
            }
        }
        if heic {
            let means = channel_means(&enhanced);
            let want: Vec<f64> = photo["enhanced_mean_rgb"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_f64().unwrap())
                .collect();
            for (channel, (got, want)) in means.iter().zip(&want).enumerate() {
                if (got - want).abs() > HEIC_MEAN_TOLERANCE {
                    failures.push(format!(
                        "{id}: channel {channel}'s mean {got:.4}, Python {want:.4}, beyond \
                         {HEIC_MEAN_TOLERANCE}"
                    ));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
