//! `cv2.fillPoly` for one polygon on an 8-bit image, and `cv2.mean(src, mask)` on `float32`: the
//! text detector's box score. [M29 P4]
//!
//! `DBPostProcess.box_score_fast` fills a candidate box into a mask and averages the probability
//! map under it, and the average decides whether the box is kept (`box_thresh`). Twins of OpenCV
//! 4.10's `fillPoly`, which is `CollectPolyEdges` and `FillEdgeCollection` over its own `Line`,
//! `LineIterator` and `clipLine` (`imgproc/src/drawing.cpp`), and of `cv::mean`'s masked loop
//! (`core/src/mean.dispatch.cpp` with `sum_` from `core/src/sum.simd.hpp`).
//!
//! # Why the outline is drawn as well as filled
//!
//! For the solid line types `CollectPolyEdges` draws every edge with an 8-connected Bresenham line
//! before the scanline fill runs, and the two do not cover the same pixels: the fill's spans come
//! from fixed-point edge positions, the line's from integer error terms, so along a slanted edge
//! the line can reach a pixel the fill stops short of. The mask is their union, so both are
//! ported.
//!
//! # The mean, and IPP
//!
//! OpenCV's own masked mean adds each selected `f32` into an `f64` in row order and multiplies
//! by the reciprocal of the count, which is what [`mean_masked`] does. The pip wheel's `cv2.mean`
//! runs Intel IPP's `ippiMean_32f_C1MR` instead, whose summation order is its own. `tests/det.rs`
//! records how far apart the two land on the bay photos' boxes: far below anything that moves a
//! box across `box_thresh`.

const XY_SHIFT: u32 = 16;
const XY_ONE: i64 = 1 << XY_SHIFT;

/// `PolyEdge`: an edge's rows `[y0, y1)`, its `x` at the current row and its slope, both in
/// 16.16 fixed point, and the active list's link.
#[derive(Clone, Copy, Debug)]
struct PolyEdge {
    y0: i32,
    y1: i32,
    x: i64,
    dx: i64,
    next: usize,
}

/// The null link.
const NONE: usize = usize::MAX;

/// `clipLine(Size2l, Point2l&, Point2l&)`: Cohen-Sutherland against `[0, w) x [0, h)`, moving each
/// end onto the edge it crosses with the `double` arithmetic OpenCV uses. False when the segment
/// misses the image.
fn clip_line(width: i64, height: i64, p1: &mut [i64; 2], p2: &mut [i64; 2]) -> bool {
    if width <= 0 || height <= 0 {
        return false;
    }
    let (right, bottom) = (width - 1, height - 1);
    let code = |p: &[i64; 2]| -> i32 {
        i32::from(p[0] < 0)
            + i32::from(p[0] > right) * 2
            + i32::from(p[1] < 0) * 4
            + i32::from(p[1] > bottom) * 8
    };
    let (mut c1, mut c2) = (code(p1), code(p2));
    if (c1 & c2) == 0 && (c1 | c2) != 0 {
        if c1 & 12 != 0 {
            let a = if c1 < 8 { 0 } else { bottom };
            p1[0] += ((a - p1[1]) as f64 * (p2[0] - p1[0]) as f64 / (p2[1] - p1[1]) as f64) as i64;
            p1[1] = a;
            c1 = i32::from(p1[0] < 0) + i32::from(p1[0] > right) * 2;
        }
        if c2 & 12 != 0 {
            let a = if c2 < 8 { 0 } else { bottom };
            p2[0] += ((a - p2[1]) as f64 * (p2[0] - p1[0]) as f64 / (p2[1] - p1[1]) as f64) as i64;
            p2[1] = a;
            c2 = i32::from(p2[0] < 0) + i32::from(p2[0] > right) * 2;
        }
        if (c1 & c2) == 0 && (c1 | c2) != 0 {
            if c1 != 0 {
                let a = if c1 == 1 { 0 } else { right };
                p1[1] +=
                    ((a - p1[0]) as f64 * (p2[1] - p1[1]) as f64 / (p2[0] - p1[0]) as f64) as i64;
                p1[0] = a;
                c1 = 0;
            }
            if c2 != 0 {
                let a = if c2 == 1 { 0 } else { right };
                p2[1] +=
                    ((a - p2[0]) as f64 * (p2[1] - p1[1]) as f64 / (p2[0] - p1[0]) as f64) as i64;
                p2[0] = a;
                c2 = 0;
            }
        }
    }
    (c1 | c2) == 0
}

