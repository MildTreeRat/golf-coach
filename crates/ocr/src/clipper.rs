//! A closed polygon grown outward with round corners: the text detector's unclip. [M29 P4]
//!
//! The twin of what `DBPostProcess.unclip` asks of `pyclipper.PyclipperOffset` (pyclipper 1.4.0,
//! which wraps Angus Johnson's Clipper 6.4.2, `clipper.cpp`): one `AddPath(box, JT_ROUND,
//! ET_CLOSEDPOLYGON)` and one `Execute(distance)`, with pyclipper's defaults (`MiterLimit` 2,
//! `ArcTolerance` 0.25). Ported are the parts that decide the vertices: `AddPath`'s clean-up and
//! lowest point, `FixOrientations`, `DoOffset`'s arc steps, `OffsetPoint` and `DoRound`, in the
//! same `f64` steps, with Clipper's `Round` (half away from zero) and pyclipper's conversion of
//! a float coordinate to an integer one (truncation toward zero, as a C cast).
//!
//! # What stands in for the union
//!
//! `Execute` ends by passing the offset polygon through Clipper's union (a Vatti sweep, a few
//! thousand lines) with the positive fill rule. On a convex box grown outward the offset is one
//! simple polygon, and the union gives back its own vertices in its own direction, less two
//! things: `FixupOutPolygon` drops every point equal to a neighbour or exactly collinear with its
//! two, and the list **ends at its topmost vertex** (least `y`, the rightmost of a tie), because
//! the sweep's last point is the polygon's local maximum. [`offset_round`] does those two things
//! instead of the sweep. Both matter: `minAreaRect` rotates its hull to follow the input's order
//! (`crate::cv::rect`), so the same points from another start can come back as the same box with
//! its sides swapped, and the corners' last bits with it. Measured against pyclipper on 6,000
//! random boxes the detector could make (any angle, and axis-aligned): the same vertices in the
//! same order on all of them (M29 P4). The rule is the observation, not a port of the sweep, so it
//! is held only where the detector uses it: a convex four-gon grown outward.

/// pyclipper's `ArcTolerance` default, which is Clipper's `def_arc_tolerance`.
const ARC_TOLERANCE: f64 = 0.25;

/// pyclipper's `_to_clipper_point`: a Python float becomes Clipper's 64-bit `cInt` by C's cast,
/// which truncates toward zero, so `-0.7` is `0` and `10.6` is `10` (measured, M29 P4).
pub fn to_clipper_point(point: [f32; 2]) -> [i64; 2] {
    [point[0] as i64, point[1] as i64]
}

/// Clipper's `Round`: half away from zero, by adding or subtracting a half and truncating.
fn round(value: f64) -> i64 {
    if value < 0.0 {
        (value - 0.5) as i64
    } else {
        (value + 0.5) as i64
    }
}

/// `GetUnitNormal`: the left-hand unit normal of the edge from `a` to `b`, `(0, 0)` when the two
/// coincide.
fn unit_normal(a: [i64; 2], b: [i64; 2]) -> [f64; 2] {
    if a == b {
        return [0.0, 0.0];
    }
    let mut dx = (b[0] - a[0]) as f64;
    let mut dy = (b[1] - a[1]) as f64;
    let f = 1.0 / (dx * dx + dy * dy).sqrt();
    dx *= f;
    dy *= f;
    [dy, -dx]
}

/// Clipper's `Area`: the shoelace sum written its way, `(x_j + x_i) * (y_j - y_i)`, negated and
/// halved. Its sign is the orientation.
fn area(path: &[[i64; 2]]) -> f64 {
    let size = path.len();
    if size < 3 {
        return 0.0;
    }
    let mut a = 0.0;
    let mut j = size - 1;
    for i in 0..size {
        a += (path[j][0] as f64 + path[i][0] as f64) * (path[j][1] as f64 - path[i][1] as f64);
        j = i;
    }
    -a * 0.5
}

/// `AddPath` for a closed polygon: drop the repeated closing points and every point equal to the
/// one before it. `None` when fewer than three points remain, which Clipper refuses silently.
fn clean(path: &[[i64; 2]]) -> Option<Vec<[i64; 2]>> {
    let mut high = path.len().checked_sub(1)?;
    while high > 0 && path[0] == path[high] {
        high -= 1;
    }
    let mut contour = vec![path[0]];
    for &point in &path[1..=high] {
        if *contour.last().expect("starts with one point") != point {
            contour.push(point);
        }
    }
    (contour.len() >= 3).then_some(contour)
}

