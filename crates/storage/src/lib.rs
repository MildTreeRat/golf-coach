//! The stores the many-shot layer reads and writes. The Rust half of `src/golf_coach/storage/`,
//! with the two Python modules outside that package that are stores in all but name. [M36 P8]
//!
//! The tenth crate in the workspace, and the one the M36 plan's call 6 places by Python module, not
//! by what a module does:
//!
//! - [`manifest`] — `storage/manifest.py`: which files have arrived for one swing, their content
//!   hashes, and the tolerant read and atomic write of `manifest.json`;
//! - [`state`] — `api/state.py`'s `AnalysisState` and its tolerant readers for `analysis.state.json`
//!   and `analysis.json`: what a machine later derived from a swing, and whether it is still true;
//! - [`golfer_store`] — `storage/golfer_store.py`, the golfer registry, with
//!   `contracts/golfer.py::slugify`, which needs a Unicode normalization table `contracts` does not
//!   carry;
//! - [`shot_store`] — `launch_monitor/screen/store.py`, the parsed-shot cache keyed by the photo's
//!   hash;
//! - [`catalogue`] — `clubs/catalogue.py`'s key and keyed reader, which fold through `slugify` and
//!   so could not sit beside `contracts::catalogue`'s row reader (M36 P5's finding 4);
//! - [`bag_store`] — `storage/bag_store.py`, the declared bags beside the golfers, with the write
//!   guard that refuses to replace a bag it cannot read (M36 P9);
//! - [`bundle_store`] — `storage/bundle_store.py`, which slots each upload into a swing by role and
//!   holds the per-swing repairs (M36 P9);
//! - [`corpus`] — `storage/corpus.py`, one golfer's distinct swings across every session and the
//!   honest sample size behind each metric, read over the bundle store and the state readers, with
//!   the engine generations it judges a stored analysis against as a parameter (M36 P10).
//!
//! # ADR-008's exception is not carried over
//!
//! In Python, `storage/corpus.py` imports *upward* into `api.state` for the tolerant artifact
//! readers, the one knowing exception ADR-008's addendum records, and that file's own comment names
//! the honest fix: move the readers into `storage/` and let the API re-export them. [`state`] is that
//! fix. The readers are a module of this crate, the corpus reaches them without reaching the API, and
//! the Rust workspace has no exception to record: this crate depends on `contracts` and `pyfmt`, and
//! nothing of ours depends on it but `crates/core`, the shell.
//!
//! # Tolerant readers, and the coercions they refuse
//!
//! Every reader that frozen Python answers `None` from answers `None` here on the same file: missing,
//! unreadable, not JSON, a missing required key, a value out of bounds, an enum member nobody
//! declared, a `null` where the field is not optional. Unknown keys are ignored, as pydantic's
//! `extra="ignore"` ignores them. A half-written or older-schema file should cost a re-analysis or a
//! re-entered name, never an error on the page someone is holding between swings.
//!
//! **What pydantic also accepts, and nothing here writes, is refused** (the M36 plan's call 9): a
//! numeric string or an integral float for an `int`, `1` or `"true"` for a `bool`, a string for a
//! `float`, and a naive, Unix-number or date-only datetime (`contracts::time` lists the spellings).
//! So a file holding one reads as `None` here where Python reads it. No vector covers this, on
//! purpose: every file the stores read was written by pydantic or by this crate, and neither writes
//! any of them. `json.loads`, behind [`state::load_analysis`], is laxer in its own ways, and those are
//! refused too: `NaN` and `Infinity`, an unpaired surrogate escape, and an integer past `u64` (which
//! `serde_json` reads as a float, so [`state::stored_analysis_version`] reads it as 0 where Python
//! reads a very new engine).
//!
//! # The clock is an argument
//!
//! Every operation that stamps a time takes `now: Timestamp` from its caller (the M36 plan's call 4),
//! where frozen Python reads `datetime.now(tz=UTC)` for itself. A caller with a real clock passes
//! [`contracts::Timestamp::now_utc`]; the storage family passes each op's recorded `now`, which is
//! how the vectors pin what a write stamps without freezing a process clock.
//!
//! # What a call fails with
//!
//! [`StoreError`], by the Python exception each variant stands for, so the storage family's
//! `raised: {type, message}` maps onto it without parsing a message back.
//!
//! # What is not ported, because it is the machine and not the store
//!
//! The storage family's header lists the edges no vector reaches because each would record the
//! recording machine, and this crate takes the POSIX answer to each: a `glob` is case-sensitive and
//! a listing sorts by code point (Windows' `pathlib` folds case for both); an original filename's
//! name is what follows its last `/` or `\` (Windows' rule, which keeps a backslash out of a stored
//! filename on every platform); and the filesystem's own case-folding is left to the filesystem.
//! Line endings differ too: Python's `write_text` writes `\r\n` on Windows, this writes `\n`, and
//! both are whitespace between JSON tokens, which is all either reader sees.

