//! The facade the iPhone app calls. [M38 P4]
//!
//! The app's Dart holds layout and no golf (the M38 plan's rule 8), so every question the phone asks
//! about a shot is answered here or below here, by the crates the lab already trusts. This crate
//! adds no scoring, parsing or storage of its own: it is the one place those crates are shaped into
//! what a screen can show.
//!
//! # Why it holds no `flutter_rust_bridge`
//!
//! The bridge is `app/rust` (package `golf_bridge`), a shell of generated glue with its own lock and
//! its own `rust-toolchain.toml`, detached from this workspace by an empty `[workspace]` table, which
//! takes this crate as a path dependency (C1). The planning spike bound a plain external crate this
//! way, so nothing here needs to know the bridge exists. What that buys:
//!
//! - **The root lock never meets the bridge.** FRB's template resolves `tokio 1.34.0` where
//!   `crates/mcp` pins `=1.54.0`, so one lock could not hold both.
//! - **Neither do the root's Rust version nor the Windows box's `cargo test`.** `native_toolchain_rust`
//!   needs an exact channel, and a member of this workspace has to build wherever the workspace does.
//! - **Everything here is tested by `cargo test`**, in seconds and on any OS. A test through the bridge
//!   needs the Simulator, which only the phases that name it pay for (rule 15).
//!
//! # The boundary, which binds every phase after this one (rule 7)
//!
//! [`api`] holds only what crosses to Dart: free functions, and structs and plain enums of `String`,
//! `i64`, `f64`, `bool`, `Option`, `Vec` and tuples, returning `Result<T, String>` where they can
//! refuse.
//!
//! - **No enum carries fields.** That needs `freezed` and `build_runner` in the app (the spike's
//!   `MissingDep`), and an `Option` field says the same thing.
//! - **No `pub` helper goes in `api`**, because the codegen binds every one, and a helper bound is a
//!   helper Dart can call. Everything else lives outside `api`, here at the root or in a module of
//!   its own, as [`parse_now`] and [`Dirs`] do.
//!
//! # The one device, and the one threshold
//!
//! [`DEVICE`] and [`MIN_CONFIDENCE`] are constants because nothing on the phone may choose either
//! (D3, C18). Times cross as RFC 3339 text with an offset, and [`parse_now`] is the one place they
//! are read. Clubs cross as `ClubId`'s own spelling, and [`club_id`] is that place for them (C2).

use std::path::{Path, PathBuf};

use contracts::club::ClubId;
use contracts::swing::{ANALYSIS_VERSION, COMPARABLE_FROM};
use contracts::Timestamp;
use storage::corpus::EngineVersions;

pub mod api;
/// `golf-bay-report`'s arithmetic, `pub` for the bin and outside [`api`], so the codegen never meets it.
pub mod bay;
mod card;
mod export;
mod history;
mod profile;
mod record;
mod state;

pub use profile::profile_view;

/// The launch monitor the phone reads, as `crates/contracts/devices.json` and
/// `crates/screen/profiles.json` spell it.
///
/// Full Swing only, with no picker (D3): `hd_golf`'s profile stays in `crates/screen`, unoffered,
/// because no Vision evidence stands behind it.
pub const DEVICE: &str = "full_swing";

/// The confidence below which a read that passed every check is still flagged for review.
///
/// **M33's value, "provisional and vacuous"** (`spikes/vision-ocr/log.md`, and C18): it is the
/// rule's floor on an empty set, since no phone-mode cell differed from the truth at any threshold,
/// so the rule had nothing to test. "0.6 is enough" is not a finding of that run. It stays until the
/// bay session's confidences put `W_OCR` to the user (D8).
pub const MIN_CONFIDENCE: f64 = 0.6;

/// One time crossing from Dart or Swift, read or refused.
///
/// **One helper because the trap is on the caller's side.** Dart's `DateTime.now().toIso8601String()`
/// writes a local time with no offset, and that is the one spelling that must not get through: it
/// would stamp a shot with an instant nobody can recover, and it names a session by a date in no
/// known offset (C8). `Timestamp` refuses it, as it refuses everything outside the grammar every
/// time on disk is written in, and its message already says which rule failed, so it is passed on
/// as the `String` a facade function refuses with.
pub fn parse_now(text: &str) -> Result<Timestamp, String> {
    Timestamp::parse(text).map_err(|refused| refused.to_string())
}

/// One club crossing from Dart, read or refused. [M38 P5]
///
/// **`ClubId`'s own spelling and nothing else** (C2), which is why this is not
/// `contracts::club::parse_club`: that one is the tolerant parser for what a person types, and every
/// club Dart holds came from `all_clubs()` or a stored shot. `7 iron` reaching here is a bug in the
/// app, and reading it would hide the bug.
pub fn club_id(text: &str) -> Result<ClubId, String> {
    ClubId::ALL
        .into_iter()
        .find(|club| club.as_str() == text)
        .ok_or_else(|| format!("'{text}' is not a club, e.g. 7i or pw"))
}

/// What every read of the corpus on this phone passes, this build's pair (C23). It decides which
/// stored analyses count as current, so one place says so. [M38 P6]
pub fn engine_versions() -> EngineVersions {
    EngineVersions {
        installed: ANALYSIS_VERSION,
        comparable_from: COMPARABLE_FROM,
    }
}

