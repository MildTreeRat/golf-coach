//! One golfer's swings across every session — the shape a personal baseline is cut from.
//! `contracts/career.py`. [M36 P6]
//!
//! Everything else in this repo judges a swing against a *tour population*. Career mode judges it
//! against the golfer's own history, and `storage::corpus::read_corpus` (M36 P10) is what assembles
//! that history into a [`CareerCorpus`]. These shapes live here rather than beside the reader
//! because `storage` produces them and `analysis` consumes them, and the two never depend on each
//! other (ADR-008).
//!
//! **The honest `n` is the product, not the swing list.** The same three files re-uploaded three
//! times are one swing, and a reader that counts directories hands the baseline one swing's numbers
//! three times — which does not merely inflate the count, it *collapses the variance*, and variance
//! is the whole reason career mode is worth building.
//!
//! **Two dedupe keys, because a measurement's `n` depends on the artifact it was *derived from*,
//! not the instrument that named it.** A pose metric's sample count is the number of distinct
//! face-on clips; a launch-monitor metric's, and ADR-027's simulated flight's, is the number of
//! distinct shot photos. They are free to diverge in one direction: the bundle store's "newest swing
//! missing this role" rule can attach one shot photo to two genuinely different swings, which is two
//! pose samples and one launch-monitor sample. The opposite direction is not extra data — one clip
//! carrying two different photos is a misattachment, reported as `conflicting_shots` for repair
//! rather than counted. [`CorpusSwing::artifact_key`] is the single definition of the rule.
//!
//! **Nothing is excluded silently**, as with `SwingResult.unscored`: a swing dropped from the corpus
//! is named in [`CareerCorpus::excluded`] with its reason.
//!
//! # What ages on [`CAREER_VERSION`]
//!
//! This module also holds the version key for the two vector families the many-shot layer is gated
//! by, `spec/vectors/storage/` and `spec/vectors/career/`. It is here because the corpus is the one
//! shape both families carry: the storage family's corpus cases end in one, and every career case
//! starts from one.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::club::ClubId;
use crate::mishit::{MishitVerdict, MISHIT_EXCLUDED_METRICS};
use crate::swing::Measurement;
use crate::{each, ContractError, Timestamp, Validate};

/// The generation of the many-shot layer's rules, stamped as `career_version` at the top of every
/// vector under `spec/vectors/storage/` and `spec/vectors/career/`. **Bump it whenever either family
/// would record something different** — a store that writes another file, a corpus that pools or
/// excludes another swing, an aggregate that moves, a report that prints another sentence — and
/// re-record both families through `golf-core rerecord` with a `career-v<N>.json` declaration, as
/// [`crate::shot::SCREEN_PARSER_VERSION`] picks the screen family (the M36 plan's decision 8).
///
/// One key for both families, because the career family's real case starts from the storage
/// family's real corpus (P3 asserts the two are equal when it records), so a change to what the
/// reader produces moves both, and two keys would let one be re-recorded without the other.
///
/// **It is not [`crate::swing::ANALYSIS_VERSION`], and an engine bump never moves it.**
/// `read_corpus` takes the engine generations as a parameter (the M36 plan's call 7), and each
/// vector records the pair it was read under, so a new engine changes no recorded answer.
///
/// 0 (M36 P2, P3): frozen Python's rules, recorded from it once by `conformance.py regenerate
///   --storage-once` and `--career-once`. Every vector carries it, and the faithful Rust port is
///   held to them. Not an entry so much as the baseline the entries are changes to.
///
/// 1 (M36 P14): **the corpus's `OUTDATED` means *not comparable*.** `read_corpus` excludes a swing
///   stored below `versions.comparable_from` (the verbs pass
///   [`crate::swing::COMPARABLE_FROM`]), where frozen Python's rule, version 0's, excludes one below
///   `versions.installed`, and the exclusion's sentence names the line it fell under. Every
///   Python-recorded case was read under `{16, 16}`, where the two rules exclude the same swings, so
///   `spec/declarations/career-v1.json` moves those sentences, their adopted copies and this number,
///   and nothing else; no aggregate moved. The cases where the rules part are hand-worked, under
///   `spec/vectors/storage/hand/`. The scripts' report text was carried at 0's until a renderer ran
///   it. M36 P15 rendered `career_corpus`, whose `--verbose` form prints each exclusion's sentence,
///   so the same declaration, run a second time, moved `expected.reports.career_corpus_verbose` in
///   the four adopted copies: this rule reaching the report, not a new rule, so not a new number.
///   M36 P16 rendered `club_profile` and `flag_mishit`, and no report moved: neither prints an
///   exclusion's sentence.
pub const CAREER_VERSION: i64 = 1;

/// `Measurement.source` for anything read off the pose stream: it keys on the face-on clip.
pub const POSE_SOURCE_PREFIX: &str = "pose:";

/// `Measurement.source` for anything printed on the launch-monitor screen: it keys on the photo.
pub const LAUNCH_MONITOR_SOURCE_PREFIX: &str = "launch_monitor:";

