"""Claude writes the coaching, from the numbers this repo actually measured. [M6]

`rules.py` produces a ranked list of tips and a headline, each keyed to one checkpoint. That is
correct and it is also three disconnected sentences: it cannot say *the finish is loose because
the swing is quick*, because no rule spans two checkpoints. This module hands the whole scored
swing to Claude and asks for the paragraph a coach would say.

## The thing this module exists to not do

Everything else here is arranged around one failure: an LLM presents whatever it is handed as
fact. A percentile that clamps at 90, a checkpoint that could not be measured, a shot read off a
photograph, two views interpolated between anchors — each is a number this repo knows to be
provisional, and each becomes a confident lie the moment it is stated without its caveat. So:

  - the brief is **rendered, not dumped**. `build_brief` labels every value with the vocabulary
    `contracts.caveats` warns about (`unscored`, `percentile`, `needs_review`,
    `alignment_caveat`), so a caveat about `unscored` lands next to a line that says `unscored`.
  - the system prompt is `contracts.caveats` **verbatim**, the same text the MCP server shows an
    external client. One source, two consumers (ADR-008 rules out sharing it any other way).
  - the model is told, in the prompt, the closed list of what is measured — and that spine angle,
    hip rotation, swing plane and club path are not. Those are the four a golf model will
    otherwise volunteer from its training data, fluently, about a swing nobody measured them on.
  - the output is stamped with `CoachingProvenance`, so a reader downstream can always tell the
    written sentence from the measured number.

## Structure

Two halves, split on the `llm` extra, the same seam `mcp/query.py` holds against the MCP SDK and
`api/pipeline.py` holds against fastapi:

  - `build_brief` is pure and imports nothing heavy, so the part worth testing — does every
    provisional signal survive into the text? — tests on the base install.
  - `generate_coaching` imports `anthropic` **inside the function**, and never raises for an
    expected failure. No key, no extra, a rate limit, a refusal: each returns an outcome carrying
    the reason, and the pipeline appends it to `notes` and finishes the swing. Coaching is the
    last thing that happens to a result and the least important; it must never be able to cost a
    golfer their score.
"""

from __future__ import annotations

import hashlib
from dataclasses import dataclass
from datetime import UTC, datetime
from typing import Any

from golf_coach.contracts.alignment import AlignmentQuality
from golf_coach.contracts.caveats import (
    ALIGNMENT_CAVEAT,
    READING_THIS_DATA_HONESTLY,
    TWO_AXES,
)
from golf_coach.contracts.feedback import CoachingProvenance
from golf_coach.contracts.shot import ShotData
from golf_coach.contracts.swing import CheckpointScore, SwingBundleResult

#: Big enough that adaptive thinking plus a short paragraph never truncates. Thinking is ON by
#: default on Opus 5 and `max_tokens` bounds thinking *and* response text together, so sizing
#: this to the prose alone (~400 tokens) would cut answers off mid-sentence.
MAX_TOKENS = 4096

#: Coaching is a summarization-shaped task over data already computed — no search, no tools, no
#: multi-step reasoning. `low` is the right rung and keeps the per-swing cost near nothing.
EFFORT = "low"

SYSTEM_PROMPT = f"""\
You are a golf coach reading one swing that has already been measured. Write the short spoken
verdict a coach gives walking back from the bay.

{TWO_AXES}

{READING_THIS_DATA_HONESTLY}

How to write it:

- One paragraph naming the single thing to work on and why, then at most two concrete things to
  try. Nothing else.
- Plain sentences. No headers, no bullet lists, no markdown, no preamble like "Based on the
  data". Write as if speaking.
- The ranked tips in the brief are this system's own verdict. Elaborate on them and connect them
  to each other; do not contradict them or invent a competing diagnosis.
- Never state a measurement that is not in the brief. If you want to reference something that
  was not measured, say it was not measured.
- When the brief flags a number as uncertain, say so in the sentence that uses it rather than
  quoting it flat.
- Address the golfer as "you". Be direct and encouraging; do not pad."""


