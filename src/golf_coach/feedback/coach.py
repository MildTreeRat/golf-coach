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

## Two routes to the brief [M29 P17]

  - **In process**: `build_brief` over frozen Python's `SwingBundleResult`, with the system prompt
    over frozen `contracts/caveats.py`. `api/pipeline.py` keeps this route until M40 deletes it.
  - **Over stdio**: `python -m golf_coach.feedback.coach --session S --swing W` reads the swing
    through the Rust server, `golf-mcp`, and prints one JSON object, `{text, provenance}` or
    `{note}`, which is what `golf-lab analyze` folds into `analysis.json` (the M29 plan's D15).
    `brief_from_view` renders `get_swing`'s answer as `build_brief` renders the result, and the two
    caveat blocks the prompt quotes come from `golf-mcp`'s caveats resource, so **this route keeps
    no copy of a caveat sentence and reads no pydantic shape** (ADR-035 clause 1 names
    `contracts/keypoints.py` as the only one of the mirrored contract to survive M40).
    `scripts/ask_swing.py` seeds its conversations from the same rendering.

`brief_from_view`'s docstring lists every line the view renders differently, and why none of them
is a loss. Two lines needed keys no view carried, and `golf-mcp` gained them: a checkpoint's
`one_sided` and an unscored entry's `family` (`crates/mcp/src/query.rs`'s module doc).

The stdio pieces `conversation.py`'s route shares (`GolfMcpServer`, the caveats resource, the read
timeout) live here, beside `_sdk` and `_note_for`, for the reason those already do:
`conversation.py` imports this module, and the reverse would be a cycle.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import sys
from collections.abc import Mapping
from dataclasses import dataclass, field
from datetime import UTC, datetime
from pathlib import Path
from typing import Any, TextIO

from golf_coach.contracts.alignment import AlignmentQuality
from golf_coach.contracts.caveats import (
    ALIGNMENT_CAVEAT,
    READING_THIS_DATA_HONESTLY,
    TWO_AXES,
)
from golf_coach.contracts.feedback import CoachingProvenance
from golf_coach.contracts.pivots import PIVOTS_BY_NAME
from golf_coach.contracts.placements import PLACEMENTS_BY_NAME
from golf_coach.contracts.shot import ShotData
from golf_coach.contracts.swing import CheckpointScore, Measurement, SwingBundleResult
from golf_coach.contracts.unscored import INFERENCE_REASONS

#: Big enough that adaptive thinking plus a short paragraph never truncates. Thinking is ON by
#: default on Opus 5 and `max_tokens` bounds thinking *and* response text together, so sizing
#: this to the prose alone (~400 tokens) would cut answers off mid-sentence.
MAX_TOKENS = 4096

#: Coaching is a summarization-shaped task over data already computed — no search, no tools, no
#: multi-step reasoning. `low` is the right rung and keeps the per-swing cost near nothing.
EFFORT = "low"


def coaching_system_prompt(two_axes: str, reading_this_data_honestly: str) -> str:
    """The coaching instructions, around the two caveat blocks they quote. [M29 P17]

    A function of the two blocks rather than a constant over an import, for the reason
    `conversation.answering_followups` is one: the two routes take them from different places, the
    in-process route from frozen `contracts/caveats.py` and the stdio route from `golf-mcp`'s
    caveats resource.
    """
    return f"""\
You are a golf coach reading one swing that has already been measured. Write the short spoken
verdict a coach gives walking back from the bay.

{two_axes}

{reading_this_data_honestly}

How to write it:

- One paragraph naming the single thing to work on and why, then at most two concrete things to
  try. Nothing else.
- Plain sentences. No headers, no bullet lists, no markdown, no preamble like "Based on the
  data". Write as if speaking.
- The ranked tips in the brief are this system's own verdict. Elaborate on them and connect them
  to each other; do not contradict them or invent a competing diagnosis.
- Never state a measurement that is not in the brief. If you want to reference something that
  was not measured, say it was not measured.
- The population placement is context, not a fault, and it is never the headline. Bring it in only
  when it sharpens the thing you are already naming — a metric that passes its own band while
  driving the swing's distance from the tour population is worth a clause, because no band can
  say that. An unusual swing that works is a style, not a problem.
- When the brief flags a number as uncertain, say so in the sentence that uses it rather than
  quoting it flat.
- Address the golfer as "you". Be direct and encouraging; do not pad."""


#: The in-process route's prompt, over frozen Python's copies of the two blocks.
SYSTEM_PROMPT = coaching_system_prompt(TWO_AXES, READING_THIS_DATA_HONESTLY)

#: What to run when `golf-mcp` is not there. Release, because that is the build `default_golf_mcp`
#: looks for: a debug server answers the same, only slower.
BUILD_GOLF_MCP = "cargo build --release -p golf-mcp"

#: `golf-mcp`'s caveats resource (`crates/mcp/src/server.rs`'s `CAVEATS_URI`): one JSON object of
#: every named caveat sentence, keyed by the Python constant's name.
CAVEATS_URI = "golf-coach://caveats"

