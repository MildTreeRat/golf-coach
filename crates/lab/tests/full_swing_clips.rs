//! The Full Swing clip gate (M42 P14): the grid of numbers read off one frame of each grid Live
//! Photo's motion part, through `golf_lab::shot::read_clip_grid` with the shipping reader, held to
//! what the still prints, read by eye.
//!
//! **No Python twin, and no vector behind it**, as `crates/ocr/tests/full_swing.rs` (P9's photo
//! gate) has none. This copies that gate's form and its truth table: the five grid photos with a
//! motion part (`2026-10-07-1.jpg` is a plain JPEG with none), the twelve stored values of each in
//! the table's own spelling, turned into `ShotData`'s conventions in one place, [`Clip::truth`].
//! The clip and its still are one Live Photo of one screen, so the still's truth is the clip's,
//! for every frame that shows this shot's numbers rather than the last one's (call 14).
//!
//! **What it holds**: no value is **wrong** (the M42 plan's decision 10: ADR-010 §2, the rule the
//! stills are held to). Each stored field is *read* (exactly the truth, by `to_bits`), *withheld*
//! (`None`, with the parser's warning) or *wrong* (anything else), and a wrong value fails the
//! gate. Coverage is held too, at no fewer values read than [`READ_AT_LEAST`], P14's measurement
//! through [`GRID_FRAME`]: a reader that starts withholding what it read is a change someone should
//! see. Each clip's frame, warnings, `needs_review` and sign letters are printed for the findings,
//! with what the replay gave: the reader runs here too, and its accuracy is not judged (decision 3).
//!
//! The clips are checked against the plan's digests first, so the table is only ever held to the
//! clips it was read from. Each read takes minutes (one OCR read of a 1744 × 1308 frame per clip),
//! so it is ignored by default and runs alone, in release, into `target/m42`.

use std::path::{Path, PathBuf};
use std::time::Instant;

use contracts::shot::ShotData;
use golf_lab::ffmpeg::Ffmpeg;
use golf_lab::shot::{
    read_clip_grid, GridFrame, NewImport, OcrReader, PhotoReader, Prepared, ShotOptions, GRID_FRAME,
};
use screen::profile::{load_profile, DeviceProfile};
use storage::shot_store::hash_image;

/// The fewest values the gate must read through [`GRID_FRAME`], of the 60: every one, as P14
/// measured (the plan's P14 findings). The middle frame read 59, its one gap `IMG_2893`'s sidespin
/// withheld because neither `D` nor `F` came back as a box, and neither rule read a wrong value.
const READ_AT_LEAST: usize = 60;

