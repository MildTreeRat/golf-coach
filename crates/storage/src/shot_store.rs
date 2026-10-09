//! The parsed-shot cache — parse each photo once, keep the result. `launch_monitor/screen/store.py`.
//! [M36 P8, ADR-014]
//!
//! OCR on a rectified 4K photo is slow enough that re-running an import over a session's photos
//! would hurt, so each parse is written out, keyed by the photo's SHA-256. Content addressing rather
//! than filename means re-importing the same photo under another name is still a hit, and a photo of
//! a *different* shot never collides. One JSON file per shot: a bad parse is deleted by deleting one
//! file, and there is no shared index to lose data from.
//!
//! # A shot is written in Rust's shape
//!
//! `contracts::shot::ShotData` is M32's wider shape, so a shot this writes carries the seven keys
//! M32 added and the three provenance keys beside them, each at its default for a shot parsed by
//! frozen Python's rules, where frozen Python's file has none of them. That is the M31.5 plan's
//! P1 finding 1 in the direction it was measured: Python reads a Rust-shaped shot by ignoring the
//! extras. So the storage family's gate compares a written shot on the keys Python wrote and holds
//! every other key at its default (M36 P2's finding 6, settled at P8), rather than this store
//! leaving defaults out, which would change how every engine vector serializes its shot.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use contracts::shot::ShotData;

use crate::StoreError;

/// `_SUFFIX`.
pub const SUFFIX: &str = ".shot.json";

/// Content hash of a photo — the cache key for its parse. The manifest's hash, by another name, as
/// it is in Python.
pub fn hash_image(data: &[u8]) -> String {
    crate::manifest::hash_bytes(data)
}

/// A directory of parsed shots, one JSON file each, keyed by source-photo hash.
#[derive(Debug, Clone)]
pub struct ShotStore {
    root: PathBuf,
}

