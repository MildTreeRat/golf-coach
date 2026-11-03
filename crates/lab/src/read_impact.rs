//! `golf-lab read-impact`: the impact offset off a Live Photo clip's replay of Full Swing's screen,
//! one line a clip, and review images of what each reading came from. [M42 P13]
//!
//! **It has no Python twin.** No script ever read the replay (`crates/impact`'s crate doc), so this
//! verb is new rather than ported, and nothing stores what it prints: it is the M42 plan's P13, the
//! reader run on real frames for the first time, and the tool its tuning was done with. `golf-lab
//! import-video` runs the same reader on every clip it imports (decision 9, M42 P14b), at the same
//! [`DECODE_WIDTH`], and stores the outcome (`crate::shot::read_impact_video`).
//!
//! Per clip the frames are decoded by [`Ffmpeg::decode_frames`](crate::ffmpeg::Ffmpeg::decode_frames)
//! at [`DECODE_WIDTH`] and handed to [`impact::read_impact`]; the outcome is one of three, as call
//! 13 names them: read, refused (a named [`ImpactRefusal`]), or a clip ffmpeg could not decode.
//!
//! **Review images** (`--review DIR`, for the user's later look, decision 3) are the frames the
//! reader read, at its size, as PNGs under `DIR/<stem>/` named by frame index. A reading writes the
//! contact frame and up to two frames either side; a refusal every fifth frame from the first,
//! since there is no contact frame to centre on. Every image outlines the ball in magenta when one
//! was found (on a refusal by [`impact::find_ball`], the one stage the crate makes public for it);
//! the contact frame alone also carries the head's heel-to-toe segment in green and a red disc on
//! its toe end, because the segment is the contact frame's own and drawn on a neighbour it would
//! cover that frame's head. The per-frame heads behind a refusal stay crate-private in `impact`, so
//! a refused clip's images show the ball and not the heads (P12 left that call to P13, and P13 kept
//! the crate's public surface as it was). The `frame-NNN.png` files a previous run left in a clip's
//! directory are removed first, so the directory always shows the latest run's frames and a
//! re-run after tuning leaves no stale frame beside them; nothing else there is touched.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use image::{Rgb, RgbImage};
use impact::{find_ball, read_impact, Frame, ImpactReading, ImpactRefusal};
use serde_json::{json, Value};

use crate::shot::MotionFrames;

/// The width every clip is decoded to before the reader sees it, pixels (the M42 plan's call 14).
/// Half of the 1920 a Live Photo's motion part is stored at: the ball is ≈ 20 px across there
/// (finding 9's ≈ 40 at 1920), every size test in `impact` is a fraction of the frame or of the
/// ball, and a quarter of the pixels is a quarter of the labelling. A portrait clip comes out 960 ×
/// 1280, rotation applied.
pub const DECODE_WIDTH: u32 = 960;

/// How many frames either side of the contact frame a reading's review writes.
const REVIEW_SPAN: usize = 2;

/// How often a refusal's review writes a frame: every this many, from the first.
const REVIEW_STRIDE: usize = 5;

const BALL_COLOUR: Rgb<u8> = Rgb([255, 0, 255]);
const HEAD_COLOUR: Rgb<u8> = Rgb([0, 255, 0]);
const TOE_COLOUR: Rgb<u8> = Rgb([255, 0, 0]);

/// What `read-impact` was asked to do.
#[derive(Debug, Clone, PartialEq)]
pub struct Request {
    /// The Live Photo clips, in the order given; each a `.mov` ([`is_clip`]).
    pub clips: Vec<PathBuf>,
    /// One JSON object a line instead of one sentence a line.
    pub json: bool,
    /// Where to write the review images, when asked.
    pub review: Option<PathBuf>,
}

/// One clip's outcome: call 13's three.
#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    Read(ImpactReading),
    Refused(ImpactRefusal),
    /// ffmpeg could not hand back the clip's frames: a machine problem, not the replay's.
    Undecodable(String),
}

impl Outcome {
    pub fn is_read(&self) -> bool {
        matches!(self, Outcome::Read(_))
    }
}

/// Whether a path names a Live Photo's motion part: a `.mov`, any case (decision 8). A still is
/// `import-photos`' to read, never this verb's.
pub fn is_clip(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("mov"))
}

