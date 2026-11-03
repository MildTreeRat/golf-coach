//! Located OCR text in, a [`ParsedShot`] out: `parser.py::parse_screen`, ported faithfully and then
//! changed in one place, which box is which label [M34 P5, P8]; and since M42, given rules a
//! profile opts into, which frozen Python does not have.
//!
//! The screen is a grid of tiles, each a label with its value stacked underneath (`Carry` over
//! `128.1` over `yds`). Nothing here knows where a tile sits. The parser *finds* the labels, groups
//! them into rows, derives each tile's column from its neighbours, and reads whatever falls in the
//! cell below, so a photo framed differently parses the same. The profile supplies the labels and
//! the sign rules, and the geometry is worked out per image.
//!
//! # The change: the tie rule
//!
//! §M34's *record, port, then change* ported every rule here faithfully first (P5), and M34 P8 then
//! changed how labels are located, and nothing after that. Frozen Python's `_find_labels` gives each
//! label to the first box with the best score (`>`). On `2026-08-10-1` the two `Impact Position`
//! boxes both score 1.0, the `Impact Position V` tile's is seen first and wins, and the real tile's
//! `CENTER` then has no label above it and spills into two neighbours' text. On `2026-08-23-1` the
//! real tile wins on a margin of 0.0002, which is luck. [`parse_screen`] assigns boxes to labels
//! one-to-one instead, and **withholds** any label whose box the evidence cannot decide (the M34
//! plan's call 8, decisions 3, 5 and 6, written out at [`TIE_MARGIN`]). A wrong value is worse than
//! none (ADR-010 §2), and a label that could be either of two boxes is one coin-flip from a wrong
//! value.
//!
//! The faithful rules (`>` in `_find_labels`, and the later of two same-labelled fields taking the
//! value) were kept behind a `frozen` module from P8 until M34 P10 re-recorded the Python-recorded
//! vectors through this parser, and were deleted then (the M34 plan's decision 2). The frozen Python
//! parser itself is the alternative that stays runnable until M40 (`docs/README.md` §Conventions),
//! and git history and each re-recorded vector's `provenance.rerecords` ledger carry the evidence.
//! Everything after the labels are located — the rows, the columns, the cells, the signs, the
//! confidence — was one copy shared by both readers while both existed, so the Python-recorded
//! vectors gated it the whole way.
//!
//! # Rules a profile opts into [M42]
//!
//! Full Swing's screen needs rules HD Golf's does not. Each is off unless a profile turns it on, so
//! a profile that does not name it, `hd_golf` included, reads exactly as before and no committed
//! vector moves (the M42 plan's call 3). None has a Python twin: the frozen parser has none of them
//! and gains none (ADR-035 clause 4).
//!
//! **Merged labels are split** (`split_merged_labels`, the M42 plan's call 6). PP-OCR sometimes
//! returns two adjacent tile labels as one box: `CLUBSPEEDMPHBALLSPEEDMPH` on `IMG_2891`, and
//! `BALLDIRECTIONDEGLAUNCHANGLE` on `IMG_2891` and `2026-10-07-1` (the plan's finding 3). Such a box
//! scores below the threshold against both labels, so neither tile is located and neither value is
//! read, on a photo where both are printed plainly. [`split_merged`] runs before anything else, and
//! tries each box that matches no label as two, cut between two letters of its text with the spaces
//! taken out. A cut counts only when each half matches a label on its own and the two are different
//! labels, so a value, a stray glyph or one label read twice is left whole. Two-way only: every
//! merge measured was two labels.
//!
//! **The box's width is shared in proportion to the letters, and that is an approximation.** The font
//! is proportional, and the gap the OCR closed between the two labels is counted as neither's. It is
//! good enough because of where the cut lands: a label's column runs halfway to its neighbour's
//! edge, so the cut *is* the boundary between the two columns, and a value is placed by its centre,
//! which sits under its own label, about half a label's width from that boundary. An error of a
//! letter or two in the cut leaves it in its own column. The Full Swing photo gate (M42 P9) is what
//! holds that on real photos.
//!
//! **A tile printed twice must agree with itself** (`printed_twice` on a field, the M42 plan's call
//! 7). Full Swing prints `TOTAL YD` in its header row and again in its data row, the same number on
//! every photo the plan read (its finding 6). Without the rule the two boxes fit one label equally,
//! the tie rule withholds it as it must (the M34 plan's decision 6, the `carry-twice` vector), and
//! the total is lost on every photo. With it, that field may take two boxes ([`assign_labels`]),
//! and [`read_pair`] reads both cells and treats the second copy as a check on the first, not a
//! rival: equal, the value is stored; different, nothing is, because one is a misread and nothing
//! says which; only one readable, it is stored, since one copy is all any other tile ever has. A
//! third box fitting the label is still a tie, and is withheld as any other.
//!
//! **A sign is read from a lit letter** (`lit_sign` on a field, the M42 plan's calls 4 and 5). Full
//! Swing prints ball direction and sidespin unsigned, with both letters of a pair beside the number
//! (`L` `R`, `D` `F`), and lights the one that applies yellow. Nothing in the text says which, so
//! HD Golf's word rule (`sign_tokens`) cannot read it: both letters are always there. The OCR does
//! not return every letter (0, 1 or 2 of a pair on the photos the plan read, its finding 4), so
//! [`lit_sign`] decides from what it did return, by colour ([`is_lit`] on `TextBox::rgb`), and
//! **withholds** the value where the letters cannot decide, rather than storing the magnitude
//! unsigned as the word rule does: on a signed field that is a wrong number half the time
//! (ADR-010 §2). The colour is measured by the host that ran the OCR (`crates/ocr`, the phone's
//! Vision in M38), and the rule that reads it is here so both hosts share it.
//!
//! **A number printed in feet is stored in yards** (`printed_unit` on a field, the M42 plan's call
//! 8). Full Swing prints its apex as `APEX FT`, and `ShotData::apex_height` is yards (the plan's
//! finding 7). Stored as printed, the height would be three times too great, and the validator's
//! 80-yard ceiling would not catch it on three of the six photos the plan read (`39.9`, `71.4` and
//! `15.5` feet all pass as yards). [`read_text`] divides the number by 3.0 once its sign is applied
//! (`PrintedUnit::to_yards`, which says why it divides and does not round), and `raw_fields` keeps
//! the text as printed, so a record still says what the screen showed. A tile printed twice
//! compares its two copies after the conversion, each converted alike.
//!
//! # Where CPython is reproduced rather than approximated
//!
//! Every warning is compared exactly, and every value a golfer sees came through one of these, so
//! each is the CPython operation and not its Rust look-alike. `pyfmt` holds the language's half, each
//! with the measured difference in its own doc: [`pyfmt::str_repr`] for every `{text!r}`,
//! [`pyfmt::floor_div`] for a cell's line bucket, [`pyfmt::upper`], [`pyfmt::split`],
//! [`pyfmt::strip`] and [`pyfmt::strip_space`] for the Unicode string handling, and [`pyfmt::sum`]
//! for the OCR mean, which CPython has compensated since 3.12. Two-argument `min` and `max` keep
//! their first argument on a tie (`first_min`, `first_max`), as `phases.rs` found for `max`.
//!
//! The number pattern is a **hand scanner**, not the `regex` crate: `[-+]?\d[\d,]*(?:\.\d+)?` is one
//! leftmost pass with no backtracking that can change the answer, and the `_THOUSANDS` rule that
//! follows it only ever looks one character past three digits (M34 P4's finding 8). What a scanner
//! cannot borrow from std is `\d` itself. Python's `str` patterns match every Unicode decimal digit,
//! and `float()` reads them, so `١٢٣` is 123.0. Rust's `char::is_numeric` is wider (`²`, `Ⅷ`), and
//! `to_digit` is ASCII only, so `decimal_value` carries CPython's own table.
//!
//! # Four private functions are public
//!
//! [`first_number`], [`sign_from`], [`is_blank`] and [`cell_text`] are `parser.py`'s private
//! `_first_number`, `_sign_from`, `_is_blank` and `_Cell.text`. They are public because
//! `spec/vectors/screen/units/` gates each on a table of its own, and `tests/units.rs` is outside
//! the crate. Nothing outside this crate's tests has a reason to call them.

use std::borrow::Cow;
use std::cmp::Ordering;
use std::collections::BTreeSet;

use pyfmt::OrderedMap;
use serde::Serialize;

use crate::profile::{
    normalize_label, DeviceProfile, FieldKind, LitToken, ProfileField, LABEL_MATCH_THRESHOLD,
};
use crate::TextBox;

/// **A tie is a margin, not an equality** (the M34 plan's decision 5): a label is withheld when an
/// assignment of boxes to labels that gives it a different box totals within this of the best one.
///
/// Measured on the stored photos before it was chosen, as the margin between the two ways of
/// assigning the `Impact Position` pair (the plan's finding 3). OCR damage to a label is noise of
/// **0.0002 to 0.004**: `ImpactPosition` against `Impact Position` on `2026-08-23-1` is 0.0002, and
/// an `lmpact Position` V tile would be 0.004. Real evidence is **0.066 and up**: an `Impact
/// Position Y` V tile is 0.066, and the exact `Impact Position V` on eleven bay photos is 0.125.
/// 0.02 sits five times above the noise and three below the evidence. Strict equality would read
/// `2026-08-23-1` on its 0.0002, and its mirror image (the V tile read exactly, the real tile
/// damaged) would store `HEEL` as `impact_position_v`.
pub const TIE_MARGIN: f64 = 0.02;

/// The most boxes, or labels, one tie is weighed over. A bigger tie is withheld whole, with a
/// warning, rather than enumerated.
///
/// The rule enumerates every one-to-one assignment within a tie, and a tie of `b` boxes that all
/// fit `f` labels has `Σₖ C(b,k)·C(f,k)·k!` of them: 7 at 2×2, which is every tie a stored photo
/// has, 1,441,729 at 8×8, and 17,572,114 at 9×9, walked twice. (A field printed twice, M42, may
/// take a second box, which adds assignments the sum leaves out; the cap bounds the boxes and the
/// labels either way.) The 8×8 case takes about 0.1 s in
/// an unoptimised test build on the dev box, and each size past it multiplies that by ten or more,
/// on a phone. A screen
/// with more than eight boxes competing for one set of labels is not a screen this parser can read
/// anyway: no hd_golf label is near another but the `Impact Position` pair
/// (`only_the_impact_position_pair_overlaps`), so a tie that size is a photo of something else, and
/// withholding it costs nothing a golfer could have had.
pub const TIE_CAP: usize = 8;

/// A lit letter's red and green must both reach this (0–255), so a dim, yellowish glyph is never
/// read as lit on its blue alone. No Python twin. [M42 P6]
///
/// On the photos it was set from, it separates nothing: the dimmest letter or text box measured
/// min(r, g) 161, and the unlit `F` on `IMG_2889` 183, so [`LIT_BLUE_RATIO`] decides every one of
/// them (M42 P2's finding). It is a floor under the ratio for a photo the plan did not have, a dark
/// one where noise could make a grey box's blue fall below 0.7 of a red and green near zero.
pub const LIT_MIN_BRIGHTNESS: f64 = 120.0;

/// A letter is lit when its blue is at most this fraction of the lesser of its red and green:
/// Full Swing lights a sign letter yellow, and prints the other one, and every number, white or
/// grey. No Python twin. [M42 P6]
///
/// Measured as the mean colour of the brightest quarter of each letter box's pixels (`TextBox::rgb`),
/// and set between the two groups it separates (the M42 plan's finding 4 and call 4):
///
/// | photo | letter | rgb | lit, by eye | b ÷ min(r, g) |
/// |---|---|---|---|---|
/// | `IMG_2889` | R | (239, 222, 90) | yes | 0.41 |
/// | `IMG_2889` | D | (227, 207, 72) | yes | 0.35 |
/// | `IMG_2889` | F | (187, 183, 176) | no | 0.96 |
/// | `IMG_2891` | R | (200, 206, 197) | no | 0.98 |
/// | `IMG_2891` | F | (215, 214, 206) | no | 0.96 |
/// | `IMG_2892` | R | (243, 225, 73) | yes | 0.32 |
/// | `IMG_2892` | D | (241, 223, 91) | yes | 0.41 |
/// | `2026-10-07-1` | R | (249, 220, 96) | yes | 0.44 |
/// | `2026-10-07-1` | F | (254, 238, 113) | yes | 0.47 |
///
/// Lit letters measure 0.32–0.47 and unlit ones 0.96–0.98; the white value digits on the same
/// photos 0.95–1.0, and through the shipping reader every label and value box 0.99–1.02 (M42 P2,
/// on `IMG_2889`). 0.7 sits in the gap, 0.23 above the highest lit letter and 0.26 below the
/// lowest unlit one. A grey or white glyph would have to lose a third of its blue to cross it.
pub const LIT_BLUE_RATIO: f64 = 0.7;

/// Two labels share a row when their centers sit within this fraction of the taller one's height.
/// Generous enough for OCR jitter, tight enough that the HD Golf grid's two label rows never merge.
const ROW_TOLERANCE: f64 = 0.7;

/// Within a cell, boxes whose centers are closer than this fraction of the label's height are one
/// line of text, read left to right.
const LINE_BUCKET: f64 = 0.6;

/// The three independent things that can go wrong: the wrong screen was found, the labels were
/// found but not their values, or the OCR itself was unsure. `parser.py`'s weights, and its reason
/// for keeping them apart: a clean read of the wrong screen and a barely legible right one are both
/// untrustworthy, for different reasons.
const W_LABELS: f64 = 0.35;
const W_VALUES: f64 = 0.35;
const W_OCR: f64 = 0.30;

/// What one tile held: a number, or words.
///
/// Serialized untagged, as the bare number or string, because that is what `ShotData(**values)`
/// receives in Python and what `to_shot_data` hands serde by key (M34 P6).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum FieldValue {
    Number(f64),
    Text(String),
}

