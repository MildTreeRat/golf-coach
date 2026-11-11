//! `golf-bay-report`: the bay session's verdict from the phone's export, the two checks with a known
//! answer beside it, and what the app's own logs say happened. [M38 P14, P15, P30c]
//!
//! ```text
//! golf-bay-report bars <export.zip|dir> [--bars F] [--source camera|library|scanner|all]
//!                      [--since RFC3339] [--json]
//! golf-bay-report truth <export.zip|dir> --photos F --truth F [--json]
//! golf-bay-report compare <export.zip|dir> --sessions-dir D --player P [--json]
//! golf-bay-report events <export.zip|dir> [--since RFC3339] [--json]
//! golf-bay-report --help
//! ```
//!
//! A shell over `app::bay`, which holds what the numbers mean. `--bars` defaults to the repo's
//! `docs/plans/m38-bay-thresholds.md`, found from where this was built, because this runs on the Mac
//! that built it and never on a phone (C21). It is read at run time, so a limit has no second copy here.
//! `--source` and `--since` choose whose reads are counted, and `app::bay`'s module doc says why each
//! exists, `--since` with its time's offset required, since a time with none is no instant.
//!
//! **A verdict is an answer and not a failure**, so `PASS`, `MISS` and `INVALID` all exit 0, and the
//! caller reads the words. A verb's flags or a file it cannot read exit 2, and a missing or unknown verb
//! exits 1, as `golf-core`'s and `golf-lab`'s do.
//!
//! **`truth` and `compare` hold something to a known answer, so missing it exits 3**: a wrong cell on an
//! unflagged photo or a capture not found, and a difference between the corpora. P28's script runs both
//! under `set -e`, and a compare that exits 0 on a difference is a trap for whoever writes the next one.
//! The words are printed either way, and 3 collides with nothing above.
//!
//! **`events` judges nothing** [M38 P30c]: it prints what the app's own logs say happened (D16), so it
//! exits 0 whenever it can read the export, and 2 when it cannot, as `bars` does.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use app::api::bay::truth_report;
use app::api::record::ReadSource;
use app::bay::{
    bars_report, compare_corpora, events_report, render_bars, render_comparison, render_events,
    Bars, SourceChoice, BARS_FILE,
};
use app::parse_now;

const USAGE: &str = "usage: golf-bay-report bars <export.zip|dir> [--bars F] \
                     [--source camera|library|scanner|all] [--since RFC3339] [--json]
       golf-bay-report truth <export.zip|dir> --photos F --truth F [--json]
       golf-bay-report compare <export.zip|dir> --sessions-dir D --player P [--json]
       golf-bay-report events <export.zip|dir> [--since RFC3339] [--json]";

/// What a verb prints, and whether the thing it held to a known answer held.
struct Answer {
    text: String,
    held: bool,
}

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("bars") => finish("bars", bars(args.collect())),
        Some("truth") => finish("truth", truth(args.collect())),
        Some("compare") => finish("compare", compare(args.collect())),
        Some("events") => finish("events", events(args.collect())),
        Some("--help" | "-h" | "help") => {
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
        Some(other) => usage(&format!("unknown verb '{other}'")),
        None => usage("a verb is required"),
    }
}

fn finish(verb: &str, answer: Result<Answer, String>) -> ExitCode {
    match answer {
        Ok(Answer { text, held }) => {
            print!("{text}");
            if held {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(3)
            }
        }
        Err(refused) => {
            eprintln!("golf-bay-report {verb}: {refused}");
            ExitCode::from(2)
        }
    }
}

fn usage(problem: &str) -> ExitCode {
    eprintln!("golf-bay-report: {problem}");
    eprintln!("{USAGE}");
    ExitCode::FAILURE
}

/// The repo's own bars, from this crate's directory at build time.
fn default_bars() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(BARS_FILE)
}

