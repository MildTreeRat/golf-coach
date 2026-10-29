//! `cv2.cvtColor(..., COLOR_BGR2LAB)` and `COLOR_LAB2BGR` on 8-bit pixels. [M29 P3]
//!
//! The twins of OpenCV 4.10's `RGB2Lab_b` and `Lab2RGBinteger` (`imgproc/src/color_lab.cpp`),
//! which `preprocess.py`'s `enhance` reaches on either side of its CLAHE. Neither is floating
//! point: 8-bit L\*a\*b\* in OpenCV is fixed-point arithmetic over lookup tables (the sRGB gamma
//! curve in 1/8 levels, a cube root in 1/32768ths, an inverse gamma over 4096 steps, and two
//! tables for the return trip), with `enableBitExactness` on so that the trip back never takes the
//! float path. The arithmetic is integer, so the port is exact once the tables are, and the tables
//! are built here as OpenCV builds them: `softfloat` and `softdouble` are IEEE `f32` and `f64`
//! with correct rounding, which Rust's are too, and the fused `mulAdd` is [`f32::mul_add`]. Its
//! `cbrt` is not correctly rounded, so it is ported (`softfloat_cbrt`): the library's cube root
//! put one table entry on the wrong side of a half (the function says which). Its `pow` is OpenCV's
//! own too (`f64_exp(y * f64_log(x))`), and [`f64::powf`] stands in for it, because no gamma entry
//! sits close enough to a half for the difference to show: the exhaustive check below is what
//! says so, and it is the check, not the argument, that holds.
//!
//! RGB order throughout. OpenCV's coefficients are placed by its `blueIdx`, which only says which
//! end of the pixel is blue; the sums are the same.
//!
//! Neither direction has an IPP path in the pip wheel (`IPP_DISABLE_RGB_LAB`,
//! `IPP_DISABLE_LAB_RGB`).
//!
//! # How it is held to OpenCV
//!
//! Every input, both ways: `tests/rectify.rs` converts all 2^24 colours and all 2^24 L\*a\*b\*
//! triples and compares the digests with `cv2`'s, recorded in
//! `tests/fixtures/rectify_reference.json`. Exhaustive is cheaper than reasoning about which
//! table entries a photo can reach.

use std::sync::OnceLock;

/// `lab_shift` (`xyz_shift`), `gamma_shift` and their sum: the fixed-point positions of the XYZ
/// sums, of the gamma table's eighths of a level, and of the cube-root table.
const LAB_SHIFT: i32 = 12;
const GAMMA_SHIFT: i32 = 3;
const LAB_SHIFT2: i32 = LAB_SHIFT + GAMMA_SHIFT;

/// `LAB_CBRT_TAB_SIZE_B`: the cube-root table covers 1.5 times the white point's range.
const CBRT_TAB_SIZE: usize = 256 * 3 / 2 * (1 << GAMMA_SHIFT);

/// The inverse gamma table's steps (`inv_gamma_shift`, `INV_GAMMA_TAB_SIZE`).
const INV_GAMMA_SHIFT: i32 = 12;
const INV_GAMMA_TAB_SIZE: usize = 1 << INV_GAMMA_SHIFT;

/// The return trip's fixed point (`LUT_BASE`, `Lab2RGBinteger::base_shift`), and the lowest
/// index `abToXZ_b` is asked for (`minABvalue`), reached at L\*a\*b\* `(0, _, 255)`.
const BASE_SHIFT: i32 = 14;
const LUT_BASE: i32 = 1 << BASE_SHIFT;
const MIN_AB: i32 = -8145;