#: How long one request to `golf-mcp` may take before it is given up on. A hung server must become
#: a note rather than a hung terminal; sixty seconds is the Rust stdio test's own bound
#: (`crates/mcp/tests/stdio.rs`). Measured over the real `data/` with the release build
#: (2026-10-22): spawn and handshake 18 ms, and the slowest of the twelve, `get_golfer_profile`,
#: 74 ms. So the bound is for a server that has stopped answering, never for a slow one.
READ_TIMEOUT_SECONDS = 60.0

#: The two caveat blocks the coaching prompt quotes, by their name in the caveats resource.
PROMPT_CAVEATS = ("TWO_AXES", "READING_THIS_DATA_HONESTLY")


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


def _placement_lines(measurements: list[Measurement]) -> list[str]:
    """The population placements, each labelled with whether it may be read on its own.

    The five `population:golfdb` entries were recorded by M8 and rendered by nothing — this brief
    did not include `measurements` at all, so the one thing these models can say that the six bands
    cannot never reached the golfer. That is what they are here for.

    `calibrated` is stamped on every line rather than left to the caveat block, for the reason
    `_checkpoint_line` carries `one_sided`: a warning three hundred words above a number is a
    warning a model applies to the numbers it remembers. The uncalibrated ones are the residuals,
    and on the stored corpus the largest of them is a mis-detected anchor rather than an unusual
    swing — a value this brief would otherwise present as the swing's most striking feature.

    Only placements are rendered. The rest of `measurements` is pose metrics the checkpoints above
    already judged and shot fields `_shot_lines` already prints, so including them would repeat
    every number in the brief under a second name.
    """
    lines: list[str] = []
    for measurement in measurements:
        spec = PLACEMENTS_BY_NAME.get(measurement.name)
        if spec is None:
            continue
        calibration = (
            "calibrated" if spec.calibrated else "NOT calibrated - read beside its T2, never alone"
        )
        lines.append(
            f"- {measurement.name} ({spec.view} view, {calibration}): "
            f"{_fmt(measurement.value)} {measurement.unit}"
        )
        lines.append(f"  {measurement.detail}")
    return lines


def _pivot_lines(measurements: list[Measurement]) -> list[str]:
    """The rotation measurements, each labelled with its view and why it is interim.

    `_placement_lines`'s sibling and one step short of it: a placement is at least a distance from
    a population that exists, where a pivot row has no reference population behind it at all — only
    the fact that a calibrated source would replace it. Rendering `spec.interim_reason` on every
    line rather than leaving it to the caveat block is the same call `_placement_lines` makes about
    `calibrated`: a warning stated once, far above the numbers, is a warning a model applies to the
    numbers it still remembers.

    Only pivot rows are rendered, for `_placement_lines`'s reason: the rest of `measurements` is
    pose metrics the checkpoints already judged or shot fields `_shot_lines` already prints.
    """
    lines: list[str] = []
    for measurement in measurements:
        spec = PIVOTS_BY_NAME.get(measurement.name)
        if spec is None:
            continue
        lines.append(
            f"- {measurement.name} ({spec.view} view, interim): "
            f"{_fmt(measurement.value)} {measurement.unit}"
        )
        lines.append(f"  {spec.interim_reason}")
    return lines


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

    Of `measurements` it renders the population placements and the rotation readings only — see
    `_placement_lines` and `_pivot_lines` for why the rest would be every number in this brief a
    second time.
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
    # Split by family [M15 P11]. Both halves are absences named in the same list, and the heading
    # below is true of only one of them: a checkpoint that could not be scored *was* going to be in
    # `overall_score`, and a `flight_*` measurement never was. Telling a coaching model that a
    # simulated carry was "excluded from overall_score" invites it to explain a score that never
    # moved. `contracts.unscored.INFERENCE_REASONS` is what tells them apart.
    unscored = [entry for entry in swing.unscored if entry.reason not in INFERENCE_REASONS]
    flight = [entry for entry in swing.unscored if entry.reason in INFERENCE_REASONS]
    if unscored:
        # The reason goes in the brief, not just the name. A model told only that tempo is missing
        # will reach for the likeliest explanation and state it — and "your camera moved" is a
        # confident wrong answer to give a golfer whose clip was fine and whose band simply does
        # not exist. Naming the cause is what makes the caveat about `unscored` actionable rather
        # than merely honest.
        out.append(
            "unscored (attempted but could not be scored, and excluded from overall_score"
            " rather than counted as zero):"
        )
        out += [f"- {entry.name}: {entry.spec.summary}" for entry in unscored]
    else:
        out.append("unscored: none - every checkpoint was scored on this swing")

    if flight:
        out.append("")
        out.append(
            "simulated ball flight (a model over the printed launch conditions, in no score at"
            " all - not a checkpoint and never part of overall_score):"
        )
        out += [f"- {entry.name}: {entry.spec.summary}" for entry in flight]

    placements = _placement_lines(swing.measurements)
    out += ["", "POPULATION PLACEMENT (recorded, never scored - nothing here has a band)"]
    if placements:
        out += placements
    else:
        out.append(
            "none - this swing did not produce a placement, which happens when a metric or a "
            "phase anchor the model needs was missing. Say nothing about where it sits."
        )

    pivots = _pivot_lines(swing.measurements)
    out += [
        "",
        "ROTATION (interim - a 2-D-per-view reading, no reference population, never a band, "
        "never part of overall_score, never blended across views)",
    ]
    if pivots:
        out += pivots
    else:
        out.append(
            "none - this swing did not produce a rotation reading, which happens when the "
            "tracking a check needs was missing. Say nothing about the golfer's turn."
        )

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

    client, refused = _coaching_client(api_key, client)
    if refused is not None:
        return CoachingOutcome(note=refused)

    written = _write(client, brief, SYSTEM_PROMPT, model)
    if written.text is None:
        return CoachingOutcome(note=written.note)
    return CoachingOutcome(
        text=written.text,
        provenance=CoachingProvenance(
            model=written.model,
            generated_at=datetime.now(UTC),
            input_digest=_digest(brief),
        ),
    )


