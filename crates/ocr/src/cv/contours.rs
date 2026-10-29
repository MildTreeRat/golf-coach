//! `cv2.findContours(..., RETR_EXTERNAL, CHAIN_APPROX_SIMPLE)` and the four measurements
//! `find_screen_quad` takes of a contour [M29 P2]; and the text detector's `RETR_LIST` scan, with
//! the two measurements its unclip takes of a box's `float32` corners [M29 P4].
//!
//! Twins of OpenCV 4.10's border follower (`cvFindNextContour` and `icvFetchContour`,
//! `imgproc/src/contours.cpp`, Suzuki and Abe's algorithm), `contourArea`, `arcLength`
//! (`imgproc/src/shapedescr.cpp`), `approxPolyDP_` (`imgproc/src/approx.cpp`) and
//! `isContourConvex_` (`imgproc/src/convhull.cpp`).
//!
//! # Why the follower is ported line for line
//!
//! Where a contour *starts* and which way it runs decide the answer downstream, not just its
//! shape: `approxPolyDP` on a closed curve seeds its split from the contour's first point, so the
//! same outline traced from another pixel can reduce to a different polygon. So the scan order,
//! the clockwise search for the second pixel, the counter-clockwise walk, the marks it leaves
//! (which decide what counts as inside an already-found outline) and the simple chain's rule for
//! which points it keeps are all OpenCV's. `imageproc::contours::find_contours` implements the same
//! paper but returns every border pixel from its own start, so it was not used.
//!
//! # The order contours come back in
//!
//! Reverse discovery: each new outline is inserted at the head of its parent's child list, so the
//! last one found, the lowest in the image, is first. Frozen Python's stable sort by area keeps
//! that order among equal areas, so it is part of the answer too.

/// A contour point, in the image's pixel coordinates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

/// `icvCodeDeltas`: the eight chain directions, counter-clockwise from +x with y pointing down.
const CODE_DELTAS: [(i32, i32); 8] = [
    (1, 0),
    (1, -1),
    (0, -1),
    (-1, -1),
    (-1, 0),
    (-1, 1),
    (0, 1),
    (1, 1),
];

/// The mark a followed border pixel gets (`nbd`), and its "right bound" form (`nbd | -128`).
const NBD: i8 = 2;
const NBD_RIGHT: i8 = NBD | -128;

/// Which borders the scan follows: `RETR_EXTERNAL` (mode 0) or `RETR_LIST` (mode 1).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Retrieval {
    External,
    List,
}

/// The outer contours of every non-zero region, simple-chain compressed (only the ends of each
/// horizontal, vertical or diagonal run are kept), in OpenCV's order.
pub fn find_external_contours(src: &[u8], width: usize, height: usize) -> Vec<Vec<Point>> {
    find_contours(src, width, height, Retrieval::External)
}

/// `cv2.findContours(..., RETR_LIST, CHAIN_APPROX_SIMPLE)`: **every** border, outer and hole, of
/// every region at any depth, simple-chain compressed, in OpenCV's order. The text detector's
/// postprocessing traces its thresholded map this way (`DBPostProcess.boxes_from_bitmap`), so a
/// hole in a blob is a candidate box of its own. [M29 P4]
///
/// `RETR_LIST` is `RETR_EXTERNAL` with two of the scan's skips removed: a hole border (a
/// background pixel after an unmarked or left-marked foreground one) is followed from the
/// foreground pixel left of it, its first neighbour searched clockwise from the right rather than
/// the left, and an outer border inside an outline already found is followed rather than passed
/// over. The mark (`nbd`) stays 2 throughout, because OpenCV numbers borders only in the modes
/// that build a hierarchy. Every contour hangs off the frame, so the order is still reverse
/// discovery.
pub fn find_list_contours(src: &[u8], width: usize, height: usize) -> Vec<Vec<Point>> {
    find_contours(src, width, height, Retrieval::List)
}

