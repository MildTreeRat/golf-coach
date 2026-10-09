//! The career scripts' reports, rendered to a `String`, and the `main` the four read verbs share.
//! [M36 P15–P16]
//!
//! Each submodule ports one script's `_report` and the helpers only it calls:
//! [`corpus`] is `scripts/career_corpus.py`, [`baseline`] is `scripts/career_baseline.py`,
//! [`dispersion`] is `scripts/career_dispersion.py` and [`club_profile`] is
//! `scripts/club_profile.py`. [`flag_mishit`] is `scripts/flag_mishit.py`, its listing and its
//! write, and holds its own `main` because it is the one verb that writes. A renderer returns what
//! its script's `_report` prints,
//! every `print` one line and its `"\n"`, which is exactly what the recorder captured with
//! `redirect_stdout` into each career vector's `expected.reports` (the M36 plan's call 3). The
//! script's `main` adds one trailing `print()` after the last golfer, and that is the verb's, not
//! the renderer's.
//!
//! # Gated twice
//!
//! `crates/core/tests/reports.rs` holds every recorded report of a rendered script to exact string
//! equality, through [`crate::career_family::run_career`], the runner `golf-core rerecord` records
//! with; and M36 P16's parity run diffed every verb against its script over `data/` once (the plan's
//! decision 3), every flag combination the scripts take, stdout and exit code: identical, once
//! Python's Windows console differences were set aside (its text-mode `\r\n`, and cp1252 in a pipe
//! unless `PYTHONIOENCODING=utf-8`). The verbs write UTF-8 and `\n` on every platform. The report
//! text outlives the scripts that way: M29 deletes them, and the career family still says what they
//! printed.
//!
//! # What is CPython's here, and what is std's
//!
//! P3's format scan, reached by these scripts:
//!
//! - `f"{value:.{p}f}"` is [`pyfmt::fixed`], and `-0.0` keeps its sign (`past-the-tables` prints
//!   `-0.0`). `{percentile:.0f}` is the same at 0 places, half-even on an exact half (`standings`
//!   prints 42.5 as `42` and 87.5 as `88`).
//! - `{tolerance:g}`, and `club_profile`'s `{loft_deg:g}` and `{length_in:g}`, are [`pyfmt::g`].
//! - `f"{recorded_at:%Y-%m-%d}"` is `Timestamp::date_ymd`, the date in the stamp's own offset.
//! - Padding (`{name:<{width}}`, `{reason:<14} {count:>3}`, `{claim:<8}`, `{label:<10}`) counts code
//!   points in Python and `char`s in Rust's `{:<N}`, which are the same thing, so std formatting
//!   needs no edge; nor does `flag_mishit`'s `{n_shots:>3}` on an integer. Every padded string is
//!   ASCII today in any case.
//! - The greedy wrap splits on Unicode whitespace ([`pyfmt::split`]: `bag-declared` carries an NBSP
//!   and an EM SPACE that print as single spaces) and measures in code points (an em dash is one).
//! - `swing.face_on_sha256[:12]` slices code points; every hash is ASCII hex.

pub mod baseline;
pub mod club_profile;
pub mod corpus;
pub mod dispersion;
pub mod flag_mishit;

use std::path::{Path, PathBuf};

use serde_json::json;

use analysis::baseline::build_baseline;
use analysis::club_profile::build_bag_profile;
use analysis::comparison::build_standing;
use analysis::dispersion::build_dispersion;
use contracts::club::parse_club;
use storage::bag_store::BagStore;
use storage::corpus::{read_corpus, EngineVersions};
use storage::golfer_store::{slugify, GolferStore};

/// What a report prints, as Python's `print` writes it into a `StringIO`: one line per call, each
/// with its `"\n"`.
#[derive(Debug, Default)]
pub(crate) struct Printed(String);

impl Printed {
    /// `print(text)`.
    pub(crate) fn line(&mut self, text: impl AsRef<str>) {
        self.0.push_str(text.as_ref());
        self.0.push('\n');
    }

