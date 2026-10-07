//! [`screen::golfer_warnings`] on every warning the screen documents carry. [M34 P9]
//!
//! Each document's warnings are taken twice: as recorded in its `expected.shot`, and as the shipping
//! [`screen::read`] writes them for the same boxes. Until M34 P10 the recorded ones were frozen
//! Python's, and held the filter to the spelling frozen Python wrote, which is what [`NO_TILE_FOUND`]
//! claims to be. P10 re-recorded them through the shipping reader, the tie lines and the reference
//! photos' missing `Impact Position V` among them, so the two are now one set. [`SPELLED`] is still
//! frozen Python's spelling: the re-record replaced each warnings list whole, and every line the old
//! and new lists share, every `no tile found` line among them, is word for word the same (the M34
//! plan's P10 findings).
//!
//! The prefix is spelt out in [`SPELLED`] rather than read from the constant. A filter checked
//! against its own constant would pass whatever the constant said; checked against the text the
//! vectors hold, a constant edited away from it lets the line through and fails here.

mod common;

use serde_json::Value;

use screen::parser::NO_TILE_FOUND;
use screen::profile::load_profile;
use screen::{golfer_warnings, read, ScreenInput};

/// `parser.py`'s `f"no tile found for {field.label!r}"`, up to the label's opening quote.
const SPELLED: &str = "no tile found for '";

/// The sub-families whose documents are photos, as opposed to screens built for a test: the bay
/// photos the corpus holds and the two reference photos.
const PHOTOS: [&str; 2] = ["corpus", "reference"];

/// Every screen document, the hand-worked ones included, by `id`.
fn documents() -> Vec<(String, Value)> {
    let mut documents = common::every_document();
    documents.extend(
        common::sub_family(common::HAND)
            .into_iter()
            .map(|(stem, vector)| (format!("screen/{}/{stem}", common::HAND), vector)),
    );
    documents
}

fn strings(value: &Value) -> Vec<String> {
    value
        .as_array()
        .expect("warnings are a list")
        .iter()
        .map(|line| line.as_str().expect("a warning is a string").to_string())
        .collect()
}

/// The shipping reader's warnings for one document, or `None` where it reads no shot.
fn shipped(id: &str, vector: &Value) -> Option<Vec<String>> {
    let input: ScreenInput = serde_json::from_value(vector["input"].clone())
        .unwrap_or_else(|e| panic!("{id}: input: {e}"));
    let shot = read(&input).unwrap_or_else(|e| panic!("{id}: {e}"))?;
    Some(
        shot.provenance
            .expect("a read shot has provenance")
            .warnings,
    )
}

/// The recorded warnings for one document, or `None` where its expected shot is `null`.
fn recorded(vector: &Value) -> Option<Vec<String>> {
    let shot = &vector["expected"]["shot"];
    (!shot.is_null()).then(|| strings(&shot["provenance"]["warnings"]))
}

#[test]
fn the_spelling_is_frozen_pythons() {
    assert_eq!(format!("{NO_TILE_FOUND}'"), SPELLED);
}

/// The filter drops the `no tile found` lines and nothing else, keeping the rest in order, on every
/// list of warnings any document carries or the shipping reader writes.
#[test]
fn only_the_no_tile_lines_are_dropped_and_the_rest_keep_their_order() {
    let mut dropped = [0usize; 2];
    let mut ties_kept = 0;
    let mut lists = 0;
    for (id, vector) in documents() {
        for (source, warnings) in [recorded(&vector), shipped(&id, &vector)]
            .into_iter()
            .enumerate()
        {
            let Some(warnings) = warnings else { continue };
            lists += 1;
            let kept: Vec<String> = warnings
                .iter()
                .filter(|line| !line.starts_with(SPELLED))
                .cloned()
                .collect();
            assert_eq!(
                golfer_warnings(&warnings),
                kept,
                "{id}, {}",
                ["as recorded", "as the shipping reader writes them"][source]
            );
            dropped[source] += warnings.len() - kept.len();
            ties_kept += kept
                .iter()
                .filter(|line| line.contains(": label tie "))
                .count();
        }
    }
    // Not vacuous on either side: both sources carry the line, and the shipping one carries tie
    // lines, which must come through.
    assert!(lists > 0);
    assert!(
        dropped[0] > 0,
        "no recorded document carries a no-tile line"
    );
    assert!(dropped[1] > 0, "the shipping reader writes no no-tile line");
    assert!(ties_kept > 0, "no tie line reached the filter");
}

/// `golfer_warnings`' doc says that on every stored photo the line is the layout and never a missed
/// label: each bay photo lacks only `Bounce & Roll` and each reference photo only `Impact Position V`,
/// and in each the field it names is exactly what `fields_present` leaves out. Read by the shipping
/// parser, which is the one that knows the V tile.
#[test]
fn on_every_photo_the_dropped_line_is_a_tile_its_layout_lacks() {
    for family in PHOTOS {
        let lacks = match family {
            "corpus" => "Bounce & Roll",
            _ => "Impact Position V",
        };
        for (stem, vector) in common::sub_family(family) {
            let id = format!("screen/{family}/{stem}");
            let input: ScreenInput = serde_json::from_value(vector["input"].clone())
                .unwrap_or_else(|e| panic!("{id}: input: {e}"));
            let shot = read(&input)
                .unwrap_or_else(|e| panic!("{id}: {e}"))
                .unwrap_or_else(|| panic!("{id}: a photo reads as a shot"));
            let provenance = shot.provenance.expect("a read shot has provenance");

            let named: Vec<&str> = provenance
                .warnings
                .iter()
                .filter_map(|line| line.strip_prefix(SPELLED))
                .map(|rest| rest.strip_suffix('\'').expect("a quoted label"))
                .collect();
            assert_eq!(named, [lacks], "{id}");

            let present = provenance
                .fields_present
                .expect("the shipping read stamps it");
            let unlocated: Vec<&str> = load_profile(&input.device)
                .expect("a photo's device has a profile")
                .stored_fields()
                .into_iter()
                .filter(|field| {
                    let target = field.target.as_ref().expect("a stored field has a target");
                    !present.contains(target)
                })
                .map(|field| field.label.as_str())
                .collect();
            assert_eq!(
                unlocated, named,
                "{id}: the line and fields_present disagree"
            );
        }
    }
}

/// A made-up list, for the cases no document has: an empty list, a line that only mentions the
/// phrase, and two no-tile lines among others.
#[test]
fn the_filter_matches_a_prefix_and_keeps_order() {
    let lines = |texts: &[&str]| -> Vec<String> { texts.iter().map(|t| t.to_string()).collect() };
    assert!(golfer_warnings(&[]).is_empty());
    assert_eq!(
        golfer_warnings(&lines(&[
            "screen outline not found - parsing the photo uncropped",
            "no tile found for 'Bounce & Roll'",
            "Spin: no value text under the label",
            "no tile found for 'Impact Position V'",
            "Carry: the text said no tile found for 'Carry'",
        ])),
        lines(&[
            "screen outline not found - parsing the photo uncropped",
            "Spin: no value text under the label",
            "Carry: the text said no tile found for 'Carry'",
        ])
    );
}
