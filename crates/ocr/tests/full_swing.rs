//! The Full Swing photo gate (M42 P9): the shipping reader end to end on the six photos of Full
//! Swing's grid screen, held to what each photo prints, read by eye.
//!
//! **No Python twin, and no vector behind it.** Frozen Python has no `full_swing` profile, and the
//! oracle is the M42 plan's truth table: the twelve stored values of each photo, read from the
//! photos in the planning session with the lit letter of each pair confirmed on a crop. It is
//! written out below in the table's own spelling (a magnitude and its lit letter, an apex in feet)
//! and turned into `ShotData`'s conventions in one place, [`Photo::truth`].
//!
//! **What it holds**: no value is **wrong**. Each stored field is *read* (exactly the truth, as an
//! `f64`), *withheld* (`None`: the parser refused to store it, and said why in a warning) or
//! *wrong* (anything else), and only a wrong value fails the gate, because a withheld one is what
//! ADR-010 §2 asks of a reader that cannot be sure, and a wrong one is the thing this reader exists
//! not to store. How many are read is printed, with each photo's `needs_review`, warnings and sign
//! letters, for the plan's findings; the plan's target is 66 of the 72, and missing it is recorded
//! there rather than failed here.
//!
//! The photos are checked against the plan's digests first, so the table is only ever held to the
//! photos it was read from. `2026-10-07-1.jpg` is M41's course-play photo, the same grid screen.

use std::fs;
use std::path::{Path, PathBuf};

use contracts::shot::ShotData;
use ocr::models::Models;
use screen::profile::load_profile;
use screen::ScreenInput;
use sha2::{Digest, Sha256};

/// The threshold every read takes: `config.py`'s `ocr_min_confidence`, `golf-ocr read`'s default.
const MIN_CONFIDENCE: f64 = 0.6;

/// One photo of the truth table, as printed on the screen.
struct Photo {
    file: &'static str,
    sha256: &'static str,
    carry: f64,
    /// Printed twice, header and data row, the same on every photo (the plan's call 7).
    total: f64,
    club_speed: f64,
    ball_speed: f64,
    path: f64,
    face: f64,
    /// The magnitude and the lit letter, `L` or `R`.
    ball_direction: (f64, char),
    launch: f64,
    /// In feet, as printed (the plan's call 8).
    apex_ft: f64,
    smash: f64,
    backspin: f64,
    /// The magnitude and the lit letter, `D` or `F`.
    sidespin: (f64, char),
}