@dataclass(frozen=True)
class CoachingOutcome:
    """What one coaching attempt produced. `text is None` means it did not happen, and why.

    Mirrors `PipelineOutcome`: an expected failure is a value, not an exception, because the
    caller's job is to record it and carry on rather than to handle it.
    """

    text: str | None = None
    provenance: CoachingProvenance | None = None
    #: Why there is no text, phrased for `SwingBundleResult.notes` — that is, for the golfer
    #: reading the results page, not for a log.
    note: str | None = None


def _fmt(value: float | None, digits: int = 2) -> str:
    """Trim trailing zeros, but only past a decimal point.

    The guard is the whole point: at `digits=0` there is no point to strip back to, so a bare
    `.rstrip("0")` turns a percentile of 90 into 9 and 10 into 1 — a wrong number that looks
    entirely plausible, handed to a model that will state it as fact. Exactly the failure this
    module exists to prevent, produced by the module itself.
    """
    if value is None:
        return "-"
    text = f"{value:.{digits}f}"
    return text.rstrip("0").rstrip(".") if "." in text else text


def _band(checkpoint: CheckpointScore) -> str:
    if checkpoint.expected_low is None and checkpoint.expected_high is None:
        return "no band"
    return f"{_fmt(checkpoint.expected_low)} to {_fmt(checkpoint.expected_high)}"


def _checkpoint_line(checkpoint: CheckpointScore) -> str:
    """One checkpoint, with every qualifier the caveats refer to on the same line.

    `one_sided` is carried because it changes what a percentile *means*: at the 10th percentile a
    one-sided metric is excellent and a two-sided one is as wrong as it is at the 90th.
    """
    verdict = "PASS" if checkpoint.passed else "MISS"
    parts = [
        f"- {checkpoint.name}: observed {_fmt(checkpoint.observed)}, tour band {_band(checkpoint)},"
        f" {verdict}, score {checkpoint.score:.2f}"
    ]
    if checkpoint.percentile is not None:
        sided = "lower is better" if checkpoint.one_sided else "both extremes are faults"
        population = f" of {checkpoint.population_n}" if checkpoint.population_n else ""
        parts.append(
            f"  percentile {_fmt(checkpoint.percentile, 0)}{population} tour swings ({sided})"
        )
    if checkpoint.message:
        parts.append(f"  rule-based reading: {checkpoint.message}")
    return "\n".join(parts)


def _shot_lines(shot: ShotData) -> list[str]:
    """The launch-monitor numbers, with the OCR flag first so it is read before them."""
    lines: list[str] = []
    provenance = shot.provenance
    if provenance is not None:
        flag = "yes" if provenance.needs_review else "no"
        lines.append(
            f"needs_review: {flag} (read off a photo of the {provenance.device} screen by OCR,"
            f" parse confidence {provenance.parse_confidence:.2f})"
        )
        for warning in provenance.warnings:
            lines.append(f"  warning: {warning}")

    fields = [
        ("carry", shot.carry_distance, "yards"),
        ("total", shot.total_distance, "yards"),
        ("ball speed", shot.ball_speed, "mph"),
        ("club head speed", shot.club_head_speed, "mph"),
        ("smash factor", shot.smash_factor, ""),
        ("club path", shot.club_path, "degrees, + = in-to-out"),
        ("club face angle", shot.club_face_angle, "degrees, + = open"),
        ("launch angle", shot.launch_angle, "degrees"),
        ("spin rate", shot.spin_rate, "rpm"),
    ]
    for label, value, unit in fields:
        if value is not None:
            lines.append(f"{label}: {_fmt(value)}{f' {unit}' if unit else ''}")
    for label, text in (("shape", shot.shot_type), ("strike", shot.impact_position)):
        if text:
            lines.append(f"{label}: {text}")
    return lines