/// One screen's worth of extracted metrics, with everything needed to audit it. `ParsedShot`.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedShot {
    pub device: String,
    /// By `ShotData` key, in the order the tiles were read.
    pub values: OrderedMap<FieldValue>,
    /// By tile label, the text each read tile held, `""` included.
    pub raw_fields: OrderedMap<String>,
    /// `0.0` to `1.0`, unrounded. `to_shot_data` rounds it to three places.
    pub confidence: f64,
    pub warnings: Vec<String>,
    /// Set by validation, never by parsing: the parser reports what it saw, the validator decides
    /// whether to trust it.
    pub needs_review: bool,
    /// The `ShotData` keys whose tiles were *located*, in profile tile order: read ones, blank ones,
    /// and ones the tie rule withheld alike (the M34 plan's call 9). `ShotProvenance::fields_present`
    /// on the shipping read, and the reason it can tell the two layouts apart: a bay photo has
    /// `impact_position_v` and no `bounce_and_roll`, a reference photo the reverse.
    ///
    /// **Located and unread is a structural mark** (decision 3): a key here whose label has no
    /// `raw_fields` entry is a tile the parser found and would not read. Every tile it *read* has a
    /// `raw_fields` entry, `""` included.
    pub fields_present: Vec<String>,
}

impl ParsedShot {
    fn new(device: &str) -> Self {
        Self {
            device: device.to_string(),
            values: OrderedMap::new(),
            raw_fields: OrderedMap::new(),
            confidence: 0.0,
            warnings: Vec::new(),
            needs_review: false,
            fields_present: Vec::new(),
        }
    }

    /// No value was read. `import_screen` refuses such a read rather than storing an empty shot.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

/// A label box, by index into the boxes, and the profile field it was resolved to, or `None` for a
/// box the tie rule would not resolve. Either way the box is a label: it bounds its row's columns,
/// as a boundary-only tile (`Custom`) does, and it is never value text.
///
/// `_Located` holds the box itself, and `_build_cells` keeps label boxes out of every cell by
/// `id()`. An index is that identity: each box in the list is a distinct object in Python.
type Located<'a> = (Option<&'a ProfileField>, usize);

/// What locating the labels found, before anything under them is read.
struct Labels<'a> {
    /// Every label box, in the order the rows are built from.
    tiles: Vec<Located<'a>>,
    /// Fields located but not read, each with the warning that says why.
    withheld: Vec<(&'a ProfileField, String)>,
}

/// A located tile: its label box and the text found underneath it. `field` is `None` for a box the
/// tie rule left unresolved, which is read as nothing, like a boundary-only tile.
struct Cell<'a> {
    field: Option<&'a ProfileField>,
    label_box: &'a TextBox,
    value_boxes: Vec<&'a TextBox>,
}

impl Cell<'_> {
    fn text(&self) -> String {
        cell_text(self.label_box, &self.value_boxes)
    }

    /// The tile's field when it feeds a `ShotData` key.
    fn stored(&self) -> Option<&ProfileField> {
        self.field.filter(|field| field.target.is_some())
    }
}

/// Extract shot metrics from one screen's recognized text: the labels located by the tie rule
/// ([`TIE_MARGIN`]), then everything under them read as the frozen parser reads it.
///
/// When the profile opts in, boxes holding two merged labels are split first ([`split_merged`]),
/// and every step after, the title check included, sees the split list.
pub fn parse_screen(boxes: &[TextBox], profile: &DeviceProfile) -> ParsedShot {
    let boxes: Cow<[TextBox]> = if profile.split_merged_labels {
        Cow::Owned(split_merged(boxes, profile))
    } else {
        Cow::Borrowed(boxes)
    };
    read_labels(&boxes, profile, assign_labels(&boxes, profile))
}

/// The boxes with each one that holds two merged labels replaced, where it stood, by the two. The M42
/// plan's call 6, with no Python twin. [M42 P4]
///
/// 1. **Only a box that matches no label is tried**: one [`DeviceProfile::field_for`] places is a
///    label already, and splitting it would make two labels of one.
/// 2. **Each cut `k` in `1..n`** of its text normalized with the spaces taken out (`despace`, `n`
///    letters) scores each half against every field, label and aliases, by `field_for`, so a half
///    is "a label" on exactly the terms a whole box is, and a half scoring the same against two
///    fields names the later one, as a box does.
/// 3. **A cut qualifies when both halves reach [`LABEL_MATCH_THRESHOLD`] and name different
///    fields.** One label twice (`CARRYCARRY`) is not two tiles.
/// 4. **The qualifying cut with the highest `min(left, right)` wins**, the weaker half deciding
///    because a split is only as good as its worse label; **ties go to the smallest `k`**.
/// 5. **The two boxes share the box's width in proportion to their letters** (the module doc says
///    why that is close enough): the left keeps `x` and takes `width * k / n`, the right starts
///    where it ends and takes the rest. Both keep `y`, `height`, `confidence` and `rgb`, and their
///    texts are the two halves.
///
/// What a split box became is not reported: its halves are labels, located or tied like any other,
/// and a box no cut qualifies for is left as it came.
fn split_merged(boxes: &[TextBox], profile: &DeviceProfile) -> Vec<TextBox> {
    let mut split = Vec::with_capacity(boxes.len());
    for text_box in boxes {
        if profile.field_for(&text_box.text).is_some() {
            split.push(text_box.clone());
            continue;
        }
        // `normalize_label` keeps ASCII letters and digits alone, so every `k` is a char boundary.
        let letters = despace(&text_box.text);
        let Some(k) = merged_cut(&letters, profile) else {
            split.push(text_box.clone());
            continue;
        };
        let (left, right) = letters.split_at(k);
        let left_width = text_box.width * k as f64 / letters.len() as f64;
        split.push(TextBox {
            text: left.to_string(),
            width: left_width,
            ..text_box.clone()
        });
        split.push(TextBox {
            text: right.to_string(),
            x: text_box.x + left_width,
            width: text_box.width - left_width,
            ..text_box.clone()
        });
    }
    split
}

/// Where `letters` reads as two different labels, by [`split_merged`]'s steps 2 to 4, or `None`.
fn merged_cut(letters: &str, profile: &DeviceProfile) -> Option<usize> {
    let mut best: Option<(usize, f64)> = None;
    for k in 1..letters.len() {
        let (left, right) = letters.split_at(k);
        let (Some(left_field), Some(right_field)) =
            (profile.field_for(left), profile.field_for(right))
        else {
            continue;
        };
        if std::ptr::eq(left_field, right_field) {
            continue;
        }
        let weaker = left_field.matches(left).min(right_field.matches(right));
        // Strictly greater, so an equal later cut leaves the earlier one standing.
        match best {
            Some((_, standing)) if weaker <= standing => {}
            _ => best = Some((k, weaker)),
        }
    }
    best.map(|(k, _)| k)
}

/// Everything after the labels are located. A function of its own because, from M34 P8 to P10, the
/// faithful `frozen::parse_screen` handed it frozen Python's labels, so that one copy of the reading
/// was gated by both families of vectors; it stays the seam between locating and reading.
fn read_labels(boxes: &[TextBox], profile: &DeviceProfile, labels: Labels) -> ParsedShot {
    let mut parsed = ParsedShot::new(&profile.device);

    if labels.tiles.is_empty() {
        parsed.warnings.push(format!(
            "no {} tile labels recognized - wrong screen, or the crop failed",
            profile.device
        ));
        return parsed;
    }

    if !has_title(boxes, profile) && title_band_was_captured(boxes, &labels.tiles) {
        parsed.warnings.push(format!(
            "screen title {} not found - is this the right page?",
            pyfmt::str_repr(&profile.title)
        ));
    }

    let cells = build_cells(&labels.tiles, boxes, profile);

    let mut used: Vec<&TextBox> = Vec::new();
    for (at, cell) in cells.iter().enumerate() {
        used.push(cell.label_box);
        used.extend(cell.value_boxes.iter().copied());
        // A tile printed twice is read once, from both its cells, where the first of them stands.
        // Cells come top row first, so the first is the top one.
        let Some(field) = cell.stored().filter(|field| field.printed_twice) else {
            read_cell(cell, profile, &mut parsed);
            continue;
        };
        let same = |other: &&Cell| other.field.is_some_and(|f| std::ptr::eq(f, field));
        if cells[..at].iter().any(|earlier| same(&earlier)) {
            continue;
        }
        match cells[at + 1..].iter().find(same) {
            Some(second) => read_pair([cell, second], field, profile, &mut parsed),
            None => read_cell(cell, profile, &mut parsed),
        }
    }

    // One walk over the stored fields in tile order, so a located-and-withheld field and a missing
    // one each say so where the tile sits. The faithful labels withhold nothing, and then this is
    // `parse_screen`'s missing-label loop exactly.
    let located: BTreeSet<&str> = cells
        .iter()
        .filter_map(|cell| cell.field)
        .chain(labels.withheld.iter().map(|&(field, _)| field))
        .map(|field| field.label.as_str())
        .collect();
    for field in profile.stored_fields() {
        let target = field.target.as_ref().expect("a stored field has a target");
        if let Some((_, warning)) = labels
            .withheld
            .iter()
            .find(|(withheld, _)| withheld.label == field.label)
        {
            parsed.warnings.push(warning.clone());
        } else if !located.contains(field.label.as_str()) {
            parsed
                .warnings
                .push(format!("{NO_TILE_FOUND}{}", pyfmt::str_repr(&field.label)));
            continue;
        }
        parsed.fields_present.push(target.clone());
    }

    parsed.confidence = score(&cells, &used, &labels.withheld, profile);
    parsed
}

/// How a stored tile the screen does not carry is reported: `no tile found for 'Bounce & Roll'`.
///
/// One spelling, read by the walk in [`parse_screen`] that writes the line and by
/// [`golfer_warnings`] that drops it, so the producer and the filter cannot drift apart (§M34, the
/// M31.5 plan's carried decision 3). Frozen Python's spelling, which the Python-recorded vectors
/// hold this to.
pub const NO_TILE_FOUND: &str = "no tile found for ";

/// The warnings a golfer is shown: `warnings` without its `no tile found for '<label>'` lines, the
/// rest kept in order. `screen::golfer_warnings`. [M34 P9]
///
/// **That line, because it is about the layout and not the photo.** Every bay photo lacks `Bounce &
/// Roll` and every reference photo lacks `Impact Position V`, so the line is on every shot either
/// layout takes, and each one tells the golfer about a stat their screen never showed them, where
/// ADR-034 §2 says a golfer is never told about one. The line stays in `provenance.warnings`, beside
/// the `fields_present` that says the same thing structurally. A label the OCR damaged past
/// recognising reads the same way and is dropped with the rest: the line cannot tell the two apart,
/// and on every stored photo it is the layout (`tests/golfer_warnings.rs`).
///
/// **And only that line.** Every other one is about a tile the golfer can see on their own screen:
/// - `label tie between …` is a tile that was there and was not read, which M35 and M37 grade
///   `misread`, and a retake can fix it. P8 left whether the filter drops it to this phase, and it
///   does not: a stat the golfer can see, gone with no reason given, reads as a bug;
/// - `no value text under the label`, `could not read a number`, a missing direction word, a failed
///   cross-check and the preprocessing notes each say why a number they can see is missing or
///   doubted.
///
/// Every Rust surface that shows warnings calls this: M38's review screen and M29's `rmcp` shot
/// view. Frozen Python goes on showing the line until M40 (§M34).
pub fn golfer_warnings(warnings: &[String]) -> Vec<String> {
    warnings
        .iter()
        .filter(|warning| !warning.starts_with(NO_TILE_FOUND))
        .cloned()
        .collect()
}

// --- locating tiles ------------------------------------------------------------------------------

/// Is the screen's title anywhere on it, compared with the spaces taken out?
///
/// HD Golf sets the title in wide-tracked capitals and PaddleOCR returns it as one token,
/// `SHOTDATA`, so a substring test against `SHOT DATA` fails on every real photo. The letters
/// identify the page; the kerning the OCR infers between them is evidence of nothing.
fn has_title(boxes: &[TextBox], profile: &DeviceProfile) -> bool {
    let want = despace(&profile.title);
    boxes
        .iter()
        .any(|text_box| despace(&text_box.text).contains(&want))
}

fn despace(text: &str) -> String {
    normalize_label(text).replace(' ', "")
}

