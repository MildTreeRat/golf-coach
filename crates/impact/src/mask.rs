//! The three colour classes, and one class's pixels as a mask over part of a frame. [M42 P11]
//!
//! No Python twin (see the crate doc). The thresholds are the crate root's tunables, read here and
//! nowhere else, so a tuning in P13 moves one class everywhere it is used.

use crate::frame::{luminance, Frame, Rect};
use crate::{DARK_RATIO, LIGHT_MIN, LIGHT_SPREAD, WHITE_MIN, WHITE_SPREAD};

/// The smallest and largest channel of a pixel.
fn extremes([r, g, b]: [u8; 3]) -> (u8, u8) {
    (r.min(g).min(b), r.max(g).max(b))
}

/// *White*: every channel at least [`WHITE_MIN`], `max − min` at most [`WHITE_SPREAD`]. The ball.
pub(crate) fn is_white(rgb: [u8; 3]) -> bool {
    let (low, high) = extremes(rgb);
    low >= WHITE_MIN && high - low <= WHITE_SPREAD
}

/// *Light*: every channel at least [`LIGHT_MIN`] and `max − min` at most [`LIGHT_SPREAD`]. The
/// club head. At the starting values every white pixel is light too (200 >= 140, 45 <= 60); the
/// brackets allow tunings where that stops holding, and nothing depends on it but the head stage's
/// removal of the ball's disc from the light mask, which is harmless when the disc was never light.
pub(crate) fn is_light(rgb: [u8; 3]) -> bool {
    let (low, high) = extremes(rgb);
    low >= LIGHT_MIN && high - low <= LIGHT_SPREAD
}

/// *Dark*: luminance at most [`DARK_RATIO`] × `median`, the median luminance of the search region
/// ([`median_luminance`]). The shaft.
pub(crate) fn is_dark(rgb: [u8; 3], median: f64) -> bool {
    luminance(rgb) <= DARK_RATIO * median
}

/// The median luminance of a rectangle's pixels, the mean of the middle two when the count is even.
/// `None` for an empty rectangle.
pub(crate) fn median_luminance(frame: &Frame, rect: Rect) -> Option<f64> {
    let mut values = Vec::with_capacity(rect.width() * rect.height());
    for y in rect.y0..rect.y1 {
        for x in rect.x0..rect.x1 {
            values.push(luminance(frame.pixel(x, y)));
        }
    }
    crate::median(values)
}

/// One class's pixels over a rectangle of a frame. Coordinates in and out are the frame's.
pub(crate) struct Mask {
    pub rect: Rect,
    /// Row-major over `rect`.
    pub bits: Vec<bool>,
}

impl Mask {
    /// The pixels of `rect` that `class` accepts.
    pub(crate) fn of(frame: &Frame, rect: Rect, class: impl Fn([u8; 3]) -> bool) -> Mask {
        let mut bits = Vec::with_capacity(rect.width() * rect.height());
        for y in rect.y0..rect.y1 {
            for x in rect.x0..rect.x1 {
                bits.push(class(frame.pixel(x, y)));
            }
        }
        Mask { rect, bits }
    }

    /// Clears every pixel whose centre lies within `radius` of `center`, frame coordinates. The
    /// club head's stage takes the ball out of the light mask with it, while the ball is still on
    /// screen: a head touching the ball would otherwise be one component with it, and its axis,
    /// length and width the pair's rather than the head's.
    pub(crate) fn clear_disc(&mut self, center: [f64; 2], radius: f64) {
        let stride = self.rect.width();
        for (index, bit) in self.bits.iter_mut().enumerate() {
            let x = (self.rect.x0 + index % stride) as f64 + 0.5;
            let y = (self.rect.y0 + index / stride) as f64 + 0.5;
            if (x - center[0]).powi(2) + (y - center[1]).powi(2) <= radius * radius {
                *bit = false;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_stills_colours_land_in_the_classes_the_plan_expects() {
        // Finding 4's measurements, the brightest quarter of each box in the rectified stills.
        let white_digit = [240, 241, 232];
        let lit_letter = [239, 222, 90]; // IMG_2889's R
        let unlit_letter = [187, 183, 176]; // IMG_2889's F
        let field = [60, 80, 50]; // the tests' grass
        assert!(is_white(white_digit) && is_light(white_digit));
        assert!(!is_white(lit_letter) && !is_light(lit_letter));
        assert!(!is_white(unlit_letter) && is_light(unlit_letter));
        assert!(!is_white(field) && !is_light(field));
    }

    #[test]
    fn dark_is_relative_to_the_median() {
        assert!(is_dark([30, 30, 30], 100.0));
        assert!(is_dark([60, 60, 60], 100.0)); // exactly 0.6 × 100
        assert!(!is_dark([61, 61, 61], 100.0));
        assert!(!is_dark([30, 30, 30], 40.0));
    }

    #[test]
    fn the_median_luminance_of_a_rect() {
        let mut rgb = Vec::new();
        for value in [10u8, 20, 30, 40] {
            rgb.extend_from_slice(&[value, value, value]);
        }
        let frame = Frame {
            width: 2,
            height: 2,
            rgb,
        };
        let whole = Rect::whole(&frame);
        assert!((median_luminance(&frame, whole).unwrap() - 25.0).abs() < 1e-9);
        let top = Rect { y1: 1, ..whole };
        assert!((median_luminance(&frame, top).unwrap() - 15.0).abs() < 1e-9);
        let empty = Rect { x1: 0, ..whole };
        assert_eq!(median_luminance(&frame, empty), None);
    }

    #[test]
    fn a_mask_covers_its_rect_only() {
        let mut rgb = vec![0u8; 3 * 3 * 3];
        rgb[3 * 4..3 * 4 + 3].copy_from_slice(&[255, 255, 255]); // the centre pixel
        let frame = Frame {
            width: 3,
            height: 3,
            rgb,
        };
        let rect = Rect {
            x0: 1,
            y0: 1,
            x1: 3,
            y1: 2,
        };
        let mask = Mask::of(&frame, rect, is_white);
        assert_eq!(mask.bits, [true, false]);
    }

    #[test]
    fn a_cleared_disc_takes_the_pixels_whose_centres_it_holds() {
        // A 4 × 3 mask over columns 2..6 and rows 1..4 of a frame, all set.
        let rect = Rect {
            x0: 2,
            y0: 1,
            x1: 6,
            y1: 4,
        };
        let mut mask = Mask {
            rect,
            bits: vec![true; 12],
        };
        // Centred on pixel (3, 2)'s centre: radius 1 reaches its four neighbours' centres exactly
        // and no diagonal's (√2 away).
        mask.clear_disc([3.5, 2.5], 1.0);
        #[rustfmt::skip]
        let expected = [
            true,  false, true,  true,
            false, false, false, true,
            true,  false, true,  true,
        ];
        assert_eq!(mask.bits, expected);
    }
}