/// `sRGB2XYZ_D65`, `XYZ2sRGB_D65` and `D65`, as OpenCV spells them: by their bits.
const SRGB_TO_XYZ: [u64; 9] = [
    0x3fda65a14488c60d,
    0x3fd6e297396d0918,
    0x3fc71819d2391d58,
    0x3fcb38cda6e75ff6,
    0x3fe6e297396d0918,
    0x3fb279aae6c8f755,
    0x3f93cc4ac6cdaf4b,
    0x3fbe836eb4e98138,
    0x3fee68427418d691,
];
const XYZ_TO_SRGB: [u64; 9] = [
    0x4009ec804102ff8f,
    0xbff8982a9930be0e,
    0xbfdfe7ff583a53b9,
    0xbfef042528ae74f3,
    0x3ffe040f23897204,
    0x3fa546d3f9e7b80b,
    0x3fac7de5082cf52c,
    0xbfca1e14bdfd2631,
    0x3ff0eabef06b3786,
];
const D65: [u64; 3] = [0x3fee6a22b3892ee8, 0x3ff0000000000000, 0x3ff16b8950763a19];

/// The sRGB curve's constants (`gammaThreshold` and the rest), each a `softdouble` quotient.
const GAMMA_THRESHOLD: f64 = 809.0 / 20000.0;
const GAMMA_INV_THRESHOLD: f64 = 7827.0 / 2500000.0;
const GAMMA_LOW_SCALE: f64 = 323.0 / 25.0;
const GAMMA_POWER: f64 = 12.0 / 5.0;
const GAMMA_X_SHIFT: f64 = 11.0 / 200.0;

struct Tables {
    /// `sRGBGammaTab_b`: an 8-bit level's linear light, in eighths of a level.
    gamma: [i32; 256],
    /// `LabCbrtTab_b`: L\*a\*b\*'s `f(t)` over the XYZ sums, scaled by `1 << 15`.
    cbrt: Vec<i32>,
    /// `sRGBInvGammaTab_b`: linear light in 4096 steps back to an 8-bit level.
    inv_gamma: Vec<i32>,
    /// `LabToYF_b`: an 8-bit L\*'s `Y` and `f(Y)`, in `LUT_BASE` units.
    l_to_yf: Vec<(i32, i32)>,
    /// `abToXZ_b`: `f⁻¹` over `f(Y) ± a/500, b/200`, from `MIN_AB`.
    ab_to_xz: Vec<i32>,
    /// `RGB2Lab_b`'s coefficients: rows X, Y, Z over the white point; columns R, G, B.
    to_xyz: [[i32; 3]; 3],
    /// `Lab2RGBinteger`'s: rows R, G, B; columns X, Y, Z times the white point.
    to_rgb: [[i32; 3]; 3],
}

/// `cvRound`, for `softfloat`, `softdouble` and the hardware: half to even.
fn round_even(value: f64) -> i32 {
    value.round_ties_even() as i32
}

/// `applyGamma`: sRGB to linear light, in `softdouble`, returned as `softfloat`.
fn apply_gamma(x: f32) -> f32 {
    let xd = f64::from(x);
    let linear = if xd <= GAMMA_THRESHOLD {
        xd / GAMMA_LOW_SCALE
    } else {
        ((xd + GAMMA_X_SHIFT) / (1.0 + GAMMA_X_SHIFT)).powf(GAMMA_POWER)
    };
    linear as f32
}

/// `applyInvGamma`: linear light back to sRGB.
fn apply_inv_gamma(x: f32) -> f32 {
    let xd = f64::from(x);
    let encoded = if xd <= GAMMA_INV_THRESHOLD {
        xd * GAMMA_LOW_SCALE
    } else {
        xd.powf(1.0 / GAMMA_POWER) * (1.0 + GAMMA_X_SHIFT) - GAMMA_X_SHIFT
    };
    encoded as f32
}

/// The rational approximation's coefficients in `f32_cbrt`, by their bits: `A1..A5` over `A6..A9`
/// and one.
const CBRT_NUMERATOR: [u64; 5] = [
    0x4046a09e6653ba70,
    0x406808f46c6116e0,
    0x405dca97439cae14,
    0x402add70d2827500,
    0x3fc4f15f83f55d2d,
];
const CBRT_DENOMINATOR: [u64; 4] = [
    0x402d9e20660edb21,
    0x4062ff15c0285815,
    0x406510d06a8112ce,
    0x4040fecbc9e2c375,
];