/// Was there anything above the top row of tiles for the title to be *in*?
///
/// A crop to the screen can land below the title, and a title missing from a frame that starts at
/// the first tile row is a fact about the crop, not the page. So the check is "we looked where it
/// would be and it was not there", the only form of it that is evidence.
fn title_band_was_captured(boxes: &[TextBox], labels: &[Located<'_>]) -> bool {
    let top_of_grid = labels
        .iter()
        .map(|&(_, index)| boxes[index].y)
        .reduce(first_min)
        .expect("called only once a label is found");
    boxes
        .iter()
        .any(|text_box| text_box.bottom() <= top_of_grid)
}

/// The tie rule: boxes assigned to labels one-to-one, and a label read only where the evidence
/// decides its box. The M34 plan's call 8, which is this:
///
/// 1. **A candidate is a (box, field) pair scoring at least [`LABEL_MATCH_THRESHOLD`]**, with the
///    matcher, the threshold and the normalisation unchanged. A box may be a candidate for several
///    fields (`Impact Position` scores 0.9375 against `Impact Position V`), which is the point:
///    frozen Python asked each box for its single best field, and a box whose best field is taken
///    had nowhere else to go.
/// 2. **Candidates that share a box or a field are one tie**, and each is decided alone: the
///    connected components of the candidate graph, every one of them a single box and a single
///    field on a clean screen.
/// 3. **Within a tie, every one-to-one assignment is totalled** (a box takes at most one field and a
///    field at most one box, and either may take none), and the best total found. Every assignment
///    within [`TIE_MARGIN`] of it is one the evidence cannot rule out.
/// 4. **A field is read only if every one of those gives it the same box.** Otherwise it is
///    withheld: located, so in `fields_present`, but with no `raw_fields` entry and no value, and a
///    warning naming the boxes it could have been (decision 3). A field none of them gives a box is
///    not located at all, and is `no tile found` as before.
/// 5. **Every box any of them assigns is a label box.** One that is not resolved to a field bounds
///    its neighbours' columns as a boundary-only tile does, and is never read as anyone's value
///    text. That is what stops `2026-08-10-1`'s `CENTER` spilling: the box it sits under is still a
///    label, even unread.
///
/// Two boxes that fit one label equally well (two `Carry` boxes, decision 6) are the same ambiguity
/// as two labels that fit two boxes crosswise, and need no rule of their own: the two assignments
/// tie, and the label is withheld. A tie bigger than [`TIE_CAP`] is withheld whole.
///
/// **Except a field `printed_twice`, which takes up to two boxes** (the M42 plan's call 7), so two
/// boxes that fit it are one assignment giving it both, not two rivals. An assignment is each box's
/// field, so two that differ only in which of the field's boxes is which are the same assignment,
/// and cannot tie with each other. Step 4 then compares the *set* of boxes each near-best
/// assignment gives a field; a third box that fits it makes three sets of two, and the field is
/// withheld as any tied one is. Every other field takes one, and its set is the one box or none.
///
/// The tiles come out in box order. Row grouping sorts them by position and keeps equal positions
/// in the order given, which is the only thing that order reaches.
fn assign_labels<'a>(boxes: &[TextBox], profile: &'a DeviceProfile) -> Labels<'a> {
    // Each box's candidates, as (field index, score), in tile order.
    let candidates: Vec<Vec<(usize, f64)>> = boxes
        .iter()
        .map(|text_box| {
            profile
                .fields
                .iter()
                .enumerate()
                .map(|(field, profile_field)| (field, profile_field.matches(&text_box.text)))
                .filter(|&(_, score)| score >= LABEL_MATCH_THRESHOLD)
                .collect()
        })
        .collect();

    let mut labels = Labels {
        tiles: Vec::new(),
        withheld: Vec::new(),
    };
    for tie in ties(&candidates) {
        let fields: Vec<&ProfileField> = tie.fields.iter().map(|&f| &profile.fields[f]).collect();
        if tie.boxes.len() > TIE_CAP || tie.fields.len() > TIE_CAP {
            for field in fields {
                let warning = format!(
                    "{}: label tie among {} boxes and {} labels, more than the tie rule weighs - \
                     not read",
                    field.label,
                    tie.boxes.len(),
                    tie.fields.len()
                );
                labels.withheld.push((field, warning));
            }
            labels
                .tiles
                .extend(tie.boxes.iter().map(|&index| (None, index)));
            continue;
        }

        let capacity: Vec<u8> = fields.iter().copied().map(boxes_taken).collect();
        let decided = decide(&tie, &candidates, &capacity);
        let mut resolved: BTreeSet<usize> = BTreeSet::new();
        for (field, outcomes) in fields.iter().copied().zip(&decided.outcomes) {
            let mut each = outcomes.iter();
            match (each.next(), each.next()) {
                // One set of boxes, the empty set included: decided.
                (Some(given), None) => {
                    labels
                        .tiles
                        .extend(given.iter().map(|&index| (Some(field), index)));
                    resolved.extend(given);
                }
                _ => {
                    let warning = format!(
                        "{}: label tie between {} - not read",
                        field.label,
                        tied_boxes(outcomes, boxes)
                    );
                    labels.withheld.push((field, warning));
                }
            }
        }
        labels.tiles.extend(
            decided
                .label_boxes
                .difference(&resolved)
                .map(|&index| (None, index)),
        );
    }
    labels.tiles.sort_by_key(|&(_, index)| index);
    labels
}

/// How many label boxes `field` may take: two for a tile printed twice, one for any other.
fn boxes_taken(field: &ProfileField) -> u8 {
    if field.printed_twice {
        2
    } else {
        1
    }
}

/// One connected component of the candidate graph: boxes and the fields they compete for, each by
/// index and ascending.
struct Tie {
    boxes: Vec<usize>,
    fields: Vec<usize>,
}

/// The candidate graph's components, each found from its first box.
fn ties(candidates: &[Vec<(usize, f64)>]) -> Vec<Tie> {
    let mut seen = vec![false; candidates.len()];
    let mut ties = Vec::new();
    for start in 0..candidates.len() {
        if seen[start] || candidates[start].is_empty() {
            continue;
        }
        seen[start] = true;
        let mut boxes = vec![start];
        let mut fields: BTreeSet<usize> = BTreeSet::new();
        let mut next = 0;
        while next < boxes.len() {
            let at = boxes[next];
            next += 1;
            for &(field, _) in &candidates[at] {
                if !fields.insert(field) {
                    continue;
                }
                for (other, theirs) in candidates.iter().enumerate() {
                    if !seen[other] && theirs.iter().any(|&(f, _)| f == field) {
                        seen[other] = true;
                        boxes.push(other);
                    }
                }
            }
        }
        boxes.sort_unstable();
        ties.push(Tie {
            boxes,
            fields: fields.into_iter().collect(),
        });
    }
    ties
}

/// What the assignments within [`TIE_MARGIN`] of the best say, for one tie.
struct Decided {
    /// Per field of the tie, in its order: every set of boxes some near-best assignment gives it,
    /// each ascending, and the empty one where one gives it none. A set holds one box at most
    /// unless the field is printed twice.
    outcomes: Vec<BTreeSet<Vec<usize>>>,
    /// Every box some near-best assignment gives a field.
    label_boxes: BTreeSet<usize>,
}

/// Enumerate the tie's assignments twice: once for the best total, once to collect what every
/// assignment within [`TIE_MARGIN`] of it gives each field. Two walks rather than one walk that
/// keeps every assignment, because the cap allows over a million of them. `capacity` is how many
/// boxes each of the tie's fields may take, in its order ([`boxes_taken`]).
fn decide(tie: &Tie, candidates: &[Vec<(usize, f64)>], capacity: &[u8]) -> Decided {
    // Each box's candidates as (position in `tie.fields`, score).
    let local: Vec<Vec<(usize, f64)>> = tie
        .boxes
        .iter()
        .map(|&index| {
            candidates[index]
                .iter()
                .map(|&(field, score)| {
                    let at = tie
                        .fields
                        .binary_search(&field)
                        .expect("a field of this tie");
                    (at, score)
                })
                .collect()
        })
        .collect();

    let mut best = f64::NEG_INFINITY;
    assignments(&local, capacity, &mut |_, total| {
        if total > best {
            best = total;
        }
    });

    let mut decided = Decided {
        outcomes: vec![BTreeSet::new(); tie.fields.len()],
        label_boxes: BTreeSet::new(),
    };
    assignments(&local, capacity, &mut |assignment, total| {
        if best - total > TIE_MARGIN {
            return;
        }
        // Boxes are visited in ascending order, so each field's set is built sorted, and one
        // assignment has one spelling of it.
        let mut given = vec![Vec::new(); tie.fields.len()];
        for (at, field) in assignment.iter().enumerate() {
            if let Some(field) = *field {
                given[field].push(tie.boxes[at]);
                decided.label_boxes.insert(tie.boxes[at]);
            }
        }
        for (outcomes, boxes) in decided.outcomes.iter_mut().zip(given) {
            outcomes.insert(boxes);
        }
    });
    decided
}

/// Visit every assignment of boxes to fields, as each box's field (or `None`) and the total of the
/// scores taken, summed in box order. A box takes one field at most, and field `f` takes
/// `capacity[f]` boxes at most: one, or two for a tile printed twice, which is what makes every
/// other assignment one-to-one. Fields are positions below [`TIE_CAP`], so an array that long
/// counts what each has taken.
fn assignments(
    local: &[Vec<(usize, f64)>],
    capacity: &[u8],
    visit: &mut impl FnMut(&[Option<usize>], f64),
) {
    fn walk(
        local: &[Vec<(usize, f64)>],
        capacity: &[u8],
        at: usize,
        taken: [u8; TIE_CAP],
        assignment: &mut Vec<Option<usize>>,
        total: f64,
        visit: &mut impl FnMut(&[Option<usize>], f64),
    ) {
        if at == local.len() {
            visit(assignment, total);
            return;
        }
        assignment.push(None);
        walk(local, capacity, at + 1, taken, assignment, total, visit);
        assignment.pop();
        for &(field, score) in &local[at] {
            if taken[field] < capacity[field] {
                let mut now = taken;
                now[field] += 1;
                assignment.push(Some(field));
                walk(
                    local,
                    capacity,
                    at + 1,
                    now,
                    assignment,
                    total + score,
                    visit,
                );
                assignment.pop();
            }
        }
    }
    walk(
        local,
        capacity,
        0,
        [0; TIE_CAP],
        &mut Vec::with_capacity(local.len()),
        0.0,
        visit,
    );
}

/// `'Impact Position' and 'ImpactPosition'`: the texts of the boxes a withheld field could have
/// been, in box order, and `no box` last where a near-best assignment gives it none.
fn tied_boxes(outcomes: &BTreeSet<Vec<usize>>, boxes: &[TextBox]) -> String {
    let given: BTreeSet<usize> = outcomes.iter().flatten().copied().collect();
    let mut named: Vec<String> = given
        .iter()
        .map(|&index| pyfmt::str_repr(&boxes[index].text))
        .collect();
    if outcomes.iter().any(Vec::is_empty) {
        named.push("no box".to_string());
    }
    match named.split_last() {
        Some((last, rest)) if !rest.is_empty() => format!("{} and {last}", rest.join(", ")),
        _ => named.join(""),
    }
}

/// Cluster labels into screen rows by vertical position, each row then ordered left to right.
///
/// Both sorts are stable, as Python's is, so labels on an equal center keep the order they came in.
fn group_rows<'a>(labels: &[Located<'a>], boxes: &[TextBox]) -> Vec<Vec<Located<'a>>> {
    let mut sorted = labels.to_vec();
    sorted.sort_by(|a, b| by_key(boxes[a.1].center_y(), boxes[b.1].center_y()));

    let mut rows: Vec<Vec<Located>> = Vec::new();
    for item in sorted {
        let text_box = &boxes[item.1];
        let row = rows.iter().position(|row| {
            let reference = &boxes[row.last().expect("a row is never empty").1];
            let tolerance = first_max(reference.height, text_box.height) * ROW_TOLERANCE;
            (text_box.center_y() - reference.center_y()).abs() <= tolerance
        });
        match row {
            Some(at) => rows[at].push(item),
            None => rows.push(vec![item]),
        }
    }
    for row in &mut rows {
        row.sort_by(|a, b| by_key(boxes[a.1].center_x(), boxes[b.1].center_x()));
    }
    rows
}

/// Derive each tile's bounds from its neighbours, then collect the text inside it.
fn build_cells<'a>(
    labels: &[Located<'a>],
    boxes: &'a [TextBox],
    profile: &DeviceProfile,
) -> Vec<Cell<'a>> {
    let label_boxes: BTreeSet<usize> = labels.iter().map(|&(_, index)| index).collect();
    let rows = group_rows(labels, boxes);
    let mut cells = Vec::new();

    for (row_index, row) in rows.iter().enumerate() {
        let next_row_top = rows.get(row_index + 1).and_then(|next| {
            next.iter()
                .map(|&(_, index)| boxes[index].y)
                .reduce(first_min)
        });
        for (col_index, &(field, index)) in row.iter().enumerate() {
            // `field` is `None` for a box the tie rule left unresolved: a label all the same, so
            // its column is bounded and bounds its neighbours exactly as a resolved one does.
            let label_box = &boxes[index];
            // Column: halfway to the neighbouring tiles, so a wide value can overhang its label
            // without being claimed by the tile next door.
            let left = if col_index > 0 {
                (boxes[row[col_index - 1].1].right() + label_box.x) / 2.0
            } else {
                label_box.x - label_box.width
            };
            let right = if col_index + 1 < row.len() {
                (label_box.right() + boxes[row[col_index + 1].1].x) / 2.0
            } else {
                label_box.right() + label_box.width
            };
            // Rows: down to the next row of labels, but never further than a value could plausibly
            // stack (number, unit, qualifier).
            let mut bottom = label_box.bottom() + label_box.height * profile.value_span_ratio;
            if let Some(top) = next_row_top {
                bottom = first_min(bottom, top);
            }

            let value_boxes = boxes
                .iter()
                .enumerate()
                .filter(|(at, text_box)| {
                    !label_boxes.contains(at)
                        && left <= text_box.center_x()
                        && text_box.center_x() <= right
                        && label_box.bottom() <= text_box.center_y()
                        && text_box.center_y() <= bottom
                })
                .map(|(_, text_box)| text_box)
                .collect();
            cells.push(Cell {
                field,
                label_box,
                value_boxes,
            });
        }
    }
    cells
}

// --- reading a tile ------------------------------------------------------------------------------

fn read_cell(cell: &Cell, profile: &DeviceProfile, parsed: &mut ParsedShot) {
    // A boundary-only tile (the settings gear), or a box the tie rule would not resolve: located so
    // its neighbours stay in their columns, and never read.
    let Some(field) = cell.stored() else {
        return;
    };
    let text = cell.text();
    parsed.raw_fields.insert(field.label.clone(), text.clone());
    let reading = read_text(&text, &cell.value_boxes, field, profile);
    reading.write(field, parsed);
}