fn bars(args: Vec<String>) -> Result<Answer, String> {
    let mut export = None;
    let mut bars_path = None;
    let mut source = SourceChoice::Only(ReadSource::Camera);
    let mut since = None;
    let mut json = false;
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--bars" => {
                let path = args.next().ok_or("--bars needs a file")?;
                bars_path = Some(PathBuf::from(path));
            }
            "--source" => {
                let name = args.next().ok_or("--source needs a name")?;
                source = SourceChoice::parse(&name)?;
            }
            "--since" => {
                let time = args.next().ok_or("--since needs a time")?;
                since = Some(parse_now(&time).map_err(|refused| format!("--since: {refused}"))?);
            }
            "--json" => json = true,
            flag if flag.starts_with('-') => return Err(format!("unknown flag '{flag}'")),
            path if export.is_none() => export = Some(PathBuf::from(path)),
            extra => return Err(format!("one export at a time, and '{extra}' is a second")),
        }
    }
    let export = export.ok_or("an export is required, a zip or a directory")?;
    let bars = Bars::read(&bars_path.unwrap_or_else(default_bars))?;
    let report = bars_report(&export, &bars, source, since)?;
    let text = if json {
        pretty(&report)?
    } else {
        format!(
            "golf-bay-report bars: {}\n\n{}",
            export.display(),
            render_bars(&report)
        )
    };
    Ok(Answer { text, held: true })
}

fn pretty<T: serde::Serialize>(value: &T) -> Result<String, String> {
    let text = serde_json::to_string_pretty(value).map_err(|failed| failed.to_string())?;
    Ok(format!("{text}\n"))
}

/// One export, these flags each with a value, and `--json`.
struct Flags {
    export: PathBuf,
    values: Vec<(String, String)>,
    json: bool,
}

impl Flags {
    fn of(args: Vec<String>, names: &[&str]) -> Result<Self, String> {
        let mut export = None;
        let mut values = Vec::new();
        let mut json = false;
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--json" => json = true,
                flag if names.contains(&flag) => {
                    let value = args.next().ok_or(format!("{flag} needs a value"))?;
                    values.push((flag.to_string(), value));
                }
                flag if flag.starts_with('-') => return Err(format!("unknown flag '{flag}'")),
                path if export.is_none() => export = Some(PathBuf::from(path)),
                extra => return Err(format!("one export at a time, and '{extra}' is a second")),
            }
        }
        for name in names {
            if !values.iter().any(|(flag, _)| flag == name) {
                return Err(format!("{name} is required"));
            }
        }
        Ok(Flags {
            export: export.ok_or("an export is required, a zip or a directory")?,
            values,
            json,
        })
    }

    /// The last one given, as `bars`' flags have it. `of` has held each name to being there.
    fn value(&self, name: &str) -> &str {
        self.values
            .iter()
            .rev()
            .find(|(flag, _)| flag == name)
            .map_or("", |(_, value)| value)
    }
}

fn truth(args: Vec<String>) -> Result<Answer, String> {
    let flags = Flags::of(args, &["--photos", "--truth"])?;
    let report = truth_report(
        flags.export.display().to_string(),
        flags.value("--photos").to_string(),
        flags.value("--truth").to_string(),
    )?;
    let text = if flags.json {
        pretty(&report)?
    } else {
        format!(
            "golf-bay-report truth: {}\n\n{}",
            flags.export.display(),
            report.text
        )
    };
    Ok(Answer {
        text,
        held: report.held,
    })
}

fn compare(args: Vec<String>) -> Result<Answer, String> {
    let flags = Flags::of(args, &["--sessions-dir", "--player"])?;
    let comparison = compare_corpora(
        &flags.export,
        Path::new(flags.value("--sessions-dir")),
        flags.value("--player"),
    )?;
    let text = if flags.json {
        pretty(&comparison)?
    } else {
        format!(
            "golf-bay-report compare: {} against {}\n\n{}",
            flags.export.display(),
            flags.value("--sessions-dir"),
            render_comparison(&comparison)
        )
    };
    Ok(Answer {
        text,
        held: comparison.difference.is_none(),
    })
}

/// What the logs say, over the lines at or after `--since`, read as `bars` reads its own. [M38 P30c]
fn events(args: Vec<String>) -> Result<Answer, String> {
    let mut export = None;
    let mut since = None;
    let mut json = false;
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--since" => {
                let time = args.next().ok_or("--since needs a time")?;
                since = Some(parse_now(&time).map_err(|refused| format!("--since: {refused}"))?);
            }
            "--json" => json = true,
            flag if flag.starts_with('-') => return Err(format!("unknown flag '{flag}'")),
            path if export.is_none() => export = Some(PathBuf::from(path)),
            extra => return Err(format!("one export at a time, and '{extra}' is a second")),
        }
    }
    let export = export.ok_or("an export is required, a zip or a directory")?;
    let report = events_report(&export, since)?;
    let text = if json {
        pretty(&report)?
    } else {
        format!(
            "golf-bay-report events: {}\n\n{}",
            export.display(),
            render_events(&report)
        )
    };
    Ok(Answer { text, held: true })
}
