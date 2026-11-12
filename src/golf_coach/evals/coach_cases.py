"""The coach-feedback eval's cases: synthetic swings, recorded once, served by `golf-mcp`. [M43 P15]

    python -m golf_coach.evals.coach_cases [--golf-mcp PATH]

`evals/coach_feedback/cases.json` is written by hand: each case's id, tags, where it is stored and
what it is made from. This turns it into what the coach eval reads, both committed beside it:

- `sessions/<session>/<swing>/`, a manifest and an `analysis.json` per case, exactly as a stored
  swing is laid out, so the eval reaches each one the way the app does, through `golf-mcp`'s
  `get_swing` (the plan's Decision 4: synthetic swings, so the whole eval can be committed);
- `cases.md`, each case's brief **as `golf-mcp` serves it**, which is the text the model coaches
  from, for the user's review (the plan's P16).

Re-running it changes no byte: every time it stamps is fixed, and the engine is a pure function of
the keypoints. `tests/evals/test_coach_cases.py` fails when the committed files are not what this
writes.

## A recorder, and what it may import

ADR-036 §1 keeps the eval modules off the frozen lab: the runner, the flows and the judge import
only the standard library, the SDK, `config.py`, `llm_log` and the LLM survivors. This module is
the one exception that clause names: a recorder in ADR-035 §4's sense. It runs the frozen lab's
behaviour once to capture it as committed data, nothing on the run path imports it, and it goes when
its imports go (M40), leaving the cases behind. So it reaches the frozen engine
(`analysis.engine`), the rule-based verdict (`feedback.rules`), the screen validator
(`launch_monitor.screen`) and the manifest store, and nothing else in the eval package reaches them.

## How one case is made

1. **The swing** is `synthetic_swing.make_swing` with the case's arguments, once for the face-on
   view and, when the case names one, once for the down-the-line view.
2. **The shot** is what the screen printed (`printed`, `ShotData`'s field names) and what the reader
   made of the photo (`read_confidence`, `read_warnings`), put through the frozen screen validator.
   So whether a shot is flagged for review, and the warning that says why, is the validator's call
   and never typed in: a "clean" shot whose smash factor does not divide out is flagged like a
   real one would be.
3. **The analysis** is the frozen `analyze_swing_bundle` over both, with the case's handedness and
   loft, and **the verdict** is `build_feedback` over its result, joined as the pipeline joins them.
4. **The placements and the rotation rows** are kept only where the case names them (`population`,
   `rotation`). The engine records both families on every swing it can, and on these keypoints both
   say more about the fixture than about a swing: `make_swing`'s shoulders never move, so four of
   the five rotation rows read exactly zero, and its motion is far from any tour swing's shape, so
   every trajectory placement reads past the 90th percentile. Kept everywhere, they would put the
   same two caveats in every brief and leave the cases that are about them no different from the
   rest. A name the engine did not record is refused rather than skipped.

## Tags are checked, not trusted

A case declares its tags, and `derived_tags` reads them back off the result: a case whose result
does not show exactly what its tags claim is refused. Without that, a change to `make_swing`'s
arguments could quietly lose the feature a case exists for, and the eval would report coverage it
does not have.
"""

from __future__ import annotations

import argparse
import json
import re
import tempfile
from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from datetime import UTC, datetime
from pathlib import Path
from typing import Any

from golf_coach.analysis.engine import analyze_swing_bundle
from golf_coach.config import REPO_ROOT
from golf_coach.contracts.club import ClubId
from golf_coach.contracts.golfer import Handedness
from golf_coach.contracts.keypoints import KeypointsFile
from golf_coach.contracts.pivots import PIVOTS_BY_NAME
from golf_coach.contracts.placements import PLACEMENTS_BY_NAME
from golf_coach.contracts.shot import ShotData, ShotProvenance, ShotSource
from golf_coach.contracts.swing import SwingBundleResult
from golf_coach.contracts.unscored import UnscoredReason
from golf_coach.evals.synthetic_swing import make_swing
from golf_coach.feedback.coach import GolfMcpServer, brief_through_golf_mcp, default_golf_mcp
from golf_coach.feedback.rules import build_feedback
from golf_coach.launch_monitor.screen.parser import ParsedShot
from golf_coach.launch_monitor.screen.validate import validate_parse
from golf_coach.storage.manifest import SwingManifest, manifest_path, save_manifest