/// ADR-027's simulated flight, the one provenance that is not a reading of anything, so it is
/// versioned with the coefficient artifact it evaluates (`model:flight_v1`). It keys on the **shot
/// photo**, because the integrator flies that tile's launch conditions — and so it inherits the
/// photo's flagged-parse refusal too. Under the `swing:` fallback, a flight simulated off a parse
/// flagged for review counted as a sample while the carry printed beside it on the same tile did not.
pub const MODEL_SOURCE_PREFIX: &str = "model:";

/// The second camera's pose provenance [M17 P5], and the one `pose:` source that keys on
/// **nothing**.
///
/// [`CorpusSwing`] carries `face_on_sha256` and no hash for the down-the-line clip, so a `_dtl` row
/// keyed `pose:{face_on_sha256}` would assert that two different rear clips over one face-on clip
/// are one reading of it, and the pooled value would be whichever was read first. A full source
/// rather than a prefix, tested by equality before the prefix that would otherwise claim it. The
/// fiducial work, or anything else that gives the second clip an identity, is what reverses this
/// (ADR-029's 2026-09-09b addendum §5).
///
/// **The one definition**: `analysis::engine` stamps the `_dtl` rows with this constant (the M36
/// plan's finding 7), as Python's engine imports it from here, so the string that writes a row and
/// the string that special-cases it cannot drift apart.
pub const POSE_DTL_SOURCE: &str = "pose:down_the_line";

/// Every prefix [`CorpusSwing::artifact_key`] recognises, in one place because two callers test
/// membership: the dispatch itself, and [`count_metrics`]' unknown-source report. The two drifting
/// apart is worse than either being wrong alone — a source would take a real artifact key *and* be
/// reported as unrecognised.
///
/// **`population:golfdb` is missing on purpose, and adding it is a decision rather than a
/// tidy-up.** Registering it moves no count (the corpus groups *by* `face_on_sha256`, so the
/// `swing:` fallback partitions it exactly as `pose:` would) and every test stays green, which is
/// the whole hazard. ADR-022's fourth addendum defers two questions it would answer by accident:
/// whether a distance from a tour population is a personal quantity at all, and — if it is — that
/// the two down-the-line placements are read off a clip this shape carries no hash for, so `pose:`
/// would over-count them exactly where the shot photo over-counted before it was split out. Until
/// then the prefix staying in [`CareerCorpus::unknown_sources`] is the deferral being visible.
pub const KNOWN_SOURCE_PREFIXES: [&str; 3] = [
    POSE_SOURCE_PREFIX,
    LAUNCH_MONITOR_SOURCE_PREFIX,
    MODEL_SOURCE_PREFIX,
];

/// `source.startswith(KNOWN_SOURCE_PREFIXES)`.
pub fn is_known_source(source: &str) -> bool {
    KNOWN_SOURCE_PREFIXES
        .iter()
        .any(|prefix| source.starts_with(prefix))
}

/// Why a swing on disk is not in the corpus. Every one of these is reported, never inferred.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExclusionReason {
    /// Nobody had selected a golfer when it arrived. Repairable — the upload page has a per-swing
    /// link — so it is listed, not just counted.
    Unattributed,
    /// No face-on clip, the view every checkpoint is measured from, so this swing can never carry a
    /// pose measurement however long it sits there.
    NoFaceOn,
    /// The same face-on bytes already appear in an earlier swing. The re-upload, not the swing.
    Duplicate,
    /// Uploaded but never analyzed — no `analysis.json`. Fixable by running the pipeline.
    NotAnalyzed,
    /// Analyzed, but a clip has been re-uploaded since; the stored numbers describe bytes that are
    /// no longer on disk. Fixable by re-running the pipeline.
    Stale,
    /// Analyzed by an engine generation whose numbers today's does not agree with. The inputs are
    /// unchanged — this is the *other* axis of staleness. Kept out of the counts because a baseline
    /// reads spread: pooling two engine generations that disagree manufactures variance out of a
    /// code change, the way counting a re-uploaded clip twice destroys it.
    ///
    /// Rust's reader asks "older than [`crate::swing::COMPARABLE_FROM`]" (M36 P14, the M36 plan's
    /// decision 1). Frozen Python's asks `analysis_version < ANALYSIS_VERSION`, and so would exclude a
    /// swing an engine bump only *added* a measurement to; it keeps that rule until M40 deletes it.
    Outdated,
}

impl ExclusionReason {
    /// Every reason, in declaration order.
    pub const ALL: [ExclusionReason; 6] = [
        ExclusionReason::Unattributed,
        ExclusionReason::NoFaceOn,
        ExclusionReason::Duplicate,
        ExclusionReason::NotAnalyzed,
        ExclusionReason::Stale,
        ExclusionReason::Outdated,
    ];