/// `softfloat`'s `cbrt`, OpenCV's `f32_cbrt` (`core/src/softfloat.cpp`, Ken Turkowski's fast cube
/// root): the mantissa scaled into `[1/8, 1)`, a quartic rational polynomial in `f64`, and the
/// result's mantissa **truncated** to 23 bits under the exponent divided by three. It is not
/// correctly rounded, and a correctly rounded cube root (`f64::cbrt` narrowed to `f32`) lands one
/// `f32` step higher on table entry 324 (`324 / 2040`), whose `32768 * f(x)` then sits on the half
/// between 17745 and 17746. That one entry moved the L\* of 17,445 of the 2^24 colours, and the
/// `a` or `b` of a few hundred more, until this replaced the library call (M29 P3).
fn softfloat_cbrt(x: f32) -> f32 {
    if x.is_nan() || x.is_infinite() {
        return x;
    }
    let bits = x.to_bits();
    if bits & 0x7fff_ffff == 0 {
        // Either zero is returned as +0, as `f32_cbrt` does.
        return 0.0;
    }
    let sign = bits >> 31;
    let exponent = ((bits >> 23) & 0xff) as i32 - 127;
    // C's `%` truncates toward zero, as Rust's does; the shift lands in -3..=-1.
    let mut shift = exponent % 3;
    if shift >= 0 {
        shift -= 3;
    }
    let root_exponent = (exponent - shift) / 3 - 1;
    let fr = f64::from_bits(((1023 + shift) as u64) << 52 | u64::from(bits & 0x007f_ffff) << 29);
    let [a1, a2, a3, a4, a5] = CBRT_NUMERATOR.map(f64::from_bits);
    let [a6, a7, a8, a9] = CBRT_DENOMINATOR.map(f64::from_bits);
    let root = ((((a1 * fr + a2) * fr + a3) * fr + a4) * fr + a5)
        / ((((a6 * fr + a7) * fr + a8) * fr + a9) * fr + 1.0);
    // `packToF32UI` adds the fields rather than or-ing them, and takes the fraction alone: the
    // root is assumed to lie in [0.5, 1), and its own exponent is dropped.
    let fraction = ((root.to_bits() & 0x000f_ffff_ffff_ffff) >> 29) as u32;
    f32::from_bits(
        (sign << 31)
            .wrapping_add(((root_exponent + 127) as u32) << 23)
            .wrapping_add(fraction),
    )
}

