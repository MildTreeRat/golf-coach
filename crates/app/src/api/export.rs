//! How a session's shots leave the phone: one cumulative zip of the stored layout, and what Settings
//! says about the last one. [M38 P13]
//!
//! The shapes Dart sees and two one-line calls (rule 7). `crate::export` holds what they do and why.

/// What one export wrote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportSummary {
    /// The zip, at the path it was asked for.
    pub path: String,
    /// The time the export was given, RFC 3339 with its offset. **What to hand `set_state` as
    /// `last_export_at`, with `shots` as `last_export_shots`, once the share has gone**: the shots
    /// since are counted from when this zip was cut, and not from when the share sheet closed.
    pub exported_at: String,
    /// The photo entries the zip holds, as [`ExportStatus::shots`] counts them.
    pub shots: i64,
    /// Every file it holds, the diagnostics the lab ignores among them.
    pub files: i64,
    /// The zip's own size.
    pub bytes: i64,
}

/// What Settings shows about exporting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportStatus {
    /// `None` before any export, as `state.json` holds it.
    pub last_export_at: Option<String>,
    /// `2026-11-05 11:00`, in the offset the export was cut under, since Dart formats nothing (rule 8).
    pub last_export_display: Option<String>,
    /// How many shots that export held.
    pub last_export_shots: Option<i64>,
    /// Every photo entry on this phone, which is what the next export will hold, since each one is
    /// cumulative (C16).
    pub shots: i64,
    /// Entries opened since the last export, and all of them before there has been one. Counted from
    /// the entries' own times and not by subtracting, so a stray deleted and a photo retaken reads 1
    /// and not 0.
    pub shots_since: i64,
}

/// Write everything C16 names to a zip at `out_path`, replacing any there, and answer what it holds.
/// **`state.json` stays home.** Nothing else on the phone moves, `last_export_at` included, since an
/// export the share sheet then cancels has not left: [`ExportSummary::exported_at`] says what to save.
///
/// `Err` is a sentence to show, and nothing is left at `out_path` that was not there. `now` refuses a
/// time with no offset, as every time crossing from Dart does. A path inside `root` is refused, and so
/// is a phone with no golfer on it yet, whose zip the lab would refuse.
pub fn export_sessions(
    root: String,
    out_path: String,
    now: String,
) -> Result<ExportSummary, String> {
    crate::export::export(&root, &out_path, &now)
}

/// The last export and the shots since. Never refuses, as `get_state` does not.
pub fn export_status(root: String) -> ExportStatus {
    crate::export::status(&root)
}
