//! A JPEG file in, the pixels `cv2.imread` decodes out. [M29 P2]
//!
//! **No Python twin; the twin is C**: libjpeg-turbo 3.1 as OpenCV 4.10 drives it inside frozen
//! `preprocess.py`'s `load_image` (`cv2.imread(path, IMREAD_COLOR)`), and Pillow's decode is the
//! same library with the same answer. Each step names the libjpeg-turbo file it follows:
//!
//! - the markers: `jdmarker.c`, with `jdapimin.c`'s rule for which colour space three components
//!   are in;
//! - the entropy decode: `jdhuff.c` (`jpeg_make_d_derived_tbl`, `decode_mcu`), with
//!   `jutils.c`'s `jpeg_natural_order` and its guard entries;
//! - the inverse DCT: `jidctint.c`'s `jpeg_idct_islow`, the default `JDCT_ISLOW` that neither
//!   OpenCV nor Pillow changes, with `jdmaster.c`'s range-limit table after it;
//! - the chroma upsampling: `jdsample.c`'s default "fancy" (triangle) filters, with `jdmainct.c`'s
//!   rule for the rows above the first and below the last;
//! - the colour conversion: `jdcolor.c`'s YCbCr tables.
//!
//! Then OpenCV's own step: the EXIF orientation, read from the **first** APP1 segment
//! (`grfmt_jpeg.cpp`, `exif.cpp`) and applied as `ExifTransform` applies it.
//!
//! # Why by hand
//!
//! The `image` crate decodes JPEG with `zune-jpeg`, whose IDCT is not libjpeg-turbo's ISLOW. The
//! two land one level apart on about 0.5% of values, and on two of the ten bay JPEGs that moved the
//! screen quad: a rotated box found where Python finds none, and a corner 114 px away (the M29
//! plan's P2 finding). ffmpeg's `int` and `simple` IDCTs differ by more. The user chose this over
//! linking libjpeg-turbo through a C crate (2026-10-16): it is D4's rule, a hand port and no native
//! OpenCV, applied to the decoder.
//!
//! # How close it is (M29 P2)
//!
//! Identical pixels on all ten bay JPEGs (`tests/quad.rs`), and on `tests/fixtures/jpeg/`'s small
//! photos written by libjpeg-turbo and Pillow, which cover what the bay photos do not: 4:2:2,
//! 4:2:0, 4:4:0 and 4:1:1 sampling, the narrow photos whose chroma is too thin for the fancy
//! filters, restart intervals, grey, RGB by component id, and all eight EXIF orientations
//! (`tests/jpeg.rs`).
//!
//! # What it reads, and what it refuses
//!
//! Baseline and extended sequential Huffman JPEG (`SOF0`, `SOF1`), 8-bit, with one component
//! (grey) or three (YCbCr, or RGB by libjpeg's rule), at every sampling libjpeg-turbo's upsampler
//! accepts. Every bay JPEG is baseline 4:4:4, and an iPhone writes HEIC or baseline JPEG.
//!
//! It refuses by name, as [`JpegError::Unsupported`], the kinds libjpeg-turbo reads and it does
//! not: progressive, lossless, arithmetic-coded, 12-bit and four-component (CMYK, YCCK) files.
//! Progressive is the one a photo could plausibly arrive as (a web export); its coefficients are
//! the sequential ones reached in several passes, so supporting it is `jdphuff.c`'s four scan
//! decoders and a whole-image coefficient buffer, with this IDCT unchanged.
//!
//! Where libjpeg-turbo warns and carries on over a damaged file, this refuses, as
//! [`JpegError::Corrupt`]: a Huffman code no table holds (libjpeg substitutes a zero), entropy
//! data that runs out before the last block (it pads with zero bits), a restart marker out of
//! sequence (it resynchronises), a component no scan codes (it leaves it grey). `cv2` would hand
//! back an image with grey or garbled blocks, and a reading from that is worse than none (ADR-010
//! §2's rule, applied to a photo). It also refuses a scan that names an undefined Huffman table,
//! where libjpeg-turbo first fills slots 0 and 1 with the standard tables for Motion-JPEG frames
//! (`jstdhuff.c`). A still photo always carries its tables.
//!
//! Two small departures from libjpeg-turbo's arithmetic, neither reachable by a valid 8-bit file:
//! the IDCT runs in `i64` (libjpeg's `JLONG` is 32 bits on Windows, and its AVX2 kernel
//! dequantises in 16), and a DC prediction wraps instead of overflowing. OpenCV's EXIF reader also
//! stops early on some malformed entries before the orientation tag (a string whose offset lies
//! outside the segment); this reads the orientation entry alone.

use image::metadata::Orientation;
use image::{DynamicImage, RgbImage};

/// Why a JPEG could not be decoded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum JpegError {
    /// A kind of JPEG this decoder does not read, named ("progressive", "four-component", ...).
    Unsupported(String),
    /// Not a well-formed JPEG, or one damaged in a way libjpeg-turbo would paper over.
    Corrupt(String),
}

fn corrupt(reason: impl Into<String>) -> JpegError {
    JpegError::Corrupt(reason.into())
}

/// The three bytes `cv2.imread` takes for a JPEG signature (`JpegDecoder::signatureLength`).
pub(crate) fn is_jpeg(bytes: &[u8]) -> bool {
    bytes.starts_with(&[0xFF, 0xD8, 0xFF])
}

/// Decode a JPEG to 8-bit RGB with its EXIF orientation applied: `cv2.imread`'s pixels, in RGB
/// order rather than OpenCV's BGR.
pub(crate) fn decode(bytes: &[u8]) -> Result<RgbImage, JpegError> {
    let mut decoder = Decoder::default();
    decoder.read(bytes)?;
    let image = decoder.output()?;
    let orientation = decoder.exif.and_then(exif_orientation);
    Ok(apply_orientation(image, orientation))
}

/// `jpeg_natural_order`: the zigzag position of each coefficient, then the 16 guard entries
/// libjpeg keeps so that a run that overshoots 63 in a damaged block lands on coefficient 63
/// rather than out of bounds. Mirrored rather than refused, because libjpeg's answer for that
/// block is then still a definite one.
const NATURAL_ORDER: [usize; 80] = [
    0, 1, 8, 16, 9, 2, 3, 10, 17, 24, 32, 25, 18, 11, 4, 5, 12, 19, 26, 33, 40, 48, 41, 34, 27, 20,
    13, 6, 7, 14, 21, 28, 35, 42, 49, 56, 57, 50, 43, 36, 29, 22, 15, 23, 30, 37, 44, 51, 58, 59,
    52, 45, 38, 31, 39, 46, 53, 60, 61, 54, 47, 55, 62, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63, 63,
    63, 63, 63, 63, 63, 63,
];

/// libjpeg's `D_MAX_BLOCKS_IN_MCU`: an interleaved scan whose MCU holds more blocks is refused
/// (`JERR_BAD_MCU_SIZE`), and `cv2.imread` returns nothing.
const MAX_BLOCKS_IN_MCU: usize = 10;

/// libjpeg's `JPEG_MAX_DIMENSION`, and OpenCV's `CV_IO_MAX_IMAGE_PIXELS` (`1 << 30`), past which
/// `cv2.imread` refuses a photo. They are also what keeps a hostile header from asking for a
/// buffer of many gigabytes.
const MAX_DIMENSION: usize = 65500;
const MAX_PIXELS: usize = 1 << 30;

