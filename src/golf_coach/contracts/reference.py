"""Reference-swing contract — one analyzed swing from an external corpus. [M4-REF]

A `ReferenceSwing` is the per-clip record behind a benchmark band: the metadata we stratify on,
the ground-truth event frames, and every metric derived from them. Benchmark bands in
`ranges.json` are *aggregates* over thousands of these, so this is the layer you re-cut bands from
when a definition changes — without re-running pose estimation, which is the expensive step.

**Interchange, not storage.** These are written one-per-line as JSONL under `data/reference/`
(gitignored). The shape is deliberately the shape the future `swings` table will take (ROADMAP M4,
`storage/`), so persisting them later is a loader change rather than a redesign.

`events` and `metrics` are open dicts on purpose: every corpus names its own instants (GolfDB
labels eight; our own captures will label fewer), and the metric vocabulary grows every time a
checkpoint is added. Pinning either into fixed fields would mean a contract change per corpus.

Pydantic only — no pandas, no numpy. The base install can read these (ADR-008).
"""

from __future__ import annotations

from pydantic import BaseModel, Field


class ReferenceSwing(BaseModel):
    """One swing from a reference corpus, with its ground-truth events and derived metrics."""

    source: str = Field(description="Corpus name, e.g. 'golfdb'.")
    source_id: str = Field(description="Clip identifier within that corpus.")

    group_id: str | None = Field(
        default=None,
        description=(
            "Clips sharing a group came from one source video. Any split or resampling must keep "
            "a group together, or the same swing leaks across both sides."
        ),
    )
    subject: str | None = Field(
        default=None,
        description=(
            "The golfer. Used to count *distinct people* behind a band — 200 clips from 6 players "
            "is not a sample of 200 — not to identify anyone."
        ),
    )

    sex: str | None = None
    club: str | None = Field(
        default=None,
        description="Club as the corpus names it, not our ClubCategory; mapping is the consumer's.",
    )
    view: str | None = Field(default=None, description="Camera view, e.g. 'face-on'.")
    slow_motion: bool = Field(
        default=False,
        description=(
            "Slow-motion capture. Frame *ratios* stay valid; absolute durations do not, because "
            "the container fps no longer reflects real time."
        ),
    )

    pixel_aspect: float = Field(
        default=1.0,
        description=(
            "Multiply normalized `y` by this to put it in the same units as normalized `x`. "
            "1.0 when the frame's pixels are square in the original scene. Corpora built by "
            "resizing non-square crops to a square (GolfDB's videos_160) squash the axes by "
            "different factors, so any metric mixing x and y is biased without it; metrics that "
            "are a ratio of two x-distances are unaffected."
        ),
        gt=0.0,
    )

    events: dict[str, int] = Field(
        default_factory=dict,
        description="Ground-truth instant name -> frame index within the clip.",
    )
    metrics: dict[str, float] = Field(
        default_factory=dict,
        description="Derived metric name -> value. Unitless ratios unless the name says otherwise.",
    )

    pose_estimator: str | None = Field(
        default=None,
        description=(
            "What produced the pose-derived metrics, if any (e.g. 'mediapipe:lite'). None means "
            "this row carries label-derived metrics only."
        ),
    )
    metric_definitions_version: int = Field(
        default=1,
        description=(
            "Which generation of metric definitions produced `metrics`. A band is only comparable "
            "to a swing measured under the same version — see docs/M4_POSE_BAKEOFF.md."
        ),
    )
