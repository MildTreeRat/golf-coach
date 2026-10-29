//! What `golf-lab analyze` prints: `scripts/analyze_bundle.py`'s `_print_report`, `_print_shot` and
//! `_print_coaching` after a run, and `_print_swings` under `--list-swings`. [M29 P26]
//!
//! **The script's text, line for line** (the M29 plan's C8), each format spec through `pyfmt`'s
//! CPython edges: `:.0f`, `:.2f` and `:.1f` through [`pyfmt::fixed`], `:g` through [`pyfmt::g`],
//! `:.0%` through [`pyfmt::percent`], and a bare float in an f-string through [`pyfmt::repr`], which
//! is `str(float)`. The coaching paragraph is wrapped by `textwrap.wrap(width=88)`, which is
//! `golf_core::textwrap`'s port of it.
//!
//! # The one line that is not the script's
//!
//! `not measured: …` joins `swing.unscored` in Python, which was a list of names when the line was
//! written and has been a list of `UnscoredCheckpoint` since: `', '.join` raises `TypeError` on
//! the first entry, after `analysis.json` and its state are written and before anything else of the
//! report prints, and the script exits 1 with a traceback. Measured at P26 on two of three stored
//! bundles. This prints what the line was written to print, the names, and the rest of the report
//! after it.
//!
//! **`analyze_swing.py`'s report shares two blocks** (P27): the checkpoints, `not measured:` and
//! its crash included, and the tips, written once here ([`print_checkpoints`], [`print_tips`]) for
//! `golf_lab::analyze_keypoints`.

use std::io::{self, Write};

use analysis::phases::{candidate_downswings, window_around, CANDIDATE_MIN_RISE, LEAD_WRIST};
use analysis::smoothing::{smooth_keypoints, DEFAULT_WINDOW};
use contracts::feedback::FeedbackPayload;
use contracts::keypoints::KeypointsFile;
use contracts::shot::ShotData;
use contracts::swing::{SwingBundleResult, SwingResult};
use golf_core::textwrap;
use serde_json::Value;

/// `_print_report`.
pub fn print_report(result: &SwingBundleResult, out: &mut dyn Write) -> io::Result<()> {
    let swing = &result.swing;
    writeln!(
        out,
        "\nSwing {}  (session {})",
        result.swing_id, result.session_id
    )?;
    let mut overall = format!(
        "Overall score: {}/100",
        pyfmt::fixed(swing.overall_score, 0)
    );
    if let Some(mechanics) = swing.mechanics_score {
        overall.push_str(&format!("  (mechanics {}/100)", pyfmt::fixed(mechanics, 0)));
    }
    writeln!(out, "{overall}")?;

    print_checkpoints(swing, out)?;

    if let Some(alignment) = &result.alignment {
        if let (Some(a), Some(b)) = (&alignment.a, &alignment.b) {
            let (a, b) = (&a.anchors, &b.anchors);
            writeln!(out, "\nAlignment: {}", alignment.quality.summary())?;
            writeln!(out, "  face-on        top {}  impact {}", a.top, a.impact)?;
            writeln!(out, "  down-the-line  top {}  impact {}", b.top, b.impact)?;
        }
    }

    print_shot(swing.shot.as_ref(), out)?;

    // Before the tips, as the script has it: a note can say a checkpoint's own input was
    // impossible, and a headline built on that number must not be read without it.
    if !result.notes.is_empty() {
        writeln!(out, "\nNotes:")?;
        for note in &result.notes {
            writeln!(out, "  ! {note}")?;
        }
    }

    if let Some(feedback) = &result.feedback {
        if let Some(headline) = feedback.headline.as_deref().filter(|h| !h.is_empty()) {
            writeln!(out, "\n>> {headline}")?;
        }
        print_tips(feedback, out)?;
        print_coaching(feedback, out)?;
    }
    Ok(())
}

/// The checkpoint block both scripts print, `analyze_bundle.py`'s and `analyze_swing.py`'s
/// `_print_report` alike, `not measured:` included (the module doc).
pub(crate) fn print_checkpoints(swing: &SwingResult, out: &mut dyn Write) -> io::Result<()> {
    writeln!(out, "\nCheckpoints:")?;
    if swing.checkpoint_scores.is_empty() {
        writeln!(
            out,
            "  (none scored - no benchmark band matched, or the swing was unsegmentable)"
        )?;
    }
    for cp in &swing.checkpoint_scores {
        let band = match cp.expected_low {
            Some(low) => format!("[{} - {}]", repr(Some(low)), repr(cp.expected_high)),
            None => "[no band]".to_string(),
        };
        let place = cp
            .percentile
            .map_or_else(String::new, |pct| format!("  tour pct={}", pyfmt::g(pct)));
        writeln!(
            out,
            "  {:<16} observed={}  band={band}  score={}  {}{place}",
            cp.name,
            repr(cp.observed),
            pyfmt::percent(cp.score, 0),
            if cp.passed { "PASS" } else { "MISS" }
        )?;
    }
    if !swing.unscored.is_empty() {
        let names: Vec<&str> = swing
            .unscored
            .iter()
            .map(|entry| entry.name.as_str())
            .collect();
        writeln!(out, "  not measured: {}", names.join(", "))?;
    }
    Ok(())
}