/// The arc constants `DoOffset` derives from `delta`: `m_sin`, `m_cos` and `m_StepsPerRad`.
struct Arc {
    sin: f64,
    cos: f64,
    steps_per_rad: f64,
}

fn arc(delta: f64) -> Arc {
    let y = if ARC_TOLERANCE > delta.abs() * ARC_TOLERANCE {
        delta.abs() * ARC_TOLERANCE
    } else {
        ARC_TOLERANCE
    };
    let mut steps = std::f64::consts::PI / (1.0 - y / delta.abs()).acos();
    if steps > delta.abs() * std::f64::consts::PI {
        steps = delta.abs() * std::f64::consts::PI;
    }
    let two_pi = std::f64::consts::PI * 2.0;
    let mut sin = (two_pi / steps).sin();
    if delta < 0.0 {
        sin = -sin;
    }
    Arc {
        sin,
        cos: (two_pi / steps).cos(),
        steps_per_rad: steps / two_pi,
    }
}

/// `SlopesEqual(a, b, c)` on 64-bit integers: `b` lies on the line through `a` and `c`, or on one
/// of them. Exact, so only truly collinear points are dropped.
fn collinear(a: [i64; 2], b: [i64; 2], c: [i64; 2]) -> bool {
    (a[1] - b[1]) * (b[0] - c[0]) == (a[0] - b[0]) * (b[1] - c[1])
}

/// `FixupOutPolygon` with `PreserveCollinear` off: walk the ring from its first point, unlinking
/// any point equal to a neighbour or collinear with its two and stepping back after each, until a
/// whole lap passes with nothing removed. The ring it leaves starts where the walk stopped.
fn fixup(points: &[[i64; 2]]) -> Vec<[i64; 2]> {
    let n = points.len();
    if n < 3 {
        return Vec::new();
    }
    let mut next: Vec<usize> = (0..n).map(|i| (i + 1) % n).collect();
    let mut prev: Vec<usize> = (0..n).map(|i| (i + n - 1) % n).collect();
    let mut alive = n;
    let mut pp = 0usize;
    let mut last_ok: Option<usize> = None;
    loop {
        if prev[pp] == pp || prev[pp] == next[pp] {
            return Vec::new();
        }
        let (a, b, c) = (points[prev[pp]], points[pp], points[next[pp]]);
        if b == c || b == a || collinear(a, b, c) {
            last_ok = None;
            let (p, q) = (prev[pp], next[pp]);
            next[p] = q;
            prev[q] = p;
            alive -= 1;
            pp = p;
        } else if last_ok == Some(pp) {
            break;
        } else {
            last_ok.get_or_insert(pp);
            pp = next[pp];
        }
    }
    let mut ring = Vec::with_capacity(alive);
    let mut at = pp;
    for _ in 0..alive {
        ring.push(points[at]);
        at = next[at];
    }
    ring
}