CASES_DIR = REPO_ROOT / "evals" / "coach_feedback"
CASES = CASES_DIR / "cases.json"
SESSIONS = "sessions"
CASES_MD = "cases.md"

#: When every case was stored and its shot taken. Fixed, so a rerun writes the same bytes.
STORED_AT = datetime(2026, 7, 1, 15, 0, tzinfo=UTC)

#: The launch monitor every shot is read as. `golf-mcp`'s brief does not name it.
DEVICE = "hd_golf"

#: Every tag a case may carry, in the order the plan's P15 lists them.
TAGS = (
    "clean",
    "unscored",
    "uncalibrated-placement",
    "interim-pivot",
    "flagged-shot",
    "partial-alignment",
    "no-handedness",
    "nothing-to-fix",
)

_ID = re.compile(r"^[a-z0-9]+(-[a-z0-9]+)*$")


class CaseError(ValueError):
    """A case that cannot be made as written, named, with what is wrong with it."""


@dataclass(frozen=True)
class CoachCase:
    """One case as `cases.json` writes it."""

    id: str
    tags: tuple[str, ...]
    session_id: str
    swing_id: str
    club: ClubId | None
    about: str
    inputs: Mapping[str, Any]


def load_cases(path: Path = CASES) -> list[CoachCase]:
    """`cases.json`'s cases, refusing a malformed id, an unknown tag or a reused swing."""
    doc = json.loads(path.read_text(encoding="utf-8"))
    cases: list[CoachCase] = []
    for raw in doc["cases"]:
        club = raw.get("club")
        case = CoachCase(
            id=raw["id"],
            tags=tuple(raw["tags"]),
            session_id=raw["session_id"],
            swing_id=raw["swing_id"],
            club=ClubId(club) if club is not None else None,
            about=raw["about"],
            inputs=raw["inputs"],
        )
        if not _ID.match(case.id):
            raise CaseError(f"{case.id!r}: an id is lowercase words joined by hyphens")
        unknown = sorted(set(case.tags) - set(TAGS))
        if unknown or not case.tags:
            raise CaseError(f"{case.id}: tags {unknown or '[]'} are not among {list(TAGS)}")
        cases.append(case)
    for label, keys in (
        ("id", [case.id for case in cases]),
        ("swing", [f"{case.session_id}/{case.swing_id}" for case in cases]),
    ):
        reused = sorted({key for key in keys if keys.count(key) > 1})
        if reused:
            raise CaseError(f"more than one case has the {label} {reused}")
    return cases


def store_swing(
    sessions_dir: Path,
    result: SwingBundleResult,
    *,
    club: ClubId | None = None,
    when: datetime = datetime(2026, 8, 10, 15, 0, tzinfo=UTC),
) -> Path:
    """`result` as the pipeline stores it: a manifest, and `analysis.json` beside it.

    `tests/feedback/conftest.py`'s `store_swing` fixture until M43 P15, which moved it here so the
    generator and the stdio routes' tests write a swing one way (`tests/` is not a package). The
    keypoints and detections are left out, as `analysis.json` leaves them; `when` stamps the
    manifest, fixed so that a rerun writes the same bytes.
    """
    swing_dir = sessions_dir / result.session_id / result.swing_id
    swing_dir.mkdir(parents=True, exist_ok=True)
    manifest = SwingManifest(
        swing_id=result.swing_id,
        session_id=result.session_id,
        created_at=when,
        updated_at=when,
        club=club,
    )
    save_manifest(manifest, manifest_path(swing_dir))
    analysis = result.model_dump(mode="json", exclude={"keypoints", "detections"})
    (swing_dir / "analysis.json").write_text(
        json.dumps(analysis, indent=2) + "\n", encoding="utf-8"
    )
    return swing_dir


