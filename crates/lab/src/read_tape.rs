//! `golf-lab read-tape`: where on the face, read off photos of a STRIKEPRO label on the club, one
//! JSON line a photo, and review images of what each reading came from. [M46 P8]
//!
//! **It has no Python twin and stores nothing.** It is the M46 plan's C23, the desk tool every
//! reader phase after it is tuned with (P9 to P12 on synthetic photos, P29 on any originals the
//! phone keeps), and it runs the one reader the phone runs, [`impact::tape::read_tape`]. It runs it
//! with no golfer's hand, since the desk has no golfer: the shaft alone orients each label, without
//! the cross-check the phone makes (M46 P10).
//!
//! Each photo is decoded by [`ocr::decode::load_image`] (JPEG and PNG in-process, HEIC through
//! ffmpeg, as the OCR gate decodes them) into a [`Frame`] and read against the one label type the
//! request names. A photo that will not decode is `decode_failed`, with the decoder's own sentence
//! beside the reason, and the photos after it are still read.
//!
//! # `--sequence`: the reference is the last placed strike's crop
//!
//! Without it, every photo is read as a fresh label (`previous: None`). With it, a photo is read
//! against the crop of the latest photo before it that came out [`TapeOutcome::Read`] or
//! [`TapeOutcome::NeedsPick`], because that is what the phone's `last/<club>.rgb` will hold: the
//! last stored strike with a crop (C7, C8), a pick being stored once it is tapped. ADR-037 clause 3
//! says why the reference is a crop and not the photo: the diff is in label millimetres. A refusal
//! leaves the reference where it was, so a mark the reader saw and did not place (`mark_clipped`,
//! `no_orientation`) shows as new again on the next photo. Whether the phone does the same is P12's
//! and P18's (P8's finding). The JSON line names the photo whose crop was the reference.
//!
//! # The line
//!
//! One JSON object a photo, with the outcome's every field: `photo` (the file name), `label`,
//! `previous` and `outcome` (`read`, `needs_pick` or `not_read`), then the outcome's own. A found
//! label is its `corners` in the photo and its `crop`'s grid with the count of `outside_px`; a
//! reading adds its `orientation`, `new_label` and `marks`, each mark its `centroid_px` in the crop
//! and its `offset_h_mm` and `offset_v_mm`; a refusal is its `reason`, the golfer's `sentence` and
//! `retake_helps` (both `contracts::drill::TapeNotRead`'s, mapped by wire name), and the decoder's
//! `error` when there is one. The crop's pixels are the review's, not the line's.
//!
//! # Review images
//!
//! `--review DIR` writes, under `DIR/<stem>/`:
//! - [`PHOTO_REVIEW`]: the photo, shrunk to fit [`REVIEW_MAX_SIDE`], with a found label's box
//!   (the four corners the finder fitted) in yellow, its outline through the same map in cyan, the
//!   cross in white, and once the label is oriented, an arrow from the cross toward the toe in red
//!   and one toward the top line in blue. Written for every photo that decoded, so a `no_label` is
//!   looked at too.
//! - [`CROP_REVIEW`]: the crop at its own size, in the printed frame, with the cross, the same two
//!   arrows, and each new mark ringed in magenta round a dot at its centroid. Written whenever a
//!   label was found.
//!
//! The ring is a fixed [`MARK_RING_MM`] round the centroid, because a mark carries no extent yet
//! (P11's to add if it needs one). Both files a previous run left are removed first, so a re-run
//! after tuning never leaves a stale crop beside a photo that now finds none; nothing else in the
//! directory is touched. The drawing is `read_impact`'s helpers, shared rather than copied.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use contracts::drill::TapeNotRead;
use image::imageops::{self, FilterType};
use image::{Rgb, RgbImage};
use impact::tape::{
    read_tape, FoundLabel, Homography, LabelType, Orientation, TapeOutcome, TapeRefusal,
};
use impact::Frame;
use serde_json::{json, Map, Value};

