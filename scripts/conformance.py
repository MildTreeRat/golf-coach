"""The Python core as an oracle: schemas out, golden vectors in, a diff either way. [M19]

    python scripts/conformance.py check              # every committed vector, against this build
    python scripts/conformance.py check --id corpus/2026-08-09-2
    python scripts/conformance.py run < vector.json  # one vector in, its result on stdout
    python scripts/conformance.py list               # what is committed, and where it came from
    python scripts/conformance.py regenerate         # rewrite spec/ from contracts + data/

**Why this exists.** ADR-030 commits to a second implementation of the swing loop in Rust, and
two cores that disagree *silently* is the failure mode that whole plan has to survive. Review does
not catch a drift of a fraction of a unit, and the only oracle before this was the Python test
suite — which a Rust port cannot run. So the suite is not the specification; `spec/` is, and this
is the program that produces it and checks against it.

**The contract with a port is exactly three things**, and nothing here is allowed to be a fourth:

1. `spec/schemas/*.schema.json` — the shapes that cross the seam, exported from `contracts/`.
2. `spec/vectors/**` — inputs paired with the output this build produces for them.
3. `docs/CONFORMANCE.md` — which fields must match exactly and which to an epsilon, written down
   rather than inferred from this file.

A port passes when it reads a vector's `input`, produces a result, and `compare_results` below
finds no differences against the vector's `expected`. It does not have to be written in Python and
it does not have to call anything in this repo: `run` reads a vector on stdin and writes the
serialized result on stdout, so an implementation in any language is diffed by a shell pipeline.

**The serialization is the shell's, not the contract's.** `api/pipeline.py` writes `analysis.json`
as `model_dump_json(exclude={"swing": {"keypoints", "detections"}})` — the exclusion lives at the
call site, so a port that serialized `SwingBundleResult` faithfully would emit the whole keypoint
list and differ on a field nobody meant to compare. `_serialize` below is the one copy of that
decision that a port is asked to match, and `tests/test_conformance.py` pins it against the
pipeline's so the two cannot drift.

Base install only — no extras. Generating corpus vectors reads `data/processed/`, which exists on
the capture machine and nowhere else; *checking* them reads `spec/` alone, which is the whole point
of committing them.
"""

from __future__ import annotations

import argparse
import gzip
import json
import math
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from golf_coach.api.state import AnalysisState
from golf_coach.contracts.audio import AudioFile
from golf_coach.contracts.bag import Bag
from golf_coach.contracts.golfer import Golfer, Handedness
from golf_coach.contracts.intent import PracticeGoal
from golf_coach.contracts.keypoints import KeypointsFile
from golf_coach.contracts.shot import ShotData
from golf_coach.contracts.swing import ANALYSIS_VERSION, SwingBundleResult, SwingResult
from golf_coach.storage.manifest import SwingManifest
from golf_coach.storage.session_meta import SessionMeta

REPO = Path(__file__).resolve().parent.parent
SPEC = REPO / "spec"
SCHEMAS = SPEC / "schemas"
VECTORS = SPEC / "vectors"
MANIFEST = VECTORS / "MANIFEST.json"

#: What `api/pipeline.py:analyze_swing_dir` drops before writing `analysis.json`, restated here
#: because a port has to make the same drop and has no reason to guess at it. Keypoints are the
#: *input*, echoed back on the result as the data it was computed from; round-tripping them
#: through the comparison would make every vector 30x larger and check nothing.
EXCLUDED_FROM_RESULT: dict[str, set[str]] = {"swing": {"keypoints", "detections"}}


# --------------------------------------------------------------------------- schemas (P1)

