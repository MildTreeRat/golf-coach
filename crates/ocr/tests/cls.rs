//! The textline-orientation gate: the classifier's input, its answers and the half turn, held to
//! PaddleX's (M29 P6).
//!
//! One reference, `tests/fixtures/cls_reference.json`, recorded once by a scratch script calling
//! frozen Python's own libraries (the live pipeline's `textline_orientation_model`, its
//! preprocessors one by one, and `rotate_image`):
//!
//! - `scene` — answers on inputs small enough to commit, so the always-run suite holds the port
//!   without the photos or the models: PaddleX's textline input tensors for the synthetic scene's
//!   60 crops (`rec_reference.json`'s boxes, cropped by PaddleX) in the pipeline's batches, with a
//!   61st crop cut straight from the scene at exactly the model's 160x80, which `Resize` leaves
//!   alone; and `rotate_image(crop, 180)` on each of the 60.
//! - `photos` — frozen Paddle's textline model on the 13 bay photos: on PaddleX's crops of frozen
//!   Python's boxes (`det_boxes.json`'s `sorted`) on frozen Python's working image, in the
//!   pipeline's batches, each batch's input-tensor digest and each crop's class and two
//!   probabilities; and the same for every crop turned upside down (a true flip), because **no
//!   crop of the 13 is upside down**, so without them the photos would never show the model the
//!   class it exists to find.
//!
//! What the photo gate holds the port to is in its own doc comment.

use std::fs;
use std::path::{Path, PathBuf};

use image::{imageops, RgbImage};
use ocr::cls::{classify, cls_input, turn_half, Orientation};
use ocr::models::{Manifest, Models};
use ocr::rec::crop;
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

fn u32_of(value: &Value) -> u32 {
    value.as_u64().unwrap() as u32
}

fn box_of(value: &Value) -> [[i32; 2]; 4] {
    std::array::from_fn(|i| {
        [
            value[i][0].as_i64().unwrap() as i32,
            value[i][1].as_i64().unwrap() as i32,
        ]
    })
}

/// The scene's 60 crops, cut by this crate from `rec_reference.json`'s boxes (`tests/rec.rs`
/// holds each to PaddleX's pixel for pixel).
fn scene_crops(scene: &RgbImage) -> Vec<RgbImage> {
    fixture("rec_reference.json")["crops"]
        .as_array()
        .unwrap()
        .iter()
        .map(|case| crop(scene, &box_of(&case["points"])))
        .collect()
}

#[test]
fn the_scenes_crops_become_paddlexs_textline_tensors_bit_for_bit() {
    let scene = scene();
    let reference = fixture("cls_reference.json")["scene"].clone();
    let mut crops = scene_crops(&scene);
    let exact = &reference["exact"];
    let at = [&exact["x"], &exact["y"], &exact["width"], &exact["height"]].map(u32_of);
    crops.push(imageops::crop_imm(&scene, at[0], at[1], at[2], at[3]).to_image());
    assert_eq!(crops.last().unwrap().dimensions(), (160, 80));
    let textline = &Manifest::committed().models.textline;
    let batches = reference["batches"].as_array().unwrap();
    assert_eq!(batches.len(), crops.len().div_ceil(textline.batch_size));
    for (k, (members, case)) in crops.chunks(textline.batch_size).zip(batches).enumerate() {
        let members: Vec<&RgbImage> = members.iter().collect();
        let input = cls_input(&members, textline);
        let shape: Vec<usize> = case["shape"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap() as usize)
            .collect();
        assert_eq!(input.shape, shape, "batch {k}");
        assert_eq!(
            f32_digest(&input.data),
            case["sha256"].as_str().unwrap(),
            "batch {k}"
        );
    }
}

#[test]
fn the_half_turn_is_paddlexs_rotate_image_on_every_scene_crop() {
    let scene = scene();
    let turned = fixture("cls_reference.json")["scene"]["turned"].clone();
    let turned = turned.as_array().unwrap();
    let crops = scene_crops(&scene);
    assert_eq!(crops.len(), turned.len());
    for (i, (cut, case)) in crops.iter().zip(turned).enumerate() {
        let half = turn_half(cut);
        assert_eq!(
            half.dimensions(),
            (u32_of(&case["width"]), u32_of(&case["height"])),
            "crop {i}"
        );
        assert_eq!(
            hex(half.as_raw()),
            case["rgb_sha256"].as_str().unwrap(),
            "crop {i}"
        );
    }
}

/// How far a probability may land from frozen Paddle's on a crop of the twelve photos held to the
/// bound, upright or flipped: the worst measured, 1.2e-5 (a flipped crop), rounded up. Upright,
/// the worst was 3.6e-7 (M29 P6).
const PROBABILITY_TOLERANCE: f32 = 2e-5;

/// How far a probability may land from frozen Paddle's on the HEIC's upright crops. A guard, not
/// the bound: the worst measured, 0.133, rounded up (M29 P6), and the gate's doc says why the HEIC
/// is apart.
const HEIC_PROBABILITY_GUARD: f32 = 0.14;

/// How many of the HEIC's flipped crops may take the other class from frozen Paddle's: the six
/// measured, each where Paddle's own two probabilities were within 0.22 of each other (M29 P6).
const HEIC_FLIPPED_CLASSES_MOVED: usize = 6;

