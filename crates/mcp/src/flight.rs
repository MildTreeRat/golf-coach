//! The flight of the shot a swing was hit with, for a reader with no canvas.
//! `src/golf_coach/mcp/flight.py`. [M29 P14]
//!
//! Nothing can be *shown* to a model, so this sends the six numbers, where each came from, and
//! every sentence they have to be read with, and no path at all. The flight itself is
//! `crates/analysis`' ([`fly_shot`]), and so are the sentences ([`caveats_for`]) and the printed
//! comparison ([`compare_to_printed`]): this module is the view over them, as Python's is over
//! `analysis/flight_measure.py` and `analysis/flight_caveats.py`.
//!
//! **It re-flies rather than reading `analysis.json`**, for Python's reason: two of a flight's
//! inputs, the club's declared loft and the golfer's handedness, live in *editable* artifacts, so
//! the flight recorded when the engine ran and the flight the same shot has today can differ.
//! `differs_from_recorded` is that difference made visible, with the sentence saying which of two
//! causes is in play ([`recorded_reading`]).
//!
//! **The resolution is the route's**: the loft through `storage::flight_inputs::loft_for_club`,
//! from the manifest already in hand, and never through the corpus-wide photo join, which names one
//! swing per photo and would tell the other two swings sharing a photo that their screen was never
//! read ([`shot_for`]).
//!
//! **A refusal is an answer.** Most shots on disk cannot be flown, and a tool that reported those as
//! a miss would teach a model this repo is broken. [`NotFound`] is for a swing id nothing carries.
//! No shot screen, an unread one, and a carry no spin reaches are three sentences that all arrive
//! with `flew: false`, and only the third carries `unscored`, because only the engine records it.
//!
//! # Where this answers differently from Python
//!
//! Nowhere in the flight: the numbers, the refusals, the comparison and the caveats are frozen
//! Python's, to the character on every recorded call (`tests/tools.rs`). The one difference is the
//! generation [`recorded_reading`] counts as current, `COMPARABLE_FROM` onwards rather than the
//! installed engine alone, which its own doc argues and `crate::family::DECLARED` declares.

use std::path::Path;

use analysis::benchmarks::flight_model::load_flight_model;
use analysis::flight::DEFAULT_STEP_S;
use analysis::flight_caveats::caveats_for;
use analysis::flight_infer::ShotFlight;
use analysis::flight_measure::{
    compare_to_printed, flight_unscored, fly_shot, FlownShot, FLIGHT_MEASUREMENTS, FLIGHT_SOURCE,
};
use contracts::shot::ShotData;
use contracts::swing::{ANALYSIS_VERSION, COMPARABLE_FROM};
use contracts::unscored::spec_for;
use serde::Serialize;
use serde_json::Number;
use storage::bundle_store::SwingBundleStore;
use storage::flight_inputs::loft_for_club;
use storage::golfer_store::GolferStore;
use storage::manifest::Role;

use crate::query::{self, wire_name, NotFound, SimulatedView};
use crate::source::ShotSource;

// -------------------------------------------------------------------------------------------------
// The views, each in its pydantic model's field order
// -------------------------------------------------------------------------------------------------

/// One thing this flight could not produce, with the case it fell into. Not `UnscoredView`, which
/// drops `detail`: here the detail *is* the finding, since `127.0 yd, case two_branches` and
/// `121.8 yd, case above_peak` both collapse onto `carry_unreachable`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FlightRefusalView {
    pub name: String,
    pub reason: String,
    pub why: String,
    pub detail: String,
    /// Always false on a flight: the video was never an input to it.
    pub refilming_helps: bool,
}

/// The five conditions the ball left on, whichever way each was arrived at.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LaunchView {
    pub ball_speed_mph: f64,
    pub launch_angle_deg: f64,
    pub spin_rpm: f64,
    pub launch_direction_deg: f64,
    /// Zero is a flight drawn flat as often as an axis measured at zero; `AxisView` tells them apart.
    pub spin_axis_deg: f64,
}

/// The spin, its provenance and the cap it sits under, never one without the others.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SpinView {
    pub rpm: Option<f64>,
    /// `measured` (off the screen) or `inferred` (solved).
    pub source: Option<String>,
    /// Which of the spin solve's shapes the printed carry landed on, whenever a solve ran.
    pub case: Option<String>,
    pub cap_rpm: Option<f64>,
    pub at_cap: Option<bool>,
    pub detail: String,
}