#[derive(Default)]
struct Decoder<'a> {
    /// Each quantisation table in natural (not zigzag) order, as `get_dqt` stores it.
    quant: [Option<[u16; 64]>; 4],
    /// Each Huffman table as its DHT segment defines it; derived when a scan first uses it,
    /// as `jdhuff.c` does, so a bad table no scan uses is no error.
    dc: [Option<HuffmanSpec>; 4],
    ac: [Option<HuffmanSpec>; 4],
    frame: Option<Frame>,
    restart_interval: usize,
    saw_jfif: bool,
    adobe_transform: Option<u8>,
    /// The first APP1 segment's payload seen before the first scan: the one OpenCV parses.
    exif: Option<&'a [u8]>,
    seen_scan: bool,
}

struct Frame {
    width: usize,
    height: usize,
    max_h: usize,
    max_v: usize,
    /// Interleaved MCUs across and down: `ceil(width / (8 * max_h))` and the height's twin.
    mcus_x: usize,
    mcus_y: usize,
    components: Vec<Component>,
}

struct Component {
    id: u8,
    h: usize,
    v: usize,
    quant_table: usize,
    /// `width_in_blocks` and `height_in_blocks`: what a non-interleaved scan codes.
    width_in_blocks: usize,
    height_in_blocks: usize,
    /// `downsampled_width` and `downsampled_height`: the component's real samples, which the
    /// upsampler reads and replicates past.
    downsampled_width: usize,
    downsampled_height: usize,
    /// The decoded samples, `stride` per row: the interleaved MCU grid's blocks, which covers a
    /// non-interleaved scan's too.
    samples: Vec<u8>,
    stride: usize,
    coded: bool,
}

#[derive(Clone)]
struct HuffmanSpec {
    /// `bits[l]`: how many codes have length `l`, for `l` in 1..=16 (`bits[0]` unused).
    bits: [u8; 17],
    values: Vec<u8>,
}

impl<'a> Decoder<'a> {
    /// `read_markers`, to the end of the image.
    fn read(&mut self, data: &'a [u8]) -> Result<(), JpegError> {
        if !data.starts_with(&[0xFF, 0xD8]) {
            return Err(corrupt("no start-of-image marker"));
        }
        let mut pos = 2;
        // A file that ends without EOI, after its last scan: libjpeg-turbo warns
        // (`JWRN_JPEG_EOF`) and outputs what it decoded, and `output` refuses only if a
        // component was never coded.
        while let Some((marker, after)) = next_marker(data, pos) {
            pos = after;
            match marker {
                0xD9 => break,
                0xD8 => return Err(corrupt("a second start-of-image marker")),
                // RSTn and TEM carry no parameters; outside a scan libjpeg ignores them.
                0xD0..=0xD7 | 0x01 => {}
                _ => {
                    let payload = segment(data, pos, marker)?;
                    pos += 2 + payload.len();
                    match marker {
                        0xC0 | 0xC1 => self.start_of_frame(payload)?,
                        0xC2 => return Err(unsupported("progressive")),
                        0xC3 => return Err(unsupported("lossless")),
                        0xC5..=0xC7 | 0xCD..=0xCF => return Err(unsupported("hierarchical")),
                        0xC9..=0xCB => return Err(unsupported("arithmetic-coded")),
                        0xC4 => self.define_huffman(payload)?,
                        0xDB => self.define_quant(payload)?,
                        0xDD => self.define_restart_interval(payload)?,
                        0xDA => pos = self.scan(data, payload, pos)?,
                        0xE0 => self.app0(payload),
                        0xE1 => {
                            if self.exif.is_none() && !self.seen_scan {
                                self.exif = Some(payload);
                            }
                        }
                        0xEE => self.app14(payload),
                        // Every other APPn, COM, DNL and DAC: skipped by their length, as libjpeg
                        // skips them.
                        0xE2..=0xED | 0xEF | 0xFE | 0xDC | 0xCC => {}
                        // JPG, DHP, EXP, JPGn, RESn: libjpeg's `JERR_UNKNOWN_MARKER`.
                        other => return Err(corrupt(format!("an unknown marker 0xFF{other:02X}"))),
                    }
                }
            }
        }
        Ok(())
    }

    /// `get_sof` and `initial_setup`.
    fn start_of_frame(&mut self, payload: &[u8]) -> Result<(), JpegError> {
        if self.frame.is_some() {
            return Err(corrupt("a second start-of-frame marker"));
        }
        if payload.len() < 6 {
            return Err(corrupt(
                "a start-of-frame segment too short to hold its header",
            ));
        }
        let precision = payload[0];
        let height = usize::from(u16::from_be_bytes([payload[1], payload[2]]));
        let width = usize::from(u16::from_be_bytes([payload[3], payload[4]]));
        let count = usize::from(payload[5]);
        if precision != 8 {
            return Err(unsupported(format!("{precision}-bit")));
        }
        if width == 0 || height == 0 || count == 0 {
            // Height 0 means a DNL marker gives it later, which libjpeg refuses too.
            return Err(corrupt(
                "an empty image (a zero dimension or no components)",
            ));
        }
        if payload.len() != 6 + 3 * count {
            return Err(corrupt(
                "a start-of-frame segment whose length disagrees with it",
            ));
        }
        match count {
            1 | 3 => {}
            4 => return Err(unsupported("four-component (CMYK or YCCK)")),
            other => return Err(unsupported(format!("{other}-component"))),
        }
        if width > MAX_DIMENSION || height > MAX_DIMENSION || width * height > MAX_PIXELS {
            return Err(corrupt(format!(
                "a {width}x{height} image, larger than OpenCV reads"
            )));
        }
        let mut components = Vec::with_capacity(count);
        for spec in payload[6..].chunks_exact(3) {
            let (h, v) = (usize::from(spec[1] >> 4), usize::from(spec[1] & 15));
            if !(1..=4).contains(&h) || !(1..=4).contains(&v) {
                return Err(corrupt("a sampling factor outside 1 to 4"));
            }
            if usize::from(spec[2]) >= 4 {
                return Err(corrupt("a quantisation table number outside 0 to 3"));
            }
            if components.iter().any(|c: &Component| c.id == spec[0]) {
                // libjpeg-turbo renames a duplicate id and carries on; nothing well-formed has one.
                return Err(unsupported("duplicate-component-id"));
            }
            components.push(Component {
                id: spec[0],
                h,
                v,
                quant_table: usize::from(spec[2]),
                width_in_blocks: 0,
                height_in_blocks: 0,
                downsampled_width: 0,
                downsampled_height: 0,
                samples: Vec::new(),
                stride: 0,
                coded: false,
            });
        }
        let max_h = components.iter().map(|c| c.h).max().unwrap_or(1);
        let max_v = components.iter().map(|c| c.v).max().unwrap_or(1);
        let mcus_x = width.div_ceil(8 * max_h);
        let mcus_y = height.div_ceil(8 * max_v);
        for c in &mut components {
            c.width_in_blocks = (width * c.h).div_ceil(8 * max_h);
            c.height_in_blocks = (height * c.v).div_ceil(8 * max_v);
            c.downsampled_width = (width * c.h).div_ceil(max_h);
            c.downsampled_height = (height * c.v).div_ceil(max_v);
            c.stride = mcus_x * c.h * 8;
            c.samples = vec![0; c.stride * mcus_y * c.v * 8];
        }
        self.frame = Some(Frame {
            width,
            height,
            max_h,
            max_v,
            mcus_x,
            mcus_y,
            components,
        });
        Ok(())
    }

