//! The swing analysis entry point. [M22 P8b]
//!
//! The port of `analysis/engine.py`, and it arrived in four pieces. **P5b brought the three
//! `measurements` groups that read only face-on pose** — [`pose_measurements`], [`placements`] and
//! [`pivot_measurements`]. **P6 brought `analyze_swing_bundle` itself**: the checkpoint loop,
//! `unscored`, the scoring policy, [`windowed`]/[`shifted`] and the `feedback` seam. **P7 brought the
//! second view**: [`dtl_placements`], the down-the-line pivot rows, `align_swings`' whole result and
//! [`without_contradicted_scores`]. **P8b brings the outcome**: [`shot_measurements`],
//! [`flight_measurements`] and the `fly_shot` call whose refusals land in `unscored` — the last two
//! sevenths of the `measurements` stage, and what takes the fifteen corpus vectors into the
//! whole-bundle gate.
//!
//! That split is the phase boundary and not a convenience: `conformance.py::run_stages` records
//! `measurements` as **seven groups of one flat list**, and each group is a phase's worth of work.
//! Shipping a group with its gate is the rule ADR-032 §2 sets; the alternative — a gate test that
//! rebuilds the grouping itself — is the second copy of `_measurements` that `run_stages`' own
//! docstring refuses to write.
//!
//! # Recorded, not judged
//!
//! Everything here produces a `Measurement`, which has no band, no `passed` and no path to
//! `overall_score`. That is the firewall ADR-010 §2 puts around a quantity with no population
//! behind it, and it is what lets a new metric be measured across the corpus *before* a band for it
//! exists. [`crate::checkpoints`] is the other half, and the two never meet.
//!
//! # Where the names come from
//!
//! Not from here. [`contracts::placements`] and [`contracts::pivots`] carry the name, unit and (for
//! pivots) detail of every row below, because `caveats.py` derives the prose every MCP client and
//! every coaching call reads out of those same registries — and a name typed at this call site
//! could be renamed without the warning about it following. That is the M6.5 failure both modules
//! were written to stop.
//!
//! # `pyfmt::g` here is invisible today, exactly as P5 found it in the sentences
//!
//! Two placement `detail`s interpolate a percentile with `:g`, and replacing [`g`] with Rust's `{}`
//! passes all 21 vectors, every test in this workspace and the whole Python suite. That is not
//! luck and it is not a hole: a percentile is `round(x, 1)` clamped into `[10, 90]`, so it carries
//! at most three significant digits and never reaches either boundary where `%g` switches to an
//! exponent — and Rust's `{}` drops a trailing `.0` the same way `%g` does. The two functions agree
//! on the whole reachable domain. They stop agreeing the moment something interpolates a value that
//! is not a clamped percentile, and `spec/vectors/format/` is what stands there; see
//! `pyfmt`'s crate doc, which records the same shape at `mechanics.rs`'s call sites.

use contracts::alignment::{ClipAlignment, SwingAlignment, SwingAnchors};
use contracts::capability::device_of;
use contracts::checkpoints::{checkpoint_names, CHECKPOINT_REGISTRY, CONTRADICTED_BY_A_LATE_TOP};
use contracts::detections::FrameDetections;
use contracts::golfer::Handedness;
use contracts::intent::PracticeGoal;
use contracts::keypoints::{FrameKeypoints, KeypointsFile};
use contracts::pivots::{FrameOfReference, PIVOT_MEASUREMENT_REGISTRY};
use contracts::placements::{spec_for as placement_spec, DOWN_THE_LINE, FACE_ON};
use contracts::shot::ShotData;
use contracts::swing::{
    CheckpointScore, Measurement, PhaseSegment, SwingBundleResult, SwingResult, ANALYSIS_VERSION,
};
use contracts::unscored::{UnscoredCheckpoint, UnscoredReason};
use contracts::Validate;

use crate::alignment::{
    align_swings, anchors_from_keypoints, anchors_from_phases, with_measured_impact,
    MIN_PLAUSIBLE_TEMPO,
};
use crate::benchmarks::flight_model::load_flight_model;
use crate::benchmarks::joint::placement_for as joint_placement;
use crate::benchmarks::trajectory::{placement_from_anchors, trajectory_placement_for};
use crate::checkpoints::{evaluator_for, CheckpointOutcome};
use crate::flight::DEFAULT_STEP_S;
use crate::flight_measure::{
    flight_unscored, fly_shot, FlownShot, FLIGHT_MEASUREMENTS, FLIGHT_SOURCE,
};
use crate::measure::POSE_MEASUREMENTS;
use crate::phases::{segment_phases, LEAD_WRIST, TRAIL_WRIST};
use crate::pivot::{check_for, pivot_observations};
use crate::scoring::policy_for;
use crate::shot_measure::SHOT_MEASUREMENTS;
use crate::smoothing::{smooth_keypoints, DEFAULT_WINDOW};
use crate::trajectory::anchors_from_phases as event_time_anchors;
use pyfmt::{fixed, g, registry_rank, round_to, signed_fixed, OrderedMap};

/// `Measurement.source` for anything read off the face-on pose stream.
const POSE_FACE_ON_SOURCE: &str = "pose:face_on";

/// `Measurement.source` for anything read off the down-the-line pose stream. [M22 P7]
///
/// **Python keeps this one in `contracts/career.py` and the face-on one at its call site**, and the
/// split is not arbitrary: this is the one `pose:` source that `CorpusSwing::artifact_key` keys on
/// *nothing*, because the corpus holds no hash for the rear clip (ADR-029's 2026-09-09b addendum
/// §5), so `career.py` has to name it in order to special-case it. `career.py` is not ported — §8
/// does not list it — and its only ported reader is [`pivot_view`], so the constant arrives here
/// beside its partner rather than dragging a module in for one string.
const POSE_DTL_SOURCE: &str = "pose:down_the_line";

/// `Measurement.source` for a placement, whichever basis produced it. One string for both views:
/// `CorpusSwing.artifact_key` deliberately does not dedupe on `population:` at all, so the two
/// bases share it where the two pose streams do not.
const POPULATION_SOURCE: &str = "population:golfdb";

/// The `measurements` stage's `pose` group, and the metric values [`placements`] needs beside it.
///
/// `_measurements`' first stanza, returning both of what it computes. The Python accumulates the
/// rows and the `pose_values` dict in one loop and hands the dict to `_placements` a few lines
/// later; splitting that into two walks would measure every metric twice, and the second walk is
/// the one that would quietly go out of step.
///
/// **The reason a measurement is missing is not recorded.** `measurements` is the unjudged half — a
/// metric absent from it has no band to be unscored *against*, and the reasons that matter are the
/// ones attached to a checkpoint. `MeasureOutcome::reason` is read by [`crate::checkpoints`], not
/// here.
///
/// The returned map is a [`OrderedMap`] and its order is [`POSE_MEASUREMENTS`]', which is the
/// Python dict's insertion order. Nothing downstream iterates it — [`joint_placement`] looks up by
/// name — so this is the edge held rather than the edge exercised.
pub fn pose_measurements(
    smoothed: &[FrameKeypoints],
    phases: &[PhaseSegment],
) -> (Vec<Measurement>, OrderedMap<f64>) {
    let mut rows = Vec::new();
    let mut values = OrderedMap::new();
    for (name, pose) in POSE_MEASUREMENTS {
        let Some(value) = (pose.measure)(smoothed, phases).value else {
            continue;
        };
        values.insert(*name, value);
        rows.push(Measurement {
            name: name.to_string(),
            value: round_to(value, 4),
            unit: pose.unit.to_string(),
            source: POSE_FACE_ON_SOURCE.to_string(),
            detail: pose.detail.to_string(),
        });
    }
    (rows, values)
}

/// Where this swing sits against the tour *population*, as recorded quantities.
///
/// Three numbers that no single checkpoint can produce, because each is about the swing as a whole:
/// how unusual the six metrics are **as a combination** (ADR-022's joint model), and how far the
/// motion sits from the tour shape both **inside** the fitted subspace (T²) and **off** it entirely
/// (Q).
///
/// **Recorded, not judged.** They ride on `measurements` rather than `checkpoint_scores`, so they
/// cannot touch `overall_score` — the same firewall ADR-010 §2 puts around percentiles. A band for
/// any of them would have to be earned separately, against a population of these values that does
/// not exist yet.
///
/// Each returns nothing rather than a guess when its inputs are incomplete: the joint model needs
/// all six metrics, the trajectory model needs three usable phase anchors and a body tracked well
/// enough to resample. **On every one of the fifteen corpus vectors the second refusal fires** and
/// this returns the joint row alone — see `tests/measurements.rs`.
pub fn placements(
    smoothed: &[FrameKeypoints],
    phases: &[PhaseSegment],
    pose_values: &OrderedMap<f64>,
    handedness: Option<Handedness>,
) -> Vec<Measurement> {
    let mut out = Vec::new();

    if let Some(joint) = joint_placement(pose_values) {
        // The largest contributor by magnitude, named in the sentence. `next(iter(…), "?")` — the
        // default is unreachable for a six-metric model and is carried anyway, because the Python
        // carries it and a port that "simplified" it would differ on an empty model.
        let leading = joint.contributions.first_key().unwrap_or("?");
        let spec = placement_spec("tour_joint_distance");
        out.push(Measurement {
            name: spec.name.to_string(),
            value: round_to(joint.distance, 4),
            unit: spec.unit.to_string(),
            source: POPULATION_SOURCE.to_string(),
            detail: format!(
                "Mahalanobis distance of the six metrics as a combination, against {} face-on \
                 tour swings; more unusual than {}% of them. Largest contributor: {leading}",
                joint.population_n,
                g(joint.percentile),
            ),
        });
    }

    let placement = trajectory_placement_for(
        smoothed,
        phases,
        handedness == Some(Handedness::Left),
        FACE_ON,
    );
    if let Some(placement) = placement {
        let interval = placement.residual_by_interval.first_key().unwrap_or("?");
        let (t2_spec, q_spec) = (
            placement_spec("tour_trajectory_t2"),
            placement_spec("tour_trajectory_q"),
        );
        out.push(Measurement {
            name: t2_spec.name.to_string(),
            value: round_to(placement.t2, 4),
            unit: t2_spec.unit.to_string(),
            source: POPULATION_SOURCE.to_string(),
            detail: format!(
                "distance from the tour swing shape inside the fitted subspace; more unusual \
                 than {}% of {} tour swings",
                g(placement.t2_percentile),
                placement.population_n,
            ),
        });
        out.push(Measurement {
            name: q_spec.name.to_string(),
            value: round_to(placement.q, 4),
            unit: q_spec.unit.to_string(),
            source: POPULATION_SOURCE.to_string(),
            detail: format!(
                "residual off the tour subspace — shape the basis cannot represent; most of it \
                 falls in {interval}. NOT calibrated: it over-flags golfers the basis never saw, \
                 so read it beside T2 rather than alone"
            ),
        });
    }

    out
}

/// The down-the-line view's own trajectory placement, against its own basis. [M22 P7]
///
/// Separate names rather than a `view` field on the existing ones, because `measurements` is keyed by
/// name everywhere downstream — `analysis/baseline.py::pooled_samples` groups by it, so two entries
/// called `tour_trajectory_t2` would silently pool a face-on and a down-the-line number into one
/// personal baseline.
///
/// Takes the smoothed frames and the three instants rather than the raw clip and a `SwingAnchors`,
/// because M17 P5 gave the rear clip a second reader: denoising and building the anchor tuple at the
/// one call site is what keeps the trajectory and the pivot paths resampling onto literally the same
/// numbers, instead of two copies of that conversion drifting apart.
///
/// **The two placements are never combined into one number.** They are two cameras answering the
/// same question about different planes of the same swing, and blending them would be exactly the
/// mistake ADR-009 avoided by keeping mechanics and outcome as separate axes. Disagreement between
/// them is a finding rather than a defect: a swing that looks ordinary face-on and unusual from
/// behind has departed in the plane the face-on camera cannot see.
pub fn dtl_placements(
    smoothed: &[FrameKeypoints],
    anchors: (f64, f64, f64),
    handedness: Option<Handedness>,
) -> Vec<Measurement> {
    let Some(placement) = placement_from_anchors(
        smoothed,
        anchors,
        handedness == Some(Handedness::Left),
        DOWN_THE_LINE,
    ) else {
        return Vec::new();
    };

    let interval = placement.residual_by_interval.first_key().unwrap_or("?");
    let t2_spec = placement_spec("tour_trajectory_t2_dtl");
    let q_spec = placement_spec("tour_trajectory_q_dtl");
    vec![
        Measurement {
            name: t2_spec.name.to_string(),
            value: round_to(placement.t2, 4),
            unit: t2_spec.unit.to_string(),
            source: POPULATION_SOURCE.to_string(),
            detail: format!(
                "down-the-line: distance from the tour swing shape inside its fitted subspace; \
                 more unusual than {}% of {} tour swings. A separate basis from the face-on one — \
                 never combine the two",
                g(placement.t2_percentile),
                placement.population_n,
            ),
        },
        Measurement {
            name: q_spec.name.to_string(),
            value: round_to(placement.q, 4),
            unit: q_spec.unit.to_string(),
            source: POPULATION_SOURCE.to_string(),
            detail: format!(
                "down-the-line: residual off that basis; most of it falls in {interval}. NOT \
                 calibrated, same caveat as the face-on Q"
            ),
        },
    ]
}