def build_brief(result: SwingBundleResult) -> str:
    """Render one analyzed swing as the text the model is asked to coach from.

    Pure — no I/O, no `anthropic` import — which is what lets the interesting assertions (does a
    flagged shot arrive flagged? does `unscored` survive?) run on the base install.

    Deliberately excludes `keypoints`, `detections` and `phases`: several hundred frames of 33
    landmarks each, none of which a coach reasons from, all of which would dominate the prompt.
    """
    swing = result.swing
    out: list[str] = [
        f"SWING {result.swing_id} (session {result.session_id})",
        "",
        f"overall_score: {_fmt(swing.overall_score, 1)} out of 100",
    ]
    if swing.mechanics_score is not None:
        out.append(f"mechanics_score: {_fmt(swing.mechanics_score, 1)} out of 100")
    if swing.outcome_score is None:
        out.append(
            "outcome_score: not computed - the launch-monitor numbers are attached and displayed"
            " but not yet scored, so overall_score reflects mechanics only"
        )

    out += ["", "CHECKPOINTS MEASURED (face-on view only)"]
    if swing.checkpoint_scores:
        out += [_checkpoint_line(c) for c in swing.checkpoint_scores]
    else:
        out.append("- none scored")

    out.append("")
    if swing.unscored:
        out.append(
            "unscored (attempted but could not be measured, and excluded from overall_score"
            f" rather than counted as zero): {', '.join(swing.unscored)}"
        )
    else:
        out.append("unscored: none - every checkpoint was measurable on this swing")

    feedback = result.feedback
    if feedback is not None:
        out += ["", "THIS SYSTEM'S OWN RANKED VERDICT (rule-based, most actionable first)"]
        if feedback.headline:
            out.append(f"headline: {feedback.headline}")
        for index, tip in enumerate(feedback.tips, start=1):
            out.append(f"{index}. [{tip.severity}] {tip.checkpoint}: {tip.text}")

    out += ["", "LAUNCH MONITOR SHOT"]
    if swing.shot is None:
        out.append("none attached to this swing")
    else:
        out += _shot_lines(swing.shot)

    out += ["", "TWO-VIEW ALIGNMENT"]
    alignment = result.alignment
    if alignment is None:
        out.append("no down-the-line view - nothing to align, and nothing scored from it anyway")
    else:
        out.append(f"quality: {alignment.quality.value} ({alignment.quality_summary})")
        if alignment.quality is not AlignmentQuality.FULL:
            out.append(
                "alignment_caveat: "
                + ALIGNMENT_CAVEAT.format(summary=alignment.quality_summary)
            )
        out += [f"  note: {note}" for note in alignment.notes]

    out += ["", "NOTES RECORDED WHILE ANALYZING THIS SWING"]
    out += [f"- {note}" for note in result.notes] or ["none"]

    return "\n".join(out)


def _digest(brief: str) -> str:
    return hashlib.sha256(brief.encode("utf-8")).hexdigest()


def _sdk() -> Any | None:
    """The `anthropic` module, or None when the `llm` extra is not installed.

    The single place this module touches the extra. Importing here rather than at module scope is
    what lets `api/pipeline.py` import this file on a `vision`-only install — the same boundary
    `tests/api/test_pipeline_imports.py` pins for fastapi.
    """
    try:
        import anthropic
    except ImportError:
        return None
    return anthropic


def _text_from(response: object) -> str:
    """Concatenate the text blocks of a Messages response, skipping thinking blocks."""
    blocks = getattr(response, "content", None) or []
    parts = [
        block.text
        for block in blocks
        if getattr(block, "type", None) == "text" and getattr(block, "text", "")
    ]
    return "\n".join(parts).strip()