/// `Line(img, pt1, pt2, color, 8)`: `LineIterator` with `leftToRight`, after clipping to the
/// image, one pixel per step along the major axis.
fn line(img: &mut [u8], width: usize, height: usize, p1: [i64; 2], p2: [i64; 2], color: u8) {
    let (mut p1, mut p2) = (p1, p2);
    let outside = |p: &[i64; 2]| {
        (p[0] as u64) >= width as u64 || (p[1] as u64) >= height as u64 || p[0] < 0 || p[1] < 0
    };
    if (outside(&p1) || outside(&p2)) && !clip_line(width as i64, height as i64, &mut p1, &mut p2) {
        return;
    }
    let mut dx = p2[0] - p1[0];
    let mut dy = p2[1] - p1[1];
    if dx < 0 {
        dx = -dx;
        dy = -dy;
        std::mem::swap(&mut p1, &mut p2);
    }
    let (delta_x, mut delta_y) = (1i64, 1i64);
    if dy < 0 {
        dy = -dy;
        delta_y = -1;
    }
    let vertical = dy > dx;
    if vertical {
        std::mem::swap(&mut dx, &mut dy);
    }
    // 8-connected: the major axis moves every step, the minor one when the error goes negative.
    let mut err = dx - (dy + dy);
    let plus_delta = dx + dx;
    let minus_delta = -(dy + dy);
    let (mut x, mut y) = (p1[0], p1[1]);
    for _ in 0..=dx {
        img[y as usize * width + x as usize] = color;
        let minor = err < 0;
        err += minus_delta + if minor { plus_delta } else { 0 };
        if vertical {
            y += delta_y;
            if minor {
                x += delta_x;
            }
        } else {
            x += delta_x;
            if minor {
                y += delta_y;
            }
        }
    }
}

/// `CollectPolyEdges` with `shift = 0` and no offset, for a solid line type: draws each edge and
/// returns the non-horizontal ones, clipped edges started where the clip moved them.
fn collect_poly_edges(
    img: &mut [u8],
    width: usize,
    height: usize,
    points: &[[i32; 2]],
    color: u8,
) -> Vec<PolyEdge> {
    let mut edges = Vec::with_capacity(points.len() + 1);
    let last = points[points.len() - 1];
    let mut pt0 = [i64::from(last[0]) << XY_SHIFT, i64::from(last[1])];
    for &point in points {
        let pt1 = [i64::from(point[0]) << XY_SHIFT, i64::from(point[1])];
        let (mut pt0c, mut pt1c) = (pt0, pt1);
        let mut t0 = [(pt0[0] + (XY_ONE >> 1)) >> XY_SHIFT, pt0[1]];
        let mut t1 = [(pt1[0] + (XY_ONE >> 1)) >> XY_SHIFT, pt1[1]];
        line(img, width, height, t0, t1, color);
        let off =
            |p: &[i64; 2]| p[0] < 0 || p[0] >= width as i64 || p[1] < 0 || p[1] >= height as i64;
        if off(&t0) || off(&t1) {
            // The 4.x correction: an edge that leaves the image starts its spans where it is
            // clipped, so its slope is not taken across the part outside.
            clip_line(width as i64, height as i64, &mut t0, &mut t1);
            if t0[1] != t1[1] {
                pt0c = [t0[0] << XY_SHIFT, t0[1]];
                pt1c = [t1[0] << XY_SHIFT, t1[1]];
            }
        } else {
            pt0c[0] += XY_ONE >> 1;
            pt1c[0] += XY_ONE >> 1;
        }
        if pt0[1] != pt1[1] {
            let dx = (pt1c[0] - pt0c[0]) / (pt1c[1] - pt0c[1]);
            let edge = if pt0[1] < pt1[1] {
                PolyEdge {
                    y0: pt0[1] as i32,
                    y1: pt1[1] as i32,
                    x: pt0c[0] + (pt0[1] - pt0c[1]) * dx,
                    dx,
                    next: NONE,
                }
            } else {
                PolyEdge {
                    y0: pt1[1] as i32,
                    y1: pt0[1] as i32,
                    x: pt1c[0] + (pt1[1] - pt1c[1]) * dx,
                    dx,
                    next: NONE,
                }
            };
            edges.push(edge);
        }
        pt0 = pt1;
    }
    edges
}