/// One view's rotation numbers: the swing read as three moving points.
///
/// One helper for both cameras, because [`pivot_observations`] has one signature for both — it takes
/// the three anchors rather than a `[PhaseSegment]`, which the down-the-line clip never has
/// (nothing segments it). The face-on caller converts its phases with [`event_time_anchors`].
///
/// **Recorded, not judged**, behind the same firewall as [`placements`]: no band, no
/// `CheckpointScore`, nothing that reaches `overall_score`. A rotation checkpoint needs a calibrated
/// instrument and a population and M17 has neither (ADR-010 §2), so each row carries its
/// `PivotMeasurementSpec::interim_reason` into the derived caveat prose instead.
///
/// **Two ways to record nothing, and they are not the same one.** An unreadable swing — collapsed
/// anchors, no usable shoulder width, a landmark missing too much of its timeline — yields no
/// observations and therefore no rows at all; a single check that refuses drops its own row and
/// leaves the other four. Neither is reported in `unscored`, which is a list of `UnscoredCheckpoint`
/// and nothing here is a checkpoint: `Measurement.value` is a required float, so a refusal ships as
/// an absence. Five per view is a ceiling, not a count.
pub fn pivot_measurements(
    smoothed: &[FrameKeypoints],
    anchors: (f64, f64, f64),
    view: &str,
) -> Vec<Measurement> {
    let (space, source) = pivot_view(view);
    let Some(observations) = pivot_observations(smoothed, anchors, space) else {
        return Vec::new();
    };

    PIVOT_MEASUREMENT_REGISTRY
        .iter()
        .filter(|spec| spec.view == view)
        // Keyed by `spec.check` and never by `spec.name`: a face-on spec and its `_dtl` partner name
        // the same implementation, and the view is already decided by the caller.
        .filter_map(|spec| {
            let value = check_for(spec.check)(&observations).value?;
            Some(Measurement {
                name: spec.name.to_string(),
                value: round_to(value, 4),
                unit: spec.unit.to_string(),
                source: source.to_string(),
                detail: spec.detail.to_string(),
            })
        })
        .collect()
}

/// View -> the space its pivot coordinates live in, and the `Measurement.source` its rows carry.
///
/// One lookup rather than two, because the pair has to move together. The source is what
/// `CorpusSwing.artifact_key` dedupes on: a `_dtl` row borrowing `pose:face_on` would be counted as
/// a reading of a clip it was never taken from.
///
/// **`CALIBRATED_3D` is deliberately absent and is not an omission**: [`pivot_observations`] raises
/// on it. A calibrated source arrives as a second *producer* of `PivotObservation`, not as a third
/// row here.
fn pivot_view(view: &str) -> (FrameOfReference, &'static str) {
    match view {
        FACE_ON => (FrameOfReference::ImagePlaneFaceOn, POSE_FACE_ON_SOURCE),
        DOWN_THE_LINE => (FrameOfReference::ImagePlaneDtl, POSE_DTL_SOURCE),
        other => panic!("no pivot view registered for {other:?}"),
    }
}

/// The three `measurements` groups a face-on clip alone produces, in the order they are recorded.
///
/// `_measurements`' first three stanzas as one call, which is how the stage vector records them:
/// one flat list, sliced. Order is pose first, then the placements, then the face-on rotation
/// numbers — and within each the registry's order, stable across runs so a diff of two
/// `analysis.json` files is readable.
///
/// The face-on rotation numbers come after the placements because they are the same kind of thing —
/// measured off the whole swing and judged by nothing. No anchors is a swing that could not be
/// segmented into three usable instants, which is already the reason [`placements`] recorded no
/// trajectory placement; the pivot rows simply go missing with it.
pub fn face_on_measurements(
    smoothed: &[FrameKeypoints],
    phases: &[PhaseSegment],
    handedness: Option<Handedness>,
) -> (Vec<Measurement>, Vec<Measurement>, Vec<Measurement>) {
    let (pose, pose_values) = pose_measurements(smoothed, phases);
    let placed = placements(smoothed, phases, &pose_values, handedness);
    let pivots = match event_time_anchors(phases) {
        Some(anchors) => pivot_measurements(smoothed, anchors, FACE_ON),
        None => Vec::new(),
    };
    (pose, placed, pivots)
}

/// The `measurements` stage's `shot` group: what the launch monitor printed, and what follows from
/// it. [M22 P8b]
///
/// `_measurements`' fourth stanza. Seven rows on every corpus vector, none on a swing with no shot —
/// and the *source* is per shot rather than constant, which is the one thing here that is not a
/// registry walk: `launch_monitor:{device}`, where the device is the OCR parse's own provenance and
/// falls back to the source enum when a shot arrived without one. That rule is
/// [`device_of`]'s, called rather than restated (M32 P10), so the corpus `measurements` stage that
/// compares this string exactly is what gates the capability model's reading of a shot's device.
///
/// Recorded and judged by nothing, like everything else in [`measurements`]. Carry has no band, and
/// ADR-010 §4 names TrackMan / Arccos as the population that would eventually cut one.
pub fn shot_measurements(shot: &ShotData) -> Vec<Measurement> {
    let source = format!("launch_monitor:{}", device_of(shot));
    SHOT_MEASUREMENTS
        .iter()
        .filter_map(|(name, row)| {
            (row.measure)(shot).map(|value| Measurement {
                name: (*name).to_string(),
                value: round_to(value, 4),
                unit: row.unit.to_string(),
                source: source.clone(),
                detail: row.detail.to_string(),
            })
        })
        .collect()
}

/// The `measurements` stage's `flight` group: one flight, six readings. [M22 P8b]
///
/// `_measurements`' fifth and last stanza. **One flight and six readings, not six flights**: the
/// registry's functions take the flown shot rather than the launch conditions for exactly that
/// reason — [`shot_measurements`]' one-function-per-number shape would re-integrate the whole
/// trajectory six times and the spin solve behind it dozens more.
///
/// Empty on a refused flight, which is ten of the fifteen corpus vectors; five rows on a flight whose
/// axis was refused, six only when both the spin was solved *and* the axis resolved — a combination
/// no vector in `spec/` contains.
pub fn flight_measurements(flown: &FlownShot) -> Vec<Measurement> {
    FLIGHT_MEASUREMENTS
        .iter()
        .filter_map(|(name, row)| {
            (row.measure)(flown).map(|value| Measurement {
                name: (*name).to_string(),
                value: round_to(value, 4),
                unit: row.unit.to_string(),
                source: FLIGHT_SOURCE.to_string(),
                detail: row.detail.to_string(),
            })
        })
        .collect()
}

/// Every quantity measurable off this swing, judged by nothing. `_measurements`. [M22 P8b]
///
/// [`face_on_measurements`]' three groups plus [`shot_measurements`] and [`flight_measurements`], as
/// one flat list — the shape that reaches `SwingResult.measurements`. Order is the registry's within
/// each group and the group order is the engine's, stable across runs so a diff of two
/// `analysis.json` files is readable.
///
/// Deliberately independent of the checkpoint loop. A metric appears here whether or not a band
/// exists for it, which is the whole point: bands are cut from populations of measurements, so a
/// metric that could only be measured once it had a band could never acquire one.
///
/// **The flight goes last because it is the only family that *consumes* another** —
/// `flight_infer` reads the shot's tiles through `shot_measure`'s own extractors — so a reader
/// meeting `flight_carry_yds` has already met the `carry_distance_yds` it was solved from.
///
/// **`flown` is passed in rather than computed here**, because a refused flight is also reported in
/// `unscored` and this function does not build that list. [`analyze_swing`] flies the ball once and
/// hands the result to both; flying it twice would double the most expensive call in the engine and
/// give two answers nothing guarantees are the same.
pub fn measurements(
    smoothed: &[FrameKeypoints],
    phases: &[PhaseSegment],
    shot: Option<&ShotData>,
    handedness: Option<Handedness>,
    flown: Option<&FlownShot>,
) -> Vec<Measurement> {
    let (pose, placed, pivots) = face_on_measurements(smoothed, phases, handedness);
    let mut out = pose;
    out.extend(placed);
    out.extend(pivots);
    if let Some(shot) = shot {
        out.extend(shot_measurements(shot));
    }
    if let Some(flown) = flown {
        out.extend(flight_measurements(flown));
    }
    out
}

/// One swing's data streams and the intent to judge them against — Python's keyword arguments.
///
/// **A struct because Rust has none, and because two pairs of these arguments are interchangeable
/// by type.** `analyze_swing` takes eight and `analyze_swing_bundle` thirteen, among them two
/// `Option<(i64, i64)>` windows and two `Option<&[i64]>` strike lists; positional parameters would
/// let a swapped pair compile silently, and the six vectors P6 is gated by pass `None` for all four,
/// so the gate could not see it either. Named fields are also the closer mirror of how the Python is
/// actually *called* — `conformance.py::run_vector` passes every one of the thirteen by keyword.
#[derive(Debug, Clone)]
pub struct SwingRequest<'a> {
    pub swing_id: &'a str,
    pub session_id: &'a str,
    pub keypoints: &'a [FrameKeypoints],
    /// Club and ball detections, echoed onto the result and read by nothing. M2's seam, and M1.5
    /// said no-go.
    pub detections: Option<&'a [FrameDetections]>,
    /// Attached and reported, never scored — and not waiting to be. ADR-034 §5 grades a shot per
    /// club over many shots (M35, M37), against the golfer's own baseline and `METRIC_TARGETS`
    /// rather than a tour band (§5.6), so no shot metric is ever banded in `ranges.json` and the
    /// per-swing `outcome_score` stays `None`: a share of one shot is 0 or 100 and means nothing.
    /// What a shot reaches here is [`shot_measurements`] and [`flight_measurements`], recorded and
    /// judged by nothing. [M32 P7]
    pub shot: Option<&'a ShotData>,
    /// `None` is Fundamentals — grade mechanics only.
    pub intent: Option<&'a PracticeGoal>,
    /// **Identity, not intent, which is why it is a separate field.** Every signed quantity this
    /// engine measures is camera-relative — a face-on camera sees a left-handed swing mirrored — so
    /// `head_stays_back` cannot be scored without it. `PracticeGoal` was the tempting place and is
    /// the wrong one: intent is what the golfer was *trying to do* and is chosen per session, while
    /// handedness is *who they are* and is recorded once.
    ///
    /// `None` costs the swing that one checkpoint, reported in `unscored` with `NoHandedness`, and
    /// costs it nothing else. `analysis` stays pure: resolving a `player_id` to a golfer is the
    /// shell's job and nothing here imports the golfer registry.
    pub handedness: Option<Handedness>,
    /// The club's declared loft. Its **entire** involvement is choosing between the two candidate
    /// spins the ball-flight solve returns (ADR-027 §Decision 3): loft is not an input to ball
    /// flight, so a club bent 2° strong changes the inferred spin and not the path the ball takes.
    ///
    /// Five of the fifteen corpus vectors carry `30.5`, resolved from the swing manifest's club by
    /// `conformance_vectors::_identity`, and the other ten carry nothing — so both sides of
    /// [`crate::flight_infer::LOFT_FLOOR_FOR_THE_FALLING_BRANCH_DEG`]'s *presence* check are gated and
    /// neither side of its *value* is: every loft in `spec/` is seventeen degrees clear of the floor.
    pub loft_deg: Option<f64>,
}