/// A tile printed twice, read from its two cells, top first: the M42 plan's call 7, with no Python
/// twin. [M42 P5]
///
/// Each cell is read as [`read_cell`] reads a tile, and the two are then combined:
///
/// - **both give a number, and it is the same number**: it is stored, with the top cell's
///   warnings (a cell's own warnings are only ever about its sign);
/// - **both give a number, and they differ**: nothing is stored, and the warning says what each
///   cell held. One copy is a misread and nothing says which, so a value would be a coin-flip from
///   a wrong one (ADR-010 §2);
/// - **one gives a number** (the other empty, blank or unreadable): it is stored, with its own
///   cell's warnings. The other's are dropped, because a line saying a number is missing under a
///   tile that holds one would tell the golfer something false;
/// - **neither does**: each cell's warnings in turn, a line the top cell already raised not
///   repeated (two empty cells are one `no value text under the label`).
///
/// `raw_fields` holds the two texts joined by `" | "`, top first, so a record says what both
/// copies held and a disagreement can be audited from it.
fn read_pair(
    cells: [&Cell; 2],
    field: &ProfileField,
    profile: &DeviceProfile,
    parsed: &mut ParsedShot,
) {
    let texts = cells.map(Cell::text);
    parsed
        .raw_fields
        .insert(field.label.clone(), format!("{} | {}", texts[0], texts[1]));
    let [top, bottom] =
        [0, 1].map(|at| read_text(&texts[at], &cells[at].value_boxes, field, profile));
    match (top.number(), bottom.number()) {
        (Some(a), Some(b)) if a == b => top.write(field, parsed),
        (Some(_), Some(_)) => parsed.warnings.push(format!(
            "{}: printed twice as {} and {} - not stored",
            field.label,
            pyfmt::str_repr(&texts[0]),
            pyfmt::str_repr(&texts[1])
        )),
        (Some(_), None) => top.write(field, parsed),
        (None, Some(_)) => bottom.write(field, parsed),
        (None, None) => {
            let raised = top.warnings.clone();
            top.write(field, parsed);
            parsed.warnings.extend(
                bottom
                    .warnings
                    .into_iter()
                    .filter(|warning| !raised.contains(warning)),
            );
        }
    }
}

/// What one cell's text gives a stored tile, before any of it is written: the value, if one was
/// read, and the warnings reading it raised.
///
/// A value of its own, rather than writes into the [`ParsedShot`] as `parser.py`'s `_read_cell`
/// makes them, because a tile printed twice decides what to keep only once both its cells are read
/// ([`read_pair`]). [`read_cell`] writes it straight back, so a tile read once is read as before.
struct Reading {
    value: Option<FieldValue>,
    warnings: Vec<String>,
}

impl Reading {
    fn number(&self) -> Option<f64> {
        match self.value {
            Some(FieldValue::Number(number)) => Some(number),
            _ => None,
        }
    }

    /// The warnings, then the value under `field`'s target.
    fn write(self, field: &ProfileField, parsed: &mut ParsedShot) {
        parsed.warnings.extend(self.warnings);
        if let Some(value) = self.value {
            let target = field.target.as_ref().expect("a stored field has a target");
            parsed.values.insert(target.clone(), value);
        }
    }
}

/// `_read_cell` from its text on: empty, blank, words, or a number with its sign.
///
/// `value_boxes` are the boxes `text` was built from, which only a `lit_sign` tile reads (for the
/// colour of its letters, [`lit_sign`]); every other tile is read from its text alone, as before.
fn read_text(
    text: &str,
    value_boxes: &[&TextBox],
    field: &ProfileField,
    profile: &DeviceProfile,
) -> Reading {
    let mut reading = Reading {
        value: None,
        warnings: Vec::new(),
    };
    if text.is_empty() {
        reading
            .warnings
            .push(format!("{}: no value text under the label", field.label));
        return reading;
    }
    if is_blank(text, &profile.blank_markers) {
        return reading; // the device measured nothing here: a correct read of an absent metric
    }

    if field.kind == FieldKind::Text {
        let upper = pyfmt::upper(text);
        let words: Vec<&str> = pyfmt::split(&upper).collect();
        reading.value = Some(FieldValue::Text(words.join(" ")));
        return reading;
    }

    let Some((mut magnitude, matched)) = first_number(text) else {
        reading.warnings.push(format!(
            "{}: could not read a number from {}",
            field.label,
            pyfmt::str_repr(text)
        ));
        return reading;
    };

    if !field.lit_sign.is_empty() {
        // The digits' own sign is discarded: a tile that signs by its letters prints the number
        // unsigned, so a `-` there is a stray mark, not a convention (the M42 plan's call 5).
        magnitude = magnitude.abs();
        // Zero needs no sign, and `abs` has already made a printed `-0` the `0.0` stored.
        if magnitude != 0.0 {
            let Some(sign) = lit_sign(&field.lit_sign, value_boxes) else {
                // Withheld, never stored unsigned: an unsigned number on a signed field is a
                // wrong one half the time (ADR-010 §2).
                let tokens: Vec<&str> = field.lit_sign.iter().map(|t| t.token.as_str()).collect();
                reading.warnings.push(format!(
                    "{}: neither {} could be read as lit - sign unknown, not stored",
                    field.label,
                    tokens.join(" nor ")
                ));
                return reading;
            };
            magnitude *= sign as f64;
        }
    } else if !field.sign_tokens.is_empty() || field.printed_sign.is_some() {
        // Python's `str.replace(old, new, 1)`: the first occurrence, which is the match itself,
        // since any earlier occurrence would have been the leftmost match.
        let sign = sign_from(&text.replacen(matched, " ", 1), field);
        let printed = field
            .printed_sign
            .filter(|_| matched.starts_with(['+', '-']));
        match (sign, printed) {
            // A direction word beats a printed sign: it states the convention in the device's own
            // words, where a bare `-` states one only if we already know which way it points.
            (Some(sign), _) => magnitude = magnitude.abs() * sign as f64,
            // The tile printed its own sign and the profile records how that polarity relates to
            // the contract, so nothing is unknown and nothing is warned.
            (None, Some(printed)) => magnitude *= printed as f64,
            (None, None) => reading.warnings.push(format!(
                "{}: no direction word in {} - sign unknown, storing the magnitude as printed",
                field.label,
                pyfmt::str_repr(text)
            )),
        }
    }
    // After the sign, so the conversion sees the signed number and a zero the sign rule made `+0.0`
    // stays `+0.0`; `raw_fields` already holds the text as printed (the M42 plan's call 8).
    if let Some(unit) = field.printed_unit {
        magnitude = unit.to_yards(magnitude);
    }
    reading.value = Some(FieldValue::Number(magnitude));
    reading
}

/// `_Cell.text`: a tile's value text, read line by line, left to right.
///
/// Lines are `int(center_y // bucket)`, with `//` CPython's ([`pyfmt::floor_div`]): `21.0 // 4.2`
/// is `4.0` where `(21.0 / 4.2).floor()` is `5.0`, and a box on a different line is read in a
/// different order. The key is kept as the quotient rather than cast, because `int()` changes
/// nothing a comparison sees: the quotient is already integral, and `-0.0 == 0.0`. Each box is
/// stripped with Python's whitespace and dropped when nothing is left.
pub fn cell_text(label_box: &TextBox, value_boxes: &[&TextBox]) -> String {
    let bucket = first_max(label_box.height * LINE_BUCKET, 1e-6);
    let line = |text_box: &TextBox| {
        (
            pyfmt::floor_div(text_box.center_y(), bucket),
            text_box.center_x(),
        )
    };
    let mut ordered = value_boxes.to_vec();
    ordered.sort_by(|a, b| line(a).partial_cmp(&line(b)).unwrap_or(Ordering::Equal));
    ordered
        .iter()
        .map(|text_box| pyfmt::strip(&text_box.text))
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// `_is_blank`: `text` equals one of the markers once `\s+` is taken out of both.
///
/// So `- - -` and `---` are one marker, and a marker that is all whitespace makes an empty cell
/// blank. Takes the markers rather than the profile, which is all `parser.py` reads of it.
pub fn is_blank(text: &str, blank_markers: &[String]) -> bool {
    let compact = pyfmt::strip_space(text);
    blank_markers
        .iter()
        .any(|marker| compact == pyfmt::strip_space(marker))
}

/// `_first_number`: the first number in `text`, with the exact substring it came from.
///
/// The first match of `[-+]?\d[\d,]*(?:\.\d+)?`, then `_THOUSANDS` (a `,` between a digit and
/// exactly three digits) dropped, every other `,` read as a decimal point, and `float()` on the
/// result, which refuses `1,2,3` and `1,2.5`. `None` where nothing matches or `float()` refuses.
pub fn first_number(text: &str) -> Option<(f64, &str)> {
    let matched = find_number(text)?;
    let chars: Vec<char> = matched.chars().collect();
    let is_digit = |at: usize| chars.get(at).copied().and_then(decimal_value).is_some();

    let mut ascii = String::with_capacity(chars.len());
    for (at, &c) in chars.iter().enumerate() {
        if c == ',' {
            // `(?<=\d),(?=\d{3}\b)`, run over the matched text, where the only thing that can
            // follow three digits is the end, a `,`, a `.` or a fourth digit. So `\b` is "not
            // followed by a digit", and `5,991rpm` loses its comma exactly as `5,991` does.
            let thousands = at > 0
                && is_digit(at - 1)
                && (1..=3).all(|k| is_digit(at + k))
                && !is_digit(at + 4);
            if !thousands {
                ascii.push('.');
            }
        } else if let Some(value) = decimal_value(c) {
            // `float()` reads any decimal digit as its value, by the same table `\d` matched with.
            ascii.push(char::from_digit(value, 10).expect("a decimal digit is 0-9"));
        } else {
            ascii.push(c);
        }
    }
    // Only a sign, ASCII digits and `.` reach this, where Rust's grammar and `float()`'s agree,
    // and both round correctly.
    ascii.parse().ok().map(|value| (value, matched))
}

/// The leftmost match of `[-+]?\d[\d,]*(?:\.\d+)?` in `text`.
///
/// Greedy throughout, and nothing after the sign can backtrack into a different answer: the
/// optional fraction either matches whole or the match ends before it.
fn find_number(text: &str) -> Option<&str> {
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let digit_at = |at: usize| {
        chars
            .get(at)
            .is_some_and(|&(_, c)| decimal_value(c).is_some())
    };
    let is_at = |at: usize, want: char| chars.get(at).is_some_and(|&(_, c)| c == want);

    for start in 0..chars.len() {
        let mut at = start;
        if (is_at(at, '-') || is_at(at, '+')) && digit_at(at + 1) {
            at += 1;
        }
        if !digit_at(at) {
            continue;
        }
        at += 1;
        while digit_at(at) || is_at(at, ',') {
            at += 1;
        }
        if is_at(at, '.') && digit_at(at + 1) {
            at += 2;
            while digit_at(at) {
                at += 1;
            }
        }
        let end = chars.get(at).map_or(text.len(), |&(byte, _)| byte);
        return Some(&text[chars[start].0..end]);
    }
    None
}

/// `_sign_from`: the direction word left over once the number is taken out (`O>I`, `Closed`).
///
/// Both sides are upper-cased with Python's `upper` and cut to `A-Z0-9>`, which is ASCII whatever
/// came in. Rules are tried in order, and the first token that hits wins. A one-letter token must
/// be the whole residual, or `L` would match inside `CLOSED`; a token that cuts to nothing is
/// skipped.
pub fn sign_from(residual: &str, field: &ProfileField) -> Option<i64> {
    let compact = sign_text(residual);
    if compact.is_empty() {
        return None;
    }
    for rule in &field.sign_tokens {
        for token in &rule.tokens {
            let wanted = sign_text(token);
            if wanted.is_empty() {
                continue;
            }
            // ASCII by construction, so a byte length is Python's `len`.
            let hit = if wanted.len() == 1 {
                compact == wanted
            } else {
                compact.contains(&wanted)
            };
            if hit {
                return Some(rule.sign);
            }
        }
    }
    None
}

/// `_SIGN_NOISE.sub("", text.upper())`: letters, digits and the `>` of `O>I`. The class has no
/// `IGNORECASE`, so it is the ASCII ranges only, and `Ō` survives `upper` and is then dropped.
fn sign_text(text: &str) -> String {
    pyfmt::upper(text)
        .chars()
        .filter(|&c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '>')
        .collect()
}

/// Whether a glyph of colour `rgb` (`TextBox::rgb`) is a lit sign letter: bright in red and green
/// ([`LIT_MIN_BRIGHTNESS`]) and short of blue ([`LIT_BLUE_RATIO`]), which is yellow. The M42 plan's
/// call 4, with no Python twin. [M42 P6]
///
/// Here, in `screen`, rather than in `ocr` where the colour is measured, so the phone (M38), whose
/// boxes come from Vision and not from `ocr`, decides lit by the same rule from the same number.
pub fn is_lit(rgb: [f64; 3]) -> bool {
    let [red, green, blue] = rgb;
    let floor = first_min(red, green);
    floor >= LIT_MIN_BRIGHTNESS && blue <= LIT_BLUE_RATIO * floor
}

/// What a sign letter's boxes in one cell say of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Letter {
    /// No box in the cell holds it.
    Absent,
    Lit,
    Unlit,
    /// A box of it has no colour, or two boxes of it disagree.
    Unknown,
}

/// The sign a `lit_sign` tile's letters give, or `None` when they cannot decide it: the M42 plan's
/// call 5, with no Python twin. [M42 P6]
///
/// A **letter box** is a value box of the tile's cell whose text, stripped and upper-cased, is one
/// of the pair's tokens exactly, so `DD` (read on `IMG_2893`, M42 P3's finding) is no letter. Then:
///
/// | the two letters | sign |
/// |---|---|
/// | one lit, the other absent or unlit | the lit one's |
/// | one unlit, the other absent | the other one's |
/// | both lit, both unlit, or both absent | none |
/// | either with a box that has no colour | none |
///
/// The device lights exactly one, so one unlit letter alone still says which: the other. Two lit,
/// or two unlit, is a photo that contradicts the screen, and nothing says which letter to believe.
///
/// **Two boxes of one letter** are not a case the plan names, and the one this reads least into:
/// they count as one letter when they agree, lit or unlit, and as [`Letter::Unknown`] when they
/// do not, since one of them is then a misreading and nothing says which.
fn lit_sign(pair: &[LitToken], value_boxes: &[&TextBox]) -> Option<i64> {
    let [a, b] = pair else {
        return None; // `DeviceProfile::check` holds every shipped profile to two
    };
    let letter = |token: &LitToken| {
        let mut seen = Letter::Absent;
        for text_box in value_boxes {
            if pyfmt::upper(pyfmt::strip(&text_box.text)) != token.token {
                continue;
            }
            let this = match text_box.rgb {
                Some(rgb) if is_lit(rgb) => Letter::Lit,
                Some(_) => Letter::Unlit,
                None => Letter::Unknown,
            };
            seen = match seen {
                Letter::Absent => this,
                _ if seen == this => this,
                _ => Letter::Unknown,
            };
        }
        seen
    };
    match (letter(a), letter(b)) {
        (Letter::Unknown, _) | (_, Letter::Unknown) => None,
        (Letter::Lit, Letter::Absent | Letter::Unlit) | (Letter::Absent, Letter::Unlit) => {
            Some(a.sign)
        }
        (Letter::Absent | Letter::Unlit, Letter::Lit) | (Letter::Unlit, Letter::Absent) => {
            Some(b.sign)
        }
        (Letter::Lit, Letter::Lit)
        | (Letter::Unlit, Letter::Unlit)
        | (Letter::Absent, Letter::Absent) => None,
    }
}

