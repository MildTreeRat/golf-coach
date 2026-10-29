//! `cv2.morphologyEx(image, cv2.MORPH_CLOSE, getStructuringElement(MORPH_RECT, (k, k)))`.
//! [M29 P2]
//!
//! The twin of OpenCV 4.10's `morphologyEx` for `MORPH_CLOSE` with a full rectangle
//! (`imgproc/src/morph.dispatch.cpp`): a dilation, then an erosion, each over the `k`x`k` window
//! centred on the pixel (anchor `k / 2`). With the default border value OpenCV treats outside the
//! image as the identity of each step, so it never wins: the minimum (0) while dilating and the
//! maximum (255) while eroding. A rectangle is separable, so each step is a row pass then a column
//! pass, which gives the same extremum as the whole window and is exact.

/// The extremum of each `k`-wide window along one axis, ignoring what lies outside the image.
fn pass(src: &[u8], width: usize, height: usize, k: usize, along_rows: bool, max: bool) -> Vec<u8> {
    let anchor = k / 2;
    let mut dst = vec![0u8; src.len()];
    let (outer, inner) = if along_rows {
        (height, width)
    } else {
        (width, height)
    };
    let index = |o: usize, i: usize| {
        if along_rows {
            o * width + i
        } else {
            i * width + o
        }
    };
    for o in 0..outer {
        for i in 0..inner {
            let start = i.saturating_sub(anchor);
            let end = (i + k - anchor).min(inner);
            let mut best = if max { 0 } else { 255 };
            for j in start..end {
                let v = src[index(o, j)];
                best = if max { best.max(v) } else { best.min(v) };
            }
            dst[index(o, i)] = best;
        }
    }
    dst
}

/// Closing by a `k`x`k` rectangle: fills gaps narrower than the kernel in a binary edge map.
pub fn close_rect(src: &[u8], width: usize, height: usize, k: usize) -> Vec<u8> {
    assert_eq!(src.len(), width * height);
    assert!(k >= 1);
    let dilated = pass(
        &pass(src, width, height, k, true, true),
        width,
        height,
        k,
        false,
        true,
    );
    pass(
        &pass(&dilated, width, height, k, true, false),
        width,
        height,
        k,
        false,
        false,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closing_bridges_a_gap_narrower_than_the_kernel() {
        // A horizontal line with a two-pixel gap, in a 9x3 image.
        let mut img = vec![0u8; 9 * 3];
        for x in [0, 1, 2, 5, 6, 7, 8] {
            img[9 + x] = 255;
        }
        let closed = close_rect(&img, 9, 3, 3);
        assert_eq!(&closed[9..18], &[255; 9]);
        // Too wide a gap for a 3x3 kernel stays open.
        let mut wide = vec![0u8; 9 * 3];
        for x in [0, 1, 5, 6, 7, 8] {
            wide[9 + x] = 255;
        }
        assert_eq!(close_rect(&wide, 9, 3, 3)[9 + 3], 0);
    }

    #[test]
    fn the_border_never_erodes_what_touches_it() {
        // With 255 outside while eroding, a block on the image edge keeps its edge pixels.
        let mut img = vec![0u8; 6 * 6];
        for y in 0..3 {
            for x in 0..3 {
                img[y * 6 + x] = 255;
            }
        }
        assert_eq!(close_rect(&img, 6, 6, 5), img);
    }
}
