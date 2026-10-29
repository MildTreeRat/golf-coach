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
//! - **No removal in an engine declaration.** §M32 names two kinds a declaration covers, an added
//!   key and a moved value. A [`DifferenceKind::RemovedKey`] is never declarable there, so a port
//!   that drops a key from the engine's answer cannot re-record its way past the drop.
//! - **A screen declaration names its removals, path by path** (M34 P10, the user's answer to P7's
//!   finding 3). The M34 plan's decision 3 makes a field the tie rule withholds *located, unread*:
//!   in `fields_present`, with no `raw_fields` key, and that absence is the structural mark M35 and
//!   M37 grade `misread`. The screen family's first re-record therefore has to drop keys Python
//!   recorded, so a screen declaration may carry `removed`, which works as `added` does — each path
//!   exact, ending in a key, held to the typo guard, and written into the ledger — and [`apply`]
//!   deletes the key. The rule's purpose survives the exception: a key still never goes *silently*,
//!   because the report lists every removal as it lists every addition. An engine declaration that
//!   carries the key at all is refused at load, so M32's rule stands there unchanged, and frozen
//!   Python's `frozen_view` and `ledger_covers`, which read only engine ledgers, learn nothing new.
//!   A removal for both families was rejected because it would teach frozen Python a case no
//!   engine change has needed; and keeping the rule by giving a withheld field a `raw_fields` entry
//!   was rejected because it erases the mark M35 reads.
//! - **Not the root, and not Python's spelling.** `""` would declare the whole document, and
//!   `.analysis_version` — `compare_results`' spelling — would match nothing here, declaring nothing
//!   while looking right. Both are refused, and so is any spelling [`compare`] would not produce
//!   (`[01]`, `[+1]`, `a..b`), which is checked by re-rendering the parsed path. Both languages
//!   *parse* a ledger path and walk the document with it; neither matches Python's spelling as a
//!   string.
//! - **A key holding `.`, `[` or `]` cannot be named.** No committed engine or stage document has one
//!   (scanned 2026-10-01); a document that grows one needs an escape, and this parser is where it
//!   goes. The storage family's op cases are the first that do (M36 P13): every file a store wrote
//!   is a key of `expected.files` named by its path, `…/manifest.json`. The gate matches a declared
//!   path to a difference as a string, so such a path would match and then walk to another place;
//!   the run refuses it by name instead (`a_declared_path_through_a_dotted_key_is_refused_by_name`).
//!   Nothing in M36 moves a store's write, so the escape waits for the change that does.
//!
//! An entry that matches nothing *in one document* is not an error here: the synthetic vectors carry
//! no shot and match none of M32's added keys. Whether an entry matched *anywhere* is the run's
//! check (call 5), made across every document's [`Applied`].
//!
//! **A list is named at its own path when its length changes, and at each index when it does
//! not** (M34 P7, asked before the screen family's first re-record, whose warnings lists grow).
//! [`compare`] reports a list of another length as one [`DifferenceKind::Moved`] of the whole list,
//! never as a move per index past the shorter end, so `expected.parsed.warnings` declared `moved`
//! matches it and [`apply`] copies the whole list across. A list of the *same* length whose entries
//! differ is a move per differing index, which the list's own path does not cover. A declaration
//! that has both kinds across its vectors names both spellings, and the typo guard holds each to
//! having matched somewhere. `a_list_is_declared_whole_when_its_length_changes` pins all three.
//!
//! # Three version keys, one gate (M34 P7, M36 P13)
//!
//! A declaration names the family it re-records by the version key it carries, and carries exactly
//! one (the M34 plan's call 6). `analysis_version` is M32's: the engine and stage families, run
//! through [`crate::run`] and [`run_stages`], and since M35 P6 the shot family, run through
//! [`run_shot`] — under the engine's key because every number a shot case holds is the engine's
//! (the M35 plan's call 7), so a bump that moves a swing's shot row moves its shot case's in the
//! same run, or the gate refuses it. Since M29 P19 it walks the selection family after the shot
//! family, run through [`run_selection`], because which descent a clip is windowed on is the
//! engine's rule, and then the mcp family, because `simulate_flight` answers from the engine's
//! flight model ([`walk_mcp`]'s doc). `screen_parser_version` is the screen family's, the
//! documents under `vectors/screen/`, run through [`run_screen`]. `career_version` is the many-shot
//! layer's (M36 P13, the M36 plan's decision 8): the storage and career families together, run
//! through [`run_storage`] and [`run_career`], the runners their gates call, since M37 P11 the
//! profile family after them, run through [`run_profile_case`] (the M37 plan's call 1), and since
//! M29 P15 the mcp family last, answered by the `golf-mcp` binary ([`walk_mcp`], the M29 plan's
//! call 1). One key for four families, because each of the first three adopts the one before it (a
//! career twin's corpus is a storage case's answer, and a profile corpus case's input is a career
//! case's), and the server's tools read the corpus, the bag and the aggregates the three hold, so a
//! change to what the reader produces moves all four ([`CAREER_VERSION`]'s doc). A run never reads another key's files, so
//! a screen re-record cannot move an engine vector, nor a career one a screen vector: each key ages
//! a different record, and a declaration describes one version's change of one of them. Everything
//! else — the gate, [`apply`], [`ledger`], the typo guard and the writer — is the same code for all
//! three, so the screen and career families are held to every rule M32's re-record was. A career
//! declaration, like an engine one, may not carry `removed`: nothing either family holds has needed
//! a key to go.
//!
//! # The run
//!
//! [`plan`] walks one family and does everything but write. For an engine declaration that is the
//! engine family (`vectors/synthetic/`, `vectors/corpus/`), the stage family
//! (`vectors/stages/{synthetic,corpus}/`), the shot family (`vectors/shot/`, flat, M35 P6), the
//! [`SELECTION_DOCUMENTS`] under `vectors/selection/` and then the [`MCP_DOCUMENTS`] (M29 P19);
//! for a screen declaration, the [`SCREEN_DOCUMENTS`] under
//! `vectors/screen/`; for a career declaration, the [`STORAGE_DOCUMENTS`] under `vectors/storage/`,
//! the [`CAREER_DOCUMENTS`] under `vectors/career/`, the [`PROFILE_DOCUMENTS`] under
//! `vectors/profile/` (M37 P11) and then the [`MCP_DOCUMENTS`] under `vectors/mcp/` (M29 P15).
//!
//! - **The version guard first** (call 6): a declaration not at its family's constant —
//!   [`ANALYSIS_VERSION`], [`SCREEN_PARSER_VERSION`] or [`CAREER_VERSION`] — is refused before a
//!   vector is read, so a stale one cannot be re-run against a later engine, parser or layer.
//! - **Each engine vector** is run through [`crate::run`], gated, applied and ledgered. **Each stage
//!   vector** is joined to its engine vector by `provenance.derived_from`, run through
//!   [`run_stages`], gated, applied, ledgered — and then composed onto the engine document *as it
//!   will be written* (call 9), so the family is held to the bundle answer it is about to sit beside.
//!   **Each screen document** is run through [`run_screen`], gated, applied and ledgered; nothing
//!   composes onto it, because nothing else holds a copy of a screen's answer.
//! - **Each shot case** is run through [`run_shot`], gated, applied and ledgered, and keeps
//!   `provenance.oracle: "hand"` as [`ledger`] finds it. Nothing composes onto it either, though a
//!   `corpus-*` case's rows are its corpus vector's at another path (`expected.measurements[i]`
//!   against `expected.swing.measurements[j]`): the two are held together by the gates, not here —
//!   `tests/shot.rs` holds the case to [`run_shot`] and `crates/analysis/tests/shot.rs`' Decision-12
//!   pin holds [`run_shot`]'s engine to the corpus vector — so a declaration that moves the one row
//!   and not the other is refused at the row it forgot (`walk_shot`'s doc).
//! - **Each selection case** is run through [`run_selection`], gated, applied and ledgered. Since
//!   M29 P25 every call is answered, `_auto_windows` included, so nothing is carried; a call the
//!   runner does not answer would be carried as committed, and refused if a call it is built from
//!   moved (`walk_selection`'s doc).
//! - **Each storage document** is run through [`run_storage`], gated, applied and ledgered. **Each
//!   career document** is run through [`run_career`], with the report text of each script no
//!   renderer runs yet carried as committed
//!   ([`CARRIED`](crate::career_family::CARRIED)), and a career document that adopts a storage
//!   corpus is run on that corpus *as it will be written*, its copy compared like any other value —
//!   the storage family's counterpart of the compose check (`walk_career`'s doc). **Each profile
//!   case** is run through [`run_profile_case`], and a corpus case, whose input is a career case's,
//!   is run on that career case's input *as it will be written*, one link further (M37 P11).
//! - **Each mcp case**, under a career declaration and under an engine one, is answered by
//!   `golf-mcp answer-case`, a process, because nothing may depend on `golf-mcp`; it is gated against the committed answers as that server answers them
//!   on purpose and written back onto the committed ones, so `golf_mcp::family::DECLARED`'s paths
//!   keep frozen Python's values in the file (M29 P15, [`walk_mcp`]'s doc).
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
//! does not enable `preserve_order`), a trailing newline on plain `.json` and none inside a `.gz`,
//! and gzip at level 9 with mtime 0 and the uncompressed name in the header, as Python's `GzipFile`
//! stamps it. Three things are matched to the *committed file* rather than to `_write_json`'s
//! source, because the source does not decide them:
//!
//! - **Line endings.** `_write_json`'s `write_text` translates `\n` to the platform's newline, and
//!   `spec/** -text` stores the bytes as they were written — so every plain `.json` vector in `spec/`
//!   is CRLF, because the repo is developed on Windows (`git ls-files --eol`, 2026-10-01: all 17
//!   plain files `i/crlf`), and every gzipped one is LF, because `GzipFile` is a binary stream. A
//!   writer that picked one would rewrite every line of the other kind on the first re-record; one
//!   that kept the file's own makes the same choice on every machine.
//! - **The indent** (M35 P6). `_write_json` indents by 2, and so does every file it wrote. Two
//!   families worked by hand were written at 4: the screen family's `hand/` (M34 P8) and the whole
//!   shot family (M35 P4–P5), 32 files on 2026-10-09 against every other vector's 2 (the storage
//!   family's `hand/` among them). The shot family is the first a routine engine bump is expected
//!   to rewrite, so a writer
//!   that indented by 2 would rewrite every line of each case it touched, and leave the family in
//!   two layouts. The indent is the second line's, which is the first key's: every vector is an
//!   object, and a file of one line keeps 2.
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
use std::process::{Command, Stdio};

use contracts::career::CAREER_VERSION;
use contracts::shot::SCREEN_PARSER_VERSION;
use contracts::swing::ANALYSIS_VERSION;
use flate2::read::GzDecoder;
use flate2::{Compression, GzBuilder};
use screen::orient::label_ratio;
use screen::parser::parse_screen;
use screen::profile::{load_profile, UnknownProfile};
use screen::ScreenInput;
use serde::{Deserialize, Serialize as _};
use serde_json::ser::PrettyFormatter;
use serde_json::{json, Map, Serializer, Value};

use crate::career_family::{adopted_from, carry, run_career};
use crate::compare::{compare, Difference, DifferenceKind};
use crate::profile_family::{self, run_profile_case, ADOPTED};
use crate::selection_family::{self, run_selection};
use crate::stages::{run_stages, verify_compose};
use crate::storage_family::run_storage;
use crate::{run_shot, ShotInput, VectorInput};

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

/// What `provenance.oracle` says on a vector Rust's own run recorded, with nothing independent
/// behind it: ADR-032's twelfth addendum's third kind, as `profile/corpus/` and `mcp/rust/` carry
/// it, and what `golf-lab build-vectors` writes into every vector it builds (M29 P30). The ledger
/// keeps it, because [`ledger`] writes [`PYTHON_ORACLE`] only where no oracle is named.
pub const RUST_ORACLE: &str = "rust";

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
        let (last, parent) = self.parent_mut(document)?;
        match last {
            Segment::Key(key) => {
                parent.as_object_mut()?.insert(key.clone(), value);
            }
            Segment::Index(index) => *parent.as_array_mut()?.get_mut(*index)? = value,
        }
        Some(())
    }

    /// Take the key at this path out of its object, returning what it held. `None` if the key is not
    /// there, or the path ends in a list index — a removal is of a key, never of a list element,
    /// because the comparator reports a shorter list as a moved list.
    fn remove(&self, document: &mut Value) -> Option<Value> {
        let (Segment::Key(key), parent) = self.parent_mut(document)? else {
            return None;
        };
        parent.as_object_mut()?.remove(key)
    }

    /// The last step and the node it steps from.
    fn parent_mut<'a>(&'a self, document: &'a mut Value) -> Option<(&'a Segment, &'a mut Value)> {
        let (last, parent) = self.segments.split_last()?;
        let parent = parent
            .iter()
            .try_fold(document, |node, segment| match segment {
                Segment::Key(key) => node.get_mut(key.as_str()),
                Segment::Index(index) => node.get_mut(*index),
            })?;
        Some((last, parent))
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

