//! `cv2.resize(..., interpolation=cv2.INTER_AREA)` for a shrink, and the size frozen Python asks
//! it for [M29 P2]; and `cv2.resize`'s default bilinear, which the text detector resizes with
//! ([`resize_linear`], M29 P4).
//!
//! The twin of OpenCV 4.10's `resizeArea_<uchar, float>` and `computeResizeAreaTab`
//! (`imgproc/src/resize.cpp`), which `preprocess.py` reaches twice: `_downscale_for_detection`
//! to 1000 px here, and `_resize` to 1600 px in P3. Each output pixel is the area-weighted mean of
//! the source pixels its footprint covers, accumulated in `f32` row by row exactly as OpenCV's
//! loop does, then rounded half to even (`saturate_cast<uchar>`). On the 13 bay photos the 1000 px
//! copy is identical to OpenCV's, every pixel.
//!
//! # The one path not taken
//!
//! When both ratios are exact integers OpenCV switches to `resizeAreaFast`, which sums integers
//! and, for a 2x2 block, rounds half up in its SIMD lanes and half to even in its scalar tail.
//! This always takes the general path, which on an integer ratio gives each pixel the same
//! footprint and can differ only where a mean lands exactly on a half. No bay photo has an integer
//! ratio (5712 and 1920 px wide, to 1000 and 1600), so the difference is named here rather than
//! ported blind.

/// One source pixel's share of one destination pixel: `DecimateAlpha`.
#[derive(Clone, Copy)]
struct Share {
    dst: usize,
    src: usize,
    alpha: f32,
}

/// `computeResizeAreaTab`: which source pixels each destination pixel covers, and by how much.
/// `scale` is source pixels per destination pixel, computed as OpenCV does (`1 / (dsize/ssize)`,
/// which is not always `ssize/dsize` in the last bit).
fn area_table(ssize: usize, dsize: usize, scale: f64) -> Vec<Share> {
    let mut table = Vec::with_capacity(ssize * 2);
    for dx in 0..dsize {
        let fsx1 = dx as f64 * scale;
        let fsx2 = fsx1 + scale;
        let cell_width = scale.min(ssize as f64 - fsx1);
        let mut sx1 = fsx1.ceil() as i64;
        let mut sx2 = fsx2.floor() as i64;
        sx2 = sx2.min(ssize as i64 - 1);
        sx1 = sx1.min(sx2);
        if sx1 as f64 - fsx1 > 1e-3 {
            table.push(Share {
                dst: dx,
                src: (sx1 - 1) as usize,
                alpha: ((sx1 as f64 - fsx1) / cell_width) as f32,
            });
        }
        for sx in sx1..sx2 {
            table.push(Share {
                dst: dx,
                src: sx as usize,
                alpha: (1.0 / cell_width) as f32,
            });
        }
        if fsx2 - sx2 as f64 > 1e-3 {
            table.push(Share {
                dst: dx,
                src: sx2 as usize,
                alpha: ((fsx2 - sx2 as f64).min(1.0).min(cell_width) / cell_width) as f32,
            });
        }
    }
    table
}

/// `saturate_cast<uchar>(float)`: round half to even (`cvRound`), then clamp.
pub(crate) fn saturate_u8(value: f32) -> u8 {
    value.round_ties_even().clamp(0.0, 255.0) as u8
}

/// Shrink interleaved 8-bit pixels (`channels` per pixel) from `width`x`height` to
/// `dst_width`x`dst_height` by area averaging. Both destination sides must be no larger than the
/// source's, which is the only way frozen Python calls it (`INTER_AREA` on an enlargement is
/// bilinear in OpenCV, and nothing here enlarges).
pub fn resize_area(
    src: &[u8],
    width: usize,
    height: usize,
    channels: usize,
    dst_width: usize,
    dst_height: usize,
) -> Vec<u8> {
    assert_eq!(src.len(), width * height * channels);
    assert!(dst_width >= 1 && dst_height >= 1);
    assert!(
        dst_width <= width && dst_height <= height,
        "resize_area only shrinks"
    );
    let scale_x = 1.0 / (dst_width as f64 / width as f64);
    let scale_y = 1.0 / (dst_height as f64 / height as f64);
    let xtab = area_table(width, dst_width, scale_x);
    let ytab = area_table(height, dst_height, scale_y);

    let row_len = dst_width * channels;
    let mut dst = vec![0u8; dst_height * row_len];
    let mut buf = vec![0f32; row_len];
    let mut sum = vec![0f32; row_len];
    let mut prev_dy = ytab[0].dst;
    for share in &ytab {
        let beta = share.alpha;
        let row = &src[share.src * width * channels..(share.src + 1) * width * channels];
        buf.iter_mut().for_each(|value| *value = 0.0);
        for x in &xtab {
            let (d, s) = (x.dst * channels, x.src * channels);
            for c in 0..channels {
                buf[d + c] += f32::from(row[s + c]) * x.alpha;
            }
        }
        if share.dst != prev_dy {
            let out = &mut dst[prev_dy * row_len..(prev_dy + 1) * row_len];
            for ((o, s), b) in out.iter_mut().zip(sum.iter_mut()).zip(&buf) {
                *o = saturate_u8(*s);
                *s = beta * b;
            }
            prev_dy = share.dst;
        } else {
            for (s, b) in sum.iter_mut().zip(&buf) {
                *s += beta * b;
            }
        }
    }
    let out = &mut dst[prev_dy * row_len..(prev_dy + 1) * row_len];
    for (o, s) in out.iter_mut().zip(&sum) {
        *o = saturate_u8(*s);
    }
    dst
}

