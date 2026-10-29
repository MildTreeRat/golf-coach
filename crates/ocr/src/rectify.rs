//! The image the recognizer sees: the screen warped front-on, shrunk to 1600 px, its contrast
//! lifted, and turned. [M29 P3]
//!
//! The twin of `launch_monitor/screen/preprocess.py`'s `warp_to_rect`, `_resize`, `enhance` and
//! `_rotate`, and of the part of `prepare_screen` before its rotation vote ([`working_image`]),
//! over the OpenCV ports in [`crate::cv`]: `warp` for the perspective, `resize` for the shrink,
//! `lab` and `clahe` for the contrast. The vote itself needs the recognizer, so it is P7's
//! `prepare.rs`, which starts from [`working_image`] and tries each [`Rotation`] in
//! [`Rotation::VOTE_ORDER`].
//!
//! # How close it lands (M29 P3, the 13 bay photos)
//!
//! **Identical, every pixel, on the twelve photos whose decode is identical** (`crate::decode`):
//! the warped screen, the 1600 px copy and the working image after `enhance`, held by digest in
//! `tests/fixtures/screen_working.json` (`tests/rectify.rs`). Six are warped (two from corners,
//! four from the rotated box) and six read uncropped, so both of `prepare_screen`'s branches are
//! covered. Fed frozen Python's own decoded pixels, the HEIC's working image is identical too, so
//! the port adds nothing there; from this crate's ffmpeg decode it differs by what the decode
//! differs by, carried through the warp and spread by CLAHE (`tests/rectify.rs` says how far).
//!
//! The photos are gitignored, so `tests/rectify.rs` also holds the port where the always-run suite
//! can: the L\*a\*b\* conversion both ways on every 8-bit input, and the warp, CLAHE (each of its
//! padding branches) and the turns on a small synthetic screen, against `cv2`'s answers in
//! `tests/fixtures/rectify_reference.json`.
//!
//! # What OpenCV does that this does not
//!
//! The 1600 px shrink is `resize_area`, which always takes OpenCV's general area path; OpenCV
//! switches to `resizeAreaFast` when both ratios are whole numbers, as a warped screen exactly 3200
//! px wide with an even height would make them (`crate::cv::resize` says what then differs). No bay
//! photo does.

use image::{imageops, RgbImage};

use crate::cv::clahe::clahe;
use crate::cv::lab::{lab_to_rgb, rgb_to_lab};
use crate::cv::resize::{resize_area, shrink_to_width};
use crate::cv::warp::{get_perspective_transform, invert3, warp_perspective};
use crate::quad::{find_screen_quad, Quad, ScreenQuad};

/// The rectified screen is shrunk to this width before OCR (`_TARGET_WIDTH`): big enough that the
/// tile values stay legible, small enough that four rotation attempts stay quick.
pub const TARGET_WIDTH: usize = 1600;

/// `enhance`'s CLAHE: clip limit 2.0 on an 8x8 grid.
const CLAHE_CLIP_LIMIT: f64 = 2.0;
const CLAHE_TILES: usize = 8;

/// `warp_to_rect`: the region inside `quad` warped to a front-on rectangle. Each side is the
/// longer of the quad's two opposite edges (the far edge of a tilted screen is foreshortened),
/// measured as NumPy measures a `float32` difference, `sqrt(dx*dx + dy*dy)` in `f32` (checked
/// against `np.linalg.norm` on 200,000 random pairs, M29 P3), and truncated as Python's `int`
/// does.
pub fn warp_to_rect(image: &RgbImage, quad: &Quad) -> RgbImage {
    let [top_left, top_right, bottom_right, bottom_left] = *quad;
    let edge = |a: [f32; 2], b: [f32; 2]| {
        let (dx, dy) = (a[0] - b[0], a[1] - b[1]);
        (dx * dx + dy * dy).sqrt()
    };
    let longer = |a: f32, b: f32| if b > a { b } else { a };
    let width =
        (longer(edge(top_right, top_left), edge(bottom_right, bottom_left)) as usize).max(1);
    let height =
        (longer(edge(bottom_left, top_left), edge(bottom_right, top_right)) as usize).max(1);

    let (right, bottom) = ((width - 1) as f32, (height - 1) as f32);
    let target = [[0.0, 0.0], [right, 0.0], [right, bottom], [0.0, bottom]];
    let inverse = invert3(&get_perspective_transform(quad, &target));
    let pixels = warp_perspective(
        image.as_raw(),
        image.width() as usize,
        image.height() as usize,
        &inverse,
        width,
        height,
    );
    from_raw(width, height, pixels)
}

/// `_resize`: shrunk by area averaging to [`TARGET_WIDTH`] wide, the height scaled and truncated
/// as Python does; an image already no wider is returned as it is.
pub fn resize(image: &RgbImage) -> RgbImage {
    let (width, height) = (image.width() as usize, image.height() as usize);
    match shrink_to_width(width, height, TARGET_WIDTH) {
        Some((_, w, h)) => from_raw(w, h, resize_area(image.as_raw(), width, height, 3, w, h)),
        None => image.clone(),
    }
}