use crate::read_impact::{disc, ring, segment};

/// The longest side a photo's review image is shrunk to, pixels. A full-size iPhone photo is 4032
/// across, about 25 MB as a PNG, and the label and the shaft are what the review is for: at 1600
/// a label filling a third of the frame is still 500 pixels across.
pub const REVIEW_MAX_SIDE: u32 = 1600;

/// The orientation arrows' length on the label, millimetres: inside the smallest label's
/// inscribed radius (the iron's 14.6), so both stay on every label type.
pub const ARROW_MM: f64 = 12.0;

/// The ring drawn round a mark's centroid on the crop's review, millimetres.
pub const MARK_RING_MM: f64 = 3.0;

/// The photo's review image's file name.
pub const PHOTO_REVIEW: &str = "photo.png";

/// The crop's review image's file name.
pub const CROP_REVIEW: &str = "crop.png";

const BOX_COLOUR: Rgb<u8> = Rgb([255, 255, 0]);
const OUTLINE_COLOUR: Rgb<u8> = Rgb([0, 255, 255]);
const CROSS_COLOUR: Rgb<u8> = Rgb([255, 255, 255]);
const TOE_COLOUR: Rgb<u8> = Rgb([255, 0, 0]);
const TOP_LINE_COLOUR: Rgb<u8> = Rgb([0, 0, 255]);
const MARK_COLOUR: Rgb<u8> = Rgb([255, 0, 255]);

/// What `read-tape` was asked to do.
#[derive(Debug, Clone, PartialEq)]
pub struct Request {
    /// The photos, in the order given, which with `sequence` is the order they were taken in.
    pub photos: Vec<PathBuf>,
    /// Read each photo against the last placed strike's crop (the module doc).
    pub sequence: bool,
    /// The label type every photo is read as.
    pub label: &'static LabelType,
    /// Where to write the review images, when asked.
    pub review: Option<PathBuf>,
}

/// The reader the run calls: [`read_tape`]'s signature less the hand, so a test can stand in for
/// it.
pub type Reader<'a> = dyn FnMut(&Frame, Option<&Frame>, &LabelType) -> TapeOutcome + 'a;

/// The decoder the run calls: a photo's pixels, or the sentence saying why there are none.
pub type Decoder<'a> = dyn FnMut(&Path) -> Result<Frame, String> + 'a;

/// A photo decoded upright to 8-bit RGB by [`ocr::decode::load_image`], as a [`Frame`].
pub fn decode_photo(path: &Path) -> Result<Frame, String> {
    let image = ocr::decode::load_image(path).map_err(|error| error.to_string())?;
    let (width, height) = image.dimensions();
    Ok(Frame {
        width: width as usize,
        height: height as usize,
        rgb: image.into_raw(),
    })
}

/// The first photo the request names that is not there, which the verb refuses before reading
/// anything: with `--sequence` a missing photo would change what every later one is read against.
pub fn missing_photo(request: &Request) -> Option<&Path> {
    request
        .photos
        .iter()
        .map(PathBuf::as_path)
        .find(|photo| !photo.exists())
}

/// The golfer's side of a reason: `contracts::drill::TapeNotRead`, by the wire name both spell.
///
/// It panics on a reason `contracts` does not spell, which is a wiring bug and never a photo's
/// doing (`docs/CODE_STANDARDS.md` R8); the module's tests walk [`TapeRefusal::ALL`] through it.
pub fn not_read(reason: TapeRefusal) -> TapeNotRead {
    serde_json::from_value(Value::String(reason.as_str().to_owned()))
        .unwrap_or_else(|_| panic!("contracts::drill::TapeNotRead has no {:?}", reason.as_str()))
}