    /// The wire name, which the reports print.
    pub const fn as_str(self) -> &'static str {
        match self {
            ExclusionReason::Unattributed => "unattributed",
            ExclusionReason::NoFaceOn => "no_face_on",
            ExclusionReason::Duplicate => "duplicate",
            ExclusionReason::NotAnalyzed => "not_analyzed",
            ExclusionReason::Stale => "stale",
            ExclusionReason::Outdated => "outdated",
        }
    }
}

impl Validate for ExclusionReason {
    fn validate(&self) -> Result<(), ContractError> {
        Ok(())
    }
}

/// One swing that did not make it into the corpus, and the reason in both forms.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Self")]
pub struct ExcludedSwing {
    pub session_id: String,
    pub swing_id: String,
    pub reason: ExclusionReason,
    /// The sentence a human needs — which swing absorbed a duplicate, and so on.
    #[serde(default)]
    pub detail: String,
}

crate::validated!(ExcludedSwing);

impl Validate for ExcludedSwing {
    fn validate(&self) -> Result<(), ContractError> {
        Ok(())
    }
}

impl ExcludedSwing {
    /// `session/swing`. Python's property is `ref`, a Rust keyword, so it is spelled as
    /// [`crate::baseline::MetricSample::swing_ref`] spells the same string.
    pub fn swing_ref(&self) -> String {
        format!("{}/{}", self.session_id, self.swing_id)
    }
}

/// One **distinct** swing of one golfer's, with the measurements a baseline reads.
///
/// Distinct is doing real work: this is one physical swing, not one swing directory. Any re-uploads
/// of the same face-on clip were folded in here and are named in `duplicates`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Self")]
pub struct CorpusSwing {
    pub player_id: String,
    pub session_id: String,
    pub swing_id: String,

    /// The *earliest* arrival across this swing and its duplicates. A re-upload's timestamp is an
    /// upload date, not a capture date, so keying a trend on it would date a swing to the day
    /// someone re-tested the upload path.
    pub captured_at: Timestamp,

    /// Swing identity. Two swings sharing it are one swing.
    pub face_on_sha256: String,
    /// The shot photo's hash — the identity launch-monitor measurements dedupe on, independently of
    /// the clip. `None` when no shot screen was uploaded.
    #[serde(default)]
    pub shot_sha256: Option<String>,

    /// Which club hit it, read off the **survivor's** manifest — the earliest arrival, so the tag
    /// stamped closest to the swing. A duplicate's club is never consulted: a re-upload stamps the
    /// cursor as it stood when that file arrived. `None` means the swing predates M9 P6. **An
    /// untagged swing is not excluded** — it is a good contributor to every pose metric and every
    /// whole-bag number, and absent only from per-club views ([`CareerCorpus::untagged_swings`]).
    #[serde(default)]
    pub club: Option<ClubId>,

    /// Carried whole rather than flattened to name -> value, because `source` is what decides which
    /// artifact a metric's sample count is keyed on, and `unit`/`detail` are what make a stored
    /// number re-derivable when a band is eventually cut from it.
    #[serde(default)]
    pub measurements: Vec<Measurement>,

    /// `session/swing` refs whose face-on bytes are identical to this one's.
    #[serde(default)]
    pub duplicates: Vec<String>,

    /// Shot-photo hashes carried by this swing's duplicates that differ from `shot_sha256`. A data
    /// error, not a second reading — one swing has one ball flight — so never counted as a sample,
    /// and surfaced so the misattached photo can be repaired.
    #[serde(default)]
    pub conflicting_shots: Vec<String>,

    /// An `analysis.json` was found and read.
    #[serde(default)]
    pub analyzed: bool,
    /// Analyzed, but an input has been re-uploaded since. Excluded from the counts.
    #[serde(default)]
    pub stale: bool,
    /// Which generation of the engine wrote this swing's `analysis.json`. 0 for artifacts written
    /// before the stamp existed.
    #[serde(default)]
    pub analysis_version: i64,
    /// Analyzed by an engine whose numbers do not pool with today's. The second axis of staleness:
    /// `stale` means the *inputs* moved, this means the *code* did. Excluded from the counts for the
    /// same reason.
    #[serde(default)]
    pub outdated: bool,
    /// The attached shot's OCR parse was flagged (ADR-014). Its numbers stay on the swing but
    /// contribute to no launch-monitor sample count.
    #[serde(default)]
    pub shot_needs_review: bool,
    /// The attached shot carried far below this club's own median ([`crate::mishit`], ADR-028).
    /// Set by the corpus reader after the swings are grouped by club, so it is always false on a
    /// swing with no club, no shot photo or a flagged parse. A verdict in `manual_mishit` overrides
    /// it; read [`CorpusSwing::is_mishit`], never this.
    #[serde(default)]
    pub auto_mishit: bool,
    /// The golfer's own verdict, lifted from the swing's manifest. Wins over `auto_mishit` in either
    /// direction; `None` defers to it. The only mishit signal a human ever sets.
    #[serde(default)]
    pub manual_mishit: Option<MishitVerdict>,
    #[serde(default)]
    pub missing_roles: Vec<String>,
}