/// Which side of the face a reading's offset is on, as the line names it.
pub(crate) fn side(offset_h_mm: f64) -> &'static str {
    if offset_h_mm > 0.0 {
        "toe"
    } else if offset_h_mm < 0.0 {
        "heel"
    } else {
        "middle"
    }
}

/// The verb's line for one clip. The contact frame is the reading's own index, counted from 0 as
/// the review images are named, so `contact frame 21 of 36` is `frame-021.png` of 36 frames.
pub fn line(name: &str, outcome: &Outcome) -> String {
    match outcome {
        Outcome::Read(reading) => format!(
            "{name}: impact offset {:+.1} mm ({}), contact frame {} of {}, ball {:.1} px",
            reading.offset_h_mm,
            side(reading.offset_h_mm),
            reading.contact_frame,
            reading.frames,
            reading.ball_diameter_px
        ),
        Outcome::Refused(refusal) => format!("{name}: not read - {}", refusal.sentence()),
        Outcome::Undecodable(error) => format!("{name}: not read - could not be decoded: {error}"),
    }
}

/// The verb's `--json` object for one clip: the reading's fields, or its refusal's sentence, or
/// the decoder's error, each beside the clip's name.
pub fn json_line(name: &str, outcome: &Outcome) -> Value {
    match outcome {
        Outcome::Read(reading) => json!({
            "clip": name,
            "offset_h_mm": reading.offset_h_mm,
            "contact_frame": reading.contact_frame,
            "frames": reading.frames,
            "ball_center": reading.ball_center,
            "ball_diameter_px": reading.ball_diameter_px,
            "heel": reading.heel,
            "toe": reading.toe,
        }),
        Outcome::Refused(refusal) => json!({ "clip": name, "refusal": refusal.sentence() }),
        Outcome::Undecodable(error) => {
            json!({ "clip": name, "error": format!("could not be decoded: {error}") })
        }
    }
}

/// Run the verb over every clip, printing one line (or one JSON object) a clip to `out` and the
/// review images when asked. `motion` is where the frames come from. Returns the exit code: 0 when
/// every clip was read, 1 when any was refused or could not be decoded. A review image that cannot
/// be written is an error, after the clips before it have been printed.
pub fn read_impact_clips(
    request: &Request,
    motion: &mut dyn MotionFrames,
    out: &mut dyn Write,
) -> io::Result<u8> {
    let mut every_read = true;
    for clip in &request.clips {
        let name = crate::file_name(clip);
        let (outcome, frames) = match motion.frames(clip, Some(DECODE_WIDTH)) {
            Ok(frames) => (read_frames(&frames), frames),
            Err(error) => (Outcome::Undecodable(error), Vec::new()),
        };
        every_read &= outcome.is_read();
        if request.json {
            writeln!(out, "{}", json_line(&name, &outcome))?;
        } else {
            writeln!(out, "{}", line(&name, &outcome))?;
        }
        if let Some(dir) = &request.review {
            if !frames.is_empty() {
                let stem = clip
                    .file_stem()
                    .map(|stem| stem.to_string_lossy().into_owned())
                    .unwrap_or_default();
                write_review(&dir.join(stem), &frames, &outcome)?;
            }
        }
    }
    Ok(if every_read { 0 } else { 1 })
}

/// The reader over one clip's frames, its outcome as read or refused.
pub(crate) fn read_frames(frames: &[Frame]) -> Outcome {
    match read_impact(frames) {
        Ok(reading) => Outcome::Read(reading),
        Err(refusal) => Outcome::Refused(refusal),
    }
}

/// The frame indices a clip's review writes: the contact frame and [`REVIEW_SPAN`] either side
/// for a reading, every [`REVIEW_STRIDE`]th from the first otherwise.
pub fn review_frames(outcome: &Outcome, frames: usize) -> Vec<usize> {
    match outcome {
        Outcome::Read(reading) => {
            let first = reading.contact_frame.saturating_sub(REVIEW_SPAN);
            let last = (reading.contact_frame + REVIEW_SPAN).min(frames.saturating_sub(1));
            (first..=last).collect()
        }
        _ => (0..frames).step_by(REVIEW_STRIDE).collect(),
    }
}

