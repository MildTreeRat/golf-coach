//! `cv2.getPerspectiveTransform` and `cv2.warpPerspective` on 8-bit three-channel pixels: bilinear
//! with a constant black border, as `preprocess.py` warps the screen [M29 P3], and bicubic with a
//! replicated border, as PaddleX crops each text box for recognition [M29 P5].
//!
//! The twins of OpenCV 4.10's `getPerspectiveTransform` and `warpPerspective`
//! (`imgproc/src/imgwarp.cpp`), with the two pieces of `core/src/lapack.cpp` they reach: `solve`
//! by `DECOMP_LU` (`hal::LU64f`'s `LUImpl`, `core/src/matrix_decomp.cpp`) for the transform, and
//! `invert`'s closed-form 3x3 inverse, which `warpPerspective` applies to the matrix it is given
//! because it samples the source from each destination pixel. `preprocess.py`'s `warp_to_rect`
//! makes one call to each, and so does PaddleX's `CropByPolys.get_rotate_crop_image` per box.
//! Neither warp has an Intel IPP path in the pip wheel (`IPP_DISABLE_WARPPERSPECTIVE` and
//! `IPP_DISABLE_REMAP` are set in OpenCV's `private.hpp`, and the IPP warp would take only a
//! constant or transparent border anyway), and `cv2.ipp.setUseIPP(False)` changes no pixel of
//! either.
//!
//! # Why every detail is OpenCV's
//!
//! Not one step is real-valued interpolation, and each of these was ported because it decides a
//! value:
//!
//! - **The transform's system mixes `f32` and `f64`.** `a[i][6] = -src[i].x*dst[i].x` multiplies
//!   two `float`s, so the product is rounded to `f32` before it enters the `f64` system.
//! - **Source coordinates are fixed point.** For each destination pixel the source position is
//!   `(X0 + M[0]*x1) * (32 / W)` in `f64`, rounded half to even to an integer in 1/32 px, whose
//!   top bits are the pixel and low five bits each axis's fraction (`INTER_BITS`). `X0` is
//!   computed once per row of a **block** (`BLOCK_SZ` 32: blocks 64 px wide when the output is),
//!   from the block's first column, so the sum is grouped by block and that is followed here.
//!   Both interpolations share this step ([`sample_positions`]).
//! - **The weights are 15-bit integers** (`BilinearTab_i`, `BicubicTab_i`,
//!   `INTER_REMAP_COEF_BITS`), and the sum is rounded by adding half and shifting
//!   (`FixedPtCast`). Each table is `initInterTab2D`'s: the `f32` weights scaled to 1 << 15 and
//!   rounded half to even, then **repaired** so they sum to exactly 32768. The repair looks only
//!   at the 2x2 block starting at index `ksize / 2` in each axis. For bilinear that is the fourth
//!   weight and three entries past the block; for bicubic it is the bottom-right quarter of the
//!   4x4, not its centre. At a zero fraction the centre weight would be 32768, which
//!   `saturate_cast<short>` clips to 32767, and the repair gives the missing unit to the
//!   neighbour diagonally below and right. That changes a value only when the rest of a sum sits
//!   within that one unit of a rounding edge, but the table is OpenCV's, so it is kept.
//! - **Bilinear's border is per neighbour.** A sample whose 2x2 neighbourhood straddles the
//!   photo's edge takes black (`BORDER_CONSTANT`, value 0) for the neighbours outside and the
//!   photo for the rest. The screen quads' rotated boxes reach past the photo (the corners in
//!   `tests/fixtures/screen_quads.json` go negative), so this decides the warped image's rim.
//! - **Bicubic's border is the nearest edge pixel** (`BORDER_REPLICATE`, `borderInterpolate`
//!   clamping each of the 4x4 taps), and its sum can overshoot either end of 0..255, which the
//!   final `saturate_cast<uchar>` clips (`remapBicubic`).
//!
//! OpenCV's SSE4.1 and SIMD128 paths do the same arithmetic as its scalar loop, lane for lane, so
//! the scalar loop is what is followed. `remapBicubic` has no vector path at all.

