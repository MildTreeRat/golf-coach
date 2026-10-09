//! The launch-monitor screen reader: located OCR text in, a parsed shot out. The Rust half of
//! `src/golf_coach/launch_monitor/screen/`. [M34 P3]
//!
//! ADR-034 makes a photo of the simulator screen the product's unit, so this is the code every shot
//! passes through on the phone (M38), and on the laptop once the lab is ported (M29). OCR itself is
//! not here: the phone has VisionKit and the lab gets `ort` in M29, so this crate starts where both
//! of them stop, at a list of [`TextBox`]es, exactly where the frozen parser starts at its
//! `TextRecognizer` port.
//!
//! # The order it is built in
//!
//! §M34's *record, port, then change*. What exists today is recorded from frozen Python and ported
//! **faithfully** first, so that every vector the frozen parser answers is passed before anything
//! about the answer changes; only then do the tie rule and the `Impact Position V` tile land, gated
//! by hand-worked vectors (`spec/vectors/screen/hand/`). M34 P3 built the half with no parser in it,
//! P5 the parser, P6 the checks and the record, P8 the change, and P9 the ranges for M32's fields
//! and the golfer's view of the warnings:
//!
//! - [`profile`] — the device profiles, and the label matcher every later step locates tiles with;
//! - [`difflib`] — CPython's `SequenceMatcher.ratio`, which that matcher scores with, reproduced to
//!   the bit;
//! - [`orient`] — `label_ratio`, the fraction of a profile's labels a set of boxes carries;
//! - [`parser`] — `parse_screen`: labels located by the tie rule, then every tile read as the frozen
//!   parser reads it;
//! - [`validate`] — `validate_parse`, the physics cross-checks that decide whether a read is
//!   trusted;
//! - [`read`] — all of it composed into a stamped `ShotData`, `import_screen` from its pixels on.
//!   This is the entry point: M38's phone calls it with VisionKit's boxes, and `golf-core
//!   parse-screen` is its stdin-to-stdout seam;
//! - [`golfer_warnings`] — a record's warnings as a golfer is shown them, which every Rust surface
//!   that shows them calls (M34 P9). It lives beside the one line that writes what it drops, and is
//!   re-exported here because a surface showing a shot should not need to know the parser's layout.
//!
//! The faithful rules the change replaced lived behind a `frozen` module from P8, so the
//! Python-recorded vectors went on gating the code both readers shared, until P10 re-recorded those
//! vectors through this reader (`spec/declarations/screen-v1.json`) and deleted it. Every screen
//! vector now answers to [`read`].
//!
//! # The profile is forked, and that gives up one copy on disk for one file
//!
//! `profiles.json` beside this crate's `Cargo.toml` is **a fork** of the frozen parser's package data,
//! not a second reader of it (the M34 plan's decision 1). The frozen lab may not change (ADR-035
//! clause 4), and the V tile this crate gains would be new behaviour there: every bay shot would gain
//! a warning and lose about 0.02 of `parse_confidence`. So ADR-032 §5's one copy on disk is given
//! up for this file until M40 deletes the Python one, and `tests/profile_fork.rs` holds the two
//! equal except a delta it declares by name.

pub mod difflib;
pub mod orient;
pub mod parser;
pub mod profile;
pub mod validate;

pub use crate::parser::golfer_warnings;

use std::collections::BTreeMap;

use contracts::shot::{ShotData, ShotProvenance, ShotSource, SCREEN_PARSER_VERSION};
use contracts::Timestamp;
use serde::Deserialize;
use serde_json::{json, Map, Value};

use crate::parser::{parse_screen, ParsedShot};
use crate::profile::{load_profile, UnknownProfile};
use crate::validate::validate_parse;

/// One run of recognized text and where it sat on the screen. `recognizer.py`'s `TextBox`.
///
/// Pixels in the rectified screen image, origin top-left. The parser only ever compares boxes with
/// each other, so the absolute scale is irrelevant: a 900 px crop and a 4,000 px one parse the same.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct TextBox {
    pub text: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    /// The dataclass's default of `1.0`, for a recognizer that reports none.
    #[serde(default = "full_confidence")]
    pub confidence: f64,
}