/// Write one clip's review images into `dir`, after removing the `frame-NNN.png` files a previous
/// run left there (the module doc says why).
pub fn write_review(dir: &Path, frames: &[Frame], outcome: &Outcome) -> io::Result<()> {
    fs::create_dir_all(dir)?;
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(is_review_name)
        {
            fs::remove_file(&path)?;
        }
    }
    let ball = match outcome {
        Outcome::Read(reading) => Some((reading.ball_center, reading.ball_diameter_px)),
        _ => find_ball(frames)
            .ok()
            .map(|ball| (ball.center, ball.diameter)),
    };
    for index in review_frames(outcome, frames.len()) {
        let frame = &frames[index];
        let mut image = RgbImage::from_raw(
            u32::try_from(frame.width).map_err(io::Error::other)?,
            u32::try_from(frame.height).map_err(io::Error::other)?,
            frame.rgb.clone(),
        )
        .ok_or_else(|| io::Error::other(format!("frame {index} is not width × height × 3")))?;
        if let Some((center, diameter)) = ball {
            ring(&mut image, center, diameter / 2.0, BALL_COLOUR);
        }
        if let Outcome::Read(reading) = outcome {
            if index == reading.contact_frame {
                segment(&mut image, reading.heel, reading.toe, HEAD_COLOUR);
                disc(&mut image, reading.toe, 3.0, TOE_COLOUR);
            }
        }
        image
            .save(dir.join(review_name(index)))
            .map_err(io::Error::other)?;
    }
    Ok(())
}

/// A review image's file name: the frame's index, three digits so a directory lists in order.
pub fn review_name(index: usize) -> String {
    format!("frame-{index:03}.png")
}

/// Whether a file name is one [`review_name`] writes: `frame-`, digits, `.png`.
fn is_review_name(name: &str) -> bool {
    name.strip_prefix("frame-")
        .and_then(|rest| rest.strip_suffix(".png"))
        .is_some_and(|digits| !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()))
}

/// Paint every pixel whose centre satisfies `inside`, within the box `[x0, x1) × [y0, y1)` in
/// continuous coordinates (`crates/impact`'s: pixel `(i, j)`'s centre is `(i + 0.5, j + 0.5)`).
fn paint(
    image: &mut RgbImage,
    [x0, y0]: [f64; 2],
    [x1, y1]: [f64; 2],
    colour: Rgb<u8>,
    inside: impl Fn(f64, f64) -> bool,
) {
    let clip = |value: f64, limit: u32| value.max(0.0).min(f64::from(limit)) as u32;
    let (width, height) = image.dimensions();
    for j in clip(y0.floor(), height)..clip(y1.ceil(), height) {
        for i in clip(x0.floor(), width)..clip(x1.ceil(), width) {
            if inside(f64::from(i) + 0.5, f64::from(j) + 0.5) {
                image.put_pixel(i, j, colour);
            }
        }
    }
}

/// A circle's outline, a pixel and a half thick, at `radius` round `center`.
fn ring(image: &mut RgbImage, center: [f64; 2], radius: f64, colour: Rgb<u8>) {
    let reach = radius + 1.0;
    paint(
        image,
        [center[0] - reach, center[1] - reach],
        [center[0] + reach, center[1] + reach],
        colour,
        |x, y| ((x - center[0]).hypot(y - center[1]) - radius).abs() <= 0.75,
    );
}

/// A filled disc of `radius` round `center`.
fn disc(image: &mut RgbImage, center: [f64; 2], radius: f64, colour: Rgb<u8>) {
    paint(
        image,
        [center[0] - radius, center[1] - radius],
        [center[0] + radius, center[1] + radius],
        colour,
        |x, y| (x - center[0]).hypot(y - center[1]) <= radius,
    );
}