#: The shapes that cross the seam between the Rust core, the Python pose sidecar and the store.
#:
#: Deliberately not "every model in `contracts/`": a schema is a promise to keep a shape stable,
#: and promising that for shapes nothing outside Python reads would freeze the parts of the
#: contract that still move.
#:
#: **The rule is: a schema exists for every JSON artifact a non-Python implementation opens off
#: disk.** That is mechanically checkable against a swing directory, which the looser "something
#: other than Python parses it" is not — the first pass at this list read that loosely, dropped
#: `SwingManifest` as internal, and missed that ADR-030 §1 gives Rust *storage*, so a Rust core
#: opens `manifest.json` on its way to every swing. `tests/test_conformance.py` now pins the list
#: against the artifacts a stored swing actually holds.
SCHEMA_ROOTS: dict[str, type] = {
    # In: what the pose sidecar hands the core, one file per clip — `{role}.keypoints.json`.
    "keypoints_file": KeypointsFile,
    # In: what the audio edge hands the core — `{role}.audio.json`. The core itself receives frame
    # indices (ADR-025), but the stored artifact is this, and it is what a port reads off disk.
    "audio_file": AudioFile,
    # In: what the launch-monitor edge attaches. Carried and reported, never scored (ADR-009).
    "shot_data": ShotData,
    # In: the swing directory's own index — `manifest.json`. Which clip plays which role, the
    # content hash that keys every cache, the club tag and the `player_id` that resolves to a
    # handedness. A port cannot find a clip without parsing this.
    "swing_manifest": SwingManifest,
    # In: `session.json`, one level up — what the session was.
    "session_meta": SessionMeta,
    # In: the golfer registry and the bag — the two files behind the `handedness` and `loft_deg`
    # arguments `analysis` is forbidden to fetch for itself, and which a vector therefore has to
    # be handed. A Rust core owns that resolution (ADR-030 §1) and so parses both.
    "golfer": Golfer,
    "bag": Bag,
    # Out: the scored face-on view, and the whole bundle verdict around it — `analysis.json`.
    "swing_result": SwingResult,
    "swing_bundle_result": SwingBundleResult,
    # Out: `analysis.state.json`, the denormalised sidecar that decides whether a stored result is
    # stale. `reanalyze.py` is built on it, and a Rust core needs the same judgement.
    "analysis_state": AnalysisState,
}


def export_schemas() -> dict[str, str]:
    """Render each root as JSON Schema text, keyed by filename stem.

    Returned rather than written so the freshness test can compare against the committed files
    without a temporary directory — the test is the only reason the write is separable at all.
    """
    return {
        name: json.dumps(model.model_json_schema(), indent=2, sort_keys=True) + "\n"
        for name, model in SCHEMA_ROOTS.items()
    }


def write_schemas() -> list[Path]:
    SCHEMAS.mkdir(parents=True, exist_ok=True)
    written = []
    for name, text in export_schemas().items():
        path = SCHEMAS / f"{name}.schema.json"
        path.write_text(text, encoding="utf-8")
        written.append(path)
    return written


# --------------------------------------------------------------------------- vectors (P2)


def _read_json(path: Path) -> dict[str, Any]:
    """Read a vector, gzipped or not, decided by the suffix rather than by sniffing.

    The corpus vectors are gzipped because the un-gzipped set is 25.6 MB of float text against
    8.3 MB compressed, and nobody will ever read a diff of either. The synthetic ones are not,
    because they are small and being *readable* is most of their value.
    """
    if path.suffix == ".gz":
        with gzip.open(path, "rt", encoding="utf-8") as handle:
            return json.load(handle)
    return json.loads(path.read_text(encoding="utf-8"))


