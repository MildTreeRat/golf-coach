//! `golf-core` — the CLI end of `docs/CONFORMANCE.md` §4's cross-language seam [M22 P6], and from
//! M32 the recorder of the vectors that seam is judged by.
//!
//! ```text
//! golf-core run < vector.json                                      # vector in, result out
//! golf-core rerecord --declare <file> [--dry-run] [--spec <dir>]   # re-record spec/vectors/, gated
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
//!   an added key at a declared path or a moved value at a declared path, and never a removed key.
//!   A change to the engine's output has to be *named* before it can be recorded.
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
//! `--dry-run` prints the report and writes nothing. `--spec` points the verb at another `spec/` —
//! its own tests run on a copy — and defaults to this repository's, found from the source rather than
//! from the working directory, the way `conformance.py` finds `REPO` from `__file__`: a run started
//! from `crates/core/` should re-record the repo's vectors, not refuse to find any.
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

const USAGE: &str = "usage: golf-core run < vector.json\n       \
                     golf-core rerecord --declare <file> [--dry-run] [--spec <dir>]";

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("run") if args.next().is_none() => run(),
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
    let mut text = String::new();
    if let Err(e) = io::stdin().read_to_string(&mut text) {
        eprintln!("golf-core: reading stdin: {e}");
        return ExitCode::FAILURE;
    }

    // Accept either a whole vector file or a bare `input` object, the way a caller piping a slice of
    // one would expect. `conformance.py run` takes the whole file; the bare form costs one fallback
    // and saves anyone diffing a hand-built input from having to wrap it.
    let input = match serde_json::from_str::<golf_core::Vector>(&text) {
        Ok(vector) => vector.input,
        Err(whole) => match serde_json::from_str::<golf_core::VectorInput>(&text) {
            Ok(input) => input,
            Err(bare) => {
                eprintln!("golf-core: stdin is neither a vector nor a vector input");
                eprintln!("  as a vector:       {whole}");
                eprintln!("  as a vector input: {bare}");
                return ExitCode::FAILURE;
            }
        },
    };

    let result = golf_core::run(&input);
    // Pretty-printed, because the thing on the other side of this pipe is usually a human reading a
    // diff. `compare_results` parses either way.
    let mut out = io::stdout().lock();
    let written = serde_json::to_writer_pretty(&mut out, &result)
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
