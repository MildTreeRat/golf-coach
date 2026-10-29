//! `cv2.cvtColor(..., COLOR_BGR2GRAY)` and `cv2.bilateralFilter(gray, 9, 75, 75)`, the two steps
//! `find_screen_quad` takes before its edge search. [M29 P2]
//!
//! Twins of OpenCV 4.10's `RGB2Gray<uchar>` (`imgproc/src/color_rgb.simd.hpp`) and
//! `bilateralFilter_8u` with its `BilateralFilter_8u_Invoker` (`imgproc/src/bilateral_filter.*`).
//! The grey conversion is fixed-point and exact. The bilateral filter is OpenCV's own C++ loop,
//! not the Intel IPP kernel the pip wheel routes it to by default: IPP's source is not public, and
//! the two differ by one level on at most one pixel per bay photo (measured with
//! `cv2.ipp.setUseIPP(False)`, M29 P2), with the Canny edges after it identical either way.

/// `COLOR_BGR2GRAY` on 8-bit pixels: `(B*3735 + G*19235 + R*9798 + 2^14) >> 15`, the 0.114,
/// 0.587, 0.299 weights in **15-bit** fixed point. OpenCV 4.x moved the 8-bit path to 15 bits; the
/// older 14-bit weights (1868, 9617, 4899) were tried first and disagree with `cv2` on about 300
/// pixels of every bay photo's 1000 px copy, where a mean lands on a half, while these agree on
/// every pixel of all 13 and of four million random ones (M29 P2). `rgb` is interleaved R, G, B,
/// as `image` holds it.
pub fn rgb_to_gray(rgb: &[u8]) -> Vec<u8> {
    rgb.chunks_exact(3)
        .map(|p| {
            let (r, g, b) = (u32::from(p[0]), u32::from(p[1]), u32::from(p[2]));
            ((b * 3735 + g * 19235 + r * 9798 + (1 << 14)) >> 15) as u8
        })
        .collect()
}

/// OpenCV's `BORDER_REFLECT_101` (`BORDER_DEFAULT`): `gfedcb|abcdefgh|gfedcba`.
fn reflect_101(index: isize, len: usize) -> usize {
    let len = len as isize;
    if len == 1 {
        return 0;
    }
    let mut i = index;
    loop {
        if i < 0 {
            i = -i;
        } else if i >= len {
            i = 2 * len - 2 - i;
        } else {
            return i as usize;
        }
    }
}

/// How many columns of a row the pip wheel's AVX2 kernel covers before its scalar tail: 8 `f32`
/// lanes. The two halves round differently (below), so the split is part of the answer.
const SIMD_LANES: usize = 8;

