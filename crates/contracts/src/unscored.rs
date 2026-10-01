//! Why a checkpoint has no score, and the remedy each cause deserves. `contracts/unscored.py`.
//! [M22 P2, prose table M22 P6]
//!
//! ADR-010 §2 says a checkpoint that cannot be measured returns nothing rather than a guess.
//! ADR-013 added the disclosure half: it is *named*, with the reason, so `overall_score` being a
//! mean over five and a mean over six are distinguishable. [`UNSCORED_REASONS`] is the third part
//! of that argument — **which** of the things that can go wrong did, and what to tell the golfer
//! about it.
//!
//! **The prose table arrived a phase after the enum, and that was P2's rule rather than an
//! oversight.** A round-trip gate proves a *shape* carries every field on disk and can say nothing
//! about a table, so each of `contracts/`'s tables landed beside the code that walks it:
//! `CHECKPOINT_REGISTRY` with `mechanics` in P5, the two placement registries with their groups in
//! P5b, and this one with `feedback::rules` in P6 — which interpolates [`ReasonSpec::remedy`] into
//! a `Tip.text` `docs/CONFORMANCE.md` §3 compares exactly.
//!
//! # `MEASUREMENT_REASONS` is deliberately absent
//!
//! Python carries a third set beside [`INFERENCE_REASONS`] — the reasons `analysis/measure.py` is
//! allowed to report — and its only reader is `tests/analysis/test_measure.py`, which pins that the
//! measuring half never reaches for a judging reason. Nothing on the ported surface walks it, so
//! bringing it here would be a table no gate in this workspace can see, which is the same rule that
//! kept this one out of P2. It follows its test, not this module.
//!
//! # The photo-side reasons are Rust's alone
//!
//! [`UnscoredReason::PrintedBlank`] and [`UnscoredReason::Misread`] arrived in M32 (the M31.5
//! plan's carried decision 5) for the shot-first product, where the photo of the launch monitor's
//! screen is the capture and one tile on one shot can fail in two ways a golfer must be told apart:
//! the screen printed nothing there, or it printed something the photo did not give up. ADR-034 §2
//! routes "printed but blank on one shot" through this table, and `Misread` is the case beside it.
//!
//! **Python has neither and never will.** `contracts/unscored.py` is frozen (ADR-035 clause 4) and
//! its `UnscoredReason` is closed, so a `SwingResult` carrying one would be refused by every frozen
//! reader. Nothing emits either in M32; M35 and M37 are the first, into `shot_analysis.json`,
//! never `analysis.json`. Neither is in [`INFERENCE_REASONS`]: they are about reading the screen,
//! where the inference family is about what the screen's numbers can and cannot be made to fly.
//!
//! # Why the vocabulary lives in `contracts/` at all
//!
//! `feedback` may not import `analysis` (ADR-008), which is why `rules.py` used to hold checkpoint
//! names as bare string literals with a comment apologising for it, and why it used to *infer* the
//! remedy from a side channel: if the metric survived into `measurements` the footage must have been
//! readable, so a missing `head_stays_back` score had to be the handedness case. That worked for
//! exactly one checkpoint and could never work for a second, because the signal it read was what
//! happened to be measurable beside the failure rather than the failure. The cause lives here, so
//! every consumer does a lookup.

use serde::{Deserialize, Serialize};

use crate::{ContractError, Validate};

