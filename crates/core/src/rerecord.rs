//! The re-record: its rules as pure functions [M32 P4], and the run over `spec/` that applies them
//! [M32 P5].
//!
//! ADR-035 clause 3 moves the oracle to Rust, so from M32 a change to the engine's output is
//! recorded by `golf-core rerecord` rather than by Python's `regenerate`. A recorder that is also the
//! thing the vectors certify passes by construction — the objection `golf-core`'s module doc made
//! against a Rust `regenerate` until M32 — unless something stands between what it computes and
//! what it writes. This module is that something. Its first half is written over
//! [`serde_json::Value`] with no file in sight, so every rule is unit-tested before anything touches
//! `spec/`. Its second half, [`plan`] and [`Run::write`], is the walk over the files, and the
//! `golf-core rerecord` verb is argument parsing and printing around those two.
//!
//! # The steps, and which function is which
//!
//! §M32 "The verb" runs five steps per vector. The first (run Rust on the committed input) and the
//! file I/O belong to [`plan`] and [`Run::write`]. The rest are here:
//!
//! - **Compare, and refuse what is undeclared** (steps 2–3): [`gate`]. It runs [`compare`] between
//!   the committed document and `ours` — the committed document with Rust's answer substituted — and
//!   lets a difference through only at a path the [`Declaration`] names for that kind. Every
//!   difference it refuses comes back, never only the first, as an [`Undeclared`].
//! - **The committed document with only the declared paths replaced** (step 4): [`apply`].
//! - **Nothing written when nothing differs** (step 5): [`Applied::is_empty`], which the verb reads
//!   and [`ledger`] refuses to record.
//! - **The file says which of its values are Rust's**: [`ledger`] — `provenance.oracle`, and one
//!   `provenance.rerecords` entry naming what *this* document's re-record matched (call 8).
//!
//! # Why the base is the committed document, never Rust's
//!
//! Because the gate compares under §3's tolerance and the file must not. M32 P2 measured it: over the
//! 21 committed stage documents, 142 numbers Rust computes are inside [`RTOL`](crate::compare::RTOL)
//! of the recorded ones without being bit-identical — reassociation in the last bits, mostly of the
//! unrounded pose numbers. Writing Rust's own document would churn every one of them on a re-record
//! that declared none of them, and the file would stop being what ADR-035 clause 3 says it is: every
//! value Python recorded, surviving, except the ones a declaration names. So [`apply`] starts from
//! the committed `Value` and copies across only the declared paths, and an undeclared in-tolerance
//! float keeps Python's bits.
//!
//! # What a declaration can name, and what it cannot
//!
//! Exact paths, in [`compare`]'s spelling (M32's plan, call 3): document-rooted, no leading dot, a
//! list index as `[i]` — `expected.swing.shot.attack_angle`. A declared path matches a difference at
//! that path and at nothing beneath or above it. That is narrower than it could be, on purpose: a
//! `moved` path that covered its descendants would let `expected` declare the whole answer, and the
//! ledger would then claim as Rust's a value nobody looked at.
//!
//! - **No wildcard.** §M32 says "pattern", and nothing M32 declares needs one, so `[*]` is refused at
//!   parse time (call 4): a later declaration that writes one gets an error, not a silent non-match.
//! - **No removal.** §M32 names two kinds a declaration covers, an added key and a moved value. A
//!   [`DifferenceKind::RemovedKey`] is never declarable, so a port that drops a key cannot re-record
//!   its way past the drop.
//! - **Not the root, and not Python's spelling.** `""` would declare the whole document, and
//!   `.analysis_version` — `compare_results`' spelling — would match nothing here, declaring nothing
//!   while looking right. Both are refused, and so is any spelling [`compare`] would not produce
//!   (`[01]`, `[+1]`, `a..b`), which is checked by re-rendering the parsed path. Both languages
//!   *parse* a ledger path and walk the document with it; neither matches Python's spelling as a
//!   string.
//! - **A key holding `.`, `[` or `]` cannot be named.** No committed engine or stage document has one
//!   (scanned 2026-10-01); a document that grows one needs an escape, and this parser is where it
//!   goes.
//!
//! An entry that matches nothing *in one document* is not an error here: the synthetic vectors carry
//! no shot and match none of M32's added keys. Whether an entry matched *anywhere* is the run's
//! check (call 5), made across every document's [`Applied`].
//!
//! # The run
//!
//! [`plan`] walks the engine family (`vectors/synthetic/`, `vectors/corpus/`) and then the stage
//! family (`vectors/stages/{synthetic,corpus}/`), and does everything but write:
//!
//! - **The version guard first** (call 6): a declaration not at [`ANALYSIS_VERSION`] is refused before
//!   a vector is read, so a stale one cannot be re-run against a later engine.
//! - **Each engine vector** is run through [`crate::run`], gated, applied and ledgered. **Each stage
//!   vector** is joined to its engine vector by `provenance.derived_from`, run through
//!   [`run_stages`], gated, applied, ledgered — and then composed onto the engine document *as it
//!   will be written* (call 9), so the family is held to the bundle answer it is about to sit beside.
//! - **Every refusal across the run is collected**, and one anywhere means [`plan`] returns no
//!   [`Run`] at all — so nothing can be written anywhere (call 7). A reviewer fixes a declaration
//!   once, from one red report.
//! - **The typo guard** (call 5) runs last, when the gate has passed everywhere: every declared path
//!   must have matched some document — *or already sit in some document's ledger at this version*.
//!   That second clause is what makes the second run of a real declaration a no-op rather than a
//!   failure: by then nothing differs, so nothing matches, and every entry is a path an earlier run
//!   of the same declaration wrote down. A misspelled path is in no ledger, because a path only
//!   reaches one by matching. The ledger is matched on its version and its `by`, not on the
//!   declaration's file name, which depends on where the verb was run from.
//!
//! [`Run::write`] then writes the documents that changed, and only those (step 5).
//!
//! # How a file is written
//!
//! As `conformance.py::_write_json` wrote it, as far as `serde_json` can: keys sorted (the workspace
//! does not enable `preserve_order`), an indent of 2, a trailing newline on plain `.json` and none
//! inside a `.gz`, and gzip at level 9 with mtime 0 and the uncompressed name in the header, as
//! Python's `GzipFile` stamps it. Two things are matched to the *committed file* rather than to
//! `_write_json`'s source, because the source does not decide them:
//!
//! - **Line endings.** `_write_json`'s `write_text` translates `\n` to the platform's newline, and
//!   `spec/** -text` stores the bytes as they were written — so every plain `.json` vector in `spec/`
//!   is CRLF, because the repo is developed on Windows (`git ls-files --eol`, 2026-10-01: all 17
//!   plain files `i/crlf`), and every gzipped one is LF, because `GzipFile` is a binary stream. A
//!   writer that picked one would rewrite every line of the other kind on the first re-record; one
//!   that kept the file's own makes the same choice on every machine.
//! - **The deflate stream.** `flate2`'s backend is not zlib, so a gzipped vector is not
//!   byte-identical to Python's even when its text is. Nothing reads the compressed bytes; the gate
//!   reads values.
//!
//! The text still churns where `serde_json` and `json.dumps` spell a value differently (exponent
//! floats, `\u` escapes), which is why the report is the review and `git diff` never is.

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsStr;
use std::fmt::{self, Write as _};
use std::fs;
use std::io::{Read as _, Write as _};
use std::path::{Component, Path, PathBuf};

use contracts::swing::ANALYSIS_VERSION;
use flate2::read::GzDecoder;
use flate2::{Compression, GzBuilder};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::compare::{compare, Difference, DifferenceKind};
use crate::stages::{run_stages, verify_compose};
use crate::VectorInput;