/// A 3x3 matrix, row-major, as OpenCV's `CV_64F` `Mat` holds it.
pub type Matrix = [f64; 9];

/// Bits of sub-pixel position (`INTER_BITS`), and the steps per pixel (`INTER_TAB_SIZE`).
const INTER_BITS: i32 = 5;
const INTER_TAB_SIZE: i32 = 1 << INTER_BITS;

/// The fixed-point scale of the interpolation weights (`INTER_REMAP_COEF_BITS`).
const COEF_BITS: i32 = 15;

/// `WarpPerspectiveInvoker`'s block: `BLOCK_SZ * BLOCK_SZ` destination pixels at a time.
const BLOCK_SZ: usize = 32;

/// `getPerspectiveTransform(src, dst)`: the matrix taking each `src` corner to its `dst` corner,
/// solved as OpenCV builds and solves its 8x8 system. A singular system gives the matrix OpenCV
/// gives, every coefficient zero but the last (`solve` zeroes its answer when `LU` fails).
pub fn get_perspective_transform(src: &[[f32; 2]; 4], dst: &[[f32; 2]; 4]) -> Matrix {
    let mut a = [[0f64; 8]; 8];
    let mut b = [0f64; 8];
    for i in 0..4 {
        let ([sx, sy], [dx, dy]) = (src[i], dst[i]);
        a[i][0] = f64::from(sx);
        a[i + 4][3] = f64::from(sx);
        a[i][1] = f64::from(sy);
        a[i + 4][4] = f64::from(sy);
        a[i][2] = 1.0;
        a[i + 4][5] = 1.0;
        // `float * float` in the C++, rounded to f32 before it is stored as a double.
        a[i][6] = f64::from(-sx * dx);
        a[i][7] = f64::from(-sy * dx);
        a[i + 4][6] = f64::from(-sx * dy);
        a[i + 4][7] = f64::from(-sy * dy);
        b[i] = f64::from(dx);
        b[i + 4] = f64::from(dy);
    }
    if !lu_solve(&mut a, &mut b) {
        b = [0.0; 8];
    }
    [b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7], 1.0]
}

/// `LUImpl` with `b`, at `DBL_EPSILON * 100`: Gaussian elimination with partial pivoting, then
/// back substitution, in OpenCV's order of operations. `false` when a pivot is below the
/// threshold, where OpenCV's returns 0. Only the columns right of the pivot are updated or
/// swapped, as in the C++; the stale entries left of it are never read again.
fn lu_solve(a: &mut [[f64; 8]; 8], b: &mut [f64; 8]) -> bool {
    let m = a.len();
    let eps = f64::EPSILON * 100.0;
    for i in 0..m {
        let mut k = i;
        for j in i + 1..m {
            if a[j][i].abs() > a[k][i].abs() {
                k = j;
            }
        }
        if a[k][i].abs() < eps {
            return false;
        }
        if k != i {
            for j in i..m {
                let held = a[i][j];
                a[i][j] = a[k][j];
                a[k][j] = held;
            }
            b.swap(i, k);
        }
        let d = -1.0 / a[i][i];
        for j in i + 1..m {
            let alpha = a[j][i] * d;
            for k in i + 1..m {
                a[j][k] += alpha * a[i][k];
            }
            b[j] += alpha * b[i];
        }
    }
    for i in (0..m).rev() {
        // Subtracted left to right, as the C++'s `for( k = i+1; k < m; k++ )` does.
        let mut s = b[i];
        for (coefficient, solved) in a[i][i + 1..].iter().zip(&b[i + 1..]) {
            s -= coefficient * solved;
        }
        b[i] = s / a[i][i];
    }
    true
}

