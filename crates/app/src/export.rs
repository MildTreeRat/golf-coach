//! The phone's export: the stored layout in a zip, for `golf-lab import-export`. [M38 P13]
//!
//! There is no export contract to drift from the stores' own (the M29 plan's D16): each file goes as
//! it is stored, at its path below the data root, and `tests/export_roundtrip.rs` holds the result to
//! what the real `import-export` makes of it. That test is half of the milestone's exit.
//!
//! # What goes
//!
//! **C16's list is walked by name, and `.app/` itself is never walked**: `sessions/`, `shots/`,
//! `golfers/`, `.app/reads.jsonl`, `.app/boxes/` and `.app/failed/`, and since P30b
//! `.app/events.jsonl` (D16). So `state.json`, its `.tmp` and
//! `.app/deleted.json` stay home because nothing here looks at them, and a file a later phase adds to
//! `.app/` stays home until someone adds it to this list. The lab counts a dotted path as ignored, so
//! the diagnostics travel in the export and never reach its stores (C7).
//!
//! **Since M46 P21 the list holds `drills/` too** (M46's C17), whole: each golfer's strikes, drill
//! state, crops and each club's last crop. The lab merges the first three by its own rule and never
//! imports `last/`, the phone's reference for the club's next photo. A strike waiting for the golfer's
//! tap is under `.app/drill_pending/` and stays home, so every crop that goes is a stored strike's,
//! save one whose strike a crash cut off (the M46 plan's P18 finding 6).
//!
//! **What a crash or the store's own working left behind does not go** ([`left_behind`]): a dotted
//! name, which covers `sessions/.incoming/`, and a `.tmp` or `.part`. The lab would ignore each as
//! well, by the same list, and counting them there is only noise for someone reading its tally. A
//! directory is never an entry, so an empty one lands nowhere and counts as nothing.
//!
//! # How it is written
//!
//! - **Sorted, so the same store is the same bytes.** The time on each entry is the file's, and `now`
//!   reaches only the answer, so exporting an unchanged store twice writes one zip twice. The
//!   entry's DOS time stays `zip`'s 1980 default, since it is local with no zone: the lab does not read
//!   it, which is why the extended-timestamp field is here.
//! - **JSON deflated, photos stored** (C16), as a JPEG or HEIC has nothing left to give. A file is
//!   streamed and not read whole, because a phone's memory is what a session of photos would cost.
//! - **Each entry carries its time in a `0x5455` extra field** (Info-ZIP's extended timestamp), UTC,
//!   whole seconds, which is how a photo keeps its time at the lab (C16). A sub-second part is lost on
//!   the file there, and a time outside its 32 bits has no field at all, so that file takes the time it
//!   is imported at. A stored shot and its manifest cross as bytes, so neither loses anything: only a
//!   `--force` re-read at the lab, which stamps the shot from the photo, could see the difference.
//! - **A `.part` beside the path and a rename**, so a failed export leaves an older one whole, and
//!   nothing half-written is ever at the path Settings hands to the share sheet.
//!
//! # One moment
//!
//! The whole walk holds `crate::record::one_writer()`. `ShotStore::put` writes in place, and an entry can
//! vanish under a delete, so without it a shot torn in the zip refuses the **whole** import at the lab
//! (exit 2, "not a shot the shot store reads"), where a wait costs one photo a few seconds at worst.
//! [`status`] does not take it: it reads only manifests, each renamed into place whole, and a Settings
//! row should not wait on an export to say when the last one was.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use contracts::Timestamp;
use storage::bundle_store::SwingBundleStore;
use storage::manifest::{Role, SwingManifest};
use zip::write::{FullFileOptions, ZipWriter};
use zip::CompressionMethod;

use crate::api::export::{ExportStatus, ExportSummary};
use crate::history::clock_of;
use crate::record::{one_writer, EVENTS_NAME, READS_NAME};
use crate::{parse_now, state, Dirs};

/// Info-ZIP's extended-timestamp extra field.
const EXTENDED_TIMESTAMP: u16 = 0x5455;

/// The field's one flag: the modification time follows.
const HAS_MODIFIED: u8 = 1;

/// A zip holding none of the four trees is one `import-export` refuses (exit 2, "it holds no file
/// under"), so a phone saying so itself is better than a file handed to the share sheet that fails at
/// the far end. Only a phone with no golfer is here, and first-run setup makes one before anything
/// else, since a drill is a golfer's too. A golfer with no shots does export, since a session of nothing but failed reads is the one
/// whose diagnostics are most wanted.
const NOTHING_YET: &str = "There is nothing on this phone to export yet.";