fn build() -> Tables {
    let f255 = 255f32;
    let mut gamma = [0i32; 256];
    for (i, entry) in gamma.iter_mut().enumerate() {
        let x = i as f32 / f255;
        *entry = round_even(f64::from(
            (255 * (1 << GAMMA_SHIFT)) as f32 * apply_gamma(x),
        ));
    }

    // L*a*b*'s f(t): linear below (6/29)^3, a cube root above, all in softfloat.
    let lthresh = 216f32 / 24389f32;
    let lscale = 841f32 / 108f32;
    let lbias = 16f32 / 116f32;
    let cb_tab_scale = 1f32 / (f255 * (1 << GAMMA_SHIFT) as f32);
    let lshift2 = (1 << LAB_SHIFT2) as f32;
    let cbrt = (0..CBRT_TAB_SIZE)
        .map(|i| {
            let x = cb_tab_scale * i as f32;
            let f = if x < lthresh {
                x.mul_add(lscale, lbias)
            } else {
                softfloat_cbrt(x)
            };
            round_even(f64::from(lshift2 * f))
        })
        .collect();

    let inv_scale = 1f32 / INV_GAMMA_TAB_SIZE as f32;
    let inv_gamma = (0..INV_GAMMA_TAB_SIZE)
        .map(|i| round_even(f64::from(f255 * apply_inv_gamma(inv_scale * i as f32))))
        .collect();

    let base = LUT_BASE as f32;
    let l_to_yf = (0..256i32)
        .map(|i| {
            // 8 * 255 / 100 = 20.4: the 8-bit L* below which Y is linear in L*.
            if i <= 20 {
                let y = round_even(f64::from(
                    (i * LUT_BASE * 20 * 9) as f32 / (17 * 29 * 29 * 29) as f32,
                ));
                let ify = round_even(f64::from(
                    base * (16f32 / 116f32 + (i * 5) as f32 / (3 * 17 * 29) as f32),
                ));
                (y, ify)
            } else {
                let fy = (i * 100 * LUT_BASE) as f32 / (255 * 116) as f32
                    + (16 * LUT_BASE) as f32 / 116f32;
                let ify = round_even(f64::from(fy));
                let y = round_even(f64::from(fy * fy * fy / (LUT_BASE * LUT_BASE) as f32));
                (y, ify)
            }
        })
        .collect();

    let ab_to_xz = (MIN_AB..LUT_BASE * 9 / 4 + MIN_AB)
        .map(|i| {
            // 6/29 * LUT_BASE = 3389.73: f(t)'s linear segment below, its cube above.
            if i <= 3390 {
                i * 108 / 841 - LUT_BASE * 16 / 116 * 108 / 841
            } else {
                i * i / LUT_BASE * i / LUT_BASE
            }
        })
        .collect();

    let lshift = f64::from(1 << LAB_SHIFT);
    let mut to_xyz = [[0i32; 3]; 3];
    let mut to_rgb = [[0i32; 3]; 3];
    for row in 0..3 {
        for col in 0..3 {
            to_xyz[row][col] = round_even(
                lshift * f64::from_bits(SRGB_TO_XYZ[row * 3 + col]) / f64::from_bits(D65[row]),
            );
            to_rgb[row][col] = round_even(
                lshift * f64::from_bits(XYZ_TO_SRGB[row * 3 + col]) * f64::from_bits(D65[col]),
            );
        }
    }

    Tables {
        gamma,
        cbrt,
        inv_gamma,
        l_to_yf,
        ab_to_xz,
        to_xyz,
        to_rgb,
    }
}

fn tables() -> &'static Tables {
    static TABLES: OnceLock<Tables> = OnceLock::new();
    TABLES.get_or_init(build)
}

/// `CV_DESCALE`: shift right by `n`, rounding half up.
fn descale(value: i32, n: i32) -> i32 {
    (value + (1 << (n - 1))) >> n
}

fn saturate_u8(value: i32) -> u8 {
    value.clamp(0, 255) as u8
}

fn dot(row: &[i32; 3], v: [i32; 3]) -> i32 {
    row[0] * v[0] + row[1] * v[1] + row[2] * v[2]
}

/// One 8-bit RGB pixel to OpenCV's 8-bit L\*a\*b\*: `L * 255 / 100`, and `a`, `b` offset by 128.
pub fn rgb_to_lab(rgb: [u8; 3]) -> [u8; 3] {
    let t = tables();
    let linear = rgb.map(|v| t.gamma[usize::from(v)]);
    let f = |row: usize| t.cbrt[descale(dot(&t.to_xyz[row], linear), LAB_SHIFT) as usize];
    let (fx, fy, fz) = (f(0), f(1), f(2));
    // `Lscale` and `Lshift`: 116 * 255 / 100 and 16 * 255 / 100, rounded as the C++ does.
    let l_scale = (116 * 255 + 50) / 100;
    let l_shift = -((16 * 255 * (1 << LAB_SHIFT2) + 50) / 100);
    let ab_shift = 128 * (1 << LAB_SHIFT2);
    [
        saturate_u8(descale(l_scale * fy + l_shift, LAB_SHIFT2)),
        saturate_u8(descale(500 * (fx - fy) + ab_shift, LAB_SHIFT2)),
        saturate_u8(descale(200 * (fy - fz) + ab_shift, LAB_SHIFT2)),
    ]
}

