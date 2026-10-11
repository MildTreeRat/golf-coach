//! `session.json` — the session's cursors: who is swinging, with what, and practising what.
//! `storage/session_meta.py`, plus the intent cursor it does not have. [M35 P7]
//!
//! **These are cursors, not records.** They say who the *next* swing is attributed to, which club
//! it is tagged with and, since M35, what it is practised as. What actually happened lives on each
//! [`crate::manifest::SwingManifest`], stamped when the swing is created and never rewritten from
//! here. Keeping the two apart is why a buddy can take a few swings mid-session without rewriting
//! the history of the golfer hitting before them, why reaching for the next club does not retag the
//! shots hit with the last, and why switching to a drill does not turn the shots already hit into
//! drills.
//!
//! Server-side, not per phone, because two phones upload into the *same* swing: a name typed on
//! two keyboards is the bundle store's "two people cannot be trusted to type matching swing
//! numbers" with worse consequences, since a mismatch splits one golfer's baseline in two. ADR-024
//! §5 reads the club from here on the same argument, and the intent follows it: a drill declared on
//! one phone and not the other would land in the manifest as whichever phone was faster.
//!
//! A sidecar for the reason `analysis.state.json` is one: it records something a human chose, next
//! to but not inside the manifest that records what arrived from a camera.
//!
//! # The intent cursor, and the frozen reader that drops it
//!
//! `intent` is **written only when set**, so a session file with no intent cursor is written with
//! the keys frozen Python's is, and `session_meta.schema.json` is Rust's from here
//! (`conformance.py::RUST_OWNED_SCHEMAS`). pydantic's `SessionMeta` ignores the key it does not
//! declare and its `_update_cursor` writes the record back without it, so **picking a golfer or a
//! club on the frozen server clears a drill cursor** Rust set (the M35 plan's finding 3), and the
//! next shot is tracked. Named, not fixed: M29 replaces the server, and until then the cursor is
//! safe only on a session the frozen server no longer writes to.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use contracts::club::ClubId;
use contracts::intent::PracticeGoal;
use contracts::Timestamp;
use serde::{Deserialize, Serialize};

/// `SESSION_META_NAME`.
pub const SESSION_META_NAME: &str = "session.json";

/// Session-scoped choices: the golfer cursor, the club cursor and the intent cursor.
///
/// The three are independent — a golfer swings many clubs, a club is swung by many golfers, and
/// either can be drilled — and keeping them independent is what `update_cursor` exists to
/// guarantee.
///
/// `updated_at` is **one timestamp for the whole record**, not one per cursor: it says when the
/// session's choices last changed, and nothing reads it. So it cannot answer "when was the club
/// chosen", and a surface wanting that needs a field of its own.
///
/// Fields in pydantic's declaration order, then `intent`, which frozen Python does not declare.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SessionMeta {
    #[serde(default)]
    pub player_id: Option<String>,
    #[serde(default)]
    pub club: Option<ClubId>,
    #[serde(default)]
    pub updated_at: Option<Timestamp>,
    /// The goal the next swing created in this session is stamped with (the M35 plan's decision
    /// 2). `None` means none was chosen, and a swing created then records none, which reads as
    /// `fundamentals`. Written only when set (the module doc says why).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intent: Option<PracticeGoal>,
}

pub fn session_meta_path(session_dir: &Path) -> PathBuf {
    session_dir.join(SESSION_META_NAME)
}