/// `export_sessions`' body (the module doc).
pub(crate) fn export(root: &str, out_path: &str, now: &str) -> Result<ExportSummary, String> {
    let now = parse_now(now)?;
    let (root, out) = (Path::new(root), Path::new(out_path));
    // The next export would carry the last one if it sat in a tree, and C16 puts it in `tmp/`.
    if out.starts_with(root) {
        return Err(format!(
            "{out_path} is inside the data it would export, so it cannot be written there"
        ));
    }
    let _one_at_a_time = one_writer();
    let dirs = Dirs::of(root);
    let files = listed(root, &dirs)
        .map_err(|failed| format!("could not read what there is to export: {failed}"))?;
    if files.keys().all(|name| name.starts_with(".app/")) {
        return Err(NOTHING_YET.to_string());
    }
    let bytes =
        written(&files, out).map_err(|failed| format!("could not write {out_path}: {failed}"))?;
    Ok(ExportSummary {
        path: out_path.to_string(),
        exported_at: now.isoformat(),
        shots: count(photo_entries(&dirs).len()),
        files: count(files.len()),
        bytes: i64::try_from(bytes).unwrap_or(i64::MAX),
    })
}

/// `export_status`' body (the module doc).
pub(crate) fn status(root: &str) -> ExportStatus {
    let state = state::read(Path::new(root));
    // `set_state` refuses a time like this, so only a file edited by hand holds one, and a read here
    // never refuses (`crate::state`'s doc). Having no last export overstates what is unsent.
    let last = state
        .last_export_at
        .as_deref()
        .and_then(|at| parse_now(at).ok());
    let entries = photo_entries(&Dirs::of(root));
    let since = entries
        .iter()
        .filter(|entry| last.is_none_or(|last| entry.created_at > last))
        .count();
    ExportStatus {
        last_export_at: last.map(|at| at.isoformat()),
        last_export_display: last.map(display_of),
        last_export_shots: state.last_export_shots,
        shots: count(entries.len()),
        shots_since: count(since),
    }
}

/// `2026-11-05 11:00`, in the time's own offset.
fn display_of(at: Timestamp) -> String {
    format!("{} {}", at.date_ymd(), clock_of(at))
}

fn count(items: usize) -> i64 {
    i64::try_from(items).unwrap_or(i64::MAX)
}

/// Every entry holding a photo, which is what the phone means by a shot. `get_session` skips what does
/// not read, as every other surface here does.
fn photo_entries(dirs: &Dirs) -> Vec<SwingManifest> {
    let store = SwingBundleStore::new(&dirs.sessions);
    store
        .list_session_ids()
        .iter()
        .flat_map(|session_id| store.get_session(session_id))
        .filter(|entry| entry.roles.contains_key(&Role::ShotScreen))
        .collect()
}

/// C16's list as `(name in the zip, file)`, sorted by name.
fn listed(root: &Path, dirs: &Dirs) -> io::Result<BTreeMap<String, PathBuf>> {
    let mut files = BTreeMap::new();
    for tree in [
        &dirs.sessions,
        &dirs.shots,
        &dirs.golfers,
        &dirs.drills,
        &dirs.app.join("boxes"),
        &dirs.app.join("failed"),
    ] {
        walk(root, tree, &mut files)?;
    }
    // Each log is there once something has been written to it. Neither is a reason to refuse.
    for log in [READS_NAME, EVENTS_NAME] {
        let path = dirs.app.join(log);
        if path.is_file() {
            files.insert(name_below(root, &path)?, path);
        }
    }
    Ok(files)
}

/// Every plain file below `dir`. A store nothing has been written to yet has no directory, which is no
/// reason to refuse: a phone with no failed read has no `.app/failed/`. A symbolic link is neither, and
/// no store writes one.
fn walk(root: &Path, dir: &Path, files: &mut BTreeMap<String, PathBuf>) -> io::Result<()> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(missing) if missing.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(failed) => return Err(failed),
    };
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if left_behind(&entry.file_name().to_string_lossy()) {
            continue;
        }
        let kind = entry.file_type()?;
        if kind.is_dir() {
            walk(root, &path, files)?;
        } else if kind.is_file() {
            files.insert(name_below(root, &path)?, path);
        }
    }
    Ok(())
}

/// What is not a stored file (the module doc), by `golf_lab::import_export::ignored`'s rule for a name,
/// which this crate cannot reach. The round trip's tally is what holds the two together: a file this
/// let through and that one ignored would count there.
fn left_behind(name: &str) -> bool {
    name.starts_with('.') || name.ends_with(".tmp") || name.ends_with(".part")
}

/// The path below the root, with a `/` between names whatever the host's separator, or a refusal: a
/// name that is not text cannot be written as one the lab reads, and a shot left out quietly would be
/// worse than an export refused by its name.
fn name_below(root: &Path, path: &Path) -> io::Result<String> {
    let relative = path.strip_prefix(root).map_err(io::Error::other)?;
    let names: Option<Vec<&str>> = relative
        .components()
        .map(|name| name.as_os_str().to_str())
        .collect();
    names
        .map(|names| names.join("/"))
        .ok_or_else(|| io::Error::other(format!("{} is not a name in text", path.display())))
}