    pub(crate) fn into_string(self) -> String {
        self.0
    }
}

/// Every report's first two lines: the golfer, then a rule. The leading blank line is the script's
/// (`print(f"\n{display_name}  ({player_id})")`), so golfers printed one after another are separated
/// by it.
pub(crate) fn header(out: &mut Printed, display_name: &str, player_id: &str) {
    out.line(format!("\n{display_name}  ({player_id})"));
    out.line("-".repeat(72));
}

/// `'' if n == 1 else 's'`.
pub(crate) fn plural(n: i64) -> &'static str {
    if n == 1 {
        ""
    } else {
        "s"
    }
}

/// Unit -> decimal places, the scripts' `_PRECISION`.
///
/// **One table where the scripts have three copies**, and the copies were the scripts' constraint
/// rather than a choice: `scripts/` is not a package and no script imports another
/// (`career_baseline.py`'s comment says so). The three are identical, `club_profile.py`'s spelt as
/// a table plus a `_DEFAULT_PRECISION`, so its renderer takes this one too. Why the physical units
/// print one place: the old rule, `1dp if degrees else 3dp`, printed "152.000 yards", precision the
/// launch monitor does not have.
const PRECISION: [(&str, usize); 6] = [
    ("degrees", 1),
    ("yards", 1),
    ("mph", 1),
    ("ms", 1),
    ("ratio", 3),
    ("shoulder_widths", 3),
];

/// An unlisted unit takes three places, deliberately: too much precision is noise a reader can see,
/// too little is a digit that quietly went missing (`rpm`, `seconds`, `sd_units` and the family's
/// built `furlongs` reach it).
const DEFAULT_PRECISION: usize = 3;

/// The scripts' `_fmt`: `f"{value:.{_PRECISION.get(unit, 3)}f}"`.
pub(crate) fn fmt(value: f64, unit: &str) -> String {
    let places = PRECISION
        .iter()
        .find(|(name, _)| *name == unit)
        .map_or(DEFAULT_PRECISION, |(_, places)| *places);
    pyfmt::fixed(value, places)
}