/// Who wrote a `provenance.rerecords` entry — spelled as the command, so a reader of a vector knows
/// what to run.
pub const RERECORDED_BY: &str = "golf-core rerecord";

/// What `provenance.oracle` says on a vector that does not say yet.
///
/// ADR-032's thirteenth addendum left open what a Rust-re-recorded vector says about itself, and
/// §M32 answers it: `oracle` **stays** `"python"`, because every value a declaration does not name is
/// still Python's, and `rerecords` names the ones that are Rust's. No committed vector carried the
/// key before M32 (M32 P0), so the first re-record is what writes it — and never over a value
/// already there, since a later family may name another oracle on purpose.
pub const PYTHON_ORACLE: &str = "python";

/// One step of a [`LedgerPath`]: into an object by key, or into a list by index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Segment {
    Key(String),
    Index(usize),
}

/// A place in a vector document, in [`compare`]'s spelling, parsed — and so known to be spelled the
/// one way [`compare`] would spell it.
///
/// Deserializes from a string through [`LedgerPath::parse`], so a declaration file with a path this
/// refuses does not load at all.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(try_from = "String")]
pub struct LedgerPath {
    text: String,
    segments: Vec<Segment>,
}

impl LedgerPath {
    /// Parse `text`, refusing every spelling the module doc names.
    pub fn parse(text: &str) -> Result<Self, String> {
        let refused = |why: &str| format!("{text:?} is not a ledger path: {why}");
        if text.is_empty() {
            return Err(refused(
                "the empty path is the whole document, and declaring it would declare everything",
            ));
        }
        if text.starts_with('.') {
            return Err(refused(
                "a ledger path has no leading dot — that is `compare_results`' spelling, and here it \
                 would match nothing (M32's plan, call 3)",
            ));
        }
        if text.contains("[*]") {
            return Err(refused(
                "a wildcard is not built until a declaration needs one (M32's plan, call 4)",
            ));
        }

        let mut segments = Vec::new();
        for step in text.split('.') {
            let (key, mut rest) = step.split_at(step.find('[').unwrap_or(step.len()));
            if key.is_empty() || key.contains(']') {
                return Err(refused(
                    "every step starts with a key — a document is an object at its root, and a \
                     list index follows the key it indexes",
                ));
            }
            segments.push(Segment::Key(key.to_string()));
            while let Some(inner) = rest.strip_prefix('[') {
                let Some(close) = inner.find(']') else {
                    return Err(refused("a `[` with no `]`"));
                };
                let index = inner[..close]
                    .parse::<usize>()
                    .map_err(|_| refused(&format!("{:?} is not a list index", &inner[..close])))?;
                segments.push(Segment::Index(index));
                rest = &inner[close + 1..];
            }
            if !rest.is_empty() {
                return Err(refused(&format!(
                    "{rest:?} follows a list index, where only `.` or `[` may"
                )));
            }
        }

        // `usize`'s parser takes `+1` and `01`; the comparator writes neither, and a path it never
        // writes is one no difference will ever match. Re-rendering is the one check that covers
        // every such spelling, including ones nobody thought to list.
        let spelled = render(&segments);
        if spelled != text {
            return Err(refused(&format!(
                "the comparator spells this place {spelled:?}"
            )));
        }
        Ok(Self {
            text: text.to_string(),
            segments,
        })
    }

    /// The path as written, which is also how [`compare`] spells it.
    pub fn as_str(&self) -> &str {
        &self.text
    }

    pub fn segments(&self) -> &[Segment] {
        &self.segments
    }

    /// The value at this path in `document`, if there is one.
    pub fn get<'a>(&self, document: &'a Value) -> Option<&'a Value> {
        self.segments
            .iter()
            .try_fold(document, |value, segment| match segment {
                Segment::Key(key) => value.get(key.as_str()),
                Segment::Index(index) => value.get(*index),
            })
    }

    /// Put `value` at this path: insert or replace a key, or replace a list element. `None` if the
    /// parent is not there, or is not an object (for a key) or a list long enough (for an index).
    fn put(&self, document: &mut Value, value: Value) -> Option<()> {
        let (last, parent) = self.segments.split_last()?;
        let parent = parent
            .iter()
            .try_fold(document, |node, segment| match segment {
                Segment::Key(key) => node.get_mut(key.as_str()),
                Segment::Index(index) => node.get_mut(*index),
            })?;
        match last {
            Segment::Key(key) => {
                parent.as_object_mut()?.insert(key.clone(), value);
            }
            Segment::Index(index) => *parent.as_array_mut()?.get_mut(*index)? = value,
        }
        Some(())
    }
}

/// [`compare`]'s `child` and its `[{index}]`, applied to a parsed path.
fn render(segments: &[Segment]) -> String {
    let mut out = String::new();
    for segment in segments {
        match segment {
            Segment::Key(key) => {
                if !out.is_empty() {
                    out.push('.');
                }
                out.push_str(key);
            }
            Segment::Index(index) => {
                write!(out, "[{index}]").expect("writing to a String does not fail");
            }
        }
    }
    out
}

impl TryFrom<String> for LedgerPath {
    type Error = String;

    fn try_from(text: String) -> Result<Self, String> {
        Self::parse(&text)
    }
}

impl fmt::Display for LedgerPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.text)
    }
}

/// What a re-record may change, read from `spec/declarations/v{N}.json` (M32's decision 2).
///
/// Every key is required and no other is allowed. A misspelled `"moves"` would otherwise declare
/// nothing and still load, and a gate fed an empty declaration refuses everything — which fails
/// safe, but names the wrong mistake.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(try_from = "DeclarationFile")]
pub struct Declaration {
    /// The version the re-record moves to. The verb refuses a declaration that is not at
    /// `contracts::swing::ANALYSIS_VERSION` (call 6), so a stale one cannot be re-run against a
    /// later engine; [`ledger`] writes it into each entry.
    pub analysis_version: i64,
    /// Why, for the reviewer. Not copied into the ledger, which points at the file instead.
    pub note: String,
    /// Keys Rust's answer has and the recorded one lacks, each matched exactly.
    pub added: Vec<LedgerPath>,
    /// Values Rust's answer moves, each matched exactly.
    pub moved: Vec<LedgerPath>,
}

/// The file's shape before the checks that need more than one path at once.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DeclarationFile {
    analysis_version: i64,
    note: String,
    added: Vec<LedgerPath>,
    moved: Vec<LedgerPath>,
}

impl TryFrom<DeclarationFile> for Declaration {
    type Error = String;

    fn try_from(file: DeclarationFile) -> Result<Self, String> {
        // A path in both lists says a place both gained a key and moved a value, which no one
        // difference is; and twice in one list is a copy-paste that the ledger would echo.
        let mut seen = BTreeSet::new();
        for path in file.added.iter().chain(&file.moved) {
            if !seen.insert(path.as_str()) {
                return Err(format!("{path} is declared twice"));
            }
        }
        // The comparator reports an added key at the key's own path, so an `added` path ending in a
        // list index can never match. A list that grew is a `moved` list.
        if let Some(path) = file
            .added
            .iter()
            .find(|path| matches!(path.segments.last(), Some(Segment::Index(_))))
        {
            return Err(format!(
                "{path} is declared `added`, but it ends in a list index and an added key's path \
                 ends in the key; a list of another length is a `moved` list"
            ));
        }
        Ok(Self {
            analysis_version: file.analysis_version,
            note: file.note,
            added: file.added,
            moved: file.moved,
        })
    }
}

/// The declared paths one document's re-record matched, in the declaration's order.
///
/// Only [`gate`] makes one, so every path in it is a difference [`compare`] found between the pair
/// it was given. Empty is §M32's step 5: nothing differs, so nothing is written.
#[derive(Debug, Clone, PartialEq)]
pub struct Applied {
    added: Vec<LedgerPath>,
    moved: Vec<LedgerPath>,
}

