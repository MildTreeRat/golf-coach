//! The written coaching paragraph, from Python: `golf-lab analyze` spawns `coach.py`'s JSON entry
//! and folds its answer into `analysis.json`. [M29 P26]
//!
//! **Python's twin is `api/pipeline.py`'s coaching step**, `generate_coaching` in process, and this
//! is the route the M29 plan's D15 chose for it: Rust stays the only writer of the artifact and
//! Python the only caller of the LLM (ADR-035 clause 1). The entry is P17's,
//! `python -m golf_coach.feedback.coach --session S --swing W`, which reads the stored swing through
//! `golf-mcp`'s `get_swing`, so the caller writes `analysis.json` before it asks
//! (`analyze::analyze_swing_dir`). Its contract, from P17's finding 4:
//!
//! - **stdout** is one ASCII JSON object: `{"text", "provenance"}` with exit 0, or `{"note"}` with
//!   exit 2, the note already in the results page's vocabulary (`no written coaching: …`);
//! - **stderr** carries `golf-mcp`'s banner, so it is captured and read only to say why a run that
//!   answered nothing readable failed;
//! - an argparse error is exit 2 with no JSON, so **anything but one of the two shapes with its own
//!   exit code is a failure**, and a failure is a note, as every refusal of Python's is: the swing
//!   keeps its score.
//!
//! The entry refuses on its own when the `llm` extra or the key is missing, before it starts
//! `golf-mcp`, in Python's wording. So the one switch that is the caller's is C5's,
//! [`coaching_enabled`].
//!
//! # The interpreter is the checkout's `.venv`, and only that
//!
//! Where `crates/pose` tries `PATH` before `.venv` and keeps the first that handshakes (ADR-033's
//! second addendum), this entry has no handshake to judge a candidate by, and the `python` on this
//! box's `PATH` is a system 3.13 that cannot import `golf_coach` (measured there). So the entry runs
//! in `.venv` under the repository this crate was built from, the interpreter `CLAUDE.md` names as
//! the real environment, and a checkout without one is a note naming the path looked at. The child
//! runs in the repository root and inherits the environment, so `GOLF_MCP` and `.env`'s key reach
//! it as they reach `scripts/ask_swing.py`.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use contracts::feedback::CoachingProvenance;
use serde_json::Value;

/// `settings.coaching_enabled`'s environment name.
pub const COACHING_ENABLED_ENV: &str = "GOLF_COACHING_ENABLED";

/// The entry, as `python -m` names it.
pub const COACH_MODULE: &str = "golf_coach.feedback.coach";

/// What every failure note ends with, as Python's refusals in `_coaching_client` end.
const UNAFFECTED: &str = "The scores and ranked tips above are unaffected.";

/// C5's switch: `GOLF_COACHING_ENABLED`, read by pydantic's rule for a `bool`, else `config.py`'s
/// default, on. `env` is asked by name, so a test can answer for the process environment.
///
/// Pydantic's rule, measured on frozen Python: `1`, `t`, `true`, `y`, `yes` and `on` are true and
/// `0`, `f`, `false`, `n`, `no` and `off` false, in any case, with no whitespace trimmed; anything
/// else is refused, as `Settings()` refuses it. **An empty variable counts as unset**, the rule
/// `golf-core`'s `DataDirs` keeps, where pydantic would refuse it. **`.env` is not read**, for
/// `DataDirs`' reason: it holds no coaching setting today (an upload token and an API key, and the
/// key is the entry's to read), and a dotenv parser is a dependency for a file that cannot move
/// this answer.
pub fn coaching_enabled(env: impl Fn(&str) -> Option<String>) -> Result<bool, String> {
    let Some(text) = env(COACHING_ENABLED_ENV).filter(|text| !text.is_empty()) else {
        return Ok(true);
    };
    match text.to_ascii_lowercase().as_str() {
        "1" | "t" | "true" | "y" | "yes" | "on" => Ok(true),
        "0" | "f" | "false" | "n" | "no" | "off" => Ok(false),
        _ => Err(format!(
            "{COACHING_ENABLED_ENV}={text:?} is not a bool (use 1 or 0, true or false)"
        )),
    }
}

/// One stored swing to coach, as `golf-mcp` finds it: `{sessions_dir}/{session_id}/{swing_id}`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoachRequest<'a> {
    pub sessions_dir: &'a Path,
    pub shots_dir: &'a Path,
    pub session_id: &'a str,
    pub swing_id: &'a str,
}

/// What the entry answered: a paragraph and who wrote it, or the note saying why there is none.
#[derive(Debug, Clone, PartialEq)]
pub enum CoachAnswer {
    Written {
        text: String,
        provenance: CoachingProvenance,
    },
    Note(String),
}