/// How far the ball was tilted, which way it curved, and whether those two agree.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AxisView {
    /// `None` is a flight drawn in the vertical plane, not a straight shot.
    pub spin_axis_deg: Option<f64>,
    pub source: Option<String>,
    pub curve_direction: Option<String>,
    pub screen_shape: Option<String>,
    pub sign_disagrees: bool,
    pub curve_is_drawn: bool,
    pub why: String,
}

/// One simulated number beside the printed one it is about, and whether that is a check.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ComparisonView {
    pub simulated_name: String,
    pub measured_name: String,
    pub quantity: String,
    pub simulated: f64,
    pub measured: f64,
    pub difference: f64,
    pub comparable: bool,
    pub reading: String,
}

/// One of the six where the stored artifact and today's flight do not say the same thing.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RecordedGap {
    pub name: String,
    /// What `analysis.json` holds; `None` is an artifact with no such row.
    pub recorded: Option<f64>,
    /// What the shot flies now; `None` is a flight refused now.
    pub flown: Option<f64>,
    pub difference: Option<f64>,
}

/// One stored swing's shot, flown, or every reason it was not, in the same shape.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FlightView {
    pub session_id: String,
    pub swing_id: String,
    pub shot_id: Option<String>,
    pub club: Option<String>,
    pub flew: bool,
    /// The provenance of every number under `simulated`, versioned with the coefficient table.
    pub source: String,
    pub simulated: Vec<SimulatedView>,
    pub reason: Option<String>,
    pub detail: String,
    pub unscored: Vec<FlightRefusalView>,
    pub launch: Option<LaunchView>,
    pub spin: Option<SpinView>,
    pub axis: Option<AxisView>,
    pub comparison: Vec<ComparisonView>,
    pub caveats: Vec<String>,
    pub differs_from_recorded: Vec<RecordedGap>,
    pub recorded_reading: String,
}

// -------------------------------------------------------------------------------------------------
// The miss and the two silences
// -------------------------------------------------------------------------------------------------

/// A swing id nothing on disk carries, the one miss this tool has: `query::missing_swing`'s own
/// sentence, because a wrong id must read the same whichever tool it was typed into.
pub fn missing_swing_flight(session_id: &str, swing_id: &str) -> NotFound {
    query::missing_swing(session_id, swing_id)
}

/// Said instead of a flight when the swing arrived without a shot screen. Not a re-film: ball
/// flight is simulated from what the launch monitor printed, never from the video.
pub const NO_SHOT_SCREEN: &str = "This swing has no shot-screen photo, so there are no launch \
                                  conditions to fly. Ball flight is simulated from what the \
                                  launch monitor printed, never from the video.";

/// Said when the screen it arrived with has not been read.
///
/// **Its last clause is Rust's** [M29 P34]: frozen Python's ends on
/// [`UNREAD_SHOT_SCREEN_FROZEN_TAIL`], naming `scripts/import_shot_screens.py`, which P34 deleted;
/// this ends on [`UNREAD_SHOT_SCREEN_TAIL`], the verb that replaced it. Declared at
/// `simulate_flight`'s `detail` in `crate::family`, as P32's retag note is at `get_bag_profile`'s.
pub const UNREAD_SHOT_SCREEN: &str = "This swing's shot screen has not been read yet, so its \
                                      launch conditions are not on file. Analyzing the swing \
                                      reads it, or it can be imported with golf-lab \
                                      import-photos.";

/// How [`UNREAD_SHOT_SCREEN`] ends in frozen Python's `flight.py`, and in every case it recorded.
pub const UNREAD_SHOT_SCREEN_FROZEN_TAIL: &str = "scripts/import_shot_screens.py.";

/// How [`UNREAD_SHOT_SCREEN`] ends here.
pub const UNREAD_SHOT_SCREEN_TAIL: &str = "golf-lab import-photos.";

// -------------------------------------------------------------------------------------------------
// The flight
// -------------------------------------------------------------------------------------------------