impl Applied {
    pub fn added(&self) -> &[LedgerPath] {
        &self.added
    }

    pub fn moved(&self) -> &[LedgerPath] {
        &self.moved
    }

    /// Nothing differs — the second run of a re-record, and the verb writes nothing.
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.moved.is_empty()
    }
}

/// A difference the declaration does not cover. Its `Display` is the comparator's sentence plus why
/// the declaration did not let it through.
#[derive(Debug, Clone, PartialEq)]
pub struct Undeclared(pub Difference);

impl fmt::Display for Undeclared {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let why = match self.0.kind {
            DifferenceKind::AddedKey => "and the declaration's `added` does not name it",
            DifferenceKind::Moved { .. } => "and the declaration's `moved` does not name it",
            DifferenceKind::RemovedKey => "and a re-record never removes a key",
        };
        write!(f, "{} — {why}", self.0)
    }
}

/// Steps 2–3: compare `ours` against `committed`, and let through only what `declaration` names.
///
/// `ours` is the committed document with Rust's answer substituted — `expected` from `run` or
/// `stages` from `run_stages`, and `analysis_version` from the constant — so the version move and
/// the answer go through one comparison, at the document-rooted paths a declaration is written in.
/// An added key passes only at a declared `added` path, a moved value only at a declared `moved`
/// path, and a removed key never. Every refusal is returned, so one red run names them all.
pub fn gate(
    committed: &Value,
    ours: &Value,
    declaration: &Declaration,
) -> Result<Applied, Vec<Undeclared>> {
    let mut added = BTreeSet::new();
    let mut moved = BTreeSet::new();
    let mut undeclared = Vec::new();
    for difference in compare(committed, ours) {
        let (declared, matched) = match difference.kind {
            DifferenceKind::AddedKey => (&declaration.added, &mut added),
            DifferenceKind::Moved { .. } => (&declaration.moved, &mut moved),
            DifferenceKind::RemovedKey => {
                undeclared.push(Undeclared(difference));
                continue;
            }
        };
        if declared.iter().any(|path| path.as_str() == difference.path) {
            matched.insert(difference.path);
        } else {
            undeclared.push(Undeclared(difference));
        }
    }
    if !undeclared.is_empty() {
        return Err(undeclared);
    }
    let pick = |declared: &[LedgerPath], matched: &BTreeSet<String>| {
        declared
            .iter()
            .filter(|path| matched.contains(path.as_str()))
            .cloned()
            .collect()
    };
    Ok(Applied {
        added: pick(&declaration.added, &added),
        moved: pick(&declaration.moved, &moved),
    })
}

/// Step 4: the committed document with only `applied`'s paths taken from `ours`.
///
/// Every other value is the committed one, untouched — in-tolerance floats included, which is the
/// module doc's reason this starts from `committed` rather than from `ours`.
///
/// # Panics
///
/// If `applied` is not [`gate`]'s answer for this same pair. Every path it holds is one [`compare`]
/// found as a value in both documents (a move) or as a key under a parent both share (an addition),
/// so for the pair it came from both lookups below succeed.
pub fn apply(committed: &Value, ours: &Value, applied: &Applied) -> Value {
    let mut document = committed.clone();
    for path in applied.added.iter().chain(&applied.moved) {
        let value = path
            .get(ours)
            .unwrap_or_else(|| {
                panic!("{path}: not in `ours`, so `applied` is not gate's answer for this pair")
            })
            .clone();
        path.put(&mut document, value).unwrap_or_else(|| {
            panic!(
                "{path}: no parent in `committed`, so `applied` is not gate's answer for this pair"
            )
        });
    }
    document
}

/// Write the re-record into `document`'s own `provenance`: `oracle` if absent, and one `rerecords`
/// entry listing what `applied` matched (call 8) — not the whole declaration, so each file says
/// which of *its* values are Rust's.
///
/// `declaration_path` is recorded verbatim; the verb decides its spelling. Refuses an empty
/// `applied` (an entry for a re-record that moved nothing would be false, and §M32 step 5 writes
/// nothing instead), a document with no `provenance` object, and a `rerecords` that is not a list —
/// and writes nothing when it refuses.
pub fn ledger(
    document: &mut Value,
    applied: &Applied,
    declaration: &Declaration,
    declaration_path: &str,
) -> Result<(), String> {
    let id = document
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("<a document with no id>")
        .to_string();
    if applied.is_empty() {
        return Err(format!(
            "{id}: nothing was applied, so there is no re-record to write down (§M32 step 5)"
        ));
    }
    let provenance = document
        .get_mut("provenance")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| format!("{id}: no `provenance` object to write the ledger into"))?;
    if let Some(other) = provenance.get("rerecords").filter(|v| !v.is_array()) {
        return Err(format!(
            "{id}: `provenance.rerecords` is {other}, not a list"
        ));
    }

    let spelled = |paths: &[LedgerPath]| paths.iter().map(LedgerPath::as_str).collect::<Value>();
    let entry = json!({
        "analysis_version": declaration.analysis_version,
        "by": RERECORDED_BY,
        "declaration": declaration_path,
        "added": spelled(&applied.added),
        "moved": spelled(&applied.moved),
    });
    provenance
        .entry("oracle")
        .or_insert_with(|| json!(PYTHON_ORACLE));
    provenance
        .entry("rerecords")
        .or_insert_with(|| json!([]))
        .as_array_mut()
        .expect("checked to be a list, or just inserted as one")
        .push(entry);
    Ok(())
}

// --------------------------------------------------------------------------- the run [M32 P5]

/// The engine family's two halves under `spec/vectors/`. Each name is also the `provenance.kind`
/// its files carry, and the stage family mirrors the pair under [`STAGES`].
pub const ENGINE_HALVES: [&str; 2] = ["synthetic", "corpus"];

/// The stage family's directory under `spec/vectors/`, and the `provenance.kind` its files carry.
pub const STAGES: &str = "stages";

/// Which of Rust's answers a vector is re-recorded from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Family {
    /// `synthetic/` and `corpus/`: `expected` is [`crate::run`]'s answer.
    Engine,
    /// `stages/`: `stages` is [`run_stages`]' answer, composed onto its engine vector.
    Stage,
}

/// Why a run wrote nothing. Every variant is a refusal of the whole run (call 7).
#[derive(Debug, Clone, PartialEq)]
pub enum Refused {
    /// The declaration did not load, is at another version than this build (call 6), or sits where
    /// a ledger cannot point at it.
    Declaration(String),
    /// A vector could not be listed, read or parsed, or is not what its directory says it is. These
    /// stop the walk at once: they are a broken `spec/`, not a re-record's answer.
    Files(String),
    /// Every difference the declaration does not cover and every stage document that does not
    /// compose, across the whole run, each naming its vector.
    Gate(Vec<String>),
    /// Declared paths that matched no document and sit in no document's ledger (call 5).
    Unmatched(Vec<String>),
    /// Writing failed. Every file is staged beside its target before any is replaced, so a failure
    /// while staging leaves every vector as it was; the message says if one came later.
    Write(String),
}

impl fmt::Display for Refused {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Declaration(why) => write!(f, "the declaration is refused: {why}"),
            Self::Files(why) => write!(f, "the vectors cannot be walked: {why}"),
            Self::Gate(refusals) => {
                write!(
                    f,
                    "{} refusal(s); nothing was written anywhere:",
                    refusals.len()
                )?;
                refusals.iter().try_for_each(|r| write!(f, "\n  {r}"))
            }
            Self::Unmatched(paths) => {
                write!(
                    f,
                    "{} declared path(s) matched nothing; nothing was written anywhere:",
                    paths.len()
                )?;
                paths.iter().try_for_each(|p| write!(f, "\n  {p}"))
            }
            Self::Write(why) => write!(f, "writing failed: {why}"),
        }
    }
}

