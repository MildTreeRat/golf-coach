"""The Python core as an oracle: schemas out, golden vectors in, a diff either way. [M19]

    python scripts/conformance.py check              # the freeze: frozen Python against each vector
    python scripts/conformance.py check --id corpus/2026-08-09-2
    python scripts/conformance.py run < vector.json  # one vector in, its result on stdout
    python scripts/conformance.py list               # what is committed, and where it came from
    python scripts/conformance.py regenerate --schemas-only   # the Python-owned schema roots
    python scripts/conformance.py regenerate --format-only    # the format family, from CPython
    python scripts/conformance.py regenerate --screen-once    # the screen family, once (M34 P4)
    python scripts/conformance.py regenerate --storage-once   # the storage family, once (M36 P2)
    python scripts/conformance.py regenerate --career-once    # the career family, once (M36 P3)

**From M32 this is the frozen half, not the oracle** (ADR-035 clauses 3 and 4). `golf-core
rerecord` records the engine and stage families, and `cargo test` certifies them. Each Rust
re-record leaves a `provenance.rerecords` entry naming the paths it moved or added, and every
other value in the file is still the one Python recorded. So `check` compares in `frozen_view`,
with those paths taken out of both sides: what it certifies is that frozen Python still
reproduces every value it recorded, which is the freeze, checked. It no longer says the vectors
are right. `regenerate` refuses the two families for the same reason, below.

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
import copy
import gzip
import json
import math
import re
import sys
from dataclasses import dataclass
from enum import Enum
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

#: The roots whose committed schema Rust owns from M32, edited by hand rather than exported here.
#:
#: They are the three whose shape M32's wider `ShotData` moves. Frozen Python's models never gain
#: those keys (ADR-035 clause 4), so an export from them would write the old shape back over the
#: new one. `crates/contracts/tests/schemas.rs` pins the three against the Rust structs instead.
#: They stay in `SCHEMA_ROOTS`, because that dict is also the map
#: `test_every_on_disk_artifact_has_a_schema` checks every stored artifact against, and an
#: artifact's schema does not stop existing when its owner changes.
RUST_OWNED_SCHEMAS = frozenset({"shot_data", "swing_result", "swing_bundle_result"})


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
    """Write the Python-owned roots, and never a `RUST_OWNED_SCHEMAS` one."""
    SCHEMAS.mkdir(parents=True, exist_ok=True)
    written = []
    for name, text in export_schemas().items():
        if name in RUST_OWNED_SCHEMAS:
            continue
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


#: Vector families this program can run. Everything else is discovered and deferred.
#:
#: `vector_paths` stays "every vector committed" on purpose — that discovery is what stops a
#: vector existing on disk and in no index — so the split between *what is committed* and *what
#: this program can execute* lives here instead. The audio family is the Rust detector's
#: (ADR-030 §1, M20) and `cargo test` runs it; there is no Python detector left to run it against.
ENGINE_KINDS = frozenset({"synthetic", "corpus"})


#: Path -> the `provenance.kind` last read from it, keyed on the file's own identity.
#:
#: Memoized because reading a kind means parsing the whole vector, and the three `*_vector_paths`
#: helpers below are each a full sweep — `cmd_check` alone called all three and re-parsed 18 MB
#: three times, which took `check` from 7 s to over two minutes the day the stages family landed.
#: Keyed on `(path, mtime, size)` rather than on the path alone so that a `regenerate` inside one
#: process — which is what `tests/test_conformance.py` does — invalidates it rather than serving
#: the previous generation's answer.
_KIND_CACHE: dict[tuple[str, int, int], str | None] = {}


def _kind(path: Path) -> str | None:
    stat = path.stat()
    key = (str(path), stat.st_mtime_ns, stat.st_size)
    if key not in _KIND_CACHE:
        _KIND_CACHE[key] = _read_json(path).get("provenance", {}).get("kind")
    return _KIND_CACHE[key]


def engine_vector_paths() -> list[Path]:
    """Every committed vector this program is the implementation for."""
    return [p for p in vector_paths() if _kind(p) in ENGINE_KINDS]


def audio_vector_paths() -> list[Path]:
    """Every committed vector `crates/trigger` is the implementation for."""
    return [p for p in vector_paths() if _kind(p) == "audio"]


def stage_vector_paths() -> list[Path]:
    """Every committed vector `crates/analysis` is the implementation for (M22).

    Deferred by `check` for the same reason the audio family is — the implementation under test is
    Rust — but for a *different* reason than audio's, and the difference matters when one of these
    goes stale. Audio has no recorder at all, and `regenerate` refuses it. These had
    `regenerate --stages-only` until M32, and have `golf-core rerecord` from it (ADR-035 clause
    3). So `check` still reads their `analysis_version` and still fails on a stale one, because
    unlike audio there is always something to do about it. Since M32 "stale" means `ledger_covers`
    says no: a version below frozen Python's, or one above it with no ledger entry saying so.
    """
    return [p for p in vector_paths() if _kind(p) == "stages"]


def format_vector_paths() -> list[Path]:
    """Every committed vector of CPython's own behaviour, which a Rust crate implements (M22 P3).

    The third deferred family, and the only one that does **not** age on `ANALYSIS_VERSION`: what
    it records is CPython's own `round`, `format` and `sorted` — and since M34 P2 its `repr` of a
    `str`, float `//`, string case and whitespace, `sum` and `difflib` — so an engine version bump
    leaves every answer in it true. `check` therefore reports these without a staleness test,
    where it runs one over the stages — the field these carry is `python_version`.

    Most are `crates/pyfmt`'s. A vector naming another crate in `provenance.implemented_by` is that
    crate's (`difflib_ratio` is `crates/screen`'s), and one without the key predates it.
    """
    return [p for p in vector_paths() if _kind(p) == "format"]


def screen_vector_paths() -> list[Path]:
    """Every committed vector `crates/screen` is the implementation for (M34 P4).

    The fourth deferred family, and the second that does not age on `ANALYSIS_VERSION`: what it
    records is the screen parser, whose generation is `SCREEN_PARSER_VERSION` in
    `crates/contracts/src/shot.rs`, a constant frozen Python does not have. So `check` reports these
    without a staleness test of its own. Freshness is `golf-core rerecord`'s to judge, because it is
    the only thing that moves the version these carry. Frozen Python recorded them once, through
    `regenerate --screen-once`, and refuses to again.
    """
    return [p for p in vector_paths() if _kind(p) == "screen"]


def storage_vector_paths() -> list[Path]:
    """Every committed vector `crates/storage` is the implementation for (M36 P2).

    The fifth deferred family, and the third that does not age on `ANALYSIS_VERSION`: it records
    the stores and the corpus reader over them, whose generation is `CAREER_VERSION`, Rust's
    constant (the M36 plan's decision 8). `read_corpus` takes the engine versions as a parameter
    recorded in each case's input, so an engine bump moves nothing here, and `check` runs no
    staleness test, as it runs none over the screen family. Frozen Python recorded them once,
    through `regenerate --storage-once`, and refuses to again.
    """
    return [p for p in vector_paths() if _kind(p) == "storage"]


def career_vector_paths() -> list[Path]:
    """Every committed vector of the career aggregates and the scripts' reports (M36 P3).

    The sixth deferred family, aging on the storage family's key: `CAREER_VERSION`, Rust's alone
    (the M36 plan's decision 8), so `check` runs no staleness test over it either. The aggregates
    are `crates/analysis`' to reproduce and the report text `crates/core`'s. Frozen Python recorded
    them once, through `regenerate --career-once`, and refuses to again.
    """
    return [p for p in vector_paths() if _kind(p) == "career"]


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


# --------------------------------------------------------------------------- stages (M22 P1)

#: The engine's own function boundaries, in dependency order — the `stages` family's keys.
#:
#: Not invented seams: each is a call `analyze_swing_bundle` already makes, which is what makes
#: them cheap to record and meaningful to fail on. ADR-032 §2 names them and M22's phase list is
#: built on them — P4 is gated by the first three, P5 by the next two, P7 by `alignment` and P8
#: by `flight`.
#:
#: **What earns a key here: a port must be able to run the stage in isolation from the vector's
#: input and compare.** ADR-032 §2 listed a ninth, `feedback`, and it does not meet that rule —
#: `build_feedback` takes the *assembled* `SwingResult`, which no stage produces and this family
#: does not hold, so a port cannot reach it without having finished the engine. At that point
#: `expected.feedback` on the engine vector already gates it, which is what M22 P6's end-to-end
#: gate is. Recording it here would be a second copy of an answer already committed a few
#: hundred bytes away, and a second copy is a thing that drifts.
STAGE_NAMES = (
    "smoothed",
    "phases",
    "measure",
    "measurements",
    "checkpoints",
    "alignment",
    "flight",
)

#: How `measurements` is grouped, in the order the engine concatenates the groups.
#:
#: A tuple rather than a dict because `_write_json` sorts keys, so insertion order does not
#: survive to disk — and the order *is* part of the answer here: `swing.measurements` is one flat
#: list, and a port producing the right rows in the wrong order fails CONFORMANCE §3's "structure
#: is exact" rule. The grouping is what lets one phase be gated without the ones after it: M22 P5
#: owns `pose` through `pivot_face_on`, P7 owns the two `_dtl` groups, P8 owns `shot` and
#: `flight`.
MEASUREMENT_GROUPS = (
    "pose",
    "placements",
    "pivot_face_on",
    "shot",
    "flight",
    "placements_dtl",
    "pivot_dtl",
)


def run_stages(vector: dict[str, Any]) -> dict[str, Any]:
    """Run one vector's input and return the engine's intermediates, keyed by stage.

    **This calls the engine's own functions, including its private ones, rather than
    reimplementing any of them.** That import is the point: the alternative is a second copy of
    `_measurements`' grouping, or of `_windowed`'s clamping, living here — and a second copy is a
    thing that drifts from the definition it is supposed to be recording. What this function owns
    is the *orchestration*, which calls in which order, and that is the one thing it can still
    get wrong. `conformance_vectors._verify_stages_compose` is why that is safe: it refuses to
    build a stage vector whose parts do not add back up to the committed bundle answer.

    Window handling matches `analyze_swing_bundle` exactly — the face-on frames are sliced first
    and every frame index below is **window-relative**, the way the engine computes them before
    `_shifted` puts them back. Corpus vectors are handed their slice with no window, so only
    `synthetic/windowed` exercises the difference, and it is the vector that fails a port which
    forgets the shift.
    """
    from golf_coach.analysis import engine as E
    from golf_coach.analysis.alignment import (
        align_swings,
        anchors_from_keypoints,
        anchors_from_phases,
    )
    from golf_coach.analysis.checkpoints import CHECKPOINT_EVALUATORS
    from golf_coach.analysis.flight_measure import FLIGHT_SOURCE, flight_unscored, fly_shot
    from golf_coach.analysis.measure import POSE_MEASUREMENTS
    from golf_coach.analysis.phases import TRAIL_WRIST, segment_phases
    from golf_coach.analysis.smoothing import smooth_keypoints
    from golf_coach.analysis.trajectory import anchors_from_phases as event_time_anchors
    from golf_coach.contracts.checkpoints import CHECKPOINT_REGISTRY
    from golf_coach.contracts.placements import DOWN_THE_LINE, FACE_ON

    given = vector["input"]
    face_on = KeypointsFile.model_validate(given["face_on"])
    dtl_given = given.get("down_the_line")
    down_the_line = KeypointsFile.model_validate(dtl_given) if dtl_given else None
    shot = ShotData.model_validate(given["shot"]) if given.get("shot") else None
    intent = PracticeGoal.model_validate(given["intent"]) if given.get("intent") else PracticeGoal()
    handedness = _handedness(given.get("handedness"))
    loft_deg = given.get("loft_deg")

    _start, frames = E._windowed(face_on.frames, _tuple(given.get("face_on_window")))
    smoothed = smooth_keypoints(frames)
    phases = segment_phases(smoothed)

    # The unrounded pose numbers, and the reason for each that refused. Both are things the
    # bundle answer cannot show: `_measurements` rounds to four decimals on the way into a
    # `Measurement`, and it drops an unmeasurable metric with no record of which gate failed.
    # That makes this the only committed evidence of Python's pre-rounding float, which is what
    # ADR-032 §3's banker's-rounding edge needs in order to be diagnosable rather than merely
    # detectable.
    measure_rows = []
    pose_values: dict[str, float] = {}
    for name, pose in POSE_MEASUREMENTS.items():
        outcome = pose.measure(smoothed, phases)
        measure_rows.append(
            {
                "name": name,
                "value": outcome.value,
                "reason": outcome.reason.value if outcome.reason else None,
                "detail": outcome.detail,
            }
        )
        if outcome.value is not None:
            pose_values[name] = outcome.value

    checkpoint_rows = []
    for spec in CHECKPOINT_REGISTRY:
        judged = CHECKPOINT_EVALUATORS[spec.name](smoothed, phases, handedness, intent.club, None)
        checkpoint_rows.append(
            {
                "name": spec.name,
                "score": _jsonable(judged.score),
                "reason": judged.reason.value if judged.reason else None,
                "detail": judged.detail,
            }
        )

    flown = fly_shot(shot, loft_deg=loft_deg, handedness=handedness) if shot is not None else None

    # `_measurements` is called once and its answer is *sliced*, not rebuilt group by group.
    #
    # **Grouping by `Measurement.source` is the obvious way to do this and it is wrong**, which
    # this discovered by producing eighteen `pose` rows against thirteen pose measurements: the
    # face-on pivot rows carry `source="pose:face_on"` as well (`engine._PIVOT_VIEWS`), and so do
    # `placements` and `placements_dtl` both carry `population:golfdb`. `source` answers "which
    # instrument read this", which is not the same question as "which function appended it", and
    # only the second one maps onto a port's phases. The sub-calls below are therefore used for
    # their *lengths*, and the rows themselves come off the one real list in the one real order.
    groups: dict[str, list[Any]] = {name: [] for name in MEASUREMENT_GROUPS}
    every = E._measurements(smoothed, phases, shot, handedness, flown)
    n_placements = len(E._placements(smoothed, phases, pose_values, handedness))
    face_on_anchors = event_time_anchors(phases)
    n_pivot = (
        0
        if face_on_anchors is None
        else len(E._pivot_measurements(smoothed, face_on_anchors, FACE_ON))
    )
    n_shot = n_flight = 0
    if shot is not None:
        device = shot.provenance.device if shot.provenance else shot.source.value
        n_shot = len([m for m in every if m.source == f"launch_monitor:{device}"])
        n_flight = len([m for m in every if m.source == FLIGHT_SOURCE])
    cuts = (len(every) - n_placements - n_pivot - n_shot - n_flight, n_placements, n_pivot,
            n_shot, n_flight)
    at = 0
    for name, size in zip(("pose", "placements", "pivot_face_on", "shot", "flight"), cuts,
                          strict=True):
        groups[name] = [_jsonable(m) for m in every[at : at + size]]
        at += size
    assert at == len(every), "the measurement groups do not partition `_measurements`"

    # The two views' anchors, before and after the ball strike moves the impact one. The pinned
    # pair is what `align_swings` receives and what M11 built; the unpinned pair is where
    # `alignment.py`'s ten frame-index roundings live, so recording both tells a port *which
    # half* it got wrong rather than only that the warp disagreed.
    notes: list[str] = []
    face_from_phases = anchors_from_phases(
        phases, clip=face_on.clip, camera_id=E._camera_id(frames)
    )
    face_anchors = E._anchored_on_strike(
        face_from_phases, given.get("face_on_strikes"), "face-on", notes
    )
    dtl_from_keypoints = None
    if down_the_line is not None:
        dtl_from_keypoints = anchors_from_keypoints(
            down_the_line.frames,
            clip=down_the_line.clip,
            window=_tuple(given.get("down_the_line_window")),
            wrist=TRAIL_WRIST,
        )
    dtl_anchors = E._anchored_on_strike(
        dtl_from_keypoints, given.get("down_the_line_strikes"), "down-the-line", notes
    )

    if down_the_line is not None and dtl_anchors is not None:
        dtl_frames = smooth_keypoints(down_the_line.frames)
        dtl_events = (
            float(dtl_anchors.motion_start),
            float(dtl_anchors.top),
            float(dtl_anchors.impact),
        )
        groups["placements_dtl"] = [
            _jsonable(m) for m in E._dtl_placements(dtl_frames, dtl_events, handedness)
        ]
        groups["pivot_dtl"] = [
            _jsonable(m) for m in E._pivot_measurements(dtl_frames, dtl_events, DOWN_THE_LINE)
        ]

    return {
        # `x`/`y` only, all 33 landmarks. `smooth_keypoints` copies `z`, `visibility`,
        # `frame_index` and `timestamp_ms` through untouched, so recording them would record the
        # *input* a second time and cost 2.3 MB gzipped to test a copy. The pass-through is
        # asserted at build time instead — the same trade `spec/vectors/audio/` makes when it
        # buys chunk-independence as a property rather than 1.7 MB of waveform.
        "smoothed": [[[lm.x, lm.y] for lm in frame.landmarks] for frame in smoothed],
        "phases": [_jsonable(p) for p in phases],
        "measure": measure_rows,
        "measurements": [{"group": g, "rows": groups[g]} for g in MEASUREMENT_GROUPS],
        "checkpoints": checkpoint_rows,
        "alignment": {
            "face_anchors_from_phases": _jsonable(face_from_phases),
            "face_anchors": _jsonable(face_anchors),
            "dtl_anchors_from_keypoints": _jsonable(dtl_from_keypoints),
            "dtl_anchors": _jsonable(dtl_anchors),
            "clip_alignment": (
                _jsonable(align_swings(face_anchors, dtl_anchors))
                if face_anchors is not None and dtl_anchors is not None
                else None
            ),
        },
        "flight": {
            "flown": None
            if flown is None
            else {
                "resolved": _jsonable(flown.resolved),
                "flight": _jsonable(flown.flight),
                "reason": flown.reason.value if flown.reason else None,
                "detail": flown.detail,
            },
            "unscored": [_jsonable(u) for u in flight_unscored(flown)] if flown else [],
        },
    }


def _jsonable(value: Any) -> Any:
    """One serializer for every shape a stage records: pydantic, NamedTuple, enum or plain.

    The NamedTuples are the awkward half. `FlownShot`, `ShotFlight` and `FlightResult` are tuples
    holding pydantic models and enums, and `json.dumps` renders a NamedTuple as a bare array —
    a shape no reader can address a difference *inside* of. Expanding them with `_asdict` and
    recursing is what keeps every leaf reachable by the JSON path `compare_results` prints, which
    is the whole reason that function reports a path at all.
    """
    if value is None or isinstance(value, bool | int | float | str):
        return value
    if hasattr(value, "model_dump_json"):
        return json.loads(value.model_dump_json())
    if isinstance(value, Enum):
        return value.value
    if hasattr(value, "_asdict"):
        return {k: _jsonable(v) for k, v in value._asdict().items()}
    if isinstance(value, dict):
        return {k: _jsonable(v) for k, v in value.items()}
    if isinstance(value, list | tuple):
        return [_jsonable(v) for v in value]
    return value


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


# --------------------------------------------------------------------------- the freeze (M32 P6)
#
# From M32 Rust records the engine and stage families (ADR-035 clause 3), and frozen Python stops
# moving (clause 4). A Rust re-record writes the committed document with only its declared paths
# replaced, and lists those paths in `provenance.rerecords`. Every other value is still the one
# Python recorded. So what frozen Python can still be held to is exactly those other values: take
# the ledgered paths out of both sides and compare what is left. That is a check that the freeze
# held. It is not a check that the vectors are right; `cargo test` is that.

#: A ledger path, in the one spelling `golf-core rerecord` writes (M32's plan, call 3): rooted at
#: the document, no leading dot, a list index as `[i]` — `expected.swing.shot.attack_angle`.
#:
#: The grammar is `crates/core/src/rerecord.rs::LedgerPath::parse`'s, and it is *parsed*, not
#: matched as a string, in both languages. `compare_results` spells the same place
#: `.swing.shot.attack_angle`, rooted at the answer and with a leading dot, so a string match
#: between the two would quietly match nothing. Refused like Rust refuses: the empty path, a
#: leading dot, a `[*]` wildcard (call 4), an empty step, and an index spelled `01` or `+1`.
_LEDGER_PATH = re.compile(r"[^.\[\]]+(?:\.[^.\[\]]+|\[(?:0|[1-9][0-9]*)\])*")
_LEDGER_STEP = re.compile(r"(?:^|\.)([^.\[\]]+)|\[([0-9]+)\]")


def parse_ledger_path(text: str) -> list[str | int]:
    """`"expected.swing[0].x"` -> `["expected", "swing", 0, "x"]`, or `ValueError`."""
    if not _LEDGER_PATH.fullmatch(text):
        raise ValueError(
            f"{text!r} is not a ledger path: the spelling is `key(.key|[n])*`, with no leading "
            f"dot and no wildcard (crates/core/src/rerecord.rs::LedgerPath)"
        )
    return [key or int(index) for key, index in _LEDGER_STEP.findall(text)]


def _ledger(vector: dict[str, Any]) -> list[dict[str, Any]]:
    """The vector's `provenance.rerecords`, or no entries for a vector Rust never re-recorded."""
    return list(vector.get("provenance", {}).get("rerecords") or [])


def _answer_key(vector: dict[str, Any]) -> str:
    """Where a vector keeps the answer a run is compared with: `stages` on a stage vector."""
    return "stages" if vector.get("provenance", {}).get("kind") == "stages" else "expected"


def _remove(document: Any, steps: list[str | int]) -> None:
    """Take the value at `steps` out of `document`, if it is there.

    Absent is not an error, on either side. Frozen Python never writes a key Rust added, so on
    the produced side an added path is absent by design. A `None` on the way down is absent too:
    a synthetic vector's `swing.shot` is null.

    **A list element is blanked to `None`, not popped.** Popping would shift every index after it,
    so the two lists would line up wrongly for everything beyond the declared element. Blanking
    both sides keeps the lengths and the alignment, and `None` against `None` compares equal.
    """
    *parents, last = steps
    node = document
    for step in parents:
        if isinstance(step, str) and isinstance(node, dict):
            node = node.get(step)
        elif isinstance(step, int) and isinstance(node, list) and step < len(node):
            node = node[step]
        else:
            return
    if isinstance(last, str) and isinstance(node, dict):
        node.pop(last, None)
    elif isinstance(last, int) and isinstance(node, list) and last < len(node):
        node[last] = None


def frozen_view(vector: dict[str, Any], actual: Any) -> tuple[Any, Any]:
    """The vector's answer and frozen Python's, each without the paths a Rust re-record owns.

    Every path any `provenance.rerecords` entry lists, `added` and `moved` alike, comes out of
    both. A path is rooted at the vector document, so only the ones under its answer key reach the
    comparison (`expected.swing.shot.attack_angle` comes out of `expected` as
    `swing.shot.attack_angle`, and out of `actual` the same way). A top-level path like
    `analysis_version` is not inside the answer, and the version rule is `ledger_covers`'.

    **A vector with no ledger comes back as-is**, not copied, so before M32's first re-record
    this is the plain comparison it replaced. With a ledger both sides are deep-copied first:
    the caller's vector is read again by later checks, and a view must not edit what it views.

    A ledger path in any spelling but `parse_ledger_path`'s raises rather than removing nothing.
    A hand-edited ledger in Python's spelling would otherwise hide nothing, and leave a declared
    difference failing for a reason nobody could see in the file.
    """
    answer = _answer_key(vector)
    entries = _ledger(vector)
    if not entries:
        return vector[answer], actual
    expected, actual = copy.deepcopy(vector[answer]), copy.deepcopy(actual)
    for entry in entries:
        for text in [*entry.get("added", []), *entry.get("moved", [])]:
            steps = parse_ledger_path(text)
            if steps[0] == answer and len(steps) > 1:
                _remove(expected, steps[1:])
                _remove(actual, steps[1:])
    return expected, actual


def ledger_covers(vector: dict[str, Any], version: int) -> bool:
    """Whether `vector` may be compared with an engine at `version`. The version rule since M32.

    True at `version` exactly, as before M32. True above it only when the vector carries a
    `rerecords` entry for every version between: frozen Python stays at its version for good
    (ADR-035 clause 4), and those entries are what say which values the later versions moved, so
    `frozen_view` can take them out. Above it without them, nothing says which values are Rust's.
    Below it, the vector is from an engine older than the one that is frozen. Both are stale.
    """
    stated = vector.get("analysis_version")
    if not isinstance(stated, int) or stated < version:
        return False
    ledgered = {entry.get("analysis_version") for entry in _ledger(vector)}
    return all(v in ledgered for v in range(version + 1, stated + 1))


def _staleness(vector: dict[str, Any]) -> str | None:
    """Why `check` cannot compare `vector` with frozen Python, or `None` when it can."""
    if ledger_covers(vector, ANALYSIS_VERSION):
        return None
    stated = vector.get("analysis_version")
    if isinstance(stated, int) and stated > ANALYSIS_VERSION:
        return (
            f"recorded at v{stated}, above frozen v{ANALYSIS_VERSION}, with no `rerecords` entry "
            f"for each version between — only `golf-core rerecord` writes one"
        )
    return (
        f"recorded at v{stated}, frozen engine is v{ANALYSIS_VERSION} — re-record with "
        f"`golf-core rerecord` (ADR-035 clause 3)"
    )


# --------------------------------------------------------------------------- the commands (P4)


def cmd_check(args: argparse.Namespace) -> int:
    paths = vector_paths()
    if args.id:
        paths = [p for p in paths if _vector_id(p) in set(args.id)]
        if not paths:
            print(f"no vector matches {args.id}", file=sys.stderr)
            return 2
    if not paths:
        # Not "run `regenerate`": it refuses the engine and stage families from M32, and the
        # vectors are committed, so an empty `spec/vectors/` is a broken checkout.
        print(f"no vectors under {VECTORS} — is this a full checkout?", file=sys.stderr)
        return 2

    # Named rather than silently skipped: these are committed vectors that this program is not
    # the implementation for (ADR-030 §1 gives strike detection to Rust), and a run that quietly
    # ignored 30 files would be a run nobody could reconcile with `list`.
    deferred = len([p for p in paths if p in set(audio_vector_paths())])
    stages = [p for p in paths if p in set(stage_vector_paths())]
    formats = [p for p in paths if p in set(format_vector_paths())]
    screens = [p for p in paths if p in set(screen_vector_paths())]
    storages = [p for p in paths if p in set(storage_vector_paths())]
    careers = [p for p in paths if p in set(career_vector_paths())]
    paths = [p for p in paths if p in set(engine_vector_paths())]

    failed = 0
    stale_stages = 0
    ledgered = 0
    # The stages family is `crates/analysis`' to run, but its freshness is this program's to
    # judge: it carries `ANALYSIS_VERSION`, and a stage vector from an older engine certifies a
    # port mid-build against answers this repo has retracted — which is worse than the
    # whole-bundle version of the same mistake, because it is believed four phases earlier. From
    # M32 the rule is `ledger_covers`, not equality: a Rust re-record moves the version past
    # frozen Python's on purpose, and says so in the ledger.
    for path in stages:
        reason = _staleness(_read_json(path))
        if reason:
            print(f"STALE {_vector_id(path)}: {reason}")
            stale_stages += 1
    for path in paths:
        vector = _read_json(path)
        name = _vector_id(path)
        reason = _staleness(vector)
        if reason:
            # Not a conformance failure: the vector and the engine are different generations, so
            # a diff between them measures the version bump rather than the implementation.
            print(f"STALE {name}: {reason}")
            failed += 1
            continue
        if _ledger(vector):
            ledgered += 1
        diffs = compare_results(*frozen_view(vector, run_vector(vector)))
        if diffs:
            failed += 1
            print(f"FAIL  {name}  ({len(diffs)} difference{'s' if len(diffs) > 1 else ''})")
            for d in diffs[: args.max_diffs]:
                print(f"        {d}")
            if len(diffs) > args.max_diffs:
                print(f"        ... and {len(diffs) - args.max_diffs} more")
        elif args.verbose:
            print(f"ok    {name}")

    # `paths` already holds only the engine families — the audio and stage ones were counted
    # separately and filtered out above, so subtracting them again here would double-count. The
    # ratio stays *the engine's*: a stale stage vector is a real failure and exits non-zero, but
    # it is not a vector this program ran, and folding it into this line would make the one
    # number a reader quotes mean two different things.
    #
    # "Hold the freeze", not "conform", since M32: this compares frozen Python with the values it
    # recorded, outside the paths a Rust re-record owns. Whether the vectors are *right* is
    # `cargo test`'s to say now (ADR-035 clause 3), and a line reading "conform" would claim it.
    print(
        f"\n{len(paths) - failed}/{len(paths)} vectors hold the freeze (frozen engine "
        f"v{ANALYSIS_VERSION}; {ledgered} re-recorded by Rust) — `cargo test` certifies them"
    )
    if deferred:
        print(f"{deferred} audio vectors are the Rust detector's — run `cargo test`")
    if stages:
        fresh = len(stages) - stale_stages
        print(
            f"{fresh}/{len(stages)} stage vectors are at frozen v{ANALYSIS_VERSION} or ledgered "
            f"above it — `cargo test` runs them"
        )
    if formats:
        # No freshness line, and the absence is the point: these age on CPython rather than on
        # `ANALYSIS_VERSION`, so there is no version here for this program to compare against.
        vectors = [_read_json(p) for p in formats]
        cases = sum(len(vector.get("cases", [])) for vector in vectors)
        crates = sorted(
            {vector["provenance"].get("implemented_by", "pyfmt") for vector in vectors}
        )
        owners = " and ".join(f"`{crate}`'s" for crate in crates)
        print(f"{len(formats)} format vectors ({cases} cases) are {owners} — run `cargo test`")
    if screens:
        # No freshness line either, for a different reason: these age on `SCREEN_PARSER_VERSION`,
        # which is Rust's alone, and the only thing that moves it on a vector is `golf-core
        # rerecord`. Nothing here can say a screen vector is stale, so nothing here tries.
        print(
            f"{len(screens)} screen vectors are `crates/screen`'s, and age on its "
            f"SCREEN_PARSER_VERSION — run `cargo test`"
        )
    if storages:
        # The screen family's reason again, under the many-shot layer's key: `CAREER_VERSION` is
        # Rust's alone, and `golf-core rerecord` is the only thing that moves it on a vector.
        print(
            f"{len(storages)} storage vectors are `crates/storage`'s, and age on "
            f"CAREER_VERSION — run `cargo test`"
        )
    if careers:
        # The same key, so the same silence: nothing here can say a career vector is stale.
        print(
            f"{len(careers)} career vectors are `crates/analysis`' and `crates/core`'s, and age "
            f"on CAREER_VERSION — run `cargo test`"
        )
    return 1 if failed or stale_stages else 0


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
                # An audio vector ages on `AUDIO_DETECTOR_VERSION`, not on `ANALYSIS_VERSION` —
                # a detector change and an engine change are not the same event. A format vector
                # ages on neither: it records CPython's own rounding and formatting, so what it
                # carries is the interpreter version that answered. A screen vector ages on
                # `SCREEN_PARSER_VERSION`, which is 0 for frozen Python's parse — so the first key
                # *present* wins here, not the first truthy one. A storage or career vector ages
                # on `CAREER_VERSION`, 0 for frozen Python's stores and aggregates in the same way.
                "v{}".format(
                    next(
                        (
                            vector[key]
                            for key in (
                                "analysis_version",
                                "detector_version",
                                "screen_parser_version",
                                "career_version",
                                "python_version",
                            )
                            if vector.get(key) is not None
                        ),
                        None,
                    )
                ),
                prov.get("kind", "?"),
                prov.get("note", ""),
            )
        )
    width = max((len(r[0]) for r in rows), default=0)
    for name, version, kind, note in rows:
        print(f"{name:<{width}}  {version:>4}  {kind:<9}  {note}")
    print(f"\n{len(rows)} vectors, engine v{ANALYSIS_VERSION}")
    return 0