def _coaching_client(api_key: str | None, client: Any | None) -> tuple[Any, str | None]:
    """The client the coaching request goes out on, or the note saying why it cannot go out.

    Shared by both routes, so the stdio route refuses before it starts `golf-mcp`: a paragraph with
    no extra or no key is not worth spawning a server for, and both routes say so in one wording.
    """
    if client is not None:
        return client, None
    sdk = _sdk()
    # The extra is checked before the key, because it is the more fundamental blocker and only one
    # of the two can be reported. With no `anthropic` package there is nothing to authenticate
    # against, so answering that install with "no API key is configured" sends the reader to
    # `.env` — where they find a perfectly good key — instead of to pip.
    if sdk is None:
        return None, (
            "no written coaching: the `llm` extra is not installed "
            "(pip install -e '.[llm]'). The scores and ranked tips above are unaffected."
        )
    if not api_key:
        return None, (
            "no written coaching: no Anthropic API key is configured, so the coaching "
            "call was skipped (set GOLF_ANTHROPIC_API_KEY). The scores and ranked tips "
            "above are unaffected."
        )
    return sdk.Anthropic(api_key=api_key), None


@dataclass(frozen=True)
class _Written:
    """What one coaching request answered: the text and the model that wrote it, or the note."""

    text: str | None = None
    model: str = ""
    note: str | None = None


def _write(client: Any, brief: str, system: str, model: str) -> _Written:
    """One coaching request, from a brief and a system prompt. Never raises for an expected failure.

    Both routes end here, so the request shape the tests pin and the notes a failure becomes are
    spelled once, whichever route built the brief.
    """
    sdk = _sdk()
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
                    "text": system,
                    # The prompt is identical for every swing, so a session's second swing reads
                    # it from cache. Worth a check against `usage.cache_read_input_tokens` — Opus
                    # 5 will not cache a prefix under 512 tokens, and silently writes nothing.
                    "cache_control": {"type": "ephemeral"},
                }
            ],
            messages=[{"role": "user", "content": brief}],
        )
    except Exception as exc:  # narrowed in `_note_for`; the SDK may be absent entirely
        return _Written(note=_note_for(exc, sdk))

    stop_reason = getattr(response, "stop_reason", None)
    if stop_reason == "refusal":
        return _Written(note="no written coaching: the model declined to answer for this swing.")

    text = _text_from(response)
    if not text:
        return _Written(note="no written coaching: the model returned no text for this swing.")
    if stop_reason == "max_tokens":
        # Say it rather than silently serving a paragraph that stops mid-sentence.
        text += "\n\n(This note was cut off before it finished.)"
    return _Written(text=text, model=getattr(response, "model", None) or model)


#: Long enough for the API's own sentence, short enough that a note stays a note. What this clips
#: is the rare message that embeds a request echo; the actionable ones are a single line.
_MAX_DETAIL = 200


def _api_detail(exc: Exception) -> str:
    """The API's own sentence about a failure, or "" when there is nothing worth printing.

    `APIStatusError.message` is assembled by the SDK as ``Error code: 400 - {…whole body dict…}``,
    which is accurate and unreadable. The parsed body carries the same sentence on its own, so
    prefer that and fall back to `message` only when the shape is unfamiliar. Everything is read
    through `getattr`: the tests build exceptions via `__new__` to skip the response plumbing, and
    a diagnostic that raises while explaining a failure is worse than the failure.
    """
    body = getattr(exc, "body", None)
    if isinstance(body, dict):
        error = body.get("error")
        if isinstance(error, dict) and isinstance(error.get("message"), str):
            detail: str = error["message"].strip()
            if detail:
                return detail[:_MAX_DETAIL]
    message = getattr(exc, "message", None)
    if isinstance(message, str) and message.strip():
        return message.strip()[:_MAX_DETAIL]
    return ""


