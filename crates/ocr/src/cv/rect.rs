//! `cv2.minAreaRect(contour)` and `cv2.boxPoints(rect)`: `find_screen_quad`'s fallback, the
//! rotated bounding box of an outline that never reduced to four corners. [M29 P2]
//!
//! Twins of OpenCV 4.10's `convexHull` with its `Sklansky_` scan (`imgproc/src/convhull.cpp`),
//! `rotatingCalipers` and `minAreaRect` (`imgproc/src/rotcalipers.cpp`), and
//! `RotatedRect::points` (`core/src/types.cpp`), in the same `f32` and `f64` steps. The fallback is
//! not a corner case on the bay photos: it is the quad on five of the thirteen, and the four-corner
//! path on only two (M29 P2).
//!
//! Which rectangle is minimal is geometry; which corner the calipers report from, and so the last
//! bits of each coordinate, is the order of the hull and the tie rule (`area <= minarea` keeps the
//! *last* of equal areas). Both are ported, so an axis-aligned box comes back with OpenCV's
//! `cos(90 degrees)` residue (a coordinate of `-1.2e-13` rather than `0`) on the same corner.

use super::contours::Point;

/// `cv::RotatedRect`: centre, size (width, height) and angle in degrees.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RotatedRect {
    pub center: [f32; 2],
    pub size: [f32; 2],
    pub angle: f32,
}

/// `Sklansky_` on sorted point indices: one monotone chain from `start` towards `end`, as stack
/// indices into `sorted`. `nsign` and `sign2` pick the half and the turn, as in OpenCV.
fn sklansky(
    sorted: &[Point],
    start: usize,
    end: usize,
    stack: &mut [usize],
    nsign: i64,
    sign2: i64,
) -> usize {
    let incr: isize = if end > start { 1 } else { -1 };
    let mut pprev = start as isize;
    let mut pcur = pprev + incr;
    let mut pnext = pcur + incr;
    let mut stacksize = 3usize;
    if start == end || (sorted[start].x == sorted[end].x && sorted[start].y == sorted[end].y) {
        stack[0] = start;
        return 1;
    }
    stack[0] = pprev as usize;
    stack[1] = pcur as usize;
    stack[2] = pnext as usize;
    let after_end = end as isize + incr;
    while pnext != after_end {
        let cur = sorted[pcur as usize];
        let next = sorted[pnext as usize];
        let by = i64::from(next.y) - i64::from(cur.y);
        if by.signum() != nsign {
            let prev = sorted[pprev as usize];
            let ax = i64::from(cur.x) - i64::from(prev.x);
            let bx = i64::from(next.x) - i64::from(cur.x);
            let ay = i64::from(cur.y) - i64::from(prev.y);
            let convexity = ay * bx - ax * by;
            if convexity.signum() == sign2 && (ax != 0 || ay != 0) {
                pprev = pcur;
                pcur = pnext;
                pnext += incr;
                stack[stacksize] = pnext as usize;
                stacksize += 1;
            } else if pprev == start as isize {
                pcur = pnext;
                stack[1] = pcur as usize;
                pnext += incr;
                stack[2] = pnext as usize;
            } else {
                stack[stacksize - 2] = pnext as usize;
                pcur = pprev;
                pprev = stack[stacksize - 4] as isize;
                stacksize -= 1;
            }
        } else {
            pnext += incr;
            stack[stacksize - 1] = pnext as usize;
        }
    }
    stacksize - 1
}