/// Why one checkpoint produced no score.
///
/// The `refilming_helps` split ADR-027 cares about is a property of the reason and not of the
/// checkpoint, which is why consumers branch on this rather than on a name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnscoredReason {
    /// A phase the checkpoint reads is absent from the segmentation, so its window was never
    /// located.
    PhaseNotSegmented,
    /// The start of the backswing was **estimated rather than detected** (ADR-013). That estimate
    /// is derived from an assumed tempo ratio, so a tempo reported from it would echo the
    /// assumption back as an observation.
    BoundaryEstimated,
    /// The instants landed in an order that gives a non-positive backswing or downswing duration.
    /// Distinct from `PhaseNotSegmented`: every phase is present, they just disagree.
    TimingDegenerate,
    /// The landmarks this checkpoint reads never cleared the visibility gate across the window it
    /// needs.
    LandmarksUnconfident,
    /// The window held usable frames, but fewer than the minimum the statistic needs. Separate from
    /// `LandmarksUnconfident` because "a few good frames, not enough of them" and "no good frames at
    /// all" are different clips.
    TooFewFrames,
    /// Shoulder width — the ruler every `_norm` metric divides by — was unmeasurable or degenerate.
    /// The quantity may well have been readable; there was no scale to express it in.
    ScaleUnavailable,
    /// The swing was measured fine and no benchmark band resolved. Not a capture problem: the number
    /// is still recorded in `measurements`, which is the order M6.5 exists to allow.
    NoBand,
    /// No golfer is attributed and this checkpoint's sign is camera-relative. Guessing right-handed
    /// would score a left-handed golfer's ordinary impact position as a gross fault.
    NoHandedness,
    /// The other view, on a shared clock, contradicts the instants this checkpoint was timed from.
    /// Only reachable on a bundle whose two clips **both heard the ball strike** (M11 P7).
    CrossViewContradicted,
    /// No spin at all reproduces the carry the launch monitor printed, from the launch conditions it
    /// printed beside it. A finding about the two models rather than about the golfer.
    CarryUnreachable,
    /// Two spins fly that carry, one either side of the peak, and no loft is on record to choose
    /// between them (ADR-027 §Decision 3). The one reason in this family a golfer can clear.
    NoClubLoft,
    /// The carry does not pin a spin the club could have produced; `detail` says which shape.
    SpinNotRecoverable,
    /// Nothing on the screen fixes how far the spin axis is tilted, so the flight is simulated in the
    /// vertical plane and its landing offline is withheld. A direction with no magnitude is not an
    /// axis.
    SpinAxisUnresolved,
    /// The screen printed too little of the launch to fly anything.
    NoLaunchConditions,
    /// The stored result predates reasons being recorded at all. **Never produced by the engine** —
    /// it exists so a tolerant reader can say "this artifact does not know" instead of inventing a
    /// cause or dropping the entry.
    Unrecorded,
    /// The launch monitor printed this number's tile and left it empty, or printed `---`, on this
    /// shot. ADR-034 §2's "printed but blank on one shot": the device declares the field and the
    /// golfer's screen shows it, so the shot is named rather than guessed — and nothing about the
    /// photo went wrong, because another photo of the same screen reads the same blank.
    ///
    /// **Photo-side, and never written into a [`crate::swing::SwingResult`].** Frozen Python's
    /// `UnscoredReason` is a closed enum, so an `analysis.json` carrying this would be refused by
    /// every Python reader of it (ADR-035 clause 4). M35 and M37 are the first to emit it, into
    /// `shot_analysis.json`; nothing does in M32. [M32 P7]
    PrintedBlank,
    /// Text sat under the tile and no value could be read from it. The screen *did* print
    /// something, which is what separates this from [`Self::PrintedBlank`] and why it is the
    /// photo-side twin of the capture reasons: a sharper, square-on photo may recover it.
    ///
    /// Photo-side and never in a `SwingResult`, for [`Self::PrintedBlank`]'s reason; M35 and M37
    /// are the first to emit it. [M32 P7]
    Misread,
}

/// What one reason means, and what to tell the golfer about it.
///
/// A `const` struct of `&'static str`, the same shape [`crate::checkpoints::CheckpointSpec`] takes
/// and for the same reason: it is a compile-time constant, never parsed from a boundary, so it
/// carries no [`Validate`] impl.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReasonSpec {
    /// One clause, for a brief or a table cell. No leading capital and no full stop — callers embed
    /// it in a sentence of their own.
    pub summary: &'static str,
    /// The golfer-facing sentence, and the whole point of the module: it is chosen by cause rather
    /// than by inference, so it can be specific without being a guess. `feedback::rules` puts this
    /// on a `Tip` verbatim, which is what makes it a compared string.
    pub remedy: &'static str,
    /// Whether capturing again — the clip, or for [`UnscoredReason::Misread`] the photo of the
    /// screen — could plausibly fix this. The one bit every consumer needs and none could
    /// previously derive — `NoBand` and `NoHandedness` are not capture problems, and telling those
    /// golfers to steady their camera answers a question they did not ask.
    ///
    /// The name is the clip's because consumers already read it by that name (the MCP server's
    /// instructions promise a `refilming_helps` flag), and a photo-side reason reuses it rather
    /// than adding a second bit that every consumer would have to learn to `or` with the first.
    pub refilming_helps: bool,
}

