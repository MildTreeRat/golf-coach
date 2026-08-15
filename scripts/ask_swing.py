"""Dev CLI: ask a follow-up question about a swing. [M6, ADR-020]

Usage:
    python scripts/ask_swing.py 2026-08-10/2 "why is my tempo quick?"
    python scripts/ask_swing.py --resume conv-2026-08-15-a1b2c3d4 "is that worse than last time?"

    python scripts/ask_swing.py --list              # conversations on file, newest first
    python scripts/ask_swing.py --show conv-...     # replay one

`scripts/analyze_bundle.py` produces one coaching paragraph and stops. This is the conversation
after it: the swing is seeded from its stored `analysis.json`, and everything past that the model
looks up through the same eight tools the MCP server offers external clients — the difference is
that these are called in-process rather than over stdio (ADR-020).

The transcript is stored under `settings.conversations_dir`, one JSON per conversation, so
`--resume` continues where you left off rather than starting again.

Reads only. Nothing here analyzes a swing; point it at something `analyze_bundle.py` or the
upload worker has already produced.

Needs a key (`GOLF_ANTHROPIC_API_KEY` in `.env`, per ADR-019) and the `llm` extra. Without either
it prints why and exits 2 rather than raising.

Exit codes: 0 answered, 1 answered with something flagged, 2 no answer.
"""

from __future__ import annotations

import argparse
import sys
import textwrap
from pathlib import Path

from golf_coach.config import settings
from golf_coach.contracts.conversation import Transcript
from golf_coach.storage.transcript_store import (
    list_transcripts,
    load_transcript,
    new_transcript,
    save_transcript,
    visible_turns,
)

_WRAP = 92


def _wrap(text: str, indent: str = "") -> str:
    return "\n".join(
        textwrap.fill(line, width=_WRAP, initial_indent=indent, subsequent_indent=indent)
        if line.strip()
        else ""
        for line in text.splitlines()
    )


def _load_brief(sessions_dir: Path, swing: str) -> tuple[str, str, str] | None:
    """`SESSION/SWING` -> (brief, session_id, swing_id), or None if it is not analyzed.

    Reuses `feedback.coach.build_brief` — the identical text the one-shot coaching call is
    written from, so a conversation opens from the same picture of the swing the paragraph on the
    results page was written from, rather than a second rendering that could disagree with it.
    """
    from golf_coach.contracts.swing import SwingBundleResult
    from golf_coach.feedback.coach import build_brief

    session_id, _, swing_id = swing.partition("/")
    if not swing_id:
        return None

    path = sessions_dir / session_id / swing_id / "analysis.json"
    try:
        result = SwingBundleResult.model_validate_json(path.read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return None
    return build_brief(result), session_id, swing_id


def _print_turns(transcript: Transcript) -> None:
    for turn in visible_turns(transcript):
        who = "you" if turn["role"] == "user" else "coach"
        print(f"\n[{who}]")
        print(_wrap(turn["text"], indent="  "))


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("swing", nargs="?", help="SESSION/SWING, e.g. 2026-08-10/2")
    parser.add_argument("question", nargs="?", help="What to ask about it")
    parser.add_argument("--resume", metavar="ID", help="Continue an existing conversation")
    parser.add_argument("--list", action="store_true", help="List conversations, newest first")
    parser.add_argument("--show", metavar="ID", help="Print one conversation and exit")
    parser.add_argument("--sessions-dir", type=Path, default=settings.sessions_dir)
    parser.add_argument("--shots-dir", type=Path, default=settings.shots_dir)
    parser.add_argument("--golfers-dir", type=Path, default=settings.golfers_dir)
    parser.add_argument("--conversations-dir", type=Path, default=settings.conversations_dir)
    parser.add_argument("--model", default=settings.coaching_model)
    args = parser.parse_args(argv)

    if args.list:
        found = list_transcripts(args.conversations_dir)
        if not found:
            print("no conversations yet")
            return 0
        for transcript in found:
            about = (
                f"{transcript.session_id}/{transcript.swing_id}"
                if transcript.swing_id
                else "no swing"
            )
            turns = len(visible_turns(transcript))
            print(
                f"{transcript.conversation_id}  {transcript.updated_at:%Y-%m-%d %H:%M}  "
                f"{about:<24} {turns} turns"
            )
        return 0

    if args.show:
        transcript = load_transcript(args.conversations_dir, args.show)
        if transcript is None:
            print(f"no conversation {args.show!r}", file=sys.stderr)
            return 2
        _print_turns(transcript)
        return 0

    # `--resume ID "question"` puts the question in the `swing` slot, because it is the first
    # positional. Accepting it there is friendlier than making the caller pass a swing they
    # already named when the conversation started.
    question = args.question or (args.swing if args.resume else None)
    if not question:
        parser.error("a question is required (or use --list / --show)")

    # Imported here so `--help`, `--list` and `--show` work without the `llm` extra installed.
    try:
        from golf_coach.feedback.conversation import ask
        from golf_coach.mcp.runner_tools import build_tools
        from golf_coach.mcp.server import instructions
    except ImportError as exc:  # pragma: no cover - depends on which extras are installed
        print(
            f"SDK not available ({exc}). Install it with: pip install -e '.[llm]'",
            file=sys.stderr,
        )
        return 2

    from golf_coach.launch_monitor.composite import CompositeShotDataSource
    from golf_coach.launch_monitor.screen.source import ScreenShotDataSource

    brief: str | None = None
    if args.resume:
        transcript = load_transcript(args.conversations_dir, args.resume)
        if transcript is None:
            print(f"no conversation {args.resume!r}", file=sys.stderr)
            return 2
    else:
        if not args.swing or not args.question:
            parser.error("give a swing and a question, or --resume ID \"question\"")
        loaded = _load_brief(args.sessions_dir, args.swing)
        if loaded is None:
            print(
                f"no analyzed swing at {args.swing!r} under {args.sessions_dir}. "
                "Run scripts/analyze_bundle.py on it first.",
                file=sys.stderr,
            )
            return 2
        brief, session_id, swing_id = loaded
        transcript = new_transcript(
            model=args.model, session_id=session_id, swing_id=swing_id
        )

    # `golfers_dir` absent removes the three career tools rather than breaking them, exactly as
    # it does for the MCP server — and the briefing drops their reading rules to match, so the
    # model is never told how to read a tool it does not have.
    golfers_dir = args.golfers_dir if args.golfers_dir.exists() else None
    source = CompositeShotDataSource([ScreenShotDataSource(args.shots_dir)])
    tools = build_tools(args.sessions_dir, source, golfers_dir=golfers_dir)

    key = settings.anthropic_api_key
    outcome = ask(
        question,
        transcript=transcript,
        tools=tools,
        briefing=instructions(career_tools=golfers_dir is not None),
        model=args.model,
        brief=brief,
        api_key=key.get_secret_value() if key else None,
    )

    if outcome.transcript is not None:
        save_transcript(outcome.transcript, args.conversations_dir)

    print(f"\n[{transcript.conversation_id}]")
    if outcome.tools_called:
        for name in outcome.tools_called:
            print(f"  -> {name}")
    if outcome.text:
        print()
        print(_wrap(outcome.text))
    if outcome.note:
        print(f"\nnote: {_wrap(outcome.note).strip()}", file=sys.stderr)

    if outcome.text is None:
        return 2
    return 1 if outcome.note else 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