/// `cvFindNextContour`'s scan, run to the end, for either mode.
fn find_contours(src: &[u8], width: usize, height: usize, mode: Retrieval) -> Vec<Vec<Point>> {
    assert_eq!(src.len(), width * height);
    // OpenCV pads by one zero pixel on every side so a region touching the edge still has an
    // outside, then works on 0/1 bytes it marks as it goes.
    let step = width + 2;
    let rows = height + 2;
    let mut img = vec![0i8; step * rows];
    for y in 0..height {
        for x in 0..width {
            if src[y * width + x] != 0 {
                img[(y + 1) * step + x + 1] = 1;
            }
        }
    }
    let deltas: [isize; 16] = {
        let s = step as isize;
        let base = [1, -s + 1, -s, -s - 1, -1, s - 1, s, s + 1];
        let mut all = [0isize; 16];
        all[..8].copy_from_slice(&base);
        all[8..].copy_from_slice(&base);
        all
    };

    let mut found = Vec::new();
    // `img_size` is the padded size less one: the scan never reaches the last row or column.
    let (scan_width, scan_height) = (step - 1, rows - 1);
    for y in 1..scan_height {
        // `lnbd`: the last border pixel met on this row, starting at the frame's left edge.
        let mut lnbd_x = 0usize;
        let mut prev: i8 = 0;
        let mut x = 1usize;
        while x < scan_width {
            let mut p = img[y * step + x];
            while x < scan_width && p == prev {
                x += 1;
                if x < scan_width {
                    p = img[y * step + x];
                }
            }
            if x >= scan_width {
                break;
            }
            let outer_start = prev == 0 && p == 1;
            // A hole border: background after a foreground pixel no follow has marked as a
            // right side (`prev < 1` is the right-side mark, `nbd | -128`, or background).
            let hole_start = !outer_start && p == 0 && prev >= 1;
            let skip = if !outer_start && !hole_start {
                true
            } else {
                if hole_start && (prev & -2) != 0 {
                    lnbd_x = x - 1;
                }
                // RETR_EXTERNAL follows no hole, and no outer border whose last border to the
                // left is a left side (marked positive): that region is inside one already found.
                mode == Retrieval::External && (hole_start || img[y * step + lnbd_x] > 0)
            };
            if skip {
                prev = p;
                if (prev & -2) != 0 {
                    lnbd_x = x;
                }
                x += 1;
                continue;
            }
            // A hole's border is the foreground pixel before it.
            let start_x = x - usize::from(hole_start);
            let contour = follow(
                &mut img,
                &deltas,
                y * step + start_x,
                Point {
                    x: start_x as i32,
                    y: y as i32,
                },
                hole_start,
            );
            found.push(
                contour
                    .into_iter()
                    .map(|p| Point {
                        x: p.x - 1,
                        y: p.y - 1,
                    })
                    .collect(),
            );
            lnbd_x = start_x;
            // The scan resumes after the pixel that started it: for an outer border the start,
            // which the follow has just marked, and for a hole the background pixel after it.
            prev = img[y * step + x];
            x += 1;
        }
    }
    found.reverse();
    found
}

/// `icvFetchContour` with the simple chain: follow a border from `start`, mark it, and keep a
/// point wherever the direction changes.
fn follow(
    img: &mut [i8],
    deltas: &[isize; 16],
    start: usize,
    origin: Point,
    hole: bool,
) -> Vec<Point> {
    let mut points = Vec::new();
    let mut pt = origin;
    let at = |index: usize, delta: isize| -> usize { (index as isize + delta) as usize };

    // Clockwise for the first other pixel of the region: from the left neighbour on an outer
    // border, from the right one (where the hole is) on a hole's.
    let s_end_initial = if hole { 0usize } else { 4usize };
    let mut s = s_end_initial;
    let mut i1;
    loop {
        s = (s + 7) & 7;
        i1 = at(start, deltas[s]);
        if img[i1] != 0 || s == s_end_initial {
            break;
        }
    }
    if s == s_end_initial {
        // A single pixel.
        img[start] = NBD_RIGHT;
        points.push(pt);
        return points;
    }

    let mut i3 = start;
    let mut prev_s = s ^ 4;
    loop {
        let s_end = s;
        let mut i4 = i3;
        // Counter-clockwise from the direction we arrived along, for the next border pixel.
        while s < 15 {
            s += 1;
            i4 = at(i3, deltas[s]);
            if img[i4] != 0 {
                break;
            }
        }
        s &= 7;
        // `(unsigned)(s - 1) < (unsigned)s_end`: true for 1 <= s <= s_end, the wrap making s == 0
        // fail.
        if ((s as i32 - 1) as u32) < (s_end as u32) {
            img[i3] = NBD_RIGHT;
        } else if img[i3] == 1 {
            img[i3] = NBD;
        }
        if s != prev_s {
            points.push(pt);
            prev_s = s;
        }
        pt.x += CODE_DELTAS[s].0;
        pt.y += CODE_DELTAS[s].1;
        if i4 == start && i3 == i1 {
            break;
        }
        i3 = i4;
        s = (s + 4) & 7;
    }
    points
}