/// A line segment two pixels thick from `a` to `b`.
fn segment(image: &mut RgbImage, a: [f64; 2], b: [f64; 2], colour: Rgb<u8>) {
    let along = [b[0] - a[0], b[1] - a[1]];
    let length_squared = along[0] * along[0] + along[1] * along[1];
    paint(
        image,
        [a[0].min(b[0]) - 1.0, a[1].min(b[1]) - 1.0],
        [a[0].max(b[0]) + 1.0, a[1].max(b[1]) + 1.0],
        colour,
        |x, y| {
            let t = if length_squared > 0.0 {
                (((x - a[0]) * along[0] + (y - a[1]) * along[1]) / length_squared).clamp(0.0, 1.0)
            } else {
                0.0
            };
            (x - a[0] - t * along[0]).hypot(y - a[1] - t * along[1]) <= 1.0
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reading(offset_h_mm: f64) -> ImpactReading {
        ImpactReading {
            offset_h_mm,
            contact_frame: 21,
            frames: 36,
            ball_center: [480.25, 360.5],
            ball_diameter_px: 20.4375,
            heel: [440.0, 350.0],
            toe: [500.0, 350.0],
        }
    }

    #[test]
    fn a_reading_is_one_line_naming_the_side_of_the_face() {
        assert_eq!(
            line("IMG_2889.MOV", &Outcome::Read(reading(6.3))),
            "IMG_2889.MOV: impact offset +6.3 mm (toe), contact frame 21 of 36, ball 20.4 px"
        );
        assert_eq!(
            line("IMG_2889.MOV", &Outcome::Read(reading(-12.0))),
            "IMG_2889.MOV: impact offset -12.0 mm (heel), contact frame 21 of 36, ball 20.4 px"
        );
        assert_eq!(
            line("IMG_2889.MOV", &Outcome::Read(reading(0.0))),
            "IMG_2889.MOV: impact offset +0.0 mm (middle), contact frame 21 of 36, ball 20.4 px"
        );
    }

    #[test]
    fn a_refusal_and_a_clip_that_would_not_decode_say_why() {
        assert_eq!(
            line("IMG_2892.MOV", &Outcome::Refused(ImpactRefusal::NoBall)),
            "IMG_2892.MOV: not read - no ball could be found sitting still in the replay"
        );
        assert_eq!(
            line(
                "x.mov",
                &Outcome::Undecodable("Video file not found: x.mov".into())
            ),
            "x.mov: not read - could not be decoded: Video file not found: x.mov"
        );
    }

    #[test]
    fn the_json_carries_the_readings_fields_or_why_there_is_none() {
        assert_eq!(
            json_line("IMG_2889.MOV", &Outcome::Read(reading(6.3))),
            json!({
                "clip": "IMG_2889.MOV",
                "offset_h_mm": 6.3,
                "contact_frame": 21,
                "frames": 36,
                "ball_center": [480.25, 360.5],
                "ball_diameter_px": 20.4375,
                "heel": [440.0, 350.0],
                "toe": [500.0, 350.0],
            })
        );
        assert_eq!(
            json_line("a.MOV", &Outcome::Refused(ImpactRefusal::NoShaft)),
            json!({ "clip": "a.MOV", "refusal": ImpactRefusal::NoShaft.sentence() })
        );
        assert_eq!(
            json_line("a.MOV", &Outcome::Undecodable("boom".into())),
            json!({ "clip": "a.MOV", "error": "could not be decoded: boom" })
        );
    }

    #[test]
    fn only_a_mov_is_a_clip() {
        for clip in ["IMG_2889.MOV", "a.mov", "dir/b.MoV"] {
            assert!(is_clip(Path::new(clip)), "{clip}");
        }
        for still in ["IMG_2889.HEIC", "a.jpg", "mov", "a.mov.png", "a"] {
            assert!(!is_clip(Path::new(still)), "{still}");
        }
    }

    #[test]
    fn a_reading_reviews_its_contact_frame_and_two_either_side_a_refusal_every_fifth() {
        assert_eq!(
            review_frames(&Outcome::Read(reading(1.0)), 36),
            [19, 20, 21, 22, 23]
        );
        let at_the_end = ImpactReading {
            contact_frame: 35,
            ..reading(1.0)
        };
        assert_eq!(review_frames(&Outcome::Read(at_the_end), 36), [33, 34, 35]);
        let at_the_start = ImpactReading {
            contact_frame: 1,
            ..reading(1.0)
        };
        assert_eq!(
            review_frames(&Outcome::Read(at_the_start), 36),
            [0, 1, 2, 3]
        );
        assert_eq!(
            review_frames(&Outcome::Refused(ImpactRefusal::NoBall), 11),
            [0, 5, 10]
        );
        assert!(review_frames(&Outcome::Refused(ImpactRefusal::NoFrames), 0).is_empty());
    }

    fn blank(width: usize, height: usize) -> Frame {
        Frame {
            width,
            height,
            rgb: vec![40; width * height * 3],
        }
    }

    #[test]
    fn the_run_prints_a_line_a_clip_writes_the_review_and_exits_on_the_worst() {
        let dir = tempfile::tempdir().unwrap();
        let review = dir.path().join("review");
        // A stale image a previous run left, and a file the verb did not write.
        let stale_dir = review.join("blank");
        fs::create_dir_all(&stale_dir).unwrap();
        fs::write(stale_dir.join("frame-099.png"), b"old").unwrap();
        fs::write(stale_dir.join("notes.txt"), b"mine").unwrap();
        let request = Request {
            clips: vec![PathBuf::from("blank.MOV"), PathBuf::from("broken.mov")],
            json: false,
            review: Some(review.clone()),
        };
        let mut asked = Vec::new();
        let mut decode = |clip: &Path, width: Option<u32>| {
            asked.push((clip.to_path_buf(), width));
            if clip == Path::new("broken.mov") {
                Err("ffmpeg could not decode the video of broken.mov: boom".to_owned())
            } else {
                Ok(vec![blank(64, 48); 7])
            }
        };
        let mut out = Vec::new();
        let code = read_impact_clips(&request, &mut decode, &mut out).unwrap();
        assert_eq!(code, 1);
        assert_eq!(
            String::from_utf8(out).unwrap(),
            "blank.MOV: not read - no ball could be found sitting still in the replay\n\
             broken.mov: not read - could not be decoded: ffmpeg could not decode the video of \
             broken.mov: boom\n"
        );
        assert_eq!(
            asked,
            [
                (PathBuf::from("blank.MOV"), Some(DECODE_WIDTH)),
                (PathBuf::from("broken.mov"), Some(DECODE_WIDTH)),
            ]
        );
        let mut written: Vec<String> = fs::read_dir(&stale_dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        written.sort();
        assert_eq!(written, ["frame-000.png", "frame-005.png", "notes.txt"]);
        assert!(!review.join("broken").exists(), "no frames, no review");
        let image = image::open(stale_dir.join("frame-005.png"))
            .unwrap()
            .to_rgb8();
        assert_eq!(image.dimensions(), (64, 48));
        assert_eq!(image.get_pixel(10, 10), &Rgb([40, 40, 40]));
    }

    #[test]
    fn a_readings_contact_frame_carries_the_ball_the_head_and_the_toe() {
        let dir = tempfile::tempdir().unwrap();
        let frames = vec![blank(100, 80); 3];
        let reading = ImpactReading {
            offset_h_mm: 2.0,
            contact_frame: 1,
            frames: 3,
            ball_center: [30.0, 40.0],
            ball_diameter_px: 10.0,
            heel: [50.0, 20.0],
            toe: [80.0, 20.0],
        };
        write_review(dir.path(), &frames, &Outcome::Read(reading)).unwrap();
        let open = |index| {
            image::open(dir.path().join(review_name(index)))
                .unwrap()
                .to_rgb8()
        };
        let contact = open(1);
        assert_eq!(
            contact.get_pixel(35, 40),
            &BALL_COLOUR,
            "the ring's right edge"
        );
        assert_eq!(
            contact.get_pixel(30, 40),
            &Rgb([40, 40, 40]),
            "the ring is hollow"
        );
        assert_eq!(
            contact.get_pixel(65, 19),
            &HEAD_COLOUR,
            "the segment's middle"
        );
        assert_eq!(contact.get_pixel(79, 19), &TOE_COLOUR, "the toe's disc");
        assert_eq!(
            contact.get_pixel(50, 19),
            &HEAD_COLOUR,
            "the heel is not marked"
        );
        for neighbour in [0, 2] {
            let image = open(neighbour);
            assert_eq!(image.get_pixel(35, 40), &BALL_COLOUR, "frame {neighbour}");
            assert_eq!(
                image.get_pixel(65, 19),
                &Rgb([40, 40, 40]),
                "frame {neighbour}"
            );
        }
    }

    #[test]
    fn only_the_verbs_own_file_names_count_as_review_images() {
        assert!(is_review_name("frame-000.png"));
        assert!(is_review_name("frame-1234.png"));
        for name in [
            "frame-.png",
            "frame-01a.png",
            "frame-001.jpg",
            "xframe-001.png",
            "notes",
        ] {
            assert!(!is_review_name(name), "{name}");
        }
    }
}