crate::validated!(CorpusSwing);

impl Validate for CorpusSwing {
    fn validate(&self) -> Result<(), ContractError> {
        each("CorpusSwing.measurements", &self.measurements)
    }
}

impl CorpusSwing {
    /// `session/swing` (Python's `ref`; see [`ExcludedSwing::swing_ref`]).
    pub fn swing_ref(&self) -> String {
        format!("{}/{}", self.session_id, self.swing_id)
    }

    /// May this swing contribute a sample? Analyzed, still describing its own inputs, and produced
    /// by an engine whose numbers pool with today's — one question asked of three things: the
    /// artifact, the bytes under it, and the code that joined them.
    pub fn counts_toward_metrics(&self) -> bool {
        self.analyzed && !self.stale && !self.outdated
    }

    /// Should this shot's carry and total distance be held out of the club's averages?
    ///
    /// Three states collapse to a bool: the golfer said yes, the golfer said no, or nobody said
    /// anything and the automatic rule stands. The override is absolute both ways — a `Cleared`
    /// shot counts however far below the floor it fell, because the golfer is the one who knows it
    /// was a stung punch shot and not a top.
    pub fn is_mishit(&self) -> bool {
        match self.manual_mishit {
            Some(MishitVerdict::Confirmed) => true,
            Some(MishitVerdict::Cleared) => false,
            None => self.auto_mishit,
        }
    }

    /// Which artifact this measurement is a reading *of*, or `None` if it contributes nothing.
    ///
    /// **The single definition of the dedupe rule**, called by both sides that need it: the corpus
    /// reader to count samples ([`count_metrics`]) and the baseline to pool the values behind those
    /// counts. Two definitions would let the printed `n` and the number of values averaged disagree,
    /// silently, and only in the cases the rule exists for.
    ///
    /// Keys are namespaced (`pose:` / `shot:` / `swing:`) so a clip hash can never collide with a
    /// photo hash. The same key from two swings asserts they are one reading; `None` asserts there
    /// is no reading here at all. Three `None`s, each a different assertion:
    ///
    /// - [`POSE_DTL_SOURCE`]: a reading of a clip this shape cannot name. Tested first, or the
    ///   `pose:` prefix absorbs it into the face-on clip's count.
    /// - A launch-monitor or model source on a flagged parse or with no photo: a suspect sample is
    ///   no sample, the rule `mcp.query.get_session_summary` applies before averaging.
    /// - **A mishit's carry and total distance** (ADR-028): a 20-yard 7 iron is a real reading of
    ///   the wrong thing, and the average is for the distance the golfer *meant*. Deliberately
    ///   narrow: ball speed, launch and offline on that same photo key normally, as does every pose
    ///   metric.
    ///
    /// An unrecognised provenance falls back to swing identity: conservative, since it can only
    /// over-count relative to a real artifact key, and [`CareerCorpus::unknown_sources`] names it.
    pub fn artifact_key(&self, measurement: &Measurement) -> Option<String> {
        let source = measurement.source.as_str();
        if source == POSE_DTL_SOURCE {
            return None;
        }
        if source.starts_with(POSE_SOURCE_PREFIX) {
            return Some(format!("pose:{}", self.face_on_sha256));
        }
        if source.starts_with(LAUNCH_MONITOR_SOURCE_PREFIX)
            || source.starts_with(MODEL_SOURCE_PREFIX)
        {
            if self.shot_needs_review {
                return None;
            }
            let shot = self.shot_sha256.as_deref()?;
            if MISHIT_EXCLUDED_METRICS.contains(&measurement.name.as_str()) && self.is_mishit() {
                return None;
            }
            return Some(format!("shot:{shot}"));
        }
        Some(format!("swing:{}", self.swing_ref()))
    }
}

/// `narrowed_to`'s keyword arguments, each `None` for "do not narrow on this".
///
/// A struct rather than three positional `Option`s, because the call sites each name one or two of
/// them (`Narrowing { club: Some(club), ..Narrowing::default() }`) the way Python's callers pass
/// keywords, and three adjacent `Option`s are a transposition nobody would see. It reads the storage
/// family's `input.narrowings` entries directly, which are these three keys.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Narrowing {
    /// Keep swings captured at or after this instant.
    #[serde(default)]
    pub since: Option<Timestamp>,
    /// Keep swings from these sessions. `Some(vec![])` keeps nothing, as an empty collection does
    /// in Python; it is not "no filter".
    #[serde(default)]
    pub sessions: Option<Vec<String>>,
    /// Keep swings tagged with this club. An untagged swing matches no club.
    #[serde(default)]
    pub club: Option<ClubId>,
}

/// Every distinct swing of one golfer's, and the honest count behind each metric.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Self")]
pub struct CareerCorpus {
    pub player_id: String,

    /// Distinct swings, oldest `captured_at` first.
    #[serde(default)]
    pub swings: Vec<CorpusSwing>,