/// Whatever writes the paragraph. [`Entry`] in a run; a test answers for it, so no case spawns
/// Python or spends a call.
pub trait Coach {
    fn coach(&mut self, request: &CoachRequest<'_>) -> CoachAnswer;
}

/// P17's entry, run in the checkout's `.venv` (the module doc).
#[derive(Debug, Clone)]
pub struct Entry {
    python: PathBuf,
    root: PathBuf,
    calls: u64,
}

impl Entry {
    /// The entry under the repository this crate was built from.
    pub fn configured() -> Self {
        let root = crate::repo_root();
        Self::new(venv_python(&root), root)
    }

    /// The entry run by `python`, in `root`.
    pub fn new(python: impl Into<PathBuf>, root: impl Into<PathBuf>) -> Self {
        Entry {
            python: python.into(),
            root: root.into(),
            calls: 0,
        }
    }

    /// How many times the entry was started, so a test can hold a run with coaching off to none.
    pub fn calls(&self) -> u64 {
        self.calls
    }

    /// The command for one request: the entry's flags, and nothing it would default itself.
    /// `--golf-mcp` and `--model` are left to the entry (`GOLF_MCP`, else the release build, and
    /// `settings.coaching_model`), so the lab and `scripts/ask_swing.py` find both by one rule.
    pub fn command(&self, request: &CoachRequest<'_>) -> Command {
        let mut command = Command::new(&self.python);
        command
            .arg("-m")
            .arg(COACH_MODULE)
            .arg("--session")
            .arg(request.session_id)
            .arg("--swing")
            .arg(request.swing_id)
            .arg("--sessions-dir")
            .arg(request.sessions_dir)
            .arg("--shots-dir")
            .arg(request.shots_dir)
            .current_dir(&self.root)
            .stdin(Stdio::null());
        command
    }
}

impl Coach for Entry {
    fn coach(&mut self, request: &CoachRequest<'_>) -> CoachAnswer {
        if !self.python.is_file() {
            return CoachAnswer::Note(format!(
                "no written coaching: there is no Python interpreter at {}, where the coaching \
                 entry runs (the checkout's .venv). {UNAFFECTED}",
                self.python.display()
            ));
        }
        self.calls += 1;
        match self.command(request).output() {
            Ok(output) => answer_from(output.status.code(), &output.stdout, &output.stderr),
            Err(error) => failed(&format!(
                "{} would not start: {error}",
                self.python.display()
            )),
        }
    }
}

/// `.venv`'s interpreter under `root`, by the layout `crates/pose`'s resolution uses.
pub fn venv_python(root: &Path) -> PathBuf {
    if cfg!(windows) {
        root.join(".venv").join("Scripts").join("python.exe")
    } else {
        root.join(".venv").join("bin").join("python")
    }
}

/// Read one run of the entry: its exit code, stdout and stderr, into an answer (the module doc's
/// rule). Public so the parse is tested on what the entry prints without spawning it.
pub fn answer_from(code: Option<i32>, stdout: &[u8], stderr: &[u8]) -> CoachAnswer {
    let parsed: Option<Value> = std::str::from_utf8(stdout)
        .ok()
        .and_then(|text| serde_json::from_str(text.trim()).ok());
    if let Some(Value::Object(answer)) = &parsed {
        match (code, answer.get("text"), answer.get("note")) {
            (Some(0), Some(Value::String(text)), None) if !text.is_empty() => {
                if let Some(provenance) = answer
                    .get("provenance")
                    .and_then(|value| serde_json::from_value(value.clone()).ok())
                {
                    return CoachAnswer::Written {
                        text: text.clone(),
                        provenance,
                    };
                }
            }
            (Some(2), None, Some(Value::String(note))) => return CoachAnswer::Note(note.clone()),
            _ => {}
        }
    }
    let code = code.map_or_else(|| "no exit code".to_string(), |code| format!("exit {code}"));
    let why = String::from_utf8_lossy(stderr)
        .lines()
        .map(str::trim)
        .rfind(|line| !line.is_empty())
        .map(str::to_string);
    failed(&match (parsed.is_some(), why) {
        (true, Some(why)) => format!("{code}, an answer that is not its JSON; {why}"),
        (true, None) => format!("{code}, an answer that is not its JSON"),
        (false, Some(why)) => format!("{code}: {why}"),
        (false, None) => format!("{code}, with nothing on stdout or stderr"),
    })
}

/// The note for an entry that answered nothing readable, in `brief_through_golf_mcp`'s words for a
/// `golf-mcp` that could not be used.
fn failed(why: &str) -> CoachAnswer {
    CoachAnswer::Note(format!(
        "no written coaching: the coaching entry could not be used ({why}). {UNAFFECTED}"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(value: &'static str) -> impl Fn(&str) -> Option<String> {
        move |name| (name == COACHING_ENABLED_ENV).then(|| value.to_string())
    }

    /// Pydantic's `bool` from a string, measured on frozen Python's `TypeAdapter(bool)`.
    #[test]
    fn the_switch_reads_a_bool_as_pydantic_does() {
        assert_eq!(coaching_enabled(|_| None), Ok(true), "config.py's default");
        assert_eq!(coaching_enabled(env("")), Ok(true), "empty is unset");
        for on in ["1", "t", "TRUE", "Yes", "on", "oN", "y"] {
            assert_eq!(coaching_enabled(env(on)), Ok(true), "{on}");
        }
        for off in ["0", "F", "false", "No", "OFF", "n"] {
            assert_eq!(coaching_enabled(env(off)), Ok(false), "{off}");
        }
        for refused in [" true", "true ", "2", "enabled"] {
            let problem = coaching_enabled(env(refused)).unwrap_err();
            assert!(problem.starts_with(COACHING_ENABLED_ENV), "{problem}");
        }
    }

    /// What P17's entry prints, both shapes, as its `json.dumps` writes them.
    #[test]
    fn each_of_the_two_shapes_is_read_with_its_exit_code() {
        // ASCII, as `json.dumps` writes it: the dash arrives escaped.
        let written = concat!(
            r#"{"text": "Tempo first "#,
            "\x5cu2014",
            r#" take it back longer.", "provenance": "#,
            r#"{"model": "claude-opus-5", "generated_at": "2026-10-22T09:41:07Z", "#,
            r#""input_digest": "0465f6a5"}}"#,
            "\n"
        )
        .as_bytes();
        let CoachAnswer::Written { text, provenance } = answer_from(Some(0), written, b"banner")
        else {
            panic!("a paragraph was not read");
        };
        assert_eq!(text, "Tempo first \u{2014} take it back longer.");
        assert_eq!(provenance.model, "claude-opus-5");
        assert_eq!(provenance.input_digest.as_deref(), Some("0465f6a5"));
        assert_eq!(
            provenance.generated_at,
            contracts::Timestamp::parse("2026-10-22T09:41:07Z").unwrap()
        );

        let note = br#"{"note": "no written coaching: no Anthropic API key is configured"}"#;
        assert_eq!(
            answer_from(Some(2), note, b""),
            CoachAnswer::Note("no written coaching: no Anthropic API key is configured".into())
        );
    }

    /// An argparse error, a traceback, a shape under the other's exit code, a paragraph with no
    /// provenance: each a note saying why, and never a paragraph.
    #[test]
    fn anything_else_is_a_failure_note() {
        let note_of = |answer: CoachAnswer| match answer {
            CoachAnswer::Note(note) => note,
            written => panic!("read as a paragraph: {written:?}"),
        };
        let traceback = b"Traceback (most recent call last):\n  ...\nModuleNotFoundError: x\n\n";
        let note = note_of(answer_from(Some(1), b"", traceback));
        assert_eq!(
            note,
            format!(
                "no written coaching: the coaching entry could not be used (exit 1: \
                 ModuleNotFoundError: x). {UNAFFECTED}"
            )
        );
        let usage = b"usage: python -m golf_coach.feedback.coach [-h] --session SESSION\n";
        assert!(note_of(answer_from(Some(2), b"", usage)).contains("(exit 2: usage:"));
        assert!(
            note_of(answer_from(Some(0), br#"{"note": "x"}"#, b"")).contains("not its JSON"),
            "a note under exit 0"
        );
        assert!(note_of(answer_from(
            Some(2),
            br#"{"text": "x", "provenance": {"model": "m", "generated_at": "2026-10-22T09:41:07Z"}}"#,
            b""
        ))
        .contains("not its JSON"));
        assert!(note_of(answer_from(Some(0), br#"{"text": "x"}"#, b"")).contains("not its JSON"));
        assert!(note_of(answer_from(None, b"", b"")).contains("(no exit code, with nothing"));
    }

    /// The flags P17's entry takes, and none it defaults itself.
    #[test]
    fn the_command_is_the_entrys_flags() {
        let entry = Entry::new("/venv/python", "/repo");
        let request = CoachRequest {
            sessions_dir: Path::new("/data/sessions"),
            shots_dir: Path::new("/data/shots"),
            session_id: "2026-08-10",
            swing_id: "2",
        };
        let command = entry.command(&request);
        let args: Vec<_> = command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            args,
            [
                "-m",
                COACH_MODULE,
                "--session",
                "2026-08-10",
                "--swing",
                "2",
                "--sessions-dir",
                "/data/sessions",
                "--shots-dir",
                "/data/shots"
            ]
        );
        assert_eq!(command.get_current_dir(), Some(Path::new("/repo")));
    }

    #[test]
    fn a_missing_interpreter_is_a_note_and_starts_nothing() {
        let mut entry = Entry::new("/no/such/python", "/repo");
        let request = CoachRequest {
            sessions_dir: Path::new("s"),
            shots_dir: Path::new("t"),
            session_id: "a",
            swing_id: "1",
        };
        let CoachAnswer::Note(note) = entry.coach(&request) else {
            panic!("coached with no interpreter");
        };
        assert!(note.starts_with("no written coaching: there is no Python interpreter at "));
        assert!(note.ends_with(UNAFFECTED), "{note}");
        assert_eq!(entry.calls(), 0);
    }
}