fn full_confidence() -> f64 {
    1.0
}

impl TextBox {
    pub fn right(&self) -> f64 {
        self.x + self.width
    }

    pub fn bottom(&self) -> f64 {
        self.y + self.height
    }

    /// `x + width / 2`, the dataclass's two operations in its order, so the same bits. Not
    /// `(x + right) / 2`, which rounds at a different step, and the parser buckets on these.
    pub fn center_x(&self) -> f64 {
        self.x + self.width / 2.0
    }

    pub fn center_y(&self) -> f64 {
        self.y + self.height / 2.0
    }
}

/// One screen to read: `import_screen`'s arguments from `prepare_screen`'s answer on, and a screen
/// vector's `input` (the M34 plan's call 1).
///
/// `deny_unknown_fields` because this is an argument list, not a record: a misspelt `notes` would
/// otherwise read as no notes, and the warnings would quietly lose the line that explains the rest.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScreenInput {
    /// The profile to read with, by device name.
    pub device: String,
    pub boxes: Vec<TextBox>,
    /// What preprocessing noticed (`screen outline not found - …`), which goes *first* in the
    /// warnings because it explains the rest. The lab's `prepare_screen` writes them; the phone
    /// passes VisionKit's, or none.
    #[serde(default)]
    pub notes: Vec<String>,
    pub shot_id: String,
    pub session_id: String,
    /// Read as pydantic reads a `datetime`, and written to the record in pydantic's spelling — what
    /// `import_screen`'s `ShotData(timestamp=…)` does. A naive one is refused here where frozen
    /// Python takes it (`contracts::time`'s named divergence), so the phone sends an offset.
    pub timestamp: Timestamp,
    #[serde(default)]
    pub image_sha256: Option<String>,
    #[serde(default)]
    pub image_path: Option<String>,
    /// Below this a read that passed every check is still flagged for review. Required, as it is in
    /// `import_screen`: the threshold is the caller's (`settings.ocr_min_confidence` in the lab), and
    /// a default here would be a second copy of that setting.
    pub min_confidence: f64,
}

/// One screen's boxes to the `ShotData` they read as, or `None` where nothing was read.
///
/// `import_screen` from its `prepare_screen` call on, the half that never touches a pixel: parse,
/// the notes prepended, validate, and the record. `None` is exactly where `import_screen` returns
/// `failed`, a parse with no value in it: an empty shot is not stored, because a wrong number is
/// worse than a missing one (ADR-014's whole thesis) and a shot of nothing is a wrong record.
///
/// **An unknown device is an error, not a failed read.** The M34 plan wrote this as returning a bare
/// `Option`; that has no answer for a device no profile names, where Python's `load_profile` raises
/// before `import_screen` is reached. A photo that read as nothing and a caller that named a
/// profile that does not exist are different mistakes, and only the first is the photo's.
///
/// **Every record is stamped** (M34 P8): `parser_version` is [`SCREEN_PARSER_VERSION`] and
/// `fields_present` the tiles the parse located, so a stored shot can say which parser read it and
/// what its screen carried. Frozen Python's records carry neither (`parser_version` 0 and no
/// `fields_present`), so a stored shot says which of the two readers wrote it.
pub fn read(input: &ScreenInput) -> Result<Option<ShotData>, UnknownProfile> {
    let profile = load_profile(&input.device)?;
    Ok(record(parse_screen(&input.boxes, profile), input))
}

/// A parse to the record `import_screen` would store: the notes put first, validation, and `None`
/// for a read of nothing.
fn record(mut parsed: ParsedShot, input: &ScreenInput) -> Option<ShotData> {
    // `parsed.warnings[:0] = prepared.notes`, before validation, so the checks' lines come last.
    parsed.warnings.splice(0..0, input.notes.iter().cloned());
    let parsed = validate_parse(parsed, input.min_confidence);
    if parsed.is_empty() {
        return None;
    }
    Some(to_shot_data(&parsed, input))
}