/// `FillEdgeCollection` for a solid line type: an active edge list swept down the rows, each pair
/// of crossings filled between them, re-sorted by `x` after every row.
fn fill_edge_collection(
    img: &mut [u8],
    width: usize,
    height: usize,
    mut edges: Vec<PolyEdge>,
    color: u8,
) {
    let total = edges.len();
    if total < 2 {
        return;
    }
    let (mut y_min, mut y_max) = (i32::MAX, i32::MIN);
    // OpenCV starts `x_max` at `0xFFFFFFFFFFFFFFFF`, which as an `int64` is -1.
    let (mut x_min, mut x_max) = (i64::MAX, -1i64);
    for e in &edges {
        let x1 = e.x + i64::from(e.y1 - e.y0) * e.dx;
        y_min = y_min.min(e.y0);
        y_max = y_max.max(e.y1);
        x_min = x_min.min(e.x).min(x1);
        x_max = x_max.max(e.x).max(x1);
    }
    if y_max < 0 || y_min >= height as i32 || x_max < 0 || x_min >= (width as i64) << XY_SHIFT {
        return;
    }
    // `CmpEdges`: by top row, then `x`, then slope. `std::sort` is not stable; edges equal on all
    // three start at one point along one line, and which of those is first fills the same pixels.
    edges.sort_by(|a, b| (a.y0, a.x, a.dx).cmp(&(b.y0, b.x, b.dx)));
    // The sentinel `edges.push_back(tmp)` adds, then the list head `tmp` itself.
    edges.push(PolyEdge {
        y0: i32::MAX,
        y1: 0,
        x: 0,
        dx: 0,
        next: NONE,
    });
    let head = edges.len();
    edges.push(PolyEdge {
        y0: i32::MAX,
        y1: 0,
        x: 0,
        dx: 0,
        next: NONE,
    });

    let mut i = 0usize;
    let mut e = 0usize;
    let y_max = y_max.min(height as i32);
    let mut y = edges[e].y0;
    while y < y_max {
        let mut draw = false;
        let clipline = y < 0;
        let mut prelast = head;
        let mut last = edges[head].next;
        while last != NONE || edges[e].y0 == y {
            if last != NONE && edges[last].y1 == y {
                // The edge ends on this row: drop it from the active list.
                edges[prelast].next = edges[last].next;
                last = edges[last].next;
                continue;
            }
            let keep_prelast = prelast;
            if last != NONE && (edges[e].y0 > y || edges[last].x < edges[e].x) {
                prelast = last;
                last = edges[last].next;
            } else if i < total {
                // The next edge starts on this row: insert it here.
                edges[prelast].next = e;
                edges[e].next = last;
                prelast = e;
                i += 1;
                e = i;
            } else {
                break;
            }
            if draw {
                if !clipline {
                    let (a, b) = (edges[keep_prelast].x, edges[prelast].x);
                    let (mut x1, mut x2) = if a > b {
                        ((b >> XY_SHIFT) as i32, (a >> XY_SHIFT) as i32)
                    } else {
                        ((a >> XY_SHIFT) as i32, (b >> XY_SHIFT) as i32)
                    };
                    if x1 < width as i32 && x2 >= 0 {
                        x1 = x1.max(0);
                        x2 = x2.min(width as i32 - 1);
                        let row = y as usize * width;
                        img[row + x1 as usize..=row + x2 as usize].fill(color);
                    }
                }
                edges[keep_prelast].x += edges[keep_prelast].dx;
                edges[prelast].x += edges[prelast].dx;
            }
            draw = !draw;
        }

        // Bubble the active list back into `x` order.
        let mut keep_prelast = NONE;
        loop {
            let mut prelast = head;
            let mut last = edges[head].next;
            let mut last_exchange = NONE;
            while last != keep_prelast && last != NONE && edges[last].next != NONE {
                let te = edges[last].next;
                if edges[last].x > edges[te].x {
                    edges[prelast].next = te;
                    edges[last].next = edges[te].next;
                    edges[te].next = last;
                    prelast = te;
                    last_exchange = prelast;
                } else {
                    prelast = last;
                    last = te;
                }
            }
            if last_exchange == NONE {
                break;
            }
            keep_prelast = last_exchange;
            if keep_prelast == edges[head].next || keep_prelast == head {
                break;
            }
        }
        y += 1;
    }
}