/// The verb's line for one photo: the module doc's fields. `previous` names the photo whose crop
/// the reading was against, and `error` is the decoder's sentence for a photo that did not decode.
pub fn json_line(
    name: &str,
    label: &LabelType,
    previous: Option<&str>,
    outcome: &TapeOutcome,
    error: Option<&str>,
) -> Value {
    let mut line = Map::new();
    line.insert("photo".into(), json!(name));
    line.insert("label".into(), json!(label.id));
    line.insert("previous".into(), json!(previous));
    if let Some(found) = outcome.found() {
        line.insert("corners".into(), json!(found.corners));
        let grid = &found.crop.grid;
        line.insert(
            "crop".into(),
            json!({
                "width": grid.width,
                "height": grid.height,
                "px_per_mm": grid.px_per_mm,
                "left_mm": grid.left_mm,
                "top_mm": grid.top_mm,
                "outside_px": found.crop.outside.iter().filter(|&&outside| outside).count(),
            }),
        );
    }
    match outcome {
        TapeOutcome::Read(reading) => {
            line.insert("outcome".into(), json!("read"));
            line.insert("orientation".into(), orientation_json(reading.orientation));
            line.insert("new_label".into(), json!(reading.new_label));
            let marks: Vec<Value> = reading
                .marks
                .iter()
                .map(|mark| {
                    json!({
                        "centroid_px": mark.centroid_px,
                        "offset_h_mm": mark.offset_h_mm,
                        "offset_v_mm": mark.offset_v_mm,
                    })
                })
                .collect();
            line.insert("marks".into(), Value::Array(marks));
        }
        TapeOutcome::NeedsPick(pick) => {
            line.insert("outcome".into(), json!("needs_pick"));
            line.insert("orientation".into(), orientation_json(pick.orientation));
        }
        TapeOutcome::NotRead { reason, .. } => {
            let golfer = not_read(*reason);
            line.insert("outcome".into(), json!("not_read"));
            line.insert("reason".into(), json!(reason.as_str()));
            line.insert("sentence".into(), json!(golfer.sentence()));
            line.insert("retake_helps".into(), json!(golfer.retake_helps()));
            if let Some(error) = error {
                line.insert("error".into(), json!(error));
            }
        }
    }
    Value::Object(line)
}

/// An orientation as the line spells it: which printed side the toe and the top line are on.
fn orientation_json(orientation: Orientation) -> Value {
    json!({
        "toe": if orientation.toe_printed_right { "printed_right" } else { "printed_left" },
        "top_line": if orientation.top_line_printed_top { "printed_top" } else { "printed_bottom" },
    })
}

/// Run the verb over every photo through [`decode_photo`] and [`read_tape`], with no hand (the
/// module doc).
pub fn read_tape_photos(request: &Request, out: &mut dyn Write) -> io::Result<u8> {
    let mut read = |photo: &Frame, previous: Option<&Frame>, label: &LabelType| {
        read_tape(photo, previous, label, None)
    };
    read_tape_photos_with(request, &mut decode_photo, &mut read, out)
}

/// Run the verb over every photo with the decoder and reader given, printing one line a photo to
/// `out` and the review images when asked. Returns the exit code: 0 when every photo was read, 1
/// when any needed a pick or was not read. A review image that cannot be written is an error, after
/// the lines before it have been printed.
pub fn read_tape_photos_with(
    request: &Request,
    decode: &mut Decoder<'_>,
    read: &mut Reader<'_>,
    out: &mut dyn Write,
) -> io::Result<u8> {
    let mut every_read = true;
    // The last placed strike's crop, and the photo it came from (the module doc).
    let mut reference: Option<(String, Frame)> = None;
    for photo in &request.photos {
        let name = crate::file_name(photo);
        let decoded = decode(photo);
        let previous = reference
            .as_ref()
            .filter(|_| request.sequence && decoded.is_ok());
        let outcome = match &decoded {
            Ok(frame) => read(frame, previous.map(|(_, crop)| crop), request.label),
            Err(_) => TapeOutcome::NotRead {
                reason: TapeRefusal::DecodeFailed,
                found: None,
            },
        };
        let line = json_line(
            &name,
            request.label,
            previous.map(|(from, _)| from.as_str()),
            &outcome,
            decoded.as_ref().err().map(String::as_str),
        );
        writeln!(out, "{line}")?;
        every_read &= matches!(outcome, TapeOutcome::Read(_));
        if let Some(dir) = &request.review {
            let stem = photo
                .file_stem()
                .map(|stem| stem.to_string_lossy().into_owned())
                .unwrap_or_default();
            let dir = dir.join(stem);
            match &decoded {
                Ok(frame) => write_review(&dir, frame, request.label, &outcome)?,
                Err(_) => clear_review(&dir)?,
            }
        }
        if let TapeOutcome::Read(_) | TapeOutcome::NeedsPick(_) = &outcome {
            if let Some(found) = outcome.found() {
                reference = Some((name, found.crop.frame.clone()));
            }
        }
    }
    Ok(if every_read { 0 } else { 1 })
}

