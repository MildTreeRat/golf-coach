//! Every swing and every shot one golfer has hit, across every session, counted honestly.
//! `storage/corpus.py`, which reads swings alone. [M36 P10; photo entries and drills, M35 P10]
//!
//! The reader behind [`CareerCorpus`]. Pure reads over artifacts that already exist —
//! `manifest.json` for identity; for a swing, `analysis.json` for measurements and
//! `analysis.state.json` for whether those measurements still describe the files on disk; for a
//! photo entry, `shot_analysis.json` for both. Nothing here runs pose, opens a video or re-hashes a
//! byte: [`crate::manifest::RoleFile::content_sha256`] was recorded as the upload streamed in, and it
//! is the same digest a re-read would produce.
//!
//! **What makes this more than a loop over directories.** On disk, the same three files were once
//! re-uploaded three times while the upload path was being tested, so four swing directories held
//! two swings. Counting directories would hand a personal baseline one swing's numbers three times,
//! which does not just inflate `n`, it collapses the variance the baseline exists to measure. So
//! swings are grouped by the face-on clip's hash, and shot metrics are counted on the shot photo's
//! hash, which are independent keys for the reason `contracts::career` sets out at length.
//!
//! **The exclusions are output, not control flow.** Every swing that contributes no sample is named
//! in [`CareerCorpus::excluded`] with a reason. A count that shrank silently is indistinguishable
//! from a reader with a bug, and this is the one place where too small an `n` and too large an `n`
//! are both wrong in the same confident voice.
//!
//! # Two kinds of entry, and the drill that counts nothing [M35 P10, `CAREER_VERSION` 2]
//!
//! Since ADR-034 §1 the shot is the unit, so a corpus entry is one of two things (the M35 plan's
//! call 6), and frozen Python knows only the first:
//!
//! - a **swing**: a manifest with a face-on clip, read exactly as before M35, from `analysis.json`,
//!   and grouped on the clip's hash;
//! - a **photo entry**: no face-on clip and a shot photo, read from `shot_analysis.json` alone
//!   ([`crate::state::load_shot_analysis`]), and grouped on the photo's hash. A down-the-line clip
//!   beside the photo does not make it a swing, because nothing is scored from that clip alone. Its
//!   `face_on_sha256` is `None`.
//!
//! A manifest with neither is `no_face_on`. **One artifact per kind**: a swing's stray
//! `shot_analysis.json` and a photo entry's stray `analysis.json` are never read, so no number
//! reaches the corpus by two routes (the hand case `mixed-session`).
//!
//! **The same photo saved twice, once beside a clip and once on its own, is one ball flight, and the
//! copy with video counts** (the M35 plan's decision 4). A surviving photo entry whose photo is a
//! surviving swing's `shot_sha256` is a `duplicate` of the earliest such swing by `arrival`, though
//! it arrived first. Every surviving swing is looked at, a drill's included, so a drill's photo cannot
//! re-enter the stats through a photo-only copy (`drills-untracked`). Such a copy is in no swing's
//! `duplicates`, which lists re-uploads of that swing's clip, so it is named in `excluded` alone.
//!
//! **A drill is `untracked`** ([`contracts::intent::PracticeMode::is_tracked`], ADR-034 §3). After
//! the dedupes, a survivor whose manifest records an untracked mode is excluded and is **not** in
//! `swings`, as a duplicate is not, so no count, aggregate, mishit median or report sees it. It is
//! decided off the manifest before any artifact is read, so a drill is named once, and never also
//! `not_analyzed`, `stale` or `outdated`: it is not work waiting on a pipeline run. Its own
//! duplicates stay duplicates of it. The goal is the survivor's manifest's, never
//! `ShotResult.intent`'s, as the club is: the manifest is the intent of record.
//!
//! # The engine generations are a parameter
//!
//! Frozen Python's reader reads `ANALYSIS_VERSION` for itself. This one takes [`EngineVersions`]
//! from its caller (the M36 plan's call 7), so the storage family can record which pair each case
//! was read under and an engine bump moves no recorded answer.
//!
//! **A swing is `OUTDATED` when its stored version is below `comparable_from`** — the oldest
//! generation whose numbers today's engine still agrees with — and not, as in frozen Python, below
//! `installed` (M36 P14, the M36 plan's decision 1, `CAREER_VERSION` 1). Python's rule excluded a
//! swing for an engine bump that only *added* a measurement, which an honest per-metric `n` already
//! accounts for, and asked of Rust's `ANALYSIS_VERSION` it excluded every swing on disk.
//! `installed` is read by nothing here now, and stays in the pair because the storage family
//! records both and the faithful vectors were read under `{16, 16}`, where the two rules agree.
//! The cases where they part are `spec/vectors/storage/hand/`. [`crate::state::is_outdated`] is not
//! called here and never will be: it asks *this build's* engine, which is the re-analysis verb's
//! question — is there a newer engine to run — and it stays faithful for that verb (M36 P8's
//! finding 5).
//!
//! # What is read where, and in which order
//!
//! Each step carries its Python reason below. The orders are part of the answer, because the
//! storage family records `swings` and `excluded` as lists:
//!
//! - **`excluded` is scan order, kind by kind.** Sessions by code point, swing directories in
//!   [`SwingBundleStore::get_session`]'s numeric order. Then: the unattributed, in scan order; the
//!   `no_face_on`, in scan order; each clip group in the order its first member was scanned, its
//!   duplicates in arrival order, then its survivor's `untracked`, or else its own `not_analyzed`,
//!   `stale` and `outdated`; and last each photo group the same way, its duplicates in arrival order,
//!   then its survivor's `duplicate` of a swing, or else its `untracked`, or else its three. **The
//!   photo groups come after every clip group** because the cross-kind dedupe needs every surviving
//!   swing first. One walk over both kinds in first-scan order would give another list, which M35
//!   P9's `excluded-order-across-kinds` pins.
//! - **A duplicate group's survivor is the earliest by `arrival`**, for either kind, a *string* sort
//!   on CPython's `isoformat()`, not on the instant (M36 P1's finding 5).
//! - **`swings` is by instant**, both kinds together, then session and swing id as strings, stable.
//! - **The artifacts are read from `manifest.session_id / manifest.swing_id`**, not from the
//!   directory the manifest was found in. The manifest is trusted over its directory here as the
//!   bundle store trusts it (the storage family's `manifest-names-another-dir`).
//!
//! # Where this reads less than Python
//!
//! An `analysis.json` holding what `json.loads` takes beyond JSON (`NaN`, an unpaired surrogate) is
//! no analysis here, and a measurement or a shot that leans on one of pydantic's lax coercions is
//! refused where Python reads it — the crate doc's list, and no vector reaches either. The shot is
//! validated through Rust's [`ShotData`], which is M32's wider shape; a shot frozen Python wrote is
//! read the same by both (the storage family's `shot-review` and the real corpus hold it).

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use contracts::career::{CareerCorpus, CorpusSwing, ExcludedSwing, ExclusionReason, Narrowing};
use contracts::club::ClubId;
use contracts::mishit::mishit_carry_floor;
use contracts::shot::ShotData;
use contracts::swing::Measurement;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::bundle_store::SwingBundleStore;
use crate::manifest::{Role, SwingManifest};
use crate::state::{
    is_older_than, load_analysis, load_shot_analysis, load_state, shot_analysis_matches,
    stored_analysis_version,
};