def _shown(path: Path) -> str:
    """`path` as a reader of this repo would type it, or whole when it is outside the repo."""
    return path.relative_to(REPO).as_posix() if path.is_relative_to(REPO) else path.as_posix()


def _files_under(directory: Path) -> list[Path]:
    """Every file below `directory`, or none when it does not exist."""
    return [p for p in directory.rglob("*") if p.is_file()] if directory.exists() else []


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

    regen = sub.add_parser(
        "regenerate",
        help="rewrite the Python-owned schemas or the format family; the engine and stage "
        "families are `golf-core rerecord`'s from M32",
    )
    regen.add_argument(
        "--schemas-only",
        action="store_true",
        help="rewrite the Python-owned schema roots — not the three in RUST_OWNED_SCHEMAS",
    )
    regen.add_argument(
        "--stages-only",
        action="store_true",
        help="refused from M32: the stage family is `golf-core rerecord`'s (ADR-035 clause 3)",
    )
    regen.add_argument(
        "--format-only",
        action="store_true",
        help="rebuild only spec/vectors/format/, from CPython itself — needs no captures",
    )
    regen.add_argument(
        "--screen-once",
        action="store_true",
        help="record spec/vectors/screen/ from the frozen parser, once: needs the `ocr` extra and "
        "data/, and refuses once any screen vector is committed (M34 P4)",
    )
    regen.add_argument(
        "--storage-once",
        action="store_true",
        help="record spec/vectors/storage/ from the frozen stores and corpus reader, once: needs "
        "data/, and refuses once anything is under spec/vectors/storage/ (M36 P2)",
    )
    regen.add_argument(
        "--career-once",
        action="store_true",
        help="record spec/vectors/career/ from the frozen career aggregates and the scripts' "
        "reports, once: needs data/ and the storage family, and refuses once anything is under "
        "spec/vectors/career/ (M36 P3)",
    )
    regen.set_defaults(func=cmd_regenerate)

    args = parser.parse_args(argv)
    return int(args.func(args))