/// Remove the review files a previous run left in `dir`, if it is there.
fn clear_review(dir: &Path) -> io::Result<()> {
    for name in [PHOTO_REVIEW, CROP_REVIEW] {
        let path = dir.join(name);
        if path.is_file() {
            fs::remove_file(path)?;
        }
    }
    Ok(())
}

/// Write one photo's review images into `dir` (the module doc), after removing the ones a previous
/// run left there.
pub fn write_review(
    dir: &Path,
    photo: &Frame,
    label: &LabelType,
    outcome: &TapeOutcome,
) -> io::Result<()> {
    fs::create_dir_all(dir)?;
    clear_review(dir)?;
    let orientation = match outcome {
        TapeOutcome::Read(reading) => Some(reading.orientation),
        TapeOutcome::NeedsPick(pick) => Some(pick.orientation),
        TapeOutcome::NotRead { .. } => None,
    };
    let (mut image, scale) = review_photo(photo)?;
    if let Some(found) = outcome.found() {
        draw_found(&mut image, scale, found, label, orientation);
        let mut crop = rgb_image(&found.crop.frame)?;
        let grid = &found.crop.grid;
        let to_crop = |mm: [f64; 2]| grid.to_crop_px(mm);
        draw_cross_and_arrows(&mut crop, &to_crop, orientation, 6.0);
        if let TapeOutcome::Read(reading) = outcome {
            for mark in &reading.marks {
                ring(
                    &mut crop,
                    mark.centroid_px,
                    MARK_RING_MM * grid.px_per_mm,
                    MARK_COLOUR,
                );
                disc(&mut crop, mark.centroid_px, 3.0, MARK_COLOUR);
            }
        }
        crop.save(dir.join(CROP_REVIEW)).map_err(io::Error::other)?;
    }
    image.save(dir.join(PHOTO_REVIEW)).map_err(io::Error::other)
}

/// The photo as an image no longer than [`REVIEW_MAX_SIDE`] on either side, and the scale from the
/// photo's pixels to its pixels on each axis.
fn review_photo(photo: &Frame) -> io::Result<(RgbImage, [f64; 2])> {
    let image = rgb_image(photo)?;
    let (width, height) = image.dimensions();
    let longest = width.max(height);
    if longest <= REVIEW_MAX_SIDE {
        return Ok((image, [1.0, 1.0]));
    }
    let shrink = |side: u32| {
        ((u64::from(side) * u64::from(REVIEW_MAX_SIDE) + u64::from(longest) / 2)
            / u64::from(longest))
        .max(1) as u32
    };
    let (small_width, small_height) = (shrink(width), shrink(height));
    let small = imageops::resize(&image, small_width, small_height, FilterType::Triangle);
    Ok((
        small,
        [
            f64::from(small_width) / f64::from(width),
            f64::from(small_height) / f64::from(height),
        ],
    ))
}

/// A frame's pixels as an image, refusing one that is not `width × height × 3`.
fn rgb_image(frame: &Frame) -> io::Result<RgbImage> {
    RgbImage::from_raw(
        u32::try_from(frame.width).map_err(io::Error::other)?,
        u32::try_from(frame.height).map_err(io::Error::other)?,
        frame.rgb.clone(),
    )
    .ok_or_else(|| io::Error::other("the photo is not width × height × 3 bytes"))
}