    /// `get_dht`: one or more tables, each class and slot, counts and values.
    fn define_huffman(&mut self, mut payload: &[u8]) -> Result<(), JpegError> {
        while !payload.is_empty() {
            if payload.len() < 17 {
                return Err(corrupt("a Huffman table segment with a truncated table"));
            }
            let index = payload[0];
            let mut bits = [0u8; 17];
            bits[1..].copy_from_slice(&payload[1..17]);
            let count: usize = bits.iter().map(|&b| usize::from(b)).sum();
            if count > 256 || payload.len() < 17 + count {
                return Err(corrupt(
                    "a Huffman table with more codes than it holds values",
                ));
            }
            let values = payload[17..17 + count].to_vec();
            payload = &payload[17 + count..];
            let (slot, class) = (usize::from(index & 0x0F), index >> 4);
            let table = match (class, slot) {
                (0, 0..=3) => &mut self.dc[slot],
                (1, 0..=3) => &mut self.ac[slot],
                _ => return Err(corrupt(format!("a Huffman table index 0x{index:02X}"))),
            };
            *table = Some(HuffmanSpec { bits, values });
        }
        Ok(())
    }

    /// `get_dqt`: one or more tables, 8- or 16-bit, zigzag order in the file.
    fn define_quant(&mut self, mut payload: &[u8]) -> Result<(), JpegError> {
        while !payload.is_empty() {
            let (precision, slot) = (payload[0] >> 4, usize::from(payload[0] & 0x0F));
            if slot >= 4 {
                return Err(corrupt(format!("a quantisation table number {slot}")));
            }
            let size = if precision == 0 { 64 } else { 128 };
            if payload.len() < 1 + size {
                return Err(corrupt(
                    "a quantisation table segment with a truncated table",
                ));
            }
            let mut table = [0u16; 64];
            for (k, &position) in NATURAL_ORDER[..64].iter().enumerate() {
                table[position] = if precision == 0 {
                    u16::from(payload[1 + k])
                } else {
                    u16::from_be_bytes([payload[1 + 2 * k], payload[2 + 2 * k]])
                };
            }
            self.quant[slot] = Some(table);
            payload = &payload[1 + size..];
        }
        Ok(())
    }

    fn define_restart_interval(&mut self, payload: &[u8]) -> Result<(), JpegError> {
        let [high, low] = payload else {
            return Err(corrupt("a restart interval segment of the wrong length"));
        };
        self.restart_interval = usize::from(u16::from_be_bytes([*high, *low]));
        Ok(())
    }

    /// `examine_app0`: a JFIF marker says three components are YCbCr, whatever their ids.
    fn app0(&mut self, payload: &[u8]) {
        if payload.len() >= 14 && payload.starts_with(b"JFIF\0") {
            self.saw_jfif = true;
        }
    }

    /// `examine_app14`: an Adobe marker's transform byte says RGB (0) or YCbCr (1).
    fn app14(&mut self, payload: &[u8]) {
        if payload.len() >= 12 && payload.starts_with(b"Adobe") {
            self.adobe_transform = Some(payload[11]);
        }
    }

    /// `get_sos`, then every MCU of the scan through `decode_mcu`, each block straight through the
    /// IDCT into its component's samples. Returns where the marker after the scan starts.
    fn scan(&mut self, data: &[u8], header: &[u8], start: usize) -> Result<usize, JpegError> {
        self.seen_scan = true;
        let frame = self
            .frame
            .as_mut()
            .ok_or_else(|| corrupt("a scan before any start-of-frame marker"))?;
        let count = usize::from(*header.first().unwrap_or(&0));
        if count == 0 || count > 4 || header.len() != 1 + 2 * count + 3 {
            return Err(corrupt(
                "a start-of-scan segment whose length disagrees with it",
            ));
        }
        // Ss, Se, Ah and Al (the header's last three bytes) are ignored: a sequential scan is
        // all 64 coefficients whatever they say, and libjpeg only warns (`JWRN_NOT_SEQUENTIAL`)
        // because some baseline encoders write zeros there.
        let mut members = Vec::with_capacity(count);
        for spec in header[1..1 + 2 * count].chunks_exact(2) {
            let index = frame
                .components
                .iter()
                .position(|c| c.id == spec[0])
                .ok_or_else(|| corrupt(format!("a scan naming no component {}", spec[0])))?;
            if members.iter().any(|m: &ScanMember| m.index == index) {
                return Err(corrupt("a scan naming one component twice"));
            }
            let component = &frame.components[index];
            let quant = self.quant[component.quant_table].ok_or_else(|| {
                corrupt(format!(
                    "component {} uses quantisation table {}, which the file never defines",
                    component.id, component.quant_table
                ))
            })?;
            let derive = |tables: &[Option<HuffmanSpec>; 4], slot: usize, is_dc: bool| {
                tables
                    .get(slot)
                    .and_then(Option::as_ref)
                    .ok_or_else(|| {
                        corrupt(format!(
                            "a scan uses Huffman table {slot}, which the file never defines"
                        ))
                    })
                    .and_then(|spec| Huffman::derive(spec, is_dc))
            };
            members.push(ScanMember {
                index,
                dc: derive(&self.dc, usize::from(spec[1] >> 4), true)?,
                ac: derive(&self.ac, usize::from(spec[1] & 15), false)?,
                quant,
                prediction: 0,
            });
        }

        // An interleaved scan walks the frame's MCU grid; a single-component one walks that
        // component's own blocks, one per MCU, and so codes none of the grid's padding blocks.
        let (mcus_x, mcus_y) = if count == 1 {
            let c = &frame.components[members[0].index];
            (c.width_in_blocks, c.height_in_blocks)
        } else {
            let blocks: usize = members
                .iter()
                .map(|m| frame.components[m.index].h * frame.components[m.index].v)
                .sum();
            if blocks > MAX_BLOCKS_IN_MCU {
                return Err(corrupt(format!(
                    "an MCU of {blocks} blocks, more than the {MAX_BLOCKS_IN_MCU} JPEG allows"
                )));
            }
            (frame.mcus_x, frame.mcus_y)
        };

        let mut bits = BitReader::new(data, start);
        let mut coefficients = [0i16; 64];
        let total = mcus_x * mcus_y;
        let mut restarts_to_go = self.restart_interval;
        let mut next_restart = 0u8;
        for mcu in 0..total {
            if self.restart_interval > 0 {
                if restarts_to_go == 0 {
                    bits.restart(next_restart)?;
                    next_restart = (next_restart + 1) & 7;
                    restarts_to_go = self.restart_interval;
                    for member in &mut members {
                        member.prediction = 0;
                    }
                }
                restarts_to_go -= 1;
            }
            let (mx, my) = (mcu % mcus_x, mcu / mcus_x);
            for member in &mut members {
                let component = &mut frame.components[member.index];
                let (bw, bh) = if count == 1 {
                    (1, 1)
                } else {
                    (component.h, component.v)
                };
                for by in 0..bh {
                    for bx in 0..bw {
                        decode_block(
                            &mut bits,
                            &member.dc,
                            &member.ac,
                            &mut member.prediction,
                            &mut coefficients,
                        )?;
                        let (x, y) = (mx * bw + bx, my * bh + by);
                        let offset = y * 8 * component.stride + x * 8;
                        idct_islow(
                            &coefficients,
                            &member.quant,
                            &mut component.samples[offset..],
                            component.stride,
                        );
                    }
                }
            }
        }
        for member in &members {
            frame.components[member.index].coded = true;
        }
        Ok(bits.end())
    }