impl ShotStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        ShotStore { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// `<root>/<key>.shot.json`, with each run of characters outside `[A-Za-z0-9._-]` — non-ASCII
    /// included, since Python's class is ASCII ranges — replaced by one `-`.
    pub fn path_for(&self, key: &str) -> PathBuf {
        let mut safe = String::with_capacity(key.len());
        let mut in_run = false;
        for ch in key.chars() {
            if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-') {
                safe.push(ch);
                in_run = false;
            } else if !in_run {
                safe.push('-');
                in_run = true;
            }
        }
        self.root.join(format!("{safe}{SUFFIX}"))
    }

    pub fn has(&self, key: &str) -> bool {
        self.path_for(key).exists()
    }

    /// The stored shot, or `None` when there is no file.
    ///
    /// **The one reader here that raises on a file it cannot read**, where every other store's
    /// reader answers `None` and [`Self::all`] skips it: a file that is not a shot is
    /// [`StoreError::Unreadable`], and one that cannot be opened is [`StoreError::Io`]. Ported as it
    /// is; the storage family's `shot-all-order` pins it.
    pub fn get(&self, key: &str) -> Result<Option<ShotData>, StoreError> {
        let path = self.path_for(key);
        if !path.exists() {
            return Ok(None);
        }
        let bytes = fs::read(&path)?;
        serde_json::from_slice(&bytes)
            .map(Some)
            .map_err(|error| StoreError::Unreadable {
                path,
                reason: error.to_string(),
            })
    }

    /// Write a shot, keyed by [`Self::key_for`], and answer the path written. A second `put` of a
    /// key writes over the first.
    ///
    /// Written in place, not through a `.tmp`, as Python writes it: this is a cache, and a torn file
    /// is one [`Self::all`] skips and one re-parse replaces.
    pub fn put(&self, shot: &ShotData) -> io::Result<PathBuf> {
        fs::create_dir_all(&self.root)?;
        let path = self.path_for(Self::key_for(shot));
        fs::write(&path, crate::to_pretty_json(shot))?;
        Ok(path)
    }

    /// Every stored shot, newest first. Unreadable files are skipped, not fatal.
    ///
    /// **Newest by instant, not by spelling**: `08:00-05:00` is 13:00Z and leads `12:00Z`, which is
    /// why `Timestamp` became a parsed type (M36 P4). The files are read in sorted-name order and the
    /// sort is stable with the comparison reversed, which is what Python's `sort(reverse=True)` does
    /// — two shots at one instant keep the files' order, where sorting ascending and reversing the
    /// list would swap them.
    pub fn all(&self) -> Vec<ShotData> {
        let Ok(entries) = fs::read_dir(&self.root) else {
            return Vec::new();
        };
        let mut names: Vec<String> = entries
            .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
            .filter(|name| name.ends_with(SUFFIX))
            .collect();
        names.sort();
        let mut shots: Vec<ShotData> = names
            .iter()
            .filter_map(|name| {
                let bytes = fs::read(self.root.join(name)).ok()?;
                serde_json::from_slice(&bytes).ok()
            })
            .collect();
        shots.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        shots
    }

    /// `_key_for`: the photo's hash when the shot has one, else its `shot_id`. An empty hash is no
    /// hash, as Python's truthiness test reads it.
    pub fn key_for(shot: &ShotData) -> &str {
        shot.provenance
            .as_ref()
            .and_then(|provenance| provenance.image_sha256.as_deref())
            .filter(|digest| !digest.is_empty())
            .unwrap_or(&shot.shot_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shot(shot_id: &str, when: &str, digest: Option<&str>) -> ShotData {
        let provenance = digest.map(|digest| {
            serde_json::json!({"device": "hd_golf", "parse_confidence": 0.95, "image_sha256": digest})
        });
        serde_json::from_value(serde_json::json!({
            "shot_id": shot_id, "session_id": "range", "timestamp": when, "source": "screen",
            "carry_distance": 128.1, "provenance": provenance,
        }))
        .unwrap()
    }

    #[test]
    fn a_key_is_made_safe_one_run_at_a_time() {
        let store = ShotStore::new("root");
        for (key, file) in [
            (
                "2026-08-04 12:00/swing#1",
                "2026-08-04-12-00-swing-1.shot.json",
            ),
            ("na\u{ef}ve\u{2026}", "na-ve-.shot.json"),
            ("a.b_c-D9", "a.b_c-D9.shot.json"),
            ("", ".shot.json"),
        ] {
            assert_eq!(store.path_for(key), Path::new("root").join(file), "{key:?}");
        }
    }

    #[test]
    fn the_key_falls_back_to_the_shot_id() {
        assert_eq!(
            ShotStore::key_for(&shot("s1", "2026-08-04T12:00:00Z", Some("abc"))),
            "abc"
        );
        assert_eq!(
            ShotStore::key_for(&shot("s2", "2026-08-04T12:00:00Z", Some(""))),
            "s2"
        );
        assert_eq!(
            ShotStore::key_for(&shot("s3", "2026-08-04T12:00:00Z", None)),
            "s3"
        );
    }

    #[test]
    fn a_shot_round_trips_and_a_missing_store_is_empty() {
        let dir = tempfile::tempdir().unwrap();
        let store = ShotStore::new(dir.path().join("shots"));
        assert!(store.all().is_empty());
        assert!(store.get("abc").unwrap().is_none());

        let first = shot("s1", "2026-08-04T12:00:00Z", Some("abc"));
        let path = store.put(&first).unwrap();
        assert_eq!(path, store.path_for("abc"));
        assert!(store.has("abc"));
        assert_eq!(store.get("abc").unwrap(), Some(first));
    }

    #[test]
    fn all_is_newest_first_by_instant_and_stable_on_a_tie() {
        let dir = tempfile::tempdir().unwrap();
        let store = ShotStore::new(dir.path());
        for (id, when) in [
            ("a", "2026-08-04T12:00:00Z"),
            ("b", "2026-08-04T12:00:00Z"),
            ("c", "2026-08-04T17:00:00+05:30"),
            ("d", "2026-08-04T08:00:00-05:00"),
        ] {
            store.put(&shot(id, when, None)).unwrap();
        }
        fs::write(store.path_for("e"), "{not json").unwrap();
        let order: Vec<String> = store.all().into_iter().map(|s| s.shot_id).collect();
        assert_eq!(order, ["d", "a", "b", "c"]);
        assert!(matches!(store.get("e"), Err(StoreError::Unreadable { .. })));
    }
}