impl std::error::Error for Refused {}

/// One committed vector, and what the run does to it.
#[derive(Debug)]
pub struct Rerecorded {
    id: String,
    path: PathBuf,
    family: Family,
    applied: Applied,
    /// One line per applied path, with the value before and after — what the reviewer reads.
    lines: Vec<String>,
    /// The committed file's line ending, kept (module doc, "How a file is written").
    crlf: bool,
    /// The document as it will be written; `None` when nothing in it changed.
    written: Option<Value>,
}

impl Rerecorded {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn family(&self) -> Family {
        self.family
    }

    pub fn applied(&self) -> &Applied {
        &self.applied
    }

    /// The document [`Run::write`] writes for this vector, ledger included; `None` if it writes none.
    pub fn written(&self) -> Option<&Value> {
        self.written.as_ref()
    }
}

/// A gated, composed re-record that has not been written: [`plan`]'s answer. Holding one means every
/// vector in both families passed.
#[derive(Debug)]
pub struct Run {
    declaration: Declaration,
    vectors: Vec<Rerecorded>,
}

impl Run {
    /// Every vector the run read: the engine family, then the stage family; within each, the
    /// [`ENGINE_HALVES`] in their order, and each half in path order.
    pub fn vectors(&self) -> &[Rerecorded] {
        &self.vectors
    }

    /// The vectors [`Run::write`] would write.
    pub fn changed(&self) -> impl Iterator<Item = &Rerecorded> {
        self.vectors.iter().filter(|v| v.written.is_some())
    }

    /// Write every changed vector, and nothing else; the number written.
    ///
    /// Every file is rendered, then staged beside its target, and only then is any target replaced —
    /// so a disk that fills while staging leaves `spec/` exactly as it was, and the window in which a
    /// failure leaves the family half-written is the renames alone.
    pub fn write(&self) -> Result<usize, Refused> {
        let files: Vec<(&Path, Vec<u8>)> = self
            .changed()
            .map(|v| {
                let document = v
                    .written
                    .as_ref()
                    .expect("changed() yields written vectors");
                (v.path.as_path(), file_bytes(document, &v.path, v.crlf))
            })
            .collect();

        let mut staged: Vec<PathBuf> = Vec::with_capacity(files.len());
        for (path, bytes) in &files {
            let stage = staging_path(path);
            if let Err(e) = fs::write(&stage, bytes) {
                let _ = fs::remove_file(&stage);
                for earlier in &staged {
                    let _ = fs::remove_file(earlier);
                }
                return Err(Refused::Write(format!(
                    "staging {}: {e}; no vector was replaced",
                    stage.display()
                )));
            }
            staged.push(stage);
        }
        for (done, ((path, _), stage)) in files.iter().zip(&staged).enumerate() {
            fs::rename(stage, path).map_err(|e| {
                Refused::Write(format!(
                    "replacing {} with {}: {e}; {done} of {} vectors were already replaced, and \
                     the rest are staged beside their targets",
                    path.display(),
                    stage.display(),
                    files.len()
                ))
            })?;
        }
        Ok(files.len())
    }

    /// The review (§M32: the report is what gets reviewed, never `git diff`): every applied path of
    /// every changed vector, with its value before and after, then the totals.
    pub fn report(&self, dry_run: bool, files_written: usize) -> String {
        let d = &self.declaration;
        let mut out = format!(
            "golf-core rerecord: a declaration at analysis_version {}, {} added and {} moved — {}\n",
            d.analysis_version,
            d.added.len(),
            d.moved.len(),
            d.note
        );
        if dry_run {
            out.push_str("--dry-run: nothing is written\n");
        }
        for vector in self.changed() {
            writeln!(out, "{}", vector.id).expect("writing to a String does not fail");
            for line in &vector.lines {
                writeln!(out, "  {line}").expect("writing to a String does not fail");
            }
        }
        let engine = self
            .vectors
            .iter()
            .filter(|v| v.family == Family::Engine)
            .count();
        let changed = self.changed().count();
        if changed == 0 {
            out.push_str("nothing differs from the committed vectors\n");
        }
        writeln!(
            out,
            "{} vectors run ({engine} engine, {} stage); {changed} changed; {files_written} files \
             written",
            self.vectors.len(),
            self.vectors.len() - engine,
        )
        .expect("writing to a String does not fail");
        out
    }
}

/// Run, gate, apply, ledger and compose every engine and stage vector under `spec`, writing nothing.
///
/// `Ok` only if every vector in both families passed and every declared path matched (calls 5–7);
/// [`Run::write`] is then the only thing left to do, and `--dry-run` is not calling it.
pub fn plan(spec: &Path, declaration_file: &Path) -> Result<Run, Refused> {
    let declaration = load_declaration(declaration_file)?;
    if declaration.analysis_version != ANALYSIS_VERSION {
        return Err(Refused::Declaration(format!(
            "{} is at analysis_version {}, and this engine is at {ANALYSIS_VERSION}; a declaration \
             describes one version's change, so it is re-run against no other (M32's plan, call 6)",
            declaration_file.display(),
            declaration.analysis_version
        )));
    }
    let spelling = ledger_spelling(spec, declaration_file)?;
    let vectors_dir = spec.join("vectors");

    let mut vectors: Vec<Rerecorded> = Vec::new();
    let mut refusals: Vec<String> = Vec::new();
    let mut ledgered = Ledgered::default();
    // Every engine document as it will be written (as committed, if the gate refused it), for the
    // stage family's compose check — and the index of its `Rerecorded`, if it has one.
    let mut engines: BTreeMap<String, (Option<usize>, Value)> = BTreeMap::new();

    for half in ENGINE_HALVES {
        for path in vector_files(&vectors_dir.join(half))? {
            let (committed, crlf) = read_vector(&path)?;
            let id = identify(&committed, &path, half)?;
            ledgered.note(&committed, &declaration);
            let input = parse_input(&committed, &id)?;

            let mut ours = committed.clone();
            ours["expected"] = crate::run(&input);
            ours["analysis_version"] = json!(ANALYSIS_VERSION);

            let entry = match one(&id, &committed, &ours, &declaration, spelling.as_deref()) {
                Ok((applied, written, lines)) => {
                    vectors.push(Rerecorded {
                        id: id.clone(),
                        path,
                        family: Family::Engine,
                        applied,
                        lines,
                        crlf,
                        // Filled after the stage family, which composes onto it first.
                        written: None,
                    });
                    (Some(vectors.len() - 1), written)
                }
                Err(refused) => {
                    refusals.extend(refused);
                    (None, committed)
                }
            };
            if engines.insert(id.clone(), entry).is_some() {
                return Err(Refused::Files(format!(
                    "{id} is the id of two engine vectors"
                )));
            }
        }
    }
    if engines.is_empty() {
        return Err(Refused::Files(format!(
            "no engine vector under {}",
            vectors_dir.display()
        )));
    }

    for half in ENGINE_HALVES {
        for path in vector_files(&vectors_dir.join(STAGES).join(half))? {
            let (committed, crlf) = read_vector(&path)?;
            let id = identify(&committed, &path, STAGES)?;
            ledgered.note(&committed, &declaration);
            let derived_from = committed["provenance"]["derived_from"]
                .as_str()
                .ok_or_else(|| {
                    Refused::Files(format!("{id}: no `provenance.derived_from` to join it by"))
                })?;
            let (engine_index, engine) = engines.get(derived_from).ok_or_else(|| {
                Refused::Files(format!(
                    "{id} is derived from {derived_from}, which is not an engine vector here"
                ))
            })?;
            let input = parse_input(engine, derived_from)?;

            let mut ours = committed.clone();
            ours["stages"] = run_stages(&input);
            ours["analysis_version"] = json!(ANALYSIS_VERSION);

            match one(&id, &committed, &ours, &declaration, spelling.as_deref()) {
                Ok((applied, written, lines)) => {
                    // A refused engine vector has no document "as it will be written", and the run
                    // fails on its refusal anyway, so composing onto the committed one would only add
                    // noise to the report.
                    if engine_index.is_some() {
                        if let Err(why) = verify_compose(&written["stages"], engine) {
                            refusals.push(why);
                        }
                    }
                    let written = (!applied.is_empty()).then_some(written);
                    vectors.push(Rerecorded {
                        id,
                        path,
                        family: Family::Stage,
                        applied,
                        lines,
                        crlf,
                        written,
                    });
                }
                Err(refused) => refusals.extend(refused),
            }
        }
    }

    for (index, written) in engines.into_values() {
        if let Some(index) = index {
            if !vectors[index].applied.is_empty() {
                vectors[index].written = Some(written);
            }
        }
    }

    if !refusals.is_empty() {
        return Err(Refused::Gate(refusals));
    }
    if spelling.is_none() {
        if let Some(changed) = vectors.iter().find(|v| !v.applied.is_empty()) {
            return Err(Refused::Declaration(format!(
                "{} is outside the directory that holds {}, so a ledger cannot name it — and {} \
                 (and perhaps others) would carry that ledger. Commit the declaration under \
                 `spec/declarations/` (M32's decision 2)",
                declaration_file.display(),
                spec.display(),
                changed.id
            )));
        }
    }

    let unmatched = unmatched(&declaration, &vectors, &ledgered);
    if !unmatched.is_empty() {
        return Err(Refused::Unmatched(unmatched));
    }
    Ok(Run {
        declaration,
        vectors,
    })
}

