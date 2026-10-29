//! `cv2.createCLAHE(clipLimit, tileGridSize).apply(...)` on an 8-bit single channel. [M29 P3]
//!
//! The twin of OpenCV 4.10's `CLAHE_Impl::apply` with `CLAHE_CalcLut_Body` and
//! `CLAHE_Interpolation_Body` (`imgproc/src/clahe.cpp`), which `preprocess.py`'s `enhance` runs on
//! the L\*a\*b\* lightness at clip 2.0 over an 8x8 grid. Each tile's histogram is clipped, the
//! excess spread back evenly with the remainder dealt out at a stride, and turned into a lookup
//! table; each pixel then blends the four nearest tiles' tables bilinearly in `f32`, in the C++'s
//! order of operations, and rounds half to even. No IPP path exists for it.
//!
//! # The padding quirk
//!
//! When the image does not divide into the grid **on both axes**, OpenCV pads it on the right and
//! bottom by reflection (`BORDER_REFLECT_101`) before cutting tiles, by `tiles - size % tiles` on
//! **each** axis. An axis that already divided is padded by a whole extra `tiles` pixels, so a
//! 1600x1093 image (the working size of 2026-08-23-1) gets tiles 201 px wide, not 200. The padded
//! pixels count only in the histograms; the blend runs over the image itself, with the padded tile
//! size. Followed here exactly, because it changes every table.

use crate::cv::resize::saturate_u8;

/// `borderInterpolate(p, len, BORDER_REFLECT_101)` for the one direction CLAHE pads, past the end.
fn reflect_101(mut p: usize, len: usize) -> usize {
    if len == 1 {
        return 0;
    }
    // `gfedcb|abcdefgh|gfedcba`: the edge pixel is not repeated.
    while p >= len {
        p = (2 * len - 2).abs_diff(p);
    }
    p
}

/// CLAHE over `src`, `width` by `height` grey pixels, with OpenCV's `clipLimit` and a grid of
/// `tiles_x` by `tiles_y` tiles.
pub fn clahe(
    src: &[u8],
    width: usize,
    height: usize,
    clip_limit: f64,
    tiles_x: usize,
    tiles_y: usize,
) -> Vec<u8> {
    const HIST_SIZE: usize = 256;
    assert_eq!(src.len(), width * height);
    assert!(width > 0 && height > 0 && tiles_x > 0 && tiles_y > 0);

    let (ext_width, ext_height) = if width % tiles_x == 0 && height % tiles_y == 0 {
        (width, height)
    } else {
        (
            width + tiles_x - width % tiles_x,
            height + tiles_y - height % tiles_y,
        )
    };
    let (tile_w, tile_h) = (ext_width / tiles_x, ext_height / tiles_y);
    let tile_total = tile_w * tile_h;
    let lut_scale = (HIST_SIZE - 1) as f32 / tile_total as f32;
    let clip = if clip_limit > 0.0 {
        ((clip_limit * tile_total as f64 / HIST_SIZE as f64) as i32).max(1)
    } else {
        0
    };

    // One lookup table per tile, row-major over the grid (`lut_`).
    let mut luts = vec![[0u8; HIST_SIZE]; tiles_x * tiles_y];
    for (k, lut) in luts.iter_mut().enumerate() {
        let (ty, tx) = (k / tiles_x, k % tiles_x);
        let mut hist = [0i32; HIST_SIZE];
        for y in ty * tile_h..(ty + 1) * tile_h {
            let row = &src[reflect_101(y, height) * width..][..width];
            for x in tx * tile_w..(tx + 1) * tile_w {
                hist[usize::from(row[reflect_101(x, width)])] += 1;
            }
        }

        if clip > 0 {
            let mut clipped = 0;
            for count in hist.iter_mut() {
                if *count > clip {
                    clipped += *count - clip;
                    *count = clip;
                }
            }
            let batch = clipped / HIST_SIZE as i32;
            let mut residual = clipped - batch * HIST_SIZE as i32;
            for count in hist.iter_mut() {
                *count += batch;
            }
            if residual != 0 {
                let step = (HIST_SIZE / residual as usize).max(1);
                let mut i = 0;
                while i < HIST_SIZE && residual > 0 {
                    hist[i] += 1;
                    i += step;
                    residual -= 1;
                }
            }
        }

        let mut sum = 0i32;
        for (entry, count) in lut.iter_mut().zip(hist) {
            sum += count;
            *entry = saturate_u8(sum as f32 * lut_scale);
        }
    }

    // Each column's two tiles and its weights, as the constructor tabulates them.
    let inv_tw = 1f32 / tile_w as f32;
    let columns: Vec<(usize, usize, f32, f32)> = (0..width)
        .map(|x| {
            let txf = x as f32 * inv_tw - 0.5;
            let tx1 = txf.floor() as i64;
            let xa = txf - tx1 as f32;
            let left = tx1.max(0) as usize;
            let right = (tx1 + 1).min(tiles_x as i64 - 1) as usize;
            (left, right, xa, 1.0 - xa)
        })
        .collect();

    let inv_th = 1f32 / tile_h as f32;
    let mut dst = vec![0u8; width * height];
    for y in 0..height {
        let tyf = y as f32 * inv_th - 0.5;
        let ty1 = tyf.floor() as i64;
        let ya = tyf - ty1 as f32;
        let ya1 = 1.0 - ya;
        let top = &luts[ty1.max(0) as usize * tiles_x..];
        let bottom = &luts[(ty1 + 1).min(tiles_y as i64 - 1) as usize * tiles_x..];
        for (x, &(left, right, xa, xa1)) in columns.iter().enumerate() {
            let v = usize::from(src[y * width + x]);
            let res = (f32::from(top[left][v]) * xa1 + f32::from(top[right][v]) * xa) * ya1
                + (f32::from(bottom[left][v]) * xa1 + f32::from(bottom[right][v]) * xa) * ya;
            dst[y * width + x] = saturate_u8(res);
        }
    }
    dst
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reflection_skips_the_edge_pixel() {
        assert_eq!(reflect_101(5, 6), 5);
        assert_eq!(reflect_101(6, 6), 4);
        assert_eq!(reflect_101(7, 6), 3);
        assert_eq!(reflect_101(3, 1), 0);
    }

    #[test]
    fn a_flat_image_maps_every_pixel_to_the_same_level() {
        // 8x6 tiles of 48 pixels, all at level 100. The clip is int(2 * 48 / 256) = 0, raised to
        // 1, so 47 pixels are clipped: none per level evenly, and all 47 as the residual, one
        // every 256 / 47 = 5 levels from 0. Up to level 100 that is 21 of them plus the one left
        // at 100, and 22 * 255 / 48 = 116.875 rounds to 117.
        let out = clahe(&[100; 64 * 48], 64, 48, 2.0, 8, 8);
        assert!(out.iter().all(|&v| v == 117));
    }
}
