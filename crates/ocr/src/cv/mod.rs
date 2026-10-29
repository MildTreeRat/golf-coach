//! Hand ports of the OpenCV calls frozen Python makes on a photo before OCR, and inside PaddleX's
//! text detector. [M29 P2, P3, P4]
//!
//! `launch_monitor/screen/preprocess.py` runs OpenCV 4.10 (the `vision` extra), and so does
//! PaddleX's detection postprocessing; the M29 plan's D4 rules out a native OpenCV here: each call
//! is written again from OpenCV's own source, one module per family, each naming the C++ it
//! follows. They are ports of C++, not of Python, so they are held to OpenCV's answers rather than
//! to a vector: the 13 bay photos' intermediates, dumped by frozen Python into a scratch
//! directory, compared stage by stage (the plan's P2 to P4 findings record how close each came).
//! The screen quad at the end of P2's stages is pinned by `tests/quad.rs`, the working image at
//! the end of P3's by `tests/rectify.rs`, and the detector's boxes by `tests/det.rs`, each with
//! OpenCV's answers on small committed inputs beside it.
//!
//! RGB throughout where OpenCV holds BGR; only the grey conversion's weights see the difference.

pub mod canny;
pub mod clahe;
pub mod contours;
pub mod draw;
pub mod filter;
pub mod lab;
pub mod morph;
pub mod rect;
pub mod resize;
pub mod warp;
