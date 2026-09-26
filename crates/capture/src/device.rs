//! What a camera *is*, and what it can be asked for. [M21 P1]
//!
//! Backend-free on purpose. Media Foundation, V4L2 and AVFoundation each describe a camera in
//! their own vocabulary; this module is the one a session, a manifest and a `golf-capture list`
//! row are written against, so that adding the second platform is a new backend module beside
//! `mf` and not a new set of types for everything downstream to branch on.

use std::fmt;

/// A camera's identity: the thing a session writes down and has to recognise tomorrow.
///
/// **Not an index**, which is ADR-031 §7's whole point — an index is a position in a list and
/// another device can take it. On Windows this string is Media Foundation's
/// `MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE_VIDCAP_SYMBOLIC_LINK`, the device-interface path.
///
/// # What it survives, measured rather than assumed
///
/// "Survives a replug" is weaker than it sounds, and the camera this repo owns is the weak case.
/// Its interface path, read out of the registry while it was unplugged, is
///
/// ```text
/// \\?\USB#VID_1BCF&PID_28C4&MI_00#6&1da11eb4&2&0000#{e5323777-f976-4f5b-9b55-b94699c46e44}
/// ```
///
/// and the middle segment is not a serial number — it is the instance id Windows *generates*
/// from the hub and port, which is what it does for a USB device that reports no serial. (A
/// device on the same machine that does report one enumerates as
/// `USB\VID_0CF2&PID_A100\6243168001`: the serial, verbatim, in that position.)
///
/// So for this camera the identity names **the device and the port**, and:
///
/// - a replug into the same port keeps it;
/// - a move to a different port changes it, and the session must say the camera is *gone*
///   rather than adopt whatever is now in the old position;
/// - two identical cameras — and ADR-003 buys two — are told apart *only* by port.
///
/// That last point is the reason this is recorded as a property rather than a limitation. A
/// two-camera rig has a face-on camera and a down-the-line camera, and what makes one of them
/// the face-on one is which tripod it is bolted to, not which of two identical serial-less
/// sensors it happens to be. The port is the closest thing the host has to that fact.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DeviceId(String);

impl DeviceId {
    /// Wrap a backend's identifier. Opaque above this crate: nothing downstream parses it,
    /// because the three platforms agree on nothing about its shape.
    pub fn new(raw: impl Into<String>) -> Self {
        Self(raw.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for DeviceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// One camera the host can see.
#[derive(Debug, Clone)]
pub struct Device {
    pub id: DeviceId,
    /// What the driver calls itself — "FHD Camera", "Integrated Webcam". For a human reading a
    /// list, and never for deciding which device is which: it is neither unique nor stable.
    pub name: String,
    pub capabilities: Capabilities,
}

/// What a camera says it can be asked for — or why it would not say.
///
/// Two variants rather than a plain `Vec`, for the reason ADR-010 §2 gives everywhere else in
/// this repo: a camera that is already open in another application enumerates fine and refuses
/// to describe itself, and an empty `Vec` for that case reads as *"this camera offers no
/// formats"*. It is not the same fact and the caller must not have to guess which one it has.
#[derive(Debug, Clone)]
pub enum Capabilities {
    Reported(Vec<Capability>),
    /// The device enumerated but would not open, or opened and would not describe itself. The
    /// string is the platform's own account of why, passed through rather than summarised.
    Unavailable(String),
}

impl Capabilities {
    /// The formats, or an empty slice when the device would not say. For display only — a caller
    /// deciding what to *record at* must match on the variant.
    pub fn reported(&self) -> &[Capability] {
        match self {
            Self::Reported(c) => c,
            Self::Unavailable(_) => &[],
        }
    }
}

/// One `(resolution, frame rate, pixel format)` a camera offers.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Capability {
    pub width: u32,
    pub height: u32,
    /// Frame rate as the ratio the driver reported, **not** as a float.
    ///
    /// 30000/1001 is a real UVC frame rate and it is not 30. Rounding it here would put a 0.1%
    /// error into every timestamp derived from a frame count, and the capture edge is the one
    /// place in this repo where a timing error cannot be recovered afterwards — `phases.py` is
    /// measuring tempo in hundredths of a second off these frames.
    pub fps_numerator: u32,
    pub fps_denominator: u32,
    pub format: PixelFormat,
}

impl Capability {
    /// The frame rate as a number, or `None` if the driver reported a zero denominator.
    ///
    /// `None` rather than a default: a rate nobody can compute is not 30 fps and is not 0 fps,
    /// and this crate does not get to invent one.
    pub fn fps(&self) -> Option<f64> {
        match self.fps_denominator {
            0 => None,
            d => Some(f64::from(self.fps_numerator) / f64::from(d)),
        }
    }
}

impl fmt::Display for Capability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.fps() {
            Some(fps) => write!(
                f,
                "{}x{} @ {:.3} fps ({}/{}) {}",
                self.width, self.height, fps, self.fps_numerator, self.fps_denominator, self.format
            ),
            None => write!(
                f,
                "{}x{} @ ? fps ({}/{}) {}",
                self.width, self.height, self.fps_numerator, self.fps_denominator, self.format
            ),
        }
    }
}