/// The zip at `out`, whole or not at all, and its size.
fn written(files: &BTreeMap<String, PathBuf>, out: &Path) -> io::Result<u64> {
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut part = out.as_os_str().to_owned();
    part.push(".part");
    let part = PathBuf::from(part);
    let placed = zipped(files, &part).and_then(|size| fs::rename(&part, out).map(|()| size));
    if placed.is_err() {
        let _ = fs::remove_file(&part);
    }
    placed
}

fn zipped(files: &BTreeMap<String, PathBuf>, to: &Path) -> io::Result<u64> {
    let mut zip = ZipWriter::new(fs::File::create(to)?);
    for (name, path) in files {
        let mut source = fs::File::open(path)?;
        let modified = source.metadata()?.modified()?;
        zip.start_file(name.as_str(), options_for(name, modified)?)
            .map_err(io::Error::other)?;
        io::copy(&mut source, &mut zip)?;
    }
    let finished = zip.finish().map_err(io::Error::other)?;
    Ok(finished.metadata()?.len())
}

/// How one entry is written (the module doc).
fn options_for(name: &str, modified: SystemTime) -> io::Result<FullFileOptions<'static>> {
    let method = if name.ends_with(".json") || name.ends_with(".jsonl") {
        CompressionMethod::Deflated
    } else {
        CompressionMethod::Stored
    };
    let mut options = FullFileOptions::default().compression_method(method);
    if let Some(seconds) = seconds_of(modified) {
        let mut field = vec![HAS_MODIFIED];
        field.extend(seconds.to_le_bytes());
        options
            .add_extra_data(EXTENDED_TIMESTAMP, field, false)
            .map_err(io::Error::other)?;
    }
    Ok(options)
}

/// A file's time as the field spells it, or `None` where it cannot (the module doc).
fn seconds_of(modified: SystemTime) -> Option<u32> {
    let since = modified.duration_since(UNIX_EPOCH).ok()?;
    u32::try_from(since.as_secs()).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    /// What a store's writer or a crash leaves, and what a stored name looks like beside it. A swing's
    /// `.jpg`, a digest with no suffix and a name holding a dot all stay.
    #[test]
    fn only_a_dotted_name_or_an_unfinished_write_is_left_behind() {
        for name in [
            ".incoming",
            ".DS_Store",
            "manifest.tmp",
            "photo.jpg.123.part",
        ] {
            assert!(left_behind(name), "{name}");
        }
        for name in [
            "manifest.json",
            "aaron.golfer.json",
            "shot_screen.jpg",
            "3f2a",
            "tmp",
            "part",
        ] {
            assert!(!left_behind(name), "{name}");
        }
    }

    #[test]
    fn a_time_the_field_cannot_spell_has_no_field_rather_than_a_wrong_one() {
        let at = |seconds: u64| UNIX_EPOCH + Duration::from_secs(seconds);
        assert_eq!(seconds_of(at(1_793_613_600)), Some(1_793_613_600));
        assert_eq!(seconds_of(at(u64::from(u32::MAX))), Some(u32::MAX));
        assert_eq!(seconds_of(at(u64::from(u32::MAX) + 1)), None);
        assert_eq!(seconds_of(UNIX_EPOCH - Duration::from_secs(1)), None);
        assert_eq!(
            seconds_of(UNIX_EPOCH + Duration::from_millis(1_500)),
            Some(1),
            "whole seconds, rounded down"
        );
    }

    /// The module doc's one moment, as a direct pin: `tests/export_roundtrip.rs` overlaps real writers
    /// and depends on them landing inside an export, and this does not. Only a mutated export can pass
    /// the wait, so a slow machine cannot fail it, and one that gets past in 300 ms is what it catches.
    #[test]
    fn an_export_waits_for_the_writer_holding_the_lock() {
        let dir = tempfile::tempdir().expect("a scratch directory");
        let root = dir.path().join("processed");
        let golfers = Dirs::of(&root).golfers;
        fs::create_dir_all(&golfers).expect("the store");
        fs::write(golfers.join("aaron.golfer.json"), b"{}").expect("a file to export");
        let out = dir.path().join("export.zip");

        let writer = one_writer();
        let exporting = {
            let (root, out) = (root.display().to_string(), out.display().to_string());
            thread::spawn(move || export(&root, &out, "2026-11-05T11:00:00-05:00"))
        };
        thread::sleep(Duration::from_millis(300));
        assert!(!exporting.is_finished(), "it did not wait");
        assert!(!out.exists(), "it wrote under a writer");

        drop(writer);
        exporting
            .join()
            .expect("no panic")
            .expect("it runs once the writer has gone");
        assert!(out.is_file());
    }

    #[test]
    fn an_export_time_shows_in_the_offset_it_was_cut_under() {
        let at = |text: &str| Timestamp::parse(text).expect("a time");
        assert_eq!(
            display_of(at("2026-11-04T23:30:00-05:00")),
            "2026-11-04 23:30"
        );
        assert_eq!(display_of(at("2026-11-05T04:30:00Z")), "2026-11-05 04:30");
    }
}
