//! The swing bundle store — groups uploaded files into swings, keyed by role.
//! `storage/bundle_store.py`. [M36 P9]
//!
//! **Swing identity is assigned by the store, not the uploader**: two people holding two phones
//! cannot be trusted to type matching swing numbers. Each upload declares only its role; the store
//! slots it into the newest swing in the session lacking that role, opening a new one if none does.
//! Content-addressed dedupe means a retried or double-tapped upload never creates a phantom swing.
//!
//! The documented limitation, ported with the rule: if two swings are missing the same role at once,
//! "newest wins" can attribute an out-of-order upload to the wrong swing. It is not engineered around
//! here. The repair is an explicit target — [`Upload::swing_id`], which names the swing outright and
//! opens it if it is absent — and that target has been in frozen Python since 2026-08-07, so it is
//! ported and recorded like the rest (the M36 plan's finding 1). **M35 and the phone should always
//! pass it**: the default misattaches in a mixed session, and only the caller knows which swing it
//! means.
//!
//! **Golfer attribution follows the same shape.** `player_id` is stamped from the session cursor as
//! files land, never sent by the uploading phone, and stamping is **write-once**: a swing that
//! already names a golfer is never re-attributed by anything here, so switching the cursor
//! mid-session touches only swings still unlabeled. [`SwingBundleStore::set_player`] is the repair.
//! **The club stamps by the same rule**, with two differences: it is required at the upload boundary
//! where the golfer is not (an untagged golfer is repairable later and an untagged club is not), and
//! it has **no backfill**: [`SwingBundleStore::attribute_unlabeled`] may reach backwards over a
//! session because a session usually has one golfer, whereas a session has many clubs and the same
//! reach would confidently mislabel every earlier swing. Both asymmetries are ADR-024 §5's.
//!
//! Flat files, one `manifest.json` per swing: no shared index, no read-modify-write across swings.
//!
//! # A manifest is trusted over its directory
//!
//! As in Python, and each a recorded case (the storage family's `manifest-names-another-swing` and
//! `assign-over-corrupt`): a file is placed in `<call's session>/<manifest.swing_id>`, an answer
//! names the manifest's session except a dedupe's, which names the call's; the next swing number is
//! one past the highest numeric **manifest** id; and `attribute_unlabeled` saves each manifest to
//! the directory its own `swing_id` names, so it can write one directory twice. A corrupt manifest is
//! invisible, so its swing number is reused and its directory written over.
//!
//! # What differs from frozen Python, and why none of it is reachable from a store-named swing
//!
//! - **A swing number is ASCII digits.** Python's `str.isdigit` is Unicode-aware (798 non-ASCII code
//!   points, M36 P4's finding 8) and `int()` reads the decimal ones, so a hand-made directory `٣`
//!   sorts as 3 there and among the non-numeric names here; a superscript `²` passes `isdigit`,
//!   fails `int()`, and crashes Python's `get_session`, where here it is a non-numeric name. The store
//!   names every swing `str(int)`, which is ASCII, so only a hand-made directory reaches either.
//!   Arbitrarily long numbers are compared and incremented exactly, as Python's `int` does.
//! - **Two directories the sort key ties** (`7` and `007`) are ordered by name here, where Python
//!   leaves them in the filesystem's listing order; the crate doc's "a listing sorts by code point".
//! - **A listing that fails is empty.** Python's `exists()` guard lets `iterdir` raise
//!   `NotADirectoryError` when a session id names a file; this reads it as a session with no swings,
//!   which is what the guard was for.
//! - **`delete_swing` on a symbolic link** removes the link here, where `shutil.rmtree` refuses it.
//!
//! # The lock
//!
//! Python holds a `threading.Lock` per store across every write, so two uploads landing at once
//! cannot both open swing 3. Here it is a [`Mutex`] per store, taken by the same methods. Like
//! Python's it serializes callers sharing one store, not two processes sharing a directory. A panic
//! while it is held poisons it; the next caller takes it anyway, because what it guards is on disk
//! and every write there is atomic.

use std::cmp::Ordering;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};

use contracts::club::ClubId;
use contracts::mishit::MishitVerdict;
use contracts::Timestamp;
use serde::Serialize;