/// The version a re-record moves to, and by its key the family it re-records (the M34 plan's
/// call 6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Version {
    /// `analysis_version`: the engine and stage families (M32).
    Analysis(i64),
    /// `screen_parser_version`: the screen family alone (M34).
    ScreenParser(i64),
    /// `career_version`: the storage and career families together (M36 P13, the M36 plan's
    /// decision 8).
    Career(i64),
}

impl Version {
    /// The key a declaration, a vector's top level and a ledger entry all spell this version with.
    pub fn key(self) -> &'static str {
        match self {
            Self::Analysis(_) => "analysis_version",
            Self::ScreenParser(_) => "screen_parser_version",
            Self::Career(_) => "career_version",
        }
    }

    pub fn number(self) -> i64 {
        match self {
            Self::Analysis(n) | Self::ScreenParser(n) | Self::Career(n) => n,
        }
    }

    /// This build's version for the family: what the guard holds a declaration to, and what Rust's
    /// side of every comparison carries at the top level.
    pub fn current(self) -> i64 {
        match self {
            Self::Analysis(_) => ANALYSIS_VERSION,
            Self::ScreenParser(_) => SCREEN_PARSER_VERSION,
            Self::Career(_) => CAREER_VERSION,
        }
    }

    /// What has that version, for a refusal to name.
    fn of(self) -> &'static str {
        match self {
            Self::Analysis(_) => "engine",
            Self::ScreenParser(_) => "screen parser",
            Self::Career(_) => "many-shot layer",
        }
    }

    /// Whether a declaration at this version may name removed keys: a screen one alone (M34 P10).
    fn may_remove(self) -> bool {
        matches!(self, Self::ScreenParser(_))
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.key(), self.number())
    }
}

/// What a re-record may change, read from `spec/declarations/` (M32's decision 2).
///
/// Every key is required but the version, of which there must be exactly one of three, and
/// `removed`, which only a screen declaration may carry; no other key is allowed. A misspelled
/// `"moves"` would otherwise declare nothing and still load, and a gate fed an empty declaration
/// refuses everything — which fails safe, but names the wrong mistake.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(try_from = "DeclarationFile")]
pub struct Declaration {
    /// The version the re-record moves to, whose key picks the family (call 6). The verb refuses a
    /// declaration not at its family's constant, so a stale one cannot be re-run against a later
    /// engine or parser; [`ledger`] writes it into each entry under the same key.
    pub version: Version,
    /// Why, for the reviewer. Not copied into the ledger, which points at the file instead.
    pub note: String,
    /// Keys Rust's answer has and the recorded one lacks, each matched exactly.
    pub added: Vec<LedgerPath>,
    /// Values Rust's answer moves, each matched exactly.
    pub moved: Vec<LedgerPath>,
    /// Keys the recorded answer has and Rust's lacks, each matched exactly. Empty on every engine
    /// and career declaration, which the load refuses to give one (the module doc, "What a
    /// declaration can name"). [M34 P10]
    pub removed: Vec<LedgerPath>,
}

/// The file's shape before the checks that need more than one key at once.
///
/// The three version keys are optional *here* so that the check that exactly one is present can
/// name the mistake; `serde` alone would say "missing field" of whichever key it tried first.
/// `removed` is optional for good: an engine or career declaration must not carry it, and
/// `v17.json` predates it. A misspelling of it is still refused, by `deny_unknown_fields`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DeclarationFile {
    #[serde(default)]
    analysis_version: Option<i64>,
    #[serde(default)]
    screen_parser_version: Option<i64>,
    #[serde(default)]
    career_version: Option<i64>,
    note: String,
    added: Vec<LedgerPath>,
    moved: Vec<LedgerPath>,
    #[serde(default)]
    removed: Option<Vec<LedgerPath>>,
}

impl TryFrom<DeclarationFile> for Declaration {
    type Error = String;

