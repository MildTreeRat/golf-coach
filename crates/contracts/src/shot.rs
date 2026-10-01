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
    /// device is `provenance.device` where there is one and this where there is not
    /// ([`crate::capability::device_of`]) — so on a shot with no provenance this string reaches an
    /// artifact field `docs/CONFORMANCE.md` §3 compares exactly, and names the entry
    /// `devices.json` is searched for. [`crate::intent::ClubCategory::as_str`]'s argument for a
    /// hand-written table.
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

    // The three below are Rust's alone (M32). Frozen Python's `ShotProvenance` never gains them
    // (ADR-035 clause 4), and the frozen lab goes on writing provenance without them until M40 — so
    // each defaults, and a Python-shaped record reads as "not recorded" rather than failing.
    /// Which screen parser produced this record — an entry in [`SCREEN_PARSER_VERSION`]'s ledger.
    ///
    /// **0 means unstamped**, which is every shot frozen Python's parser wrote: that parser predates
    /// the ledger, so 0 is not a version of it but the absence of one. [`parse_is_current`] reads it.
    #[serde(default)]
    pub parser_version: i64,
    /// The `ShotData` field names whose tiles the parse *located*, blank or not.
    ///
    /// It is what separates "the screen left this blank" from "this layout has no such tile", which
    /// a `None` metric cannot say on its own: ADR-034 §2's printed set is the device's declared
    /// fields ∩ this. **`None` means not recorded**, not "nothing located" — every stored shot reads
    /// `None` until M29's Rust lab re-reads it, and M34's re-read over the committed boxes writes no
    /// `data/` (§M34). A `Vec` rather than a set because it is the parser's own list, kept as read.
    #[serde(default)]
    pub fields_present: Option<Vec<String>>,
    /// Field name -> text, for M38's review-and-edit of a parse. Empty until M38 writes one, and
    /// what an entry's value records is M38's to settle.
    ///
    /// Added in M32 rather than in M38 so that M38 does not force a second re-record of every
    /// committed vector for a key that is empty on all of them. A `BTreeMap` for `raw_fields`'
    /// reason above.
    #[serde(default)]
    pub corrections: BTreeMap<String, String>,
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
        )?;
        // Not a bound on the ledger's top: a record stamped by a parser newer than this build is a
        // newer record, and [`parse_is_current`] already reads it as current. Only a negative has
        // no meaning — 0 is "unstamped" and the ledger counts up from 1.
        ge("ShotProvenance.parser_version", self.parser_version, 0)
    }
}

/// The generation of the screen parser, stamped as [`ShotProvenance::parser_version`] on every shot
/// it reads. **Bump it whenever a re-read of the same photo could produce a different shot** — a
/// moved tile, a new field, a changed tie rule — so a stored shot can say whether re-reading its
/// photo would change it. [`parse_is_current`] is the reader.
///
/// It is a separate ledger from [`crate::swing::ANALYSIS_VERSION`] because the two age different
/// records: a parser bump stales a stored *shot* and leaves every analysis of it meaning what it
/// said, and an engine bump does the reverse.
///
/// **Nothing stamps it before M34.** 0 is not an entry (it is the unstamped default), and frozen
/// Python's parser never gains a version: it is the thing entry 1 replaces.
///
/// 1 (M34, not yet landed): the screen reader in Rust, `crates/screen`. Whether M34's faithful port
///   of the frozen parser and the tie rule it then changes are one version or two is M34's call,
///   and this entry is rewritten when it makes it.
pub const SCREEN_PARSER_VERSION: i64 = 1;