/// The session's cursors, or an empty set of them.
///
/// [`SessionMeta::default`] rather than `None` for a missing or unreadable file: "nothing selected
/// yet" is every session's state until someone picks, so a caller should not have to tell it from
/// "no file". Tolerant as [`crate::manifest::load_manifest`] is, and on the same files (the crate
/// doc's list): a club or mode no enum names, or a `null` where a goal's field is not optional,
/// empties **every** cursor, as pydantic's `ValidationError` does.
///
/// Tolerant does **not** soften the club's requiredness: this answers `club: None`, and refusing an
/// upload without one is the caller's job, not the reader's.
pub fn load_session_meta(session_dir: &Path) -> SessionMeta {
    fs::read(session_meta_path(session_dir))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

/// Write atomically, creating the session directory: the status poll reads this while the golfer
/// form rewrites it.
pub fn save_session_meta(meta: &SessionMeta, session_dir: &Path) -> io::Result<()> {
    fs::create_dir_all(session_dir)?;
    crate::write_atomically(meta, &session_meta_path(session_dir))
}

/// `_update_cursor`: change one cursor, keep every field not changed, stamp `updated_at`, persist.
///
/// Load-modify-save in one place, because the alternative is the bug Python's was written to fix.
/// Until M9 there was one cursor, so a setter could build a fresh record and lose nothing; with a
/// second, that same line cleared the club every time someone picked a golfer. Here the edit is a
/// closure over the record as loaded, so a field it does not name is carried by construction, and
/// the third cursor got that without asking — which is the property Python's merged dump buys.
fn update_cursor(
    session_dir: &Path,
    now: Timestamp,
    edit: impl FnOnce(&mut SessionMeta),
) -> io::Result<SessionMeta> {
    let mut meta = load_session_meta(session_dir);
    edit(&mut meta);
    meta.updated_at = Some(now);
    save_session_meta(&meta, session_dir)?;
    Ok(meta)
}

/// Point the golfer cursor at a golfer, or clear it, and persist. Answers the stored record.
pub fn set_current_player(
    session_dir: &Path,
    player_id: Option<&str>,
    now: Timestamp,
) -> io::Result<SessionMeta> {
    update_cursor(session_dir, now, |meta| {
        meta.player_id = player_id.map(str::to_string);
    })
}

/// Point the club cursor at a club, or clear it, and persist. Answers the stored record.
///
/// No backfill counterpart, deliberately: the bundle store's `attribute_unlabeled` reaches back
/// over a session's unlabeled swings because a session usually has one golfer, and a session has
/// many clubs, so the same reach would mislabel every earlier swing (ADR-024 §5). The club's repair
/// is per swing.
pub fn set_current_club(
    session_dir: &Path,
    club: Option<ClubId>,
    now: Timestamp,
) -> io::Result<SessionMeta> {
    update_cursor(session_dir, now, |meta| meta.club = club)
}

/// Point the intent cursor at a goal, or clear it, and persist. Answers the stored record. [M35 P7]
///
/// No backfill either, for the club's reason: a session mixes drills and tracked shots, so only the
/// golfer can say which earlier swing was which, and
/// [`crate::bundle_store::SwingBundleStore::set_intent`] is that per-shot repair. What a goal may
/// hold (a shape only with `shot_shaping`, and `shot_shaping` only with a shape) is the verb's to
/// refuse, as an upload without a club is the route's: the store records what it is given.
pub fn set_current_intent(
    session_dir: &Path,
    intent: Option<PracticeGoal>,
    now: Timestamp,
) -> io::Result<SessionMeta> {
    update_cursor(session_dir, now, |meta| meta.intent = intent)
}

/// `tests/storage/test_session_meta.py`, case for case, then the intent cursor's own.
#[cfg(test)]
mod tests {
    use super::*;
    use contracts::intent::{PracticeMode, TargetShape};

    fn now() -> Timestamp {
        "2026-08-12T12:00:00Z".parse().unwrap()
    }

    fn later() -> Timestamp {
        "2026-08-12T12:05:00.250000Z".parse().unwrap()
    }

    fn drill() -> PracticeGoal {
        PracticeGoal {
            mode: PracticeMode::Drill,
            ..PracticeGoal::default()
        }
    }

    fn session(dir: &tempfile::TempDir) -> PathBuf {
        dir.path().join("2026-08-12")
    }

    // ----------------------------------------------------------------------- tolerant reads

    /// "Nobody selected yet" is every session's starting state, not an error.
    #[test]
    fn a_missing_session_meta_reads_as_nothing_selected() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(load_session_meta(dir.path()), SessionMeta::default());
    }

    #[test]
    fn a_corrupt_session_meta_reads_as_nothing_selected() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(session_meta_path(dir.path()), "{not json").unwrap();
        assert_eq!(load_session_meta(dir.path()), SessionMeta::default());
    }

    /// Written as a literal, for the reason the manifest's twin of this test states: the bytes such
    /// a file holds, not a struct that carries today's fields.
    #[test]
    fn a_session_written_before_club_existed_still_loads() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            session_meta_path(dir.path()),
            r#"{"player_id": "aaron", "updated_at": "2026-08-12T12:00:00Z"}"#,
        )
        .unwrap();
        let meta = load_session_meta(dir.path());
        assert_eq!(meta.player_id.as_deref(), Some("aaron"));
        assert_eq!((meta.club, meta.intent), (None, None));
    }

    /// What empties every cursor is what empties them in Python, an unreadable intent included,
    /// and an unknown key does not.
    #[test]
    fn a_value_no_enum_names_empties_every_cursor_and_an_unknown_key_does_not() {
        let dir = tempfile::tempdir().unwrap();
        let read = |text: &str| {
            fs::write(session_meta_path(dir.path()), text).unwrap();
            load_session_meta(dir.path())
        };
        assert_eq!(
            read(r#"{"player_id": "aaron", "club": "10i"}"#),
            SessionMeta::default()
        );
        assert_eq!(
            read(r#"{"player_id": "aaron", "intent": {"mode": "warm_up"}}"#),
            SessionMeta::default()
        );
        assert_eq!(read("[]"), SessionMeta::default());
        let kept = read(r#"{"player_id": "aaron", "future_cursor": "x", "intent": null}"#);
        assert_eq!(kept.player_id.as_deref(), Some("aaron"));
        assert_eq!(kept.intent, None);
    }

    // ----------------------------------------------------------------------- golfer cursor

    #[test]
    fn the_golfer_cursor_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let stored = set_current_player(&session(&dir), Some("aaron"), now()).unwrap();
        assert_eq!(stored.player_id.as_deref(), Some("aaron"));
        assert_eq!(
            load_session_meta(&session(&dir)).player_id.as_deref(),
            Some("aaron")
        );
    }

    #[test]
    fn the_golfer_cursor_can_be_repointed() {
        let dir = tempfile::tempdir().unwrap();
        set_current_player(&session(&dir), Some("aaron"), now()).unwrap();
        set_current_player(&session(&dir), Some("dave"), later()).unwrap();
        assert_eq!(
            load_session_meta(&session(&dir)).player_id.as_deref(),
            Some("dave")
        );
    }

    #[test]
    fn saving_meta_creates_the_session_directory_and_leaves_no_tmp() {
        let dir = tempfile::tempdir().unwrap();
        let meta = SessionMeta {
            player_id: Some("aaron".to_string()),
            ..SessionMeta::default()
        };
        save_session_meta(&meta, &session(&dir)).unwrap();
        let path = session_meta_path(&session(&dir));
        assert!(path.exists());
        assert!(!path.with_extension("tmp").exists());
    }

    // ----------------------------------------------------------------------- club cursor

    #[test]
    fn the_club_cursor_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let stored = set_current_club(&session(&dir), Some(ClubId::SevenIron), now()).unwrap();
        assert_eq!(stored.club, Some(ClubId::SevenIron));
        assert_eq!(
            load_session_meta(&session(&dir)).club,
            Some(ClubId::SevenIron)
        );
    }

    /// The ordinary bay motion: hit some 7 irons, then reach for the wedge.
    #[test]
    fn the_club_cursor_can_be_repointed() {
        let dir = tempfile::tempdir().unwrap();
        set_current_club(&session(&dir), Some(ClubId::SevenIron), now()).unwrap();
        set_current_club(&session(&dir), Some(ClubId::SandWedge), later()).unwrap();
        assert_eq!(
            load_session_meta(&session(&dir)).club,
            Some(ClubId::SandWedge)
        );
    }

    // ----------------------------------------------------------------------- intent cursor

    #[test]
    fn the_intent_cursor_round_trips_and_can_be_repointed() {
        let dir = tempfile::tempdir().unwrap();
        let stored = set_current_intent(&session(&dir), Some(drill()), now()).unwrap();
        assert_eq!(stored.intent, Some(drill()));
        assert_eq!(load_session_meta(&session(&dir)).intent, Some(drill()));

        let fade = PracticeGoal {
            mode: PracticeMode::ShotShaping,
            target_shape: Some(TargetShape::Fade),
            ..PracticeGoal::default()
        };
        set_current_intent(&session(&dir), Some(fade.clone()), later()).unwrap();
        assert_eq!(load_session_meta(&session(&dir)).intent, Some(fade));
    }

    // ----------------------------------------------------------------------- independence

    /// The regression `_update_cursor` exists to prevent, from every side: setting one cursor used
    /// to replace the whole record. Written out per pair rather than folded into one loop's
    /// assertion, because a hand-rolled setter breaks exactly one direction of this.
    #[test]
    fn setting_any_cursor_preserves_the_other_two() {
        let dir = tempfile::tempdir().unwrap();
        let path = session(&dir);
        set_current_player(&path, Some("aaron"), now()).unwrap();
        set_current_club(&path, Some(ClubId::SevenIron), now()).unwrap();
        set_current_intent(&path, Some(drill()), now()).unwrap();
        let all = load_session_meta(&path);
        assert_eq!(
            (all.player_id.as_deref(), all.club, all.intent.clone()),
            (Some("aaron"), Some(ClubId::SevenIron), Some(drill()))
        );

        set_current_club(&path, Some(ClubId::PitchingWedge), later()).unwrap();
        let after_club = load_session_meta(&path);
        assert_eq!(after_club.player_id.as_deref(), Some("aaron"));
        assert_eq!(after_club.intent, Some(drill()));

        set_current_player(&path, Some("dave"), later()).unwrap();
        let after_player = load_session_meta(&path);
        assert_eq!(after_player.club, Some(ClubId::PitchingWedge));
        assert_eq!(after_player.intent, Some(drill()));

        set_current_intent(&path, None, later()).unwrap();
        let after_intent = load_session_meta(&path);
        assert_eq!(after_intent.player_id.as_deref(), Some("dave"));
        assert_eq!(after_intent.club, Some(ClubId::PitchingWedge));
    }

    /// Clearing is a repoint at `None`, not a reset: passing `None` must not widen its blast.
    #[test]
    fn clearing_one_cursor_leaves_the_others_alone() {
        type Clear = fn(&Path) -> io::Result<SessionMeta>;
        let clears: [(Clear, &str); 3] = [
            (|d| set_current_club(d, None, later()), "club"),
            (|d| set_current_player(d, None, later()), "player_id"),
            (|d| set_current_intent(d, None, later()), "intent"),
        ];
        for (clear, cleared) in clears {
            let dir = tempfile::tempdir().unwrap();
            let path = session(&dir);
            set_current_player(&path, Some("aaron"), now()).unwrap();
            set_current_club(&path, Some(ClubId::SevenIron), now()).unwrap();
            set_current_intent(&path, Some(drill()), now()).unwrap();

            clear(&path).unwrap();

            let meta = load_session_meta(&path);
            let set = [
                ("player_id", meta.player_id.is_some()),
                ("club", meta.club.is_some()),
                ("intent", meta.intent.is_some()),
            ];
            for (field, is_set) in set {
                assert_eq!(is_set, field != cleared, "clearing {cleared}: {field}");
            }
        }
    }

    /// Reads go through the same loader that writes, so the file itself is worth one assertion,
    /// and it is also where "written only when set" shows: no cursor is spelled `"intent": null`.
    #[test]
    fn the_cursors_reach_disk_and_an_unset_intent_is_not_written() {
        let dir = tempfile::tempdir().unwrap();
        let path = session(&dir);
        set_current_player(&path, Some("aaron"), now()).unwrap();
        set_current_club(&path, Some(ClubId::SevenIron), now()).unwrap();
        let written = fs::read_to_string(session_meta_path(&path)).unwrap();
        assert!(written.contains("\"aaron\""));
        assert!(written.contains("\"7i\""));
        assert!(!written.contains("intent"), "{written}");

        set_current_intent(&path, Some(drill()), now()).unwrap();
        let written = fs::read_to_string(session_meta_path(&path)).unwrap();
        assert!(written.contains("\"drill\""), "{written}");
    }

    #[test]
    fn setting_a_cursor_stamps_updated_at() {
        let dir = tempfile::tempdir().unwrap();
        let stored = set_current_club(&session(&dir), Some(ClubId::SevenIron), later()).unwrap();
        assert_eq!(stored.updated_at, Some(later()));
        assert_eq!(load_session_meta(&session(&dir)).updated_at, Some(later()));
    }

    /// A session file frozen Python wrote re-saves to the value it was read from, so a cursor Rust
    /// moves leaves the shape frozen Python reads unchanged but for the cursor itself.
    #[test]
    fn a_frozen_session_file_re_saves_to_the_value_it_was_read_from() {
        let dir = tempfile::tempdir().unwrap();
        let frozen =
            r#"{"player_id": "aaron", "club": "3w", "updated_at": "2026-08-23T04:49:18.558642Z"}"#;
        fs::write(session_meta_path(dir.path()), frozen).unwrap();
        let read = load_session_meta(dir.path());
        save_session_meta(&read, dir.path()).unwrap();
        let written: serde_json::Value =
            serde_json::from_slice(&fs::read(session_meta_path(dir.path())).unwrap()).unwrap();
        assert_eq!(
            written,
            serde_json::from_str::<serde_json::Value>(frozen).unwrap()
        );
    }
}