/// `convexHull(points, clockwise=false, returnPoints=true)` on integer points, in OpenCV's vertex
/// order (the order `rotatingCalipers` walks).
pub fn convex_hull(points: &[Point]) -> Vec<Point> {
    let total = points.len();
    if total == 0 {
        return Vec::new();
    }
    // Sort indices by (x, y), ties by original position, as `CHullCmpPoints` does by address.
    let mut order: Vec<usize> = (0..total).collect();
    order.sort_by(|&a, &b| (points[a].x, points[a].y, a).cmp(&(points[b].x, points[b].y, b)));
    let sorted: Vec<Point> = order.iter().map(|&i| points[i]).collect();
    let (mut miny_ind, mut maxy_ind) = (0usize, 0usize);
    for (i, p) in sorted.iter().enumerate().skip(1) {
        if sorted[miny_ind].y > p.y {
            miny_ind = i;
        }
        if sorted[maxy_ind].y < p.y {
            maxy_ind = i;
        }
    }
    let mut hull: Vec<usize> = Vec::with_capacity(total);
    let mut stack = vec![0usize; total + 2];
    if sorted[0] == sorted[total - 1] {
        hull.push(order[0]);
    } else {
        // Upper half. With `clockwise` false the two chains are swapped before gathering.
        let tl_count = sklansky(&sorted, 0, maxy_ind, &mut stack, -1, 1);
        let tl: Vec<usize> = stack[..tl_count].to_vec();
        let tr_count = sklansky(&sorted, total - 1, maxy_ind, &mut stack[tl_count..], -1, -1);
        let tr: Vec<usize> = stack[tl_count..tl_count + tr_count].to_vec();
        let (tl, tr) = (tr, tl);
        for &i in &tl[..tl.len() - 1] {
            hull.push(order[i]);
        }
        for i in (1..tr.len()).rev() {
            hull.push(order[tr[i]]);
        }
        let stop_idx: Option<usize> = if tr.len() > 2 {
            Some(tr[1])
        } else if tl.len() > 2 {
            Some(tl[tl.len() - 2])
        } else {
            None
        };

        // Lower half.
        let bl_count = sklansky(&sorted, 0, miny_ind, &mut stack, 1, -1);
        let mut bl: Vec<usize> = stack[..bl_count].to_vec();
        let br_count = sklansky(&sorted, total - 1, miny_ind, &mut stack[bl_count..], 1, 1);
        let mut br: Vec<usize> = stack[bl_count..bl_count + br_count].to_vec();
        if let Some(stop) = stop_idx {
            let check_idx: Option<usize> = if bl.len() > 2 {
                Some(bl[1])
            } else if bl.len() + br.len() > 2 {
                Some(br[2 - bl.len()])
            } else {
                None
            };
            if let Some(check) = check_idx {
                if check == stop || sorted[check] == sorted[stop] {
                    // Every point on one line: the lower chain mirrors the upper.
                    bl.truncate(2);
                    br.truncate(2);
                }
            }
        }
        for &i in &bl[..bl.len() - 1] {
            hull.push(order[i]);
        }
        for i in (1..br.len()).rev() {
            hull.push(order[br[i]]);
        }

        // Rotate the output so the original indices run ascending or descending, if they can.
        let nout = hull.len();
        if nout >= 3 {
            let (mut min_idx, mut max_idx, mut lt) = (0usize, 0usize, 0usize);
            for i in 1..nout {
                let idx = hull[i];
                lt += usize::from(hull[i - 1] < idx);
                // More than one rise and more than one fall: neither order is reachable. (`lt < i`
                // was tried first and refused rotations `cv2` makes, on 11 of 3,000 random sets.)
                if lt > 1 && lt + 1 < i {
                    break;
                }
                if idx < hull[min_idx] {
                    min_idx = i;
                }
                if idx > hull[max_idx] {
                    max_idx = i;
                }
            }
            let mmdist = max_idx.abs_diff(min_idx);
            if (mmdist == 1 || mmdist == nout - 1) && (lt <= 1 || lt >= nout - 2) {
                let ascending = (max_idx + 1) % nout == min_idx;
                let i0 = if ascending { min_idx } else { max_idx };
                if i0 > 0 {
                    let mut rotated = Vec::with_capacity(nout);
                    let mut j = i0;
                    let mut complete = true;
                    for i in 0..nout {
                        let curr = hull[j];
                        rotated.push(curr);
                        let next_j = if j + 1 < nout { j + 1 } else { 0 };
                        let next = hull[next_j];
                        if i < nout - 1 && (ascending != (curr < next)) {
                            complete = false;
                            break;
                        }
                        j = next_j;
                    }
                    if complete {
                        hull = rotated;
                    }
                }
            }
        }
    }
    hull.into_iter().map(|i| points[i]).collect()
}