/// `INTER_RESIZE_COEF_BITS`: bilinear weights are 11-bit fixed point for 8-bit images.
const COEF_SCALE: f32 = 2048.0;

/// `saturate_cast<short>(float)`: round half to even (`cvRound`), then clamp.
fn saturate_i16(value: f32) -> i16 {
    (value.round_ties_even() as i32).clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16
}

/// `cv2.resize(..., interpolation=cv2.INTER_LINEAR)`, OpenCV's default, on interleaved 8-bit
/// pixels: the text detector's resize to sides that are multiples of 32 (`DetResizeForTest`).
///
/// The twin of `resizeGeneric_` with `HResizeLinear<uchar, int, short, 2048>` and
/// `VResizeLinear` for 8-bit output (`imgproc/src/resize.cpp`). Each axis's weights are `1 - f`
/// and `f` in `f32`, rounded to 11 bits. The horizontal pass is exact integer arithmetic. The
/// vertical pass is `VResizeLinearVec_32s8u`'s arithmetic ([`vresize`]), which is coarser than
/// full precision.
///
/// Two of OpenCV's routes are not taken. `ipp_resize` refuses 8-bit bilinear unless
/// `useIPP_NotExact` is set, which it is not by default, so IPP never runs this. An exact halving
/// on both axes becomes `INTER_AREA` in OpenCV, and here goes to [`resize_area`], whose general
/// path can differ from OpenCV's `resizeAreaFast` where a mean lands on a half. The detector never
/// asks for one: its `min` limit only enlarges, and on the bay's 1600 px working images each side
/// moves by under 16 px to reach a multiple of 32.
pub fn resize_linear(
    src: &[u8],
    width: usize,
    height: usize,
    channels: usize,
    dst_width: usize,
    dst_height: usize,
) -> Vec<u8> {
    assert_eq!(src.len(), width * height * channels);
    assert!(dst_width >= 1 && dst_height >= 1);
    if (dst_width, dst_height) == (width, height) {
        // `cv::resize` copies when the size does not change.
        return src.to_vec();
    }
    let scale_x = 1.0 / (dst_width as f64 / width as f64);
    let scale_y = 1.0 / (dst_height as f64 / height as f64);
    // `is_area_fast && iscale_x == 2 && iscale_y == 2`.
    let halving = |scale: f64| (scale - 2.0).abs() < f64::EPSILON;
    if halving(scale_x) && halving(scale_y) {
        return resize_area(src, width, height, channels, dst_width, dst_height);
    }

    // The horizontal table, per destination value: the source offset and the two weights.
    let row_len = dst_width * channels;
    let mut xofs = vec![0usize; row_len];
    let mut alpha = vec![[0i16; 2]; row_len];
    let mut xmax = dst_width;
    for dx in 0..dst_width {
        let mut fx = ((dx as f64 + 0.5) * scale_x - 0.5) as f32;
        let mut sx = fx.floor() as i64;
        fx -= sx as f32;
        if sx < 0 {
            (fx, sx) = (0.0, 0);
        }
        if sx + 1 >= width as i64 {
            xmax = xmax.min(dx);
            if sx >= width as i64 - 1 {
                (fx, sx) = (0.0, width as i64 - 1);
            }
        }
        let weights = [
            saturate_i16((1.0 - fx) * COEF_SCALE),
            saturate_i16(fx * COEF_SCALE),
        ];
        for c in 0..channels {
            xofs[dx * channels + c] = sx as usize * channels + c;
            alpha[dx * channels + c] = weights;
        }
    }
    let xmax = xmax * channels;
    let hresize = |row: &[u8], out: &mut [i32]| {
        for (dx, value) in out.iter_mut().enumerate() {
            let sx = xofs[dx];
            *value = if dx < xmax {
                i32::from(row[sx]) * i32::from(alpha[dx][0])
                    + i32::from(row[sx + channels]) * i32::from(alpha[dx][1])
            } else {
                i32::from(row[sx]) * 2048
            };
        }
    };

    let src_row = |y: usize| &src[y * width * channels..(y + 1) * width * channels];
    let mut dst = vec![0u8; dst_height * row_len];
    // The two horizontal rows the current output row blends, and which source rows they hold.
    // Reusing a row is OpenCV's saving and changes nothing in the answer.
    let mut rows = [vec![0i32; row_len], vec![0i32; row_len]];
    let mut held = [usize::MAX; 2];
    for dy in 0..dst_height {
        let fy = ((dy as f64 + 0.5) * scale_y - 0.5) as f32;
        let sy = fy.floor() as i64;
        let fy = fy - sy as f32;
        let beta = [
            saturate_i16((1.0 - fy) * COEF_SCALE),
            saturate_i16(fy * COEF_SCALE),
        ];
        for k in 0..2 {
            let wanted = (sy + k as i64).clamp(0, height as i64 - 1) as usize;
            if held[k] != wanted {
                if let Some(other) = (0..2).find(|&o| o != k && held[o] == wanted) {
                    let copy = rows[other].clone();
                    rows[k] = copy;
                } else {
                    hresize(src_row(wanted), &mut rows[k]);
                }
                held[k] = wanted;
            }
        }
        vresize(
            &rows[0],
            &rows[1],
            beta,
            &mut dst[dy * row_len..(dy + 1) * row_len],
        );
    }
    dst
}