/// One document's steps 2–4 and its ledger: what applied, the document as it will be written, and
/// the report's lines — or every refusal, each naming the vector.
///
/// With no `spelling` the ledger is left off; [`plan`] refuses the run if anything applied, once,
/// rather than once per vector.
fn one(
    id: &str,
    committed: &Value,
    ours: &Value,
    declaration: &Declaration,
    spelling: Option<&str>,
) -> Result<(Applied, Value, Vec<String>), Vec<String>> {
    let applied = gate(committed, ours, declaration).map_err(|refused| {
        refused
            .iter()
            .map(|u| format!("{id}: {u}"))
            .collect::<Vec<_>>()
    })?;
    let mut written = apply(committed, ours, &applied);
    let mut lines: Vec<String> = applied
        .added
        .iter()
        .map(|path| format!("added  {path} = {}", brief(path.get(&written))))
        .collect();
    lines.extend(applied.moved.iter().map(|path| {
        format!(
            "moved  {path}: {} -> {}",
            brief(path.get(committed)),
            brief(path.get(&written))
        )
    }));
    if let Some(spelling) = spelling.filter(|_| !applied.is_empty()) {
        ledger(&mut written, &applied, declaration, spelling).map_err(|e| vec![e])?;
    }
    Ok((applied, written, lines))
}

/// A value for one report line: compact JSON, cut short so a moved list does not swamp the review.
fn brief(value: Option<&Value>) -> String {
    const LONGEST: usize = 72;
    let Some(value) = value else {
        return "<absent>".to_string();
    };
    let text = value.to_string();
    match text.char_indices().nth(LONGEST) {
        Some((cut, _)) => format!("{}…", &text[..cut]),
        None => text,
    }
}

/// The declared paths already in some committed document's ledger at the declaration's version —
/// what the typo guard accepts in place of a match on a run where nothing differs.
#[derive(Debug, Default)]
struct Ledgered {
    added: BTreeSet<String>,
    moved: BTreeSet<String>,
}

impl Ledgered {
    fn note(&mut self, committed: &Value, declaration: &Declaration) {
        let entries = committed["provenance"]["rerecords"].as_array();
        for entry in entries.into_iter().flatten() {
            if entry["analysis_version"].as_i64() != Some(declaration.analysis_version)
                || entry["by"] != RERECORDED_BY
            {
                continue;
            }
            for (kind, into) in [("added", &mut self.added), ("moved", &mut self.moved)] {
                let paths = entry[kind].as_array().into_iter().flatten();
                into.extend(paths.filter_map(Value::as_str).map(str::to_string));
            }
        }
    }
}

/// Call 5: every declared path that matched no document in this run and sits in no ledger.
fn unmatched(
    declaration: &Declaration,
    vectors: &[Rerecorded],
    ledgered: &Ledgered,
) -> Vec<String> {
    let mut found: BTreeSet<(&str, &str)> = BTreeSet::new();
    for vector in vectors {
        found.extend(vector.applied.added.iter().map(|p| ("added", p.as_str())));
        found.extend(vector.applied.moved.iter().map(|p| ("moved", p.as_str())));
    }
    let mut unmatched = Vec::new();
    for (kind, declared, recorded) in [
        ("added", &declaration.added, &ledgered.added),
        ("moved", &declaration.moved, &ledgered.moved),
    ] {
        for path in declared {
            if found.contains(&(kind, path.as_str())) || recorded.contains(path.as_str()) {
                continue;
            }
            unmatched.push(format!(
                "{path} (declared `{kind}`): no vector differs there and no vector's ledger records \
                 it at v{} — a misspelling, or a value that moved within the tolerance or not at all, \
                 which the gate does not report as a difference (M32's plan, call 5)",
                declaration.analysis_version
            ));
        }
    }
    unmatched
}

fn load_declaration(path: &Path) -> Result<Declaration, Refused> {
    let text = fs::read_to_string(path)
        .map_err(|e| Refused::Declaration(format!("reading {}: {e}", path.display())))?;
    serde_json::from_str(&text)
        .map_err(|e| Refused::Declaration(format!("{}: {e}", path.display())))
}

/// The declaration's path as a ledger records it: relative to the directory holding `spec`, with
/// `/` between components, so the same committed file is the same string on every machine (M32 P4's
/// finding: a Windows absolute path would otherwise land in every file). `None` when the file is
/// outside that directory — fine for a run that writes nothing, refused by [`plan`] otherwise.
///
/// Refused outright under `spec/vectors/`, M32's decision 2: `conformance.py::vector_paths` globs
/// `*.json*` there and would read the declaration as a vector.
fn ledger_spelling(spec: &Path, declaration_file: &Path) -> Result<Option<String>, Refused> {
    let canonical = |path: &Path| {
        fs::canonicalize(path)
            .map_err(|e| Refused::Declaration(format!("resolving {}: {e}", path.display())))
    };
    let spec = canonical(spec)?;
    let file = canonical(declaration_file)?;
    if file.starts_with(spec.join("vectors")) {
        return Err(Refused::Declaration(format!(
            "{} is under spec/vectors/, where `conformance.py` would read it as a vector; \
             declarations live in `spec/declarations/` (M32's decision 2)",
            declaration_file.display()
        )));
    }
    let Some(relative) = spec.parent().and_then(|root| file.strip_prefix(root).ok()) else {
        return Ok(None);
    };
    let parts: Option<Vec<&str>> = relative
        .components()
        .map(|component| match component {
            Component::Normal(part) => part.to_str(),
            _ => None,
        })
        .collect();
    Ok(parts.map(|parts| parts.join("/")))
}

