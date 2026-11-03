//! One decoded frame, a pixel's luminance, and the clipped rectangles every stage reads. [M42 P11]
//!
//! No Python twin (see the crate doc).

/// One frame of a clip: `width × height` pixels, row-major, three bytes a pixel (red, green, blue).
///
/// The decoder's choice of size is the caller's: `golf-lab` asks ffmpeg for 960 wide (the M42
/// plan's call 14), and every threshold here that depends on size is a fraction of the frame's or
/// of the ball's, so no stage assumes one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub width: usize,
    pub height: usize,
    pub rgb: Vec<u8>,
}

impl Frame {
    /// Whether `rgb` holds exactly `width × height × 3` bytes.
    pub fn is_well_formed(&self) -> bool {
        self.width
            .checked_mul(self.height)
            .and_then(|n| n.checked_mul(3))
            == Some(self.rgb.len())
    }

    /// The pixel at column `x`, row `y`. The caller keeps both inside the frame.
    pub(crate) fn pixel(&self, x: usize, y: usize) -> [u8; 3] {
        let at = (y * self.width + x) * 3;
        [self.rgb[at], self.rgb[at + 1], self.rgb[at + 2]]
    }
}

/// Rec. 601 luma, `0.299r + 0.587g + 0.114b`: what the dark class compares against the search
/// region's median. 601 rather than 709 because the comparison is relative, so the weights only
/// have to be one rule applied twice, and 601's are the ones call 12 wrote down.
pub(crate) fn luminance([r, g, b]: [u8; 3]) -> f64 {
    0.299 * f64::from(r) + 0.587 * f64::from(g) + 0.114 * f64::from(b)
}

/// The sharpest frame of a clip, by index: the one with the largest sum of squared luminance
/// differences between each pixel and its right and lower neighbours, ties to the earliest; `None`
/// when no frame is well formed ([`Frame::is_well_formed`]), which a frame that is not is never
/// chosen. [M42 P14]
///
/// **No Python twin**, as nothing in this crate has one. It is the M42 plan's call 14: the frame a
/// Live Photo clip's grid of numbers is read from, which `golf-lab import-video` writes as a PNG and
/// hands the still's own reader. It lives here, beside [`Frame`] and away from the replay's
/// stages, because the phone (M38) has to choose the same frame from its own decoder's frames
/// before its own reader sees it, and this crate is the one both hosts link.
///
/// The sum is of the luminance [`luminance`] gives, so it is the gradient energy a focus measure
/// reads: a frame caught mid-pan or mid-refocus smears each edge over several pixels, and its
/// squared steps shrink. It is not normalised by size, because one clip's frames share one size.
/// The middle frame (`len / 2`) was the alternative P14 measured against it; the plan's P14
/// findings hold both counts.
pub fn sharpest_frame(frames: &[Frame]) -> Option<usize> {
    let mut best: Option<(usize, f64)> = None;
    for (index, frame) in frames.iter().enumerate() {
        if !frame.is_well_formed() {
            continue;
        }
        let energy = gradient_energy(frame);
        // Strictly greater, so an equal sum later in the clip never displaces an earlier one.
        if best.is_none_or(|(_, most)| energy > most) {
            best = Some((index, energy));
        }
    }
    best.map(|(index, _)| index)
}

/// `Σ (L(x+1, y) − L(x, y))² + Σ (L(x, y+1) − L(x, y))²` over the pixels that have each neighbour,
/// a row at a time so only two rows of luminance are held, summed in raster order so the same
/// frame always gives the same bits.
fn gradient_energy(frame: &Frame) -> f64 {
    let row = |y: usize| -> Vec<f64> {
        (0..frame.width)
            .map(|x| luminance(frame.pixel(x, y)))
            .collect()
    };
    let mut energy = 0.0;
    let mut above: Option<Vec<f64>> = None;
    for y in 0..frame.height {
        let here = row(y);
        if let Some(above) = &above {
            for (upper, lower) in above.iter().zip(&here) {
                energy += (lower - upper).powi(2);
            }
        }
        for pair in here.windows(2) {
            energy += (pair[1] - pair[0]).powi(2);
        }
        above = Some(here);
    }
    energy
}

/// A rectangle of whole pixels, half-open: columns `x0..x1`, rows `y0..y1`, inside its frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Rect {
    pub x0: usize,
    pub y0: usize,
    pub x1: usize,
    pub y1: usize,
}

impl Rect {
    /// The whole frame.
    pub(crate) fn whole(frame: &Frame) -> Rect {
        Rect {
            x0: 0,
            y0: 0,
            x1: frame.width,
            y1: frame.height,
        }
    }

    /// The square of side `side` centred on `center`, clipped to a `width × height` frame by the
    /// rule `TextBox.rgb`'s box is clipped by (M42 P2): with `x` its left edge, `x0 =
    /// floor(max(x, 0))` and `x1 = min(ceil(x + side), width)`, likewise `y`. Empty, never
    /// inverted, when the square misses the frame.
    pub(crate) fn square(center: [f64; 2], side: f64, width: usize, height: usize) -> Rect {
        let clip = |centre: f64, limit: usize| {
            let low = (centre - side / 2.0).max(0.0).floor();
            let high = (centre + side / 2.0).ceil().min(limit as f64).max(0.0);
            // Both ends are whole and at least 0, and `high` at most `limit`; `low` past `high` is
            // a square beyond the far edge, held to `high` so it comes out empty.
            let (low, high) = (low as usize, high as usize);
            (low.min(high), high)
        };
        let (x0, x1) = clip(center[0], width);
        let (y0, y1) = clip(center[1], height);
        Rect { x0, y0, x1, y1 }
    }