/// A found label on the photo's review: its box, its outline through the map its corners make, the
/// cross and, once oriented, the arrows. A map the corners cannot make draws the box alone.
fn draw_found(
    image: &mut RgbImage,
    [sx, sy]: [f64; 2],
    found: &FoundLabel,
    label: &LabelType,
    orientation: Option<Orientation>,
) {
    let scaled = |[x, y]: [f64; 2]| [x * sx, y * sy];
    polygon(image, &found.corners.map(scaled), BOX_COLOUR);
    let Ok(photo_to_label) = Homography::from_corners(found.corners, label.box_corners_mm()) else {
        return;
    };
    let label_to_photo = photo_to_label.inverse();
    let to_photo = |mm: [f64; 2]| label_to_photo.apply(mm).map(scaled);
    let outline: Option<Vec<[f64; 2]>> = label.outline_mm.iter().map(|&mm| to_photo(mm)).collect();
    if let Some(outline) = outline {
        polygon(image, &outline, OUTLINE_COLOUR);
    }
    let to_photo_or_far = |mm: [f64; 2]| to_photo(mm).unwrap_or([f64::NAN; 2]);
    draw_cross_and_arrows(image, &to_photo_or_far, orientation, 3.0);
}

/// The cross, and from it the toe's arrow and the top line's when the label is oriented, each a
/// segment with a disc at its head, mapped from label millimetres by `to_image`.
fn draw_cross_and_arrows(
    image: &mut RgbImage,
    to_image: &dyn Fn([f64; 2]) -> [f64; 2],
    orientation: Option<Orientation>,
    cross_radius: f64,
) {
    let cross = to_image([0.0, 0.0]);
    if let Some(orientation) = orientation {
        for (toward, colour) in [
            ([ARROW_MM, 0.0], TOE_COLOUR),
            ([0.0, ARROW_MM], TOP_LINE_COLOUR),
        ] {
            let head = to_image(orientation.to_printed_mm(toward));
            if head.iter().chain(&cross).all(|c| c.is_finite()) {
                segment(image, cross, head, colour);
                disc(image, head, cross_radius, colour);
            }
        }
    }
    if cross.iter().all(|c| c.is_finite()) {
        disc(image, cross, cross_radius, CROSS_COLOUR);
    }
}

