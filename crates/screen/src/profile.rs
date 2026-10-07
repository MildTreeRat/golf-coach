//! Device profiles, and the label matcher every tile is located with. `profiles.py`. [M34 P3]
//!
//! A profile is the only device-specific knowledge in the reader: which tile labels to look for,
//! which `ShotData` field each feeds, and how to read a sign off the word a device prints beside a
//! number. It carries no pixel coordinates, because a hard-coded region breaks the moment the photo
//! is taken from somewhere else; the parser finds cells from the labels' own geometry instead.
//!
//! # The fork
//!
//! The data is `crates/screen/profiles.json`, read by `include_str!` (`contracts/devices.json`'s
//! pattern) and parsed once. It is a fork of the frozen parser's package data, byte-identical as
//! M34 P3 lands it, and `tests/profile_fork.rs` holds the two equal except the delta that test
//! declares — see the crate doc for why it is a fork at all.
//!
//! # Faithful, with two places Rust is stricter
//!
//! The shapes are pydantic's, defaults included, and the matcher is `profiles.py` to the operator.
//! Where the two languages part is only at load time, on a file this crate compiles in:
//!
//! - **`kind` is an enum.** Python keeps a `str` and the parser asks `kind == "text"`, so a typo
//!   reads as a number tile. Here it fails to parse, and the profile tests fail with it.
//! - **The types are not coerced.** Pydantic's lax mode would take `"sign": "1"`; serde will not.
//!   Neither profile file has a value that needs coercing.

use std::collections::BTreeMap;
use std::fmt;
use std::sync::OnceLock;

use serde::Deserialize;

use crate::difflib;

/// ADR-032 §5's read-at-compile-time, of this crate's own copy.
const PROFILES_JSON: &str = include_str!("../profiles.json");

/// Two labels are "the same" at or above this similarity. `profiles.py`'s value and reason: 0.8
/// absorbs ordinary OCR damage (`Carry` read as `Carny` is exactly 0.8) without letting `Club Speed`
/// match `Club Path` (0.63) or `Spin` match `Spin Axis` (0.62 after normalization).
pub const LABEL_MATCH_THRESHOLD: f64 = 0.8;

/// Qualifier tokens that set the sign of a number the device prints unsigned.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct SignRule {
    pub tokens: Vec<String>,
    /// `+1` or `-1`, applied to the magnitude read from the tile.
    pub sign: i64,
}

/// Whether a tile holds a number or words.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FieldKind {
    #[default]
    Number,
    Text,
}

/// One tile on the screen.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ProfileField {
    pub label: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    /// The `ShotData` key this tile fills, or `None` for a tile located for layout only: it is not
    /// stored, but finding it keeps its neighbour's value from spilling into its column.
    #[serde(default)]
    pub target: Option<String>,
    #[serde(default)]
    pub kind: FieldKind,
    #[serde(default)]
    pub unit: Option<String>,
    #[serde(default)]
    pub sign_tokens: Vec<SignRule>,
    /// Set when the device prints this tile's sign in the digits rather than as a word: `+1` when
    /// that polarity is already `ShotData`'s, `-1` when it is inverted. Left `None`, a printed sign
    /// is still reported as unknown, because a sign no stated convention explains is a guess.
    #[serde(default)]
    pub printed_sign: Option<i64>,
}

impl ProfileField {
    /// Similarity of `text` to this field's label or any alias, `0.0` to `1.0`.
    ///
    /// Text that normalizes to nothing (`---`, a lone `°`) scores `0.0` without reaching `difflib`,
    /// where two empty strings would score `1.0` against an empty label.
    pub fn matches(&self, text: &str) -> f64 {
        let candidate = normalize_label(text);
        if candidate.is_empty() {
            return 0.0;
        }
        let mut best = 0.0;
        for name in std::iter::once(&self.label).chain(&self.aliases) {
            let ratio = difflib::ratio(&candidate, &normalize_label(name));
            // Python's `max(best, ratio)`, which keeps `best` unless `ratio` is strictly greater.
            if ratio > best {
                best = ratio;
            }
        }
        best
    }
}

/// Everything the parser needs to read one make of launch-monitor screen.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct DeviceProfile {
    pub device: String,
    pub title: String,
    /// In the file's order, which is the screen's tile order, and which [`DeviceProfile::field_for`]
    /// walks.
    pub fields: Vec<ProfileField>,
    /// How far below a label its value may sit, as a multiple of the label's height: room for a
    /// number, its unit and a qualifier stacked (`1.1 °` over `Closed`).
    #[serde(default = "default_value_span_ratio")]
    pub value_span_ratio: f64,
    /// Cell contents meaning "this device did not measure it", read as `None`.
    #[serde(default = "default_blank_markers")]
    pub blank_markers: Vec<String>,
}