def _write_json(path: Path, payload: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    if path.suffix == ".gz":
        # mtime=0 so a regeneration that changes nothing produces a byte-identical file; the
        # default stamps the current time into the header and makes every run look like a change.
        with gzip.GzipFile(path, "wb", compresslevel=9, mtime=0) as handle:
            handle.write(json.dumps(payload, indent=2, sort_keys=True).encode("utf-8"))
        return
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def vector_paths() -> list[Path]:
    """Every committed vector, in a stable order, discovered rather than listed.

    Discovery over a listing for the same reason `tests/test_docs_truth.py` discovers its phase
    docs: a vector that exists on disk and in no index is a vector nothing runs.
    """
    return sorted(
        p
        for p in VECTORS.rglob("*.json*")
        if p.name != MANIFEST.name and p.suffix in {".json", ".gz"}
    )


def run_vector(vector: dict[str, Any]) -> dict[str, Any]:
    """Run one vector's input through this build and return the serialized result.

    This is the whole of what a port has to reimplement. Everything else in this file is
    plumbing around it.

    **Two calls, not one, and the second is not optional.** `analysis` may not import `feedback`
    (ADR-008), so `analyze_swing_bundle` leaves `SwingBundleResult.feedback` as None and
    `api/pipeline.py` fills it in immediately afterwards — which means the artifact this repo
    actually writes has ranked tips in it and a bare engine call does not. A vector recorded off
    the engine alone would pin `"feedback": null` and quietly tell a port to ship a results page
    with no coaching on it.
    """
    from golf_coach.analysis.engine import analyze_swing_bundle
    from golf_coach.feedback.rules import build_feedback

    given = vector["input"]
    dtl = given.get("down_the_line")
    result = analyze_swing_bundle(
        swing_id=given["swing_id"],
        session_id=given["session_id"],
        face_on=KeypointsFile.model_validate(given["face_on"]),
        down_the_line=KeypointsFile.model_validate(dtl) if dtl else None,
        shot=ShotData.model_validate(given["shot"]) if given.get("shot") else None,
        intent=PracticeGoal.model_validate(given["intent"]) if given.get("intent") else None,
        face_on_window=_tuple(given.get("face_on_window")),
        down_the_line_window=_tuple(given.get("down_the_line_window")),
        face_on_strikes=given.get("face_on_strikes"),
        down_the_line_strikes=given.get("down_the_line_strikes"),
        handedness=_handedness(given.get("handedness")),
        loft_deg=given.get("loft_deg"),
    )
    result.feedback = build_feedback(result.swing)
    return _serialize(result)


def _tuple(window: list[int] | None) -> tuple[int, int] | None:
    """JSON has no tuple, so a window arrives as a two-element list and must go back."""
    return None if window is None else (window[0], window[1])


def _handedness(value: str | None) -> Handedness | None:
    """`"right"` back into the enum — the engine reads `.value` off it and JSON has no enums.

    Not a cosmetic coercion: `flight_infer.infer_spin_axis` formats `handedness.value` into a
    provenance string, so a bare `str` gets all the way to a flight before it fails.
    """
    return None if value is None else Handedness(value)


def _serialize(result: SwingBundleResult) -> dict[str, Any]:
    """The shell's serialization, and the one a port is asked to match. See the module docstring."""
    return json.loads(result.model_dump_json(exclude=EXCLUDED_FROM_RESULT))


# --------------------------------------------------------------------------- tolerances (P3)

#: Floats are compared as `|a - b| <= ATOL + RTOL * |expected|`. Two implementations that do the
#: same arithmetic in a different *order* differ in the last bits and nowhere else, so the
#: tolerance has to admit reassociation and nothing wider. RTOL at 1e-9 is roughly six orders of
#: magnitude above f64 epsilon and six below anything this engine would call a difference; ATOL
#: covers the quantities that live near zero (normalized landmark deltas, a scored share).
#:
#: **A band edge is not protected by this and is not meant to be.** `passed` is a bool and is
#: compared exactly, so a port that lands a hair the other side of a band fails on the verdict
#: rather than on the number — which is the failure worth being loud, and is why widening this
#: tolerance would be the wrong repair for it.
RTOL = 1e-9
ATOL = 1e-12


@dataclass(frozen=True)
class Difference:
    """One disagreement, addressed by a JSON path a human can find in the file."""

    path: str
    expected: Any
    actual: Any
    rule: str

    def __str__(self) -> str:
        return f"{self.path}: expected {self.expected!r}, got {self.actual!r} [{self.rule}]"


def compare_results(expected: Any, actual: Any, path: str = "") -> list[Difference]:
    """Diff two serialized results under the rules `docs/CONFORMANCE.md` states.

    The rules, in the order they are applied below:

    - **`None` is a value, not a zero.** ADR-010 §2 says a checkpoint that could not be measured
      returns `None` and is named in `unscored`; a port that emits 0.0 there has turned "could not
      measure" into "measured zero", which is the single most damaging thing it could do. So a
      `None`/number mismatch is reported as a type difference and never tested numerically.
    - **Structure is exact.** Same keys, same list lengths, same order. `unscored` and
      `checkpoint_scores` are ordered by `CHECKPOINT_REGISTRY`, which a port walks too, so an
      order difference is a real finding and not noise.
    - **Bools, ints and strings are exact.** The verdict, the frame indices and the sentences.
      `bool` is checked before `int` on purpose — in Python `True == 1` and `isinstance(True, int)`
      is true, so the obvious ordering would compare a verdict numerically.
    - **Floats are within `RTOL`/`ATOL`.**
    """
    if expected is None or actual is None:
        if expected is not actual:
            return [Difference(path or "$", expected, actual, "null is not a number (ADR-010 §2)")]
        return []

    if isinstance(expected, dict):
        if not isinstance(actual, dict):
            return [Difference(path or "$", type(expected).__name__, type(actual).__name__, "type")]
        out: list[Difference] = []
        for key in sorted(set(expected) | set(actual)):
            if key not in expected or key not in actual:
                out.append(
                    Difference(
                        f"{path}.{key}",
                        "present" if key in expected else "absent",
                        "present" if key in actual else "absent",
                        "key set is exact",
                    )
                )
                continue
            out.extend(compare_results(expected[key], actual[key], f"{path}.{key}"))
        return out

    if isinstance(expected, list):
        if not isinstance(actual, list):
            return [Difference(path or "$", "list", type(actual).__name__, "type")]
        if len(expected) != len(actual):
            return [Difference(path or "$", len(expected), len(actual), "list length is exact")]
        out = []
        for i, (e, a) in enumerate(zip(expected, actual, strict=True)):
            out.extend(compare_results(e, a, f"{path}[{i}]"))
        return out

    # Before the int branch: `isinstance(True, int)` is true, and a verdict compared as a number
    # is a verdict not compared at all.
    if isinstance(expected, bool) or isinstance(actual, bool):
        if expected != actual or type(expected) is not type(actual):
            return [Difference(path, expected, actual, "bool is exact")]
        return []

    if isinstance(expected, str):
        if expected != actual:
            return [Difference(path, expected, actual, "string is exact")]
        return []

    if isinstance(expected, int) and isinstance(actual, int):
        if expected != actual:
            return [Difference(path, expected, actual, "int is exact")]
        return []

    if isinstance(expected, int | float) and isinstance(actual, int | float):
        if math.isnan(expected) or math.isnan(actual):
            # NaN never reaches a stored artifact — it is not valid JSON — so seeing one means a
            # producer wrote something that cannot round-trip, which is a finding either way.
            return [Difference(path, expected, actual, "NaN is never a result")]
        if abs(expected - actual) > ATOL + RTOL * abs(expected):
            return [Difference(path, expected, actual, f"float within {RTOL:g} rel")]
        return []

    return [Difference(path, expected, actual, "unhandled type")]


# --------------------------------------------------------------------------- the commands (P4)


def cmd_check(args: argparse.Namespace) -> int:
    paths = vector_paths()
    if args.id:
        paths = [p for p in paths if _vector_id(p) in set(args.id)]
        if not paths:
            print(f"no vector matches {args.id}", file=sys.stderr)
            return 2
    if not paths:
        print(f"no vectors under {VECTORS} — run `regenerate` first", file=sys.stderr)
        return 2

    failed = 0
    for path in paths:
        vector = _read_json(path)
        name = _vector_id(path)
        stated = vector.get("analysis_version")
        if stated != ANALYSIS_VERSION:
            # Not a conformance failure: the vector and the engine are different generations, so
            # a diff between them measures the version bump rather than the implementation.
            print(f"STALE {name}: recorded at v{stated}, engine is v{ANALYSIS_VERSION}")
            failed += 1
            continue
        diffs = compare_results(vector["expected"], run_vector(vector))
        if diffs:
            failed += 1
            print(f"FAIL  {name}  ({len(diffs)} difference{'s' if len(diffs) > 1 else ''})")
            for d in diffs[: args.max_diffs]:
                print(f"        {d}")
            if len(diffs) > args.max_diffs:
                print(f"        ... and {len(diffs) - args.max_diffs} more")
        elif args.verbose:
            print(f"ok    {name}")

    print(f"\n{len(paths) - failed}/{len(paths)} vectors conform (engine v{ANALYSIS_VERSION})")
    return 1 if failed else 0


def cmd_run(args: argparse.Namespace) -> int:
    """Read a vector on stdin, write this build's result on stdout. The cross-language seam."""
    vector = json.load(sys.stdin)
    json.dump(run_vector(vector), sys.stdout, indent=2, sort_keys=True)
    sys.stdout.write("\n")
    return 0


def cmd_list(args: argparse.Namespace) -> int:
    rows = []
    for path in vector_paths():
        vector = _read_json(path)
        prov = vector.get("provenance", {})
        rows.append(
            (
                _vector_id(path),
                f"v{vector.get('analysis_version')}",
                prov.get("kind", "?"),
                prov.get("note", ""),
            )
        )
    width = max((len(r[0]) for r in rows), default=0)
    for name, version, kind, note in rows:
        print(f"{name:<{width}}  {version:>4}  {kind:<9}  {note}")
    print(f"\n{len(rows)} vectors, engine v{ANALYSIS_VERSION}")
    return 0


def _vector_id(path: Path) -> str:
    """`spec/vectors/corpus/2026-08-09-2.json.gz` -> `corpus/2026-08-09-2`."""
    rel = path.relative_to(VECTORS).as_posix()
    for suffix in (".json.gz", ".json"):
        if rel.endswith(suffix):
            return rel[: -len(suffix)]
    return rel


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    sub = parser.add_subparsers(dest="command", required=True)

    check = sub.add_parser("check", help="run every committed vector against this build")
    check.add_argument("--id", action="append", help="check only this vector id (repeatable)")
    check.add_argument("--max-diffs", type=int, default=10, help="differences printed per vector")
    check.add_argument("-v", "--verbose", action="store_true", help="name the passing vectors too")
    check.set_defaults(func=cmd_check)

    run = sub.add_parser("run", help="vector on stdin, serialized result on stdout")
    run.set_defaults(func=cmd_run)

    listing = sub.add_parser("list", help="what is committed, and where it came from")
    listing.set_defaults(func=cmd_list)

    regen = sub.add_parser("regenerate", help="rewrite spec/ from contracts and data/processed")
    regen.add_argument(
        "--schemas-only",
        action="store_true",
        help="skip the vectors, which need data/processed and exist on the capture machine only",
    )
    regen.set_defaults(func=cmd_regenerate)

    args = parser.parse_args(argv)
    return int(args.func(args))


def cmd_regenerate(args: argparse.Namespace) -> int:
    # Imported here, not at module scope: building vectors reads `data/processed/` and the
    # synthetic fixtures under `tests/`, neither of which a *checking* run should need to exist.
    # The bare name relies on this directory being on `sys.path`, which is the convention
    # `scripts/golfdb/common.py` already documents — inserted explicitly because this module is
    # also imported by `tests/test_conformance.py`, where it is not.
    sys.path.insert(0, str(Path(__file__).resolve().parent))
    from conformance_vectors import build_all

    for path in write_schemas():
        print(f"schema   {path.relative_to(REPO).as_posix()}")
    if args.schemas_only:
        return 0
    for path, payload in build_all():
        _write_json(path, payload)
        print(f"vector   {path.relative_to(REPO).as_posix()}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