/// One grid Live Photo's clip and what its screen prints.
struct Clip {
    file: &'static str,
    sha256: &'static str,
    carry: f64,
    /// Printed twice, header and data row (the plan's call 7).
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

impl Clip {
    /// The twelve stored values in `ShotData`'s conventions, as P9's `Photo::truth` turns them:
    /// `launch_direction` + = right, `sidespin` + = fade, `apex_height` the printed feet / 3.0.
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

/// The M42 plan's truth table, its five Live Photos, with "The photos" digests of their `.MOV`s.
const CLIPS: [Clip; 5] = [
    Clip {
        file: "IMG_2889.MOV",
        sha256: "ed2112dbf7f85320d04a1b3d2681853ba5329c989669b8dc1e6095a705c6a2b7",
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
    Clip {
        file: "IMG_2891.MOV",
        sha256: "0c86193a1af6cf592ced2aa23cca34fe67f19cd1b8eada2601c7019758790815",
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
    Clip {
        file: "IMG_2892.MOV",
        sha256: "0fa5aa0d2d799d87800693a52d5454ed66ccdf08c293967edc59be9425b13513",
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
    Clip {
        file: "IMG_2893.MOV",
        sha256: "bf97341db7521dd3b8b50836cc4e90120c192d437b03f18dc6b5903c17b8520a",
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
    Clip {
        file: "IMG_2894.MOV",
        sha256: "6ed8385c1891a3ba1c5c1db0fd129a77eaf283be88db3395b03fca8fc3943d31",
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
];

/// The pairs of sign letters the grid prints, each tile lighting one (the plan's finding 4).
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

/// The shipping reader, keeping the last read's boxes so the gate can print the sign letters.
struct Keeping {
    reader: OcrReader,
    last: Option<Prepared>,
}

impl PhotoReader for Keeping {
    fn ready(&mut self) -> Result<(), String> {
        self.reader.ready()
    }

    fn prepare(&mut self, photo: &Path, profile: &DeviceProfile) -> Result<Prepared, String> {
        let prepared = self.reader.prepare(photo, profile)?;
        self.last = Some(prepared.clone());
        Ok(prepared)
    }
}

/// What one rule made of the five clips.
struct Run {
    read: usize,
    total: usize,
    wrong: Vec<String>,
}

/// Every clip through [`read_clip_grid`] by `rule`, printed, and counted.
fn run(rule: GridFrame) -> Run {
    let root = repo_root();
    let mut ffmpeg = Ffmpeg::find().unwrap_or_else(|missing| panic!("{missing}"));
    let mut reader = Keeping {
        reader: OcrReader::new(root.join(ocr::DEFAULT_DIR)),
        last: None,
    };
    let profile = load_profile("full_swing").unwrap();
    let scratch = tempfile::tempdir().unwrap();
    let mut options = ShotOptions::new(scratch.path().join("shots"));
    options.device = profile.device.clone();

    let mut wrong = Vec::new();
    let mut rows = Vec::new();
    let mut details = Vec::new();
    let mut read = 0;
    let mut total = 0;
    for clip in &CLIPS {
        let path = root.join("data/raw/shot_screens").join(clip.file);
        let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        assert_eq!(
            hash_image(&bytes),
            clip.sha256,
            "{} is not the clip the truth table was read with",
            path.display()
        );
        drop(bytes);

        let png = scratch.path().join(clip.file).with_extension("png");
        let started = Instant::now();
        let grid = read_clip_grid(
            &path,
            &png,
            rule,
            NewImport {
                session_id: "full-swing-clip-gate",
                shot_id: None,
            },
            profile,
            &options,
            &mut ffmpeg,
            &mut reader,
        )
        .unwrap_or_else(|e| panic!("{}: {e}", clip.file));
        let seconds = started.elapsed().as_secs_f64();

        let mut cells = Vec::new();
        for (field, want) in clip.truth() {
            total += 1;
            let got = grid.shot.as_ref().and_then(|shot| stored(shot, field));
            cells.push(match got {
                Some(got) if got.to_bits() == want.to_bits() => {
                    read += 1;
                    "read"
                }
                None => "withheld",
                Some(got) => {
                    wrong.push(format!(
                        "{} (frame {} of {}): {field} = {got:?}, the screen prints {want:?}",
                        clip.file, grid.frame, grid.frames
                    ));
                    "WRONG"
                }
            });
        }
        rows.push(format!("{:<13} {}", clip.file, cells.join(" ")));

        let prepared = reader.last.take().expect("the reader read the frame");
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
            match grid.shot.as_ref().and_then(|s| s.provenance.as_ref()) {
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
        let offset = grid.shot.as_ref().and_then(|shot| shot.impact_offset_h);
        details.push(format!(
            "{}: frame {} of {}, {} boxes, needs_review {needs_review}, confidence {confidence}, \
             impact_offset_h {offset:?}, {seconds:.0} s\n  notes: {}\n  letters: {}\n  warnings: {}",
            clip.file,
            grid.frame,
            grid.frames,
            prepared.boxes.len(),
            if prepared.notes.is_empty() {
                "none".to_string()
            } else {
                prepared.notes.join(" | ")
            },
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

    let header: Vec<&str> = CLIPS[0].truth().iter().map(|(field, _)| *field).collect();
    eprintln!("rule: {rule:?}");
    eprintln!("columns: {}", header.join(" "));
    for row in &rows {
        eprintln!("{row}");
    }
    for detail in &details {
        eprintln!("{detail}");
    }
    eprintln!("coverage: {read} of {total} read, {} wrong", wrong.len());
    Run { read, total, wrong }
}

/// The gate on the five grid clips, by the rule the importer runs.
#[test]
#[ignore = "needs the five Full Swing Live Photo clips in data/raw/shot_screens/, ffmpeg and the ONNX models in data/models/ocr/"]
fn every_grid_clip_reads_as_its_screen_prints_or_withholds() {
    let run = run(GRID_FRAME);
    assert!(run.wrong.is_empty(), "{}", run.wrong.join("\n"));
    assert!(
        run.read >= READ_AT_LEAST,
        "{} of {} read, where P14 measured {READ_AT_LEAST}",
        run.read,
        run.total
    );
}
