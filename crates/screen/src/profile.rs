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
//!
//! # Keys the frozen shapes do not have
//!
//! From M42 a profile may turn on parser rules frozen Python does not have (ADR-035 clause 4: the
//! frozen parser gains nothing). Each such key defaults to off, so a profile that does not name it,
//! `hd_golf` and the frozen copy included, is read exactly as before. Each is pinned by a test that a
//! profile naming it parses to the value meant (the M42 plan's call 9), because the load ignores a
//! key it does not know, and a misspelt one would otherwise switch nothing on in silence. A key
//! spelt right and set where its rule means nothing is the other way to be wrong, and
//! [`DeviceProfile::check`] refuses it when the shipped profiles load.

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

/// One of the two letters a device prints beside a number and lights to give its sign: Full Swing
/// prints `L` and `R` over ball direction and lights one yellow, and the number itself is unsigned.
/// No Python twin: the frozen profiles have no such key. [M42 P6]
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct LitToken {
    /// Compared with a box's text stripped and upper-cased, so it is written that way
    /// ([`DeviceProfile::check`] holds it).
    pub token: String,
    /// `+1` or `-1`, applied to the magnitude when this letter is the lit one.
    pub sign: i64,
}

/// The unit a device prints a tile's number in, where it is not the unit `ShotData` holds that
/// field in. One variant, because one has been met: Full Swing prints its apex as `APEX FT`, and
/// `ShotData::apex_height` is yards (the M42 plan's finding 7). No Python twin: every frozen profile
/// prints its numbers in `ShotData`'s own units. [M42 P7]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PrintedUnit {
    /// Printed in feet, stored in yards.
    Feet,
}

impl PrintedUnit {
    /// `printed`, a number read in this unit, in the yards `ShotData` holds it in (the M42 plan's
    /// call 8). A yard is three feet exactly, by definition.
    ///
    /// **Divided by 3.0, never multiplied by a third**, because `1.0 / 3.0` is rounded before it is
    /// used: `85.2 / 3.0` is `28.400000000000002`, and `85.2 * (1.0 / 3.0)` is `28.4`, a different
    /// float (and so are `15.5` and `142.2`, three of the six apexes the plan's truth table holds).
    /// The division is the one the plan's photo gate computes its oracle with.
    ///
    /// **Not rounded**: the screen prints a tenth of a foot, a thirtieth of a yard, and rounding to
    /// any number of decimals would state a precision the screen did not print. `raw_fields` keeps
    /// the number as printed.
    pub fn to_yards(self, printed: f64) -> f64 {
        match self {
            Self::Feet => printed / 3.0,
        }
    }
}

