//! The lab's OCR reader: a photo of the launch-monitor screen in, located text out. [M29 P1]
//!
//! The Rust half of what frozen Python does with OpenCV and PaddleOCR before its parser starts:
//! `launch_monitor/screen/preprocess.py` finds and rectifies the screen, and
//! `launch_monitor/screen/paddle.py` hands the result to PaddleOCR, whose boxes become the
//! `TextBox`es `crates/screen` reads a shot from. ADR-035 §2 ports it to Rust through `ort`,
//! running **the same Paddle models**, so the reader can be held to the boxes PaddleOCR already
//! committed for the 13 stored bay photos (`spec/vectors/screen/corpus/`, the M29 plan's D7). M38's
//! phone does not use any of this: it has VisionKit, and starts at `crates/screen`.
//!
//! # Why it is a crate of its own
//!
//! **It is the only crate with `ort`**, and keeping it that way is the reason the crate exists
//! (the M29 plan's D13). ONNX Runtime is a large native library with a build that downloads it,
//! and nothing on the phone's path may pull it in, so the `ort` edge lives here and nowhere else.
//! `tests/ort_reach.rs` walks every crate's manifest and fails if `contracts`, `pyfmt`,
//! `analysis`, `feedback`, `screen`, `storage` or `core` can reach it; `crates/lab` (P20) reaches
//! it only through this crate. It depends on `screen` for the `TextBox` seam (P6), and never on
//! `analysis`, `feedback` or `core` (ADR-008 as a cargo edge); each edge is declared in the phase
//! that first crosses it, as `crates/screen`'s were.
//!
//! # The order it is built in
//!
//! The M29 plan's Part A, one stage per phase so each is checked before the next leans on it:
//!
//! - [`models`] — the three ONNX models, the committed manifest pinning them
//!   (`crates/ocr/models.json`), and the sessions opened from them, refusing a missing file or a
//!   digest that disagrees (P1);
//! - [`decode`] and [`quad`] — photo decode (JPEG through a hand port of libjpeg-turbo's decode,
//!   PNG through `image`, HEIC through ffmpeg) and the screen-quad finder, over [`cv`]'s hand ports
//!   of the OpenCV calls frozen Python makes (P2);
//! - [`rectify`] — the screen warped front-on, shrunk to 1600 px and its contrast lifted, the image
//!   the recognizer sees, over [`cv`]'s ports of `warpPerspective`, CLAHE and the L\*a\*b\*
//!   conversions (P3);
//! - [`det`] — text detection: the working image resized and normalised for the `det` model, and
//!   its probability map turned into boxes by DB postprocessing, over [`cv`]'s bilinear resize,
//!   `RETR_LIST` contours and `fillPoly`, and [`clipper`]'s port of the unclip pyclipper does (P4);
//! - [`rec`] — recognition: each box cut out front-on by a bicubic warp, read by the `rec` model in
//!   the pipeline's batches, and its output decoded by CTC to a text and a score (P5);
//! - [`cls`] — textline orientation: each crop classified upright or upside down, and the
//!   upside-down ones turned as the pipeline turns them, which is not a flip (P6);
//! - [`recognizer`] — the `recognize` seam, all of the above in the pipeline's order, ending in
//!   `crates/screen`'s `TextBox`es: `paddle.py`'s `PaddleOCRRecognizer` without the document
//!   orientation and unwarping models it also runs (P6);
//! - [`prepare`] — the reader end to end: [`read_photo`] decodes a photo and [`prepare_screen`]
//!   finds, rectifies, orients and recognizes it, as `preprocess.py::prepare_screen` does, with its
//!   rotation vote and notes, ending at the boxes and notes [`screen::read`] reads a shot from;
//!   `golf-ocr read` is its command, and `tests/read.rs` the OCR gate on the 13 photos (P7).

pub mod clipper;
pub mod cls;
pub mod cv;
pub mod decode;
pub mod det;
pub mod models;
pub mod prepare;
pub mod quad;
pub mod rec;
pub mod recognizer;
pub mod rectify;

pub use models::{Manifest, Model, ModelError, Models, Tensor, DEFAULT_DIR};
pub use prepare::{prepare_screen, read_photo, screen_timestamp, PreparedScreen, ReadError};
pub use recognizer::recognize;