    /// Upsample each component to the photo's size and convert to RGB.
    fn output(&self) -> Result<RgbImage, JpegError> {
        let frame = self
            .frame
            .as_ref()
            .ok_or_else(|| corrupt("no start-of-frame marker"))?;
        if let Some(c) = frame.components.iter().find(|c| !c.coded) {
            return Err(corrupt(format!(
                "component {} is never coded by any scan (the file is cut short?)",
                c.id
            )));
        }
        let planes = frame
            .components
            .iter()
            .map(|c| upsample(c, frame.max_h, frame.max_v))
            .collect::<Result<Vec<_>, _>>()?;
        let (width, height) = (frame.width, frame.height);
        let mut rgb = vec![0u8; width * height * 3];
        match (planes.as_slice(), self.colour_space(frame)) {
            ([grey], _) => {
                for y in 0..height {
                    let row = &grey.samples[y * grey.stride..][..width];
                    for (out, &g) in rgb[y * width * 3..][..width * 3]
                        .chunks_exact_mut(3)
                        .zip(row)
                    {
                        out.copy_from_slice(&[g, g, g]);
                    }
                }
            }
            ([a, b, c], ColourSpace::Rgb) => {
                for y in 0..height {
                    let rows = (
                        &a.samples[y * a.stride..][..width],
                        &b.samples[y * b.stride..][..width],
                        &c.samples[y * c.stride..][..width],
                    );
                    for (x, out) in rgb[y * width * 3..][..width * 3]
                        .chunks_exact_mut(3)
                        .enumerate()
                    {
                        out.copy_from_slice(&[rows.0[x], rows.1[x], rows.2[x]]);
                    }
                }
            }
            ([luma, cb, cr], ColourSpace::YCbCr) => {
                let tables = YccTables::new();
                for y in 0..height {
                    let rows = (
                        &luma.samples[y * luma.stride..][..width],
                        &cb.samples[y * cb.stride..][..width],
                        &cr.samples[y * cr.stride..][..width],
                    );
                    for (x, out) in rgb[y * width * 3..][..width * 3]
                        .chunks_exact_mut(3)
                        .enumerate()
                    {
                        out.copy_from_slice(&tables.rgb(rows.0[x], rows.1[x], rows.2[x]));
                    }
                }
            }
            _ => unreachable!("start_of_frame admits one or three components"),
        }
        RgbImage::from_raw(width as u32, height as u32, rgb)
            .ok_or_else(|| corrupt("the decoded buffer does not fit the image"))
    }

    /// `default_decompress_parms` for three components: JFIF first, then Adobe's transform, then
    /// the component ids ('R', 'G', 'B' means RGB), and YCbCr otherwise.
    fn colour_space(&self, frame: &Frame) -> ColourSpace {
        if self.saw_jfif {
            return ColourSpace::YCbCr;
        }
        if let Some(transform) = self.adobe_transform {
            return if transform == 0 {
                ColourSpace::Rgb
            } else {
                ColourSpace::YCbCr
            };
        }
        let ids: Vec<u8> = frame.components.iter().map(|c| c.id).collect();
        if ids == b"RGB" {
            ColourSpace::Rgb
        } else {
            ColourSpace::YCbCr
        }
    }
}

#[derive(Clone, Copy)]
enum ColourSpace {
    YCbCr,
    Rgb,
}

fn unsupported(kind: impl Into<String>) -> JpegError {
    JpegError::Unsupported(kind.into())
}

struct ScanMember {
    index: usize,
    dc: Huffman,
    ac: Huffman,
    quant: [u16; 64],
    prediction: i32,
}

/// `next_marker`: skip anything that is not 0xFF (garbage libjpeg counts and discards), swallow
/// 0xFF fill bytes, and step over a stuffed 0xFF00. Returns the marker code and the position after
/// it, or `None` at the end of the data.
fn next_marker(data: &[u8], mut pos: usize) -> Option<(u8, usize)> {
    loop {
        while *data.get(pos)? != 0xFF {
            pos += 1;
        }
        while *data.get(pos)? == 0xFF {
            pos += 1;
        }
        let code = data[pos];
        pos += 1;
        if code != 0 {
            return Some((code, pos));
        }
    }
}

/// A marker segment's payload: the two length bytes at `pos` count themselves.
fn segment(data: &[u8], pos: usize, marker: u8) -> Result<&[u8], JpegError> {
    let length = data
        .get(pos..pos + 2)
        .map(|b| usize::from(u16::from_be_bytes([b[0], b[1]])))
        .ok_or_else(|| {
            corrupt(format!(
                "the file ends inside marker 0xFF{marker:02X} (it is cut short?)"
            ))
        })?;
    if length < 2 {
        return Err(corrupt(format!(
            "marker 0xFF{marker:02X} with a length under 2"
        )));
    }
    data.get(pos + 2..pos + length).ok_or_else(|| {
        corrupt(format!(
            "the file ends inside marker 0xFF{marker:02X} (it is cut short?)"
        ))
    })
}

/// The entropy-coded bits of one scan, `jdhuff.c`'s `fill_bit_buffer`: 0xFF00 is a data 0xFF
/// (with any 0xFF fill before the 00 swallowed, as libjpeg accepts), and any other 0xFF pair is
/// the marker that ends the data. Past it the reader yields zero bits for a peek, as libjpeg does,
/// but **consuming** one is an error: valid data never needs a bit it does not have.
struct BitReader<'a> {
    data: &'a [u8],
    pos: usize,
    /// The next bits, most significant first; bits past `count` are zero.
    acc: u64,
    count: u32,
    /// Where the marker (or the end of the data) that stopped the reader begins.
    stopped_at: Option<usize>,
}

impl<'a> BitReader<'a> {
    fn new(data: &'a [u8], pos: usize) -> Self {
        BitReader {
            data,
            pos,
            acc: 0,
            count: 0,
            stopped_at: None,
        }
    }

    fn refill(&mut self) {
        while self.count <= 56 && self.stopped_at.is_none() {
            let Some(&byte) = self.data.get(self.pos) else {
                self.stopped_at = Some(self.pos);
                return;
            };
            let value = if byte == 0xFF {
                let mut next = self.pos + 1;
                while self.data.get(next) == Some(&0xFF) {
                    next += 1;
                }
                if self.data.get(next) != Some(&0) {
                    self.stopped_at = Some(self.pos);
                    return;
                }
                self.pos = next + 1;
                0xFF
            } else {
                self.pos += 1;
                byte
            };
            self.acc |= u64::from(value) << (56 - self.count);
            self.count += 8;
        }
    }

    /// The next `n` bits (1 to 16) without consuming them, zero-filled past the data's end.
    fn peek(&mut self, n: u32) -> u32 {
        if self.count < n {
            self.refill();
        }
        (self.acc >> (64 - n)) as u32
    }

    fn consume(&mut self, n: u32) -> Result<(), JpegError> {
        if n > self.count {
            return Err(corrupt(
                "the entropy-coded data ends before the last block (the file is cut short?)",
            ));
        }
        self.acc <<= n;
        self.count -= n;
        Ok(())
    }

    fn bits(&mut self, n: u32) -> Result<u32, JpegError> {
        if n == 0 {
            return Ok(0);
        }
        let value = self.peek(n);
        self.consume(n)?;
        Ok(value)
    }