/// Whether `shot`'s parse is as new as this build's screen parser — `false` means re-reading its
/// photo could change it.
///
/// **A shot with no provenance is current**: a direct feed (R10, mock, import) was never parsed, so
/// there is no parse to redo, and calling it stale would send a caller looking for a photo that does
/// not exist. An unstamped screen shot (`parser_version` 0) is stale by construction, which is the
/// point — every one of them was read by the parser entry 1 replaces.
///
/// No caller until a Rust shot lookup exists: the lab's in M29 or the phone's store in M38,
/// whichever lands first (§M34).
pub fn parse_is_current(shot: &ShotData) -> bool {
    shot.provenance
        .as_ref()
        .is_none_or(|provenance| provenance.parser_version >= SCREEN_PARSER_VERSION)
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

    // Added in M32, in Rust only (ADR-035 clause 4): frozen Python's `ShotData` never gains these,
    // and the frozen lab writes shots without them until M40 — so every one defaults, and a stored
    // shot reads with each at `None`. No source fills any of them yet. Which devices may print
    // them is `capability`'s, not this struct's: an absent field here says nothing about whether the
    // screen had a tile for it (`ShotProvenance::fields_present` says that).
    /// degrees, + = up (hitting up on the ball)
    #[serde(default)]
    pub attack_angle: Option<f64>,
    /// degrees
    #[serde(default)]
    pub dynamic_loft: Option<f64>,
    /// inches, + = ahead of the ball
    #[serde(default)]
    pub low_point: Option<f64>,
    /// millimetres, + = toe
    #[serde(default)]
    pub impact_offset_h: Option<f64>,
    /// millimetres, + = high
    #[serde(default)]
    pub impact_offset_v: Option<f64>,
    /// The vertical partner of `impact_position`, in the source's own words — free text for that
    /// field's reason above.
    #[serde(default)]
    pub impact_position_v: Option<String>,
    /// yards, + = right — the ball's lateral distance off the target line at carry
    #[serde(default)]
    pub carry_offline: Option<f64>,

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

    /// A screen shot in exactly the shape frozen Python writes: none of M32's ten keys, a
    /// provenance object, and an explicit `null` where pydantic writes one.
    const PYTHON_SHAPED: &str = r#"{
        "shot_id": "2026-08-10-1", "session_id": "2026-08-10",
        "timestamp": "2026-08-10T01:38:46.828488Z", "source": "screen",
        "club_head_speed": 91.0, "ball_speed": 90.7, "impact_position": null,
        "provenance": {
            "device": "hd_golf", "parse_confidence": 0.925, "needs_review": false,
            "warnings": [], "image_sha256": null, "image_path": null,
            "raw_fields": {"Ball Speed": "90.7 mph"}
        }
    }"#;

    /// Every key M32 added, each at a value no default could produce.
    const ALL_TEN: &str = r#"{
        "shot_id": "s", "session_id": "d", "timestamp": "2026-10-01T00:00:00Z",
        "source": "screen",
        "attack_angle": -4.5, "dynamic_loft": 22.25, "low_point": 3.5,
        "impact_offset_h": -6.0, "impact_offset_v": 2.5, "impact_position_v": "HIGH",
        "carry_offline": -11.75,
        "provenance": {
            "device": "hd_golf", "parse_confidence": 1.0,
            "parser_version": 1, "fields_present": ["ball_speed", "attack_angle"],
            "corrections": {"ball_speed": "90.7"}
        }
    }"#;

    /// The frozen lab keeps writing [`PYTHON_SHAPED`] shots until M40, and every committed vector's
    /// `input.shot` is one, so this has to hold for as long as the two languages share `data/`. The
    /// reverse of the M31.5 plan's P1 finding 1 (Python reads a Rust-shaped shot by ignoring extras).
    #[test]
    fn a_python_shaped_shot_reads_with_every_new_key_at_its_default() {
        let shot: ShotData = serde_json::from_str(PYTHON_SHAPED).expect("a frozen-Python shot");
        assert_eq!(
            (
                shot.attack_angle,
                shot.dynamic_loft,
                shot.low_point,
                shot.impact_offset_h,
                shot.impact_offset_v,
                shot.carry_offline,
            ),
            (None, None, None, None, None, None)
        );
        assert_eq!(shot.impact_position_v, None);

        let provenance = shot.provenance.expect("the shot carries provenance");
        assert_eq!(provenance.parser_version, 0, "0 is unstamped");
        assert_eq!(provenance.fields_present, None, "None is not recorded");
        assert!(provenance.corrections.is_empty());
    }

    /// Read, written, read again: every one of the ten comes back with its value, and the written
    /// form spells each key under its own name.
    #[test]
    fn a_shot_carrying_all_ten_new_keys_round_trips() {
        let shot: ShotData = serde_json::from_str(ALL_TEN).expect("a shot with all ten keys");
        let written = serde_json::to_value(&shot).expect("serialize");
        let read_back: ShotData = serde_json::from_value(written.clone()).expect("read back");
        assert_eq!(read_back, shot);

        let original: serde_json::Value = serde_json::from_str(ALL_TEN).unwrap();
        for key in [
            "attack_angle",
            "dynamic_loft",
            "low_point",
            "impact_offset_h",
            "impact_offset_v",
            "impact_position_v",
            "carry_offline",
        ] {
            assert_eq!(written[key], original[key], "{key}");
        }
        for key in ["parser_version", "fields_present", "corrections"] {
            assert_eq!(
                written["provenance"][key], original["provenance"][key],
                "provenance.{key}"
            );
        }
    }

    #[test]
    fn a_negative_parser_version_is_refused() {
        let json = r#"{"device":"hd_golf","parse_confidence":1.0,"parser_version":-1}"#;
        assert!(serde_json::from_str::<ShotProvenance>(json).is_err());
        let zero = r#"{"device":"hd_golf","parse_confidence":1.0,"parser_version":0}"#;
        assert!(serde_json::from_str::<ShotProvenance>(zero).is_ok());
    }

    #[test]
    fn a_parse_is_current_when_stamped_by_this_parser_and_a_direct_feed_always_is() {
        let mut shot: ShotData = serde_json::from_str(ALL_TEN).unwrap();
        let stamp = |shot: &mut ShotData, version: i64| {
            shot.provenance.as_mut().unwrap().parser_version = version;
        };

        stamp(&mut shot, SCREEN_PARSER_VERSION);
        assert!(parse_is_current(&shot), "stamped by this parser");
        stamp(&mut shot, SCREEN_PARSER_VERSION + 1);
        assert!(parse_is_current(&shot), "stamped by a newer one");

        let unstamped: ShotData = serde_json::from_str(PYTHON_SHAPED).unwrap();
        assert!(
            !parse_is_current(&unstamped),
            "every shot frozen Python's parser wrote is stale by construction"
        );

        shot.provenance = None;
        shot.source = ShotSource::R10;
        assert!(
            parse_is_current(&shot),
            "a direct feed has no parse to redo"
        );
    }
}
