//! `cv2.Canny(image, low, high)` with OpenCV's defaults: a 3x3 Sobel and the L1 gradient.
//! [M29 P2]
//!
//! The twin of OpenCV 4.10's `Canny` and its `parallelCanny` and `finalPass` bodies
//! (`imgproc/src/canny.cpp`). It is integer arithmetic throughout, so it is exact: the gradients
//! are `Sobel(..., CV_16S, ksize=3, BORDER_REPLICATE)`, the magnitude is `|dx| + |dy|`, the
//! non-maximum suppression picks its neighbours by OpenCV's fixed-point tangent test, and the
//! hysteresis keeps every candidate connected to a strong pixel through 8-neighbours. OpenCV
//! splits the rows across threads and stitches the hysteresis at the seams afterwards; the edge
//! map is the same as one pass over the whole image, which is what this is.
//!
//! `imageproc::edges::canny` was not used: it blurs with a Gaussian first and quantises the
//! gradient angle rather than testing tangents, so its edges are not OpenCV's.

/// `tan(22.5 degrees)` in 15-bit fixed point, as `canny.cpp`'s `TG22`.
const TG22: i32 = 13573;

/// Sobel's 3x3 derivatives with the edge replicated, as `i16` (they reach at most 4 * 255).
fn sobel(src: &[u8], width: usize, height: usize) -> (Vec<i16>, Vec<i16>) {
    let at = |x: isize, y: isize| -> i32 {
        let x = x.clamp(0, width as isize - 1) as usize;
        let y = y.clamp(0, height as isize - 1) as usize;
        i32::from(src[y * width + x])
    };
    let mut dx = vec![0i16; width * height];
    let mut dy = vec![0i16; width * height];
    for y in 0..height as isize {
        for x in 0..width as isize {
            let gx = (at(x + 1, y - 1) + 2 * at(x + 1, y) + at(x + 1, y + 1))
                - (at(x - 1, y - 1) + 2 * at(x - 1, y) + at(x - 1, y + 1));
            let gy = (at(x - 1, y + 1) + 2 * at(x, y + 1) + at(x + 1, y + 1))
                - (at(x - 1, y - 1) + 2 * at(x, y - 1) + at(x + 1, y - 1));
            let i = y as usize * width + x as usize;
            dx[i] = gx as i16;
            dy[i] = gy as i16;
        }
    }
    (dx, dy)
}