/// How the pixels arrive.
///
/// Decisive for M21 rather than descriptive: ADR-031 §6 sizes the ring in **encoded** frames
/// because raw 1080p60 is ~180 MB/s and 31 s of it is ~5.6 GB per camera. Whether a device can
/// hand over MJPG or H264 itself, or only NV12/YUY2 that the host must then encode, is the
/// question P2 answers — and this field is where the answer is read.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum PixelFormat {
    /// A FourCC — `MJPG`, `NV12`, `YUY2`, `H264`. The shape every UVC format takes.
    FourCc([u8; 4]),
    /// A media subtype that is not FourCC-shaped. Kept verbatim rather than dropped: a format
    /// this crate does not recognise is still a format the camera offers, and silently losing
    /// the row would make `list` a liar about what the hardware can do.
    Other(String),
}

impl fmt::Display for PixelFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FourCc(cc) => {
                for &b in cc {
                    // Trailing spaces are real in FourCCs ("Y8  "); non-ASCII is not, and a
                    // control byte reaching a terminal is worse than a visible placeholder.
                    let c = if b.is_ascii_graphic() || b == b' ' {
                        b as char
                    } else {
                        '?'
                    };
                    f.write_fmt(format_args!("{c}"))?;
                }
                Ok(())
            }
            Self::Other(s) => f.write_str(s),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fourcc_prints_as_its_four_characters() {
        assert_eq!(PixelFormat::FourCc(*b"MJPG").to_string(), "MJPG");
        assert_eq!(PixelFormat::FourCc(*b"NV12").to_string(), "NV12");
    }

    #[test]
    fn a_fourcc_with_padding_keeps_it_and_one_with_a_control_byte_does_not() {
        assert_eq!(PixelFormat::FourCc(*b"Y8  ").to_string(), "Y8  ");
        assert_eq!(
            PixelFormat::FourCc([b'A', 0x01, b'B', b'C']).to_string(),
            "A?BC"
        );
    }

    #[test]
    fn ntsc_frame_rate_is_not_thirty() {
        let cap = Capability {
            width: 1920,
            height: 1080,
            fps_numerator: 30_000,
            fps_denominator: 1001,
            format: PixelFormat::FourCc(*b"MJPG"),
        };
        let fps = cap.fps().expect("a 1001 denominator is computable");
        assert!((fps - 29.970_029_97).abs() < 1e-6, "got {fps}");
        assert_ne!(fps, 30.0);
    }

    #[test]
    fn a_zero_denominator_refuses_rather_than_defaulting() {
        let cap = Capability {
            width: 640,
            height: 480,
            fps_numerator: 30,
            fps_denominator: 0,
            format: PixelFormat::FourCc(*b"YUY2"),
        };
        assert_eq!(cap.fps(), None);
        assert!(cap.to_string().contains("? fps"));
    }

    #[test]
    fn unavailable_capabilities_are_not_an_empty_list() {
        let unavailable = Capabilities::Unavailable("device in use".into());
        assert!(unavailable.reported().is_empty());
        // The distinction the type exists for: `reported()` flattens both to nothing, so a
        // caller that cares has to match, and this test is what notices if the enum is ever
        // collapsed into a bare `Vec`.
        assert!(matches!(unavailable, Capabilities::Unavailable(_)));
    }
}