/// `invert(m, DECOMP_LU)` on a 3x3 `CV_64F` matrix: the closed form `invert` uses for `n <= 3`,
/// the determinant expanded along the first row and each cofactor times its reciprocal. A zero
/// determinant gives the zero matrix, as OpenCV's does.
pub fn invert3(m: &Matrix) -> Matrix {
    let s = |row: usize, col: usize| m[row * 3 + col];
    let det = s(0, 0) * (s(1, 1) * s(2, 2) - s(1, 2) * s(2, 1))
        - s(0, 1) * (s(1, 0) * s(2, 2) - s(1, 2) * s(2, 0))
        + s(0, 2) * (s(1, 0) * s(2, 1) - s(1, 1) * s(2, 0));
    if det == 0.0 {
        return [0.0; 9];
    }
    let d = 1.0 / det;
    [
        (s(1, 1) * s(2, 2) - s(1, 2) * s(2, 1)) * d,
        (s(0, 2) * s(2, 1) - s(0, 1) * s(2, 2)) * d,
        (s(0, 1) * s(1, 2) - s(0, 2) * s(1, 1)) * d,
        (s(1, 2) * s(2, 0) - s(1, 0) * s(2, 2)) * d,
        (s(0, 0) * s(2, 2) - s(0, 2) * s(2, 0)) * d,
        (s(0, 2) * s(1, 0) - s(0, 0) * s(1, 2)) * d,
        (s(1, 0) * s(2, 1) - s(1, 1) * s(2, 0)) * d,
        (s(0, 1) * s(2, 0) - s(0, 0) * s(2, 1)) * d,
        (s(0, 0) * s(1, 1) - s(0, 1) * s(1, 0)) * d,
    ]
}

/// `BilinearTab_i[alpha]`: the four weights for a sample whose fractions are `alpha & 31` across
/// and `alpha >> 5` down, summing to 32768. The zero fraction's quirk is the module doc's.
fn weights(alpha: i32) -> [i32; 4] {
    let (fy, fx) = (alpha >> INTER_BITS, alpha & (INTER_TAB_SIZE - 1));
    let (gy, gx) = (INTER_TAB_SIZE - fy, INTER_TAB_SIZE - fx);
    if alpha == 0 {
        return [(1 << COEF_BITS) - 1, 0, 0, 1];
    }
    // (1 - y)(1 - x), (1 - y)x, y(1 - x), yx, each in 1/1024 steps scaled to 1 << 15.
    let scale = (1 << COEF_BITS) / (INTER_TAB_SIZE * INTER_TAB_SIZE);
    [
        gy * gx * scale,
        gy * fx * scale,
        fy * gx * scale,
        fy * fx * scale,
    ]
}

/// `cvRound` of a `f64` already clamped to `int`'s range, as `_mm_cvtpd_epi32` rounds: half to
/// even.
fn round_to_i32(value: f64) -> i32 {
    value.round_ties_even() as i32
}

/// `saturate_cast<short>`.
fn saturate_i16(value: i32) -> i32 {
    value.clamp(i32::from(i16::MIN), i32::from(i16::MAX))
}

/// `WarpPerspectiveInvoker`'s position step, shared by both interpolations: for each destination
/// pixel `(x, y)`, the source pixel `(sx, sy)` its fixed-point position falls in (each already
/// through `saturate_cast<short>`, as the `XY` map holds them) and `alpha`, the index of its
/// fractions in the weight tables (`(fy << 5) | fx`). `inverse` maps destination to source.
fn sample_positions(
    inverse: &Matrix,
    dst_width: usize,
    dst_height: usize,
    mut visit: impl FnMut(usize, usize, i32, i32, usize),
) {
    let m = inverse;
    if dst_width == 0 || dst_height == 0 {
        return;
    }
    // The block width `WarpPerspectiveInvoker` settles on. Only the column blocks reach the
    // arithmetic (through `X0`); its block height, and how `parallel_for_` stripes the rows, do
    // not, because each row's sum starts from the absolute row index.
    let bh0 = (BLOCK_SZ / 2).min(dst_height);
    let bw0 = (BLOCK_SZ * BLOCK_SZ / bh0).min(dst_width);
    for y in 0..dst_height {
        for x in (0..dst_width).step_by(bw0) {
            let bw = bw0.min(dst_width - x);
            let (xf, yf) = (x as f64, y as f64);
            let x0 = m[0] * xf + m[1] * yf + m[2];
            let y0 = m[3] * xf + m[4] * yf + m[5];
            let w0 = m[6] * xf + m[7] * yf + m[8];
            for x1 in 0..bw {
                let x1f = x1 as f64;
                let mut w = w0 + m[6] * x1f;
                w = if w != 0.0 {
                    f64::from(INTER_TAB_SIZE) / w
                } else {
                    0.0
                };
                let fx = ((x0 + m[0] * x1f) * w).clamp(f64::from(i32::MIN), f64::from(i32::MAX));
                let fy = ((y0 + m[3] * x1f) * w).clamp(f64::from(i32::MIN), f64::from(i32::MAX));
                let (big_x, big_y) = (round_to_i32(fx), round_to_i32(fy));
                let sx = saturate_i16(big_x >> INTER_BITS);
                let sy = saturate_i16(big_y >> INTER_BITS);
                let alpha = (big_y & (INTER_TAB_SIZE - 1)) * INTER_TAB_SIZE
                    + (big_x & (INTER_TAB_SIZE - 1));
                visit(x + x1, y, sx, sy, alpha as usize);
            }
        }
    }
}