/// Which engine generations the reader judges a stored analysis against: the storage family's
/// `input.versions`, and the verbs' `{ANALYSIS_VERSION, COMPARABLE_FROM}`.
///
/// Two numbers because they answer two questions. `installed` is the engine this build runs;
/// `comparable_from` is the oldest generation whose stored numbers today's engine still agrees
/// with, and it is the one the corpus asks (the module doc). Frozen Python has only the first and
/// asks it of the corpus, which is what every Python-recorded case's `{16, 16}` cannot tell apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EngineVersions {
    pub installed: i64,
    pub comparable_from: i64,
}

const UNATTRIBUTED: &str =
    "nobody had selected a golfer when this arrived — repair it on the upload \
                            page or with scripts/backfill_golfer.py";
/// Neither kind of entry (the module doc). Before M35 P10 it said only that the face-on clip was
/// missing, which stopped being the reason once a photo alone makes an entry.
const NO_FACE_ON: &str = "neither a face-on clip nor a shot photo, and every checkpoint is \
                          measured from the one and every shot number read from the other";
const NOT_ANALYZED: &str = "no analysis.json — run scripts/analyze_bundle.py over it";
const STALE: &str =
    "a clip was re-uploaded after this was analyzed, so the stored numbers describe \
                     bytes that are no longer here — re-run the pipeline";
/// A photo entry's three repairs name its artifact and the Rust verb that writes it (M35 P11), where
/// a swing's name frozen Python's: a photo entry is Rust's alone, and no script of Python's reads or
/// writes `shot_analysis.json`.
const PHOTO_NOT_ANALYZED: &str = "no shot_analysis.json — run golf-core run-shot over its session";
const PHOTO_STALE: &str = "shot_analysis.json names another shot photo than this one, or none, so \
                           the stored numbers may describe a photo that is no longer here — re-run \
                           golf-core run-shot over its session";
const UNTRACKED: &str = "hit as a drill, which feeds no club or player stat and no mechanics \
                         baseline — clear its intent with golf-core set-intent if it should count";

/// The measurement [`flag_auto_mishits`] reads a carry from.
const CARRY: &str = "carry_distance_yds";

