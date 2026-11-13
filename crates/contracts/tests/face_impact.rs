//! `contracts::face_impact`: one answer to *where on the face*, each method declaring its axes.
//! [M46 P5]
//!
//! **No Python twin and no vector**: nothing makes a `FaceImpact` before the tape reader (P7 to
//! P13), so this holds the shape to what it can be held to on its own (the M46 plan's C2, ADR-037
//! clause 1). Each method sees the axes it declares and no more, a value on an axis the method
//! cannot see is refused both when assembled in code and when read from JSON, a non-finite offset is
//! refused, and the wire names are pinned, because the drill store (P17) writes them to disk and the
//! app (P22 on) reads them.

use contracts::face_impact::{FaceImpact, ImpactAxes, ImpactMethod, TAPE_READER_VERSION};
use contracts::Validate;
use serde_json::{json, Value};

fn tape(h: Option<f64>, v: Option<f64>) -> FaceImpact {
    FaceImpact {
        offset_h_mm: h,
        offset_v_mm: v,
        method: ImpactMethod::StrikeTape,
        method_version: TAPE_READER_VERSION,
    }
}

/// A replay reading. `method_version` is 0 because the replay reader has no version key, which the
/// field's doc names as a gap and nothing here checks.
fn replay(h: Option<f64>, v: Option<f64>) -> FaceImpact {
    FaceImpact {
        offset_h_mm: h,
        offset_v_mm: v,
        method: ImpactMethod::OverheadReplay,
        method_version: 0,
    }
}

#[test]
fn each_method_declares_the_axes_it_can_see() {
    assert_eq!(
        ImpactMethod::OverheadReplay.axes(),
        ImpactAxes {
            heel_toe: true,
            high_low: false
        }
    );
    assert_eq!(
        ImpactMethod::StrikeTape.axes(),
        ImpactAxes {
            heel_toe: true,
            high_low: true
        }
    );
}

#[test]
fn a_replay_reading_holding_a_high_low_value_is_refused() {
    let err = replay(Some(-6.0), Some(2.5)).validate().unwrap_err();
    assert_eq!(err.field, "FaceImpact.offset_v_mm");
    assert_eq!(
        err.to_string(),
        "FaceImpact.offset_v_mm: overhead_replay cannot see high to low, so it holds no value \
         there (got 2.5)"
    );
    // Not even face centre: 0 is a claim, and the replay cannot make it.
    assert!(replay(Some(-6.0), Some(0.0)).validate().is_err());
    // What the replay does read passes.
    assert_eq!(replay(Some(-6.0), None).validate(), Ok(()));
}

#[test]
fn a_replay_reading_holding_a_high_low_value_is_refused_when_read_from_json() {
    let wire = json!({
        "offset_h_mm": -6.0, "offset_v_mm": 2.5,
        "method": "overhead_replay", "method_version": 0,
    });
    let err = serde_json::from_value::<FaceImpact>(wire).unwrap_err();
    assert!(
        err.to_string().contains("FaceImpact.offset_v_mm"),
        "the refusal names the field: {err}"
    );
}

#[test]
fn a_tape_reading_holding_both_axes_passes() {
    let reading = tape(Some(8.4), Some(-2.1));
    assert_eq!(reading.validate(), Ok(()));
    let back: FaceImpact = serde_json::from_value(serde_json::to_value(reading).unwrap()).unwrap();
    assert_eq!(back, reading);
}

#[test]
fn a_non_finite_offset_is_refused_on_either_axis() {
    let nan = tape(Some(f64::NAN), Some(1.0)).validate().unwrap_err();
    assert_eq!(nan.field, "FaceImpact.offset_h_mm");
    assert_eq!(
        nan.to_string(),
        "FaceImpact.offset_h_mm: NaN is not a finite number of millimetres"
    );
    let v = tape(Some(1.0), Some(f64::NAN)).validate().unwrap_err();
    assert_eq!(v.field, "FaceImpact.offset_v_mm");
    for infinite in [f64::INFINITY, f64::NEG_INFINITY] {
        assert!(tape(Some(infinite), None).validate().is_err());
        assert!(tape(None, Some(infinite)).validate().is_err());
    }
    // The replay's one axis is held to the same rule.
    assert!(replay(Some(f64::NAN), None).validate().is_err());
}

#[test]
fn the_wire_names_are_pinned() {
    assert_eq!(
        serde_json::to_value(ImpactMethod::OverheadReplay).unwrap(),
        json!("overhead_replay")
    );
    assert_eq!(
        serde_json::to_value(ImpactMethod::StrikeTape).unwrap(),
        json!("strike_tape")
    );
    assert_eq!(
        serde_json::to_value(ImpactMethod::StrikeTape.axes()).unwrap(),
        json!({"heel_toe": true, "high_low": true})
    );
    assert_eq!(
        serde_json::to_value(tape(Some(8.4), Some(-2.1))).unwrap(),
        json!({
            "offset_h_mm": 8.4, "offset_v_mm": -2.1,
            "method": "strike_tape", "method_version": 1,
        })
    );
    // And the other way, with every key spelled as it is written.
    let read: FaceImpact = serde_json::from_value(json!({
        "offset_h_mm": -3.0, "method": "overhead_replay", "method_version": 0,
    }))
    .unwrap();
    assert_eq!(read, replay(Some(-3.0), None));
}

#[test]
fn an_unread_axis_is_an_absent_key_and_a_null_still_reads_as_absent() {
    let written = serde_json::to_value(replay(Some(-3.0), None)).unwrap();
    let keys: Vec<&str> = written
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert!(
        !keys.contains(&"offset_v_mm"),
        "no null on the wire: {written}"
    );
    assert!(!written.to_string().contains("null"));

    let read: FaceImpact = serde_json::from_value(json!({
        "offset_h_mm": null, "offset_v_mm": null,
        "method": "strike_tape", "method_version": 1,
    }))
    .unwrap();
    assert_eq!(read, tape(None, None));
}

#[test]
fn an_unknown_method_is_refused() {
    let wire: Value = json!({"offset_h_mm": 1.0, "method": "foot_spray", "method_version": 1});
    assert!(serde_json::from_value::<FaceImpact>(wire).is_err());
}

/// The ledger starts at 1, and 0 is no entry (the constant's doc). Bumping it is a deliberate act
/// with a ledger line beside it, so this line moves with that line, in the same change.
#[test]
fn the_tape_reader_version_is_the_first_entry_on_its_ledger() {
    assert_eq!(TAPE_READER_VERSION, 1);
}