fn default_value_span_ratio() -> f64 {
    3.5
}

fn default_blank_markers() -> Vec<String> {
    vec!["---".to_string()]
}

impl DeviceProfile {
    /// The fields that feed a `ShotData` key, in tile order.
    pub fn stored_fields(&self) -> Vec<&ProfileField> {
        self.fields
            .iter()
            .filter(|field| field.target.is_some())
            .collect()
    }

    /// The field whose label best matches `text`, or `None` when nothing reaches
    /// [`LABEL_MATCH_THRESHOLD`].
    ///
    /// **`>=`, so of two fields scoring the same, the later one wins.** That is the frozen parser's
    /// rule, ported as it stands. This picks a field for one box; which *box* a field then gets,
    /// when two boxes pick the same field, is the parser's question (`_find_labels`), and that is
    /// where the `Impact Position` tie lives and where M34 P8's tie rule lands.
    pub fn field_for(&self, text: &str) -> Option<&ProfileField> {
        let mut best = None;
        let mut best_score = LABEL_MATCH_THRESHOLD;
        for field in &self.fields {
            let score = field.matches(text);
            if score >= best_score {
                best = Some(field);
                best_score = score;
            }
        }
        best
    }
}

/// `normalize_label`: upper-cased, every run of anything but `A-Z` and `0-9` made one space, and
/// the ends stripped. The form every label comparison is made in.
///
/// Python's `upper` first, because it can turn one character into ASCII letters (`ß` to `SS`, `ﬁ`
/// to `FI`) that the class then keeps. `[^A-Z0-9]` is compiled without `IGNORECASE`, so it is the
/// ASCII ranges and nothing else: `É` survives `upper` and is then replaced, like `°`.
pub fn normalize_label(text: &str) -> String {
    let upper = pyfmt::upper(text);
    let mut normalized = String::with_capacity(upper.len());
    let mut in_run = false;
    for c in upper.chars() {
        if c.is_ascii_uppercase() || c.is_ascii_digit() {
            normalized.push(c);
            in_run = false;
        } else if !in_run {
            normalized.push(' ');
            in_run = true;
        }
    }
    pyfmt::strip(&normalized).to_string()
}

/// `profiles.json`'s top level.
#[derive(Deserialize)]
struct ProfileFile {
    profiles: Vec<DeviceProfile>,
}

/// Every profile in `json`, by device name. A device listed twice is its later entry, as in the
/// dict comprehension `_load_profiles` builds.
///
/// Takes the text rather than reading [`PROFILES_JSON`] itself, because the unit tests here and in
/// `parser` and `validate` build made-up profiles through the same shapes (and M34 P8's `frozen`
/// module read the frozen Python copy through them until P10 deleted it).
pub(crate) fn parse_profiles(json: &str) -> serde_json::Result<BTreeMap<String, DeviceProfile>> {
    let file: ProfileFile = serde_json::from_str(json)?;
    Ok(file
        .profiles
        .into_iter()
        .map(|profile| (profile.device.clone(), profile))
        .collect())
}

/// This crate's profiles, parsed once. `capability.rs`'s `OnceLock`, for its reason.
///
/// `pub(crate)` so `lib.rs`'s tests can hold every shipped profile's targets to `ShotData`'s fields
/// (M34 P6), which no single-device lookup can do.
pub(crate) fn profiles() -> &'static BTreeMap<String, DeviceProfile> {
    static PROFILES: OnceLock<BTreeMap<String, DeviceProfile>> = OnceLock::new();
    PROFILES.get_or_init(|| {
        parse_profiles(PROFILES_JSON).expect("profiles.json ships in this crate and parses")
    })
}

/// A device no profile names. Python raises `KeyError` with [`fmt::Display`]'s text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownProfile {
    pub device: String,
    /// Every device a profile names, sorted.
    pub known: Vec<String>,
}

impl fmt::Display for UnknownProfile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Unknown launch-monitor profile {}. Known profiles: {}",
            pyfmt::str_repr(&self.device),
            self.known.join(", ")
        )
    }
}

impl std::error::Error for UnknownProfile {}