def generate_coaching(
    result: SwingBundleResult,
    *,
    model: str,
    api_key: str | None = None,
    client: Any | None = None,
) -> CoachingOutcome:
    """Ask Claude for the coaching paragraph. Never raises for an expected failure.

    `client` is the injection seam the tests use — anything exposing `messages.create` works, so
    the request shape is asserted without a network call and without the `llm` extra installed.
    When it is None a real client is constructed, which is the only path that imports `anthropic`.
    """
    brief = build_brief(result)

    sdk = _sdk()
    if client is None:
        if not api_key:
            return CoachingOutcome(
                note=(
                    "no written coaching: no Anthropic API key is configured, so the coaching "
                    "call was skipped (set GOLF_ANTHROPIC_API_KEY). The scores and ranked tips "
                    "above are unaffected."
                )
            )
        if sdk is None:
            return CoachingOutcome(
                note=(
                    "no written coaching: the `llm` extra is not installed "
                    "(pip install -e '.[llm]'). The scores and ranked tips above are unaffected."
                )
            )
        client = sdk.Anthropic(api_key=api_key)

    try:
        response = client.messages.create(
            model=model,
            max_tokens=MAX_TOKENS,
            # Thinking is on by default on Opus 5; saying so explicitly keeps this request honest
            # if the default ever moves. `budget_tokens` is removed on this model and 400s, as do
            # temperature / top_p / top_k — none of them belong here.
            thinking={"type": "adaptive"},
            output_config={"effort": EFFORT},
            system=[
                {
                    "type": "text",
                    "text": SYSTEM_PROMPT,
                    # The prompt is identical for every swing, so a session's second swing reads
                    # it from cache. Worth a check against `usage.cache_read_input_tokens` — Opus
                    # 5 will not cache a prefix under 512 tokens, and silently writes nothing.
                    "cache_control": {"type": "ephemeral"},
                }
            ],
            messages=[{"role": "user", "content": brief}],
        )
    except Exception as exc:  # narrowed in `_note_for`; the SDK may be absent entirely
        return CoachingOutcome(note=_note_for(exc, sdk))

    stop_reason = getattr(response, "stop_reason", None)
    if stop_reason == "refusal":
        return CoachingOutcome(
            note="no written coaching: the model declined to answer for this swing."
        )

    text = _text_from(response)
    if not text:
        return CoachingOutcome(
            note="no written coaching: the model returned no text for this swing."
        )
    if stop_reason == "max_tokens":
        # Say it rather than silently serving a paragraph that stops mid-sentence.
        text += "\n\n(This note was cut off before it finished.)"

    return CoachingOutcome(
        text=text,
        provenance=CoachingProvenance(
            model=getattr(response, "model", None) or model,
            generated_at=datetime.now(UTC),
            input_digest=_digest(brief),
        ),
    )


def _note_for(exc: Exception, anthropic: Any | None) -> str:
    """Turn an SDK exception into the sentence a golfer should read on the results page.

    Most specific first, per the SDK's own guidance — a 404 on the model id and a rate limit want
    different words, and collapsing both into "an error occurred" is how a stale `coaching_model`
    goes unnoticed for a month.
    """
    prefix = "no written coaching: "
    if anthropic is not None:
        if isinstance(exc, anthropic.NotFoundError):  # type: ignore[attr-defined]
            return (
                f"{prefix}the API rejected the configured model id. Check "
                f"`coaching_model` in config.py ({exc})."
            )
        if isinstance(exc, anthropic.AuthenticationError):  # type: ignore[attr-defined]
            return f"{prefix}the configured Anthropic API key was rejected."
        if isinstance(exc, anthropic.RateLimitError):  # type: ignore[attr-defined]
            return f"{prefix}the coaching request was rate limited. Re-run to try again."
        if isinstance(exc, anthropic.APIStatusError):  # type: ignore[attr-defined]
            return f"{prefix}the API returned {exc.status_code}."  # type: ignore[attr-defined]
        if isinstance(exc, anthropic.APIConnectionError):  # type: ignore[attr-defined]
            return f"{prefix}could not reach the API. The scores above are unaffected."
    return f"{prefix}the coaching request failed ({type(exc).__name__}: {exc})."
