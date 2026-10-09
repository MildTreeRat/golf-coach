//! The swing bundle manifest — which files have arrived for one swing. `storage/manifest.py`.
//! [M36 P8]
//!
//! A swing is complete once all three roles (face-on video, down-the-line video, shot-screen photo)
//! have arrived, and useful while it is still `collecting`. [`SwingManifest::status`] is computed,
//! never stored: nothing consumes a `ready` or `analyzed` signal, so persisting one would claim more
//! than is true, and an analysis worker that needs one can add a field as a pure addition.
//!
//! `player_id`, `club` and `mishit` were each added that way — optional, defaulted, read through the
//! same tolerant loader — which is why no manifest written before any of them ever needed a
//! migration: every one on disk still loads and answers `None`.

use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use contracts::club::ClubId;
use contracts::mishit::MishitVerdict;
use contracts::Timestamp;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Which camera angle or document an uploaded file represents.
///
/// `Ord` is declaration order, which is what a `BTreeMap` of roles iterates in. Nothing reads that
/// order as an answer: [`SwingManifest::missing_roles`] walks [`EXPECTED_ROLES`], and
/// [`crate::state::input_hashes`] keys by the wire name, which is the order Python's `sorted` over a
/// `StrEnum` gives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    FaceOn,
    DownTheLine,
    ShotScreen,
}

impl Role {
    /// The wire name (`role.value`).
    pub const fn as_str(self) -> &'static str {
        match self {
            Role::FaceOn => "face_on",
            Role::DownTheLine => "down_the_line",
            Role::ShotScreen => "shot_screen",
        }
    }

    /// `_DEFAULT_SUFFIX`: the suffix a role's file takes when the upload's name has none.
    pub const fn default_suffix(self) -> &'static str {
        match self {
            Role::FaceOn | Role::DownTheLine => ".mov",
            Role::ShotScreen => ".jpg",
        }
    }
}

/// Every role a complete swing has, in the order a missing one is reported.
pub const EXPECTED_ROLES: [Role; 3] = [Role::FaceOn, Role::DownTheLine, Role::ShotScreen];

/// `_MANIFEST_NAME`.
pub const MANIFEST_NAME: &str = "manifest.json";

/// One uploaded file, filling one role slot in a swing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoleFile {
    pub role: Role,
    /// Relative to the swing directory.
    pub filename: String,
    pub content_sha256: String,
    pub original_filename: String,
    pub content_type: String,
    pub size_bytes: i64,
    pub received_at: Timestamp,
    #[serde(default)]
    pub warnings: Vec<String>,
}

/// Everything known about one swing: identity plus whichever roles have arrived.
///
/// Fields in pydantic's declaration order, so a file this writes reads in the order a person
/// comparing it with frozen Python's expects; nothing depends on it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SwingManifest {
    pub swing_id: String,
    pub session_id: String,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
    /// Keyed by role. A key no [`Role`] names refuses the whole manifest, as pydantic's does, and a
    /// `role` field that disagrees with its key is kept as written: Python checks neither.
    #[serde(default)]
    pub roles: BTreeMap<Role, RoleFile>,

    /// Who swung it, stamped from the session cursor when the swing was created. `None` means
    /// nobody had selected a golfer yet, an expected state, since uploads are never blocked on it.
    /// **Write-once in practice**: the bundle store sets it only when it is `None`, so switching
    /// the cursor mid-session cannot rewrite swings already attributed to the first golfer.
    #[serde(default)]
    pub player_id: Option<String>,
    /// Which club hit it, stamped from the session cursor exactly as `player_id` is, and write-once
    /// for the same reason. `None` means only that the swing predates the field, because the
    /// upload route refuses a swing with no club (ADR-024 §5).
    #[serde(default)]
    pub club: Option<ClubId>,
    /// The golfer's own verdict on this swing's shot, never stamped from a cursor and never
    /// inferred. `None` is the common case: the automatic rule in `contracts::mishit` decides.
    #[serde(default)]
    pub mishit: Option<MishitVerdict>,
}

/// `Literal["collecting", "complete"]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BundleStatus {
    Collecting,
    Complete,
}

impl BundleStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            BundleStatus::Collecting => "collecting",
            BundleStatus::Complete => "complete",
        }
    }
}

impl SwingManifest {
    pub fn missing_roles(&self) -> Vec<Role> {
        EXPECTED_ROLES
            .into_iter()
            .filter(|role| !self.roles.contains_key(role))
            .collect()
    }

    pub fn status(&self) -> BundleStatus {
        if self.missing_roles().is_empty() {
            BundleStatus::Complete
        } else {
            BundleStatus::Collecting
        }
    }
}

/// Content hash of a file's bytes — the identity an upload is deduped by. Lowercase hex, as
/// `hexdigest()` writes it.
pub fn hash_bytes(data: &[u8]) -> String {
    hex(&Sha256::digest(data))
}