/// `rotatingCalipers(..., CALIPERS_MINAREARECT)`: the corner and the two side vectors of the
/// smallest enclosing rectangle of a convex polygon with at least three vertices.
fn rotating_calipers(points: &[[f32; 2]]) -> [[f32; 2]; 3] {
    let n = points.len();
    let mut vect = vec![[0f32; 2]; n];
    let mut inv_vect_length = vec![0f32; n];
    let (mut left, mut bottom, mut right, mut top) = (0usize, 0usize, 0usize, 0usize);
    let mut pt0 = points[0];
    let (mut left_x, mut right_x, mut top_y, mut bottom_y) = (pt0[0], pt0[0], pt0[1], pt0[1]);
    for i in 0..n {
        if pt0[0] < left_x {
            left_x = pt0[0];
            left = i;
        }
        if pt0[0] > right_x {
            right_x = pt0[0];
            right = i;
        }
        if pt0[1] > top_y {
            top_y = pt0[1];
            top = i;
        }
        if pt0[1] < bottom_y {
            bottom_y = pt0[1];
            bottom = i;
        }
        let pt = points[if i + 1 < n { i + 1 } else { 0 }];
        let dx = f64::from(pt[0]) - f64::from(pt0[0]);
        let dy = f64::from(pt[1]) - f64::from(pt0[1]);
        vect[i] = [dx as f32, dy as f32];
        inv_vect_length[i] = (1.0 / (dx * dx + dy * dy).sqrt()) as f32;
        pt0 = pt;
    }

    // The hull's orientation, from the first turn that is not straight. OpenCV 4.10 still
    // computes it and seeds the base vector with it, but its caliper choice no longer reads the
    // base, and every turn overwrites it before the width and height do, so only the assertion
    // that the hull is not degenerate is left of it.
    let mut orientation = 0f32;
    {
        let (mut ax, mut ay) = (f64::from(vect[n - 1][0]), f64::from(vect[n - 1][1]));
        for v in &vect {
            let (bx, by) = (f64::from(v[0]), f64::from(v[1]));
            let convexity = ax * by - ay * bx;
            if convexity != 0.0 {
                orientation = if convexity > 0.0 { 1.0 } else { -1.0 };
                break;
            }
            ax = bx;
            ay = by;
        }
        assert!(orientation != 0.0, "a degenerate hull");
    }

    let mut seq = [bottom, right, top, left];
    let mut minarea = f32::MAX;
    let mut best_left = 0usize;
    let mut best_bottom = 0usize;
    let (mut best_a, mut best_b, mut best_width, mut best_height) = (0f32, 0f32, 0f32, 0f32);
    for _ in 0..n {
        // Which caliper turns next: each one's next hull edge, turned into the bottom caliper's
        // frame (`rotate90CW`, `rotate180`, `rotate90CCW`), and the one furthest clockwise wins,
        // by the sign of a cross product (`firstVecIsRight`), the first on a tie. Older OpenCV
        // compared each edge's cosine with its caliper instead (`dp` times the inverse edge
        // length, the largest winning), and this port first followed that. It missed 4.10 in the
        // last bits on about one contour in a thousand, which M29 P2 took for exact ties; M29 P5
        // read 4.10's `rotcalipers.cpp` and found this.
        let v = |i: usize| vect[seq[i]];
        let rot_vect = [
            v(0),
            [v(1)[1], -v(1)[0]],
            [-v(2)[0], -v(2)[1]],
            [-v(3)[1], v(3)[0]],
        ];
        // `tmp = rotate90CW(vec1); tmp.x * vec2.x + tmp.y * vec2.y < 0`, each product in `f32`.
        let first_is_right = |a: [f32; 2], b: [f32; 2]| a[1] * b[0] + (-a[0]) * b[1] < 0.0;
        let mut main_element = 0usize;
        for i in 1..4 {
            if first_is_right(rot_vect[i], rot_vect[main_element]) {
                main_element = i;
            }
        }
        let pindex = seq[main_element];
        let lead_x = vect[pindex][0] * inv_vect_length[pindex];
        let lead_y = vect[pindex][1] * inv_vect_length[pindex];
        let (base_a, base_b) = match main_element {
            0 => (lead_x, lead_y),
            1 => (lead_y, -lead_x),
            2 => (-lead_x, -lead_y),
            _ => (-lead_y, lead_x),
        };
        seq[main_element] += 1;
        if seq[main_element] == n {
            seq[main_element] = 0;
        }

        let dx = points[seq[1]][0] - points[seq[3]][0];
        let dy = points[seq[1]][1] - points[seq[3]][1];
        let width = dx * base_a + dy * base_b;
        let dx = points[seq[2]][0] - points[seq[0]][0];
        let dy = points[seq[2]][1] - points[seq[0]][1];
        let height = -dx * base_b + dy * base_a;
        let area = width * height;
        if area <= minarea {
            minarea = area;
            best_left = seq[3];
            best_a = base_a;
            best_width = width;
            best_b = base_b;
            best_height = height;
            best_bottom = seq[0];
        }
    }

    let (a1, b1) = (best_a, best_b);
    let (a2, b2) = (-best_b, best_a);
    let c1 = a1 * points[best_left][0] + points[best_left][1] * b1;
    let c2 = a2 * points[best_bottom][0] + points[best_bottom][1] * b2;
    let idet = 1.0f32 / (a1 * b2 - a2 * b1);
    let px = (c1 * b2 - c2 * b1) * idet;
    let py = (a1 * c2 - a2 * c1) * idet;
    [
        [px, py],
        [a1 * best_width, b1 * best_width],
        [a2 * best_height, b2 * best_height],
    ]
}