use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use contracts::ContractError;
use serde::Serialize;

pub mod bag_store;
pub mod bundle_store;
pub mod catalogue;
pub mod corpus;
pub mod golfer_store;
pub mod manifest;
pub mod shot_store;
pub mod state;

/// What a store call can fail with, named for the Python exception it stands for.
#[derive(Debug)]
pub enum StoreError {
    /// A `ValueError` the store raises itself, with frozen Python's message: the golfer store's
    /// `name '!!!' has no usable characters for an id`, or the bag store's `bag at <path> exists but
    /// cannot be read; refusing to overwrite it`. The storage family records the text, so it is
    /// built with `pyfmt`'s `repr` where Python's f-string used `!r`, and a path as the platform
    /// spells it, which the gate reads back with the scratch root as `<root>/`.
    Refused(String),
    /// A value the store assembled that a contract refuses: pydantic's `ValidationError` raised by a
    /// validator, whose message is [`ContractError::problem`] (M36 P5's finding 6).
    Invalid(ContractError),
    /// A file that does not hold the shape it should: pydantic's `ValidationError` over text that is
    /// not JSON, misses a required key or breaks a bound. Its wording is serde's, which is not
    /// pydantic's, so the storage family records none for it and nothing compares it.
    Unreadable { path: PathBuf, reason: String },
    /// An `OSError`.
    Io(io::Error),
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StoreError::Refused(message) => f.write_str(message),
            StoreError::Invalid(error) => write!(f, "{error}"),
            StoreError::Unreadable { path, reason } => {
                write!(
                    f,
                    "{} does not hold what it should: {reason}",
                    path.display()
                )
            }
            StoreError::Io(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for StoreError {}

impl From<io::Error> for StoreError {
    fn from(error: io::Error) -> Self {
        StoreError::Io(error)
    }
}

/// `path.with_suffix(".tmp")`, where every store writes before it renames.
///
/// `Path::with_extension` and Python's `with_suffix` disagree on a name ending in `.` or starting
/// with one (`clip.` is `clip.tmp` here and `clip..tmp` there), and agree on every name ending in a
/// suffix. Every caller's name ends in `.json` — `manifest.json`, `<id>.golfer.json`,
/// `analysis.state.json` — so the stale `manifest.tmp` the storage family pins being consumed is the
/// same file in both languages.
pub(crate) fn tmp_beside(path: &Path) -> PathBuf {
    path.with_extension("tmp")
}

/// Write `value` as indented JSON beside `path` and rename it over `path`: atomic against a
/// concurrent reader, which is why the status page never sees half a manifest.
///
/// Byte identity with pydantic's `model_dump_json(indent=2)` is not the contract and does not hold
/// (key order, float spelling, line endings): what frozen Python reads by is the value, and that is
/// what the storage family pins.
pub(crate) fn write_atomically<T: Serialize>(value: &T, path: &Path) -> io::Result<()> {
    let tmp = tmp_beside(path);
    fs::write(&tmp, to_pretty_json(value))?;
    fs::rename(&tmp, path)
}

/// `model_dump_json(indent=2)`'s layout: two spaces, non-ASCII written as itself.
pub(crate) fn to_pretty_json<T: Serialize>(value: &T) -> String {
    serde_json::to_string_pretty(value).expect("a store's shapes serialize")
}
