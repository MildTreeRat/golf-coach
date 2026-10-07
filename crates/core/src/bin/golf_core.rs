//! `golf-core` — the CLI end of `docs/CONFORMANCE.md` §4's cross-language seam [M22 P6], and from
//! M32 the recorder of the vectors that seam is judged by.
//!
//! ```text
//! golf-core run < vector.json                                      # vector in, result out
//! golf-core rerecord --declare <file> [--dry-run] [--spec <dir>]   # re-record one family, gated
//! golf-core parse-screen < screen-vector.json                      # OCR boxes in, ShotData out
//! ```
//!
//! # `run`
//!
//! The counterpart of `python scripts/conformance.py run < vector.json`, and the two are meant to be
//! diffed against each other with no Python in the loop but the reference:
//!
//! ```text
//! diff <(python scripts/conformance.py run < v.json) <(cargo run -q --bin golf-core -- run < v.json)
//! ```
//!
//! **That diff is not the gate**, and the difference matters. A byte comparison of the two is
//! unavailable in either direction — Python writes `-1.636758133827243e-05` where `serde_json` writes
//! `-0.00001636758133827243` for the same f64, and seventy-seven distinct floats in the committed
//! vectors take the exponent form (M22 P2). So the diff above shows *where* two implementations
//! disagree once you have already decided they do; `golf_core::compare` is what decides, under §3's
//! structural rules, and `crates/core/tests/engine.rs` is the gate that runs it.
//!
//! # `rerecord`, and why it is not `regenerate` under another name
//!
//! Until M32 this doc said `regenerate` was deliberately not a candidate here: the vectors are the
//! oracle, and a port that can rewrite them is a port that passes by construction — the argument
//! `conformance_vectors._audio` makes about the family M20 deleted. That was ADR-032 §7's position
//! while the port was still being judged by Python's answers, and it was right then. ADR-035 clause
//! 3 moves the oracle to Rust, so something in Rust has to record. What keeps that from being a
//! self-portrait is that `rerecord` is **gated** and `regenerate` never was — `regenerate` rebuilt
//! every file from whatever the engine said:
//!
//! - **Rust's answer is compared with the committed one**, and the whole run is refused on any
//!   difference the declaration (`--declare`, committed under `spec/declarations/`) does not name:
//!   an added key at a declared path or a moved value at a declared path, and never a removed key
//!   — except that a screen declaration may name removals path by path in `removed` (M34 P10),
//!   because the tie rule's withheld tiles lose a key Python recorded. A change to the engine's
//!   output has to be *named* before it can be recorded.
//! - **The file written is the committed one with only the declared paths replaced**, so every
//!   undeclared value keeps the bits Python recorded — the file stays the record of what Python said,
//!   and a port that drifts inside the tolerance cannot launder the drift into it.
//! - **Each file says what was changed in it**: `provenance.rerecords` gains an entry naming the
//!   declaration and the paths that matched in that file.
//! - **A run that finds nothing writes nothing**, and one refusal anywhere writes nothing anywhere.
//!
//! The declaration and this verb's report are what a reviewer reads, never `git diff`: the text
//! churns where `serde_json` and `json.dumps` spell the same value differently. The rules are
//! `golf_core::rerecord`'s and are unit-tested there; this file parses arguments and prints.
//!
//! **Which vectors a run re-records is the declaration's to say, by its version key** (M34 P7, the
//! M34 plan's call 6): `analysis_version` re-records the engine and stage families, and
//! `screen_parser_version` the screen family, each held to its own constant. There is no flag for
//! it, because a flag that disagreed with the declaration would be a second answer to one question.
//!
//! `--dry-run` prints the report and writes nothing. `--spec` points the verb at another `spec/` —
//! its own tests run on a copy — and defaults to this repository's, found from the source rather than
//! from the working directory, the way `conformance.py` finds `REPO` from `__file__`: a run started
//! from `crates/core/` should re-record the repo's vectors, not refuse to find any.
//!
//! # `parse-screen`
//!
//! The screen reader's seam (M34 P6), `run`'s shape for `crates/screen`: a screen vector, or its
//! bare `input`, on stdin; the `ShotData` that [`screen::read`] makes of it on stdout, or `null`
//! where the read failed, which is `import_screen`'s `failed`. A failed read is an answer, not an
//! error, so it exits 0; a device no profile names is the caller's mistake, and exits 1 with
//! nothing on stdout. Nothing here is Python's: there is no `conformance.py` verb to diff it
//! against, because the screen family is recorded once and then belongs to `golf-core rerecord`
//! (the M34 plan's call 3). It is the way to see what the reader makes of a vector without a test.
//!
//! # Why subcommands
//!
//! Because `conformance.py` has five and a port is asked to match one of them, and spelling `run`
//! explicitly from the start is what let `rerecord` be an addition rather than a redesign. ADR-032 §7
//! names a `golf-core` subprocess seam as a candidate for how ADR-022's fitting scripts reach a
//! measurement once `analysis/measure.py` is gone, which would be a third verb here.

use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use golf_core::rerecord;
use screen::ScreenInput;
use serde::{Deserialize, Serialize};

const USAGE: &str = "usage: golf-core run < vector.json\n       \
                     golf-core rerecord --declare <file> [--dry-run] [--spec <dir>]\n         \
                     (the declaration's version key picks the family: analysis_version for the \
                     engine and stage vectors, screen_parser_version for the screen vectors)\n       \
                     golf-core parse-screen < screen-vector.json";

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("run") if args.next().is_none() => run(),
        Some("parse-screen") if args.next().is_none() => parse_screen(),
        Some("rerecord") => match RerecordArgs::parse(args) {
            Ok(parsed) => rerecord(&parsed),
            Err(problem) => usage(&problem),
        },
        Some(other) => usage(&format!("unknown command {other:?}")),
        None => usage("no command given"),
    }
}