/// Reason -> what it means and what to do. One table, so the prose cannot drift between the tip,
/// the brief, the MCP view and the results page.
///
/// **A slice of pairs in the Python dict's order, not a map.** Entries looked up by a reason that
/// came off an `UnscoredCheckpoint`, which is the shape [`crate::checkpoints::CHECKPOINT_REGISTRY`]
/// and `CHECKPOINT_EVALUATORS` already use — and a `HashMap` would buy nothing here but a
/// nondeterministic iteration order in a crate where order is repeatedly an answer. Nothing
/// iterates it today; keeping the Python's order is what makes a future iteration agree rather
/// than something to discover later.
///
/// **The photo-side rows come last, after `Unrecorded`**, so the Python dict's rows stay an
/// unbroken prefix in its own order: a walk that stops where the Python's table ends still agrees
/// with it row for row. Grouping them beside the capture reasons they resemble would have read
/// better and broken that.
pub static UNSCORED_REASONS: &[(UnscoredReason, ReasonSpec)] = &[
    (
        UnscoredReason::PhaseNotSegmented,
        ReasonSpec {
            summary: "the swing could not be split into the phases this checkpoint reads",
            remedy: "The swing could not be broken into its phases, so the part of it this \
                         checkpoint looks at was never located. Try a clip with the whole swing in \
                         frame, from address through the finish, and the camera steady.",
            refilming_helps: true,
        },
    ),
    (
        UnscoredReason::BoundaryEstimated,
        ReasonSpec {
            summary: "the start of the backswing was estimated rather than detected",
            remedy: "The swing itself was readable - but the moment the backswing starts had to be \
                         estimated rather than detected, and timing measured from an estimate would \
                         only repeat the assumption back to you. A clip that starts with you already \
                         settled over the ball gives it something to find.",
            refilming_helps: true,
        },
    ),
    (
        UnscoredReason::TimingDegenerate,
        ReasonSpec {
            summary: "the detected instants give a non-positive phase duration",
            remedy: "The moments this checkpoint times came out in an impossible order, which \
                         usually means a practice swing or a second movement in the clip was picked up \
                         instead. Try a clip containing one swing.",
            refilming_helps: true,
        },
    ),
    (
        UnscoredReason::LandmarksUnconfident,
        ReasonSpec {
            summary: "the body landmarks it reads were never confidently visible in that window",
            remedy: "The body points this checkpoint tracks were never clearly visible for long \
                         enough to measure. Better light, and nothing between you and the camera, is \
                         what usually fixes it.",
            refilming_helps: true,
        },
    ),
    (
        UnscoredReason::TooFewFrames,
        ReasonSpec {
            summary: "the window held fewer usable frames than the measurement needs",
            remedy: "There were some usable frames but not enough of them to measure this reliably. \
                         Let the clip keep running through the finish rather than cutting it at impact.",
            refilming_helps: true,
        },
    ),
    (
        UnscoredReason::ScaleUnavailable,
        ReasonSpec {
            summary: "shoulder width, the ruler this metric is expressed in, was unmeasurable",
            remedy: "This checkpoint is measured in shoulder widths, and your shoulder width could \
                         not be read from this clip - usually a camera that is not square to you, or \
                         shoulders the pose model lost. Film face-on, with the camera side-on to the \
                         target line.",
            refilming_helps: true,
        },
    ),
    (
        UnscoredReason::NoBand,
        ReasonSpec {
            summary: "no benchmark band exists for it yet",
            remedy: "The swing was measured fine - there is just no tour benchmark for this \
                         checkpoint yet, so there is nothing to judge the number against. Nothing to \
                         fix; the measurement is recorded either way.",
            refilming_helps: false,
        },
    ),
    (
        UnscoredReason::NoHandedness,
        ReasonSpec {
            summary: "no golfer is attributed, and this checkpoint's sign depends on which side you \
                          swing from",
            remedy: "The swing itself was measured fine - this checkpoint also needs to know which \
                         side you swing from, and no golfer is attributed to this swing. Pick a golfer \
                         for the session and it will score without re-filming.",
            refilming_helps: false,
        },
    ),
    (
        UnscoredReason::CrossViewContradicted,
        ReasonSpec {
            summary: "the other camera, synchronized on the ball strike, contradicts its timing",
            remedy: "Both cameras heard the strike, so the two clips share a real clock - and on \
                         that clock they disagree about when the backswing ended. The face-on view puts \
                         the top later than the other camera does, which would make this reading a \
                         comparison between one instant that is right and one that is not. Nothing to \
                         re-film: both clips are good, and it is the swing's own pause at the top that \
                         makes the moment hard to place.",
            refilming_helps: false,
        },
    ),
    (
        UnscoredReason::CarryUnreachable,
        ReasonSpec {
            summary: "no spin flies the ball the distance the launch monitor printed",
            remedy: "The launch conditions and the carry the simulator printed do not fit together: \
                         no spin rate at all makes this model fly the ball that far. Nothing to re-film \
                         and probably nothing you did - the two flight models disagree by about as much \
                         as this shot misses by.",
            refilming_helps: false,
        },
    ),
    (
        UnscoredReason::NoClubLoft,
        ReasonSpec {
            summary: "two spins fly that carry and no club loft is on record to choose between them",
            remedy: "Two different spin rates fly this ball exactly as far as the simulator says it \
                         went, and the club's loft is what decides which one it was. Tag this swing \
                         with the club you hit, and fill that club's loft into your bag, and it \
                         resolves without re-filming.",
            refilming_helps: false,
        },
    ),
    (
        UnscoredReason::SpinNotRecoverable,
        ReasonSpec {
            summary: "the printed carry does not pin a spin the club could have produced",
            remedy: "The carry is reachable, but not by a spin rate this club plausibly makes - \
                         either every spin above a threshold flies exactly this far, or the only one \
                         that fits is far below anything a struck golf shot spins at. Nothing to fix; a \
                         launch monitor that prints spin is what settles it.",
            refilming_helps: false,
        },
    ),
    (
        UnscoredReason::SpinAxisUnresolved,
        ReasonSpec {
            summary: "the screen printed which way the ball curved but not how far the axis is \
                          tilted",
            remedy: "Which way this shot curved is known - the simulator prints it in words - but \
                         nothing on the screen says how much the spin axis was tilted, so the flight is \
                         drawn straight and where it finished sideways is left blank. Nothing to \
                         re-film.",
            refilming_helps: false,
        },
    ),
    (
        UnscoredReason::NoLaunchConditions,
        ReasonSpec {
            summary: "the screen did not print enough of the launch for a flight to be simulated",
            remedy: "Drawing the ball's path needs the ball speed and the launch angle, and this \
                         shot did not arrive with both of them - so there is no flight to draw. Nothing \
                         about the swing: the number never came off the photo of the simulator's \
                         screen, and re-reading that photo is what would recover it.",
            refilming_helps: false,
        },
    ),
    (
        UnscoredReason::Unrecorded,
        ReasonSpec {
            summary: "this result was produced before the reason was recorded",
            remedy: "This result was analyzed before the reason was recorded, so all we know is \
                         that the checkpoint could not be scored. Re-running the analysis will say why.",
            refilming_helps: false,
        },
    ),
    (
        UnscoredReason::PrintedBlank,
        ReasonSpec {
            summary: "the launch monitor printed nothing for this number on this shot",
            remedy: "The simulator showed the tile for this number but left it blank on this shot - \
                         empty, or dashes - so there was no value to read. Nothing went wrong with the \
                         photo: another photo of the same screen would read the same blank.",
            refilming_helps: false,
        },
    ),
    (
        UnscoredReason::Misread,
        ReasonSpec {
            summary: "the number on the screen could not be read from the photo",
            remedy: "There was text under this tile on the screen, but no number could be read from \
                         it in the photo. A sharper photo, taken square-on to the screen, may fix it.",
            refilming_helps: true,
        },
    ),
];