#: Why `regenerate` will not rebuild the engine or stage family, printed when it is asked to.
_REFUSED = """\
refused: the engine and stage families are recorded by `golf-core rerecord` from M32
(ADR-035 clause 3).

A rebuild from frozen Python would write its v{version} answers over the Rust-recorded vectors,
and drop the `provenance.rerecords` ledger that says which of their values are Rust's. Re-record
through the verb's gate instead, from the repo root:

    cargo run --release --bin golf-core -- rerecord --declare spec/declarations/v<N>.json

`regenerate` still does two jobs: --schemas-only (the Python-owned schema roots) and
--format-only (the format family, which records CPython rather than this engine). Three more,
--screen-once, --storage-once and --career-once, recorded the screen, storage and career families
and refuse to run again."""


#: Why `--screen-once` will not run a second time, printed when it is asked to.
_SCREEN_RECORDED = """\
refused: the screen family is already recorded ({count} file{plural} under {where}).

`--screen-once` is the one run of frozen Python's screen parser over the photos (M34 P4, ADR-035
clause 3: record once from frozen Python). From there `crates/screen` is the implementation, and
`golf-core rerecord` is the only thing that writes these files, behind a declaration that names
every value it moves:

    cargo run --release --bin golf-core -- rerecord --declare spec/declarations/screen-v<N>.json

A second run from frozen Python would write its answers over Rust's, and drop the
`provenance.rerecords` ledger that says which of them are Rust's."""