/// Fly the shot this swing was hit with, or say which of the silences applies. `flight_for_swing`.
///
/// `None` only for no such swing, whose sentence is [`missing_swing_flight`]; every refusal is a
/// view. `golfers_dir` is optional rather than gating the tool, as Python's is: a shot whose screen
/// printed its spin needs neither a bag nor a handedness. Without a registry the loft resolves to
/// no bag entry and the axis refuses, which is a flat flight rather than no flight.
pub fn flight_for_swing(
    sessions_dir: &Path,
    source: &ShotSource,
    session_id: &str,
    swing_id: &str,
    golfers_dir: Option<&Path>,
) -> Option<FlightView> {
    let manifest = SwingBundleStore::new(sessions_dir).get_swing(session_id, swing_id)?;
    let mut view = FlightView {
        session_id: session_id.to_string(),
        swing_id: swing_id.to_string(),
        shot_id: None,
        club: manifest.club.map(|club| club.as_str().to_string()),
        flew: false,
        source: FLIGHT_SOURCE.to_string(),
        simulated: Vec::new(),
        reason: None,
        detail: String::new(),
        unscored: Vec::new(),
        launch: None,
        spin: None,
        axis: None,
        comparison: Vec::new(),
        caveats: Vec::new(),
        differs_from_recorded: Vec::new(),
        recorded_reading: String::new(),
    };

    let Some(role_file) = manifest.roles.get(&Role::ShotScreen) else {
        view.detail = NO_SHOT_SCREEN.to_string();
        return Some(view);
    };
    let Some(shot) = shot_for(source, &role_file.content_sha256) else {
        view.detail = UNREAD_SHOT_SCREEN.to_string();
        return Some(view);
    };

    // The two things a shot borrows from the swing it was hit with, off the manifest in hand.
    let player_id = manifest.player_id.as_deref();
    let loft_deg = loft_for_club(player_id, manifest.club, golfers_dir).ok();
    let golfer = match (golfers_dir, player_id) {
        (Some(golfers_dir), Some(player_id)) if !player_id.is_empty() => {
            GolferStore::new(golfers_dir).get(player_id)
        }
        _ => None,
    };
    let model = load_flight_model();
    let flown = fly_shot(
        &shot,
        loft_deg,
        golfer.map(|golfer| golfer.handedness),
        model,
        DEFAULT_STEP_S,
    );
    fill(&mut view, &shot, &flown);
    compare_to_recorded(&mut view, sessions_dir, session_id, swing_id);
    Some(view)
}

/// The parse filed under this swing's shot photo, or `None` if nothing has read it. `_shot_for`.
///
/// A scan of the source's oldest-first stream for the photo's hash, as `get_shot`'s is for an id,
/// and so the first parse filed under that photo wins. Not the corpus's photo join: that answers
/// "which swing was this shot hit on" and names one swing per photo, which is right for counting a
/// shot once and wrong from this side, where one photo attached to three swings must fly for all
/// three. No OCR runs; an unread photo is a `None`, which is the answer and not an error.
fn shot_for(source: &ShotSource, photo_sha256: &str) -> Option<ShotData> {
    source.stream().find(|shot| {
        shot.provenance
            .as_ref()
            .and_then(|provenance| provenance.image_sha256.as_deref())
            == Some(photo_sha256)
    })
}

/// Fill one view from a flight that ran, refused, or could not be resolved at all. `_fill`.
fn fill(view: &mut FlightView, shot: &ShotData, flown: &FlownShot) {
    let resolved = flown.resolved.as_ref();
    view.shot_id = Some(shot.shot_id.clone());
    view.flew = flown.flew();
    view.reason = flown.reason.map(|reason| wire_name(&reason));
    view.detail = flown.detail.clone();
    view.simulated = simulated(flown);
    view.unscored = flight_unscored(flown)
        .into_iter()
        .map(|entry| {
            let spec = spec_for(entry.reason);
            FlightRefusalView {
                name: entry.name,
                reason: wire_name(&entry.reason),
                why: spec.summary.to_string(),
                detail: entry.detail,
                refilming_helps: spec.refilming_helps,
            }
        })
        .collect();
    view.launch = launch(resolved);
    view.spin = spin(resolved);
    view.axis = axis(resolved);
    view.comparison = compare_to_printed(flown, shot)
        .into_iter()
        .map(|row| ComparisonView {
            simulated_name: row.simulated_name.to_string(),
            measured_name: row.measured_name.to_string(),
            quantity: row.quantity.to_string(),
            simulated: row.simulated,
            measured: row.measured,
            difference: row.difference,
            comparable: row.comparable,
            reading: row.reading,
        })
        .collect();
    // A refused flight owes none of the caveats: each is a sentence about a line that was drawn.
    // What it owes instead is `reason` and `detail`. Python's keyword default is sea level.
    if let (Some(flight), Some(resolved)) = (flown.flight.as_ref(), resolved) {
        view.caveats = caveats_for(flight, resolved, load_flight_model(), 0.0);
    }
}