/// The prose registered for `reason`.
///
/// Panics on a miss, the way [`crate::checkpoints::spec_for`] does: Python indexes the dict
/// directly and a `KeyError` there means a member was added without a row, which is a wiring bug
/// rather than a data condition. [`every_reason_has_a_row`] is what makes the panic unreachable.
pub fn spec_for(reason: UnscoredReason) -> &'static ReasonSpec {
    UNSCORED_REASONS
        .iter()
        .find(|(registered, _)| *registered == reason)
        .map(|(_, spec)| spec)
        .unwrap_or_else(|| panic!("no prose registered for {reason:?}"))
}

/// The reasons the **ball-flight inference** may report (ADR-027 §Decisions 3 and 5, M15 P9).
///
/// Nothing about a swing produces one of these; they come from `analysis::flight_infer` failing to
/// recover a launch condition the launch monitor's screen did not print. `feedback::rules` is the
/// ported reader and it uses them as an *exclusion*: a flight refusal rides `SwingResult.unscored`
/// because that is where this repo names an absence, but a `Tip` carries a `checkpoint` and a
/// simulated flight is not one — so `_unmeasured_tip`'s sentence, *"so it is not included in the
/// score"*, would be false of every `flight_*` measurement, which was never in `overall_score` to be
/// excluded from.
///
/// **Four reasons over `SpinSolveCase`'s seven shapes**, and the split is deliberate: the criterion
/// is what the *reader* must do, and for three of the four the answer is "nothing", so the seven
/// cases collapse onto them and `UnscoredCheckpoint::detail` carries which one it was. (Five
/// members, not four — `NoLaunchConditions` joined them; the Python's own comment counts the spin
/// solve's share.)
pub static INFERENCE_REASONS: &[UnscoredReason] = &[
    UnscoredReason::CarryUnreachable,
    UnscoredReason::NoClubLoft,
    UnscoredReason::SpinNotRecoverable,
    UnscoredReason::SpinAxisUnresolved,
    UnscoredReason::NoLaunchConditions,
];