/// OpenCV reads an integer contour as `Point2f` before measuring it, so the integer measurements
/// are the float ones on the same values.
fn as_f32(contour: &[Point]) -> Vec<[f32; 2]> {
    contour.iter().map(|p| [p.x as f32, p.y as f32]).collect()
}

/// `contourArea(contour)`: the shoelace sum in `f64`, halved, unsigned. Exact on integer points.
pub fn contour_area(contour: &[Point]) -> f64 {
    polygon_area(&as_f32(contour))
}

/// `contourArea` on `float32` points, the `is_float` branch: each product widened to `f64`
/// before it is taken. The text detector's unclip measures its four box corners this way.
pub fn polygon_area(points: &[[f32; 2]]) -> f64 {
    let Some(&last) = points.last() else {
        return 0.0;
    };
    let mut prev = last;
    let mut a00 = 0.0f64;
    for &point in points {
        a00 += f64::from(prev[0]) * f64::from(point[1]) - f64::from(prev[1]) * f64::from(point[0]);
        prev = point;
    }
    (a00 * 0.5).abs()
}

/// `arcLength(contour, closed=True)`: each side's length in `f32` (`std::sqrt` on a float),
/// summed in `f64`.
pub fn arc_length(contour: &[Point]) -> f64 {
    polygon_perimeter(&as_f32(contour))
}

/// `arcLength(points, closed=True)` on `float32` points: the same `f32` sides and `f64` sum.
pub fn polygon_perimeter(points: &[[f32; 2]]) -> f64 {
    if points.len() <= 1 {
        return 0.0;
    }
    let mut prev = points[points.len() - 1];
    let mut perimeter = 0.0f64;
    for &point in points {
        let (dx, dy) = (point[0] - prev[0], point[1] - prev[1]);
        perimeter += f64::from((dx * dx + dy * dy).sqrt());
        prev = point;
    }
    perimeter
}

/// `approxPolyDP(contour, epsilon, closed=True)` on integer points: Douglas-Peucker seeded, for a
/// closed curve, from two roughly farthest points found by three passes starting at point 0, then
/// a clean-up pass that drops nearly collinear points.
pub fn approx_poly_dp(contour: &[Point], epsilon: f64) -> Vec<Point> {
    let count = contour.len();
    if count == 0 {
        return Vec::new();
    }
    let eps = epsilon * epsilon;
    let mut dst: Vec<Point> = Vec::with_capacity(count);
    let mut stack: Vec<(usize, usize)> = Vec::new();
    let read = |pos: &mut usize| -> Point {
        let p = contour[*pos];
        *pos += 1;
        if *pos >= count {
            *pos = 0;
        }
        p
    };

    // 1. Two approximately farthest points.
    let mut pos = 0usize;
    let mut right_start = 0usize;
    let mut start_pt = Point {
        x: -1_000_000,
        y: -1_000_000,
    };
    let mut le_eps = false;
    for _ in 0..3 {
        pos = (pos + right_start) % count;
        start_pt = read(&mut pos);
        let mut max_dist = 0.0f64;
        for j in 1..count {
            let pt = read(&mut pos);
            let dx = f64::from(pt.x - start_pt.x);
            let dy = f64::from(pt.y - start_pt.y);
            let dist = dx * dx + dy * dy;
            if dist > max_dist {
                max_dist = dist;
                right_start = j;
            }
        }
        le_eps = max_dist <= eps;
    }

    // 2. The two halves on the stack.
    if !le_eps {
        let slice_start = pos % count;
        let right_end = slice_start;
        let slice_end = (right_start + slice_start) % count;
        stack.push((slice_end, right_end));
        stack.push((slice_start, slice_end));
    } else {
        dst.push(start_pt);
    }

    // 3. Split each slice at its farthest point until every point is within epsilon.
    while let Some((slice_start, slice_end)) = stack.pop() {
        let end_pt = contour[slice_end];
        let mut pos = slice_start;
        let mut start_pt = read(&mut pos);
        let mut split_at = 0usize;
        let le_eps;
        if pos != slice_end {
            let dx = f64::from(end_pt.x - start_pt.x);
            let dy = f64::from(end_pt.y - start_pt.y);
            assert!(dx != 0.0 || dy != 0.0, "a slice with coincident ends");
            let mut max_dist = 0.0f64;
            while pos != slice_end {
                let pt = read(&mut pos);
                let dist =
                    (f64::from(pt.y - start_pt.y) * dx - f64::from(pt.x - start_pt.x) * dy).abs();
                if dist > max_dist {
                    max_dist = dist;
                    split_at = (pos + count - 1) % count;
                }
            }
            le_eps = max_dist * max_dist <= eps * (dx * dx + dy * dy);
        } else {
            le_eps = true;
            start_pt = contour[slice_start];
        }
        if le_eps {
            dst.push(start_pt);
        } else {
            stack.push((split_at, slice_end));
            stack.push((slice_start, split_at));
        }
    }

    // 4. Drop points on nearly straight runs.
    let count = dst.len();
    let mut new_count = count;
    let mut pos = count - 1;
    let read_dst = |dst: &[Point], pos: &mut usize| -> Point {
        let p = dst[*pos];
        *pos += 1;
        if *pos >= count {
            *pos = 0;
        }
        p
    };
    let mut start_pt = read_dst(&dst, &mut pos);
    let mut wpos = pos;
    let mut pt = read_dst(&dst, &mut pos);
    let mut i = 0usize;
    while i < count && new_count > 2 {
        let end_pt = read_dst(&dst, &mut pos);
        let dx = f64::from(end_pt.x - start_pt.x);
        let dy = f64::from(end_pt.y - start_pt.y);
        let dist = (f64::from(pt.x - start_pt.x) * dy - f64::from(pt.y - start_pt.y) * dx).abs();
        let successive_inner_product = f64::from(pt.x - start_pt.x) * f64::from(end_pt.x - pt.x)
            + f64::from(pt.y - start_pt.y) * f64::from(end_pt.y - pt.y);
        if dist * dist <= 0.5 * eps * (dx * dx + dy * dy)
            && dx != 0.0
            && dy != 0.0
            && successive_inner_product >= 0.0
        {
            new_count -= 1;
            start_pt = end_pt;
            dst[wpos] = end_pt;
            wpos += 1;
            if wpos >= count {
                wpos = 0;
            }
            pt = read_dst(&dst, &mut pos);
            i += 2;
            continue;
        }
        start_pt = pt;
        dst[wpos] = pt;
        wpos += 1;
        if wpos >= count {
            wpos = 0;
        }
        pt = end_pt;
        i += 1;
    }
    dst.truncate(new_count);
    dst
}

