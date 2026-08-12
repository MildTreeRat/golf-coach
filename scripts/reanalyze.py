"""Dev CLI: re-run the pipeline over swings whose stored analysis is out of date. [Career step 3]

Usage:
    python scripts/reanalyze.py                    # every swing that needs it
    python scripts/reanalyze.py --dry-run          # which ones, and why, changing nothing
    python scripts/reanalyze.py 2026-08-09/2       # one, by SESSION/SWING or directory
    python scripts/reanalyze.py --player aaron     # one golfer's swings
    python scripts/reanalyze.py --all              # every swing, up to date or not
    python scripts/reanalyze.py --video            # re-render aligned.mp4 as well

**"Out of date" has three forms, and only one of them was visible before this existed.** A swing
needs re-running when it has no `analysis.json` at all, when a clip has been re-uploaded since it
was analyzed (`AnalysisState.matches`), or when it was analyzed by an **older engine** —
`analysis_version` below `contracts.swing.ANALYSIS_VERSION`. That third case is the one this
script was written for: the inputs are unchanged, so nothing in the older staleness check could
see it, and the artifact looks current while carrying numbers that today's code would not produce.

Why that matters enough to have a command: career mode reads a golfer's *spread* across sessions,
and pooling two engine generations manufactures variance out of a code change — the mirror image
of the re-upload counting `storage.corpus` already refuses. `read_corpus` reports those swings as
`ExclusionReason.OUTDATED` and keeps them out of the counts; this is how they get back in.

**It is cheap.** Pose keypoints are cached per view against the clip's sha256 and shots are looked
up in the shot store by the photo's hash, so a re-run of an unchanged bundle touches neither
MediaPipe nor OCR — seconds, not minutes. The two genuinely expensive steps are the side-by-side
render and the Claude coaching call, and both are **off by default** here: `--video` and
`--coaching` opt back in. Anything analyzed without a render keeps whatever `aligned.mp4` it
already had, so the script checks whether the alignment anchors moved and says so, rather than
leaving a video that silently disagrees with the JSON beside it.

Idempotent: run it twice and the second run reports nothing to do.

Exit codes: 0 clean, 1 something wants a human (a run failed, a role is missing, a shot needs
review, a video went stale), 2 a named target could not be found.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from golf_coach.api.pipeline import (
    ALIGNED_NAME,
    PipelineOptions,
    PipelineOutcome,
    analyze_swing_dir,
    resolve_swing_dir,
)
from golf_coach.api.state import is_outdated, load_analysis, load_state, stored_analysis_version
from golf_coach.config import settings
from golf_coach.contracts.swing import ANALYSIS_VERSION, SwingBundleResult
from golf_coach.storage.bundle_store import SwingBundleStore
from golf_coach.storage.manifest import SwingManifest, load_manifest, manifest_path


class Target:
    """One swing directory to re-run, and the sentence saying why."""

    def __init__(self, swing_dir: Path, manifest: SwingManifest, reason: str) -> None:
        self.swing_dir = swing_dir
        self.manifest = manifest
        self.reason = reason
        self.before = load_analysis(swing_dir)

    @property
    def ref(self) -> str:
        return f"{self.manifest.session_id}/{self.manifest.swing_id}"


def _reason_to_rerun(swing_dir: Path, manifest: SwingManifest) -> str | None:
    """Why this swing wants re-running, or None if its stored result is current.

    Ordered by how fundamental the problem is, and only the first is reported: a swing with no
    analysis at all is not also "outdated", and an outdated result over re-uploaded bytes is going
    to be re-run either way.
    """
    analysis = load_analysis(swing_dir)
    if analysis is None:
        return "never analyzed"

    state = load_state(swing_dir)
    if state is not None and not state.matches(manifest):
        return "an input was re-uploaded after it was analyzed"

    if is_outdated(analysis):
        return (
            f"analyzed by engine version {stored_analysis_version(analysis)}, "
            f"{ANALYSIS_VERSION} is installed"
        )
    return None


def _collect(
    store: SwingBundleStore,
    *,
    refs: list[str],
    sessions_dir: Path,
    player_id: str | None,
    everything: bool,
) -> tuple[list[Target], list[str]]:
    """The swings to re-run, and any named ref that resolved to nothing.

    Walks the bundle store directly rather than `storage.corpus.read_corpus`, and the difference
    is deliberate: the corpus collapses re-uploads of one clip into a single swing, but every one
    of those directories still holds an `analysis.json` that the results page and the MCP server
    will happily serve. A repair that skipped them would leave the duplicates quoting older
    numbers than the swing they duplicate.
    """
    missing: list[str] = []
    pairs: list[tuple[Path, SwingManifest]] = []

    if refs:
        for ref in refs:
            swing_dir = resolve_swing_dir(ref, sessions_dir)
            manifest = (
                load_manifest(manifest_path(swing_dir)) if swing_dir is not None else None
            )
            if swing_dir is None or manifest is None:
                missing.append(ref)
            else:
                pairs.append((swing_dir, manifest))
    else:
        for session_id in store.list_session_ids():
            for manifest in store.get_session(session_id):
                pairs.append((store.root / session_id / manifest.swing_id, manifest))

    targets: list[Target] = []
    for swing_dir, manifest in pairs:
        if player_id is not None and manifest.player_id != player_id:
            continue
        reason = _reason_to_rerun(swing_dir, manifest)
        if reason is None and not everything and not refs:
            continue
        targets.append(Target(swing_dir, manifest, reason or "up to date, re-run by request"))
    return targets, missing


# --------------------------------------------------------------------------------------
# Reporting
# --------------------------------------------------------------------------------------


def _as_dict(analysis: object) -> dict:
    """A stored `analysis.json` or a fresh result, as one comparable shape.

    Both sides of every diff below go through here, because the "before" side is whatever an
    older engine happened to write: a missing key must read as unknown rather than raise, and a
    window stored as a JSON list must compare equal to the tuple the engine returns.
    """
    if isinstance(analysis, SwingBundleResult):
        return analysis.model_dump(mode="json")
    return analysis if isinstance(analysis, dict) else {}


def _window(value: object) -> tuple[int, ...] | None:
    """`[574, 790]` and `(574, 790)` are the same window. JSON only has the first."""
    return tuple(value) if isinstance(value, list | tuple) else None


def _anchors(analysis: object) -> tuple[tuple[int | None, ...], ...]:
    """`(motion_start, top, impact)` per view — the instants the side-by-side render warps to."""
    alignment = _as_dict(analysis).get("alignment")
    if not isinstance(alignment, dict):
        return ((None,), (None,))

    out: list[tuple[int | None, ...]] = []
    for view in ("a", "b"):
        side = alignment.get(view)
        anchors = side.get("anchors") if isinstance(side, dict) else None
        if isinstance(anchors, dict):
            out.append(tuple(anchors.get(k) for k in ("motion_start", "top", "impact")))
        else:
            out.append((None,))
    return (out[0], out[1])


def _before_after(target: Target, result: SwingBundleResult) -> list[str]:
    """The one-line diff per swing: what the re-run actually moved."""
    before, after = _as_dict(target.before), _as_dict(result)
    swing_before = before.get("swing") if isinstance(before.get("swing"), dict) else {}
    assert isinstance(swing_before, dict)

    def _num(value: object) -> str:
        return f"{value:.4f}" if isinstance(value, int | float) else "--"

    rows: list[tuple[str, object, object]] = [
        ("version", stored_analysis_version(target.before), result.analysis_version),
        (
            "score",
            _num(swing_before.get("overall_score")),
            _num(result.swing.overall_score),
        ),
        (
            "measurements",
            len(swing_before.get("measurements") or []),
            len(result.swing.measurements),
        ),
        ("window", _window(before.get("face_on_window")), _window(after.get("face_on_window"))),
        ("anchors", _anchors(target.before), _anchors(result)),
    ]
    return [f"{label} {old} -> {new}" for label, old, new in rows if old != new] or ["no change"]


def _video_went_stale(target: Target, result: SwingBundleResult, *, rendered: bool) -> bool:
    """Did the anchors move under an `aligned.mp4` this run did not re-render?

    The video is drawn from the alignment, so moved anchors mean the file on disk no longer shows
    what the JSON beside it claims. Worth a line rather than a silent inconsistency — this is the
    same class of drift the state sidecar had.
    """
    if rendered or not (target.swing_dir / ALIGNED_NAME).exists():
        return False
    return _anchors(target.before) != _anchors(result)


def _run_one(target: Target, options: PipelineOptions, *, verbose: bool) -> tuple[bool, bool]:
    """Re-run one swing. Returns `(ok, wants_attention)`."""
    print(f"\n{target.ref}  ({target.reason})")
    outcome: PipelineOutcome = analyze_swing_dir(
        target.swing_dir, options=options, log=print if verbose else lambda _: None
    )
    if outcome.result is None:
        print(f"  FAILED: {outcome.error}")
        return False, True

    print(f"  {' | '.join(_before_after(target, outcome.result))}")
    stale_video = _video_went_stale(target, outcome.result, rendered=options.render_video)
    if stale_video:
        print(
            f"  ! the alignment anchors moved, so {ALIGNED_NAME} no longer matches this result"
            " — re-run with --video"
        )
    return True, bool(outcome.flagged or stale_video)


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(
        prog="reanalyze", description=__doc__.splitlines()[0], allow_abbrev=False
    )
    parser.add_argument(
        "swings",
        nargs="*",
        help="SESSION/SWING refs or swing directories; default is every swing that needs it",
    )
    parser.add_argument("--sessions-dir", type=Path, default=settings.sessions_dir)
    parser.add_argument("--shots-dir", type=Path, default=settings.shots_dir)
    parser.add_argument("--player", help="Limit to one golfer's swings, by stored id.")
    parser.add_argument(
        "--all",
        action="store_true",
        help="re-run every swing, including ones whose stored result is already current",
    )
    parser.add_argument("--dry-run", action="store_true", help="List the targets, change nothing.")
    parser.add_argument(
        "--video",
        action="store_true",
        help="also re-render aligned.mp4 (minutes per swing; off by default)",
    )
    parser.add_argument(
        "--coaching",
        action="store_true",
        help="also ask Claude for the written paragraph (off by default)",
    )
    parser.add_argument(
        "--verbose", action="store_true", help="Print the pipeline's own narration per swing."
    )
    args = parser.parse_args(argv)

    store = SwingBundleStore(args.sessions_dir)
    targets, missing = _collect(
        store,
        refs=args.swings,
        sessions_dir=args.sessions_dir,
        player_id=args.player,
        everything=args.all,
    )

    for ref in missing:
        print(f"error: no swing bundle at {ref!r} (looked in {args.sessions_dir})", file=sys.stderr)
    if missing:
        return 2

    if not targets:
        print(f"Nothing to re-analyze in {args.sessions_dir} — every stored result is current.")
        return 0

    print(f"{len(targets)} swing(s) to re-analyze (engine version {ANALYSIS_VERSION}):")
    for target in targets:
        print(f"  {target.ref:<24} {target.reason}")
    if args.dry_run:
        print("\n--dry-run: nothing was changed.")
        return 0

    defaults = PipelineOptions()
    options = PipelineOptions(
        shots_dir=args.shots_dir,
        render_video=args.video,
        coaching=defaults.coaching and args.coaching,
    )

    failures = 0
    attention = 0
    for target in targets:
        ok, wants_attention = _run_one(target, options, verbose=args.verbose)
        failures += 0 if ok else 1
        attention += 1 if wants_attention else 0

    print(f"\nRe-analyzed {len(targets) - failures}/{len(targets)} swing(s).")
    if failures:
        print(f"{failures} failed — see above.")
    return 1 if (failures or attention) else 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