def _shot(case: CoachCase, spec: Mapping[str, Any]) -> ShotData:
    """The printed values, flagged or not by the frozen screen validator (step 2 above)."""
    printed: dict[str, float | str] = dict(spec["printed"])
    read = ParsedShot(
        device=DEVICE,
        values=printed,
        confidence=float(spec["read_confidence"]),
        warnings=list(spec.get("read_warnings", [])),
    )
    checked = validate_parse(read)
    return ShotData.model_validate(
        {
            "shot_id": f"{case.session_id}-{case.swing_id}",
            "session_id": case.session_id,
            "timestamp": STORED_AT,
            "source": ShotSource.SCREEN,
            **printed,
            "provenance": ShotProvenance(
                device=DEVICE,
                parse_confidence=round(checked.confidence, 3),
                needs_review=checked.needs_review,
                warnings=checked.warnings,
            ),
        }
    )


def _view(arguments: Mapping[str, Any] | None) -> KeypointsFile | None:
    return None if arguments is None else KeypointsFile(frames=make_swing(**arguments))


def _kept(case: CoachCase, result: SwingBundleResult) -> SwingBundleResult:
    """Only the placements and rotation rows the case names (step 4 above)."""
    wanted = {*case.inputs.get("population", []), *case.inputs.get("rotation", [])}
    recorded = {measurement.name for measurement in result.swing.measurements}
    missing = sorted(wanted - recorded)
    if missing:
        raise CaseError(f"{case.id}: the engine recorded no {missing} on this swing")
    family = PLACEMENTS_BY_NAME.keys() | PIVOTS_BY_NAME.keys()
    measurements = [
        measurement
        for measurement in result.swing.measurements
        if measurement.name not in family or measurement.name in wanted
    ]
    return result.model_copy(
        update={"swing": result.swing.model_copy(update={"measurements": measurements})}
    )


def build_bundle(case: CoachCase) -> SwingBundleResult:
    """One case's stored result: swing, shot, analysis, verdict, then the kept rows."""
    inputs = case.inputs
    face_on = _view(inputs["face_on"])
    assert face_on is not None, "a case always has a face-on view"
    shot_spec = inputs.get("shot")
    handedness = inputs.get("handedness")
    result = analyze_swing_bundle(
        case.swing_id,
        case.session_id,
        face_on,
        down_the_line=_view(inputs.get("down_the_line")),
        shot=None if shot_spec is None else _shot(case, shot_spec),
        handedness=None if handedness is None else Handedness(handedness),
        loft_deg=inputs.get("loft_deg"),
    )
    result = result.model_copy(update={"feedback": build_feedback(result.swing)})
    result = _kept(case, result)
    shown = derived_tags(result)
    if shown != set(case.tags):
        raise CaseError(
            f"{case.id}: tagged {sorted(case.tags)}, but the result shows {sorted(shown)}"
        )
    return result


def derived_tags(result: SwingBundleResult) -> set[str]:
    """The tags `result` shows, read off it rather than taken from the case.

    `clean` is a swing with something to fix and none of the other features. `nothing-to-fix` is a
    full panel, every checkpoint scored and passed, and may sit beside the features that do not
    touch the panel. `partial-alignment` is the contract's own `is_degraded`, so a pair aligned on a
    heard strike is not partial.
    """
    swing = result.swing
    tags: set[str] = set()
    reasons = [entry.reason for entry in swing.unscored]
    if any(reason is not UnscoredReason.NO_HANDEDNESS for reason in reasons):
        tags.add("unscored")
    if UnscoredReason.NO_HANDEDNESS in reasons:
        tags.add("no-handedness")
    names = [measurement.name for measurement in swing.measurements]
    if any(
        name in PLACEMENTS_BY_NAME and not PLACEMENTS_BY_NAME[name].calibrated for name in names
    ):
        tags.add("uncalibrated-placement")
    if any(name in PIVOTS_BY_NAME for name in names):
        tags.add("interim-pivot")
    shot = swing.shot
    if shot is not None and shot.provenance is not None and shot.provenance.needs_review:
        tags.add("flagged-shot")
    if result.alignment is not None and result.alignment.quality.is_degraded:
        tags.add("partial-alignment")
    checkpoints = swing.checkpoint_scores
    if checkpoints and all(score.passed for score in checkpoints) and not swing.unscored:
        tags.add("nothing-to-fix")
    if not tags:
        tags.add("clean")
    return tags


def store_cases(sessions_dir: Path, cases: Sequence[CoachCase]) -> None:
    """Every case's swing under `sessions_dir`, as `golf-mcp` reads a sessions directory."""
    for case in cases:
        store_swing(sessions_dir, build_bundle(case), club=case.club, when=STORED_AT)