/// Assemble one golfer's distinct swings and the honest sample size behind each metric.
///
/// An unknown `player_id`, or a `sessions_dir` that does not exist, is an empty corpus rather than
/// an error — "this golfer has no swings yet" is a real answer and the first one every golfer has.
pub fn read_corpus(sessions_dir: &Path, player_id: &str, versions: EngineVersions) -> CareerCorpus {
    let store = SwingBundleStore::new(sessions_dir);
    let session_ids = store.list_session_ids();

    let mut excluded: Vec<ExcludedSwing> = Vec::new();
    let mut mine: Vec<SwingManifest> = Vec::new();
    let mut seen = 0usize;
    let mut unattributed = 0usize;
    let mut other_golfers = 0usize;

    for session_id in &session_ids {
        for manifest in store.get_session(session_id) {
            seen += 1;
            match manifest.player_id.as_deref() {
                None => {
                    unattributed += 1;
                    excluded.push(exclude(
                        &manifest,
                        ExclusionReason::Unattributed,
                        UNATTRIBUTED,
                    ));
                }
                Some(owner) if owner == player_id => mine.push(manifest),
                Some(_) => other_golfers += 1,
            }
        }
    }

    // The two kinds (the module doc), each grouped on its own identity in the order its group's first
    // member was scanned: a Python dict's insertion order, which `excluded` inherits.
    let mut clips = Groups::default();
    let mut photos = Groups::default();
    for manifest in mine {
        if let Some(face_on) = manifest.roles.get(&Role::FaceOn) {
            let sha256 = face_on.content_sha256.clone();
            clips.add(sha256, manifest);
        } else if let Some(photo) = manifest.roles.get(&Role::ShotScreen) {
            let sha256 = photo.content_sha256.clone();
            photos.add(sha256, manifest);
        } else {
            excluded.push(exclude(&manifest, ExclusionReason::NoFaceOn, NO_FACE_ON));
        }
    }

    let mut swings: Vec<CorpusSwing> = Vec::with_capacity(clips.0.len() + photos.0.len());
    // Each photo a surviving swing holds, against the earliest such swing by `arrival` and its ref:
    // what a photo entry carrying the same photo is a duplicate of. Every survivor's, a drill's too.
    let mut beside_video: HashMap<String, ((String, String, String), String)> = HashMap::new();
    for (sha256, members) in clips.0 {
        let (survivor, duplicates) = survivor_of(members);
        for duplicate in &duplicates {
            excluded.push(exclude(
                duplicate,
                ExclusionReason::Duplicate,
                &format!(
                    "face-on bytes {} are already in {} — the same swing uploaded again, not a \
                     second swing",
                    cut(&sha256),
                    swing_ref(&survivor),
                ),
            ));
        }
        if let Some(photo) = survivor.roles.get(&Role::ShotScreen) {
            let held = (arrival(&survivor), swing_ref(&survivor));
            beside_video
                .entry(photo.content_sha256.clone())
                .and_modify(|earliest| {
                    if held.0 < earliest.0 {
                        *earliest = held.clone();
                    }
                })
                .or_insert(held);
        }
        if !is_tracked(&survivor) {
            excluded.push(exclude(&survivor, ExclusionReason::Untracked, UNTRACKED));
            continue;
        }
        let mut swing = entry(&survivor, Some(&sha256), &duplicates);
        read_swing_artifacts(sessions_dir, &survivor, &mut swing, &mut excluded, versions);
        swing.conflicting_shots = conflicting_shots(swing.shot_sha256.as_deref(), &duplicates);
        swings.push(swing);
    }

    for (sha256, members) in photos.0 {
        let (survivor, duplicates) = survivor_of(members);
        for duplicate in &duplicates {
            excluded.push(exclude(
                duplicate,
                ExclusionReason::Duplicate,
                &format!(
                    "shot photo {} is already in {} — the same shot uploaded again, not a second \
                     shot",
                    cut(&sha256),
                    swing_ref(&survivor),
                ),
            ));
        }
        // Before the intent is asked, so a drill's photo saved on its own is named as the copy it
        // is, and a photo-only copy cannot carry a drill's ball flight back into the stats.
        if let Some((_, kept)) = beside_video.get(&sha256) {
            excluded.push(exclude(
                &survivor,
                ExclusionReason::Duplicate,
                &format!(
                    "shot photo {} is already in {kept}, beside that swing's video — one ball \
                     flight, so the copy with the video is the one kept",
                    cut(&sha256),
                ),
            ));
            continue;
        }
        if !is_tracked(&survivor) {
            excluded.push(exclude(&survivor, ExclusionReason::Untracked, UNTRACKED));
            continue;
        }
        // A photo group shares its photo by definition, so `conflicting_shots` stays empty.
        let mut photo_entry = entry(&survivor, None, &duplicates);
        read_photo_artifacts(
            sessions_dir,
            &survivor,
            &mut photo_entry,
            &mut excluded,
            versions,
        );
        swings.push(photo_entry);
    }

    // Stable, as `list.sort` is, so two manifests naming one swing from two directories keep their
    // scan order.
    swings.sort_by(|a, b| {
        a.captured_at
            .cmp(&b.captured_at)
            .then_with(|| a.session_id.cmp(&b.session_id))
            .then_with(|| a.swing_id.cmp(&b.swing_id))
    });

    // Group the sorted swings by club and stamp `auto_mishit` on the gross tops — **before** the
    // counts, so a mishit's carry is already withheld when the sample count is taken and the printed
    // `n` cannot disagree with the pooled one (`CorpusSwing::artifact_key`, ADR-028).
    flag_auto_mishits(&mut swings);

    let read = CareerCorpus {
        player_id: player_id.to_string(),
        swings,
        sessions_scanned: as_count(session_ids.len()),
        swing_dirs_seen: as_count(seen),
        metric_counts: BTreeMap::new(),
        outdated_swings: 0,
        analyzed_without_measurements: 0,
        unattributed_swings: as_count(unattributed),
        other_golfers: as_count(other_golfers),
        unknown_sources: Vec::new(),
        excluded,
    };
    // The four counts derived from `swings` — `metric_counts`, `unknown_sources`, `outdated_swings`
    // and `analyzed_without_measurements` — are `narrowed_to`'s recomputation over the narrowing
    // that keeps every swing. Python spells them twice, here and in the narrowing; one definition
    // cannot drift from the other, and M36 P6 measured the two equal on every recorded corpus.
    read.narrowed_to(&Narrowing::default())
}