/// A closed polygon's edges.
fn polygon(image: &mut RgbImage, points: &[[f64; 2]], colour: Rgb<u8>) {
    for (index, &from) in points.iter().enumerate() {
        segment(image, from, points[(index + 1) % points.len()], colour);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use impact::tape::{
        rectify, TapeMark, TapePick, TapeReading, CROP_PX_PER_MM, STRIKEPRO_DRIVER, STRIKEPRO_IRON,
    };

    /// A grey photo `scale` pixels a millimetre with the iron label's box upright at `origin`, and
    /// the label found there: its corners, and the crop cut through them.
    fn found_iron(scale: f64, origin: [f64; 2]) -> (Frame, FoundLabel) {
        let (width, height) = (400, 240);
        let photo = Frame {
            width,
            height,
            rgb: vec![60; width * height * 3],
        };
        let label = &STRIKEPRO_IRON;
        let corners = label.box_corners_mm().map(|[x, y]| {
            [
                origin[0] + (x + label.centre_from_left_mm) * scale,
                origin[1] + (label.centre_from_top_mm - y) * scale,
            ]
        });
        let map = Homography::from_corners(corners, label.box_corners_mm()).unwrap();
        let crop = rectify(&photo, &map, label, CROP_PX_PER_MM);
        (photo, FoundLabel { corners, crop })
    }

    fn right_handed() -> Orientation {
        Orientation {
            toe_printed_right: false,
            top_line_printed_top: true,
        }
    }

    fn reading(found: FoundLabel) -> TapeOutcome {
        let centroid_px = found.crop.grid.to_crop_px([-4.0, 2.5]);
        TapeOutcome::Read(TapeReading {
            found,
            orientation: right_handed(),
            new_label: true,
            marks: vec![TapeMark {
                centroid_px,
                offset_h_mm: 4.0,
                offset_v_mm: 2.5,
            }],
        })
    }

    #[test]
    fn every_reason_maps_to_the_golfers_sentence_of_the_same_name() {
        for reason in TapeRefusal::ALL {
            let golfer = not_read(reason);
            assert_eq!(
                serde_json::to_value(golfer).unwrap(),
                json!(reason.as_str()),
                "{reason:?}"
            );
        }
    }

    #[test]
    fn a_refusal_is_its_reason_its_sentence_and_its_flag_and_the_decoders_error() {
        let refused = TapeOutcome::NotRead {
            reason: TapeRefusal::NoLabel,
            found: None,
        };
        assert_eq!(
            json_line("IMG_1.jpg", &STRIKEPRO_IRON, None, &refused, None),
            json!({
                "photo": "IMG_1.jpg",
                "label": "strikepro_iron",
                "previous": null,
                "outcome": "not_read",
                "reason": "no_label",
                "sentence": TapeNotRead::NoLabel.sentence(),
                "retake_helps": true,
            })
        );
        let undecoded = TapeOutcome::NotRead {
            reason: TapeRefusal::DecodeFailed,
            found: None,
        };
        let line = json_line(
            "IMG_2.HEIC",
            &STRIKEPRO_DRIVER,
            Some("IMG_1.jpg"),
            &undecoded,
            Some("could not read image IMG_2.HEIC: boom"),
        );
        assert_eq!(line["reason"], "decode_failed");
        assert_eq!(line["previous"], "IMG_1.jpg");
        assert_eq!(line["error"], "could not read image IMG_2.HEIC: boom");
        assert_eq!(line["label"], "strikepro_driver");
    }

    #[test]
    fn a_reading_carries_the_label_the_orientation_and_every_mark() {
        let (_, found) = found_iron(4.0, [50.0, 40.0]);
        let corners = found.corners;
        let line = json_line("IMG_3.jpg", &STRIKEPRO_IRON, None, &reading(found), None);
        assert_eq!(line["outcome"], "read");
        assert_eq!(line["corners"], json!(corners));
        assert_eq!(
            line["crop"],
            json!({
                "width": 730,
                "height": 399,
                "px_per_mm": 10.0,
                "left_mm": -36.6,
                "top_mm": 24.8,
                "outside_px": 0,
            })
        );
        assert_eq!(
            line["orientation"],
            json!({ "toe": "printed_left", "top_line": "printed_top" })
        );
        assert_eq!(line["new_label"], true);
        assert_eq!(line["marks"][0]["offset_h_mm"], 4.0);
        assert_eq!(line["marks"][0]["offset_v_mm"], 2.5);
        assert_eq!(line["marks"].as_array().unwrap().len(), 1);
        assert!(line.get("reason").is_none());
    }

    #[test]
    fn a_pick_and_a_refusal_past_the_finder_carry_the_label_they_found() {
        let (_, found) = found_iron(4.0, [50.0, 40.0]);
        let pick = TapeOutcome::NeedsPick(TapePick {
            found: found.clone(),
            orientation: right_handed(),
        });
        let line = json_line("a.jpg", &STRIKEPRO_IRON, None, &pick, None);
        assert_eq!(line["outcome"], "needs_pick");
        assert_eq!(line["crop"]["width"], 730);
        assert!(line.get("marks").is_none());
        let clipped = TapeOutcome::NotRead {
            reason: TapeRefusal::MarkClipped,
            found: Some(found),
        };
        let line = json_line("a.jpg", &STRIKEPRO_IRON, None, &clipped, None);
        assert_eq!(line["reason"], "mark_clipped");
        assert_eq!(line["retake_helps"], false);
        assert_eq!(line["crop"]["height"], 399);
        assert!(line.get("orientation").is_none());
    }

    #[test]
    fn a_readings_review_draws_the_box_the_cross_the_arrows_and_the_mark() {
        let dir = tempfile::tempdir().unwrap();
        let (photo, found) = found_iron(4.0, [50.0, 40.0]);
        let corners = found.corners;
        let outcome = reading(found);
        write_review(dir.path(), &photo, &STRIKEPRO_IRON, &outcome).unwrap();
        let shot = image::open(dir.path().join(PHOTO_REVIEW))
            .unwrap()
            .to_rgb8();
        assert_eq!(shot.dimensions(), (400, 240));
        // The box's top edge 10 px in from its left corner, where the iron's shoulder has dropped
        // the outline 35 px below it; and the outline on the box's top edge above the cross, where
        // the label's flat top runs along it. Then the photo's grey away from everything drawn.
        let box_top = [corners[0][0] + 10.0, corners[0][1]];
        assert_eq!(
            shot.get_pixel(box_top[0] as u32, box_top[1] as u32),
            &BOX_COLOUR
        );
        let flat_top = [corners[0][0] + 36.6 * 4.0, corners[0][1]];
        assert_eq!(
            shot.get_pixel(flat_top[0] as u32, flat_top[1] as u32),
            &OUTLINE_COLOUR
        );
        assert_eq!(shot.get_pixel(10, 10), &Rgb([60, 60, 60]));
        // The cross is 36.6 mm in and 24.8 mm down at 4 px/mm; the toe is to its left on a
        // right-handed club, and the top line above it.
        let cross = [50.0 + 36.6 * 4.0, 40.0 + 24.8 * 4.0];
        assert_eq!(
            shot.get_pixel(cross[0] as u32, cross[1] as u32),
            &CROSS_COLOUR
        );
        let toe = [cross[0] - ARROW_MM * 4.0, cross[1]];
        assert_eq!(shot.get_pixel(toe[0] as u32, toe[1] as u32), &TOE_COLOUR);
        let top_line = [cross[0], cross[1] - ARROW_MM * 4.0];
        assert_eq!(
            shot.get_pixel(top_line[0] as u32, top_line[1] as u32),
            &TOP_LINE_COLOUR
        );

        let crop = image::open(dir.path().join(CROP_REVIEW)).unwrap().to_rgb8();
        assert_eq!(crop.dimensions(), (730, 399));
        let TapeOutcome::Read(reading) = &outcome else {
            unreachable!()
        };
        let [mx, my] = reading.marks[0].centroid_px;
        assert_eq!(crop.get_pixel(mx as u32, my as u32), &MARK_COLOUR);
        let rim = mx + MARK_RING_MM * CROP_PX_PER_MM;
        assert_eq!(crop.get_pixel(rim as u32, my as u32), &MARK_COLOUR);
        assert_eq!(
            crop.get_pixel((mx + 15.0) as u32, my as u32),
            &Rgb([60, 60, 60]),
            "the ring is hollow"
        );
    }

    #[test]
    fn a_photo_with_no_label_reviews_bare_and_a_large_one_is_shrunk_to_fit() {
        let dir = tempfile::tempdir().unwrap();
        // A stale crop a previous run left, and a file the verb did not write.
        fs::write(dir.path().join(CROP_REVIEW), b"old").unwrap();
        fs::write(dir.path().join("notes.txt"), b"mine").unwrap();
        let photo = Frame {
            width: 3200,
            height: 100,
            rgb: vec![30; 3200 * 100 * 3],
        };
        let refused = TapeOutcome::NotRead {
            reason: TapeRefusal::NoLabel,
            found: None,
        };
        write_review(dir.path(), &photo, &STRIKEPRO_IRON, &refused).unwrap();
        let shot = image::open(dir.path().join(PHOTO_REVIEW))
            .unwrap()
            .to_rgb8();
        assert_eq!(shot.dimensions(), (REVIEW_MAX_SIDE, 50));
        assert_eq!(shot.get_pixel(800, 25), &Rgb([30, 30, 30]));
        assert!(!dir.path().join(CROP_REVIEW).exists(), "the stale crop");
        assert!(dir.path().join("notes.txt").exists());
    }

    /// The reader here answers by each photo's one byte: 0 no label, 1 read, 2 a pick, 3 a clipped
    /// mark. Each crop it returns carries its photo's byte, so what a photo was read against is
    /// seen by that byte.
    #[test]
    fn a_sequence_reads_each_photo_against_the_last_placed_strikes_crop() {
        let (_, found) = found_iron(4.0, [50.0, 40.0]);
        let photos: Vec<PathBuf> = ["a.jpg", "b.jpg", "c.jpg", "d.jpg", "e.png", "f.jpg"]
            .iter()
            .map(PathBuf::from)
            .collect();
        // a read, b refused clipped, c not decoded, d a pick, e no label, f read.
        let first_byte = |photo: &Path| match photo.to_str().unwrap() {
            "a.jpg" | "f.jpg" => 1,
            "b.jpg" => 3,
            "d.jpg" => 2,
            _ => 0,
        };
        let mut decode = |photo: &Path| {
            if photo == Path::new("c.jpg") {
                Err("could not read image c.jpg: boom".to_owned())
            } else {
                Ok(Frame {
                    width: 1,
                    height: 1,
                    rgb: vec![first_byte(photo); 3],
                })
            }
        };
        let against = std::cell::RefCell::new(Vec::new());
        let mut read = |photo: &Frame, previous: Option<&Frame>, _: &LabelType| {
            against.borrow_mut().push(previous.map(|crop| crop.rgb[0]));
            // The crop each placed strike leaves is marked by its photo's byte.
            let mut found = found.clone();
            found.crop.frame.rgb[0] = photo.rgb[0];
            match photo.rgb[0] {
                0 => TapeOutcome::NotRead {
                    reason: TapeRefusal::NoLabel,
                    found: None,
                },
                1 => reading(found),
                2 => TapeOutcome::NeedsPick(TapePick {
                    found,
                    orientation: right_handed(),
                }),
                _ => TapeOutcome::NotRead {
                    reason: TapeRefusal::MarkClipped,
                    found: Some(found),
                },
            }
        };
        let request = Request {
            photos,
            sequence: true,
            label: &STRIKEPRO_IRON,
            review: None,
        };
        let mut out = Vec::new();
        let code = read_tape_photos_with(&request, &mut decode, &mut read, &mut out).unwrap();
        assert_eq!(code, 1);
        // c never reached the reader, so it was read against nothing; b's clipped mark left the
        // reference at a, and d's pick moved it.
        assert_eq!(
            *against.borrow(),
            [None, Some(1), Some(1), Some(2), Some(2)]
        );
        let lines: Vec<Value> = String::from_utf8(out)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        let previous: Vec<&Value> = lines.iter().map(|line| &line["previous"]).collect();
        assert_eq!(
            previous,
            [
                &Value::Null,
                &json!("a.jpg"),
                &Value::Null,
                &json!("a.jpg"),
                &json!("d.jpg"),
                &json!("d.jpg"),
            ]
        );
        assert_eq!(lines[2]["reason"], "decode_failed");
        assert_eq!(lines[2]["error"], "could not read image c.jpg: boom");

        // Without --sequence every photo is read as a fresh label, and all read is exit 0.
        let request = Request {
            photos: vec![PathBuf::from("a.jpg"), PathBuf::from("f.jpg")],
            sequence: false,
            ..request
        };
        against.borrow_mut().clear();
        let mut out = Vec::new();
        let code = read_tape_photos_with(&request, &mut decode, &mut read, &mut out).unwrap();
        assert_eq!(code, 0);
        assert_eq!(*against.borrow(), [None, None]);
    }
}