    fn try_from(file: DeclarationFile) -> Result<Self, String> {
        // One key, because a run re-records one family: a declaration naming two would describe
        // two versions' changes in one review, and the ledger entry could only spell one of them.
        let carried: Vec<Version> = [
            file.analysis_version.map(Version::Analysis),
            file.screen_parser_version.map(Version::ScreenParser),
            file.career_version.map(Version::Career),
        ]
        .into_iter()
        .flatten()
        .collect();
        let version = match carried[..] {
            [one] => one,
            [] => {
                return Err(
                    "a declaration carries no version: `analysis_version` re-records the \
                     engine and stage families, `screen_parser_version` the screen family, and \
                     `career_version` the storage and career families (the M34 plan's call 6)"
                        .to_string(),
                )
            }
            _ => {
                let keys: Vec<&str> = carried.iter().map(|v| v.key()).collect();
                return Err(format!(
                    "a declaration carries more than one version ({}); a run re-records one \
                     family, so it names one (the M34 plan's call 6)",
                    keys.join(", ")
                ));
            }
        };
        // Present at all, even empty, on an engine or career declaration: an empty list there
        // would say a removal is something that re-record could declare, and it is not (M32's
        // rule, kept by the user's answer to the M34 plan's P7 finding 3, and given to the career
        // families at M36 P13 because nothing they hold needed one).
        if !version.may_remove() && file.removed.is_some() {
            return Err(format!(
                "a declaration at `{}` carries `removed`, and an engine or career re-record \
                 never removes a key; only a screen declaration names its removals (M34 P10)",
                version.key()
            ));
        }
        let removed = file.removed.unwrap_or_default();
        // A path in two lists says one place differs in two ways, which no one difference does;
        // and twice in one list is a copy-paste that the ledger would echo.
        let mut seen = BTreeSet::new();
        for path in file.added.iter().chain(&file.moved).chain(&removed) {
            if !seen.insert(path.as_str()) {
                return Err(format!("{path} is declared twice"));
            }
        }
        // The comparator reports an added or removed key at the key's own path, so such a path
        // ending in a list index can never match. A list that grew or shrank is a `moved` list.
        for (kind, paths) in [("added", &file.added), ("removed", &removed)] {
            if let Some(path) = paths
                .iter()
                .find(|path| matches!(path.segments.last(), Some(Segment::Index(_))))
            {
                return Err(format!(
                    "{path} is declared `{kind}`, but it ends in a list index and an {kind} key's \
                     path ends in the key; a list of another length is a `moved` list"
                ));
            }
        }
        Ok(Self {
            version,
            note: file.note,
            added: file.added,
            moved: file.moved,
            removed,
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
    removed: Vec<LedgerPath>,
}

impl Applied {
    pub fn added(&self) -> &[LedgerPath] {
        &self.added
    }

    pub fn moved(&self) -> &[LedgerPath] {
        &self.moved
    }

    /// Empty on every engine re-record, whose declaration has no `removed`. [M34 P10]
    pub fn removed(&self) -> &[LedgerPath] {
        &self.removed
    }

    /// Nothing differs — the second run of a re-record, and the verb writes nothing.
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.moved.is_empty() && self.removed.is_empty()
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
            DifferenceKind::RemovedKey => {
                "and the declaration's `removed` does not name it — an engine or career re-record \
                 never removes a key, and a screen one only where it is named"
            }
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
/// path, and a removed key only at a declared `removed` path — which an engine declaration never
/// has, so there a removed key never passes. Every refusal is returned, so one red run names them
/// all.
pub fn gate(
    committed: &Value,
    ours: &Value,
    declaration: &Declaration,
) -> Result<Applied, Vec<Undeclared>> {
    let mut added = BTreeSet::new();
    let mut moved = BTreeSet::new();
    let mut removed = BTreeSet::new();
    let mut undeclared = Vec::new();
    for difference in compare(committed, ours) {
        let (declared, matched) = match difference.kind {
            DifferenceKind::AddedKey => (&declaration.added, &mut added),
            DifferenceKind::Moved { .. } => (&declaration.moved, &mut moved),
            DifferenceKind::RemovedKey => (&declaration.removed, &mut removed),
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
        removed: pick(&declaration.removed, &removed),
    })
}

/// Step 4: the committed document with only `applied`'s paths taken from `ours`, and its removed
/// keys taken out.
///
/// Every other value is the committed one, untouched — in-tolerance floats included, which is the
/// module doc's reason this starts from `committed` rather than from `ours`.
///
/// # Panics
///
/// If `applied` is not [`gate`]'s answer for this same pair. Every path it holds is one [`compare`]
/// found as a value in both documents (a move), as a key under a parent both share (an addition),
/// or as a key `committed` has under such a parent (a removal), so for the pair it came from every
/// lookup below succeeds.
pub fn apply(committed: &Value, ours: &Value, applied: &Applied) -> Value {
    try_apply(committed, ours, applied)
        .unwrap_or_else(|(path, why)| panic!("{path}: {why}, so `applied` is not gate's answer"))
}

/// [`apply`], answering the path it could not walk instead of panicking.
///
/// The one way [`gate`]'s answer can fail to walk is a key holding `.`, `[` or `]`: the gate matches
/// a declared path to a difference as a string, and `a.json` under `files` and `json` under `a` are
/// the same string. The run reaches such keys since M36 P13 (a store's written files), so [`one`]
/// asks this rather than [`apply`] and refuses the path by name.
fn try_apply(
    committed: &Value,
    ours: &Value,
    applied: &Applied,
) -> Result<Value, (LedgerPath, &'static str)> {
    let mut document = committed.clone();
    for path in applied.added.iter().chain(&applied.moved) {
        let value = path
            .get(ours)
            .ok_or((path.clone(), "not in `ours`"))?
            .clone();
        path.put(&mut document, value)
            .ok_or((path.clone(), "no parent in `committed`"))?;
    }
    for path in &applied.removed {
        path.remove(&mut document)
            .ok_or((path.clone(), "not in `committed`"))?;
    }
    Ok(document)
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
    // Under the declaration's own version key, so an engine vector's entry says `analysis_version`
    // and a screen vector's `screen_parser_version`, each the version that vector ages on.
    let mut entry = Map::new();
    entry.insert(
        declaration.version.key().to_string(),
        json!(declaration.version.number()),
    );
    entry.insert("by".to_string(), json!(RERECORDED_BY));
    entry.insert("declaration".to_string(), json!(declaration_path));
    entry.insert("added".to_string(), spelled(&applied.added));
    entry.insert("moved".to_string(), spelled(&applied.moved));
    // On every screen entry, empty or not, so the family's entries share one shape; on no engine
    // entry, whose shape frozen Python's `frozen_view` reads and which M32 fixed, and on no career
    // entry, whose declaration cannot remove. [M34 P10, M36 P13]
    if declaration.version.may_remove() {
        entry.insert("removed".to_string(), spelled(&applied.removed));
    }
    let entry = Value::Object(entry);
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

/// The shot family's directory under `spec/vectors/`, and the `provenance.kind` its files carry.
/// Flat, one plain `.json` per case (the M35 plan's P4 finding 4), and re-recorded with the engine
/// family under `analysis_version`. [M35 P6]
pub const SHOT: &str = "shot";

/// The selection family's directory under `spec/vectors/`, and the `provenance.kind` its files
/// carry. Re-recorded with the engine family under `analysis_version`, after the shot family
/// (the M29 plan's call 1). [M29 P19]
pub const SELECTION: &str = selection_family::KIND;

/// The selection sub-families an engine declaration re-records, in this order, each answered by
/// [`run_selection`]: `rules/` (the rules over the tests' clips), `windows/` (the pipeline's picks
/// over its tests' bundles) and `corpus/` (the stored bundles), all recorded once from frozen
/// Python (M29 P18). Every entry under `vectors/selection/` must be one of these. [M29 P19]
pub const SELECTION_DOCUMENTS: [&str; 3] = ["rules", "windows", "corpus"];

/// The screen family's directory under `spec/vectors/`, and the `provenance.kind` every one of its
/// files carries, whichever sub-family it sits in. [M34 P7]
pub const SCREEN: &str = "screen";

/// The screen sub-families a screen declaration re-records, in this order: each file one screen's
/// `input` and its `expected`, which [`run_screen`] answers whole.
///
/// `hand/` (M34 P8) is a document like the others, whose oracle is a person rather than frozen
/// Python. A later parser change re-records it under a declaration as it does the rest, and the
/// ledger says which of its values are then Rust's, as it does on an engine vector: the re-record
/// keeps `provenance.oracle` as it found it.
pub const SCREEN_DOCUMENTS: [&str; 4] = ["corpus", "reference", "synthetic", "hand"];

/// The screen sub-families the re-record never reads.
///
/// `units/` is tables of cases for the parser's private functions, not documents, so [`run_screen`]
/// has nothing to answer them with. Nothing a re-record could declare moves them either: each
/// `field_for` case carries the profile it ran against, so a tile the shipping profile gains cannot
/// reach one, and the functions themselves are gated case by case in `crates/screen/tests/units.rs`.
/// A change that needs one re-recorded is a change that gives this module a runner for it.
///
/// Every other directory under `vectors/screen/` is refused rather than skipped, so a sub-family
/// added later is placed in one list or the other on purpose, never left out by default.
pub const SCREEN_UNREAD: [&str; 1] = ["units"];

/// The storage family's directory under `spec/vectors/`, and the `provenance.kind` its files carry.
/// [M36 P13]
pub const STORAGE: &str = "storage";

/// The career family's directory under `spec/vectors/`, and the `provenance.kind` its files carry.
/// [M36 P13]
pub const CAREER: &str = "career";

/// The storage sub-families a career declaration re-records, in this order, each answered by
/// [`run_storage`]. `corpus/` holds the corpus cases and `bundle/` and `stores/` the op cases, all
/// recorded from frozen Python; `hand/` holds the corpus cases worked by hand at `career_version` 1,
/// where `COMPARABLE_FROM` and the installed engine part (M36 P14). Which runner answers a vector is
/// the runner its provenance names (`StorageCase::of`), not its directory. Every directory under
/// `vectors/storage/` must be one of these, as under `vectors/screen/`, so a sub-family added later
/// is placed here on purpose. [M36 P13]
pub const STORAGE_DOCUMENTS: [&str; 4] = ["corpus", "bundle", "stores", "hand"];

/// The career sub-families a career declaration re-records, in this order, each answered by
/// [`run_career`] after the storage family, whose corpus cases some of them adopt. [M36 P13]
pub const CAREER_DOCUMENTS: [&str; 2] = ["synthetic", "real"];

/// The profile family's directory under `spec/vectors/`, and the `provenance.kind` its files carry.
/// [M37 P11]
pub const PROFILE: &str = profile_family::KIND;

/// The profile sub-families a career declaration re-records, in this order, each answered by
/// [`run_profile_case`] after the career family, whose inputs the corpus cases adopt. `hand/` (M37
/// P5) is worked by hand and keeps `provenance.oracle: "hand"`; `corpus/` (P11) was recorded by Rust
/// and keeps `"rust"`, because [`ledger`] writes `"python"` only where no oracle is named. [M37 P11]
pub const PROFILE_DOCUMENTS: [&str; 2] = ["hand", "corpus"];

/// The MCP server's family under `spec/vectors/`, and the `provenance.kind` its files carry.
/// [M29 P15]
pub const MCP: &str = "mcp";

/// The mcp sub-families a career declaration re-records, after the profile family, each answered
/// by `golf-mcp answer-case` ([`walk_mcp`]'s doc): `tools/` and `real/` recorded once from frozen
/// Python (M29 P8), `hand/` worked by hand (P11 on) and `rust/` recorded once by Rust (P15), each
/// keeping the `provenance.oracle` it has. [M29 P15]
pub const MCP_DOCUMENTS: [&str; 4] = ["tools", "real", "hand", "rust"];

/// The one mcp sub-family the walk never reads: `surface/`, what a client sees on connect. Nothing
/// `CAREER_VERSION` ages reaches it (the tool list, the descriptions and the briefing read no
/// store, corpus or aggregate), and its five declared differences are `crates/mcp/tests/surface.rs`'
/// to hold, so it keeps `career_version` 0 and no ledger. [M29 P15]
pub const MCP_UNREAD: [&str; 1] = ["surface"];

/// The environment variable that names the `golf-mcp` binary the mcp walk runs, ahead of the one
/// beside this binary ([`golf_mcp_binary`]). [M29 P15]
pub const GOLF_MCP_ENV: &str = "GOLF_MCP";

/// Which of Rust's answers a vector is re-recorded from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Family {
    /// `synthetic/` and `corpus/`: `expected` is [`crate::run`]'s answer.
    Engine,
    /// `stages/`: `stages` is [`run_stages`]' answer, composed onto its engine vector.
    Stage,
    /// [`SHOT`]: `expected` is [`run_shot`]'s answer. [M35 P6]
    Shot,
    /// [`SELECTION`]'s [`SELECTION_DOCUMENTS`]: `expected.results` is [`run_selection`]'s answer
    /// to each call, and the committed one to each call it does not answer yet. [M29 P19]
    Selection,
    /// `screen/`'s [`SCREEN_DOCUMENTS`]: `expected` is [`run_screen`]'s answer. [M34 P7]
    Screen,
    /// `storage/`'s [`STORAGE_DOCUMENTS`]: `expected` is [`run_storage`]'s answer. [M36 P13]
    Storage,
    /// `career/`'s [`CAREER_DOCUMENTS`]: `expected` is [`run_career`]'s answer and the
    /// [`CARRIED`](crate::career_family::CARRIED) reports as committed, and an adopted
    /// `input.corpus` is its storage vector's answer as the run writes it. [M36 P13]
    Career,
    /// `profile/`'s [`PROFILE_DOCUMENTS`]: `expected` is [`run_profile_case`]'s answer, and a corpus
    /// case's [`ADOPTED`] inputs are its career vector's as the run writes it. [M37 P11]
    Profile,
    /// `mcp/`'s [`MCP_DOCUMENTS`]: `expected.results` is `golf-mcp answer-case`'s answer to each
    /// call, gated against the committed answers as that server answers them on purpose. [M29 P15]
    Mcp,
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
    /// The committed file's line ending and indent, kept (module doc, "How a file is written").
    layout: Layout,
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
/// vector the declaration's family walks passed — both families for an engine declaration, the
/// screen documents for a screen one, the storage, career and profile families for a career one.
#[derive(Debug)]
pub struct Run {
    declaration: Declaration,
    vectors: Vec<Rerecorded>,
    /// The `golf-mcp` the mcp walk ran — an engine or a career run's — named in the report because
    /// it is a second binary whose build the reviewer must trust to be this tree's. [M29 P15; the
    /// engine run's P19]
    mcp_answered_by: Option<PathBuf>,
}

impl Run {
    /// Every vector the run read. For an engine declaration, the engine family, then the stage
    /// family, within each the [`ENGINE_HALVES`] in their order and each half in path order, then
    /// the shot family in path order, then the [`SELECTION_DOCUMENTS`] and then the
    /// [`MCP_DOCUMENTS`], each in their order and path order (M29 P19). For a screen declaration, the
    /// [`SCREEN_DOCUMENTS`] in their order, each in path order. For a career declaration, the
    /// [`STORAGE_DOCUMENTS`], the [`CAREER_DOCUMENTS`], the [`PROFILE_DOCUMENTS`] and then the
    /// [`MCP_DOCUMENTS`], each in path order.
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
                (v.path.as_path(), file_bytes(document, &v.path, v.layout))
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
        let declared = match d.version {
            Version::Analysis(_) | Version::Career(_) => {
                format!("{} added and {} moved", d.added.len(), d.moved.len())
            }
            Version::ScreenParser(_) => format!(
                "{} added, {} moved and {} removed",
                d.added.len(),
                d.moved.len(),
                d.removed.len()
            ),
        };
        let mut out = format!(
            "golf-core rerecord: a declaration at {}, {declared} — {}\n",
            d.version, d.note
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
        let count = |family: Family| self.vectors.iter().filter(|v| v.family == family).count();
        let run = match d.version {
            Version::Analysis(_) => format!(
                "{} engine, {} stage, {} shot, {} selection, {} mcp",
                count(Family::Engine),
                count(Family::Stage),
                count(Family::Shot),
                count(Family::Selection),
                count(Family::Mcp)
            ),
            Version::ScreenParser(_) => format!("{} screen", count(Family::Screen)),
            Version::Career(_) => format!(
                "{} storage, {} career, {} profile, {} mcp",
                count(Family::Storage),
                count(Family::Career),
                count(Family::Profile),
                count(Family::Mcp)
            ),
        };
        if let Some(binary) = &self.mcp_answered_by {
            writeln!(out, "the mcp family was answered by {}", binary.display())
                .expect("writing to a String does not fail");
        }
        let changed = self.changed().count();
        if changed == 0 {
            out.push_str("nothing differs from the committed vectors\n");
        }
        writeln!(
            out,
            "{} vectors run ({run}); {changed} changed; {files_written} files written",
            self.vectors.len(),
        )
        .expect("writing to a String does not fail");
        out
    }
}

/// Run, gate, apply and ledger every vector of the declaration's families under `spec` — composing
/// each stage vector onto its engine vector on the way — and write nothing.
///
/// `Ok` only if every vector the family walks passed and every declared path matched (calls 5–7);
/// [`Run::write`] is then the only thing left to do, and `--dry-run` is not calling it.
pub fn plan(spec: &Path, declaration_file: &Path) -> Result<Run, Refused> {
    let declaration = load_declaration(declaration_file)?;
    let version = declaration.version;
    if version.number() != version.current() {
        return Err(Refused::Declaration(format!(
            "{} is at {version}, and this {} is at {}; a declaration describes one version's \
             change, so it is re-run against no other (M32's plan, call 6)",
            declaration_file.display(),
            version.of(),
            version.current()
        )));
    }
    let spelling = ledger_spelling(spec, declaration_file)?;
    let vectors_dir = spec.join("vectors");

    let mut mcp_answered_by = None;
    let Walked {
        vectors,
        refusals,
        ledgered,
    } = match version {
        Version::Analysis(_) => {
            // The mcp family is the last an engine run walks too (M29 P19, `walk_mcp`'s doc), so
            // its binary is found before any vector is read, as a career run finds it.
            let binary = golf_mcp_binary().map_err(Refused::Files)?;
            let mut walked = walk_engine(&vectors_dir, &declaration, spelling.as_deref())?;
            walked.extend(walk_shot(&vectors_dir, &declaration, spelling.as_deref())?);
            walked.extend(walk_selection(
                &vectors_dir,
                &declaration,
                spelling.as_deref(),
            )?);
            walked.extend(walk_mcp(
                &vectors_dir,
                &declaration,
                spelling.as_deref(),
                &binary,
            )?);
            mcp_answered_by = Some(binary);
            walked
        }
        Version::ScreenParser(_) => walk_screen(&vectors_dir, &declaration, spelling.as_deref())?,
        Version::Career(_) => {
            // Found before any vector is read, so a run that cannot reach the last family stops
            // before it has walked the first three.
            let binary = golf_mcp_binary().map_err(Refused::Files)?;
            let mut walked = walk_career(&vectors_dir, &declaration, spelling.as_deref())?;
            walked.extend(walk_mcp(
                &vectors_dir,
                &declaration,
                spelling.as_deref(),
                &binary,
            )?);
            mcp_answered_by = Some(binary);
            walked
        }
    };

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
        mcp_answered_by,
    })
}

/// One family's walk: every vector it read, every refusal across them, and the ledger entries
/// already committed at the declaration's version. [`plan`] judges the three the same way whichever
/// family made them.
struct Walked {
    vectors: Vec<Rerecorded>,
    refusals: Vec<String>,
    ledgered: Ledgered,
}

impl Walked {
    /// Another family's walk after this one's, for a declaration whose key re-records several: the
    /// engine and stage families, then the shot family (M35 P6), the selection family and the mcp
    /// family (M29 P19); the storage, career and profile families, then the mcp family (M29 P15).
    /// The typo guard reads the union, so a declared path may match in any of them.
    fn extend(&mut self, later: Walked) {
        self.vectors.extend(later.vectors);
        self.refusals.extend(later.refusals);
        self.ledgered.extend(later.ledgered);
    }
}

/// The engine family and then the stage family, each stage document composed onto its engine
/// vector as that vector will be written (call 9). M32's walk, unchanged but for being one of two.
fn walk_engine(
    vectors_dir: &Path,
    declaration: &Declaration,
    spelling: Option<&str>,
) -> Result<Walked, Refused> {
    let mut vectors: Vec<Rerecorded> = Vec::new();
    let mut refusals: Vec<String> = Vec::new();
    let mut ledgered = Ledgered::default();
    // Every engine document as it will be written (as committed, if the gate refused it), for the
    // stage family's compose check — and the index of its `Rerecorded`, if it has one.
    let mut engines: BTreeMap<String, (Option<usize>, Value)> = BTreeMap::new();

    for half in ENGINE_HALVES {
        for path in vector_files(&vectors_dir.join(half))? {
            let (committed, layout) = read_vector(&path)?;
            let id = identify(&committed, &path, half)?;
            ledgered.note(&committed, declaration);
            let input = parse_input(&committed, &id)?;

            let mut ours = committed.clone();
            ours["expected"] = crate::run(&input);
            ours["analysis_version"] = json!(ANALYSIS_VERSION);

            let entry = match one(&id, &committed, &ours, declaration, spelling) {
                Ok((applied, written, lines)) => {
                    vectors.push(Rerecorded {
                        id: id.clone(),
                        path,
                        family: Family::Engine,
                        applied,
                        lines,
                        layout,
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
            let (committed, layout) = read_vector(&path)?;
            let id = identify(&committed, &path, STAGES)?;
            ledgered.note(&committed, declaration);
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

            match one(&id, &committed, &ours, declaration, spelling) {
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
                        layout,
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
    Ok(Walked {
        vectors,
        refusals,
        ledgered,
    })
}

/// The shot family, every case run through [`run_shot`], gated, applied and ledgered. [M35 P6]
///
/// After the engine and stage families, under the same declaration, because the cases' numbers
/// are the engine's (the M35 plan's call 7): an `ANALYSIS_VERSION` bump moves `analysis_version`
/// and `expected.analysis_version` on a case as it does on an engine vector, and the one
/// declaration covers both, since a declared path is matched by its spelling wherever it falls.
/// A shot row has its own spelling, `expected.measurements[i].value` where the swing's is
/// `expected.swing.measurements[j].value`, so a declaration that moves a corpus vector's shot row
/// names the case's too, or the gate refuses the case at the path it left out. That is what keeps
/// the two copies of one shot's rows in step without a compose check here.
///
/// **Inputs are never rewritten**, as `v17.json` says of the engine's: only `expected` and the
/// top-level version are Rust's answer. A case `run_shot` refuses (an undeclared device, a profile
/// that does not exist) is a broken `spec/`, since every committed case is an answer, and stops
/// the walk. **The family is flat**: every entry under `vectors/shot/` must be a plain `.json`
/// file, checked before any case is read, because `tests/shot.rs`' `cases()` reads every entry as
/// a case, and a sub-family needs it and this walk extended together (the M35 plan's P4 finding
/// 4), never one of the two.
fn walk_shot(
    vectors_dir: &Path,
    declaration: &Declaration,
    spelling: Option<&str>,
) -> Result<Walked, Refused> {
    let shot_dir = vectors_dir.join(SHOT);
    let listing =
        |e: &dyn fmt::Display| Refused::Files(format!("listing {}: {e}", shot_dir.display()));
    for entry in fs::read_dir(&shot_dir).map_err(|e| listing(&e))? {
        let path = entry.map_err(|e| listing(&e))?.path();
        let name = path.file_name().and_then(OsStr::to_str).unwrap_or_default();
        if !(path.is_file() && name.ends_with(".json")) {
            return Err(Refused::Files(format!(
                "{} is not a shot case. The shot family is flat, one plain `.json` per case, so \
                 a sub-family or another kind of file needs this walk and `tests/shot.rs`' \
                 `cases()` extended together (golf_core::rerecord::walk_shot)",
                path.display()
            )));
        }
    }

    let mut vectors: Vec<Rerecorded> = Vec::new();
    let mut refusals: Vec<String> = Vec::new();
    let mut ledgered = Ledgered::default();
    let mut ids: BTreeSet<String> = BTreeSet::new();
    for path in vector_files(&shot_dir)? {
        let (committed, layout) = read_vector(&path)?;
        let id = identify(&committed, &path, SHOT)?;
        if !ids.insert(id.clone()) {
            return Err(Refused::Files(format!("{id} is the id of two shot cases")));
        }
        ledgered.note(&committed, declaration);
        // The trait method, so `ShotInput`'s `deny_unknown_fields` and every contract type's
        // `Validate` hold here as they do in `run-shot`.
        let input = ShotInput::deserialize(&committed["input"]).map_err(|e| {
            Refused::Files(format!(
                "{id}: input does not parse into `golf_core::ShotInput`: {e}"
            ))
        })?;
        let answer = run_shot(&input)
            .map_err(|e| Refused::Files(format!("{id}: run_shot refused the case: {e}")))?;

        let mut ours = committed.clone();
        ours["expected"] = serde_json::to_value(answer).expect("a ShotResult serializes");
        ours["analysis_version"] = json!(ANALYSIS_VERSION);

        match one(&id, &committed, &ours, declaration, spelling) {
            Ok((applied, written, lines)) => {
                vectors.push(Rerecorded {
                    id,
                    path,
                    family: Family::Shot,
                    written: (!applied.is_empty()).then_some(written),
                    applied,
                    lines,
                    layout,
                });
            }
            Err(refused) => refusals.extend(refused),
        }
    }
    if ids.is_empty() {
        return Err(Refused::Files(format!(
            "no shot case under {}",
            shot_dir.display()
        )));
    }
    Ok(Walked {
        vectors,
        refusals,
        ledgered,
    })
}

/// The selection family's [`SELECTION_DOCUMENTS`], every case answered by [`run_selection`], gated,
/// applied and ledgered, after the shot family under an engine declaration. [M29 P19]
///
/// The family ages on `ANALYSIS_VERSION` (the M29 plan's call 1): its answers are the engine's
/// clip-choosing rules, so a bump that moves which descent a clip is windowed on moves a case in the
/// same run, or the gate refuses it here. Only `expected.results` and the top-level
/// `analysis_version` are this build's answer — a case has no `expected.analysis_version`, which
/// `v17.json` declares and the engine vectors' ledgers already hold — and inputs are never
/// rewritten: each is clips and calls no other vector answers, so nothing is adopted and nothing
/// composes.
///
/// **A call [`run_selection`] does not answer is carried as committed**
/// (`selection_family::NOT_YET`), as the career walk carried a report no renderer ran. The carry is
/// sound only while nothing it is built from moves, and an `_auto_windows` answer is built from the
/// `_pick_swing` calls listed ahead of it. So **a case carrying an answer is refused if any answered
/// call in it differs**, declared or not, naming the phase that ports the carried call: a
/// declaration cannot move a pick and leave the windows chosen from it as they were. **Since M29
/// P25 the table is empty and nothing is carried**: `_auto_windows` is answered by
/// `golf_core::auto_windows`, so a pick that moves moves the answer built from it, and the gate
/// refuses the move it was not told of (`tests/rerecord.rs`'
/// `a_moved_pick_moves_the_windows_chosen_from_it`). The guard stays with the table, as
/// `career_family::CARRIED`'s carry stayed once it was empty.
fn walk_selection(
    vectors_dir: &Path,
    declaration: &Declaration,
    spelling: Option<&str>,
) -> Result<Walked, Refused> {
    let selection_dir = vectors_dir.join(SELECTION);
    every_entry_is_placed(
        &selection_dir,
        SELECTION,
        (&SELECTION_DOCUMENTS, "SELECTION_DOCUMENTS"),
        None,
    )?;

    let mut vectors: Vec<Rerecorded> = Vec::new();
    let mut refusals: Vec<String> = Vec::new();
    let mut ledgered = Ledgered::default();
    let mut ids: BTreeSet<String> = BTreeSet::new();
    for half in SELECTION_DOCUMENTS {
        for path in vector_files(&selection_dir.join(half))? {
            let (committed, layout) = read_vector(&path)?;
            let id = identify(&committed, &path, SELECTION)?;
            if !ids.insert(id.clone()) {
                return Err(Refused::Files(format!(
                    "{id} is the id of two selection cases"
                )));
            }
            ledgered.note(&committed, declaration);

            let answers = run_selection(&committed["input"]).map_err(|e| {
                Refused::Files(format!("{id}: run_selection refused the case: {e}"))
            })?;
            let recorded = committed["expected"]["results"]
                .as_array()
                .ok_or_else(|| Refused::Files(format!("{id}: no expected.results list")))?;
            if recorded.len() != answers.len() {
                return Err(Refused::Files(format!(
                    "{id}: {} calls and {} recorded results",
                    answers.len(),
                    recorded.len()
                )));
            }
            let mut carried: Vec<usize> = Vec::new();
            let mut moved: Vec<usize> = Vec::new();
            let results: Vec<Value> = answers
                .into_iter()
                .zip(recorded)
                .enumerate()
                .map(|(index, (answer, recorded))| match answer {
                    Some(answer) => {
                        if !compare(recorded, &answer).is_empty() {
                            moved.push(index);
                        }
                        answer
                    }
                    None => {
                        carried.push(index);
                        recorded.clone()
                    }
                })
                .collect();
            if !carried.is_empty() && !moved.is_empty() {
                let calls = &committed["input"]["calls"];
                let named = |indices: &[usize]| {
                    indices
                        .iter()
                        .map(|&i| format!("{i} ({})", calls[i]["fn"].as_str().unwrap_or("?")))
                        .collect::<Vec<_>>()
                        .join(", ")
                };
                let phases: BTreeSet<&str> = carried
                    .iter()
                    .filter_map(|&i| calls[i]["fn"].as_str())
                    .filter_map(selection_family::not_yet)
                    .collect();
                refusals.push(format!(
                    "{id}: call(s) {} moved, and call(s) {} are carried as committed until M29 {} \
                     ports them, so they cannot move with the calls they are built from \
                     (golf_core::rerecord::walk_selection)",
                    named(&moved),
                    named(&carried),
                    phases.into_iter().collect::<Vec<_>>().join(", ")
                ));
                continue;
            }

            let mut ours = committed.clone();
            ours["expected"]["results"] = Value::Array(results);
            ours["analysis_version"] = json!(ANALYSIS_VERSION);

            match one(&id, &committed, &ours, declaration, spelling) {
                Ok((applied, written, lines)) => {
                    vectors.push(Rerecorded {
                        id,
                        path,
                        family: Family::Selection,
                        written: (!applied.is_empty()).then_some(written),
                        applied,
                        lines,
                        layout,
                    });
                }
                Err(refused) => refusals.extend(refused),
            }
        }
    }
    if ids.is_empty() {
        return Err(Refused::Files(format!(
            "no selection case under {}",
            selection_dir.display()
        )));
    }
    Ok(Walked {
        vectors,
        refusals,
        ledgered,
    })
}

/// The screen family's [`SCREEN_DOCUMENTS`], each run through [`run_screen`], gated, applied and
/// ledgered. [M34 P7]
///
/// Nothing composes onto a screen document, because nothing else holds a copy of its answer. Every
/// entry under `vectors/screen/` must be a directory named in [`SCREEN_DOCUMENTS`] or
/// [`SCREEN_UNREAD`], checked before any vector is read, so a sub-family nobody placed stops the walk
/// instead of going un-re-recorded while its tests go on reading the old answers.
fn walk_screen(
    vectors_dir: &Path,
    declaration: &Declaration,
    spelling: Option<&str>,
) -> Result<Walked, Refused> {
    let screen_dir = vectors_dir.join(SCREEN);
    every_entry_is_placed(
        &screen_dir,
        SCREEN,
        (&SCREEN_DOCUMENTS, "SCREEN_DOCUMENTS"),
        Some((&SCREEN_UNREAD, "SCREEN_UNREAD")),
    )?;

    let mut vectors: Vec<Rerecorded> = Vec::new();
    let mut refusals: Vec<String> = Vec::new();
    let mut ledgered = Ledgered::default();
    let mut ids: BTreeSet<String> = BTreeSet::new();
    for half in SCREEN_DOCUMENTS {
        for path in vector_files(&screen_dir.join(half))? {
            let (committed, layout) = read_vector(&path)?;
            let id = identify(&committed, &path, SCREEN)?;
            if !ids.insert(id.clone()) {
                return Err(Refused::Files(format!(
                    "{id} is the id of two screen vectors"
                )));
            }
            ledgered.note(&committed, declaration);
            // The trait method, so `ScreenInput`'s `deny_unknown_fields` holds here as it does in
            // `parse-screen`: a vector whose input the reader would not take is a broken `spec/`.
            let input = ScreenInput::deserialize(&committed["input"]).map_err(|e| {
                Refused::Files(format!(
                    "{id}: input does not parse into `screen::ScreenInput`: {e}"
                ))
            })?;

            let mut ours = committed.clone();
            ours["expected"] =
                run_screen(&input).map_err(|e| Refused::Files(format!("{id}: {e}")))?;
            ours["screen_parser_version"] = json!(SCREEN_PARSER_VERSION);

            match one(&id, &committed, &ours, declaration, spelling) {
                Ok((applied, written, lines)) => {
                    let written = (!applied.is_empty()).then_some(written);
                    vectors.push(Rerecorded {
                        id,
                        path,
                        family: Family::Screen,
                        applied,
                        lines,
                        layout,
                        written,
                    });
                }
                Err(refused) => refusals.extend(refused),
            }
        }
    }
    if ids.is_empty() {
        return Err(Refused::Files(format!(
            "no screen document under {}",
            screen_dir.display()
        )));
    }
    Ok(Walked {
        vectors,
        refusals,
        ledgered,
    })
}

/// Refuse any entry of `dir` that is not a directory named in `documents` or `unread`, before any
/// vector under it is read, so a sub-family nobody placed stops the walk instead of going
/// un-re-recorded while its tests go on reading the old answers. [M34 P7, shared at M36 P13]
fn every_entry_is_placed(
    dir: &Path,
    family: &str,
    (documents, documents_name): (&[&str], &str),
    unread: Option<(&[&str], &str)>,
) -> Result<(), Refused> {
    let listing = |e: &dyn fmt::Display| Refused::Files(format!("listing {}: {e}", dir.display()));
    for entry in fs::read_dir(dir).map_err(|e| listing(&e))? {
        let path = entry.map_err(|e| listing(&e))?.path();
        let name = path.file_name().and_then(OsStr::to_str).unwrap_or_default();
        let placed =
            documents.contains(&name) || unread.is_some_and(|(unread, _)| unread.contains(&name));
        if !(placed && path.is_dir()) {
            let or_unread = unread
                .map(|(_, unread_name)| format!(", and one it must never read in `{unread_name}`"))
                .unwrap_or_default();
            return Err(Refused::Files(format!(
                "{} is not a {family} sub-family the re-record knows. A directory of documents \
                 belongs in `{documents_name}`{or_unread} (golf_core::rerecord), so that a new \
                 one is placed on purpose",
                path.display()
            )));
        }
    }
    Ok(())
}

/// The storage family's [`STORAGE_DOCUMENTS`], the career family's [`CAREER_DOCUMENTS`] and then
/// the profile family's [`PROFILE_DOCUMENTS`], each run, gated, applied and ledgered. [M36 P13, the
/// profile family M37 P11]
///
/// - **Each storage vector** is answered by [`run_storage`], the gate's own runner, so the op cases'
///   written files are already in the committed spelling wherever their values agree
///   (`storage_family`'s module doc).
/// - **Each career vector** is answered by [`run_career`], and its
///   [`CARRIED`](crate::career_family::CARRIED) reports are copied from the committed document, so
///   a report no renderer runs neither moves nor goes missing. None is carried since M36 P16
///   rendered the last two scripts.
/// - **An adopted corpus is derived, not read** (`career_family`'s module doc): a career vector whose
///   corpus is a storage vector's answer is run on that answer *as this run will write it*, and the
///   copy in its `input` is compared with it like any other value. A source whose answer moved, or a
///   copy that drifted from its source, is a difference at `input.corpus…` that the declaration must
///   name, in the same run. It is the one input a re-record can change, because it is the one input
///   that is another vector's answer; the engine's stage vectors read their engine vector's input,
///   which no re-record moves.
///
/// - **Each profile case** is answered by [`run_profile_case`], the profile gate's own runner. A
///   corpus case's [`ADOPTED`] inputs are its career vector's *as this run will write it*, one link
///   further along the same rule: a career input that moved moves the profile case's copy in the
///   same run, and the declaration names both (M37 P11).
///
/// No family has an unread sub-family. A hand-worked case (`storage/hand/`, M36 P14;
/// `profile/hand/`, M37 P5) is run, gated and ledgered like a recorded one, and keeps its
/// `provenance.oracle` as the ledger finds it, as a profile corpus case keeps `"rust"`.
fn walk_career(
    vectors_dir: &Path,
    declaration: &Declaration,
    spelling: Option<&str>,
) -> Result<Walked, Refused> {
    let storage_dir = vectors_dir.join(STORAGE);
    let career_dir = vectors_dir.join(CAREER);
    let profile_dir = vectors_dir.join(PROFILE);
    every_entry_is_placed(
        &storage_dir,
        STORAGE,
        (&STORAGE_DOCUMENTS, "STORAGE_DOCUMENTS"),
        None,
    )?;
    every_entry_is_placed(
        &career_dir,
        CAREER,
        (&CAREER_DOCUMENTS, "CAREER_DOCUMENTS"),
        None,
    )?;
    every_entry_is_placed(
        &profile_dir,
        PROFILE,
        (&PROFILE_DOCUMENTS, "PROFILE_DOCUMENTS"),
        None,
    )?;

    let mut vectors: Vec<Rerecorded> = Vec::new();
    let mut refusals: Vec<String> = Vec::new();
    let mut ledgered = Ledgered::default();
    // Every storage document as it will be written (as committed, if the gate refused it), for the
    // career vectors that adopt one's corpus.
    let mut storage: BTreeMap<String, Value> = BTreeMap::new();

    for half in STORAGE_DOCUMENTS {
        for path in vector_files(&storage_dir.join(half))? {
            let (committed, layout) = read_vector(&path)?;
            let id = identify(&committed, &path, STORAGE)?;
            ledgered.note(&committed, declaration);

            let mut ours = committed.clone();
            ours["expected"] = run_storage(&committed).map_err(Refused::Files)?;
            ours["career_version"] = json!(CAREER_VERSION);

            let as_written = match one(&id, &committed, &ours, declaration, spelling) {
                Ok((applied, written, lines)) => {
                    vectors.push(Rerecorded {
                        id: id.clone(),
                        path,
                        family: Family::Storage,
                        written: (!applied.is_empty()).then(|| written.clone()),
                        applied,
                        lines,
                        layout,
                    });
                    written
                }
                Err(refused) => {
                    refusals.extend(refused);
                    committed
                }
            };
            if storage.insert(id.clone(), as_written).is_some() {
                return Err(Refused::Files(format!(
                    "{id} is the id of two storage vectors"
                )));
            }
        }
    }
    if storage.is_empty() {
        return Err(Refused::Files(format!(
            "no storage vector under {}",
            storage_dir.display()
        )));
    }

    // Every career document as it will be written (as committed, if the gate refused it), for the
    // profile cases that adopt one's input. [M37 P11]
    let mut career: BTreeMap<String, Value> = BTreeMap::new();
    for half in CAREER_DOCUMENTS {
        for path in vector_files(&career_dir.join(half))? {
            let (committed, layout) = read_vector(&path)?;
            let id = identify(&committed, &path, CAREER)?;
            if career.contains_key(&id) {
                return Err(Refused::Files(format!(
                    "{id} is the id of two career vectors"
                )));
            }
            ledgered.note(&committed, declaration);

            let mut ours = committed.clone();
            if let Some(source) = adopted_from(&committed).map_err(Refused::Files)? {
                let corpus = storage
                    .get(&source)
                    .and_then(|document| document["expected"].get("corpus"))
                    .ok_or_else(|| {
                        Refused::Files(format!(
                            "{id} adopts the corpus of {source}, which is not a storage corpus \
                             case here"
                        ))
                    })?;
                ours["input"]["corpus"] = corpus.clone();
            }
            let mut expected = run_career(&ours).map_err(Refused::Files)?;
            carry(&mut expected, &committed["expected"]);
            ours["expected"] = expected;
            ours["career_version"] = json!(CAREER_VERSION);

            let as_written = match one(&id, &committed, &ours, declaration, spelling) {
                Ok((applied, written, lines)) => {
                    vectors.push(Rerecorded {
                        id: id.clone(),
                        path,
                        family: Family::Career,
                        written: (!applied.is_empty()).then(|| written.clone()),
                        applied,
                        lines,
                        layout,
                    });
                    written
                }
                Err(refused) => {
                    refusals.extend(refused);
                    committed
                }
            };
            career.insert(id, as_written);
        }
    }
    if career.is_empty() {
        return Err(Refused::Files(format!(
            "no career vector under {}",
            career_dir.display()
        )));
    }

    let mut profile: BTreeSet<String> = BTreeSet::new();
    for half in PROFILE_DOCUMENTS {
        for path in vector_files(&profile_dir.join(half))? {
            let (committed, layout) = read_vector(&path)?;
            let id = identify(&committed, &path, PROFILE)?;
            if !profile.insert(id.clone()) {
                return Err(Refused::Files(format!(
                    "{id} is the id of two profile cases"
                )));
            }
            ledgered.note(&committed, declaration);

            let mut ours = committed.clone();
            if let Some(source) =
                profile_family::adopted_from(&committed).map_err(Refused::Files)?
            {
                let twin = career.get(&source).ok_or_else(|| {
                    Refused::Files(format!(
                        "{id} adopts the input of {source}, which is not a career vector here"
                    ))
                })?;
                for key in ADOPTED {
                    ours["input"][key] = twin["input"][key].clone();
                }
            }
            ours["expected"] = run_profile_case(&ours).map_err(Refused::Files)?;
            ours["career_version"] = json!(CAREER_VERSION);

            match one(&id, &committed, &ours, declaration, spelling) {
                Ok((applied, written, lines)) => {
                    vectors.push(Rerecorded {
                        id,
                        path,
                        family: Family::Profile,
                        written: (!applied.is_empty()).then_some(written),
                        applied,
                        lines,
                        layout,
                    });
                }
                Err(refused) => refusals.extend(refused),
            }
        }
    }
    if profile.is_empty() {
        return Err(Refused::Files(format!(
            "no profile case under {}",
            profile_dir.display()
        )));
    }
    Ok(Walked {
        vectors,
        refusals,
        ledgered,
    })
}

/// The `golf-mcp` binary the mcp walk runs: [`GOLF_MCP_ENV`] when it is set, else `golf-mcp`
/// beside this binary, else one directory up when this binary sits in cargo's `deps/` (a test
/// harness does). `Err` names every place tried and the build that puts it there. [M29 P15; an
/// engine run needs it too since P19]
///
/// **Found, never assumed** (the M29 plan's rules): nothing may depend on `golf-mcp`, so this crate
/// cannot link the family's runner and runs the binary instead (`golf_mcp::family`'s module doc),
/// and a missing one refuses the run by name rather than skipping a family that ages on this key.
/// The binary is a second build the run trusts, so [`Run::report`] names the one it ran.
pub fn golf_mcp_binary() -> Result<PathBuf, String> {
    if let Some(named) = std::env::var_os(GOLF_MCP_ENV) {
        let named = PathBuf::from(named);
        return if named.is_file() {
            Ok(named)
        } else {
            Err(format!(
                "{GOLF_MCP_ENV} names {}, which is not a file; the mcp family is answered by \
                 golf-mcp (`cargo build --release -p golf-mcp`)",
                named.display()
            ))
        };
    }
    let name = format!("golf-mcp{}", std::env::consts::EXE_SUFFIX);
    let mut tried = Vec::new();
    if let Ok(this) = std::env::current_exe() {
        let mut dir = this.parent().map(Path::to_path_buf);
        while let Some(here) = dir {
            let candidate = here.join(&name);
            if candidate.is_file() {
                return Ok(candidate);
            }
            tried.push(candidate.display().to_string());
            dir = if here.file_name() == Some(OsStr::new("deps")) {
                here.parent().map(Path::to_path_buf)
            } else {
                None
            };
        }
    }
    Err(format!(
        "an engine or a career declaration re-records the mcp family, which golf-mcp answers, and \
         no golf-mcp was found (tried {}); build it beside this binary with `cargo build -p \
         golf-mcp` (add `--release` for a release golf-core), or name one in {GOLF_MCP_ENV}",
        if tried.is_empty() {
            "nowhere: this binary's path is unknown".to_string()
        } else {
            tried.join(", ")
        }
    ))
}

/// One mcp case through `golf-mcp answer-case`: `(answers, recorded)`, the server's answer to each
/// call and the committed answers as it answers them on purpose (`golf_mcp::family::answer_case`).
fn answer_with(binary: &Path, id: &str, committed: &Value) -> Result<(Value, Value), Refused> {
    let failed = |why: String| Refused::Files(format!("{id}: golf-mcp answer-case {why}"));
    let mut child = Command::new(binary)
        .arg("answer-case")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| failed(format!("did not start ({}): {e}", binary.display())))?;
    let input = serde_json::to_vec(committed).expect("a Value always serializes");
    let mut stdin = child.stdin.take().expect("stdin was piped");
    // Written from a thread so a child that answers before it has read everything cannot block on
    // a full stdout while this side blocks on a full stdin. `answer-case` reads all of stdin first,
    // but the pipe's rule is the one to code against.
    let writer = std::thread::spawn(move || stdin.write_all(&input));
    let output = child
        .wait_with_output()
        .map_err(|e| failed(format!("did not finish: {e}")))?;
    writer
        .join()
        .expect("the writer thread does not panic")
        .map_err(|e| failed(format!("was not sent the case: {e}")))?;
    if !output.status.success() {
        return Err(failed(format!(
            "exited {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    let mut answered: Value = serde_json::from_slice(&output.stdout)
        .map_err(|e| failed(format!("printed something that is not JSON: {e}")))?;
    Ok((answered["answers"].take(), answered["recorded"].take()))
}

/// The document with every answer's text blocks parsed, so the gate compares a block as the mcp
/// gate does (`crates/mcp/tests/tools.rs`: each block is one view's JSON) and a declaration names a
/// path inside one: `expected.results[3].content[0].text.n_swings`. A block whose text is not JSON
/// stays a string. [M29 P15]
fn blocks_parsed(document: &Value) -> Value {
    let mut parsed = document.clone();
    for block in blocks_mut(&mut parsed) {
        let text = block
            .get("text")
            .and_then(Value::as_str)
            .map(str::to_string);
        if let Some(value) = text.and_then(|text| serde_json::from_str::<Value>(&text).ok()) {
            block["text"] = value;
        }
    }
    parsed
}

/// Every text block of every answer in `document`, in order.
fn blocks_mut(document: &mut Value) -> impl Iterator<Item = &mut Value> {
    document["expected"]["results"]
        .as_array_mut()
        .into_iter()
        .flatten()
        .filter_map(|result| result.get_mut("content").and_then(Value::as_array_mut))
        .flatten()
}

/// [`blocks_parsed`] undone on a document [`apply`] wrote: each block back to text, spelled as it
/// was committed where its value did not move, as `golf-mcp` spelled it where it became that
/// answer's block whole, and otherwise by `serde_json`'s pretty printer (keys sorted). That last is
/// the one spelling no server wrote: a block holding both a declared move and a value
/// `golf_mcp::family::DECLARED` keeps as Python recorded it. [M29 P15]
fn blocks_as_text(written: Value, committed: &Value, ours: &Value) -> Value {
    let texts = |document: &Value| -> Vec<Value> {
        let mut document = document.clone();
        blocks_mut(&mut document)
            .map(|block| block["text"].take())
            .collect()
    };
    let (committed, ours) = (texts(committed), texts(ours));
    // `blocks_parsed`'s reading of one text: its JSON, or the string itself.
    let read = |text: &Value| {
        text.as_str()
            .and_then(|text| serde_json::from_str::<Value>(text).ok())
            .unwrap_or_else(|| text.clone())
    };
    let mut written = written;
    for (i, block) in blocks_mut(&mut written).enumerate() {
        let value = block["text"].take();
        let spelled = [committed.get(i), ours.get(i)]
            .into_iter()
            .flatten()
            .find(|text| read(text) == value);
        block["text"] = match spelled {
            Some(text) => text.clone(),
            None if value.is_string() => value,
            None => {
                Value::String(serde_json::to_string_pretty(&value).expect("a Value serializes"))
            }
        };
    }
    written
}

/// The mcp family's [`MCP_DOCUMENTS`], each answered by `golf-mcp answer-case`, gated, applied and
/// ledgered: after the profile family under a career declaration [M29 P15], and after the
/// selection family under an engine declaration [M29 P19].
///
/// The family ages on `CAREER_VERSION` (the M29 plan's call 1): its tools read the corpus, the bag
/// and the aggregates the storage, career and profile families hold, so a change to what those
/// produce moves its answers in the same run, or the gate refuses them here.
///
/// **It holds the engine's numbers too, so an engine run walks it as well** (the M29 plan's call 1,
/// and P8's and P15's findings for P19). `simulate_flight` answers from the flight model (29
/// recorded calls, `provenance.flight_model_calls`), and the career and club tools join
/// `ranges.json`'s bands, so a `v<N>` that moves either moves these answers, and a walk that
/// skipped them would let the move land in the engine family and never here. Every case is walked
/// rather than only those with flight calls, because the bands reach further than the flight does.
/// Under an engine declaration the `career_version` this walk stamps is already current, so only an
/// answer that moved differs, and the ledger names the engine declaration's key. Frozen Python's
/// freeze pin (`test_the_committed_mcp_cases_are_what_frozen_python_answers`) cannot reach inside
/// a text block, so the first declaration of either key that moves one also teaches it to.
///
/// - **Answered by the binary.** Nothing may depend on `golf-mcp`, so this crate runs it
///   ([`golf_mcp_binary`], [`answer_with`]) rather than linking the family's runner. `answer-case`
///   answers every call and hands back the committed answers as it answers them on purpose: a
///   Python-recorded case through `golf_mcp::family::DECLARED`, a hand or Rust case as committed.
///   That list stays in `golf-mcp`, the one place the mcp gate reads it too, so the gate and this
///   walk cannot hold the family to two lists.
/// - **Gated against that, written onto the committed document** ([`one_against`]): a file keeps
///   frozen Python's value at every path no career declaration names, `DECLARED`'s paths included,
///   and only `expected.results` and `career_version` are this build's answer. Inputs are trees no
///   other vector answers, so nothing is adopted and nothing composes.
/// - **Text blocks are compared parsed** ([`blocks_parsed`]), as the mcp gate compares them, so a
///   declared path can name a value inside one, and the written file spells each block as it was
///   committed wherever its value held ([`blocks_as_text`]). Frozen Python's `frozen_view` cannot
///   reach inside a block, so the first career declaration that moves one also teaches the freeze
///   pin (`tests/test_conformance.py`) to; none has yet.
/// - **A protocol error is a refusal**, because nothing recorded one. `surface/` is
///   [`MCP_UNREAD`], and any other entry under `vectors/mcp/` stops the walk.
fn walk_mcp(
    vectors_dir: &Path,
    declaration: &Declaration,
    spelling: Option<&str>,
    binary: &Path,
) -> Result<Walked, Refused> {
    let mcp_dir = vectors_dir.join(MCP);
    every_entry_is_placed(
        &mcp_dir,
        MCP,
        (&MCP_DOCUMENTS, "MCP_DOCUMENTS"),
        Some((&MCP_UNREAD, "MCP_UNREAD")),
    )?;

    let mut vectors: Vec<Rerecorded> = Vec::new();
    let mut refusals: Vec<String> = Vec::new();
    let mut ledgered = Ledgered::default();
    let mut ids: BTreeSet<String> = BTreeSet::new();
    for half in MCP_DOCUMENTS {
        for path in vector_files(&mcp_dir.join(half))? {
            let (committed, layout) = read_vector(&path)?;
            let id = identify(&committed, &path, MCP)?;
            if !ids.insert(id.clone()) {
                return Err(Refused::Files(format!("{id} is the id of two mcp cases")));
            }
            ledgered.note(&committed, declaration);

            let (answers, recorded) = answer_with(binary, &id, &committed)?;
            let calls = committed["input"]["calls"].as_array().map_or(0, Vec::len);
            let (Some(answered), Some(_)) = (answers.as_array(), recorded.as_array()) else {
                return Err(Refused::Files(format!(
                    "{id}: golf-mcp answered no `answers` or no `recorded` list, so the case has \
                     no expected.results to hold it to"
                )));
            };
            if answered.len() != calls {
                return Err(Refused::Files(format!(
                    "{id}: {calls} calls and {} answers",
                    answered.len()
                )));
            }
            let errors: Vec<String> = answered
                .iter()
                .enumerate()
                .filter_map(|(i, answer)| {
                    answer["protocol_error"]
                        .as_str()
                        .map(|error| format!("{id}: call {i} was a protocol error: {error}"))
                })
                .collect();
            if !errors.is_empty() {
                refusals.extend(errors);
                continue;
            }

            let mut base = committed.clone();
            base["expected"]["results"] = recorded;
            let mut ours = committed.clone();
            ours["expected"]["results"] = answers;
            ours["career_version"] = json!(CAREER_VERSION);

            match one_against(
                &id,
                &blocks_parsed(&base),
                &blocks_parsed(&committed),
                &blocks_parsed(&ours),
                declaration,
                spelling,
            ) {
                Ok((applied, written, lines)) => {
                    let written =
                        (!applied.is_empty()).then(|| blocks_as_text(written, &committed, &ours));
                    vectors.push(Rerecorded {
                        id,
                        path,
                        family: Family::Mcp,
                        written,
                        applied,
                        lines,
                        layout,
                    });
                }
                Err(refused) => refusals.extend(refused),
            }
        }
    }
    if ids.is_empty() {
        return Err(Refused::Files(format!(
            "no mcp case under {}",
            mcp_dir.display()
        )));
    }
    Ok(Walked {
        vectors,
        refusals,
        ledgered,
    })
}

/// One screen document's `input` through `crates/screen`, as that document's whole `expected` —
/// `conformance_vectors._run_screen`'s counterpart, the definition every screen vector records.
/// [M34 P7]
///
/// The M34 plan's call 1, three answers:
///
/// - `label_ratio`, [`label_ratio`] over the boxes and the device's profile;
/// - `parsed`, the parse *before* validation: its `confidence` unrounded, its `warnings` without
///   the notes, and neither `device` nor `needs_review`, which `_run_screen` does not record — so a
///   key `ParsedShot` gains reaches a vector only by being listed here. M34 P8's `fields_present`
///   is deliberately not: it reaches every document that reads as a shot through
///   `shot.provenance.fields_present`, and a second copy here would be a second path to declare;
/// - `shot`, [`screen::read`]'s record, `null` where the read failed.
///
/// Each is this build's reader, whatever `parse_screen` and `read` are when the verb runs: a
/// re-record records the reader that ships, and the gate is what holds it to the committed answer.
/// Here rather than in `screen` for [`run_stages`]' reason: a vector's layout is vector plumbing, and
/// the reader knows nothing about `spec/`. P6's `parse-screen` is not reused, because it is a seam for
/// a person and prints the shot alone.
pub fn run_screen(input: &ScreenInput) -> Result<Value, UnknownProfile> {
    let profile = load_profile(&input.device)?;
    let parsed = parse_screen(&input.boxes, profile);
    let values: Map<String, Value> = parsed
        .values
        .iter()
        .map(|(target, value)| (target.to_string(), json!(value)))
        .collect();
    let raw_fields: Map<String, Value> = parsed
        .raw_fields
        .iter()
        .map(|(label, text)| (label.to_string(), json!(text)))
        .collect();
    Ok(json!({
        "label_ratio": label_ratio(&input.boxes, profile),
        "parsed": {
            "values": values,
            "raw_fields": raw_fields,
            "confidence": parsed.confidence,
            "warnings": parsed.warnings,
        },
        "shot": screen::read(input)?,
    }))
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
    one_against(id, committed, committed, ours, declaration, spelling)
}

/// [`one`], gating `ours` against `base` rather than against the committed document, and applying
/// what that lets through onto `committed`. The mcp walk's (M29 P15): its Python-recorded answers
/// are gated as `golf-mcp` answers them on purpose (`golf_mcp::family::DECLARED` applied, `base`),
/// and written back as Python recorded them but for the declared paths, so a file keeps every value
/// no declaration names. Every other family's `base` is its committed document.
fn one_against(
    id: &str,
    base: &Value,
    committed: &Value,
    ours: &Value,
    declaration: &Declaration,
    spelling: Option<&str>,
) -> Result<(Applied, Value, Vec<String>), Vec<String>> {
    let applied = gate(base, ours, declaration).map_err(|refused| {
        refused
            .iter()
            .map(|u| format!("{id}: {u}"))
            .collect::<Vec<_>>()
    })?;
    // A declared path the gate matched as a string and that walks to another place, or nowhere: a
    // key on it holds `.`, `[` or `]` (`try_apply`'s doc). Refused by name rather than written to the
    // wrong place or panicking. After a walk that succeeds, every difference the declaration
    // matched must be gone, which is what catches the wrong place.
    let unreachable = |path: &LedgerPath| {
        vec![format!(
            "{id}: {path} is declared and differs, but a key on that path holds `.`, `[` or `]`, \
             which a ledger path cannot name yet (a store's written files are such keys; the \
             module doc, \"What a declaration can name\")"
        )]
    };
    let mut written =
        try_apply(committed, ours, &applied).map_err(|(path, _)| unreachable(&path))?;
    if !applied.is_empty() {
        let still = compare(&written, ours);
        let mut declared = applied
            .added
            .iter()
            .chain(&applied.moved)
            .chain(&applied.removed);
        if let Some(path) = declared.find(|p| still.iter().any(|d| d.path == p.as_str())) {
            return Err(unreachable(path));
        }
    }
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
    lines.extend(
        applied
            .removed
            .iter()
            .map(|path| format!("removed {path} (was {})", brief(path.get(committed)))),
    );
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
    removed: BTreeSet<String>,
}

impl Ledgered {
    fn extend(&mut self, later: Ledgered) {
        self.added.extend(later.added);
        self.moved.extend(later.moved);
        self.removed.extend(later.removed);
    }

    fn note(&mut self, committed: &Value, declaration: &Declaration) {
        let entries = committed["provenance"]["rerecords"].as_array();
        let version = declaration.version;
        for entry in entries.into_iter().flatten() {
            if entry[version.key()].as_i64() != Some(version.number())
                || entry["by"] != RERECORDED_BY
            {
                continue;
            }
            for (kind, into) in [
                ("added", &mut self.added),
                ("moved", &mut self.moved),
                ("removed", &mut self.removed),
            ] {
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
        found.extend(
            vector
                .applied
                .removed
                .iter()
                .map(|p| ("removed", p.as_str())),
        );
    }
    let mut unmatched = Vec::new();
    for (kind, declared, recorded) in [
        ("added", &declaration.added, &ledgered.added),
        ("moved", &declaration.moved, &ledgered.moved),
        ("removed", &declaration.removed, &ledgered.removed),
    ] {
        for path in declared {
            if found.contains(&(kind, path.as_str())) || recorded.contains(path.as_str()) {
                continue;
            }
            unmatched.push(format!(
                "{path} (declared `{kind}`): no vector differs there and no vector's ledger records \
                 it at {} — a misspelling, or a value that moved within the tolerance or not at all, \
                 which the gate does not report as a difference (M32's plan, call 5)",
                declaration.version
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

/// How a committed file's text is laid out, which the writer keeps (module doc, "How a file is
/// written").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Layout {
    crlf: bool,
    /// Spaces per level. [M35 P6]
    indent: usize,
}

impl Layout {
    /// `_write_json`'s, which every recorded file has and a file of one line is given.
    const RECORDED: Self = Self {
        crlf: false,
        indent: 2,
    };

    /// The text's own: CRLF if it holds one, and the second line's leading spaces, which are the
    /// first key's, since every vector is an object.
    fn of(text: &str) -> Self {
        let indent = text
            .lines()
            .nth(1)
            .map(|line| line.len() - line.trim_start_matches(' ').len())
            .filter(|&spaces| spaces > 0)
            .unwrap_or(Self::RECORDED.indent);
        Self {
            crlf: text.contains("\r\n"),
            indent,
        }
    }
}

/// A committed vector, and its text's [`Layout`]. Gzipped or not by the suffix, as `_read_json`.
fn read_vector(path: &Path) -> Result<(Value, Layout), Refused> {
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
    Ok((document, Layout::of(&text)))
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

/// The bytes of a vector with no committed file yet, laid out as `_write_json` laid out every file
/// it wrote ([`Layout::RECORDED`]) and otherwise as the module doc's "How a file is written" says.
/// [M29 P30]
///
/// `golf-lab build-vectors` writes a new vector through this, so a built file and a re-recorded one
/// come out of one writer: keys sorted, indent 2, gzip by the `.gz` suffix with mtime 0 and the
/// uncompressed name. A plain `.json` comes out LF, which is `_write_json`'s source; the CRLF a
/// committed plain file carries is Windows', and a re-record keeps it only because it is there.
pub fn new_vector_bytes(document: &Value, path: &Path) -> Vec<u8> {
    file_bytes(document, path, Layout::RECORDED)
}

/// The bytes of one vector file, as the module doc's "How a file is written" says.
fn file_bytes(document: &Value, path: &Path, layout: Layout) -> Vec<u8> {
    let indent = " ".repeat(layout.indent);
    let mut pretty = Vec::new();
    document
        .serialize(&mut Serializer::with_formatter(
            &mut pretty,
            PrettyFormatter::with_indent(indent.as_bytes()),
        ))
        .expect("a Value always serializes");
    let mut text = String::from_utf8(pretty).expect("serde_json writes UTF-8");
    let gzipped = is_gzipped(path);
    if !gzipped {
        text.push('\n');
    }
    if layout.crlf {
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

    /// Declared or not, a key dropped from an engine answer is refused: an engine declaration has no
    /// `removed` (the load refuses one), so naming the path under `moved` changes nothing.
    #[test]
    fn a_removed_key_never_passes_an_engine_declaration() {
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
        assert_eq!(m32().version, Version::Analysis(17));
    }

    /// Call 6 (M34's): the version key a declaration carries picks its family, and it carries
    /// exactly one — both would be two versions' changes in one review, and neither leaves the run
    /// with no family to walk.
    #[test]
    fn a_declaration_names_exactly_one_version_and_it_picks_the_family() {
        let screen: Declaration = serde_json::from_value(json!({
            "screen_parser_version": 1, "note": "", "added": [], "moved": [],
        }))
        .expect("a screen declaration");
        assert_eq!(screen.version, Version::ScreenParser(1));
        assert_eq!(screen.version.key(), "screen_parser_version");
        assert_eq!(screen.version.current(), SCREEN_PARSER_VERSION);
        assert_eq!(screen.version.to_string(), "screen_parser_version 1");
        assert_eq!(m32().version.key(), "analysis_version");
        assert_eq!(m32().version.current(), ANALYSIS_VERSION);
        let career: Declaration = serde_json::from_value(json!({
            "career_version": 1, "note": "", "added": [], "moved": [],
        }))
        .expect("a career declaration");
        assert_eq!(career.version, Version::Career(1));
        assert_eq!(career.version.key(), "career_version");
        assert_eq!(career.version.current(), CAREER_VERSION);
        assert_eq!(career.version.to_string(), "career_version 1");

        for (file, why) in [
            (
                json!({"analysis_version": 17, "screen_parser_version": 1, "note": "",
                       "added": [], "moved": []}),
                "more than one version (analysis_version, screen_parser_version)",
            ),
            (
                json!({"screen_parser_version": 1, "career_version": 1, "note": "",
                       "added": [], "moved": []}),
                "more than one version (screen_parser_version, career_version)",
            ),
            (
                json!({"analysis_version": 17, "screen_parser_version": 1,
                       "career_version": 1, "note": "", "added": [], "moved": []}),
                "more than one version",
            ),
            (
                json!({"note": "", "added": [], "moved": []}),
                "carries no version",
            ),
            (
                json!({"careers_version": 1, "note": "", "added": [], "moved": []}),
                "unknown field",
            ),
        ] {
            let refused =
                serde_json::from_value::<Declaration>(file.clone()).expect_err(&file.to_string());
            assert!(refused.to_string().contains(why), "{file}: {refused}");
        }
    }

    /// A screen vector's ledger entry says `screen_parser_version`, the version that vector ages on,
    /// and never `analysis_version`: a screen vector carrying an engine version would go stale on
    /// every engine bump (`tests/test_conformance.py` pins the top level the same way).
    #[test]
    fn a_screen_ledger_entry_is_keyed_by_the_screen_parser_version() {
        let screen: Declaration = serde_json::from_value(json!({
            "screen_parser_version": 1,
            "note": "",
            "added": ["expected.shot.provenance.parser_version"],
            "moved": ["screen_parser_version"],
        }))
        .expect("a screen declaration");
        let committed = json!({
            "id": "screen/corpus/2026-08-23-2",
            "screen_parser_version": 0,
            "provenance": {"kind": "screen", "oracle": "python"},
            "expected": {"shot": {"provenance": {"device": "hd_golf"}}},
        });
        let mut ours = committed.clone();
        ours["screen_parser_version"] = json!(1);
        ours["expected"]["shot"]["provenance"]["parser_version"] = json!(0);

        let applied = gate(&committed, &ours, &screen).expect("declared");
        let mut written = apply(&committed, &ours, &applied);
        ledger(
            &mut written,
            &applied,
            &screen,
            "spec/declarations/screen-v1.json",
        )
        .expect("a ledger");
        assert_eq!(
            written["provenance"]["rerecords"],
            json!([{
                "screen_parser_version": 1,
                "by": "golf-core rerecord",
                "declaration": "spec/declarations/screen-v1.json",
                "added": ["expected.shot.provenance.parser_version"],
                "moved": ["screen_parser_version"],
                "removed": [],
            }])
        );
        assert_eq!(written["provenance"]["oracle"], json!("python"));
    }

    /// A storage or career vector's ledger entry says `career_version`, the version both families
    /// age on, and has no `removed` list: a career declaration cannot remove, so its entries keep the
    /// engine's shape. [M36 P13]
    #[test]
    fn a_career_ledger_entry_is_keyed_by_the_career_version_and_never_removes() {
        let career: Declaration = serde_json::from_value(json!({
            "career_version": 1,
            "note": "",
            "added": [],
            "moved": ["career_version", "expected.corpus.excluded[0].detail"],
        }))
        .expect("a career declaration");
        let committed = json!({
            "id": "storage/corpus/outdated-versions",
            "career_version": 0,
            "provenance": {"kind": "storage", "oracle": "python"},
            "expected": {"corpus": {"excluded": [{"detail": "older than installed"}]}},
        });
        let mut ours = committed.clone();
        ours["career_version"] = json!(1);
        ours["expected"]["corpus"]["excluded"][0]["detail"] = json!("older than comparable");

        let applied = gate(&committed, &ours, &career).expect("declared");
        let mut written = apply(&committed, &ours, &applied);
        ledger(
            &mut written,
            &applied,
            &career,
            "spec/declarations/career-v1.json",
        )
        .expect("a ledger");
        assert_eq!(
            written["provenance"]["rerecords"],
            json!([{
                "career_version": 1,
                "by": "golf-core rerecord",
                "declaration": "spec/declarations/career-v1.json",
                "added": [],
                "moved": ["career_version", "expected.corpus.excluded[0].detail"],
            }])
        );

        // And a removed key is refused there as on an engine vector, saying so.
        let mut dropped = committed.clone();
        dropped["expected"]
            .as_object_mut()
            .expect("an object")
            .remove("corpus");
        let refused = gate(&committed, &dropped, &career).expect_err("a removal");
        assert_eq!(paths(&refused), ["expected.corpus"]);
        assert!(
            refused[0]
                .to_string()
                .contains("an engine or career re-record never removes a key"),
            "{}",
            refused[0]
        );
    }

    /// A store's written file is a key holding `.`, so a declared path naming a move inside one
    /// matches the gate as a string and walks to another place. [`one`] refuses it by name rather
    /// than panicking or writing elsewhere — whether the walk finds nothing, or finds a decoy at
    /// the place the dots spell. [M36 P13]
    #[test]
    fn a_declared_path_through_a_dotted_key_is_refused_by_name() {
        let declaration: Declaration = serde_json::from_value(json!({
            "career_version": 1,
            "note": "",
            "added": [],
            "moved": ["expected.files.a.json"],
        }))
        .expect("a career declaration");
        let committed = json!({
            "id": "storage/bundle/x",
            "provenance": {"kind": "storage"},
            "expected": {"files": {"a.json": "{\"x\": 1}"}},
        });
        let mut ours = committed.clone();
        ours["expected"]["files"]["a.json"] = json!("{\"x\": 2}");
        let refused = one("storage/bundle/x", &committed, &ours, &declaration, None)
            .expect_err("the walk finds no `a`");
        assert_eq!(refused.len(), 1);
        assert!(
            refused[0].starts_with("storage/bundle/x: expected.files.a.json is declared")
                && refused[0].contains("cannot name yet"),
            "{}",
            refused[0]
        );

        // A decoy at `files.a.json` spelled by the dots, which the walk would reach and copy.
        let mut decoyed = committed.clone();
        decoyed["expected"]["files"]["a"] = json!({"json": 1});
        let mut ours = decoyed.clone();
        ours["expected"]["files"]["a.json"] = json!("{\"x\": 2}");
        ours["expected"]["files"]["a"]["json"] = json!(1);
        let refused = one("storage/bundle/x", &decoyed, &ours, &declaration, None)
            .expect_err("the walk reaches the decoy");
        assert!(refused[0].contains("cannot name yet"), "{}", refused[0]);
    }

    /// The user's answer to the M34 plan's P7 finding 3, on the shape decision 3 makes: a tile the
    /// tie rule withholds loses its `raw_fields` key. A screen declaration's `removed` lets exactly
    /// the named key go, [`apply`] takes it out, the ledger lists it, and the report says what it
    /// held — while an unnamed removal in the same document is still refused, saying why.
    #[test]
    fn a_screen_declaration_removes_exactly_the_keys_it_names() {
        let named = "expected.parsed.raw_fields.Impact Position";
        let screen = |removed: &[&str]| -> Declaration {
            serde_json::from_value(json!({
                "screen_parser_version": 1,
                "note": "",
                "added": [],
                "moved": ["screen_parser_version"],
                "removed": removed,
            }))
            .expect("a screen declaration")
        };
        let committed = json!({
            "id": "screen/corpus/2026-08-23-1",
            "screen_parser_version": 0,
            "provenance": {"kind": "screen", "oracle": "python"},
            "expected": {"parsed": {"raw_fields": {"Carry": "151.5", "Impact Position": "HEEL"}}},
        });
        let mut ours = committed.clone();
        ours["screen_parser_version"] = json!(1);
        ours["expected"]["parsed"]["raw_fields"]
            .as_object_mut()
            .expect("raw fields")
            .remove("Impact Position");

        let applied = gate(&committed, &ours, &screen(&[named])).expect("the removal is named");
        assert_eq!(applied.removed().len(), 1);
        assert_eq!(applied.removed()[0].as_str(), named);
        let mut written = apply(&committed, &ours, &applied);
        assert_eq!(written, ours, "the key is gone and nothing else moved");
        ledger(
            &mut written,
            &applied,
            &screen(&[named]),
            "spec/declarations/screen-v1.json",
        )
        .expect("a ledger");
        assert_eq!(
            written["provenance"]["rerecords"][0]["removed"],
            json!([named])
        );

        let (_, _, lines) = one(
            "screen/corpus/2026-08-23-1",
            &committed,
            &ours,
            &screen(&[named]),
            None,
        )
        .expect("declared");
        assert!(
            lines.contains(&format!("removed {named} (was \"HEEL\")")),
            "{lines:?}"
        );

        let refused = gate(&committed, &ours, &screen(&[])).expect_err("an unnamed removal");
        assert_eq!(paths(&refused), [named]);
        assert_eq!(refused[0].0.kind, DifferenceKind::RemovedKey);
        assert!(
            refused[0].to_string().contains("`removed`"),
            "{}",
            refused[0]
        );

        // A removal named where nothing is removed matches nothing here, as an `added` path that
        // finds no key does; whether it matched anywhere is the run's typo guard.
        let applied = gate(&committed, &committed, &screen(&[named])).expect("nothing differs");
        assert!(applied.is_empty());
    }

    /// `removed` is the screen family's alone, and held to `added`'s spelling rules.
    #[test]
    fn removed_is_a_screen_declarations_alone() {
        for (file, why) in [
            (
                json!({"analysis_version": 17, "note": "", "added": [], "moved": [],
                       "removed": ["expected.swing.shot.ball_speed"]}),
                "never removes a key",
            ),
            (
                json!({"analysis_version": 17, "note": "", "added": [], "moved": [],
                       "removed": []}),
                "never removes a key",
            ),
            (
                json!({"career_version": 1, "note": "", "added": [], "moved": [],
                       "removed": ["expected.corpus.excluded"]}),
                "a declaration at `career_version` carries `removed`",
            ),
            (
                json!({"career_version": 1, "note": "", "added": [], "moved": [],
                       "removed": []}),
                "never removes a key",
            ),
            (
                json!({"screen_parser_version": 1, "note": "", "added": [], "moved": [],
                       "removed": ["expected.parsed.warnings[2]"]}),
                "ends in a list index",
            ),
            (
                json!({"screen_parser_version": 1, "note": "", "added": [], "moved": ["a"],
                       "removed": ["a"]}),
                "declared twice",
            ),
            (
                json!({"screen_parser_version": 1, "note": "", "added": [], "moved": [],
                       "remove": ["a"]}),
                "unknown field",
            ),
        ] {
            let refused =
                serde_json::from_value::<Declaration>(file.clone()).expect_err(&file.to_string());
            assert!(refused.to_string().contains(why), "{file}: {refused}");
        }
        assert!(
            m32().removed.is_empty(),
            "v17's shape loads with no removal"
        );
    }

    /// The question M34 P7 was asked before the screen family's warnings lists grow: a list of
    /// another length is one move *at the list's own path*, declared there, and lands whole; a list
    /// of the same length moves per index, which the list's path does not cover. So a declaration
    /// can name a grown list, and needs the index spelling only where a list kept its length.
    #[test]
    fn a_list_is_declared_whole_when_its_length_changes() {
        let committed = json!({"id": "x", "provenance": {}, "warnings": ["a", "b"]});
        let grown = json!({"id": "x", "provenance": {}, "warnings": ["a", "b", "c"]});
        let replaced = json!({"id": "x", "provenance": {}, "warnings": ["a", "z"]});
        let whole = declaration(&[], &["warnings"]);

        let applied = gate(&committed, &grown, &whole).expect("the grown list is declared whole");
        assert_eq!(applied.moved()[0].as_str(), "warnings");
        assert_eq!(apply(&committed, &grown, &applied), grown);

        let refused = gate(&committed, &replaced, &whole).expect_err("a same-length list");
        assert_eq!(paths(&refused), ["warnings[1]"]);
        let indexed = declaration(&[], &["warnings", "warnings[1]"]);
        let applied = gate(&committed, &replaced, &indexed).expect("declared at the index");
        assert_eq!(apply(&committed, &replaced, &applied), replaced);
        assert_eq!(
            gate(&committed, &grown, &indexed).map(|a| a.moved().len()),
            Ok(1)
        );
    }

    /// The writer's bytes, against what `_write_json` and `GzipFile` wrote into the committed files:
    /// a plain file keeps its CRLF and ends in a newline; a gzipped one is LF, ends without one, and
    /// carries `GzipFile`'s header — the name less `.gz`, mtime 0, level 9's XFL, OS "unknown".
    #[test]
    fn a_file_is_written_the_way_the_committed_ones_were() {
        let document = json!({"b": [1, 2.5], "a": {"x": null}, "c": "line\nbreak"});
        let crlf = Layout {
            crlf: true,
            ..Layout::RECORDED
        };
        let lf = Layout::RECORDED;

        let plain =
            String::from_utf8(file_bytes(&document, Path::new("v.json"), crlf)).expect("utf-8");
        assert!(plain.starts_with("{\r\n  \"a\": {\r\n    \"x\": null\r\n  },\r\n"));
        assert!(plain.ends_with("}\r\n"));
        assert!(!plain.replace("\r\n", "").contains('\n'), "a bare LF");
        assert!(
            plain.contains(r#""line\nbreak""#),
            "the string's newline is still escaped"
        );
        let unix = file_bytes(&document, Path::new("v.json"), lf);
        assert!(!unix.contains(&b'\r'));
        assert_eq!(
            serde_json::from_str::<Value>(&plain).expect("parses"),
            document
        );

        let gz = file_bytes(&document, Path::new("2026-08-10-1.json.gz"), lf);
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

    /// **A file keeps its own indent** (M35 P6): read from the second line, written back at it, so a
    /// hand-worked case at 4 is rewritten at 4 and a recorded vector at 2 stays at 2 — and a file of
    /// one line, which has no second line to read, is given `_write_json`'s 2.
    #[test]
    fn a_file_keeps_the_indent_it_was_committed_with() {
        let document = json!({"a": {"x": [1]}, "b": "c"});
        let hand = "{\n    \"a\": {\n        \"x\": [\n            1\n        ]\n    },\n    \"b\": \"c\"\n}\n";
        assert_eq!(
            Layout::of(hand),
            Layout {
                crlf: false,
                indent: 4
            }
        );
        assert_eq!(
            String::from_utf8(file_bytes(&document, Path::new("v.json"), Layout::of(hand)))
                .expect("utf-8"),
            hand
        );

        let recorded = hand.replace("    ", "  ").replace('\n', "\r\n");
        assert_eq!(
            Layout::of(&recorded),
            Layout {
                crlf: true,
                indent: 2
            }
        );
        assert_eq!(
            String::from_utf8(file_bytes(
                &document,
                Path::new("v.json"),
                Layout::of(&recorded)
            ))
            .expect("utf-8"),
            recorded
        );
        assert_eq!(Layout::of(&document.to_string()), Layout::RECORDED);
    }

    /// **A new vector is laid out as `_write_json` laid one out** (M29 P30): keys sorted, indent 2,
    /// LF and a trailing newline on a plain file, and a gzip that holds the same text less the
    /// newline, stamped with the uncompressed name.
    #[test]
    fn a_new_vector_is_written_as_write_json_wrote_one() {
        let document = json!({"b": [1], "a": "é"});
        let plain =
            String::from_utf8(new_vector_bytes(&document, Path::new("x/v.json"))).expect("utf-8");
        assert_eq!(plain, "{\n  \"a\": \"é\",\n  \"b\": [\n    1\n  ]\n}\n");

        let gzipped = new_vector_bytes(&document, Path::new("x/v.json.gz"));
        let mut decoder = GzDecoder::new(&gzipped[..]);
        let mut text = String::new();
        decoder.read_to_string(&mut text).expect("a gzip");
        assert_eq!(text, plain.trim_end_matches('\n'));
        let header = decoder.header().expect("a gzip header");
        assert_eq!(header.filename(), Some(&b"v.json"[..]));
        assert_eq!(header.mtime(), 0);
    }

    #[test]
    fn a_report_value_is_cut_short_on_a_character_boundary() {
        assert_eq!(brief(None), "<absent>");
        assert_eq!(brief(Some(&json!(17))), "17");
        let long = brief(Some(&json!("é".repeat(100))));
        assert!(long.ends_with('…'));
        assert_eq!(long.chars().count(), 73);
    }

    /// An mcp case with one answer of one text block, the block spelled as `spelling` writes its
    /// item. [M29 P15]
    fn mcp_case(item: &Value, spelling: fn(&Value) -> String, version: i64) -> Value {
        json!({
            "id": "mcp/tools/one",
            "career_version": version,
            "provenance": {"kind": "mcp", "oracle": "python"},
            "input": {"calls": [{"tool": "get_bag_profile"}]},
            "expected": {"results": [{
                "content": [{"type": "text", "text": spelling(item)}],
                "structuredContent": {"result": item},
                "isError": false,
            }]},
        })
    }

    /// Python's `indent=2` with `ensure_ascii`, the committed spelling, as serde_json would not
    /// write it: keys out of order and a `—` escaped.
    fn pythons(item: &Value) -> String {
        let n = &item["n"];
        format!(
            "{{\n  \"note\": {},\n  \"n\": {n}\n}}",
            item["note"].to_string().replace('—', "\\u2014")
        )
    }

    fn rusts(item: &Value) -> String {
        serde_json::to_string_pretty(item).unwrap()
    }

    /// **A text block is gated as the value it spells and written back as it was spelled** (M29
    /// P15): Python's and Rust's spellings of one item are no difference, a value moved inside a
    /// block is one at a path inside its text, and a run that moves only the version leaves the
    /// block's committed spelling byte for byte; a block whose value moved takes Rust's spelling.
    #[test]
    fn an_mcp_block_is_compared_parsed_and_written_in_its_committed_spelling() {
        let item = json!({"note": "a — b", "n": 3});
        let committed = mcp_case(&item, pythons, 0);
        let ours = mcp_case(&item, rusts, CAREER_VERSION);
        let career = |moved: &[&str]| -> Declaration {
            serde_json::from_value(json!({
                "career_version": CAREER_VERSION, "note": "a test", "added": [], "moved": moved,
            }))
            .unwrap()
        };
        let (c, o) = (blocks_parsed(&committed), blocks_parsed(&ours));
        let applied = gate(&c, &o, &career(&["career_version"])).expect("only the version moved");
        assert_eq!(
            applied
                .moved()
                .iter()
                .map(LedgerPath::as_str)
                .collect::<Vec<_>>(),
            ["career_version"]
        );
        let written = blocks_as_text(apply(&c, &o, &applied), &committed, &ours);
        assert_eq!(
            written["expected"]["results"][0]["content"][0]["text"],
            committed["expected"]["results"][0]["content"][0]["text"],
            "an unmoved block keeps Python's spelling"
        );
        assert_eq!(written["career_version"], CAREER_VERSION);

        let moved = json!({"note": "a — b", "n": 4});
        let ours = mcp_case(&moved, rusts, CAREER_VERSION);
        let o = blocks_parsed(&ours);
        let refused = gate(&c, &o, &career(&["career_version"])).expect_err("n moved");
        let paths: Vec<&str> = refused.iter().map(|u| u.0.path.as_str()).collect();
        assert_eq!(
            paths,
            [
                "expected.results[0].content[0].text.n",
                "expected.results[0].structuredContent.result.n",
            ]
        );
        let both = career(&[
            "career_version",
            "expected.results[0].content[0].text.n",
            "expected.results[0].structuredContent.result.n",
        ]);
        let applied = gate(&c, &o, &both).expect("declared");
        let written = blocks_as_text(apply(&c, &o, &applied), &committed, &ours);
        assert_eq!(
            written["expected"]["results"][0]["content"][0]["text"],
            rusts(&moved),
            "a block that became Rust's answer takes Rust's spelling"
        );
    }

    /// A block that is not JSON stays a string through both directions, and `one_against` gates
    /// against its base while writing onto the committed document. [M29 P15]
    #[test]
    fn one_against_gates_on_its_base_and_writes_onto_the_committed_document() {
        let mut committed = mcp_case(&json!({"note": "recorded", "n": 1}), rusts, 0);
        committed["expected"]["results"][0]["content"][0]["text"] = json!("not json");
        assert_eq!(blocks_parsed(&committed), committed);
        // The base is the committed answer as the server answers it on purpose; ours agrees with
        // the base and not with the committed document, at a path no declaration names.
        let mut base = committed.clone();
        base["expected"]["results"][0]["structuredContent"]["result"]["note"] = json!("ours");
        let mut ours = base.clone();
        ours["career_version"] = json!(CAREER_VERSION);
        let declaration: Declaration = serde_json::from_value(json!({
            "career_version": CAREER_VERSION, "note": "a test", "added": [],
            "moved": ["career_version"],
        }))
        .unwrap();
        let (applied, written, lines) = one_against(
            "mcp/tools/one",
            &base,
            &committed,
            &ours,
            &declaration,
            None,
        )
        .expect("the base agrees with ours but for the version");
        assert_eq!(
            lines,
            [format!("moved  career_version: 0 -> {CAREER_VERSION}")]
        );
        assert!(!applied.is_empty());
        assert_eq!(
            written["expected"]["results"][0]["structuredContent"]["result"]["note"], "recorded",
            "a value the base rewrote stays as committed in the file"
        );
        assert!(one("mcp/tools/one", &committed, &ours, &declaration, None).is_err());
    }
}