/// `isContourConvex` on integer points: every turn the same way. A straight or doubled-back
/// corner (a zero cross product) makes it not convex, as in OpenCV.
pub fn is_contour_convex(contour: &[Point]) -> bool {
    let n = contour.len();
    if n == 0 {
        return false;
    }
    let mut prev_pt = contour[(2 * n - 2) % n];
    let mut cur_pt = contour[n - 1];
    let mut dx0 = i64::from(cur_pt.x - prev_pt.x);
    let mut dy0 = i64::from(cur_pt.y - prev_pt.y);
    let mut orientation = 0;
    for &p in contour {
        prev_pt = cur_pt;
        cur_pt = p;
        let dx = i64::from(cur_pt.x - prev_pt.x);
        let dy = i64::from(cur_pt.y - prev_pt.y);
        let dxdy0 = dx * dy0;
        let dydx0 = dy * dx0;
        orientation |= if dydx0 > dxdy0 {
            1
        } else if dydx0 < dxdy0 {
            2
        } else {
            3
        };
        if orientation == 3 {
            return false;
        }
        dx0 = dx;
        dy0 = dy;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pts(list: &[(i32, i32)]) -> Vec<Point> {
        list.iter().map(|&(x, y)| Point { x, y }).collect()
    }

    fn image(rows: &[&str]) -> (Vec<u8>, usize, usize) {
        let width = rows[0].len();
        let data = rows
            .iter()
            .flat_map(|row| row.bytes().map(|b| if b == b'#' { 255 } else { 0 }))
            .collect();
        (data, width, rows.len())
    }

    #[test]
    fn a_filled_rectangle_keeps_its_four_corners_counter_clockwise_from_the_top_left() {
        let (img, w, h) = image(&["......", ".####.", ".####.", ".####.", "......"]);
        let contours = find_external_contours(&img, w, h);
        assert_eq!(contours, vec![pts(&[(1, 1), (1, 3), (4, 3), (4, 1)])]);
        assert_eq!(contour_area(&contours[0]), 6.0);
        assert_eq!(arc_length(&contours[0]), 10.0);
    }

    #[test]
    fn a_region_on_the_image_edge_is_still_traced() {
        let (img, w, h) = image(&["###", "###"]);
        assert_eq!(
            find_external_contours(&img, w, h),
            vec![pts(&[(0, 0), (0, 1), (2, 1), (2, 0)])]
        );
    }

    #[test]
    fn holes_and_what_sits_in_them_are_not_external() {
        let (img, w, h) = image(&[
            "#######", "#.....#", "#..#..#", "#.....#", "#######", ".......", "..##...",
        ]);
        let contours = find_external_contours(&img, w, h);
        // The ring and the blob below it, the lower one first; the dot inside the ring is not
        // external.
        assert_eq!(contours.len(), 2);
        assert_eq!(contours[0], pts(&[(2, 6), (3, 6)]));
        assert_eq!(contours[1], pts(&[(0, 0), (0, 4), (6, 4), (6, 0)]));
    }

    #[test]
    fn a_list_scan_also_follows_the_hole_and_what_sits_in_it() {
        let (img, w, h) = image(&[
            "#######", "#.....#", "#..#..#", "#.....#", "#######", ".......", "..##...",
        ]);
        // Discovered top to bottom: the ring's outside, its hole (traced along the ring's own
        // pixels, from the one left of the first background pixel, clockwise), the dot, the blob.
        // Returned in reverse. Each list is `cv2.findContours`' own answer (OpenCV 4.10).
        assert_eq!(
            find_list_contours(&img, w, h),
            vec![
                pts(&[(2, 6), (3, 6)]),
                pts(&[(3, 2)]),
                pts(&[
                    (0, 1),
                    (1, 0),
                    (5, 0),
                    (6, 1),
                    (6, 3),
                    (5, 4),
                    (1, 4),
                    (0, 3)
                ]),
                pts(&[(0, 0), (0, 4), (6, 4), (6, 0)]),
            ]
        );
        assert_eq!(find_external_contours(&img, w, h).len(), 2);
    }

    #[test]
    fn a_list_scan_follows_a_blob_inside_a_hole_away_from_the_edge() {
        let (img, w, h) = image(&[
            "........", ".######.", ".#....#.", ".#.##.#.", ".#.##.#.", ".#....#.", ".######.",
            "........",
        ]);
        assert_eq!(
            find_list_contours(&img, w, h),
            vec![
                pts(&[(3, 3), (3, 4), (4, 4), (4, 3)]),
                pts(&[
                    (1, 2),
                    (2, 1),
                    (5, 1),
                    (6, 2),
                    (6, 5),
                    (5, 6),
                    (2, 6),
                    (1, 5)
                ]),
                pts(&[(1, 1), (1, 6), (6, 6), (6, 1)]),
            ]
        );
    }

    #[test]
    fn float_corners_measure_as_their_integer_twins_do() {
        let square = pts(&[(1, 1), (1, 3), (4, 3), (4, 1)]);
        let floats: Vec<[f32; 2]> = square.iter().map(|p| [p.x as f32, p.y as f32]).collect();
        assert_eq!(polygon_area(&floats), contour_area(&square));
        assert_eq!(polygon_perimeter(&floats), arc_length(&square));
        // A half-pixel box: 2.5 x 1.5.
        let half = [[0.5, 0.5], [3.0, 0.5], [3.0, 2.0], [0.5, 2.0]];
        assert_eq!(polygon_area(&half), 3.75);
        assert_eq!(polygon_perimeter(&half), 8.0);
    }

    #[test]
    fn a_single_pixel_is_one_point() {
        let (img, w, h) = image(&["...", ".#.", "..."]);
        assert_eq!(find_external_contours(&img, w, h), vec![pts(&[(1, 1)])]);
    }

    #[test]
    fn a_square_reduces_to_its_corners_and_is_convex() {
        // Every pixel on a square's boundary, as a contour without compression would hold them.
        let mut square = Vec::new();
        for y in 0..10 {
            square.push((0, y));
        }
        for x in 0..10 {
            square.push((x, 10));
        }
        for y in (1..=10).rev() {
            square.push((10, y));
        }
        for x in (1..=10).rev() {
            square.push((x, 0));
        }
        let square = pts(&square);
        let approx = approx_poly_dp(&square, 0.02 * arc_length(&square));
        assert_eq!(approx.len(), 4);
        assert!(is_contour_convex(&approx));
        assert_eq!(contour_area(&approx), 100.0);
    }

    #[test]
    fn convexity_fails_on_a_dent_and_on_a_straight_corner() {
        assert!(is_contour_convex(&pts(&[(0, 0), (0, 4), (4, 4), (4, 0)])));
        assert!(!is_contour_convex(&pts(&[
            (0, 0),
            (0, 4),
            (2, 2),
            (4, 4),
            (4, 0)
        ])));
        assert!(!is_contour_convex(&pts(&[
            (0, 0),
            (0, 2),
            (0, 4),
            (4, 4),
            (4, 0)
        ])));
    }
}