/// `ClipperOffset` on one closed polygon with round joins, grown by `delta`, as `Execute` returns
/// it: the offset polygon tidied as the closing union tidies it, ending at its topmost vertex
/// (the module doc says what that stands in for). Empty when the path has fewer than three
/// distinct points, where Clipper returns nothing too.
///
/// Only growing is ported. A `delta` within `1e-20` of zero makes Clipper copy the polygon, and a
/// negative one shrinks it; neither is the detector's case, whose distance is a box's area
/// times 1.5 over its perimeter for a box at least 3 px a side. Both return empty here rather
/// than an answer Clipper would not give.
pub fn offset_round(path: &[[i64; 2]], delta: f64) -> Vec<[i64; 2]> {
    if delta.is_nan() || delta <= 1e-20 {
        return Vec::new();
    }
    let Some(mut src) = clean(path) else {
        return Vec::new();
    };
    // `FixOrientations`: with one polygon, its own lowest point decides, and a polygon Clipper
    // calls negative is reversed so the offset grows outward.
    if area(&src) < 0.0 {
        src.reverse();
    }

    let arc = arc(delta);
    let len = src.len();
    let mut normals: Vec<[f64; 2]> = (0..len - 1)
        .map(|j| unit_normal(src[j], src[j + 1]))
        .collect();
    normals.push(unit_normal(src[len - 1], src[0]));

    let mut out = Vec::new();
    let at = |point: [i64; 2], normal: [f64; 2]| -> [i64; 2] {
        [
            round(point[0] as f64 + normal[0] * delta),
            round(point[1] as f64 + normal[1] * delta),
        ]
    };
    let mut k = len - 1;
    for j in 0..len {
        // `OffsetPoint(j, k, jtRound)`.
        let (nk, nj) = (normals[k], normals[j]);
        let mut sin_a = nk[0] * nj[1] - nj[0] * nk[1];
        if (sin_a * delta).abs() < 1.0 {
            let cos_a = nk[0] * nj[0] + nj[1] * nk[1];
            if cos_a > 0.0 {
                // Nearly straight on: one point, and `k` stays where it was.
                out.push(at(src[j], nk));
                continue;
            }
        } else {
            sin_a = sin_a.clamp(-1.0, 1.0);
        }
        if sin_a * delta < 0.0 {
            // A concave corner: the two offset ends and the vertex between them.
            out.push(at(src[j], nk));
            out.push(src[j]);
            out.push(at(src[j], nj));
        } else {
            // `DoRound`: from the incoming normal to the outgoing one in fixed angular steps.
            let a = sin_a.atan2(nk[0] * nj[0] + nk[1] * nj[1]);
            let steps = (round(arc.steps_per_rad * a.abs()) as i32).max(1);
            let (mut x, mut y) = (nk[0], nk[1]);
            for _ in 0..steps {
                out.push([
                    round(src[j][0] as f64 + x * delta),
                    round(src[j][1] as f64 + y * delta),
                ]);
                let x2 = x;
                x = x * arc.cos - arc.sin * y;
                y = x2 * arc.sin + y * arc.cos;
            }
            out.push(at(src[j], nj));
        }
        k = j;
    }

    let mut ring = fixup(&out);
    // End at the least `y`, the greatest `x` of those: the first such point found, scanning in
    // order, is unique because the ring now has no repeated point.
    let len = ring.len();
    if let Some(top) = (0..len).min_by_key(|&i| (ring[i][1], -ring[i][0])) {
        ring.rotate_left((top + 1) % len);
    }
    ring
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_float_corner_truncates_toward_zero_as_pyclipper_does() {
        assert_eq!(to_clipper_point([10.6, 5.9]), [10, 5]);
        assert_eq!(to_clipper_point([-0.7, -0.2]), [0, 0]);
        assert_eq!(to_clipper_point([-1.5, 2.5]), [-1, 2]);
    }

    #[test]
    fn rounding_is_half_away_from_zero() {
        assert_eq!(round(2.5), 3);
        assert_eq!(round(-2.5), -3);
        assert_eq!(round(2.4999), 2);
    }

    #[test]
    fn a_box_grown_by_three_has_pyclippers_vertices() {
        // pyclipper 1.4.0's answer for this box and distance, in its order (M29 P4): ending at
        // the top edge's right end.
        assert_eq!(
            offset_round(&[[0, 0], [10, 0], [10, 5], [0, 5]], 3.0),
            vec![
                [12, -2],
                [13, 0],
                [13, 5],
                [12, 7],
                [10, 8],
                [0, 8],
                [-2, 7],
                [-3, 5],
                [-3, 0],
                [-2, -2],
                [0, -3],
                [10, -3],
            ]
        );
    }

    #[test]
    fn duplicates_and_collinear_points_are_dropped_as_the_union_drops_them() {
        // A square with a repeated corner and a point midway along its bottom edge.
        let ring = fixup(&[[0, 0], [5, 0], [10, 0], [10, 10], [10, 10], [0, 10]]);
        let mut sorted = ring.clone();
        sorted.sort_unstable();
        assert_eq!(sorted, vec![[0, 0], [0, 10], [10, 0], [10, 10]]);
    }

    #[test]
    fn a_degenerate_path_offsets_to_nothing() {
        assert!(offset_round(&[[0, 0], [5, 0], [0, 0]], 2.0).is_empty());
        assert!(offset_round(&[[0, 0], [5, 0], [5, 5]], 0.0).is_empty());
    }
}