fn usage(problem: &str) -> ExitCode {
    eprintln!("golf-core: {problem}");
    eprintln!("{USAGE}");
    ExitCode::FAILURE
}

/// Read one vector from stdin, run it, write the serialized result to stdout.
///
/// **A read to EOF rather than a line-oriented parse**, because a committed vector is pretty-printed
/// across thousands of lines. And one vector per invocation rather than a stream, because that is the
/// contract `conformance.py run` has and a batching mode nobody asked for would make the two
/// commands' stdin mean different things.
fn run() -> ExitCode {
    let input = match read_stdin::<golf_core::Vector, golf_core::VectorInput>(|vector| vector.input)
    {
        Ok(input) => input,
        Err(failed) => return failed,
    };
    write_stdout(&golf_core::run(&input))
}

/// A whole screen vector, of which only `input` is read. `golf_core::Vector`'s counterpart.
#[derive(Deserialize)]
struct ScreenVector {
    input: ScreenInput,
}

/// Read one screen vector from stdin and write the shot it reads as, or `null`.
fn parse_screen() -> ExitCode {
    let input = match read_stdin::<ScreenVector, ScreenInput>(|vector| vector.input) {
        Ok(input) => input,
        Err(failed) => return failed,
    };
    match screen::read(&input) {
        Ok(shot) => write_stdout(&shot),
        Err(unknown) => {
            eprintln!("golf-core parse-screen: {unknown}");
            ExitCode::FAILURE
        }
    }
}

/// Stdin, read to EOF, as either a whole vector file or its bare `input` object, the way a caller
/// piping a slice of one would expect. `conformance.py run` takes the whole file; the bare form costs
/// one fallback and saves anyone diffing a hand-built input from having to wrap it. Every verb that
/// reads a vector reads it this way, so the two forms mean the same thing on each.
fn read_stdin<W, I>(unwrap: impl FnOnce(W) -> I) -> Result<I, ExitCode>
where
    W: for<'de> Deserialize<'de>,
    I: for<'de> Deserialize<'de>,
{
    let mut text = String::new();
    if let Err(e) = io::stdin().read_to_string(&mut text) {
        eprintln!("golf-core: reading stdin: {e}");
        return Err(ExitCode::FAILURE);
    }
    match serde_json::from_str::<W>(&text) {
        Ok(vector) => Ok(unwrap(vector)),
        Err(whole) => serde_json::from_str::<I>(&text).map_err(|bare| {
            eprintln!("golf-core: stdin is neither a vector nor a vector input");
            eprintln!("  as a vector:       {whole}");
            eprintln!("  as a vector input: {bare}");
            ExitCode::FAILURE
        }),
    }
}

/// Write one answer to stdout, pretty-printed, because the thing on the other side of this pipe is
/// usually a human reading a diff. `compare_results` parses either way.
fn write_stdout(answer: &impl Serialize) -> ExitCode {
    let mut out = io::stdout().lock();
    let written = serde_json::to_writer_pretty(&mut out, answer)
        .map_err(|e| e.to_string())
        .and_then(|()| {
            out.write_all(b"\n")
                .and_then(|()| out.flush())
                .map_err(|e| e.to_string())
        });
    match written {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            // A broken pipe lands here too, which is what `golf-core run < v.json | head` does — so
            // the message names the write rather than claiming the analysis failed.
            eprintln!("golf-core: writing stdout: {e}");
            ExitCode::FAILURE
        }
    }
}

/// `rerecord`'s flags. Hand-parsed: three flags do not earn an argument-parsing dependency, and the
/// refusals below are the whole of what one would add.
struct RerecordArgs {
    declare: PathBuf,
    dry_run: bool,
    spec: PathBuf,
}

impl RerecordArgs {
    fn parse(mut args: impl Iterator<Item = String>) -> Result<Self, String> {
        let (mut declare, mut dry_run, mut spec) = (None, false, None);
        while let Some(flag) = args.next() {
            let mut value = |name: &str| {
                args.next()
                    .map(PathBuf::from)
                    .ok_or_else(|| format!("{name} needs a value"))
            };
            match flag.as_str() {
                "--declare" if declare.is_none() => declare = Some(value("--declare")?),
                "--spec" if spec.is_none() => spec = Some(value("--spec")?),
                "--dry-run" if !dry_run => dry_run = true,
                "--declare" | "--spec" | "--dry-run" => return Err(format!("{flag} given twice")),
                other => return Err(format!("rerecord: unknown argument {other:?}")),
            }
        }
        Ok(Self {
            // Required rather than defaulted to the newest file in `spec/declarations/`: which change
            // is being recorded is the one thing a reviewer must have said out loud.
            declare: declare.ok_or("rerecord needs --declare <file>")?,
            dry_run,
            spec: spec.unwrap_or_else(|| {
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("..")
                    .join("..")
                    .join("spec")
            }),
        })
    }
}

/// Plan the whole run, write it unless `--dry-run`, and print the report — or the refusal, to
/// stderr, having written nothing.
fn rerecord(args: &RerecordArgs) -> ExitCode {
    let planned = match rerecord::plan(&args.spec, &args.declare) {
        Ok(planned) => planned,
        Err(refused) => {
            eprintln!("golf-core rerecord: {refused}");
            return ExitCode::FAILURE;
        }
    };
    let written = if args.dry_run {
        0
    } else {
        match planned.write() {
            Ok(written) => written,
            Err(refused) => {
                eprintln!("golf-core rerecord: {refused}");
                return ExitCode::FAILURE;
            }
        }
    };
    print!("{}", planned.report(args.dry_run, written));
    ExitCode::SUCCESS
}