/// The same corpus restricted to a window, named sessions or one club, counts recomputed.
///
/// A delegate, as Python's is: the filter is [`CareerCorpus::narrowed_to`], on the contract because
/// `analysis`'s per-club builder needs it and may depend on `contracts` alone (ADR-008), and its doc
/// is where the reasoning lives. The name stays here because a caller holding a corpus off disk
/// looks for the operation that narrows one beside the operation that read it.
pub fn narrow_to(corpus: &CareerCorpus, narrowing: &Narrowing) -> CareerCorpus {
    corpus.narrowed_to(narrowing)
}

// ------------------------------------------------------------------------------------- internals

/// A survivor's corpus entry, before any artifact is read: what its manifest says, and the refs of
/// the group's duplicates. `face_on_sha256` is `None` for a photo entry.
///
/// The club is read off *this* manifest — the survivor's — and no duplicate's is consulted, which
/// is where it parts company with `conflicting_shots`. A second shot photo names a repair: one of
/// the two is attached to some swing being scored on it. A duplicate's club is attached to a
/// directory that contributes nothing, and disagrees for a mundane reason — each upload stamps the
/// cursor as it stood when *that* file arrived, so a re-upload after the cursor moved on is a stale
/// reading of the same swing. By the same argument an untagged survivor does not borrow a tagged
/// duplicate's club: that infers what hit the swing from what the cursor said when someone re-sent
/// the clip, the guess `parse_club` refuses at the boundary (R7).
///
/// Built whole before either artifact reader runs, so a field assigned after it cannot be silently
/// absent on every unanalyzed entry. The golfer's `mishit` verdict is lifted here for the same
/// reason; the automatic half is stamped later, once every club's carries are in hand.
fn entry(
    manifest: &SwingManifest,
    face_on_sha256: Option<&str>,
    duplicates: &[SwingManifest],
) -> CorpusSwing {
    CorpusSwing {
        player_id: manifest.player_id.clone().unwrap_or_default(),
        session_id: manifest.session_id.clone(),
        swing_id: manifest.swing_id.clone(),
        captured_at: manifest.created_at,
        face_on_sha256: face_on_sha256.map(str::to_string),
        shot_sha256: manifest
            .roles
            .get(&Role::ShotScreen)
            .map(|file| file.content_sha256.clone()),
        club: manifest.club,
        measurements: Vec::new(),
        duplicates: duplicates.iter().map(swing_ref).collect(),
        conflicting_shots: Vec::new(),
        analyzed: false,
        stale: false,
        analysis_version: 0,
        outdated: false,
        shot_needs_review: false,
        auto_mishit: false,
        manual_mishit: manifest.mishit,
        missing_roles: manifest
            .missing_roles()
            .into_iter()
            .map(|role| role.as_str().to_string())
            .collect(),
        // The survivor's goal, as its club is, and for the same reason: the tag stamped closest to
        // the swing. No duplicate's is consulted. Carried since M35 P8; a survivor that reaches here
        // is tracked, so this is never a drill.
        intent: manifest.intent.clone(),
    }
}

/// A swing's `analysis.json` and `analysis.state.json`, onto its entry.
///
/// Appends to `excluded` for the three states that keep a real swing out of the *counts* without
/// keeping it out of the corpus: never analyzed, analyzed against files that have since changed,
/// and analyzed by an engine whose numbers do not pool. All three are repairable by re-running the
/// pipeline, which is why they are reported as work rather than treated as absence.
fn read_swing_artifacts(
    sessions_dir: &Path,
    manifest: &SwingManifest,
    swing: &mut CorpusSwing,
    excluded: &mut Vec<ExcludedSwing>,
    versions: EngineVersions,
) {
    let swing_dir = swing_dir(sessions_dir, manifest);
    let Some(analysis) = load_analysis(&swing_dir) else {
        excluded.push(exclude(
            manifest,
            ExclusionReason::NotAnalyzed,
            NOT_ANALYZED,
        ));
        return;
    };

    swing.analyzed = true;
    // An unreadable state is no state, so **not** stale; a readable one with no `inputs` matches no
    // manifest with a role, so it is (the storage family's `unreadable-state`).
    if load_state(&swing_dir).is_some_and(|state| !state.matches(manifest)) {
        swing.stale = true;
        excluded.push(exclude(manifest, ExclusionReason::Stale, STALE));
    }

    // The other axis of staleness. `matches` compares the *inputs*, so a result produced by an older
    // engine over unchanged bytes passes it — exactly how the pre-M6.5 swings on disk looked current
    // while carrying no measurements at all. The line is `comparable_from`, not `installed` (the
    // module doc): a swing between the two lacks a newer measurement or two and agrees about the
    // rest, and pools.
    swing.analysis_version = stored_analysis_version(Some(&analysis));
    if is_older_than(Some(&analysis), versions.comparable_from) {
        swing.outdated = true;
        excluded.push(exclude(
            manifest,
            ExclusionReason::Outdated,
            &outdated(
                swing.analysis_version,
                versions.comparable_from,
                "a swing",
                "scripts/reanalyze.py",
            ),
        ));
    }

    let Some(Value::Object(raw_swing)) = analysis.get("swing") else {
        return;
    };
    swing.measurements = measurements(raw_swing.get("measurements"));
    swing.shot_needs_review = needs_review(raw_swing.get("shot"));
}

