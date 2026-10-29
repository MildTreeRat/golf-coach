//! `golf-lab analyze-keypoints`: one keypoints file analysed with no bundle around it, and the
//! report `scripts/analyze_swing.py` prints. [M29 P27]
//!
//! The analysis spine and nothing else, as the script runs it: `analyze_swing` over the file's
//! frames, with no shot, no intent, no handedness and no loft, then `build_feedback`. It is the
//! no-hardware check that segmentation landed on the right moments, so the report names the three
//! instants the overlay would stamp, read from `alignment::anchors_from_phases` as the script reads
//! them, so the frames printed here and the frames a warp pins to cannot disagree. With no golfer,
//! `head_stays_back` is always unscored (`no_handedness`), as it is in the script.
//!
//! # Where the text differs from the script's (the M29 plan's C8)
//!
//! - **`not measured:` prints the names**, where the script's `', '.join(result.unscored)` raises
//!   `TypeError` on the first `UnscoredCheckpoint`, and so on every run of it, since every run here
//!   has `head_stays_back` unscored: the script prints its checkpoints, then a traceback, and exits
//!   1 with no tips. `golf_lab::report`'s module doc says the same of `analyze_bundle.py`'s copy of
//!   the line, and the block is written once there ([`report::print_checkpoints`]).
//! - **No overlay** (C4): `--overlay`, which renders the annotated clip, is refused by the binary.
//! - **The arguments are parsed** where the script reads `argv[0]` and looks for `--overlay`
//!   anywhere: a second path or an unknown flag is refused, `--help` prints the usage on stdout with
//!   0 as every `golf-lab` verb does (the script printed it on stderr with 2), and no path is
//!   argparse's refusal, exit 2, as the script's own.

use std::io::{self, Write};
use std::path::Path;

use analysis::alignment::anchors_from_phases;
use analysis::engine::{analyze_swing, SwingRequest};
use contracts::swing::SwingResult;
use feedback::rules::build_feedback;
use storage::keypoints_io::load_keypoints;
use storage::StoreError;

use crate::report;

/// The session id the script analyses under: a keypoints file belongs to no session.
pub const SESSION_ID: &str = "cli";

/// The swing id the script names a result by: the file's stem with `.keypoints` dropped, so
/// `face_on.keypoints.json` reads as `face_on`, the clip it was posed from.
///
/// `Path.stem` by Python's rule, which differs from [`Path::file_stem`] on a name ending in a dot:
/// Python's suffix needs a character after the dot, so `x.` is its own stem.
pub fn swing_id_for(path: &Path) -> String {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let stem = match name.rfind('.') {
        Some(dot) if dot > 0 && dot + 1 < name.len() => &name[..dot],
        _ => name.as_str(),
    };
    stem.strip_suffix(".keypoints").unwrap_or(stem).to_string()
}

/// The script's `main` up to the report: the file read in either shape `load_keypoints` reads, and
/// the engine run over its frames.
pub fn analyze_keypoints(path: &Path) -> Result<SwingResult, StoreError> {
    let keypoints = load_keypoints(path)?;
    let swing_id = swing_id_for(path);
    Ok(analyze_swing(&SwingRequest {
        swing_id: &swing_id,
        session_id: SESSION_ID,
        keypoints: &keypoints.frames,
        detections: None,
        shot: None,
        intent: None,
        handedness: None,
        loft_deg: None,
    }))
}

/// `_instants`: the address, top and impact frames, or none for a clip that would not segment.
pub fn instants(result: &SwingResult) -> Vec<(&'static str, i64)> {
    match anchors_from_phases(&result.phases, None, None, 0) {
        Some(anchors) => vec![
            ("ADDRESS", anchors.motion_start),
            ("TOP OF BACKSWING", anchors.top),
            ("IMPACT", anchors.impact),
        ],
        None => Vec::new(),
    }
}

/// `_print_report`: the scores, the instants, the headline, the checkpoints and the tips, then the
/// script's closing `print()`.
pub fn print_keypoints_report(result: &SwingResult, out: &mut dyn Write) -> io::Result<()> {
    writeln!(
        out,
        "\nSwing: {}   frames: {}",
        result.swing_id,
        result.keypoints.len()
    )?;
    let mut overall = format!(
        "Overall score: {}/100",
        pyfmt::fixed(result.overall_score, 0)
    );
    if let Some(mechanics) = result.mechanics_score {
        overall.push_str(&format!("  (mechanics {}/100)", pyfmt::fixed(mechanics, 0)));
    }
    writeln!(out, "{overall}")?;
    let instants: Vec<String> = instants(result)
        .iter()
        .map(|(label, frame)| format!("{label} @ {frame}"))
        .collect();
    writeln!(out, "Detected instants (frame): {}", instants.join(", "))?;

    let feedback = build_feedback(result);
    if let Some(headline) = feedback.headline.as_deref().filter(|h| !h.is_empty()) {
        writeln!(out, "\n>> {headline}")?;
    }
    report::print_checkpoints(result, out)?;
    report::print_tips(&feedback, out)?;
    writeln!(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_swing_id_is_the_stem_without_keypoints() {
        for (path, want) in [
            ("data/x/face_on.keypoints.json", "face_on"),
            ("swing.json", "swing"),
            ("swing", "swing"),
            ("a.b.keypoints.json", "a.b"),
            ("dir/.keypoints", ""),
            ("x.", "x."),
        ] {
            assert_eq!(swing_id_for(Path::new(path)), want, "{path}");
        }
    }
}