def _note_for(
    exc: Exception, anthropic: Any | None, *, prefix: str = "no written coaching: "
) -> str:
    """Turn an SDK exception into the sentence a golfer should read on the results page.

    Most specific first, per the SDK's own guidance — a 404 on the model id and a rate limit want
    different words, and collapsing both into "an error occurred" is how a stale `coaching_model`
    goes unnoticed for a month.

    `prefix` is what lets `conversation.py` share this. The failures are identical; the vocabulary
    around them is not — the results page says "no written coaching", the Ask panel says "no
    answer". It was hardcoded to the coaching wording once, and conversation.py had no way to say
    otherwise, so a rate-limited *question* answered itself with "no written coaching:" directly
    underneath the question the golfer had just typed.
    """
    if anthropic is not None:
        if isinstance(exc, anthropic.NotFoundError):  # type: ignore[attr-defined]
            return (
                f"{prefix}the API rejected the configured model id. Check "
                f"`coaching_model` in config.py ({exc})."
            )
        if isinstance(exc, anthropic.AuthenticationError):  # type: ignore[attr-defined]
            return f"{prefix}the configured Anthropic API key was rejected."
        if isinstance(exc, anthropic.RateLimitError):  # type: ignore[attr-defined]
            return f"{prefix}the request was rate limited. Re-run to try again."
        if isinstance(exc, anthropic.OverloadedError):  # type: ignore[attr-defined]
            # 529 is the API being busy, and says nothing about this request. Named apart from the
            # status branch below so it reads as "wait", which is the whole of the fix.
            return f"{prefix}the API is overloaded right now. Re-run to try again."
        if isinstance(exc, anthropic.APIStatusError):  # type: ignore[attr-defined]
            status = getattr(exc, "status_code", None)
            code = str(status) if status is not None else "an error"
            # The status alone is not actionable, and 400 is the case that proves it: a malformed
            # request and an exhausted credit balance arrive as the same number, so "the API
            # returned 400." sends you to read request-building code when the answer was billing.
            # The body says which, and keeping it is the difference between a dead end and a fix.
            detail = _api_detail(exc)
            if detail:
                return f"{prefix}the API returned {code}: {detail}"
            return f"{prefix}the API returned {code}."
        if isinstance(exc, anthropic.APIConnectionError):  # type: ignore[attr-defined]
            return f"{prefix}could not reach the API. The scores above are unaffected."
    return f"{prefix}the request failed ({type(exc).__name__}: {exc})."


# --------------------------------------------------------------------------------------------------
# The stdio route [M29 P17]: the brief from `golf-mcp`'s view, and the JSON entry
# --------------------------------------------------------------------------------------------------

#: `UnscoredView.family`'s value for a ball-flight number the simulation could not produce
#: (`crates/mcp/src/query.rs`'s `FLIGHT_FAMILY`). Everything else is a checkpoint.
FLIGHT_FAMILY = "flight"

#: The numeric shot metrics the brief prints, in `_shot_lines`' order and with its labels, keyed by
#: the name `ShotView.metrics` answers them under.
_VIEW_SHOT_FIELDS = (
    ("carry", "carry_distance", "yards"),
    ("total", "total_distance", "yards"),
    ("ball speed", "ball_speed", "mph"),
    ("club head speed", "club_head_speed", "mph"),
    ("smash factor", "smash_factor", ""),
    ("club path", "club_path", "degrees, + = in-to-out"),
    ("club face angle", "club_face_angle", "degrees, + = open"),
    ("launch angle", "launch_angle", "degrees"),
    ("spin rate", "spin_rate", "rpm"),
)

#: The keys of `get_swing`'s view an analysis fills. A view with none of them is the bare one
#: `golf-mcp` answers for a swing with no `analysis.json` (`SwingView::bare`): its identity, its
#: status and what the manifest stamps, which is nothing to coach from.
_ANALYSIS_KEYS = (
    "overall_score",
    "mechanics_score",
    "outcome_score",
    "headline",
    "checkpoints",
    "tips",
    "unscored",
    "measurements",
    "simulated",
    "population",
    "rotation",
    "analysis_version",
    "alignment_quality",
    "alignment_caveat",
    "shot",
    "notes",
)


def is_analyzed(view: Mapping[str, Any]) -> bool:
    """Whether `get_swing`'s view holds an analysis, rather than only the swing's identity."""
    return any(view.get(key) not in (None, [], {}) for key in _ANALYSIS_KEYS)


def _items(value: Any) -> list[Mapping[str, Any]]:
    """The objects in a list the view answers, and nothing else."""
    return [item for item in value if isinstance(item, Mapping)] if isinstance(value, list) else []


def _number(value: Any) -> float | None:
    if isinstance(value, bool) or not isinstance(value, int | float):
        return None
    return float(value)


def _view_band(checkpoint: Mapping[str, Any]) -> str:
    low, high = _number(checkpoint.get("expected_low")), _number(checkpoint.get("expected_high"))
    if low is None and high is None:
        return "no band"
    return f"{_fmt(low)} to {_fmt(high)}"


def _view_checkpoint_line(checkpoint: Mapping[str, Any]) -> str:
    """`_checkpoint_line` over a `CheckpointView`."""
    verdict = "PASS" if checkpoint.get("passed") else "MISS"
    score = _number(checkpoint.get("score"))
    parts = [
        f"- {checkpoint.get('name')}: observed {_fmt(_number(checkpoint.get('observed')))}, "
        f"tour band {_view_band(checkpoint)}, {verdict}, "
        f"score {'-' if score is None else f'{score:.2f}'}"
    ]
    percentile = _number(checkpoint.get("percentile"))
    if percentile is not None:
        one_sided = checkpoint.get("one_sided")
        sided = (
            ""
            if not isinstance(one_sided, bool)
            else " (lower is better)"
            if one_sided
            else " (both extremes are faults)"
        )
        population_n = checkpoint.get("population_n")
        population = f" of {population_n}" if population_n else ""
        parts.append(f"  percentile {_fmt(percentile, 0)}{population} tour swings{sided}")
    if checkpoint.get("message"):
        parts.append(f"  rule-based reading: {checkpoint['message']}")
    return "\n".join(parts)