/// `bilateralFilter(src, d, sigma_color, sigma_space)` on one 8-bit channel, `BORDER_DEFAULT`.
///
/// The kernel is the disc of offsets within `d / 2` of the centre, in OpenCV's row-major order,
/// weighted `exp(-r^2 / 2 sigma_space^2) * exp(-dv^2 / 2 sigma_color^2)` from `f32` tables built in
/// `f64`. Each pixel accumulates `sum += v * w` and `wsum += w` in that order, then takes
/// `round_half_even(sum / wsum)`. **The accumulate is a fused multiply-add** in the columns the
/// AVX2 kernel covers (`v_muladd` is `_mm256_fmadd_ps` there) and a separate multiply and add in
/// the scalar tail, so both are reproduced; on a 1000 px row, the detection width, there is no
/// tail.
pub fn bilateral_filter(
    src: &[u8],
    width: usize,
    height: usize,
    d: i32,
    sigma_color: f64,
    sigma_space: f64,
) -> Vec<u8> {
    assert_eq!(src.len(), width * height);
    let sigma_color = if sigma_color <= 0.0 { 1.0 } else { sigma_color };
    let sigma_space = if sigma_space <= 0.0 { 1.0 } else { sigma_space };
    let gauss_color_coeff = -0.5 / (sigma_color * sigma_color);
    let gauss_space_coeff = -0.5 / (sigma_space * sigma_space);
    let radius = if d <= 0 {
        (sigma_space * 1.5).round_ties_even() as i32
    } else {
        d / 2
    }
    .max(1);

    let color_weight: Vec<f32> = (0..256)
        .map(|i: i32| (f64::from(i * i) * gauss_color_coeff).exp() as f32)
        .collect();
    let mut space: Vec<(isize, isize, f32)> = Vec::new();
    for i in -radius..=radius {
        for j in -radius..=radius {
            let r = (f64::from(i) * f64::from(i) + f64::from(j) * f64::from(j)).sqrt();
            if r > f64::from(radius) {
                continue;
            }
            space.push((
                i as isize,
                j as isize,
                (r * r * gauss_space_coeff).exp() as f32,
            ));
        }
    }

    // The bordered copy OpenCV filters (`copyMakeBorder` by `radius` on every side).
    let r = radius as usize;
    let padded_width = width + 2 * r;
    let mut padded = vec![0u8; padded_width * (height + 2 * r)];
    for py in 0..height + 2 * r {
        let sy = reflect_101(py as isize - r as isize, height);
        for px in 0..padded_width {
            let sx = reflect_101(px as isize - r as isize, width);
            padded[py * padded_width + px] = src[sy * width + sx];
        }
    }

    let simd_end = width - width % SIMD_LANES;
    let mut dst = vec![0u8; width * height];
    let mut sum = vec![0f32; width];
    let mut wsum = vec![0f32; width];
    for y in 0..height {
        sum.iter_mut().for_each(|v| *v = 0.0);
        wsum.iter_mut().for_each(|v| *v = 0.0);
        let centre_row = (y + r) * padded_width + r;
        for &(di, dj, space_weight) in &space {
            let row = ((y + r) as isize + di) as usize * padded_width;
            let col0 = (r as isize + dj) as usize;
            for x in 0..width {
                let centre = i32::from(padded[centre_row + x]);
                let value = i32::from(padded[row + col0 + x]);
                let w = space_weight * color_weight[(value - centre).unsigned_abs() as usize];
                wsum[x] += w;
                sum[x] = if x < simd_end {
                    (value as f32).mul_add(w, sum[x])
                } else {
                    sum[x] + value as f32 * w
                };
            }
        }
        for x in 0..width {
            dst[y * width + x] = (sum[x] / wsum[x]).round_ties_even() as u8;
        }
    }
    dst
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grey_is_fixed_point_with_the_bgr2gray_weights() {
        assert_eq!(rgb_to_gray(&[255, 255, 255, 0, 0, 0]), vec![255, 0]);
        // Pure red, green and blue: 9798, 19235 and 3735 out of 32768, rounded.
        assert_eq!(
            rgb_to_gray(&[255, 0, 0, 0, 255, 0, 0, 0, 255]),
            vec![76, 150, 29]
        );
        // A mean exactly on a half, where the 14-bit weights round down and `cv2` rounds up
        // (B 129, G 156, R 178 is 159.5; a pixel of the bay photo 2026-08-10-1).
        assert_eq!(rgb_to_gray(&[178, 156, 129]), vec![160]);
    }

    #[test]
    fn reflect_101_does_not_repeat_the_edge() {
        let row: Vec<usize> = (-3..8).map(|i| reflect_101(i, 5)).collect();
        assert_eq!(row, vec![3, 2, 1, 0, 1, 2, 3, 4, 3, 2, 1]);
    }

    #[test]
    fn a_flat_image_is_unchanged_and_an_edge_survives() {
        let flat = vec![77u8; 12 * 10];
        assert_eq!(bilateral_filter(&flat, 12, 10, 9, 75.0, 75.0), flat);
        // A hard step much larger than sigma_color stays a step: across it the colour weight is
        // exp(-200^2 / 11250), about 0.03, so each side keeps its own value within a few levels.
        let mut step = vec![20u8; 16 * 4];
        for y in 0..4 {
            for x in 8..16 {
                step[y * 16 + x] = 220;
            }
        }
        let out = bilateral_filter(&step, 16, 4, 9, 75.0, 75.0);
        assert!(out[0] < 30 && out[7] < 40, "{:?}", &out[..16]);
        assert!(out[8] > 200 && out[15] > 210, "{:?}", &out[..16]);
    }
}
