//! 4-connected components of a mask, each with its moments. [M42 P11]
//!
//! No Python twin (see the crate doc). 4-connected rather than 8 because a club head and the ball
//! touching corner to corner, or a shaft grazing a head's corner, should stay two components: an
//! 8-connected fill joins anything that meets at one diagonal pixel.

use crate::frame::Rect;
use crate::mask::Mask;

/// One connected run of a mask's pixels, in the frame's coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Component {
    /// Pixels in it.
    pub area: usize,
    /// The mean of its pixels' centres.
    pub centroid: [f64; 2],
    /// The smallest rectangle of whole pixels holding it.
    pub bbox: Rect,
    /// Its principal axis, a unit vector at `θ = ½·atan2(2·μ₁₁, μ₂₀ − μ₀₂)` in `(−π/2, π/2]` from
    /// the second central moments. A round blob has no axis to speak of and gets `θ = 0`.
    pub axis: [f64; 2],
    /// Its extent along the axis: the spread of its pixel centres' projections, plus one pixel for
    /// the footprint, so a straight run of `n` pixels measures `n`.
    pub length: f64,
    /// Its extent across the axis, measured the same way.
    pub width: f64,
    /// The two ends of its axis segment: the centroid moved along the axis to each extreme
    /// projection, widened by half a pixel so the ends are `length` apart. The lower one first.
    pub ends: [[f64; 2]; 2],
}

impl Component {
    /// `2·sqrt(area / π)`: the diameter of the disc with its area.
    pub(crate) fn equivalent_diameter(&self) -> f64 {
        2.0 * (self.area as f64 / std::f64::consts::PI).sqrt()
    }

    /// Its pixels' share of its bounding box. A drawn disc fills `π/4 ≈ 0.785`, a square 1.
    pub(crate) fn fill(&self) -> f64 {
        self.area as f64 / (self.bbox.width() * self.bbox.height()) as f64
    }

    /// The bounding box's width over its height.
    pub(crate) fn aspect(&self) -> f64 {
        self.bbox.width() as f64 / self.bbox.height() as f64
    }

    /// Builds one from its pixels, given as indices row-major into a mask over `rect`.
    fn from_pixels(pixels: &[usize], rect: Rect) -> Component {
        let stride = rect.width();
        let centre = |index: usize| {
            [
                (rect.x0 + index % stride) as f64 + 0.5,
                (rect.y0 + index / stride) as f64 + 0.5,
            ]
        };
        let n = pixels.len() as f64;
        let (mut sum_x, mut sum_y) = (0.0, 0.0);
        let mut bbox = Rect {
            x0: usize::MAX,
            y0: usize::MAX,
            x1: 0,
            y1: 0,
        };
        for &index in pixels {
            let [x, y] = centre(index);
            sum_x += x;
            sum_y += y;
            let (column, row) = (rect.x0 + index % stride, rect.y0 + index / stride);
            bbox.x0 = bbox.x0.min(column);
            bbox.y0 = bbox.y0.min(row);
            bbox.x1 = bbox.x1.max(column + 1);
            bbox.y1 = bbox.y1.max(row + 1);
        }
        let centroid = [sum_x / n, sum_y / n];
        // Central moments on a second pass, rather than raw sums less the mean squared, so a
        // component far from the origin keeps its precision.
        let (mut mu20, mut mu02, mut mu11) = (0.0, 0.0, 0.0);
        for &index in pixels {
            let [x, y] = centre(index);
            let (dx, dy) = (x - centroid[0], y - centroid[1]);
            mu20 += dx * dx;
            mu02 += dy * dy;
            mu11 += dx * dy;
        }
        let theta = 0.5 * (2.0 * mu11).atan2(mu20 - mu02);
        let axis = [theta.cos(), theta.sin()];
        let across = [-axis[1], axis[0]];
        let (mut along_low, mut along_high) = (f64::INFINITY, f64::NEG_INFINITY);
        let (mut across_low, mut across_high) = (f64::INFINITY, f64::NEG_INFINITY);
        for &index in pixels {
            let [x, y] = centre(index);
            let (dx, dy) = (x - centroid[0], y - centroid[1]);
            let along = dx * axis[0] + dy * axis[1];
            let side = dx * across[0] + dy * across[1];
            along_low = along_low.min(along);
            along_high = along_high.max(along);
            across_low = across_low.min(side);
            across_high = across_high.max(side);
        }
        let end = |t: f64| [centroid[0] + t * axis[0], centroid[1] + t * axis[1]];
        Component {
            area: pixels.len(),
            centroid,
            bbox,
            axis,
            length: along_high - along_low + 1.0,
            width: across_high - across_low + 1.0,
            ends: [end(along_low - 0.5), end(along_high + 0.5)],
        }
    }
}

