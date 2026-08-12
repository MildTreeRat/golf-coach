"""Every swing one golfer has hit, across every session, counted honestly. [Career mode, step 2]

The reader behind `contracts.career.CareerCorpus`. Pure reads over artifacts that already exist —
`manifest.json` for identity, `analysis.json` for measurements, `analysis.state.json` for whether
those measurements still describe the files on disk. Nothing here runs pose, opens a video or
re-hashes a byte: `RoleFile.content_sha256` was recorded as the upload streamed in, and it is the
same digest a re-read would produce.

**What makes this more than a `for` loop over directories.** Four swing directories currently hold
two swings — the same three files were re-uploaded three times while the upload path was being
tested. Counting directories would hand a personal baseline one swing's numbers three times, which
does not just inflate `n`, it collapses the variance the baseline exists to measure. So swings are
grouped by the face-on clip's hash and shot metrics are counted on the shot photo's hash, which are
independent keys for the reason `contracts.career` sets out at length.

**The exclusions are output, not control flow.** Every swing that contributes no sample is named in
`CareerCorpus.excluded` with a reason. A count that shrank silently is indistinguishable from a
reader with a bug, and this is the one place in the pipeline where too small an `n` and too large an
`n` are both wrong in the same confident voice.

Base install only — no vision, no OCR, no MCP SDK.
"""

from __future__ import annotations

from pathlib import Path

# `storage` importing `api` inverts ADR-008's direction. It is deliberate and follows the
# precedent `mcp/query.py` set: `load_analysis` / `load_state` are the tolerant readers for the two
# artifacts an analysis run leaves behind, and a second copy of a tolerant reader is a second copy
# that drifts. The honest fix is moving them to `storage/analysis_io.py` and re-exporting from
# `api.state` — a contained change, and not this commit's.
from golf_coach.api.state import (
    is_outdated,
    load_analysis,
    load_state,
    stored_analysis_version,
)
from golf_coach.contracts.career import (
    LAUNCH_MONITOR_SOURCE_PREFIX,
    POSE_SOURCE_PREFIX,
    CareerCorpus,
    CorpusSwing,
    ExcludedSwing,
    ExclusionReason,
)
from golf_coach.contracts.shot import ShotData
from golf_coach.contracts.swing import ANALYSIS_VERSION, Measurement
from golf_coach.storage.bundle_store import SwingBundleStore
from golf_coach.storage.manifest import Role, SwingManifest


def read_corpus(sessions_dir: Path, player_id: str) -> CareerCorpus:
    """Assemble one golfer's distinct swings and the honest sample size behind each metric.

    An unknown `player_id`, or a `sessions_dir` that does not exist, is an empty corpus rather
    than an error — "this golfer has no swings yet" is a real answer and the first one every
    golfer has.
    """
    store = SwingBundleStore(sessions_dir)
    session_ids = store.list_session_ids()

    excluded: list[ExcludedSwing] = []
    mine: list[SwingManifest] = []
    seen = 0
    unattributed = 0
    other_golfers = 0

    for session_id in session_ids:
        for manifest in store.get_session(session_id):
            seen += 1
            if manifest.player_id is None:
                unattributed += 1
                excluded.append(
                    _excluded(
                        manifest,
                        ExclusionReason.UNATTRIBUTED,
                        "nobody had selected a golfer when this arrived — repair it on the "
                        "upload page or with scripts/backfill_golfer.py",
                    )
                )
            elif manifest.player_id == player_id:
                mine.append(manifest)
            else:
                other_golfers += 1

    groups: dict[str, list[SwingManifest]] = {}
    for manifest in mine:
        face_on = manifest.roles.get(Role.FACE_ON)
        if face_on is None:
            excluded.append(
                _excluded(
                    manifest,
                    ExclusionReason.NO_FACE_ON,
                    "no face-on clip, and that is the view every checkpoint is measured from",
                )
            )
            continue
        groups.setdefault(face_on.content_sha256, []).append(manifest)

    swings: list[CorpusSwing] = []
    for sha256, members in groups.items():
        # Earliest arrival wins. A re-upload's timestamp dates the upload, not the swing, so
        # taking the latest would file a swing under the day someone retested the upload path.
        members.sort(key=_arrival)
        survivor, duplicates = members[0], members[1:]
        for duplicate in duplicates:
            excluded.append(
                _excluded(
                    duplicate,
                    ExclusionReason.DUPLICATE,
                    f"face-on bytes {sha256[:12]} are already in {_ref(survivor)} — "
                    "the same swing uploaded again, not a second swing",
                )
            )
        swing = _corpus_swing(
            sessions_dir, survivor, sha256, [_ref(m) for m in duplicates], excluded
        )
        swing.conflicting_shots = _conflicting_shots(swing.shot_sha256, duplicates)
        swings.append(swing)

    swings.sort(key=lambda swing: (swing.captured_at, swing.session_id, swing.swing_id))
    metric_counts, unknown_sources = _count_metrics(swings)

    return CareerCorpus(
        player_id=player_id,
        swings=swings,
        sessions_scanned=len(session_ids),
        swing_dirs_seen=seen,
        metric_counts=metric_counts,
        outdated_swings=sum(1 for swing in swings if swing.outdated),
        analyzed_without_measurements=sum(
            1 for swing in swings if swing.counts_toward_metrics() and not swing.measurements
        ),
        unattributed_swings=unattributed,
        other_golfers=other_golfers,
        unknown_sources=unknown_sources,
        excluded=excluded,
    )