/// A photo entry's `shot_analysis.json`, onto its entry: the swing's three repairs, in the swing's
/// order, each asked of the one file (M35 P8's finding 4).
///
/// - **Not analyzed** is a file [`load_shot_analysis`] answers `None` from. It reads typed, so a
///   version spelled `16.0` makes the whole file unreadable and the entry `not_analyzed`, where the
///   same value in an `analysis.json` is version 0 and so `outdated` (the hand case
///   `photo-entry-repairs`' entry 6, and `state`'s module doc).
/// - **Stale** is [`shot_analysis_matches`] failing: the result names another photo than the
///   manifest's, or none. It is the photo entry's `analysis.state.json`, so Rust never writes frozen
///   Python's.
/// - **Outdated** is the stored version below `comparable_from`, read off the struct, as a swing's
///   is read off its raw map.
///
/// `shot_needs_review` is the stored shot's own flag. A typed result has already validated its shot,
/// so the swing path's "a shot that no longer validates reads as flagged" cannot arise here.
fn read_photo_artifacts(
    sessions_dir: &Path,
    manifest: &SwingManifest,
    photo_entry: &mut CorpusSwing,
    excluded: &mut Vec<ExcludedSwing>,
    versions: EngineVersions,
) {
    let Some(result) = load_shot_analysis(&swing_dir(sessions_dir, manifest)) else {
        excluded.push(exclude(
            manifest,
            ExclusionReason::NotAnalyzed,
            PHOTO_NOT_ANALYZED,
        ));
        return;
    };

    photo_entry.analyzed = true;
    if !shot_analysis_matches(&result, manifest) {
        photo_entry.stale = true;
        excluded.push(exclude(manifest, ExclusionReason::Stale, PHOTO_STALE));
    }

    photo_entry.analysis_version = result.analysis_version;
    if result.analysis_version < versions.comparable_from {
        photo_entry.outdated = true;
        excluded.push(exclude(
            manifest,
            ExclusionReason::Outdated,
            &outdated(
                result.analysis_version,
                versions.comparable_from,
                "a shot",
                "golf-core run-shot over its session",
            ),
        ));
    }

    photo_entry.shot_needs_review = result
        .shot
        .provenance
        .as_ref()
        .is_some_and(|provenance| provenance.needs_review);
    photo_entry.measurements = result.measurements;
}

/// The `outdated` sentence, for either kind: what it was analyzed by, the line it fell under, and
/// what re-runs it.
fn outdated(stored: i64, line: i64, kind: &str, rerun: &str) -> String {
    format!(
        "analyzed by engine version {stored}, older than {line}, the oldest engine whose numbers \
         still agree with today's — they are not comparable with {kind} analyzed today, so they \
         are reported rather than pooled. Re-run {rerun}",
    )
}

/// Is this survivor's recorded goal one that counts? No goal is the default, `fundamentals`, which
/// does: every manifest on disk before M35. [`contracts::intent::PracticeMode::is_tracked`] is the
/// one rule, derived and never stored (ADR-034 §3).
fn is_tracked(manifest: &SwingManifest) -> bool {
    manifest
        .intent
        .as_ref()
        .is_none_or(|goal| goal.mode.is_tracked())
}

/// Manifests grouped on one hash, in the order each group's first member was scanned.
#[derive(Default)]
struct Groups(Vec<(String, Vec<SwingManifest>)>, HashMap<String, usize>);

impl Groups {
    fn add(&mut self, sha256: String, manifest: SwingManifest) {
        match self.1.get(&sha256) {
            Some(&at) => self.0[at].1.push(manifest),
            None => {
                self.1.insert(sha256.clone(), self.0.len());
                self.0.push((sha256, vec![manifest]));
            }
        }
    }
}

/// A group's survivor and its duplicates in arrival order. Earliest arrival wins: a re-upload's
/// timestamp dates the upload, not the swing, so taking the latest would file a swing under the day
/// someone retested the upload path.
fn survivor_of(mut members: Vec<SwingManifest>) -> (SwingManifest, Vec<SwingManifest>) {
    members.sort_by_key(arrival);
    let survivor = members.remove(0);
    (survivor, members)
}

/// Where a manifest's artifacts are: `manifest.session_id / manifest.swing_id` (the module doc).
fn swing_dir(sessions_dir: &Path, manifest: &SwingManifest) -> PathBuf {
    sessions_dir
        .join(&manifest.session_id)
        .join(&manifest.swing_id)
}

/// A hash as a sentence prints it: its first 12 characters, a clip's or a photo's alike.
fn cut(sha256: &str) -> String {
    sha256.chars().take(12).collect()
}

/// Shot photos on the re-uploads that disagree with the one the survivor kept, sorted.
///
/// Deliberately *not* extra launch-monitor samples. The duplicates share the survivor's face-on
/// bytes, so they are the same physical swing, and one swing produced one ball flight — a second
/// photo is a misattachment, and counting it would put a number into a dispersion that no swing
/// ever produced.
fn conflicting_shots(kept: Option<&str>, duplicates: &[SwingManifest]) -> Vec<String> {
    duplicates
        .iter()
        .filter_map(|manifest| manifest.roles.get(&Role::ShotScreen))
        .map(|shot| shot.content_sha256.as_str())
        .filter(|&sha256| Some(sha256) != kept)
        .collect::<BTreeSet<&str>>()
        .into_iter()
        .map(str::to_string)
        .collect()
}

