//! The `recognize` seam's gate: this crate's reader against frozen PaddleOCR's, on the 13 bay
//! photos (M29 P6).
//!
//! Two references, because frozen PaddleOCR runs two models more than this crate does (the M29
//! plan's D5, and `src/recognizer.rs`'s doc):
//!
//! - `tests/fixtures/recognize_boxes.json` — frozen PaddleOCR with **D5's three models**:
//!   `paddle.py`'s construction with the document preprocessor turned off, on frozen Python's
//!   working image, normalised by `paddle.py`. This is the reader this crate ports, so it is the
//!   bound. Recorded once by a scratch script; its detected polygons were `det_boxes.json`'s on
//!   all 13, and it turned no crop.
//! - `spec/vectors/screen/corpus/<shot>.json`'s `input.boxes` — frozen `recognize` as it runs,
//!   **all five models**, document orientation and UVDoc's unwarping ahead of detection. A scratch
//!   run in P6 reproduced every one of those vectors' boxes exactly, on frozen Python's working
//!   image, and the document-orientation model turned no photo, so every difference between the
//!   two references is UVDoc's. This is what the parser was given when the vectors were
//!   recorded, so it measures what D5 costs, which P7's gate (D7) then judges on the whole read.
//!
//! What each is held to is in the gate's doc comment.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use ocr::models::Models;
use screen::parser::{parse_screen, ParsedShot};
use screen::profile::load_profile;
use screen::TextBox;
use serde_json::Value;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

fn fixture(name: &str) -> Value {
    json(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join(name),
    )
}

fn json(path: &Path) -> Value {
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

fn edges(found: &TextBox) -> [f64; 4] {
    [found.x, found.y, found.right(), found.bottom()]
}

/// Each text and how many boxes carry it.
fn texts(boxes: &[TextBox]) -> BTreeMap<&str, usize> {
    let mut counts = BTreeMap::new();
    for found in boxes {
        *counts.entry(found.text.as_str()).or_insert(0) += 1;
    }
    counts
}

/// The texts one side has more of than the other, with the surplus.
fn surplus<'a>(
    ours: &BTreeMap<&'a str, usize>,
    theirs: &BTreeMap<&'a str, usize>,
) -> Vec<(&'a str, usize)> {
    ours.iter()
        .filter_map(|(&text, &n)| {
            let m = theirs.get(text).copied().unwrap_or(0);
            (n > m).then(|| (text, n - m))
        })
        .collect()
}

/// Where two parses of one screen part: the tiles located, the text read under each, the values.
fn parse_differences(id: &str, ours: &ParsedShot, theirs: &ParsedShot) -> Vec<String> {
    let mut differences = Vec::new();
    if ours.fields_present != theirs.fields_present {
        differences.push(format!(
            "{id}: tiles located {:?}, frozen {:?}",
            ours.fields_present, theirs.fields_present
        ));
    }
    for (label, text) in theirs.raw_fields.iter() {
        if ours.raw_fields.get(label) != Some(text) {
            differences.push(format!(
                "{id}: {label:?} read {:?}, frozen {text:?}",
                ours.raw_fields.get(label)
            ));
        }
    }
    for (label, text) in ours.raw_fields.iter() {
        if theirs.raw_fields.get(label).is_none() {
            differences.push(format!("{id}: {label:?} read {text:?}, frozen nothing"));
        }
    }
    if ours.values != theirs.values {
        differences.push(format!(
            "{id}: values {:?}, frozen {:?}",
            ours.values, theirs.values
        ));
    }
    differences
}

/// How far a box edge may land from the three-model reading's on the twelve photos held to the
/// bound, and how many boxes may be off at all: P4's detection bound, which is where a box's
/// corners come from (`tests/det.rs`).
const CORNER_TOLERANCE_PX: f64 = 2.0;
const MOVED_BOXES_ALLOWED: usize = 1;

/// How far a confidence may land from the three-model reading's on the twelve: P5's recognition
/// bound (`tests/rec.rs`).
const SCORE_TOLERANCE: f64 = 3e-5;