/// The same hash as [`hash_bytes`], read in 1 MiB chunks so a 200 MB clip never lands in memory.
pub fn hash_file(path: &Path) -> io::Result<String> {
    hash_file_in_chunks(path, 1 << 20)
}

/// [`hash_file`] at a chosen chunk size, Python's `chunk_bytes` keyword: the digest cannot depend
/// on it, and a test holds that.
pub fn hash_file_in_chunks(path: &Path, chunk_bytes: usize) -> io::Result<String> {
    let mut handle = fs::File::open(path)?;
    let mut digest = Sha256::new();
    let mut chunk = vec![0u8; chunk_bytes.max(1)];
    loop {
        let read = handle.read(&mut chunk)?;
        if read == 0 {
            break;
        }
        digest.update(&chunk[..read]);
    }
    Ok(hex(&digest.finalize()))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// A content-addressed filename for a role slot: `face_on.deadbeef1234.mov`.
///
/// The suffix is Python's `Path(original_filename).suffix`, which is not Rust's
/// `Path::extension`: a name ending in `.` (`clip.`) or starting with one and holding no other
/// (`.hidden`) has **no** suffix and takes the role's default, where `extension` answers `Some("")`
/// for the first. `photo.tar.gz` keeps `.gz`, and case is kept. Measured against CPython 3.13 for
/// those and for `..`, `...`, `a..b`, a trailing `/` and a trailing `/.` (the unit tests).
pub fn content_filename(role: Role, sha256: &str, original_filename: &str) -> String {
    let suffix = path_suffix(original_filename).unwrap_or(role.default_suffix());
    // `sha256[:12]`, by code point: a digest is ASCII, and slicing bytes would panic on one that
    // is not rather than answer what Python answers.
    let short: String = sha256.chars().take(12).collect();
    format!("{}.{short}{suffix}", role.as_str())
}

/// `PurePath(text).suffix`, or `None` where Python's is `''`.
///
/// The name is the last component that is neither empty nor `.`, as `pathlib` drops both; the
/// separators are `/` and `\` (the crate doc says why). Then the suffix is from the last `.`, when
/// that `.` is neither the name's first character nor its last.
fn path_suffix(text: &str) -> Option<&str> {
    let name = text
        .rsplit(['/', '\\'])
        .find(|part| !part.is_empty() && *part != ".")?;
    let dot = name.rfind('.')?;
    (dot > 0 && dot < name.len() - 1).then(|| &name[dot..])
}

pub fn manifest_path(swing_dir: &Path) -> PathBuf {
    swing_dir.join(MANIFEST_NAME)
}

/// Tolerant read: a missing, unreadable or invalid manifest is `None`, never an error. The crate
/// doc lists what "invalid" covers, and what pydantic reads that this refuses.
pub fn load_manifest(path: &Path) -> Option<SwingManifest> {
    let bytes = fs::read(path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

/// Atomic write (tmp, then rename), safe against a status page reading while it lands. Creates the
/// swing directory if it is not there yet.
pub fn save_manifest(manifest: &SwingManifest, path: &Path) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    crate::write_atomically(manifest, path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn noon() -> Timestamp {
        "2026-08-06T12:00:00Z".parse().unwrap()
    }

    fn role_file(role: Role, digest: &str) -> RoleFile {
        RoleFile {
            role,
            filename: format!("{}.{digest}.mov", role.as_str()),
            content_sha256: digest.to_string(),
            original_filename: "clip.mov".to_string(),
            content_type: "video/quicktime".to_string(),
            size_bytes: 1024,
            received_at: noon(),
            warnings: vec![],
        }
    }

    fn manifest(roles: &[Role]) -> SwingManifest {
        SwingManifest {
            swing_id: "1".to_string(),
            session_id: "2026-08-06".to_string(),
            created_at: noon(),
            updated_at: noon(),
            roles: roles.iter().map(|&r| (r, role_file(r, "abc"))).collect(),
            player_id: None,
            club: None,
            mishit: None,
        }
    }

    /// The manifest `test_manifest.py` pins as written before `player_id`, `club` and `mishit`
    /// existed: the bytes such a file holds, not a struct that carries the three fields today.
    const PRE_M9: &str = r#"{"swing_id": "1", "session_id": "2026-08-07-aaron1",
        "created_at": "2026-08-07T12:00:00Z", "updated_at": "2026-08-07T12:00:00Z", "roles": {}}"#;

    #[test]
    fn status_is_collecting_until_all_roles_are_present() {
        let one = manifest(&[Role::FaceOn]);
        assert_eq!(one.status(), BundleStatus::Collecting);
        assert_eq!(one.missing_roles(), [Role::DownTheLine, Role::ShotScreen]);

        let all = manifest(&EXPECTED_ROLES);
        assert_eq!(all.status(), BundleStatus::Complete);
        assert!(all.missing_roles().is_empty());
    }

    #[test]
    fn a_manifest_round_trips_through_disk_and_leaves_no_tmp() {
        let dir = tempfile::tempdir().unwrap();
        let path = manifest_path(&dir.path().join("2026-08-06").join("1"));
        let mut written = manifest(&[Role::FaceOn]);
        written.player_id = Some("aaron".to_string());
        written.club = Some(ClubId::SevenIron);
        written.mishit = Some(MishitVerdict::Confirmed);

        save_manifest(&written, &path).unwrap();

        assert_eq!(load_manifest(&path), Some(written));
        assert!(!path.with_extension("tmp").exists());
    }

    #[test]
    fn a_missing_or_corrupt_manifest_is_none_not_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = manifest_path(dir.path());
        assert_eq!(load_manifest(&path), None);
        fs::write(&path, "{not json").unwrap();
        assert_eq!(load_manifest(&path), None);
        assert_eq!(
            load_manifest(dir.path()),
            None,
            "a directory reads as nothing"
        );
    }

    /// The no-migration guarantee, for all three fields at once: the bytes on disk before any of
    /// them existed still load, and each answers `None`.
    #[test]
    fn a_manifest_written_before_the_three_optional_fields_still_loads() {
        let parsed: SwingManifest = serde_json::from_str(PRE_M9).unwrap();
        assert_eq!(
            (parsed.player_id, parsed.club, parsed.mishit),
            (None, None, None)
        );
    }

    /// P1's finding 5: a club, role or verdict no enum names drops the whole manifest, a missing
    /// required key does, a `null` where the field is not optional does, and an unknown key does not.
    #[test]
    fn what_drops_a_manifest_is_what_drops_it_in_python() {
        let base: serde_json::Value = serde_json::from_str(PRE_M9).unwrap();
        let with = |key: &str, value: serde_json::Value| {
            let mut doc = base.clone();
            doc[key] = value;
            serde_json::from_value::<SwingManifest>(doc).is_ok()
        };
        assert!(!with("club", json!("10i")));
        assert!(!with("mishit", json!("maybe")));
        assert!(!with("roles", json!({"side_on": {}})));
        assert!(!with("roles", json!(null)));
        assert!(!with("swing_id", json!(1)));
        assert!(with("player_id", json!(null)));
        assert!(with("future_field", json!({"anything": 1})));
        let mut missing = base.clone();
        missing.as_object_mut().unwrap().remove("updated_at");
        assert!(serde_json::from_value::<SwingManifest>(missing).is_err());
        assert!(serde_json::from_value::<SwingManifest>(json!([])).is_err());
    }

    #[test]
    fn hash_file_matches_hashing_the_bytes_at_any_chunk_size() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("clip.mov");
        let data: Vec<u8> = (0..=255u8).cycle().take(256 * 40).collect();
        fs::write(&path, &data).unwrap();
        for chunk in [1, 7, 1024, 4096, 1 << 20] {
            assert_eq!(
                hash_file_in_chunks(&path, chunk).unwrap(),
                hash_bytes(&data)
            );
        }
        assert_eq!(hash_file(&path).unwrap(), hash_bytes(&data));

        let empty = dir.path().join("empty.mov");
        fs::write(&empty, b"").unwrap();
        assert_eq!(
            hash_file(&empty).unwrap(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    /// `Path(name).suffix` as CPython 3.13 answered each, measured at M36 P8.
    #[test]
    fn the_suffix_is_pythons_not_rusts() {
        let cases = [
            ("clip.mov", Some(".mov")),
            ("CLIP.MOV", Some(".MOV")),
            ("photo.tar.gz", Some(".gz")),
            ("clip.", None),
            (".hidden", None),
            ("..", None),
            ("...", None),
            ("a..b", Some(".b")),
            ("", None),
            (".", None),
            ("foo.mov/", Some(".mov")),
            ("foo.mov/.", Some(".mov")),
            ("a/b.c", Some(".c")),
            (" .mov", Some(".mov")),
            ("x. ", Some(". ")),
            ("a.b ", Some(".b ")),
        ];
        for (name, python) in cases {
            assert_eq!(path_suffix(name), python, "{name:?}");
        }
        let digest = "0123456789abcdef";
        assert_eq!(
            content_filename(Role::ShotScreen, digest, "clip."),
            "shot_screen.0123456789ab.jpg"
        );
        assert_eq!(
            content_filename(Role::FaceOn, digest, "photo.tar.gz"),
            "face_on.0123456789ab.gz"
        );
        assert_eq!(
            content_filename(Role::DownTheLine, "short", ".hidden"),
            "down_the_line.short.mov"
        );
    }
}