/// `SwingResult.measurements` back through its own contract, entry by entry.
///
/// Validated rather than hand-mapped so `source` — which decides the whole sample count — arrives
/// with the meaning the pipeline wrote it with. An entry the contract refuses is dropped and the
/// rest kept; anything but a list is no measurements.
///
/// Read through the `Deserialize` *trait*, which validates, and not the inherent `deserialize` that
/// `#[serde(remote = "Self")]` generates beside it, which does not: `Measurement::deserialize` would
/// name the second. [`needs_review`] reads a shot the same way. Today neither choice moves an answer
/// (M36 P10 measured both): `Measurement`'s validator checks nothing, and `ShotData`'s only bound
/// is its provenance's, which the nested field's own trait read already holds. The trait is used so
/// that a validator added to either later is not silently skipped here.
fn measurements(raw: Option<&Value>) -> Vec<Measurement> {
    let Some(Value::Array(entries)) = raw else {
        return Vec::new();
    };
    entries
        .iter()
        .filter_map(|entry| <Measurement as Deserialize>::deserialize(entry).ok())
        .collect()
}

/// Stamp `auto_mishit` on the shots that carried far below their club's own median (ADR-028).
///
/// Must run **before the counts**: the flag has to be visible when the sample count is taken, or the
/// printed `n` and the `n` the baseline pools drift apart. Grouped by club, because a 20-yard carry
/// is a mishit only relative to what *that* club normally does; `contracts::mishit` holds the rule
/// and the fraction. A `manual_mishit` verdict is never touched — [`CorpusSwing::is_mishit`] lets it
/// win — so this sets only the automatic half.
///
/// Each club is judged alone, so the order the clubs are visited in moves nothing, and a `BTreeMap`
/// stands in for Python's insertion-ordered dict.
fn flag_auto_mishits(swings: &mut [CorpusSwing]) {
    let mut by_club: BTreeMap<ClubId, Vec<usize>> = BTreeMap::new();
    for (at, swing) in swings.iter().enumerate() {
        let Some(club) = swing.club else { continue };
        if !swing.counts_toward_metrics()
            || swing.shot_sha256.is_none()
            || swing.shot_needs_review
            || carry_of(swing).is_none()
        {
            continue;
        }
        by_club.entry(club).or_default().push(at);
    }

    for members in by_club.values() {
        // One carry per distinct shot photo — the set the baseline pools, so this median is the one
        // a golfer is shown. `members` is in `captured_at` order, so first-seen is the earliest,
        // matching the survivor the reader kept when it collapsed the re-uploads.
        let floor = {
            let mut seen_photos: BTreeSet<&str> = BTreeSet::new();
            let mut carries: Vec<f64> = Vec::new();
            for &at in members {
                let swing = &swings[at];
                if let (Some(photo), Some(carry)) = (swing.shot_sha256.as_deref(), carry_of(swing))
                {
                    if seen_photos.insert(photo) {
                        carries.push(carry);
                    }
                }
            }
            mishit_carry_floor(&carries)
        };
        let Some(floor) = floor else { continue };
        for &at in members {
            if carry_of(&swings[at]).is_some_and(|carry| carry < floor) {
                swings[at].auto_mishit = true;
            }
        }
    }
}

/// This swing's printed carry — the first `carry_distance_yds` it carries — or `None`.
fn carry_of(swing: &CorpusSwing) -> Option<f64> {
    swing
        .measurements
        .iter()
        .find(|measurement| measurement.name == CARRY)
        .map(|measurement| measurement.value)
}

/// Was the attached shot's OCR parse flagged (ADR-014)?
///
/// Read through [`ShotData`] for the reason Python's is: the provenance rules live in the contract,
/// and a shot that no longer validates should read as untrusted rather than as trusted-by-default.
/// Anything but an object is no shot, and so not flagged.
fn needs_review(raw: Option<&Value>) -> bool {
    match raw {
        Some(shot @ Value::Object(_)) => match <ShotData as Deserialize>::deserialize(shot) {
            Ok(shot) => shot
                .provenance
                .is_some_and(|provenance| provenance.needs_review),
            Err(_) => true,
        },
        _ => false,
    }
}

fn exclude(manifest: &SwingManifest, reason: ExclusionReason, detail: &str) -> ExcludedSwing {
    ExcludedSwing {
        session_id: manifest.session_id.clone(),
        swing_id: manifest.swing_id.clone(),
        reason,
        detail: detail.to_string(),
    }
}

/// `session/swing`.
fn swing_ref(manifest: &SwingManifest) -> String {
    format!("{}/{}", manifest.session_id, manifest.swing_id)
}

/// `_arrival`: the sort key for a duplicate group — arrival time, then a stable tiebreak.
///
/// **The time is CPython's `isoformat()` string, not the instant**, because that is what Python
/// sorts on: in the manifest's own offset, `+00:00` for UTC, and a fraction only when there is one.
/// So `15:00+05:30` loses to `12:00Z` though it is earlier, and a whole second sorts before `.5`
/// because `+` is below `.` (the storage family's `duplicate-across-offsets`).
///
/// The tiebreak is not decoration. Two manifests written in the same second — plausible when a bulk
/// import replays a session — would otherwise make which swing survives depend on directory
/// iteration order, and the corpus would quietly change shape between runs. It compares the ids as
/// strings, so swing `10` survives swing `9` (`duplicate-same-second`).
fn arrival(manifest: &SwingManifest) -> (String, String, String) {
    (
        manifest.created_at.isoformat(),
        manifest.session_id.clone(),
        manifest.swing_id.clone(),
    )
}

