//! `lab-images`: M33's helper over the crates that already do the work (the plan's call 6). Spike
//! code (rule 11).
//!
//!   lab-images glyph IMAGE BOXES.json    `ocr::recognizer::glyph_rgb` for each box, the answer
//!                                        `vision-ocr`'s `rgb` is held to (P3)
//!   lab-images working IMAGE OUT.png     `ocr::rectify::working_image`: `lab` mode's input, and
//!                                        JSON saying what the finder found (P5)
//!
//! BOXES.json is any document with a `boxes` list of `x`, `y`, `width` and `height`, so
//! `vision-ocr read`'s output is one. `float_roundtrip` is on because without it serde_json can land
//! a long decimal one bit away from what Swift printed, and the box this measures would then not be
//! the box Swift measured.

use std::path::Path;
use std::process::ExitCode;

use ocr::quad::{QuadSource, ScreenQuad};
use serde_json::{json, Value};

const USAGE: &str =
    "usage: lab-images glyph IMAGE BOXES.json\n       lab-images working IMAGE OUT.png";

/// `load_image` rather than `image::open`, because P5 and P8 read through it and it is what a lab
/// image means. On a PNG it is the `image` crate's decode, with no colour matched.
fn glyph(image: &Path, boxes: &Path) -> Result<String, String> {
    let decoded = ocr::decode::load_image(image).map_err(|error| error.to_string())?;
    let text = std::fs::read_to_string(boxes)
        .map_err(|error| format!("cannot read {}: {error}", boxes.display()))?;
    let document: Value = serde_json::from_str(&text)
        .map_err(|error| format!("{} is not JSON: {error}", boxes.display()))?;
    let rows = document["boxes"]
        .as_array()
        .ok_or_else(|| format!("{} has no `boxes` list", boxes.display()))?;
    let mut answers = Vec::with_capacity(rows.len());
    for row in rows {
        let number = |key: &str| {
            row[key]
                .as_f64()
                .ok_or_else(|| format!("a box in {} has no number `{key}`", boxes.display()))
        };
        let rgb = ocr::recognizer::glyph_rgb(
            &decoded,
            number("x")?,
            number("y")?,
            number("width")?,
            number("height")?,
        );
        answers.push(serde_json::to_string(&rgb).map_err(|error| error.to_string())?);
    }
    let lines: Vec<String> = answers.iter().map(|row| format!("  {row}")).collect();
    Ok(format!("[\n{}\n]", lines.join(",\n")))
}

/// The finder's answer as the sidecar records it. `Debug` would do for a reader, but a later phase
/// asks whether two runs or two photos agree on the path taken, so each knob is a key of its own.
fn describe_source(source: &QuadSource) -> Value {
    match source {
        QuadSource::Corners {
            low,
            close,
            epsilon,
        } => {
            json!({"kind": "corners", "low": low, "close": close, "epsilon": epsilon})
        }
        QuadSource::RotatedBox { low, close } => {
            json!({"kind": "rotated_box", "low": low, "close": close})
        }
    }
}

fn describe_quad(quad: &Option<ScreenQuad>) -> Value {
    match quad {
        Some(found) => json!({"corners": found.corners, "source": describe_source(&found.source)}),
        None => Value::Null,
    }
}

/// `lab` mode's input: the lab's own rectifier up to its rotation vote, which the spike does not run
/// (the plan's finding 9). `notes` is the constant `prepare_screen` pushes rather than a copy of its
/// words, so call 3's "verbatim" cannot drift.
///
/// A HEIC is refused here and not left to `load_image`, which would hand it to whichever ffmpeg it
/// finds: rule 6's binary decodes one 896x640 tile of a HEIC and reports success, and that wrong
/// image would then be measured. Call 2 says a HEIC's lab image starts from `vision-ocr decode`'s PNG.
fn working(image: &Path, out: &Path) -> Result<String, String> {
    if ocr::decode::is_heif(image) {
        return Err(format!(
            "{} is a HEIC: give `working` its `vision-ocr decode` PNG instead (rule 6, call 2)",
            image.display()
        ));
    }
    let photo = ocr::decode::load_image(image).map_err(|error| error.to_string())?;
    let result = ocr::rectify::working_image(&photo);
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("cannot make {}: {error}", parent.display()))?;
    }
    result
        .image
        .save_with_format(out, image::ImageFormat::Png)
        .map_err(|error| format!("cannot write {}: {error}", out.display()))?;
    let notes: Vec<&str> = match result.quad {
        Some(_) => vec![],
        None => vec![ocr::prepare::UNCROPPED_NOTE],
    };
    let answer = json!({
        "found": result.quad.is_some(),
        "quad": describe_quad(&result.quad),
        "photo": {"width": photo.width(), "height": photo.height()},
        "image": {"width": result.image.width(), "height": result.image.height()},
        "notes": notes,
    });
    serde_json::to_string_pretty(&answer).map_err(|error| error.to_string())
}

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let arguments: Vec<&str> = arguments.iter().map(String::as_str).collect();
    let result = match arguments.as_slice() {
        ["glyph", image, boxes] => glyph(Path::new(image), Path::new(boxes)),
        ["working", image, out] => working(Path::new(image), Path::new(out)),
        _ => {
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
    };
    match result {
        Ok(text) => {
            println!("{text}");
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("lab-images: {message}");
            ExitCode::FAILURE
        }
    }
}