/// The six, read through the registry the stored artifact is written from, in its order. An
/// absent one is dropped rather than sent as null: the conditions two of them record on are the
/// subject of `unscored`, and a null would be a second, quieter way of saying so. `_simulated`.
fn simulated(flown: &FlownShot) -> Vec<SimulatedView> {
    FLIGHT_MEASUREMENTS
        .iter()
        .filter_map(|(name, row)| {
            (row.measure)(flown).map(|value| SimulatedView {
                name: (*name).to_string(),
                value,
                unit: row.unit.to_string(),
                source: FLIGHT_SOURCE.to_string(),
                detail: row.detail.to_string(),
            })
        })
        .collect()
}

fn launch(resolved: Option<&ShotFlight>) -> Option<LaunchView> {
    let launch = resolved?.launch.as_ref()?;
    Some(LaunchView {
        ball_speed_mph: launch.ball_speed_mph,
        launch_angle_deg: launch.launch_angle_deg,
        spin_rpm: launch.spin_rpm,
        launch_direction_deg: launch.launch_direction_deg,
        spin_axis_deg: launch.spin_axis_deg,
    })
}

fn spin(resolved: Option<&ShotFlight>) -> Option<SpinView> {
    let resolved = resolved?;
    let inferred = resolved.spin.as_ref();
    Some(SpinView {
        rpm: resolved.spin_rpm(),
        source: resolved
            .spin_source
            .map(|source| source.as_str().to_string()),
        case: inferred.map(|spin| spin.solution.case.as_str().to_string()),
        cap_rpm: inferred.map(|spin| spin.cap_rpm()),
        at_cap: inferred.map(|spin| spin.at_cap()),
        detail: resolved.detail.clone(),
    })
}

fn axis(resolved: Option<&ShotFlight>) -> Option<AxisView> {
    let resolved = resolved?;
    let axis = &resolved.axis;
    Some(AxisView {
        spin_axis_deg: axis.spin_axis_deg,
        source: axis.source.map(|source| source.as_str().to_string()),
        curve_direction: axis.curve_direction.map(|shape| wire_name(&shape)),
        screen_shape: axis.screen_shape.map(|shape| wire_name(&shape)),
        sign_disagrees: axis.sign_disagrees(),
        curve_is_drawn: resolved.curve_is_drawn(),
        why: if axis.spin_axis_deg.is_none() {
            axis.detail.clone()
        } else {
            String::new()
        },
    })
}

// -------------------------------------------------------------------------------------------------
// Today's flight beside the recorded one
// -------------------------------------------------------------------------------------------------

/// Two causes, and a reader has to be told which is in play before being told a number moved.
/// `_STALE_READING`: repaired by re-analysing the swing.
pub const STALE_READING: &str = "This artifact was written by an older engine, so the difference \
                                 is the engine and not the shot. Re-analyze the swing to bring the \
                                 recorded flight up to date.";

/// `_EDITED_READING`: an input that lives in an editable artifact was edited after the engine ran,
/// and no version changed because none of the *uploads* did.
pub const EDITED_READING: &str = "This artifact is current, so the difference is an input that \
                                  lives outside it: the club's declared loft or the golfer's \
                                  handedness was edited after the swing was analyzed. Nothing \
                                  about the swing changed, and re-analyzing it is what makes the \
                                  stored numbers agree again.";

/// Which of the two causes an artifact's disagreement is, by the engine generation that wrote it.
/// Python's `stored.analysis_version == ANALYSIS_VERSION`, **widened down to [`COMPARABLE_FROM`]**,
/// and the one place this module answers differently from Python. [M29 P14]
///
/// The stale sentence says *the difference is the engine and not the shot*. Python's test of that
/// is "written by the installed engine", and asked of Rust's 17 it would say so of every artifact
/// on disk, all at 16, though 16 -> 17 moved no number: a declared 3 wood's loft would be blamed on
/// an engine that changed nothing. `COMPARABLE_FROM` is the generation from which a stored number
/// still means what today's would (`contracts::swing`), the question the corpus asks before it
/// pools a swing (ADR-024's M36 addendum), and so the question the career tools on this same server
/// already ask. So an artifact from `COMPARABLE_FROM` to this engine reads as current, and its
/// disagreement as an edited input, which over every artifact frozen Python writes (16) is Python's
/// own answer.
///
/// - **What moves.** A recorded artifact older than frozen Python's engine and not older than
///   `COMPARABLE_FROM`, which Python reads as stale and this as edited: `tools/flight`'s version-15
///   one, since no flight number has moved since 15. Declared in `crate::family::DECLARED`.
/// - **What stays Python's.** An artifact from before `COMPARABLE_FROM`, from a newer engine than
///   this one, or with no readable version still reads as stale, as `==` reads them.
/// - **The one false case.** A version-14 artifact is within the range and predates the flight
///   family (14 -> 15 added it), so a flight it never recorded would read as an edited input. None
///   exists in `data/` (`COMPARABLE_FROM`'s doc), M29 P29 re-runs every stored swing at this engine,
///   and the phone writes this engine's or later.
pub fn recorded_reading(analysis_version: Option<&Number>) -> &'static str {
    match analysis_version.and_then(Number::as_i64) {
        Some(version) if (COMPARABLE_FROM..=ANALYSIS_VERSION).contains(&version) => EDITED_READING,
        _ => STALE_READING,
    }
}

