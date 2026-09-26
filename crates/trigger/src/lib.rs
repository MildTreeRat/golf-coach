//! Ball-strike detection.
//!
//! The first Rust in this repo, and the first module moved off Python under ADR-030. The decision
//! there gives Rust "the app process, session orchestration, camera capture, the ring buffer,
//! clip cutting, **audio strike detection**, phase segmentation, measurement, checkpoint scoring,
//! storage and the IPC layer"; `docs/CONFORMANCE.md` §5 files `audio/impact.py` as tier 1. This
//! crate is that port. The rule it lands under is narrower than "rewrite everything": **Python
//! keeps only what does not translate** — MediaPipe pose, and the bands cut from its landmarks —
//! and a module is retired from Python once a conforming Rust implementation exists and the
//! vectors that prove it are committed.
//!
//! # Why the port is checkable at all
//!
//! `docs/CONFORMANCE.md` §5 named `audio/impact.py` end to end as the one tier-1 module the M19
//! suite could not cover: its vectors take strike *frames* as an input, so a port's detector was
//! unchecked, and `tests/audio/test_impact.py` synthesizes its clips from a seeded numpy RNG,
//! which does not reproduce in another language. M20 closes that with `spec/vectors/audio/` — 30
//! vectors, one per stored clip, recorded by the Python reference and verified against the
//! `{role}.audio.json` the pipeline actually read. `tests/conformance.rs` is what runs them.
//!
//! That matters more here than it does for the rest of the port, because the Python reference is
//! being **deleted** rather than kept alongside. The vectors, not the code, are what survive as
//! the oracle — so they had to be right before the delete, and there is no second chance at
//! recording them.
//!
//! # The two halves
//!
//! - [`offline`] is `detect_strikes`: every transient in a clip that is already on disk. It is
//!   what the pipeline calls today and what the vectors pin.
//! - [`stream`] is the live trigger: the same signal, decided with bounded latency against
//!   a rolling floor, so that a session can cut a clip around a strike it has only just heard.
//!   The offline detector is its ground truth — precision and recall are measured against what
//!   it found on the same clips, which removes any need to hand-label anything.
//!
//! [`ring`] and [`clip`] are what turn the second of those into a file: a bounded window of the
//! recent past, and the rules deciding how much of it a strike is worth. Those rules are not this
//! crate's to choose — they are read out of `analysis/phases.py::window_around`, because a clip
//! the analysis cannot frame a swing in is a clip that was not worth cutting.

pub mod clip;
pub mod flux;
pub mod offline;
pub mod offset;
pub mod ring;
pub mod stream;
pub mod strike;
#[cfg(test)]
pub(crate) mod testing;

pub use clip::{extract, Clip, Cutter};
pub use flux::{envelope, Geometry};
pub use offline::{detect_in_envelope, detect_strikes};
pub use offset::{offset_between, ClipOffset};
pub use ring::Ring;
pub use stream::{OnlineDetector, Trigger};
pub use strike::Strike;