    pub(crate) fn width(&self) -> usize {
        self.x1 - self.x0
    }

    pub(crate) fn height(&self) -> usize {
        self.y1 - self.y0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn well_formed_means_three_bytes_a_pixel() {
        let frame = |len| Frame {
            width: 4,
            height: 3,
            rgb: vec![0; len],
        };
        assert!(frame(36).is_well_formed());
        assert!(!frame(35).is_well_formed());
        assert!(!frame(37).is_well_formed());
    }

    #[test]
    fn a_pixel_is_read_row_major() {
        let mut rgb = vec![0; 2 * 2 * 3];
        rgb[9..12].copy_from_slice(&[1, 2, 3]); // column 1, row 1
        let frame = Frame {
            width: 2,
            height: 2,
            rgb,
        };
        assert_eq!(frame.pixel(1, 1), [1, 2, 3]);
        assert_eq!(frame.pixel(0, 1), [0, 0, 0]);
    }

    #[test]
    fn luminance_weighs_green_most() {
        assert_eq!(luminance([0, 0, 0]), 0.0);
        assert!((luminance([255, 255, 255]) - 255.0).abs() < 1e-9);
        assert!(luminance([0, 100, 0]) > luminance([100, 0, 0]));
        assert!(luminance([100, 0, 0]) > luminance([0, 0, 100]));
    }

    /// A grey frame with a white bar down its middle, the bar's edges stepped (`sharp`) or ramped
    /// over three columns, as a frame caught mid-pan smears them.
    fn bar(sharp: bool) -> Frame {
        let (width, height) = (12, 6);
        let column = |x: usize| -> u8 {
            match (sharp, x) {
                (_, 5..=6) => 240,
                (false, 4 | 7) => 160,
                (false, 3 | 8) => 100,
                _ => 40,
            }
        };
        let mut rgb = Vec::new();
        for _ in 0..height {
            for x in 0..width {
                rgb.extend([column(x); 3]);
            }
        }
        Frame { width, height, rgb }
    }

    #[test]
    fn a_smeared_copy_loses_to_the_sharp_one_wherever_it_sits() {
        assert_eq!(
            sharpest_frame(&[bar(false), bar(true), bar(false)]),
            Some(1)
        );
        assert_eq!(sharpest_frame(&[bar(true), bar(false)]), Some(0));
        assert_eq!(
            sharpest_frame(&[bar(false), bar(false), bar(true)]),
            Some(2)
        );
        // The energy itself: the sharp bar's two steps of 200, on each of its six rows, and no
        // vertical step at all in a frame whose rows are alike.
        assert_eq!(gradient_energy(&bar(true)), 6.0 * 2.0 * 200.0_f64.powi(2));
    }

    #[test]
    fn equal_frames_go_to_the_earliest() {
        assert_eq!(sharpest_frame(&[bar(true), bar(true)]), Some(0));
        assert_eq!(sharpest_frame(&[bar(false), bar(true), bar(true)]), Some(1));
        // A flat frame has no energy at all, and a clip of flat frames still has a sharpest.
        let flat = Frame {
            width: 3,
            height: 3,
            rgb: vec![90; 27],
        };
        assert_eq!(gradient_energy(&flat), 0.0);
        assert_eq!(sharpest_frame(&[flat.clone(), flat]), Some(0));
    }

    #[test]
    fn no_frames_or_none_well_formed_choose_nothing() {
        assert_eq!(sharpest_frame(&[]), None);
        let broken = Frame {
            rgb: vec![255; 5],
            ..bar(true)
        };
        assert_eq!(sharpest_frame(std::slice::from_ref(&broken)), None);
        // Never chosen, however much it would weigh, so the frame after it is.
        assert_eq!(sharpest_frame(&[broken, bar(false)]), Some(1));
    }

    #[test]
    fn a_square_is_clipped_to_whole_pixels_inside_the_frame() {
        // Inside: 10.5 ± 3 is [7.5, 13.5], so pixels 7..14.
        assert_eq!(
            Rect::square([10.5, 20.5], 6.0, 100, 100),
            Rect {
                x0: 7,
                y0: 17,
                x1: 14,
                y1: 24
            }
        );
        // Over the top-left corner and the right edge.
        let clipped = Rect::square([2.0, 98.0], 10.0, 100, 100);
        assert_eq!((clipped.x0, clipped.x1), (0, 7));
        assert_eq!((clipped.y0, clipped.y1), (93, 100));
        // Wholly outside: empty, not inverted.
        let outside = Rect::square([-50.0, 500.0], 10.0, 100, 100);
        assert_eq!(outside.width(), 0);
        assert_eq!(outside.height(), 0);
    }
}