/// The HEIC's guards. Its box edges are held to P4's guard (`tests/det.rs`'s
/// `HEIC_CORNER_GUARD_PX`), and the worst measured here is that guard, 3 px.
const HEIC_CORNER_GUARD_PX: f64 = 3.0;
/// The HEIC's confidences, a guard looser than P5's 0.03 because the stage is: P5 read frozen
/// Python's own boxes off ffmpeg's working image, and here the boxes are this crate's too, 16 of
/// the 47 a pixel or three off (P4), so a crop differs in its size as well as its pixels. The worst
/// measured, 0.057 ("HD GOLF"), rounded up; every text is still the three-model reading's (M29 P6).
const HEIC_SCORE_GUARD: f64 = 0.06;

/// The one photo whose values UVDoc moves: on 2026-08-10-1 the five-model reading has the
/// `Impact Position V` label as `Impact Position`, so frozen Python's parse sees two labels tie and
/// withholds the tile, and this crate's reading has the `V`, so the parse reads `CENTER` there
/// (M29 P6). Named so the gate fails if UVDoc's cost grows or this one goes. P7's gate judged it on
/// the whole read and the user kept it (2026-10-21, the M29 plan's D5 and D7): the reader stays
/// without UVDoc, and `tests/read.rs`'s `DECLARED_DIFFERENCES` names the value it moves.
const UVDOC_MOVES_VALUES: [&str; 1] = ["2026-08-10-1"];

