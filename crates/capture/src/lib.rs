//! The capture edge: the cameras a host can see, and what they can be asked for. [M21 P1]
//!
//! The second crate in this workspace, and the one ADR-031 decides into existence. Capture is
//! Rust because ADR-030 §1 gives Rust *"camera capture, the ring buffer, clip cutting"* and its
//! 2026-09-22 addendum keeps in Python only what does not translate — MediaPipe, and the lab. A
//! camera loop is neither, so no Python `LiveCameraSource` is written, now or later.
//!
//! What this crate will hand over, once P3 lands, is a **swing directory on disk** —
//! `storage/manifest.py`'s unit, read by `api/pipeline.py::analyze_swing` exactly as it is today
//! (ADR-031 §3). That is what takes M21 off M22's critical path. P1 is the first step of it:
//! naming the cameras.
//!
//! # What P1 is, and what it deliberately is not
//!
//! Enumeration only. [`list`] answers *"what can this host see, and what will each of them give
//! me"*; [`find`] answers *"is the camera this session was told to use still here"*. Neither
//! opens a stream. The capture loop, the arrival timestamps and the dropped-frame rate are P2;
//! the video ring over encoded frames is P3, and it instantiates `trigger::Ring` from the
//! sibling crate rather than writing a second one — `ring.rs` was made generic over its item
//! for exactly this.
//!
//! # Which camera crate — **unfinished, and this is the honest state**
//!
//! ADR-031 §4 names three candidates — `nokhwa`, an ffmpeg capture path, and the platform APIs
//! directly — and requires the choice be made *"against hardware rather than against
//! documentation"*. **M21 P1 could not make it**: the machine this repo is built on is a
//! desktop (MSI MS-7D25) with no built-in camera, and the one camera Windows has a record of is
//! a phantom entry — `CM_PROB_PHANTOM`, code 45, not connected. `nokhwa::query` returns zero
//! devices; so does the code below. A bake-off with nothing to enumerate decides nothing.
//!
//! So the backend here is a **provisional** choice with one measured fact behind it and one
//! structural one, and the verdict is still owed:
//!
//! - **Measured.** `nokhwa` 0.10.11 does not build under this workspace's `rust-version =
//!   "1.87"`: it pulls `image` 0.25.10, which requires 1.88. It builds with
//!   `default-features = false` and `image` pinned back to 0.25.5 — but its default `decoding`
//!   feature drags in `mozjpeg-sys` and `nasm-rs`, i.e. a C and NASM build toolchain, into the
//!   half of this repo whose entire dependency list is four crates.
//! - **Structural.** `nokhwa`'s Windows backend *is* Media Foundation — it depends on the same
//!   `windows` crate this module calls directly. Going direct costs the three platform backends
//!   (`nokhwa` is one API over all three) and buys the symbolic link, unabridged, which is the
//!   identity [`DeviceId`] is about.
//!
//! What would overturn it, on hardware: `nokhwa` reporting capabilities this module cannot, or
//! this module's enumeration missing a device `nokhwa` sees. The probe that measured the build
//! is not committed — it is six lines of `nokhwa::query` — and re-running it is the first thing
//! P1's completion asks for.
//!
//! # The invariant this does not break
//!
//! `CLAUDE.md` binds the *analysis core* to stdlib plus `contracts`, and ADR-030 §8 extends that
//! to the Rust core's scoring path. Capture is the I/O edge, where `crates/trigger` already
//! takes `rustfft` and `audio/ffmpeg.py` already shells out to a binary. The rule that was
//! always meant, and is kept: **no numeric library between a measurement and a verdict**
//! (ADR-031 §4).

pub mod device;

#[cfg(windows)]
mod mf;

pub use device::{Capabilities, Capability, Device, DeviceId, PixelFormat};

use std::fmt;