/// `FixedPtCast<int, uchar, INTER_REMAP_COEF_BITS>`: a 15-bit fixed-point sum rounded by adding
/// half and shifting (arithmetically, so a negative sum floors), then `saturate_cast<uchar>`.
fn fixed_to_u8(sum: i32) -> u8 {
    ((sum + (1 << (COEF_BITS - 1))) >> COEF_BITS).clamp(0, 255) as u8
}

/// `warpPerspective(src, M, (dst_width, dst_height))` with `INTER_LINEAR` and a black constant
/// border, given `inverse` already inverted (what OpenCV computes from `M` before sampling; the
/// caller does it with [`invert3`]). `src` is interleaved 8-bit RGB, `width` by `height`.
pub fn warp_perspective(
    src: &[u8],
    width: usize,
    height: usize,
    inverse: &Matrix,
    dst_width: usize,
    dst_height: usize,
) -> Vec<u8> {
    assert_eq!(src.len(), width * height * 3);
    let mut dst = vec![0u8; dst_width * dst_height * 3];
    let (sw, sh) = (width as i32, height as i32);
    let pixel = |sx: i32, sy: i32| -> Option<usize> {
        (sx >= 0 && sx < sw && sy >= 0 && sy < sh).then(|| (sy as usize * width + sx as usize) * 3)
    };
    sample_positions(inverse, dst_width, dst_height, |x, y, sx, sy, alpha| {
        let at = (y * dst_width + x) * 3;
        let out = &mut dst[at..at + 3];
        if sx >= sw || sx + 1 < 0 || sy >= sh || sy + 1 < 0 {
            out.fill(0);
            return;
        }
        let w = weights(alpha as i32);
        let taps = [
            pixel(sx, sy),
            pixel(sx + 1, sy),
            pixel(sx, sy + 1),
            pixel(sx + 1, sy + 1),
        ];
        for (c, value) in out.iter_mut().enumerate() {
            let mut sum = 0i32;
            for (tap, weight) in taps.iter().zip(w) {
                if let Some(at) = tap {
                    sum += i32::from(src[at + c]) * weight;
                }
            }
            *value = fixed_to_u8(sum);
        }
    });
    dst
}

/// `interpolateCubic(x)`: the four taps' weights at fraction `x`, Keys' kernel with
/// `A = -0.75`, in `f32` and in the C++'s order of operations; the last is one minus the others.
fn cubic_coefficients(x: f32) -> [f32; 4] {
    const A: f32 = -0.75;
    let c0 = ((A * (x + 1.0) - 5.0 * A) * (x + 1.0) + 8.0 * A) * (x + 1.0) - 4.0 * A;
    let c1 = ((A + 2.0) * x - (A + 3.0)) * x * x + 1.0;
    let c2 = ((A + 2.0) * (1.0 - x) - (A + 3.0)) * (1.0 - x) * (1.0 - x) + 1.0;
    let c3 = 1.0 - c0 - c1 - c2;
    [c0, c1, c2, c3]
}