/// `VResizeLinear` for 8-bit output, in `VResizeLinearVec_32s8u`'s arithmetic: each horizontal
/// sum shifted right by 4 to fit a 16-bit lane (`v_pack(v_shr<4>(..))`), the high half of its
/// product with the weight kept (`v_mul_hi`), the two added with saturation, then rounded off by
/// 2 bits and packed to 8 bits (`v_rshr_pack_u<2>`).
///
/// **On every value, the row's tail included.** The C++ reads as though the values after the last
/// whole vector block take `FixedPtCast<int, uchar, 22>`, the full sum rounded by 22 bits, and
/// that was ported first. Against `cv2.resize` on 400 random sizes and channel counts it was a
/// level high on the tail of 346 of them, and this formula alone matched all 400 (M29 P4). A det
/// input row on the bay's photos is 4,800 values, a whole number of blocks, so the photos alone
/// could not have told the two apart.
fn vresize(s0: &[i32], s1: &[i32], beta: [i16; 2], out: &mut [u8]) {
    let (b0, b1) = (i32::from(beta[0]), i32::from(beta[1]));
    let pack = |v: i32| (v >> 4).clamp(i32::from(i16::MIN), i32::from(i16::MAX));
    let high = |v: i32, w: i32| ((v * w) >> 16) as i16;
    for ((o, &a), &b) in out.iter_mut().zip(s0).zip(s1) {
        let sum = high(pack(a), b0).saturating_add(high(pack(b), b1));
        *o = (sum.saturating_add(2) >> 2).clamp(0, 255) as u8;
    }
}

/// The size `_downscale_for_detection` and `_resize` ask for: `target` wide, and
/// `max(int(height * (target / width)), 1)` tall, the `int` truncating the `f64` product (so a
/// 1108x554 image goes to 1000x499, not 500). `None` when the image is already no wider than
/// `target`, which both leave untouched. The scale is returned as Python holds it, for the caller
/// that maps coordinates back.
pub fn shrink_to_width(width: usize, height: usize, target: usize) -> Option<(f64, usize, usize)> {
    if width <= target {
        return None;
    }
    let scale = target as f64 / width as f64;
    let new_height = ((height as f64 * scale) as usize).max(1);
    Some((scale, target, new_height))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_detection_size_truncates_as_python_int_does() {
        // 554 * (1000 / 1108) is 499.99999999999994 in f64, so Python's int() makes it 499.
        assert_eq!(
            shrink_to_width(1108, 554, 1000),
            Some((1000.0 / 1108.0, 1000, 499))
        );
        // The bay photos' two sizes, both landing on 750 exactly (frozen Python, M29 P2).
        assert_eq!(shrink_to_width(5712, 4284, 1000).unwrap().2, 750);
        assert_eq!(shrink_to_width(1920, 1440, 1000).unwrap().2, 750);
        assert_eq!(shrink_to_width(1000, 10, 1000), None);
    }

    #[test]
    fn an_integer_ratio_averages_each_block() {
        // 4x2 grey to 2x1: each output is the mean of a 2x2 block.
        let src = [0, 10, 100, 200, 20, 30, 50, 250];
        assert_eq!(resize_area(&src, 4, 2, 1, 2, 1), vec![15, 150]);
    }

    #[test]
    fn a_fractional_ratio_weights_the_split_pixel() {
        // 3 px to 2: footprints are 1.5 px, so the middle pixel is split evenly.
        // (0 + 0.5 * 90) / 1.5 = 30 and (0.5 * 90 + 255) / 1.5 = 200.
        let src = [0, 90, 255];
        assert_eq!(resize_area(&src, 3, 1, 1, 2, 1), vec![30, 200]);
    }

    #[test]
    fn channels_are_averaged_independently() {
        let src = [10, 20, 30, 30, 40, 50];
        assert_eq!(resize_area(&src, 2, 1, 3, 1, 1), vec![20, 30, 40]);
    }

    #[test]
    fn rounding_is_half_to_even() {
        assert_eq!(saturate_u8(0.5), 0);
        assert_eq!(saturate_u8(1.5), 2);
        assert_eq!(saturate_u8(2.5), 2);
        assert_eq!(saturate_u8(-3.0), 0);
        assert_eq!(saturate_u8(300.0), 255);
    }
}