/// The tips, most actionable first, as both scripts print them.
pub(crate) fn print_tips(feedback: &FeedbackPayload, out: &mut dyn Write) -> io::Result<()> {
    writeln!(out, "\nTips (most actionable first):")?;
    for tip in &feedback.tips {
        writeln!(
            out,
            "  [{:<5}] {}",
            wire_name(serde_json::to_value(tip.severity)).to_uppercase(),
            tip.text
        )?;
    }
    Ok(())
}

/// `_print_coaching`: the paragraph under a heading that says who wrote it, attribution first.
fn print_coaching(feedback: &FeedbackPayload, out: &mut dyn Write) -> io::Result<()> {
    let (Some(text), Some(coaching)) = (
        feedback.coaching_text.as_deref().filter(|t| !t.is_empty()),
        &feedback.coaching,
    ) else {
        return Ok(());
    };
    writeln!(
        out,
        "\nCoach ({}, written from the numbers above):",
        coaching.model
    )?;
    for line in textwrap::wrap(text, 88, "", "") {
        writeln!(out, "  {line}")?;
    }
    Ok(())
}

/// `_print_shot`: the launch monitor's numbers, and how much to trust them (ADR-014).
fn print_shot(shot: Option<&ShotData>, out: &mut dyn Write) -> io::Result<()> {
    writeln!(out, "\nShot data:")?;
    let Some(shot) = shot else {
        return writeln!(out, "  (none attached)");
    };
    let provenance = shot.provenance.as_ref();
    let confidence = provenance.map_or(1.0, |p| p.parse_confidence);
    let flag = if provenance.is_some_and(|p| p.needs_review) {
        "  ** NEEDS REVIEW **"
    } else {
        ""
    };
    writeln!(
        out,
        "  {}   source={}  conf={}{flag}",
        shot.shot_id,
        wire_name(serde_json::to_value(shot.source)),
        pyfmt::fixed(confidence, 2)
    )?;
    // `_SHOT_FIELDS`, in its order: a number through `:g`, a string as itself.
    let numbers = [
        ("carry", shot.carry_distance),
        ("total", shot.total_distance),
        ("ball", shot.ball_speed),
        ("club", shot.club_head_speed),
        ("smash", shot.smash_factor),
        ("path", shot.club_path),
        ("face", shot.club_face_angle),
        ("launch", shot.launch_angle),
    ];
    let words = [
        ("shape", shot.shot_type.as_deref()),
        ("strike", shot.impact_position.as_deref()),
    ];
    let metrics: Vec<String> = numbers
        .iter()
        .filter_map(|(label, value)| value.map(|value| format!("{label}={}", pyfmt::g(value))))
        .chain(
            words
                .iter()
                .filter_map(|(label, value)| value.map(|value| format!("{label}={value}"))),
        )
        .collect();
    writeln!(out, "  {}", metrics.join("  "))?;
    if let Some(provenance) = provenance {
        for warning in &provenance.warnings {
            writeln!(out, "    - {warning}")?;
        }
    }
    Ok(())
}

/// `_print_swings`: every descent of the hands in one view, which is how a swing the rule missed is
/// found. `flag` is the option that selects a window in this view.
pub fn print_swings(
    label: &str,
    flag: &str,
    keypoints: &KeypointsFile,
    out: &mut dyn Write,
) -> io::Result<()> {
    // `if fps`: a recorded zero is no rate, as Python's truthiness has it.
    let fps = keypoints
        .clip
        .as_ref()
        .and_then(|clip| clip.fps)
        .filter(|&fps| fps != 0.0);
    let swings = candidate_downswings(
        &smooth_keypoints(&keypoints.frames, DEFAULT_WINDOW),
        CANDIDATE_MIN_RISE,
        LEAD_WRIST,
    );
    writeln!(
        out,
        "\n{label}: {} frames, {} candidate descent(s)",
        keypoints.frames.len(),
        swings.len()
    )?;
    if swings.is_empty() {
        return writeln!(out, "  (none - no detectable descent of the hands)");
    }
    writeln!(
        out,
        "  {:>2}  {:>7}  {:>6}  {:>6}  {:>10}  select with",
        "#", "at", "top", "impact", "downswing"
    )?;
    for (i, swing) in swings.iter().enumerate() {
        let frames = swing.impact as i64 - swing.top as i64;
        let (at, duration) = match fps {
            Some(fps) => (
                format!("{:>6}s", pyfmt::fixed(swing.top as f64 / fps, 1)),
                format!("{:>8}s", pyfmt::fixed(frames as f64 / fps, 2)),
            ),
            None => (format!("{:>6}f", swing.top), format!("{frames:>7}f")),
        };
        let (start, end) = window_around(swing, fps);
        writeln!(
            out,
            "  {i:>2}  {at:>7}  {:>6}  {:>6}  {duration:>10}  {flag} {start}:{end}",
            swing.top, swing.impact
        )?;
    }
    writeln!(
        out,
        "  a real downswing runs ~0.2-0.3s; much longer is a rehearsal or a setup move."
    )
}

/// `str()` of an optional float: its `repr`, or `None`.
fn repr(value: Option<f64>) -> String {
    value.map_or_else(|| "None".to_string(), pyfmt::repr)
}

/// An enum's wire name, through its own `Serialize`: `severity.value`, `source.value`.
fn wire_name(value: Result<Value, serde_json::Error>) -> String {
    match value {
        Ok(Value::String(name)) => name,
        other => unreachable!("a unit enum serializes to its name, not {other:?}"),
    }
}