/// Whether `reason` came from the ball-flight inference rather than from a checkpoint.
///
/// The membership test `feedback::rules` runs per `unscored` entry, spelled once so no caller
/// writes a five-way `matches!` that a sixth member would silently fall out of.
pub fn is_inference_reason(reason: UnscoredReason) -> bool {
    INFERENCE_REASONS.contains(&reason)
}

/// One checkpoint that was attempted and produced no score, with the reason in both forms.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnscoredCheckpoint {
    /// The registered checkpoint name, taken from the registry rather than retyped, so it cannot
    /// disagree with the `CheckpointScore` the same spec would have produced.
    pub name: String,
    pub reason: UnscoredReason,
    /// Which window or landmark group, in the words of the code that failed. The reason is what a
    /// consumer branches on; this is the part a human needs to tell two clips apart.
    #[serde(default)]
    pub detail: String,
}

impl Validate for UnscoredCheckpoint {
    fn validate(&self) -> Result<(), ContractError> {
        Ok(())
    }
}

impl UnscoredCheckpoint {
    /// The prose for this entry's reason. A method so no consumer indexes the table by hand — the
    /// Python's `spec` property, and the reason the table is reachable from a payload at all.
    pub fn spec(&self) -> &'static ReasonSpec {
        spec_for(self.reason)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every member of the enum, hand-listed because Rust has no reflection over one.
    ///
    /// Two tests read it and that is the point: the wire-name pin and the prose-row pin then fail on
    /// the *same* omission, so adding a member without spelling it here is the only way to get past
    /// either — and a `_ =>` arm nobody wrote cannot swallow it. [M22 P6]
    fn every_reason() -> Vec<UnscoredReason> {
        vec![
            UnscoredReason::PhaseNotSegmented,
            UnscoredReason::BoundaryEstimated,
            UnscoredReason::TimingDegenerate,
            UnscoredReason::LandmarksUnconfident,
            UnscoredReason::TooFewFrames,
            UnscoredReason::ScaleUnavailable,
            UnscoredReason::NoBand,
            UnscoredReason::NoHandedness,
            UnscoredReason::CrossViewContradicted,
            UnscoredReason::CarryUnreachable,
            UnscoredReason::NoClubLoft,
            UnscoredReason::SpinNotRecoverable,
            UnscoredReason::SpinAxisUnresolved,
            UnscoredReason::NoLaunchConditions,
            UnscoredReason::Unrecorded,
            UnscoredReason::PrintedBlank,
            UnscoredReason::Misread,
        ]
    }