/// The textline gate on the 13 bay photos.
///
/// Each photo is decoded by this crate and cut by frozen Python's quad, and each of frozen
/// Python's own boxes (`det_boxes.json`'s `sorted`) cropped from that working image, as
/// `tests/rec.rs` does, so the stage under test is the classifier alone, fed the crops PaddleX was
/// fed. Every crop is classified as it is, and again turned upside down by a true flip. On the
/// twelve photos whose working image is frozen Python's, held to **the bound**:
///
/// - every batch's **input tensor bit for bit**, upright and flipped;
/// - **every class Paddle's**, upright and flipped;
/// - both probabilities within [`PROBABILITY_TOLERANCE`].
///
/// Paddle reads every upright crop of the 13 as upright, and fewer than half of the flipped ones
/// as upside down: short numbers and symmetric text read either way up, and the model's
/// probabilities on a flipped crop come as close as 0.50008 to 0.49992 (2026-08-23-3, crop 31).
/// That is why the flipped crops are compared with Paddle's classes and not with "upside down".
///
/// **The HEIC (2026-08-23-1) gets a guard instead**, as the earlier gates give it one. Its working
/// image comes from ffmpeg's decode, up to two levels darker than libheif's (the M29 plan's P2 and
/// P3 findings), so none of its crops is PaddleX's. Upright, every class is still Paddle's, with
/// the probabilities within [`HEIC_PROBABILITY_GUARD`]. Flipped, the model is unsure of many crops,
/// and the different pixels tip six of them to the other class, each one where Paddle's own two
/// probabilities were within 0.22 of each other; [`HEIC_FLIPPED_CLASSES_MOVED`] holds the count.
/// No bay crop is upside down, so the class the pipeline acts on is the upright one.
#[test]
#[ignore = "needs the 13 bay photos in data/processed/, ffmpeg 7.1 or later (GOLF_FFMPEG or PATH) and the ONNX models in data/models/ocr/"]
fn every_bay_photos_crops_are_classified_as_frozen_paddle_classifies_them() {
    let root = repo_root();
    let mut models = Models::open(&root.join(ocr::DEFAULT_DIR)).unwrap();
    let textline = &Manifest::committed().models.textline;
    let det = fixture("det_boxes.json");
    let reference = fixture("cls_reference.json");
    let (det, photos) = (
        det["photos"].as_array().unwrap(),
        reference["photos"].as_array().unwrap(),
    );
    assert_eq!(det.len(), photos.len());
    let mut failures = Vec::new();
    let mut heif_photos = 0;
    let (mut crops_seen, mut heic_flips_moved) = (0, 0);
    // The worst probability difference: the twelve upright and flipped, then the HEIC's.
    let mut worst = [0f32; 4];
    for (photo, want) in det.iter().zip(photos) {
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
        let upright: Vec<RgbImage> = photo["sorted"]
            .as_array()
            .unwrap()
            .iter()
            .map(|points| crop(&working, &box_of(points)))
            .collect();
        let flipped: Vec<RgbImage> = upright.iter().map(imageops::rotate180).collect();
        for (crops, classes, batches, what) in [
            (&upright, "classes", "batches", "upright"),
            (&flipped, "flipped", "flipped_batches", "flipped"),
        ] {
            let want_classes = want[classes].as_array().unwrap();
            assert_eq!(crops.len(), want_classes.len(), "{id}");
            crops_seen += crops.len();
            if !heif {
                for (k, (members, case)) in crops
                    .chunks(textline.batch_size)
                    .zip(want[batches].as_array().unwrap())
                    .enumerate()
                {
                    let members: Vec<&RgbImage> = members.iter().collect();
                    let digest = f32_digest(&cls_input(&members, textline).data);
                    if digest != case["sha256"].as_str().unwrap() {
                        failures.push(format!(
                            "{id} {what} batch {k}: the input tensor differs from PaddleX's"
                        ));
                    }
                }
            }
            let got: Vec<Orientation> = classify(&mut models, crops).unwrap();
            let heic_flipped = heif && what == "flipped";
            for (i, (orientation, case)) in got.iter().zip(want_classes).enumerate() {
                let paddle = case["class"].as_u64().unwrap() as usize;
                let paddle_p = [
                    case["p0"].as_f64().unwrap() as f32,
                    case["p1"].as_f64().unwrap() as f32,
                ];
                if orientation.class != paddle {
                    if heic_flipped {
                        heic_flips_moved += 1;
                        eprintln!(
                            "{id} flipped crop {i}: class {} at {:?}, Paddle's {paddle} at \
                             {paddle_p:?}",
                            orientation.class, orientation.probabilities
                        );
                    } else {
                        failures.push(format!(
                            "{id} {what} crop {i}: class {}, Paddle's {paddle}",
                            orientation.class
                        ));
                    }
                }
                let off = (0..2)
                    .map(|c| (orientation.probabilities[c] - paddle_p[c]).abs())
                    .fold(0f32, f32::max);
                let slot = usize::from(heif) * 2 + usize::from(what == "flipped");
                worst[slot] = worst[slot].max(off);
                let bound = if !heif {
                    PROBABILITY_TOLERANCE
                } else if heic_flipped {
                    // Held by the count of classes moved, below.
                    f32::INFINITY
                } else {
                    HEIC_PROBABILITY_GUARD
                };
                if off > bound {
                    failures.push(format!(
                        "{id} {what} crop {i}: probabilities {:?}, Paddle's {paddle_p:?}",
                        orientation.probabilities
                    ));
                }
            }
        }
    }
    eprintln!(
        "{crops_seen} crops classified; worst probability difference upright {:e} and flipped \
         {:e} on the twelve, upright {:e} and flipped {:e} on the HEIC, whose flipped crops moved \
         {heic_flips_moved} classes",
        worst[0], worst[1], worst[2], worst[3]
    );
    assert_eq!(heif_photos, 1, "the bay photos hold one HEIC");
    if heic_flips_moved > HEIC_FLIPPED_CLASSES_MOVED {
        failures.push(format!(
            "the HEIC's flipped crops moved {heic_flips_moved} classes, more than \
             {HEIC_FLIPPED_CLASSES_MOVED}"
        ));
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