def _view_shot_lines(shot: Mapping[str, Any]) -> list[str]:
    """`_shot_lines` over a `ShotView`: the OCR flag first, so it is read before the numbers.

    `parse_confidence` stands for the provenance block: it is required on one, and the view answers
    `None` exactly when the shot has none.
    """
    lines: list[str] = []
    confidence = _number(shot.get("parse_confidence"))
    if confidence is not None:
        flag = "yes" if shot.get("needs_review") else "no"
        lines.append(
            f"needs_review: {flag} (read off a photo of the launch monitor's screen by OCR,"
            f" parse confidence {confidence:.2f})"
        )
        warnings = shot.get("warnings")
        for warning in warnings if isinstance(warnings, list) else []:
            lines.append(f"  warning: {warning}")

    metrics = shot.get("metrics")
    metrics = metrics if isinstance(metrics, Mapping) else {}
    for label, key, unit in _VIEW_SHOT_FIELDS:
        value = _number(metrics.get(key))
        if value is not None:
            lines.append(f"{label}: {_fmt(value)}{f' {unit}' if unit else ''}")
    for label, key in (("shape", "shot_type"), ("strike", "impact_position")):
        text = metrics.get(key)
        if isinstance(text, str) and text:
            lines.append(f"{label}: {text}")
    return lines


def _require_p17_keys(view: Mapping[str, Any]) -> None:
    """Refuse a view from a `golf-mcp` built before it answered `one_sided` and `family`.

    A missing key is not a missing value: the server answers `one_sided: null` for a score that
    does not say, and `family` on every unscored entry. Without them the brief would lose the
    percentile's direction and call every flight refusal a checkpoint excluded from
    `overall_score`, silently, so a stale release build is a refusal naming the rebuild instead.
    """
    for key, items in (("one_sided", "checkpoints"), ("family", "unscored")):
        if any(key not in item for item in _items(view.get(items))):
            raise ValueError(
                f"golf-mcp's get_swing answer has no `{key}` in its `{items}`, so the server "
                f"predates the coaching brief's keys: rebuild it (`{BUILD_GOLF_MCP}`)"
            )