    #[serde(default)]
    pub sessions_scanned: i64,
    /// Swing directories read across every session, for every golfer — the number a naive count
    /// would have reported. The gap between this and `distinct_swings` is the point.
    #[serde(default)]
    pub swing_dirs_seen: i64,

    /// Metric name -> honest sample size: distinct contributing **artifacts**, not swings, so three
    /// re-uploads of one swing contribute 1. The field the per-metric minimum-N guard reads, and it
    /// should read nothing else. A `BTreeMap`, which is the order Python's is built in
    /// ([`count_metrics`] sorts by name, and a `str` sorts by code point as UTF-8 bytes do).
    #[serde(default)]
    pub metric_counts: BTreeMap<String, i64>,

    /// Distinct swings whose analysis does not pool with today's engine. Re-analyze them and they
    /// rejoin the counts; every one is also itemised in `excluded`.
    #[serde(default)]
    pub outdated_swings: i64,

    /// Distinct swings that count toward the metrics and still carry no measurement at all —
    /// measurement failed on a current engine, rather than the engine being old, which is a
    /// different problem with a different fix.
    #[serde(default)]
    pub analyzed_without_measurements: i64,

    /// Swings on disk naming no golfer. Also itemised in `excluded`.
    #[serde(default)]
    pub unattributed_swings: i64,
    /// Swings belonging to someone else: counted for context, not itemised, because they are
    /// correctly absent where an unattributed swing may be repairable into this corpus.
    #[serde(default)]
    pub other_golfers: i64,

    /// `Measurement.source` values matching no prefix in [`KNOWN_SOURCE_PREFIXES`], sorted. **Not a
    /// to-do list**: `population:golfdb` is in it by decision (ADR-022's fourth addendum), so a
    /// reader must ask what each entry is waiting on rather than register it.
    #[serde(default)]
    pub unknown_sources: Vec<String>,

    /// Every swing on disk that contributes **no sample to any metric count**, with its reason.
    /// Not the complement of `swings`: a distinct swing that is merely unanalyzed, stale or outdated
    /// appears in both — a real swing of this golfer's that carries no usable numbers yet.
    #[serde(default)]
    pub excluded: Vec<ExcludedSwing>,
}

crate::validated!(CareerCorpus);

impl Validate for CareerCorpus {
    fn validate(&self) -> Result<(), ContractError> {
        each("CareerCorpus.swings", &self.swings)?;
        each("CareerCorpus.excluded", &self.excluded)
    }
}

impl CareerCorpus {
    /// This corpus narrowed to a window, named sessions or one club, **counts recomputed**.
    ///
    /// It lives on the corpus rather than in each caller because of `metric_counts`: a filtered
    /// `swings` list beside the unfiltered counts is a corpus whose printed `n` describes a different
    /// set of swings than its values do. The guard then falls out for free — narrow, build a
    /// baseline, and a window holding three swings refuses everything a corpus holding three swings
    /// refuses. A per-club carry needs five distinct *7-iron* shots, not five shots, with nothing new
    /// having learned the guard (ADR-024).
    ///
    /// **The scan counters are carried unchanged and still describe the whole read**
    /// (`swing_dirs_seen`, `sessions_scanned`, `unattributed_swings`, `other_golfers`, `excluded`):
    /// they are facts about what was on disk, and nothing downstream of a narrowed corpus reads them.
    ///
    /// **A swing naming no club matches no club, so a club narrowing drops it** — and that asymmetry
    /// is the point. The reader does not exclude an untagged swing, because the club was never an
    /// input to measuring head sway; a per-club view is the one place the tag is load-bearing.
    /// `untagged_swings` needs no recomputation, being derived.
    ///
    /// On the contract rather than on the reader because `analysis`'s per-club builder needs it and
    /// may depend on `contracts` alone (ADR-008); [`CorpusSwing::artifact_key`] is the precedent.
    pub fn narrowed_to(&self, narrowing: &Narrowing) -> CareerCorpus {
        let kept: Vec<CorpusSwing> = self
            .swings
            .iter()
            .filter(|swing| {
                narrowing
                    .since
                    .is_none_or(|since| swing.captured_at >= since)
                    && narrowing
                        .sessions
                        .as_ref()
                        .is_none_or(|sessions| sessions.contains(&swing.session_id))
                    && narrowing.club.is_none_or(|club| swing.club == Some(club))
            })
            .cloned()
            .collect();
        let (metric_counts, unknown_sources) = count_metrics(&kept);
        let outdated_swings = count(kept.iter().filter(|swing| swing.outdated));
        let analyzed_without_measurements = count(
            kept.iter()
                .filter(|swing| swing.counts_toward_metrics() && swing.measurements.is_empty()),
        );

        CareerCorpus {
            swings: kept,
            metric_counts,
            unknown_sources,
            outdated_swings,
            analyzed_without_measurements,
            ..self.clone()
        }
    }

    /// `len(swings)`.
    pub fn distinct_swings(&self) -> usize {
        self.swings.len()
    }

