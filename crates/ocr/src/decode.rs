//! A photo file in, the pixels frozen Python decodes out. [M29 P2]
//!
//! The twin of `launch_monitor/screen/preprocess.py`'s `load_image`, `_is_heif` and
//! `_load_via_pillow`. Python tries `cv2.imread` (which applies the EXIF orientation) and falls back
//! to Pillow with `pillow-heif` for what OpenCV has no codec for, which in this bay is HEIC, the
//! iPhone camera's default. Here the split is made up front by the file's own bytes instead of by
//! one decoder failing first: a HEIF brand in the `ftyp` box goes to ffmpeg (the M29 plan's D3), a
//! JPEG signature to `jpeg` (`decode/jpeg.rs`), a hand port of libjpeg-turbo's decode, and
//! anything else to the `image` crate, which is built with its PNG codec alone.
//!
//! # How close the pixels are (measured on the 13 bay photos, M29 P2)
//!
//! Sizes are identical on all 13, and so are the pixels of all but the HEIC:
//!
//! - **JPEG** (ten photos): identical, through `jpeg`. `image`'s own JPEG decode was tried first
//!   and was not close enough: `zune-jpeg`'s IDCT is not libjpeg-turbo's ISLOW, one level apart on
//!   about 0.5% of values, and that changed the screen quad on two of the ten (the plan's P2
//!   finding). `jpeg`'s doc says what it reads and what it refuses.
//! - **PNG** (two photos): identical, through `image`.
//! - **HEIC** (one photo): the HEVC tiles decode identically (the codec is normative), but the YCbCr-to-RGB
//!   step is ffmpeg's `swscale` here and libheif's in `pillow-heif`, and they round differently:
//!   at most 2 levels apart, mean 0.57, on the one bay HEIC. ffmpeg assembles the tile grid inside
//!   an automatic filter graph, which is why `-sws_flags` and `-vf` cannot reach that conversion
//!   (both were tried: the first changes nothing, the second is refused).
//!
//! Whether either moves a recognised character is the OCR gate's question (P7), not this module's.
//!
//! # What `cv2.imread` does that this does not
//!
//! Nothing the bay photos exercise. A 16-bit PNG narrows by rounding here (`image`) and by a shift
//! in OpenCV, and a grey or alpha PNG becomes RGB in both; every stored photo is 8-bit RGB. The
//! kinds of JPEG `jpeg` refuses by name (progressive, CMYK and rarer) `cv2` reads, and so do the
//! formats with no decoder here at all (BMP, TIFF, WebP); no photo in the bay arrives as either.

mod jpeg;

use std::env;
use std::ffi::OsString;
use std::fmt;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use image::{DynamicImage, ImageDecoder, ImageFormat, ImageReader, RgbImage};

/// The environment variable naming the ffmpeg binary, looked at before `PATH` (the M29 plan's
/// D17). The same name every later lab phase reads, so one setting serves the whole lab.
pub const FFMPEG_ENV: &str = "GOLF_FFMPEG";

/// Brand codes in an ISO-BMFF `ftyp` box that mean HEIF: `_HEIF_BRANDS`, verbatim. Frozen Python
/// uses the set only to word an error; here it routes the file to ffmpeg.
const HEIF_BRANDS: [&[u8; 4]; 8] = [
    b"heic", b"heix", b"hevc", b"hevx", b"heim", b"heis", b"mif1", b"msf1",
];