    /// `process_restart`: drop the bits left in the interval, then the next marker must be
    /// `RST<expected>`. libjpeg resynchronises on a wrong one; this refuses.
    fn restart(&mut self, expected: u8) -> Result<(), JpegError> {
        self.acc = 0;
        self.count = 0;
        let from = self.stopped_at.take().unwrap_or(self.pos);
        match next_marker(self.data, from) {
            Some((marker, after)) if marker == 0xD0 + expected => {
                self.pos = after;
                Ok(())
            }
            found => Err(corrupt(format!(
                "restart marker {expected} expected, found {}",
                found.map_or("the end of the file".into(), |(m, _)| format!(
                    "0xFF{m:02X}"
                ))
            ))),
        }
    }

    /// Where the main marker loop resumes after the scan.
    fn end(&self) -> usize {
        self.stopped_at.unwrap_or(self.pos)
    }
}

/// How many bits the fast Huffman lookup resolves at once. libjpeg-turbo's `HUFF_LOOKAHEAD` is 8;
/// the width is a speed choice and cannot change a decoded symbol.
const LOOKAHEAD: u32 = 9;

/// A derived Huffman table, `jpeg_make_d_derived_tbl`'s `d_derived_tbl`.
struct Huffman {
    /// For each `LOOKAHEAD`-bit prefix: `length << 8 | symbol`, or 0 when no code that short
    /// matches it.
    lookup: Vec<u16>,
    /// `maxcode[l]`: the largest code of length `l`, or -1 if there is none.
    maxcode: [i32; 17],
    /// `valoffset[l]`: the index into `values` of the first code of length `l`, less that code.
    valoffset: [i32; 17],
    values: Vec<u8>,
}

impl Huffman {
    /// Figures C.1, C.2 and F.15, with libjpeg's two refusals: a code that no longer fits its
    /// length (an over-full table), and a DC symbol above 15 (`JERR_BAD_HUFF_TABLE` both).
    fn derive(spec: &HuffmanSpec, is_dc: bool) -> Result<Self, JpegError> {
        let bad = || corrupt("a malformed Huffman table");
        let mut sizes = Vec::with_capacity(spec.values.len());
        for (length, &n) in spec.bits.iter().enumerate().skip(1) {
            sizes.extend(std::iter::repeat_n(length as u32, usize::from(n)));
        }
        let mut codes = Vec::with_capacity(sizes.len());
        let mut code = 0u32;
        let mut length = sizes.first().copied().unwrap_or(0);
        let mut p = 0;
        while p < sizes.len() {
            while p < sizes.len() && sizes[p] == length {
                codes.push(code);
                code += 1;
                p += 1;
            }
            if code >= 1 << length {
                return Err(bad());
            }
            code <<= 1;
            length += 1;
        }
        if is_dc && spec.values.iter().any(|&symbol| symbol > 15) {
            return Err(bad());
        }
        let mut maxcode = [-1i32; 17];
        let mut valoffset = [0i32; 17];
        let mut p = 0usize;
        for l in 1..=16 {
            let n = usize::from(spec.bits[l]);
            if n > 0 {
                valoffset[l] = p as i32 - codes[p] as i32;
                p += n;
                maxcode[l] = codes[p - 1] as i32;
            }
        }
        let mut lookup = vec![0u16; 1 << LOOKAHEAD];
        for ((&size, &code), &symbol) in sizes.iter().zip(&codes).zip(&spec.values) {
            if size <= LOOKAHEAD {
                let shift = LOOKAHEAD - size;
                let first = (code << shift) as usize;
                let entry = ((size as u16) << 8) | u16::from(symbol);
                lookup[first..first + (1 << shift)].fill(entry);
            }
        }
        Ok(Huffman {
            lookup,
            maxcode,
            valoffset,
            values: spec.values.clone(),
        })
    }

    /// One symbol: the lookup for short codes, then `jpeg_huff_decode`'s walk up the lengths.
    fn decode(&self, bits: &mut BitReader) -> Result<u8, JpegError> {
        let entry = self.lookup[bits.peek(LOOKAHEAD) as usize];
        if entry != 0 {
            bits.consume(u32::from(entry >> 8))?;
            return Ok(entry as u8);
        }
        let window = bits.peek(16);
        for length in LOOKAHEAD + 1..=16 {
            let code = (window >> (16 - length)) as i32;
            if code <= self.maxcode[length as usize] {
                bits.consume(length)?;
                let index = code + self.valoffset[length as usize];
                return usize::try_from(index)
                    .ok()
                    .and_then(|i| self.values.get(i).copied())
                    .ok_or_else(|| corrupt("a Huffman code outside its table"));
            }
        }
        Err(corrupt("a Huffman code no table holds"))
    }
}

/// `HUFF_EXTEND`: the `s`-bit magnitude category's value, negative when its top bit is clear.
fn extend(value: u32, s: u32) -> i32 {
    if value < 1 << (s - 1) {
        value as i32 - (1 << s) + 1
    } else {
        value as i32
    }
}

/// `decode_mcu`'s body for one block: the DC difference against the component's prediction, then
/// the AC run-lengths in zigzag order, into `coefficients` in natural order.
fn decode_block(
    bits: &mut BitReader,
    dc: &Huffman,
    ac: &Huffman,
    prediction: &mut i32,
    coefficients: &mut [i16; 64],
) -> Result<(), JpegError> {
    *coefficients = [0; 64];
    let s = u32::from(dc.decode(bits)?);
    let difference = if s == 0 { 0 } else { extend(bits.bits(s)?, s) };
    *prediction = prediction.wrapping_add(difference);
    coefficients[0] = *prediction as i16;
    let mut k = 1;
    while k < 64 {
        let symbol = ac.decode(bits)?;
        let (run, s) = (usize::from(symbol >> 4), u32::from(symbol & 15));
        if s != 0 {
            k += run;
            coefficients[NATURAL_ORDER[k]] = extend(bits.bits(s)?, s) as i16;
        } else if run == 15 {
            k += 15;
        } else {
            break;
        }
        k += 1;
    }
    Ok(())
}

/// `jidctint.c`'s constants, `FIX(x)` at `CONST_BITS = 13`, as the file spells them out.
const CONST_BITS: u32 = 13;
const PASS1_BITS: u32 = 2;
const FIX_0_298631336: i64 = 2446;
const FIX_0_390180644: i64 = 3196;
const FIX_0_541196100: i64 = 4433;
const FIX_0_765366865: i64 = 6270;
const FIX_0_899976223: i64 = 7373;
const FIX_1_175875602: i64 = 9633;
const FIX_1_501321110: i64 = 12299;
const FIX_1_847759065: i64 = 15137;
const FIX_1_961570560: i64 = 16069;
const FIX_2_053119869: i64 = 16819;
const FIX_2_562915447: i64 = 20995;
const FIX_3_072711026: i64 = 25172;

/// `DESCALE`: shift right by `n`, rounding half up.
fn descale(x: i64, n: u32) -> i64 {
    (x + (1 << (n - 1))) >> n
}

/// The post-IDCT half of `prepare_range_limit_table`, indexed by the IDCT's output `& 1023`
/// (`RANGE_MASK`): -128..127 becomes 0..255, 128..511 saturates to 255 and -512..-129 to 0, and
/// anything wilder wraps, as a damaged block's does in libjpeg.
const IDCT_RANGE: [u8; 1024] = {
    let mut table = [0u8; 1024];
    let mut i = 0;
    while i < 1024 {
        table[i] = match i {
            0..=127 => (i + 128) as u8,
            128..=511 => 255,
            512..=895 => 0,
            _ => (i - 896) as u8,
        };
        i += 1;
    }
    table
};