    /// Distinct shot photos across the corpus — the ceiling on any launch-monitor metric.
    pub fn distinct_shots(&self) -> usize {
        self.swings
            .iter()
            .filter_map(|swing| swing.shot_sha256.as_deref())
            .collect::<BTreeSet<_>>()
            .len()
    }

    /// Distinct sessions these swings came from — the occasions, not the swings.
    ///
    /// **Not the number any TREND claim gates on.** That gate reads `MetricBaseline.n_sessions`,
    /// which counts only sessions that contributed a *sample to that metric*, so a session whose
    /// swings are unanalyzed raises this and not that. Both are right; they answer different
    /// questions.
    pub fn distinct_sessions(&self) -> usize {
        self.swings
            .iter()
            .map(|swing| swing.session_id.as_str())
            .collect::<BTreeSet<_>>()
            .len()
    }

    /// Distinct swings naming no club — everything per-club work cannot see.
    ///
    /// **Derived, where `unattributed_swings` is tallied.** A manifest naming no golfer never
    /// becomes a [`CorpusSwing`], so that counter has to be counted during the scan; a manifest
    /// naming no club is a real swing sitting in `swings`, so this reads it back off them, and a
    /// narrowed corpus has no field to forget to recompute. **Deliberately not an
    /// [`ExclusionReason`]** (ADR-024): it is a perfectly good contributor to every pose metric.
    pub fn untagged_swings(&self) -> usize {
        self.swings
            .iter()
            .filter(|swing| swing.club.is_none())
            .count()
    }

    pub fn duplicates_collapsed(&self) -> usize {
        self.swings.iter().map(|swing| swing.duplicates.len()).sum()
    }

    /// Swings whose re-uploads disagree about which shot photo belongs to them.
    pub fn shot_conflicts(&self) -> usize {
        self.swings
            .iter()
            .filter(|swing| !swing.conflicting_shots.is_empty())
            .count()
    }

    /// Distinct shot photos held out of the carry and total-distance averages as mishits.
    ///
    /// Counts photos, not swings, on the same footing as [`CareerCorpus::distinct_shots`] — a mishit
    /// is a property of the ball flight, and the ball flight dedupes on the photo (ADR-028 §4).
    pub fn mishit_shots(&self) -> usize {
        self.swings
            .iter()
            .filter(|swing| suppressed_as_mishit(swing))
            .filter_map(|swing| swing.shot_sha256.as_deref())
            .collect::<BTreeSet<_>>()
            .len()
    }

    /// `session/swing` of every mishit, sorted — so the exclusion is named, not just counted.
    pub fn mishit_refs(&self) -> Vec<String> {
        let mut refs: Vec<String> = self
            .swings
            .iter()
            .filter(|swing| suppressed_as_mishit(swing))
            .map(CorpusSwing::swing_ref)
            .collect();
        refs.sort();
        refs
    }

    /// Of [`CareerCorpus::mishit_shots`], the ones the automatic rule flagged and no golfer has
    /// ruled on — the "waiting for you" number.
    pub fn mishit_shots_unconfirmed(&self) -> usize {
        self.swings
            .iter()
            .filter(|swing| suppressed_as_mishit(swing) && swing.manual_mishit.is_none())
            .filter_map(|swing| swing.shot_sha256.as_deref())
            .collect::<BTreeSet<_>>()
            .len()
    }
}

/// A mishit whose shot would otherwise have been a launch-monitor sample.
///
/// [`CorpusSwing::is_mishit`] alone is not it: a swing with no photo, a flagged parse or an engine
/// too old to count contributes nothing to a distance average with or without the flag, so counting
/// it would report an exclusion that moved no number. A method on `CareerCorpus` in Python that
/// reads nothing of the corpus, so a free function here.
fn suppressed_as_mishit(swing: &CorpusSwing) -> bool {
    swing.is_mishit()
        && swing.counts_toward_metrics()
        && swing.shot_sha256.is_some()
        && !swing.shot_needs_review
}

/// `sum(1 for … if …)`, as the `i64` the contract field holds.
fn count<T>(items: impl Iterator<Item = T>) -> i64 {
    i64::try_from(items.count()).expect("a count fits an i64")
}

/// Metric -> distinct contributing artifacts, and any `source` no known prefix claimed (sorted).
///
/// The keying is [`CorpusSwing::artifact_key`] and deliberately not repeated, so the printed `n` and
/// the number of values pooled under it cannot drift apart. What lives here is the unknown-source
/// report, which is about the reader's coverage rather than part of the rule, and it reads
/// [`KNOWN_SOURCE_PREFIXES`] so a provenance registered in the dispatch cannot go on being reported
/// as unrecognised. A free function rather than a method because the reader needs it on a bare list
/// before there is a corpus to call it on; [`CareerCorpus::narrowed_to`] is its second caller.
pub fn count_metrics(swings: &[CorpusSwing]) -> (BTreeMap<String, i64>, Vec<String>) {
    let mut artifacts: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut unknown: BTreeSet<String> = BTreeSet::new();

    for swing in swings.iter().filter(|swing| swing.counts_toward_metrics()) {
        for measurement in &swing.measurements {
            if !is_known_source(&measurement.source) {
                unknown.insert(measurement.source.clone());
            }
            if let Some(key) = swing.artifact_key(measurement) {
                artifacts
                    .entry(measurement.name.clone())
                    .or_default()
                    .insert(key);
            }
        }
    }

    let counts = artifacts
        .into_iter()
        .map(|(name, keys)| (name, count(keys.iter())))
        .collect();
    (counts, unknown.into_iter().collect())
}