/// `len()` or `sum(1 for …)`, as the `i64` the contract's count fields hold.
fn as_count(n: usize) -> i64 {
    i64::try_from(n).expect("a count fits an i64")
}

/// What the storage family's recorded cases cannot reach. Every `corpus/` vector was recorded under
/// `{16, 16}`, where frozen Python's rule and P14's agree, so none says *which* of the two numbers
/// the reader asks — the hand-worked `hand/` cases do, and the first test below says it alone; and
/// two of frozen Python's behaviours have no recorded case at all, one of them found by a negative
/// control that passed the whole family (M36 P10). Each expectation below but the first and the
/// drill's was read off frozen Python's `_run_corpus` over the same tree, by a scratch run. The rest
/// of `tests/storage/test_corpus.py` is a recorded case already, and is gated there rather than
/// copied here. The drill's test has no Python twin, as nothing of M35 P10's rule does: its cases are
/// `spec/vectors/storage/hand/`, and the test says the one thing those cannot, that a drill on disk
/// and no drill at all read the same.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{hash_bytes, MANIFEST_NAME};
    use crate::state::ANALYSIS_NAME;
    use serde_json::json;
    use std::fs;

    const PLAYER: &str = "aaron";
    const AT: &str = "2026-08-09T12:00:00Z";

    /// An analyzed swing for [`PLAYER`], face-on only, created at [`AT`], stored by engine
    /// generation `version` with `shot` as its attached shot.
    fn write_swing(root: &Path, at: (&str, &str), face_on: &[u8], version: i64, shot: Value) {
        let (session, swing) = at;
        let dir = root.join(session).join(swing);
        fs::create_dir_all(&dir).unwrap();
        let manifest = json!({
            "swing_id": swing,
            "session_id": session,
            "created_at": AT,
            "updated_at": AT,
            "roles": {"face_on": {
                "role": "face_on",
                "filename": "face_on.mov",
                "content_sha256": hash_bytes(face_on),
                "original_filename": "clip.mov",
                "content_type": "video/quicktime",
                "size_bytes": 7,
                "received_at": AT,
                "warnings": [],
            }},
            "player_id": PLAYER,
        });
        fs::write(dir.join(MANIFEST_NAME), manifest.to_string()).unwrap();
        let analysis = json!({
            "analysis_version": version,
            "swing": {"measurements": [], "shot": shot},
        });
        fs::write(dir.join(ANALYSIS_NAME), analysis.to_string()).unwrap();
    }

    fn one_swing(version: i64) -> tempfile::TempDir {
        let root = tempfile::tempdir().unwrap();
        write_swing(root.path(), ("2026-08-09", "1"), b"a", version, Value::Null);
        root
    }

    fn read(root: &tempfile::TempDir, installed: i64, comparable_from: i64) -> CareerCorpus {
        let versions = EngineVersions {
            installed,
            comparable_from,
        };
        read_corpus(root.path(), PLAYER, versions)
    }

    /// **The rule asks `comparable_from`, and `installed` moves nothing** (M36 P14). Until P14 this
    /// was the faithful rule's test, with the second and third reads below the other way round:
    /// frozen Python excludes a version-16 swing under an installed 17, and pools it when 17 is only
    /// the line.
    #[test]
    fn outdated_is_older_than_comparable_from_and_installed_is_not_read() {
        let root = one_swing(16);

        let current = read(&root, 16, 16);
        assert!(!current.swings[0].outdated && current.excluded.is_empty());
        assert_eq!(current.swings[0].analysis_version, 16);

        let superseded = read(&root, 17, 16);
        assert!(!superseded.swings[0].outdated && superseded.excluded.is_empty());
        assert_eq!(superseded.outdated_swings, 0);

        let newer_floor = read(&root, 16, 17);
        assert!(newer_floor.swings[0].outdated);
        assert_eq!(newer_floor.outdated_swings, 1);
        let [excluded] = newer_floor.excluded.as_slice() else {
            panic!("one exclusion, got {:?}", newer_floor.excluded)
        };
        assert_eq!(excluded.reason, ExclusionReason::Outdated);
        assert!(
            excluded.detail.starts_with(
                "analyzed by engine version 16, older than 17, the oldest engine whose numbers \
                 still agree with today's — "
            ),
            "the sentence names the line the swing fell under: {}",
            excluded.detail
        );
    }

    /// Rewrite the manifest [`write_swing`] wrote at `at`.
    fn edit_manifest(root: &Path, at: (&str, &str), edit: impl FnOnce(&mut Value)) {
        let path = root.join(at.0).join(at.1).join(MANIFEST_NAME);
        let mut manifest: Value =
            serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        edit(&mut manifest);
        fs::write(&path, manifest.to_string()).unwrap();
    }

    /// Make the swing at `at` a photo entry: its one role a shot photo hashed from `photo`, so its
    /// `analysis.json` is never read (one artifact per kind) and it has no `shot_analysis.json`.
    fn as_photo_entry(root: &Path, at: (&str, &str), photo: &[u8]) {
        edit_manifest(root, at, |manifest| {
            let mut file = manifest["roles"]["face_on"].clone();
            file["role"] = json!("shot_screen");
            file["filename"] = json!("shot_screen.png");
            file["content_sha256"] = json!(hash_bytes(photo));
            manifest["roles"] = json!({ "shot_screen": file });
        });
    }

    /// **A corpus with a drill is the corpus without it, plus one `untracked` exclusion** (the M35
    /// plan's call 11, the first half; the second, over every aggregate and report, is
    /// `crates/core/tests/career.rs`'), and plus the directory it was read from, which
    /// `swing_dirs_seen` counts because it was scanned. One drill of each kind, each beside a
    /// tracked swing and a tracked photo entry; pooled, either would be a third entry in `swings`.
    #[test]
    fn a_corpus_with_a_drill_is_the_corpus_without_it_plus_one_untracked_exclusion() {
        const DRILL: (&str, &str) = ("2026-08-09", "3");
        for drill_is_a_photo_entry in [false, true] {
            let root = tempfile::tempdir().unwrap();
            write_swing(root.path(), ("2026-08-09", "1"), b"a", 17, Value::Null);
            write_swing(root.path(), ("2026-08-09", "2"), b"b", 17, Value::Null);
            as_photo_entry(root.path(), ("2026-08-09", "2"), b"photo-2");
            write_swing(root.path(), DRILL, b"c", 17, Value::Null);
            if drill_is_a_photo_entry {
                as_photo_entry(root.path(), DRILL, b"photo-3");
            }
            edit_manifest(root.path(), DRILL, |manifest| {
                manifest["intent"] = json!({"mode": "drill"});
            });

            let with = read(&root, 17, 14);
            fs::remove_dir_all(root.path().join(DRILL.0).join(DRILL.1)).unwrap();
            let without = read(&root, 17, 14);

            let kind = if drill_is_a_photo_entry {
                "photo entry"
            } else {
                "swing"
            };
            assert_eq!(with.swings.len(), 2, "a drill {kind} is in no swing list");
            let named: Vec<&ExcludedSwing> = with
                .excluded
                .iter()
                .filter(|e| e.swing_ref() == "2026-08-09/3")
                .collect();
            let [drill] = named.as_slice() else {
                panic!("a drill {kind} is named once: {named:?}")
            };
            assert_eq!(
                (drill.reason, drill.detail.as_str()),
                (ExclusionReason::Untracked, UNTRACKED)
            );
            let mut accounted = with.clone();
            accounted
                .excluded
                .retain(|e| e.swing_ref() != "2026-08-09/3");
            accounted.swing_dirs_seen -= 1;
            assert_eq!(accounted, without, "a drill {kind} moved the corpus");
        }
    }

    /// `crate::state::is_outdated` asks this build's engine, which is not the stored one: a
    /// version-16 swing read under `{16, 16}` stays in, whatever `ANALYSIS_VERSION` is (M36 P8's
    /// finding 5). The storage family holds this too, on every recorded swing; this says it alone.
    #[test]
    fn the_reader_does_not_ask_this_builds_engine() {
        assert_ne!(
            contracts::swing::ANALYSIS_VERSION,
            16,
            "the test needs a gap to see"
        );
        assert!(!read(&one_swing(16), 16, 16).swings[0].outdated);
    }

    /// Two swings at one instant in two sessions sort by session first, so `2026-08-09/2` precedes
    /// `2026-08-10/1` though `"1" < "2"`. `sort-order` ties three swings inside one session only, so
    /// a sort that dropped the session key passed the whole family.
    #[test]
    fn a_tie_across_sessions_breaks_on_the_session_before_the_swing() {
        let root = tempfile::tempdir().unwrap();
        write_swing(root.path(), ("2026-08-10", "1"), b"b", 16, Value::Null);
        write_swing(root.path(), ("2026-08-09", "2"), b"a", 16, Value::Null);
        let refs: Vec<String> = read(&root, 16, 16)
            .swings
            .iter()
            .map(CorpusSwing::swing_ref)
            .collect();
        assert_eq!(refs, ["2026-08-09/2", "2026-08-10/1"]);
    }

    /// A shot refused by a **bound** alone — a parse confidence of 1.5, every key present and of its
    /// type — is untrusted, as `ShotData.model_validate` raising makes it in Python. `shot-review`'s
    /// invalid shot lacks a required key, so no recorded case reaches a refusal that comes from a
    /// bound.
    #[test]
    fn a_shot_out_of_bounds_reads_as_needing_review() {
        let shot = json!({
            "shot_id": "t",
            "session_id": "2026-08-09",
            "source": "screen",
            "timestamp": "2026-08-09T12:00:00+00:00",
            "provenance": {
                "device": "hd_golf",
                "parse_confidence": 1.5,
                "needs_review": false,
                "warnings": [],
            },
        });
        let root = tempfile::tempdir().unwrap();
        write_swing(root.path(), ("2026-08-09", "1"), b"a", 16, shot.clone());
        assert!(read(&root, 16, 16).swings[0].shot_needs_review);

        let mut trusted = shot;
        trusted["provenance"]["parse_confidence"] = json!(0.95);
        write_swing(root.path(), ("2026-08-09", "1"), b"a", 16, trusted);
        assert!(!read(&root, 16, 16).swings[0].shot_needs_review);
    }
}