/// The scripts' `_wrap`: a naive greedy wrap, not `textwrap.fill`, because the indent differs per
/// caller. `None` and an empty string wrap to nothing.
///
/// Every step is CPython's: `text.split()` is [`pyfmt::split`], `len` counts code points, and
/// `f"{current} {word}".strip()` is [`pyfmt::strip`]. A word longer than `width` takes a line of its
/// own rather than being broken, as the script's does.
pub(crate) fn wrap(text: Option<&str>, width: usize) -> Vec<String> {
    let Some(text) = text.filter(|text| !text.is_empty()) else {
        return Vec::new();
    };
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in pyfmt::split(text) {
        if !current.is_empty() && current.chars().count() + 1 + word.chars().count() > width {
            lines.push(std::mem::replace(&mut current, word.to_string()));
        } else {
            current = pyfmt::strip(&format!("{current} {word}")).to_string();
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

// ------------------------------------------------------------------------------- the verbs' `main`

/// Who a career verb reports on, as the scripts' `main` decides it before its loop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Targets {
    /// `(player_id, display_name)` for each golfer to report on, in order.
    Golfers(Vec<(String, String)>),
    /// `--name` slugs to nothing and no `--player-id` was given: the script prints
    /// `'<name>' has no usable characters for an id.` and exits 2.
    NoUsableId(String),
    /// Neither flag, and the registry is empty: the script prints `No golfers registered in <dir>.`
    /// and exits 0, since an empty registry is a finding rather than a failure.
    NoneRegistered,
}

/// The scripts' `main`, up to the loop: one golfer by `--player-id` or by the slug of `--name`, else
/// every golfer in the registry by display name.
///
/// **An empty string is absent**, as Python's `if args.name or args.player_id` and
/// `args.player_id or slugify(args.name)` read it: `--player-id ""` beside `--name Aaron` reports on
/// `aaron`, and both empty lists the registry. An id with no record is still read, displayed as
/// `<id> (not registered)`: swings can name a golfer whose registry record was deleted, and a corpus
/// of zero says that out loud rather than erroring (`career_corpus.py`'s comment).
pub fn targets(golfers: &GolferStore, name: Option<&str>, player_id: Option<&str>) -> Targets {
    let name = name.filter(|name| !name.is_empty());
    let player_id = player_id.filter(|id| !id.is_empty());
    if name.is_none() && player_id.is_none() {
        let all: Vec<(String, String)> = golfers
            .list_all()
            .into_iter()
            .map(|golfer| (golfer.player_id, golfer.display_name))
            .collect();
        return if all.is_empty() {
            Targets::NoneRegistered
        } else {
            Targets::Golfers(all)
        };
    }
    let player_id = match player_id {
        Some(id) => id.to_string(),
        None => slugify(name.unwrap_or_default()),
    };
    if player_id.is_empty() {
        return Targets::NoUsableId(name.unwrap_or_default().to_string());
    }
    let display_name = match golfers.get(&player_id) {
        Some(golfer) => golfer.display_name,
        None => format!("{player_id} (not registered)"),
    };
    Targets::Golfers(vec![(player_id, display_name)])
}

/// The data directories a verb reads (the M36 plan's call 11): `--sessions-dir` and `--golfers-dir`
/// when given, else `GOLF_SESSIONS_DIR` and `GOLF_GOLFERS_DIR`, which are pydantic-settings' names
/// for `settings.sessions_dir` and `settings.golfers_dir`, else `data/processed/{sessions,golfers}`
/// under the repository root, found from the source as `config.py` finds `REPO_ROOT` from
/// `__file__`, so a verb started from anywhere reads the repo's data.
///
/// **`.env` is not read**, where `Settings` reads `REPO_ROOT / ".env"`: it holds no directory
/// override today (an upload token and an API key), and a dotenv parser is a dependency for a file
/// that cannot move these two answers. An empty variable counts as unset here, where pydantic would
/// read it as `Path("")`, the working directory: a directory nobody meant to name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataDirs {
    pub sessions: PathBuf,
    pub golfers: PathBuf,
}

impl DataDirs {
    /// Resolve each directory: the flag, else the environment variable (`env` is asked by name, so a
    /// test can answer for the process environment), else the repository's default.
    pub fn resolve(
        sessions: Option<PathBuf>,
        golfers: Option<PathBuf>,
        env: impl Fn(&str) -> Option<String>,
    ) -> Self {
        let pick = |flag: Option<PathBuf>, var: &str, leaf: &str| {
            flag.or_else(|| env(var).filter(|v| !v.is_empty()).map(PathBuf::from))
                .unwrap_or_else(|| repo_root().join("data").join("processed").join(leaf))
        };
        DataDirs {
            sessions: pick(sessions, "GOLF_SESSIONS_DIR", "sessions"),
            golfers: pick(golfers, "GOLF_GOLFERS_DIR", "golfers"),
        }
    }
}

/// The repository root: two directories above this crate's manifest (`crates/core`).
pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crates/core sits two directories below the repository root")
        .to_path_buf()
}

/// The career verbs that read and report, one per script (the M36 plan's decision 3).
/// `flag-mishit` is not one: it writes, and [`flag_mishit::flag_mishit`] is its `main`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verb {
    /// `golf-core career-corpus`, `scripts/career_corpus.py`.
    CareerCorpus,
    /// `golf-core career-baseline`, `scripts/career_baseline.py`.
    CareerBaseline,
    /// `golf-core career-dispersion`, `scripts/career_dispersion.py`.
    CareerDispersion,
    /// `golf-core club-profile`, `scripts/club_profile.py`, the one verb that takes `--club`.
    ClubProfile,
}

impl Verb {
    pub const ALL: [Verb; 4] = [
        Verb::CareerCorpus,
        Verb::CareerBaseline,
        Verb::CareerDispersion,
        Verb::ClubProfile,
    ];