def brief_from_view(view: Mapping[str, Any]) -> str:
    """`build_brief`, over `get_swing`'s answer from `golf-mcp` rather than frozen Python's result.

    Line for line `build_brief`'s rendering, so its docstring's calls hold here too: the heavy
    streams are absent because the view never had them, and of the measurements only the
    placements and the rotation rows are rendered, which the view already carries apart as
    `population` and `rotation`. Raises `ValueError` for a view from a server that predates the two
    keys this needs (`_require_p17_keys`).

    **Where it renders differently, and why nothing is lost** (the M29 plan's P17 measured each over
    the 15 stored swings):

      - **The shot's device is not named**: the line says "the launch monitor's screen". The view
        carries the shot's `source`, not its provenance's `device`, whose value (`hd_golf`) is a
        storage key rather than anything a coach says.
      - **The shot's warnings are the golfer-facing ones**: `golf-mcp` drops M34's `no tile found
        for` layout line (the plan's P11), which is about the screen's layout and not the photo.
      - **The alignment's summary and notes are not repeated** under TWO-VIEW ALIGNMENT. The
        summary is inside `alignment_caveat` on every tier below `full`, and both engines copy every
        alignment note into the swing's notes as `alignment: …`, so NOTES carries each one once.
      - **The verdict section appears when there is a headline or a tip.** The view cannot tell a
        result with no feedback from one with an empty one, and the pipeline writes feedback on
        every swing it analyses.
      - **A percentile whose score does not say which side is the fault** is printed without the
        direction rather than with a guessed one. No engine writes a score like that.
      - **An unscored reason the server does not know** reads as `unrecorded`, as `golf-mcp`'s
        tolerant reader answers it, where `build_brief` could not have read the artifact at all.
    """
    _require_p17_keys(view)
    overall = _number(view.get("overall_score"))
    out: list[str] = [
        f"SWING {view.get('swing_id')} (session {view.get('session_id')})",
        "",
        f"overall_score: {_fmt(overall, 1)} out of 100",
    ]
    mechanics = _number(view.get("mechanics_score"))
    if mechanics is not None:
        out.append(f"mechanics_score: {_fmt(mechanics, 1)} out of 100")
    if view.get("outcome_score") is None:
        out.append(
            "outcome_score: not computed - the launch-monitor numbers are attached and displayed"
            " but not yet scored, so overall_score reflects mechanics only"
        )

    out += ["", "CHECKPOINTS MEASURED (face-on view only)"]
    out += [_view_checkpoint_line(c) for c in _items(view.get("checkpoints"))] or ["- none scored"]

    out.append("")
    # Split by family, for `build_brief`'s reason, with the family read off the view rather than
    # off a copy of the reason table: `golf-mcp` answers it from `is_inference_reason`.
    entries = _items(view.get("unscored"))
    unscored = [entry for entry in entries if entry.get("family") != FLIGHT_FAMILY]
    flight = [entry for entry in entries if entry.get("family") == FLIGHT_FAMILY]
    if unscored:
        out.append(
            "unscored (attempted but could not be scored, and excluded from overall_score"
            " rather than counted as zero):"
        )
        out += [f"- {entry.get('name')}: {entry.get('why')}" for entry in unscored]
    else:
        out.append("unscored: none - every checkpoint was scored on this swing")

    if flight:
        out.append("")
        out.append(
            "simulated ball flight (a model over the printed launch conditions, in no score at"
            " all - not a checkpoint and never part of overall_score):"
        )
        out += [f"- {entry.get('name')}: {entry.get('why')}" for entry in flight]

    out += ["", "POPULATION PLACEMENT (recorded, never scored - nothing here has a band)"]
    placements = _items(view.get("population"))
    for row in placements:
        calibration = (
            "calibrated"
            if row.get("calibrated")
            else "NOT calibrated - read beside its T2, never alone"
        )
        out.append(
            f"- {row.get('name')} ({row.get('view')} view, {calibration}): "
            f"{_fmt(_number(row.get('value')))} {row.get('unit')}"
        )
        out.append(f"  {row.get('detail')}")
    if not placements:
        out.append(
            "none - this swing did not produce a placement, which happens when a metric or a "
            "phase anchor the model needs was missing. Say nothing about where it sits."
        )

    out += [
        "",
        "ROTATION (interim - a 2-D-per-view reading, no reference population, never a band, "
        "never part of overall_score, never blended across views)",
    ]
    pivots = _items(view.get("rotation"))
    for row in pivots:
        out.append(
            f"- {row.get('name')} ({row.get('view')} view, interim): "
            f"{_fmt(_number(row.get('value')))} {row.get('unit')}"
        )
        out.append(f"  {row.get('interim_reason')}")
    if not pivots:
        out.append(
            "none - this swing did not produce a rotation reading, which happens when the "
            "tracking a check needs was missing. Say nothing about the golfer's turn."
        )

    headline, tips = view.get("headline"), _items(view.get("tips"))
    if headline or tips:
        out += ["", "THIS SYSTEM'S OWN RANKED VERDICT (rule-based, most actionable first)"]
        if headline:
            out.append(f"headline: {headline}")
        for index, tip in enumerate(tips, start=1):
            out.append(
                f"{index}. [{tip.get('severity')}] {tip.get('checkpoint')}: {tip.get('text')}"
            )

    out += ["", "LAUNCH MONITOR SHOT"]
    shot = view.get("shot")
    if isinstance(shot, Mapping):
        out += _view_shot_lines(shot)
    else:
        out.append("none attached to this swing")

    out += ["", "TWO-VIEW ALIGNMENT"]
    quality = view.get("alignment_quality")
    if quality is None:
        out.append("no down-the-line view - nothing to align, and nothing scored from it anyway")
    else:
        out.append(f"quality: {quality}")
        if view.get("alignment_caveat"):
            out.append(f"alignment_caveat: {view['alignment_caveat']}")

    out += ["", "NOTES RECORDED WHILE ANALYZING THIS SWING"]
    notes = view.get("notes")
    out += [f"- {note}" for note in (notes if isinstance(notes, list) else [])] or ["none"]

    return "\n".join(out)


@dataclass(frozen=True)
class GolfMcpServer:
    """How to start `golf-mcp`: the binary, and the stores it serves.

    The three directories are `golf-mcp`'s own flags (the M29 plan's C10), passed always rather
    than left to its defaults, so the server reads exactly the stores the caller names.
    `golfers_dir=None` is `--no-registry`: the server then offers no per-golfer tool and its
    briefing drops their reading rules, which is `build_tools(golfers_dir=None)`'s contract on the
    in-process route.
    """

    binary: Path
    sessions_dir: Path
    shots_dir: Path
    golfers_dir: Path | None = None

    def args(self) -> list[str]:
        registry = (
            ["--golfers-dir", str(self.golfers_dir)]
            if self.golfers_dir is not None
            else ["--no-registry"]
        )
        return [
            "--sessions-dir",
            str(self.sessions_dir),
            "--shots-dir",
            str(self.shots_dir),
            *registry,
        ]


def default_golf_mcp() -> Path:
    """`GOLF_MCP`, else the release build: the order `golf-core rerecord` finds the binary in.

    Release, because it is the build Claude Code's registration points at from the M29 plan's
    P34 (D21). A debug build answers the same, only slower.
    """
    from golf_coach.config import REPO_ROOT

    named = os.environ.get("GOLF_MCP")
    if named:
        return Path(named)
    exe = "golf-mcp.exe" if os.name == "nt" else "golf-mcp"
    return REPO_ROOT / "target" / "release" / exe


@dataclass(frozen=True)
class SwingRead:
    """What `golf-mcp` answered about one swing: its view, or the miss sentence; and the caveats."""

    view: dict[str, Any] | None
    miss: str | None
    caveats: dict[str, str]