/// `minAreaRect(points)`: the smallest rotated rectangle around integer points.
pub fn min_area_rect(points: &[Point]) -> RotatedRect {
    let hull: Vec<[f32; 2]> = convex_hull(points)
        .into_iter()
        .map(|p| [p.x as f32, p.y as f32])
        .collect();
    let mut rect = RotatedRect {
        center: [0.0, 0.0],
        size: [0.0, 0.0],
        angle: 0.0,
    };
    match hull.len() {
        0 => {}
        1 => rect.center = hull[0],
        2 => {
            rect.center = [
                (hull[0][0] + hull[1][0]) * 0.5,
                (hull[0][1] + hull[1][1]) * 0.5,
            ];
            let dx = f64::from(hull[1][0]) - f64::from(hull[0][0]);
            let dy = f64::from(hull[1][1]) - f64::from(hull[0][1]);
            rect.size = [(dx * dx + dy * dy).sqrt() as f32, 0.0];
            rect.angle = dy.atan2(dx) as f32;
        }
        _ => {
            let [corner, v1, v2] = rotating_calipers(&hull);
            rect.center = [
                corner[0] + (v1[0] + v2[0]) * 0.5,
                corner[1] + (v1[1] + v2[1]) * 0.5,
            ];
            rect.size = [
                (f64::from(v1[0]) * f64::from(v1[0]) + f64::from(v1[1]) * f64::from(v1[1])).sqrt()
                    as f32,
                (f64::from(v2[0]) * f64::from(v2[0]) + f64::from(v2[1]) * f64::from(v2[1])).sqrt()
                    as f32,
            ];
            rect.angle = f64::from(v1[1]).atan2(f64::from(v1[0])) as f32;
        }
    }
    // `box.angle*180/CV_PI`: the multiply is `float * int`, so it happens in `f32`, and only the
    // division by the `double` constant widens.
    rect.angle = (f64::from(rect.angle * 180.0) / std::f64::consts::PI) as f32;
    rect
}