fn idct_clamp(x: i64) -> u8 {
    IDCT_RANGE[(x & 1023) as usize]
}

/// The odd part of the 8-point inverse DCT (figure 8 of the LL&M paper `jidctint.c` follows),
/// shared by both passes: inputs are y1, y3, y5, y7, outputs `tmp0..tmp3` as the C names them.
fn odd_part(y1: i64, y3: i64, y5: i64, y7: i64) -> [i64; 4] {
    let (tmp0, tmp1, tmp2, tmp3) = (y7, y5, y3, y1);
    let z1 = tmp0 + tmp3;
    let z2 = tmp1 + tmp2;
    let z3 = tmp0 + tmp2;
    let z4 = tmp1 + tmp3;
    let z5 = (z3 + z4) * FIX_1_175875602;
    let tmp0 = tmp0 * FIX_0_298631336;
    let tmp1 = tmp1 * FIX_2_053119869;
    let tmp2 = tmp2 * FIX_3_072711026;
    let tmp3 = tmp3 * FIX_1_501321110;
    let z1 = z1 * -FIX_0_899976223;
    let z2 = z2 * -FIX_2_562915447;
    let z3 = z3 * -FIX_1_961570560 + z5;
    let z4 = z4 * -FIX_0_390180644 + z5;
    [
        tmp0 + z1 + z3,
        tmp1 + z2 + z4,
        tmp2 + z2 + z3,
        tmp3 + z1 + z4,
    ]
}

/// The even part: inputs y0, y2, y4, y6, outputs `tmp10..tmp13`; y0 and y4 are scaled up by
/// `CONST_BITS` here, to meet the rotator's products.
fn even_part(y0: i64, y2: i64, y4: i64, y6: i64) -> [i64; 4] {
    let z1 = (y2 + y6) * FIX_0_541196100;
    let tmp2 = z1 + y6 * -FIX_1_847759065;
    let tmp3 = z1 + y2 * FIX_0_765366865;
    let tmp0 = (y0 + y4) << CONST_BITS;
    let tmp1 = (y0 - y4) << CONST_BITS;
    [tmp0 + tmp3, tmp1 + tmp2, tmp1 - tmp2, tmp0 - tmp3]
}

/// `jpeg_idct_islow`: dequantise, columns into a workspace scaled by `2^PASS1_BITS`, then rows
/// out through the range limit, with both of libjpeg's all-AC-zero shortcuts. The shortcuts give
/// the same numbers the full pass would; they are kept because they are what the C does, so a
/// reader can lay the two side by side.
fn idct_islow(coefficients: &[i16; 64], quant: &[u16; 64], out: &mut [u8], stride: usize) {
    let mut workspace = [0i32; 64];
    for col in 0..8 {
        let input =
            |row: usize| i64::from(coefficients[row * 8 + col]) * i64::from(quant[row * 8 + col]);
        if (1..8).all(|row| coefficients[row * 8 + col] == 0) {
            let dc = (input(0) << PASS1_BITS) as i32;
            for row in 0..8 {
                workspace[row * 8 + col] = dc;
            }
            continue;
        }
        let [tmp10, tmp11, tmp12, tmp13] = even_part(input(0), input(2), input(4), input(6));
        let [tmp0, tmp1, tmp2, tmp3] = odd_part(input(1), input(3), input(5), input(7));
        let shift = CONST_BITS - PASS1_BITS;
        let outputs = [
            tmp10 + tmp3,
            tmp11 + tmp2,
            tmp12 + tmp1,
            tmp13 + tmp0,
            tmp13 - tmp0,
            tmp12 - tmp1,
            tmp11 - tmp2,
            tmp10 - tmp3,
        ];
        for (row, value) in outputs.into_iter().enumerate() {
            workspace[row * 8 + col] = descale(value, shift) as i32;
        }
    }
    for row in 0..8 {
        let w = &workspace[row * 8..row * 8 + 8];
        let line = &mut out[row * stride..row * stride + 8];
        if w[1..].iter().all(|&v| v == 0) {
            line.fill(idct_clamp(descale(i64::from(w[0]), PASS1_BITS + 3)));
            continue;
        }
        let at = |i: usize| i64::from(w[i]);
        let [tmp10, tmp11, tmp12, tmp13] = even_part(at(0), at(2), at(4), at(6));
        let [tmp0, tmp1, tmp2, tmp3] = odd_part(at(1), at(3), at(5), at(7));
        let shift = CONST_BITS + PASS1_BITS + 3;
        let outputs = [
            tmp10 + tmp3,
            tmp11 + tmp2,
            tmp12 + tmp1,
            tmp13 + tmp0,
            tmp13 - tmp0,
            tmp12 - tmp1,
            tmp11 - tmp2,
            tmp10 - tmp3,
        ];
        for (pixel, value) in line.iter_mut().zip(outputs) {
            *pixel = idct_clamp(descale(value, shift));
        }
    }
}

/// One component's samples at the photo's resolution: `stride` per row, at least as many rows
/// and columns as the photo has.
struct Plane<'c> {
    samples: std::borrow::Cow<'c, [u8]>,
    stride: usize,
}

