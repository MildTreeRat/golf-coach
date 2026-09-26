//! Launch-monitor shot data. `contracts/shot.py`. [M22 P2]
//!
//! One shape whatever the source: a real Garmin R10, a photo of an HD Golf screen, or the mock
//! (ADR-007). Swapping hardware changes which adapter fills it in, never the consumers (ADR-006).
//! Field set and units come from ADR-004. Every metric is optional because not all sources report
//! every field — the HD Golf screen leaves spin blank (ADR-014).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{ge, le, nested, ContractError, Timestamp, Validate};

/// Where a `ShotData` record came from — for provenance and debugging.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShotSource {
    Mock,
    R10,
    /// Bulk export / historical JSON.
    Import,
    /// Parsed from a launch-monitor screen image/frame (ADR-014).
    Screen,
}

impl ShotSource {
    /// The wire name, for a sentence rather than for a serializer. [M22 P8b]
    ///
    /// `engine::shot_measurements` writes `Measurement.source` as `launch_monitor:{device}`, and the
    /// device is `provenance.device` where there is one and this where there is not — so on a shot
    /// with no provenance this string reaches an artifact field `docs/CONFORMANCE.md` §3 compares
    /// exactly. [`crate::intent::ClubCategory::as_str`]'s argument for a hand-written table.
    ///
    /// `R10` is `"r10"` and not `"r_10"`, which is what serde's `rename_all` produces for it and what
    /// `contracts/shot.py` spells — the one member where the two conventions could have disagreed.
    pub fn as_str(self) -> &'static str {
        match self {
            ShotSource::Mock => "mock",
            ShotSource::R10 => "r10",
            ShotSource::Import => "import",
            ShotSource::Screen => "screen",
        }
    }
}

/// How a shot was obtained, for sources that *infer* rather than *receive* metrics.
///
/// A shot read off a photograph is a measurement of a measurement: OCR can drop a digit and produce
/// a number that is wrong but perfectly plausible. Carrying the confidence, the physics-check
/// warnings and the raw on-screen text alongside the metrics is what makes a bad parse auditable
/// after the fact instead of silently poisoning a session's scores (ADR-014). Sources that receive
/// metrics directly (R10, mock) leave this `None`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Self")]
pub struct ShotProvenance {
    /// Device profile that parsed this, e.g. `hd_golf`.
    pub device: String,
    /// 0-1; 1 = fully trusted.
    pub parse_confidence: f64,
    /// True when confidence fell below threshold or a physics check failed.
    #[serde(default)]
    pub needs_review: bool,
    /// Human-readable reasons this parse is suspect.
    #[serde(default)]
    pub warnings: Vec<String>,
    /// Content hash of the source image.
    #[serde(default)]
    pub image_sha256: Option<String>,
    /// Where the image was read from.
    #[serde(default)]
    pub image_path: Option<String>,
    /// On-screen label -> raw recognized text, exactly as read.
    ///
    /// A `BTreeMap`, not an insertion-ordered one. ADR-032 §3's third edge — dict order deciding
    /// which name lands in a sentence — is about the ordered share maps in `benchmarks/`; nothing
    /// iterates this one, and the committed vectors are written with sorted keys, so sorted is both
    /// deterministic and what is on disk.
    #[serde(default)]
    pub raw_fields: BTreeMap<String, String>,
}

crate::validated!(ShotProvenance);

impl Validate for ShotProvenance {
    fn validate(&self) -> Result<(), ContractError> {
        ge(
            "ShotProvenance.parse_confidence",
            self.parse_confidence,
            0.0,
        )?;
        le(
            "ShotProvenance.parse_confidence",
            self.parse_confidence,
            1.0,
        )
    }
}

/// Metrics for a single shot. Units in the field docs (ADR-004).
///
/// Carries no bounds at all, and that is the Python's choice too: a launch monitor printing a
/// negative club path or a smash factor above 2 is reporting a mishit or a bad OCR read, and
/// [`ShotProvenance`] is where that is judged. Refusing the record would lose the evidence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Self")]
pub struct ShotData {
    pub shot_id: String,
    pub session_id: String,
    pub timestamp: Timestamp,
    #[serde(default = "ShotData::default_source")]
    pub source: ShotSource,

    // Club metrics — what the camera-observed mechanics should correlate with.
    /// mph
    #[serde(default)]
    pub club_head_speed: Option<f64>,
    /// degrees, + = open
    #[serde(default)]
    pub club_face_angle: Option<f64>,
    /// degrees, + = in-to-out
    #[serde(default)]
    pub club_path: Option<f64>,

    // Ball metrics.
    /// mph
    #[serde(default)]
    pub ball_speed: Option<f64>,
    /// degrees, vertical
    #[serde(default)]
    pub launch_angle: Option<f64>,
    /// degrees, + = right
    #[serde(default)]
    pub launch_direction: Option<f64>,
    /// rpm
    #[serde(default)]
    pub spin_rate: Option<f64>,
    /// degrees, + = fade
    #[serde(default)]
    pub spin_axis: Option<f64>,
    /// `ball_speed / club_head_speed`
    #[serde(default)]
    pub smash_factor: Option<f64>,

    // Flight estimates.
    /// yards
    #[serde(default)]
    pub carry_distance: Option<f64>,
    /// yards
    #[serde(default)]
    pub total_distance: Option<f64>,
    /// yards, total - carry
    #[serde(default)]
    pub bounce_and_roll: Option<f64>,
    /// yards
    #[serde(default)]
    pub apex_height: Option<f64>,

    // Categorical read-outs. Free text rather than enums: these are the source's own vocabulary
    // (HD Golf says "SLIGHT DRAW", the R10 will say something else), and normalizing them belongs
    // in analysis, not at the ingest boundary.
    /// e.g. `DRAW`, `SLIGHT FADE`
    #[serde(default)]
    pub shot_type: Option<String>,
    /// e.g. `CENTER`, `TOE`
    #[serde(default)]
    pub impact_position: Option<String>,

    /// Set only by sources that infer metrics (screen capture); `None` for direct feeds.
    #[serde(default)]
    pub provenance: Option<ShotProvenance>,
}

crate::validated!(ShotData);

impl ShotData {
    fn default_source() -> ShotSource {
        ShotSource::Mock
    }
}

impl Validate for ShotData {
    fn validate(&self) -> Result<(), ContractError> {
        nested("ShotData.provenance", self.provenance.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_source_wire_names_are_the_python_values() {
        let pairs = [
            (ShotSource::Mock, "\"mock\""),
            (ShotSource::R10, "\"r10\""),
            (ShotSource::Import, "\"import\""),
            (ShotSource::Screen, "\"screen\""),
        ];
        for (variant, wire) in pairs {
            assert_eq!(serde_json::to_string(&variant).unwrap(), wire);
        }
    }

    #[test]
    fn a_confidence_above_one_is_refused() {
        let json = r#"{"device":"hd_golf","parse_confidence":1.2}"#;
        assert!(serde_json::from_str::<ShotProvenance>(json).is_err());
    }
}