use crate::manifest::{
    content_filename, load_manifest, manifest_path, save_manifest, BundleStatus, Role, RoleFile,
    SwingManifest,
};

/// What one upload's assignment answered: where the file went and what the swing still lacks.
///
/// Fields in Python's dataclass order, which is the order the storage family records them in.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AssignmentResult {
    pub session_id: String,
    pub swing_id: String,
    pub role: Role,
    pub status: BundleStatus,
    pub missing_roles: Vec<Role>,
    pub deduped: bool,
    pub player_id: Option<String>,
    pub club: Option<ClubId>,
}

/// One file already streamed to disk, and what the uploader said about it: `assign_from_path`'s
/// keyword arguments.
#[derive(Debug, Clone, Copy)]
pub struct Upload<'a> {
    pub session_id: &'a str,
    pub role: Role,
    /// Where the bytes are now. Renamed into the swing, or removed when the upload is a duplicate.
    pub tmp_path: &'a Path,
    /// The bytes' SHA-256, which the caller computed while streaming them; the store does not hash.
    pub digest: &'a str,
    pub original_filename: &'a str,
    pub content_type: &'a str,
    pub size_bytes: i64,
    /// The explicit target: this swing, created if absent, never deduped. `None` is the automatic
    /// rule.
    pub swing_id: Option<&'a str>,
    /// The session cursor's golfer, stamped only onto a swing that names none.
    pub player_id: Option<&'a str>,
    /// The session cursor's club, stamped only onto a swing that has none.
    pub club: Option<ClubId>,
}

/// Sessions of swings, each swing a directory of role-tagged files and a manifest.
#[derive(Debug)]
pub struct SwingBundleStore {
    root: PathBuf,
    lock: Mutex<()>,
}

impl SwingBundleStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        SwingBundleStore {
            root: root.into(),
            lock: Mutex::new(()),
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn lock(&self) -> MutexGuard<'_, ()> {
        self.lock.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The session an upload at `now` belongs to: its date, in `now`'s **own** offset.
    ///
    /// Server-side, never client-supplied, so it is immune to phone clock skew. A caller passes
    /// [`Timestamp::now_utc`], as Python reads `datetime.now(tz=UTC)`; given `00:30+05:30` it is that
    /// day, though UTC says the day before (the storage family's `reads`).
    pub fn current_session_id(&self, now: Timestamp) -> String {
        now.date_ymd()
    }