#[cfg(test)]
mod tests {
    //! `tests/contracts/test_career.py`, case for case, and the rules around it the corpus vectors
    //! do not isolate. The vectors themselves are `tests/career.rs`.

    use super::*;

    fn now() -> Timestamp {
        "2026-09-01T12:00:00Z".parse().unwrap()
    }

    fn swing() -> CorpusSwing {
        CorpusSwing {
            player_id: "aaron".into(),
            session_id: "2026-09-01".into(),
            swing_id: "1".into(),
            captured_at: now(),
            face_on_sha256: "clip-a".into(),
            shot_sha256: Some("photo-1".into()),
            club: None,
            measurements: Vec::new(),
            duplicates: Vec::new(),
            conflicting_shots: Vec::new(),
            analyzed: false,
            stale: false,
            analysis_version: 0,
            outdated: false,
            shot_needs_review: false,
            auto_mishit: false,
            manual_mishit: None,
            missing_roles: Vec::new(),
        }
    }

    fn measurement(name: &str, source: &str) -> Measurement {
        Measurement {
            name: name.into(),
            value: 150.0,
            unit: "yards".into(),
            source: source.into(),
            detail: String::new(),
        }
    }

    fn lm(name: &str) -> Measurement {
        measurement(name, "launch_monitor:hd_golf")
    }

    /// One untagged swing in session `2026-09-01`, captured at noon UTC, read from one directory.
    fn one_swing_corpus() -> CareerCorpus {
        CareerCorpus {
            player_id: "aaron".into(),
            swings: vec![swing()],
            sessions_scanned: 1,
            swing_dirs_seen: 1,
            metric_counts: BTreeMap::new(),
            outdated_swings: 0,
            analyzed_without_measurements: 0,
            unattributed_swings: 0,
            other_golfers: 0,
            unknown_sources: Vec::new(),
            excluded: Vec::new(),
        }
    }

    #[test]
    fn the_verdict_wins_over_the_auto_flag_both_ways() {
        use MishitVerdict::{Cleared, Confirmed};
        let table = [
            (None, false, false),
            (None, true, true),
            (Some(Confirmed), false, true),
            (Some(Confirmed), true, true),
            (Some(Cleared), true, false),
            (Some(Cleared), false, false),
        ];
        for (manual, auto, expected) in table {
            let swing = CorpusSwing {
                manual_mishit: manual,
                auto_mishit: auto,
                ..swing()
            };
            assert_eq!(swing.is_mishit(), expected, "{manual:?} {auto}");
        }
    }

    #[test]
    fn a_mishit_withholds_carry_and_total_and_keeps_every_other_metric() {
        let swing = CorpusSwing {
            auto_mishit: true,
            ..swing()
        };
        assert_eq!(swing.artifact_key(&lm("carry_distance_yds")), None);
        assert_eq!(swing.artifact_key(&lm("total_distance_yds")), None);
        for still_counts in [
            "ball_speed_mph",
            "launch_angle_deg",
            "start_line_offline_yds",
        ] {
            assert_eq!(
                swing.artifact_key(&lm(still_counts)).as_deref(),
                Some("shot:photo-1")
            );
        }
        let pose = measurement("head_sway_norm", "pose:face_on");
        assert_eq!(swing.artifact_key(&pose).as_deref(), Some("pose:clip-a"));
    }

    #[test]
    fn a_cleared_shot_counts_its_carry_like_any_other() {
        let swing = CorpusSwing {
            auto_mishit: true,
            manual_mishit: Some(MishitVerdict::Cleared),
            ..swing()
        };
        assert_eq!(
            swing.artifact_key(&lm("carry_distance_yds")).as_deref(),
            Some("shot:photo-1")
        );
    }

    /// Both halves together, because the defect is the pair agreeing: under the prefix alone a
    /// `_dtl` row would take the face-on clip's key.
    #[test]
    fn the_second_camera_is_a_reading_of_a_clip_this_corpus_cannot_name() {
        let swing = swing();
        let dtl = measurement("pivot_hip_axis_drift_norm_dtl", POSE_DTL_SOURCE);
        let face_on = measurement("pivot_hip_axis_drift_norm", "pose:face_on");
        assert_eq!(swing.artifact_key(&dtl), None);
        assert_eq!(swing.artifact_key(&face_on).as_deref(), Some("pose:clip-a"));
    }

