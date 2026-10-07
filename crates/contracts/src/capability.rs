//! Which `ShotData` fields a launch monitor can print, and which of those analysis may use.
//! [M32 P10]
//!
//! [ADR-034](../../../docs/decisions/034-shot-first-phone-first.md) §2's device capability model.
//! **It is the one module in this crate with no Python twin.** Frozen Python never gains it
//! (ADR-035 clause 4), and §M32 dropped the `contracts/capability.py` the pivot first planned, so the
//! crate's one-for-one naming rule has nothing to mirror here.
//!
//! # Declared, printed, blank: three words, applied in that order
//!
//! - **Declared** is what a device can print, as a [`DeviceCapability`] in `devices.json`, each field
//!   [`FieldUse::Analysed`] or [`FieldUse::ShownOnly`].
//! - **Printed** is what one golfer's screen shows, and it is narrower, because layouts vary per bay:
//!   the bay photos carry `Custom` and `Impact Position V` where the two reference photos carry
//!   `Bounce & Roll` (M31 P2). [`printed_on`] cuts it per shot and [`printed_fields`] takes the union
//!   over a golfer's shots, which is ADR-034 §2's *declared ∩ `fields_present`, across the golfer's
//!   own shots*.
//! - **Blank** is printed and empty on one shot. That is an `unscored` refusal, named
//!   (ADR-010 §2). A field that is not printed is never blank: it produces nothing downstream at all,
//!   because a golfer is never told about a stat their screen does not show.
//!
//! So the question this module answers for a consumer is "is this `None` a blank, or a tile that
//! was never there?", which a `None` metric cannot answer on its own.
//!
//! # The data is a file, and the file is in `crates/`
//!
//! `devices.json` sits beside this crate's `Cargo.toml` and is read by `include_str!`, which keeps
//! ADR-032 §5's one copy on disk. It is not beside `profiles.json` because nothing Python reads it.
//! Its field names are `ShotData` keys, never screen labels: turning a label into a key is the
//! parser's business (`crates/screen/profiles.json`, the Rust fork of the frozen lab's copy, M34).
//! `hd_golf`'s entry is held equal to that profile's targets by `crates/screen/tests/capability.rs`,
//! on that crate's side because this one cannot see it.

use std::collections::BTreeSet;
use std::sync::OnceLock;

use serde::Deserialize;

use crate::shot::ShotData;
use crate::ContractError;

/// ADR-032 §5: one copy on disk, read at compile time.
const DEVICES_JSON: &str = include_str!("../devices.json");

/// What analysis may do with a field a device prints.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldUse {
    /// May be measured and graded.
    Analysed,
    /// Displayed as printed, never analysed. The field's `note` says why, and every one carries
    /// one: a stat the golfer can see and the app refuses to grade needs a reason it can show.
    ///
    /// A variant rather than leaving the field undeclared, because the two are different promises.
    /// Undeclared means the screen does not show it, so nothing may mention it. Shown-only means the
    /// screen does show it, so the app displays it and says why it is not judged.
    ShownOnly,
}

/// One field a device declares.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct DeclaredField {
    /// A `ShotData` key.
    pub field: String,
    /// `"use"` on disk, which is a Rust keyword.
    #[serde(rename = "use")]
    pub field_use: FieldUse,
    /// Why, where the declaration needs a reason: always on [`FieldUse::ShownOnly`], and on an
    /// analysed field only when the parsers do not all deliver what it declares
    /// (`impact_position_v`, which the Rust parser locates from M34 and the frozen lab's never does).
    #[serde(default)]
    pub note: Option<String>,
}

/// Every field one launch monitor can print.
///
/// `added` is on disk and not here, like `store.rs`'s `BenchmarkRange.added`: nothing reads it, and
/// serde ignores unknown keys. `source` stays, because it is the provenance ADR-022 asks committed
/// data to carry, and the tests hold every entry to having one.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct DeviceCapability {
    /// The name [`device_of`] gives a shot from this device.
    pub device: String,
    /// Where this declaration came from.
    pub source: String,
    /// In the order the file lists them, which for `hd_golf` is its profile's tile order.
    pub fields: Vec<DeclaredField>,
}

impl DeviceCapability {
    /// The fields this device declares, as `ShotData` keys.
    pub fn declared(&self) -> impl Iterator<Item = &str> {
        self.fields.iter().map(|declared| declared.field.as_str())
    }

    /// What analysis may do with `field`, or `None` when the device does not print it.
    pub fn use_of(&self, field: &str) -> Option<FieldUse> {
        self.fields
            .iter()
            .find(|declared| declared.field == field)
            .map(|declared| declared.field_use)
    }
}