    /// The subcommand, the script's name with `-` for `_`.
    pub const fn name(self) -> &'static str {
        match self {
            Verb::CareerCorpus => "career-corpus",
            Verb::CareerBaseline => "career-baseline",
            Verb::CareerDispersion => "career-dispersion",
            Verb::ClubProfile => "club-profile",
        }
    }

    pub fn from_name(name: &str) -> Option<Verb> {
        Verb::ALL.into_iter().find(|verb| verb.name() == name)
    }
}

/// One career verb's flags, parsed: the scripts' `--name`, `--player-id` and `--verbose`,
/// `club_profile.py`'s `--club`, and the verbs' own `--json` and data directories.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub name: Option<String>,
    pub player_id: Option<String>,
    pub verbose: bool,
    /// `--club` as typed, in any spelling [`parse_club`] takes, and read by [`Verb::ClubProfile`]
    /// alone. An empty string is absent, as the script's `if args.club` reads it.
    pub club: Option<String>,
    /// Print the aggregate each report was rendered from, as JSON, instead of the report.
    pub json: bool,
    pub dirs: DataDirs,
}

/// What a verb printed and how it exits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Answer {
    pub stdout: String,
    pub stderr: String,
    pub code: u8,
}

/// The scripts' `main`, whole: who to report on ([`targets`]), each golfer's corpus read under
/// `versions` (the verbs pass `{ANALYSIS_VERSION, COMPARABLE_FROM}`), and each report printed, then
/// the one trailing blank line the script's last `print()` writes. Exit 0, since an `n` of zero is a
/// finding rather than a failure; exit 2 for a name with no usable characters, printed on stdout as
/// the script prints it, and for `club-profile`'s `--club` text that is not a club, which the script
/// checks first, before it looks for a golfer.
///
/// `club-profile` reads each golfer's bag from the golfers directory, beside the golfer it belongs
/// to, as `api/app.py` builds its store. An absent or unreadable bag is `None`, which costs the loft
/// and nothing else.
///
/// **`--json`** prints one JSON array instead, an object per golfer the report would have covered:
/// `display_name` and the aggregate the report was rendered from, under the name the career family
/// gives it (`corpus`; `baseline` and `standing`; `dispersion`; `bag_profile`). `--club` narrows the
/// text and not the JSON, because one club's report is cut from the whole bag profile by
/// `BagProfile::profile_for`. Nothing else goes on stdout in that mode, so a refusal's sentence moves
/// to stderr and an empty registry is `[]`.
pub fn answer(verb: Verb, request: &Request, versions: EngineVersions) -> Answer {
    let golfers = GolferStore::new(&request.dirs.golfers);
    let (mut stdout, mut stderr) = (String::new(), String::new());
    let refuse = |text: String| {
        let (stdout, stderr) = if request.json {
            (String::new(), text)
        } else {
            (text, String::new())
        };
        Answer {
            stdout,
            stderr,
            code: 2,
        }
    };
    let club = match request.club.as_deref().filter(|text| !text.is_empty()) {
        Some(text) if verb == Verb::ClubProfile => match parse_club(text) {
            Some(club) => Some(club),
            // Refused rather than nudged toward a nearest match, `parse_club`'s own posture: a retype
            // costs one line, and a wrong club pools a wedge's carries into a 7 iron's where nothing
            // downstream will ever flag it.
            None => {
                return refuse(format!(
                    "'{text}' is not a club. Try '7i', '7 iron', 'driver' or 'pw'.\n'wedge' and \
                     'iron' name a category rather than a club, so neither is accepted.\n"
                ))
            }
        },
        _ => None,
    };
    let list = match targets(
        &golfers,
        request.name.as_deref(),
        request.player_id.as_deref(),
    ) {
        Targets::Golfers(list) => list,
        Targets::NoUsableId(name) => {
            return refuse(format!("'{name}' has no usable characters for an id.\n"))
        }
        Targets::NoneRegistered => {
            let finding = format!(
                "No golfers registered in {}.\n",
                request.dirs.golfers.display()
            );
            if request.json {
                stdout = "[]\n".to_string();
                stderr = finding;
            } else {
                stdout = finding;
            }
            return Answer {
                stdout,
                stderr,
                code: 0,
            };
        }
    };

    let bags = BagStore::new(&request.dirs.golfers);
    let mut documents = Vec::new();
    for (player_id, display_name) in &list {
        let corpus = read_corpus(&request.dirs.sessions, player_id, versions);
        let verbose = request.verbose;
        match verb {
            Verb::CareerCorpus => {
                if request.json {
                    documents.push(json!({"display_name": display_name, "corpus": corpus}));
                } else {
                    stdout +=
                        &corpus::career_corpus(&corpus, display_name, versions.installed, verbose);
                }
            }
            Verb::CareerBaseline => {
                let (built, standing) = (build_baseline(&corpus), build_standing(&corpus));
                if request.json {
                    documents.push(json!({
                        "display_name": display_name,
                        "baseline": built,
                        "standing": standing,
                    }));
                } else {
                    stdout += &baseline::career_baseline(&built, &standing, display_name, verbose);
                }
            }
            Verb::CareerDispersion => {
                let built = build_dispersion(&corpus);
                if request.json {
                    documents.push(json!({"display_name": display_name, "dispersion": built}));
                } else {
                    stdout += &dispersion::career_dispersion(&built, display_name, verbose);
                }
            }
            Verb::ClubProfile => {
                let built = build_bag_profile(&corpus, bags.get(player_id).as_ref());
                if request.json {
                    documents.push(json!({"display_name": display_name, "bag_profile": built}));
                } else {
                    stdout += &club_profile::club_profile(&built, display_name, verbose, club);
                }
            }
        }
    }
    if request.json {
        stdout = serde_json::to_string_pretty(&documents)
            .expect("the aggregates serialize, as the career family's recorder relies on");
    }
    stdout.push('\n');
    Answer {
        stdout,
        stderr,
        code: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The wrap's three branches: a word that fits, one that starts a new line, and one longer than
    /// the width, which takes a line of its own. Width counts code points, so the em dash is one.
    #[test]
    fn the_wrap_is_greedy_and_counts_code_points() {
        assert_eq!(wrap(None, 10), Vec::<String>::new());
        assert_eq!(wrap(Some(""), 10), Vec::<String>::new());
        assert_eq!(wrap(Some("  \u{a0} "), 10), Vec::<String>::new());
        assert_eq!(wrap(Some("aaaa — bbbb"), 11), ["aaaa — bbbb"]);
        assert_eq!(wrap(Some("aaaa — bbbb"), 10), ["aaaa —", "bbbb"]);
        assert_eq!(
            wrap(Some("a\u{2003}abcdefghijklm b"), 5),
            ["a", "abcdefghijklm", "b"]
        );
    }

    #[test]
    fn an_unlisted_unit_takes_three_places() {
        assert_eq!(fmt(152.04, "yards"), "152.0");
        assert_eq!(fmt(0.12345, "shoulder_widths"), "0.123");
        assert_eq!(fmt(2500.0, "rpm"), "2500.000");
        assert_eq!(fmt(-0.02, "degrees"), "-0.0");
    }

    #[test]
    fn a_flag_beats_the_environment_which_beats_the_default() {
        let env = |var: &str| (var == "GOLF_GOLFERS_DIR").then(|| "from-env".to_string());
        let dirs = DataDirs::resolve(Some(PathBuf::from("flag")), None, env);
        assert_eq!(dirs.sessions, PathBuf::from("flag"));
        assert_eq!(dirs.golfers, PathBuf::from("from-env"));
        let dirs = DataDirs::resolve(None, None, |_| Some(String::new()));
        assert_eq!(
            dirs.sessions,
            repo_root().join("data").join("processed").join("sessions")
        );
        assert!(repo_root().join("Cargo.toml").is_file());
    }
}
