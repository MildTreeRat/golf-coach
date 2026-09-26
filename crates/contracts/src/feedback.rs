//! The user-facing feedback payload. `contracts/feedback.py`. [M22 P2]
//!
//! The score, the structured rule-based tips, the LLM coaching text and a pointer to the annotated
//! replay. `feedback/rules.py` builds the first two and is ported in P6; the prose is the sidecar's
//! (ADR-030 §2), so [`CoachingProvenance`] and `coaching_text` are carried here and written by
//! nothing in Rust.

use serde::{Deserialize, Serialize};

use crate::{each, ge, le, nested, ContractError, Timestamp, Validate};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Info,
    Minor,
    Major,
}

/// A single rule-based coaching tip tied to a checkpoint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Self")]
pub struct Tip {
    pub checkpoint: String,
    pub text: String,
    #[serde(default = "Tip::default_severity")]
    pub severity: Severity,
}

crate::validated!(Tip);

impl Tip {
    fn default_severity() -> Severity {
        Severity::Info
    }
}

impl Validate for Tip {
    fn validate(&self) -> Result<(), ContractError> {
        Ok(())
    }
}

/// Where the coaching prose came from. Present if and only if `coaching_text` is. [M6]
///
/// The same idea as `ShotProvenance`, for the same reason: coaching is *inferred* from the
/// measurements rather than measured, and a reader who cannot tell which is which will read a
/// sentence a model wrote as a number this repo stands behind.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Self")]
pub struct CoachingProvenance {
    /// Exact model id that produced the text.
    pub model: String,
    pub generated_at: Timestamp,
    /// sha256 of the brief the text was generated from — so a stored result can say whether it
    /// still describes the swing beside it, or whether the numbers moved underneath it.
    #[serde(default)]
    pub input_digest: Option<String>,
}

crate::validated!(CoachingProvenance);

impl Validate for CoachingProvenance {
    fn validate(&self) -> Result<(), ContractError> {
        Ok(())
    }
}

/// Everything the UI needs to render feedback for one swing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Self")]
pub struct FeedbackPayload {
    pub swing_id: String,
    pub overall_score: f64,
    /// The one thing to work on, or a clean bill of health naming the closest call. `None` when
    /// nothing could be scored, which a UI should render as "no verdict" rather than as "fine".
    #[serde(default)]
    pub headline: Option<String>,
    /// Ranked most-actionable first: failures by overshoot, then passes by percentile.
    #[serde(default)]
    pub tips: Vec<Tip>,
    /// Claude's coaching prose, written from the measured numbers in this payload (M6). `None` when
    /// coaching was disabled, unconfigured or failed — never a placeholder, so an absent verdict
    /// never reads as a neutral one. Always carries `coaching`.
    #[serde(default)]
    pub coaching_text: Option<String>,
    /// Which model wrote `coaching_text`, and when. `None` iff `coaching_text` is.
    #[serde(default)]
    pub coaching: Option<CoachingProvenance>,
    /// Path to the rendered overlay video, if generated.
    #[serde(default)]
    pub annotated_video_path: Option<String>,
}

crate::validated!(FeedbackPayload);

impl Validate for FeedbackPayload {
    fn validate(&self) -> Result<(), ContractError> {
        ge("FeedbackPayload.overall_score", self.overall_score, 0.0)?;
        le("FeedbackPayload.overall_score", self.overall_score, 100.0)?;
        each("FeedbackPayload.tips", &self.tips)?;
        nested("FeedbackPayload.coaching", self.coaching.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_severity_wire_names_are_the_python_values() {
        let pairs = [
            (Severity::Info, "\"info\""),
            (Severity::Minor, "\"minor\""),
            (Severity::Major, "\"major\""),
        ];
        for (variant, wire) in pairs {
            assert_eq!(serde_json::to_string(&variant).unwrap(), wire);
        }
    }

    #[test]
    fn a_score_out_of_the_hundred_point_scale_is_refused() {
        let json = r#"{"swing_id":"1","overall_score":101.0}"#;
        assert!(serde_json::from_str::<FeedbackPayload>(json).is_err());
    }
}