/// `jinit_upsampler`'s choice, then the method it chose. The fancy filters read the component's
/// real samples only, replicating the first and last row and column past its edge
/// (`jdmainct.c`'s context rows, and each filter's own first- and last-column cases), never the
/// padding blocks the MCU grid decoded beyond them.
fn upsample(component: &Component, max_h: usize, max_v: usize) -> Result<Plane<'_>, JpegError> {
    let (h, v) = (component.h, component.v);
    if h == max_h && v == max_v {
        return Ok(Plane {
            samples: std::borrow::Cow::Borrowed(&component.samples),
            stride: component.stride,
        });
    }
    if max_h % h != 0 || max_v % v != 0 {
        // libjpeg's `JERR_FRACT_SAMPLE_NOTIMPL`: `cv2.imread` reads nothing either.
        return Err(unsupported("fractional-sampling"));
    }
    let (hx, vx) = (max_h / h, max_v / v);
    let (dw, dh) = (component.downsampled_width, component.downsampled_height);
    let row = |r: usize| &component.samples[r * component.stride..][..dw];
    let (stride, rows) = (dw * hx, dh * vx);
    let mut out = vec![0u8; stride * rows];
    match (hx, vx) {
        // `h2v1_fancy_upsample`: 3/4 of the nearer sample and 1/4 of the further, rounding the
        // two outputs of a pair in opposite directions (+1 then +2) so neither side is favoured.
        (2, 1) if dw > 2 => {
            for r in 0..dh {
                let (input, line) = (row(r), &mut out[r * stride..][..stride]);
                let at = |i: usize| i32::from(input[i]);
                line[0] = input[0];
                line[1] = ((at(0) * 3 + at(1) + 2) >> 2) as u8;
                for i in 1..dw - 1 {
                    let near = at(i) * 3;
                    line[2 * i] = ((near + at(i - 1) + 1) >> 2) as u8;
                    line[2 * i + 1] = ((near + at(i + 1) + 2) >> 2) as u8;
                }
                line[2 * dw - 2] = ((at(dw - 1) * 3 + at(dw - 2) + 1) >> 2) as u8;
                line[2 * dw - 1] = input[dw - 1];
            }
        }
        // `h1v2_fancy_upsample`: the same weights down a column, bias 1 above and 2 below.
        (1, 2) => {
            for r in 0..dh {
                for (half, far_row, bias) in
                    [(0, r.saturating_sub(1), 1), (1, (r + 1).min(dh - 1), 2)]
                {
                    let (near, far) = (row(r), row(far_row));
                    let line = &mut out[(2 * r + half) * stride..][..stride];
                    for ((pixel, &n), &f) in line.iter_mut().zip(near).zip(far) {
                        *pixel = ((i32::from(n) * 3 + i32::from(f) + bias) >> 2) as u8;
                    }
                }
            }
        }
        // `h2v2_fancy_upsample`: column sums of 3/4 nearer row and 1/4 further, then the h2v1
        // weights across them: 9/16, 3/16, 3/16, 1/16, rounded +8 and +7 in turn.
        (2, 2) if dw > 2 => {
            let mut sums = vec![0i32; dw];
            for r in 0..dh {
                for (half, far_row) in [(0, r.saturating_sub(1)), (1, (r + 1).min(dh - 1))] {
                    let (near, far) = (row(r), row(far_row));
                    for ((sum, &n), &f) in sums.iter_mut().zip(near).zip(far) {
                        *sum = i32::from(n) * 3 + i32::from(f);
                    }
                    let line = &mut out[(2 * r + half) * stride..][..stride];
                    line[0] = ((sums[0] * 4 + 8) >> 4) as u8;
                    line[1] = ((sums[0] * 3 + sums[1] + 7) >> 4) as u8;
                    for i in 1..dw - 1 {
                        line[2 * i] = ((sums[i] * 3 + sums[i - 1] + 8) >> 4) as u8;
                        line[2 * i + 1] = ((sums[i] * 3 + sums[i + 1] + 7) >> 4) as u8;
                    }
                    line[2 * dw - 2] = ((sums[dw - 1] * 3 + sums[dw - 2] + 8) >> 4) as u8;
                    line[2 * dw - 1] = ((sums[dw - 1] * 4 + 7) >> 4) as u8;
                }
            }
        }
        // `h2v1_upsample`, `h2v2_upsample` (a chroma row too narrow for the fancy filters,
        // `downsampled_width <= 2`) and `int_upsample` (every other integral ratio, 4:1:1's
        // included): each sample repeated `hx` times across and `vx` times down.
        _ => {
            for r in 0..dh {
                let input = row(r);
                for half in 0..vx {
                    let line = &mut out[(r * vx + half) * stride..][..stride];
                    for (chunk, &sample) in line.chunks_exact_mut(hx).zip(input) {
                        chunk.fill(sample);
                    }
                }
            }
        }
    }
    Ok(Plane {
        samples: std::borrow::Cow::Owned(out),
        stride,
    })
}

/// `build_ycc_rgb_table` at `SCALEBITS = 16`: R and B from rounded tables, G from two scaled
/// tables summed and then shifted, with the rounding half folded into the Cb one.
struct YccTables {
    cr_r: [i32; 256],
    cb_b: [i32; 256],
    cr_g: [i32; 256],
    cb_g: [i32; 256],
}

impl YccTables {
    fn new() -> Self {
        const SCALEBITS: u32 = 16;
        const ONE_HALF: i32 = 1 << (SCALEBITS - 1);
        // `FIX(x)`: `(JLONG)((x) * (1L << SCALEBITS) + 0.5)`, computed in double as the C does.
        let fix = |x: f64| (x * f64::from(1u32 << SCALEBITS) + 0.5) as i32;
        let mut tables = YccTables {
            cr_r: [0; 256],
            cb_b: [0; 256],
            cr_g: [0; 256],
            cb_g: [0; 256],
        };
        for i in 0..256 {
            let x = i as i32 - 128;
            tables.cr_r[i] = (fix(1.40200) * x + ONE_HALF) >> SCALEBITS;
            tables.cb_b[i] = (fix(1.77200) * x + ONE_HALF) >> SCALEBITS;
            tables.cr_g[i] = -fix(0.71414) * x;
            tables.cb_g[i] = -fix(0.34414) * x + ONE_HALF;
        }
        tables
    }

    /// `ycc_rgb_convert` for one pixel, through `sample_range_limit` (a clamp to 0..255).
    fn rgb(&self, y: u8, cb: u8, cr: u8) -> [u8; 3] {
        let (y, cb, cr) = (i32::from(y), usize::from(cb), usize::from(cr));
        let clamp = |v: i32| v.clamp(0, 255) as u8;
        [
            clamp(y + self.cr_r[cr]),
            clamp(y + ((self.cb_g[cb] + self.cr_g[cr]) >> 16)),
            clamp(y + self.cb_b[cb]),
        ]
    }
}

/// The orientation tag from an APP1 payload, as OpenCV's `ExifReader` reads it: the TIFF header
/// six bytes in (OpenCV skips `Exif\0\0` without checking it), `II` for little-endian and anything
/// else big-endian, the 0x002A mark, then IFD0's entries in order, the first 0x0112 winning (a
/// `std::map` insert keeps the first). The value is the entry's first SHORT, whatever its type.
fn exif_orientation(app1: &[u8]) -> Option<u16> {
    let tiff = app1.get(6..)?;
    let little = tiff.starts_with(b"II");
    let u16_at = |at: usize| {
        let b = tiff.get(at..at + 2)?;
        Some(if little {
            u16::from_le_bytes([b[0], b[1]])
        } else {
            u16::from_be_bytes([b[0], b[1]])
        })
    };
    let u32_at = |at: usize| {
        let b = tiff.get(at..at + 4)?;
        Some(if little {
            u32::from_le_bytes([b[0], b[1], b[2], b[3]])
        } else {
            u32::from_be_bytes([b[0], b[1], b[2], b[3]])
        })
    };
    if u16_at(2)? != 0x002A {
        return None;
    }
    let ifd = usize::try_from(u32_at(4)?).ok()?;
    let entries = usize::from(u16_at(ifd)?);
    (0..entries)
        .map(|i| ifd + 2 + 12 * i)
        .map_while(|entry| Some((entry, u16_at(entry)?)))
        .find(|&(_, tag)| tag == 0x0112)
        .and_then(|(entry, _)| u16_at(entry + 8))
}

