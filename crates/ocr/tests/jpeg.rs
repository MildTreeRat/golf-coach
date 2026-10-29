//! The JPEG decoder against libjpeg-turbo on small committed photos (M29 P2).
//!
//! `tests/fixtures/jpeg/` holds photos written by libjpeg-turbo (`cv2.imencode`) and, for the EXIF
//! orientations, by Pillow, with `expected.json` recording what `cv2.imread(path, IMREAD_COLOR)`
//! decodes from each: its size and the SHA-256 of its pixels as RGB. A scratch script recorded it
//! once. They cover what the ten bay JPEGs (all baseline 4:4:4, orientation 1, no restart markers,
//! gated by `tests/quad.rs`) do not: 4:2:2, 4:2:0, 4:4:0 and 4:1:1 sampling, chroma too narrow for
//! the fancy upsampling filters, a restart interval, quality 100's large coefficients, grey, RGB
//! named by component id with no JFIF marker, and all eight EXIF orientations. They are committed,
//! so this gate always runs.

use std::fs;
use std::path::{Path, PathBuf};

use ocr::decode::{load_image, DecodeError};
use serde_json::Value;
use sha2::{Digest, Sha256};

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/jpeg")
}

fn expected() -> Vec<Value> {
    let text = fs::read_to_string(fixtures().join("expected.json")).unwrap();
    let doc: Value = serde_json::from_str(&text).unwrap();
    doc["photos"].as_array().unwrap().clone()
}

fn hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

#[test]
fn every_photo_decodes_to_libjpeg_turbos_pixels_or_is_refused_by_name() {
    let photos = expected();
    // A gate that found no photos would pass on nothing.
    assert!(photos.len() >= 20, "{} photos", photos.len());
    let mut failures = Vec::new();
    for photo in &photos {
        let name = photo["file"].as_str().unwrap();
        let path = fixtures().join(name);
        assert_eq!(
            hex(&fs::read(&path).unwrap()),
            photo["sha256"].as_str().unwrap(),
            "{name}: not the photo the reference was recorded from"
        );
        let decoded = load_image(&path);
        if let Some(kind) = photo["refused"].as_str() {
            match decoded {
                Err(DecodeError::UnsupportedJpeg { kind: said, .. }) if said.contains(kind) => {}
                other => failures.push(format!("{name}: expected a {kind} refusal, got {other:?}")),
            }
            continue;
        }
        let image = match decoded {
            Ok(image) => image,
            Err(error) => {
                failures.push(format!("{name}: {error}"));
                continue;
            }
        };
        let size = (u64::from(image.width()), u64::from(image.height()));
        let want = (
            photo["width"].as_u64().unwrap(),
            photo["height"].as_u64().unwrap(),
        );
        if size != want {
            failures.push(format!("{name}: decoded {size:?}, cv2 {want:?}"));
        } else if hex(image.as_raw()) != photo["rgb_sha256"].as_str().unwrap() {
            failures.push(format!("{name}: pixels differ from cv2's"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn the_fixtures_cover_every_sampling_and_orientation() {
    let names: Vec<String> = expected()
        .iter()
        .map(|p| p["file"].as_str().unwrap().to_string())
        .collect();
    for required in [
        "s444.jpg",
        "s422.jpg",
        "s420.jpg",
        "s440.jpg",
        "s411.jpg",
        "s420_narrow.jpg",
        "s422_narrow.jpg",
        "s420_restart.jpg",
        "gray.jpg",
        "rgb_ids.jpg",
        "progressive.jpg",
        "cmyk.jpg",
    ] {
        assert!(names.iter().any(|n| n == required), "{required} missing");
    }
    for orientation in 1..=8 {
        let name = format!("orient{orientation}.jpg");
        assert!(names.contains(&name), "{name} missing");
    }
}

#[test]
fn a_cut_short_jpeg_is_refused_rather_than_padded_grey() {
    // libjpeg-turbo pads missing entropy data with zero bits and `cv2.imread` returns the photo
    // with its lower blocks grey; here that is a refusal, never a partial photo.
    let whole = fs::read(fixtures().join("s420.jpg")).unwrap();
    let dir = tempfile::tempdir().unwrap();
    for keep in [whole.len() - 2, whole.len() / 2, 700] {
        let path = dir.path().join(format!("cut{keep}.jpg"));
        fs::write(&path, &whole[..keep]).unwrap();
        match load_image(&path) {
            // Losing only the end-of-image marker loses no pixel, as in libjpeg.
            Ok(image) if keep == whole.len() - 2 => assert_eq!(image.dimensions(), (45, 29)),
            Err(DecodeError::Undecodable { reason, .. }) => {
                assert!(reason.contains("cut short"), "{keep}: {reason}")
            }
            other => panic!("{keep} bytes: {other:?}"),
        }
    }
}