    /// Every session on disk, oldest first. Dotted directories (`.incoming`) and files are not
    /// sessions.
    ///
    /// Session ids are dates, so a sort by code point is a chronological one, which is what lets a
    /// cross-session reader walk history in order without parsing the id. `2026-08-07-aaron1` sorts
    /// before `2026-08-09` on the same rule.
    pub fn list_session_ids(&self) -> Vec<String> {
        let Ok(entries) = fs::read_dir(&self.root) else {
            return Vec::new();
        };
        let mut ids: Vec<String> = entries
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let name = entry.file_name().into_string().ok()?;
                (!name.starts_with('.') && entry.path().is_dir()).then_some(name)
            })
            .collect();
        ids.sort();
        ids
    }

    /// Every swing in the session, oldest first: numeric directories by number, then the rest by
    /// name. Dotted entries, files, and directories whose manifest is missing or corrupt are skipped.
    pub fn get_session(&self, session_id: &str) -> Vec<SwingManifest> {
        let session_dir = self.root.join(session_id);
        let Ok(entries) = fs::read_dir(&session_dir) else {
            return Vec::new();
        };
        let mut names: Vec<String> = entries
            .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
            .collect();
        names.sort_by(|a, b| swing_order(a, b));
        names
            .iter()
            .filter(|name| !name.starts_with('.'))
            .map(|name| session_dir.join(name))
            .filter(|swing_dir| swing_dir.is_dir())
            .filter_map(|swing_dir| load_manifest(&manifest_path(&swing_dir)))
            .collect()
    }

    /// One swing's manifest, read from `<session>/<swing>/manifest.json`; `None` when it is missing
    /// or corrupt.
    pub fn get_swing(&self, session_id: &str, swing_id: &str) -> Option<SwingManifest> {
        load_manifest(&manifest_path(&self.swing_dir(session_id, swing_id)))
    }

    /// Slot an already-streamed-to-disk file into a swing, under the store's lock.
    ///
    /// In order: an explicit [`Upload::swing_id`] names the swing (its manifest, or a new one) and
    /// the file is placed there, the same bytes again included; otherwise **the same bytes in the
    /// same role anywhere in the session are a duplicate** — the scratch file is removed, nothing is
    /// stamped, and the answer reports the stored golfer and club, not the requested ones; otherwise
    /// the newest swing lacking the role takes it; otherwise a new swing opens, numbered one past
    /// the highest numeric swing id.
    pub fn assign_from_path(
        &self,
        upload: &Upload<'_>,
        now: Timestamp,
    ) -> io::Result<AssignmentResult> {
        let _held = self.lock();
        let session_dir = self.root.join(upload.session_id);
        let manifests = self.get_session(upload.session_id);

        if let Some(swing_id) = upload.swing_id {
            let target = self
                .get_swing(upload.session_id, swing_id)
                .unwrap_or_else(|| new_manifest(upload, swing_id, now));
            return self.place(&session_dir, target, upload, now);
        }

        for manifest in &manifests {
            let duplicate = manifest
                .roles
                .get(&upload.role)
                .is_some_and(|existing| existing.content_sha256 == upload.digest);
            if duplicate {
                remove_if_present(upload.tmp_path)?;
                return Ok(AssignmentResult {
                    session_id: upload.session_id.to_string(),
                    swing_id: manifest.swing_id.clone(),
                    role: upload.role,
                    status: manifest.status(),
                    missing_roles: manifest.missing_roles(),
                    deduped: true,
                    player_id: manifest.player_id.clone(),
                    club: manifest.club,
                });
            }
        }

        let candidate = manifests
            .iter()
            .rev()
            .find(|manifest| !manifest.roles.contains_key(&upload.role))
            .cloned()
            .unwrap_or_else(|| new_manifest(upload, &next_swing_id(&manifests), now));
        self.place(&session_dir, candidate, upload, now)
    }

    /// Stamp every still-unlabeled swing in the session with `player_id`, and answer the swing ids
    /// changed, in session order.
    ///
    /// What makes "never block uploads" safe: files land whether or not anyone has picked a golfer
    /// yet, so picking one has to reach backwards over the swings that already arrived, or the cost
    /// of forgetting is a permanently anonymous swing. It reaches over *unlabeled* swings only: one
    /// that already names someone is another golfer's. It touches no club (the module doc).
    pub fn attribute_unlabeled(
        &self,
        session_id: &str,
        player_id: &str,
        now: Timestamp,
    ) -> io::Result<Vec<String>> {
        let _held = self.lock();
        let mut changed = Vec::new();
        for mut manifest in self.get_session(session_id) {
            if manifest.player_id.is_some() {
                continue;
            }
            manifest.player_id = Some(player_id.to_string());
            manifest.updated_at = now;
            let swing_dir = self.swing_dir(session_id, &manifest.swing_id);
            save_manifest(&manifest, &manifest_path(&swing_dir))?;
            changed.push(manifest.swing_id);
        }
        Ok(changed)
    }

    /// Re-attribute one swing, overwriting whatever it said: the repair path, and the only thing
    /// here that overwrites a `player_id`. `None` if there is no such swing.
    pub fn set_player(
        &self,
        session_id: &str,
        swing_id: &str,
        player_id: &str,
        now: Timestamp,
    ) -> io::Result<Option<SwingManifest>> {
        self.repair(session_id, swing_id, now, |manifest| {
            manifest.player_id = Some(player_id.to_string());
        })
    }

    /// Retag one swing, overwriting whatever it said: the club's **only** repair path, per swing and
    /// nothing else, since nothing but a human naming it can say which club hit swing 3. `None` if
    /// there is no such swing.
    pub fn set_club(
        &self,
        session_id: &str,
        swing_id: &str,
        club: ClubId,
        now: Timestamp,
    ) -> io::Result<Option<SwingManifest>> {
        self.repair(session_id, swing_id, now, |manifest| {
            manifest.club = Some(club);
        })
    }

    /// Record the golfer's verdict on this swing's shot, or clear it with `None` so the automatic
    /// rule decides again (ADR-028). Per swing, for `set_club`'s reason: only the golfer who hit it
    /// knows whether they topped it. `None` if there is no such swing.
    pub fn set_mishit(
        &self,
        session_id: &str,
        swing_id: &str,
        verdict: Option<MishitVerdict>,
        now: Timestamp,
    ) -> io::Result<Option<SwingManifest>> {
        self.repair(session_id, swing_id, now, |manifest| {
            manifest.mishit = verdict;
        })
    }

    /// Remove one swing and everything in its directory. `true` if there was one to remove.
    ///
    /// The counterpart to "swing identity is assigned by the store": a corrective re-upload of a
    /// role the swing already has lands as a *phantom* swing holding one file, and a phantom is the
    /// newest swing lacking both clips, so the next real swing's clips slot into it and pair with a
    /// shot screen from another swing — silently. Recursive and unconditional, since the directory
    /// holds only this swing's uploads and what was derived from them. Analysis in flight is not
    /// waited on; the API refuses the delete while a run is queued, and this layer stays mechanical.
    pub fn delete_swing(&self, session_id: &str, swing_id: &str) -> io::Result<bool> {
        let _held = self.lock();
        let swing_dir = self.swing_dir(session_id, swing_id);
        if !swing_dir.is_dir() {
            return Ok(false);
        }
        fs::remove_dir_all(&swing_dir)?;
        Ok(true)
    }

    fn swing_dir(&self, session_id: &str, swing_id: &str) -> PathBuf {
        self.root.join(session_id).join(swing_id)
    }

    /// `set_player`, `set_club` and `set_mishit`'s one shape: read the manifest in this directory,
    /// edit one field, stamp `updated_at`, save it back **to this directory**. A missing or corrupt
    /// manifest is `None` and stays as it was.
    fn repair(
        &self,
        session_id: &str,
        swing_id: &str,
        now: Timestamp,
        edit: impl FnOnce(&mut SwingManifest),
    ) -> io::Result<Option<SwingManifest>> {
        let _held = self.lock();
        let path = manifest_path(&self.swing_dir(session_id, swing_id));
        let Some(mut manifest) = load_manifest(&path) else {
            return Ok(None);
        };
        edit(&mut manifest);
        manifest.updated_at = now;
        save_manifest(&manifest, &path)?;
        Ok(Some(manifest))
    }

    /// `_place`: move the upload into `<session_dir>/<manifest.swing_id>` and record it.
    ///
    /// The golfer and the club are **stamp-if-empty**. That covers the ordinary case (the swing was
    /// opened by this very upload) and the one that only shows up with two phones: the face-on phone
    /// uploads before anyone picked a golfer, the down-the-line phone after, and the swing is
    /// attributed on the second file rather than staying anonymous because of which phone was
    /// faster. For the club it covers a swing written before the field existed receiving a later
    /// role, tagged from the cursor current *now*, when the swing is being completed.
    ///
    /// A role already filled under another filename has that file unlinked, so a replaced clip is
    /// gone and not merely unreferenced; the same filename is simply written over by the rename.
    fn place(
        &self,
        session_dir: &Path,
        mut manifest: SwingManifest,
        upload: &Upload<'_>,
        now: Timestamp,
    ) -> io::Result<AssignmentResult> {
        let swing_dir = session_dir.join(&manifest.swing_id);
        fs::create_dir_all(&swing_dir)?;
        let filename = content_filename(upload.role, upload.digest, upload.original_filename);

        if manifest.player_id.is_none() {
            manifest.player_id = upload.player_id.map(str::to_string);
        }
        if manifest.club.is_none() {
            manifest.club = upload.club;
        }

        if let Some(old) = manifest.roles.get(&upload.role) {
            let old_path = swing_dir.join(&old.filename);
            if old.filename != filename && old_path.exists() {
                fs::remove_file(old_path)?;
            }
        }
        fs::rename(upload.tmp_path, swing_dir.join(&filename))?;

        manifest.roles.insert(
            upload.role,
            RoleFile {
                role: upload.role,
                filename,
                content_sha256: upload.digest.to_string(),
                original_filename: upload.original_filename.to_string(),
                content_type: upload.content_type.to_string(),
                size_bytes: upload.size_bytes,
                received_at: now,
                warnings: Vec::new(),
            },
        );
        manifest.updated_at = now;
        save_manifest(&manifest, &manifest_path(&swing_dir))?;

        Ok(AssignmentResult {
            session_id: manifest.session_id.clone(),
            swing_id: manifest.swing_id.clone(),
            role: upload.role,
            status: manifest.status(),
            missing_roles: manifest.missing_roles(),
            deduped: false,
            player_id: manifest.player_id,
            club: manifest.club,
        })
    }
}