/// `parser.py::to_shot_data`: the parse as a record, with its audit trail in `provenance`.
///
/// **The values reach `ShotData` through serde, by key, and that is the port of `**parsed.values`.**
/// Pydantic reads keyword arguments by name, and serde reads a map by name, so `ShotData`'s own field
/// list is the only list: a new field and a profile target for it need no edit here.
///
/// The two part only where a profile is wrong, which is measured rather than assumed. A target
/// `ShotData` does not have is dropped by both. A number for a text field is refused by both. Words
/// for a number field are refused by both unless they look like a number (`'1.5'`), which pydantic
/// coerces and serde refuses. A target that is also an identity key is Python's `got multiple values
/// for keyword argument`, and the `assert!` below. None of these is reachable from a shipped profile,
/// because `every_shipped_target_is_a_shot_data_field_of_its_kind` holds every target to a field of
/// its kind, and that is also what makes the `expect`s unreachable. The values cross as
/// `serde_json` numbers, which carry an `f64`'s bits unchanged.
fn to_shot_data(parsed: &ParsedShot, input: &ScreenInput) -> ShotData {
    let provenance = ShotProvenance {
        device: parsed.device.clone(),
        // `round(parsed.confidence, 3)`, decimal and half-even, which a scaled `f64::round` is not.
        parse_confidence: pyfmt::round_to(parsed.confidence, 3),
        needs_review: parsed.needs_review,
        warnings: parsed.warnings.clone(),
        image_sha256: input.image_sha256.clone(),
        image_path: input.image_path.clone(),
        raw_fields: parsed
            .raw_fields
            .iter()
            .map(|(label, text)| (label.to_string(), text.clone()))
            .collect(),
        // M32's three (the M34 plan's call 9). `corrections` is M38's to write, after a person has
        // reviewed the parse, so a read never has any.
        parser_version: SCREEN_PARSER_VERSION,
        fields_present: Some(parsed.fields_present.clone()),
        corrections: BTreeMap::new(),
    };

    let mut record: Map<String, Value> = parsed
        .values
        .iter()
        .map(|(target, value)| {
            let value = serde_json::to_value(value).expect("a number or a string serializes");
            (target.to_string(), value)
        })
        .collect();
    let identity = [
        ("shot_id", json!(input.shot_id)),
        ("session_id", json!(input.session_id)),
        ("timestamp", json!(input.timestamp)),
        ("source", json!(ShotSource::Screen)),
        ("provenance", json!(provenance)),
    ];
    for (key, value) in identity {
        let clash = record.insert(key.to_string(), value);
        assert!(
            clash.is_none(),
            "a profile target names `ShotData.{key}`, which the read sets itself"
        );
    }
    serde_json::from_value(Value::Object(record))
        .expect("every shipped profile's targets are ShotData fields of their kind")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::FieldValue;
    use crate::profile::FieldKind;

    #[test]
    fn a_box_measures_itself_as_the_dataclass_does() {
        let text_box: TextBox = serde_json::from_str(
            r#"{"text": "Carry", "x": 10.0, "y": 20.0, "width": 7.0, "height": 3.0}"#,
        )
        .unwrap();
        assert_eq!(text_box.confidence, 1.0);
        assert_eq!(text_box.right(), 17.0);
        assert_eq!(text_box.bottom(), 23.0);
        assert_eq!(text_box.center_x(), 13.5);
        assert_eq!(text_box.center_y(), 21.5);
    }

    fn an_input(device: &str) -> ScreenInput {
        ScreenInput {
            device: device.to_string(),
            boxes: Vec::new(),
            notes: Vec::new(),
            shot_id: "a-shot".to_string(),
            session_id: "a-session".to_string(),
            timestamp: "2026-08-04T12:00:00Z"
                .parse()
                .expect("a pydantic timestamp"),
            image_sha256: None,
            image_path: None,
            min_confidence: 0.6,
        }
    }

    /// A parse with every stored field of `profile` read, each as a value of its tile's kind.
    fn every_field_read(profile: &profile::DeviceProfile) -> ParsedShot {
        ParsedShot {
            device: profile.device.clone(),
            values: profile
                .stored_fields()
                .iter()
                .map(|field| {
                    let value = match field.kind {
                        FieldKind::Number => FieldValue::Number(1.5),
                        FieldKind::Text => FieldValue::Text("SLIGHT FADE".to_string()),
                    };
                    (
                        field.target.clone().expect("a stored field has a target"),
                        value,
                    )
                })
                .collect(),
            raw_fields: Default::default(),
            confidence: 0.5,
            warnings: Vec::new(),
            needs_review: false,
            fields_present: vec!["carry_distance".to_string()],
        }
    }

    /// `to_shot_data`'s doc rests on this: on every shipped profile, a value of each tile's kind
    /// lands on the `ShotData` key its target names, unchanged. A target `ShotData` lacks would come
    /// back `null`, one of the wrong kind or naming an identity key would panic, so all three
    /// mistakes fail here rather than on a golfer's photo.
    #[test]
    fn every_shipped_target_is_a_shot_data_field_of_its_kind() {
        for profile in profile::profiles().values() {
            let parsed = every_field_read(profile);
            let shot = to_shot_data(&parsed, &an_input(&profile.device));
            let shot = serde_json::to_value(shot).expect("a ShotData serializes");
            for (target, value) in parsed.values.iter() {
                let value = serde_json::to_value(value).expect("a value serializes");
                assert_eq!(shot[target], value, "{}: {target}", profile.device);
            }
        }
    }

    /// The shipping read stamps the parser's version and records what it located, as the parse
    /// listed it; it never corrects.
    #[test]
    fn the_shipping_read_stamps_its_version_and_what_it_located() {
        let profile = load_profile("hd_golf").expect("the shipped profile");
        let shot = to_shot_data(&every_field_read(profile), &an_input("hd_golf"));
        let provenance = shot.provenance.expect("a read shot carries its provenance");
        assert_eq!(provenance.parser_version, SCREEN_PARSER_VERSION);
        assert_eq!(
            provenance.fields_present,
            Some(vec!["carry_distance".to_string()])
        );
        assert!(provenance.corrections.is_empty());
    }

    /// `round(conf, 3)` on the two confidences where a scaled `f64::round` gets it wrong, neither
    /// reached by a vector: `0.0625` is an exact binary tie, which Python takes to the even
    /// `0.062` and half-away to `0.063`, and `0.0005` sits just above the half, which Python
    /// takes up to `0.001` and `round(x * 1000) / 1000` down to `0.0`. Frozen Python's answers.
    #[test]
    fn the_confidence_rounds_as_python_rounds() {
        let profile = load_profile("hd_golf").expect("the shipped profile");
        for (confidence, rounded) in [(0.0625, 0.062), (0.0005, 0.001)] {
            let mut parsed = every_field_read(profile);
            parsed.confidence = confidence;
            let shot = to_shot_data(&parsed, &an_input("hd_golf"));
            let provenance = shot.provenance.expect("a read shot carries its provenance");
            assert_eq!(provenance.parse_confidence.to_bits(), f64::to_bits(rounded));
        }
    }

    #[test]
    fn an_unknown_device_is_an_error_and_a_screen_of_nothing_is_no_shot() {
        let refused = read(&an_input("no_such_device")).expect_err("no profile names it");
        assert_eq!(refused.device, "no_such_device");
        assert_eq!(read(&an_input("hd_golf")), Ok(None));
    }

    #[test]
    fn a_misspelt_argument_is_refused_rather_than_defaulted() {
        let misspelt = json!({
            "device": "hd_golf", "boxes": [], "note": ["screen outline not found"],
            "shot_id": "a", "session_id": "b", "timestamp": "2026-08-04T12:00:00Z",
            "min_confidence": 0.6
        });
        assert!(serde_json::from_value::<ScreenInput>(misspelt).is_err());
    }
}