/// Why enumeration could not answer.
///
/// Note what is *not* in here: "no cameras". A host with nothing plugged in is a fact, not a
/// failure, and [`list`] returns an empty `Vec` for it.
#[derive(Debug)]
pub enum Error {
    /// The platform's capture stack would not start at all — Media Foundation failing
    /// `MFStartup`, a COM apartment that cannot be joined. Nothing can be enumerated.
    Platform(String),
    /// The camera a session was told to use is not here.
    ///
    /// ADR-031 §7: a session that loses its camera **stops and names the device that is gone**.
    /// It does not fall back to another camera, because footage from the wrong camera is a swing
    /// measured against the wrong view's bands — a confident wrong answer, which is the failure
    /// mode ADR-010 §2 refuses everywhere else. This variant is that refusal's raw material, so
    /// it carries the id rather than a message about it.
    Gone(DeviceId),
    /// No backend compiled in for this OS. Windows first, because that is the machine this is
    /// built on; V4L2 and AVFoundation are not foreclosed and are not written (ADR-031 §4).
    Unsupported(&'static str),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Platform(detail) => write!(f, "the capture stack would not start: {detail}"),
            Self::Gone(id) => write!(f, "camera is not connected: {id}"),
            Self::Unsupported(os) => write!(f, "no capture backend for {os} yet"),
        }
    }
}

impl std::error::Error for Error {}

/// Every camera this host can see, in a stable order.
///
/// Sorted by [`DeviceId`] rather than left in the platform's order, because the platform's order
/// is the thing ADR-031 §7 says not to trust: two runs of `golf-capture list` that disagree on
/// which camera is "the first one" would make the list unreadable as a record.
///
/// An empty `Vec` means nothing is plugged in. It is not an error and must not be reported as
/// one — as of M21 P1 it is also the only answer this repo has ever gotten back.
pub fn list() -> Result<Vec<Device>, Error> {
    #[cfg(windows)]
    {
        let mut devices = mf::enumerate()?;
        devices.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(devices)
    }
    #[cfg(not(windows))]
    {
        Err(Error::Unsupported(std::env::consts::OS))
    }
}

/// The camera with this id, or [`Error::Gone`] naming it.
///
/// The lookup a session does at every start and every resume. It matches on the id and never on
/// the name, because a name is neither unique nor stable — two identical cameras report the same
/// one, which is exactly the rig ADR-003 buys.
pub fn find(id: &DeviceId) -> Result<Device, Error> {
    list()?
        .into_iter()
        .find(|d| &d.id == id)
        .ok_or_else(|| Error::Gone(id.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_host_with_no_cameras_is_an_empty_list_and_not_an_error() {
        // Passes either way on purpose: this pins the *shape* of the answer, which is the thing
        // a session branches on. On the build machine (no camera attached) it exercises the
        // empty path; on a machine with one it exercises the populated path, and both are the
        // `Ok` this asserts. A backend that reported "no devices" as an error would fail here.
        let listed = list().expect("enumeration must not fail merely for finding nothing");
        for device in &listed {
            assert!(!device.id.as_str().is_empty(), "a device with no identity");
        }
    }

    #[test]
    fn an_unknown_id_is_gone_and_says_which() {
        let id = DeviceId::new(r"\\?\USB#VID_0000&PID_0000#nothing-is-plugged-in-here");
        match find(&id) {
            Err(Error::Gone(named)) => {
                assert_eq!(named, id);
                // The message is the one a session prints when it stops (ADR-031 §7), so it has
                // to carry the device, not just the fact.
                assert!(Error::Gone(named).to_string().contains("VID_0000"));
            }
            other => panic!("expected Gone, got {other:?}"),
        }
    }

    #[test]
    fn listing_twice_gives_the_same_order() {
        let first: Vec<_> = list().unwrap().into_iter().map(|d| d.id).collect();
        let second: Vec<_> = list().unwrap().into_iter().map(|d| d.id).collect();
        assert_eq!(first, second);
    }
}