    /// Every member's wire form, pinned against `contracts/unscored.py`'s `StrEnum` values.
    ///
    /// `rename_all = "snake_case"` is a rule, and a rule is a thing that can be right about every
    /// member but one. These are the strings on disk in every stored `analysis.json`, so getting one
    /// wrong is an artifact this port cannot read.
    ///
    /// **The last two have no Python twin, by design** (M32 P7). `printed_blank` and `misread` are
    /// photo-side and never reach an `analysis.json` (ADR-035 clause 4), so there is no Python value
    /// to pin them against; they are pinned here anyway, because `shot_analysis.json` (M35) is where
    /// these strings will sit on disk.
    #[test]
    fn the_reason_wire_names_are_the_python_values() {
        let pairs = [
            (UnscoredReason::PhaseNotSegmented, "phase_not_segmented"),
            (UnscoredReason::BoundaryEstimated, "boundary_estimated"),
            (UnscoredReason::TimingDegenerate, "timing_degenerate"),
            (
                UnscoredReason::LandmarksUnconfident,
                "landmarks_unconfident",
            ),
            (UnscoredReason::TooFewFrames, "too_few_frames"),
            (UnscoredReason::ScaleUnavailable, "scale_unavailable"),
            (UnscoredReason::NoBand, "no_band"),
            (UnscoredReason::NoHandedness, "no_handedness"),
            (
                UnscoredReason::CrossViewContradicted,
                "cross_view_contradicted",
            ),
            (UnscoredReason::CarryUnreachable, "carry_unreachable"),
            (UnscoredReason::NoClubLoft, "no_club_loft"),
            (UnscoredReason::SpinNotRecoverable, "spin_not_recoverable"),
            (UnscoredReason::SpinAxisUnresolved, "spin_axis_unresolved"),
            (UnscoredReason::NoLaunchConditions, "no_launch_conditions"),
            (UnscoredReason::Unrecorded, "unrecorded"),
            (UnscoredReason::PrintedBlank, "printed_blank"),
            (UnscoredReason::Misread, "misread"),
        ];
        for (variant, wire) in pairs {
            assert_eq!(
                serde_json::to_string(&variant).unwrap(),
                format!("\"{wire}\"")
            );
        }
        assert_eq!(
            pairs.map(|(reason, _)| reason).to_vec(),
            every_reason(),
            "a member was added without a wire pin"
        );
    }

