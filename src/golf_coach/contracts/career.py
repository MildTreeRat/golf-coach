"""One golfer's swings across every session — the shape a personal baseline is cut from.
[Career mode, step 2]

Everything else in this repo judges a swing against a *tour population*. Career mode judges it
against the golfer's own history, and until now that history was not assemblable: every reader is
per-session (`SwingBundleStore.get_session`, `mcp.query.get_session_summary`), so "every swing
Aaron has ever hit" was not a question the code could answer. `storage.corpus` answers it and
returns a `CareerCorpus`.

**The honest `n` is the product here, not the swing list.** Four swing directories sit on disk and
they hold two swings: the same three files were re-uploaded three times while the upload path was
being tested. A reader that counts directories reports `n=4` and hands the baseline one swing's
numbers three times — which does not merely inflate the count, it *collapses the variance*, and
variance is the whole reason career mode is worth building (dispersion splits a static cause from a
timing cause without seeing the body). A confident "your face angle is remarkably repeatable" built
out of a re-upload is the exact failure this milestone was deferred to avoid.

**Two dedupe keys, because a measurement's `n` depends on which artifact it came from.**
`Measurement.source` is `pose:face_on` for the seven pose metrics and `launch_monitor:hd_golf` for
`face_to_path_deg` and `start_line_deg`. So a pose metric's sample count is the number of distinct
**face-on clips**, and a shot metric's is the number of distinct **shot photos**. They agree today
and are free to diverge, in one direction: `bundle_store`'s "newest swing missing this role" rule
can attach one shot photo to two genuinely different swings, which is two pose samples and one
launch-monitor sample. Counting both on a single all-three-roles key would report 2 for
`face_to_path_deg` — a dispersion computed from one reading counted twice.

The opposite direction is not extra data. One face-on clip carrying two *different* shot photos
cannot be two readings, because one physical swing produced one ball flight; one of the photos is
misattached. That is `conflicting_shots`, reported for repair rather than counted — the same
posture `bundle_store` takes toward a misattributed upload, which it documents and hands to a
human instead of engineering around.

**Nothing is excluded silently.** Same principle as `SwingResult.unscored` — a swing dropped from
the corpus is a swing the baseline does not know about, and a count that shrank for a reason nobody
recorded is indistinguishable from one that shrank because the reader has a bug.

Stdlib + pydantic only (ADR-008). Consumed by the pure functions of step 4 (`PersonalBaseline` and
the per-metric minimum-N guard), which is why these shapes live in `contracts/` rather than beside
the reader: `storage` produces them, `analysis` consumes them, and the two never import each other.
"""

from __future__ import annotations

from datetime import datetime
from enum import StrEnum

from pydantic import BaseModel, Field

from golf_coach.contracts.club import ClubId
from golf_coach.contracts.swing import Measurement

#: `Measurement.source` prefixes, and the artifact each provenance dedupes on. A pose metric is
#: derived from the face-on clip's pixels, so two swings sharing that clip are one sample of it; a
#: launch-monitor metric is derived from the photographed shot screen, so two swings sharing that
#: photo are one sample of it. Anything else falls back to swing identity — conservative, and
#: visible, because `CareerCorpus.unknown_sources` names it rather than absorbing it.
POSE_SOURCE_PREFIX = "pose:"
LAUNCH_MONITOR_SOURCE_PREFIX = "launch_monitor:"


class ExclusionReason(StrEnum):
    """Why a swing on disk is not in the corpus. Every one of these is reported, never inferred."""

    #: Nobody had selected a golfer when it arrived. Repairable — the upload page has a per-swing
    #: link, and `scripts/backfill_golfer.py` does it in bulk — so it is listed, not just counted.
    UNATTRIBUTED = "unattributed"
    #: No face-on clip. That is the view every checkpoint is measured from, so this swing can
    #: never carry a pose measurement however long it sits there.
    NO_FACE_ON = "no_face_on"
    #: The same face-on bytes already appear in an earlier swing. The re-upload, not the swing.
    DUPLICATE = "duplicate"
    #: Uploaded but never analyzed — no `analysis.json`. Fixable by running the pipeline.
    NOT_ANALYZED = "not_analyzed"
    #: Analyzed, but a clip has been re-uploaded since; the stored numbers describe bytes that are
    #: no longer on disk (`AnalysisState.matches`). Fixable by re-running the pipeline.
    STALE = "stale"
    #: Analyzed by an older generation of the engine (`analysis_version < ANALYSIS_VERSION`). The
    #: inputs are unchanged — this is the *other* axis of staleness, and the one nothing could see
    #: before the version stamp existed. Kept out of the counts rather than merged into them
    #: because a baseline reads spread: pooling two engine generations manufactures variance out
    #: of a code change, the same way counting a re-uploaded clip twice destroys it. Fixable by
    #: re-running the pipeline (`scripts/reanalyze.py`).
    OUTDATED = "outdated"


class ExcludedSwing(BaseModel):
    """One swing that did not make it into the corpus, and the reason in both forms."""

    session_id: str
    swing_id: str
    reason: ExclusionReason
    detail: str = Field(
        default="",
        description="The sentence a human needs — which swing absorbed a duplicate, and so on.",
    )

    @property
    def ref(self) -> str:
        return f"{self.session_id}/{self.swing_id}"