/// The `recognize` gate on the 13 bay photos.
///
/// Each photo is decoded by this crate and cut by frozen Python's quad, as the earlier gates cut
/// it, and the working image goes through [`ocr::recognize`], each photo once.
///
/// **Against the three-model reading, the port's bound.** On the twelve photos whose working image
/// is frozen Python's: the same number of boxes, in the same order, **every text identical**,
/// every box edge within [`CORNER_TOLERANCE_PX`] with at most [`MOVED_BOXES_ALLOWED`] box off at
/// all, and every confidence within [`SCORE_TOLERANCE`]. The HEIC, whose working image comes from
/// ffmpeg's decode, gets guards ([`HEIC_CORNER_GUARD_PX`], [`HEIC_SCORE_GUARD`]). On all 13, the two readings parse alike
/// (`crates/screen`'s parser, the vector's profile): the same tiles located, the same text under
/// each, the same values.
///
/// **Against the five-model reading, what UVDoc costs.** The boxes are not expected to match, and
/// their differences are printed for the plan's P6 finding. The parses must locate the same tiles
/// on all 13, and make the same values of them on all but [`UVDOC_MOVES_VALUES`]; the text under a
/// tile differs on several photos without moving a value (a degree sign or a unit read or not),
/// and is printed.
#[test]
#[ignore = "needs the 13 bay photos in data/processed/, ffmpeg 7.1 or later (GOLF_FFMPEG or PATH) and the ONNX models in data/models/ocr/"]
fn every_bay_photo_is_recognized_as_frozen_paddleocr_recognizes_it() {
    let root = repo_root();
    let mut models = Models::open(&root.join(ocr::DEFAULT_DIR)).unwrap();
    let det = fixture("det_boxes.json");
    let three = fixture("recognize_boxes.json");
    let (photos, three) = (
        det["photos"].as_array().unwrap(),
        three["photos"].as_array().unwrap(),
    );
    assert_eq!(photos.len(), 13);
    assert_eq!(photos.len(), three.len());
    let mut failures = Vec::new();
    let (mut heif_photos, mut moved_boxes) = (0, 0);
    let (mut worst_edge, mut worst_score, mut worst_heif_edge, mut worst_heif_score) =
        (0f64, 0f64, 0f64, 0f64);
    for (photo, want) in photos.iter().zip(three) {
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
        let ours = ocr::recognize(&mut models, &working).unwrap();

        // The three-model reading.
        let reference: Vec<TextBox> = serde_json::from_value(want["boxes"].clone()).unwrap();
        if ours.len() != reference.len() {
            failures.push(format!(
                "{id}: {} boxes, the three-model reading {}",
                ours.len(),
                reference.len()
            ));
        }
        let (edge_bound, score_bound) = if heif {
            (HEIC_CORNER_GUARD_PX, HEIC_SCORE_GUARD)
        } else {
            (CORNER_TOLERANCE_PX, SCORE_TOLERANCE)
        };
        for (i, (got, expected)) in ours.iter().zip(&reference).enumerate() {
            if got.text != expected.text {
                failures.push(format!(
                    "{id} box {i}: read {:?}, the three-model reading {:?}",
                    got.text, expected.text
                ));
            }
            let edge = edges(got)
                .iter()
                .zip(edges(expected))
                .map(|(a, b)| (a - b).abs())
                .fold(0f64, f64::max);
            let score = (got.confidence - expected.confidence).abs();
            if heif {
                worst_heif_edge = worst_heif_edge.max(edge);
                worst_heif_score = worst_heif_score.max(score);
            } else {
                worst_edge = worst_edge.max(edge);
                worst_score = worst_score.max(score);
                moved_boxes += usize::from(edge > 0.0);
            }
            if edge > edge_bound || score > score_bound {
                failures.push(format!(
                    "{id} box {i} {:?}: edges {:?} and confidence {}, the three-model reading's \
                     {:?} and {}",
                    got.text,
                    edges(got),
                    got.confidence,
                    edges(expected),
                    expected.confidence
                ));
            }
        }

        let vector = json(
            &root
                .join("spec")
                .join("vectors")
                .join("screen")
                .join("corpus")
                .join(format!("{id}.json")),
        );
        let input = &vector["input"];
        assert_eq!(
            input["image_path"].as_str(),
            photo["image_path"].as_str(),
            "{id}: the vector is another photo's"
        );
        let profile = load_profile(input["device"].as_str().unwrap()).unwrap();
        let parsed = parse_screen(&ours, profile);
        failures.extend(parse_differences(
            id,
            &parsed,
            &parse_screen(&reference, profile),
        ));

        // The five-model reading.
        let five: Vec<TextBox> = serde_json::from_value(input["boxes"].clone()).unwrap();
        let uvdoc = parse_differences(id, &parsed, &parse_screen(&five, profile));
        let named = UVDOC_MOVES_VALUES.contains(&id);
        let values_moved = uvdoc.iter().any(|d| d.contains(": values "));
        if values_moved != named {
            failures.push(format!(
                "{id}: UVDoc {} the values, and UVDOC_MOVES_VALUES says it {}",
                if values_moved {
                    "moves"
                } else {
                    "does not move"
                },
                if named { "does" } else { "does not" }
            ));
        }
        if let Some(located) = uvdoc.iter().find(|d| d.contains(": tiles located ")) {
            failures.push(located.clone());
        }
        let (ours_texts, five_texts) = (texts(&ours), texts(&five));
        eprintln!(
            "{id}: {} boxes, five-model {}; only here {:?}; only five-model {:?}",
            ours.len(),
            five.len(),
            surplus(&ours_texts, &five_texts),
            surplus(&five_texts, &ours_texts)
        );
        for difference in &uvdoc {
            eprintln!("  UVDoc: {difference}");
        }
    }
    eprintln!(
        "against the three-model reading: worst edge {worst_edge} px over {moved_boxes} boxes \
         moved and worst confidence {worst_score:e} on the twelve; worst edge {worst_heif_edge} px \
         and confidence {worst_heif_score:e} on the HEIC"
    );
    assert_eq!(heif_photos, 1, "the bay photos hold one HEIC");
    if moved_boxes > MOVED_BOXES_ALLOWED {
        failures.push(format!(
            "{moved_boxes} boxes moved on the twelve, more than {MOVED_BOXES_ALLOWED}"
        ));
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