/// `_new_manifest`: a swing opened by this upload, stamped with the cursor's golfer and club.
fn new_manifest(upload: &Upload<'_>, swing_id: &str, now: Timestamp) -> SwingManifest {
    SwingManifest {
        swing_id: swing_id.to_string(),
        session_id: upload.session_id.to_string(),
        created_at: now,
        updated_at: now,
        roles: Default::default(),
        player_id: upload.player_id.map(str::to_string),
        club: upload.club,
        mishit: None,
    }
}

/// `tmp_path.unlink(missing_ok=True)`.
fn remove_if_present(path: &Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}

/// `_next_swing_id`: one past the highest numeric manifest id, or `1`. Python's `int` has no
/// ceiling, so neither does this: the number is carried as its digits and incremented as text.
fn next_swing_id(manifests: &[SwingManifest]) -> String {
    let highest = manifests
        .iter()
        .map(|manifest| manifest.swing_id.as_str())
        .filter(|id| is_swing_number(id))
        .map(Number::of)
        .max()
        .unwrap_or(Number("0"));
    highest.plus_one()
}

/// Python's `name.isdigit()`, for ASCII (the module doc says why only ASCII).
fn is_swing_number(name: &str) -> bool {
    !name.is_empty() && name.bytes().all(|byte| byte.is_ascii_digit())
}

