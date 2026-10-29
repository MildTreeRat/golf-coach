"""Dev CLI: ask a follow-up question about a swing. [M6, ADR-020]

Usage:
    python scripts/ask_swing.py 2026-08-10/2 "why is my tempo quick?"
    python scripts/ask_swing.py --resume conv-2026-08-15-a1b2c3d4 "is that worse than last time?"

    python scripts/ask_swing.py --list              # conversations on file, newest first
    python scripts/ask_swing.py --show conv-...     # replay one

`golf-lab analyze` produces one coaching paragraph and stops. This is the conversation after
it: the swing is seeded from `golf-mcp`'s view of it, rendered as the coaching paragraph's
brief is (`feedback.coach.brief_from_view`, since M29 P17), and everything past that the model
looks up through the same twelve tools `golf-mcp` offers every client, over stdio (ADR-020's
Option B, since M29 P16). The briefing, and the caveat sentence the follow-up instructions quote,
come from that server too, so this script keeps no copy of either. The frozen results page's chat
panel still calls frozen Python's eleven in process, until M40.

`golf-mcp` is found at `--golf-mcp`, else `GOLF_MCP`, else the release build under `target/`, and
one is started per question, plus one to read the swing when a conversation starts. Not built is
exit 2, naming `cargo build --release -p golf-mcp`.

The transcript is stored under `settings.conversations_dir`, one JSON per conversation, so
`--resume` continues where you left off rather than starting again.

Reads only. Nothing here analyzes a swing; point it at something `golf-lab analyze` has already
produced, or the frozen upload worker did before M29 closed its uploads.

Needs a key (`GOLF_ANTHROPIC_API_KEY` in `.env`, per ADR-019) and the `llm` extra. Without either
it prints why and exits 2 rather than raising.

Exit codes: 0 answered, 1 answered with something flagged, 2 no answer.
"""

from __future__ import annotations

import argparse
import shutil
import sys
import textwrap
from pathlib import Path

from golf_coach.config import settings
from golf_coach.contracts.conversation import Transcript

# No SDK at module scope: `--help`, `--list` and `--show` work without the `llm` extra, and
# `ask_through_golf_mcp` turns a missing one into the note this script prints.
from golf_coach.feedback.coach import (
    BUILD_GOLF_MCP,
    GolfMcpServer,
    brief_through_golf_mcp,
    default_golf_mcp,
)
from golf_coach.feedback.conversation import ask_through_golf_mcp
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


def _load_brief(server: GolfMcpServer, swing: str) -> tuple[str, str, str] | str:
    """`SESSION/SWING` -> (brief, session_id, swing_id), or the sentence saying why there is none.

    `feedback.coach.brief_through_golf_mcp`: the identical text the coaching paragraph is written
    from by `python -m golf_coach.feedback.coach`, so a conversation opens from the same picture of
    the swing the paragraph was written from, rather than a second rendering that could disagree
    with it. Until M29 P17 this was frozen Python's `build_brief` over `analysis.json`, which reads
    a pydantic shape ADR-035 clause 1 does not keep past M40, and quoted frozen Python's copy of the
    alignment caveat where this quotes `golf-mcp`'s.
    """
    not_analyzed = (
        f"no analyzed swing at {swing!r} under {server.sessions_dir}. "
        "Run golf-lab analyze on it first."
    )
    session_id, _, swing_id = swing.partition("/")
    if not swing_id:
        return not_analyzed
    brief = brief_through_golf_mcp(server, session_id, swing_id)
    if brief.text is not None:
        return brief.text, session_id, swing_id
    return not_analyzed if brief.unanalyzed else f"no brief: {brief.refusal}"


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
    parser.add_argument(
        "--golf-mcp",
        type=Path,
        default=default_golf_mcp(),
        help="The golf-mcp binary (default: GOLF_MCP, else target/release)",
    )
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

    # Checked here as well as in `ask_through_golf_mcp`, so that the commonest setup slip answers
    # before anything is read, with the command that fixes it.
    if shutil.which(str(args.golf_mcp)) is None:
        print(
            f"golf-mcp is not built at {args.golf_mcp}. Build it with `{BUILD_GOLF_MCP}`, or "
            "point --golf-mcp or GOLF_MCP at a build.",
            file=sys.stderr,
        )
        return 2

    # `golfers_dir` absent is `--no-registry`: the server then offers none of its per-golfer
    # tools rather than six that answer "unknown golfer" to every name, and its briefing drops
    # their reading rules to match, so the model is never told how to read a tool it does not have.
    server = GolfMcpServer(
        binary=args.golf_mcp,
        sessions_dir=args.sessions_dir,
        shots_dir=args.shots_dir,
        golfers_dir=args.golfers_dir if args.golfers_dir.exists() else None,
    )

    brief: str | None = None
    if args.resume:
        transcript = load_transcript(args.conversations_dir, args.resume)
        if transcript is None:
            print(f"no conversation {args.resume!r}", file=sys.stderr)
            return 2
    else:
        if not args.swing or not args.question:
            parser.error("give a swing and a question, or --resume ID \"question\"")
        loaded = _load_brief(server, args.swing)
        if isinstance(loaded, str):
            print(loaded, file=sys.stderr)
            return 2
        brief, session_id, swing_id = loaded
        transcript = new_transcript(
            model=args.model, session_id=session_id, swing_id=swing_id
        )

    key = settings.anthropic_api_key
    outcome = ask_through_golf_mcp(
        question,
        transcript=transcript,
        server=server,
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
