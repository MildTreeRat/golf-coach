"""Which mechanics checkpoints exist, and what to call them.

**Identity only — no evaluator lives here.** The functions that score these are
`analysis/checkpoints/mechanics.py`, bound to the names below by
`analysis.checkpoints.CHECKPOINT_EVALUATORS`. This module is the half that `contracts` is allowed
to know: a name, a word for it in prose, the metric it reads, and which way its band points.

Why the split, and why this half is in `contracts/` at all: `caveats.py` has to *say* how many
checkpoints there are and name them, that text ships into the coaching system prompt and to every
MCP client, and ADR-008 forbids `contracts` importing `analysis`. Hand-typing the list here was the
alternative and it is exactly what went wrong — `caveats.py` claimed five checkpoints for a
milestone while six shipped, so the product told the model a false fact about itself on every
swing. Deriving the sentence from a registry deletes that failure mode rather than fixing one
instance of it.

The direction is also just correct. Checkpoint identity was already contract vocabulary before this
module existed: `CheckpointScore.one_sided` is the field consumers read to know which way a band
points, and `dispersion.METRIC_TARGETS` is keyed by the same metric names. Both were describing
this registry without having one.

**The registry is the point** (the same argument `measure.POSE_MEASUREMENTS` makes): adding a
checkpoint touches thirteen places today, which is thirteen chances to miss one — and the two that
went stale were both prose, because prose has no test. Everything derivable from a spec is now
derived from it, so a partial edit is impossible rather than merely unlikely.

**Order is load-bearing.** `SwingResult.unscored` is built by walking this tuple, so the order below
is the order an unscored checkpoint is reported in. Append rather than insert.
"""

from __future__ import annotations

from typing import NamedTuple


class CheckpointSpec(NamedTuple):
    """One checkpoint's identity: what it is called, and what shape its band has.

    A `NamedTuple` rather than a `BaseModel` because this is a compile-time constant, never parsed
    from JSON and never validated at a boundary — the same reason `benchmarks.store.ResolvedRange`
    is one.
    """

    #: The name on `CheckpointScore.name` and in `SwingResult.unscored`. The vocabulary consumers
    #: match on, so it is an identifier rather than prose.
    name: str
    #: The same checkpoint in a sentence. Used to build caveat text a model reads, which is why it
    #: is spaced words and not the identifier with underscores swapped out — "hip shift at top"
    #: reads as English, `hip_shift_at_top` reads as a key.
    label: str
    #: The quantity this checkpoint judges: a key in `measure.POSE_MEASUREMENTS`, a `checkpoint`
    #: row in `benchmarks/ranges.json`, and a `Measurement.metric` on a stored swing. One name for
    #: the number across measuring, banding and storing.
    metric: str
    #: True when only overshoot is judged and the band is `[0, high]`, mirroring
    #: `CheckpointScore.one_sided`. False means both edges are asserted and *less is not better* —
    #: the distinction `caveats.py` warns a model about, because a low number on a two-sided
    #: checkpoint is a failure that looks like a strength.
    one_sided: bool


#: Every mechanics checkpoint that ships, in the order they are evaluated and reported.
#:
#: `one_sided` here must agree with what each evaluator puts on its `CheckpointScore`; a test pins
#: that rather than trusting two files to stay in step. Whether a band is one- or two-sided is a
#: measured choice per metric and not a default — see the ADR-010 addendum of 2026-08-12, which
#: sets the rule: assert a band edge only where it clears the instrument.
CHECKPOINT_REGISTRY: tuple[CheckpointSpec, ...] = (
    CheckpointSpec("tempo", "tempo", "tempo_ratio", one_sided=False),
    CheckpointSpec("head_sway", "head sway", "head_sway_norm", one_sided=True),
    CheckpointSpec("finish_balance", "finish balance", "finish_balance_norm", one_sided=True),
    CheckpointSpec("hip_sway", "hip sway", "hip_sway_norm", one_sided=False),
    CheckpointSpec(
        "hip_shift_at_top", "hip shift at top", "hip_shift_at_top_norm", one_sided=True
    ),
    # Signed, and its sign is camera-relative — so it is the one checkpoint that cannot be scored
    # without knowing who swung, and the one whose metric name does not echo its own.
    CheckpointSpec("head_stays_back", "head stays back", "head_hip_gain_norm", one_sided=False),
)


#: The checkpoints a **late top** is known to invalidate, so a bundle whose two views were
#: synchronized on the ball strike can retire them rather than ship a number measured off the wrong
#: frame (`unscored.UnscoredReason.CROSS_VIEW_CONTRADICTED`, M11 P8).
#:
#: **This is a claim about evidence, not about which frames a checkpoint reads**, and the
#: difference is why it is a hand-held set rather than a derivation. `hip_shift_at_top` reads the
#: top too and is the obvious second member — it is deliberately not one, because nothing has yet
#: measured what a ten-frame shift in the top does to a hip position sampled there, and adding it
#: on the strength of the name would be the guess ADR-010 §2 exists to refuse. `tempo` is in
#: because the arbitration restates its number outright: the top is both halves of
#: `tempo_ratio`, and `analysis.alignment._tempo_restated` computes the before and after
#: (2026-08-23 bundle 2 reads 4.92:1 against 2.35:1 on the corrected top).
#:
#: What would earn a second member is a measurement, not an argument. Until then a checkpoint left
#: out of this set still ships its score beside a `SwingAlignment` that says the top was late, so
#: the finding is disclosed either way — what this set decides is only which scores are *withdrawn*.
CONTRADICTED_BY_A_LATE_TOP: frozenset[str] = frozenset({"tempo"})


def checkpoint_names() -> tuple[str, ...]:
    """Registered checkpoint names, in evaluation order."""
    return tuple(spec.name for spec in CHECKPOINT_REGISTRY)


def spec_for(name: str) -> CheckpointSpec:
    """The spec registered under `name`.

    Raises `KeyError` rather than returning `None`: every caller here holds a name that came out of
    the registry in the first place, so a miss is a wiring bug and not a data condition.
    """
    for spec in CHECKPOINT_REGISTRY:
        if spec.name == name:
            return spec
    raise KeyError(f"no checkpoint registered under {name!r}")