def read_swing(
    server: GolfMcpServer,
    session_id: str,
    swing_id: str,
    *,
    caveats: tuple[str, ...] = (),
    errlog: TextIO | None = None,
) -> SwingRead:
    """`get_swing`, and the caveat sentences named, over one stdio session to `golf-mcp`.

    Raises when the server cannot be used: the MCP client not installed (`ImportError`), the binary
    not found, a spawn or handshake that fails, or an answer that is no view. Each caller turns
    that into a note in its own vocabulary. A miss is an answer, not a failure: `view` is `None`
    and `miss` is the server's own sentence saying what to do. `errlog` takes the server's stderr,
    `sys.stderr` when not given.
    """
    import anyio
    from mcp import ClientSession, StdioServerParameters
    from mcp.client.stdio import stdio_client

    binary = shutil.which(str(server.binary))
    if binary is None:
        raise FileNotFoundError(f"golf-mcp is not built at {server.binary}")
    read: list[SwingRead] = []

    async def converse() -> None:
        parameters = StdioServerParameters(command=binary, args=server.args())
        async with (
            stdio_client(parameters, errlog=errlog or sys.stderr) as (stream_in, stream_out),
            ClientSession(
                stream_in, stream_out, read_timeout_seconds=READ_TIMEOUT_SECONDS
            ) as session,
        ):
            await session.initialize()
            sentences: dict[str, str] = {}
            if caveats:
                resource = await session.read_resource(CAVEATS_URI)
                sentences = {name: _caveat(resource, name) for name in caveats}
            answer = await session.call_tool(
                "get_swing", {"session_id": session_id, "swing_id": swing_id}
            )
            read.append(_swing_read(answer, sentences))

    anyio.run(converse)
    return read[0]


def _swing_read(answer: Any, caveats: dict[str, str]) -> SwingRead:
    """`get_swing`'s `CallToolResult` as a view or a miss; anything else is a failure."""
    structured = getattr(answer, "structured_content", None)
    result = structured.get("result") if isinstance(structured, dict) else None
    if getattr(answer, "is_error", False) or not isinstance(result, dict):
        blocks = getattr(answer, "content", None) or []
        said = getattr(blocks[0], "text", "") if blocks else ""
        raise ValueError(f"golf-mcp answered get_swing with no view: {said or 'nothing'}")
    if result.get("found") is False:
        return SwingRead(view=None, miss=str(result.get("message", "")), caveats=caveats)
    return SwingRead(view=result, miss=None, caveats=caveats)


def _caveat(result: Any, name: str) -> str:
    """One named sentence from `golf-mcp`'s caveats resource (`resources/read`'s answer).

    Indexed by name, as the resource is keyed (the M29 plan's P9 finding 6), so a sentence added
    to the resource changes nothing here and a sentence removed from it is a refusal by name.
    """
    contents = getattr(result, "contents", None) or []
    text = getattr(contents[0], "text", None) if contents else None
    if not isinstance(text, str):
        raise ValueError(f"golf-mcp's {CAVEATS_URI} is not text")
    caveats = json.loads(text)
    sentence = caveats.get(name) if isinstance(caveats, dict) else None
    if not isinstance(sentence, str):
        raise ValueError(f"golf-mcp's {CAVEATS_URI} has no {name}")
    return sentence


def _innermost(exc: BaseException) -> str:
    """The failure worth printing, out of the exception groups anyio's task groups wrap it in.

    A server that exits during the handshake surfaces as an `ExceptionGroup` around the one error
    that says why, and "unhandled errors in a TaskGroup" is not a sentence anybody can act on.
    """
    while isinstance(exc, BaseExceptionGroup) and exc.exceptions:
        exc = exc.exceptions[0]
    return f"{type(exc).__name__}: {exc}"


def _spelled(instant: datetime) -> str:
    """An aware UTC instant as pydantic writes it into `analysis.json`, with no pydantic.

    `2026-10-22T08:23:05.300000Z`, and no fraction at all on a whole second, which is what
    `CoachingProvenance.model_dump(mode="json")` writes and `crates/contracts`' `Timestamp` reads
    (measured, pydantic 2.13.4, and pinned in `tests/feedback/test_coach.py`).
    """
    return instant.astimezone(UTC).isoformat().replace("+00:00", "Z")


@dataclass(frozen=True)
class Brief:
    """One stored swing's brief as `golf-mcp` serves it, or the sentence saying why not."""

    text: str | None = None
    #: The caveat sentences asked for, by name, when there is a brief.
    caveats: dict[str, str] = field(default_factory=dict)
    #: Why there is no brief, as a sentence.
    refusal: str | None = None
    #: The server answered, and the swing has nothing to coach from: a miss, or no analysis. False
    #: for a server that could not be used at all, which a caller words differently.
    unanalyzed: bool = False


