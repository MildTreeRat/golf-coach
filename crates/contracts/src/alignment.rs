//! How two clips of one swing correspond. `contracts/alignment.py`. [M22 P2]
//!
//! Two hand-held phones film one swing and agree on nothing: not the frame rate, not the clip
//! start, not the duration. What they do agree on is the swing itself, so the correspondence is
//! built on three instants rather than on a clock:
//!
//! ```text
//! tau = 0  motion start        tau = 1  top of backswing        tau = 2  impact
//! ```
//!
//! `FramePairing` is not ported: it is the renderer's seam — `analysis/alignment.py` produces a
//! schedule of them and an overlay consumes it — and overlays are `api/`'s, which §M29 retires into
//! the Flutter shell rather than porting here.

use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize};

use crate::{ge, gt, nested, ContractError, Validate};

/// The middle fixed point of the axis, named because it appears in both the warp and its inverse and
/// a bare `1.0` in that arithmetic is unreadable. [M22 P7]
///
/// **Its two siblings are not here**, on the rule P2 set for the registries: `TAU_MOTION_START` and
/// `TAU_IMPACT` are read only by `pose/side_by_side.py`'s banner table, which is an overlay and
/// therefore `api/`'s — retired into the Flutter shell by §M29 rather than ported. `TAU_TOP` has a
/// second reader that *is* ported, `analysis::alignment::tau_of_frame`, which is why it arrives
/// alone.
pub const TAU_TOP: f64 = 1.0;

/// How much of the swing the two clips were actually aligned *on*.
///
/// Ordered from most to least evidence. The value is what a UI should surface — rendering a
/// side-by-side video implies frame correspondence everywhere, and only `Full` earns that.
///
/// **`Synchronized` is a different kind of claim from the three below it** (M11 P6). Those count
/// *inferred* anchors: three pose estimates agree, or two, or one. `Synchronized` says the tau=2
/// anchor was **heard** in both clips — the ball strike reaches both microphones, so it is the one
/// instant the two recordings share on a real clock rather than by inference. That is why it sits
/// above `Full` instead of beside it, and why it overwrites the count: one measured anchor is
/// better evidence than three estimated ones.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlignmentQuality {
    Synchronized,
    Full,
    TopImpact,
    ImpactOnly,
    Unaligned,
}

impl AlignmentQuality {
    /// One human-readable clause, for a HUD line or a results page.
    ///
    /// Reaches two compared strings, so it is prose the suite pins exactly: it is serialized as
    /// [`SwingAlignment::quality_summary`], and `engine.py` appends `"alignment degraded: {…}"` to
    /// `SwingBundleResult.notes`.
    pub fn summary(self) -> &'static str {
        match self {
            Self::Synchronized => "synchronized on the ball strike",
            Self::Full => "aligned on motion start, top and impact",
            Self::TopImpact => "aligned on top and impact",
            Self::ImpactOnly => "aligned on impact only",
            Self::Unaligned => "not aligned",
        }
    }

    pub fn is_aligned(self) -> bool {
        self != Self::Unaligned
    }

    /// Whether the warp is standing on less than the best evidence available to it.
    ///
    /// **Deliberately not the same test as the one that gates the alignment caveat**, which is
    /// `!= Full` at each of its two call sites. This one asks *did something go wrong* — and a pair
    /// anchored on a heard ball strike is the opposite of that, so a `Synchronized` result must not
    /// be reported as degraded.
    pub fn is_degraded(self) -> bool {
        !matches!(self, Self::Synchronized | Self::Full)
    }
}

impl Validate for AlignmentQuality {
    fn validate(&self) -> Result<(), ContractError> {
        Ok(())
    }
}

/// The three swing instants one clip contributes to the warp, as frame indices.
///
/// Built either from `segment_phases()` output or by hand when a detector has to be overridden.
/// Both routes produce this same shape on purpose: the manual path is a *parameter* of the
/// alignment, not a second code path through it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Self")]
pub struct SwingAnchors {
    pub motion_start: i64,
    pub top: i64,
    pub impact: i64,

    /// False when `motion_start` is `segment_phases`' bounded estimate rather than something found
    /// in the signal (ADR-013). It is still the best available number — it just may not be used as
    /// a shared anchor, because two clips guessing separately do not agree.
    #[serde(default = "crate::yes")]
    pub motion_start_detected: bool,

    /// True when `impact` was pinned to a ball strike **heard** in this clip rather than inferred
    /// from pose (M11 P6). Defaults false so every artifact written before M11 reads back as what it
    /// was — inferred — rather than silently claiming a measurement nobody took.
    #[serde(default)]
    pub impact_measured: bool,

    /// Which view this clip is, when the keypoints recorded it.
    #[serde(default)]
    pub camera_id: Option<String>,
    /// Clip length, used to clamp mapped indices in range.
    #[serde(default)]
    pub frame_count: Option<i64>,
    /// Container-reported frame rate. Nothing in the *warp* needs it — that is the whole point of a
    /// normalized axis — but everything that compares the two clips in real seconds does.
    #[serde(default)]
    pub fps: Option<f64>,
}

crate::validated!(SwingAnchors);