/// One 8-bit L\*a\*b\* pixel, as [`rgb_to_lab`] writes them, back to 8-bit RGB.
pub fn lab_to_rgb(lab: [u8; 3]) -> [u8; 3] {
    let t = tables();
    let [l, a, b] = lab.map(i32::from);
    let (y, ify) = t.l_to_yf[l as usize];
    // `a * LUT_BASE / 500` and `b * LUT_BASE / 200`, by OpenCV's integer approximations.
    let adiv = ((5 * a * 53687 + (1 << 7)) >> 13) - 128 * LUT_BASE / 500;
    let bdiv = ((b * 41943 + (1 << 4)) >> 9) - 128 * LUT_BASE / 200 + 1;
    let x = t.ab_to_xz[(ify + adiv - MIN_AB) as usize];
    let z = t.ab_to_xz[(ify - bdiv - MIN_AB) as usize];
    // `Lab2RGBinteger::shift`: the XYZ sums land in the inverse gamma table's 4096 steps.
    let shift = LAB_SHIFT + (BASE_SHIFT - INV_GAMMA_SHIFT);
    let channel = |row: usize| {
        let linear = descale(dot(&t.to_rgb[row], [x, y, z]), shift);
        saturate_u8(t.inv_gamma[linear.clamp(0, INV_GAMMA_TAB_SIZE as i32 - 1) as usize])
    };
    [channel(0), channel(1), channel(2)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_coefficients_are_opencvs() {
        // `sRGB2XYZ_D65_i`, OpenCV's integer copy of the same matrix before the white point,
        // differs from these only by the white point; the Y row has none, so it must match.
        assert_eq!(tables().to_xyz[1], [871, 2929, 296]);
    }

    #[test]
    fn the_cube_root_table_is_opencvs_where_a_correctly_rounded_root_is_not() {
        // 32768 * cbrt(324 / 2040) is within a hair of 17745.5; OpenCV's truncated root lands
        // below the half and a correctly rounded one does not.
        let x = (1f32 / 2040f32) * 324f32;
        assert_eq!(tables().cbrt[324], 17745);
        assert_eq!(
            (32768f32 * (f64::from(x).cbrt() as f32)).round_ties_even(),
            17746.0
        );
        // Elsewhere the two agree within one f32 step, the truncation's whole effect.
        for i in (19..CBRT_TAB_SIZE).step_by(7) {
            let x = (1f32 / 2040f32) * i as f32;
            let (ours, library) = (softfloat_cbrt(x), f64::from(x).cbrt() as f32);
            assert!(ours.to_bits().abs_diff(library.to_bits()) <= 1, "{i}");
        }
        assert_eq!(softfloat_cbrt(0.125), 0.5);
        assert_eq!(softfloat_cbrt(-0.0).to_bits(), 0);
    }

    #[test]
    fn black_white_and_grey_land_on_the_neutral_axis() {
        assert_eq!(rgb_to_lab([0, 0, 0]), [0, 128, 128]);
        assert_eq!(rgb_to_lab([255, 255, 255]), [255, 128, 128]);
        let [_, a, b] = rgb_to_lab([128, 128, 128]);
        assert_eq!((a, b), (128, 128));
        assert_eq!(lab_to_rgb([0, 128, 128]), [0, 0, 0]);
        assert_eq!(lab_to_rgb([255, 128, 128]), [255, 255, 255]);
    }

    #[test]
    fn the_return_trip_reaches_every_table_index_it_asks_for() {
        // The extremes of a and b at the extremes of L index the ab table's two ends.
        for lab in [[0, 0, 255], [255, 255, 0], [0, 255, 255], [255, 0, 0]] {
            let _ = lab_to_rgb(lab);
        }
    }
}
