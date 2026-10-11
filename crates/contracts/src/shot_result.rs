//! The analysed result for one shot, written as `shot_analysis.json`. [M35 P1]
//!
//! **No Python twin, and none is coming.** ADR-034 §1 made the shot the unit, with mechanics a
//! separate panel; a photo of the launch monitor's screen with no face-on clip beside it is a whole
//! entry, and frozen Python never analyses one (ADR-035 clause 4). So this shape is Rust's alone, as
//! [`crate::capability`] is, and its schema, `spec/schemas/shot_result.schema.json`, is written by
//! hand for M38's Dart side and held to this struct by `tests/schemas.rs`.
//!
//! **Never carried in a [`SwingResult`](crate::swing::SwingResult), so never in `analysis.json`.**
//! A shot's refusals include the photo-side two,
//! [`PrintedBlank`](crate::unscored::UnscoredReason::PrintedBlank) and
//! [`Misread`](crate::unscored::UnscoredReason::Misread), and frozen Python reads `analysis.json`
//! with a closed `UnscoredReason`: one of them there would make the artifact unreadable to every
//! frozen reader. They land here instead. The rejected alternative was a `SwingResult` with no
//! phases and no checkpoints. It has to carry a required `overall_score` that a photo has none of,
//! and its `unscored` would then be the field frozen Python refuses, so it is the wrong shape twice.
//!
//! Its producer is `analysis::engine::analyze_shot`, the M35 plan's call 2, since M35 P2, and
//! `crates/analysis/tests/shot.rs` holds what it measures to the committed corpus. Nothing writes
//! one to disk before M35 P8's writer.

use serde::{Deserialize, Serialize};

use crate::intent::PracticeGoal;
use crate::shot::ShotData;
use crate::swing::Measurement;
use crate::unscored::UnscoredCheckpoint;
use crate::{each, nested, ContractError, Validate};

/// One shot, measured and flown from what its screen printed (the M35 plan's call 1).
///
/// The keys are written in this order, which is also the order a reader wants them: which entry,
/// which engine and photo, under what intent, then the shot and what came of it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Self")]
pub struct ShotResult {
    /// The entry this describes. A photo with no clip is still a swing directory with a manifest,
    /// so it is named the way a swing is.
    pub swing_id: String,
    pub session_id: String,

    /// Which generation of the engine produced this ([`crate::swing::ANALYSIS_VERSION`]). **Defaults
    /// to 0, for [`SwingBundleResult`](crate::swing::SwingBundleResult)'s reason**: a file that
    /// lacks the key must read as older than every engine, so the corpus excludes it as outdated
    /// rather than pooling a number nobody can date. The corpus applies
    /// [`crate::swing::COMPARABLE_FROM`] to it exactly as to a swing.
    #[serde(default)]
    pub analysis_version: i64,

    /// Content hash of the photo this was computed from. It is the entry's staleness check, the
    /// manifest's photo hash compared against it, which is why Rust never writes Python's
    /// `analysis.state.json` beside a shot. `None` reads as stale.
    #[serde(default)]
    pub photo_sha256: Option<String>,

    /// The goal the shot was analysed under. A record of the analysis rather than the shot's intent
    /// of record, which is the manifest's and can be changed after this was written; no number
    /// below depends on it, because every mode measures a shot alike (the M35 plan's decision 1).
    #[serde(default)]
    pub intent: PracticeGoal,

    /// The shot as stored, `shown_only` fields included, because the phone displays them. What was
    /// *measured* from it is narrower: see `measurements`.
    pub shot: ShotData,

    /// The shot's rows, then the flight's, measured over the **analysed view**: the shot with every
    /// field outside what the device printed and analysis may use set to `None` (ADR-034 §2). So a
    /// field the screen did not print, or printed only to be shown, produces nothing here.
    #[serde(default)]
    pub measurements: Vec<Measurement>,

    /// What could not be produced, named and never guessed (ADR-010 §2): the photo-side refusals in
    /// the device's declared tile order, then the flight's — the order `analyze_swing` uses,
    /// checkpoints then flight. A photo-side entry's `name` is the `ShotData` field (`spin_rate`),
    /// the vocabulary `devices.json` and `fields_present` speak.
    #[serde(default)]
    pub unscored: Vec<UnscoredCheckpoint>,

    /// Everything that degraded, in order of discovery: ADR-014's needs-review note, then what the
    /// shell appends, such as a handedness or loft gap in the shot's own words.
    #[serde(default)]
    pub notes: Vec<String>,
}

crate::validated!(ShotResult);