/// Where everything lives under the data root, `<Documents>/processed/` on the phone (C7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dirs {
    /// `SwingBundleStore`'s root, each photo's entry under it.
    pub sessions: PathBuf,
    /// `ShotStore`'s root, the parsed shots keyed by each photo's hash.
    pub shots: PathBuf,
    /// `GolferStore`'s and `BagStore`'s root.
    pub golfers: PathBuf,
    /// The app's own directory: `state.json`, the numbers deleted (`crate::record`'s module doc), and
    /// the read diagnostics.
    ///
    /// **Dotted on purpose**: `golf-lab import-export` ignores and counts a dotted name, so the
    /// diagnostics travel in an export and never reach the lab's stores.
    pub app: PathBuf,
}

impl Dirs {
    /// The directories under `root`. Nothing is created, because each store makes its own.
    ///
    /// The three store names are the lab's `import_export::TREES`, which this crate cannot reach
    /// (`golf-lab` holds `ort`), so P13's round trip through `import-export` is what holds them equal.
    pub fn of(root: impl AsRef<Path>) -> Self {
        let root = root.as_ref();
        Self {
            sessions: root.join("sessions"),
            shots: root.join("shots"),
            golfers: root.join("golfers"),
            app: root.join(".app"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A time keeps the offset it was written in, because C8 names a session by the photo's date in
    /// its own offset: 20:42 in New York is already tomorrow in UTC.
    #[test]
    fn parse_now_reads_a_time_with_an_offset_and_keeps_that_offset() {
        let time = parse_now("2026-11-04T20:42:00-05:00").expect("an offset is present");
        assert_eq!(time.offset_minutes(), -300);
        assert_eq!(time.to_string(), "2026-11-04T20:42:00-05:00");
        assert_eq!(time, parse_now("2026-11-05T01:42:00Z").expect("UTC"));
    }

    /// What the two callers actually write for a time they know the offset of: Dart's
    /// `toUtc().toIso8601String()`, with and without microseconds, and Swift's
    /// `ISO8601DateFormatter`, with and without fractional seconds. A spelling refused here would
    /// refuse every shot from that path.
    #[test]
    fn parse_now_reads_what_dart_and_swift_write_for_a_time_they_know_the_offset_of() {
        for (written, utc_micros) in [
            ("2026-11-05T01:42:00.123456Z", 1_793_842_920_123_456),
            ("2026-11-05T01:42:00.123Z", 1_793_842_920_123_000),
            ("2026-11-05T01:42:00Z", 1_793_842_920_000_000),
            ("2026-11-04T20:42:00-05:00", 1_793_842_920_000_000),
            ("2026-11-04T20:42:00.123-05:00", 1_793_842_920_123_000),
        ] {
            let time = parse_now(written).unwrap_or_else(|refused| panic!("{refused}"));
            assert_eq!(time.utc_micros(), utc_micros, "{written}");
        }
    }

    /// The two naive spellings, the second of them Dart's `DateTime.now().toIso8601String()`. The
    /// refusal is the text a facade function throws, so it has to name the value and the offset.
    #[test]
    fn parse_now_refuses_a_naive_time_and_says_an_offset_is_what_is_missing() {
        for naive in ["2026-11-04T20:42:00", "2026-11-04T20:42:00.123456"] {
            let refused = parse_now(naive).expect_err(naive);
            assert!(refused.contains(naive), "{refused}");
            assert!(refused.contains("offset"), "{refused}");
        }
    }

    #[test]
    fn parse_now_refuses_what_is_not_rfc_3339_with_seconds() {
        for text in [
            "",
            "now",
            "2026-11-04",
            "2026-11-04 20:42:00Z",
            "2026-11-04T20:42Z",
            "2026-11-04T20:42:00z",
            "2026-11-04T20:42:00+0530",
            "2026-13-04T20:42:00Z",
        ] {
            assert!(parse_now(text).is_err(), "{text:?} was read");
        }
    }

    #[test]
    fn club_id_reads_every_clubs_own_spelling() {
        for club in ClubId::ALL {
            assert_eq!(club_id(club.as_str()), Ok(club));
        }
    }

    /// Each of these is a club to `parse_club`, so each is a spelling this could wrongly be
    /// loosened to read.
    #[test]
    fn club_id_refuses_the_spellings_only_a_person_types_and_names_the_text() {
        for text in ["7 iron", "7-iron", "7I", "Seven Iron", " driver ", ""] {
            let refused = club_id(text).expect_err(text);
            assert!(refused.contains(&format!("'{text}'")), "{refused}");
        }
    }

    #[test]
    fn dirs_names_the_three_stores_and_the_apps_own_dotted_directory_under_the_root() {
        let dirs = Dirs::of("/Documents/processed");
        assert_eq!(dirs.sessions, Path::new("/Documents/processed/sessions"));
        assert_eq!(dirs.shots, Path::new("/Documents/processed/shots"));
        assert_eq!(dirs.golfers, Path::new("/Documents/processed/golfers"));
        assert_eq!(dirs.app, Path::new("/Documents/processed/.app"));
    }
}