/// Why a photo could not be decoded. Each names the file, and the ffmpeg ones say what was tried,
/// because the repair differs: install ffmpeg, point `GOLF_FFMPEG` at it, or re-take the photo.
#[derive(Debug)]
pub enum DecodeError {
    /// The file could not be read at all.
    Unreadable { path: PathBuf, source: io::Error },
    /// A non-HEIF file no decoder here could read: a corrupt or truncated JPEG or PNG, or a format
    /// with no decoder here (only JPEG, PNG and HEIC have one).
    Undecodable { path: PathBuf, reason: String },
    /// A well-formed JPEG of a kind `jpeg` does not read, which `kind` names ("progressive",
    /// "four-component (CMYK or YCCK)", ...). Re-saving it as a baseline JPEG or a PNG repairs it.
    UnsupportedJpeg { path: PathBuf, kind: String },
    /// A HEIF photo, and no ffmpeg to decode it: every place looked, in order.
    FfmpegMissing { path: PathBuf, tried: Vec<String> },
    /// ffmpeg ran and did not produce an image.
    FfmpegFailed {
        path: PathBuf,
        ffmpeg: PathBuf,
        reason: String,
    },
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DecodeError::Unreadable { path, source } => {
                write!(f, "could not read image {}: {source}", path.display())
            }
            DecodeError::Undecodable { path, reason } => write!(
                f,
                "could not read image {}: no decoder for it here (JPEG and PNG in-process, \
                 HEIC through ffmpeg) could read this file: {reason}",
                path.display()
            ),
            DecodeError::UnsupportedJpeg { path, kind } => write!(
                f,
                "could not read image {}: it is a {kind} JPEG, and only baseline and extended \
                 sequential 8-bit JPEGs with one or three components are decoded here (so that \
                 the pixels match OpenCV's exactly). Re-save it as a baseline JPEG or a PNG",
                path.display()
            ),
            DecodeError::FfmpegMissing { path, tried } => write!(
                f,
                "could not read image {}: it is a HEIC/HEIF photo (the iPhone camera default), \
                 which is decoded by ffmpeg, and no ffmpeg was found (tried {}). Install ffmpeg \
                 or set {FFMPEG_ENV} to its path",
                path.display(),
                tried.join(", ")
            ),
            DecodeError::FfmpegFailed {
                path,
                ffmpeg,
                reason,
            } => write!(
                f,
                "could not read image {}: {} did not decode it: {reason}",
                path.display(),
                ffmpeg.display()
            ),
        }
    }
}

