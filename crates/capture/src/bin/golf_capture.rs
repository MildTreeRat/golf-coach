//! `golf-capture` — what cameras this host can see. [M21 P1]
//!
//! ```text
//! golf-capture list                 # every camera, with its identity and its formats
//! golf-capture list --formats       # ... and every format, not just the summary
//! ```
//!
//! **Text rather than JSON, and that is the difference from `golf-trigger`.** That binary emits
//! an `AudioFile` because `api/pipeline.py` parses it. Nothing parses this: the consumer of
//! enumeration is a session inside `crates/capture` itself, which calls [`capture::list`]
//! directly. A machine format with no reader is a schema nobody validates, so this prints for a
//! human and the day something needs to parse it is the day it gets `--json` and a schema in
//! `spec/`.
//!
//! On the machine this was written on it prints `no cameras` — see the crate docs; the build box
//! is a desktop with nothing plugged in.

use std::io::Write;
use std::process::ExitCode;

use capture::{Capabilities, Capability, Device};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str).unwrap_or("") {
        "list" => list(args.iter().any(|a| a == "--formats")),
        other => {
            eprintln!("unknown command {other:?}; expected `list`");
            ExitCode::from(2)
        }
    }
}

fn list(every_format: bool) -> ExitCode {
    let devices = match capture::list() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };
    let mut out = std::io::stdout().lock();
    if devices.is_empty() {
        // Not an error and not an empty page. A host with no cameras is a fact a user needs
        // stated, because the alternative reading — "the tool is broken" — is the one they will
        // reach for otherwise.
        let _ = writeln!(out, "no cameras");
        return ExitCode::SUCCESS;
    }
    for (i, device) in devices.iter().enumerate() {
        if i > 0 {
            let _ = writeln!(out);
        }
        let _ = print_device(&mut out, device, every_format);
    }
    ExitCode::SUCCESS
}

fn print_device(out: &mut impl Write, device: &Device, every_format: bool) -> std::io::Result<()> {
    writeln!(out, "{}", device.name)?;
    // The id on its own line, unabridged. It is long — a Media Foundation symbolic link is ~100
    // characters — and truncating it would produce something that looks copyable and is not.
    writeln!(out, "  id  {}", device.id)?;
    match &device.capabilities {
        Capabilities::Unavailable(why) => writeln!(out, "  formats  unavailable — {why}"),
        Capabilities::Reported(formats) if formats.is_empty() => {
            writeln!(out, "  formats  none reported")
        }
        Capabilities::Reported(formats) => {
            if every_format {
                writeln!(out, "  formats  {}", formats.len())?;
                for format in formats {
                    writeln!(out, "    {format}")?;
                }
                Ok(())
            } else {
                writeln!(
                    out,
                    "  formats  {} — best {}",
                    formats.len(),
                    best(formats).expect("the empty case is handled above")
                )?;
                writeln!(out, "           (--formats for all of them)")
            }
        }
    }
}

/// The format a summary line should show: most pixels per second, then most pixels.
///
/// A summary has to pick one row out of the many a webcam offers — how many is P1's first
/// unanswered question, since no camera has been attached — and "the first one" is the
/// platform's order, which ADR-031 §7 says not to read anything into. Pixels per second
/// is the honest ranking for a swing camera — this milestone's whole difficulty is that a golf
/// swing is fast — with resolution breaking the tie so that 1080p30 beats 720p30.
///
/// It is a display choice and nothing else. What a session *records* at is P2's, decided against
/// the ring budget in ADR-031 §6 rather than against the largest number here.
fn best(formats: &[Capability]) -> Option<&Capability> {
    formats
        .iter()
        .filter(|c| c.fps().is_some())
        .max_by(|a, b| {
            let rate =
                |c: &Capability| f64::from(c.width) * f64::from(c.height) * c.fps().unwrap_or(0.0);
            rate(a)
                .total_cmp(&rate(b))
                .then_with(|| (a.width * a.height).cmp(&(b.width * b.height)))
        })
        // Every format having an uncomputable rate is a broken driver, not an empty camera, so
        // fall back to something rather than claiming there are no formats at all.
        .or_else(|| formats.first())
}

#[cfg(test)]
mod tests {
    use super::*;
    use capture::PixelFormat;

    fn cap(width: u32, height: u32, num: u32, den: u32) -> Capability {
        Capability {
            width,
            height,
            fps_numerator: num,
            fps_denominator: den,
            format: PixelFormat::FourCc(*b"MJPG"),
        }
    }

    #[test]
    fn the_summary_picks_pixels_per_second_not_resolution() {
        let formats = vec![cap(1920, 1080, 30, 1), cap(1280, 720, 120, 1)];
        // 720p120 is 110M px/s against 1080p30's 62M. A swing camera wants the frame rate, and
        // a summary that led with "1920x1080" would hide that this camera can do 120.
        let picked = best(&formats).unwrap();
        assert_eq!((picked.width, picked.fps_numerator), (1280, 120));
    }

    #[test]
    fn resolution_breaks_a_tie_at_equal_rate() {
        let formats = vec![cap(1280, 720, 60, 1), cap(1920, 1080, 60, 1)];
        assert_eq!(best(&formats).unwrap().width, 1920);
    }

    #[test]
    fn a_format_with_an_uncomputable_rate_does_not_win_but_does_not_empty_the_list() {
        let formats = vec![cap(4096, 2160, 60, 0), cap(1280, 720, 30, 1)];
        assert_eq!(best(&formats).unwrap().width, 1280);
        assert!(best(&[cap(4096, 2160, 60, 0)]).is_some());
    }

    #[test]
    fn an_unavailable_device_says_why_and_still_prints() {
        let device = Device {
            id: capture::DeviceId::new(r"\\?\USB#VID_1BCF&PID_28C4"),
            name: "FHD Camera".into(),
            capabilities: Capabilities::Unavailable("access denied".into()),
        };
        let mut out = Vec::new();
        print_device(&mut out, &device, false).unwrap();
        let printed = String::from_utf8(out).unwrap();
        assert!(printed.contains("FHD Camera"), "{printed}");
        assert!(printed.contains("VID_1BCF"), "{printed}");
        // "unavailable" and not "0 formats" — the distinction `Capabilities` exists for has to
        // survive all the way to what a user reads.
        assert!(printed.contains("unavailable — access denied"), "{printed}");
    }
}