def serve_briefs(server: GolfMcpServer, cases: Sequence[CoachCase]) -> dict[str, str]:
    """Each case's brief as `golf-mcp` serves it: the text the model coaches from."""
    briefs: dict[str, str] = {}
    for case in cases:
        brief = brief_through_golf_mcp(server, case.session_id, case.swing_id)
        if brief.text is None:
            raise CaseError(f"{case.id}: golf-mcp served no brief: {brief.refusal}")
        briefs[case.id] = brief.text
    return briefs


def render_md(cases: Sequence[CoachCase], briefs: Mapping[str, str]) -> str:
    """`cases.md`: a table of the cases, then each one's inputs and its brief."""
    out = [
        "# Coach-feedback eval cases",
        "",
        "<!-- Written by `python -m golf_coach.evals.coach_cases` from cases.json, through"
        " golf-mcp. tests/evals/test_coach_cases.py fails when this file, the sessions and"
        " cases.json disagree. Edit cases.json, then run that command. -->",
        "",
        "What the coach-feedback eval asks a model to coach from (M43,"
        " [ADR-036](../../docs/decisions/036-llm-evaluation-and-observability.md)). Every swing is"
        " synthetic: pose keypoints from `make_swing`, a shot's printed values, analysed by the"
        " frozen engine and stored under `sessions/`. Each brief below is the one `golf-mcp`"
        " serves for that swing, which is the user message the model is sent. The system prompt"
        " is `coaching_system_prompt` over `golf-mcp`'s caveats, the same for every case, and is"
        " not repeated here.",
        "",
        f"{len(cases)} cases.",
        "",
        "| Case | Tags | Swing | Club | About |",
        "|---|---|---|---|---|",
    ]
    for case in cases:
        out.append(
            f"| `{case.id}` | {', '.join(case.tags)} | {case.session_id}/{case.swing_id}"
            f" | {case.club.value if case.club else 'none'} | {case.about} |"
        )
    for case in cases:
        out += [
            "",
            f"## `{case.id}`",
            "",
            f"Tags: {', '.join(case.tags)}. Swing {case.session_id}/{case.swing_id}, club"
            f" {case.club.value if case.club else 'none'}.",
            "",
            case.about,
            "",
            "Made from:",
            "",
            "```json",
            json.dumps(case.inputs, indent=2),
            "```",
            "",
            "The brief:",
            "",
            "```text",
            briefs[case.id],
            "```",
        ]
    return "\n".join(out) + "\n"


def written_files(golf_mcp: Path, cases: Sequence[CoachCase]) -> dict[Path, bytes]:
    """Every file the generator writes, by its path under `evals/coach_feedback/`."""
    with tempfile.TemporaryDirectory() as staging_dir:
        staging = Path(staging_dir)
        store_cases(staging / SESSIONS, cases)
        (staging / "shots").mkdir()
        server = GolfMcpServer(
            binary=golf_mcp, sessions_dir=staging / SESSIONS, shots_dir=staging / "shots"
        )
        briefs = serve_briefs(server, cases)
        files = {
            path.relative_to(staging): path.read_bytes()
            for path in sorted((staging / SESSIONS).rglob("*"))
            if path.is_file()
        }
    files[Path(CASES_MD)] = render_md(cases, briefs).encode("utf-8")
    return files


def write(out_dir: Path, files: Mapping[Path, bytes]) -> None:
    """`files` under `out_dir`, and any older file under its `sessions/` removed."""
    stale = [
        path
        for path in (out_dir / SESSIONS).rglob("*")
        if path.is_file() and path.relative_to(out_dir) not in files
    ]
    for path in stale:
        path.unlink()
    for relative, data in files.items():
        target = out_dir / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(data)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        prog="python -m golf_coach.evals.coach_cases",
        description="Write the coach-feedback eval's sessions and cases.md from cases.json.",
    )
    parser.add_argument(
        "--golf-mcp",
        type=Path,
        default=default_golf_mcp(),
        help="The golf-mcp binary (default: GOLF_MCP, else target/release)",
    )
    args = parser.parse_args(argv)
    cases = load_cases(CASES)
    write(CASES_DIR, written_files(args.golf_mcp, cases))
    print(f"wrote {len(cases)} cases under {CASES_DIR}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