class CorpusSwing(BaseModel):
    """One **distinct** swing of one golfer's, with the measurements a baseline reads.

    Distinct is doing real work: this is one physical swing, not one swing directory. Any
    re-uploads of the same face-on clip were folded in here and are named in `duplicates`.
    """

    player_id: str
    session_id: str
    swing_id: str

    captured_at: datetime = Field(
        description=(
            "The *earliest* arrival time across this swing and its duplicates. A re-upload's "
            "timestamp is an upload date, not a capture date, so keying a trend on it would date "
            "a swing to the day someone re-tested the upload path."
        )
    )

    face_on_sha256: str = Field(description="Swing identity. Two swings sharing it are one swing.")
    shot_sha256: str | None = Field(
        default=None,
        description=(
            "The shot photo's hash — the identity launch-monitor measurements dedupe on, "
            "independently of the clip. None when no shot screen was uploaded."
        ),
    )

    club: ClubId | None = Field(
        default=None,
        description=(
            "Which club hit it, read off the **survivor's** manifest — the earliest arrival, and "
            "so the tag stamped closest to the swing. A duplicate's club is never consulted: a "
            "re-upload stamps the cursor as it stood when that file arrived, so consulting it "
            "would let a clip re-sent after the cursor moved on rename the swing it duplicates. "
            "None means the swing predates M9 P6 and nothing else, since the upload route now "
            "refuses an untagged swing (ADR-024 §5). **An untagged swing is not excluded** — it "
            "is a good contributor to every pose metric and to every whole-bag number, and is "
            "absent only from per-club views. See `CareerCorpus.untagged_swings`."
        ),
    )

    measurements: list[Measurement] = Field(
        default_factory=list,
        description=(
            "Carried whole rather than flattened to name -> value, because `source` is what "
            "decides which artifact a metric's sample count is keyed on, and `unit`/`detail` are "
            "what make a stored number re-derivable when a band is eventually cut from it."
        ),
    )

    duplicates: list[str] = Field(
        default_factory=list,
        description="`session/swing` refs whose face-on bytes are identical to this one's.",
    )

    conflicting_shots: list[str] = Field(
        default_factory=list,
        description=(
            "Shot-photo hashes carried by this swing's duplicates that differ from `shot_sha256`. "
            "A data error, not a second reading: the same clip is the same swing, and one swing "
            "has one ball flight. Never counted as a sample — surfaced so the misattached photo "
            "can be repaired, since whichever of the two is wrong is attached to some swing."
        ),
    )

    analyzed: bool = Field(default=False, description="An `analysis.json` was found and read.")
    stale: bool = Field(
        default=False,
        description="Analyzed, but an input has been re-uploaded since. Excluded from the counts.",
    )
    analysis_version: int = Field(
        default=0,
        description=(
            "Which generation of the engine wrote this swing's `analysis.json` "
            "(`contracts.swing.ANALYSIS_VERSION`). 0 for artifacts written before the stamp "
            "existed."
        ),
    )
    outdated: bool = Field(
        default=False,
        description=(
            "Analyzed by an older engine than the one installed. The second axis of staleness: "
            "`stale` means the *inputs* moved, this means the *code* did. Excluded from the "
            "counts for the same reason."
        ),
    )
    shot_needs_review: bool = Field(
        default=False,
        description=(
            "The attached shot's OCR parse was flagged (ADR-014). Its numbers stay on the swing "
            "but contribute to no launch-monitor sample count — the same rule "
            "`mcp.query.get_session_summary` applies before averaging a metric."
        ),
    )
    missing_roles: list[str] = Field(default_factory=list)

    @property
    def ref(self) -> str:
        return f"{self.session_id}/{self.swing_id}"

    def counts_toward_metrics(self) -> bool:
        """May this swing contribute a sample?

        Analyzed, still describing its own inputs, and produced by the engine installed now.
        The three conditions are one question asked of three different things — the artifact,
        the bytes under it, and the code that joined them.
        """
        return self.analyzed and not self.stale and not self.outdated

    def artifact_key(self, measurement: Measurement) -> str | None:
        """Which artifact this measurement is a reading *of*, or None if it contributes nothing.

        **The single definition of the dedupe rule**, called by both sides that need it:
        `storage.corpus` to count samples, and `analysis.baseline` to pool the values behind those
        counts. Two definitions would be worse than none — the printed `n` and the number of values
        actually averaged would disagree, silently, and only in the cases the rule exists for.

        Keys are namespaced (`pose:` / `shot:` / `swing:`) so a clip hash can never collide with a
        photo hash. Returning the same key from two swings is the assertion that they are one
        reading; returning None is the assertion that there is no reading here at all.
        """
        if measurement.source.startswith(POSE_SOURCE_PREFIX):
            return f"pose:{self.face_on_sha256}"
        if measurement.source.startswith(LAUNCH_MONITOR_SOURCE_PREFIX):
            # A flagged parse contributes nothing rather than a suspect sample — the rule
            # `mcp.query.get_session_summary` already applies before averaging a shot metric.
            if self.shot_needs_review or self.shot_sha256 is None:
                return None
            return f"shot:{self.shot_sha256}"
        # An unrecognised provenance falls back to swing identity: conservative, since it can only
        # over-count relative to a real artifact key, never under-count. `CareerCorpus
        # .unknown_sources` names the source so a new provenance cannot silently acquire it.
        return f"swing:{self.ref}"