impl SwingAnchors {
    /// Frames from the top to impact — the clip's own time base (ADR-013).
    pub fn downswing_frames(&self) -> i64 {
        self.impact - self.top
    }

    pub fn backswing_frames(&self) -> i64 {
        self.top - self.motion_start
    }

    /// Backswing:downswing, or `None` when there is no backswing to measure.
    ///
    /// Frame rate cancels, so two clips of the *same* swing must agree on this however differently
    /// the phones were configured. That is what makes it usable as a cross-check on the soft anchor
    /// rather than merely a checkpoint metric.
    pub fn tempo_ratio(&self) -> Option<f64> {
        if self.backswing_frames() <= 0 {
            return None;
        }
        Some(self.backswing_frames() as f64 / self.downswing_frames() as f64)
    }
}

impl Validate for SwingAnchors {
    fn validate(&self) -> Result<(), ContractError> {
        ge("SwingAnchors.motion_start", self.motion_start, 0)?;
        ge("SwingAnchors.top", self.top, 0)?;
        ge("SwingAnchors.impact", self.impact, 0)?;
        ge("SwingAnchors.frame_count", self.frame_count, 0)?;
        gt("SwingAnchors.fps", self.fps, 0.0)?;
        // A swing goes motion start -> top -> impact, and impact is strictly after the top.
        // `motion_start == top` is tolerated (a clip that opens mid-takeaway); a zero-length
        // downswing is not, because it is the denominator of the entire tau axis. The asymmetry is
        // the whole of ADR-032 §4's first named validator.
        if self.motion_start > self.top {
            return Err(ContractError {
                field: "SwingAnchors".to_string(),
                problem: format!(
                    "motion_start ({}) must not be after top ({})",
                    self.motion_start, self.top
                ),
            });
        }
        if self.impact <= self.top {
            return Err(ContractError {
                field: "SwingAnchors".to_string(),
                problem: format!("impact ({}) must be after top ({})", self.impact, self.top),
            });
        }
        Ok(())
    }
}

/// One clip's place on the shared tau axis.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Self")]
pub struct ClipAlignment {
    /// The instants as detected or supplied.
    pub anchors: SwingAnchors,

    /// The frame the warp actually pins to tau=0. Equal to `anchors.motion_start` when the soft
    /// anchor was accepted; otherwise the tour-median estimate substituted into *both* clips, so a
    /// degraded pre-top region degrades identically in each rather than in one panel only.
    pub warp_motion_start: i64,

    /// The frame the warp pins to tau=1, when that is *not* `anchors.top`. Only the `ImpactOnly`
    /// tier sets it. `None` on every other tier — and on artifacts written before the tier existed.
    /// Read it through [`ClipAlignment::top`].
    #[serde(default)]
    pub warp_top: Option<i64>,

    /// How many of **this clip's** frames its detected top sits past the real one, when a shared
    /// clock can say (M11 P7/P8). `None` means either that the tops agree or that nothing could
    /// arbitrate them; **it never means zero**, which is why the bound is `gt` rather than `ge`.
    ///
    /// This is the diagnosis and `warp_top` is the correction — separate on purpose. `engine.py`
    /// reads *this* field, not `warp_top`, when it retires a checkpoint timed from a contradicted
    /// instant.
    #[serde(default)]
    pub top_late_by: Option<i64>,

    /// tau at frame 0 — negative when the clip rolls early.
    pub tau_start: f64,
    /// tau at the last frame.
    pub tau_end: f64,
}

crate::validated!(ClipAlignment);

impl ClipAlignment {
    /// The frame tau=1 resolves to: the override when there is one, else the detected top.
    pub fn top(&self) -> i64 {
        self.warp_top.unwrap_or(self.anchors.top)
    }

    /// Whether a shared clock contradicted this top at all. `top_late_by` says how far.
    ///
    /// A method so no consumer writes `is_some()` against a field whose `None` means "nothing could
    /// decide" rather than "nothing was wrong".
    pub fn top_is_late(&self) -> bool {
        self.top_late_by.is_some()
    }
}

impl Validate for ClipAlignment {
    fn validate(&self) -> Result<(), ContractError> {
        ge("ClipAlignment.warp_motion_start", self.warp_motion_start, 0)?;
        ge("ClipAlignment.warp_top", self.warp_top, 0)?;
        gt("ClipAlignment.top_late_by", self.top_late_by, 0)?;
        nested("ClipAlignment.anchors", Some(&self.anchors))
    }
}

/// The correspondence between two clips of one swing, plus how much to trust it.
///
/// `a` and `b` are `None` only when the swing could not be aligned at all, which is reported rather
/// than raised: a detector that fails should say so (ADR-013).
///
/// **Serialization is hand-written**, because `quality_summary` is a `@computed_field` on the Python
/// side and is therefore *output*, not a convenience (ADR-032 §4's second named validator). Serde
/// has no computed-field attribute, so the choice is a real [`Serialize`] impl or a stored field
/// that would have to be kept in step with `quality` by every writer. The reason it is serialized at
/// all is that the alternative is every consumer keeping its own copy of the enum-to-sentence
/// mapping — and the results page, which had exactly that job, instead printed the bare enum value
/// at a viewer for two milestones.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(remote = "Self")]
pub struct SwingAlignment {
    #[serde(default)]
    pub a: Option<ClipAlignment>,
    #[serde(default)]
    pub b: Option<ClipAlignment>,