/// The profile for `device`, or a refusal naming the ones that exist.
///
/// No default device, where Python's signature has `"hd_golf"`: a caller that means it says it.
pub fn load_profile(device: &str) -> Result<&'static DeviceProfile, UnknownProfile> {
    let profiles = profiles();
    profiles.get(device).ok_or_else(|| UnknownProfile {
        device: device.to_string(),
        // A `BTreeMap`'s keys are sorted by code point, which is Python's `sorted` on `str`.
        known: profiles.keys().cloned().collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hd_golf() -> &'static DeviceProfile {
        load_profile("hd_golf").unwrap()
    }

    /// A profile from inline JSON, through the same parse the shipped file takes.
    fn profile(json: &str) -> DeviceProfile {
        parse_profiles(json)
            .unwrap()
            .into_values()
            .next()
            .expect("one profile")
    }

    /// Each case is frozen Python's answer, measured when this was written.
    #[test]
    fn labels_normalize_as_python_normalizes_them() {
        for (text, want) in [
            ("Bounce & Roll", "BOUNCE ROLL"),
            ("  Smash  Factor:", "SMASH FACTOR"),
            ("1.6 \u{b0} O>I", "1 6 O I"),
            ("Stra\u{df}e", "STRASSE"),
            ("\u{fb01}ne", "FINE"),
            ("\u{112}lo", "LO"),
            ("Club\u{a0}Path", "CLUB PATH"),
            ("\u{b0}", ""),
            ("---", ""),
        ] {
            assert_eq!(normalize_label(text), want, "{text:?}");
        }
    }

    #[test]
    fn an_alias_matches_where_the_label_does_not() {
        let bounce = hd_golf()
            .fields
            .iter()
            .find(|field| field.label == "Bounce & Roll")
            .unwrap();
        assert_eq!(bounce.matches("Bounce and Roll"), 1.0);
        assert_eq!(bounce.matches("---"), 0.0);
    }

    /// `Carny` scores exactly the threshold and is taken; `Car` scores 0.75 and is not.
    #[test]
    fn the_threshold_is_inclusive() {
        let profile = hd_golf();
        let carry = profile.field_for("Carny").expect("0.8 is a match");
        assert_eq!(carry.label, "Carry");
        assert_eq!(carry.matches("Carny"), LABEL_MATCH_THRESHOLD);
        assert_eq!(carry.matches("Car"), 0.75);
        assert!(profile.field_for("Car").is_none());
        assert!(profile.field_for("").is_none());
    }

    /// `Club Speed` scores 0.7 against `Ball Speed` and 1.0 against itself: the best field wins,
    /// whatever the tile order.
    #[test]
    fn the_best_field_wins() {
        assert_eq!(
            hd_golf().field_for("Club Speed").unwrap().label,
            "Club Speed"
        );
        assert_eq!(
            hd_golf().field_for("ImpactPosition").unwrap().label,
            "Impact Position"
        );
    }

    /// `Path` scores 8/9 against both `Paths` and `Pathy`, and the later field takes it.
    #[test]
    fn a_tie_goes_to_the_later_field() {
        let profile = profile(
            r#"{"profiles": [{"device": "t", "title": "T", "fields": [
                {"label": "Paths", "target": "a"},
                {"label": "Pathy", "target": "b"}
            ]}]}"#,
        );
        assert_eq!(
            profile.fields[0].matches("Path"),
            profile.fields[1].matches("Path")
        );
        assert_eq!(
            profile.field_for("Path").unwrap().target.as_deref(),
            Some("b")
        );
    }

    #[test]
    fn an_absent_key_takes_pydantics_default() {
        let profile = profile(
            r#"{"profiles": [{"device": "t", "title": "T", "fields": [{"label": "Carry"}]}]}"#,
        );
        assert_eq!(profile.value_span_ratio, 3.5);
        assert_eq!(profile.blank_markers, ["---"]);
        let field = &profile.fields[0];
        assert_eq!(field.kind, FieldKind::Number);
        assert!(field.aliases.is_empty() && field.sign_tokens.is_empty());
        assert_eq!((field.target.as_ref(), field.unit.as_ref()), (None, None));
        assert_eq!(field.printed_sign, None);
    }

    #[test]
    fn a_layout_only_tile_is_not_stored() {
        let stored = hd_golf().stored_fields();
        assert!(stored.iter().all(|field| field.target.is_some()));
        assert!(!stored.iter().any(|field| field.label == "Custom"));
        assert!(hd_golf().fields.iter().any(|field| field.label == "Custom"));
    }

    #[test]
    fn an_unknown_device_is_refused_in_pythons_words() {
        let refusal = load_profile("r10").unwrap_err();
        assert_eq!(
            refusal.to_string(),
            "Unknown launch-monitor profile 'r10'. Known profiles: hd_golf"
        );
    }
}