    #[test]
    fn detail_defaults_to_empty_rather_than_absent() {
        let entry: UnscoredCheckpoint =
            serde_json::from_str(r#"{"name":"tempo","reason":"no_band"}"#).unwrap();
        assert_eq!(entry.detail, "");
    }

    /// Every member has a row, which is what makes [`spec_for`]'s panic unreachable.
    ///
    /// Python keeps the same assertion in `tests/contracts/test_unscored.py`. It is not
    /// ceremonial: a new member is added beside a `return None` in `measure.py`, a long way from
    /// this table, and the failure without this test is a `KeyError` inside a golfer's tip.
    ///
    /// **Driven off the wire-name table rather than off `UNSCORED_REASONS` itself**, because a walk
    /// over the table can only prove that the rows it holds are the rows it holds. That table names
    /// every enum member, so this is the pass that a *member* with no row fails — the direction the
    /// dict version got for free and a slice does not.
    #[test]
    fn every_reason_has_a_row() {
        for reason in every_reason() {
            let spec = spec_for(reason);
            assert!(!spec.remedy.is_empty(), "{reason:?} has an empty remedy");
        }
        assert_eq!(
            UNSCORED_REASONS.len(),
            every_reason().len(),
            "the prose table and the enum disagree about how many reasons there are"
        );
        // And no reason has two rows, which a slice permits where a dict did not: the second would
        // be dead, so the prose a golfer reads would depend on which row was written first.
        for (index, (reason, _)) in UNSCORED_REASONS.iter().enumerate() {
            assert!(
                !UNSCORED_REASONS[index + 1..]
                    .iter()
                    .any(|(other, _)| other == reason),
                "{reason:?} has two rows"
            );
        }
    }

    /// Every remedy is a sentence a golfer reads, so the shape of one is pinned rather than its
    /// wording: capitalised, full-stopped, and never empty. The wording itself is gated by the
    /// engine vectors, where it lands inside a compared `Tip.text`.
    #[test]
    fn a_summary_is_a_clause_and_a_remedy_is_a_sentence() {
        for (reason, spec) in UNSCORED_REASONS {
            assert!(!spec.summary.is_empty(), "{reason:?}");
            assert!(
                !spec.summary.ends_with('.'),
                "{reason:?}: a summary is embedded in someone else's sentence"
            );
            assert!(
                spec.summary
                    .starts_with(|c: char| c.is_lowercase() || !c.is_alphabetic()),
                "{reason:?}: a summary carries no leading capital"
            );
            assert!(spec.remedy.ends_with('.'), "{reason:?}");
            assert!(
                spec.remedy.starts_with(char::is_uppercase),
                "{reason:?}: a remedy is a sentence of its own"
            );
        }
    }

    /// `refilming_helps` is the one bit a consumer branches on, and it splits the table the way
    /// the MCP server's own instructions promise: a capture problem — the clip or the photo — is
    /// one worth taking again, and everything else is not the capture's fault.
    ///
    /// `Misread` is the one photo-side reason on the true side; `PrintedBlank` is on the false side
    /// with `NoBand`, because a blank the screen printed reads blank in every photo of it.
    #[test]
    fn refilming_helps_exactly_where_the_clip_is_the_problem() {
        let helps: Vec<UnscoredReason> = UNSCORED_REASONS
            .iter()
            .filter(|(_, spec)| spec.refilming_helps)
            .map(|(reason, _)| *reason)
            .collect();
        assert_eq!(
            helps,
            vec![
                UnscoredReason::PhaseNotSegmented,
                UnscoredReason::BoundaryEstimated,
                UnscoredReason::TimingDegenerate,
                UnscoredReason::LandmarksUnconfident,
                UnscoredReason::TooFewFrames,
                UnscoredReason::ScaleUnavailable,
                UnscoredReason::Misread,
            ]
        );
        // No inference reason is a capture problem: nothing about a screen the OCR could not read
        // is fixed by filming the swing again.
        for reason in INFERENCE_REASONS {
            assert!(!spec_for(*reason).refilming_helps, "{reason:?}");
        }
    }

    /// The inference family is a subset of the table and disjoint from the capture half, which is
    /// what keeps `rules.py`'s exclusion an exclusion rather than a coincidence.
    #[test]
    fn the_inference_family_is_five_rows_of_this_table() {
        assert_eq!(INFERENCE_REASONS.len(), 5);
        for reason in INFERENCE_REASONS {
            assert!(is_inference_reason(*reason));
            spec_for(*reason);
        }
        assert!(!is_inference_reason(UnscoredReason::NoHandedness));
        assert!(!is_inference_reason(UnscoredReason::Unrecorded));
    }

    /// The table is reachable from a payload, which is the only way `rules.py` gets at it.
    #[test]
    fn an_entry_reaches_its_own_prose() {
        let entry: UnscoredCheckpoint =
            serde_json::from_str(r#"{"name":"head_stays_back","reason":"no_handedness"}"#).unwrap();
        assert!(!entry.spec().refilming_helps);
        assert!(entry.spec().remedy.contains("Pick a golfer"));
    }
}