    #[serde(default = "SwingAlignment::default_quality")]
    pub quality: AlignmentQuality,

    /// Why the quality is what it is, in order of discovery. A bare enum says the alignment
    /// degraded; these say what to fix.
    #[serde(default)]
    pub notes: Vec<String>,

    /// The tau range both clips actually cover. Rendering outside it would mean holding one panel
    /// frozen while the other moves, which looks like a sync failure and is really just one phone
    /// that started rolling later.
    #[serde(default)]
    pub overlap: Option<(f64, f64)>,
}

impl<'de> Deserialize<'de> for SwingAlignment {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = SwingAlignment::deserialize(deserializer)?;
        Validate::validate(&value).map_err(serde::de::Error::custom)?;
        Ok(value)
    }
}

impl Serialize for SwingAlignment {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut out = serializer.serialize_struct("SwingAlignment", 6)?;
        out.serialize_field("a", &self.a)?;
        out.serialize_field("b", &self.b)?;
        out.serialize_field("quality", &self.quality)?;
        out.serialize_field("notes", &self.notes)?;
        out.serialize_field("overlap", &self.overlap)?;
        out.serialize_field("quality_summary", self.quality_summary())?;
        out.end()
    }
}

impl SwingAlignment {
    fn default_quality() -> AlignmentQuality {
        AlignmentQuality::Unaligned
    }

    /// `quality.summary()`, carried in the serialized payload.
    pub fn quality_summary(&self) -> &'static str {
        self.quality.summary()
    }
}

impl Validate for SwingAlignment {
    fn validate(&self) -> Result<(), ContractError> {
        nested("SwingAlignment.a", self.a.as_ref())?;
        nested("SwingAlignment.b", self.b.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn anchors(motion_start: i64, top: i64, impact: i64) -> String {
        format!(r#"{{"motion_start":{motion_start},"top":{top},"impact":{impact}}}"#)
    }

    #[test]
    fn the_quality_wire_names_are_the_python_values() {
        let pairs = [
            (AlignmentQuality::Synchronized, "synchronized"),
            (AlignmentQuality::Full, "full"),
            (AlignmentQuality::TopImpact, "top_impact"),
            (AlignmentQuality::ImpactOnly, "impact_only"),
            (AlignmentQuality::Unaligned, "unaligned"),
        ];
        for (variant, wire) in pairs {
            assert_eq!(
                serde_json::to_string(&variant).unwrap(),
                format!("\"{wire}\"")
            );
        }
    }

    /// The ordering rule is strict on one side and not the other, and both halves are load-bearing.
    #[test]
    fn a_clip_that_opens_mid_takeaway_is_accepted_and_a_zero_downswing_is_not() {
        assert!(serde_json::from_str::<SwingAnchors>(&anchors(10, 10, 20)).is_ok());
        assert!(serde_json::from_str::<SwingAnchors>(&anchors(11, 10, 20)).is_err());
        assert!(serde_json::from_str::<SwingAnchors>(&anchors(0, 10, 10)).is_err());
        assert!(serde_json::from_str::<SwingAnchors>(&anchors(0, 10, 11)).is_ok());
    }

    #[test]
    fn motion_start_detected_defaults_true_and_impact_measured_defaults_false() {
        let parsed: SwingAnchors = serde_json::from_str(&anchors(0, 10, 20)).unwrap();
        assert!(parsed.motion_start_detected);
        assert!(!parsed.impact_measured);
    }

    /// `None` means nothing could arbitrate the tops; zero would mean they agreed, which is a
    /// different answer and not one this field is allowed to carry.
    #[test]
    fn top_late_by_refuses_zero() {
        let with_zero = format!(
            r#"{{"anchors":{},"warp_motion_start":0,"top_late_by":0,"tau_start":0.0,"tau_end":2.0}}"#,
            anchors(0, 10, 20)
        );
        assert!(serde_json::from_str::<ClipAlignment>(&with_zero).is_err());
    }

    #[test]
    fn quality_summary_is_written_and_is_not_read_back() {
        let alignment = SwingAlignment {
            a: None,
            b: None,
            quality: AlignmentQuality::TopImpact,
            notes: vec![],
            overlap: None,
        };
        let json = serde_json::to_value(&alignment).unwrap();
        assert_eq!(json["quality_summary"], "aligned on top and impact");
        // Round-tripping it back must not need the computed key, and must not be confused by it.
        let back: SwingAlignment = serde_json::from_value(json).unwrap();
        assert_eq!(back, alignment);
    }

    #[test]
    fn overlap_is_a_pair_not_a_list() {
        let json = r#"{"overlap":[-0.5,2.0]}"#;
        let parsed: SwingAlignment = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.overlap, Some((-0.5, 2.0)));
        assert_eq!(
            serde_json::to_value(&parsed).unwrap()["overlap"],
            serde_json::json!([-0.5, 2.0])
        );
    }
}