    /// It contributes no sample, and that is not the same as being unrecognised.
    #[test]
    fn the_second_camera_s_source_is_still_a_recognised_provenance() {
        assert!(is_known_source(POSE_DTL_SOURCE));
    }

    /// The parse, not the strike, is the reason: a cleared mishit on a bad parse still contributes
    /// nothing.
    #[test]
    fn a_flagged_parse_still_wins_over_a_mishit_that_would_have_kept_carry() {
        let swing = CorpusSwing {
            auto_mishit: true,
            manual_mishit: Some(MishitVerdict::Cleared),
            shot_needs_review: true,
            ..swing()
        };
        assert_eq!(swing.artifact_key(&lm("carry_distance_yds")), None);
        assert_eq!(swing.artifact_key(&lm("ball_speed_mph")), None);
    }

    /// The flight keys on the photo and takes the photo's refusals, and `population:` falls back to
    /// the swing and is reported — the deferral being visible.
    #[test]
    fn the_flight_keys_on_the_photo_and_a_placement_on_the_swing() {
        let flight = measurement("flight_apex_yds", "model:flight_v1");
        assert_eq!(
            swing().artifact_key(&flight).as_deref(),
            Some("shot:photo-1")
        );
        let no_photo = CorpusSwing {
            shot_sha256: None,
            ..swing()
        };
        assert_eq!(no_photo.artifact_key(&flight), None);
        assert_eq!(no_photo.artifact_key(&lm("ball_speed_mph")), None);

        let placement = measurement("trajectory_distance", "population:golfdb");
        assert_eq!(
            swing().artifact_key(&placement).as_deref(),
            Some("swing:2026-09-01/1")
        );
        assert!(!is_known_source("population:golfdb"));
        assert!(!is_known_source("Pose:face_on"));
    }

    #[test]
    fn counting_skips_what_cannot_count_and_names_the_unknown() {
        let counted = CorpusSwing {
            analyzed: true,
            measurements: vec![
                measurement("head_sway_norm", "pose:face_on"),
                measurement("trajectory_distance", "population:golfdb"),
                measurement("pivot_hip_axis_drift_norm_dtl", POSE_DTL_SOURCE),
            ],
            ..swing()
        };
        let reupload = CorpusSwing {
            swing_id: "2".into(),
            ..counted.clone()
        };
        let outdated = CorpusSwing {
            swing_id: "3".into(),
            face_on_sha256: "clip-b".into(),
            outdated: true,
            measurements: vec![measurement("tempo_ratio", "mystery:source")],
            ..counted.clone()
        };
        let (counts, unknown) = count_metrics(&[counted, reupload, outdated]);
        assert_eq!(
            counts.into_iter().collect::<Vec<_>>(),
            [
                ("head_sway_norm".to_string(), 1),
                ("trajectory_distance".to_string(), 2),
            ]
        );
        assert_eq!(unknown, ["population:golfdb"]);
    }

    #[test]
    fn an_empty_session_list_keeps_nothing_and_no_list_keeps_everything() {
        let corpus = one_swing_corpus();
        let none = corpus.narrowed_to(&Narrowing {
            sessions: Some(Vec::new()),
            ..Narrowing::default()
        });
        assert_eq!(none.distinct_swings(), 0);
        assert_eq!(none.swing_dirs_seen, 1, "the scan counters are carried");
        assert_eq!(corpus.narrowed_to(&Narrowing::default()), corpus);
        let club = corpus.narrowed_to(&Narrowing {
            club: Some(ClubId::SevenIron),
            ..Narrowing::default()
        });
        assert_eq!(
            club.distinct_swings(),
            0,
            "an untagged swing matches no club"
        );
    }

    /// The window is inclusive and compares instants, not spellings.
    #[test]
    fn since_is_inclusive_and_reads_the_instant() {
        let corpus = one_swing_corpus();
        for (since, kept) in [
            ("2026-09-01T12:00:00Z", 1),
            ("2026-09-01T17:30:00+05:30", 1),
            ("2026-09-01T12:00:00.000001Z", 0),
            ("2026-09-01T07:00:01-05:00", 0),
        ] {
            let narrowed = corpus.narrowed_to(&Narrowing {
                since: Some(since.parse().unwrap()),
                ..Narrowing::default()
            });
            assert_eq!(narrowed.distinct_swings(), kept, "{since}");
        }
    }

    #[test]
    fn exclusion_reasons_are_the_wire_names_in_declaration_order() {
        let names: Vec<String> = ExclusionReason::ALL
            .iter()
            .map(|reason| serde_json::to_string(reason).unwrap())
            .collect();
        assert_eq!(
            names,
            [
                "\"unattributed\"",
                "\"no_face_on\"",
                "\"duplicate\"",
                "\"not_analyzed\"",
                "\"stale\"",
                "\"outdated\"",
            ]
        );
        for reason in ExclusionReason::ALL {
            assert_eq!(
                serde_json::to_string(&reason).unwrap(),
                format!("\"{}\"", reason.as_str())
            );
        }
    }
}