/// The vector files in one directory, sorted so a report and a failure name a stable order.
fn vector_files(dir: &Path) -> Result<Vec<PathBuf>, Refused> {
    let entries =
        fs::read_dir(dir).map_err(|e| Refused::Files(format!("listing {}: {e}", dir.display())))?;
    let mut paths = Vec::new();
    for entry in entries {
        let path = entry
            .map_err(|e| Refused::Files(format!("listing {}: {e}", dir.display())))?
            .path();
        let name = path.file_name().and_then(OsStr::to_str).unwrap_or_default();
        if name.ends_with(".json") || name.ends_with(".json.gz") {
            paths.push(path);
        }
    }
    paths.sort();
    Ok(paths)
}

fn is_gzipped(path: &Path) -> bool {
    path.extension().is_some_and(|e| e == "gz")
}

/// A committed vector, and whether its text is CRLF. Gzipped or not by the suffix, as `_read_json`.
fn read_vector(path: &Path) -> Result<(Value, bool), Refused> {
    let refused = |what: &str, e: &dyn fmt::Display| {
        Refused::Files(format!("{what} {}: {e}", path.display()))
    };
    let bytes = fs::read(path).map_err(|e| refused("reading", &e))?;
    let text = if is_gzipped(path) {
        let mut text = String::new();
        GzDecoder::new(&bytes[..])
            .read_to_string(&mut text)
            .map_err(|e| refused("gunzipping", &e))?;
        text
    } else {
        String::from_utf8(bytes).map_err(|e| refused("decoding", &e))?
    };
    let document = serde_json::from_str(&text).map_err(|e| refused("parsing", &e))?;
    Ok((document, text.contains("\r\n")))
}

/// The vector's `id`, after checking its `provenance.kind` is the one its directory holds — a
/// stage vector filed under `corpus/` would otherwise be run as an engine vector and refused for
/// reasons that name neither mistake.
fn identify(document: &Value, path: &Path, kind: &str) -> Result<String, Refused> {
    let id = document["id"]
        .as_str()
        .ok_or_else(|| Refused::Files(format!("{} carries no id", path.display())))?;
    match document["provenance"]["kind"].as_str() {
        Some(found) if found == kind => Ok(id.to_string()),
        found => Err(Refused::Files(format!(
            "{id} ({}) is under {kind}/ but its provenance.kind is {found:?}",
            path.display()
        ))),
    }
}

/// An engine vector's `input`, through the derived `Deserialize` — the trait method, so every
/// contract type's `Validate` runs (M32 P3's finding) — and without cloning the keypoints out.
fn parse_input(engine_vector: &Value, id: &str) -> Result<VectorInput, Refused> {
    VectorInput::deserialize(&engine_vector["input"]).map_err(|e| {
        Refused::Files(format!(
            "{id}: input does not parse into the ported shapes: {e}"
        ))
    })
}

/// The bytes of one vector file, as the module doc's "How a file is written" says.
fn file_bytes(document: &Value, path: &Path, crlf: bool) -> Vec<u8> {
    let mut text = serde_json::to_string_pretty(document).expect("a Value always serializes");
    let gzipped = is_gzipped(path);
    if !gzipped {
        text.push('\n');
    }
    if crlf {
        // Safe as a blanket replace: `serde_json` escapes a newline inside a string as `\n`, so
        // every raw one is a line break of the pretty printer's.
        text = text.replace('\n', "\r\n");
    }
    if !gzipped {
        return text.into_bytes();
    }
    // `GzipFile` stamps the file's name less `.gz` (`x.json`), as `file_stem` gives it.
    let name = path
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut encoder = GzBuilder::new()
        .filename(name)
        .mtime(0)
        .write(Vec::new(), Compression::best());
    encoder
        .write_all(text.as_bytes())
        .expect("writing to a Vec does not fail");
    encoder
        .finish()
        .expect("finishing into a Vec does not fail")
}