def brief_through_golf_mcp(
    server: GolfMcpServer,
    session_id: str,
    swing_id: str,
    *,
    caveats: tuple[str, ...] = (),
    errlog: TextIO | None = None,
) -> Brief:
    """`brief_from_view` over `get_swing`, and the caveats named, read through `golf-mcp`.

    The one place both stdio callers start from (the coaching entry here, and `ask_swing.py`'s
    seed), so the conversation and the paragraph open from one rendering of the swing. Never raises
    for an expected failure: a server that is not built, will not start, or predates the brief's
    keys is a refusal, as a swing with no analysis is.
    """
    if shutil.which(str(server.binary)) is None:
        return Brief(refusal=f"golf-mcp is not built at {server.binary} (run `{BUILD_GOLF_MCP}`).")
    try:
        read = read_swing(server, session_id, swing_id, caveats=caveats, errlog=errlog)
    except ImportError as exc:
        return Brief(
            refusal=(
                f"the MCP client is not installed ({exc}). It is part of the `llm` extra "
                "(pip install -e '.[llm]')."
            )
        )
    except Exception as exc:  # the spawn, the pipe, the handshake and the answer's shape
        return Brief(refusal=f"golf-mcp could not be used ({_innermost(exc)}).")
    if read.view is None:
        return Brief(refusal=read.miss, unanalyzed=True)
    if not is_analyzed(read.view):
        return Brief(
            refusal=f"swing {swing_id!r} in session {session_id!r} has no analysis on file.",
            unanalyzed=True,
        )
    try:
        return Brief(text=brief_from_view(read.view), caveats=read.caveats)
    except ValueError as exc:
        return Brief(refusal=f"{exc}.")


def coach_through_golf_mcp(
    session_id: str,
    swing_id: str,
    *,
    server: GolfMcpServer,
    model: str,
    api_key: str | None = None,
    client: Any | None = None,
    errlog: TextIO | None = None,
) -> dict[str, Any]:
    """The coaching paragraph for one stored swing, read through `golf-mcp`: the entry's answer.

    `{"text", "provenance"}` when a paragraph was written, `provenance` being `CoachingProvenance`
    as `analysis.json` stores it beside `coaching_text`, or `{"note"}` when it was not, in the
    results page's vocabulary, for the caller to append to the swing's notes as `api/pipeline.py`
    does. Never raises for an expected failure: no extra or no key is refused before `golf-mcp`
    is started, and every refusal `brief_through_golf_mcp` can give is a note, as a rate limit is.
    """
    client, refused = _coaching_client(api_key, client)
    if refused is not None:
        return {"note": refused}
    brief = brief_through_golf_mcp(
        server, session_id, swing_id, caveats=PROMPT_CAVEATS, errlog=errlog
    )
    if brief.text is None:
        return {"note": f"no written coaching: {brief.refusal}"}

    system = coaching_system_prompt(*(brief.caveats[name] for name in PROMPT_CAVEATS))
    written = _write(client, brief.text, system, model)
    if written.text is None:
        return {"note": written.note}
    return {
        "text": written.text,
        "provenance": {
            "model": written.model,
            "generated_at": _spelled(datetime.now(UTC)),
            "input_digest": _digest(brief.text),
        },
    }


def main(argv: list[str] | None = None) -> int:
    """`python -m golf_coach.feedback.coach --session S --swing W`: one JSON object on stdout.

    `coach_through_golf_mcp`'s answer, exit 0 with a paragraph and 2 with a note, which is what
    `golf-lab analyze` spawns and folds into `analysis.json` (the M29 plan's D15): Rust stays the
    only writer of the artifact and Python the only caller of the LLM. The JSON is ASCII, with every
    other character escaped, so no console code page can mangle it on its way to the reader.

    `golf-mcp` is found at `--golf-mcp`, else `GOLF_MCP`, else the release build, and is started
    with no registry: `get_swing` and the caveats resource read none. Whether to coach at all
    (`GOLF_COACHING_ENABLED`) is the caller's switch, as `--no-coaching` is `analyze_bundle.py`'s.
    """
    from golf_coach.config import settings

    parser = argparse.ArgumentParser(
        prog="python -m golf_coach.feedback.coach",
        description="Write one stored swing's coaching paragraph, read through golf-mcp.",
    )
    parser.add_argument("--session", required=True, help="The session id, e.g. 2026-08-10")
    parser.add_argument("--swing", required=True, help="The swing id within it, e.g. 2")
    parser.add_argument("--sessions-dir", type=Path, default=settings.sessions_dir)
    parser.add_argument("--shots-dir", type=Path, default=settings.shots_dir)
    parser.add_argument("--model", default=settings.coaching_model)
    parser.add_argument(
        "--golf-mcp",
        type=Path,
        default=default_golf_mcp(),
        help="The golf-mcp binary (default: GOLF_MCP, else target/release)",
    )
    args = parser.parse_args(argv)

    server = GolfMcpServer(
        binary=args.golf_mcp, sessions_dir=args.sessions_dir, shots_dir=args.shots_dir
    )
    # One of the sanctioned `get_secret_value` sites (tests/test_config.py pins the set): the
    # Anthropic SDK needs a plain `str`, and this entry is that boundary for the one-shot paragraph,
    # as `api/pipeline.py`'s is on the in-process route until M40.
    key = settings.anthropic_api_key
    answer = coach_through_golf_mcp(
        args.session,
        args.swing,
        server=server,
        model=args.model,
        api_key=key.get_secret_value() if key else None,
    )
    print(json.dumps(answer))
    return 0 if "text" in answer else 2


if __name__ == "__main__":
    raise SystemExit(main())