/// `boxPoints(rect)`: the four corners, from the angle's cosine and sine in `f32`.
pub fn box_points(rect: &RotatedRect) -> [[f32; 2]; 4] {
    let angle = f64::from(rect.angle) * std::f64::consts::PI / 180.0;
    let b = angle.cos() as f32 * 0.5;
    let a = angle.sin() as f32 * 0.5;
    let [cx, cy] = rect.center;
    let [w, h] = rect.size;
    let p0 = [cx - a * h - b * w, cy + b * h - a * w];
    let p1 = [cx + a * h - b * w, cy - b * h - a * w];
    let p2 = [2.0 * cx - p0[0], 2.0 * cy - p0[1]];
    let p3 = [2.0 * cx - p1[0], 2.0 * cy - p1[1]];
    [p0, p1, p2, p3]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pts(list: &[(i32, i32)]) -> Vec<Point> {
        list.iter().map(|&(x, y)| Point { x, y }).collect()
    }

    #[test]
    fn the_hull_drops_interior_points() {
        let hull = convex_hull(&pts(&[(0, 0), (2, 1), (4, 0), (4, 4), (2, 2), (0, 4)]));
        let mut corners: Vec<(i32, i32)> = hull.iter().map(|p| (p.x, p.y)).collect();
        corners.sort();
        assert_eq!(corners, vec![(0, 0), (0, 4), (4, 0), (4, 4)]);
    }

    #[test]
    fn an_axis_aligned_rectangle_is_its_own_box() {
        let rect = min_area_rect(&pts(&[(0, 23), (0, 749), (999, 749), (999, 23)]));
        let mut corners: Vec<(i64, i64)> = box_points(&rect)
            .iter()
            .map(|p| (p[0].round() as i64, p[1].round() as i64))
            .collect();
        corners.sort();
        assert_eq!(corners, vec![(0, 23), (0, 749), (999, 23), (999, 749)]);
        assert_eq!(rect.size[0] * rect.size[1], 726.0 * 999.0);
    }

    #[test]
    fn a_tilted_square_is_found_at_its_tilt() {
        // A diamond: the minimal box is the diamond itself, at 45 degrees, side sqrt(50).
        let rect = min_area_rect(&pts(&[(5, 0), (10, 5), (5, 10), (0, 5)]));
        assert!((rect.size[0] - 50f32.sqrt()).abs() < 1e-4, "{rect:?}");
        assert!((rect.size[1] - 50f32.sqrt()).abs() < 1e-4, "{rect:?}");
        assert!((rect.center[0] - 5.0).abs() < 1e-4 && (rect.center[1] - 5.0).abs() < 1e-4);
        assert!((rect.angle.abs() - 45.0).abs() < 1e-3, "{rect:?}");
    }

    /// Four corners, and `cv2.minAreaRect`'s centre, size and angle for them.
    type Case = ([(i32, i32); 4], [f32; 5]);

    #[test]
    fn the_caliper_is_chosen_as_opencv_4_10_chooses_it() {
        // `cv2.minAreaRect` (OpenCV 4.10) on four boxes where comparing cosines, as older OpenCV
        // did, lands an ulp or more away (M29 P5). The first is a detector's box from 2026-08-23-10,
        // whose crop came out a rounding off; the others are from 30,000 random sets. Each `f32` is
        // written in its shortest spelling, so equality is to the bit.
        let cases: [Case; 4] = [
            (
                [(671, 467), (761, 471), (760, 500), (670, 497)],
                [715.51666, 483.50058, 90.0833, 30.016663, 1.9091525],
            ),
            (
                [(801, 590), (879, 578), (880, 584), (802, 597)],
                [840.41895, 587.0135, 7.0691557, 79.07591, 80.537674],
            ),
            (
                [(818, 1471), (775, 1453), (929, 1085), (972, 1103)],
                [873.49994, 1278.0, 46.615448, 398.92856, 22.70825],
            ),
            (
                [(1141, 585), (1101, 569), (1155, 434), (1194, 450)],
                [1148.0, 509.49997, 43.081318, 145.39944, 21.801407],
            ),
        ];
        for (points, want) in cases {
            let rect = min_area_rect(&pts(&points));
            let got = [
                rect.center[0],
                rect.center[1],
                rect.size[0],
                rect.size[1],
                rect.angle,
            ];
            assert_eq!(got, want, "{points:?}");
        }
        // And the corners `get_mini_boxes` sorts, `cv2.boxPoints`' to the bit on the first.
        let rect = min_area_rect(&pts(&cases[0].0));
        assert_eq!(
            box_points(&rect),
            [
                [670.0, 497.00003],
                [671.0, 467.00003],
                [761.0333, 470.00113],
                [760.0333, 500.00113],
            ]
        );
    }
}