/// Where a file is staged before it replaces its target: beside it, so the rename stays on one
/// volume, and under a name [`vector_files`] does not list.
fn staging_path(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".rerecord-staged");
    path.with_file_name(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    const DECLARATION_PATH: &str = "spec/declarations/v17.json";

    fn declaration(added: &[&str], moved: &[&str]) -> Declaration {
        serde_json::from_value(json!({
            "analysis_version": 17,
            "note": "a test",
            "added": added,
            "moved": moved,
        }))
        .expect("a valid declaration")
    }

    /// §M32 "M32's declaration, in full".
    fn m32() -> Declaration {
        let mut added: Vec<String> = [
            "attack_angle",
            "dynamic_loft",
            "low_point",
            "impact_offset_h",
            "impact_offset_v",
            "impact_position_v",
            "carry_offline",
        ]
        .iter()
        .map(|key| format!("expected.swing.shot.{key}"))
        .collect();
        added.extend(
            ["parser_version", "fields_present", "corrections"]
                .iter()
                .map(|key| format!("expected.swing.shot.provenance.{key}")),
        );
        let added: Vec<&str> = added.iter().map(String::as_str).collect();
        declaration(&added, &["analysis_version", "expected.analysis_version"])
    }

    const RECORDED: f64 = 0.611_111_111_111_111;
    /// Inside the tolerance of [`RECORDED`] and not the same f64 — the same measurement summed in
    /// another order, which is what M32 P2 found 142 of in the stage family.
    const RESUMMED: f64 = 0.611_111_111_111_111_2;

    /// A corpus-shaped engine document as Python recorded it.
    fn committed() -> Value {
        json!({
            "id": "corpus/2026-08-10-1",
            "analysis_version": 16,
            "provenance": {"kind": "corpus", "note": "recorded by Python"},
            "input": {"shot": {"ball_speed": 130.0}},
            "expected": {
                "analysis_version": 16,
                "swing": {
                    "shot": {"ball_speed": 130.0, "provenance": {"device": "hd_golf"}},
                    "measurements": [{"name": "tempo_ratio", "value": RECORDED}],
                },
            },
        })
    }

    /// The same document as Rust answers it at 17: both versions moved, two of the new keys present
    /// (one per object, which is enough to reach both), and one float re-summed.
    fn ours() -> Value {
        let mut ours = committed();
        ours["analysis_version"] = json!(17);
        ours["expected"]["analysis_version"] = json!(17);
        ours["expected"]["swing"]["shot"]["attack_angle"] = json!(null);
        ours["expected"]["swing"]["shot"]["provenance"]["parser_version"] = json!(0);
        ours["expected"]["swing"]["measurements"][0]["value"] = json!(RESUMMED);
        ours
    }

    fn paths(undeclared: &[Undeclared]) -> Vec<&str> {
        undeclared.iter().map(|u| u.0.path.as_str()).collect()
    }

    #[test]
    fn an_undeclared_added_key_fails_and_names_its_path() {
        let partial = declaration(
            &["expected.swing.shot.provenance.parser_version"],
            &["analysis_version", "expected.analysis_version"],
        );
        let refused = gate(&committed(), &ours(), &partial).expect_err("an undeclared key");
        assert_eq!(paths(&refused), ["expected.swing.shot.attack_angle"]);
        assert_eq!(refused[0].0.kind, DifferenceKind::AddedKey);
        let message = refused[0].to_string();
        assert!(
            message.contains("expected.swing.shot.attack_angle") && message.contains("`added`"),
            "{message}"
        );
    }

    #[test]
    fn an_undeclared_moved_value_fails_and_names_its_path() {
        let partial = declaration(
            &[
                "expected.swing.shot.attack_angle",
                "expected.swing.shot.provenance.parser_version",
            ],
            &["analysis_version"],
        );
        let refused = gate(&committed(), &ours(), &partial).expect_err("an undeclared move");
        assert_eq!(paths(&refused), ["expected.analysis_version"]);
        assert_eq!(
            refused[0].0.kind,
            DifferenceKind::Moved {
                expected: json!(16),
                actual: json!(17)
            }
        );
        assert!(refused[0].to_string().contains("`moved`"));
    }

    /// One red run names every refusal, not the first — the reviewer fixes a declaration once.
    #[test]
    fn every_undeclared_difference_is_named_at_once() {
        let refused = gate(&committed(), &ours(), &declaration(&[], &[])).expect_err("four");
        let mut found = paths(&refused);
        found.sort_unstable();
        assert_eq!(
            found,
            [
                "analysis_version",
                "expected.analysis_version",
                "expected.swing.shot.attack_angle",
                "expected.swing.shot.provenance.parser_version",
            ]
        );
    }

    /// Declared or not, a dropped key is refused: no declaration kind covers it, so naming the path
    /// under `moved` changes nothing.
    #[test]
    fn a_removed_key_never_passes() {
        let mut dropped = ours();
        dropped["expected"]["swing"]["shot"]
            .as_object_mut()
            .expect("a shot object")
            .remove("ball_speed");
        let mut paths_declared = m32();
        paths_declared
            .moved
            .push(LedgerPath::parse("expected.swing.shot.ball_speed").expect("a path"));
        let refused = gate(&committed(), &dropped, &paths_declared).expect_err("a removal");
        assert_eq!(paths(&refused), ["expected.swing.shot.ball_speed"]);
        assert_eq!(refused[0].0.kind, DifferenceKind::RemovedKey);
        assert!(refused[0].to_string().contains("never removes a key"));
    }

    #[test]
    fn a_declared_added_key_and_a_declared_moved_value_land() {
        let (committed, ours) = (committed(), ours());
        let applied = gate(&committed, &ours, &m32()).expect("everything is declared");
        let spelled = |paths: &[LedgerPath]| {
            paths
                .iter()
                .map(|p| p.as_str().to_string())
                .collect::<Vec<_>>()
        };
        // In the declaration's order, not the walk's.
        assert_eq!(
            spelled(applied.added()),
            [
                "expected.swing.shot.attack_angle",
                "expected.swing.shot.provenance.parser_version"
            ]
        );
        assert_eq!(
            spelled(applied.moved()),
            ["analysis_version", "expected.analysis_version"]
        );

        let written = apply(&committed, &ours, &applied);
        assert_eq!(written["analysis_version"], json!(17));
        assert_eq!(written["expected"]["analysis_version"], json!(17));
        let shot = &written["expected"]["swing"]["shot"];
        assert!(shot
            .as_object()
            .expect("a shot")
            .contains_key("attack_angle"));
        assert!(shot["attack_angle"].is_null());
        assert_eq!(shot["provenance"]["parser_version"], json!(0));
    }

    /// ADR-035 clause 3's "every value Python recorded survives in the file", checked two ways: the
    /// re-summed float keeps Python's bits, and undoing the declared paths gives back the committed
    /// document exactly — `Value`'s `==` compares floats as f64s, so a bit anywhere would show.
    #[test]
    fn every_undeclared_value_is_the_committed_one_bit_for_bit() {
        assert_ne!(
            RECORDED.to_bits(),
            RESUMMED.to_bits(),
            "the fixture needs two different f64s, or this test checks nothing"
        );
        let (committed, ours) = (committed(), ours());
        let applied = gate(&committed, &ours, &m32()).expect("everything is declared");
        let written = apply(&committed, &ours, &applied);

        let value = written["expected"]["swing"]["measurements"][0]["value"]
            .as_f64()
            .expect("a float");
        assert_eq!(value.to_bits(), RECORDED.to_bits());

        let mut undone = written;
        for path in applied.added() {
            let (Segment::Key(key), parent) = path.segments().split_last().expect("not the root")
            else {
                panic!("{path}: an added path ends in a key");
            };
            let parent = parent
                .iter()
                .try_fold(&mut undone, |node, segment| match segment {
                    Segment::Key(step) => node.get_mut(step.as_str()),
                    Segment::Index(index) => node.get_mut(*index),
                })
                .expect("the parent is there");
            parent.as_object_mut().expect("an object").remove(key);
        }
        for path in applied.moved() {
            let recorded = path.get(&committed).expect("recorded").clone();
            path.put(&mut undone, recorded).expect("there");
        }
        assert_eq!(undone, committed);
    }

    /// Step 5. An identical pair, and a pair that differs only inside the tolerance — the second
    /// run of a re-record, with Rust's floats as they always come out — both apply nothing, and the
    /// ledger will not record a re-record that did not happen.
    #[test]
    fn an_identical_pair_applies_nothing() {
        let committed = committed();
        let mut resummed = committed.clone();
        resummed["expected"]["swing"]["measurements"][0]["value"] = json!(RESUMMED);
        for ours in [&committed, &resummed] {
            let applied = gate(&committed, ours, &m32()).expect("nothing to refuse");
            assert!(applied.is_empty(), "{applied:?}");
            assert_eq!(apply(&committed, ours, &applied), committed);

            let mut document = committed.clone();
            let refused = ledger(&mut document, &applied, &m32(), DECLARATION_PATH)
                .expect_err("an empty re-record is not written down");
            assert!(refused.contains("nothing was applied"), "{refused}");
            assert_eq!(document, committed, "a refused ledger wrote something");
        }
    }

    #[test]
    fn a_wildcard_in_a_declaration_is_refused() {
        let refused = serde_json::from_value::<Declaration>(json!({
            "analysis_version": 17,
            "note": "",
            "added": [],
            "moved": ["expected.swing.checkpoint_scores[*].message"],
        }))
        .expect_err("a wildcard");
        assert!(refused.to_string().contains("wildcard"), "{refused}");
    }

    /// Call 8, on the three shapes M32 re-records: a corpus engine vector adds the keys it matched
    /// and moves both versions; a synthetic one carries no shot and moves the versions alone; a
    /// stage vector has only the top-level version. The same declaration serves all three, and
    /// each entry lists what matched *that* document.
    #[test]
    fn the_ledger_lists_only_what_this_document_matched() {
        let corpus = (committed(), ours());

        let mut synthetic = committed();
        synthetic["id"] = json!("synthetic/baseline-3to1");
        synthetic["expected"]["swing"]["shot"] = json!(null);
        let mut synthetic_ours = synthetic.clone();
        synthetic_ours["analysis_version"] = json!(17);
        synthetic_ours["expected"]["analysis_version"] = json!(17);

        let stage = json!({
            "id": "stages/synthetic/baseline-3to1",
            "analysis_version": 16,
            "provenance": {"derived_from": "synthetic/baseline-3to1", "kind": "stages"},
            "stages": {"phases": [{"start_frame": 0}]},
        });
        let mut stage_ours = stage.clone();
        stage_ours["analysis_version"] = json!(17);

        let cases = [
            (
                corpus,
                json!([
                    "expected.swing.shot.attack_angle",
                    "expected.swing.shot.provenance.parser_version"
                ]),
                json!(["analysis_version", "expected.analysis_version"]),
            ),
            (
                (synthetic, synthetic_ours),
                json!([]),
                json!(["analysis_version", "expected.analysis_version"]),
            ),
            ((stage, stage_ours), json!([]), json!(["analysis_version"])),
        ];
        for ((committed, ours), added, moved) in cases {
            let applied = gate(&committed, &ours, &m32()).expect("declared");
            let mut written = apply(&committed, &ours, &applied);
            ledger(&mut written, &applied, &m32(), DECLARATION_PATH).expect("a ledger");
            assert_eq!(
                written["provenance"]["rerecords"],
                json!([{
                    "analysis_version": 17,
                    "by": "golf-core rerecord",
                    "declaration": DECLARATION_PATH,
                    "added": added,
                    "moved": moved,
                }]),
                "{}",
                committed["id"]
            );
            // And the provenance that was there is still there.
            for (key, value) in committed["provenance"].as_object().expect("provenance") {
                assert_eq!(&written["provenance"][key], value, "provenance.{key}");
            }
        }
    }

    /// `oracle` is written as `"python"` where absent and never over a value already there; a
    /// second entry appends rather than replacing the first; and a document the ledger cannot be
    /// written into is refused untouched.
    #[test]
    fn the_oracle_is_written_when_absent_and_never_overwritten() {
        let (committed, ours) = (committed(), ours());
        let applied = gate(&committed, &ours, &m32()).expect("declared");

        let mut first = apply(&committed, &ours, &applied);
        ledger(&mut first, &applied, &m32(), DECLARATION_PATH).expect("a ledger");
        assert_eq!(first["provenance"]["oracle"], json!("python"));
        ledger(&mut first, &applied, &m32(), "spec/declarations/v18.json").expect("a second");
        let entries = first["provenance"]["rerecords"].as_array().expect("a list");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0]["declaration"], json!(DECLARATION_PATH));

        let mut named = apply(&committed, &ours, &applied);
        named["provenance"]["oracle"] = json!("rust");
        ledger(&mut named, &applied, &m32(), DECLARATION_PATH).expect("a ledger");
        assert_eq!(named["provenance"]["oracle"], json!("rust"));

        for broken in [
            json!({"id": "x", "provenance": null}),
            json!({"id": "x", "provenance": {"rerecords": {}}}),
        ] {
            let mut document = broken.clone();
            assert!(ledger(&mut document, &applied, &m32(), DECLARATION_PATH).is_err());
            assert_eq!(document, broken, "a refused ledger wrote something");
        }
    }

    /// Call 3's spelling, both ways: every good spelling parses to itself, and every path the
    /// comparator writes parses back to the string it wrote — so a declared path and a difference's
    /// path are the same string for the same place.
    #[test]
    fn a_ledger_path_is_spelled_the_way_the_comparator_spells_it() {
        for good in [
            "analysis_version",
            "expected.swing.shot.attack_angle",
            "a.b[1].c",
            "a[0][12]",
        ] {
            assert_eq!(LedgerPath::parse(good).expect(good).as_str(), good);
        }
        assert_eq!(
            LedgerPath::parse("a.b[1].c").expect("a path").segments(),
            [
                Segment::Key("a".into()),
                Segment::Key("b".into()),
                Segment::Index(1),
                Segment::Key("c".into())
            ]
        );

        let found = compare(
            &json!({"a": {"b": [0, {"c": 1}], "l": [[1, 2]]}, "z": 1}),
            &json!({"a": {"b": [0, {"c": 2, "d": true}], "l": [[1, 3]]}, "z": 2}),
        );
        assert_eq!(found.len(), 4, "{found:?}");
        for difference in &found {
            let path = LedgerPath::parse(&difference.path).expect("the comparator's own spelling");
            assert_eq!(path.as_str(), difference.path);
            assert!(path
                .get(&json!({"a": {"b": [0, {"c": 2, "d": true}], "l": [[1, 3]]}, "z": 2}))
                .is_some());
        }
    }

    #[test]
    fn a_ledger_path_refuses_every_spelling_the_comparator_would_not_write() {
        for (bad, why) in [
            ("", "whole document"),
            (".analysis_version", "leading dot"),
            ("a[*]", "wildcard"),
            ("a..b", "starts with a key"),
            ("a.", "starts with a key"),
            ("[0]", "starts with a key"),
            ("a]b", "starts with a key"),
            ("a[1", "no `]`"),
            ("a[]", "not a list index"),
            ("a[x]", "not a list index"),
            ("a[-1]", "not a list index"),
            ("a[0]b", "follows a list index"),
            ("a[01]", "spells this place \"a[1]\""),
            ("a[+1]", "spells this place \"a[1]\""),
        ] {
            let refused = LedgerPath::parse(bad).expect_err(bad);
            assert!(refused.contains(why), "{bad:?}: {refused}");
        }
    }

    /// The checks a declaration needs beyond each path parsing: no unknown key (a misspelled list
    /// would otherwise declare nothing), every key present, no path twice, and no `added` path that
    /// could never match.
    #[test]
    fn a_declaration_that_cannot_mean_what_it_says_does_not_load() {
        for (file, why) in [
            (
                json!({"analysis_version": 17, "note": "", "added": [], "moved": [], "moves": []}),
                "unknown field",
            ),
            (
                json!({"analysis_version": 17, "added": [], "moved": []}),
                "missing field",
            ),
            (
                json!({"analysis_version": 17, "note": "", "added": ["a"], "moved": ["a"]}),
                "declared twice",
            ),
            (
                json!({"analysis_version": 17, "note": "", "added": [], "moved": ["a", "a"]}),
                "declared twice",
            ),
            (
                json!({"analysis_version": 17, "note": "", "added": ["a.b[2]"], "moved": []}),
                "ends in a list index",
            ),
            (
                json!({"analysis_version": 17, "note": "", "added": [], "moved": [".a"]}),
                "leading dot",
            ),
        ] {
            let refused =
                serde_json::from_value::<Declaration>(file.clone()).expect_err(&file.to_string());
            assert!(refused.to_string().contains(why), "{file}: {refused}");
        }
        assert_eq!(m32().added.len() + m32().moved.len(), 12);
        assert_eq!(m32().analysis_version, 17);
    }

    /// The writer's bytes, against what `_write_json` and `GzipFile` wrote into the committed files:
    /// a plain file keeps its CRLF and ends in a newline; a gzipped one is LF, ends without one, and
    /// carries `GzipFile`'s header — the name less `.gz`, mtime 0, level 9's XFL, OS "unknown".
    #[test]
    fn a_file_is_written_the_way_the_committed_ones_were() {
        let document = json!({"b": [1, 2.5], "a": {"x": null}, "c": "line\nbreak"});

        let plain =
            String::from_utf8(file_bytes(&document, Path::new("v.json"), true)).expect("utf-8");
        assert!(plain.starts_with("{\r\n  \"a\": {\r\n    \"x\": null\r\n  },\r\n"));
        assert!(plain.ends_with("}\r\n"));
        assert!(!plain.replace("\r\n", "").contains('\n'), "a bare LF");
        assert!(
            plain.contains(r#""line\nbreak""#),
            "the string's newline is still escaped"
        );
        let lf = file_bytes(&document, Path::new("v.json"), false);
        assert!(!lf.contains(&b'\r'));
        assert_eq!(
            serde_json::from_str::<Value>(&plain).expect("parses"),
            document
        );

        let gz = file_bytes(&document, Path::new("2026-08-10-1.json.gz"), false);
        assert_eq!(&gz[..4], b"\x1f\x8b\x08\x08", "magic, deflate, FNAME");
        assert_eq!(&gz[4..8], [0, 0, 0, 0], "mtime 0");
        assert_eq!(&gz[8..10], [2, 255], "XFL for level 9, OS unknown");
        assert_eq!(&gz[10..28], b"2026-08-10-1.json\0");
        let mut text = String::new();
        GzDecoder::new(&gz[..])
            .read_to_string(&mut text)
            .expect("gunzips");
        assert!(text.ends_with("\n}") && !text.contains('\r'));
        assert_eq!(
            serde_json::from_str::<Value>(&text).expect("parses"),
            document
        );
    }

    #[test]
    fn a_report_value_is_cut_short_on_a_character_boundary() {
        assert_eq!(brief(None), "<absent>");
        assert_eq!(brief(Some(&json!(17))), "17");
        let long = brief(Some(&json!("é".repeat(100))));
        assert!(long.ends_with('…'));
        assert_eq!(long.chars().count(), 73);
    }
}