impl std::error::Error for DecodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            DecodeError::Unreadable { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// Decode a photo to 8-bit RGB, upright: `load_image`'s twin.
///
/// RGB, not the BGR every OpenCV call in frozen Python holds. The channel order is a convention of
/// the library, not of the algorithm, and the one place it reaches a number (the grey conversion's
/// weights) says which channel it means.
pub fn load_image(path: &Path) -> Result<RgbImage, DecodeError> {
    if is_heif(path) {
        return decode_with_ffmpeg(
            path,
            &find_ffmpeg().map_err(|tried| DecodeError::FfmpegMissing {
                path: path.to_path_buf(),
                tried,
            })?,
        );
    }
    let bytes = fs::read(path).map_err(|source| DecodeError::Unreadable {
        path: path.to_path_buf(),
        source,
    })?;
    if jpeg::is_jpeg(&bytes) {
        return jpeg::decode(&bytes).map_err(|error| match error {
            jpeg::JpegError::Unsupported(kind) => DecodeError::UnsupportedJpeg {
                path: path.to_path_buf(),
                kind,
            },
            jpeg::JpegError::Corrupt(reason) => DecodeError::Undecodable {
                path: path.to_path_buf(),
                reason,
            },
        });
    }
    decode_in_process(&bytes, None).map_err(|reason| DecodeError::Undecodable {
        path: path.to_path_buf(),
        reason,
    })
}

/// True if the file's `ftyp` box names a HEIF brand: `_is_heif`'s twin. Never fails; an unreadable
/// file is simply not HEIF, and the decoder that is tried next says what is wrong with it.
pub fn is_heif(path: &Path) -> bool {
    let mut header = [0u8; 16];
    let Ok(mut file) = fs::File::open(path) else {
        return false;
    };
    let mut filled = 0;
    while filled < header.len() {
        match file.read(&mut header[filled..]) {
            Ok(0) | Err(_) => break,
            Ok(n) => filled += n,
        }
    }
    is_heif_header(&header[..filled])
}

/// The brand check on the first bytes alone: a 4-byte box size, `ftyp`, then the major brand.
fn is_heif_header(header: &[u8]) -> bool {
    header.len() >= 12
        && &header[4..8] == b"ftyp"
        && HEIF_BRANDS.iter().any(|brand| &header[8..12] == *brand)
}

/// Where ffmpeg is: `GOLF_FFMPEG` if set, otherwise the first `ffmpeg` on `PATH` (D17). `Err` holds
/// every place looked, in order, for the refusal to name. `.venv`'s bundled ffmpeg is deliberately
/// not a fallback: it is 7.1, which decodes one 896x640 tile of the bay HEIC rather than the photo
/// (the plan's planning finding 5), and it leaves with the `audio` extra in M40.
pub fn find_ffmpeg() -> Result<PathBuf, Vec<String>> {
    find_ffmpeg_in(env::var_os(FFMPEG_ENV), env::var_os("PATH"))
}

fn find_ffmpeg_in(
    configured: Option<OsString>,
    path: Option<OsString>,
) -> Result<PathBuf, Vec<String>> {
    let mut tried = Vec::new();
    if let Some(configured) = configured.filter(|value| !value.is_empty()) {
        let candidate = PathBuf::from(configured);
        if candidate.is_file() {
            return Ok(candidate);
        }
        tried.push(format!("{FFMPEG_ENV}={}", candidate.display()));
    } else {
        tried.push(format!("{FFMPEG_ENV} (not set)"));
    }
    let names: &[&str] = if cfg!(windows) {
        &["ffmpeg.exe", "ffmpeg"]
    } else {
        &["ffmpeg"]
    };
    let mut searched = false;
    if let Some(path) = path {
        for dir in env::split_paths(&path) {
            searched = true;
            for name in names {
                let candidate = dir.join(name);
                if candidate.is_file() {
                    return Ok(candidate);
                }
            }
        }
    }
    tried.push(if searched {
        "every PATH directory".to_string()
    } else {
        "PATH (empty)".to_string()
    });
    Err(tried)
}

/// HEIF through ffmpeg: the primary image, as an RGB PNG on stdout, decoded in-process.
///
/// ffmpeg 7.1 and later assemble a HEIF tile grid into the whole photo, and the CLI applies the
/// container's rotation (`irot`/`imir`) by default, as `pillow-heif` does before Python's
/// `exif_transpose`. The EXIF orientation is not applied on top: `pillow-heif` neutralises it once
/// the container's transform is applied, so applying both would turn the photo twice.
fn decode_with_ffmpeg(path: &Path, ffmpeg: &Path) -> Result<RgbImage, DecodeError> {
    let failed = |reason: String| DecodeError::FfmpegFailed {
        path: path.to_path_buf(),
        ffmpeg: ffmpeg.to_path_buf(),
        reason,
    };
    let output = Command::new(ffmpeg)
        .args(["-v", "error", "-nostdin", "-i"])
        .arg(path)
        .args([
            "-frames:v",
            "1",
            "-f",
            "image2pipe",
            "-c:v",
            "png",
            "-pix_fmt",
            "rgb24",
            "-",
        ])
        .stdin(Stdio::null())
        .output()
        .map_err(|error| failed(format!("could not run it: {error}")))?;
    if !output.status.success() || output.stdout.is_empty() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(failed(format!(
            "exit {}, {}",
            output.status,
            stderr.trim().lines().last().unwrap_or("no message")
        )));
    }
    decode_in_process(&output.stdout, Some(ImageFormat::Png)).map_err(failed)
}

