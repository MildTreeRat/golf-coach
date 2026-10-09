//! Every swing one golfer has hit, across every session, counted honestly. `storage/corpus.py`.
//! [M36 P10]
//!
//! The reader behind [`CareerCorpus`]. Pure reads over artifacts that already exist —
//! `manifest.json` for identity, `analysis.json` for measurements, `analysis.state.json` for whether
//! those measurements still describe the files on disk. Nothing here runs pose, opens a video or
//! re-hashes a byte: [`crate::manifest::RoleFile::content_sha256`] was recorded as the upload
//! streamed in, and it is the same digest a re-read would produce.
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
//! - **`excluded` is scan order.** Sessions by code point, swing directories in
//!   [`SwingBundleStore::get_session`]'s numeric order, then the unattributed, the faceless, and each
//!   face-on group in the order its first member was scanned, its duplicates in arrival order and its
//!   survivor's own exclusions after them.
//! - **A duplicate group's survivor is the earliest by `arrival`**, a *string* sort on CPython's
//!   `isoformat()`, not on the instant (M36 P1's finding 5).
//! - **`swings` is by instant**, then session and swing id as strings, stable.
//! - **The analysis and state are read from `manifest.session_id / manifest.swing_id`**, not from
//!   the directory the manifest was found in. The manifest is trusted over its directory here as the
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
use std::path::Path;

use contracts::career::{CareerCorpus, CorpusSwing, ExcludedSwing, ExclusionReason, Narrowing};
use contracts::club::ClubId;
use contracts::mishit::mishit_carry_floor;
use contracts::shot::ShotData;
use contracts::swing::Measurement;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::bundle_store::SwingBundleStore;
use crate::manifest::{Role, SwingManifest};
use crate::state::{is_older_than, load_analysis, load_state, stored_analysis_version};

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
const NO_FACE_ON: &str = "no face-on clip, and that is the view every checkpoint is measured from";
const NOT_ANALYZED: &str = "no analysis.json — run scripts/analyze_bundle.py over it";
const STALE: &str =
    "a clip was re-uploaded after this was analyzed, so the stored numbers describe \
                     bytes that are no longer here — re-run the pipeline";

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

    // Grouped by the face-on clip's hash, in the order each group's first member was scanned: a
    // Python dict's insertion order, which `excluded` inherits.
    let mut groups: Vec<(String, Vec<SwingManifest>)> = Vec::new();
    let mut group_of: HashMap<String, usize> = HashMap::new();
    for manifest in mine {
        let Some(face_on) = manifest.roles.get(&Role::FaceOn) else {
            excluded.push(exclude(&manifest, ExclusionReason::NoFaceOn, NO_FACE_ON));
            continue;
        };
        let sha256 = face_on.content_sha256.clone();
        match group_of.get(&sha256) {
            Some(&at) => groups[at].1.push(manifest),
            None => {
                group_of.insert(sha256.clone(), groups.len());
                groups.push((sha256, vec![manifest]));
            }
        }
    }

    let mut swings: Vec<CorpusSwing> = Vec::with_capacity(groups.len());
    for (sha256, mut members) in groups {
        // Earliest arrival wins. A re-upload's timestamp dates the upload, not the swing, so taking
        // the latest would file a swing under the day someone retested the upload path.
        members.sort_by_key(arrival);
        let (survivor, duplicates) = members.split_first().expect("a group has a member");
        for duplicate in duplicates {
            excluded.push(exclude(
                duplicate,
                ExclusionReason::Duplicate,
                &format!(
                    "face-on bytes {} are already in {} — the same swing uploaded again, not a \
                     second swing",
                    sha256.chars().take(12).collect::<String>(),
                    swing_ref(survivor),
                ),
            ));
        }
        let mut swing = corpus_swing(
            sessions_dir,
            survivor,
            &sha256,
            duplicates.iter().map(swing_ref).collect(),
            &mut excluded,
            versions,
        );
        swing.conflicting_shots = conflicting_shots(swing.shot_sha256.as_deref(), duplicates);
        swings.push(swing);
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

/// One survivor, with whatever its analysis artifacts say about it.
///
/// Appends to `excluded` for the three states that keep a real swing out of the *counts* without
/// keeping it out of the corpus: never analyzed, analyzed against files that have since changed,
/// and analyzed by an engine whose numbers do not pool. All three are repairable by re-running the
/// pipeline, which is why they are reported as work rather than treated as absence.
fn corpus_swing(
    sessions_dir: &Path,
    manifest: &SwingManifest,
    face_on_sha256: &str,
    duplicates: Vec<String>,
    excluded: &mut Vec<ExcludedSwing>,
    versions: EngineVersions,
) -> CorpusSwing {
    let swing_dir = sessions_dir
        .join(&manifest.session_id)
        .join(&manifest.swing_id);
    let shot_file = manifest.roles.get(&Role::ShotScreen);

    // The club is read off *this* manifest — the survivor's — and no duplicate's is consulted, which
    // is where it parts company with `conflicting_shots`. A second shot photo names a repair: one of
    // the two is attached to some swing being scored on it. A duplicate's club is attached to a
    // directory that contributes nothing, and disagrees for a mundane reason — each upload stamps the
    // cursor as it stood when *that* file arrived, so a re-upload after the cursor moved on is a stale
    // reading of the same swing. By the same argument an untagged survivor does not borrow a tagged
    // duplicate's club: that infers what hit the swing from what the cursor said when someone re-sent
    // the clip, the guess `parse_club` refuses at the boundary (R7).
    //
    // Built whole before the first early return, so a field assigned after it cannot be silently
    // absent on every unanalyzed swing. The golfer's `mishit` verdict is lifted here for the same
    // reason; the automatic half is stamped later, once every club's carries are in hand.
    let mut swing = CorpusSwing {
        player_id: manifest.player_id.clone().unwrap_or_default(),
        session_id: manifest.session_id.clone(),
        swing_id: manifest.swing_id.clone(),
        captured_at: manifest.created_at,
        face_on_sha256: face_on_sha256.to_string(),
        shot_sha256: shot_file.map(|file| file.content_sha256.clone()),
        club: manifest.club,
        measurements: Vec::new(),
        duplicates,
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
    };

    let Some(analysis) = load_analysis(&swing_dir) else {
        excluded.push(exclude(
            manifest,
            ExclusionReason::NotAnalyzed,
            NOT_ANALYZED,
        ));
        return swing;
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
            &format!(
                "analyzed by engine version {}, older than {}, the oldest engine whose numbers \
                 still agree with today's — they are not comparable with a swing analyzed today, so \
                 they are reported rather than pooled. Re-run scripts/reanalyze.py",
                swing.analysis_version, versions.comparable_from,
            ),
        ));
    }

    let Some(Value::Object(raw_swing)) = analysis.get("swing") else {
        return swing;
    };
    swing.measurements = measurements(raw_swing.get("measurements"));
    swing.shot_needs_review = needs_review(raw_swing.get("shot"));
    swing
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
/// control that passed the whole family (M36 P10). Each expectation below but the first was read off
/// frozen Python's `_run_corpus` over the same tree, by a scratch run. The rest of
/// `tests/storage/test_corpus.py` is a recorded case already, and is gated there rather than copied
/// here.
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