/// The `ShotData` fields held in yards, by `crates/contracts/src/shot.rs`'s own docs: the only
/// targets [`PrintedUnit`] may convert to, because its one conversion ends in yards. In ShotData's
/// committed schema these are the fields whose description starts `yards`, and a test here holds
/// the two equal, so a yard field added later is not refused in silence. [M42 P7]
const STORED_IN_YARDS: [&str; 5] = [
    "apex_height",
    "carry_distance",
    "total_distance",
    "bounce_and_roll",
    "carry_offline",
];

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
    /// Set when the screen prints this tile twice, and the two must agree before either is stored:
    /// Full Swing's `TOTAL YD` stands in both its header row and its data row. The field may then
    /// take two label boxes where any other takes one, and the two cells are read together. The
    /// rule is `parser::read_pair`'s (the M42 plan's call 7). Allowed only on a stored number tile,
    /// which [`DeviceProfile::check`] holds. No Python twin: the frozen parser gives a label one
    /// box. [M42 P5]
    #[serde(default)]
    pub printed_twice: bool,
    /// Set when the device prints the tile's number unsigned and gives its sign by lighting one of
    /// two letters printed beside it (`L`/`R`, `D`/`F`). The sign is then read from the colour of
    /// the letters' boxes and never from the digits, and a sign the colours cannot decide withholds
    /// the value. The rule is `parser::read_text`'s, the colour test `parser::is_lit` (the M42
    /// plan's calls 4 and 5). Empty, the default, is off. [`DeviceProfile::check`] holds it to two
    /// letters of opposite sign, on a stored number tile with no other sign rule. No Python twin:
    /// the frozen parser reads a sign only from words and digits. [M42 P6]
    #[serde(default)]
    pub lit_sign: Vec<LitToken>,
    /// Set when the device prints the tile's number in a unit other than the one `ShotData` holds
    /// the field in: Full Swing prints its apex in feet, and `apex_height` is yards. The number is
    /// converted ([`PrintedUnit::to_yards`]) once its sign is applied, and `raw_fields` keeps it as
    /// printed. The rule is `parser::read_text`'s (the M42 plan's call 8). Allowed only on a number
    /// tile stored in yards, which [`DeviceProfile::check`] holds. `None`, the default, stores the
    /// number in the unit it was printed in, as every profile before M42 does. No Python twin.
    /// [M42 P7]
    #[serde(default)]
    pub printed_unit: Option<PrintedUnit>,
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
    /// Whether a box that matches no label is tried as two labels the OCR ran together
    /// (`CLUBSPEEDMPHBALLSPEEDMPH`), and split where each half matches a different label on its
    /// own. Off unless the profile turns it on, so a profile that does not name it reads exactly as
    /// it did before M42. The rule, and why it is a profile's choice, are `parser::split_merged`'s
    /// (the M42 plan's call 6). No Python twin: the frozen profiles have no such key and the frozen
    /// parser splits nothing. [M42 P4]
    #[serde(default)]
    pub split_merged_labels: bool,
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

    /// The rules this crate refuses to load, as the first one broken, or `Ok`. No Python twin: the
    /// frozen profiles have none of the keys it checks. [M42 P5]
    ///
    /// **Why at load time.** Each key it checks turns on a parser rule that is meaningful on some
    /// fields and not on others, and the parse would otherwise either ignore a misplaced key in
    /// silence or apply it where it reads a wrong number. [`profiles`] runs it on every shipped
    /// profile and panics on a refusal, as it does on a file that does not parse, so a misplaced
    /// key fails every test that reads a profile rather than one photo's read; the unit tests run
    /// it on made-up profiles. The clauses, each the M42 plan's:
    ///
    /// - **`printed_twice` only on a stored number tile** (call 7): agreement is compared between
    ///   two numbers, and a tile stored nowhere has nothing to agree about.
    /// - **`lit_sign` only on a stored number tile** (call 5), for the same reason: a sign is
    ///   applied to a number, and a layout-only or text tile would ignore the key in silence.
    ///   [M42 P6]
    /// - **`lit_sign` never beside `sign_tokens` or `printed_sign`** (call 5): a tile's sign has one
    ///   source, and a word or a printed `-` the profile also trusted could contradict the letters.
    /// - **`lit_sign` is two different letters, one signed `1` and one `-1`, each written as a box
    ///   is compared** (stripped and upper-cased, and not empty). The plan names the pair and the
    ///   signs; the spelling clause is this phase's, because a token no box can equal, or two
    ///   tokens one box equals, would leave the rule unable to decide in silence on every photo.
    /// - **`printed_unit` only on a number tile whose target `ShotData` holds in yards** (call 8):
    ///   the one conversion there is ends in yards, so on a tile stored in degrees or mph it would
    ///   store a third of the number, and on a layout-only or text tile it would be ignored in
    ///   silence. [M42 P7]
    pub fn check(&self) -> Result<(), String> {
        for field in &self.fields {
            let refused = |rule: &str| {
                Err(format!(
                    "{}: {} sets {rule}",
                    self.device,
                    pyfmt::str_repr(&field.label)
                ))
            };
            let stored_number = field.target.is_some() && field.kind == FieldKind::Number;
            if field.printed_twice && !stored_number {
                return refused("printed_twice, which only a stored number tile may");
            }
            let in_yards = field
                .target
                .as_deref()
                .is_some_and(|target| STORED_IN_YARDS.contains(&target));
            if field.printed_unit.is_some() && !(stored_number && in_yards) {
                return refused("printed_unit, which only a number tile stored in yards may");
            }
            if field.lit_sign.is_empty() {
                continue;
            }
            if !stored_number {
                return refused("lit_sign, which only a stored number tile may");
            }
            if !field.sign_tokens.is_empty() || field.printed_sign.is_some() {
                return refused(
                    "lit_sign beside sign_tokens or printed_sign, and a sign has one source",
                );
            }
            let spelt =
                |token: &str| !token.is_empty() && pyfmt::upper(pyfmt::strip(token)) == token;
            let pair = match field.lit_sign.as_slice() {
                [a, b] => Some((a, b)),
                _ => None,
            };
            if !pair.is_some_and(|(a, b)| {
                a.token != b.token
                    && spelt(&a.token)
                    && spelt(&b.token)
                    && [a.sign, b.sign].contains(&1)
                    && [a.sign, b.sign].contains(&-1)
            }) {
                return refused(
                    "lit_sign, which takes two different tokens, stripped and upper-case, \
                     one signed 1 and one -1",
                );
            }
        }
        Ok(())
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

/// This crate's profiles, parsed and checked once. `capability.rs`'s `OnceLock`, for its reason.
///
/// Each profile passes [`DeviceProfile::check`] or this panics with its refusal, as a file that does
/// not parse does: both are a fault in a file this crate compiles in, never in a photo. [M42 P5]
///
/// `pub(crate)` so `lib.rs`'s tests can hold every shipped profile's targets to `ShotData`'s fields
/// (M34 P6), which no single-device lookup can do.
pub(crate) fn profiles() -> &'static BTreeMap<String, DeviceProfile> {
    static PROFILES: OnceLock<BTreeMap<String, DeviceProfile>> = OnceLock::new();
    PROFILES.get_or_init(|| {
        let profiles =
            parse_profiles(PROFILES_JSON).expect("profiles.json ships in this crate and parses");
        for profile in profiles.values() {
            if let Err(problem) = profile.check() {
                panic!("profiles.json ships a profile its check refuses: {problem}");
            }
        }
        profiles
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
        assert!(!profile.split_merged_labels);
        let field = &profile.fields[0];
        assert_eq!(field.kind, FieldKind::Number);
        assert!(field.aliases.is_empty() && field.sign_tokens.is_empty());
        assert_eq!((field.target.as_ref(), field.unit.as_ref()), (None, None));
        assert_eq!(field.printed_sign, None);
        assert!(!field.printed_twice);
        assert!(field.lit_sign.is_empty());
        assert_eq!(field.printed_unit, None);
    }

    /// The M42 plan's call 9 for `printed_unit`: read where it is spelt right, on no `hd_golf`
    /// field, and a unit there is no variant for refused when the file is parsed, where an unknown
    /// key is not, because a unit read as none would store feet as yards.
    #[test]
    fn a_field_opts_into_a_printed_unit_by_name() {
        let profile = profile(
            r#"{"profiles": [{"device": "t", "title": "T", "fields": [
                {"label": "Apex Ft", "target": "apex_height", "printed_unit": "feet"},
                {"label": "Carry Yd", "target": "carry_distance"}
            ]}]}"#,
        );
        assert_eq!(profile.fields[0].printed_unit, Some(PrintedUnit::Feet));
        assert_eq!(profile.fields[1].printed_unit, None);
        assert_eq!(profile.check(), Ok(()));
        assert!(hd_golf()
            .fields
            .iter()
            .all(|field| field.printed_unit.is_none()));
        for unit in ["Feet", "metres"] {
            let json = format!(
                r#"{{"profiles": [{{"device": "t", "title": "T", "fields": [
                    {{"label": "Apex Ft", "target": "apex_height", "printed_unit": "{unit}"}}
                ]}}]}}"#
            );
            assert!(parse_profiles(&json).is_err(), "{unit:?}");
        }
    }

    /// Call 8's clause of the load-time check: a tile stored in degrees or mph, a layout-only tile
    /// and a text tile are each refused, by name; each of the five yard targets is allowed; and
    /// every shipped profile passes.
    #[test]
    fn printed_unit_is_refused_except_on_a_number_tile_stored_in_yards() {
        let check = |keys: &str| {
            profile(&format!(
                r#"{{"profiles": [{{"device": "t", "title": "T", "fields": [
                    {{"label": "X", {keys}"printed_unit": "feet"}}
                ]}}]}}"#
            ))
            .check()
        };
        for keys in [
            r#""target": "launch_angle", "#,
            r#""target": "club_head_speed", "#,
            "",
            r#""target": "apex_height", "kind": "text", "#,
        ] {
            assert_eq!(
                check(keys),
                Err(
                    "t: 'X' sets printed_unit, which only a number tile stored in yards may".into()
                ),
                "{keys}"
            );
        }
        for target in STORED_IN_YARDS {
            assert_eq!(
                check(&format!(r#""target": "{target}", "#)),
                Ok(()),
                "{target}"
            );
        }
        for shipped in profiles().values() {
            assert_eq!(shipped.check(), Ok(()), "{}", shipped.device);
        }
    }

    /// The yard targets are the `ShotData` fields whose committed schema description starts
    /// `yards` (each from its doc in `crates/contracts/src/shot.rs`), no more and no fewer.
    #[test]
    fn the_yard_targets_are_the_fields_shot_data_holds_in_yards() {
        let schema: serde_json::Value =
            serde_json::from_str(include_str!("../../../spec/schemas/shot_data.schema.json"))
                .unwrap();
        let in_yards: std::collections::BTreeSet<&str> = schema["properties"]
            .as_object()
            .unwrap()
            .iter()
            .filter(|(_, property)| {
                property["description"]
                    .as_str()
                    .is_some_and(|text| text.starts_with("yards"))
            })
            .map(|(key, _)| key.as_str())
            .collect();
        assert_eq!(in_yards, STORED_IN_YARDS.into_iter().collect());
    }

    /// `85.2 / 3.0`, and not the float a multiplication by a third gives: the two differ in the
    /// last bit, and the photo gate's oracle divides.
    #[test]
    fn feet_are_divided_by_three() {
        let yards = PrintedUnit::Feet.to_yards(85.2);
        assert_eq!(yards.to_bits(), (85.2_f64 / 3.0).to_bits());
        assert_ne!(yards.to_bits(), (85.2_f64 * (1.0 / 3.0)).to_bits());
    }

    /// The M42 plan's call 9 for `lit_sign`: read where it is spelt right, in the file's order, and
    /// on no `hd_golf` field.
    #[test]
    fn a_field_opts_into_a_lit_sign_by_name() {
        let profile = profile(
            r#"{"profiles": [{"device": "t", "title": "T", "fields": [
                {"label": "Ball Direction Deg", "target": "launch_direction",
                 "lit_sign": [{"token": "L", "sign": -1}, {"token": "R", "sign": 1}]},
                {"label": "Carry Yd", "target": "carry_distance"}
            ]}]}"#,
        );
        let token = |token: &str, sign| LitToken {
            token: token.to_string(),
            sign,
        };
        assert_eq!(profile.fields[0].lit_sign, [token("L", -1), token("R", 1)]);
        assert!(profile.fields[1].lit_sign.is_empty());
        assert_eq!(profile.check(), Ok(()));
        assert!(hd_golf()
            .fields
            .iter()
            .all(|field| field.lit_sign.is_empty()));
    }

    /// Call 5's clauses of the load-time check, each refusal named, and every shipped profile
    /// passing. A sign of `2`, a lower-case or padded token, one token twice, one token or three:
    /// each is a pair the parser could not read a sign from on any photo.
    #[test]
    fn lit_sign_is_refused_except_as_two_letters_of_opposite_sign_on_a_stored_number_tile() {
        let pair = r#""lit_sign": [{"token": "L", "sign": -1}, {"token": "R", "sign": 1}]"#;
        let tile = "lit_sign, which only a stored number tile may";
        let source = "lit_sign beside sign_tokens or printed_sign, and a sign has one source";
        let shape = "lit_sign, which takes two different tokens, stripped and upper-case, one \
                     signed 1 and one -1";
        for (keys, rule) in [
            (format!(r#""label": "X", {pair}"#), tile),
            (
                format!(r#""label": "X", "target": "x", "kind": "text", {pair}"#),
                tile,
            ),
            (
                format!(r#""label": "X", "target": "x", "printed_sign": 1, {pair}"#),
                source,
            ),
            (
                format!(
                    r#""label": "X", "target": "x", {pair},
                       "sign_tokens": [{{"tokens": ["R"], "sign": 1}}]"#
                ),
                source,
            ),
            (
                r#""label": "X", "target": "x", "lit_sign": [{"token": "R", "sign": 1}]"#.into(),
                shape,
            ),
            (
                r#""label": "X", "target": "x", "lit_sign": [{"token": "L", "sign": -1},
                   {"token": "R", "sign": 1}, {"token": "C", "sign": 1}]"#
                    .into(),
                shape,
            ),
            (
                r#""label": "X", "target": "x", "lit_sign": [{"token": "L", "sign": 1},
                   {"token": "R", "sign": 1}]"#
                    .into(),
                shape,
            ),
            (
                r#""label": "X", "target": "x", "lit_sign": [{"token": "L", "sign": -2},
                   {"token": "R", "sign": 2}]"#
                    .into(),
                shape,
            ),
            (
                r#""label": "X", "target": "x", "lit_sign": [{"token": "R", "sign": -1},
                   {"token": "R", "sign": 1}]"#
                    .into(),
                shape,
            ),
            (
                r#""label": "X", "target": "x", "lit_sign": [{"token": "l", "sign": -1},
                   {"token": "R", "sign": 1}]"#
                    .into(),
                shape,
            ),
            (
                r#""label": "X", "target": "x", "lit_sign": [{"token": "L ", "sign": -1},
                   {"token": "R", "sign": 1}]"#
                    .into(),
                shape,
            ),
            (
                r#""label": "X", "target": "x", "lit_sign": [{"token": "", "sign": -1},
                   {"token": "R", "sign": 1}]"#
                    .into(),
                shape,
            ),
        ] {
            let refused = profile(&format!(
                r#"{{"profiles": [{{"device": "t", "title": "T", "fields": [{{{keys}}}]}}]}}"#
            ));
            assert_eq!(
                refused.check(),
                Err(format!("t: 'X' sets {rule}")),
                "{keys}"
            );
        }
        for shipped in profiles().values() {
            assert_eq!(shipped.check(), Ok(()), "{}", shipped.device);
        }
    }

    /// The M42 plan's call 9 for `printed_twice`: read where it is spelt right, and on no `hd_golf`
    /// field.
    #[test]
    fn a_field_opts_into_being_printed_twice_by_name() {
        let profile = profile(
            r#"{"profiles": [{"device": "t", "title": "T", "fields": [
                {"label": "Total Yd", "target": "total_distance", "printed_twice": true},
                {"label": "Carry Yd", "target": "carry_distance"}
            ]}]}"#,
        );
        assert!(profile.fields[0].printed_twice);
        assert!(!profile.fields[1].printed_twice);
        assert_eq!(profile.check(), Ok(()));
        assert!(hd_golf().fields.iter().all(|field| !field.printed_twice));
    }

    /// Call 7's clause of the load-time check: a layout-only tile and a text tile are refused, each
    /// named, and every shipped profile passes (`profiles` would have panicked otherwise).
    #[test]
    fn printed_twice_is_refused_except_on_a_stored_number_tile() {
        for (field, label) in [
            (r#"{"label": "To Hole", "printed_twice": true}"#, "To Hole"),
            (
                r#"{"label": "Shot Type", "target": "shot_type", "kind": "text",
                    "printed_twice": true}"#,
                "Shot Type",
            ),
        ] {
            let refused = profile(&format!(
                r#"{{"profiles": [{{"device": "t", "title": "T", "fields": [{field}]}}]}}"#
            ));
            assert_eq!(
                refused.check(),
                Err(format!(
                    "t: '{label}' sets printed_twice, which only a stored number tile may"
                ))
            );
        }
        for shipped in profiles().values() {
            assert_eq!(shipped.check(), Ok(()), "{}", shipped.device);
        }
    }

    /// The M42 plan's call 9: the key is read where it is spelt right. `hd_golf` never names it,
    /// because its profile entry is the one M42 leaves byte-unchanged.
    #[test]
    fn a_profile_opts_into_splitting_merged_labels_by_name() {
        let profile = profile(
            r#"{"profiles": [{"device": "t", "title": "T", "split_merged_labels": true,
                "fields": [{"label": "Carry"}]}]}"#,
        );
        assert!(profile.split_merged_labels);
        assert!(!hd_golf().split_merged_labels);
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
            "Unknown launch-monitor profile 'r10'. Known profiles: full_swing, hd_golf"
        );
    }
}