/// `BicubicTab_i`, built as `initInterTab2D(INTER_CUBIC, true)` builds it: entry `alpha` holds
/// the 4x4 weights, row by row, for fractions `alpha >> 5` down and `alpha & 31` across, summing
/// to 32768 after the repair the module doc describes.
fn bicubic_table() -> &'static [[i32; 16]] {
    static TABLE: std::sync::OnceLock<Vec<[i32; 16]>> = std::sync::OnceLock::new();
    TABLE.get_or_init(|| {
        let scale = 1.0f32 / INTER_TAB_SIZE as f32;
        let one_d: Vec<[f32; 4]> = (0..INTER_TAB_SIZE)
            .map(|i| cubic_coefficients(i as f32 * scale))
            .collect();
        let mut table = Vec::with_capacity((INTER_TAB_SIZE * INTER_TAB_SIZE) as usize);
        for vy in &one_d {
            for vx in &one_d {
                let mut itab = [0i32; 16];
                for k1 in 0..4 {
                    for k2 in 0..4 {
                        let v = vy[k1] * vx[k2];
                        // `saturate_cast<short>(v * INTER_REMAP_COEF_SCALE)`: `cvRound`, then
                        // clamped.
                        let scaled = (v * (1 << COEF_BITS) as f32).round_ties_even() as i32;
                        itab[k1 * 4 + k2] = saturate_i16(scaled);
                    }
                }
                let isum: i32 = itab.iter().sum();
                if isum != 1 << COEF_BITS {
                    let diff = isum - (1 << COEF_BITS);
                    let (mut big, mut small) = ((2, 2), (2, 2));
                    for k1 in 2..4 {
                        for k2 in 2..4 {
                            let v = itab[k1 * 4 + k2];
                            if v < itab[small.0 * 4 + small.1] {
                                small = (k1, k2);
                            } else if v > itab[big.0 * 4 + big.1] {
                                big = (k1, k2);
                            }
                        }
                    }
                    // `(short)(itab[..] - diff)`: the C++ narrows, which a weight this close to
                    // its sum never needs.
                    let (k1, k2) = if diff < 0 { big } else { small };
                    itab[k1 * 4 + k2] = i32::from((itab[k1 * 4 + k2] - diff) as i16);
                }
                table.push(itab);
            }
        }
        table
    })
}