/// A non-negative integer as its canonical digits, ordered by value at any length.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Number<'a>(&'a str);

impl<'a> Number<'a> {
    /// `int(digits)`: leading zeros dropped, so `007` is `7`.
    fn of(digits: &'a str) -> Self {
        match digits.trim_start_matches('0') {
            "" => Number("0"),
            canonical => Number(canonical),
        }
    }

    /// `str(n + 1)`.
    fn plus_one(self) -> String {
        let mut digits = self.0.as_bytes().to_vec();
        for digit in digits.iter_mut().rev() {
            if *digit == b'9' {
                *digit = b'0';
            } else {
                *digit += 1;
                return String::from_utf8(digits).expect("ASCII digits");
            }
        }
        format!("1{}", String::from_utf8(digits).expect("ASCII digits"))
    }
}

impl Ord for Number<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0
            .len()
            .cmp(&other.0.len())
            .then_with(|| self.0.cmp(other.0))
    }
}

impl PartialOrd for Number<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// `10**9`, where `_swing_sort_key` ranks every non-numeric name: after swing 999,999,999 and
/// before swing 1,000,000,001, as Python's tuple does.
const NON_NUMERIC_RANK: Number<'static> = Number("1000000000");

/// `_swing_sort_key`: a numeric directory by its number, anything else at `10**9` by its name.
pub(crate) fn swing_sort_key(name: &str) -> (Number<'_>, &str) {
    if is_swing_number(name) {
        (Number::of(name), "")
    } else {
        (NON_NUMERIC_RANK, name)
    }
}