impl Validate for ShotResult {
    fn validate(&self) -> Result<(), ContractError> {
        nested("ShotResult.intent", Some(&self.intent))?;
        nested("ShotResult.shot", Some(&self.shot))?;
        each("ShotResult.measurements", &self.measurements)?;
        each("ShotResult.unscored", &self.unscored)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intent::{ClubCategory, PracticeMode, TargetShape};
    use crate::unscored::UnscoredReason;
    use serde_json::{json, Value};

    /// A shot read off an HD Golf photo, with one tile blank and one misread, refused in
    /// `screen::tile_refusals`' words (M35 P3): a direction word with no digits gives no number.
    /// Every key spelled out, so the round trip has nothing to default.
    fn every_key() -> Value {
        json!({
            "swing_id": "3",
            "session_id": "2026-10-09",
            "analysis_version": 17,
            "photo_sha256": "ab12",
            "intent": {
                "mode": "shot_shaping",
                "target_shape": "draw",
                "club": "mid_iron",
                "focus_checkpoint": null
            },
            "shot": {
                "shot_id": "s1",
                "session_id": "2026-10-09",
                "timestamp": "2026-10-09T12:00:00Z",
                "source": "screen",
                "ball_speed": 112.5,
                "launch_angle": 17.2,
                "carry_distance": 161.0,
                "provenance": {
                    "device": "hd_golf",
                    "parse_confidence": 0.9,
                    "raw_fields": {"Spin": "", "Spin Axis": "R"},
                    "parser_version": 2,
                    "fields_present": ["ball_speed", "launch_angle", "carry_distance",
                                       "spin_rate", "spin_axis"]
                }
            },
            "measurements": [{
                "name": "shot_ball_speed",
                "value": 112.5,
                "unit": "mph",
                "source": "launch_monitor:hd_golf",
                "detail": "as printed"
            }],
            "unscored": [
                {"name": "spin_rate", "reason": "printed_blank",
                 "detail": "Spin: nothing under the label"},
                {"name": "spin_axis", "reason": "misread",
                 "detail": "Spin Axis: 'R' under the label, and no value could be read from it"}
            ],
            "notes": ["a note"]
        })
    }

    /// Every key that went in comes back with the value it went in with, after the nested shot has
    /// filled in its own defaults: the shot is `ShotData`'s to round-trip, and `tests/round_trip.rs`
    /// holds that against every committed vector.
    #[test]
    fn a_shot_result_round_trips() {
        let input = every_key();
        let result: ShotResult = serde_json::from_value(input.clone()).expect("a shot result");
        let back = serde_json::to_value(&result).expect("serialize");
        for (key, value) in input.as_object().expect("an object") {
            if key == "shot" {
                let shot: ShotData = serde_json::from_value(value.clone()).expect("a shot");
                assert_eq!(back[key], serde_json::to_value(shot).unwrap(), "{key}");
            } else {
                assert_eq!(&back[key], value, "{key}");
            }
        }
        assert_eq!(
            back.as_object().unwrap().len(),
            input.as_object().unwrap().len(),
            "a key came back that did not go in"
        );
        assert_eq!(result.intent.mode, PracticeMode::ShotShaping);
        assert_eq!(result.intent.target_shape, Some(TargetShape::Draw));
        assert_eq!(result.intent.club, ClubCategory::MidIron);
    }

    /// The other half: a file holding only the required keys reads, and every key it lacked comes
    /// back at its default — the optional one as an explicit `null`, never as an absent key, so a
    /// written `shot_analysis.json` always carries the whole shape.
    #[test]
    fn an_absent_key_comes_back_at_its_default_and_a_null_stays_null() {
        let minimal = json!({
            "swing_id": "3",
            "session_id": "2026-10-09",
            "shot": every_key()["shot"].clone(),
        });
        let result: ShotResult = serde_json::from_value(minimal).expect("the required keys only");
        let back = serde_json::to_value(&result).expect("serialize");
        assert_eq!(back["analysis_version"], json!(0));
        assert_eq!(back["photo_sha256"], Value::Null);
        assert!(back.as_object().unwrap().contains_key("photo_sha256"));
        assert_eq!(
            back["intent"],
            serde_json::to_value(PracticeGoal::default()).unwrap()
        );
        for list in ["measurements", "unscored", "notes"] {
            assert_eq!(back[list], json!([]), "{list}");
        }

        // An explicit `null` reads as the absent key does, and is written back as `null`.
        let mut spelled = every_key();
        spelled["photo_sha256"] = Value::Null;
        let result: ShotResult = serde_json::from_value(spelled).expect("a null photo hash");
        assert_eq!(result.photo_sha256, None);
        assert_eq!(
            serde_json::to_value(&result).unwrap()["photo_sha256"],
            Value::Null
        );
    }

    /// An analysis version below the oldest comparable one, defaulted or written, is a number on
    /// disk rather than a refusal: excluding it as outdated is the corpus's call, made with the
    /// count it reports, and refusing the file here would make it vanish instead.
    #[test]
    fn a_version_zero_result_still_reads() {
        let mut old = every_key();
        old["analysis_version"] = json!(0);
        let result: ShotResult = serde_json::from_value(old).expect("a version-zero result");
        assert_eq!(result.analysis_version, 0);
    }

    /// The photo-side reasons are what this shape exists to carry, so one holding them must
    /// validate, both through `Deserialize` and when assembled in code.
    #[test]
    fn a_shot_result_holding_the_photo_side_reasons_validates() {
        let result: ShotResult = serde_json::from_value(every_key()).expect("a shot result");
        let reasons: Vec<UnscoredReason> = result.unscored.iter().map(|u| u.reason).collect();
        assert_eq!(
            reasons,
            [UnscoredReason::PrintedBlank, UnscoredReason::Misread]
        );
        assert_eq!(result.validate(), Ok(()));
    }

    /// The bounds reach through the tree, so a malformed shot inside a result is refused as the
    /// result, naming the route to the bad value.
    #[test]
    fn a_bad_shot_fails_the_result_that_holds_it() {
        let mut bad = every_key();
        bad["shot"]["provenance"]["parse_confidence"] = json!(1.5);
        let err = serde_json::from_value::<ShotResult>(bad).unwrap_err();
        assert!(
            err.to_string().contains("ShotProvenance.parse_confidence"),
            "{err}"
        );

        let mut assembled: ShotResult = serde_json::from_value(every_key()).unwrap();
        assembled.shot.provenance.as_mut().unwrap().parse_confidence = -0.1;
        let err = assembled.validate().unwrap_err();
        assert_eq!(
            err.field,
            "ShotResult.shot -> ShotData.provenance -> ShotProvenance.parse_confidence"
        );
    }
}