// --- confidence ----------------------------------------------------------------------------------

/// Blend the three failure modes into one `0.0`-`1.0` trust score.
///
/// The OCR mean is [`pyfmt::sum`], compensated, and the weighted sum is left to right with no fused
/// multiply-add, as CPython evaluates it. Both matter only in the last bit, and the last bit is
/// what `round(conf, 3)` and the `< min_confidence` test downstream read.
///
/// **A withheld field counts as a found label and as no value** (M34 P8): it was located, which is
/// what label coverage measures, and nothing was read from it, which is what value coverage does.
/// Its boxes are in `used` as every located box is. With nothing withheld this is frozen Python's
/// score exactly, which the Python-recorded vectors held it to until M34 P10 re-recorded them.
fn score(
    cells: &[Cell],
    used: &[&TextBox],
    withheld: &[(&ProfileField, String)],
    profile: &DeviceProfile,
) -> f64 {
    let expected = profile.stored_fields().len();
    if expected == 0 {
        return 0.0;
    }

    let stored: Vec<&ProfileField> = cells.iter().filter_map(Cell::stored).collect();
    let found_labels: BTreeSet<&str> = stored
        .iter()
        .copied()
        .chain(withheld.iter().map(|&(field, _)| field))
        .filter(|field| field.target.is_some())
        .map(|field| field.label.as_str())
        .collect();
    // Counted by field, not by cell: a tile printed twice is one value however many of its cells
    // hold text (M42 P5). Every other field has one cell at most, so this is the count of cells it
    // always was.
    let read_values = cells
        .iter()
        .filter(|cell| !cell.text().is_empty())
        .filter_map(Cell::stored)
        .map(std::ptr::from_ref)
        .collect::<BTreeSet<*const ProfileField>>()
        .len();
    let ocr: Vec<f64> = used.iter().map(|text_box| text_box.confidence).collect();

    let label_coverage = found_labels.len() as f64 / expected as f64;
    let value_coverage = first_min(read_values as f64 / expected as f64, 1.0);
    let ocr_confidence = if ocr.is_empty() {
        0.0
    } else {
        pyfmt::sum(&ocr) / ocr.len() as f64
    };

    let score = W_LABELS * label_coverage + W_VALUES * value_coverage + W_OCR * ocr_confidence;
    first_max(0.0, first_min(1.0, score))
}

// --- CPython's semantics, where std has a near miss ----------------------------------------------

/// Python's `min(a, b)`: `a` unless `b` is strictly smaller. `f64::min` returns the other argument
/// where one is a NaN, and Python returns whichever came first, so only this form is the same
/// function. No vector reaches the difference: box geometry is never a NaN.
///
/// `pub(crate)` because `validate` clamps with the same two (M34 P6), and a second copy is a
/// second thing to drift.
pub(crate) fn first_min(a: f64, b: f64) -> f64 {
    if b < a {
        b
    } else {
        a
    }
}

/// Python's `max(a, b)`: `a` unless `b` is strictly larger. A longer `max(a, b, c)` is this folded
/// from the left, which is how CPython walks its arguments.
pub(crate) fn first_max(a: f64, b: f64) -> f64 {
    if b > a {
        b
    } else {
        a
    }
}

/// A sort key compared as Python's `<` compares two floats, so a stable sort keeps equal keys in
/// the order they came, as `sorted` does. Never `total_cmp`, which orders `-0.0` before `0.0`.
fn by_key(a: f64, b: f64) -> Ordering {
    a.partial_cmp(&b).unwrap_or(Ordering::Equal)
}

/// The digit zero of every run of ten Unicode decimal digits, as CPython 3.13.3 knows them.
///
/// `\d` in a `str` pattern is `Py_UNICODE_ISDECIMAL`, which is general category `Nd`, and Unicode
/// guarantees `Nd` comes in contiguous runs of ten, ascending from zero. So a run's first code point
/// is the whole of what a table needs. Generated from the recording interpreter (Unicode 15.1:
/// `chr(c).isdecimal()` over every code point gives 680 characters in 68 runs, each digit's
/// `unicodedata.decimal` its offset from the run's start, and `re`'s `\d` matching exactly those).
/// Rust 1.87's Unicode 16.0 has more runs, and they are left out on purpose: the answer is
/// CPython's, and none is a script a launch monitor prints.
const DECIMAL_ZEROS: [u32; 68] = [
    0x30, 0x660, 0x6F0, 0x7C0, 0x966, 0x9E6, 0xA66, 0xAE6, 0xB66, 0xBE6, 0xC66, 0xCE6, 0xD66,
    0xDE6, 0xE50, 0xED0, 0xF20, 0x1040, 0x1090, 0x17E0, 0x1810, 0x1946, 0x19D0, 0x1A80, 0x1A90,
    0x1B50, 0x1BB0, 0x1C40, 0x1C50, 0xA620, 0xA8D0, 0xA900, 0xA9D0, 0xA9F0, 0xAA50, 0xABF0, 0xFF10,
    0x104A0, 0x10D30, 0x11066, 0x110F0, 0x11136, 0x111D0, 0x112F0, 0x11450, 0x114D0, 0x11650,
    0x116C0, 0x11730, 0x118E0, 0x11950, 0x11C50, 0x11D50, 0x11DA0, 0x11F50, 0x16A60, 0x16AC0,
    0x16B50, 0x1D7CE, 0x1D7D8, 0x1D7E2, 0x1D7EC, 0x1D7F6, 0x1E140, 0x1E2F0, 0x1E4F0, 0x1E950,
    0x1FBF0,
];