/// Analyze one swing from its data streams, judged against a practice intent. `analyze_swing`.
///
/// The spine: denoise, segment, evaluate every registered checkpoint, combine by the intent's
/// policy. No I/O, no hardware, no network.
///
/// Denoising happens once, up front, so the phase instants and every checkpoint read a stable
/// signal — raw MediaPipe landmarks jitter frame to frame. `SwingResult` still keeps the **raw**
/// keypoints as the source data this result was computed from.
///
/// # Why the registry is walked rather than the evaluators being listed
///
/// Each evaluator refuses rather than guesses when it cannot score (ADR-010 §2, ADR-013). That is
/// right, but a dropped score still has to be *reported*: `overall_score` is a mean over whatever
/// survived, so a two-checkpoint swing and a three-checkpoint swing otherwise print the same number
/// with nothing to distinguish them. Walking `CHECKPOINT_REGISTRY` is what lets `unscored` say which
/// one went missing — the name comes off the spec, so it cannot disagree with the `CheckpointScore`
/// the same spec's evaluator produced — and the evaluator supplies *why*, which nothing downstream
/// could recover.
///
/// Registry order is the reported order (`contracts::checkpoints` says so), and every evaluator is
/// called through the one adapted signature, so the two that read a narrower argument set — tempo
/// takes no keypoints, `head_stays_back` is the only one needing handedness — do not each need a
/// line here.
///
/// **Python's `assert judged.reason is not None` has no counterpart here**, and that is
/// [`CheckpointOutcome`] being an enum rather than a struct of two optionals: the compiler will not
/// let a scoreless, reasonless outcome exist, so there is nothing left to assert.
///
/// # The simulated flight, and why it is appended rather than looped over [M22 P8b]
///
/// `fly_shot` runs **after** the checkpoint loop and its refusals extend the same `unscored` list,
/// because nothing it produces is a checkpoint: ADR-027 §Decision 6 gives the flight its own
/// measurement names and no `CHECKPOINT_REGISTRY` entry, and registry order is what
/// `contracts::checkpoints` promises for the entries above it. So the flight's one or two entries
/// always come last in `unscored`, and `docs/CONFORMANCE.md` §3 compares that list positionally.
///
/// It is flown **once** and read by both halves — [`measurements`]' `flight` group and
/// [`crate::flight_measure::flight_unscored`] — which is why the `FlownShot` is threaded through
/// rather than each caller flying its own.
///
/// `outcome` is an empty list and the Fundamentals policy ignores it — the pose-only boundary, which
/// needs M2 detection and M3 shot bands to move.
pub fn analyze_swing(request: &SwingRequest) -> SwingResult {
    let intent = request.intent.cloned().unwrap_or_default();

    let smoothed = smooth_keypoints(request.keypoints, DEFAULT_WINDOW);
    let phases = segment_phases(&smoothed, LEAD_WRIST);

    let mut mechanics: Vec<CheckpointScore> = Vec::new();
    let mut unscored: Vec<UnscoredCheckpoint> = Vec::new();
    for spec in CHECKPOINT_REGISTRY {
        // Not `outcome` — that name is the outcome *axis* a few lines down (ADR-009), and the two
        // mean opposite things.
        match evaluator_for(spec.name)(&smoothed, &phases, request.handedness, intent.club, None) {
            CheckpointOutcome::Scored(score) => mechanics.push(score),
            CheckpointOutcome::Unscored { reason, detail } => unscored.push(UnscoredCheckpoint {
                name: spec.name.to_string(),
                reason,
                detail,
            }),
        }
    }

    // The simulated flight. Python defaults `model` and `step_s`; both are explicit here, following
    // P4's precedent with `smooth_keypoints`' window — a defaulted argument in a port is a place the
    // two languages can silently disagree about what the default was.
    let flown = request.shot.map(|shot| {
        fly_shot(
            shot,
            request.loft_deg,
            request.handedness,
            load_flight_model(),
            DEFAULT_STEP_S,
        )
    });
    if let Some(flown) = flown.as_ref() {
        unscored.extend(flight_unscored(flown));
    }

    // Pose-only, and staying so: ADR-034 §5 grades a shot over many shots per club, never as a
    // per-swing outcome checkpoint, so this list is empty by decision rather than by schedule.
    let outcome: Vec<CheckpointScore> = Vec::new();
    let scores = policy_for(intent.mode)(&mechanics, &outcome);

    let mut checkpoint_scores = mechanics;
    checkpoint_scores.extend(outcome);

    let result = SwingResult {
        swing_id: request.swing_id.to_string(),
        session_id: request.session_id.to_string(),
        measurements: measurements(
            &smoothed,
            &phases,
            request.shot,
            request.handedness,
            flown.as_ref(),
        ),
        phases,
        checkpoint_scores,
        unscored,
        intent: Some(intent),
        mechanics_score: scores.mechanics,
        outcome_score: scores.outcome,
        overall_score: scores.overall,
        keypoints: request.keypoints.to_vec(),
        detections: request.detections.unwrap_or_default().to_vec(),
        shot: request.shot.cloned(),
    };
    // Pydantic validates at `SwingResult(...)`; Rust has no constructor hook, so ADR-032 §4's rule —
    // the constraints are behaviour — needs the call spelled at the moment the shape is assembled.
    // The same choice `CheckpointOutcome::scored` makes, and it is *stricter* than pydantic in one
    // way worth naming: this recurses into the nested models, where pydantic's
    // `revalidate_instances='never'` does not re-check a `CheckpointScore` its own constructor
    // already validated. Every one of them was, so the extra reach costs nothing and catches a
    // struct literal assembled by hand.
    result
        .validate()
        .unwrap_or_else(|e| panic!("the engine assembled an invalid SwingResult: {e}"));
    result
}

/// One assembled bundle: two camera views plus the launch-monitor shot. Python's keyword arguments,
/// for the reason [`SwingRequest`] gives.
#[derive(Debug, Clone)]
pub struct BundleRequest<'a> {
    pub swing_id: &'a str,
    pub session_id: &'a str,
    pub face_on: &'a KeypointsFile,
    /// **P7's**, and [`analyze_swing_bundle`] refuses a bundle carrying one until then.
    pub down_the_line: Option<&'a KeypointsFile>,
    /// Never scored, for [`SwingRequest::shot`]'s reason.
    pub shot: Option<&'a ShotData>,
    pub intent: Option<&'a PracticeGoal>,
    /// Restrict the view to `[start, end)`, which is how a clip containing practice swings is
    /// pointed at the real one. **Not cosmetic**: the window decides which frames get *scored*, not
    /// merely which get rendered.
    pub face_on_window: Option<(i64, i64)>,
    pub down_the_line_window: Option<(i64, i64)>,
    /// The ball strikes heard in this view's own clip, as frame indices in that clip's own
    /// numbering. They move **only the alignment** — no checkpoint changes, because
    /// [`analyze_swing`] has already scored the face-on view off its own phases by the time these
    /// are read, which is the property that lets M11 improve the anchor without re-opening M4's
    /// scoring.
    pub face_on_strikes: Option<&'a [i64]>,
    pub down_the_line_strikes: Option<&'a [i64]>,
    pub handedness: Option<Handedness>,
    /// The spin solve's branch prior, as on [`SwingRequest`].
    pub loft_deg: Option<f64>,
}

/// The note a bundle with one camera carries, and the only note all six synthetic vectors produce.
///
/// A constant because the dash is a U+2014 em dash and `docs/CONFORMANCE.md` §3 compares
/// `SwingBundleResult.notes` exactly — a hyphen here is a failing vector and an invisible diff.
const NO_SECOND_VIEW_NOTE: &str = "no down-the-line view in this bundle — face-on analysis only";

/// Analyze one assembled swing bundle. `analyze_swing_bundle`, the entry point `run_vector` calls.
///
/// **The face-on view is scored by [`analyze_swing`], called unmodified.** That is the whole design:
/// the checkpoints were validated against 461 face-on tour clips and there is no reason for a second
/// camera to put that at risk. The down-the-line clip is segmented only to produce alignment anchors
/// — no checkpoint is measured from it, because the reference corpus behind every benchmark band is
/// face-on and a down-the-line metric would have nothing to be judged against (ADR-012, ADR-015).
///
/// **Everything comes back in whole-clip coordinates.** A window is a search restriction, not a
/// coordinate system: the face-on phases are shifted back by the window offset and the result
/// carries the full frame list, so `swing.phases[i].start_frame` indexes `swing.keypoints` and the
/// alignment anchors address the same frames the video does. A caller never tracks an offset.
///
/// Degradation is reported, not raised (ADR-013): a missing or unsegmentable down-the-line view
/// leaves `alignment` `None` with a note saying so, and the face-on result stands on its own.
///
/// # What the second view contributes, and in what order [M22 P7]
///
/// Five things, and the order between them is load-bearing rather than incidental:
///
/// 1. `anchors_from_keypoints` on the **trail** wrist, which is the only place the two views are
///    told apart.
/// 2. [`anchored_on_strike`] on those anchors, after the face-on pair and after
///    [`tempo_notes`] — that note is about the tempo *checkpoint*, scored off the pose phases, so
///    pinning impact first would have it quote a ratio no checkpoint on this swing was measured
///    from.
/// 3. The two `_dtl` measurement groups, off one smoothing pass and one anchor tuple shared by both
///    readers of the rear clip.
/// 4. `align_swings`, and its quality and notes.
/// 5. [`without_contradicted_scores`], **last** — everything above it is measured from one clip, and
///    this is the only finding that does not exist inside either clip alone.
pub fn analyze_swing_bundle(request: &BundleRequest) -> SwingBundleResult {
    let (start, frames) = windowed(&request.face_on.frames, request.face_on_window);
    let mut swing = analyze_swing(&SwingRequest {
        swing_id: request.swing_id,
        session_id: request.session_id,
        keypoints: frames,
        detections: None,
        shot: request.shot,
        intent: request.intent,
        handedness: request.handedness,
        loft_deg: request.loft_deg,
    });

    // Back into whole-clip coordinates, and re-attach the frames the window sliced away. Keyed on
    // whether a window was actually *applied* rather than on `start`, because a window opening at
    // frame 0 still truncates the tail and would otherwise leave `keypoints` shorter than the phase
    // indices addressing it. Python tests `frames is not face_on.frames`; the length comparison is
    // that same question asked without an identity one, because `windowed` returns either the whole
    // list or a strictly shorter slice of it and never an equal-length copy.
    if frames.len() != request.face_on.frames.len() {
        swing.phases = swing.phases.iter().map(|s| shifted(s, start)).collect();
        swing.keypoints = request.face_on.frames.clone();
    }

    let mut notes: Vec<String> = Vec::new();

    // The anchors come from the phases `analyze_swing` just computed rather than from a second
    // segmentation pass. Two reasons, and the second is the important one: it is free, and it makes
    // it *impossible* for the frame a checkpoint was measured on and the frame the warp pins to
    // tau=1 to disagree.
    let face_anchors = anchors_from_phases(
        &swing.phases,
        request.face_on.clip.as_ref(),
        camera_id(&request.face_on.frames),
        0,
    );
    if face_anchors.is_none() {
        notes.push(
            "face-on: the swing could not be segmented into usable anchors, so the two views \
             cannot be aligned"
                .to_string(),
        );
    }

    // Before the strike is read, on purpose: this note is about the *tempo checkpoint*, and the
    // checkpoint was scored off the pose phases. Pinning impact to the sound first would have it
    // quote a ratio no checkpoint on this swing was measured from.
    notes.extend(tempo_notes(face_anchors.as_ref()));

    let face_anchors =
        anchored_on_strike(face_anchors, request.face_on_strikes, "face-on", &mut notes);

    let mut dtl_anchors: Option<SwingAnchors> = None;
    match request.down_the_line {
        None => notes.push(NO_SECOND_VIEW_NOTE.to_string()),
        Some(down_the_line) => {
            // The **trail** wrist, not the lead one — this is the only place the two views are told
            // apart, and it is worth the special case. From behind, the lead wrist is the far arm
            // and is tracked in 39% of frames; the shipped rule then misses the top on 30% of
            // GolfDB's down-the-line clips and impact on 35%. On the trail wrist it misses 7% and
            // 2%, which is better than face-on manages. Measured over 584 labelled clips.
            dtl_anchors = anchors_from_keypoints(
                &down_the_line.frames,
                down_the_line.clip.as_ref(),
                request.down_the_line_window,
                Some(TRAIL_WRIST),
            );
            if dtl_anchors.is_none() {
                notes.push(
                    "down-the-line: the clip could not be segmented into a swing, so the two \
                     views cannot be aligned"
                        .to_string(),
                );
            }
        }
    }

    let dtl_anchors = anchored_on_strike(
        dtl_anchors,
        request.down_the_line_strikes,
        "down-the-line",
        &mut notes,
    );

    // The second camera's own placement and rotation numbers, built from `dtl_anchors` rather than
    // by segmenting again — those are already the three instants this model resamples onto, so
    // reusing them makes it impossible for the frames the trajectory reads and the frames the warp
    // pins to disagree.
    //
    // Denoised once and the three instants built once, for **both** readers of this clip. Two
    // constructions of the same anchor tuple would be two things to keep in step, and what is worth
    // keeping is that the trajectory and the pivot paths resample onto the same numbers.
    if let (Some(down_the_line), Some(anchors)) = (request.down_the_line, dtl_anchors.as_ref()) {
        let dtl_frames = smooth_keypoints(&down_the_line.frames, DEFAULT_WINDOW);
        let dtl_events = (
            anchors.motion_start as f64,
            anchors.top as f64,
            anchors.impact as f64,
        );
        swing
            .measurements
            .extend(dtl_placements(&dtl_frames, dtl_events, request.handedness));
        swing
            .measurements
            .extend(pivot_measurements(&dtl_frames, dtl_events, DOWN_THE_LINE));
    }

    let mut alignment: Option<SwingAlignment> = None;
    if let (Some(face), Some(dtl)) = (face_anchors.as_ref(), dtl_anchors.as_ref()) {
        let aligned = align_swings(face, dtl);
        if aligned.quality.is_degraded() {
            notes.push(format!("alignment degraded: {}", aligned.quality.summary()));
        }
        notes.extend(
            aligned
                .notes
                .iter()
                .map(|note| format!("alignment: {note}")),
        );

        // The one thing in this function that reaches back into a score [`analyze_swing`] already
        // produced, and it is deliberately the *last* thing: everything above it is measured from
        // one clip, and this is the only finding that does not exist inside either clip alone.
        // `align_swings` is passed the face-on anchors as `a`, so `alignment.a` is always the
        // scored view.
        if aligned.a.as_ref().is_some_and(ClipAlignment::top_is_late) {
            let clip = aligned.a.as_ref().expect("checked just above");
            swing = without_contradicted_scores(swing, clip, &mut notes);
        }
        alignment = Some(aligned);
    }

    // ADR-014's disclosure, and the one note on this path a *shot* produces rather than a clip. No
    // synthetic vector carries a shot, so it is covered by the unit test below rather than by P6's
    // gate — `tests/engine.rs` pins that absence.
    if let Some(provenance) = request.shot.and_then(|shot| shot.provenance.as_ref()) {
        if provenance.needs_review {
            notes.push(format!(
                "shot data needs review (parse confidence {}) — the numbers below were read off a \
                 photograph and at least one check on them failed (ADR-014)",
                fixed(provenance.parse_confidence, 2),
            ));
        }
    }

    let bundle = SwingBundleResult {
        swing_id: request.swing_id.to_string(),
        session_id: request.session_id.to_string(),
        swing,
        // Set explicitly, because the field defaults to 0 so that artifacts written before it
        // existed read as older-than-current rather than as current. This is the one place that gets
        // to claim otherwise, and it earns it by being the thing that just did the work.
        analysis_version: ANALYSIS_VERSION,
        alignment,
        face_on_window: request.face_on_window,
        down_the_line_window: request.down_the_line_window,
        notes,
        // `analysis` may not import `feedback` (ADR-008). The caller fills this in, which is why
        // `conformance.py::run_vector` makes two calls and `crates/core`'s `run` seam does too.
        feedback: None,
    };
    bundle
        .validate()
        .unwrap_or_else(|e| panic!("the engine assembled an invalid SwingBundleResult: {e}"));
    bundle
}

