//! `scripts/flag_mishit.py`: a golfer's verdict on one shot, and the per-club mishit tally.
//! [M36 P16]
//!
//! The corpus reader flags a carry below half its club's median automatically, once the club has
//! five clean shots (`contracts::mishit`, ADR-028). This verb is for the shots that rule cannot see:
//! a heavier miss the golfer wants gone, or a punch shot the rule flagged that should stay. **One
//! swing at a time**, because a session has many shots and nothing but the golfer's memory says
//! which was a top (ADR-024 §5), so there is no bulk form.
//!
//! [`flag_mishit_list`] is `_list`'s report for one golfer, the text the career family recorded as
//! `flag_mishit_list`. [`flag_mishit`] is the script's `main` after argparse, and [`action`] is the
//! one decision argparse's caller made (`parser.error`'s refusal). The only write is
//! [`SwingBundleStore::set_mishit`], with the clock passed in (the M36 plan's call 4): the verb
//! passes `Timestamp::now_utc()`.
//!
//! No `--json` here, where the read verbs have it: the listing is the bag profile built with no bag,
//! which `golf-core club-profile --json` already prints, and a flag's answer is one line.

use analysis::club_profile::build_bag_profile;
use contracts::club_profile::BagProfile;
use contracts::mishit::MishitVerdict;
use contracts::Timestamp;
use storage::bundle_store::SwingBundleStore;
use storage::corpus::{read_corpus, EngineVersions};
use storage::golfer_store::{slugify, GolferStore};

use super::{Answer, DataDirs, Printed};

/// `_list`'s report for one golfer: how many shots the bag holds out, then each club with a shot,
/// its count, its mishits and every held-out shot's `session/swing`, so a held-out sample is
/// something the golfer can go and look at.
///
/// `bag` is [`build_bag_profile`] over the golfer's corpus **with no bag**, as the script builds it,
/// so a club declared and never hit has no row here (it has no shots to hold out).
pub fn flag_mishit_list(player_id: &str, bag: &BagProfile) -> String {
    let mut out = Printed::default();
    out.line(format!(
        "\n{player_id} -- {} shot(s) held out across the bag",
        bag.mishits_excluded
    ));
    let rows: Vec<_> = bag.clubs.iter().filter(|club| club.n_shots > 0).collect();
    if rows.is_empty() {
        out.line("  (no tagged shots yet)");
        return out.into_string();
    }
    for club in rows {
        let note = if club.mishits != 0 {
            let ruled = club.mishits - club.mishits_unconfirmed;
            format!(
                "{} mishit(s): {ruled} confirmed, {} auto and unconfirmed",
                club.mishits, club.mishits_unconfirmed
            )
        } else {
            "clean".to_string()
        };
        out.line(format!(
            "  {:<6} {:>3} shots   {note}",
            club.club.as_str(),
            club.n_shots
        ));
        for reference in &club.mishit_refs {
            out.line(format!("           {reference}"));
        }
    }
    out.into_string()
}

/// What one `flag-mishit` run does, once its flags are read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MishitAction {
    /// `SESSION/SWING` and one of `--confirm`, `--clear` or `--auto` (`None`, the rule decides).
    Flag {
        reference: String,
        verdict: Option<MishitVerdict>,
    },
    /// `--list`, every golfer or the one `--name` slugs to.
    List { name: Option<String> },
}

/// The script's `main` between argparse and the store: `--list` wins over everything else, a
/// reference included, and otherwise a reference and **exactly one** verdict flag are required.
/// `Err` is `parser.error`'s message, which argparse prints under the usage and exits 2 on.
pub fn action(
    reference: Option<String>,
    confirm: bool,
    clear: bool,
    auto: bool,
    list: bool,
    name: Option<String>,
) -> Result<MishitAction, String> {
    if list {
        return Ok(MishitAction::List { name });
    }
    let chosen: Vec<Option<MishitVerdict>> = [
        (confirm, Some(MishitVerdict::Confirmed)),
        (clear, Some(MishitVerdict::Cleared)),
        (auto, None),
    ]
    .into_iter()
    .filter_map(|(given, verdict)| given.then_some(verdict))
    .collect();
    match (reference, chosen.as_slice()) {
        (Some(reference), [verdict]) => Ok(MishitAction::Flag {
            reference,
            verdict: *verdict,
        }),
        _ => Err(
            "give a SESSION/SWING ref and exactly one of --confirm / --clear / --auto".to_string(),
        ),
    }
}