/// `enhance`: CLAHE on the L\*a\*b\* lightness, which lifts the local contrast of text inside the
/// dark tiles without amplifying the ceiling's reflections, then back to RGB with `a` and `b`
/// untouched.
pub fn enhance(image: &RgbImage) -> RgbImage {
    let (width, height) = (image.width() as usize, image.height() as usize);
    let lab: Vec<[u8; 3]> = image
        .as_raw()
        .chunks_exact(3)
        .map(|p| rgb_to_lab([p[0], p[1], p[2]]))
        .collect();
    let lightness: Vec<u8> = lab.iter().map(|p| p[0]).collect();
    let lifted = clahe(
        &lightness,
        width,
        height,
        CLAHE_CLIP_LIMIT,
        CLAHE_TILES,
        CLAHE_TILES,
    );
    let pixels = lab
        .iter()
        .zip(lifted)
        .flat_map(|(p, l)| lab_to_rgb([l, p[1], p[2]]))
        .collect();
    from_raw(width, height, pixels)
}

/// The four orientations the vote tries (`_ROTATIONS`), each a turn `cv2.rotate` makes exactly.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rotation {
    Upright,
    /// Upside down, the common phone flip.
    Half,
    /// A quarter turn clockwise (`ROTATE_90_CLOCKWISE`).
    Clockwise,
    /// A quarter turn counter-clockwise (`ROTATE_90_COUNTERCLOCKWISE`), Python's 270.
    CounterClockwise,
}

impl Rotation {
    /// Best guess first, as `_ROTATIONS` orders them: upright, upside down, then the two sideways.
    pub const VOTE_ORDER: [Rotation; 4] = [
        Rotation::Upright,
        Rotation::Half,
        Rotation::Clockwise,
        Rotation::CounterClockwise,
    ];

    /// The angle frozen Python records in `PreparedScreen.rotation` and its note.
    pub fn degrees(self) -> u16 {
        match self {
            Rotation::Upright => 0,
            Rotation::Half => 180,
            Rotation::Clockwise => 90,
            Rotation::CounterClockwise => 270,
        }
    }
}

/// `_rotate`.
pub fn rotate(image: &RgbImage, rotation: Rotation) -> RgbImage {
    match rotation {
        Rotation::Upright => image.clone(),
        Rotation::Half => imageops::rotate180(image),
        Rotation::Clockwise => imageops::rotate90(image),
        Rotation::CounterClockwise => imageops::rotate270(image),
    }
}

/// The image `prepare_screen` hands its rotation vote, and the quad it was cut by (`None` when the
/// screen's outline was not found and the photo is read uncropped, which `prepare_screen` notes).
#[derive(Clone, Debug)]
pub struct Working {
    pub image: RgbImage,
    pub quad: Option<ScreenQuad>,
}

/// `prepare_screen` up to its vote: find the screen, warp it front-on if found, shrink, enhance.
pub fn working_image(photo: &RgbImage) -> Working {
    let quad = find_screen_quad(photo);
    let shrunk = match &quad {
        Some(found) => resize(&warp_to_rect(photo, &found.corners)),
        None => resize(photo),
    };
    Working {
        image: enhance(&shrunk),
        quad,
    }
}

fn from_raw(width: usize, height: usize, pixels: Vec<u8>) -> RgbImage {
    RgbImage::from_raw(width as u32, height as u32, pixels)
        .expect("a buffer of width * height * 3 bytes")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_vote_order_and_angles_are_pythons() {
        let degrees: Vec<u16> = Rotation::VOTE_ORDER.iter().map(|r| r.degrees()).collect();
        assert_eq!(degrees, vec![0, 180, 90, 270]);
    }

    #[test]
    fn a_quarter_turn_clockwise_puts_the_left_column_on_top() {
        // 2 wide, 1 tall: red then blue. Clockwise, red ends on top of a 1x2 image.
        let image = RgbImage::from_raw(2, 1, vec![255, 0, 0, 0, 0, 255]).unwrap();
        let turned = rotate(&image, Rotation::Clockwise);
        assert_eq!(turned.dimensions(), (1, 2));
        assert_eq!(turned.as_raw(), &vec![255, 0, 0, 0, 0, 255]);
        let back = rotate(&image, Rotation::CounterClockwise);
        assert_eq!(back.as_raw(), &vec![0, 0, 255, 255, 0, 0]);
    }

    #[test]
    fn an_image_no_wider_than_the_target_is_not_resized() {
        let image = RgbImage::new(TARGET_WIDTH as u32, 10);
        assert_eq!(resize(&image).dimensions(), (TARGET_WIDTH as u32, 10));
        let wide = RgbImage::new(1920, 1440);
        assert_eq!(resize(&wide).dimensions(), (1600, 1200));
    }

    #[test]
    fn the_warp_size_is_the_longer_edge_truncated() {
        // A trapezoid whose top edge is 100 px and bottom 120.5: the output is 120 wide.
        let image = RgbImage::new(200, 100);
        let quad = [[40.0, 10.0], [140.0, 10.0], [150.25, 60.0], [29.75, 60.0]];
        let warped = warp_to_rect(&image, &quad);
        assert_eq!(warped.width(), 120);
        // The sides are sqrt(10.25^2 + 50^2) = 51.04 long: 51 tall.
        assert_eq!(warped.height(), 51);
    }
}