/// `(offset, frames)` for a window, clamped into range. An empty window is ignored. `_windowed`.
///
/// `pub` for one outside caller, `crates/core`'s stage recorder (M32 P2), which slices the clip the
/// way this function does because `conformance.py::run_stages` calls `E._windowed` for the same
/// reason: a second copy of the clamping is a thing that drifts.
pub fn windowed(
    keypoints: &[FrameKeypoints],
    window: Option<(i64, i64)>,
) -> (i64, &[FrameKeypoints]) {
    let Some((lo, hi)) = window else {
        return (0, keypoints);
    };
    let start = lo.max(0);
    let end = hi.min(keypoints.len() as i64);
    if end - start <= 0 {
        return (0, keypoints);
    }
    (start, &keypoints[start as usize..end as usize])
}

/// One phase boundary moved from window coordinates into whole-clip coordinates. `_shifted`.
///
/// Only the frame indices move. `start_ms` / `end_ms` are read off the frames themselves, which
/// carry the original clip's timestamps through a slice untouched, so they are already right.
///
/// `pub` for one outside caller, `crates/core`'s stage module (M32), whose compose check puts the
/// window-relative `phases` stage back onto the bundle's whole-clip phases with it — as
/// `conformance_vectors._verify_stages_compose` does with `E._shifted`.
pub fn shifted(segment: &PhaseSegment, offset: i64) -> PhaseSegment {
    PhaseSegment {
        start_frame: segment.start_frame + offset,
        end_frame: segment.end_frame + offset,
        ..segment.clone()
    }
}

/// The first `camera_id` any frame recorded, or `None`. `_camera_id`.
///
/// Reads the **raw** frames rather than the smoothed ones, and that is load-bearing:
/// `smooth_keypoints` drops `camera_id` (P4's reproduced wart), so asking the smoothed timeline
/// would answer `None` on every clip in the corpus.
///
/// `pub` for one outside caller, `crates/core`'s stage recorder (M32 P2), which names the face-on
/// anchors' camera the way `run_stages` does, through `E._camera_id`.
pub fn camera_id(keypoints: &[FrameKeypoints]) -> Option<&str> {
    keypoints
        .iter()
        .find_map(|frame| frame.camera_id.as_deref())
}

/// `anchors` with tau=2 pinned to the ball strike this view heard, saying so when it moved.
/// `_anchored_on_strike`.
///
/// **Never substitutes silently** (ADR-010 §2). A correction of five to seven frames is exactly the
/// size of the defect M10 handed forward, so a reader comparing this run against an older one has to
/// be able to see that the impact frame changed and by how much — and a reader who *only* has this
/// run has to be able to see that the number is a measurement rather than an estimate.
///
/// Runs on `None` too, so the two call sites do not each need the guard: a view that could not be
/// segmented has no anchor to pin.
///
/// `pub` for one outside caller, `crates/core`'s stage recorder (M32 P2), which records the pinned
/// anchors beside the unpinned ones through this function rather than through
/// `with_measured_impact` alone — `run_stages` calls `E._anchored_on_strike`, and the notes it
/// writes are discarded there as they are here.
pub fn anchored_on_strike(
    anchors: Option<SwingAnchors>,
    strikes: Option<&[i64]>,
    label: &str,
    notes: &mut Vec<String>,
) -> Option<SwingAnchors> {
    let anchors = anchors?;
    let measured = with_measured_impact(&anchors, strikes);
    let moved = measured.impact - anchors.impact;
    if measured.impact_measured && moved != 0 {
        // `impact_measured` is only ever true when fps was known — `with_measured_impact` sizes its
        // window with it — so the conversion to seconds below cannot divide by nothing.
        let fps = measured
            .fps
            .expect("impact_measured without the fps that sized its window");
        notes.push(format!(
            "{label}: impact moved {moved:+} frames ({}s) onto the ball strike heard in this clip \
             — pose had it {}. tau=2 is now a measured instant, not an estimate",
            signed_fixed(moved as f64 / fps, 3),
            if moved > 0 { "early" } else { "late" },
        ));
    }
    Some(measured)
}

/// `swing` with every score a late top invalidates withdrawn into `unscored`.
/// `_without_contradicted_scores`. [M11 P8, M22 P7]
///
/// **The only place a cross-view finding re-opens a face-on score**, and it exists because M10
/// handed forward three `tempo` readings that score, *fail* at 4.92-6.09:1 against a 4.71 ceiling,
/// and are not coaching truth — the top they divide by landed late, which shortens the downswing and
/// lengthens the backswing at once. From inside the face-on clip nothing about that is visible;
/// `phases` reports the boundary as detected and is not wrong to. It takes the other camera, on a
/// clock both of them heard, to know.
///
/// **Withdrawn rather than restated, and the distinction is ADR-010 §2.** `align_swings` can say
/// what the ratio reads on the corrected top, and on one bundle that is 2.35:1 — a pass. Writing it
/// into `CheckpointScore::observed` would mean a score whose number came from the alignment and
/// whose band came from the engine, measured over frames `segment_phases` never agreed to; and where
/// the two views disagree without a shared clock to arbitrate them there is no restatement to write
/// at all. No score beats a wrong one, and a corrected top belongs in `phases` where the boundary is
/// found, not patched in at the seam that noticed.
///
/// `mechanics` and `outcome` are split back apart by **registry membership** rather than by
/// position, because the policy weighs the two axes differently and `checkpoint_scores` is their
/// concatenation with nothing marking the join (ADR-009). Today the outcome list is empty and that
/// partition is a no-op; it is written this way so it stays right when it is not.
///
/// # Nothing in `spec/vectors/` reaches this function, in either language
///
/// It needs `ClipAlignment::top_is_late`, which needs `alignment::arbitrate_tops` to name a culprit,
/// which needs the two views' downswings to disagree past `DOWNSWING_AGREEMENT`. The widest
/// disagreement the fifteen corpus pairs produce is 27.6% against that 30% — so P7 ports this with a
/// gate that cannot run it, and the unit tests below are the whole of its coverage. See
/// `alignment`'s module doc for why the corpus stopped containing the case this was built for.
fn without_contradicted_scores(
    swing: SwingResult,
    clip: &ClipAlignment,
    notes: &mut Vec<String>,
) -> SwingResult {
    let withdrawn: Vec<CheckpointScore> = swing
        .checkpoint_scores
        .iter()
        .filter(|score| CONTRADICTED_BY_A_LATE_TOP.contains(&score.name.as_str()))
        .cloned()
        .collect();
    if withdrawn.is_empty() {
        return swing;
    }

    let mut swing = swing;
    let late_by = clip
        .top_late_by
        .expect("the caller reached here through `top_is_late`");

    let kept: Vec<CheckpointScore> = swing
        .checkpoint_scores
        .iter()
        .filter(|score| !withdrawn.iter().any(|gone| gone.name == score.name))
        .cloned()
        .collect();
    let registered = checkpoint_names();
    let (mechanics, outcome): (Vec<CheckpointScore>, Vec<CheckpointScore>) = kept
        .iter()
        .cloned()
        .partition(|score| registered.contains(&score.name.as_str()));
    let scores = policy_for(swing.intent.clone().unwrap_or_default().mode)(&mechanics, &outcome);

    let detail = format!(
        "the other view, synchronized on the ball strike, puts the top {late_by} frames earlier \
         than this clip did"
    );
    for score in &withdrawn {
        let observed = match score.observed {
            None => String::new(),
            Some(value) => format!(" at {}", fixed(value, 2)),
        };
        notes.push(format!(
            "{} withdrawn: it scored {}{observed} off a top the down-the-line view puts {late_by} \
             frames earlier on a shared clock",
            score.name,
            fixed(score.score, 2),
        ));
    }

    swing
        .unscored
        .extend(withdrawn.iter().map(|score| UnscoredCheckpoint {
            name: score.name.clone(),
            reason: UnscoredReason::CrossViewContradicted,
            detail: detail.clone(),
        }));
    // Registry order is the reported order of `unscored` (`contracts::checkpoints` says so), so the
    // withdrawn entries are sorted into place rather than appended — an appended `tempo` would put
    // the first checkpoint last and quietly break the one ordering that module asks for.
    //
    // `sort_by_key` is Rust's **stable** sort, which is load-bearing for the same reason
    // [`registry_rank`] documents: every unregistered name shares one rank, and their relative order
    // is then whatever they arrived in.
    swing
        .unscored
        .sort_by_key(|entry| registry_rank(&entry.name, &registered));
    swing.checkpoint_scores = kept;
    swing.mechanics_score = scores.mechanics;
    swing.outcome_score = scores.outcome;
    swing.overall_score = scores.overall;
    swing
}