/// How far `point` lies from the segment `ends`: to the nearest point of the segment, which is an
/// end when the point projects beyond it. The club head's stage ranks its candidates by it and the
/// contact frame is chosen by it (the M42 plan's call 12), so a ball off the head's toe is measured
/// to the toe and not to the line the head lies on.
pub(crate) fn segment_distance(ends: [[f64; 2]; 2], point: [f64; 2]) -> f64 {
    let [a, b] = ends;
    let (ab, ap) = (
        [b[0] - a[0], b[1] - a[1]],
        [point[0] - a[0], point[1] - a[1]],
    );
    let length_squared = ab[0] * ab[0] + ab[1] * ab[1];
    // A segment of no length is its one point; a component's never is (its length is at least
    // one pixel), but the guard keeps the division honest.
    let t = if length_squared > 0.0 {
        ((ap[0] * ab[0] + ap[1] * ab[1]) / length_squared).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (ap[0] - t * ab[0]).hypot(ap[1] - t * ab[1])
}

/// Every 4-connected component of `mask`, in the order a row-major scan first meets each.
pub(crate) fn components(mask: &Mask) -> Vec<Component> {
    let (width, height) = (mask.rect.width(), mask.rect.height());
    let mut seen = vec![false; width * height];
    let mut found = Vec::new();
    let mut stack = Vec::new();
    let mut pixels = Vec::new();
    for start in 0..width * height {
        if !mask.bits[start] || seen[start] {
            continue;
        }
        seen[start] = true;
        stack.push(start);
        pixels.clear();
        while let Some(index) = stack.pop() {
            pixels.push(index);
            let (x, y) = (index % width, index / width);
            let mut visit = |next: usize| {
                if mask.bits[next] && !seen[next] {
                    seen[next] = true;
                    stack.push(next);
                }
            };
            if x > 0 {
                visit(index - 1);
            }
            if x + 1 < width {
                visit(index + 1);
            }
            if y > 0 {
                visit(index - width);
            }
            if y + 1 < height {
                visit(index + width);
            }
        }
        found.push(Component::from_pixels(&pixels, mask.rect));
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A mask over a `width × height` frame from the pixels whose centres `inside` accepts.
    fn drawn(width: usize, height: usize, inside: impl Fn(f64, f64) -> bool) -> Mask {
        let rect = Rect {
            x0: 0,
            y0: 0,
            x1: width,
            y1: height,
        };
        let mut bits = Vec::with_capacity(width * height);
        for y in 0..height {
            for x in 0..width {
                bits.push(inside(x as f64 + 0.5, y as f64 + 0.5));
            }
        }
        Mask { rect, bits }
    }

    #[test]
    fn a_tilted_bar_has_its_axis_length_and_width() {
        // tests/impact.rs draws its head as a 60 × 10 bar at 30°, and the stages read these four.
        let (centre, angle) = ([100.3, 80.6], 30f64.to_radians());
        let (u, v) = ([angle.cos(), angle.sin()], [-angle.sin(), angle.cos()]);
        let mask = drawn(200, 160, |x, y| {
            let (dx, dy) = (x - centre[0], y - centre[1]);
            (dx * u[0] + dy * u[1]).abs() <= 30.0 && (dx * v[0] + dy * v[1]).abs() <= 5.0
        });
        let found = components(&mask);
        assert_eq!(found.len(), 1);
        let bar = found[0];
        assert!((bar.centroid[0] - centre[0]).abs() < 0.2, "{bar:?}");
        assert!((bar.centroid[1] - centre[1]).abs() < 0.2, "{bar:?}");
        let theta = bar.axis[1].atan2(bar.axis[0]);
        assert!((theta - angle).abs() < 0.01, "θ = {theta}");
        assert!((bar.length - 60.0).abs() <= 1.5, "length {}", bar.length);
        assert!((bar.width - 10.0).abs() <= 1.5, "width {}", bar.width);
        // The ends straddle the centroid along the axis, `length` apart.
        let [a, b] = bar.ends;
        let span = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt();
        assert!((span - bar.length).abs() < 1e-9);
        assert!(((a[0] + b[0]) / 2.0 - centre[0]).abs() < 1.0);
        assert!(a[0] < b[0], "the first end is at the lower projection");
    }

    #[test]
    fn a_straight_run_of_n_pixels_measures_n() {
        let mask = drawn(20, 5, |x, y| {
            (3.0..10.0).contains(&x) && (2.0..3.0).contains(&y)
        });
        let run = components(&mask)[0];
        assert_eq!(run.area, 7);
        assert!((run.length - 7.0).abs() < 1e-9);
        assert!((run.width - 1.0).abs() < 1e-9);
        assert_eq!(
            run.bbox,
            Rect {
                x0: 3,
                y0: 2,
                x1: 10,
                y1: 3
            }
        );
    }

    #[test]
    fn diagonal_neighbours_are_two_components() {
        let mask = drawn(4, 4, |x, y| {
            (x < 1.0 && y < 1.0) || ((1.0..2.0).contains(&x) && (1.0..2.0).contains(&y))
        });
        let found = components(&mask);
        assert_eq!(found.len(), 2);
        assert_eq!((found[0].area, found[1].area), (1, 1));
    }

    #[test]
    fn components_come_in_scan_order_with_frame_coordinates() {
        // A mask over part of a frame reports the frame's coordinates, not the rectangle's.
        let rect = Rect {
            x0: 10,
            y0: 20,
            x1: 14,
            y1: 23,
        };
        #[rustfmt::skip]
        let bits = vec![
            false, false, false, true,
            true,  false, false, true,
            true,  false, false, false,
        ];
        let found = components(&Mask { rect, bits });
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].centroid, [13.5, 21.0]); // met first, at (3, 0)
        assert_eq!(found[1].centroid, [10.5, 22.0]);
    }

    #[test]
    fn a_disc_fills_a_quarter_pi_of_its_box_and_a_square_all_of_it() {
        let disc = components(&drawn(60, 60, |x, y| {
            (x - 30.0).powi(2) + (y - 30.0).powi(2) <= 100.0
        }))[0];
        assert!(
            (disc.fill() - std::f64::consts::FRAC_PI_4).abs() < 0.03,
            "{}",
            disc.fill()
        );
        assert!((disc.equivalent_diameter() - 20.0).abs() < 0.5);
        assert!((disc.aspect() - 1.0).abs() < 1e-9);
        let square = components(&drawn(60, 60, |x, y| {
            (10.0..30.0).contains(&x) && (10.0..30.0).contains(&y)
        }))[0];
        assert!((square.fill() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn a_point_is_measured_to_the_segment_and_past_its_ends_to_the_end() {
        let ends = [[0.0, 0.0], [10.0, 0.0]];
        assert!((segment_distance(ends, [4.0, 3.0]) - 3.0).abs() < 1e-12); // beside it
        assert!((segment_distance(ends, [13.0, 4.0]) - 5.0).abs() < 1e-12); // past the far end
        assert!((segment_distance(ends, [-3.0, -4.0]) - 5.0).abs() < 1e-12); // before the near one
        assert!((segment_distance([[1.0, 1.0], [1.0, 1.0]], [4.0, 5.0]) - 5.0).abs() < 1e-12);
    }
}