impl Photo {
    /// The twelve stored values in `ShotData`'s conventions: `launch_direction` + = right (`R`
    /// lit), `sidespin` + = fade (`F` lit) and − = draw (`D` lit), both by the plan's call 2, and
    /// `apex_height` the printed feet divided by 3.0, the parser's own float (P7's finding: `85.2 /
    /// 3.0` is not `85.2 * (1.0 / 3.0)`), never typed as a decimal.
    fn truth(&self) -> [(&'static str, f64); 12] {
        let signed = |(magnitude, letter): (f64, char), minus: char, plus: char| match letter {
            l if l == minus => -magnitude,
            l if l == plus => magnitude,
            other => panic!("{}: {other} is neither {minus} nor {plus}", self.file),
        };
        [
            ("carry_distance", self.carry),
            ("total_distance", self.total),
            ("club_head_speed", self.club_speed),
            ("ball_speed", self.ball_speed),
            ("club_path", self.path),
            ("club_face_angle", self.face),
            ("launch_direction", signed(self.ball_direction, 'L', 'R')),
            ("launch_angle", self.launch),
            ("apex_height", self.apex_ft / 3.0),
            ("smash_factor", self.smash),
            ("backspin", self.backspin),
            ("sidespin", signed(self.sidespin, 'D', 'F')),
        ]
    }
}

/// The M42 plan's truth table and "The photos" digests.
const PHOTOS: [Photo; 6] = [
    Photo {
        file: "IMG_2889.HEIC",
        sha256: "d47e3e9607211a1350c09416d96f2f7e64a3d237a29d6f5a9ba57890f5257d8e",
        carry: 131.0,
        total: 134.0,
        club_speed: 95.0,
        ball_speed: 102.5,
        path: -2.0,
        face: 13.2,
        ball_direction: (10.1, 'R'),
        launch: 24.5,
        apex_ft: 85.2,
        smash: 1.08,
        backspin: 9899.0,
        sidespin: (4199.0, 'D'),
    },
    Photo {
        file: "IMG_2891.HEIC",
        sha256: "73be98e34cee2eafa22ad8d693c9c1c4c4efac585616f452d6cd5ceda58a15c0",
        carry: 162.0,
        total: 203.0,
        club_speed: 86.9,
        ball_speed: 112.8,
        path: -1.7,
        face: -0.6,
        ball_direction: (0.8, 'L'),
        launch: 12.1,
        apex_ft: 39.9,
        smash: 1.30,
        backspin: 4236.0,
        sidespin: (464.0, 'D'),
    },
    Photo {
        file: "IMG_2892.HEIC",
        sha256: "1c4b731de2871303b6387a843e65d893a0c37d059b7ff0cca321eea2f1fc6d04",
        carry: 80.0,
        total: 82.0,
        club_speed: 91.6,
        ball_speed: 77.4,
        path: -4.5,
        face: 6.9,
        ball_direction: (4.6, 'R'),
        launch: 34.6,
        apex_ft: 71.4,
        smash: 0.85,
        backspin: 8855.0,
        sidespin: (2032.0, 'D'),
    },
    Photo {
        file: "IMG_2893.HEIC",
        sha256: "204872ef4b6ccd2c7c2730ca14283b43e4a3c5f4e57164373d52c19fb72d9ca8",
        carry: 184.0,
        total: 224.0,
        club_speed: 107.6,
        ball_speed: 134.0,
        path: -1.7,
        face: 3.2,
        ball_direction: (2.2, 'R'),
        launch: 5.5,
        apex_ft: 15.5,
        smash: 1.25,
        backspin: 2284.0,
        sidespin: (638.0, 'D'),
    },
    Photo {
        file: "IMG_2894.HEIC",
        sha256: "227d9985b337a70248f0e268eea4e73fc3e5860bc0b2418afe6082a4b19c399b",
        carry: 105.0,
        total: 107.0,
        club_speed: 81.3,
        ball_speed: 87.2,
        path: -1.7,
        face: 7.5,
        ball_direction: (5.7, 'R'),
        launch: 31.4,
        apex_ft: 80.6,
        smash: 1.07,
        backspin: 9985.0,
        sidespin: (2209.0, 'D'),
    },
    Photo {
        file: "2026-10-07-1.jpg",
        sha256: "96241978f183d70cfbd633802b395362a68ea0b3905324fbbc9a40e18c51b0e1",
        carry: 279.0,
        total: 289.0,
        club_speed: 111.3,
        ball_speed: 158.6,
        path: 1.6,
        face: 4.0,
        ball_direction: (3.5, 'R'),
        launch: 19.0,
        apex_ft: 142.2,
        smash: 1.43,
        backspin: 2521.0,
        sidespin: (336.0, 'F'),
    },
];

/// The pairs of sign letters a grid photo prints, each tile lighting one (the plan's finding 4).
const SIGN_LETTERS: [&str; 4] = ["L", "R", "D", "F"];

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

/// The value a shot stored under one of the truth table's keys.
fn stored(shot: &ShotData, field: &str) -> Option<f64> {
    match field {
        "carry_distance" => shot.carry_distance,
        "total_distance" => shot.total_distance,
        "club_head_speed" => shot.club_head_speed,
        "ball_speed" => shot.ball_speed,
        "club_path" => shot.club_path,
        "club_face_angle" => shot.club_face_angle,
        "launch_direction" => shot.launch_direction,
        "launch_angle" => shot.launch_angle,
        "apex_height" => shot.apex_height,
        "smash_factor" => shot.smash_factor,
        "backspin" => shot.backspin,
        "sidespin" => shot.sidespin,
        other => panic!("{other} is not a truth-table field"),
    }
}

/// The gate on the six grid photos.
///
/// Read/withheld per photo and field is printed as a table, followed by each photo's turn, label
/// ratio, `needs_review`, warnings and the sign letters the OCR returned (with their glyph colour
/// and whether [`screen::parser::is_lit`] calls them lit), because which letters come back is
/// what decides a sign (the plan's call 5) and it differs from photo to photo.
#[test]
#[ignore = "needs the six Full Swing photos in data/raw/shot_screens/, ffmpeg and the ONNX models in data/models/ocr/"]
fn every_grid_photo_reads_as_it_prints_or_withholds() {
    let root = repo_root();
    let mut models = Models::open(&root.join(ocr::DEFAULT_DIR)).unwrap();
    let profile = load_profile("full_swing").unwrap();
    let mut wrong = Vec::new();
    let mut rows = Vec::new();
    let mut details = Vec::new();
    let mut read = 0;
    let mut total = 0;
    for photo in &PHOTOS {
        let relative = format!("data/raw/shot_screens/{}", photo.file);
        let path = root.join(&relative);
        let bytes = fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let digest = format!("{:x}", Sha256::digest(&bytes));
        assert_eq!(
            digest,
            photo.sha256,
            "{} is not the photo the truth table was read from",
            path.display()
        );

        let prepared = ocr::read_photo(&mut models, &path, profile)
            .unwrap_or_else(|e| panic!("{}: {e}", photo.file));
        let input = ScreenInput {
            device: profile.device.clone(),
            boxes: prepared.boxes.clone(),
            notes: prepared.notes.clone(),
            shot_id: format!("full-swing-gate-{}", &digest[..12]),
            session_id: "full-swing-gate".to_string(),
            timestamp: ocr::screen_timestamp(&path).unwrap(),
            image_sha256: Some(digest),
            image_path: Some(relative),
            min_confidence: MIN_CONFIDENCE,
        };
        let shot = screen::read(&input).unwrap();

        let mut cells = Vec::new();
        for (field, want) in photo.truth() {
            total += 1;
            let got = shot.as_ref().and_then(|shot| stored(shot, field));
            cells.push(match got {
                Some(got) if got.to_bits() == want.to_bits() => {
                    read += 1;
                    "read"
                }
                None => "withheld",
                Some(got) => {
                    wrong.push(format!(
                        "{}: {field} = {got:?}, the photo prints {want:?}",
                        photo.file
                    ));
                    "WRONG"
                }
            });
        }
        rows.push(format!("{:<17} {}", photo.file, cells.join(" ")));

        let letters: Vec<String> = prepared
            .boxes
            .iter()
            .filter(|b| SIGN_LETTERS.contains(&b.text.trim().to_uppercase().as_str()))
            .map(|b| match b.rgb {
                Some(rgb) => format!(
                    "{} ({:.1}, {:.1}, {:.1}) {}",
                    b.text.trim(),
                    rgb[0],
                    rgb[1],
                    rgb[2],
                    if screen::parser::is_lit(rgb) {
                        "lit"
                    } else {
                        "unlit"
                    }
                ),
                None => format!("{} (no rgb)", b.text.trim()),
            })
            .collect();
        let (needs_review, confidence, warnings) =
            match shot.as_ref().and_then(|s| s.provenance.as_ref()) {
                Some(p) => (
                    p.needs_review.to_string(),
                    format!("{:.3}", p.parse_confidence),
                    p.warnings.clone(),
                ),
                None => (
                    "-".to_string(),
                    "-".to_string(),
                    vec!["nothing read".to_string()],
                ),
            };
        details.push(format!(
            "{}: turned {}, {} boxes, label ratio {}, needs_review {needs_review}, confidence \
             {confidence}\n  letters: {}\n  warnings: {}",
            photo.file,
            prepared.rotation.degrees(),
            prepared.boxes.len(),
            prepared.label_ratio,
            if letters.is_empty() {
                "none".to_string()
            } else {
                letters.join("; ")
            },
            if warnings.is_empty() {
                "none".to_string()
            } else {
                warnings.join(" | ")
            },
        ));
    }

    let header: Vec<&str> = PHOTOS[0].truth().iter().map(|(field, _)| *field).collect();
    eprintln!("columns: {}", header.join(" "));
    for row in &rows {
        eprintln!("{row}");
    }
    for detail in &details {
        eprintln!("{detail}");
    }
    eprintln!("coverage: {read} of {total} read, {} wrong", wrong.len());
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
