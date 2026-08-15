"""One follow-up conversation about a swing, as it is kept between turns. [M6, ADR-020]

The Messages API is stateless: every turn resends the whole history. So a conversation that
survives a process exit needs the history on disk, and the only safe thing to write there is
**exactly what the model produced**.

That is the one design rule in this file, and it is not a stylistic preference:

  - **Thinking blocks must be replayed unchanged.** `settings.coaching_model` is `claude-opus-5`,
    where thinking is on by default. A transcript that stored rendered text and rebuilt the rest
    could not replay — the blocks are not reconstructable, and they are bound to the model that
    produced them, which is why `model` is recorded beside them.
  - **A `tool_use` block and its `tool_result` are a matched pair.** Dropping or re-deriving
    either breaks the turn it belongs to.

So `messages` holds content-block dicts verbatim — no filtering, no text extraction, no
normalisation. Anything that wants prose out of a transcript renders it at the point of display
(`storage.transcript_store.visible_turns`), and the stored bytes stay the model's own.

The shapes here are deliberately *not* the SDK's types. `contracts/` depends on pydantic and
stdlib alone (ADR-008), and a base install has no `anthropic` — so the blocks are plain `dict`.
That is a real trade: nothing here validates that a stored block is a well-formed block. The
alternative was either a heavy dependency in `contracts/` or a hand-written mirror of the SDK's
union that would drift the first time a block type was added, which is the worse of the three.
"""

from __future__ import annotations

from datetime import datetime
from typing import Any

from pydantic import BaseModel, Field

#: Bumped when a stored transcript can no longer be replayed by the current code. Mirrors
#: `ANALYSIS_VERSION`'s job for `analysis.json`: the reader compares rather than guesses, and an
#: older transcript is *reseeded* rather than half-understood.
TRANSCRIPT_VERSION = 1


class Transcript(BaseModel):
    """One conversation, replayable as-is into the model that recorded it."""

    conversation_id: str
    transcript_version: int = TRANSCRIPT_VERSION
    created_at: datetime
    updated_at: datetime

    #: The model these blocks were recorded against. **A replay guard, not bookkeeping**: thinking
    #: blocks are model-bound, so a resume under a changed `coaching_model` has to reseed rather
    #: than replay. Without this the failure is a 400 on turn three, long after the config moved.
    model: str

    #: The swing this was seeded from, when there was one. A conversation can range wider than its
    #: seed — "how does this compare to Tuesday" is a different session — so these are the origin,
    #: never a filter on what the tools may read.
    session_id: str | None = None
    swing_id: str | None = None

    #: sha256 of the brief this was seeded with, the same trick `CoachingProvenance.input_digest`
    #: plays for a stored coaching paragraph. A swing that has been re-analyzed since gives a
    #: different digest, so a resumed conversation can say the numbers moved instead of quietly
    #: discussing figures nothing on disk agrees with any more.
    brief_digest: str | None = None

    #: The wire history, verbatim. Every entry is a `{"role": ..., "content": [...]}` mapping in
    #: the shape the Messages API accepts, with thinking and tool blocks intact — see this
    #: module's docstring for why nothing is stripped.
    messages: list[dict[str, Any]] = Field(default_factory=list)

    #: What a reader of this conversation should know — a coaching call's `notes`, one level up.
    #: An expected failure (no key, no extra, an exhausted iteration budget) lands here rather
    #: than raising, so a transcript records the turn that did not happen as well as the ones
    #: that did.
    notes: list[str] = Field(default_factory=list)

    def replayable_by(self, model: str) -> bool:
        """Can this history be sent to `model` as-is?

        False means reseed. The check is the model identity rather than a capability flag because
        that is the actual constraint: blocks recorded under one model are not portable to
        another, whatever the two models can otherwise do.
        """
        return self.model == model and self.transcript_version == TRANSCRIPT_VERSION