/// Set today's flight beside the one `analysis.json` holds, and say why they can differ.
/// `_compare_to_recorded`.
///
/// The stored numbers come back through [`query::get_swing`], which already owns the tolerant read
/// and the split that keeps a simulated number out of `measurements`, rather than through a second
/// reader of the artifact. Both sides are rounded to the four decimals the engine stores before
/// they are compared, as Python's are, or every number would read as moved on every swing.
fn compare_to_recorded(
    view: &mut FlightView,
    sessions_dir: &Path,
    session_id: &str,
    swing_id: &str,
) {
    let Some(stored) = query::get_swing(sessions_dir, session_id, swing_id) else {
        return;
    };
    // `{row.name: row.value for row in …}`: a repeated name keeps the last value.
    let by_name = |rows: &[SimulatedView], name: &str| {
        rows.iter()
            .rev()
            .find(|row| row.name == name)
            .map(|row| row.value)
    };
    let mut gaps = Vec::new();
    for (name, _) in FLIGHT_MEASUREMENTS {
        let (here, there) = (
            by_name(&view.simulated, name),
            by_name(&stored.simulated, name),
        );
        if let (Some(here), Some(there)) = (here, there) {
            if pyfmt::round_to(here, 4) == pyfmt::round_to(there, 4) {
                continue;
            }
        }
        if here.is_none() && there.is_none() {
            continue;
        }
        gaps.push(RecordedGap {
            name: (*name).to_string(),
            recorded: there,
            flown: here,
            difference: here
                .zip(there)
                .map(|(here, there)| pyfmt::round_to(here - there, 4)),
        });
    }
    if gaps.is_empty() {
        return;
    }
    view.differs_from_recorded = gaps;
    view.recorded_reading = recorded_reading(stored.analysis_version.as_ref()).to_string();
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The four sentences are one paragraph each, as Python's adjacent string literals join.
    #[test]
    fn the_silences_and_readings_are_single_spaced_paragraphs() {
        for text in [
            NO_SHOT_SCREEN,
            UNREAD_SHOT_SCREEN,
            STALE_READING,
            EDITED_READING,
        ] {
            assert!(!text.contains("  ") && !text.contains('\n'), "{text}");
            assert!(text.ends_with('.'), "{text}");
        }
    }

    /// The reading's range, edge by edge. No recorded artifact is older than 15, so this is the only
    /// gate on the stale branch below `COMPARABLE_FROM` and above this engine.
    #[test]
    fn an_artifact_reads_as_current_from_comparable_from_to_this_engine() {
        let reading = |version: serde_json::Value| match version {
            serde_json::Value::Number(n) => recorded_reading(Some(&n)),
            _ => recorded_reading(None),
        };
        use serde_json::json;
        assert_eq!(reading(json!(COMPARABLE_FROM - 1)), STALE_READING);
        assert_eq!(reading(json!(COMPARABLE_FROM)), EDITED_READING);
        assert_eq!(
            reading(json!(16)),
            EDITED_READING,
            "frozen Python's own engine"
        );
        assert_eq!(reading(json!(ANALYSIS_VERSION)), EDITED_READING);
        assert_eq!(reading(json!(ANALYSIS_VERSION + 1)), STALE_READING);
        assert_eq!(
            reading(json!(null)),
            STALE_READING,
            "no version is Python's stale"
        );
        assert_eq!(reading(json!(u64::MAX)), STALE_READING);
    }
}