#: Why `--storage-once` will not run a second time, printed when it is asked to.
_STORAGE_RECORDED = """\
refused: the storage family is already recorded ({count} file{plural} under {where}).

`--storage-once` is the one run of frozen Python's stores and corpus reader (M36 P2, ADR-035
clause 3: record once from frozen Python). From there `crates/storage` is the implementation, and
`golf-core rerecord` is the only thing that writes these files, behind a declaration that names
every value it moves:

    cargo run --release --bin golf-core -- rerecord --declare spec/declarations/career-v<N>.json

A second run from frozen Python would write its answers over Rust's, and drop the
`provenance.rerecords` ledger that says which of them are Rust's."""


#: Why `--career-once` will not run a second time, printed when it is asked to.
_CAREER_RECORDED = """\
refused: the career family is already recorded ({count} file{plural} under {where}).

`--career-once` is the one run of frozen Python's career aggregates and the five career scripts'
reports (M36 P3, ADR-035 clause 3: record once from frozen Python). From there `crates/analysis`
and `crates/core` are the implementation, and `golf-core rerecord` is the only thing that writes
these files, behind a declaration that names every value it moves:

    cargo run --release --bin golf-core -- rerecord --declare spec/declarations/career-v<N>.json

A second run from frozen Python would write its answers over Rust's, and drop the
`provenance.rerecords` ledger that says which of them are Rust's."""