/// `image`'s decode with the EXIF orientation applied, as `cv2.imread` applies it: the PNGs, and
/// ffmpeg's PNG of a HEIF photo.
fn decode_in_process(bytes: &[u8], format: Option<ImageFormat>) -> Result<RgbImage, String> {
    let mut reader = ImageReader::new(io::Cursor::new(bytes));
    match format {
        Some(format) => reader.set_format(format),
        None => {
            reader = reader
                .with_guessed_format()
                .map_err(|error| error.to_string())?;
        }
    }
    let mut decoder = reader.into_decoder().map_err(|error| error.to_string())?;
    let orientation = decoder.orientation().map_err(|error| error.to_string())?;
    let mut image = DynamicImage::from_decoder(decoder).map_err(|error| error.to_string())?;
    image.apply_orientation(orientation);
    Ok(image.into_rgb8())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_brand_check_reads_the_major_brand_after_ftyp() {
        assert!(is_heif_header(b"\0\0\0,ftypheic\0\0\0\0"));
        assert!(is_heif_header(b"\0\0\0\x18ftypmif1"));
        // An MP4's brand, and a JPEG, are not HEIF.
        assert!(!is_heif_header(b"\0\0\0\x18ftypisom\0\0\0\0"));
        assert!(!is_heif_header(b"\xff\xd8\xff\xe0\0\x10JFIF\0\x01\x01\x01"));
        // Too short to hold a brand.
        assert!(!is_heif_header(b"\0\0\0,ftyphe"));
    }

    #[test]
    fn ffmpeg_is_looked_for_in_the_variable_then_on_path_and_every_miss_is_named() {
        let empty = tempfile::tempdir().unwrap();
        let missing = empty.path().join("no-ffmpeg-here");
        let tried = find_ffmpeg_in(
            Some(missing.clone().into_os_string()),
            Some(empty.path().as_os_str().to_owned()),
        )
        .unwrap_err();
        assert_eq!(
            tried,
            vec![
                format!("{FFMPEG_ENV}={}", missing.display()),
                "every PATH directory".to_string()
            ]
        );
        assert_eq!(
            find_ffmpeg_in(None, None).unwrap_err(),
            vec![
                format!("{FFMPEG_ENV} (not set)"),
                "PATH (empty)".to_string()
            ]
        );
    }

    #[test]
    fn the_variable_wins_over_path() {
        let first = tempfile::tempdir().unwrap();
        let second = tempfile::tempdir().unwrap();
        let name = if cfg!(windows) {
            "ffmpeg.exe"
        } else {
            "ffmpeg"
        };
        let configured = first.path().join("my-ffmpeg");
        fs::write(&configured, b"").unwrap();
        fs::write(second.path().join(name), b"").unwrap();
        let path = env::join_paths([second.path()]).unwrap();
        assert_eq!(
            find_ffmpeg_in(
                Some(configured.clone().into_os_string()),
                Some(path.clone())
            )
            .unwrap(),
            configured
        );
        assert_eq!(
            find_ffmpeg_in(None, Some(path)).unwrap(),
            second.path().join(name)
        );
    }

    #[test]
    fn a_missing_file_is_unreadable_and_a_corrupt_one_undecodable() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("absent.jpg");
        assert!(matches!(
            load_image(&missing),
            Err(DecodeError::Unreadable { .. })
        ));
        let corrupt = dir.path().join("corrupt.jpg");
        fs::write(&corrupt, b"\xff\xd8\xff\xe0 not really a jpeg").unwrap();
        let error = load_image(&corrupt).unwrap_err();
        assert!(matches!(error, DecodeError::Undecodable { .. }), "{error}");
        assert!(error.to_string().contains("corrupt.jpg"));
    }

    #[test]
    fn a_heif_photo_with_no_ffmpeg_is_refused_by_name() {
        let dir = tempfile::tempdir().unwrap();
        let photo = dir.path().join("shot.HEIC");
        fs::write(&photo, b"\0\0\0,ftypheic\0\0\0\0").unwrap();
        assert!(is_heif(&photo));
        let error = decode_with_ffmpeg(&photo, &dir.path().join("no-ffmpeg")).unwrap_err();
        assert!(matches!(error, DecodeError::FfmpegFailed { .. }), "{error}");
        let missing = DecodeError::FfmpegMissing {
            path: photo.clone(),
            tried: vec![
                "GOLF_FFMPEG (not set)".into(),
                "every PATH directory".into(),
            ],
        };
        let message = missing.to_string();
        assert!(
            message.contains("HEIC") && message.contains(FFMPEG_ENV),
            "{message}"
        );
    }

    #[test]
    fn a_png_round_trips_with_its_size_and_pixels() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tiny.png");
        let mut image = RgbImage::new(3, 2);
        image.put_pixel(2, 1, image::Rgb([10, 20, 30]));
        image.save(&path).unwrap();
        let decoded = load_image(&path).unwrap();
        assert_eq!(decoded.dimensions(), (3, 2));
        assert_eq!(decoded.get_pixel(2, 1).0, [10, 20, 30]);
    }
}