/// The script's `main` from its store on: flag one swing, or list every golfer's tally. Exit 0 on a
/// flag or a listing; 2 on input it cannot use, printed on stdout as the script prints it; 1 if the
/// manifest cannot be written, where the script would have died on the `OSError`.
///
/// `versions` is the corpus the listing reads under (the verbs pass `{ANALYSIS_VERSION,
/// COMPARABLE_FROM}`), and `now` stamps the manifest's `updated_at`.
pub fn flag_mishit(
    action: &MishitAction,
    dirs: &DataDirs,
    versions: EngineVersions,
    now: Timestamp,
) -> Answer {
    let refused = |stdout: String| Answer {
        stdout,
        stderr: String::new(),
        code: 2,
    };
    match action {
        MishitAction::Flag { reference, verdict } => {
            // `ref.count("/") != 1 or not all(ref.split("/"))`: one slash, and text either side.
            let Some((session_id, swing_id)) =
                reference.split_once('/').filter(|(session, swing)| {
                    !session.is_empty() && !swing.is_empty() && !swing.contains('/')
                })
            else {
                return refused(format!(
                    "'{reference}' is not a SESSION/SWING reference, e.g. 2026-08-10/3\n"
                ));
            };
            let store = SwingBundleStore::new(&dirs.sessions);
            match store.set_mishit(session_id, swing_id, *verdict, now) {
                Ok(Some(_)) => {}
                Ok(None) => return refused(format!("no swing {reference}\n")),
                Err(e) => {
                    return Answer {
                        stdout: String::new(),
                        stderr: format!("golf-core flag-mishit: writing {reference}: {e}\n"),
                        code: 1,
                    }
                }
            }
            let said = verdict.map_or(
                "auto (no verdict -- the rule decides)",
                MishitVerdict::as_str,
            );
            Answer {
                stdout: format!("{reference}: mishit = {said}\n"),
                stderr: String::new(),
                code: 0,
            }
        }
        MishitAction::List { name } => {
            // `slugify(args.name) if args.name else None`: an empty name lists everyone, and a name
            // that slugs to nothing asks for golfer `''`, which no record answers.
            let only = name.as_deref().filter(|name| !name.is_empty()).map(slugify);
            let golfers = GolferStore::new(&dirs.golfers);
            if let Some(only) = &only {
                if golfers.get(only).is_none() {
                    return refused(format!("no golfer '{only}'\n"));
                }
            }
            let ids: Vec<String> = match only.filter(|only| !only.is_empty()) {
                Some(only) => vec![only],
                None => golfers
                    .list_all()
                    .into_iter()
                    .map(|golfer| golfer.player_id)
                    .collect(),
            };
            let mut stdout = String::new();
            for player_id in &ids {
                let corpus = read_corpus(&dirs.sessions, player_id, versions);
                stdout += &flag_mishit_list(player_id, &build_bag_profile(&corpus, None));
            }
            // No trailing blank line: the script's `main` returns `_list`'s code directly, where the
            // career scripts end with a `print()`.
            Answer {
                stdout,
                stderr: String::new(),
                code: 0,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `--list` wins, a reference and one verdict flag make a flag, and anything else is argparse's
    /// refusal: no reference, no verdict, or two verdicts.
    #[test]
    fn the_flags_decide_as_the_scripts_main_does() {
        let reference = || Some("2026-08-10/3".to_string());
        assert_eq!(
            action(reference(), true, true, false, true, Some("Aaron".into())),
            Ok(MishitAction::List {
                name: Some("Aaron".into())
            })
        );
        for (confirm, clear, auto, verdict) in [
            (true, false, false, Some(MishitVerdict::Confirmed)),
            (false, true, false, Some(MishitVerdict::Cleared)),
            (false, false, true, None),
        ] {
            assert_eq!(
                action(reference(), confirm, clear, auto, false, None),
                Ok(MishitAction::Flag {
                    reference: "2026-08-10/3".into(),
                    verdict
                })
            );
        }
        for (reference, confirm, clear) in [
            (None, true, false),
            (reference(), false, false),
            (reference(), true, true),
        ] {
            assert!(action(reference, confirm, clear, false, false, None)
                .is_err_and(|e| e.contains("exactly one of")));
        }
    }
}