/// `c`'s value as a decimal digit, or `None` where `\d` would not match it.
fn decimal_value(c: char) -> Option<u32> {
    let code = u32::from(c);
    let run = DECIMAL_ZEROS.partition_point(|&zero| zero <= code);
    let value = code - DECIMAL_ZEROS[run.checked_sub(1)?];
    (value < 10).then_some(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::parse_profiles;

    fn text_box(text: &str, x: f64, y: f64, width: f64, height: f64, confidence: f64) -> TextBox {
        TextBox {
            text: text.to_string(),
            x,
            y,
            width,
            height,
            confidence,
            rgb: None,
        }
    }

    /// The table is what Unicode promises `Nd` is: ascending runs of ten that never overlap, ASCII
    /// first, and every member a numeric character to Rust as well.
    #[test]
    fn the_decimal_table_is_runs_of_ten() {
        assert_eq!(DECIMAL_ZEROS[0], u32::from('0'));
        for pair in DECIMAL_ZEROS.windows(2) {
            assert!(
                pair[1] >= pair[0] + 10,
                "{:#X} overlaps {:#X}",
                pair[0],
                pair[1]
            );
        }
        let mut count = 0;
        for &zero in &DECIMAL_ZEROS {
            for value in 0..10 {
                let c = char::from_u32(zero + value).expect("a scalar value");
                assert!(c.is_numeric(), "{c:?}");
                assert_eq!(decimal_value(c), Some(value), "{c:?}");
                count += 1;
            }
        }
        assert_eq!(count, 680, "CPython 3.13.3's `isdecimal` count");
    }

    /// Numeric to Rust and not a decimal digit to CPython: a superscript, a Roman numeral, a
    /// fraction. Each is `is_numeric` and none is `\d`.
    #[test]
    fn a_numeric_character_is_not_always_a_digit() {
        for c in [
            '\u{b2}', '\u{2167}', '\u{bd}', 'a', '/', '\u{65f}', '\u{66a}',
        ] {
            assert_eq!(decimal_value(c), None, "{c:?}");
        }
        assert_eq!(decimal_value('\u{663}'), Some(3));
        assert_eq!(decimal_value('\u{ff19}'), Some(9));
    }

    /// A one-device profile from its `fields` alone.
    fn profile(fields: &str) -> DeviceProfile {
        profile_with("", fields)
    }

    /// A one-device profile from its `fields` and profile-level `keys`, each key followed by a
    /// comma, through the same parse the shipped file takes.
    fn profile_with(keys: &str, fields: &str) -> DeviceProfile {
        parse_profiles(&format!(
            r#"{{"profiles": [{{"device": "t", "title": "T", {keys}"fields": {fields}}}]}}"#
        ))
        .unwrap()
        .remove("t")
        .unwrap()
    }

    fn read(parsed: &ParsedShot) -> Vec<(&str, &str)> {
        parsed
            .raw_fields
            .iter()
            .map(|(label, text)| (label, text.as_str()))
            .collect()
    }

    // Each test below holds frozen Python's answer, measured when it was written, for a rule the
    // committed vectors do not reach: removing the rule left every vector green (M34 P5's mutation
    // run). None of them has two boxes for one label, where the tie rule parts from frozen Python.
    // The one that turned on `by_label` went with the faithful rules in M34 P10.

    /// `Spin` overlaps `Carry`'s left end and shares its row, so `Carry`'s column starts at 15 and
    /// `Spin`'s center (20, 11) lies inside `Carry`'s cell. Only the exclusion of label boxes keeps
    /// it out; without it `Carry` reads `Spin 128.1`. Rows read left to right, so `Spin` is first.
    #[test]
    fn a_label_box_is_never_value_text() {
        let profile = profile(
            r#"[{"label": "Carry", "target": "carry_distance"},
                {"label": "Spin", "target": "spin_rate"}]"#,
        );
        let boxes = [
            text_box("Carry", 0.0, 0.0, 100.0, 10.0, 1.0),
            text_box("Spin", 10.0, 6.0, 20.0, 10.0, 1.0),
            text_box("128.1", 40.0, 20.0, 30.0, 10.0, 1.0),
        ];
        let parsed = parse_screen(&boxes, &profile);
        assert_eq!(read(&parsed), [("Spin", ""), ("Carry", "128.1")]);
        assert_eq!(parsed.confidence, 0.825);
        assert_eq!(parsed.warnings, ["Spin: no value text under the label"]);
    }

    /// `Spin Axis` prints its sign inverted (`printed_sign: -1`), so `-9.3` alone is `+9.3`, and
    /// `-9.3 L` is `-9.3` because the word wins. The committed `word-beats-printed-sign` vector
    /// reads `-9.3 R`, where the word and the inverted print agree on `+9.3`, so it cannot tell
    /// which rule answered; neither can the Python test it was recorded from.
    #[test]
    fn a_direction_word_beats_a_printed_sign_where_they_disagree() {
        let profile = profile(
            r#"[{"label": "Spin Axis", "target": "spin_axis", "printed_sign": -1,
                 "sign_tokens": [{"tokens": ["R", "RIGHT"], "sign": 1},
                                 {"tokens": ["L", "LEFT"], "sign": -1}]}]"#,
        );
        for (printed, want) in [("-9.3 \u{b0} L", -9.3), ("-9.3 \u{b0}", 9.3)] {
            let boxes = [
                text_box("Spin Axis", 0.0, 0.0, 100.0, 10.0, 1.0),
                text_box(printed, 10.0, 20.0, 60.0, 10.0, 1.0),
            ];
            let parsed = parse_screen(&boxes, &profile);
            assert_eq!(
                parsed.values.get("spin_axis"),
                Some(&FieldValue::Number(want)),
                "{printed:?}"
            );
            assert!(parsed.warnings.is_empty(), "{:?}", parsed.warnings);
        }
    }

    /// A text tile's words are split on Python's whitespace, which counts U+001C as a space where
    /// `split_whitespace` does not. `raw_fields` keeps the box as read.
    #[test]
    fn a_text_tile_splits_on_pythons_whitespace() {
        let profile = profile(r#"[{"label": "Shot Type", "target": "shot_type", "kind": "text"}]"#);
        let boxes = [
            text_box("Shot Type", 0.0, 0.0, 100.0, 10.0, 1.0),
            text_box("slight\u{1c}draw", 10.0, 20.0, 60.0, 10.0, 1.0),
        ];
        let parsed = parse_screen(&boxes, &profile);
        assert_eq!(
            parsed.values.get("shot_type"),
            Some(&FieldValue::Text("SLIGHT DRAW".to_string()))
        );
        assert_eq!(read(&parsed), [("Shot Type", "slight\u{1c}draw")]);
    }

    // --- the tie rule [M34 P8] -------------------------------------------------------------------

    fn hd_golf() -> &'static DeviceProfile {
        crate::profile::load_profile("hd_golf").expect("the shipped profile")
    }

    /// One box per text, all on one line, so only the matcher decides anything.
    fn in_a_row(texts: &[&str]) -> Vec<TextBox> {
        texts
            .iter()
            .enumerate()
            .map(|(at, text)| text_box(text, 200.0 * at as f64, 0.0, 120.0, 20.0, 1.0))
            .collect()
    }

    /// What locating found: each label box's text and the label it was resolved to (`None` for a
    /// box left unresolved), in box order, and the labels withheld.
    fn locate(
        profile: &DeviceProfile,
        boxes: &[TextBox],
    ) -> (Vec<(String, Option<String>)>, Vec<String>) {
        let labels = assign_labels(boxes, profile);
        let tiles = labels
            .tiles
            .iter()
            .map(|&(field, at)| (boxes[at].text.clone(), field.map(|f| f.label.clone())))
            .collect();
        let withheld = labels
            .withheld
            .iter()
            .map(|(field, _)| field.label.clone())
            .collect();
        (tiles, withheld)
    }

    /// `|(A→IP + B→IPV) − (A→IPV + B→IP)|`, the margin between the two ways of assigning a pair.
    fn crosswise_margin(a: &str, b: &str) -> f64 {
        let field = |label: &str| {
            hd_golf()
                .fields
                .iter()
                .find(|field| field.label == label)
                .expect("an hd_golf label")
        };
        let (ip, ipv) = (field("Impact Position"), field("Impact Position V"));
        ((ip.matches(a) + ipv.matches(b)) - (ipv.matches(a) + ip.matches(b))).abs()
    }

    /// [`TIE_MARGIN`]'s measurements, re-measured: the margins the M34 plan's finding 3 quotes for
    /// the `Impact Position` pair, the real tile first and the V tile second.
    #[test]
    fn the_margin_sits_between_label_damage_and_evidence() {
        for (real, v_tile, margin) in [
            ("Impact Position", "Impact Position", 0.0),
            ("Impact Position", "ImpactPosition", 0.0002),
            ("Impact Position", "lmpact Position", 0.004),
            ("Impact Position", "Impact Position Y", 0.066),
            ("Impact Position", "Impact Position V", 0.125),
        ] {
            let measured = crosswise_margin(real, v_tile);
            assert!(
                (measured - margin).abs() < 0.0005,
                "{v_tile:?}: {measured} is not {margin}"
            );
            assert_eq!(measured <= TIE_MARGIN, margin < 0.01, "{v_tile:?}");
        }
    }

    /// The pairs the tie rule resolves, and the label it then gives each box.
    #[test]
    fn evidence_resolves_the_pair() {
        for v_tile in ["Impact Position V", "Impact Position Y"] {
            let (tiles, withheld) = locate(hd_golf(), &in_a_row(&["Impact Position", v_tile]));
            assert_eq!(
                tiles,
                [
                    ("Impact Position".into(), Some("Impact Position".into())),
                    (v_tile.into(), Some("Impact Position V".into())),
                ],
                "{v_tile:?}"
            );
            assert!(withheld.is_empty(), "{v_tile:?}: {withheld:?}");
        }
    }

    /// The pairs it withholds: both labels, both boxes still labels. The mirror of `2026-08-23-1`
    /// (the V tile read exactly, the real tile damaged) is the case strict equality gets wrong: its
    /// best assignment by 0.0002 is the crosswise one, which would store the real tile's value as
    /// `impact_position_v`.
    #[test]
    fn label_damage_withholds_the_pair() {
        for pair in [
            ["Impact Position", "Impact Position"],
            ["Impact Position", "ImpactPosition"],
            ["ImpactPosition", "Impact Position"],
            ["Impact Position", "lmpact Position"],
        ] {
            let (tiles, withheld) = locate(hd_golf(), &in_a_row(&pair));
            assert_eq!(
                tiles,
                [(pair[0].into(), None), (pair[1].into(), None)],
                "{pair:?}"
            );
            assert_eq!(
                withheld,
                ["Impact Position", "Impact Position V"],
                "{pair:?}"
            );
        }
    }

    /// One `Impact Position` alone is the plain tile (1.0 against 0.9375), and the V label is not
    /// located at all: the reference layout, where it is `no tile found`.
    #[test]
    fn one_box_takes_its_best_label_and_the_other_is_not_located() {
        let (tiles, withheld) = locate(hd_golf(), &in_a_row(&["Impact Position"]));
        assert_eq!(
            tiles,
            [("Impact Position".into(), Some("Impact Position".into()))]
        );
        assert!(withheld.is_empty());
    }

    /// Decision 6: two boxes that fit one label equally are the same ambiguity, and need no rule of
    /// their own. Within the margin too: `Carry` and `Carny` (0.8) are not a tie, and the exact box
    /// keeps the label while the other is left as text.
    #[test]
    fn two_boxes_for_one_label_withhold_it() {
        let (tiles, withheld) = locate(hd_golf(), &in_a_row(&["Carry", "Spin", "Carry"]));
        assert_eq!(
            tiles,
            [
                ("Carry".into(), None),
                ("Spin".into(), Some("Spin".into())),
                ("Carry".into(), None),
            ]
        );
        assert_eq!(withheld, ["Carry"]);

        let (tiles, withheld) = locate(hd_golf(), &in_a_row(&["Carny", "Carry"]));
        assert_eq!(tiles, [("Carry".into(), Some("Carry".into()))]);
        assert!(withheld.is_empty());
    }

    /// The warning names the boxes a withheld label could have been, in box order.
    #[test]
    fn a_withheld_label_says_which_boxes_tied() {
        let labels = assign_labels(&in_a_row(&["Impact Position", "ImpactPosition"]), hd_golf());
        let warnings: Vec<&str> = labels.withheld.iter().map(|(_, w)| w.as_str()).collect();
        assert_eq!(
            warnings,
            [
                "Impact Position: label tie between 'Impact Position' and 'ImpactPosition' - not read",
                "Impact Position V: label tie between 'Impact Position' and 'ImpactPosition' - not read",
            ]
        );
    }

    /// A profile of `n` labels that each score 8/9 against every other, so `n` exact boxes make one
    /// tie of every box against every label.
    fn crowded(n: usize) -> DeviceProfile {
        let fields: Vec<String> = (1..=n)
            .map(|i| format!(r#"{{"label": "ABCDEFGH{i}", "target": "t{i}"}}"#))
            .collect();
        profile(&format!("[{}]", fields.join(", ")))
    }

    /// [`TIE_CAP`]'s counts: a complete tie of `n` boxes against `n` labels has
    /// `Σₖ C(n,k)²·k!` one-to-one assignments, empty and partial ones included.
    #[test]
    fn the_assignments_of_a_complete_tie_are_counted_as_the_cap_says() {
        for (n, want) in [(2, 7), (TIE_CAP, 1_441_729)] {
            let local: Vec<Vec<(usize, f64)>> =
                (0..n).map(|_| (0..n).map(|f| (f, 1.0)).collect()).collect();
            let mut count = 0_u64;
            assignments(&local, &[1; TIE_CAP], &mut |_, _| count += 1);
            assert_eq!(count, want, "{n}×{n}");
        }
    }

    /// [`TIE_CAP`] is enumerated at its limit: eight exact boxes against eight labels that all fit
    /// each other is one tie of 1,441,729 assignments, and every label resolves to its own box,
    /// because any swap costs `2 × (1 − 8/9)`. A ninth box makes the tie too big, and it is
    /// withheld whole, every box still a label.
    #[test]
    fn a_tie_at_the_cap_is_weighed_and_one_above_it_is_withheld() {
        let profile = crowded(TIE_CAP);
        let texts: Vec<String> = (1..=TIE_CAP).map(|i| format!("ABCDEFGH{i}")).collect();
        let mut boxes = in_a_row(&texts.iter().map(String::as_str).collect::<Vec<_>>());
        let (tiles, withheld) = locate(&profile, &boxes);
        assert!(withheld.is_empty(), "{withheld:?}");
        assert!(
            tiles
                .iter()
                .all(|(text, label)| label.as_deref() == Some(text)),
            "{tiles:?}"
        );

        boxes.push(text_box("ABCDEFGH1", 0.0, 100.0, 120.0, 20.0, 1.0));
        let labels = assign_labels(&boxes, &profile);
        assert_eq!(labels.tiles.len(), TIE_CAP + 1);
        assert!(labels.tiles.iter().all(|&(field, _)| field.is_none()));
        assert_eq!(labels.withheld.len(), TIE_CAP);
        assert_eq!(
            labels.withheld[0].1,
            "ABCDEFGH1: label tie among 9 boxes and 8 labels, more than the tie rule weighs - not read"
        );
    }

    // --- merged labels [M42 P4] ------------------------------------------------------------------

    /// The two merges the M42 plan's finding 3 measured, as unit-bearing labels, the way Full
    /// Swing prints them.
    const MERGED_FIELDS: &str = r#"[
        {"label": "Club Speed Mph", "target": "club_head_speed"},
        {"label": "Ball Speed Mph", "target": "ball_speed"},
        {"label": "Ball Direction Deg", "target": "launch_direction"},
        {"label": "Launch Angle", "target": "launch_angle"}]"#;

    fn splitting(fields: &str) -> DeviceProfile {
        profile_with(r#""split_merged_labels": true, "#, fields)
    }

    fn texts(boxes: &[TextBox]) -> Vec<&str> {
        boxes
            .iter()
            .map(|text_box| text_box.text.as_str())
            .collect()
    }

    /// Each merge is cut where its two labels meet, and the halves share the box's width by their
    /// letters: 12 and 12 of 24, then 16 and 11 of 27. Everything else is the box's own, and a value
    /// box beside it is left alone.
    #[test]
    fn a_merged_pair_is_split_where_its_labels_meet() {
        let profile = splitting(MERGED_FIELDS);
        for (merged, width, halves, left_width) in [
            (
                "CLUBSPEEDMPHBALLSPEEDMPH",
                240.0,
                ["CLUBSPEEDMPH", "BALLSPEEDMPH"],
                120.0,
            ),
            (
                "BALLDIRECTIONDEGLAUNCHANGLE",
                270.0,
                ["BALLDIRECTIONDEG", "LAUNCHANGLE"],
                160.0,
            ),
        ] {
            let whole = TextBox {
                rgb: Some([238.0, 236.0, 229.0]),
                ..text_box(merged, 100.0, 40.0, width, 20.0, 0.93)
            };
            let value = text_box("95", 100.0, 70.0, 30.0, 20.0, 0.99);
            let split = split_merged(&[whole.clone(), value.clone()], &profile);
            assert_eq!(
                split,
                [
                    TextBox {
                        text: halves[0].to_string(),
                        width: left_width,
                        ..whole.clone()
                    },
                    TextBox {
                        text: halves[1].to_string(),
                        x: 100.0 + left_width,
                        width: width - left_width,
                        ..whole.clone()
                    },
                    value,
                ],
                "{merged}"
            );
        }
    }

    /// The split list is what every later step sees: both halves are located labels, and each reads
    /// the value under its own half. Without the flag the same boxes locate nothing, as before M42.
    #[test]
    fn a_split_pair_is_located_and_read_and_only_where_the_profile_opts_in() {
        let fields = r#"[{"label": "Club Speed Mph", "target": "club_head_speed"},
                         {"label": "Ball Speed Mph", "target": "ball_speed"}]"#;
        let boxes = [
            text_box("CLUBSPEEDMPHBALLSPEEDMPH", 0.0, 0.0, 240.0, 20.0, 1.0),
            text_box("95", 45.0, 30.0, 30.0, 20.0, 1.0),
            text_box("102.5", 155.0, 30.0, 50.0, 20.0, 1.0),
        ];

        let parsed = parse_screen(&boxes, &splitting(fields));
        assert_eq!(
            read(&parsed),
            [("Club Speed Mph", "95"), ("Ball Speed Mph", "102.5")]
        );
        assert_eq!(
            parsed.values.get("club_head_speed"),
            Some(&FieldValue::Number(95.0))
        );
        assert_eq!(
            parsed.values.get("ball_speed"),
            Some(&FieldValue::Number(102.5))
        );
        assert_eq!(parsed.fields_present, ["club_head_speed", "ball_speed"]);
        assert!(parsed.warnings.is_empty(), "{:?}", parsed.warnings);

        let unsplit = parse_screen(&boxes, &profile(fields));
        assert!(unsplit.is_empty() && unsplit.fields_present.is_empty());
        assert_eq!(
            unsplit.warnings,
            ["no t tile labels recognized - wrong screen, or the crop failed"]
        );
        // The cut is there to be found without the flag: it is `parse_screen` that does not look.
        assert_eq!(split_merged(&boxes, &profile(fields)).len(), 4);
    }

    /// `Ball Speed` is a label of its own, so a box reading it is never cut, though `BALL` and
    /// `SPEED` would each match a label exactly.
    #[test]
    fn a_box_that_already_matches_a_label_is_never_split() {
        let profile = splitting(
            r#"[{"label": "Ball", "target": "a"}, {"label": "Speed", "target": "b"},
                {"label": "Ball Speed", "target": "ball_speed"}]"#,
        );
        assert_eq!(profile.fields[0].matches("BALL"), 1.0);
        assert_eq!(profile.fields[1].matches("SPEED"), 1.0);
        for text in ["Ball Speed", "BALLSPEED"] {
            let boxes = [text_box(text, 0.0, 0.0, 90.0, 20.0, 1.0)];
            assert_eq!(split_merged(&boxes, &profile), boxes, "{text:?}");
        }
    }

    /// `CARRYCARRY` cut in the middle is `Carry` twice at 1.0, and that is one tile read twice, not
    /// two tiles: no cut qualifies and the box is left whole.
    #[test]
    fn a_cut_whose_halves_name_one_label_does_not_qualify() {
        let profile = splitting(
            r#"[{"label": "Carry", "target": "carry_distance"},
                {"label": "Total", "target": "total_distance"}]"#,
        );
        assert_eq!(
            profile.field_for("CARRY").map(|field| field.label.as_str()),
            Some("Carry")
        );
        let boxes = [text_box("CARRYCARRY", 0.0, 0.0, 100.0, 20.0, 1.0)];
        assert_eq!(split_merged(&boxes, &profile), boxes);
    }

    /// `CARRY | XTOTAL` and `CARRYX | TOTAL` each have a weaker half of 10/11, so the earlier cut
    /// wins. Five of eleven letters is five elevenths of the width.
    #[test]
    fn a_tie_on_the_weaker_half_goes_to_the_earlier_cut() {
        let profile = splitting(
            r#"[{"label": "Carry", "target": "carry_distance"},
                {"label": "Total", "target": "total_distance"}]"#,
        );
        let (carry, total) = (&profile.fields[0], &profile.fields[1]);
        let early = carry.matches("CARRY").min(total.matches("XTOTAL"));
        let late = carry.matches("CARRYX").min(total.matches("TOTAL"));
        assert_eq!((early, late), (10.0 / 11.0, 10.0 / 11.0));

        let split = split_merged(
            &[text_box("CARRYXTOTAL", 0.0, 0.0, 110.0, 20.0, 1.0)],
            &profile,
        );
        assert_eq!(texts(&split), ["CARRY", "XTOTAL"]);
        assert_eq!(
            split.iter().map(|b| (b.x, b.width)).collect::<Vec<_>>(),
            [(0.0, 50.0), (50.0, 60.0)]
        );
    }

    // --- a tile printed twice [M42 P5] ------------------------------------------------------------

    /// `Total Yd` printed twice, as Full Swing prints it, beside a `Carry Yd` that is printed once.
    const TOTAL_TWICE: &str = r#"[{"label": "Carry Yd", "target": "carry_distance"},
        {"label": "Total Yd", "target": "total_distance", "printed_twice": true}]"#;

    /// The same tiles with no rule: what `hd_golf` and every profile before M42 is.
    const TOTAL_ONCE: &str = r#"[{"label": "Carry Yd", "target": "carry_distance"},
        {"label": "Total Yd", "target": "total_distance"}]"#;

    /// Full Swing's arrangement, cut down: `Total Yd` alone in a header row over `top`, then a data
    /// row of `Carry Yd` over `131` and `Total Yd` again over `bottom`. `None` leaves that cell with
    /// no box under its label.
    fn total_twice(top: Option<&str>, bottom: Option<&str>) -> Vec<TextBox> {
        let mut boxes = vec![text_box("Total Yd", 0.0, 0.0, 100.0, 20.0, 1.0)];
        boxes.extend(top.map(|text| text_box(text, 20.0, 30.0, 40.0, 20.0, 1.0)));
        boxes.push(text_box("Carry Yd", 0.0, 100.0, 100.0, 20.0, 1.0));
        boxes.push(text_box("131", 20.0, 130.0, 40.0, 20.0, 1.0));
        boxes.push(text_box("Total Yd", 200.0, 100.0, 100.0, 20.0, 1.0));
        boxes.extend(bottom.map(|text| text_box(text, 220.0, 130.0, 40.0, 20.0, 1.0)));
        boxes
    }

    fn total(parsed: &ParsedShot) -> Option<&FieldValue> {
        parsed.values.get("total_distance")
    }

    /// Two cells that agree store the value once, where the top one stands, and `raw_fields` keeps
    /// both texts top first. `134.0` and `134` are one number. Both cells hold text and count as
    /// one value read: with `Carry Yd` empty the value coverage is a half, as it would be with the
    /// total printed once, where two would have counted it whole.
    #[test]
    fn two_cells_that_agree_store_the_value() {
        let profile = profile(TOTAL_TWICE);
        for (top, bottom) in [("134", "134"), ("134", "134.0")] {
            let parsed = parse_screen(&total_twice(Some(top), Some(bottom)), &profile);
            let joined = format!("{top} | {bottom}");
            assert_eq!(
                read(&parsed),
                [("Total Yd", joined.as_str()), ("Carry Yd", "131")]
            );
            assert_eq!(total(&parsed), Some(&FieldValue::Number(134.0)));
            assert_eq!(parsed.fields_present, ["carry_distance", "total_distance"]);
            assert!(parsed.warnings.is_empty(), "{:?}", parsed.warnings);
            assert_eq!(parsed.confidence, 1.0);
        }

        let mut boxes = total_twice(Some("134"), Some("134"));
        boxes.retain(|text_box| text_box.text != "131");
        let parsed = parse_screen(&boxes, &profile);
        assert_eq!(total(&parsed), Some(&FieldValue::Number(134.0)));
        assert_eq!(parsed.confidence, 0.825);
    }

    /// Two cells that disagree store nothing, and say what each held, top first. The tile was
    /// located and read, so it is in `fields_present` and `raw_fields` both.
    #[test]
    fn two_cells_that_disagree_store_nothing() {
        let parsed = parse_screen(
            &total_twice(Some("134"), Some("137")),
            &profile(TOTAL_TWICE),
        );
        assert_eq!(total(&parsed), None);
        assert_eq!(
            parsed.warnings,
            ["Total Yd: printed twice as '134' and '137' - not stored"]
        );
        assert_eq!(read(&parsed)[0], ("Total Yd", "134 | 137"));
        assert_eq!(parsed.fields_present, ["carry_distance", "total_distance"]);
        assert_eq!(
            parsed.values.get("carry_distance"),
            Some(&FieldValue::Number(131.0))
        );
    }

    /// One copy giving a number is enough, whichever it is and whatever the other held, and the
    /// other's own warning is not raised over a tile that holds a value. One box located is read as
    /// any tile is. Neither giving one raises each cell's warning, a repeated line once.
    #[test]
    fn one_cell_with_a_number_stores_it() {
        let profile = profile(TOTAL_TWICE);
        for (top, bottom, raw) in [
            (Some("134"), None, "134 | "),
            (None, Some("134"), " | 134"),
            (Some("---"), Some("134"), "--- | 134"),
            (Some("134"), Some("yds"), "134 | yds"),
        ] {
            let parsed = parse_screen(&total_twice(top, bottom), &profile);
            assert_eq!(total(&parsed), Some(&FieldValue::Number(134.0)), "{raw:?}");
            assert_eq!(read(&parsed)[0], ("Total Yd", raw));
            assert!(parsed.warnings.is_empty(), "{raw:?}: {:?}", parsed.warnings);
        }

        let mut boxes = total_twice(None, Some("134"));
        boxes.remove(0);
        let parsed = parse_screen(&boxes, &profile);
        assert_eq!(total(&parsed), Some(&FieldValue::Number(134.0)));
        assert_eq!(read(&parsed), [("Carry Yd", "131"), ("Total Yd", "134")]);

        for (top, bottom, warnings) in [
            (None, None, &["Total Yd: no value text under the label"][..]),
            (
                Some("yds"),
                None,
                &[
                    "Total Yd: could not read a number from 'yds'",
                    "Total Yd: no value text under the label",
                ][..],
            ),
            (Some("---"), Some("---"), &[][..]),
        ] {
            let parsed = parse_screen(&total_twice(top, bottom), &profile);
            assert_eq!(total(&parsed), None, "{top:?} {bottom:?}");
            assert_eq!(parsed.warnings, warnings, "{top:?} {bottom:?}");
        }
    }

    /// The field's two boxes are one assignment giving it both, not two rivals: both are resolved
    /// to it and nothing is withheld, a damaged copy included. Capacity two adds exactly the
    /// assignment giving the field both boxes to the three one-to-one ones.
    #[test]
    fn the_fields_two_boxes_never_tie_with_each_other() {
        let profile = profile(TOTAL_TWICE);
        for second in ["Total Yd", "TOTALYD"] {
            let boxes = in_a_row(&["Total Yd", second]);
            let (tiles, withheld) = locate(&profile, &boxes);
            assert_eq!(
                tiles,
                [
                    ("Total Yd".into(), Some("Total Yd".into())),
                    (second.into(), Some("Total Yd".into())),
                ]
            );
            assert!(withheld.is_empty(), "{second:?}: {withheld:?}");
        }

        let local = vec![vec![(0, 1.0)], vec![(0, 1.0)]];
        for (capacity, want) in [(1, 3), (2, 4)] {
            let mut count = 0;
            assignments(&local, &[capacity], &mut |_, _| count += 1);
            assert_eq!(count, want, "capacity {capacity}");
        }
    }

    /// A third box that fits the label is a real tie: three pairs total the same, and the field is
    /// withheld with all three named and all three still labels.
    #[test]
    fn a_third_box_for_the_label_withholds_it() {
        let profile = profile(TOTAL_TWICE);
        let boxes = in_a_row(&["Total Yd", "Total Yd", "Total Yd"]);
        let (tiles, withheld) = locate(&profile, &boxes);
        assert_eq!(tiles, vec![("Total Yd".to_string(), None::<String>); 3]);
        assert_eq!(withheld, ["Total Yd"]);
        let labels = assign_labels(&boxes, &profile);
        assert_eq!(
            labels.withheld[0].1,
            "Total Yd: label tie between 'Total Yd', 'Total Yd' and 'Total Yd' - not read"
        );
    }

    /// Without the rule, the same photo is the committed `carry-twice` vector's case: two boxes fit
    /// one label equally, and it is located, withheld, and never read.
    #[test]
    fn without_the_rule_two_boxes_withhold_the_label_as_before() {
        let parsed = parse_screen(&total_twice(Some("134"), Some("134")), &profile(TOTAL_ONCE));
        assert_eq!(total(&parsed), None);
        assert_eq!(
            parsed.warnings,
            ["Total Yd: label tie between 'Total Yd' and 'Total Yd' - not read"]
        );
        assert_eq!(read(&parsed), [("Carry Yd", "131")]);
        assert_eq!(parsed.fields_present, ["carry_distance", "total_distance"]);
    }

    // --- the sign from a lit letter (M42 P6) --------------------------------------------------

    const BALL_DIRECTION: &str = r#"[{"label": "Ball Direction Deg", "target": "launch_direction",
        "lit_sign": [{"token": "L", "sign": -1}, {"token": "R", "sign": 1}]}]"#;

    /// `IMG_2889`'s lit `R` and unlit `F`, as the M42 plan's finding 4 measured them.
    const LIT: Option<[f64; 3]> = Some([239.0, 222.0, 90.0]);
    const UNLIT: Option<[f64; 3]> = Some([187.0, 183.0, 176.0]);

    /// Ball direction as `IMG_2889` lays it out, cut down: the label, then each of `letters` with
    /// its colour on a line of their own, left to right, then `value` under them, all in the
    /// label's cell. So the cell's text is the letters, then the value.
    fn direction(value: &str, letters: &[(&str, Option<[f64; 3]>)]) -> Vec<TextBox> {
        let mut boxes = vec![text_box("Ball Direction Deg", 0.0, 0.0, 200.0, 20.0, 1.0)];
        for (at, &(text, rgb)) in letters.iter().enumerate() {
            let x = 20.0 + 140.0 * at as f64;
            boxes.push(TextBox {
                rgb,
                ..text_box(text, x, 25.0, 20.0, 20.0, 1.0)
            });
        }
        boxes.push(text_box(value, 60.0, 50.0, 80.0, 40.0, 1.0));
        boxes
    }

    fn launch_direction(parsed: &ParsedShot) -> Option<&FieldValue> {
        parsed.values.get("launch_direction")
    }

    const UNDECIDED: &str =
        "Ball Direction Deg: neither L nor R could be read as lit - sign unknown, not stored";

    /// Every row of the M42 plan's finding 4 is lit exactly where it was lit by eye, and so are
    /// M42 P2's three letters through the shipping reader; white digits and labels from the same
    /// photos are not. Then each edge on its own: the brightness floor is inclusive and decides a
    /// dim yellow, and the ratio decides a bright one.
    #[test]
    fn the_measured_letters_are_lit_where_they_were_lit_by_eye() {
        for (rgb, lit) in [
            ([239.0, 222.0, 90.0], true),
            ([227.0, 207.0, 72.0], true),
            ([187.0, 183.0, 176.0], false),
            ([200.0, 206.0, 197.0], false),
            ([215.0, 214.0, 206.0], false),
            ([243.0, 225.0, 73.0], true),
            ([241.0, 223.0, 91.0], true),
            ([249.0, 220.0, 96.0], true),
            ([254.0, 238.0, 113.0], true),
            ([238.93, 222.16, 90.45], true),
            ([226.9, 207.04, 71.97], true),
            ([186.55, 183.21, 175.62], false),
            ([233.0, 235.0, 233.0], false),
            ([246.0, 245.0, 246.0], false),
            ([230.0, 233.0, 228.0], false),
        ] {
            assert_eq!(is_lit(rgb), lit, "{rgb:?}");
        }
        for (rgb, lit) in [
            ([120.0, 130.0, 10.0], true),
            ([130.0, 120.0, 10.0], true),
            ([119.0, 130.0, 10.0], false),
            ([130.0, 119.0, 10.0], false),
            ([200.0, 220.0, 139.0], true),
            ([200.0, 220.0, 141.0], false),
            ([220.0, 200.0, 141.0], false),
        ] {
            assert_eq!(is_lit(rgb), lit, "{rgb:?}");
        }
    }

    /// Each way call 5 says the pair can fall, read through `parse_screen`. One letter is enough
    /// either way, lit or unlit; two decide when exactly one is lit; anything else, or a letter
    /// with no colour, withholds the value with call 5's warning, the tile still located and its
    /// text still recorded.
    #[test]
    fn each_way_the_letters_can_fall_decides_or_withholds_the_sign() {
        let profile = profile(BALL_DIRECTION);
        for (letters, want) in [
            (&[("R", LIT)][..], Some(10.1)),
            (&[("L", LIT)][..], Some(-10.1)),
            (&[("R", UNLIT)][..], Some(-10.1)),
            (&[("L", UNLIT)][..], Some(10.1)),
            (&[("L", UNLIT), ("R", LIT)][..], Some(10.1)),
            (&[("L", LIT), ("R", UNLIT)][..], Some(-10.1)),
            (&[("R", LIT), ("L", UNLIT)][..], Some(10.1)),
            (&[("L", LIT), ("R", LIT)][..], None),
            (&[("L", UNLIT), ("R", UNLIT)][..], None),
            (&[][..], None),
            (&[("L", None), ("R", LIT)][..], None),
            (&[("L", UNLIT), ("R", None)][..], None),
            (&[("R", None)][..], None),
        ] {
            let parsed = parse_screen(&direction("10.1", letters), &profile);
            assert_eq!(
                launch_direction(&parsed),
                want.map(FieldValue::Number).as_ref(),
                "{letters:?}"
            );
            let warnings: &[&str] = if want.is_some() { &[] } else { &[UNDECIDED] };
            assert_eq!(parsed.warnings, warnings, "{letters:?}");
            assert_eq!(parsed.fields_present, ["launch_direction"]);
            let texts: Vec<&str> = letters.iter().map(|&(text, _)| text).collect();
            let raw = [texts.as_slice(), &["10.1"]].concat().join(" ");
            assert_eq!(read(&parsed), [("Ball Direction Deg", raw.as_str())]);
        }
    }

    /// The digits give the magnitude and never the sign, so a stray `-` or `+` is dropped and the
    /// letters decide. A zero needs no sign, so it is stored with no letter read at all, as `0.0`
    /// whichever sign was printed. A cell with no number says so, as any tile's does.
    #[test]
    fn the_digits_give_the_magnitude_and_a_zero_needs_no_sign() {
        let profile = profile(BALL_DIRECTION);
        for (value, letters, want) in [
            ("-10.1", &[("R", LIT)][..], 10.1),
            ("+10.1", &[("L", LIT)][..], -10.1),
            ("10.1\u{b0}", &[("L", UNLIT)][..], 10.1),
            ("0.0", &[][..], 0.0),
            ("-0.0", &[][..], 0.0),
            ("0", &[("L", LIT), ("R", LIT)][..], 0.0),
        ] {
            let parsed = parse_screen(&direction(value, letters), &profile);
            assert!(
                parsed.warnings.is_empty(),
                "{value:?}: {:?}",
                parsed.warnings
            );
            let Some(&FieldValue::Number(number)) = launch_direction(&parsed) else {
                panic!("{value:?} stored no number");
            };
            assert_eq!(number, want, "{value:?}");
            assert!(number.is_sign_positive() || want < 0.0, "{value:?}");
        }

        let parsed = parse_screen(&direction("--", &[("R", LIT)]), &profile);
        assert_eq!(launch_direction(&parsed), None);
        assert_eq!(
            parsed.warnings,
            ["Ball Direction Deg: could not read a number from 'R --'"]
        );
    }

    /// A letter box is a box holding one token and nothing else, once stripped and upper-cased:
    /// ` r ` is `R`, and `RR` (`IMG_2893` read `DD`, M42 P3's finding) or `R.` is no letter. Two
    /// boxes of one letter count once when they agree and decide nothing when they do not.
    #[test]
    fn a_letter_box_holds_one_token_and_nothing_else() {
        let profile = profile(BALL_DIRECTION);
        for (letters, want) in [
            (&[(" r ", LIT)][..], Some(10.1)),
            (&[("RR", LIT)][..], None),
            (&[("R.", LIT)][..], None),
            (&[("RR", LIT), ("L", UNLIT)][..], Some(10.1)),
            (&[("R", LIT), ("R", LIT)][..], Some(10.1)),
            (&[("R", UNLIT), ("R", UNLIT)][..], Some(-10.1)),
            (&[("R", LIT), ("R", UNLIT)][..], None),
            (&[("R", LIT), ("R", None)][..], None),
        ] {
            let parsed = parse_screen(&direction("10.1", letters), &profile);
            assert_eq!(
                launch_direction(&parsed),
                want.map(FieldValue::Number).as_ref(),
                "{letters:?}"
            );
        }
    }

    /// A tile without `lit_sign` reads its letters as text, exactly as before: with no sign rule
    /// the number is stored as printed and the colours are never looked at, and with HD Golf's word
    /// rule the residual `L R` is no direction word, so the magnitude is stored with the word
    /// rule's own warning. That second case is why the word rule cannot read this screen.
    #[test]
    fn a_tile_without_the_rule_reads_its_letters_as_text() {
        let boxes = direction("-10.1", &[("L", UNLIT), ("R", LIT)]);
        let plain = parse_screen(
            &boxes,
            &profile(r#"[{"label": "Ball Direction Deg", "target": "launch_direction"}]"#),
        );
        assert_eq!(launch_direction(&plain), Some(&FieldValue::Number(-10.1)));
        assert!(plain.warnings.is_empty(), "{:?}", plain.warnings);
        assert_eq!(read(&plain), [("Ball Direction Deg", "L R -10.1")]);

        let boxes = direction("10.1", &[("L", UNLIT), ("R", LIT)]);
        let worded = parse_screen(
            &boxes,
            &profile(
                r#"[{"label": "Ball Direction Deg", "target": "launch_direction",
                     "sign_tokens": [{"tokens": ["R"], "sign": 1},
                                     {"tokens": ["L"], "sign": -1}]}]"#,
            ),
        );
        assert_eq!(launch_direction(&worded), Some(&FieldValue::Number(10.1)));
        assert_eq!(
            worded.warnings,
            [
                "Ball Direction Deg: no direction word in 'L R 10.1' - sign unknown, storing the \
                 magnitude as printed"
            ]
        );
    }

    // --- a number printed in feet, stored in yards (M42 P7) --------------------------------------

    /// One tile: `label` over `value`, each in a box of its own.
    fn tile(label: &str, value: &str) -> [TextBox; 2] {
        [
            text_box(label, 0.0, 0.0, 100.0, 20.0, 1.0),
            text_box(value, 20.0, 30.0, 40.0, 20.0, 1.0),
        ]
    }

    fn number(parsed: &ParsedShot, target: &str) -> Option<f64> {
        match parsed.values.get(target) {
            Some(&FieldValue::Number(number)) => Some(number),
            _ => None,
        }
    }

    /// Every apex the M42 plan's truth table holds, in feet, is stored as the printed number divided
    /// by 3.0, to the bit, with `raw_fields` keeping the text as printed and nothing warned. With no
    /// `printed_unit` the same tile stores the number as printed, as every profile before M42 does.
    #[test]
    fn a_number_printed_in_feet_is_stored_in_yards() {
        let feet =
            profile(r#"[{"label": "Apex Ft", "target": "apex_height", "printed_unit": "feet"}]"#);
        let plain = profile(r#"[{"label": "Apex Ft", "target": "apex_height"}]"#);
        for (printed, printed_feet) in [
            ("85.2", 85.2_f64),
            ("39.9", 39.9),
            ("71.4", 71.4),
            ("15.5", 15.5),
            ("80.6", 80.6),
            ("142.2", 142.2),
        ] {
            let parsed = parse_screen(&tile("Apex Ft", printed), &feet);
            assert_eq!(
                number(&parsed, "apex_height").map(f64::to_bits),
                Some((printed_feet / 3.0).to_bits()),
                "{printed}"
            );
            assert_eq!(read(&parsed), [("Apex Ft", printed)]);
            assert!(
                parsed.warnings.is_empty(),
                "{printed}: {:?}",
                parsed.warnings
            );

            let parsed = parse_screen(&tile("Apex Ft", printed), &plain);
            assert_eq!(number(&parsed, "apex_height"), Some(printed_feet));
            assert_eq!(read(&parsed), [("Apex Ft", printed)]);
        }
    }

    /// The number is converted whichever rule gave its sign: a printed `-30` feet is `-10` yards, a
    /// lit `L` over `30` is `-10`, and a zero the letters never sign stays `+0.0`. (A sign is a
    /// multiplication by one, exact, so before or after the division gives the same float; the
    /// division comes last so it is written once.) A tile printed twice compares its copies in
    /// yards and still quotes them as printed.
    #[test]
    fn every_sign_rule_is_converted_and_the_copies_agree_in_yards() {
        let printed_sign = profile(
            r#"[{"label": "Offline Ft", "target": "carry_offline", "printed_sign": 1,
                 "printed_unit": "feet"}]"#,
        );
        let parsed = parse_screen(&tile("Offline Ft", "-30"), &printed_sign);
        assert_eq!(number(&parsed, "carry_offline"), Some(-10.0));

        let lit = profile(
            r#"[{"label": "Offline Ft", "target": "carry_offline", "printed_unit": "feet",
                 "lit_sign": [{"token": "L", "sign": -1}, {"token": "R", "sign": 1}]}]"#,
        );
        for (value, want) in [("30", -10.0), ("0", 0.0)] {
            let boxes = [
                text_box("Offline Ft", 0.0, 0.0, 200.0, 20.0, 1.0),
                TextBox {
                    rgb: LIT,
                    ..text_box("L", 20.0, 25.0, 20.0, 20.0, 1.0)
                },
                text_box(value, 60.0, 50.0, 80.0, 40.0, 1.0),
            ];
            let parsed = parse_screen(&boxes, &lit);
            let stored = number(&parsed, "carry_offline").expect("a number");
            assert_eq!(stored, want, "{value}");
            assert_eq!(stored.is_sign_negative(), want < 0.0, "{value}");
        }

        let twice = profile(
            r#"[{"label": "Carry Yd", "target": "carry_distance"},
                {"label": "Total Yd", "target": "total_distance", "printed_twice": true,
                 "printed_unit": "feet"}]"#,
        );
        let parsed = parse_screen(&total_twice(Some("402"), Some("402.0")), &twice);
        assert_eq!(total(&parsed), Some(&FieldValue::Number(134.0)));
        assert_eq!(number(&parsed, "carry_distance"), Some(131.0));
        let parsed = parse_screen(&total_twice(Some("402"), Some("405")), &twice);
        assert_eq!(total(&parsed), None);
        assert_eq!(
            parsed.warnings,
            ["Total Yd: printed twice as '402' and '405' - not stored"]
        );
    }

    /// What [`TIE_CAP`]'s argument and `raw_fields`' keying rest on, held of the shipped profiles:
    /// every label is distinct, and no label or alias reaches the match threshold against another
    /// field but within the `Impact Position` pair.
    #[test]
    fn only_the_impact_position_pair_overlaps() {
        for profile in crate::profile::profiles().values() {
            let labels: BTreeSet<&str> = profile.fields.iter().map(|f| f.label.as_str()).collect();
            assert_eq!(labels.len(), profile.fields.len(), "{}", profile.device);
            for field in &profile.fields {
                for other in &profile.fields {
                    if std::ptr::eq(field, other) {
                        continue;
                    }
                    let pair: BTreeSet<&str> = [field.label.as_str(), other.label.as_str()].into();
                    let expected = pair == BTreeSet::from(["Impact Position", "Impact Position V"]);
                    for name in std::iter::once(&other.label).chain(&other.aliases) {
                        let overlaps = field.matches(name) >= LABEL_MATCH_THRESHOLD;
                        assert_eq!(overlaps, expected, "{} against {name:?}", field.label);
                    }
                }
            }
        }
    }

    // --- the full_swing profile [M42 P8] ----------------------------------------------------------

    fn full_swing() -> &'static DeviceProfile {
        crate::profile::load_profile("full_swing").expect("the shipped profile")
    }

    /// Full Swing's labels and the unit-less aliases beside them stay apart: no name of one field
    /// reaches the threshold against another field. `only_the_impact_position_pair_overlaps` walks
    /// every profile and would catch it too; this one names the profile, so an alias P9's tuning
    /// adds fails here by name, and it records the margin.
    ///
    /// The closest pair is `Club Speed Mph` and `Ball Speed Mph`, 0.786 either way (CPython's
    /// `difflib`, measured at M42 P8), with `Club Speed` against `Ball Speed Mph` at 0.70 and `Total
    /// Yd` against `To Hole Yd`, and `Backspin Rpm` against `Sidespin Rpm`, at 0.667. So the speed
    /// pair sits 0.014 under the threshold, and an alias closer to the other speed than its own
    /// label is would make the two tie.
    #[test]
    fn full_swing_labels_do_not_overlap() {
        let profile = full_swing();
        let mut closest = (0.0, String::new());
        for field in &profile.fields {
            for other in &profile.fields {
                if std::ptr::eq(field, other) {
                    continue;
                }
                for name in std::iter::once(&other.label).chain(&other.aliases) {
                    let score = field.matches(name);
                    assert!(
                        score < LABEL_MATCH_THRESHOLD,
                        "{} scores {score} against {name:?}, another tile's name",
                        field.label
                    );
                    if score > closest.0 {
                        closest = (score, format!("{} against {name}", field.label));
                    }
                }
            }
        }
        assert_eq!(closest.1, "Club Speed Mph against Ball Speed Mph");
        assert_eq!(closest.0, 11.0 / 14.0);
    }

    /// The shipped `full_swing` profile turns on each of M42's four rules where the M42 plan says
    /// (call 9), and on no other tile: a key the load ignores in silence, or one on the wrong tile,
    /// would otherwise read this screen wrong with every test of the rule itself passing.
    #[test]
    fn full_swing_uses_every_rule() {
        let profile = full_swing();
        let on = |rule: fn(&ProfileField) -> bool| -> Vec<&str> {
            profile
                .fields
                .iter()
                .filter(|field| rule(field))
                .map(|field| field.label.as_str())
                .collect()
        };

        // Rule 1 (P4), the profile's own key: the OCR runs adjacent labels together on this screen.
        assert!(profile.split_merged_labels);
        // Rule 2 (P5): `TOTAL YD` stands in the header row and the data row.
        assert_eq!(on(|field| field.printed_twice), ["Total Yd"]);
        // Rule 3 (P6): ball direction lights `L` or `R`, sidespin `D` or `F`, + being right and
        // fade (call 2).
        assert_eq!(
            on(|field| !field.lit_sign.is_empty()),
            ["Ball Direction Deg", "Sidespin Rpm"]
        );
        let letters = |label: &str| -> Vec<(&str, i64)> {
            profile
                .fields
                .iter()
                .find(|field| field.label == label)
                .expect("the tile")
                .lit_sign
                .iter()
                .map(|lit| (lit.token.as_str(), lit.sign))
                .collect()
        };
        assert_eq!(letters("Ball Direction Deg"), [("L", -1), ("R", 1)]);
        assert_eq!(letters("Sidespin Rpm"), [("D", -1), ("F", 1)]);
        // Rule 4 (P7): the apex is printed in feet.
        assert_eq!(
            on(|field| field.printed_unit == Some(crate::profile::PrintedUnit::Feet)),
            ["Apex Ft"]
        );
        assert_eq!(on(|field| field.printed_unit.is_some()), ["Apex Ft"]);

        // No word or printed-sign rule anywhere: path and face print their own sign in ShotData's
        // convention (the plan's finding 8), and the lit tiles may not have one beside the letters.
        assert_eq!(
            on(|field| !field.sign_tokens.is_empty() || field.printed_sign.is_some()),
            Vec::<&str>::new()
        );
        // Three 17 px labels on `IMG_2889` have their value 3.62–3.85 label heights below them, past
        // the 3.5 default, which would lose all three (M42 P6's finding). P9 may tune it within
        // [4.0, 8.0].
        assert!((4.0..=8.0).contains(&profile.value_span_ratio));
    }
}