class CareerCorpus(BaseModel):
    """Every distinct swing of one golfer's, and the honest count behind each metric."""

    player_id: str

    swings: list[CorpusSwing] = Field(
        default_factory=list, description="Distinct swings, oldest `captured_at` first."
    )

    sessions_scanned: int = 0
    swing_dirs_seen: int = Field(
        default=0,
        description=(
            "Swing directories read across every session, for every golfer. The number a naive "
            "count would have reported; the gap between this and `distinct_swings` is the point."
        ),
    )

    metric_counts: dict[str, int] = Field(
        default_factory=dict,
        description=(
            "metric name -> honest sample size. **Counts distinct contributing artifacts, not "
            "swings**: a pose metric counts distinct face-on clips, a launch-monitor metric counts "
            "distinct shot photos, so three re-uploads of one swing contribute 1. This is the "
            "field the per-metric minimum-N guard reads, and it should read nothing else — "
            "`len(swings)` is not a sample size for any particular metric."
        ),
    )

    outdated_swings: int = Field(
        default=0,
        description=(
            "Distinct swings whose `analysis.json` was written by an older engine. Career mode "
            "step 3's worklist, and the general form of it: re-run `scripts/reanalyze.py` and "
            "they rejoin the counts. Every one of them is also itemised in `excluded`."
        ),
    )

    analyzed_without_measurements: int = Field(
        default=0,
        description=(
            "Distinct swings that count toward the metrics and still carry no measurement at "
            "all. Narrower than it looks, and deliberately: a pre-M6.5 artifact is `outdated`, "
            "so it is *not* here. What lands here is a current-engine swing where every metric "
            "returned None — measurement failed, rather than the engine being old — which is a "
            "different problem with a different fix. Their `checkpoint_scores` may still hold "
            "`observed` values; those are never read as measurements, because it would mix two "
            "derivation paths under one metric name."
        ),
    )

    unattributed_swings: int = Field(
        default=0, description="Swings on disk naming no golfer. Also itemised in `excluded`."
    )
    other_golfers: int = Field(
        default=0,
        description=(
            "Swings belonging to someone else. Counted for context but not itemised — they are "
            "correctly absent, unlike an unattributed swing, which may be repairable into this "
            "corpus."
        ),
    )

    unknown_sources: list[str] = Field(
        default_factory=list,
        description=(
            "`Measurement.source` values matching neither known prefix, sorted. They fall back to "
            "swing identity for counting; naming them here is what stops a new provenance from "
            "silently acquiring the wrong sample size."
        ),
    )

    excluded: list[ExcludedSwing] = Field(
        default_factory=list,
        description=(
            "Every swing on disk that contributes **no sample to any metric count**, with its "
            "reason. Not the complement of `swings`: a distinct swing that is merely unanalyzed, "
            "stale or outdated appears in both — it is a real swing of this golfer's, it just "
            "carries no usable numbers yet, and all three of those states are repaired by "
            "running the pipeline rather than by capturing anything new."
        ),
    )

    @property
    def distinct_swings(self) -> int:
        return len(self.swings)

    @property
    def distinct_shots(self) -> int:
        """Distinct shot photos across the corpus — the ceiling on any launch-monitor metric."""
        return len({swing.shot_sha256 for swing in self.swings if swing.shot_sha256 is not None})

    @property
    def untagged_swings(self) -> int:
        """Distinct swings naming no club — everything per-club work cannot see.

        **Derived, where `unattributed_swings` is tallied, and the asymmetry is the whole reason.**
        A manifest naming no golfer never becomes a `CorpusSwing` at all — it is excluded before
        the swings are grouped — so that counter has to be counted during the scan or not at all.
        A manifest naming no *club* is a real swing sitting in `swings`, so this reads it back off
        them. That is what stops `storage.corpus.narrow_to` reporting the whole read's figure
        beside a filtered swing list: there is no field for it to forget to recompute.

        **Deliberately not an `ExclusionReason`** (ADR-024, Consequences). An untagged swing is a
        perfectly good contributor to every pose metric — the club was never an input to measuring
        head sway — so excluding it would shrink the mechanics `n` to punish a missing tag that
        mechanics never needed. It is counted here and nowhere else, and it shares
        `distinct_swings`' denominator so the two can be honestly printed in one sentence.
        """
        return sum(1 for swing in self.swings if swing.club is None)

    @property
    def duplicates_collapsed(self) -> int:
        return sum(len(swing.duplicates) for swing in self.swings)

    @property
    def shot_conflicts(self) -> int:
        """Swings whose re-uploads disagree about which shot photo belongs to them."""
        return sum(1 for swing in self.swings if swing.conflicting_shots)