/// `ExifTransform`: orientations 2 to 8 as flips and transposes; 1, and anything that is not an
/// orientation, leave the photo as stored. `image`'s `Orientation` is the same eight EXIF cases.
fn apply_orientation(image: RgbImage, orientation: Option<u16>) -> RgbImage {
    let Some(orientation) = orientation
        .and_then(|o| u8::try_from(o).ok())
        .and_then(Orientation::from_exif)
    else {
        return image;
    };
    let mut image = DynamicImage::ImageRgb8(image);
    image.apply_orientation(orientation);
    image.into_rgb8()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_dc_only_block_is_flat_at_its_mean_plus_128() {
        let mut coefficients = [0i16; 64];
        let quant = [1u16; 64];
        let mut out = vec![0u8; 64];
        // DC is 8 times the block's mean, centred on zero: 8 * (200 - 128).
        coefficients[0] = 8 * 72;
        idct_islow(&coefficients, &quant, &mut out, 8);
        assert!(out.iter().all(|&p| p == 200), "{out:?}");
        // The range limit saturates instead of wrapping inside -512..511.
        coefficients[0] = 8 * 300;
        idct_islow(&coefficients, &quant, &mut out, 8);
        assert!(out.iter().all(|&p| p == 255));
        coefficients[0] = -8 * 300;
        idct_islow(&coefficients, &quant, &mut out, 8);
        assert!(out.iter().all(|&p| p == 0));
    }

    #[test]
    fn islow_is_within_one_level_of_the_exact_inverse_dct() {
        // IEEE 1180's accuracy, roughly: the fixed-point IDCT may round a level away from the
        // real-valued one, never more. Blocks of a deterministic LCG's coefficients, a mix of the
        // shortcut's columns and rows and the full passes'. Exactness against libjpeg-turbo is
        // `tests/jpeg.rs`'s job; this says the port is an inverse DCT at all.
        let mut seed = 29u32;
        let mut next = |span: i32| {
            seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
            (seed >> 16) as i32 % (2 * span + 1) - span
        };
        let quant = [1u16; 64];
        for _ in 0..300 {
            let mut coefficients = [0i16; 64];
            coefficients[0] = next(600) as i16;
            for _ in 0..6 {
                let at = (next(15) + 15) as usize % 24;
                coefficients[NATURAL_ORDER[at + 1]] = next(60) as i16;
            }
            let mut out = vec![0u8; 64];
            idct_islow(&coefficients, &quant, &mut out, 8);
            for (i, &pixel) in out.iter().enumerate() {
                let (y, x) = ((i / 8) as f64, (i % 8) as f64);
                let mut sum = 0.0;
                for (k, &c) in coefficients.iter().enumerate() {
                    let (v, u) = ((k / 8) as f64, (k % 8) as f64);
                    let cu = if u == 0.0 { 1.0 / 2f64.sqrt() } else { 1.0 };
                    let cv = if v == 0.0 { 1.0 / 2f64.sqrt() } else { 1.0 };
                    sum += cu
                        * cv
                        * f64::from(c)
                        * ((2.0 * x + 1.0) * u * std::f64::consts::PI / 16.0).cos()
                        * ((2.0 * y + 1.0) * v * std::f64::consts::PI / 16.0).cos();
                }
                let exact = (sum / 4.0 + 128.0).clamp(0.0, 255.0);
                assert!(
                    (f64::from(pixel) - exact).abs() <= 1.0,
                    "{coefficients:?}: pixel {i} is {pixel}, exactly {exact}"
                );
            }
        }
    }

    #[test]
    fn the_range_table_is_libjpegs() {
        assert_eq!(idct_clamp(-128), 0);
        assert_eq!(idct_clamp(0), 128);
        assert_eq!(idct_clamp(127), 255);
        assert_eq!(idct_clamp(511), 255);
        assert_eq!(idct_clamp(-512), 0);
        // Past -512..511 the mask wraps, as a damaged block's value does in libjpeg.
        assert_eq!(idct_clamp(512), 0);
        assert_eq!(idct_clamp(1024), 128);
    }

    #[test]
    fn extend_reads_a_clear_top_bit_as_negative() {
        assert_eq!(extend(0b0, 1), -1);
        assert_eq!(extend(0b1, 1), 1);
        assert_eq!(extend(0b011, 3), -4);
        assert_eq!(extend(0b100, 3), 4);
        assert_eq!(extend(0x7FFF, 16), -32768);
    }

    #[test]
    fn the_ycc_tables_are_libjpegs() {
        let t = YccTables::new();
        // FIX(1.402) = 91881, FIX(1.772) = 116130, FIX(0.71414) = 46802, FIX(0.34414) = 22554.
        assert_eq!(t.cr_r[255], (91881 * 127 + 32768) >> 16);
        assert_eq!(t.cb_b[0], (116130 * -128 + 32768) >> 16);
        assert_eq!(t.cr_g[0], 46802 * 128);
        assert_eq!(t.cb_g[128], 32768);
        assert_eq!(t.rgb(128, 128, 128), [128, 128, 128]);
        assert_eq!(t.rgb(255, 0, 255), [255, 208, 28]);
    }

    fn spec(bits: &[(usize, u8)], values: &[u8]) -> HuffmanSpec {
        let mut counts = [0u8; 17];
        for &(length, n) in bits {
            counts[length] = n;
        }
        HuffmanSpec {
            bits: counts,
            values: values.to_vec(),
        }
    }

    #[test]
    fn huffman_codes_short_and_long_decode_and_a_full_table_is_refused() {
        // One 2-bit code (00), one 3-bit (010), and one 12-bit code (0110 0000 0000), which only
        // the walk past the lookup can resolve.
        let table = Huffman::derive(&spec(&[(2, 1), (3, 1), (12, 1)], &[5, 7, 9]), true).unwrap();
        let data = [0b0001_0011, 0b0000_0000, 0b0000_0000];
        let mut bits = BitReader::new(&data, 0);
        assert_eq!(table.decode(&mut bits).unwrap(), 5);
        assert_eq!(table.decode(&mut bits).unwrap(), 7);
        assert_eq!(table.decode(&mut bits).unwrap(), 9);
        // Two 1-bit codes fill the code space: libjpeg's `JERR_BAD_HUFF_TABLE`.
        assert!(Huffman::derive(&spec(&[(1, 2)], &[0, 1]), false).is_err());
        // A DC symbol above 15 is refused; the same symbol in an AC table is not.
        assert!(Huffman::derive(&spec(&[(2, 1)], &[16]), true).is_err());
        assert!(Huffman::derive(&spec(&[(2, 1)], &[16]), false).is_ok());
    }

    #[test]
    fn stuffed_bytes_are_data_and_a_marker_ends_the_bits() {
        // 0xFF00 is one 0xFF data byte (even after 0xFF fill); 0xFFD9 stops the reader there.
        let data = [0xFF, 0xFF, 0x00, 0xA5, 0xFF, 0xD9];
        let mut bits = BitReader::new(&data, 0);
        assert_eq!(bits.bits(8).unwrap(), 0xFF);
        assert_eq!(bits.bits(8).unwrap(), 0xA5);
        // A peek past the marker sees zeros; consuming them is the truncation libjpeg pads over.
        assert_eq!(bits.peek(4), 0);
        assert!(matches!(bits.consume(1), Err(JpegError::Corrupt(_))));
        assert_eq!(bits.end(), 4);
    }

    #[test]
    fn the_orientation_is_read_in_either_byte_order_from_ifd0() {
        let mut little = b"Exif\0\0II*\0\x08\0\0\0\x02\0".to_vec();
        little.extend_from_slice(&[0x0F, 0x01, 2, 0, 4, 0, 0, 0, 0, 0, 0, 0]);
        little.extend_from_slice(&[0x12, 0x01, 3, 0, 1, 0, 0, 0, 6, 0, 0, 0]);
        assert_eq!(exif_orientation(&little), Some(6));
        let mut big = b"Exif\0\0MM\0*\0\0\0\x08\0\x01".to_vec();
        big.extend_from_slice(&[0x01, 0x12, 0, 3, 0, 0, 0, 1, 0, 8, 0, 0]);
        assert_eq!(exif_orientation(&big), Some(8));
        // No TIFF mark, or an IFD that runs off the end: no orientation, and no panic.
        assert_eq!(exif_orientation(b"Exif\0\0II\0\0"), None);
        assert_eq!(exif_orientation(b"Exif\0\0II*\0\xFF\0\0\0"), None);
    }
}