/// `warpPerspective(src, M, (dst_width, dst_height), flags=INTER_CUBIC,
/// borderMode=BORDER_REPLICATE)`, given `inverse` already inverted as for [`warp_perspective`]:
/// each destination pixel the 4x4 neighbourhood around its source position (one pixel up and
/// left of it to two down and right), each tap clamped to the photo, weighted by
/// [`bicubic_table`] (`remapBicubic`). `src` is interleaved 8-bit RGB, `width` by `height`, and
/// must not be empty.
pub fn warp_perspective_cubic(
    src: &[u8],
    width: usize,
    height: usize,
    inverse: &Matrix,
    dst_width: usize,
    dst_height: usize,
) -> Vec<u8> {
    assert_eq!(src.len(), width * height * 3);
    assert!(width > 0 && height > 0, "OpenCV asserts a non-empty source");
    let table = bicubic_table();
    let mut dst = vec![0u8; dst_width * dst_height * 3];
    let clamp = |v: i32, size: usize| v.clamp(0, size as i32 - 1) as usize;
    sample_positions(inverse, dst_width, dst_height, |x, y, sx, sy, alpha| {
        let w = &table[alpha];
        let (sx, sy) = (sx - 1, sy - 1);
        let xs: [usize; 4] = std::array::from_fn(|i| clamp(sx + i as i32, width) * 3);
        let ys: [usize; 4] = std::array::from_fn(|i| clamp(sy + i as i32, height) * width * 3);
        let at = (y * dst_width + x) * 3;
        for c in 0..3 {
            // Integer arithmetic, so the order of the sum is free; OpenCV's interior loop and
            // its border loop (whose constant term is zero for a replicated border) agree.
            let mut sum = 0i32;
            for (row, ry) in ys.iter().enumerate() {
                for (col, cx) in xs.iter().enumerate() {
                    sum += i32::from(src[ry + cx + c]) * w[row * 4 + col];
                }
            }
            dst[at + c] = fixed_to_u8(sum);
        }
    });
    dst
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_weights_sum_to_one_in_fixed_point() {
        for alpha in 0..INTER_TAB_SIZE * INTER_TAB_SIZE {
            assert_eq!(
                weights(alpha).iter().sum::<i32>(),
                1 << COEF_BITS,
                "{alpha}"
            );
        }
        // Half a pixel across, none down: the two left-right neighbours share it evenly.
        assert_eq!(weights(16), [16384, 16384, 0, 0]);
    }

    #[test]
    fn the_bicubic_weights_sum_to_one_in_fixed_point() {
        let table = bicubic_table();
        assert_eq!(table.len(), 1024);
        for (alpha, w) in table.iter().enumerate() {
            assert_eq!(w.iter().sum::<i32>(), 1 << COEF_BITS, "{alpha}");
        }
        // A zero fraction: the centre tap clipped to 32767, and the unit the repair gives back
        // landing diagonally below and right of it, as `initInterTab2D` leaves it.
        let mut zero = [0; 16];
        zero[5] = 32767;
        zero[10] = 1;
        assert_eq!(table[0], zero);
    }

    #[test]
    fn a_cubic_sample_on_the_grid_is_the_pixel_and_the_border_repeats() {
        // A 4x1 ramp shifted two pixels left: on the grid the kernel is the pixel itself, and
        // past the right edge the last pixel repeats.
        let src: Vec<u8> = [10u8, 60, 110, 160].iter().flat_map(|&v| [v; 3]).collect();
        let shift = [1.0, 0.0, 2.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
        let out = warp_perspective_cubic(&src, 4, 1, &shift, 4, 1);
        assert_eq!(
            out,
            [110, 110, 110, 160, 160, 160, 160, 160, 160, 160, 160, 160]
        );
    }

    #[test]
    fn the_transform_of_a_square_onto_itself_is_the_identity() {
        let square = [[0.0, 0.0], [9.0, 0.0], [9.0, 9.0], [0.0, 9.0]];
        let m = get_perspective_transform(&square, &square);
        let identity = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
        for (got, want) in m.iter().zip(identity) {
            assert!((got - want).abs() < 1e-12, "{m:?}");
        }
    }

    #[test]
    fn a_degenerate_quad_gives_opencvs_zero_matrix() {
        let point = [[1.0, 1.0]; 4];
        let dst = [[0.0, 0.0], [9.0, 0.0], [9.0, 9.0], [0.0, 9.0]];
        assert_eq!(
            get_perspective_transform(&point, &dst),
            [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0]
        );
        assert_eq!(invert3(&[0.0; 9]), [0.0; 9]);
    }

    #[test]
    fn the_inverse_undoes_the_matrix() {
        let m = [2.0, 0.5, 3.0, -0.25, 1.5, 7.0, 0.001, 0.002, 1.0];
        let inv = invert3(&m);
        for row in 0..3 {
            for col in 0..3 {
                let product: f64 = (0..3).map(|k| m[row * 3 + k] * inv[k * 3 + col]).sum();
                let want = if row == col { 1.0 } else { 0.0 };
                assert!((product - want).abs() < 1e-12);
            }
        }
    }

    #[test]
    fn a_half_pixel_shift_averages_neighbours_and_blackens_the_rim() {
        // One row, values 0, 100, 200 in every channel; sampled half a pixel to the right.
        let src: Vec<u8> = [0u8, 100, 200].iter().flat_map(|&v| [v; 3]).collect();
        let shift = [1.0, 0.0, 0.5, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
        let out = warp_perspective(&src, 3, 1, &shift, 3, 1);
        // 50, 150, then the last pixel's right neighbour is outside: (200 + 0) / 2 = 100. The row
        // below is outside too, but its weight is zero at a zero vertical fraction.
        assert_eq!(out, vec![50, 50, 50, 150, 150, 150, 100, 100, 100]);
    }
}