/// Edge map of an 8-bit single-channel image: 255 on an edge, 0 elsewhere. The thresholds are
/// swapped if given high-first and floored to integers, as OpenCV does; a pixel is a candidate
/// above `low` and strong above `high`.
pub fn canny(src: &[u8], width: usize, height: usize, low: f64, high: f64) -> Vec<u8> {
    assert_eq!(src.len(), width * height);
    let (low, high) = if low > high { (high, low) } else { (low, high) };
    let (low, high) = (low.floor() as i32, high.floor() as i32);
    let (dx, dy) = sobel(src, width, height);
    let magnitude: Vec<i32> = dx
        .iter()
        .zip(&dy)
        .map(|(&x, &y)| i32::from(x).abs() + i32::from(y).abs())
        .collect();
    // The magnitude outside the image is 0, as OpenCV's ring buffer is zeroed past each edge.
    let mag = |x: isize, y: isize| -> i32 {
        if x < 0 || y < 0 || x >= width as isize || y >= height as isize {
            0
        } else {
            magnitude[y as usize * width + x as usize]
        }
    };

    // The map with a one-pixel frame: 0 a candidate, 1 not an edge, 2 an edge.
    let map_width = width + 2;
    let mut map = vec![1u8; map_width * (height + 2)];
    let mut stack: Vec<usize> = Vec::new();
    for y in 0..height {
        for x in 0..width {
            let i = y * width + x;
            let m = magnitude[i];
            let cell = (y + 1) * map_width + x + 1;
            if m <= low {
                continue;
            }
            let (xs, ys) = (dx[i], dy[i]);
            let ax = i32::from(xs).abs();
            let ay = i32::from(ys).abs() << 15;
            let tg22x = ax * TG22;
            let (xi, yi) = (x as isize, y as isize);
            let is_peak = if ay < tg22x {
                m > mag(xi - 1, yi) && m >= mag(xi + 1, yi)
            } else {
                let tg67x = tg22x + (ax << 16);
                if ay > tg67x {
                    m > mag(xi, yi - 1) && m >= mag(xi, yi + 1)
                } else {
                    let s: isize = if (xs ^ ys) < 0 { -1 } else { 1 };
                    m > mag(xi - s, yi - 1) && m > mag(xi + s, yi + 1)
                }
            };
            if is_peak {
                if m > high {
                    map[cell] = 2;
                    stack.push(cell);
                } else {
                    map[cell] = 0;
                }
            }
        }
    }

    // Hysteresis: grow every strong pixel through the candidates around it.
    while let Some(cell) = stack.pop() {
        for neighbour in [
            cell - map_width - 1,
            cell - map_width,
            cell - map_width + 1,
            cell - 1,
            cell + 1,
            cell + map_width - 1,
            cell + map_width,
            cell + map_width + 1,
        ] {
            if map[neighbour] == 0 {
                map[neighbour] = 2;
                stack.push(neighbour);
            }
        }
    }

    let mut dst = vec![0u8; width * height];
    for y in 0..height {
        for x in 0..width {
            if map[(y + 1) * map_width + x + 1] == 2 {
                dst[y * width + x] = 255;
            }
        }
    }
    dst
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_vertical_step_gives_one_thin_edge() {
        // Left half 0, right half 200: dx peaks on the two columns either side of the step, and
        // non-maximum suppression keeps the first of the tied pair (`m > left && m >= right`).
        let (w, h) = (10, 6);
        let mut img = vec![0u8; w * h];
        for y in 0..h {
            for x in 5..w {
                img[y * w + x] = 200;
            }
        }
        let edges = canny(&img, w, h, 30.0, 120.0);
        for y in 0..h {
            let row: Vec<u8> = edges[y * w..(y + 1) * w].to_vec();
            assert_eq!(row, vec![0, 0, 0, 0, 255, 0, 0, 0, 0, 0], "row {y}");
        }
    }

    #[test]
    fn a_weak_edge_survives_only_when_joined_to_a_strong_one() {
        // A vertical step whose contrast fades down the image: strong at the top, weak below.
        let (w, h) = (8, 8);
        let mut img = vec![0u8; w * h];
        for y in 0..h {
            let level = if y < 3 { 100 } else { 10 };
            for x in 4..w {
                img[y * w + x] = level;
            }
        }
        // Strong rows have magnitude 400, weak ones 40: candidates above 30, strong above 120.
        // Where the contrast changes the gradient turns diagonal and the edge steps one column
        // right for two rows; the weak rows below are kept because they touch it.
        let joined = canny(&img, w, h, 30.0, 120.0);
        assert!(
            (0..h).all(|y| joined[y * w + 3] == 255 || joined[y * w + 4] == 255),
            "{joined:?}"
        );
        assert!((4..h).all(|y| joined[y * w + 3] == 255), "{joined:?}");
        // With the strong part gone, nothing is above `high`, and nothing survives.
        let mut faint = img.clone();
        for v in faint.iter_mut() {
            *v = (*v).min(10);
        }
        assert!(canny(&faint, w, h, 30.0, 120.0).iter().all(|&v| v == 0));
    }

    #[test]
    fn thresholds_given_high_first_are_swapped() {
        let (w, h) = (6, 4);
        let mut img = vec![0u8; w * h];
        for y in 0..h {
            for x in 3..w {
                img[y * w + x] = 90;
            }
        }
        assert_eq!(
            canny(&img, w, h, 120.0, 30.0),
            canny(&img, w, h, 30.0, 120.0)
        );
    }
}
