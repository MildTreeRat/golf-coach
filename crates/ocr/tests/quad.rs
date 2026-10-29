//! The decode and screen-quad gate on the 13 bay photos (M29 P2).
//!
//! `tests/fixtures/screen_quads.json` is frozen Python's answer, recorded once by a scratch script
//! calling `preprocess.py`'s `load_image` and `find_screen_quad` on the photos the screen corpus
//! vectors name: each photo's decoded size and pixel digest, and its quad (or none) with the path
//! that found it. The photos are gitignored, so the gate is an ignored test that fails, rather than
//! passes, when they are missing (the M29 plan's rules and C2).
//!
//! What it holds the port to:
//!
//! - **the same size** on every photo, HEIC included;
//! - **the same pixels** on every photo but the HEIC, whose colour conversion is ffmpeg's rather
//!   than libheif's (`ocr::decode`'s module doc says how far apart);
//! - **a quad found or not found as Python finds it, by the same path**, and its corners within
//!   [`CORNER_TOLERANCE_PX`], which is zero. M29 P2 set it at 0.01 px for `minAreaRect`, whose last
//!   bits it found differing on about one contour in a thousand and took for ties between caliper
//!   positions. M29 P5 found the cause instead: OpenCV 4.10 chooses its caliper by a cross
//!   product, not the cosines older OpenCV compared (`ocr::cv::rect`). Ported so, `minAreaRect`
//!   and `boxPoints` are `cv2`'s to the bit on 30,000 random point sets, and every corner here,
//!   the HEIC's included, is Python's exactly.

use std::fs;
use std::path::{Path, PathBuf};

use ocr::decode::{is_heif, load_image};
use ocr::quad::{find_screen_quad, QuadSource};
use serde_json::Value;
use sha2::{Digest, Sha256};

/// How far a corner may land from frozen Python's, in the photo's pixels: not at all (M29 P5).
const CORNER_TOLERANCE_PX: f32 = 0.0;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

fn reference() -> Vec<Value> {
    let text = include_str!("fixtures/screen_quads.json");
    let doc: Value = serde_json::from_str(text).unwrap();
    doc["photos"].as_array().unwrap().clone()
}

fn hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

#[test]
fn the_reference_covers_the_thirteen_photos_and_every_path() {
    let photos = reference();
    assert_eq!(photos.len(), 13);
    let mut paths: Vec<&str> = photos
        .iter()
        .map(|p| p["source"]["path"].as_str().unwrap_or("none"))
        .collect();
    paths.sort_unstable();
    paths.dedup();
    // Both of find_screen_quad's answers and its refusal are exercised, so a port that always
    // took one path could not pass.
    assert_eq!(paths, vec!["corners", "none", "rotated_box"]);
    assert!(photos
        .iter()
        .any(|p| p["image_path"].as_str().unwrap().ends_with(".HEIC")));
}

#[test]
#[ignore = "needs the 13 bay photos in data/processed/ and ffmpeg 7.1 or later (GOLF_FFMPEG or PATH)"]
fn every_bay_photo_decodes_and_finds_its_screen_as_frozen_python_does() {
    let root = repo_root();
    let mut failures = Vec::new();
    for photo in reference() {
        let id = photo["id"].as_str().unwrap();
        let path = root.join(photo["image_path"].as_str().unwrap());
        let bytes = fs::read(&path).unwrap_or_else(|e| panic!("{id}: {}: {e}", path.display()));
        assert_eq!(
            hex(&bytes),
            photo["image_sha256"].as_str().unwrap(),
            "{id}: the photo on disk is not the one the reference was recorded from"
        );

        let image = load_image(&path).unwrap_or_else(|e| panic!("{id}: {e}"));
        let size = (u64::from(image.width()), u64::from(image.height()));
        let expected_size = (
            photo["width"].as_u64().unwrap(),
            photo["height"].as_u64().unwrap(),
        );
        if size != expected_size {
            failures.push(format!("{id}: decoded {size:?}, Python {expected_size:?}"));
            continue;
        }
        if !is_heif(&path) && hex(image.as_raw()) != photo["rgb_sha256"].as_str().unwrap() {
            failures.push(format!("{id}: decoded pixels differ from Python's"));
        }

        let found = find_screen_quad(&image);
        match (&photo["quad"], found) {
            (Value::Null, None) => {}
            (Value::Null, Some(q)) => failures.push(format!("{id}: found {q:?}, Python none")),
            (expected, None) => failures.push(format!("{id}: found none, Python {expected}")),
            (expected, Some(q)) => {
                let source = &photo["source"];
                let (path_name, low, close) = match q.source {
                    QuadSource::Corners { low, close, .. } => ("corners", low, close),
                    QuadSource::RotatedBox { low, close } => ("rotated_box", low, close),
                };
                if source["path"] != path_name
                    || source["low"].as_f64() != Some(low)
                    || source["close"].as_u64() != Some(close as u64)
                {
                    failures.push(format!("{id}: found by {:?}, Python by {source}", q.source));
                }
                for (corner, want) in q.corners.iter().zip(expected.as_array().unwrap()) {
                    for axis in 0..2 {
                        let want = want[axis].as_f64().unwrap() as f32;
                        if (corner[axis] - want).abs() > CORNER_TOLERANCE_PX {
                            failures.push(format!(
                                "{id}: corner {corner:?} is not within {CORNER_TOLERANCE_PX} px \
                                 of Python's {expected}"
                            ));
                        }
                    }
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