/// `cv2.fillPoly(img, [points], color)` with the default line type (8-connected) and no shift:
/// fills one polygon, its outline included, into a `width`x`height` single-channel image. Points
/// outside the image are clipped as OpenCV clips them.
pub fn fill_poly(img: &mut [u8], width: usize, height: usize, points: &[[i32; 2]], color: u8) {
    assert_eq!(img.len(), width * height);
    if points.is_empty() {
        return;
    }
    let edges = collect_poly_edges(img, width, height, points, color);
    fill_edge_collection(img, width, height, edges, color);
}

/// `cv2.mean(src[y0:y0 + h, x0:x0 + w], mask)[0]` on a single-channel `f32` image `src_width`
/// wide, by OpenCV's own loop: the selected values summed in `f64` in row order, times the
/// reciprocal of their count. Zero when the mask selects nothing.
pub fn mean_masked(
    src: &[f32],
    src_width: usize,
    origin: [usize; 2],
    size: [usize; 2],
    mask: &[u8],
) -> f64 {
    let [x0, y0] = origin;
    let [w, h] = size;
    assert_eq!(mask.len(), w * h);
    let mut sum = 0.0f64;
    let mut count = 0usize;
    for y in 0..h {
        let row = &src[(y0 + y) * src_width + x0..(y0 + y) * src_width + x0 + w];
        for (value, &selected) in row.iter().zip(&mask[y * w..(y + 1) * w]) {
            if selected != 0 {
                sum += f64::from(*value);
                count += 1;
            }
        }
    }
    if count == 0 {
        0.0
    } else {
        sum * (1.0 / count as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(width: usize, height: usize, points: &[[i32; 2]]) -> Vec<String> {
        let mut img = vec![0u8; width * height];
        fill_poly(&mut img, width, height, points, 1);
        img.chunks(width)
            .map(|row| {
                row.iter()
                    .map(|&v| if v == 1 { '#' } else { '.' })
                    .collect()
            })
            .collect()
    }

    #[test]
    fn an_axis_aligned_box_fills_its_corners_inclusive() {
        assert_eq!(
            render(6, 5, &[[1, 1], [4, 1], [4, 3], [1, 3]]),
            vec!["......", ".####.", ".####.", ".####.", "......"]
        );
    }

    #[test]
    fn a_box_past_the_edge_is_clipped() {
        assert_eq!(
            render(4, 3, &[[-2, -1], [2, -1], [2, 5], [-2, 5]]),
            vec!["###.", "###.", "###."]
        );
    }

    #[test]
    fn the_mean_counts_only_the_mask() {
        let src = [0.0, 1.0, 2.0, 3.0, 4.0, 5.0];
        // The 2x2 window at (1, 0) of a 3-wide image, masked to its diagonal: 1 and 5.
        assert_eq!(mean_masked(&src, 3, [1, 0], [2, 2], &[1, 0, 0, 1]), 3.0);
        assert_eq!(mean_masked(&src, 3, [0, 0], [1, 1], &[0]), 0.0);
    }
}