def cmd_regenerate(args: argparse.Namespace) -> int:
    # **The full rebuild and `--stages-only` are refused from M32**, on the precedent of
    # `conformance_vectors._audio`: a family whose recorder has moved is rebuilt by the new
    # recorder or not at all, and a working-looking rebuild path here would let that decision be
    # skipped by running a script. `build_all` and `build_stages_from_disk` stay in
    # `conformance_vectors` and stay runnable (docs/README.md §Conventions); only this command
    # stops reaching them. Refused before anything is imported or written, schemas included,
    # so a refused run touches nothing.
    jobs = (
        args.schemas_only,
        args.format_only,
        args.screen_once,
        args.storage_once,
        args.career_once,
    )
    if args.stages_only or not any(jobs):
        print(_REFUSED.format(version=ANALYSIS_VERSION), file=sys.stderr)
        return 2

    # **The career family is recorded once** (M36 P3), on `--storage-once`'s rule: refused on any
    # file under `spec/vectors/career/`, before `conformance_vectors` is imported. A run on an
    # empty family writes nothing until `build_career` has built every case, which reads the
    # committed storage family and holds the real case's corpus to `data/` before it returns.
    if args.career_once:
        recorded = sorted({*career_vector_paths(), *_files_under(VECTORS / "career")})
        if recorded:
            print(
                _CAREER_RECORDED.format(
                    count=len(recorded),
                    plural="" if len(recorded) == 1 else "s",
                    where=_shown(VECTORS / "career"),
                ),
                file=sys.stderr,
            )
            return 2
        sys.path.insert(0, str(Path(__file__).resolve().parent))
        from conformance_vectors import build_career

        for path, payload in build_career():
            _write_json(path, payload)
            print(f"career   {_shown(path)}")
        return 0

    # **The storage family is recorded once** (M36 P2), on `--screen-once`'s rule below: refused
    # on any file under `spec/vectors/storage/`, before `conformance_vectors` is imported, so a
    # refused run cannot reach `data/`. A run on an empty family writes nothing until
    # `build_storage` has built every case, the real corpus's verify included.
    if args.storage_once:
        recorded = sorted({*storage_vector_paths(), *_files_under(VECTORS / "storage")})
        if recorded:
            print(
                _STORAGE_RECORDED.format(
                    count=len(recorded),
                    plural="" if len(recorded) == 1 else "s",
                    where=_shown(VECTORS / "storage"),
                ),
                file=sys.stderr,
            )
            return 2
        sys.path.insert(0, str(Path(__file__).resolve().parent))
        from conformance_vectors import build_storage

        for path, payload in build_storage():
            _write_json(path, payload)
            print(f"storage  {_shown(path)}")
        return 0

    # **The screen family is recorded once** (M34 P4), and this branch is the whole of "once": it
    # refuses on any file under `spec/vectors/screen/`, not only on a file that reads as a screen
    # vector, because `_write_json` would write over either. Refused before `conformance_vectors`
    # is imported, so a refused run cannot reach PaddleOCR or `data/`. A run on an empty family
    # builds everything first and writes only if the whole build, the corpus verify included,
    # succeeded — `build_screen` returns nothing until it has.
    if args.screen_once:
        recorded = sorted({*screen_vector_paths(), *_files_under(VECTORS / "screen")})
        if recorded:
            print(
                _SCREEN_RECORDED.format(
                    count=len(recorded),
                    plural="" if len(recorded) == 1 else "s",
                    where=_shown(VECTORS / "screen"),
                ),
                file=sys.stderr,
            )
            return 2
        sys.path.insert(0, str(Path(__file__).resolve().parent))
        from conformance_vectors import build_screen

        from golf_coach.launch_monitor.screen.importer import MissingOCRExtra

        try:
            built = build_screen()
        except MissingOCRExtra as exc:
            print(f"refused: {exc}", file=sys.stderr)
            return 2
        for path, payload in built:
            _write_json(path, payload)
            print(f"screen   {_shown(path)}")
        return 0

    # The format table derives from CPython itself and runs anywhere, so this branch is taken
    # *before* the schemas are written — it is not a partial `regenerate`, it is a different job
    # (ADR-032 §2). Imported here, not at module scope: the bare name relies on this directory
    # being on `sys.path`, which is the convention `scripts/golfdb/common.py` already documents —
    # inserted explicitly because this module is also imported by `tests/test_conformance.py`,
    # where it is not.
    if args.format_only:
        sys.path.insert(0, str(Path(__file__).resolve().parent))
        from conformance_vectors import build_format

        for path, payload in build_format():
            _write_json(path, payload)
            print(f"format   {path.relative_to(REPO).as_posix()}")
        return 0

    for path in write_schemas():
        print(f"schema   {path.relative_to(REPO).as_posix()}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
