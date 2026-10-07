//! Located OCR text in, a [`ParsedShot`] out: `parser.py::parse_screen`, ported faithfully and then
//! changed in one place, which box is which label. [M34 P5, P8]
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

use std::cmp::Ordering;
use std::collections::BTreeSet;

use pyfmt::OrderedMap;
use serde::Serialize;

use crate::profile::{
    normalize_label, DeviceProfile, FieldKind, ProfileField, LABEL_MATCH_THRESHOLD,
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
/// has, 1,441,729 at 8×8, and 17,572,114 at 9×9, walked twice. The 8×8 case takes about 0.1 s in
/// an unoptimised test build on the dev box, and each size past it multiplies that by ten or more,
/// on a phone. A screen
/// with more than eight boxes competing for one set of labels is not a screen this parser can read
/// anyway: no hd_golf label is near another but the `Impact Position` pair
/// (`only_the_impact_position_pair_overlaps`), so a tie that size is a photo of something else, and
/// withholding it costs nothing a golfer could have had.
pub const TIE_CAP: usize = 8;

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
pub fn parse_screen(boxes: &[TextBox], profile: &DeviceProfile) -> ParsedShot {
    read_labels(boxes, profile, assign_labels(boxes, profile))
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
    for cell in &cells {
        used.push(cell.label_box);
        used.extend(cell.value_boxes.iter().copied());
        read_cell(cell, profile, &mut parsed);
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

        let decided = decide(&tie, &candidates);
        let mut resolved: BTreeSet<usize> = BTreeSet::new();
        for (field, outcomes) in fields.iter().copied().zip(&decided.outcomes) {
            let mut each = outcomes.iter();
            match (each.next(), each.next()) {
                (Some(None), None) => {}
                (Some(&Some(index)), None) => {
                    labels.tiles.push((Some(field), index));
                    resolved.insert(index);
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
    /// Per field of the tie, in its order: every box some near-best assignment gives it, and `None`
    /// where one gives it none.
    outcomes: Vec<BTreeSet<Option<usize>>>,
    /// Every box some near-best assignment gives a field.
    label_boxes: BTreeSet<usize>,
}

/// Enumerate the tie's one-to-one assignments twice: once for the best total, once to collect what
/// every assignment within [`TIE_MARGIN`] of it gives each field. Two walks rather than one walk
/// that keeps every assignment, because the cap allows over a million of them.
fn decide(tie: &Tie, candidates: &[Vec<(usize, f64)>]) -> Decided {
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
    assignments(&local, &mut |_, total| {
        if total > best {
            best = total;
        }
    });

    let mut decided = Decided {
        outcomes: vec![BTreeSet::new(); tie.fields.len()],
        label_boxes: BTreeSet::new(),
    };
    assignments(&local, &mut |assignment, total| {
        if best - total > TIE_MARGIN {
            return;
        }
        let mut given = vec![None; tie.fields.len()];
        for (at, field) in assignment.iter().enumerate() {
            if let Some(field) = *field {
                given[field] = Some(tie.boxes[at]);
                decided.label_boxes.insert(tie.boxes[at]);
            }
        }
        for (outcomes, box_index) in decided.outcomes.iter_mut().zip(given) {
            outcomes.insert(box_index);
        }
    });
    decided
}

/// Visit every one-to-one assignment of boxes to fields, as each box's field (or `None`) and the
/// total of the scores taken, summed in box order. Fields are positions below [`TIE_CAP`], so a
/// bit mask holds which are taken.
fn assignments(local: &[Vec<(usize, f64)>], visit: &mut impl FnMut(&[Option<usize>], f64)) {
    fn walk(
        local: &[Vec<(usize, f64)>],
        at: usize,
        taken: u32,
        assignment: &mut Vec<Option<usize>>,
        total: f64,
        visit: &mut impl FnMut(&[Option<usize>], f64),
    ) {
        if at == local.len() {
            visit(assignment, total);
            return;
        }
        assignment.push(None);
        walk(local, at + 1, taken, assignment, total, visit);
        assignment.pop();
        for &(field, score) in &local[at] {
            if taken & (1 << field) == 0 {
                assignment.push(Some(field));
                walk(
                    local,
                    at + 1,
                    taken | (1 << field),
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
        0,
        0,
        &mut Vec::with_capacity(local.len()),
        0.0,
        visit,
    );
}

/// `'Impact Position' and 'ImpactPosition'`: the texts of the boxes a withheld field could have
/// been, in box order, and `no box` last where a near-best assignment gives it none.
fn tied_boxes(outcomes: &BTreeSet<Option<usize>>, boxes: &[TextBox]) -> String {
    let mut named: Vec<String> = outcomes
        .iter()
        .flatten()
        .map(|&index| pyfmt::str_repr(&boxes[index].text))
        .collect();
    if outcomes.contains(&None) {
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
    let target = field.target.as_ref().expect("a stored field has a target");

    let text = cell.text();
    parsed.raw_fields.insert(field.label.clone(), text.clone());

    if text.is_empty() {
        parsed
            .warnings
            .push(format!("{}: no value text under the label", field.label));
        return;
    }
    if is_blank(&text, &profile.blank_markers) {
        return; // the device measured nothing here: a correct read of an absent metric
    }

    if field.kind == FieldKind::Text {
        let upper = pyfmt::upper(&text);
        let words: Vec<&str> = pyfmt::split(&upper).collect();
        parsed
            .values
            .insert(target.clone(), FieldValue::Text(words.join(" ")));
        return;
    }

    let Some((mut magnitude, matched)) = first_number(&text) else {
        parsed.warnings.push(format!(
            "{}: could not read a number from {}",
            field.label,
            pyfmt::str_repr(&text)
        ));
        return;
    };

    if !field.sign_tokens.is_empty() || field.printed_sign.is_some() {
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
            (None, None) => parsed.warnings.push(format!(
                "{}: no direction word in {} - sign unknown, storing the magnitude as printed",
                field.label,
                pyfmt::str_repr(&text)
            )),
        }
    }
    parsed
        .values
        .insert(target.clone(), FieldValue::Number(magnitude));
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
    let read_values = cells
        .iter()
        .filter(|cell| cell.stored().is_some() && !cell.text().is_empty())
        .count();
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
        parse_profiles(&format!(
            r#"{{"profiles": [{{"device": "t", "title": "T", "fields": {fields}}}]}}"#
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
            assignments(&local, &mut |_, _| count += 1);
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
}