/// Say out loud when the tempo checkpoint's own input is not physically possible. `_tempo_notes`.
///
/// A backswing that measures shorter than its own downswing is not a golf swing — the same fact
/// `align_swings` refuses a motion-start anchor over, read here against the checkpoint that
/// *divides* by that boundary. It happens on real footage for a specific reason: a golfer who pauses
/// at the top hands `phases::motion_start` a quiet stretch immediately, so the boundary lands a
/// frame or two below the top and the backswing measures near zero.
///
/// `phases` reports `detected = true` and is not wrong to — from inside one clip nothing about it
/// looks wrong — so `evaluate_tempo` scores it and `feedback` leads with *"work on tempo first … take
/// it back longer"*, prescribing a backswing target off a downswing that was never measured against
/// a real top.
///
/// **The score is deliberately left alone here.** Dropping the checkpoint would mean this function
/// disagreeing with [`analyze_swing`] about the same frames, and fixing the boundary belongs in
/// `phases::motion_start` where the bug is — measured against the GolfDB corpus the tempo band was
/// derived from, not patched downstream. What this does is make the contradiction impossible to
/// render without seeing it.
///
/// Returns nothing on a plausible swing, which is every one of the six synthetic vectors — so both
/// sentences here are covered by the unit tests below and by no committed answer.
fn tempo_notes(anchors: Option<&SwingAnchors>) -> Vec<String> {
    let Some(anchors) = anchors else {
        return Vec::new();
    };
    let Some(ratio) = anchors.tempo_ratio() else {
        return vec![
            "face-on: no measurable backswing, so any tempo reading on this swing is meaningless"
                .to_string(),
        ];
    };
    if ratio < MIN_PLAUSIBLE_TEMPO {
        return vec![format!(
            "face-on: the backswing measures {} downswings, which no golf swing does — the \
             motion-start boundary has collapsed onto the top (a pause at the top reads as the \
             quiet stretch the takeaway is measured back to). Any tempo score on this swing is \
             measuring that artifact, not the golfer; ignore it",
            fixed(ratio, 2),
        )];
    }
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{
        a_body, a_body_that_pauses_at_the_top, a_rear_swinging_body, a_segmentation,
        a_swinging_body, phase, with_preroll,
    };
    use contracts::keypoints::ClipMetadata;
    use contracts::swing::SwingPhase;

    #[test]
    fn every_pose_row_carries_the_face_on_source_and_its_registry_unit() {
        let (rows, values) = pose_measurements(&a_body(50), &a_segmentation());
        assert!(!rows.is_empty());
        for row in &rows {
            assert_eq!(row.source, "pose:face_on");
            let (_, spec) = POSE_MEASUREMENTS
                .iter()
                .find(|(name, _)| *name == row.name)
                .expect("a row nothing registers");
            assert_eq!(row.unit, spec.unit);
            assert_eq!(row.detail, spec.detail);
        }
        // Every row has a value and every value has a row: the two outputs are one walk.
        assert_eq!(rows.len(), values.len());
        for row in &rows {
            assert!(values.get(&row.name).is_some());
        }
    }

    /// The rows are in [`POSE_MEASUREMENTS`]' order, which is ADR-032 §3's third edge reaching this
    /// module: `docs/CONFORMANCE.md` §3 compares the flat list positionally.
    #[test]
    fn the_pose_rows_follow_the_registry_order() {
        let (rows, _) = pose_measurements(&a_body(50), &a_segmentation());
        let registered: Vec<&str> = POSE_MEASUREMENTS.iter().map(|(name, _)| *name).collect();
        let emitted: Vec<&str> = rows.iter().map(|row| row.name.as_str()).collect();
        let mut at = 0;
        for name in emitted {
            at = registered[at..]
                .iter()
                .position(|r| *r == name)
                .expect("a row out of registry order")
                + at
                + 1;
        }
    }

    /// A refused metric is simply absent — never a zero, never a sentinel. ADR-010 §2 in the one
    /// place it is easiest to get wrong, because a `Measurement.value` is a required float.
    #[test]
    fn a_refused_metric_leaves_no_row_at_all() {
        // Two frames cannot produce a phase timeline, so most of the registry refuses.
        let (rows, values) = pose_measurements(&a_body(2), &[]);
        assert!(rows.len() < POSE_MEASUREMENTS.len());
        assert_eq!(rows.len(), values.len());
        assert!(rows.iter().all(|row| row.value.is_finite()));
    }

    /// Every pivot row's name, unit and detail come off the registry rather than this module, and
    /// the source is the face-on pose stream's. A row that invented any of the four would be a
    /// measurement `caveats.py` cannot describe.
    #[test]
    fn every_pivot_row_is_the_registrys_own_face_on_half() {
        let phases = a_segmentation();
        let anchors = event_time_anchors(&phases).expect("the fixture segments");
        let rows = pivot_measurements(&a_body(50), anchors, FACE_ON);
        for row in &rows {
            let spec = contracts::pivots::spec_for(&row.name);
            assert_eq!(spec.view, FACE_ON);
            assert_eq!(row.unit, spec.unit);
            assert_eq!(row.detail, spec.detail);
            assert_eq!(row.source, "pose:face_on");
        }
        assert!(rows.len() <= 5, "five per view is a ceiling, not a count");
    }

    /// And the down-the-line half carries `pose:down_the_line`, which is the row the corpus vectors
    /// gate and the *source* they cannot check twice.
    ///
    /// `artifact_key` dedupes on it, and a `_dtl` row borrowing `pose:face_on` would be counted as a
    /// reading of a clip it was never taken from. The committed answer does pin this string — every
    /// corpus vector records a five-row `pivot_dtl` group with it — so this test is the *structural*
    /// half: that the view decides the name and the source together and neither is retyped.
    #[test]
    fn every_pivot_row_is_the_registrys_own_dtl_half() {
        let rows = pivot_measurements(&a_body(50), (0.0, 10.0, 20.0), DOWN_THE_LINE);
        for row in &rows {
            let spec = contracts::pivots::spec_for(&row.name);
            assert_eq!(spec.view, DOWN_THE_LINE);
            assert!(row.name.ends_with("_dtl"));
            assert_eq!(row.unit, spec.unit);
            assert_eq!(row.detail, spec.detail);
            assert_eq!(row.source, "pose:down_the_line");
        }
        assert!(rows.len() <= 5, "five per view is a ceiling, not a count");
    }

    /// A view with no frame of reference is a wiring bug, and the message says which views exist.
    ///
    /// `CALIBRATED_3D` is the one that is absent on purpose rather than pending: a calibrated source
    /// arrives as a second *producer* of `PivotObservation`, never as a third row in [`pivot_view`],
    /// and `pivot_observations` refuses it at that layer instead.
    #[test]
    #[should_panic(expected = "no pivot view registered for \"calibrated_3d\"")]
    fn an_unregistered_pivot_view_is_a_wiring_bug() {
        pivot_measurements(&a_body(50), (0.0, 10.0, 20.0), "calibrated_3d");
    }

    /// A swing with no usable phase timeline records no pivot rows and no trajectory placement —
    /// the two refusals that travel together, because both need the same three instants.
    #[test]
    fn a_swing_that_cannot_be_segmented_records_neither_pivots_nor_a_trajectory() {
        let frames = a_body(50);
        let (_, placed, pivots) = face_on_measurements(&frames, &[], None);
        assert!(pivots.is_empty());
        assert!(placed
            .iter()
            .all(|row| !row.name.starts_with("tour_trajectory")));
    }

    // ----------------------------------------------------------------- P6, the assembly
    //
    // Everything below covers a branch **no committed vector reaches**, and that set was measured
    // by mutation rather than guessed: of 55 deliberate divergences across P6's five files, sixteen
    // survived `cargo test` on the six synthetic vectors and nine of those were in this module.
    // `tests/engine.rs`'s module doc lists the absences; these are the tests that stand in for them.

    fn a_clip(frames: Vec<FrameKeypoints>) -> KeypointsFile {
        KeypointsFile {
            clip: None,
            frames,
            pose_estimator: None,
        }
    }

    fn a_bundle<'a>(face_on: &'a KeypointsFile) -> BundleRequest<'a> {
        BundleRequest {
            swing_id: "1",
            session_id: "s",
            face_on,
            down_the_line: None,
            shot: None,
            intent: None,
            face_on_window: None,
            down_the_line_window: None,
            face_on_strikes: None,
            down_the_line_strikes: None,
            handedness: None,
            loft_deg: None,
        }
    }

    /// A shot carrying `provenance`, given as the JSON that field holds (`"null"` for none).
    ///
    /// A parse rather than a struct literal: `ShotData` has twenty optional fields this test cares
    /// about none of, and going through `serde_json` runs the shape's own bounds on the way in —
    /// ADR-032 §4's rule that a pydantic constraint is behaviour rather than an annotation.
    fn a_shot(provenance: &str) -> ShotData {
        let json = format!(
            r#"{{"shot_id": "1", "session_id": "s", "timestamp": "2026-09-25T00:00:00Z",
                 "source": "screen", "provenance": {provenance}}}"#
        );
        serde_json::from_str(&json).expect("the fixture is a valid ShotData")
    }

    fn some_anchors() -> SwingAnchors {
        SwingAnchors {
            motion_start: 10,
            top: 40,
            impact: 50,
            motion_start_detected: true,
            impact_measured: false,
            camera_id: None,
            frame_count: Some(100),
            fps: Some(60.0),
        }
    }

    /// `_windowed`'s three guards, none of which any vector exercises: the only committed window is
    /// `[40, 96]` on a 120-frame clip, so it is in range at both ends and non-empty.
    #[test]
    fn a_window_is_clamped_and_an_empty_one_is_ignored() {
        let frames = a_body(50);
        // No window at all.
        assert_eq!(windowed(&frames, None), (0, &frames[..]));
        // In range.
        let (start, sliced) = windowed(&frames, Some((10, 20)));
        assert_eq!((start, sliced.len()), (10, 10));
        // A negative start clamps to 0 rather than casting to an enormous index and panicking.
        let (start, sliced) = windowed(&frames, Some((-5, 20)));
        assert_eq!((start, sliced.len()), (0, 20));
        // An end past the clip clamps to its length.
        let (start, sliced) = windowed(&frames, Some((40, 900)));
        assert_eq!((start, sliced.len()), (40, 10));
        // Empty and inverted windows are *ignored*, which is not the same as slicing nothing: the
        // whole clip comes back with a zero offset, so a caller that mis-computed a window still
        // scores a swing rather than a zero-frame one.
        for empty in [(20, 20), (30, 10), (900, 901)] {
            assert_eq!(
                windowed(&frames, Some(empty)),
                (0, &frames[..]),
                "{empty:?}"
            );
        }
    }

    /// `_shifted` moves the frame indices and **nothing else**. `start_ms` / `end_ms` are read off
    /// the frames themselves, which carry the original clip's timestamps through a slice untouched.
    #[test]
    fn shifting_a_segment_moves_the_frames_and_not_the_clock() {
        let segment = phase(SwingPhase::Transition, 27, 33, false);
        let moved = shifted(&segment, 40);
        assert_eq!((moved.start_frame, moved.end_frame), (67, 73));
        assert_eq!(
            (moved.start_ms, moved.end_ms),
            (segment.start_ms, segment.end_ms)
        );
        assert_eq!(moved.phase, segment.phase);
        assert_eq!(moved.detected, segment.detected);
    }

    /// The **first** frame that recorded one wins, and a clip that recorded none answers `None`.
    ///
    /// Read off the raw frames rather than the smoothed ones, which is load-bearing:
    /// `smooth_keypoints` drops `camera_id`, so the smoothed timeline answers `None` on every clip.
    #[test]
    fn the_camera_id_is_the_first_one_any_raw_frame_recorded() {
        let mut frames = a_body(4);
        assert_eq!(camera_id(&frames), None);
        frames[1].camera_id = Some("face-on".to_string());
        frames[3].camera_id = Some("down-the-line".to_string());
        assert_eq!(camera_id(&frames), Some("face-on"));
        // And the smoothed timeline is exactly where it cannot be read from.
        assert_eq!(camera_id(&smooth_keypoints(&frames, DEFAULT_WINDOW)), None);
    }

    /// **The windowed bundle's re-attachment, which no vector in either language can check.**
    ///
    /// `EXCLUDED_FROM_RESULT` drops `keypoints` before a vector is written, so `expected` holds no
    /// frame list at all — which means `windowed.json` gates the phase *shift* and is structurally
    /// incapable of gating the frames those shifted indices address. Mutating the re-attachment to a
    /// no-op passes all six vectors, which is how P6 found this.
    ///
    /// The clip is a swing with forty frames of pre-roll in front of it, so the window is doing the
    /// job it exists for and the comparison is exact: windowing onto the swing must produce the
    /// unwindowed swing's phases, shifted by forty and nothing else.
    #[test]
    fn a_windowed_bundle_comes_back_in_whole_clip_coordinates() {
        let swing = a_swinging_body(80);
        let whole = a_clip(with_preroll(40, &swing));
        assert_eq!(whole.frames.len(), 120);

        let mut request = a_bundle(&whole);
        request.face_on_window = Some((40, 120));
        let windowed_run = analyze_swing_bundle(&request);

        // The window found a swing, so nothing below is vacuous — the trap this test was rewritten
        // to escape, because `a_body` stands still and segments to an empty phase list.
        assert!(!windowed_run.swing.phases.is_empty());

        let bare = a_clip(swing);
        let unwindowed = analyze_swing_bundle(&a_bundle(&bare));
        assert_eq!(
            windowed_run.swing.phases.len(),
            unwindowed.swing.phases.len()
        );
        for (shifted_segment, own) in windowed_run
            .swing
            .phases
            .iter()
            .zip(&unwindowed.swing.phases)
        {
            assert_eq!(shifted_segment.phase, own.phase);
            assert_eq!(shifted_segment.start_frame, own.start_frame + 40);
            assert_eq!(shifted_segment.end_frame, own.end_frame + 40);
            // **The milliseconds are not shifted and are still right**, which is the half of this
            // that reads like a bug. `_shifted` moves only the indices; the clock comes off the
            // frames, and a slice carries the whole clip's timestamps through untouched — so the
            // windowed run reports 400 ms where the bare clip reports 0, because in the recording
            // those frames really are 400 ms in. Shifting them too would double the offset.
            assert_eq!(shifted_segment.start_ms, own.start_ms + 400.0);
            assert_eq!(shifted_segment.end_ms, own.end_ms + 400.0);
        }

        // And the sliced frames are back, so a shifted index addresses one.
        assert_eq!(windowed_run.swing.keypoints.len(), 120);
        assert_eq!(windowed_run.swing.keypoints, whole.frames);
        assert_eq!(windowed_run.face_on_window, Some((40, 120)));
        for segment in &windowed_run.swing.phases {
            assert!(
                (segment.end_frame as usize) < windowed_run.swing.keypoints.len(),
                "a phase addresses a frame the result does not carry: {segment:?}"
            );
        }

        // The scores are the windowed swing's, not the whole recording's — a window decides which
        // frames get *scored*, which is the sentence `analyze_swing_bundle`'s doc calls not cosmetic.
        assert_eq!(
            windowed_run.swing.overall_score,
            unwindowed.swing.overall_score
        );
    }

    /// A window that opens at frame 0 still re-attaches, because the guard is about whether a slice
    /// happened and not about whether the offset is zero. A `start`-keyed guard passes every vector
    /// and leaves `keypoints` shorter than the phase indices addressing it.
    #[test]
    fn a_window_opening_at_frame_zero_still_truncates_the_tail() {
        let clip = a_clip(a_swinging_body(120));
        let mut request = a_bundle(&clip);
        request.face_on_window = Some((0, 60));
        let bundle = analyze_swing_bundle(&request);
        assert_eq!(bundle.swing.keypoints.len(), 120);
        assert!(!bundle.swing.phases.is_empty());
        for segment in &bundle.swing.phases {
            assert!((segment.end_frame as usize) < bundle.swing.keypoints.len());
        }
    }

    /// `_anchored_on_strike`'s sentence, byte for byte against CPython's, in both directions.
    ///
    /// No synthetic vector carries a strike list, so this string is gated here and nowhere else. The
    /// three cases are the ones the wording turns on: a strike **after** the pose estimate means pose
    /// had impact early, a strike before it means late, and the seconds carry a forced sign through
    /// `pyfmt::signed_fixed`.
    #[test]
    fn a_moved_impact_says_which_way_it_moved_and_by_how_much() {
        for (strikes, expected) in [
            (
                vec![44],
                "face-on: impact moved -6 frames (-0.100s) onto the ball strike heard in this clip \
                 — pose had it late. tau=2 is now a measured instant, not an estimate",
            ),
            (
                vec![56],
                "face-on: impact moved +6 frames (+0.100s) onto the ball strike heard in this clip \
                 — pose had it early. tau=2 is now a measured instant, not an estimate",
            ),
        ] {
            let mut notes = Vec::new();
            let out = anchored_on_strike(Some(some_anchors()), Some(&strikes), "face-on", &mut notes);
            assert_eq!(out.expect("anchors survive").impact, strikes[0]);
            assert_eq!(notes, vec![expected.to_string()]);
        }

        // 29.97 fps, which is where the seconds stop being a round number — the same `-6` frames
        // reads as a bigger correction because each frame is longer.
        let mut notes = Vec::new();
        anchored_on_strike(
            Some(SwingAnchors {
                fps: Some(30000.0 / 1001.0),
                ..some_anchors()
            }),
            Some(&[44]),
            "down-the-line",
            &mut notes,
        );
        assert_eq!(
            notes,
            vec![
                "down-the-line: impact moved -6 frames (-0.200s) onto the ball strike heard in \
                 this clip — pose had it late. tau=2 is now a measured instant, not an estimate"
            ]
        );
    }

    /// Nothing to pin to, or a strike the pose estimate already agreed with, says nothing. **Never
    /// substitutes silently** cuts both ways: a note about a correction that did not happen is as
    /// misleading as a silent one that did.
    #[test]
    fn an_unmoved_impact_appends_no_note() {
        for strikes in [None, Some(&[][..]), Some(&[50][..])] {
            let mut notes = Vec::new();
            let out = anchored_on_strike(Some(some_anchors()), strikes, "face-on", &mut notes);
            assert_eq!(out.expect("anchors survive").impact, 50);
            assert!(notes.is_empty(), "{strikes:?} produced {notes:?}");
        }
        // And `None` anchors pass through, so the two call sites do not each need the guard.
        let mut notes = Vec::new();
        assert!(anchored_on_strike(None, Some(&[44]), "face-on", &mut notes).is_none());
        assert!(notes.is_empty());
    }

    /// `_tempo_notes`' two sentences, byte for byte against CPython's. No synthetic vector produces
    /// either — all six segment into a plausible swing.
    #[test]
    fn an_impossible_backswing_is_said_out_loud() {
        // `motion_start == top` is tolerated by `SwingAnchors` and gives no backswing to divide.
        let collapsed = SwingAnchors {
            motion_start: 40,
            ..some_anchors()
        };
        assert_eq!(
            tempo_notes(Some(&collapsed)),
            vec![
                "face-on: no measurable backswing, so any tempo reading on this swing is \
                 meaningless"
            ]
        );

        // Three frames of backswing against ten of downswing: 0.30 downswings, which no golf swing
        // does. The `.2f` is `pyfmt::fixed`'s and the sentence is compared exactly.
        let implausible = SwingAnchors {
            motion_start: 37,
            ..some_anchors()
        };
        assert_eq!(
            tempo_notes(Some(&implausible)),
            vec![
                "face-on: the backswing measures 0.30 downswings, which no golf swing does — the \
                 motion-start boundary has collapsed onto the top (a pause at the top reads as the \
                 quiet stretch the takeaway is measured back to). Any tempo score on this swing is \
                 measuring that artifact, not the golfer; ignore it"
            ]
        );

        // A plausible swing says nothing, and so does a view with no anchors at all.
        assert!(tempo_notes(Some(&some_anchors())).is_empty());
        assert!(tempo_notes(None).is_empty());
    }

    /// The floor is `< MIN_PLAUSIBLE_TEMPO` and not `<=`: a swing whose backswing measures exactly
    /// one downswing is implausible-looking and is **not** flagged, because 1:1 is the bound the
    /// constant names rather than a value inside it.
    #[test]
    fn the_tempo_floor_is_exclusive() {
        // 10 frames of backswing against 10 of downswing.
        let exactly_one = SwingAnchors {
            motion_start: 30,
            ..some_anchors()
        };
        assert_eq!(exactly_one.tempo_ratio(), Some(1.0));
        assert!(tempo_notes(Some(&exactly_one)).is_empty());
        let just_under = SwingAnchors {
            motion_start: 31,
            ..some_anchors()
        };
        assert_eq!(tempo_notes(Some(&just_under)).len(), 1);
    }

    /// A clip that cannot be segmented into anchors says so, and still returns a result — degraded,
    /// reported, not raised (ADR-013). Two frames cannot hold a swing.
    #[test]
    fn a_clip_with_no_usable_anchors_says_so_and_still_scores() {
        let clip = a_clip(a_body(2));
        let bundle = analyze_swing_bundle(&a_bundle(&clip));
        assert_eq!(
            bundle.notes,
            vec![
                "face-on: the swing could not be segmented into usable anchors, so the two views \
                 cannot be aligned"
                    .to_string(),
                NO_SECOND_VIEW_NOTE.to_string(),
            ]
        );
        // And the result stands: nothing scored, so nothing is claimed.
        assert!(bundle.swing.checkpoint_scores.is_empty());
        assert_eq!(bundle.swing.overall_score, 0.0);
        assert_eq!(bundle.analysis_version, ANALYSIS_VERSION);
    }

    /// ADR-014's disclosure, which no synthetic vector carries a shot to produce. The confidence is
    /// `.2f` and the sentence is compared exactly.
    #[test]
    fn a_shot_that_needs_review_is_named_in_the_notes() {
        let clip = a_clip(a_swinging_body(50));
        // Built from JSON rather than from twenty field literals, which also runs the shape's own
        // validator on the way in — the `le(parse_confidence, 1)` bound ADR-032 §4 calls behaviour.
        let shot = a_shot(
            r#"{"device": "hd_golf", "parse_confidence": 0.425,
                              "needs_review": true,
                              "warnings": ["carry disagrees with ball speed"]}"#,
        );
        let mut request = a_bundle(&clip);
        request.shot = Some(&shot);
        let bundle = analyze_swing_bundle(&request);
        assert_eq!(
            bundle.notes.last().map(String::as_str),
            Some(
                "shot data needs review (parse confidence 0.42) — the numbers below were read off \
                 a photograph and at least one check on them failed (ADR-014)"
            )
        );
        // A trusted parse says nothing, and neither does a shot with no provenance at all — the
        // mock and R10 sources leave it `None`, so `needs_review` is not a field they can set.
        let trusted = a_shot(r#"{"device": "hd_golf", "parse_confidence": 0.425}"#);
        let says_review = |notes: &[String]| {
            notes
                .iter()
                .any(|n| n.starts_with("shot data needs review"))
        };
        request.shot = Some(&trusted);
        assert!(!says_review(&analyze_swing_bundle(&request).notes));
        let unparsed = a_shot("null");
        request.shot = Some(&unparsed);
        assert!(!says_review(&analyze_swing_bundle(&request).notes));
        request.shot = None;
        assert!(!says_review(&analyze_swing_bundle(&request).notes));
    }

    // ------------------------------------------------------------- P7, the second view
    //
    // The `alignment` stage gates the anchors and the warp on the fifteen corpus vectors. What it
    // cannot gate is this module's *wiring* of them — which note carries which prefix, where the two
    // `_dtl` groups land in the flat list, and the whole of `without_contradicted_scores`, which no
    // committed vector reaches in either language.

    /// A clip whose frames all record `camera_id`, because every note in `align_swings` interpolates
    /// it and the fallback labels (`"a"`, `"b"`) appear on no real recording.
    fn a_view(name: &str, frames: Vec<FrameKeypoints>) -> KeypointsFile {
        let frames = frames
            .into_iter()
            .map(|mut frame| {
                frame.camera_id = Some(name.to_string());
                frame
            })
            .collect();
        KeypointsFile {
            clip: Some(ClipMetadata {
                fps: Some(60.0),
                width: None,
                height: None,
                frame_count: None,
                source_sha256: None,
            }),
            frames,
            pose_estimator: None,
        }
    }

    /// A two-camera bundle aligns, and the `no down-the-line view` note is gone.
    ///
    /// What is pinned here is that the alignment exists, that `a` is the **face-on** clip (the
    /// convention `without_contradicted_scores` depends on), and that each of `align_swings`' own
    /// notes reaches the bundle under the `alignment: ` prefix rather than bare.
    ///
    /// **The 80-frame rear clip is chosen so the pair is degraded**, which is not incidental: a
    /// fixture whose two views agree produces no notes at all, the loop below is then vacuous, and
    /// dropping the prefix passes it. That is the failure mode `testing::a_swinging_body`'s own doc
    /// warns about, met for the second time in this crate — so the non-emptiness is asserted rather
    /// than hoped for.
    #[test]
    fn a_second_camera_aligns_and_its_notes_are_prefixed() {
        let face_on = a_view("face_on", a_swinging_body(50));
        let dtl = a_view("down_the_line", a_rear_swinging_body(80));
        let mut request = a_bundle(&face_on);
        request.down_the_line = Some(&dtl);
        let bundle = analyze_swing_bundle(&request);

        let alignment = bundle.alignment.as_ref().expect("two views align");
        assert_eq!(
            alignment
                .a
                .as_ref()
                .and_then(|clip| clip.anchors.camera_id.as_deref()),
            Some("face_on"),
            "`a` is the scored view and `without_contradicted_scores` reads it as such"
        );
        assert!(!bundle.notes.iter().any(|note| note == NO_SECOND_VIEW_NOTE));
        assert!(
            !alignment.notes.is_empty(),
            "a fixture with nothing to say cannot check the prefix"
        );
        for note in &alignment.notes {
            assert!(
                bundle.notes.contains(&format!("alignment: {note}")),
                "{note:?} reached the bundle unprefixed"
            );
        }
        assert!(alignment.quality.is_degraded(), "{:?}", alignment.quality);
        assert!(bundle
            .notes
            .iter()
            .any(|note| note == &format!("alignment degraded: {}", alignment.quality.summary())));
    }

    /// A second clip that cannot be segmented says so and leaves the face-on result standing.
    ///
    /// Reported, not raised (ADR-013), and the *absence* of an alignment is the point: a still body
    /// has no rising run, so `anchors_from_keypoints` returns `None` and there is nothing to warp
    /// onto. No corpus vector reaches this branch — all fifteen rear clips segment.
    #[test]
    fn an_unsegmentable_second_view_degrades_with_a_note() {
        let face_on = a_view("face_on", a_swinging_body(50));
        let dtl = a_view("down_the_line", a_body(50));
        let mut request = a_bundle(&face_on);
        request.down_the_line = Some(&dtl);
        let bundle = analyze_swing_bundle(&request);

        assert!(bundle.alignment.is_none());
        assert!(bundle.notes.iter().any(|note| note
            == "down-the-line: the clip could not be segmented into a swing, so the two views \
                cannot be aligned"));
        // And nothing `_dtl` was recorded, because both groups hang off the same missing anchors.
        assert!(!bundle
            .swing
            .measurements
            .iter()
            .any(|row| row.name.ends_with("_dtl")));
        assert!(bundle.swing.overall_score > 0.0, "the face-on half stands");
    }

    /// The two `_dtl` groups are appended to `measurements` **after** everything `_measurements`
    /// produced, which is the order `conformance.MEASUREMENT_GROUPS` records and §3 compares
    /// positionally.
    ///
    /// They go on the end and not beside their face-on partners: `analyze_swing_bundle` extends the
    /// list `analyze_swing` already built, so the `shot` and `flight` groups sit *between* the
    /// face-on rows and these. A port that interleaved them by name would read identically and fail
    /// every corpus vector.
    #[test]
    fn the_dtl_rows_are_appended_after_every_face_on_row() {
        let face_on = a_view("face_on", a_swinging_body(50));
        let dtl = a_view("down_the_line", a_rear_swinging_body(60));
        let mut request = a_bundle(&face_on);
        request.down_the_line = Some(&dtl);
        let rows = analyze_swing_bundle(&request).swing.measurements;

        let first_dtl = rows.iter().position(|row| row.name.ends_with("_dtl"));
        let last_plain = rows.iter().rposition(|row| !row.name.ends_with("_dtl"));
        let (Some(first_dtl), Some(last_plain)) = (first_dtl, last_plain) else {
            panic!("the fixture records both halves: {rows:?}");
        };
        assert!(
            last_plain < first_dtl,
            "a `_dtl` row landed inside the face-on groups: {:?}",
            rows.iter().map(|row| &row.name).collect::<Vec<_>>()
        );
        assert!(rows[first_dtl..]
            .iter()
            .all(|row| row.source == "pose:down_the_line" || row.source == POPULATION_SOURCE));

        // **And the two groups are in the engine's order within that suffix**, placements before
        // pivots. Nothing else can see this: the stage vector records the two groups *separately*, so
        // the gate compares each one and never their concatenation, and swapping the two `extend`
        // calls above passes it. The flat list is what `docs/CONFORMANCE.md` §3 compares
        // positionally, which makes the swap a wrong answer rather than a reordering.
        let last_placement = rows
            .iter()
            .rposition(|row| row.name.starts_with("tour_trajectory"));
        let first_pivot = rows[first_dtl..]
            .iter()
            .position(|row| row.name.starts_with("pivot_"))
            .map(|at| at + first_dtl);
        if let (Some(last_placement), Some(first_pivot)) = (last_placement, first_pivot) {
            assert!(
                last_placement < first_pivot,
                "the rear placements come before the rear pivots: {:?}",
                rows[first_dtl..]
                    .iter()
                    .map(|row| &row.name)
                    .collect::<Vec<_>>()
            );
        }
    }

    /// The bundle builds its `_dtl` rows from the **smoothed** rear clip and the **pinned** anchors,
    /// and no committed vector can see either.
    ///
    /// `run_stages` — and therefore `tests/alignment.rs` — does that smoothing and that anchor tuple
    /// itself before calling `dtl_placements` and `pivot_measurements`, so the two lines inside
    /// `analyze_swing_bundle` that produce them are unexercised by the gate: handing the raw frames
    /// through, or building the events off the *unpinned* anchors, passes all 21 vectors. Both are
    /// mutations this closes.
    #[test]
    fn the_dtl_rows_are_built_from_the_smoothed_clip_and_the_pinned_anchors() {
        let face_on = a_view("face_on", a_swinging_body(50));
        let dtl = a_view("down_the_line", a_rear_swinging_body(60));
        let unpinned = anchors_from_keypoints(
            &dtl.frames,
            dtl.clip.as_ref(),
            None,
            Some(crate::phases::TRAIL_WRIST),
        )
        .expect("the rear fixture segments");
        // Two frames past the pose estimate, which is inside the 0.20 s window at 60 fps.
        let strikes = [unpinned.impact + 2];
        let pinned = with_measured_impact(&unpinned, Some(&strikes));
        assert_eq!(pinned.impact, unpinned.impact + 2, "the strike moves tau=2");

        let mut request = a_bundle(&face_on);
        request.down_the_line = Some(&dtl);
        request.down_the_line_strikes = Some(&strikes);
        let rows = analyze_swing_bundle(&request).swing.measurements;
        let got: Vec<&Measurement> = rows
            .iter()
            .filter(|row| row.name.ends_with("_dtl"))
            .collect();

        let smoothed = smooth_keypoints(&dtl.frames, DEFAULT_WINDOW);
        let events = |anchors: &SwingAnchors| {
            (
                anchors.motion_start as f64,
                anchors.top as f64,
                anchors.impact as f64,
            )
        };
        let expected: Vec<Measurement> = dtl_placements(&smoothed, events(&pinned), None)
            .into_iter()
            .chain(pivot_measurements(
                &smoothed,
                events(&pinned),
                DOWN_THE_LINE,
            ))
            .collect();
        assert!(!expected.is_empty(), "the fixture records rear rows");
        assert_eq!(got.len(), expected.len());
        for (got, want) in got.iter().zip(&expected) {
            assert_eq!(*got, want);
        }

        // And both halves are load-bearing: the raw clip and the unpinned anchors each give a
        // different answer, so neither line is a formality.
        //
        // Read off the *placements* and not the pivot rows, which is itself worth recording: this
        // fixture's hips and shoulders never move, so all five rotation numbers are 0.0 and neither
        // mutation moves them. The trajectory vector reads the whole resampled body, so it does.
        let values = |rows: &[Measurement]| rows.iter().map(|row| row.value).collect::<Vec<_>>();
        let reference = values(&dtl_placements(&smoothed, events(&pinned), None));
        assert_ne!(
            values(&dtl_placements(&dtl.frames, events(&pinned), None)),
            reference,
            "smoothing changes the answer"
        );
        assert_ne!(
            values(&dtl_placements(&smoothed, events(&unpinned), None)),
            reference,
            "the pinning changes the answer"
        );
    }

    /// The two pivot views map to the two image planes, and each to its own `Measurement.source`.
    ///
    /// One lookup rather than two because the pair has to move together — and **the frame of
    /// reference reaches no number today**, so nothing else in this workspace can see it: giving the
    /// rear rows `ImagePlaneFaceOn` passes every vector and every other test here. It is carried
    /// because `PivotObservation` is what a calibrated producer will one day have to disagree with,
    /// and a row claiming the wrong plane is a row no later reader can correct.
    #[test]
    fn each_pivot_view_maps_to_its_own_plane_and_source() {
        assert_eq!(
            pivot_view(FACE_ON),
            (FrameOfReference::ImagePlaneFaceOn, POSE_FACE_ON_SOURCE)
        );
        assert_eq!(
            pivot_view(DOWN_THE_LINE),
            (FrameOfReference::ImagePlaneDtl, POSE_DTL_SOURCE)
        );
    }

    /// The rear placement mirrors a left-handed swing, which **no committed vector checks**: all
    /// fifteen two-camera vectors are right-handed.
    ///
    /// P5b's counterpart *was* gated, by `synthetic/face-on-only` — left-handed and one of the six
    /// that reach the face-on basis. There is no left-handed two-camera vector, so this is the whole
    /// of the rear mirror's coverage.
    #[test]
    fn the_rear_placement_mirrors_a_left_handed_swing() {
        let smoothed = smooth_keypoints(&a_rear_swinging_body(60), DEFAULT_WINDOW);
        let events = (5.0, 30.0, 50.0);
        let right = dtl_placements(&smoothed, events, Some(Handedness::Right));
        let left = dtl_placements(&smoothed, events, Some(Handedness::Left));
        assert_eq!(right.len(), 2, "the fixture reaches the rear basis");
        assert_eq!(left.len(), 2);
        assert_ne!(
            right.iter().map(|row| row.value).collect::<Vec<_>>(),
            left.iter().map(|row| row.value).collect::<Vec<_>>(),
        );
        // `None` is not a guessed handedness: it takes the unmirrored path, as the right-handed
        // camera frame the basis was fitted in implies.
        let unattributed = dtl_placements(&smoothed, events, None);
        assert_eq!(
            unattributed.iter().map(|row| row.value).collect::<Vec<_>>(),
            right.iter().map(|row| row.value).collect::<Vec<_>>(),
        );
    }

    /// A late top withdraws `tempo` into `unscored` and **re-combines the remaining scores**.
    ///
    /// The whole of `without_contradicted_scores`, which no committed vector reaches: it needs
    /// `top_is_late`, which needs the two views' downswings to disagree past 30%, and the widest gap
    /// the corpus produces is 27.6%. So the arithmetic below is this function's only coverage.
    #[test]
    fn a_contradicted_top_withdraws_the_tempo_score() {
        let face_on = a_clip(a_swinging_body(50));
        let swing = analyze_swing(&SwingRequest {
            swing_id: "1",
            session_id: "s",
            keypoints: &face_on.frames,
            detections: None,
            shot: None,
            intent: None,
            handedness: Some(Handedness::Right),
            loft_deg: None,
        });
        assert!(
            swing.checkpoint_scores.iter().any(|s| s.name == "tempo"),
            "the fixture scores the checkpoint this withdraws"
        );
        let before = swing.mechanics_score;

        let clip = ClipAlignment {
            anchors: some_anchors(),
            warp_motion_start: 10,
            warp_top: Some(36),
            top_late_by: Some(4),
            tau_start: 0.0,
            tau_end: 2.0,
        };
        let mut notes = Vec::new();
        let after = without_contradicted_scores(swing, &clip, &mut notes);

        assert!(!after.checkpoint_scores.iter().any(|s| s.name == "tempo"));
        let withdrawn = after
            .unscored
            .iter()
            .find(|entry| entry.name == "tempo")
            .expect("it moved rather than vanished");
        assert_eq!(withdrawn.reason, UnscoredReason::CrossViewContradicted);
        assert_eq!(
            withdrawn.detail,
            "the other view, synchronized on the ball strike, puts the top 4 frames earlier than \
             this clip did"
        );
        assert_eq!(notes.len(), 1);
        assert!(
            notes[0].starts_with("tempo withdrawn: it scored ")
                && notes[0].ends_with(
                    " off a top the down-the-line view puts 4 frames earlier on a shared clock"
                ),
            "{:?}",
            notes[0]
        );
        // **The score is recombined, not merely shortened.** A port that dropped the checkpoint and
        // left `mechanics_score` alone would report an average over a checkpoint it just withdrew.
        assert_ne!(after.mechanics_score, before);
        assert_eq!(after.mechanics_score, Some(after.overall_score));
    }

    /// `unscored` stays in **registry order** after the withdrawal, which is the one ordering
    /// `contracts::checkpoints` asks for.
    ///
    /// `tempo` is the *first* registry entry, so appending it would put it last. The fixture makes a
    /// swing that already has an unscored entry — no handedness retires `head_stays_back` — and
    /// checks the withdrawn `tempo` sorts in front of it.
    #[test]
    fn a_withdrawn_checkpoint_sorts_into_registry_order() {
        let face_on = a_clip(a_swinging_body(50));
        let swing = analyze_swing(&SwingRequest {
            swing_id: "1",
            session_id: "s",
            keypoints: &face_on.frames,
            detections: None,
            shot: None,
            intent: None,
            handedness: None,
            loft_deg: None,
        });
        assert!(
            !swing.unscored.is_empty(),
            "no handedness retires a checkpoint, which is what makes the order visible"
        );
        let clip = ClipAlignment {
            anchors: some_anchors(),
            warp_motion_start: 10,
            warp_top: Some(36),
            top_late_by: Some(4),
            tau_start: 0.0,
            tau_end: 2.0,
        };
        let after = without_contradicted_scores(swing, &clip, &mut Vec::new());

        let registry = checkpoint_names();
        let ranks: Vec<usize> = after
            .unscored
            .iter()
            .map(|entry| registry_rank(&entry.name, &registry))
            .collect();
        assert!(ranks.windows(2).all(|pair| pair[0] <= pair[1]), "{ranks:?}");
        assert_eq!(after.unscored[0].name, "tempo", "the first registry entry");
    }

    /// A late top on a swing with **nothing to withdraw** returns the result untouched.
    ///
    /// The early return matters because the alternative is re-running the scoring policy over the
    /// same list and writing three fields back — which is a no-op today and would stop being one the
    /// moment the policy is not a plain mean.
    #[test]
    fn a_late_top_with_no_contradicted_score_changes_nothing() {
        let face_on = a_clip(a_swinging_body(50));
        let swing = analyze_swing(&SwingRequest {
            swing_id: "1",
            session_id: "s",
            keypoints: &face_on.frames,
            detections: None,
            shot: None,
            intent: None,
            handedness: Some(Handedness::Right),
            loft_deg: None,
        });
        let mut stripped = swing.clone();
        stripped
            .checkpoint_scores
            .retain(|score| !CONTRADICTED_BY_A_LATE_TOP.contains(&score.name.as_str()));

        let clip = ClipAlignment {
            anchors: some_anchors(),
            warp_motion_start: 10,
            warp_top: None,
            top_late_by: Some(4),
            tau_start: 0.0,
            tau_end: 2.0,
        };
        let mut notes = Vec::new();
        let after = without_contradicted_scores(stripped.clone(), &clip, &mut notes);
        assert_eq!(after, stripped);
        assert!(notes.is_empty());
    }

    /// `_measurements` is the five groups concatenated in order, which is what
    /// `docs/CONFORMANCE.md` §3 compares positionally.
    ///
    /// **The shot and the flight go after the three face-on groups and in that order** [M22 P8b],
    /// which the committed vectors do gate — but only where a shot is present, and the assembled
    /// answer cannot show that `measurements` reads the same `FlownShot` the `unscored` list did.
    /// Built here from the group functions directly so a reordering inside `measurements` fails
    /// without a vector.
    #[test]
    fn the_measurement_groups_are_concatenated_in_the_engines_order() {
        let frames = a_body(50);
        let phases = a_segmentation();
        let mut shot = a_shot("null");
        shot.ball_speed = Some(89.8);
        shot.launch_angle = Some(22.4);
        shot.carry_distance = Some(126.1);
        shot.spin_rate = Some(5991.0);
        let flown = fly_shot(
            &shot,
            None,
            Some(Handedness::Right),
            load_flight_model(),
            DEFAULT_STEP_S,
        );

        let (pose, placed, pivots) = face_on_measurements(&frames, &phases, None);
        let flat = measurements(&frames, &phases, Some(&shot), None, Some(&flown));
        let expected: Vec<String> = pose
            .iter()
            .chain(&placed)
            .chain(&pivots)
            .chain(&shot_measurements(&shot))
            .chain(&flight_measurements(&flown))
            .map(|row| row.name.clone())
            .collect();
        assert_eq!(
            flat.iter().map(|row| row.name.clone()).collect::<Vec<_>>(),
            expected
        );
        // And both new groups actually contributed, so the check is not vacuous.
        assert!(flat.iter().any(|row| row.name == "carry_distance_yds"));
        assert!(flat.iter().any(|row| row.name == "flight_carry_yds"));
    }

    /// **The source is per shot**, and it is the one string in `measurements` that is not a
    /// constant or a registry entry: `launch_monitor:{device}` off the OCR parse's provenance, or
    /// the source enum where a shot arrived without one.
    ///
    /// All fifteen corpus vectors carry a `screen` provenance whose device is `hd_golf`, so the
    /// fallback is reached by no committed vector at all.
    #[test]
    fn the_shot_source_names_the_device_and_falls_back_to_the_source_enum() {
        let mut shot = a_shot(r#"{"device": "hd_golf", "parse_confidence": 1.0}"#);
        shot.carry_distance = Some(126.1);
        assert_eq!(
            shot_measurements(&shot)[0].source,
            "launch_monitor:hd_golf".to_string()
        );

        let mut bare = a_shot("null");
        bare.carry_distance = Some(126.1);
        assert_eq!(
            shot_measurements(&bare)[0].source,
            "launch_monitor:screen".to_string()
        );
    }

    /// Every row in both new groups is rounded to four decimals on the way in, which is
    /// `_measurements`' own `round(value, 4)` and is why the `measure` stage records the *unrounded*
    /// value separately.
    #[test]
    fn the_outcome_rows_are_rounded_to_four_decimals() {
        let mut shot = a_shot("null");
        shot.carry_distance = Some(125.6);
        shot.launch_direction = Some(-5.3);
        shot.ball_speed = Some(90.7);
        shot.launch_angle = Some(20.9);
        shot.spin_rate = Some(5991.0);
        let flown = fly_shot(&shot, None, None, load_flight_model(), DEFAULT_STEP_S);

        for row in shot_measurements(&shot)
            .iter()
            .chain(&flight_measurements(&flown))
        {
            assert_eq!(row.value, round_to(row.value, 4), "{}", row.name);
        }
        // The offline is the one shot row with digits past the fourth to lose.
        let offline = shot_measurements(&shot)
            .into_iter()
            .find(|row| row.name == "start_line_offline_yds")
            .expect("both tiles printed");
        assert_eq!(offline.value, -11.6017);
    }

    /// **A swing with no shot records neither group and refuses nothing**, which is every synthetic
    /// vector: the flight is not flown at all, so there is no `flight_carry_yds` entry to explain.
    #[test]
    fn a_swing_with_no_shot_has_no_outcome_rows_and_no_flight_refusal() {
        let clip = a_clip(a_swinging_body(50));
        let result = analyze_swing_bundle(&a_bundle(&clip)).swing;
        assert!(!result
            .measurements
            .iter()
            .any(|row| row.source.starts_with("launch_monitor:")
                || row.source == crate::flight_measure::FLIGHT_SOURCE));
        assert!(!result
            .unscored
            .iter()
            .any(|entry| entry.name.starts_with("flight_")));
    }

    /// **The flight's refusals come last in `unscored`**, after every checkpoint the registry walk
    /// produced — which `docs/CONFORMANCE.md` §3 compares positionally and the corpus half gates on
    /// eleven of fifteen. Pinned here too because the ordering is a one-line choice in
    /// `analyze_swing` with nothing else in the file to protect it.
    #[test]
    fn a_flight_refusal_is_appended_after_every_checkpoint_entry() {
        let clip = a_clip(a_swinging_body(50));
        let mut shot = a_shot("null");
        shot.ball_speed = Some(89.8);
        shot.launch_angle = Some(22.4);
        shot.carry_distance = Some(400.0);
        let mut request = a_bundle(&clip);
        request.shot = Some(&shot);
        // No handedness either, so a checkpoint refuses too and there is something to come after.
        let unscored = analyze_swing_bundle(&request).swing.unscored;

        let flight_at = unscored
            .iter()
            .position(|entry| entry.name == "flight_carry_yds")
            .expect("the carry is unreachable, so the flight refused");
        assert_eq!(flight_at, unscored.len() - 1);
        assert!(flight_at > 0, "the fixture also refuses a checkpoint");
        assert!(unscored[..flight_at]
            .iter()
            .all(|entry| checkpoint_names().contains(&entry.name.as_str())));
    }

    /// The intent reaches the checkpoint loop, which the six vectors cannot show: all six pass
    /// none, so every club lookup on them is the default. Mutating the pass-through to `None` passes
    /// the gate.
    #[test]
    fn the_intent_reaches_the_result_it_was_judged_against() {
        let clip = a_clip(a_swinging_body(50));
        let intent = PracticeGoal {
            club: contracts::intent::ClubCategory::MidIron,
            ..PracticeGoal::default()
        };
        let mut request = a_bundle(&clip);
        request.intent = Some(&intent);
        let bundle = analyze_swing_bundle(&request);
        assert_eq!(bundle.swing.intent, Some(intent));
        // And no intent is the Fundamentals default rather than an absence.
        assert_eq!(
            analyze_swing_bundle(&a_bundle(&clip)).swing.intent,
            Some(PracticeGoal::default())
        );
    }

    /// The raw keypoints ride on the result as the data it was computed from — *not* the smoothed
    /// ones, which is the difference between an artifact a later run can re-analyse and one that has
    /// already been filtered.
    #[test]
    fn the_result_carries_the_raw_keypoints_and_not_the_smoothed_ones() {
        let mut frames = a_swinging_body(50);
        frames[10].camera_id = Some("face-on".to_string());
        let clip = a_clip(frames.clone());
        let bundle = analyze_swing_bundle(&a_bundle(&clip));
        assert_eq!(bundle.swing.keypoints, frames);
        // `smooth_keypoints` drops `camera_id`, so this is also the check that no smoothing happened
        // on the way out.
        assert_eq!(
            bundle.swing.keypoints[10].camera_id.as_deref(),
            Some("face-on")
        );
    }

    /// **The `tempo_notes` call site, which nothing in `spec/vectors/` reaches — either half.**
    ///
    /// Removing the call passes all 21 committed vectors, and the reason is sharper than "the
    /// synthetic six are clean swings": every collapsed motion-start boundary in the *corpus* is in
    /// the **down-the-line** view, and `tempo_notes` reads the face-on anchors alone. The corpus
    /// notes that look like this one — `alignment: down_the_line: backswing measures 0.05
    /// downswings…` — are `align_swings`' own sentence, which is a different string in a different
    /// function and is P7's. `aaron-1`, the swing this note was written for, is on disk as
    /// `corpus/2026-08-07-aaron1-1` and its face-on view is fine.
    ///
    /// So this test is the only thing anywhere that says the sentence a golfer reads is wired to the
    /// result they read it on. It also pins the *ordering* the Python comment insists on: the tempo
    /// note comes before the strike is read, because it is about the **tempo checkpoint** and the
    /// checkpoint was scored off the pose phases — pinning impact to the sound first would have the
    /// note quote a ratio no checkpoint on this swing was measured from.
    #[test]
    fn an_implausible_tempo_reaches_the_bundles_notes() {
        let clip = a_clip(a_body_that_pauses_at_the_top(60));
        let bundle = analyze_swing_bundle(&a_bundle(&clip));
        let tempo_note = bundle
            .notes
            .iter()
            .find(|note| note.contains("downswings, which no golf swing does"))
            .expect("the pause collapsed the boundary and nothing said so");
        assert_eq!(
            tempo_note,
            "face-on: the backswing measures 0.50 downswings, which no golf swing does — the \
             motion-start boundary has collapsed onto the top (a pause at the top reads as the \
             quiet stretch the takeaway is measured back to). Any tempo score on this swing is \
             measuring that artifact, not the golfer; ignore it"
        );

        // Before the one-camera note, which is appended after the anchors are pinned.
        let positions: Vec<usize> = bundle
            .notes
            .iter()
            .enumerate()
            .filter(|(_, n)| *n == tempo_note || n.as_str() == NO_SECOND_VIEW_NOTE)
            .map(|(i, _)| i)
            .collect();
        assert_eq!(positions.len(), 2);
        assert!(positions[0] < positions[1]);

        // **The note is independent of the verdict**, which this fixture shows by accident and is
        // worth keeping: on it `tempo` refuses outright with `BoundaryEstimated`, and the note still
        // fires. `tempo_notes` reads the anchors, not the score — so it speaks on the swing where
        // the checkpoint refused *and* on the one where it scored a number off a collapsed boundary,
        // which is the case the sentence was written for and which no fixture here reproduces.
        assert!(bundle
            .swing
            .unscored
            .iter()
            .any(|entry| entry.name == "tempo"
                && entry.reason == contracts::unscored::UnscoredReason::BoundaryEstimated));
    }

    /// **The `anchored_on_strike` call site**, which no vector reaches either: none of the six carries
    /// a `face_on_strikes` list, so passing `None` there passes the whole gate.
    #[test]
    fn a_heard_strike_reaches_the_bundles_notes() {
        let frames = a_swinging_body(80);
        let clip = KeypointsFile {
            clip: Some(contracts::keypoints::ClipMetadata {
                fps: Some(60.0),
                width: None,
                height: None,
                frame_count: Some(frames.len() as i64),
                source_sha256: None,
            }),
            frames,
            pose_estimator: None,
        };

        // Where pose thinks impact is, so the strike below is a real correction rather than a guess.
        let unstruck = analyze_swing_bundle(&a_bundle(&clip));
        let pose_impact = unstruck
            .swing
            .phases
            .iter()
            .find(|segment| segment.phase == SwingPhase::Impact)
            .expect("the fixture segments")
            .start_frame;

        let strikes = [pose_impact + 3];
        let mut request = a_bundle(&clip);
        request.face_on_strikes = Some(&strikes);
        let bundle = analyze_swing_bundle(&request);
        assert_eq!(
            bundle.notes.first().map(String::as_str),
            Some(
                "face-on: impact moved +3 frames (+0.050s) onto the ball strike heard in this clip \
                 — pose had it early. tau=2 is now a measured instant, not an estimate"
            )
        );
        // **No score moved**, which is the property that lets M11 improve the anchor without
        // re-opening M4's scoring: `analyze_swing` had already scored the face-on view off its own
        // phases by the time the strike was read.
        assert_eq!(
            bundle.swing.checkpoint_scores,
            unstruck.swing.checkpoint_scores
        );
        assert_eq!(bundle.swing.phases, unstruck.swing.phases);
    }
}
