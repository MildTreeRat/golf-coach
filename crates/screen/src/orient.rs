//! The orientation vote: what fraction of a profile's labels a set of boxes carries.
//! `preprocess._label_ratio`. [M34 P3]
//!
//! Frozen Python OCRs a photo at each quarter turn until one scores well enough here, keeps the best
//! turn, and when even that one scores too low says so in the shot's warnings as a legibility note.
//! The phone orients its own photos, so the vote may never run in Rust. The number still does:
//! every corpus screen vector records it (the M34 plan's finding 7), so it is ported beside the
//! parser and gated with it.
//!
//! **It moves when the profile does.** The denominator is the profile's stored fields, so the fork's
//! `Impact Position V` tile (M34 P8) changes it on every photo, read or not. That is a declared change
//! at the re-record, not a regression.

use std::collections::BTreeSet;

use crate::profile::DeviceProfile;
use crate::TextBox;

/// Distinct stored labels some box matches, over the profile's stored fields; `0.0` for a profile
/// with none.
///
/// A box matching a layout-only tile (`Custom`) counts for nothing, and two boxes matching one
/// label count once. Both counts are exact integers, so the division is the one correctly rounded
/// operation Python's `/` is.
pub fn label_ratio(boxes: &[TextBox], profile: &DeviceProfile) -> f64 {
    let expected = profile.stored_fields().len();
    if expected == 0 {
        return 0.0;
    }
    let found: BTreeSet<&str> = boxes
        .iter()
        .filter_map(|text_box| profile.field_for(&text_box.text))
        .filter(|field| field.target.is_some())
        .map(|field| field.label.as_str())
        .collect();
    found.len() as f64 / expected as f64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::load_profile;

    fn boxes(texts: &[&str]) -> Vec<TextBox> {
        texts
            .iter()
            .map(|text| TextBox {
                text: (*text).to_string(),
                x: 0.0,
                y: 0.0,
                width: 1.0,
                height: 1.0,
                confidence: 1.0,
            })
            .collect()
    }

    /// `Carry` and `Carny` are one label, `Custom` is layout only and `12.3` is no label, so two of
    /// the stored fields are found. Frozen Python agreed on the profile as forked.
    #[test]
    fn a_label_counts_once_and_a_layout_tile_not_at_all() {
        let profile = load_profile("hd_golf").unwrap();
        let stored = profile.stored_fields().len() as f64;
        let found = label_ratio(
            &boxes(&["Carry", "Carny", "Custom", "Club Speed", "12.3"]),
            profile,
        );
        assert_eq!(found, 2.0 / stored);
        assert_eq!(label_ratio(&[], profile), 0.0);
    }

    #[test]
    fn a_profile_with_nothing_stored_scores_zero() {
        let mut profile = load_profile("hd_golf").unwrap().clone();
        profile.fields.retain(|field| field.target.is_none());
        assert_eq!(label_ratio(&boxes(&["Custom"]), &profile), 0.0);
    }
}