#[derive(Debug, Deserialize)]
struct DevicesFile {
    devices: Vec<DeviceCapability>,
}

/// Every device `devices.json` declares, parsed once. `store.rs`'s `OnceLock`, for its reason.
pub fn devices() -> &'static [DeviceCapability] {
    static DEVICES: OnceLock<Vec<DeviceCapability>> = OnceLock::new();
    DEVICES.get_or_init(|| {
        let parsed: DevicesFile = serde_json::from_str(DEVICES_JSON)
            .expect("devices.json ships in this crate and parses");
        parsed.devices
    })
}

/// The declaration for `device`, or a refusal.
///
/// **Refused rather than defaulted.** A shot from a device nobody declared could print anything,
/// and guessing a default would either hide a stat the golfer can see or grade one they cannot. So
/// an R10 shot is refused here until an `r10` entry exists, and that refusal is the reminder to
/// write it.
pub fn capability_for(device: &str) -> Result<&'static DeviceCapability, ContractError> {
    devices()
        .iter()
        .find(|capability| capability.device == device)
        .ok_or_else(|| ContractError {
            field: "DeviceCapability.device".to_string(),
            problem: format!("{device:?} is not a device devices.json declares"),
        })
}

/// The device a shot came from: its provenance's `device` where it has one, else its source's wire
/// name.
///
/// **This is the one definition of that rule.** It was born inside `engine::shot_measurements`,
/// which writes `launch_monitor:{device}` into every shot measurement's `source`, and that string
/// is compared exactly by the corpus `measurements` stage vectors. That function now calls this
/// rather than restating it, so those vectors gate this rule too.
pub fn device_of(shot: &ShotData) -> &str {
    match shot.provenance.as_ref() {
        Some(provenance) => provenance.device.as_str(),
        None => shot.source.as_str(),
    }
}

/// The fields this shot's screen printed: what its device declares, cut to the tiles the shot shows.
///
/// - **A screen shot with `fields_present`** prints declared ∩ `fields_present`. A field whose tile
///   the parse located is printed even when its value is blank, and that blank is a refusal to name.
///   A field whose tile the layout lacks is not printed, and is never counted as blank.
/// - **A direct feed**, a shot with no provenance, prints everything its device declares, because
///   it was never parsed, so there are no tiles to locate.
/// - **An unstamped screen shot** (`fields_present: None`, which is every stored shot until M29's
///   re-read) is read conservatively, as declared ∩ the fields it holds a value for. So a blank on
///   such a shot is left out rather than named, which under-reports blanks on the stored shots
///   until that re-read. It never invents one, and ADR-010 §2 puts those two errors in that order.
///
/// Refuses a shot whose device is not declared ([`capability_for`]).
pub fn printed_on(shot: &ShotData) -> Result<BTreeSet<&'static str>, ContractError> {
    let declared = capability_for(device_of(shot))?.declared();
    Ok(match shot.provenance.as_ref() {
        None => declared.collect(),
        Some(provenance) => match provenance.fields_present.as_deref() {
            Some(present) => declared
                .filter(|field| present.iter().any(|located| located == field))
                .collect(),
            None => {
                let held = fields_holding_a_value(shot);
                declared.filter(|field| held.contains(*field)).collect()
            }
        },
    })
}

/// The union of [`printed_on`] over a golfer's shots: a field is printed for them if any of their
/// screens shows its tile.
///
/// A union and not an intersection, because the golfer who moves between two bays sees each
/// layout's tiles on that bay's shots, and [`printed_on`] has already kept each shot to its own.
/// One undeclared device anywhere refuses the whole set, since a set computed over the shots that
/// happened to be declared would describe a golfer who does not exist.
pub fn printed_fields<'a>(
    shots: impl IntoIterator<Item = &'a ShotData>,
) -> Result<BTreeSet<&'static str>, ContractError> {
    let mut printed = BTreeSet::new();
    for shot in shots {
        printed.extend(printed_on(shot)?);
    }
    Ok(printed)
}

/// The `ShotData` keys this shot holds a non-null value for.
///
/// Read off the serialized shot rather than a hand-written match over field names, so the struct
/// stays the only list of its fields and a key M34 adds is covered without an edit here. A NaN
/// metric serializes as `null` and so reads as holding nothing, which is right: a value that is not
/// a number was not printed as one.
fn fields_holding_a_value(shot: &ShotData) -> BTreeSet<String> {
    let written =
        serde_json::to_value(shot).expect("a ShotData serializes: every key it writes is a string");
    written
        .as_object()
        .map(|object| {
            object
                .iter()
                .filter(|(_, value)| !value.is_null())
                .map(|(key, _)| key.clone())
                .collect()
        })
        .unwrap_or_default()
}