# --------------------------------------------------------------------------------------
# Internals
# --------------------------------------------------------------------------------------


def _corpus_swing(
    sessions_dir: Path,
    manifest: SwingManifest,
    face_on_sha256: str,
    duplicates: list[str],
    excluded: list[ExcludedSwing],
) -> CorpusSwing:
    """One survivor, with whatever its analysis artifacts say about it.

    Appends to `excluded` for the three states that keep a real swing out of the *counts* without
    keeping it out of the corpus: never analyzed, analyzed against files that have since changed,
    and analyzed by an engine older than the installed one. All three are repairable by re-running
    the pipeline, which is why they are reported as work rather than treated as absence.
    """
    swing_dir = sessions_dir / manifest.session_id / manifest.swing_id
    shot_file = manifest.roles.get(Role.SHOT_SCREEN)

    swing = CorpusSwing(
        player_id=manifest.player_id or "",
        session_id=manifest.session_id,
        swing_id=manifest.swing_id,
        captured_at=manifest.created_at,
        face_on_sha256=face_on_sha256,
        shot_sha256=shot_file.content_sha256 if shot_file else None,
        duplicates=duplicates,
        missing_roles=[role.value for role in manifest.missing_roles()],
    )

    analysis = load_analysis(swing_dir)
    if analysis is None:
        excluded.append(
            _excluded(
                manifest,
                ExclusionReason.NOT_ANALYZED,
                "no analysis.json — run scripts/analyze_bundle.py over it",
            )
        )
        return swing

    swing.analyzed = True
    state = load_state(swing_dir)
    if state is not None and not state.matches(manifest):
        swing.stale = True
        excluded.append(
            _excluded(
                manifest,
                ExclusionReason.STALE,
                "a clip was re-uploaded after this was analyzed, so the stored numbers "
                "describe bytes that are no longer here — re-run the pipeline",
            )
        )

    # The other axis of staleness. `matches` compares the *inputs*, so a result produced by an
    # older engine over unchanged bytes passes it — which is exactly how the pre-M6.5 swings on
    # disk looked current while carrying no measurements at all.
    swing.analysis_version = stored_analysis_version(analysis)
    if is_outdated(analysis):
        swing.outdated = True
        excluded.append(
            _excluded(
                manifest,
                ExclusionReason.OUTDATED,
                f"analyzed by engine version {swing.analysis_version}, and "
                f"{ANALYSIS_VERSION} is installed — the numbers are not comparable with a "
                "swing analyzed today, so they are reported rather than pooled. Re-run "
                "scripts/reanalyze.py",
            )
        )

    raw_swing = analysis.get("swing")
    if not isinstance(raw_swing, dict):
        return swing

    swing.measurements = _measurements(raw_swing.get("measurements"))
    swing.shot_needs_review = _needs_review(raw_swing.get("shot"))
    return swing