/// The order [`SwingBundleStore::get_session`] walks a session in: [`swing_sort_key`], then the
/// name, so a tie (`7` and `007`) is not the filesystem's to break.
pub(crate) fn swing_order(a: &str, b: &str) -> Ordering {
    swing_sort_key(a)
        .cmp(&swing_sort_key(b))
        .then_with(|| a.cmp(b))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::hash_bytes;

    const SESSION: &str = "2026-08-06";

    fn now() -> Timestamp {
        "2026-08-06T12:00:00Z".parse().unwrap()
    }

    fn store() -> (tempfile::TempDir, SwingBundleStore) {
        let dir = tempfile::tempdir().unwrap();
        let store = SwingBundleStore::new(dir.path());
        (dir, store)
    }

    /// `test_bundle_store.py::_upload`: stream the bytes to `.incoming/`, then assign them.
    fn upload(
        store: &SwingBundleStore,
        role: Role,
        data: &[u8],
        edit: impl FnOnce(&mut Upload<'_>),
    ) -> AssignmentResult {
        let incoming = store.root().join(".incoming");
        fs::create_dir_all(&incoming).unwrap();
        let digest = hash_bytes(data);
        let tmp = incoming.join(format!("{digest}-{}.part", role.as_str()));
        fs::write(&tmp, data).unwrap();
        let mut given = Upload {
            session_id: SESSION,
            role,
            tmp_path: &tmp,
            digest: &digest,
            original_filename: "clip.mov",
            content_type: "video/quicktime",
            size_bytes: i64::try_from(data.len()).unwrap(),
            swing_id: None,
            player_id: None,
            club: None,
        };
        edit(&mut given);
        store.assign_from_path(&given, now()).unwrap()
    }

    fn plain(store: &SwingBundleStore, role: Role, data: &[u8]) -> AssignmentResult {
        upload(store, role, data, |_| {})
    }

    fn swing_ids(store: &SwingBundleStore) -> Vec<String> {
        store
            .get_session(SESSION)
            .into_iter()
            .map(|m| m.swing_id)
            .collect()
    }

    #[test]
    fn current_session_id_is_the_date_in_nows_own_offset() {
        let (_dir, store) = store();
        let at = |text: &str| store.current_session_id(text.parse().unwrap());
        assert_eq!(at("2026-03-05T09:30:00Z"), "2026-03-05");
        assert_eq!(at("2026-08-07T00:30:00+05:30"), "2026-08-07");
        assert_eq!(at("2026-08-06T23:30:00-05:00"), "2026-08-06");
    }

    #[test]
    fn roles_fill_one_swing_in_any_order_and_the_next_opens_only_once_it_is_complete() {
        let (_dir, store) = store();
        let first = plain(&store, Role::ShotScreen, b"screen");
        let second = plain(&store, Role::DownTheLine, b"dtl");
        assert_eq!(
            (first.swing_id.as_str(), second.swing_id.as_str()),
            ("1", "1")
        );
        assert_eq!(second.status, BundleStatus::Collecting);
        assert_eq!(second.missing_roles, [Role::FaceOn]);

        let third = plain(&store, Role::FaceOn, b"face-on");
        assert_eq!(
            (third.swing_id.as_str(), third.status),
            ("1", BundleStatus::Complete)
        );
        assert_eq!(plain(&store, Role::FaceOn, b"swing2-face").swing_id, "2");
    }

    #[test]
    fn the_same_bytes_dedupe_and_report_what_is_stored() {
        let (_dir, store) = store();
        let first = upload(&store, Role::FaceOn, b"same-bytes", |u| {
            u.club = Some(ClubId::SevenIron)
        });
        let again = upload(&store, Role::FaceOn, b"same-bytes", |u| {
            u.club = Some(ClubId::PitchingWedge);
            u.player_id = Some("dave");
        });
        assert!(again.deduped && !first.deduped);
        assert_eq!(again.swing_id, first.swing_id);
        assert_eq!(
            (again.club, again.player_id),
            (Some(ClubId::SevenIron), None)
        );
        assert_eq!(store.get_session(SESSION).len(), 1);
        let leftovers = fs::read_dir(store.root().join(".incoming"))
            .unwrap()
            .count();
        assert_eq!(leftovers, 0, "the duplicate's scratch file was removed");
    }

    /// The documented limitation, twice: a re-upload of a filled role opens the next swing, and a
    /// role both swings lack lands in the newest.
    #[test]
    fn different_bytes_open_a_new_swing_and_the_newest_wins() {
        let (_dir, store) = store();
        plain(&store, Role::FaceOn, b"first-recording");
        assert_eq!(plain(&store, Role::FaceOn, b"different").swing_id, "2");
        assert_eq!(plain(&store, Role::DownTheLine, b"dtl").swing_id, "2");
    }

    #[test]
    fn an_explicit_target_replaces_the_role_in_place() {
        let (_dir, store) = store();
        plain(&store, Role::ShotScreen, b"bad-photo");
        let fixed = upload(&store, Role::ShotScreen, b"good-photo", |u| {
            u.swing_id = Some("1");
            u.original_filename = "shot.jpg";
        });
        assert_eq!(fixed.swing_id, "1");
        assert_eq!(swing_ids(&store), ["1"]);
        let manifest = store.get_swing(SESSION, "1").unwrap();
        assert_eq!(
            manifest.roles[&Role::ShotScreen].content_sha256,
            hash_bytes(b"good-photo")
        );
        let names: Vec<String> = fs::read_dir(store.root().join(SESSION).join("1"))
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .filter(|name| name != "manifest.json")
            .collect();
        assert_eq!(names.len(), 1, "the superseded file is gone: {names:?}");
        assert!(names[0].ends_with(".jpg"));

        let opened = upload(&store, Role::FaceOn, b"named", |u| u.swing_id = Some("7"));
        assert_eq!(opened.swing_id, "7");
        assert_eq!(swing_ids(&store), ["1", "7"]);
        assert_eq!(plain(&store, Role::ShotScreen, b"auto").swing_id, "7");
        assert_eq!(plain(&store, Role::ShotScreen, b"auto-2").swing_id, "8");
    }

    #[test]
    fn a_corrupt_or_missing_session_is_skipped_not_fatal() {
        let (_dir, store) = store();
        assert!(store.get_session(SESSION).is_empty());
        assert!(store.list_session_ids().is_empty());
        plain(&store, Role::FaceOn, b"face-on");
        let corrupt = store.root().join(SESSION).join("corrupt");
        fs::create_dir_all(&corrupt).unwrap();
        fs::write(corrupt.join("manifest.json"), "{not json").unwrap();
        assert_eq!(swing_ids(&store), ["1"]);
        assert_eq!(store.list_session_ids(), [SESSION]);
    }

    #[test]
    fn the_golfer_and_the_club_stamp_once_and_are_never_restamped() {
        let (_dir, store) = store();
        let anonymous = plain(&store, Role::FaceOn, b"a");
        assert_eq!((anonymous.player_id, anonymous.club), (None, None));
        let labelled = upload(&store, Role::DownTheLine, b"b", |u| {
            u.player_id = Some("aaron");
            u.club = Some(ClubId::SevenIron);
        });
        assert_eq!(labelled.swing_id, "1");
        assert_eq!(labelled.player_id.as_deref(), Some("aaron"));
        let kept = upload(&store, Role::ShotScreen, b"c", |u| {
            u.player_id = Some("dave");
            u.club = Some(ClubId::PitchingWedge);
        });
        assert_eq!(
            (kept.player_id.as_deref(), kept.club),
            (Some("aaron"), Some(ClubId::SevenIron))
        );
    }

    #[test]
    fn attribute_unlabeled_adopts_only_the_anonymous_swings_and_no_club() {
        let (_dir, store) = store();
        plain(&store, Role::FaceOn, b"one");
        upload(&store, Role::FaceOn, b"two", |u| u.player_id = Some("dave"));
        upload(&store, Role::FaceOn, b"three", |u| {
            u.club = Some(ClubId::SevenIron)
        });
        let changed = store.attribute_unlabeled(SESSION, "aaron", now()).unwrap();
        assert_eq!(changed, ["1", "3"]);
        assert!(store
            .attribute_unlabeled(SESSION, "aaron", now())
            .unwrap()
            .is_empty());
        let players: Vec<_> = store
            .get_session(SESSION)
            .into_iter()
            .map(|m| (m.player_id.unwrap(), m.club))
            .collect();
        assert_eq!(
            players,
            [
                ("aaron".to_string(), None),
                ("dave".to_string(), None),
                ("aaron".to_string(), Some(ClubId::SevenIron)),
            ]
        );
    }

    #[test]
    fn the_repairs_overwrite_one_field_of_one_swing() {
        let (_dir, store) = store();
        let later: Timestamp = "2026-08-06T13:00:00Z".parse().unwrap();
        upload(&store, Role::FaceOn, b"one", |u| {
            u.player_id = Some("dave");
            u.club = Some(ClubId::PitchingWedge);
        });
        upload(&store, Role::FaceOn, b"two", |u| {
            u.player_id = Some("dave");
            u.club = Some(ClubId::PitchingWedge);
        });

        let player = store.set_player(SESSION, "1", "aaron", later).unwrap();
        let player = player.unwrap();
        assert_eq!(
            (player.player_id.as_deref(), player.club),
            (Some("aaron"), Some(ClubId::PitchingWedge))
        );
        assert_eq!(player.updated_at, later);
        let club = store
            .set_club(SESSION, "2", ClubId::SevenIron, later)
            .unwrap()
            .unwrap();
        assert_eq!(
            (club.player_id.as_deref(), club.club),
            (Some("dave"), Some(ClubId::SevenIron))
        );
        for verdict in [
            Some(MishitVerdict::Confirmed),
            Some(MishitVerdict::Cleared),
            None,
        ] {
            let marked = store.set_mishit(SESSION, "1", verdict, later).unwrap();
            assert_eq!(marked.unwrap().mishit, verdict);
            assert_eq!(store.get_swing(SESSION, "1").unwrap().mishit, verdict);
        }
        assert!(store
            .set_player(SESSION, "99", "aaron", later)
            .unwrap()
            .is_none());
        assert!(store
            .set_club(SESSION, "99", ClubId::SevenIron, later)
            .unwrap()
            .is_none());
        assert!(store
            .set_mishit(SESSION, "99", None, later)
            .unwrap()
            .is_none());
        assert!(!store.root().join(SESSION).join("99").exists());
    }

    /// Why `delete_swing` exists, written as the failure it prevents, then the delete itself.
    #[test]
    fn deleting_a_phantom_stops_it_swallowing_the_next_swing() {
        let (_dir, store) = store();
        plain(&store, Role::FaceOn, b"clip-1");
        plain(&store, Role::ShotScreen, b"bad-photo");
        assert_eq!(plain(&store, Role::ShotScreen, b"good-photo").swing_id, "2");
        assert_eq!(plain(&store, Role::FaceOn, b"clip-2").swing_id, "2");
        fs::write(
            store.root().join(SESSION).join("2").join("analysis.json"),
            "{}",
        )
        .unwrap();

        assert!(store.delete_swing(SESSION, "2").unwrap());
        assert_eq!(swing_ids(&store), ["1"]);
        assert!(!store.root().join(SESSION).join("2").exists());
        assert!(!store.delete_swing(SESSION, "2").unwrap());
        assert!(!store.delete_swing(SESSION, "99").unwrap());
    }

    /// `_swing_sort_key` and `_next_swing_id` as Python's `int` reads them: by value at any length,
    /// leading zeros dropped, non-numeric names after swing 10**9 - 1 and before 10**9 + 1.
    #[test]
    fn swing_numbers_order_and_increment_as_python_ints() {
        let mut names = vec![
            "x",
            "10",
            "2",
            "007",
            "7",
            "0",
            ".incoming",
            "999999999",
            "1000000000",
            "1000000001",
            "99999999999999999999999",
        ];
        names.sort_by(|a, b| swing_order(a, b));
        assert_eq!(
            names,
            [
                "0",
                "2",
                "007",
                "7",
                "10",
                "999999999",
                "1000000000",
                ".incoming",
                "x",
                "1000000001",
                "99999999999999999999999",
            ]
        );

        let manifest = |id: &str| SwingManifest {
            swing_id: id.to_string(),
            ..new_manifest(
                &Upload {
                    session_id: SESSION,
                    role: Role::FaceOn,
                    tmp_path: Path::new("unused"),
                    digest: "",
                    original_filename: "",
                    content_type: "",
                    size_bytes: 0,
                    swing_id: None,
                    player_id: None,
                    club: None,
                },
                id,
                now(),
            )
        };
        let next =
            |ids: &[&str]| next_swing_id(&ids.iter().map(|id| manifest(id)).collect::<Vec<_>>());
        assert_eq!(next(&[]), "1");
        assert_eq!(next(&["x", "0"]), "1");
        assert_eq!(next(&["9", "007", "x"]), "10");
        assert_eq!(next(&["099"]), "100");
        assert_eq!(
            next(&["99999999999999999999999"]),
            "100000000000000000000000"
        );
        assert_eq!(
            next(&["\u{663}"]),
            "1",
            "a non-ASCII digit is not a swing number here"
        );
    }
}