def _conflicting_shots(kept: str | None, duplicates: list[SwingManifest]) -> list[str]:
    """Shot photos on the re-uploads that disagree with the one the survivor kept.

    Deliberately *not* treated as extra launch-monitor samples. The duplicates share the
    survivor's face-on bytes, so they are the same physical swing, and one swing produced one ball
    flight — a second photo is a misattachment, and counting it would put a number into a
    dispersion that no swing ever produced.
    """
    others = {
        shot.content_sha256
        for manifest in duplicates
        if (shot := manifest.roles.get(Role.SHOT_SCREEN)) is not None
    }
    return sorted(others - ({kept} if kept is not None else set()))


def _measurements(raw: object) -> list[Measurement]:
    """`SwingResult.measurements` back through its own contract.

    Validated rather than hand-mapped so `source` — which decides the whole sample count — arrives
    with the same meaning the pipeline wrote it with.
    """
    if not isinstance(raw, list):
        return []
    out: list[Measurement] = []
    for entry in raw:
        try:
            out.append(Measurement.model_validate(entry))
        except ValueError:
            continue
    return out


def _needs_review(raw: object) -> bool:
    """Was the attached shot's OCR parse flagged (ADR-014)?

    Read through `ShotData` for the same reason `mcp.query._shot_view_from_dict` does: the
    provenance rules live in the contract, and a shot that no longer validates should read as
    untrusted rather than as trusted-by-default.
    """
    if not isinstance(raw, dict):
        return False
    try:
        shot = ShotData.model_validate(raw)
    except ValueError:
        return True
    return bool(shot.provenance and shot.provenance.needs_review)


def _count_metrics(swings: list[CorpusSwing]) -> tuple[dict[str, int], list[str]]:
    """metric -> distinct contributing artifacts, and any `source` neither prefix claimed.

    The keying itself is `CorpusSwing.artifact_key` and deliberately not repeated here: career
    mode step 4 pools the *values* behind these counts, and a second copy of the rule is a way for
    the printed `n` and the number of values averaged under it to drift apart. What stays here is
    `unknown_sources`, which is a report about the reader's coverage rather than part of the rule.
    """
    artifacts: dict[str, set[str]] = {}
    unknown: set[str] = set()

    for swing in swings:
        if not swing.counts_toward_metrics():
            continue
        for measurement in swing.measurements:
            if not (
                measurement.source.startswith(POSE_SOURCE_PREFIX)
                or measurement.source.startswith(LAUNCH_MONITOR_SOURCE_PREFIX)
            ):
                unknown.add(measurement.source)
            key = swing.artifact_key(measurement)
            if key is None:
                continue
            artifacts.setdefault(measurement.name, set()).add(key)

    counts = {name: len(keys) for name, keys in sorted(artifacts.items())}
    return counts, sorted(unknown)


def _excluded(manifest: SwingManifest, reason: ExclusionReason, detail: str) -> ExcludedSwing:
    return ExcludedSwing(
        session_id=manifest.session_id,
        swing_id=manifest.swing_id,
        reason=reason,
        detail=detail,
    )


def _ref(manifest: SwingManifest) -> str:
    return f"{manifest.session_id}/{manifest.swing_id}"


def _arrival(manifest: SwingManifest) -> tuple[str, str, str]:
    """Sort key for a duplicate group: arrival time, then a stable tiebreak.

    The tiebreak is not decoration. Two manifests written in the same second — plausible when a
    bulk import replays a session — would otherwise make which swing survives depend on directory
    iteration order, and the corpus would quietly change shape between runs.
    """
    return (manifest.created_at.isoformat(), manifest.session_id, manifest.swing_id)
